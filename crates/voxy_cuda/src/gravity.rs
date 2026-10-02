use crate::{CudaCompute, CudaError};

/// Explicit double-precision CUDA body, retaining the CPU solver's f64 inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CudaGravityBody {
    pub mass: f64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
}
/// Sticky device failure; snapshots still expose the last committed finite state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaGravityFailure {
    SingularPair,
    NumericalOverflow,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CudaGravitySnapshot {
    pub bodies: Vec<CudaGravityBody>,
    pub failure: Option<CudaGravityFailure>,
}
#[derive(Clone, Copy, Debug)]
pub struct CudaGravityParameters {
    pub constant: f64,
    pub softening: f64,
    pub uniform_acceleration: [f64; 3],
    pub dt: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct CudaGravityBudget {
    pub max_bodies: u32,
    pub max_steps_per_call: u32,
}
impl Default for CudaGravityBudget {
    fn default() -> Self {
        Self {
            max_bodies: 4096,
            max_steps_per_call: 256,
        }
    }
}
impl CudaCompute {
    /// Uploads one bounded f64 N-body system into reusable private CUDA storage.
    /// Compiles/caches fixed kernels with fast math and FMA contraction disabled.
    /// # Errors
    /// Rejects malformed data and allocation budgets before driver work; reports
    /// missing NVRTC and compile/load/transfer failures without CPU fallback.
    #[allow(unsafe_code)]
    pub fn create_gravity_job(
        &self,
        bodies: &[CudaGravityBody],
        parameters: CudaGravityParameters,
        budget: CudaGravityBudget,
    ) -> Result<CudaGravityJob, CudaError> {
        let words = pack(bodies, parameters, budget, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            let allocation_bytes = words.len().checked_mul(8).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let count = u32::try_from(bodies.len()).map_err(|_| CudaError::BufferLimit)?;
            let functions = {
                let mut cached = self
                    .gravity
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("gravity.cu"),
                        self.compiler_options("gravity.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cached = Some([
                        module
                            .load_function("gravity_predict")
                            .map_err(CudaError::Driver)?,
                        module
                            .load_function("gravity_correct")
                            .map_err(CudaError::Driver)?,
                        module
                            .load_function("gravity_commit")
                            .map_err(CudaError::Driver)?,
                        module
                            .load_function("gravity_view_validate")
                            .map_err(CudaError::Driver)?,
                        module
                            .load_function("gravity_view_commit")
                            .map_err(CudaError::Driver)?,
                    ]);
                }
                cached
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let data = stream.clone_htod(&words).map_err(CudaError::Driver)?;
            Ok(CudaGravityJob {
                data,
                stream,
                functions,
                count,
                max_steps: budget.max_steps_per_call,
                reservation,
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = words;
            Err(CudaError::Disabled)
        }
    }
}

/// One resident CUDA system; stream/context/module ownership survives owner drop.
/// Failed physics passes preserve committed input and retain a sticky error.
#[derive(Debug)]
pub struct CudaGravityJob {
    #[cfg(feature = "cuda")]
    data: cudarc::driver::CudaSlice<f64>,
    #[cfg(feature = "cuda")]
    stream: std::sync::Arc<cudarc::driver::CudaStream>,
    #[cfg(feature = "cuda")]
    functions: [cudarc::driver::CudaFunction; 5],
    #[cfg(feature = "cuda")]
    count: u32,
    #[cfg(feature = "cuda")]
    max_steps: u32,
    #[cfg(feature = "cuda")]
    reservation: crate::budget::Reservation,
}
impl CudaGravityJob {
    /// Completes work and checks device-storage cleanup before releasing budget.
    /// Cleanup failure conservatively retains the reservation.
    /// # Errors
    /// Reports synchronization/destruction errors or disabled CUDA support.
    pub fn release(self) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            let Self {
                data,
                stream,
                functions,
                reservation,
                ..
            } = self;
            reservation.release((data, functions), stream.context())
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = self;
            Err(CudaError::Disabled)
        }
    }
    /// Enqueues ordered predict/correct/commit kernels without host transfers.
    /// Numeric errors are reported by readback; recreate a failed job to recover.
    /// # Errors
    /// Rejects step budgets before launch and reports CUDA launch failures.
    #[allow(unsafe_code)]
    pub fn step(&mut self, steps: u32) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            if steps == 0 || steps > self.max_steps {
                return Err(CudaError::BufferLimit);
            }
            let config = LaunchConfig {
                grid_dim: (self.count.div_ceil(64), 1, 1),
                block_dim: (64, 1, 1),
                shared_mem_bytes: 0,
            };
            for _ in 0..steps {
                for function in &self.functions[..3] {
                    let mut launch = self.stream.launch_builder(function);
                    launch.arg(&mut self.data);
                    // SAFETY: Each immutable kernel has exactly one double-pointer
                    // argument. pack validates the count and all three fixed regions.
                    // The count guard covers partial workgroups; one ordered stream
                    // makes prior-pass writes visible before correction/publication.
                    unsafe { launch.launch(config) }.map_err(CudaError::Driver)?;
                }
            }
            Ok(())
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = steps;
            Err(CudaError::Disabled)
        }
    }
    /// Writes a render-only f32 view directly to imported graphics memory.
    /// Layout matches the gravity vertex shader: eight header u32s followed by
    /// position xyz, mass, velocity xyz and padding per body. Only a four-byte
    /// status is read on CPU. The import's graphics ownership contract still applies.
    /// Conversion/solver failure leaves the destination unchanged; solver state
    /// remains f64 and is not replaced by the render view.
    /// # Errors
    /// Rejects a different CUDA context or insufficient destination capacity;
    /// reports solver, f32 conversion, launch and synchronization failures.
    #[allow(unsafe_code)]
    pub fn write_render_view(
        &mut self,
        destination: &mut crate::CudaExternalU32Buffer,
    ) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{DevicePtr, LaunchConfig, PushKernelArg};
            if !std::sync::Arc::ptr_eq(self.stream.context(), destination.stream.context()) {
                return Err(CudaError::ContextMismatch);
            }
            let required = self
                .count
                .checked_mul(8)
                .and_then(|count| count.checked_add(8))
                .ok_or(CudaError::BufferLimit)?;
            if destination.count < required {
                return Err(CudaError::BufferLimit);
            }
            let status_reservation = self.reservation.reserve(4)?;
            let mut status = self
                .stream
                .clone_htod(&[0_u32])
                .map_err(CudaError::Driver)?;
            let result = (|| {
                let config = LaunchConfig {
                    grid_dim: (self.count.div_ceil(64), 1, 1),
                    block_dim: (64, 1, 1),
                    shared_mem_bytes: 0,
                };
                let mut validation = self.stream.launch_builder(&self.functions[3]);
                validation.arg(&mut self.data).arg(&mut status);
                // SAFETY: Fixed kernel; validated resident count bounds every body access.
                unsafe { validation.launch(config) }.map_err(CudaError::Driver)?;
                let (pointer, usage) = destination.mapping.device_ptr(&self.stream);
                let mut commit = self.stream.launch_builder(&self.functions[4]);
                commit.arg(&self.data).arg(&status).arg(&pointer);
                // SAFETY: Same context, exclusive imported range, sufficient u32 capacity;
                // ordered validation finishes before conditional destination publication.
                let result = unsafe { commit.launch(config) }.map_err(CudaError::Driver);
                drop(usage);
                result?;
                self.stream.clone_dtoh(&status).map_err(CudaError::Driver)
            })();
            status_reservation.release(status, self.stream.context())?;
            let status = result?;
            match status[0] {
                0 => Ok(()),
                1 => Err(CudaError::SingularPair),
                2 => Err(CudaError::NumericalOverflow),
                3 => Err(CudaError::RenderViewOutOfRange),
                _ => Err(CudaError::InvalidGravityInput),
            }
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = destination;
            Err(CudaError::Disabled)
        }
    }
    /// Synchronously reads only the header and committed bodies. Does not consume
    /// or free resident state; repeated readback and subsequent steps are allowed.
    /// # Errors
    /// Reports sticky singular/overflow failures, malformed output and driver errors.
    pub fn read(&self) -> Result<Vec<CudaGravityBody>, CudaError> {
        checked(self.snapshot()?)
    }
    /// Reads committed bodies even after a sticky kernel failure, for recovery
    /// diagnostics. Does not clear errors or make a failed job usable again.
    /// # Errors
    /// Reports malformed/nonfinite committed state or driver/transfer failures.
    pub fn snapshot(&self) -> Result<CudaGravitySnapshot, CudaError> {
        #[cfg(feature = "cuda")]
        {
            let size = 8 + usize::try_from(self.count).map_err(|_| CudaError::BufferLimit)? * 8;
            let view = self
                .data
                .try_slice(..size)
                .ok_or(CudaError::InvalidGravityInput)?;
            let words = self.stream.clone_dtoh(&view).map_err(CudaError::Driver)?;
            self.stream.synchronize().map_err(CudaError::Driver)?;
            self.stream
                .context()
                .check_err()
                .map_err(CudaError::Driver)?;
            decode_snapshot(&words, self.count)
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
}

fn pack(
    bodies: &[CudaGravityBody],
    parameters: CudaGravityParameters,
    budget: CudaGravityBudget,
    max_bytes: usize,
) -> Result<Vec<f64>, CudaError> {
    let count = u32::try_from(bodies.len()).map_err(|_| CudaError::BufferLimit)?;
    if count == 0
        || budget.max_bodies == 0
        || budget.max_bodies > u32::MAX / 24
        || count > budget.max_bodies
        || budget.max_steps_per_call == 0
    {
        return Err(CudaError::BufferLimit);
    }
    let eps2 = parameters.softening * parameters.softening;
    if !parameters.constant.is_finite()
        || parameters.constant < 0.0
        || !parameters.softening.is_finite()
        || parameters.softening < 0.0
        || !eps2.is_finite()
        || (parameters.softening > 0.0 && eps2 == 0.0)
        || !parameters.dt.is_finite()
        || parameters.dt <= 0.0
        || parameters
            .uniform_acceleration
            .iter()
            .any(|v| !v.is_finite())
        || bodies.iter().any(|b| {
            !b.mass.is_finite()
                || b.mass <= 0.0
                || b.position.iter().chain(&b.velocity).any(|v| !v.is_finite())
        })
    {
        return Err(CudaError::InvalidGravityInput);
    }
    let size = bodies
        .len()
        .checked_mul(24)
        .and_then(|n| n.checked_add(8))
        .ok_or(CudaError::BufferLimit)?;
    if size.checked_mul(8).is_none_or(|bytes| bytes > max_bytes) {
        return Err(CudaError::BufferLimit);
    }
    let mut words = Vec::with_capacity(size);
    words.extend([parameters.constant, eps2, parameters.dt, f64::from(count)]);
    words.extend(parameters.uniform_acceleration);
    words.push(0.0);
    for body in bodies {
        words.extend(body.position);
        words.push(body.mass);
        words.extend(body.velocity);
        words.push(0.0);
    }
    words.resize(size, 0.0);
    Ok(words)
}
#[cfg(any(feature = "cuda", test))]
fn decode_snapshot(words: &[f64], count: u32) -> Result<CudaGravitySnapshot, CudaError> {
    let count = usize::try_from(count).map_err(|_| CudaError::BufferLimit)?;
    if words.len() != 8 + count * 8 {
        return Err(CudaError::InvalidGravityInput);
    }
    let failure = match words[7].to_bits() {
        0 => None,
        1 => Some(CudaGravityFailure::SingularPair),
        2 => Some(CudaGravityFailure::NumericalOverflow),
        _ => return Err(CudaError::InvalidGravityInput),
    };
    let bodies = words[8..]
        .chunks_exact(8)
        .map(|b| CudaGravityBody {
            position: [b[0], b[1], b[2]],
            mass: b[3],
            velocity: [b[4], b[5], b[6]],
        })
        .collect::<Vec<_>>();
    if bodies.iter().any(|b| {
        !b.mass.is_finite()
            || b.mass <= 0.0
            || b.position.iter().chain(&b.velocity).any(|v| !v.is_finite())
    }) {
        return Err(CudaError::NumericalOverflow);
    }
    Ok(CudaGravitySnapshot { bodies, failure })
}
fn checked(snapshot: CudaGravitySnapshot) -> Result<Vec<CudaGravityBody>, CudaError> {
    match snapshot.failure {
        None => Ok(snapshot.bodies),
        Some(CudaGravityFailure::SingularPair) => Err(CudaError::SingularPair),
        Some(CudaGravityFailure::NumericalOverflow) => Err(CudaError::NumericalOverflow),
    }
}
#[cfg(test)]
fn decode(words: &[f64], count: u32) -> Result<Vec<CudaGravityBody>, CudaError> {
    checked(decode_snapshot(words, count)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> CudaGravityParameters {
        CudaGravityParameters {
            constant: 1.0,
            softening: 0.25,
            uniform_acceleration: [0.0; 3],
            dt: 1.0 / 1024.0,
        }
    }
    fn body() -> CudaGravityBody {
        CudaGravityBody {
            mass: 1.0,
            position: [0.0; 3],
            velocity: [0.0; 3],
        }
    }

    #[test]
    fn validation_and_error_metadata() {
        let budget = CudaGravityBudget::default();
        assert!(matches!(
            pack(&[], parameters(), budget, usize::MAX),
            Err(CudaError::BufferLimit)
        ));
        assert!(matches!(
            pack(&[body()], parameters(), budget, 255),
            Err(CudaError::BufferLimit)
        ));
        assert!(matches!(
            pack(
                &[CudaGravityBody {
                    mass: 0.0,
                    ..body()
                }],
                parameters(),
                budget,
                usize::MAX
            ),
            Err(CudaError::InvalidGravityInput)
        ));
        let mut words = pack(&[body()], parameters(), budget, 256).unwrap();
        assert_eq!(words.len(), 32);
        assert_eq!(decode(&words[..16], 1).unwrap(), vec![body()]);
        words[7] = f64::from_bits(1);
        assert!(matches!(
            decode(&words[..16], 1),
            Err(CudaError::SingularPair)
        ));
        let snapshot = decode_snapshot(&words[..16], 1).unwrap();
        assert_eq!(snapshot.bodies, vec![body()]);
        assert_eq!(snapshot.failure, Some(CudaGravityFailure::SingularPair));
        words[7] = f64::from_bits(2);
        assert!(matches!(
            decode(&words[..16], 1),
            Err(CudaError::NumericalOverflow)
        ));
        words[7] = 0.0;
        words[8] = f64::NAN;
        assert!(matches!(
            decode(&words[..16], 1),
            Err(CudaError::NumericalOverflow)
        ));
    }

    #[test]
    #[ignore = "requires native clang++; validates CUDA source arithmetic, not NVIDIA execution"]
    fn cuda_gravity_host_parity() {
        let harness = HostHarness::new();
        let bodies: Vec<_> = (0..257_u32)
            .map(|i| CudaGravityBody {
                mass: 1.0 + f64::from(i % 7) * 0.25,
                position: [
                    f64::from(i % 17) * 0.5,
                    f64::from(i / 17) * 0.5,
                    f64::from(i % 3) * 0.25,
                ],
                velocity: [0.01, -0.02, 0.005],
            })
            .collect();
        let p = CudaGravityParameters {
            constant: 0.03,
            ..parameters()
        };
        let (_, output) = harness.run(&bodies, p, 128);
        let actual = decode(&output[..8 + bodies.len() * 8], 257).unwrap();
        let maximum_error = compare_cpu(&bodies, p, 128, &actual);
        let speed = (1.0_f64 / 6.0).sqrt();
        let orbit = [
            CudaGravityBody {
                position: [-1.5, 0.0, 0.0],
                velocity: [0.0, -speed, 0.0],
                ..body()
            },
            CudaGravityBody {
                position: [1.5, 0.0, 0.0],
                velocity: [0.0, speed, 0.0],
                ..body()
            },
        ];
        let p = CudaGravityParameters {
            softening: 0.0,
            dt: 1.0 / 256.0,
            ..parameters()
        };
        let (_, output) = harness.run(&orbit, p, 2560);
        let actual = decode(&output[..24], 2).unwrap();
        compare_cpu(&orbit, p, 2560, &actual);
        for overflow in [false, true] {
            let mut bodies = [
                CudaGravityBody {
                    position: [-1.0, 0.0, 0.0],
                    velocity: [0.875, 0.0, 0.0],
                    ..body()
                },
                CudaGravityBody {
                    position: [1.0, 0.0, 0.0],
                    velocity: [-0.875, 0.0, 0.0],
                    ..body()
                },
            ];
            if overflow {
                bodies[0].position[0] = -1e300;
                bodies[1].position[0] = 1e300;
            }
            let p = CudaGravityParameters {
                softening: 0.0,
                dt: 1.0,
                ..parameters()
            };
            let (before, after) = harness.run(&bodies, p, 2);
            assert_eq!(after[7].to_bits(), if overflow { 2 } else { 1 });
            assert_eq!(
                before[8..24]
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>(),
                after[8..24].iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
        }
        for constant in [0.0, 1.0] {
            let p = CudaGravityParameters {
                constant,
                softening: if constant == 0.0 { 0.0 } else { 0.25 },
                uniform_acceleration: [0.0, -9.0, 0.0],
                ..parameters()
            };
            let input = [body(); 2];
            let (_, output) = harness.run(&input, p, 128);
            compare_cpu(&input, p, 128, &decode(&output[..24], 2).unwrap());
        }
        println!(
            "CUDA f64 source host parity: 257 bodies x128 steps, 2560 orbit steps, max CPU error {maximum_error}, singular/overflow rollback and zero-G/softening; NVIDIA execution unverified"
        );
        std::fs::remove_dir_all(&harness.directory).unwrap();
    }

    struct HostHarness {
        directory: std::path::PathBuf,
        binary: std::path::PathBuf,
    }
    impl HostHarness {
        fn new() -> Self {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap();
            let directory = root
                .join("target")
                .join(format!("cuda-gravity-host-{}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let binary = directory.join("gravity-host");
            let compile = std::process::Command::new("clang++")
                .args([
                    "-std=c++17",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-O2",
                    "-ffp-contract=off",
                ])
                .arg(root.join("tools/cuda/gravity_host.cpp"))
                .arg("-o")
                .arg(&binary)
                .output()
                .unwrap();
            assert!(
                compile.status.success(),
                "{}",
                String::from_utf8_lossy(&compile.stderr)
            );
            Self { directory, binary }
        }
        fn run(
            &self,
            bodies: &[CudaGravityBody],
            parameters: CudaGravityParameters,
            steps: u32,
        ) -> (Vec<f64>, Vec<f64>) {
            let words = pack(
                bodies,
                parameters,
                CudaGravityBudget::default(),
                1024 * 1024,
            )
            .unwrap();
            let input = self.directory.join("input.bin");
            let output = self.directory.join("output.bin");
            std::fs::write(
                &input,
                words
                    .iter()
                    .flat_map(|v| v.to_ne_bytes())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let result = std::process::Command::new(&self.binary)
                .arg(input)
                .arg(&output)
                .arg(steps.to_string())
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let bytes = std::fs::read(output).unwrap();
            assert_eq!(bytes.len(), words.len() * 8);
            let output: Vec<f64> = bytes
                .chunks_exact(8)
                .map(|v| f64::from_ne_bytes(v.try_into().unwrap()))
                .collect();
            (words, output)
        }
    }

    fn compare_cpu(
        bodies: &[CudaGravityBody],
        p: CudaGravityParameters,
        steps: u32,
        actual: &[CudaGravityBody],
    ) -> f64 {
        let mut cpu: Vec<_> = bodies
            .iter()
            .map(|b| physics::gravity::Body {
                mass: b.mass,
                position: b.position,
                velocity: b.velocity,
            })
            .collect();
        let gravity = physics::gravity::Gravity {
            constant: p.constant,
            softening: p.softening,
            uniform_acceleration: p.uniform_acceleration,
        };
        for _ in 0..steps {
            gravity.step(&mut cpu, p.dt).unwrap();
        }
        let mut maximum_error = 0.0_f64;
        for (a, b) in actual.iter().zip(&cpu) {
            assert_eq!(a.mass.to_bits(), b.mass.to_bits());
            for (x, y) in a
                .position
                .iter()
                .chain(&a.velocity)
                .zip(b.position.iter().chain(&b.velocity))
            {
                let error = (x - y).abs();
                maximum_error = maximum_error.max(error);
                assert!(error < 1e-10, "f64 CPU difference {error}");
            }
        }
        maximum_error
    }
}

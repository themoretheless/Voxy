use crate::{CudaCompute, CudaError};
use physics::biomechanics::TissueSearchSnapshot;

fn pack(
    snapshot: &TissueSearchSnapshot,
    direction: &[[f64; 3]],
    limit: usize,
) -> Result<(Vec<f64>, usize), CudaError> {
    let n = snapshot.pinned().len();
    let e = snapshot.elements().len();
    let words = n
        .checked_mul(6)
        .and_then(|x| e.checked_mul(23).and_then(|y| x.checked_add(y)))
        .and_then(|x| x.checked_add(3))
        .ok_or(CudaError::BufferLimit)?;
    let total = n
        .checked_mul(9)
        .and_then(|x| e.checked_mul(35).and_then(|y| x.checked_add(y)))
        .and_then(|x| x.checked_add(3))
        .and_then(|x| x.checked_mul(8))
        .ok_or(CudaError::BufferLimit)?;
    if n == 0 || e == 0 || words > u32::MAX as usize || total > limit {
        return Err(CudaError::BufferLimit);
    }
    if direction.len() != n || direction.iter().flatten().any(|x| !x.is_finite()) {
        return Err(CudaError::InvalidTissueSearchInput);
    }
    let mut data = Vec::with_capacity(words);
    data.extend([n as f64, e as f64]);
    data.extend(snapshot.pinned().iter().map(|&p| if p { 1. } else { 0. }));
    data.extend_from_slice(snapshot.inertia_weights());
    data.extend(direction.iter().flatten().copied());
    let mut refs = vec![Vec::new(); n];
    for (id, element) in snapshot.elements().iter().enumerate() {
        data.extend(element.nodes.map(|i| i as f64));
        data.extend(element.gradients_m_inverse.iter().flatten().copied());
        data.extend([
            element.reference_volume_m3,
            element.shear_pa,
            element.bulk_pa,
        ]);
        for (corner, &node) in element.nodes.iter().enumerate() {
            refs[node].push((4 * id + corner) as f64);
        }
    }
    let mut offset = 0usize;
    data.push(0.);
    for list in &refs {
        offset += list.len();
        data.push(offset as f64);
    }
    for list in refs {
        data.extend(list);
    }
    debug_assert_eq!(data.len(), words);
    Ok((data, total))
}
#[cfg(any(feature = "cuda", test))]
fn decode(snapshot: &TissueSearchSnapshot, words: &[f64]) -> Result<Vec<[f64; 3]>, CudaError> {
    if words.len() != snapshot.pinned().len() * 3 || words.iter().any(|x| !x.is_finite()) {
        return Err(CudaError::NumericalOverflow);
    }
    let result: Vec<_> = words.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect();
    if result
        .iter()
        .zip(snapshot.pinned())
        .any(|(v, &pin)| pin && v.iter().any(|&x| x != 0.))
    {
        return Err(CudaError::InvalidTissueSearchInput);
    }
    Ok(result)
}
impl CudaCompute {
    /// Evaluate the canonical rest-material search metric using two f64 CUDA kernels.
    /// This is not nonlinear force integration. No CPU fallback or state publication.
    /// # Errors
    /// Invalid directions/budgets, driver/compiler failures or nonfinite output.
    #[allow(unsafe_code)]
    pub fn tissue_search_action(
        &self,
        snapshot: &TissueSearchSnapshot,
        direction: &[[f64; 3]],
    ) -> Result<Vec<[f64; 3]>, CudaError> {
        let (packed, bytes) = pack(snapshot, direction, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let reservation = self.allocation_budget.reserve(bytes)?;
            let functions = {
                let mut cached = self
                    .tissue_search
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("tissue_search.cu"),
                        self.compiler_options("tissue_search.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cached = Some([
                        module
                            .load_function("tissue_search_elements")
                            .map_err(CudaError::Driver)?,
                        module
                            .load_function("tissue_search_nodes")
                            .map_err(CudaError::Driver)?,
                    ]);
                }
                cached
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let input = stream.clone_htod(&packed).map_err(CudaError::Driver)?;
            let mut force = match stream.alloc_zeros::<f64>(12 * snapshot.elements().len()) {
                Ok(x) => x,
                Err(e) => {
                    reservation.release((input, functions), &self.context)?;
                    return Err(CudaError::Driver(e));
                }
            };
            let mut output = match stream.alloc_zeros::<f64>(3 * direction.len()) {
                Ok(x) => x,
                Err(e) => {
                    reservation.release((input, force, functions), &self.context)?;
                    return Err(CudaError::Driver(e));
                }
            };
            let result = (|| {
                let config = |count: usize| LaunchConfig {
                    grid_dim: ((count as u32).div_ceil(64), 1, 1),
                    block_dim: (64, 1, 1),
                    shared_mem_bytes: 0,
                };
                // SAFETY: Snapshot topology is immutable and validated by native Body;
                // checked packed ABI bounds all indices. Disjoint element then node writes.
                unsafe {
                    stream
                        .launch_builder(&functions[0])
                        .arg(&input)
                        .arg(&mut force)
                        .launch(config(snapshot.elements().len()))
                }
                .map_err(CudaError::Driver)?;
                unsafe {
                    stream
                        .launch_builder(&functions[1])
                        .arg(&input)
                        .arg(&force)
                        .arg(&mut output)
                        .launch(config(direction.len()))
                }
                .map_err(CudaError::Driver)?;
                stream.clone_dtoh(&output).map_err(CudaError::Driver)
            })();
            reservation.release((input, force, output, functions), &self.context)?;
            let words = result?;
            decode(snapshot, &words)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (packed, bytes);
            Err(CudaError::Disabled)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::biomechanics::{Body, Material};
    fn specimen(count: usize) -> Body {
        let mut points = Vec::new();
        let mut cells = Vec::new();
        let mut pins = Vec::new();
        for i in 0..count {
            let first = points.len();
            let x = i as f64 * 2.;
            points.extend([
                [x, 0., 0.],
                [x + 1., 0., 0.],
                [x, 1., 0.],
                [x, 0., 1.],
                [x, 0., -1.],
            ]);
            pins.extend([i % 2 == 0, false, false, i % 3 == 0, false]);
            cells.push((
                [first, first + 1, first + 2, first + 3],
                Material::from_young_poisson(300. + i as f64, 0.4).unwrap(),
            ));
            cells.push((
                [first, first + 2, first + 1, first + 4],
                Material::from_young_poisson(500. + i as f64, 0.3).unwrap(),
            ));
        }
        let mut body = Body::new(points, pins, cells).unwrap();
        let law = physics::biomechanics::ViscoelasticOgden::new(
            vec![physics::biomechanics::OgdenTerm {
                shear_pa: 100.,
                exponent: 2.,
            }],
            1000.,
            vec![physics::biomechanics::MaxwellBranch {
                shear_pa: 700.,
                relaxation_seconds: 0.2,
            }],
        )
        .unwrap();
        body.set_viscoelastic_ogden(0, law).unwrap();
        body
    }
    #[test]
    fn input_budget_and_canonical_assembly_order() {
        let body = specimen(2);
        let snapshot = body.tissue_search_snapshot(&[2.; 10]).unwrap();
        let (data, bytes) = pack(&snapshot, &[[1.; 3]; 10], usize::MAX).unwrap();
        assert_eq!(bytes, 8 * (3 + 9 * 10 + 35 * 4));
        assert!(pack(&snapshot, &[[1.; 3]; 10], bytes - 1).is_err());
        assert!(pack(&snapshot, &[[1.; 3]; 9], bytes).is_err());
        assert!(pack(&snapshot, &[[f64::NAN; 3]; 10], bytes).is_err());
        let offsets = 2 + 5 * 10 + 19 * 4;
        let refs = offsets + 11;
        assert_eq!(&data[refs..refs + 2], &[0., 4.]);
        assert_eq!(data[offsets + 10], 16.);
    }
    #[cfg(not(feature = "cuda"))]
    #[test]
    fn disabled_backend_does_not_silently_run_native_physics() {
        let body = specimen(1);
        let snapshot = body.tissue_search_snapshot(&[2.; 5]).unwrap();
        let compute = CudaCompute { max_bytes: 4096 };
        assert!(matches!(
            compute.tissue_search_action(&snapshot, &[[1.; 3]; 5]),
            Err(CudaError::Disabled)
        ));
        assert!(matches!(
            compute.tissue_search_action(&snapshot, &[[f64::NAN; 3]; 5]),
            Err(CudaError::InvalidTissueSearchInput)
        ));
    }
    #[test]
    fn rejects_nonfinite_truncated_or_pinned_output_before_publication() {
        let body = specimen(1);
        let snapshot = body.tissue_search_snapshot(&[2.; 5]).unwrap();
        assert!(decode(&snapshot, &[0.; 14]).is_err());
        assert!(decode(&snapshot, &[f64::INFINITY; 15]).is_err());
        let mut words = vec![0.; 15];
        words[0] = 1.;
        assert!(decode(&snapshot, &words).is_err());
        words[0] = 0.;
        words[3] = 2.;
        assert_eq!(decode(&snapshot, &words).unwrap()[1], [2., 0., 0.]);
    }
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires explicit VOXY_TEST_CUDA_DEVICE and physical NVIDIA driver/device"]
    fn physical_cuda_tissue_search_matches_native_and_releases_budget() {
        let ordinal: usize = std::env::var("VOXY_TEST_CUDA_DEVICE")
            .expect("explicit NVIDIA device selection required")
            .parse()
            .unwrap();
        let compute = CudaCompute::new(ordinal, 2 * 1024 * 1024).unwrap();
        eprintln!("TISSUE_CUDA_DEVICE {:?}", compute.capabilities().unwrap());
        for count in [1, 17, 257] {
            let body = specimen(count);
            let n = count * 5;
            let weights: Vec<_> = (0..n).map(|i| 2. + (i % 7) as f64 / 8.).collect();
            let snapshot = body.tissue_search_snapshot(&weights).unwrap();
            let direction: Vec<_> = (0..n).map(|i| [i as f64 * 0.0001, 0.2, -0.1]).collect();
            let expected = body.tissue_search_action(&weights, &direction).unwrap();
            let actual = compute.tissue_search_action(&snapshot, &direction).unwrap();
            assert_eq!(
                actual
                    .iter()
                    .flatten()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .flatten()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>()
            );
            assert_eq!(compute.reserved_device_bytes().unwrap(), 0);
            let mut invalid = direction;
            invalid[0][0] = f64::NAN;
            assert!(compute.tissue_search_action(&snapshot, &invalid).is_err());
            assert_eq!(compute.reserved_device_bytes().unwrap(), 0);
        }
    }
    #[cfg(feature = "cuda")]
    #[derive(Debug)]
    struct CountBackend {
        inner: std::sync::Arc<dyn physics::biomechanics::TissueSearchBackend>,
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    #[cfg(feature = "cuda")]
    #[derive(Debug)]
    struct CountOperation {
        inner: Box<dyn physics::biomechanics::TissueSearchOperation>,
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    #[cfg(feature = "cuda")]
    impl physics::biomechanics::TissueSearchBackend for CountBackend {
        fn prepare(
            &self,
            snapshot: TissueSearchSnapshot,
        ) -> Result<Box<dyn physics::biomechanics::TissueSearchOperation>, &'static str> {
            Ok(Box::new(CountOperation {
                inner: self.inner.prepare(snapshot)?,
                calls: self.calls.clone(),
            }))
        }
    }
    #[cfg(feature = "cuda")]
    impl physics::biomechanics::TissueSearchOperation for CountOperation {
        fn apply(&self, v: &[[f64; 3]]) -> Result<Vec<[f64; 3]>, &'static str> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.inner.apply(v)
        }
    }
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires explicit VOXY_TEST_CUDA_DEVICE and physical NVIDIA driver/device"]
    fn physical_cuda_backend_preserves_implicit_transaction() {
        let ordinal: usize = std::env::var("VOXY_TEST_CUDA_DEVICE")
            .expect("explicit NVIDIA device required")
            .parse()
            .unwrap();
        let compute = std::sync::Arc::new(CudaCompute::new(ordinal, 2 * 1024 * 1024).unwrap());
        eprintln!(
            "TISSUE_CUDA_TRANSACTION_DEVICE {:?}",
            compute.capabilities().unwrap()
        );
        let initial = dynamic_specimen();
        let mut native = initial.clone();
        let mut gpu = initial;
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        gpu.set_tissue_search_backend(Some(std::sync::Arc::new(CountBackend {
            inner: compute.tissue_search_backend(),
            calls: calls.clone(),
        })));
        let a = native
            .step_implicit_with_supports(None, 0.001, 1e-8)
            .unwrap();
        let b = gpu.step_implicit_with_supports(None, 0.001, 1e-8).unwrap();
        assert!(calls.load(std::sync::atomic::Ordering::SeqCst) > 0);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert_eq!(native.body().positions(), gpu.body().positions());
        assert_eq!(native.velocities(), gpu.velocities());
        assert_eq!(
            format!("{:?}", native.diagnostics().unwrap()),
            format!("{:?}", gpu.diagnostics().unwrap())
        );
        assert_eq!(compute.reserved_device_bytes().unwrap(), 0);
    }
    fn dynamic_specimen() -> physics::biomechanics::InertialBody {
        let body = Body::new(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0., 0., -1.],
            ],
            vec![true, false, false, true, false],
            vec![
                (
                    [0, 1, 2, 3],
                    Material::from_young_poisson(300., 0.4).unwrap(),
                ),
                (
                    [0, 2, 1, 4],
                    Material::from_young_poisson(500., 0.3).unwrap(),
                ),
            ],
        )
        .unwrap();
        let pins = body
            .tissue_search_snapshot(&[2.; 5])
            .unwrap()
            .pinned()
            .to_vec();
        let velocity = pins
            .iter()
            .map(|&p| if p { [0.; 3] } else { [0.1, -0.2, 0.3] })
            .collect();
        let mut owner =
            physics::biomechanics::InertialBody::new_with_fixed_supports(body, &[6.; 2], velocity)
                .unwrap();
        owner.set_uniform_acceleration([0., -2., 0.]).unwrap();
        owner
    }
    #[test]
    fn physical_cuda_transaction_fixture_is_admitted_by_native_solver() {
        let mut owner = dynamic_specimen();
        let before = owner.body().positions().to_vec();
        owner
            .step_implicit_with_supports(None, 0.001, 1e-8)
            .unwrap();
        assert_eq!(owner.body().positions()[0], before[0]);
        assert_eq!(owner.body().positions()[3], before[3]);
        assert_ne!(owner.body().positions()[1], before[1]);
    }
    #[cfg(not(feature = "cuda"))]
    #[test]
    fn disabled_cuda_backend_rejects_implicit_step_without_mutating_owner() {
        let mut owner = dynamic_specimen();
        let before = format!("{owner:?}");
        let compute = std::sync::Arc::new(CudaCompute { max_bytes: 4096 });
        owner.set_tissue_search_backend(Some(compute.tissue_search_backend()));
        assert_eq!(
            owner
                .step_implicit_with_supports(None, 0.001, 1e-8)
                .unwrap_err(),
            "CUDA tissue search disabled"
        );
        owner.set_tissue_search_backend(None);
        assert_eq!(format!("{owner:?}"), before);
    }
    #[test]
    #[ignore = "requires clang++; actual CUDA source host arithmetic, not NVIDIA execution"]
    fn cuda_tissue_search_source_matches_canonical_native_operator() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let dir = root
            .join("target")
            .join(format!("cuda-tissue-search-host-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("host");
        let compile = std::process::Command::new("clang++")
            .args([
                "-std=c++17",
                "-O2",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-ffp-contract=off",
            ])
            .arg(root.join("tools/cuda/tissue_search_host.cpp"))
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        for count in [1, 17, 257] {
            let body = specimen(count);
            let n = count * 5;
            let weights: Vec<_> = (0..n).map(|i| 2. + (i % 7) as f64 / 8.).collect();
            let snapshot = body.tissue_search_snapshot(&weights).unwrap();
            for mode in 0..5 {
                let direction: Vec<_> = (0..n)
                    .map(|i| match mode {
                        0 => [0.; 3],
                        1 => [0.1, -0.2, 0.3],
                        2 => [i as f64 * 0.0001, 0.2, -0.1],
                        3 => [
                            (i as f64 * 0.13).sin(),
                            (i as f64 * 0.19).cos(),
                            -0.03 * i as f64,
                        ],
                        _ => [-0., 0., -0.],
                    })
                    .collect();
                let expected = body.tissue_search_action(&weights, &direction).unwrap();
                let (data, _) = pack(&snapshot, &direction, usize::MAX).unwrap();
                let input = dir.join("input.bin");
                let output = dir.join("output.bin");
                std::fs::write(
                    &input,
                    data.iter()
                        .flat_map(|v| v.to_ne_bytes())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
                assert!(
                    std::process::Command::new(&binary)
                        .arg(input)
                        .arg(&output)
                        .status()
                        .unwrap()
                        .success()
                );
                let bytes = std::fs::read(output).unwrap();
                assert_eq!(bytes.len(), n * 3 * 8);
                let actual: Vec<_> = bytes
                    .chunks_exact(8)
                    .map(|b| f64::from_ne_bytes(b.try_into().unwrap()))
                    .collect();
                decode(&snapshot, &actual).unwrap();
                assert_eq!(
                    actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    expected
                        .iter()
                        .flatten()
                        .map(|x| x.to_bits())
                        .collect::<Vec<_>>(),
                    "count={count} mode={mode}"
                );
            }
        }
        eprintln!(
            "CUDA tissue search source exact native parity: 1/17/257 paired tetrahedra specimens x5 vector modes; pins/shared nodes/heterogeneous materials/Ogden-Maxwell moduli; NVIDIA execution unverified"
        );
    }
}

#[derive(Debug)]
struct CudaTissueSearchBackend {
    compute: std::sync::Arc<CudaCompute>,
}
#[cfg(feature = "cuda")]
#[derive(Debug)]
struct CudaTissueSearchOperation {
    compute: std::sync::Arc<CudaCompute>,
    snapshot: TissueSearchSnapshot,
}
#[cfg(feature = "cuda")]
fn backend_failure(error: CudaError) -> &'static str {
    match error {
        CudaError::Disabled => "CUDA tissue search disabled",
        CudaError::DriverUnavailable => "CUDA tissue search driver unavailable",
        CudaError::BufferLimit => "CUDA tissue search allocation budget exceeded",
        CudaError::NumericalOverflow => "CUDA tissue search numerical overflow",
        CudaError::InvalidTissueSearchInput => "invalid CUDA tissue search input or output",
        _ => "CUDA tissue search backend failure",
    }
}
impl physics::biomechanics::TissueSearchBackend for CudaTissueSearchBackend {
    fn prepare(
        &self,
        snapshot: TissueSearchSnapshot,
    ) -> Result<Box<dyn physics::biomechanics::TissueSearchOperation>, &'static str> {
        #[cfg(feature = "cuda")]
        {
            Ok(Box::new(CudaTissueSearchOperation {
                compute: std::sync::Arc::clone(&self.compute),
                snapshot,
            }))
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (&self.compute, snapshot);
            Err("CUDA tissue search disabled")
        }
    }
}
#[cfg(feature = "cuda")]
impl physics::biomechanics::TissueSearchOperation for CudaTissueSearchOperation {
    fn apply(&self, direction: &[[f64; 3]]) -> Result<Vec<[f64; 3]>, &'static str> {
        self.compute
            .tissue_search_action(&self.snapshot, direction)
            .map_err(backend_failure)
    }
}
impl CudaCompute {
    /// Explicit adapter for native implicit rest-material search. No automatic CPU fallback.
    /// The native solver retains nonlinear force/work/contact and transaction admission.
    #[must_use]
    pub fn tissue_search_backend(
        self: &std::sync::Arc<Self>,
    ) -> std::sync::Arc<dyn physics::biomechanics::TissueSearchBackend> {
        std::sync::Arc::new(CudaTissueSearchBackend {
            compute: std::sync::Arc::clone(self),
        })
    }
}

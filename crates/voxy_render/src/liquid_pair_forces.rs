//! Pair force compute transport. Original physics must admit every result.
use crate::{LiquidDensityError, LiquidDensityKernel};
pub const LIQUID_PAIR_FORCES_SHADER: &str = include_str!("liquid_pair_forces.wgsl");
#[derive(Clone, Copy, Debug)]
pub struct LiquidPairForceParameters {
    pub delta: [f64; 3],
    pub relative_velocity: [f64; 3],
    pub normal_difference: [f64; 3],
    pub mass: [f64; 2],
    pub density: [f64; 2],
    pub pressure: [f64; 2],
    pub reference_density: [f64; 2],
    pub smoothing_radius: f64,
    pub viscosity: f64,
    pub surface_strength: f64,
    pub interface_coefficient: f64,
    pub kernel: LiquidDensityKernel,
    pub gas_pressure: bool,
    pub viscous_heating: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidPairForceCandidate {
    pub force: [f64; 3],
    pub viscous_conductance: f64,
}
#[derive(Debug)]
pub struct LiquidPairForcesInput {
    words: Vec<u32>,
    output: usize,
    count: usize,
}
impl LiquidPairForcesInput {
    pub fn new(pairs: &[LiquidPairForceParameters]) -> Result<Self, LiquidDensityError> {
        let count = pairs.len();
        let output = count
            .checked_mul(51)
            .and_then(|n| n.checked_add(2))
            .ok_or(LiquidDensityError::Capacity)?;
        let total = count
            .checked_mul(9)
            .and_then(|n| n.checked_add(output))
            .filter(|&n| n <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        if count == 0 {
            return Err(LiquidDensityError::Shape);
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| LiquidDensityError::Capacity)?;
        words.extend([count as u32, output as u32]);
        for p in pairs {
            if p.mass
                .iter()
                .chain(&p.density)
                .chain(&p.reference_density)
                .chain([&p.smoothing_radius])
                .any(|&x| !x.is_finite() || x <= 0.)
                || p.pressure
                    .iter()
                    .chain([&p.viscosity, &p.surface_strength, &p.interface_coefficient])
                    .any(|&x| !x.is_finite() || x < 0.)
            {
                return Err(LiquidDensityError::Shape);
            }
            let h = p.smoothing_radius;
            let pi = std::f64::consts::PI;
            let coefficients = [
                45. / (pi * h.powi(6)),
                6. * 315. / (64. * pi * h.powi(9)),
                210. / (pi * h.powi(4)),
                32. / (pi * h.powi(3)),
            ];
            for x in p
                .delta
                .into_iter()
                .chain(p.relative_velocity)
                .chain(p.normal_difference)
                .chain(p.mass)
                .chain(p.density)
                .chain(p.pressure)
                .chain(p.reference_density)
                .chain([h, p.viscosity, p.surface_strength, p.interface_coefficient])
                .chain(coefficients)
            {
                if !x.is_finite() {
                    return Err(LiquidDensityError::Shape);
                }
                let hi = x as f32;
                let lo = (x - f64::from(hi)) as f32;
                if !hi.is_finite() || !lo.is_finite() || (x != 0. && hi == 0. && lo == 0.) {
                    return Err(LiquidDensityError::NumericRange);
                }
                words.extend([hi.to_bits(), lo.to_bits()]);
            }
            let mode = match p.kernel {
                LiquidDensityKernel::MassPoly6 => 0,
                LiquidDensityKernel::VolumePoly6 => 1,
                LiquidDensityKernel::VolumeWendland => 2,
            };
            words.push(mode | (u32::from(p.gas_pressure) * 4) | (u32::from(p.viscous_heating) * 8));
        }
        assert_eq!(words.len(), output);
        words.resize(total, u32::MAX);
        Ok(Self {
            words,
            output,
            count,
        })
    }
    pub(crate) fn count(&self) -> usize {
        self.count
    }
    pub(crate) fn force_output_range(&self, index: usize) -> Option<(u64, u64)> {
        (index < self.count).then(|| (((self.output + 9 * index) * 4) as u64, 32))
    }
    pub(crate) fn output_block_range(&self) -> (u64, u64) {
        ((self.output * 4) as u64, (self.count * 36) as u64)
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [(self.count as u32).div_ceil(64), 1, 1]
    }
    pub fn candidate(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<LiquidPairForceCandidate>, LiquidDensityError> {
        Ok(self.audit(bytes)?.into_values())
    }
    pub(crate) fn audit<'a>(
        &self,
        bytes: &'a [u8],
    ) -> Result<AuditedLiquidPairForces<'a>, LiquidDensityError> {
        if bytes.len() != self.words.len() * 4
            || bytes[..self.output * 4] != self.bytes()[..self.output * 4]
        {
            return Err(LiquidDensityError::Output);
        }
        AuditedLiquidPairForces::decode(bytes, self.output, self.count)
    }
}

/// Proof tied to the exact immutable snapshot audited by its typed input owner.
/// Fields and constructor stay private, so consumers cannot swap unaudited bytes.
#[derive(Debug)]
pub(crate) struct AuditedLiquidPairForces<'a> {
    bytes: &'a [u8],
    output: usize,
    count: usize,
    values: Vec<LiquidPairForceCandidate>,
}
impl<'a> AuditedLiquidPairForces<'a> {
    fn decode(bytes: &'a [u8], output: usize, count: usize) -> Result<Self, LiquidDensityError> {
        let values = Self::decode_values(bytes, output, count)?;
        Ok(Self {
            bytes,
            output,
            count,
            values,
        })
    }
    fn decode_values(
        bytes: &[u8],
        output: usize,
        count: usize,
    ) -> Result<Vec<LiquidPairForceCandidate>, LiquidDensityError> {
        let word = |i: usize| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        (0..count)
            .map(|i| {
                let base = output + 9 * i;
                if word(base + 8) != 0 {
                    return Err(LiquidDensityError::Output);
                }
                let mut values = [0.; 4];
                for (axis, value) in values.iter_mut().enumerate() {
                    let hi = f32::from_bits(word(base + 2 * axis));
                    let lo = f32::from_bits(word(base + 2 * axis + 1));
                    if !hi.is_finite() || !lo.is_finite() {
                        return Err(LiquidDensityError::Output);
                    }
                    *value = f64::from(hi) + f64::from(lo);
                }
                if values[3] < 0. {
                    return Err(LiquidDensityError::Output);
                }
                Ok(LiquidPairForceCandidate {
                    force: [values[0], values[1], values[2]],
                    viscous_conductance: values[3],
                })
            })
            .collect()
    }
    pub(crate) fn count(&self) -> usize {
        self.count
    }
    pub(crate) fn force_bytes(&self, index: usize) -> Option<&[u8]> {
        (index < self.count)
            .then(|| &self.bytes[(self.output + 9 * index) * 4..(self.output + 9 * index + 8) * 4])
    }
    pub(crate) fn block(&self) -> &[u8] {
        &self.bytes[self.output * 4..(self.output + 9 * self.count) * 4]
    }
    pub(crate) fn into_values(self) -> Vec<LiquidPairForceCandidate> {
        self.values
    }
}

/// Resident preparation of force inputs from supported normals/density/pressure.
/// Upstream transport and original physics admission remain the caller's owners.
#[derive(Debug)]
pub struct LiquidPreparedPairForcesInput {
    original: LiquidPairForcesInput,
    words: Vec<u32>,
    particles: usize,
    fields: usize,
    pairs: Vec<(usize, usize)>,
}
impl LiquidPreparedPairForcesInput {
    pub fn new(
        parameters: &[LiquidPairForceParameters],
        particles: usize,
        pairs: &[(usize, usize)],
    ) -> Result<Self, LiquidDensityError> {
        let topology = crate::LiquidPairTopology::new(particles, pairs)?;
        Self::with_topology(parameters, &topology)
    }
    pub(crate) fn with_topology(
        parameters: &[LiquidPairForceParameters],
        topology: &crate::LiquidPairTopology,
    ) -> Result<Self, LiquidDensityError> {
        let particles = topology.particle_count();
        let pairs = topology.pairs();
        if pairs.len() != parameters.len() {
            return Err(LiquidDensityError::Shape);
        }
        let original = LiquidPairForcesInput::new(parameters)?;
        let fields = original.words.len();
        let topology = particles
            .checked_mul(11)
            .and_then(|n| n.checked_add(fields))
            .ok_or(LiquidDensityError::Capacity)?;
        let total = pairs
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(topology))
            .and_then(|n| n.checked_add(4))
            .filter(|&n| n <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        let mut words = original.words.clone();
        words
            .try_reserve_exact(total - fields)
            .map_err(|_| LiquidDensityError::Capacity)?;
        words.resize(topology, u32::MAX);
        for &(i, j) in pairs {
            words.extend([i as u32, j as u32]);
        }
        words.extend([
            particles as u32,
            fields as u32,
            topology as u32,
            fields as u32,
        ]);
        Ok(Self {
            original,
            words,
            particles,
            fields,
            pairs: pairs.to_vec(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        self.original.workgroups()
    }
    /// Destination for the upstream output block: normals/status, density, pressure.
    pub fn particle_fields_range(&self) -> (u64, u64) {
        ((self.fields * 4) as u64, (self.particles * 44) as u64)
    }
    pub(crate) fn output_block_range(&self) -> (u64, u64) {
        self.original.output_block_range()
    }
    /// Audit every immutable field and every copied word. The supplied upstream
    /// block must first pass its own typed density/normal/pressure transport audit.
    pub fn candidate(
        &self,
        bytes: &[u8],
        upstream: &[u8],
    ) -> Result<Vec<LiquidPairForceCandidate>, LiquidDensityError> {
        Ok(self.audit(bytes, upstream)?.into_values())
    }
    pub(crate) fn audit<'a>(
        &self,
        bytes: &'a [u8],
        upstream: &[u8],
    ) -> Result<AuditedLiquidPairForces<'a>, LiquidDensityError> {
        if bytes.len() != self.words.len() * 4 || upstream.len() != self.particles * 44 {
            return Err(LiquidDensityError::Output);
        }
        let word = |i: usize| u32::from_le_bytes(upstream[4 * i..4 * i + 4].try_into().unwrap());
        for i in 0..self.particles {
            if word(7 * i + 6) != 0 {
                return Err(LiquidDensityError::Output);
            }
            for index in [
                7 * i,
                7 * i + 2,
                7 * i + 4,
                7 * self.particles + 2 * i,
                9 * self.particles + 2 * i,
            ] {
                let hi = f32::from_bits(word(index));
                let lo = f32::from_bits(word(index + 1));
                let value = f64::from(hi) + f64::from(lo);
                if !hi.is_finite()
                    || !lo.is_finite()
                    || (index == 7 * self.particles + 2 * i && value <= 0.)
                    || (index == 9 * self.particles + 2 * i && value < 0.)
                {
                    return Err(LiquidDensityError::Output);
                }
            }
        }
        let mut expected = self.words.clone();
        expected[self.fields..self.fields + 11 * self.particles].copy_from_slice(
            &upstream
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .collect::<Vec<_>>(),
        );
        for (pair, &(i, j)) in self.pairs.iter().enumerate() {
            let base = 2 + 51 * pair;
            for axis in 0..3 {
                let a = [
                    f32::from_bits(word(7 * i + 2 * axis)),
                    f32::from_bits(word(7 * i + 2 * axis + 1)),
                ];
                let b = [
                    -f32::from_bits(word(7 * j + 2 * axis)),
                    -f32::from_bits(word(7 * j + 2 * axis + 1)),
                ];
                let r = paired_add(a, b);
                expected[base + 12 + 2 * axis] = r[0].to_bits();
                expected[base + 13 + 2 * axis] = r[1].to_bits();
            }
            for (endpoint, particle) in [i, j].into_iter().enumerate() {
                for (source, destination) in [
                    (7 * self.particles + 2 * particle, base + 22 + 2 * endpoint),
                    (9 * self.particles + 2 * particle, base + 26 + 2 * endpoint),
                ] {
                    expected[destination] = word(source);
                    expected[destination + 1] = word(source + 1);
                }
            }
        }
        // Force output is the sole remaining mutable range.
        let start = self.original.output;
        let end = self.fields;
        if &bytes[..start * 4] != bytemuck::cast_slice::<u32, u8>(&expected[..start])
            || &bytes[end * 4..] != bytemuck::cast_slice::<u32, u8>(&expected[end..])
        {
            return Err(LiquidDensityError::Output);
        }
        AuditedLiquidPairForces::decode(&bytes[..end * 4], start, self.original.count)
    }
}
// Match the shader's explicitly rounded operations, including cancellation.
fn paired_add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    let add = |x: f32, y: f32| x.mul_add(1., y);
    let s = add(a[0], b[0]);
    let v = add(s, -a[0]);
    let left = add(a[0], -add(s, -v));
    let right = add(b[0], -v);
    let e = add(add(add(left, right), a[1]), b[1]);
    let hi = add(s, e);
    [hi, add(e, -add(hi, -s))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::liquid::{Formulation, LiquidPairForceInput, LiquidPairForceResult};
    fn upload(p: LiquidPairForceInput) -> LiquidPairForceParameters {
        LiquidPairForceParameters {
            delta: p.delta,
            relative_velocity: p.relative_velocity,
            normal_difference: p.normal_difference,
            mass: p.mass,
            density: p.density,
            pressure: p.pressure,
            reference_density: p.reference_density,
            smoothing_radius: p.smoothing_radius,
            viscosity: p.viscosity,
            surface_strength: p.surface_strength,
            interface_coefficient: p.interface_coefficient,
            kernel: match p.formulation {
                Formulation::MassDensity => LiquidDensityKernel::MassPoly6,
                Formulation::RestVolume => LiquidDensityKernel::VolumePoly6,
                Formulation::RestVolumeWendland => LiquidDensityKernel::VolumeWendland,
            },
            gas_pressure: p.gas_pressure,
            viscous_heating: p.viscous_heating,
        }
    }
    fn fixture() -> LiquidPairForceInput {
        LiquidPairForceInput {
            delta: [0.03, -0.01, 0.02],
            relative_velocity: [0.2, -0.1, 0.3],
            normal_difference: [0.1, 0.2, -0.3],
            mass: [0.002, 0.003],
            density: [1050., 900.],
            pressure: [120., 80.],
            reference_density: [1000., 850.],
            smoothing_radius: 0.1,
            viscosity: 0.03,
            surface_strength: 0.07,
            interface_coefficient: 0.2,
            formulation: Formulation::MassDensity,
            gas_pressure: false,
            viscous_heating: false,
        }
    }
    fn fields(count: usize) -> Vec<u32> {
        let mut words = Vec::new();
        let mut encode = |x: f64| {
            let hi = x as f32;
            words.extend([hi.to_bits(), ((x - f64::from(hi)) as f32).to_bits()]);
        };
        for i in 0..count {
            for axis in 0..3 {
                encode(0.123 + 0.001 * i as f64 + 0.017 * axis as f64);
            }
        }
        // Insert each status after the corresponding normal vector.
        let mut normals = Vec::new();
        for chunk in words.chunks_exact(6) {
            normals.extend_from_slice(chunk);
            normals.push(0);
        }
        for base in [1050., 120.] {
            for i in 0..count {
                let x = base + i as f64;
                let hi = x as f32;
                normals.extend([hi.to_bits(), ((x - f64::from(hi)) as f32).to_bits()]);
            }
        }
        normals
    }
    #[test]
    fn prepared_pair_input_rejects_invalid_topology_and_unwritten_fields() {
        let p = upload(fixture());
        for pairs in [vec![(0, 0)], vec![(1, 0)], vec![(0, 3)]] {
            assert!(LiquidPreparedPairForcesInput::new(&[p], 3, &pairs).is_err());
        }
        assert!(LiquidPreparedPairForcesInput::new(&[p, p], 3, &[(0, 1), (0, 1)]).is_err());
        let input = LiquidPreparedPairForcesInput::new(&[p], 3, &[(0, 1)]).unwrap();
        assert!(
            input
                .candidate(input.bytes(), bytemuck::cast_slice(&fields(3)))
                .is_err()
        );
    }
    #[test]
    #[ignore = "requires actual GPU; resident particle-to-pair preparation and force law"]
    fn gpu_particle_fields_prepare_pair_forces_without_host_read() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let prepare = pollster::block_on(crate::ComputeProgram::with_entry_point(
            &device,
            LIQUID_PAIR_FORCES_SHADER,
            "cs_prepare",
        ))
        .unwrap();
        let force = pollster::block_on(crate::ComputeProgram::new(
            &device,
            LIQUID_PAIR_FORCES_SHADER,
        ))
        .unwrap();
        let n = 70;
        let pairs = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let upstream = fields(n);
        let number = |offset: usize| {
            f64::from(f32::from_bits(upstream[offset]))
                + f64::from(f32::from_bits(upstream[offset + 1]))
        };
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            let owners = pairs
                .iter()
                .map(|&(i, j)| {
                    let mut p = fixture();
                    p.formulation = formulation;
                    p.normal_difference = std::array::from_fn(|axis| {
                        number(7 * i + 2 * axis) - number(7 * j + 2 * axis)
                    });
                    p.density = [number(7 * n + 2 * i), number(7 * n + 2 * j)];
                    p.pressure = [number(9 * n + 2 * i), number(9 * n + 2 * j)];
                    p
                })
                .collect::<Vec<_>>();
            let parameters = owners
                .iter()
                .map(|p| {
                    let mut p = upload(*p);
                    p.normal_difference = [0.; 3];
                    p.density = [1.; 2];
                    p.pressure = [0.; 2];
                    p
                })
                .collect::<Vec<_>>();
            let input = LiquidPreparedPairForcesInput::new(&parameters, n, &pairs).unwrap();
            let (to, length) = input.particle_fields_range();
            // A preceding resident stage supplies the block. The no-op kernel
            // preserves it; real density/normal/pressure integration is separate.
            let upstream_program=pollster::block_on(crate::ComputeProgram::new(&device,
                "@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(1) fn cs_main() {data[0]=data[0];}")).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            let dispatch = crate::compute_chain::encode_chain(
                &device,
                &mut encoder,
                &[
                    crate::compute_chain::ResidentStage {
                        bytes: bytemuck::cast_slice(&upstream),
                        passes: &[(&upstream_program, [1, 1, 1])],
                        copies: &[],
                    },
                    crate::compute_chain::ResidentStage {
                        bytes: input.bytes(),
                        passes: &[(&prepare, input.workgroups()), (&force, input.workgroups())],
                        copies: &[(0, to, length)],
                    },
                ],
            )
            .unwrap();
            let submission = queue.submit([encoder.finish()]);
            let mut read = dispatch.begin_read();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let result = read.try_read().unwrap().unwrap();
            let (source, output) = result.split_at(upstream.len() * 4);
            assert_eq!(source, bytemuck::cast_slice::<u32, u8>(&upstream));
            let candidate = input.candidate(output, source).unwrap();
            for (owner, value) in owners.iter().zip(candidate) {
                owner
                    .admit(
                        LiquidPairForceResult {
                            force: value.force,
                            viscous_conductance: value.viscous_conductance,
                        },
                        1e-12,
                        2e-11,
                    )
                    .unwrap();
            }
            // Cover static fields, dynamic inputs, copied upstream block, topology,
            // metadata and output status without accepting a partial vector.
            for word in [
                0,
                1,
                2,
                14,
                24,
                28,
                input.fields,
                input.fields + 7 * n,
                input.fields + 9 * n,
                input.words.len() - 5,
                input.words.len() - 1,
                input.original.output + 8,
            ] {
                let mut corrupt = output.to_vec();
                corrupt[4 * word] ^= 1;
                assert!(input.candidate(&corrupt, source).is_err(), "word={word}");
            }
            let mut bad_source = source.to_vec();
            bad_source[6 * 4] = 1;
            assert!(input.candidate(output, &bad_source).is_err());
            eprintln!(
                "GPU PAIR PREPARATION particles=70 pairs={} formulation={formulation:?} submissions=1 intermediate_cpu_reads=0 physical_forces_admitted=true upstream_fixture=synthetic",
                pairs.len()
            );
        }
    }
    #[test]
    fn force_shader_validates_and_complete_decode_rejects_corruption() {
        let module = naga::front::wgsl::parse_str(LIQUID_PAIR_FORCES_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let input = LiquidPairForcesInput::new(&[upload(fixture())]).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[input.output..].fill(0);
        assert_eq!(
            input.candidate(bytemuck::cast_slice(&words)).unwrap()[0].force,
            [0.; 3]
        );
        for index in 0..input.output {
            let mut corrupt = words.clone();
            corrupt[index] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        words[input.output + 6] = (-1f32).to_bits();
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_err());
    }
    #[test]
    #[ignore = "requires actual GPU; pair force arithmetic, not a resident fluid simulation"]
    fn gpu_pair_forces_admit_original_model_branches() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            LIQUID_PAIR_FORCES_SHADER,
        ))
        .unwrap();
        let mut owners = Vec::new();
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            for gas_pressure in [false, true] {
                for viscous_heating in [false, true] {
                    for scale in [0., 1e-11, 0.2, 1., 2., 3.] {
                        let mut p = fixture();
                        p.formulation = formulation;
                        p.gas_pressure = gas_pressure;
                        p.viscous_heating = viscous_heating;
                        p.delta = p.delta.map(|x| x * scale);
                        owners.push(p);
                    }
                }
            }
        }
        let input =
            LiquidPairForcesInput::new(&owners.iter().copied().map(upload).collect::<Vec<_>>())
                .unwrap();
        let job = program.create_job(&device, input.bytes()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, input.workgroups()).unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let candidate = input.candidate(&read.try_read().unwrap().unwrap()).unwrap();
        for (index, (owner, candidate)) in owners.iter().zip(candidate).enumerate() {
            let result = LiquidPairForceResult {
                force: candidate.force,
                viscous_conductance: candidate.viscous_conductance,
            };
            owner.admit(result, 1e-12, 2e-11).unwrap_or_else(|error| {
                panic!(
                    "pair {index}: {error:?}, expected {:?}, actual {result:?}",
                    owner.evaluate_native()
                )
            });
        }
        eprintln!(
            "LIQUID GPU PAIR FORCES admitted={} formulations=3 gas_branches=2 heating_branches=2 separations=6",
            owners.len()
        );
        use physics::liquid::{
            Config, Liquid, LiquidComputeBackend, LiquidDensityOperator, Material, Particle,
        };
        let normal_program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            crate::LIQUID_NORMALS_SHADER,
        ))
        .unwrap();
        let accumulation_program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            crate::LIQUID_ACCUMULATION_SHADER,
        ))
        .unwrap();
        let density_program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            crate::LIQUID_DENSITY_SHADER,
        ))
        .unwrap();
        struct Backend<'a> {
            device: &'a wgpu::Device,
            queue: &'a wgpu::Queue,
            program: &'a crate::ComputeProgram,
            normal_program: &'a crate::ComputeProgram,
            accumulation_program: &'a crate::ComputeProgram,
            density_program: &'a crate::ComputeProgram,
        }
        impl LiquidComputeBackend for Backend<'_> {
            fn candidate(&mut self, owner: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
                let particles = owner
                    .particles
                    .iter()
                    .zip(owner.properties)
                    .enumerate()
                    .map(|(i, (p, m))| crate::LiquidDensityParticle {
                        position: p.position,
                        mass: p.mass,
                        rest_density: m.rest_density,
                        carrier_droplet: owner.carrier_droplets.is_some_and(|f| f[i]),
                    })
                    .collect::<Vec<_>>();
                let kernel = match owner.formulation {
                    Formulation::MassDensity => LiquidDensityKernel::MassPoly6,
                    Formulation::RestVolume => LiquidDensityKernel::VolumePoly6,
                    Formulation::RestVolumeWendland => LiquidDensityKernel::VolumeWendland,
                };
                let input = crate::LiquidDensityInput::new(
                    &particles,
                    owner.pairs,
                    owner.smoothing_radius,
                    kernel,
                )
                .ok()?;
                let job = self
                    .density_program
                    .create_job(self.device, input.bytes())
                    .ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                job.encode_step(&mut encoder, input.workgroups()).ok()?;
                let snapshot = job.encode_snapshot(&mut encoder).ok()?;
                let submission = self.queue.submit([encoder.finish()]);
                let mut read = snapshot.begin_read();
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .ok()?;
                input.candidate(&read.try_read().ok()??).ok()
            }
            fn supports_accumulation(&self) -> bool {
                true
            }
            fn accumulation_candidate(
                &mut self,
                owner: &physics::liquid::LiquidAccumulationOperator<'_>,
            ) -> Option<physics::liquid::LiquidAccumulationResult> {
                let masses = owner.particles.iter().map(|p| p.mass).collect::<Vec<_>>();
                let forces = owner
                    .forces
                    .iter()
                    .map(|p| LiquidPairForceCandidate {
                        force: p.force,
                        viscous_conductance: p.viscous_conductance,
                    })
                    .collect::<Vec<_>>();
                let input = crate::LiquidAccumulationInput::new(
                    &masses,
                    owner.pairs,
                    &forces,
                    owner.gravity,
                )
                .ok()?;
                let job = self
                    .accumulation_program
                    .create_job(self.device, input.bytes())
                    .ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                job.encode_step(&mut encoder, input.workgroups()).ok()?;
                let snapshot = job.encode_snapshot(&mut encoder).ok()?;
                let submission = self.queue.submit([encoder.finish()]);
                let mut read = snapshot.begin_read();
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .ok()?;
                let result = input.candidate(&read.try_read().ok()??).ok()?;
                Some(physics::liquid::LiquidAccumulationResult {
                    accelerations: result.accelerations,
                    viscous_rates: result.viscous_rates,
                })
            }
            fn supports_normals(&self) -> bool {
                true
            }
            fn normal_candidates(
                &mut self,
                owner: &physics::liquid::LiquidNormalOperator<'_>,
            ) -> Option<Vec<[f64; 3]>> {
                let particles = owner
                    .particles
                    .iter()
                    .zip(owner.densities)
                    .map(|(p, &density)| {
                        Some(crate::LiquidNormalParticle {
                            position: p.position,
                            mass: p.mass,
                            density,
                            material: u32::try_from(p.material).ok()?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                let input =
                    crate::LiquidNormalsInput::new(&particles, owner.pairs, owner.smoothing_radius)
                        .ok()?;
                let job = self
                    .normal_program
                    .create_job(self.device, input.bytes())
                    .ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                job.encode_step(&mut encoder, input.workgroups()).ok()?;
                let snapshot = job.encode_snapshot(&mut encoder).ok()?;
                let submission = self.queue.submit([encoder.finish()]);
                let mut read = snapshot.begin_read();
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .ok()?;
                input.candidate(&read.try_read().ok()??).ok()
            }
            fn supports_pair_forces(&self) -> bool {
                true
            }
            fn pair_force_candidates(
                &mut self,
                inputs: &[LiquidPairForceInput],
            ) -> Option<Vec<LiquidPairForceResult>> {
                let input = LiquidPairForcesInput::new(
                    &inputs.iter().copied().map(upload).collect::<Vec<_>>(),
                )
                .ok()?;
                let job = self.program.create_job(self.device, input.bytes()).ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                job.encode_step(&mut encoder, input.workgroups()).ok()?;
                let snapshot = job.encode_snapshot(&mut encoder).ok()?;
                let submission = self.queue.submit([encoder.finish()]);
                let mut read = snapshot.begin_read();
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .ok()?;
                Some(
                    input
                        .candidate(&read.try_read().ok()??)
                        .ok()?
                        .into_iter()
                        .map(|p| LiquidPairForceResult {
                            force: p.force,
                            viscous_conductance: p.viscous_conductance,
                        })
                        .collect(),
                )
            }
        }
        let mut backend = Backend {
            device: &device,
            queue: &queue,
            program: &program,
            normal_program: &normal_program,
            accumulation_program: &accumulation_program,
            density_program: &density_program,
        };
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            let particles = (0..8)
                .map(|i| Particle {
                    position: [
                        (i % 2) as f64 * 0.02,
                        ((i / 2) % 2) as f64 * 0.02,
                        (i / 4) as f64 * 0.02,
                    ],
                    velocity: [0.01 * i as f64, -0.02 * i as f64, 0.03],
                    mass: 0.02 * (1. + (i % 3) as f64),
                    material: i % 2,
                })
                .collect();
            let mut native = Liquid::new(
                particles,
                vec![Material::WATER, Material::OIL],
                Config {
                    gravity: [0.; 3],
                    smoothing_radius: 0.05,
                    particle_radius: 0.005,
                    ..Config::default()
                },
            )
            .unwrap();
            native.set_formulation(formulation);
            native.set_surface_strength(0, 0.07).unwrap();
            native.set_surface_strength(1, 0.03).unwrap();
            native.set_interface_penalty(0, 1, 0.02).unwrap();
            let snapshot = native.pair_force_inputs().unwrap();
            assert!(!snapshot.is_empty());
            assert!(snapshot.iter().any(|p| p.pressure.iter().any(|&x| x > 0.)));
            assert!(snapshot.iter().any(|p| p.surface_strength > 0.));
            assert!(snapshot.iter().any(|p| p.interface_coefficient > 0.));
            let mut accelerated = native.clone();
            let mut admissions = 0;
            for _ in 0..3 {
                native.step(1e-6, None).unwrap();
                let (_, stats) = accelerated
                    .step_with_compute_backend(1e-6, None, &mut backend, 1e-12, 2e-11)
                    .unwrap();
                assert!(stats.pair_force_admitted > 0);
                assert_eq!(stats.pair_force_native_fallbacks, 0);
                admissions += stats.pair_force_admitted;
                assert!(stats.normal_admitted > 0);
                assert_eq!(stats.normal_native_fallbacks, 0);
                assert!(stats.accumulation_admitted > 0);
                assert_eq!(stats.accumulation_native_fallbacks, 0);
                assert!(stats.admitted > 0);
                assert_eq!(stats.native_fallbacks, 0);
                for (a, b) in native.particles().iter().zip(accelerated.particles()) {
                    assert_eq!(a.mass, b.mass);
                    assert_eq!(a.material, b.material);
                    for axis in 0..3 {
                        assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                        assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
                    }
                }
            }
            eprintln!(
                "LIQUID GPU PAIR FORCE STEP formulation={formulation:?} steps=3 admitted={admissions} pair_native_fallbacks=0 normal_native_fallbacks=0 density_native_fallbacks=0 accumulation_native_fallbacks=0 stages=density_normals_pair_forces_accumulation_gpu geometry_and_integration=native"
            );
        }
    }
}

//! Original-order particle accumulation; independent threads, no float atomics.
use crate::{LiquidDensityError, LiquidPairForceCandidate};
pub const LIQUID_ACCUMULATION_SHADER: &str = include_str!("liquid_accumulation.wgsl");
#[derive(Debug)]
pub struct LiquidAccumulationInput {
    words: Vec<u32>,
    output: usize,
    count: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidAccumulationCandidate {
    pub accelerations: Vec<[f64; 3]>,
    pub viscous_rates: Vec<f64>,
}
impl LiquidAccumulationInput {
    pub fn new(
        masses: &[f64],
        pairs: &[(usize, usize)],
        forces: &[LiquidPairForceCandidate],
        gravity: [f64; 3],
    ) -> Result<Self, LiquidDensityError> {
        let topology = crate::LiquidPairTopology::new(masses.len(), pairs)?;
        Self::with_topology(masses, &topology, forces, gravity)
    }
    pub(crate) fn with_topology(
        masses: &[f64],
        topology: &crate::LiquidPairTopology,
        forces: &[LiquidPairForceCandidate],
        gravity: [f64; 3],
    ) -> Result<Self, LiquidDensityError> {
        if masses.len() != topology.particle_count() {
            return Err(LiquidDensityError::Shape);
        }
        let pairs = topology.pairs();
        let n = masses.len();
        if n == 0 || pairs.len() != forces.len() {
            return Err(LiquidDensityError::Shape);
        }
        let output = n
            .checked_mul(3)
            .and_then(|v| v.checked_add(pairs.len().checked_mul(12)?))
            .and_then(|v| v.checked_add(13))
            .ok_or(LiquidDensityError::Capacity)?;
        let total = n
            .checked_mul(9)
            .and_then(|v| v.checked_add(output))
            .filter(|&v| v <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        let offsets = 12 + 2 * n + 10 * pairs.len();
        let adjacency = offsets + n + 1;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| LiquidDensityError::Capacity)?;
        words.extend([
            n as u32,
            pairs.len() as u32,
            offsets as u32,
            adjacency as u32,
            output as u32,
            0,
        ]);
        let pack = |x: f64| -> Result<[u32; 2], LiquidDensityError> {
            let hi = x as f32;
            let lo = (x - f64::from(hi)) as f32;
            if !x.is_finite()
                || !hi.is_finite()
                || !lo.is_finite()
                || (x != 0. && hi == 0. && lo == 0.)
            {
                return Err(LiquidDensityError::NumericRange);
            }
            Ok([hi.to_bits(), lo.to_bits()])
        };
        for x in gravity {
            words.extend(pack(x)?);
        }
        for &mass in masses {
            if mass <= 0. {
                return Err(LiquidDensityError::Shape);
            }
            words.extend(pack(mass)?);
        }
        for (&(i, j), force) in pairs.iter().zip(forces) {
            if force.viscous_conductance < 0. {
                return Err(LiquidDensityError::Shape);
            }
            words.extend([i as u32, j as u32]);
            for x in force.force.into_iter().chain([force.viscous_conductance]) {
                words.extend(pack(x)?);
            }
        }
        topology.append_csr(&mut words);
        assert_eq!(words.len(), output);
        words.resize(total, u32::MAX);
        Ok(Self {
            words,
            output,
            count: n,
        })
    }
    pub(crate) fn force_input_range(&self, index: usize) -> Option<(u64, u64)> {
        (index < (self.words[1] as usize))
            .then(|| (((12 + 2 * self.count + 10 * index + 2) * 4) as u64, 32))
    }
    /// Dynamic force slots must match the audited upstream snapshot bit for bit.
    /// All other input words retain the original immutable-prefix audit.
    pub(crate) fn candidate_with_audited_forces(
        &self,
        bytes: &[u8],
        source: &crate::liquid_pair_forces::AuditedLiquidPairForces<'_>,
    ) -> Result<LiquidAccumulationCandidate, LiquidDensityError> {
        if source.count() != self.words[1] as usize {
            return Err(LiquidDensityError::Shape);
        }
        let mut expected = self.words.clone();
        for i in 0..source.count() {
            let from = source.force_bytes(i).ok_or(LiquidDensityError::Shape)?;
            let (to, _) = self.force_input_range(i).ok_or(LiquidDensityError::Shape)?;
            for (word, chunk) in from.chunks_exact(4).enumerate() {
                expected[to as usize / 4 + word] = u32::from_le_bytes(chunk.try_into().unwrap());
            }
        }
        Self {
            words: expected,
            output: self.output,
            count: self.count,
        }
        .candidate(bytes)
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
    ) -> Result<LiquidAccumulationCandidate, LiquidDensityError> {
        if bytes.len() != self.words.len() * 4
            || bytes[..self.output * 4] != self.bytes()[..self.output * 4]
        {
            return Err(LiquidDensityError::Output);
        }
        let word = |i: usize| u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        let mut accelerations = Vec::with_capacity(self.count);
        let mut viscous_rates = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let base = self.output + 9 * i;
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
            accelerations.push([values[0], values[1], values[2]]);
            viscous_rates.push(values[3]);
        }
        Ok(LiquidAccumulationCandidate {
            accelerations,
            viscous_rates,
        })
    }
}
/// One contiguous GPU copy followed by resident scatter into ordered force slots.
#[derive(Debug)]
pub(crate) struct LiquidPreparedAccumulationInput {
    original: LiquidAccumulationInput,
    words: Vec<u32>,
    fields: usize,
}
impl LiquidPreparedAccumulationInput {
    pub(crate) fn new(original: LiquidAccumulationInput) -> Result<Self, LiquidDensityError> {
        let fields = original.words.len();
        let count = original.words[1] as usize;
        let total = count
            .checked_mul(9)
            .and_then(|v| v.checked_add(fields))
            .and_then(|v| v.checked_add(2))
            .filter(|&v| v <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        let mut words = original.words.clone();
        words
            .try_reserve_exact(total - fields)
            .map_err(|_| LiquidDensityError::Capacity)?;
        words.resize(total - 2, u32::MAX);
        words.extend([count as u32, fields as u32]);
        Ok(Self {
            original,
            words,
            fields,
        })
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub(crate) fn workgroups(&self) -> [u32; 3] {
        self.original.workgroups()
    }
    pub(crate) fn prepare_workgroups(&self) -> [u32; 3] {
        [(self.original.words[1]).div_ceil(64), 1, 1]
    }
    pub(crate) fn fields_range(&self) -> (u64, u64) {
        (
            (self.fields * 4) as u64,
            (self.original.words[1] as u64) * 36,
        )
    }
    pub(crate) fn candidate(
        &self,
        bytes: &[u8],
        source: &crate::liquid_pair_forces::AuditedLiquidPairForces<'_>,
    ) -> Result<LiquidAccumulationCandidate, LiquidDensityError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(LiquidDensityError::Output);
        }
        let block = source.block();
        let (to, expected) = self.fields_range();
        if block.len() as u64 != expected
            || &bytes[to as usize..to as usize + block.len()] != block
            || bytes[to as usize + block.len()..] != self.bytes()[to as usize + block.len()..]
        {
            return Err(LiquidDensityError::Output);
        }
        self.original
            .candidate_with_audited_forces(&bytes[..self.fields * 4], source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accumulation_shader_validates_and_rejects_partial_or_changed_results() {
        let module = naga::front::wgsl::parse_str(LIQUID_ACCUMULATION_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let input = LiquidAccumulationInput::new(&[1.], &[], &[], [0.; 3]).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[input.output..].fill(0);
        assert_eq!(
            input
                .candidate(bytemuck::cast_slice(&words))
                .unwrap()
                .accelerations,
            vec![[0.; 3]]
        );
        for i in 0..input.output {
            let mut corrupt = words.clone();
            corrupt[i] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        words[input.output + 6] = (-1f32).to_bits();
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_err());
    }
    #[test]
    #[ignore = "requires actual GPU; original ordered force accumulation"]
    fn gpu_accumulation_preserves_unequal_mass_and_pair_order() {
        use physics::liquid::{
            LiquidAccumulationOperator, LiquidAccumulationResult, LiquidPairForceResult, Particle,
        };
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            LIQUID_ACCUMULATION_SHADER,
        ))
        .unwrap();
        let particles = (0..70)
            .map(|i| Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 0.001 * (1. + (i % 3) as f64),
                material: 0,
            })
            .collect::<Vec<_>>();
        let masses = particles.iter().map(|p| p.mass).collect::<Vec<_>>();
        let mut pairs = (0..70)
            .flat_map(|i| (i + 1..70).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        for reversed in [false, true] {
            if reversed {
                pairs.reverse();
            }
            let forces = pairs
                .iter()
                .map(|&(i, j)| LiquidPairForceResult {
                    force: [
                        (i as f64 - j as f64) * 0.01,
                        (i as f64 + j as f64) * 0.001,
                        -0.02,
                    ],
                    viscous_conductance: 0.03,
                })
                .collect::<Vec<_>>();
            let upload = forces
                .iter()
                .map(|p| LiquidPairForceCandidate {
                    force: p.force,
                    viscous_conductance: p.viscous_conductance,
                })
                .collect::<Vec<_>>();
            let gravity = [0.1, -9.81, 0.2];
            let input = LiquidAccumulationInput::new(&masses, &pairs, &upload, gravity).unwrap();
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
            let owner = LiquidAccumulationOperator {
                particles: &particles,
                pairs: &pairs,
                forces: &forces,
                gravity,
            };
            owner
                .admit(
                    &LiquidAccumulationResult {
                        accelerations: candidate.accelerations,
                        viscous_rates: candidate.viscous_rates,
                    },
                    1e-12,
                    2e-11,
                )
                .unwrap();
            eprintln!(
                "LIQUID GPU ACCUMULATION particles=70 pairs={} reversed={reversed}",
                pairs.len()
            );
        }
    }
}

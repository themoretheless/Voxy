//! Ordered same-material color-field normals; no floating-point atomics.
use crate::LiquidDensityError;
pub const LIQUID_NORMALS_SHADER: &str = include_str!("liquid_normals.wgsl");
#[derive(Clone, Copy, Debug)]
pub struct LiquidNormalParticle {
    pub position: [f64; 3],
    pub mass: f64,
    pub density: f64,
    pub material: u32,
}
#[derive(Debug)]
pub struct LiquidNormalsInput {
    words: Vec<u32>,
    output: usize,
    count: usize,
}
impl LiquidNormalsInput {
    pub fn new(
        particles: &[LiquidNormalParticle],
        pairs: &[(usize, usize)],
        h: f64,
    ) -> Result<Self, LiquidDensityError> {
        let topology = crate::LiquidPairTopology::new(particles.len(), pairs)?;
        Self::with_topology(particles, &topology, h)
    }
    pub(crate) fn with_topology(
        particles: &[LiquidNormalParticle],
        topology: &crate::LiquidPairTopology,
        h: f64,
    ) -> Result<Self, LiquidDensityError> {
        if particles.len() != topology.particle_count() {
            return Err(LiquidDensityError::Shape);
        }
        let pairs = topology.pairs();
        let n = particles.len();
        if n == 0 || !h.is_finite() || h <= 0. {
            return Err(LiquidDensityError::Shape);
        }
        let output = n
            .checked_mul(12)
            .and_then(|v| v.checked_add(pairs.len().checked_mul(4)?))
            .and_then(|v| v.checked_add(9))
            .ok_or(LiquidDensityError::Capacity)?;
        let total = n
            .checked_mul(7)
            .and_then(|v| v.checked_add(output))
            .filter(|&v| v <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        let offsets = 8 + 11 * n + 2 * pairs.len();
        let adjacency = offsets + n + 1;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| LiquidDensityError::Capacity)?;
        words.extend([
            n as u32,
            0,
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
        words.extend(pack(
            -6. * (315. / (64. * std::f64::consts::PI * h.powi(9))) * h.powi(6),
        )?);
        let origin = particles[0].position;
        for p in particles {
            if !p.mass.is_finite() || p.mass <= 0. || !p.density.is_finite() || p.density <= 0. {
                return Err(LiquidDensityError::Shape);
            }
            for (x, base) in p.position.into_iter().zip(origin) {
                words.extend(pack((x - base) / h)?);
            }
            words.extend(pack(p.mass)?);
            words.extend(pack(p.density)?);
            words.push(p.material);
        }
        for &(i, j) in pairs {
            words.extend([i as u32, j as u32]);
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
    /// Preserve each particle's original boundary/image contribution order.
    pub(crate) fn with_support(
        particles: &[LiquidNormalParticle],
        topology: &crate::LiquidPairTopology,
        h: f64,
        support: &[(usize, f64)],
    ) -> Result<Self, LiquidDensityError> {
        let mut input = Self::with_topology(particles, topology, h)?;
        let n = particles.len();
        let mut rows = vec![Vec::new(); n];
        for &(i, x) in support {
            if i >= n || !x.is_finite() || x < 0. {
                return Err(LiquidDensityError::Shape);
            }
            let hi = x as f32;
            let lo = (x - f64::from(hi)) as f32;
            if !hi.is_finite() || !lo.is_finite() || (x != 0. && hi == 0. && lo == 0.) {
                return Err(LiquidDensityError::NumericRange);
            }
            rows[i].push([hi.to_bits(), lo.to_bits()]);
        }
        let extra = n
            .checked_add(1)
            .and_then(|v| v.checked_add(support.len().checked_mul(2)?))
            .ok_or(LiquidDensityError::Capacity)?;
        let output = input
            .output
            .checked_add(extra)
            .ok_or(LiquidDensityError::Capacity)?;
        let total = output
            .checked_add(n.checked_mul(9).ok_or(LiquidDensityError::Capacity)?)
            .filter(|&v| v <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        input.words.truncate(input.output);
        input
            .words
            .try_reserve_exact(total - input.words.len())
            .map_err(|_| LiquidDensityError::Capacity)?;
        input.words[1] = input.output as u32;
        input.words[4] = output as u32;
        input.words[5] = (output + 7 * n) as u32;
        let mut cursor = 0;
        input.words.push(0);
        for row in &rows {
            cursor += row.len();
            input.words.push(cursor as u32);
        }
        for row in rows {
            for value in row {
                input.words.extend(value);
            }
        }
        assert_eq!(input.words.len(), output);
        input.words.resize(total, u32::MAX);
        input.output = output;
        Ok(input)
    }
    pub(crate) fn with_pressure(
        mut self,
        laws: &[crate::LiquidPressureParameters],
    ) -> Result<Self, LiquidDensityError> {
        if laws.len() != self.count || self.words[1] == 0 {
            return Err(LiquidDensityError::Shape);
        }
        let output = self
            .output
            .checked_add(
                self.count
                    .checked_mul(5)
                    .ok_or(LiquidDensityError::Capacity)?,
            )
            .ok_or(LiquidDensityError::Capacity)?;
        let total = output
            .checked_add(
                self.count
                    .checked_mul(11)
                    .ok_or(LiquidDensityError::Capacity)?,
            )
            .filter(|&v| v <= 16 * 1024 * 1024)
            .ok_or(LiquidDensityError::Capacity)?;
        self.words.truncate(self.output);
        self.words
            .try_reserve_exact(total - self.words.len())
            .map_err(|_| LiquidDensityError::Capacity)?;
        for law in laws {
            if !law.rest_density.is_finite()
                || law.rest_density <= 0.
                || !law.sound_speed.is_finite()
                || law.sound_speed <= 0.
                || law
                    .heat_capacity_ratio
                    .is_some_and(|x| !x.is_finite() || x <= 1.)
            {
                return Err(LiquidDensityError::Shape);
            }
            let coefficient = law.sound_speed.powi(2) / law.heat_capacity_ratio.unwrap_or(1.);
            for x in [law.rest_density, coefficient] {
                let hi = x as f32;
                let lo = (x - f64::from(hi)) as f32;
                if !hi.is_finite() || !lo.is_finite() || (x != 0. && hi == 0. && lo == 0.) {
                    return Err(LiquidDensityError::NumericRange);
                }
                self.words.extend([hi.to_bits(), lo.to_bits()]);
            }
            self.words
                .push(u32::from(law.heat_capacity_ratio.is_some()));
        }
        assert_eq!(self.words.len(), output);
        self.words[4] = output as u32;
        self.words[5] = (output + 7 * self.count) as u32;
        self.words.resize(total, u32::MAX);
        self.output = output;
        Ok(self)
    }
    pub(crate) fn pressures_after_density_audit(
        &self,
        bytes: &[u8],
    ) -> Result<Option<Vec<f64>>, LiquidDensityError> {
        let support = self.words[1] as usize;
        if support == 0 {
            return Ok(None);
        }
        let laws = support + self.count + 1 + 2 * self.words[support + self.count] as usize;
        if self.output != laws + 5 * self.count {
            return Ok(None);
        }
        if bytes.len() != self.words.len() * 4 {
            return Err(LiquidDensityError::Output);
        }
        let base = self.words[5] as usize + 2 * self.count;
        (0..self.count)
            .map(|i| {
                let read = |word: usize| {
                    f32::from_bits(u32::from_le_bytes(
                        bytes[word * 4..word * 4 + 4].try_into().unwrap(),
                    ))
                };
                let hi = read(base + 2 * i);
                let lo = read(base + 2 * i + 1);
                let value = f64::from(hi) + f64::from(lo);
                if !hi.is_finite() || !lo.is_finite() || value < 0. {
                    return Err(LiquidDensityError::Output);
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }
    pub(crate) fn pressure_fields_range(&self) -> Result<(u64, u64), LiquidDensityError> {
        let support = self.words[1] as usize;
        if support == 0 {
            return Err(LiquidDensityError::Shape);
        }
        let laws = support + self.count + 1 + 2 * self.words[support + self.count] as usize;
        if self.output != laws + 5 * self.count {
            return Err(LiquidDensityError::Shape);
        }
        Ok(((self.output * 4) as u64, (self.count * 44) as u64))
    }
    pub(crate) fn density_input_range(&self, index: usize) -> Option<(u64, u64)> {
        (index < self.count).then(|| (((8 + 11 * index + 8) * 4) as u64, 8))
    }
    pub(crate) fn candidate_with_gpu_density(
        &self,
        bytes: &[u8],
        source: &crate::LiquidDensityInput,
        source_bytes: &[u8],
    ) -> Result<(Vec<f64>, Vec<[f64; 3]>), LiquidDensityError> {
        if source.count() != self.count {
            return Err(LiquidDensityError::Shape);
        }
        source.candidate(source_bytes)?;
        let mut expected = self.words.clone();
        for i in 0..self.count {
            let (from, _) = source
                .density_output_range(i)
                .ok_or(LiquidDensityError::Shape)?;
            let (to, _) = self
                .density_input_range(i)
                .ok_or(LiquidDensityError::Shape)?;
            for word in 0..2 {
                let offset = from as usize + 4 * word;
                expected[to as usize / 4 + word] =
                    u32::from_le_bytes(source_bytes[offset..offset + 4].try_into().unwrap());
            }
        }
        let normals = Self {
            words: expected,
            output: self.output,
            count: self.count,
        }
        .candidate(bytes)?;
        let base = self.words[5] as usize;
        if base == 0 {
            return Err(LiquidDensityError::Shape);
        }
        let mut densities = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let read = |word: usize| {
                f32::from_bits(u32::from_le_bytes(
                    bytes[word * 4..word * 4 + 4].try_into().unwrap(),
                ))
            };
            let hi = read(base + 2 * i);
            let lo = read(base + 2 * i + 1);
            let value = f64::from(hi) + f64::from(lo);
            if !hi.is_finite() || !lo.is_finite() || value <= 0. {
                return Err(LiquidDensityError::Output);
            }
            densities.push(value);
        }
        Ok((densities, normals))
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [(self.count as u32).div_ceil(64), 1, 1]
    }
    pub fn candidate(&self, bytes: &[u8]) -> Result<Vec<[f64; 3]>, LiquidDensityError> {
        if bytes.len() != self.words.len() * 4
            || bytes[..self.output * 4] != self.bytes()[..self.output * 4]
        {
            return Err(LiquidDensityError::Output);
        }
        let word = |i: usize| u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        (0..self.count)
            .map(|i| {
                let base = self.output + 7 * i;
                if word(base + 6) != 0 {
                    return Err(LiquidDensityError::Output);
                }
                let mut normal = [0.; 3];
                for (axis, value) in normal.iter_mut().enumerate() {
                    let hi = f32::from_bits(word(base + 2 * axis));
                    let lo = f32::from_bits(word(base + 2 * axis + 1));
                    if !hi.is_finite() || !lo.is_finite() {
                        return Err(LiquidDensityError::Output);
                    }
                    *value = f64::from(hi) + f64::from(lo);
                }
                Ok(normal)
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_shader_validates_and_decoding_requires_complete_unchanged_input() {
        let module = naga::front::wgsl::parse_str(LIQUID_NORMALS_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let input = LiquidNormalsInput::new(
            &[LiquidNormalParticle {
                position: [0.; 3],
                mass: 1.,
                density: 1000.,
                material: 0,
            }],
            &[],
            0.2,
        )
        .unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[input.output..].fill(0);
        assert_eq!(
            input.candidate(bytemuck::cast_slice(&words)).unwrap(),
            vec![[0.; 3]]
        );
        for i in 0..input.output {
            let mut corrupt = words.clone();
            corrupt[i] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        words[input.output] = f32::NAN.to_bits();
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_err());
    }
    #[test]
    #[ignore = "requires actual GPU; original color-field normal admission"]
    fn gpu_normals_preserve_material_boundaries_and_pair_order() {
        use physics::liquid::{LiquidNormalOperator, Particle};
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program =
            pollster::block_on(crate::ComputeProgram::new(&device, LIQUID_NORMALS_SHADER)).unwrap();
        let particles = (0..70)
            .map(|i| Particle {
                position: [1e8 + (i % 7) as f64 * 0.01, (i / 7) as f64 * 0.01, 0.],
                velocity: [0.; 3],
                mass: 0.001 * (1. + (i % 3) as f64),
                material: i % 2,
            })
            .collect::<Vec<_>>();
        let densities = (0..70).map(|i| 850. + i as f64).collect::<Vec<_>>();
        let mut pairs = (0..70)
            .flat_map(|i| (i + 1..70).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let upload = particles
            .iter()
            .zip(&densities)
            .map(|(p, &density)| LiquidNormalParticle {
                position: p.position,
                mass: p.mass,
                density,
                material: p.material as u32,
            })
            .collect::<Vec<_>>();
        for reversed in [false, true] {
            if reversed {
                pairs.reverse();
            }
            let input = LiquidNormalsInput::new(&upload, &pairs, 0.2).unwrap();
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
            let owner = LiquidNormalOperator {
                particles: &particles,
                pairs: &pairs,
                densities: &densities,
                smoothing_radius: 0.2,
            };
            owner.admit(&candidate, 1e-12, 2e-11).unwrap();
            let maximum_absolute = candidate
                .iter()
                .zip(owner.evaluate_native().unwrap())
                .flat_map(|(a, b)| a.iter().zip(b).map(|(&a, b)| (a - b).abs()))
                .fold(0f64, f64::max);
            eprintln!(
                "LIQUID GPU NORMALS particles=70 pairs={} reversed={reversed} maximum_absolute_error={maximum_absolute:e}",
                pairs.len()
            );
        }
    }
}

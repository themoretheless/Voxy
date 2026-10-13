//! Experimental complete resident coordinate candidate; no physical admission here.
use super::{JointDotError, pack, rescale};

/// Prototype input for one complete unilateral coordinate operator. Unsupported
/// shapes/ranges reject the complete request; no rows or coefficients are dropped.
#[derive(Debug)]
pub struct ExperimentalResidentContactInput {
    words: Vec<u32>,
    total_words: usize,
    immutable_end: usize,
    reactions: usize,
    state: usize,
    status: usize,
    rows: usize,
    width: usize,
    column_scale: f64,
    state_scale: f64,
}
/// A GPU candidate only. The original f64 physical owner must validate it.
pub const EXPERIMENTAL_RESIDENT_CONTACT_SHADER: &str = include_str!("joint_contact_active.wgsl");
/// Experimental seeded variant; requires 1024 compute invocations per workgroup.
pub const EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER: &str =
    include_str!("joint_contact_active_seeded_1024.wgsl");
/// Experimental seeded variant for 64-lane workgroups.
pub const EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER: &str =
    include_str!("joint_contact_active_seeded_64.wgsl");
/// Select from enabled device limits, never from adapter maxima. Returning None
/// keeps unsupported devices out of pipeline creation and preserves fallback.
pub fn experimental_seeded_resident_contact_shader(limits: &wgpu::Limits) -> Option<&'static str> {
    let lanes = limits
        .max_compute_workgroup_size_x
        .min(limits.max_compute_invocations_per_workgroup);
    if lanes >= 1024 && limits.max_compute_workgroup_storage_size >= 16640 {
        Some(EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER)
    } else if lanes >= 64 && limits.max_compute_workgroup_storage_size >= 8960 {
        Some(EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER)
    } else {
        None
    }
}

impl ExperimentalResidentContactInput {
    /// Cold active-set construction, retaining all original coordinates.
    pub fn new(
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = columns.first().map_or(0, Vec::len);
        if rows == 0
            || width == 0
            || bounds.len() != rows
            || !tolerance.is_finite()
            || tolerance <= 0.
            || columns
                .iter()
                .any(|c| c.len() != width || c.iter().any(|v| !v.is_finite()))
            || bounds.iter().any(|v| !v.is_finite())
        {
            return Err(JointDotError::Shape);
        }
        if rows > 1024 {
            return Err(JointDotError::Capacity);
        }
        let matrix = rows
            .checked_mul(width)
            .and_then(|n| n.checked_mul(2))
            .ok_or(JointDotError::Capacity)?;
        let square = rows
            .checked_mul(rows)
            .and_then(|n| n.checked_mul(2))
            .ok_or(JointDotError::Capacity)?;
        let immutable_end = 4usize
            .checked_add(matrix)
            .and_then(|n| n.checked_add(rows * 2 + 2))
            .ok_or(JointDotError::Capacity)?;
        let z = immutable_end
            .checked_add(matrix)
            .and_then(|n| n.checked_add(square))
            .ok_or(JointDotError::Capacity)?;
        let reactions = z
            .checked_add(rows * 4)
            .and_then(|n| n.checked_add(width.checked_mul(2)?))
            .ok_or(JointDotError::Capacity)?;
        let state = reactions
            .checked_add(rows * 2)
            .ok_or(JointDotError::Capacity)?;
        let status = state
            .checked_add(width.checked_mul(2).ok_or(JointDotError::Capacity)?)
            .and_then(|n| n.checked_add(rows * 2))
            .ok_or(JointDotError::Capacity)?;
        let total = status
            .checked_add(1)
            .filter(|&n| n <= 16 * 1024 * 1024 && n <= u32::MAX as usize)
            .ok_or(JointDotError::Capacity)?;
        let column_scale = columns.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
        let bound_scale = bounds.iter().map(|v| v.abs()).fold(tolerance, f64::max);
        let state_scale = bound_scale / column_scale;
        if !column_scale.is_finite()
            || column_scale <= 0.
            || !column_scale.recip().is_finite()
            || !state_scale.is_finite()
            || state_scale <= 0.
        {
            return Err(JointDotError::NumericRange);
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| JointDotError::Capacity)?;
        words.extend([width as u32, rows as u32, 512u32.max(rows as u32 + 1), 0]);
        for v in columns.iter().flatten() {
            let normalized = v / column_scale;
            if *v != 0. && normalized == 0. {
                return Err(JointDotError::NumericRange);
            }
            words.extend(pack(normalized)?);
        }
        for &v in bounds {
            let normalized = v / bound_scale;
            if v != 0. && normalized == 0. {
                return Err(JointDotError::NumericRange);
            }
            words.extend(pack(normalized)?);
        }
        words.extend(pack(tolerance / bound_scale)?);
        assert_eq!(words.len(), immutable_end);
        words.resize(total, 0);
        words[status] = u32::MAX;
        Ok(Self {
            words,
            total_words: total,
            immutable_end,
            reactions,
            state,
            status,
            rows,
            width,
            column_scale,
            state_scale,
        })
    }
    /// Experimental hint upload for a shader that explicitly supports header[3].
    /// Hints never replace original coefficients or physical admission. Invalid
    /// or unrepresentable hints retain the complete cold-start input.
    pub fn new_seeded(
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
        seeds: &[f64],
    ) -> Result<Self, JointDotError> {
        let mut input = Self::new(columns, bounds, tolerance)?;
        if seeds.len() != input.rows
            || seeds.iter().any(|v| !v.is_finite() || *v < 0.)
            || !seeds.iter().any(|&v| v > 0.)
        {
            return Ok(input);
        }
        let packed = seeds
            .iter()
            .map(|&value| {
                let normalized = rescale(value, input.column_scale, 1. / input.state_scale);
                if value > 0. && normalized == 0. {
                    return Err(JointDotError::NumericRange);
                }
                pack(normalized)
            })
            .collect::<Result<Vec<_>, _>>();
        let Ok(packed) = packed else {
            return Ok(input);
        };
        for (i, value) in packed.into_iter().enumerate() {
            input.words[input.reactions + 2 * i..input.reactions + 2 * i + 2]
                .copy_from_slice(&value);
        }
        input.words[3] = 1;
        Ok(input)
    }
    /// Whether this input contains experimental initial reaction hints.
    pub fn has_seed_hint(&self) -> bool {
        self.words[3] == 1
    }
    /// Complete immutable upload plus private resident scratch storage.
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Decode only completed, finite output. This is not original KKT/physics admission.
    pub fn candidate(&self, bytes: &[u8]) -> Result<(Vec<f64>, Vec<f64>), JointDotError> {
        if bytes.len() != self.total_words * 4 {
            return Err(JointDotError::Output);
        }
        // Read the admitted ABI in place. Copying every private QR scratch word
        // into another Vec made CPU memory proportional to GPU workspace size.
        // The full immutable operator audit remains bit-exact and endian-explicit.
        let word = |offset: usize| {
            u32::from_le_bytes(bytes[offset * 4..offset * 4 + 4].try_into().unwrap())
        };
        self.decode_candidate(word)
    }
    fn decode_candidate(
        &self,
        word: impl Fn(usize) -> u32,
    ) -> Result<(Vec<f64>, Vec<f64>), JointDotError> {
        if (0..self.immutable_end).any(|i| word(i) != self.words[i]) || word(self.status) != 0 {
            return Err(JointDotError::Output);
        }
        let scalar = |offset| {
            let hi = f32::from_bits(word(offset)) as f64;
            let lo = f32::from_bits(word(offset + 1)) as f64;
            if hi.is_finite() && lo.is_finite() {
                Ok(hi + lo)
            } else {
                Err(JointDotError::Output)
            }
        };
        let state = (0..self.width)
            .map(|i| {
                let v = rescale(scalar(self.state + 2 * i)?, self.state_scale, 1.);
                v.is_finite().then_some(v).ok_or(JointDotError::Output)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reactions = (0..self.rows)
            .map(|i| {
                let v = rescale(
                    scalar(self.reactions + 2 * i)?,
                    self.state_scale,
                    1. / self.column_scale,
                );
                (v.is_finite() && v >= 0.)
                    .then_some(v)
                    .ok_or(JointDotError::Output)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((state, reactions))
    }
}

/// Independent complete operators dispatched concurrently in disjoint workgroups.
/// The header and every immutable original operator are audited on readback.
#[derive(Debug)]
pub struct ExperimentalResidentContactBatch {
    inputs: Vec<ExperimentalResidentContactInput>,
    words: Vec<u32>,
    offsets: Vec<usize>,
    header_end: usize,
}
impl ExperimentalResidentContactBatch {
    /// All-or-nothing packing; never partitions a physical operator or drops rows.
    pub fn new(mut inputs: Vec<ExperimentalResidentContactInput>) -> Result<Self, JointDotError> {
        if inputs.is_empty() || inputs.len() > 65535 {
            return Err(JointDotError::Shape);
        }
        let header_end = 4usize
            .checked_add(inputs.len())
            .ok_or(JointDotError::Capacity)?;
        let total = inputs
            .iter()
            .try_fold(header_end, |n, i| n.checked_add(i.words.len()))
            .filter(|&n| n <= 16 * 1024 * 1024)
            .ok_or(JointDotError::Capacity)?;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| JointDotError::Capacity)?;
        words.extend([u32::MAX, inputs.len() as u32, header_end as u32, 0]);
        let mut offsets = Vec::with_capacity(inputs.len());
        let mut offset = header_end;
        for input in &inputs {
            words.push(offset as u32);
            offsets.push(offset);
            offset += input.words.len();
        }
        for input in &mut inputs {
            words.extend_from_slice(&input.words);
            // The batch owns the complete upload. Keep only the immutable
            // certificate per input, not a second copy of private QR scratch.
            input.words = input.words[..input.immutable_end].to_vec();
        }
        Ok(Self {
            inputs,
            words,
            offsets,
            header_end,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [self.inputs.len() as u32, 1, 1]
    }
    /// Ordered byte ranges for an audited readback without private QR scratch.
    /// Concatenate these ranges exactly; every original immutable word remains.
    pub fn candidate_readback_ranges(&self) -> Vec<std::ops::Range<u64>> {
        let mut ranges = Vec::with_capacity(1 + 2 * self.inputs.len());
        ranges.push(0..(self.header_end * 4) as u64);
        for (input, &offset) in self.inputs.iter().zip(&self.offsets) {
            ranges.push((offset * 4) as u64..((offset + input.immutable_end) * 4) as u64);
            ranges.push(
                ((offset + input.reactions) * 4) as u64..((offset + input.total_words) * 4) as u64,
            );
        }
        ranges
    }
    /// Same publication gates as full readback; original physical admission follows.
    pub fn candidate_from_compact_readback(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<(Vec<f64>, Vec<f64>)>, JointDotError> {
        let expected = self.header_end * 4
            + self
                .inputs
                .iter()
                .map(|i| (i.immutable_end + i.total_words - i.reactions) * 4)
                .sum::<usize>();
        if bytes.len() != expected
            || bytes[..self.header_end * 4] != self.bytes()[..self.header_end * 4]
        {
            return Err(JointDotError::Output);
        }
        let mut cursor = self.header_end * 4;
        self.inputs
            .iter()
            .map(|input| {
                let length = (input.immutable_end + input.total_words - input.reactions) * 4;
                let current = &bytes[cursor..cursor + length];
                cursor += length;
                input.decode_candidate(|index| {
                    let compact = if index < input.immutable_end {
                        index
                    } else {
                        input.immutable_end + index - input.reactions
                    };
                    u32::from_le_bytes(current[compact * 4..compact * 4 + 4].try_into().unwrap())
                })
            })
            .collect()
    }

    /// Publish a complete batch only after all operators pass ABI/output checks.
    /// Original f64 physical admission is still required by each owner.
    pub fn candidate(&self, bytes: &[u8]) -> Result<Vec<(Vec<f64>, Vec<f64>)>, JointDotError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(JointDotError::Output);
        }
        if bytes[..self.header_end * 4] != self.bytes()[..self.header_end * 4] {
            return Err(JointDotError::Output);
        }
        self.inputs
            .iter()
            .zip(&self.offsets)
            .map(|(input, &offset)| {
                input.candidate(&bytes[offset * 4..(offset + input.total_words) * 4])
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_variant_uses_enabled_limits_and_rejects_insufficient_lanes() {
        for (x, invocations, expected) in [
            (1024, 1024, 1024),
            (1023, 1024, 64),
            (1024, 64, 64),
            (64, 1024, 64),
            (64, 64, 64),
            (63, 1024, 0),
            (1024, 63, 0),
        ] {
            let mut limits = wgpu::Limits::default();
            limits.max_compute_workgroup_size_x = x;
            limits.max_compute_invocations_per_workgroup = invocations;
            limits.max_compute_workgroup_storage_size = 32768;
            let selected = experimental_seeded_resident_contact_shader(&limits);
            match expected {
                1024 => assert_eq!(selected, Some(EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER)),
                64 => assert_eq!(
                    selected,
                    Some(EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER)
                ),
                _ => assert_eq!(selected, None),
            }
        }
    }

    #[test]
    fn seeded_variant_checks_workgroup_storage_before_pipeline_creation() {
        let mut limits = wgpu::Limits::default();
        limits.max_compute_workgroup_size_x = 1024;
        limits.max_compute_invocations_per_workgroup = 1024;
        limits.max_compute_workgroup_storage_size = 16384;
        assert_eq!(
            experimental_seeded_resident_contact_shader(&limits),
            Some(EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER)
        );
        limits.max_compute_workgroup_storage_size = 8192;
        assert_eq!(experimental_seeded_resident_contact_shader(&limits), None);
    }

    #[test]
    #[ignore = "requires 1024-lane GPU and seeded shader fixture; coordinate gate only"]
    fn gpu_seeded_resident_candidate_uses_current_columns_and_releases_hints() {
        let source = std::env::var_os("VOXY_HAIR_RESIDENT_SEEDED_SHADER_FIXTURE")
            .map(|path| std::fs::read_to_string(path).unwrap())
            .unwrap_or_else(|| EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER.to_owned());
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let mut descriptor = wgpu::DeviceDescriptor::default();
        descriptor.required_limits.max_compute_workgroup_size_x = 1024;
        descriptor
            .required_limits
            .max_compute_invocations_per_workgroup = 1024;
        descriptor
            .required_limits
            .max_compute_workgroup_storage_size = 16640;
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(&device, &source)).unwrap();
        let solve = |input: &ExperimentalResidentContactInput| {
            let job = program.create_job(&device, input.bytes()).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
            let snapshot = job.encode_snapshot(&mut encoder).unwrap();
            let submission = queue.submit([encoder.finish()]);
            let mut read = snapshot.begin_read();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            input.candidate(&read.try_read().unwrap().unwrap())
        };
        let columns = vec![vec![1., 0.], vec![0., 1.]];
        for (bounds, hints) in [
            ([1., 2.], [1., 2.]),
            ([-1., 2.], [1., 2.]),
            ([1., 3.], [1., 0.]),
            ([1., 2.], [20., 10.]),
        ] {
            let warm =
                ExperimentalResidentContactInput::new_seeded(&columns, &bounds, 1e-12, &hints)
                    .unwrap();
            let actual = solve(&warm).unwrap();
            let expected = bounds.map(|v| v.max(0.));
            for (a, b) in actual
                .0
                .iter()
                .chain(&actual.1)
                .zip(expected.iter().cycle())
            {
                assert!((a - b).abs() <= 1e-12);
            }
        }
        let changed = vec![vec![2., 0.], vec![0., 4.]];
        let actual = solve(
            &ExperimentalResidentContactInput::new_seeded(&changed, &[2., 8.], 1e-12, &[1., 2.])
                .unwrap(),
        )
        .unwrap();
        for (a, b) in actual.0.iter().chain(&actual.1).zip([1., 2., 0.5, 0.5]) {
            assert!((a - b).abs() <= 1e-12);
        }
        let mut warm =
            ExperimentalResidentContactInput::new_seeded(&columns, &[1., 2.], 1e-12, &[1., 2.])
                .unwrap();
        warm.words[2] = 1;
        assert!(
            solve(&warm).is_ok(),
            "valid hints not consumed in one iteration"
        );
        let mut cold = ExperimentalResidentContactInput::new(&columns, &[1., 2.], 1e-12).unwrap();
        cold.words[2] = 1;
        assert!(solve(&cold).is_err());
        let dependent = vec![vec![1.], vec![1.]];
        let warm =
            ExperimentalResidentContactInput::new_seeded(&dependent, &[1., 1.], 1e-12, &[1., 1.])
                .unwrap();
        assert!(
            solve(&warm).is_err(),
            "dependent hint basis published a partial solution"
        );
        assert!(
            solve(&ExperimentalResidentContactInput::new(&dependent, &[1., 1.], 1e-12).unwrap())
                .is_ok()
        );
    }
    #[test]
    fn resident_hint_upload_preserves_current_operator_and_rejects_bad_hints_cold() {
        let columns = vec![vec![2., 0.], vec![0., 2.]];
        let bounds = [2., 4.];
        let cold = ExperimentalResidentContactInput::new(&columns, &bounds, 1e-12).unwrap();
        let seeded =
            ExperimentalResidentContactInput::new_seeded(&columns, &bounds, 1e-12, &[0.5, 1.])
                .unwrap();
        assert_eq!(seeded.words[3], 1);
        assert_eq!(cold.total_words, seeded.total_words);
        assert_eq!(
            cold.words[4..cold.immutable_end],
            seeded.words[4..seeded.immutable_end]
        );
        assert_eq!(
            &seeded.words[seeded.reactions..seeded.reactions + 2],
            &pack(0.5).unwrap()
        );
        for hints in [
            vec![],
            vec![1.],
            vec![-1., 1.],
            vec![f64::NAN, 1.],
            vec![f64::INFINITY, 1.],
            vec![0., 0.],
        ] {
            assert_eq!(
                ExperimentalResidentContactInput::new_seeded(&columns, &bounds, 1e-12, &hints)
                    .unwrap()
                    .bytes(),
                cold.bytes()
            );
        }
        let tiny_bounds = [1e-300, 1e-300];
        let cold = ExperimentalResidentContactInput::new(&columns, &tiny_bounds, 1e-312).unwrap();
        let overflow = ExperimentalResidentContactInput::new_seeded(
            &columns,
            &tiny_bounds,
            1e-312,
            &[f64::MAX, 1.],
        )
        .unwrap();
        assert_eq!(overflow.bytes(), cold.bytes());
    }
    #[test]
    fn resident_candidate_codec_rejects_incomplete_corrupt_and_unrepresentable_input() {
        for source in [
            EXPERIMENTAL_RESIDENT_CONTACT_SHADER,
            EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER,
            EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER,
        ] {
            let module = naga::front::wgsl::parse_str(source).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap();
        }
        assert!(
            ExperimentalResidentContactInput::new(&vec![vec![1.]; 1024], &vec![1.; 1024], 1e-12)
                .is_ok()
        );
        assert!(matches!(
            ExperimentalResidentContactInput::new(&vec![vec![1.]; 1025], &vec![1.; 1025], 1e-12),
            Err(JointDotError::Capacity)
        ));
        let input =
            ExperimentalResidentContactInput::new(&[vec![1., 0.], vec![0., 1.]], &[1., 2.], 1e-12)
                .unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        assert!(ExperimentalResidentContactInput::new(&[vec![f64::NAN]], &[1.], 1e-12).is_err());
        assert!(
            ExperimentalResidentContactInput::new(&vec![vec![1.]; 513], &vec![1.; 513], 1e-12)
                .is_ok()
        );
        assert!(
            ExperimentalResidentContactInput::new(
                &[vec![f64::MAX, f64::from_bits(1)]],
                &[1.],
                1e-12
            )
            .is_err()
        );
        let mut words = input.words.clone();
        words[input.status] = 0;
        words[4] ^= 1;
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_err());
    }
    #[test]
    fn resident_readback_audits_complete_operator_and_output_without_scratch_copy() {
        let input =
            ExperimentalResidentContactInput::new(&[vec![2., 0.], vec![0., 2.]], &[2., 4.], 1e-12)
                .unwrap();
        let mut words = input.words.clone();
        words[input.status] = 0;
        // C/2 is identity, bound scale is 4, state scale is 2.
        for (offset, value) in [
            (input.state, 0.5),
            (input.state + 2, 1.),
            (input.reactions, 0.5),
            (input.reactions + 2, 1.),
        ] {
            words[offset..offset + 2].copy_from_slice(&pack(value).unwrap());
        }
        let decode = |words: &[u32]| input.candidate(bytemuck::cast_slice(words));
        assert_eq!(decode(&words).unwrap(), (vec![1., 2.], vec![0.5, 1.]));
        for offset in 0..input.immutable_end {
            let mut changed = words.clone();
            changed[offset] ^= 1;
            assert!(
                decode(&changed).is_err(),
                "immutable word {offset} escaped audit"
            );
        }
        for offset in [
            input.state,
            input.state + 1,
            input.state + 2,
            input.reactions,
            input.reactions + 1,
        ] {
            let mut changed = words.clone();
            changed[offset] = f32::NAN.to_bits();
            assert!(decode(&changed).is_err());
        }
        let mut negative = words.clone();
        negative[input.reactions..input.reactions + 2].copy_from_slice(&pack(-1.).unwrap());
        assert!(decode(&negative).is_err());
        let mut incomplete = words.clone();
        incomplete[input.status] = 1;
        assert!(decode(&incomplete).is_err());
        assert!(
            input
                .candidate(&input.bytes()[..input.bytes().len() - 4])
                .is_err()
        );
        // Private factor scratch is not a certificate and is intentionally ignored.
        words[input.immutable_end] = f32::NAN.to_bits();
        assert_eq!(decode(&words).unwrap(), (vec![1., 2.], vec![0.5, 1.]));
    }
    #[test]
    fn resident_batch_layout_rejects_overlap_and_incomplete_operators() {
        assert!(ExperimentalResidentContactBatch::new(Vec::new()).is_err());
        let inputs = [1., 2., 3.]
            .into_iter()
            .map(|b| ExperimentalResidentContactInput::new(&[vec![1.]], &[b], 1e-12).unwrap())
            .collect();
        let batch = ExperimentalResidentContactBatch::new(inputs).unwrap();
        assert!(
            batch
                .inputs
                .iter()
                .all(|input| input.words.len() == input.immutable_end
                    && input.words.len() < input.total_words)
        );
        let mut words = batch.words.clone();
        for (&offset, input) in batch.offsets.iter().zip(&batch.inputs) {
            words[offset + input.status] = 0;
        }
        assert_eq!(
            batch.candidate(bytemuck::cast_slice(&words)).unwrap().len(),
            3
        );
        let full: &[u8] = bytemuck::cast_slice(&words);
        let ranges = batch.candidate_readback_ranges();
        let compact: Vec<u8> = ranges
            .iter()
            .flat_map(|range| {
                full[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect();
        assert!(compact.len() < full.len());
        assert_eq!(
            batch.candidate_from_compact_readback(&compact).unwrap(),
            batch.candidate(full).unwrap()
        );
        for index in 0..batch.header_end * 4 {
            let mut changed = compact.clone();
            changed[index] ^= 1;
            assert!(batch.candidate_from_compact_readback(&changed).is_err());
        }
        let mut cursor = batch.header_end * 4;
        for input in &batch.inputs {
            for index in 0..input.immutable_end * 4 {
                let mut changed = compact.clone();
                changed[cursor + index] ^= 1;
                assert!(batch.candidate_from_compact_readback(&changed).is_err());
            }
            let status = cursor + (input.immutable_end + input.status - input.reactions) * 4;
            let mut changed = compact.clone();
            changed[status] = 1;
            assert!(batch.candidate_from_compact_readback(&changed).is_err());
            let state = cursor + (input.immutable_end + input.state - input.reactions) * 4;
            let mut changed = compact.clone();
            changed[state..state + 4].copy_from_slice(&f32::NAN.to_le_bytes());
            assert!(batch.candidate_from_compact_readback(&changed).is_err());
            cursor += (input.immutable_end + input.total_words - input.reactions) * 4;
        }
        assert!(
            batch
                .candidate_from_compact_readback(&compact[..compact.len() - 1])
                .is_err()
        );

        for index in 0..batch.header_end {
            let mut changed = words.clone();
            changed[index] ^= 1;
            assert!(batch.candidate(bytemuck::cast_slice(&changed)).is_err());
        }
        for (&offset, input) in batch.offsets.iter().zip(&batch.inputs) {
            let mut changed = words.clone();
            changed[offset + input.status] = 1;
            assert!(batch.candidate(bytemuck::cast_slice(&changed)).is_err());
            changed = words.clone();
            changed[offset + 4] ^= 1;
            assert!(batch.candidate(bytemuck::cast_slice(&changed)).is_err());
        }
        assert!(
            batch
                .candidate(&batch.bytes()[..batch.bytes().len() - 4])
                .is_err()
        );
    }
    #[test]
    #[ignore = "requires an actual GPU; experimental complete resident active-set candidate"]
    fn gpu_resident_active_set_candidate_preserves_original_coordinate_kkt() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let source = std::env::var_os("VOXY_HAIR_RESIDENT_ACTIVE_SHADER_FIXTURE")
            .map(|path| std::fs::read_to_string(path).unwrap());
        let program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            source
                .as_deref()
                .unwrap_or(EXPERIMENTAL_RESIDENT_CONTACT_SHADER),
        ))
        .unwrap();
        let mut cases = vec![
            (vec![vec![1., 0.], vec![0., 1.]], vec![1., 2.]),
            (vec![vec![1., 0.], vec![1., 1.]], vec![1., 3.]),
            (vec![vec![1., 0.], vec![1., 1.]], vec![1., -2.]),
            (vec![vec![1., 0.], vec![2., 1.]], vec![1., 3.]),
            (vec![vec![1., 0.], vec![2., 1.]], vec![1., 1.5]),
            (
                vec![vec![1., 0.], vec![0., 1.], vec![-1., 1.]],
                vec![0., 0., 1.],
            ),
        ]
        .into_iter()
        .map(|(c, b)| (c, b, 1e-12))
        .collect::<Vec<_>>();
        cases.push((vec![vec![1.]; 1024], vec![1.; 1024], 1e-12));
        if let Ok(path) = std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE") {
            use std::io::Read;
            let mut input = std::io::Cursor::new(std::fs::read(path).unwrap());
            fn integer(input: &mut std::io::Cursor<Vec<u8>>) -> usize {
                let mut b = [0; 4];
                input.read_exact(&mut b).unwrap();
                u32::from_le_bytes(b) as usize
            }
            fn scalar(input: &mut std::io::Cursor<Vec<u8>>) -> f64 {
                let mut b = [0; 8];
                input.read_exact(&mut b).unwrap();
                f64::from_le_bytes(b)
            }
            let mut magic = [0; 4];
            input.read_exact(&mut magic).unwrap();
            assert_eq!(&magic, b"VQC1");
            let rows = integer(&mut input);
            let width = integer(&mut input);
            let _systems = integer(&mut input);
            let _ = integer(&mut input);
            assert!(rows <= 1024 && width <= 65536);
            let tolerance = scalar(&mut input);
            let bounds = (0..rows).map(|_| scalar(&mut input)).collect::<Vec<_>>();
            for _ in 0..rows {
                let _ = scalar(&mut input);
            }
            let columns = (0..rows)
                .map(|_| (0..width).map(|_| scalar(&mut input)).collect())
                .collect();
            cases.push((columns, bounds, tolerance));
        }
        fn accurate(products: impl Iterator<Item = (f64, f64)>) -> f64 {
            let mut sum = 0f64;
            let mut correction = 0.;
            for (x, y) in products {
                let product = x * y;
                let next = sum + product;
                correction += if sum.abs() >= product.abs() {
                    (sum - next) + product
                } else {
                    (product - next) + sum
                };
                correction += x.mul_add(y, -product);
                sum = next;
            }
            sum + correction
        }
        let batch_inputs = cases
            .iter()
            .map(|(columns, bounds, tolerance)| {
                ExperimentalResidentContactInput::new(columns, bounds, *tolerance).unwrap()
            })
            .collect();
        let batch = ExperimentalResidentContactBatch::new(batch_inputs).unwrap();
        let mut serial_results = Vec::new();
        for (columns, bounds, tolerance) in cases {
            let input =
                ExperimentalResidentContactInput::new(&columns, &bounds, tolerance).unwrap();
            let began = std::time::Instant::now();
            let job = program.create_job(&device, input.bytes()).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
            let dispatch = job.encode_snapshot(&mut encoder).unwrap();
            let submission = queue.submit([encoder.finish()]);
            let mut read = dispatch.begin_read();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let bytes = read.try_read().unwrap().unwrap();
            let candidate = input.candidate(&bytes);
            if candidate.is_err() {
                let status = u32::from_le_bytes(
                    bytes[input.status * 4..input.status * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                eprintln!(
                    "RESIDENT ACTIVE REJECTION rows={} width={} status={status:#x} scope=GPU_candidate_not_physical_admission",
                    columns.len(),
                    input.width
                );
                let scalar = |offset: usize| {
                    let component =
                        |i| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()) as f64;
                    component(offset) + component(offset + 1)
                };
                let x = (0..input.width)
                    .map(|i| rescale(scalar(input.state + 2 * i), input.state_scale, 1.))
                    .collect::<Vec<_>>();
                let reactions = (0..input.rows)
                    .map(|i| {
                        rescale(
                            scalar(input.reactions + 2 * i),
                            input.state_scale,
                            1. / input.column_scale,
                        )
                    })
                    .collect::<Vec<_>>();
                let mut worst = (0usize, 0f64);
                let mut active = 0;
                for (i, (column, bound)) in columns.iter().zip(&bounds).enumerate() {
                    let gap = accurate(column.iter().copied().zip(x.iter().copied())) - bound;
                    let defect = if reactions[i] > 0. {
                        active += 1;
                        gap.abs()
                    } else {
                        (-gap).max(0.)
                    };
                    if defect > worst.1 {
                        worst = (i, defect);
                    }
                }
                eprintln!(
                    "RESIDENT REJECTED ORIGINAL RESIDUAL worst_row={} max_defect={:e} tolerance={tolerance:e} active={active} minimum_reaction={:e} scope=diagnostic_only_rejected_candidate",
                    worst.0,
                    worst.1,
                    reactions.iter().copied().fold(f64::INFINITY, f64::min)
                );
                if let Some(path) = std::env::var_os("VOXY_HAIR_RESIDENT_REJECTION_EXPORT") {
                    let report = serde_json::json!({"schema":"voxy-rejected-resident-operator-v1",
                        "status":status,"columns_bits":columns.iter().map(|c|c.iter().map(|v|v.to_bits()).collect::<Vec<_>>()).collect::<Vec<_>>(),
                        "bounds_bits":bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                        "tolerance_bits":tolerance.to_bits(),"packed_input_words":input.words[..input.immutable_end],
                        "column_scale_bits":input.column_scale.to_bits(),"state_scale_bits":input.state_scale.to_bits(),
                        "coordinates_bits":x.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                        "reactions_bits":reactions.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                        "scope":"rejected GPU diagnostic only; never admitted or published"});
                    std::fs::write(path, serde_json::to_vec(&report).unwrap()).unwrap();
                }
            }
            let (x, reactions) = candidate.unwrap();
            for (i, (column, bound)) in columns.iter().zip(&bounds).enumerate() {
                let gap = accurate(column.iter().copied().zip(x.iter().copied())) - bound;
                assert!(
                    if reactions[i] > 0. {
                        gap.abs() <= tolerance
                    } else {
                        gap >= -tolerance
                    },
                    "gap={gap} lambda={}",
                    reactions[i]
                );
            }
            eprintln!(
                "RESIDENT ACTIVE CANDIDATE rows={} width={} elapsed_ms={} submissions=1 tolerance={} original_coordinate_gaps_admitted=true scope=one_coordinate_operator_not_full_physical_system_or_FPS",
                columns.len(),
                x.len(),
                began.elapsed().as_secs_f64() * 1000.,
                tolerance
            );
            for k in 0..x.len() {
                let reconstructed =
                    accurate(columns.iter().zip(&reactions).map(|(c, l)| (c[k], *l)));
                assert!((x[k] - reconstructed).abs() <= tolerance);
            }
            serial_results.push((x, reactions));
        }
        let began = std::time::Instant::now();
        let job = program.create_job(&device, batch.bytes()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, batch.workgroups()).unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let mut compact_size = 0;
        let ranges = batch
            .candidate_readback_ranges()
            .into_iter()
            .map(|range| {
                let length = range.end - range.start;
                let destination = compact_size;
                compact_size += length;
                (range.start, destination, length)
            })
            .collect::<Vec<_>>();
        let compact = crate::ComputeDispatch::gather_buffer(
            &device,
            &mut encoder,
            job.buffer(),
            &ranges,
            compact_size,
        )
        .unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        let mut compact_read = compact.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let bytes = read.try_read().unwrap().unwrap();
        let compact_bytes = compact_read.try_read().unwrap().unwrap();
        assert_eq!(
            batch
                .candidate_from_compact_readback(&compact_bytes)
                .unwrap(),
            serial_results,
            "compact hardware readback changed complete candidate"
        );
        assert!(compact_bytes.len() < bytes.len());
        eprintln!(
            "RESIDENT COMPACT READBACK full_bytes={} compact_bytes={} exact_candidate_match=true",
            bytes.len(),
            compact_bytes.len()
        );
        assert_eq!(
            batch.candidate(&bytes).unwrap(),
            serial_results,
            "independent concurrent operators changed results"
        );
        eprintln!(
            "RESIDENT CONCURRENT BATCH operators={} submissions=1 dispatches=1 elapsed_ms={} identical_to_individually_admitted_operators=true",
            serial_results.len(),
            began.elapsed().as_secs_f64() * 1000.
        );
        let mut corrupt = bytes.clone();
        let offset = (batch.offsets[1] + batch.inputs[1].status) * 4;
        corrupt[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(
            batch.candidate(&corrupt).is_err(),
            "partial batch escaped complete publication gate"
        );
        // Neither an exhausted iteration budget nor an inconsistent dependent
        // active set may publish its intermediate feasible-looking state.
        for (columns, bounds, short_budget) in [
            (vec![vec![1.], vec![-1.]], vec![1., 1.], false),
            (vec![vec![1., 0.], vec![0., 1.]], vec![1., 2.], true),
        ] {
            let mut input =
                ExperimentalResidentContactInput::new(&columns, &bounds, 1e-12).unwrap();
            if short_budget {
                input.words[2] = 1;
            }
            let job = program.create_job(&device, input.bytes()).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
            let dispatch = job.encode_snapshot(&mut encoder).unwrap();
            let submission = queue.submit([encoder.finish()]);
            let mut read = dispatch.begin_read();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            assert!(input.candidate(&read.try_read().unwrap().unwrap()).is_err());
        }
    }
}

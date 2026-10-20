//! Experimental exact input transport. This is not a contact solver or admission.
use crate::joint_contact_dot::JointDotError;
const MAGIC: u32 = 0x31433556; // V5C1; cannot alias the legacy triple layout.
const HEADER: usize = 16;
const PARTS: usize = 5;

fn two_sum(a: f64, b: f64) -> [f64; 2] {
    let sum = a + b;
    let virtual_b = sum - a;
    [sum, (a - (sum - virtual_b)) + (b - virtual_b)]
}
fn pack(parts: [f64; 2], scale: f64) -> Option<[u32; PARTS]> {
    let [source_hi, source_lo] = parts;
    let mut hi = source_hi / scale;
    let mut lo = source_lo / scale;
    if !hi.is_finite()
        || hi.abs() > 2f64.powi(48)
        || !lo.is_finite()
        || hi * scale != source_hi
        || lo * scale != source_lo
    {
        return None;
    }
    let mut words = [0; PARTS];
    for word in &mut words {
        let term = hi as f32;
        if !term.is_finite() || term.is_subnormal() {
            return None;
        }
        *word = term.to_bits();
        [hi, lo] = two_sum(hi - f64::from(term), lo);
    }
    // No relative tolerance: even a single remaining source bit rejects packing.
    (hi == 0. && lo == 0.).then_some(words)
}
fn binary_scale(value: f64) -> f64 {
    let exponent = value.to_bits() & 0x7ff0_0000_0000_0000;
    if exponent != 0 {
        f64::from_bits(exponent)
    } else {
        f64::from_bits(1u64 << value.to_bits().ilog2())
    }
}

/// Owned, versioned five-component coefficient input with a separate roundtrip
/// output region. The GPU probe only tests transport, not numerical operations.
#[derive(Debug)]
pub struct ExperimentalExactFiveContactInput {
    words: Vec<u32>,
    items: usize,
    columns: usize,
    column_scale: f64,
    bound_scale: f64,
}
impl ExperimentalExactFiveContactInput {
    pub fn new(
        columns: &[Vec<[f64; 2]>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = columns.first().map_or(0, Vec::len);
        if rows == 0
            || width == 0
            || rows != bounds.len()
            || columns.iter().any(|c| c.len() != width)
        {
            return Err(JointDotError::Shape);
        }
        if !tolerance.is_finite()
            || tolerance <= 0.
            || bounds.iter().any(|v| !v.is_finite())
            || columns
                .iter()
                .flatten()
                .any(|&[hi, lo]| !hi.is_finite() || !lo.is_finite() || hi + lo != hi)
        {
            return Err(JointDotError::NumericRange);
        }
        let count = rows.checked_mul(width).ok_or(JointDotError::Capacity)?;
        let items = count
            .checked_add(rows)
            .and_then(|n| n.checked_add(1))
            .ok_or(JointDotError::Capacity)?;
        let total = items
            .checked_mul(PARTS * 2)
            .and_then(|n| n.checked_add(HEADER))
            .ok_or(JointDotError::Capacity)?;
        if total > 16 * 1024 * 1024 / 4 {
            return Err(JointDotError::Capacity);
        }
        let maximum = columns
            .iter()
            .flatten()
            .map(|v| v[0].abs())
            .fold(0., f64::max);
        let mut column_scale = if maximum == 0. {
            1.
        } else {
            binary_scale(maximum)
        };
        let mut packed_columns = None;
        for _ in 0..=64 {
            let candidate = columns
                .iter()
                .flatten()
                .map(|&v| pack(v, column_scale))
                .collect::<Option<Vec<_>>>();
            if let Some(candidate) = candidate {
                packed_columns = Some(candidate);
                break;
            }
            column_scale *= 0.5;
            if column_scale == 0. {
                break;
            }
        }
        let packed_columns = packed_columns.ok_or(JointDotError::NumericRange)?;
        let bound_scale = binary_scale(bounds.iter().map(|v| v.abs()).fold(tolerance, f64::max));
        let packed_bounds = bounds
            .iter()
            .copied()
            .chain([tolerance])
            .map(|v| pack([v, 0.], bound_scale))
            .collect::<Option<Vec<_>>>()
            .ok_or(JointDotError::NumericRange)?;
        let output = HEADER + items * PARTS;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| JointDotError::Capacity)?;
        words.extend([
            MAGIC,
            1,
            rows as u32,
            width as u32,
            PARTS as u32,
            u32::MAX,
            items as u32,
            0,
        ]);
        for value in [column_scale, bound_scale] {
            let bits = value.to_bits();
            words.extend([bits as u32, (bits >> 32) as u32]);
        }
        words.extend([0, 0, output as u32, total as u32]);
        words.extend(packed_columns.into_iter().flatten());
        words.extend(packed_bounds.into_iter().flatten());
        words.resize(total, 0);
        Ok(Self {
            words,
            items,
            columns: count,
            column_scale,
            bound_scale,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn column_scale(&self) -> f64 {
        self.column_scale
    }
    pub fn bound_scale(&self) -> f64 {
        self.bound_scale
    }
    pub fn packed_columns(&self) -> &[[u32; PARTS]] {
        bytemuck::cast_slice(&self.words[HEADER..HEADER + self.columns * PARTS])
    }
    pub fn packed_bounds(&self) -> &[[u32; PARTS]] {
        let rows = self.items - self.columns - 1;
        bytemuck::cast_slice(
            &self.words[HEADER + self.columns * PARTS..HEADER + (self.columns + rows) * PARTS],
        )
    }
    pub fn packed_tolerance(&self) -> [u32; PARTS] {
        self.words[HEADER + (self.items - 1) * PARTS..HEADER + self.items * PARTS]
            .try_into()
            .unwrap()
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [(self.items as u32).div_ceil(64), 1, 1]
    }
    pub fn roundtrip_shader() -> &'static str {
        include_str!("contact_exact_five_roundtrip.wgsl")
    }
    pub fn checked_roundtrip(&self, bytes: &[u8]) -> Result<Vec<[u32; PARTS]>, JointDotError> {
        if bytes.len() != self.bytes().len() {
            return Err(JointDotError::Output);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        let output = self.words[14] as usize;
        if words[5] != 0
            || (0..output).any(|i| i != 5 && words[i] != self.words[i])
            || words[output..] != self.words[HEADER..output]
        {
            return Err(JointDotError::Output);
        }
        Ok(words[output..]
            .chunks_exact(PARTS)
            .map(|v| v.try_into().unwrap())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_five_retains_tail_and_rejects_stale_or_mutated_readback() {
        let input =
            ExperimentalExactFiveContactInput::new(&[vec![[1., 2f64.powi(-100)]]], &[1.], 1e-14)
                .unwrap();
        assert!(input.packed_columns()[0].iter().skip(1).any(|&v| v != 0));
        assert!(input.checked_roundtrip(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[5] = 0;
        let output = words[14] as usize;
        words.copy_within(HEADER..output, output);
        assert_eq!(
            input
                .checked_roundtrip(bytemuck::cast_slice(&words))
                .unwrap()
                .len(),
            3
        );
        for i in [0, 1, 4, 8, HEADER, output] {
            let mut bad = words.clone();
            bad[i] ^= 1;
            assert!(input.checked_roundtrip(bytemuck::cast_slice(&bad)).is_err());
        }
        assert!(ExperimentalExactFiveContactInput::new(&[vec![[1., 1.]]], &[1.], 1e-14).is_err());
        assert!(
            ExperimentalExactFiveContactInput::new(&[vec![[f64::INFINITY, 0.]]], &[1.], 1e-14)
                .is_err()
        );
    }
    #[test]
    fn exact_five_roundtrip_shader_validates() {
        let module =
            naga::front::wgsl::parse_str(ExperimentalExactFiveContactInput::roundtrip_shader())
                .unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    #[ignore = "requires real GPU; transport only, not five-component contact arithmetic"]
    fn gpu_exact_five_roundtrip_preserves_all_words() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let columns = if let Some(path) = std::env::var_os("VOXY_EXACT_FIVE_AUDIT_INPUT") {
            let bytes = std::fs::read(path).unwrap();
            assert!(bytes.len() >= 20 && &bytes[..4] == b"VPA1");
            let rows = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
            let width = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
            assert!(rows > 0 && rows <= 1024 && width > 0 && width <= 65536);
            assert_eq!(bytes.len(), 20 + rows * width * 28);
            bytes[20..]
                .chunks_exact(28)
                .map(|record| {
                    [
                        f64::from_le_bytes(record[..8].try_into().unwrap()),
                        f64::from_le_bytes(record[8..16].try_into().unwrap()),
                    ]
                })
                .collect::<Vec<_>>()
                .chunks_exact(width)
                .map(|row| row.to_vec())
                .collect::<Vec<_>>()
        } else {
            vec![vec![[1., 2f64.powi(-100)], [-0.5, -2f64.powi(-105)]]]
        };
        let bounds = vec![1.; columns.len()];
        let input = ExperimentalExactFiveContactInput::new(&columns, &bounds, 1e-14).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(
            &device,
            ExperimentalExactFiveContactInput::roundtrip_shader(),
        ))
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
        let roundtrip = input
            .checked_roundtrip(&read.try_read().unwrap().unwrap())
            .unwrap();
        assert_eq!(
            roundtrip.len(),
            columns.len() * columns[0].len() + columns.len() + 1
        );
        if let Some(path) = std::env::var_os("VOXY_EXACT_FIVE_NATIVE_EXPORT") {
            use std::io::Write;
            let mut file = std::io::BufWriter::new(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .unwrap(),
            );
            file.write_all(b"VPA2").unwrap();
            for value in [columns.len() as u32, columns[0].len() as u32] {
                file.write_all(&value.to_le_bytes()).unwrap();
            }
            file.write_all(&input.column_scale().to_le_bytes()).unwrap();
            file.write_all(&5u32.to_le_bytes()).unwrap();
            for (source, packed) in columns.iter().flatten().zip(&roundtrip) {
                for value in source {
                    file.write_all(&value.to_le_bytes()).unwrap();
                }
                for value in packed {
                    file.write_all(&value.to_le_bytes()).unwrap();
                }
            }
            file.flush().unwrap();
        }
        eprintln!(
            "EXACT FIVE GPU TRANSFER rows={} width={} coefficients={} column_scale={} all_words_preserved=true scope=transport_only_not_arithmetic_or_physical_admission",
            columns.len(),
            columns[0].len(),
            input.columns,
            input.column_scale()
        );
    }
}

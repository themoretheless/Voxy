//! Experimental full-column residuals; no inequality is omitted or admitted here.
use crate::{ExperimentalFiveQrValue, joint_contact_dot::JointDotError};
const HEADER: usize = 16;
/// A certificate about the entire residual interval and the unchanged tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ExperimentalFiveGapDecision {
    Violated = 1,
    Positive = 2,
    WithinTolerance = 3,
    Uncertain = 4,
}

#[derive(Debug)]
pub struct ExperimentalFiveGapInput {
    words: Vec<u32>,
    rows: usize,
    output: usize,
    scale: f64,
}
impl ExperimentalFiveGapInput {
    pub fn new(
        columns: &[Vec<[f64; 2]>],
        state: &[[f64; 2]],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = state.len();
        if rows == 0 || width == 0 || bounds.len() != rows {
            return Err(JointDotError::Shape);
        }
        let mut combined = columns.to_vec();
        combined.push(state.to_vec());
        let mut all_bounds = bounds.to_vec();
        all_bounds.push(0.);
        let packed =
            crate::contact_exact_five::pack_five_contact(&combined, &all_bounds, tolerance)?;
        let state = packed.columns[rows * width..]
            .iter()
            .map(|&words| ExperimentalFiveQrValue {
                words,
                exponent: 0,
                error_bound: 0.,
            })
            .collect::<Vec<_>>();
        Self::new_with_runtime_state(columns, &state, packed.column_scale, bounds, tolerance)
    }
    /// Preserve the five GPU terms and their uncertainty across active-set rounds.
    pub fn new_with_runtime_state(
        columns: &[Vec<[f64; 2]>],
        state: &[ExperimentalFiveQrValue],
        state_scale: f64,
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = state.len();
        if rows == 0
            || rows > 1024
            || width == 0
            || width > 65536
            || bounds.len() != rows
            || columns.iter().any(|c| c.len() != width)
        {
            return Err(JointDotError::Shape);
        }
        let packed = crate::contact_exact_five::pack_five_contact(columns, bounds, tolerance)?;
        let shift = scale_exponent(packed.column_scale)? - scale_exponent(packed.bound_scale)?;
        let state_offset = HEADER + rows * width * 5;
        let bound_offset = state_offset + width * 8;
        let tolerance_offset = bound_offset + rows * 5;
        let output = tolerance_offset + 5;
        let total = output + rows * 9;
        if total > 16 * 1024 * 1024 / 4 {
            return Err(JointDotError::Capacity);
        }
        let mut words = vec![
            0x31473556,
            3,
            rows as u32,
            width as u32,
            5,
            u32::MAX,
            HEADER as u32,
            state_offset as u32,
            bound_offset as u32,
            output as u32,
            total as u32,
            shift as u32,
            tolerance_offset as u32,
        ];
        words.resize(HEADER, 0);
        words.extend(packed.columns.iter().flatten().copied());
        words.resize(bound_offset, 0);
        words.extend(packed.bounds().iter().flatten().copied());
        words.extend(packed.tolerance());
        assert_eq!(words.len(), output);
        words.resize(total, 0);
        let mut input = Self {
            words,
            rows,
            output,
            scale: packed.bound_scale,
        };
        input.replace_runtime_state(state, state_scale)?;
        Ok(input)
    }

    /// Replace only the dynamic state, retaining the exact operator and bounds.
    ///
    /// # Errors
    /// Rejects a different width, invalid scale or nonfinite state.
    pub fn replace_runtime_state(
        &mut self,
        state: &[ExperimentalFiveQrValue],
        state_scale: f64,
    ) -> Result<(), JointDotError> {
        if state.len() != self.words[3] as usize {
            return Err(JointDotError::Shape);
        }
        let shift = scale_exponent(state_scale)?;
        let values = state
            .iter()
            .map(|value| {
                let exponent = value
                    .exponent
                    .checked_add(shift)
                    .ok_or(JointDotError::NumericRange)?;
                if exponent.unsigned_abs() > 2048
                    || value
                        .words
                        .iter()
                        .any(|&word| !f32::from_bits(word).is_finite())
                    || !value.error_bound.is_finite()
                    || value.error_bound < 0.
                {
                    return Err(JointDotError::NumericRange);
                }
                let [a, b, c, d, e] = value.words;
                Ok([
                    a,
                    b,
                    c,
                    d,
                    e,
                    exponent as u32,
                    value.error_bound.to_bits(),
                    1,
                ])
            })
            .collect::<Result<Vec<_>, _>>()?;
        let start = self.words[7] as usize;
        self.words[start..start + state.len() * 8].copy_from_slice(values.as_flattened());
        Ok(())
    }

    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Byte ranges to upload before reusing a resident job. Clearing the
    /// output prevents an incomplete dispatch from returning a previous result.
    pub fn runtime_uploads(&self) -> [(u64, &[u8]); 2] {
        let state = self.words[7] as usize;
        let bounds = self.words[8] as usize;
        [
            (
                (state * 4) as u64,
                bytemuck::cast_slice(&self.words[state..bounds]),
            ),
            (
                (self.output * 4) as u64,
                bytemuck::cast_slice(&self.words[self.output..]),
            ),
        ]
    }

    pub fn workgroups(&self) -> [u32; 3] {
        [self.rows as u32, 1, 1]
    }
    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn shader_source() -> String {
        format!(
            "{}\n{}",
            include_str!("contact_five_product.wgsl"),
            include_str!("contact_five_gaps.wgsl")
        )
    }
    pub fn decisions(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<ExperimentalFiveGapDecision>, JointDotError> {
        self.results(bytes).map(|(_, decisions)| decisions)
    }

    /// Decode and validate residuals and decisions in one pass.
    ///
    /// # Errors
    /// Rejects a changed input prefix or an invalid output record.
    pub fn results(
        &self,
        bytes: &[u8],
    ) -> Result<
        (
            Vec<ExperimentalFiveQrValue>,
            Vec<ExperimentalFiveGapDecision>,
        ),
        JointDotError,
    > {
        let mut residuals = Vec::with_capacity(self.rows);
        let mut decisions = Vec::with_capacity(self.rows);
        for row in self.output_rows(bytes)? {
            residuals.push(decode_residual(row)?);
            decisions.push(match read_word(row, 8) {
                1 => ExperimentalFiveGapDecision::Violated,
                2 => ExperimentalFiveGapDecision::Positive,
                3 => ExperimentalFiveGapDecision::WithinTolerance,
                4 => ExperimentalFiveGapDecision::Uncertain,
                _ => return Err(JointDotError::Output),
            });
        }
        Ok((residuals, decisions))
    }

    pub fn residuals(&self, bytes: &[u8]) -> Result<Vec<ExperimentalFiveQrValue>, JointDotError> {
        self.output_rows(bytes)?.map(decode_residual).collect()
    }

    fn output_rows<'a>(
        &self,
        bytes: &'a [u8],
    ) -> Result<std::slice::ChunksExact<'a, u8>, JointDotError> {
        if bytes.len() != self.bytes().len() {
            return Err(JointDotError::Output);
        }
        let (prefix, output) = bytes.split_at(self.output * 4);
        let expected = &self.words[..self.output];
        let matches = if cfg!(target_endian = "little") {
            prefix == bytemuck::cast_slice::<u32, u8>(expected)
        } else {
            prefix
                .chunks_exact(4)
                .zip(expected)
                .all(|(bytes, &word)| read_word(bytes, 0) == word)
        };
        if !matches {
            return Err(JointDotError::Output);
        }
        Ok(output.chunks_exact(36))
    }
}

fn read_word(bytes: &[u8], index: usize) -> u32 {
    let start = index * 4;
    u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
}

fn decode_residual(row: &[u8]) -> Result<ExperimentalFiveQrValue, JointDotError> {
    let words = std::array::from_fn(|i| read_word(row, i));
    let error_bound = f32::from_bits(read_word(row, 6));
    if read_word(row, 7) != 1
        || words.iter().any(|&word| !f32::from_bits(word).is_finite())
        || !error_bound.is_finite()
        || error_bound < 0.
    {
        return Err(JointDotError::Output);
    }
    Ok(ExperimentalFiveQrValue {
        words,
        exponent: read_word(row, 5) as i32,
        error_bound,
    })
}

fn scale_exponent(v: f64) -> Result<i32, JointDotError> {
    if !v.is_normal() || v <= 0. || v.to_bits() & 0x000f_ffff_ffff_ffff != 0 {
        return Err(JointDotError::NumericRange);
    }
    Ok(((v.to_bits() >> 52) & 2047) as i32 - 1023)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_gap_shader_validates() {
        let module =
            naga::front::wgsl::parse_str(&ExperimentalFiveGapInput::shader_source()).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    fn five_gap_input_rejects_stale_and_shape() {
        let input =
            ExperimentalFiveGapInput::new(&[vec![[1., 2f64.powi(-60)]]], &[[2., 0.]], &[2.], 1e-14)
                .unwrap();
        assert!(input.residuals(input.bytes()).is_err());
        assert!(ExperimentalFiveGapInput::new(&[vec![[1., 0.]]], &[], &[2.], 1e-14).is_err());
    }

    #[test]
    fn replacing_runtime_state_matches_a_fresh_packet() {
        let columns = vec![
            vec![[1., 2f64.powi(-60)], [2., 0.]],
            vec![[3., 0.], [4., 0.]],
        ];
        let bounds = [2., 3.];
        let state: [_; 2] = std::array::from_fn(|_| ExperimentalFiveQrValue {
            words: [0; 5],
            exponent: 0,
            error_bound: 0.,
        });
        let mut packet =
            ExperimentalFiveGapInput::new_with_runtime_state(&columns, &state, 1., &bounds, 1e-14)
                .unwrap();
        let mut updated = state;
        updated[0].words[0] = 1.25_f32.to_bits();
        updated[0].exponent = -7;
        updated[1].error_bound = 2f32.powi(-60);
        let expected = ExperimentalFiveGapInput::new_with_runtime_state(
            &columns, &updated, 8., &bounds, 1e-14,
        )
        .unwrap();
        let mut resident = packet.bytes().to_vec();
        resident[packet.output * 4..].fill(0xff);
        packet.replace_runtime_state(&updated, 8.).unwrap();
        for (offset, bytes) in packet.runtime_uploads() {
            let offset = offset as usize;
            resident[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        assert_eq!(resident, expected.bytes());
        assert_eq!(packet.bytes(), expected.bytes());
        updated[1].error_bound = f32::NAN;
        assert!(packet.replace_runtime_state(&updated, 1.).is_err());
        assert_eq!(packet.bytes(), expected.bytes());
    }

    #[test]
    fn combined_output_checks_prefix_residual_and_decision() {
        let packet =
            ExperimentalFiveGapInput::new(&[vec![[1., 0.]]], &[[2., 0.]], &[2.], 1e-14).unwrap();
        let mut bytes = packet.bytes().to_vec();
        let output = packet.output * 4;
        for (index, word) in [1.25_f32.to_bits(), 0, 0, 0, 0, (-3_i32) as u32, 0, 1, 3]
            .into_iter()
            .enumerate()
        {
            bytes[output + index * 4..output + index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        let (residuals, decisions) = packet.results(&bytes).unwrap();
        assert_eq!(residuals[0].words[0], 1.25_f32.to_bits());
        assert_eq!(residuals[0].exponent, -3);
        assert_eq!(decisions, [ExperimentalFiveGapDecision::WithinTolerance]);
        bytes[0] ^= 1;
        assert!(packet.results(&bytes).is_err());
        bytes[0] ^= 1;
        bytes[output + 28] = 0;
        assert!(packet.results(&bytes).is_err());
        bytes[output + 28] = 1;
        bytes[output + 32] = 0;
        assert!(packet.results(&bytes).is_err());
        assert!(packet.residuals(&bytes).is_ok());
    }
}

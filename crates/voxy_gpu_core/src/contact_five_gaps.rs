//! Experimental full-column residuals; no inequality is omitted or admitted here.
use crate::{
    ExperimentalExactFiveContactInput, ExperimentalFiveQrValue, joint_contact_dot::JointDotError,
};
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
        let packed = ExperimentalExactFiveContactInput::new(&combined, &all_bounds, tolerance)?;
        let state = packed.packed_columns()[rows * width..]
            .iter()
            .map(|&words| ExperimentalFiveQrValue {
                words,
                exponent: 0,
                error_bound: 0.,
            })
            .collect::<Vec<_>>();
        Self::new_with_runtime_state(columns, &state, packed.column_scale(), bounds, tolerance)
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
        fn exponent(v: f64) -> Result<i32, JointDotError> {
            if !v.is_normal() || v <= 0. || v.to_bits() & 0x000f_ffff_ffff_ffff != 0 {
                return Err(JointDotError::NumericRange);
            }
            Ok(((v.to_bits() >> 52) & 2047) as i32 - 1023)
        }
        let state_shift = exponent(state_scale)?;
        let packed = ExperimentalExactFiveContactInput::new(columns, bounds, tolerance)?;
        let shift = exponent(packed.column_scale())? - exponent(packed.bound_scale())?;
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
        words.extend(packed.packed_columns().iter().flatten().copied());
        for value in state {
            let exponent = value
                .exponent
                .checked_add(state_shift)
                .ok_or(JointDotError::NumericRange)?;
            if exponent.unsigned_abs() > 2048
                || value.words.iter().any(|&w| !f32::from_bits(w).is_finite())
                || !value.error_bound.is_finite()
                || value.error_bound < 0.
            {
                return Err(JointDotError::NumericRange);
            }
            words.extend(value.words);
            words.extend([exponent as u32, value.error_bound.to_bits(), 1]);
        }
        words.extend(packed.packed_bounds().iter().flatten().copied());
        words.extend(packed.packed_tolerance());
        assert_eq!(words.len(), output);
        words.resize(total, 0);
        Ok(Self {
            words,
            rows,
            output,
            scale: packed.bound_scale(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
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
        self.residuals(bytes)?;
        bytes[self.output * 4..]
            .chunks_exact(36)
            .map(
                |v| match u32::from_le_bytes(v[32..36].try_into().unwrap()) {
                    1 => Ok(ExperimentalFiveGapDecision::Violated),
                    2 => Ok(ExperimentalFiveGapDecision::Positive),
                    3 => Ok(ExperimentalFiveGapDecision::WithinTolerance),
                    4 => Ok(ExperimentalFiveGapDecision::Uncertain),
                    _ => Err(JointDotError::Output),
                },
            )
            .collect()
    }
    pub fn residuals(&self, bytes: &[u8]) -> Result<Vec<ExperimentalFiveQrValue>, JointDotError> {
        if bytes.len() != self.bytes().len() {
            return Err(JointDotError::Output);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        if words[..self.output] != self.words[..self.output] {
            return Err(JointDotError::Output);
        }
        words[self.output..]
            .chunks_exact(9)
            .map(|v| {
                let parts: [u32; 5] = v[..5].try_into().unwrap();
                let error = f32::from_bits(v[6]);
                if v[7] != 1
                    || parts.iter().any(|&w| !f32::from_bits(w).is_finite())
                    || !error.is_finite()
                    || error < 0.
                {
                    return Err(JointDotError::Output);
                }
                Ok(ExperimentalFiveQrValue {
                    words: parts,
                    exponent: v[5] as i32,
                    error_bound: error,
                })
            })
            .collect()
    }
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
}

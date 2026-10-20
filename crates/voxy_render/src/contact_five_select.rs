//! Experimental certified entering-row selection from full precision GPU gaps.
use crate::{ExperimentalFiveQrValue, joint_contact_dot::JointDotError};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentalFiveSelection {
    Enter(usize),
    Feasible,
    Uncertain,
}
#[derive(Debug)]
pub struct ExperimentalFiveSelectionInput {
    words: Vec<u32>,
    output: usize,
    rows: usize,
}
impl ExperimentalFiveSelectionInput {
    pub fn new(
        gaps: &[ExperimentalFiveQrValue],
        decisions: &[u32],
        active: &[bool],
    ) -> Result<Self, JointDotError> {
        let rows = gaps.len();
        if rows == 0 || rows > 1024 || decisions.len() != rows || active.len() != rows {
            return Err(JointDotError::Shape);
        }
        let output = 8 + 10 * rows;
        let mut words = vec![
            0x31533556,
            1,
            rows as u32,
            10,
            8,
            output as u32,
            (output + 2) as u32,
            0,
        ];
        for ((gap, &decision), &active) in gaps.iter().zip(decisions).zip(active) {
            if !(1..=4).contains(&decision)
                || !gap.error_bound.is_finite()
                || gap.error_bound < 0.
                || gap.words.iter().any(|&w| !f32::from_bits(w).is_finite())
                || gap.exponent.unsigned_abs() > 2048
            {
                return Err(JointDotError::NumericRange);
            }
            words.extend(gap.words);
            words.extend([
                gap.exponent as u32,
                gap.error_bound.to_bits(),
                1,
                decision,
                u32::from(active),
            ]);
        }
        words.extend([u32::MAX, u32::MAX]);
        Ok(Self {
            words,
            output,
            rows,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn shader_source() -> String {
        format!(
            "{}\n{}",
            include_str!("contact_five_product.wgsl"),
            include_str!("contact_five_select.wgsl")
        )
    }
    pub fn selection(&self, bytes: &[u8]) -> Result<ExperimentalFiveSelection, JointDotError> {
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
        match (words[self.output], words[self.output + 1]) {
            (1, row)
                if (row as usize) < self.rows
                    && self.words[8 + row as usize * 10 + 8] == 1
                    && self.words[8 + row as usize * 10 + 9] == 0 =>
            {
                Ok(ExperimentalFiveSelection::Enter(row as usize))
            }
            (2, u32::MAX) => Ok(ExperimentalFiveSelection::Feasible),
            (3, u32::MAX) => Ok(ExperimentalFiveSelection::Uncertain),
            _ => Err(JointDotError::Output),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_selection_shader_validates() {
        let m =
            naga::front::wgsl::parse_str(&ExperimentalFiveSelectionInput::shader_source()).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
    }
    #[test]
    fn five_selection_rejects_stale_and_active_row() {
        let gap = ExperimentalFiveQrValue {
            words: [(-1f32).to_bits(), 0, 0, 0, 0],
            exponent: 0,
            error_bound: 0.,
        };
        let input = ExperimentalFiveSelectionInput::new(&[gap], &[1], &[true]).unwrap();
        assert!(input.selection(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[input.output] = 1;
        words[input.output + 1] = 0;
        assert!(input.selection(bytemuck::cast_slice(&words)).is_err());
    }
}

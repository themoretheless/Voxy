//! Experimental dual release step; all inputs share physical reaction units.
use crate::{ExperimentalFiveQrValue, joint_contact_dot::JointDotError};
#[derive(Debug)]
pub enum ExperimentalFiveRelease {
    Release {
        row: usize,
        fraction: ExperimentalFiveQrValue,
        reactions: Vec<ExperimentalFiveQrValue>,
        coordinates: Vec<ExperimentalFiveQrValue>,
    },
    Accept,
    Uncertain,
}
#[derive(Debug)]
pub struct ExperimentalFiveReleaseInput {
    words: Vec<u32>,
    rows: usize,
    output: usize,
}
impl ExperimentalFiveReleaseInput {
    pub fn new(
        current: &[ExperimentalFiveQrValue],
        candidate: &[ExperimentalFiveQrValue],
    ) -> Result<Self, JointDotError> {
        Self::new_with_state(current, candidate, &[], &[])
    }
    /// State coordinates use their own shared units, independently of reaction units.
    /// Five runtime terms and their error bounds remain intact through the update.
    pub fn new_with_state(
        current: &[ExperimentalFiveQrValue],
        candidate: &[ExperimentalFiveQrValue],
        state: &[ExperimentalFiveQrValue],
        next_state: &[ExperimentalFiveQrValue],
    ) -> Result<Self, JointDotError> {
        let width = state.len();
        if width > 65536 || next_state.len() != width {
            return Err(JointDotError::Shape);
        }
        let rows = current.len();
        if rows == 0 || rows > 1024 || candidate.len() != rows {
            return Err(JointDotError::Shape);
        }
        let output = 8 + 16 * (rows + width);
        let total = output + 2 + 8 * (rows + width + 1);
        if total > 16 * 1024 * 1024 / 4 {
            return Err(JointDotError::Capacity);
        }
        let mut words = vec![
            0x31523556,
            2,
            rows as u32,
            8,
            8,
            output as u32,
            total as u32,
            width as u32,
        ];
        for value in current
            .iter()
            .chain(candidate)
            .chain(state)
            .chain(next_state)
        {
            if value.words.iter().any(|&v| !f32::from_bits(v).is_finite())
                || !value.error_bound.is_finite()
                || value.error_bound < 0.
                || value.exponent.unsigned_abs() > 2048
            {
                return Err(JointDotError::NumericRange);
            }
            words.extend(value.words);
            words.extend([value.exponent as u32, value.error_bound.to_bits(), 1]);
        }
        words.resize(total, 0);
        words[output] = u32::MAX;
        words[output + 1] = u32::MAX;
        Ok(Self {
            words,
            rows,
            output,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn shader_source() -> String {
        format!(
            "{}\n{}",
            include_str!("contact_five_product.wgsl"),
            include_str!("contact_five_release.wgsl")
        )
    }
    pub fn result(&self, bytes: &[u8]) -> Result<ExperimentalFiveRelease, JointDotError> {
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
            (2, u32::MAX) => Ok(ExperimentalFiveRelease::Accept),
            (3, u32::MAX) => Ok(ExperimentalFiveRelease::Uncertain),
            (1, row) if (row as usize) < self.rows => {
                let mut values = words[self.output + 2..]
                    .chunks_exact(8)
                    .map(|v| {
                        let parts: [u32; 5] = v[..5].try_into().unwrap();
                        let error = f32::from_bits(v[6]);
                        let exponent = v[5] as i32;
                        if v[7] != 1
                            || parts.iter().any(|&w| !f32::from_bits(w).is_finite())
                            || !error.is_finite()
                            || error < 0.
                            || exponent.unsigned_abs() > 2048
                        {
                            return Err(JointDotError::Output);
                        }
                        Ok(ExperimentalFiveQrValue {
                            words: parts,
                            exponent,
                            error_bound: error,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let fraction = values.remove(0);
                let coordinates = values.split_off(self.rows);
                Ok(ExperimentalFiveRelease::Release {
                    row: row as usize,
                    fraction,
                    reactions: values,
                    coordinates,
                })
            }
            _ => Err(JointDotError::Output),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_release_shader_validates() {
        let m =
            naga::front::wgsl::parse_str(&ExperimentalFiveReleaseInput::shader_source()).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
    }
    #[test]
    fn five_release_rejects_stale() {
        let v = ExperimentalFiveQrValue {
            words: [1f32.to_bits(), 0, 0, 0, 0],
            exponent: 0,
            error_bound: 0.,
        };
        let input =
            ExperimentalFiveReleaseInput::new(std::slice::from_ref(&v), std::slice::from_ref(&v))
                .unwrap();
        assert!(input.result(input.bytes()).is_err());
    }
}

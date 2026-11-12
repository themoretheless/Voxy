//! Experimental rectangular equality QR; physical inequality admission is external.
use crate::joint_contact_dot::JointDotError;
const MAGIC: u32 = 0x31513556; // V5Q1, distinct from input and arithmetic probes.
const HEADER: usize = 32;
const VALUE: usize = 8;
#[derive(Clone, Debug)]
pub struct ExperimentalFiveQrValue {
    pub words: [u32; 5],
    pub exponent: i32,
    pub error_bound: f32,
}
#[derive(Debug)]
pub struct ExperimentalFiveQrReadback {
    pub coordinates: Vec<ExperimentalFiveQrValue>,
    pub reactions: Vec<ExperimentalFiveQrValue>,
    pub basis: Vec<ExperimentalFiveQrValue>,
    pub triangular: Vec<ExperimentalFiveQrValue>,
}
#[derive(Debug)]
pub struct ExperimentalFiveQrInput {
    words: Vec<u32>,
    rows: usize,
    width: usize,
    input_end: usize,
    q: usize,
    r: usize,
    state: usize,
    reaction: usize,
    state_scale: f64,
    reaction_scale: f64,
}
impl ExperimentalFiveQrInput {
    pub fn new(
        columns: &[impl AsRef<[[f64; 2]]>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = columns.first().map_or(0, |column| column.as_ref().len());
        if rows == 0 || rows > width || rows > 1024 {
            return Err(JointDotError::Shape);
        }
        let packed = crate::contact_exact_five::pack_five_contact(columns, bounds, tolerance)?;
        let count = rows.checked_mul(width).ok_or(JointDotError::Capacity)?;
        let q = HEADER
            .checked_add(
                (count + rows + 1)
                    .checked_mul(5)
                    .ok_or(JointDotError::Capacity)?,
            )
            .ok_or(JointDotError::Capacity)?;
        let r = q
            .checked_add(count.checked_mul(VALUE).ok_or(JointDotError::Capacity)?)
            .ok_or(JointDotError::Capacity)?;
        let state = r
            .checked_add(
                rows.checked_mul(rows)
                    .and_then(|n| n.checked_mul(VALUE))
                    .ok_or(JointDotError::Capacity)?,
            )
            .ok_or(JointDotError::Capacity)?;
        let reaction = state
            .checked_add(width.checked_mul(VALUE).ok_or(JointDotError::Capacity)?)
            .ok_or(JointDotError::Capacity)?;
        let total = reaction
            .checked_add(rows.checked_mul(VALUE).ok_or(JointDotError::Capacity)?)
            .ok_or(JointDotError::Capacity)?;
        if total > 16 * 1024 * 1024 / 4 {
            return Err(JointDotError::Capacity);
        }
        let (state_scale, reaction_scale) =
            response_scales(packed.column_scale, packed.bound_scale)?;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| JointDotError::Capacity)?;
        words.extend([
            MAGIC,
            1,
            rows as u32,
            width as u32,
            5,
            VALUE as u32,
            u32::MAX,
            0,
            HEADER as u32,
            (HEADER + count * 5) as u32,
            (HEADER + (count + rows) * 5) as u32,
            q as u32,
            r as u32,
            state as u32,
            reaction as u32,
            total as u32,
        ]);
        for value in [packed.column_scale, packed.bound_scale] {
            let bits = value.to_bits();
            words.extend([bits as u32, (bits >> 32) as u32]);
        }
        words.resize(HEADER, 0);
        words.extend(packed.columns.iter().flatten().copied());
        words.extend(packed.bounds().iter().flatten().copied());
        words.extend(packed.tolerance());
        assert_eq!(words.len(), q);
        words.resize(total, 0);
        Ok(Self {
            words,
            rows,
            width,
            input_end: q,
            q,
            r,
            state,
            reaction,
            state_scale,
            reaction_scale,
        })
    }
    /// Reuse the packed operator with freshly encoded bounds and tolerance.
    ///
    /// # Errors
    /// Rejects incompatible dimensions and nonrepresentable bounds or scales.
    pub fn replace_bounds(&mut self, bounds: &[f64], tolerance: f64) -> Result<(), JointDotError> {
        if bounds.len() != self.rows {
            return Err(JointDotError::Shape);
        }
        let (scale, packed) = crate::contact_exact_five::pack_five_bounds(bounds, tolerance)?;
        let column_scale =
            f64::from_bits(u64::from(self.words[16]) | (u64::from(self.words[17]) << 32));
        let (state_scale, reaction_scale) = response_scales(column_scale, scale)?;
        let bits = scale.to_bits();
        self.words[18] = bits as u32;
        self.words[19] = (bits >> 32) as u32;
        let start = self.words[9] as usize;
        self.words[start..self.input_end].copy_from_slice(packed.as_flattened());
        self.state_scale = state_scale;
        self.reaction_scale = reaction_scale;
        Ok(())
    }

    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Byte range occupied by the basis and triangular factor in an admitted readback.
    pub fn factor_range(&self) -> std::ops::Range<usize> {
        self.q * 4..self.state * 4
    }

    /// Prepare a solve dispatch using factors from a validated readback of the
    /// same ordered columns and column scale. Bounds may differ.
    ///
    /// # Errors
    /// Rejects a factor buffer with an incompatible size.
    pub fn factorized_bytes(&self, factors: &[u8]) -> Result<Vec<u8>, JointDotError> {
        let range = self.factor_range();
        if factors.len() != range.len() {
            return Err(JointDotError::Shape);
        }
        let mut bytes = self.bytes().to_vec();
        bytes[28..32].copy_from_slice(&(self.rows as u32).to_ne_bytes());
        bytes[range].copy_from_slice(factors);
        Ok(bytes)
    }

    pub fn steps(&self) -> usize {
        self.rows + 1
    }
    pub fn state_scale(&self) -> f64 {
        self.state_scale
    }
    pub fn reaction_scale(&self) -> f64 {
        self.reaction_scale
    }
    pub fn shader_source() -> String {
        format!(
            "{}\n{}",
            include_str!("contact_five_product.wgsl"),
            include_str!("contact_five_qr.wgsl")
        )
    }
    pub fn candidate(&self, bytes: &[u8]) -> Result<ExperimentalFiveQrReadback, JointDotError> {
        if bytes.len() != self.bytes().len() {
            return Err(JointDotError::Output);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        if words[6] != 0
            || words[7] as usize != self.steps()
            || (0..self.input_end).any(|i| i != 6 && i != 7 && words[i] != self.words[i])
        {
            return Err(JointDotError::Output);
        }
        let read =
            |start: usize, count: usize| -> Result<Vec<ExperimentalFiveQrValue>, JointDotError> {
                words[start..start + count * VALUE]
                    .chunks_exact(VALUE)
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
            };
        Ok(ExperimentalFiveQrReadback {
            coordinates: read(self.state, self.width)?,
            reactions: read(self.reaction, self.rows)?,
            basis: read(self.q, self.rows * self.width)?,
            triangular: read(self.r, self.rows * self.rows)?,
        })
    }
}
fn response_scales(column: f64, bound: f64) -> Result<(f64, f64), JointDotError> {
    let state = bound / column;
    let reaction = state / column;
    if !state.is_finite() || state <= 0. || !reaction.is_finite() || reaction <= 0. {
        return Err(JointDotError::NumericRange);
    }
    Ok((state, reaction))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacing_qr_bounds_matches_fresh_packet_and_preserves_failed_input() {
        let columns = [
            vec![[1., 2f64.powi(-60)], [0., 0.]],
            vec![[0., 0.], [2., 0.]],
        ];
        let mut packet = ExperimentalFiveQrInput::new(&columns, &[1., 2.], 1e-14).unwrap();
        let bounds = [-32., 17.];
        let expected = ExperimentalFiveQrInput::new(&columns, &bounds, 1e-15).unwrap();
        packet.replace_bounds(&bounds, 1e-15).unwrap();
        assert_eq!(packet.bytes(), expected.bytes());
        assert_eq!(packet.state_scale(), expected.state_scale());
        assert_eq!(packet.reaction_scale(), expected.reaction_scale());
        assert!(packet.replace_bounds(&[f64::NAN, 17.], 1e-15).is_err());
        assert_eq!(packet.bytes(), expected.bytes());
    }

    #[test]
    fn borrowed_active_columns_match_owned_qr_packet() {
        let columns = [
            vec![[1., 2f64.powi(-60)], [0., 0.]],
            vec![[0., 0.], [2., 0.]],
        ];
        let bounds = [1., 2.];
        let owned = ExperimentalFiveQrInput::new(&columns, &bounds, 1e-14).unwrap();
        let borrowed = ExperimentalFiveQrInput::new(
            &[columns[0].as_slice(), columns[1].as_slice()],
            &bounds,
            1e-14,
        )
        .unwrap();
        assert_eq!(borrowed.bytes(), owned.bytes());
        assert_eq!(borrowed.steps(), owned.steps());
    }

    #[test]
    fn five_qr_shader_validates() {
        let module =
            naga::front::wgsl::parse_str(&ExperimentalFiveQrInput::shader_source()).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    fn five_qr_input_rejects_stale_and_mutated_readbacks() {
        let input = ExperimentalFiveQrInput::new(&[vec![[1., 0.]]], &[1.], 1e-14).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words = input.words.clone();
        words[6] = 0;
        words[7] = 2;
        for at in [input.q, input.r, input.state, input.reaction] {
            words[at] = 1f32.to_bits();
            words[at + 7] = 1;
        }
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_ok());
        for i in [0, 1, 4, 5, 8, HEADER, input.state + 7] {
            let mut bad = words.clone();
            bad[i] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        assert!(
            ExperimentalFiveQrInput::new(&[vec![[1., 0.]], vec![[2., 0.]]], &[1., 2.], 1e-14)
                .is_err()
        );
    }
}

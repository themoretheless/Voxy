//! Exact source-f32 morph sums. Source products are accumulated in units 2^-298.
use crate::{SkinnedMesh, SkinnedMeshError};
use std::sync::Arc;

/// Maximum simultaneously evaluated targets; no truncation or weight normalization.
pub const MAX_MORPH_TARGETS: usize = 256;
/// Missing streams contribute zero in original vertex order.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedMorphTarget {
    pub positions: Option<Vec<[f32; 3]>>,
    pub normals: Option<Vec<[f32; 3]>>,
}

#[derive(Clone, Debug)]
pub struct SkinnedMorphMesh {
    mesh: Arc<SkinnedMesh>,
    targets: Arc<[SkinnedMorphTarget]>,
}
impl SkinnedMorphMesh {
    /// Validate immutable targets against the source and a total delta-vector cap.
    /// # Errors
    /// Rejects wrong counts, nonfinite streams, missing normal bases and budgets.
    pub fn new(
        mesh: Arc<SkinnedMesh>,
        targets: Vec<SkinnedMorphTarget>,
        max_vectors: usize,
    ) -> Result<Self, SkinnedMeshError> {
        if targets.is_empty() || targets.len() > MAX_MORPH_TARGETS {
            return Err(SkinnedMeshError::InvalidGeometry);
        }
        let mut total = 0usize;
        for target in &targets {
            if target.positions.is_none() && target.normals.is_none() {
                return Err(SkinnedMeshError::InvalidGeometry);
            }
            if target.normals.is_some() && mesh.vertices().iter().any(|v| v.normal == [0.; 3]) {
                return Err(SkinnedMeshError::InvalidGeometry);
            }
            for values in [target.positions.as_ref(), target.normals.as_ref()]
                .into_iter()
                .flatten()
            {
                total = total
                    .checked_add(values.len())
                    .ok_or(SkinnedMeshError::InvalidGeometry)?;
                if total > max_vectors
                    || values.len() != mesh.vertices().len()
                    || values.iter().flatten().any(|v| !v.is_finite())
                {
                    return Err(SkinnedMeshError::InvalidGeometry);
                }
            }
        }
        Ok(Self {
            mesh,
            targets: targets.into(),
        })
    }
    #[must_use]
    pub fn mesh(&self) -> &Arc<SkinnedMesh> {
        &self.mesh
    }
    #[must_use]
    pub fn targets(&self) -> &[SkinnedMorphTarget] {
        &self.targets
    }
    /// Exact sum followed by one ties-to-even f32 rounding before existing skinning.
    /// # Errors
    /// Rejects invalid weights, overflowing results or invalid deformed attributes.
    pub fn evaluate(&self, weights: &[f32]) -> Result<SkinnedMesh, SkinnedMeshError> {
        if weights.len() != self.targets.len() || weights.iter().any(|v| !v.is_finite()) {
            return Err(SkinnedMeshError::InvalidGeometry);
        }
        let mut vertices = self.mesh.vertices().to_vec();
        for (index, vertex) in vertices.iter_mut().enumerate() {
            for axis in 0..3 {
                for normal in [false, true] {
                    let base = if normal {
                        vertex.normal[axis]
                    } else {
                        vertex.position[axis]
                    };
                    let terms = self.targets.iter().zip(weights).map(|(target, weight)| {
                        let values = if normal {
                            &target.normals
                        } else {
                            &target.positions
                        };
                        (values.as_ref().map_or(0., |v| v[index][axis]), *weight)
                    });
                    let value = exact_morph_component(base, terms)
                        .ok_or(SkinnedMeshError::NonFiniteVertex(index))?;
                    if normal {
                        vertex.normal[axis] = value;
                    } else {
                        vertex.position[axis] = value;
                    }
                }
            }
        }
        SkinnedMesh::new(
            vertices,
            self.mesh.indices().to_vec(),
            self.mesh.joint_count(),
        )
    }
    pub(crate) fn packed_deltas(&self) -> Vec<f32> {
        let mut values = Vec::with_capacity(self.targets.len() * self.mesh.vertices().len() * 6);
        for target in self.targets.iter() {
            for index in 0..self.mesh.vertices().len() {
                values.extend(target.positions.as_ref().map_or([0.; 3], |v| v[index]));
                values.extend(target.normals.as_ref().map_or([0.; 3], |v| v[index]));
            }
        }
        values
    }
}

// 36 radix-65536 digits hold 576 bits. Any finite f32*f32 term has at
// most 554 magnitude bits at this scale; 256 terms need at most 562 bits.
const WORDS: usize = 36;
fn float_parts(value: f32) -> Option<(u64, usize, bool)> {
    let bits = value.to_bits();
    let exponent = (bits >> 23) & 255;
    if exponent == 255 {
        return None;
    }
    let mantissa = u64::from((bits & 0x7fffff) | if exponent == 0 { 0 } else { 0x800000 });
    Some((
        mantissa,
        if exponent == 0 {
            0
        } else {
            exponent as usize - 1
        },
        bits >> 31 != 0,
    ))
}
fn accumulate(digits: &mut [i64; WORDS], mantissa: u64, shift: usize, negative: bool) {
    let value = u128::from(mantissa) << (shift % 16);
    let first = shift / 16;
    for index in 0..4 {
        let part = ((value >> (index * 16)) & 65535) as i64;
        digits[first + index] += if negative { -part } else { part };
    }
}
/// Exact local component, exposed for import/reference consumers sharing the
/// same source-f32 convention. Canonicalizes an exact zero to positive zero.
/// Returns None for nonfinite inputs, >256 terms or an overflowing f32 result.
fn exact_morph_words(
    base: f32,
    terms: impl IntoIterator<Item = (f32, f32)>,
) -> Option<([u32; WORDS], bool)> {
    let mut digits = [0i64; WORDS];
    let (mantissa, exponent, negative) = float_parts(base)?;
    accumulate(&mut digits, mantissa, exponent + 149, negative);
    let mut count = 0;
    for (a, b) in terms {
        count += 1;
        if count > MAX_MORPH_TARGETS {
            return None;
        }
        let (am, ae, an) = float_parts(a)?;
        let (bm, be, bn) = float_parts(b)?;
        accumulate(&mut digits, am * bm, ae + be, an ^ bn);
    }
    let mut words = [0u32; WORDS];
    let mut carry = 0i64;
    for (word, digit) in words.iter_mut().zip(digits) {
        let value = digit + carry;
        *word = (value & 65535) as u32;
        carry = value >> 16;
    }
    let negative = carry == -1;
    if carry != 0 && !negative {
        return None;
    }
    if negative {
        let mut carry = 1;
        for word in &mut words {
            let value = (*word ^ 65535) + carry;
            *word = value & 65535;
            carry = value >> 16;
        }
    }
    Some((words, negative))
}

/// Exact source sum rounded once to binary32, including subnormal/tie handling.
/// Returns None for nonfinite inputs, more than 256 terms or f32 overflow.
pub fn exact_morph_component(
    base: f32,
    terms: impl IntoIterator<Item = (f32, f32)>,
) -> Option<f32> {
    let (words, negative) = exact_morph_words(base, terms)?;
    let Some(top) = words.iter().rposition(|v| *v != 0) else {
        return Some(0.);
    };
    let mut high = top * 16 + words[top].ilog2() as usize;
    let shift = 149.max(high.saturating_sub(23));
    let bit = |index: usize| (words[index / 16] >> (index % 16)) & 1;
    let mut significand = 0u32;
    for index in 0..24 {
        significand |= bit(shift + index) << index;
    }
    let guard = bit(shift - 1) != 0;
    let sticky = (0..shift - 1).any(|i| bit(i) != 0);
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    let sign = u32::from(negative) << 31;
    if high < 172 {
        return Some(f32::from_bits(sign | significand));
    }
    if significand == 0x1000000 {
        significand >>= 1;
        high += 1;
    }
    let exponent = high - 171;
    if exponent >= 255 {
        return None;
    }
    Some(f32::from_bits(
        sign | ((exponent as u32) << 23) | (significand & 0x7fffff),
    ))
}

/// Exact source sum rounded once to binary64 for physical/reference consumers.
/// Every finite binary32 product and the bounded sum fit the normal f64 range.
/// Returns None for nonfinite inputs or more than 256 terms.
pub fn exact_morph_component64(
    base: f32,
    terms: impl IntoIterator<Item = (f32, f32)>,
) -> Option<f64> {
    let (words, negative) = exact_morph_words(base, terms)?;
    let Some(top) = words.iter().rposition(|v| *v != 0) else {
        return Some(0.);
    };
    let mut high = top * 16 + words[top].ilog2() as usize;
    let shift = high.saturating_sub(52);
    let bit = |index: usize| (words[index / 16] >> (index % 16)) & 1;
    let mut significand = 0u64;
    for index in 0..53 {
        significand |= u64::from(bit(shift + index)) << index;
    }
    if shift != 0
        && bit(shift - 1) != 0
        && ((0..shift - 1).any(|i| bit(i) != 0) || significand & 1 != 0)
    {
        significand += 1;
    }
    if significand == 1u64 << 53 {
        significand >>= 1;
        high += 1;
    }
    if high < 52 {
        significand <<= 52 - high;
    }
    let exponent = (high + 725) as u64;
    Some(f64::from_bits(
        (u64::from(negative) << 63) | (exponent << 52) | (significand & 0x000f_ffff_ffff_ffff),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_sum_preserves_cancellation_products_subnormals_and_rounding() {
        assert_eq!(
            exact_morph_component(1e30, [(1., 1.), (-1e30, 1.)]),
            Some(1.)
        );
        assert_eq!(
            exact_morph_component64(1e30, [(1., 1.), (-1e30, 1.)]),
            Some(1.)
        );
        assert_eq!(
            exact_morph_component64(0., [(f32::from_bits(1), 0.5)]),
            Some(2f64.powi(-150))
        );
        assert_eq!(
            exact_morph_component64(f32::MAX, [(f32::MAX, 1.)]),
            Some(2. * f64::from(f32::MAX))
        );
        assert_eq!(
            exact_morph_component(1., [(f32::from_bits(1), 1.)]),
            Some(1.)
        );
        assert_eq!(
            exact_morph_component(0., [(f32::from_bits(1), 1.)])
                .unwrap()
                .to_bits(),
            1
        );
        assert_eq!(
            exact_morph_component(0., [(f32::from_bits(1), 0.5)])
                .unwrap()
                .to_bits(),
            0
        );
        assert_eq!(
            exact_morph_component(0., [(f32::from_bits(3), 0.5)])
                .unwrap()
                .to_bits(),
            2
        );
        assert_eq!(exact_morph_component(1., [(2f32.powi(-24), 1.)]), Some(1.));
        assert_eq!(
            exact_morph_component(1., [(2f32.powi(-24), 1.), (2f32.powi(-30), 1.)])
                .unwrap()
                .to_bits(),
            1f32.to_bits() + 1
        );
        assert!(exact_morph_component(f32::MAX, [(f32::MAX, 1.)]).is_none());
        assert!(exact_morph_component(0., [(f32::NAN, 1.)]).is_none());
        assert!(exact_morph_component(0., vec![(0., 0.); 257]).is_none());
    }
}

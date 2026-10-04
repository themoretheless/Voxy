//! Exact dyadic SAT gap sign for finite stored binary64 boxes and axes.
//! Products occupy bits 0..4195 in units 2^-2148. At most 24 signed products
//! enter one gap, requiring <=4201 bits; 66 limbs provide 4224 bits. No rounding,
//! transcendental assumptions, heap allocation or arbitrary contact epsilon.
use super::*;
use std::cmp::Ordering;
const LIMBS: usize = 66;
#[derive(Clone, Copy)]
struct Exact {
    negative: bool,
    limbs: [u64; LIMBS],
}
impl Exact {
    const ZERO: Self = Self {
        negative: false,
        limbs: [0; LIMBS],
    };
    fn magnitude_cmp(&self, b: &Self) -> Ordering {
        for i in (0..LIMBS).rev() {
            let order = self.limbs[i].cmp(&b.limbs[i]);
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    }
    fn sign(self) -> i8 {
        if self.limbs.iter().all(|v| *v == 0) {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }
    fn negated(mut self) -> Self {
        self.negative = !self.negative;
        self
    }
    fn absolute(mut self) -> Self {
        self.negative = false;
        self
    }
    fn add(self, b: Self) -> Self {
        if self.negative == b.negative {
            let mut result = Self::ZERO;
            result.negative = self.negative;
            let mut carry = 0_u128;
            for i in 0..LIMBS {
                let value = u128::from(self.limbs[i]) + u128::from(b.limbs[i]) + carry;
                result.limbs[i] = value as u64;
                carry = value >> 64;
            }
            debug_assert_eq!(carry, 0, "finite projection gap bit bound");
            result
        } else {
            let (large, small) = match self.magnitude_cmp(&b) {
                Ordering::Equal => return Self::ZERO,
                Ordering::Greater => (self, b),
                Ordering::Less => (b, self),
            };
            let mut result = Self::ZERO;
            result.negative = large.negative;
            let mut borrow = false;
            for i in 0..LIMBS {
                let (value, first) = large.limbs[i].overflowing_sub(small.limbs[i]);
                let (value, second) = value.overflowing_sub(u64::from(borrow));
                result.limbs[i] = value;
                borrow = first || second;
            }
            debug_assert!(!borrow);
            result
        }
    }
    fn product(a: f64, b: f64) -> Self {
        fn decode(v: f64) -> (bool, u64, i32) {
            let bits = v.to_bits();
            let exponent = ((bits >> 52) & 0x7ff) as i32;
            let fraction = bits & ((1_u64 << 52) - 1);
            (
                bits >> 63 != 0,
                if exponent == 0 {
                    fraction
                } else {
                    fraction | (1_u64 << 52)
                },
                if exponent == 0 {
                    -1074
                } else {
                    exponent - 1023 - 52
                },
            )
        }
        let (an, am, ae) = decode(a);
        let (bn, bm, be) = decode(b);
        let product = u128::from(am) * u128::from(bm);
        if product == 0 {
            return Self::ZERO;
        }
        let shift = (ae + be + 2148) as usize;
        let limb = shift / 64;
        let offset = shift % 64;
        let mut result = Self::ZERO;
        result.negative = an != bn;
        result.limbs[limb] = (product as u64) << offset;
        if offset == 0 {
            result.limbs[limb + 1] = (product >> 64) as u64;
        } else {
            result.limbs[limb + 1] = (product >> (64 - offset)) as u64;
            if limb + 2 < LIMBS {
                result.limbs[limb + 2] = (product >> (128 - offset)) as u64;
            }
        }
        result
    }
}
fn dot(a: DVec3, b: DVec3) -> Exact {
    let mut result = Exact::ZERO;
    for i in 0..3 {
        result = result.add(Exact::product(a[i], b[i]));
    }
    result
}
/// Exact sign of |(center-obstacle.center) dot axis| minus both support radii.
/// Nonnegative means disjoint interiors on this arbitrary nonzero direction;
/// equality proves touching in projection, not a continuous support trajectory.
pub(super) fn sign(
    center: DVec3,
    edges: [DVec3; 3],
    obstacle: &AffineBox,
    axis: DVec3,
) -> Result<i8, PhysicsError> {
    if [center, obstacle.center, axis]
        .into_iter()
        .chain(edges)
        .chain(obstacle.edges)
        .any(|v| !v.is_finite())
        || axis == DVec3::ZERO
    {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut gap = dot(center, axis)
        .add(dot(obstacle.center, axis).negated())
        .absolute();
    for edge in edges.into_iter().chain(obstacle.edges) {
        gap = gap.add(dot(edge, axis).absolute().negated());
    }
    Ok(gap.sign())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_gap_distinguishes_touching_from_adjacent_float_penetration() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let obstacle = AffineBox {
            center: DVec3::X,
            edges,
        };
        for center in [0.75_f64.next_down(), 0.75, 0.75_f64.next_up()] {
            let position = DVec3::X * center;
            let result = sign(position, edges, &obstacle, DVec3::X).unwrap();
            println!(
                "EXACT_GAP_SIGN {:?}",
                (
                    position.to_array(),
                    edges.map(|v| v.to_array()),
                    obstacle.center.to_array(),
                    obstacle.edges.map(|v| v.to_array()),
                    DVec3::X.to_array(),
                    result
                )
            );
            assert_eq!(
                result,
                if center < 0.75 {
                    1
                } else if center == 0.75 {
                    0
                } else {
                    -1
                }
            );
        }
        for magnitude in [
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            1e-100,
            1.,
            1e100,
            f64::MAX,
        ] {
            for axis in [
                DVec3::X,
                DVec3::X * magnitude,
                DVec3::new(magnitude, -magnitude, magnitude),
            ] {
                let position = DVec3::new(magnitude, magnitude, -magnitude);
                let body = [
                    DVec3::X * (magnitude / 4.),
                    DVec3::Y * (magnitude / 8.),
                    DVec3::Z * (magnitude / 16.),
                ];
                let box_shape = AffineBox {
                    center: -position,
                    edges: body,
                };
                let result = sign(position, body, &box_shape, axis).unwrap();
                println!(
                    "EXACT_GAP_SIGN {:?}",
                    (
                        position.to_array(),
                        body.map(|v| v.to_array()),
                        box_shape.center.to_array(),
                        box_shape.edges.map(|v| v.to_array()),
                        axis.to_array(),
                        result
                    )
                );
            }
        }
        assert!(sign(DVec3::ZERO, edges, &obstacle, DVec3::ZERO).is_err());
    }
}

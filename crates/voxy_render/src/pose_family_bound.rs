//! Sufficient certificate for every UNORM16 convex blend of one joint family.
//! It never supplies posed geometry: uncertain bounds use the existing admission.
use glam::Mat4;

#[derive(Clone, Copy, Debug)]
struct Interval {
    lo: f64,
    hi: f64,
}
impl Interval {
    fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn add(self, b: Self) -> Self {
        Self {
            lo: self.lo + b.lo,
            hi: self.hi + b.hi,
        }
    }
    fn sub(self, b: Self) -> Self {
        Self {
            lo: self.lo - b.hi,
            hi: self.hi - b.lo,
        }
    }
    fn mul(self, b: Self) -> Self {
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self {
            lo: v.into_iter().fold(f64::INFINITY, f64::min),
            hi: v.into_iter().fold(f64::NEG_INFINITY, f64::max),
        }
    }
    fn widen(self, margin: f64) -> Self {
        Self {
            lo: self.lo - margin,
            hi: self.hi + margin,
        }
    }
    fn abs_max(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    fn abs_min(self) -> f64 {
        if self.lo > 0. {
            self.lo
        } else if self.hi < 0. {
            -self.hi
        } else {
            0.
        }
    }
}

/// Sentinel-padded sorted active joint IDs; zero-weight influences are excluded.
/// Palette/model finite-affine admission must have succeeded before this call.
pub(super) fn certify(family: [u16; 4], joints: &[Mat4], model: Mat4) -> bool {
    const EPS: f64 = f32::EPSILON as f64;
    const TINY: f64 = f32::MIN_POSITIVE as f64;
    // Restrict this fast certificate to modest finite matrices; all others fall
    // back. This ensures every bounded f32 multiply/add is far from overflow.
    const LIMIT: f64 = 4_294_967_296.;
    let model = model.to_cols_array();
    if model
        .iter()
        .any(|v| !v.is_finite() || f64::from(v.abs()) > LIMIT)
    {
        return false;
    }
    let mut skin = [[Interval {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3]; 3];
    for joint in family.into_iter().take_while(|&joint| joint != u16::MAX) {
        let matrix = joints[usize::from(joint)].to_cols_array();
        if matrix
            .iter()
            .any(|v| !v.is_finite() || f64::from(v.abs()) > LIMIT)
        {
            return false;
        }
        for col in 0..3 {
            for row in 0..3 {
                let x = f64::from(matrix[col * 4 + row]);
                skin[col][row].lo = skin[col][row].lo.min(x);
                skin[col][row].hi = skin[col][row].hi.max(x);
            }
        }
    }
    // An exact normalized-weight blend is in the componentwise convex hull.
    // Cover four decoded weights, products and ordered f32 accumulations, including
    // cancellation/underflow. EPS is twice unit roundoff; these margins are loose.
    for column in &mut skin {
        for entry in column {
            *entry = entry.widen(16. * EPS * entry.abs_max() + 16. * TINY);
        }
    }
    let mut world = [[Interval::point(0.); 3]; 3];
    for col in 0..3 {
        for row in 0..3 {
            let mut magnitude = 0.;
            for k in 0..3 {
                let coefficient = f64::from(model[k * 4 + row]);
                world[col][row] =
                    world[col][row].add(skin[col][k].mul(Interval::point(coefficient)));
                magnitude += coefficient.abs() * skin[col][k].abs_max();
            }
            // The fourth affine term is exactly zero. Cover f32 dot-product error
            // and the much smaller f64 rounding in endpoint propagation together.
            world[col][row] = world[col][row].widen(32. * EPS * magnitude + 32. * TINY);
        }
    }
    let lower = world
        .iter()
        .flatten()
        .map(|x| x.abs_min())
        .fold(0., f64::max);
    let upper = world
        .iter()
        .flatten()
        .map(|x| x.abs_max())
        .fold(0., f64::max);
    if !lower.is_finite() || lower <= TINY || upper >= 1. / TINY {
        return false;
    }
    let reciprocal = Interval {
        lo: 1. / upper,
        hi: 1. / lower,
    };
    for column in &mut world {
        for entry in column {
            // NormalTransform scales by a rounded reciprocal and rounded multiply.
            // Actual scaled entries have magnitude <=1+4EPS; intersect two enclosures.
            *entry = entry.mul(reciprocal).widen(8. * EPS);
            entry.lo = entry.lo.max(-1. - 4. * EPS);
            entry.hi = entry.hi.min(1. + 4. * EPS);
            if entry.lo > entry.hi {
                return false;
            }
        }
    }
    let [a, b, c] = world;
    let determinant = a[0]
        .mul(b[1].mul(c[2]).sub(b[2].mul(c[1])))
        .sub(b[0].mul(a[1].mul(c[2]).sub(a[2].mul(c[1]))))
        .add(c[0].mul(a[1].mul(b[2]).sub(a[2].mul(b[1]))));
    // Cover the actual f32 determinant, including reassociation/contraction.
    // A 1/512 margin implies the existing |det|>=1/1024 certificate and exceeds
    // the existing uncertainty bound (<=16EPS*6*(1+4EPS)^3).
    let determinant = determinant.widen(64. * EPS);
    determinant.lo >= 1. / 512. || determinant.hi <= -1. / 512.
}

//! Small f64 vector/quaternion operations; quaternion layout is [x,y,z,w].
pub type V = [f64; 3];
pub type Q = [f64; 4];
pub fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn mul(a: V, s: f64) -> V {
    a.map(|x| x * s)
}
pub fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn len(a: V) -> f64 {
    dot(a, a).sqrt()
}
pub fn unit(a: V) -> V {
    mul(a, 1.0 / len(a).max(1e-30))
}
pub fn conj(q: Q) -> Q {
    [-q[0], -q[1], -q[2], q[3]]
}
pub fn qm(a: Q, b: Q) -> Q {
    let av = [a[0], a[1], a[2]];
    let bv = [b[0], b[1], b[2]];
    let v = add(add(mul(bv, a[3]), mul(av, b[3])), cross(av, bv));
    [v[0], v[1], v[2], a[3] * b[3] - dot(av, bv)]
}
pub fn qunit(q: Q) -> Q {
    let l = q.iter().map(|x| x * x).sum::<f64>().sqrt();
    q.map(|x| x / l)
}
pub fn rotate(q: Q, v: V) -> V {
    let p = qm(qm(q, [v[0], v[1], v[2], 0.]), conj(q));
    [p[0], p[1], p[2]]
}
pub fn exp(v: V) -> Q {
    let angle = len(v);
    if angle < 1e-12 {
        return qunit([v[0] * 0.5, v[1] * 0.5, v[2] * 0.5, 1.]);
    }
    let s = (angle * 0.5).sin() / angle;
    [v[0] * s, v[1] * s, v[2] * s, (angle * 0.5).cos()]
}
pub fn log(mut q: Q) -> V {
    if q[3] < 0. {
        q = q.map(|x| -x);
    }
    let v = [q[0], q[1], q[2]];
    let l = len(v);
    mul(v, 2. * l.atan2(q[3]) / l.max(1e-30))
}
pub fn apply(q: &mut Q, v: V) {
    // The identity update must not renormalize an already admitted state.
    // Repeated normalization is not bitwise idempotent in binary64.
    if v==[0.;3] {return;}
    let candidate=qm(exp(v),*q);
    if candidate!=*q {*q=qunit(candidate);}
}

#[cfg(test)]
mod identity_update_tests {
    use super::*;
    #[test]
    fn zero_and_unrepresentable_rotations_preserve_the_admitted_orientation() {
        let original=qunit([0.1,0.2,0.3,0.4]);
        assert_ne!(original,qunit(original),"fixture exposes normalization drift");
        for increment in [[0.;3],[f64::MIN_POSITIVE,0.,0.]] {
            let mut actual=original;apply(&mut actual,increment);
            assert_eq!(actual.map(f64::to_bits),original.map(f64::to_bits));
        }
        let mut signed_zero=[-0.,0.,0.,1.];let bits=signed_zero.map(f64::to_bits);
        apply(&mut signed_zero,[0.;3]);assert_eq!(signed_zero.map(f64::to_bits),bits);
        let mut rotated=original;apply(&mut rotated,[0.1,-0.2,0.3]);
        assert_ne!(rotated,original);
        assert!((rotated.iter().map(|x|x*x).sum::<f64>()-1.).abs()<1e-15);
    }
}
pub fn from_z(v: V) -> Q {
    let t = unit(v);
    if t[2] < -0.999999 {
        return [1., 0., 0., 0.];
    }
    qunit([-t[1], t[0], 0., 1. + t[2]])
}
pub fn finite(v: V) -> bool {
    v.iter().all(|x| x.is_finite())
}

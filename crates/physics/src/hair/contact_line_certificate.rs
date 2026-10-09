//! Sufficient continuous separation certificate for moving supporting lines.
//! Every arithmetic operation encloses its exact result using outward rounding.
//! Failure to prove separation is unknown, never permission to publish motion.
use crate::hair::CapsuleMotion;

#[derive(Clone, Copy, Debug)]
struct Bound {
    lo: f64,
    hi: f64,
}
impl Bound {
    fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn add(self, b: Self) -> Self {
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    fn mul(self, b: Self) -> Self {
        let products = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if products.iter().any(|x| !x.is_finite()) {
            return Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        Self {
            lo: products
                .into_iter()
                .fold(f64::INFINITY, f64::min)
                .next_down(),
            hi: products
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
}
type Poly = [Bound; 7];
fn zero() -> Poly {
    [Bound::exact(0.); 7]
}
fn sum(a: Poly, b: Poly) -> Poly {
    std::array::from_fn(|i| a[i].add(b[i]))
}
fn difference(a: Poly, b: Poly) -> Poly {
    std::array::from_fn(|i| a[i].sub(b[i]))
}
fn product(a: Poly, da: usize, b: Poly, db: usize) -> Poly {
    assert!(da + db <= 6);
    let mut out = zero();
    for i in 0..=da {
        for j in 0..=db {
            out[i + j] = out[i + j].add(a[i].mul(b[j]));
        }
    }
    out
}
fn linear_difference(
    a: CapsuleMotion,
    pa: usize,
    b: CapsuleMotion,
    pb: usize,
    axis: usize,
) -> Poly {
    let mut out = zero();
    let start = Bound::exact(a.start[pa][axis]).sub(Bound::exact(b.start[pb][axis]));
    let end = Bound::exact(a.end[pa][axis]).sub(Bound::exact(b.end[pb][axis]));
    out[0] = start;
    out[1] = end.sub(start);
    out
}
fn choose(n: usize, k: usize) -> u32 {
    (0..k).fold(1, |value, i| value * (n - i) as u32 / (i + 1) as u32)
}
fn bernstein(power: Poly, degree: usize) -> Poly {
    let mut out = zero();
    for i in 0..=degree {
        for (j, coefficient) in power.iter().enumerate().take(i + 1) {
            let ratio = choose(i, j) as f64 / choose(degree, j) as f64;
            let enclosed = Bound {
                lo: ratio.next_down(),
                hi: ratio.next_up(),
            };
            out[i] = out[i].add(coefficient.mul(enclosed));
        }
    }
    out
}
fn split(mut values: Poly, degree: usize) -> (Poly, Poly) {
    let mut left = zero();
    let mut right = zero();
    left[0] = values[0];
    right[degree] = values[degree];
    for level in 1..=degree {
        for i in 0..=degree - level {
            values[i] = values[i].add(values[i + 1]).mul(Bound::exact(0.5));
        }
        left[level] = values[0];
        right[degree - level] = values[degree - level];
    }
    (left, right)
}

pub(super) fn clear(a: CapsuleMotion, b: CapsuleMotion, budget: usize) -> bool {
    let u: [Poly; 3] = std::array::from_fn(|i| linear_difference(a, 1, a, 0, i));
    let v: [Poly; 3] = std::array::from_fn(|i| linear_difference(b, 1, b, 0, i));
    let w: [Poly; 3] = std::array::from_fn(|i| linear_difference(a, 0, b, 0, i));
    let cross: [Poly; 3] = std::array::from_fn(|i| {
        difference(
            product(u[(i + 1) % 3], 1, v[(i + 2) % 3], 1),
            product(u[(i + 2) % 3], 1, v[(i + 1) % 3], 1),
        )
    });
    let mut norm = zero();
    let mut triple = zero();
    for i in 0..3 {
        norm = sum(norm, product(cross[i], 2, cross[i], 2));
        triple = sum(triple, product(w[i], 1, cross[i], 2));
    }
    let threshold = Bound::exact(a.radius)
        .add(Bound::exact(b.radius))
        .sub(Bound::exact(1e-10));
    if threshold.lo <= 0. || !threshold.hi.is_finite() {
        return false;
    }
    let squared = threshold.mul(threshold);
    let clearance = difference(
        product(triple, 3, triple, 3),
        std::array::from_fn(|i| norm[i].mul(squared)),
    );
    let mut stack = vec![(bernstein(clearance, 6), bernstein(norm, 4), 0usize)];
    let mut visited = 0;
    while let Some((c, n, depth)) = stack.pop() {
        visited += 1;
        if visited > budget {
            return false;
        }
        if c.iter()
            .chain(&n[..5])
            .any(|x| !x.lo.is_finite() || !x.hi.is_finite())
        {
            return false;
        }
        if c.iter().all(|x| x.lo >= 0.) && n[..5].iter().all(|x| x.lo > 0.) {
            continue;
        }
        if c.iter().all(|x| x.hi < 0.) || depth >= 48 {
            return false;
        }
        let (cl, cr) = split(c, 6);
        let (nl, nr) = split(n, 4);
        stack.push((cr, nr, depth + 1));
        stack.push((cl, nl, depth + 1));
    }
    true
}

#[cfg(test)]
#[path = "contact_line_fixture_tests.rs"]
mod fixture_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_line_bound_rejects_tunnel_and_unknown_parallel_lines() {
        let a = CapsuleMotion {
            start: [[-0.001, -0.01, 0.], [-0.001, 0.01, 0.]],
            end: [[0.001, -0.01, 0.], [0.001, 0.01, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[0., 0., -0.01], [0., 0., 0.01]],
            end: [[0., 0., -0.01], [0., 0., 0.01]],
            radius: 40e-6,
        };
        assert!(!clear(a, b, 512));
        let mut stationary = a;
        stationary.end = stationary.start;
        assert!(clear(stationary, b, 512));
        assert!(!clear(stationary, b, 0));
        let mut parallel = stationary;
        parallel.start = [[0.001, -0.01, 0.], [0.001, 0.01, 0.]];
        parallel.end = parallel.start;
        assert!(!clear(stationary, parallel, 512));
    }
    #[test]
    fn overflow_never_certifies_clearance() {
        let a = CapsuleMotion {
            start: [[1e300, -1e300, 0.], [1e300, 1e300, 0.]],
            end: [[1e300, -1e300, 0.], [1e300, 1e300, 0.]],
            radius: 40e-6,
        };
        let b = CapsuleMotion {
            start: [[0., 0., -1e300], [0., 0., 1e300]],
            end: [[0., 0., -1e300], [0., 0., 1e300]],
            radius: 40e-6,
        };
        assert!(!clear(a, b, 512));
    }
}

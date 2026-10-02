//! Exact first/second derivatives of small element energies.
use std::ops::{Add, Mul, Neg, Sub};
#[derive(Clone, Copy, Debug)]
pub(super) struct D<const N: usize> {
    pub v: f64,
    pub g: [f64; N],
    pub h: [[f64; N]; N],
}
impl<const N: usize> D<N> {
    pub fn c(v: f64) -> Self {
        Self {
            v,
            g: [0.0; N],
            h: [[0.0; N]; N],
        }
    }
    pub fn variable(v: f64, i: usize) -> Self {
        let mut d = Self::c(v);
        d.g[i] = 1.0;
        d
    }
    fn chain(self, value: f64, first: f64, second: f64) -> Self {
        Self {
            v: value,
            g: self.g.map(|x| first * x),
            h: std::array::from_fn(|i| {
                std::array::from_fn(|j| first * self.h[i][j] + second * self.g[i] * self.g[j])
            }),
        }
    }
    pub fn reciprocal(self) -> Self {
        self.chain(1.0 / self.v, -1.0 / self.v.powi(2), 2.0 / self.v.powi(3))
    }
    pub fn sqrt(self) -> Self {
        let s = self.v.sqrt();
        self.chain(s, 0.5 / s, -0.25 / (self.v * s))
    }
    pub fn ln(self) -> Self {
        self.chain(self.v.ln(), 1.0 / self.v, -1.0 / self.v.powi(2))
    }
    pub fn exp(self) -> Self {
        let e = self.v.exp();
        self.chain(e, e, e)
    }
    pub fn square(self) -> Self {
        self * self
    }
    pub fn positive(self) -> Self {
        if self.v > 0.0 { self } else { Self::c(0.0) }
    }
    pub fn atan2(self, x: Self) -> Self {
        let r = x.v * x.v + self.v * self.v;
        let r2 = r * r;
        let fy = x.v / r;
        let fx = -self.v / r;
        let fyy = -2.0 * x.v * self.v / r2;
        let fxx = 2.0 * x.v * self.v / r2;
        let fyx = (self.v * self.v - x.v * x.v) / r2;
        Self {
            v: self.v.atan2(x.v),
            g: std::array::from_fn(|i| fy * self.g[i] + fx * x.g[i]),
            h: std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    fy * self.h[i][j]
                        + fx * x.h[i][j]
                        + fyy * self.g[i] * self.g[j]
                        + fxx * x.g[i] * x.g[j]
                        + fyx * (self.g[i] * x.g[j] + x.g[i] * self.g[j])
                })
            }),
        }
    }
    pub fn finite(&self) -> bool {
        self.v.is_finite()
            && self.g.iter().all(|v| v.is_finite())
            && self.h.iter().flatten().all(|v| v.is_finite())
    }
}
impl<const N: usize> Add for D<N> {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            v: self.v + b.v,
            g: std::array::from_fn(|i| self.g[i] + b.g[i]),
            h: std::array::from_fn(|i| std::array::from_fn(|j| self.h[i][j] + b.h[i][j])),
        }
    }
}
impl<const N: usize> Sub for D<N> {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            v: self.v - b.v,
            g: std::array::from_fn(|i| self.g[i] - b.g[i]),
            h: std::array::from_fn(|i| std::array::from_fn(|j| self.h[i][j] - b.h[i][j])),
        }
    }
}
impl<const N: usize> Neg for D<N> {
    type Output = Self;
    fn neg(self) -> Self {
        self * -1.0
    }
}
impl<const N: usize> Mul<f64> for D<N> {
    type Output = Self;
    fn mul(self, b: f64) -> Self {
        Self {
            v: self.v * b,
            g: self.g.map(|v| v * b),
            h: self.h.map(|r| r.map(|v| v * b)),
        }
    }
}
impl<const N: usize> Mul for D<N> {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            v: self.v * b.v,
            g: std::array::from_fn(|i| self.g[i] * b.v + b.g[i] * self.v),
            h: std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    self.h[i][j] * b.v
                        + b.h[i][j] * self.v
                        + self.g[i] * b.g[j]
                        + b.g[i] * self.g[j]
                })
            }),
        }
    }
}
pub(super) fn dot<const N: usize>(a: [D<N>; 3], b: [D<N>; 3]) -> D<N> {
    a.into_iter().zip(b).fold(D::c(0.0), |s, (a, b)| s + a * b)
}
pub(super) fn cross<const N: usize>(a: [D<N>; 3], b: [D<N>; 3]) -> [D<N>; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(super) fn sub<const N: usize>(a: [D<N>; 3], b: [D<N>; 3]) -> [D<N>; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

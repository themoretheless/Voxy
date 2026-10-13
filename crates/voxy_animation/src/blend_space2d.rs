//! Phase-synchronized 2D blend space (e.g. forward speed × strafe speed).
//! Samples are triangulated once (Bowyer–Watson Delaunay in f64); a query
//! inside the hull blends the three corners barycentrically, a query outside
//! is projected onto the nearest hull edge.

use std::sync::Arc;

use glam::{DVec2, Vec2};

use crate::{AnimationClip, AnimationError, Pose, Skeleton};

/// One clip placed at a 2D parameter.
#[derive(Clone, Debug)]
pub struct BlendSample2D {
    pub parameter: Vec2,
    pub clip: Arc<AnimationClip>,
}

/// Up to three corner samples with weights summing to one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendWeights2D {
    pub indices: [usize; 3],
    pub weights: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct BlendSpace2D {
    samples: Vec<BlendSample2D>,
    triangles: Vec<[usize; 3]>,
    /// Counter-clockwise convex hull sample indices.
    hull: Vec<usize>,
}

impl BlendSpace2D {
    /// # Errors
    /// Rejects fewer than three samples, nonfinite or duplicate parameters,
    /// collinear sample sets and clips incompatible with `skeleton`.
    pub fn new(samples: Vec<BlendSample2D>, skeleton: &Skeleton) -> Result<Self, AnimationError> {
        if samples.len() < 3 {
            return Err(AnimationError::InvalidClipHeader);
        }
        if samples
            .iter()
            .any(|s| !s.parameter.is_finite() || !s.clip.is_compatible_with(skeleton))
        {
            return Err(AnimationError::SkeletonMismatch);
        }
        let points: Vec<DVec2> = samples.iter().map(|s| s.parameter.as_dvec2()).collect();
        for (i, a) in points.iter().enumerate() {
            if points[..i].iter().any(|b| b == a) {
                return Err(AnimationError::InvalidClipHeader);
            }
        }
        let hull = convex_hull(&points);
        if hull.len() < 3 {
            return Err(AnimationError::InvalidClipHeader);
        }
        let triangles = delaunay(&points);
        if triangles.is_empty() {
            return Err(AnimationError::InvalidClipHeader);
        }
        Ok(Self {
            samples,
            triangles,
            hull,
        })
    }

    #[must_use]
    pub fn samples(&self) -> &[BlendSample2D] {
        &self.samples
    }

    #[must_use]
    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }

    #[must_use]
    pub fn hull(&self) -> &[usize] {
        &self.hull
    }

    /// Corner samples and weights for `p`, clamping to the hull.
    ///
    /// # Errors
    /// Rejects a nonfinite point.
    pub fn weights(&self, p: Vec2) -> Result<BlendWeights2D, AnimationError> {
        if !p.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let p = p.as_dvec2();
        let pt = |i: usize| self.samples[i].parameter.as_dvec2();
        for &[a, b, c] in &self.triangles {
            if let Some(w) = barycentric(p, pt(a), pt(b), pt(c)) {
                return Ok(BlendWeights2D {
                    indices: [a, b, c],
                    weights: [w.0 as f32, w.1 as f32, w.2 as f32],
                });
            }
        }
        // Outside: nearest point on the hull polygon.
        let mut best: Option<(f64, usize, usize, f64)> = None;
        for k in 0..self.hull.len() {
            let i = self.hull[k];
            let j = self.hull[(k + 1) % self.hull.len()];
            let (a, b) = (pt(i), pt(j));
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            let d = (a + ab * t - p).length_squared();
            if best.is_none_or(|(bd, ..)| d < bd) {
                best = Some((d, i, j, t));
            }
        }
        let (_, i, j, t) = best.ok_or(AnimationError::InvalidBlendWeight)?;
        Ok(BlendWeights2D {
            indices: [i, j, i],
            weights: [(1.0 - t) as f32, t as f32, 0.0],
        })
    }

    /// Weighted cycle length in seconds.
    ///
    /// # Errors
    /// Rejects a nonfinite point.
    pub fn cycle_duration(&self, p: Vec2) -> Result<f32, AnimationError> {
        let w = self.weights(p)?;
        Ok(w.indices
            .iter()
            .zip(w.weights)
            .map(|(&i, weight)| self.samples[i].clip.duration * weight)
            .sum())
    }

    /// Samples the corners at the shared `phase` and blends them. The first
    /// two corners blend by `w1 / (w0 + w1)`, then the third by `w2`.
    ///
    /// # Errors
    /// Rejects nonfinite input, phase outside `[0, 1]` and invalid poses.
    pub fn evaluate(
        &self,
        skeleton: &Skeleton,
        p: Vec2,
        phase: f64,
    ) -> Result<Pose, AnimationError> {
        let w = self.weights(p)?;
        let sample = |i: usize| self.samples[i].clip.try_sample_phase(skeleton, phase);
        let [w0, w1, w2] = w.weights;
        let [i0, i1, i2] = w.indices;
        let pair = w0 + w1;
        let first = if w1 <= 0.0 {
            sample(i0)?
        } else if w0 <= 0.0 {
            sample(i1)?
        } else {
            Pose::blend(&sample(i0)?, &sample(i1)?, w1 / pair)?
        };
        if w2 <= 0.0 {
            return Ok(first);
        }
        Pose::blend(&first, &sample(i2)?, w2)
    }
}

/// Owns the shared phase of a [`BlendSpace2D`].
#[derive(Clone, Debug)]
pub struct BlendSpace2DPlayer {
    space: Arc<BlendSpace2D>,
    phase: f64,
    parameter: Vec2,
}

impl BlendSpace2DPlayer {
    #[must_use]
    pub fn new(space: Arc<BlendSpace2D>, parameter: Vec2) -> Self {
        Self {
            space,
            phase: 0.0,
            parameter,
        }
    }

    #[must_use]
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// # Errors
    /// Rejects nonfinite/negative `dt` or a nonfinite parameter.
    pub fn advance(&mut self, parameter: Vec2, dt: f32) -> Result<(), AnimationError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let duration = self.space.cycle_duration(parameter)?;
        self.parameter = parameter;
        self.phase = (self.phase + f64::from(dt) / f64::from(duration)).rem_euclid(1.0);
        Ok(())
    }

    /// # Errors
    /// Propagates sampling errors.
    pub fn pose(&self, skeleton: &Skeleton) -> Result<Pose, AnimationError> {
        self.space.evaluate(skeleton, self.parameter, self.phase)
    }
}

fn cross(o: DVec2, a: DVec2, b: DVec2) -> f64 {
    (a - o).perp_dot(b - o)
}

/// Barycentric coordinates if `p` lies inside or on triangle `abc`.
fn barycentric(p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> Option<(f64, f64, f64)> {
    let area = cross(a, b, c);
    if area == 0.0 {
        return None;
    }
    let wa = cross(p, b, c) / area;
    let wb = cross(p, c, a) / area;
    let wc = 1.0 - wa - wb;
    let eps = -1e-12;
    if wa >= eps && wb >= eps && wc >= eps {
        Some((wa.max(0.0), wb.max(0.0), wc.max(0.0)))
    } else {
        None
    }
}

/// Andrew's monotone chain; counter-clockwise, collinear points dropped.
fn convex_hull(points: &[DVec2]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&i, &j| {
        points[i]
            .x
            .total_cmp(&points[j].x)
            .then(points[i].y.total_cmp(&points[j].y))
    });
    let mut hull: Vec<usize> = Vec::with_capacity(points.len() * 2);
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &usize>> = if pass == 0 {
            Box::new(order.iter())
        } else {
            Box::new(order.iter().rev())
        };
        for &i in iter {
            while hull.len() >= start + 2
                && cross(
                    points[hull[hull.len() - 2]],
                    points[hull[hull.len() - 1]],
                    points[i],
                ) <= 0.0
            {
                hull.pop();
            }
            hull.push(i);
        }
        hull.pop();
    }
    hull
}

fn circumcircle_contains(a: DVec2, b: DVec2, c: DVec2, p: DVec2) -> bool {
    let (ax, ay) = (a.x - p.x, a.y - p.y);
    let (bx, by) = (b.x - p.x, b.y - p.y);
    let (cx, cy) = (c.x - p.x, c.y - p.y);
    let det = (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay);
    // Orientation-independent: positive for CCW triangles.
    if cross(a, b, c) > 0.0 {
        det > 0.0
    } else {
        det < 0.0
    }
}

/// Bowyer–Watson with a super-triangle; returns CCW triangles over sample indices.
fn delaunay(points: &[DVec2]) -> Vec<[usize; 3]> {
    let min = points.iter().fold(DVec2::INFINITY, |m, p| m.min(*p));
    let max = points.iter().fold(DVec2::NEG_INFINITY, |m, p| m.max(*p));
    let span = (max - min).max_element().max(1e-9);
    let mid = (min + max) * 0.5;
    let n = points.len();
    let mut all: Vec<DVec2> = points.to_vec();
    all.push(mid + DVec2::new(-20.0 * span, -span));
    all.push(mid + DVec2::new(0.0, 20.0 * span));
    all.push(mid + DVec2::new(20.0 * span, -span));
    let mut tris: Vec<[usize; 3]> = vec![[n, n + 1, n + 2]];
    for (i, &p) in points.iter().enumerate() {
        let mut bad = Vec::new();
        for (t, &[a, b, c]) in tris.iter().enumerate() {
            if circumcircle_contains(all[a], all[b], all[c], p) {
                bad.push(t);
            }
        }
        // Boundary edges of the cavity appear exactly once.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for &t in &bad {
            let [a, b, c] = tris[t];
            for e in [(a, b), (b, c), (c, a)] {
                if let Some(pos) = edges.iter().position(|&(x, y)| (x, y) == (e.1, e.0)) {
                    edges.swap_remove(pos);
                } else {
                    edges.push(e);
                }
            }
        }
        for &t in bad.iter().rev() {
            tris.swap_remove(t);
        }
        for (a, b) in edges {
            let tri = if cross(all[a], all[b], p) > 0.0 {
                [a, b, i]
            } else {
                [b, a, i]
            };
            tris.push(tri);
        }
    }
    tris.retain(|t| t.iter().all(|&v| v < n));
    tris.sort_unstable();
    tris
}

#[cfg(test)]
mod tests;

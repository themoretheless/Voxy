//! Root-anchored Verlet strands with position constraints. Use a fixed timestep.
use crate::math::add;
use crate::math::scale;
use crate::math::sub;
#[derive(Clone, Copy, Debug)]
pub struct StrandConfig {
    /// Velocity retained per second, in 0..=1.
    pub damping: f64,
    /// Acceleration toward the root-relative rest pose, in s^-2.
    pub stiffness: f64,
    pub iterations: usize,
    pub radius: f64,
}
impl Default for StrandConfig {
    fn default() -> Self {
        Self {
            damping: 0.05,
            stiffness: 20.0,
            iterations: 24,
            radius: 0.01,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SphereCollider {
    pub center: [f64; 3],
    pub radius: f64,
}
#[derive(Clone, Debug)]
pub struct Strand {
    positions: Vec<[f64; 3]>,
    previous: Vec<[f64; 3]>,
    rest: Vec<[f64; 3]>,
    lengths: Vec<f64>,
    config: StrandConfig,
}
fn length(a: [f64; 3]) -> f64 {
    a.iter().map(|v| v * v).sum::<f64>().sqrt()
}
fn finite(a: [f64; 3]) -> bool {
    a.iter().all(|v| v.is_finite())
}
impl Strand {
    /// Creates a strand; the first particle is pinned. Coordinates use local units.
    /// # Errors
    /// Rejects invalid configuration, nonfinite points and zero-length segments.
    pub fn new(points: Vec<[f64; 3]>, config: StrandConfig) -> Result<Self, &'static str> {
        let lengths: Vec<_> = points.windows(2).map(|p| length(sub(p[1], p[0]))).collect();
        if points.len() < 2
            || !points.iter().copied().all(finite)
            || lengths.iter().any(|l| !l.is_finite() || *l <= 1e-9)
            || !config.damping.is_finite()
            || !(0.0..=1.0).contains(&config.damping)
            || !config.stiffness.is_finite()
            || config.stiffness < 0.0
            || !config.radius.is_finite()
            || config.radius < 0.0
            || config.iterations == 0
            || config.iterations > 256
        {
            return Err("invalid strand");
        }
        let rest = points.iter().map(|&p| sub(p, points[0])).collect();
        Ok(Self {
            previous: points.clone(),
            positions: points,
            rest,
            lengths,
            config,
        })
    }
    /// Creates world-space particles whose rest offsets are expressed in `frame`.
    /// # Errors
    /// Rejects invalid strands and frames.
    pub fn new_oriented(
        points: Vec<[f64; 3]>,
        config: StrandConfig,
        frame: [[f64; 3]; 3],
    ) -> Result<Self, &'static str> {
        let mut strand = Self::new(points, config)?;
        // Reuse the step admission without publishing the scratch step.
        let mut scratch = strand.clone();
        scratch.step_oriented(1.0 / 120.0, strand.positions[0], [0.0; 3], &[], frame)?;
        for offset in &mut strand.rest {
            *offset = std::array::from_fn(|i| {
                frame[i].into_iter().zip(*offset).map(|(a, b)| a * b).sum()
            });
        }
        Ok(strand)
    }
    #[must_use]
    pub fn positions(&self) -> &[[f64; 3]] {
        &self.positions
    }
    /// Moving the root leaves free particles behind, producing attachment inertia.
    /// Collision is discrete, particle versus sphere; segments and self-collision are not solved.
    /// # Errors
    /// Rejects invalid inputs and steps above 1/30 second without changing state.
    pub fn step(
        &mut self,
        dt: f64,
        root: [f64; 3],
        acceleration: [f64; 3],
        colliders: &[SphereCollider],
    ) -> Result<(), &'static str> {
        self.step_oriented(
            dt,
            root,
            acceleration,
            colliders,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        )
    }

    /// Advances a moving groom with an orthonormal world-space rest frame.
    /// The frame is column-major. Roots remain pinned; free particles retain inertia.
    /// # Errors
    /// Rejects non-orthonormal frames and invalid inputs without changing state.
    pub fn step_oriented(
        &mut self,
        dt: f64,
        root: [f64; 3],
        acceleration: [f64; 3],
        colliders: &[SphereCollider],
        frame: [[f64; 3]; 3],
    ) -> Result<(), &'static str> {
        let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        if !frame.iter().copied().all(finite)
            || (0..3).any(|i| (dot(frame[i], frame[i]) - 1.0).abs() > 1e-8)
            || (0..3).any(|i| (i + 1..3).any(|j| dot(frame[i], frame[j]).abs() > 1e-8))
        {
            return Err("invalid strand rest frame");
        }
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 1.0 / 30.0
            || !finite(root)
            || !finite(acceleration)
            || colliders
                .iter()
                .any(|c| !finite(c.center) || !c.radius.is_finite() || c.radius < 0.0)
        {
            return Err("invalid strand step");
        }
        let mut next = self.positions.clone();
        next[0] = root;
        let damping = self.config.damping.powf(dt);
        for (i, p) in next.iter_mut().enumerate().skip(1) {
            let rest =
                std::array::from_fn(|axis| (0..3).map(|j| frame[j][axis] * self.rest[i][j]).sum());
            let spring = scale(sub(add(root, rest), *p), self.config.stiffness);
            *p = add(
                add(*p, scale(sub(*p, self.previous[i]), damping)),
                scale(add(acceleration, spring), dt * dt),
            );
        }
        for _ in 0..self.config.iterations {
            for i in 1..next.len() {
                let delta = sub(next[i], next[i - 1]);
                let distance = length(delta);
                let direction = if distance > 1e-12 {
                    scale(delta, 1.0 / distance)
                } else {
                    let rest_delta = sub(self.rest[i], self.rest[i - 1]);
                    scale(
                        std::array::from_fn(|axis| {
                            (0..3).map(|j| frame[j][axis] * rest_delta[j]).sum()
                        }),
                        1.0 / self.lengths[i - 1],
                    )
                };
                let correction = scale(direction, distance - self.lengths[i - 1]);
                if i == 1 {
                    next[i] = sub(next[i], correction);
                } else {
                    next[i - 1] = add(next[i - 1], scale(correction, 0.5));
                    next[i] = sub(next[i], scale(correction, 0.5));
                }
            }
            for p in next.iter_mut().skip(1) {
                for c in colliders {
                    let delta = sub(*p, c.center);
                    let distance = length(delta);
                    let radius = c.radius + self.config.radius;
                    if distance < radius {
                        let direction = if distance > 1e-12 {
                            scale(delta, 1.0 / distance)
                        } else {
                            [1.0, 0.0, 0.0]
                        };
                        *p = add(c.center, scale(direction, radius));
                    }
                }
            }
        }
        if !next.iter().copied().all(finite) {
            return Err("strand overflow");
        }
        self.previous.clone_from(&self.positions);
        self.positions = next;
        Ok(())
    }
}

#[cfg(test)]
mod oriented_tests {
    use super::*;
    #[test]
    fn rotated_groom_preserves_authored_rest_direction() {
        let frame = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let mut strand = Strand::new_oriented(
            vec![[2.0, 3.0, 0.0], [2.0, 3.02, 0.0], [2.0, 3.04, 0.0]],
            StrandConfig {
                stiffness: 900.0,
                ..StrandConfig::default()
            },
            frame,
        )
        .unwrap();
        for _ in 0..60 {
            strand
                .step_oriented(1.0 / 120.0, [2.0, 3.0, 0.0], [0.0; 3], &[], frame)
                .unwrap();
        }
        assert!((strand.positions()[2][1] - 3.04).abs() < 1e-10);
        assert!((strand.positions()[2][0] - 2.0).abs() < 1e-10);
    }
    #[test]
    fn invalid_groom_frame_is_atomic() {
        let mut strand =
            Strand::new(vec![[0.0; 3], [0.0, 0.02, 0.0]], StrandConfig::default()).unwrap();
        let before = strand.positions().to_vec();
        assert!(
            strand
                .step_oriented(1.0 / 60.0, [1.0, 0.0, 0.0], [0.0; 3], &[], [[0.0; 3]; 3])
                .is_err()
        );
        assert_eq!(strand.positions(), before);
    }
}

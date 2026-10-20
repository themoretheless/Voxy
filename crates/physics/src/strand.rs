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
/// Axis-aligned solid obstacle in the strand's simulation coordinates.
#[derive(Clone, Copy, Debug)]
pub struct BoxCollider {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
#[derive(Debug)]
pub struct Strand {
    positions: Vec<[f64; 3]>,
    previous: Vec<[f64; 3]>,
    scratch: Vec<[f64; 3]>,
    rest: Vec<[f64; 3]>,
    lengths: Vec<f64>,
    config: StrandConfig,
}
impl Clone for Strand {
    fn clone(&self) -> Self {
        Self {
            positions: self.positions.clone(),
            previous: self.previous.clone(),
            scratch: self.scratch.clone(),
            rest: self.rest.clone(),
            lengths: self.lengths.clone(),
            config: self.config,
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.positions.clone_from(&source.positions);
        self.previous.clone_from(&source.previous);
        self.scratch.clone_from(&source.scratch);
        self.rest.clone_from(&source.rest);
        self.lengths.clone_from(&source.lengths);
        self.config = source.config;
    }
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
            scratch: Vec::with_capacity(points.len()),
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
        self.step_oriented_with_boxes(dt,root,acceleration,colliders,&[],frame)
    }

    /// Discrete free-particle contacts against spheres and expanded solid boxes.
    /// Pinned roots are preserved. Segment and strand self-collision are not solved.
    /// # Errors
    /// Invalid bounds or frame data reject the step before changing state.
    pub fn step_oriented_with_boxes(
        &mut self,
        dt: f64,
        root: [f64; 3],
        acceleration: [f64; 3],
        colliders: &[SphereCollider],
        boxes: &[BoxCollider],
        frame: [[f64; 3]; 3],
    ) -> Result<(), &'static str> {
        if boxes.iter().any(|b| !finite(b.min)||!finite(b.max)||(0..3).any(|i|b.min[i]>=b.max[i])) {
            return Err("invalid strand box collider");
        }
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
        self.scratch.clone_from(&self.positions);
        let next = &mut self.scratch;
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
                *p=project_box_union(*p,boxes,self.config.radius);
            }
        }
        if !next.iter().copied().all(finite) {
            return Err("strand overflow");
        }
        // Rotate three owned buffers: publish only after full finite admission.
        std::mem::swap(&mut self.previous, &mut self.positions);
        std::mem::swap(&mut self.positions, &mut self.scratch);
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

#[cfg(test)]
mod scratch_tests {
    use super::*;
    #[test]
    fn clone_from_reuses_storage_and_copies_velocity_history() {
        let mut source = Strand::new(
            vec![[0., 0., 0.], [0., 0.02, 0.], [0., 0.04, 0.]],
            StrandConfig::default(),
        )
        .unwrap();
        let mut destination = source.clone();
        source
            .step(1. / 120., [0.02, 0., 0.], [0., -9.81, 0.], &[])
            .unwrap();
        destination.clone_from(&source); // First call admits scratch capacity.
        let addresses = |s: &Strand| {
            [
                s.positions.as_ptr() as usize,
                s.previous.as_ptr() as usize,
                s.scratch.as_ptr() as usize,
                s.rest.as_ptr() as usize,
                s.lengths.as_ptr() as usize,
            ]
        };
        let initial = addresses(&destination);
        for _ in 0..20 {
            destination.clone_from(&source);
            assert_eq!(addresses(&destination), initial);
        }
        for _ in 0..10 {
            for strand in [&mut source, &mut destination] {
                strand
                    .step(1. / 120., [0.04, 0., 0.], [0., -9.81, 0.], &[])
                    .unwrap();
            }
            assert_eq!(destination.positions, source.positions);
            assert_eq!(destination.previous, source.previous);
        }
    }

    #[test]
    fn overflow_preserves_state_and_scratch_is_safe_to_reuse() {
        let mut strand = Strand::new(
            vec![[0., 0., 0.], [0., 0.02, 0.], [0., 0.04, 0.]],
            StrandConfig::default(),
        )
        .unwrap();
        let mut control = strand.clone();
        let old = strand.positions.clone();
        let previous = strand.previous.clone();
        assert_eq!(
            strand.step(1. / 120., [f64::MAX; 3], [f64::MAX; 3], &[]),
            Err("strand overflow")
        );
        assert_eq!(strand.positions, old);
        assert_eq!(strand.previous, previous);
        for _ in 0..10 {
            strand
                .step(1. / 120., [0.01, 0., 0.], [0., -9.81, 0.], &[])
                .unwrap();
            control
                .step(1. / 120., [0.01, 0., 0.], [0., -9.81, 0.], &[])
                .unwrap();
            assert_eq!(strand.positions, control.positions);
            assert_eq!(strand.previous, control.previous);
        }
    }
    #[test]
    fn stepping_reuses_the_same_three_allocations() {
        let mut strand = Strand::new(
            vec![[0., 0., 0.], [0., 0.02, 0.], [0., 0.04, 0.]],
            StrandConfig::default(),
        )
        .unwrap();
        let addresses = |s: &Strand| {
            let mut a = [
                s.positions.as_ptr() as usize,
                s.previous.as_ptr() as usize,
                s.scratch.as_ptr() as usize,
            ];
            a.sort();
            a
        };
        let initial = addresses(&strand);
        for _ in 0..100 {
            strand
                .step(1. / 120., [0.01, 0., 0.], [0., -9.81, 0.], &[])
                .unwrap();
            assert_eq!(addresses(&strand), initial);
        }
    }
}

// Find the nearest axis-aligned exit from the solid union. Expanded adjacent
// voxels overlap: treating their shared faces as surfaces traps particles at
// seams. Connected intervals along each axis discard those internal faces.
fn project_box_union(p: [f64;3], boxes: &[BoxCollider], radius:f64)->[f64;3] {
    if !boxes.iter().any(|b|(0..3).all(|i|p[i]>b.min[i]-radius&&p[i]<b.max[i]+radius)) {return p;}
    let mut nearest=(f64::INFINITY,0,0.0);
    for axis in 0..3 {
        let (mut lo,mut hi)=(p[axis],p[axis]);
        loop {
            let before=(lo,hi);
            for b in boxes {
                if (0..3).filter(|&i|i!=axis).all(|i|p[i]>b.min[i]-radius&&p[i]<b.max[i]+radius)
                    && b.min[axis]-radius<=hi && b.max[axis]+radius>=lo {
                    lo=lo.min(b.min[axis]-radius);
                    hi=hi.max(b.max[axis]+radius);
                }
            }
            if before==(lo,hi) {break;}
        }
        for boundary in [lo,hi] {
            let distance=(p[axis]-boundary).abs();
            if distance<nearest.0 {nearest=(distance,axis,boundary);}
        }
    }
    let mut projected=p;
    projected[nearest.1]=nearest.2;
    projected
}

#[cfg(test)]
mod box_contact_tests {
    use super::*;
    const FRAME: [[f64;3];3]=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
    #[test]
    fn floor_contacts_preserve_the_pinned_root_and_particle_clearance() {
        let root=[0.,0.08,0.];
        let mut strand=Strand::new_oriented(vec![root,[0.04,-0.03,0.],[0.10,-0.08,0.]],StrandConfig{radius:0.002,..Default::default()},FRAME).unwrap();
        let floor=BoxCollider{min:[-2.,-2.,-2.],max:[2.,0.,2.]};
        for _ in 0..120 {
            strand.step_oriented_with_boxes(1./60.,root,[0.,-9.81,0.],&[],&[floor],FRAME).unwrap();
            assert_eq!(strand.positions()[0],root);
            assert!(strand.positions().iter().skip(1).all(|p|p[1]>=0.002-1e-12));
        }
    }
    #[test]
    fn invalid_box_rejects_before_publishing_positions() {
        let mut strand=Strand::new_oriented(vec![[0.,1.,0.],[0.1,1.,0.]],StrandConfig::default(),FRAME).unwrap();
        let before=strand.positions().to_vec();
        let invalid=BoxCollider{min:[0.;3],max:[0.;3]};
        assert!(strand.step_oriented_with_boxes(1./60.,[1.,1.,1.],[0.;3],&[],&[invalid],FRAME).is_err());
        assert_eq!(strand.positions(),before);
    }
}

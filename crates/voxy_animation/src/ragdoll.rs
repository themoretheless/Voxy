//! Self-contained articulated ragdoll: box bodies attached to joints,
//! position-based joint constraints with swing/twist limits, rotational
//! motors toward an animated target (powered ragdoll), and box-vs-environment
//! contacts through a caller trait. Deterministic, no physics-crate dependency.

use std::sync::Arc;

use glam::{DQuat, DVec3, Quat, Vec3};

use crate::ik::limits::JointLimit;
use crate::{AnimationError, BoneMask, Joint, Pose, Skeleton, Transform, rigs_match};

/// Constraint and contact passes per substep.
const CONSTRAINT_ITERATIONS: usize = 4;

/// Rigid body mapped onto one skeleton joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RagdollBody {
    pub joint: u16,
    pub half_extents: Vec3,
    pub mass: f32,
    /// Body frame relative to the joint frame.
    pub offset: Transform,
}

/// Constraint between two bodies with anchors in each body frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RagdollJoint {
    pub parent: usize,
    pub child: usize,
    pub anchor_parent: Vec3,
    pub anchor_child: Vec3,
    pub limit: Option<JointLimit>,
}

#[derive(Clone, Debug, Default)]
pub struct RagdollDesc {
    pub bodies: Vec<RagdollBody>,
    pub joints: Vec<RagdollJoint>,
}

/// Contact returned by the environment for one body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactManifold {
    /// Model-space point on the body pushed out along `normal`.
    pub point: DVec3,
    pub normal: DVec3,
    pub depth: f64,
}

pub trait RagdollEnvironment {
    fn resolve(
        &self,
        center: DVec3,
        half_extents: DVec3,
        rotation: DQuat,
    ) -> Option<ContactManifold>;
    fn gravity(&self) -> DVec3;
    /// Coulomb friction coefficient for contacts.
    fn friction(&self) -> f64 {
        0.6
    }
}

/// Infinite plane `normal · x = offset`, resolving the deepest box corner.
#[derive(Clone, Copy, Debug)]
pub struct PlaneEnvironment {
    pub normal: DVec3,
    pub offset: f64,
    pub gravity: DVec3,
}

impl RagdollEnvironment for PlaneEnvironment {
    /// Averages all penetrating corners (a contact patch) at the deepest
    /// depth, so a flat landing produces a pure translation instead of a
    /// torque from one arbitrarily chosen corner.
    fn resolve(
        &self,
        center: DVec3,
        half_extents: DVec3,
        rotation: DQuat,
    ) -> Option<ContactManifold> {
        let mut sum = DVec3::ZERO;
        let mut count = 0.0;
        let mut deepest = 0.0f64;
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    let corner = center + rotation * (half_extents * DVec3::new(sx, sy, sz));
                    let depth = self.offset - corner.dot(self.normal);
                    if depth > 0.0 {
                        sum += corner;
                        count += 1.0;
                        deepest = deepest.max(depth);
                    }
                }
            }
        }
        (count > 0.0).then(|| ContactManifold {
            point: sum / count,
            normal: self.normal,
            depth: deepest,
        })
    }

    fn gravity(&self) -> DVec3 {
        self.gravity
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorSettings {
    /// Fraction of the remaining rotation toward the target removed per substep, in `[0, 1]`.
    pub stiffness: f32,
    /// Fraction of angular velocity removed per second, in `[0, 1]`.
    pub damping: f32,
}

#[derive(Clone, Debug)]
pub struct Ragdoll {
    rig: Arc<[Joint]>,
    desc: Arc<RagdollDesc>,
    /// Body index per joint.
    map: Vec<Option<usize>>,
    inverse_mass: Vec<f64>,
    /// Inverse inertia in body frame (diagonal, box).
    inverse_inertia: Vec<DVec3>,
    /// Per joint: bodies on the child side and on the parent side.
    sides: Vec<(Vec<usize>, Vec<usize>)>,
}

/// Body positions, orientations and velocities in model space.
#[derive(Clone, Debug, PartialEq)]
pub struct RagdollState {
    pub positions: Vec<DVec3>,
    pub rotations: Vec<DQuat>,
    pub linear_velocities: Vec<DVec3>,
    pub angular_velocities: Vec<DVec3>,
}

impl Ragdoll {
    /// # Errors
    /// Rejects unknown or repeated joints, nonpositive mass/extents, invalid
    /// offsets, dangling joint indices or self-joints.
    pub fn new(skeleton: &Skeleton, desc: RagdollDesc) -> Result<Self, AnimationError> {
        let joints = skeleton.joints();
        if desc.bodies.is_empty() {
            return Err(AnimationError::InvalidRagdoll);
        }
        let mut map = vec![None; joints.len()];
        let mut inverse_mass = Vec::with_capacity(desc.bodies.len());
        let mut inverse_inertia = Vec::with_capacity(desc.bodies.len());
        for (index, body) in desc.bodies.iter().enumerate() {
            let joint = usize::from(body.joint);
            if joint >= joints.len()
                || map[joint].is_some()
                || !body.mass.is_finite()
                || body.mass <= 0.0
                || !body.half_extents.is_finite()
                || body.half_extents.min_element() <= 0.0
                || !body.offset.is_valid()
            {
                return Err(AnimationError::InvalidRagdoll);
            }
            map[joint] = Some(index);
            let m = f64::from(body.mass);
            let h = body.half_extents.as_dvec3() * 2.0;
            let inertia = DVec3::new(
                m / 12.0 * (h.y * h.y + h.z * h.z),
                m / 12.0 * (h.x * h.x + h.z * h.z),
                m / 12.0 * (h.x * h.x + h.y * h.y),
            );
            inverse_mass.push(1.0 / m);
            inverse_inertia.push(1.0 / inertia);
        }
        for joint in &desc.joints {
            if joint.parent >= desc.bodies.len()
                || joint.child >= desc.bodies.len()
                || joint.parent == joint.child
                || !joint.anchor_parent.is_finite()
                || !joint.anchor_child.is_finite()
            {
                return Err(AnimationError::InvalidRagdoll);
            }
        }
        let sides = joint_sides(&desc);
        Ok(Self {
            rig: skeleton.joints.clone(),
            desc: Arc::new(desc),
            map,
            inverse_mass,
            inverse_inertia,
            sides,
        })
    }

    #[must_use]
    pub fn body_count(&self) -> usize {
        self.desc.bodies.len()
    }

    fn globals(&self, pose: &Pose) -> Result<Vec<(DVec3, DQuat)>, AnimationError> {
        let mut out: Vec<(DVec3, DQuat)> = Vec::with_capacity(pose.local.len());
        for (i, (local, joint)) in pose.local.iter().zip(self.rig.iter()).enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(i));
            }
            let r = DQuat::from_array(local.rotation.to_array().map(f64::from)).normalize();
            let t = local.translation.as_dvec3();
            let (position, rotation) = match joint.parent {
                Some(parent) => {
                    let (pp, pr) = out[usize::from(parent)];
                    (pp + pr * t, (pr * r).normalize())
                }
                None => (t, r),
            };
            out.push((position, rotation));
        }
        Ok(out)
    }

    /// Builds body state from a pose; `previous` with `dt` supplies velocities.
    ///
    /// # Errors
    /// Rejects foreign rigs, invalid poses or a nonpositive `dt` with `previous`.
    pub fn state_from_pose(
        &self,
        pose: &Pose,
        previous: Option<&Pose>,
        dt: f64,
    ) -> Result<RagdollState, AnimationError> {
        if !rigs_match(&pose.rig, &self.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let now = self.globals(pose)?;
        let before = match previous {
            Some(p) => {
                if !rigs_match(&p.rig, &self.rig) || !dt.is_finite() || dt <= 0.0 {
                    return Err(AnimationError::InvalidAnimationTimeStep);
                }
                Some(self.globals(p)?)
            }
            None => None,
        };
        let mut state = RagdollState {
            positions: Vec::new(),
            rotations: Vec::new(),
            linear_velocities: Vec::new(),
            angular_velocities: Vec::new(),
        };
        for body in &self.desc.bodies {
            let frame = |g: &[(DVec3, DQuat)]| {
                let (p, r) = g[usize::from(body.joint)];
                let offset_r = DQuat::from_array(body.offset.rotation.to_array().map(f64::from));
                (
                    p + r * body.offset.translation.as_dvec3(),
                    (r * offset_r).normalize(),
                )
            };
            let (p, r) = frame(&now);
            let (v, w) = match &before {
                Some(b) => {
                    let (p0, r0) = frame(b);
                    let delta = (r * r0.conjugate()).normalize();
                    let (axis, angle) = delta.to_axis_angle();
                    let angle = if angle > std::f64::consts::PI {
                        angle - 2.0 * std::f64::consts::PI
                    } else {
                        angle
                    };
                    ((p - p0) / dt, axis * (angle / dt))
                }
                None => (DVec3::ZERO, DVec3::ZERO),
            };
            state.positions.push(p);
            state.rotations.push(r);
            state.linear_velocities.push(v);
            state.angular_velocities.push(w);
        }
        Ok(state)
    }

    /// Advances the state by `dt` with `substeps` XPBD iterations. `target`
    /// drives motors toward the animated pose's joint rotations, scaled per
    /// joint by the mask.
    ///
    /// # Errors
    /// Rejects invalid steps, mismatched state sizes and foreign target rigs.
    pub fn step(
        &self,
        state: &mut RagdollState,
        target: Option<(&Pose, &MotorSettings, &BoneMask)>,
        env: &dyn RagdollEnvironment,
        dt: f64,
        substeps: u8,
    ) -> Result<(), AnimationError> {
        let n = self.desc.bodies.len();
        if !dt.is_finite() || dt < 0.0 || substeps == 0 {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        if state.positions.len() != n
            || state.rotations.len() != n
            || state.linear_velocities.len() != n
            || state.angular_velocities.len() != n
        {
            return Err(AnimationError::InvalidRagdoll);
        }
        let targets = match target {
            Some((pose, motor, mask)) => {
                if !rigs_match(&pose.rig, &self.rig) || mask.weights().len() != self.rig.len() {
                    return Err(AnimationError::SkeletonMismatch);
                }
                if !motor.stiffness.is_finite()
                    || !(0.0..=1.0).contains(&motor.stiffness)
                    || !motor.damping.is_finite()
                    || !(0.0..=1.0).contains(&motor.damping)
                {
                    return Err(AnimationError::InvalidRagdoll);
                }
                let g = self.globals(pose)?;
                // Target relative rotation parent→child per joint constraint.
                let relative: Vec<Option<(DQuat, f64)>> = self
                    .desc
                    .joints
                    .iter()
                    .map(|j| {
                        let pb = &self.desc.bodies[j.parent];
                        let cb = &self.desc.bodies[j.child];
                        let frame = |b: &RagdollBody| {
                            let (_, r) = g[usize::from(b.joint)];
                            (r * DQuat::from_array(b.offset.rotation.to_array().map(f64::from)))
                                .normalize()
                        };
                        let weight = f64::from(mask.weights()[usize::from(cb.joint)]);
                        (weight > 0.0)
                            .then(|| ((frame(pb).conjugate() * frame(cb)).normalize(), weight))
                    })
                    .collect();
                Some((relative, *motor))
            }
            None => None,
        };
        let h = dt / f64::from(substeps);
        let gravity = env.gravity();
        let friction = env.friction();
        let mut candidate = state.clone();
        for _ in 0..substeps {
            let prev_positions = candidate.positions.clone();
            let prev_rotations = candidate.rotations.clone();
            // Motor displacement is kinematic: it moves bodies toward the
            // animation without generating momentum.
            let mut kinematic = Kinematic {
                positions: vec![DVec3::ZERO; n],
                rotations: vec![DQuat::IDENTITY; n],
            };
            // Integrate.
            for i in 0..n {
                candidate.linear_velocities[i] += gravity * h;
                if let Some((_, motor)) = &targets {
                    // `damping` is the fraction of angular velocity removed per second.
                    let keep = 1.0 - f64::from(motor.damping);
                    let factor = if keep <= 0.0 { 0.0 } else { keep.powf(h) };
                    candidate.angular_velocities[i] *= factor;
                }
                candidate.positions[i] += candidate.linear_velocities[i] * h;
                let w = candidate.angular_velocities[i];
                let dq = DQuat::from_xyzw(w.x, w.y, w.z, 0.0) * candidate.rotations[i];
                candidate.rotations[i] = (candidate.rotations[i]
                    + DQuat::from_xyzw(
                        dq.x * 0.5 * h,
                        dq.y * 0.5 * h,
                        dq.z * 0.5 * h,
                        dq.w * 0.5 * h,
                    ))
                .normalize();
            }
            // Joint constraints: anchors coincide, limits, motors. Motors and
            // limits of one joint reopen neighbouring anchors, so iterate and
            // finish with an anchor-only pass.
            for _ in 0..CONSTRAINT_ITERATIONS {
                for (ji, joint) in self.desc.joints.iter().enumerate() {
                    self.solve_anchor(&mut candidate, joint);
                    if let Some((relative, motor)) = &targets
                        && let Some((target_rel, weight)) = relative[ji]
                    {
                        self.solve_motor(
                            &mut candidate,
                            &mut kinematic,
                            ji,
                            target_rel,
                            f64::from(motor.stiffness) * weight,
                        );
                    }
                    if let Some(limit) = &joint.limit {
                        self.solve_limit(&mut candidate, ji, limit);
                    }
                }
            }
            for joint in self.desc.joints.iter() {
                self.solve_anchor(&mut candidate, joint);
            }
            // Contacts: several passes so rotations from one corner's push-out
            // do not leave another corner inside the surface.
            let mut contacts: Vec<(usize, ContactManifold)> = Vec::new();
            for pass in 0..CONSTRAINT_ITERATIONS {
                for i in 0..n {
                    let half = self.desc.bodies[i].half_extents.as_dvec3();
                    if let Some(contact) =
                        env.resolve(candidate.positions[i], half, candidate.rotations[i])
                        && contact.depth > 0.0
                    {
                        self.push_out(&mut candidate, i, &contact);
                        if pass == 0 {
                            contacts.push((i, contact));
                        }
                    }
                }
            }
            // Velocities from positions; contacts remove inward normal velocity
            // and apply Coulomb friction.
            for i in 0..n {
                candidate.linear_velocities[i] =
                    (candidate.positions[i] - prev_positions[i] - kinematic.positions[i]) / h;
                let delta = (candidate.rotations[i]
                    * (kinematic.rotations[i] * prev_rotations[i]).conjugate())
                .normalize();
                let (axis, angle) = delta.to_axis_angle();
                let angle = if angle > std::f64::consts::PI {
                    angle - 2.0 * std::f64::consts::PI
                } else {
                    angle
                };
                candidate.angular_velocities[i] = axis * (angle / h);
            }
            for (i, contact) in contacts {
                let r = contact.point - candidate.positions[i];
                let v = candidate.linear_velocities[i] + candidate.angular_velocities[i].cross(r);
                let vn = v.dot(contact.normal);
                let vt = v - contact.normal * vn;
                let mut impulse = DVec3::ZERO;
                if vn < 0.0 {
                    impulse -= contact.normal * vn;
                }
                let tangential = vt.length();
                if tangential > 0.0 {
                    let max_friction = friction * vn.abs().max(gravity.length() * h);
                    impulse -= vt / tangential * tangential.min(max_friction);
                }
                self.apply_velocity_impulse(&mut candidate, i, r, impulse);
            }
        }
        if candidate.positions.iter().any(|p| !p.is_finite())
            || candidate.rotations.iter().any(|r| !r.is_finite())
        {
            return Err(AnimationError::NumericalOverflow);
        }
        *state = candidate;
        Ok(())
    }

    fn apply_velocity_impulse(&self, s: &mut RagdollState, i: usize, r: DVec3, delta_v: DVec3) {
        // Point-velocity correction split by generalized inverse mass.
        let rotation = s.rotations[i];
        let inv_inertia_world =
            |v: DVec3| rotation * (self.inverse_inertia[i] * (rotation.conjugate() * v));
        let n = delta_v.normalize_or_zero();
        if n == DVec3::ZERO {
            return;
        }
        let w = self.inverse_mass[i] + r.cross(n).dot(inv_inertia_world(r.cross(n)));
        if w <= 0.0 {
            return;
        }
        let impulse = n * (delta_v.length() / w);
        s.linear_velocities[i] += impulse * self.inverse_mass[i];
        s.angular_velocities[i] += inv_inertia_world(r.cross(impulse));
    }

    fn push_out(&self, s: &mut RagdollState, i: usize, contact: &ContactManifold) {
        let r = contact.point - s.positions[i];
        self.apply_positional(s, i, r, contact.normal * contact.depth);
    }

    /// Moves the point at body offset `r` by `delta` using generalized inverse mass.
    fn apply_positional(&self, s: &mut RagdollState, i: usize, r: DVec3, delta: DVec3) {
        let n = delta.normalize_or_zero();
        if n == DVec3::ZERO {
            return;
        }
        let rotation = s.rotations[i];
        let inv_inertia_world =
            |v: DVec3| rotation * (self.inverse_inertia[i] * (rotation.conjugate() * v));
        let w = self.inverse_mass[i] + r.cross(n).dot(inv_inertia_world(r.cross(n)));
        if w <= 0.0 {
            return;
        }
        let p = n * (delta.length() / w);
        s.positions[i] += p * self.inverse_mass[i];
        let dtheta = inv_inertia_world(r.cross(p));
        s.rotations[i] = rotate_by(s.rotations[i], dtheta);
    }

    fn solve_anchor(&self, s: &mut RagdollState, joint: &RagdollJoint) {
        let (a, b) = (joint.parent, joint.child);
        let ra = s.rotations[a] * joint.anchor_parent.as_dvec3();
        let rb = s.rotations[b] * joint.anchor_child.as_dvec3();
        let pa = s.positions[a] + ra;
        let pb = s.positions[b] + rb;
        let gap = pb - pa;
        let n = gap.normalize_or_zero();
        if n == DVec3::ZERO {
            return;
        }
        let wa = self.generalized_inverse_mass(s, a, ra, n);
        let wb = self.generalized_inverse_mass(s, b, rb, n);
        let total = wa + wb;
        if total <= 0.0 {
            return;
        }
        let correction = gap.length() / total;
        self.apply_positional(s, a, ra, n * (correction * wa));
        self.apply_positional(s, b, rb, -n * (correction * wb));
    }

    fn generalized_inverse_mass(&self, s: &RagdollState, i: usize, r: DVec3, n: DVec3) -> f64 {
        let rotation = s.rotations[i];
        let rn = r.cross(n);
        let local = rotation.conjugate() * rn;
        self.inverse_mass[i] + rn.dot(rotation * (self.inverse_inertia[i] * local))
    }

    /// Rotates the child side and the parent side of `joint` about the shared
    /// anchor so the relative rotation changes by `delta` (parent frame).
    /// Whole sides move rigidly, keeping every other anchor closed; the split
    /// by side mass keeps the centre of mass in place.
    fn apply_relative_rotation(
        &self,
        s: &mut RagdollState,
        kinematic: Option<&mut Kinematic>,
        joint_index: usize,
        delta: DQuat,
    ) {
        let joint = &self.desc.joints[joint_index];
        let (a, b) = (joint.parent, joint.child);
        let (axis, angle) = delta.to_axis_angle();
        let angle = if angle > std::f64::consts::PI {
            angle - 2.0 * std::f64::consts::PI
        } else {
            angle
        };
        if angle.abs() < 1e-12 {
            return;
        }
        let world_axis = (s.rotations[a] * axis).normalize();
        let (child_side, parent_side) = &self.sides[joint_index];
        let mass = |side: &[usize]| {
            side.iter()
                .map(|&i| 1.0 / self.inverse_mass[i])
                .sum::<f64>()
        };
        let (mc, mp) = (mass(child_side), mass(parent_side));
        let total = mc + mp;
        if total <= 0.0 {
            return;
        }
        let pa = s.positions[a] + s.rotations[a] * joint.anchor_parent.as_dvec3();
        let pb = s.positions[b] + s.rotations[b] * joint.anchor_child.as_dvec3();
        let pivot = (pa + pb) * 0.5;
        let mut kinematic = kinematic;
        for (side, share) in [
            (child_side, angle * mp / total),
            (parent_side, -angle * mc / total),
        ] {
            let r = DQuat::from_axis_angle(world_axis, share);
            for &i in side {
                let moved = pivot + r * (s.positions[i] - pivot);
                if let Some(k) = kinematic.as_deref_mut() {
                    k.positions[i] += moved - s.positions[i];
                    k.rotations[i] = (r * k.rotations[i]).normalize();
                }
                s.positions[i] = moved;
                s.rotations[i] = (r * s.rotations[i]).normalize();
            }
        }
    }

    fn solve_motor(
        &self,
        s: &mut RagdollState,
        kinematic: &mut Kinematic,
        joint_index: usize,
        target_rel: DQuat,
        fraction: f64,
    ) {
        let joint = &self.desc.joints[joint_index];
        let current =
            (s.rotations[joint.parent].conjugate() * s.rotations[joint.child]).normalize();
        let full = (target_rel * current.conjugate()).normalize();
        let partial = DQuat::IDENTITY
            .slerp(full, fraction.clamp(0.0, 1.0))
            .normalize();
        self.apply_relative_rotation(s, Some(kinematic), joint_index, partial);
    }

    fn solve_limit(&self, s: &mut RagdollState, joint_index: usize, limit: &JointLimit) {
        let joint = &self.desc.joints[joint_index];
        let current =
            (s.rotations[joint.parent].conjugate() * s.rotations[joint.child]).normalize();
        let axis = joint.anchor_child.as_dvec3().normalize_or_zero();
        let axis = if axis == DVec3::ZERO { DVec3::Y } else { -axis };
        let clamped = limit.clamp_delta(current, axis);
        let correction = (clamped * current.conjugate()).normalize();
        self.apply_relative_rotation(s, None, joint_index, correction);
    }

    /// Writes body orientations back into joint local rotations and the root
    /// body position into its joint translation; unmapped joints keep `base`.
    ///
    /// # Errors
    /// Rejects foreign rigs or mismatched state sizes.
    pub fn pose_from_state(
        &self,
        state: &RagdollState,
        base: &Pose,
    ) -> Result<Pose, AnimationError> {
        if !rigs_match(&base.rig, &self.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if state.positions.len() != self.desc.bodies.len() {
            return Err(AnimationError::InvalidRagdoll);
        }
        let mut pose = base.clone();
        // Global frames of the result, computed parent-first.
        let mut global: Vec<(DVec3, DQuat)> = Vec::with_capacity(self.rig.len());
        for (index, joint) in self.rig.iter().enumerate() {
            let parent = joint.parent.map(|p| global[usize::from(p)]);
            let frame = match self.map[index] {
                Some(body_index) => {
                    let body = &self.desc.bodies[body_index];
                    let offset_r =
                        DQuat::from_array(body.offset.rotation.to_array().map(f64::from));
                    let rotation = (state.rotations[body_index] * offset_r.conjugate()).normalize();
                    let position =
                        state.positions[body_index] - rotation * body.offset.translation.as_dvec3();
                    (position, rotation)
                }
                None => {
                    let local = pose.local[index];
                    let r = DQuat::from_array(local.rotation.to_array().map(f64::from)).normalize();
                    let t = local.translation.as_dvec3();
                    match parent {
                        Some((pp, pr)) => (pp + pr * t, (pr * r).normalize()),
                        None => (t, r),
                    }
                }
            };
            if self.map[index].is_some() {
                let local_rotation = match parent {
                    Some((_, pr)) => (pr.conjugate() * frame.1).normalize(),
                    None => frame.1,
                };
                pose.set_joint_rotation(
                    index,
                    Quat::from_array(local_rotation.to_array().map(|v| v as f32)).normalize(),
                )?;
                if parent.is_none() {
                    pose.set_joint_translation(index, frame.0.as_vec3())?;
                }
            }
            global.push(frame);
        }
        Ok(pose)
    }
}

/// Splits bodies into the child side and the parent side of each joint by
/// flood fill without crossing that joint. A cyclic graph falls back to the
/// two endpoint bodies.
fn joint_sides(desc: &RagdollDesc) -> Vec<(Vec<usize>, Vec<usize>)> {
    let n = desc.bodies.len();
    desc.joints
        .iter()
        .enumerate()
        .map(|(skip, joint)| {
            let flood = |start: usize| {
                let mut seen = vec![false; n];
                let mut stack = vec![start];
                seen[start] = true;
                while let Some(body) = stack.pop() {
                    for (k, other) in desc.joints.iter().enumerate() {
                        if k == skip {
                            continue;
                        }
                        for (from, to) in [(other.parent, other.child), (other.child, other.parent)]
                        {
                            if from == body && !seen[to] {
                                seen[to] = true;
                                stack.push(to);
                            }
                        }
                    }
                }
                seen
            };
            let child_seen = flood(joint.child);
            if child_seen[joint.parent] {
                return (vec![joint.child], vec![joint.parent]);
            }
            let child_side: Vec<usize> = (0..n).filter(|&i| child_seen[i]).collect();
            let parent_side: Vec<usize> = (0..n).filter(|&i| !child_seen[i]).collect();
            (child_side, parent_side)
        })
        .collect()
}

/// Per-substep displacement produced by motors, excluded from velocities.
struct Kinematic {
    positions: Vec<DVec3>,
    rotations: Vec<DQuat>,
}

fn rotate_by(q: DQuat, dtheta: DVec3) -> DQuat {
    let angle = dtheta.length();
    if angle < 1e-15 {
        return q;
    }
    (DQuat::from_axis_angle(dtheta / angle, angle) * q).normalize()
}

/// Final blend of an animated pose with the ragdoll pose per joint.
///
/// # Errors
/// Propagates masked blend errors.
pub fn powered_blend(
    animated: &Pose,
    ragdoll: &Pose,
    physics_weight: &BoneMask,
) -> Result<Pose, AnimationError> {
    Pose::blend_masked(animated, ragdoll, physics_weight)
}

#[cfg(test)]
mod tests;

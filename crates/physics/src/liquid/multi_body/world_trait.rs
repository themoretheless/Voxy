//! Geometry-owned contact witness types and backend trait for multi-body liquid coupling.
use super::{Error, GeometryHit, Particle, TranslatingBody, finite};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactWitness {
    pub point: [f64; 3],
    pub tolerance_m: f64,
}
/// Optional geometry-owned world point at the reported contact time. Legacy
/// translating backends can omit it; angular response must require a witness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyGeometryHit {
    pub geometry: GeometryHit,
    pub witness: Option<ContactWitness>,
}
impl From<GeometryHit> for BodyGeometryHit {
    fn from(geometry: GeometryHit) -> Self {
        Self {
            geometry,
            witness: None,
        }
    }
}

/// Query-local geometry-owned feature token. Existing contact callbacks can omit
/// it; feature-aware callbacks must validate it against the same geometry world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidGeometryHit {
    pub contact: BodyGeometryHit,
    pub feature: Option<u64>,
}
impl From<BodyGeometryHit> for RigidGeometryHit {
    fn from(contact: BodyGeometryHit) -> Self {
        Self {
            contact,
            feature: None,
        }
    }
}
/// Snapshot support point with its geometry-owned branch and world error budget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidSupportPoint {
    pub support: crate::contact::NormalSupport,
    pub feature: Option<u64>,
    pub tolerance_m: f64,
    /// Explicit numerical geometry admission budget; never a position correction.
    pub admission_error_m: f64,
    /// Set from the solved normal force when preparing this interval.
    pub carrying_reaction: bool,
}
/// Cubic world motion of a common support application point, starting at the
/// support snapshot. Geometry must admit this same model over the full interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidSupportPointMotion {
    pub velocity: [f64; 3],
    pub acceleration: [f64; 3],
    pub jerk: [f64; 3],
}
impl RigidSupportPointMotion {
    pub(super) fn validate(self) -> Result<Self, Error> {
        if !finite(self.velocity) || !finite(self.acceleration) || !finite(self.jerk) {
            return Err(Error::InvalidCollision);
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedGeometryHit {
    pub event: RigidGeometryHit,
    /// Maximum admitted nominal support/ownership excursion in metres.
    pub support_error_m: f64,
}
/// Geometry ownership stays with the backend; all fractions refer to this interval.
pub trait LiquidBodyWorld {
    fn sweep_particle_body(
        &self,
        particle: &Particle,
        radius: f64,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_body_pair(
        &self,
        first_index: usize,
        first: &TranslatingBody,
        second_index: usize,
        second: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_particle_environment(
        &self,
        particle: &Particle,
        radius: f64,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_body_environment(
        &self,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_particle_body_contact(
        &self,
        particle: &Particle,
        radius: f64,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_particle_body(particle, radius, body_index, body, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_body_pair_contact(
        &self,
        first_index: usize,
        first: &TranslatingBody,
        second_index: usize,
        second: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_body_pair(first_index, first, second_index, second, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_particle_environment_contact(
        &self,
        particle: &Particle,
        radius: f64,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_particle_environment(particle, radius, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_body_environment_contact(
        &self,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_body_environment(body_index, body, dt, max_candidates)
            .map(Into::into)
    }
    /// Prepared angular/accelerated trajectory; legacy backends reject both.
    fn sweep_particle_rigid_contact(
        &self,
        p: &Particle,
        radius: f64,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        if !path.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        let body = path.initial();
        if body.spin.is_some() || path.acceleration() != [0.; 3] {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_body_contact(
            p,
            radius,
            index,
            &TranslatingBody {
                position: body.motion.position,
                velocity: body.motion.velocity,
                mass: body.motion.mass,
            },
            path.duration(),
            budget,
        )
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        first: &crate::rigid_motion::RigidMotion,
        j: usize,
        second: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        if !first.has_constant_acceleration() || !second.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        let a = first.initial();
        let b = second.initial();
        if a.spin.is_some()
            || b.spin.is_some()
            || first.acceleration() != [0.; 3]
            || second.acceleration() != [0.; 3]
        {
            return Err(Error::CollisionBackend);
        }
        self.sweep_body_pair_contact(
            i,
            &TranslatingBody {
                position: a.motion.position,
                velocity: a.motion.velocity,
                mass: a.motion.mass,
            },
            j,
            &TranslatingBody {
                position: b.motion.position,
                velocity: b.motion.velocity,
                mass: b.motion.mass,
            },
            first.duration(),
            budget,
        )
    }
    fn sweep_rigid_environment_contact(
        &self,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        if !path.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        let body = path.initial();
        if body.spin.is_some() || path.acceleration() != [0.; 3] {
            return Err(Error::CollisionBackend);
        }
        self.sweep_body_environment_contact(
            index,
            &TranslatingBody {
                position: body.motion.position,
                velocity: body.motion.velocity,
                mass: body.motion.mass,
            },
            path.duration(),
            budget,
        )
    }
    /// Particle metadata plus the prepared drift trajectory. Linear-only
    /// adapters reject accelerated particles instead of replacing them by chords.
    fn sweep_particle_motion_body_event(
        &self,
        p: &Particle,
        radius: f64,
        particle: &crate::rigid_motion::RigidMotion,
        index: usize,
        body: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if !particle.has_constant_acceleration() || !body.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        if particle.acceleration() != [0.; 3] || particle.initial().spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_rigid_event(p, radius, index, body, budget)
    }
    fn sweep_particle_motion_environment_event(
        &self,
        p: &Particle,
        radius: f64,
        particle: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if !particle.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        if particle.acceleration() != [0.; 3] || particle.initial().spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_environment_contact(p, radius, particle.duration(), budget)
            .map(Into::into)
    }
    /// Particle sweep over a subinterval of the outer fixed step.
    fn sweep_particle_motion_environment_event_at(
        &self,
        particle: &Particle,
        radius: f64,
        path: &crate::rigid_motion::RigidMotion,
        _elapsed_s: f64,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        self.sweep_particle_motion_environment_event(particle, radius, path, budget)
    }

    /// Prescribed translational velocity of a particle's impact surface.
    fn particle_environment_impact_velocity(
        &self,
        _feature: Option<u64>,
        _time_s: f64,
    ) -> Result<[f64; 3], Error> {
        Ok([0.; 3])
    }

    fn sweep_particle_rigid_event(
        &self,
        p: &Particle,
        radius: f64,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if !path.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_rigid_contact(p, radius, index, path, budget)
            .map(Into::into)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        first: &crate::rigid_motion::RigidMotion,
        j: usize,
        second: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if !first.has_constant_acceleration() || !second.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_rigid_pair_contact(i, first, j, second, budget)
            .map(Into::into)
    }
    fn sweep_rigid_environment_event(
        &self,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if !path.has_constant_acceleration() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_rigid_environment_contact(index, path, budget)
            .map(Into::into)
    }
    /// Sweep over a subinterval of the outer fixed step. Moving geometry
    /// evaluates its own trajectory at elapsed time, including after impacts.
    fn sweep_rigid_environment_event_at(
        &self,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        _elapsed_s: f64,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        self.sweep_rigid_environment_event(index, path, budget)
    }

    /// Prescribed translational velocity at an admitted environment impact.
    /// Finite pairs and stationary geometry use the compatibility default.
    fn rigid_environment_impact_velocity(
        &self,
        _index: usize,
        _feature: Option<u64>,
        _time_s: f64,
    ) -> Result<[f64; 3], Error> {
        Ok([0.; 3])
    }

    /// Read the admitted patch at the same outer-step time as the sweep.
    fn rigid_environment_patch_at(
        &self,
        index: usize,
        body: &crate::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        _time_s: f64,
        budget: usize,
    ) -> Result<Vec<crate::contact::NormalContact>, Error> {
        self.rigid_environment_patch_for_feature(index, body, witness, normal, feature, budget)
    }

    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        first: &crate::contact::ContactBody,
        j: usize,
        second: &crate::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        _feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<crate::contact::NormalContact>, Error> {
        self.rigid_pair_patch(i, first, j, second, witness, normal, budget)
    }
    fn rigid_environment_patch_for_feature(
        &self,
        index: usize,
        body: &crate::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        _feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<crate::contact::NormalContact>, Error> {
        self.rigid_environment_patch(index, body, witness, normal, budget)
    }
    /// World velocity of the geometry-owned common application point.
    /// The compatibility default freezes its world arm relative to first COM.
    /// Moving feature owners must override this; the first material point is
    /// not automatically the owner of a clipped/common contact point.
    fn rigid_support_point_velocity(
        &self,
        bodies: &[crate::contact::ContactBody],
        support: crate::contact::NetworkSupport,
        _geometry: RigidSupportPoint,
    ) -> Result<[f64; 3], Error> {
        bodies
            .get(support.first)
            .map(|body| body.motion.velocity)
            .ok_or(Error::InvalidCollision)
    }

    /// Explicit finite-interval point model. None retains the existing point
    /// following first COM. Called first with zero load rates and again with
    /// solved rates; velocity and acceleration must agree between both calls.
    /// A custom model requires the matching trajectory-admission callbacks.
    fn rigid_support_point_motion(
        &self,
        _bodies: &[crate::contact::ContactBody],
        _loads: &[crate::contact::ContactWrench],
        _rates: &[crate::contact::ContactWrench],
        _support: crate::contact::NetworkSupport,
        _geometry: RigidSupportPoint,
    ) -> Result<Option<RigidSupportPointMotion>, Error> {
        Ok(None)
    }

    /// Geometry-owned second derivative for the exact emitted normal branch.
    /// Non-edge owners are derived by the shared contact kernel.
    fn rigid_support_normal_acceleration(
        &self,
        _bodies: &[crate::contact::ContactBody],
        _loads: &[crate::contact::ContactWrench],
        support: crate::contact::NetworkSupport,
        _geometry: RigidSupportPoint,
    ) -> Result<Option<[f64; 3]>, Error> {
        if matches!(
            support.support.plane,
            crate::contact::SupportPlane::Rate { .. }
        ) {
            Err(Error::CollisionBackend)
        } else {
            Ok(None)
        }
    }

    /// Supporting branches require explicit geometry provenance; legacy callbacks
    /// cannot infer a rotating normal owner from a world normal alone.
    fn rigid_pair_supports(
        &self,
        _i: usize,
        _first: &crate::contact::ContactBody,
        _j: usize,
        _second: &crate::contact::ContactBody,
        _witness: ContactWitness,
        _normal: [f64; 3],
        _feature: u64,
        _budget: usize,
    ) -> Result<Vec<crate::contact::NormalSupport>, Error> {
        Err(Error::CollisionBackend)
    }
    fn rigid_environment_supports(
        &self,
        _index: usize,
        _body: &crate::contact::ContactBody,
        _witness: ContactWitness,
        _normal: [f64; 3],
        _feature: u64,
        _budget: usize,
    ) -> Result<Vec<crate::contact::NormalSupport>, Error> {
        Err(Error::CollisionBackend)
    }
    /// All admitted supporting patches for this pair at the supplied snapshots.
    /// This is a geometry query, independent of velocity and future event time.
    /// Implementors must distinguish touching, separated and overlapping shapes.
    /// The default rejects: an event-only backend cannot prove an empty network.
    fn rigid_pair_support_contacts(
        &self,
        _i: usize,
        _first: &crate::contact::ContactBody,
        _j: usize,
        _second: &crate::contact::ContactBody,
        _budget: usize,
    ) -> Result<Vec<RigidSupportPoint>, Error> {
        Err(Error::CollisionBackend)
    }
    /// All admitted patches against fixed geometry at this snapshot. Fixed walls
    /// have no finite body index. Plane provenance must be resolved by geometry.
    fn rigid_environment_support_contacts(
        &self,
        _index: usize,
        _body: &crate::contact::ContactBody,
        _budget: usize,
    ) -> Result<Vec<RigidSupportPoint>, Error> {
        Err(Error::CollisionBackend)
    }
    fn rigid_pair_support_contacts_with_error(
        &self,
        i: usize,
        first: &crate::contact::ContactBody,
        j: usize,
        second: &crate::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<RigidSupportPoint>, Error> {
        if error_m == 0. {
            self.rigid_pair_support_contacts(i, first, j, second, budget)
        } else {
            Err(Error::CollisionBackend)
        }
    }
    /// Snapshot support of a translation-only particle against a finite body.
    /// The particle geometry must match the one used by its swept event query.
    fn particle_body_support_contacts_with_error(
        &self,
        _particle: &crate::contact::ContactBody,
        _radius: f64,
        _index: usize,
        _body: &crate::contact::ContactBody,
        _budget: usize,
        _error_m: f64,
    ) -> Result<Vec<RigidSupportPoint>, Error> {
        Err(Error::CollisionBackend)
    }

    fn rigid_environment_support_contacts_with_error(
        &self,
        i: usize,
        body: &crate::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<RigidSupportPoint>, Error> {
        if error_m == 0. {
            self.rigid_environment_support_contacts(i, body, budget)
        } else {
            Err(Error::CollisionBackend)
        }
    }
    /// Admit active supporting branches over the whole prepared interval, and
    /// search every other shape for collisions. Returning Clear cannot mean
    /// that all geometry was skipped. Unproved support evolution must reject.
    fn sweep_supported_rigid_pair_event(
        &self,
        _i: usize,
        _first: &crate::rigid_motion::RigidMotion,
        _j: usize,
        _second: &crate::rigid_motion::RigidMotion,
        _supports: &[RigidSupportPoint],
        _budget: usize,
    ) -> Result<SupportedGeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_supported_rigid_environment_event(
        &self,
        _i: usize,
        _body: &crate::rigid_motion::RigidMotion,
        _supports: &[RigidSupportPoint],
        _budget: usize,
    ) -> Result<SupportedGeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    /// Admit the exact common-point polynomial used by reaction mechanics.
    /// Legacy admission is valid only when every model is None.
    fn sweep_supported_rigid_pair_point_motion(
        &self,
        i: usize,
        first: &crate::rigid_motion::RigidMotion,
        j: usize,
        second: &crate::rigid_motion::RigidMotion,
        supports: &[RigidSupportPoint],
        motion: &[Option<RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<SupportedGeometryHit, Error> {
        if supports.len() != motion.len() {
            return Err(Error::InvalidCollision);
        }
        if motion.iter().any(Option::is_some) {
            return Err(Error::CollisionBackend);
        }
        self.sweep_supported_rigid_pair_event(i, first, j, second, supports, budget)
    }
    /// Second derivative of the edge normal for a particle/body support.
    fn particle_body_support_normal_acceleration(
        &self,
        _particle: &crate::contact::ContactBody,
        _radius: f64,
        _index: usize,
        _body: &crate::contact::ContactBody,
        geometry: RigidSupportPoint,
        _loads: [crate::contact::ContactWrench; 2],
    ) -> Result<Option<[f64; 3]>, Error> {
        match geometry.support.plane {
            crate::contact::SupportPlane::Rate { .. } => Err(Error::CollisionBackend),
            _ => Ok(None),
        }
    }

    /// Motion of the shared clipped point, with loads ordered particle then body.
    #[allow(clippy::too_many_arguments)]
    fn particle_body_support_point_motion(
        &self,
        _particle: &crate::contact::ContactBody,
        _radius: f64,
        _index: usize,
        _body: &crate::contact::ContactBody,
        _geometry: RigidSupportPoint,
        _loads: [crate::contact::ContactWrench; 2],
        _rates: [crate::contact::ContactWrench; 2],
    ) -> Result<Option<RigidSupportPointMotion>, Error> {
        Ok(None)
    }

    /// Admit particle-to-body support over the complete prepared interval.
    /// Other shape pairs on the same body still require collision queries.
    #[allow(clippy::too_many_arguments)]
    fn sweep_supported_particle_body_point_motion(
        &self,
        _particle: &crate::rigid_motion::RigidMotion,
        _radius: f64,
        _index: usize,
        _body: &crate::rigid_motion::RigidMotion,
        _supports: &[RigidSupportPoint],
        _motion: &[Option<RigidSupportPointMotion>],
        _budget: usize,
    ) -> Result<SupportedGeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_supported_rigid_environment_point_motion(
        &self,
        i: usize,
        body: &crate::rigid_motion::RigidMotion,
        supports: &[RigidSupportPoint],
        motion: &[Option<RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<SupportedGeometryHit, Error> {
        if supports.len() != motion.len() {
            return Err(Error::InvalidCollision);
        }
        if motion.iter().any(Option::is_some) {
            return Err(Error::CollisionBackend);
        }
        self.sweep_supported_rigid_environment_event(i, body, supports, budget)
    }
    /// Contact-time geometry-owned normal patch for an inelastic rigid pair.
    /// Defaults preserve point-only backends; geometry can return up to 128 points.
    fn rigid_pair_patch(
        &self,
        _first_index: usize,
        _first: &crate::contact::ContactBody,
        _second_index: usize,
        _second: &crate::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        _max_candidates: usize,
    ) -> Result<Vec<crate::contact::NormalContact>, Error> {
        Ok(vec![crate::contact::NormalContact {
            point: witness.point,
            normal,
        }])
    }
    /// Contact-time geometry-owned patch against a fixed environment.
    fn rigid_environment_patch(
        &self,
        _index: usize,
        _body: &crate::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        _max_candidates: usize,
    ) -> Result<Vec<crate::contact::NormalContact>, Error> {
        Ok(vec![crate::contact::NormalContact {
            point: witness.point,
            normal,
        }])
    }
    fn has_environment(&self) -> bool {
        true
    }
}

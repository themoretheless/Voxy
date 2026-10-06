//! Shared earliest-event solver for fluid, translating/rotating bodies and fixed walls.
use super::dynamic_world::ContactLedger;
use super::{
    DynamicEnvironmentReport, DynamicWorldConfig, DynamicWorldReport, Error, GeometryHit, Liquid,
    Particle, TranslatingBody, finite, positive,
};

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
        if particle.acceleration() != [0.; 3] || particle.initial().spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_environment_contact(p, radius, particle.duration(), budget)
            .map(Into::into)
    }
    fn sweep_particle_rigid_event(
        &self,
        p: &Particle,
        radius: f64,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
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
        self.sweep_rigid_pair_contact(i, first, j, second, budget)
            .map(Into::into)
    }
    fn sweep_rigid_environment_event(
        &self,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        self.sweep_rigid_environment_contact(index, path, budget)
            .map(Into::into)
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
#[derive(Clone, Copy)]
struct Node {
    spin: Option<crate::astrophysics_spin::Spin>,
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
}
impl Node {
    fn contact(self) -> crate::contact::ContactBody {
        crate::contact::ContactBody {
            motion: crate::gravity::Body {
                mass: self.mass,
                position: self.position,
                velocity: self.velocity,
            },
            spin: self.spin,
        }
    }
    fn update(&mut self, body: crate::contact::ContactBody) {
        self.position = body.motion.position;
        self.velocity = body.motion.velocity;
        self.spin = body.spin;
    }
}

/// Body work accompanies the existing environment report without changing its
/// legacy DTO. Includes configured gravity and additional world COM wrenches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidWorldReport {
    pub world: DynamicEnvironmentReport,
    pub external_work: f64,
    /// Nominal rotation integration and floating work discrepancy, not heat.
    pub integration_energy_residual: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedWorldReport {
    pub rigid: RigidWorldReport,
    /// Signed work of reactions on the admitted nominal trajectories, not heat.
    pub reaction_work: f64,
    pub environment_reaction_impulse: [f64; 3],
    pub supported_intervals: usize,
    pub support_points: usize,
    pub max_support_error_m: f64,
}
#[derive(Default)]
struct RigidWork {
    external: f64,
    residual: f64,
    reaction: f64,
    environment_reaction_impulse: [f64; 3],
    supported_intervals: usize,
    support_points: usize,
    max_support_error_m: f64,
}

type CollisionEvent = (
    usize,
    Option<usize>,
    f64,
    [f64; 3],
    Option<ContactWitness>,
    Option<u64>,
);

/// Resolve an exactly simultaneous, frictionless inelastic rigid event group.
/// Tokens and witnesses retain each emitting backend query's shape provenance.
fn resolve_rigid_events(
    nodes: &mut [Node],
    particle_count: usize,
    events: &[CollisionEvent],
    world: &impl LiquidBodyWorld,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
) -> Result<(), Error> {
    let total = ledger
        .contacts
        .checked_add(events.len())
        .ok_or(Error::CollisionBudget)?;
    if total > config.max_contacts {
        return Err(Error::CollisionBudget);
    }
    let mut owners = Vec::new();
    for &(first, second, _, _, _, _) in events {
        for index in std::iter::once(first).chain(second) {
            if index < particle_count {
                return Err(Error::InvalidCollision);
            }
            if !owners.contains(&index) {
                if owners.len() >= 128 {
                    return Err(Error::CollisionBudget);
                }
                owners.push(index);
            }
        }
    }
    let mut states: Vec<_> = owners.iter().map(|index| nodes[*index].contact()).collect();
    let mut contacts = Vec::new();
    let mut scale = 1_f64;
    for &(first, second, _, normal, witness, feature) in events {
        let a = nodes[first].contact();
        let b = second.map(|j| nodes[j].contact());
        let patch = if a.spin.is_some() || b.is_some_and(|body| body.spin.is_some()) {
            let witness = witness.ok_or(Error::InvalidCollision)?;
            charge(ledger, config)?;
            if let Some(index) = second {
                world.rigid_pair_patch_for_feature(
                    first - particle_count,
                    &a,
                    index - particle_count,
                    b.as_ref().ok_or(Error::InvalidCollision)?,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            } else {
                world.rigid_environment_patch_for_feature(
                    first - particle_count,
                    &a,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            }
        } else {
            vec![crate::contact::NormalContact {
                point: witness.map_or(a.motion.position, |w| w.point),
                normal,
            }]
        };
        if patch.is_empty() {
            return Err(Error::InvalidCollision);
        }
        if contacts.len() + patch.len() > 128 {
            return Err(Error::CollisionBudget);
        }
        let first_local = owners
            .iter()
            .position(|j| *j == first)
            .ok_or(Error::InvalidCollision)?;
        let second_local = second
            .map(|index| {
                owners
                    .iter()
                    .position(|j| *j == index)
                    .ok_or(Error::InvalidCollision)
            })
            .transpose()?;
        for contact in patch {
            for velocity in [
                a.point_velocity(contact.point),
                b.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point)),
            ] {
                let velocity = velocity.map_err(|_| Error::InvalidCollision)?;
                scale = velocity.iter().map(|v| v.abs()).fold(scale, f64::max);
            }
            contacts.push(crate::contact::NetworkContact {
                first: first_local,
                second: second_local,
                contact,
            });
        }
    }
    let report = crate::contact::resolve_normal_contact_network(
        &mut states,
        &contacts,
        crate::contact::ManifoldConfig {
            max_sweeps: 10000,
            velocity_tolerance: 128. * f64::EPSILON * scale,
        },
    )
    .map_err(|e| match e {
        crate::contact::Error::Budget => Error::CollisionBudget,
        crate::contact::Error::InvalidInput => Error::InvalidCollision,
        crate::contact::Error::NumericalFailure => Error::NumericalFailure,
    })?;
    ledger.contacts = total;
    ledger.loss += (-report.kinetic_energy_change).max(0.);
    for (contact, impulse) in contacts.iter().zip(report.impulses) {
        if contact.second.is_none() {
            for k in 0..3 {
                ledger.environment_impulse[k] -= impulse[k];
            }
        }
    }
    if !ledger.loss.is_finite() || !finite(ledger.environment_impulse) {
        return Err(Error::NumericalFailure);
    }
    for (index, state) in owners.into_iter().zip(states) {
        nodes[index].update(state);
    }
    Ok(())
}

fn default_rotation() -> crate::spin_path::Config {
    crate::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 1024,
        max_trials: 4096,
    }
}
pub(super) fn solve(
    particles: &mut [Particle],
    bodies: &mut [TranslatingBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
) -> Result<(), Error> {
    let mut rigid: Vec<_> = bodies
        .iter()
        .map(|b| crate::contact::ContactBody {
            motion: crate::gravity::Body {
                position: b.position,
                velocity: b.velocity,
                mass: b.mass,
            },
            spin: None,
        })
        .collect();
    let forces = vec![crate::contact::ContactWrench::default(); rigid.len()];
    let mut work = RigidWork::default();
    solve_contact(
        particles,
        &mut rigid,
        radius,
        dt,
        config,
        ledger,
        world,
        default_rotation(),
        &forces,
        [0.; 3],
        &mut work,
        None,
    )?;
    for (body, state) in bodies.iter_mut().zip(rigid) {
        body.position = state.motion.position;
        body.velocity = state.motion.velocity;
    }
    Ok(())
}

fn solve_contact(
    particles: &mut [Particle],
    bodies: &mut [crate::contact::ContactBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
    rotation: crate::spin_path::Config,
    forces: &[crate::contact::ContactWrench],
    particle_acceleration: [f64; 3],
    work: &mut RigidWork,
    supported: Option<super::SupportedWorldConfig>,
) -> Result<(), Error> {
    if forces.len() != bodies.len() {
        return Err(Error::InvalidCollision);
    }
    let count = particles.len();
    let mut nodes: Vec<_> = particles
        .iter()
        .map(|p| Node {
            spin: None,
            position: p.position,
            velocity: p.velocity,
            mass: p.mass,
        })
        .chain(bodies.iter().map(|b| Node {
            position: b.motion.position,
            velocity: b.motion.velocity,
            mass: b.motion.mass,
            spin: b.spin,
        }))
        .collect();
    let mut remaining = dt;
    let mut horizon = supported.map_or(dt, |c| dt.min(c.max_interval_s));
    let require_witness = bodies.iter().any(|body| body.spin.is_some());
    'intervals: while remaining > 0. {
        let states: Vec<_> = nodes[count..].iter().map(|n| n.contact()).collect();
        let support_report = if let Some(c) = supported {
            if states.is_empty() {
                None
            } else {
                if ledger.queries >= config.max_queries {
                    return Err(Error::CollisionBudget);
                }
                let limits = DynamicWorldConfig {
                    max_queries: config.max_queries - ledger.queries,
                    ..config
                };
                let report = super::support_world::assemble_reactions(
                    &states,
                    world,
                    forces,
                    limits,
                    c.reaction,
                    true,
                    c.max_geometry_error_m,
                )?;
                ledger.queries += report.queries;
                Some(report)
            }
        } else {
            None
        };
        let active = support_report
            .as_ref()
            .is_some_and(|r| !r.supports.is_empty());
        if !active {
            horizon = remaining;
        }
        if active && work.supported_intervals >= supported.unwrap().max_intervals {
            return Err(Error::CollisionBudget);
        }
        let mut effective = forces.to_vec();
        if let Some(reaction) = support_report.as_ref().and_then(|r| r.reaction.as_ref()) {
            for (w, r) in effective.iter_mut().zip(&reaction.wrenches) {
                for k in 0..3 {
                    w.force[k] += r.force[k];
                    w.torque[k] += r.torque[k];
                }
            }
        }
        let pair_supports = |i: usize, j: Option<usize>| -> Vec<RigidSupportPoint> {
            let Some(report) = &support_report else {
                return Vec::new();
            };
            let Some(reaction) = &report.reaction else {
                return Vec::new();
            };
            report
                .supports
                .iter()
                .zip(&report.geometry)
                .zip(&reaction.forces)
                .filter(|((s, _), _)| s.first == i && s.second == j)
                .map(|((_, geometry), force)| {
                    let mut point = *geometry;
                    point.carrying_reaction = *force != [0.; 3];
                    point
                })
                .collect()
        };
        let mut support_error: f64 = 0.;
        let paths: Vec<_> = nodes
            .iter()
            .enumerate()
            .map(|(index, n)| {
                let wrench = if index < count {
                    crate::contact::ContactWrench {
                        force: particle_acceleration.map(|a| a * n.mass),
                        torque: [0.; 3],
                    }
                } else {
                    effective[index - count]
                };
                n.contact()
                    .prepare_motion(wrench.force, wrench.torque, horizon, rotation)
                    .map_err(|_| Error::NumericalFailure)
            })
            .collect::<Result<_, _>>()?;
        let interval_duration = horizon;
        let mut earliest: Option<CollisionEvent> = None;
        let mut simultaneous = Vec::new();
        let mut admit = |first: usize,
                         second: Option<usize>,
                         event: RigidGeometryHit|
         -> Result<(), Error> {
            let hit = event.contact;
            if event.feature.is_some() && !matches!(hit.geometry, GeometryHit::Contact { .. }) {
                return Err(Error::InvalidCollision);
            }
            if hit.witness.is_some() && !matches!(hit.geometry, GeometryHit::Contact { .. }) {
                return Err(Error::InvalidCollision);
            }
            let GeometryHit::Contact { fraction, normal } = hit.geometry else {
                return if matches!(hit.geometry, GeometryHit::Overlap) {
                    Err(Error::InitialOverlap)
                } else {
                    Ok(())
                };
            };
            if hit.witness.is_some_and(|w| {
                !finite(w.point) || !w.tolerance_m.is_finite() || w.tolerance_m < 0.
            }) {
                return Err(Error::InvalidCollision);
            }
            if require_witness && hit.witness.is_none() {
                return Err(Error::InvalidCollision);
            }
            let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
            if !fraction.is_finite()
                || !(0. ..=1.).contains(&fraction)
                || !norm.is_finite()
                || (norm - 1.).abs() > 5e-11
            {
                return Err(Error::InvalidCollision);
            }
            let normal = normal.map(|n| n / norm);
            let time = interval_duration * fraction;
            let point = hit.witness.map_or(nodes[first].position, |w| w.point);
            let a = paths[first]
                .sample(time)
                .map_err(|_| Error::NumericalFailure)?;
            let va = a
                .point_velocity(point)
                .map_err(|_| Error::NumericalFailure)?;
            let vb = if let Some(index) = second {
                paths[index]
                    .sample(time)
                    .map_err(|_| Error::NumericalFailure)?
                    .point_velocity(point)
                    .map_err(|_| Error::NumericalFailure)?
            } else {
                [0.; 3]
            };
            let relative: [f64; 3] = std::array::from_fn(|k| va[k] - vb[k]);
            let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
            if !speed.is_finite() || speed >= 0. {
                return Err(Error::InvalidCollision);
            }
            if earliest.is_none_or(|(_, _, old, _, _, _)| fraction < old) {
                earliest = Some((first, second, fraction, normal, hit.witness, event.feature));
                simultaneous.clear();
            }
            if earliest.is_some_and(|(_, _, old, _, _, _)| fraction == old) {
                simultaneous.push((first, second, fraction, normal, hit.witness, event.feature));
            }
            Ok(())
        };
        for (index, original) in particles.iter().enumerate() {
            let p = Particle {
                position: nodes[index].position,
                velocity: nodes[index].velocity,
                ..*original
            };
            for body_index in 0..bodies.len() {
                charge(ledger, config)?;
                admit(
                    index,
                    Some(count + body_index),
                    world.sweep_particle_motion_body_event(
                        &p,
                        radius,
                        &paths[index],
                        body_index,
                        &paths[count + body_index],
                        config.contact.max_candidates,
                    )?,
                )?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    index,
                    None,
                    world.sweep_particle_motion_environment_event(
                        &p,
                        radius,
                        &paths[index],
                        config.contact.max_candidates,
                    )?,
                )?;
            }
        }
        for first in 0..bodies.len() {
            for second in first + 1..bodies.len() {
                charge(ledger, config)?;
                admit(count + first, Some(count + second), {
                    let supports = pair_supports(first, Some(second));
                    if supports.is_empty() {
                        world.sweep_rigid_pair_event(
                            first,
                            &paths[count + first],
                            second,
                            &paths[count + second],
                            config.contact.max_candidates,
                        )?
                    } else {
                        match world.sweep_supported_rigid_pair_event(
                            first,
                            &paths[count + first],
                            second,
                            &paths[count + second],
                            &supports,
                            config.contact.max_candidates,
                        ) {
                            Ok(hit) => {
                                if !hit.support_error_m.is_finite() || hit.support_error_m < 0. {
                                    return Err(Error::InvalidCollision);
                                }
                                support_error = support_error.max(hit.support_error_m);
                                hit.event
                            }
                            Err(Error::CollisionBudget)
                                if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                            {
                                horizon *= 0.5;
                                continue 'intervals;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                })?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(count + first, None, {
                    let supports = pair_supports(first, None);
                    if supports.is_empty() {
                        world.sweep_rigid_environment_event(
                            first,
                            &paths[count + first],
                            config.contact.max_candidates,
                        )?
                    } else {
                        match world.sweep_supported_rigid_environment_event(
                            first,
                            &paths[count + first],
                            &supports,
                            config.contact.max_candidates,
                        ) {
                            Ok(hit) => {
                                if !hit.support_error_m.is_finite() || hit.support_error_m < 0. {
                                    return Err(Error::InvalidCollision);
                                }
                                support_error = support_error.max(hit.support_error_m);
                                hit.event
                            }
                            Err(Error::CollisionBudget)
                                if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                            {
                                horizon *= 0.5;
                                continue 'intervals;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                })?;
            }
        }
        let fraction = earliest.map_or(1., |(_, _, f, _, _, _)| f);
        for (index, n) in nodes.iter_mut().enumerate() {
            let time = interval_duration * fraction;
            if index >= count {
                let change = paths[index]
                    .work(time)
                    .map_err(|_| Error::NumericalFailure)?;
                let external = forces[index - count];
                let (force, torque) = paths[index]
                    .wrench_work(time, external.force, external.torque)
                    .map_err(|_| Error::NumericalFailure)?;
                let reaction = support_report
                    .as_ref()
                    .and_then(|r| r.reaction.as_ref())
                    .map_or(crate::contact::ContactWrench::default(), |r| {
                        r.wrenches[index - count]
                    });
                let (rf, rt) = paths[index]
                    .wrench_work(time, reaction.force, reaction.torque)
                    .map_err(|_| Error::NumericalFailure)?;
                work.external += force + torque;
                work.reaction += rf + rt;
                work.residual += change.kinetic_energy_change - force - torque - rf - rt;
            }
            if index >= count || particle_acceleration != [0.; 3] {
                n.update(
                    paths[index]
                        .sample(time)
                        .map_err(|_| Error::NumericalFailure)?,
                );
            } else {
                for k in 0..3 {
                    n.position[k] += n.velocity[k] * time;
                }
            }
            if !finite(n.position) || !work.external.is_finite() || !work.residual.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        let time = interval_duration * fraction;
        if active && time > 0. {
            let report = support_report.as_ref().unwrap();
            work.supported_intervals += 1;
            work.support_points = work
                .support_points
                .checked_add(report.supports.len())
                .ok_or(Error::CollisionBudget)?;
            if work.support_points + ledger.contacts > config.max_contacts {
                return Err(Error::CollisionBudget);
            }
            work.max_support_error_m = work.max_support_error_m.max(support_error);
            for k in 0..3 {
                let impulse = report.environment_force[k] * time;
                work.environment_reaction_impulse[k] += impulse;
                ledger.environment_impulse[k] += impulse;
            }
            if !work.reaction.is_finite() || !finite(work.environment_reaction_impulse) {
                return Err(Error::NumericalFailure);
            }
        }
        let next = if supported.is_none() {
            remaining * (1. - fraction)
        } else {
            remaining - time
        };
        if time > 0. && next == remaining {
            return Err(Error::CollisionBudget);
        }
        remaining = next.max(0.);
        horizon = supported.map_or(remaining, |c| remaining.min(c.max_interval_s));
        let Some((first, second, _, normal, witness, feature)) = earliest else {
            continue;
        };
        if simultaneous.len() > 1
            && config.contact.restitution == 0.
            && config.contact.friction == 0.
            && simultaneous
                .iter()
                .all(|(first, second, ..)| *first >= count && second.is_none_or(|j| j >= count))
        {
            if ledger.contacts + work.support_points + simultaneous.len() > config.max_contacts {
                return Err(Error::CollisionBudget);
            }
            resolve_rigid_events(&mut nodes, count, &simultaneous, world, config, ledger)?;
            continue;
        }
        if ledger.contacts + work.support_points >= config.max_contacts {
            return Err(Error::CollisionBudget);
        }
        ledger.contacts += 1;
        let relative: [f64; 3] = std::array::from_fn(|a| {
            nodes[first].velocity[a] - second.map_or(0., |s| nodes[s].velocity[a])
        });
        let reduced = second.map_or(nodes[first].mass, |s| {
            1. / (1. / nodes[first].mass + 1. / nodes[s].mass)
        });
        if !positive(reduced) {
            return Err(Error::NumericalFailure);
        }
        let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
        let tangent: [f64; 3] = std::array::from_fn(|a| relative[a] - speed * normal[a]);
        let first_contact = nodes[first].contact();
        let second_contact = second.map(|s| nodes[s].contact());
        let point = witness.map_or(nodes[first].position, |w| w.point);
        let mut a = first_contact;
        let mut b = second_contact;
        let angular_patch = first >= count
            && config.contact.restitution == 0.
            && (a.spin.is_some() || b.is_some_and(|body| body.spin.is_some()));
        let (impulse, loss) = if angular_patch {
            let witness = witness.ok_or(Error::InvalidCollision)?;
            charge(ledger, config)?;
            let contacts = if let Some(index) = second {
                world.rigid_pair_patch_for_feature(
                    first - count,
                    &a,
                    index - count,
                    b.as_ref().ok_or(Error::InvalidCollision)?,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            } else {
                world.rigid_environment_patch_for_feature(
                    first - count,
                    &a,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            };
            if contacts.len() > 128 {
                return Err(Error::CollisionBudget);
            }
            let mut scale = relative.iter().map(|v| v.abs()).fold(1., f64::max);
            for contact in &contacts {
                for velocity in [
                    a.point_velocity(contact.point),
                    b.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point)),
                ] {
                    let velocity = velocity.map_err(|_| Error::InvalidCollision)?;
                    scale = velocity.iter().map(|v| v.abs()).fold(scale, f64::max);
                }
            }
            let report = crate::contact::resolve_normal_manifold(
                &mut a,
                b.as_mut(),
                &contacts,
                crate::contact::ManifoldConfig {
                    max_sweeps: 10000,
                    velocity_tolerance: 128. * f64::EPSILON * scale,
                },
            )
            .map_err(|e| match e {
                crate::contact::Error::Budget => Error::CollisionBudget,
                crate::contact::Error::InvalidInput => Error::InvalidCollision,
                crate::contact::Error::NumericalFailure => Error::NumericalFailure,
            })?;
            let total = std::array::from_fn(|k| report.impulses.iter().map(|j| j[k]).sum());
            (total, (-report.kinetic_energy_change).max(0.))
        } else {
            let response = crate::contact::normal_impulse(
                &a,
                b.as_ref(),
                point,
                normal,
                config.contact.restitution,
            )
            .map_err(|_| Error::NumericalFailure)?;
            let loss = response.dissipated_energy
                + if config.contact.friction == 0. {
                    0.
                } else {
                    0.5 * reduced
                        * tangent.iter().map(|v| v * v).sum::<f64>()
                        * config.contact.friction
                        * (2. - config.contact.friction)
                };
            let impulse: [f64; 3] = std::array::from_fn(|k| {
                response.impulse[k] - reduced * config.contact.friction * tangent[k]
            });
            a.apply_point_impulse(point, impulse)
                .map_err(|_| Error::NumericalFailure)?;
            if let Some(body) = &mut b {
                body.apply_point_impulse(point, impulse.map(|j| -j))
                    .map_err(|_| Error::NumericalFailure)?;
            }
            (impulse, loss)
        };
        ledger.loss += loss;
        if let Some(total) = ledger.particle_loss.get_mut(first) {
            *total += loss;
        }
        let reference = second.map_or(nodes[first].position, |s| nodes[s].position);
        let epsilon = 64.
            * f64::EPSILON
            * (0..3)
                .map(|k| normal[k].abs() * nodes[first].position[k].abs().max(reference[k].abs()))
                .fold(1., f64::max);
        nodes[first].update(a);
        if let Some(index) = second {
            nodes[index].update(b.ok_or(Error::InvalidCollision)?);
        } else {
            for k in 0..3 {
                ledger.environment_impulse[k] -= impulse[k];
            }
        }
        for k in 0..3 {
            nodes[first].position[k] += normal[k] * epsilon;
        }
        if !nodes
            .iter()
            .all(|n| finite(n.velocity) && finite(n.position))
            || !finite(ledger.environment_impulse)
            || !ledger.loss.is_finite()
        {
            return Err(Error::NumericalFailure);
        }
    }
    for (p, n) in particles.iter_mut().zip(&nodes) {
        p.position = n.position;
        p.velocity = n.velocity;
    }
    for (b, n) in bodies.iter_mut().zip(&nodes[count..]) {
        *b = n.contact();
    }
    Ok(())
}
fn charge(ledger: &mut ContactLedger, config: DynamicWorldConfig) -> Result<(), Error> {
    if ledger.queries >= config.max_queries {
        return Err(Error::CollisionBudget);
    }
    ledger.queries += 1;
    Ok(())
}

impl Liquid {
    /// Coupled finite-body/body, fluid/body and fixed-world events in one transaction.
    /// Rotation is constrained; backend templates may be affine or compound geometry.
    /// # Errors
    /// Invalid bodies/budgets/backend contacts or numerical failure restore every owner.
    pub fn step_with_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [TranslatingBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
    ) -> Result<DynamicEnvironmentReport, Error> {
        if bodies.len() > max_bodies || max_bodies == 0 {
            return Err(Error::InvalidCollision);
        }
        let mut rigid: Vec<_> = bodies
            .iter()
            .map(|b| crate::contact::ContactBody {
                motion: crate::gravity::Body {
                    position: b.position,
                    velocity: b.velocity,
                    mass: b.mass,
                },
                spin: None,
            })
            .collect();
        let report = self.step_with_rigid_body_world(
            dt,
            &mut rigid,
            world,
            config,
            max_bodies,
            default_rotation(),
        )?;
        for (body, state) in bodies.iter_mut().zip(rigid) {
            body.position = state.motion.position;
            body.velocity = state.motion.velocity;
        }
        Ok(report)
    }
    /// Same event loop with intrinsic spin. Geometry must admit angular paths and
    /// world witnesses. Legacy tangential damping is unsupported for rigid spin.
    pub fn step_with_rigid_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
    ) -> Result<DynamicEnvironmentReport, Error> {
        let forces = vec![crate::contact::ContactWrench::default(); bodies.len()];
        self.step_with_rigid_body_forces(dt, bodies, world, config, max_bodies, rotation, &forces)
            .map(|report| report.world)
    }

    /// Same transactional event loop with additional constant world COM forces
    /// and torques for each body. Configured body gravity is applied continuously
    /// once, in addition to these wrenches. Accelerated geometry must be supported
    /// by the backend; linear-only defaults reject rather than sweep a chord.
    pub fn step_with_rigid_body_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
    ) -> Result<RigidWorldReport, Error> {
        self.step_rigid_forces(
            dt, bodies, world, config, max_bodies, rotation, additional, None,
        )
        .map(|report| report.rigid)
    }

    /// Shared event loop with geometry-admitted constant-reaction intervals.
    /// Unsupported evolving branches reject transactionally; velocities and Spin
    /// are sampled from their real net-wrench paths, never clamped to rest.
    pub fn step_with_supported_rigid_body_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        supported: super::SupportedWorldConfig,
    ) -> Result<SupportedWorldReport, Error> {
        supported.validate()?;
        self.step_rigid_forces(
            dt,
            bodies,
            world,
            config,
            max_bodies,
            rotation,
            additional,
            Some(supported),
        )
    }

    fn step_rigid_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        supported: Option<super::SupportedWorldConfig>,
    ) -> Result<SupportedWorldReport, Error> {
        config.contact.validate()?;
        if additional.len() != bodies.len()
            || additional.iter().zip(bodies.iter()).any(|(w, b)| {
                !finite(w.force) || !finite(w.torque) || (b.spin.is_none() && w.torque != [0.; 3])
            })
            || bodies.len() > max_bodies
            || max_bodies == 0
            || config.max_contacts == 0
            || config.max_queries == 0
            || bodies
                .iter()
                .any(|b| b.energy().is_err() || (b.spin.is_some() && config.contact.friction != 0.))
        {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = self.clone();
        let mut bodies_candidate = bodies.to_vec();
        let mut ledger = ContactLedger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let forces = super::support_world::world_wrenches(bodies, additional, gravity)?;
        let mut work = RigidWork::default();
        let empty = self.particles.is_empty();
        let fluid = candidate.advance(dt, None, |particles, time| {
            // SPH has already kicked pressure, viscosity and gravity. Retain
            // the first two operators, but put uniform gravity on the same
            // continuous drift paths as the bodies to avoid artificial relative
            // acceleration at a resting fluid/body contact.
            for particle in particles.iter_mut() {
                for k in 0..3 {
                    particle.velocity[k] -= gravity[k] * time;
                }
            }
            solve_contact(
                particles,
                &mut bodies_candidate,
                radius,
                time,
                config,
                &mut ledger,
                world,
                rotation,
                &forces,
                gravity,
                &mut work,
                supported,
            )
        })?;
        if empty {
            solve_contact(
                &mut [],
                &mut bodies_candidate,
                radius,
                dt,
                config,
                &mut ledger,
                world,
                rotation,
                &forces,
                gravity,
                &mut work,
                supported,
            )?;
        }
        *self = candidate;
        bodies.copy_from_slice(&bodies_candidate);
        Ok(SupportedWorldReport {
            reaction_work: work.reaction,
            environment_reaction_impulse: work.environment_reaction_impulse,
            supported_intervals: work.supported_intervals,
            support_points: work.support_points,
            max_support_error_m: work.max_support_error_m,
            rigid: RigidWorldReport {
                external_work: work.external,
                integration_energy_residual: work.residual,
                world: DynamicEnvironmentReport {
                    dynamics: DynamicWorldReport {
                        fluid,
                        contacts: ledger.contacts,
                        queries: ledger.queries,
                        dissipated_energy: ledger.loss,
                    },
                    environment_impulse: ledger.environment_impulse,
                },
            },
        })
    }
}

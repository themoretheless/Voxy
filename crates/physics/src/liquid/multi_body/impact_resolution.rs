//! Single-event rigid/particle manifold and point impact resolution.

use super::{ContactLedger, ContactWitness, DynamicWorldConfig, Error, LiquidBodyWorld, charge};

pub(super) struct ImpactContext {
    pub(super) first: usize,
    pub(super) second: Option<usize>,
    pub(super) particle_count: usize,
    pub(super) supported: bool,
    pub(super) radius: f64,
    pub(super) event_time_s: f64,
    pub(super) normal: [f64; 3],
    pub(super) tangent: [f64; 3],
    pub(super) relative: [f64; 3],
    pub(super) reduced_mass: f64,
    pub(super) point: [f64; 3],
    pub(super) boundary_velocity: [f64; 3],
    pub(super) witness: Option<ContactWitness>,
    pub(super) feature: Option<u64>,
    pub(super) config: DynamicWorldConfig,
}

pub(super) fn resolve_single_impact(
    ctx: &ImpactContext,
    a: &mut crate::contact::ContactBody,
    b: &mut Option<crate::contact::ContactBody>,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
) -> Result<([f64; 3], f64, f64), Error> {
    let mixed_patch = ctx.first < ctx.particle_count
        && ctx.supported
        && ctx.config.contact.friction == 0.
        && world.supports_particle_body_reactions()
        && b.is_some_and(|body| body.spin.is_some());
    let angular_patch = (ctx.first >= ctx.particle_count || mixed_patch)
        && ctx.config.contact.restitution == 0.
        && (a.spin.is_some() || b.is_some_and(|body| body.spin.is_some()));
    if angular_patch {
        resolve_manifold_impact(ctx, mixed_patch, a, b, ledger, world)
    } else {
        resolve_point_impact(ctx, a, b)
    }
}

fn resolve_manifold_impact(
    ctx: &ImpactContext,
    mixed_patch: bool,
    a: &mut crate::contact::ContactBody,
    b: &mut Option<crate::contact::ContactBody>,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
) -> Result<([f64; 3], f64, f64), Error> {
    let witness = ctx.witness.ok_or(Error::InvalidCollision)?;
    charge(ledger, ctx.config)?;
    let count = ctx.particle_count;
    let max_candidates = ctx.config.contact.max_candidates;
    let contacts = if mixed_patch {
        let index = ctx.second.ok_or(Error::InvalidCollision)?;
        world.particle_body_impact_patch(
            ctx.radius,
            a,
            index - count,
            b.as_ref().ok_or(Error::InvalidCollision)?,
            witness,
            ctx.normal,
            ctx.feature,
            max_candidates,
        )?
    } else if let Some(index) = ctx.second {
        world.rigid_pair_patch_for_feature(
            ctx.first - count,
            a,
            index - count,
            b.as_ref().ok_or(Error::InvalidCollision)?,
            witness,
            ctx.normal,
            ctx.feature,
            max_candidates,
        )?
    } else {
        world.rigid_environment_patch_at(
            ctx.first - count,
            a,
            witness,
            ctx.normal,
            ctx.feature,
            ctx.event_time_s,
            max_candidates,
        )?
    };
    if contacts.len() > 128 {
        return Err(Error::CollisionBudget);
    }
    let scale = max_patch_velocity_scale(&ctx.relative, &contacts, a, b.as_ref())?;
    let manifold = crate::contact::ManifoldConfig {
        max_sweeps: 10000,
        velocity_tolerance: 128. * f64::EPSILON * scale,
    };
    let report = if ctx.boundary_velocity == [0.; 3] {
        crate::contact::resolve_normal_manifold(a, b.as_mut(), &contacts, manifold)
    } else {
        let mut states = [*a];
        let indexed: Vec<_> = contacts
            .iter()
            .map(|&contact| crate::contact::NetworkContact {
                first: 0,
                second: None,
                contact,
            })
            .collect();
        let report = crate::contact::resolve_normal_contact_network_with_boundary_velocities(
            &mut states,
            &indexed,
            &[ctx.boundary_velocity; 128][..indexed.len()],
            manifold,
        );
        *a = states[0];
        report
    }
    .map_err(|e| match e {
        crate::contact::Error::Budget => Error::CollisionBudget,
        crate::contact::Error::InvalidInput => Error::InvalidCollision,
        crate::contact::Error::NumericalFailure => Error::NumericalFailure,
    })?;
    let total = std::array::from_fn(|k| report.impulses.iter().map(|j| j[k]).sum());
    Ok((
        total,
        (report.boundary_work - report.kinetic_energy_change).max(0.),
        report.boundary_work,
    ))
}

fn max_patch_velocity_scale(
    relative: &[f64; 3],
    contacts: &[crate::contact::NormalContact],
    a: &crate::contact::ContactBody,
    b: Option<&crate::contact::ContactBody>,
) -> Result<f64, Error> {
    let mut scale = relative.iter().map(|v| v.abs()).fold(1., f64::max);
    for contact in contacts {
        for velocity in [
            a.point_velocity(contact.point),
            b.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point)),
        ] {
            let velocity = velocity.map_err(|_| Error::InvalidCollision)?;
            scale = velocity.iter().map(|v| v.abs()).fold(scale, f64::max);
        }
    }
    Ok(scale)
}

fn resolve_point_impact(
    ctx: &ImpactContext,
    a: &mut crate::contact::ContactBody,
    b: &mut Option<crate::contact::ContactBody>,
) -> Result<([f64; 3], f64, f64), Error> {
    let response = if ctx.boundary_velocity == [0.; 3] {
        crate::contact::normal_impulse(
            a,
            b.as_ref(),
            ctx.point,
            ctx.normal,
            ctx.config.contact.restitution,
        )
    } else {
        crate::contact::normal_impulse_against_moving_boundary(
            a,
            ctx.boundary_velocity,
            ctx.point,
            ctx.normal,
            ctx.config.contact.restitution,
        )
    }
    .map_err(|_| Error::NumericalFailure)?;
    let friction = ctx.config.contact.friction;
    let loss = response.dissipated_energy
        + if friction == 0. {
            0.
        } else {
            0.5 * ctx.reduced_mass
                * ctx.tangent.iter().map(|v| v * v).sum::<f64>()
                * friction
                * (2. - friction)
        };
    let impulse: [f64; 3] =
        std::array::from_fn(|k| response.impulse[k] - ctx.reduced_mass * friction * ctx.tangent[k]);
    a.apply_point_impulse(ctx.point, impulse)
        .map_err(|_| Error::NumericalFailure)?;
    if let Some(body) = b {
        body.apply_point_impulse(ctx.point, impulse.map(|j| -j))
            .map_err(|_| Error::NumericalFailure)?;
    }
    let work = (0..3)
        .map(|k| impulse[k] * ctx.boundary_velocity[k])
        .sum::<f64>();
    Ok((impulse, loss, work))
}

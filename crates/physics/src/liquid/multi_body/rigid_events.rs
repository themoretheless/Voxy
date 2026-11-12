//! Resolution of simultaneous rigid contact events.

use super::{
    ContactLedger, ContactWitness, DynamicWorldConfig, Error, LiquidBodyWorld, Node, charge, finite,
};

pub(super) type CollisionEvent = (
    usize,
    Option<usize>,
    f64,
    [f64; 3],
    Option<ContactWitness>,
    Option<u64>,
    [f64; 3],
);

/// Resolve an exactly simultaneous, frictionless inelastic rigid event group.
/// Tokens and witnesses retain each emitting backend query's shape provenance.
#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(super) fn resolve_rigid_events(
    nodes: &mut [Node],
    particle_count: usize,
    events: &[CollisionEvent],
    event_time_s: f64,
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
    for &(first, second, ..) in events {
        for index in std::iter::once(first).chain(second) {
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
    let mut boundary_velocities = Vec::new();
    let mut scale = 1_f64;
    for &(first, second, _, normal, witness, feature, velocity) in events {
        let a = nodes[first].contact();
        let b = second.map(|j| nodes[j].contact());
        let patch = if first >= particle_count
            && (a.spin.is_some() || b.is_some_and(|body| body.spin.is_some()))
        {
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
                world.rigid_environment_patch_at(
                    first - particle_count,
                    &a,
                    witness,
                    normal,
                    feature,
                    event_time_s,
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
                b.map_or(Ok(velocity), |body| body.point_velocity(contact.point)),
            ] {
                let velocity = velocity.map_err(|_| Error::InvalidCollision)?;
                scale = velocity.iter().map(|v| v.abs()).fold(scale, f64::max);
            }
            boundary_velocities.push(velocity);
            contacts.push(crate::contact::NetworkContact {
                first: first_local,
                second: second_local,
                contact,
            });
        }
    }
    let report = crate::contact::resolve_normal_contact_network_with_boundary_velocities(
        &mut states,
        &contacts,
        &boundary_velocities,
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
    ledger.loss += (report.boundary_work - report.kinetic_energy_change).max(0.);
    ledger.boundary_impact_work += report.boundary_work;
    for (contact, impulse) in contacts.iter().zip(report.impulses) {
        if contact.second.is_none() {
            for k in 0..3 {
                ledger.environment_impulse[k] -= impulse[k];
            }
        }
    }
    if !ledger.loss.is_finite()
        || !ledger.boundary_impact_work.is_finite()
        || !finite(ledger.environment_impulse)
    {
        return Err(Error::NumericalFailure);
    }
    for (index, state) in owners.into_iter().zip(states) {
        nodes[index].update(state);
    }
    Ok(())
}

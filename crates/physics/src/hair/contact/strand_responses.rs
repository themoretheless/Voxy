//! Strand contact responses: finishing gathered contacts into per-rod responses.
//!
//! Split out of `contact.rs` (plan step B4) with unchanged bodies.

use super::*;

#[derive(Clone)]
pub(in crate::hair) struct StrandResponse {
    pub(in crate::hair) a: (usize, usize, f64),
    pub(in crate::hair) b: (usize, usize, f64),
    pub(in crate::hair) normal: V,
    pub(in crate::hair) impulse: f64,
}
pub(in crate::hair) fn finish_strand_contacts(
    rods: &mut [HairRod],
    responses: &[StrandResponse],
    dt: f64,
) {
    finish_strand_contacts_with_diagnostics(rods, responses, dt, 0, None);
}
pub(in crate::hair) fn finish_strand_contacts_with_diagnostics(
    rods: &mut [HairRod],
    responses: &[StrandResponse],
    dt: f64,
    substep: usize,
    mut diagnostics: Option<&mut Vec<crate::hair::HairFrictionDiagnostic>>,
) {
    for response in responses {
        let (ra, ia, s) = response.a;
        let (rb, ib, t) = response.b;
        let wa = contact_weight(&rods[ra], ia, s);
        let wb = contact_weight(&rods[rb], ib, t);
        if wa + wb < 1e-30 {
            continue;
        }
        let va = add(
            mul(rods[ra].velocity[ia], 1. - s),
            mul(rods[ra].velocity[ia + 1], s),
        );
        let vb = add(
            mul(rods[rb].velocity[ib], 1. - t),
            mul(rods[rb].velocity[ib + 1], t),
        );
        let relative = sub(va, vb);
        let normal_speed = dot(relative, response.normal);
        let normal_impulse = (-normal_speed / (wa + wb)).max(0.);
        let tangent = sub(relative, mul(response.normal, normal_speed));
        let tangent_speed = len(tangent);
        let friction = (rods[ra].material.friction * rods[rb].material.friction).sqrt();
        let tangent_impulse =
            (friction * (response.impulse / dt + normal_impulse)).min(tangent_speed / (wa + wb));
        let impulse = sub(
            mul(response.normal, normal_impulse),
            mul(tangent, tangent_impulse / tangent_speed.max(1e-30)),
        );
        if let Some(diagnostics) = diagnostics.as_mut() {
            diagnostics.push(crate::hair::HairFrictionDiagnostic {
                substep,
                a: response.a,
                b: response.b,
                normal: response.normal,
                position_impulse: response.impulse,
                relative_velocity: relative,
                normal_speed,
                tangent_speed,
                mobility_a: wa,
                mobility_b: wb,
                friction,
                normal_impulse,
                tangent_impulse,
                applied_impulse: impulse,
            });
        }

        for (rod, segment, fraction, sign) in [(ra, ia, s, 1.), (rb, ib, t, -1.)] {
            if contact_weight(&rods[rod], segment, fraction) == 0. {
                continue;
            }
            for (point, weight) in [(segment, 1. - fraction), (segment + 1, fraction)] {
                let change = mul(impulse, sign * weight * rods[rod].inv_mass[point]);
                rods[rod].velocity[point] = add(rods[rod].velocity[point], change);
            }
        }
    }
}
// A segment can intersect several faces at numerically zero distance. Choose
// its first intersection, with a topology key for coincident shared features,
// rather than allowing roundoff in the distance to select a distant face.

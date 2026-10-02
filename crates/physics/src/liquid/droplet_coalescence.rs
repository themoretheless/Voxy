//! Prescribed force-frozen swept-sphere coalescence, separate from substrate impacts.
use super::{DropletMergeReport, Error, Liquid, norm, positive};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropletCoalescenceControl {
    pub dt: f64,
    pub surface_tension: f64,
    /// Prescribed relative normal-speed limit, not a calibrated collision outcome law.
    pub maximum_normal_speed: f64,
    pub max_events: usize,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DropletCoalescenceReport {
    pub events: Vec<DropletMergeReport>,
    pub released_surface_energy: f64,
    pub released_polymer_energy: f64,
    pub unresolved_kinetic_energy: f64,
    pub unresolved_angular_momentum: [f64; 3],
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn contact(
    a: [f64; 3],
    b: [f64; 3],
    end_a: [f64; 3],
    end_b: [f64; 3],
    radius: f64,
) -> Result<Option<f64>, Error> {
    let x: [f64; 3] = std::array::from_fn(|k| a[k] - b[k]);
    let d: [f64; 3] = std::array::from_fn(|k| end_a[k] - a[k] - end_b[k] + b[k]);
    if x.iter().chain(&d).any(|v| !v.is_finite()) {
        return Err(Error::NumericalFailure);
    }
    let scale = x.iter().chain(&d).fold(radius, |s, v| s.max(v.abs()));
    let x = x.map(|v| v / scale);
    let d = d.map(|v| v / scale);
    let r = radius / scale;
    let aa = dot(d, d);
    let bb = dot(x, d);
    let cc = dot(x, x) - r * r;
    if cc <= 64.0 * f64::EPSILON * r * r {
        return Ok(Some(0.0));
    }
    if aa <= 0.0 || bb >= 0.0 {
        return Ok(None);
    }
    let discriminant = bb * bb - aa * cc;
    if discriminant < 0.0 {
        return Ok(None);
    }
    let denominator = -bb + discriminant.sqrt();
    let t = cc / denominator;
    Ok((t.is_finite() && (0.0..=1.0).contains(&t)).then_some(t))
}
pub(super) fn pair_time(
    a: super::Particle,
    b: super::Particle,
    end_a: [f64; 3],
    end_b: [f64; 3],
    radius: f64,
    maximum_normal_speed: f64,
) -> Result<Option<f64>, Error> {
    if a.material != b.material {
        return Ok(None);
    }
    let Some(t) = contact(a.position, b.position, end_a, end_b, radius)? else {
        return Ok(None);
    };
    let x: [f64; 3] = std::array::from_fn(|k| {
        a.position[k] + t * (end_a[k] - a.position[k])
            - b.position[k]
            - t * (end_b[k] - b.position[k])
    });
    let relative: [f64; 3] = std::array::from_fn(|k| a.velocity[k] - b.velocity[k]);
    let distance = norm(x);
    let closing = if distance > 0.0 {
        -dot(relative, x) / distance
    } else {
        norm(relative)
    };
    if !closing.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if closing < 0.0 || closing > maximum_normal_speed {
        return Ok(None);
    }
    let path_relative: [f64; 3] =
        std::array::from_fn(|k| end_a[k] - a.position[k] - end_b[k] + b.position[k]);
    if distance > 0.0 && dot(path_relative, x) > 0.0 {
        return Ok(None);
    }
    Ok(Some(t))
}
impl Liquid {
    /// Sweeps linear paths, merges the earliest permitted pair, then continues its
    /// center-of-mass path for the remaining interval. Radius is volume-equivalent.
    /// Different material IDs do not merge. Same-material incompatible thermal or
    /// constitutive state rejects the entire operation. Forces remain frozen;
    /// substrate contacts, gas/phase models and collision breakup are not handled.
    pub fn coalesce_swept_droplets(
        &mut self,
        previous: &[[f64; 3]],
        control: DropletCoalescenceControl,
    ) -> Result<DropletCoalescenceReport, Error> {
        if previous.len() != self.particles.len()
            || previous.iter().flatten().any(|v| !v.is_finite())
            || !positive(control.dt)
            || control.dt > 0.1
            || !control.surface_tension.is_finite()
            || control.surface_tension < 0.0
            || !control.maximum_normal_speed.is_finite()
            || control.maximum_normal_speed < 0.0
            || !(1..=100000).contains(&control.max_events)
            || self.gas_active()
            || self.phase_fractions().is_some()
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        let mut ends: Vec<_> = self.particles.iter().map(|p| p.position).collect();
        for (p, start) in candidate.particles.iter_mut().zip(previous) {
            p.position = *start;
        }
        let mut report = DropletCoalescenceReport {
            released_polymer_energy: 0.0,
            events: Vec::new(),
            released_surface_energy: 0.0,
            unresolved_kinetic_energy: 0.0,
            unresolved_angular_momentum: [0.0; 3],
        };
        let mut checks = 0usize;
        loop {
            let radii = candidate.equivalent_sphere_radii()?;
            let mut next = None;
            for i in 0..candidate.particles.len() {
                for j in 0..i {
                    if candidate
                        .droplet_population
                        .as_ref()
                        .is_some_and(|flags| !flags[i] || !flags[j])
                    {
                        continue;
                    }

                    checks += 1;
                    if checks > candidate.config.max_neighbor_checks {
                        return Err(Error::NeighborBudget);
                    }
                    let Some(t) = pair_time(
                        candidate.particles[i],
                        candidate.particles[j],
                        ends[i],
                        ends[j],
                        radii[i] + radii[j],
                        control.maximum_normal_speed,
                    )?
                    else {
                        continue;
                    };
                    if next.is_none_or(|(_, _, old)| t < old) {
                        next = Some((i, j, t));
                    }
                }
            }
            let Some((i, j, t)) = next else {
                break;
            };
            if report.events.len() == control.max_events {
                return Err(Error::PairBudget);
            }
            let a = candidate.particles[i];
            let b = candidate.particles[j];
            let mass = a.mass + b.mass;
            let target =
                std::array::from_fn(|k| a.mass / mass * ends[i][k] + b.mass / mass * ends[j][k]);
            for (p, end) in candidate.particles.iter_mut().zip(&ends) {
                for k in 0..3 {
                    p.position[k] += t * (end[k] - p.position[k]);
                }
            }
            let event = candidate.merge_droplets(&[i, j], control.surface_tension)?;
            ends.remove(i);
            ends.remove(j);
            ends.push(target);
            report.released_surface_energy += event.released_surface_energy;
            report.released_polymer_energy += event.released_polymer_energy;
            report.unresolved_kinetic_energy += event.unresolved_kinetic_energy;
            for k in 0..3 {
                report.unresolved_angular_momentum[k] += event.unresolved_angular_momentum[k];
            }
            report.events.push(event);
        }
        for (p, end) in candidate.particles.iter_mut().zip(ends) {
            p.position = end;
        }
        if !report.released_surface_energy.is_finite()
            || !report.unresolved_kinetic_energy.is_finite()
            || report
                .unresolved_angular_momentum
                .iter()
                .any(|v| !v.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        candidate.effective_materials()?;
        *self = candidate;
        Ok(report)
    }
}

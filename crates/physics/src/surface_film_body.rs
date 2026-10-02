//! Finite translating body relaxation by a prescribed planar Newtonian film contact.
use crate::liquid::ThermalTranslatingBody;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlidingBodyReport {
    pub substrate_impulse: [f64; 3],
    pub dissipated_heat: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlidingAdvanceReport {
    pub mean_wetted_area: f64,
    pub substrate_impulse: [f64; 3],
    pub dissipated_heat: f64,
    pub substeps: usize,
}
impl super::SurfaceFilm {
    /// Exact Couette viscous relaxation, with a prescribed positive gap/wetted area
    /// and unit contact normal. Tangential body velocity decays at mu*A/(m*gap).
    /// Lost kinetic energy is deposited in the body's uniform thermal state.
    /// Film volume and body position are held fixed during this relaxation stage.
    /// The fixed lower substrate receives the opposite body momentum change.
    /// No contact geometry/normal load, pressure work or film inertia is inferred.
    pub fn relax_sliding_body(
        &self,
        dt: f64,
        gap: f64,
        area: f64,
        normal: [f64; 3],
        body: &mut ThermalTranslatingBody,
    ) -> Result<SlidingBodyReport, &'static str> {
        self.relax_sliding_body_with_viscosity(
            dt,
            gap,
            area,
            normal,
            body,
            self.material().viscosity,
        )
    }
    fn relax_sliding_body_with_viscosity(
        &self,
        dt: f64,
        gap: f64,
        area: f64,
        normal: [f64; 3],
        body: &mut ThermalTranslatingBody,
        viscosity: f64,
    ) -> Result<SlidingBodyReport, &'static str> {
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        if !dt.is_finite()
            || dt <= 0.0
            || !viscosity.is_finite()
            || viscosity <= 0.0
            || !gap.is_finite()
            || gap <= 0.0
            || !area.is_finite()
            || area <= 0.0
            || normal.iter().any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.0).abs() > 1e-12
            || body
                .mechanics
                .velocity
                .iter()
                .chain(&body.mechanics.position)
                .any(|v| !v.is_finite())
            || !(gap * area).is_finite()
            || gap * area > self.total_volume() * (1.0 + 1e-12)
        {
            return Err("invalid sliding body contact");
        }
        let normal = normal.map(|v| v / dot(normal, normal).sqrt());
        body.thermal_energy()
            .map_err(|_| "invalid sliding body thermal state")?;
        let exponent = viscosity * area / (body.mechanics.mass * gap) * dt;
        if !exponent.is_finite() {
            return Err("sliding body drag overflow");
        }
        let loss = -(-exponent).exp_m1();
        let normal_speed = dot(body.mechanics.velocity, normal);
        let tangent =
            std::array::from_fn(|k| body.mechanics.velocity[k] - normal_speed * normal[k]);
        let heat = 0.5 * body.mechanics.mass * dot(tangent, tangent) * loss * (2.0 - loss);
        let mut next = *body;
        let mut impulse = [0.0; 3];
        for k in 0..3 {
            next.mechanics.velocity[k] -= loss * tangent[k];
            impulse[k] = body.mechanics.mass * loss * tangent[k];
        }
        next.temperature += heat / (body.mechanics.mass * body.specific_heat);
        if !heat.is_finite()
            || next
                .mechanics
                .velocity
                .iter()
                .chain(&impulse)
                .any(|v| !v.is_finite())
            || (heat > 0.0 && next.temperature == body.temperature)
        {
            return Err("unrepresentable sliding body exchange");
        }
        next.thermal_energy()
            .map_err(|_| "sliding body thermal overflow")?;
        *body = next;
        Ok(SlidingBodyReport {
            substrate_impulse: impulse,
            dissipated_heat: heat,
        })
    }
}

/// Rectangular planar upper surface centered at body.mechanics.position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlidingPatch {
    pub normal: [f64; 3],
    pub tangent: [f64; 3],
    pub half_extents: [f64; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlidingPatchReport {
    pub wetted_area: f64,
    pub cells: usize,
    pub exchange: SlidingBodyReport,
}
impl super::SurfaceFilm {
    /// Exact planar triangle/rectangle overlap, with per-cell fluid depth checking.
    /// Nonparallel facets and nonpositive gaps are excluded. Partial triangles are
    /// clipped in the patch frame. Couette rates add as sum(mu*overlap/gap).
    /// No normal pressure, squeezing, patch advection, rotation or film inertia.
    pub fn relax_sliding_patch(
        &self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
    ) -> Result<SlidingPatchReport, &'static str> {
        self.sliding_patch_exchange(dt, patch, body)
            .map(|(report, _)| report)
    }

    /// Frozen-footprint body relaxation plus coverage-weighted Couette entrainment.
    /// Body and film roll back together. Pressure/squeeze/contact advection are not
    /// solved; overlap and wet membership remain frozen for this interval.
    pub fn step_sliding_patch(
        &mut self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
        max_substep: f64,
    ) -> Result<SlidingPatchReport, &'static str> {
        let mut candidate = *body;
        let (report, velocity) = self.sliding_patch_exchange(dt, patch, &mut candidate)?;
        self.step_with_advection(dt, [0.0; 3], &velocity, max_substep)?;
        *body = candidate;
        Ok(report)
    }

    /// Advances translating patch position, body relaxation and film entrainment.
    /// Contact is recomputed every bounded internal interval. Position uses the
    /// trapezoidal before/after velocity. Normal squeeze/nonpenetration is absent.
    /// Any failure restores all film volume and the complete external body.
    pub fn advance_sliding_patch(
        &mut self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
        max_substep: f64,
    ) -> Result<SlidingAdvanceReport, &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 0.1
            || !max_substep.is_finite()
            || max_substep <= 0.0
            || max_substep > 0.001
        {
            return Err("invalid moving patch interval");
        }
        let count = (dt / max_substep).ceil() as usize;
        if count > 100000 {
            return Err("moving patch substep budget");
        }
        let step = dt / count as f64;
        let original = self.volume.clone();
        let mut candidate = *body;
        let mut report = SlidingAdvanceReport {
            mean_wetted_area: 0.0,
            substrate_impulse: [0.0; 3],
            dissipated_heat: 0.0,
            substeps: count,
        };
        let result = (|| -> Result<(), &'static str> {
            for _ in 0..count {
                let before = candidate.mechanics.velocity;
                let current = self.step_sliding_patch(step, patch, &mut candidate, step)?;
                report.mean_wetted_area += current.wetted_area / count as f64;
                report.dissipated_heat += current.exchange.dissipated_heat;
                for k in 0..3 {
                    report.substrate_impulse[k] += current.exchange.substrate_impulse[k];
                    candidate.mechanics.position[k] +=
                        0.5 * step * (before[k] + candidate.mechanics.velocity[k]);
                }
                if candidate
                    .mechanics
                    .position
                    .iter()
                    .chain(&report.substrate_impulse)
                    .any(|v| !v.is_finite())
                    || !report.dissipated_heat.is_finite()
                    || !report.mean_wetted_area.is_finite()
                {
                    return Err("moving patch state overflow");
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.volume = original;
            return Err(error);
        }
        *body = candidate;
        Ok(report)
    }

    fn sliding_patch_exchange(
        &self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
    ) -> Result<(SlidingPatchReport, Vec<[f64; 3]>), &'static str> {
        self.sliding_patch_exchange_viscosities(dt, patch, body, None)
    }
    pub(super) fn sliding_patch_exchange_viscosities(
        &self,
        dt: f64,
        patch: SlidingPatch,
        body: &mut ThermalTranslatingBody,
        viscosities: Option<&[f64]>,
    ) -> Result<(SlidingPatchReport, Vec<[f64; 3]>), &'static str> {
        if viscosities.is_some_and(|values| {
            values.len() != self.volume.len() || values.iter().any(|v| !v.is_finite() || *v <= 0.0)
        }) {
            return Err("invalid patch viscosity field");
        }
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        if !dt.is_finite()
            || dt <= 0.0
            || patch
                .half_extents
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.0)
            || patch
                .normal
                .iter()
                .chain(&patch.tangent)
                .chain(&body.mechanics.position)
                .chain(&body.mechanics.velocity)
                .any(|v| !v.is_finite())
            || (dot(patch.normal, patch.normal) - 1.0).abs() > 1e-12
            || (dot(patch.tangent, patch.tangent) - 1.0).abs() > 1e-12
            || dot(patch.normal, patch.tangent).abs() > 1e-12
        {
            return Err("invalid sliding patch");
        }
        body.thermal_energy()
            .map_err(|_| "invalid patch body heat")?;
        let n = patch.normal;
        let t = patch.tangent;
        let b = [
            n[1] * t[2] - n[2] * t[1],
            n[2] * t[0] - n[0] * t[2],
            n[0] * t[1] - n[1] * t[0],
        ];
        let before_velocity = body.mechanics.velocity;
        let mut coverage = vec![0.0; self.volume.len()];
        let mut area = 0.0;
        let mut rate = 0.0;
        let mut viscous_rate = 0.0;
        let mut cells = 0;
        for (cell, triangle) in self.geometry.iter().enumerate() {
            if dot(self.normals[cell], n).abs() < 1.0 - 1e-12 {
                continue;
            }
            let offset = std::array::from_fn(|k| body.mechanics.position[k] - self.center[cell][k]);
            let gap = dot(offset, n);
            if gap <= 0.0 || self.volume[cell] / self.area[cell] < gap * (1.0 - 1e-12) {
                continue;
            }
            let mut polygon: Vec<[f64; 2]> = triangle
                .iter()
                .map(|point| {
                    let delta = std::array::from_fn(|k| point[k] - body.mechanics.position[k]);
                    [dot(delta, t), dot(delta, b)]
                })
                .collect();
            if polygon.iter().flatten().any(|v| !v.is_finite()) {
                return Err("patch geometry overflow");
            }
            for axis in 0..2 {
                for sign in [-1.0, 1.0] {
                    let mut clipped = Vec::new();
                    for i in 0..polygon.len() {
                        let a = polygon[i];
                        let end = polygon[(i + 1) % polygon.len()];
                        let da = sign * a[axis] - patch.half_extents[axis];
                        let db = sign * end[axis] - patch.half_extents[axis];
                        if da <= 0.0 {
                            clipped.push(a);
                        }
                        if (da <= 0.0) != (db <= 0.0) {
                            let fraction = da / (da - db);
                            clipped
                                .push(std::array::from_fn(|k| a[k] + fraction * (end[k] - a[k])));
                        }
                    }
                    polygon = clipped;
                }
            }
            let overlap = 0.5
                * (0..polygon.len())
                    .map(|i| {
                        let a = polygon[i];
                        let b = polygon[(i + 1) % polygon.len()];
                        a[0] * b[1] - a[1] * b[0]
                    })
                    .sum::<f64>()
                    .abs();
            if !overlap.is_finite() {
                return Err("patch area overflow");
            }
            if overlap > 0.0 {
                coverage[cell] = overlap / self.area[cell];
                area += overlap;
                rate += overlap / gap;
                if let Some(values) = viscosities {
                    viscous_rate += values[cell] * (overlap / gap);
                }
                cells += 1;
            }
        }
        if !area.is_finite() || !rate.is_finite() || !viscous_rate.is_finite() {
            return Err("patch drag overflow");
        }
        let viscosity = if viscosities.is_some() && area > 0.0 {
            viscous_rate / rate
        } else {
            self.material().viscosity
        };
        let exponent = if area > 0.0 {
            viscosity * area / (body.mechanics.mass * (area / rate)) * dt
        } else {
            0.0
        };
        if !exponent.is_finite() {
            return Err("patch entrainment overflow");
        }
        let exchange = if area > 0.0 {
            self.relax_sliding_body_with_viscosity(dt, area / rate, area, n, body, viscosity)?
        } else {
            SlidingBodyReport {
                substrate_impulse: [0.0; 3],
                dissipated_heat: 0.0,
            }
        };

        if area == 0.0 {
            return Ok((
                SlidingPatchReport {
                    wetted_area: area,
                    cells,
                    exchange,
                },
                vec![[0.0; 3]; self.volume.len()],
            ));
        }
        let mean = if exponent == 0.0 {
            1.0
        } else {
            -(-exponent).exp_m1() / exponent
        };
        let normal_speed = dot(before_velocity, n);
        let velocity = coverage
            .into_iter()
            .map(|fraction| {
                std::array::from_fn(|k| {
                    0.5 * fraction * mean * (before_velocity[k] - normal_speed * n[k])
                })
            })
            .collect();
        Ok((
            SlidingPatchReport {
                wetted_area: area,
                cells,
                exchange,
            },
            velocity,
        ))
    }
}

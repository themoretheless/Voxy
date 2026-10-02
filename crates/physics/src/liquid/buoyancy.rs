//! Hydrostatic spherical bodies coupled to particle momentum in prescribed horizontal layers.
use super::{Error, Liquid, finite, norm, positive};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingBody {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub mass: f64,
    pub radius: f64,
}
/// A hydrostatic layer, infinite in X/Z, with an explicit surface and material.
/// Heights describe a prescribed reservoir; they are not inferred from particle density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidLayer {
    pub bottom: f64,
    pub top: f64,
    pub material: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuoyancyConfig {
    /// Downward gravity magnitude. Fluid gravity must be integrated separately.
    pub gravity: f64,
    /// Quadratic sphere drag coefficient; zero disables quadratic drag.
    pub drag_coefficient: f64,
    pub max_layers: usize,
    pub max_checks: usize,
}
impl Default for BuoyancyConfig {
    fn default() -> Self {
        Self {
            gravity: 9.81,
            drag_coefficient: 0.47,
            max_layers: 64,
            max_checks: 1_000_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BuoyancyReport {
    pub submerged_volume: f64,
    pub displaced_mass: f64,
    /// Impulses received by the fluid, per layer. Excludes body weight.
    pub layer_impulses: Vec<[f64; 3]>,
}
impl Liquid {
    /// Applies buoyancy and dissipative drag with equal opposite particle impulses.
    /// Integrates body weight and translation; liquid positions are not advanced here.
    /// Call fluid integration separately with the same gravity and interval.
    /// Horizontal reservoirs are prescribed; this is not arbitrary-geometry body coupling.
    /// # Errors
    /// Invalid input, intersecting layers, no particle carrier for a submerged layer,
    /// exhausted sampling budget, or numerical overflow leave both liquid and body unchanged.
    #[allow(clippy::too_many_lines)]
    pub fn couple_floating_body(
        &mut self,
        body: &mut FloatingBody,
        layers: &[FluidLayer],
        dt: f64,
        config: BuoyancyConfig,
    ) -> Result<BuoyancyReport, Error> {
        if !positive(dt) || dt > 0.1 {
            return Err(Error::InvalidTimeStep);
        }
        if !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
            || !positive(body.radius)
            || !config.gravity.is_finite()
            || config.gravity < 0.0
            || !config.drag_coefficient.is_finite()
            || config.drag_coefficient < 0.0
            || config.max_layers == 0
            || config.max_checks == 0
            || layers.len() > config.max_layers
        {
            return Err(Error::InvalidBuoyancy);
        }
        let mut ordered: Vec<_> = layers.iter().collect();
        if layers.iter().any(|l| {
            !l.bottom.is_finite()
                || !l.top.is_finite()
                || l.bottom >= l.top
                || l.material >= self.materials.len()
        }) {
            return Err(Error::InvalidBuoyancy);
        }
        ordered.sort_by(|a, b| a.bottom.total_cmp(&b.bottom));
        if ordered.windows(2).any(|pair| pair[0].top > pair[1].bottom) {
            return Err(Error::InvalidBuoyancy);
        }
        let total_volume = 4.0 * PI / 3.0 * body.radius.powi(3);
        if !positive(total_volume) {
            return Err(Error::NumericalFailure);
        }
        let properties = self.effective_materials()?;
        let mut next = body.to_owned();
        let mut particles = self.particles.clone();
        let mut report = BuoyancyReport {
            submerged_volume: 0.0,
            displaced_mass: 0.0,
            layer_impulses: vec![[0.0; 3]; layers.len()],
        };
        let mut checks = 0;
        next.velocity[1] -= config.gravity * dt;
        for (layer_index, layer) in layers.iter().enumerate() {
            let volume = below(body.position[1], body.radius, layer.top)
                - below(body.position[1], body.radius, layer.bottom);
            if volume <= 0.0 {
                continue;
            }
            let mut material = self.materials[layer.material];
            report.submerged_volume += volume;

            let mut carriers = Vec::new();
            let mut carrier_mass = 0.0;
            let mut carrier_volume = 0.0;
            let mut viscosity_volume = 0.0;
            let mut momentum = [0.0; 3];
            for (index, particle) in particles.iter().enumerate() {
                if checks >= config.max_checks {
                    return Err(Error::BuoyancyBudget);
                }
                checks += 1;
                if particle.material == layer.material
                    && particle.position[1] >= layer.bottom
                    && particle.position[1] < layer.top
                {
                    carriers.push(index);
                    carrier_mass += particle.mass;
                    let volume = particle.mass / properties[index].rest_density;
                    carrier_volume += volume;
                    viscosity_volume += volume * properties[index].viscosity;
                    for (axis, value) in momentum.iter_mut().enumerate() {
                        *value += particle.mass * particle.velocity[axis];
                    }
                }
            }
            if carriers.is_empty() {
                return Err(Error::MissingFluidCarrier);
            }
            if !positive(carrier_mass) || !finite(momentum) {
                return Err(Error::NumericalFailure);
            }
            if !positive(carrier_volume) || !viscosity_volume.is_finite() {
                return Err(Error::NumericalFailure);
            }
            material.rest_density = carrier_mass / carrier_volume;
            material.viscosity = viscosity_volume / carrier_volume;
            report.displaced_mass += volume * material.rest_density;
            let buoyancy = config.gravity * volume * material.rest_density * dt;
            next.velocity[1] += buoyancy / next.mass;
            let mut impulse = [0.0, -buoyancy, 0.0];
            let fluid_velocity: [f64; 3] =
                std::array::from_fn(|axis| (momentum[axis] + impulse[axis]) / carrier_mass);
            let relative: [f64; 3] =
                std::array::from_fn(|axis| next.velocity[axis] - fluid_velocity[axis]);
            let fraction = volume / total_volume;
            // Fully submerged sphere: Stokes plus quadratic drag. Partial immersion is scaled.
            let linear = 6.0 * PI * material.viscosity * next.radius * fraction;
            let quadratic = 0.5
                * config.drag_coefficient
                * material.rest_density
                * PI
                * next.radius.powi(2)
                * fraction;
            let inverse_reduced_mass = 1.0 / next.mass + 1.0 / carrier_mass;
            let speed = norm(relative);
            let rate = linear * inverse_reduced_mass;
            // Exact decay of scalar speed for du/dt = -a u - b u² with fixed coefficients.
            let decay = (-rate * dt).exp();
            let integral = if rate > 1e-12 {
                -(-rate * dt).exp_m1() / rate
            } else {
                dt
            };
            let factor = decay / (1.0 + quadratic * inverse_reduced_mass * speed * integral);
            for (axis, value) in impulse.iter_mut().enumerate() {
                let drag = relative[axis] * (1.0 - factor) / inverse_reduced_mass;
                next.velocity[axis] -= drag / next.mass;
                *value += drag;
            }
            for index in carriers {
                for (axis, value) in impulse.iter().enumerate() {
                    particles[index].velocity[axis] += value / carrier_mass;
                }
            }
            report.layer_impulses[layer_index] = impulse;
        }
        for axis in 0..3 {
            next.position[axis] += next.velocity[axis] * dt;
        }
        if !finite(next.position)
            || !finite(next.velocity)
            || !report.submerged_volume.is_finite()
            || !report.displaced_mass.is_finite()
            || report
                .layer_impulses
                .iter()
                .any(|&impulse| !finite(impulse))
            || particles.iter().any(|p| !finite(p.velocity))
        {
            return Err(Error::NumericalFailure);
        }
        self.particles = particles;
        *body = next;
        Ok(report)
    }
}
// Sphere volume below a horizontal plane, evaluated as a spherical cap.
fn below(center: f64, radius: f64, plane: f64) -> f64 {
    let height = (plane - center + radius).clamp(0.0, 2.0 * radius);
    PI * height * height * (radius - height / 3.0)
}

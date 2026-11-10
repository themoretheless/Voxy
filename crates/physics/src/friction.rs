//! Fixed-normal, small-sliding penalty Coulomb contact with backward-Euler return.
//! Traction is an energy-gradient sign: force on the plus side is its negative.
use crate::math::Matrix;
mod slider;
use crate::math::dot;
pub use slider::SliderStep;
pub type Vec3 = [f64; 3];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    normal_stiffness: f64,
    tangential_stiffness: f64,
    coefficient: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    slip: Vec3,
    elastic_gap: Vec3,
    dissipated: f64,
    numerical_dissipated: f64,
    released: f64,
    closed: bool,
}
impl State {
    #[must_use]
    pub fn slip(&self) -> Vec3 {
        self.slip
    }
    #[must_use]
    pub fn dissipated_j_m2(&self) -> f64 {
        self.dissipated
    }
    #[must_use]
    pub fn numerical_dissipated_j_m2(&self) -> f64 {
        self.numerical_dissipated
    }
    #[must_use]
    pub fn released_j_m2(&self) -> f64 {
        self.released
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Open,
    Stick,
    Slip,
}
#[derive(Clone, Copy, Debug)]
pub struct Response {
    pub mode: Mode,
    pub pressure_pa: f64,
    pub tangential_traction_pa: Vec3,
    /// Includes the normal-gap derivative of the sliding friction limit.
    /// This matrix is generally nonsymmetric; it excludes normal contact stiffness.
    pub tangential_tangent_pa_m: Matrix,
    pub normal_stored_j_m2: f64,
    pub tangential_stored_j_m2: f64,
    pub dissipated_j_m2: f64,
    /// Cumulative backward-Euler endpoint-work defect, not physical friction heat.
    pub numerical_dissipated_j_m2: f64,
    /// Tangential penalty spring energy released on contact opening.
    pub released_j_m2: f64,
    pub slip_increment_m: f64,
}
impl Material {
    /// # Errors
    /// Positive finite penalty stiffnesses and finite mu >= 0 are required.
    #[must_use]
    pub fn normal_stiffness(&self) -> f64 {
        self.normal_stiffness
    }
    #[must_use]
    pub fn coefficient(&self) -> f64 {
        self.coefficient
    }
    pub fn new(
        normal_pa_m: f64,
        tangential_pa_m: f64,
        coefficient: f64,
    ) -> Result<Self, &'static str> {
        if !normal_pa_m.is_finite()
            || normal_pa_m <= 0.
            || !tangential_pa_m.is_finite()
            || tangential_pa_m <= 0.
            || !coefficient.is_finite()
            || coefficient < 0.
        {
            return Err("invalid friction material");
        }
        Ok(Self {
            normal_stiffness: normal_pa_m,
            tangential_stiffness: tangential_pa_m,
            coefficient,
        })
    }
    /// Reset the tangential reference while the contact is inactive (e.g. bonded).
    /// This operation does not add slip work; active springs must not be reset.
    /// # Errors
    /// Rejects invalid kinematics or a state with an active elastic spring.
    pub fn inactive_reference(
        &self,
        old: &State,
        jump: Vec3,
        normal: Vec3,
    ) -> Result<State, &'static str> {
        let (_, tangent) = kinematics(jump, normal)?;
        if old.closed {
            return Err("cannot reset active contact history");
        }
        Ok(State {
            slip: tangent,
            elastic_gap: [0.; 3],
            ..*old
        })
    }
    /// Returns a candidate state; nonlinear trials must use the last accepted state.
    /// Constant pressure proportional sliding dissipates `mu * p * plastic_slip`.
    /// Numerical return loss and released spring energy are distinct diagnostics.
    /// # Errors
    /// Rejects invalid normals/jumps and overflow without mutating old history.
    // Keep contact history transitions and all energy ledgers together.
    pub fn response(
        &self,
        old: &State,
        jump: Vec3,
        normal: Vec3,
    ) -> Result<(State, Response), &'static str> {
        let (normal_gap, tangent) = kinematics(jump, normal)?;
        let pressure = -self.normal_stiffness * normal_gap.min(0.);
        let limit = self.coefficient * pressure;
        if !pressure.is_finite() || !limit.is_finite() {
            return Err("friction pressure overflow");
        }
        let mut next = *old;
        let mut traction = [0.; 3];
        let mut stiffness = [[0.; 3]; 3];
        let mut increment = 0.;
        let mode = if normal_gap >= 0. {
            if old.closed {
                next.released +=
                    0.5 * self.tangential_stiffness * dot(old.elastic_gap, old.elastic_gap);
            }
            next.slip = tangent;
            next.elastic_gap = [0.; 3];
            next.closed = false;
            Mode::Open
        } else {
            next.closed = true;
            let elastic: Vec3 = std::array::from_fn(|i| tangent[i] - old.slip[i]);
            let length = elastic.iter().fold(0_f64, |a, v| a.hypot(*v));
            let elastic_limit = limit / self.tangential_stiffness;
            if !length.is_finite() || !elastic_limit.is_finite() {
                return Err("friction trial overflow");
            }
            if self.coefficient == 0. {
                next.slip = tangent;
                next.elastic_gap = [0.; 3];
                increment = length;
                Mode::Slip
            } else if length <= elastic_limit {
                next.elastic_gap = elastic;
                traction = elastic.map(|v| self.tangential_stiffness * v);
                stiffness = std::array::from_fn(|i| {
                    std::array::from_fn(|j| {
                        self.tangential_stiffness
                            * ((if i == j { 1. } else { 0. }) - normal[i] * normal[j])
                    })
                });
                Mode::Stick
            } else {
                let direction = elastic.map(|v| v / length);
                increment = length - elastic_limit;
                next.elastic_gap = direction.map(|v| v * elastic_limit);
                next.slip = std::array::from_fn(|i| tangent[i] - next.elastic_gap[i]);
                next.dissipated += limit * increment;
                traction = direction.map(|v| v * limit);
                stiffness = std::array::from_fn(|i| {
                    std::array::from_fn(|j| {
                        limit / length
                            * ((if i == j { 1. } else { 0. })
                                - normal[i] * normal[j]
                                - direction[i] * direction[j])
                            - self.coefficient * self.normal_stiffness * direction[i] * normal[j]
                    })
                });
                Mode::Slip
            }
        };
        if next.closed {
            // Backward-Euler endpoint-work defect: end traction * gap increment
            // = stored-energy change + friction work + this quadratic term.
            // It is not physical friction heat and vanishes on refinement.
            let change: Vec3 = std::array::from_fn(|i| next.elastic_gap[i] - old.elastic_gap[i]);
            next.numerical_dissipated += 0.5 * self.tangential_stiffness * dot(change, change);
        }
        let normal_stored = 0.5 * self.normal_stiffness * normal_gap.min(0.).powi(2);
        let tangential_stored =
            0.5 * self.tangential_stiffness * dot(next.elastic_gap, next.elastic_gap);
        if [
            normal_stored,
            tangential_stored,
            next.dissipated,
            next.numerical_dissipated,
            next.released,
            increment,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
            || next
                .slip
                .iter()
                .chain(&next.elastic_gap)
                .chain(&traction)
                .any(|v| !v.is_finite())
            || stiffness.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("friction response overflow");
        }
        Ok((
            next,
            Response {
                mode,
                pressure_pa: pressure,
                tangential_traction_pa: traction,
                tangential_tangent_pa_m: stiffness,
                normal_stored_j_m2: normal_stored,
                tangential_stored_j_m2: tangential_stored,
                dissipated_j_m2: next.dissipated,
                numerical_dissipated_j_m2: next.numerical_dissipated,
                released_j_m2: next.released,
                slip_increment_m: increment,
            },
        ))
    }
}
fn kinematics(jump: Vec3, normal: Vec3) -> Result<(f64, Vec3), &'static str> {
    if jump.iter().chain(&normal).any(|v| !v.is_finite())
        || (dot(normal, normal) - 1.).abs() > 1e-12
    {
        return Err("invalid friction kinematics");
    }
    let gap = dot(jump, normal);
    let tangent = std::array::from_fn(|i| jump[i] - gap * normal[i]);
    if !gap.is_finite() || tangent.iter().any(|v| !v.is_finite()) {
        return Err("friction kinematics overflow");
    }
    Ok((gap, tangent))
}

/// Integrates one step of the LuGre dynamic bristle friction model with Stribeck effect:
/// - Steady-state deflection envelope: $g(v) = \frac{F_c + (F_s - F_c) e^{-(v/v_s)^2}}{\sigma_0}$
/// - Bristle state evolution: $\dot{z} = v - \frac{|v|}{g(v)} z$
/// - Friction force: $F = \sigma_0 z + \sigma_1 \dot{z} + \sigma_2 v$.
///
/// Returns `(friction_force, next_bristle_z)`.
#[must_use]
pub fn lugre_bristle_friction_force(
    rel_velocity: f64,
    bristle_z: f64,
    sigma0: f64,
    sigma1: f64,
    sigma2: f64,
    coulomb_fc: f64,
    stiction_fs: f64,
    stribeck_vs: f64,
    dt: f64,
) -> Option<(f64, f64)> {
    if !rel_velocity.is_finite()
        || !bristle_z.is_finite()
        || !sigma0.is_finite()
        || !sigma1.is_finite()
        || !sigma2.is_finite()
        || !coulomb_fc.is_finite()
        || !stiction_fs.is_finite()
        || !stribeck_vs.is_finite()
        || !dt.is_finite()
        || sigma0 <= 0.0
        || sigma1 < 0.0
        || sigma2 < 0.0
        || coulomb_fc <= 0.0
        || stiction_fs < coulomb_fc
        || stribeck_vs <= 0.0
        || dt < 0.0
    {
        return None;
    }
    let stribeck = (-(rel_velocity / stribeck_vs).powi(2)).exp();
    let f_env = coulomb_fc + (stiction_fs - coulomb_fc) * stribeck;
    let g_v = (f_env / sigma0).max(1e-12);
    let rate = rel_velocity.abs() / g_v;
    let z_next = if rate * dt > 1e-6 {
        let z_ss = rel_velocity.signum() * g_v;
        z_ss + (bristle_z - z_ss) * (-rate * dt).exp()
    } else {
        bristle_z + (rel_velocity - rate * bristle_z) * dt
    };
    let dz_dt = if dt > 1e-12 {
        (z_next - bristle_z) / dt
    } else {
        rel_velocity - rate * bristle_z
    };
    let force = sigma0 * z_next + sigma1 * dz_dt + sigma2 * rel_velocity;
    (force.is_finite() && z_next.is_finite()).then_some((force, z_next))
}

#[cfg(test)]
mod lugre_tests {
    use super::*;

    #[test]
    fn lugre_bristle_model_exhibits_stribeck_transition_from_stiction_to_coulomb() {
        let mut z_slow = 0.0;
        let mut f_slow = 0.0;
        // At 1 mm/s the bristle relaxes with rate |v| / g(v) = 6.7 1/s, so the
        // stiction plateau needs seconds of sliding: 0.1 s reaches only 7.5 N.
        for _ in 0..2000 {
            (f_slow, z_slow) = lugre_bristle_friction_force(
                0.001, z_slow, 1e5, 300.0, 0.4, 10.0, 15.0, 0.05, 0.001,
            )
            .unwrap();
        }
        let mut z_fast = 0.0;
        let mut f_fast = 0.0;
        for _ in 0..100 {
            (f_fast, z_fast) =
                lugre_bristle_friction_force(0.5, z_fast, 1e5, 300.0, 0.0, 10.0, 15.0, 0.05, 0.001)
                    .unwrap();
        }
        assert!(f_slow > f_fast && f_fast >= 9.9);
    }
}

//! Bilinear energy-regularized cohesive interface with optional Coulomb crack closure.
//! Fixed unit normal, small sliding. Equal mode-I/II stiffness and fracture energy.
use crate::biomechanics::Matrix;
pub type Vec3 = [f64; 3];
#[derive(Clone, Copy, Debug)]
pub struct Material {
    stiffness_pa_m: f64,
    closure_pa_m: f64,
    fracture_j_m2: f64,
    onset_m: f64,
    failure_m: f64,
    friction: Option<crate::friction::Material>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    maximum_separation_m: f64,
    contact: crate::friction::State,
    fracture_offset_j_m2: f64,
}
impl State {
    #[must_use]
    pub fn maximum_separation_m(&self) -> f64 {
        self.maximum_separation_m
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Response {
    /// Energy gradient with respect to plus-side minus minus-side displacement.
    /// Physical force on the plus side has the opposite sign.
    pub traction_pa: Vec3,
    pub tangent_pa_m: Matrix,
    pub damage: f64,
    pub stored_j_m2: f64,
    /// Cohesive fracture work only; friction work is reported separately below.
    pub dissipated_j_m2: f64,
    pub effective_separation_m: f64,
    /// Physical friction work, kept separate from cohesive fracture work.
    pub friction_dissipated_j_m2: f64,
    pub friction_numerical_j_m2: f64,
    pub friction_released_j_m2: f64,
    pub friction_mode: Option<crate::friction::Mode>,
}
impl Material {
    /// # Errors
    /// All constants must be positive finite. Requires 2Gc/T > T/K:
    /// fracture energy must exceed the energy stored at peak traction.
    pub fn new(
        stiffness_pa_m: f64,
        closure_pa_m: f64,
        peak_pa: f64,
        fracture_j_m2: f64,
    ) -> Result<Self, &'static str> {
        if [stiffness_pa_m, closure_pa_m, peak_pa, fracture_j_m2]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid cohesive material");
        }
        let onset_m = peak_pa / stiffness_pa_m;
        let failure_m = 2. * (fracture_j_m2 / peak_pa);
        if !onset_m.is_finite() || onset_m <= 0. || !failure_m.is_finite() || failure_m <= onset_m {
            return Err("invalid cohesive separation range");
        }
        Ok(Self {
            stiffness_pa_m,
            closure_pa_m,
            fracture_j_m2,
            onset_m,
            failure_m,
            friction: None,
        })
    }
    /// Enable Coulomb contact after complete fracture has been accepted.
    /// Before that transition the cohesive shear law alone transmits traction.
    /// # Errors
    /// Rejects nonpositive tangential stiffness or nonfinite/negative mu.
    pub fn with_friction(
        mut self,
        coefficient: f64,
        tangential_pa_m: f64,
    ) -> Result<Self, &'static str> {
        self.friction = Some(crate::friction::Material::new(
            self.closure_pa_m,
            tangential_pa_m,
            coefficient,
        )?);
        Ok(self)
    }
    #[must_use]
    pub fn onset_m(&self) -> f64 {
        self.onset_m
    }
    #[must_use]
    pub fn failure_m(&self) -> f64 {
        self.failure_m
    }
    /// Irreversible damage from accepted maximum separation, independent of pose.
    #[must_use]
    pub fn damage(&self, state: &State) -> f64 {
        let maximum = state.maximum_separation_m;
        if maximum <= self.onset_m {
            0.
        } else if maximum >= self.failure_m {
            1.
        } else {
            ((self.failure_m / maximum)
                * ((maximum - self.onset_m) / (self.failure_m - self.onset_m)).clamp(0., 1.))
            .clamp(0., 1.)
        }
    }
    /// Candidate history and consistent tangent for total displacement jump.
    /// Normal must be unit length; compression never degrades normal contact.
    /// Commit history only after a global step has converged. No healing.
    /// # Errors
    /// Rejects invalid normal, nonfinite jumps and unrepresentable responses.
    #[allow(clippy::too_many_lines)] // Validate the joint cohesive/contact candidate before publication.
    pub fn response(
        &self,
        old: &State,
        jump: Vec3,
        normal: Vec3,
    ) -> Result<(State, Response), &'static str> {
        if jump.iter().chain(&normal).any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.).abs() > 1e-12
        {
            return Err("invalid cohesive kinematics");
        }
        let normal_gap = dot(jump, normal);
        let effective: Vec3 = std::array::from_fn(|i| jump[i] - normal_gap.min(0.) * normal[i]);
        let separation = effective.iter().fold(0_f64, |a, v| a.hypot(*v));
        let maximum = old.maximum_separation_m.max(separation);
        let mut next = State {
            maximum_separation_m: maximum,
            contact: old.contact,
            fracture_offset_j_m2: old.fracture_offset_j_m2,
        };
        let progress = ((maximum - self.onset_m) / (self.failure_m - self.onset_m)).clamp(0., 1.);
        let damage = self.damage(&next);
        // Compute the remaining stiffness directly to avoid 1-d cancellation
        // close to complete separation.
        let remaining = if maximum <= self.onset_m {
            1.
        } else if maximum >= self.failure_m {
            0.
        } else {
            (self.onset_m / maximum)
                * ((self.failure_m - maximum) / (self.failure_m - self.onset_m))
        };
        let mut traction: Vec3 = std::array::from_fn(|i| {
            self.stiffness_pa_m * remaining * effective[i]
                + self.closure_pa_m * normal_gap.min(0.) * normal[i]
        });
        let loading = separation > old.maximum_separation_m
            && separation > self.onset_m
            && separation < self.failure_m;
        let derivative = if loading {
            self.failure_m * self.onset_m / ((self.failure_m - self.onset_m) * maximum * maximum)
        } else {
            0.
        };
        let mut tangent: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let normal_projection = normal[i] * normal[j];
                self.stiffness_pa_m
                    * remaining
                    * ((if i == j { 1. } else { 0. })
                        - if normal_gap < 0. {
                            normal_projection
                        } else {
                            0.
                        })
                    - if loading {
                        self.stiffness_pa_m
                            * derivative
                            * (effective[i] / separation)
                            * effective[j]
                    } else {
                        0.
                    }
                    + if normal_gap < 0. {
                        self.closure_pa_m * normal_projection
                    } else {
                        0.
                    }
            })
        });
        let mut stored = 0.5 * self.stiffness_pa_m * remaining * separation * separation
            + 0.5 * self.closure_pa_m * normal_gap.min(0.).powi(2);
        let dissipated = self.fracture_j_m2 * progress + old.fracture_offset_j_m2;
        let mut friction_dissipated = 0.;
        let mut friction_numerical = 0.;
        let mut friction_released = 0.;
        let mut friction_mode = None;
        if let Some(material) = self.friction {
            if old.maximum_separation_m >= self.failure_m {
                let (state, response) = material.response(&old.contact, jump, normal)?;
                next.contact = state;
                for (i, value) in traction.iter_mut().enumerate() {
                    *value += response.tangential_traction_pa[i];
                    for (j, value) in tangent[i].iter_mut().enumerate() {
                        *value += response.tangential_tangent_pa_m[i][j];
                    }
                }
                stored += response.tangential_stored_j_m2;
                friction_dissipated = response.dissipated_j_m2;
                friction_numerical = response.numerical_dissipated_j_m2;
                friction_released = response.released_j_m2;
                friction_mode = Some(response.mode);
            } else {
                next.contact = material.inactive_reference(&old.contact, jump, normal)?;
            }
        }
        if !normal_gap.is_finite()
            || !separation.is_finite()
            || !maximum.is_finite()
            || !damage.is_finite()
            || !stored.is_finite()
            || !dissipated.is_finite()
            || traction.iter().any(|v| !v.is_finite())
            || tangent.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("cohesive response overflow");
        }
        Ok((
            next,
            Response {
                traction_pa: traction,
                tangent_pa_m: tangent,
                damage,
                stored_j_m2: stored,
                dissipated_j_m2: dissipated,
                effective_separation_m: separation,
                friction_dissipated_j_m2: friction_dissipated,
                friction_numerical_j_m2: friction_numerical,
                friction_released_j_m2: friction_released,
                friction_mode,
            },
        ))
    }
}
fn dot(a: Vec3, b: Vec3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

impl Material {
    /// Migrate accepted history to a changed frictionless cohesive law at fixed
    /// pose. Preserves accumulated fracture work using a history offset and
    /// rejects any law that would reduce existing damage. Returned signed work
    /// is the change in stored energy per reference area and must be accounted
    /// as external parameter work. This is not a wet chemistry energy model.
    /// # Errors
    /// Healing, new loading, friction-history migration or nonfinite balances.
    pub fn migrate_history(
        &self,
        old_material: &Self,
        old: &State,
        jump: Vec3,
        normal: Vec3,
    ) -> Result<(State, f64), &'static str> {
        if self.friction.is_some() || old_material.friction.is_some() {
            return Err("cohesive friction history migration unsupported");
        }
        if self.damage(old) < old_material.damage(old) {
            return Err("cohesive parameter change would heal damage");
        }
        let (accepted, before) = old_material.response(old, jump, normal)?;
        if accepted.maximum_separation_m != old.maximum_separation_m {
            return Err("cohesive migration requires accepted fixed pose");
        }
        let (mut next, after) = self.response(old, jump, normal)?;
        next.fracture_offset_j_m2 += before.dissipated_j_m2 - after.dissipated_j_m2;
        let work = after.stored_j_m2 - before.stored_j_m2;
        if !next.fracture_offset_j_m2.is_finite() || !work.is_finite() {
            return Err("cohesive parameter work overflow");
        }
        let (_, verified) = self.response(&next, jump, normal)?;
        if (verified.dissipated_j_m2 - before.dissipated_j_m2).abs()
            > 1e-12 * before.dissipated_j_m2.abs().max(self.fracture_j_m2)
        {
            return Err("cohesive history migration balance failure");
        }
        Ok((next, work))
    }
}

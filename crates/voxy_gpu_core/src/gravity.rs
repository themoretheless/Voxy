//! Backend-neutral N-body gravity contract (plan step D4).
//!
//! Every compute backend that integrates gravity (`voxy_gpu` on wgpu, `voxy_cuda` on CUDA)
//! exposes its resident job through [`GravitySolver`], and the body and parameter records are
//! shared here so the backends differ only in precision and execution model. The reference
//! check [`compare_with_oracle`] drives any solver and an independent CPU oracle over the same
//! step schedule and reports the largest divergence, so each backend's test is the same
//! statement: "backend == CPU oracle within tolerance".

/// Scalar precision a gravity backend computes in.
pub trait GravityScalar: Copy + PartialEq + std::fmt::Debug + Into<f64> {
    /// Rounds an `f64` reference value to this precision.
    fn from_f64(value: f64) -> Self;
}
impl GravityScalar for f32 {
    #[allow(clippy::cast_possible_truncation)]
    fn from_f64(value: f64) -> Self {
        value as Self
    }
}
impl GravityScalar for f64 {
    fn from_f64(value: f64) -> Self {
        value
    }
}

/// One gravitating body; mass must be positive and all data finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityBody<T> {
    pub mass: T,
    pub position: [T; 3],
    pub velocity: [T; 3],
}
impl<T: GravityScalar> GravityBody<T> {
    /// The same body at `f64` precision.
    #[must_use]
    pub fn widened(self) -> GravityBody<f64> {
        GravityBody {
            mass: self.mass.into(),
            position: self.position.map(Into::into),
            velocity: self.velocity.map(Into::into),
        }
    }
}
impl GravityBody<f64> {
    /// The body rounded to a backend's precision.
    #[must_use]
    pub fn narrowed<T: GravityScalar>(self) -> GravityBody<T> {
        GravityBody {
            mass: T::from_f64(self.mass),
            position: self.position.map(T::from_f64),
            velocity: self.velocity.map(T::from_f64),
        }
    }
}

/// Integration constants shared by every backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityParameters<T> {
    pub constant: T,
    /// Plummer softening length; zero gives the exact inverse-square law.
    pub softening: T,
    pub uniform_acceleration: [T; 3],
    pub dt: T,
}

/// A resident N-body system a backend can advance and read back.
pub trait GravitySolver {
    /// Precision the backend integrates in.
    type Scalar: GravityScalar;
    /// Backend failure type.
    type Error: std::error::Error;
    /// Number of committed bodies.
    fn body_count(&self) -> u32;
    /// Advances the system by `steps` velocity-Verlet steps of `dt`.
    ///
    /// # Errors
    /// Rejects step counts outside the backend's budget and reports device failures.
    fn step(&mut self, steps: u32) -> Result<(), Self::Error>;
    /// Reads the committed bodies after all prior steps.
    ///
    /// # Errors
    /// Reports singular pairs, numerical overflow, malformed output and device failures.
    fn read(&mut self) -> Result<Vec<GravityBody<Self::Scalar>>, Self::Error>;
}

/// Outcome of [`compare_with_oracle`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityReferenceReport {
    /// Rounds the solver and oracle were advanced.
    pub rounds: u32,
    /// Steps per round.
    pub steps_per_round: u32,
    /// Largest absolute position or velocity difference over all bodies and components.
    pub max_difference: f64,
}

/// Why a backend failed the reference check.
#[derive(Debug, thiserror::Error)]
pub enum GravityReferenceError<E: std::error::Error> {
    #[error("gravity backend: {0}")]
    Backend(E),
    #[error("gravity oracle: {0}")]
    Oracle(String),
    #[error("gravity backend returned {actual} bodies, oracle holds {expected}")]
    BodyCount { actual: usize, expected: usize },
    #[error("gravity body {body}: mass changed on the backend")]
    MassChanged { body: usize },
    #[error(
        "gravity body {body}, component {component}: differs by {difference} (tolerance {tolerance})"
    )]
    Mismatch {
        body: usize,
        component: usize,
        difference: f64,
        tolerance: f64,
    },
}

/// Advances `solver` and `reference` over the same schedule, then compares them.
///
/// `oracle` advances the `f64` reference bodies by the given number of steps. Masses must
/// come back bit-identical; positions and velocities must agree within `tolerance`.
///
/// # Errors
/// Reports backend and oracle failures, and the first body that diverges.
pub fn compare_with_oracle<S, O>(
    solver: &mut S,
    reference: &mut [GravityBody<f64>],
    rounds: u32,
    steps_per_round: u32,
    tolerance: f64,
    mut oracle: O,
) -> Result<GravityReferenceReport, GravityReferenceError<S::Error>>
where
    S: GravitySolver,
    O: FnMut(&mut [GravityBody<f64>], u32) -> Result<(), String>,
{
    for _ in 0..rounds {
        solver
            .step(steps_per_round)
            .map_err(GravityReferenceError::Backend)?;
        oracle(reference, steps_per_round).map_err(GravityReferenceError::Oracle)?;
    }
    let actual = solver.read().map_err(GravityReferenceError::Backend)?;
    if actual.len() != reference.len() {
        return Err(GravityReferenceError::BodyCount {
            actual: actual.len(),
            expected: reference.len(),
        });
    }
    let mut max_difference = 0.0_f64;
    for (body, (actual, expected)) in actual.iter().zip(reference.iter()).enumerate() {
        let actual = actual.widened();
        if actual.mass.to_bits() != expected.mass.to_bits() {
            return Err(GravityReferenceError::MassChanged { body });
        }
        let actual_components = actual.position.iter().chain(&actual.velocity);
        let expected_components = expected.position.iter().chain(&expected.velocity);
        for (component, (actual, expected)) in
            actual_components.zip(expected_components).enumerate()
        {
            let difference = (actual - expected).abs();
            if !difference.is_finite() || difference > tolerance {
                return Err(GravityReferenceError::Mismatch {
                    body,
                    component,
                    difference,
                    tolerance,
                });
            }
            max_difference = max_difference.max(difference);
        }
    }
    Ok(GravityReferenceReport {
        rounds,
        steps_per_round,
        max_difference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solver that integrates uniform acceleration exactly, in f32.
    struct UniformFall {
        bodies: Vec<GravityBody<f32>>,
        acceleration: [f32; 3],
        dt: f32,
    }
    #[derive(Debug, thiserror::Error)]
    #[error("budget")]
    struct Budget;
    impl GravitySolver for UniformFall {
        type Scalar = f32;
        type Error = Budget;
        fn body_count(&self) -> u32 {
            u32::try_from(self.bodies.len()).unwrap()
        }
        fn step(&mut self, steps: u32) -> Result<(), Budget> {
            if steps == 0 {
                return Err(Budget);
            }
            for _ in 0..steps {
                for body in &mut self.bodies {
                    for k in 0..3 {
                        body.velocity[k] += self.acceleration[k] * self.dt;
                        body.position[k] += body.velocity[k] * self.dt;
                    }
                }
            }
            Ok(())
        }
        fn read(&mut self) -> Result<Vec<GravityBody<f32>>, Budget> {
            Ok(self.bodies.clone())
        }
    }

    fn oracle(
        acceleration: [f64; 3],
        dt: f64,
    ) -> impl FnMut(&mut [GravityBody<f64>], u32) -> Result<(), String> {
        move |bodies, steps| {
            for _ in 0..steps {
                for body in bodies.iter_mut() {
                    for k in 0..3 {
                        body.velocity[k] += acceleration[k] * dt;
                        body.position[k] += body.velocity[k] * dt;
                    }
                }
            }
            Ok(())
        }
    }

    fn bodies() -> Vec<GravityBody<f32>> {
        (0..5)
            .map(|i| GravityBody {
                mass: 1.0 + i as f32 * 0.5,
                position: [i as f32, 0.0, -(i as f32)],
                velocity: [0.0, 0.25, 0.0],
            })
            .collect()
    }

    #[test]
    fn agreeing_solver_and_oracle_report_their_largest_rounding_gap() {
        let mut solver = UniformFall {
            bodies: bodies(),
            acceleration: [0.0, -9.81, 0.0],
            dt: 1.0 / 64.0,
        };
        let mut reference: Vec<_> = bodies().into_iter().map(GravityBody::widened).collect();
        let report = compare_with_oracle(
            &mut solver,
            &mut reference,
            3,
            8,
            1e-4,
            oracle([0.0, -9.81, 0.0], 1.0 / 64.0),
        )
        .unwrap();
        assert_eq!((report.rounds, report.steps_per_round), (3, 8));
        assert!(report.max_difference > 0.0 && report.max_difference <= 1e-4);
    }

    #[test]
    fn a_diverging_backend_names_the_first_body_and_component() {
        let mut solver = UniformFall {
            bodies: bodies(),
            acceleration: [0.0, -9.81, 0.0],
            dt: 1.0 / 64.0,
        };
        let mut reference: Vec<_> = bodies().into_iter().map(GravityBody::widened).collect();
        let error = compare_with_oracle(
            &mut solver,
            &mut reference,
            1,
            4,
            1e-9,
            oracle([0.0, -9.0, 0.0], 1.0 / 64.0),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            GravityReferenceError::Mismatch {
                body: 0,
                component: 1,
                ..
            }
        ));
    }

    #[test]
    fn backend_budget_failures_and_mass_changes_are_reported() {
        let mut solver = UniformFall {
            bodies: bodies(),
            acceleration: [0.0; 3],
            dt: 1.0,
        };
        let mut reference: Vec<_> = bodies().into_iter().map(GravityBody::widened).collect();
        assert!(matches!(
            compare_with_oracle(
                &mut solver,
                &mut reference,
                1,
                0,
                1e-6,
                oracle([0.0; 3], 1.0)
            ),
            Err(GravityReferenceError::Backend(Budget))
        ));
        reference[2].mass += 1.0;
        assert!(matches!(
            compare_with_oracle(
                &mut solver,
                &mut reference,
                1,
                1,
                1e-6,
                oracle([0.0; 3], 1.0)
            ),
            Err(GravityReferenceError::MassChanged { body: 2 })
        ));
    }

    #[test]
    fn narrowing_then_widening_round_trips_f32_values() {
        let body = GravityBody {
            mass: 1.5,
            position: [0.25, -2.0, 1024.0],
            velocity: [0.125, 0.0, -0.5],
        };
        assert_eq!(body.narrowed::<f32>().widened(), body);
    }
}

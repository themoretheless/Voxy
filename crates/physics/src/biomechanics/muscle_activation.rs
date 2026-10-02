//! Explicit phenomenological activation kinetics, not calcium or motor-unit dynamics.
use super::{Body, Equilibrium};
#[derive(Clone, Copy, Debug)]
pub struct ActivationKinetics {
    pub rise_seconds: f64,
    pub fall_seconds: f64,
    pub tonic_activation: f64,
}
impl ActivationKinetics {
    /// Exact first-order response to a constant excitation over the supplied time.
    /// # Errors
    /// Invalid state/excitation, nonpositive time constants or elapsed time.
    pub fn advance(
        self,
        activation: f64,
        excitation: f64,
        seconds: f64,
    ) -> Result<f64, &'static str> {
        if [activation, excitation, self.tonic_activation]
            .iter()
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
            || [self.rise_seconds, self.fall_seconds, seconds]
                .iter()
                .any(|x| !x.is_finite() || *x <= 0.)
        {
            return Err("invalid muscle activation kinetics");
        }
        let target = self.tonic_activation + (1. - self.tonic_activation) * excitation;
        let tau = if target >= activation {
            self.rise_seconds
        } else {
            self.fall_seconds
        };
        let next = activation + (target - activation) * (-(-seconds / tau).exp_m1());
        if !next.is_finite() || !(0. ..=1.).contains(&next) {
            return Err("muscle activation overflow");
        }
        Ok(next)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct MuscleRegionDrive {
    pub region: usize,
    pub activation: f64,
    pub excitation: f64,
    pub kinetics: ActivationKinetics,
}
impl Body {
    /// Advance explicit region activation and equilibrate staged tissue together.
    /// This is quasistatic mechanics with activation time, not inertial solid dynamics.
    /// # Errors
    /// Missing/duplicate region, invalid kinetics or failed equilibrium rolls back
    /// geometry, element activation and all caller activation histories.
    pub fn step_muscle_regions(
        &mut self,
        drives: &mut [MuscleRegionDrive],
        seconds: f64,
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<Equilibrium, &'static str> {
        if drives.is_empty() {
            return Err("empty muscle drive");
        }
        let mut body = self.clone();
        let mut next = drives.to_vec();
        let mut seen = std::collections::BTreeSet::new();
        for drive in &mut next {
            if !seen.insert(drive.region) {
                return Err("duplicate muscle region");
            }
            drive.activation =
                drive
                    .kinetics
                    .advance(drive.activation, drive.excitation, seconds)?;
            let ids: Vec<_> = body
                .elements
                .iter()
                .enumerate()
                .filter_map(|(i, e)| (e.region == drive.region).then_some(i))
                .collect();
            if ids.is_empty() {
                return Err("missing muscle region");
            }
            for i in ids {
                body.set_activation(i, drive.activation)?;
            }
        }
        let report = body.equilibrate(max_iterations, tolerance_n)?;
        if !report.converged {
            return Err("muscle tissue equilibrium nonconvergence");
        }
        *self = body;
        drives.copy_from_slice(&next);
        Ok(report)
    }
}

/// Explicit compact bell-shaped active force/stretch curve; parameters are not fitted.
#[derive(Clone, Copy, Debug)]
pub struct ActiveFiberLengthLaw {
    pub optimal_stretch: f64,
    pub half_width: f64,
}
impl ActiveFiberLengthLaw {
    pub(super) fn validate(self) -> Result<(), &'static str> {
        if [self.optimal_stretch, self.half_width]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.)
        {
            return Err("invalid active fiber length law");
        }
        Ok(())
    }
    /// Returns the normalized active tension and its dimensionless potential,
    /// with zero potential at reference stretch one. Derivative of potential is tension.
    /// # Errors
    /// Invalid parameters/stretch or derived nonfinite response.
    pub fn response(self, stretch: f64) -> Result<(f64, f64), &'static str> {
        self.validate()?;
        if !stretch.is_finite() || stretch <= 0. {
            return Err("invalid active fiber stretch");
        }
        let x = (stretch - self.optimal_stretch) / self.half_width;
        let primitive = |v: f64| {
            let t = v.clamp(-1., 1.);
            t - 2. * t.powi(3) / 3. + t.powi(5) / 5.
        };
        let tension = if x.abs() < 1. {
            (1. - x * x).powi(2)
        } else {
            0.
        };
        let potential = self.half_width
            * (primitive(x) - primitive((1. - self.optimal_stretch) / self.half_width));
        if !potential.is_finite() || !tension.is_finite() {
            return Err("active fiber response overflow");
        }
        Ok((tension, potential))
    }
}
impl Body {
    /// Assign a length-dependent active law to an aligned-fiber material element.
    /// # Errors
    /// Invalid index/law or conflicting cardiac/viscoelastic constitutive assignment.
    pub fn set_active_fiber_length_law(
        &mut self,
        index: usize,
        law: ActiveFiberLengthLaw,
    ) -> Result<(), &'static str> {
        law.validate()?;
        let element = self
            .elements
            .get_mut(index)
            .ok_or("invalid active muscle element")?;
        if element.myocardium.is_some() || element.viscoelastic.is_some() {
            return Err("conflicting active muscle material");
        }
        element.active_length_law = Some(law);
        Ok(())
    }
}

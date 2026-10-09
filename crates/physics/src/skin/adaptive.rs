use super::{Attachment, Point, Skin, SolverConfig, StepReport};

impl Skin {
    /// Advance an entire interval atomically, bisecting failed nonlinear solves.
    /// Attachment targets follow their original linear trajectory throughout retries.
    /// This method has no moving contact scene; use `step_with_contacts` for contacts.
    /// The report sums accepted iterations, retains the maximum accepted residual,
    /// minimum area ratio, and final substep energy.
    pub fn step_adaptive(
        &mut self,
        dt: f64,
        acceleration: Point,
        forces: &[Point],
        attachments: &[Attachment],
        config: SolverConfig,
        max_depth: usize,
    ) -> Result<StepReport, &'static str> {
        if max_depth > 12 {
            return Err("invalid skin subdivision depth");
        }
        let mut staged = self.clone();
        let report = advance(
            &mut staged,
            dt,
            0.0,
            acceleration,
            forces,
            attachments,
            config,
            max_depth,
        )?;
        *self = staged;
        Ok(report)
    }
}

fn advance(
    skin: &mut Skin,
    dt: f64,
    elapsed: f64,
    acceleration: Point,
    forces: &[Point],
    attachments: &[Attachment],
    config: SolverConfig,
    depth: usize,
) -> Result<StepReport, &'static str> {
    let shifted: Vec<_> = attachments
        .iter()
        .map(|a| Attachment {
            target: std::array::from_fn(|k| a.target[k] + elapsed * a.velocity[k]),
            ..*a
        })
        .collect();
    match skin.step(dt, acceleration, forces, &shifted, config) {
        Ok(report) => Ok(report),
        Err(error)
            if depth > 0
                && matches!(
                    error,
                    "skin Newton did not converge"
                        | "skin line search failed"
                        | "skin swept contact or element collapse"
                ) =>
        {
            let first = advance(
                skin,
                dt * 0.5,
                elapsed,
                acceleration,
                forces,
                attachments,
                config,
                depth - 1,
            )?;
            let second = advance(
                skin,
                dt * 0.5,
                elapsed + dt * 0.5,
                acceleration,
                forces,
                attachments,
                config,
                depth - 1,
            )?;
            Ok(StepReport {
                iterations: first.iterations + second.iterations,
                residual: first.residual.max(second.residual),
                energy: second.energy,
                min_area_ratio: first.min_area_ratio.min(second.min_area_ratio),
            })
        }
        Err(error) => Err(error),
    }
}

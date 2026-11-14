//! Axisymmetric unsteady Stokes flow in a rigid circular tube, SI units.
//! Fixed radius, no slip, Newtonian viscosity; no axial convection or wall motion.
#[derive(Clone, Debug)]
pub struct RadialPipe {
    radius: f64,
    density: f64,
    viscosity: f64,
    velocities: Vec<f64>,
}
#[derive(Clone, Copy, Debug)]
pub struct RadialPipeStep {
    pub flow_m3_per_s: f64,
    /// dQ/d(pressure gradient), m4/(Pa s).
    pub flow_tangent: f64,
    pub kinetic_energy_j_per_m: f64,
    pub viscous_power_w_per_m: f64,
}
impl RadialPipe {
    /// Start a rigid pipe at rest with annular finite-volume cells.
    /// # Errors
    /// Nonpositive properties, invalid resolution or derived coefficient overflow.
    pub fn new(
        radius_m: f64,
        density_kg_per_m3: f64,
        viscosity_pa_s: f64,
        cells: usize,
    ) -> Result<Self, &'static str> {
        if [radius_m, density_kg_per_m3, viscosity_pa_s]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.)
            || !(2..=100000).contains(&cells)
        {
            return Err("invalid radial pipe parameters");
        }
        let pipe = Self {
            radius: radius_m,
            density: density_kg_per_m3,
            viscosity: viscosity_pa_s,
            velocities: vec![0.; cells],
        };
        // Check derived coefficients without committing any state.
        let mut check = pipe.clone();
        check.step(1., 0.)?;
        Ok(pipe)
    }
    pub fn velocities_m_per_s(&self) -> &[f64] {
        &self.velocities
    }
    fn area(&self, i: usize) -> f64 {
        let dr = self.radius / self.velocities.len() as f64;
        std::f64::consts::PI * dr * dr * (2 * i + 1) as f64
    }
    /// Backward-Euler radial momentum/diffusion at a signed pressure gradient.
    /// # Errors
    /// Invalid time/forcing or derived overflow leaves the velocity history unchanged.
    pub fn step(
        &mut self,
        seconds: f64,
        pressure_gradient_pa_per_m: f64,
    ) -> Result<RadialPipeStep, &'static str> {
        if !seconds.is_finite() || seconds <= 0. || !pressure_gradient_pa_per_m.is_finite() {
            return Err("invalid radial pipe step");
        }
        let n = self.velocities.len();
        let dr = self.radius / n as f64;
        let conductance = |face: usize| {
            2. * std::f64::consts::PI
                * self.viscosity
                * face as f64
                * if face == n { 2. } else { 1. }
        };
        let mut diagonal = vec![0.; n];
        let mut upper = vec![0.; n];
        let mut rhs = vec![0.; n];
        let mut tangent = vec![0.; n];
        for i in 0..n {
            let area = self.area(i);
            let mass = self.density * area / seconds;
            let left = conductance(i);
            let right = conductance(i + 1);
            diagonal[i] = mass + left + right;
            upper[i] = if i + 1 < n { -right } else { 0. };
            rhs[i] = mass * self.velocities[i] + area * pressure_gradient_pa_per_m;
            tangent[i] = area;
            if i > 0 {
                let factor = -left / diagonal[i - 1];
                diagonal[i] -= factor * upper[i - 1];
                rhs[i] -= factor * rhs[i - 1];
                tangent[i] -= factor * tangent[i - 1];
            }
            if !diagonal[i].is_finite()
                || diagonal[i] <= 0.
                || !rhs[i].is_finite()
                || !tangent[i].is_finite()
            {
                return Err("radial pipe coefficient overflow");
            }
        }
        let mut velocity = vec![0.; n];
        let mut response = vec![0.; n];
        for i in (0..n).rev() {
            let next = if i + 1 < n { velocity[i + 1] } else { 0. };
            let next_response = if i + 1 < n { response[i + 1] } else { 0. };
            velocity[i] = (rhs[i] - upper[i] * next) / diagonal[i];
            response[i] = (tangent[i] - upper[i] * next_response) / diagonal[i];
        }
        let mut report = RadialPipeStep {
            flow_m3_per_s: 0.,
            flow_tangent: 0.,
            kinetic_energy_j_per_m: 0.,
            viscous_power_w_per_m: 0.,
        };
        for i in 0..n {
            let a = self.area(i);
            report.flow_m3_per_s += a * velocity[i];
            report.flow_tangent += a * response[i];
            report.kinetic_energy_j_per_m += 0.5 * self.density * a * velocity[i] * velocity[i];
            let next = if i + 1 < n { velocity[i + 1] } else { 0. };
            report.viscous_power_w_per_m += conductance(i + 1) * (velocity[i] - next).powi(2);
        }
        if !dr.is_finite()
            || dr <= 0.
            || velocity.iter().any(|x| !x.is_finite())
            || [
                report.flow_m3_per_s,
                report.flow_tangent,
                report.kinetic_energy_j_per_m,
                report.viscous_power_w_per_m,
            ]
            .iter()
            .any(|x| !x.is_finite())
            || report.flow_tangent <= 0.
        {
            return Err("radial pipe response overflow");
        }
        self.velocities = velocity;
        Ok(report)
    }
    /// Resolve pipe flow with a massless linear resistance in series.
    /// Drive is signed endpoint pressure difference; returns the pipe report and
    /// dQ/d(endpoint pressure difference), in m3/(Pa s). Pipe-profile tangent in
    /// the report retains its original pressure-gradient units.
    /// # Errors
    /// Invalid length/resistance, nonfinite forcing or overflow preserves history.
    pub fn step_with_series_resistance(
        &mut self,
        seconds: f64,
        pressure_difference_pa: f64,
        length_m: f64,
        resistance_pa_s_per_m3: f64,
    ) -> Result<(RadialPipeStep, f64), &'static str> {
        if !length_m.is_finite()
            || length_m <= 0.
            || !resistance_pa_s_per_m3.is_finite()
            || resistance_pa_s_per_m3 < 0.
            || !pressure_difference_pa.is_finite()
        {
            return Err("invalid radial series interface");
        }
        let mut free = self.clone();
        let response = free.step(seconds, 0.)?;
        // Solve for gradient directly: avoids subtracting two nearly equal
        // endpoint pressures when the interface dominates the pipe impedance.
        let denominator = length_m + response.flow_tangent * resistance_pa_s_per_m3;
        let gradient = (pressure_difference_pa - resistance_pa_s_per_m3 * response.flow_m3_per_s)
            / denominator;
        let tangent = response.flow_tangent / denominator;
        if !denominator.is_finite()
            || !gradient.is_finite()
            || !tangent.is_finite()
            || tangent <= 0.
        {
            return Err("radial series response overflow");
        }
        // All prior operations used copies; final step is itself atomic.
        let report = self.step(seconds, gradient)?;
        Ok((report, tangent))
    }
    /// Ideal forward check valve with a resolved radial profile and series resistance.
    /// Returns pipe state, endpoint flow tangent, and nonnegative valve reaction Pa.
    /// Closed flow is zero to linear-solve roundoff; interior velocity is not reset.
    /// This has no leaflet mechanics or leakage.
    /// # Errors
    /// Invalid interface/time/forcing or overflow preserves all history.
    pub fn step_with_ideal_valve(
        &mut self,
        seconds: f64,
        pressure_difference_pa: f64,
        length_m: f64,
        resistance_pa_s_per_m3: f64,
    ) -> Result<(RadialPipeStep, f64, f64), &'static str> {
        let mut candidate = self.clone();
        let (open, tangent) = candidate.step_with_series_resistance(
            seconds,
            pressure_difference_pa,
            length_m,
            resistance_pa_s_per_m3,
        )?;
        if open.flow_m3_per_s >= 0. {
            *self = candidate;
            return Ok((open, tangent, 0.));
        }
        let mut free = self.clone();
        let response = free.step(seconds, 0.)?;
        let gradient = -response.flow_m3_per_s / response.flow_tangent;
        // Zero-flux valve contributes a constraint pressure; massless series
        // loss is zero at zero flux. The pressure reaction opposes reverse flow.
        let reaction = length_m * gradient - pressure_difference_pa;
        if !gradient.is_finite() || !reaction.is_finite() || reaction < 0. {
            return Err("radial valve reaction overflow");
        }
        let closed = self.step(seconds, gradient)?;
        Ok((closed, 0., reaction))
    }
    /// Frequency-domain solution of the same radial diffusion discretization.
    /// Real cosine pressure-gradient amplitude; response uses exp(i*omega*t).
    /// No startup integration or modification of the current velocity history.
    /// # Errors
    /// Nonpositive frequency, invalid forcing or complex coefficient overflow.
    pub fn harmonic_response(
        &self,
        omega_rad_per_s: f64,
        gradient_amplitude_pa_per_m: f64,
    ) -> Result<(Vec<[f64; 2]>, [f64; 2]), &'static str> {
        if !omega_rad_per_s.is_finite()
            || omega_rad_per_s <= 0.
            || !gradient_amplitude_pa_per_m.is_finite()
        {
            return Err("invalid radial harmonic forcing");
        }
        let n = self.velocities.len();
        let conductance = |face: usize| {
            2. * std::f64::consts::PI
                * self.viscosity
                * face as f64
                * if face == n { 2. } else { 1. }
        };
        let mut diagonal = vec![[0.; 2]; n];
        let mut rhs = vec![[0.; 2]; n];
        let mut upper = vec![0.; n];
        let scale = |a: [f64; 2], x: f64| [a[0] * x, a[1] * x];
        let mul = |a: [f64; 2], b: [f64; 2]| [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]];
        let divide = |a: [f64; 2], b: [f64; 2]| {
            let norm = b[0].hypot(b[1]);
            let unit = [b[0] / norm, b[1] / norm];
            [
                (a[0] * unit[0] + a[1] * unit[1]) / norm,
                (a[1] * unit[0] - a[0] * unit[1]) / norm,
            ]
        };
        for i in 0..n {
            let area = self.area(i);
            let left = conductance(i);
            let right = conductance(i + 1);
            diagonal[i] = [left + right, omega_rad_per_s * self.density * area];
            rhs[i] = [area * gradient_amplitude_pa_per_m, 0.];
            upper[i] = if i + 1 < n { -right } else { 0. };
            if i > 0 {
                let factor = divide([-left, 0.], diagonal[i - 1]);
                let d = scale(factor, upper[i - 1]);
                let f = mul(factor, rhs[i - 1]);
                for k in 0..2 {
                    diagonal[i][k] -= d[k];
                    rhs[i][k] -= f[k];
                }
            }
            if diagonal[i].iter().chain(&rhs[i]).any(|x| !x.is_finite()) {
                return Err("radial harmonic coefficient overflow");
            }
        }
        let mut velocities = vec![[0.; 2]; n];
        let mut flow = [0.; 2];
        for i in (0..n).rev() {
            let next = if i + 1 < n {
                scale(velocities[i + 1], upper[i])
            } else {
                [0.; 2]
            };
            velocities[i] = divide([rhs[i][0] - next[0], rhs[i][1] - next[1]], diagonal[i]);
            for (f, v) in flow.iter_mut().zip(velocities[i]) {
                *f += self.area(i) * v;
            }
        }
        if flow
            .iter()
            .chain(velocities.iter().flatten())
            .any(|x| !x.is_finite())
        {
            return Err("radial harmonic response overflow");
        }
        Ok((velocities, flow))
    }
}

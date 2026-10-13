//! Positive tabulated mass opacity; log-bilinear interpolation in SI rho and T.
//! No built-in calibration, composition interpolation or extrapolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    GreyAbsorption,
    PlanckAbsorption,
    RosselandTotal,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub kind: Kind,
    density: Vec<f64>,
    temperature: Vec<f64>,
    log_density: Vec<f64>,
    log_temperature: Vec<f64>,
    log_opacity: Vec<f64>,
    maximum_temperature_slope: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Mass opacity in m²/kg.
    pub opacity: f64,
    pub dln_opacity_dln_density: f64,
    pub dln_opacity_dln_temperature: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidTable,
    BudgetExceeded,
    InvalidInput,
    OutsideDomain,
    NumericalOverflow,
}
fn axis(values: &[f64]) -> Result<Vec<f64>, Error> {
    if values.len() < 2
        || values.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || values.windows(2).any(|w| w[1] <= w[0])
    {
        return Err(Error::InvalidTable);
    }
    let logs: Vec<_> = values.iter().map(|v| v.ln()).collect();
    if logs.windows(2).any(|w| w[1] <= w[0]) {
        return Err(Error::InvalidTable);
    }
    Ok(logs)
}
fn location(axis: &[f64], logs: &[f64], value: f64) -> Result<(usize, f64), Error> {
    if !value.is_finite() || value <= 0.0 {
        return Err(Error::InvalidInput);
    }
    if value < axis[0] || value > axis[axis.len() - 1] {
        return Err(Error::OutsideDomain);
    }
    let i = axis
        .partition_point(|x| *x <= value)
        .saturating_sub(1)
        .min(axis.len() - 2);
    Ok((
        i,
        ((value.ln() - logs[i]) / (logs[i + 1] - logs[i])).clamp(0.0, 1.0),
    ))
}
impl Table {
    /// Global bound on |d ln opacity / d ln T| across all interpolation cells.
    #[must_use]
    pub fn maximum_temperature_slope(&self) -> f64 {
        self.maximum_temperature_slope
    }

    #[must_use]
    pub fn maximum_opacity(&self) -> f64 {
        self.log_opacity
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            .exp()
    }

    /// Density kg/m³ and temperature K axes; opacity m²/kg in density-major
    /// order: `values[density_index * temperatures.len() + temperature_index]`.
    /// At least two distinct logarithmic nodes per axis. Opacity must be positive.
    /// # Errors
    /// Invalid/nonmonotonic axes, shape, zero/negative/nonfinite opacity.
    pub fn new(
        kind: Kind,
        density: Vec<f64>,
        temperature: Vec<f64>,
        values: Vec<f64>,
    ) -> Result<Self, Error> {
        let log_density = axis(&density)?;
        let log_temperature = axis(&temperature)?;
        if density.len().checked_mul(temperature.len()) != Some(values.len())
            || values.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(Error::InvalidTable);
        }
        let log_opacity: Vec<_> = values.into_iter().map(f64::ln).collect();
        let maximum_temperature_slope = log_opacity
            .chunks(temperature.len())
            .flat_map(|row| row.windows(2).zip(log_temperature.windows(2)))
            .map(|(values, axis)| ((values[1] - values[0]) / (axis[1] - axis[0])).abs())
            .fold(0.0, f64::max);
        if !maximum_temperature_slope.is_finite() {
            return Err(Error::InvalidTable);
        }
        Ok(Self {
            kind,
            density,
            temperature,
            log_density,
            log_temperature,
            log_opacity,
            maximum_temperature_slope,
        })
    }
    /// Interpolate logarithmic opacity and its local logarithmic derivatives.
    /// At an interior node derivatives use the interval to its right; the last
    /// node uses the interval to its left. No outside-domain clamping occurs.
    /// # Errors
    /// Invalid query, outside tabulated domain or numerical overflow.
    pub fn at(&self, density: f64, temperature: f64) -> Result<State, Error> {
        let (row, density_weight) = location(&self.density, &self.log_density, density)?;
        let (column, temperature_weight) =
            location(&self.temperature, &self.log_temperature, temperature)?;
        let stride = self.temperature.len();
        let lower_left = self.log_opacity[row * stride + column];
        let upper_left = self.log_opacity[(row + 1) * stride + column];
        let lower_right = self.log_opacity[row * stride + column + 1];
        let upper_right = self.log_opacity[(row + 1) * stride + column + 1];
        let log = (1.0 - temperature_weight)
            * ((1.0 - density_weight) * lower_left + density_weight * upper_left)
            + temperature_weight
                * ((1.0 - density_weight) * lower_right + density_weight * upper_right);
        let state = State {
            opacity: log.exp(),
            dln_opacity_dln_density: ((1.0 - temperature_weight) * (upper_left - lower_left)
                + temperature_weight * (upper_right - lower_right))
                / (self.log_density[row + 1] - self.log_density[row]),
            dln_opacity_dln_temperature: ((1.0 - density_weight) * (lower_right - lower_left)
                + density_weight * (upper_right - upper_left))
                / (self.log_temperature[column + 1] - self.log_temperature[column]),
        };
        if ![
            state.opacity,
            state.dln_opacity_dln_density,
            state.dln_opacity_dln_temperature,
        ]
        .into_iter()
        .all(f64::is_finite)
            || state.opacity <= 0.0
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(state)
    }
}

impl Table {
    /// Read a rectangular SI grid with exact header
    /// `density_kg_m3,temperature_K,opacity_m2_kg`. Rows may be unordered.
    /// # Errors
    /// Invalid header/numbers, missing/duplicate nodes or row-budget exhaustion.
    pub fn from_csv(kind: Kind, text: &str, max_rows: usize) -> Result<Self, Error> {
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        if lines.next().map(str::trim) != Some("density_kg_m3,temperature_K,opacity_m2_kg") {
            return Err(Error::InvalidTable);
        }
        let mut rows = Vec::new();
        for line in lines {
            if rows.len() >= max_rows {
                return Err(Error::BudgetExceeded);
            }
            let values = line
                .split(',')
                .map(|v| v.trim().parse::<f64>().map_err(|_| Error::InvalidTable))
                .collect::<Result<Vec<_>, _>>()?;
            if values.len() != 3 || values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
                return Err(Error::InvalidTable);
            }
            rows.push([values[0], values[1], values[2]]);
        }
        let mut density: Vec<_> = rows.iter().map(|r| r[0]).collect();
        let mut temperature: Vec<_> = rows.iter().map(|r| r[1]).collect();
        density.sort_by(f64::total_cmp);
        density.dedup();
        temperature.sort_by(f64::total_cmp);
        temperature.dedup();
        let length = density
            .len()
            .checked_mul(temperature.len())
            .ok_or(Error::InvalidTable)?;
        if length != rows.len() {
            return Err(Error::InvalidTable);
        }
        let mut values = vec![0.0; length];
        for row in rows {
            let i = density
                .binary_search_by(|v| v.total_cmp(&row[0]))
                .map_err(|_| Error::InvalidTable)?;
            let j = temperature
                .binary_search_by(|v| v.total_cmp(&row[1]))
                .map_err(|_| Error::InvalidTable)?;
            let node = &mut values[i * temperature.len() + j];
            if *node != 0.0 {
                return Err(Error::InvalidTable);
            }
            *node = row[2];
        }
        Self::new(kind, density, temperature, values)
    }
}

/// Nonrelativistic thermal free-free absorption with a constant Gaunt factor.
/// Fully ionized, nondegenerate ions/electrons; no bound absorption or scattering.
/// The caller supplies the temperature domain and Gaunt factor; neither is fitted.
/// Source: McGill PHYS 642 notes, free-free opacity, Eq. (2.61) (cgs prefactor 3.7e8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreeFree {
    pub gaunt_factor: f64,
    pub min_temperature: f64,
    pub max_temperature: f64,
}
impl FreeFree {
    fn composition(
        self,
        density: f64,
        temperature: f64,
        species: &[crate::astrophysics_eos::Species],
    ) -> Result<(f64, f64), Error> {
        if ![
            density,
            temperature,
            self.gaunt_factor,
            self.min_temperature,
            self.max_temperature,
        ]
        .into_iter()
        .all(f64::is_finite)
            || density <= 0.0
            || self.gaunt_factor <= 0.0
            || self.min_temperature <= 0.0
            || self.max_temperature < self.min_temperature
        {
            return Err(Error::InvalidInput);
        }
        if temperature < self.min_temperature || temperature > self.max_temperature {
            return Err(Error::OutsideDomain);
        }
        let mixture =
            crate::astrophysics_eos::Mixture::new(species).map_err(|_| Error::InvalidInput)?;
        let total: f64 = species.iter().map(|s| s.mass_fraction).sum();
        let ionic_charge: f64 = species
            .iter()
            .map(|s| {
                s.mass_fraction / total * f64::from(s.nuclear_charge).powi(2)
                    / f64::from(s.mass_number)
            })
            .sum();
        Ok((1.0 / mixture.electron_molecular_weight(), ionic_charge))
    }
    /// Spectral mass absorption in m²/kg at frequency Hz, including stimulated
    /// emission. Uses alpha_nu=3.7e8*T^-1/2*ne*sum(Z²ni)*nu^-3*(1-exp(-hnu/kT))*g
    /// in cm^-1, with explicit SI/cgs conversion. Does not include scattering.
    pub fn spectral(
        self,
        density: f64,
        temperature: f64,
        frequency: f64,
        species: &[crate::astrophysics_eos::Species],
    ) -> Result<f64, Error> {
        let (electrons, ions) = self.composition(density, temperature, species)?;
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let x = 6.626_070_15e-34 * frequency / (crate::astrophysics_eos::BOLTZMANN * temperature);
        let stimulated = -(-x).exp_m1();
        // ne and ni in cm^-3, rho in g/cm³, then cm²/g -> m²/kg.
        let log_value = (3.7e8_f64).ln() + density.ln()
            - 2.0 * crate::astrophysics_eos::ATOMIC_MASS.ln()
            + (1e-10_f64).ln()
            + electrons.ln()
            + ions.ln()
            + self.gaunt_factor.ln()
            - 0.5 * temperature.ln()
            - 3.0 * frequency.ln()
            + stimulated.ln();
        let value = log_value.exp();
        if !value.is_finite() || value <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        Ok(value)
    }
    /// Planck-weighted absorption mean, m²/kg. Analytic frequency integration
    /// assumes the Gaunt factor is constant. This is not a Rosseland mean and
    /// must not be silently used as total extinction in an optically thick star.
    pub fn planck(
        self,
        density: f64,
        temperature: f64,
        species: &[crate::astrophysics_eos::Species],
    ) -> Result<f64, Error> {
        let (electrons, ions) = self.composition(density, temperature, species)?;
        // cgs C=3.7e8*2*pi*k_B/(c²*sigma); conversion of number densities,
        // mass density and opacity gives the additional 1e-10 factor.
        let coefficient =
            3.7e8 * 2.0 * std::f64::consts::PI * (crate::astrophysics_eos::BOLTZMANN * 1e7)
                / (crate::astrophysics_eos::LIGHT_SPEED * 100.0).powi(2)
                / (crate::astrophysics_thermal::STEFAN_BOLTZMANN * 1e3);
        let log_value = coefficient.ln() + (1e-10_f64).ln() + density.ln()
            - 2.0 * crate::astrophysics_eos::ATOMIC_MASS.ln()
            + electrons.ln()
            + ions.ln()
            + self.gaunt_factor.ln()
            - 3.5 * temperature.ln();
        let value = log_value.exp();
        if !value.is_finite() || value <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        Ok(value)
    }
    /// Tabulate the physical Planck mean for one fixed composition; no
    /// composition interpolation or extrapolation. Kind remains PlanckAbsorption.
    pub fn planck_table(
        self,
        density: Vec<f64>,
        temperature: Vec<f64>,
        species: &[crate::astrophysics_eos::Species],
    ) -> Result<Table, Error> {
        let mut values = Vec::new();
        for rho in &density {
            for t in &temperature {
                values.push(self.planck(*rho, *t, species)?);
            }
        }
        Table::new(Kind::PlanckAbsorption, density, temperature, values)
    }
}

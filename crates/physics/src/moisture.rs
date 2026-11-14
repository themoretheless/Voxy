//! Finite-capacity linear moisture diffusion network with open-boundary accounting.
mod cohesive;
pub use cohesive::{CohesiveCalibration, CohesiveProperties};
mod film;
mod material;
mod supply;
pub use film::FilmSupply;
pub use material::{Calibration, Properties};
pub use supply::{SupplyTransfer, WaterSupply};
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// Water mass at full saturation, kg. Requires material porosity/geometry.
    pub capacity_kg: f64,
    pub water_kg: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub cells: [usize; 2],
    /// Mass conductance per unit saturation difference, kg/s.
    pub conductance_kg_s: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Reservoir {
    pub cell: usize,
    /// Prescribed equilibrium saturation, not relative air humidity directly.
    pub saturation: f64,
    pub conductance_kg_s: f64,
}
#[derive(Clone, Debug)]
pub struct Body {
    cells: Vec<Cell>,
    links: Vec<Link>,
}
#[derive(Clone, Debug)]
pub struct Transfer {
    /// Signed external-to-material water transfer, kg, per reservoir in input order.
    pub reservoir_water_kg: Vec<f64>,
    pub mass_defect_kg: f64,
}
fn validate_cells(cells: &[Cell]) -> Result<(), &'static str> {
    if cells.is_empty()
        || cells.len() > 16_384
        || cells.iter().any(|c| {
            !c.capacity_kg.is_finite()
                || c.capacity_kg <= 0.
                || !c.water_kg.is_finite()
                || c.water_kg < 0.
                || c.water_kg > c.capacity_kg
        })
    {
        return Err("invalid moisture cells");
    }
    if !cells.iter().map(|c| c.capacity_kg).sum::<f64>().is_finite() {
        return Err("moisture capacity inventory overflow");
    }
    Ok(())
}
impl Body {
    /// Construct a bounded implicit moisture network (up to 16384 cells).
    /// Small networks use direct factorization; larger ones use sparse transport.
    /// # Errors
    /// Invalid inventories, indices, repeated/self links or conductances.
    pub fn new(cells: Vec<Cell>, links: Vec<Link>) -> Result<Self, &'static str> {
        validate_cells(&cells)?;
        if links.len() > 262_144 {
            return Err("moisture link budget exceeded");
        }
        let mut seen = std::collections::BTreeSet::new();
        for link in &links {
            let [a, b] = link.cells;
            if a == b
                || a >= cells.len()
                || b >= cells.len()
                || !link.conductance_kg_s.is_finite()
                || link.conductance_kg_s < 0.
                || !seen.insert([a.min(b), a.max(b)])
            {
                return Err("invalid moisture link");
            }
        }
        Ok(Self { cells, links })
    }
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
    /// Backward-Euler C*ds/dt = sum G*(s_neighbor-s) with optional fixed baths.
    /// Internal exchange conserves water; external changes are returned explicitly.
    /// Positive capacities/conductances give a discrete maximum principle. Bath
    /// inventory is external/infinite: the caller must budget finite supplied water.
    /// # Errors
    /// Invalid step/baths, singular/overflowing solve, bound or conservation failure.
    /// Accepted inventories remain unchanged on failure.
    pub fn advance(
        &mut self,
        dt_s: f64,
        reservoirs: &[Reservoir],
    ) -> Result<Transfer, &'static str> {
        let added = vec![0.; self.cells.len()];
        self.advance_with_sources(dt_s, reservoirs, &added)
    }
    fn advance_with_sources(
        &mut self,
        dt_s: f64,
        reservoirs: &[Reservoir],
        added: &[f64],
    ) -> Result<Transfer, &'static str> {
        if added.len() != self.cells.len() || added.iter().any(|x| !x.is_finite() || *x < 0.) {
            return Err("invalid moisture source inventory");
        }
        if !dt_s.is_finite()
            || dt_s <= 0.
            || reservoirs.iter().any(|r| {
                r.cell >= self.cells.len()
                    || !r.saturation.is_finite()
                    || !(0. ..=1.).contains(&r.saturation)
                    || !r.conductance_kg_s.is_finite()
                    || r.conductance_kg_s < 0.
            })
        {
            return Err("invalid moisture step");
        }
        // An exact fixed point has zero flux on every active edge and bath.
        // Preserve accepted inventories rather than introduce roundoff by solving
        // and reconstructing capacity * saturation at an unchanged state.
        let activity: Vec<_> = self
            .cells
            .iter()
            .map(|c| c.water_kg / c.capacity_kg)
            .collect();
        let stationary = added.iter().all(|&x| x == 0.)
            && self
                .links
                .iter()
                .all(|l| l.conductance_kg_s == 0. || activity[l.cells[0]] == activity[l.cells[1]])
            && reservoirs
                .iter()
                .all(|r| r.conductance_kg_s == 0. || activity[r.cell] == r.saturation);
        if stationary {
            // Invalid/unrepresentable operators still reject at equilibrium.
            let mut diagonal: Vec<_> = self.cells.iter().map(|c| c.capacity_kg).collect();
            for link in &self.links {
                let weight = dt_s * link.conductance_kg_s;
                for &i in &link.cells {
                    diagonal[i] += weight;
                }
            }
            for r in reservoirs {
                diagonal[r.cell] += dt_s * r.conductance_kg_s;
            }
            if diagonal.iter().any(|x| !x.is_finite()) {
                return Err("moisture equilibrium operator overflow");
            }
            return Ok(Transfer {
                reservoir_water_kg: vec![0.; reservoirs.len()],
                mass_defect_kg: 0.,
            });
        }
        let saturation = transport::saturations(self, dt_s, reservoirs, added)?;
        let mut candidate = self.cells.clone();
        for (cell, &s) in candidate.iter_mut().zip(&saturation) {
            if !s.is_finite() || !(-1e-12..=1. + 1e-12).contains(&s) {
                return Err("moisture saturation bound failure");
            }
            cell.water_kg = cell.capacity_kg * s.clamp(0., 1.);
        }
        let reservoir_water_kg: Vec<_> = reservoirs
            .iter()
            .map(|r| {
                dt_s * r.conductance_kg_s
                    * (r.saturation - candidate[r.cell].water_kg / candidate[r.cell].capacity_kg)
            })
            .collect();
        let change: f64 = candidate
            .iter()
            .zip(&self.cells)
            .map(|(a, b)| a.water_kg - b.water_kg)
            .sum();
        let external: f64 = reservoir_water_kg.iter().sum();
        let defect = change - external - added.iter().sum::<f64>();
        let scale = self.cells.iter().map(|c| c.capacity_kg).sum::<f64>();
        if !defect.is_finite()
            || reservoir_water_kg.iter().any(|x| !x.is_finite())
            || defect.abs() > 1e-10 * scale
        {
            return Err("moisture mass balance failure");
        }
        self.cells = candidate;
        Ok(Transfer {
            reservoir_water_kg,
            mass_defect_kg: defect,
        })
    }
}
fn solve(mut matrix: Vec<f64>, mut rhs: Vec<f64>) -> Result<Vec<f64>, &'static str> {
    let scale = matrix.iter().copied().fold(0_f64, |a, b| a.max(b.abs()));
    if !scale.is_finite() || scale <= 0. || matrix.iter().chain(&rhs).any(|x| !x.is_finite()) {
        return Err("moisture solve overflow");
    }
    for x in &mut matrix {
        *x /= scale;
    }
    for x in &mut rhs {
        *x /= scale;
    }
    let n = rhs.len();
    if !crate::math::cholesky_solve_in_place(&mut matrix, &mut rhs, n, 1e-14) {
        return Err("ill-conditioned moisture solve");
    }
    Ok(rhs)
}

/// Moisture inventory after volume removal and explicit transport-graph rebuild.
#[derive(Clone, Debug)]
pub struct MaterialPartition {
    /// None when no moisture-bearing material remains.
    pub remaining: Option<Body>,
    /// Old cell index to compacted remaining index; removed cells map to None.
    pub remap: Vec<Option<usize>>,
    pub removed_water_kg: Vec<f64>,
    pub removed_capacity_kg: Vec<f64>,
}
impl Body {
    /// Partition uniformly saturated cells by retained material-volume fraction.
    /// New conductances must be supplied for the changed geometry; links use old
    /// indices and are remapped here. No transport step occurs during partition.
    /// This avoids retaining positive-capacity network nodes after full wear.
    /// # Errors
    /// Invalid fractions, unrepresentable inventory changes, invalid new links
    /// or links touching exhausted cells. Original body is never mutated.
    pub fn partition_material(
        &self,
        retained: &[f64],
        geometry_links: &[Link],
    ) -> Result<MaterialPartition, &'static str> {
        if retained.len() != self.cells.len()
            || retained
                .iter()
                .any(|f| !f.is_finite() || !(0. ..=1.).contains(f))
        {
            return Err("invalid moisture retained volume fractions");
        }
        let mut cells = Vec::new();
        let mut remap = Vec::with_capacity(retained.len());
        let mut removed_water_kg = Vec::with_capacity(retained.len());
        let mut removed_capacity_kg = Vec::with_capacity(retained.len());
        for (old, &fraction) in self.cells.iter().zip(retained) {
            let cell = Cell {
                capacity_kg: old.capacity_kg * fraction,
                water_kg: old.water_kg * fraction,
            };
            let removed_water = old.water_kg - cell.water_kg;
            let removed_capacity = old.capacity_kg - cell.capacity_kg;
            if (fraction > 0. && cell.capacity_kg <= 0.)
                || (fraction > 0. && old.water_kg > 0. && cell.water_kg == 0.)
                || (fraction < 1. && removed_capacity <= 0.)
                || (fraction < 1. && old.water_kg > 0. && removed_water <= 0.)
            {
                return Err("unrepresentable moisture material partition");
            }
            removed_water_kg.push(removed_water);
            removed_capacity_kg.push(removed_capacity);
            if fraction == 0. {
                remap.push(None);
            } else {
                remap.push(Some(cells.len()));
                cells.push(cell);
            }
        }
        let links = geometry_links
            .iter()
            .map(|link| {
                let map = |i| {
                    remap
                        .get(i)
                        .copied()
                        .flatten()
                        .ok_or("moisture link touches missing material")
                };
                Ok(Link {
                    cells: [map(link.cells[0])?, map(link.cells[1])?],
                    conductance_kg_s: link.conductance_kg_s,
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let remaining = if cells.is_empty() {
            None
        } else {
            Some(Self::new(cells, links)?)
        };
        Ok(MaterialPartition {
            remaining,
            remap,
            removed_water_kg,
            removed_capacity_kg,
        })
    }
}

mod vapor;
pub use vapor::MaterialThermalStore;
pub use vapor::{ThermalVapor, ThermalVaporAccuracy, ThermalVaporStep};
pub use vapor::{VaporLink, VaporReservoir, VaporTransfer};
mod temperature;
pub use cohesive::ThermalCohesiveCalibration;
pub use temperature::ThermalCalibration;

mod transport;

/// Evaluates the van Genuchten unsaturated porous-media water retention curve
/// $\theta(\psi) = \theta_r + \frac{\theta_s - \theta_r}{\left[1 + (\alpha |\psi|)^n\right]^{1 - 1/n}}$.
#[must_use]
pub fn van_genuchten_soil_water_retention(
    suction_head_m: f64,
    theta_r: f64,
    theta_s: f64,
    alpha_m_inv: f64,
    n_exp: f64,
) -> Option<f64> {
    if !suction_head_m.is_finite()
        || !theta_r.is_finite()
        || !theta_s.is_finite()
        || !alpha_m_inv.is_finite()
        || !n_exp.is_finite()
        || theta_r < 0.0
        || theta_s <= theta_r
        || theta_s > 1.0
        || alpha_m_inv <= 0.0
        || n_exp <= 1.0
    {
        return None;
    }
    let psi = suction_head_m.max(0.0);
    let m = 1.0 - 1.0 / n_exp;
    let se = (1.0 + (alpha_m_inv * psi).powf(n_exp)).powf(-m);
    let theta = theta_r + (theta_s - theta_r) * se;
    theta.is_finite().then_some(theta.clamp(theta_r, theta_s))
}

#[cfg(test)]
mod van_genuchten_tests {
    use super::*;

    #[test]
    fn van_genuchten_retention_interpolates_saturated_and_residual_water_content() {
        let sat = van_genuchten_soil_water_retention(0.0, 0.05, 0.45, 2.0, 1.8).unwrap();
        let mid = van_genuchten_soil_water_retention(1.0, 0.05, 0.45, 2.0, 1.8).unwrap();
        let dry = van_genuchten_soil_water_retention(1000.0, 0.05, 0.45, 2.0, 1.8).unwrap();
        assert!((sat - 0.45).abs() < 1e-12);
        assert!(mid < sat && mid > dry);
        assert!((dry - 0.05).abs() < 0.01);
    }
}

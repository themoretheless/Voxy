//! Backward-Euler finite liquid inventory caps, solved inside the moisture network.
use super::{Body, Reservoir};
#[derive(Clone, Copy, Debug)]
pub struct WaterSupply {
    pub cell: usize,
    /// Remaining liquid inventory, kg. Liquid activity is one while available.
    pub water_kg: f64,
    pub conductance_kg_s: f64,
}
#[derive(Clone, Debug)]
pub struct SupplyTransfer {
    pub supplied_water_kg: Vec<f64>,
    pub mass_defect_kg: f64,
    pub solves: usize,
}
impl Body {
    /// Solve finite-liquid uptake with flux min(available,dt*G*(1-s_new)).
    /// A depleted source becomes a fixed mass contribution in the same implicit
    /// network rather than clamping the already-computed cell state. Binding
    /// inventory caps only reduce saturation, so the active set grows monotonically.
    /// This uses backward-Euler budget limiting, not an exact exhaustion time.
    /// # Errors
    /// Invalid supply/step, overflow, failed solve or balance. Both material and
    /// supplied inventories remain unchanged until the complete update succeeds.
    pub fn advance_with_supplies(
        &mut self,
        dt_s: f64,
        supplies: &mut [WaterSupply],
    ) -> Result<SupplyTransfer, &'static str> {
        if supplies.len() > 4096
            || supplies.iter().any(|r| {
                r.cell >= self.cells.len()
                    || !r.water_kg.is_finite()
                    || r.water_kg < 0.
                    || !r.conductance_kg_s.is_finite()
                    || r.conductance_kg_s < 0.
            })
        {
            return Err("invalid finite water supply");
        }
        let mut bound: Vec<_> = supplies.iter().map(|r| r.water_kg == 0.).collect();
        for solves in 1..=supplies.len() + 1 {
            let reservoirs: Vec<_> = supplies
                .iter()
                .zip(&bound)
                .map(|(r, &b)| Reservoir {
                    cell: r.cell,
                    saturation: 1.,
                    conductance_kg_s: if b { 0. } else { r.conductance_kg_s },
                })
                .collect();
            let mut added = vec![0.; self.cells.len()];
            for (r, &b) in supplies.iter().zip(&bound) {
                if b {
                    added[r.cell] += r.water_kg;
                }
            }
            let mut candidate = self.clone();
            let transfer = candidate.advance_with_sources(dt_s, &reservoirs, &added)?;
            let mut changed = false;
            for (index, r) in supplies.iter().enumerate() {
                if !bound[index] && transfer.reservoir_water_kg[index] > r.water_kg {
                    bound[index] = true;
                    changed = true;
                }
            }
            if changed {
                continue;
            }
            let delivered: Vec<_> = supplies
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    if bound[i] {
                        r.water_kg
                    } else {
                        transfer.reservoir_water_kg[i]
                    }
                })
                .collect();
            let mut remaining = supplies.to_vec();
            for (r, &mass) in remaining.iter_mut().zip(&delivered) {
                let original = r.water_kg;
                r.water_kg -= mass;
                if mass > 0. && r.water_kg >= original {
                    return Err("unrepresentable water supply decrement");
                }
                if !r.water_kg.is_finite() || r.water_kg < 0. || !mass.is_finite() || mass < 0. {
                    return Err("finite water supply balance failure");
                }
            }
            let change: f64 = candidate
                .cells
                .iter()
                .zip(&self.cells)
                .map(|(a, b)| a.water_kg - b.water_kg)
                .sum();
            let defect = change - delivered.iter().sum::<f64>();
            let capacity: f64 = self.cells.iter().map(|c| c.capacity_kg).sum();
            if !defect.is_finite() || defect.abs() > 1e-10 * capacity {
                return Err("finite uptake mass balance failure");
            }
            *self = candidate;
            supplies.copy_from_slice(&remaining);
            return Ok(SupplyTransfer {
                supplied_water_kg: delivered,
                mass_defect_kg: defect,
                solves,
            });
        }
        Err("finite water supply active set failed")
    }
}

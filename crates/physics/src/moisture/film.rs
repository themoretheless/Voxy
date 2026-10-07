//! Isothermal uptake from canonical surface-film species inventories.
use super::{Body, SupplyTransfer, WaterSupply};
use crate::surface_film::FilmMixture;

/// Explicit authored ownership, not proximity-based contact detection.
#[derive(Clone, Copy, Debug)]
pub struct FilmSupply {
    pub film_cell: usize,
    pub material_cell: usize,
    pub conductance_kg_s: f64,
}
impl Body {
    /// Absorb one explicitly selected water species from a finite film inventory.
    /// Uses the existing backward-Euler supply/network solve at unit liquid activity.
    /// Other species remain in the film. Each film cell may supply only one link;
    /// several distinct film cells may supply the same material cell.
    /// This is isothermal prescribed uptake, not a solution-activity/contact law.
    /// No sensible heat, momentum, swelling or mechanical update is implied.
    /// All material and film inventories publish together, or remain unchanged.
    pub fn advance_surface_film_supplies(
        &mut self,
        dt_s: f64,
        film: &mut FilmMixture,
        water_component: usize,
        links: &[FilmSupply],
    ) -> Result<SupplyTransfer, &'static str> {
        if water_component >= film.component_names().len() || links.len() > 4096 {
            return Err("invalid film water component or supply budget");
        }
        let mut owners = std::collections::BTreeSet::new();
        let mut supplies = Vec::with_capacity(links.len());
        for link in links {
            if !owners.insert(link.film_cell) {
                return Err("duplicate film supply owner");
            }
            let inventory = film
                .component_masses_kg()
                .get(link.film_cell)
                .ok_or("invalid film supply cell")?;
            supplies.push(WaterSupply {
                cell: link.material_cell,
                water_kg: inventory[water_component],
                conductance_kg_s: link.conductance_kg_s,
            });
        }
        let mut next = self.clone();
        let mut report = next.advance_with_supplies(dt_s, &mut supplies)?;
        let mut next_film = film.clone();
        let requests: Vec<_> = links
            .iter()
            .zip(&report.supplied_water_kg)
            .map(|(link, mass)| {
                let mut amounts = vec![0.; film.component_names().len()];
                amounts[water_component] = *mass;
                (link.film_cell, amounts)
            })
            .collect();
        next_film.withdraw_component_masses_batch(&requests)?;
        // Report actual canonical removal, not the requested transfer: subtracting
        // from a finite film inventory can differ by an ulp from the network flux.
        for (index, link) in links.iter().enumerate() {
            let before = film.component_masses_kg()[link.film_cell][water_component];
            let removed = before - next_film.component_masses_kg()[link.film_cell][water_component];
            let supplied = report.supplied_water_kg[index];
            if !removed.is_finite()
                || removed < 0.
                || (supplied > 0. && removed <= 0.)
                || (removed - supplied).abs() > 32. * f64::EPSILON * before.max(supplied)
            {
                return Err("film uptake decrement differs from supplied water");
            }
            report.supplied_water_kg[index] = removed;
        }
        let change: f64 = next
            .cells()
            .iter()
            .zip(self.cells())
            .map(|(new, old)| new.water_kg - old.water_kg)
            .sum();
        report.mass_defect_kg = change - report.supplied_water_kg.iter().sum::<f64>();
        let capacity: f64 = self.cells().iter().map(|c| c.capacity_kg).sum();
        if !report.mass_defect_kg.is_finite() || report.mass_defect_kg.abs() > 1e-10 * capacity {
            return Err("film uptake combined mass balance failure");
        }
        *self = next;
        *film = next_film;
        Ok(report)
    }
}

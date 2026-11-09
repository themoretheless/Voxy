//! Archard abrasive removal from a uniform finite surface layer, in SI units.
//! Caller supplies accepted normal load and relative tangential sliding distance.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    hardness_pa: f64,
    coefficient: f64,
}
impl Material {
    /// Empirical Archard coefficient and indentation hardness; no measured defaults.
    /// # Errors
    /// Nonpositive/nonfinite hardness or negative/nonfinite coefficient.
    pub fn new(hardness_pa: f64, coefficient: f64) -> Result<Self, &'static str> {
        if !hardness_pa.is_finite()
            || hardness_pa <= 0.
            || !coefficient.is_finite()
            || coefficient < 0.
        {
            return Err("invalid wear material");
        }
        Ok(Self {
            hardness_pa,
            coefficient,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Layer {
    area_m2: f64,
    initial_volume_m3: f64,
    removed_volume_m3: f64,
    density_kg_m3: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Removal {
    pub volume_m3: f64,
    pub mass_kg: f64,
    pub depth_m: f64,
    /// Distance processed with this material; stops exactly at layer exhaustion.
    pub consumed_sliding_distance_m: f64,
    /// Caller must process this distance against newly exposed material/geometry.
    pub remaining_sliding_distance_m: f64,
    pub exhausted: bool,
}
impl Layer {
    /// Uniform surface patch with finite available material. Area is fixed until
    /// the caller reconstructs geometry; this does not remesh a solid by itself.
    /// # Errors
    /// Nonpositive/nonfinite area/density, negative/nonfinite thickness or overflow.
    pub fn new(area_m2: f64, thickness_m: f64, density_kg_m3: f64) -> Result<Self, &'static str> {
        let volume = area_m2 * thickness_m;
        if !area_m2.is_finite()
            || area_m2 <= 0.
            || !thickness_m.is_finite()
            || thickness_m < 0.
            || !density_kg_m3.is_finite()
            || density_kg_m3 <= 0.
            || !volume.is_finite()
            || !(volume * density_kg_m3).is_finite()
            || (volume > 0. && volume * density_kg_m3 == 0.)
            || (thickness_m > 0. && volume == 0.)
        {
            return Err("invalid wear layer");
        }
        Ok(Self {
            area_m2,
            initial_volume_m3: volume,
            removed_volume_m3: 0.,
            density_kg_m3,
        })
    }
    #[must_use]
    pub fn thickness_m(&self) -> f64 {
        (self.initial_volume_m3 - self.removed_volume_m3) / self.area_m2
    }
    #[must_use]
    pub fn remaining_mass_kg(&self) -> f64 {
        (self.initial_volume_m3 - self.removed_volume_m3) * self.density_kg_m3
    }
    #[must_use]
    pub fn debris_mass_kg(&self) -> f64 {
        self.removed_volume_m3 * self.density_kg_m3
    }
    /// Apply dV=k*F*ds/H to this material, uniformly over the patch area.
    /// No load or no tangential sliding causes no wear. Removed volume/mass is
    /// returned as debris inventory, not destroyed. Remaining sliding distance
    /// at exhaustion must be re-evaluated with the exposed material. This law
    /// does not compute friction dissipation, heating or calibrated wear coefficients.
    /// # Errors
    /// Nonfinite/negative load/distance or unrepresentable removal. State is atomic.
    pub fn advance(
        &mut self,
        material: Material,
        normal_load_n: f64,
        sliding_distance_m: f64,
    ) -> Result<Removal, &'static str> {
        if !normal_load_n.is_finite()
            || normal_load_n < 0.
            || !sliding_distance_m.is_finite()
            || sliding_distance_m < 0.
        {
            return Err("invalid wear loading");
        }
        let available = self.initial_volume_m3 - self.removed_volume_m3;
        let mut report = Removal {
            volume_m3: 0.,
            mass_kg: 0.,
            depth_m: 0.,
            consumed_sliding_distance_m: sliding_distance_m,
            remaining_sliding_distance_m: 0.,
            exhausted: available == 0.,
        };
        if available == 0. {
            report.consumed_sliding_distance_m = 0.;
            report.remaining_sliding_distance_m = sliding_distance_m;
            return Ok(report);
        }
        if normal_load_n == 0. || sliding_distance_m == 0. || material.coefficient == 0. {
            return Ok(report);
        }
        let rate = material.coefficient * (normal_load_n / material.hardness_pa);
        if !rate.is_finite() || rate <= 0. {
            return Err("wear rate overflow or underflow");
        }
        let distance_to_exhaust = available / rate;
        if available > 0. && distance_to_exhaust == 0. {
            return Err("unrepresentable wear exhaustion distance");
        }
        let exhaust = sliding_distance_m >= distance_to_exhaust;
        let removed = if exhaust {
            available
        } else {
            rate * sliding_distance_m
        };
        if !removed.is_finite() || (available > 0. && removed == 0.) {
            return Err("wear removal overflow or underflow");
        }
        let next = if exhaust {
            self.initial_volume_m3
        } else {
            self.removed_volume_m3 + removed
        };
        if next > self.initial_volume_m3 || (removed > 0. && next <= self.removed_volume_m3) {
            return Err("unrepresentable wear increment");
        }
        let removed = next - self.removed_volume_m3;
        report.volume_m3 = removed;
        report.mass_kg = removed * self.density_kg_m3;
        report.depth_m = removed / self.area_m2;
        report.exhausted = next == self.initial_volume_m3;
        if exhaust {
            report.consumed_sliding_distance_m = distance_to_exhaust;
            report.remaining_sliding_distance_m = sliding_distance_m - distance_to_exhaust;
        }
        if !report.mass_kg.is_finite() || !report.depth_m.is_finite() {
            return Err("wear diagnostic overflow");
        }
        self.removed_volume_m3 = next;
        Ok(report)
    }
}

mod column;
mod dust;
mod fracture_dust;
pub use fracture_dust::{FractureDust, FractureDustSettings, fracture_dust};
mod suspension;
pub use column::{
    Column, ColumnRemoval, MassProperties, MovingInventory, RigidMotion, RigidRemoval, Stratum,
    WetRemoval, WetRigidRemoval,
};
pub use dust::{
    ContactDustRemoval, ContactDustSettings, DustFriction, DustRemoval, FrictionDustRemoval,
    LiquidHeatSink, SurfaceDustRemoval, SurfaceEnergy,
};
pub use suspension::{
    WearSliderInput, WearSliderStep, WearSuspension, WearSuspensionEnergy, WearSuspensionInput,
    WearSuspensionStep,
};

/// Evaluates cumulative linear fatigue damage $D = \sum_i \frac{n_i}{N_i}$ under the
/// Palmgren–Miner rule across stress-amplitude bins `(applied_cycles, failure_cycles)`.
///
/// # Errors
///
/// Rejects mismatched slice lengths or any bin with `failure_cycles == 0`.
pub fn miner_fatigue_damage(
    applied_cycles: &[u64],
    failure_cycles: &[u64],
) -> Result<f64, &'static str> {
    if applied_cycles.len() != failure_cycles.len() {
        return Err("Palmgren-Miner cycle slice length mismatch");
    }
    let mut damage = 0.0_f64;
    for (&n, &nf) in applied_cycles.iter().zip(failure_cycles.iter()) {
        if nf == 0 {
            return Err("zero fatigue endurance limit in Palmgren-Miner rule");
        }
        damage += (n as f64) / (nf as f64);
    }
    if !damage.is_finite() {
        return Err("fatigue damage overflow");
    }
    Ok(damage)
}

/// Evaluates the local Archard abrasive wear depth increment
/// $\Delta h = k \frac{P_n v_{\text{slide}} \Delta t}{H}$ in meters (`m`).
#[must_use]
pub fn archard_abrasive_wear_depth_m(
    normal_pressure_pa: f64,
    sliding_speed_m_s: f64,
    hardness_pa: f64,
    wear_coeff_k: f64,
    dt_s: f64,
) -> Option<f64> {
    if !normal_pressure_pa.is_finite()
        || !sliding_speed_m_s.is_finite()
        || !hardness_pa.is_finite()
        || !wear_coeff_k.is_finite()
        || !dt_s.is_finite()
        || normal_pressure_pa < 0.0
        || sliding_speed_m_s < 0.0
        || hardness_pa <= 0.0
        || wear_coeff_k < 0.0
        || dt_s < 0.0
    {
        return None;
    }
    let dh = wear_coeff_k * (normal_pressure_pa / hardness_pa) * sliding_speed_m_s * dt_s;
    dh.is_finite().then_some(dh)
}

#[cfg(test)]
mod miner_tests {
    use super::*;

    #[test]
    fn miner_fatigue_sums_cycle_fractions_and_rejects_zero_endurance() {
        let d = miner_fatigue_damage(&[500, 250], &[1000, 1000]).unwrap();
        assert!((d - 0.75).abs() < 1e-12);
        assert!(miner_fatigue_damage(&[10], &[0]).is_err());
        let dh = archard_abrasive_wear_depth_m(1e6, 2.0, 1e9, 1e-4, 0.5).unwrap();
        assert!((dh - 1e-7).abs() < 1e-16);
    }
}

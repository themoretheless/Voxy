use super::{Layer, Material, Removal};

/// A planar stratum, ordered from the exposed surface toward the substrate.
#[derive(Clone, Copy, Debug)]
pub struct Stratum {
    pub thickness_m: f64,
    pub density_kg_m3: f64,
    pub material: Material,
}

/// Finite planar geometry with a fixed footprint. Surface position is the
/// cumulative inward recession along the caller's fixed surface normal.
/// This is not an arbitrary mesh remesher or a voxel edit transaction.
#[derive(Clone, Debug)]
pub struct Column {
    layers: Vec<(Layer, Material)>,
    recession_m: f64,
    area_m2: f64,
}
#[derive(Clone, Debug)]
pub struct ColumnRemoval {
    /// Debris inventory by stratum; entries retain material identity by index.
    pub strata: Vec<(usize, Removal)>,
    pub recession_m: f64,
    pub remaining_sliding_distance_m: f64,
}
impl Column {
    /// # Errors
    /// Invalid geometry/material inventory, empty or more than 4096 strata.
    pub fn new(area_m2: f64, strata: &[Stratum]) -> Result<Self, &'static str> {
        if strata.is_empty() || strata.len() > 4096 {
            return Err("invalid wear stratum count");
        }
        let layers = strata
            .iter()
            .map(|s| {
                Ok((
                    Layer::new(area_m2, s.thickness_m, s.density_kg_m3)?,
                    s.material,
                ))
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let depth: f64 = strata.iter().map(|s| s.thickness_m).sum();
        let mass: f64 = layers.iter().map(|(l, _)| l.remaining_mass_kg()).sum();
        if !depth.is_finite() || !mass.is_finite() {
            return Err("wear column inventory overflow");
        }
        Ok(Self {
            layers,
            recession_m: 0.,
            area_m2,
        })
    }
    #[must_use]
    pub fn recession_m(&self) -> f64 {
        self.recession_m
    }
    #[must_use]
    pub fn remaining_mass_kg(&self) -> f64 {
        self.layers.iter().map(|(l, _)| l.remaining_mass_kg()).sum()
    }
    #[must_use]
    pub fn debris_mass_kg(&self) -> f64 {
        self.layers.iter().map(|(l, _)| l.debris_mass_kg()).sum()
    }
    /// Apply constant load over a sliding path, switching material when each
    /// stratum is exhausted. Geometry and all inventories commit together.
    /// A zero wear coefficient shields deeper strata while consuming the path.
    /// # Errors
    /// Invalid loading or unrepresentable removal/recession, with no mutation.
    pub fn advance(&mut self, load_n: f64, distance_m: f64) -> Result<ColumnRemoval, &'static str> {
        if !load_n.is_finite() || load_n < 0. || !distance_m.is_finite() || distance_m < 0. {
            return Err("invalid wear loading");
        }
        let mut next = self.clone();
        let mut remaining = distance_m;
        let mut strata = Vec::new();
        for (index, (layer, material)) in next.layers.iter_mut().enumerate() {
            if layer.thickness_m() == 0. {
                continue;
            }
            let removal = layer.advance(*material, load_n, remaining)?;
            let recession = next.recession_m + removal.depth_m;
            if !recession.is_finite() || (removal.depth_m > 0. && recession <= next.recession_m) {
                return Err("unrepresentable wear surface recession");
            }
            next.recession_m = recession;
            remaining = removal.remaining_sliding_distance_m;
            strata.push((index, removal));
            if remaining == 0. {
                break;
            }
        }
        let report = ColumnRemoval {
            strata,
            recession_m: next.recession_m,
            remaining_sliding_distance_m: remaining,
        };
        *self = next;
        Ok(report)
    }
}

/// Exact mass properties of a rectangular column in its local axes.
/// z points inward from the original surface, x/y center on the footprint.
#[derive(Clone, Copy, Debug)]
pub struct MassProperties {
    pub mass_kg: f64,
    pub center_depth_m: f64,
    /// Diagonal inertia about the center of mass, in kg m² (xy, xz, yz vanish).
    pub inertia_kg_m2: [f64; 3],
}
impl Column {
    /// Remaining rectangular geometry, including density changes at strata.
    /// Empty columns return None. Width and area determine the second dimension.
    /// # Errors
    /// Invalid dimensions or unrepresentable mass properties.
    pub fn mass_properties(&self, width_m: f64) -> Result<Option<MassProperties>, &'static str> {
        if !width_m.is_finite() || width_m <= 0. {
            return Err("invalid wear footprint width");
        }
        let height = self.area_m2 / width_m;
        if !height.is_finite() || height <= 0. {
            return Err("unrepresentable wear footprint height");
        }
        let mass = self.remaining_mass_kg();
        if mass == 0. {
            return Ok(None);
        }
        // Reference the current surface to reduce cancellation after recession.
        let mut depth = 0.;
        let mut moment = 0.;
        for (layer, _) in &self.layers {
            let t = layer.thickness_m();
            moment += layer.remaining_mass_kg() * (depth + 0.5 * t);
            depth += t;
        }
        let center = moment / mass;
        let mut inertia = [0.; 3];
        depth = 0.;
        for (layer, _) in &self.layers {
            let t = layer.thickness_m();
            let m = layer.remaining_mass_kg();
            let offset = depth + 0.5 * t - center;
            inertia[0] += m * ((height * height + t * t) / 12. + offset * offset);
            inertia[1] += m * ((width_m * width_m + t * t) / 12. + offset * offset);
            inertia[2] += m * (width_m * width_m + height * height) / 12.;
            depth += t;
        }
        let center_depth_m = self.recession_m + center;
        if !center_depth_m.is_finite() || !inertia.iter().all(|v| v.is_finite() && *v > 0.) {
            return Err("unrepresentable wear mass properties");
        }
        Ok(Some(MassProperties {
            mass_kg: mass,
            center_depth_m,
            inertia_kg_m2: inertia,
        }))
    }
}

/// Rigid motion in column-local axes; velocity is measured at the original
/// footprint center (z=0). No ejection impulse is assumed.
#[derive(Clone, Copy, Debug)]
pub struct RigidMotion {
    pub surface_velocity_m_s: [f64; 3],
    pub angular_velocity_rad_s: [f64; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct MovingInventory {
    pub geometry: MassProperties,
    pub center_velocity_m_s: [f64; 3],
    pub momentum_kg_m_s: [f64; 3],
    /// About the original footprint center, in local axes.
    pub angular_momentum_kg_m2_s: [f64; 3],
    pub kinetic_j: f64,
}
#[derive(Clone, Debug)]
pub struct RigidRemoval {
    pub wear: ColumnRemoval,
    pub debris: Vec<(usize, MovingInventory)>,
    pub remaining: Option<MovingInventory>,
}
impl MassProperties {
    /// # Errors
    /// Nonfinite motion or overflowing momenta/energy.
    pub fn moving(self, motion: RigidMotion) -> Result<MovingInventory, &'static str> {
        if !self.mass_kg.is_finite()
            || self.mass_kg <= 0.
            || !self.center_depth_m.is_finite()
            || !self.inertia_kg_m2.iter().all(|x| x.is_finite() && *x > 0.)
        {
            return Err("invalid wear mass properties");
        }
        let v = motion.surface_velocity_m_s;
        let w = motion.angular_velocity_rad_s;
        if !v.iter().chain(w.iter()).all(|x| x.is_finite()) {
            return Err("invalid wear rigid motion");
        }
        let z = self.center_depth_m;
        let center_velocity_m_s = [v[0] + w[1] * z, v[1] - w[0] * z, v[2]];
        let momentum = center_velocity_m_s.map(|x| self.mass_kg * x);
        let angular = [
            self.inertia_kg_m2[0] * w[0] - z * momentum[1],
            self.inertia_kg_m2[1] * w[1] + z * momentum[0],
            self.inertia_kg_m2[2] * w[2],
        ];
        let kinetic_j = 0.5
            * (0..3)
                .map(|i| {
                    self.mass_kg * center_velocity_m_s[i].powi(2)
                        + self.inertia_kg_m2[i] * w[i].powi(2)
                })
                .sum::<f64>();
        if !momentum
            .iter()
            .chain(angular.iter())
            .chain(center_velocity_m_s.iter())
            .all(|x| x.is_finite())
            || !kinetic_j.is_finite()
        {
            return Err("wear rigid inventory overflow");
        }
        Ok(MovingInventory {
            geometry: self,
            center_velocity_m_s,
            momentum_kg_m_s: momentum,
            angular_momentum_kg_m2_s: angular,
            kinetic_j,
        })
    }
}
impl Column {
    /// Remove planar slabs with inherited rigid velocity and spin. Every slab
    /// retains its centroidal inertia (not a point-particle approximation).
    /// No friction work, heat or fragmentation/ejection impulse is generated.
    /// # Errors
    /// Invalid geometry/loading/motion or unrepresentable inventories; atomic.
    pub fn advance_rigid(
        &mut self,
        width_m: f64,
        load_n: f64,
        distance_m: f64,
        motion: RigidMotion,
    ) -> Result<RigidRemoval, &'static str> {
        // Validate even when an exhausted column has no moving inventory.
        if !motion
            .surface_velocity_m_s
            .iter()
            .chain(motion.angular_velocity_rad_s.iter())
            .all(|x| x.is_finite())
        {
            return Err("invalid wear rigid motion");
        }
        self.mass_properties(width_m)?;
        let height = self.area_m2 / width_m;
        let mut next = self.clone();
        let wear = next.advance(load_n, distance_m)?;
        let mut front = self.recession_m;
        let mut debris = Vec::new();
        for (index, removed) in &wear.strata {
            let t = removed.depth_m;
            let m = removed.mass_kg;
            if m > 0. {
                let geometry = MassProperties {
                    mass_kg: m,
                    center_depth_m: front + 0.5 * t,
                    inertia_kg_m2: [
                        m * (height * height + t * t) / 12.,
                        m * (width_m * width_m + t * t) / 12.,
                        m * (width_m * width_m + height * height) / 12.,
                    ],
                };
                if !geometry
                    .inertia_kg_m2
                    .iter()
                    .all(|x| x.is_finite() && *x > 0.)
                {
                    return Err("unrepresentable wear debris inertia");
                }
                debris.push((*index, geometry.moving(motion)?));
            }
            front += t;
        }
        let remaining = next
            .mass_properties(width_m)?
            .map(|p| p.moving(motion))
            .transpose()?;
        *self = next;
        Ok(RigidRemoval {
            wear,
            debris,
            remaining,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WetRemoval {
    /// Mass in this wear report is dry skeleton mass; water is separate below.
    pub wear: ColumnRemoval,
    /// Removed water mass per original stratum index, kg.
    pub debris_water_kg: Vec<f64>,
}
impl Column {
    /// Wear uniformly saturated strata, carrying pore water out with removed
    /// volume. Calibration selects each exposed layer's current wear properties.
    /// Remaining pore capacity shrinks with geometry; saturation stays constant
    /// during this removal-only operation. Exhausted strata have zero capacity
    /// and water and must be removed from a diffusion network before its next solve.
    /// # Errors
    /// Invalid inventories/calibrations or unrepresentable mass changes; column
    /// geometry and caller's water inventories commit together only on success.
    pub fn advance_wet(
        &mut self,
        water: &mut [crate::moisture::Cell],
        calibration: &[crate::moisture::Calibration],
        load_n: f64,
        distance_m: f64,
    ) -> Result<WetRemoval, &'static str> {
        if water.len() != self.layers.len() || calibration.len() != self.layers.len() {
            return Err("wear moisture stratum count mismatch");
        }
        let mut next = self.clone();
        for (i, (layer, material)) in next.layers.iter_mut().enumerate() {
            if layer.thickness_m() == 0. {
                if water[i].capacity_kg != 0. || water[i].water_kg != 0. {
                    return Err("water attached to exhausted wear stratum");
                }
            } else {
                *material = water[i].properties(calibration[i])?.wear_material()?;
            }
        }
        let wear = next.advance(load_n, distance_m)?;
        let mut next_water = water.to_vec();
        let mut debris_water_kg = vec![0.; water.len()];
        for (i, removal) in &wear.strata {
            if removal.volume_m3 == 0. {
                continue;
            }
            let old = water[*i];
            let layer = &self.layers[*i].0;
            let volume = layer.initial_volume_m3 - layer.removed_volume_m3;
            let retained = if removal.exhausted {
                0.
            } else {
                (volume - removal.volume_m3) / volume
            };
            let remaining = crate::moisture::Cell {
                capacity_kg: old.capacity_kg * retained,
                water_kg: old.water_kg * retained,
            };
            let removed_water = old.water_kg - remaining.water_kg;
            if !retained.is_finite()
                || !(0. ..=1.).contains(&retained)
                || (!removal.exhausted
                    && (remaining.capacity_kg <= 0. || remaining.capacity_kg >= old.capacity_kg))
                || (!removal.exhausted && old.water_kg > 0. && remaining.water_kg == 0.)
                || (old.water_kg > 0. && removed_water <= 0.)
            {
                return Err("unrepresentable wet wear partition");
            }
            next_water[*i] = remaining;
            debris_water_kg[*i] = removed_water;
        }
        *self = next;
        water.copy_from_slice(&next_water);
        Ok(WetRemoval {
            wear,
            debris_water_kg,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WetRigidRemoval {
    pub wet: WetRemoval,
    /// Inventories include both skeleton and entrained pore water mass.
    pub debris: Vec<(usize, MovingInventory)>,
    pub remaining: Option<MovingInventory>,
}
impl Column {
    /// Exact mass properties assuming pore water is uniform within each stratum
    /// and moves with the skeleton (no relative pore-fluid flow).
    /// # Errors
    /// Invalid water inventory, dimensions or overflowing wet density/inertia.
    pub fn wet_mass_properties(
        &self,
        width_m: f64,
        water: &[crate::moisture::Cell],
    ) -> Result<Option<MassProperties>, &'static str> {
        if water.len() != self.layers.len() {
            return Err("wear moisture stratum count mismatch");
        }
        let mut wet = self.clone();
        for ((layer, _), cell) in wet.layers.iter_mut().zip(water) {
            let volume = layer.initial_volume_m3 - layer.removed_volume_m3;
            if volume == 0. {
                if cell.capacity_kg != 0. || cell.water_kg != 0. {
                    return Err("water attached to exhausted wear stratum");
                }
            } else {
                cell.saturation()?;
                layer.density_kg_m3 += cell.water_kg / volume;
                if !layer.density_kg_m3.is_finite() {
                    return Err("wet wear density overflow");
                }
            }
        }
        wet.mass_properties(width_m)
    }
    /// Joint removal, pore-water partition and rigid mechanical inventory.
    /// No ejection impulse; water is entrained with the solid at separation.
    /// # Errors
    /// Invalid inputs or nonrepresentable balances; both caller inventories
    /// remain unchanged on any failure, including late mechanical overflow.
    pub fn advance_wet_rigid(
        &mut self,
        width_m: f64,
        water: &mut [crate::moisture::Cell],
        calibration: &[crate::moisture::Calibration],
        load_n: f64,
        distance_m: f64,
        motion: RigidMotion,
    ) -> Result<WetRigidRemoval, &'static str> {
        if !motion
            .surface_velocity_m_s
            .iter()
            .chain(motion.angular_velocity_rad_s.iter())
            .all(|x| x.is_finite())
        {
            return Err("invalid wear rigid motion");
        }
        self.wet_mass_properties(width_m, water)?;
        let mut next = self.clone();
        let mut next_water = water.to_vec();
        let wet = next.advance_wet(&mut next_water, calibration, load_n, distance_m)?;
        let height = self.area_m2 / width_m;
        let mut front = self.recession_m;
        let mut debris = Vec::new();
        for (index, removed) in &wet.wear.strata {
            let t = removed.depth_m;
            let m = removed.mass_kg + wet.debris_water_kg[*index];
            if m > 0. {
                let geometry = MassProperties {
                    mass_kg: m,
                    center_depth_m: front + 0.5 * t,
                    inertia_kg_m2: [
                        m * (height * height + t * t) / 12.,
                        m * (width_m * width_m + t * t) / 12.,
                        m * (width_m * width_m + height * height) / 12.,
                    ],
                };
                if !m.is_finite()
                    || !geometry
                        .inertia_kg_m2
                        .iter()
                        .all(|x| x.is_finite() && *x > 0.)
                {
                    return Err("unrepresentable wet debris inertia");
                }
                debris.push((*index, geometry.moving(motion)?));
            }
            front += t;
        }
        let remaining = next
            .wet_mass_properties(width_m, &next_water)?
            .map(|p| p.moving(motion))
            .transpose()?;
        *self = next;
        water.copy_from_slice(&next_water);
        Ok(WetRigidRemoval {
            wet,
            debris,
            remaining,
        })
    }
}

impl Column {
    /// Coupled wear and moisture graph update. `cell_strata` maps each current
    /// network cell to the original stratum. New links use current cell indices
    /// and must describe conductances after geometry changes. Exhausted cells
    /// disappear; surviving cells keep stable stratum identity through the map.
    /// # Errors
    /// Invalid mapping, inventory, loading or changed transport graph. Column,
    /// network and index map commit together only on complete success.
    pub fn advance_wet_network(
        &mut self,
        network: &mut Option<crate::moisture::Body>,
        cell_strata: &mut Vec<usize>,
        calibration: &[crate::moisture::Calibration],
        load_n: f64,
        distance_m: f64,
        geometry_links: &[crate::moisture::Link],
    ) -> Result<WetRemoval, &'static str> {
        let cells = network.as_ref().map_or(&[][..], |b| b.cells());
        if cell_strata.len() != cells.len() {
            return Err("wear network mapping count mismatch");
        }
        let mut water = vec![
            crate::moisture::Cell {
                capacity_kg: 0.,
                water_kg: 0.
            };
            self.layers.len()
        ];
        let mut seen = std::collections::BTreeSet::new();
        for (&stratum, cell) in cell_strata.iter().zip(cells) {
            if stratum >= self.layers.len()
                || !seen.insert(stratum)
                || self.layers[stratum].0.thickness_m() == 0.
            {
                return Err("invalid wear network stratum mapping");
            }
            water[stratum] = *cell;
        }
        if self
            .layers
            .iter()
            .enumerate()
            .any(|(i, (layer, _))| layer.thickness_m() > 0. && !seen.contains(&i))
        {
            return Err("wear network omits surviving stratum");
        }
        let mut next = self.clone();
        let removal = next.advance_wet(&mut water, calibration, load_n, distance_m)?;
        let (remaining, map) = if let Some(body) = network.as_ref() {
            let retained: Vec<_> = cell_strata
                .iter()
                .zip(cells)
                .map(|(&i, c)| water[i].capacity_kg / c.capacity_kg)
                .collect();
            let partition = body.partition_material(&retained, geometry_links)?;
            let map: Vec<usize> = cell_strata
                .iter()
                .zip(&partition.remap)
                .filter_map(|(&s, index)| index.map(|_| s))
                .collect();
            if let Some(body) = &partition.remaining {
                for (cell, &stratum) in body.cells().iter().zip(&map) {
                    let expected = water[stratum].water_kg;
                    if (cell.water_kg - expected).abs() > 1e-12 * water[stratum].capacity_kg {
                        return Err("wet wear network inventory mismatch");
                    }
                }
            }
            (partition.remaining, map)
        } else {
            if !geometry_links.is_empty() {
                return Err("links supplied for empty wear network");
            }
            (None, Vec::new())
        };
        *self = next;
        *network = remaining;
        *cell_strata = map;
        Ok(removal)
    }
}

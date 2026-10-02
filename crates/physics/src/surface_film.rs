//! Conservative finite-volume thin-film prototype on triangle cells, in SI units.
//! Closed mesh edges retain fluid. No evaporation or two-way tissue coupling.
use std::collections::BTreeMap;
#[path = "surface_film_rheology.rs"]
mod rheology;
pub use rheology::{FilmRheology, SlidingFilmBalance};
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub density: f64,
    pub viscosity: f64,
    pub surface_tension: f64,
    /// Effective wetting diffusivity (m²/s); phenomenological, not a contact angle.
    pub wetting: f64,
}
impl Material {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.density.is_finite()
            || self.density <= 0.
            || !self.viscosity.is_finite()
            || self.viscosity <= 0.
            || !self.surface_tension.is_finite()
            || self.surface_tension < 0.
            || !self.wetting.is_finite()
            || self.wetting < 0.
        {
            return Err("invalid film material");
        }
        Ok(())
    }
}
impl Default for Material {
    fn default() -> Self {
        Self {
            density: 1000.,
            viscosity: 0.05,
            surface_tension: 0.04,
            wetting: 1e-7,
        }
    }
}
/// Small-angle precursor-film wetting potential; no biological values are implied.
#[derive(Clone, Copy, Debug)]
pub struct Wetting {
    /// Equilibrium angle in radians, restricted to the long-wave regime (0..0.5).
    pub contact_angle: f64,
    pub precursor_thickness: f64,
}
impl Wetting {
    fn validate(self) -> Result<(), &'static str> {
        if !self.contact_angle.is_finite()
            || !(0. ..=0.5).contains(&self.contact_angle)
            || !self.precursor_thickness.is_finite()
            || self.precursor_thickness <= 0.
        {
            return Err("invalid precursor wetting parameters");
        }
        Ok(())
    }
    /// Derivative of -A/(2h²)+A hp³/(5h⁵), regularized at hp/4.
    pub fn pressure(self, height: f64, surface_tension: f64) -> Result<f64, &'static str> {
        self.validate()?;
        if !height.is_finite()
            || height < 0.
            || !surface_tension.is_finite()
            || surface_tension < 0.
        {
            return Err("invalid wetting pressure input");
        }
        let h = height.max(self.precursor_thickness * 0.25);
        let ratio = self.precursor_thickness / h;
        let a = (5. / 3.)
            * surface_tension
            * self.contact_angle
            * self.contact_angle
            * self.precursor_thickness
            * self.precursor_thickness;
        let pressure = a / (h * h * h) * (1. - ratio.powi(3));
        if !pressure.is_finite() {
            return Err("wetting pressure overflow");
        }
        Ok(pressure)
    }
}
/// Bridge transfer is phenomenological; the geometric detection is independent.
#[derive(Clone, Copy, Debug)]
pub struct BridgeConfig {
    pub max_gap: f64,
    pub transfer_speed: f64,
    /// Reject normals that are not opposed by at least this cosine (0..1).
    pub opposing_cosine: f64,
    pub max_candidates: usize,
}
impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            max_gap: 0.001,
            transfer_speed: 0.0001,
            opposing_cosine: 0.25,
            max_candidates: 100000,
        }
    }
}
#[derive(Debug)]
pub struct SurfaceFilm {
    triangles: Vec<[usize; 3]>,
    area: Vec<f64>,
    center: Vec<[f64; 3]>,
    edges: Vec<(usize, usize, f64, f64)>,
    edge_conormals: Vec<[f64; 3]>,
    self_contact_index: std::sync::OnceLock<crate::surface_film_contact::ProximityIndex>,
    volume: Vec<f64>,
    material: Material,
    wetting_model: Option<Wetting>,
    geometry: Vec<[[f64; 3]; 3]>,
    normals: Vec<[f64; 3]>,
    substrate_curvature: Vec<f64>,
}
/// Portable numerical film state, in SI units. Geometry-derived operators and
/// proximity caches are rebuilt on restore. Unused vertex slots have zero
/// coordinates: only positions referenced by cells belong to film state.
#[derive(Clone, Debug)]
pub struct SurfaceFilmState {
    pub points: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub cell_volumes_m3: Vec<f64>,
    pub material: Material,
    pub wetting: Option<Wetting>,
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
impl SurfaceFilm {
    /// Capture the current substrate geometry and exact per-cell volumes.
    /// This is a numerical checkpoint, not a renderer or whole-body snapshot.
    pub fn state(&self) -> SurfaceFilmState {
        let count = self
            .triangles
            .iter()
            .flatten()
            .copied()
            .max()
            .map_or(0, |i| i + 1);
        let mut points = vec![[0.; 3]; count];
        for (triangle, geometry) in self.triangles.iter().zip(&self.geometry) {
            for k in 0..3 {
                points[triangle[k]] = geometry[k];
            }
        }
        SurfaceFilmState {
            points,
            triangles: self.triangles.clone(),
            cell_volumes_m3: self.volume.clone(),
            material: self.material,
            wetting: self.wetting_model,
        }
    }
    /// Restore only after validating material, geometry, wetting and all volumes.
    pub fn from_state(state: &SurfaceFilmState) -> Result<Self, &'static str> {
        if state.cell_volumes_m3.len() != state.triangles.len() {
            return Err("film checkpoint cell count mismatch");
        }
        let mut film = Self::new(&state.points, state.triangles.clone(), state.material)?;
        film.set_wetting(state.wetting)?;
        let deposits: Vec<_> = state.cell_volumes_m3.iter().copied().enumerate().collect();
        film.deposit_batch(&deposits)?;
        Ok(film)
    }
    pub fn new(
        points: &[[f64; 3]],
        triangles: Vec<[usize; 3]>,
        material: Material,
    ) -> Result<Self, &'static str> {
        if triangles.is_empty()
            || !material.density.is_finite()
            || material.density <= 0.
            || !material.viscosity.is_finite()
            || material.viscosity <= 0.
            || !material.surface_tension.is_finite()
            || material.surface_tension < 0.
            || !material.wetting.is_finite()
            || material.wetting < 0.
        {
            return Err("invalid film material or mesh");
        }
        let mut film = Self {
            volume: vec![0.; triangles.len()],
            triangles,
            area: Vec::new(),
            center: Vec::new(),
            edges: Vec::new(),
            edge_conormals: Vec::new(),
            self_contact_index: std::sync::OnceLock::new(),
            material,
            wetting_model: None,
            geometry: Vec::new(),
            normals: Vec::new(),
            substrate_curvature: Vec::new(),
        };
        film.update_geometry(points)?;
        Ok(film)
    }
    /// Conservatively redistribute existing cell volumes onto a new surface.
    /// Each donor has explicit `(recipient, fraction)` entries summing to one.
    /// Empty rows are allowed only for dry donors. Correspondence must be supplied
    /// by the caller: this operation does not infer anatomical or contact mapping.
    /// The original film is unchanged on both success and failure; no precursor
    /// is newly seeded, and material/wetting settings are retained.
    pub fn remapped(
        &self,
        points: &[[f64; 3]],
        triangles: Vec<[usize; 3]>,
        distribution: &[Vec<(usize, f64)>],
    ) -> Result<Self, &'static str> {
        if distribution.len() != self.volume.len() {
            return Err("film remap must cover every donor cell");
        }
        let mut result = Self::new(points, triangles, self.material)?;
        for (donor, row) in distribution.iter().enumerate() {
            if row.is_empty() {
                if self.volume[donor] != 0. {
                    return Err("wet donor has no remap recipient");
                }
                continue;
            }
            if row
                .iter()
                .any(|&(cell, w)| cell >= result.volume.len() || !w.is_finite() || w < 0.)
            {
                return Err("invalid film remap recipient or weight");
            }
            let sum: f64 = row.iter().map(|&(_, w)| w).sum();
            if !sum.is_finite() || (sum - 1.).abs() > 1e-12 {
                return Err("film remap fractions must sum to one");
            }
            for &(cell, w) in row {
                result.volume[cell] += self.volume[donor] * (w / sum);
            }
        }
        let before = self.total_volume();
        let after = result.total_volume();
        if result.volume.iter().any(|v| !v.is_finite() || *v < 0.)
            || !after.is_finite()
            || (after - before).abs() > 1e-10 * before.max(f64::MIN_POSITIVE)
        {
            return Err("film remap failed conservation check");
        }
        result.wetting_model = self.wetting_model;
        Ok(result)
    }
    /// Atomically update substrate geometry, add sources and advance transport.
    /// Returns supplied volume. Numerical/input failure preserves all old state.
    /// The self-contact index is retained and refitted only after success.
    pub fn advance_on_geometry(
        &mut self,
        points: &[[f64; 3]],
        dt: f64,
        sources: &[(usize, f64)],
        gravity: [f64; 3],
    ) -> Result<f64, &'static str> {
        self.advance_on_geometry_with_contact(points, dt, sources, gravity, None)
            .map(|(added, _)| added)
    }
    /// Commit geometry, sources, transport and optional self-contact together.
    /// Any failed stage preserves the original numerical state and contact cache.
    pub fn advance_on_geometry_with_contact(
        &mut self,
        points: &[[f64; 3]],
        dt: f64,
        sources: &[(usize, f64)],
        gravity: [f64; 3],
        contact: Option<BridgeConfig>,
    ) -> Result<(f64, f64), &'static str> {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return Err("invalid film frame timestep");
        }
        let mut next = Self::new(points, self.triangles.clone(), self.material)?;
        next.volume = self.volume.clone();
        next.wetting_model = self.wetting_model;
        let added = if sources.is_empty() {
            0.
        } else {
            next.add_sources(dt, sources)?
        };
        next.step(dt, gravity)?;
        if contact.is_some() {
            if let Some(index) = self.self_contact_index.get() {
                let mut staged = index.clone();
                staged.refit(&next.geometry);
                let _ = next.self_contact_index.set(staged);
            }
        }
        let transferred = if let Some(config) = contact {
            next.exchange_self_contact(dt, config)?
        } else {
            0.
        };
        if contact.is_none() {
            if let Some(mut index) = self.self_contact_index.take() {
                index.refit(&next.geometry);
                let _ = next.self_contact_index.set(index);
            }
        }
        *self = next;
        Ok((added, transferred))
    }
    pub fn material(&self) -> Material {
        self.material
    }
    /// Change physical properties without adding/removing liquid mass. Changing
    /// density rescales cell volumes inversely; this is an edit, not mixing fluids.
    /// Validation/overflow failure leaves all state unchanged.
    pub fn set_material(&mut self, material: Material) -> Result<(), &'static str> {
        material.validate()?;
        let ratio = self.material.density / material.density;
        let volumes: Vec<_> = self.volume.iter().map(|v| v * ratio).collect();
        let old_mass = self.total_mass();
        let new_mass = volumes.iter().sum::<f64>() * material.density;
        if !ratio.is_finite()
            || ratio <= 0.
            || volumes.iter().any(|v| !v.is_finite())
            || !old_mass.is_finite()
            || !new_mass.is_finite()
            || (new_mass - old_mass).abs() > old_mass.abs() * 1e-12
        {
            return Err("film material replacement overflow");
        }
        self.volume = volumes;
        self.material = material;
        Ok(())
    }
    /// Updates metrics of a moving mesh while retaining each cell's liquid volume.
    pub fn update_geometry(&mut self, points: &[[f64; 3]]) -> Result<(), &'static str> {
        if points.iter().flatten().any(|p| !p.is_finite()) {
            return Err("nonfinite surface");
        }
        let mut geometry = Vec::new();
        let mut normals = Vec::new();
        let mut area = Vec::new();
        let mut center = Vec::new();
        let mut owners = BTreeMap::<(usize, usize), Vec<usize>>::new();
        for (i, t) in self.triangles.iter().enumerate() {
            if t.iter().any(|&v| v >= points.len()) {
                return Err("invalid triangle index");
            }
            let a = points[t[0]];
            let b = sub(points[t[1]], a);
            let c = sub(points[t[2]], a);
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            let measure = norm(cross) * 0.5;
            if measure < 1e-14 || !measure.is_finite() {
                return Err("degenerate triangle");
            }
            geometry.push(t.map(|i| points[i]));
            normals.push(cross.map(|v| v / (2. * measure)));
            area.push(measure);
            center.push(std::array::from_fn(|k| {
                t.iter().map(|&v| points[v][k] / 3.).sum::<f64>()
            }));
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                owners.entry((a.min(b), a.max(b))).or_default().push(i);
            }
        }
        let mut edges = Vec::new();
        let mut edge_conormals = Vec::new();
        for ((a, b), cells) in owners {
            if cells.len() > 2 {
                return Err("nonmanifold surface");
            }
            if cells.len() == 2 {
                let length = norm(sub(points[a], points[b]));
                // Unfold the two facets around their shared edge. A triangle's
                // centroid is one third of its altitude from that edge, so the
                // cross-edge centroid separation is 2(Aa+Ab)/(3*length).
                // This is an uncorrected normal-distance scheme: tangential
                // cross-diffusion on skew cells still requires reconstruction.
                let distance = 2. * (area[cells[0]] + area[cells[1]]) / (3. * length);
                if distance < 1e-12 || !distance.is_finite() {
                    return Err("invalid cross-edge distance");
                }
                let tangent = sub(points[b], points[a]).map(|v| v / length);
                let midpoint = std::array::from_fn(|k| (points[a][k] + points[b][k]) * 0.5);
                let conormal = |cell: usize| {
                    let to_edge = sub(midpoint, center[cell]);
                    let along = dot(to_edge, tangent);
                    let perpendicular: [f64; 3] =
                        std::array::from_fn(|k| to_edge[k] - along * tangent[k]);
                    let altitude = norm(perpendicular);
                    perpendicular.map(|v| v / altitude)
                };
                let ca = conormal(cells[0]);
                let cb = conormal(cells[1]);
                let direction = std::array::from_fn(|k| (ca[k] - cb[k]) * 0.5);
                if direction.iter().any(|v: &f64| !v.is_finite()) {
                    return Err("invalid edge conormal");
                }
                edge_conormals.push(direction);
                edges.push((cells[0], cells[1], length, distance));
            }
        }
        // Cotangent position Laplacian with barycentric vertex areas. Projecting
        // onto the area-weighted normal gives signed twice-mean curvature.
        // Unlike centroid normal differences this does not attenuate curvature
        // merely because triangles are elongated along a flat direction.
        let mut vertex_area = vec![0.; points.len()];
        let mut vertex_normal = vec![[0.; 3]; points.len()];
        let mut laplacian = vec![[0.; 3]; points.len()];
        for (cell, t) in self.triangles.iter().enumerate() {
            for &v in t {
                vertex_area[v] += area[cell] / 3.;
                for k in 0..3 {
                    vertex_normal[v][k] += normals[cell][k] * area[cell];
                }
            }
            for (i, j, opposite) in [(t[0], t[1], t[2]), (t[1], t[2], t[0]), (t[2], t[0], t[1])] {
                let cotangent = dot(
                    sub(points[i], points[opposite]),
                    sub(points[j], points[opposite]),
                ) / (2. * area[cell]);
                let edge = sub(points[j], points[i]);
                for k in 0..3 {
                    laplacian[i][k] += cotangent * edge[k];
                    laplacian[j][k] -= cotangent * edge[k];
                }
            }
        }
        let vertex_curvature: Vec<f64> = (0..points.len())
            .map(|v| {
                if vertex_area[v] == 0. {
                    return 0.;
                }
                let normal_length = norm(vertex_normal[v]);
                if normal_length == 0. {
                    return f64::NAN;
                }
                -dot(laplacian[v], vertex_normal[v]) / (2. * vertex_area[v] * normal_length)
            })
            .collect();
        let substrate_curvature: Vec<f64> = self
            .triangles
            .iter()
            .map(|t| t.iter().map(|&v| vertex_curvature[v] / 3.).sum())
            .collect();
        if substrate_curvature.iter().any(|v| !v.is_finite())
            || center.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("film geometry overflow");
        }
        self.geometry = geometry;
        self.normals = normals;
        self.substrate_curvature = substrate_curvature;
        self.area = area;
        self.center = center;
        self.edges = edges;
        self.edge_conormals = edge_conormals;
        if let Some(index) = self.self_contact_index.get_mut() {
            index.refit(&self.geometry);
        }
        Ok(())
    }
    pub fn set_wetting(&mut self, wetting: Option<Wetting>) -> Result<(), &'static str> {
        if let Some(w) = wetting {
            w.validate()?;
        }
        self.wetting_model = wetting;
        Ok(())
    }
    /// Explicitly deposits the precursor layer; returned volume is added, not hidden.
    pub fn seed_precursor(&mut self) -> Result<f64, &'static str> {
        let w = self
            .wetting_model
            .ok_or("enable wetting before seeding precursor")?;
        let mut next = self.volume.clone();
        let mut added = 0.;
        for (i, v) in next.iter_mut().enumerate() {
            let minimum = w.precursor_thickness * self.area[i];
            if minimum > *v {
                added += minimum - *v;
                *v = minimum;
            }
        }
        if !added.is_finite() || next.iter().any(|v| !v.is_finite()) {
            return Err("precursor volume overflow");
        }
        self.volume = next;
        Ok(added)
    }
    pub fn detect_bridges(
        &self,
        other: &Self,
        config: BridgeConfig,
    ) -> Result<Vec<(usize, usize, f64)>, &'static str> {
        if !config.max_gap.is_finite()
            || config.max_gap <= 0.
            || !config.transfer_speed.is_finite()
            || config.transfer_speed < 0.
            || !config.opposing_cosine.is_finite()
            || !(0. ..=1.).contains(&config.opposing_cosine)
            || config.max_candidates == 0
        {
            return Err("invalid film bridge configuration");
        }
        if self.material.density != other.material.density {
            return Err("bridge exchange requires matching fluid density");
        }
        let candidates = crate::surface_film_contact::nearby(
            &self.geometry,
            &other.geometry,
            config.max_gap,
            config.max_candidates,
        )?;
        let mut links = Vec::new();
        for (a, b, gap) in candidates {
            if dot(self.normals[a], other.normals[b]) > -config.opposing_cosine {
                continue;
            }
            let proximity = (1. - gap / config.max_gap).max(0.);
            let overlap =
                crate::surface_film_contact::overlap_area(self.geometry[a], other.geometry[b])
                    .min(self.area[a])
                    .min(other.area[b]);
            if overlap <= 1e-14 {
                continue;
            }
            let conductance = config.transfer_speed * overlap / config.max_gap * proximity;
            if !conductance.is_finite() {
                return Err("bridge conductance overflow");
            }
            links.push((a, b, conductance));
        }
        Ok(links)
    }
    /// Same-surface liquid contacts. Shared-vertex neighbours and duplicate
    /// pairs are excluded; both films must span the gap. Static detection,
    /// phenomenological transfer speed; this does not resolve tissue collisions.
    pub fn detect_self_bridges(
        &self,
        config: BridgeConfig,
    ) -> Result<Vec<(usize, usize, f64)>, &'static str> {
        if !config.max_gap.is_finite()
            || config.max_gap <= 0.
            || !config.transfer_speed.is_finite()
            || config.transfer_speed < 0.
            || !config.opposing_cosine.is_finite()
            || !(0. ..=1.).contains(&config.opposing_cosine)
            || config.max_candidates == 0
        {
            return Err("invalid film bridge configuration");
        }
        if !self.volume.iter().any(|v| *v > 0.) {
            return Ok(Vec::new());
        }
        let height = self.thickness();
        let index = self
            .self_contact_index
            .get_or_init(|| crate::surface_film_contact::ProximityIndex::new(&self.geometry));
        let candidates = index.nearby_filtered(
            &self.geometry,
            &self.geometry,
            config.max_gap,
            config.max_candidates,
            |a| self.volume[a] > 0.,
            |a, b| {
                a != b
                    && !(a > b && self.volume[b] > 0.)
                    && !self.triangles[a]
                        .iter()
                        .any(|v| self.triangles[b].contains(v))
                    && dot(self.normals[a], self.normals[b]) <= -config.opposing_cosine
            },
        )?;
        let mut links = Vec::new();
        for (a, b, gap) in candidates {
            if gap > height[a] + height[b] {
                continue;
            }
            let overlap =
                crate::surface_film_contact::overlap_area(self.geometry[a], self.geometry[b])
                    .min(self.area[a])
                    .min(self.area[b]);
            if overlap <= 1e-14 {
                continue;
            }
            let conductance = config.transfer_speed * overlap / config.max_gap
                * (1. - gap / config.max_gap).max(0.);
            if !conductance.is_finite() {
                return Err("bridge conductance overflow");
            }
            links.push((a.min(b), a.max(b), conductance));
        }
        links.sort_by_key(|&(a, b, _)| (a, b));
        Ok(links)
    }
    pub fn exchange_self_contact(
        &mut self,
        dt: f64,
        config: BridgeConfig,
    ) -> Result<f64, &'static str> {
        let links = self.detect_self_bridges(config)?;
        self.exchange_self_with(dt, &links)
    }
    /// Donor-limited same-film exchange. Returns gross volume moved in m3.
    /// Duplicate unordered pairs, self-links and invalid inputs fail atomically.
    pub fn exchange_self_with(
        &mut self,
        dt: f64,
        links: &[(usize, usize, f64)],
    ) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return Err("invalid film exchange timestep");
        }
        let height = self.thickness();
        let mut seen = std::collections::BTreeSet::new();
        let mut outgoing = vec![0.; height.len()];
        let mut flux = Vec::new();
        for &(a, b, c) in links {
            if a >= height.len()
                || b >= height.len()
                || a == b
                || !c.is_finite()
                || c < 0.
                || !seen.insert((a.min(b), a.max(b)))
            {
                return Err("invalid self film contact link");
            }
            let moved = dt * c * (height[a] - height[b]);
            if !moved.is_finite() {
                return Err("film contact overflow");
            }
            outgoing[if moved >= 0. { a } else { b }] += moved.abs();
            flux.push((a, b, moved));
        }
        if outgoing.iter().any(|v| !v.is_finite()) {
            return Err("film contact overflow");
        }
        let mut delta = vec![0.; height.len()];
        let mut transferred = 0.;
        for (a, b, amount) in flux {
            let donor = if amount >= 0. { a } else { b };
            let moved = amount
                * if outgoing[donor] > self.volume[donor] {
                    self.volume[donor] / outgoing[donor]
                } else {
                    1.
                };
            delta[a] -= moved;
            delta[b] += moved;
            transferred += moved.abs();
        }
        let next: Vec<_> = self
            .volume
            .iter()
            .zip(delta)
            .map(|(v, d)| (v + d).max(0.))
            .collect();
        if !transferred.is_finite() || next.iter().any(|v| !v.is_finite()) {
            return Err("film contact overflow");
        }
        self.volume = next;
        Ok(transferred)
    }
    pub fn exchange_contact(
        &mut self,
        other: &mut Self,
        dt: f64,
        config: BridgeConfig,
    ) -> Result<f64, &'static str> {
        let links = self.detect_bridges(other, config)?;
        self.exchange_with(other, dt, &links)
    }
    /// Conservative exchange along caller-supplied contact links (cell_a, cell_b, m²/s).
    /// Contact detection is external; all links are validated before either film changes.
    pub fn exchange_with(
        &mut self,
        other: &mut Self,
        dt: f64,
        links: &[(usize, usize, f64)],
    ) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return Err("invalid film exchange timestep");
        }
        let a = self.thickness();
        let b = other.thickness();
        let mut outgoing_a = vec![0.; a.len()];
        let mut outgoing_b = vec![0.; b.len()];
        let mut flux = Vec::new();
        for &(i, j, conductance) in links {
            if i >= a.len() || j >= b.len() || !conductance.is_finite() || conductance < 0. {
                return Err("invalid film contact link");
            }
            let moved = dt * conductance * (a[i] - b[j]);
            if !moved.is_finite() {
                return Err("film contact overflow");
            }
            if moved >= 0. {
                outgoing_a[i] += moved;
            } else {
                outgoing_b[j] -= moved;
            }
            flux.push((i, j, moved));
        }
        if outgoing_a.iter().chain(&outgoing_b).any(|v| !v.is_finite()) {
            return Err("film contact overflow");
        }
        let mut delta_a = vec![0.; a.len()];
        let mut delta_b = vec![0.; b.len()];
        let mut transferred = 0.;
        for (i, j, amount) in flux {
            let (available, outgoing) = if amount >= 0. {
                (self.volume[i], outgoing_a[i])
            } else {
                (other.volume[j], outgoing_b[j])
            };
            let moved = amount
                * if outgoing > available {
                    available / outgoing
                } else {
                    1.
                };
            delta_a[i] -= moved;
            delta_b[j] += moved;
            transferred += moved;
        }
        let next_a: Vec<_> = self
            .volume
            .iter()
            .zip(delta_a)
            .map(|(v, d)| (*v + d).max(0.))
            .collect();
        let next_b: Vec<_> = other
            .volume
            .iter()
            .zip(delta_b)
            .map(|(v, d)| (*v + d).max(0.))
            .collect();
        if next_a.iter().chain(&next_b).any(|v| !v.is_finite()) {
            return Err("film contact overflow");
        }
        self.volume = next_a;
        other.volume = next_b;
        Ok(transferred)
    }
    /// Independent source rates in m³/s. No biological rates are inferred.
    pub fn add_sources(&mut self, dt: f64, sources: &[(usize, f64)]) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return Err("invalid source timestep");
        }
        let mut next = self.volume.clone();
        let mut added = 0.;
        for &(i, rate) in sources {
            if i >= next.len() || !rate.is_finite() || rate < 0. {
                return Err("invalid film source");
            }
            let amount = dt * rate;
            next[i] += amount;
            added += amount;
            if !next[i].is_finite() || !added.is_finite() {
                return Err("source overflow");
            }
        }
        self.volume = next;
        Ok(added)
    }
    /// Current fluid mass in kilograms, using the configured material density.
    pub fn total_mass(&self) -> f64 {
        self.total_volume() * self.material.density
    }
    pub fn total_volume(&self) -> f64 {
        self.volume.iter().sum()
    }
    pub fn thickness(&self) -> Vec<f64> {
        self.volume
            .iter()
            .zip(&self.area)
            .map(|(v, a)| v / a)
            .collect()
    }
    /// Pressure-equivalent driving potential (Pa) used in transport. Includes
    /// gravitational potential relative to the world origin, capillarity and
    /// optional disjoining pressure. This is not tissue contact pressure.
    pub fn driving_pressure(&self, gravity: [f64; 3]) -> Result<Vec<f64>, &'static str> {
        if gravity.iter().any(|g| !g.is_finite()) || !norm(gravity).is_finite() {
            return Err("invalid film gravity");
        }
        self.pressure_for_height(&self.thickness(), gravity)
    }
    fn pressure_for_height(
        &self,
        height: &[f64],
        gravity: [f64; 3],
    ) -> Result<Vec<f64>, &'static str> {
        let mut curvature = vec![0.; height.len()];
        for &(a, b, length, distance) in &self.edges {
            let laplace = length / distance * (height[b] - height[a]);
            curvature[a] += laplace / self.area[a];
            curvature[b] -= laplace / self.area[b];
        }
        let mut pressure = Vec::with_capacity(height.len());
        for i in 0..height.len() {
            let wetting = if let Some(w) = self.wetting_model {
                w.pressure(height[i], self.material.surface_tension)?
            } else {
                0.
            };
            let p = -self.material.density * dot(gravity, self.center[i])
                - self.material.density * dot(gravity, self.normals[i]) * height[i]
                + self.material.surface_tension * (self.substrate_curvature[i] - curvature[i])
                + wetting;
            if !p.is_finite() {
                return Err("film pressure overflow");
            }
            pressure.push(p);
        }
        Ok(pressure)
    }
    /// Atomic deposits: all indices, accumulated volumes and total mass are checked
    /// before any cell is changed. Returns added volume in cubic metres.
    pub fn deposit_batch(&mut self, deposits: &[(usize, f64)]) -> Result<f64, &'static str> {
        let mut next = self.volume.clone();
        let mut added = 0.0;
        for &(cell, volume) in deposits {
            if cell >= next.len() || !volume.is_finite() || volume < 0.0 {
                return Err("invalid film deposit");
            }
            next[cell] += volume;
            added += volume;
            if !next[cell].is_finite() || !added.is_finite() {
                return Err("film deposit overflow");
            }
        }
        let total = next.iter().sum::<f64>();
        if !total.is_finite() || !(total * self.material.density).is_finite() {
            return Err("film mass overflow");
        }
        self.volume = next;
        Ok(added)
    }

    /// Unit normal of a triangle cell in the current geometry.
    pub fn cell_normal(&self, cell: usize) -> Result<[f64; 3], &'static str> {
        self.normals.get(cell).copied().ok_or("invalid film cell")
    }

    /// First intersection of a point-center segment with this stationary triangle
    /// surface. Both sides are accepted. No particle radius or curved trajectory is
    /// inferred; callers must refine time steps for the actual path.
    pub fn first_segment_hit(
        &self,
        start: [f64; 3],
        end: [f64; 3],
    ) -> Result<Option<(usize, f64)>, &'static str> {
        if start.iter().chain(&end).any(|v| !v.is_finite()) {
            return Err("invalid film capture segment");
        }
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let direction = sub(end, start);
        let mut nearest: Option<(usize, f64)> = None;
        for (cell, triangle) in self.geometry.iter().enumerate() {
            let e1 = sub(triangle[1], triangle[0]);
            let e2 = sub(triangle[2], triangle[0]);
            let p = cross(direction, e2);
            let determinant = dot(e1, p);
            let scale = norm(e1) * norm(e2) * norm(direction);
            if !scale.is_finite() || !determinant.is_finite() {
                return Err("film capture geometry overflow");
            }
            if scale == 0.0 || determinant.abs() <= 64.0 * f64::EPSILON * scale {
                continue;
            }
            let offset = sub(start, triangle[0]);
            let u = dot(offset, p) / determinant;
            let q = cross(offset, e1);
            let v = dot(direction, q) / determinant;
            let t = dot(e2, q) / determinant;
            if !u.is_finite() || !v.is_finite() || !t.is_finite() {
                return Err("film capture intersection overflow");
            }
            let tolerance = 64.0 * f64::EPSILON;
            if u >= -tolerance
                && v >= -tolerance
                && u + v <= 1.0 + tolerance
                && t > 0.0
                && t <= 1.0 + tolerance
                && nearest.is_none_or(|(_, previous)| t < previous)
            {
                nearest = Some((cell, t.min(1.0)));
            }
        }
        Ok(nearest)
    }

    pub fn deposit(&mut self, cell: usize, volume_m3: f64) -> Result<(), &'static str> {
        if cell >= self.volume.len()
            || !volume_m3.is_finite()
            || volume_m3 < 0.
            || !(self.volume[cell] + volume_m3).is_finite()
        {
            return Err("invalid film deposit");
        }
        self.volume[cell] += volume_m3;
        Ok(())
    }
    /// Pairwise donor-limited flow. Internal steps are at most 1 ms.
    /// Positivity/conservation are enforced; time-step convergence must be checked per mesh.
    pub fn step(&mut self, dt: f64, gravity: [f64; 3]) -> Result<(), &'static str> {
        self.step_with_max_substep(dt, gravity, 0.001)
    }
    /// Explicit integration control for temporal convergence studies (max 1 ms).
    pub fn step_with_max_substep(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        self.step_driven(dt, gravity, max_substep, None, None, None)
    }

    /// Advances gravity/capillarity/wetting plus prescribed tangential surface
    /// traction in Pa per triangle. Lubrication shear flux is tau*h²/(2*mu).
    /// Normal traction is projected out. Closed boundary edges retain liquid.
    /// This model does not solve a rubbing solid's contact force or supplied work.
    pub fn step_with_surface_shear(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        if traction.len() != self.volume.len()
            || traction
                .iter()
                .any(|t| t.iter().any(|v| !v.is_finite()) || !norm(*t).is_finite())
        {
            return Err("invalid film surface traction");
        }
        self.step_driven(dt, gravity, max_substep, Some(traction), None, None)
    }

    /// Non-Newtonian pressure/shear profile with supplied, validated rheology.
    /// Midpoint profile quadrature is used when pressure and shear act together.
    pub fn step_with_rheology(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
        model: FilmRheology,
    ) -> Result<(), &'static str> {
        model.validate()?;
        if traction.len() != self.volume.len()
            || traction
                .iter()
                .any(|t| t.iter().any(|v| !v.is_finite()) || !norm(*t).is_finite())
        {
            return Err("invalid film surface traction");
        }
        self.step_driven(dt, gravity, max_substep, Some(traction), Some(model), None)
    }

    /// Adds prescribed mean-layer advection to existing free-film pressure flow.
    /// Velocities are in m/s per cell, tangentially projected before interpolation.
    /// For pure Couette loading the mean is half the upper-wall relative velocity.
    /// This does not change the pressure mobility to a confined-gap Reynolds model.
    pub fn step_with_advection(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        velocity: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        if velocity.len() != self.volume.len()
            || velocity
                .iter()
                .any(|v| v.iter().any(|x| !x.is_finite()) || !norm(*v).is_finite())
        {
            return Err("invalid film advection velocity");
        }
        self.step_driven(dt, gravity, max_substep, None, None, Some(velocity))
    }

    fn step_driven(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        max_substep: f64,
        traction: Option<&[[f64; 3]]>,
        rheology: Option<FilmRheology>,
        velocity: Option<&[[f64; 3]]>,
    ) -> Result<(), &'static str> {
        self.step_driven_components(
            dt,
            gravity,
            max_substep,
            traction,
            rheology,
            velocity,
            None,
            None,
        )
    }

    fn step_driven_components(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        max_substep: f64,
        traction: Option<&[[f64; 3]]>,
        rheology: Option<FilmRheology>,
        velocity: Option<&[[f64; 3]]>,
        components: Option<&mut Vec<Vec<f64>>>,
        component_viscosities: Option<&[f64]>,
    ) -> Result<(), &'static str> {
        for field in [traction, velocity].into_iter().flatten() {
            if field.len() != self.volume.len()
                || field
                    .iter()
                    .any(|v| v.iter().any(|x| !x.is_finite()) || !norm(*v).is_finite())
            {
                return Err("invalid film driving field");
            }
        }
        if let Some(model) = rheology {
            model.validate()?;
        }
        if component_viscosities.is_some() && (components.is_none() || rheology.is_some()) {
            return Err("composition viscosity requires Newtonian component transport");
        }
        let mut inventory = components.as_deref().cloned();
        if !max_substep.is_finite() || max_substep <= 0. || max_substep > 0.001 {
            return Err("invalid film substep");
        }
        if !dt.is_finite()
            || dt <= 0.
            || dt > 0.1
            || gravity.iter().any(|g| !g.is_finite())
            || !norm(gravity).is_finite()
        {
            return Err("invalid film timestep or gravity");
        }
        let mut volume = self.volume.clone();
        let count = (dt / max_substep).ceil() as usize;
        if count > 100000 {
            return Err("film substep budget exceeded");
        }
        let step = dt / count as f64;
        let gravity_terms = |edge: usize, a: usize, b: usize| {
            (
                self.material.density * dot(gravity, sub(self.center[b], self.center[a])),
                self.material.density * dot(gravity, self.edge_conormals[edge]),
            )
        };
        // Geometry, material and gravity are fixed for this integration call.
        // Keep the exact two operands, without reassociating the pressure sum.
        // Single-substep calls avoid allocating a cache they cannot reuse.
        let cached_gravity: Option<Vec<_>> = (count > 1).then(|| {
            self.edges
                .iter()
                .enumerate()
                .map(|(edge, &(a, b, _, _))| gravity_terms(edge, a, b))
                .collect()
        });
        for _ in 0..count {
            let height: Vec<_> = volume.iter().zip(&self.area).map(|(v, a)| v / a).collect();
            let pressure = self.pressure_for_height(&height, gravity)?;
            let cell_viscosities = match (&inventory, component_viscosities) {
                (Some(rows), Some(values)) => Some(mixture::blend_viscosities(
                    rows,
                    values,
                    self.material.viscosity,
                )),
                _ => None,
            };
            let viscosity = |cell: usize| {
                cell_viscosities
                    .as_ref()
                    .map_or(self.material.viscosity, |values| values[cell])
            };
            let mut flux = Vec::new();
            let mut outgoing = vec![0.; volume.len()];
            for (edge, &(a, b, length, distance)) in self.edges.iter().enumerate() {
                // Integrate the known gravity gradient along facet conormals.
                // Remove its centroid-difference contribution first: tangential
                // centroid offsets must not produce flow across the shared edge.
                let (centroid_gravity, edge_gravity) = cached_gravity
                    .as_ref()
                    .map_or_else(|| gravity_terms(edge, a, b), |values| values[edge]);
                let driving_gradient =
                    (pressure[a] - pressure[b] - centroid_gravity) / distance + edge_gravity;
                let donor = if driving_gradient > 0. { a } else { b };
                let mobility = height[donor].powi(3) / (3. * viscosity(donor));
                let (stress, surface_traction) = if let Some(traction) = traction {
                    let project = |cell: usize| -> [f64; 3] {
                        let normal = dot(traction[cell], self.normals[cell]);
                        std::array::from_fn(|axis| {
                            traction[cell][axis] - normal * self.normals[cell][axis]
                        })
                    };
                    let first = project(a);
                    let second = project(b);
                    let mean = std::array::from_fn(|axis| 0.5 * first[axis] + 0.5 * second[axis]);
                    let stress = dot(mean, self.edge_conormals[edge]);
                    (stress, mean)
                } else {
                    (0.0, [0.0; 3])
                };
                let flow = if let Some(model) = rheology {
                    let edge_norm = norm(self.edge_conormals[edge]);
                    if !edge_norm.is_finite() || edge_norm <= 0.0 {
                        return Err("degenerate rheology edge");
                    }
                    let direction = self.edge_conormals[edge].map(|v| v / edge_norm);
                    model.interface_flow(
                        height[a],
                        height[b],
                        driving_gradient,
                        direction,
                        surface_traction,
                    )?
                } else {
                    let donor = if stress >= 0.0 { a } else { b };
                    mobility * driving_gradient
                        + stress * height[donor].powi(2) / (2.0 * viscosity(donor))
                };
                let advection = if let Some(velocity) = velocity {
                    let projected = |cell: usize| -> [f64; 3] {
                        let normal = dot(velocity[cell], self.normals[cell]);
                        std::array::from_fn(|k| velocity[cell][k] - normal * self.normals[cell][k])
                    };
                    let first = projected(a);
                    let second = projected(b);
                    let mean = std::array::from_fn(|k| 0.5 * first[k] + 0.5 * second[k]);
                    let speed = dot(mean, self.edge_conormals[edge]);
                    speed * height[if speed >= 0.0 { a } else { b }]
                } else {
                    0.0
                };
                let amount = step
                    * length
                    * (flow
                        + self.material.wetting * (height[a] - height[b]) / distance
                        + advection);
                if !amount.is_finite() {
                    return Err("film flux overflow");
                }
                if amount >= 0. {
                    outgoing[a] += amount;
                } else {
                    outgoing[b] -= amount;
                }
                if !outgoing[a].is_finite() || !outgoing[b].is_finite() {
                    return Err("film outgoing flux overflow");
                }
                flux.push((a, b, amount));
            }
            let mut delta = vec![0.; volume.len()];
            let mut component_delta = inventory.as_ref().map(|rows| {
                rows.iter()
                    .map(|row| vec![0.0; row.len()])
                    .collect::<Vec<_>>()
            });
            for (a, b, amount) in flux {
                let donor = if amount >= 0. { a } else { b };
                let scale = if outgoing[donor] > volume[donor] {
                    volume[donor] / outgoing[donor]
                } else {
                    1.
                };
                let moved = amount * scale;
                delta[a] -= moved;
                delta[b] += moved;
                if let (Some(rows), Some(change)) = (&inventory, &mut component_delta) {
                    let fraction = if volume[donor] > 0.0 {
                        moved / volume[donor]
                    } else {
                        0.0
                    };
                    for k in 0..rows[donor].len() {
                        let carried = fraction * rows[donor][k];
                        change[a][k] -= carried;
                        change[b][k] += carried;
                    }
                }
            }
            if let (Some(rows), Some(change)) = (&mut inventory, component_delta) {
                for (row, delta) in rows.iter_mut().zip(change) {
                    for (v, d) in row.iter_mut().zip(delta) {
                        *v = (*v + d).max(0.0);
                        if !v.is_finite() {
                            return Err("film component inventory overflow");
                        }
                    }
                }
            }
            for (v, d) in volume.iter_mut().zip(delta) {
                *v = (*v + d).max(0.);
                if !v.is_finite() {
                    return Err("film volume overflow");
                }
            }
        }
        self.volume = volume;
        if let (Some(target), Some(inventory)) = (components, inventory) {
            *target = inventory;
        }
        Ok(())
    }
    /// Vertex values for a renderer, area-weighted across adjacent cells.
    pub fn vertex_thickness(&self, vertex_count: usize) -> Result<Vec<f64>, &'static str> {
        let mut height = vec![0.; vertex_count];
        let mut area = vec![0.; vertex_count];
        for (i, t) in self.triangles.iter().enumerate() {
            for &v in t {
                if v >= vertex_count {
                    return Err("invalid output vertex count");
                }
                height[v] += self.volume[i];
                area[v] += self.area[i];
            }
        }
        for (h, a) in height.iter_mut().zip(area) {
            if a > 0. {
                *h /= a;
            }
        }
        Ok(height)
    }
}

#[path = "surface_film_body.rs"]
mod sliding_body;
pub use sliding_body::{SlidingAdvanceReport, SlidingBodyReport, SlidingPatch, SlidingPatchReport};

#[path = "surface_film_mixture.rs"]
mod mixture;
pub use mixture::FilmMixture;

#[path = "surface_film_squeeze.rs"]
mod squeeze;
pub use squeeze::{SqueezePressureControl, SqueezePressureReport, SqueezeTransportReport};

#[path = "surface_film_squeeze_body.rs"]
mod squeeze_body;
pub use squeeze_body::SqueezeBodyReport;
#[path = "surface_film_sphere.rs"]
mod sphere;
pub use sphere::SphereFilmHit;
#[path = "surface_film_accelerated_sphere.rs"]
mod accelerated_sphere;

#[path = "surface_film_free_surface.rs"]
mod free_surface;
pub use free_surface::FilmFreeSurface;

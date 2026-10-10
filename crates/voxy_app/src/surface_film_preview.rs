//! Weld OBJ seams before attaching triangle-cell liquid; render mapped thickness.
use glam::Vec3;
use physics::surface_film::{Material, SurfaceFilm};
use voxy_render::SceneVertex;
#[derive(Debug)]
pub(crate) struct FilmPreview {
    film: SurfaceFilm,
    render_to_solver: Vec<usize>,
    representatives: Vec<usize>,
    source_weights: Vec<f64>,
    canonical_triangles: Vec<[[f64; 3]; 3]>,
    extra_source_weights: Vec<Vec<f64>>,
    initial_mass_kg: f64,
    source_added_mass_kg: f64,
    self_contact_gross_volume_m3: f64,
    self_contact_last_volume_m3: f64,
    self_contact_error: Option<&'static str>,
    settings: crate::film_settings::FilmSettings,
}
impl FilmPreview {
    /// Transfer through prepared body charts. Unsupported wet mouth edits reject
    /// the operation rather than dropping or relocating their liquid.
    pub(crate) fn remapped_body(
        &self,
        vertices: &[SceneVertex],
        indices: &[u32],
        source: crate::body_parameters::BodyModel,
    ) -> Result<Self, &'static str> {
        use crate::body_parameters::BodyModel;
        let (source_mesh, target_mesh, map) = match source {
            BodyModel::Female => (
                include_str!(
                    "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
                ),
                include_str!(
                    "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
                ),
                include_str!("../../../assets/characters/female-to-repaired-male-film-map.json"),
            ),
            BodyModel::Male => (
                include_str!(
                    "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
                ),
                include_str!(
                    "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
                ),
                include_str!("../../../assets/characters/repaired-male-to-female-film-map.json"),
            ),
        };
        self.remapped_reference(vertices, indices, source_mesh, target_mesh, map)
    }
    pub(crate) fn remapped_reference(
        &self,
        vertices: &[SceneVertex],
        indices: &[u32],
        source_mesh: &str,
        target_mesh: &str,
        map: &str,
    ) -> Result<Self, &'static str> {
        let key = |triangle: [[f64; 3]; 3]| {
            let mut points = triangle.map(|p| p.map(|v| (v as f32).to_bits()));
            points.sort();
            points
        };
        let reference = |source: &str| -> Result<Vec<[[f64; 3]; 3]>, &'static str> {
            let asset = voxy_render::ObjAsset::parse(source, voxy_render::ObjLimits::default())
                .map_err(|_| "invalid remap reference mesh")?;
            Ok(asset
                .mesh
                .indices()
                .chunks_exact(3)
                .map(|t| {
                    std::array::from_fn(|k| {
                        asset.mesh.vertices()[t[k] as usize].position.map(f64::from)
                    })
                })
                .collect())
        };
        let source_triangles = reference(source_mesh)?;
        let target_triangles = reference(target_mesh)?;
        let source_cells: std::collections::HashMap<_, _> = source_triangles
            .into_iter()
            .enumerate()
            .map(|(i, t)| (key(t), i))
            .collect();
        let value: serde_json::Value =
            serde_json::from_str(map).map_err(|_| "invalid body remap map")?;
        let distribution: Vec<Vec<(usize, f64)>> =
            serde_json::from_value(value["distribution"].clone())
                .map_err(|_| "invalid body remap fractions")?;
        let mut result = Self::new(
            vertices,
            indices,
            vertices
                .first()
                .ok_or("empty remap mesh")?
                .position
                .map(f64::from),
            10.,
            0.,
        )?;
        let target_cells: std::collections::HashMap<_, _> = result
            .canonical_triangles
            .iter()
            .enumerate()
            .map(|(i, &t)| (key(t), i))
            .collect();
        let thickness = self.film.thickness();
        let mut rows = Vec::with_capacity(thickness.len());
        for (triangle, height) in self.canonical_triangles.iter().zip(thickness) {
            if height == 0. {
                rows.push(Vec::new());
                continue;
            }
            let donor = *source_cells
                .get(&key(*triangle))
                .ok_or("wet surface has no verified anatomical correspondence")?;
            let row = distribution
                .get(donor)
                .ok_or("missing body remap donor")?
                .iter()
                .map(|&(i, w)| {
                    let triangle = target_triangles.get(i).ok_or("invalid body remap target")?;
                    let recipient = *target_cells
                        .get(&key(*triangle))
                        .ok_or("wet remap crosses an unsupported mouth surface edit")?;
                    Ok((recipient, w))
                })
                .collect::<Result<Vec<_>, &'static str>>()?;
            rows.push(row);
        }
        let points: Vec<_> = result
            .representatives
            .iter()
            .map(|&i| vertices[i].position.map(f64::from))
            .collect();
        let triangles: Vec<_> = indices
            .chunks_exact(3)
            .map(|t| {
                [
                    result.render_to_solver[t[0] as usize],
                    result.render_to_solver[t[1] as usize],
                    result.render_to_solver[t[2] as usize],
                ]
            })
            .collect();
        result.film = self.film.remapped(&points, triangles, &rows)?;
        result.configure(self.settings.clone())?;
        result.initial_mass_kg = self.initial_mass_kg;
        result.source_added_mass_kg = self.source_added_mass_kg;
        result.self_contact_gross_volume_m3 = self.self_contact_gross_volume_m3;
        result.self_contact_last_volume_m3 = self.self_contact_last_volume_m3;
        result.self_contact_error = self.self_contact_error;
        Ok(result)
    }
    pub(crate) fn rebound_same_body(
        &self,
        vertices: &[SceneVertex],
        indices: &[u32],
    ) -> Result<Self, &'static str> {
        let key = |triangle: [[f64; 3]; 3]| {
            let mut points = triangle.map(|p| p.map(|v| (v as f32).to_bits()));
            points.sort();
            points
        };
        let mut result = Self::new(
            vertices,
            indices,
            vertices
                .first()
                .ok_or("empty film rebind surface")?
                .position
                .map(f64::from),
            10.,
            0.,
        )?;
        let recipients: std::collections::HashMap<_, _> = result
            .canonical_triangles
            .iter()
            .enumerate()
            .map(|(i, &t)| (key(t), i))
            .collect();
        let rows = self
            .canonical_triangles
            .iter()
            .zip(self.film.thickness())
            .map(|(&triangle, h)| {
                if h == 0. {
                    return Ok(Vec::new());
                }
                Ok(vec![(
                    *recipients
                        .get(&key(triangle))
                        .ok_or("wet cell cannot rebind to the edited body")?,
                    1.,
                )])
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let points: Vec<_> = result
            .representatives
            .iter()
            .map(|&i| vertices[i].position.map(f64::from))
            .collect();
        let triangles = indices
            .chunks_exact(3)
            .map(|t| {
                [
                    result.render_to_solver[t[0] as usize],
                    result.render_to_solver[t[1] as usize],
                    result.render_to_solver[t[2] as usize],
                ]
            })
            .collect();
        result.film = self.film.remapped(&points, triangles, &rows)?;
        result.configure(self.settings.clone())?;
        result.initial_mass_kg = self.initial_mass_kg;
        result.source_added_mass_kg = self.source_added_mass_kg;
        result.self_contact_gross_volume_m3 = self.self_contact_gross_volume_m3;
        result.self_contact_last_volume_m3 = self.self_contact_last_volume_m3;
        result.self_contact_error = self.self_contact_error;
        Ok(result)
    }
    pub(crate) fn thickness_view_enabled(&self) -> bool {
        self.settings.thickness_view
    }
    pub fn new(
        vertices: &[SceneVertex],
        indices: &[u32],
        center: [f64; 3],
        radius: f64,
        volume: f64,
    ) -> Result<Self, &'static str> {
        if center.iter().any(|p| !p.is_finite())
            || !radius.is_finite()
            || radius <= 0.
            || !volume.is_finite()
            || volume < 0.
        {
            return Err("invalid film brush");
        }
        let mut lookup = std::collections::HashMap::new();
        let mut points = Vec::new();
        let mut representatives = Vec::new();
        let mut map = Vec::new();
        for (i, v) in vertices.iter().enumerate() {
            let id = *lookup
                .entry(v.position.map(f32::to_bits))
                .or_insert_with(|| {
                    let id = points.len();
                    points.push(v.position.map(f64::from));
                    representatives.push(i);
                    id
                });
            map.push(id);
        }
        let cells: Vec<_> = indices
            .chunks_exact(3)
            .map(|t| [map[t[0] as usize], map[t[1] as usize], map[t[2] as usize]])
            .collect();
        let mut film = SurfaceFilm::new(&points, cells.clone(), Material::default())?;
        let canonical_triangles: Vec<_> = cells.iter().map(|t| t.map(|i| points[i])).collect();
        let weights = source_weights(&canonical_triangles, center, radius)?;
        let sum: f64 = weights.iter().sum();
        if sum <= 0. {
            return Err("film brush misses surface");
        }
        for (i, w) in weights.iter().enumerate() {
            film.deposit(i, volume * w / sum)?;
        }
        let initial_mass_kg = film.total_mass();
        Ok(Self {
            initial_mass_kg,
            source_added_mass_kg: 0.,
            self_contact_gross_volume_m3: 0.,
            self_contact_last_volume_m3: 0.,
            self_contact_error: None,
            film,
            render_to_solver: map,
            representatives,
            source_weights: weights.iter().map(|w| w / sum).collect(),
            canonical_triangles,
            extra_source_weights: Vec::new(),
            settings: crate::film_settings::FilmSettings {
                source_center: center,
                source_radius_m: radius,
                ..Default::default()
            },
        })
    }
    pub fn configure(
        &mut self,
        settings: crate::film_settings::FilmSettings,
    ) -> Result<(), &'static str> {
        let settings = settings.patched(&serde_json::json!({}))?;
        let weights = source_weights(
            &self.canonical_triangles,
            settings.source_center,
            settings.source_radius_m,
        )?;
        let extra = settings
            .sources
            .iter()
            .map(|s| source_weights(&self.canonical_triangles, s.center, s.radius_m))
            .collect::<Result<Vec<_>, _>>()?;
        self.film.set_material(settings.material)?;
        self.film
            .set_wetting(settings.precursor_wetting_enabled.then_some(
                physics::surface_film::Wetting {
                    contact_angle: settings.contact_angle_rad,
                    precursor_thickness: settings.precursor_thickness_m,
                },
            ))?;
        self.extra_source_weights = extra;
        self.source_weights = weights;

        self.settings = settings;
        Ok(())
    }
    pub fn optical_parameters(&self) -> [f32; 4] {
        [
            self.settings.refractive_index as f32,
            self.settings.absorption_rgb_per_m[0] as f32,
            self.settings.absorption_rgb_per_m[1] as f32,
            self.settings.absorption_rgb_per_m[2] as f32,
        ]
    }
    /// Capture actual numerical film distribution and render/source mappings.
    /// Explicit use only: this can be large and does not capture a viewer image.
    pub fn state_snapshot(&self) -> serde_json::Value {
        let state = self.film.state();
        serde_json::json!({"format":"voxy.surface-film-state.v1","units":"SI",
            "physics":{"pointsM":state.points,"triangles":state.triangles,
                "cellVolumesM3":state.cell_volumes_m3,
                "material":{"density":state.material.density,"viscosity":state.material.viscosity,
                    "surfaceTension":state.material.surface_tension,"wetting":state.material.wetting},
                "precursorWetting":state.wetting.map(|w|serde_json::json!({"contactAngleRad":w.contact_angle,"precursorThicknessM":w.precursor_thickness}))},
            "renderToSolver":self.render_to_solver,"representatives":self.representatives,
            "canonicalTrianglesM":self.canonical_triangles,
            "sourceWeights":self.source_weights,"extraSourceWeights":self.extra_source_weights,
            "settings":self.settings.to_json(),"measurements":self.measurements(),
            "capture":"numericalFilmState","viewerImageCaptured":false,"wholeBodyCheckpoint":false})
    }
    /// Restore into the same canonical body chart. The body pose/solver is separate.
    /// All validation precedes replacement, so a malformed checkpoint is atomic.
    pub fn restore_snapshot(&mut self, snapshot: &serde_json::Value) -> Result<(), &'static str> {
        use physics::surface_film::{SurfaceFilmState, Wetting};
        if snapshot["format"] != "voxy.surface-film-state.v1" || snapshot["units"] != "SI" {
            return Err("unsupported film checkpoint format or units");
        }
        let decode = |key: &str| {
            snapshot
                .get(key)
                .cloned()
                .ok_or("missing film checkpoint field")
        };
        let map: Vec<usize> = serde_json::from_value(decode("renderToSolver")?)
            .map_err(|_| "invalid checkpoint render map")?;
        let representatives: Vec<usize> = serde_json::from_value(decode("representatives")?)
            .map_err(|_| "invalid checkpoint representatives")?;
        let canonical: Vec<[[f64; 3]; 3]> = serde_json::from_value(decode("canonicalTrianglesM")?)
            .map_err(|_| "invalid checkpoint canonical chart")?;
        if map != self.render_to_solver {
            return Err("checkpoint render map mismatch");
        }
        if representatives != self.representatives {
            return Err("checkpoint representatives mismatch");
        }
        if canonical != self.canonical_triangles {
            return Err("checkpoint canonical chart mismatch");
        }
        let settings =
            crate::film_settings::FilmSettings::default().patched(&snapshot["settings"])?;
        let weights = source_weights(&canonical, settings.source_center, settings.source_radius_m)?;
        let extra = settings
            .sources
            .iter()
            .map(|s| source_weights(&canonical, s.center, s.radius_m))
            .collect::<Result<Vec<_>, _>>()?;
        let captured_weights: Vec<f64> = serde_json::from_value(decode("sourceWeights")?)
            .map_err(|_| "invalid checkpoint source weights")?;
        let captured_extra: Vec<Vec<f64>> =
            serde_json::from_value(decode("extraSourceWeights")?)
                .map_err(|_| "invalid checkpoint extra source weights")?;
        // Recomputed quadrature can differ by final rounding across call sites.
        // Validate normalized L1 agreement, but retain the saved weights exactly.
        let matches = |saved: &[f64], expected: &[f64]| {
            saved.len() == expected.len()
                && saved.iter().all(|v| v.is_finite() && *v >= 0.)
                && saved
                    .iter()
                    .zip(expected)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f64>()
                    <= 128. * f64::EPSILON
        };
        if !matches(&captured_weights, &weights)
            || captured_extra.len() != extra.len()
            || !captured_extra
                .iter()
                .zip(&extra)
                .all(|(a, b)| matches(a, b))
        {
            return Err("checkpoint source weights do not match settings");
        }
        let physics = &snapshot["physics"];
        let number = |value: &serde_json::Value, key: &str| {
            value[key]
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or("invalid checkpoint number")
        };
        let material = &physics["material"];
        let wetting = settings.precursor_wetting_enabled.then_some(Wetting {
            contact_angle: settings.contact_angle_rad,
            precursor_thickness: settings.precursor_thickness_m,
        });
        let expected_wetting=wetting.map(|w|serde_json::json!({"contactAngleRad":w.contact_angle,"precursorThicknessM":w.precursor_thickness})).unwrap_or(serde_json::Value::Null);
        if number(material, "density")? != settings.material.density
            || number(material, "viscosity")? != settings.material.viscosity
            || number(material, "surfaceTension")? != settings.material.surface_tension
            || number(material, "wetting")? != settings.material.wetting
            || physics["precursorWetting"] != expected_wetting
        {
            return Err("checkpoint physics/settings mismatch");
        }
        let points = serde_json::from_value(physics["pointsM"].clone())
            .map_err(|_| "invalid checkpoint points")?;
        let triangles = serde_json::from_value(physics["triangles"].clone())
            .map_err(|_| "invalid checkpoint triangles")?;
        let volumes = serde_json::from_value(physics["cellVolumesM3"].clone())
            .map_err(|_| "invalid checkpoint volumes")?;
        let state = SurfaceFilmState {
            points,
            triangles,
            cell_volumes_m3: volumes,
            material: settings.material,
            wetting,
        };
        if state.points.len() != representatives.len()
            || state.triangles != self.film.state().triangles
        {
            return Err("checkpoint topology mismatch");
        }
        let film = SurfaceFilm::from_state(&state)?;
        let measurements = &snapshot["measurements"];
        let initial = number(measurements, "initialMassKg")?;
        let added = number(measurements, "sourceAddedMassKg")?;
        let captured_mass = number(measurements, "massKg")?;
        let gross = number(&measurements["selfContact"], "grossTransferredVolumeM3")?;
        let last = number(&measurements["selfContact"], "lastTransferredVolumeM3")?;
        if [initial, added, captured_mass, gross, last]
            .iter()
            .any(|v| *v < 0.)
            || last > gross
            || !measurements["selfContact"]["error"].is_null()
        {
            return Err("invalid checkpoint history");
        }
        let tolerance = 1e-12 * film.total_mass().max(f64::MIN_POSITIVE);
        if (captured_mass - film.total_mass()).abs() > tolerance
            || (initial + added - film.total_mass()).abs() > tolerance
        {
            return Err("checkpoint mass/history mismatch");
        }
        self.film = film;
        self.settings = settings;
        self.source_weights = captured_weights;
        self.extra_source_weights = captured_extra;
        self.initial_mass_kg = initial;
        self.source_added_mass_kg = added;
        self.self_contact_gross_volume_m3 = gross;
        self.self_contact_last_volume_m3 = last;
        self.self_contact_error = None;
        Ok(())
    }
    /// Check source-adjusted conservation; contact transfer stays within this film.
    pub(crate) fn verify_mass_balance(&self, relative_tolerance: f64) -> Result<f64, &'static str> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0. {
            return Err("invalid film mass tolerance");
        }
        let mass = self.film.total_mass();
        let expected = self.initial_mass_kg + self.source_added_mass_kg;
        if !mass.is_finite()
            || mass < 0.
            || !expected.is_finite()
            || expected < 0.
            || self.self_contact_error.is_some()
        {
            return Err("invalid film mass or failed contact state");
        }
        let error = (mass - expected).abs() / expected.max(1e-12);
        if error > relative_tolerance {
            return Err("film source-adjusted mass is not conserved");
        }
        Ok(error)
    }
    pub fn measurements(&self) -> serde_json::Value {
        let pressure = self.film.driving_pressure([0., -9.81, 0.]);
        let range = pressure.as_ref().ok().map(|p| {
            p.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |a, &p| {
                [a[0].min(p), a[1].max(p)]
            })
        });
        serde_json::json!({"selfContact":{"enabled":self.settings.self_contact_enabled,"grossTransferredVolumeM3":self.self_contact_gross_volume_m3,"lastTransferredVolumeM3":self.self_contact_last_volume_m3,"error":self.self_contact_error,"calibratedPhysiology":false},"volumeM3":self.film.total_volume(),"massKg":self.film.total_mass(),
            "drivingPotentialRangePa":range,"drivingPotentialError":pressure.err(),"drivingPotentialIncludesGravity":true,"initialMassKg":self.initial_mass_kg,"sourceAddedMassKg":self.source_added_mass_kg,
            "maximumCellThicknessM":self.film.thickness().into_iter().fold(0.,f64::max),
            "settings":self.settings.to_json(),"calibratedPhysiology":false})
    }
    pub(crate) fn update_substrate(
        &mut self,
        vertices: &[SceneVertex],
    ) -> Result<(), &'static str> {
        let points = self
            .representatives
            .iter()
            .map(|&i| {
                vertices
                    .get(i)
                    .map(|v| v.position.map(f64::from))
                    .ok_or("film substrate lacks bound vertices")
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.film.update_geometry(&points)
    }
    pub fn advance(&mut self, vertices: &[SceneVertex], dt: f64) -> Result<(), &'static str> {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return Err("film preview timestep must be finite and within (0, 0.1] seconds");
        }
        let points = self
            .representatives
            .iter()
            .map(|&i| {
                vertices
                    .get(i)
                    .map(|v| v.position.map(f64::from))
                    .ok_or("film substrate lacks bound vertices")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut sources: Vec<_> = self
            .source_weights
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > 0. && self.settings.source_rate_m3_s > 0.)
            .map(|(cell, w)| (cell, w * self.settings.source_rate_m3_s))
            .collect();
        for (weights, source) in self.extra_source_weights.iter().zip(&self.settings.sources) {
            sources.extend(
                weights
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| **w > 0. && source.rate_m3_s > 0.)
                    .map(|(cell, w)| (cell, w * source.rate_m3_s)),
            );
        }
        let contact =
            self.settings
                .self_contact_enabled
                .then_some(physics::surface_film::BridgeConfig {
                    max_gap: self.settings.self_contact_max_gap_m,
                    transfer_speed: self.settings.self_contact_transfer_speed_m_s,
                    ..Default::default()
                });
        // Stage the film and its receipts together; neither publishes on admission failure.
        self.verify_mass_balance(1e-9)?;
        let initial = self.initial_mass_kg;
        let old_added = self.source_added_mass_kg;
        let old_gross = self.self_contact_gross_volume_m3;
        let density = self.settings.material.density;
        let (source_added_mass, gross_volume, transferred) =
            self.film.advance_on_geometry_with_contact_admitted(
                &points,
                dt,
                &sources,
                [0., -9.81, 0.],
                contact,
                |candidate, added, transferred| {
                    let source_added_mass = old_added + added * density;
                    let gross_volume = old_gross + transferred;
                    let expected = initial + source_added_mass;
                    let mass = candidate.total_mass();
                    if !source_added_mass.is_finite()
                        || !gross_volume.is_finite()
                        || !transferred.is_finite()
                        || transferred < 0.
                        || !expected.is_finite()
                        || !mass.is_finite()
                        || (mass - expected).abs() > 1e-9 * expected.max(1e-12)
                    {
                        return Err("film source/contact receipt admission failed");
                    }
                    Ok((source_added_mass, gross_volume, transferred))
                },
            )?;
        self.source_added_mass_kg = source_added_mass;
        self.self_contact_last_volume_m3 = transferred;
        self.self_contact_gross_volume_m3 = gross_volume;
        self.self_contact_error = None;
        Ok(())
    }
    pub fn apply(&self, vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>, _eye: Vec3) {
        let height = self
            .film
            .vertex_thickness(self.representatives.len())
            .expect("valid welded mapping");
        let source_count = self.render_to_solver.len();
        let mut normals = vec![Vec3::ZERO; self.representatives.len()];
        for t in indices.chunks_exact(3) {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            if [a, b, c].iter().any(|&i| i >= source_count) {
                continue;
            }
            let n = (Vec3::from_array(vertices[b].position)
                - Vec3::from_array(vertices[a].position))
            .cross(Vec3::from_array(vertices[c].position) - Vec3::from_array(vertices[a].position));
            for i in [a, b, c] {
                normals[self.render_to_solver[i]] += n;
            }
        }
        let base = vertices.len() as u32;
        // Keep a fixed layer topology. Dry fragments discard in the shader;
        // heights in metres are interpolated independently of skin colour.
        for (i, &id) in self.render_to_solver.iter().enumerate() {
            let mut layer = vertices[i];
            let thickness = height[id] as f32;
            let n = normals[id].normalize_or_zero();
            layer.position =
                (Vec3::from_array(layer.position) + n * (thickness + 0.00001)).to_array();
            layer.uv = [
                if self.settings.thickness_view {
                    -3.
                } else {
                    -2.
                },
                thickness,
            ];
            vertices.push(layer);
        }
        let source_indices: Vec<_> = indices
            .chunks_exact(3)
            .filter(|t| t.iter().all(|&i| (i as usize) < source_count))
            .flat_map(|t| t.iter().map(|&i| base + i))
            .collect();
        indices.extend(source_indices);
    }
}

// Integrate the brush over reference area, rather than counting triangles.
// Seven-point degree-five quadrature is exact for the quartic kernel when the
// triangle is fully inside the brush; clipped boundary triangles converge
// under refinement. The footprint remains attached to canonical tissue cells.
fn source_weights(
    triangles: &[[[f64; 3]; 3]],
    center: [f64; 3],
    radius: f64,
) -> Result<Vec<f64>, &'static str> {
    let quadrature = [
        ([1. / 3.; 3], 0.225),
        (
            [0.059715871789770, 0.470142064105115, 0.470142064105115],
            0.132394152788506,
        ),
        (
            [0.470142064105115, 0.059715871789770, 0.470142064105115],
            0.132394152788506,
        ),
        (
            [0.470142064105115, 0.470142064105115, 0.059715871789770],
            0.132394152788506,
        ),
        (
            [0.797426985353087, 0.101286507323456, 0.101286507323456],
            0.125939180544827,
        ),
        (
            [0.101286507323456, 0.797426985353087, 0.101286507323456],
            0.125939180544827,
        ),
        (
            [0.101286507323456, 0.101286507323456, 0.797426985353087],
            0.125939180544827,
        ),
    ];
    let mut weights: Vec<_> = triangles
        .iter()
        .map(|t| {
            let a: [f64; 3] = std::array::from_fn(|k| t[1][k] - t[0][k]);
            let b: [f64; 3] = std::array::from_fn(|k| t[2][k] - t[0][k]);
            let cross = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let area = 0.5 * cross.iter().map(|v| v * v).sum::<f64>().sqrt();
            area * quadrature
                .iter()
                .map(|(bary, weight)| {
                    let d: f64 = (0..3)
                        .map(|k| {
                            let p: f64 = (0..3).map(|i| bary[i] * t[i][k]).sum();
                            (p - center[k]).powi(2)
                        })
                        .sum();
                    weight * (1. - d / (radius * radius)).max(0.).powi(2)
                })
                .sum::<f64>()
        })
        .collect();
    let sum: f64 = weights.iter().sum();
    if sum <= 0. || !sum.is_finite() {
        return Err("film source misses surface");
    }
    for w in &mut weights {
        *w /= sum;
    }
    Ok(weights)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_contains_current_geometry_distribution_and_source_history() {
        let mut vertices: Vec<_> = [
            [0., 0., 0.],
            [0.02, 0., 0.],
            [0.02, 0., 0.02],
            [0., 0., 0.02],
        ]
        .into_iter()
        .map(|position| SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        })
        .collect();
        let mut preview =
            FilmPreview::new(&vertices, &[0, 1, 2, 0, 2, 3], [0.01, 0., 0.01], 0.04, 5e-9).unwrap();
        let settings = preview
            .settings
            .patched(
                &serde_json::json!({"surface_tension":0.,"wetting":0.,"source_rate_m3_s":1e-9}),
            )
            .unwrap();
        preview.configure(settings).unwrap();
        vertices[2].position[1] = 0.002;
        preview.advance(&vertices, 0.01).unwrap();
        assert!(preview.verify_mass_balance(1e-9).unwrap() < 1e-9);
        let snapshot = preview.state_snapshot();
        let volumes: Vec<f64> =
            serde_json::from_value(snapshot["physics"]["cellVolumesM3"].clone()).unwrap();
        assert_eq!(volumes, preview.film.state().cell_volumes_m3);
        let points: Vec<[f64; 3]> =
            serde_json::from_value(snapshot["physics"]["pointsM"].clone()).unwrap();
        assert_eq!(points, preview.film.state().points);
        assert!(points.iter().any(|p| p[1] > 0.001));
        assert!(
            snapshot["measurements"]["sourceAddedMassKg"]
                .as_f64()
                .unwrap()
                > 0.
        );
        assert_eq!(
            snapshot["measurements"]["massKg"].as_f64().unwrap(),
            preview.film.total_mass()
        );
        assert_eq!(
            snapshot["renderToSolver"].as_array().unwrap().len(),
            vertices.len()
        );
        assert_eq!(snapshot["wholeBodyCheckpoint"], false);
        preview.source_added_mass_kg += 1e-6;
        assert!(preview.verify_mass_balance(1e-9).is_err());
        assert!(preview.verify_mass_balance(f64::NAN).is_err());
    }
    fn body_fixture() -> (Vec<SceneVertex>, Vec<u32>) {
        let source = include_str!("../../../assets/characters/blender-female/body.obj");
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for line in source.lines() {
            let mut words = line.split_whitespace();
            match words.next() {
                Some("v") => {
                    let position =
                        std::array::from_fn(|_| words.next().unwrap().parse::<f32>().unwrap());
                    vertices.push(SceneVertex {
                        position,
                        uv: [0.; 2],
                        color: [1.; 4],
                    });
                }
                Some("f") => {
                    let face: Vec<u32> = words
                        .map(|w| w.split('/').next().unwrap().parse::<u32>().unwrap() - 1)
                        .collect();
                    assert_eq!(face.len(), 3, "body fixture must be triangulated");
                    indices.extend(face);
                }
                _ => {}
            }
        }
        (vertices, indices)
    }

    #[test]
    fn body_morph_surface_area_quality_matrix() {
        let (vertices, indices) = body_fixture();
        let mut report = Vec::new();
        for patch in [
            serde_json::json!({}),
            serde_json::json!({"height_cm":130,"weight_kg":35}),
            serde_json::json!({"height_cm":210,"weight_kg":180}),
            serde_json::json!({"torso_depth":0.6,"waist_width":0.6,"breast_size":1.6,"buttock_size":1.6}),
            serde_json::json!({"torso_depth":1.6,"waist_width":1.6,"breast_size":0.6,"buttock_size":0.6}),
            serde_json::json!({"left_breast_size":0.6,"right_breast_size":1.6,"left_buttock_size":1.6,"right_buttock_size":0.6}),
        ] {
            let p = crate::body_parameters::BodyParameters::default()
                .patched(&patch)
                .unwrap();
            let result = p.surface_quality(&vertices, &indices).unwrap();
            if p == Default::default() {
                assert_eq!(result["minimumAreaRatio"], 1.);
                assert_eq!(result["maximumAreaRatio"], 1.);
            }
            report.push(serde_json::json!({"parameters":patch,"quality":result}));
        }
        let mut invalid = vertices.clone();
        invalid[indices[1] as usize].position = invalid[indices[0] as usize].position;
        assert!(
            crate::body_parameters::BodyParameters::default()
                .surface_quality(&invalid, &indices)
                .is_err()
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/body-morph-surface-quality.json");
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    #[test]
    #[ignore = "expensive full-body spatial refinement study"]
    fn body_gravity_transport_spatial_refinement() {
        let (vertices, indices) = body_fixture();
        let preview = FilmPreview::new(&vertices, &indices, [0., 0.2, 0.13], 0.04, 0.).unwrap();
        let base_count = preview.canonical_triangles.len();
        let mut triangles = preview.canonical_triangles;
        let mut runs = Vec::new();
        let mut report = Vec::new();
        for level in 0..3 {
            if level > 0 {
                triangles = triangles
                    .iter()
                    .flat_map(|&[a, b, c]| {
                        let ab = std::array::from_fn(|k| (a[k] + b[k]) * 0.5);
                        let bc = std::array::from_fn(|k| (b[k] + c[k]) * 0.5);
                        let ca = std::array::from_fn(|k| (c[k] + a[k]) * 0.5);
                        [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]
                    })
                    .collect();
            }
            let mut lookup = std::collections::HashMap::new();
            let mut points = Vec::new();
            let cells = triangles
                .iter()
                .map(|t| {
                    t.map(|p| {
                        *lookup.entry(p.map(f64::to_bits)).or_insert_with(|| {
                            let id = points.len();
                            points.push(p);
                            id
                        })
                    })
                })
                .collect();
            let mut film = SurfaceFilm::new(
                &points,
                cells,
                Material {
                    viscosity: 0.001,
                    surface_tension: 0.,
                    wetting: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
            let weights = source_weights(&triangles, [0., 0.2, 0.13], 0.04).unwrap();
            let volume = 5e-7;
            for (i, w) in weights.iter().enumerate() {
                film.deposit(i, w * volume).unwrap();
            }
            let areas: Vec<_> = triangles
                .iter()
                .map(|[a, b, c]| {
                    let a = glam::DVec3::from_array(*a);
                    let b = glam::DVec3::from_array(*b);
                    let c = glam::DVec3::from_array(*c);
                    0.5 * (b - a).cross(c - a).length()
                })
                .collect();
            let initial_mass = film.total_mass();
            let start = std::time::Instant::now();
            for _ in 0..80 {
                film.step_with_max_substep(0.00025, [0., -9.81, 0.], 0.00025)
                    .unwrap();
            }
            let h = film.thickness();
            assert!(h.iter().all(|h| h.is_finite() && *h >= 0.));
            let mass_error = (film.total_mass() - initial_mass).abs() / initial_mass;
            assert!(mass_error < 1e-12);
            let children = 4usize.pow(level);
            let mut restricted = vec![0.; base_count];
            let mut initial = vec![0.; base_count];
            for (i, ((&h, &area), &weight)) in h.iter().zip(&areas).zip(&weights).enumerate() {
                restricted[i / children] += h * area;
                initial[i / children] += weight * volume;
            }
            let change = restricted
                .iter()
                .zip(&initial)
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>()
                / volume;
            assert!(change > 0.01, "insufficient transport {change}");
            report.push(serde_json::json!({"level":level,"triangles":triangles.len(),"relativeMassError":mass_error,"normalizedL1ChangeFromInitial":change,"wallSeconds":start.elapsed().as_secs_f64()}));
            runs.push((initial, restricted));
        }
        let distance = |a: &Vec<f64>, b: &Vec<f64>| {
            a.iter().zip(b).map(|(a, b)| (a - b).abs()).sum::<f64>() / 5e-7
        };
        let coarse = distance(&runs[0].1, &runs[2].1);
        let fine = distance(&runs[1].1, &runs[2].1);
        let result = serde_json::json!({"runs":report,"normalizedL1CoarseVsReference":coarse,"normalizedL1FineVsReference":fine,"initialCoarseVsReference":distance(&runs[0].0,&runs[2].0),"initialFineVsReference":distance(&runs[1].0,&runs[2].0),"refinementTrendPassed":fine<coarse,"simulationSeconds":0.02,"dtSeconds":0.00025,"scope":"fixed piecewise planar body; gravity only; no capillarity, precursor wetting or continuous source; restricted volume comparisons, not physiology"});
        std::fs::write(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/body-film-space-convergence.json"),
            serde_json::to_string_pretty(&result).unwrap(),
        )
        .unwrap();
        println!("{result}");
        assert!(
            fine < coarse,
            "spatial refinement failed: coarse={coarse}, fine={fine}"
        );
    }

    #[test]
    fn preview_self_contact_moves_liquid_and_stops_after_separation() {
        let vertices: Vec<_> = [0., 0.00005]
            .into_iter()
            .flat_map(|z| {
                [[0., 0., z], [0.01, 0., z], [0., 0.01, z]].map(|position| SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
            })
            .collect();
        let mut preview =
            FilmPreview::new(&vertices, &[0, 1, 2, 3, 5, 4], [0.005, 0.005, 0.], 0.01, 0.).unwrap();
        preview.film.deposit(0, 1e-8).unwrap();
        // This direct fixture deposit establishes the initial inventory.
        preview.initial_mass_kg = preview.film.total_mass();
        let mass = preview.film.total_mass();
        let settings=preview.settings.patched(&serde_json::json!({"self_contact_enabled":true,"self_contact_transfer_speed_m_s":0.01})).unwrap();
        preview.configure(settings).unwrap();
        preview.advance(&vertices, 0.001).unwrap();
        assert!(preview.film.thickness()[1] > 0.);
        assert!(preview.self_contact_last_volume_m3 > 0.);
        assert!(preview.self_contact_error.is_none());
        assert!((preview.film.total_mass() - mass).abs() < mass * 1e-12);
        let cumulative = preview.self_contact_gross_volume_m3;
        let mut separated = vertices.clone();
        for v in &mut separated[3..] {
            v.position[2] += 0.002;
        }
        preview.advance(&separated, 0.001).unwrap();
        assert_eq!(preview.self_contact_last_volume_m3, 0.);
        assert_eq!(preview.self_contact_gross_volume_m3, cumulative);
        let invalid = preview
            .settings
            .patched(&serde_json::json!({"self_contact_max_gap_m":0}));
        assert!(invalid.is_err());
        let settings = preview
            .settings
            .patched(&serde_json::json!({"self_contact_enabled":false}))
            .unwrap();
        preview.configure(settings).unwrap();
        preview.advance(&vertices, 0.001).unwrap();
        assert_eq!(preview.self_contact_last_volume_m3, 0.);
        assert_eq!(preview.self_contact_gross_volume_m3, cumulative);
        assert_eq!(preview.measurements()["selfContact"]["enabled"], false);
    }

    #[test]
    fn configured_contact_angle_changes_potential_without_seeding_mass() {
        let vertices = vec![
            SceneVertex {
                position: [0., 0., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0.1, 0., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 0.1, 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
        ];
        let mut preview = FilmPreview::new(&vertices, &[0, 1, 2], [0., 0., 0.], 0.2, 5e-8).unwrap();
        let initial = preview.film.total_mass();
        let before = preview.film.driving_pressure([0.; 3]).unwrap();
        let settings=preview.settings.patched(&serde_json::json!({"precursor_wetting_enabled":true,"contact_angle_rad":0.2,"precursor_thickness_m":1e-6})).unwrap();
        preview.configure(settings.clone()).unwrap();
        let after = preview.film.driving_pressure([0.; 3]).unwrap();
        assert!(after[0] > before[0]);
        assert_eq!(preview.film.total_mass(), initial);
        assert!(
            settings
                .patched(&serde_json::json!({"contact_angle_rad":0.6,"density":2000}))
                .is_err()
        );
        assert_eq!(preview.film.material().density, 1000.);
        let off = settings
            .patched(&serde_json::json!({"precursor_wetting_enabled":false}))
            .unwrap();
        preview.configure(off).unwrap();
        assert_eq!(preview.film.driving_pressure([0.; 3]).unwrap(), before);
        assert_eq!(preview.film.total_mass(), initial);
    }

    #[test]
    #[ignore = "full body moving-surface source mass-budget study"]
    fn body_deformation_with_sources_preserves_mass_budget() {
        let (canonical, indices) = body_fixture();
        let mut preview =
            FilmPreview::new(&canonical, &indices, [0., 0.2, 0.13], 0.04, 5e-8).unwrap();
        let settings=preview.settings.patched(&serde_json::json!({
            "source_rate_m3_s":1e-8,
            "sources":[{"id":"second","x_m":0.03,"y_m":0.24,"z_m":0.13,"radius_m":0.05,"rate_m3_s":2e-8}]
        })).unwrap();
        preview.configure(settings).unwrap();
        let initial = preview.film.total_mass();
        let mut maximum_relative_error = 0_f64;
        let mut max_displacement = 0_f64;
        let start = std::time::Instant::now();
        let dt = 0.001;
        for step in 0..40 {
            let phase = std::f64::consts::TAU * (step + 1) as f64 / 40.;
            let parameters = crate::body_parameters::BodyParameters::default()
                .patched(&serde_json::json!({
                    "height_cm":164.+20.*phase.sin(),"torso_depth":1.+0.2*phase.sin(),
                    "waist_width":1.+0.1*phase.sin(),"weight_kg":60.+10.*phase.sin()
                }))
                .unwrap();
            let mut moving = canonical.clone();
            parameters.apply(&mut moving);
            for (a, b) in moving.iter().zip(&canonical) {
                let d =
                    glam::Vec3::from_array(a.position).distance(glam::Vec3::from_array(b.position));
                max_displacement = max_displacement.max(f64::from(d));
            }
            if step == 20 {
                let settings = preview
                    .settings
                    .patched(&serde_json::json!({"density":2000}))
                    .unwrap();
                let before = preview.film.total_mass();
                preview.configure(settings).unwrap();
                assert!((preview.film.total_mass() - before).abs() < 1e-16);
            }
            preview.advance(&moving, dt).unwrap();
            let expected = initial + preview.source_added_mass_kg;
            maximum_relative_error =
                maximum_relative_error.max((preview.film.total_mass() - expected).abs() / expected);
            assert!(
                preview
                    .film
                    .thickness()
                    .iter()
                    .all(|h| h.is_finite() && *h >= 0.)
            );
            assert!(maximum_relative_error < 1e-12);
        }
        let expected_input = 3e-8 * dt * (20. * 1000. + 20. * 2000.);
        assert!((preview.source_added_mass_kg - expected_input).abs() < 1e-16);
        assert!(max_displacement > 0.1);
        let report = serde_json::json!({"triangles":indices.len()/3,"steps":40,"dtSeconds":dt,
            "maximumDisplacementM":max_displacement,"maximumRelativeMassError":maximum_relative_error,
            "sourceAddedMassKg":preview.source_added_mass_kg,"expectedSourceAddedMassKg":expected_input,
            "finalMeasurements":preview.measurements(),"wallSeconds":start.elapsed().as_secs_f64(),
            "scope":"prescribed morph deformation, no inertial entrainment or two-way tissue force; uncalibrated engineering fixture"});
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/body-film-deformation-budget.json");
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        println!("{report}");
    }

    #[test]
    #[ignore = "full body temporal convergence study; writes measured report"]
    fn body_mesh_temporal_convergence_and_mass_budget() {
        let (vertices, indices) = body_fixture();
        let area: Vec<f64> = indices
            .chunks_exact(3)
            .map(|t| {
                let p = t
                    .iter()
                    .map(|&i| glam::DVec3::from_array(vertices[i as usize].position.map(f64::from)))
                    .collect::<Vec<_>>();
                0.5 * (p[1] - p[0]).cross(p[2] - p[0]).length()
            })
            .collect();
        let mut runs = Vec::new();
        let mut report = Vec::new();
        for dt in [0.001, 0.0005, 0.00025] {
            let mut preview =
                FilmPreview::new(&vertices, &indices, [0., 0.2, 0.13], 0.04, 5e-7).unwrap();
            let mut material = preview.film.material();
            material.viscosity = 0.001;
            preview.film.set_material(material).unwrap();
            let initial_height = preview.film.thickness();
            let initial = preview.film.total_mass();
            let steps = (0.05_f64 / dt).round() as usize;
            let start = std::time::Instant::now();
            for _ in 0..steps {
                preview
                    .film
                    .step_with_max_substep(dt, [0., -9.81, 0.], dt)
                    .unwrap();
            }
            let h = preview.film.thickness();
            assert!(h.iter().all(|v| v.is_finite() && *v >= 0.));
            let mass_error = (preview.film.total_mass() - initial).abs() / initial;
            assert!(mass_error < 1e-12, "mass error {mass_error}");
            report.push(serde_json::json!({"dtSeconds":dt,"steps":steps,"wallSeconds":start.elapsed().as_secs_f64(),"relativeMassError":mass_error,"normalizedL1ChangeFromInitial":h.iter().zip(&initial_height).zip(&area).map(|((&a,&b),&area)|(a-b).abs()*area).sum::<f64>()/5e-7,"maximumThicknessM":h.iter().copied().fold(0.,f64::max)}));
            let change = h
                .iter()
                .zip(&initial_height)
                .zip(&area)
                .map(|((&a, &b), &area)| (a - b).abs() * area)
                .sum::<f64>()
                / 5e-7;
            assert!(change > 0.1, "fixture has insufficient transport: {change}");
            runs.push(h);
        }
        let distance = |a: &Vec<f64>, b: &Vec<f64>| {
            a.iter()
                .zip(b)
                .zip(&area)
                .map(|((&a, &b), &area)| (a - b).abs() * area)
                .sum::<f64>()
                / 5e-7
        };
        let coarse = distance(&runs[0], &runs[2]);
        let fine = distance(&runs[1], &runs[2]);
        let result = serde_json::json!({"fixture":"assets/characters/blender-female/body.obj","triangles":area.len(),"simulationSeconds":0.05,"volumeM3":5e-7,"viscosityPaS":0.001,"calibratedPhysiology":false,"normalizedL1CoarseVsReference":coarse,"normalizedL1FineVsReference":fine,"runs":report,"scope":"static geometry, primary initial deposit, no continuous sources or tissue coupling"});
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/body-film-time-convergence.json");
        std::fs::write(path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        println!("{result}");
        assert!(
            fine < coarse,
            "time refinement failed: coarse={coarse}, fine={fine}"
        );
        assert!(
            fine < 0.002,
            "fine/reference L1 exceeds 0.2 percent engineering gate: {fine}"
        );
    }

    #[test]
    fn clipped_brush_distribution_converges_under_refinement() {
        let fraction = |level| {
            let mut halves = vec![
                vec![[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]],
                vec![[[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]],
            ];
            for _ in 0..level {
                for half in &mut halves {
                    *half = half
                        .iter()
                        .flat_map(|t| {
                            let ab = std::array::from_fn(|k| (t[0][k] + t[1][k]) * 0.5);
                            let bc = std::array::from_fn(|k| (t[1][k] + t[2][k]) * 0.5);
                            let ca = std::array::from_fn(|k| (t[2][k] + t[0][k]) * 0.5);
                            [[t[0], ab, ca], [ab, t[1], bc], [ca, bc, t[2]], [ab, bc, ca]]
                        })
                        .collect();
                }
            }
            let count = halves[0].len();
            let all: Vec<_> = halves.into_iter().flatten().collect();
            source_weights(&all, [0.35, 0.4, 0.], 0.3).unwrap()[..count]
                .iter()
                .sum::<f64>()
        };
        let reference = fraction(6);
        let coarse_error = (fraction(2) - reference).abs();
        let fine_error = (fraction(4) - reference).abs();
        assert!(
            fine_error < coarse_error,
            "coarse={coarse_error}, fine={fine_error}"
        );
        assert!(fine_error < 1e-5, "fine={fine_error}");
    }

    #[test]
    fn source_distribution_is_invariant_under_local_refinement() {
        let first = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let second = [[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
        let subdivide = |t: [[f64; 3]; 3]| {
            let ab = std::array::from_fn(|k| (t[0][k] + t[1][k]) * 0.5);
            let bc = std::array::from_fn(|k| (t[1][k] + t[2][k]) * 0.5);
            let ca = std::array::from_fn(|k| (t[2][k] + t[0][k]) * 0.5);
            [[t[0], ab, ca], [ab, t[1], bc], [ca, bc, t[2]], [ab, bc, ca]]
        };
        let coarse = source_weights(&[first, second], [0.2, 0.3, 0.], 2.).unwrap();
        let mut refined = subdivide(first).to_vec();
        refined.push(second);
        let fine = source_weights(&refined, [0.2, 0.3, 0.], 2.).unwrap();
        assert!((coarse[0] - fine[..4].iter().sum::<f64>()).abs() < 1e-14);
        assert!((coarse[1] - fine[4]).abs() < 1e-14);
        assert!((fine.iter().sum::<f64>() - 1.).abs() < 1e-14);
    }

    #[test]
    fn independent_sources_add_the_sum_of_their_mass_inputs() {
        let v = |position| SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = vec![
            v([0., 0., 0.]),
            v([0.1, 0., 0.]),
            v([0., 0.1, 0.]),
            v([1., 0., 0.]),
            v([1.1, 0., 0.]),
            v([1., 0.1, 0.]),
        ];
        let mut preview =
            FilmPreview::new(&vertices, &[0, 1, 2, 3, 4, 5], [0., 0., 0.], 0.2, 5e-8).unwrap();
        let settings = preview
            .settings
            .patched(&serde_json::json!({"sources":[
            {"id":"first","x_m":0.03,"y_m":0.03,"z_m":0,"radius_m":0.1,"rate_m3_s":1e-8},
            {"id":"second","x_m":1.03,"y_m":0.03,"z_m":0,"radius_m":0.1,"rate_m3_s":2e-8}]}))
            .unwrap();
        preview.configure(settings).unwrap();
        let mass = preview.film.total_mass();
        preview.advance(&vertices, 0.001).unwrap();
        assert!((preview.film.total_mass() - mass - 3e-8 * 0.001 * 1000.).abs() < 1e-15);
        assert_eq!(preview.extra_source_weights[0], [1., 0.]);
        assert_eq!(preview.extra_source_weights[1], [0., 1.]);
        let settings = preview
            .settings
            .patched(&serde_json::json!({"sources":[]}))
            .unwrap();
        preview.configure(settings).unwrap();
        let mass = preview.film.total_mass();
        preview.advance(&vertices, 0.001).unwrap();
        assert_eq!(preview.film.total_mass(), mass);
    }

    #[test]
    fn moving_source_retains_existing_liquid_and_changes_only_new_input() {
        let v = |position| SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = vec![
            v([0., 0., 0.]),
            v([0.1, 0., 0.]),
            v([0., 0.1, 0.]),
            v([1., 0., 0.]),
            v([1.1, 0., 0.]),
            v([1., 0.1, 0.]),
        ];
        let mut preview =
            FilmPreview::new(&vertices, &[0, 1, 2, 3, 4, 5], [0., 0., 0.], 0.2, 5e-8).unwrap();
        let before = preview.film.total_mass();
        let settings=crate::film_settings::FilmSettings::default().patched(&serde_json::json!({"source_x_m":1.03,"source_y_m":0.03,"source_z_m":0,"source_radius_m":0.1,"source_rate_m3_s":1e-8})).unwrap();
        preview.configure(settings.clone()).unwrap();
        assert_eq!(preview.film.total_mass(), before);
        assert_eq!(preview.source_weights[0], 0.);
        assert_eq!(preview.source_weights[1], 1.);
        preview.advance(&vertices, 0.001).unwrap();
        assert!((preview.film.total_mass() - before - 1e-8 * 0.001 * 1000.).abs() < 1e-15);
        let current = preview.film.total_mass();
        let invalid=settings.patched(&serde_json::json!({"source_x_m":2,"source_y_m":2,"source_z_m":2,"source_radius_m":0.001,"density":2000})).unwrap();
        assert!(preview.configure(invalid).is_err());
        assert_eq!(preview.film.total_mass(), current);
        assert_eq!(preview.film.material().density, 1000.);
    }

    #[test]
    fn layer_uses_solver_thickness_and_preserves_substrate() {
        let mut vertices = vec![
            SceneVertex {
                position: [0., 0., 0.],
                uv: [0., 0.],
                color: [0.5, 0.4, 0.3, 1.],
            },
            SceneVertex {
                position: [1., 0., 0.],
                uv: [0., 0.],
                color: [0.5, 0.4, 0.3, 1.],
            },
            SceneVertex {
                position: [0., 1., 0.],
                uv: [0., 0.],
                color: [0.5, 0.4, 0.3, 1.],
            },
        ];
        let source = vertices.clone();
        let mut indices = vec![0, 1, 2];
        let film = FilmPreview::new(&vertices, &indices, [0., 0., 0.], 2., 0.00005).unwrap();
        let volume = film.film.total_volume();
        assert!((film.measurements()["massKg"].as_f64().unwrap() - 0.05).abs() < 1e-12);
        assert!(
            (film.measurements()["maximumCellThicknessM"]
                .as_f64()
                .unwrap()
                - 0.0001)
                .abs()
                < 1e-12
        );
        film.apply(&mut vertices, &mut indices, Vec3::Z);
        assert_eq!(indices, [0, 1, 2, 3, 4, 5]);
        for i in 0..3 {
            assert_eq!(vertices[i].position, source[i].position);
            assert_eq!(vertices[i].color, source[i].color);
            assert!((vertices[i + 3].uv[1] - 0.0001).abs() < 1e-10);
            assert!((vertices[i + 3].position[2] - 0.00011).abs() < 1e-10);
        }
        assert_eq!(film.film.total_volume(), volume);
    }
}

#[cfg(test)]
mod body_remap_tests {
    use physics::surface_film::{Material, SurfaceFilm};
    fn film(source: &str) -> SurfaceFilm {
        let asset =
            voxy_render::ObjAsset::parse(source, voxy_render::ObjLimits::default()).unwrap();
        let points: Vec<_> = asset
            .mesh
            .vertices()
            .iter()
            .map(|v| v.position.map(f64::from))
            .collect();
        let triangles = asset
            .mesh
            .indices()
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect();
        SurfaceFilm::new(&points, triangles, Material::default()).unwrap()
    }
    #[test]
    fn actual_body_correspondence_preserves_mass_in_both_directions() {
        let female = include_str!(
            "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
        );
        let male = include_str!("../../../assets/characters/blender-male/body-refined.obj");
        for (source, target, map) in [
            (
                female,
                male,
                include_str!("../../../assets/characters/female-to-male-film-map.json"),
            ),
            (
                male,
                female,
                include_str!("../../../assets/characters/male-to-female-film-map.json"),
            ),
        ] {
            let mapping: serde_json::Value = serde_json::from_str(map).unwrap();
            let rows: Vec<Vec<(usize, f64)>> =
                serde_json::from_value(mapping["distribution"].clone()).unwrap();
            let mut source = film(source);
            for i in 0..rows.len() {
                source
                    .deposit(i, (1.1 + (i as f64 * 0.17).sin()) * 1e-10)
                    .unwrap();
            }
            let asset =
                voxy_render::ObjAsset::parse(target, voxy_render::ObjLimits::default()).unwrap();
            let points: Vec<_> = asset
                .mesh
                .vertices()
                .iter()
                .map(|v| v.position.map(f64::from))
                .collect();
            let triangles = asset
                .mesh
                .indices()
                .chunks_exact(3)
                .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
                .collect();
            let mass = source.total_mass();
            let result = source.remapped(&points, triangles, &rows).unwrap();
            let error = (result.total_mass() - mass).abs() / mass;
            assert!(error < 1e-12, "mass error {error}");
            assert!(result.thickness().iter().all(|h| h.is_finite() && *h >= 0.));
            assert_eq!(source.total_mass(), mass);
            println!("actual body remap mass relative error: {error}");
        }
    }
    #[test]
    fn repaired_body_correspondence_runs_in_actual_solver() {
        let original = include_str!("../../../assets/characters/blender-male/body-refined.obj");
        let repaired = include_str!(
            "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
        );
        let female = include_str!(
            "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
        );
        for (source, target, map) in [
            (
                original,
                repaired,
                include_str!("../../../assets/characters/male-original-to-repaired-film-map.json"),
            ),
            (
                repaired,
                original,
                include_str!("../../../assets/characters/male-repaired-to-original-film-map.json"),
            ),
            (
                female,
                repaired,
                include_str!("../../../assets/characters/female-to-repaired-male-film-map.json"),
            ),
            (
                repaired,
                female,
                include_str!("../../../assets/characters/repaired-male-to-female-film-map.json"),
            ),
        ] {
            let mapping: serde_json::Value = serde_json::from_str(map).unwrap();
            let rows: Vec<Vec<(usize, f64)>> =
                serde_json::from_value(mapping["distribution"].clone()).unwrap();
            let mut source = film(source);
            source
                .set_material(Material {
                    surface_tension: 0.,
                    wetting: 0.,
                    ..Default::default()
                })
                .unwrap();
            for i in 0..rows.len() {
                source
                    .deposit(i, (1.1 + (i as f64 * 0.17).sin()) * 1e-10)
                    .unwrap();
            }
            let asset =
                voxy_render::ObjAsset::parse(target, voxy_render::ObjLimits::default()).unwrap();
            let points: Vec<_> = asset
                .mesh
                .vertices()
                .iter()
                .map(|v| v.position.map(f64::from))
                .collect();
            let triangles = asset
                .mesh
                .indices()
                .chunks_exact(3)
                .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
                .collect();
            let mass = source.total_mass();
            let mut result = source.remapped(&points, triangles, &rows).unwrap();
            let remap_error = (result.total_mass() - mass).abs() / mass;
            assert!(remap_error < 1e-12);
            result
                .advance_on_geometry(&points, 1e-6, &[], [0., -9.81, 0.])
                .unwrap();
            let solve_error = (result.total_mass() - mass).abs() / mass;
            assert!(solve_error < 1e-12);
            assert!(result.thickness().iter().all(|h| h.is_finite() && *h >= 0.));
            assert_eq!(source.total_mass(), mass);
            println!(
                "repaired remap: relative mass error {remap_error}, after gravity step {solve_error}"
            );
        }
    }
    #[test]
    fn repaired_preview_transfer_preserves_running_source_counters() {
        let original = include_str!("../../../assets/characters/blender-male/body-refined.obj");
        let repaired = include_str!(
            "../../../assets/characters/blender-male/body-repaired-render-candidate.obj"
        );
        let map =
            include_str!("../../../assets/characters/male-original-to-repaired-film-map.json");
        let old =
            voxy_render::ObjAsset::parse(original, voxy_render::ObjLimits::default()).unwrap();
        let new =
            voxy_render::ObjAsset::parse(repaired, voxy_render::ObjLimits::default()).unwrap();
        let center = old.mesh.vertices()[0].position.map(f64::from);
        let mut source =
            super::FilmPreview::new(old.mesh.vertices(), old.mesh.indices(), center, 0.02, 1e-8)
                .unwrap();
        let mut settings = source.settings.clone();
        settings.material.surface_tension = 0.;
        settings.material.wetting = 0.;
        settings.source_rate_m3_s = 1e-9;
        source.configure(settings).unwrap();
        source.advance(old.mesh.vertices(), 1e-6).unwrap();
        let mass = source.film.total_mass();
        let added = source.source_added_mass_kg;
        assert!(added > 0.);
        let mut result = source
            .remapped_reference(
                new.mesh.vertices(),
                new.mesh.indices(),
                original,
                repaired,
                map,
            )
            .unwrap();
        assert!((result.film.total_mass() - mass).abs() / mass < 1e-12);
        assert_eq!(result.initial_mass_kg, source.initial_mass_kg);
        assert_eq!(result.source_added_mass_kg, added);
        assert_eq!(result.settings.to_json(), source.settings.to_json());
        result.advance(new.mesh.vertices(), 1e-6).unwrap();
        assert!(result.source_added_mass_kg > added);
        assert!(
            result
                .film
                .thickness()
                .iter()
                .all(|h| h.is_finite() && *h >= 0.)
        );
        assert_eq!(source.film.total_mass(), mass);
    }
    #[test]
    fn refinement_transfer_welds_seams_and_preserves_volume_history() {
        let original = "v 0 0 0\nv 0.01 0 0\nv 0 0.01 0\nf 1 2 3\n";
        let refined =
            "v 0 0 0\nv 0.01 0 0\nv 0 0.01 0\nv 0.005 0 0\nv 0.005 0 0\nf 1 4 3\nf 5 2 3\n";
        let old =
            voxy_render::ObjAsset::parse(original, voxy_render::ObjLimits::default()).unwrap();
        let new = voxy_render::ObjAsset::parse(refined, voxy_render::ObjLimits::default()).unwrap();
        let mut source = super::FilmPreview::new(
            old.mesh.vertices(),
            old.mesh.indices(),
            [0.003, 0.003, 0.],
            0.02,
            1e-8,
        )
        .unwrap();
        let mut settings = source.settings.clone();
        settings.material.surface_tension = 0.;
        settings.material.wetting = 0.;
        settings.source_rate_m3_s = 1e-9;
        source.configure(settings).unwrap();
        source.advance(old.mesh.vertices(), 1e-6).unwrap();
        let mass = source.film.total_mass();
        let added = source.source_added_mass_kg;
        let result = source
            .remapped_reference(
                new.mesh.vertices(),
                new.mesh.indices(),
                original,
                refined,
                r#"{"distribution":[[[0,0.5],[1,0.5]]]}"#,
            )
            .unwrap();
        assert_eq!(result.canonical_triangles.len(), 2);
        assert_eq!(result.representatives.len(), 4);
        assert!(added > 0.);
        assert_eq!(result.source_added_mass_kg, added);
        assert!(result.verify_mass_balance(1e-12).unwrap() < 1e-12);
        let restored = result
            .remapped_reference(
                old.mesh.vertices(),
                old.mesh.indices(),
                refined,
                original,
                r#"{"distribution":[[[0,1.0]],[[0,1.0]]]}"#,
            )
            .unwrap();
        assert_eq!(restored.canonical_triangles.len(), 1);
        assert!((restored.film.total_mass() - mass).abs() / mass < 1e-12);
        assert_eq!(restored.source_added_mass_kg, added);
        assert_eq!(source.film.total_mass(), mass);
    }
    #[test]
    #[ignore = "requires generated unadopted refinement assets; full-model transfer check"]
    fn current_refined_body_transfer_preserves_wet_foot_source_history() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
        let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap();
        let original =
            read("assets/characters/blender-female/prepared/body-current-post-ring-candidate.obj");
        let refined = read(
            "assets/characters/blender-female/prepared/body-current-residual-refined-candidate.obj",
        );
        let map = read("assets/characters/prepared-body-current-refinement-forward-film-map.json");
        let old =
            voxy_render::ObjAsset::parse(&original, voxy_render::ObjLimits::default()).unwrap();
        let new =
            voxy_render::ObjAsset::parse(&refined, voxy_render::ObjLimits::default()).unwrap();
        let mut source = super::FilmPreview::new(
            old.mesh.vertices(),
            old.mesh.indices(),
            [0.175, -0.800, 0.100],
            0.025,
            5e-8,
        )
        .unwrap();
        let mut settings = source.settings.clone();
        settings.material.surface_tension = 0.;
        settings.material.wetting = 0.;
        settings.source_rate_m3_s = 1e-9;
        source.configure(settings).unwrap();
        source.advance(old.mesh.vertices(), 1e-6).unwrap();
        let mass = source.film.total_mass();
        let added = source.source_added_mass_kg;
        let mut result = source
            .remapped_reference(
                new.mesh.vertices(),
                new.mesh.indices(),
                &original,
                &refined,
                &map,
            )
            .unwrap();
        let relative_error = (result.film.total_mass() - mass).abs() / mass;
        assert!(relative_error < 1e-12);
        assert_eq!(result.source_added_mass_kg, added);
        assert!(added > 0.);
        assert!(result.canonical_triangles.len() > source.canonical_triangles.len());
        result.advance(new.mesh.vertices(), 1e-6).unwrap();
        assert!(result.source_added_mass_kg > added);
        assert!(result.verify_mass_balance(1e-12).unwrap() < 1e-12);
        assert_eq!(source.film.total_mass(), mass);
        println!(
            "REFINED BODY TRANSFER: {} to {} canonical cells, mass relative error {relative_error}",
            source.canonical_triangles.len(),
            result.canonical_triangles.len()
        );
    }
}

#[cfg(test)]
mod unsupported_body_remap_tests {
    #[test]
    fn unsupported_wet_surface_rejects_without_dropping_mass() {
        let vertex = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [
            vertex([0., 0., 0.13]),
            vertex([0.01, 0., 0.13]),
            vertex([0., 0.01, 0.13]),
        ];
        let source =
            super::FilmPreview::new(&vertices, &[0, 1, 2], [0., 0., 0.13], 0.1, 1e-8).unwrap();
        let target = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-male/body-refined.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mass = source.film.total_mass();
        assert!(
            source
                .remapped_body(
                    target.mesh.vertices(),
                    target.mesh.indices(),
                    crate::body_parameters::BodyModel::Female
                )
                .is_err()
        );
        assert_eq!(source.film.total_mass(), mass);
    }
}

#[cfg(test)]
mod invalid_advance_tests {
    #[test]
    fn rejected_film_substep_restores_body_hair_and_retry_matches_fresh_step() {
        let settings=crate::film_settings::FilmSettings::default();
        for secondary_only in [false,true] {
            let make=|| {
                let mut demo=crate::female_demo::FemaleDemo::new().unwrap();
                demo.secondary_only=secondary_only;
                demo.simulate_hair=true;
                demo.surface_diffusion_enabled=false;
                demo.enable_film([0.,0.20,0.13],0.04,5e-8).unwrap();
                demo
            };
            let mut demo=make();
            let positions=|demo:&crate::female_demo::FemaleDemo|demo.mesh().unwrap().vertices().iter().map(|v|v.position).collect::<Vec<_>>();
            let before=positions(&demo);
            let film=demo.film.as_mut().unwrap();
            film.settings.self_contact_enabled=true;
            film.settings.self_contact_max_gap_m=f64::NAN;
            let film_before=film.state_snapshot();
            assert!(demo.advance(1./120.).is_err());
            assert_eq!(demo.steps,0);
            assert_eq!(positions(&demo),before,"failed film left physical geometry advanced");
            assert_eq!(demo.film.as_ref().unwrap().state_snapshot(),film_before);
            let film=demo.film.as_mut().unwrap();
            film.settings.self_contact_enabled=settings.self_contact_enabled;
            film.settings.self_contact_max_gap_m=settings.self_contact_max_gap_m;
            demo.advance(0.).unwrap(); // Retry the already queued physical time.
            let mut reference=make();
            reference.advance(1./120.).unwrap();
            assert_eq!(demo.steps,reference.steps);
            assert_eq!(positions(&demo),positions(&reference),"retry did not restore hidden physical history");
            assert_eq!(demo.film.as_ref().unwrap().state_snapshot(),reference.film.as_ref().unwrap().state_snapshot());
        }
    }
    #[test]
    fn failed_contact_preserves_film_and_source_receipts() {
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [v([0., 0., 0.]), v([0.01, 0., 0.]), v([0., 0.01, 0.])];
        let mut film =
            super::FilmPreview::new(&vertices, &[0, 1, 2], [0., 0., 0.], 0.1, 1e-8).unwrap();
        film.settings.source_rate_m3_s = 1e-9;
        film.settings.self_contact_enabled = true;
        // Fault injection after source/transport: public settings validation
        // normally prevents this invalid bridge parameter.
        film.settings.self_contact_max_gap_m = f64::NAN;
        let old = film.film.state();
        let measurements = film.measurements();
        let moved = [v([0., 0., 0.]), v([0.02, 0., 0.]), v([0., 0.02, 0.])];
        assert!(film.advance(&moved, 0.01).is_err());
        assert_eq!(film.film.state().points, old.points);
        assert_eq!(film.film.state().cell_volumes_m3, old.cell_volumes_m3);
        assert_eq!(film.measurements(), measurements);
    }
    #[test]
    fn invalid_source_inventory_rejects_before_moving_substrate_or_adding_liquid() {
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [v([0., 0., 0.]), v([0.01, 0., 0.]), v([0., 0.01, 0.])];
        let moved = [v([0., 0., 0.]), v([0.02, 0., 0.]), v([0., 0.02, 0.])];
        for corruption in [0.001, f64::INFINITY, f64::NAN] {
            let mut film =
                super::FilmPreview::new(&vertices, &[0, 1, 2], [0.; 3], 0.1, 1e-8).unwrap();
            film.settings.source_rate_m3_s = 1e-9;
            film.source_added_mass_kg = corruption;
            let old = film.film.state();
            assert!(film.advance(&moved, 0.01).is_err());
            assert_eq!(film.film.state().points, old.points);
            assert_eq!(film.film.state().cell_volumes_m3, old.cell_volumes_m3);
            assert_eq!(film.source_added_mass_kg.to_bits(), corruption.to_bits());
        }
    }
    #[test]
    fn invalid_contact_receipt_rolls_back_successful_geometry_and_source_candidate() {
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [v([0., 0., 0.]), v([0.01, 0., 0.]), v([0., 0.01, 0.])];
        let moved = [v([0., 0., 0.]), v([0.02, 0., 0.]), v([0., 0.02, 0.])];
        let mut film = super::FilmPreview::new(&vertices, &[0, 1, 2], [0.; 3], 0.1, 1e-8).unwrap();
        film.settings.source_rate_m3_s = 1e-9;
        film.self_contact_gross_volume_m3 = f64::INFINITY;
        let old = film.film.state();
        let mass_receipt = film.source_added_mass_kg;
        assert_eq!(
            film.advance(&moved, 0.01),
            Err("film source/contact receipt admission failed")
        );
        assert_eq!(film.film.state().points, old.points);
        assert_eq!(film.film.state().cell_volumes_m3, old.cell_volumes_m3);
        assert_eq!(film.source_added_mass_kg, mass_receipt);
        assert_eq!(film.self_contact_gross_volume_m3, f64::INFINITY);
    }
    #[test]
    fn invalid_timestep_and_substrate_preserve_geometry_mass_and_sources() {
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [v([0., 0., 0.]), v([0.01, 0., 0.]), v([0., 0.01, 0.])];
        let mut film =
            super::FilmPreview::new(&vertices, &[0, 1, 2], [0., 0., 0.], 0.1, 1e-8).unwrap();
        let settings = film
            .settings
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9}))
            .unwrap();
        film.configure(settings).unwrap();
        let before = film.measurements();
        let moved = [v([0., 0., 0.]), v([0.02, 0., 0.]), v([0., 0.02, 0.])];
        for dt in [0., -0.01, 0.101, f64::NAN, f64::INFINITY] {
            assert!(film.advance(&moved, dt).is_err());
            assert_eq!(film.measurements(), before);
        }
        assert!(film.advance(&vertices[..2], 0.01).is_err());
        assert_eq!(film.measurements(), before);
    }
}

#[cfg(test)]
mod atomic_frame_profile {
    #[test]
    #[ignore = "paired release full-body geometry/source/transport benchmark"]
    fn actual_body_atomic_frame_profile() {
        use physics::surface_film::{Material, SurfaceFilm};
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let points: Vec<_> = asset
            .mesh
            .vertices()
            .iter()
            .map(|v| v.position.map(f64::from))
            .collect();
        let triangles: Vec<_> = asset
            .mesh
            .indices()
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect();
        let material = Material {
            surface_tension: 0.,
            wetting: 0.,
            ..Default::default()
        };
        let mut legacy = SurfaceFilm::new(&points, triangles.clone(), material).unwrap();
        let mut atomic = SurfaceFilm::new(&points, triangles.clone(), material).unwrap();
        for i in 0..triangles.len() {
            legacy.deposit(i, 1e-10).unwrap();
            atomic.deposit(i, 1e-10).unwrap();
        }
        let mut legacy_samples = Vec::new();
        let mut atomic_samples = Vec::new();
        for round in 0..12 {
            let posed: Vec<_> = points
                .iter()
                .map(|p| {
                    [
                        p[0],
                        p[1],
                        p[2] + 0.001 * (p[1] * 10. + round as f64 * 0.1).sin(),
                    ]
                })
                .collect();
            let run_legacy = |film: &mut SurfaceFilm| {
                let start = std::time::Instant::now();
                film.update_geometry(&posed).unwrap();
                film.add_sources(0.0001, &[(0, 1e-9)]).unwrap();
                film.step(0.0001, [0.; 3]).unwrap();
                start.elapsed().as_secs_f64() * 1000.
            };
            let run_atomic = |film: &mut SurfaceFilm| {
                let start = std::time::Instant::now();
                film.advance_on_geometry(&posed, 0.0001, &[(0, 1e-9)], [0.; 3])
                    .unwrap();
                start.elapsed().as_secs_f64() * 1000.
            };
            let (a, b) = if round % 2 == 0 {
                (run_legacy(&mut legacy), run_atomic(&mut atomic))
            } else {
                let b = run_atomic(&mut atomic);
                let a = run_legacy(&mut legacy);
                (a, b)
            };
            assert_eq!(legacy.thickness(), atomic.thickness());
            assert_eq!(legacy.total_mass(), atomic.total_mass());
            if round >= 2 {
                legacy_samples.push(a);
                atomic_samples.push(b);
            }
        }
        legacy_samples.sort_by(f64::total_cmp);
        atomic_samples.sort_by(f64::total_cmp);
        let report = serde_json::json!({"vertices":points.len(),"cells":triangles.len(),"legacy_samples_ms":legacy_samples,"atomic_samples_ms":atomic_samples,"legacy_upper_median_ms":legacy_samples[5],"atomic_upper_median_ms":atomic_samples[5],"bitwise_equal":true,"scope":"CPU geometry/source/transport only; zero gravity, capillarity and wetting; no contact or rendering"});
        println!("{report}");
        if let Ok(path) = std::env::var("VOXY_ATOMIC_FRAME_REPORT") {
            std::fs::write(path, report.to_string()).unwrap();
        }
    }
}

#[cfg(test)]
mod checkpoint_restore_tests {
    #[test]
    #[ignore = "requires a preserved actual native film capture path"]
    fn restore_actual_native_body_capture() {
        let path = std::env::var("VOXY_FILM_CAPTURE").expect("VOXY_FILM_CAPTURE required");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        // The runtime facial cut changed after this capture. Reconstruct the
        // archived film's subset only after checking each face against the
        // unchanged imported source asset, rather than weakening chart checks.
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let reps: Vec<usize> =
            serde_json::from_value(saved["state"]["representatives"].clone()).unwrap();
        let triangles: Vec<[usize; 3]> =
            serde_json::from_value(saved["state"]["physics"]["triangles"].clone()).unwrap();
        let source_faces: std::collections::BTreeSet<[u32; 3]> = asset
            .mesh
            .indices()
            .chunks_exact(3)
            .map(|t| [t[0], t[1], t[2]])
            .collect();
        let mut indices = Vec::new();
        for triangle in triangles {
            let render = triangle.map(|i| reps[i] as u32);
            assert!(
                source_faces.contains(&render),
                "archived face is not in source asset"
            );
            indices.extend(render);
        }
        let mut film = super::FilmPreview::new(
            asset.mesh.vertices(),
            &indices,
            [0., 0.20, 0.13],
            0.04,
            5e-8,
        )
        .unwrap();
        let captured_weights: Vec<f64> =
            serde_json::from_value(saved["state"]["sourceWeights"].clone()).unwrap();
        let max_weight_difference = captured_weights
            .iter()
            .zip(&film.source_weights)
            .map(|(a, b)| (a - b).abs())
            .fold(0_f64, f64::max);
        println!("Source weight max absolute difference {max_weight_difference:e}");
        let settings = crate::film_settings::FilmSettings::default()
            .patched(&saved["state"]["settings"])
            .unwrap();
        let canonical: Vec<[[f64; 3]; 3]> =
            serde_json::from_value(saved["state"]["canonicalTrianglesM"].clone()).unwrap();
        let recalculated =
            super::source_weights(&canonical, settings.source_center, settings.source_radius_m)
                .unwrap();
        let delta = captured_weights
            .iter()
            .zip(&recalculated)
            .map(|(a, b)| (a - b).abs())
            .fold(0_f64, f64::max);
        println!(
            "Settings weight difference {delta:e}; lengths {}/{}, extra {}",
            captured_weights.len(),
            recalculated.len(),
            saved["state"]["extraSourceWeights"]
        );
        film.restore_snapshot(&saved["state"]).unwrap();
        assert_eq!(film.state_snapshot(), saved["state"]);
        let state = film.film.state();
        let mut expected = physics::surface_film::SurfaceFilm::from_state(&state).unwrap();
        let representatives = film.representatives.clone();
        let count = film.render_to_solver.len();
        let mut vertices = vec![
            voxy_render::SceneVertex {
                position: [0.; 3],
                uv: [0.; 2],
                color: [1.; 4]
            };
            count
        ];
        for (i, &render) in representatives.iter().enumerate() {
            vertices[render].position = state.points[i].map(|x| x as f32);
        }
        let points: Vec<_> = representatives
            .iter()
            .map(|&i| vertices[i].position.map(f64::from))
            .collect();
        expected
            .advance_on_geometry(&points, 0.001, &[], [0., -9.81, 0.])
            .unwrap();
        film.advance(&vertices, 0.001).unwrap();
        assert_eq!(
            film.film.state().cell_volumes_m3,
            expected.state().cell_volumes_m3
        );
    }
    fn preview() -> super::FilmPreview {
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        super::FilmPreview::new(
            &[v([0., 0., 0.]), v([0.02, 0., 0.]), v([0., 0.02, 0.])],
            &[0, 1, 2],
            [0., 0., 0.],
            0.04,
            1e-8,
        )
        .unwrap()
    }
    #[test]
    fn serialized_restore_preserves_state_and_continuation() {
        let mut original = preview();
        let settings = original
            .settings
            .patched(&serde_json::json!({"source_rate_m3_s":1e-9,"precursor_wetting_enabled":true}))
            .unwrap();
        original.configure(settings).unwrap();
        let v = |position| voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        };
        let vertices = [
            v([0., 0., 0.001]),
            v([0.021, 0., 0.001]),
            v([0., 0.022, 0.001]),
        ];
        original.advance(&vertices, 0.01).unwrap();
        let snapshot = original.state_snapshot();
        let saved = serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        let mut restored = preview();
        restored.restore_snapshot(&saved).unwrap();
        assert_eq!(restored.state_snapshot(), snapshot);
        original.advance(&vertices, 0.01).unwrap();
        restored.advance(&vertices, 0.01).unwrap();
        assert_eq!(restored.state_snapshot(), original.state_snapshot());
    }
    #[test]
    fn malformed_restore_does_not_change_existing_state() {
        let mut preview = preview();
        let original = preview.state_snapshot();
        for pointer in [
            "/units",
            "/renderToSolver/0",
            "/canonicalTrianglesM/0/0/0",
            "/physics/triangles/0/0",
            "/physics/cellVolumesM3/0",
            "/settings/density",
            "/sourceWeights/0",
            "/measurements/sourceAddedMassKg",
        ] {
            let mut invalid = original.clone();
            *invalid.pointer_mut(pointer).unwrap() = serde_json::json!(-1);
            assert!(
                preview.restore_snapshot(&invalid).is_err(),
                "accepted {pointer}"
            );
            assert_eq!(preview.state_snapshot(), original);
        }
    }
}

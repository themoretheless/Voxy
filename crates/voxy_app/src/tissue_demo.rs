//! Abstract tissue samples using the shared CPU solver and scene renderer.
use glam::{DMat4, DVec3, Mat4, Quat, Vec3};
use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, PrescribedTriangleSurface,
    SolidFilmBinding, SupportTarget, ViscoelasticOgden,
};
#[cfg(test)]
use physics::tissue::ellipsoid;
use physics::tissue::{Tissue, TissueKind, sample};
use physics::tissue_surface::EmbeddedSurface;
use std::sync::Arc;
use voxy_animation::{Joint, Skeleton, Transform};
use voxy_render::{SceneMesh, SceneVertex};
#[path = "tissue_demo/assembly.rs"]
mod assembly;
/// Test-harness fixture for external CCD replay. Constitutive Debug data is
/// evidence only; this is deliberately not a persistent physics checkpoint.
#[cfg(test)]
fn contact_geometry_fixture(
    body: &InertialBody,
    targets: &[SupportTarget],
    next: &PrescribedTriangleSurface,
    dt: f64,
    depth: usize,
    error: &str,
) -> Result<serde_json::Value, &'static str> {
    let start = body.prescribed_surface().ok_or("missing fixture contact")?;
    let mut predictor: Vec<_> = body
        .body()
        .positions()
        .iter()
        .zip(body.velocities())
        .map(|(p, v)| std::array::from_fn::<_, 3, _>(|axis| p[axis] + dt * v[axis]))
        .collect();
    for target in targets {
        if target.node >= predictor.len() {
            return Err("invalid fixture target");
        }
        predictor[target.node] = target.position_m;
    }
    Ok(serde_json::json!({
        "version": 1, "scope": "external-ccd-geometry", "error": error,
        "dt_s": dt, "depth": depth,
        "body_start": body.body().positions(), "body_predictor": predictor,
        "body_faces": body.body().surface(), "body_rest": body.body().rest_positions(),
        "velocities_m_s": body.velocities(), "masses_kg": body.masses(),
        "targets": targets.iter().map(|t| serde_json::json!({"node":t.node,"position_m":t.position_m})).collect::<Vec<_>>(),
        "surface_start": start.positions(), "surface_end": next.positions(),
        "surface_faces": start.faces(), "contact_faces": start.contact_faces(),
        "contact_parameters": {"minimum_distance_m": start.minimum_distance_m(), "activation_gap_m": start.activation_gap_m(), "pair_stiffness_n_m": start.pair_stiffness_n_m()},
        "constitutive_debug_not_checkpoint": format!("{:?}", body.body().elements()),
        "predictor_path_open_debug": format!("{:?}", start.path_is_open(next, body.body().positions(), &predictor, &body.body().surface()))
    }))
}
#[derive(Clone, Copy, Debug, Default)]
struct EnergyLedger {
    support_work_j: f64,
    surface_work_j: f64,
    heat_j: f64,
    defect_j: f64,
    conduction_defect_j: f64,
    accepted_steps: u64,
    rejected_steps: u64,
    max_refinement_depth: usize,
}
impl EnergyLedger {
    fn add(&mut self, other: Self) {
        self.support_work_j += other.support_work_j;
        self.surface_work_j += other.surface_work_j;
        self.heat_j += other.heat_j;
        self.defect_j += other.defect_j;
        self.conduction_defect_j += other.conduction_defect_j;
        self.accepted_steps += other.accepted_steps;
        self.rejected_steps += other.rejected_steps;
        self.max_refinement_depth = self.max_refinement_depth.max(other.max_refinement_depth);
    }
}
#[derive(Clone, Debug)]
enum DemoTissue {
    Xpbd(Tissue),
    Continuum {
        ledger: EnergyLedger,
        preferred_depth: usize,
        initial_energy_j: f64,
        dynamics: InertialBody,
        thermal_binding: Arc<SolidFilmBinding>,
        boundary_faces: Arc<[[usize; 3]]>,
        cells: Vec<[usize; 4]>,
    },
}
impl DemoTissue {
    fn positions(&self) -> &[[f64; 3]] {
        match self {
            Self::Xpbd(body) => body.positions(),
            Self::Continuum { dynamics, .. } => dynamics.body().positions(),
        }
    }
    fn volumes(&self) -> Vec<f64> {
        match self {
            Self::Xpbd(body) => body.volumes(),
            Self::Continuum { cells, .. } => cells
                .iter()
                .map(|cell| {
                    let [a, b, c, d] = cell.map(|i| glam::DVec3::from_array(self.positions()[i]));
                    (b - a).dot((c - a).cross(d - a)) / 6.
                })
                .collect(),
        }
    }
    fn smooth_boundary_normals(&self) -> Result<Vec<[f64; 3]>, &'static str> {
        let Self::Continuum { boundary_faces, .. } = self else {
            return Err("continuum normals require boundary topology");
        };
        let mut normals = vec![glam::DVec3::ZERO; self.positions().len()];
        for &[a, b, c] in boundary_faces.iter() {
            let p = |node| glam::DVec3::from_array(self.positions()[node]);
            let normal = (p(b) - p(a)).cross(p(c) - p(a));
            if !normal.is_finite() {
                return Err("boundary normal overflow");
            }
            for node in [a, b, c] {
                normals[node] += normal;
            }
        }
        Ok(normals
            .into_iter()
            .map(|normal| normal.normalize_or_zero().to_array())
            .collect())
    }
    fn xpbd(&self) -> Option<&Tissue> {
        if let Self::Xpbd(body) = self {
            Some(body)
        } else {
            None
        }
    }
}
#[derive(Debug)]
struct SkinRegionBinding {
    body: usize,
    joint: usize,
    vertices: Vec<usize>,
    embedding: EmbeddedSurface,
    rest: Vec<[f64; 3]>,
    cells: Vec<[usize; 4]>,
}
/// Reference-space membership is fixed at binding, never selected from an animated pose.
#[derive(Debug)]
pub(crate) struct TissueSkinBinding {
    regions: Vec<SkinRegionBinding>,
    vertex_count: usize,
    global: Option<Arc<assembly::GlobalSkinBinding>>,
}
impl TissueSkinBinding {
    pub(crate) fn tissue_owned_vertices(&self) -> Vec<usize> {
        if let Some(global) = &self.global {
            return global.tissue_owned_vertices();
        }
        let mut owned = vec![false; self.vertex_count];
        for region in &self.regions {
            for &vertex in &region.vertices {
                owned[vertex] = true;
            }
        }
        owned
            .iter()
            .enumerate()
            .filter_map(|(vertex, &bound)| bound.then_some(vertex))
            .collect()
    }
    pub(crate) fn bound_vertex_count(&self) -> usize {
        if let Some(global) = &self.global {
            return global.bound_count;
        }
        self.regions.iter().map(|r| r.vertices.len()).sum()
    }
}
#[derive(Clone, Debug)]
pub(crate) struct TissueRegionSpec {
    pub mesh: physics::biomechanics::TetraMesh,
    pub supports: Vec<usize>,
    pub joint: usize,
    pub ogden_terms: Vec<OgdenTerm>,
    pub bulk_pa: f64,
    pub maxwell_branches: Vec<MaxwellBranch>,
    pub density_kg_m3: f64,
    pub specific_heat_j_kg_k: f64,
    pub temperature_kelvin: f64,
}
impl TissueRegionSpec {
    pub(crate) fn illustrative(
        mesh: physics::biomechanics::TetraMesh,
        supports: Vec<usize>,
        joint: usize,
    ) -> Self {
        Self {
            mesh,
            supports,
            joint,
            ogden_terms: vec![OgdenTerm {
                shear_pa: 5000.,
                exponent: 2.,
            }],
            bulk_pa: 1e6,
            maxwell_branches: vec![MaxwellBranch {
                shear_pa: 10000.,
                relaxation_seconds: 0.2,
            }],
            density_kg_m3: 1000.,
            specific_heat_j_kg_k: 3500.,
            temperature_kelvin: 310.15,
        }
    }
}
#[derive(Debug)]
pub(crate) struct TissueDemo {
    bodies: Vec<DemoTissue>,
    surfaces: Vec<EmbeddedSurface>,
    attachments: Vec<(usize, Vec<(usize, [f64; 3])>)>,
    assembled_regions: Option<Arc<assembly::AssembledRegions>>,
    skin_contact_binding: Option<Arc<assembly::GlobalSkinBinding>>,
    body_rig: Option<Arc<Skeleton>>,
    body_mode: bool,
    biomechanics: Option<crate::biomechanics_demo::BiomechanicsDemo>,
    time: f64,
    accumulator: f64,
}
impl TissueDemo {
    pub(crate) fn new() -> Self {
        let kinds = [
            TissueKind::Skin,
            TissueKind::Buttock,
            TissueKind::Breast,
            TissueKind::Lip,
            TissueKind::Sphincter,
            TissueKind::Penis,
        ];
        Self {
            bodies: kinds
                .into_iter()
                .enumerate()
                .map(|(i, k)| {
                    DemoTissue::Xpbd(
                        sample(k, [-2.5 + i as f64, 0.0, 0.0]).expect("valid tissue sample"),
                    )
                })
                .collect(),
            surfaces: Vec::new(),
            attachments: Vec::new(),
            assembled_regions: None,
            skin_contact_binding: None,
            body_rig: None,
            body_mode: false,
            biomechanics: None,
            time: 0.0,
            accumulator: 0.0,
        }
    }
    pub(crate) fn biomechanics() -> Result<Self, &'static str> {
        let mut demo = Self::new();
        demo.bodies.clear();
        demo.biomechanics = Some(crate::biomechanics_demo::BiomechanicsDemo::new()?);
        Ok(demo)
    }
    pub(crate) fn biomechanics_title(&self) -> Option<String> {
        self.biomechanics.as_ref().map(|b| b.title())
    }
    pub(crate) fn body() -> Self {
        Self::body_at_centers(std::array::from_fn(Self::center), [0, 0, 3, 7])
    }
    pub(crate) fn body_at_centers(centers: [[f64; 3]; 4], joints: [usize; 4]) -> Self {
        let regions = [
            [0.08, 0.065, 0.05],
            [0.08, 0.065, 0.05],
            [0.09, 0.09, 0.065],
            [0.09, 0.09, 0.065],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, radii)| {
            (
                physics::biomechanics::TetraMesh::ellipsoid(centers[i], radii, 1)
                    .expect("valid rounded continuum mesh"),
                [3, 4, 6],
                joints[i],
            )
        })
        .collect();
        Self::body_from_regions(regions).expect("valid illustrative continuum regions")
    }
    /// Authored volumes use the same controller, thermal storage and assembly as
    /// the neutral mannequin. Mesh topology and explicit three-node supports are
    /// admitted before publication. Material constants remain illustrative.
    pub(crate) fn body_from_regions(
        regions: Vec<(physics::biomechanics::TetraMesh, [usize; 3], usize)>,
    ) -> Result<Self, &'static str> {
        Self::body_from_region_specs(
            regions
                .into_iter()
                .map(|(mesh, pins, joint)| {
                    TissueRegionSpec::illustrative(mesh, pins.to_vec(), joint)
                })
                .collect(),
        )
    }
    /// Caller-authored SI parameters are validated by the shared material and
    /// inertia owners; no anatomical calibration or support placement is inferred.
    pub(crate) fn body_from_region_specs(
        regions: Vec<TissueRegionSpec>,
    ) -> Result<Self, &'static str> {
        if regions.is_empty() || regions.len() > 64 {
            return Err("invalid authored tissue region count");
        }
        let mut demo = Self::new();
        demo.body_mode = true;
        demo.body_rig = Some(Arc::new(Self::body_skeleton()));
        demo.bodies.clear();
        for spec in regions {
            let TissueRegionSpec {
                mesh,
                supports: pins,
                joint,
                ogden_terms,
                bulk_pa,
                maxwell_branches,
                density_kg_m3,
                specific_heat_j_kg_k,
                temperature_kelvin,
            } = spec;
            let mut pinned = vec![false; mesh.points.len()];
            for &node in &pins {
                let flag = pinned
                    .get_mut(node)
                    .ok_or("invalid authored tissue supports")?;
                if *flag {
                    return Err("invalid authored tissue supports");
                }
                *flag = true;
            }
            let shear_pa = ogden_terms.iter().map(|t| t.shear_pa).sum();
            let law = ViscoelasticOgden::new(ogden_terms, bulk_pa, maxwell_branches)?;
            let material = Material {
                shear_pa,
                bulk_pa,
                fibers: vec![],
            };
            let mut body = mesh.clone().into_body(pinned, &material)?;
            body.set_viscoelastic_ogden_batch(
                &(0..mesh.cells.len())
                    .map(|cell| (cell, law.clone()))
                    .collect::<Vec<_>>(),
            )?;
            let mut dynamics = InertialBody::new_viscoelastic_with_supports(
                body,
                &vec![density_kg_m3; mesh.cells.len()],
                vec![[0.; 3]; mesh.points.len()],
            )?;
            dynamics.set_uniform_acceleration([0., -9.81, 0.])?;
            dynamics.enable_maxwell_thermal(
                &vec![specific_heat_j_kg_k; mesh.cells.len()],
                &vec![temperature_kelvin; mesh.cells.len()],
            )?;
            let energy = dynamics.diagnostics()?;
            let surface = Self::boundary_surface(dynamics.body().positions(), &mesh.boundary);
            demo.surfaces.push(EmbeddedSurface::bind(
                dynamics.body().positions(),
                &mesh.cells,
                &surface,
            )?);
            demo.attachments.push((
                joint,
                pins.into_iter()
                    .map(|node| (node, mesh.points[node]))
                    .collect(),
            ));
            demo.bodies.push(DemoTissue::Continuum {
                boundary_faces: dynamics.body().surface().into(),
                thermal_binding: Arc::new(SolidFilmBinding::new(&dynamics)?),
                initial_energy_j: energy.kinetic_j + energy.potential_j,
                ledger: EnergyLedger::default(),
                preferred_depth: 0,
                dynamics,
                cells: mesh.cells,
            });
        }
        Ok(demo)
    }
    /// Linear refinement follows the physical boundary without shrinking it.
    fn boundary_surface(rest: &[[f64; 3]], boundary: &[[usize; 3]]) -> Vec<[f64; 3]> {
        let mut triangles: Vec<[[f64; 3]; 3]> =
            boundary.iter().map(|f| f.map(|i| rest[i])).collect();
        for _ in 0..2 {
            triangles = triangles
                .into_iter()
                .flat_map(|[a, b, c]| {
                    let middle = |a: [f64; 3], b: [f64; 3]| {
                        std::array::from_fn(|axis| (a[axis] + b[axis]) * 0.5)
                    };
                    let (ab, bc, ca) = (middle(a, b), middle(b, c), middle(c, a));
                    [[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
                })
                .collect();
        }
        triangles.into_iter().flatten().collect()
    }
    pub(crate) fn is_body(&self) -> bool {
        self.body_mode
    }
    /// Sum of absolute tetrahedral cell volumes, in cubic metres.
    pub(crate) fn body_volumes_m3(&self) -> Vec<f64> {
        self.bodies
            .iter()
            .map(|body| body.volumes().iter().map(|v| v.abs()).sum())
            .collect()
    }
    /// Change in total mechanical energy, support work, released heat and
    /// independently accumulated numerical defect, all in joules.
    pub(crate) fn body_energy_receipts(&self) -> Result<Vec<[f64; 4]>, &'static str> {
        self.bodies
            .iter()
            .map(|body| {
                let DemoTissue::Continuum {
                    dynamics,
                    ledger,
                    initial_energy_j,
                    ..
                } = body
                else {
                    return Err("energy receipt requires continuum body");
                };
                let diagnostics = dynamics.diagnostics()?;
                Ok([
                    diagnostics.kinetic_j + diagnostics.potential_j - initial_energy_j,
                    ledger.support_work_j + ledger.surface_work_j,
                    ledger.heat_j,
                    ledger.defect_j,
                ])
            })
            .collect()
    }
    /// Committed accepted/rejected trial counts and deepest temporal refinement.
    pub(crate) fn body_step_counts(&self) -> Vec<(u64, u64, usize)> {
        self.bodies
            .iter()
            .filter_map(|body| {
                if let DemoTissue::Continuum { ledger, .. } = body {
                    Some((
                        ledger.accepted_steps,
                        ledger.rejected_steps,
                        ledger.max_refinement_depth,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
    /// Minimum/maximum cell Kelvin temperature and total stored heat, joules.
    pub(crate) fn body_thermal_diagnostics(&self) -> Result<Vec<(f64, f64, f64)>, &'static str> {
        self.bodies
            .iter()
            .map(|body| {
                let DemoTissue::Continuum { dynamics, .. } = body else {
                    return Err("thermal diagnostics require continuum body");
                };
                let temperatures = dynamics
                    .maxwell_temperatures_kelvin()
                    .ok_or("missing thermal owner")?;
                let heat = dynamics
                    .maxwell_sensible_energy_j()
                    .ok_or("missing thermal owner")?
                    .iter()
                    .sum();
                Ok((
                    temperatures.iter().copied().fold(f64::INFINITY, f64::min),
                    temperatures
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max),
                    heat,
                ))
            })
            .collect()
    }
    /// Free-center offset from a purely rigid bone transform, in world metres.
    pub(crate) fn body_secondary_offsets(&self) -> Vec<[f64; 3]> {
        self.secondary_offsets_for_palette(&self.body_palette())
    }
    pub(crate) fn secondary_offsets_for_palette(&self, palette: &[Mat4]) -> Vec<[f64; 3]> {
        if let Some(layout) = &self.assembled_regions {
            if let Some(DemoTissue::Continuum { dynamics, .. }) = self.bodies.first() {
                let mut offset = [0.; 3];
                let mut mass = 0.;
                for ((joint, _), range) in self.attachments.iter().zip(&layout.node_ranges) {
                    let matrix =
                        DMat4::from_cols_array(&palette[*joint].to_cols_array().map(f64::from));
                    for node in range.clone() {
                        let reference = matrix
                            .transform_point3(DVec3::from_array(
                                dynamics.body().rest_positions()[node],
                            ))
                            .to_array();
                        let weight = dynamics.masses()[node];
                        for axis in 0..3 {
                            offset[axis] += weight
                                * (dynamics.body().positions()[node][axis] - reference[axis]);
                        }
                        mass += weight;
                    }
                }
                return vec![offset.map(|v| v / mass)];
            }
        }
        self.bodies
            .iter()
            .zip(&self.attachments)
            .map(|(body, (joint, pins))| {
                // The midpoint of the opposing +/-Y pins is the reference center.
                let center = std::array::from_fn(|axis| (pins[0].1[axis] + pins[1].1[axis]) * 0.5);
                let rigid = DMat4::from_cols_array(&palette[*joint].to_cols_array().map(f64::from))
                    .transform_point3(DVec3::from_array(center))
                    .to_array();
                std::array::from_fn(|k| body.positions()[0][k] - rigid[k])
            })
            .collect()
    }
    pub(crate) fn body_motion_title(&self) -> Option<String> {
        if !self.body_mode {
            return None;
        }
        let phase = match self.time % 12. {
            t if t < 4. => "walk",
            t if t < 8. => "jump",
            _ => "settle",
        };
        let displacement_mm = self
            .body_secondary_offsets()
            .iter()
            .map(|v| v[0].hypot(v[1]).hypot(v[2]) * 1000.)
            .fold(0_f64, f64::max);
        Some(format!(
            "Voxy skeleton + soft tissues | {phase} | secondary displacement {displacement_mm:.1} mm | Space: pause"
        ))
    }
    fn center(i: usize) -> [f64; 3] {
        [
            if i % 2 == 0 { -0.23 } else { 0.23 },
            if i < 2 { 0.55 } else { -0.25 },
            if i < 2 { 0.25 } else { -0.23 },
        ]
    }
    fn lift(t: f64) -> f64 {
        // Walk, jump, then rest; twelve-second repeat.
        let t = t % 12.0;
        if t < 4.0 {
            0.045
                * (t * std::f64::consts::TAU * 1.8).sin().powi(2)
                * (std::f64::consts::PI * t / 4.).sin().powi(2)
        } else if t < 8.0 {
            let phase = (t - 4.0) % 2.0;
            if phase < 1.0 {
                0.48 * (std::f64::consts::PI * phase).sin().powi(2)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    /// Publishes a complete display frame only after every substep succeeds.
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        self.advance_with_body_step(dt, |candidate, time| {
            candidate.step_body(&candidate.body_palette_at(time))
        })
    }
    pub(crate) fn advance_with_palette(
        &mut self,
        dt: f64,
        mut palette_at: impl FnMut(f64) -> Result<Vec<Mat4>, &'static str>,
    ) -> Result<(), &'static str> {
        self.advance_with_body_step(dt, |candidate, time| {
            candidate.step_body(&palette_at(time)?)
        })
    }
    /// Install the same finite obstacle on every continuum region atomically.
    pub(crate) fn bind_contact_surface(
        &mut self,
        surface: Arc<PrescribedTriangleSurface>,
    ) -> Result<(), &'static str> {
        self.bind_contact_surfaces(&vec![surface; self.bodies.len()])
    }
    pub(crate) fn bind_contact_surfaces(
        &mut self,
        surfaces: &[Arc<PrescribedTriangleSurface>],
    ) -> Result<(), &'static str> {
        if self.assembled_regions.is_some() {
            return Err("bind regional contact before tissue assembly");
        }
        if surfaces.len() != self.bodies.len() {
            return Err("tissue contact region count mismatch");
        }
        let mut candidate = self.bodies.clone();
        for (region, surface) in candidate.iter_mut().zip(surfaces) {
            let DemoTissue::Continuum {
                dynamics,
                initial_energy_j,
                ..
            } = region
            else {
                return Err("surface contact requires continuum tissue");
            };
            *initial_energy_j += dynamics.set_prescribed_surface(Some(surface.clone()))?;
        }
        self.bodies = candidate;
        Ok(())
    }
    pub(crate) fn advance_with_palette_and_surface(
        &mut self,
        dt: f64,
        mut sample: impl FnMut(f64) -> Result<(Vec<Mat4>, Arc<PrescribedTriangleSurface>), &'static str>,
    ) -> Result<(), &'static str> {
        self.advance_with_body_step(dt, |candidate, time| {
            let (palette, surface) = sample(time)?;
            candidate.step_body_with_contact(
                &palette,
                0.5,
                Some(vec![surface; candidate.bodies.len()]),
            )
        })
    }
    pub(crate) fn advance_with_palette_and_surfaces(
        &mut self,
        dt: f64,
        mut sample: impl FnMut(
            f64,
        )
            -> Result<(Vec<Mat4>, Vec<Arc<PrescribedTriangleSurface>>), &'static str>,
    ) -> Result<(), &'static str> {
        self.advance_with_body_step(dt, |candidate, time| {
            let (palette, surfaces) = sample(time)?;
            candidate.step_body_with_contact(&palette, 0.5, Some(surfaces))
        })
    }
    /// Imported physics keeps supports and obstacle geometry on the same f64 clock.
    pub(crate) fn advance_with_palette64_and_surfaces(
        &mut self,
        dt: f64,
        mut sample: impl FnMut(
            f64,
        ) -> Result<
            (Vec<DMat4>, Vec<Arc<PrescribedTriangleSurface>>),
            &'static str,
        >,
    ) -> Result<(), &'static str> {
        self.advance_with_body_step(dt, |candidate, time| {
            let (palette, surfaces) = sample(time)?;
            candidate.step_body_with_contact64(&palette, 0.5, Some(surfaces))
        })
    }
    /// Bind only contained skin vertices. Exterior vertices retain skeletal motion.
    /// Overlapping ownership and invalid geometry are explicit errors.
    pub(crate) fn bind_skin(&self, skin: &[[f64; 3]]) -> Result<TissueSkinBinding, &'static str> {
        if self.assembled_regions.is_some() {
            return self.bind_assembled_skin(skin);
        }
        if skin.iter().flatten().any(|x| !x.is_finite()) {
            return Err("nonfinite skin binding");
        }
        let mut owners = vec![false; skin.len()];
        let mut regions = Vec::new();
        for (body, tissue) in self.bodies.iter().enumerate() {
            let DemoTissue::Continuum {
                dynamics, cells, ..
            } = tissue
            else {
                return Err("skin binding requires continuum tissue");
            };
            let rest = dynamics.body().rest_positions();
            EmbeddedSurface::bind(rest, cells, &[])?;
            let mut vertices = Vec::new();
            let mut points = Vec::new();
            for (index, &point) in skin.iter().enumerate() {
                match EmbeddedSurface::bind(rest, cells, &[point]) {
                    Ok(_) => {
                        if owners[index] {
                            return Err("overlapping skin tissue ownership");
                        }
                        owners[index] = true;
                        vertices.push(index);
                        points.push(point);
                    }
                    Err("surface vertex outside tetrahedral mesh") => {}
                    Err(error) => return Err(error),
                }
            }
            regions.push(SkinRegionBinding {
                body,
                joint: self
                    .attachments
                    .get(body)
                    .ok_or("missing skin attachment")?
                    .0,
                vertices,
                embedding: EmbeddedSurface::bind(rest, cells, &points)?,
                rest: rest.to_vec(),
                cells: cells.clone(),
            });
        }
        Ok(TissueSkinBinding {
            regions,
            vertex_count: skin.len(),
            global: None,
        })
    }
    /// Compose physical displacement with already posed skin; returns one atomic result.
    pub(crate) fn deform_skin(
        &self,
        binding: &TissueSkinBinding,
        palette: &[DMat4],
        skin: &[[f64; 3]],
    ) -> Result<Vec<[f64; 3]>, &'static str> {
        if let Some(global) = &binding.global {
            return self.deform_assembled_skin(global, palette, skin);
        }
        if skin.len() != binding.vertex_count || skin.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid posed skin");
        }
        let mut output = skin.to_vec();
        for region in &binding.regions {
            let tissue = self
                .bodies
                .get(region.body)
                .ok_or("skin body disappeared")?;
            let DemoTissue::Continuum {
                dynamics, cells, ..
            } = tissue
            else {
                return Err("skin body changed mode");
            };
            if dynamics.body().rest_positions() != region.rest || cells != &region.cells {
                return Err("skin body topology changed");
            }
            let joint = self
                .attachments
                .get(region.body)
                .ok_or("skin attachment disappeared")?
                .0;
            if joint != region.joint {
                return Err("skin attachment changed");
            }
            let matrix = palette.get(joint).ok_or("missing skin attachment joint")?;
            let determinant = matrix.determinant();
            if !matrix.is_finite() || !determinant.is_finite() || determinant == 0. {
                return Err("invalid skin attachment matrix");
            }
            let reference: Vec<_> = region
                .rest
                .iter()
                .map(|&p| matrix.transform_point3(DVec3::from_array(p)).to_array())
                .collect();
            let posed: Vec<_> = region.vertices.iter().map(|&i| skin[i]).collect();
            let mut deformed = vec![[0.; 3]; posed.len()];
            region.embedding.deform_relative_into(
                &reference,
                tissue.positions(),
                &posed,
                &mut deformed,
            )?;
            for (&index, point) in region.vertices.iter().zip(deformed) {
                output[index] = point;
            }
        }
        Ok(output)
    }
    /// World-space FEM surfaces, without the procedural mannequin display.
    pub(crate) fn tissue_mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (body, surface) in self.bodies.iter().zip(&self.surfaces) {
            let points = surface
                .deform(body.positions())
                .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
            let normals = surface
                .deform(
                    &body
                        .smooth_boundary_normals()
                        .map_err(|_| voxy_render::SceneError::InvalidGeometry)?,
                )
                .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
            for (point, normal) in points.iter().zip(normals) {
                let normal = Vec3::from_array(normal.map(|v| v as f32)).normalize_or_zero();
                let light = 0.3 + 0.7 * normal.dot(Vec3::new(-0.5, 0.8, 1.).normalize()).max(0.);
                indices.push(vertices.len() as u32);
                vertices.push(SceneVertex {
                    position: point.map(|v| v as f32),
                    uv: [0.; 2],
                    color: [0.2 * light, 0.8 * light, 0.95 * light, 1.],
                });
            }
        }
        SceneMesh::new(vertices, indices)
    }
    fn advance_with_body_step(
        &mut self,
        dt: f64,
        mut body_step: impl FnMut(&mut Self, f64) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid tissue frame duration");
        }
        if dt == 0. {
            return Ok(());
        }
        // Render embeddings stay with the owner; only simulation state is staged.
        let mut candidate = Self {
            bodies: self.bodies.clone(),
            surfaces: Vec::new(),
            attachments: self.attachments.clone(),
            assembled_regions: self.assembled_regions.clone(),
            skin_contact_binding: self.skin_contact_binding.clone(),
            body_rig: self.body_rig.clone(),
            body_mode: self.body_mode,
            biomechanics: self.biomechanics.clone(),
            time: self.time,
            accumulator: self.accumulator,
        };
        candidate.advance_in_place(dt, &mut body_step)?;
        self.bodies = candidate.bodies;
        self.biomechanics = candidate.biomechanics;
        self.time = candidate.time;
        self.accumulator = candidate.accumulator;
        Ok(())
    }
    fn advance_in_place(
        &mut self,
        dt: f64,
        body_step: &mut impl FnMut(&mut Self, f64) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        if let Some(b) = &mut self.biomechanics {
            return b.advance(dt);
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            if self.body_mode {
                let time = self.time + 1. / 240.;
                body_step(self, time)?;
                self.time = time;
                self.accumulator -= 1.0 / 240.0;
                continue;
            }
            self.time += 1. / 240.;
            for (i, b) in self.bodies.iter_mut().enumerate() {
                let DemoTissue::Xpbd(b) = b else {
                    return Err("abstract mode requires XPBD tissue");
                };
                b.set_activation(0.5 + 0.5 * (self.time * 2.0).sin())?;
                let x = -2.5 + i as f64;
                if i == 0 {
                    for j in 20..25 {
                        b.move_pin(
                            j,
                            [
                                x + (j % 5) as f64 * 0.15 - 0.3,
                                0.3,
                                0.12 * (self.time * 3.0).sin(),
                            ],
                        )?;
                    }
                } else if i < 4 {
                    let y = match i {
                        1 => 0.3,
                        2 => 0.36,
                        _ => 0.12,
                    };
                    b.move_pin(3, [x + 0.1 * (self.time * 3.0).sin(), y, 0.0])?;
                    b.move_pin(
                        6,
                        [
                            x,
                            0.0,
                            if i == 1 {
                                -0.28
                            } else if i == 2 {
                                -0.3
                            } else {
                                -0.16
                            },
                        ],
                    )?;
                } else if i == 5 {
                    for j in 0..4 {
                        let y = if j % 2 == 0 { -0.09 } else { 0.09 };
                        let z = if j < 2 { -0.09 } else { 0.09 };
                        b.move_pin(j, [x - 0.35, y + 0.06 * (self.time * 2.0).sin(), z])?;
                    }
                }
                b.step(
                    1.0 / 240.0,
                    if i == 4 {
                        [0.0; 3]
                    } else {
                        [0.0, -2.0, 1.0 * (self.time * 2.0).sin()]
                    },
                    &[],
                    24,
                )?;
            }
            self.accumulator -= 1.0 / 240.0;
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        if let Some(b) = &self.biomechanics {
            return b.mesh();
        }
        if self.body_mode {
            return self.body_mesh();
        }
        let colors = [
            [0.95, 0.75, 0.5, 1.0],
            [0.95, 0.45, 0.3, 1.0],
            [0.7, 0.55, 0.95, 1.0],
            [0.95, 0.35, 0.65, 1.0],
            [0.3, 0.85, 0.7, 1.0],
            [0.4, 0.65, 0.95, 1.0],
        ];
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (i, b) in self.bodies.iter().enumerate() {
            let b = b.xpbd().ok_or(voxy_render::SceneError::InvalidGeometry)?;
            for [a, c] in b.edges() {
                let a = b.positions()[a];
                let c = b.positions()[c];
                let dx = c[0] - a[0];
                let dy = c[1] - a[1];
                let length = dx.hypot(dy).max(1e-9);
                let x = -dy / length * 0.008;
                let y = dx / length * 0.008;
                let base = vertices.len() as u32;
                for p in [
                    [a[0] + x, a[1] + y, a[2]],
                    [a[0] - x, a[1] - y, a[2]],
                    [c[0] - x, c[1] - y, c[2]],
                    [c[0] + x, c[1] + y, c[2]],
                ] {
                    vertices.push(SceneVertex {
                        position: p.map(|v| v as f32),
                        uv: [0.0; 2],
                        color: colors[i],
                    });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        SceneMesh::new(vertices, indices)
    }
    /// Only the fixed attachments follow bones; free nodes retain solved momentum.
    fn step_body(&mut self, palette: &[Mat4]) -> Result<(), &'static str> {
        self.step_body_with_conductivity(palette, 0.5)
    }
    fn step_body_with_conductivity(
        &mut self,
        palette: &[Mat4],
        conductivity_w_m_k: f64,
    ) -> Result<(), &'static str> {
        self.step_body_with_contact(palette, conductivity_w_m_k, None)
    }
    fn step_body_with_contact(
        &mut self,
        palette: &[Mat4],
        conductivity_w_m_k: f64,
        next_surfaces: Option<Vec<Arc<PrescribedTriangleSurface>>>,
    ) -> Result<(), &'static str> {
        let palette64: Vec<_> = palette
            .iter()
            .map(|matrix| DMat4::from_cols_array(&matrix.to_cols_array().map(f64::from)))
            .collect();
        self.step_body_with_contact64(&palette64, conductivity_w_m_k, next_surfaces)
    }
    fn step_body_with_contact64(
        &mut self,
        palette: &[DMat4],
        conductivity_w_m_k: f64,
        next_surfaces: Option<Vec<Arc<PrescribedTriangleSurface>>>,
    ) -> Result<(), &'static str> {
        let workers = if next_surfaces.is_some() {
            std::thread::available_parallelism()
                .map_or(1, |n| n.get())
                .min(4)
        } else {
            1
        };
        self.step_body_with_contact64_workers(palette, conductivity_w_m_k, next_surfaces, workers)
    }
    pub(crate) fn step_body_with_contact64_workers(
        &mut self,
        palette: &[DMat4],
        conductivity_w_m_k: f64,
        next_surfaces: Option<Vec<Arc<PrescribedTriangleSurface>>>,
        max_workers: usize,
    ) -> Result<(), &'static str> {
        if self.assembled_regions.is_some() {
            return self.step_assembled_regions(palette, conductivity_w_m_k, next_surfaces);
        }
        if next_surfaces
            .as_ref()
            .is_some_and(|surfaces| surfaces.len() != self.bodies.len())
        {
            return Err("tissue contact region count mismatch");
        }
        let mut candidate = self.bodies.clone();
        let count = candidate.len().min(self.attachments.len());
        let workers = max_workers.max(1).min(count.max(1)).min(4);
        #[cfg(target_arch = "wasm32")]
        let workers = {
            let _ = workers;
            1
        };
        if workers > 1 {
            // Each region owns its mechanics, heat and ledger. Join every
            // worker before selecting the first error in authoring order;
            // publication remains one transaction for the complete frame.
            let chunk_size = count.div_ceil(workers);
            let surfaces = next_surfaces.as_deref();
            std::thread::scope(|scope| -> Result<(), &'static str> {
                let mut handles = Vec::new();
                let mut spawn_error = None;
                for (chunk, (bodies, attachments)) in candidate[..count]
                    .chunks_mut(chunk_size)
                    .zip(self.attachments[..count].chunks(chunk_size))
                    .enumerate()
                {
                    let start = chunk * chunk_size;
                    match std::thread::Builder::new()
                        .name(format!("voxy-tissue-{chunk}"))
                        .spawn_scoped(scope, move || -> Result<(), &'static str> {
                            for (local, (body, attachment)) in
                                bodies.iter_mut().zip(attachments).enumerate()
                            {
                                Self::step_contact_region(
                                    body,
                                    attachment,
                                    palette,
                                    conductivity_w_m_k,
                                    surfaces.map(|s| &s[start + local]),
                                )?;
                            }
                            Ok(())
                        }) {
                        Ok(handle) => handles.push(handle),
                        Err(_) => {
                            spawn_error = Some("tissue region worker unavailable");
                            break;
                        }
                    }
                }
                let mut first_error = None;
                for handle in handles {
                    let result = handle
                        .join()
                        .unwrap_or(Err("tissue region worker panicked"));
                    if first_error.is_none() {
                        first_error = result.err();
                    }
                }
                first_error.or(spawn_error).map_or(Ok(()), Err)
            })?;
        } else {
            for (region, (body, attachment)) in
                candidate.iter_mut().zip(&self.attachments).enumerate()
            {
                Self::step_contact_region(
                    body,
                    attachment,
                    palette,
                    conductivity_w_m_k,
                    next_surfaces.as_ref().map(|s| &s[region]),
                )?;
            }
        }
        self.bodies = candidate;
        Ok(())
    }
    fn step_contact_region(
        body: &mut DemoTissue,
        attachment: &(usize, Vec<(usize, [f64; 3])>),
        palette: &[DMat4],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
    ) -> Result<(), &'static str> {
        let (joint, pins) = attachment;
        let matrix = palette
            .get(*joint)
            .ok_or("missing tissue attachment bone")?;
        let support_matrix = *matrix;
        let targets: Vec<_> = pins
            .iter()
            .map(|(node, rest)| SupportTarget {
                node: *node,
                position_m: support_matrix
                    .transform_point3(DVec3::from_array(*rest))
                    .to_array(),
            })
            .collect();
        if targets
            .iter()
            .flat_map(|p| p.position_m)
            .any(|v| !v.is_finite())
        {
            return Err("nonfinite tissue attachment");
        }
        Self::step_continuum_targets(body, &targets, conductivity_w_m_k, next_surface)
    }
    fn step_continuum_targets(
        body: &mut DemoTissue,
        targets: &[SupportTarget],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
    ) -> Result<(), &'static str> {
        Self::step_continuum_targets_with_skin(
            body,
            targets,
            conductivity_w_m_k,
            next_surface,
            None,
        )
    }
    fn step_continuum_targets_with_skin(
        body: &mut DemoTissue,
        targets: &[SupportTarget],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
        next_skin: Option<&physics::biomechanics::StationaryEmbeddedContact>,
    ) -> Result<(), &'static str> {
        let DemoTissue::Continuum {
            dynamics,
            ledger,
            preferred_depth,
            thermal_binding,
            cells,
            ..
        } = body
        else {
            return Err("body mode requires continuum tissue");
        };
        // Illustrative isotropic conductivity, not calibrated tissue data.
        // The outer frame candidate stages both thermal half-steps and all
        // mechanical trials. Pure conduction does not alter mechanical work.
        let binding = thermal_binding;
        let conductivity = vec![conductivity_w_m_k; cells.len()];
        let links = binding.internal_heat_contacts(dynamics, &conductivity)?;
        let first_conduction_defect = dynamics.conduct_maxwell_heat(&links, 1. / 480., 1e-10)?;
        let start: Vec<_> = targets
            .iter()
            .map(|p| dynamics.body().positions()[p.node])
            .collect();
        // Start one level coarser than the previous frame's finest accepted
        // step; each trial still passes the unchanged work/heat/path checks.
        let start_surface = dynamics.prescribed_surface().cloned();
        let start_skin = dynamics.body().stationary_embedded_contact().cloned();
        if next_surface.is_some() && start_surface.is_none() {
            return Err("surface motion requires installed contact");
        }
        let initial_depth = *preferred_depth;
        let subdivisions = 4_u32 << initial_depth;
        let mut frame_receipt = EnergyLedger::default();
        for substep in 1..=subdivisions {
            let fraction = f64::from(substep) / f64::from(subdivisions);
            let segment: Vec<_> = targets
                .iter()
                .zip(&start)
                .map(|(target, old)| SupportTarget {
                    node: target.node,
                    position_m: std::array::from_fn(|axis| {
                        if substep == subdivisions {
                            target.position_m[axis]
                        } else {
                            old[axis] + fraction * (target.position_m[axis] - old[axis])
                        }
                    }),
                })
                .collect();
            let segment_surface = next_surface
                .map(|next| {
                    let start = start_surface
                        .as_ref()
                        .ok_or("surface motion requires installed contact")?;
                    if substep == subdivisions {
                        return Ok(next.clone());
                    }
                    start
                        .with_positions(
                            start
                                .positions()
                                .iter()
                                .zip(next.positions())
                                .map(|(a, b)| {
                                    std::array::from_fn(|axis| {
                                        a[axis] + fraction * (b[axis] - a[axis])
                                    })
                                })
                                .collect(),
                        )
                        .map(Arc::new)
                })
                .transpose()?;
            let dt = 1. / (240. * f64::from(subdivisions));
            if let Some(next_skin) = next_skin {
                let skin = start_skin
                    .as_ref()
                    .ok_or("missing installed skin contact")?
                    .sample_linear_pose(next_skin, fraction)?;
                let receipt = dynamics
                    .step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
                        Some(&segment),
                        segment_surface.ok_or("skin motion requires native integration pose")?,
                        skin,
                        dt,
                        1e-5 * dt * 240.,
                        256,
                    )?;
                frame_receipt.add(EnergyLedger {
                    support_work_j: receipt.step.support.support_work_j,
                    surface_work_j: receipt.step.support.surface_work_j,
                    heat_j: receipt.step.viscous_heat_j,
                    defect_j: receipt.step.total_energy_defect_j,
                    conduction_defect_j: 0.,
                    accepted_steps: receipt.substeps as u64,
                    rejected_steps: u64::from(receipt.substeps.ilog2()),
                    max_refinement_depth: initial_depth + receipt.substeps.ilog2() as usize,
                });
            } else {
                frame_receipt.add(Self::advance_continuum_contact(
                    dynamics,
                    &segment,
                    segment_surface,
                    dt,
                    initial_depth,
                )?);
            }
        }
        let links = binding.internal_heat_contacts(dynamics, &conductivity)?;
        frame_receipt.conduction_defect_j =
            first_conduction_defect + dynamics.conduct_maxwell_heat(&links, 1. / 480., 1e-10)?;
        *preferred_depth = frame_receipt.max_refinement_depth.saturating_sub(1);
        ledger.add(frame_receipt);
        Ok(())
    }
    fn advance_continuum(
        body: &mut InertialBody,
        targets: &[SupportTarget],
        dt: f64,
        depth: usize,
    ) -> Result<EnergyLedger, &'static str> {
        Self::advance_continuum_contact(body, targets, None, dt, depth)
    }
    fn advance_continuum_contact(
        body: &mut InertialBody,
        targets: &[SupportTarget],
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        depth: usize,
    ) -> Result<EnergyLedger, &'static str> {
        // Retry a rejected mechanical step with a finer temporal partition.
        // The surrounding frame candidate owns all successful subdivisions.
        // The solver itself is atomic. A successful leaf needs no extra clone.
        let result = match &next_surface {
            Some(surface) => body.step_viscoelastic_implicit_with_surface_motion(
                Some(targets),
                surface.clone(),
                dt,
                1e-5 * dt * 240.,
            ),
            None => body.step_viscoelastic(Some(targets), dt, 1e-5 * dt * 240.),
        };
        match result {
            Ok(receipt) => Ok(EnergyLedger {
                support_work_j: receipt.support.support_work_j,
                surface_work_j: receipt.support.surface_work_j,
                heat_j: receipt.viscous_heat_j,
                defect_j: receipt.total_energy_defect_j,
                conduction_defect_j: 0.,
                accepted_steps: 1,
                rejected_steps: 0,
                max_refinement_depth: depth,
            }),
            // Imported supports can require finer steps than the procedural
            // fixture. Keep a finite refinement limit and unchanged per-time
            // energy admission; a rejected frame still commits nothing.
            Err(error) if depth >= 12 => {
                #[cfg(test)]
                if let (Some(directory), Some(next)) =
                    (std::env::var_os("VOXY_CONTACT_FIXTURE_DIR"), &next_surface)
                {
                    let capture = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let value =
                            contact_geometry_fixture(body, targets, next, dt, depth, error)?;
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory)?;
                        let position = body.body().positions()[0];
                        let name = format!(
                            "contact-{:016x}-{:016x}-{:016x}.json",
                            dt.to_bits(),
                            position[0].to_bits(),
                            position[1].to_bits()
                        );
                        let file = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(directory.join(name))?;
                        serde_json::to_writer(file, &value)?;
                        Ok(())
                    })();
                    if let Err(capture_error) = capture {
                        eprintln!("CONTACT_FIXTURE_ERROR {capture_error}");
                    }
                }
                if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                    // Optional probes must not replace the original rejection
                    // when a diagnostic subdivision itself fails.
                    let observation = (|| -> Result<(), &'static str> {
                        if let Some(surface) = body.prescribed_surface() {
                            let feature = surface.nearest_active_contact(
                                body.body().positions(),
                                &body.body().surface(),
                            )?;
                            let pinned_weight = feature.as_ref().map(|feature| {
                                feature
                                    .body_face
                                    .iter()
                                    .zip(feature.body_weights)
                                    .filter(|(node, _)| {
                                        targets.iter().any(|target| target.node == **node)
                                    })
                                    .map(|(_, weight)| weight)
                                    .sum::<f64>()
                            });
                            eprintln!(
                                "CONTACT_FEATURE before={feature:?} pinned_weight={pinned_weight:?}"
                            );
                        }
                        let mut diagnostic = body.clone();
                        let trial = match &next_surface {
                            Some(surface) => diagnostic
                                .step_viscoelastic_implicit_with_surface_motion(
                                    Some(targets),
                                    surface.clone(),
                                    dt,
                                    1e100,
                                ),
                            None => diagnostic.step_viscoelastic(Some(targets), dt, 1e100),
                        };
                        eprintln!(
                            "CONTACT_REJECTION dt={dt:.17e} depth={depth} budget_j={:.17e} error={error} before={:?} diagnostic_trial={trial:?} after={:?}",
                            1e-5 * dt * 240.,
                            body.diagnostics(),
                            diagnostic.diagnostics()
                        );
                        if let Some(surface) = diagnostic.prescribed_surface() {
                            eprintln!(
                                "CONTACT_FEATURE after={:?}",
                                surface.nearest_active_contact(
                                    diagnostic.body().positions(),
                                    &diagnostic.body().surface()
                                )
                            );
                        }
                        if trial.is_ok() {
                            if let (Some(start), Some(end)) =
                                (body.prescribed_surface(), diagnostic.prescribed_surface())
                            {
                                let faces = body.body().surface();
                                let initial = start.response(body.body().positions(), &faces)?;
                                let final_response =
                                    end.response(diagnostic.body().positions(), &faces)?;
                                let mut integrands = Vec::new();
                                for sample in 0..=8 {
                                    let fraction = sample as f64 / 8.;
                                    let points: Vec<_> = body
                                        .body()
                                        .positions()
                                        .iter()
                                        .zip(diagnostic.body().positions())
                                        .map(|(a, b)| {
                                            std::array::from_fn(|axis| {
                                                a[axis] + fraction * (b[axis] - a[axis])
                                            })
                                        })
                                        .collect();
                                    let surface = start.with_positions(
                                        start
                                            .positions()
                                            .iter()
                                            .zip(end.positions())
                                            .map(|(a, b)| {
                                                std::array::from_fn(|axis| {
                                                    a[axis] + fraction * (b[axis] - a[axis])
                                                })
                                            })
                                            .collect(),
                                    )?;
                                    let response = surface.response(&points, &faces)?;
                                    let mut work = 0.;
                                    for (node, g) in response.body_gradient_n.iter().enumerate() {
                                        for axis in 0..3 {
                                            work += g[axis]
                                                * (diagnostic.body().positions()[node][axis]
                                                    - body.body().positions()[node][axis]);
                                        }
                                    }
                                    for (node, g) in response.obstacle_gradient_n.iter().enumerate()
                                    {
                                        for axis in 0..3 {
                                            work += g[axis]
                                                * (end.positions()[node][axis]
                                                    - start.positions()[node][axis]);
                                        }
                                    }
                                    integrands.push(work);
                                }
                                let simpson = (integrands[0]
                                    + integrands[8]
                                    + 4. * (integrands[1]
                                        + integrands[3]
                                        + integrands[5]
                                        + integrands[7])
                                    + 2. * (integrands[2] + integrands[4] + integrands[6]))
                                    / 24.;
                                let delta = final_response.potential_j - initial.potential_j;
                                let gauss = start.path_response(
                                    end,
                                    body.body().positions(),
                                    diagnostic.body().positions(),
                                    &faces,
                                )?;
                                let mut gauss_work = 0.;
                                let mut gauss_obstacle_work = 0.;
                                for (node, g) in gauss.body_gradient_n.iter().enumerate() {
                                    for axis in 0..3 {
                                        gauss_work += g[axis]
                                            * (diagnostic.body().positions()[node][axis]
                                                - body.body().positions()[node][axis]);
                                    }
                                }
                                for (node, g) in gauss.obstacle_gradient_n.iter().enumerate() {
                                    for axis in 0..3 {
                                        gauss_obstacle_work += g[axis]
                                            * (end.positions()[node][axis]
                                                - start.positions()[node][axis]);
                                    }
                                }
                                gauss_work += gauss_obstacle_work;
                                eprintln!(
                                    "CONTACT_GAUSS_WORK work_j={gauss_work:.17e} quadrature_error_j={:.17e} obstacle_work_j={gauss_obstacle_work:.17e}",
                                    delta - gauss_work
                                );
                                eprintln!(
                                    "CONTACT_PATH_WORK contact_energy_change_j={delta:.17e} midpoint_work_j={:.17e} simpson_work_j={simpson:.17e} midpoint_error_j={:.17e} simpson_error_j={:.17e}",
                                    integrands[4],
                                    delta - integrands[4],
                                    delta - simpson
                                );
                            }
                        }
                        for count in [2, 4] {
                            let mut proof = body.clone();
                            let mut total_defect = 0.;
                            let mut maximum_defect = 0.0_f64;
                            for step in 1..=count {
                                let fraction = step as f64 / count as f64;
                                let interpolated: Vec<_> = targets
                                    .iter()
                                    .map(|target| SupportTarget {
                                        node: target.node,
                                        position_m: std::array::from_fn(|axis| {
                                            body.body().positions()[target.node][axis]
                                                + fraction
                                                    * (target.position_m[axis]
                                                        - body.body().positions()[target.node]
                                                            [axis])
                                        }),
                                    })
                                    .collect();
                                let sub_surface = next_surface
                                    .as_ref()
                                    .map(|next| {
                                        let start = body
                                            .prescribed_surface()
                                            .ok_or("missing diagnostic contact")?;
                                        start
                                            .with_positions(
                                                start
                                                    .positions()
                                                    .iter()
                                                    .zip(next.positions())
                                                    .map(|(a, b)| {
                                                        std::array::from_fn(|axis| {
                                                            a[axis] + fraction * (b[axis] - a[axis])
                                                        })
                                                    })
                                                    .collect(),
                                            )
                                            .map(Arc::new)
                                    })
                                    .transpose()?;
                                let sub = match sub_surface {
                                    Some(surface) => proof
                                        .step_viscoelastic_implicit_with_surface_motion(
                                            Some(&interpolated),
                                            surface,
                                            dt / count as f64,
                                            1e100,
                                        ),
                                    None => proof.step_viscoelastic(
                                        Some(&interpolated),
                                        dt / count as f64,
                                        1e100,
                                    ),
                                }?;
                                total_defect += sub.total_energy_defect_j;
                                maximum_defect =
                                    maximum_defect.max(sub.support.energy_defect_j.abs());
                            }
                            eprintln!(
                                "CONTACT_CONVERGENCE subdivisions={count} total_defect_j={total_defect:.17e} maximum_leaf_defect_j={maximum_defect:.17e} mechanical_leaf_budget_j={:.17e}",
                                0.5 * 1e-5 * dt * 240. / count as f64
                            );
                        }
                        Ok(())
                    })();
                    if let Err(probe_error) = observation {
                        eprintln!(
                            "CONTACT_REJECTION_PROBE_FAILED original={error:?} probe={probe_error:?}"
                        );
                    }
                }
                Err(error)
            }
            Err(_) => {
                // Keep both children transactional: a successful first half
                // cannot escape when the second half fails.
                let mut candidate = body.clone();
                let middle: Vec<_> = targets
                    .iter()
                    .map(|target| SupportTarget {
                        node: target.node,
                        position_m: std::array::from_fn(|axis| {
                            (candidate.body().positions()[target.node][axis]
                                + target.position_m[axis])
                                * 0.5
                        }),
                    })
                    .collect();
                let middle_surface = next_surface
                    .as_ref()
                    .map(|next| {
                        let current = candidate
                            .prescribed_surface()
                            .ok_or("surface motion requires installed contact")?;
                        let positions = current
                            .positions()
                            .iter()
                            .zip(next.positions())
                            .map(|(a, b)| std::array::from_fn(|axis| a[axis] * 0.5 + b[axis] * 0.5))
                            .collect();
                        current.with_positions(positions).map(Arc::new)
                    })
                    .transpose()?;
                let mut receipt = Self::advance_continuum_contact(
                    &mut candidate,
                    &middle,
                    middle_surface,
                    dt * 0.5,
                    depth + 1,
                )?;
                receipt.add(Self::advance_continuum_contact(
                    &mut candidate,
                    targets,
                    next_surface,
                    dt * 0.5,
                    depth + 1,
                )?);
                receipt.rejected_steps += 1;
                *body = candidate;
                Ok(receipt)
            }
        }
    }
    /// Shared animation skeleton: root, then shoulder/elbow and hip/knee per side.
    fn body_skeleton() -> Skeleton {
        let mut joints = vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }];
        for side in [-1.0, 1.0] {
            for (name, parent, origin, parent_origin) in [
                ("shoulder", 0, Vec3::new(side * 0.55, 0.89, 0.), Vec3::ZERO),
                (
                    "elbow",
                    joints.len(),
                    Vec3::new(side * 0.55, 0.35, 0.),
                    Vec3::new(side * 0.55, 0.89, 0.),
                ),
                ("hip", 0, Vec3::new(side * 0.2, -0.34, 0.), Vec3::ZERO),
                (
                    "knee",
                    joints.len() + 2,
                    Vec3::new(side * 0.2, -0.91, 0.),
                    Vec3::new(side * 0.2, -0.34, 0.),
                ),
            ] {
                joints.push(Joint {
                    name: format!("{side}_{name}").into(),
                    parent: Some(parent as u16),
                    bind_local: Transform {
                        translation: origin - parent_origin,
                        ..Transform::IDENTITY
                    },
                    inverse_bind: Mat4::from_translation(-origin),
                });
            }
        }
        Skeleton::new(joints).expect("fixed mannequin hierarchy")
    }
    fn body_palette(&self) -> Vec<Mat4> {
        self.body_palette_at(self.time)
    }
    fn body_palette_at(&self, time: f64) -> Vec<Mat4> {
        let skeleton = self.body_rig.as_deref().expect("body mode owns skeleton");
        let mut pose = skeleton.bind_pose();
        let phase = (time % 12.) as f32;
        let walking = if phase < 4. {
            (phase * std::f32::consts::TAU * 1.8).sin()
                * (std::f32::consts::PI * phase / 4.).sin().powi(2)
        } else {
            0.
        };
        let jumping = if (4.0..8.0).contains(&phase) {
            (Self::lift(time) / 0.48) as f32
        } else {
            0.
        };
        pose.set_joint_rotation(0, Quat::from_rotation_y(walking * 0.12))
            .expect("finite torso rotation");
        for (side, start) in [(-1., 1), (1., 5)] {
            pose.set_joint_rotation(
                start,
                Quat::from_rotation_x(-side * walking * 0.55 - jumping * 0.8),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 1,
                Quat::from_rotation_x(-0.25 * walking.abs() - jumping * 0.45),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 2,
                Quat::from_rotation_x(side * walking * 0.45 + jumping * 0.3),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 3,
                Quat::from_rotation_x(-0.5 * (side * walking).max(0.) - jumping * 0.6),
            )
            .expect("finite joint rotation");
        }
        pose.skin_matrices(skeleton)
            .expect("finite mannequin pose")
            .into_iter()
            .map(|matrix| Mat4::from_translation(Vec3::Y * Self::lift(time) as f32) * matrix)
            .collect()
    }
    fn body_mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let palette = self.body_palette();
        // Two full mannequins, front and rear, retain a fixed camera and scale.
        for (view, x) in [(1.0, -1.25), (-1.0, 1.25)] {
            let mut ellipsoid = |center: [f64; 3], r: [f64; 3], color: [f32; 4], joint: usize| {
                let base = vertices.len() as u32;
                for j in 0..=16 {
                    let theta = j as f64 * std::f64::consts::PI / 16.0;
                    for k in 0..=24 {
                        let phi = k as f64 * std::f64::consts::TAU / 24.0;
                        let p = [
                            center[0] + r[0] * theta.sin() * phi.cos(),
                            center[1] + r[1] * theta.cos(),
                            center[2] + r[2] * theta.sin() * phi.sin(),
                        ];
                        let p =
                            palette[joint].transform_point3(Vec3::from_array(p.map(|v| v as f32)));
                        let normal = Vec3::new(
                            (theta.sin() * phi.cos() / r[0]) as f32,
                            (theta.cos() / r[1]) as f32,
                            (theta.sin() * phi.sin() / r[2]) as f32,
                        );
                        let normal = palette[joint].transform_vector3(normal).normalize()
                            * Vec3::new(view as f32, 1., view as f32);
                        let light =
                            0.3 + 0.7 * normal.dot(Vec3::new(-0.5, 0.8, 1.).normalize()).max(0.);
                        vertices.push(SceneVertex {
                            position: [x as f32 + p.x * view as f32, p.y, p.z * view as f32],
                            uv: [0.0; 2],
                            color: [
                                color[0] * light,
                                color[1] * light,
                                color[2] * light,
                                color[3],
                            ],
                        });
                    }
                }
                for j in 0..16 {
                    for k in 0..24 {
                        let a = base + j * 25 + k;
                        indices.extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
                    }
                }
            };
            let suit = [0.22, 0.48, 0.67, 1.0];
            ellipsoid([0.0, 0.38, 0.0], [0.39, 0.61, 0.22], suit, 0);
            ellipsoid([0.0, -0.24, 0.0], [0.41, 0.32, 0.25], suit, 0);
            ellipsoid(
                [0.0, 1.24, 0.0],
                [0.23, 0.28, 0.23],
                [0.7, 0.73, 0.77, 1.0],
                0,
            );
            for (side, start) in [(-1.0, 1), (1.0, 5)] {
                ellipsoid(
                    [side * 0.2, -0.625, 0.0],
                    [0.14, 0.285, 0.15],
                    suit,
                    start + 2,
                );
                ellipsoid(
                    [side * 0.2, -1.195, 0.0],
                    [0.13, 0.285, 0.14],
                    suit,
                    start + 3,
                );
                ellipsoid([side * 0.55, 0.62, 0.0], [0.12, 0.27, 0.12], suit, start);
                ellipsoid(
                    [side * 0.55, 0.08, 0.0],
                    [0.10, 0.27, 0.10],
                    suit,
                    start + 1,
                );
                ellipsoid(
                    [side * 0.2, -1.47, 0.09],
                    [0.15, 0.1, 0.23],
                    suit,
                    start + 3,
                );
            }
            for (i, (body, surface)) in self.bodies.iter().zip(&self.surfaces).enumerate() {
                let points = surface
                    .deform(body.positions())
                    .expect("validated tissue state");
                let color = if i < 2 {
                    [0.32, 0.67, 0.8, 1.0]
                } else {
                    [0.28, 0.58, 0.73, 1.0]
                };
                let normals = body
                    .smooth_boundary_normals()
                    .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
                let normals = surface
                    .deform(&normals)
                    .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
                for (p, normal) in points.iter().zip(normals) {
                    let normal = Vec3::from_array(normal.map(|v| v as f32)).normalize_or_zero()
                        * Vec3::new(view as f32, 1., view as f32);
                    let light =
                        0.3 + 0.7 * normal.dot(Vec3::new(-0.5, 0.8, 1.).normalize()).max(0.);
                    indices.push(vertices.len() as u32);
                    vertices.push(SceneVertex {
                        position: [(x + p[0] * view) as f32, p[1] as f32, (p[2] * view) as f32],
                        uv: [0.; 2],
                        color: [
                            color[0] * light,
                            color[1] * light,
                            color[2] * light,
                            color[3],
                        ],
                    });
                }
            }
        }
        SceneMesh::new(vertices, indices)
    }
}
#[cfg(test)]
mod tests {

    #[test]
    fn skin_binding_rejects_overlap_reassigned_joint_and_singular_pose() {
        let overlap = super::TissueDemo::body_at_centers([[0.; 3]; 4], [0, 0, 3, 7]);
        assert!(overlap.bind_skin(&[[0.; 3]]).is_err());
        let mut demo = super::TissueDemo::body();
        let skin = [demo.bodies[0].positions()[1]];
        let binding = demo.bind_skin(&skin).unwrap();
        let mut palette = vec![glam::DMat4::IDENTITY; 8];
        palette[0] = glam::DMat4::ZERO;
        assert!(demo.deform_skin(&binding, &palette, &skin).is_err());
        palette[0] = glam::DMat4::IDENTITY;
        demo.attachments[0].0 = 1;
        assert!(demo.deform_skin(&binding, &palette, &skin).is_err());
    }
    #[test]
    fn bound_skin_follows_physical_node_and_keeps_exterior_vertices() {
        let mut demo = super::TissueDemo::body();
        let skin = [demo.bodies[0].positions()[1], [100., 100., 100.]];
        let binding = demo.bind_skin(&skin).unwrap();
        assert_eq!(binding.bound_vertex_count(), 1);
        let palette = vec![glam::DMat4::IDENTITY; 8];
        assert_eq!(demo.deform_skin(&binding, &palette, &skin).unwrap(), skin);
        demo.step_body_with_contact64_workers(&palette, 0.5, None, 1)
            .unwrap();
        let result = demo.deform_skin(&binding, &palette, &skin).unwrap();
        assert_eq!(result[1], skin[1]);
        for axis in 0..3 {
            assert!((result[0][axis] - demo.bodies[0].positions()[1][axis]).abs() < 1e-14);
        }
        assert_ne!(result[0], skin[0]);
        assert!(demo.deform_skin(&binding, &palette[..1], &skin).is_err());
        assert!(demo.deform_skin(&binding, &palette, &skin[..1]).is_err());
        let other = super::TissueDemo::body_at_centers([[20.; 3]; 4], [0, 0, 3, 7]);
        assert!(other.deform_skin(&binding, &palette, &skin).is_err());
    }

    use super::*;
    #[test]
    fn contact_fixture_roundtrip_preserves_ccd_and_does_not_mutate_body() {
        for origin in [0., 1e6] {
            let positions = vec![
                [origin + 0.1, origin + 0.1, 0.000105],
                [origin + 0.2, origin + 0.1, 0.1],
                [origin + 0.1, origin + 0.2, 0.1],
                [origin + 0.1, origin + 0.1, 0.2],
            ];
            let solid = Body::new(
                positions.clone(),
                vec![true, false, false, false],
                vec![(
                    [0, 1, 2, 3],
                    Material::from_young_poisson(1e6, 0.45).unwrap(),
                )],
            )
            .unwrap();
            let mut body =
                InertialBody::new_with_fixed_supports(solid, &[1000.], vec![[0.; 3]; 4]).unwrap();
            let start = PrescribedTriangleSurface::new(
                vec![
                    [origin - 1., origin - 1., 0.],
                    [origin + 1., origin - 1., 0.],
                    [origin, origin + 1., 0.],
                ],
                vec![[0, 1, 2]],
                0.0001,
                0.003,
                100.,
            )
            .unwrap();
            let next = start
                .with_positions(
                    start
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], 0.00001])
                        .collect(),
                )
                .unwrap();
            body.set_prescribed_surface(Some(Arc::new(start.clone())))
                .unwrap();
            let targets = [SupportTarget {
                node: 0,
                position_m: positions[0],
            }];
            let before = format!("{body:?}");
            let fixture =
                contact_geometry_fixture(&body, &targets, &next, 0.02, 12, "fixture rejection")
                    .unwrap();
            let fixture: serde_json::Value =
                serde_json::from_slice(&serde_json::to_vec(&fixture).unwrap()).unwrap();
            let points =
                |name| serde_json::from_value::<Vec<[f64; 3]>>(fixture[name].clone()).unwrap();
            for (original, restored) in [
                (body.body().positions(), points("body_start")),
                (start.positions(), points("surface_start")),
                (next.positions(), points("surface_end")),
            ] {
                assert_eq!(original.len(), restored.len());
                for (original, restored) in original.iter().zip(restored) {
                    for axis in 0..3 {
                        assert_eq!(original[axis].to_bits(), restored[axis].to_bits());
                    }
                }
            }
            let faces: Vec<[usize; 3]> =
                serde_json::from_value(fixture["surface_faces"].clone()).unwrap();
            let enabled: Vec<bool> =
                serde_json::from_value(fixture["contact_faces"].clone()).unwrap();
            let parameters = &fixture["contact_parameters"];
            let restored = PrescribedTriangleSurface::new(
                points("surface_start"),
                faces,
                parameters["minimum_distance_m"].as_f64().unwrap(),
                parameters["activation_gap_m"].as_f64().unwrap(),
                parameters["pair_stiffness_n_m"].as_f64().unwrap(),
            )
            .unwrap()
            .with_contact_faces(enabled)
            .unwrap();
            let end = restored.with_positions(points("surface_end")).unwrap();
            let faces: Vec<[usize; 3]> =
                serde_json::from_value(fixture["body_faces"].clone()).unwrap();
            let expected = start
                .path_is_open(
                    &next,
                    body.body().positions(),
                    body.body().positions(),
                    &faces,
                )
                .unwrap();
            let actual = restored
                .path_is_open(
                    &end,
                    &points("body_start"),
                    &points("body_predictor"),
                    &faces,
                )
                .unwrap();
            assert!(!expected);
            assert_eq!(actual, expected);
            assert_eq!(format!("{body:?}"), before);
        }
    }
    #[test]
    fn wide_supports_keep_sub_render_scale_motion() {
        let mut demo = TissueDemo::body();
        let scale = 1. + 1e-9;
        assert_eq!(scale as f32, 1.);
        let palette = vec![DMat4::from_scale(DVec3::splat(scale)); 9];
        demo.step_body_with_contact64(&palette, 0.5, None).unwrap();
        for (body, (_, pins)) in demo.bodies.iter().zip(&demo.attachments) {
            for (node, rest) in pins {
                let actual = body.positions()[*node];
                for axis in 0..3 {
                    assert_eq!(actual[axis], rest[axis] * scale);
                }
            }
        }
    }
    #[test]
    fn failed_rejection_observation_preserves_error_and_state() {
        let mut demo = TissueDemo::body();
        let DemoTissue::Continuum { dynamics, .. } = &mut demo.bodies[0] else {
            unreachable!()
        };
        let before = format!("{dynamics:?}");
        // A rejected step and its diagnostic subdivisions both fail support
        // admission. With tracing enabled, failed probes must preserve the
        // original error and all mechanical/material/thermal state.
        let error =
            TissueDemo::advance_continuum_contact(dynamics, &[], None, f64::MAX, 12).unwrap_err();
        assert_eq!(error, "incomplete prescribed support targets");
        assert_eq!(format!("{dynamics:?}"), before);
    }
    #[test]
    fn distinct_regional_contact_domains_commit_together_and_reject_swaps() {
        let mut demo = TissueDemo::body();
        let reference = PrescribedTriangleSurface::new(
            vec![
                [-10., -10., -10.],
                [10., -10., -10.],
                [0., 10., -10.],
                [-10., -10., -20.],
                [10., -10., -20.],
                [0., 10., -20.],
            ],
            vec![[0, 1, 2], [3, 4, 5]],
            0.001,
            0.03,
            100.,
        )
        .unwrap();
        let surfaces: Vec<_> = (0..4)
            .map(|i| {
                Arc::new(
                    reference
                        .with_contact_faces(vec![i % 2 == 0, i % 2 != 0])
                        .unwrap(),
                )
            })
            .collect();
        let snapshot = format!("{demo:?}");
        assert!(demo.bind_contact_surfaces(&surfaces[..3]).is_err());
        assert_eq!(format!("{demo:?}"), snapshot);
        demo.bind_contact_surfaces(&surfaces).unwrap();
        let moved: Vec<_> = surfaces
            .iter()
            .map(|surface| {
                Arc::new(
                    surface
                        .with_positions(
                            surface
                                .positions()
                                .iter()
                                .map(|p| [p[0], p[1], p[2] + 0.001])
                                .collect(),
                        )
                        .unwrap(),
                )
            })
            .collect();
        let palette = vec![Mat4::IDENTITY; 9];
        demo.advance_with_palette_and_surfaces(1. / 240., |_| Ok((palette.clone(), moved.clone())))
            .unwrap();
        for (region, expected) in demo.bodies.iter().zip(&moved) {
            let DemoTissue::Continuum { dynamics, .. } = region else {
                unreachable!()
            };
            let actual = dynamics.prescribed_surface().unwrap();
            assert_eq!(actual.contact_faces(), expected.contact_faces());
            assert_eq!(actual.positions(), expected.positions());
        }
        let snapshot = format!("{demo:?}");
        let mut swapped = moved.clone();
        swapped.swap(2, 3);
        assert!(
            demo.advance_with_palette_and_surfaces(1. / 240., |_| Ok((
                palette.clone(),
                swapped.clone()
            )))
            .is_err()
        );
        assert_eq!(format!("{demo:?}"), snapshot);
        assert!(
            demo.advance_with_palette_and_surfaces(1. / 240., |_| Ok((
                palette.clone(),
                moved[..3].to_vec()
            )))
            .is_err()
        );
        assert_eq!(format!("{demo:?}"), snapshot);
    }
    #[test]
    fn frame_surface_binding_and_motion_are_atomic() {
        let mut demo = TissueDemo::body();
        let far = Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-10., -10., -10.], [10., -10., -10.], [0., 10., -10.]],
                vec![[0, 1, 2]],
                0.001,
                0.03,
                100.,
            )
            .unwrap(),
        );
        demo.bind_contact_surface(far.clone()).unwrap();
        let moved = Arc::new(
            far.with_positions(
                far.positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.001])
                    .collect(),
            )
            .unwrap(),
        );
        let palette = vec![Mat4::IDENTITY; 9];
        demo.advance_with_palette_and_surface(1. / 240., |_| Ok((palette.clone(), moved.clone())))
            .unwrap();
        for region in &demo.bodies {
            let DemoTissue::Continuum {
                dynamics, ledger, ..
            } = region
            else {
                unreachable!()
            };
            assert_eq!(
                dynamics.prescribed_surface().unwrap().positions(),
                moved.positions()
            );
            assert_eq!(ledger.surface_work_j, 0.);
        }
        let snapshot = format!("{demo:?}");
        let wrong_owner = Arc::new(
            PrescribedTriangleSurface::new(
                moved.positions().to_vec(),
                moved.faces().to_vec(),
                0.001,
                0.03,
                100.,
            )
            .unwrap(),
        );
        assert!(
            demo.advance_with_palette_and_surface(1. / 240., |_| Ok((
                palette.clone(),
                wrong_owner.clone()
            )))
            .is_err()
        );
        assert_eq!(format!("{demo:?}"), snapshot);
        let mut calls = 0;
        assert!(
            demo.advance_with_palette_and_surface(2. / 240., |_| {
                calls += 1;
                if calls == 2 {
                    Err("fixture pose failure")
                } else {
                    Ok((palette.clone(), moved.clone()))
                }
            })
            .is_err()
        );
        assert_eq!(format!("{demo:?}"), snapshot);
    }
    #[test]
    fn adaptive_surface_motion_books_work_and_crossing_rolls_back() {
        let mut material = Body::new(
            vec![
                [0.1, 0.1, 0.015],
                [0.2, 0.1, 0.02],
                [0.1, 0.2, 0.025],
                [0.1, 0.1, 0.115],
            ],
            vec![true; 4],
            vec![(
                [0, 1, 2, 3],
                Material {
                    shear_pa: 5000.,
                    bulk_pa: 1000000.,
                    fibers: vec![],
                },
            )],
        )
        .unwrap();
        material
            .set_viscoelastic_ogden(
                0,
                ViscoelasticOgden::new(
                    vec![OgdenTerm {
                        shear_pa: 5000.,
                        exponent: 2.,
                    }],
                    1000000.,
                    vec![MaxwellBranch {
                        shear_pa: 10000.,
                        relaxation_seconds: 0.2,
                    }],
                )
                .unwrap(),
            )
            .unwrap();
        let mut dynamics =
            InertialBody::new_viscoelastic_with_supports(material, &[1000.], vec![[0.; 3]; 4])
                .unwrap();
        let surface = Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
                vec![[0, 1, 2]],
                0.001,
                0.03,
                100.,
            )
            .unwrap(),
        );
        dynamics
            .set_prescribed_surface(Some(surface.clone()))
            .unwrap();
        let targets: Vec<_> = dynamics
            .body()
            .positions()
            .iter()
            .enumerate()
            .map(|(node, &position_m)| SupportTarget { node, position_m })
            .collect();
        let before = dynamics.diagnostics().unwrap();
        let moved = Arc::new(
            surface
                .with_positions(
                    surface
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + 1e-6])
                        .collect(),
                )
                .unwrap(),
        );
        let receipt = TissueDemo::advance_continuum_contact(
            &mut dynamics,
            &targets,
            Some(moved.clone()),
            1e-4,
            0,
        )
        .unwrap();
        assert!(receipt.surface_work_j > 0.);
        let after = dynamics.diagnostics().unwrap();
        assert!(
            (after.potential_j
                - before.potential_j
                - receipt.surface_work_j
                - receipt.support_work_j
                - receipt.defect_j)
                .abs()
                < 1e-10
        );
        assert_eq!(
            dynamics.prescribed_surface().unwrap().positions(),
            moved.positions()
        );
        let snapshot = format!("{dynamics:?}");
        let crossing = Arc::new(
            moved
                .with_positions(
                    moved
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], 0.2])
                        .collect(),
                )
                .unwrap(),
        );
        assert!(
            TissueDemo::advance_continuum_contact(
                &mut dynamics,
                &targets,
                Some(crossing),
                1e-4,
                11
            )
            .is_err()
        );
        assert_eq!(format!("{dynamics:?}"), snapshot);
    }
    #[test]
    fn physical_boundary_binds_without_shrinkage_and_follows_affine_motion() {
        let body = ellipsoid(
            [0.2, -0.1, 0.3],
            [0.3, 0.36, 0.3],
            1000.,
            1,
            &[3, 6],
            TissueKind::Breast.material(),
        )
        .unwrap();
        let rest = body.positions();
        let shell = TissueDemo::boundary_surface(rest, body.surface_triangles());
        assert_eq!(shell.len(), body.surface_triangles().len() * 16 * 3);
        for axis in 0..3 {
            let bounds = |points: &[[f64; 3]]| {
                points
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), p| {
                        (low.min(p[axis]), high.max(p[axis]))
                    })
            };
            assert_eq!(bounds(&shell), bounds(rest));
        }
        let binding =
            EmbeddedSurface::bind(rest, &body.tetrahedra().collect::<Vec<_>>(), &shell).unwrap();
        let transform = |p: [f64; 3]| [p[0] + 0.2 * p[1] + 1., 0.9 * p[1] - 2., p[2] + 0.1 * p[0]];
        let moved: Vec<_> = rest.iter().copied().map(transform).collect();
        for (actual, expected) in binding
            .deform(&moved)
            .unwrap()
            .iter()
            .zip(shell.into_iter().map(transform))
        {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn smooth_boundary_normals_are_outward_and_rotation_covariant() {
        let demo = TissueDemo::body();
        for body in &demo.bodies {
            let normals = body.smooth_boundary_normals().unwrap();
            let center = glam::DVec3::from_array(body.positions()[0]);
            for (node, normal) in normals.iter().enumerate().skip(1) {
                let normal = glam::DVec3::from_array(*normal);
                assert!((normal.length() - 1.).abs() < 1e-12);
                assert!(normal.dot(glam::DVec3::from_array(body.positions()[node]) - center) > 0.);
            }
            let DemoTissue::Continuum { cells, .. } = body else {
                unreachable!()
            };
            let positions: Vec<_> = body
                .positions()
                .iter()
                .map(|p| [-p[1] + 2., p[0] - 1., p[2] + 3.])
                .collect();
            let moved = Body::new(
                positions.clone(),
                vec![false; positions.len()],
                cells
                    .iter()
                    .map(|&cell| {
                        (
                            cell,
                            Material {
                                shear_pa: 100.,
                                bulk_pa: 1000.,
                                fibers: vec![],
                            },
                        )
                    })
                    .collect(),
            )
            .unwrap();
            let dynamics = InertialBody::new(
                moved,
                &vec![1000.; cells.len()],
                vec![[0.; 3]; positions.len()],
            )
            .unwrap();
            let moved = DemoTissue::Continuum {
                boundary_faces: dynamics.body().surface().into(),
                thermal_binding: Arc::new(SolidFilmBinding::new(&dynamics).unwrap()),
                dynamics,
                cells: cells.clone(),
                ledger: EnergyLedger::default(),
                preferred_depth: 0,
                initial_energy_j: 0.,
            };
            for (original, actual) in normals.iter().zip(moved.smooth_boundary_normals().unwrap()) {
                let expected = [-original[1], original[0], original[2]];
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
            }
        }
    }
    #[test]
    fn failed_second_refinement_child_rolls_back_successful_first_child() {
        let mut material = Body::new(
            vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            vec![true; 4],
            vec![(
                [0, 1, 2, 3],
                Material {
                    shear_pa: 1e-8,
                    bulk_pa: 1e-8,
                    fibers: vec![],
                },
            )],
        )
        .unwrap();
        material
            .set_viscoelastic_ogden(
                0,
                ViscoelasticOgden::new(
                    vec![OgdenTerm {
                        shear_pa: 1e-8,
                        exponent: 2.,
                    }],
                    1e-8,
                    vec![MaxwellBranch {
                        shear_pa: 1e-8,
                        relaxation_seconds: 0.2,
                    }],
                )
                .unwrap(),
            )
            .unwrap();
        let mut body =
            InertialBody::new_viscoelastic_with_supports(material, &[1.], vec![[0.; 3]; 4])
                .unwrap();
        let controls: Vec<_> = body
            .body()
            .positions()
            .iter()
            .enumerate()
            .map(|(node, p)| SupportTarget {
                node,
                position_m: if node == 2 { [0., -0.5, 0.] } else { *p },
            })
            .collect();
        let middle: Vec<_> = controls
            .iter()
            .map(|target| SupportTarget {
                node: target.node,
                position_m: std::array::from_fn(|axis| {
                    (body.body().positions()[target.node][axis] + target.position_m[axis]) * 0.5
                }),
            })
            .collect();
        let mut proof = body.clone();
        let first = TissueDemo::advance_continuum(&mut proof, &middle, 1. / 480., 1).unwrap();
        assert!(first.accepted_steps > 0 && first.heat_j > 0.);
        assert!(TissueDemo::advance_continuum(&mut proof, &controls, 1. / 480., 1).is_err());
        let before = format!("{body:?}");
        assert!(TissueDemo::advance_continuum(&mut body, &controls, 1. / 240., 0).is_err());
        assert_eq!(format!("{body:?}"), before);
    }
    #[test]
    fn external_palette_failure_preserves_frame_clock_and_shifted_reference() {
        let centers = std::array::from_fn(|i| {
            let mut center = TissueDemo::center(i);
            center[0] += 0.3;
            center[1] += 0.7;
            center
        });
        let mut demo = TissueDemo::body_at_centers(centers, [0, 0, 3, 7]);
        let identity = vec![Mat4::IDENTITY; 9];
        for offset in demo.secondary_offsets_for_palette(&identity) {
            assert!(offset.iter().all(|value| value.abs() < 1e-15));
        }
        let before = format!("{demo:?}");
        let mut sampled = Vec::new();
        assert_eq!(
            demo.advance_with_palette(2. / 240., |time| {
                sampled.push(time);
                if sampled.len() == 2 {
                    Err("fixture palette failure")
                } else {
                    Ok(identity.clone())
                }
            }),
            Err("fixture palette failure")
        );
        assert_eq!(sampled, vec![1. / 240., 2. / 240.]);
        assert_eq!(format!("{demo:?}"), before);
        demo.advance_with_palette(2. / 240., |_| Ok(identity.clone()))
            .unwrap();
        assert_eq!(demo.time, 2. / 240.);
        assert_eq!(demo.accumulator, 0.);
        assert!(demo.tissue_mesh().unwrap().vertices().len() > 100);
    }
    #[test]
    fn identity_bone_preserves_fem_attachment_precision() {
        let mut demo = TissueDemo::body();
        let palette = vec![Mat4::IDENTITY; 9];
        demo.step_body(&palette).unwrap();
        for (body, (_, pins)) in demo.bodies.iter().zip(&demo.attachments) {
            for (node, rest) in pins {
                assert_eq!(body.positions()[*node], *rest);
            }
        }
        assert!(
            demo.attachments
                .iter()
                .flat_map(|(_, pins)| pins)
                .any(|(_, rest)| rest.iter().any(|value| f64::from(*value as f32) != *value))
        );
    }
    #[test]
    fn imported_rig_support_motion_refines_without_weakening_admission() {
        let model = voxy_render::ModelAsset::parse(
            include_bytes!("../../../assets/animation/cesium-man/CesiumMan.glb"),
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap();
        let mut demo = TissueDemo::body();
        let names = [
            "Skeleton_torso_joint_2",
            "Skeleton_torso_joint_2",
            "leg_joint_R_1",
            "leg_joint_L_1",
        ];
        for ((joint, _), name) in demo.attachments.iter_mut().zip(names) {
            *joint = usize::from(model.resolve_joint_name(name).unwrap());
        }
        let reference = model
            .sample_pose_phase(Some(0), 0.)
            .unwrap()
            .skin_matrices(&model.skeleton)
            .unwrap();
        let duration = f64::from(model.animations[0].duration());
        let steps = (duration * 240.).ceil() as u32;
        for step in 1..=steps {
            let phase = (f64::from(step) / (240. * duration)).min(1.);
            let current = model
                .sample_pose_phase(Some(0), phase)
                .unwrap()
                .skin_matrices(&model.skeleton)
                .unwrap();
            // Material reference is the first imported pose. Relative bone motion
            // maps those already-world-space supports; no second skin pass on FEM.
            let palette: Vec<_> = current
                .iter()
                .zip(&reference)
                .map(|(current, rest)| *current * rest.inverse())
                .collect();
            demo.step_body(&palette)
                .unwrap_or_else(|error| panic!("imported rig step {step}: {error}"));
            for (body, (joint, pins)) in demo.bodies.iter().zip(&demo.attachments) {
                for (node, rest) in pins {
                    let expected =
                        DMat4::from_cols_array(&palette[*joint].to_cols_array().map(f64::from))
                            .transform_point3(DVec3::from_array(*rest))
                            .to_array();
                    assert_eq!(body.positions()[*node], expected);
                }
            }
            let surface = demo.mesh().unwrap();
            assert!(
                surface
                    .vertices()
                    .iter()
                    .flat_map(|vertex| vertex.position)
                    .all(f32::is_finite)
            );
        }
        let counts = demo.body_step_counts();
        println!("IMPORTED RIG refinement receipts: {counts:?}");
        assert!(counts.iter().any(|(_, _, depth)| *depth > 8));
        for receipt in demo.body_energy_receipts().unwrap() {
            assert!(receipt.iter().all(|value| value.is_finite()));
            assert!((receipt[0] - receipt[1] + receipt[2] - receipt[3]).abs() < 1e-8);
        }
    }
    #[test]
    fn mannequin_regions_have_si_dimensions_and_volume_derived_mass() {
        let demo = TissueDemo::body();
        let mut total = 0.;
        for (i, body) in demo.bodies.iter().enumerate() {
            let DemoTissue::Continuum { dynamics, .. } = body else {
                panic!("continuum required");
            };
            let volume = body.volumes().iter().sum::<f64>();
            let mass = dynamics.diagnostics().unwrap().mass_kg;
            assert!((mass - volume * 1000.).abs() < 1e-10);
            assert!((0.1..2.).contains(&mass));
            let center = TissueDemo::center(i);
            let radii = if i < 2 {
                [0.08, 0.065, 0.05]
            } else {
                [0.09, 0.09, 0.065]
            };
            for axis in 0..3 {
                let extent = body
                    .positions()
                    .iter()
                    .map(|point| (point[axis] - center[axis]).abs())
                    .fold(0.0_f64, f64::max);
                assert!((extent - radii[axis]).abs() < 1e-12);
            }
            total += mass;
        }
        assert!((1.0..5.).contains(&total));
        println!("MANNEQUIN SOFT REGION MASS kg={total}");
    }
    #[test]
    fn invalid_frame_duration_preserves_clock_and_all_modes() {
        for mut demo in [
            TissueDemo::body(),
            TissueDemo::new(),
            TissueDemo::biomechanics().unwrap(),
        ] {
            demo.advance(0.5 / 240.).unwrap();
            let before = format!("{demo:?}");
            for dt in [-0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert_eq!(demo.advance(dt), Err("invalid tissue frame duration"));
                assert_eq!(format!("{demo:?}"), before);
            }
            demo.advance(0.).unwrap();
            assert_eq!(format!("{demo:?}"), before);
        }
    }
    #[test]
    fn second_substep_failure_rolls_back_complete_frame_and_recovers() {
        let mut demo = TissueDemo::body();
        let mut control = TissueDemo::body();
        demo.advance(0.5 / 240.).unwrap();
        control.advance(0.5 / 240.).unwrap();
        let before = format!("{demo:?}");
        let mut calls = 0;
        let result = demo.advance_with_body_step(0.05, |candidate, time| {
            candidate.step_body(&candidate.body_palette_at(time))?;
            calls += 1;
            if calls == 2 {
                Err("injected second substep failure")
            } else {
                Ok(())
            }
        });
        assert_eq!(calls, 2);
        assert_eq!(result, Err("injected second substep failure"));
        assert_eq!(format!("{demo:?}"), before);
        demo.advance(0.05).unwrap();
        control.advance(0.05).unwrap();
        assert_eq!(format!("{demo:?}"), format!("{control:?}"));
        assert!(!demo.mesh().unwrap().vertices().is_empty());
    }
    #[test]
    fn failed_last_bone_keeps_all_tissue_states() {
        let mut demo = TissueDemo::body();
        let before = format!("{:?}", demo.bodies);
        let mut palette = demo.body_palette();
        palette[7] = Mat4::from_cols_array(&[f32::NAN; 16]);
        assert!(demo.step_body(&palette).is_err());
        assert_eq!(format!("{:?}", demo.bodies), before);
        let valid = demo.body_palette();
        assert_eq!(
            demo.step_body(&valid[..7]),
            Err("missing tissue attachment bone")
        );
        assert_eq!(format!("{:?}", demo.bodies), before);
        demo.step_body(&valid).unwrap();
    }
    #[test]
    fn parallel_contact_second_frame_failure_preserves_clock_and_all_regions() {
        let mut demo = TissueDemo::body();
        let mut control = TissueDemo::body();
        let sampler = TissueDemo::body();
        let last_joint = demo.attachments.last().unwrap().0;
        let surface = Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-10., -10., -10.], [10., -10., -10.], [0., 10., -10.]],
                vec![[0, 1, 2]],
                0.0001,
                0.003,
                100.,
            )
            .unwrap(),
        );
        let surfaces = vec![surface; demo.bodies.len()];
        demo.bind_contact_surfaces(&surfaces).unwrap();
        control.bind_contact_surfaces(&surfaces).unwrap();
        demo.advance(0.5 / 240.).unwrap();
        control.advance(0.5 / 240.).unwrap();
        let before = format!("{demo:?}");
        let mut calls = 0;
        let result = demo.advance_with_palette64_and_surfaces(2. / 240., |time| {
            calls += 1;
            let mut palette: Vec<_> = sampler
                .body_palette_at(time)
                .iter()
                .map(|m| DMat4::from_cols_array(&m.to_cols_array().map(f64::from)))
                .collect();
            if calls == 2 {
                palette[last_joint] = DMat4::from_cols_array(&[f64::NAN; 16]);
            }
            Ok((palette, surfaces.clone()))
        });
        assert_eq!(result, Err("nonfinite tissue attachment"));
        assert_eq!(calls, 2);
        assert_eq!(format!("{demo:?}"), before);
        for owner in [&mut demo, &mut control] {
            owner
                .advance_with_palette64_and_surfaces(2. / 240., |time| {
                    let palette = sampler
                        .body_palette_at(time)
                        .iter()
                        .map(|m| DMat4::from_cols_array(&m.to_cols_array().map(f64::from)))
                        .collect();
                    Ok((palette, surfaces.clone()))
                })
                .unwrap();
        }
        assert_eq!(format!("{demo:?}"), format!("{control:?}"));
    }
    #[test]
    fn parallel_regions_match_serial_state_and_preserve_error_order_and_rollback() {
        let mut serial = TissueDemo::body();
        let mut parallel = TissueDemo::body();
        let surface = Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-10., -10., -10.], [10., -10., -10.], [0., 10., -10.]],
                vec![[0, 1, 2]],
                0.0001,
                0.003,
                100.,
            )
            .unwrap(),
        );
        let surfaces = vec![surface; serial.bodies.len()];
        serial.bind_contact_surfaces(&surfaces).unwrap();
        parallel.bind_contact_surfaces(&surfaces).unwrap();
        for step in 1..=12 {
            let palette: Vec<_> = serial
                .body_palette_at(step as f64 / 240.)
                .iter()
                .map(|m| DMat4::from_cols_array(&m.to_cols_array().map(f64::from)))
                .collect();
            serial
                .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces.clone()), 1)
                .unwrap();
            parallel
                .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces.clone()), 4)
                .unwrap();
            assert_eq!(format!("{serial:?}"), format!("{parallel:?}"));
        }
        let palette: Vec<_> = serial
            .body_palette()
            .iter()
            .map(|m| DMat4::from_cols_array(&m.to_cols_array().map(f64::from)))
            .collect();
        let before = format!("{parallel:?}");
        let mut invalid = palette.clone();
        let first_joint = parallel.attachments[0].0;
        invalid[first_joint] = DMat4::from_cols_array(&[f64::NAN; 16]);
        invalid.truncate(first_joint + 1);
        assert_eq!(
            parallel.step_body_with_contact64_workers(&invalid, 0.5, Some(surfaces.clone()), 4),
            Err("nonfinite tissue attachment")
        );
        assert_eq!(format!("{parallel:?}"), before);
        let mut invalid = palette.clone();
        let last_joint = parallel.attachments.last().unwrap().0;
        invalid[last_joint] = DMat4::from_cols_array(&[f64::NAN; 16]);
        assert!(
            parallel
                .step_body_with_contact64_workers(&invalid, 0.5, Some(surfaces.clone()), 4)
                .is_err()
        );
        assert_eq!(format!("{parallel:?}"), before);
        serial
            .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces.clone()), 1)
            .unwrap();
        parallel
            .step_body_with_contact64_workers(&palette, 0.5, Some(surfaces), 4)
            .unwrap();
        assert_eq!(format!("{serial:?}"), format!("{parallel:?}"));
    }
    #[test]
    fn rotating_bones_drive_pins_and_free_tissue_response() {
        let mut driven = TissueDemo::body();
        let mut translated = TissueDemo::body();
        for step in 1..=480 {
            let time = step as f64 / 240.;
            driven.time = time;
            let palette = driven.body_palette();
            driven.step_body(&palette).unwrap();
            let root_only =
                vec![Mat4::from_translation(Vec3::Y * TissueDemo::lift(time) as f32); 9];
            translated.step_body(&root_only).unwrap();
            for (body, (joint, pins)) in driven.bodies.iter().zip(&driven.attachments) {
                for (index, rest) in pins {
                    let expected =
                        DMat4::from_cols_array(&palette[*joint].to_cols_array().map(f64::from))
                            .transform_point3(DVec3::from_array(*rest));
                    assert_eq!(body.positions()[*index], expected.to_array());
                }
            }
        }
        for (rotating, control) in driven.bodies.iter().zip(&translated.bodies) {
            let distance = rotating.positions()[0]
                .iter()
                .zip(control.positions()[0])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(
                distance > 0.001,
                "free tissue ignored bone rotation: {distance}"
            );
            assert!(rotating.positions().iter().flatten().all(|v| v.is_finite()));
        }
        driven.mesh().unwrap();
    }
    #[test]
    fn skeletal_limbs_move_and_child_pivots_remain_connected() {
        let mut demo = TissueDemo::body();
        let rest = demo.body_palette();
        demo.time = 0.7;
        let moving = demo.body_palette();
        for (start, side) in [(1, -1.), (5, 1.)] {
            let elbow = Vec3::new(side * 0.55, 0.35, 0.);
            let knee = Vec3::new(side * 0.2, -0.91, 0.);
            assert!(
                moving[start]
                    .transform_point3(elbow)
                    .distance(moving[start + 1].transform_point3(elbow))
                    < 1e-6
            );
            assert!(
                moving[start + 2]
                    .transform_point3(knee)
                    .distance(moving[start + 3].transform_point3(knee))
                    < 1e-6
            );
            let wrist = Vec3::new(side * 0.55, -0.19, 0.);
            assert!(
                moving[start + 1]
                    .transform_point3(wrist)
                    .distance(rest[start + 1].transform_point3(wrist))
                    > 0.1
            );
        }
        let count = demo.mesh().unwrap().vertices().len();
        demo.time = 9.;
        assert!(
            demo.body_palette()
                .iter()
                .all(|m| m.abs_diff_eq(Mat4::IDENTITY, 1e-6))
        );
        assert_eq!(demo.mesh().unwrap().vertices().len(), count);
        for boundary in [4., 8., 12.] {
            demo.time = boundary - 1e-6;
            let before = demo.body_palette();
            demo.time = boundary + 1e-6;
            let after = demo.body_palette();
            assert!(
                before
                    .iter()
                    .zip(after)
                    .all(|(a, b)| a.abs_diff_eq(b, 1e-4)),
                "pose jumps at {boundary}"
            );
        }
    }
    #[test]
    fn conduction_changes_heat_distribution_without_changing_mechanics() {
        let mut slow = TissueDemo::body();
        let mut fast = TissueDemo::body();
        for frame in 1..=24 {
            let palette = slow.body_palette_at(f64::from(frame) / 240.);
            slow.step_body_with_conductivity(&palette, 0.5).unwrap();
            fast.step_body_with_conductivity(&palette, 5000.).unwrap();
        }
        let mut changed = false;
        for (a, b) in slow.bodies.iter().zip(&fast.bodies) {
            let (
                DemoTissue::Continuum {
                    dynamics: a,
                    ledger: la,
                    ..
                },
                DemoTissue::Continuum {
                    dynamics: b,
                    ledger: lb,
                    ..
                },
            ) = (a, b)
            else {
                panic!()
            };
            assert_eq!(a.body().positions(), b.body().positions());
            assert_eq!(a.velocities(), b.velocities());
            let ta = a.maxwell_temperatures_kelvin().unwrap();
            let tb = b.maxwell_temperatures_kelvin().unwrap();
            changed |= ta.iter().zip(tb).any(|(a, b)| (a - b).abs() > 1e-9);
            for (body, ledger) in [(a, la), (b, lb)] {
                let stored = body
                    .maxwell_sensible_energy_j()
                    .unwrap()
                    .iter()
                    .sum::<f64>();
                assert!((stored - ledger.heat_j - ledger.conduction_defect_j).abs() < 1e-8);
            }
        }
        assert!(changed, "conduction was not executed");
        let before = format!("{:?}", fast.bodies);
        assert!(
            fast.step_body_with_conductivity(&fast.body_palette(), f64::NAN)
                .is_err()
        );
        assert_eq!(before, format!("{:?}", fast.bodies));
    }
    #[test]
    fn body_walk_jump_and_settle() {
        let mut d = TissueDemo::body();
        let n = d.mesh().unwrap().vertices().len();
        let mut low = [1e9_f64; 4];
        let mut high = [-1e9_f64; 4];
        let rest_volumes: Vec<_> = d
            .bodies
            .iter()
            .map(|b| b.volumes().iter().map(|v| v.abs()).sum::<f64>())
            .collect();
        let mut max_volume_error = [0.0_f64; 4];
        for _ in 0..1600 {
            d.advance(1.0 / 240.0).unwrap();
            for (i, body) in d.bodies.iter().enumerate() {
                let p = body.positions();
                let relative = p[0][1] - p[3][1];
                low[i] = low[i].min(relative);
                high[i] = high[i].max(relative);
                let volume: f64 = body.volumes().iter().map(|v| v.abs()).sum();
                max_volume_error[i] =
                    max_volume_error[i].max((volume / rest_volumes[i] - 1.).abs());
            }
        }
        for i in 0..4 {
            assert!(
                max_volume_error[i] < 0.05,
                "physical tissue volume drift for region {i}: {}",
                max_volume_error[i]
            );
            assert!(
                high[i] - low[i] > 0.005,
                "no secondary motion for region {i}"
            );
        }
        for _ in 0..1000 {
            d.advance(1.0 / 240.0).unwrap();
        }
        let previous: Vec<_> = d.bodies.iter().map(|b| b.positions()[0]).collect();
        d.advance(0.1).unwrap();
        for (body, p) in d.bodies.iter().zip(previous) {
            assert!(
                (body.positions()[0][1] - p[1]).abs() < 0.01,
                "motion did not settle"
            );
        }
        for ((minimum, maximum, stored), [energy, work, heat, defect]) in d
            .body_thermal_diagnostics()
            .unwrap()
            .into_iter()
            .zip(d.body_energy_receipts().unwrap())
        {
            assert!(minimum >= 310.15 && maximum > 310.15);
            assert!((stored - heat).abs() < 1e-8);
            assert!(heat > 0.);
            assert!((energy - work + heat - defect).abs() < 1e-8);
            assert!(defect.abs() < 0.03, "cumulative numerical defect: {defect}");
        }
        assert_eq!(d.mesh().unwrap().vertices().len(), n);
    }
    #[test]
    fn demo_runs_ten_seconds() {
        let mut d = TissueDemo::new();
        let n = d.mesh().unwrap().vertices().len();
        for _ in 0..100 {
            d.advance(0.1).unwrap();
        }
        assert_eq!(d.mesh().unwrap().vertices().len(), n);
    }
}

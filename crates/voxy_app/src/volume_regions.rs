//! Coarse volumetric cages embedded in the imported model's rest surface.
use physics::{
    strand::SphereCollider,
    tissue::{Material, Tissue},
    tissue_surface::EmbeddedSurface,
};
use voxy_render::SceneVertex;
#[derive(Clone, Debug)]
pub(crate) struct Region {
    body: Tissue,
    rest: Vec<[f64; 3]>,
    binding: EmbeddedSurface,
    vertices: Vec<(usize, f64)>,
    collider: SphereCollider,
    pin: usize,
    embedded_rest: Vec<[f64; 3]>,
    lumped_mass_kg: Vec<f64>,
}
impl Region {
    pub(crate) fn mass_kg(&self) -> f64 {
        self.lumped_mass_kg.iter().sum()
    }
    /// Deformation relative to the moving rest cage, excluding rigid root motion.
    pub(crate) fn maximum_local_displacement(&self, bob: f64) -> f64 {
        self.body
            .positions()
            .iter()
            .zip(&self.rest)
            .map(|(p, r)| {
                (0..3)
                    .map(|axis| (p[axis] - r[axis] - if axis == 1 { bob } else { 0. }).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .fold(0., f64::max)
    }
    pub(crate) fn volume_m3(&self) -> f64 {
        self.body.volumes().iter().map(|v| v.abs()).sum()
    }
    pub(crate) fn build(mesh: &[SceneVertex]) -> Result<Vec<Self>, &'static str> {
        Self::build_parameterized(mesh, crate::body_parameters::BodyParameters::default())
    }
    pub(crate) fn build_parameterized(
        mesh: &[SceneVertex],
        parameters: crate::body_parameters::BodyParameters,
    ) -> Result<Vec<Self>, &'static str> {
        parameters.validate()?;
        // Use the same stature normalization at the reference parameters too;
        // skipping it at defaults creates an inertia jump on the first edit.
        let morph = |p: [f64; 3]| parameters.transform(p.map(|v| v as f32)).map(f64::from);
        let mut regions = Vec::new();
        for i in 0..5 {
            let anterior = i < 2 || i == 4;
            let center = if i == 4 {
                [0.0, 0.12, 0.10]
            } else { [
                if i % 2 == 0 { -0.10 } else { 0.10 },
                if i < 2 { 0.36 } else { -0.10 },
                if i < 2 { 0.10 } else { -0.11 },
            ] };
            let radii = if i == 4 { [0.14, 0.20, 0.12] } else { [0.16, 0.18, 0.16] };
            let mut rest = vec![center];
            for axis in 0..3 {
                for sign in [1.0, -1.0] {
                    let mut p = center;
                    p[axis] += sign * radii[axis];
                    rest.push(p);
                }
            }
            let mut cells = Vec::new();
            for a in [1, 2] {
                for b in [3, 4] {
                    for c in [5, 6] {
                        cells.push([0, a, b, c]);
                    }
                }
            }
            let mut edges = std::collections::BTreeSet::new();
            for t in &cells {
                for a in 0..4 {
                    for b in a + 1..4 {
                        edges.insert((t[a].min(t[b]), t[a].max(t[b])));
                    }
                }
            }
            let pin = if anterior { 6 } else { 5 };
            let mut weights = vec![1.0; 7];
            weights[3] = 0.0;
            weights[pin] = 0.0;
            let mut vertices = Vec::new();
            let mut surface = Vec::new();
            for (index, v) in mesh.iter().enumerate() {
                let p = v.position.map(f64::from);
                let distance: f64 = (0..3).map(|k| ((p[k] - center[k]) / radii[k]).abs()).sum();
                if distance < 0.95 {
                    surface.push(p);
                    vertices.push((index, (1.0 - distance / 0.95)));
                }
            }
            let binding = EmbeddedSurface::bind(&rest, &cells, &surface)?;
            let transformed_rest: Vec<_> = rest.iter().copied().map(morph).collect();
            let embedded_rest = binding.deform(&transformed_rest)?;
            // Lumped volumetric mass (illustrative density 1000 kg/m³).
            let mut mass = vec![0.; 7];
            {
                for ids in &cells {
                    let p = ids.map(|i| transformed_rest[i]);
                    let a = std::array::from_fn::<_, 3, _>(|k| p[1][k] - p[0][k]);
                    let b = std::array::from_fn::<_, 3, _>(|k| p[2][k] - p[0][k]);
                    let c = std::array::from_fn::<_, 3, _>(|k| p[3][k] - p[0][k]);
                    let cross = [
                        b[1] * c[2] - b[2] * c[1],
                        b[2] * c[0] - b[0] * c[2],
                        b[0] * c[1] - b[1] * c[0],
                    ];
                    let volume = (0..3).map(|k| a[k] * cross[k]).sum::<f64>().abs() / 6.;
                    for &index in ids {
                        mass[index] += 1000. * volume / 4.;
                    }
                }
                for index in 0..7 {
                    if weights[index] > 0. {
                        weights[index] = 1. / mass[index];
                    }
                }
            }
            let mut body = Tissue::new(
                transformed_rest.clone(),
                weights,
                edges.into_iter().map(|(a, b)| ([a, b], false)).collect(),
                cells.clone(),
                Material {
                    stretch_compliance: 2e-5,
                    volume_compliance: 1e-9,
                    damping: 8.0,
                    particle_radius: 0.002,
                },
            )?;
            body.set_hardening(25.0)?;
            let rest = transformed_rest;
            let mut collider_center = center;
            collider_center[2] += if anterior { -0.27 } else { 0.27 };
            regions.push(Self {
                body,
                rest,
                binding,
                vertices,
                embedded_rest,
                lumped_mass_kg: mass,
                collider: SphereCollider {
                    center: morph(collider_center),
                    radius: (0..3)
                        .map(|axis| {
                            let mut p = collider_center;
                            p[axis] += 0.10;
                            let a = morph(p);
                            let b = morph(collider_center);
                            (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
                        })
                        // Keep the spherical torso proxy inside its transformed
                        // axial envelope. The largest stretch inflates it through
                        // fixed skin anchors when height grows at fixed mass.
                        .fold(f64::INFINITY, f64::min),
                },
                pin,
            });
        }
        Ok(regions)
    }
    pub(crate) fn step(&mut self, dt: f64, bob: f64) -> Result<(), &'static str> {
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 || !bob.is_finite() {
            return Err("invalid volume region step");
        }
        let count = (dt * 240.0).ceil() as usize;
        let old_bob = self.body.positions()[3][1] - self.rest[3][1];
        let mut next = self.clone();
        for i in 1..=count {
            let target = old_bob + (bob - old_bob) * i as f64 / count as f64;
            next.substep(dt / count as f64, target)?;
        }
        *self = next;
        Ok(())
    }
    fn substep(&mut self, dt: f64, bob: f64) -> Result<(), &'static str> {
        for index in [3, self.pin] {
            let mut p = self.rest[index];
            p[1] += bob;
            self.body.move_pin(index, p)?;
        }
        let mut collider = self.collider;
        collider.center[1] += bob;
        self.body
            .step_with_contact(dt, [0.0, -2.0, 0.0], &[collider], 32, 0.35)
    }
    pub(crate) fn gpu_displacement_weights(
        &self,
        base: u32,
        rows: &mut [Vec<voxy_render::SurfaceDeformationWeight>],
    ) {
        for ((vertex, blend), binding) in self
            .vertices
            .iter()
            .zip(self.binding.displacement_bindings())
        {
            if let Some((indices, weights)) = binding {
                for (node, weight) in indices.into_iter().zip(weights) {
                    rows[*vertex].push(voxy_render::SurfaceDeformationWeight {
                        control: base + node as u32,
                        weight: (weight * blend) as f32,
                    });
                }
            }
        }
    }
    pub(crate) fn gpu_displacements(&self, bob: f64) -> Vec<[f32; 4]> {
        self.body
            .positions()
            .iter()
            .zip(&self.rest)
            .map(|(p, r)| {
                [
                    (p[0] - r[0]) as f32,
                    (p[1] - r[1] - bob) as f32,
                    (p[2] - r[2]) as f32,
                    0.,
                ]
            })
            .collect()
    }
    pub(crate) fn apply(&self, _rest: &[SceneVertex], mesh: &mut [SceneVertex], bob: f64) {
        let points = self
            .binding
            .deform(self.body.positions())
            .expect("finite validated volume");
        for (((index, weight), p), embedded) in
            self.vertices.iter().zip(points).zip(&self.embedded_rest)
        {
            for axis in 0..3 {
                let delta = p[axis] - embedded[axis] - if axis == 1 { bob } else { 0.0 };
                mesh[*index].position[axis] += (delta * weight) as f32;
            }
        }
    }
}

#[cfg(test)]
mod collider_rebase_regression {
    #[test]
    fn fixed_mass_height_changes_keep_torso_proxy_outside_pins() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        for height in [130., 182., 210.] {
            for weight in [35., 60., 180.] {
                let parameters=crate::body_parameters::BodyParameters::default().patched(&serde_json::json!({"height_cm":height,"weight_kg":weight,"breast_size":1.3})).unwrap();
                let mut regions =
                    super::Region::build_parameterized(asset.mesh.vertices(), parameters).unwrap();
                for region in &mut regions {
                    for pin in [3, region.pin] {
                        let distance = (0..3)
                            .map(|k| (region.rest[pin][k] - region.collider.center[k]).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        assert!(
                            distance > region.collider.radius + 0.002,
                            "height {height}, weight {weight}"
                        );
                    }
                    region.step(1. / 240., 0.).unwrap();
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_model_regions_preserve_volume_and_recover_after_load() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mut regions = Region::build(asset.mesh.vertices()).unwrap();
        assert!(regions.iter().all(|r| r.vertices.len() > 20));
        for r in &mut regions {
            let volume: f64 = r.body.volumes().iter().map(|v| v.abs()).sum();
            for _ in 0..1000 {
                r.step(1. / 240., 0.0).unwrap();
            }
            let baseline = r.body.positions().to_vec();
            for frame in 0..20 {
                r.step(
                    0.05,
                    0.025 * (frame as f64 * 0.05 * std::f64::consts::TAU * 1.5).sin(),
                )
                .unwrap();
            }
            for frame in 0..1000 {
                r.step(
                    1. / 240.,
                    0.025 * (frame as f64 / 240. * std::f64::consts::TAU * 1.5).sin(),
                )
                .unwrap();
                let current: f64 = r.body.volumes().iter().map(|v| v.abs()).sum();
                assert!((current / volume - 1.0).abs() < 0.1);
            }
            for _ in 0..1500 {
                r.step(1. / 240., 0.0).unwrap();
            }
            for (p, b) in r.body.positions().iter().zip(baseline) {
                for axis in 0..3 {
                    assert!((p[axis] - b[axis]).abs() < 0.003);
                }
            }
        }
    }
}

#[cfg(test)]
mod parameterized_tests {
    #[test]
    fn tiny_height_edit_keeps_cage_inertia_continuous() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = super::Region::build(asset.mesh.vertices()).unwrap();
        let parameters = crate::body_parameters::BodyParameters {
            height_cm: 164.0001,
            ..Default::default()
        };
        let edited = super::Region::build_parameterized(asset.mesh.vertices(), parameters).unwrap();
        for (a, b) in original.iter().zip(&edited) {
            assert!((a.mass_kg() - b.mass_kg()).abs() / a.mass_kg() < 1e-5);
            for (&before, &after) in a.body.inverse_masses().iter().zip(b.body.inverse_masses()) {
                if before == 0. {
                    assert_eq!(after, 0.);
                } else {
                    assert!((after - before).abs() / before < 1e-5);
                }
            }
        }
    }
    #[test]
    fn rebuilding_cages_changes_physical_rest_geometry_and_preserves_unloaded_surface() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mut mesh = asset.mesh.vertices().to_vec();
        let p = crate::body_parameters::BodyParameters {
            height_cm: 182.,
            weight_kg: 78.,
            breast_size: 1.3,
            leg_length: 1.15,
            ..Default::default()
        };
        let original = super::Region::build(&mesh).unwrap();
        for region in &original {
            assert!((region.mass_kg() - 1000. * region.volume_m3()).abs() < 1e-12);
            for (i, &weight) in region.body.inverse_masses().iter().enumerate() {
                if i == 3 || i == region.pin {
                    assert_eq!(weight, 0.);
                } else {
                    assert!((weight * region.lumped_mass_kg[i] - 1.).abs() < 1e-12);
                }
            }
        }
        let mut regions = super::Region::build_parameterized(&mesh, p).unwrap();
        for region in &regions {
            assert!((region.mass_kg() - 1000. * region.volume_m3()).abs() < 1e-12);
        }
        assert!(
            regions
                .iter()
                .all(|r| r.maximum_local_displacement(0.) < 1e-12)
        );
        assert_ne!(original[0].rest, regions[0].rest);
        p.apply(&mut mesh);
        let before = mesh.clone();
        for r in &regions {
            r.apply(asset.mesh.vertices(), &mut mesh, 0.);
        }
        assert!(
            mesh.iter()
                .zip(&before)
                .all(|(a, b)| (0..3).all(|k| (a.position[k] - b.position[k]).abs() < 1e-6))
        );
        for _ in 0..120 {
            for r in &mut regions {
                r.step(1. / 120., 0.).unwrap();
            }
        }
        assert!(
            regions
                .iter()
                .any(|r| r.maximum_local_displacement(0.) > 1e-6)
        );
        for r in &regions {
            r.apply(asset.mesh.vertices(), &mut mesh, 0.);
        }
        assert!(
            mesh.iter()
                .all(|v| v.position.iter().all(|p| p.is_finite()))
        );
    }
}

#[cfg(test)]
mod gpu_transfer_regression {
    #[test]
    fn sparse_gpu_weights_match_existing_region_transfer_after_dynamics() {
        let mesh = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap()
        .mesh;
        let mut regions = super::Region::build(mesh.vertices()).unwrap();
        let mut rows = vec![Vec::new(); mesh.vertices().len()];
        for (i, region) in regions.iter().enumerate() {
            region.gpu_displacement_weights(i as u32 * 7, &mut rows);
        }
        for frame in 0..24 {
            let bob = 0.015 * (frame as f64 / 24. * std::f64::consts::TAU).sin();
            for region in &mut regions {
                region.step(1. / 240., bob).unwrap();
            }
            let mut cpu = mesh.vertices().to_vec();
            for region in &regions {
                region.apply(mesh.vertices(), &mut cpu, bob);
            }
            let controls: Vec<_> = regions
                .iter()
                .flat_map(|r| r.gpu_displacements(bob))
                .collect();
            let mut error = 0_f32;
            for (i, row) in rows.iter().enumerate() {
                for axis in 0..3 {
                    let delta: f32 = row
                        .iter()
                        .map(|w| w.weight * controls[w.control as usize][axis])
                        .sum();
                    let gpu = mesh.vertices()[i].position[axis] + delta;
                    error = error.max((gpu - cpu[i].position[axis]).abs());
                }
            }
            assert!(error < 1e-6, "Sparse transfer mismatch: {error}");
        }
    }
}

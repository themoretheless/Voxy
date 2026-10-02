//! Procedural shell and a solver-driven muscle ring; illustrative, not anatomy.
use glam::Vec3;
use physics::tissue::{Tissue, TissueKind, sample};
use voxy_render::{SceneError, SceneMesh, SceneVertex};

#[derive(Debug)]
pub(crate) struct XrayDemo {
    ring: Tissue,
    model: Option<voxy_render::ObjAsset>,
    pub closeup: bool,
    time: f64,
    accumulator: f64,
    pub enabled: bool,
    effect: voxy_render::XrayEffect,
    pub slow: bool,
    pub local: bool,
}
impl XrayDemo {
    pub fn new() -> Self {
        Self {
            model: None,
            closeup: false,
            ring: sample(TissueKind::Sphincter, [0.0; 3]).expect("valid ring"),
            time: 0.0,
            accumulator: 0.0,
            enabled: true,
            effect: voxy_render::XrayEffect::default(),
            slow: false,
            local: false,
        }
    }
    pub fn with_model() -> Result<Self, Box<dyn std::error::Error>> {
        let mut demo = Self::new();
        demo.model = Some(voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )?);
        demo.closeup = true;
        demo.effect.set_enabled(true);
        demo.effect.advance(0.3)?;
        Ok(demo)
    }
    pub fn focus(&self) -> Vec3 {
        if self.model.is_some() && self.closeup {
            Vec3::new(0.0, -0.025, -0.025)
        } else {
            Vec3::ZERO
        }
    }
    pub fn eye(&self, model: glam::Mat4) -> Vec3 {
        model.transform_point3(self.focus())
            + if self.model.is_some() && self.closeup {
                Vec3::new(0.0, 0.28, 0.34)
            } else {
                Vec3::new(0.0, 0.12, if self.model.is_some() { 2.1 } else { 3.0 })
            }
    }
    fn ring_point(&self, p: Vec3) -> Vec3 {
        if self.model.is_some() {
            Vec3::new(p.x, p.z, -p.y) * 0.075 + Vec3::new(0.0, -0.025, -0.025)
        } else {
            p
        }
    }
    pub fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        if !dt.is_finite() || dt < 0.0 {
            return Err("invalid X-ray delta");
        }
        self.effect.set_region(if self.local {
            Some(
                voxy_render::XrayRegion::sphere(Vec3::new(0.2, 0.0, 0.0), 0.4, 0.08)
                    .expect("valid region"),
            )
        } else {
            None
        });
        self.effect.set_enabled(self.enabled);
        self.effect
            .advance(dt as f32)
            .map_err(|_| "invalid reveal delta")?;
        self.accumulator += dt.min(0.1) * if self.slow { 0.15 } else { 1.0 };
        while self.accumulator >= 1.0 / 240.0 {
            self.time += 1.0 / 240.0;
            self.ring
                .set_activation(0.5 + 0.5 * (self.time * 2.0).sin())?;
            self.ring.step(1.0 / 240.0, [0.0; 3], &[], 24)?;
            self.accumulator -= 1.0 / 240.0;
        }
        Ok(())
    }
    pub fn shell_depth_mode(&self) -> voxy_render::SceneDepthMode {
        self.effect.shell_depth_mode()
    }
    pub fn internal_depth_mode(&self) -> voxy_render::SceneDepthMode {
        self.effect.internal_depth_mode()
    }
    pub fn title(&self) -> String {
        format!(
            "Voxy X-ray {} | F: body/pelvis | X: X-ray {} | S: slow {} | L: local region | arrows: orbit | Space: pause | Esc: exit",
            if self.model.is_some() {
                if self.closeup { "pelvis" } else { "body" }
            } else {
                "abstract"
            },
            if self.enabled { "ON" } else { "OFF" },
            if self.slow { "ON" } else { "OFF" }
        )
    }
    pub fn shell(&self, eye: Vec3) -> Result<SceneMesh, SceneError> {
        if let Some(model) = &self.model {
            let mut vertices = model.mesh.vertices().to_vec();
            for (i, vertex) in vertices.iter_mut().enumerate() {
                let p = Vec3::from_array(vertex.position);
                let n = Vec3::from_array(model.normals[i].unwrap_or([0.0, 1.0, 0.0]))
                    .normalize_or_zero();
                let rim = (1.0 - n.dot((eye - p).normalize_or_zero()).abs()).powi(3);
                let shade = 0.3 + 0.7 * n.dot(Vec3::new(0.3, 0.5, 1.0).normalize()).max(0.0);
                vertex.color = self
                    .effect
                    .shell_color([0.78 * shade, 0.4 * shade, 0.3 * shade, 1.0], rim);
            }
            let mut mesh = SceneMesh::new(vertices, model.mesh.indices().to_vec())?;
            mesh.sort_back_to_front(glam::camera::rh::view::look_at_mat4(
                eye,
                self.focus(),
                Vec3::Y,
            ))?;
            return Ok(mesh);
        }
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for lat in 0..=24 {
            let v = lat as f32 / 24.0 * std::f32::consts::PI;
            for lon in 0..=48 {
                let u = lon as f32 / 48.0 * std::f32::consts::TAU;
                let n = Vec3::new(v.sin() * u.cos(), v.cos(), v.sin() * u.sin());
                let p = n * Vec3::new(0.85, 1.0, 0.55);
                let normal = (n / Vec3::new(0.85, 1.0, 0.55)).normalize_or_zero();
                let facing = normal.dot((eye - p).normalize_or_zero()).abs();
                let rim = (1.0 - facing).powi(3);
                let light = 0.35 + 0.65 * normal.dot(Vec3::new(0.3, 0.6, 1.0).normalize()).max(0.0);
                let color = self
                    .effect
                    .shell_color([0.78 * light, 0.4 * light, 0.3 * light, 1.0], rim);
                vertices.push(SceneVertex {
                    position: p.to_array(),
                    uv: [0.0; 2],
                    color,
                });
            }
        }
        for lat in 0..24 {
            for lon in 0..48 {
                let a = lat * 49 + lon;
                indices.extend([a, a + 49, a + 1, a + 1, a + 49, a + 50]);
            }
        }
        let mut mesh = SceneMesh::new(vertices, indices)?;
        mesh.sort_back_to_front(glam::camera::rh::view::look_at_mat4(
            eye,
            Vec3::ZERO,
            Vec3::Y,
        ))?;
        Ok(mesh)
    }
    pub fn internal(&self) -> Result<SceneMesh, SceneError> {
        self.internal_for_eye(Vec3::Z * 3.0)
    }
    pub fn internal_for_eye(&self, eye: Vec3) -> Result<SceneMesh, SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let points = self.ring.positions();
        for (i, p) in points.iter().enumerate() {
            let center = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
            let radial = Vec3::new(center.x, center.y, 0.0).normalize_or_zero();
            for j in 0..12 {
                let a = j as f32 / 12.0 * std::f32::consts::TAU;
                let n = radial * a.cos() + Vec3::Z * a.sin();
                let shade = 0.4 + 0.6 * n.dot(Vec3::new(0.3, 0.5, 1.0).normalize()).max(0.0);
                vertices.push(SceneVertex {
                    position: self.ring_point(center + n * 0.055).to_array(),
                    uv: [0.0; 2],
                    color: {
                        let mut color = if self.local && self.effect.amount() > 0.0 {
                            self.effect
                                .reveal_color_at([0.65, 0.3, 0.22, 1.0], center + n * 0.055)
                                .expect("finite mesh vertex")
                        } else {
                            self.effect.internal_color([0.65, 0.3, 0.22, 1.0])
                        };
                        for c in &mut color[..3] {
                            *c *= shade;
                        }
                        color
                    },
                });
                let a = (i * 12 + j) as u32;
                let b = (i * 12 + (j + 1) % 12) as u32;
                let c = (((i + 1) % points.len()) * 12 + j) as u32;
                let d = (((i + 1) % points.len()) * 12 + (j + 1) % 12) as u32;
                indices.extend([a, b, c, b, d, c]);
            }
        }
        let mut mesh = SceneMesh::new(vertices, indices)?;
        mesh.sort_back_to_front(glam::camera::rh::view::look_at_mat4(
            eye,
            Vec3::ZERO,
            Vec3::Y,
        ))?;
        Ok(mesh)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_body_is_used_as_shell_and_ring_is_scaled_into_pelvis() {
        let mut demo = XrayDemo::with_model().unwrap();
        demo.advance(0.1).unwrap();
        let shell = demo.shell(demo.eye(glam::Mat4::IDENTITY)).unwrap();
        assert_eq!(
            shell.vertices().len(),
            demo.model.as_ref().unwrap().mesh.vertices().len()
        );
        assert!(shell.vertices().len() > 10_000);
        let ring = demo.internal().unwrap();
        assert!(ring.vertices().iter().all(|v| {
            let p = Vec3::from_array(v.position);
            p.x.abs() < 0.04 && (p.y + 0.025).abs() < 0.01 && (p.z + 0.025).abs() < 0.04
        }));
        demo.closeup = true;
        demo.shell(demo.eye(glam::Mat4::IDENTITY)).unwrap();
    }
    #[test]
    fn local_reveal_masks_only_part_of_the_ring() {
        let mut demo = XrayDemo::new();
        demo.local = true;
        for _ in 0..4 {
            demo.advance(0.1).unwrap();
        }
        let mesh = demo.internal().unwrap();
        assert!(mesh.vertices().iter().any(|v| v.color[3] == 0.0));
        assert!(mesh.vertices().iter().any(|v| v.color[3] > 0.5));
        demo.local = false;
        demo.advance(0.1).unwrap();
        assert!(
            demo.internal()
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.color[3] == 1.0)
        );
    }
    #[test]
    fn contraction_deforms_surface_without_changing_topology() {
        let mut demo = XrayDemo::new();
        let before = demo.internal().unwrap();
        for _ in 0..100 {
            demo.advance(0.1).unwrap();
        }
        let after = demo.internal().unwrap();
        assert_eq!(before.indices().len(), after.indices().len());
        assert_ne!(before.vertices(), after.vertices());
        demo.shell(Vec3::new(0.0, 0.2, 3.0)).unwrap();
        demo.enabled = false;
        for _ in 0..4 {
            demo.advance(0.1).unwrap();
        }
        assert!(
            demo.shell(Vec3::Z * 3.0)
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.color[3] == 1.0)
        );
        assert!(demo.advance(f64::NAN).is_err());
    }
}

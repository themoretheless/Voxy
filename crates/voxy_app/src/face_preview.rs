//! Shared character sampling for native and hardware-ray inspection.
use glam::Vec3;
use voxy_render::{SceneError, SceneMesh};

/// Deterministic facial/skeletal preview of the existing character assets.
#[derive(Debug)]
pub struct FacePreview {
    demo: crate::female_demo::FemaleDemo,
}
impl FacePreview {
    /// Returns isolated experimental lid patches at closure 0..1.
    /// These surfaces are not stitched into the character or bound to lashes.
    /// # Errors
    /// Rejects invalid closure, source assets or generated geometry.
    pub fn lid_surface(closure: f32) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )?;
        Ok(
            crate::female_lids::LidSurface::new(body.mesh.vertices(), body.mesh.indices())?
                .mesh(closure)?,
        )
    }
    /// Isolated lid/globe contact inspection using the character's real eye assets.
    /// # Errors
    /// Returns asset or surface errors; this does not modify the full character.
    pub fn lid_surface_with_eyes(closure: f32) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )?;
        let lids = crate::female_lids::LidSurface::new(body.mesh.vertices(), body.mesh.indices())?
            .mesh_with_canthi(closure)?;
        Self::append_eye_globes(lids)
    }
    /// Bind-space research view of replacement lids stitched geometrically to skin.
    /// # Errors
    /// Returns asset/clipping errors. Rig, groom and morphology integration is pending.
    pub fn lid_surface_with_head(closure: f32) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )?;
        let surface =
            crate::female_lids::LidSurface::new(body.mesh.vertices(), body.mesh.indices())?;
        Self::append_eye_globes(surface.mesh_with_body(&body.mesh, closure)?)
    }
    fn append_eye_globes(lids: SceneMesh) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let mut vertices = lids.vertices().to_vec();
        let mut indices = lids.indices().to_vec();
        for source in [
            include_str!("../../../assets/characters/blender-female/eye-l.obj"),
            include_str!("../../../assets/characters/blender-female/eye-r.obj"),
        ] {
            let eye = voxy_render::ObjAsset::parse(source, voxy_render::ObjLimits::default())?;
            let (mut globe, triangles) = crate::female_eyes::refined_globe(&eye.mesh);
            let base = vertices.len() as u32;
            for vertex in &mut globe {
                let point = Vec3::from_array(vertex.position);
                let normal = (point - crate::female_eyes::center(point.x)).normalize();
                vertex.uv = crate::female_eyes::uv(point);
                vertex.color = [
                    0.5 + 0.5 * normal.x,
                    0.5 + 0.5 * normal.y,
                    0.5 + 0.5 * normal.z,
                    1.,
                ];
            }
            vertices.extend(globe);
            indices.extend(triangles.into_iter().map(|i| i + base));
        }
        Ok(SceneMesh::new(vertices, indices)?)
    }
    /// # Errors
    /// Returns asset, rig or physical-shell initialization errors.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut demo = crate::female_demo::FemaleDemo::new()?;
        demo.animation_only = true;
        Ok(Self { demo })
    }
    /// Samples a pose without advancing the nonlinear skin/hair solvers.
    /// An optional jaw value exposes oral geometry for inspection.
    /// # Errors
    /// Rejects nonfinite inputs, out-of-range jaw values or invalid resulting geometry.
    pub fn sample(
        &mut self,
        time: f64,
        eye: Vec3,
        jaw: Option<f32>,
    ) -> Result<SceneMesh, SceneError> {
        self.sample_oral(time, eye, jaw, 0., 0.)
    }
    /// Samples independent tongue lift (-1..1) and forward motion (0..1).
    /// # Errors
    /// Rejects nonfinite inputs and controls outside their supported ranges.
    pub fn sample_oral(
        &mut self,
        time: f64,
        eye: Vec3,
        jaw: Option<f32>,
        tongue_lift: f32,
        tongue_forward: f32,
    ) -> Result<SceneMesh, SceneError> {
        if !tongue_lift.is_finite()
            || !(-1. ..=1.).contains(&tongue_lift)
            || !tongue_forward.is_finite()
            || !(0. ..=1.).contains(&tongue_forward)
        {
            return Err(SceneError::InvalidGeometry);
        }
        if !time.is_finite()
            || !eye.is_finite()
            || jaw.is_some_and(|j| !j.is_finite() || !(0.0..=1.0).contains(&j))
        {
            return Err(SceneError::InvalidGeometry);
        }
        self.demo.preview_camera_eye = Some(eye);
        self.demo.preview_expression = jaw.map(|j| crate::female_face::FacePose {
            jaw: j,
            tongue_lift,
            tongue_forward,
            ..Default::default()
        });
        self.demo.preview_pose(time);
        self.demo.mesh()
    }
    /// Samples equal closure of both eyes independently of other expressions.
    /// # Errors
    /// Rejects nonfinite inputs and closure values outside 0..1.
    pub fn sample_blink(
        &mut self,
        time: f64,
        eye: Vec3,
        blink: f32,
    ) -> Result<SceneMesh, SceneError> {
        if !time.is_finite()
            || !eye.is_finite()
            || !blink.is_finite()
            || !(0. ..=1.).contains(&blink)
        {
            return Err(SceneError::InvalidGeometry);
        }
        self.demo.preview_camera_eye = Some(eye);
        self.demo.preview_expression = Some(crate::female_face::FacePose {
            blink,
            ..Default::default()
        });
        self.demo.preview_pose(time);
        self.demo.mesh()
    }
    /// Samples brow motion independently of blink, smile and jaw.
    /// # Errors
    /// Rejects nonfinite inputs and brow values outside -1..1.
    pub fn sample_brow(
        &mut self,
        time: f64,
        eye: Vec3,
        brow: f32,
    ) -> Result<SceneMesh, SceneError> {
        if !time.is_finite()
            || !eye.is_finite()
            || !brow.is_finite()
            || !(-1. ..=1.).contains(&brow)
        {
            return Err(SceneError::InvalidGeometry);
        }
        self.demo.preview_camera_eye = Some(eye);
        self.demo.preview_expression = Some(crate::female_face::FacePose {
            brow,
            ..Default::default()
        });
        self.demo.preview_pose(time);
        self.demo.mesh()
    }
    /// Sets a validated face preset for subsequent samples.
    pub fn set_parameters(&mut self, parameters: crate::face_parameters::FaceParameters) {
        self.demo.face_parameters = parameters;
    }
    /// Width, height and cached sRGB material atlas bytes.
    #[must_use]
    pub fn texture() -> (u32, u32, &'static [u8]) {
        (
            crate::female_complexion::WIDTH,
            crate::female_complexion::SIZE,
            crate::female_complexion::atlas(),
        )
    }
    /// Material atlas including the selected preset's crease microheight.
    #[must_use]
    pub fn material_texture(&self) -> (u32, u32, Vec<u8>) {
        (
            crate::female_complexion::WIDTH,
            crate::female_complexion::SIZE,
            crate::female_complexion::atlas_for_parameters(&self.demo.face_parameters),
        )
    }
    /// Scene shader using skin roughness/oil/microheight and eye pigmentation.
    #[must_use]
    pub const fn material_shader() -> &'static str {
        crate::female_eyes::MATERIAL_SHADER
    }
}

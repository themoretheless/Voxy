//! Retained HDR raster capture for a planar reflection camera.
use crate::{HdrMipPyramid, RendererError, SceneDraw, SceneError, SceneRenderer};

/// Projective planar surface shader. Supply both matrices using
/// `SceneTransform::update_planar_projection`; mesh UVs are ignored.
/// This shader uses the motion-history matrix slot for capture projection.
pub const PLANAR_REFLECTION_SURFACE_SHADER: &str = include_str!("planar_surface.wgsl");

/// Projective reflection with scalar Schlick Fresnel in blend alpha.
/// Set projection, world/neutral base tint, view position and PBR metallic weight.
/// Supports rigid/uniform world transforms and neutral metals; no rough filtering.
pub const PLANAR_REFLECTION_FRESNEL_SHADER: &str = include_str!("planar_fresnel.wgsl");

/// Fresnel with roughness-squared LOD selection from the HDR area mip pyramid.
/// Bind all mips with linear min/mag/mipmap filtering for continuous transitions.
/// This screen-space approximation is not a GGX angular prefilter.
pub const PLANAR_REFLECTION_ROUGH_SHADER: &str = include_str!("planar_rough.wgsl");

/// Unlit capture material clipping the negative half-space of a world plane.
/// Configure each draw with `SceneTransform::update_planar_clip`.
/// The plane occupies the point-light ABI slot; use a dedicated capture transform.
pub const PLANAR_REFLECTION_CLIP_SHADER: &str = include_str!("planar_clip.wgsl");

/// Point-light GGX capture with world-plane clipping. Configure the normal material,
/// reflected view position, then `SceneTransform::update_pbr_capture_plane`.
/// The plane occupies the first column of the previous-MVP slot; no motion history.
#[must_use]
pub fn planar_reflection_pbr_clip_shader() -> String {
    clip_pbr_source(crate::TEXTURED_POINT_LIGHT_SHADER)
}

fn clip_pbr_source(source: &str) -> String {
    source.replace(
            "@location(6) @interpolate(flat) material: vec3<f32>,",
            "@location(6) @interpolate(flat) material: vec3<f32>, @location(7) clip_distance: f32,",
        )
        .replace(
            "out.material = transform.authored.yzw;",
            "out.material = transform.authored.yzw; out.clip_distance = dot(transform.previous_mvp[0], vec4(out.world, 1.0));",
        )
        .replace(
            "if texel.a <= 0.0 { discard; }",
            "if texel.a <= 0.0 || in.clip_distance < 0.0 { discard; }",
        )
}

#[derive(Debug)]
pub struct PlanarReflectionCapture {
    device: wgpu::Device,
    renderer: SceneRenderer,
    color: HdrMipPyramid,
    depth: wgpu::TextureView,
}
impl PlanarReflectionCapture {
    /// Allocate single-sample RGBA16Float mip pyramid and conventional depth.
    /// # Errors
    /// Rejects dimensions outside the device's texture limits.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, RendererError> {
        let color = HdrMipPyramid::new(device, width, height)?;
        Ok(Self {
            device: device.clone(),
            renderer: SceneRenderer::new(device, wgpu::TextureFormat::Rgba16Float),
            color,
            depth: depth(device, width, height),
        })
    }
    /// Attach initialized environment resources and install the clipped IBL capture shader.
    /// # Errors
    /// Rejects foreign resources or shader validation.
    pub async fn enable_environment_lighting(
        &mut self,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
    ) -> Result<(), crate::SceneShaderError> {
        self.renderer
            .enable_environment_source(
                &self.device,
                environment,
                dfg,
                None,
                &clip_pbr_source(&crate::environment_lighting::shader(false)),
            )
            .await
    }

    /// Attach diffuse and specular environment lighting to the clipped capture material.
    /// # Errors
    /// Rejects foreign resources or shader validation, preserving the prior configuration.
    pub async fn enable_full_environment_lighting(
        &mut self,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: &crate::DiffuseEnvironmentConvolution,
    ) -> Result<(), crate::SceneShaderError> {
        self.renderer
            .enable_environment_source(
                &self.device,
                environment,
                dfg,
                Some(diffuse),
                &clip_pbr_source(&crate::environment_lighting::shader_combined(false, true)),
            )
            .await
    }

    /// The capture renderer creates compatible geometry, textures and transforms.
    #[must_use]
    pub fn renderer(&self) -> &SceneRenderer {
        &self.renderer
    }

    /// Update environment lighting scale in the clipped reflection renderer.
    /// # Errors
    /// Rejects invalid values or missing environment lighting.
    pub fn set_environment_intensity(
        &mut self,
        queue: &wgpu::Queue,
        intensity: f32,
    ) -> Result<(), crate::SceneShaderError> {
        self.renderer.set_environment_intensity(queue, intensity)
    }
    /// Linear HDR capture, initialized only after encoding and submitting its producer.
    #[must_use]
    pub fn color(&self) -> &wgpu::Texture {
        self.color.texture()
    }
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        self.color.base_view()
    }
    /// Bind the current linear HDR capture as a material for a subsequent scene pass.
    /// Recreate this binding after a successful resize; existing bindings retain the old image.
    /// Never sample it while rendering into this capture.
    /// # Errors
    /// Rejects a renderer owned by another device or unsupported sampling settings.
    pub fn material_binding(
        &self,
        renderer: &SceneRenderer,
        sampling: crate::TextureSampling,
    ) -> Result<crate::SceneTexture, SceneError> {
        renderer.bind_image(&self.device, self.color.managed_texture(), sampling, 1)
    }
    /// Bind every initialized HDR mip level for explicit LOD or trilinear sampling.
    /// Recreate after resize; never sample while rendering into this capture.
    /// # Errors
    /// Rejects foreign renderers and invalid sampling settings.
    pub fn mip_material_binding(
        &self,
        renderer: &SceneRenderer,
        sampling: crate::TextureSampling,
    ) -> Result<crate::SceneTexture, SceneError> {
        renderer.bind_image(
            &self.device,
            self.color.managed_texture(),
            sampling,
            self.color.texture().mip_level_count(),
        )
    }
    /// Retain attachments when size is unchanged. Failed resize preserves both attachments.
    /// # Errors
    /// Rejects dimensions outside the device's texture limits.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<bool, RendererError> {
        if self.color.texture().width() == width && self.color.texture().height() == height {
            return Ok(false);
        }
        let color = HdrMipPyramid::new(&self.device, width, height)?;
        let depth = depth(&self.device, width, height);
        self.color = color;
        self.depth = depth;
        Ok(true)
    }
    /// Select a material shader without changing capture format or attachment ownership.
    /// # Errors
    /// Returns the renderer's shader validation error, preserving the previous pipeline.
    pub async fn reload_shader(&mut self, source: &str) -> Result<(), crate::SceneShaderError> {
        self.renderer
            .reload_shader(&self.device, source)
            .await
            .map(|_| ())
    }
    /// Render draws whose transforms contain the reflected camera's view-projection.
    /// Then initialize every lower HDR mip in order; no per-frame GPU allocation.
    /// Caller excludes the reflector and clips unwanted geometry behind its plane.
    /// No submission, presentation or temporal history is committed here.
    /// # Errors
    /// Rejects foreign geometry/textures and sampling this capture before beginning a pass.
    /// Transforms and encoder
    /// must also belong to this device and remain subject to wgpu validation.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        if draws.iter().any(|draw| {
            !draw.geometry.belongs_to(&self.device) || !draw.texture.belongs_to(&self.device)
        }) {
            return Err(SceneError::DeviceMismatch);
        }
        if draws
            .iter()
            .any(|draw| draw.texture.texture() == self.color.texture())
        {
            return Err(SceneError::InvalidTexture);
        }
        self.renderer
            .encode(encoder, self.color.base_view(), &self.depth, clear, draws);
        self.color.encode(encoder);
        Ok(())
    }
}
fn depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("planar reflection capture depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

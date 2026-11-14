//! General textured triangle rendering, independent of voxel storage.

#[cfg(test)]
#[path = "fiber_surface_draw_tests.rs"]
mod fiber_surface_draw_tests;
#[cfg(test)]
#[path = "surface_color_transfer_tests.rs"]
mod surface_color_transfer_tests;
#[cfg(test)]
#[path = "surface_light_transfer_tests.rs"]
mod surface_light_transfer_tests;

/// Default shader and the resource/vertex ABI for custom scene shaders.
pub const DEFAULT_SCENE_SHADER: &str = include_str!("scene.wgsl");

use glam::Mat4;

mod encode;
mod geometry;
mod instancing;
mod resources;

#[cfg(test)]
use encode::validate_view_regions;
pub use encode::{SceneDraw, SceneView, SceneViewTargets};
#[doc(hidden)]
pub use geometry::{NormalCache, smooth_normals};
pub use geometry::{SceneError, SceneMesh, SceneVertex};
pub use instancing::{SceneInstanceBatch, SceneInstanceData, SceneInstancedDraw, SceneInstancer};
pub(crate) use resources::next_geometry_identity;
pub use resources::{
    SceneDepthMode, SceneGeometry, SceneShaderError, SceneTexture, SceneTransform, TextureFilter,
    TextureSampling, TextureWrap,
};
use resources::{UploadedGeometryBatch, geometry_sizes, managed_scene_indices, scene_memory_error};

#[derive(Debug)]
struct PreparedScenePipelines {
    solid_opaque: wgpu::RenderPipeline,
    world: wgpu::RenderPipeline,
    overlay: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    msaa: Option<(
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    )>,
}

/// Prepared bindings and validated pipelines; creation never mutates a scene.
/// Commit rejects another scene, a changed lighting/shader state, or changed MSAA.
#[derive(Debug)]
pub struct PreparedSceneEnvironment {
    owner: std::sync::Arc<()>,
    base_shader_revision: u64,
    base_shader_source: String,
    base_lighting_revision: u64,
    msaa_enabled: bool,
    environment: crate::environment_lighting::EnvironmentBindings,
    pipelines: Option<PreparedScenePipelines>,
    source: String,
    dfg: wgpu::Texture,
}

#[derive(Debug)]
pub struct SceneRenderer {
    identity: std::sync::Arc<()>,
    lighting_revision: u64,
    device: wgpu::Device,
    compute_memory_budget: crate::ComputeMemoryBudget,
    reverse_z: bool,
    solid_opaque_pipeline: wgpu::RenderPipeline,
    world_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    transform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    color_format: wgpu::TextureFormat,
    shader_source: String,
    shader_revision: u64,
    shadow: Option<crate::shadow_visibility::ShadowBindings>,
    environment: Option<crate::environment_lighting::EnvironmentBindings>,
    msaa_pipelines: Option<(
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    )>,
}

impl SceneRenderer {
    /// Attachment format used by the native scene pipelines.
    #[must_use]
    pub const fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    /// Whether depth comparisons use reversed-Z (`GreaterEqual`, clear `0.0`).
    #[must_use]
    pub const fn is_reverse_z(&self) -> bool {
        self.reverse_z
    }

    #[inline]
    pub(super) const fn depth_clear_value(&self) -> f32 {
        if self.reverse_z { 0.0 } else { 1.0 }
    }

    /// Shared device owner retained even when no scene resource is resident.
    #[must_use]
    pub fn compute_memory_budget(&self) -> &crate::ComputeMemoryBudget {
        &self.compute_memory_budget
    }

    fn resource_device(&self, supplied: &wgpu::Device) -> Result<&wgpu::Device, SceneError> {
        if supplied != &self.device {
            return Err(SceneError::DeviceMismatch);
        }
        Ok(&self.device)
    }

    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::new_with_depth(device, format, false)
    }

    /// Creates a scene renderer matching reversed-Z depth targets (`GreaterEqual`, clear `0.0`).
    #[must_use]
    pub fn new_reverse_z(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::new_with_depth(device, format, true)
    }

    fn new_with_depth(device: &wgpu::Device, format: wgpu::TextureFormat, reverse_z: bool) -> Self {
        let transform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene transform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(256),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(DEFAULT_SCENE_SHADER.into()),
        });
        // Build initial pipelines before constructing the owner.
        let mut renderer = Self::from_layouts(
            device,
            format,
            transform_layout,
            texture_layout,
            &shader,
            reverse_z,
        );
        DEFAULT_SCENE_SHADER.clone_into(&mut renderer.shader_source);
        renderer
    }

    /// Creates both single-sample and four-sample presentation pipelines.
    /// The caller must verify four-sample color/depth format support.
    #[must_use]
    pub fn new_msaa4(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let mut renderer = Self::new(device, format);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MSAA scene shader"),
            source: wgpu::ShaderSource::Wgsl(DEFAULT_SCENE_SHADER.into()),
        });
        renderer.msaa_pipelines = Some(Self::create_shader_pipelines(
            device,
            format,
            &renderer.transform_layout,
            &renderer.texture_layout,
            &shader,
            4,
            None,
            None,
            renderer.reverse_z,
            true,
        ));
        renderer
    }

    /// Adds four-sample pipelines to the current shader/layouts without replacing
    /// existing resources. Repeated enable is a no-op. The caller verifies target
    /// format support before configuring a multisampled surface.
    /// # Errors
    /// A foreign device or validation failure leaves existing pipelines intact.
    pub async fn enable_msaa4(&mut self, device: &wgpu::Device) -> Result<bool, SceneShaderError> {
        if device != &self.device {
            return Err(SceneShaderError("MSAA enable device mismatch".into()));
        }
        if self.msaa_pipelines.is_some() {
            return Ok(false);
        }
        let device = &self.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("current scene shader for MSAA"),
            source: wgpu::ShaderSource::Wgsl(self.shader_source.as_str().into()),
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipelines = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            4,
            self.shadow.as_ref().map(|shadow| &shadow.layout),
            self.environment
                .as_ref()
                .map(|environment| &environment.layout),
            self.reverse_z,
            self.shader_source == DEFAULT_SCENE_SHADER,
        );
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        self.msaa_pipelines = Some(pipelines);
        Ok(true)
    }

    fn from_layouts(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: wgpu::BindGroupLayout,
        texture_layout: wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        reverse_z: bool,
    ) -> Self {
        let (solid_opaque_pipeline, world_pipeline, overlay_pipeline, transparent_pipeline) =
            Self::create_shader_pipelines(
                device,
                format,
                &transform_layout,
                &texture_layout,
                shader,
                1,
                None,
                None,
                reverse_z,
                true,
            );
        Self {
            identity: std::sync::Arc::new(()),
            lighting_revision: 0,
            device: device.clone(),
            compute_memory_budget: crate::ComputeMemoryBudget::for_device(device),
            reverse_z,
            solid_opaque_pipeline,
            world_pipeline,
            overlay_pipeline,
            transparent_pipeline,
            transform_layout,
            texture_layout,
            color_format: format,
            shader_source: String::new(),
            shader_revision: 0,
            shadow: None,
            environment: None,
            msaa_pipelines: None,
        }
    }

    /// Attach an opaque shadow map and atomically install textured GGX visibility shading.
    /// Map production must precede lighting in the same ordered stream.
    /// # Errors
    /// Rejects foreign resources, invalid settings, or shader/pipeline validation.
    pub async fn enable_shadowed_point_light(
        &mut self,
        device: &wgpu::Device,
        map: &crate::ShadowMap,
        settings: crate::ShadowSettings,
    ) -> Result<(), SceneShaderError> {
        self.resource_device(device)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        if self.lighting_revision == u64::MAX {
            return Err(SceneShaderError("lighting revision exhausted".into()));
        }
        let device = &self.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = crate::shadow_visibility::ShadowBindings::new(device, map, settings);
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let candidate = candidate.map_err(|e| SceneShaderError(e.to_string()))?;
        let source =
            crate::environment_lighting::shader_with_environment(self.environment.as_ref());
        let pipelines = self
            .prepare_scene_pipelines(
                &source,
                Some(&candidate.layout),
                self.environment.as_ref().map(|e| &e.layout),
            )
            .await?;
        self.shadow = Some(candidate);
        self.install_scene_pipelines(source, pipelines);
        self.lighting_revision += 1;
        Ok(())
    }

    /// Attach initialized GGX environment and DFG resources for single-scattering specular IBL.
    /// Production must precede scene rendering. Diffuse IBL and multiscattering are separate.
    /// # Errors
    /// Rejects foreign resources or shader/pipeline validation; preserves the previous state.
    pub async fn enable_environment_lighting(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
    ) -> Result<(), SceneShaderError> {
        let source = crate::environment_lighting::shader_for_dfg(self.shadow.is_some(), false, dfg);
        self.enable_environment_source(device, environment, dfg, None, &source)
            .await
    }

    /// Attach specular and normalized diffuse environment cubes with DFG integration.
    /// # Errors
    /// Rejects foreign resources and invalid shaders, preserving the previous configuration.
    pub async fn enable_full_environment_lighting(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: &crate::DiffuseEnvironmentConvolution,
    ) -> Result<(), SceneShaderError> {
        let source = crate::environment_lighting::shader_for_dfg(self.shadow.is_some(), true, dfg);
        self.enable_environment_source(device, environment, dfg, Some(diffuse), &source)
            .await
    }

    pub(crate) async fn enable_environment_source(
        &mut self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: Option<&crate::DiffuseEnvironmentConvolution>,
        source: &str,
    ) -> Result<(), SceneShaderError> {
        let prepared = self
            .prepare_environment_source(device, environment, dfg, diffuse, source)
            .await?;
        self.commit_prepared_environment(prepared)
    }

    /// Prepare off to the side. Existing lighting, pipelines and intensity remain live.
    pub async fn prepare_environment_lighting(
        &self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: Option<&crate::DiffuseEnvironmentConvolution>,
    ) -> Result<PreparedSceneEnvironment, SceneShaderError> {
        let source = crate::environment_lighting::shader_for_dfg(
            self.shadow.is_some(),
            diffuse.is_some(),
            dfg,
        );
        self.prepare_environment_source(device, environment, dfg, diffuse, &source)
            .await
    }

    pub(crate) async fn prepare_environment_source(
        &self,
        device: &wgpu::Device,
        environment: &crate::GgxEnvironmentPrefilter,
        dfg: &crate::GgxDfgLut,
        diffuse: Option<&crate::DiffuseEnvironmentConvolution>,
        source: &str,
    ) -> Result<PreparedSceneEnvironment, SceneShaderError> {
        self.resource_device(device)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        let device = &self.device;
        let intensity = self.environment.as_ref().map_or(1.0, |e| e.intensity);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = crate::environment_lighting::EnvironmentBindings::new(
            device,
            environment,
            dfg,
            diffuse,
            intensity,
            self.environment.as_ref().map(|e| &e.intensity_buffer),
        );
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let candidate = candidate.map_err(|e| SceneShaderError(e.to_string()))?;
        let pipelines = self
            .prepare_scene_pipelines(
                source,
                self.shadow.as_ref().map(|s| &s.layout),
                Some(&candidate.layout),
            )
            .await?;
        Ok(PreparedSceneEnvironment {
            owner: self.identity.clone(),
            base_shader_revision: self.shader_revision,
            base_shader_source: self.shader_source.clone(),
            base_lighting_revision: self.lighting_revision,
            msaa_enabled: self.msaa_pipelines.is_some(),
            environment: candidate,
            pipelines,
            source: source.to_owned(),
            dfg: dfg.output().clone(),
        })
    }

    fn validate_environment_candidate(
        &self,
        prepared: &PreparedSceneEnvironment,
    ) -> Result<(), SceneShaderError> {
        if !std::sync::Arc::ptr_eq(&self.identity, &prepared.owner) {
            return Err(SceneShaderError(
                "prepared environment belongs to another scene".into(),
            ));
        }
        if self.shader_revision != prepared.base_shader_revision
            || self.shader_source != prepared.base_shader_source
            || self.lighting_revision != prepared.base_lighting_revision
            || self.msaa_pipelines.is_some() != prepared.msaa_enabled
        {
            return Err(SceneShaderError("stale prepared scene environment".into()));
        }
        if self.lighting_revision == u64::MAX {
            return Err(SceneShaderError("lighting revision exhausted".into()));
        }
        Ok(())
    }

    fn install_environment_candidate(&mut self, mut prepared: PreparedSceneEnvironment) {
        if let Some(current) = &self.environment {
            prepared.environment.intensity = current.intensity;
        }
        self.install_scene_pipelines(prepared.source, prepared.pipelines);
        self.environment = Some(prepared.environment);
        self.lighting_revision += 1;
    }

    /// Validate completely before changing any scene state. No await occurs in commit.
    pub fn commit_prepared_environment(
        &mut self,
        prepared: PreparedSceneEnvironment,
    ) -> Result<(), SceneShaderError> {
        self.validate_environment_candidate(&prepared)?;
        self.install_environment_candidate(prepared);
        Ok(())
    }

    /// Commit one upload-complete atlas and every supplied scene together. All
    /// scene revisions, exact DFG handles and the requested generation are checked
    /// before replacing either the owner or any binding/pipeline set.
    pub fn commit_environment_atlas_batch(
        targets: Vec<(&mut SceneRenderer, PreparedSceneEnvironment)>,
        slot: &mut crate::GgxEnergyAtlasSlot,
        ready: crate::ReadyGgxEnergyAtlas,
    ) -> Result<crate::GgxDfgLut, SceneShaderError> {
        if targets.is_empty() {
            return Err(SceneShaderError("empty atlas scene transaction".into()));
        }
        for (scene, prepared) in &targets {
            scene.validate_environment_candidate(prepared)?;
            if &prepared.dfg != ready.atlas().output() {
                return Err(SceneShaderError(
                    "prepared environment uses a different DFG resource".into(),
                ));
            }
        }
        let previous = slot
            .try_install(ready)
            .map_err(|e| SceneShaderError(e.to_string()))?;
        for (scene, prepared) in targets {
            scene.install_environment_candidate(prepared);
        }
        Ok(previous)
    }

    #[must_use]
    pub fn lighting_revision(&self) -> u64 {
        self.lighting_revision
    }

    /// Scale diffuse and specular IBL without recomputing environment textures.
    /// Queue must belong to this renderer's device (wgpu validation).
    /// # Errors
    /// Rejects negative/nonfinite intensity or missing environment bindings.
    pub fn set_environment_intensity(
        &mut self,
        queue: &wgpu::Queue,
        intensity: f32,
    ) -> Result<(), SceneShaderError> {
        if !intensity.is_finite() || intensity < 0.0 {
            return Err(SceneShaderError(
                "environment intensity must be finite and nonnegative".into(),
            ));
        }
        let environment = self
            .environment
            .as_mut()
            .ok_or_else(|| SceneShaderError("environment lighting is not attached".into()))?;
        queue.write_buffer(
            &environment.intensity_buffer,
            0,
            bytemuck::cast_slice(&[intensity, 0.0, 0.0, 0.0]),
        );
        environment.intensity = intensity;
        Ok(())
    }

    /// Update shadow settings on the renderer's queue before encoding new consumers.
    /// # Errors
    /// Rejects absent shadow resources or invalid settings before writing GPU memory.
    pub fn update_shadow_settings(
        &self,
        queue: &wgpu::Queue,
        settings: crate::ShadowSettings,
    ) -> Result<(), SceneError> {
        self.shadow
            .as_ref()
            .ok_or(SceneError::InvalidGeometry)?
            .update(queue, settings)
    }

    /// Replaces both pipelines atomically after compilation and ABI validation.
    /// Existing textures and transforms remain compatible. An identical source is
    /// a cache hit and does not compile again. Call from the device owner task.
    ///
    /// # Errors
    /// Returns shader diagnostics for invalid WGSL or a mismatched pipeline ABI;
    /// the previously working shader and revision remain active.
    pub async fn reload_shader(
        &mut self,
        device: &wgpu::Device,
        source: &str,
    ) -> Result<bool, SceneShaderError> {
        if device != &self.device {
            return Err(SceneShaderError("shader reload device mismatch".into()));
        }
        let pipelines = self
            .prepare_scene_pipelines(
                source,
                self.shadow.as_ref().map(|s| &s.layout),
                self.environment.as_ref().map(|e| &e.layout),
            )
            .await?;
        let Some(pipelines) = pipelines else {
            return Ok(false);
        };
        Ok(self.install_scene_pipelines(source.to_owned(), Some(pipelines)))
    }

    async fn prepare_scene_pipelines(
        &self,
        source: &str,
        shadow_layout: Option<&wgpu::BindGroupLayout>,
        environment_layout: Option<&wgpu::BindGroupLayout>,
    ) -> Result<Option<PreparedScenePipelines>, SceneShaderError> {
        if source == self.shader_source {
            return Ok(None);
        }
        let device = &self.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prepared scene shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let is_default = source == DEFAULT_SCENE_SHADER;
        let (solid_opaque, world, overlay, transparent) = Self::create_shader_pipelines(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            1,
            shadow_layout,
            environment_layout,
            self.reverse_z,
            is_default,
        );
        let msaa = self.msaa_pipelines.as_ref().map(|_| {
            Self::create_shader_pipelines(
                device,
                self.color_format,
                &self.transform_layout,
                &self.texture_layout,
                &shader,
                4,
                shadow_layout,
                environment_layout,
                self.reverse_z,
                is_default,
            )
        });
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        Ok(Some(PreparedScenePipelines {
            solid_opaque,
            world,
            overlay,
            transparent,
            msaa,
        }))
    }

    fn install_scene_pipelines(
        &mut self,
        source: String,
        pipelines: Option<PreparedScenePipelines>,
    ) -> bool {
        let Some(p) = pipelines else {
            return false;
        };
        self.solid_opaque_pipeline = p.solid_opaque;
        self.world_pipeline = p.world;
        self.overlay_pipeline = p.overlay;
        self.transparent_pipeline = p.transparent;
        self.msaa_pipelines = p.msaa;
        self.shader_source = source;
        self.shader_revision = self.shader_revision.saturating_add(1);
        true
    }

    #[must_use]
    pub const fn shader_revision(&self) -> u64 {
        self.shader_revision
    }

    #[allow(clippy::too_many_arguments)]
    fn create_shader_pipelines(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
        shadow_layout: Option<&wgpu::BindGroupLayout>,
        environment_layout: Option<&wgpu::BindGroupLayout>,
        reverse_z: bool,
        has_solid_entry: bool,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
        Self::create_shader_pipelines_with_tangents(
            device,
            format,
            transform_layout,
            texture_layout,
            shader,
            sample_count,
            shadow_layout,
            environment_layout,
            false,
            reverse_z,
            has_solid_entry,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create_shader_pipelines_with_tangents(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
        shadow_layout: Option<&wgpu::BindGroupLayout>,
        environment_layout: Option<&wgpu::BindGroupLayout>,
        has_tangents: bool,
        reverse_z: bool,
        has_solid_entry: bool,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
        let mut layouts = vec![Some(transform_layout), Some(texture_layout)];
        if shadow_layout.is_some() || environment_layout.is_some() {
            layouts.push(shadow_layout);
        }
        if let Some(environment) = environment_layout {
            layouts.push(Some(environment));
        }
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene pipeline layout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
        let normal_attributes = wgpu::vertex_attr_array![3 => Float32x3];
        let coordinate_attributes = wgpu::vertex_attr_array![5 => Float32x3];
        let material_attributes = wgpu::vertex_attr_array![4 => Float32x4];
        let tangent_attributes = wgpu::vertex_attr_array![6 => Float32x4];
        let mut vertex_buffers = vec![
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SceneVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &normal_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &material_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &coordinate_attributes,
            }),
        ];
        if has_tangents {
            vertex_buffers.push(Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &tangent_attributes,
            }));
        }
        let world_compare = if reverse_z {
            wgpu::CompareFunction::GreaterEqual
        } else {
            wgpu::CompareFunction::LessEqual
        };
        let pipeline = |overlay: bool, depth_write: bool, solid: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("scene pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &vertex_buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: if solid { Some(wgpu::Face::Back) } else { None },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(if overlay {
                        wgpu::CompareFunction::Always
                    } else {
                        world_compare
                    }),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(if solid && has_solid_entry {
                        "fs_solid_opaque"
                    } else {
                        "fs_main"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: if solid {
                            None
                        } else {
                            Some(wgpu::BlendState::ALPHA_BLENDING)
                        },
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        (
            pipeline(false, true, true),
            pipeline(false, true, false),
            pipeline(true, false, false),
            pipeline(false, false, false),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create_single_opaque_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
        shadow_layout: Option<&wgpu::BindGroupLayout>,
        environment_layout: Option<&wgpu::BindGroupLayout>,
        has_tangents: bool,
        reverse_z: bool,
    ) -> wgpu::RenderPipeline {
        let mut layouts = vec![Some(transform_layout), Some(texture_layout)];
        if shadow_layout.is_some() || environment_layout.is_some() {
            layouts.push(shadow_layout);
        }
        if let Some(environment) = environment_layout {
            layouts.push(Some(environment));
        }
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene custom opaque layout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
        let normal_attributes = wgpu::vertex_attr_array![3 => Float32x3];
        let coordinate_attributes = wgpu::vertex_attr_array![5 => Float32x3];
        let material_attributes = wgpu::vertex_attr_array![4 => Float32x4];
        let tangent_attributes = wgpu::vertex_attr_array![6 => Float32x4];
        let mut vertex_buffers = vec![
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SceneVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &normal_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &material_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &coordinate_attributes,
            }),
        ];
        if has_tangents {
            vertex_buffers.push(Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &tangent_attributes,
            }));
        }
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("geometry opaque pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &vertex_buffers,
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(if reverse_z {
                    wgpu::CompareFunction::GreaterEqual
                } else {
                    wgpu::CompareFunction::LessEqual
                }),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Logical buffer bytes required by a validated mesh upload. Includes the
    /// normal/material-coordinate buffers and fixed material parameters.
    #[must_use]
    pub fn mesh_allocation_bytes(mesh: &SceneMesh) -> u64 {
        mesh.vertices().len() as u64 * (std::mem::size_of::<SceneVertex>() as u64 + 24)
            + mesh.indices().len() as u64 * 4
            + 16
    }

    /// # Errors
    /// Rejects a foreign device or invalid geometry, including primitive helper input.
    pub fn upload_shared_mesh_partitions(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices(), mesh.indices()],
            wgpu::BufferUsages::empty(),
        )
    }

    /// Shared draw streams that may be updated by admitted compute transfers.
    pub fn upload_compute_shared_mesh_partitions(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices(), mesh.indices()],
            wgpu::BufferUsages::STORAGE,
        )
    }

    pub async fn set_geometry_opaque_shader(
        &self,
        device: &wgpu::Device,
        geometry: &mut SceneGeometry,
        source: &str,
    ) -> Result<(), SceneShaderError> {
        if device != &self.device || device != &geometry.device {
            return Err(SceneShaderError("geometry material device mismatch".into()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("geometry opaque shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let one = Self::create_single_opaque_pipeline(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            1,
            self.shadow.as_ref().map(|s| &s.layout),
            self.environment.as_ref().map(|e| &e.layout),
            geometry.tangents.is_some(),
            self.reverse_z,
        );
        let four = Self::create_single_opaque_pipeline(
            device,
            self.color_format,
            &self.transform_layout,
            &self.texture_layout,
            &shader,
            4,
            self.shadow.as_ref().map(|s| &s.layout),
            self.environment.as_ref().map(|e| &e.layout),
            geometry.tangents.is_some(),
            self.reverse_z,
        );
        if let Some(error) = scope.pop().await {
            return Err(SceneShaderError(error.to_string()));
        }
        geometry.opaque_shader = Some((
            self.shader_revision,
            self.transform_layout.clone(),
            one,
            four,
        ));
        Ok(())
    }

    pub fn upload_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_mesh_with_usage(device, mesh, wgpu::BufferUsages::empty())
    }

    /// Fixed topology with vertex and normal storage writable by admitted compute work.
    pub fn upload_compute_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_mesh_with_usage(
            device,
            mesh,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        )
    }

    /// Upload optional, independent xyz/handedness tangents for a custom material.
    /// Ordinary meshes allocate no tangent stream. Invalid tangents publish no geometry.
    pub fn upload_tangent_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        tangents: &[[f32; 4]],
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_tangent_mesh_with_usage(device, mesh, tangents, wgpu::BufferUsages::empty())
    }

    /// Tangent/position/normal streams writable by ordered compute work.
    pub fn upload_compute_tangent_mesh(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        tangents: &[[f32; 4]],
    ) -> Result<SceneGeometry, SceneError> {
        self.upload_tangent_mesh_with_usage(device, mesh, tangents, wgpu::BufferUsages::STORAGE)
    }

    fn upload_tangent_mesh_with_usage(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        tangents: &[[f32; 4]],
        usage: wgpu::BufferUsages,
    ) -> Result<SceneGeometry, SceneError> {
        use crate::ManagedBufferDescriptor;
        if tangents.len() != mesh.vertices().len()
            || tangents.iter().any(|t| {
                t.iter().any(|v| !v.is_finite())
                    || (glam::Vec3::from_slice(t).length_squared() - 1.).abs() > 1e-4
                    || t[3].abs() != 1.
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let mut geometry = self.upload_mesh_with_usage(device, mesh, usage)?;
        let bytes = bytemuck::cast_slice(tangents);
        let mut buffers = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&[ManagedBufferDescriptor {
                label: "scene vertex tangents",
                size: bytes.len() as u64,
                contents: Some(bytes),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC
                    | usage,
            }])
            .map_err(scene_memory_error)?;
        geometry.tangents = Some(std::sync::Arc::new(
            buffers.pop().ok_or(SceneError::InvalidGeometry)?,
        ));
        Ok(geometry)
    }

    /// Keep a fixed mesh and sparse displacement binding resident on the GPU.
    /// Only control-node displacements are uploaded per pose; rendering consumes
    /// the resulting geometry without CPU vertex readback.
    pub fn prepare_surface_deformation(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        offsets: &[u32],
        weights: &[crate::SurfaceDeformationWeight],
        control_count: u32,
    ) -> Result<crate::SurfaceDeformation, SceneError> {
        crate::surface_deformation::validate(device, mesh, offsets, weights, control_count)?;
        let geometry = self.upload_mesh_with_usage(
            device,
            mesh,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        )?;
        crate::SurfaceDeformation::new(device, mesh, geometry, offsets, weights, control_count)
    }

    /// Attach fixed-topology deformation to the prefix of existing compute
    /// geometry. Shared GPU allocations retain appended facial/detail vertices.
    /// Admission failure leaves the caller's draw geometry untouched.
    pub fn prepare_surface_deformation_in_geometry(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        target: &SceneGeometry,
        offsets: &[u32],
        weights: &[crate::SurfaceDeformationWeight],
        controls: u32,
    ) -> Result<crate::SurfaceDeformation, SceneError> {
        crate::surface_deformation::validate(device, mesh, offsets, weights, controls)?;
        if *device != self.device
            || target.device != *device
            || mesh.vertices().len() > target.vertex_capacity
            || !target
                .vertices
                .usage()
                .contains(wgpu::BufferUsages::STORAGE)
            || !target.normals.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let view = SceneGeometry {
            identity: next_geometry_identity(),
            revision: 0,
            device: target.device.clone(),
            vertices: target.vertices.clone(),
            normals: target.normals.clone(),
            normal_cache: NormalCache::default(),
            tangents: target.tangents.clone(),
            material_coordinates: target.material_coordinates.clone(),
            coordinate_cache: Vec::new(),
            material_parameters: target.material_parameters.clone(),
            indices: target.indices.clone(),
            index_count: target.index_count,
            vertex_capacity: target.vertex_capacity,
            index_capacity: target.index_capacity,
            depth_mode: target.depth_mode,
            opaque_shader: target.opaque_shader.clone(),
            partitioned_indices: target.partitioned_indices,
            partition_cache: target.partition_cache.clone(),
        };
        crate::SurfaceDeformation::new(device, mesh, view, offsets, weights, controls)
    }

    fn upload_mesh_with_usage(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        additional_vertex_usage: wgpu::BufferUsages,
    ) -> Result<SceneGeometry, SceneError> {
        let mut geometry = self.upload_mesh_index_variants(
            device,
            mesh,
            &[mesh.indices()],
            additional_vertex_usage,
        )?;
        Ok(geometry.remove(0))
    }

    fn upload_mesh_index_variants(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &[&[u32]],
        additional_vertex_usage: wgpu::BufferUsages,
    ) -> Result<Vec<SceneGeometry>, SceneError> {
        Ok(self
            .upload_mesh_buffer_batch(device, mesh, variants, additional_vertex_usage, &[])?
            .geometries)
    }

    fn upload_mesh_buffer_batch(
        &self,
        device: &wgpu::Device,
        mesh: &SceneMesh,
        variants: &[&[u32]],
        additional_vertex_usage: wgpu::BufferUsages,
        additional_buffers: &[voxy_gpu_core::ManagedBufferDescriptor<'_>],
    ) -> Result<UploadedGeometryBatch, SceneError> {
        use voxy_gpu_core::ManagedBufferDescriptor;
        let device = self.resource_device(device)?;
        mesh.validate_for_upload()?;
        if variants.is_empty() {
            return Err(SceneError::InvalidGeometry);
        }
        for indices in variants {
            geometry_sizes(device, mesh.vertices.len(), indices.len())?;
            if indices.iter().any(|i| *i as usize >= mesh.vertices.len()) {
                return Err(SceneError::InvalidIndex);
            }
        }
        let normal_cache = NormalCache::from_mesh(mesh);
        let coordinates = mesh.material_coordinates();
        let vertex_usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let data = [
            bytemuck::cast_slice(&mesh.vertices),
            bytemuck::cast_slice(&normal_cache.normals),
            bytemuck::cast_slice(coordinates.as_ref()),
            bytemuck::cast_slice(&mesh.material_parameters),
        ];
        let labels = [
            "scene vertices",
            "scene smooth normals",
            "scene material coordinates",
            "scene material parameters",
        ];
        let usages = [
            vertex_usage | additional_vertex_usage,
            vertex_usage | additional_vertex_usage,
            vertex_usage,
            vertex_usage,
        ];
        let mut descriptors: Vec<_> = (0..4)
            .map(|i| ManagedBufferDescriptor {
                label: labels[i],
                size: data[i].len() as u64,
                contents: Some(data[i]),
                usage: usages[i],
            })
            .collect();
        descriptors.extend(variants.iter().map(|indices| ManagedBufferDescriptor {
            label: "scene indices",
            size: indices.len() as u64 * 4,
            contents: Some(bytemuck::cast_slice(indices)),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        }));
        descriptors.extend_from_slice(additional_buffers);
        let mut buffers = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .map_err(scene_memory_error)?
            .into_iter()
            .map(std::sync::Arc::new);
        let vertices = buffers.next().expect("validated vertex stream");
        let normals = buffers.next().expect("validated normal stream");
        let material_coordinates = buffers.next().expect("validated coordinate stream");
        let material_parameters = buffers.next().expect("validated parameter stream");
        let mut cache = Some((normal_cache, coordinates.into_owned()));
        let geometries = buffers
            .by_ref()
            .take(variants.len())
            .zip(variants)
            .map(|(indices, source)| {
                let (normal_cache, coordinate_cache) = cache.take().unwrap_or_default();
                SceneGeometry {
                    device: device.clone(),
                    vertices: vertices.clone(),
                    normals: normals.clone(),
                    normal_cache,
                    tangents: None,
                    material_coordinates: material_coordinates.clone(),
                    coordinate_cache,
                    material_parameters: material_parameters.clone(),
                    indices,
                    index_count: source.len() as u32,
                    depth_mode: SceneDepthMode::Opaque,
                    opaque_shader: None,
                    partitioned_indices: false,
                    partition_cache: Vec::new(),
                    vertex_capacity: mesh.vertices.len(),
                    index_capacity: source.len(),
                    identity: next_geometry_identity(),
                    revision: 0,
                }
            })
            .collect();
        Ok(UploadedGeometryBatch {
            geometries,
            additional_buffers: buffers.collect(),
        })
    }

    /// Replace published geometry only after validating and uploading its replacement.
    /// Invalid imports and foreign devices leave the existing GPU geometry intact.
    /// The target's depth mode is preserved across successful replacement.
    /// # Errors
    /// Preserves upload validation errors before changing the target geometry.
    pub fn replace_mesh(
        &self,
        device: &wgpu::Device,
        target: &mut SceneGeometry,
        mesh: &SceneMesh,
    ) -> Result<(), SceneError> {
        let mut replacement = self.upload_mesh(device, mesh)?;
        replacement.depth_mode = target.depth_mode;
        *target = replacement;
        Ok(())
    }

    /// Reserves empty GPU geometry for dynamic meshes/sprite batches.
    /// Call `SceneGeometry::update` before drawing; `clear` reuses the buffers.
    /// # Errors
    /// Rejects a foreign device, zero, overflow or excessive buffer capacities.
    pub fn reserve_geometry(
        &self,
        device: &wgpu::Device,
        vertex_capacity: usize,
        index_capacity: usize,
    ) -> Result<SceneGeometry, SceneError> {
        let device = self.resource_device(device)?;
        let (vertex_bytes, index_bytes) = geometry_sizes(device, vertex_capacity, index_capacity)?;
        use voxy_gpu_core::ManagedBufferDescriptor;
        let vertex_usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let parameters = [1.333_f32, 0.2, 0.08, 0.04];
        let sizes = [
            vertex_bytes,
            vertex_capacity as u64 * 12,
            vertex_capacity as u64 * 12,
            16,
            index_bytes,
        ];
        let labels = [
            "reserved scene vertices",
            "reserved scene normals",
            "reserved material coordinates",
            "reserved scene material parameters",
            "reserved scene indices",
        ];
        let descriptors = std::array::from_fn(|i| ManagedBufferDescriptor {
            label: labels[i],
            size: sizes[i],
            contents: (i == 3).then(|| bytemuck::cast_slice(&parameters)),
            usage: if i == 4 {
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            } else {
                vertex_usage
            },
        });
        let [
            vertices,
            normals,
            material_coordinates,
            material_parameters,
            indices,
        ] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers(descriptors)
            .map_err(scene_memory_error)?
            .map(std::sync::Arc::new);
        Ok(SceneGeometry {
            device: device.clone(),
            vertices,
            normals,
            material_coordinates,
            material_parameters,
            indices,
            normal_cache: NormalCache::default(),
            tangents: None,
            coordinate_cache: Vec::new(),
            index_count: 0,
            depth_mode: SceneDepthMode::Opaque,
            opaque_shader: None,
            partitioned_indices: false,
            partition_cache: Vec::new(),
            vertex_capacity,
            index_capacity,
            identity: next_geometry_identity(),
            revision: 0,
        })
    }
}

#[cfg(test)]
#[path = "scene/multi_view_tests.rs"]
mod multi_view_tests;

mod lod_geometry;
#[cfg(test)]
mod tests;
pub use lod_geometry::{SceneLodGeometry, SceneLodHistory};
mod skinning;
pub use skinning::{
    SceneSkinError, SceneSkinInstance, SceneSkinLodLevel, SceneSkinPose, SceneSkinSource,
    SceneSkinner,
};

#[cfg(test)]
mod memory_tests;

#[cfg(test)]
mod texture_memory_tests;

#[cfg(test)]
#[path = "scene/transform_memory_tests.rs"]
mod transform_memory_tests;

#[cfg(test)]
mod background_upload_regression;

#[cfg(test)]
mod opaque_partition_regression;

#[cfg(test)]
#[path = "scene/environment_transaction_tests.rs"]
mod environment_transaction_tests;

#[cfg(test)]
#[path = "scene/environment_pixel_tests.rs"]
mod environment_pixel_tests;

//! Opaque raster shadow depths, independent of ray-query availability.
use crate::{SceneError, SceneGeometry, SceneVertex};
use glam::Mat4;
use wgpu::util::DeviceExt;

/// Flag indicating which draw buffers have dirty geometry data.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShadowDrawMask {
    /// All mesh transforms have moved since last shadow pass.
    pub dirty: bool,
}

impl ShadowDrawMask {
    /// Create fresh mask with all geometry marked dirty (default for new shadow map).
    #[must_use]
    pub fn full() -> Self {
        Self { dirty: true }
    }

    /// Mark as clean — no geometry moved, shadows can be skipped.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Mark as dirty — at least one mesh moved, require refresh.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// What one shadow raster depended on: geometry identity and revision plus the
/// light clip-from-model transform bits.
type ShadowKey = (u64, u64, [u32; 16]);

#[derive(Debug)]
pub struct ShadowMap {
    device: wgpu::Device,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    /// Keys of the draws the retained depth texture currently holds.
    encoded: std::sync::Mutex<Vec<ShadowKey>>,
}
/// Immutable light-camera snapshot borrowing the current opaque geometry.
#[derive(Debug)]
pub struct ShadowDraw<'a> {
    device: wgpu::Device,
    geometry: &'a SceneGeometry,
    bindings: wgpu::BindGroup,
    key: ShadowKey,
}
impl ShadowMap {
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    /// Create single-sample conventional depth, cleared to 1 and using Less.
    /// The map is retained until explicitly replaced; no automatic resizing.
    /// # Errors
    /// Rejects zero dimensions or dimensions exceeding device texture limits.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, SceneError> {
        let limit = device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > limit || height > limit {
            return Err(SceneError::InvalidGeometry);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("opaque shadow depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow light transform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow depth layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow depth"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow_depth.wgsl").into()),
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("opaque shadow depth"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<SceneVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            texture,
            view,
            layout,
            pipeline,
            encoded: std::sync::Mutex::new(Vec::new()),
        })
    }
    /// Prepare an opaque draw with a finite light clip-from-model matrix.
    /// Geometry updates must follow submission of earlier draws on the same queue.
    /// # Errors
    /// Rejects a foreign device's geometry or a nonfinite transform before allocation.
    pub fn prepare<'a>(
        &self,
        geometry: &'a SceneGeometry,
        clip_from_model: Mat4,
    ) -> Result<ShadowDraw<'a>, SceneError> {
        if !geometry.belongs_to(&self.device) {
            return Err(SceneError::DeviceMismatch);
        }
        if !clip_from_model.is_finite() {
            return Err(SceneError::InvalidTransform);
        }
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow light snapshot"),
                contents: bytemuck::cast_slice(&clip_from_model.to_cols_array()),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow draw"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let (identity, revision) = geometry.shadow_key();
        Ok(ShadowDraw {
            device: self.device.clone(),
            geometry,
            bindings,
            key: (
                identity,
                revision,
                clip_from_model.to_cols_array().map(f32::to_bits),
            ),
        })
    }
    /// Clear and rasterize all supplied opaque triangles. Encode before map consumers.
    /// This stage ignores texture alpha/transmission and supplies no comparison bias.
    /// # Errors
    /// Rejects foreign-device draws before opening the render pass.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        draws: &[ShadowDraw<'_>],
    ) -> Result<(), SceneError> {
        if draws.iter().any(|draw| draw.device != self.device) {
            return Err(SceneError::DeviceMismatch);
        }
        // The depth texture is retained, so an identical draw list (same geometry
        // allocations, no vertex/index writes since, same light transforms) already
        // holds this raster and the pass is skipped.
        let keys: Vec<ShadowKey> = draws.iter().map(|draw| draw.key).collect();
        let mut encoded = self
            .encoded
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !keys.is_empty() && *encoded == keys {
            return Ok(());
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("opaque shadow raster"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        for draw in draws {
            pass.set_bind_group(0, &draw.bindings, &[]);
            draw.geometry.encode_shadow_geometry(&mut pass);
        }
        drop(pass);
        *encoded = keys;
        Ok(())
    }
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

/// Snaps a light-space orthographic cascade center `[cx, cy]` to exact shadow-map texel increments
/// so sub-texel camera panning does not cause shadow-edge shimmering.
///
/// Returns `(snapped_min_xy, snapped_max_xy)`. Returns `None` if `world_extent <= 0.0`,
/// `resolution == 0`, or inputs are non-finite.
#[must_use]
pub fn snap_cascade_ortho_bounds(
    light_space_center_xy: [f32; 2],
    world_extent: f32,
    resolution: u32,
) -> Option<([f32; 2], [f32; 2])> {
    if !light_space_center_xy[0].is_finite()
        || !light_space_center_xy[1].is_finite()
        || !world_extent.is_finite()
        || world_extent <= 0.0
        || resolution == 0
    {
        return None;
    }
    let texel_size = world_extent / (resolution as f32);
    if !texel_size.is_finite() || texel_size <= 0.0 {
        return None;
    }
    let snapped_cx = (light_space_center_xy[0] / texel_size).floor() * texel_size;
    let snapped_cy = (light_space_center_xy[1] / texel_size).floor() * texel_size;
    let half = 0.5 * world_extent;
    Some(([snapped_cx - half, snapped_cy - half], [snapped_cx + half, snapped_cy + half]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_cascade_ortho_bounds_stabilizes_sub_texel_panning() {
        // 100m cascade over 1000px => 0.1m texel size.
        let (min_a, max_a) = snap_cascade_ortho_bounds([12.31, -4.52], 100.0, 1000).unwrap();
        let (min_b, max_b) = snap_cascade_ortho_bounds([12.38, -4.59], 100.0, 1000).unwrap();
        assert!((min_a[0] - min_b[0]).abs() < 1e-5);
        assert!((min_a[1] - min_b[1]).abs() < 1e-5);
        assert!(((max_a[0] - min_a[0]) - 100.0).abs() < 1e-4);
        assert!(((max_b[1] - min_b[1]) - 100.0).abs() < 1e-4);
        assert!(snap_cascade_ortho_bounds([0.0, 0.0], 0.0, 1024).is_none());
    }
}

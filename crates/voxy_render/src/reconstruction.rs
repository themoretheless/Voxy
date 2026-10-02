//! Linear material guide render targets for temporal ray reconstruction.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconstructionGuideError {
    InvalidDimensions,
    UnsupportedFormats,
    AttachmentLimits,
}
impl std::fmt::Display for ReconstructionGuideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ray reconstruction guide error: {self:?}")
    }
}
impl std::error::Error for ReconstructionGuideError {}

/// Input-resolution guides; contents must come from the matching ray/material pass.
/// Normals are normalized world-space XYZ with linear roughness in alpha;
/// albedos are linear reflectance and hit distance is world-space specular ray length.
#[derive(Debug)]
pub struct RayReconstructionGuides {
    normal_roughness: wgpu::Texture,
    material_f0: wgpu::Texture,
    object_ids: wgpu::Texture,
    diffuse_albedo: wgpu::Texture,
    specular_albedo: wgpu::Texture,
    specular_hit_distance: wgpu::Texture,
}
impl RayReconstructionGuides {
    /// Allocate four renderable, sampleable/readable targets on the adapter's device.
    /// Supply the adapter from which `device` was requested. Does not initialize
    /// material data or synthesize reflection distances from primary depth.
    /// Prefers `R32Float` distances; uses `R16Float` when R32 is not renderable.
    /// Inspect the selected format and keep distances within its finite range
    /// (65504 for binary16); the R16 path also reduces distance precision.
    /// # Errors
    /// Rejects empty/oversized dimensions, unsupported usages and MRT limits.
    pub fn new(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> Result<Self, ReconstructionGuideError> {
        let limits = device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
        {
            return Err(ReconstructionGuideError::InvalidDimensions);
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        let distance_format = [wgpu::TextureFormat::R32Float, wgpu::TextureFormat::R16Float]
            .into_iter()
            .find(|format| {
                adapter
                    .get_texture_format_features(*format)
                    .allowed_usages
                    .contains(usage)
            })
            .ok_or(ReconstructionGuideError::UnsupportedFormats)?;
        let bytes_per_sample = if distance_format == wgpu::TextureFormat::R32Float {
            28
        } else {
            26
        };
        if limits.max_color_attachments < 4
            || limits.max_color_attachment_bytes_per_sample < bytes_per_sample
        {
            return Err(ReconstructionGuideError::AttachmentLimits);
        }
        for format in [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::R32Uint,
        ] {
            if !adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(usage)
            {
                return Err(ReconstructionGuideError::UnsupportedFormats);
            }
        }
        let create = |label, format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        Ok(Self {
            object_ids: create("primary object IDs", wgpu::TextureFormat::R32Uint),
            material_f0: create("primary material F0", wgpu::TextureFormat::Rgba16Float),
            normal_roughness: create(
                "RR world normal/roughness",
                wgpu::TextureFormat::Rgba16Float,
            ),
            diffuse_albedo: create("RR linear diffuse albedo", wgpu::TextureFormat::Rgba16Float),
            specular_albedo: create(
                "RR linear specular albedo",
                wgpu::TextureFormat::Rgba16Float,
            ),
            specular_hit_distance: create("RR specular hit distance", distance_format),
        })
    }
    /// Replace the complete set only after allocation validation succeeds.
    /// Old resource users must retain their own owners through GPU/SDK completion;
    /// invalidate temporal history and rebuild views/bindings after replacement.
    /// # Errors
    /// Preserves allocation validation errors without changing this set.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> Result<(), ReconstructionGuideError> {
        let replacement = Self::new(device, adapter, width, height)?;
        *self = replacement;
        Ok(())
    }
    #[must_use]
    pub fn size(&self) -> wgpu::Extent3d {
        self.normal_roughness.size()
    }
    #[must_use]
    pub fn normal_roughness(&self) -> &wgpu::Texture {
        &self.normal_roughness
    }
    /// Primary opaque object identity; background uses `u32::MAX`.
    #[must_use]
    pub fn object_ids(&self) -> &wgpu::Texture {
        &self.object_ids
    }
    /// Linear material F0 for tracing, separate from the view-dependent RR albedo.
    #[must_use]
    pub fn material_f0(&self) -> &wgpu::Texture {
        &self.material_f0
    }
    #[must_use]
    pub fn diffuse_albedo(&self) -> &wgpu::Texture {
        &self.diffuse_albedo
    }
    #[must_use]
    pub fn specular_albedo(&self) -> &wgpu::Texture {
        &self.specular_albedo
    }
    #[must_use]
    pub fn specular_hit_distance(&self) -> &wgpu::Texture {
        &self.specular_hit_distance
    }
}

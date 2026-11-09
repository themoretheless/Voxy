//! Declarative builders for `wgpu` bind group layouts, bind groups, and compute pipelines.
//!
//! Eliminates repetitive `wgpu::BindGroupLayoutEntry` and `wgpu::BindGroupEntry` boilerplate
//! across `voxy_gpu_core`, `voxy_gpu`, and `voxy_render`.

/// Declarative builder for a [`wgpu::BindGroupLayout`].
#[derive(Clone, Debug, Default)]
pub struct BindGroupLayoutBuilder<'a> {
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    #[must_use]
    pub fn new(label: Option<&'a str>) -> Self {
        Self {
            label,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(label: Option<&'a str>, capacity: usize) -> Self {
        Self {
            label,
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Appends a raw [`wgpu::BindGroupLayoutEntry`].
    #[must_use]
    pub fn entry(mut self, entry: wgpu::BindGroupLayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Appends a uniform buffer binding entry.
    #[must_use]
    pub fn uniform_buffer(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Appends a storage buffer binding entry (`read_only = true` or `false`).
    #[must_use]
    pub fn storage_buffer(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        read_only: bool,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Appends a 2D texture binding entry.
    #[must_use]
    pub fn texture_2d(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    /// Appends a sampler binding entry.
    #[must_use]
    pub fn sampler(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        sampler_type: wgpu::SamplerBindingType,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(sampler_type),
            count: None,
        });
        self
    }

    /// Returns the accumulated entries slice.
    #[must_use]
    pub fn entries(&self) -> &[wgpu::BindGroupLayoutEntry] {
        &self.entries
    }

    /// Creates the [`wgpu::BindGroupLayout`] on `device`.
    #[must_use]
    pub fn build(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &self.entries,
        })
    }
}

/// Declarative builder for a [`wgpu::BindGroup`].
#[derive(Debug)]
pub struct BindGroupBuilder<'a> {
    label: Option<&'a str>,
    layout: &'a wgpu::BindGroupLayout,
    entries: Vec<wgpu::BindGroupEntry<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
    #[must_use]
    pub fn new(label: Option<&'a str>, layout: &'a wgpu::BindGroupLayout) -> Self {
        Self {
            label,
            layout,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub fn buffer(mut self, binding: u32, buffer: &'a wgpu::Buffer) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        });
        self
    }

    #[must_use]
    pub fn texture_view(mut self, binding: u32, view: &'a wgpu::TextureView) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
        self
    }

    #[must_use]
    pub fn sampler(mut self, binding: u32, sampler: &'a wgpu::Sampler) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        self
    }

    #[must_use]
    pub fn build(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: self.layout,
            entries: &self.entries,
        })
    }
}

//! Explicit portable graphics selection and adapter capability discovery.

/// A strict selection never silently falls back to another API.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GraphicsBackend {
    #[default]
    Auto,
    DirectX12,
    Metal,
    Vulkan,
    OpenGl,
    WebGl,
    WebGpu,
}

impl GraphicsBackend {
    #[must_use]
    pub fn backends(self) -> wgpu::Backends {
        match self {
            Self::Auto => wgpu::Backends::PRIMARY | wgpu::Backends::GL,
            Self::DirectX12 => wgpu::Backends::DX12,
            Self::Metal => wgpu::Backends::METAL,
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::OpenGl | Self::WebGl => wgpu::Backends::GL,
            Self::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GraphicsOptions {
    pub backend: GraphicsBackend,
    pub power_preference: wgpu::PowerPreference,
    pub force_fallback_adapter: bool,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            backend: GraphicsBackend::Auto,
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }
    }
}

impl GraphicsOptions {
    #[must_use]
    pub fn create_instance_with_display(
        self,
        display: impl raw_window_handle::HasDisplayHandle + std::fmt::Debug + Send + Sync + 'static,
    ) -> wgpu::Instance {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.display = Some(Box::new(display));
        descriptor.backends = self.backend.backends();
        wgpu::Instance::new(descriptor)
    }

    #[must_use]
    pub fn create_instance(self) -> wgpu::Instance {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = self.backend.backends();
        wgpu::Instance::new(descriptor)
    }
}

/// Adapter support is distinct from features enabled on a device.
#[derive(Clone, Debug)]
pub struct GraphicsCapabilities {
    pub adapter: wgpu::AdapterInfo,
    /// Supported optional features, not features enabled on any created device.
    pub supported_features: wgpu::Features,
    pub downlevel: wgpu::DownlevelCapabilities,
    pub compute_shaders: bool,
    pub experimental_ray_query: bool,
    pub limits: wgpu::Limits,
}

impl GraphicsCapabilities {
    #[must_use]
    pub fn discover(adapter: &wgpu::Adapter) -> Self {
        Self {
            adapter: adapter.get_info(),
            supported_features: adapter.features(),
            downlevel: adapter.get_downlevel_capabilities(),
            compute_shaders: adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS),
            experimental_ray_query: adapter
                .features()
                .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY),
            limits: adapter.limits(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_selection_excludes_other_backends() {
        for backend in [
            GraphicsBackend::DirectX12,
            GraphicsBackend::Metal,
            GraphicsBackend::Vulkan,
            GraphicsBackend::OpenGl,
            GraphicsBackend::WebGl,
            GraphicsBackend::WebGpu,
        ] {
            assert_eq!(backend.backends().bits().count_ones(), 1);
        }
        assert!(
            !GraphicsBackend::Auto
                .backends()
                .contains(wgpu::Backends::NOOP)
        );
    }
}

use voxy_render::{GraphicsBackend, GraphicsOptions};

#[derive(Debug)]
pub struct NativeSelection {
    pub graphics: GraphicsOptions,
    pub require_nvidia: bool,
}
impl NativeSelection {
    pub fn create_instance(&self) -> wgpu::Instance {
        self.graphics.create_instance()
    }
    pub async fn adapter(&self, instance: &wgpu::Instance) -> Result<wgpu::Adapter, String> {
        if self.require_nvidia {
            return instance
                .enumerate_adapters(wgpu::Backends::all())
                .await
                .into_iter()
                .find(|adapter| {
                    let info = adapter.get_info();
                    nvidia_hardware(info.vendor, info.device_type)
                })
                .ok_or_else(|| "requested backend has no physical NVIDIA adapter".into());
        }
        instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|error| error.to_string())
    }
}
fn nvidia_hardware(vendor: u32, kind: wgpu::DeviceType) -> bool {
    vendor == 0x10de
        && matches!(
            kind,
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
        )
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<NativeSelection, String> {
    let mut args = args.into_iter();
    let backend = match args.next().as_deref() {
        None | Some("auto") => GraphicsBackend::Auto,
        Some("metal") => GraphicsBackend::Metal,
        Some("vulkan") => GraphicsBackend::Vulkan,
        Some("dx12") => GraphicsBackend::DirectX12,
        Some("gl") => GraphicsBackend::OpenGl,
        _ => return Err("expected native backend auto|metal|vulkan|dx12|gl".into()),
    };
    let require_nvidia = match args.next().as_deref() {
        None => false,
        Some("--require-nvidia") => true,
        _ => return Err("expected optional --require-nvidia after backend".into()),
    };
    if args.next().is_some() {
        return Err("unexpected native backend argument".into());
    }
    Ok(NativeSelection {
        graphics: GraphicsOptions {
            backend,
            ..GraphicsOptions::default()
        },
        require_nvidia,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_native_selection_and_invalid_arguments() {
        for (name, backend) in [
            ("metal", GraphicsBackend::Metal),
            ("vulkan", GraphicsBackend::Vulkan),
            ("dx12", GraphicsBackend::DirectX12),
            ("gl", GraphicsBackend::OpenGl),
        ] {
            let options = parse([name.to_owned()]).unwrap();
            assert_eq!(options.graphics.backend, backend);
            assert!(!options.graphics.force_fallback_adapter);
        }
        assert_eq!(parse([]).unwrap().graphics.backend, GraphicsBackend::Auto);
        assert!(
            parse(["vulkan".to_owned(), "--require-nvidia".to_owned()])
                .unwrap()
                .require_nvidia
        );
        assert!(!parse([]).unwrap().require_nvidia);
        assert!(nvidia_hardware(0x10de, wgpu::DeviceType::DiscreteGpu));
        assert!(nvidia_hardware(0x10de, wgpu::DeviceType::IntegratedGpu));
        assert!(!nvidia_hardware(0x10de, wgpu::DeviceType::Cpu));
        assert!(!nvidia_hardware(0x8086, wgpu::DeviceType::IntegratedGpu));
        assert!(parse(["cuda".to_owned()]).is_err());
        assert!(parse(["metal".to_owned(), "vulkan".to_owned()]).is_err());
    }
}

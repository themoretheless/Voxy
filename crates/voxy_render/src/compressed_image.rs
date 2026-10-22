//! Preserve original BC7 DDS blocks and complete mip chains for GPU upload.
use std::{ops::Range, sync::Arc};
#[derive(Clone, Debug)]
pub struct Bc7ImageAsset {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub mip_ranges: Vec<Range<usize>>,
    source: Arc<[u8]>,
}
impl Bc7ImageAsset {
    /// Parse a bounded 2D DDS/DX10 texture. No recompression or pixel conversion.
    pub fn decode_dds(
        source: Arc<[u8]>,
        max_bytes: usize,
        max_dimension: u32,
    ) -> Result<Self, &'static str> {
        if source.len() > max_bytes {
            return Err("DDS source budget exceeded");
        }
        if source.len() < 148 || &source[..4] != b"DDS " {
            return Err("invalid DDS header");
        }
        let word =
            |offset: usize| u32::from_le_bytes(source[offset..offset + 4].try_into().unwrap());
        if word(4) != 124 || word(76) != 32 || &source[84..88] != b"DX10" {
            return Err("DDS requires DX10 pixel header");
        }
        let height = word(12);
        let width = word(16);
        let mips = word(28).max(1);
        if width == 0
            || height == 0
            || width > max_dimension
            || height > max_dimension
            || mips > width.max(height).ilog2() + 1
        {
            return Err("invalid DDS dimensions or mip count");
        }
        if word(132) != 3 || word(136) & 4 != 0 || word(140) != 1 || word(24) > 1 {
            return Err("only single-layer 2D BC7 DDS supported");
        }
        let format = match word(128) {
            98 => wgpu::TextureFormat::Bc7RgbaUnorm,
            99 => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
            _ => return Err("DDS compression unsupported; texture was not substituted"),
        };
        let mut offset = 148usize;
        let mut mip_ranges = Vec::new();
        for mip in 0..mips {
            let w = (width >> mip).max(1);
            let h = (height >> mip).max(1);
            let bytes = u64::from(w.div_ceil(4))
                .checked_mul(u64::from(h.div_ceil(4)))
                .and_then(|n| n.checked_mul(16))
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("DDS mip size overflow")?;
            let end = offset.checked_add(bytes).ok_or("DDS mip offset overflow")?;
            if end > source.len() {
                return Err("truncated DDS mip data");
            }
            mip_ranges.push(offset..end);
            offset = end;
        }
        if offset != source.len() {
            return Err("unexpected DDS trailing data");
        }
        Ok(Self {
            width,
            height,
            format,
            mip_ranges,
            source,
        })
    }
    /// Upload original blocks through the shared device memory owner.
    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
        if !device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
            || !self.width.is_multiple_of(4)
            || !self.height.is_multiple_of(4)
            || self.width > device.limits().max_texture_dimension_2d
            || self.height > device.limits().max_texture_dimension_2d
        {
            return Err(crate::ComputeError::Validation(
                "BC7 texture dimensions/features unavailable".into(),
            ));
        }
        let texture = crate::ComputeMemoryBudget::for_device(device).allocate_texture(
            &wgpu::TextureDescriptor {
                label: Some("authored BC7 scene image"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: self.mip_ranges.len() as u32,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[
                    wgpu::TextureFormat::Bc7RgbaUnorm,
                    wgpu::TextureFormat::Bc7RgbaUnormSrgb,
                ],
            },
        )?;
        for (mip, range) in self.mip_ranges.iter().enumerate() {
            let width = (self.width >> mip).max(1).div_ceil(4) * 4;
            let height = (self.height >> mip).max(1).div_ceil(4) * 4;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.source[range.clone()],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width / 4 * 16),
                    rows_per_image: Some(height / 4),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        Ok(texture)
    }
    pub fn mip_bytes(&self, mip: usize) -> Option<&[u8]> {
        self.mip_ranges
            .get(mip)
            .map(|range| &self.source[range.clone()])
    }
    pub fn byte_len(&self) -> usize {
        self.source.len() - 148
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 164];
        b[..4].copy_from_slice(b"DDS ");
        b[84..88].copy_from_slice(b"DX10");
        for (offset, value) in [
            (4, 124u32),
            (76, 32),
            (12, 4),
            (16, 4),
            (28, 1),
            (128, 99),
            (132, 3),
            (140, 1),
        ] {
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        b[148..].fill(17);
        b
    }
    #[test]
    fn bc7_keeps_source_blocks() {
        let b = fixture();
        let image = Bc7ImageAsset::decode_dds(b.clone().into(), 1024, 8192).unwrap();
        assert_eq!(image.format, wgpu::TextureFormat::Bc7RgbaUnormSrgb);
        assert_eq!(image.mip_bytes(0).unwrap(), &b[148..]);
        assert!(image.mip_bytes(1).is_none());
    }
    #[test]
    fn bc7_rejects_truncation_and_wrong_formats() {
        let b = fixture();
        assert!(Bc7ImageAsset::decode_dds(b[..163].into(), 1024, 8192).is_err());
        for (offset, value) in [(128, 97u32), (132, 4), (140, 2), (28, 4), (16, 0)] {
            let mut b = b.clone();
            b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(Bc7ImageAsset::decode_dds(b.into(), 1024, 8192).is_err());
        }
    }
    #[test]
    #[ignore = "requires physical GPU with BC compression and a real DDS image"]
    fn bc7_real_gpu_upload_preserves_last_mip() {
        let source = std::fs::read(std::env::var_os("VOXY_BC7_IMAGE").unwrap()).unwrap();
        let image = Bc7ImageAsset::decode_dds(source.into(), 512 * 1024 * 1024, 16384).unwrap();
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        assert!(
            adapter
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
        );
        let descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let texture = image.upload(&device, &queue).unwrap();
        assert_eq!(texture.allocation_bytes(), image.byte_len() as u64);
        let last = image.mip_ranges.len() - 1;
        let expected = image.mip_bytes(last).unwrap();
        assert_eq!(expected.len(), 16);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BC7 authored mip readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: last as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        assert_eq!(&mapped[..16], expected);
        drop(mapped);
        readback.unmap();
        assert!(pollster::block_on(scope.pop()).is_none());
        eprintln!(
            "BC7 original GPU transfer passed adapter={} mip_levels={} payload_bytes={}",
            adapter.get_info().name,
            image.mip_ranges.len(),
            image.byte_len()
        );
    }
}

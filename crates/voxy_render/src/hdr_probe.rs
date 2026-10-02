//! One-shot asynchronous RGBA16/32Float pixel readback for render diagnostics.
use std::sync::{Arc, Mutex};
type ProbeResult = Result<[f32; 4], String>;
#[derive(Debug)]
pub struct HdrPixelProbe {
    buffer: wgpu::Buffer,
    encoded: bool,
    format: wgpu::TextureFormat,
    mapped: bool,
    result: Arc<Mutex<Option<ProbeResult>>>,
}
impl HdrPixelProbe {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("HDR diagnostic pixel"),
                size: 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            encoded: false,
            format: wgpu::TextureFormat::Rgba16Float,
            mapped: false,
            result: Arc::new(Mutex::new(None)),
        }
    }
    /// Copy one pixel once. Source must be RGBA16/32Float/COPY_SRC on the same device.
    /// # Errors
    /// Rejects invalid source format or coordinates before encoding.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        x: u32,
        y: u32,
    ) -> Result<(), crate::SceneError> {
        if self.encoded {
            return Ok(());
        }
        if !matches!(
            texture.format(),
            wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
        ) || texture.sample_count() != 1
            || x >= texture.width()
            || y >= texture.height()
            || !texture.usage().contains(wgpu::TextureUsages::COPY_SRC)
        {
            return Err(crate::SceneError::InvalidTexture);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.format = texture.format();
        self.encoded = true;
        Ok(())
    }
    /// Call after submitting the encoder containing the copy. Mapping is nonblocking.
    pub fn begin_read(&mut self) {
        if !self.encoded || self.mapped {
            return;
        }
        self.mapped = true;
        let buffer = self.buffer.clone();
        let result = Arc::clone(&self.result);
        let format = self.format;
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |status| {
                let values = status.map_err(|e| e.to_string()).and_then(|()| {
                    let bytes = buffer
                        .slice(..)
                        .get_mapped_range()
                        .map_err(|e| e.to_string())?;
                    Ok(std::array::from_fn(|c| {
                        if format == wgpu::TextureFormat::Rgba32Float {
                            f32::from_le_bytes(
                                bytes[c * 4..c * 4 + 4].try_into().expect("RGBA32 pixel"),
                            )
                        } else {
                            half::f16::from_bits(u16::from_le_bytes([
                                bytes[c * 2],
                                bytes[c * 2 + 1],
                            ]))
                            .to_f32()
                        }
                    }))
                });
                buffer.unmap();
                *result.lock().expect("HDR probe result lock") = Some(values);
            });
    }
    /// Retrieve the mapped result once; browser callbacks progress asynchronously.
    pub fn take_result(&self) -> Option<ProbeResult> {
        self.result.lock().expect("HDR probe result lock").take()
    }
}

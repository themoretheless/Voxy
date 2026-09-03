use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialLayer {
    pub rgba8_srgb: Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialPack {
    width: u32,
    height: u32,
    layers: Arc<[MaterialLayer]>,
}

impl MaterialPack {
    /// Validates a CPU-retained texture-array source.
    ///
    /// # Errors
    ///
    /// Rejects zero/oversized dimensions, empty/oversized arrays, and invalid RGBA byte counts.
    pub fn new(width: u32, height: u32, layers: Vec<MaterialLayer>) -> Result<Self, MaterialError> {
        if width == 0 || height == 0 || width > 2048 || height > 2048 {
            return Err(MaterialError::InvalidDimensions { width, height });
        }
        if layers.is_empty() || layers.len() > 256 {
            return Err(MaterialError::InvalidLayerCount(layers.len()));
        }
        let expected = usize::try_from(width)
            .ok()
            .and_then(|value| value.checked_mul(usize::try_from(height).ok()?))
            .and_then(|value| value.checked_mul(4))
            .ok_or(MaterialError::SizeOverflow)?;
        for (index, layer) in layers.iter().enumerate() {
            if layer.rgba8_srgb.len() != expected {
                return Err(MaterialError::InvalidLayerBytes {
                    layer: index,
                    expected,
                    actual: layer.rgba8_srgb.len(),
                });
            }
        }
        Ok(Self {
            width,
            height,
            layers: layers.into(),
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn layer_count(&self) -> u32 {
        u32::try_from(self.layers.len()).unwrap_or(256)
    }

    #[must_use]
    pub fn layers(&self) -> &[MaterialLayer] {
        &self.layers
    }
}

#[derive(Debug)]
pub struct MaterialSet {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub layer_count: u32,
}

impl MaterialSet {
    #[must_use]
    pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, pack: &MaterialPack) -> Self {
        let size = wgpu::Extent3d {
            width: pack.width,
            height: pack.height,
            depth_or_array_layers: pack.layer_count(),
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("voxy material texture array"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer_index, layer) in pack.layers.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: u32::try_from(layer_index).unwrap_or(0),
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &layer.rgba8_srgb,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pack.width * 4),
                    rows_per_image: Some(pack.height),
                },
                wgpu::Extent3d {
                    width: pack.width,
                    height: pack.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("voxy material array view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("voxy pixel material sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            layer_count: pack.layer_count(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialError {
    InvalidDimensions {
        width: u32,
        height: u32,
    },
    InvalidLayerCount(usize),
    InvalidLayerBytes {
        layer: usize,
        expected: usize,
        actual: usize,
    },
    SizeOverflow,
}

impl fmt::Display for MaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid material pack: {self:?}")
    }
}

impl std::error::Error for MaterialError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_pack_validates_exact_rgba_layers() {
        let layer = MaterialLayer {
            rgba8_srgb: Arc::from([255_u8; 64]),
        };
        let pack = MaterialPack::new(4, 4, vec![layer]).unwrap();
        assert_eq!(pack.layer_count(), 1);
        assert!(matches!(
            MaterialPack::new(
                4,
                4,
                vec![MaterialLayer {
                    rgba8_srgb: Arc::from([0_u8; 63])
                }]
            ),
            Err(MaterialError::InvalidLayerBytes { .. })
        ));
    }
}

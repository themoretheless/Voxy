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

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid material pack: {self:?}")]
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

/// Converts normal-map slope variance `(sigma_x^2, sigma_y^2)` into filtered perceptual
/// roughness `(roughness_x, roughness_y)` following LEADR / Toksvig microfacet filtering:
/// `alpha' = sqrt(alpha^2 + 2 * sigma^2)`.
#[must_use]
pub fn slope_variance_to_roughness(
    base_roughness: f32,
    slope_variance_x: f32,
    slope_variance_y: f32,
) -> [f32; 2] {
    let base_r = if base_roughness.is_finite() {
        base_roughness.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let alpha = base_r * base_r;
    let alpha_sq = alpha * alpha;
    let vx = if slope_variance_x.is_finite() {
        slope_variance_x.max(0.0)
    } else {
        0.0
    };
    let vy = if slope_variance_y.is_finite() {
        slope_variance_y.max(0.0)
    } else {
        0.0
    };
    let alpha_x = (alpha_sq + 2.0 * vx).sqrt().clamp(0.0, 1.0);
    let alpha_y = (alpha_sq + 2.0 * vy).sqrt().clamp(0.0, 1.0);
    [alpha_x.sqrt(), alpha_y.sqrt()]
}

/// Evaluates exact unpolarized conductor Fresnel reflectance `(R_s + R_p) / 2` per RGB channel
/// for complex index of refraction `eta + i * k`.
#[must_use]
pub fn conductor_fresnel_complex_rgb(
    cos_theta: f32,
    eta_rgb: [f32; 3],
    k_rgb: [f32; 3],
) -> [f32; 3] {
    if !cos_theta.is_finite() {
        return [0.0; 3];
    }
    let c = cos_theta.clamp(0.0, 1.0);
    let c2 = c * c;
    let sin2 = (1.0 - c2).max(0.0);
    std::array::from_fn(|i| {
        let eta = eta_rgb[i];
        let k = k_rgb[i];
        if !eta.is_finite() || !k.is_finite() || eta <= 0.0 || k < 0.0 {
            return 0.0;
        }
        let eta2 = eta * eta;
        let k2 = k * k;
        let t0 = eta2 - k2 - sin2;
        let a2_plus_b2 = (t0 * t0 + 4.0 * eta2 * k2).sqrt();
        let a = (0.5 * (a2_plus_b2 + t0)).max(0.0).sqrt();
        let two_a_c = 2.0 * a * c;
        let rs = ((a2_plus_b2 + c2 - two_a_c) / (a2_plus_b2 + c2 + two_a_c).max(1e-8)).clamp(0.0, 1.0);
        let two_a_c_sin2 = two_a_c * sin2;
        let sin4 = sin2 * sin2;
        let rp = (rs * ((c2 * a2_plus_b2 + sin4 - two_a_c_sin2) / (c2 * a2_plus_b2 + sin4 + two_a_c_sin2).max(1e-8))).clamp(0.0, 1.0);
        0.5 * (rs + rp)
    })
}

/// Computes clearcoat double-pass Beer-Lambert spectral attenuation and interface transmission:
/// `(1 - F_coat(cos_theta_i))^2 * exp(-2 * sigma_a * thickness_mm / cos_theta_t)`.
#[must_use]
pub fn clearcoat_layer_attenuation(
    cos_theta_i: f32,
    coat_ior: f32,
    thickness_mm: f32,
    absorption_rgb: [f32; 3],
) -> [f32; 3] {
    if !cos_theta_i.is_finite() || !coat_ior.is_finite() || !thickness_mm.is_finite() || coat_ior < 1.0 {
        return [0.0; 3];
    }
    let cos_i = cos_theta_i.clamp(0.0, 1.0);
    let f0 = ((coat_ior - 1.0) / (coat_ior + 1.0)).powi(2);
    let f_coat = f0 + (1.0 - f0) * (1.0 - cos_i).powi(5);
    let interface_tx = (1.0 - f_coat).clamp(0.0, 1.0).powi(2);
    let sin2_i = (1.0 - cos_i * cos_i).max(0.0);
    let sin2_t = sin2_i / (coat_ior * coat_ior);
    let cos_t = (1.0 - sin2_t).max(1e-4).sqrt();
    let d = thickness_mm.max(0.0);
    let optical_path = 2.0 * d / cos_t;
    std::array::from_fn(|c| {
        let sigma = if absorption_rgb[c].is_finite() {
            absorption_rgb[c].max(0.0)
        } else {
            0.0
        };
        (interface_tx * (-sigma * optical_path).exp()).clamp(0.0, 1.0)
    })
}

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

    #[test]
    fn conductor_fresnel_clearcoat_and_slope_variance_behave_physically() {
        let gold_normal = conductor_fresnel_complex_rgb(1.0, [0.18, 0.42, 1.37], [3.42, 2.35, 1.77]);
        let gold_grazing = conductor_fresnel_complex_rgb(0.0, [0.18, 0.42, 1.37], [3.42, 2.35, 1.77]);
        assert!(gold_normal[0] > gold_normal[2]);
        assert!((gold_grazing[0] - 1.0).abs() < 1e-5);

        let clear = clearcoat_layer_attenuation(1.0, 1.5, 0.0, [0.5, 0.5, 0.5]);
        let tinted = clearcoat_layer_attenuation(0.7, 1.5, 1.2, [0.1, 0.5, 1.0]);
        assert!((clear[0] - 0.96_f32.powi(2)).abs() < 1e-4);
        assert!(tinted[0] > tinted[1] && tinted[1] > tinted[2]);

        let r = slope_variance_to_roughness(0.2, 0.0, 0.08);
        assert!((r[0] - 0.2).abs() < 1e-5);
        assert!(r[1] > r[0] && r[1] <= 1.0);
    }
}

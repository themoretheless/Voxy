//! Prepared GPU environment derived from a bounded linear HDR panorama.
use crate::{
    DiffuseEnvironmentConvolution, GgxDfgLut, GgxEnvironmentPrefilter, HdrImageAsset,
    HdrImageError, ImageLimits, RendererError, SceneRenderer, SceneShaderError,
};
#[derive(Debug)]
pub enum ImportedEnvironmentError {
    Image(HdrImageError),
    Renderer(RendererError),
    /// RGBA16Float cannot store radiance above 65504 without infinity.
    HalfFloatRange,
}
impl std::fmt::Display for ImportedEnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "environment import error: {self:?}")
    }
}
impl std::error::Error for ImportedEnvironmentError {}
impl From<HdrImageError> for ImportedEnvironmentError {
    fn from(e: HdrImageError) -> Self {
        Self::Image(e)
    }
}
impl From<RendererError> for ImportedEnvironmentError {
    fn from(e: RendererError) -> Self {
        Self::Renderer(e)
    }
}
#[derive(Debug)]
pub struct ImportedEnvironment {
    device: wgpu::Device,
    specular: GgxEnvironmentPrefilter,
    diffuse: DiffuseEnvironmentConvolution,
    dfg: GgxDfgLut,
}
impl ImportedEnvironment {
    /// Resample and upload a panorama. Encode the retained producers before consuming outputs.
    /// CPU limits include source RGB32F, resampled faces and packed RGBA16Float faces.
    /// Queue must belong to device (wgpu validation). Radiance is never tone mapped.
    /// # Errors
    /// Rejects CPU limits, nonrepresentable half radiance and device dimensions.
    pub fn from_panorama(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        panorama: &HdrImageAsset,
        size: u32,
        limits: ImageLimits,
    ) -> Result<Self, ImportedEnvironmentError> {
        let packed_bytes = u64::from(size)
            .checked_mul(u64::from(size))
            .and_then(|n| n.checked_mul(6 * 8))
            .ok_or(HdrImageError::LimitExceeded)?;
        let face_limits = ImageLimits {
            pixel_bytes: limits
                .pixel_bytes
                .checked_sub(packed_bytes)
                .ok_or(HdrImageError::LimitExceeded)?,
            ..limits
        };
        let faces = panorama.cube_faces(size, face_limits)?;
        if faces.iter().flatten().any(|v| *v > 65504.) {
            return Err(ImportedEnvironmentError::HalfFloatRange);
        }
        let packed: [Vec<u8>; 6] = faces.map(|face| {
            face.chunks_exact(3)
                .flat_map(|rgb| {
                    [rgb[0], rgb[1], rgb[2], 1.]
                        .into_iter()
                        .flat_map(|v| half::f16::from_f32(v).to_bits().to_le_bytes())
                })
                .collect()
        });
        let specular = GgxEnvironmentPrefilter::new(device, size)?;
        let diffuse = DiffuseEnvironmentConvolution::new(device, size)?;
        let dfg = GgxDfgLut::new(device, 64)?;
        for destination in [specular.input(), diffuse.input()] {
            for (face, pixels) in packed.iter().enumerate() {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: destination,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: u32::try_from(face).expect("six faces"),
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size * 8),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        Ok(Self {
            device: device.clone(),
            specular,
            diffuse,
            dfg,
        })
    }
    /// Generate specular/diffuse/DFG in order; no allocation or submission.
    /// The encoder must belong to this environment's device.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.specular.encode(encoder);
        self.diffuse.encode(encoder);
        self.dfg.encode(encoder);
    }
    /// Attach the initialized outputs to a scene renderer, preserving its shadow path.
    /// # Errors
    /// Rejects a foreign renderer or shader validation; previous configuration is retained.
    pub async fn attach(&self, renderer: &mut SceneRenderer) -> Result<(), SceneShaderError> {
        renderer
            .enable_full_environment_lighting(
                &self.device,
                &self.specular,
                &self.dfg,
                &self.diffuse,
            )
            .await
    }
    /// Transfer the retained producer resources to an owning application scene.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        GgxEnvironmentPrefilter,
        GgxDfgLut,
        DiffuseEnvironmentConvolution,
    ) {
        (self.specular, self.dfg, self.diffuse)
    }
    #[must_use]
    pub fn specular(&self) -> &GgxEnvironmentPrefilter {
        &self.specular
    }
    #[must_use]
    pub fn diffuse(&self) -> &DiffuseEnvironmentConvolution {
        &self.diffuse
    }
    #[must_use]
    pub fn dfg(&self) -> &GgxDfgLut {
        &self.dfg
    }
}

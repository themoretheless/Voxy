//! glTF PBR base-color texture and embedded GLB image decoding.

use super::{ModelAsset, ModelError, ModelTexture, fail};

impl ModelAsset {
    /// Decode all embedded GLB images in document order with a shared pixel budget.
    /// External image buffers and image URIs are rejected; callers retain ownership
    /// of decoded images separately from skeleton/mesh data and GPU resources.
    /// Mesh and skeleton validation remains the responsibility of `parse`.
    /// # Errors
    /// Invalid GLB, external resources, out-of-bounds views or exceeded budgets.
    pub fn decode_embedded_images(
        source: &[u8],
        limits: voxy_render::ImageLimits,
        max_images: usize,
    ) -> Result<Vec<voxy_render::ImageAsset>, ModelError> {
        if source.len() > limits.source_bytes {
            return Err(fail("model image source budget exceeded"));
        }
        let gltf = gltf::Gltf::from_slice(source).map_err(|error| fail(error.to_string()))?;
        if gltf.images().count() > max_images {
            return Err(fail("model image count budget exceeded"));
        }
        let mut images = Vec::new();
        let mut remaining = limits.pixel_bytes;
        for image in gltf.images() {
            let gltf::image::Source::View { view, .. } = image.source() else {
                return Err(fail("external model image requires explicit import"));
            };
            if !matches!(view.buffer().source(), gltf::buffer::Source::Bin) {
                return Err(fail("external image buffer requires explicit import"));
            }
            let blob = gltf
                .blob
                .as_deref()
                .ok_or_else(|| fail("missing image buffer"))?;
            if blob.len() < view.buffer().length() {
                return Err(fail("truncated model image buffer"));
            }
            let end = view
                .offset()
                .checked_add(view.length())
                .ok_or_else(|| fail("model image view overflow"))?;
            if end > view.buffer().length() {
                return Err(fail("model image view outside declared buffer"));
            }
            let bytes = blob
                .get(view.offset()..end)
                .ok_or_else(|| fail("model image view outside buffer"))?;
            let decoded = voxy_render::ImageAsset::decode(
                bytes,
                voxy_render::ImageLimits {
                    pixel_bytes: remaining,
                    ..limits
                },
            )
            .map_err(|error| fail(error.to_string()))?;
            remaining = remaining
                .checked_sub(decoded.rgba().len() as u64)
                .ok_or_else(|| fail("model decoded image budget exceeded"))?;
            images.push(decoded);
        }
        Ok(images)
    }

    pub(super) fn parse_base_color_texture(
        primitive: &gltf::Primitive<'_>,
        pbr: &gltf::material::PbrMetallicRoughness<'_>,
    ) -> Result<Option<ModelTexture>, ModelError> {
        pbr.base_color_texture()
            .map(|info| {
                if info.tex_coord() != 0 || primitive.get(&gltf::Semantic::TexCoords(0)).is_none() {
                    return Err(fail("base color texture requires TEXCOORD_0"));
                }
                let sampler = info.texture().sampler();
                let wrap = |mode| match mode {
                    gltf::texture::WrappingMode::ClampToEdge => voxy_render::TextureWrap::Clamp,
                    gltf::texture::WrappingMode::Repeat => voxy_render::TextureWrap::Repeat,
                    gltf::texture::WrappingMode::MirroredRepeat => voxy_render::TextureWrap::Mirror,
                };
                Ok(ModelTexture {
                    image: info.texture().source().index(),
                    use_mips: !matches!(
                        sampler.min_filter(),
                        Some(gltf::texture::MinFilter::Nearest | gltf::texture::MinFilter::Linear)
                    ),
                    sampling: voxy_render::TextureSampling {
                        wrap_u: wrap(sampler.wrap_s()),
                        wrap_v: wrap(sampler.wrap_t()),
                        min_filter: if matches!(
                            sampler.min_filter(),
                            Some(
                                gltf::texture::MinFilter::Nearest
                                    | gltf::texture::MinFilter::NearestMipmapNearest
                                    | gltf::texture::MinFilter::NearestMipmapLinear
                            )
                        ) {
                            voxy_render::TextureFilter::Nearest
                        } else {
                            voxy_render::TextureFilter::Linear
                        },
                        mag_filter: if sampler.mag_filter()
                            == Some(gltf::texture::MagFilter::Nearest)
                        {
                            voxy_render::TextureFilter::Nearest
                        } else {
                            voxy_render::TextureFilter::Linear
                        },
                        mipmap_filter: Some(
                            if matches!(
                                sampler.min_filter(),
                                Some(
                                    gltf::texture::MinFilter::NearestMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                )
                            ) {
                                voxy_render::TextureFilter::Nearest
                            } else {
                                voxy_render::TextureFilter::Linear
                            },
                        ),
                        anisotropy: 1,
                    },
                })
            })
            .transpose()
    }
}

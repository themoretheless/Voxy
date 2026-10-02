//! PNG/JPEG assets with explicit source, dimension and decoded-buffer limits.
use image::{ImageDecoder, ImageFormat, ImageReader};
use std::io::Cursor;

#[derive(Clone, Copy, Debug)]
pub struct ImageLimits {
    pub source_bytes: usize,
    pub dimension: u32,
    /// Maximum combined decoded pixel buffer and RGBA8 conversion buffer.
    /// Decoder scratch allocations have a separate best-effort codec limit.
    pub pixel_bytes: u64,
}
impl Default for ImageLimits {
    fn default() -> Self {
        Self {
            source_bytes: 32 * 1024 * 1024,
            dimension: 8192,
            pixel_bytes: 256 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ImageAsset {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
#[derive(Debug)]
pub enum ImageAssetError {
    LimitExceeded,
    UnsupportedFormat,
    Decode(image::ImageError),
}
impl std::fmt::Display for ImageAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LimitExceeded => f.write_str("image asset limit exceeded"),
            Self::UnsupportedFormat => f.write_str("only PNG and JPEG image assets are supported"),
            Self::Decode(error) => write!(f, "image decoding failed: {error}"),
        }
    }
}
impl std::error::Error for ImageAssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}
impl ImageAsset {
    /// Owns a validated straight-alpha RGBA8 image from a bounded CPU producer.
    /// # Errors
    /// Rejects zero dimensions, overflow, byte counts and decoded-image limits.
    pub fn from_rgba(
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        limits: ImageLimits,
    ) -> Result<Self, ImageAssetError> {
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ImageAssetError::LimitExceeded)?;
        if width == 0
            || height == 0
            || width > limits.dimension
            || height > limits.dimension
            || bytes > limits.pixel_bytes
            || usize::try_from(bytes).ok() != Some(rgba.len())
        {
            return Err(ImageAssetError::LimitExceeded);
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
    /// Converts the first static PNG/JPEG image to straight-alpha RGBA8.
    /// Pixels are uploaded as sRGB; ICC transforms and EXIF orientation are not applied.
    /// # Errors
    /// Rejects unsupported/corrupt files and buffers exceeding the supplied limits.
    pub fn decode(bytes: &[u8], limits: ImageLimits) -> Result<Self, ImageAssetError> {
        if bytes.len() > limits.source_bytes {
            return Err(ImageAssetError::LimitExceeded);
        }
        let format = image::guess_format(bytes).map_err(ImageAssetError::Decode)?;
        if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
            return Err(ImageAssetError::UnsupportedFormat);
        }
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        let mut codec_limits = image::Limits::default();
        codec_limits.max_image_width = Some(limits.dimension);
        codec_limits.max_image_height = Some(limits.dimension);
        codec_limits.max_alloc = Some(limits.pixel_bytes);
        reader.limits(codec_limits);
        let decoder = reader.into_decoder().map_err(ImageAssetError::Decode)?;
        let (width, height) = decoder.dimensions();
        let rgba_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4));
        let peak = rgba_bytes.and_then(|n| n.checked_add(decoder.total_bytes()));
        if width == 0
            || height == 0
            || width > limits.dimension
            || height > limits.dimension
            || peak.is_none_or(|n| n > limits.pixel_bytes)
        {
            return Err(ImageAssetError::LimitExceeded);
        }
        let pixels = image::DynamicImage::from_decoder(decoder).map_err(ImageAssetError::Decode)?;
        let rgba = pixels.into_rgba8().into_raw();
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
    /// Build a complete box-filtered mip chain including this base image.
    /// RGB is averaged in linear light with alpha weighting, then encoded back
    /// to sRGB. Fully transparent output texels have zero RGB. Odd dimensions
    /// partition the entire source, preserving the final row and column.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn mip_chain(&self) -> Vec<Self> {
        let mut levels = vec![self.clone()];
        while levels
            .last()
            .is_some_and(|image| image.width > 1 || image.height > 1)
        {
            let Some(source) = levels.last() else {
                break;
            };
            let width = (source.width / 2).max(1);
            let height = (source.height / 2).max(1);
            let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
            for y in 0..height {
                for x in 0..width {
                    let x0 = u64::from(x) * u64::from(source.width) / u64::from(width);
                    let x1 = u64::from(x + 1) * u64::from(source.width) / u64::from(width);
                    let y0 = u64::from(y) * u64::from(source.height) / u64::from(height);
                    let y1 = u64::from(y + 1) * u64::from(source.height) / u64::from(height);
                    let mut sum = [0.0_f64; 4];
                    for sy in y0..y1 {
                        for sx in x0..x1 {
                            let offset = ((sy * u64::from(source.width) + sx) * 4) as usize;
                            let pixel = &source.rgba[offset..offset + 4];
                            let alpha = f64::from(pixel[3]) / 255.0;
                            for channel in 0..3 {
                                let srgb = f64::from(pixel[channel]) / 255.0;
                                let linear = if srgb <= 0.04045 {
                                    srgb / 12.92
                                } else {
                                    ((srgb + 0.055) / 1.055).powf(2.4)
                                };
                                sum[channel] += linear * alpha;
                            }
                            sum[3] += alpha;
                        }
                    }
                    for channel in sum.iter().take(3) {
                        let linear = if sum[3] > 0.0 { channel / sum[3] } else { 0.0 };
                        let srgb = if linear <= 0.003_130_8 {
                            linear * 12.92
                        } else {
                            1.055 * linear.powf(1.0 / 2.4) - 0.055
                        };
                        rgba.push((srgb * 255.0).round().clamp(0.0, 255.0) as u8);
                    }
                    let count = ((x1 - x0) * (y1 - y0)) as f64;
                    rgba.push((sum[3] / count * 255.0).round().clamp(0.0, 255.0) as u8);
                }
            }
            levels.push(Self {
                width,
                height,
                rgba,
            });
        }
        levels
    }
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                &[255, 128, 0, 127, 0, 64, 255, 255],
                2,
                1,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }
    #[test]
    fn png_preserves_rgba_and_dimensions() {
        let asset = ImageAsset::decode(&png(), ImageLimits::default()).unwrap();
        assert_eq!((asset.width(), asset.height()), (2, 1));
        assert_eq!(asset.rgba(), &[255, 128, 0, 127, 0, 64, 255, 255]);
    }
    #[test]
    fn mips_use_linear_light_and_ignore_transparent_color() {
        let opaque = ImageAsset {
            width: 2,
            height: 1,
            rgba: vec![0, 0, 0, 255, 255, 255, 255, 255],
        };
        assert_eq!(opaque.mip_chain()[1].rgba(), &[188, 188, 188, 255]);
        let alpha = ImageAsset {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 0, 0, 0, 255, 255],
        };
        assert_eq!(alpha.mip_chain()[1].rgba(), &[0, 0, 255, 128]);
        let clear = ImageAsset {
            width: 1,
            height: 2,
            rgba: vec![255, 0, 0, 0, 0, 255, 0, 0],
        };
        assert_eq!(clear.mip_chain()[1].rgba(), &[0, 0, 0, 0]);
    }
    #[test]
    fn odd_mips_include_final_column() {
        let image = ImageAsset {
            width: 3,
            height: 1,
            rgba: vec![0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255],
        };
        let levels = image.mip_chain();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[1].rgba(), &[156, 156, 156, 255]);
    }
    #[test]
    fn jpeg_adds_opaque_alpha() {
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 100)
            .encode(&[120, 120, 120], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let asset = ImageAsset::decode(&bytes, ImageLimits::default()).unwrap();
        assert_eq!(asset.rgba()[3], 255);
        assert!(asset.rgba()[0].abs_diff(120) <= 2);
    }
    #[test]
    fn corruption_and_limits_rejected() {
        let bytes = png();
        for limits in [
            ImageLimits {
                source_bytes: 1,
                ..Default::default()
            },
            ImageLimits {
                dimension: 1,
                ..Default::default()
            },
            ImageLimits {
                pixel_bytes: 15,
                ..Default::default()
            },
        ] {
            assert!(ImageAsset::decode(&bytes, limits).is_err());
        }
        assert!(ImageAsset::decode(&bytes[..bytes.len() / 2], ImageLimits::default()).is_err());
        assert!(ImageAsset::decode(b"not an image", ImageLimits::default()).is_err());
    }
    #[test]
    fn raw_rgba_admission_rejects_size_overflow_and_keeps_straight_alpha() {
        let limits = ImageLimits {
            dimension: 2,
            pixel_bytes: 4,
            ..ImageLimits::default()
        };
        let image = ImageAsset::from_rgba(1, 1, vec![255, 10, 20, 128], limits).unwrap();
        assert_eq!(image.rgba(), &[255, 10, 20, 128]);
        for (width, height, bytes) in [
            (0, 1, vec![]),
            (2, 1, vec![0; 8]),
            (1, 1, vec![0; 3]),
            (u32::MAX, u32::MAX, vec![]),
        ] {
            assert!(ImageAsset::from_rgba(width, height, bytes, limits).is_err());
        }
    }
}

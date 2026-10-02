//! Bounded Radiance RGBE loading into linear RGB32F, without tone mapping.
use crate::ImageLimits;
use image::{ImageDecoder, ImageFormat, ImageReader};
use std::io::Cursor;
#[derive(Clone, Debug)]
pub struct HdrImageAsset {
    width: u32,
    height: u32,
    rgb: Vec<f32>,
}
#[derive(Debug)]
pub enum HdrImageError {
    LimitExceeded,
    InvalidRadiance,
    UnsupportedFormat,
    Decode(image::ImageError),
}
impl std::fmt::Display for HdrImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HDR image error: {self:?}")
    }
}
impl std::error::Error for HdrImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}
impl HdrImageAsset {
    /// Own a linear nonnegative RGB32F image. No exposure, gamma or color conversion.
    /// # Errors
    /// Rejects dimensions, buffer lengths, allocation limits and nonfinite/negative radiance.
    pub fn from_rgb(
        width: u32,
        height: u32,
        rgb: Vec<f32>,
        limits: ImageLimits,
    ) -> Result<Self, HdrImageError> {
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(12))
            .ok_or(HdrImageError::LimitExceeded)?;
        if width == 0
            || height == 0
            || width > limits.dimension
            || height > limits.dimension
            || bytes > limits.pixel_bytes
            || usize::try_from(bytes / 4).ok() != Some(rgb.len())
        {
            return Err(HdrImageError::LimitExceeded);
        }
        if rgb.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(HdrImageError::InvalidRadiance);
        }
        Ok(Self { width, height, rgb })
    }
    /// Decode Radiance .hdr RGBE bytes into linear RGB32F; EXR is not accepted.
    /// Header exposure/color correction metadata is not applied; values are stored radiance.
    /// ImageLimits bounds source, dimensions and combined decoded/conversion buffers.
    /// # Errors
    /// Rejects unsupported/corrupt input or exceeded limits.
    pub fn decode(bytes: &[u8], limits: ImageLimits) -> Result<Self, HdrImageError> {
        if bytes.len() > limits.source_bytes {
            return Err(HdrImageError::LimitExceeded);
        }
        let format = image::guess_format(bytes).map_err(HdrImageError::Decode)?;
        if format != ImageFormat::Hdr {
            return Err(HdrImageError::UnsupportedFormat);
        }
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        let mut codec_limits = image::Limits::default();
        codec_limits.max_image_width = Some(limits.dimension);
        codec_limits.max_image_height = Some(limits.dimension);
        codec_limits.max_alloc = Some(limits.pixel_bytes);
        reader.limits(codec_limits);
        let decoder = reader.into_decoder().map_err(HdrImageError::Decode)?;
        let (width, height) = decoder.dimensions();
        let converted = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(12))
            .ok_or(HdrImageError::LimitExceeded)?;
        if converted
            .checked_add(decoder.total_bytes())
            .is_none_or(|n| n > limits.pixel_bytes)
        {
            return Err(HdrImageError::LimitExceeded);
        }
        let rgb = image::DynamicImage::from_decoder(decoder)
            .map_err(HdrImageError::Decode)?
            .into_rgb32f()
            .into_raw();
        Self::from_rgb(width, height, rgb, limits)
    }
    /// Resample a full equirectangular panorama into +X,-X,+Y,-Y,+Z,-Z RGB32F faces.
    /// Longitude atan2(Z,X) maps +X to U=0.5, +Z to U=0.75; +Y is the north pole.
    /// Bilinear sampling wraps longitude and clamps latitude, preserving linear HDR values.
    /// Limits include the retained panorama and all six output faces. No GPU upload.
    /// # Errors
    /// Rejects zero/excessive face sizes, overflow and combined pixel-buffer limits.
    pub fn cube_faces(
        &self,
        size: u32,
        limits: ImageLimits,
    ) -> Result<[Vec<f32>; 6], HdrImageError> {
        let face_bytes = u64::from(size)
            .checked_mul(u64::from(size))
            .and_then(|n| n.checked_mul(12))
            .ok_or(HdrImageError::LimitExceeded)?;
        let source_bytes = u64::try_from(self.rgb.len())
            .ok()
            .and_then(|n| n.checked_mul(4))
            .ok_or(HdrImageError::LimitExceeded)?;
        if size == 0
            || size > limits.dimension
            || face_bytes
                .checked_mul(6)
                .and_then(|n| n.checked_add(source_bytes))
                .is_none_or(|n| n > limits.pixel_bytes)
        {
            return Err(HdrImageError::LimitExceeded);
        }
        let count = usize::try_from(face_bytes / 4).map_err(|_| HdrImageError::LimitExceeded)?;
        let mut faces = std::array::from_fn(|_| Vec::with_capacity(count));
        for (face, output) in faces.iter_mut().enumerate() {
            for y in 0..size {
                for x in 0..size {
                    let u = (f64::from(x) + 0.5) / f64::from(size) * 2. - 1.;
                    let v = (f64::from(y) + 0.5) / f64::from(size) * 2. - 1.;
                    let direction = match face {
                        0 => [1., -v, -u],
                        1 => [-1., -v, u],
                        2 => [u, 1., v],
                        3 => [u, -1., -v],
                        4 => [u, -v, 1.],
                        _ => [-u, -v, -1.],
                    };
                    let length = direction.iter().map(|n| n * n).sum::<f64>().sqrt();
                    let longitude = direction[2].atan2(direction[0]) / std::f64::consts::TAU + 0.5;
                    let latitude =
                        (direction[1] / length).clamp(-1., 1.).acos() / std::f64::consts::PI;
                    let sx = longitude * f64::from(self.width) - 0.5;
                    let sy = (latitude * f64::from(self.height) - 0.5)
                        .clamp(0., f64::from(self.height - 1));
                    let x0 = sx.floor() as i64;
                    let y0 = sy.floor() as u32;
                    let fx = sx - sx.floor();
                    let fy = sy - sy.floor();
                    for c in 0..3 {
                        let sample = |px: i64, py: u32| {
                            f64::from(
                                self.rgb[((u64::from(py) * u64::from(self.width)
                                    + px.rem_euclid(i64::from(self.width)) as u64)
                                    * 3) as usize
                                    + c],
                            )
                        };
                        let upper = sample(x0, y0) * (1. - fx) + sample(x0 + 1, y0) * fx;
                        let lower = sample(x0, (y0 + 1).min(self.height - 1)) * (1. - fx)
                            + sample(x0 + 1, (y0 + 1).min(self.height - 1)) * fx;
                        output.push((upper * (1. - fy) + lower * fy) as f32);
                    }
                }
            }
        }
        Ok(faces)
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
    pub fn rgb(&self) -> &[f32] {
        &self.rgb
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Vec<u8> {
        let mut b = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 2\n".to_vec();
        b.extend_from_slice(&[128, 64, 32, 130, 128, 64, 32, 132]);
        b
    }
    #[test]
    fn rgbe_linear_hdr_preserved() {
        let asset = HdrImageAsset::decode(&source(), ImageLimits::default()).unwrap();
        assert_eq!((asset.width(), asset.height()), (2, 1));
        assert_eq!(asset.rgb(), &[2., 1., 0.5, 8., 4., 2.]);
    }
    #[test]
    fn source_and_combined_buffers_bounded() {
        let b = source();
        for limits in [
            ImageLimits {
                source_bytes: b.len() - 1,
                ..Default::default()
            },
            ImageLimits {
                pixel_bytes: 47,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                HdrImageAsset::decode(&b, limits),
                Err(HdrImageError::LimitExceeded)
            ));
        }
        assert!(HdrImageAsset::decode(&b[..b.len() - 1], ImageLimits::default()).is_err());
    }
    #[test]
    fn format_and_exact_budget() {
        assert!(matches!(
            HdrImageAsset::decode(b"\x89PNG\r\n\x1a\n", ImageLimits::default()),
            Err(HdrImageError::UnsupportedFormat)
        ));
        assert!(
            HdrImageAsset::decode(
                &source(),
                ImageLimits {
                    pixel_bytes: 48,
                    ..Default::default()
                }
            )
            .is_ok()
        );
        assert!(
            HdrImageAsset::decode(
                &source(),
                ImageLimits {
                    dimension: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn panorama_constant_and_limits() {
        let asset =
            HdrImageAsset::from_rgb(8, 4, [4., 2., 8.].repeat(32), ImageLimits::default()).unwrap();
        for size in [1, 3, 8] {
            let faces = asset.cube_faces(size, ImageLimits::default()).unwrap();
            for face in faces {
                assert_eq!(face, [4., 2., 8.].repeat((size * size) as usize));
            }
        }
        assert!(asset.cube_faces(0, ImageLimits::default()).is_err());
        assert!(
            asset
                .cube_faces(
                    1,
                    ImageLimits {
                        pixel_bytes: 455,
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert!(
            asset
                .cube_faces(
                    1,
                    ImageLimits {
                        pixel_bytes: 456,
                        ..Default::default()
                    }
                )
                .is_ok()
        );
        assert!(asset.cube_faces(u32::MAX, ImageLimits::default()).is_err());
    }
    #[test]
    fn panorama_face_axes_and_wrapped_seam() {
        let mut rgb = Vec::new();
        for y in 0..4 {
            for x in 0..8 {
                rgb.extend([x as f32 + 1., y as f32 + 1., 8.]);
            }
        }
        let asset = HdrImageAsset::from_rgb(8, 4, rgb, ImageLimits::default()).unwrap();
        let faces = asset.cube_faces(1, ImageLimits::default()).unwrap();
        assert_eq!(faces[0], vec![4.5, 2.5, 8.]);
        assert_eq!(faces[1], vec![4.5, 2.5, 8.]);
        assert_eq!(faces[2][1], 1.);
        assert_eq!(faces[3][1], 4.);
        assert_eq!(faces[4], vec![6.5, 2.5, 8.]);
        assert_eq!(faces[5], vec![2.5, 2.5, 8.]);
    }
    #[test]
    fn cpu_radiance_validated() {
        for invalid in [f32::NAN, f32::INFINITY, -1.] {
            assert!(matches!(
                HdrImageAsset::from_rgb(1, 1, vec![invalid, 0., 0.], ImageLimits::default()),
                Err(HdrImageError::InvalidRadiance)
            ));
        }
        assert!(HdrImageAsset::from_rgb(2, 1, vec![0.; 3], ImageLimits::default()).is_err());
    }
}

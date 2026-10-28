use super::*;
use crate::{GeometryArchiveLimits, GeometryCheckpointError, ImageCheckpoint};
impl ImageAsset {
    #[must_use]
    pub fn checkpoint(&self) -> ImageCheckpoint {
        let Self {
            width,
            height,
            rgba,
        } = self;
        ImageCheckpoint {
            version: 1,
            width: *width,
            height: *height,
            rgba: rgba.clone(),
        }
    }
}
impl ImageCheckpoint {
    pub fn prepare(
        &self,
        limits: GeometryArchiveLimits,
    ) -> Result<ImageAsset, GeometryCheckpointError> {
        self.encoded_size(limits)?;
        ImageAsset::from_rgba(
            self.width,
            self.height,
            self.rgba.clone(),
            ImageLimits {
                source_bytes: 0,
                dimension: limits.max_image_dimension,
                pixel_bytes: limits.max_pixel_bytes,
            },
        )
        .map_err(GeometryCheckpointError::Image)
    }
}

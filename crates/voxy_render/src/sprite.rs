//! CPU sprite batching for a shared texture/atlas and painter-ordered overlays.
use crate::{SceneError, SceneMesh, SceneVertex};
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub center: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    /// UV rectangle; reversed endpoints mirror the image.
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub color: [f32; 4],
}
impl Sprite {
    /// Crops visual top-left fractions of the unrotated rectangle and its UVs.
    /// Rotation is preserved around the new center, keeping the same visible
    /// textured subrectangle instead of stretching the original image.
    /// # Errors
    /// Rejects nonfinite/out-of-range fractions or empty crop.
    pub fn cropped(self, min: Vec2, max: Vec2) -> Result<Self, SpriteBatchError> {
        if !min.is_finite()
            || !max.is_finite()
            || min.min_element() < 0.0
            || max.max_element() > 1.0
            || (max - min).min_element() <= 0.0
        {
            return Err(SpriteBatchError::InvalidSprite);
        }
        let midpoint = (min + max) * 0.5;
        let offset = Vec2::new(midpoint.x - 0.5, 0.5 - midpoint.y) * self.size;
        let (sin, cos) = self.rotation.sin_cos();
        let center = self.center
            + Vec2::new(
                cos * offset.x - sin * offset.y,
                sin * offset.x + cos * offset.y,
            );
        let range = self.uv_max - self.uv_min;
        Ok(Self {
            center,
            size: self.size * (max - min),
            uv_min: self.uv_min + range * min,
            uv_max: self.uv_min + range * max,
            ..self
        })
    }
    /// Converts a top-left logical pixel rectangle to clip-space sprite geometry.
    /// Logical viewport size must match the coordinates used by input/layout.
    /// Render with identity MVP and an overlay draw. Device DPI does not enter
    /// this mapping: the render target scales the normalized geometry.
    /// # Errors
    /// Rejects invalid viewport/rectangle/color or nonfinite converted geometry.
    pub fn from_logical_rect(
        origin: Vec2,
        size: Vec2,
        viewport: Vec2,
        color: [f32; 4],
    ) -> Result<Self, SpriteBatchError> {
        if !origin.is_finite()
            || !size.is_finite()
            || size.min_element() <= 0.0
            || !viewport.is_finite()
            || viewport.min_element() <= 0.0
            || color.iter().any(|c| !c.is_finite())
        {
            return Err(SpriteBatchError::InvalidSprite);
        }
        let normalized_size = size / viewport * 2.0;
        let midpoint = (origin + size * 0.5) / viewport * 2.0;
        let center = Vec2::new(midpoint.x - 1.0, 1.0 - midpoint.y);
        if !center.is_finite()
            || !normalized_size.is_finite()
            || normalized_size.min_element() <= 0.0
        {
            return Err(SpriteBatchError::InvalidSprite);
        }
        Ok(Self {
            center,
            size: normalized_size,
            color,
            ..Self::default()
        })
    }
}
impl Default for Sprite {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            size: Vec2::ONE,
            rotation: 0.0,
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            color: [1.0; 4],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpriteBatchError {
    InvalidSprite,
    Capacity,
    Empty,
    Geometry(SceneError),
}
impl std::fmt::Display for SpriteBatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sprite batch error: {self:?}")
    }
}
impl std::error::Error for SpriteBatchError {}

#[derive(Debug)]
pub struct SpriteBatch {
    vertices: Vec<SceneVertex>,
    indices: Vec<u32>,
    capacity: usize,
}
impl SpriteBatch {
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            capacity,
        }
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.vertices.len() / 4
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Appends in painter order. All sprites use the texture assigned to the draw.
    /// # Errors
    /// Rejects invalid sprites/capacity before modifying batch geometry.
    pub fn push(&mut self, sprite: Sprite) -> Result<(), SpriteBatchError> {
        if !sprite.center.is_finite()
            || !sprite.size.is_finite()
            || sprite.size.min_element() <= 0.0
            || !sprite.rotation.is_finite()
            || !sprite.uv_min.is_finite()
            || !sprite.uv_max.is_finite()
            || sprite.color.into_iter().any(|c| !c.is_finite())
        {
            return Err(SpriteBatchError::InvalidSprite);
        }
        if self.len() >= self.capacity {
            return Err(SpriteBatchError::Capacity);
        }
        let base = u32::try_from(self.vertices.len()).map_err(|_| SpriteBatchError::Capacity)?;
        if base.checked_add(3).is_none()
            || self
                .indices
                .len()
                .checked_add(6)
                .is_none_or(|count| u32::try_from(count).is_err())
        {
            return Err(SpriteBatchError::Capacity);
        }
        let (sin, cos) = sprite.rotation.sin_cos();
        let corners = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ];
        let uvs = [
            Vec2::new(sprite.uv_min.x, sprite.uv_max.y),
            sprite.uv_max,
            Vec2::new(sprite.uv_max.x, sprite.uv_min.y),
            sprite.uv_min,
        ];
        let mut vertices = [SceneVertex {
            position: [0.0; 3],
            uv: [0.0; 2],
            color: sprite.color,
        }; 4];
        for ((vertex, corner), uv) in vertices.iter_mut().zip(corners).zip(uvs) {
            let p = corner * sprite.size;
            let p = sprite.center + Vec2::new(cos * p.x - sin * p.y, sin * p.x + cos * p.y);
            if !p.is_finite() {
                return Err(SpriteBatchError::InvalidSprite);
            }
            vertex.position = Vec3::new(p.x, p.y, 0.0).to_array();
            vertex.uv = uv.to_array();
        }
        self.vertices.extend(vertices);
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        Ok(())
    }

    /// Produces one indexed mesh. Geometry is in the batch's coordinate space;
    /// use an orthographic MVP and an overlay draw for screen-space sprites.
    /// # Errors
    /// Rejects empty geometry or unrepresentable mesh size.
    pub fn mesh(&self) -> Result<SceneMesh, SpriteBatchError> {
        if self.is_empty() {
            return Err(SpriteBatchError::Empty);
        }
        SceneMesh::new(self.vertices.clone(), self.indices.clone())
            .map_err(SpriteBatchError::Geometry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_sprite_and_atlas_uvs_are_painter_ordered() {
        let mut batch = SpriteBatch::new(2);
        batch
            .push(Sprite {
                center: Vec2::new(10.0, 20.0),
                size: Vec2::new(4.0, 2.0),
                rotation: std::f32::consts::FRAC_PI_2,
                uv_min: Vec2::new(0.25, 0.5),
                uv_max: Vec2::new(0.75, 1.0),
                ..Default::default()
            })
            .unwrap();
        let p = Vec2::from_array([batch.vertices[0].position[0], batch.vertices[0].position[1]]);
        assert!(p.distance(Vec2::new(11.0, 18.0)) < 1e-5);
        batch.push(Sprite::default()).unwrap();
        assert_eq!(&batch.indices[6..], &[4, 5, 6, 4, 6, 7]);
        assert!(Vec2::from_array(batch.vertices[0].uv).distance(Vec2::new(0.25, 1.0)) < 1e-5);
        assert!(batch.mesh().is_ok());
    }
    #[test]
    fn invalid_or_full_batch_does_not_publish_partial_geometry() {
        let mut batch = SpriteBatch::new(1);
        assert!(batch.mesh().is_err());
        batch.push(Sprite::default()).unwrap();
        assert_eq!(
            batch.push(Sprite::default()),
            Err(SpriteBatchError::Capacity)
        );
        assert_eq!(batch.len(), 1);
        batch.clear();
        assert_eq!(
            batch.push(Sprite {
                size: Vec2::new(-1.0, 2.0),
                ..Default::default()
            }),
            Err(SpriteBatchError::InvalidSprite)
        );
        assert!(batch.is_empty());
        assert_eq!(
            batch.push(Sprite {
                center: Vec2::splat(f32::MAX),
                size: Vec2::splat(f32::MAX),
                ..Default::default()
            }),
            Err(SpriteBatchError::InvalidSprite)
        );
        assert!(batch.is_empty());
    }
}

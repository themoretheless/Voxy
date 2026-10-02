//! Reversible X-ray transitions and shared linear-color material policy.
use crate::SceneDepthMode;

/// Colors are linear RGB; alpha is straight alpha. Parameters are validated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrayStyle {
    pub shell_color: [f32; 3],
    pub internal_color: [f32; 3],
    pub shell_opacity: f32,
    pub rim_opacity: f32,
    /// Time for a complete 0 -> 1 transition in seconds. Zero switches instantly.
    pub transition_seconds: f32,
}
impl Default for XrayStyle {
    fn default() -> Self {
        Self {
            shell_color: [0.12, 0.45, 1.0],
            internal_color: [1.0, 0.38, 0.05],
            shell_opacity: 0.035,
            rim_opacity: 0.22,
            transition_seconds: 0.3,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XrayError {
    InvalidStyle,
    InvalidDelta,
    InvalidRegion,
    InvalidPosition,
    InvalidTransform,
    InvalidMesh,
}
impl std::fmt::Display for XrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "X-ray error: {self:?}")
    }
}
impl std::error::Error for XrayError {}

/// A sphere in caller-chosen model/world coordinates. Feather lies inside its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrayRegion {
    center: glam::Vec3,
    radius: f32,
    feather: f32,
}
impl XrayRegion {
    /// # Errors
    /// Requires finite center, positive radius and feather in [0, radius].
    pub fn sphere(center: glam::Vec3, radius: f32, feather: f32) -> Result<Self, XrayError> {
        if !center.is_finite()
            || !radius.is_finite()
            || radius <= 0.0
            || !feather.is_finite()
            || !(0.0..=radius).contains(&feather)
        {
            return Err(XrayError::InvalidRegion);
        }
        Ok(Self {
            center,
            radius,
            feather,
        })
    }
    #[must_use]
    pub fn center(&self) -> glam::Vec3 {
        self.center
    }
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }
    #[must_use]
    pub fn feather(&self) -> f32 {
        self.feather
    }
    /// Smoothstep weight; uses f64 distance to avoid f32 subtraction overflow.
    /// # Errors
    /// Rejects nonfinite sample positions.
    #[allow(clippy::cast_possible_truncation)] // Smoothstep is bounded to [0, 1].
    pub fn weight(&self, position: glam::Vec3) -> Result<f32, XrayError> {
        if !position.is_finite() {
            return Err(XrayError::InvalidPosition);
        }
        let distance = (position.as_dvec3() - self.center.as_dvec3()).length();
        if distance >= f64::from(self.radius) {
            return Ok(0.0);
        }
        if self.feather == 0.0 {
            return Ok(1.0);
        }
        let t =
            ((f64::from(self.radius) - distance) / f64::from(self.feather)).clamp(0.0, 1.0) as f32;
        Ok(t * t * (3.0 - 2.0 * t))
    }
}

/// Independent of geometry, physics, camera and GPU ownership.
/// Apply colors to the original material every frame, not the previous tinted result.
#[derive(Clone, Copy, Debug)]
pub struct XrayEffect {
    style: XrayStyle,
    enabled: bool,
    amount: f32,
    region: Option<XrayRegion>,
}
impl Default for XrayEffect {
    fn default() -> Self {
        Self {
            style: XrayStyle::default(),
            enabled: false,
            amount: 0.0,
            region: None,
        }
    }
}
impl XrayEffect {
    /// # Errors
    /// Rejects nonfinite/negative colors or duration and alpha outside [0, 1].
    pub fn new(style: XrayStyle) -> Result<Self, XrayError> {
        let mut effect = Self::default();
        effect.set_style(style)?;
        Ok(effect)
    }
    /// Updates material parameters without resetting an in-flight transition.
    /// # Errors
    /// Invalid styles leave the entire effect unchanged.
    pub fn set_style(&mut self, style: XrayStyle) -> Result<(), XrayError> {
        if style
            .shell_color
            .iter()
            .chain(&style.internal_color)
            .any(|c| !c.is_finite() || *c < 0.0)
            || !style.transition_seconds.is_finite()
            || style.transition_seconds < 0.0
            || !style.shell_opacity.is_finite()
            || !(0.0..=1.0).contains(&style.shell_opacity)
            || !style.rim_opacity.is_finite()
            || !(0.0..=1.0).contains(&style.rim_opacity)
        {
            return Err(XrayError::InvalidStyle);
        }
        self.style = style;
        if style.transition_seconds == 0.0 {
            self.amount = if self.enabled { 1.0 } else { 0.0 };
        }
        Ok(())
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if self.style.transition_seconds == 0.0 {
            self.amount = if enabled { 1.0 } else { 0.0 };
        }
    }
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    #[must_use]
    pub fn amount(&self) -> f32 {
        self.amount
    }
    #[must_use]
    pub fn style(&self) -> XrayStyle {
        self.style
    }
    /// Advance with real time; slowing physics need not slow the reveal animation.
    /// # Errors
    /// Invalid deltas leave the effect unchanged.
    pub fn advance(&mut self, seconds: f32) -> Result<(), XrayError> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(XrayError::InvalidDelta);
        }
        if self.style.transition_seconds > 0.0 {
            let step = seconds / self.style.transition_seconds;
            self.amount = (self.amount + if self.enabled { step } else { -step }).clamp(0.0, 1.0);
        }
        Ok(())
    }
    #[must_use]
    pub fn shell_depth_mode(&self) -> SceneDepthMode {
        if self.amount == 0.0 {
            SceneDepthMode::Opaque
        } else {
            SceneDepthMode::Transparent
        }
    }
    #[must_use]
    pub fn internal_depth_mode(&self) -> SceneDepthMode {
        if self.amount == 0.0 {
            SceneDepthMode::Opaque
        } else {
            SceneDepthMode::Xray
        }
    }
    /// None reveals the entire supplied internal mesh.
    pub fn set_region(&mut self, region: Option<XrayRegion>) {
        self.region = region;
    }
    #[must_use]
    pub fn region(&self) -> Option<XrayRegion> {
        self.region
    }
    /// Colors a separate reveal layer; fades its alpha as well as its tint.
    /// Keep ordinary internals as a separate world draw if needed when disabled.
    /// # Errors
    /// Rejects nonfinite positions.
    pub fn reveal_color_at(
        &self,
        base: [f32; 4],
        position: glam::Vec3,
    ) -> Result<[f32; 4], XrayError> {
        if !position.is_finite() {
            return Err(XrayError::InvalidPosition);
        }
        let weight = self
            .region
            .map_or(Ok(1.0), |region| region.weight(position))?;
        let mut color = self.internal_color(base);
        color[3] *= self.amount * weight;
        Ok(color)
    }
    /// Builds a vertex-sampled regional reveal without modifying source geometry.
    /// `local_to_region` must be affine and invertible. Fine tessellation gives
    /// smoother mask boundaries; this is not exact per-fragment sphere clipping.
    /// # Errors
    /// Rejects invalid transforms, positions or source vertex attributes.
    pub fn reveal_mesh(
        &self,
        source: &crate::SceneMesh,
        local_to_region: glam::Mat4,
    ) -> Result<crate::SceneMesh, XrayError> {
        if !local_to_region.is_finite()
            || local_to_region.row(3) != glam::Vec4::W
            || !local_to_region.determinant().is_finite()
            || local_to_region.determinant() == 0.0
        {
            return Err(XrayError::InvalidTransform);
        }
        let mut vertices = source.vertices().to_vec();
        for vertex in &mut vertices {
            vertex.color = self.reveal_color_at(
                vertex.color,
                local_to_region.transform_point3(glam::Vec3::from_array(vertex.position)),
            )?;
        }
        crate::SceneMesh::new(vertices, source.indices().to_vec())
            .map_err(|_| XrayError::InvalidMesh)
    }
    /// `rim` is a caller-provided Fresnel weight; clamped to [0, 1].
    #[must_use]
    pub fn shell_color(&self, base: [f32; 4], rim: f32) -> [f32; 4] {
        let rim = if rim.is_finite() {
            rim.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let alpha = (self.style.shell_opacity + rim * self.style.rim_opacity).min(1.0);
        self.mix(base, self.style.shell_color, alpha * base[3])
    }
    /// Preserves source alpha, mixes source RGB toward the highlight tint.
    #[must_use]
    pub fn internal_color(&self, base: [f32; 4]) -> [f32; 4] {
        self.mix(base, self.style.internal_color, base[3])
    }
    fn mix(&self, base: [f32; 4], rgb: [f32; 3], alpha: f32) -> [f32; 4] {
        let target = [rgb[0], rgb[1], rgb[2], alpha];
        std::array::from_fn(|i| base[i] * (1.0 - self.amount) + target[i] * self.amount)
    }
}
#[cfg(test)]
// These tests assert exact boundary weights and unchanged material values.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    #[test]
    fn regional_mask_feathers_and_handles_extreme_coordinates() {
        let region = XrayRegion::sphere(glam::Vec3::ZERO, 2.0, 1.0).unwrap();
        assert_eq!(region.weight(glam::Vec3::ZERO).unwrap(), 1.0);
        assert_eq!(region.weight(glam::Vec3::X * 1.5).unwrap(), 0.5);
        assert_eq!(region.weight(glam::Vec3::X * 2.0).unwrap(), 0.0);
        assert_eq!(region.weight(glam::Vec3::splat(f32::MAX)).unwrap(), 0.0);
        assert_eq!(
            region.weight(glam::Vec3::NAN),
            Err(XrayError::InvalidPosition)
        );
        assert!(XrayRegion::sphere(glam::Vec3::ZERO, 1.0, 2.0).is_err());
    }
    #[test]
    fn mesh_mask_respects_transform_and_preserves_source() {
        let mut effect = XrayEffect::new(XrayStyle {
            transition_seconds: 0.0,
            ..Default::default()
        })
        .unwrap();
        effect.set_enabled(true);
        effect.set_region(Some(
            XrayRegion::sphere(glam::Vec3::ZERO, 1.0, 0.0).unwrap(),
        ));
        let source = crate::SceneMesh::quad([1.0; 4]);
        let mesh = effect
            .reveal_mesh(&source, glam::Mat4::from_translation(glam::Vec3::X * 3.0))
            .unwrap();
        assert!(mesh.vertices().iter().all(|v| v.color[3] == 0.0));
        assert!(source.vertices().iter().all(|v| v.color == [1.0; 4]));
        assert!(effect.reveal_mesh(&source, glam::Mat4::ZERO).is_err());
        effect.set_enabled(false);
        assert!(
            effect
                .reveal_mesh(&source, glam::Mat4::IDENTITY)
                .unwrap()
                .vertices()
                .iter()
                .all(|v| v.color[3] == 0.0)
        );
    }
    #[test]
    fn reversal_is_continuous_and_restores_original_material() {
        let mut effect = XrayEffect::new(XrayStyle {
            transition_seconds: 1.0,
            ..Default::default()
        })
        .unwrap();
        let base = [0.5, 0.4, 0.3, 1.0];
        effect.set_enabled(true);
        effect.advance(0.4).unwrap();
        let before = effect.shell_color(base, 0.5);
        effect.set_enabled(false);
        assert_eq!(effect.shell_color(base, 0.5), before);
        effect.advance(0.2).unwrap();
        assert!((effect.amount() - 0.2).abs() < 1e-6);
        effect.advance(1.0).unwrap();
        assert_eq!(effect.shell_color(base, 0.5), base);
        assert_eq!(effect.shell_depth_mode(), SceneDepthMode::Opaque);
    }
    #[test]
    fn timestep_partition_and_source_transparency_are_preserved() {
        let mut a = XrayEffect::default();
        let mut b = a;
        a.set_enabled(true);
        b.set_enabled(true);
        a.advance(0.15).unwrap();
        for _ in 0..3 {
            b.advance(0.05).unwrap();
        }
        assert!((a.amount() - b.amount()).abs() < 1e-6);
        assert_eq!(a.shell_color([1.0, 1.0, 1.0, 0.0], 1.0)[3], 0.0);
        assert_eq!(a.internal_color([1.0, 1.0, 1.0, 0.25])[3], 0.25);
        assert_eq!(a.advance(-1.0), Err(XrayError::InvalidDelta));
        assert_eq!(a.advance(f32::INFINITY), Err(XrayError::InvalidDelta));
    }
    #[test]
    fn validation_is_transactional_and_zero_duration_is_immediate() {
        let mut effect = XrayEffect::default();
        for style in [
            XrayStyle {
                shell_opacity: 1.1,
                ..Default::default()
            },
            XrayStyle {
                transition_seconds: f32::NAN,
                ..Default::default()
            },
            XrayStyle {
                internal_color: [-1.0; 3],
                ..Default::default()
            },
        ] {
            assert_eq!(effect.set_style(style), Err(XrayError::InvalidStyle));
            assert_eq!(effect.style(), XrayStyle::default());
        }
        effect
            .set_style(XrayStyle {
                transition_seconds: 0.0,
                ..Default::default()
            })
            .unwrap();
        effect.set_enabled(true);
        assert_eq!(effect.amount(), 1.0);
        assert_eq!(effect.advance(f32::NAN), Err(XrayError::InvalidDelta));
        assert_eq!(effect.amount(), 1.0);
        assert_eq!(effect.internal_depth_mode(), SceneDepthMode::Xray);
        effect.set_enabled(false);
        assert_eq!(effect.amount(), 0.0);
    }
}

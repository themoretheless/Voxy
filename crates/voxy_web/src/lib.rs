//! Browser platform shell using the shared general 2D/3D renderer.
#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
mod gravity_validation;

#[cfg(target_arch = "wasm32")]
mod collision_validation;
#[cfg(target_arch = "wasm32")]
mod water_validation;

#[cfg(target_arch = "wasm32")]
mod voxel_scene;

#[cfg(target_arch = "wasm32")]
mod voxel_game;

#[cfg(target_arch = "wasm32")]
mod voxel_vehicle;

#[cfg(target_arch = "wasm32")]
mod voxel_terrain;

#[cfg(target_arch = "wasm32")]
mod voxel_water;

#[cfg(target_arch = "wasm32")]
mod animated_scene;

#[cfg(target_arch = "wasm32")]
mod temporal_validation;

#[cfg(target_arch = "wasm32")]
mod temporal_guides_probe;

#[cfg(target_arch = "wasm32")]
mod temporal_color_probe;

/// Computes the physical backing-store canvas dimensions `(width_px, height_px)` from CSS logical
/// size `(css_width, css_height)` and `device_pixel_ratio`, clamped to `max_texture_dimension_2d`.
///
/// Returns `None` if any input is non-finite, non-positive, or `max_dimension == 0`.
#[must_use]
pub fn clamp_canvas_backing_size(
    css_width: f64,
    css_height: f64,
    device_pixel_ratio: f64,
    max_dimension: u32,
) -> Option<(u32, u32)> {
    if !css_width.is_finite()
        || !css_height.is_finite()
        || !device_pixel_ratio.is_finite()
        || css_width <= 0.0
        || css_height <= 0.0
        || device_pixel_ratio <= 0.0
        || max_dimension == 0
    {
        return None;
    }
    let dpr = device_pixel_ratio.clamp(0.5, 4.0);
    let raw_w = (css_width * dpr).round().max(1.0);
    let raw_h = (css_height * dpr).round().max(1.0);
    let max_f = f64::from(max_dimension);
    let scale = (max_f / raw_w).min(max_f / raw_h).min(1.0);
    let w = ((raw_w * scale).round() as u32).clamp(1, max_dimension);
    let h = ((raw_h * scale).round() as u32).clamp(1, max_dimension);
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_canvas_backing_size_respects_dpr_and_aspect_ratio_under_max_limit() {
        assert_eq!(
            clamp_canvas_backing_size(1280.0, 720.0, 2.0, 4096),
            Some((2560, 1440))
        );
        // Exceeding 2048 on width scales both axes proportionally:
        assert_eq!(
            clamp_canvas_backing_size(2000.0, 1000.0, 2.0, 2000),
            Some((2000, 1000))
        );
        assert!(clamp_canvas_backing_size(0.0, 720.0, 1.0, 2048).is_none());
    }
}

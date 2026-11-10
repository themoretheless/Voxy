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

/// Clamps a requested WebGPU buffer allocation budget to a safe browser tab limit
/// (`<= 512 MiB` on 32-bit/WASM32 address spaces and aligned down to 256-byte uniform alignment).
#[must_use]
pub fn clamp_webgpu_buffer_allocation_budget(
    requested_bytes: u64,
    device_max_buffer_size: u64,
) -> u64 {
    const WASM32_TAB_SAFE_CAP: u64 = 512 * 1024 * 1024;
    let cap = device_max_buffer_size.min(WASM32_TAB_SAFE_CAP);
    let clamped = requested_bytes.min(cap);
    clamped & !255_u64
}

/// Evaluates the Peak Signal-to-Noise Ratio $\text{PSNR} = 10 \log_{10}(\text{peak}^2 / \text{mse})$ in dB,
/// returning `100.0` dB for identical frames (`mse == 0.0`) and `None` for invalid inputs.
#[must_use]
pub fn psnr_db_from_mse(mse: f64, peak_signal: f64) -> Option<f64> {
    if !mse.is_finite() || mse < 0.0 || !peak_signal.is_finite() || peak_signal <= 0.0 {
        return None;
    }
    if mse <= 1e-20 {
        return Some(100.0);
    }
    let ratio = (peak_signal * peak_signal) / mse;
    let db = (10.0 * ratio.log10()).min(100.0);
    db.is_finite().then_some(db)
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
        assert_eq!(
            clamp_webgpu_buffer_allocation_budget(1024 * 1024 * 1024, 2 * 1024 * 1024 * 1024),
            512 * 1024 * 1024
        );
        assert!((psnr_db_from_mse(0.01, 1.0).unwrap() - 20.0).abs() < 1e-9);
        assert_eq!(psnr_db_from_mse(0.0, 1.0), Some(100.0));
    }
}

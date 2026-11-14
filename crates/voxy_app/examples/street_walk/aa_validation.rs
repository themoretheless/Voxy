//! Deterministic full-scene camera route shared by every AA mode.
use glam::{Quat, Vec3};
use std::path::PathBuf;
#[allow(dead_code)]
#[derive(Debug)]
pub struct AaValidation {
    pub directory: PathBuf,
    pub eye: Vec3,
    pub direction: Vec3,
    pub frame_ms: Vec<f64>,
    pub cycle_metalfx: bool,
}
#[allow(dead_code)]
impl AaValidation {
    pub fn from_environment() -> Option<Self> {
        std::env::var_os("VOXY_STREET_AA_CAPTURE").map(|path| Self {
            directory: path.into(),
            eye: Vec3::ZERO,
            direction: Vec3::NEG_Z,
            frame_ms: Vec::new(),
            cycle_metalfx: std::env::var_os("VOXY_STREET_AA_CYCLE").is_some(),
        })
    }
    pub fn camera(&self, frame: u64) -> (Vec3, Vec3) {
        match frame {
            0..32 => (self.eye, self.direction),
            32..64 => (
                self.eye,
                Quat::from_rotation_y((frame - 32) as f32 * 0.005) * self.direction,
            ),
            64..96 => (
                self.eye + Vec3::X * ((frame - 64) as f32 * 0.015),
                Quat::from_rotation_y(0.155) * self.direction,
            ),
            96..128 => (
                self.eye + Vec3::X * 3.,
                Quat::from_rotation_y(-0.2) * self.direction,
            ),
            _ => (
                self.eye + Vec3::X * 3.,
                Quat::from_rotation_y(-0.2) * self.direction,
            ),
        }
    }
    pub fn capture(frame: u64) -> bool {
        (24..=32).contains(&frame)
            || [64, 96, 97, 98, 112, 128, 129, 130, 144, 160].contains(&frame)
    }
}

/// Computes the RMS subpixel temporal luma flicker score across high-contrast edges
/// between two consecutive frames of equal length.
#[allow(dead_code)]
pub fn edge_flicker_score(prev_luma: &[f32], curr_luma: &[f32]) -> Option<f32> {
    if prev_luma.is_empty() || prev_luma.len() != curr_luma.len() {
        return None;
    }
    let mut sum_sq = 0.0_f64;
    let mut count = 0usize;
    for (&p, &c) in prev_luma.iter().zip(curr_luma.iter()) {
        if p.is_finite() && c.is_finite() {
            let diff = f64::from(c - p);
            sum_sq += diff * diff;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some((sum_sq / count as f64).sqrt() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_flicker_score_is_zero_for_identical_frames() {
        let a = [0.1, 0.5, 0.9, 0.2];
        assert_eq!(edge_flicker_score(&a, &a), Some(0.0));
        let b = [0.2, 0.4, 0.8, 0.3];
        let score = edge_flicker_score(&a, &b).unwrap();
        assert!((score - 0.1).abs() < 1e-6);
        assert_eq!(edge_flicker_score(&a, &[0.1]), None);

        let solid_wire = [0.8_f32, 0.82, 0.79, 0.81, 0.80];
        let broken_wire = [0.8_f32, 0.0, 0.85, 0.0, 0.8];
        assert!(
            subpixel_wire_continuity_score(&solid_wire)
                > subpixel_wire_continuity_score(&broken_wire)
        );
    }
}

/// Evaluates a spatial continuity score in `[0.0, 1.0]` along a 1D scanline of a thin subpixel wire
/// or powerline (`1.0` = uniform unbroken coverage, lower = broken/stippled aliasing gaps).
#[allow(dead_code)]
pub fn subpixel_wire_continuity_score(scanline_luma: &[f32]) -> f32 {
    if scanline_luma.len() < 2 {
        return 0.0;
    }
    let valid: Vec<f32> = scanline_luma
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .collect();
    if valid.len() < 2 {
        return 0.0;
    }
    let mean = valid.iter().sum::<f32>() / (valid.len() as f32);
    if mean <= 1e-5 {
        return 0.0;
    }
    let mean_abs_step =
        valid.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / ((valid.len() - 1) as f32);
    (1.0 / (1.0 + mean_abs_step / mean)).clamp(0.0, 1.0)
}

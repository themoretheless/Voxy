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

//▫~•◦------------------------------------------------------------------------------------◦•~▫//
// █████████████████████████████████████ VOXY ENGINE █████████████████████████████████████ //
//▫~•◦------------------------------------------------------------------------------------◦•~▫//

//! Real-time "Bodycam" / "Unrecord" Photorealistic Optics, CMOS Sensor, ISP Post-Processing,
//! and 6-DOF Chest-Mounted Camera Biomechanics (`voxy_render::bodycam`).

use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::CameraView;

/// Exact framerate-independent 3D second-order spring-damper solver.
/// Supports both underdamped ($\zeta < 1$, oscillatory rattle/footstep shock)
/// and critically damped ($\zeta \ge 1$, smooth heavy torso follow) regimes
/// without Euler integration instability.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpringDamper3D {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl SpringDamper3D {
    pub fn add_impulse(&mut self, impulse: Vec3) {
        if impulse.is_finite() {
            self.velocity += impulse;
        }
    }

    pub fn step(&mut self, goal: Vec3, dt: f32, frequency_hz: f32, damping_zeta: f32) {
        let dt = dt.clamp(0.0, 0.1);
        if dt <= 1e-6 || !goal.is_finite() {
            return;
        }
        let omega = 2.0 * PI * frequency_hz.max(0.05);
        let zeta = damping_zeta.max(0.01);
        let x0 = self.position - goal;
        let v0 = self.velocity;

        if zeta < 0.9999 {
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            let exp_term = (-zeta * omega * dt).exp();
            let cos_t = (omega_d * dt).cos();
            let sin_t = (omega_d * dt).sin();

            let c1 = x0;
            let c2 = (v0 + zeta * omega * x0) / omega_d;

            self.position = goal + exp_term * (c1 * cos_t + c2 * sin_t);
            self.velocity = exp_term
                * ((c2 * omega_d - c1 * zeta * omega) * cos_t
                    - (c1 * omega_d + c2 * zeta * omega) * sin_t);
        } else {
            let exp_term = (-omega * dt).exp();
            let c1 = x0;
            let c2 = v0 + omega * x0;

            self.position = goal + (c1 + c2 * dt) * exp_term;
            self.velocity = (v0 - c2 * omega * dt) * exp_term;
        }
    }
}

/// 6-DOF Chest-Mounted Bodycam Kinematic Rig (Axon Body 3 / Tactical Plate Carrier).
///
/// Decouples raw aim/flashlight direction from torso/camera orientation via an elliptical
/// free-aim deadzone, offsets the lens from the spine pivot by the physical chest-mount
/// lever arm `(+0.06m, +0.18m, +0.22m)`, and layers analytical second-order spring-dampers
/// for torso tracking, heel-strike footstep shock, vest clip micro-rattle, and asymmetric
/// human respiration.
#[derive(Clone, Debug)]
pub struct BodycamRig {
    /// Raw aim orientation `(yaw, pitch)` in radians (controls flashlight/weapon immediately).
    pub aim_angles: Vec2,
    /// Goal torso orientation `(yaw, pitch)` pulled by the elliptical free-aim deadzone.
    pub chest_goal_angles: Vec2,
    /// Critically damped torso rotation spring `(yaw, pitch, roll)` in radians.
    pub torso_spring: SpringDamper3D,
    /// Underdamped footstep heel-strike translation spring (meters).
    pub footstep_pos_spring: SpringDamper3D,
    /// Underdamped footstep heel-strike angular shock spring (radians).
    pub footstep_rot_spring: SpringDamper3D,
    /// High-frequency loose plastic mount clip chatter spring (radians).
    pub mount_rattle_spring: SpringDamper3D,
    /// Accumulated stride phase in radians `[0, 2*PI)`; one step per `PI`,
    /// heel strikes at `0` and `PI`.
    pub stride_phase: f32,
    /// Smoothed gait envelope `0..=1`: ramps in when walking starts and lets the
    /// sway settle over a few steps after stopping instead of freezing.
    pub gait_amount: f32,
    /// Smoothed ground speed driving gait amplitudes (m/s).
    pub gait_speed: f32,
    /// Accumulated breathing phase in radians.
    pub breath_phase: f32,
    /// Elapsed time in seconds.
    pub elapsed_seconds: f32,
    /// Last step parity (`false` = left foot, `true` = right foot).
    pub step_parity: bool,
    /// Current screen-space angular velocity `(d_yaw, d_pitch)` for 180-deg shutter & rolling shutter.
    pub screen_angular_velocity: Vec2,
    /// Dynamic Electronic Image Stabilization (EIS) / mount shift offset in UV space.
    pub eis_mount_offset: Vec2,
    /// Chest mount lever-arm offset from upper spine pivot in meters.
    pub chest_mount_offset: Vec3,
    /// Static mount tilt roll bias in radians (chest cams never sit 100% level).
    pub static_roll_bias: f32,
    /// Vertical field of view in radians (default ~102 deg for wide-angle bodycam before fisheye warp).
    pub vertical_fov_radians: f32,
    /// Whether the tactical chest/weapon flashlight is switched on.
    pub flashlight_enabled: bool,
    /// Asymmetric auto-exposure state (current EV100).
    pub current_ev100: f32,
    /// Quantized exposure gain sent to sensor.
    pub exposure_gain: f32,
    /// Automatic ISO gain multiplier (`1.0` in daylight up to `12.0` in dark interiors).
    pub iso_gain: f32,
    /// Optical & biomechanical profile (`Bodycam`, `CleanCam`, `Camcorder`).
    pub lens_profile: PhotorealLensProfile,
    /// Motorized camcorder optical zoom multiplier (`1.0x` to `12.0x`, smoothed by servo spring).
    pub camcorder_zoom: f32,
    /// Target optical zoom multiplier set by mouse wheel (`1.0`..=`12.0`).
    pub camcorder_zoom_goal: f32,
    /// Smooth servo zoom velocity (`1/s`).
    pub camcorder_zoom_vel: f32,
}

/// Optical & biomechanical preset for the photorealistic HDR pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhotorealLensProfile {
    /// *Bodycam* / *Unrecord* chest-mounted action camera (Axon Body 3 / GoPro Hero 11):
    /// Kannala-Brandt fisheye distortion, corner housing bezel, strong CA, ISP halos,
    /// elliptical free-aim deadzone, and chest lever-arm spring kinematics.
    #[default]
    Bodycam,
    /// Clean full-frame rectilinear cinema/eye camera (`--cam`):
    /// Full-bleed frame (no corner bezel or fisheye warp), natural eye-level view (`70 deg` FOV),
    /// subtle 35mm cinema bloom & fine grain, AgX tone mapping, and direct responsive aiming.
    CleanCam,
    /// Hand-held digital camcorder / ENG video camera (`--camcorder`, Sony DCR-VX1000 / Panasonic AG-DVX100):
    /// Shoulder/wrist-mounted fluid-head inertia, motorized servo zoom (`1.0x–12.0x`), autofocus
    /// breathing pulse, CCD vertical highlight smear, horizontal interlaced scanlines, and live
    /// viewfinder OSD overlay (`REC ●`, SMPTE timecode `HH:MM:SS:FF`, battery, zoom bar, center crosshair).
    Camcorder,
}

impl Default for BodycamRig {
    fn default() -> Self {
        let initial_aim = Vec2::new(-135_f32.to_radians(), -8_f32.to_radians());
        Self {
            aim_angles: initial_aim,
            chest_goal_angles: initial_aim,
            torso_spring: SpringDamper3D {
                position: Vec3::new(initial_aim.x, initial_aim.y, 0.0),
                velocity: Vec3::ZERO,
            },
            footstep_pos_spring: SpringDamper3D::default(),
            footstep_rot_spring: SpringDamper3D::default(),
            mount_rattle_spring: SpringDamper3D::default(),
            stride_phase: 0.0,
            gait_amount: 0.0,
            gait_speed: 0.0,
            breath_phase: 0.0,
            elapsed_seconds: 0.0,
            step_parity: false,
            screen_angular_velocity: Vec2::ZERO,
            eis_mount_offset: Vec2::ZERO,
            chest_mount_offset: Vec3::new(0.06, 0.18, 0.22),
            static_roll_bias: 2.1_f32.to_radians(),
            vertical_fov_radians: 98_f32.to_radians(),
            flashlight_enabled: true,
            current_ev100: 0.2,
            exposure_gain: 1.0,
            iso_gain: 1.8,
            lens_profile: PhotorealLensProfile::Bodycam,
            camcorder_zoom: 1.0,
            camcorder_zoom_goal: 1.0,
            camcorder_zoom_vel: 0.0,
        }
    }
}

impl BodycamRig {
    /// Switch between `PhotorealLensProfile::Bodycam` (fisheye chest-cam),
    /// `PhotorealLensProfile::CleanCam` (rectilinear eye-level camera), and
    /// `PhotorealLensProfile::Camcorder` (hand-held video camcorder with OSD & servo zoom).
    pub fn set_lens_profile(&mut self, profile: PhotorealLensProfile) {
        self.lens_profile = profile;
        match profile {
            PhotorealLensProfile::Bodycam => {
                self.chest_mount_offset = Vec3::new(0.06, 0.18, 0.22);
                self.static_roll_bias = 2.1_f32.to_radians();
                self.vertical_fov_radians = 98_f32.to_radians();
            }
            PhotorealLensProfile::CleanCam => {
                self.chest_mount_offset = Vec3::new(0.0, 0.28, 0.08);
                self.static_roll_bias = 0.0;
                self.vertical_fov_radians = 70_f32.to_radians();
                self.chest_goal_angles = self.aim_angles;
                self.torso_spring.position = Vec3::new(self.aim_angles.x, self.aim_angles.y, 0.0);
            }
            PhotorealLensProfile::Camcorder => {
                // Hand-held right-side shoulder/palm camcorder offset (+12cm right, +24cm up, +16cm forward)
                self.chest_mount_offset = Vec3::new(0.12, 0.24, 0.16);
                self.static_roll_bias = -0.85_f32.to_radians();
                self.vertical_fov_radians = 62_f32.to_radians();
                self.chest_goal_angles = self.aim_angles;
            }
        }
    }

    /// Adjust motorized camcorder optical zoom target (e.g., from mouse scroll wheel `lines`).
    pub fn apply_servo_zoom(&mut self, wheel_lines: f32) {
        if !wheel_lines.is_finite() {
            return;
        }
        let factor = 1.18_f32.powf(wheel_lines.clamp(-4.0, 4.0));
        self.camcorder_zoom_goal = (self.camcorder_zoom_goal * factor).clamp(1.0, 12.0);
    }

    /// Apply mouse/controller look delta `(delta_yaw, delta_pitch)` with elliptical Free-Aim deadzone.
    pub fn apply_look(&mut self, look_delta: Vec2) {
        if !look_delta.is_finite() {
            return;
        }
        // Scale look sensitivity inversely with optical zoom when in Camcorder mode
        let zoom_sens = if self.lens_profile == PhotorealLensProfile::Camcorder {
            1.0 / self.camcorder_zoom.clamp(1.0, 12.0).sqrt()
        } else {
            1.0
        };
        let scaled_delta = look_delta * zoom_sens;
        self.aim_angles.x -= scaled_delta.x;
        self.aim_angles.y =
            (self.aim_angles.y - scaled_delta.y).clamp(-78_f32.to_radians(), 72_f32.to_radians());

        if self.lens_profile == PhotorealLensProfile::CleanCam {
            self.chest_goal_angles = self.aim_angles;
            return;
        }
        if self.lens_profile == PhotorealLensProfile::Camcorder {
            // Smooth fluid-head wrist follow (small 3.5x2.5 deg cushion for organic hand-held pan)
            self.chest_goal_angles.x -= scaled_delta.x * 0.65;
            self.chest_goal_angles.y = (self.chest_goal_angles.y - scaled_delta.y * 0.65)
                .clamp(-75_f32.to_radians(), 68_f32.to_radians());
            let deadzone = Vec2::new(3.5_f32.to_radians(), 2.5_f32.to_radians());
            let diff = self.aim_angles - self.chest_goal_angles;
            let norm_dist = ((diff.x / deadzone.x).powi(2) + (diff.y / deadzone.y).powi(2)).sqrt();
            if norm_dist > 1.0 {
                self.chest_goal_angles = self.aim_angles - diff / norm_dist;
            }
            return;
        }

        // Sympathetic 20% torso coupling inside the deadzone
        self.chest_goal_angles.x -= look_delta.x * 0.20;
        self.chest_goal_angles.y = (self.chest_goal_angles.y - look_delta.y * 0.20)
            .clamp(-75_f32.to_radians(), 68_f32.to_radians());

        // Elliptical free-aim deadzone: 13 deg horizontal yaw x 8.5 deg vertical pitch
        let deadzone = Vec2::new(13.0_f32.to_radians(), 8.5_f32.to_radians());
        let diff = self.aim_angles - self.chest_goal_angles;
        let norm_dist = ((diff.x / deadzone.x).powi(2) + (diff.y / deadzone.y).powi(2)).sqrt();
        if norm_dist > 1.0 {
            self.chest_goal_angles = self.aim_angles - diff / norm_dist;
        }
    }

    /// Inject a sudden physical recoil or impact shock (e.g., firing projectile, explosion, landing).
    pub fn add_recoil_impulse(&mut self, pitch_up_rad: f32, yaw_rad: f32, roll_rad: f32) {
        self.footstep_rot_spring
            .add_impulse(Vec3::new(pitch_up_rad, yaw_rad, roll_rad));
        self.mount_rattle_spring.add_impulse(Vec3::new(
            pitch_up_rad * 2.2,
            -yaw_rad * 1.8,
            roll_rad * 3.1,
        ));
        self.footstep_pos_spring
            .add_impulse(Vec3::new(0.0, -0.06, -0.14));
    }

    /// Advance the 6-DOF biomechanical rig and auto-exposure loop by `dt` seconds.
    pub fn step(
        &mut self,
        dt: f32,
        planar_speed: f32,
        grounded: bool,
        just_jumped_or_landed: bool,
        estimated_scene_luma: f32,
    ) {
        let dt = dt.clamp(0.0, 0.05);
        if dt <= 1e-6 {
            return;
        }
        self.elapsed_seconds += dt;

        // 1. Footstep Heel-Strike Impulse Generation
        let speed = if grounded {
            planar_speed.clamp(0.0, 12.0)
        } else {
            0.0
        };
        // Gait envelope and smoothed speed: people do not start or stop swaying instantly.
        let moving = speed > 0.15;
        let envelope_rate = if moving { 3.5 } else { 2.2 };
        self.gait_amount +=
            (f32::from(u8::from(moving)) - self.gait_amount) * (1.0 - (-envelope_rate * dt).exp());
        if moving {
            self.gait_speed += (speed - self.gait_speed) * (1.0 - (-4.0 * dt).exp());
        }
        if self.gait_amount > 0.01 {
            let prev_phase = self.stride_phase;
            let cadence = step_cadence_hz(self.gait_speed);
            self.stride_phase = (self.stride_phase + cadence * PI * dt) % (2.0 * PI);
            let heel_strike =
                (prev_phase < PI && self.stride_phase >= PI) || self.stride_phase < prev_phase;
            if heel_strike && moving {
                self.step_parity = !self.step_parity;
                let side = if self.step_parity { 1.0 } else { -1.0 };
                // The continuous gait curve carries the bob; the impulse only adds
                // the short heel-strike shock transmitted through the skeleton.
                let intensity = (speed / 5.5).clamp(0.25, 1.6) * 0.35;

                self.footstep_pos_spring.add_impulse(Vec3::new(
                    side * 0.045 * intensity,
                    -0.19 * intensity,
                    0.05 * intensity,
                ));
                self.footstep_rot_spring.add_impulse(Vec3::new(
                    -1.6_f32.to_radians() * intensity,
                    side * 0.9_f32.to_radians() * intensity,
                    side * 2.4_f32.to_radians() * intensity,
                ));
                self.mount_rattle_spring.add_impulse(Vec3::new(
                    2.8_f32.to_radians() * intensity,
                    -side * 1.9_f32.to_radians() * intensity,
                    side * 3.6_f32.to_radians() * intensity,
                ));
            }
        }

        if just_jumped_or_landed {
            self.add_recoil_impulse(-4.5_f32.to_radians(), 0.0, 3.2_f32.to_radians());
        }

        // 2. Torso Banking Roll Coupled to Horizontal Turn Rate (Whip-Pan Inertia)
        let yaw_error = self.chest_goal_angles.x - self.torso_spring.position.x;
        let bank_roll = (-yaw_error * 0.22).clamp(-7.5_f32.to_radians(), 7.5_f32.to_radians());
        let torso_goal = Vec3::new(
            self.chest_goal_angles.x,
            self.chest_goal_angles.y,
            self.static_roll_bias + bank_roll,
        );

        let prev_torso = self.torso_spring.position;
        self.torso_spring.step(torso_goal, dt, 3.1, 1.0); // Critically damped heavy torso
        self.footstep_pos_spring.step(Vec3::ZERO, dt, 6.4, 0.40); // Underdamped step shock
        self.footstep_rot_spring.step(Vec3::ZERO, dt, 7.8, 0.38); // Underdamped rotational shock
        self.mount_rattle_spring.step(Vec3::ZERO, dt, 15.5, 0.22); // Loose vest mount chatter

        // 3. Asymmetric Respiration + Postural Hand-Held Micro-Sway
        let breath_rate_hz = 0.28 + 0.06 * speed;
        self.breath_phase = (self.breath_phase + 2.0 * PI * breath_rate_hz * dt) % (2.0 * PI);

        let total_angular_vel = (self.torso_spring.position - prev_torso) / dt
            + self.footstep_rot_spring.velocity
            + self.mount_rattle_spring.velocity * 0.5;

        self.screen_angular_velocity = Vec2::new(
            (total_angular_vel.x * 0.0028).clamp(-0.015, 0.015),
            (total_angular_vel.y * 0.0028).clamp(-0.015, 0.015),
        );
        self.eis_mount_offset = Vec2::new(
            (-self.footstep_pos_spring.position.x * 0.14 - total_angular_vel.x * 0.0018)
                .clamp(-0.020, 0.020),
            (-self.footstep_pos_spring.position.y * 0.18 + total_angular_vel.y * 0.0018)
                .clamp(-0.020, 0.020),
        );

        // 4. Asymmetric Auto-Exposure (AE) Adaptation with 1/10-EV Hardware Stepping
        let target_luma = estimated_scene_luma.clamp(0.02, 16.0);
        // Positive EV bias indoors blows out windows and flashlight hotspots like real bodycams
        let ev_bias = if self.flashlight_enabled { 0.48 } else { 0.32 };
        let target_ev100 = (target_luma / 0.18).log2() - ev_bias;
        let adapt_rate = if target_ev100 > self.current_ev100 {
            2.4 // Closing iris / shortening shutter on bright light
        } else {
            0.85 // Sluggish gain boost when entering dark rooms
        };
        self.current_ev100 +=
            (target_ev100 - self.current_ev100) * (1.0 - (-adapt_rate * dt).exp());

        // Quantize to 1/10 EV steps to mimic digital ISP register stepping
        let quantized_ev = (self.current_ev100 * 12.0).round() / 12.0;
        self.exposure_gain = (1.0 / (1.15 * 2.0_f32.powf(quantized_ev))).clamp(0.18, 6.5);
        self.iso_gain = (self.exposure_gain * 1.45).clamp(1.0, 12.0);

        // 5. Motorized Camcorder Servo Zoom Integration (critically damped 2.8 Hz rocker servo)
        let zoom_omega = 2.0 * PI * 2.8;
        let zoom_err = self.camcorder_zoom - self.camcorder_zoom_goal;
        let zoom_exp = (-zoom_omega * dt).exp();
        let zoom_c2 = self.camcorder_zoom_vel + zoom_omega * zoom_err;
        self.camcorder_zoom =
            (self.camcorder_zoom_goal + (zoom_err + zoom_c2 * dt) * zoom_exp).clamp(1.0, 12.0);
        self.camcorder_zoom_vel = (self.camcorder_zoom_vel - zoom_c2 * zoom_omega * dt) * zoom_exp;
    }

    /// Continuous gait sway of the upper torso: `(translation (right, up, forward)
    /// in metres, rotation (pitch, yaw, roll) in radians)`.
    ///
    /// Follows the inverted-pendulum walk: the body is lowest at each heel strike
    /// and highest at mid-stance (one vertical cycle per step), shifts and leans over
    /// the stance leg (one lateral cycle per stride), and the shoulders counter-rotate
    /// against the pelvis. Running raises the bob and narrows the lateral shift.
    #[must_use]
    pub fn gait_offsets(&self) -> (Vec3, Vec3) {
        let amount = self.gait_amount.clamp(0.0, 1.0);
        if amount <= 1e-4 {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        let v = self.gait_speed.clamp(0.0, 8.0);
        let phase = self.stride_phase;
        let step_wave = (2.0 * phase).cos(); // +1 at heel strike, -1 at mid-stance
        let stride_wave = phase.sin(); // +1 over the right stance, -1 over the left

        let bob = (0.012 + 0.006 * v).min(0.045);
        let lateral = 0.028 / (1.0 + 0.15 * v);
        let surge = 0.004 * (v / 1.4).min(2.0);
        let nod = (0.45 + 0.12 * v).min(1.4).to_radians();
        let twist = (1.4 + 0.25 * v).min(3.2).to_radians();
        let lean = (1.1 + 0.08 * v).min(1.8).to_radians();

        // A chest mount follows the torso fully; hands absorb some of it, and an
        // eye-level camera is stabilised by the head and neck.
        let (translate, rotate) = match self.lens_profile {
            PhotorealLensProfile::Bodycam => (1.0, 1.0),
            PhotorealLensProfile::Camcorder => (0.75, 0.8),
            PhotorealLensProfile::CleanCam => (0.55, 0.4),
        };
        let position = Vec3::new(
            lateral * stride_wave,
            -bob * step_wave,
            surge * (2.0 * phase).sin(),
        ) * (amount * translate);
        let rotation = Vec3::new(-nod * step_wave, twist * stride_wave, lean * stride_wave)
            * (amount * rotate);
        (position, rotation)
    }

    /// Compute the 6-DOF chest-mounted [`CameraView`] and tactical flashlight `(origin, direction)`
    /// for a character standing at `actor_feet_pos`.
    #[must_use]
    pub fn camera_and_flashlight(&self, actor_feet_pos: Vec3) -> (CameraView, Vec3, Vec3) {
        let t = self.elapsed_seconds;
        // Asymmetric human breathing waveform: faster inhalation, slower exhalation
        let breath_wave = self.breath_phase.sin() - 0.25 * (2.0 * self.breath_phase).sin();
        // Hand-held camcorder has slightly more pronounced wrist tremor & autofocus breathing at telephoto zoom
        let hand_amp = if self.lens_profile == PhotorealLensProfile::Camcorder {
            1.45
        } else {
            1.0
        };
        // Two-octave postural sway so the camera never freezes into a dead CGI screenshot
        let sway_pitch = hand_amp
            * (0.0042 * (t * 2.1).sin() + 0.0021 * (t * 5.3).cos() + 0.0009 * (t * 11.7).sin());
        let sway_yaw = hand_amp
            * (0.0038 * (t * 1.7).cos() + 0.0019 * (t * 4.7).sin() + 0.0008 * (t * 10.3).cos());
        let sway_roll = hand_amp * (0.0035 * (t * 2.9).sin() + 0.0012 * (t * 7.1).cos());

        let (gait_pos, gait_rot) = self.gait_offsets();

        let yaw = self.torso_spring.position.x
            + gait_rot.y
            + self.footstep_rot_spring.position.y
            + self.mount_rattle_spring.position.y * 0.45
            + sway_yaw;
        let pitch = (self.torso_spring.position.y
            + gait_rot.x
            + self.footstep_rot_spring.position.x
            + self.mount_rattle_spring.position.x * 0.45
            + 0.0065 * breath_wave
            + sway_pitch)
            .clamp(-82_f32.to_radians(), 82_f32.to_radians());
        let roll = self.torso_spring.position.z
            + gait_rot.z
            + self.footstep_rot_spring.position.z
            + self.mount_rattle_spring.position.z * 0.65
            + sway_roll;

        let body_forward = Vec3::new(
            self.torso_spring.position.x.sin(),
            0.0,
            self.torso_spring.position.x.cos(),
        );
        let body_right = body_forward.cross(Vec3::Y);

        // Spine upper-thoracic pivot (1.38m above feet)
        let spine_pivot = actor_feet_pos
            + Vec3::new(0.0, 1.38 + 0.005 * breath_wave, 0.0)
            + self.footstep_pos_spring.position
            + body_right * gait_pos.x
            + Vec3::Y * gait_pos.y
            + body_forward * gait_pos.z;

        // Rotate chest-mount lever arm by torso orientation -> creates authentic 3D arc parallax!
        let chest_rot = Quat::from_euler(glam::EulerRot::YXZ, yaw, -pitch, roll);
        let forward = Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        )
        .normalize_or_zero();
        // Right-handed camera basis: right = forward x WORLD_UP, base_up = right x forward (positive Y!)
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let base_up = right.cross(forward).normalize_or_zero();
        let rolled_up = (base_up * roll.cos() + right * roll.sin()).normalize_or_zero();

        let eye = spine_pivot
            + right * self.chest_mount_offset.x
            + rolled_up * self.chest_mount_offset.y
            + forward * self.chest_mount_offset.z;

        // Flashlight / on-camera video light is aimed by the player's hands (`aim_angles`) with slight inertia
        let aim_yaw = self.aim_angles.x + sway_yaw * 0.6;
        let aim_pitch = self.aim_angles.y + sway_pitch * 0.6;
        let flashlight_dir = Vec3::new(
            aim_yaw.sin() * aim_pitch.cos(),
            aim_pitch.sin(),
            aim_yaw.cos() * aim_pitch.cos(),
        )
        .normalize_or_zero();
        // Flashlight / hot-shoe video light mounted slightly above/right of the lens
        let flashlight_origin = eye + right * 0.14 - rolled_up * 0.08 + forward * 0.18;
        let _ = chest_rot;

        let effective_fov = if self.lens_profile == PhotorealLensProfile::Camcorder {
            // Optical zoom: FOV = 2 * atan(tan(base_fov / 2) / zoom) + subtle AF breathing pulse
            let zoom = self.camcorder_zoom.clamp(1.0, 12.0);
            let af_breath = 1.0
                + 0.0045
                    * (self.camcorder_zoom_vel.abs().min(2.0) * (t * 14.0).sin()
                        + 0.25 * (t * 2.3).sin());
            (2.0 * ((self.vertical_fov_radians * 0.5).tan() / zoom).atan() * af_breath)
                .clamp(6_f32.to_radians(), 110_f32.to_radians())
        } else {
            self.vertical_fov_radians
        };

        let camera = CameraView {
            eye,
            target: eye + forward * 10.0,
            up: rolled_up,
            vertical_fov_radians: effective_fov,
            near_plane: 0.08,
        };
        (camera, flashlight_origin, flashlight_dir)
    }
}

/// Steps per second for a ground speed in m/s: ~1.9 at a 1.4 m/s walk,
/// ~2.9 at a 5.5 m/s run.
fn step_cadence_hz(speed: f32) -> f32 {
    (1.55 + 0.25 * speed.max(0.0)).min(3.1)
}

/// GPU Uniform layout matching `BodycamPostUniforms` in `bodycam_post.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BodycamPostUniforms {
    pub resolution: [f32; 4],
    pub sensor_timing: [f32; 4],
    pub lens_distortion: [f32; 4],
    pub optics_params: [f32; 4],
    pub isp_params: [f32; 4],
    pub kinematics: [f32; 4],
    pub white_balance: [f32; 4],
}

impl Default for BodycamPostUniforms {
    fn default() -> Self {
        Self {
            resolution: [1280.0, 720.0, 1.0 / 1280.0, 1.0 / 720.0],
            sensor_timing: [0.0, 0.0, 1.05, 1.6],
            // Kannala-Brandt / Brown-Conrady Axon Body 3 wide-angle lens profile
            lens_distortion: [-0.215, 0.058, 0.0014, -0.0011],
            // ca_strength, vignette_strength, bloom_intensity, dirt_intensity
            optics_params: [0.0092, 0.84, 0.15, 0.42],
            // unsharp_amount (ISP halo), highlight_clip, compression_q, rolling_shutter
            isp_params: [1.75, 4.0, 96.0, 0.24],
            kinematics: [0.0, 0.0, 0.0, 0.0],
            // Cold overcast / industrial mixed Bradford AWB + 0.88 inner lens crop zoom
            white_balance: [0.97, 1.01, 1.04, 0.88],
        }
    }
}

/// Linear 16-bit floating-point HDR texture format used for offscreen scene rendering
/// before the physical lens / CMOS sensor / ISP post-processing pass.
#[allow(dead_code)]
pub const HDR_SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Offscreen Linear HDR (`Rgba16Float`) Target + Physical Lens/CMOS/ISP Post-Processing Pass.
#[derive(Debug)]
pub struct BodycamPostProcess {
    hdr_texture: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    uniforms: BodycamPostUniforms,
    width: u32,
    height: u32,
}

impl BodycamPostProcess {
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let (hdr_texture, hdr_view) = create_hdr_target(device, width, height);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Voxy bodycam HDR linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let uniforms = BodycamPostUniforms {
            resolution: [
                width as f32,
                height as f32,
                1.0 / width as f32,
                1.0 / height as f32,
            ],
            ..BodycamPostUniforms::default()
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy bodycam post uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Voxy bodycam post bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = create_post_bind_group(
            device,
            &bind_group_layout,
            &hdr_view,
            &sampler,
            &uniform_buffer,
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Voxy bodycam post shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bodycam_post.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Voxy bodycam post pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Voxy bodycam post pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            hdr_texture,
            hdr_view,
            sampler,
            uniform_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            uniforms,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        (self.hdr_texture, self.hdr_view) = create_hdr_target(device, width, height);
        self.uniforms.resolution = [
            width as f32,
            height as f32,
            1.0 / width as f32,
            1.0 / height as f32,
        ];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
        self.bind_group = create_post_bind_group(
            device,
            &self.bind_group_layout,
            &self.hdr_view,
            &self.sampler,
            &self.uniform_buffer,
        );
    }

    pub fn update_from_rig(&mut self, queue: &wgpu::Queue, rig: &BodycamRig, frame_index: u64) {
        let drift = (rig.elapsed_seconds * 0.35).sin() * 0.015;
        match rig.lens_profile {
            PhotorealLensProfile::Bodycam => {
                self.uniforms.sensor_timing = [
                    rig.elapsed_seconds,
                    (frame_index % 16384) as f32,
                    rig.exposure_gain,
                    rig.iso_gain,
                ];
                self.uniforms.lens_distortion = [-0.215, 0.058, 0.0014, -0.0011];
                self.uniforms.optics_params = [0.0092, 0.84, 0.15, 0.42];
                self.uniforms.isp_params = [1.75, 4.0, 96.0, 0.24];
                self.uniforms.kinematics = [
                    rig.screen_angular_velocity.x,
                    rig.screen_angular_velocity.y,
                    rig.eis_mount_offset.x,
                    rig.eis_mount_offset.y,
                ];
                self.uniforms.white_balance = if rig.flashlight_enabled {
                    [0.96 + drift, 1.01, 1.05 - drift, 0.88]
                } else {
                    [0.94 + drift, 1.02, 1.08 - drift, 0.88]
                };
            }
            PhotorealLensProfile::CleanCam => {
                // Full-frame rectilinear cinema camera: clean geometry, no housing bezel (zoom = 1.0),
                // subtle 35mm optical vignette, low sensor noise, crisp natural acutance
                self.uniforms.sensor_timing = [
                    rig.elapsed_seconds,
                    (frame_index % 16384) as f32,
                    rig.exposure_gain,
                    (rig.iso_gain * 0.35).clamp(0.4, 1.8),
                ];
                self.uniforms.lens_distortion = [-0.018, 0.004, 0.0, 0.0];
                self.uniforms.optics_params = [0.0018, 0.28, 0.10, 0.08];
                self.uniforms.isp_params = [0.65, 6.5, 255.0, 0.04];
                self.uniforms.kinematics = [
                    rig.screen_angular_velocity.x * 0.35,
                    rig.screen_angular_velocity.y * 0.35,
                    0.0,
                    0.0,
                ];
                self.uniforms.white_balance = [0.99 + drift * 0.4, 1.00, 1.01 - drift * 0.4, 1.0];
            }
            PhotorealLensProfile::Camcorder => {
                // Hand-held MiniDV / Digital Video Camcorder (Sony DCR-VX1000 / Panasonic AG-DVX100):
                // - Moderate telephoto/wide lens barrel distortion (-0.075)
                // - Noticeable lateral chromatic aberration at zoom edges (0.0052)
                // - Video ISP peaking / edge sharpening halos (1.45)
                // - Tape/DV 4:2:0 chroma subsampling & quantization (78.0)
                // - Negative white_balance.w (-camcorder_zoom) triggers the Camcorder OSD HUD,
                //   CCD vertical smear, and interlaced scanlines in `bodycam_post.wgsl`!
                self.uniforms.sensor_timing = [
                    rig.elapsed_seconds,
                    (frame_index % 16384) as f32,
                    rig.exposure_gain,
                    (rig.iso_gain * 0.85).clamp(0.9, 6.0),
                ];
                self.uniforms.lens_distortion = [-0.075, 0.018, 0.0005, -0.0004];
                self.uniforms.optics_params = [0.0052, 0.52, 0.16, 0.22];
                self.uniforms.isp_params = [1.45, 3.8, 78.0, 0.16];
                self.uniforms.kinematics = [
                    rig.screen_angular_velocity.x * 0.75,
                    rig.screen_angular_velocity.y * 0.75,
                    rig.eis_mount_offset.x * 0.65,
                    rig.eis_mount_offset.y * 0.65,
                ];
                self.uniforms.white_balance = [
                    0.98 + drift * 0.8,
                    1.01,
                    1.02 - drift * 0.8,
                    -rig.camcorder_zoom.clamp(1.0, 12.0),
                ];
            }
        }
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }

    #[must_use]
    pub const fn hdr_view(&self) -> &wgpu::TextureView {
        &self.hdr_view
    }

    #[must_use]
    pub const fn hdr_texture(&self) -> &wgpu::Texture {
        &self.hdr_texture
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Voxy bodycam post-process pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn create_hdr_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Voxy bodycam linear HDR target"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_post_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    hdr_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Voxy bodycam post bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(hdr_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}

#[cfg(test)]
mod gait_tests {
    use super::*;

    fn walk(rig: &mut BodycamRig, speed: f32, seconds: f32) -> (f32, f32) {
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        let mut lateral = 0.0_f32;
        for _ in 0..(seconds * 120.0) as usize {
            rig.step(1.0 / 120.0, speed, true, false, 0.18);
            let (p, _) = rig.gait_offsets();
            low = low.min(p.y);
            high = high.max(p.y);
            lateral = lateral.max(p.x.abs());
        }
        (high - low, lateral)
    }

    #[test]
    fn walking_bob_and_lateral_sway_are_human_scale() {
        let mut rig = BodycamRig::default();
        let (bob, lateral) = walk(&mut rig, 1.4, 4.0);
        assert!((0.03..0.06).contains(&bob), "walk bob {bob}");
        assert!((0.015..0.03).contains(&lateral), "walk lateral {lateral}");
        let mut runner = BodycamRig::default();
        let (run_bob, run_lateral) = walk(&mut runner, 5.5, 4.0);
        assert!(run_bob > bob && run_lateral < lateral);
    }

    #[test]
    fn sway_settles_after_stopping() {
        let mut rig = BodycamRig::default();
        walk(&mut rig, 2.2, 3.0);
        assert!(rig.gait_offsets().0.length() > 0.0);
        walk(&mut rig, 0.0, 3.0);
        assert!(rig.gait_offsets().0.length() < 0.001);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_damper_is_stable_and_converges_in_both_regimes() {
        let mut underdamped = SpringDamper3D::default();
        underdamped.add_impulse(Vec3::new(1.0, -2.0, 0.5));
        let goal = Vec3::new(0.5, 1.0, -0.25);
        for _ in 0..240 {
            underdamped.step(goal, 1.0 / 60.0, 6.5, 0.4);
        }
        assert!(underdamped.position.distance(goal) < 1e-3);

        let mut critical = SpringDamper3D::default();
        for _ in 0..240 {
            critical.step(goal, 1.0 / 60.0, 3.0, 1.0);
        }
        assert!(critical.position.distance(goal) < 1e-3);
    }

    #[test]
    fn bodycam_rig_chest_lever_arm_and_free_aim_deadzone_work() {
        let mut rig = BodycamRig::default();
        let initial_chest = rig.chest_goal_angles;
        // Small look delta inside the 13x8.5 deg deadzone moves aim 100%, chest only 20%
        rig.apply_look(Vec2::new(0.05, -0.03));
        assert!((rig.aim_angles.x - (initial_chest.x - 0.05)).abs() < 1e-5);
        assert!((rig.chest_goal_angles.x - (initial_chest.x - 0.01)).abs() < 1e-5);

        rig.step(1.0 / 60.0, 4.2, true, true, 0.25);
        let (camera, light_pos, light_dir) = rig.camera_and_flashlight(Vec3::new(10.0, 2.0, 10.0));
        assert!(camera.eye.is_finite());
        assert!(light_pos.is_finite());
        assert!((light_dir.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn noop_backend_validates_bodycam_post_process_pipeline() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
            label: Some("bodycam post validation device"),
            ..Default::default()
        });
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut post =
            BodycamPostProcess::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb, 320, 180);
        let rig = BodycamRig::default();
        post.update_from_rig(&queue, &rig, 1);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("out srgb"),
            size: wgpu::Extent3d {
                width: 320,
                height: 180,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        post.encode(&mut encoder, &out_view);
        queue.submit([encoder.finish()]);
        let err = pollster::block_on(scope.pop());
        assert!(err.is_none(), "{err:?}");
    }

    #[test]
    fn camcorder_servo_zoom_narrows_fov_and_updates_osd_uniform() {
        let mut rig = BodycamRig::default();
        rig.set_lens_profile(PhotorealLensProfile::Camcorder);
        let (cam_1x, _, _) = rig.camera_and_flashlight(Vec3::ZERO);

        rig.apply_servo_zoom(4.0);
        for _ in 0..60 {
            rig.step(1.0 / 60.0, 1.0, true, false, 0.35);
        }
        assert!(rig.camcorder_zoom > 1.5);
        let (cam_zoomed, _, _) = rig.camera_and_flashlight(Vec3::ZERO);
        assert!(cam_zoomed.vertical_fov_radians < cam_1x.vertical_fov_radians);
    }
}

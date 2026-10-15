# Facial rendering source scan: sean-reid/tongue-simulator

Repository: https://github.com/sean-reid/tongue-simulator

Pinned source revision: 6c63de6f7bdef94c47f85301787d98de9ff23ee7

Discovery topic: tongue animation

Description: Physics-simulated tongue animation that speaks your text in the browser.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"tongue": ["crates/tongue-sim/Cargo.toml", "crates/tongue-sim/data/cmudict_compact.json", "crates/tongue-sim/data/phoneme_targets.json", "crates/tongue-sim/src/airflow/mod.rs", "crates/tongue-sim/src/airflow/particles.rs", "crates/tongue-sim/src/anatomy/defaults.rs", "crates/tongue-sim/src/anatomy/mod.rs", "crates/tongue-sim/src/api.rs", "crates/tongue-sim/src/g2p/cmudict.rs", "crates/tongue-sim/src/g2p/fallback.rs", "crates/tongue-sim/src/g2p/mod.rs", "crates/tongue-sim/src/gestures/blending.rs", "crates/tongue-sim/src/gestures/mod.rs", "crates/tongue-sim/src/gestures/tiers.rs", "crates/tongue-sim/src/gestures/timeline.rs", "crates/tongue-sim/src/lib.rs", "crates/tongue-sim/src/phoneme/durations.rs", "crates/tongue-sim/src/phoneme/mod.rs", "crates/tongue-sim/src/phoneme/targets.rs", "crates/tongue-sim/src/physics/collisions.rs", "crates/tongue-sim/src/physics/mesh.rs", "crates/tongue-sim/src/physics/mod.rs", "crates/tongue-sim/src/physics/muscles.rs", "crates/tongue-sim/src/physics/pbd.rs", "crates/tongue-sim/src/physics/rigid_bodies.rs", "crates/tongue-sim/tests/g2p_tests.rs", "crates/tongue-sim/tests/physics_tests.rs", "web/src/renderer/drawTongue.ts"], "rig": ["crates/tongue-sim/src/physics/rigid_bodies.rs"]}

## Source file: crates/tongue-sim/src/physics/rigid_bodies.rs

{
  "path": "crates/tongue-sim/src/physics/rigid_bodies.rs",
  "blob_sha": "5093722cbeb2a802993d2e2dd7415509d86b6c8c",
  "sha256": "3bd36f6c7abfbc68d3af6f64adff65af6f71b91ae5318335cf223657e3f25b54",
  "lines": 144,
  "bytes": 4639,
  "selection_score": 18,
  "keyword_line_counts": {
    "jaw": 11,
    "tongue": 1,
    "rig": 6,
    "lip": 34
  },
  "url": "https://github.com/sean-reid/tongue-simulator/blob/6c63de6f7bdef94c47f85301787d98de9ff23ee7/crates/tongue-sim/src/physics/rigid_bodies.rs",
  "function_signature_candidates": [
    {
      "line": 34,
      "text": "pub fn new(anatomy: &AnatomyConfig) -> Self {"
    },
    {
      "line": 53,
      "text": "pub fn step(&mut self, target: &ArticulatoryState, dt: f64) {"
    },
    {
      "line": 86,
      "text": "pub fn upper_lip_points(&self, anatomy: &AnatomyConfig) -> Vec<Vec2> {"
    },
    {
      "line": 101,
      "text": "pub fn lower_lip_points(&self, anatomy: &AnatomyConfig) -> Vec<Vec2> {"
    },
    {
      "line": 116,
      "text": "pub fn velum_tip(&self, anatomy: &AnatomyConfig) -> Vec2 {"
    },
    {
      "line": 127,
      "text": "fn pd_step("
    }
  ]
}

```text

use crate::anatomy::{AnatomyConfig, Vec2};
use crate::phoneme::ArticulatoryState;

/// Rigid body state for all non-tongue articulators.
pub struct RigidBodies {
    pub jaw_angle: f64,        // degrees
    pub jaw_velocity: f64,     // deg/s
    pub velum_angle: f64,      // degrees
    pub velum_velocity: f64,
    pub hyoid_y: f64,          // mm offset from rest
    pub hyoid_vy: f64,
    pub lip_protrusion: f64,
    pub lip_rounding: f64,
    pub lip_spread: f64,
    pub lip_prot_vel: f64,
    pub lip_round_vel: f64,
    pub lip_spread_vel: f64,

    // Rest positions from anatomy
    pub hyoid_rest_y: f64,
}

// PD controller gains (from architecture §13.1)
const KP_JAW: f64 = 100.0;
const KD_JAW: f64 = 15.0;
const KP_VELUM: f64 = 80.0;
const KD_VELUM: f64 = 12.0;
const KP_HYOID: f64 = 120.0;
const KD_HYOID: f64 = 18.0;
const KP_LIP: f64 = 150.0;
const KD_LIP: f64 = 22.0;

impl RigidBodies {
    pub fn new(anatomy: &AnatomyConfig) -> Self {
        RigidBodies {
            jaw_angle: 8.0,
            jaw_velocity: 0.0,
            velum_angle: 2.0,
            velum_velocity: 0.0,
            hyoid_y: 0.0,
            hyoid_vy: 0.0,
            lip_protrusion: 0.0,
            lip_rounding: 0.0,
            lip_spread: 0.0,
            lip_prot_vel: 0.0,
            lip_round_vel: 0.0,
            lip_spread_vel: 0.0,
            hyoid_rest_y: anatomy.hyoid_position.y,
        }
    }

    /// Advance all rigid bodies by dt seconds using PD control.
    pub fn step(&mut self, target: &ArticulatoryState, dt: f64) {
        self.jaw_angle = pd_step(
            self.jaw_angle, self.jaw_velocity, target.jaw_angle,
            KP_JAW, KD_JAW, dt, 0.0, 35.0, &mut self.jaw_velocity,
        );

        self.velum_angle = pd_step(
            self.velum_angle, self.velum_velocity, target.velum_angle,
            KP_VELUM, KD_VELUM, dt, 0.0, 45.0, &mut self.velum_velocity,
        );

        self.hyoid_y = pd_step(
            self.hyoid_y, self.hyoid_vy, target.hyoid_offset,
            KP_HYOID, KD_HYOID, dt, -6.0, 4.0, &mut self.hyoid_vy,
        );

        self.lip_protrusion = pd_step(
            self.lip_protrusion, self.lip_prot_vel, target.lip_protrusion,
            KP_LIP, KD_LIP, dt, -1.0, 1.0, &mut self.lip_prot_vel,
        );

        self.lip_rounding = pd_step(
            self.lip_rounding, self.lip_round_vel, target.lip_rounding,
            KP_LIP, KD_LIP, dt, 0.0, 1.0, &mut self.lip_round_vel,
        );

        self.lip_spread = pd_step(
            self.lip_spread, self.lip_spread_vel, target.lip_spread,
            KP_LIP, KD_LIP, dt, 0.0, 1.0, &mut self.lip_spread_vel,
        );
    }

    /// Compute the current upper lip curve (5 points) based on rigid body state.
    pub fn upper_lip_points(&self, anatomy: &AnatomyConfig) -> Vec<Vec2> {
        let base = &anatomy.upper_lip;
        let prot = self.lip_protrusion;  // -1..1
        let round = self.lip_rounding;   // 0..1
        let spread = self.lip_spread;    // 0..1

        base.iter().map(|p| {
            Vec2::new(
                p.x,
                p.y + round * 2.0 - spread * 2.0,
            )
        }).collect()
    }

    /// Compute the current lower lip curve (5 points).
    pub fn lower_lip_points(&self, anatomy: &AnatomyConfig) -> Vec<Vec2> {
        let base = &anatomy.lower_lip;
        let jaw_offset_y = self.jaw_angle * (-0.4); // jaw opening lowers lower lip
        let round = self.lip_rounding;

        base.iter().map(|p| {
            Vec2::new(
                p.x,
                p.y + jaw_offset_y + round * (-1.0),
            )
        }).collect()
    }

    /// Compute the velum tip position given current velum_angle.
    /// Hinge is at anatomy.velum_hinge, velum hangs downward.
    pub fn velum_tip(&self, anatomy: &AnatomyConfig) -> Vec2 {
        let hinge = &anatomy.velum_hinge;
        let angle_rad = self.velum_angle.to_radians();
        // Velum hangs down at angle from vertical; 0° = vertical (raised), 45° = swung out
        let x = hinge.x + anatomy.velum_length * angle_rad.sin();
        let y = hinge.y - anatomy.velum_length * angle_rad.cos();
        Vec2::new(x, y)
    }
}

/// Simple PD controller step for a scalar DOF.
fn pd_step(
    current: f64,
    velocity: f64,
    target: f64,
    kp: f64,
    kd: f64,
    dt: f64,
    min: f64,
    max: f64,
    vel_out: &mut f64,
) -> f64 {
    let error = target - current;
    let accel = kp * error - kd * velocity;
    let new_vel = (velocity + accel * dt).clamp(-200.0, 200.0);
    let new_pos = (current + new_vel * dt).clamp(min, max);
    *vel_out = new_vel;
    new_pos
}


```

## Source file: web/src/renderer/drawTongue.ts

{
  "path": "web/src/renderer/drawTongue.ts",
  "blob_sha": "2aaa3d0919fd6e96658384f25930493a7d391aef",
  "sha256": "63fb6438b700c6e7de4810b2396125a17f9fcdc6a56310ddd2ddf4196a903db2",
  "lines": 76,
  "bytes": 2165,
  "selection_score": 15,
  "keyword_line_counts": {
    "tongue": 13,
    "face": 1
  },
  "url": "https://github.com/sean-reid/tongue-simulator/blob/6c63de6f7bdef94c47f85301787d98de9ff23ee7/web/src/renderer/drawTongue.ts",
  "function_signature_candidates": []
}

```text

/// Draws the tongue using Catmull-Rom spline smoothing.

import { COLORS } from './colors';
import { catmullRomSpline } from '../utils/spline';
import { buildTransform } from './drawTract';
import type { RenderState } from '../types/simulation';

export function drawTongue(
  ctx: CanvasRenderingContext2D,
  state: RenderState,
  cw: number,
  ch: number
) {
  const { toCanvas, scale } = buildTransform(cw, ch);

  if (state.tongue_dorsal.length < 2 || state.tongue_ventral.length < 2) return;

  // Convert from VTC coords to canvas coords
  const dorsal: [number, number][] = state.tongue_dorsal.map(
    ([x, y]) => toCanvas(x, y)
  );
  const ventral: [number, number][] = state.tongue_ventral.map(
    ([x, y]) => toCanvas(x, y)
  );

  // Smooth the dorsal and ventral curves
  const smoothDorsal = catmullRomSpline(dorsal, 6);
  const smoothVentral = catmullRomSpline(ventral, 6);

  // Build tongue outline: dorsal (root → tip) + ventral (tip → root)
  const tonguePath: [number, number][] = [
    ...smoothDorsal,
    ...smoothVentral.slice().reverse(),
  ];

  // Fill tongue body
  ctx.save();
  ctx.beginPath();
  if (tonguePath.length > 0) {
    ctx.moveTo(tonguePath[0][0], tonguePath[0][1]);
    for (let i = 1; i < tonguePath.length; i++) {
      ctx.lineTo(tonguePath[i][0], tonguePath[i][1]);
    }
    ctx.closePath();
  }

  // Flat fill — no gradient
  ctx.fillStyle = COLORS.tongue;
  ctx.fill();

  // Dorsal outline
  ctx.beginPath();
  if (smoothDorsal.length > 0) {
    ctx.moveTo(smoothDorsal[0][0], smoothDorsal[0][1]);
    for (let i = 1; i < smoothDorsal.length; i++) {
      ctx.lineTo(smoothDorsal[i][0], smoothDorsal[i][1]);
    }
  }
  ctx.strokeStyle = 'rgba(80, 30, 30, 0.75)';
  ctx.lineWidth = 1.5 * scale;
  ctx.stroke();

  // Ventral outline (lighter — interior surface)
  ctx.beginPath();
  if (smoothVentral.length > 0) {
    ctx.moveTo(smoothVentral[0][0], smoothVentral[0][1]);
    for (let i = 1; i < smoothVentral.length; i++) {
      ctx.lineTo(smoothVentral[i][0], smoothVentral[i][1]);
    }
  }
  ctx.strokeStyle = 'rgba(120, 55, 55, 0.4)';
  ctx.lineWidth = scale;
  ctx.stroke();

  ctx.restore();
}


```

## Source file: crates/tongue-sim/src/api.rs

{
  "path": "crates/tongue-sim/src/api.rs",
  "blob_sha": "9d81020d44ffaa3d43ceec9532164c33a128266f",
  "sha256": "acc4c4c5e628608878e560c5327e3fcfcdb7030c13837bee7f7ca0aa150d809f",
  "lines": 312,
  "bytes": 10959,
  "selection_score": 3,
  "keyword_line_counts": {
    "jaw": 10,
    "tongue": 20,
    "rig": 25,
    "lip": 30
  },
  "url": "https://github.com/sean-reid/tongue-simulator/blob/6c63de6f7bdef94c47f85301787d98de9ff23ee7/crates/tongue-sim/src/api.rs",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "pub fn build_render_state("
    },
    {
      "line": 71,
      "text": "fn snapshot_to_render(snap: &Snapshot, anatomy: &AnatomyConfig) -> RenderState {"
    },
    {
      "line": 125,
      "text": "pub fn new(text: &str, speaking_rate: f64) -> SimulationSession {"
    },
    {
      "line": 142,
      "text": "pub fn precompute(&mut self) -> f64 {"
    },
    {
      "line": 188,
      "text": "pub fn get_render_state(&self, time_ms: f64) -> JsValue {"
    },
    {
      "line": 227,
      "text": "pub fn get_phoneme_timeline(&self) -> JsValue {"
    },
    {
      "line": 232,
      "text": "pub fn get_word_sync_map(&self) -> JsValue {"
    },
    {
      "line": 238,
      "text": "pub fn set_speaking_rate(&mut self, rate: f64) {"
    },
    {
      "line": 247,
      "text": "pub fn duration_ms(&self) -> f64 {"
    },
    {
      "line": 253,
      "text": "fn interpolate_snapshots(a: &Snapshot, b: &Snapshot, t: f64, anatomy: &AnatomyConfig) -> RenderState {"
    }
  ]
}

```text

use wasm_bindgen::prelude::*;
use serde::Serialize;

use crate::anatomy::AnatomyConfig;
use crate::anatomy::defaults::default_anatomy;
use crate::g2p;
use crate::gestures::GesturalScore;
use crate::phoneme::{ArticulatoryState, PhonemeTimeline};
use crate::physics::{PhysicsEngine, Snapshot};
use crate::physics::rigid_bodies::RigidBodies;
use crate::physics::mesh::TongueMesh;

/// Serializable render state sent across the WASM ↔ JS boundary.
#[derive(Clone, Serialize)]
pub struct RenderState {
    pub tongue_dorsal: Vec<[f64; 2]>,
    pub tongue_ventral: Vec<[f64; 2]>,
    pub jaw_angle: f64,
    pub upper_lip: Vec<[f64; 2]>,
    pub lower_lip: Vec<[f64; 2]>,
    pub velum_angle: f64,
    pub velum_tip: [f64; 2],
    pub hyoid_y: f64,
    pub glottal_aperture: f64,
    pub voicing: f64,
    pub current_phoneme_ipa: String,
    pub current_phoneme_index: usize,
    pub is_turbulent: bool,
    pub min_area: f64,
}

/// Build a RenderState from current mesh/rigid state.
pub fn build_render_state(
    mesh: &TongueMesh,
    rigid: &RigidBodies,
    articulatory: &ArticulatoryState,
    ipa: &str,
) -> RenderState {
    let anatomy = default_anatomy();

    let tongue_dorsal: Vec<[f64; 2]> = mesh.dorsal.iter()
        .map(|n| [n.position.x, n.position.y])
        .collect();

    let tongue_ventral: Vec<[f64; 2]> = mesh.ventral.iter()
        .map(|n| [n.position.x, n.position.y])
        .collect();

    let upper_lip = rigid.upper_lip_points(&anatomy);
    let lower_lip = rigid.lower_lip_points(&anatomy);
    let velum_tip = rigid.velum_tip(&anatomy);

    RenderState {
        tongue_dorsal,
        tongue_ventral,
        jaw_angle: rigid.jaw_angle,
        upper_lip: upper_lip.iter().map(|p| [p.x, p.y]).collect(),
        lower_lip: lower_lip.iter().map(|p| [p.x, p.y]).collect(),
        velum_angle: rigid.velum_angle,
        velum_tip: [velum_tip.x, velum_tip.y],
        hyoid_y: rigid.hyoid_y,
        glottal_aperture: articulatory.glottal_aperture,
        voicing: articulatory.voicing,
        current_phoneme_ipa: ipa.to_string(),
        current_phoneme_index: 0,
        is_turbulent: false,
        min_area: 1.0,
    }
}

fn snapshot_to_render(snap: &Snapshot, anatomy: &AnatomyConfig) -> RenderState {
    let rigid = RigidBodies {
        jaw_angle: snap.jaw_angle,
        jaw_velocity: 0.0,
        velum_angle: snap.velum_angle,
        velum_velocity: 0.0,
        hyoid_y: snap.hyoid_y,
        hyoid_vy: 0.0,
        lip_protrusion: snap.lip_protrusion,
        lip_rounding: snap.lip_rounding,
        lip_spread: 0.0,
        lip_prot_vel: 0.0,
        lip_round_vel: 0.0,
        lip_spread_vel: 0.0,
        hyoid_rest_y: anatomy.hyoid_position.y,
    };

    let upper_lip = rigid.upper_lip_points(anatomy);
    let lower_lip = rigid.lower_lip_points(anatomy);
    let velum_tip = rigid.velum_tip(anatomy);

    RenderState {
        tongue_dorsal: snap.tongue_dorsal.iter().map(|p| [p.x, p.y]).collect(),
        tongue_ventral: snap.tongue_ventral.iter().map(|p| [p.x, p.y]).collect(),
        jaw_angle: snap.jaw_angle,
        upper_lip: upper_lip.iter().map(|p| [p.x, p.y]).collect(),
        lower_lip: lower_lip.iter().map(|p| [p.x, p.y]).collect(),
        velum_angle: snap.velum_angle,
        velum_tip: [velum_tip.x, velum_tip.y],
        hyoid_y: snap.hyoid_y,
        glottal_aperture: snap.glottal_aperture,
        voicing: snap.voicing,
        current_phoneme_ipa: snap.phoneme_ipa.clone(),
        current_phoneme_index: snap.phoneme_index,
        is_turbulent: false,
        min_area: 1.0,
    }
}

/// The primary simulation session exposed to JavaScript.
#[wasm_bindgen]
pub struct SimulationSession {
    text: String,
    speaking_rate: f64,
    timeline: PhonemeTimeline,
    gestural_score: GesturalScore,
    snapshots: Vec<Snapshot>,
    anatomy: AnatomyConfig,
}

#[wasm_bindgen]
impl SimulationSession {
    /// Create a new simulation session for the given text and speaking rate.
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, speaking_rate: f64) -> SimulationSession {
        let rate = speaking_rate.clamp(0.1, 4.0);
        let timeline = g2p::build_timeline(text, rate);
        let gestural_score = GesturalScore::new(timeline.clone(), rate);
        let anatomy = default_anatomy();
        SimulationSession {
            text: text.to_string(),
            speaking_rate: rate,
            timeline,
            gestural_score,
            snapshots: Vec::new(),
            anatomy,
        }
    }

    /// Pre-compute physics for the entire utterance.
    /// Returns total duration in ms.
    pub fn precompute(&mut self) -> f64 {
        let dt = 1.0 / 240.0; // 240 Hz physics
        let total_ms = self.timeline.total_duration_ms;

        // Add some padding for return-to-rest
        let sim_duration_ms = total_ms + 500.0;
        let n_frames = (sim_duration_ms / 1000.0 * 240.0) as usize;

        let mut engine = PhysicsEngine::new(&self.anatomy);
        self.snapshots = Vec::with_capacity(n_frames);

        // Warm up the engine for 50 steps at rest position
        let rest = ArticulatoryState::rest();
        for _ in 0..50 {
            engine.step(&rest, dt);
        }

        for frame in 0..n_frames {
            let t_ms = frame as f64 / 240.0 * 1000.0;
            let articulatory = self.gestural_score.get_state_at(t_ms);
            let (ipa, pidx) = self.gestural_score.get_phoneme_info(t_ms);

            engine.step(&articulatory, dt);

            let snap = Snapshot {
                time_ms: t_ms,
                tongue_dorsal: engine.mesh.dorsal.iter().map(|n| n.position.clone()).collect(),
                tongue_ventral: engine.mesh.ventral.iter().map(|n| n.position.clone()).collect(),
                jaw_angle: engine.rigid.jaw_angle,
                hyoid_y: engine.rigid.hyoid_y,
                velum_angle: engine.rigid.velum_angle,
                glottal_aperture: articulatory.glottal_aperture,
                voicing: articulatory.voicing,
                lip_protrusion: engine.rigid.lip_protrusion,
                lip_rounding: engine.rigid.lip_rounding,
                phoneme_ipa: ipa,
                phoneme_index: pidx,
            };
            self.snapshots.push(snap);
        }

        total_ms
    }

    /// Get the render state at a given simulation time (ms).
    /// Interpolates between nearest precomputed snapshots.
    pub fn get_render_state(&self, time_ms: f64) -> JsValue {
        if self.snapshots.is_empty() {
            let rest = ArticulatoryState::rest();
            let anatomy = default_anatomy();
            let mesh = TongueMesh::new(&anatomy);
            let rigid = RigidBodies::new(&anatomy);
            let rs = build_render_state(&mesh, &rigid, &rest, "");
            return serde_wasm_bindgen::to_value(&rs).unwrap_or(JsValue::NULL);
        }

        // Find bracketing snapshots
        let n = self.snapshots.len();
        let idx = {
            let mut lo = 0usize;
            let mut hi = n;
            while lo + 1 < hi {
                let mid = (lo + hi) / 2;
                if self.snapshots[mid].time_ms <= time_ms { lo = mid; } else { hi = mid; }
            }
            lo.min(n - 1)
        };

        let rs = if idx + 1 < n {
            let a = &self.snapshots[idx];
            let b = &self.snapshots[idx + 1];
            let t = if (b.time_ms - a.time_ms).abs() < 1e-6 {
                0.0
            } else {
                (time_ms - a.time_ms) / (b.time_ms - a.time_ms)
            };
            interpolate_snapshots(a, b, t, &self.anatomy)
        } else {
            snapshot_to_render(&self.snapshots[idx], &self.anatomy)
        };

        serde_wasm_bindgen::to_value(&rs).unwrap_or(JsValue::NULL)
    }

    /// Get the phoneme timeline as a JS array.
    pub fn get_phoneme_timeline(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.timeline).unwrap_or(JsValue::NULL)
    }

    /// Get word boundary sync map.
    pub fn get_word_sync_map(&self) -> JsValue {
        let sync_map = g2p::build_word_sync_map(&self.text, &self.timeline);
        serde_wasm_bindgen::to_value(&sync_map).unwrap_or(JsValue::NULL)
    }

    /// Rebuild timeline at a new speaking rate.
    pub fn set_speaking_rate(&mut self, rate: f64) {
        let rate = rate.clamp(0.1, 4.0);
        self.speaking_rate = rate;
        self.timeline = g2p::build_timeline(&self.text, rate);
        self.gestural_score = GesturalScore::new(self.timeline.clone(), rate);
        self.snapshots.clear();
    }

    /// Total simulation duration in ms.
    pub fn duration_ms(&self) -> f64 {
        self.timeline.total_duration_ms
    }
}

/// Linearly interpolate between two snapshots.
fn interpolate_snapshots(a: &Snapshot, b: &Snapshot, t: f64, anatomy: &AnatomyConfig) -> RenderState {
    let n = a.tongue_dorsal.len();
    let dorsal: Vec<[f64; 2]> = (0..n).map(|i| {
        let pa = &a.tongue_dorsal[i];
        let pb = &b.tongue_dorsal[i];
        [pa.x + (pb.x - pa.x) * t, pa.y + (pb.y - pa.y) * t]
    }).collect();

    let ventral: Vec<[f64; 2]> = (0..n).map(|i| {
        let pa = &a.tongue_ventral[i];
        let pb = &b.tongue_ventral[i];
        [pa.x + (pb.x - pa.x) * t, pa.y + (pb.y - pa.y) * t]
    }).collect();

    // Use 'a' snapshot's rigid state interpolated
    let jaw = a.jaw_angle + (b.jaw_angle - a.jaw_angle) * t;
    let velum_a = a.velum_angle + (b.velum_angle - a.velum_angle) * t;
    let hyoid = a.hyoid_y + (b.hyoid_y - a.hyoid_y) * t;
    let lip_prot = a.lip_protrusion + (b.lip_protrusion - a.lip_protrusion) * t;
    let lip_round = a.lip_rounding + (b.lip_rounding - a.lip_rounding) * t;
    let glottal = a.glottal_aperture + (b.glottal_aperture - a.glottal_aperture) * t;
    let voicing = a.voicing + (b.voicing - a.voicing) * t;

    let rigid = RigidBodies {
        jaw_angle: jaw,
        jaw_velocity: 0.0,
        velum_angle: velum_a,
        velum_velocity: 0.0,
        hyoid_y: hyoid,
        hyoid_vy: 0.0,
        lip_protrusion: lip_prot,
        lip_rounding: lip_round,
        lip_spread: 0.0,
        lip_prot_vel: 0.0,
        lip_round_vel: 0.0,
        lip_spread_vel: 0.0,
        hyoid_rest_y: anatomy.hyoid_position.y,
    };

    let upper_lip = rigid.upper_lip_points(anatomy);
    let lower_lip = rigid.lower_lip_points(anatomy);
    let velum_tip = rigid.velum_tip(anatomy);

    RenderState {
        tongue_dorsal: dorsal,
        tongue_ventral: ventral,
        jaw_angle: jaw,
        upper_lip: upper_lip.iter().map(|p| [p.x, p.y]).collect(),
        lower_lip: lower_lip.iter().map(|p| [p.x, p.y]).collect(),
        velum_angle: velum_a,
        velum_tip: [velum_tip.x, velum_tip.y],
        hyoid_y: hyoid,
        glottal_aperture: glottal,
        voicing,
        current_phoneme_ipa: a.phoneme_ipa.clone(),
        current_phoneme_index: a.phoneme_index,
        is_turbulent: false,
        min_area: 1.0,
    }
}


```

## Source file: crates/tongue-sim/src/lib.rs

{
  "path": "crates/tongue-sim/src/lib.rs",
  "blob_sha": "83829f828a1b57736b932c848ef9b6784e5e6ec3",
  "sha256": "0fd20325ecd28d2096801aaad1c1d221f00a8e86a9ebe628b4ffe6d323991f7c",
  "lines": 56,
  "bytes": 1815,
  "selection_score": 3,
  "keyword_line_counts": {
    "tongue": 1,
    "rig": 3
  },
  "url": "https://github.com/sean-reid/tongue-simulator/blob/6c63de6f7bdef94c47f85301787d98de9ff23ee7/crates/tongue-sim/src/lib.rs",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "pub fn start() {"
    },
    {
      "line": 20,
      "text": "pub fn get_phoneme_list() -> JsValue {"
    },
    {
      "line": 34,
      "text": "pub fn get_default_anatomy() -> JsValue {"
    },
    {
      "line": 41,
      "text": "pub fn solve_static_posture(arpabet: &str) -> JsValue {"
    }
  ]
}

```text

use wasm_bindgen::prelude::*;

pub mod anatomy;
pub mod api;
pub mod g2p;
pub mod gestures;
pub mod phoneme;
pub mod physics;

pub use api::SimulationSession;

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Returns the list of all supported ARPAbet phonemes as a JS array of strings.
#[wasm_bindgen]
pub fn get_phoneme_list() -> JsValue {
    let list: Vec<&str> = vec![
        "AA", "AE", "AH", "AO", "AW", "AY",
        "B", "CH", "D", "DH", "EH", "ER", "EY",
        "F", "G", "HH", "IH", "IY", "JH", "K",
        "L", "M", "N", "NG", "OW", "OY", "P",
        "R", "S", "SH", "T", "TH", "UH", "UW",
        "V", "W", "Y", "Z", "ZH",
    ];
    serde_wasm_bindgen::to_value(&list).unwrap_or(JsValue::NULL)
}

/// Returns the default anatomy configuration as a JS object.
#[wasm_bindgen]
pub fn get_default_anatomy() -> JsValue {
    let cfg = anatomy::defaults::default_anatomy();
    serde_wasm_bindgen::to_value(&cfg).unwrap_or(JsValue::NULL)
}

/// Solve the static posture for a single ARPAbet phoneme and return the render state.
#[wasm_bindgen]
pub fn solve_static_posture(arpabet: &str) -> JsValue {
    let target = phoneme::targets::get_target(arpabet);
    let anat = anatomy::defaults::default_anatomy();
    let mut mesh = physics::mesh::TongueMesh::new(&anat);
    let mut rigid = physics::rigid_bodies::RigidBodies::new(&anat);

    // Run physics to convergence (~150 steps at 240 Hz)
    for _ in 0..150 {
        let dt = 1.0 / 240.0;
        physics::pbd::step_mesh(&mut mesh, &target, &anat, dt);
        rigid.step(&target, dt);
    }

    let render = api::build_render_state(&mesh, &rigid, &target, phoneme::arpabet_to_ipa(arpabet));
    serde_wasm_bindgen::to_value(&render).unwrap_or(JsValue::NULL)
}


```

//! Authored finite translating body; scene pose is published by the liquid owner.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidBody {
    pub mass_kg: f64,
    pub initial_velocity_m_s: [f64; 3],
}

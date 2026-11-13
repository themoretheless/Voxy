//! `SceneDemo` adapters: each demo with the GPU geometry, window title, input
//! and smoke checks the scene shell needs to present it.
mod fem;
mod gravity;
mod liquids;
mod strands;
mod tissues;
mod wear;
mod xray;

pub use fem::FemScene;
pub use gravity::GravityScene;
pub use liquids::LiquidScene;
pub use strands::StrandsScene;
pub use tissues::TissueScene;
pub use wear::WearScene;
pub use xray::XrayScene;

use voxy_scene_host::{DemoInput, DemoKey, DemoTick};

/// Step of the liquid, wear and fracture scenes: fixed under smoke, clamped wall time otherwise.
fn fixed_smoke_step(tick: DemoTick) -> f64 {
    if tick.smoke {
        1.0 / 60.0
    } else {
        f64::from(tick.dt)
    }
}

/// The key of a fresh (non-repeated) key press.
fn pressed_key(input: DemoInput) -> Option<DemoKey> {
    match input {
        DemoInput::Key {
            key,
            pressed: true,
            repeat: false,
        } => Some(key),
        _ => None,
    }
}

//! Soft-tissue biomechanics, skin deformation, secondary motion and physiology.

pub mod biomechanics;
/// Closed-loop blood-volume, chamber and valve mechanics in SI units.
pub mod circulation;
/// Conservative plasma/interstitial/lymph fluid and protein exchange.
pub mod lymph;
mod osmotic;
/// Airway volume-flow and compliant lung-unit mechanics in SI units.
pub mod respiration;
pub mod secondary_motion;
pub mod skin;
mod spatial_bounds;
pub mod tissue;
pub mod tissue_surface;

pub use physics::{heat_exchange, math, positive_transport, strand, surface_film, triangle_index};

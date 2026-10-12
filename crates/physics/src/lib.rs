//! Geometry-independent mechanics. No renderer, voxel world or ECS dependencies.
//!
//! Module map (plan step B3). Paths are stable; the groups below are the
//! responsibility boundaries the crate graph test and the refactoring plan
//! refer to:
//!
//! - **Shared math**: [`math`] (`[f64; 3]` helpers every domain uses).
//! - **Rigid bodies and contact**: [`contact`], [`rigid_motion`], [`spin_path`],
//!   [`mass_properties`], [`friction`], [`cohesive`], [`planar`], the character
//!   controller re-exported at the root, and the [`CollisionWorld`] port.
//! - **Gravity and astrophysics**: [`gravity`] (field, spheres, character) and
//!   [`astrophysics`] (binary, spin, equation of state, nuclear, radiation,
//!   spherical models); the flat `astrophysics_*`/`gravity_*` names are kept as
//!   re-exports.
//! - **Deformable tissue and biomechanics**: [`tissue`], [`tissue_surface`],
//!   [`soft_body`], [`biomechanics`], [`plasticity`], [`skin`], [`strand`],
//!   [`hair`], [`secondary_motion`].
//! - **Fluids and films**: [`liquid`], [`surface_film`], [`suspension`],
//!   [`moisture`], [`wear`].
//! - **Physiology**: [`respiration`], [`circulation`], [`lymph`].
//!
//! Private helpers (`symmetric_eigen`, `triangle_index`, `spatial_bounds`,
//! `positive_transport`, `heat_exchange`, contact glue) stay crate-internal.
pub mod astrophysics;
pub use astrophysics::atmosphere as astrophysics_atmosphere;
pub use astrophysics::binary as astrophysics_binary;
pub use astrophysics::column as astrophysics_column;
pub use astrophysics::eos as astrophysics_eos;
pub use astrophysics::equilibrium as astrophysics_equilibrium;
pub use astrophysics::evolution as astrophysics_evolution;
pub use astrophysics::gas as astrophysics_gas;
pub use astrophysics::gas_gravity as astrophysics_gas_gravity;
pub use astrophysics::nuclear as astrophysics_nuclear;
pub use astrophysics::opacity as astrophysics_opacity;
pub use astrophysics::radhydro as astrophysics_radhydro;
pub use astrophysics::radiation as astrophysics_radiation;
pub use astrophysics::reaclib as astrophysics_reaclib;
pub use astrophysics::spherical as astrophysics_spherical;
pub use astrophysics::spherical_radiation as astrophysics_spherical_radiation;
pub use astrophysics::spin as astrophysics_spin;
pub use astrophysics::star as astrophysics_star;
pub use astrophysics::thermal as astrophysics_thermal;
mod character;
pub mod contact;
pub mod gravity;
pub use gravity::character as gravity_character;
pub use gravity::field as gravity_field;
pub use gravity::spheres as gravity_spheres;
pub mod mass_properties;
pub mod moisture;
pub mod planar;
pub mod rigid_motion;
pub mod spin_path;
pub mod strand;
mod symmetric_eigen;
pub mod wear;
pub use character::*;

/// Integer origin plus local floating-point bounds preserve precision in large worlds.
/// Units are chosen by the application, not tied to a voxel size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Origin {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredAabb {
    pub anchor: Origin,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepResult<O> {
    pub fraction: f64,
    /// Axis-aligned unit normal; zero denotes initial overlap.
    pub normal: [i8; 3],
    pub obstacle: Option<O>,
}

/// Backend contract for continuous AABB collision queries.
/// Return the earliest hit, fraction in [0,1], in the body's local frame.
/// Missing geometry must be represented as an obstacle or an error, never as empty space.
pub trait CollisionWorld {
    type Obstacle;
    type Error;
    /// # Errors
    /// Backend-specific failures, including exhausted query budgets.
    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<SweepResult<Self::Obstacle>, Self::Error>;
}

/// Sweeps against a static box in the same local frame (the anchor is not used).
/// Bounds and displacement must be finite, and each minimum must be below its maximum.
/// Equal contact times prefer X, then Y, then Z. Initial overlaps return a zero normal.
#[must_use]
pub fn sweep_box(
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
) -> Option<(f64, [i8; 3])> {
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut normal = [0_i8; 3];
    for (axis, &axis_displacement) in displacement.iter().enumerate() {
        let voxel_min = min[axis];
        let voxel_max = max[axis];
        let velocity = axis_displacement;
        if velocity == 0.0 {
            if aabb.max[axis] <= voxel_min || aabb.min[axis] >= voxel_max {
                return None;
            }
            continue;
        }
        let first = (voxel_min - aabb.max[axis]) / velocity;
        let second = (voxel_max - aabb.min[axis]) / velocity;
        let axis_enter = first.min(second);
        let axis_exit = first.max(second);
        if axis_enter > enter {
            enter = axis_enter;
            normal = [0; 3];
            normal[axis] = if velocity > 0.0 { -1 } else { 1 };
        }
        exit = exit.min(axis_exit);
        if enter > exit {
            return None;
        }
    }
    if exit < 0.0 || enter > 1.0 {
        None
    } else if enter < 0.0 {
        Some((0.0, [0; 3]))
    } else {
        Some((enter, normal))
    }
}

pub mod liquid;

pub mod soft_body;

pub mod tissue;

pub mod skin;

pub mod biomechanics;

pub mod cohesive;
pub mod friction;
pub mod plasticity;

pub mod tissue_surface;

pub mod hair;

/// Airway volume-flow and compliant lung-unit mechanics in SI units.
pub mod respiration;

pub mod secondary_motion;

/// Closed-loop blood-volume, chamber and valve mechanics in SI units.
pub mod circulation;

/// Conservative plasma/interstitial/lymph fluid and protein exchange.
pub mod lymph;

mod tissue_contact;

pub mod surface_film;

mod surface_film_contact;
mod triangle_index;

pub mod suspension;

mod heat_exchange;

mod spatial_bounds;

mod positive_transport;

pub mod math;

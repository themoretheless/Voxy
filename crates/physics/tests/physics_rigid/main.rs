//! Physics integration tests: rigid bodies, normal contact and rotation.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_rigid <module>::` runs it alone.

#[path = "../support/rigid.rs"]
mod support_rigid;

mod boxes;
mod contact_network;
mod gravity;
mod gravity_character;
mod gravity_field;
mod gravity_spheres;
mod mass_properties;
mod material_pair;
mod normal_manifold;
mod normal_reactions;
mod reaction_rates;
mod rigid_motion;
mod rigid_point_impulse;
mod rigid_world_forces;
mod simultaneous_rigid_events;
mod spin_path;

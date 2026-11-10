//! Physics integration tests: plasticity, cohesion, friction, wear, moisture and debris.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_solid <module>::` runs it alone.

#[path = "../support/cohesive.rs"]
mod support_cohesive;

mod beam_accuracy;
mod cohesive;
mod cohesive_mesh;
mod cohesive_topology;
mod debris_release;
mod fragment_impact;
mod fragment_sweep;
mod friction;
mod friction_mesh;
mod friction_slider;
mod moisture;
mod moisture_film;
mod moisture_partition;
mod moisture_sparse;
mod moisture_supply;
mod moisture_vapor;
mod plastic_mesh;
mod plastic_work;
mod plasticity;
mod quadratic_advance;
mod quadratic_bending;
mod quadratic_closest;
mod quadratic_cohesive;
mod quadratic_coulomb;
mod quadratic_coulomb_dynamics;
mod quadratic_dynamic_surface_contact;
mod quadratic_dynamics;
mod quadratic_energy_rate;
mod quadratic_finite;
mod quadratic_fracture_work;
mod quadratic_friction;
mod quadratic_friction_dynamics;
mod quadratic_interface_mesh;
mod quadratic_interface_work;
mod quadratic_mass;
mod quadratic_normal_impact;
mod quadratic_patch;
mod quadratic_plane;
mod quadratic_render_surface;
mod quadratic_rotation;
mod quadratic_surface;
mod quadratic_surface_contact;
mod quadratic_sweep;
mod quadratic_wet_update;
mod quadratic_work;
mod soft_body;
mod solid_dynamics;
mod solid_film_binding;
mod solid_heat_conduction;
mod solid_surface;
mod sopromat;
mod wear;
mod wear_column;
mod wear_dust;
mod wear_suspension;
mod wear_wet;
mod wet_cohesive;
mod wet_material;

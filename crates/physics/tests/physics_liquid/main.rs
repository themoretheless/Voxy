//! Physics integration tests: liquids, droplets, buoyancy and suspensions.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_liquid <module>::` runs it alone.

#[path = "../support/fluid.rs"]
mod support_fluid;
#[path = "../support/rigid.rs"]
mod support_rigid;

mod buoyancy;
mod liquid;
mod liquid_adhesion;
mod liquid_ballistic_population;
mod liquid_body_contacts;
mod liquid_boundary;
mod liquid_collision;
mod liquid_contact_angle;
mod liquid_density_operator;
mod liquid_dissipation;
mod liquid_droplet_drag;
mod liquid_droplet_finite_gas;
mod liquid_droplet_gas_grid;
mod liquid_droplet_lifecycle;
mod liquid_droplet_merge;
mod liquid_droplet_optics;
mod liquid_droplet_population;
mod liquid_droplet_split;
mod liquid_evaporation;
mod liquid_exchange;
mod liquid_film_capture;
mod liquid_film_rebound;
mod liquid_forcing;
mod liquid_gas;
mod liquid_gas_grid_flow;
mod liquid_gas_grid_heat;
mod liquid_gas_grid_viscosity;
mod liquid_gas_grid_wall_viscosity;
mod liquid_geometry;
mod liquid_impact_events;
mod liquid_impact_spray;
mod liquid_mixture_heat;
mod liquid_mixture_profile;
mod liquid_mixture_properties;
mod liquid_multi_body_world;
mod liquid_multi_rebound;
mod liquid_phase;
mod liquid_pressure_work;
mod liquid_properties;
mod liquid_reaction;
mod liquid_reflecting;
mod liquid_rigid_body_world;
mod liquid_saturation;
mod liquid_spatial_vapor;
mod liquid_species;
mod liquid_sphere_deposition;
mod liquid_splash_onset;
mod liquid_surface;
mod liquid_surface_volume;
mod liquid_suspension;
mod liquid_tensile_pressure;
mod liquid_thixotropy;
mod liquid_transport;
mod liquid_viscoelastic;
mod liquid_wall_heat;
mod suspension;
mod water_coexistence;
mod water_equilibrium;
mod water_helmholtz;
mod water_saturation;

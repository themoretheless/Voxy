//! Physics integration tests: astrophysics and gravity.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_astro <module>::` runs it alone.

mod astrophysics;
mod astrophysics_atmosphere;
mod astrophysics_binary;
mod astrophysics_column;
mod astrophysics_coupled;
mod astrophysics_eos;
mod astrophysics_evolution;
mod astrophysics_free_free;
mod astrophysics_gas;
mod astrophysics_gas_gravity;
mod astrophysics_ionized_evolution;
mod astrophysics_nuclear;
mod astrophysics_opacity;
mod astrophysics_radhydro;
mod astrophysics_radiation;
mod astrophysics_reaclib;
mod astrophysics_reactive;
mod astrophysics_spherical;
mod astrophysics_spherical_eos;
mod astrophysics_spherical_radiation;
mod astrophysics_star;
mod astrophysics_thermal;
mod astrophysics_tides;
mod gravity;
mod gravity_character;
mod gravity_field;
mod gravity_spheres;

//! Physics integration tests: surface films.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_film <module>::` runs it alone.

#[path = "../support/film.rs"]
mod support_film;

mod surface_film;
mod surface_film_accelerated_sphere;
mod surface_film_advection;
mod surface_film_atomic_contact;
mod surface_film_body;
mod surface_film_checkpoint;
mod surface_film_free_surface;
mod surface_film_mixture;
mod surface_film_rheology;
mod surface_film_shear;
mod surface_film_smear_contact;
mod surface_film_sphere;
mod surface_film_squeeze;
mod surface_film_squeeze_body;
mod surface_film_thermal;
mod surface_film_vapor;
mod surface_film_withdrawal;

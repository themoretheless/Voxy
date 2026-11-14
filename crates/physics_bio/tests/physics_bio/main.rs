//! Physics integration tests: biomechanics, tissues, skin, circulation, lymph and respiration.
//!
//! One binary per domain instead of one per file. A module is one former test
//! file: `cargo test -p physics --test physics_bio <module>::` runs it alone.

mod anatomy_tet;
mod biomechanics;
mod cell_poroelastic;
mod circulation;
mod convex_tissue_surface;
mod embedded_skin_contact;
mod finite_contact;
mod finite_inertia;
mod finite_quadratic_dynamics;
mod gauge_pressure;
mod genital_folds;
mod implicit_pore;
mod inertial_tissue_assembly;
mod lattice_tissue_volume;
mod lbfgs;
mod lymph;
mod medit_tissue_volume;
mod mixed_darcy;
mod moving_fem_supports;
mod moving_plane_contact;
mod muscle_activation;
mod muscle_dynamics;
mod myocardium;
mod organ_coupling;
mod perfusion;
mod poroelastic;
mod prescribed_surface_contact;
mod respiration;
mod secondary_motion;
mod skin;
mod skin_embedding;
mod skin_rebase;
mod solid_film_binding;
mod solid_heat_conduction;
mod stress;
mod surface_contact;
mod tetrahedral_overlap;
mod tissue_assembly_history;
mod tissue_bulk_modulus;
mod tissue_continuum_geometry;
mod tissue_gaps;
mod tissue_geometry;
mod tissue_surface;
mod tissues;
mod urogenital;
mod viscoelastic;
mod viscoelastic_hgo_body;
mod viscoelastic_inertia;
mod wall_refinement;

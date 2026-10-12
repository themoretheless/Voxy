//! Shared one-triangle surface film fixture (plan step B3).
use physics::surface_film::{Material, SurfaceFilm};

/// A unit right triangle in the XZ plane with the default film material.
pub fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap()
}

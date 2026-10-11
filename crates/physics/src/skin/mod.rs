//! Layered, anisotropic, nonlinear, viscoelastic skin shell in SI units.
//! Material energies are differentiated exactly and solved implicitly.
mod contact;
mod dual;
mod material;
mod shell;
mod adaptive;
pub use contact::*;
pub use material::*;
pub use shell::*;

mod embedding;
pub use embedding::{SkinEmbedding, SurfaceBinding};

mod genital_folds;
pub use genital_folds::{LabiaMinoraGeometry, PrepuceGeometry, TissueFold};

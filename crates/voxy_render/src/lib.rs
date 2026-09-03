//! GPU ownership, surface lifecycle, mesh upload formats, and material textures.

mod material;
mod renderer;
mod skinned;

pub use material::{MaterialError, MaterialLayer, MaterialPack, MaterialSet};
pub use renderer::{CameraView, GpuQuad, RenderOutcome, Renderer, RendererError, SurfaceState};
pub use skinned::{SkinnedMesh, SkinnedMeshError, SkinnedUploadError, SkinnedVertex};

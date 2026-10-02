//! Native Vulkan external-memory ownership boundary. No implicit API fallback.
//! Portable GPU code keeps its unsafe-code prohibition; audited Vulkan FFI is
//! confined to this crate's native module.
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod export;
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub use export::{EXTERNAL_MEMORY_FEATURE, VulkanExportBuffer, adapter_uuid, device_uuid};

#[cfg(all(any(target_os = "linux", target_os = "windows"), feature = "cuda"))]
mod gravity;
#[cfg(all(any(target_os = "linux", target_os = "windows"), feature = "cuda"))]
pub use gravity::CudaGravityGraphics;

#[cfg(target_os = "windows")]
mod d3d12;
#[cfg(target_os = "windows")]
pub use d3d12::{D3d12ExportBuffer, adapter_luid};

#[cfg(all(target_os = "windows", feature = "cuda"))]
mod d3d12_gravity;
#[cfg(all(target_os = "windows", feature = "cuda"))]
pub use d3d12_gravity::CudaGravityD3d12Graphics;

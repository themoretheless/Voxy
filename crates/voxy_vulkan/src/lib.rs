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

/// Verifies that a 16-byte Vulkan `VK_KHR_external_memory_capabilities` device UUID matches a
/// CUDA device UUID and is non-zero (preventing accidental fallback to an unpopulated UUID).
#[must_use]
pub const fn uuids_match_non_zero(vulkan_uuid: &[u8; 16], cuda_uuid: &[u8; 16]) -> bool {
    let mut non_zero = false;
    let mut i = 0;
    while i < 16 {
        if vulkan_uuid[i] != cuda_uuid[i] {
            return false;
        }
        if vulkan_uuid[i] != 0 {
            non_zero = true;
        }
        i += 1;
    }
    non_zero
}

/// Aligns an external memory byte size upward to the power-of-two `alignment` required by
/// Vulkan/CUDA dedicated memory export, returning `None` if `size == 0`, `alignment` is not a
/// power of two, or arithmetic overflows.
#[must_use]
pub const fn align_external_allocation_size(size: u64, alignment: u64) -> Option<u64> {
    if size == 0 || !alignment.is_power_of_two() {
        return None;
    }
    let mask = alignment - 1;
    match size.checked_add(mask) {
        Some(sum) => Some(sum & !mask),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_uuid_match_and_alignment_invariants() {
        let zero = [0_u8; 16];
        let mut valid = [0_u8; 16];
        valid[15] = 0x42;
        assert!(!uuids_match_non_zero(&zero, &zero));
        assert!(uuids_match_non_zero(&valid, &valid));
        assert_eq!(align_external_allocation_size(4097, 4096), Some(8192));
        assert_eq!(align_external_allocation_size(0, 4096), None);
        assert_eq!(align_external_allocation_size(1024, 3000), None);
    }
}

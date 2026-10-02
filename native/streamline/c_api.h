#ifndef VOXY_STREAMLINE_C_API_H
#define VOXY_STREAMLINE_C_API_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct VoxyStreamlineRuntime VoxyStreamlineRuntime;
// domain: 0 success, 1 invalid argument, 2 module error, 3 allocation/exception,
// 4 SDK result, 5 adapter ConfigureError.
// code is ModuleError for domain 2; windows_error preserves the loader's error.
typedef struct VoxyStreamlineStatus {
    uint32_t domain;
    uint32_t code;
    uint32_t windows_error;
} VoxyStreamlineStatus;
// Windows-only. UTF-16 absolute path with explicit code-unit length (no NUL).
// On failure output is cleared. Caller owns the resulting opaque handle.
VoxyStreamlineStatus voxy_streamline_load(
    const uint16_t* path, uint32_t length, VoxyStreamlineRuntime** output);
// Feature bits: 1 SR/DLAA, 2 FG (includes Reflex/PCL), 4 Reflex/PCL, 8 RR, 16 NR.
VoxyStreamlineStatus voxy_streamline_initialize_dx12(VoxyStreamlineRuntime* runtime, uint32_t features);
// interposition: 0 automatic, 1 manual, 2 manual with DXGI factory proxy.
VoxyStreamlineStatus voxy_streamline_initialize_dx12_ex(VoxyStreamlineRuntime* runtime, uint32_t features, uint32_t interposition);
// Device must be a live ID3D12Device, retained through successful SDK shutdown.
VoxyStreamlineStatus voxy_streamline_register_dx12(VoxyStreamlineRuntime* runtime, void* device);
// Manual hooking: call immediately after base interface creation. SDK may replace
// the slot. Caller manages COM ownership/refcounts and keeps proxy/SDK lifetimes valid.
VoxyStreamlineStatus voxy_streamline_native_interface(VoxyStreamlineRuntime* runtime, void* proxy, void** output);
VoxyStreamlineStatus voxy_streamline_upgrade_interface(VoxyStreamlineRuntime* runtime, void** base_interface);
// Feature selector: 1 SR/DLAA, 2 FG, 4 Reflex, 8 RR, 16 NR. LUID is exactly 8 bytes.
VoxyStreamlineStatus voxy_streamline_dx12_support(VoxyStreamlineRuntime* runtime, uint32_t feature, const uint8_t* luid);
VoxyStreamlineStatus voxy_streamline_close(VoxyStreamlineRuntime* runtime);
typedef struct VoxyFrameGenerationState {
    uint32_t status; uint32_t minimum_dimension; uint32_t maximum_generated_frames;
    uint32_t frames_presented; uint32_t dynamic_supported; uint32_t vsync_supported;
    uint64_t estimated_vram_bytes;
    void* inputs_completion_fence;
    uint64_t inputs_completion_value;
} VoxyFrameGenerationState;
// SDK-owned fence; retain runtime/device, wait before reusing tagged FG inputs.
// Query on present thread; SDK frames_presented counter is consumed by the query.
VoxyStreamlineStatus voxy_streamline_fg_state(VoxyStreamlineRuntime* runtime, uint32_t viewport, VoxyFrameGenerationState* output);
typedef struct VoxyDlssSize { uint32_t width; uint32_t height; } VoxyDlssSize;
typedef struct VoxyRayReconstructionOptions {
    uint32_t mode; uint32_t width; uint32_t height; uint32_t hdr;
    uint32_t packed_normal_roughness;
    float world_to_view[16]; float view_to_world[16];
} VoxyRayReconstructionOptions;
// Same quality selectors as SR; matrices use SDK row-vector/row-major convention.
VoxyStreamlineStatus voxy_streamline_configure_rr(VoxyStreamlineRuntime* runtime,
    uint32_t viewport, const VoxyRayReconstructionOptions* options, VoxyDlssSize* output);
// Mode: 1 performance, 2 balanced, 3 quality, 4 ultra performance, 5 ultra quality, 6 DLAA.
VoxyStreamlineStatus voxy_streamline_configure_dlss(VoxyStreamlineRuntime* runtime, uint32_t viewport,
    uint32_t mode, uint32_t width, uint32_t height, uint32_t hdr, VoxyDlssSize* output);
// Reflex mode: 0 off, 1 low latency, 2 low latency with boost.
VoxyStreamlineStatus voxy_streamline_configure_reflex(VoxyStreamlineRuntime* runtime, uint32_t mode, uint32_t frame_limit_us);
// FG mode: 0 off, 1 fixed count, 2 dynamic. target_fps=0 lets SDK use display rate.
VoxyStreamlineStatus voxy_streamline_configure_fg(VoxyStreamlineRuntime* runtime, uint32_t viewport,
    uint32_t mode, uint32_t generated_frames, float target_fps);
typedef struct VoxyStreamlineFrame VoxyStreamlineFrame;
typedef struct VoxyDx12TextureTag {
    void* resource;
    uint32_t state;
    // Official SDK BufferType: 0..8 basic/material tags, 14 normal/roughness,
    // 42 specular hit distance, 45 diffuse hit distance. Other values rejected.
    uint32_t type;
    // 0 only valid now, 1 valid until present, 2 valid until evaluate.
    uint32_t lifecycle;
    uint32_t left; uint32_t top; uint32_t width; uint32_t height;
} VoxyDx12TextureTag;
// Each resource must be a live ID3D12Resource texture on the registered device.
// Extents must fit the texture; state must be correct when SDK uses the resource.
// Caller owns resources, command list, barriers and GPU lifetime synchronization.
// Volatile tags require a recording command list; until-present tags may omit it.
VoxyStreamlineStatus voxy_streamline_frame_tag_dx12(VoxyStreamlineFrame* frame,
    uint32_t viewport, const VoxyDx12TextureTag* tags, uint32_t count, void* command_list);
VoxyStreamlineStatus voxy_streamline_begin_frame(VoxyStreamlineRuntime* runtime, const uint32_t* index, VoxyStreamlineFrame** output);
VoxyStreamlineStatus voxy_streamline_frame_sleep(VoxyStreamlineFrame* frame);
// Marker selector 0..5: simulation start/end, render-submit start/end, present start/end.
VoxyStreamlineStatus voxy_streamline_frame_marker(VoxyStreamlineFrame* frame, uint32_t marker);
typedef struct VoxyCameraConstants {
    float projection[16]; float inverse_projection[16];
    float clip_to_previous[16]; float previous_to_clip[16];
    float position[3]; float up[3]; float right[3]; float forward[3];
    float jitter[2]; float motion_scale[2];
    float near_plane; float far_plane; float fov; float aspect;
    uint32_t reset;
} VoxyCameraConstants;
// Unjittered SDK row-major matrices; perspective, normal [0,1] depth,
// backward 2D vectors including camera motion. Inputs must match tagged textures.
VoxyStreamlineStatus voxy_streamline_frame_constants(VoxyStreamlineFrame* frame, uint32_t viewport, const VoxyCameraConstants* constants);
// Feature: 1 SR/DLAA or 8 RR. command_list is a live, recording
// ID3D12GraphicsCommandList on the registered device. Caller supplies resource
// tags, states, barriers, submission and GPU completion lifetime protection.
// FG runs through the presentation integration, not this evaluation entrypoint.
VoxyStreamlineStatus voxy_streamline_frame_evaluate_dx12(VoxyStreamlineFrame* frame,
    uint32_t viewport, uint32_t feature, void* command_list);
void voxy_streamline_frame_destroy(VoxyStreamlineFrame* frame);
void voxy_streamline_destroy(VoxyStreamlineRuntime* runtime);
#ifdef __cplusplus
}
#endif
#endif

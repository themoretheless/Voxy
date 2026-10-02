#pragma once
#include <sl_dlss_g.h>

namespace voxy::streamline {
// Functions must be resolved after Streamline initialization/device registration.
// They must outlive this adapter. Calls belong on the SDK's present thread.
struct FrameGenerationApi {
    PFun_slDLSSGGetState* get_state = nullptr;
    PFun_slDLSSGSetOptions* set_options = nullptr;
};
enum class ConfigureError {
    none, missing_function, reflex_required, reflex_unavailable, invalid_count,
    dynamic_unavailable, invalid_target, invalid_mode, invalid_state, invalid_sdk_output, sdk_failure
};
struct ConfigureResult {
    ConfigureError error;
    sl::Result sdk_result;
};
// Clears output on any failure so partial/stale function tables cannot be used.
ConfigureResult resolve_frame_generation(
    PFun_slGetFeatureFunction* resolver, FrameGenerationApi& output);
// Snapshot query consumes SDK presented-frame counters; call on present thread.
ConfigureResult query_frame_generation(FrameGenerationApi api, unsigned viewport, sl::DLSSGState& output);
struct FrameGenerationRequest {
    sl::DLSSGMode mode = sl::DLSSGMode::eOff;
    unsigned generated_frames = 1;
    // Zero selects display refresh rate, as defined by the SDK.
    float dynamic_target_fps = 0.0f;
};
ConfigureResult configure_frame_generation(
    FrameGenerationApi api, unsigned viewport,
    const FrameGenerationRequest& request, bool reflex_active);
}

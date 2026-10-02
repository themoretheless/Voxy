#ifdef _WIN32
#include "c_api.h"
#include "windows_runtime.hpp"
#include "dlss.hpp"
#include "reflex.hpp"
#include "frame_token.hpp"
#include <memory>
#include <vector>
#include <cmath>
static_assert(sl::kBufferTypeNormals == 5 && sl::kBufferTypeRoughness == 6 &&
    sl::kBufferTypeAlbedo == 7 && sl::kBufferTypeSpecularAlbedo == 8 &&
    sl::kBufferTypeNormalRoughness == 14 && sl::kBufferTypeSpecularHitDistance == 42 &&
    sl::kBufferTypeDiffuseHitDistance == 45, "C ABI resource roles must match the SDK");
struct VoxyStreamlineRuntime {
    bool reflex_enabled = false;
    bool fg_enabled = false;
    std::vector<sl::Feature> features;
    voxy::streamline::WindowsRuntime runtime;
};
extern "C" VoxyStreamlineStatus voxy_streamline_load(
    const uint16_t* path, uint32_t length, VoxyStreamlineRuntime** output) {
    if (!output) return {1, 0, 0};
    *output = nullptr;
    if (!path || !length || length > 32767) return {1, 0, 0};
    try {
        std::wstring wide;
        wide.reserve(length);
        for (uint32_t i = 0; i < length; ++i) {
            if (!path[i]) return {1, 0, 0};
            wide.push_back(static_cast<wchar_t>(path[i]));
        }
        auto candidate = std::make_unique<VoxyStreamlineRuntime>();
        const auto result = candidate->runtime.load(wide);
        if (result != voxy::streamline::ModuleError::none)
            return {2, static_cast<uint32_t>(result), candidate->runtime.last_windows_error()};
        *output = candidate.release();
        return {0, 0, 0};
    } catch (...) {
        // Never unwind C++ exceptions across the Rust/C boundary.
        return {3, 0, 0};
    }
}
static VoxyStreamlineStatus status(voxy::streamline::ConfigureResult result) {
    using voxy::streamline::ConfigureError;
    if (result.error == ConfigureError::none) return {0, 0, 0};
    if (result.error == ConfigureError::sdk_failure) return {4, static_cast<uint32_t>(result.sdk_result), 0};
    return {5, static_cast<uint32_t>(result.error), 0};
}
extern "C" VoxyStreamlineStatus voxy_streamline_initialize_dx12(VoxyStreamlineRuntime* runtime, uint32_t features) {
    return voxy_streamline_initialize_dx12_ex(runtime, features, 0);
}
extern "C" VoxyStreamlineStatus voxy_streamline_initialize_dx12_ex(
    VoxyStreamlineRuntime* runtime, uint32_t features, uint32_t interposition) {
    if (!runtime || !features || (features & ~31u) || interposition > 2u) return {1, 0, 0};
    try {
        auto* session = runtime->runtime.session();
        if (!session || session->initialized()) return {5, static_cast<uint32_t>(voxy::streamline::ConfigureError::invalid_state), 0};
        runtime->features.clear();
        if (features & 1u) runtime->features.push_back(sl::kFeatureDLSS);
        if (features & 2u) runtime->features.push_back(sl::kFeatureDLSS_G);
        if (features & 6u) {
            runtime->features.push_back(sl::kFeatureReflex);
            runtime->features.push_back(sl::kFeaturePCL);
        }
        if (features & 8u) runtime->features.push_back(sl::kFeatureDLSS_RR);
        if (features & 16u) runtime->features.push_back(sl::kFeatureDLSS_NR);
        sl::Preferences preferences{};
        preferences.engineVersion = "0.1.0";
        preferences.renderAPI = sl::RenderAPI::eD3D12;
        preferences.flags |= sl::PreferenceFlags::eUseFrameBasedResourceTagging;
        if (!voxy::streamline::configure_dx12_interposition(preferences, interposition)) return {1, 0, 0};
        preferences.featuresToLoad = runtime->features.data();
        preferences.numFeaturesToLoad = static_cast<uint32_t>(runtime->features.size());
        return status(session->initialize(preferences));
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_register_dx12(VoxyStreamlineRuntime* runtime, void* device) {
    if (!runtime || !device) return {1, 0, 0};
    try { return status(runtime->runtime.session()->set_d3d_device(device)); }
    catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_native_interface(
    VoxyStreamlineRuntime* runtime, void* proxy, void** output) {
    if (output) *output = nullptr;
    if (!runtime || !proxy || !output) return {1, 0, 0};
    try { return status(runtime->runtime.session()->native_interface(proxy, output)); }
    catch (...) { *output = nullptr; return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_upgrade_interface(
    VoxyStreamlineRuntime* runtime, void** base_interface) {
    if (!runtime || !base_interface || !*base_interface) return {1, 0, 0};
    try { return status(runtime->runtime.session()->upgrade_interface(base_interface)); }
    catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_dx12_support(VoxyStreamlineRuntime* runtime, uint32_t feature, const uint8_t* luid) {
    if (!runtime || !luid) return {1, 0, 0};
    sl::Feature selected;
    switch (feature) {
    case 1: selected = sl::kFeatureDLSS; break;
    case 2: selected = sl::kFeatureDLSS_G; break;
    case 4: selected = sl::kFeatureReflex; break;
    case 8: selected = sl::kFeatureDLSS_RR; break;
    case 16: selected = sl::kFeatureDLSS_NR; break;
    default: return {1, 0, 0};
    }
    try {
        uint8_t copy[8];
        for (unsigned i = 0; i < 8; ++i) copy[i] = luid[i];
        sl::AdapterInfo adapter{};
        adapter.deviceLUID = copy;
        adapter.deviceLUIDSizeInBytes = 8;
        return status(runtime->runtime.session()->feature_support(selected, adapter));
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_close(VoxyStreamlineRuntime* runtime) {
    if (!runtime) return {1, 0, 0};
    try {
        const auto result = runtime->runtime.session()->close();
        if (result.error == voxy::streamline::ConfigureError::none) { runtime->reflex_enabled = false; runtime->fg_enabled = false; }
        return status(result);
    }
    catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_configure_dlss(VoxyStreamlineRuntime* runtime, uint32_t viewport,
    uint32_t mode, uint32_t width, uint32_t height, uint32_t hdr, VoxyDlssSize* output) {
    if (!output) return {1, 0, 0};
    *output = {};
    if (!runtime || hdr > 1 || mode < 1 || mode > 6) return {1, 0, 0};
    try {
        voxy::streamline::DlssApi api{};
        auto result = voxy::streamline::resolve_dlss(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        sl::DLSSOptions options{};
        // Adapter-owned selector mapping, never a Rust SDK struct layout.
        const sl::DLSSMode modes[] = {sl::DLSSMode::eOff, sl::DLSSMode::eMaxPerformance,
            sl::DLSSMode::eBalanced, sl::DLSSMode::eMaxQuality, sl::DLSSMode::eUltraPerformance,
            sl::DLSSMode::eUltraQuality, sl::DLSSMode::eDLAA};
        options.mode = modes[mode];
        options.outputWidth = width;
        options.outputHeight = height;
        options.colorBuffersHDR = hdr ? sl::Boolean::eTrue : sl::Boolean::eFalse;
        sl::DLSSOptimalSettings settings{};
        result = voxy::streamline::optimal_dlss_settings(api, options, settings);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        result = voxy::streamline::configure_dlss(api, viewport, options);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        *output = {settings.optimalRenderWidth, settings.optimalRenderHeight};
        return {0, 0, 0};
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_configure_reflex(VoxyStreamlineRuntime* runtime, uint32_t mode, uint32_t frame_limit_us) {
    if (!runtime || mode > 2) return {1, 0, 0};
    if (!mode && runtime->fg_enabled) return {5, static_cast<uint32_t>(voxy::streamline::ConfigureError::reflex_required), 0};
    try {
        voxy::streamline::ReflexApi api{};
        auto result = voxy::streamline::resolve_reflex(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        const sl::ReflexMode modes[] = {sl::ReflexMode::eOff, sl::ReflexMode::eLowLatency,
            sl::ReflexMode::eLowLatencyWithBoost};
        result = voxy::streamline::configure_reflex(api, modes[mode], frame_limit_us);
        if (result.error == voxy::streamline::ConfigureError::none) runtime->reflex_enabled = mode != 0;
        return status(result);
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_fg_state(
    VoxyStreamlineRuntime* runtime, uint32_t viewport, VoxyFrameGenerationState* output) {
    if (output) *output = {};
    if (!runtime || !output) return {1, 0, 0};
    try {
        voxy::streamline::FrameGenerationApi api{};
        auto result = voxy::streamline::resolve_frame_generation(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        sl::DLSSGState state{};
        result = voxy::streamline::query_frame_generation(api, viewport, state);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        *output = {static_cast<uint32_t>(state.status), state.minWidthOrHeight,
            state.numFramesToGenerateMax, state.numFramesActuallyPresented,
            state.bIsDynamicMFGSupported == sl::Boolean::eTrue ? 1u : 0u,
            state.bIsVsyncSupportAvailable == sl::Boolean::eTrue ? 1u : 0u,
            state.estimatedVRAMUsageInBytes, state.inputsProcessingCompletionFence,
            state.lastPresentInputsProcessingCompletionFenceValue};
        return {0, 0, 0};
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_configure_fg(VoxyStreamlineRuntime* runtime, uint32_t viewport,
    uint32_t mode, uint32_t generated_frames, float target_fps) {
    if (!runtime || mode > 2) return {1, 0, 0};
    try {
        voxy::streamline::FrameGenerationApi api{};
        auto result = voxy::streamline::resolve_frame_generation(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        voxy::streamline::FrameGenerationRequest request{};
        const sl::DLSSGMode modes[] = {sl::DLSSGMode::eOff, sl::DLSSGMode::eOn, sl::DLSSGMode::eDynamic};
        request.mode = modes[mode];
        request.generated_frames = generated_frames;
        request.dynamic_target_fps = target_fps;
        result = voxy::streamline::configure_frame_generation(api, viewport, request, runtime->reflex_enabled);
        if (result.error == voxy::streamline::ConfigureError::none) runtime->fg_enabled = mode != 0;
        return status(result);
    } catch (...) { return {3, 0, 0}; }
}
struct VoxyStreamlineFrame {
    VoxyStreamlineRuntime* runtime;
    sl::FrameToken* token;
    std::unique_ptr<voxy::streamline::ReflexFrame> reflex;
};
extern "C" VoxyStreamlineStatus voxy_streamline_begin_frame(VoxyStreamlineRuntime* runtime, const uint32_t* index, VoxyStreamlineFrame** output) {
    if (!output) return {1, 0, 0};
    *output = nullptr;
    if (!runtime) return {1, 0, 0};
    try {
        voxy::streamline::ReflexApi api{};
        auto result = voxy::streamline::resolve_reflex(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        sl::FrameToken* token = nullptr;
        result = voxy::streamline::acquire_frame_token(runtime->runtime.frame_tokens(), index, token);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        auto frame = std::make_unique<VoxyStreamlineFrame>();
        frame->runtime = runtime;
        frame->token = token;
        frame->reflex = std::make_unique<voxy::streamline::ReflexFrame>(api, *token);
        *output = frame.release();
        return {0, 0, 0};
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_frame_sleep(VoxyStreamlineFrame* frame) {
    if (!frame) return {1, 0, 0};
    try {
        const auto result = frame->reflex->sleep();
        return result == sl::Result::eOk ? VoxyStreamlineStatus{0, 0, 0} : VoxyStreamlineStatus{4, static_cast<uint32_t>(result), 0};
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_frame_marker(VoxyStreamlineFrame* frame, uint32_t marker) {
    if (!frame || marker > 5) return {1, 0, 0};
    try {
        const sl::PCLMarker markers[] = {sl::PCLMarker::eSimulationStart, sl::PCLMarker::eSimulationEnd,
            sl::PCLMarker::eRenderSubmitStart, sl::PCLMarker::eRenderSubmitEnd,
            sl::PCLMarker::ePresentStart, sl::PCLMarker::ePresentEnd};
        const auto result = frame->reflex->mark(markers[marker]);
        return result == sl::Result::eOk ? VoxyStreamlineStatus{0, 0, 0} : VoxyStreamlineStatus{4, static_cast<uint32_t>(result), 0};
    } catch (...) { return {3, 0, 0}; }
}
static bool finite_values(const float* values, unsigned count) {
    for (unsigned i = 0; i < count; ++i) if (!std::isfinite(values[i])) return false;
    return true;
}
extern "C" VoxyStreamlineStatus voxy_streamline_frame_tag_dx12(
    VoxyStreamlineFrame* frame, uint32_t viewport, const VoxyDx12TextureTag* input,
    uint32_t count, void* command_list) {
    if (!frame || !input || !count || count > 12) return {1, 0, 0};
    uint64_t seen = 0;
    for (unsigned i = 0; i < count; ++i) {
        const auto& tag = input[i];
        const bool supported = tag.type <= 8 || tag.type == sl::kBufferTypeNormalRoughness ||
            tag.type == sl::kBufferTypeSpecularHitDistance || tag.type == sl::kBufferTypeDiffuseHitDistance;
        if (!tag.resource || tag.state == UINT32_MAX || !supported || tag.lifecycle > 2 ||
            !tag.width || !tag.height || tag.left > UINT32_MAX - tag.width ||
            tag.top > UINT32_MAX - tag.height || (seen & (uint64_t{1} << tag.type)) ||
            (!command_list && tag.lifecycle != 1)) return {1, 0, 0};
        seen |= uint64_t{1} << tag.type;
    }
    try {
        std::vector<sl::Resource> resources;
        std::vector<sl::ResourceTag> tags;
        resources.reserve(count); tags.reserve(count);
        for (unsigned i = 0; i < count; ++i) {
            const auto& tag = input[i];
            resources.emplace_back(sl::ResourceType::eTex2d, tag.resource, tag.state);
            const sl::Extent extent{tag.top, tag.left, tag.width, tag.height};
            tags.emplace_back(&resources.back(), tag.type,
                static_cast<sl::ResourceLifecycle>(tag.lifecycle), &extent);
        }
        return status(frame->runtime->runtime.session()->tag_resources(
            *frame->token, viewport, tags.data(), count, command_list));
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_frame_evaluate_dx12(
    VoxyStreamlineFrame* frame, uint32_t viewport, uint32_t feature, void* command_list) {
    if (!frame || !command_list || (feature != 1 && feature != 8)) return {1, 0, 0};
    try {
        const sl::ViewportHandle handle(viewport);
        const sl::BaseStructure* inputs[] = {&handle};
        return status(frame->runtime->runtime.session()->evaluate(
            feature == 1 ? sl::kFeatureDLSS : sl::kFeatureDLSS_RR,
            *frame->token, inputs, 1, command_list));
    } catch (...) { return {3, 0, 0}; }
}
static sl::float4x4 matrix(const float* values) {
    sl::float4x4 output{};
    for (unsigned row = 0; row < 4; ++row)
        output[row] = {values[row*4], values[row*4+1], values[row*4+2], values[row*4+3]};
    return output;
}
extern "C" VoxyStreamlineStatus voxy_streamline_configure_rr(VoxyStreamlineRuntime* runtime,
    uint32_t viewport, const VoxyRayReconstructionOptions* input, VoxyDlssSize* output) {
    if (!output) return {1, 0, 0};
    *output = {};
    if (!runtime || !input || input->mode < 1 || input->mode > 6 || input->hdr > 1 ||
        input->packed_normal_roughness > 1) return {1, 0, 0};
    try {
        voxy::streamline::RayReconstructionApi api{};
        auto result = voxy::streamline::resolve_ray_reconstruction(runtime->runtime.resolver(), api);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        sl::DLSSDOptions options{};
        const sl::DLSSMode modes[] = {sl::DLSSMode::eOff, sl::DLSSMode::eMaxPerformance,
            sl::DLSSMode::eBalanced, sl::DLSSMode::eMaxQuality, sl::DLSSMode::eUltraPerformance,
            sl::DLSSMode::eUltraQuality, sl::DLSSMode::eDLAA};
        options.mode = modes[input->mode];
        options.outputWidth = input->width; options.outputHeight = input->height;
        options.colorBuffersHDR = input->hdr ? sl::Boolean::eTrue : sl::Boolean::eFalse;
        options.normalRoughnessMode = input->packed_normal_roughness ?
            sl::DLSSDNormalRoughnessMode::ePacked : sl::DLSSDNormalRoughnessMode::eUnpacked;
        options.worldToCameraView = matrix(input->world_to_view);
        options.cameraViewToWorld = matrix(input->view_to_world);
        sl::DLSSDOptimalSettings settings{};
        result = voxy::streamline::configure_ray_reconstruction(api, viewport, options, settings);
        if (result.error != voxy::streamline::ConfigureError::none) return status(result);
        *output = {settings.optimalRenderWidth, settings.optimalRenderHeight};
        return {0, 0, 0};
    } catch (...) { return {3, 0, 0}; }
}
extern "C" VoxyStreamlineStatus voxy_streamline_frame_constants(VoxyStreamlineFrame* frame, uint32_t viewport, const VoxyCameraConstants* input) {
    if (!frame || !input || input->reset > 1) return {1, 0, 0};
    if (!finite_values(input->projection, 16) || !finite_values(input->inverse_projection, 16) ||
        !finite_values(input->clip_to_previous, 16) || !finite_values(input->previous_to_clip, 16) ||
        !finite_values(input->position, 3) || !finite_values(input->up, 3) || !finite_values(input->right, 3) ||
        !finite_values(input->forward, 3) || !finite_values(input->jitter, 2) || !finite_values(input->motion_scale, 2) ||
        !std::isfinite(input->near_plane) || !std::isfinite(input->far_plane) ||
        !std::isfinite(input->fov) || !std::isfinite(input->aspect) || input->near_plane <= 0 ||
        input->far_plane <= input->near_plane || input->fov <= 0 || input->fov >= 3.141592654f || input->aspect <= 0)
        return {1, 0, 0};
    try {
        sl::Constants constants{};
        constants.cameraViewToClip = matrix(input->projection);
        constants.clipToCameraView = matrix(input->inverse_projection);
        constants.clipToPrevClip = matrix(input->clip_to_previous);
        constants.prevClipToClip = matrix(input->previous_to_clip);
        const float identity[] = {1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1};
        constants.clipToLensClip = matrix(identity);
        constants.cameraPos = {input->position[0], input->position[1], input->position[2]};
        constants.cameraUp = {input->up[0], input->up[1], input->up[2]};
        constants.cameraRight = {input->right[0], input->right[1], input->right[2]};
        constants.cameraFwd = {input->forward[0], input->forward[1], input->forward[2]};
        constants.jitterOffset = {input->jitter[0], input->jitter[1]};
        constants.mvecScale = {input->motion_scale[0], input->motion_scale[1]};
        constants.cameraPinholeOffset = {0,0};
        constants.cameraNear = input->near_plane; constants.cameraFar = input->far_plane;
        constants.cameraFOV = input->fov; constants.cameraAspectRatio = input->aspect;
        constants.depthInverted = sl::Boolean::eFalse;
        constants.cameraMotionIncluded = sl::Boolean::eTrue;
        constants.motionVectors3D = sl::Boolean::eFalse;
        constants.reset = input->reset ? sl::Boolean::eTrue : sl::Boolean::eFalse;
        return status(frame->runtime->runtime.session()->set_constants(constants, *frame->token, viewport));
    } catch (...) { return {3, 0, 0}; }
}
extern "C" void voxy_streamline_frame_destroy(VoxyStreamlineFrame* frame) { delete frame; }
extern "C" void voxy_streamline_destroy(VoxyStreamlineRuntime* runtime) {
    delete runtime;
}
#endif

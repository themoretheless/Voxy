#include "c_api.h"
#include <cstddef>
static_assert(sizeof(VoxyFrameGenerationState) == 48);
static_assert(offsetof(VoxyFrameGenerationState, estimated_vram_bytes) == 24);
static_assert(offsetof(VoxyFrameGenerationState, inputs_completion_fence) == 32);
static_assert(sizeof(VoxyCameraConstants) == 340);
static_assert(offsetof(VoxyCameraConstants, position) == 256);
static_assert(offsetof(VoxyCameraConstants, reset) == 336);
static_assert(sizeof(VoxyRayReconstructionOptions) == 148);
static_assert(offsetof(VoxyRayReconstructionOptions, world_to_view) == 20);
static_assert(offsetof(VoxyRayReconstructionOptions, view_to_world) == 84);
#include "frame_generation.hpp"
#include "reflex.hpp"
#include "frame_token.hpp"
#include "session.hpp"
#include "dlss.hpp"
#ifdef _WIN32
#include "windows_runtime.hpp"
#endif
#include <cassert>
#include <limits>
#include <cstring>

using namespace voxy::streamline;
static unsigned changes = 0;
static sl::Result reported_result = sl::Result::eOk;
static sl::Result options_result = sl::Result::eOk;
static bool dynamic_supported = true;
static unsigned maximum_generated_frames = 5;
static sl::DLSSGOptions last_options{};
static sl::Result get_state(const sl::ViewportHandle& viewport, sl::DLSSGState& state,
                            const sl::DLSSGOptions*) {
    assert(static_cast<unsigned>(viewport) == 7);
    state.numFramesToGenerateMax = maximum_generated_frames;
    state.bIsDynamicMFGSupported = dynamic_supported ? sl::Boolean::eTrue : sl::Boolean::eFalse;
    return reported_result;
}
static sl::Result set_options(const sl::ViewportHandle& viewport, const sl::DLSSGOptions& options) {
    assert(static_cast<unsigned>(viewport) == 7);
    ++changes;
    if (options_result != sl::Result::eOk) return options_result;
    last_options = options;
    return reported_result;
}
static bool fail_second = false;
static bool null_second = false;
static sl::Result resolve(sl::Feature feature, const char* name, void*& function) {
    assert(feature == sl::kFeatureDLSS_G);
    if (std::strcmp(name, "slDLSSGGetState") == 0) {
        function = reinterpret_cast<void*>(get_state);
        return sl::Result::eOk;
    }
    assert(std::strcmp(name, "slDLSSGSetOptions") == 0);
    if (fail_second) return sl::Result::eErrorNoPlugins;
    function = null_second ? nullptr : reinterpret_cast<void*>(set_options);
    return sl::Result::eOk;
}
static unsigned reflex_changes = 0;
static bool reflex_available = true;
static sl::Result reflex_state_result = sl::Result::eOk;
static sl::Result reflex_state(sl::ReflexState& state) {
    state.lowLatencyAvailable = reflex_available;
    return reflex_state_result;
}
static sl::Result reflex_options(const sl::ReflexOptions& options) {
    ++reflex_changes;
    assert(options.mode == sl::ReflexMode::eLowLatencyWithBoost);
    assert(options.frameLimitUs == 6944);
    assert(!options.useMarkersToOptimize);
    return sl::Result::eOk;
}
// Only the fake SDK implementation constructs this token in tests.
struct TestToken : sl::FrameToken {
    operator uint32_t() const override { return 42; }
};
static TestToken test_token;
static bool token_null = false;
static sl::Result token_result = sl::Result::eOk;
static sl::Result new_token(sl::FrameToken*& token, const uint32_t* index) {
    assert(!index || *index == 42);
    token = token_null ? nullptr : &test_token;
    return token_result;
}
static sl::Result marker_result = sl::Result::eOk;
static unsigned marker_calls = 0;
static sl::Result sleep_frame(const sl::FrameToken& token) {
    assert(&token == &test_token);
    return sl::Result::eOk;
}
static sl::Result mark_frame(sl::PCLMarker, const sl::FrameToken& token) {
    assert(&token == &test_token);
    ++marker_calls;
    return marker_result;
}
static unsigned shutdown_calls = 0;
static sl::Result init_result = sl::Result::eOk;
static sl::Result shutdown_result = sl::Result::eOk;
static sl::Result device_result = sl::Result::eOk;
static sl::Result session_init(const sl::Preferences&, uint64_t version) {
    assert(version == sl::kSDKVersion);
    return init_result;
}
static sl::Result session_shutdown() { ++shutdown_calls; return shutdown_result; }
static sl::Result session_device(void* device) { assert(device); return device_result; }
static int upgraded_object = 0;
static unsigned upgrade_calls = 0;
static sl::Result upgrade_result = sl::Result::eOk;
static sl::Result upgrade_interface(void** base_interface) {
    ++upgrade_calls;
    if (upgrade_result == sl::Result::eOk) *base_interface = &upgraded_object;
    return upgrade_result;
}
static sl::Result native_interface(void* proxy, void** output) {
    assert(proxy);
    *output = &upgraded_object;
    return upgrade_result;
}
#ifdef VOXY_STREAMLINE_VULKAN
static sl::Result vulkan_device(const sl::VulkanInfo& info) {
    assert(info.computeQueueFamily == 2 && info.graphicsQueueFamily == 3);
    assert(info.opticalFlowQueueFamily == 4 && info.opticalFlowQueueIndex == 1);
    return device_result;
}
#endif
static sl::Result support_result = sl::Result::eOk;
static unsigned support_calls = 0;
static sl::Result feature_support(sl::Feature feature, const sl::AdapterInfo& adapter) {
    assert(feature == sl::kFeatureDLSS_G);
    assert(adapter.deviceLUID && adapter.deviceLUIDSizeInBytes == 8);
    ++support_calls;
    return support_result;
}
static unsigned constants_calls = 0;
static sl::Result constants_result = sl::Result::eOk;
static sl::Result frame_constants(const sl::Constants& constants, const sl::FrameToken& token,
                                  const sl::ViewportHandle& viewport) {
    assert(&token == &test_token && static_cast<unsigned>(viewport) == 7);
    assert(constants.reset == sl::Boolean::eTrue);
    ++constants_calls;
    return constants_result;
}
static unsigned tag_calls = 0;
static sl::Result resource_tags(const sl::FrameToken& token, const sl::ViewportHandle& viewport,
    const sl::ResourceTag* tags, uint32_t count, sl::CommandBuffer* command) {
    assert(&token == &test_token && static_cast<unsigned>(viewport) == 7);
    assert(tags && count == 1 && !command);
    ++tag_calls;
    return sl::Result::eOk;
}
static unsigned evaluation_calls = 0;
static sl::Result evaluation_result = sl::Result::eOk;
static sl::Result evaluate_feature(sl::Feature feature, const sl::FrameToken& token,
    const sl::BaseStructure** inputs, uint32_t count, sl::CommandBuffer* command) {
    assert(feature == sl::kFeatureDLSS);
    assert(&token == &test_token && inputs && count == 1 && command);
    assert(inputs[0]->structType == sl::ViewportHandle::s_structType);
    ++evaluation_calls;
    return evaluation_result;
}
static unsigned dlss_changes = 0;
static sl::Result dlss_set(const sl::ViewportHandle& viewport, const sl::DLSSOptions& options) {
    assert(static_cast<unsigned>(viewport) == 7 && options.mode == sl::DLSSMode::eDLAA);
    assert(options.outputWidth == 1920 && options.outputHeight == 1080);
    assert(options.colorBuffersHDR == sl::Boolean::eFalse);
    ++dlss_changes;
    return sl::Result::eOk;
}
static unsigned malformed_settings = 0;
static sl::Result dlss_optimal(const sl::DLSSOptions&, sl::DLSSOptimalSettings& settings) {
    settings.optimalRenderWidth = 1920;
    settings.optimalRenderHeight = 1080;
    settings.renderWidthMin = 1920;
    settings.renderWidthMax = 1920;
    settings.renderHeightMin = 1080;
    settings.renderHeightMax = 1080;
    if (malformed_settings == 1) settings.renderWidthMin = 0;
    if (malformed_settings == 2) settings.renderWidthMax = 1000;
    if (malformed_settings == 3) settings.optimalRenderHeight = 0;
    return sl::Result::eOk;
}
static bool rr_malformed = false;
static unsigned rr_changes = 0;
static sl::Result rr_optimal(const sl::DLSSDOptions&, sl::DLSSDOptimalSettings& output) {
    output.optimalRenderWidth = 1280; output.optimalRenderHeight = 720;
    output.renderWidthMin = 640; output.renderWidthMax = rr_malformed ? 1000 : 1920;
    output.renderHeightMin = 360; output.renderHeightMax = 1080;
    return sl::Result::eOk;
}
static sl::Result rr_set(const sl::ViewportHandle& viewport, const sl::DLSSDOptions& options) {
    assert(static_cast<uint32_t>(viewport) == 7 && options.mode == sl::DLSSMode::eMaxQuality);
    ++rr_changes;
    return sl::Result::eOk;
}
int main() {
    sl::DLSSDOptions rr_options{};
    rr_options.mode = sl::DLSSMode::eMaxQuality;
    rr_options.colorBuffersHDR = sl::Boolean::eTrue;
    rr_options.outputWidth = 1920; rr_options.outputHeight = 1080;
    for (unsigned i = 0; i < 4; ++i) {
        const sl::float4 identity[] = {{1,0,0,0}, {0,1,0,0}, {0,0,1,0}, {0,0,0,1}};
        rr_options.worldToCameraView[i] = identity[i];
        rr_options.cameraViewToWorld[i] = identity[i];
    }
    sl::DLSSDOptimalSettings rr_output{};
    RayReconstructionApi rr_api{rr_optimal, rr_set};
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::none);
    assert(rr_output.optimalRenderWidth == 1280 && rr_changes == 1);
    rr_options.colorBuffersHDR = sl::Boolean::eFalse;
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::invalid_target);
    assert(rr_output.optimalRenderWidth == 0 && rr_changes == 1);
    rr_options.colorBuffersHDR = sl::Boolean::eTrue;
    rr_malformed = true;
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::invalid_sdk_output);
    assert(rr_output.optimalRenderWidth == 0 && rr_changes == 1);
    rr_malformed = false;
    rr_options.worldToCameraView[3].x = 10;
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::invalid_target);
    assert(rr_output.optimalRenderWidth == 0 && rr_changes == 1);
    rr_options.cameraViewToWorld[3].x = -10;
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::none);
    assert(rr_changes == 2);
    rr_options.worldToCameraView[0].x = 0;
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::invalid_target);
    rr_options.worldToCameraView[0].x = std::numeric_limits<float>::quiet_NaN();
    assert(configure_ray_reconstruction(rr_api, 7, rr_options, rr_output).error == ConfigureError::invalid_target);
    RayReconstructionApi empty_rr{rr_optimal, rr_set};
    assert(resolve_ray_reconstruction(nullptr, empty_rr).error == ConfigureError::missing_function);
    assert(!empty_rr.optimal && !empty_rr.set_options);
    DlssApi dlss{dlss_optimal, dlss_set};
    sl::DLSSOptions dlss_options{};
    dlss_options.mode = sl::DLSSMode::eDLAA;
    dlss_options.outputWidth = 1920;
    dlss_options.outputHeight = 1080;
    dlss_options.colorBuffersHDR = sl::Boolean::eFalse;
    assert(configure_dlss(dlss, 7, dlss_options).error == ConfigureError::none);
    sl::DLSSOptimalSettings recommended{};
    assert(optimal_dlss_settings(dlss, dlss_options, recommended).error == ConfigureError::none);
    assert(recommended.optimalRenderWidth == 1920 && recommended.optimalRenderHeight == 1080);
    for (malformed_settings = 1; malformed_settings <= 3; ++malformed_settings) {
        assert(optimal_dlss_settings(dlss, dlss_options, recommended).error == ConfigureError::invalid_sdk_output);
        assert(!recommended.optimalRenderWidth && !recommended.optimalRenderHeight);
    }
    malformed_settings = 0;
    dlss_options.preExposure = std::numeric_limits<float>::quiet_NaN();
    assert(configure_dlss(dlss, 7, dlss_options).error == ConfigureError::invalid_target);
    assert(optimal_dlss_settings(dlss, dlss_options, recommended).error == ConfigureError::invalid_target);
    assert(recommended.optimalRenderWidth == 0 && dlss_changes == 1);

    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.evaluate = evaluate_feature;
        Session session(core);
        sl::ViewportHandle viewport(7u);
        const sl::BaseStructure* inputs[] = {&viewport};
        int fake_device = 0;
        auto* command = reinterpret_cast<sl::CommandBuffer*>(&fake_device);
        assert(session.evaluate(sl::kFeatureDLSS, test_token, inputs, 1, command).error == ConfigureError::invalid_state);
        assert(session.initialize(sl::Preferences{}).error == ConfigureError::none);
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::none);
        assert(session.evaluate(sl::kFeatureDLSS, test_token, inputs, 1, nullptr).error == ConfigureError::invalid_state);
        assert(session.evaluate(sl::kFeatureDLSS, test_token, inputs, 1, command).error == ConfigureError::none);
        evaluation_result = sl::Result::eErrorNGXFailed;
        assert(session.evaluate(sl::kFeatureDLSS, test_token, inputs, 1, command).sdk_result == evaluation_result);
        inputs[0] = nullptr;
        assert(session.evaluate(sl::kFeatureDLSS, test_token, inputs, 1, command).error == ConfigureError::invalid_state);
        assert(evaluation_calls == 2);
    }
    shutdown_calls = 0;

    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.set_tags = resource_tags;
        Session session(core);
        sl::Preferences preferences{};
        preferences.flags |= sl::PreferenceFlags::eUseFrameBasedResourceTagging;
        assert(session.initialize(preferences).error == ConfigureError::none);
        int fake_device = 0;
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::none);
        sl::ResourceTag tag(nullptr, sl::kBufferTypeDepth, sl::ResourceLifecycle::eValidUntilPresent);
        assert(session.tag_resources(test_token, 7, &tag, 1, nullptr).error == ConfigureError::none);
        assert(session.tag_resources(test_token, 7, nullptr, 1, nullptr).error == ConfigureError::invalid_state);
        sl::Resource resource{};
        tag.resource = &resource;
        tag.lifecycle = sl::ResourceLifecycle::eOnlyValidNow;
        assert(session.tag_resources(test_token, 7, &tag, 1, nullptr).error == ConfigureError::invalid_state);
        assert(tag_calls == 1);
    }
    shutdown_calls = 0;

    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.set_constants = frame_constants;
        Session session(core);
        sl::Constants constants{};
        constants.reset = sl::Boolean::eTrue;
        assert(session.set_constants(constants, test_token, 7).error == ConfigureError::invalid_state);
        assert(session.initialize(sl::Preferences{}).error == ConfigureError::none);
        assert(session.set_constants(constants, test_token, 7).error == ConfigureError::invalid_state);
        int fake_device = 0;
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::none);
        assert(session.set_constants(constants, test_token, 7).error == ConfigureError::none);
        constants_result = sl::Result::eErrorInvalidParameter;
        assert(session.set_constants(constants, test_token, 7).sdk_result == constants_result);
        assert(constants_calls == 2);
    }
    shutdown_calls = 0;

#ifdef _WIN32
    {
        VoxyStreamlineRuntime* output = nullptr;
        assert(voxy_streamline_load(nullptr, 0, &output).domain == 1);
        assert(!output);
        const uint16_t path[] = {'x', 0, 'y'};
        assert(voxy_streamline_load(path, 3, &output).domain == 1);
        assert(!output);
        voxy_streamline_destroy(nullptr);
    }
    {
        WindowsRuntime runtime;
        assert(!runtime.session() && !runtime.resolver() && !runtime.frame_tokens());
        assert(runtime.load(L"sl.interposer.dll") == ModuleError::invalid_path);
        assert(!runtime.session());
        std::wstring embedded_null = L"C:\\SDK\\sl.interposer.dll";
        embedded_null.push_back(L'\0');
        embedded_null += L"extra";
        assert(runtime.load(embedded_null) == ModuleError::invalid_path);
        assert(!runtime.session() && !runtime.resolver() && !runtime.frame_tokens());
    }
#endif

    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.is_feature_supported = feature_support;
        Session session(core);
        sl::AdapterInfo adapter{};
        assert(session.feature_support(sl::kFeatureDLSS_G, adapter).error == ConfigureError::invalid_state);
        assert(session.initialize(sl::Preferences{}).error == ConfigureError::none);
        assert(session.feature_support(sl::kFeatureDLSS_G, adapter).error == ConfigureError::invalid_state);
        uint8_t luid[8]{};
        adapter.deviceLUID = luid;
        adapter.deviceLUIDSizeInBytes = 8;
        assert(session.feature_support(sl::kFeatureDLSS_G, adapter).error == ConfigureError::none);
        support_result = sl::Result::eErrorAdapterNotSupported;
        const auto unsupported = session.feature_support(sl::kFeatureDLSS_G, adapter);
        assert(unsupported.error == ConfigureError::sdk_failure && unsupported.sdk_result == support_result);
        adapter.deviceLUIDSizeInBytes = 7;
        assert(session.feature_support(sl::kFeatureDLSS_G, adapter).error == ConfigureError::invalid_state);
        assert(support_calls == 2);
    }
    shutdown_calls = 0;

#ifdef VOXY_STREAMLINE_VULKAN
    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.set_vulkan_info = vulkan_device;
        Session session(core);
        sl::VulkanInfo info{};
        assert(session.set_vulkan_info(info).error == ConfigureError::invalid_state);
        sl::Preferences preferences{};
        preferences.renderAPI = sl::RenderAPI::eVulkan;
        assert(session.initialize(preferences).error == ConfigureError::none);
        assert(session.set_vulkan_info(info).error == ConfigureError::invalid_state);
        int fake_handle = 0;
        info.device = reinterpret_cast<VkDevice>(&fake_handle);
        info.instance = reinterpret_cast<VkInstance>(&fake_handle);
        info.physicalDevice = reinterpret_cast<VkPhysicalDevice>(&fake_handle);
        info.computeQueueFamily = 2;
        info.graphicsQueueFamily = 3;
        info.opticalFlowQueueFamily = 4;
        info.opticalFlowQueueIndex = 1;
        assert(session.set_d3d_device(&fake_handle).error == ConfigureError::invalid_state);
        device_result = sl::Result::eErrorVulkanAPI;
        assert(session.set_vulkan_info(info).sdk_result == device_result);
        assert(!session.device_registered());
        device_result = sl::Result::eOk;
        assert(session.set_vulkan_info(info).error == ConfigureError::none);
        assert(session.set_d3d_device(&fake_handle).error == ConfigureError::invalid_state);
    }
    {
        CoreApi core{session_init, session_shutdown, session_device};
        core.set_vulkan_info = vulkan_device;
        Session d3d_session(core);
        assert(d3d_session.initialize(sl::Preferences{}).error == ConfigureError::none);
        sl::VulkanInfo wrong_api{};
        int fake_handle = 0;
        wrong_api.device = reinterpret_cast<VkDevice>(&fake_handle);
        wrong_api.instance = reinterpret_cast<VkInstance>(&fake_handle);
        wrong_api.physicalDevice = reinterpret_cast<VkPhysicalDevice>(&fake_handle);
        assert(d3d_session.set_vulkan_info(wrong_api).error == ConfigureError::invalid_state);
    }
    shutdown_calls = 0;
#endif

    {
        Session session({session_init, session_shutdown, session_device});
        int fake_device = 0;
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::invalid_state);
        assert(session.initialize(sl::Preferences{}).error == ConfigureError::none);
        assert(session.initialize(sl::Preferences{}).error == ConfigureError::invalid_state);
        assert(session.set_d3d_device(nullptr).error == ConfigureError::invalid_state);
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::none);
        assert(session.initialized() && session.device_registered());
        assert(session.set_d3d_device(&fake_device).error == ConfigureError::invalid_state);
    }
    assert(shutdown_calls == 1);
    {
        Session failed({session_init, session_shutdown, session_device});
        init_result = sl::Result::eErrorNoPlugins;
        const auto result = failed.initialize(sl::Preferences{});
        assert(result.error == ConfigureError::sdk_failure && result.sdk_result == init_result);
        assert(!failed.initialized());
    }
    assert(shutdown_calls == 1); // Failed init has no matching shutdown.
    init_result = sl::Result::eOk;
    {
        Session retry({session_init, session_shutdown, session_device});
        assert(retry.initialize(sl::Preferences{}).error == ConfigureError::none);
        int fake_device = 0;
        device_result = sl::Result::eErrorDeviceNotCreated;
        const auto device_failure = retry.set_d3d_device(&fake_device);
        assert(device_failure.sdk_result == device_result);
        assert(!retry.device_registered());
        device_result = sl::Result::eOk;
        assert(retry.set_d3d_device(&fake_device).error == ConfigureError::none);
        shutdown_result = sl::Result::eErrorInvalidParameter;
        const auto close_failure = retry.close();
        assert(close_failure.error == ConfigureError::sdk_failure);
        assert(close_failure.sdk_result == shutdown_result);
        assert(retry.initialized() && retry.device_registered());
        shutdown_result = sl::Result::eOk;
        assert(retry.close().error == ConfigureError::none);
        assert(!retry.initialized() && !retry.device_registered());
        const auto closed_calls = shutdown_calls;
        assert(retry.close().error == ConfigureError::none);
        assert(shutdown_calls == closed_calls);
    }
    assert(shutdown_calls == 3); // Destructor does not repeat successful close.


    ReflexApi frame_api{};
    frame_api.sleep = sleep_frame;
    frame_api.marker = mark_frame;
    ReflexFrame frame(frame_api, test_token);
    assert(frame.mark(sl::PCLMarker::eSimulationStart) == sl::Result::eErrorInvalidParameter);
    assert(frame.sleep() == sl::Result::eOk);
    assert(frame.sleep() == sl::Result::eErrorInvalidParameter);
    assert(frame.mark(sl::PCLMarker::ePresentEnd) == sl::Result::eErrorInvalidParameter);
    assert(marker_calls == 0);
    marker_result = sl::Result::eErrorDeviceNotCreated;
    assert(frame.mark(sl::PCLMarker::eSimulationStart) == marker_result);
    marker_result = sl::Result::eOk;
    const sl::PCLMarker markers[] = {
        sl::PCLMarker::eSimulationStart, sl::PCLMarker::eSimulationEnd,
        sl::PCLMarker::eRenderSubmitStart, sl::PCLMarker::eRenderSubmitEnd,
        sl::PCLMarker::ePresentStart, sl::PCLMarker::ePresentEnd
    };
    for (const auto marker : markers) {
        if (marker == sl::PCLMarker::ePresentEnd) {
            marker_result = sl::Result::eErrorDeviceNotCreated;
            assert(frame.mark(marker) == marker_result);
            assert(!frame.complete());
            const auto calls_before = marker_calls;
            // Failed end marker leaves only end retry legal; do not repeat present.
            assert(frame.mark(sl::PCLMarker::ePresentStart) == sl::Result::eErrorInvalidParameter);
            assert(marker_calls == calls_before);
            marker_result = sl::Result::eOk;
        }
        assert(frame.mark(marker) == sl::Result::eOk);
    }
    assert(frame.complete() && marker_calls == 8);
    assert(frame.mark(sl::PCLMarker::ePresentEnd) == sl::Result::eErrorInvalidParameter);

    sl::FrameToken* token = nullptr;
    unsigned frame_index = 42;
    assert(acquire_frame_token(new_token, &frame_index, token).error == ConfigureError::none);
    assert(token == &test_token);
    assert(acquire_frame_token(new_token, nullptr, token).error == ConfigureError::none);
    token_result = sl::Result::eErrorDeviceNotCreated;
    auto token_failure = acquire_frame_token(new_token, &frame_index, token);
    assert(token_failure.error == ConfigureError::sdk_failure && token_failure.sdk_result == token_result);
    assert(!token);
    token_result = sl::Result::eOk;
    token_null = true;
    assert(acquire_frame_token(new_token, nullptr, token).error == ConfigureError::missing_function);
    assert(!token);
    assert(acquire_frame_token(nullptr, nullptr, token).error == ConfigureError::missing_function);

    ReflexApi reflex{};
    reflex.set_options = reflex_options;
    reflex.get_state = reflex_state;
    assert(configure_reflex(reflex, sl::ReflexMode::eLowLatencyWithBoost, 6944).error == ConfigureError::none);
    assert(configure_reflex(reflex, static_cast<sl::ReflexMode>(99), 0).error == ConfigureError::invalid_mode);
    reflex_available = false;
    assert(configure_reflex(reflex, sl::ReflexMode::eLowLatency, 0).error == ConfigureError::reflex_unavailable);
    reflex_available = true;
    reflex_state_result = sl::Result::eErrorDriverOutOfDate;
    const auto reflex_failure = configure_reflex(reflex, sl::ReflexMode::eLowLatency, 0);
    assert(reflex_failure.error == ConfigureError::sdk_failure);
    assert(reflex_failure.sdk_result == reflex_state_result);
    reflex_state_result = sl::Result::eOk;
    assert(reflex_changes == 1);
    assert(configure_reflex({}, sl::ReflexMode::eOff, 0).error == ConfigureError::missing_function);
    assert(resolve_reflex(nullptr, reflex).error == ConfigureError::missing_function);
    assert(!reflex.get_state && !reflex.set_options && !reflex.sleep && !reflex.marker);

    FrameGenerationApi resolved{};
    assert(resolve_frame_generation(resolve, resolved).error == ConfigureError::none);
    assert(resolved.get_state == get_state && resolved.set_options == set_options);
    fail_second = true;
    auto resolution = resolve_frame_generation(resolve, resolved);
    assert(resolution.error == ConfigureError::sdk_failure);
    assert(resolution.sdk_result == sl::Result::eErrorNoPlugins);
    assert(!resolved.get_state && !resolved.set_options);
    fail_second = false;
    null_second = true;
    assert(resolve_frame_generation(resolve, resolved).error == ConfigureError::missing_function);
    assert(!resolved.get_state && !resolved.set_options);
    assert(resolve_frame_generation(nullptr, resolved).error == ConfigureError::missing_function);

    const FrameGenerationApi api{get_state, set_options};
    sl::DLSSGState snapshot{};
    snapshot.numFramesToGenerateMax = 99;
    assert(query_frame_generation({}, 7, snapshot).error == ConfigureError::missing_function);
    assert(snapshot.numFramesToGenerateMax == 0);
    assert(query_frame_generation(api, 7, snapshot).error == ConfigureError::none);
    assert(snapshot.numFramesToGenerateMax == 5);
    assert(snapshot.bIsDynamicMFGSupported == sl::Boolean::eTrue);
    reported_result = sl::Result::eErrorInvalidParameter;
    assert(query_frame_generation(api, 7, snapshot).sdk_result == reported_result);
    assert(snapshot.numFramesToGenerateMax == 0);
    reported_result = sl::Result::eOk;
    FrameGenerationRequest request{};
    assert(configure_frame_generation(api, 7, request, false).error == ConfigureError::none);
    assert(last_options.mode == sl::DLSSGMode::eOff);
    request.mode = sl::DLSSGMode::eOn;
    request.generated_frames = 5;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::none);
    assert(last_options.numFramesToGenerate == 5);
    const auto before = changes;
    assert(configure_frame_generation(api, 7, request, false).error == ConfigureError::reflex_required);
    request.generated_frames = 6;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::invalid_count);
    reported_result = sl::Result::eErrorDriverOutOfDate;
    const auto failure = configure_frame_generation(api, 7, request, true);
    assert(failure.error == ConfigureError::sdk_failure);
    assert(failure.sdk_result == reported_result);
    reported_result = sl::Result::eOk;
    request.mode = sl::DLSSGMode::eDynamic;
    dynamic_supported = false;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::dynamic_unavailable);
    dynamic_supported = true;
    maximum_generated_frames = 0;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::dynamic_unavailable);
    assert(changes == before);
    maximum_generated_frames = 5;
    request.dynamic_target_fps = std::numeric_limits<float>::infinity();
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::invalid_target);
    assert(changes == before);
    request.dynamic_target_fps = 144.0f;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::none);
    assert(last_options.dynamicTargetFrameRate == 144.0f);
    const auto accepted_options = last_options;
    const auto calls_before_failure = changes;
    options_result = sl::Result::eErrorDriverOutOfDate;
    request.dynamic_target_fps = 240.0f;
    const auto options_failure = configure_frame_generation(api, 7, request, true);
    assert(options_failure.error == ConfigureError::sdk_failure);
    assert(options_failure.sdk_result == options_result);
    assert(changes == calls_before_failure + 1);
    assert(last_options.dynamicTargetFrameRate == accepted_options.dynamicTargetFrameRate);
    options_result = sl::Result::eOk;
    request.dynamic_target_fps = 0.0f;
    assert(configure_frame_generation(api, 7, request, true).error == ConfigureError::none);
    assert(configure_frame_generation({}, 7, request, true).error == ConfigureError::missing_function);
    {
        sl::Preferences options{};
        options.renderAPI = sl::RenderAPI::eD3D12;
        options.flags |= sl::PreferenceFlags::eUseFrameBasedResourceTagging;
        const auto original_flags = static_cast<unsigned>(options.flags);
        const auto manual = static_cast<unsigned>(sl::PreferenceFlags::eUseManualHooking);
        const auto proxy = static_cast<unsigned>(sl::PreferenceFlags::eUseDXGIFactoryProxy);
        assert(configure_dx12_interposition(options, 2));
        assert(static_cast<unsigned>(options.flags) == (original_flags | manual | proxy));
        assert(configure_dx12_interposition(options, 1));
        assert(static_cast<unsigned>(options.flags) == (original_flags | manual));
        assert(!configure_dx12_interposition(options, 3));
        assert(static_cast<unsigned>(options.flags) == (original_flags | manual));
        assert(configure_dx12_interposition(options, 0));
        assert(static_cast<unsigned>(options.flags) == original_flags);
        options.renderAPI = sl::RenderAPI::eVulkan;
        assert(!configure_dx12_interposition(options, 2));
        assert(static_cast<unsigned>(options.flags) == original_flags);
        CoreApi core{session_init, session_shutdown, session_device};
        core.upgrade_interface = upgrade_interface;
        core.get_native_interface = native_interface;
        Session session(core);
        int original = 0;
        void* base_interface = &original;
        assert(session.upgrade_interface(&base_interface).error == ConfigureError::invalid_state);
        sl::Preferences preferences{};
        preferences.renderAPI = sl::RenderAPI::eD3D12;
        init_result = shutdown_result = sl::Result::eOk;
        assert(session.initialize(preferences).error == ConfigureError::none);
        void* native = &original;
        assert(session.native_interface(nullptr, &native).error == ConfigureError::invalid_state);
        assert(native == nullptr);
        assert(session.native_interface(base_interface, &native).error == ConfigureError::none);
        assert(native == &upgraded_object);
        assert(session.upgrade_interface(nullptr).error == ConfigureError::invalid_state);
        assert(upgrade_calls == 0);
        assert(session.upgrade_interface(&base_interface).error == ConfigureError::none);
        assert(base_interface == &upgraded_object && upgrade_calls == 1);
        upgrade_result = sl::Result::eErrorInvalidParameter;
        assert(session.upgrade_interface(&base_interface).sdk_result == upgrade_result);
        assert(upgrade_calls == 2);
        assert(session.native_interface(base_interface, &native).sdk_result == upgrade_result);
        assert(native == nullptr);
        assert(session.close().error == ConfigureError::none);
        assert(session.upgrade_interface(&base_interface).error == ConfigureError::invalid_state);
    }
}

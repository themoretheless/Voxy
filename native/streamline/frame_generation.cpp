#include "frame_generation.hpp"
#include <cmath>

namespace voxy::streamline {
ConfigureResult resolve_frame_generation(
    PFun_slGetFeatureFunction* resolver, FrameGenerationApi& output) {
    output = {};
    if (!resolver) return {ConfigureError::missing_function, sl::Result::eOk};
    void* get_state = nullptr;
    auto result = resolver(sl::kFeatureDLSS_G, "slDLSSGGetState", get_state);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!get_state) return {ConfigureError::missing_function, sl::Result::eOk};
    void* set_options = nullptr;
    result = resolver(sl::kFeatureDLSS_G, "slDLSSGSetOptions", set_options);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!set_options) return {ConfigureError::missing_function, sl::Result::eOk};
    // Streamline's official resolver returns native function pointers as void*.
    output = {reinterpret_cast<PFun_slDLSSGGetState*>(get_state),
              reinterpret_cast<PFun_slDLSSGSetOptions*>(set_options)};
    return {ConfigureError::none, sl::Result::eOk};
}

ConfigureResult query_frame_generation(FrameGenerationApi api, unsigned viewport, sl::DLSSGState& output) {
    output = {};
    if (!api.get_state) return {ConfigureError::missing_function, sl::Result::eOk};
    sl::DLSSGState state{};
    const auto result = api.get_state(sl::ViewportHandle(viewport), state, nullptr);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    output = state;
    return {ConfigureError::none, sl::Result::eOk};
}
ConfigureResult configure_frame_generation(
    FrameGenerationApi api, unsigned viewport,
    const FrameGenerationRequest& request, bool reflex_active) {
    const auto reject = [](ConfigureError error) {
        return ConfigureResult{error, sl::Result::eOk};
    };
    if (!api.get_state || !api.set_options)
        return reject(ConfigureError::missing_function);
    sl::DLSSGOptions options{};
    options.mode = request.mode;
    if (request.mode != sl::DLSSGMode::eOff) {
        if (!reflex_active) return reject(ConfigureError::reflex_required);
        sl::DLSSGState state{};
        const auto result = api.get_state(sl::ViewportHandle(viewport), state, nullptr);
        if (result != sl::Result::eOk)
            return {ConfigureError::sdk_failure, result};
        if (request.mode == sl::DLSSGMode::eOn) {
            if (!request.generated_frames ||
                request.generated_frames > state.numFramesToGenerateMax)
                return reject(ConfigureError::invalid_count);
            options.numFramesToGenerate = request.generated_frames;
        } else if (request.mode == sl::DLSSGMode::eDynamic) {
            if (!state.numFramesToGenerateMax ||
                state.bIsDynamicMFGSupported != sl::Boolean::eTrue)
                return reject(ConfigureError::dynamic_unavailable);
            if (!std::isfinite(request.dynamic_target_fps) || request.dynamic_target_fps < 0.0f)
                return reject(ConfigureError::invalid_target);
            options.dynamicTargetFrameRate = request.dynamic_target_fps;
        } else {
            // Automatic SDK policy is not exposed by the engine request API.
            return reject(ConfigureError::invalid_count);
        }
    }
    const auto result = api.set_options(sl::ViewportHandle(viewport), options);
    return {result == sl::Result::eOk ? ConfigureError::none : ConfigureError::sdk_failure,
            result};
}
}

#include "reflex.hpp"
namespace voxy::streamline {
ConfigureResult resolve_reflex(PFun_slGetFeatureFunction* resolver, ReflexApi& output) {
    output = {};
    if (!resolver) return {ConfigureError::missing_function, sl::Result::eOk};
    const char* names[] = {"slReflexGetState", "slReflexSetOptions", "slReflexSleep", "slPCLSetMarker"};
    void* functions[4]{};
    for (unsigned i = 0; i < 4; ++i) {
        const auto feature = i == 3 ? sl::kFeaturePCL : sl::kFeatureReflex;
        const auto result = resolver(feature, names[i], functions[i]);
        if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
        if (!functions[i]) return {ConfigureError::missing_function, sl::Result::eOk};
    }
    output = {reinterpret_cast<PFun_slReflexGetState*>(functions[0]),
              reinterpret_cast<PFun_slReflexSetOptions*>(functions[1]),
              reinterpret_cast<PFun_slReflexSleep*>(functions[2]),
              reinterpret_cast<PFun_slPCLSetMarker*>(functions[3])};
    return {ConfigureError::none, sl::Result::eOk};
}
ConfigureResult configure_reflex(ReflexApi api, sl::ReflexMode mode, unsigned frame_limit_us) {
    if (!api.set_options) return {ConfigureError::missing_function, sl::Result::eOk};
    if (mode != sl::ReflexMode::eOff && mode != sl::ReflexMode::eLowLatency &&
        mode != sl::ReflexMode::eLowLatencyWithBoost)
        return {ConfigureError::invalid_mode, sl::Result::eOk};
    if (mode != sl::ReflexMode::eOff) {
        if (!api.get_state) return {ConfigureError::missing_function, sl::Result::eOk};
        sl::ReflexState state{};
        const auto result = api.get_state(state);
        if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
        if (!state.lowLatencyAvailable)
            return {ConfigureError::reflex_unavailable, sl::Result::eOk};
    }
    sl::ReflexOptions options{};
    options.mode = mode;
    options.frameLimitUs = frame_limit_us;
    const auto result = api.set_options(options);
    return {result == sl::Result::eOk ? ConfigureError::none : ConfigureError::sdk_failure, result};
}
sl::Result reflex_sleep(ReflexApi api, const sl::FrameToken& token) {
    return api.sleep ? api.sleep(token) : sl::Result::eErrorMissingOrInvalidAPI;
}
sl::Result reflex_marker(ReflexApi api, sl::PCLMarker marker, const sl::FrameToken& token) {
    return api.marker ? api.marker(marker, token) : sl::Result::eErrorMissingOrInvalidAPI;
}
}

namespace voxy::streamline {
sl::Result ReflexFrame::sleep() {
    if (stage_ != 0) return sl::Result::eErrorInvalidParameter;
    const auto result = reflex_sleep(api_, token_);
    if (result == sl::Result::eOk) stage_ = 1;
    return result;
}
sl::Result ReflexFrame::mark(sl::PCLMarker marker) {
    const sl::PCLMarker order[] = {
        sl::PCLMarker::eSimulationStart, sl::PCLMarker::eSimulationEnd,
        sl::PCLMarker::eRenderSubmitStart, sl::PCLMarker::eRenderSubmitEnd,
        sl::PCLMarker::ePresentStart, sl::PCLMarker::ePresentEnd
    };
    if (stage_ < 1 || stage_ > 6 || marker != order[stage_ - 1])
        return sl::Result::eErrorInvalidParameter;
    const auto result = reflex_marker(api_, marker, token_);
    if (result == sl::Result::eOk) ++stage_;
    return result;
}
}

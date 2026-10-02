#include "dlss.hpp"
#include <cmath>
namespace voxy::streamline {
static double component(const sl::float4& row, unsigned column) {
    switch (column) {
        case 0: return row.x;
        case 1: return row.y;
        case 2: return row.z;
        default: return row.w;
    }
}
static bool inverse_pair(const sl::float4x4& a, const sl::float4x4& b) {
    for (unsigned row = 0; row < 4; ++row) {
        for (unsigned column = 0; column < 4; ++column) {
            double value = 0, magnitude = 0;
            for (unsigned k = 0; k < 4; ++k) {
                const double product = component(a[row], k) * component(b[k], column);
                value += product; magnitude += std::abs(product);
            }
            const double expected = row == column ? 1.0 : 0.0;
            if (std::abs(value - expected) > 0.00001 * (1.0 + magnitude)) return false;
        }
    }
    return true;
}
ConfigureResult resolve_ray_reconstruction(PFun_slGetFeatureFunction* resolver, RayReconstructionApi& output) {
    output = {};
    if (!resolver) return {ConfigureError::missing_function, sl::Result::eOk};
    void* optimal = nullptr;
    auto result = resolver(sl::kFeatureDLSS_RR, "slDLSSDGetOptimalSettings", optimal);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    void* set = nullptr;
    result = resolver(sl::kFeatureDLSS_RR, "slDLSSDSetOptions", set);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!optimal || !set) return {ConfigureError::missing_function, sl::Result::eOk};
    output = {reinterpret_cast<PFun_slDLSSDGetOptimalSettings*>(optimal),
        reinterpret_cast<PFun_slDLSSDSetOptions*>(set)};
    return {ConfigureError::none, sl::Result::eOk};
}
ConfigureResult configure_ray_reconstruction(RayReconstructionApi api, unsigned viewport,
    const sl::DLSSDOptions& options, sl::DLSSDOptimalSettings& output) {
    output = {};
    if (options.mode <= sl::DLSSMode::eOff || options.mode >= sl::DLSSMode::eCount ||
        !options.outputWidth || !options.outputHeight || options.outputWidth == sl::INVALID_UINT ||
        options.outputHeight == sl::INVALID_UINT ||
        options.normalRoughnessMode >= sl::DLSSDNormalRoughnessMode::eCount ||
        !std::isfinite(options.preExposure) || options.preExposure <= 0 ||
        !std::isfinite(options.exposureScale) || options.exposureScale <= 0 ||
        options.colorBuffersHDR != sl::Boolean::eTrue)
        return {ConfigureError::invalid_target, sl::Result::eOk};
    for (unsigned i = 0; i < 4; ++i) {
        const auto a = options.worldToCameraView[i];
        const auto b = options.cameraViewToWorld[i];
        if (!std::isfinite(a.x) || !std::isfinite(a.y) || !std::isfinite(a.z) || !std::isfinite(a.w) ||
            !std::isfinite(b.x) || !std::isfinite(b.y) || !std::isfinite(b.z) || !std::isfinite(b.w))
            return {ConfigureError::invalid_target, sl::Result::eOk};
    }
    if (!inverse_pair(options.worldToCameraView, options.cameraViewToWorld) ||
        !inverse_pair(options.cameraViewToWorld, options.worldToCameraView))
        return {ConfigureError::invalid_target, sl::Result::eOk};
    if (!api.optimal || !api.set_options) return {ConfigureError::missing_function, sl::Result::eOk};
    sl::DLSSDOptimalSettings candidate{};
    auto result = api.optimal(options, candidate);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!candidate.renderWidthMin || !candidate.renderHeightMin ||
        candidate.optimalRenderWidth < candidate.renderWidthMin || candidate.optimalRenderWidth > candidate.renderWidthMax ||
        candidate.optimalRenderHeight < candidate.renderHeightMin || candidate.optimalRenderHeight > candidate.renderHeightMax)
        return {ConfigureError::invalid_sdk_output, sl::Result::eOk};
    result = api.set_options(sl::ViewportHandle(viewport), options);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    output = candidate;
    return {ConfigureError::none, sl::Result::eOk};
}
static bool valid(const sl::DLSSOptions& options) {
    return options.mode < sl::DLSSMode::eCount && options.outputWidth > 0 &&
        options.outputHeight > 0 && options.outputWidth != sl::INVALID_UINT &&
        options.outputHeight != sl::INVALID_UINT && std::isfinite(options.preExposure) &&
        options.preExposure > 0 && std::isfinite(options.exposureScale) && options.exposureScale > 0 &&
        (options.colorBuffersHDR == sl::Boolean::eTrue || options.colorBuffersHDR == sl::Boolean::eFalse);
}
ConfigureResult resolve_dlss(PFun_slGetFeatureFunction* resolver, DlssApi& output) {
    output = {};
    if (!resolver) return {ConfigureError::missing_function, sl::Result::eOk};
    void* optimal = nullptr;
    auto result = resolver(sl::kFeatureDLSS, "slDLSSGetOptimalSettings", optimal);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    void* set = nullptr;
    result = resolver(sl::kFeatureDLSS, "slDLSSSetOptions", set);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!optimal || !set) return {ConfigureError::missing_function, sl::Result::eOk};
    output = {reinterpret_cast<PFun_slDLSSGetOptimalSettings*>(optimal),
              reinterpret_cast<PFun_slDLSSSetOptions*>(set)};
    return {ConfigureError::none, sl::Result::eOk};
}
ConfigureResult configure_dlss(DlssApi api, unsigned viewport, const sl::DLSSOptions& options) {
    if (!valid(options)) return {ConfigureError::invalid_target, sl::Result::eOk};
    if (!api.set_options) return {ConfigureError::missing_function, sl::Result::eOk};
    const auto result = api.set_options(sl::ViewportHandle(viewport), options);
    return {result == sl::Result::eOk ? ConfigureError::none : ConfigureError::sdk_failure, result};
}
ConfigureResult optimal_dlss_settings(DlssApi api, const sl::DLSSOptions& options,
    sl::DLSSOptimalSettings& output) {
    output = {};
    if (!valid(options) || options.mode == sl::DLSSMode::eOff)
        return {ConfigureError::invalid_target, sl::Result::eOk};
    if (!api.optimal) return {ConfigureError::missing_function, sl::Result::eOk};
    sl::DLSSOptimalSettings candidate{};
    const auto result = api.optimal(options, candidate);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!candidate.renderWidthMin || !candidate.renderHeightMin ||
        candidate.optimalRenderWidth < candidate.renderWidthMin ||
        candidate.optimalRenderWidth > candidate.renderWidthMax ||
        candidate.optimalRenderHeight < candidate.renderHeightMin ||
        candidate.optimalRenderHeight > candidate.renderHeightMax)
        return {ConfigureError::invalid_sdk_output, sl::Result::eOk};
    output = candidate;
    return {ConfigureError::none, sl::Result::eOk};
}
}

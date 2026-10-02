#include "session.hpp"
namespace voxy::streamline {
bool configure_dx12_interposition(sl::Preferences& preferences, unsigned mode) {
    if (mode > 2 || preferences.renderAPI != sl::RenderAPI::eD3D12) return false;
    const auto manual = static_cast<unsigned>(sl::PreferenceFlags::eUseManualHooking);
    const auto proxy = static_cast<unsigned>(sl::PreferenceFlags::eUseDXGIFactoryProxy);
    auto flags = static_cast<unsigned>(preferences.flags) & ~(manual | proxy);
    if (mode != 0) flags |= manual;
    if (mode == 2) flags |= proxy;
    preferences.flags = static_cast<sl::PreferenceFlags>(flags);
    return true;
}
static ConfigureResult sdk_result(sl::Result result) {
    return {result == sl::Result::eOk ? ConfigureError::none : ConfigureError::sdk_failure, result};
}
Session::~Session() { if (initialized_) close(); }
ConfigureResult Session::initialize(const sl::Preferences& preferences) {
    if (initialized_) return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.init || !api_.shutdown) return {ConfigureError::missing_function, sl::Result::eOk};
    const auto result = api_.init(preferences, sl::kSDKVersion);
    initialized_ = result == sl::Result::eOk;
    if (initialized_) {
        render_api_ = preferences.renderAPI;
        frame_tagging_ = (static_cast<unsigned>(preferences.flags) &
            static_cast<unsigned>(sl::PreferenceFlags::eUseFrameBasedResourceTagging)) != 0;
    }
    return sdk_result(result);
}
ConfigureResult Session::set_d3d_device(void* device) {
    if (!initialized_ || device_registered_ || !device ||
        (render_api_ != sl::RenderAPI::eD3D11 && render_api_ != sl::RenderAPI::eD3D12))
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.set_d3d_device) return {ConfigureError::missing_function, sl::Result::eOk};
    const auto result = api_.set_d3d_device(device);
    device_registered_ = result == sl::Result::eOk;
    return sdk_result(result);
}
ConfigureResult Session::upgrade_interface(void** base_interface) {
    if (!initialized_ || !base_interface || !*base_interface ||
        (render_api_ != sl::RenderAPI::eD3D11 && render_api_ != sl::RenderAPI::eD3D12))
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.upgrade_interface) return {ConfigureError::missing_function, sl::Result::eOk};
    return sdk_result(api_.upgrade_interface(base_interface));
}
ConfigureResult Session::native_interface(void* proxy, void** output) {
    if (output) *output = nullptr;
    if (!initialized_ || !proxy || !output ||
        (render_api_ != sl::RenderAPI::eD3D11 && render_api_ != sl::RenderAPI::eD3D12))
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.get_native_interface) return {ConfigureError::missing_function, sl::Result::eOk};
    const auto result = api_.get_native_interface(proxy, output);
    if (result != sl::Result::eOk) *output = nullptr;
    return sdk_result(result);
}
ConfigureResult Session::close() {
    if (!initialized_) return {ConfigureError::none, sl::Result::eOk};
    const auto result = api_.shutdown();
    if (result == sl::Result::eOk) {
        initialized_ = false;
        device_registered_ = false;
        render_api_ = sl::RenderAPI::eCount;
        frame_tagging_ = false;
    }
    return sdk_result(result);
}
}

#ifdef VOXY_STREAMLINE_VULKAN
namespace voxy::streamline {
ConfigureResult Session::set_vulkan_info(const sl::VulkanInfo& info) {
    if (!initialized_ || device_registered_ || render_api_ != sl::RenderAPI::eVulkan ||
        !info.device || !info.instance || !info.physicalDevice)
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.set_vulkan_info) return {ConfigureError::missing_function, sl::Result::eOk};
    const auto result = api_.set_vulkan_info(info);
    device_registered_ = result == sl::Result::eOk;
    return sdk_result(result);
}
}
#endif

namespace voxy::streamline {
ConfigureResult Session::feature_support(sl::Feature feature, const sl::AdapterInfo& adapter) const {
    if (!initialized_) return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.is_feature_supported) return {ConfigureError::missing_function, sl::Result::eOk};
    if (render_api_ == sl::RenderAPI::eVulkan) {
        if (!adapter.vkPhysicalDevice) return {ConfigureError::invalid_state, sl::Result::eOk};
    } else if (!adapter.deviceLUID || adapter.deviceLUIDSizeInBytes != 8 || adapter.vkPhysicalDevice) {
        return {ConfigureError::invalid_state, sl::Result::eOk};
    }
    return sdk_result(api_.is_feature_supported(feature, adapter));
}
}

namespace voxy::streamline {
ConfigureResult Session::set_constants(const sl::Constants& constants, const sl::FrameToken& token, unsigned viewport) {
    if (!initialized_ || !device_registered_) return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.set_constants) return {ConfigureError::missing_function, sl::Result::eOk};
    return sdk_result(api_.set_constants(constants, token, sl::ViewportHandle(viewport)));
}
}

namespace voxy::streamline {
ConfigureResult Session::tag_resources(const sl::FrameToken& token, unsigned viewport,
    const sl::ResourceTag* tags, unsigned count, sl::CommandBuffer* command_buffer) {
    if (!initialized_ || !device_registered_ || !frame_tagging_ || !tags || !count)
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.set_tags) return {ConfigureError::missing_function, sl::Result::eOk};
    if (!command_buffer) {
        for (unsigned i = 0; i < count; ++i) {
            if (tags[i].resource && tags[i].lifecycle != sl::ResourceLifecycle::eValidUntilPresent)
                return {ConfigureError::invalid_state, sl::Result::eOk};
        }
    }
    return sdk_result(api_.set_tags(token, sl::ViewportHandle(viewport), tags, count, command_buffer));
}
}

namespace voxy::streamline {
ConfigureResult Session::evaluate(sl::Feature feature, const sl::FrameToken& token,
    const sl::BaseStructure** inputs, unsigned count, sl::CommandBuffer* command_buffer) {
    if (!initialized_ || !device_registered_ || !inputs || !count || !command_buffer)
        return {ConfigureError::invalid_state, sl::Result::eOk};
    if (!api_.evaluate) return {ConfigureError::missing_function, sl::Result::eOk};
    for (unsigned i = 0; i < count; ++i) {
        if (!inputs[i]) return {ConfigureError::invalid_state, sl::Result::eOk};
    }
    return sdk_result(api_.evaluate(feature, token, inputs, count, command_buffer));
}
}

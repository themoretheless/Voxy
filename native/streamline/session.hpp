#pragma once
#include "frame_generation.hpp"
#ifdef VOXY_STREAMLINE_VULKAN
#include <vulkan/vulkan.h>
#include <sl_helpers_vk.h>
#endif
namespace voxy::streamline {
// Apply DX12 interception flags without discarding other SDK preferences.
// Invalid modes leave preferences unchanged.
bool configure_dx12_interposition(sl::Preferences& preferences, unsigned mode);
struct CoreApi {
    PFun_slInit* init = nullptr;
    PFun_slShutdown* shutdown = nullptr;
    PFun_slSetD3DDevice* set_d3d_device = nullptr;
#ifdef VOXY_STREAMLINE_VULKAN
    PFun_slSetVulkanInfo* set_vulkan_info = nullptr;
#endif
    PFun_slIsFeatureSupported* is_feature_supported = nullptr;
    PFun_slSetConstants* set_constants = nullptr;
    PFun_slSetTagForFrame* set_tags = nullptr;
    PFun_slEvaluateFeature* evaluate = nullptr;
    PFun_slUpgradeInterface* upgrade_interface = nullptr;
    PFun_slGetNativeInterface* get_native_interface = nullptr;
};
// One process-global SDK session. Keep the loaded library alive until destruction.
// Initialize before graphics-device creation, as required by Streamline interposition.
class Session {
public:
    explicit Session(CoreApi api) : api_(api) {}
    ~Session();
    Session(const Session&) = delete;
    Session& operator=(const Session&) = delete;
    ConfigureResult initialize(const sl::Preferences& preferences);
    ConfigureResult set_d3d_device(void* device);
    // Manual hooking only: call immediately after creating a D3D/DXGI base_interface.
    // Caller owns COM replacement/refcount semantics, including SDK failures.
    ConfigureResult upgrade_interface(void** base_interface);
    // Returned COM ownership follows the SDK contract; no AddRef is added here.
    ConfigureResult native_interface(void* proxy, void** output);
    // Success means SDK-supported; other results preserve the precise SDK reason.
    ConfigureResult feature_support(sl::Feature feature, const sl::AdapterInfo& adapter) const;
#ifdef VOXY_STREAMLINE_VULKAN
    ConfigureResult set_vulkan_info(const sl::VulkanInfo& info);
#endif
    ConfigureResult set_constants(const sl::Constants& constants, const sl::FrameToken& token, unsigned viewport);
    ConfigureResult tag_resources(const sl::FrameToken& token, unsigned viewport,
        const sl::ResourceTag* tags, unsigned count, sl::CommandBuffer* command_buffer);
    ConfigureResult evaluate(sl::Feature feature, const sl::FrameToken& token,
        const sl::BaseStructure** inputs, unsigned count, sl::CommandBuffer* command_buffer);
    // Explicit close lets the caller inspect a shutdown failure; do not unload then.
    ConfigureResult close();
    bool initialized() const { return initialized_; }
    bool device_registered() const { return device_registered_; }
private:
    CoreApi api_;
    sl::RenderAPI render_api_ = sl::RenderAPI::eCount;
    bool frame_tagging_ = false;
    bool initialized_ = false;
    bool device_registered_ = false;
};
}

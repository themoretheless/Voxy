#ifdef _WIN32
#include "windows_module.hpp"
#include <sl_security.h>
#include <filesystem>
#include <cstring>
namespace voxy::streamline {
template<typename Function> static Function* export_function(HMODULE module, const char* name) {
    const auto address = GetProcAddress(module, name);
    Function* function = nullptr;
    static_assert(sizeof(function) == sizeof(address));
    std::memcpy(&function, &address, sizeof(function));
    return function;
}
WindowsModule::~WindowsModule() { if (module_ && unload_) FreeLibrary(module_); }
ModuleError WindowsModule::open(const std::wstring& path) {
    if (module_) return ModuleError::already_loaded;
    last_error_ = 0;
    if (path.find(L'\0') != std::wstring::npos || !std::filesystem::path(path).is_absolute())
        return ModuleError::invalid_path;
    if (!sl::security::verifyEmbeddedSignature(path.c_str())) return ModuleError::signature;
    const auto candidate = LoadLibraryExW(path.c_str(), nullptr,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!candidate) { last_error_ = GetLastError(); return ModuleError::load; }
    CoreApi core{};
    core.init = export_function<PFun_slInit>(candidate, "slInit");
    core.shutdown = export_function<PFun_slShutdown>(candidate, "slShutdown");
    core.set_d3d_device = export_function<PFun_slSetD3DDevice>(candidate, "slSetD3DDevice");
    core.get_native_interface = export_function<PFun_slGetNativeInterface>(candidate, "slGetNativeInterface");
    core.upgrade_interface = export_function<PFun_slUpgradeInterface>(candidate, "slUpgradeInterface");
    core.is_feature_supported = export_function<PFun_slIsFeatureSupported>(candidate, "slIsFeatureSupported");
#ifdef VOXY_STREAMLINE_VULKAN
    core.set_vulkan_info = export_function<PFun_slSetVulkanInfo>(candidate, "slSetVulkanInfo");
#endif
    core.set_constants = export_function<PFun_slSetConstants>(candidate, "slSetConstants");
    core.set_tags = export_function<PFun_slSetTagForFrame>(candidate, "slSetTagForFrame");
    core.evaluate = export_function<PFun_slEvaluateFeature>(candidate, "slEvaluateFeature");
    const auto resolver = export_function<PFun_slGetFeatureFunction>(candidate, "slGetFeatureFunction");
    const auto tokens = export_function<PFun_slGetNewFrameToken>(candidate, "slGetNewFrameToken");
    if (!core.init || !core.shutdown || !core.set_d3d_device || !core.is_feature_supported ||
        !resolver || !tokens || !core.set_constants || !core.set_tags || !core.evaluate
#ifdef VOXY_STREAMLINE_VULKAN
        || !core.set_vulkan_info
#endif
    ) {
        last_error_ = ERROR_PROC_NOT_FOUND;
        FreeLibrary(candidate);
        return ModuleError::missing_export;
    }
    module_ = candidate;
    core_ = core;
    resolver_ = resolver;
    tokens_ = tokens;
    return ModuleError::none;
}
}
#endif

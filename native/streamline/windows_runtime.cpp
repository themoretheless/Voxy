#ifdef _WIN32
#include "windows_runtime.hpp"
namespace voxy::streamline {
WindowsRuntime::~WindowsRuntime() {
    if (session_ && session_->initialized()) {
        if (session_->close().error != ConfigureError::none) {
            // Outstanding SDK state may reference DLL code. Preserve the OS module
            // reference for process lifetime rather than unloading on failed cleanup.
            module_.retain_until_process_exit();
        }
    }
    session_.reset();
}
ModuleError WindowsRuntime::load(const std::wstring& path) {
    if (session_) return ModuleError::already_loaded;
    const auto result = module_.open(path);
    if (result != ModuleError::none) return result;
    session_ = std::make_unique<Session>(module_.core());
    return ModuleError::none;
}
PFun_slGetFeatureFunction* WindowsRuntime::resolver() const {
    return session_ && session_->device_registered() ? module_.resolver() : nullptr;
}
PFun_slGetNewFrameToken* WindowsRuntime::frame_tokens() const {
    return session_ && session_->device_registered() ? module_.frame_tokens() : nullptr;
}
}
#endif

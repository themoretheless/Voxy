#pragma once
#ifdef _WIN32
#include "session.hpp"
#include <windows.h>
#include <string>
namespace voxy::streamline {
enum class ModuleError { none, invalid_path, already_loaded, signature, load, missing_export };
// Must outlive Session, feature tables and all SDK tokens. Close Session first.
class WindowsModule {
public:
    WindowsModule() = default;
    ~WindowsModule();
    WindowsModule(const WindowsModule&) = delete;
    WindowsModule& operator=(const WindowsModule&) = delete;
    ModuleError open(const std::wstring& absolute_path);
    CoreApi core() const { return core_; }
    PFun_slGetFeatureFunction* resolver() const { return resolver_; }
    PFun_slGetNewFrameToken* frame_tokens() const { return tokens_; }
    DWORD last_windows_error() const { return last_error_; }
private:
    friend class WindowsRuntime;
    void retain_until_process_exit() { unload_ = false; }
    bool unload_ = true;
    HMODULE module_ = nullptr;
    CoreApi core_{};
    PFun_slGetFeatureFunction* resolver_ = nullptr;
    PFun_slGetNewFrameToken* tokens_ = nullptr;
    DWORD last_error_ = 0;
};
}
#endif

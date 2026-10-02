#pragma once
#ifdef _WIN32
#include "windows_module.hpp"
#include <memory>
namespace voxy::streamline {
// Owns loader and SDK session in destruction order. Feature tables/tokens borrow it.
class WindowsRuntime {
public:
    WindowsRuntime() = default;
    ~WindowsRuntime();
    WindowsRuntime(const WindowsRuntime&) = delete;
    WindowsRuntime& operator=(const WindowsRuntime&) = delete;
    ModuleError load(const std::wstring& absolute_path);
    Session* session() { return session_.get(); }
    PFun_slGetFeatureFunction* resolver() const;
    PFun_slGetNewFrameToken* frame_tokens() const;
    DWORD last_windows_error() const { return module_.last_windows_error(); }
private:
    WindowsModule module_;
    std::unique_ptr<Session> session_;
};
}
#endif

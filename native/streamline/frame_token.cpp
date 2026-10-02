#include "frame_token.hpp"
namespace voxy::streamline {
ConfigureResult acquire_frame_token(
    PFun_slGetNewFrameToken* acquire, const unsigned* frame_index,
    sl::FrameToken*& output) {
    output = nullptr;
    if (!acquire) return {ConfigureError::missing_function, sl::Result::eOk};
    sl::FrameToken* candidate = nullptr;
    const auto result = acquire(candidate, frame_index);
    if (result != sl::Result::eOk) return {ConfigureError::sdk_failure, result};
    if (!candidate) return {ConfigureError::missing_function, sl::Result::eOk};
    output = candidate;
    return {ConfigureError::none, sl::Result::eOk};
}
}

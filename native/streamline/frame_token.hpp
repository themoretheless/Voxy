#pragma once
#include "frame_generation.hpp"
namespace voxy::streamline {
// SDK owns token storage. Do not delete or retain it past SDK lifetime.
// Call on the integration's frame thread; pass the same token to every feature.
ConfigureResult acquire_frame_token(
    PFun_slGetNewFrameToken* acquire, const unsigned* frame_index,
    sl::FrameToken*& output);
}

#pragma once
#include "frame_generation.hpp"
#include <sl_reflex.h>
#include <sl_pcl.h>
namespace voxy::streamline {
struct ReflexApi {
    PFun_slReflexGetState* get_state = nullptr;
    PFun_slReflexSetOptions* set_options = nullptr;
    PFun_slReflexSleep* sleep = nullptr;
    PFun_slPCLSetMarker* marker = nullptr;
};
ConfigureResult resolve_reflex(PFun_slGetFeatureFunction* resolver, ReflexApi& output);
// Enabling queries lowLatencyAvailable first. Success means SDK-accepted options,
// not proof of marker placement, sleep execution or measured low latency.
ConfigureResult configure_reflex(ReflexApi api, sl::ReflexMode mode, unsigned frame_limit_us);
// Obtain tokens from slGetNewFrameToken; never construct a fake token from a counter.
sl::Result reflex_sleep(ReflexApi api, const sl::FrameToken& token);
sl::Result reflex_marker(ReflexApi api, sl::PCLMarker marker, const sl::FrameToken& token);
}

namespace voxy::streamline {
// One rendered frame. SDK token and runtime must outlive this object.
// Methods are explicit so markers surround actual engine work, not a batch call.
class ReflexFrame {
public:
    ReflexFrame(ReflexApi api, const sl::FrameToken& token) : api_(api), token_(token) {}
    ReflexFrame(const ReflexFrame&) = delete;
    ReflexFrame& operator=(const ReflexFrame&) = delete;
    sl::Result sleep();
    sl::Result mark(sl::PCLMarker marker);
    bool complete() const { return stage_ == 7; }
private:
    ReflexApi api_;
    const sl::FrameToken& token_;
    unsigned stage_ = 0;
};
}

#pragma once
#include "frame_generation.hpp"
#include <sl_dlss.h>
#include <sl_dlss_d.h>
namespace voxy::streamline {
struct DlssApi {
    PFun_slDLSSGetOptimalSettings* optimal = nullptr;
    PFun_slDLSSSetOptions* set_options = nullptr;
};
struct RayReconstructionApi {
    PFun_slDLSSDGetOptimalSettings* optimal = nullptr;
    PFun_slDLSSDSetOptions* set_options = nullptr;
};
ConfigureResult resolve_ray_reconstruction(PFun_slGetFeatureFunction* resolver, RayReconstructionApi& output);
ConfigureResult configure_ray_reconstruction(RayReconstructionApi api, unsigned viewport,
    const sl::DLSSDOptions& options, sl::DLSSDOptimalSettings& output);
ConfigureResult resolve_dlss(PFun_slGetFeatureFunction* resolver, DlssApi& output);
ConfigureResult configure_dlss(DlssApi api, unsigned viewport, const sl::DLSSOptions& options);
ConfigureResult optimal_dlss_settings(DlssApi api, const sl::DLSSOptions& options,
    sl::DLSSOptimalSettings& output);
}

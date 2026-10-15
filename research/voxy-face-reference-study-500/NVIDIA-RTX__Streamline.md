# Facial rendering source scan: NVIDIA-RTX/Streamline

Repository: https://github.com/NVIDIA-RTX/Streamline

Pinned source revision: 2122257e0fce486f91b385aa63b9a09b0a34b363

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["external/json/LICENSE.MIT", "external/ngx-sdk/license.txt", "external/nsight-sdk/SystemsGraphics/LICENSE.txt", "license.txt"]

Topic paths: {"shader": ["external/dx-agility-sdk-headers-1.714.0-preview/include/d3d12shader.h", "external/nvapi/nvShaderExtnEnums.h", "shaders/copy.hlsl", "shaders/copy_to_buffer.hlsl", "shaders/copy_to_buffer_cs.h", "shaders/copy_to_buffer_spv.h", "shaders/mvec.hlsl", "shaders/vulkan_clear_image_view.comp", "shaders/vulkan_clear_image_view_spirv.h", "source/plugins/sl.nis/NIS_shaders.h", "tools/build_shader.bat"], "dlss": ["docs/DLSS Programming Guide.pdf", "docs/DLSS-FG Programming Guide.pdf", "docs/DLSS-RR Integration Guide.pdf", "docs/ProgrammingGuideDLSS.md", "docs/ProgrammingGuideDLSS_G.md", "docs/ProgrammingGuideDLSS_RR.md", "docs/media/dlssg_docs_bidirectional_distortion_field.png", "docs/media/dlssg_docs_bidirectional_distortion_field_rg_abs.png", "docs/media/dlssg_docs_depth.png", "docs/media/dlssg_docs_final_color.png", "docs/media/dlssg_docs_final_color_subrect.png", "docs/media/dlssg_docs_hudless.png", "docs/media/dlssg_docs_mvec.png", "docs/media/dlssg_docs_overview.png", "docs/media/dlssg_docs_ui_alpha.png", "docs/media/dlssg_docs_ui_color_and_alpha.png", "docs/media/sl_imgui_dlss.png", "docs/media/sl_imgui_dlssg.png", "docs/media/sl_imgui_dlssg_buffer_alignment.png", "docs/media/sl_imgui_dlssg_dynamic_objs.png", "external/ngx-sdk/include/nvsdk_ngx_defs_dlssd.h", "external/ngx-sdk/include/nvsdk_ngx_helpers_dlssd.h", "external/ngx-sdk/include/nvsdk_ngx_helpers_dlssd_d3d.h", "external/ngx-sdk/include/nvsdk_ngx_helpers_dlssd_vk.h", "external/ngx-sdk/include/nvsdk_ngx_params_dlssd.h", "include/sl_dlss.h", "include/sl_dlss_d.h", "include/sl_dlss_g.h", "plugins/streamline-agent-skills/skills/authoring-dlss-agent-skills/SKILL.md", "plugins/streamline-agent-skills/skills/integrate-dlss-fg/SKILL.md"], "face": ["external/nvapi/nvapi_interface.h", "source/plugins/sl.common/commonDRSInterface.h", "source/plugins/sl.common/commonInterface.cpp", "source/plugins/sl.common/commonInterface.h"]}

## Source file: include/sl_dlss.h

{
  "path": "include/sl_dlss.h",
  "blob_sha": "9a875bd4f843fdd594ee1afd01eef8060b75b0f4",
  "sha256": "d2c8c61fa71794c8ba424cebce5b3079ebfab8e47ed98b4b00cf8b257ecfd6b5",
  "lines": 193,
  "bytes": 8489,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 7,
    "dlss": 35,
    "rig": 4
  },
  "url": "https://github.com/NVIDIA-RTX/Streamline/blob/2122257e0fce486f91b385aa63b9a09b0a34b363/include/sl_dlss.h",
  "function_signature_candidates": []
}

```text

/*
* Copyright (c) 2022-2023 NVIDIA CORPORATION. All rights reserved
*
* Permission is hereby granted, free of charge, to any person obtaining a copy
* of this software and associated documentation files (the "Software"), to deal
* in the Software without restriction, including without limitation the rights
* to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
* copies of the Software, and to permit persons to whom the Software is
* furnished to do so, subject to the following conditions:
*
* The above copyright notice and this permission notice shall be included in all
* copies or substantial portions of the Software.
*
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
* IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
* FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
* AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
* LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
* OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
* SOFTWARE.
*/ 

#pragma once

#if __cplusplus >= 201402L
#define SR_DEPRECATED_SHARPENING [[deprecated("Sharpness is not supported")]]
#else
#define SR_DEPRECATED_SHARPENING
#endif

namespace sl
{

enum class DLSSMode : uint32_t
{
    eOff,
    eMaxPerformance,
    eBalanced,
    eMaxQuality,
    eUltraPerformance,
    eUltraQuality,
    eDLAA,
    eCount,
};

enum class DLSSPreset : uint32_t
{
    //! Default behavior, may or may not change after an OTA
    eDefault,
    //! Fixed DL models
                    // ePresetA removed, use presets J or K
                    // ePresetB removed, use presets J or K
                    // ePresetC removed, use presets J or K
                    // ePresetD removed, use presets J or K
    ePresetE = 5,   // Deprecated
    ePresetF = 6,   // Deprecated
    ePresetG = 7,   // Reverts to default, not recommended to use
    ePresetH = 8,   // Reverts to default, not recommended to use
    ePresetI = 9,   // Reverts to default, not recommended to use
    ePresetJ = 10,  // Similar to preset K. Preset J might exhibit slightly less ghosting at the cost of extra flickering. Preset K is generally recommended over preset J
    ePresetK = 11,  // Default preset for DLAA/Balanced/Quality modes that is transformer based. Best image quality preset at a higher performance cost
    ePresetL = 12,  // Default preset for UltraPerformance mode. Delivers a sharper, more stable image with less ghosting than Preset J, K but are more expensive performance wise
    ePresetM = 13,  //  Default preset for Performance mode. Delivers similar image quality improvements as Preset L but closer in speed to Presets J, K
    ePresetN = 14,  // Reverts to default, not recommended to use
    ePresetO = 15,  // Reverts to default, not recommended to use

    eCount
};

// {6AC826E4-4C61-4101-A92D-638D421057B8}
SL_STRUCT_BEGIN(DLSSOptions, StructType({ 0x6ac826e4, 0x4c61, 0x4101, { 0xa9, 0x2d, 0x63, 0x8d, 0x42, 0x10, 0x57, 0xb8 } }), kStructVersion3)
    //! Specifies which mode should be used
    DLSSMode mode = DLSSMode::eOff;
    //! Specifies output (final) target width
    uint32_t outputWidth = INVALID_UINT;
    //! Specifies output (final) target height
    uint32_t outputHeight = INVALID_UINT;
    //! Specifies sharpening level in range [0,1] this is a deprecated field
    float sharpness SR_DEPRECATED_SHARPENING = 0.0f;
    //! Specifies pre-exposure value
    float preExposure = 1.0f;
    //! Specifies exposure scale value
    float exposureScale = 1.0f;
    //! Specifies if tagged color buffers are full HDR or not (DLSS in HDR pipeline or not)
    Boolean colorBuffersHDR = Boolean::eTrue;
    //! Specifies if indicator on screen should invert axis
    Boolean indicatorInvertAxisX = Boolean::eFalse;
    //! Specifies if indicator on screen should invert axis
    Boolean indicatorInvertAxisY = Boolean::eFalse;
    //! Presets
    DLSSPreset dlaaPreset = DLSSPreset::eDefault;
    DLSSPreset qualityPreset = DLSSPreset::eDefault;
    DLSSPreset balancedPreset = DLSSPreset::eDefault;
    DLSSPreset performancePreset = DLSSPreset::eDefault;
    DLSSPreset ultraPerformancePreset = DLSSPreset::eDefault;
    DLSSPreset ultraQualityPreset = DLSSPreset::eDefault;

    //! Specifies if the setting for AutoExposure is used
    Boolean useAutoExposure = Boolean::eFalse;

    //! Whether or not the alpha channel should be upscaled (if false, only RGB is upscaled)
    //! Enabling alpha upscaling may impact performance
    Boolean alphaUpscalingEnabled = Boolean::eFalse;

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

//! Returned by DLSS plugin
//! 
//! {EF1D0957-FD58-4DF7-B504-8B69D8AA6B76}
SL_STRUCT_BEGIN(DLSSOptimalSettings, StructType({ 0xef1d0957, 0xfd58, 0x4df7, { 0xb5, 0x4, 0x8b, 0x69, 0xd8, 0xaa, 0x6b, 0x76 } }), kStructVersion1)
    //! Specifies render area width
    uint32_t optimalRenderWidth{};
    //! Specifies render area height
    uint32_t optimalRenderHeight{};
    //! Specifies the optimal sharpness value
    float optimalSharpness{};
    //! Specifies minimal render area width
    uint32_t renderWidthMin{};
    //! Specifies minimal render area height
    uint32_t renderHeightMin{};
    //! Specifies maximal render area width
    uint32_t renderWidthMax{};
    //! Specifies maximal render area height
    uint32_t renderHeightMax{};

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

//! Returned by DLSS plugin
//! 
//! {9366B056-8C01-463C-BB91-E68782636CE9}
SL_STRUCT_BEGIN(DLSSState, StructType({ 0x9366b056, 0x8c01, 0x463c, { 0xbb, 0x91, 0xe6, 0x87, 0x82, 0x63, 0x6c, 0xe9 } }), kStructVersion1)
    //! Specified the amount of memory expected to be used
    uint64_t estimatedVRAMUsageInBytes{};

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

}

//! Provides optimal DLSS settings
//!
//! Call this method to obtain optimal render target size and other DLSS related settings.
//!
//! @param options Specifies DLSS options to use
//! @param settings Reference to a structure where settings are returned
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSGetOptimalSettings = sl::Result(const sl::DLSSOptions & options, sl::DLSSOptimalSettings & settings);

//! Provides DLSS state for the given viewport
//!
//! Call this method to obtain optimal render target size and other DLSS related settings.
//!
//! @param viewport Specified viewport we are working with
//! @param state Reference to a structure where state is to be returned
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSGetState = sl::Result(const sl::ViewportHandle & viewport, sl::DLSSState & state);

//! Sets DLSS options
//!
//! Call this method to turn DLSS on/off, change mode etc.
//!
//! @param viewport Specified viewport we are working with
//! @param options Specifies DLSS options to use
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSSetOptions = sl::Result(const sl::ViewportHandle& viewport, const sl::DLSSOptions& options);

//! HELPERS
//! 
inline sl::Result slDLSSGetOptimalSettings(const sl::DLSSOptions& options, sl::DLSSOptimalSettings& settings)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS, slDLSSGetOptimalSettings);
    return s_slDLSSGetOptimalSettings(options, settings);
}

inline sl::Result slDLSSGetState(const sl::ViewportHandle& viewport, sl::DLSSState& state)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS, slDLSSGetState);
    return s_slDLSSGetState(viewport, state);
}

inline sl::Result slDLSSSetOptions(const sl::ViewportHandle& viewport, const sl::DLSSOptions& options)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS, slDLSSSetOptions);
    return s_slDLSSSetOptions(viewport, options);
}


```

## Source file: shaders/copy.hlsl

{
  "path": "shaders/copy.hlsl",
  "blob_sha": "5d5f4709fd674a9167e65df0b19a3b78a817ef96",
  "sha256": "cad98024a0c2f480c307f540564bd8079d729bda2a799782a4ad097fce993417",
  "lines": 22,
  "bytes": 454,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2
  },
  "url": "https://github.com/NVIDIA-RTX/Streamline/blob/2122257e0fce486f91b385aa63b9a09b0a34b363/shaders/copy.hlsl",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "void main( uint3 DTid : SV_DispatchThreadID)"
    }
  ]
}

```text

[[vk::binding(0)]] cbuffer shaderConsts : register(b0)
{
  float4 sizeAndInvSize; 
}
    
[[vk::binding(1)]] Texture2D<float4>  gSrc    : register(t0);
[[vk::binding(2)]] RWTexture2D<float4> outDst : register(u0);

[shader("compute")]
[numthreads(16, 16, 1)]
void main( uint3 DTid : SV_DispatchThreadID)
{
  uint2 pos = DTid.xy;
  if ( any ( pos >= (uint2)sizeAndInvSize.xy ) )
  {
    return;
  }

  outDst[pos] = gSrc[pos];  
}




```

## Source file: shaders/mvec.hlsl

{
  "path": "shaders/mvec.hlsl",
  "blob_sha": "53185c0cde652611c1fd8b46ee145fed28011557",
  "sha256": "fc4f837d8f7d748c0487cf2b168e78aaf18ef874da2562993a527c40ffcda848",
  "lines": 43,
  "bytes": 1496,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 2
  },
  "url": "https://github.com/NVIDIA-RTX/Streamline/blob/2122257e0fce486f91b385aa63b9a09b0a34b363/shaders/mvec.hlsl",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "void main(uint3 DTid : SV_DispatchThreadID)"
    }
  ]
}

```text

[[vk::binding(0)]] Texture2D<float4> texMVec : register(t0);
[[vk::binding(1)]] Texture2D<float4> texDepth : register(t1);
[[vk::binding(2)]] RWTexture2D<float2> rwtexMVec : register(u0);

[[vk::binding(3)]] cbuffer shaderConsts : register(b0)
{
    float4x4 currentToPreviousClipMatrixNoOffset;
    float4 textureSize;
    float2 mvecScale;
    uint showMvec;
};

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 DTid : SV_DispatchThreadID)
{
    uint2 pixelId = DTid.xy;
    float2 pixelCenter = float2(pixelId)+0.5;

    if (any(pixelId >= (uint2)textureSize.xy))
    {
        return;
    }

    float2 velocity = texMVec[pixelId].xy;

    [branch]
    if (any(velocity > 1.0) || any(velocity < -1.0) || all(velocity == 0.0))
    {
        float2 uvCurrent = pixelCenter * textureSize.zw;
        float nonLinearDepth = texDepth[pixelId].x;
        float4 screenSpacePosCurrent = float4(float2(uvCurrent.x * 2.0 - 1.0, 1.0 - uvCurrent.y * 2.0), nonLinearDepth, 1.0);
        float4 screenSpacePosPrevious = mul(currentToPreviousClipMatrixNoOffset, screenSpacePosCurrent);
        float2 uvPrevious = float2(0.5, -0.5) * screenSpacePosPrevious.xy / screenSpacePosPrevious.w + 0.5;
        velocity = uvCurrent - uvPrevious;
    }
    else
    {
        velocity *= mvecScale; // to -1,1 range
    }
    if (showMvec != 0) velocity *= textureSize.xy * 10.0f;
    rwtexMVec[pixelId] = float2(-velocity) * textureSize.xy; // to pixel space
}


```

## Source file: include/sl_dlss_d.h

{
  "path": "include/sl_dlss_d.h",
  "blob_sha": "f17cab37510b835e4bb231a9a5e93843290f4ce5",
  "sha256": "6e51cf9815301c597d0657970a668147e34a20db5a76d0af254ec11248f5c4fe",
  "lines": 188,
  "bytes": 8222,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 37,
    "rig": 4
  },
  "url": "https://github.com/NVIDIA-RTX/Streamline/blob/2122257e0fce486f91b385aa63b9a09b0a34b363/include/sl_dlss_d.h",
  "function_signature_candidates": []
}

```text

/*
* Copyright (c) 2023 NVIDIA CORPORATION. All rights reserved
*
* Permission is hereby granted, free of charge, to any person obtaining a copy
* of this software and associated documentation files (the "Software"), to deal
* in the Software without restriction, including without limitation the rights
* to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
* copies of the Software, and to permit persons to whom the Software is
* furnished to do so, subject to the following conditions:
*
* The above copyright notice and this permission notice shall be included in all
* copies or substantial portions of the Software.
*
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
* IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
* FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
* AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
* LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
* OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
* SOFTWARE.
*/ 

#pragma once

#include "sl_dlss.h"

namespace sl
{

enum class DLSSDPreset : uint32_t
{
    //! Default behavior, may or may not change after an OTA
    eDefault,
                    // ePresetA removed, use preset D, E or F
                    // ePresetB removed, use preset D, E or F
                    // ePresetC removed, use preset D, E or F
    ePresetD = 4,   // Transformer model
    ePresetE = 5,   // Transformer model
    ePresetF = 6,   // Latest and default transformer model
    ePresetG = 7,   // Reverts to default
    ePresetH = 8,   // Reverts to default
    ePresetI = 9,   // Reverts to default
    ePresetJ = 10,  // Reverts to default
    ePresetK = 11,  // Reverts to default
    ePresetL = 12,  // Reverts to default
    ePresetM = 13,  // Reverts to default. Not recommended to use
    ePresetN = 14,  // Reverts to default. Not recommended to use
    ePresetO = 15,  // Reverts to default. Not recommended to use

    eCount
};

enum class DLSSDNormalRoughnessMode : uint32_t
{
    eUnpacked,  // App needs to provide Normal resource and Roughness resource separately.
    ePacked,    // App needs to write Roughness to w channel of Normal resource.

    eCount
};

// {0AD87504-774E-4BF3-9633-A44D1F7F9CB8}
SL_STRUCT_BEGIN(DLSSDOptions, StructType({ 0x0ad87504, 0x774e, 0x4bf3, { 0x96, 0x33, 0xa4, 0x4d, 0x1f, 0x7f, 0x9c, 0xb8 } }), kStructVersion3)
    //! Specifies which mode should be used
    DLSSMode mode = DLSSMode::eOff;
    //! Specifies output (final) target width
    uint32_t outputWidth = INVALID_UINT;
    //! Specifies output (final) target height
    uint32_t outputHeight = INVALID_UINT;
    //! Specifies sharpening level in range [0,1]
    float sharpness = 0.0f;
    //! Specifies pre-exposure value
    float preExposure = 1.0f;
    //! Specifies exposure scale value
    float exposureScale = 1.0f;
    //! Specifies if tagged color buffers are full HDR or not (DLSS in HDR pipeline or not)
    Boolean colorBuffersHDR = Boolean::eTrue;
    //! Specifies if indicator on screen should invert axis
    Boolean indicatorInvertAxisX = Boolean::eFalse;
    //! Specifies if indicator on screen should invert axis
    Boolean indicatorInvertAxisY = Boolean::eFalse;
    //! Specifies which mode should be used for roughness resource
    DLSSDNormalRoughnessMode normalRoughnessMode = DLSSDNormalRoughnessMode::eUnpacked;
    //! Specifies matrix transformation from the world space to the camera view space.
    float4x4 worldToCameraView;
    //! Specifies matrix transformation from the camera view space to the world space.
    //! cameraViewToWorld = worldToCameraView.inverse()
    float4x4 cameraViewToWorld;
    //! Whether or not the alpha channel should be upscaled (if false, only RGB is upscaled)
    //! Enabling alpha upscaling may impact performance
    Boolean alphaUpscalingEnabled = Boolean::eFalse;

    //! Presets
    DLSSDPreset dlaaPreset = DLSSDPreset::eDefault;
    DLSSDPreset qualityPreset = DLSSDPreset::eDefault;
    DLSSDPreset balancedPreset = DLSSDPreset::eDefault;
    DLSSDPreset performancePreset = DLSSDPreset::eDefault;
    DLSSDPreset ultraPerformancePreset = DLSSDPreset::eDefault;
    DLSSDPreset ultraQualityPreset = DLSSDPreset::eDefault;

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

//! Returned by DLSSD plugin
//! 
//! {FBD0C637-A28F-41F2-BC91-B421FAEE8E1E}
SL_STRUCT_BEGIN(DLSSDOptimalSettings, StructType({ 0xfbd0c637, 0xa28f, 0x41f2, { 0xbc, 0x91, 0xb4, 0x21, 0xfa, 0xee, 0x8e, 0x1e } }), kStructVersion1)
//! Specifies render area width
    uint32_t optimalRenderWidth{};
    //! Specifies render area height
    uint32_t optimalRenderHeight{};
    //! Specifies the optimal sharpness value
    float optimalSharpness{};
    //! Specifies minimal render area width
    uint32_t renderWidthMin{};
    //! Specifies minimal render area height
    uint32_t renderHeightMin{};
    //! Specifies maximal render area width
    uint32_t renderWidthMax{};
    //! Specifies maximal render area height
    uint32_t renderHeightMax{};

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

//! Returned by DLSSD plugin
//! 
//! {71873C14-F8CA-4767-9EAF-3B4393EA98FA}
SL_STRUCT_BEGIN(DLSSDState, StructType({ 0x71873c14, 0xf8ca, 0x4767, { 0x9e, 0xaf, 0x3b, 0x43, 0x93, 0xea, 0x98, 0xfa } }), kStructVersion1)
//! Specified the amount of memory expected to be used
    uint64_t estimatedVRAMUsageInBytes {};

    //! IMPORTANT: New members go here or if optional can be chained in a new struct, see sl_struct.h for details
SL_STRUCT_END()

}

//! Provides optimal DLSSD settings
//!
//! Call this method to obtain optimal render target size and other DLSSD related settings.
//!
//! @param options Specifies DLSSD options to use
//! @param settings Reference to a structure where settings are returned
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSDGetOptimalSettings = sl::Result(const sl::DLSSDOptions & options, sl::DLSSDOptimalSettings & settings);

//! Provides DLSSD state for the given viewport
//!
//! Call this method to obtain optimal render target size and other DLSSD related settings.
//!
//! @param viewport Specified viewport we are working with
//! @param state Reference to a structure where state is to be returned
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSDGetState = sl::Result(const sl::ViewportHandle & viewport, sl::DLSSDState & state);

//! Sets DLSSD options
//!
//! Call this method to turn DLSSD on/off, change mode etc.
//!
//! @param viewport Specified viewport we are working with
//! @param options Specifies DLSSD options to use
//! @return sl::ResultCode::eOk if successful, error code otherwise (see sl_result.h for details)
//!
//! This method is NOT thread safe.
using PFun_slDLSSDSetOptions = sl::Result(const sl::ViewportHandle& viewport, const sl::DLSSDOptions& options);

//! HELPERS
//! 
inline sl::Result slDLSSDGetOptimalSettings(const sl::DLSSDOptions& options, sl::DLSSDOptimalSettings& settings)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS_RR, slDLSSDGetOptimalSettings);
    return s_slDLSSDGetOptimalSettings(options, settings);
}

inline sl::Result slDLSSDGetState(const sl::ViewportHandle& viewport, sl::DLSSDState& state)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS_RR, slDLSSDGetState);
    return s_slDLSSDGetState(viewport, state);
}

inline sl::Result slDLSSDSetOptions(const sl::ViewportHandle& viewport, const sl::DLSSDOptions& options)
{
    SL_FEATURE_FUN_IMPORT_STATIC(sl::kFeatureDLSS_RR, slDLSSDSetOptions);
    return s_slDLSSDSetOptions(viewport, options);
}


```

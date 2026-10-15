# Facial rendering source scan: NVIDIA/DLSS

Repository: https://github.com/NVIDIA/DLSS

Pinned source revision: 374959484e79a640feaba44c93ac8cfb0a03f5b5

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"dlss": ["DLSS Donut and Sample Source_Pre-Release License (2July2019).pdf", "doc/DLSS-FG Programming Guide.pdf", "doc/DLSS-RR Integration Guide.pdf", "doc/DLSS_Programming_Guide_Release.pdf", "include/nvsdk_ngx_defs_dlssd.h", "include/nvsdk_ngx_defs_dlssg.h", "include/nvsdk_ngx_helpers_dlssd.h", "include/nvsdk_ngx_helpers_dlssd_cuda.h", "include/nvsdk_ngx_helpers_dlssd_d3d.h", "include/nvsdk_ngx_helpers_dlssd_vk.h", "include/nvsdk_ngx_helpers_dlssg.h", "include/nvsdk_ngx_helpers_dlssg_d3d.h", "include/nvsdk_ngx_helpers_dlssg_vk.h", "include/nvsdk_ngx_params_dlssd.h", "include/nvsdk_ngx_params_dlssg.h", "lib/Linux_aarch64/dev/libnvidia-ngx-dlss.so.310.9.1", "lib/Linux_aarch64/dev/libnvidia-ngx-dlssd.so.310.9.1", "lib/Linux_aarch64/dev/libnvidia-ngx-dlssg.so.310.9.1", "lib/Linux_aarch64/rel/libnvidia-ngx-dlss.so.310.9.1", "lib/Linux_aarch64/rel/libnvidia-ngx-dlssd.so.310.9.1", "lib/Linux_aarch64/rel/libnvidia-ngx-dlssg.so.310.9.1", "lib/Linux_x86_64/dev/libnvidia-ngx-dlss.so.310.9.1", "lib/Linux_x86_64/dev/libnvidia-ngx-dlssd.so.310.9.1", "lib/Linux_x86_64/dev/libnvidia-ngx-dlssg.so.310.9.1", "lib/Linux_x86_64/rel/libnvidia-ngx-dlss.so.310.9.1", "lib/Linux_x86_64/rel/libnvidia-ngx-dlssd.so.310.9.1", "lib/Linux_x86_64/rel/libnvidia-ngx-dlssg.so.310.9.1", "lib/Windows_aarch64/dev/nvngx_dlss.dll", "lib/Windows_aarch64/dev/nvngx_dlssd.dll", "lib/Windows_aarch64/dev/nvngx_dlssg.dll"]}

## Source file: include/nvsdk_ngx_defs_dlssd.h

{
  "path": "include/nvsdk_ngx_defs_dlssd.h",
  "blob_sha": "8b73e3f347fab4f315443869ad57eeeb61168d7f",
  "sha256": "2e97fe1595b1334f71b51724f2cb67496e8c296a7a2eec5606da53055882e716",
  "lines": 166,
  "bytes": 14693,
  "selection_score": 15,
  "keyword_line_counts": {
    "subsurface": 9,
    "dlss": 108,
    "rig": 2,
    "face": 9,
    "lip": 2
  },
  "url": "https://github.com/NVIDIA/DLSS/blob/374959484e79a640feaba44c93ac8cfb0a03f5b5/include/nvsdk_ngx_defs_dlssd.h",
  "function_signature_candidates": []
}

```text

/*
 * SPDX-FileCopyrightText: Copyright (c) 2023 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: LicenseRef-NvidiaProprietary
 *
 * NVIDIA CORPORATION, its affiliates and licensors retain all intellectual
 * property and proprietary rights in and to this material, related
 * documentation and any modifications thereto. Any use, reproduction,
 * disclosure or distribution of this material and related documentation
 * without an express license agreement from NVIDIA CORPORATION or
 * its affiliates is strictly prohibited.
 */

#ifndef NVSDK_NGX_DEFS_DLSSD_H
#define NVSDK_NGX_DEFS_DLSSD_H
#pragma once

typedef enum NVSDK_NGX_DLSS_Denoise_Mode
{
    NVSDK_NGX_DLSS_Denoise_Mode_Off = 0,
    NVSDK_NGX_DLSS_Denoise_Mode_DLUnified = 1, // DL based unified upscaler
} NVSDK_NGX_DLSS_Denoise_Mode;

typedef enum NVSDK_NGX_DLSS_Roughness_Mode
{
    NVSDK_NGX_DLSS_Roughness_Mode_Unpacked = 0, // Read roughness separately
    NVSDK_NGX_DLSS_Roughness_Mode_Packed = 1, // Read roughness from normals.w
} NVSDK_NGX_DLSS_Roughness_Mode;

typedef enum NVSDK_NGX_DLSS_Depth_Type
{
    NVSDK_NGX_DLSS_Depth_Type_Linear = 0, // Linear Depth
    NVSDK_NGX_DLSS_Depth_Type_HW = 1,     // HW Depth
} NVSDK_NGX_DLSS_Depth_Type;


typedef enum NVSDK_NGX_RayReconstruction_Hint_Render_Preset
{
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_Default = 0,             // default behavior, may or may not change after OTA
                                                                            // NVSDK_NGX_RayReconstruction_Hint_Render_Preset_A removed, use preset D or E
                                                                            // NVSDK_NGX_RayReconstruction_Hint_Render_Preset_B removed, use preset D or E
                                                                            // NVSDK_NGX_RayReconstruction_Hint_Render_Preset_C removed, use preset D or E
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_D = 4,                   // Transformer model
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_E = 5,                   // Latest transformer model (must use if DoF guide is needed)
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_F = 6,                   // Default model RR2
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_G = 7,                   // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_H = 8,                   // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_I = 9,                   // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_J = 10,                  // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_K = 11,                  // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_L = 12,                  // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_M = 13,                  // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_N = 14,                  // Do not use, reverts to default behavior
    NVSDK_NGX_RayReconstruction_Hint_Render_Preset_O = 15,                  // Do not use, reverts to default behavior
} NVSDK_NGX_RayReconstruction_Hint_Render_Preset;

#define NVSDK_NGX_Parameter_DLSS_Denoise_Mode "DLSS.Denoise.Mode"
#define NVSDK_NGX_Parameter_DLSS_Roughness_Mode "DLSS.Roughness.Mode"
#define NVSDK_NGX_Parameter_DiffuseAlbedo  "DLSS.Input.DiffuseAlbedo"
#define NVSDK_NGX_Parameter_SpecularAlbedo "DLSS.Input.SpecularAlbedo"
#define NVSDK_NGX_Parameter_DLSS_Input_DiffuseAlbedo_Subrect_Base_X "DLSS.Input.DiffuseAlbedo.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSS_Input_DiffuseAlbedo_Subrect_Base_Y "DLSS.Input.DiffuseAlbedo.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSS_Input_SpecularAlbedo_Subrect_Base_X "DLSS.Input.SpecularAlbedo.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSS_Input_SpecularAlbedo_Subrect_Base_Y "DLSS.Input.SpecularAlbedo.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSS_Input_Normals_Subrect_Base_X "DLSS.Input.Normals.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSS_Input_Normals_Subrect_Base_Y "DLSS.Input.Normals.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSS_Input_Roughness_Subrect_Base_X "DLSS.Input.Roughness.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSS_Input_Roughness_Subrect_Base_Y "DLSS.Input.Roughness.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_ViewToClipMatrix "ViewToClipMatrix"
#define NVSDK_NGX_Parameter_GBuffer_Emissive "GBuffer.Emissive"
#define NVSDK_NGX_Parameter_Use_Folded_Network "DLSS.Use.Folded.Network"
#define NVSDK_NGX_Parameter_Diffuse_Ray_Direction "Diffuse.Ray.Direction"
#define NVSDK_NGX_Parameter_DLSS_WORLD_TO_VIEW_MATRIX "WorldToViewMatrix"
#define NVSDK_NGX_Parameter_DLSS_VIEW_TO_CLIP_MATRIX "ViewToClipMatrix"
#define NVSDK_NGX_Parameter_Use_HW_Depth "DLSS.Use.HW.Depth"
#define NVSDK_NGX_Parameter_DLSSD_ReflectedAlbedo "DLSSD.ReflectedAlbedo"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeParticles "DLSSD.ColorBeforeParticles"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterParticles "DLSSD.ColorAfterParticles"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeTransparency "DLSSD.ColorBeforeTransparency"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterTransparency "DLSSD.ColorAfterTransparency"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeFog "DLSSD.ColorBeforeFog"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterFog "DLSSD.ColorAfterFog"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceSubsurfaceScatteringGuide "DLSSD.ScreenSpaceSubsurfaceScatteringGuide"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceSubsurfaceScattering "DLSSD.ColorBeforeScreenSpaceSubsurfaceScattering"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceSubsurfaceScattering "DLSSD.ColorAfterScreenSpaceSubsurfaceScattering"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceRefractionGuide "DLSSD.ScreenSpaceRefractionGuide"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceRefraction "DLSSD.ColorBeforeScreenSpaceRefraction"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceRefraction "DLSSD.ColorAfterScreenSpaceRefraction"
#define NVSDK_NGX_Parameter_DLSSD_DepthOfFieldGuide "DLSSD.DepthOfFieldGuide"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeDepthOfField "DLSSD.ColorBeforeDepthOfField"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterDepthOfField "DLSSD.ColorAfterDepthOfField"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseHitDistance "DLSSD.DiffuseHitDistance"
#define NVSDK_NGX_Parameter_DLSSD_SpecularHitDistance "DLSSD.SpecularHitDistance"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirection "DLSSD.DiffuseRayDirection"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirection "DLSSD.SpecularRayDirection"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirectionHitDistance "DLSSD.DiffuseRayDirectionHitDistance"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirectionHitDistance "DLSSD.SpecularRayDirectionHitDistance"
#define NVSDK_NGX_Parameter_DLSSD_Alpha "DLSSD.Alpha"
#define NVSDK_NGX_Parameter_DLSSD_OutputAlpha "DLSSD.OutputAlpha"
#define NVSDK_NGX_Parameter_DLSSD_ResponsivityMask "DLSSD.ResponsivityMask"
#define NVSDK_NGX_Parameter_DLSSD_Alpha_Subrect_Base_X "DLSSD.Alpha.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_Alpha_Subrect_Base_Y "DLSSD.Alpha.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_OutputAlpha_Subrect_Base_X "DLSSD.OutputAlpha.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_OutputAlpha_Subrect_Base_Y "DLSSD.OutputAlpha.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ReflectedAlbedo_Subrect_Base_X "DLSSD.ReflectedAlbedo.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ReflectedAlbedo_Subrect_Base_Y "DLSSD.ReflectedAlbedo.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeParticles_Subrect_Base_X "DLSSD.ColorBeforeParticles.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeParticles_Subrect_Base_Y "DLSSD.ColorBeforeParticles.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterParticles_Subrect_Base_X "DLSSD.ColorAfterParticles.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterParticles_Subrect_Base_Y "DLSSD.ColorAfterParticles.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeTransparency_Subrect_Base_X "DLSSD.ColorBeforeTransparency.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeTransparency_Subrect_Base_Y "DLSSD.ColorBeforeTransparency.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterTransparency_Subrect_Base_X "DLSSD.ColorAfterTransparency.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterTransparency_Subrect_Base_Y "DLSSD.ColorAfterTransparency.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeFog_Subrect_Base_X "DLSSD.ColorBeforeFog.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeFog_Subrect_Base_Y "DLSSD.ColorBeforeFog.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterFog_Subrect_Base_X "DLSSD.ColorAfterFog.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterFog_Subrect_Base_Y "DLSSD.ColorAfterFog.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceSubsurfaceScatteringGuide_Subrect_Base_X "DLSSD.ScreenSpaceSubsurfaceScatteringGuide.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceSubsurfaceScatteringGuide_Subrect_Base_Y "DLSSD.ScreenSpaceSubsurfaceScatteringGuide.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceSubsurfaceScattering_Subrect_Base_X "DLSSD.ColorBeforeScreenSpaceSubsurfaceScattering.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceSubsurfaceScattering_Subrect_Base_Y "DLSSD.ColorBeforeScreenSpaceSubsurfaceScattering.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceSubsurfaceScattering_Subrect_Base_X "DLSSD.ColorAfterScreenSpaceSubsurfaceScattering.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceSubsurfaceScattering_Subrect_Base_Y "DLSSD.ColorAfterScreenSpaceSubsurfaceScattering.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceRefractionGuide_Subrect_Base_X "DLSSD.ScreenSpaceRefractionGuide.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ScreenSpaceRefractionGuide_Subrect_Base_Y "DLSSD.ScreenSpaceRefractionGuide.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceRefraction_Subrect_Base_X "DLSSD.ColorBeforeScreenSpaceRefraction.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeScreenSpaceRefraction_Subrect_Base_Y "DLSSD.ColorBeforeScreenSpaceRefraction.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceRefraction_Subrect_Base_X "DLSSD.ColorAfterScreenSpaceRefraction.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterScreenSpaceRefraction_Subrect_Base_Y "DLSSD.ColorAfterScreenSpaceRefraction.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_DepthOfFieldGuide_Subrect_Base_X "DLSSD.DepthOfFieldGuide.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_DepthOfFieldGuide_Subrect_Base_Y "DLSSD.DepthOfFieldGuide.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeDepthOfField_Subrect_Base_X "DLSSD.ColorBeforeDepthOfField.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorBeforeDepthOfField_Subrect_Base_Y "DLSSD.ColorBeforeDepthOfField.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterDepthOfField_Subrect_Base_X "DLSSD.ColorAfterDepthOfField.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ColorAfterDepthOfField_Subrect_Base_Y "DLSSD.ColorAfterDepthOfField.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseHitDistance_Subrect_Base_X "DLSSD.DiffuseHitDistance.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseHitDistance_Subrect_Base_Y "DLSSD.DiffuseHitDistance.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_SpecularHitDistance_Subrect_Base_X "DLSSD.SpecularHitDistance.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_SpecularHitDistance_Subrect_Base_Y "DLSSD.SpecularHitDistance.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirection_Subrect_Base_X "DLSSD.DiffuseRayDirection.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirection_Subrect_Base_Y "DLSSD.DiffuseRayDirection.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirection_Subrect_Base_X "DLSSD.SpecularRayDirection.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirection_Subrect_Base_Y "DLSSD.SpecularRayDirection.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirectionHitDistance_Subrect_Base_X "DLSSD.DiffuseRayDirectionHitDistance.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_DiffuseRayDirectionHitDistance_Subrect_Base_Y "DLSSD.DiffuseRayDirectionHitDistance.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirectionHitDistance_Subrect_Base_X "DLSSD.SpecularRayDirectionHitDistance.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_SpecularRayDirectionHitDistance_Subrect_Base_Y "DLSSD.SpecularRayDirectionHitDistance.Subrect.Base.Y"
#define NVSDK_NGX_Parameter_DLSSD_ResponsivityMask_Subrect_Base_X "DLSSD.ResponsivityMask.Subrect.Base.X"
#define NVSDK_NGX_Parameter_DLSSD_ResponsivityMask_Subrect_Base_Y "DLSSD.ResponsivityMask.Subrect.Base.Y"

#define NVSDK_NGX_Parameter_SuperSamplingDenoising_Available             "SuperSamplingDenoising.Available"
#define NVSDK_NGX_Parameter_SuperSamplingDenoising_NeedsUpdatedDriver    "SuperSamplingDenoising.NeedsUpdatedDriver"
#define NVSDK_NGX_Parameter_SuperSamplingDenoising_MinDriverVersionMajor "SuperSamplingDenoising.MinDriverVersionMajor"
#define NVSDK_NGX_Parameter_SuperSamplingDenoising_MinDriverVersionMinor "SuperSamplingDenoising.MinDriverVersionMinor"
#define NVSDK_NGX_Parameter_SuperSamplingDenoising_FeatureInitResult     "SuperSamplingDenoising.FeatureInitResult"
#define NVSDK_NGX_Parameter_DLSSDOptimalSettingsCallback "DLSSDOptimalSettingsCallback"
#define NVSDK_NGX_Parameter_DLSSDGetStatsCallback        "DLSSDGetStatsCallback"

#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_DLAA "RayReconstruction.Hint.Render.Preset.DLAA"
#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_Quality "RayReconstruction.Hint.Render.Preset.Quality"
#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_Balanced "RayReconstruction.Hint.Render.Preset.Balanced"
#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_Performance "RayReconstruction.Hint.Render.Preset.Performance"
#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_UltraPerformance "RayReconstruction.Hint.Render.Preset.UltraPerformance"
#define NVSDK_NGX_Parameter_RayReconstruction_Hint_Render_Preset_UltraQuality "RayReconstruction.Hint.Render.Preset.UltraQuality"

#endif // NVSDK_NGX_DEFS_DLSSD_H


```

## Source file: include/nvsdk_ngx_defs_dlssg.h

{
  "path": "include/nvsdk_ngx_defs_dlssg.h",
  "blob_sha": "3337ad5e8b2221bc914069b4171c2bc6461e2eed",
  "sha256": "98ec3a6faa5aa250828c174bf8f3491aafac693dfe57cc798049c8918597fc60",
  "lines": 372,
  "bytes": 22885,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 4,
    "dlss": 160,
    "rig": 7,
    "face": 5,
    "lip": 8
  },
  "url": "https://github.com/NVIDIA/DLSS/blob/374959484e79a640feaba44c93ac8cfb0a03f5b5/include/nvsdk_ngx_defs_dlssg.h",
  "function_signature_candidates": []
}

```text

#pragma once
/*
* Copyright (c) 2021-2024 NVIDIA CORPORATION.  All rights reserved.
*
* NVIDIA Corporation and its licensors retain all intellectual property and proprietary
* rights in and to this software, related documentation and any modifications thereto.
* Any use, reproduction, disclosure or distribution of this software and related
* documentation without an express license agreement from NVIDIA Corporation is strictly
* prohibited.
*
* TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, THIS SOFTWARE IS PROVIDED *AS IS*
* AND NVIDIA AND ITS SUPPLIERS DISCLAIM ALL WARRANTIES, EITHER EXPRESS OR IMPLIED,
* INCLUDING, BUT NOT LIMITED TO, IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
* PARTICULAR PURPOSE.  IN NO EVENT SHALL NVIDIA OR ITS SUPPLIERS BE LIABLE FOR ANY
* SPECIAL, INCIDENTAL, INDIRECT, OR CONSEQUENTIAL DAMAGES WHATSOEVER (INCLUDING, WITHOUT
* LIMITATION, DAMAGES FOR LOSS OF BUSINESS PROFITS, BUSINESS INTERRUPTION, LOSS OF
* BUSINESS INFORMATION, OR ANY OTHER PECUNIARY LOSS) ARISING OUT OF THE USE OF OR
* INABILITY TO USE THIS SOFTWARE, EVEN IF NVIDIA HAS BEEN ADVISED OF THE POSSIBILITY OF
* SUCH DAMAGES.
*/


#ifndef NVSDK_NGX_DEFS_DLSSG_H
#define NVSDK_NGX_DEFS_DLSSG_H

#include <stddef.h> // for size_t
#include <stdint.h> // for uint64_t

#include "nvsdk_ngx_defs.h"
#include "nvsdk_ngx_params.h"

typedef enum NVSDK_NGX_DLSSG_ResourceFlags
{
    NVSDK_NGX_DLSSG_ResourceFlags_Backbuffer                   = 1 << 0,
    NVSDK_NGX_DLSSG_ResourceFlags_MVecs                        = 1 << 1,
    NVSDK_NGX_DLSSG_ResourceFlags_Depth                        = 1 << 2,
    NVSDK_NGX_DLSSG_ResourceFlags_HUDLess                      = 1 << 3,
    NVSDK_NGX_DLSSG_ResourceFlags_UI                           = 1 << 4,
    NVSDK_NGX_DLSSG_ResourceFlags_BidirectionalDistortionField = 1 << 5,
    NVSDK_NGX_DLSSG_ResourceFlags_Reserved6                    = 1 << 6,
    NVSDK_NGX_DLSSG_ResourceFlags_UIAlpha                      = 1 << 7,

    NVSDK_NGX_DLSSG_ResourceFlags_OutputInterpolated           = 1 << 24,
    NVSDK_NGX_DLSSG_ResourceFlags_OutputReal                   = 1 << 25,
    NVSDK_NGX_DLSSG_ResourceFlags_OutputDisableInterpolation   = 1 << 26,
} NVSDK_NGX_DLSSG_ResourceFlags;

#define NVSDK_NGX_Parameter_FrameGeneration_Available                "FrameGeneration.Available"
#define NVSDK_NGX_Parameter_FrameGeneration_FeatureInitResult        "FrameGeneration.FeatureInitResult"
#define NVSDK_NGX_Parameter_FrameGeneration_MinDriverVersionMajor    "FrameGeneration.MinDriverVersionMajor"
#define NVSDK_NGX_Parameter_FrameGeneration_MinDriverVersionMinor    "FrameGeneration.MinDriverVersionMinor"
#define NVSDK_NGX_Parameter_FrameGeneration_NeedsUpdatedDriver       "FrameGeneration.NeedsUpdatedDriver"

#define NVSDK_NGX_Parameter_FrameInterpolation_Available                "FrameInterpolation.Available"
#define NVSDK_NGX_Parameter_FrameInterpolation_FeatureInitResult        "FrameInterpolation.FeatureInitResult"
#define NVSDK_NGX_Parameter_FrameInterpolation_MinDriverVersionMajor    "FrameInterpolation.MinDriverVersionMajor"
#define NVSDK_NGX_Parameter_FrameInterpolation_MinDriverVersionMinor    "FrameInterpolation.MinDriverVersionMinor"
#define NVSDK_NGX_Parameter_FrameInterpolation_NeedsUpdatedDriver       "FrameInterpolation.NeedsUpdatedDriver"

#define NVSDK_NGX_DLSSG_Parameter_BackbufferFormat           "DLSSG.BackbufferFormat"

// Required Input Textures

#define NVSDK_NGX_DLSSG_Parameter_Backbuffer                            "DLSSG.Backbuffer"
#define NVSDK_NGX_DLSSG_Parameter_MVecs                                 "DLSSG.MVecs"
#define NVSDK_NGX_DLSSG_Parameter_Depth                                 "DLSSG.Depth"

// Optional Input Textures to improve Image Quality (strongly suggested)

// NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionField: Resource describing post-processing distortion effects,
// such as lens distortion, that have been applied to the final color. If you use this type of effect, the final
// color is no longer aligned with the input depth, motion vectors, and camera information.
// The bidirectional distortion field serves as a mapping between the pixels in the distorted color texture
// and the undistorted guide buffers.
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionField          "DLSSG.BidirectionalDistortionField"
// NVSDK_NGX_DLSSG_Parameter_HUDLess: Contains the scene color before any UI or HUD elements
// are drawn. Its content for non-UI pixels should be identical to the backbuffer.
#define NVSDK_NGX_DLSSG_Parameter_HUDLess                               "DLSSG.HUDLess"
// NVSDK_NGX_DLSSG_Parameter_UI: A four-channel resource containing the color and opacity/alpha of the UI.
// The color should be pre-multiplied by the alpha value, such that for each pixel:
// backbuffer_color = ui_color + (1 - ui_alpha) * hudless_scene_color
#define NVSDK_NGX_DLSSG_Parameter_UI                                    "DLSSG.UI"
// NVSDK_NGX_DLSSG_Parameter_UIAlpha: A single channel resource of only the alpha UI values, between 0.0f and 1.0f.
#define NVSDK_NGX_DLSSG_Parameter_UIAlpha                               "DLSSG.UIAlpha"
// Note: Only one of NVSDK_NGX_DLSSG_Parameter_UIAlpha or NVSDK_NGX_DLSSG_Parameter_UI need to be provided.
//       For User Interface Recomposition (UIR) frame generation, both HUDless and one of UI or UIAlpha must be provided.
//       Additionally set the NVSDK_NGX_DLSSG_Parameter_UserInterfaceRecompositionEnabled parameter to trigger
//       the create time decisions required for this UIR mode. See that parameter description for more information.

// Required Output Texture (same texture format as Backbuffer)
#define NVSDK_NGX_DLSSG_Parameter_OutputInterpolated                    "DLSSG.OutputInterpolated"
// Optional Output Texture (gives FG the opportunity to render text or debug info onto the real frame)
#define NVSDK_NGX_DLSSG_Parameter_OutputReal                            "DLSSG.OutputReal"

// Legacy names for the output interpolated and real frames. Use the equivalent parameters
// NVSDK_NGX_DLSSG_Parameter_OutputInterpolated and NVSDK_NGX_DLSSG_Parameter_OutputReal
// instead.
#define NVSDK_NGX_Parameter_OutputInterpolated                          "DLSSG.OutputInterpolated"
#define NVSDK_NGX_Parameter_OutputReal                                  "DLSSG.OutputReal"

// Create-time resource flag bitmasks (see NVSDK_NGX_DLSSG_ResourceFlags)
// denoting resources that the application will always/never provide for the
// lifetime of the feature instance. Setting these values may enable memory
// optimizations to reduce VRAM. Resources not specified in either mask remain
// optional, and may be provided on some frames, but not others. Resources
// required by the algorithm (backbuffer, mvecs, depth, and output) are assumed
// to be set in AlwaysProvidedFlags and unset in NeverProvidedFlags, regardless
// of the actual values
#define NVSDK_NGX_DLSSG_Parameter_ResourceAlwaysProvided_Flags "DLSSG.ResourceAlwaysProvidedFlags"
#define NVSDK_NGX_DLSSG_Parameter_ResourceNeverProvided_Flags  "DLSSG.ResourceNeverProvidedFlags"

// DLFG-specific width/height parameters, to avoid name conflicts with other
// features. If set in CreateFeature, these parameters will be used instead of
// the generic NVSDK_NGX_Parameter{Width,Height}
#define NVSDK_NGX_DLSSG_Parameter_Width     "DLSSG.Width"
#define NVSDK_NGX_DLSSG_Parameter_Height    "DLSSG.Height"

// Output resource which allows the snippet to hint to the host application that
// the interpolated frame(s) should not be shown this frame. This resource must
// be created by the application with a minimum size of 4 bytes, and will be filled
// by the snippet: if interpolation should be disabled for this frame, boolean true
// will be written to the first byte of the buffer.
#define NVSDK_NGX_DLSSG_Parameter_OutputDisableInterpolation "DLSSG.OutputDisableInterpolation"

// IMPORTANT: All matrices should NOT contain temporal AA jitter offset
// Specifies matrix transformation from the camera view to the clip space.
// Float4x4 as void*
#define NVSDK_NGX_DLSSG_Parameter_CameraViewToClip              "DLSSG.CameraViewToClip"
// Float4x4 as void*
#define NVSDK_NGX_DLSSG_Parameter_ClipToCameraView              "DLSSG.ClipToCameraView"
// Float4x4 as void*
#define NVSDK_NGX_DLSSG_Parameter_ClipToLensClip                "DLSSG.ClipToLensClip"
// Float4x4 as void*
#define NVSDK_NGX_DLSSG_Parameter_ClipToPrevClip                "DLSSG.ClipToPrevClip"
// Float4x4 as void*
#define NVSDK_NGX_DLSSG_Parameter_PrevClipToClip                "DLSSG.PrevClipToClip"

// Specifies clip space jitter offset
#define NVSDK_NGX_DLSSG_Parameter_JitterOffsetX              "DLSSG.JitterOffsetX"
#define NVSDK_NGX_DLSSG_Parameter_JitterOffsetY              "DLSSG.JitterOffsetY"

// Specifies scale factors used to normalize motion vectors (so the values are in [-1,1] range)
#define NVSDK_NGX_DLSSG_Parameter_MvecScaleX                 "DLSSG.MvecScaleX"
#define NVSDK_NGX_DLSSG_Parameter_MvecScaleY                 "DLSSG.MvecScaleY"

// Specifies camera pinhole offset.
#define NVSDK_NGX_DLSSG_Parameter_CameraPinholeOffsetX       "DLSSG.CameraPinholeOffsetX"
#define NVSDK_NGX_DLSSG_Parameter_CameraPinholeOffsetY       "DLSSG.CameraPinholeOffsetY"

// Specifies camera position in world space.
#define NVSDK_NGX_DLSSG_Parameter_CameraPosX                 "DLSSG.CameraPosX"
#define NVSDK_NGX_DLSSG_Parameter_CameraPosY                 "DLSSG.CameraPosY"
#define NVSDK_NGX_DLSSG_Parameter_CameraPosZ                 "DLSSG.CameraPosZ"

// Specifies camera up vector in world space.
#define NVSDK_NGX_DLSSG_Parameter_CameraUpX                  "DLSSG.CameraUpX"
#define NVSDK_NGX_DLSSG_Parameter_CameraUpY                  "DLSSG.CameraUpY"
#define NVSDK_NGX_DLSSG_Parameter_CameraUpZ                  "DLSSG.CameraUpZ"

// Specifies camera right vector in world space.
#define NVSDK_NGX_DLSSG_Parameter_CameraRightX               "DLSSG.CameraRightX"
#define NVSDK_NGX_DLSSG_Parameter_CameraRightY               "DLSSG.CameraRightY"
#define NVSDK_NGX_DLSSG_Parameter_CameraRightZ               "DLSSG.CameraRightZ"

// Specifies camera forward vector in world space.
#define NVSDK_NGX_DLSSG_Parameter_CameraFwdX                 "DLSSG.CameraFwdX"
#define NVSDK_NGX_DLSSG_Parameter_CameraFwdY                 "DLSSG.CameraFwdY"
#define NVSDK_NGX_DLSSG_Parameter_CameraFwdZ                 "DLSSG.CameraFwdZ"

// Specifies camera near view plane distance.
#define NVSDK_NGX_DLSSG_Parameter_CameraNear                 "DLSSG.CameraNear"

// Specifies camera far view plane distance.
#define NVSDK_NGX_DLSSG_Parameter_CameraFar                  "DLSSG.CameraFar"

// Specifies camera field of view in radians.
#define NVSDK_NGX_DLSSG_Parameter_CameraFOV                  "DLSSG.CameraFOV"

// Specifies camera aspect ratio defined as view space width divided by height.
#define NVSDK_NGX_DLSSG_Parameter_CameraAspectRatio          "DLSSG.CameraAspectRatio"

// Specifies if tagged color buffers are full HDR (rendering to an HDR monitor) or not
#define NVSDK_NGX_DLSSG_Parameter_ColorBuffersHDR            "DLSSG.ColorBuffersHDR"

// Specifies if depth values are inverted (value closer to the camera is higher) or not.
#define NVSDK_NGX_DLSSG_Parameter_DepthInverted              "DLSSG.DepthInverted"

// Specifies if camera motion is included in the MVec buffer.
#define NVSDK_NGX_DLSSG_Parameter_CameraMotionIncluded       "DLSSG.CameraMotionIncluded"

// Specifies if motion vectors include jitter data.

// Specifies if previous frame has no connection to the current one (motion vectors are invalid)
#define NVSDK_NGX_DLSSG_Parameter_Reset                      "DLSSG.Reset"

// Specifies if Automode overrode the input Reset flag to true.
#define NVSDK_NGX_DLSSG_Parameter_AutomodeOverrideReset      "DLSSG.AutomodeOverrideReset"

// Specifies if application is not currently rendering game frames (paused in menu, playing video cut-scenes)
#define NVSDK_NGX_DLSSG_Parameter_NotRenderingGameFrames     "DLSSG.NotRenderingGameFrames"

// Specifies if orthographic projection is used or not.
#define NVSDK_NGX_DLSSG_Parameter_OrthoProjection            "DLSSG.OrthoProjection"

#define NVSDK_NGX_DLSSG_Parameter_EstimateVRAMCallback       "DLSSG.EstimateVRAMCallback"

#define NVSDK_NGX_DLSSG_Parameter_InvertXAxis        "DLSSG.InvertXAxis"
#define NVSDK_NGX_DLSSG_Parameter_InvertYAxis        "DLSSG.InvertYAxis"

#define NVSDK_NGX_DLSSG_Parameter_UserDebugText      "DLSSG.UserDebugText"

#define NVSDK_NGX_DLSSG_Parameter_MvecInvalidValue   "DLSSG.MvecInvalidValue"
#define NVSDK_NGX_DLSSG_Parameter_MvecDilated        "DLSSG.MvecDilated"
#define NVSDK_NGX_DLSSG_Parameter_MvecJittered       "DLSSG.MvecJittered"

#define NVSDK_NGX_DLSSG_Parameter_MenuDetectionEnabled "DLSSG.MenuDetectionEnabled"

// If true, allow the snippet to create internal resources on a separate thread, to avoid hitches during feature creation (default false)
#define NVSDK_NGX_DLSSG_Parameter_AsyncCreateEnabled "DLSSG.AsyncCreateEnabled"

// The DLSS Frame Generation algorithm processes all depth in a "linearized" depth space using the below formulation.
//
//     if NVSDK_NGX_DLSSG_Parameter_DepthInverted is false:    `lin_depth = 1 / (1 - depth)`
//     if NVSDK_NGX_DLSSG_Parameter_DepthInverted is true:     `lin_depth = 1 / depth`
//
// This enables the algorithm to reliably threshold and characterize depth interactions.
//
// The Frame Generation algorithm is tuned such that most game integrations do not need to modify these values.
// However, the below parameters provide mechanisms to control the shape of the internal depth processing space.
// Typically useful when input raw depth units are compressed into a small dynamic range. The scale modifier
// NVSDK_NGX_DLSSG_Parameter_LinearizedDepth_Scale is a good place to start that won't require more changes.
//
// Note: the input depth texture must be consistent with the input camera transformation matrices, e.g. clip2prevclip.

// Optional scalar on the linearized depth before algorithm processing.
//
// Defaults to 1.0f. A value of 0.1f can help with a compressed input depth range.
#define NVSDK_NGX_DLSSG_Parameter_LinearizedDepth_Scale "DLSSG.LinearizedDepth_Scale"

// Optional heuristic that defines the z-distance to the depth plane that artificially partitions near from far objects.
// DLSS Frame Generation uses this plane to restrict far plane object motion from scattering into near plane objects.
//
// Defaults to 600.0f. A value of 4000.0f can help with a compressed input depth range.
#define NVSDK_NGX_DLSSG_Parameter_LinearizedDepth_NearFarPartition "DLSSG.LinearizedDepth_NearFarPartition"

// Optional heuristic that specifies the minimum linearized depth difference between two objects in screen-space.
// DLSS Frame Generation uses this heuristic to understand contiguous depth surfaces.
//
// Defaults to 40.0f.
#define NVSDK_NGX_DLSSG_Parameter_MinRelativeLinearDepthObjectSeparation "DLSSG.MinRelativeLinearDepthObjectSeparation"

// Texture Subrect Definitions

// Subrect definitions for the input backbuffer (NVSDK_NGX_DLSSG_Parameter_Backbuffer)
#define NVSDK_NGX_DLSSG_Parameter_InputBackbufferSubrectBaseX  "DLSSG.InputBackbufferSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_InputBackbufferSubrectBaseY  "DLSSG.InputBackbufferSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_InputBackbufferSubrectWidth  "DLSSG.InputBackbufferSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_InputBackbufferSubrectHeight "DLSSG.InputBackbufferSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_MVecsSubrectBaseX "DLSSG.MVecsSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_MVecsSubrectBaseY "DLSSG.MVecsSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_MVecsSubrectWidth "DLSSG.MVecsSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_MVecsSubrectHeight "DLSSG.MVecsSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_DepthSubrectBaseX "DLSSG.DepthSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_DepthSubrectBaseY "DLSSG.DepthSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_DepthSubrectWidth "DLSSG.DepthSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_DepthSubrectHeight "DLSSG.DepthSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_HUDLessSubrectBaseX "DLSSG.HUDLessSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_HUDLessSubrectBaseY "DLSSG.HUDLessSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_HUDLessSubrectWidth "DLSSG.HUDLessSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_HUDLessSubrectHeight "DLSSG.HUDLessSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_UISubrectBaseX "DLSSG.UISubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_UISubrectBaseY "DLSSG.UISubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_UISubrectWidth "DLSSG.UISubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_UISubrectHeight "DLSSG.UISubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_UIAlphaSubrectBaseX "DLSSG.UIAlphaSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_UIAlphaSubrectBaseY "DLSSG.UIAlphaSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_UIAlphaSubrectWidth "DLSSG.UIAlphaSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_UIAlphaSubrectHeight "DLSSG.UIAlphaSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionFieldSubrectBaseX  "DLSSG.BidirectionalDistortionFieldSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionFieldSubrectBaseY  "DLSSG.BidirectionalDistortionFieldSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionFieldSubrectWidth  "DLSSG.BidirectionalDistortionFieldSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionFieldSubrectHeight "DLSSG.BidirectionalDistortionFieldSubrectHeight"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionField_LowPrecision_IsLowPrecision "DLSSG.BidirectionalDistortionFieldLowPrecision.IsLowPrecision"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionField_LowPrecision_Bias           "DLSSG.BidirectionalDistortionFieldLowPrecision.Bias"
#define NVSDK_NGX_DLSSG_Parameter_BidirectionalDistortionField_LowPrecision_Scale          "DLSSG.BidirectionalDistortionFieldLowPrecision.Scale"

#define NVSDK_NGX_DLSSG_Parameter_OutputInterpolatedSubrectBaseX  "DLSSG.OutputInterpolatedSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_OutputInterpolatedSubrectBaseY  "DLSSG.OutputInterpolatedSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_OutputInterpolatedSubrectWidth  "DLSSG.OutputInterpolatedSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_OutputInterpolatedSubrectHeight "DLSSG.OutputInterpolatedSubrectHeight"

#define NVSDK_NGX_DLSSG_Parameter_OutputRealSubrectBaseX          "DLSSG.OutputRealSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_OutputRealSubrectBaseY          "DLSSG.OutputRealSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_OutputRealSubrectWidth          "DLSSG.OutputRealSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_OutputRealSubrectHeight         "DLSSG.OutputRealSubrectHeight"

// DEPRECATED. Backbuffer subrect definitions. These apply to BOTH the input and
// output backbuffer resources (Backbuffer, OutputInterpolated, OutputReal).
//
// Use InputBackbufferSubrect, OutputInterpolatedSubrect, and OutputRealSubrect
// instead.
#define NVSDK_NGX_DLSSG_Parameter_BackbufferSubrectBaseX  "DLSSG.BackbufferSubrectBaseX"
#define NVSDK_NGX_DLSSG_Parameter_BackbufferSubrectBaseY  "DLSSG.BackbufferSubrectBaseY"
#define NVSDK_NGX_DLSSG_Parameter_BackbufferSubrectWidth  "DLSSG.BackbufferSubrectWidth"
#define NVSDK_NGX_DLSSG_Parameter_BackbufferSubrectHeight "DLSSG.BackbufferSubrectHeight"

// Specifies the internal resolution to be used (mainly for dynamic resolution)
#define NVSDK_NGX_DLSSG_Parameter_InternalWidth      "DLSSG.InternalWidth"
#define NVSDK_NGX_DLSSG_Parameter_InternalHeight     "DLSSG.InternalHeight"

// If nonzero, we expect to use dynamic resolution (MVec and Depth res will change, possibly per frame)
#define NVSDK_NGX_DLSSG_Parameter_DynamicResolution  "DLSSG.DynamicResolution"

typedef enum NVSDK_NGX_DLSSG_EvalFlags
{
   // The default behaviour for evaluate is to write the interpolated pixels inside of the
   // backbuffer extent and uninterpolated pixels - outside of the extent. If this flag is
   // set, then only the inside of the backbuffer extent is updated, the outside of the extent
   // stays untouched.
   NVSDK_NGX_DLSSG_EvalFlags_UpdateOnlyInsideExtents = (1 << 0)
} NVSDK_NGX_DLSSG_EvalFlags;
#define NVSDK_NGX_DLSSG_Parameter_EvalFlags "DLSSG.EvalFlags"

typedef enum NVSDK_NGX_DLSSG_FullscreenMode
{
    NVSDK_NGX_DLSSG_FullscreenMode_Windowed = 0,
    NVSDK_NGX_DLSSG_FullscreenMode_Borderless = 1,
    NVSDK_NGX_DLSSG_FullscreenMode_Exclusive = 2
} NVSDK_NGX_DLSSG_FullscreenMode;

#define NVSDK_NGX_DLSSG_Parameter_FullscreenMode "DLSSG.FullscreenMode"
#define NVSDK_NGX_DLSSG_Parameter_TargetFrameRate "DLSSG.TargetFrameRate"

// Callback function types
typedef NVSDK_NGX_Result(NVSDK_CONV* PFN_NVSDK_NGX_DLSSG_GetCurrentSettingsCallback)(const NVSDK_NGX_Handle* InHandle, NVSDK_NGX_Parameter* InParams);
typedef NVSDK_NGX_Result(NVSDK_CONV* PFN_NVSDK_NGX_DLSSG_EstimateVRAMCallback)(uint32_t mvecDepthWidth, uint32_t mvecDepthHeight,
    uint32_t colorWidth, uint32_t colorHeight,
    uint32_t colorBufferFormat,
    uint32_t mvecBufferFormat, uint32_t depthBufferFormat,
    uint32_t hudLessBufferFormat, uint32_t uiBufferFormat, size_t* estimatedVRAMInBytes);

// DLSS 4

// Output Capability Parameters (access via GetDeviceCapabilityParameters)
// The maximum number of generated frames supported by the feature (e.g. 3 for 4x)
// If 1 or not set, multiframe is not supported
#define NVSDK_NGX_DLSSG_Parameter_MultiFrameCountMax            "DLSSG.MultiFrameCountMax"

// InParameters
// uint32_t, Total of Number Frames expected to Interpolate for given Real-Frame Pair. For 4x, this is 3. For 2x, this is 1.
#define NVSDK_NGX_DLSSG_Parameter_MultiFrameCount               "DLSSG.MultiFrameCount"
// uint32_t, Current inner frame index. For 4x, this is 1, 2, and 3. For 2x, this is 1.
#define NVSDK_NGX_DLSSG_Parameter_MultiFrameIndex               "DLSSG.MultiFrameIndex"

// Optional: uint64_t that increments by one for each fully rendered backbuffer frame such that
//   Current Backbuffer ID == (Previous Backbuffer ID + 1). The number should increment even
//   if FG is off, for example, when rendering a menu with FG temporarily disabled.
#define NVSDK_NGX_DLSSG_Parameter_BackbufferFrameID            "DLSSG.BackbufferFrameID"

// Create Time Parameter: Specifies if User Interface Recomposition is enabled.
// When enabled, the algorithm will generate output frames using the HUDless and UI Color & Alpha textures if present.
// Boolean value: 0 (disabled) or 1 (enabled).
// The end user can override this with Nvidia App DLSS Override.
#define NVSDK_NGX_DLSSG_Parameter_UserInterfaceRecompositionEnabled "DLSSG.UserInterfaceRecompositionEnabled"

#endif // NVSDK_NGX_DEFS_DLSSG_H


```

## Source file: include/nvsdk_ngx_params_dlssd.h

{
  "path": "include/nvsdk_ngx_params_dlssd.h",
  "blob_sha": "6950ed305ba4d2eba1a2eab3c6463fec9ce10e6d",
  "sha256": "d41e3ba8f1d72583b55273092c99bf426dc4a0d46de827229e25d5b1325a370e",
  "lines": 36,
  "bytes": 1199,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 9,
    "rig": 2
  },
  "url": "https://github.com/NVIDIA/DLSS/blob/374959484e79a640feaba44c93ac8cfb0a03f5b5/include/nvsdk_ngx_params_dlssd.h",
  "function_signature_candidates": []
}

```text

/*
 * SPDX-FileCopyrightText: Copyright (c) 2023 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: LicenseRef-NvidiaProprietary
 *
 * NVIDIA CORPORATION, its affiliates and licensors retain all intellectual
 * property and proprietary rights in and to this material, related
 * documentation and any modifications thereto. Any use, reproduction,
 * disclosure or distribution of this material and related documentation
 * without an express license agreement from NVIDIA CORPORATION or
 * its affiliates is strictly prohibited.
 */


#ifndef NVSDK_NGX_PARAMS_DLSSD_H
#define NVSDK_NGX_PARAMS_DLSSD_H

#include "nvsdk_ngx_defs_dlssd.h"

typedef struct NVSDK_NGX_DLSSD_Create_Params
{
    NVSDK_NGX_DLSS_Denoise_Mode InDenoiseMode;
    NVSDK_NGX_DLSS_Roughness_Mode InRoughnessMode;
    NVSDK_NGX_DLSS_Depth_Type InUseHWDepth;

    unsigned int InWidth;
    unsigned int InHeight;
    unsigned int InTargetWidth;
    unsigned int InTargetHeight;
    /*** OPTIONAL ***/
    NVSDK_NGX_PerfQuality_Value InPerfQualityValue;
    int     InFeatureCreateFlags;
    bool    InEnableOutputSubrects;
} NVSDK_NGX_DLSSD_Create_Params;


#endif // #define NVSDK_NGX_PARAMS_DLSSD_H


```

## Source file: include/nvsdk_ngx_params_dlssg.h

{
  "path": "include/nvsdk_ngx_params_dlssg.h",
  "blob_sha": "69c6715e28456380a92e658a482d243f6c96b43e",
  "sha256": "c24fa5aa68ffa2808c828b75cfa32843f1606f80a145f84055ac450d7e6c74ad",
  "lines": 138,
  "bytes": 6137,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "dlss": 8,
    "rig": 4,
    "lip": 11
  },
  "url": "https://github.com/NVIDIA/DLSS/blob/374959484e79a640feaba44c93ac8cfb0a03f5b5/include/nvsdk_ngx_params_dlssg.h",
  "function_signature_candidates": []
}

```text

#pragma once
/*
* Copyright (c) 2021-2024 NVIDIA CORPORATION.  All rights reserved.
*
* NVIDIA Corporation and its licensors retain all intellectual property and proprietary
* rights in and to this software, related documentation and any modifications thereto.
* Any use, reproduction, disclosure or distribution of this software and related
* documentation without an express license agreement from NVIDIA Corporation is strictly
* prohibited.
*
* TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, THIS SOFTWARE IS PROVIDED *AS IS*
* AND NVIDIA AND ITS SUPPLIERS DISCLAIM ALL WARRANTIES, EITHER EXPRESS OR IMPLIED,
* INCLUDING, BUT NOT LIMITED TO, IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
* PARTICULAR PURPOSE.  IN NO EVENT SHALL NVIDIA OR ITS SUPPLIERS BE LIABLE FOR ANY
* SPECIAL, INCIDENTAL, INDIRECT, OR CONSEQUENTIAL DAMAGES WHATSOEVER (INCLUDING, WITHOUT
* LIMITATION, DAMAGES FOR LOSS OF BUSINESS PROFITS, BUSINESS INTERRUPTION, LOSS OF
* BUSINESS INFORMATION, OR ANY OTHER PECUNIARY LOSS) ARISING OUT OF THE USE OF OR
* INABILITY TO USE THIS SOFTWARE, EVEN IF NVIDIA HAS BEEN ADVISED OF THE POSSIBILITY OF
* SUCH DAMAGES.
*/


#ifndef NVSDK_NGX_PARAMS_DLSSG_H
#define NVSDK_NGX_PARAMS_DLSSG_H

#include <stdint.h> // for uint64_t

#include "nvsdk_ngx.h" // DX11/12 here
#include "nvsdk_ngx_defs.h"

typedef struct NVSDK_NGX_DLSSG_Create_Params
{
    unsigned int Width;
    unsigned int Height;
    unsigned int NativeBackbufferFormat;
    unsigned int RenderWidth;
    unsigned int RenderHeight;
    bool DynamicResolutionScaling;
} NVSDK_NGX_DLSSG_Create_Params;

typedef struct NVSDK_NGX_DLSSG_Opt_Eval_Params
{
    //! Number of intermediate frames to generate between rendered frames
    //! (1 = 2x, 2 = 3x, etc.)
    unsigned int multiFrameCount = 1;
    //! The index (starting at 1) of the intermediate frame being generated, up
    //! to multiFrameCount (inclusive)
    unsigned int multiFrameIndex = 1;

    //! IMPORTANT: All matrices should NOT contain temporal AA jitter offset
    //! Specifies matrix transformation from the camera view to the clip space.
    float cameraViewToClip[4][4];
    //! Specifies matrix transformation from the clip space to the camera view space.
    float clipToCameraView[4][4];
    //! Specifies matrix transformation describing lens distortion in clip space.
    float clipToLensClip[4][4];
    //! Specifies matrix transformation from the current clip to the previous clip space.
    float clipToPrevClip[4][4];
    //! Specifies matrix transformation from the previous clip to the current clip space.
    float prevClipToClip[4][4];

    //! Specifies clip space jitter offset
    float jitterOffset[2];
    //! Specifies scale factors used to normalize motion vectors (so the values are in [-1,1] range)
    float mvecScale[2];
    //! Specifies camera pinhole offset.
    float cameraPinholeOffset[2];
    //! Specifies camera position in world space.
    float cameraPos[3];
    //! Specifies camera up vector in world space.
    float cameraUp[3];
    //! Specifies camera right vector in world space.
    float cameraRight[3];
    //! Specifies camera forward vector in world space.
    float cameraFwd[3];

    //! Specifies camera near view plane distance.
    float cameraNear;
    //! Specifies camera far view plane distance.
    float cameraFar;
    //! Specifies camera field of view in radians.
    float cameraFOV;
    //! Specifies camera aspect ratio defined as view space width divided by height.
    float cameraAspectRatio;

    //! Specifies if tagged color buffers are full HDR (rendering to an HDR monitor) or not
    bool colorBuffersHDR;
    //! Specifies if depth values are inverted (value closer to the camera is higher) or not.
    bool depthInverted;
    //! Specifies if camera motion is included in the MVec buffer.
    bool cameraMotionIncluded;
    //! Specifies if previous frame has no connection to the current one (motion vectors are invalid)
    bool reset;
    //! Specifies if Automode overrode the input Reset flag to true.
    bool automodeOverrideReset;
    //! Specifies if application is not currently rendering game frames (paused in menu, playing video cut-scenes)
    bool notRenderingGameFrames;
    //! Specifies if orthographic projection is used or not.
    bool orthoProjection;

    //! Specifies which value represents an invalid (un-initialized) value in the motion vectors buffer
    float motionVectorsInvalidValue;
    //! Specifies if motion vectors are already dilated or not.
    bool motionVectorsDilated;

    //! Specifies whether or not fullscreen menu detection should be run
    bool menuDetectionEnabled;

    NVSDK_NGX_Coordinates mvecsSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions mvecsSubrectSize = { 0, 0 };
    NVSDK_NGX_Coordinates depthSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions depthSubrectSize = { 0, 0 };

    NVSDK_NGX_Coordinates hudLessSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions hudLessSubrectSize = { 0, 0 };
    NVSDK_NGX_Coordinates uiSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions uiSubrectSize = { 0, 0 };
    NVSDK_NGX_Coordinates uiAlphaSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions uiAlphaSubrectSize = { 0, 0 };

    NVSDK_NGX_Coordinates bidirectionalDistFieldSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions bidirectionalDistFieldSubrectSize = { 0, 0 };
    NVSDK_NGX_PrecisionInfo bidirectionalDistFieldPrecisionInfo = { 0, 0.0f, 0.0f };

    //! Optional heuristic that specifies the minimum depth difference between two objects in screen-space.
    //! See comment above NVSDK_NGX_DLSSG_Parameter_MinRelativeLinearDepthObjectSeparation for details.
    float minRelativeLinearDepthObjectSeparation = 40.0f;

    NVSDK_NGX_Coordinates backbufferSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions  backbufferSubrectSize = { 0, 0 };

    NVSDK_NGX_Coordinates outputInterpSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions  outputInterpSubrectSize = { 0, 0 };
    NVSDK_NGX_Coordinates outputRealSubrectBase = { 0, 0 };
    NVSDK_NGX_Dimensions  outputRealSubrectSize = { 0, 0 };
} NVSDK_NGX_DLSSG_Opt_Eval_Params;

#endif // NVSDK_NGX_PARAMS_DLSSG_H


```

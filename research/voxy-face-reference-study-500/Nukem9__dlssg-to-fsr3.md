# Facial rendering source scan: Nukem9/dlssg-to-fsr3

Repository: https://github.com/Nukem9/dlssg-to-fsr3

Pinned source revision: 0e253f63c3c66ee7582406bfa3dec698df7ab2c1

Discovery topic: DLSS

Description: Adds AMD FSR 3 Frame Generation to games by replacing Nvidia DLSS Frame Generation (nvngx_dlssg).

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "dependencies/FidelityFX-SDK/LICENSE.txt", "dependencies/FidelityFX-SDK/docs/license.md", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/AGS/LICENSE.txt", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/acs/LICENSE.txt", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/imgui/LICENSE.txt", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/json/License.txt", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/memoryallocator/license.txt", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/vectormath/LICENSE.txt", "dependencies/FidelityFX-SDK/sdk/LICENSE.txt", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/host/backends/dx12/license.txt", "dependencies/FidelityFX-SDK/sdk/tools/ffx_shader_compiler/libs/MD5/LICENSE", "dependencies/FidelityFX-SDK/sdk/tools/ffx_shader_compiler/libs/SPIRV-Reflect/LICENSE", "dependencies/FidelityFX-SDK/sdk/tools/ffx_shader_compiler/libs/tiny-process-library/LICENSE"]

Topic paths: {"skin": ["dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/computeskinning.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/src/render/rendermodules/skinning/skinningrendermodule.cpp", "dependencies/FidelityFX-SDK/framework/cauldron/framework/src/render/rendermodules/skinning/skinningrendermodule.h"], "sss": ["dependencies/FidelityFX-SDK/docs/samples/media/stochastic-screen-space-reflections/sssr-sample.jpg", "dependencies/FidelityFX-SDK/docs/samples/media/stochastic-screen-space-reflections/sssr-sample_resized.jpg", "dependencies/FidelityFX-SDK/docs/techniques/media/stochastic-screen-space-reflections/sssr-sample.jpg", "dependencies/FidelityFX-SDK/samples/hybridreflections/shaders/ffx_sssr_intersect.h", "dependencies/FidelityFX-SDK/samples/sssr/CMakeLists.txt", "dependencies/FidelityFX-SDK/samples/sssr/config/sssrconfig.json", "dependencies/FidelityFX-SDK/samples/sssr/shaders/sssr_apply_reflections.hlsl", "dependencies/FidelityFX-SDK/samples/sssr/shaders/sssr_apply_reflections_common.h", "dependencies/FidelityFX-SDK/samples/sssr/sssrrendermodule.cpp", "dependencies/FidelityFX-SDK/samples/sssr/sssrrendermodule.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/CMakeCompileSSSRShaders.txt", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_callbacks_glsl.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_callbacks_hlsl.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_classify_tiles.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_common.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_depth_downsample.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_intersect.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_prepare_blue_noise_texture.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_prepare_indirect_args.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/sssr/ffx_sssr_resources.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/host/ffx_sssr.h", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/CMakeShadersSSSR.txt", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/sssr/ffx_sssr_classify_tiles_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/sssr/ffx_sssr_depth_downsample_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/sssr/ffx_sssr_intersect_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/sssr/ffx_sssr_prepare_blue_noise_texture_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/sssr/ffx_sssr_prepare_indirect_args_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/shared/blob_accessors/ffx_sssr_shaderblobs.cpp", "dependencies/FidelityFX-SDK/sdk/src/backends/shared/blob_accessors/ffx_sssr_shaderblobs.h", "dependencies/FidelityFX-SDK/sdk/src/backends/vk/CMakeShadersSSSR.txt"], "shader": ["dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/render/shaderbuilder.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/render/shaderbuilderhelper.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/commonintersect.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/computeskinning.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/copytexture.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/fpslimiter/fpslimiter.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/fullscreen.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/lightfunctionscommon.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/lightingcommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/particlecommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/rasterlightfunctions.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/raytracingcommon.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/raytracinglightfunctions.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/shadercommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/shadowFiltering.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/surfacerendercommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/autoexposure.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/builddistortionfield.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/hudlessblit.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/lensdistortion.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/tonemappercommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/tonemappers.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/tonemapping/tonemapping.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/transferFunction.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/transformVS.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/ui.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/uicommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/upscaler.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/AGS/ags_shader_intrinsics_dx12.hlsl", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/agilitysdk/include/d3d12shader.h"], "dlss": ["resources/dlssg_to_fsr3.ini", "resources/read_me_dlsstweaks.txt"], "rig": ["dependencies/FidelityFX-SDK/docs/techniques/media/parallel-sort/parallel-sort-algorightm.jpg"], "face": ["dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/core/backend_implementations/backend_interface_dx12.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/core/backend_implementations/backend_interface_vk.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/core/backend_interface.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/surfacerendercommon.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/libs/dxheaders/include/directx/dxcore_interface.h", "dependencies/FidelityFX-SDK/framework/cauldron/framework/src/core/backend_implementations/backend_interface_dx12.cpp", "dependencies/FidelityFX-SDK/framework/cauldron/framework/src/core/backend_implementations/backend_interface_vk.cpp", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/brixelizergi/ffx_brixelizergi_host_interface.h", "dependencies/FidelityFX-SDK/sdk/include/FidelityFX/host/ffx_interface.h", "source/maindll/FFInterfaceWrapper.cpp", "source/maindll/FFInterfaceWrapper.h"], "lip": ["dependencies/FidelityFX-SDK/sdk/include/FidelityFX/gpu/fsr2/ffx_fsr2_depth_clip.h", "dependencies/FidelityFX-SDK/sdk/src/backends/dx12/shaders/fsr2/ffx_fsr2_depth_clip_pass.hlsl", "dependencies/FidelityFX-SDK/sdk/src/backends/vk/shaders/fsr2/ffx_fsr2_depth_clip_pass.glsl"]}

## Source file: dependencies/FidelityFX-SDK/samples/sssr/shaders/sssr_apply_reflections.hlsl

{
  "path": "dependencies/FidelityFX-SDK/samples/sssr/shaders/sssr_apply_reflections.hlsl",
  "blob_sha": "15a3a0cde1bfdb4ce3807771c2c480a70a9c89b8",
  "sha256": "4015192c97a97000af9b5030841678dfd509d919b22b5841647ed31d71ba7546",
  "lines": 75,
  "bytes": 3353,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 1,
    "shader": 1,
    "rig": 4
  },
  "url": "https://github.com/Nukem9/dlssg-to-fsr3/blob/0e253f63c3c66ee7582406bfa3dec698df7ab2c1/dependencies/FidelityFX-SDK/samples/sssr/shaders/sssr_apply_reflections.hlsl",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "float3 getIBLContribution(float perceptualRoughness, float3 f0, float3 specularLight, float3 n, float3 v)"
    },
    {
      "line": 49,
      "text": "float4 ps_main(VertexOut input) : SV_Target0"
    }
  ]
}

```text

// This file is part of the FidelityFX SDK.
//
// Copyright (C) 2024 Advanced Micro Devices, Inc.
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#include "sssr_apply_reflections_common.h"

Texture2D<float4> reflectionTarget              : register(t0);
Texture2D<float4> normalsTexture                : register(t1);
Texture2D<float4> baseColorAlphaTexture         : register(t2);
Texture2D<float4> aoRoughnessMetallicTexture    : register(t3);
Texture2D<float4> brdfTexture                   : register(t4);

SamplerState linearSampler : register(s0);

struct VertexOut
{
    float4 position : SV_Position;
    float2 texcoord : TEXCOORD0;
};

// Important bits from the PBR shader
float3 getIBLContribution(float perceptualRoughness, float3 f0, float3 specularLight, float3 n, float3 v)
{
    float NdotV = clamp(dot(n, v), 0.0, 1.0);
    float2 brdfSamplePoint = saturate(float2(NdotV, 1.0 - perceptualRoughness));
    float2 brdf = brdfTexture.Sample(linearSampler, brdfSamplePoint).rg;

    return specularLight * (f0 * brdf.x + brdf.y);;
}

float4 ps_main(VertexOut input) : SV_Target0
{
    float3 radiance = reflectionTarget.Sample(linearSampler, input.texcoord).xyz;
    float4 aoRoughnessMetallic = aoRoughnessMetallicTexture.Sample(linearSampler, input.texcoord);
    float perceptualRoughness = sqrt(aoRoughnessMetallic.g); // aoRoughnessMetallic.g contains alphaRoughness
    float3 baseColor = baseColorAlphaTexture.Sample(linearSampler, input.texcoord).rgb;
    float3 f0 = lerp(float3(0.04, 0.04, 0.04), baseColor, float3(aoRoughnessMetallic.b, aoRoughnessMetallic.b, aoRoughnessMetallic.b));
    float3 normal = 2.0 * normalsTexture.Sample(linearSampler, input.texcoord).xyz - 1.0;
    float3 view = viewDirection.xyz;

    if (showReflectionTarget == 1)
    {
        // Show just the reflection view
        return float4(radiance, 0);
    }
    else if (drawReflections == 1)
    {
        // reflectionsIntensity should always be 1 for PBR correctness. We expose it in the UI to help visualize reflections better.
        radiance = getIBLContribution(perceptualRoughness, f0, radiance, normal, view) * reflectionsIntensity;
        return float4(radiance, 1); // Show the reflections applied to the scene
    }
    else
    {
        // Show just the scene
        return float4(0, 0, 0, 1);
    }
}


```

## Source file: dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/computeskinning.hlsl

{
  "path": "dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/computeskinning.hlsl",
  "blob_sha": "720cb440d489dc809eb236b502c091577002094a",
  "sha256": "b671f4e987350b04f68e1700ed764ef2a3498e2782d3c64b85d892767310f082",
  "lines": 105,
  "bytes": 4358,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 17,
    "rig": 4,
    "face": 1
  },
  "url": "https://github.com/Nukem9/dlssg-to-fsr3/blob/0e253f63c3c66ee7582406bfa3dec698df7ab2c1/dependencies/FidelityFX-SDK/framework/cauldron/framework/inc/shaders/computeskinning.hlsl",
  "function_signature_candidates": [
    {
      "line": 70,
      "text": "void MainCS(uint3 dtID : SV_DispatchThreadID)"
    }
  ]
}

```text

// This file is part of the FidelityFX SDK.
//
// Copyright (C) 2024 Advanced Micro Devices, Inc.
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#include "surfacerendercommon.h"

cbuffer CBBones : register(b0)
{
    MatrixPair bones[MAX_NUM_BONES];
};

cbuffer CBStrides : register(b1)
{
    VertexStrides vertexStrides;
}

matrix GetCurrentSkinningMatrix(float4 Weights, uint4 Joints)
{
    matrix skinningMatrix =
        Weights.x * bones[Joints.x].Current +
        Weights.y * bones[Joints.y].Current +
        Weights.z * bones[Joints.z].Current +
        Weights.w * bones[Joints.w].Current;

    return skinningMatrix;
}
matrix GetPreviousSkinningMatrix(float4 Weights, uint4 Joints)
{
    matrix skinningMatrix =
        Weights.x * bones[Joints.x].Previous +
        Weights.y * bones[Joints.y].Previous +
        Weights.z * bones[Joints.z].Previous +
        Weights.w * bones[Joints.w].Previous;
    return skinningMatrix;
}

// Inputs
ByteAddressBuffer positions : register(t0);
ByteAddressBuffer normals :   register(t1);
ByteAddressBuffer weights :   register(t2);
ByteAddressBuffer joints :    register(t3);

// Outputs
RWByteAddressBuffer currentSkinnedPositions : register(u0);
RWByteAddressBuffer prevSkinnedPositions    : register(u1);
RWByteAddressBuffer currentSkinnedNormals   : register(u2);

//--------------------------------------------------------------------------------------
// Main function
//--------------------------------------------------------------------------------------
[numthreads(64, 1, 1)]
void MainCS(uint3 dtID : SV_DispatchThreadID)
{
    if (dtID.x >= vertexStrides.numVertices)
        return;

    const uint positionIndex = dtID.x * vertexStrides.positionStride;
    const uint normalIndex   = dtID.x * vertexStrides.normalStride;
    const uint weightsIndex  = dtID.x * vertexStrides.weights0Stride;
    const uint jointsIndex   = dtID.x * vertexStrides.joints0Stride;

    const float3 vertexPosition = asfloat(positions.Load3(positionIndex));
    const float3 vertexNormal   = DeCompressNormals(asfloat(normals.Load3(normalIndex)));
    const uint4  vertexJoints   = vertexStrides.joints0Stride == 8 ? uint4(asuint(joints.Load2(jointsIndex)), 0, 0) : asuint(joints.Load4(jointsIndex));
    const float4 vertexWeights  = asfloat(weights.Load4(weightsIndex));

    // Shorts
    uint4 jointsValues = vertexJoints;
    if (vertexStrides.joints0Stride == 8)
    {
        jointsValues.x = (vertexJoints.x & 0xFFFF);
        jointsValues.y = (vertexJoints.x >> 16);
        jointsValues.z = (vertexJoints.y & 0xFFFF);
        jointsValues.w = (vertexJoints.y >> 16);
    }

    matrix skinningMatrix = GetCurrentSkinningMatrix(vertexWeights, jointsValues);
    matrix prevSkinningMatrix = GetPreviousSkinningMatrix(vertexWeights, jointsValues);

    const float3 positionSkinned     = mul(skinningMatrix, float4(vertexPosition, 1)).xyz;
    const float3 normalSkinned       = normalize(mul(skinningMatrix, float4(vertexNormal, 1)).xyz);
    const float3 prevPositionSkinned = mul(prevSkinningMatrix, float4(vertexPosition, 1)).xyz;;

    currentSkinnedPositions.Store3(positionIndex, asuint(positionSkinned));
    currentSkinnedNormals.Store3(normalIndex, asuint(CompressNormals(normalSkinned)));
    prevSkinnedPositions.Store3(positionIndex, asuint(prevPositionSkinned));
}


```

## Source file: dependencies/FidelityFX-SDK/sdk/src/backends/shared/blob_accessors/ffx_sssr_shaderblobs.h

{
  "path": "dependencies/FidelityFX-SDK/sdk/src/backends/shared/blob_accessors/ffx_sssr_shaderblobs.h",
  "blob_sha": "0ee55423b2f1a885595d4f2a87bf9a6ec648c753",
  "sha256": "29f51324875b7829a9bb48fb10b578961c2c83652c7d41e3d66ab61a29b5dab4",
  "lines": 44,
  "bytes": 1746,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 4,
    "shader": 3,
    "rig": 4
  },
  "url": "https://github.com/Nukem9/dlssg-to-fsr3/blob/0e253f63c3c66ee7582406bfa3dec698df7ab2c1/dependencies/FidelityFX-SDK/sdk/src/backends/shared/blob_accessors/ffx_sssr_shaderblobs.h",
  "function_signature_candidates": []
}

```text

// This file is part of the FidelityFX SDK.
//
// Copyright (C) 2024 Advanced Micro Devices, Inc.
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#pragma once

#include "../ffx_shader_blobs.h"
#include <FidelityFX/host/ffx_sssr.h>
#include <stdint.h>

#if defined(__cplusplus)
extern "C" {
#endif // #if defined(__cplusplus)

// Get a DX12 shader blob for the specified pass and permutation index.
FfxErrorCode sssrGetPermutationBlobByIndex(
    FfxSssrPass passId,
    uint32_t permutationOptions,
    FfxShaderBlob* outBlob);

// Check is Wave64 is requested on this permutation
FfxErrorCode sssrIsWave64(uint32_t permutationOptions, bool& isWave64);

#if defined(__cplusplus)
}
#endif // #if defined(__cplusplus)


```

## Source file: dependencies/FidelityFX-SDK/sdk/src/backends/vk/shaders/sssr/ffx_sssr_intersect_pass.glsl

{
  "path": "dependencies/FidelityFX-SDK/sdk/src/backends/vk/shaders/sssr/ffx_sssr_intersect_pass.glsl",
  "blob_sha": "c0b86ca7aafb08f50e0b469d6a41ccd59fb38b59",
  "sha256": "83ac0513c8d7e44b537b06b90fe9e030b9058f05b13c0359c62ae889c8cd3e76",
  "lines": 64,
  "bytes": 2588,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 25,
    "shader": 1,
    "rig": 4
  },
  "url": "https://github.com/Nukem9/dlssg-to-fsr3/blob/0e253f63c3c66ee7582406bfa3dec698df7ab2c1/dependencies/FidelityFX-SDK/sdk/src/backends/vk/shaders/sssr/ffx_sssr_intersect_pass.glsl",
  "function_signature_candidates": [
    {
      "line": 61,
      "text": "void main()"
    }
  ]
}

```text

// This file is part of the FidelityFX SDK.
//
// Copyright (C) 2024 Advanced Micro Devices, Inc.
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#version 450

#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_samplerless_texture_functions : require

#define FFX_WAVE 1
#extension GL_KHR_shader_subgroup_basic : require

#define SSSR_BIND_SRV_INPUT_COLOR              0
#define SSSR_BIND_SRV_INPUT_NORMAL             1
#define SSSR_BIND_SRV_INPUT_ENVIRONMENT_MAP    2
#define SSSR_BIND_SRV_DEPTH_HIERARCHY          3
#define SSSR_BIND_SRV_EXTRACTED_ROUGHNESS      4
#define SSSR_BIND_SRV_BLUE_NOISE_TEXTURE       5

#define SSSR_BIND_UAV_RADIANCE    2000
#define SSSR_BIND_UAV_RAY_LIST    2001
#define SSSR_BIND_UAV_RAY_COUNTER 2002

#define SSSR_BIND_CB_SSSR   3000

#include "sssr/ffx_sssr_callbacks_glsl.h"
#include "sssr/ffx_sssr_intersect.h"

#ifndef FFX_SSSR_THREAD_GROUP_WIDTH
#define FFX_SSSR_THREAD_GROUP_WIDTH 8
#endif // #ifndef FFX_SSSR_THREAD_GROUP_WIDTH
#ifndef FFX_SSSR_THREAD_GROUP_HEIGHT
#define FFX_SSSR_THREAD_GROUP_HEIGHT 8
#endif // FFX_SSSR_THREAD_GROUP_HEIGHT
#ifndef FFX_SSSR_THREAD_GROUP_DEPTH
#define FFX_SSSR_THREAD_GROUP_DEPTH 1
#endif // #ifndef FFX_SSSR_THREAD_GROUP_DEPTH
#ifndef FFX_SSSR_NUM_THREADS
#define FFX_SSSR_NUM_THREADS layout (local_size_x = FFX_SSSR_THREAD_GROUP_WIDTH, local_size_y = FFX_SSSR_THREAD_GROUP_HEIGHT, local_size_z = FFX_SSSR_THREAD_GROUP_DEPTH) in;
#endif // #ifndef FFX_SSSR_NUM_THREADS

FFX_SSSR_NUM_THREADS
void main()
{
    Intersect(gl_LocalInvocationIndex, gl_WorkGroupID.x);
}


```

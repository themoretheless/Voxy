# Facial rendering source scan: skore11/GPU_voxelSkinning_MassSpringtest

Repository: https://github.com/skore11/GPU_voxelSkinning_MassSpringtest

Pinned source revision: 2b9b78354434e68e50a6c08e0e4f8f77485e2bf2

Discovery topic: skin shader

Description: Applying a compute shader for GPU voxel skinning of a vertex based mesh

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["Assets/PostProcessing/Resources/Bluenoise64/COPYING.txt", "Assets/PostProcessing/Resources/Bluenoise64/COPYING.txt.meta", "Assets/PostProcessing/Resources/Bluenoise64/LICENSE.txt", "Assets/PostProcessing/Resources/Bluenoise64/LICENSE.txt.meta"]

Topic paths: {"eye": ["Assets/PostProcessing/Editor/Models/EyeAdaptationModelEditor.cs", "Assets/PostProcessing/Editor/Models/EyeAdaptationModelEditor.cs.meta", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc.meta", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader.meta", "Assets/PostProcessing/Resources/Shaders/EyeHistogram.compute", "Assets/PostProcessing/Resources/Shaders/EyeHistogram.compute.meta", "Assets/PostProcessing/Runtime/Components/EyeAdaptationComponent.cs", "Assets/PostProcessing/Runtime/Components/EyeAdaptationComponent.cs.meta", "Assets/PostProcessing/Runtime/Models/EyeAdaptationModel.cs", "Assets/PostProcessing/Runtime/Models/EyeAdaptationModel.cs.meta", "Assets/Raw Mocap Data/DefaultAvatar.fbm/eyes_color_map.png", "Assets/Raw Mocap Data/DefaultAvatar.fbm/eyes_color_map.png.meta", "Assets/Raw Mocap Data/DefaultAvatar.fbm/eyes_specular_map.png", "Assets/Raw Mocap Data/DefaultAvatar.fbm/eyes_specular_map.png.meta", "Assets/Raw Mocap Data/Materials/DefaultAvatar-eyes.mat", "Assets/Raw Mocap Data/Materials/DefaultAvatar-eyes.mat.meta", "Assets/Raw Mocap Data/Materials/eyes_specular_map.png", "Assets/Raw Mocap Data/Materials/eyes_specular_map.png.meta"], "skin": ["Assets/Packages/Voxelizer/Demo/Scenes/GPUVoxelSkinnedMesh.asset", "Assets/Packages/Voxelizer/Demo/Scenes/GPUVoxelSkinnedMesh.asset.meta", "Assets/Packages/Voxelizer/Demo/Scenes/GPUVoxelSkinnedMesh.unity", "Assets/Packages/Voxelizer/Demo/Scenes/GPUVoxelSkinnedMesh.unity.meta", "Assets/Packages/Voxelizer/Demo/Scripts/GPUVoxelSkinnedMesh.cs", "Assets/Packages/Voxelizer/Demo/Scripts/GPUVoxelSkinnedMesh.cs.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelSkinnedMesh.compute", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelSkinnedMesh.compute.meta", "Captures/GPUVoxelSkinnedMesh.gif"], "sss": ["Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpawner.cs", "Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpawner.cs.meta", "Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpringGPUTest.cs", "Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpringGPUTest.cs.meta", "Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpringSystem.cs", "Assets/Packages/Voxelizer/Scripts/SpringSystem/MassSpringSystem.cs.meta", "Assets/Packages/Voxelizer/Shaders/MassSpringSystem.meta", "Assets/Packages/Voxelizer/Shaders/MassSpringSystem/MassSpringSystem.compute", "Assets/Packages/Voxelizer/Shaders/MassSpringSystem/MassSpringSystem.compute.meta"], "shader": ["Assets/Packages/Voxelizer/Demo/Materials/InstancedShader.shader", "Assets/Packages/Voxelizer/Demo/Materials/InstancedShader.shader.meta", "Assets/Packages/Voxelizer/Demo/Shaders.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/CuboidLine.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/CuboidLine.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/CuboidLine.shader", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/CuboidLine.shader.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelMosaic.compute", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelMosaic.compute.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelParticleSystem.compute", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelParticleSystem.compute.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelParticleSystem.shader", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelParticleSystem.shader.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelSkinnedMesh.compute", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/GPUVoxelSkinnedMesh.compute.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/ClassicNoise3D.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/ClassicNoise3D.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/SimplexNoise3D.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/SimplexNoise3D.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/SimplexNoiseGrad3D.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Noise/SimplexNoiseGrad3D.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Quaternion.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Quaternion.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Random.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/Random.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/StandardGeometry.cginc", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/StandardGeometry.cginc.meta", "Assets/Packages/Voxelizer/Demo/Shaders/GPUVoxelParticleSystem/VParticle.cginc"], "face": ["Assets/Packages/Voxelizer/UnityOptixCSharp/OptixLibraryInterface.cs", "Assets/Packages/Voxelizer/UnityOptixCSharp/OptixLibraryInterface.cs.meta", "Assets/Raw Mocap Data/DefaultAvatar.fbm/face normal.png", "Assets/Raw Mocap Data/DefaultAvatar.fbm/face normal.png.meta", "Assets/Raw Mocap Data/DefaultAvatar.fbm/face_color_map.png", "Assets/Raw Mocap Data/DefaultAvatar.fbm/face_color_map.png.meta", "Assets/Raw Mocap Data/Materials/DefaultAvatar-face.mat", "Assets/Raw Mocap Data/Materials/DefaultAvatar-face.mat.meta", "Assets/Raw Mocap Data/Materials/face normal.png", "Assets/Raw Mocap Data/Materials/face normal.png.meta"]}

## Source file: Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc

{
  "path": "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc",
  "blob_sha": "f2d1e1def735552d7d9404eefa44f9803086d674",
  "sha256": "b73daa93a9bc8fdc3b6444378081dd79dc33c36a6b35c20bec35c785a9251748",
  "lines": 22,
  "bytes": 695,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 4
  },
  "url": "https://github.com/skore11/GPU_voxelSkinning_MassSpringtest/blob/2b9b78354434e68e50a6c08e0e4f8f77485e2bf2/Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "float GetHistogramBinFromLuminance(float value, float2 scaleOffset)"
    },
    {
      "line": 17,
      "text": "float GetLuminanceFromHistogramBin(float bin, float2 scaleOffset)"
    }
  ]
}

```text

#ifndef __EYE_ADAPTATION__
#define __EYE_ADAPTATION__

// Optimal values for PS4/GCN
// Using a group size of 32x32 seems to be a bit faster on Kepler/Maxwell
// Don't forget to update 'EyeAdaptationController.cs' if you change these values !
#define HISTOGRAM_BINS          64
#define HISTOGRAM_TEXELS        HISTOGRAM_BINS / 4
#define HISTOGRAM_THREAD_X      16
#define HISTOGRAM_THREAD_Y      16

float GetHistogramBinFromLuminance(float value, float2 scaleOffset)
{
    return saturate(log2(value) * scaleOffset.x + scaleOffset.y);
}

float GetLuminanceFromHistogramBin(float bin, float2 scaleOffset)
{
    return exp2((bin - scaleOffset.y) / scaleOffset.x);
}

#endif // __EYE_ADAPTATION__


```

## Source file: Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader

{
  "path": "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader",
  "blob_sha": "8ec509d6ff4823a8b0b390f3d2fea27ca0bd6bbc",
  "sha256": "66d645c07d7c6331646bd90b60b957bb478e6bd3284c6a1d30fc355289664e0f",
  "lines": 218,
  "bytes": 6596,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 3,
    "shader": 3,
    "rig": 3,
    "lip": 1
  },
  "url": "https://github.com/skore11/GPU_voxelSkinning_MassSpringtest/blob/2b9b78354434e68e50a6c08e0e4f8f77485e2bf2/Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "float GetBinValue(uint index, float maxHistogramValue)"
    },
    {
      "line": 30,
      "text": "float FindMaxHistogramValue()"
    },
    {
      "line": 43,
      "text": "void FilterLuminance(uint i, float maxHistogramValue, inout float4 filter)"
    },
    {
      "line": 62,
      "text": "float GetAverageLuminance(float maxHistogramValue)"
    },
    {
      "line": 86,
      "text": "float GetExposureMultiplier(float avgLuminance)"
    },
    {
      "line": 101,
      "text": "float InterpolateExposure(float newExposure, float oldExposure)"
    },
    {
      "line": 110,
      "text": "float4 FragAdaptProgressive(VaryingsDefault i) : SV_Target"
    },
    {
      "line": 120,
      "text": "float4 FragAdaptFixed(VaryingsDefault i) : SV_Target"
    },
    {
      "line": 149,
      "text": "float4 FragEditorHisto(VaryingsEditorHisto i) : SV_Target"
    }
  ]
}

```text

Shader "Hidden/Post FX/Eye Adaptation"
{
    Properties
    {
        _MainTex("Texture", 2D) = "white" {}
    }

    CGINCLUDE

        #pragma target 4.5
        #pragma multi_compile __ AUTO_KEY_VALUE
        #include "UnityCG.cginc"
        #include "Common.cginc"
        #include "EyeAdaptation.cginc"

        // Eye adaptation pass
        float4 _Params; // x: lowPercent, y: highPercent, z: minBrightness, w: maxBrightness
        float2 _Speed; // x: down, y: up
        float4 _ScaleOffsetRes; // x: scale, y: offset, w: histogram pass width, h: histogram pass height
        float _ExposureCompensation;

        StructuredBuffer<uint> _Histogram;

        float GetBinValue(uint index, float maxHistogramValue)
        {
            return float(_Histogram[index]) * maxHistogramValue;
        }

        // Done in the vertex shader
        float FindMaxHistogramValue()
        {
            uint maxValue = 0u;

            for (uint i = 0; i < HISTOGRAM_BINS; i++)
            {
                uint h = _Histogram[i];
                maxValue = max(maxValue, h);
            }

            return float(maxValue);
        }

        void FilterLuminance(uint i, float maxHistogramValue, inout float4 filter)
        {
            float binValue = GetBinValue(i, maxHistogramValue);

            // Filter dark areas
            float offset = min(filter.z, binValue);
            binValue -= offset;
            filter.zw -= offset.xx;

            // Filter highlights
            binValue = min(filter.w, binValue);
            filter.w -= binValue;

            // Luminance at the bin
            float luminance = GetLuminanceFromHistogramBin(float(i) / float(HISTOGRAM_BINS), _ScaleOffsetRes.xy);

            filter.xy += float2(luminance * binValue, binValue);
        }

        float GetAverageLuminance(float maxHistogramValue)
        {
            // Sum of all bins
            uint i;
            float totalSum = 0.0;

            UNITY_LOOP
            for (i = 0; i < HISTOGRAM_BINS; i++)
                totalSum += GetBinValue(i, maxHistogramValue);

            // Skip darker and lighter parts of the histogram to stabilize the auto exposure
            // x: filtered sum
            // y: accumulator
            // zw: fractions
            float4 filter = float4(0.0, 0.0, totalSum * _Params.xy);

            UNITY_LOOP
            for (i = 0; i < HISTOGRAM_BINS; i++)
                FilterLuminance(i, maxHistogramValue, filter);

            // Clamp to user brightness range
            return clamp(filter.x / max(filter.y, EPSILON), _Params.z, _Params.w);
        }

        float GetExposureMultiplier(float avgLuminance)
        {
            avgLuminance = max(EPSILON, avgLuminance);

        #if AUTO_KEY_VALUE
            half keyValue = 1.03 - (2.0 / (2.0 + log2(avgLuminance + 1.0)));
        #else
            half keyValue = _ExposureCompensation;
        #endif

            half exposure = keyValue / avgLuminance;

            return exposure;
        }

        float InterpolateExposure(float newExposure, float oldExposure)
        {
            float delta = newExposure - oldExposure;
            float speed = delta > 0.0 ? _Speed.x : _Speed.y;
            float exposure = oldExposure + delta * (1.0 - exp2(-unity_DeltaTime.x * speed));
            //float exposure = oldExposure + delta * (unity_DeltaTime.x * speed);
            return exposure;
        }

        float4 FragAdaptProgressive(VaryingsDefault i) : SV_Target
        {
            float maxValue = 1.0 / FindMaxHistogramValue();
            float avgLuminance = GetAverageLuminance(maxValue);
            float exposure = GetExposureMultiplier(avgLuminance);
            float prevExposure = tex2D(_MainTex, (0.5).xx);
            exposure = InterpolateExposure(exposure, prevExposure);
            return exposure.xxxx;
        }

        float4 FragAdaptFixed(VaryingsDefault i) : SV_Target
        {
            float maxValue = 1.0 / FindMaxHistogramValue();
            float avgLuminance = GetAverageLuminance(maxValue);
            float exposure = GetExposureMultiplier(avgLuminance);
            return exposure.xxxx;
        }

        // ---- Editor stuff
        int _DebugWidth;

        struct VaryingsEditorHisto
        {
            float4 pos : SV_POSITION;
            float2 uv : TEXCOORD0;
            float maxValue : TEXCOORD1;
            float avgLuminance : TEXCOORD2;
        };

        VaryingsEditorHisto VertEditorHisto(AttributesDefault v)
        {
            VaryingsEditorHisto o;
            o.pos = UnityObjectToClipPos(v.vertex);
            o.uv = v.texcoord.xy;
            o.maxValue = 1.0 / FindMaxHistogramValue();
            o.avgLuminance = GetAverageLuminance(o.maxValue);
            return o;
        }

        float4 FragEditorHisto(VaryingsEditorHisto i) : SV_Target
        {
            const float3 kRangeColor = float3(0.05, 0.4, 0.6);
            const float3 kAvgColor = float3(0.8, 0.3, 0.05);

            float4 color = float4(0.0, 0.0, 0.0, 0.7);

            uint ix = (uint)(round(i.uv.x * HISTOGRAM_BINS));
            float bin = saturate(float(_Histogram[ix]) * i.maxValue);
            float fill = step(i.uv.y, bin);

            // Min / max brightness markers
            float luminanceMin = GetHistogramBinFromLuminance(_Params.z, _ScaleOffsetRes.xy);
            float luminanceMax = GetHistogramBinFromLuminance(_Params.w, _ScaleOffsetRes.xy);

            color.rgb += fill.rrr;

            if (i.uv.x > luminanceMin && i.uv.x < luminanceMax)
            {
                color.rgb = fill.rrr * kRangeColor;
                color.rgb += kRangeColor;
            }

            // Current average luminance marker
            float luminanceAvg = GetHistogramBinFromLuminance(i.avgLuminance, _ScaleOffsetRes.xy);
            float avgPx = luminanceAvg * _DebugWidth;

            if (abs(i.pos.x - avgPx) < 2)
                color.rgb = kAvgColor;

            return color;
        }

    ENDCG

    SubShader
    {
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            CGPROGRAM

                #pragma vertex VertDefault
                #pragma fragment FragAdaptProgressive

            ENDCG
        }

        Pass
        {
            CGPROGRAM

                #pragma vertex VertDefault
                #pragma fragment FragAdaptFixed

            ENDCG
        }

        Pass
        {
            CGPROGRAM

                #pragma vertex VertEditorHisto
                #pragma fragment FragEditorHisto

            ENDCG
        }
    }
}


```

## Source file: Assets/Packages/Voxelizer/Shaders/MassRenderShader.shader

{
  "path": "Assets/Packages/Voxelizer/Shaders/MassRenderShader.shader",
  "blob_sha": "0865b04fd931645aa6a25ea73b46863b31e1bc15",
  "sha256": "bd485558384f454f994243af0d8ef13e103aabbef631425165d06a59276455ee",
  "lines": 56,
  "bytes": 1838,
  "selection_score": 27,
  "keyword_line_counts": {
    "shader": 4
  },
  "url": "https://github.com/skore11/GPU_voxelSkinning_MassSpringtest/blob/2b9b78354434e68e50a6c08e0e4f8f77485e2bf2/Assets/Packages/Voxelizer/Shaders/MassRenderShader.shader",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "float4 frag (ps_input i) : COLOR"
    }
  ]
}

```text

﻿/**
 * A very simple shader that can be used to render the positions, velocities, or some other debugging-speific properties, of data points.  
 */

Shader "Custom/MassRenderShader" 
{
	SubShader
    {
        Pass
        {
 
            CGPROGRAM
            #pragma target 5.0
 
            #pragma vertex vert
            #pragma fragment frag
 
            #include "UnityCG.cginc"
 
            //The buffer containing the points we want to draw.
            StructuredBuffer<float3> buf_Points;
            StructuredBuffer<float3> buf_Debug;
            StructuredBuffer<float3> buf_Vels;
            //A simple input struct for our pixel shader step containing a position.
            struct ps_input 
            {
                float4 pos : SV_POSITION;
                float4 vel : NORMAL;
            };
 
            //Our vertex function simply fetches a point from the buffer corresponding to the vertex index
            //which we transform with the view-projection matrix before passing to the pixel program.
            ps_input vert (uint id : SV_VertexID)
            {
                ps_input o;
                float3 worldPos = buf_Points[id];
                o.pos = mul (UNITY_MATRIX_VP, float4 (worldPos, 1.0f));
                o.vel = float4 (abs(buf_Vels[id]), 1.0f);
                return o;
            }
 
            //Pixel function returns a solid color for each point.
            //Currently we debug the velocity of each point using the r and g values of the colour.
            float4 frag (ps_input i) : COLOR
            {
                float4 debugCol  = float4 (1.0f - abs (i.vel.x), abs (i.vel.y), 0.0f, 1.0f);
                float4 renderCol = float4 (1.0f, 1.0f, 1.0f, 1.0f);
                return debugCol;
            }
 
            ENDCG
 
        }
    }
    Fallback Off
}


```

## Source file: Assets/Packages/Voxelizer/Demo/Shaders/InstancedShader.shader

{
  "path": "Assets/Packages/Voxelizer/Demo/Shaders/InstancedShader.shader",
  "blob_sha": "5a25ab063296205e2cceeaaf9444fd68ed12ebf7",
  "sha256": "c5d089c780a38112a093aef1a16840c6004f02d07194bfbfd7d95d9b76fdd7f7",
  "lines": 76,
  "bytes": 1710,
  "selection_score": 25,
  "keyword_line_counts": {
    "shader": 2,
    "face": 2
  },
  "url": "https://github.com/skore11/GPU_voxelSkinning_MassSpringtest/blob/2b9b78354434e68e50a6c08e0e4f8f77485e2bf2/Assets/Packages/Voxelizer/Demo/Shaders/InstancedShader.shader",
  "function_signature_candidates": [
    {
      "line": 36,
      "text": "void rotate2D(inout float2 v, float r)"
    },
    {
      "line": 43,
      "text": "void setup()"
    },
    {
      "line": 65,
      "text": "void surf(Input IN, inout SurfaceOutputStandard o)"
    }
  ]
}

```text

﻿Shader "Instanced/InstancedShader"
{
	Properties
	{
		_MainTex("Albedo (RGB)", 2D) = "white" {}
		_Glossiness("Smoothness", Range(0,1)) = 0.5
		_Metallic("Metallic", Range(0,1)) = 0.0
		_PointColor("Point Color", Color) = (1, 1, 1, 1)
	}
	SubShader
	{
		Tags
		{ 
			"RenderType" = "Opaque" 
		}

		LOD 200

		CGPROGRAM

		// Physically based Standard lighting model
#pragma surface surf Standard addshadow
#pragma multi_compile_instancing
#pragma instancing_options procedural:setup

		sampler2D _MainTex;

		struct Input {
			float2 uv_MainTex;
		};

#ifdef UNITY_PROCEDURAL_INSTANCING_ENABLED
		StructuredBuffer<float4> positionBuffer;
#endif

		void rotate2D(inout float2 v, float r)
		{
			float s, c;
			sincos(r, s, c);
			v = float2(v.x * c - v.y * s, v.x * s + v.y * c);
		}

		void setup()
		{
#ifdef UNITY_PROCEDURAL_INSTANCING_ENABLED
			float4 data = positionBuffer[unity_InstanceID];

			float rotation = data.w * data.w * 0.5f;
			rotate2D(data.xz, rotation);

			unity_ObjectToWorld._11_21_31_41 = float4(data.w, 0, 0, 0);
			unity_ObjectToWorld._12_22_32_42 = float4(0, data.w, 0, 0);
			unity_ObjectToWorld._13_23_33_43 = float4(0, 0, data.w, 0);
			unity_ObjectToWorld._14_24_34_44 = float4(data.xyz, 1);
			unity_WorldToObject = unity_ObjectToWorld;
			unity_WorldToObject._14_24_34 *= -1;
			unity_WorldToObject._11_22_33 = 1.0f / unity_WorldToObject._11_22_33;
#endif
		}

		half _Glossiness;
		half _Metallic;
		float4 _PointColor;

		void surf(Input IN, inout SurfaceOutputStandard o)
		{
			fixed4 c = tex2D(_MainTex, IN.uv_MainTex) * _PointColor;
			o.Albedo = c.rgb;
			o.Metallic = _Metallic;
			o.Smoothness = _Glossiness;
			o.Alpha = c.a;
		}
		ENDCG
	}
	FallBack "Diffuse"
}


```

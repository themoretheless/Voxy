# Facial rendering source scan: Xerxes1138/ScreenSpaceSubsurfaceScattering

Repository: https://github.com/Xerxes1138/ScreenSpaceSubsurfaceScattering

Pinned source revision: 115cf3de1fe47892a3b51171c77ef9253d886386

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["Assets/PostProcessing/Resources/Bluenoise64/COPYING.txt", "Assets/PostProcessing/Resources/Bluenoise64/COPYING.txt.meta", "Assets/PostProcessing/Resources/Bluenoise64/LICENSE.txt", "Assets/PostProcessing/Resources/Bluenoise64/LICENSE.txt.meta", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt.meta", "LICENSE"]

Topic paths: {"eye": ["Assets/PostProcessing/Editor/Models/EyeAdaptationModelEditor.cs", "Assets/PostProcessing/Editor/Models/EyeAdaptationModelEditor.cs.meta", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.cginc.meta", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader", "Assets/PostProcessing/Resources/Shaders/EyeAdaptation.shader.meta", "Assets/PostProcessing/Resources/Shaders/EyeHistogram.compute", "Assets/PostProcessing/Resources/Shaders/EyeHistogram.compute.meta", "Assets/PostProcessing/Runtime/Components/EyeAdaptationComponent.cs", "Assets/PostProcessing/Runtime/Components/EyeAdaptationComponent.cs.meta", "Assets/PostProcessing/Runtime/Models/EyeAdaptationModel.cs", "Assets/PostProcessing/Runtime/Models/EyeAdaptationModel.cs.meta"], "skin": ["Assets/Art/Demo/Models/Budha/Materials/mat_Budha_Skin.mat", "Assets/Art/Demo/Models/Budha/Materials/mat_Budha_Skin.mat.meta"], "subsurface": ["Assets/ScreenSpaceSubsurfaceScattering.meta", "Assets/ScreenSpaceSubsurfaceScattering/Default.asset", "Assets/ScreenSpaceSubsurfaceScattering/Default.asset.meta", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/StandardShaderGUI.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/StandardShaderGUI.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/ScreenSpaceSubSurfaceScattering.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/ScreenSpaceSubSurfaceScattering.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/inxhh3lk.uvx~", "Assets/ScreenSpaceSubsurfaceScattering/Shaders.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc.meta"], "sss": ["Assets/Art/Demo/Textures/Snow_Ground_SSS.png", "Assets/Art/Demo/Textures/Snow_Ground_SSS.png.meta", "Assets/Art/Demo/Textures/Stone_Puddle_SSS.tga", "Assets/Art/Demo/Textures/Stone_Puddle_SSS.tga.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSUtils.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSUtils.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSYCbCr.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSYCbCr.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSTemplate.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSTemplate.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSWater.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSWater.shader.meta", "SSS.png", "SSSColor.png"], "shader": ["Assets/PostProcessing/Editor Resources/Monitors/HistogramRender.shader", "Assets/PostProcessing/Editor Resources/Monitors/HistogramRender.shader.meta", "Assets/PostProcessing/Editor Resources/Monitors/ParadeRender.shader", "Assets/PostProcessing/Editor Resources/Monitors/ParadeRender.shader.meta", "Assets/PostProcessing/Editor Resources/Monitors/VectorscopeRender.shader", "Assets/PostProcessing/Editor Resources/Monitors/VectorscopeRender.shader.meta", "Assets/PostProcessing/Editor Resources/Monitors/WaveformRender.shader", "Assets/PostProcessing/Editor Resources/Monitors/WaveformRender.shader.meta", "Assets/PostProcessing/Editor Resources/UI/CurveBackground.shader", "Assets/PostProcessing/Editor Resources/UI/CurveBackground.shader.meta", "Assets/PostProcessing/Editor Resources/UI/Trackball.shader", "Assets/PostProcessing/Editor Resources/UI/Trackball.shader.meta", "Assets/PostProcessing/Resources/Shaders.meta", "Assets/PostProcessing/Resources/Shaders/ACES.cginc", "Assets/PostProcessing/Resources/Shaders/ACES.cginc.meta", "Assets/PostProcessing/Resources/Shaders/AmbientOcclusion.cginc", "Assets/PostProcessing/Resources/Shaders/AmbientOcclusion.cginc.meta", "Assets/PostProcessing/Resources/Shaders/AmbientOcclusion.shader", "Assets/PostProcessing/Resources/Shaders/AmbientOcclusion.shader.meta", "Assets/PostProcessing/Resources/Shaders/Blit.shader", "Assets/PostProcessing/Resources/Shaders/Blit.shader.meta", "Assets/PostProcessing/Resources/Shaders/Bloom.cginc", "Assets/PostProcessing/Resources/Shaders/Bloom.cginc.meta", "Assets/PostProcessing/Resources/Shaders/Bloom.shader", "Assets/PostProcessing/Resources/Shaders/Bloom.shader.meta", "Assets/PostProcessing/Resources/Shaders/BuiltinDebugViews.shader", "Assets/PostProcessing/Resources/Shaders/BuiltinDebugViews.shader.meta", "Assets/PostProcessing/Resources/Shaders/ColorGrading.cginc", "Assets/PostProcessing/Resources/Shaders/ColorGrading.cginc.meta", "Assets/PostProcessing/Resources/Shaders/Common.cginc"], "rig": ["Assets/Art/Demo/Prefabs/pref_CameraRig.prefab", "Assets/Art/Demo/Prefabs/pref_CameraRig.prefab.meta"], "face": ["Assets/ScreenSpaceSubsurfaceScattering.meta", "Assets/ScreenSpaceSubsurfaceScattering/Default.asset", "Assets/ScreenSpaceSubsurfaceScattering/Default.asset.meta", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt", "Assets/ScreenSpaceSubsurfaceScattering/LICENSE.txt.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/SSSSProfileEditor.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/StandardShaderGUI.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/Editor/StandardShaderGUI.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/SSSSProfile.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/ScreenSpaceSubSurfaceScattering.cs", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/ScreenSpaceSubSurfaceScattering.cs.meta", "Assets/ScreenSpaceSubsurfaceScattering/Scripts/inxhh3lk.uvx~", "Assets/ScreenSpaceSubsurfaceScattering/Shaders.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSBRDF.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferred.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSDeferredReflections.shader.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSGBuffer.cginc.meta", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc", "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SSSSPBSLighting.cginc.meta"]}

## Source file: Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSWater.shader

{
  "path": "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSWater.shader",
  "blob_sha": "7328964694da6b02bc48be66b712eb751bacc302",
  "sha256": "0318af1601067cc0d383dc43ead325a9caba09091dbf98caae6547acc7498231",
  "lines": 138,
  "bytes": 5109,
  "selection_score": 60,
  "keyword_line_counts": {
    "subsurface": 4,
    "sss": 16,
    "shader": 3,
    "rig": 4,
    "face": 7
  },
  "url": "https://github.com/Xerxes1138/ScreenSpaceSubsurfaceScattering/blob/115cf3de1fe47892a3b51171c77ef9253d886386/Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSWater.shader",
  "function_signature_candidates": [
    {
      "line": 79,
      "text": "float3 GetWaterWavesNormal(sampler2D tex, float2 uv, float scale, float bump)"
    },
    {
      "line": 103,
      "text": "void surf (Input IN, inout SurfaceOutputStandardSSSS o)"
    }
  ]
}

```text

﻿//MIT License
//
//Copyright(c) 2018 Charles Thomas
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files(the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions :
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.
//

Shader "Xerxes1138/Surface Shader Templates/Subsurface Scattering Water" 
{
	Properties 
	{
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		[NoScaleOffset]_MetallicGlossMap ("Metallic (R), Smoothness (A)", 2D) = "white" {}
		[NoScaleOffset]_BumpMap ("Normal (RGB)", 2D) = "bump" {}
		[NoScaleOffset]_OcclusionMap ("Occlusion (G)", 2D) = "white" {}

		[Header(Sub Surface Scattering)]
		[NoScaleOffset]_SSSTex("SSS Color (RGB), SSS Radius (A)", 2D) = "white" {}
		_SSSColor("Color", Color) = (1.0, 0.3, 0.3,1)
		_SSSRadius ("Radius", Range(0.0, 1.0)) = 0.5

		[Header(Transmittance)]
		[NoScaleOffset]_TransmittanceTex ("Thickness", 2D) = "white" {}
		_TransmittanceExp ("Exponent", Range(1.0, 8.0)) = 1.0
		_Transmittance ("Scale", Range(0.0, 1.0)) = 1.0
	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200

		CGPROGRAM
		#pragma surface surf StandardSSSS exclude_path:prepass

		// SM 3.0 is required
		#pragma target 3.0

		// Screen Space Subsurface Scattering
		#define _MATERIAL_MODEL_SSS // Define our shading model

		#include "SSSSPBSLighting.cginc"

		sampler2D _MainTex; // RGB = Diffuse or specular color
		sampler2D _BumpMap; // RGB = Normal
		sampler2D _MetallicGlossMap; // R = Metallic, A = Smoothness
		sampler2D _OcclusionMap; // G = Occlusion

		half4 _Color;
		half _Glossiness;
		half _Metallic;

		// SSS
		sampler2D _SSSTex; // RGB = SSS color, A = SSS radius
		sampler2D _TransmittanceTex; // B = Transmittance

        half4 _SSSColor;
		half _SSSRadius;
		half _Transmittance;
		half _TransmittanceExp;

		float3 GetWaterWavesNormal(sampler2D tex, float2 uv, float scale, float bump)
		{
			float3 tangentNormal = float3(0, 0, 1);
			tangentNormal.xy += UnpackNormal(tex2D(tex, (uv / 8 * scale) + _Time.y * float2(0.2, 0.0))).xy * 0.5;
			tangentNormal.xy += UnpackNormal(tex2D(tex, (uv / 8 * scale) + _Time.y * float2(-0.2, 0.0) + float2(0.25, -0.25))).xy * 0.5;
			tangentNormal = lerp(float3(0, 0, 1), tangentNormal, bump);

			return tangentNormal;
		}

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_CBUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_CBUFFER_END

        struct Input 
		{
            float2 uv_MainTex;
			float3 worldPos;
            float4 screenPos; // screenPos is required for the interleaved pattern
        };

		void surf (Input IN, inout SurfaceOutputStandardSSSS o) 
		{
		    float2 screenUV = IN.screenPos.xy / max(1e-5f, IN.screenPos.w);
            int2 pos = screenUV * _ScreenParams.xy;

			half4 albedo = tex2D (_MainTex, IN.uv_MainTex) * _Color;
			half4 metallicGloss = tex2D(_MetallicGlossMap, IN.uv_MainTex);
			metallicGloss.r *= _Metallic;
			metallicGloss.a *= _Glossiness; 
			half3 normal = GetWaterWavesNormal(_BumpMap, IN.worldPos.xz, 6, 0.06); // UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
			half occlusion = tex2D(_OcclusionMap, IN.uv_MainTex).g;

			half4 sssColorAndRadius = tex2D(_SSSTex, IN.uv_MainTex);
			sssColorAndRadius.rgb *= _SSSColor.rgb;
			sssColorAndRadius.a *= _SSSRadius;

			half transmittance = pow(sqrt(tex2D(_TransmittanceTex, IN.uv_MainTex).b) * _Transmittance, _TransmittanceExp) * (1.0f - metallicGloss.r);

			o.Albedo = albedo.rgb;
			o.Metallic = metallicGloss.r;
			o.Smoothness = metallicGloss.a;
			o.Normal = normal;
			o.Occlusion = occlusion;
			o.Alpha = albedo.a;

            o.SubSurfaceScatteringColorAndRadius.rgb = sssColorAndRadius.rgb;
			o.SubSurfaceScatteringColorAndRadius.a = sssColorAndRadius.a * (1.0f - metallicGloss.r);

            o.Transmittance = transmittance;

            o.Interleaved = GetPattern (pos);
		}
		ENDCG
	}
	FallBack "Diffuse"
}

```

## Source file: Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSTemplate.shader

{
  "path": "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSTemplate.shader",
  "blob_sha": "5bcfc1418c4466e13b30ca57add378dbae076b4d",
  "sha256": "a6e38c729a7a3c9b12786dfe55e2cc1fe4da7de7986b040e23ae89d0148900f0",
  "lines": 127,
  "bytes": 4570,
  "selection_score": 60,
  "keyword_line_counts": {
    "subsurface": 4,
    "sss": 16,
    "shader": 3,
    "rig": 4,
    "face": 7
  },
  "url": "https://github.com/Xerxes1138/ScreenSpaceSubsurfaceScattering/blob/115cf3de1fe47892a3b51171c77ef9253d886386/Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderSSSTemplate.shader",
  "function_signature_candidates": [
    {
      "line": 92,
      "text": "void surf (Input IN, inout SurfaceOutputStandardSSSS o)"
    }
  ]
}

```text

﻿//MIT License
//
//Copyright(c) 2018 Charles Thomas
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files(the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions :
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.
//

Shader "Xerxes1138/Surface Shader Templates/Subsurface Scattering" 
{
	Properties 
	{
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		[NoScaleOffset]_MetallicGlossMap ("Metallic (R), Smoothness (A)", 2D) = "white" {}
		[NoScaleOffset]_BumpMap ("Normal (RGB)", 2D) = "bump" {}
		[NoScaleOffset]_OcclusionMap ("Occlusion (G)", 2D) = "white" {}

		[Header(Sub Surface Scattering)]
		[NoScaleOffset]_SSSTex("SSS Color (RGB), SSS Radius (A)", 2D) = "white" {}
		_SSSColor("Color", Color) = (1.0, 0.3, 0.3,1)
		_SSSRadius ("Radius", Range(0.0, 1.0)) = 0.5

		[Header(Transmittance)]
		[NoScaleOffset]_TransmittanceTex ("Thickness", 2D) = "white" {}
		_TransmittanceExp ("Exponent", Range(1.0, 8.0)) = 1.0
		_Transmittance ("Scale", Range(0.0, 1.0)) = 1.0
	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200

		CGPROGRAM
		#pragma surface surf StandardSSSS exclude_path:prepass

		// SM 3.0 is required
		#pragma target 3.0

		// Screen Space Subsurface Scattering
		#define _MATERIAL_MODEL_SSS // Define our shading model

		#include "SSSSPBSLighting.cginc"

		sampler2D _MainTex; // RGB = Diffuse or specular color
		sampler2D _BumpMap; // RGB = Normal
		sampler2D _MetallicGlossMap; // R = Metallic, A = Smoothness
		sampler2D _OcclusionMap; // G = Occlusion

		half4 _Color;
		half _Glossiness;
		half _Metallic;

		// SSS
		sampler2D _SSSTex; // RGB = SSS color, A = SSS radius
		sampler2D _TransmittanceTex; // B = Transmittance

        half4 _SSSColor;
		half _SSSRadius;
		half _Transmittance;
		half _TransmittanceExp;

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_CBUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_CBUFFER_END

        struct Input 
		{
            float2 uv_MainTex;
            float4 screenPos; // screenPos is required for the interleaved pattern
        };

		void surf (Input IN, inout SurfaceOutputStandardSSSS o) 
		{
		    float2 screenUV = IN.screenPos.xy / max(1e-5f, IN.screenPos.w);
            int2 pos = screenUV * _ScreenParams.xy;

			half4 albedo = tex2D (_MainTex, IN.uv_MainTex) * _Color;
			half4 metallicGloss = tex2D(_MetallicGlossMap, IN.uv_MainTex);
			metallicGloss.r *= _Metallic;
			metallicGloss.a *= _Glossiness; 
			half3 normal = UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
			half occlusion = tex2D(_OcclusionMap, IN.uv_MainTex).g;

			half4 sssColorAndRadius = tex2D(_SSSTex, IN.uv_MainTex);
			sssColorAndRadius.rgb *= _SSSColor.rgb;
			sssColorAndRadius.a *= _SSSRadius;

			half transmittance = pow(sqrt(tex2D(_TransmittanceTex, IN.uv_MainTex).b) * _Transmittance, _TransmittanceExp) * (1.0f - metallicGloss.r);

			o.Albedo = albedo.rgb;
			o.Metallic = metallicGloss.r;
			o.Smoothness = metallicGloss.a;
			o.Normal = normal;
			o.Occlusion = occlusion;
			o.Alpha = albedo.a;

            o.SubSurfaceScatteringColorAndRadius.rgb = sssColorAndRadius.rgb;
			o.SubSurfaceScatteringColorAndRadius.a = sssColorAndRadius.a * (1.0f - metallicGloss.r);

            o.Transmittance = transmittance;

            o.Interleaved = GetPattern (pos);
		}
		ENDCG
	}
	FallBack "Diffuse"
}

```

## Source file: Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderTemplate.shader

{
  "path": "Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderTemplate.shader",
  "blob_sha": "9af77f813b86e0898f3290da47a0a9bf6ec60811",
  "sha256": "29d561ebd48f3d37e1572c65264eb266de17987d7d2c2ed5d2b6c20a780163dd",
  "lines": 91,
  "bytes": 3188,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 1,
    "sss": 3,
    "shader": 3,
    "rig": 4,
    "face": 4
  },
  "url": "https://github.com/Xerxes1138/ScreenSpaceSubsurfaceScattering/blob/115cf3de1fe47892a3b51171c77ef9253d886386/Assets/ScreenSpaceSubsurfaceScattering/Shaders/SurfaceShaderTemplate.shader",
  "function_signature_candidates": [
    {
      "line": 72,
      "text": "void surf (Input IN, inout SurfaceOutputStandardSSSS o)"
    }
  ]
}

```text

﻿//MIT License
//
//Copyright(c) 2018 Charles Thomas
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files(the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions :
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.
//

Shader "Xerxes1138/Surface Shader Templates/Standard" 
{
	Properties 
	{
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		[NoScaleOffset]_MetallicGlossMap ("Metallic (R), Smoothness (A)", 2D) = "white" {}
		[NoScaleOffset]_BumpMap ("Normal (RGB)", 2D) = "bump" {}
		[NoScaleOffset]_OcclusionMap ("Occlusion (G)", 2D) = "white" {}
	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200

		CGPROGRAM
		#pragma surface surf StandardSSSS exclude_path:prepass

		// SM 3.0 is required
		#pragma target 3.0

		// Screen Space Subsurface Scattering
		#define _MATERIAL_MODEL_STANDARD // Define our shading model

		#include "SSSSPBSLighting.cginc"

		sampler2D _MainTex; // RGB = Diffuse or specular color
		sampler2D _BumpMap; // RGB = Normal
		sampler2D _MetallicGlossMap; // R = Metallic, A = Smoothness
		sampler2D _OcclusionMap; // G = Occlusion

		half4 _Color;
		half _Glossiness;
		half _Metallic;

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_CBUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_CBUFFER_END

        struct Input 
		{
            float2 uv_MainTex;
        };

		void surf (Input IN, inout SurfaceOutputStandardSSSS o)
		{
			half4 albedo = tex2D (_MainTex, IN.uv_MainTex) * _Color;
			half4 metallicGloss = tex2D(_MetallicGlossMap, IN.uv_MainTex);
			metallicGloss.r *= _Metallic;
			metallicGloss.a *= _Glossiness; 
			half3 normal = UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
			half occlusion = tex2D(_OcclusionMap, IN.uv_MainTex).g;

			o.Albedo = albedo.rgb;
			o.Metallic = metallicGloss.r;
			o.Smoothness = metallicGloss.a;
			o.Normal = normal;
			o.Occlusion = occlusion;
			o.Alpha = albedo.a;
		}
		ENDCG
	}
	FallBack "Diffuse"
}

```

## Source file: Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader

{
  "path": "Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader",
  "blob_sha": "9b1dc86a9cfde6a9fd2493fde6c820de07bc71ea",
  "sha256": "47d78daa90fbeec6555816a25393a7885ba69365a4251727ab219505230b3707",
  "lines": 578,
  "bytes": 16691,
  "selection_score": 45,
  "keyword_line_counts": {
    "eye": 4,
    "skin": 1,
    "subsurface": 3,
    "sss": 71,
    "shader": 2,
    "rig": 15,
    "face": 17,
    "lip": 1
  },
  "url": "https://github.com/Xerxes1138/ScreenSpaceSubsurfaceScattering/blob/115cf3de1fe47892a3b51171c77ef9253d886386/Assets/ScreenSpaceSubsurfaceScattering/Shaders/Resources/ScreenSpaceSubsurfaceScattering.shader",
  "function_signature_candidates": [
    {
      "line": 68,
      "text": "float4 SampleColor(sampler2D tex, float2 uv)"
    },
    {
      "line": 73,
      "text": "float SampleRawDepth (float2 uv)"
    },
    {
      "line": 82,
      "text": "float SampleLinearEyeDepth (float2 uv)"
    },
    {
      "line": 87,
      "text": "float4 SampleJitter(float2 uv)"
    },
    {
      "line": 92,
      "text": "void SampleGBuffer(half2 uv, out half4 gbuffer0, out half4 gbuffer1, out half4 gbuffer2)"
    },
    {
      "line": 99,
      "text": "float3 ScreenToWorldSpace (float3 screenPos)"
    },
    {
      "line": 110,
      "text": "float4 SSSAtenuation(float distance, float3 sssColor)"
    },
    {
      "line": 117,
      "text": "float4 SampleSSSDiffuse(float2 uv, float4 diffuse, float depth, bool followSurface)"
    },
    {
      "line": 130,
      "text": "float4 SubSuraceScattering(float2 uv, float2 offsetDirection, float sssRadius, float sssMask, float3 sssColor)"
    },
    {
      "line": 244,
      "text": "float4 SSSBlurPass_X( VertexOutput i ) : SV_Target"
    },
    {
      "line": 255,
      "text": "float4 SSSBlurPass_Y( VertexOutput i ) : SV_Target"
    },
    {
      "line": 266,
      "text": "float4 Dilation( VertexOutput i ) : SV_Target"
    },
    {
      "line": 297,
      "text": "float4 SSSColor( VertexOutput i ) : SV_Target"
    },
    {
      "line": 326,
      "text": "float4 Combine( VertexOutput i ) : SV_Target"
    },
    {
      "line": 386,
      "text": "float4 PrePass ( VertexOutput i ) : SV_Target"
    },
    {
      "line": 399,
      "text": "float4 Compute(VertexOutput i) : SV_Target"
    }
  ]
}

```text

//MIT License
//
//Copyright(c) 2018 Charles Thomas
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files(the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions :
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.
//

Shader "Hidden/Xerxes1138/ScreenSpaceSubsurfaceScattering" 
{ 
	Properties 
	{
		_MainTex ("Base (RGB)", 2D) = "black" {}
	}
	
	CGINCLUDE
	#include			"UnityCG.cginc"
	 
	uniform sampler2D	_CameraGBufferTexture0;
	uniform sampler2D	_CameraGBufferTexture1;
	uniform sampler2D	_CameraGBufferTexture2;
	 
	uniform sampler2D	_MainTex;
	uniform sampler2D	_DiffuseSSSTex;
	uniform sampler2D	_DiffuseTex;
	uniform sampler2D	_SpecularTex;
	uniform sampler2D	_DiffuseSpecularTex;
	uniform sampler2D	_SSSColorTex;

	uniform sampler2D	_CameraDepthTexture;
	uniform sampler2D	_BlueNoiseTex;

	uniform float4		_SSSParams; // x = jitter radius, y = distanceToProjectionWindow, z = aspect ratio correction, w = world unit
	uniform float4		_DiffuseSizeAndDiffuseTexelSize; // x = diffuse width, y = diffuse height, z = 1.0f / diffuse width, w = 1.0f/ diffuse height
	uniform float4		_ScreenSizeAndScreenTexelSize; // x = screen width, y = screen height, z = 1.0f / screen width, w = 1.0f/ screen height
	uniform float4		_JitterSizeAndOffset; // x = jitter width / screen width, y = jitter height / screen height, z = random offset, w = random offset
	uniform float4		_FadeDistanceAndRadius; // x = fade distance, y = fade radius, z = unused, w = unused
	uniform float4		_TemporalAAParams; // x = sample index, y = sample count, z = jitter offset x, w = jitter offset y

	uniform int			_SSS_NUM_SAMPLES;
	uniform int			_DebugPass;

	uniform float4x4	_ProjectionMatrix;
	uniform float4x4	_ViewProjectionMatrix;
	uniform float4x4	_InverseProjectionMatrix;
	uniform float4x4	_InverseViewProjectionMatrix;
	uniform float4x4	_WorldToCameraMatrix;
	uniform float4x4	_CameraToWorldMatrix;

	#include			"../SSSSUtils.cginc"
	#include			"../SSSSGBuffer.cginc"

	float4 SampleColor(sampler2D tex, float2 uv)
	{
		return tex2Dlod(tex, float4(uv, 0.0f, 0.0f));
	}

	float SampleRawDepth (float2 uv)
	{
		float z = SampleColor(_CameraDepthTexture, uv);
		/*#if defined(UNITY_REVERSED_Z)
			z = 1.0f - z;
		#endif*/
		return z;
	}
	
	float SampleLinearEyeDepth (float2 uv)
	{
		return LinearEyeDepth(tex2Dlod(_CameraDepthTexture, float4(uv, 0.0f, 0.0f)));
	}

	float4 SampleJitter(float2 uv)
	{
		return SampleColor(_BlueNoiseTex, uv * _JitterSizeAndOffset.xy + _JitterSizeAndOffset.zw);
	}

	void SampleGBuffer(half2 uv, out half4 gbuffer0, out half4 gbuffer1, out half4 gbuffer2)
	{
		gbuffer0 = SampleColor (_CameraGBufferTexture0, uv);
		gbuffer1 = SampleColor (_CameraGBufferTexture1, uv);
		gbuffer2 = SampleColor (_CameraGBufferTexture2, uv);
	}

	float3 ScreenToWorldSpace (float3 screenPos)
	{
		float4 worldPos = mul(_InverseViewProjectionMatrix, float4(screenPos.xy * 2.0f - 1.0f, screenPos.z, 1.0f));
		return worldPos.xyz / worldPos.w;
	}

	half DecodeSubsurfaceScatteringRadius(half radius)
	{
		return radius * 1.0f / _SSSParams.w; // Divide the sss radius by a world unit value
	}

	float4 SSSAtenuation(float distance, float3 sssColor)
	{
		float3 sssAttenuation = exp2(-Pow2(distance / (1e-3f + sssColor)));

		return float4(sssAttenuation, 1.0f);
	}

	float4 SampleSSSDiffuse(float2 uv, float4 diffuse, float depth, bool followSurface)
	{
		float4 sampledDiffuse = SampleColor(_MainTex, uv);
		float sampledDepth = SampleLinearEyeDepth(uv);

		float delta = saturate(8.0f * _SSSParams.y * abs(sampledDepth - depth));

		if(followSurface)
			sampledDiffuse = lerp(sampledDiffuse, diffuse, delta);
		
		return float4(sampledDiffuse.rgb, 1.0f);
	}

	float4 SubSuraceScattering(float2 uv, float2 offsetDirection, float sssRadius, float sssMask, float3 sssColor)
	{
		float4 diffuse = SampleColor(_MainTex, uv);

		float depth = SampleLinearEyeDepth(uv);

		float2 aspectRatioCorrection = _DiffuseSizeAndDiffuseTexelSize.zw * _DiffuseSizeAndDiffuseTexelSize.y /*_SSSParams.z*/;

		float scale = DecodeSubsurfaceScatteringRadius(sssRadius) * _SSSParams.y / depth;

		float2 step = scale * offsetDirection * sssMask;

		const float range = 2.0f;
		const float exponent = 2.0f;

		float4 jitter = SampleJitter(uv);

		float4 sssAttenuation = 0.0f;
		float4 sssWeights = 0.0f;
		float4 sssDiffuse = 0.0f;
		for (int k = 0; k < _SSS_NUM_SAMPLES; k++)
		{
			float dist = pow(range * ((float)k / _SSS_NUM_SAMPLES), exponent);

			float2 offset = dist * step;

			// Next-Generation Character Rendering Jorge Jimenez 2013	
			if (abs(dist * 1.0f / (range + 1.0f)) < _SSSParams.x)
			{
				offset = float2(dot(offset, float2(jitter.x, jitter.y)), dot(offset, float2(-jitter.z, jitter.w)));
			}

			offset *= aspectRatioCorrection;

			float2 offsetUV = uv + offset;

			sssAttenuation = SSSAtenuation(dist, sssColor);

			sssWeights += sssAttenuation;
			sssDiffuse.rgb += SampleSSSDiffuse(offsetUV, diffuse, depth, true) * sssAttenuation.rgb;
		}
		sssDiffuse.rgb = Max(sssDiffuse.rgb / sssWeights.rgb);

		return float4(sssDiffuse.rgb, sssMask);
	}

	struct VertexInput 
	{
		float4 vertex : POSITION;
		float2 texcoord : TEXCOORD;
	};

	struct VertexOutput
	{
		float4 pos : POSITION;
		float2 uv : TEXCOORD0;
	};

	VertexOutput vert( VertexInput v ) 
	{
		VertexOutput o;
		o.pos = UnityObjectToClipPos(v.vertex);
		o.uv = v.texcoord;
		return o;
	}

	#define SURFACE_SETUP(x) SurfaceParameter x = \
	SurfaceSetup(i.uv, _ScreenSizeAndScreenTexelSize);

	#define SURFACE_SSS_SETUP(x) SurfaceParameter x = \
	SurfaceSetup(i.uv, _DiffuseSizeAndDiffuseTexelSize);

	inline SurfaceParameter SurfaceSetup (float2 i_uv, float4 screenSize)
	{
		int2 pos = i_uv * screenSize.xy;

		float2 du = float2(screenSize.z, 0.0f);
		float2 dv = float2(0.0f, screenSize.w);

		half4 gbuffer0, gbuffer1, gbuffer2;
		SampleGBuffer(i_uv, gbuffer0, gbuffer1, gbuffer2);
		GBufferData data = StandardSSSSDataFromGbuffer(gbuffer0, gbuffer1, gbuffer2);

		bool pattern = GetPattern (pos);
		int shadingModel = DecodeShadingModel(data.shadingModel);
		half sssMask = shadingModel == SHADING_MODEL_SSS ? 1.0f : 0.0f;

		half3 diffColorYCbCr = half3(data.diffColorAndCustomData.rg, 0.0f);

		half2 diffColorLeft = SampleColor (_CameraGBufferTexture0, i_uv + du).rg;
		half2 diffColorRight = SampleColor (_CameraGBufferTexture0, i_uv - du).rg;
		half2 diffColorTop = SampleColor (_CameraGBufferTexture0, i_uv - dv).rg;
		half2 diffColorBottom = SampleColor (_CameraGBufferTexture0, i_uv + dv).rg;

		diffColorYCbCr.b = filter(diffColorYCbCr.rg, diffColorLeft, diffColorRight, diffColorTop, diffColorBottom);
		diffColorYCbCr.rgb = pattern ? diffColorYCbCr.rbg : diffColorYCbCr.rgb;
		diffColorYCbCr.rgb = YCbCrToRGB (diffColorYCbCr);
		 
		half3 diffColor = sssMask ? diffColorYCbCr : data.diffColorAndCustomData.rgb;
		half3 specColor = sssMask ? data.specColorAndCustomData.rrr : data.specColorAndCustomData.rgb;
		half3 sssColor = SampleColor(_SSSColorTex, i_uv);
		      
		SurfaceParameter o = (SurfaceParameter)0.0f;
		o.diffColor = diffColor;
		o.specColor = specColor;
		o.worldNormal = DecodeNormal(data.normalAndCustomData.rg);
		o.shadingModel = shadingModel;
		o.pattern = pattern;
		o.sssColor = sssColor;
		o.sssRadius = data.normalAndCustomData.b;
		o.sssMask = sssMask;
		return o;
	}

	float4 SSSBlurPass_X( VertexOutput i ) : SV_Target
	{	
		SURFACE_SSS_SETUP(s)

		return float4
		(
			SubSuraceScattering(i.uv, float2(1.0f, 0.0f), s.sssRadius, s.sssMask, s.sssColor)+ 
			SubSuraceScattering(i.uv, float2(-1.0f, 0.0f), s.sssRadius, s.sssMask, s.sssColor)
		) / 2.0f;
	}

	float4 SSSBlurPass_Y( VertexOutput i ) : SV_Target
	{	
		SURFACE_SSS_SETUP(s)

		return float4
		(
			SubSuraceScattering(i.uv, float2(0.0f, 1.0f), s.sssRadius, s.sssMask, s.sssColor)+
			SubSuraceScattering(i.uv, float2(0.0f, -1.0f), s.sssRadius, s.sssMask, s.sssColor)
		) / 2.0f;
	}

	float4 Dilation( VertexOutput i ) : SV_Target
	{	
		float2 uv = i.uv; 

		float2 du = float2(_DiffuseSizeAndDiffuseTexelSize.z, 0.0f);
		float2 dv = float2(0.0f, _DiffuseSizeAndDiffuseTexelSize.w);

		//[+du-dv, -dv, -du-dv]
		//[+du	 ,	  , -du	  ]
		//[+du+dv, +dv, -du+dv]

		//[{1,-1}, {0, -1}, {-1,-1}]
		//[{1, 0}, {0,  0}, {-1, 0}]
		//[{1, 1}, {0,  1}, {-1, 1}]

		float4 colorTopLeft = SampleColor(_MainTex, uv + du - dv);
		float4 colorTopCenter = SampleColor(_MainTex, uv - dv);
		float4 colorTopRight = SampleColor(_MainTex, uv - du - dv);
		float4 colorMiddleLeft = SampleColor(_MainTex, uv + du);
		float4 colorMiddleCenter = SampleColor(_MainTex, uv);
		float4 colorMiddleRight = SampleColor(_MainTex, uv - du);
		float4 colorBottomLeft = SampleColor(_MainTex, uv + du + dv);
		float4 colorBottomCenter = SampleColor(_MainTex,  uv + dv);
		float4 colorBottomRight = SampleColor(_MainTex, uv - du + dv);

		float4 colorMax = max(colorTopLeft, max(colorTopCenter, max(colorTopRight, max(colorMiddleLeft, max(colorMiddleCenter, max(colorMiddleRight, max(colorBottomLeft, max(colorBottomCenter, colorBottomRight))))))));
		float4 color = lerp(colorMax, colorMiddleCenter, colorMiddleCenter.a);

		return color;
	}

	float4 SSSColor( VertexOutput i ) : SV_Target
	{	
		float2 uv = i.uv;
		int2 pos = uv * _ScreenSizeAndScreenTexelSize.xy;

		float2 du = float2(_ScreenSizeAndScreenTexelSize.z, 0.0f);
		float2 dv = float2(0.0f, _ScreenSizeAndScreenTexelSize.w);

		half4 gbuffer0, gbuffer1, gbuffer2;
		SampleGBuffer(uv, gbuffer0, gbuffer1, gbuffer2);
		GBufferData data = StandardSSSSDataFromGbuffer(gbuffer0, gbuffer1, gbuffer2);

		bool pattern = GetPattern (pos);
		int shadingModel = DecodeShadingModel(data.shadingModel);
		half sssMask = shadingModel == SHADING_MODEL_SSS ? 1.0f : 0.0f;

		half2 sssColorLeft = SampleColor (_CameraGBufferTexture1, i.uv - du).gb;
		half2 sssColorRight = SampleColor (_CameraGBufferTexture1, i.uv + du).gb;
		half3 sssColor = half3(data.specColorAndCustomData.gb, 0.0f);
		half2 sssColorTop = SampleColor (_CameraGBufferTexture1, i.uv - dv).gb;
		half2 sssColorBottom = SampleColor (_CameraGBufferTexture1, i.uv + dv).gb;

		sssColor.b = filter(sssColor.rg, sssColorLeft, sssColorRight, sssColorTop, sssColorBottom);
		sssColor.rgb = pattern ? sssColor.rbg : sssColor.rgb;
		sssColor.rgb = YCbCrToRGB (sssColor);

		return float4(sssColor, sssMask);
	}

	float4 Combine( VertexOutput i ) : SV_Target
	{	
		SURFACE_SETUP(s)

		float2 uv = i.uv;

		float depth = SampleRawDepth(uv);
		float3 screenPos = float3(uv, depth);
		float3 worldPos = ScreenToWorldSpace(screenPos);

		float4 sceneColor = SampleColor(_MainTex, uv);
		float4 specular = SampleColor(_SpecularTex, uv);
		float4 diffuse = SampleColor(_DiffuseTex, uv);
		float4 diffuseSSS = SampleColor(_DiffuseSSSTex, uv);
	
		float len = length(_WorldSpaceCameraPos - worldPos);
		float fade = saturate((len - _FadeDistanceAndRadius.x) / _FadeDistanceAndRadius.y);
		float3 diffLerp = lerp(diffuseSSS, diffuse, fade); // SSS is visible for only a few meters so we remove it at a user define distance, it also helps to keep details in the distance when using half res resolve

		sceneColor.rgb = lerp(sceneColor.rgb, s.diffColor * diffLerp + specular, s.sssMask);

		return half4(sceneColor.rgb, 1.0f);
	}

	void DiffuseSpecularPass
	(	
		VertexOutput i,
		out half4 outDiffuse : SV_Target0, 
		out half4 outSpecular : SV_Target1
	) 
	{	
		outDiffuse = 0.0f;
		outSpecular = 0.0f;

		SURFACE_SETUP(s)

		float2 uv = i.uv;
		int2 pos = uv * _ScreenSizeAndScreenTexelSize.xy;

		float2 du = float2(_ScreenSizeAndScreenTexelSize.z, 0.0f);
		float2 dv = float2(0.0f, _ScreenSizeAndScreenTexelSize.w);

		float3 diffSpecLeft = SampleColor(_DiffuseSpecularTex, uv - du);
		float3 diffSpecRight = SampleColor(_DiffuseSpecularTex, uv + du);
		float4 diffSpecCenter = SampleColor(_DiffuseSpecularTex, uv);
		float3 diffSpecTop = SampleColor(_DiffuseSpecularTex, uv - dv);
		float3 diffSpecBottom = SampleColor(_DiffuseSpecularTex, uv + dv);

		float ab = ColorDiffBlend (diffSpecLeft, diffSpecRight);
		float cd = ColorDiffBlend (diffSpecTop, diffSpecBottom);

		float3 diffSpec = 0.5f * lerp(diffSpecLeft + diffSpecRight, diffSpecTop + diffSpecBottom, ab < cd);

		outDiffuse.rgb = lerp(diffSpec.rgb, diffSpecCenter.rgb, s.pattern);
		outDiffuse.a = s.sssMask;

		outSpecular.rgb = lerp(diffSpecCenter.rgb, diffSpec.rgb, s.pattern);
		outSpecular.a = s.sssMask;
	}

	float4 PrePass ( VertexOutput i ) : SV_Target
	{	
		SURFACE_SETUP(s)

		float2 uv = i.uv;

		float4 sceneColor = SampleColor(_MainTex, uv);
		sceneColor.rgb *= s.sssMask; // Make sure nothing else but skin is taken
		sceneColor.a = s.sssMask;

		return Max(sceneColor);
	}

	float4 Compute(VertexOutput i) : SV_Target
	{
		float2 uv = i.uv;

		float4 sceneColor = SampleColor(_MainTex, uv);

		return Max(sceneColor);
	}

	half4 Debug( VertexOutput i ) : SV_Target
	{	
		SURFACE_SETUP(s)

		float2 uv = i.uv;

		float depth = SampleRawDepth(uv);
		float3 screenPos = float3(uv, depth);
		float3 worldPos = ScreenToWorldSpace(screenPos);

		half4 sceneColor = SampleColor(_MainTex, uv); 
		half4 specular = SampleColor(_SpecularTex, uv);
		half4 diffuse = SampleColor(_DiffuseTex, uv);
		half4 diffuseSSS = SampleColor(_DiffuseSSSTex, uv);

		if(_DebugPass == 0)
			sceneColor.rgb = 0.0;
		else if(_DebugPass == 1)
			sceneColor.rgb = lerp(diffuse.rgb, diffuseSSS.rgb, s.sssMask);
		else if(_DebugPass == 2)
			sceneColor.rgb = lerp(0.0, specular, s.sssMask);
		else if(_DebugPass == 3)
			sceneColor.rgb = s.diffColor;
		else if(_DebugPass == 4)
			sceneColor.rgb = s.specColor;
		else if(_DebugPass == 5)
			sceneColor.rgb = lerp(0.0, s.sssColor, s.sssMask);
		else if(_DebugPass == 6)
		{
			if(s.shadingModel == SHADING_MODEL_STANDARD) // 0
				sceneColor.rgb = half4(1.0f, 0.0f, 0.0f, 1.0f);
			else if(s.shadingModel == SHADING_MODEL_SSS) // 1
				sceneColor.rgb = half4(0.0f, 1.0f, 0.0f, 1.0f);
			else
				sceneColor.rgb = half4(0.0f, 0.0f, 1.0f, 1.0f);
		}
		else if (_DebugPass == 7)
		{
			float len = length(_WorldSpaceCameraPos - worldPos);
			float fade = saturate((len - _FadeDistanceAndRadius.x) / _FadeDistanceAndRadius.y);

			sceneColor.rgb = 1.0f - fade;
		}

		return half4(sceneColor.rgb, 1.0);
	}

	ENDCG 
	
	Subshader 
	{
		//0
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment Combine
			
			ENDCG
		}
		//1
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment DiffuseSpecularPass
			
			ENDCG
		}
		//2
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment Debug
			
			ENDCG
		}
		//3
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment SSSBlurPass_X
			
			ENDCG
		}
		//4
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0

			#pragma vertex vert
			#pragma fragment SSSBlurPass_Y
			
			ENDCG
		}
		//5
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment Dilation
			
			ENDCG
		}
		//6
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment PrePass
			
			ENDCG
		}
		//7
		Pass 
		{
			ZTest Always Cull Off ZWrite Off
			Fog { Mode off }
			CGPROGRAM
			#pragma target 3.0 

			#pragma vertex vert
			#pragma fragment SSSColor
			
			ENDCG
		}
		//8
			Pass
		{
			ZTest Always Cull Off ZWrite Off
			Fog{ Mode off }
			CGPROGRAM
#pragma target 3.0 

#pragma vertex vert
#pragma fragment Compute

			ENDCG
		}
	}
	Fallback Off
}

```

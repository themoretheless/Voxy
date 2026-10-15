# Facial rendering source scan: luxuia/separable-sss-unity

Repository: https://github.com/luxuia/separable-sss-unity

Pinned source revision: 6c1152c2e5f8e45e3e214f2b498ed6fbbfdb4a97

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat.meta"], "subsurface": ["Assets/SeparableSubsurfaceScatter.meta", "Assets/SeparableSubsurfaceScatter/Demo.meta", "Assets/SeparableSubsurfaceScatter/Demo/Demo.unity", "Assets/SeparableSubsurfaceScatter/Demo/Demo.unity.meta", "Assets/SeparableSubsurfaceScatter/Demo/Material.meta", "Assets/SeparableSubsurfaceScatter/Demo/Material/Head2.mat", "Assets/SeparableSubsurfaceScatter/Demo/Material/Head2.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head1.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head1.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head2.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head2.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head3.obj", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head3.obj.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default (1).mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default (1).mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/defaultMat.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/defaultMat.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/lambert1.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/lambert1.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Mesh.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Mesh.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Shader.meta", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader.meta"], "shader": ["Assets/SeparableSubsurfaceScatter/Demo/Shader.meta", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader.meta", "Assets/SeparableSubsurfaceScatter/Script/ShaderIDs.cs", "Assets/SeparableSubsurfaceScatter/Script/ShaderIDs.cs.meta", "Assets/SeparableSubsurfaceScatter/Shader.meta", "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatter.shader", "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatter.shader.meta", "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatterCommon.cginc", "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatterCommon.cginc.meta"], "face": ["Assets/SeparableSubsurfaceScatter.meta", "Assets/SeparableSubsurfaceScatter/Demo.meta", "Assets/SeparableSubsurfaceScatter/Demo/Demo.unity", "Assets/SeparableSubsurfaceScatter/Demo/Demo.unity.meta", "Assets/SeparableSubsurfaceScatter/Demo/Material.meta", "Assets/SeparableSubsurfaceScatter/Demo/Material/Head2.mat", "Assets/SeparableSubsurfaceScatter/Demo/Material/Head2.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head1.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head1.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head2.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head2.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head3.obj", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Head3.obj.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default (1).mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default (1).mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Default.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/Emily_2_1_Skin_Blend_02.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/defaultMat.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/defaultMat.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/lambert1.mat", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Materials/lambert1.mat.meta", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Mesh.fbx", "Assets/SeparableSubsurfaceScatter/Demo/Mesh/Mesh.fbx.meta", "Assets/SeparableSubsurfaceScatter/Demo/Shader.meta", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader", "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader.meta"]}

## Source file: Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatter.shader

{
  "path": "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatter.shader",
  "blob_sha": "3e6575e8c532eb136124c0647600fa847400fdcf",
  "sha256": "66b5eb39335ea7ec8d1f687470d4b8b8d6dcdb54017b3a6ec335c746b56ce2be",
  "lines": 59,
  "bytes": 1141,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 2,
    "sss": 4,
    "shader": 2,
    "face": 2
  },
  "url": "https://github.com/luxuia/separable-sss-unity/blob/6c1152c2e5f8e45e3e214f2b498ed6fbbfdb4a97/Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatter.shader",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "float4 frag(VertexOutput i) : SV_TARGET {"
    },
    {
      "line": 50,
      "text": "float4 frag(VertexOutput i) : COLOR {"
    }
  ]
}

```text

﻿Shader "PostProcess/SeparableSubsurfaceScatter"
{
	Properties
	{
	}


	CGINCLUDE
#include "SeparableSubsurfaceScatterCommon.cginc"
#pragma target 3.0
	ENDCG

	SubShader
	{

		ZTest Always
		ZWrite Off
		Cull Off
		Stencil {
			Ref 5
			comp equal
			pass keep
		}

		Pass
		{
			Name "XBlur"
			CGPROGRAM
#pragma vertex vert
#pragma fragment frag
#pragma multi_compile_fwdbase

			float4 frag(VertexOutput i) : SV_TARGET {
				float4 SceneColor = tex2D(_MainTex, i.uv);
				float SSSIntencity = (_SSSScale * _CameraDepthTexture_TexelSize.x);
				float3 XBlur = SSS(SceneColor, i.uv, float2(SSSIntencity, 0)).rgb;
				return float4(XBlur, SceneColor.a);
			}
			ENDCG
		}

		Pass
		{
			Name "YBlur"
			CGPROGRAM
#pragma vertex vert
#pragma fragment frag
#pragma multi_compile_fwdbase

			float4 frag(VertexOutput i) : COLOR {
				float4 SceneColor = tex2D(_MainTex, i.uv);
				float SSSIntencity = (_SSSScale * _CameraDepthTexture_TexelSize.y);
				float3 YBlur = SSS(SceneColor, i.uv, float2(0, SSSIntencity)).rgb;
				return float4(YBlur, SceneColor.a);
			}
			ENDCG
		}
	}
}


```

## Source file: Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatterCommon.cginc

{
  "path": "Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatterCommon.cginc",
  "blob_sha": "c494e75080ee1b9b408804fb2fa47a6be9235cd9",
  "sha256": "006e1e44655e47faf6da5ec917d43bc20c37f38b995eda25f5ed7c102a805c93",
  "lines": 48,
  "bytes": 1556,
  "selection_score": 45,
  "keyword_line_counts": {
    "eye": 2,
    "sss": 9
  },
  "url": "https://github.com/luxuia/separable-sss-unity/blob/6c1152c2e5f8e45e3e214f2b498ed6fbbfdb4a97/Assets/SeparableSubsurfaceScatter/Shader/SeparableSubsurfaceScatterCommon.cginc",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "float4 SSS(float4 SceneColor, float2 UV, float2 SSSIntencity) {"
    }
  ]
}

```text

﻿#include "UnityCG.cginc"
#define DistanceToProjectionWindow 5.671281819617709             //1.0 / tan(0.5 * radians(20));
#define DPTimes300 1701.384545885313                             //DistanceToProjectionWindow * 300

#define SamplerSteps 25
uniform sampler2D _CameraDepthTexture;
float4 _CameraDepthTexture_TexelSize;

uniform sampler2D _MainTex;
uniform float4 _MainTex_ST;
uniform float _SSSScale;
uniform float4 _Kernel[SamplerSteps];

struct VertexInput {
	float4 vertex : POSITION;
	float2 uv :TEXCOORD0;
};

struct VertexOutput {
	float4 pos : SV_POSITION;
	float2 uv : TEXCOORD0;
};

VertexOutput vert(VertexInput v) {
	VertexOutput o;
	o.pos = v.vertex;
	o.uv = v.uv;
	return o;
}

float4 SSS(float4 SceneColor, float2 UV, float2 SSSIntencity) {
	float SceneDepth = LinearEyeDepth(SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, UV));
	float BlurLength = DistanceToProjectionWindow / SceneDepth;
	float2 UVOffset = SSSIntencity * BlurLength;
	float4 BlurSceneColor = SceneColor;
	BlurSceneColor.rgb *= _Kernel[0].rgb;

	[loop]
	for (int i = 1; i < SamplerSteps; i++) {
		float2 SSSUV = UV + _Kernel[i].a * UVOffset;
		float4 SSSSceneColor = tex2D(_MainTex, SSSUV);
		float SSSDepth = LinearEyeDepth(SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, SSSUV)).r;
		float SSSScale = saturate(DPTimes300 * SSSIntencity * abs(SceneDepth - SSSDepth));
		SSSSceneColor.rgb = lerp(SSSSceneColor.rgb, SceneColor.rgb, SSSScale);
		BlurSceneColor.rgb += _Kernel[i].rgb * SSSSceneColor.rgb;
	}
	return BlurSceneColor;
}

```

## Source file: Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader

{
  "path": "Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader",
  "blob_sha": "e4666dc0e412c547bbb115f24b35e885716c78f5",
  "sha256": "916e5d85718f1de56f3c4ee3055994e384815a55482d932c8ecd20ac9766a8d5",
  "lines": 112,
  "bytes": 2738,
  "selection_score": 31,
  "keyword_line_counts": {
    "shader": 4,
    "face": 1
  },
  "url": "https://github.com/luxuia/separable-sss-unity/blob/6c1152c2e5f8e45e3e214f2b498ed6fbbfdb4a97/Assets/SeparableSubsurfaceScatter/Demo/Shader/StencilSurface.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Custom/StencilSurface" {
	Properties
	{
		_Color("Color", Color) = (1,1,1,1)
		_MainTex("Albedo", 2D) = "white" {}

		_Glossiness("Smoothness", Range(0.0, 1.0)) = 0.5
		_GlossMapScale("Smoothness Scale", Range(0.0, 1.0)) = 1.0
		[Enum(Metallic Alpha,0,Albedo Alpha,1)] _SmoothnessTextureChannel("Smoothness texture channel", Float) = 0

		_BumpScale("Scale", Float) = 1.0
		_BumpMap("Normal Map", 2D) = "bump" {}

		_OcclusionStrength("Strength", Range(0.0, 1.0)) = 1.0
		_OcclusionMap("Occlusion", 2D) = "white" {}

	}

CGINCLUDE
		#define UNITY_SETUP_BRDF_INPUT MetallicSetup
ENDCG

	SubShader
	{
		Tags{ "RenderType" = "Opaque" "PerformanceChecks" = "False" }
		LOD 300

		// ------------------------------------------------------------------
		//  Base forward pass (directional light, emission, lightmaps, ...)
	Pass
	{
		Name "FORWARD"
		Tags{ "LightMode" = "ForwardBase" }

		Blend One Zero
		ZWrite On

		Stencil{
			Ref 5
			comp always
			pass replace
		}
		CGPROGRAM
#pragma target 3.0

		// -------------------------------------

#pragma shader_feature _NORMALMAP

#pragma multi_compile_fwdbase
#pragma multi_compile_fog
#pragma multi_compile_instancing
		// Uncomment the following line to enable dithering LOD crossfade. Note: there are more in the file to uncomment for other passes.
		//#pragma multi_compile _ LOD_FADE_CROSSFADE

#pragma vertex vertBase
#pragma fragment fragBase
#include "UnityStandardCoreForward.cginc"

		ENDCG
	}
		// ------------------------------------------------------------------
		//  Additive forward pass (one light per pass)
		Pass
	{
		Name "FORWARD_DELTA"
		Tags{ "LightMode" = "ForwardAdd" }
		Blend One One
		Fog{ Color(0,0,0,0) } // in additive pass fog should be black
		ZWrite Off
		ZTest LEqual

		CGPROGRAM
#pragma target 3.0

		// -------------------------------------


#pragma shader_feature _NORMALMAP

#pragma multi_compile_fwdadd_fullshadows
#pragma multi_compile_fog
		// Uncomment the following line to enable dithering LOD crossfade. Note: there are more in the file to uncomment for other passes.
		//#pragma multi_compile _ LOD_FADE_CROSSFADE

#pragma vertex vertAdd
#pragma fragment fragAdd
#include "UnityStandardCoreForward.cginc"

		ENDCG
	}
		// ------------------------------------------------------------------
		// Extracts information for lightmapping, GI (emission, albedo, ...)
		// This pass it not used during regular rendering.
		Pass
	{
		Name "META"
		Tags{ "LightMode" = "Meta" }

		Cull Off

		CGPROGRAM
#pragma vertex vert_meta
#pragma fragment frag_meta

#include "UnityStandardMeta.cginc"
		ENDCG
	}
	}

	FallBack "Standard"
}


```

## Source file: Assets/SeparableSubsurfaceScatter/Script/SubsurfaceScatterPostProcess.cs

{
  "path": "Assets/SeparableSubsurfaceScatter/Script/SubsurfaceScatterPostProcess.cs",
  "blob_sha": "0352c3178146b2ac6b0e26d21fe78b29fd8967b1",
  "sha256": "f68bcb274339c176dc4df16037da9d037b06a3d82683205890f6d5ba9e026e31",
  "lines": 63,
  "bytes": 2432,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 6,
    "sss": 6,
    "shader": 4,
    "face": 6
  },
  "url": "https://github.com/luxuia/separable-sss-unity/blob/6c1152c2e5f8e45e3e214f2b498ed6fbbfdb4a97/Assets/SeparableSubsurfaceScatter/Script/SubsurfaceScatterPostProcess.cs",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "void OnEnable() {"
    },
    {
      "line": 34,
      "text": "void  OnDisable() {"
    },
    {
      "line": 38,
      "text": "void OnPreRender() {"
    },
    {
      "line": 44,
      "text": "void UpdateSubsurface() {"
    },
    {
      "line": 57,
      "text": "void ClearBuffer() {"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

[ExecuteInEditMode]
[RequireComponent(typeof(Camera))]
public class SubsurfaceScatterPostProcess : MonoBehaviour {
    [Range(0, 3)]
    public float Scaler = 0.1f;
    public Color MainColor;
    public Color Falloff;

    Camera renderCamera;
    CommandBuffer buffer;
    Material material;
    List<Vector4> KernelArray = new List<Vector4>();

    static int SceneColorID = Shader.PropertyToID("_SceneColor");
    static int Kernel = Shader.PropertyToID("_Kernel");
    static int SSSScaler = Shader.PropertyToID("_SSSScale");

	void OnEnable() {
        renderCamera = GetComponent<Camera>();
        renderCamera.depthTextureMode |= DepthTextureMode.Depth;
        material = new Material(Shader.Find("PostProcess/SeparableSubsurfaceScatter"));

        buffer = new CommandBuffer();
        buffer.name = "Separable Subsurface Scatter";
        renderCamera.clearStencilAfterLightingPass = true;
        renderCamera.AddCommandBuffer(CameraEvent.AfterForwardOpaque, buffer);
    }

    void  OnDisable() {
        ClearBuffer();
    }

    void OnPreRender() {
        UpdateSubsurface();
    }

    //流程参考 https://github.com/iryoku/separable-sss 
    //和 https://github.com/haolange/UnityCharacterRender_SeparableSubsurfaceScatter
    void UpdateSubsurface() {
        Vector3 SSSC = Vector3.Normalize(new Vector3(MainColor.r, MainColor.g, MainColor.b));
        Vector3 SSSFC = Vector3.Normalize(new Vector3(Falloff.r, Falloff.g, Falloff.b));
        KernelCalculate.CalculateKernel(KernelArray, 25, SSSC, SSSFC);
        material.SetVectorArray(Kernel, KernelArray);
        material.SetFloat(SSSScaler, Scaler);

        buffer.Clear();
        buffer.GetTemporaryRT(SceneColorID, renderCamera.pixelWidth, renderCamera.pixelHeight, 0, FilterMode.Trilinear, RenderTextureFormat.DefaultHDR);
        buffer.BlitStencil(BuiltinRenderTextureType.CameraTarget, SceneColorID, BuiltinRenderTextureType.CameraTarget, material, 0);
        buffer.BlitSRT(SceneColorID, BuiltinRenderTextureType.CameraTarget, material, 1);
    }

    void ClearBuffer() {
        buffer.ReleaseTemporaryRT(SceneColorID);
        renderCamera.RemoveCommandBuffer(CameraEvent.AfterForwardOpaque, buffer);
        buffer.Release();
        buffer.Dispose();
    }
}


```

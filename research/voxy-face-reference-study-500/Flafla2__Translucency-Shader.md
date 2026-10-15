# Facial rendering source scan: Flafla2/Translucency-Shader

Repository: https://github.com/Flafla2/Translucency-Shader

Pinned source revision: 32da1723e9538ba800f59a9f938353ae4a75113b

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["Assets/Editor/StandardTranslucentShaderGUI.cs", "Assets/Editor/StandardTranslucentShaderGUI.cs.meta", "Assets/Shaders.meta", "Assets/Shaders/Custom-DeferredShading.shader", "Assets/Shaders/Custom-DeferredShading.shader.meta", "Assets/Shaders/DICETranslucency.cginc", "Assets/Shaders/DICETranslucency.cginc.meta", "Assets/Shaders/Standard-Translucent.shader", "Assets/Shaders/Standard-Translucent.shader.meta", "Assets/Shaders/TranslucentScaledLight.shader", "Assets/Shaders/TranslucentScaledLight.shader.meta", "Assets/Shaders/UnityStandardTranslucentCore.cginc", "Assets/Shaders/UnityStandardTranslucentCore.cginc.meta", "Assets/Shaders/UnityStandardTranslucentInput.cginc", "Assets/Shaders/UnityStandardTranslucentInput.cginc.meta"]}

## Source file: Assets/Shaders/DICETranslucency.cginc

{
  "path": "Assets/Shaders/DICETranslucency.cginc",
  "blob_sha": "7cf19615077f92538be4559b4811fa9be5fcb8dd",
  "sha256": "9c35723fcc6446687f5736c76fe8261b9dd3d411e935bb9c7ed1a6f48fff2349",
  "lines": 75,
  "bytes": 2534,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "shader": 1
  },
  "url": "https://github.com/Flafla2/Translucency-Shader/blob/32da1723e9538ba800f59a9f938353ae4a75113b/Assets/Shaders/DICETranslucency.cginc",
  "function_signature_candidates": []
}

```text

﻿#ifndef DICE_TRANSLUCENCY_INCLUDED
#define DICE_TRANSLUCENCY_INCLUDED

#include "UnityCG.cginc"
#include "UnityDeferredLibrary.cginc"
#include "UnityPBSLighting.cginc"
#include "UnityStandardUtils.cginc"
#include "UnityStandardBRDF.cginc"

// Used to have compatibility with default (Unity) lights, and a custom deferred shader.
#ifndef _LIGHTSCALE
#define _LIGHTSCALE 1
#endif

#ifndef _TRANSLIGHTCOLOR
#define _TRANSLIGHTCOLOR _LightColor
#endif

#ifndef LIGHT_PARAMS_FUNC
#define LIGHT_PARAMS_FUNC UnityDeferredCalculateLightParams
#endif

sampler2D _CameraGBufferTexture0;
sampler2D _CameraGBufferTexture1;
sampler2D _CameraGBufferTexture2;
		
half4 CalculateLight (unity_v2f_deferred i)
{
	float3 wpos;
	float2 uv;
	float atten, fadeDist;
	UnityLight light;
	UNITY_INITIALIZE_OUTPUT(UnityLight, light);
	LIGHT_PARAMS_FUNC (i, wpos, uv, light.dir, atten, fadeDist);

	half4 gbuffer0 = tex2D (_CameraGBufferTexture0, uv);
	half4 gbuffer1 = tex2D (_CameraGBufferTexture1, uv);
	half4 gbuffer2 = tex2D (_CameraGBufferTexture2, uv);

	light.color = _TRANSLIGHTCOLOR.rgb * atten;
	half3 baseColor = gbuffer0.rgb;

	half translucent = 1-gbuffer2.a;

	// If we have translucency enabled, there is only 1 specular term (ie specularity is grayscale).
	// So essentially if the translucent flag is enabled specColor = gbuffer1.rrr
	half3 specColor = half3(gbuffer1.r,lerp(gbuffer1.g,gbuffer1.r,translucent),lerp(gbuffer1.b,gbuffer1.r,translucent));
	half oneMinusRoughness = gbuffer1.a;
	half3 normalWorld = gbuffer2.rgb * 2 - 1;
	normalWorld = normalize(normalWorld);
	float3 eyeVec = normalize(wpos-_WorldSpaceCameraPos);
	half oneMinusReflectivity = 1 - SpecularStrength(specColor.rgb);
	light.ndotl = LambertTerm (normalWorld, light.dir);

	UnityIndirect ind;
	UNITY_INITIALIZE_OUTPUT(UnityIndirect, ind);
	ind.diffuse = 0;
	ind.specular = 0;

    half4 res = UNITY_BRDF_PBS (baseColor, specColor, oneMinusReflectivity, oneMinusRoughness, normalWorld, -eyeVec, light, ind);

	// DICE Translucency Calculation
	// http://www.gdcvault.com/play/1014538/Approximating-Translucency-for-a-Fast

	half3 vLTLight = -light.dir + normalWorld * gbuffer1.g; // gbuffer1.g = distortion
	half fLTDot = pow(saturate(dot(eyeVec, -vLTLight)), gbuffer1.b*10) * _LIGHTSCALE; // gbuffer1.b = power
	half3 fLT = atten * fLTDot * gbuffer0.a; // gbuffer0.a = thickness, ignoring ambient term
	res += float4(baseColor * _TRANSLIGHTCOLOR.rgb * fLT,0) * translucent;

	// End DICE Translucency Calculation

	return res;
}

#endif // DICE_TRANSLUCENCY_INCLUDED

```

## Source file: Assets/Shaders/Standard-Translucent.shader

{
  "path": "Assets/Shaders/Standard-Translucent.shader",
  "blob_sha": "888d2b8e3669bbc1001069a17538ab92d536e6b2",
  "sha256": "1166e94e40423dc532e165ef7350d8e9cc0500a5f9cc97204fbb21751b4be85a",
  "lines": 328,
  "bytes": 8828,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 40
  },
  "url": "https://github.com/Flafla2/Translucency-Shader/blob/32da1723e9538ba800f59a9f938353ae4a75113b/Assets/Shaders/Standard-Translucent.shader",
  "function_signature_candidates": []
}

```text

Shader "Standard-Translucent"
{
	Properties
	{
		_Color("Color", Color) = (1,1,1,1)
		_MainTex("Albedo", 2D) = "white" {}
		
		_Cutoff("Alpha Cutoff", Range(0.0, 1.0)) = 0.5

		_Glossiness("Smoothness", Range(0.0, 1.0)) = 0.5
		[Gamma] _Metallic("Metallic", Range(0.0, 1.0)) = 0.0
		_MetallicGlossMap("Metallic", 2D) = "white" {}

		_BumpScale("Scale", Float) = 1.0
		_BumpMap("Normal Map", 2D) = "bump" {}

		_Thickness ("Thickness Map", 2D) = "black" {}
		_TranslucentDistortion ("Translucent Distortion",Range(0,1)) = 0.1
		_TranslucentPower ("Translucent Power",Range(1,10)) = 4

		_Parallax ("Height Scale", Range (0.005, 0.08)) = 0.02
		_ParallaxMap ("Height Map", 2D) = "black" {}

		_OcclusionStrength("Strength", Range(0.0, 1.0)) = 1.0
		_OcclusionMap("Occlusion", 2D) = "white" {}

		_EmissionColor("Color", Color) = (0,0,0)
		_EmissionMap("Emission", 2D) = "white" {}
		
		_DetailMask("Detail Mask", 2D) = "white" {}

		_DetailAlbedoMap("Detail Albedo x2", 2D) = "grey" {}
		_DetailNormalMapScale("Scale", Float) = 1.0
		_DetailNormalMap("Normal Map", 2D) = "bump" {}

		[Enum(UV0,0,UV1,1)] _UVSec ("UV Set for secondary textures", Float) = 0

		// UI-only data
		[HideInInspector] _EmissionScaleUI("Scale", Float) = 0.0
		[HideInInspector] _EmissionColorUI("Color", Color) = (1,1,1)

		// Blending state
		[HideInInspector] _Mode ("__mode", Float) = 0.0
		[HideInInspector] _SrcBlend ("__src", Float) = 1.0
		[HideInInspector] _DstBlend ("__dst", Float) = 0.0
		[HideInInspector] _ZWrite ("__zw", Float) = 1.0
	}

	CGINCLUDE
		#define UNITY_SETUP_BRDF_INPUT MetallicSetup
	ENDCG

	SubShader
	{
		Tags { "RenderType"="Opaque" "PerformanceChecks"="False" }
		LOD 300
	

		// ------------------------------------------------------------------
		//  Base forward pass (directional light, emission, lightmaps, ...)
		Pass
		{
			Name "FORWARD" 
			Tags { "LightMode" = "ForwardBase" }

			Blend [_SrcBlend] [_DstBlend]
			ZWrite [_ZWrite]

			CGPROGRAM
			#pragma target 3.0
			// TEMPORARY: GLES2.0 temporarily disabled to prevent errors spam on devices without textureCubeLodEXT
			#pragma exclude_renderers gles
			
			// -------------------------------------
					
			#pragma shader_feature _NORMALMAP
			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma shader_feature _EMISSION
			#pragma shader_feature _METALLICGLOSSMAP 
			#pragma shader_feature ___ _DETAIL_MULX2
			#pragma shader_feature _PARALLAXMAP
			
			#pragma multi_compile_fwdbase
			#pragma multi_compile_fog
				
			#pragma vertex vertForwardBase
			#pragma fragment fragForwardBase

			#include "UnityStandardTranslucentCore.cginc"

			ENDCG
		}
		// ------------------------------------------------------------------
		//  Additive forward pass (one light per pass)
		Pass
		{
			Name "FORWARD_DELTA"
			Tags { "LightMode" = "ForwardAdd" }
			Blend [_SrcBlend] One
			Fog { Color (0,0,0,0) } // in additive pass fog should be black
			ZWrite Off
			ZTest LEqual

			CGPROGRAM
			#pragma target 3.0
			// GLES2.0 temporarily disabled to prevent errors spam on devices without textureCubeLodEXT
			#pragma exclude_renderers gles

			// -------------------------------------

			
			#pragma shader_feature _NORMALMAP
			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma shader_feature _METALLICGLOSSMAP
			#pragma shader_feature ___ _DETAIL_MULX2
			#pragma shader_feature _PARALLAXMAP
			
			#pragma multi_compile_fwdadd_fullshadows
			#pragma multi_compile_fog
			
			#pragma vertex vertForwardAdd
			#pragma fragment fragForwardAdd

			#include "UnityStandardTranslucentCore.cginc"

			ENDCG
		}
		// ------------------------------------------------------------------
		//  Shadow rendering pass
		Pass {
			Name "ShadowCaster"
			Tags { "LightMode" = "ShadowCaster" }
			
			ZWrite On ZTest LEqual

			CGPROGRAM
			#pragma target 3.0
			// TEMPORARY: GLES2.0 temporarily disabled to prevent errors spam on devices without textureCubeLodEXT
			#pragma exclude_renderers gles
			
			// -------------------------------------


			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma multi_compile_shadowcaster

			#pragma vertex vertShadowCaster
			#pragma fragment fragShadowCaster

			#include "UnityStandardShadow.cginc"

			ENDCG
		}
		// ------------------------------------------------------------------
		//  Deferred pass
		Pass
		{
			Name "DEFERRED"
			Tags { "LightMode" = "Deferred" }

			CGPROGRAM
			#pragma target 3.0
			// TEMPORARY: GLES2.0 temporarily disabled to prevent errors spam on devices without textureCubeLodEXT
			#pragma exclude_renderers nomrt gles
			

			// -------------------------------------

			#pragma shader_feature _NORMALMAP
			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma shader_feature _EMISSION
			#pragma shader_feature _METALLICGLOSSMAP
			#pragma shader_feature ___ _DETAIL_MULX2
			#pragma shader_feature _PARALLAXMAP

			#pragma multi_compile ___ UNITY_HDR_ON
			#pragma multi_compile LIGHTMAP_OFF LIGHTMAP_ON
			#pragma multi_compile DIRLIGHTMAP_OFF DIRLIGHTMAP_COMBINED DIRLIGHTMAP_SEPARATE
			#pragma multi_compile DYNAMICLIGHTMAP_OFF DYNAMICLIGHTMAP_ON
			
			#pragma vertex vertDeferred
			#pragma fragment fragDeferred

			#include "UnityStandardTranslucentCore.cginc"

			ENDCG
		}

		// ------------------------------------------------------------------
		// Extracts information for lightmapping, GI (emission, albedo, ...)
		// This pass it not used during regular rendering.
		Pass
		{
			Name "META" 
			Tags { "LightMode"="Meta" }

			Cull Off

			CGPROGRAM
			#pragma vertex vert_meta
			#pragma fragment frag_meta

			#pragma shader_feature _EMISSION
			#pragma shader_feature _METALLICGLOSSMAP
			#pragma shader_feature ___ _DETAIL_MULX2

			#include "UnityStandardMeta.cginc"
			ENDCG
		}
	}

	SubShader
	{
		Tags { "RenderType"="Opaque" "PerformanceChecks"="False" }
		LOD 150

		// ------------------------------------------------------------------
		//  Base forward pass (directional light, emission, lightmaps, ...)
		Pass
		{
			Name "FORWARD" 
			Tags { "LightMode" = "ForwardBase" }

			Blend [_SrcBlend] [_DstBlend]
			ZWrite [_ZWrite]

			CGPROGRAM
			#pragma target 2.0
			
			#pragma shader_feature _NORMALMAP
			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma shader_feature _EMISSION 
			#pragma shader_feature _METALLICGLOSSMAP 
			#pragma shader_feature ___ _DETAIL_MULX2
			// SM2.0: NOT SUPPORTED shader_feature _PARALLAXMAP

			#pragma skip_variants SHADOWS_SOFT DIRLIGHTMAP_COMBINED DIRLIGHTMAP_SEPARATE

			#pragma multi_compile_fwdbase
			#pragma multi_compile_fog
	
			#pragma vertex vertForwardBase
			#pragma fragment fragForwardBase

			#include "UnityStandardTranslucentCore.cginc"

			ENDCG
		}
		// ------------------------------------------------------------------
		//  Additive forward pass (one light per pass)
		Pass
		{
			Name "FORWARD_DELTA"
			Tags { "LightMode" = "ForwardAdd" }
			Blend [_SrcBlend] One
			Fog { Color (0,0,0,0) } // in additive pass fog should be black
			ZWrite Off
			ZTest LEqual
			
			CGPROGRAM
			#pragma target 2.0

			#pragma shader_feature _NORMALMAP
			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma shader_feature _METALLICGLOSSMAP
			#pragma shader_feature ___ _DETAIL_MULX2
			// SM2.0: NOT SUPPORTED shader_feature _PARALLAXMAP
			#pragma skip_variants SHADOWS_SOFT
			
			#pragma multi_compile_fwdadd_fullshadows
			#pragma multi_compile_fog
			
			#pragma vertex vertForwardAdd
			#pragma fragment fragForwardAdd

			#include "UnityStandardTranslucentCore.cginc"

			ENDCG
		}
		// ------------------------------------------------------------------
		//  Shadow rendering pass
		Pass {
			Name "ShadowCaster"
			Tags { "LightMode" = "ShadowCaster" }
			
			ZWrite On ZTest LEqual

			CGPROGRAM
			#pragma target 2.0

			#pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
			#pragma skip_variants SHADOWS_SOFT
			#pragma multi_compile_shadowcaster

			#pragma vertex vertShadowCaster
			#pragma fragment fragShadowCaster

			#include "UnityStandardShadow.cginc"

			ENDCG
		}

		// ------------------------------------------------------------------
		// Extracts information for lightmapping, GI (emission, albedo, ...)
		// This pass it not used during regular rendering.
		Pass
		{
			Name "META" 
			Tags { "LightMode"="Meta" }

			Cull Off

			CGPROGRAM
			#pragma vertex vert_meta
			#pragma fragment frag_meta

			#pragma shader_feature _EMISSION
			#pragma shader_feature _METALLICGLOSSMAP
			#pragma shader_feature ___ _DETAIL_MULX2

			#include "UnityStandardMeta.cginc"
			ENDCG
		}
	}

	FallBack "VertexLit"
	CustomEditor "StandardTranslucentShaderGUI"
}


```

## Source file: Assets/Shaders/Custom-DeferredShading.shader

{
  "path": "Assets/Shaders/Custom-DeferredShading.shader",
  "blob_sha": "1ff482970253da3ee4047e81345595aa5715d0a4",
  "sha256": "afb74fef9d25c43787485dac8a68a2c2a2c2653eb77fc86918e82f6120df832d",
  "lines": 89,
  "bytes": 1710,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "face": 1
  },
  "url": "https://github.com/Flafla2/Translucency-Shader/blob/32da1723e9538ba800f59a9f938353ae4a75113b/Assets/Shaders/Custom-DeferredShading.shader",
  "function_signature_candidates": []
}

```text

Shader "Hidden/Custom-DeferredShading" {
Properties {
	_LightTexture0 ("", any) = "" {}
	_LightTextureB0 ("", 2D) = "" {}
	_ShadowMapTexture ("", any) = "" {}
	_SrcBlend ("", Float) = 1
	_DstBlend ("", Float) = 1
}
SubShader {

// Pass 1: Lighting pass
//  LDR case - Lighting encoded into a subtractive ARGB8 buffer
//  HDR case - Lighting additively blended into floating point buffer
Pass {
	ZWrite Off
	Blend [_SrcBlend] [_DstBlend]

CGPROGRAM
#pragma target 3.0
#pragma vertex vert_deferred
#pragma fragment frag
#pragma multi_compile_lightpass
#pragma multi_compile ___ UNITY_HDR_ON

#pragma exclude_renderers nomrt

#include "DICETranslucency.cginc"

#ifdef UNITY_HDR_ON
half4
#else
fixed4
#endif
frag (unity_v2f_deferred i) : SV_Target
{
	half4 c = CalculateLight(i);
	#ifdef UNITY_HDR_ON
	return c;
	#else
	return exp2(-c);
	#endif
}

ENDCG
}


// Pass 2: Final decode pass.
// Used only with HDR off, to decode the logarithmic buffer into the main RT
Pass {
	ZTest Always Cull Off ZWrite Off
	Stencil {
		ref [_StencilNonBackground]
		readmask [_StencilNonBackground]
		// Normally just comp would be sufficient, but there's a bug and only front face stencil state is set (case 583207)
		compback equal
		compfront equal
	}

CGPROGRAM
#pragma target 3.0
#pragma vertex vert
#pragma fragment frag
#pragma exclude_renderers nomrt

sampler2D _LightBuffer;
struct v2f {
	float4 vertex : SV_POSITION;
	float2 texcoord : TEXCOORD0;
};

v2f vert (float4 vertex : POSITION, float2 texcoord : TEXCOORD0)
{
	v2f o;
	o.vertex = mul(UNITY_MATRIX_MVP, vertex);
	o.texcoord = texcoord.xy;
	return o;
}

fixed4 frag (v2f i) : SV_Target
{
	return -log2(tex2D(_LightBuffer, i.texcoord));
}
ENDCG 
}

}
Fallback Off
}


```

## Source file: Assets/Shaders/TranslucentScaledLight.shader

{
  "path": "Assets/Shaders/TranslucentScaledLight.shader",
  "blob_sha": "19a0c307cc31a854293149792ae58e19c631a4f9",
  "sha256": "4d095f3d577dcf23309bfc30998ebcc7cb6a39dc6bcc32671249b881106f98a6",
  "lines": 105,
  "bytes": 2910,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 4,
    "face": 1
  },
  "url": "https://github.com/Flafla2/Translucency-Shader/blob/32da1723e9538ba800f59a9f938353ae4a75113b/Assets/Shaders/TranslucentScaledLight.shader",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "void TranslucentCalculateLightParams ("
    }
  ]
}

```text

﻿Shader "CustomLights/PointTranslucent" {
	SubShader {
		Tags { "Queue"="Transparent-1" }
CGINCLUDE

float4 _LightParams;
half4 _CustomLightColor;

#define _LIGHTSCALE _LightParams.x
#define _LIGHTSIZE _LightParams.y
#define _LIGHTINVSQUARERADIUS _LightParams.z
#define _TRANSLIGHTCOLOR _CustomLightColor
#define LIGHT_PARAMS_FUNC TranslucentCalculateLightParams
#define POINT // for UnityDeferredLibrary.cginc

#include "UnityCG.cginc"
#include "UnityDeferredLibrary.cginc"

// Calculates some basic light parameters used later to calculate lighting
void TranslucentCalculateLightParams (
	unity_v2f_deferred i,
	out float3 outWorldPos,
	out float2 outUV,
	out half3 outLightDir,
	out float outAtten,
	out float outFadeDist)
{
	// Ray from the camera to the given pixel
	i.ray = i.ray * (_ProjectionParams.z / i.ray.z);
	// Screen-space UV of this pixel
	float2 uv = i.uv.xy / i.uv.w;
	
	// read depth and reconstruct world position
	float depth = SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, uv);
	depth = Linear01Depth (depth);
	// Camera-space position of this pixel
	float4 vpos = float4(i.ray * depth,1);
	// World-space position of this pixel
	float3 wpos = mul (_CameraToWorld, vpos).xyz;
	
	// Position of the light in world space
	float3 lightPos = float3(_Object2World[0][3], _Object2World[1][3], _Object2World[2][3]);

	// Point light
	float3 tolight = wpos - lightPos;
	half3 lightDir = -normalize (tolight);
	
	// att is the distance from this pixel to the light
	// ||tolight||^2 / radius^2, because a self dot product = square of magnitude
	float att = dot(tolight, tolight) * _LIGHTINVSQUARERADIUS;
	// Light attenuation data is stored in a texture from unity.  att.rr is a Vector2 of distance
	float atten = tex2D (_LightTextureB0, att.rr).UNITY_ATTEN_CHANNEL;

	outWorldPos = wpos;
	outUV = uv;
	outLightDir = lightDir;
	outAtten = atten;
	outFadeDist = 0;
}

#include "DICETranslucency.cginc"

ENDCG
		
		Pass {			
			Fog { Mode Off }
			ZTest Always Cull Front ZWrite Off
			Blend One One
			// For some dumbshit reason we need to use the stencil buffer to fix edit mode.
			// If we don't do this, then edit mode has a giant black sphere around the light.
			Stencil {
				ref [_StencilNonBackground]
				readmask [_StencilNonBackground]
				// Normally just comp would be sufficient, but there's a bug and only front face stencil state is set (case 583207)
				compback equal
				compfront equal
			}


CGPROGRAM
#pragma target 3.0
#pragma vertex vert
#pragma fragment frag
#pragma exclude_renderers nomrt

unity_v2f_deferred vert (float4 vertex : POSITION)
{
	unity_v2f_deferred o;
	o.pos = mul(UNITY_MATRIX_MVP, vertex);
	o.uv = ComputeScreenPos (o.pos);
	o.ray = mul (UNITY_MATRIX_MV, vertex).xyz * float3(-1,-1,1);
	return o;
}

half4 frag (unity_v2f_deferred i) : SV_Target
{
	half4 c = CalculateLight(i);
	return c;
}
			
ENDCG
		} // Pass
	} // Subshader
	FallBack Off
} // Shader


```

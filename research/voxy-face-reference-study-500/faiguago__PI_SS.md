# Facial rendering source scan: faiguago/PI_SS

Repository: https://github.com/faiguago/PI_SS

Pinned source revision: 649ce1fefc1d479d45f755fc4c06d543782f381a

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/_Shaders/SS_Skin.cginc", "Assets/_Shaders/SS_Skin.cginc.meta", "Assets/_Shaders/Skin.shader", "Assets/_Shaders/Skin.shader.meta"], "shader": ["Assets/_Shaders.meta", "Assets/_Shaders/SS_Common.cginc", "Assets/_Shaders/SS_Common.cginc.meta", "Assets/_Shaders/SS_Lighting.cginc", "Assets/_Shaders/SS_Lighting.cginc.meta", "Assets/_Shaders/SS_Skin.cginc", "Assets/_Shaders/SS_Skin.cginc.meta", "Assets/_Shaders/SS_Utils.cginc", "Assets/_Shaders/SS_Utils.cginc.meta", "Assets/_Shaders/Skin.shader", "Assets/_Shaders/Skin.shader.meta", "Assets/_Shaders/ToneMapping.cginc", "Assets/_Shaders/ToneMapping.cginc.meta"]}

## Source file: Assets/_Shaders/Skin.shader

{
  "path": "Assets/_Shaders/Skin.shader",
  "blob_sha": "04f292f319c93acb0aaa4ca8324e764201800da2",
  "sha256": "36022a41e4a6f84ee832439c017a172a4ffcfcf96fa878cab4673228223f7939",
  "lines": 151,
  "bytes": 3244,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 3,
    "shader": 2,
    "lip": 2
  },
  "url": "https://github.com/faiguago/PI_SS/blob/649ce1fefc1d479d45f755fc4c06d543782f381a/Assets/_Shaders/Skin.shader",
  "function_signature_candidates": [
    {
      "line": 120,
      "text": "float4 vert(appdata v) : SV_POSITION"
    },
    {
      "line": 128,
      "text": "float4 frag() : SV_Target"
    },
    {
      "line": 135,
      "text": "float4 vert(appdata v) : SV_POSITION"
    },
    {
      "line": 141,
      "text": "float4 frag() : SV_Target"
    }
  ]
}

```text

﻿Shader "FI/Skin"
{
    Properties
	{
		// Tone Mapping
		_Exposure("Exposure", Range(0, 2)) = 1
		[Toggle] _Tonemap("Use Tone Mapping", Float) = 1

		// Albedo
		_Color("Color", Color) = (1, 1, 1, 1)
		[NoScaleOffset] _MainTex("Albedo (RGB)", 2D) = "white" {}

		// Specular Reflectance
		_Glossiness("Smoothness", Range(0, 1)) = 0.5
		_SpecularIntensity("Specular Intensity", Range(0, 2)) = 1
		[NoScaleOffset] _SpecularTex("Specular (R)", 2D) = "white" {}
		[NoScaleOffset] _SpecularMaskTex("Specular Mask (R)", 2D) = "white" {}

		// Curvature (R), Ambient Occlussion (G), and Thickness (B)
		_DatasTex("Cu (R), AO (G), Th (B)", 2D) = "white" {}

		// Fast Translucency
		[HDR] _TrnColor("Translucency Color", Color) = (1, 0, 0, 0)
		_TrnDistortion("Distortion", Range(0, 1)) = 0.5
		_TrnPower("Power", Range(1, 12)) = 12
			
		// Bump
		_BumpScale("BumpScale", Range(0, 2)) = 1
		_NormalBlurredBias("NormalBlurredBias", Range(0, 10)) = 3
		[NoScaleOffset] _NormalTex("NormalTexture", 2D) = "bump" {}
			
		// Curvature LUT
		[NoScaleOffset] _CurvatureTex_LUT("Curvature LUT", 2D) = "black" {}
		_CurvatureBias("CurvatureBias", Vector) = (1, 0, 0, 0)

		// Shadow LUT
		[NoScaleOffset] _ShadowTex_LUT("Shadow LUT", 2D) = "black" {}
		_ShadowBias("ShadowBias", Vector) = (1, 0, 0, 0)

		// Diffuse IBL
		[NoScaleOffset] _DiffuseEnvironment("Diffuse Environment (IBL)", CUBE) = "" {}
		_DiffuseEnvironmetAmount("Diffuse Environment Amount", Range(0, 1)) = 1

		// Specular IBL
		[NoScaleOffset] _SpecularEnvironment("Specular Environment (IBL)", CUBE) = "" {}
		_SpecularEnvironmetAmount("Specular Environment Amount", Range(0, 1)) = 1
    }
    SubShader
    {
        Tags { "RenderType"="Opaque" }
        LOD 100

        Pass
        {
			Tags { "LightMode" = "ForwardBase" }

            CGPROGRAM

            #pragma vertex vert
            #pragma fragment frag

			#pragma multi_compile_fwdbase
			#pragma multi_compile _ _TONEMAP_ON

			#define FORWARD_BASE_PASS

			#include "SS_Skin.cginc"
				
            ENDCG
        }

		Pass
		{
			Tags { "LightMode" = "ForwardAdd" }

			Blend One One
			ZWrite Off

			CGPROGRAM

			#pragma vertex vert
			#pragma fragment frag

			#pragma multi_compile_fwdadd
			#pragma multi_compile _ _TONEMAP_ON

			#include "SS_Skin.cginc"

			ENDCG
		}

		Pass
		{
			Tags { "LightMode"="ShadowCaster" }

			CGPROGRAM

			#pragma target 3.0

			#pragma multi_compile_shadowcaster
	
			#pragma vertex vert
			#pragma fragment frag

			#include "UnityCG.cginc"

			struct appdata 
			{
				float4 vertex : POSITION;
				float3 normal : NORMAL;
			};

			#if defined(SHADOWS_CUBE)

			struct v2f {
				float4 pos : SV_POSITION;
				float3 lightVec : TEXCOORD0;
			};

			float4 vert(appdata v) : SV_POSITION
			{
				v2f o;
				o.pos = UnityObjectToClipPos(v.vertex);
				o.lightVec = mul(unity_ObjectToWorld, v.vertex).xyz - _LightPositionRange.xyz;
				return o;
			}

			float4 frag() : SV_Target
			{
				return 0;
			}

			#else

			float4 vert(appdata v) : SV_POSITION 
			{
				float4 position = UnityClipSpaceShadowCasterPos(v.vertex.xyz, v.normal);
				return UnityApplyLinearShadowBias(position);
			}

			float4 frag() : SV_Target
			{
				return 0;
			}

			#endif

			ENDCG
		}
    }
}


```

## Source file: Assets/_Shaders/SS_Skin.cginc

{
  "path": "Assets/_Shaders/SS_Skin.cginc",
  "blob_sha": "baf2bfa0a49ffd1d57e193bca661a865c4581251",
  "sha256": "e5ab8f0e2dd8b142dc907e23ce635c93b40f255008c76ddee103848ba0e2320e",
  "lines": 64,
  "bytes": 2146,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 2,
    "lip": 1
  },
  "url": "https://github.com/faiguago/PI_SS/blob/649ce1fefc1d479d45f755fc4c06d543782f381a/Assets/_Shaders/SS_Skin.cginc",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "float4 frag(v2f i) : SV_Target"
    }
  ]
}

```text

#ifndef ___SS_SKIN___
#define ___SS_SKIN___

#include "SS_Utils.cginc"
#include "SS_Common.cginc"
#include "SS_Lighting.cginc"

v2f vert(appdata v)
{
    v2f o;

    o.uv = v.uv;
    o.pos = UnityObjectToClipPos(v.vertex);
    o.worldPos = mul(unity_ObjectToWorld, v.vertex).xyz;
    o.normal = UnityObjectToWorldNormal(v.normal);
    o.tangent = float4(UnityObjectToWorldDir(v.tangent.xyz), v.tangent.w);
    TRANSFER_SHADOW(o);

    return o;
}

float4 frag(v2f i) : SV_Target
{
    float3 albedo = tex2D(_MainTex, i.uv).rgb;
    float specReflectance = tex2D(_SpecularTex, i.uv);
    float3 datas = tex2D(_DatasTex, i.uv);

    float3 ambientLight = 0;
				
    UNITY_LIGHT_ATTENUATION(attenuation, i, i.worldPos);
    
    float3 lightDir = _WorldSpaceLightPos0.xyz;
	
    #if defined(POINT) || defined(POINT_COOKIE) || defined(SPOT)
        lightDir = normalize(lightDir - i.worldPos);
    #endif

    float3 viewDir = normalize(_WorldSpaceCameraPos - i.worldPos);

    i.normal = normalize(i.normal);
    i.tangent.xyz = normalize(i.tangent.xyz);
    float3 binormal = CreateBinormal(i.normal, i.tangent.xyz, i.tangent.w);

    float3x3 matTangentToWorld = float3x3(i.tangent.xyz, binormal, i.normal);

    float3 normalTangent = UnpackSNormal(tex2D(_NormalTex, i.uv), _BumpScale);
    float3 normalTangentBlurred = UnpackSNormal(tex2Dbias(_NormalTex, float4(i.uv, 0, _NormalBlurredBias)), _BumpScale);

    float3 normalShade = normalize(mul(normalTangent, matTangentToWorld));
    float3 normalBlurred = normalize(mul(normalTangentBlurred, matTangentToWorld));
				
    float3 rgbLitDiffuse = EvaluateDiffuseLight(lightDir, albedo, i.normal, normalShade, normalBlurred, datas.r, attenuation, ambientLight) * datas.g;

    float3 rgbLitSpecular = EvaluateSpecularLight(lightDir, i.uv, i.normal, normalShade, viewDir, specReflectance, _Glossiness, attenuation) * datas.g;
    rgbLitSpecular *= _SpecularIntensity;
    
    float3 translucency = EvaluateFastTranslucency(lightDir, datas.b, albedo, i.normal, normalShade, viewDir);

    float3 col = rgbLitDiffuse + rgbLitSpecular + translucency;
        		
    return Tonemap(col);
}

#endif

```

## Source file: Assets/_Shaders/SS_Utils.cginc

{
  "path": "Assets/_Shaders/SS_Utils.cginc",
  "blob_sha": "cdbc646905c0661e50ffa2dbdfd1f732372c461e",
  "sha256": "3b2e51bf0421b3628df57773accd76e48c52bb0602991aeeb8a7d3016e5635a6",
  "lines": 31,
  "bytes": 825,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 4
  },
  "url": "https://github.com/faiguago/PI_SS/blob/649ce1fefc1d479d45f755fc4c06d543782f381a/Assets/_Shaders/SS_Utils.cginc",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "float2 Remap(float2 original_value, float2 original_min,"
    },
    {
      "line": 12,
      "text": "float3 CreateBinormal(float3 normal, float3 tangent, float binormalSign)"
    },
    {
      "line": 18,
      "text": "float3 UnpackSNormal(float4 packednormal, float bumpScale)"
    }
  ]
}

```text

#ifndef ___SS_UTILS____
#define ___SS_UTILS____

float2 Remap(float2 original_value, float2 original_min,
    float2 original_max, float2 new_min, float2 new_max)
{
    return new_min + (((original_value - original_min) /
        (original_max - original_min)) * (new_max - new_min));

}

float3 CreateBinormal(float3 normal, float3 tangent, float binormalSign)
{
    return normalize(cross(normal, tangent.xyz)
        * binormalSign * unity_WorldTransformParams.w);
}

float3 UnpackSNormal(float4 packednormal, float bumpScale)
{
    #if defined(UNITY_NO_DXT5nm)
	    return packednormal.xyz * 2 - 1;
    #else
        half3 normal;
        normal.xy = (packednormal.wy * 2 - 1);
        normal.xy *= bumpScale;
        normal.z = sqrt(1.0 - saturate(dot(normal.xy, normal.xy)));
        return normal;
    #endif
}

#endif

```

## Source file: Assets/_Shaders/SS_Common.cginc

{
  "path": "Assets/_Shaders/SS_Common.cginc",
  "blob_sha": "5b39eabd98516229cde7e62b4c373632fec12084",
  "sha256": "a72a8db2ce01cb90425ce368fa995e227e778321a9890a6caaeb9cbe6f5a52e8",
  "lines": 39,
  "bytes": 1041,
  "selection_score": 15,
  "keyword_line_counts": {
    "iris": 2
  },
  "url": "https://github.com/faiguago/PI_SS/blob/649ce1fefc1d479d45f755fc4c06d543782f381a/Assets/_Shaders/SS_Common.cginc",
  "function_signature_candidates": []
}

```text

#ifndef ___SS_VARIABLES_INCLUDED____
#define ___SS_VARIABLES_INCLUDED____

#include "SS_Utils.cginc"
#include "AutoLight.cginc"
#include "ToneMapping.cginc"
#include "UnityStandardBRDF.cginc"

float4 _TrnColor, _Color;

float2 _CurvatureBias, _ShadowBias;

float _IrisRadiusSource, _IrisRadiusDest, _IrisEdgeHardness, _IrisDilation;

float _AlphaMultiplier, _Glossiness, _Anisotropy, _SpecularIntensity, _TrnAmbient, _BumpScale, _NormalBlurredBias, _DiffuseEnvironmetAmount, _SpecularEnvironmentAmount, _TrnDistortion, _TrnPower;

samplerCUBE _DiffuseEnvironment, _SpecularEnvironment;

sampler2D _MainTex, _IrisTex, _SpecularTex, _DatasTex, _SpecularMaskTex, _NormalTex, _CurvatureTex_LUT, _ShadowTex_LUT;

struct appdata
{
    float4 vertex : POSITION;
    float3 normal : NORMAL;
    float4 tangent : TANGENT;
    float2 uv : TEXCOORD0;
};

struct v2f
{
    float2 uv : TEXCOORD0;
    float3 worldPos : TEXCOORD1;
    float3 normal : TEXCOORD2;
    float4 tangent : TEXCOORD3;
    float4 pos : SV_POSITION;
    SHADOW_COORDS(4)
};

#endif

```

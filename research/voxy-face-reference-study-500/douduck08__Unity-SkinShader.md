# Facial rendering source scan: douduck08/Unity-SkinShader

Repository: https://github.com/douduck08/Unity-SkinShader

Pinned source revision: e7f8272be668fef9e3a778f12d88f93cbb5cdf13

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/Materials/Skin_Pre-integrated 1.mat", "Assets/Materials/Skin_Pre-integrated 1.mat.meta", "Assets/Materials/Skin_Pre-integrated 2.mat", "Assets/Materials/Skin_Pre-integrated 2.mat.meta", "Assets/Materials/Skin_Pre-integrated 3.mat", "Assets/Materials/Skin_Pre-integrated 3.mat.meta", "Assets/Materials/human_skin_4.mat", "Assets/Materials/human_skin_4.mat.meta", "Assets/Shaders/PreIntegratedSkin.shader", "Assets/Shaders/PreIntegratedSkin.shader.meta", "Assets/Shaders/PreIntegratedSkin_Combined.shader", "Assets/Shaders/PreIntegratedSkin_Combined.shader.meta", "Assets/Shaders/SkinLighting.cginc", "Assets/Shaders/SkinLighting.cginc.meta", "Assets/Textures/1K-human_skin_4.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_ao.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_ao.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_diffuseOriginal.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_diffuseOriginal.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_edge.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_edge.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_height.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_height.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_metallic.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_metallic.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_normal.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_normal.jpg.meta", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_smoothness.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_smoothness.jpg.meta", "images/PreIntegratedSkin_1.png"], "shader": ["Assets/Shaders.meta", "Assets/Shaders/PreIntegratedSkin.shader", "Assets/Shaders/PreIntegratedSkin.shader.meta", "Assets/Shaders/PreIntegratedSkin_Combined.shader", "Assets/Shaders/PreIntegratedSkin_Combined.shader.meta", "Assets/Shaders/SkinLighting.cginc", "Assets/Shaders/SkinLighting.cginc.meta"], "rig": ["Assets/Textures/1K-human_skin_4/1K-human_skin_4_diffuseOriginal.jpg", "Assets/Textures/1K-human_skin_4/1K-human_skin_4_diffuseOriginal.jpg.meta"]}

## Source file: Assets/Shaders/SkinLighting.cginc

{
  "path": "Assets/Shaders/SkinLighting.cginc",
  "blob_sha": "6504e907b01f909f76b78578194dda3bde70b658",
  "sha256": "9291f364f578ea1183c02b9b70413441a2dde1edfd097d33f59dc2167111c183",
  "lines": 100,
  "bytes": 3303,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 8,
    "rig": 1,
    "face": 3
  },
  "url": "https://github.com/douduck08/Unity-SkinShader/blob/e7f8272be668fef9e3a778f12d88f93cbb5cdf13/Assets/Shaders/SkinLighting.cginc",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "float CaculateCurve(float3 worldNormal, float3 worldPos) {"
    },
    {
      "line": 91,
      "text": "inline void LightingSkin_GI (SurfaceOutputSkin s, UnityGIInput data, inout UnityGI gi) {"
    }
  ]
}

```text

#ifndef SKIN_LIGHTING_INCLUDED
#define SKIN_LIGHTING_INCLUDED


struct SurfaceOutputSkin {
    half3 Albedo;
    half3 Normal;
    half3 Emission;
    half Metallic;
    half Smoothness;
    half Occlusion;
    half Alpha;

    half Specular;
    half Curve;
    half Thickness;
    half Translucency;
};

// sampler2D _KelemenLUT; // TODO
sampler2D _SkinLUT;
sampler2D _TranslucencyLUT;

float CaculateCurve(float3 worldNormal, float3 worldPos) {
    return length(fwidth(worldNormal)) / length(fwidth(worldPos));
}

half FresnelReflectance(half3 H, half3 V, half F0) {
    half base = 1.0 - dot(V, H);
    half exponential = pow(base, 5.0);
    return exponential + F0 * (1.0 - exponential);
} 

half PHBeckmann(half ndoth, half m) {
    half alpha = acos(ndoth);
    half ta = tan(alpha);
    half val = 1.0 / (m * m * pow(ndoth, 4.0)) * exp(-(ta * ta) / (m * m));
    return val;
}

half3 PreintegratedDiffuse(half NdotL, half curve) {
    return tex2D(_SkinLUT, half2(NdotL * 0.5 + 0.5, curve)).rgb;
}

half SpecularBRDF(half3 normal, half3 lightDir, half3 viewDir, half roughness) {
    // Kelemen and Szirmay-Kalos specular BRDF
    // D: Beckmann
    // G: Simplification of the Cook-Torrance

    half3 halfDir = lightDir + viewDir;
    half G = rcp(dot(halfDir, halfDir)); // Simplify from `G_original / (4 * NdotL * NdotV)`

    halfDir = normalize(halfDir);
    half NdotH = dot(halfDir, normal);
    half D = PHBeckmann(NdotH, roughness);
    half F = FresnelReflectance(halfDir, viewDir, 0.028);

    return max(D * G * F, 0);
}

half3 Transmittance (half3 normal, half3 lightDir, half d) {
    half irradiance = max(0.3 + dot(-normal, lightDir), 0.0);
    return tex2D(_TranslucencyLUT, half2(d, 0)).rgb * irradiance;
}

inline half4 LightingSkin (SurfaceOutputSkin s, float3 viewDir, UnityGI gi) {
    s.Normal = normalize(s.Normal);

    half oneMinusReflectivity;
    half3 specColor;
    s.Albedo = DiffuseAndSpecularFromMetallic (s.Albedo, s.Metallic, /*out*/ specColor, /*out*/ oneMinusReflectivity);

    half outputAlpha;
    s.Albedo = PreMultiplyAlpha (s.Albedo, s.Alpha, oneMinusReflectivity, /*out*/ outputAlpha);

    // -- lighting --
    half NdotL = saturate(dot(gi.light.dir, s.Normal));
    half roughness = max(0.02, (1 - s.Smoothness) * (1 - s.Smoothness));

    half3 diffuseTerm = PreintegratedDiffuse(NdotL, s.Curve);
    half3 transmitTerm = Transmittance(s.Normal, gi.light.dir, s.Thickness) * s.Translucency;
    half3 specularTerm = SpecularBRDF(s.Normal, gi.light.dir, viewDir, roughness) * NdotL * s.Specular;

    half3 diffuse = gi.light.color * NdotL;
    half3 c = s.Albedo * (gi.light.color * (diffuseTerm + transmitTerm) + gi.indirect.diffuse);
    c += specColor * gi.light.color * specularTerm;

    return half4(c, outputAlpha);
}

inline void LightingSkin_GI (SurfaceOutputSkin s, UnityGIInput data, inout UnityGI gi) {
    #if defined(UNITY_PASS_DEFERRED) && UNITY_ENABLE_REFLECTION_BUFFERS
    gi = UnityGlobalIllumination(data, s.Occlusion, s.Normal);
    #else
    Unity_GlossyEnvironmentData g = UnityGlossyEnvironmentSetup(s.Smoothness, data.worldViewDir, s.Normal, lerp(unity_ColorSpaceDielectricSpec.rgb, s.Albedo, s.Metallic));
    gi = UnityGlobalIllumination(data, s.Occlusion, s.Normal, g);
    #endif
}

#endif // SKIN_LIGHTING_INCLUDED

```

## Source file: Assets/Shaders/PreIntegratedSkin.shader

{
  "path": "Assets/Shaders/PreIntegratedSkin.shader",
  "blob_sha": "3734a8b8500f64450f1879ae03d68ae4f9b5361c",
  "sha256": "2d1f2e9dd81a5b8aa76303b33c76b8e9a4b65e829774c58e90ddcfadf81b1cda",
  "lines": 102,
  "bytes": 3274,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 5,
    "shader": 3,
    "face": 2
  },
  "url": "https://github.com/douduck08/Unity-SkinShader/blob/e7f8272be668fef9e3a778f12d88f93cbb5cdf13/Assets/Shaders/PreIntegratedSkin.shader",
  "function_signature_candidates": [
    {
      "line": 74,
      "text": "void surf (Input IN, inout SurfaceOutputSkin o) {"
    }
  ]
}

```text

﻿Shader "Skin/Pre-integrated Skin"
{
    Properties
    {
        _Color ("Color", Color) = (1,1,1,1)
        _MainTex ("Albedo (RGB)", 2D) = "white" {}
        _Specular ("Specular", Range(0,10)) = 1.0

        _MetallicMap ("Metallic Map", 2D) = "white" {}
        _Metallic ("Metallic", Range(0,1)) = 0.0
        _GlossinessMap ("Smoothness Map", 2D) = "white" {}
        _Glossiness ("Smoothness", Range(0,1)) = 0.5
        
        _BumpMap ("Normal Map", 2D) = "bump" {}
        _BumpStrength ("Normal Strength", Range(0,2)) = 1
        _OcclusionMap ("Occlusion Map", 2D) = "white" {}
        _Occlusion ("Occlusion", Range(0,1)) = 0.0

        _TranslucencyMap ("Translucency Map", 2D) = "white" {}
        _Translucency ("Translucency", Range(0,1)) = 1.0
        _ThicknessMap ("Thickness Map", 2D) = "white" {}
        _Thickness ("Thickness", Range(0,1)) = 1.0

        [Header(Scattering Curve)]
        _Curve ("Curve", Range(0,1)) = 1.0
        _CurveMap ("Curve Map (R)", 2D) = "white" {}
        [Toggle(_CALC_CURVE)] _CalcuCurve ("Caculate Curve", Int) = 0

        [Header(LUT)]
        _SkinLUT ("Skin LUT", 2D) = "white" {}
        _TranslucencyLUT ("Translucency LUT", 2D) = "white" {}
    }
    SubShader
    {
        Tags { "RenderType"="Opaque" }
        LOD 200

        CGPROGRAM
        #pragma surface surf Skin fullforwardshadows
        #pragma target 3.0

        #pragma shader_feature _CALC_CURVE
        #include "SkinLighting.cginc"

        struct Input {
            float2 uv_MainTex;
            #ifdef _CALC_CURVE
            float3 worldPos;
            float3 worldNormal;
            INTERNAL_DATA
            #endif
        };

        sampler2D _MainTex;
        sampler2D _MetallicMap;
        sampler2D _GlossinessMap;
        sampler2D _BumpMap;
        sampler2D _OcclusionMap;
        sampler2D _TranslucencyMap;
        sampler2D _ThicknessMap;

        half4 _Color;
        half _Specular;
        half _Metallic;
        half _Glossiness;
        half _BumpStrength;
        half _Occlusion;
        half _Translucency;
        half _Thickness;

        sampler2D _CurveMap;
        half _Curve;

        void surf (Input IN, inout SurfaceOutputSkin o) {
            half4 c = tex2D (_MainTex, IN.uv_MainTex) * _Color;
            o.Albedo = c.rgb;
            o.Alpha = c.a;
            
            o.Normal = UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
            o.Normal.xy *= _BumpStrength;

            o.Metallic = tex2D(_MetallicMap, IN.uv_MainTex).r * _Metallic;
            o.Smoothness = tex2D(_GlossinessMap, IN.uv_MainTex).r * _Glossiness;
            o.Occlusion = tex2D(_OcclusionMap, IN.uv_MainTex).r * _Occlusion;
            o.Emission = 0;

            o.Specular = _Specular;
            o.Thickness = tex2D(_ThicknessMap, IN.uv_MainTex).r * _Thickness;
            o.Translucency = tex2D(_TranslucencyMap, IN.uv_MainTex).r * _Translucency;

            #ifdef _CALC_CURVE
            IN.worldNormal = WorldNormalVector(IN, float3(0,0,1));
            o.Curve = length(fwidth(IN.worldNormal)) / length(fwidth(IN.worldPos)) * _Curve;
            #else
            o.Curve = tex2D (_CurveMap, IN.uv_MainTex).r * _Curve;
            #endif
        }

        ENDCG
    }
    FallBack "Diffuse"
}


```

## Source file: Assets/Shaders/PreIntegratedSkin_Combined.shader

{
  "path": "Assets/Shaders/PreIntegratedSkin_Combined.shader",
  "blob_sha": "ef8b32dae3a05175a3afb8f54c96c4170ad8cc80",
  "sha256": "5d9b5885e2bd27d46a0f2d0ae3a5b37cf31bdf31f433434a9d16b74b97a63d8e",
  "lines": 96,
  "bytes": 2932,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 5,
    "shader": 3,
    "face": 2
  },
  "url": "https://github.com/douduck08/Unity-SkinShader/blob/e7f8272be668fef9e3a778f12d88f93cbb5cdf13/Assets/Shaders/PreIntegratedSkin_Combined.shader",
  "function_signature_candidates": [
    {
      "line": 67,
      "text": "void surf (Input IN, inout SurfaceOutputSkin o) {"
    }
  ]
}

```text

﻿Shader "Skin/Pre-integrated Skin (Combined)"
{
    Properties
    {
        _Color ("Color", Color) = (1,1,1,1)
        _MainTex ("Albedo (RGB)", 2D) = "white" {}
        _Specular ("Specular", Range(0,10)) = 1.0

        _CombinedMap ("Occlusion(R) Smoothness(G) Metallic(B) Translucency(A)", 2D) = "white" {}
        _Metallic ("Metallic", Range(0,1)) = 0.0
        _Glossiness ("Smoothness", Range(0,1)) = 0.5
        _Occlusion ("Occlusion", Range(0,1)) = 0.0
        _Translucency ("Translucency", Range(0,1)) = 1.0
        
        _BumpMap ("Normal Map", 2D) = "bump" {}
        _BumpStrength ("Normal Strength", Range(0,2)) = 1
        _ThicknessMap ("Thickness Map", 2D) = "white" {}
        _Thickness ("Thickness", Range(0,1)) = 1.0

        [Header(Scattering Curve)]
        _Curve ("Curve", Range(0,1)) = 1.0
        _CurveMap ("Curve Map (R)", 2D) = "white" {}
        [Toggle(_CALC_CURVE)] _CalcuCurve ("Caculate Curve", Int) = 0

        [Header(LUT)]
        _SkinLUT ("Skin LUT", 2D) = "white" {}
        _TranslucencyLUT ("Translucency LUT", 2D) = "white" {}
    }
    SubShader
    {
        Tags { "RenderType"="Opaque" }
        LOD 200

        CGPROGRAM
        #pragma surface surf Skin fullforwardshadows
        #pragma target 3.0

        #pragma shader_feature _CALC_CURVE
        #include "SkinLighting.cginc"

        struct Input {
            float2 uv_MainTex;
            #ifdef _CALC_CURVE
            float3 worldPos;
            float3 worldNormal;
            INTERNAL_DATA
            #endif
        };

        sampler2D _MainTex;
        sampler2D _CombinedMap;
        sampler2D _BumpMap;
        sampler2D _ThicknessMap;

        half4 _Color;
        half _Specular;
        half _Metallic;
        half _Glossiness;
        half _BumpStrength;
        half _Occlusion;
        half _Translucency;
        half _Thickness;

        sampler2D _CurveMap;
        half _Curve;

        void surf (Input IN, inout SurfaceOutputSkin o) {
            half4 c = tex2D (_MainTex, IN.uv_MainTex) * _Color;
            o.Albedo = c.rgb;
            o.Alpha = c.a;
            
            o.Normal = UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
            o.Normal.xy *= _BumpStrength;

            half4 combined = tex2D(_CombinedMap, IN.uv_MainTex);

            o.Metallic = combined.b * _Metallic;
            o.Smoothness = combined.g * _Glossiness;
            o.Occlusion = combined.r * _Occlusion;
            o.Emission = 0;

            o.Specular = _Specular;
            o.Thickness = tex2D(_ThicknessMap, IN.uv_MainTex).r * _Thickness;
            o.Translucency = combined.a * _Translucency;

            #ifdef _CALC_CURVE
            o.Curve = length(fwidth(IN.worldNormal)) / length(fwidth(IN.worldPos)) * _Curve;
            #else
            o.Curve = tex2D (_CurveMap, IN.uv_MainTex).r * _Curve;
            #endif
        }

        ENDCG
    }
    FallBack "Diffuse"
}


```

## Source file: Assets/Editor/PrecomputeTranslucency.cs

{
  "path": "Assets/Editor/PrecomputeTranslucency.cs",
  "blob_sha": "55b08d95df0c0166eecf9fe33b5e8469f19e6f0d",
  "sha256": "f882d737e79ded9328567eb364e18a32bd259b2afcfed4306a05ea6a0ba1a045",
  "lines": 46,
  "bytes": 1673,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/douduck08/Unity-SkinShader/blob/e7f8272be668fef9e3a778f12d88f93cbb5cdf13/Assets/Editor/PrecomputeTranslucency.cs",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "static void CreatePrecomputeTranslucencyTexture () {"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using UnityEditor;

public static class PrecomputeTranslucency {

    const int WIDTH = 512;
    const int HEIGHT = 4;

    // http://iryoku.com/translucency/
    static Vector3 T (float s) {
        return new Vector3 (0.233f, 0.455f, 0.649f) * Mathf.Exp (-s * s / 0.0064f) +
            new Vector3 (0.100f, 0.336f, 0.344f) * Mathf.Exp (-s * s / 0.0484f) +
            new Vector3 (0.118f, 0.198f, 0.000f) * Mathf.Exp (-s * s / 0.187f) +
            new Vector3 (0.113f, 0.007f, 0.007f) * Mathf.Exp (-s * s / 0.567f) +
            new Vector3 (0.358f, 0.004f, 0.000f) * Mathf.Exp (-s * s / 1.99f) +
            new Vector3 (0.078f, 0.000f, 0.000f) * Mathf.Exp (-s * s / 7.41f);
    }

    [MenuItem ("Tools/Create Precompute Translucency Texture")]
    static void CreatePrecomputeTranslucencyTexture () {
        string path = EditorUtility.SaveFilePanelInProject ("Create Precompute Translucency Texture", "PrecomputeTranslucency", "jpg", "");
        if (string.IsNullOrEmpty (path)) {
            return;
        }

        var output = new Texture2D (WIDTH, HEIGHT, TextureFormat.RGB24, false);
        for (int x = 0; x < WIDTH; x++) {
            var s = x * (1f / (WIDTH - 1));
            var trans = T (s);
            var color = new Color (trans.x, trans.y, trans.z);
            for (int y = 0; y < HEIGHT; y++) {
                output.SetPixel (x, y, color);
            }
        }
        output.Apply ();

        var bytes = ImageConversion.EncodeToJPG (output, 100);
        File.WriteAllBytes (path, bytes);

        AssetDatabase.Refresh ();
    }

}

```

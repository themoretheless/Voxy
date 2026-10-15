# Facial rendering source scan: Yhisken/Unity_SGSSS

Repository: https://github.com/Yhisken/Unity_SGSSS

Pinned source revision: 32324559abf2eacd4f0f44d2e39cd48aa3c5d4a5

Discovery topic: subsurface scattering

Description: Spherical Gaussian Subsurface Scattering Based by Unity Shader

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"sss": ["SGSSS/Head.fbx", "SGSSS/Head.fbx.meta", "SGSSS/Materials.meta", "SGSSS/Materials/SGSSS.mat", "SGSSS/Materials/SGSSS.mat.meta", "SGSSS/Materials/SGSSS_Test.mat", "SGSSS/Materials/SGSSS_Test.mat.meta", "SGSSS/SGSSS.unity", "SGSSS/SGSSS.unity.meta", "SGSSS/Shader.meta", "SGSSS/Shader/BakedLitDepthNormalsPass.hlsl", "SGSSS/Shader/BakedLitDepthNormalsPass.hlsl.meta", "SGSSS/Shader/BakedLitForwardPass.hlsl", "SGSSS/Shader/BakedLitForwardPass.hlsl.meta", "SGSSS/Shader/BakedLitInput.hlsl", "SGSSS/Shader/BakedLitInput.hlsl.meta", "SGSSS/Shader/BakedLitMetaPass.hlsl", "SGSSS/Shader/BakedLitMetaPass.hlsl.meta", "SGSSS/Shader/DepthNormalsPass.hlsl", "SGSSS/Shader/DepthNormalsPass.hlsl.meta", "SGSSS/Shader/DepthOnlyPass.hlsl", "SGSSS/Shader/DepthOnlyPass.hlsl.meta", "SGSSS/Shader/LitDepthNormalsPass.hlsl", "SGSSS/Shader/LitDepthNormalsPass.hlsl.meta", "SGSSS/Shader/LitForwardPass.hlsl", "SGSSS/Shader/LitForwardPass.hlsl.meta", "SGSSS/Shader/LitGBufferPass.hlsl", "SGSSS/Shader/LitGBufferPass.hlsl.meta", "SGSSS/Shader/LitInput.hlsl", "SGSSS/Shader/LitInput.hlsl.meta"], "shader": ["SGSSS/Shader.meta", "SGSSS/Shader/BakedLitDepthNormalsPass.hlsl", "SGSSS/Shader/BakedLitDepthNormalsPass.hlsl.meta", "SGSSS/Shader/BakedLitForwardPass.hlsl", "SGSSS/Shader/BakedLitForwardPass.hlsl.meta", "SGSSS/Shader/BakedLitInput.hlsl", "SGSSS/Shader/BakedLitInput.hlsl.meta", "SGSSS/Shader/BakedLitMetaPass.hlsl", "SGSSS/Shader/BakedLitMetaPass.hlsl.meta", "SGSSS/Shader/DepthNormalsPass.hlsl", "SGSSS/Shader/DepthNormalsPass.hlsl.meta", "SGSSS/Shader/DepthOnlyPass.hlsl", "SGSSS/Shader/DepthOnlyPass.hlsl.meta", "SGSSS/Shader/LitDepthNormalsPass.hlsl", "SGSSS/Shader/LitDepthNormalsPass.hlsl.meta", "SGSSS/Shader/LitForwardPass.hlsl", "SGSSS/Shader/LitForwardPass.hlsl.meta", "SGSSS/Shader/LitGBufferPass.hlsl", "SGSSS/Shader/LitGBufferPass.hlsl.meta", "SGSSS/Shader/LitInput.hlsl", "SGSSS/Shader/LitInput.hlsl.meta", "SGSSS/Shader/LitMetaPass.hlsl", "SGSSS/Shader/LitMetaPass.hlsl.meta", "SGSSS/Shader/SGSSS.shader", "SGSSS/Shader/SGSSS.shader.meta", "SGSSS/Shader/SGSSS_Test.shader", "SGSSS/Shader/SGSSS_Test.shader.meta", "SGSSS/Shader/ShadowCasterPass.hlsl", "SGSSS/Shader/ShadowCasterPass.hlsl.meta", "SGSSS/Shader/SimpleLitDepthNormalsPass.hlsl"], "face": ["SGSSS/ShaderLibrary/NormalSurfaceGradient.hlsl", "SGSSS/ShaderLibrary/NormalSurfaceGradient.hlsl.meta", "SGSSS/ShaderLibrary/SurfaceData.hlsl", "SGSSS/ShaderLibrary/SurfaceData.hlsl.meta", "SGSSS/ShaderLibrary/SurfaceInput.hlsl", "SGSSS/ShaderLibrary/SurfaceInput.hlsl.meta"]}

## Source file: SGSSS/ShaderLibrary/SurfaceData.hlsl

{
  "path": "SGSSS/ShaderLibrary/SurfaceData.hlsl",
  "blob_sha": "5e6246ff5e99b0f1ca34eeead1d7bd2fc4da1f47",
  "sha256": "531cb92748277ae683de2652ef18e04fb4e018b194233a41ec742d418861f605",
  "lines": 22,
  "bytes": 439,
  "selection_score": 33,
  "keyword_line_counts": {
    "shader": 1,
    "face": 3
  },
  "url": "https://github.com/Yhisken/Unity_SGSSS/blob/32324559abf2eacd4f0f44d2e39cd48aa3c5d4a5/SGSSS/ShaderLibrary/SurfaceData.hlsl",
  "function_signature_candidates": []
}

```text

#ifndef UNIVERSAL_SURFACE_DATA_INCLUDED
#define UNIVERSAL_SURFACE_DATA_INCLUDED

// Must match Universal ShaderGraph master node
struct SurfaceData
{
    half3 albedo;
    half3 specular;
    half  metallic;
    half  smoothness;
    half  smoothness_2;
    half curvature;
    half thickness;
    half3 normalTS;
    half3 emission;
    half  occlusion;
    half  alpha;
    half  clearCoatMask;
    half  clearCoatSmoothness;
};

#endif


```

## Source file: SGSSS/ShaderLibrary/SurfaceInput.hlsl

{
  "path": "SGSSS/ShaderLibrary/SurfaceInput.hlsl",
  "blob_sha": "2f8b80dfd82f9b268a9efdff3c37c53c78dd7211",
  "sha256": "6d7513fb224b8f8fc44d591fe69f8ad396b3ee7424e31f322b18ea673aa58a69",
  "lines": 62,
  "bytes": 1816,
  "selection_score": 33,
  "keyword_line_counts": {
    "sss": 4,
    "shader": 4,
    "face": 3
  },
  "url": "https://github.com/Yhisken/Unity_SGSSS/blob/32324559abf2eacd4f0f44d2e39cd48aa3c5d4a5/SGSSS/ShaderLibrary/SurfaceInput.hlsl",
  "function_signature_candidates": []
}

```text

#ifndef UNIVERSAL_INPUT_SURFACE_INCLUDED
#define UNIVERSAL_INPUT_SURFACE_INCLUDED

#include "Assets/SGSSS/ShaderLibrary/Core.hlsl"
#include "Assets/SGSSS/ShaderLibrary/SurfaceData.hlsl"
#include "Assets/SGSSS/ShaderLibrary/Packing.hlsl"
#include "Assets/SGSSS/ShaderLibrary/CommonMaterial.hlsl"

TEXTURE2D(_BaseMap);
SAMPLER(sampler_BaseMap);
float4 _BaseMap_TexelSize;
float4 _BaseMap_MipInfo;
TEXTURE2D(_BumpMap);
SAMPLER(sampler_BumpMap);
TEXTURE2D(_EmissionMap);
SAMPLER(sampler_EmissionMap);

///////////////////////////////////////////////////////////////////////////////
//                      Material Property Helpers                            //
///////////////////////////////////////////////////////////////////////////////
half Alpha(half albedoAlpha, half4 color, half cutoff)
{
#if !defined(_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A) && !defined(_GLOSSINESS_FROM_BASE_ALPHA)
    half alpha = albedoAlpha * color.a;
#else
    half alpha = color.a;
#endif

    alpha = AlphaDiscard(alpha, cutoff);

    return alpha;
}

half4 SampleAlbedoAlpha(float2 uv, TEXTURE2D_PARAM(albedoAlphaMap, sampler_albedoAlphaMap))
{
    return half4(SAMPLE_TEXTURE2D(albedoAlphaMap, sampler_albedoAlphaMap, uv));
}

half3 SampleNormal(float2 uv, TEXTURE2D_PARAM(bumpMap, sampler_bumpMap), half scale = half(1.0))
{
#ifdef _NORMALMAP
    half4 n = SAMPLE_TEXTURE2D(bumpMap, sampler_bumpMap, uv);
    #if BUMP_SCALE_NOT_SUPPORTED
        return UnpackNormal(n);
    #else
        return UnpackNormalScale(n, scale);
    #endif
#else
    return half3(0.0h, 0.0h, 1.0h);
#endif
}

half3 SampleEmission(float2 uv, half3 emissionColor, TEXTURE2D_PARAM(emissionMap, sampler_emissionMap))
{
#ifndef _EMISSION
    return 0;
#else
    return SAMPLE_TEXTURE2D(emissionMap, sampler_emissionMap, uv).rgb * emissionColor;
#endif
}

#endif


```

## Source file: SGSSS/ShaderLibrary/NormalSurfaceGradient.hlsl

{
  "path": "SGSSS/ShaderLibrary/NormalSurfaceGradient.hlsl",
  "blob_sha": "7a406327879bba81cacb8612f8c0a0f04405b90a",
  "sha256": "a34e9ccf9994599108d67048d8e44d912b9397727eeef1a9281f793d36939c89",
  "lines": 100,
  "bytes": 5058,
  "selection_score": 33,
  "keyword_line_counts": {
    "rig": 1,
    "face": 15,
    "lip": 2
  },
  "url": "https://github.com/Yhisken/Unity_SGSSS/blob/32324559abf2eacd4f0f44d2e39cd48aa3c5d4a5/SGSSS/ShaderLibrary/NormalSurfaceGradient.hlsl",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "void SurfaceGradientGenBasisTB(float3 nrmVertexNormal, float3 sigmaX, float3 sigmaY, float flipSign, float2 texST, out float3 vT, out float3 vB)"
    }
  ]
}

```text

// this produces an orthonormal basis of the tangent and bitangent WITHOUT vertex level tangent/bitangent for any UV including procedurally generated
// method released with the demo for publication of "bump mapping unparametrized surfaces on the GPU"
// http://mmikkelsen3d.blogspot.com/2011/07/derivative-maps.html
void SurfaceGradientGenBasisTB(float3 nrmVertexNormal, float3 sigmaX, float3 sigmaY, float flipSign, float2 texST, out float3 vT, out float3 vB)
{
    float2 dSTdx = ddx_fine(texST), dSTdy = ddy_fine(texST);

    float det = dot(dSTdx, float2(dSTdy.y, -dSTdy.x));
    float sign_det = det < 0 ? -1 : 1;

    // invC0 represents (dXds, dYds); but we don't divide by determinant (scale by sign instead)
    float2 invC0 = sign_det * float2(dSTdy.y, -dSTdx.y);
    vT = sigmaX * invC0.x + sigmaY * invC0.y;
    if (abs(det) > 0.0)
        vT = normalize(vT);
    vB = (sign_det * flipSign) * cross(nrmVertexNormal, vT);
}

// surface gradient from an on the fly TBN (deriv obtained using tspaceNormalToDerivative()) or from conventional vertex level TBN (mikktspace compliant and deriv obtained using tspaceNormalToDerivative())
real3 SurfaceGradientFromTBN(real2 deriv, real3 vT, real3 vB)
{
    return deriv.x * vT + deriv.y * vB;
}

// surface gradient from an already generated "normal" such as from an object or world space normal map
// CAUTION: nrmVertexNormal and v must be in the same space. i.e world or object
// this allows us to mix the contribution together with a series of other contributions including tangent space normals
// v does not need to be unit length as long as it establishes the direction.
real3 SurfaceGradientFromPerturbedNormal(real3 nrmVertexNormal, real3 v)
{
    real3 n = nrmVertexNormal;
    real s = 1.0 / max(REAL_EPS, abs(dot(n, v)));
    return s * (dot(n, v) * n - v);
}

// used to produce a surface gradient from the gradient of a volume bump function such as a volume of perlin noise.
// equation 2. in "bump mapping unparametrized surfaces on the GPU".
// Observe the difference in figure 2. between using the gradient vs. the surface gradient to do bump mapping (the original method is proved wrong in the paper!).
real3 SurfaceGradientFromVolumeGradient(real3 nrmVertexNormal, real3 grad)
{
    return grad - dot(nrmVertexNormal, grad) * nrmVertexNormal;
}

// triplanar projection considered special case of volume bump map
// described here:  http://mmikkelsen3d.blogspot.com/2013/10/volume-height-maps-and-triplanar-bump.html
// derivs obtained using tspaceNormalToDerivative() and weights using computeTriplanarWeights().
real3 SurfaceGradientFromTriplanarProjection(real3 nrmVertexNormal, real3 triplanarWeights, real2 deriv_xplane, real2 deriv_yplane, real2 deriv_zplane)
{
    const real w0 = triplanarWeights.x, w1 = triplanarWeights.y, w2 = triplanarWeights.z;

    // Assume derivXplane, derivYPlane and derivZPlane sampled using (z,y), (x,z) and (x,y) respectively
    // (ie using Morten's convention http://jcgt.org/published/0009/03/04/ p80-81 for left handed worldspace)
    // positive scales of the look-up coordinate will work as well but for negative scales the derivative components will need to be negated accordingly.
    real3 volumeGrad = real3(w2 * deriv_zplane.x + w1 * deriv_yplane.x, w2 * deriv_zplane.y + w0 * deriv_xplane.y, w0 * deriv_xplane.x + w1 * deriv_yplane.y);

    return SurfaceGradientFromVolumeGradient(nrmVertexNormal, volumeGrad);
}

real3 SurfaceGradientResolveNormal(real3 nrmVertexNormal, real3 surfGrad)
{
    return SafeNormalize(nrmVertexNormal - surfGrad);
}

real2 ConvertTangentSpaceNormalToHeightMapGradient(real2 normalXY, real rcpNormalZ, real scale)
{
    // scale * (-normal.xy / normal.z)
    return normalXY * (-rcpNormalZ * scale);
}

real3 SurfaceGradientFromTangentSpaceNormalAndFromTBN(real3 normalTS, real3 vT, real3 vB, real scale = 1.0)
{
    float2 deriv = ConvertTangentSpaceNormalToHeightMapGradient(normalTS.xy, rcp(max(normalTS.z, REAL_EPS)), scale);
    return SurfaceGradientFromTBN(deriv, vT, vB);
}

// Converts tangent space normal to slopes (height map gradient).
real2 UnpackDerivativeNormalRGB(real4 packedNormal, real scale = 1.0)
{
    real3 vT   = packedNormal.rgb * 2.0 - 1.0; // Unsigned to signed
    real  rcpZ = rcp(max(vT.z, REAL_EPS));      // Clamp to avoid INF

    return ConvertTangentSpaceNormalToHeightMapGradient(vT.xy, rcpZ, scale);
}

// Converts tangent space normal to slopes (height map gradient).
real2 UnpackDerivativeNormalAG(real4 packedNormal, real scale = 1.0)
{
    real2 vT   = packedNormal.ag * 2.0 - 1.0;                      // Unsigned to signed
    real  rcpZ = rsqrt(max(1 - Sq(vT.x) - Sq(vT.y), HALF_MIN_SQRT)); // Clamp to avoid INF

    return ConvertTangentSpaceNormalToHeightMapGradient(vT.xy, rcpZ, scale);
}

// Unpack normal as DXT5nm (1, y, 0, x) or BC5 (x, y, 0, 1)
real2 UnpackDerivativeNormalRGorAG(real4 packedNormal, real scale = 1.0)
{
    // Convert to (?, y, 0, x)
    packedNormal.a *= packedNormal.r;
    return UnpackDerivativeNormalAG(packedNormal, scale);
}


```

## Source file: SGSSS/Shader/SGSSS.shader

{
  "path": "SGSSS/Shader/SGSSS.shader",
  "blob_sha": "00253973fc08929a144921b363f6faaf780eb0a0",
  "sha256": "01295e5edb80c19ce4e34a9c74f6ecab885dac06d6b0fa63b76559503d777d38",
  "lines": 365,
  "bytes": 14533,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 7,
    "shader": 48,
    "face": 2,
    "lip": 1
  },
  "url": "https://github.com/Yhisken/Unity_SGSSS/blob/32324559abf2eacd4f0f44d2e39cd48aa3c5d4a5/SGSSS/Shader/SGSSS.shader",
  "function_signature_candidates": []
}

```text

Shader "Universal Render Pipeline/SGSSS_Lit"
{
    Properties
    {
        _WorkflowMode("WorkflowMode", Float) = 1.0

        [MainTexture] _BaseMap("Albedo", 2D) = "white" {}
        [MainColor] _BaseColor("Color", Color) = (1,1,1,1)
        [Curvature] _Curvature("Curvature", 2D) = "black" {}
        [ScatterAmt] _ScatterAmt("ScatterAmt", Vector) = (1, 1, 1, 1)
        _Cutoff("Alpha Cutoff", Range(0.0, 1.0)) = 0.5
        
        _Smoothness("Smoothness", Range(0.0, 1.0)) = 0.5
        _Smoothness2 ("Smoothness2", Range(0.0, 1.0)) = 0.5
        _SmoothRatio ("SmoothRatio", Range(0.0, 1.0)) = 0.5
        _SmoothnessTextureChannel("Smoothness texture channel", Float) = 0

        _Metallic("Metallic", Range(0.0, 1.0)) = 0.0
        _MetallicGlossMap("Metallic", 2D) = "white" {}

        _SpecColor("Specular", Color) = (0.2, 0.2, 0.2)
        _SpecGlossMap("Specular", 2D) = "white" {}

        [ToggleOff] _SpecularHighlights("Specular Highlights", Float) = 1.0
        [ToggleOff] _EnvironmentReflections("Environment Reflections", Float) = 1.0

        _BumpScale("Scale", Float) = 1.0
        _BumpMap("Normal Map", 2D) = "bump" {}

        _Parallax("Scale", Range(0.005, 0.08)) = 0.005
        _ParallaxMap("Height Map", 2D) = "black" {}

        _OcclusionStrength("Strength", Range(0.0, 1.0)) = 1.0
        _OcclusionMap("Occlusion", 2D) = "white" {}

        [HDR] _EmissionColor("Color", Color) = (0,0,0)
        _EmissionMap("Emission", 2D) = "white" {}
        _Thickness("Thickness Map", 2D) = "black" {}
        
        _DetailMask("Detail Mask", 2D) = "white" {}
        _DetailAlbedoMapScale("Scale", Range(0.0, 2.0)) = 1.0
        _DetailAlbedoMap("Detail Albedo x2", 2D) = "linearGrey" {}
        _DetailNormalMapScale("Scale", Range(0.0, 2.0)) = 1.0
        [Normal] _DetailNormalMap("Normal Map", 2D) = "bump" {}

        // SRP batching compatibility for Clear Coat (Not used in Lit)
        [HideInInspector] _ClearCoatMask("_ClearCoatMask", Float) = 0.0
        [HideInInspector] _ClearCoatSmoothness("_ClearCoatSmoothness", Float) = 0.0

        // Blending state
        _Surface("__surface", Float) = 0.0
        _Blend("__blend", Float) = 0.0
        _Cull("__cull", Float) = 2.0
        [ToggleUI] _AlphaClip("__clip", Float) = 0.0
        [HideInInspector] _SrcBlend("__src", Float) = 1.0
        [HideInInspector] _DstBlend("__dst", Float) = 0.0
        [HideInInspector] _SrcBlendAlpha("__srcA", Float) = 1.0
        [HideInInspector] _DstBlendAlpha("__dstA", Float) = 0.0
        [HideInInspector] _ZWrite("__zw", Float) = 1.0
        [HideInInspector] _BlendModePreserveSpecular("_BlendModePreserveSpecular", Float) = 1.0
        [HideInInspector] _AlphaToMask("__alphaToMask", Float) = 0.0

        [ToggleUI] _ReceiveShadows("Receive Shadows", Float) = 1.0
        // Editmode props
        _QueueOffset("Queue offset", Float) = 0.0

        // ObsoleteProperties
        [HideInInspector] _MainTex("BaseMap", 2D) = "white" {}
        [HideInInspector] _Color("Base Color", Color) = (1, 1, 1, 1)
        [HideInInspector] _GlossMapScale("Smoothness", Float) = 0.0
        [HideInInspector] _Glossiness("Smoothness", Float) = 0.0
        [HideInInspector] _GlossyReflections("EnvironmentReflections", Float) = 0.0

        [HideInInspector][NoScaleOffset]unity_Lightmaps("unity_Lightmaps", 2DArray) = "" {}
        [HideInInspector][NoScaleOffset]unity_LightmapsInd("unity_LightmapsInd", 2DArray) = "" {}
        [HideInInspector][NoScaleOffset]unity_ShadowMasks("unity_ShadowMasks", 2DArray) = "" {}
    }

    SubShader
    {
        // Universal Pipeline tag is required. If Universal render pipeline is not set in the graphics settings
        // this Subshader will fail. One can add a subshader below or fallback to Standard built-in to make this
        // material work with both Universal Render Pipeline and Builtin Unity Pipeline
        Tags
        {
            "RenderType" = "Opaque"
            "RenderPipeline" = "UniversalPipeline"
            "UniversalMaterialType" = "Lit"
            "IgnoreProjector" = "True"
        }
        LOD 300

        // ------------------------------------------------------------------
        //  Forward pass. Shades all light in a single pass. GI + emission + Fog
        Pass
        {
            // Lightmode matches the ShaderPassName set in UniversalRenderPipeline.cs. SRPDefaultUnlit and passes with
            // no LightMode tag are also rendered by Universal Render Pipeline
            Name "ForwardLit"
            Tags
            {
                "LightMode" = "UniversalForward"
            }

            // -------------------------------------
            // Render State Commands
            Blend[_SrcBlend][_DstBlend], [_SrcBlendAlpha][_DstBlendAlpha]
            ZWrite[_ZWrite]
            Cull[_Cull]
            AlphaToMask[_AlphaToMask]

            HLSLPROGRAM
            #pragma target 2.0

            // -------------------------------------
            // Shader Stages
            #pragma vertex LitPassVertex
            #pragma fragment LitPassFragment

            // -------------------------------------
            // Material Keywords
            #pragma shader_feature_local _NORMALMAP
            #pragma shader_feature_local _PARALLAXMAP
            #pragma shader_feature_local _RECEIVE_SHADOWS_OFF
            #pragma shader_feature_local _ _DETAIL_MULX2 _DETAIL_SCALED
            #pragma shader_feature_local_fragment _SURFACE_TYPE_TRANSPARENT
            #pragma shader_feature_local_fragment _ALPHATEST_ON
            #pragma shader_feature_local_fragment _ _ALPHAPREMULTIPLY_ON _ALPHAMODULATE_ON
            #pragma shader_feature_local_fragment _EMISSION
            #pragma shader_feature_local_fragment _METALLICSPECGLOSSMAP
            #pragma shader_feature_local_fragment _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A
            #pragma shader_feature_local_fragment _OCCLUSIONMAP
            #pragma shader_feature_local_fragment _SPECULARHIGHLIGHTS_OFF
            #pragma shader_feature_local_fragment _ENVIRONMENTREFLECTIONS_OFF
            #pragma shader_feature_local_fragment _SPECULAR_SETUP

            // -------------------------------------
            // Universal Pipeline keywords
            #pragma multi_compile _ _MAIN_LIGHT_SHADOWS _MAIN_LIGHT_SHADOWS_CASCADE _MAIN_LIGHT_SHADOWS_SCREEN
            #pragma multi_compile _ _ADDITIONAL_LIGHTS_VERTEX _ADDITIONAL_LIGHTS
            #pragma multi_compile _ EVALUATE_SH_MIXED EVALUATE_SH_VERTEX
            #pragma multi_compile_fragment _ _ADDITIONAL_LIGHT_SHADOWS
            #pragma multi_compile_fragment _ _REFLECTION_PROBE_BLENDING
            #pragma multi_compile_fragment _ _REFLECTION_PROBE_BOX_PROJECTION
            #pragma multi_compile_fragment _ _SHADOWS_SOFT _SHADOWS_SOFT_LOW _SHADOWS_SOFT_MEDIUM _SHADOWS_SOFT_HIGH
            #pragma multi_compile_fragment _ _SCREEN_SPACE_OCCLUSION
            #pragma multi_compile_fragment _ _DBUFFER_MRT1 _DBUFFER_MRT2 _DBUFFER_MRT3
            #pragma multi_compile_fragment _ _LIGHT_COOKIES
            #pragma multi_compile _ _LIGHT_LAYERS
            #pragma multi_compile _ _FORWARD_PLUS
            #include_with_pragmas "Packages/com.unity.render-pipelines.core/ShaderLibrary/FoveatedRenderingKeywords.hlsl"
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/RenderingLayers.hlsl"


            // -------------------------------------
            // Unity defined keywords
            #pragma multi_compile _ LIGHTMAP_SHADOW_MIXING
            #pragma multi_compile _ SHADOWS_SHADOWMASK
            #pragma multi_compile _ DIRLIGHTMAP_COMBINED
            #pragma multi_compile _ LIGHTMAP_ON
            #pragma multi_compile _ DYNAMICLIGHTMAP_ON
            #pragma multi_compile_fragment _ LOD_FADE_CROSSFADE
            #pragma multi_compile_fog
            #pragma multi_compile_fragment _ DEBUG_DISPLAY

            //--------------------------------------
            // GPU Instancing
            #pragma multi_compile_instancing
            #pragma instancing_options renderinglayer
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/DOTS.hlsl"

            #include "LitInput.hlsl"
            #include "LitForwardPass.hlsl"
            ENDHLSL
        }

        Pass
        {
            Name "ShadowCaster"
            Tags
            {
                "LightMode" = "ShadowCaster"
            }

            // -------------------------------------
            // Render State Commands
            ZWrite On
            ZTest LEqual
            ColorMask 0
            Cull[_Cull]

            HLSLPROGRAM
            #pragma target 2.0

            // -------------------------------------
            // Shader Stages
            #pragma vertex ShadowPassVertex
            #pragma fragment ShadowPassFragment

            // -------------------------------------
            // Material Keywords
            #pragma shader_feature_local _ALPHATEST_ON
            #pragma shader_feature_local_fragment _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A

            //--------------------------------------
            // GPU Instancing
            #pragma multi_compile_instancing
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/DOTS.hlsl"

            // -------------------------------------
            // Universal Pipeline keywords

            // -------------------------------------
            // Unity defined keywords
            #pragma multi_compile_fragment _ LOD_FADE_CROSSFADE

            // This is used during shadow map generation to differentiate between directional and punctual light shadows, as they use different formulas to apply Normal Bias
            #pragma multi_compile_vertex _ _CASTING_PUNCTUAL_LIGHT_SHADOW

            // -------------------------------------
            // Includes
            #include "LitInput.hlsl"
            #include "ShadowCasterPass.hlsl"
            ENDHLSL
        }



        Pass
        {
            Name "DepthOnly"
            Tags
            {
                "LightMode" = "DepthOnly"
            }

            // -------------------------------------
            // Render State Commands
            ZWrite On
            ColorMask R
            Cull[_Cull]

            HLSLPROGRAM
            #pragma target 2.0

            // -------------------------------------
            // Shader Stages
            #pragma vertex DepthOnlyVertex
            #pragma fragment DepthOnlyFragment

            // -------------------------------------
            // Material Keywords
            #pragma shader_feature_local _ALPHATEST_ON
            #pragma shader_feature_local_fragment _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A

            // -------------------------------------
            // Unity defined keywords
            #pragma multi_compile_fragment _ LOD_FADE_CROSSFADE

            //--------------------------------------
            // GPU Instancing
            #pragma multi_compile_instancing
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/DOTS.hlsl"

            // -------------------------------------
            // Includes
            #include "LitInput.hlsl"
            #include "DepthOnlyPass.hlsl"
            ENDHLSL
        }

        // This pass is used when drawing to a _CameraNormalsTexture texture
        Pass
        {
            Name "DepthNormals"
            Tags
            {
                "LightMode" = "DepthNormals"
            }

            // -------------------------------------
            // Render State Commands
            ZWrite On
            Cull[_Cull]

            HLSLPROGRAM
            #pragma target 2.0

            // -------------------------------------
            // Shader Stages
            #pragma vertex DepthNormalsVertex
            #pragma fragment DepthNormalsFragment

            // -------------------------------------
            // Material Keywords
            #pragma shader_feature_local _NORMALMAP
            #pragma shader_feature_local _PARALLAXMAP
            #pragma shader_feature_local _ _DETAIL_MULX2 _DETAIL_SCALED
            #pragma shader_feature_local_fragment _ALPHATEST_ON
            #pragma shader_feature_local_fragment _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A

            // -------------------------------------
            // Unity defined keywords
            #pragma multi_compile_fragment _ LOD_FADE_CROSSFADE

            // -------------------------------------
            // Universal Pipeline keywords
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/RenderingLayers.hlsl"

            //--------------------------------------
            // GPU Instancing
            #pragma multi_compile_instancing
            #include_with_pragmas "Assets/SGSSS/ShaderLibrary/DOTS.hlsl"

            // -------------------------------------
            // Includes
            #include "LitInput.hlsl"
            #include "LitDepthNormalsPass.hlsl"
            ENDHLSL
        }

        // This pass it not used during regular rendering, only for lightmap baking.
        Pass
        {
            Name "Meta"
            Tags
            {
                "LightMode" = "Meta"
            }

            // -------------------------------------
            // Render State Commands
            Cull Off

            HLSLPROGRAM
            #pragma target 2.0

            // -------------------------------------
            // Shader Stages
            #pragma vertex UniversalVertexMeta
            #pragma fragment UniversalFragmentMetaLit

            // -------------------------------------
            // Material Keywords
            #pragma shader_feature_local_fragment _SPECULAR_SETUP
            #pragma shader_feature_local_fragment _EMISSION
            #pragma shader_feature_local_fragment _METALLICSPECGLOSSMAP
            #pragma shader_feature_local_fragment _ALPHATEST_ON
            #pragma shader_feature_local_fragment _ _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A
            #pragma shader_feature_local _ _DETAIL_MULX2 _DETAIL_SCALED
            #pragma shader_feature_local_fragment _SPECGLOSSMAP
            #pragma shader_feature EDITOR_VISUALIZATION

            // -------------------------------------
            // Includes
            #include "LitInput.hlsl"
            #include "LitMetaPass.hlsl"

            ENDHLSL
        }
    }

    FallBack "Hidden/Universal Render Pipeline/FallbackError"
    //CustomEditor "UnityEditor.Rendering.Universal.ShaderGUI.LitShader"
}


```

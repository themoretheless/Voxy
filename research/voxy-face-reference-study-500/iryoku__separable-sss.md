# Facial rendering source scan: iryoku/separable-sss

Repository: https://github.com/iryoku/separable-sss

Pinned source revision: b2174689ab22d90647825fdbada7bbd08e7e4e49

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"sss": ["Demo/Code/SeparableSSS.cpp", "Demo/Code/SeparableSSS.h", "Demo/Shaders/SeparableSSS.fx", "SeparableSSS.h"], "shader": ["Demo/Shaders/Bloom.fx", "Demo/Shaders/DepthOfField.fx", "Demo/Shaders/FilmGrain.fx", "Demo/Shaders/Main.fx", "Demo/Shaders/SMAA.fx", "Demo/Shaders/SMAA.h", "Demo/Shaders/SeparableSSS.fx", "Demo/Shaders/ShadowMap.fx", "Demo/Shaders/SkyDome.fx"]}

## Prior selected-function review (partial)

{"files": ["SeparableSSS.h"], "finding": "RGB diffusion kernels; thickness from shadow depth; transmittance profile distinct from separable blur. Need read remainder of blur implementation."}

This prior partial review may target a different pinned revision; compare hashes before reusing findings.

## Source file: SeparableSSS.h

{
  "path": "SeparableSSS.h",
  "blob_sha": "c3338df3c93f728351b29858ff8865ec8689b181",
  "sha256": "c7d2d84122dd4902df0a10925c58036184eb1af26f78ad2cef0c980c29fe88fd",
  "lines": 474,
  "bytes": 18866,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "subsurface": 1,
    "sss": 109,
    "shader": 5,
    "rig": 9,
    "face": 8,
    "lip": 1
  },
  "url": "https://github.com/iryoku/separable-sss/blob/b2174689ab22d90647825fdbada7bbd08e7e4e49/SeparableSSS.h",
  "function_signature_candidates": [
    {
      "line": 283,
      "text": "float3 SSSSTransmittance("
    },
    {
      "line": 368,
      "text": "void SSSSBlurVS(float4 position,"
    },
    {
      "line": 377,
      "text": "float4 SSSSBlurPS("
    }
  ]
}

```text

/**
 * Copyright (C) 2012 Jorge Jimenez (jorge@iryoku.com)
 * Copyright (C) 2012 Diego Gutierrez (diegog@unizar.es)
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 * 
 *    1. Redistributions of source code must retain the above copyright notice,
 *       this list of conditions and the following disclaimer.
 *
 *    2. Redistributions in binary form must reproduce the following disclaimer
 *       in the documentation and/or other materials provided with the 
 *       distribution:
 *
 *       "Uses Separable SSS. Copyright (C) 2012 by Jorge Jimenez and Diego
 *        Gutierrez."
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS 
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, 
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR 
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL COPYRIGHT HOLDERS OR CONTRIBUTORS 
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR 
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF 
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS 
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN 
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) 
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE 
 * POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation are 
 * those of the authors and should not be interpreted as representing official
 * policies, either expressed or implied, of the copyright holders.
 */


/**
 *                  _______      _______      _______      _______
 *                 /       |    /       |    /       |    /       |
 *                |   (----    |   (----    |   (----    |   (----
 *                 \   \        \   \        \   \        \   \
 *              ----)   |    ----)   |    ----)   |    ----)   |
 *             |_______/    |_______/    |_______/    |_______/
 *
 *        S E P A R A B L E   S U B S U R F A C E   S C A T T E R I N G
 *
 *                           http://www.iryoku.com/
 *
 * Hi, thanks for your interest in Separable SSS!
 *
 * It's a simple shader composed of two components:
 *
 * 1) A transmittance function, 'SSSSTransmittance', which allows to calculate
 *    light transmission in thin slabs, useful for ears and nostrils. It should
 *    be applied during the main rendering pass as follows:
 *
 *        float3 t = albedo.rgb * lights[i].color * attenuation * spot;
 *        color.rgb += t * SSSSTransmittance(...)
 *
 *    (See 'Main.fx' for more details).
 *
 * 2) A simple two-pass reflectance post-processing shader, 'SSSSBlur*', which
 *    softens the skin appearance. It should be applied as a regular
 *    post-processing effect like bloom (the usual framebuffer ping-ponging):
 *
 *    a) The first pass (horizontal) must be invoked by taking the final color
 *       framebuffer as input, and storing the results into a temporal
 *       framebuffer.
 *    b) The second pass (vertical) must be invoked by taking the temporal
 *       framebuffer as input, and storing the results into the original final
 *       color framebuffer.
 *
 *    Note that This SSS filter should be applied *before* tonemapping.
 *
 * Before including SeparableSSS.h you'll have to setup the target. The
 * following targets are available:
 *         SMAA_HLSL_3
 *         SMAA_HLSL_4
 *         SMAA_GLSL_3
 *
 * For more information of what's under the hood, you can check the following
 * URLs (but take into account that the shader has evolved a little bit since
 * these publications):
 *
 * 1) Reflectance: http://www.iryoku.com/sssss/
 * 2) Transmittance: http://www.iryoku.com/translucency/
 *
 * If you've got any doubts, just contact us!
 */


//-----------------------------------------------------------------------------
// Configurable Defines

/**
 * SSSS_FOV must be set to the value used to render the scene.
 */
#ifndef SSSS_FOVY
#define SSSS_FOVY 20.0
#endif

/**
 * Light diffusion should occur on the surface of the object, not in a screen 
 * oriented plane. Setting SSSS_FOLLOW_SURFACE to 1 will ensure that diffusion
 * is more accurately calculated, at the expense of more memory accesses.
 */
#ifndef SSSS_FOLLOW_SURFACE
#define SSSS_FOLLOW_SURFACE 0
#endif

/**
 * This define allows to specify a different source for the SSS strength
 * (instead of using the alpha channel of the color framebuffer). This is
 * useful when the alpha channel of the mian color buffer is used for something
 * else.
 */
#ifndef SSSS_STREGTH_SOURCE
#define SSSS_STREGTH_SOURCE (colorM.a)
#endif

/**
 * If SSSS_N_SAMPLES is defined at this point, a custom filter kernel must be
 * set by the runtime.
 */
#ifdef SSSS_N_SAMPLES
/**
 * Filter kernel layout is as follows:
 *   - Weights in the RGB channels.
 *   - Offsets in the A channel.
 */
float4 kernel[SSSS_N_SAMPLES]; 
#else
/**
 * Here you have ready-to-use kernels for quickstarters. Three kernels are 
 * readily available, with varying quality.
 * To create new kernels take a look into SSS::calculateKernel, or simply
 * push CTRL+C in the demo to copy the customized kernel into the clipboard.
 *
 * Note: these preset kernels are not used by the demo. They are calculated on
 * the fly depending on the selected values in the interface, by directly using
 * SSS::calculateKernel.
 *
 * Quality ranges from 0 to 2, being 2 the highest quality available.
 * The quality is with respect to 1080p; for 720p Quality=0 suffices.
 */
#define SSSS_QUALITY 1

#if SSSS_QUALITY == 2
#define SSSS_N_SAMPLES 25
float4 kernel[] = {
    float4(0.530605, 0.613514, 0.739601, 0),
    float4(0.000973794, 1.11862e-005, 9.43437e-007, -3),
    float4(0.00333804, 7.85443e-005, 1.2945e-005, -2.52083),
    float4(0.00500364, 0.00020094, 5.28848e-005, -2.08333),
    float4(0.00700976, 0.00049366, 0.000151938, -1.6875),
    float4(0.0094389, 0.00139119, 0.000416598, -1.33333),
    float4(0.0128496, 0.00356329, 0.00132016, -1.02083),
    float4(0.017924, 0.00711691, 0.00347194, -0.75),
    float4(0.0263642, 0.0119715, 0.00684598, -0.520833),
    float4(0.0410172, 0.0199899, 0.0118481, -0.333333),
    float4(0.0493588, 0.0367726, 0.0219485, -0.1875),
    float4(0.0402784, 0.0657244, 0.04631, -0.0833333),
    float4(0.0211412, 0.0459286, 0.0378196, -0.0208333),
    float4(0.0211412, 0.0459286, 0.0378196, 0.0208333),
    float4(0.0402784, 0.0657244, 0.04631, 0.0833333),
    float4(0.0493588, 0.0367726, 0.0219485, 0.1875),
    float4(0.0410172, 0.0199899, 0.0118481, 0.333333),
    float4(0.0263642, 0.0119715, 0.00684598, 0.520833),
    float4(0.017924, 0.00711691, 0.00347194, 0.75),
    float4(0.0128496, 0.00356329, 0.00132016, 1.02083),
    float4(0.0094389, 0.00139119, 0.000416598, 1.33333),
    float4(0.00700976, 0.00049366, 0.000151938, 1.6875),
    float4(0.00500364, 0.00020094, 5.28848e-005, 2.08333),
    float4(0.00333804, 7.85443e-005, 1.2945e-005, 2.52083),
    float4(0.000973794, 1.11862e-005, 9.43437e-007, 3),
};
#elif SSSS_QUALITY == 1
#define SSSS_N_SAMPLES 17
float4 kernel[] = {
    float4(0.536343, 0.624624, 0.748867, 0),
    float4(0.00317394, 0.000134823, 3.77269e-005, -2),
    float4(0.0100386, 0.000914679, 0.000275702, -1.53125),
    float4(0.0144609, 0.00317269, 0.00106399, -1.125),
    float4(0.0216301, 0.00794618, 0.00376991, -0.78125),
    float4(0.0347317, 0.0151085, 0.00871983, -0.5),
    float4(0.0571056, 0.0287432, 0.0172844, -0.28125),
    float4(0.0582416, 0.0659959, 0.0411329, -0.125),
    float4(0.0324462, 0.0656718, 0.0532821, -0.03125),
    float4(0.0324462, 0.0656718, 0.0532821, 0.03125),
    float4(0.0582416, 0.0659959, 0.0411329, 0.125),
    float4(0.0571056, 0.0287432, 0.0172844, 0.28125),
    float4(0.0347317, 0.0151085, 0.00871983, 0.5),
    float4(0.0216301, 0.00794618, 0.00376991, 0.78125),
    float4(0.0144609, 0.00317269, 0.00106399, 1.125),
    float4(0.0100386, 0.000914679, 0.000275702, 1.53125),
    float4(0.00317394, 0.000134823, 3.77269e-005, 2),
};
#elif SSSS_QUALITY == 0
#define SSSS_N_SAMPLES 11
float4 kernel[] = {
    float4(0.560479, 0.669086, 0.784728, 0),
    float4(0.00471691, 0.000184771, 5.07566e-005, -2),
    float4(0.0192831, 0.00282018, 0.00084214, -1.28),
    float4(0.03639, 0.0130999, 0.00643685, -0.72),
    float4(0.0821904, 0.0358608, 0.0209261, -0.32),
    float4(0.0771802, 0.113491, 0.0793803, -0.08),
    float4(0.0771802, 0.113491, 0.0793803, 0.08),
    float4(0.0821904, 0.0358608, 0.0209261, 0.32),
    float4(0.03639, 0.0130999, 0.00643685, 0.72),
    float4(0.0192831, 0.00282018, 0.00084214, 1.28),
    float4(0.00471691, 0.000184771, 5.07565e-005, 2),
};
#else
#error Quality must be one of {0,1,2}
#endif
#endif

//-----------------------------------------------------------------------------
// Porting Functions

#if SSSS_HLSL_3 == 1
#define SSSSTexture2D sampler2D
#define SSSSSampleLevelZero(tex, coord) tex2Dlod(tex, float4(coord, 0.0, 0.0))
#define SSSSSampleLevelZeroPoint(tex, coord) tex2Dlod(tex, float4(coord, 0.0, 0.0))
#define SSSSSample(tex, coord) tex2D(tex, coord)
#define SSSSSamplePoint(tex, coord) tex2D(tex, coord)
#define SSSSSampleLevelZeroOffset(tex, coord, offset) tex2Dlod(tex, float4(coord + offset * SSSS_PIXEL_SIZE, 0.0, 0.0))
#define SSSSSampleOffset(tex, coord, offset) tex2D(tex, coord + offset * SSSS_PIXEL_SIZE)
#define SSSSLerp(a, b, t) lerp(a, b, t)
#define SSSSSaturate(a) saturate(a)
#define SSSSMad(a, b, c) mad(a, b, c)
#define SSSSMul(v, m) mul(v, m)
#define SSSS_FLATTEN [flatten]
#define SSSS_BRANCH [branch]
#define SSSS_UNROLL [unroll]
#endif
#if SSSS_HLSL_4 == 1
SamplerState LinearSampler { Filter = MIN_MAG_LINEAR_MIP_POINT; AddressU = Clamp; AddressV = Clamp; };
SamplerState PointSampler { Filter = MIN_MAG_MIP_POINT; AddressU = Clamp; AddressV = Clamp; };
#define SSSSTexture2D Texture2D
#define SSSSSampleLevelZero(tex, coord) tex.SampleLevel(LinearSampler, coord, 0)
#define SSSSSampleLevelZeroPoint(tex, coord) tex.SampleLevel(PointSampler, coord, 0)
#define SSSSSample(tex, coord) SSSSSampleLevelZero(tex, coord)
#define SSSSSamplePoint(tex, coord) SSSSSampleLevelZeroPoint(tex, coord)
#define SSSSSampleLevelZeroOffset(tex, coord, offset) tex.SampleLevel(LinearSampler, coord, 0, offset)
#define SSSSSampleOffset(tex, coord, offset) SSSSSampleLevelZeroOffset(tex, coord, offset)
#define SSSSLerp(a, b, t) lerp(a, b, t)
#define SSSSSaturate(a) saturate(a)
#define SSSSMad(a, b, c) mad(a, b, c)
#define SSSSMul(v, m) mul(v, m)
#define SSSS_FLATTEN [flatten]
#define SSSS_BRANCH [branch]
#define SSSS_UNROLL [unroll]
#endif
#if SSSS_GLSL_3 == 1
#define SSSSTexture2D sampler2D
#define SSSSSampleLevelZero(tex, coord) textureLod(tex, coord, 0.0)
#define SSSSSampleLevelZeroPoint(tex, coord) textureLod(tex, coord, 0.0)
#define SSSSSample(tex, coord) texture(tex, coord)
#define SSSSSamplePoint(tex, coord) texture(tex, coord)
#define SSSSSampleLevelZeroOffset(tex, coord, offset) textureLodOffset(tex, coord, 0.0, offset)
#define SSSSSampleOffset(tex, coord, offset) texture(tex, coord, offset)
#define SSSSLerp(a, b, t) mix(a, b, t)
#define SSSSSaturate(a) clamp(a, 0.0, 1.0)
#define SSSSMad(a, b, c) (a * b + c)
#define SSSSMul(v, m) (m * v)
#define SSSS_FLATTEN
#define SSSS_BRANCH
#define SSSS_UNROLL
#define float2 vec2
#define float3 vec3
#define float4 vec4
#define int2 ivec2
#define int3 ivec3
#define int4 ivec4
#define float4x4 mat4x4
#endif


//-----------------------------------------------------------------------------
// Separable SSS Transmittance Function

float3 SSSSTransmittance(
        /**
         * This parameter allows to control the transmittance effect. Its range
         * should be 0..1. Higher values translate to a stronger effect.
         */
        float translucency,

        /**
         * This parameter should be the same as the 'SSSSBlurPS' one. See below
         * for more details.
         */
        float sssWidth,

        /**
         * Position in world space.
         */
        float3 worldPosition,

        /**
         * Normal in world space.
         */
        float3 worldNormal,

        /**
         * Light vector: lightWorldPosition - worldPosition.
         */
        float3 light,

        /**
         * Linear 0..1 shadow map.
         */
        SSSSTexture2D shadowMap,

        /**
         * Regular world to light space matrix.
         */
        float4x4 lightViewProjection,

        /**
         * Far plane distance used in the light projection matrix.
         */
        float lightFarPlane) {
    /**
     * Calculate the scale of the effect.
     */
    float scale = 8.25 * (1.0 - translucency) / sssWidth;
       
    /**
     * First we shrink the position inwards the surface to avoid artifacts:
     * (Note that this can be done once for all the lights)
     */
    float4 shrinkedPos = float4(worldPosition - 0.005 * worldNormal, 1.0);

    /**
     * Now we calculate the thickness from the light point of view:
     */
    float4 shadowPosition = SSSSMul(shrinkedPos, lightViewProjection);
    float d1 = SSSSSample(shadowMap, shadowPosition.xy / shadowPosition.w).r; // 'd1' has a range of 0..1
    float d2 = shadowPosition.z; // 'd2' has a range of 0..'lightFarPlane'
    d1 *= lightFarPlane; // So we scale 'd1' accordingly:
    float d = scale * abs(d1 - d2);

    /**
     * Armed with the thickness, we can now calculate the color by means of the
     * precalculated transmittance profile.
     * (It can be precomputed into a texture, for maximum performance):
     */
    float dd = -d * d;
    float3 profile = float3(0.233, 0.455, 0.649) * exp(dd / 0.0064) +
                     float3(0.1,   0.336, 0.344) * exp(dd / 0.0484) +
                     float3(0.118, 0.198, 0.0)   * exp(dd / 0.187)  +
                     float3(0.113, 0.007, 0.007) * exp(dd / 0.567)  +
                     float3(0.358, 0.004, 0.0)   * exp(dd / 1.99)   +
                     float3(0.078, 0.0,   0.0)   * exp(dd / 7.41);

    /** 
     * Using the profile, we finally approximate the transmitted lighting from
     * the back of the object:
     */
    return profile * SSSSSaturate(0.3 + dot(light, -worldNormal));
}

//-----------------------------------------------------------------------------
// Separable SSS Reflectance Vertex Shader

void SSSSBlurVS(float4 position,
                out float4 svPosition,
                inout float2 texcoord) {
    svPosition = position;
}

//-----------------------------------------------------------------------------
// Separable SSS Reflectance Pixel Shader

float4 SSSSBlurPS(
        /**
         * The usual quad texture coordinates.
         */
        float2 texcoord,

        /**
         * This is a SRGB or HDR color input buffer, which should be the final
         * color frame, resolved in case of using multisampling. The desired
         * SSS strength should be stored in the alpha channel (1 for full
         * strength, 0 for disabling SSS). If this is not possible, you an
         * customize the source of this value using SSSS_STREGTH_SOURCE.
         *
         * When using non-SRGB buffers, you
         * should convert to linear before processing, and back again to gamma
         * space before storing the pixels (see Chapter 24 of GPU Gems 3 for
         * more info)
         *
         * IMPORTANT: WORKING IN A NON-LINEAR SPACE WILL TOTALLY RUIN SSS!
         */
        SSSSTexture2D colorTex,

        /**
         * The linear depth buffer of the scene, resolved in case of using
         * multisampling. The resolve should be a simple average to avoid
         * artifacts in the silhouette of objects.
         */
        SSSSTexture2D depthTex,

        /**
         * This parameter specifies the global level of subsurface scattering
         * or, in other words, the width of the filter. It's specified in
         * world space units.
         */
        float sssWidth,

        /**
         * Direction of the blur:
         *   - First pass:   float2(1.0, 0.0)
         *   - Second pass:  float2(0.0, 1.0)
         */
        float2 dir,

        /**
         * This parameter indicates whether the stencil buffer should be
         * initialized. Should be set to 'true' for the first pass if not
         * previously initialized, to enable optimization of the second
         * pass.
         */
        bool initStencil) {

    // Fetch color of current pixel:
    float4 colorM = SSSSSamplePoint(colorTex, texcoord);

    // Initialize the stencil buffer in case it was not already available:
    if (initStencil) // (Checked in compile time, it's optimized away)
        if (SSSS_STREGTH_SOURCE == 0.0) discard;

    // Fetch linear depth of current pixel:
    float depthM = SSSSSamplePoint(depthTex, texcoord).r;

    // Calculate the sssWidth scale (1.0 for a unit plane sitting on the
    // projection window):
    float distanceToProjectionWindow = 1.0 / tan(0.5 * radians(SSSS_FOVY));
    float scale = distanceToProjectionWindow / depthM;

    // Calculate the final step to fetch the surrounding pixels:
    float2 finalStep = sssWidth * scale * dir;
    finalStep *= SSSS_STREGTH_SOURCE; // Modulate it using the alpha channel.
    finalStep *= 1.0 / 3.0; // Divide by 3 as the kernels range from -3 to 3.

    // Accumulate the center sample:
    float4 colorBlurred = colorM;
    colorBlurred.rgb *= kernel[0].rgb;

    // Accumulate the other samples:
    SSSS_UNROLL
    for (int i = 1; i < SSSS_N_SAMPLES; i++) {
        // Fetch color and depth for current sample:
        float2 offset = texcoord + kernel[i].a * finalStep;
        float4 color = SSSSSample(colorTex, offset);

        #if SSSS_FOLLOW_SURFACE == 1
        // If the difference in depth is huge, we lerp color back to "colorM":
        float depth = SSSSSample(depthTex, offset).r;
        float s = SSSSSaturate(300.0f * distanceToProjectionWindow *
                               sssWidth * abs(depthM - depth));
        color.rgb = SSSSLerp(color.rgb, colorM.rgb, s);
        #endif

        // Accumulate:
        colorBlurred.rgb += kernel[i].rgb * color.rgb;
    }

    return colorBlurred;
}

//-----------------------------------------------------------------------------


```

## Source file: Demo/Code/SeparableSSS.h

{
  "path": "Demo/Code/SeparableSSS.h",
  "blob_sha": "95e8ea93bfc78a04acc2f4464721dcf7bdec21b0",
  "sha256": "cb533dad6ef5a46bd007b1361d419541409eed31d290de60b3fe3705cd7189eb",
  "lines": 187,
  "bytes": 8120,
  "selection_score": 13,
  "keyword_line_counts": {
    "skin": 1,
    "subsurface": 4,
    "sss": 17,
    "shader": 7,
    "rig": 9,
    "face": 5
  },
  "url": "https://github.com/iryoku/separable-sss/blob/b2174689ab22d90647825fdbada7bbd08e7e4e49/Demo/Code/SeparableSSS.h",
  "function_signature_candidates": [
    {
      "line": 122,
      "text": "void go(ID3D10RenderTargetView *mainRTV,"
    },
    {
      "line": 133,
      "text": "void setWidth(float width) { this->sssWidth = width; }"
    },
    {
      "line": 134,
      "text": "float getWidth() const { return sssWidth; }"
    },
    {
      "line": 145,
      "text": "void setStrength(D3DXVECTOR3 strength) { this->strength = strength; calculateKernel(); }"
    },
    {
      "line": 154,
      "text": "void setFalloff(D3DXVECTOR3 falloff) { this->falloff = falloff; calculateKernel(); }"
    },
    {
      "line": 166,
      "text": "void calculateKernel();"
    }
  ]
}

```text

/**
 * Copyright (C) 2012 Jorge Jimenez (jorge@iryoku.com)
 * Copyright (C) 2012 Diego Gutierrez (diegog@unizar.es)
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 * 
 *    1. Redistributions of source code must retain the above copyright notice,
 *       this list of conditions and the following disclaimer.
 *
 *    2. Redistributions in binary form must reproduce the following disclaimer
 *       in the documentation and/or other materials provided with the 
 *       distribution:
 *
 *       "Uses Separable SSS. Copyright (C) 2012 by Jorge Jimenez and Diego
 *        Gutierrez."
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS 
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, 
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR 
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL COPYRIGHT HOLDERS OR CONTRIBUTORS 
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR 
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF 
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS 
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN 
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) 
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE 
 * POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation are 
 * those of the authors and should not be interpreted as representing official
 * policies, either expressed or implied, of the copyright holders.
 */


#ifndef SSS_H
#define SSS_H

#include "RenderTarget.h"
#include <string>
#include <dxgi.h>
#include <d3d10.h>
#include <d3dx10.h>
#include <dxerr.h>

class SeparableSSS {
    public:
        /**
         * width, height: should match the backbuffer's size.
         *
         * fovy: field of view angle used for rendering the scene, in degrees.
         *
         * sssWidth: specifies the global level of subsurface scattering, or in
         *     other words, the width of the filter.
         *
         * nSamples: number of samples of the kernel convolution.
         *
         * followShape: irradiance diffusion should occur on the surface of the
         *     object, not in a screen oriented plane. Setting 'followShape' to
         *     'true' will ensure that diffusion is more accurately calculated,
         *     at the expense of more memory accesses. This is usually only
         *     noticeable under specific lighting configurations.
         *
         * separateStrengthSource: this parameter enables to use a specific
         *     buffer for fetching the SSS strength, instead of using the alpha
         *     channel of the color buffer.
         *
         * stencilInitialized: if an stencil is initialized for selectively
         *     processing the frame buffer, set this to 'true'. For example,
         *     this cannot be easily done, when using multisampling. See
         *     @STENCIL below for more info.
         *
         * format: format of the temporal framebuffer. Should be left as is,
         *     unless an HDR format is desired.
         */
        SeparableSSS(ID3D10Device *device, 
                     int width, int height,
                     float fovy,
                     float sssWidth,
                     int nSamples=17,
                     bool stencilInitialized=true,
                     bool followShape=true,
                     bool separateStrengthSource=false,
                     DXGI_FORMAT format=DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);
        ~SeparableSSS();

        /**
         * IMPORTANT NOTICE: all render targets below must not be multisampled.
         *
         * mainRTV and mainSRV: render target and shader resources of the
         *     final rendered image. The filter works on place, so they should
         *     be associated to the same resource. The SSS intensity should be
         *     stored in the alpha channel.
         *
         * depthSRV: shader resource of the the linear depth buffer of the
         *     scene, resolved in case of using multisampling. The resolve
         *     should be a simple average to avoid artifacts in the silhouette
         *     of objects.
         *
         * @STENCIL
         * depthDSV: two possibilities over here:
         *     a) if 'stencilInitialized' was set to 'true', then only the
         *        pixels marked in the stencil buffer with the value 'id' will
         *        be processed in both the horizontal and vertical passes.
         *     b) if 'stencilInitialized' was set to 'false' (for example when
         *        using multisampling), then it will be initialized on the fly
         *        for optimizing the second vertical pass.
         *
         * stregthSRV: if 'separateStrengthSource' was set to 'true' when
         *     creating this object, the SSS stregth will be fetched from the
         *     alpha channel of this texture instead of the alpha channel of
         *     the color buffer.
         *
         * id: stencil value used to render the objects we must apply
         *     subsurface scattering on. Should be left as is in case
         *     'stencilInitialized' was set to false when creating the object.
         *
         * Note that 'depthSRV' and 'depthDSV' cannot be the views of the same
         * depth-stencil buffer.
         */
        void go(ID3D10RenderTargetView *mainRTV,
                ID3D10ShaderResourceView *mainSRV,
                ID3D10ShaderResourceView *depthSRV,
                ID3D10DepthStencilView *depthDSV,
                ID3D10ShaderResourceView *stregthSRV = NULL,
                int id=1);

        /**
         * This parameter specifies the global level of subsurface scattering,
         * or in other words, the width of the filter.
         */
        void setWidth(float width) { this->sssWidth = width; }
        float getWidth() const { return sssWidth; }

        /**
         * @STRENGTH
         *
         * This parameter specifies the how much of the diffuse light gets into
         * the skin, and thus gets modified by the SSS mechanism.
         *
         * It can be seen as a per-channel mix factor between the original
         * image, and the SSS-filtered image.
         */
        void setStrength(D3DXVECTOR3 strength) { this->strength = strength; calculateKernel(); }
        D3DXVECTOR3 getStrength() const { return strength; }

        /**
         * This parameter defines the per-channel falloff of the gradients
         * produced by the subsurface scattering events.
         *
         * It can be used to fine tune the color of the gradients.
         */
        void setFalloff(D3DXVECTOR3 falloff) { this->falloff = falloff; calculateKernel(); }
        D3DXVECTOR3 getFalloff() const { return falloff; }

        /**
         * This one is just for convenience, it returns the shader code of
         * current kernel.
         */
        std::string getKernelCode() const;

    private:
        D3DXVECTOR3 gaussian(float variance, float r);
        D3DXVECTOR3 profile(float r);
        void calculateKernel();

        ID3D10Device *device;

        float sssWidth;
        int nSamples;
        bool stencilInitialized;
        D3DXVECTOR3 strength;
        D3DXVECTOR3 falloff;

        ID3D10Effect *effect;
        RenderTarget *tmpRT;
        Quad *quad;

        std::vector<D3DXVECTOR4> kernel;
        ID3D10EffectScalarVariable *idVariable, *sssWidthVariable;
        ID3D10EffectVectorVariable *kernelVariable;
        ID3D10EffectShaderResourceVariable *colorTexVariable, *depthTexVariable, *strengthTexVariable;
        ID3D10EffectTechnique *technique;
};

#endif


```

## Source file: Demo/Code/SeparableSSS.cpp

{
  "path": "Demo/Code/SeparableSSS.cpp",
  "blob_sha": "cdd2478f7b7d5a2d8ab7246ddaa33fa2ba8ffae0",
  "sha256": "d80ad1b6b3e8f901ada1aa89057d5bdb17b1a6d102235a00573c432aa808ee9e",
  "lines": 327,
  "bytes": 12433,
  "selection_score": 13,
  "keyword_line_counts": {
    "skin": 1,
    "sss": 23,
    "shader": 14,
    "rig": 9,
    "face": 3
  },
  "url": "https://github.com/iryoku/separable-sss/blob/b2174689ab22d90647825fdbada7bbd08e7e4e49/Demo/Code/SeparableSSS.cpp",
  "function_signature_candidates": []
}

```text

/**
 * Copyright (C) 2012 Jorge Jimenez (jorge@iryoku.com)
 * Copyright (C) 2012 Diego Gutierrez (diegog@unizar.es)
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 * 
 *    1. Redistributions of source code must retain the above copyright notice,
 *       this list of conditions and the following disclaimer.
 *
 *    2. Redistributions in binary form must reproduce the following disclaimer
 *       in the documentation and/or other materials provided with the 
 *       distribution:
 *
 *       "Uses Separable SSS. Copyright (C) 2012 by Jorge Jimenez and Diego
 *        Gutierrez."
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS 
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, 
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR 
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL COPYRIGHT HOLDERS OR CONTRIBUTORS 
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR 
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF 
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS 
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN 
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) 
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE 
 * POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation are 
 * those of the authors and should not be interpreted as representing official
 * policies, either expressed or implied, of the copyright holders.
 */


#include <sstream>
#include "SeparableSSS.h"
using namespace std;


#pragma region Useful Macros from DXUT (copy-pasted here as we prefer this to be as self-contained as possible)
#if defined(DEBUG) || defined(_DEBUG)
#ifndef V
#define V(x) { hr = (x); if (FAILED(hr)) { DXTrace(__FILE__, (DWORD)__LINE__, hr, L#x, true); } }
#endif
#ifndef V_RETURN
#define V_RETURN(x) { hr = (x); if (FAILED(hr)) { return DXTrace(__FILE__, (DWORD)__LINE__, hr, L#x, true); } }
#endif
#else
#ifndef V
#define V(x) { hr = (x); }
#endif
#ifndef V_RETURN
#define V_RETURN(x) { hr = (x); if( FAILED(hr) ) { return hr; } }
#endif
#endif

#ifndef SAFE_DELETE
#define SAFE_DELETE(p) { if (p) { delete (p); (p) = NULL; } }
#endif
#ifndef SAFE_DELETE_ARRAY
#define SAFE_DELETE_ARRAY(p) { if (p) { delete[] (p); (p) = NULL; } }
#endif
#ifndef SAFE_RELEASE
#define SAFE_RELEASE(p) { if (p) { (p)->Release(); (p) = NULL; } }
#endif
#pragma endregion

#pragma region This stuff is for loading headers from resources
class ID3D10IncludeResource : public ID3D10Include {
    public:
        STDMETHOD(Open)(THIS_ D3D_INCLUDE_TYPE, LPCSTR pFileName, LPCVOID, LPCVOID *ppData, UINT *pBytes)  {
            wstringstream s;
            s << pFileName;
            HRSRC src = FindResource(GetModuleHandle(NULL), s.str().c_str(), RT_RCDATA);
            HGLOBAL res = LoadResource(GetModuleHandle(NULL), src);

            *pBytes = SizeofResource(GetModuleHandle(NULL), src);
            *ppData = (LPCVOID) LockResource(res);

            return S_OK;
        }

        STDMETHOD(Close)(THIS_ LPCVOID)  {
            return S_OK;
        }
};
#pragma endregion


SeparableSSS::SeparableSSS(ID3D10Device *device,
                           int width,
                           int height,
                           float fovy,
                           float sssWidth,
                           int nSamples,
                           bool stencilInitialized,
                           bool followSurface,
                           bool separateStrengthSource,
                           DXGI_FORMAT format) : device(device),
                           sssWidth(sssWidth),
                           nSamples(nSamples),
                           stencilInitialized(stencilInitialized),
                           strength(D3DXVECTOR3(0.48f, 0.41f, 0.28f)),
                           falloff(D3DXVECTOR3(1.0f, 0.37f, 0.3f)) {
    HRESULT hr;

    // Setup the defines for compiling the effect:
    vector<D3D10_SHADER_MACRO> defines;

    stringstream s;

    s << fovy;
    string fovyText = s.str();
    D3D10_SHADER_MACRO fovyMacro = { "SSSS_FOVY", fovyText.c_str() };
    defines.push_back(fovyMacro);

    s.str("");
    s << nSamples;
    string nSamplesText = s.str();
    D3D10_SHADER_MACRO nSamplesMacro = { "SSSS_N_SAMPLES", nSamplesText.c_str() };
    defines.push_back(nSamplesMacro);

    D3D10_SHADER_MACRO followSurfaceMacro = { "SSSS_FOLLOW_SURFACE", followSurface? "1" : "0" };
    defines.push_back(followSurfaceMacro);

    D3D10_SHADER_MACRO stencilInitializedMacro = { "STENCIL_INITIALIZED", stencilInitialized? "1" : "0" };
    defines.push_back(stencilInitializedMacro);

    if (separateStrengthSource) {
        D3D10_SHADER_MACRO strengthSourceMacro = { "SSSS_STREGTH_SOURCE", "(SSSSSample(strengthTex, texcoord).a)"};
        defines.push_back(strengthSourceMacro);
    }

    D3D10_SHADER_MACRO null = { NULL, NULL };
    defines.push_back(null);

    // Now, let's load the effect:
    ID3D10IncludeResource includeResource;
    V(D3DX10CreateEffectFromResource(GetModuleHandle(NULL), L"SeparableSSS.fx", NULL, &defines.front(), &includeResource, "fx_4_0", D3D10_SHADER_ENABLE_STRICTNESS | D3D10_SHADER_OPTIMIZATION_LEVEL3, 0, device, NULL, NULL, &effect, NULL, NULL));

    // Setup the fullscreen quad:
    D3D10_PASS_DESC desc;
    V(effect->GetTechniqueByName("SSS")->GetPassByName("SSSSBlurX")->GetDesc(&desc));
    quad = new Quad(device, desc);

    // Create the temporal render target:
    tmpRT = new RenderTarget(device, width, height, format);

    // Create some handles for techniques and variables:
    technique = effect->GetTechniqueByName("SSS");
    idVariable = effect->GetVariableByName("id")->AsScalar();
    sssWidthVariable = effect->GetVariableByName("sssWidth")->AsScalar();
    kernelVariable = effect->GetVariableByName("kernel")->AsVector();
    colorTexVariable = effect->GetVariableByName("colorTex")->AsShaderResource();
    depthTexVariable = effect->GetVariableByName("depthTex")->AsShaderResource();
    strengthTexVariable = effect->GetVariableByName("strengthTex")->AsShaderResource();

    // And finally, calculate the sample positions and weights:
    calculateKernel();
}


SeparableSSS::~SeparableSSS() {
    SAFE_DELETE(tmpRT);
    SAFE_RELEASE(effect);
    SAFE_DELETE(quad);
}


D3DXVECTOR3 SeparableSSS::gaussian(float variance, float r) {
    /**
     * We use a falloff to modulate the shape of the profile. Big falloffs
     * spreads the shape making it wider, while small falloffs make it
     * narrower.
     */
    D3DXVECTOR3 g;
    for (int i = 0; i < 3; i++) {
        float rr = r / (0.001f + falloff[i]);
        g[i] = exp((-(rr * rr)) / (2.0f * variance)) / (2.0f * 3.14f * variance);
    }
    return g;
}


D3DXVECTOR3 SeparableSSS::profile(float r) {
    /**
     * We used the red channel of the original skin profile defined in
     * [d'Eon07] for all three channels. We noticed it can be used for green
     * and blue channels (scaled using the falloff parameter) without
     * introducing noticeable differences and allowing for total control over
     * the profile. For example, it allows to create blue SSS gradients, which
     * could be useful in case of rendering blue creatures.
     */
    return  // 0.233f * gaussian(0.0064f, r) + /* We consider this one to be directly bounced light, accounted by the strength parameter (see @STRENGTH) */
               0.100f * gaussian(0.0484f, r) +
               0.118f * gaussian( 0.187f, r) +
               0.113f * gaussian( 0.567f, r) +
               0.358f * gaussian(  1.99f, r) +
               0.078f * gaussian(  7.41f, r);
} 


void SeparableSSS::calculateKernel() {
    HRESULT hr;

    const float RANGE = nSamples > 20? 3.0f : 2.0f;
    const float EXPONENT = 2.0f;

    kernel.resize(nSamples);

    // Calculate the offsets:
    float step = 2.0f * RANGE / (nSamples - 1);
    for (int i = 0; i < nSamples; i++) {
        float o = -RANGE + float(i) * step;
        float sign = o < 0.0f? -1.0f : 1.0f;
        kernel[i].w = RANGE * sign * abs(pow(o, EXPONENT)) / pow(RANGE, EXPONENT);
    }

    // Calculate the weights:
    for (int i = 0; i < nSamples; i++) {
        float w0 = i > 0? abs(kernel[i].w - kernel[i - 1].w) : 0.0f;
        float w1 = i < nSamples - 1? abs(kernel[i].w - kernel[i + 1].w) : 0.0f;
        float area = (w0 + w1) / 2.0f;
        D3DXVECTOR3 t = area * profile(kernel[i].w);
        kernel[i].x = t.x;
        kernel[i].y = t.y;
        kernel[i].z = t.z;
    }

    // We want the offset 0.0 to come first:
    D3DXVECTOR4 t = kernel[nSamples / 2];
    for (int i = nSamples / 2; i > 0; i--)
        kernel[i] = kernel[i - 1];
    kernel[0] = t;
    
    // Calculate the sum of the weights, we will need to normalize them below:
    D3DXVECTOR3 sum = D3DXVECTOR3(0.0f, 0.0f, 0.0f);
    for (int i = 0; i < nSamples; i++)
        sum += D3DXVECTOR3(kernel[i]);

    // Normalize the weights:
    for (int i = 0; i < nSamples; i++) {
        kernel[i].x /= sum.x;
        kernel[i].y /= sum.y;
        kernel[i].z /= sum.z;
    }

    // Tweak them using the desired strength. The first one is:
    //     lerp(1.0, kernel[0].rgb, strength)
    kernel[0].x = (1.0f - strength.x) * 1.0f + strength.x * kernel[0].x;
    kernel[0].y = (1.0f - strength.y) * 1.0f + strength.y * kernel[0].y;
    kernel[0].z = (1.0f - strength.z) * 1.0f + strength.z * kernel[0].z;

    // The others:
    //     lerp(0.0, kernel[0].rgb, strength)
    for (int i = 1; i < nSamples; i++) {
        kernel[i].x *= strength.x;
        kernel[i].y *= strength.y;
        kernel[i].z *= strength.z;
    }

    // Finally, set 'em!
    V(kernelVariable->SetFloatVectorArray((float *) &kernel.front(), 0, nSamples));
}


void SeparableSSS::go(ID3D10RenderTargetView *mainRTV,
                      ID3D10ShaderResourceView *mainSRV,
                      ID3D10ShaderResourceView *depthSRV,
                      ID3D10DepthStencilView *depthDSV,
                      ID3D10ShaderResourceView *stregthSRV,
                      int id) {
    HRESULT hr;

    // Save the state:
    SaveViewportsScope saveViewport(device);
    SaveRenderTargetsScope saveRenderTargets(device);
    SaveInputLayoutScope saveInputLayout(device);

    // Clear the temporal render target:
    float clearColor[4] = { 0.0f, 0.0f, 0.0f, 0.0f };
    device->ClearRenderTargetView(*tmpRT, clearColor);

    // Clear the stencil buffer if it was not available, and thus one must be 
    // initialized on the fly:
    if (!stencilInitialized)
        device->ClearDepthStencilView(depthDSV, D3D10_CLEAR_STENCIL, 1.0, 0);

    // Set variables:
    V(idVariable->SetInt(id));
    V(sssWidthVariable->SetFloat(sssWidth));
    V(depthTexVariable->SetResource(depthSRV));
    V(strengthTexVariable->SetResource(stregthSRV));

    // Set input layout and viewport:
    quad->setInputLayout();
    tmpRT->setViewport();

    // Run the horizontal pass:
    V(colorTexVariable->SetResource(mainSRV));
    technique->GetPassByName("SSSSBlurX")->Apply(0);
    device->OMSetRenderTargets(1, *tmpRT, depthDSV);
    quad->draw();
    device->OMSetRenderTargets(0, NULL, NULL);

    // And finish with the vertical one:
    V(colorTexVariable->SetResource(*tmpRT));
    technique->GetPassByName("SSSSBlurY")->Apply(0);
    device->OMSetRenderTargets(1, &mainRTV, depthDSV);
    quad->draw();
    device->OMSetRenderTargets(0, NULL, NULL);
}


string SeparableSSS::getKernelCode() const {
    stringstream s;
    s << "float4 kernel[] = {" << "\r\n";
    for (int i = 0; i < nSamples; i++)
        s << "    float4(" << kernel[i].x << ", "
                           << kernel[i].y << ", "
                           << kernel[i].z << ", "
                           << kernel[i].w << ")," << "\r\n";
    s << "};" << "\r\n";
    return s.str();
}


```

## Source file: Demo/Code/Demo.cpp

{
  "path": "Demo/Code/Demo.cpp",
  "blob_sha": "e4a3cc4b4d14c2bb10070ff5bcfb4ba7c841f98f",
  "sha256": "08db238b2254e339177061aa1070bcb746d0bd70eb6bc7fe8d8d95d4a16431f7",
  "lines": 1742,
  "bytes": 68972,
  "selection_score": -2,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 1,
    "subsurface": 1,
    "sss": 46,
    "shader": 25,
    "rig": 7,
    "face": 14,
    "lip": 4
  },
  "url": "https://github.com/iryoku/separable-sss/blob/b2174689ab22d90647825fdbada7bbd08e7e4e49/Demo/Code/Demo.cpp",
  "function_signature_candidates": [
    {
      "line": 267,
      "text": "void shadowPass(ID3D10Device *device) {"
    },
    {
      "line": 293,
      "text": "bool isMSAA() {"
    },
    {
      "line": 301,
      "text": "void mainPass(ID3D10Device *device) {"
    },
    {
      "line": 405,
      "text": "void addSpecular(ID3D10Device *device, RenderTarget *dst) {"
    },
    {
      "line": 416,
      "text": "void smaaPass(ID3D10Device *device) {"
    },
    {
      "line": 433,
      "text": "void renderText() {"
    },
    {
      "line": 466,
      "text": "void renderScene(ID3D10Device *device, double time, float elapsedTime) {"
    },
    {
      "line": 505,
      "text": "void updateLightControls(int i) {"
    },
    {
      "line": 513,
      "text": "float updateSlider(CDXUTDialog &hud, UINT slider, UINT label, float scale, wstring text) {"
    },
    {
      "line": 526,
      "text": "void loadShot(istream &f) {"
    },
    {
      "line": 570,
      "text": "void loadPreset(int i) {"
    },
    {
      "line": 582,
      "text": "void setExposure(float value) {"
    },
    {
      "line": 588,
      "text": "void correct(float amount) {"
    },
    {
      "line": 595,
      "text": "bool setupIntro(float t) {"
    },
    {
      "line": 736,
      "text": "void reset() {"
    },
    {
      "line": 850,
      "text": "void setupSSS(ID3D10Device *device, const DXGI_SURFACE_DESC *desc) {"
    },
    {
      "line": 1014,
      "text": "void setupMainEffect() {"
    },
    {
      "line": 1366,
      "text": "void loadMesh(CDXUTSDKMesh &mesh, ID3D10Device *device, const wstring &name, const wstring &path) {"
    },
    {
      "line": 1549,
      "text": "void releaseAudio() {"
    },
    {
      "line": 1558,
      "text": "void initApp() {"
    }
  ]
}

```text

/**
 * Copyright (C) 2012 Jorge Jimenez (jorge@iryoku.com). All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 *    1. Redistributions of source code must retain the above copyright notice,
 *       this list of conditions and the following disclaimer.
 *
 *    2. Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in the
 *       documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL COPYRIGHT HOLDERS OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation are 
 * those of the authors and should not be interpreted as representing official
 * policies, either expressed or implied, of the copyright holders.
 */


#include "DXUT.h"
#include "DXUTgui.h"
#include "DXUTsettingsDlg.h"
#include "SDKmisc.h"
#include "SDKmesh.h"
#include "SDKwavefile.h"
#include <xaudio2.h>

#include <fstream>
#include <sstream>
#include <iomanip>
#include <limits>

#include "Timer.h"
#include "Camera.h"
#include "RenderTarget.h"
#include "ShadowMap.h"
#include "Animation.h"
#include "Fade.h"
#include "SplashScreen.h"
#include "SeparableSSS.h"
#include "Bloom.h"
#include "FilmGrain.h"
#include "DepthOfField.h"
#include "SkyDome.h"
#include "SMAA.h"

using namespace std;

/**
 * This allows to enable/disable the splash screen and the intro.
 */
#define INTRO_BUILD 1

#pragma region Global Stuff
CDXUTDialogResourceManager dialogResourceManager;
CDXUTDialog mainHud, secondaryHud, helpHud;

IXAudio2 *xaudio;
IXAudio2MasteringVoice *masteringVoice;
IXAudio2SourceVoice *sourceVoice;
vector<BYTE> waveData;
vector<UINT32> dpdsData;
bool audioEnabled = true;
bool audioStarted = false;

Timer *timer;
ID3DX10Font *font;
ID3DX10Sprite *sprite;
CDXUTTextHelper *txtHelper;

CDXUTSDKMesh mesh;
ID3D10ShaderResourceView *specularAOSRV;
ID3D10ShaderResourceView *beckmannSRV;
ID3D10ShaderResourceView *irradianceSRV[3];
ID3D10Effect *mainEffect;
ID3D10InputLayout *vertexLayout;

Camera camera;
SplashScreen *splashScreen;
SkyDome *skyDome[3];

int scene = 0;
float sceneTime = 0.0f;
int currentSkyDome = 0;

float lastTimeMeasured = 0.0f;
int nFrames = 0;
int nFramesInverval = 0;
int minFrames = numeric_limits<int>::max(), maxFrames = numeric_limits<int>::min();
float introTime = 0.0f;

BackbufferRenderTarget *backbufferRT;
RenderTarget *mainRTMS;
RenderTarget *mainRT;
RenderTarget *tmpRT_SRGB;
RenderTarget *tmpRT;
RenderTarget *finalRT[2];
RenderTarget *specularsRTMS;
RenderTarget *specularsRT;
RenderTarget *depthRTMS;
RenderTarget *depthRT;
RenderTarget *velocityRTMS;
RenderTarget *velocityRT;
DepthStencil *depthStencilMS;
DepthStencil *depthStencil;

SeparableSSS *separableSSS;
SMAA *smaa;
Bloom *bloom;
FilmGrain *filmGrain;
DepthOfField *dof;
Quad *quad;


D3DXMATRIX prevViewProj, currViewProj;
int subsampleIndex = 0;

struct MSAAMode {
    wstring name;
    DXGI_SAMPLE_DESC desc;
};
MSAAMode msaaModes[] = {
    {L"MSAA 1x",   {1,  0}},
    {L"MSAA 2x",   {2,  0}},
    {L"MSAA 4x",   {4,  0}},
    {L"CSAA 8x",   {4,  8}},
    {L"CSAA 8xQ ", {8,  8}},
    {L"CSAA 16x",  {4, 16}},
    {L"CSAA 16xQ", {8, 16}}
};
vector<MSAAMode> supportedMsaaModes;
int antialiasingMode = int(SMAA::MODE_SMAA_T2X);


double t0 = numeric_limits<double>::min();
double tFade = numeric_limits<double>::max();
bool skipIntro = false;

bool showHud = true;
bool showHelp = false;
bool loaded = false;


const int N_LIGHTS = 5;
const int N_HEADS = 1;
const int SHADOW_MAP_SIZE = 2048;
const int HUD_WIDTH = 140;
const int HUD_WIDTH2 = 100;
const float CAMERA_FOV = 20.0f;


struct Light {
    Camera camera;
    float fov;
    float falloffWidth;
    D3DXVECTOR3 color;
    float attenuation;
    float farPlane;
    float bias;
    ShadowMap *shadowMap;
};
Light lights[N_LIGHTS];


enum Object { OBJECT_CAMERA, OBJECT_LIGHT1, OBJECT_LIGHT2, OBJECT_LIGHT3, OBJECT_LIGHT4, OBJECT_LIGHT5 };
Object object = OBJECT_CAMERA;


enum State { STATE_SPLASH_INTRO, STATE_INTRO, STATE_SPLASH_OUTRO, STATE_RUNNING };
#if INTRO_BUILD == 1
State state = STATE_SPLASH_INTRO;
#else
State state = STATE_RUNNING;
#endif
#pragma endregion

#pragma region IDC Macros
#define IDC_SSS_HELP              1
#define IDC_HELP_TEXT             2
#define IDC_HELP_CLOSE_BUTTON     3
#define IDC_TOGGLEFULLSCREEN      4
#define IDC_MSAA                  5
#define IDC_HDR                   6
#define IDC_SEPARATE_SPECULARS    7
#define IDC_PROFILE               8
#define IDC_SSS                   9
#define IDC_FOLLOW_SURFACE       10
#define IDC_NSAMPLES_LABEL       11
#define IDC_NSAMPLES             12
#define IDC_SSSWIDTH_LABEL       13
#define IDC_SSSWIDTH             14
#define IDC_STRENGTH_LABEL       15
#define IDC_STRENGTH_R_LABEL     16
#define IDC_STRENGTH_R           17
#define IDC_STRENGTH_G_LABEL     18
#define IDC_STRENGTH_G           19
#define IDC_STRENGTH_B_LABEL     20
#define IDC_STRENGTH_B           21
#define IDC_FALLOFF_LABEL        22
#define IDC_FALLOFF_R_LABEL      23
#define IDC_FALLOFF_R            24
#define IDC_FALLOFF_G_LABEL      25
#define IDC_FALLOFF_G            26
#define IDC_FALLOFF_B_LABEL      27
#define IDC_FALLOFF_B            28
#define IDC_TRANSLUCENCY_LABEL   29
#define IDC_TRANSLUCENCY         30
#define IDC_SPEC_INTENSITY_LABEL 31
#define IDC_SPEC_INTENSITY       32
#define IDC_SPEC_ROUGHNESS_LABEL 33
#define IDC_SPEC_ROUGHNESS       34
#define IDC_SPEC_FRESNEL_LABEL   35
#define IDC_SPEC_FRESNEL         36
#define IDC_BUMPINESS_LABEL      37
#define IDC_BUMPINESS            38
#define IDC_EXPOSURE_LABEL       39
#define IDC_EXPOSURE             40
#define IDC_ENVMAP_LABEL         41
#define IDC_ENVMAP               42
#define IDC_AMBIENT              43
#define IDC_AMBIENT_LABEL        44
#define IDC_FOCUS_DISTANCE       45
#define IDC_FOCUS_DISTANCE_LABEL 46
#define IDC_FOCUS_RANGE          47
#define IDC_FOCUS_RANGE_LABEL    48
#define IDC_FOCUS_FALLOFF        49
#define IDC_FOCUS_FALLOFF_LABEL  50
#define IDC_CAMERA_BUTTON        51
#define IDC_LIGHT1_BUTTON        52
#define IDC_LIGHT1_LABEL         (53 + 5)
#define IDC_LIGHT1               (54 + 5 * 2)
#pragma endregion


class ID3D10IncludeResource : public ID3D10Include {
    public:
        STDMETHOD(Open)(THIS_ D3D_INCLUDE_TYPE, LPCSTR pFileName, LPCVOID, LPCVOID *ppData, UINT *pBytes)  {
            wstringstream s;
            s << pFileName;
            HRSRC src = FindResource(GetModuleHandle(NULL), s.str().c_str(), RT_RCDATA);
            HGLOBAL res = LoadResource(GetModuleHandle(NULL), src);

            *pBytes = SizeofResource(GetModuleHandle(NULL), src);
            *ppData = (LPCVOID) LockResource(res);

            return S_OK;
        }

        STDMETHOD(Close)(THIS_ LPCVOID)  {
            return S_OK;
        }
};


void shadowPass(ID3D10Device *device) {
    for (int i = 0; i < N_LIGHTS; i++) {
        if (D3DXVec3Length(&lights[i].color) > 0.0f) {
            lights[i].shadowMap->begin(lights[i].camera.getViewMatrix(), lights[i].camera.getProjectionMatrix());
            for (int j = 0; j < N_HEADS; j++) {
                D3DXMATRIX world;
                D3DXMatrixTranslation(&world, j - (N_HEADS - 1) / 2.0f, 0.0f, 0.0f);

                lights[i].shadowMap->setWorldMatrix((float*) world);

                mesh.Render(device, lights[i].shadowMap->getTechnique());
            }
            lights[i].shadowMap->end();
        }
    }
}


MSAAMode currentMode() {
    if (antialiasingMode < 10)
        return supportedMsaaModes[0];
    else
        return supportedMsaaModes[antialiasingMode - 10];
}


bool isMSAA() {
    if (antialiasingMode < 10)
        return false;
    else
        return currentMode().desc.Count > 1;
}


void mainPass(ID3D10Device *device) {
    HRESULT hr;

    // Jitter setup:
    D3DXVECTOR2 jitters[] = {
        D3DXVECTOR2( 0.25f, -0.25f),
        D3DXVECTOR2(-0.25f,  0.25f)
    };
    const DXGI_SURFACE_DESC *desc = DXUTGetDXGIBackBufferSurfaceDesc();
    D3DXVECTOR2 jitterProjectionSpace = 2.0f * jitters[subsampleIndex];
    jitterProjectionSpace.x /= float(desc->Width); jitterProjectionSpace.y /= float(desc->Height);
    if (SMAA::Mode(antialiasingMode) == SMAA::MODE_SMAA_T2X) {
        V(mainEffect->GetVariableByName("jitter")->AsVector()->SetFloatVector((float *) jitterProjectionSpace));
    } else {
        V(mainEffect->GetVariableByName("jitter")->AsVector()->SetFloatVector((float *) D3DXVECTOR2(0.0f, 0.0f)));
    }

    // Calculate current view-projection matrix:
    currViewProj = camera.getViewMatrix() * camera.getProjectionMatrix();

    // Variables setup:
    V(mainEffect->GetVariableByName("cameraPosition")->AsVector()->SetFloatVector((float *) (D3DXVECTOR3) camera.getEyePosition()));

    for (int i = 0; i < N_LIGHTS; i++) {
        D3DXVECTOR3 t = lights[i].camera.getLookAtPosition() - lights[i].camera.getEyePosition();
        D3DXVECTOR3 dir;
        D3DXVec3Normalize(&dir, &t);

        D3DXVECTOR3 pos = lights[i].camera.getEyePosition();

        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("position")->AsVector()->SetFloatVector((float *) pos));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("direction")->AsVector()->SetFloatVector((float *) dir));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("falloffStart")->AsScalar()->SetFloat(cos(0.5f * lights[i].fov)));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("falloffWidth")->AsScalar()->SetFloat(lights[i].falloffWidth));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("color")->AsVector()->SetFloatVector((float *) lights[i].color));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("attenuation")->AsScalar()->SetFloat(lights[i].attenuation));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("farPlane")->AsScalar()->SetFloat(lights[i].farPlane));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("bias")->AsScalar()->SetFloat(lights[i].bias));
        V(mainEffect->GetVariableByName("lights")->GetElement(i)->GetMemberByName("viewProjection")->AsMatrix()->SetMatrix(ShadowMap::getViewProjectionTextureMatrix(lights[i].camera.getViewMatrix(), lights[i].camera.getProjectionMatrix()))); 
        V(mainEffect->GetVariableByName("shadowMaps")->GetElement(i)->AsShaderResource()->SetResource(*lights[i].shadowMap));
    }
    mainEffect->GetTechniqueByName("Render")->GetPassByIndex(0)->Apply(0); // Just to clear some runtime warnings.

     // Render target setup:
    float clearColor[4] = { 0.0f, 0.0f, 0.0f, 0.0f };
    if (isMSAA()) {
        device->ClearDepthStencilView(*depthStencilMS, D3D10_CLEAR_DEPTH | D3D10_CLEAR_STENCIL, 1.0, 0);
        device->ClearRenderTargetView(*mainRTMS, clearColor);
        device->ClearRenderTargetView(*depthRTMS, clearColor);
        device->ClearRenderTargetView(*specularsRTMS, clearColor);

        ID3D10RenderTargetView *rt[] = { *mainRTMS, *depthRTMS, *velocityRTMS, *specularsRTMS };
        device->OMSetRenderTargets(4, rt, *depthStencilMS);
    } else {
        device->ClearDepthStencilView(*depthStencil, D3D10_CLEAR_DEPTH | D3D10_CLEAR_STENCIL, 1.0, 0);
        device->ClearRenderTargetView(*mainRT, clearColor);
        device->ClearRenderTargetView(*depthRT, clearColor);
        device->ClearRenderTargetView(*specularsRT, clearColor);
        device->ClearRenderTargetView(*velocityRT, clearColor);

        ID3D10RenderTargetView *rt[] = { *mainRT, *depthRT, *velocityRT, *specularsRT };
        device->OMSetRenderTargets(4, rt, *depthStencil);
    }

    // Heads rendering:
    device->IASetInputLayout(vertexLayout);

    for (int i = 0; i < N_HEADS; i++) {
        D3DXMATRIX world;
        D3DXMatrixTranslation(&world, i - (N_HEADS - 1) / 2.0f, 0.0f, 0.0f);

        D3DXMATRIX currWorldViewProj = world * currViewProj;
        D3DXMATRIX prevWorldViewProj = world * prevViewProj;

        V(mainEffect->GetVariableByName("currWorldViewProj")->AsMatrix()->SetMatrix((float *) currWorldViewProj));
        V(mainEffect->GetVariableByName("prevWorldViewProj")->AsMatrix()->SetMatrix((float *) prevWorldViewProj));
        V(mainEffect->GetVariableByName("world")->AsMatrix()->SetMatrix((float *) world));
        V(mainEffect->GetVariableByName("worldInverseTranspose")->AsMatrix()->SetMatrixTranspose((float *) world));
        V(mainEffect->GetVariableByName("id")->AsScalar()->SetInt(1));

        mesh.Render(device, mainEffect->GetTechniqueByName("Render"), mainEffect->GetVariableByName("diffuseTex")->AsShaderResource(), mainEffect->GetVariableByName("normalTex")->AsShaderResource());
    }

    // Update previous view-projection matrix:
    prevViewProj = currViewProj;

    // Sky dome rendering:
    if (isMSAA())
        device->OMSetRenderTargets(1, *mainRTMS, *depthStencilMS);
    else
        device->OMSetRenderTargets(1, *mainRT, *depthStencil);
    skyDome[currentSkyDome]->render(camera.getViewMatrix(), camera.getProjectionMatrix(), 10.0f);
    device->OMSetRenderTargets(0, NULL, NULL);

    // Multisample buffers resolve:
    if (isMSAA()) {
        DXGI_FORMAT format = mainHud.GetCheckBox(IDC_HDR)->GetChecked()? DXGI_FORMAT_R16G16B16A16_FLOAT : DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
        device->ResolveSubresource(*mainRT, 0, *mainRTMS, 0, format);
        device->ResolveSubresource(*depthRT, 0, *depthRTMS, 0, DXGI_FORMAT_R32_FLOAT);
        device->ResolveSubresource(*specularsRT, 0, *specularsRTMS, 0, format);
    }
}


void addSpecular(ID3D10Device *device, RenderTarget *dst) {
    HRESULT hr;
    quad->setInputLayout();
    V(mainEffect->GetVariableByName("specularsTex")->AsShaderResource()->SetResource(*specularsRT));
    V(mainEffect->GetTechniqueByName("AddSpecular")->GetPassByIndex(0)->Apply(0));
    device->OMSetRenderTargets(1, *dst, NULL);
    quad->draw();
    device->OMSetRenderTargets(0, NULL, NULL);
}


void smaaPass(ID3D10Device *device) {
    // Clear the stencil buffer:
    device->ClearDepthStencilView(*depthStencil, D3D10_CLEAR_STENCIL, 1.0, 0);

    // Calculate next subpixel index:
    int previousIndex = subsampleIndex;
    int currentIndex = (subsampleIndex + 1) % 2;

    // Run SMAA:
    smaa->go(*tmpRT, *tmpRT_SRGB, *depthRT, *finalRT[currentIndex], *depthStencil, SMAA::INPUT_LUMA,  SMAA::MODE_SMAA_T2X, subsampleIndex);
    smaa->reproject(*finalRT[currentIndex], *finalRT[previousIndex], *velocityRT, *tmpRT_SRGB);

    // Update subpixel index:
    subsampleIndex = currentIndex;
}


void renderText() {
    txtHelper->Begin();
    txtHelper->SetInsertionPos(7, 5);
    txtHelper->SetForegroundColor(D3DXCOLOR(1.0f, 1.0f, 1.0f, 1.0f));

    wstringstream s;

    if (nFrames > 0) {
        s << setprecision(2) << std::fixed;
        s << "Min FPS: " << minFrames << endl;
        txtHelper->DrawTextLine(s.str().c_str());

        s.str(L"");
        s << "Max FPS: " << maxFrames << endl;
        txtHelper->DrawTextLine(s.str().c_str());

        s.str(L"");
        s << "Average FPS: " << nFrames / introTime << endl;
        txtHelper->DrawTextLine(s.str().c_str());

        txtHelper->DrawTextLine(L"");
    }

    if (timer->isEnabled()) {
        s.str(L"");
        s << *timer;
        txtHelper->DrawTextLine(s.str().c_str());
    }

    txtHelper->End();
}


void renderScene(ID3D10Device *device, double time, float elapsedTime) {
    // Shadow Pass
    timer->start();
    shadowPass(device);
    timer->clock(L"Shadows");

    // Main Pass
    mainPass(device);
    timer->clock(L"Main Pass");

    // Subsurface Scattering Pass
    if (mainHud.GetCheckBox(IDC_SSS)->GetChecked())
        separableSSS->go(*mainRT, *mainRT, *depthRT, *depthStencil, *specularsRT, 1);
    timer->clock(L"Skin");

    if (mainHud.GetCheckBox(IDC_SEPARATE_SPECULARS)->GetChecked())
        addSpecular(device, mainRT);
    timer->clock(L"Separate Speculars");

    // Bloom Pass
    bloom->go(*mainRT, *tmpRT_SRGB);
    timer->clock(L"Bloom");
    
    // Depth of Field Pass
    dof->go(*tmpRT_SRGB, *tmpRT_SRGB, *depthRT);
    timer->clock(L"DoF");

    // SMAA Pass
    if (SMAA::Mode(antialiasingMode) == SMAA::MODE_SMAA_T2X) {
        smaaPass(device);
        timer->clock(L"SMAA");
    }

    // Film Grain Pass
    filmGrain->go(*tmpRT_SRGB, *backbufferRT, 2.5f * float(time));
    timer->clock(L"Film Grain");
}


void updateLightControls(int i) {
    wstringstream s;
    s << lights[i].color.x;
    secondaryHud.GetStatic(IDC_LIGHT1_LABEL + i)->SetText(s.str().c_str());
    secondaryHud.GetSlider(IDC_LIGHT1 + i)->SetValue(int((lights[i].color.x / 5.0f) * 100.0f));
}


float updateSlider(CDXUTDialog &hud, UINT slider, UINT label, float scale, wstring text) {
    int min, max;
    hud.GetSlider(slider)->GetRange(min, max);
    float value = scale * float(hud.GetSlider(slider)->GetValue()) / (max - min);

    wstringstream s;
    s << text << value;
    hud.GetStatic(label)->SetText(s.str().c_str());

    return value;
}


void loadShot(istream &f) {
    HRESULT hr;

    f >> camera;

    for (int i = 0; i < N_LIGHTS; i++) {
        f >> lights[i].camera;
        f >> lights[i].color.x;
        f >> lights[i].color.y;
        f >> lights[i].color.z;
        updateLightControls(i);
    }

    float ambient;
    f >> ambient;
    secondaryHud.GetSlider(IDC_AMBIENT)->SetValue(int(ambient * 100.0f));
    updateSlider(secondaryHud, IDC_AMBIENT, IDC_AMBIENT_LABEL, 1.0f, L"Ambient: ");
    V(mainEffect->GetVariableByName("ambient")->AsScalar()->SetFloat(ambient));

    float environment;
    f >> environment;
    secondaryHud.GetSlider(IDC_ENVMAP)->SetValue(int(environment * 100.0f));
    updateSlider(secondaryHud, IDC_ENVMAP, IDC_ENVMAP_LABEL, 1.0f, L"Enviroment: ");
    for (int i = 0; i < 3; i++)
        skyDome[i]->setIntensity(environment);

    float value;
    f >> value;
    secondaryHud.GetSlider(IDC_FOCUS_DISTANCE)->SetValue(int((value / 2.0f) * 100.0f));
    updateSlider(secondaryHud, IDC_FOCUS_DISTANCE, IDC_FOCUS_DISTANCE_LABEL, 2.0f, L"Focus dist.: "); 
    dof->setFocusDistance(value);

    f >> value;
    secondaryHud.GetSlider(IDC_FOCUS_RANGE)->SetValue(int(value * 100.0f));
    updateSlider(secondaryHud, IDC_FOCUS_RANGE, IDC_FOCUS_RANGE_LABEL, 1.0f, L"Focus Range: "); 
    dof->setFocusRange(pow(value, 5.0f));

    f >> value;
    secondaryHud.GetSlider(IDC_FOCUS_FALLOFF)->SetValue(int((value / 20.0f)* 100.0f));
    updateSlider(secondaryHud, IDC_FOCUS_FALLOFF, IDC_FOCUS_FALLOFF_LABEL, 20.0f, L"Focus Falloff: "); 
    dof->setFocusFalloff(D3DXVECTOR2(value, value));
}


void loadPreset(int i) {
    wstringstream s;
    s << L"Preset" << i << L".txt";
    HRSRC src = FindResource(GetModuleHandle(NULL), s.str().c_str(), RT_RCDATA);
    HGLOBAL res = LoadResource(GetModuleHandle(NULL), src);
    LPBYTE data = (LPBYTE) LockResource(res);

    stringstream f((char *) data);
    loadShot(f);
}


void setExposure(float value) {
    bloom->setExposure(value);
    filmGrain->setExposure(value);
}


void correct(float amount) {
    camera.setDistanceVelocity(amount * camera.getDistanceVelocity());
    camera.setPanVelocity(amount * camera.getPanVelocity());
    camera.setAngularVelocity(amount * camera.getAngularVelocity());
}


bool setupIntro(float t) {
    HRESULT hr;

    switch (scene) {
        case 0:
            setExposure(0.0f);

            loadPreset(scene);
            camera.setDistanceVelocity(-25.1f);
            camera.setPanVelocity(D3DXVECTOR2(0.0f, -1.0f));
            correct(0.63f);
            scene++;
            break;
        case 1: {
            t = t - sceneTime;
            setExposure(2.0f * Animation::smoothFade(t, 0.0f, 39.0f, 15.0f, 5.0f));
            float focusFalloff = Animation::smooth2(t, 13.0f, 39.0f, 0.0f, 15.0f, 0.0f);
            dof->setFocusFalloff(D3DXVECTOR2(focusFalloff, focusFalloff));

            if (t > 40.5f) {
                sceneTime += 40.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                scene++;
            }
            break;
        }   
        case 2:
            t = t - sceneTime;
            setExposure(0.5f * Animation::smoothFade(t, 0.0f, 16.0f, 3.0f, 3.0f));
            lights[1].color = Animation::smooth2(t, 5.0, 16.0f, D3DXVECTOR3(0.0f, 0.0f, 0.0f), D3DXVECTOR3(1.6f, 1.6f, 1.6f), D3DXVECTOR3(0.0f, 0.0f, 0.0f));

            if (t > 16.5f) {
                sceneTime += 16.5f;
                loadPreset(scene);
                correct(0.85f);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                scene++;
            }
            break;
        case 3:
            t = t - sceneTime;
            setExposure(1.75f * Animation::smoothFade(t, 0.0f, 17.0f, 3.0f, 3.0f));

            if (t > 17.5f) {
                sceneTime += 17.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.15f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                scene++;
            }
            break;
        case 4:
            t = t - sceneTime;
            setExposure(1.2f * Animation::smoothFade(t, 0.0f, 12.0f, 3.0f, 3.0f));
            lights[1].color = Animation::smooth2(t, 0.0, 10.0f, D3DXVECTOR3(0.5f, 0.5f, 0.5f), D3DXVECTOR3(4.0f, 4.0f, 4.0f), D3DXVECTOR3(0.0f, 0.0f, 0.0f));

            if (t > 12.5f) {
                sceneTime += 12.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                scene++;
            }
            break;
        case 5: {
            t = t - sceneTime;
            float distance = camera.getDistance();
            setExposure(1.2f * Animation::smoothFade(t, 0.0f, 16.0f, 3.0f, 3.0f));
            dof->setFocusDistance(distance - 1.0f + Animation::smooth2(t, 4.5, 8.0f, 0.82f, 0.4f, 0.0f));

            if (t > 16.5f) {
                sceneTime += 16.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                scene++;
            }
            break;
        }
        case 6:
            t = t - sceneTime;
            setExposure(2.3f * Animation::smoothFade(t, 0.0f, 16.0f, 8.0f, 3.0f));
            dof->setFocusRange(Animation::smooth2(t, 0.0f, 7.0f, 0.0f, 0.18f, 0.0f));

            if (t > 16.5f) {
                sceneTime += 16.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                lights[0].color.x *= 0.4f;
                lights[0].color.y *= 0.7f;
                lights[0].color.z *= 0.6f;
                lights[0].color *= 1.5f;
                scene++;
            }
            break;
        case 7:
            t = t - sceneTime;
            setExposure(1.5f * Animation::smoothFade(t, 0.0f, 16.0f, 3.0f, 3.0f));

            if (t > 16.5f) {
                sceneTime += 16.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(0.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
                V(mainEffect->GetVariableByName("specularRoughness")->AsScalar()->SetFloat(0.2f));
                V(mainEffect->GetVariableByName("specularIntensity")->AsScalar()->SetFloat(1.2f));
                scene++;
            }
            break;
        case 8:
            t = t - sceneTime;
            setExposure(2.0f * Animation::smoothFade(t, 0.0f, 16.0f, 3.0f, 3.0f));

            if (t > 16.5f) {
                sceneTime += 16.5f;
                loadPreset(scene);
                camera.setDistanceVelocity(20.0f);
                camera.setPanVelocity(D3DXVECTOR2(0.0f, 1.4f));
                V(mainEffect->GetVariableByName("specularRoughness")->AsScalar()->SetFloat(0.3f));
                V(mainEffect->GetVariableByName("specularIntensity")->AsScalar()->SetFloat(1.88f));
                scene++;
            }
            break;
        case 9:
            t = t - sceneTime;
            setExposure(2.5f * Animation::smoothFade(t, 0.0f, 30.0f, 27.0f, 3.0f));
            float focusFalloff = Animation::smooth2(t, 0.0f, 10.0f, 15.0f, 0.0f, 0.0f);
            dof->setFocusFalloff(D3DXVECTOR2(focusFalloff, focusFalloff));

            if (t > 30.0f)
                return true;
            break;
    }
    return false;
}


void reset() {
    HRESULT hr;

    loadPreset(9);
    camera.setDistanceVelocity(0.0f);
    camera.setPanVelocity(D3DXVECTOR2(0.0f, 0.0f));
    setExposure(2.0f);
    V(mainEffect->GetVariableByName("specularRoughness")->AsScalar()->SetFloat(0.3f));
    V(mainEffect->GetVariableByName("specularIntensity")->AsScalar()->SetFloat(1.88f));
}


void CALLBACK onFrameRender(ID3D10Device *device, double time, float elapsedTime, void *context) {
    if (t0 == numeric_limits<double>::min())
        t0 = time;
    if (tFade == numeric_limits<double>::max() && skipIntro)
        tFade = time;

    #if INTRO_BUILD == 1
    if (float(time - t0) > 1.0f && audioEnabled && !audioStarted) {
        sourceVoice->Start(0);
        audioStarted = true;
        ShowCursor(FALSE);
    }
    #endif

    switch (state) {
        case STATE_SPLASH_INTRO:
            // Run the intro:
            splashScreen->intro(float(time - t0));
            Fade::go(Animation::linear(float(time - tFade), 0.0f, 1.0f, 1.0f, 0.0f));

            // If the scene or the fade have finished:
            if (splashScreen->introHasFinished(float(time - t0)) || float(time - tFade) > 1.0f) {
               // Setup next state:
               state = STATE_INTRO;
               t0 = time;
            }
            break;
        case STATE_INTRO: {
            // Setup the intro:
            bool finished = setupIntro(float(time - t0));

            // Render the scene:
            renderScene(device, time, elapsedTime);
            device->OMSetRenderTargets(1, *backbufferRT, NULL);
            Fade::go(Animation::linear(float(time - tFade), 0.0f, 1.0f, 1.0f, 0.0f));

            // Update counters:
            nFramesInverval += 1;
            if (lastTimeMeasured + 1.0f < float(time - t0)) {
                nFrames += nFramesInverval;
                minFrames = min(minFrames, nFramesInverval);
                maxFrames = max(maxFrames, nFramesInverval);
                lastTimeMeasured = float(time - t0);
                nFramesInverval = 0;
            }

            // If the scene or the fade have finished:
            if (finished || float(time - tFade) > 1.0f) {
               // Setup next state:
               state = STATE_SPLASH_OUTRO;
               introTime = float(time - t0);
               t0 = time;

               // Reset the scene:
               reset();
               break;
            }
            break;
        }
        case STATE_SPLASH_OUTRO:
            // Run the outro:
            splashScreen->outro(float(time - t0));
            Fade::go(Animation::linear(float(time - tFade), 0.0f, 1.0f, 1.0f, 0.0f));

            // If the scene or the fade have finished:
            if (splashScreen->outroHasFinished(float(time - t0)) || float(time - tFade) > 1.0f) {
               // Setup next state:
               state = STATE_RUNNING;
               t0 = time;
               ShowCursor(TRUE);
            }
            break;
        case STATE_RUNNING: {
            // Render the scene:
            renderScene(device, time, elapsedTime);

            // Render the HUD:
            device->OMSetRenderTargets(1, *backbufferRT, NULL);
            if (showHud) {
                mainHud.OnRender(elapsedTime);
                secondaryHud.OnRender(elapsedTime);
                renderText();
            }

            // Show the help:
            if (showHelp) {
                Fade::go(0.25);
                helpHud.OnRender(elapsedTime);
            }

            // Fade out the audio:
            #if INTRO_BUILD == 1
            if (audioEnabled)
                sourceVoice->SetVolume(Animation::smooth2(log(float(time - t0)), 0.0f, 2.0f, 1.0f, 0.0f, 0.0f));
            Fade::go(Animation::linear(float(time - t0), 1.0f, 4.0f, 0.0f, 1.0f));
            #endif
            break;
        }
    }
}


void setupSSS(ID3D10Device *device, const DXGI_SURFACE_DESC *desc) {
    int min, max;
    mainHud.GetSlider(IDC_SSSWIDTH)->GetRange(min, max);
    float sssLevel = 0.025f * float(mainHud.GetSlider(IDC_SSSWIDTH)->GetValue()) / (max - min);
    float value = 33.0f * float(mainHud.GetSlider(IDC_NSAMPLES)->GetValue()) / (max - min);
    int nSamples = std::max(3, 2 * (int(value) / 2) + 1);
    bool stencilInitialized = currentMode().desc.Count == 1;
    bool followSurface = mainHud.GetCheckBox(IDC_FOLLOW_SURFACE)->GetChecked();
    DXGI_FORMAT format = mainHud.GetCheckBox(IDC_HDR)->GetChecked()? DXGI_FORMAT_R16G16B16A16_FLOAT : DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;

    SAFE_DELETE(separableSSS);
    separableSSS = new SeparableSSS(device, desc->Width, desc->Height, CAMERA_FOV, sssLevel, nSamples, stencilInitialized, followSurface, true, format);

    smaa = new SMAA(device, desc->Width, desc->Height, SMAA::PRESET_HIGH, true, true);

    D3DXVECTOR3 strength;
    mainHud.GetSlider(IDC_STRENGTH_R)->GetRange(min, max);
    strength.x = float(mainHud.GetSlider(IDC_STRENGTH_R)->GetValue()) / (max - min);
    strength.y = float(mainHud.GetSlider(IDC_STRENGTH_G)->GetValue()) / (max - min);
    strength.z = float(mainHud.GetSlider(IDC_STRENGTH_B)->GetValue()) / (max - min);
    separableSSS->setStrength(strength);
    
    D3DXVECTOR3 falloff;
    mainHud.GetSlider(IDC_FALLOFF_R)->GetRange(min, max);
    falloff.x = float(mainHud.GetSlider(IDC_FALLOFF_R)->GetValue()) / (max - min);
    falloff.y = float(mainHud.GetSlider(IDC_FALLOFF_G)->GetValue()) / (max - min);
    falloff.z = float(mainHud.GetSlider(IDC_FALLOFF_B)->GetValue()) / (max - min);
    separableSSS->setFalloff(falloff);
}


Camera *currentObject() { 
    switch (object) {
        case OBJECT_CAMERA:
            return &camera;
        default:
            return &lights[object - OBJECT_LIGHT1].camera;
    }
}


void CALLBACK onFrameMove(double time, float elapsedTime, void *context) {
    camera.frameMove(elapsedTime);
    for (int i = 0; i < N_LIGHTS; i++)
        lights[i].camera.frameMove(elapsedTime);

    int min, max;
    secondaryHud.GetSlider(IDC_FOCUS_DISTANCE)->GetRange(min, max);
    float focusDistance = 2.0f * float(secondaryHud.GetSlider(IDC_FOCUS_DISTANCE)->GetValue()) / (max - min);

    float distance = camera.getDistance();
    dof->setFocusDistance(distance - 1.0f + focusDistance);
}


void CALLBACK keyboardProc(UINT nChar, bool keydown, bool altdown, void *context) {
    HRESULT hr;

    if (keydown)
    switch (nChar) {
        case VK_TAB:
            showHud = !showHud;
            break;
        case 'O': {
            fstream f("Config.txt", fstream::out);
            f << camera;
            for (int i = 0; i < N_LIGHTS; i++) {
                f << lights[i].camera;
                f << lights[i].color.x << endl;
                f << lights[i].color.y << endl;
                f << lights[i].color.z << endl;
            }
            float ambient;
            V(mainEffect->GetVariableByName("ambient")->AsScalar()->GetFloat(&ambient));
            f << ambient << endl;
            f << skyDome[currentSkyDome]->getIntensity() << endl;
            f << (dof->getFocusDistance() - camera.getDistance() + 1) << endl;
            f << pow(dof->getFocusRange(), 1.0f / 5.0f) << endl;
            f << dof->getFocusFalloff().x << endl;
            break;
        }
        case 'P': {
            fstream f("Config.txt", fstream::in);
            loadShot(f);
            break;
        }
        case 'S': {
            bool sssEnabled = mainHud.GetCheckBox(IDC_SSS)->GetChecked();
            mainHud.GetCheckBox(IDC_SSS)->SetChecked(!sssEnabled);
            V(mainEffect->GetVariableByName("sssEnabled")->AsScalar()->SetBool(!sssEnabled));
            break;
        }
        case 'X':
            mainHud.GetCheckBox(IDC_PROFILE)->SetChecked(!mainHud.GetCheckBox(IDC_PROFILE)->GetChecked());
            timer->setEnabled(mainHud.GetCheckBox(IDC_PROFILE)->GetChecked());
            break;
        case '1':
            object = OBJECT_CAMERA;
            for (int i = 0; i < N_LIGHTS + 1; i++)
                secondaryHud.GetButton(IDC_CAMERA_BUTTON + i)->SetEnabled(i != int(object));
            break;
        case '2':
        case '3':
        case '4':
        case '5':
        case '6':
            object = Object(OBJECT_LIGHT1 + nChar - '2');
            for (int i = 0; i < N_LIGHTS + 1; i++)
                secondaryHud.GetButton(IDC_CAMERA_BUTTON + i)->SetEnabled(i != int(object));
            break;
        case 'C': {
            if (DXUTIsKeyDown(VK_LCONTROL) || DXUTIsKeyDown(VK_RCONTROL)) {
                string code = separableSSS->getKernelCode();
                if (OpenClipboard(DXUTGetHWND())) {
                    EmptyClipboard();
                    HGLOBAL data;
                    data = GlobalAlloc(GMEM_DDESHARE, code.size() + 1);
                    char *buffer = (char*) GlobalLock(data);
                    strcpy_s(buffer, code.size() + 1, code.c_str());
                    GlobalUnlock(data);
                    SetClipboardData(CF_TEXT, data);
                    CloseClipboard();
                }
            }
            break;
        }
        case ' ':
            if (state == STATE_SPLASH_INTRO || state == STATE_INTRO)
                skipIntro = true;
            break;
    }
}


LRESULT CALLBACK msgProc(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam, bool *pbNoFurtherProcessing, void *context) {
    switch (state) {
        case STATE_RUNNING:
            *pbNoFurtherProcessing = dialogResourceManager.MsgProc(hwnd, msg, wparam, lparam);
            if (*pbNoFurtherProcessing)
                return 0;
            
            if (showHelp) {
                *pbNoFurtherProcessing = helpHud.MsgProc(hwnd, msg, wparam, lparam);
                if (*pbNoFurtherProcessing)
                    return 0;
            } else if (showHud) {
                *pbNoFurtherProcessing = mainHud.MsgProc(hwnd, msg, wparam, lparam);
                if (*pbNoFurtherProcessing)
                    return 0;
            
                *pbNoFurtherProcessing = secondaryHud.MsgProc(hwnd, msg, wparam, lparam);
                if (*pbNoFurtherProcessing)
                    return 0;
            }

            currentObject()->handleMessages(hwnd, msg, wparam, lparam);

            break;
    }

    return 0;
}


void setupMainEffect() {
    HRESULT hr;

    SAFE_RELEASE(mainEffect);

    vector<D3D10_SHADER_MACRO> defines;

    D3D10_SHADER_MACRO separateSpecularsMacro = { "SEPARATE_SPECULARS", NULL };
    if (mainHud.GetCheckBox(IDC_SEPARATE_SPECULARS)->GetChecked())
        defines.push_back(separateSpecularsMacro);

    D3D10_SHADER_MACRO null = { NULL, NULL };
    defines.push_back(null);
    
    ID3D10IncludeResource includeResource;
    V(D3DX10CreateEffectFromResource(GetModuleHandle(NULL), L"Main.fx", NULL, &defines.front(), &includeResource, "fx_4_0", D3D10_SHADER_ENABLE_STRICTNESS, 0, DXUTGetD3D10Device(), NULL, NULL, &mainEffect, NULL, NULL));

    V(mainEffect->GetVariableByName("specularAOTex")->AsShaderResource()->SetResource(specularAOSRV));
    V(mainEffect->GetVariableByName("beckmannTex")->AsShaderResource()->SetResource(beckmannSRV));
    V(mainEffect->GetVariableByName("irradianceTex")->AsShaderResource()->SetResource(irradianceSRV[currentSkyDome]));

    int min, max;
    mainHud.GetSlider(IDC_TRANSLUCENCY)->GetRange(min, max);
    float scale = float(mainHud.GetSlider(IDC_TRANSLUCENCY)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("translucency")->AsScalar()->SetFloat(scale));

    bool sssEnabled = mainHud.GetCheckBox(IDC_SSS)->GetChecked();
    V(mainEffect->GetVariableByName("sssEnabled")->AsScalar()->SetBool(sssEnabled));

    mainHud.GetSlider(IDC_TRANSLUCENCY)->GetRange(min, max);
    float sssWidth = 0.025f * float(mainHud.GetSlider(IDC_SSSWIDTH)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("sssWidth")->AsScalar()->SetFloat(sssWidth));

    secondaryHud.GetSlider(IDC_SPEC_INTENSITY)->GetRange(min, max);
    float specularIntensity = float(secondaryHud.GetSlider(IDC_SPEC_INTENSITY)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("specularIntensity")->AsScalar()->SetFloat(4.0f * specularIntensity));

    secondaryHud.GetSlider(IDC_SPEC_ROUGHNESS)->GetRange(min, max);
    float specularRoughness = float(secondaryHud.GetSlider(IDC_SPEC_ROUGHNESS)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("specularRoughness")->AsScalar()->SetFloat(specularRoughness));

    secondaryHud.GetSlider(IDC_SPEC_FRESNEL)->GetRange(min, max);
    float specularFresnel = float(secondaryHud.GetSlider(IDC_SPEC_FRESNEL)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("specularFresnel")->AsScalar()->SetFloat(specularFresnel));

    secondaryHud.GetSlider(IDC_BUMPINESS)->GetRange(min, max);
    float bumpiness = float(secondaryHud.GetSlider(IDC_BUMPINESS)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("bumpiness")->AsScalar()->SetFloat(bumpiness));

    secondaryHud.GetSlider(IDC_AMBIENT)->GetRange(min, max);
    float ambient = float(secondaryHud.GetSlider(IDC_AMBIENT)->GetValue()) / (max - min);
    V(mainEffect->GetVariableByName("ambient")->AsScalar()->SetFloat(ambient));
}


HRESULT CALLBACK onResizedSwapChain(ID3D10Device *device, IDXGISwapChain *swapChain, const DXGI_SURFACE_DESC *desc, void *context) {
    HRESULT hr;

    V_RETURN(dialogResourceManager.OnD3D10ResizedSwapChain(device, desc));

    DXGI_FORMAT format = mainHud.GetCheckBox(IDC_HDR)->GetChecked()? DXGI_FORMAT_R16G16B16A16_FLOAT : DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;

    mainHud.GetComboBox(IDC_MSAA)->RemoveAllItems();
    mainHud.GetComboBox(IDC_MSAA)->AddItem(L"SMAA T2x", (LPVOID) SMAA::MODE_SMAA_T2X);

    supportedMsaaModes.clear();
    for(char i = 0; i < sizeof(msaaModes) / sizeof(MSAAMode); i++){
        UINT quality;
        device->CheckMultisampleQualityLevels(format, msaaModes[i].desc.Count, &quality);
        if (quality > msaaModes[i].desc.Quality) {
            mainHud.GetComboBox(IDC_MSAA)->AddItem(msaaModes[i].name.c_str(), (void*) (i + 10));
            supportedMsaaModes.push_back(msaaModes[i]);
        }
    }
    mainHud.GetComboBox(IDC_MSAA)->SetSelectedByData((void *) antialiasingMode);

    if (isMSAA()) {
        MSAAMode mode = currentMode();
        mainRTMS = new RenderTarget(device, desc->Width, desc->Height, format, mode.desc);
        depthRTMS = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R32_FLOAT, mode.desc);
        specularsRTMS = new RenderTarget(device, desc->Width, desc->Height, format, mode.desc);
        velocityRTMS = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R16G16_FLOAT, mode.desc);
        depthStencilMS = new DepthStencil(device, desc->Width, desc->Height,  DXGI_FORMAT_R24G8_TYPELESS, DXGI_FORMAT_D24_UNORM_S8_UINT, DXGI_FORMAT_R24_UNORM_X8_TYPELESS, mode.desc);
    }

    backbufferRT = new BackbufferRenderTarget(device, DXUTGetDXGISwapChain());
    mainRT = new RenderTarget(device, desc->Width, desc->Height, format);
    tmpRT_SRGB = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);
    tmpRT = new RenderTarget(device, *tmpRT_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM);
    for (int i = 0; i < 2; i++)
        finalRT[i] = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);

    depthRT = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R32_FLOAT);
    specularsRT = new RenderTarget(device, desc->Width, desc->Height, format);
    velocityRT = new RenderTarget(device, desc->Width, desc->Height, DXGI_FORMAT_R16G16_FLOAT);
    depthStencil = new DepthStencil(device, desc->Width, desc->Height, DXGI_FORMAT_R24G8_TYPELESS, DXGI_FORMAT_D24_UNORM_S8_UINT, DXGI_FORMAT_R24_UNORM_X8_TYPELESS);

    float aspect = (float) desc->Width / desc->Height;
    camera.setProjection(CAMERA_FOV * D3DX_PI / 180.f, aspect, 0.1f, 100.0f);

    setupSSS(device, desc);

    int min, max;
    secondaryHud.GetSlider(IDC_EXPOSURE)->GetRange(min, max);
    float exposure = 5.0f * float(secondaryHud.GetSlider(IDC_EXPOSURE)->GetValue()) / (max - min);
    bloom = new Bloom(device, desc->Width, desc->Height, format, Bloom::TONEMAP_FILMIC, exposure, 0.63f, 1.0f, 1.0f, 0.2f);

    filmGrain = new FilmGrain(device, desc->Width, desc->Height, 1.0f, exposure);

    secondaryHud.GetSlider(IDC_FOCUS_DISTANCE)->GetRange(min, max);
    float focusDistance = 2.0f * float(secondaryHud.GetSlider(IDC_FOCUS_DISTANCE)->GetValue()) / (max - min);
    float focusRange = pow(float(secondaryHud.GetSlider(IDC_FOCUS_RANGE)->GetValue()) / (max - min), 5.0f);
    float focusFalloff = 20.0f * float(secondaryHud.GetSlider(IDC_FOCUS_FALLOFF)->GetValue()) / (max - min);
    dof = new DepthOfField(device, desc->Width, desc->Height, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, focusDistance, focusRange, D3DXVECTOR2(focusFalloff, focusFalloff), 2.5f);

    camera.setViewportSize(D3DXVECTOR2((float) desc->Width, (float) desc->Height));
    for (int i = 0; i < N_LIGHTS; i++)
        lights[i].camera.setViewportSize(D3DXVECTOR2((float) desc->Width, (float) desc->Height));

    mainHud.SetLocation(desc->Width - (45 + HUD_WIDTH), 0);
    secondaryHud.SetLocation(0, desc->Height - 520);
    helpHud.SetLocation(0, 0);
    helpHud.GetStatic(IDC_HELP_TEXT)->SetSize(desc->Width, desc->Height);
    helpHud.GetButton(IDC_HELP_CLOSE_BUTTON)->SetLocation(desc->Width - (45 + HUD_WIDTH) + 35, 10);

    if (!loaded) {
        #if INTRO_BUILD == 1
        loadPreset(0);
        #else
        loadPreset(9);
        #endif
        loaded = true;
    }

    return S_OK;
}


void CALLBACK onReleasingSwapChain(void *context) {
    dialogResourceManager.OnD3D10ReleasingSwapChain();

    SAFE_DELETE(mainRTMS);
    SAFE_DELETE(depthStencilMS);
    SAFE_DELETE(depthRTMS);
    SAFE_DELETE(specularsRTMS);
    SAFE_DELETE(velocityRTMS);

    SAFE_DELETE(backbufferRT);
    SAFE_DELETE(mainRT);
    SAFE_DELETE(tmpRT_SRGB);
    SAFE_DELETE(tmpRT);
    for (int i = 0; i < 2; i++)
        SAFE_DELETE(finalRT[i]);

    SAFE_DELETE(depthRT);
    SAFE_DELETE(specularsRT);
    SAFE_DELETE(velocityRT);
    SAFE_DELETE(depthStencil);

    SAFE_DELETE(separableSSS);
    SAFE_DELETE(smaa);
    SAFE_DELETE(bloom);
    SAFE_DELETE(filmGrain);
    SAFE_DELETE(dof);
}


void CALLBACK onGUIEvent(UINT event, int id, CDXUTControl *control, void *context) {
    HRESULT hr;

    switch (id) {
        case IDC_SSS_HELP:
            showHelp = true;
            mainHud.GetButton(IDC_SSS_HELP)->SetVisible(false);
            break;
        case IDC_HELP_CLOSE_BUTTON:
            showHelp = false;
            mainHud.GetButton(IDC_SSS_HELP)->SetVisible(true);
            break;
        case IDC_TOGGLEFULLSCREEN:
            DXUTToggleFullScreen();
            break;
        case IDC_MSAA: {
            CDXUTComboBox *box = (CDXUTComboBox *) control;
            antialiasingMode = int(box->GetSelectedData());
            onReleasingSwapChain(NULL);
            onResizedSwapChain(DXUTGetD3D10Device(), DXUTGetDXGISwapChain(), DXUTGetDXGIBackBufferSurfaceDesc(), NULL);
            break;
        }
        case IDC_HDR:
            if (event == EVENT_CHECKBOX_CHANGED) {
                onReleasingSwapChain(NULL);
                onResizedSwapChain(DXUTGetD3D10Device(), DXUTGetDXGISwapChain(), DXUTGetDXGIBackBufferSurfaceDesc(), NULL);
            }
            break;
        case IDC_SEPARATE_SPECULARS:
            if (event == EVENT_CHECKBOX_CHANGED)
                setupMainEffect();
            break;
        case IDC_PROFILE:
            if (event == EVENT_CHECKBOX_CHANGED)
                timer->setEnabled(mainHud.GetCheckBox(IDC_PROFILE)->GetChecked());
            break;
        case IDC_SSS: {
            bool sssEnabled = mainHud.GetCheckBox(IDC_SSS)->GetChecked();
            V(mainEffect->GetVariableByName("sssEnabled")->AsScalar()->SetBool(sssEnabled));
            break;
        }
        case IDC_NSAMPLES: {
            int min, max;
            mainHud.GetSlider(IDC_NSAMPLES)->GetRange(min, max);
            float value = 33.0f * float(mainHud.GetSlider(IDC_NSAMPLES)->GetValue()) / (max - min);

            wstringstream s;
            s << L"Samples: " << std::max(3, 2 * (int(value) / 2) + 1);
            mainHud.GetStatic(IDC_NSAMPLES_LABEL)->SetText(s.str().c_str());

            setupSSS(DXUTGetD3D10Device(), DXUTGetDXGIBackBufferSurfaceDesc());
            break;
        }
        case IDC_FOLLOW_SURFACE:
            if (event == EVENT_CHECKBOX_CHANGED) {
                onReleasingSwapChain(NULL);
                onResizedSwapChain(DXUTGetD3D10Device(), DXUTGetDXGISwapChain(), DXUTGetDXGIBackBufferSurfaceDesc(), NULL);
            }
            break;
        case IDC_SSSWIDTH: {
            float value = updateSlider(mainHud, IDC_SSSWIDTH, IDC_SSSWIDTH_LABEL, 0.025f, L"SSS Width: "); 
            separableSSS->setWidth(value);
            V(mainEffect->GetVariableByName("sssWidth")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_STRENGTH_R:
        case IDC_STRENGTH_G:
        case IDC_STRENGTH_B: {
            D3DXVECTOR3 strength; 
            strength.x = updateSlider(mainHud, IDC_STRENGTH_R, IDC_STRENGTH_R_LABEL, 1.0f, L"R: ");
            strength.y = updateSlider(mainHud, IDC_STRENGTH_G, IDC_STRENGTH_G_LABEL, 1.0f, L"G: "); 
            strength.z = updateSlider(mainHud, IDC_STRENGTH_B, IDC_STRENGTH_B_LABEL, 1.0f, L"B: ");  
            separableSSS->setStrength(strength);
            break;
        }
        case IDC_FALLOFF_R:
        case IDC_FALLOFF_G:
        case IDC_FALLOFF_B: {
            D3DXVECTOR3 falloff; 
            falloff.x = updateSlider(mainHud, IDC_FALLOFF_R, IDC_FALLOFF_R_LABEL, 1.0f, L"R: ");
            falloff.y = updateSlider(mainHud, IDC_FALLOFF_G, IDC_FALLOFF_G_LABEL, 1.0f, L"G: "); 
            falloff.z = updateSlider(mainHud, IDC_FALLOFF_B, IDC_FALLOFF_B_LABEL, 1.0f, L"B: ");  
            separableSSS->setFalloff(falloff);
            break;
        }
        case IDC_TRANSLUCENCY: {
            float value = updateSlider(mainHud, IDC_TRANSLUCENCY, IDC_TRANSLUCENCY_LABEL, 1.0f, L"Translucency: "); 
            V(mainEffect->GetVariableByName("translucency")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_SPEC_INTENSITY: {
            float value = updateSlider(secondaryHud, IDC_SPEC_INTENSITY, IDC_SPEC_INTENSITY_LABEL, 4.0f, L"Spec. Intensity: "); 
            V(mainEffect->GetVariableByName("specularIntensity")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_SPEC_ROUGHNESS: {
            float value = updateSlider(secondaryHud, IDC_SPEC_ROUGHNESS, IDC_SPEC_ROUGHNESS_LABEL, 1.0f, L"Spec. Rough.: "); 
            V(mainEffect->GetVariableByName("specularRoughness")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_SPEC_FRESNEL: {
            float value = updateSlider(secondaryHud, IDC_SPEC_FRESNEL, IDC_SPEC_FRESNEL_LABEL, 1.0f, L"Spec. Fresnel: "); 
            V(mainEffect->GetVariableByName("specularFresnel")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_BUMPINESS: {
            float value = updateSlider(secondaryHud, IDC_BUMPINESS, IDC_BUMPINESS_LABEL, 1.0f, L"Bumpiness: "); 
            V(mainEffect->GetVariableByName("bumpiness")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_EXPOSURE: {
            float value = updateSlider(secondaryHud, IDC_EXPOSURE, IDC_EXPOSURE_LABEL, 5.0f, L"Exposure: "); 
            bloom->setExposure(value);
            filmGrain->setExposure(value);
            break;
        }
        case IDC_ENVMAP: {
            float value = updateSlider(secondaryHud, IDC_ENVMAP, IDC_ENVMAP_LABEL, 1.0f, L"Enviroment: "); 
            for (int i = 0; i < 3; i++)
                skyDome[i]->setIntensity(value);
            break;
        }
        case IDC_AMBIENT: {
            float value = updateSlider(secondaryHud, IDC_AMBIENT, IDC_AMBIENT_LABEL, 1.0f, L"Ambient: ");
            V(mainEffect->GetVariableByName("ambient")->AsScalar()->SetFloat(value));
            break;
        }
        case IDC_FOCUS_DISTANCE: {
            updateSlider(secondaryHud, IDC_FOCUS_DISTANCE, IDC_FOCUS_DISTANCE_LABEL, 2.0f, L"Focus dist.: ");
            break;
        }
        case IDC_FOCUS_RANGE: {
            float value = updateSlider(secondaryHud, IDC_FOCUS_RANGE, IDC_FOCUS_RANGE_LABEL, 1.0f, L"Focus Range: ");
            dof->setFocusRange(pow(value, 5.0f));
            break;
        }
        case IDC_FOCUS_FALLOFF: {
            float value = updateSlider(secondaryHud, IDC_FOCUS_FALLOFF, IDC_FOCUS_FALLOFF_LABEL, 20.0f, L"Focus Falloff: ");
            dof->setFocusFalloff(D3DXVECTOR2(value, value));
            break;
        }
        case IDC_CAMERA_BUTTON:
        case IDC_LIGHT1_BUTTON:
        case IDC_LIGHT1_BUTTON + 1:
        case IDC_LIGHT1_BUTTON + 2:
        case IDC_LIGHT1_BUTTON + 3:
        case IDC_LIGHT1_BUTTON + 4:
            object = Object(id - IDC_CAMERA_BUTTON);
            for (int i = 0; i < N_LIGHTS + 1; i++)
                secondaryHud.GetButton(IDC_CAMERA_BUTTON + i)->SetEnabled(i != int(object));
            break;
        case IDC_LIGHT1:
        case IDC_LIGHT1 + 1:
        case IDC_LIGHT1 + 2:
        case IDC_LIGHT1 + 3:
        case IDC_LIGHT1 + 4: {
            float value = updateSlider(secondaryHud, id, IDC_LIGHT1_LABEL + id - IDC_LIGHT1, 5.0f, L""); 
            lights[id - IDC_LIGHT1].color = value * D3DXVECTOR3(1.0, 1.0, 1.0);
            break;
        }
    }
}


void CALLBACK createTextureFromFile(ID3D10Device *device, char *filename, ID3D10ShaderResourceView **shaderResourceView, void *context, bool srgb) {
    HRESULT hr;

    wstringstream s;
    s << ((wchar_t *) context) << "\\" << filename;

    D3DX10_IMAGE_LOAD_INFO loadInfo;
    loadInfo.BindFlags = D3D10_BIND_SHADER_RESOURCE;

    if (srgb) {
        loadInfo.Filter = D3DX10_FILTER_POINT | D3DX10_FILTER_SRGB_IN;
        
        D3DX10_IMAGE_INFO info;
        V(D3DX10GetImageInfoFromResource(GetModuleHandle(NULL), s.str().c_str(), NULL, &info, NULL));
        loadInfo.Format = MAKE_SRGB(info.Format);
    }

    V(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), s.str().c_str(), &loadInfo, NULL, shaderResourceView, NULL));
}


void loadMesh(CDXUTSDKMesh &mesh, ID3D10Device *device, const wstring &name, const wstring &path) {
    HRESULT hr;

    HRSRC src = FindResource(GetModuleHandle(NULL), name.c_str(), RT_RCDATA);
    HGLOBAL res = LoadResource(GetModuleHandle(NULL), src);
    UINT size = SizeofResource(GetModuleHandle(NULL), src);
    LPBYTE data = (LPBYTE) LockResource(res);

    SDKMESH_CALLBACKS10 callbacks;
    ZeroMemory(&callbacks, sizeof(SDKMESH_CALLBACKS10));  
    callbacks.pCreateTextureFromFile = &createTextureFromFile;
    callbacks.pContext = (void *) path.c_str();

    V(mesh.Create(device, data, size, true, true, &callbacks));
}


HRESULT CALLBACK onCreateDevice(ID3D10Device *device, const DXGI_SURFACE_DESC *desc, void *context) {
    HRESULT hr;

    V_RETURN(dialogResourceManager.OnD3D10CreateDevice(device));

    timer = new Timer(device);
    timer->setEnabled(mainHud.GetCheckBox(IDC_PROFILE)->GetChecked());

    V_RETURN(D3DX10CreateFont(device, 15, 0, FW_BOLD, 1, FALSE, DEFAULT_CHARSET,
                              OUT_DEFAULT_PRECIS, DEFAULT_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                              L"Arial", &font));
    V_RETURN(D3DX10CreateSprite(device, 512, &sprite));
    txtHelper = new CDXUTTextHelper(NULL, NULL, font, sprite, 15);

    loadMesh(mesh, device, L"Head\\Head.sdkmesh", L"Head");

    D3DX10_IMAGE_LOAD_INFO loadInfo;
    ZeroMemory(&loadInfo, sizeof(D3DX10_IMAGE_LOAD_INFO));
    loadInfo.BindFlags = D3D10_BIND_SHADER_RESOURCE;
    loadInfo.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    V_RETURN(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), L"Head\\SpecularAOMap.dds", &loadInfo, NULL, &specularAOSRV, NULL));

    ZeroMemory(&loadInfo, sizeof(D3DX10_IMAGE_LOAD_INFO));
    loadInfo.BindFlags = D3D10_BIND_SHADER_RESOURCE;
    loadInfo.MipLevels = 1;
    loadInfo.Format = DXGI_FORMAT_R8_UNORM;
    V_RETURN(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), L"BeckmannMap.dds", &loadInfo, NULL, &beckmannSRV, NULL));

    ZeroMemory(&loadInfo, sizeof(D3DX10_IMAGE_LOAD_INFO));
    loadInfo.BindFlags = D3D10_BIND_SHADER_RESOURCE;
    loadInfo.Format = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
    V_RETURN(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), L"Enviroment\\StPeters\\IrradianceMap.dds", &loadInfo, NULL, &irradianceSRV[0], NULL));
    V_RETURN(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), L"Enviroment\\Grace\\IrradianceMap.dds", &loadInfo, NULL, &irradianceSRV[1], NULL));
    V_RETURN(D3DX10CreateShaderResourceViewFromResource(device, GetModuleHandle(NULL), L"Enviroment\\Eucalyptus\\IrradianceMap.dds", &loadInfo, NULL, &irradianceSRV[2], NULL));

    setupMainEffect();
    
    const D3D10_INPUT_ELEMENT_DESC layout[] = {
        { "POSITION",  0, DXGI_FORMAT_R32G32B32_FLOAT, 0, D3D10_APPEND_ALIGNED_ELEMENT, D3D10_INPUT_PER_VERTEX_DATA, 0 },
        { "NORMAL",    0, DXGI_FORMAT_R32G32B32_FLOAT, 0, D3D10_APPEND_ALIGNED_ELEMENT, D3D10_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD",  0, DXGI_FORMAT_R32G32_FLOAT,    0, D3D10_APPEND_ALIGNED_ELEMENT, D3D10_INPUT_PER_VERTEX_DATA, 0 },
        { "TANGENT",   0, DXGI_FORMAT_R32G32B32_FLOAT, 0, D3D10_APPEND_ALIGNED_ELEMENT, D3D10_INPUT_PER_VERTEX_DATA, 0 }
    };
    UINT numElements = sizeof(layout) / sizeof(D3D10_INPUT_ELEMENT_DESC);

    D3D10_PASS_DESC passDesc;
    V_RETURN(mainEffect->GetTechniqueByName("Render")->GetPassByIndex(0)->GetDesc(&passDesc));
    V_RETURN(device->CreateInputLayout(layout, numElements, passDesc.pIAInputSignature, passDesc.IAInputSignatureSize, &vertexLayout));

    device->IASetInputLayout(vertexLayout);

    for (int i = 0; i < N_LIGHTS; i++) {
        lights[i].fov = 45.0f * D3DX_PI / 180.f;
        lights[i].falloffWidth = 0.05f;
        lights[i].attenuation = 1.0f / 128.0f;
        lights[i].farPlane = 10.0f;
        lights[i].bias = -0.01f;

        lights[i].camera.setDistance(2.0);
        lights[i].camera.setProjection(lights[i].fov, 1.0f, 0.1f, lights[i].farPlane);
        lights[i].color = D3DXVECTOR3(0.0f, 0.0f, 0.0f);
        lights[i].shadowMap = new ShadowMap(device, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE);
    }

    splashScreen = new SplashScreen(device, sprite);
    skyDome[0] = new SkyDome(device, L"Enviroment\\StPeters", 0.0f);
    skyDome[1] = new SkyDome(device, L"Enviroment\\Grace", 0.0f);
    skyDome[2] = new SkyDome(device, L"Enviroment\\Eucalyptus", 0.0f);

    V(mainEffect->GetTechniqueByName("AddSpecular")->GetPassByIndex(0)->GetDesc(&passDesc));
    quad = new Quad(device, passDesc);

    ShadowMap::init(device);
    Fade::init(device);

    return S_OK;
}


void CALLBACK onDestroyDevice(void *context) {
    dialogResourceManager.OnD3D10DestroyDevice();
    DXUTGetGlobalResourceCache().OnDestroyDevice();

    SAFE_DELETE(timer);
    SAFE_RELEASE(font);
    SAFE_RELEASE(sprite);
    SAFE_DELETE(txtHelper);

    mesh.Destroy();
    SAFE_RELEASE(specularAOSRV);
    SAFE_RELEASE(beckmannSRV);
    SAFE_RELEASE(mainEffect);
    SAFE_RELEASE(vertexLayout);

    for (int i = 0; i < N_LIGHTS; i++)
        SAFE_DELETE(lights[i].shadowMap);

    SAFE_DELETE(splashScreen);

    for (int i = 0; i < 3; i++) {
        SAFE_DELETE(skyDome[i]);
        SAFE_RELEASE(irradianceSRV[i]);
    }

    SAFE_DELETE(quad);

    ShadowMap::release();
    Fade::release();
}


HRESULT initAudio() {
    HRESULT hr;

    UINT32 flags = 0;
    #if defined(DEBUG) | defined(_DEBUG)
    flags |= XAUDIO2_DEBUG_ENGINE;
    #endif

    if (FAILED(hr = XAudio2Create(&xaudio, flags)))
        return hr;

    if (FAILED(hr = xaudio->CreateMasteringVoice(&masteringVoice)))
        return hr;

    CWaveFile wav;
    // I modified CWaveFile::Open so that it can open xWMA files
    V_RETURN(wav.Open(L"Music.xwm", NULL, WAVEFILE_READ));
    WAVEFORMATEX *format = wav.GetFormat();
    DWORD size = wav.GetSize();

    waveData.resize(size);
    V_RETURN(wav.Read(&waveData.front(), size, &size));
    V_RETURN(xaudio->CreateSourceVoice(&sourceVoice, format));

    XAUDIO2_BUFFER buffer = {0};
    buffer.pAudioData = &waveData.front();
    buffer.Flags = XAUDIO2_END_OF_STREAM;
    buffer.AudioBytes = size;


    if (mmioSeek(wav.m_hmmio, 0, SEEK_SET) == -1)
        return E_FAIL;

    MMCKINFO ckinfo;
    if (mmioDescend(wav.m_hmmio, &ckinfo, NULL, 0) != 0)
        return E_FAIL;

    ckinfo.ckid = mmioFOURCC( 'd', 'p', 'd', 's' );
    if (mmioDescend(wav.m_hmmio, &ckinfo, NULL, MMIO_FINDCHUNK) != 0)
        return E_FAIL;

    XAUDIO2_BUFFER_WMA packetWMA;
    packetWMA.PacketCount = ckinfo.cksize / sizeof(UINT32);
    dpdsData.resize(packetWMA.PacketCount);
    packetWMA.pDecodedPacketCumulativeBytes = &dpdsData.front();

    if (unsigned(mmioRead(wav.m_hmmio, (HPSTR) packetWMA.pDecodedPacketCumulativeBytes, ckinfo.cksize)) != ckinfo.cksize)
        return E_FAIL;

    V_RETURN(sourceVoice->SubmitSourceBuffer(&buffer, &packetWMA))

    return S_OK;
}


void releaseAudio() {
    if (sourceVoice) sourceVoice->DestroyVoice(); sourceVoice = NULL;
    if (masteringVoice) masteringVoice->DestroyVoice(); masteringVoice = NULL;
    SAFE_RELEASE(xaudio);
    waveData.clear();
    dpdsData.clear();
}


void initApp() {
    mainHud.Init(&dialogResourceManager);
    mainHud.SetCallback(onGUIEvent);

    secondaryHud.Init(&dialogResourceManager);
    secondaryHud.SetCallback(onGUIEvent);

    helpHud.Init(&dialogResourceManager);
    helpHud.SetCallback(onGUIEvent);

    /**
     * Create main hud (the one on the right)
     */
    int iY = 10;
    mainHud.AddButton(IDC_SSS_HELP, L"Help", 35, iY, HUD_WIDTH, 22, VK_F1);
    iY += 15;

    mainHud.AddButton(IDC_TOGGLEFULLSCREEN, L"Toggle Fullscreen", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddComboBox(IDC_MSAA, 35, iY += 24, HUD_WIDTH, 22);

    iY += 15;
    mainHud.AddCheckBox(IDC_HDR, L"HDR", 35, iY += 24, HUD_WIDTH, 22, true);
    mainHud.AddCheckBox(IDC_SEPARATE_SPECULARS, L"Separate Speculars", 35, iY += 24, HUD_WIDTH, 22, true);
    mainHud.AddCheckBox(IDC_PROFILE, L"Profile", 35, iY += 24, HUD_WIDTH, 22, false);

    iY += 15;
    mainHud.AddCheckBox(IDC_SSS, L"SSS Rendering", 35, iY += 24, HUD_WIDTH, 22, true);
    mainHud.AddCheckBox(IDC_FOLLOW_SURFACE, L"SSS Follow Surface", 35, iY += 24, HUD_WIDTH, 22, true);

    iY += 15;
    mainHud.AddStatic(IDC_NSAMPLES_LABEL, L"Samples: 17", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddSlider(IDC_NSAMPLES, 35, iY += 24, HUD_WIDTH, 22, 0, 100, int((17.0f / 33.0f) * 100.0f));
    mainHud.AddStatic(IDC_SSSWIDTH_LABEL, L"SSS Width: 0.012", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddSlider(IDC_SSSWIDTH, 35, iY += 24, HUD_WIDTH, 22, 0, 100, int((0.012f / 0.025f) * 100.0f));

    iY += 15;
    mainHud.AddStatic(IDC_STRENGTH_LABEL, L"SSS Strength", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddStatic(IDC_STRENGTH_R_LABEL, L"R: 0.48", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_STRENGTH_R, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(0.48f * 100.0f));
    mainHud.AddStatic(IDC_STRENGTH_G_LABEL, L"G: 0.41", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_STRENGTH_G, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(0.41f * 100.0f));
    mainHud.AddStatic(IDC_STRENGTH_B_LABEL, L"B: 0.28", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_STRENGTH_B, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(0.28f * 100.0f));

    iY += 15;
    mainHud.AddStatic(IDC_FALLOFF_LABEL, L"SSS Falloff", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddStatic(IDC_FALLOFF_R_LABEL, L"R: 1.0", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_FALLOFF_R, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(1.0f * 100.0f));
    mainHud.AddStatic(IDC_FALLOFF_G_LABEL, L"G: 0.37", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_FALLOFF_G, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(0.37f * 100.0f));
    mainHud.AddStatic(IDC_FALLOFF_B_LABEL, L"B: 0.3", 35, iY += 24, 50, 22);
    mainHud.AddSlider(IDC_FALLOFF_B, 80, iY, HUD_WIDTH - 45, 22, 0, 100, int(0.3f * 100.0f));

    iY += 15;
    mainHud.AddStatic(IDC_TRANSLUCENCY_LABEL, L"Translucency: 0.83", 35, iY += 24, HUD_WIDTH, 22);
    mainHud.AddSlider(IDC_TRANSLUCENCY, 35, iY += 24, HUD_WIDTH, 22, 0, 100, int(0.83f * 100.0f));

    /**
     * Create the speculars and light step hud (the one on the left)
     */
    iY = 0;
    int iX = 10;

    secondaryHud.AddStatic(IDC_SPEC_INTENSITY_LABEL, L"Spec. Intensity: 1.88", iX, iY, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_SPEC_INTENSITY, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int((1.88f / 4.0f) * 100.0f));
    secondaryHud.AddStatic(IDC_SPEC_ROUGHNESS_LABEL, L"Spec. Rough.: 0.3", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_SPEC_ROUGHNESS, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int(0.3f * 100.0f));
    secondaryHud.AddStatic(IDC_SPEC_FRESNEL_LABEL, L"Spec. Fresnel: 0.82", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_SPEC_FRESNEL, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int(0.82f * 100.0f));
    iY += 15;

    secondaryHud.AddStatic(IDC_BUMPINESS_LABEL, L"Bumpiness: 0.9", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_BUMPINESS, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int(0.9f * 100.0f));
    iY += 15;

    secondaryHud.AddStatic(IDC_EXPOSURE_LABEL, L"Exposure: 2.0", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_EXPOSURE, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int((2.0f / 5.0f) * 100.0f));

    secondaryHud.AddStatic(IDC_ENVMAP_LABEL, L"Env. Map: 0", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_ENVMAP, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int(0.0f * 100.0f));

    secondaryHud.AddStatic(IDC_AMBIENT_LABEL, L"Ambient: 0", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_AMBIENT, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int(0.0f * 100.0f));

    secondaryHud.AddStatic(IDC_FOCUS_DISTANCE_LABEL, L"Focus Dist.: 0.55", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_FOCUS_DISTANCE, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int((0.55f / 2.0f) * 100.0f));

    secondaryHud.AddStatic(IDC_FOCUS_RANGE_LABEL, L"Focus Range: 0.86", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_FOCUS_RANGE, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int((0.86f / 1.0f) * 100.0f));

    secondaryHud.AddStatic(IDC_FOCUS_FALLOFF_LABEL, L"Focus Falloff: 10.0", iX, iY += 24, HUD_WIDTH2, 22);
    secondaryHud.AddSlider(IDC_FOCUS_FALLOFF, iX, iY += 24, HUD_WIDTH2, 22, 0, 100, int((10.0f / 20.0f) * 100.0f));

    int top = iY - 24 * 3;
    iX += HUD_WIDTH2 + 20;

    iY = top;
    secondaryHud.AddButton(IDC_CAMERA_BUTTON, L"Camera", iX, iY += 24, 45, 22);
    secondaryHud.GetButton(IDC_CAMERA_BUTTON)->SetEnabled(false);
    iX += 45 + 20;

    for (int i = 0; i < N_LIGHTS; i++) {
        iY = top;

        wstringstream s;
        s << L"Light " << (i + 1); 
        secondaryHud.AddButton(IDC_LIGHT1_BUTTON + i, s.str().c_str(), iX, iY += 24, 45, 22);
        secondaryHud.AddStatic(IDC_LIGHT1_LABEL + i, L"0", iX, iY += 24, 45, 22);
        secondaryHud.AddSlider(IDC_LIGHT1 + i, iX, iY += 24, 45, 22, 0, 100, 0);

        iX += 45 + 20;
    }

    /**
     * Create the help page
     */
    HRSRC src = FindResource(GetModuleHandle(NULL), L"Help.txt", RT_RCDATA);
    HGLOBAL res = LoadResource(GetModuleHandle(NULL), src);
    UINT size = SizeofResource(GetModuleHandle(NULL), src);
    CHAR *data = (CHAR *) LockResource(res);

    int n = MultiByteToWideChar(CP_ACP, 0, data, size, NULL, 0);
    WCHAR *wdata = new WCHAR[n + 1];
    MultiByteToWideChar(CP_ACP, 0, data, size, wdata, n);
    wdata[n] = 0;
    helpHud.AddStatic(IDC_HELP_TEXT, wdata, 0, 0, 1280, 720);
    delete [] wdata;

    helpHud.AddButton(IDC_HELP_CLOSE_BUTTON, L"Close", 0, 0, HUD_WIDTH, 22, VK_F1);
}


bool CALLBACK modifyDeviceSettings(DXUTDeviceSettings *settings, void *context) {
    settings->d3d10.AutoCreateDepthStencil = false;
    settings->d3d10.sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT | DXGI_USAGE_SHADER_INPUT;
    return true;
}


bool CALLBACK isDeviceAcceptable(UINT adapter, UINT output, D3D10_DRIVER_TYPE deviceType, DXGI_FORMAT format, bool windowed, void *context) {
    return true;
}


INT WINAPI wWinMain(HINSTANCE, HINSTANCE, LPWSTR, int) {
    // Enable run-time memory check for debug builds.
    #if defined(DEBUG) | defined(_DEBUG)
    _CrtSetDbgFlag(_CRTDBG_ALLOC_MEM_DF | _CRTDBG_LEAK_CHECK_DF);
    #endif

    DXUTSetCallbackD3D10DeviceAcceptable(isDeviceAcceptable);
    DXUTSetCallbackD3D10DeviceCreated(onCreateDevice);
    DXUTSetCallbackD3D10DeviceDestroyed(onDestroyDevice);
    DXUTSetCallbackD3D10SwapChainResized(onResizedSwapChain);
    DXUTSetCallbackD3D10SwapChainReleasing(onReleasingSwapChain);
    DXUTSetCallbackD3D10FrameRender(onFrameRender);
    
    DXUTSetCallbackDeviceChanging(modifyDeviceSettings);
    DXUTSetCallbackMsgProc(msgProc);
    DXUTSetCallbackKeyboard(keyboardProc);
    DXUTSetCallbackFrameMove(onFrameMove);

    #if INTRO_BUILD == 1
    CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (FAILED(initAudio())) {
        audioEnabled = false;
        releaseAudio();
    }
    #endif

    initApp();
    DXUTInit(true, true, L"-forcevsync:0");
    DXUTSetCursorSettings(true, true);
    DXUTCreateWindow(L"Separable SSS Demo");
    DXUTCreateDevice(true, 1280, 720);
    ShowWindow(DXUTGetHWNDDeviceWindowed(), SW_MAXIMIZE);
    DXUTMainLoop();

    #if INTRO_BUILD == 1
    releaseAudio();
    CoUninitialize();
    #endif

    return DXUTGetExitCode();
}


```

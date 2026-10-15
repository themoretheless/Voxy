# Facial rendering source scan: DENO1017/PSSSLutInUnity

Repository: https://github.com/DENO1017/PSSSLutInUnity

Pinned source revision: 1bbe95c13e6371495442578bcd2d062d12c7ef03

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"sss": ["PsssScenePackage.unitypackage", "SSSCore.cginc"], "shader": ["DiffuseProfile.shader", "LineChart.shader", "PssLut.shader"]}

## Source file: SSSCore.cginc

{
  "path": "SSSCore.cginc",
  "blob_sha": "15b56b1e954293e725a7ba35aed2691d0e15621e",
  "sha256": "06e7c38a8a604ebb7c0985081f10c3d8cc820fc2288750fdcae3f4bf1a339bb4",
  "lines": 85,
  "bytes": 2037,
  "selection_score": 27,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/DENO1017/PSSSLutInUnity/blob/1bbe95c13e6371495442578bcd2d062d12c7ef03/SSSCore.cginc",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "float G1(float Neg_r_2, float v)"
    },
    {
      "line": 36,
      "text": "float G2(float Neg_r_2, float v)"
    },
    {
      "line": 41,
      "text": "float G(float Neg_r_2, float v)"
    },
    {
      "line": 50,
      "text": "float3 CalcGuss(float distance)"
    },
    {
      "line": 62,
      "text": "float NormDiff(float r, float d)"
    },
    {
      "line": 67,
      "text": "float3 CalcNorm(float distance)"
    },
    {
      "line": 78,
      "text": "float3 RadianceProfile(float distance)"
    }
  ]
}

```text

#include "UnityCG.cginc"

float4 _D;
half4 _DiffColor1;
half4 _DiffColor2;
half4 _DiffColor3;
half4 _DiffColor4;
#define PI 3.141592654

struct appdata
{
    float4 vertex : POSITION;
    float2 uv : TEXCOORD0;
};

struct v2f
{
    float2 uv : TEXCOORD0;
    float4 vertex : SV_POSITION;
};
v2f vert (appdata v)
{
    v2f o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.uv = v.uv;
    return o;
}

float G1(float Neg_r_2, float v)
{
    v =  1.414 * v;
    return 1.0/ sqrt(2.0 * PI * v) * exp(Neg_r_2 / (2 * v));
}


float G2(float Neg_r_2, float v)
{
    return 1.0/(2.0 * PI * v) * exp(Neg_r_2 / (2 * v));
}

float G(float Neg_r_2, float v)
{
    #if _RADIANCE_G1
        return G1(Neg_r_2,v);
    #else 
        return G2(Neg_r_2,v);
    #endif
}

float3 CalcGuss(float distance)
{
    float Neg_r_2 = -distance*distance;
    float3 rgb = float3(0.233,0.455,0.649) * G(Neg_r_2 , 0.0064)+\
        float3(0.100,0.336,0.344) * G(Neg_r_2 , 0.0484)+\
        float3(0.118,0.198,0.000) * G(Neg_r_2 , 0.1870)+\
        float3(0.113,0.007,0.007) * G(Neg_r_2 , 0.5670)+\
        float3(0.358,0.004,0.000) * G(Neg_r_2 , 1.9900)+\
        float3(0.078,0.000,0.000) * G(Neg_r_2 , 7.4100);
    return rgb;
}

float NormDiff(float r, float d)
{
    return saturate(exp(-r/d) + exp(-r/(3 * d)) / (8 * PI * d * r));
}

float3 CalcNorm(float distance)
{
    float3 rgb = float3(0,0,0);
    half sum = 1 / (_DiffColor1.a + _DiffColor2.a + _DiffColor3.a + _DiffColor4.a);
    rgb += NormDiff(distance,_D.x) * _DiffColor1.rgb *  _DiffColor1.a * sum;
    rgb += NormDiff(distance,_D.y) * _DiffColor2.rgb *  _DiffColor2.a * sum;
    rgb += NormDiff(distance,_D.z) * _DiffColor3.rgb *  _DiffColor3.a * sum;
    rgb += NormDiff(distance,_D.w) * _DiffColor4.rgb *  _DiffColor4.a * sum;
    return rgb;
}

float3 RadianceProfile(float distance)
{
    #if _RADIANCE_NORMDIFF
        return CalcNorm(distance);
    #else
        return CalcGuss(distance);
    #endif
}

```

## Source file: PssLut.shader

{
  "path": "PssLut.shader",
  "blob_sha": "59759d435589043588800bf48ceebbe90c289200",
  "sha256": "49a86ce915775409b62d777e958b18d4873493f8dc287fccd8b13080f3e1db67",
  "lines": 111,
  "bytes": 3919,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "shader": 3
  },
  "url": "https://github.com/DENO1017/PSSSLutInUnity/blob/1bbe95c13e6371495442578bcd2d062d12c7ef03/PssLut.shader",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "float3 F(float3 x)"
    },
    {
      "line": 47,
      "text": "float3 Uncharted2ToneMapping(float3 color)"
    },
    {
      "line": 53,
      "text": "float3 ACESToneMapping(float3 x)"
    },
    {
      "line": 63,
      "text": "float3 IntegrateDiffuseScatteringOnRing(float uvx,float Radius)"
    }
  ]
}

```text

﻿Shader "SSS/PssLut"
{
    Properties
    {
        _Max1R("Max 1/Radiance", float) = 1
        [KeywordEnum(NormDiff, G1, G2)] _Radiance ("Radiance Profile type", Float) = 0
        _D ("D", Vector) = (1,1,1,1)
        _DiffColor1 ("NormDiff Color 1", Color) = (1,1,1,1)
        _DiffColor2 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor3 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor4 ("NormDiff Color 2", Color) = (0,0,0,0)
        // [KeywordEnum(None, Uncharted, ACES)] _Tone ("Tone type", Float) = 0
        // _AdaptedLum ("Uncharted-AdaptedLum", float) = 1
        [Toggle] _Gamma ("Gamma", Float) = 0
    }
    SubShader
    {
        Tags { "RenderType"="Opaque" }
        LOD 100

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag

            #pragma shader_feature _GAMMA_ON
            #pragma multi_compile _RADIANCE_NORMDIFF _RADIANCE_G1 _RADIANCE_G2
            // #pragma multi_compile _TONE_NONE _TONE_UNCHARTED _TONE_ACES
            #include "SSSCore.cginc"
            float _Max1R;
            float _AdaptedLum;


            float3 F(float3 x)
            {
                float A = 0.15;
                float B = 0.50;
                float C = 0.10;
                float D = 0.20;
                float E = 0.02;
                float F = 0.30;

                return ((x * ( A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E/F;
            }

            float3 Uncharted2ToneMapping(float3 color)
            {
                const float WHITE = 11.2f;
                return F(1.6f * _AdaptedLum * color) / F(WHITE);
            }

            float3 ACESToneMapping(float3 x)
            {
                float a = 2.51f;
                float b = 0.03f;
                float c = 2.43f;
                float d = 0.59f;
                float e = 0.14f;
                return saturate((x*(a*x+b))/(x*(c*x+d)+e));
            }

            float3 IntegrateDiffuseScatteringOnRing(float uvx,float Radius)
            {
                float theta =acos(uvx);
                float a = 0;
                float b = 0;
                float3 totalWeights = float3(0,0,0);
                float3 totalLight = float3(0,0,0);
                while(b <= 0.5 * PI)
                {
                    a = 0;
                    while (a <= 2 * PI)
                    {
                        // float sampleDist = 2 * Radius * sin(0.5 * a);
                        float sampleDist = sqrt(2 - 2 * cos(a) * cos(b)) * Radius;
                        
                        // float diffuse = saturate(cos(theta + a));
                        float diffuse = saturate(cos(b) * cos(theta + a));

                        float3 weight = RadianceProfile(sampleDist);
                        totalLight += diffuse * weight;
                        totalWeights += weight;
                        a += 0.05;
                    }
                    b += 0.05;
                }
                // #if _TONE_UNCHARTED
                //     float3 rgb = Uncharted2ToneMapping(totalLight/totalWeights);
                // #elif _TONE_ACES
                //     float3 rgb = ACESToneMapping(totalLight/totalWeights);
                // #else
                    float3 rgb = totalLight/totalWeights;
                // #endif

                #if _GAMMA_ON
                    rgb = pow(rgb, 1/2.2);//转换到Gamma空间
                #endif
                return rgb;
            }
            fixed4 frag (v2f i) : SV_Target
            {
                float radians = 1.0/((i.uv.y * _Max1R+0.0001));

                float3 rgb = IntegrateDiffuseScatteringOnRing(lerp(-1,1,i.uv.x),radians);
                return float4(rgb,1);
            }
            ENDCG
        }
    }
}

```

## Source file: LineChart.shader

{
  "path": "LineChart.shader",
  "blob_sha": "8534574bc353a8410f4f29d1d081b31eacdeef29",
  "sha256": "618800c62b863725509cf7309dcdd10e62bc67fae7e535e9238907ff5461b3e9",
  "lines": 40,
  "bytes": 1238,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "shader": 2
  },
  "url": "https://github.com/DENO1017/PSSSLutInUnity/blob/1bbe95c13e6371495442578bcd2d062d12c7ef03/LineChart.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "SSS/LineChart"
{
    Properties
    {
        _LineWidth ("Line Width", Range(0.001,0.1)) = 0.001
        _Axis ("AxisXY", Vector) = (1,1,1,1)
        [KeywordEnum(NormDiff, G1, G2)] _Radiance ("Radiance Profile type", Float) = 0
        _D ("D", Vector) = (1,1,1,1)
        _DiffColor1 ("NormDiff Color 1", Color) = (1,1,1,1)
        _DiffColor2 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor3 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor4 ("NormDiff Color 2", Color) = (0,0,0,0)
    }
    SubShader
    {
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma multi_compile _RADIANCE_NORMDIFF _RADIANCE_G1 _RADIANCE_G2

            #include "SSSCore.cginc"
            float _LineWidth;
            float4 _Axis;


            fixed4 frag (v2f i) : SV_Target
            {
                float2 axis = i.uv.xy * _Axis.xy;
                fixed3 rgb = RadianceProfile(axis.x);
                rgb = _LineWidth / abs(axis.x * rgb / axis.y - 1) / i.uv.y;
                return fixed4(rgb,1);
            }
            ENDCG
        }
    }
}


```

## Source file: DiffuseProfile.shader

{
  "path": "DiffuseProfile.shader",
  "blob_sha": "5d4a90edf9219250463d11ba534c2184cd002aaa",
  "sha256": "6385b959f9ec07122f8cf89823c0b76c17ad0c07b335c59919704c8974a2c051",
  "lines": 37,
  "bytes": 1137,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "shader": 2
  },
  "url": "https://github.com/DENO1017/PSSSLutInUnity/blob/1bbe95c13e6371495442578bcd2d062d12c7ef03/DiffuseProfile.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "SSS/DiffuseProfile"
{
    Properties
    {
        _MaxRadiance("Max Radiance", float) = 1
        [KeywordEnum(NormDiff, G1, G2)] _Radiance ("Radiance Profile type", Float) = 0
        _D ("D", Vector) = (1,1,1,1)
        _DiffColor1 ("NormDiff Color 1", Color) = (1,1,1,1)
        _DiffColor2 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor3 ("NormDiff Color 2", Color) = (0,0,0,0)
        _DiffColor4 ("NormDiff Color 2", Color) = (0,0,0,0)
    }
    SubShader
    {
        // No culling or depth
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma multi_compile _RADIANCE_NORMDIFF _RADIANCE_G1 _RADIANCE_G2

            #include "SSSCore.cginc"
            float _MaxRadiance;

            fixed4 frag (v2f i) : SV_Target
            {
                float2 distance = length(i.uv.xy - 0.5) * _MaxRadiance;
                fixed3 rgb = RadianceProfile(distance);
                return fixed4(rgb,1);
            }
            ENDCG
        }
    }
}


```

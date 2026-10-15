# Facial rendering source scan: wingstone/PreintegratedSubsurfaceScattering

Repository: https://github.com/wingstone/PreintegratedSubsurfaceScattering

Pinned source revision: 7da53e317711bef8b44a7cf17e107351647e968e

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/Human/Resources/Shaders/PreIntegratedSkin.shader", "Assets/Human/Resources/Shaders/PreIntegratedSkin.shader.meta", "Assets/Human/Texture/lut/PreintegratedLookupSkin.png", "Assets/Human/Texture/lut/PreintegratedLookupSkin.png.meta", "Assets/Human/Texture/lut/PreintegratedLookupSkinLinear.png", "Assets/Human/Texture/lut/PreintegratedLookupSkinLinear.png.meta"], "subsurface": ["Images/PreintegratedSubsurfaceScattering.png"], "shader": ["Assets/Human/Resources/Shaders.meta", "Assets/Human/Resources/Shaders/IndirectLight.shader", "Assets/Human/Resources/Shaders/IndirectLight.shader.meta", "Assets/Human/Resources/Shaders/LookupTexture.shader", "Assets/Human/Resources/Shaders/LookupTexture.shader.meta", "Assets/Human/Resources/Shaders/MonteCarloProject.shader", "Assets/Human/Resources/Shaders/MonteCarloProject.shader.meta", "Assets/Human/Resources/Shaders/PreIntegratedSkin.shader", "Assets/Human/Resources/Shaders/PreIntegratedSkin.shader.meta", "Assets/Human/Resources/Shaders/ProfileDisplay.shader", "Assets/Human/Resources/Shaders/ProfileDisplay.shader.meta", "Assets/Human/Resources/Shaders/Reduce_MC_1024.compute", "Assets/Human/Resources/Shaders/Reduce_MC_1024.compute.meta", "Assets/Human/Resources/Shaders/Reduce_Uniform.compute", "Assets/Human/Resources/Shaders/Reduce_Uniform.compute.meta", "Assets/Human/Resources/Shaders/SH_Utils.cginc", "Assets/Human/Resources/Shaders/SH_Utils.cginc.meta"], "face": ["Images/PreintegratedSubsurfaceScattering.png"]}

## Source file: Assets/Human/Resources/Shaders/PreIntegratedSkin.shader

{
  "path": "Assets/Human/Resources/Shaders/PreIntegratedSkin.shader",
  "blob_sha": "f5151a9a5e7f881dc9d35ea39d0541692bf72ed3",
  "sha256": "66863fa957b3756e0d2f9f48da6d7dc11193058446eb47260d0fcb2d96e7907e",
  "lines": 241,
  "bytes": 9745,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 10,
    "shader": 6,
    "lip": 1
  },
  "url": "https://github.com/wingstone/PreintegratedSubsurfaceScattering/blob/7da53e317711bef8b44a7cf17e107351647e968e/Assets/Human/Resources/Shaders/PreIntegratedSkin.shader",
  "function_signature_candidates": [
    {
      "line": 97,
      "text": "float GSquare(float x)"
    },
    {
      "line": 102,
      "text": "float CurvatureFromLight("
    },
    {
      "line": 124,
      "text": "float3 SkinNol(float old_nol, float curvature)"
    },
    {
      "line": 130,
      "text": "float3 SkinNol(float old_nol, float old_bluredNol, float curvature)"
    },
    {
      "line": 137,
      "text": "float3 SkinShadow(float atten, float inv_width)"
    },
    {
      "line": 142,
      "text": "float ZHRed0Approx(float cur, float cur2, float cur3, float cur4)"
    },
    {
      "line": 146,
      "text": "float ZHRed1Approx(float cur, float cur2, float cur3, float cur4)"
    },
    {
      "line": 150,
      "text": "float ZHRed2Approx(float cur, float cur2, float cur3, float cur4)"
    },
    {
      "line": 161,
      "text": "float4 frag (v2f i) : SV_Target"
    }
  ]
}

```text

﻿Shader "Human/PreIntegratedShader(no specular)"
{
    Properties
    {
        _MainTex ("Main Texture", 2D) = "white" {}
        _NormalTex ("Normal Texture", 2D) = "bump" {}
        _OcclusionTex ("Occlusion Texture", 2D) = "white" {}
        _PreIntegratedSkinTex ("PreIntegrated Skin Texture", 2D) = "black" {}
        _PreIntegratedShadowTex ("PreIntegrated Shadow Texture", 2D) = "black" {}
        _SHProfile ("SH Profile", 2D) = "black" {}
        _CurvatureScale("Curvature Scale", Range(0, 10)) = 5
        
        // 当前无法从unity获取半影宽度，需要使用light的frustom的widt除以shadowmap的resolution，乘以kernel的半径
        // 因此这里以参数数值手动匹配当前环境
        _InvPenumbraWidth("Inv Penumbra Width", Range(0.0001, 1)) = 0.117
        _BluredNormalLevel("Blured normal level", Range(0, 10)) = 0
    }
    SubShader
    {
        Tags { "LightMode"="ForwardBase" "RenderType"="Opaque" }
        LOD 100

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag

            #pragma multi_compile_fwdbase

            #include "UnityCG.cginc"        //常用函数，宏，结构体
            #include "Lighting.cginc"		//光源相关变量
            #include "AutoLight.cginc"		//光照，阴影相关宏，函数
            #include "SH_Utils.cginc"

            struct appdata
            {
                float4 color : COLOR;
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
                float3 normal : NORMAL;
                float4 tangent : TANGENT;
            };

            struct v2f
            {
                float4 color : COLOR;
                float2 uv : TEXCOORD0;
                float3 worldPos : TEXCOORD1;
                float3 tangent : TEXCOORD2;
                float3 binormal : TEXCOORD3;
                float3 normal : TEXCOORD4;
                UNITY_LIGHTING_COORDS(5, 6)
                float4 pos : SV_POSITION;
            };

            sampler2D _MainTex;
            sampler2D _NormalTex;
            sampler2D _OcclusionTex;
            sampler2D _PreIntegratedSkinTex;
            sampler2D _PreIntegratedShadowTex;
            sampler2D _SHProfile;
            float _CurvatureScale;
            float _InvPenumbraWidth;
            float _BluredNormalLevel;
            
			float4 c0;
			float4 c1;
			float4 c2;
			float4 c3;
			float4 c4;
			float4 c5;
			float4 c6;
			float4 c7;
			float4 c8;

            v2f vert (appdata v)
            {
                v2f o = (v2f)0;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.worldPos = mul(UNITY_MATRIX_M, v.vertex).xyz;
                o.uv = v.uv;
                o.color = v.color;

                o.normal = UnityObjectToWorldNormal(v.normal);
                o.tangent = UnityObjectToWorldDir(v.tangent.xyz);

                float3x3 tangentToWorld = CreateTangentToWorldPerVertex(o.normal, o.tangent, v.tangent.w);
                o.tangent = tangentToWorld[0];
                o.binormal = tangentToWorld[1];
                o.normal = tangentToWorld[2];

                UNITY_TRANSFER_LIGHTING(o, v.uv);
                return o;
            }

            float GSquare(float x)
            {
                return x*x;
            }
            
            float CurvatureFromLight(
            float3 tangent,
            float3 bitangent,
            float3 curvTensor,
            float3 lightDir)
            {
                // Project light vector into tangent plane
                
                float2 lightDirProj = float2(dot(lightDir, tangent), dot(lightDir, bitangent));
                
                // NOTE (jasminp) We should normalize lightDirProj here in order to correctly
                //    calculate curvature in the light direction projected to the tangent plane.
                //    However, it makes no perceptible difference, since the skin LUT does not vary
                //    much with curvature when N.L is large.
                
                float curvature = curvTensor.x * GSquare(lightDirProj.x) +
                2.0f * curvTensor.y * lightDirProj.x * lightDirProj.y +
                curvTensor.z * GSquare(lightDirProj.y);
                
                return curvature;
            }

            float3 SkinNol(float old_nol, float curvature)
            {
                return tex2D(_PreIntegratedSkinTex, float2(old_nol*0.5+0.5, curvature)).rgb;
            }

            // g,b channel shold use different mip level, we use 0 level, because it is not important
            float3 SkinNol(float old_nol, float old_bluredNol, float curvature)
            {
                float3 lut = tex2D(_PreIntegratedSkinTex, float2(old_nol*0.5+0.5, curvature)).rgb;
                float3 bluredlut = tex2D(_PreIntegratedSkinTex, float2(old_bluredNol*0.5+0.5, curvature)).rgb;
                return float3(bluredlut.r, lut.gb);
            }

            float3 SkinShadow(float atten, float inv_width)
            {
                return tex2D(_PreIntegratedShadowTex, float2(atten, inv_width)).rgb;
            }

            float ZHRed0Approx(float cur, float cur2, float cur3, float cur4) 
            {
                return 1.04297 - 0.0395131*cur + 0.854299*cur2 - 0.92605*cur3 + 0.308906*cur4; 
            }
            float ZHRed1Approx(float cur, float cur2, float cur3, float cur4) 
            {
                return 0.614122 + 0.0177265*cur - 0.283127*cur2 + 0.254445*cur3 - 0.0657637*cur4; 
            }
            float ZHRed2Approx(float cur, float cur2, float cur3, float cur4) 
            {
                return 0.105369 - 0.000255739*cur - 0.523248*cur2 + 0.634608*cur3 -0.234107*cur4;
            } 
            #define ZHGreen0Approx 1.05
            #define ZHGreen1Approx 0.611
            #define ZHGreen2Approx 0.098 
            #define ZHBlue0Approx 1.045
            #define ZHBluelApprox 0.6135 
            #define ZHBlue2Approx 0.1 

            float4 frag (v2f i) : SV_Target
            {
                // sample the texture
                float4 albedo = tex2D(_MainTex, i.uv);
                float3 diffcolor = unity_ColorSpaceDielectricSpec.a*albedo;
                float3 speccolor = unity_ColorSpaceDielectricSpec.rgb;
                float3 texNormal = UnpackNormal(tex2D(_NormalTex, i.uv));
                float3 bluredTexNormal = UnpackNormal(tex2Dlod(_NormalTex, float4(i.uv, 0, _BluredNormalLevel)));
                float occlusion = tex2D(_OcclusionTex, i.uv).r;
                
                float3 N = normalize(texNormal.x*i.tangent + texNormal.y*i.binormal + texNormal.z*i.normal);
                float3 bluredN = normalize(bluredTexNormal.x*i.tangent + bluredTexNormal.y*i.binormal + bluredTexNormal.z*i.normal);
                float3 L = _WorldSpaceLightPos0.xyz;
                float old_nol = dot(N, L);
                float old_bluredNol = dot(bluredN, L);

                float3 col = 0;

                //shadow
                UNITY_LIGHT_ATTENUATION(atten, i, i.worldPos);
                float3 lightcolor = _LightColor0.rgb;
                
                //diffuse
                // float meanCurv = i.color.a*_CurvatureScale;
                float dirCurv = CurvatureFromLight(i.tangent, i.binormal, i.color.rgb, L)*_CurvatureScale;
                float3 diffuse = lightcolor * SkinNol(old_nol, old_bluredNol, dirCurv) * SkinShadow(atten, _InvPenumbraWidth);
                col += diffcolor * diffuse;

                //ambient
                float3 zh0 = tex2Dlod(_SHProfile,float4(dirCurv, 0.166666, 0, 0));
                float3 zh1 = tex2Dlod(_SHProfile,float4(dirCurv, 0.5, 0, 0));
                float3 zh2 = tex2Dlod(_SHProfile,float4(dirCurv, 0.833333, 0, 0));
                // float cur = curvature;
                // float cur2 = cur * cur;
                // float cur3 = cur2*cur;
                // float cur4 = cur2*cur2;
                // float3 zh0 = float3(ZHRed0Approx(cur, cur2,cur3,cur4), ZHGreen0Approx, ZHBlue0Approx);
                // float3 zh1 = float3(ZHRed1Approx(cur, cur2,cur3,cur4), ZHGreen1Approx, ZHBluelApprox);
                // float3 zh2 = float3(ZHRed2Approx(cur, cur2,cur3,cur4), ZHGreen2Approx, ZHBlue2Approx);
                float3 ambient = zh0 * c0 * Y0(N) + 
                    zh1 * (c1 * Y1(N) + c2 * Y2(N) + c3 * Y3(N)) + 
                    zh2 * (c4 * Y4(N) + c5 * Y5(N) + c6 * Y6(N) + c7 * Y7(N) + c8 * Y8(N));
                // ambient *= UNITY_INV_PI; // premultiply in zh
                col += diffcolor * ambient;

                return float4(col, 1);
            }
            ENDCG
        }

        Pass
        {
            //copy from unity standard shadowcaster
            Name "ShadowCaster"
            Tags { "LightMode" = "ShadowCaster" }

            ZWrite On ZTest LEqual

            CGPROGRAM
            #pragma target 3.0

            // -------------------------------------

            #pragma shader_feature _ _ALPHATEST_ON _ALPHABLEND_ON _ALPHAPREMULTIPLY_ON
            #pragma shader_feature _SPECGLOSSMAP
            #pragma shader_feature _SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A
            #pragma shader_feature _PARALLAXMAP
            #pragma multi_compile_shadowcaster
            #pragma multi_compile_instancing
            // Uncomment the following line to enable dithering LOD crossfade. Note: there are more in the file to uncomment for other passes.
            //#pragma multi_compile _ LOD_FADE_CROSSFADE

            #pragma vertex vertShadowCaster
            #pragma fragment fragShadowCaster

            #include "UnityStandardShadow.cginc"

            ENDCG
        }
    }
}


```

## Source file: Assets/Human/Resources/Shaders/SH_Utils.cginc

{
  "path": "Assets/Human/Resources/Shaders/SH_Utils.cginc",
  "blob_sha": "bc40ad750a595f98ebfff654c7fa7ba33cc23e05",
  "sha256": "9e2ee457ae838466b13dbd2d506616c034ffb222916135f8c5c1324da16bf8b1",
  "lines": 113,
  "bytes": 1761,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/wingstone/PreintegratedSubsurfaceScattering/blob/7da53e317711bef8b44a7cf17e107351647e968e/Assets/Human/Resources/Shaders/SH_Utils.cginc",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "float AreaElement(float x, float y)"
    },
    {
      "line": 6,
      "text": "float DifferentialSolidAngle(float textureSize, float2 uv)"
    },
    {
      "line": 18,
      "text": "float rand(float2 n)"
    },
    {
      "line": 23,
      "text": "float Y0(float3 v)"
    },
    {
      "line": 28,
      "text": "float Y1(float3 v)"
    },
    {
      "line": 33,
      "text": "float Y2(float3 v)"
    },
    {
      "line": 38,
      "text": "float Y3(float3 v)"
    },
    {
      "line": 43,
      "text": "float Y4(float3 v)"
    },
    {
      "line": 48,
      "text": "float Y5(float3 v)"
    },
    {
      "line": 53,
      "text": "float Y6(float3 v)"
    },
    {
      "line": 58,
      "text": "float Y7(float3 v)"
    },
    {
      "line": 63,
      "text": "float Y8(float3 v)"
    },
    {
      "line": 69,
      "text": "float3 RfromUV(uint face, float u, float v)"
    }
  ]
}

```text

float AreaElement(float x, float y)
{
	return atan2(x * y, sqrt(x * x + y * y + 1));
}

float DifferentialSolidAngle(float textureSize, float2 uv)
{
	float inv = 1.0 / textureSize;
	float u = 2.0 * (uv.x + 0.5 * inv) - 1;
	float v = 2.0 * (uv.y + 0.5 * inv) - 1;
	float x0 = u - inv;
	float y0 = v - inv;
	float x1 = u + inv;
	float y1 = v + inv;
	return AreaElement(x0, y0) - AreaElement(x0, y1) - AreaElement(x1, y0) + AreaElement(x1, y1);
}

float rand(float2 n)
{
	return frac(sin(dot(n, float2(12.9898, 4.1414))) * 43758.5453);
}

float Y0(float3 v)
{
	return 0.2820947917f;
}

float Y1(float3 v)
{
	return 0.4886025119f * v.y;
}

float Y2(float3 v)
{
	return 0.4886025119f * v.z;
}

float Y3(float3 v)
{
	return 0.4886025119f * v.x;
}

float Y4(float3 v)
{
	return 1.0925484306f * v.x * v.y;
}

float Y5(float3 v)
{
	return 1.0925484306f * v.y * v.z;
}

float Y6(float3 v)
{
	return 0.3153915652f * (3.0f * v.z * v.z - 1.0f);
}

float Y7(float3 v)
{
	return 1.0925484306f * v.x * v.z;
}

float Y8(float3 v)
{
	return 0.5462742153f * (v.x * v.x - v.y * v.y);
}

//GET TEXEL DIRECTION VECTOR FROM UV
float3 RfromUV(uint face, float u, float v)
{
	float3 dir;

	switch (face)
	{
	case 0: //+X
		dir.x = 1;
		dir.y = v * -2.0f + 1.0f;
		dir.z = u * -2.0f + 1.0f;
		break;

	case 1: //-X
		dir.x = -1;
		dir.y = v * -2.0f + 1.0f;
		dir.z = u * 2.0f - 1.0f;
		break;

	case 2: //+Y
		dir.x = u * 2.0f - 1.0f;
		dir.y = 1.0f;
		dir.z = v * 2.0f - 1.0f;
		break;

	case 3: //-Y
		dir.x = u * 2.0f - 1.0f;
		dir.y = -1.0f;
		dir.z = v * -2.0f + 1.0f;
		break;

	case 4: //+Z
		dir.x = u * 2.0f - 1.0f;
		dir.y = v * -2.0f + 1.0f;
		dir.z = 1;
		break;

	case 5: //-Z
		dir.x = u * -2.0f + 1.0f;
		dir.y = v * -2.0f + 1.0f;
		dir.z = -1;
		break;
	}

	return dir;
}

```

## Source file: Assets/Human/Resources/Shaders/IndirectLight.shader

{
  "path": "Assets/Human/Resources/Shaders/IndirectLight.shader",
  "blob_sha": "6fcf754a9cbe4aaa52e59f5610df33ac780795d7",
  "sha256": "def48625fae68009b88b9c4904502729275d701caf07558cfb5c246377f38041",
  "lines": 75,
  "bytes": 1897,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/wingstone/PreintegratedSubsurfaceScattering/blob/7da53e317711bef8b44a7cf17e107351647e968e/Assets/Human/Resources/Shaders/IndirectLight.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Human/IndirectLight"
{
    Properties
    {
        [IntRange]_Mode("sh or gi", Range(0, 1)) = 0
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

            #include "UnityCG.cginc"
            #include "SH_Utils.cginc"

            struct appdata
            {
                float4 vertex : POSITION;
                float3 normal : NORMAL;
            };

            struct v2f
            {
                float3 normal : NORMAL;
                float4 vertex : SV_POSITION;
            };

            v2f vert (appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.normal = UnityObjectToWorldNormal(v.normal);
                return o;
            }
                
			float4 c0;
			float4 c1;
			float4 c2;
			float4 c3;
			float4 c4;
			float4 c5;
			float4 c6;
			float4 c7;
			float4 c8;
            int _Mode;

            fixed4 frag (v2f i) : SV_Target
            {
                float3 n = normalize(i.normal);
                float4 col = 0;
                float3 cosZH = float3(3.141593, 2.094395, 0.785398);
                
                if (_Mode == 0)
                    col = c0 * Y0(n) + c1 * Y1(n) + c2 * Y2(n) + c3 * Y3(n) + c4 * Y4(n) + c5 * Y5(n) + c6 * Y6(n) + c7 * Y7(n) + c8 * Y8(n);

                if (_Mode == 1)
                {
                    col = cosZH.x * c0 * Y0(n) + 
                        cosZH.y * (c1 * Y1(n) + c2 * Y2(n) + c3 * Y3(n)) + 
                        cosZH.z * (c4 * Y4(n) + c5 * Y5(n) + c6 * Y6(n) + c7 * Y7(n) + c8 * Y8(n));
                    col *= UNITY_INV_PI;
                }   

                col.a = 1;
                return col;
            }
            ENDCG
        }
    }
}


```

## Source file: Assets/Human/Resources/Shaders/LookupTexture.shader

{
  "path": "Assets/Human/Resources/Shaders/LookupTexture.shader",
  "blob_sha": "7896637918f9188b7e68333edf359a075c70b9c4",
  "sha256": "32e71f2ca127191e6fc1e30c45bc4348fa541fbd688d37a47c23e4ff574526be",
  "lines": 341,
  "bytes": 10499,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 17,
    "shader": 2,
    "lip": 4
  },
  "url": "https://github.com/wingstone/PreintegratedSubsurfaceScattering/blob/7da53e317711bef8b44a7cf17e107351647e968e/Assets/Human/Resources/Shaders/LookupTexture.shader",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "float Gaussian (float v, float r)"
    },
    {
      "line": 32,
      "text": "float3 RadialScatter(float r)"
    },
    {
      "line": 42,
      "text": "float3 LinearSactterLut(float r)"
    },
    {
      "line": 47,
      "text": "// float newPenumbra(float pos, float penumbraWidth)"
    },
    {
      "line": 53,
      "text": "float newPenumbra(float pos, float penumbraWidth)"
    },
    {
      "line": 60,
      "text": "float3 integrateShadowScattering(float penumbraLocation, float penumbraWidth)"
    },
    {
      "line": 82,
      "text": "float3 integrateDiffuseScatteringOnRing(float cosTheta , float skinRadius)"
    },
    {
      "line": 111,
      "text": "float3 LinearSactter(float r)"
    },
    {
      "line": 159,
      "text": "float4 frag(v2f i) : SV_Target"
    },
    {
      "line": 200,
      "text": "float4 frag(v2f i) : SV_Target"
    },
    {
      "line": 241,
      "text": "float4 frag(v2f i) : SV_Target"
    },
    {
      "line": 281,
      "text": "float3 SkinNol(float nol, float curvature)"
    },
    {
      "line": 286,
      "text": "float4 frag(v2f i) : SV_Target"
    }
  ]
}

```text

﻿Shader "Human/LookupTexture"
{
    Properties
    {
        _Use_Linear_Profile("use linear profile", Int) = 0
        _Linear_Profile("linear profile", 2D) = "gray" {}
        _SkinLut("skin lut", 2D) = "gray" {}
    }
    SubShader
    {
        // No culling or depth
        Cull Off ZWrite Off ZTest Always

        CGINCLUDE
        #include "UnityCG.cginc"        //常用函数，宏，结构体
        #include "SH_Utils.cginc"
        
        // 50mm范围lut
        #define linearProfileLength 50
        // 3倍方差来代替
        #define PROFILE_WIDTH 8.166

        int _Use_Linear_Profile;
        sampler2D _Linear_Profile;
        sampler2D _SkinLut;
        
        float Gaussian (float v, float r)
        {
            return 1.0/sqrt(2.0*UNITY_PI*v)*exp(-(r*r)/(2*v));
        }

        float3 RadialScatter(float r)
        {
            return Gaussian(0.0064*1.414, r)*float3(0.233,0.455,0.649)+
            Gaussian(0.0484*1.414, r)*float3(0.100,0.336,0.344)+
            Gaussian(0.1870*1.414, r)*float3(0.118,0.198,0.000)+
            Gaussian(0.5670*1.414, r)*float3(0.113,0.007,0.007)+
            Gaussian(1.9900*1.414, r)*float3(0.358,0.004,0.000)+
            Gaussian(7.4100*1.414, r)*float3(0.078,0.000,0.000);
        }

        float3 LinearSactterLut(float r)
        {
            return tex2D(_Linear_Profile, float2(r/linearProfileLength, 0.5)).rgb;
        }

        // float newPenumbra(float pos, float penumbraWidth)
        // {
        //     return saturate(pos*2-1);
        // }
        
        // 散射范围为PROFILE_WIDTH
        float newPenumbra(float pos, float penumbraWidth)
        {
            return saturate((pos*penumbraWidth -PROFILE_WIDTH) / (penumbraWidth -PROFILE_WIDTH));
        }

        //penumbraLocation为归一化后的在半影区域内的位置；
        // 这里的反函数是有些问题的，unity使用的是tent filter，并不是box filter
        float3 integrateShadowScattering(float penumbraLocation, float penumbraWidth)
        {
            float3 totalWeights = 0;
            float3 totalLight = 0;
            float inc = 0.001;

            penumbraWidth = max(penumbraWidth, PROFILE_WIDTH + 1e-5);
            
            float a = -PROFILE_WIDTH;
            while(a <= PROFILE_WIDTH)   //理论上积分区域应从负无穷到正无穷，这里以3倍方差来代替
            {
                float light = newPenumbra(penumbraLocation+a/penumbraWidth, penumbraWidth);
                float sampleDist = abs(a);
                float3 weights = RadialScatter(sampleDist);
                totalWeights += weights;
                totalLight += light*weights;
                a+=inc;
            }

            return totalLight/totalWeights;
        }

        float3 integrateDiffuseScatteringOnRing(float cosTheta , float skinRadius)
        {
            // Angle from lighting direction.
            float theta = acos(cosTheta);
            float3 totalWeights = 0;
            float3 totalLight = 0;
            float a= -UNITY_HALF_PI;   // 正常来说积分范围应该从-pi到pi，不过用Gaussian profile的积分结果有蓝色成分，所以这里仍然采用-pi/2到pi/2;
            float inc = 0.001;

            while(a <= UNITY_HALF_PI)
            {
                float sampleAngle = theta + a;
                float diffuse= saturate(cos(sampleAngle));
                float sampleDist = abs(2.0*skinRadius*sin(a*0.5)); // Distance.

                float3 weights = 0;
                if(_Use_Linear_Profile == 1)
                    weights = LinearSactterLut(sampleDist);
                else
                    weights = RadialScatter(sampleDist);

                // Profile Weight.
                totalWeights += weights;
                totalLight += diffuse*weights;
                a+=inc;
            }
            return totalLight/totalWeights;
        }

        float3 LinearSactter(float r)
        {
            // 理论上积分范围应该从负无穷到正无穷
            float a = -50;
            int count = 50;
            float3 scatter = 0;

            while(a < 50)
            {
                float len = length(float2(r, a));
                scatter += RadialScatter(len);
                a += 0.01;
                count++;
            }
            return scatter/count*100;
        }

        ENDCG

        // 0 shadow lut
        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag

            #include "UnityCG.cginc"

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

            float4 frag(v2f i) : SV_Target
            {
                float3 col = 0;
                float penumbraLocation = i.uv.x;
                float penumbraWidth = 1/i.uv.y;
                col = integrateShadowScattering(penumbraLocation, penumbraWidth);
                return float4(col, 1);
            }
            ENDCG
        }

        // pass 1 skin lut
        Pass
        {
            CGPROGRAM
            
            #pragma vertex vert
            #pragma fragment frag

            #include "UnityCG.cginc"

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

            v2f vert(appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            float4 frag(v2f i) : SV_Target
            {
                float3 col = 0;
                float cosTheta = i.uv.x*2-1;
                float skinRadius = clamp(1/i.uv.y, 1e-5, 100);  //限制下半径范围, 单位mm
                col = integrateDiffuseScatteringOnRing(cosTheta, skinRadius);
                return float4(col, 1);
            }
            ENDCG
        }

        // pass 2 linear profile
        Pass
        {
            CGPROGRAM
            
            #pragma vertex vert
            #pragma fragment frag

            #include "UnityCG.cginc"

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

            v2f vert(appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            float4 frag(v2f i) : SV_Target
            {
                float3 col = 0;
                float skinRadius = i.uv.x;
                col = LinearSactter(skinRadius*linearProfileLength);
                return float4(col, 1);
            }
            ENDCG
        }

        // pass 3 SH profile
        Pass
        {
            CGPROGRAM
            
            #pragma vertex vert
            #pragma fragment frag

            #include "UnityCG.cginc"

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

            v2f vert(appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            float3 SkinNol(float nol, float curvature)
            {
                return tex2Dlod(_SkinLut, float4(nol*0.5 + 0.5, curvature, 0, 0)).rgb;
            }

            float4 frag(v2f i) : SV_Target
            {
                float3 col = 0;
                float3 integral = 0;
                int count = 0;
                float theta = 0;

                // ZH0
                if (i.uv.y < 0.33333)
                {
                    while(theta < UNITY_PI) //应该从0到pi积分，而不是pi/2
                    {
                        float nol = saturate(cos(theta));
                        float y0 = Y0(float3(0,0,nol));     // zh的计算只跟z有关
                        float sinTheta = sqrt(1 - nol*nol);
                        float3 skinNol = SkinNol(nol, i.uv.x);
                        integral += y0*sinTheta*skinNol;
                        theta += 0.001;
                        count++;
                    }
                    col = integral / count * UNITY_PI * sqrt(UNITY_FOUR_PI)*UNITY_TWO_PI * UNITY_INV_PI;
                }
                else if (i.uv.y < 0.666666)
                {
                    while(theta < UNITY_PI)
                    {
                        float nol = saturate(cos(theta));
                        float y2 = Y2(float3(0,0,nol));
                        float sinTheta = sqrt(1 - nol*nol);
                        float3 skinNol = SkinNol(nol, i.uv.x);
                        integral += y2*sinTheta*skinNol;
                        theta += 0.001;
                        count++;
                    }
                    col = integral / count * UNITY_PI * sqrt(UNITY_FOUR_PI/3)*UNITY_TWO_PI * UNITY_INV_PI;
                }
                else
                {
                    while(theta < UNITY_PI)
                    {
                        float nol = saturate(cos(theta));
                        float y6 = Y6(float3(0,0,nol));
                        float sinTheta = sqrt(1 - nol*nol);
                        float3 skinNol = SkinNol(nol, i.uv.x);
                        integral += y6*sinTheta*skinNol;
                        theta += 0.001;
                        count++;
                    }
                    col = integral / count * UNITY_PI * sqrt(UNITY_FOUR_PI/5)*UNITY_TWO_PI * UNITY_INV_PI;
                }
                return float4(col, 1);
            }
            ENDCG
        }
    }
}


```

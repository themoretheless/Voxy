# Facial rendering source scan: wjw12/Fast-Approximate-SSS

Repository: https://github.com/wjw12/Fast-Approximate-SSS

Pinned source revision: b1a1482fe96cb1b6f068d30524870cc90336806c

Discovery topic: subsurface scattering

Description: Approximate Subsurface Scattering

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": [".vs/SSS/v15/.suo", "Assets/Materials/SSS.mat", "Assets/Materials/SSS.mat.meta", "Assets/Materials/Shaders/SSS.shader", "Assets/Materials/Shaders/SSS.shader.meta", "SSS.csproj", "SSS.sln"], "shader": ["Assets/Materials/Shaders.meta", "Assets/Materials/Shaders/SSS.shader", "Assets/Materials/Shaders/SSS.shader.meta", "Library/ShaderCache.db", "Library/ShaderCache/0/0147ed9c3cee11a326f506028f4bb2db.bin", "Library/ShaderCache/0/0151c76bb30ca01dd50bbff0fa8320fc.bin", "Library/ShaderCache/0/01b659d786012cd96174e3cec2ca14da.bin", "Library/ShaderCache/0/0257b3e4f017cacf80f46105852cf163.bin", "Library/ShaderCache/0/037b9f281f9d58b1c667917c866f0cb4.bin", "Library/ShaderCache/0/05662ed1775baeac2bdcbc9e1ad03f69.bin", "Library/ShaderCache/0/06376e4a7969f63b00405c67c399e330.bin", "Library/ShaderCache/0/06c351e9ee16534e044f05b69c85eaf9.bin", "Library/ShaderCache/0/082c739a0fa945f88be9994e5dbc9ad4.bin", "Library/ShaderCache/0/0a8b96f7d0689175a3bfd8f2159d2f34.bin", "Library/ShaderCache/0/0aa6963e824547eef1566ff87373fc06.bin", "Library/ShaderCache/0/0b6c4648da588931137caa379aed7bc8.bin", "Library/ShaderCache/0/0d6e363caa3b96d4059f38a56e5947bc.bin", "Library/ShaderCache/0/0e887e8586851374fe0c07560ef53c42.bin", "Library/ShaderCache/0/0f8c20305a58ed2868889c9a8eaf2e68.bin", "Library/ShaderCache/1/1081a244abfe2c53d6837e1d430fd82b.bin", "Library/ShaderCache/1/123c11cba793881c92613b12052d31b9.bin", "Library/ShaderCache/1/130822afca8da3bcd8fb6be781e99576.bin", "Library/ShaderCache/1/14aa26038372b02149db40b0c232717f.bin", "Library/ShaderCache/1/155be030bd1c6cbc764ce95902cc348a.bin", "Library/ShaderCache/1/1683722949552ae2ced7238350ab0ea1.bin", "Library/ShaderCache/1/169785a86726a82ea70eb9234fb30234.bin", "Library/ShaderCache/1/17314e2fa0db141f299804740de81d46.bin", "Library/ShaderCache/1/17dfb9f33a163dfc4726b729c62eea12.bin", "Library/ShaderCache/1/194a2ebe43d2e6b64b196c7616fbaac4.bin", "Library/ShaderCache/1/19d47b31ba270ad78f86ed58063e7b04.bin"]}

## Source file: Assets/Materials/Shaders/SSS.shader

{
  "path": "Assets/Materials/Shaders/SSS.shader",
  "blob_sha": "96e0072309cf99e908fa8c699579ddfae132ad59",
  "sha256": "7ef83d04570355d17c2c1c7c82d99f854bc51fcd6ab4c9e03ef14680359a3fb8",
  "lines": 80,
  "bytes": 2193,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 4,
    "shader": 5,
    "rig": 1,
    "face": 5
  },
  "url": "https://github.com/wjw12/Fast-Approximate-SSS/blob/b1a1482fe96cb1b6f068d30524870cc90336806c/Assets/Materials/Shaders/SSS.shader",
  "function_signature_candidates": [
    {
      "line": 51,
      "text": "void LightingSSSTranslucent_GI(SurfaceOutputStandard s, UnityGIInput data, inout UnityGI gi)"
    },
    {
      "line": 68,
      "text": "void surf (Input IN, inout SurfaceOutputStandard o) {"
    }
  ]
}

```text

﻿Shader "Custom/SSS" {
	Properties {
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		_Distortion ("Distortion", Range(0,1)) = 0.5
		_Power ("Power", Range(0,5)) = 1.0
		_Scale ("Scale", Range(0,5)) = 1.0
	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200

		CGPROGRAM
		// Physically based Standard lighting model, and enable shadows on all light types
		#pragma surface surf SSSTranslucent fullforwardshadows

		// Use shader model 3.0 target, to get nicer looking lighting
		#pragma target 3.0

		sampler2D _MainTex;

		struct Input {
			float2 uv_MainTex;
		};

		half _Glossiness;
		half _Metallic;
		fixed4 _Color;
		half _Distortion;
		half _Power;
		half _Scale;

		#include "UnityPBSLighting.cginc"
		inline fixed4 LightingSSSTranslucent(SurfaceOutputStandard s, float3 viewDir, UnityGI gi)
		{
			// original color 
			fixed4 c = LightingStandard(s, viewDir, gi);

			float3 lightDir = gi.light.dir;
			float3 normal = s.Normal;

			float3 H = lightDir + _Distortion * normal;
			float VdotH = pow(saturate(dot(viewDir, -H)), _Power) * _Scale;
 
			c.rgb += gi.light.color * VdotH;
			return c;
		}
 
		void LightingSSSTranslucent_GI(SurfaceOutputStandard s, UnityGIInput data, inout UnityGI gi)
		{
			LightingStandard_GI(s, data, gi); 
		}
 
		


		// the following are in the standard surface shader

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_BUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_BUFFER_END(Props)

		void surf (Input IN, inout SurfaceOutputStandard o) {
			// Albedo comes from a texture tinted by color
			fixed4 c = tex2D (_MainTex, IN.uv_MainTex) * _Color;
			o.Albedo = c.rgb;
			// Metallic and smoothness come from slider variables
			o.Metallic = _Metallic;
			o.Smoothness = _Glossiness;
			o.Alpha = c.a;
		}
		ENDCG
	}
	FallBack "Diffuse"
}


```

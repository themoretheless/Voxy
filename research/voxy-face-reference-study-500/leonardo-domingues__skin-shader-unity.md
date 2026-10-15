# Facial rendering source scan: leonardo-domingues/skin-shader-unity

Repository: https://github.com/leonardo-domingues/skin-shader-unity

Pinned source revision: 80e8c17424dd2d541922b7af227768f94fd30b6d

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["Skin/Assembly-CSharp.csproj", "Skin/Assets/Materials.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/AlphaMask.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/AlphaMask.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ApplyShadowsToIrradiance.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ApplyShadowsToIrradiance.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeAttenuation.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeAttenuation.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeBeckmann.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeBeckmann.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeGaussian_U.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeGaussian_U.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeGaussian_V.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeGaussian_V.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeIrradiance.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeIrradiance.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeStretchMaps.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ComputeStretchMaps.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ShadowMaterial.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/ShadowMaterial.mat.meta", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/TSM.mat", "Skin/Assets/Materials/Skin_GPUGems3-14-Aux/TSM.mat.meta", "Skin/Assets/Models.meta", "Skin/Assets/Models/3dScanStore-BlenderHead.meta", "Skin/Assets/Models/3dScanStore-BlenderHead/HeadDecimated.fbx", "Skin/Assets/Models/3dScanStore-BlenderHead/HeadDecimated.fbx.meta", "Skin/Assets/Models/3dScanStore-BlenderHead/Materials.meta", "Skin/Assets/Models/3dScanStore-BlenderHead/Materials/defaultMat.001.mat", "Skin/Assets/Models/3dScanStore-BlenderHead/Materials/defaultMat.001.mat.meta"], "shader": ["Skin/Assets/Shaders.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ApplyShadowsToIrradiance.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ApplyShadowsToIrradiance.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/BRDF.cginc", "Skin/Assets/Shaders/Skin_GPUGems3-14/BRDF.cginc.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeAlphaMask.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeAlphaMask.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeAttenuation.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeAttenuation.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeBeckmann.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeBeckmann.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeIrradiance.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeIrradiance.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeShadow.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeShadow.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeStretchMaps.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeStretchMaps.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/GaussianConvolution_U.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/GaussianConvolution_U.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/GaussianConvolution_V.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/GaussianConvolution_V.shader.meta", "Skin/Assets/Shaders/Skin_GPUGems3-14/TSM.shader", "Skin/Assets/Shaders/Skin_GPUGems3-14/TSM.shader.meta"]}

## Source file: Skin/Assets/Shaders/Skin_GPUGems3-14.shader

{
  "path": "Skin/Assets/Shaders/Skin_GPUGems3-14.shader",
  "blob_sha": "369b5be52be4612ccb450bcfdd02b6fb34ab0ec9",
  "sha256": "692e9a06a8cec12c65d99df4e75181786ef0ffe58a357f6dfb219907909b76fb",
  "lines": 201,
  "bytes": 7294,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 2,
    "shader": 2
  },
  "url": "https://github.com/leonardo-domingues/skin-shader-unity/blob/80e8c17424dd2d541922b7af227768f94fd30b6d/Skin/Assets/Shaders/Skin_GPUGems3-14.shader",
  "function_signature_candidates": [
    {
      "line": 112,
      "text": "float3 translucency(v2f i, float4 tap2, float4 tap3, float4 tap4, float4 tap5,"
    },
    {
      "line": 141,
      "text": "float4 frag(v2f i) : COLOR"
    }
  ]
}

```text

﻿Shader "Skin/Skin (GPU Gems 3 - 14)"
{
	Properties
	{
		_MainTex("Diffuse Texture", 2D) = "white" {}
		_NormalMap("Normal Map (Object Space)", 2D) = "black" {}
		_Roughness("Roughness", Range(0.0, 1.0)) = 0.0
		_SpecPower("Specular Power", Range(0.0, 1.0)) = 1.0
		_TranslucencyPower("Translucency Power", Range(0.0, 1.0)) = 1.0
		_Mix("Pre and Post Scatter Mix", Range(0.0, 1.0)) = 0.5
		_EnergyConservation("Scattering Energy Conservation (bool)", Range(0.0, 1.0)) = 0.0
		_BlurStepScale("Gaussian Sample Distance Scale", Range(0.01, 1.0)) = 0.01
		_Blur1WV("Gaussian 1 Weight and Variance", Vector) = (0.233, 0.455, 0.649, 0.0064)
		_Blur2WV("Gaussian 2 Weight and Variance", Vector) = (0.100, 0.336, 0.344, 0.0484)
		_Blur3WV("Gaussian 3 Weight and Variance", Vector) = (0.118, 0.198, 0.000, 0.1870)
		_Blur4WV("Gaussian 4 Weight and Variance", Vector) = (0.113, 0.007, 0.007, 0.5670)
		_Blur5WV("Gaussian 5 Weight and Variance", Vector) = (0.358, 0.004, 0.000, 1.9900)
		_Blur6WV("Gaussian 6 Weight and Variance", Vector) = (0.078, 0.000, 0.000, 7.4100)
		_Grow("TSM - Grow Along Normal", Range(0.0, 0.05)) = 0.00005
		_ThicknessConst("TSM - Thickness Constant", Range(0.1, 20)) = 0.2
		_TSMSpread("TSM Spread", Range(0.0, 200.0)) = 200.0
	}

	SubShader
	{
		Pass
		{
			Tags { "LightMode" = "ForwardBase" }

			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			#pragma multi_compile_fwdbase
			#include "UnityCG.cginc"
			#include "AutoLight.cginc"
			#include "Skin_GPUGems3-14/BRDF.cginc"

			// Properties
			uniform sampler2D _MainTex;
			uniform sampler2D _NormalMap;
			uniform float _Roughness;
			uniform float _SpecPower;
			uniform float _TranslucencyPower;
			uniform float _Mix;
			uniform float _EnergyConservation;
			uniform float _BlurStepScale;
			uniform float4 _Blur1WV;
			uniform float4 _Blur2WV;
			uniform float4 _Blur3WV;
			uniform float4 _Blur4WV;
			uniform float4 _Blur5WV;
			uniform float4 _Blur6WV;
			uniform float _Grow;
			uniform float _ThicknessConst;
			uniform float _TSMSpread;

			// Set by script
			uniform sampler2D _IrradianceTex;
			uniform sampler2D _AttenuationTex;
			uniform sampler2D _BeckmannTex;
			uniform sampler2D _Blur2Tex;
			uniform sampler2D _Blur3Tex;
			uniform sampler2D _Blur4Tex;
			uniform sampler2D _Blur5Tex;
			uniform sampler2D _Blur6Tex;
			uniform sampler2D _TSMTex;
			uniform sampler2D _StretchTex;
			uniform sampler2D _Blur6StretchTex;
			uniform sampler2D _AlphaMaskTex;
			uniform float4x4 _LightViewProj;
			uniform float _TextureSize;
			uniform float4 _TsmLightPosWorld;
			uniform float _StretchScale;

			// Unity
			uniform float4 _LightColor0;

			struct a2v
			{
				float4 vertex : POSITION;
				float3 normal : NORMAL;
				float4 tangent : TANGENT;
				float2 tex : TEXCOORD0;
			};

			struct v2f
			{
				float4 pos : SV_POSITION;
				float2 tex : TEXCOORD0;
				float3 posWorld : TEXCOORD1;
				float3 lightDir : TEXCOORD2;
				float3 viewDir : TEXCOORD3;
				LIGHTING_COORDS(4, 5)
			};

			v2f vert(a2v v)
			{
				v2f o;

				o.pos = mul(UNITY_MATRIX_MVP, v.vertex);
				o.tex = v.tex;
				o.posWorld = mul(_Object2World, v.vertex);
				o.lightDir = UnityWorldSpaceLightDir(o.posWorld);
				o.viewDir = _WorldSpaceCameraPos.xyz - o.posWorld;

				TRANSFER_VERTEX_TO_FRAGMENT(o);

				return o;
			}

			// Compute global scatter from TSM
			float3 translucency(v2f i, float4 tap2, float4 tap3, float4 tap4, float4 tap5, 
				float3 totalWeight, float4 stretchTap, float3 normal)
			{				
				float4 posLightProjCoord = mul(_LightViewProj, float4(i.posWorld, 1.0));
				float2 lightCoord = posLightProjCoord.xy / posLightProjCoord.w;
				lightCoord = lightCoord * 0.5 + 0.5;
				lightCoord.y = 1.0 - lightCoord.y;
				float4 tsmTap = tex2D(_TSMTex, lightCoord);

				// We only consider the contribution for the 4 most blurred taps. For tap n, 
				// we use the thickness stored in texture n-1
				float4 thickness = (-1.0 / _ThicknessConst) * log(float4(tap2.a, tap3.a, tap4.a, tap5.a)) * _TSMSpread;

				float4 variances = float4(_Blur3WV.w, _Blur4WV.w, _Blur5WV.w, _Blur6WV.w);
				float4 stdDevs = sqrt(variances);
				float4 fades = exp(-(thickness * thickness) / variances);

				float stretch = 0.5 * (stretchTap.r + stretchTap.g);
				float textureScale = _TextureSize * 0.1 / stretch;
				float4 blendFactor = saturate(textureScale * length(i.tex - tsmTap.yz) / (6.0 * stdDevs));

				float3 translucency = float3(0.0, 0.0, 0.0);
				translucency += ((_Blur4WV.xyz / totalWeight) * fades.y * blendFactor.y * tex2D(_Blur4Tex, tsmTap.yz).rgb);
				translucency += ((_Blur5WV.xyz / totalWeight) * fades.z * blendFactor.z * tex2D(_Blur5Tex, tsmTap.yz).rgb);
				translucency += ((_Blur6WV.xyz / totalWeight) * fades.w * blendFactor.w * tex2D(_Blur6Tex, tsmTap.yz).rgb);

				return translucency * max(0.0, -dot(normal, i.lightDir));
			}

			float4 frag(v2f i) : COLOR
			{
				i.lightDir = normalize(i.lightDir);
				i.viewDir = normalize(i.viewDir);

				// Sample object space normal
				float3 normalObjectSpace = tex2D(_NormalMap, i.tex).rgb - 0.5;
				normalObjectSpace.x = -normalObjectSpace.x;
				float3 normal = normalize(mul(normalObjectSpace, (float3x3)_World2Object));

				// Diffuse lighting from sum-of-Gaussians
				float3 albedo = tex2D(_MainTex, i.tex).rgb;
				float4 tap1 = tex2D(_IrradianceTex, i.tex);
				float4 tap2 = tex2D(_Blur2Tex, i.tex);
				float4 tap3 = tex2D(_Blur3Tex, i.tex);
				float4 tap4 = tex2D(_Blur4Tex, i.tex);
				float4 tap5 = tex2D(_Blur5Tex, i.tex);
				float4 tap6 = tex2D(_Blur6Tex, i.tex);
				float3 totalWeight = _Blur1WV.xyz + _Blur2WV.xyz + _Blur3WV.xyz + _Blur4WV.xyz + _Blur5WV.xyz + _Blur6WV.xyz;
				float3 diffuse = float3(0.0, 0.0, 0.0);
				diffuse += _Blur1WV.xyz * tap1.rgb;
				diffuse += _Blur2WV.xyz * tap2.rgb;
				diffuse += _Blur3WV.xyz * tap3.rgb;
				diffuse += _Blur4WV.xyz * tap4.rgb;
				diffuse += _Blur5WV.xyz * tap5.rgb;
				diffuse += _Blur6WV.xyz * tap6.rgb;
				diffuse /= totalWeight;

				// Add translucency
				float4 stretchTap = tex2D(_Blur6StretchTex, i.tex);
				float3 localScattering = translucency(i, tap2, tap3, tap4, tap5, totalWeight, stretchTap, normal);
				diffuse += (_TranslucencyPower * localScattering);

				// Account for pre and post scatter mix
				diffuse *= pow(albedo, 1.0 - _Mix);

				// Remove seams caused by connected areas being disconnected in texture space
				float4 diffuseTap = tex2D(_MainTex, i.tex);
				float nDotL = max(0.0, dot(normal, i.lightDir));
				float atten = LIGHT_ATTENUATION(i);
				float3 localDiffuse = diffuseTap * (nDotL * _LightColor0.rgb * atten + UNITY_LIGHTMODEL_AMBIENT);
				float4 alphaMask = tex2D(_AlphaMaskTex, i.tex).r;
				diffuse = lerp(localDiffuse, diffuse, alphaMask);

				// Account for energy conservation
				float reflectedEnergy = tex2D(_AttenuationTex, float2(dot(normal, i.viewDir), _Roughness)).r;
				float energyFactor = lerp(1.0, (1.0 - _SpecPower * reflectedEnergy), _EnergyConservation);
				diffuse *= energyFactor;

				// Specular lighting
				float brdf = brdf_KS(normal, i.lightDir, i.viewDir, _Roughness, _SpecPower, _BeckmannTex);
				float3 specular = brdf * _LightColor0.rgb * atten;

				float3 final = diffuse + specular;
				return float4(final, 1.0);
			}
			ENDCG
		}
	}
	Fallback "Diffuse"
}

```

## Source file: Skin/Assets/Shaders/Skin_GPUGems3-14/BRDF.cginc

{
  "path": "Skin/Assets/Shaders/Skin_GPUGems3-14/BRDF.cginc",
  "blob_sha": "bf108d46083c2b7a502345e74006d455923d2020",
  "sha256": "3132fdaff974b879629c2f4d4bd9dd8a8a1646b7d58c2b7ed86ff00dd90a4927",
  "lines": 31,
  "bytes": 985,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/leonardo-domingues/skin-shader-unity/blob/80e8c17424dd2d541922b7af227768f94fd30b6d/Skin/Assets/Shaders/Skin_GPUGems3-14/BRDF.cginc",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "float fresnelReflectance(float3 halfDir, float3 viewDir, float F0)"
    },
    {
      "line": 13,
      "text": "float brdf_KS(float3 normal, float3 lightDir, float3 viewDir, float roughness, float specPower, sampler2D beckmannTex)"
    }
  ]
}

```text

﻿#ifndef BRDF_INCLUDED
#define BRDF_INCLUDED

// Calculate the fresnel reflectance using Schlick's approximation
float fresnelReflectance(float3 halfDir, float3 viewDir, float F0)
{
	float base = 1.0 - dot(viewDir, halfDir);
	float exponential = pow(base, 5.0);
	return exponential + F0 * (1.0 - exponential);
}

// Kelemen / Szirmay-Kalos specular BRDF
float brdf_KS(float3 normal, float3 lightDir, float3 viewDir, float roughness, float specPower, sampler2D beckmannTex)
{
	float result = 0.0;
	float nDotL = dot(normal, lightDir);
	if (nDotL > 0.0)
	{
		float3 h = viewDir + lightDir;
		float3 halfDir = normalize(h);
		float nDotH = dot(normal, halfDir);
		float PH = pow(2.0 * tex2D(beckmannTex, float2(nDotH, roughness)).r, 10.0);
		const float F0 = 0.028; // Reflectance at normal incidence
		float F = fresnelReflectance(halfDir, viewDir, F0);
		float frSpec = max(PH * F / dot(h, h), 0.0);
		result = frSpec * nDotL * specPower;
	}
	return result;
}

#endif // BRDF_INCLUDED

```

## Source file: Skin/Assets/Shaders/Skin_GPUGems3-14/TSM.shader

{
  "path": "Skin/Assets/Shaders/Skin_GPUGems3-14/TSM.shader",
  "blob_sha": "70c151854c70a159172ddbb2cb82d64752c165c5",
  "sha256": "0ea5ec280657344efdd73b2d2aec2fe845561c16f0327116076ba893fc8291cf",
  "lines": 47,
  "bytes": 722,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 1,
    "shader": 2
  },
  "url": "https://github.com/leonardo-domingues/skin-shader-unity/blob/80e8c17424dd2d541922b7af227768f94fd30b6d/Skin/Assets/Shaders/Skin_GPUGems3-14/TSM.shader",
  "function_signature_candidates": [
    {
      "line": 39,
      "text": "float4 frag(v2f i) : COLOR"
    }
  ]
}

```text

﻿Shader "Skin/TSM"
{
	SubShader
	{
		Pass
		{
			Tags { "LightMode" = "ForwardBase" }
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag

			uniform float _Grow;

			struct a2v
			{
				float4 vertex : POSITION;
				float4 normal : NORMAL;
				float2 tex : TEXCOORD0;
			};

			struct v2f
			{
				float4 pos : SV_POSITION;
				float2 tex : TEXCOORD0;
				float depth : TEXCOORD1;
			};

			v2f vert(a2v v)
			{
				v2f o;

				o.pos = mul(UNITY_MATRIX_MVP, v.vertex + _Grow * v.normal);
				o.tex = v.tex;
				o.depth = length(mul(UNITY_MATRIX_MV, v.vertex));

				return o;
			}

			float4 frag(v2f i) : COLOR
			{
				return float4(i.depth, i.tex.x, i.tex.y, 1.0);
			}
			ENDCG
		}
	}
	Fallback "Diffuse"
}

```

## Source file: Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeShadow.shader

{
  "path": "Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeShadow.shader",
  "blob_sha": "d58719b403dd45ce4b88e228589612a6e91e5054",
  "sha256": "610759b494e3e6d87303d980d56794d2441934690e5643816642457b92a3e2af",
  "lines": 96,
  "bytes": 2408,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 1,
    "shader": 2
  },
  "url": "https://github.com/leonardo-domingues/skin-shader-unity/blob/80e8c17424dd2d541922b7af227768f94fd30b6d/Skin/Assets/Shaders/Skin_GPUGems3-14/ComputeShadow.shader",
  "function_signature_candidates": [
    {
      "line": 52,
      "text": "float computeThickness(float3 posWorld, float3 normal, float nDotL)"
    },
    {
      "line": 73,
      "text": "float4 frag(v2f i) : COLOR"
    }
  ]
}

```text

﻿Shader "Skin/Compute shadow"
{
	SubShader
	{
		Pass
		{
			Cull Off
			Tags { "LightMode" = "ForwardBase" }

			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			#include "UnityCG.cginc"

			// Set by inheritance
			uniform sampler2D _NormalMap;
			uniform sampler2D _TSMTex;
			uniform float4x4 _LightViewProj;
			uniform float4 _TsmLightPosWorld;

			struct a2v
			{
				float4 vertex : POSITION;
				float3 normal : NORMAL;
				float4 tangent : TANGENT;
				float2 tex : TEXCOORD0;
			};

			struct v2f
			{
				float4 pos : SV_POSITION;
				float2 tex : TEXCOORD0;
				float3 posWorld : TEXCOORD1;
				float3 viewDir : TEXCOORD2;
				float3 lightDir : TEXCOORD3;
			};

			v2f vert(a2v v)
			{
				v2f o;

				float2 coords = float2(2.0 * v.tex.x - 1.0, 1.0 - 2.0 * v.tex.y);
				o.pos = float4(coords, 0.0, 1.0);
				o.posWorld = mul(_Object2World, v.vertex).xyz;
				o.tex = v.tex;
				o.viewDir = _WorldSpaceCameraPos.xyz - o.posWorld;
				o.lightDir = UnityWorldSpaceLightDir(o.posWorld);

				return o;
			}

			float computeThickness(float3 posWorld, float3 normal, float nDotL)
			{
				// Read TSM
				float4 posLightProjCoord = mul(_LightViewProj, float4(posWorld, 1.0));
				float2 lightCoord = posLightProjCoord.xy / posLightProjCoord.w;
				lightCoord = lightCoord * 0.5 + 0.5;
				lightCoord.y = 1.0 - lightCoord.y;
				float4 tsmTap = tex2D(_TSMTex, lightCoord);

				// Incident normal
				float3 normal_i_ObjectSpace = tex2D(_NormalMap, tsmTap.yz).rgb - 0.5;
				normal_i_ObjectSpace.x = -normal_i_ObjectSpace.x;
				float3 normal_i = normalize(mul(normal_i_ObjectSpace, (float3x3)_World2Object));

				// Compute thickness
				float distanceToLight = length(_TsmLightPosWorld.xyz - posWorld);
				float thickness = distanceToLight - tsmTap.x;

				return thickness;
			}

			float4 frag(v2f i) : COLOR
			{
				i.viewDir = normalize(i.viewDir);
				i.lightDir = normalize(i.lightDir);

				// Sample object space normal
				float3 normalObjectSpace = tex2D(_NormalMap, i.tex).rgb - 0.5;
				normalObjectSpace.x = -normalObjectSpace.x;
				float3 normal = normalize(mul(normalObjectSpace, (float3x3)_World2Object));

				float nDotL = dot(normal, i.lightDir);
				float thickness = computeThickness(i.posWorld, normal, nDotL);
				float atten = 0.0;
				if (thickness <= 0.0)
				{
					atten = 1.0;
				}

				return float4(atten, atten, atten, 1.0);
			}
			ENDCG
		}
	}
}

```

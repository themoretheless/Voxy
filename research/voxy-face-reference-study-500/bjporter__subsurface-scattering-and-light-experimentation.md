# Facial rendering source scan: bjporter/subsurface-scattering-and-light-experimentation

Repository: https://github.com/bjporter/subsurface-scattering-and-light-experimentation

Pinned source revision: 0358ae36e85a3a160f9e93affe5b5b52e02d22fe

Discovery topic: subsurface scattering

Description: Various light effects including SSS in Unity 5.3+ and applying it to real world situations

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["Assets/Standard Assets/Effects/ImageEffects/Scripts/Fisheye.cs", "Assets/Standard Assets/Effects/ImageEffects/Scripts/Fisheye.cs.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/FisheyeShader.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/FisheyeShader.shader.meta"], "subsurface": ["Assets/Scenes/SubsurfaceScatteringTest.unity", "Assets/Scenes/SubsurfaceScatteringTest.unity.meta"], "sss": ["Assets/Farfarer Method/TranslucentSSSFarfarer.shader", "Assets/Farfarer Method/TranslucentSSSFarfarer.shader.meta", "Assets/Lucy Test/Models/lucy/LucyWhite 4 farfarer SSS.mat", "Assets/Lucy Test/Models/lucy/LucyWhite 4 farfarer SSS.mat.meta", "Assets/Lucy Test/Shaders/SSS.shader", "Assets/Lucy Test/Shaders/SSS.shader.meta", "Assets/Materials/SSS Material.mat", "Assets/Materials/SSS Material.mat.meta", "Assets/Scenes/Testing SSS.unity", "Assets/Scenes/Testing SSS.unity.meta", "Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader", "Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader.meta", "Assets/Standard Assets/Effects/GlassRefraction/Textures/GlassStainedAlbedo.tif", "Assets/Standard Assets/Effects/GlassRefraction/Textures/GlassStainedAlbedo.tif.meta", "Assets/Standard Assets/Effects/GlassRefraction/Textures/GlassStainedNormals.tif", "Assets/Standard Assets/Effects/GlassRefraction/Textures/GlassStainedNormals.tif.meta"], "shader": ["Assets/FPires/Shaders.meta", "Assets/FPires/Shaders/Translucency - Basic.shader", "Assets/FPires/Shaders/Translucency - Basic.shader.meta", "Assets/FPires/Shaders/Translucency - Beers.shader", "Assets/FPires/Shaders/Translucency - Beers.shader.meta", "Assets/Farfarer Method/TranslucentSSSFarfarer.shader", "Assets/Farfarer Method/TranslucentSSSFarfarer.shader.meta", "Assets/Lucy Test/Shaders.meta", "Assets/Lucy Test/Shaders/SSS.shader", "Assets/Lucy Test/Shaders/SSS.shader.meta", "Assets/Lucy Test/UpdateShader.cs", "Assets/Lucy Test/UpdateShader.cs.meta", "Assets/Standard Assets/Effects/GlassRefraction/Shaders.meta", "Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader", "Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/BlendModesOverlay.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/BlendModesOverlay.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/BlurEffectConeTaps.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/BlurEffectConeTaps.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/CameraMotionBlur.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/CameraMotionBlur.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/CameraMotionBlurDX11.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/CameraMotionBlurDX11.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ChromaticAberrationShader.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ChromaticAberrationShader.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ColorCorrection3DLut.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ColorCorrection3DLut.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ColorCorrectionCurves.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/ColorCorrectionCurves.shader.meta"], "rig": ["Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter.shader.meta", "Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter2.shader", "Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter2.shader.meta"], "face": ["Assets/Lucy Test/Models/lucy/Materials/FitnessGrandma_FaceAnimMap.mat", "Assets/Lucy Test/Models/lucy/Materials/FitnessGrandma_FaceAnimMap.mat.meta", "Assets/Lucy Test/Models/lucy/Materials/Grandma2/FitnessGrandma_FaceAnimMap 1.mat", "Assets/Lucy Test/Models/lucy/Materials/Grandma2/FitnessGrandma_FaceAnimMap 1.mat.meta", "Assets/Lucy Test/Models/lucy/Sporty_Granny@guitar_playing 1.fbm/FitnessGrandma_FaceAnimMap.png", "Assets/Lucy Test/Models/lucy/Sporty_Granny@guitar_playing 1.fbm/FitnessGrandma_FaceAnimMap.png.meta", "Assets/Lucy Test/Models/lucy/Sporty_Granny@guitar_playing.fbm/FitnessGrandma_FaceAnimMap.png", "Assets/Lucy Test/Models/lucy/Sporty_Granny@guitar_playing.fbm/FitnessGrandma_FaceAnimMap.png.meta", "Assets/Scenes/Materials/FitnessGrandma_FaceAnimMap.mat", "Assets/Scenes/Materials/FitnessGrandma_FaceAnimMap.mat.meta", "Assets/Scenes/Sporty_Granny@guitar_playing.fbm/FitnessGrandma_FaceAnimMap.png", "Assets/Scenes/Sporty_Granny@guitar_playing.fbm/FitnessGrandma_FaceAnimMap.png.meta", "Assets/Scenes/SubsurfaceScatteringTest.unity", "Assets/Scenes/SubsurfaceScatteringTest.unity.meta"]}

## Source file: Assets/Standard Assets/Effects/ImageEffects/Shaders/FisheyeShader.shader

{
  "path": "Assets/Standard Assets/Effects/ImageEffects/Shaders/FisheyeShader.shader",
  "blob_sha": "5338e2e5b8b24546e530af3a0df9a4d0e37a6ab6",
  "sha256": "ba9d42ee00bd7727c01d66d536dccdb2e83a8a9277474a8802a5b17c5f9e3d86",
  "lines": 58,
  "bytes": 963,
  "selection_score": 42,
  "keyword_line_counts": {
    "eye": 1,
    "shader": 4
  },
  "url": "https://github.com/bjporter/subsurface-scattering-and-light-experimentation/blob/0358ae36e85a3a160f9e93affe5b5b52e02d22fe/Assets/Standard%20Assets/Effects/ImageEffects/Shaders/FisheyeShader.shader",
  "function_signature_candidates": []
}

```text

Shader "Hidden/FisheyeShader" {
	Properties {
		_MainTex ("Base (RGB)", 2D) = "" {}
	}
	
	// Shader code pasted into all further CGPROGRAM blocks
	CGINCLUDE
	
	#include "UnityCG.cginc"
	
	struct v2f {
		float4 pos : SV_POSITION;
		float2 uv : TEXCOORD0;
	};
	
	sampler2D _MainTex;
	
	float2 intensity;
	
	v2f vert( appdata_img v ) 
	{
		v2f o;
		o.pos = mul(UNITY_MATRIX_MVP, v.vertex);
		o.uv = v.texcoord.xy;
		return o;
	} 
	
	half4 frag(v2f i) : SV_Target 
	{
		half2 coords = i.uv;
		coords = (coords - 0.5) * 2.0;		
		
		half2 realCoordOffs;
		realCoordOffs.x = (1-coords.y * coords.y) * intensity.y * (coords.x); 
		realCoordOffs.y = (1-coords.x * coords.x) * intensity.x * (coords.y);
		
		half4 color = tex2D (_MainTex, i.uv - realCoordOffs);	 
		
		return color;
	}

	ENDCG 
	
Subshader {
 Pass {
	  ZTest Always Cull Off ZWrite Off

      CGPROGRAM
      #pragma vertex vert
      #pragma fragment frag
      ENDCG
  }
  
}

Fallback off
	
} // shader


```

## Source file: Assets/Farfarer Method/TranslucentSSSFarfarer.shader

{
  "path": "Assets/Farfarer Method/TranslucentSSSFarfarer.shader",
  "blob_sha": "1342ae87433f3dc8497328aa2571e5955550a1f2",
  "sha256": "dfa501d8be29c9c1ef7897bedd78892ec91ac7c54cc0c15ba0fb7a329b9f9927",
  "lines": 77,
  "bytes": 2528,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 6,
    "sss": 1,
    "shader": 2,
    "face": 9
  },
  "url": "https://github.com/bjporter/subsurface-scattering-and-light-experimentation/blob/0358ae36e85a3a160f9e93affe5b5b52e02d22fe/Assets/Farfarer%20Method/TranslucentSSSFarfarer.shader",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "void surf(Input IN, inout SurfaceOutput o) {"
    }
  ]
}

```text

﻿Shader "Farfarer/TranslucentSSS" {
	Properties{
		_MainTex("Base (RGB)", 2D) = "white" {}
	_BumpMap("Normal (Normal)", 2D) = "bump" {}
	_Color("Main Color", Color) = (1,1,1,1)
		_SpecColor("Specular Color", Color) = (0.5, 0.5, 0.5, 1)
		_Shininess("Shininess", Range(0.03, 1)) = 0.078125

		//_Thickness = Thickness texture (invert normals, bake AO).
		//_Power = "Sharpness" of translucent glow.
		//_Distortion = Subsurface distortion, shifts surface normal, effectively a refractive index.
		//_Scale = Multiplier for translucent glow - should be per-light, really.
		//_SubColor = Subsurface colour.
		_Thickness("Thickness (R)", 2D) = "bump" {}
	_Power("Subsurface Power", Float) = 1.0
		_Distortion("Subsurface Distortion", Float) = 0.0
		_Scale("Subsurface Scale", Float) = 0.5
		_SubColor("Subsurface Color", Color) = (1.0, 1.0, 1.0, 1.0)
	}
		SubShader{
		Tags{ "RenderType" = "Opaque" }
		LOD 200

		CGPROGRAM
#pragma surface surf Translucent
#pragma exclude_renderers flash

		sampler2D _MainTex, _BumpMap, _Thickness;
	float _Scale, _Power, _Distortion;
	fixed4 _Color, _SubColor;
	half _Shininess;

	struct Input {
		float2 uv_MainTex;
	};

	void surf(Input IN, inout SurfaceOutput o) {
		fixed4 tex = tex2D(_MainTex, IN.uv_MainTex);
		o.Albedo = tex.rgb * _Color.rgb;
		o.Alpha = tex2D(_Thickness, IN.uv_MainTex).r;
		o.Gloss = tex.a;
		o.Specular = _Shininess;
		o.Normal = UnpackNormal(tex2D(_BumpMap, IN.uv_MainTex));
	}

	inline fixed4 LightingTranslucent(SurfaceOutput s, fixed3 lightDir, fixed3 viewDir, fixed atten)
	{
		// You can remove these two lines,
		// to save some instructions. They're just
		// here for visual fidelity.
		viewDir = normalize(viewDir);
		lightDir = normalize(lightDir);

		// Translucency.
		half3 transLightDir = lightDir + s.Normal * _Distortion;
		float transDot = pow(max(0, dot(viewDir, -transLightDir)), _Power) * _Scale;
		fixed3 transLight = (atten * 2) * (transDot)* s.Alpha * _SubColor.rgb;
		fixed3 transAlbedo = s.Albedo * _LightColor0.rgb * transLight;

		// Regular BlinnPhong.
		half3 h = normalize(lightDir + viewDir);
		fixed diff = max(0, dot(s.Normal, lightDir));
		float nh = max(0, dot(s.Normal, h));
		float spec = pow(nh, s.Specular*128.0) * s.Gloss;
		fixed3 diffAlbedo = (s.Albedo * _LightColor0.rgb * diff + _LightColor0.rgb * _SpecColor.rgb * spec) * (atten * 2);

		// Add the two together.
		fixed4 c;
		c.rgb = diffAlbedo + transAlbedo;
		c.a = _LightColor0.a * _SpecColor.a * spec * atten;
		return c;
	}

	ENDCG
	}
		FallBack "Bumped Diffuse"
}

```

## Source file: Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader

{
  "path": "Assets/Standard Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader",
  "blob_sha": "aaccf5f773e86eda75ba1bc557c67a18f413fd61",
  "sha256": "46626351f7f99f0babdec25ee5ad1657c35fc7c09a01b79c4567f149bd3f9981",
  "lines": 107,
  "bytes": 2486,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 3
  },
  "url": "https://github.com/bjporter/subsurface-scattering-and-light-experimentation/blob/0358ae36e85a3a160f9e93affe5b5b52e02d22fe/Assets/Standard%20Assets/Effects/GlassRefraction/Shaders/GlassStainedBumpDistort.shader",
  "function_signature_candidates": []
}

```text

// Per pixel bumped refraction.
// Uses a normal map to distort the image behind, and
// an additional texture to tint the color.

Shader "FX/Glass/Stained BumpDistort" {
Properties {
	_BumpAmt  ("Distortion", range (0,128)) = 10
	_MainTex ("Tint Color (RGB)", 2D) = "white" {}
	_BumpMap ("Normalmap", 2D) = "bump" {}
}

Category {

	// We must be transparent, so other objects are drawn before this one.
	Tags { "Queue"="Transparent" "RenderType"="Opaque" }


	SubShader {

		// This pass grabs the screen behind the object into a texture.
		// We can access the result in the next pass as _GrabTexture
		GrabPass {
			Name "BASE"
			Tags { "LightMode" = "Always" }
		}
		
		// Main pass: Take the texture grabbed above and use the bumpmap to perturb it
		// on to the screen
		Pass {
			Name "BASE"
			Tags { "LightMode" = "Always" }
			
CGPROGRAM
#pragma vertex vert
#pragma fragment frag
#pragma multi_compile_fog
#include "UnityCG.cginc"

struct appdata_t {
	float4 vertex : POSITION;
	float2 texcoord: TEXCOORD0;
};

struct v2f {
	float4 vertex : SV_POSITION;
	float4 uvgrab : TEXCOORD0;
	float2 uvbump : TEXCOORD1;
	float2 uvmain : TEXCOORD2;
	UNITY_FOG_COORDS(3)
};

float _BumpAmt;
float4 _BumpMap_ST;
float4 _MainTex_ST;

v2f vert (appdata_t v)
{
	v2f o;
	o.vertex = mul(UNITY_MATRIX_MVP, v.vertex);
	#if UNITY_UV_STARTS_AT_TOP
	float scale = -1.0;
	#else
	float scale = 1.0;
	#endif
	o.uvgrab.xy = (float2(o.vertex.x, o.vertex.y*scale) + o.vertex.w) * 0.5;
	o.uvgrab.zw = o.vertex.zw;
	o.uvbump = TRANSFORM_TEX( v.texcoord, _BumpMap );
	o.uvmain = TRANSFORM_TEX( v.texcoord, _MainTex );
	UNITY_TRANSFER_FOG(o,o.vertex);
	return o;
}

sampler2D _GrabTexture;
float4 _GrabTexture_TexelSize;
sampler2D _BumpMap;
sampler2D _MainTex;

half4 frag (v2f i) : SV_Target
{
	// calculate perturbed coordinates
	half2 bump = UnpackNormal(tex2D( _BumpMap, i.uvbump )).rg; // we could optimize this by just reading the x & y without reconstructing the Z
	float2 offset = bump * _BumpAmt * _GrabTexture_TexelSize.xy;
	i.uvgrab.xy = offset * i.uvgrab.z + i.uvgrab.xy;
	
	half4 col = tex2Dproj( _GrabTexture, UNITY_PROJ_COORD(i.uvgrab));
	half4 tint = tex2D(_MainTex, i.uvmain);
	col *= tint;
	UNITY_APPLY_FOG(i.fogCoord, col);
	return col;
}
ENDCG
		}
	}

	// ------------------------------------------------------------------
	// Fallback for older cards and Unity non-Pro

	SubShader {
		Blend DstColor Zero
		Pass {
			Name "BASE"
			SetTexture [_MainTex] {	combine texture }
		}
	}
}

}


```

## Source file: Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter.shader

{
  "path": "Assets/Standard Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter.shader",
  "blob_sha": "7af07899a5bf1f3f4e71a90f3257219d2b239b22",
  "sha256": "ca8ca8bdd771b645c9bc01add6437d296063ebadce708127fe0ecbae1b0e15a3",
  "lines": 57,
  "bytes": 926,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 2,
    "rig": 1
  },
  "url": "https://github.com/bjporter/subsurface-scattering-and-light-experimentation/blob/0358ae36e85a3a160f9e93affe5b5b52e02d22fe/Assets/Standard%20Assets/Effects/ImageEffects/Shaders/_BloomAndFlares/BrightPassFilter.shader",
  "function_signature_candidates": []
}

```text

Shader "Hidden/BrightPassFilterForBloom"
{
	Properties 
	{
		_MainTex ("Base (RGB)", 2D) = "" {}
	}
	
	CGINCLUDE
	
	#include "UnityCG.cginc"
	
	struct v2f 
	{
		float4 pos : SV_POSITION;
		float2 uv : TEXCOORD0;
	};
	
	sampler2D _MainTex;	
	
	half4 threshold;
	half useSrcAlphaAsMask;
		
	v2f vert( appdata_img v ) 
	{
		v2f o;
		o.pos = mul(UNITY_MATRIX_MVP, v.vertex);
		o.uv =  v.texcoord.xy;
		return o;
	} 
	
	half4 frag(v2f i) : SV_Target 
	{
		half4 color = tex2D(_MainTex, i.uv);
		//color = color * saturate((color-threshhold.x) * 75.0); // didn't go well with HDR and din't make sense
		color = color * lerp(1.0, color.a, useSrcAlphaAsMask);
		color = max(half4(0,0,0,0), color-threshold.x);
		return color;
	}

	ENDCG 
	
	Subshader 
	{
		Pass 
 		{
			  ZTest Always Cull Off ZWrite Off
		
		      CGPROGRAM
		      
		      #pragma vertex vert
		      #pragma fragment frag
		
		      ENDCG
		}
	}
	Fallback off
}


```

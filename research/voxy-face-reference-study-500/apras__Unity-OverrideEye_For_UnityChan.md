# Facial rendering source scan: apras/Unity-OverrideEye_For_UnityChan

Repository: https://github.com/apras/Unity-OverrideEye_For_UnityChan

Pinned source revision: 372c8239167ce3de2d89340721b1b89973a4c57c

Discovery topic: eye shader

Description: This is the script and shader to render the eye in front of the hair at Unity5 for UnityChan.

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["Assets/UnityChan/License.meta", "LICENSE"]

Topic paths: {"eye": ["Assets/OverrideEye.meta", "Assets/OverrideEye/Resources.meta", "Assets/OverrideEye/Resources/Shaders.meta", "Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader", "Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader.meta", "Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader", "Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader.meta", "Assets/OverrideEye/SampleScenes.meta", "Assets/OverrideEye/SampleScenes/Scene01.unity", "Assets/OverrideEye/SampleScenes/Scene01.unity.meta", "Assets/OverrideEye/Scripts.meta", "Assets/OverrideEye/Scripts/CamOrbit.cs", "Assets/OverrideEye/Scripts/CamOrbit.cs.meta", "Assets/OverrideEye/Scripts/OverrideEye.cs", "Assets/OverrideEye/Scripts/OverrideEye.cs.meta", "Assets/OverrideEye/Scripts/RenderOverrideEye.cs", "Assets/OverrideEye/Scripts/RenderOverrideEye.cs.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eyelash_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eyelash_blend.shader.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/eye_close@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/eye_close@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Humanoid/eye_close@sd_hmd.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Humanoid/eye_close@sd_hmd.anim.meta"], "skin": ["Assets/UnityChan/Models/UnityChanShader/Shader/CharaSkin.cg", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaSkin.cg.meta", "Assets/UnityChan/SD_unitychan/Models/Materials/skin_mat.mat", "Assets/UnityChan/SD_unitychan/Models/Materials/skin_mat.mat.meta", "Assets/UnityChan/SD_unitychan/Models/Textures/FO_SKIN1.tga", "Assets/UnityChan/SD_unitychan/Models/Textures/FO_SKIN1.tga.meta"], "mouth": ["Assets/UnityChan/SD_unitychan/Models/Materials/mouth_mat.mat", "Assets/UnityChan/SD_unitychan/Models/Materials/mouth_mat.mat.meta"], "shader": ["Assets/OverrideEye/Resources/Shaders.meta", "Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader", "Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader.meta", "Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader", "Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader.meta", "Assets/UnityChan/Models/UnityChanShader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaMain.cg", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaMain.cg.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaOutline.cg", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaOutline.cg.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaSkin.cg", "Assets/UnityChan/Models/UnityChanShader/Shader/CharaSkin.cg.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_akarami_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_akarami_blend.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eyelash_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eyelash_blend.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_fuku.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_fuku.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_fuku_ds.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_fuku_ds.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_hada.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_hada.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_hada_blend.shader", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_hada_blend.shader.meta", "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_hair.shader"], "rig": ["Assets/UnityChan/Scripts/IKCtrlRightHand.cs", "Assets/UnityChan/Scripts/IKCtrlRightHand.cs.meta"], "face": ["Assets/UnityChan/SD_unitychan/FaceAnimations.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/angry@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/angry@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/confuse@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/confuse@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/damaged@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/damaged@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/default@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/default@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/eye_close@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/eye_close@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_L@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_L@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_R@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_R@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_a@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_a@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_e@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_e@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_i@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_i@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_o@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_o@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_u@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/mth_u@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/relux@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/relux@sd_generic.anim.meta", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/sad@sd_generic.anim", "Assets/UnityChan/SD_unitychan/FaceAnimations/Generic/sad@sd_generic.anim.meta"]}

## Source file: Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader

{
  "path": "Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader",
  "blob_sha": "7ee4afcd3111970161a7a3b6f2e9572ee583ded1",
  "sha256": "4c747390a6042eb4010745ac2725ce23f12c1b500cad7c5d35749c0c6f6880d4",
  "lines": 101,
  "bytes": 1724,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "shader": 2,
    "lip": 5
  },
  "url": "https://github.com/apras/Unity-OverrideEye_For_UnityChan/blob/372c8239167ce3de2d89340721b1b89973a4c57c/Assets/OverrideEye/Resources/Shaders/OverrideEyeForward.shader",
  "function_signature_candidates": [
    {
      "line": 36,
      "text": "float4 fragPass0 (v2f i) : COLOR"
    },
    {
      "line": 41,
      "text": "float4 fragPass1(v2f_img i) : COLOR"
    },
    {
      "line": 50,
      "text": "float fragPass2 (v2f i) : COLOR"
    }
  ]
}

```text

Shader "OverrideEye/OverrideEyeForward"
{
	Properties
	{
		_MainTex ("Base (RGB)", 2D) = "white" {}
		_alpha ("Blend Alpha", Range(0.0,1.0)) = 1.0
	}
	
	CGINCLUDE
	#include "UnityCG.cginc"
	sampler2D _MainTex;
	sampler2D _ForwardMaskTex;
	sampler2D _ForwardClipTex;
	float _alpha;
	
	struct BuildInGBufferOutput
	{
		float4 Col0 : COLOR0;
		float4 Col1 : COLOR1;
		float4 Col2 : COLOR2;
		float4 Col3 : COLOR3;
	};	
	
	struct v2f
	{
		float4 pos			: SV_POSITION;
	};
	
	v2f vert (appdata_full v)
	{
		v2f o;
		o.pos = mul (UNITY_MATRIX_MVP, v.vertex);
		return o;
	}		
	
	float4 fragPass0 (v2f i) : COLOR
	{
		return 0;			
	}
	
	float4 fragPass1(v2f_img i) : COLOR
	{
		float4 _tex = tex2Dlod(_MainTex, float4(i.uv, 0, 0));
		float _mask = tex2Dlod(_ForwardMaskTex, float4(i.uv, 0, 0)).x;
		float _clip = tex2Dlod(_ForwardClipTex, float4(i.uv, 0, 0)).x;
		//return float4(_clip.xxx, 1);
		return float4(_tex.xyz, _mask * _clip * _alpha);
	}		
	
	float fragPass2 (v2f i) : COLOR
	{
		return 1;
	}		
	
	ENDCG	
	
	SubShader
	{
		//0 Mask
		Pass
		{
			Cull off
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert
			#pragma fragment fragPass0
			ENDCG			
		}
		
	
		//1 Composit 0
		Pass
		{
			ZTest Always Cull Off ZWrite Off
			Blend SrcAlpha OneMinusSrcAlpha
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass1
			ENDCG			
		}		
		
		//2 Render Clip
		Pass
		{
			Cull back
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass2
			ENDCG			
		}		
	} 
	FallBack Off
}

```

## Source file: Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader

{
  "path": "Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader",
  "blob_sha": "474ce0012bfa2c0fcaf45d675eb34507d9a72c60",
  "sha256": "91dc339f348848ebb339ccdf6a1dc199c4093920de3721afd9d901e73f996244",
  "lines": 175,
  "bytes": 3448,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "shader": 2,
    "lip": 10
  },
  "url": "https://github.com/apras/Unity-OverrideEye_For_UnityChan/blob/372c8239167ce3de2d89340721b1b89973a4c57c/Assets/OverrideEye/Resources/Shaders/OverrideEyeDeferred.shader",
  "function_signature_candidates": [
    {
      "line": 48,
      "text": "float4 fragPass1(v2f_img i) : COLOR"
    },
    {
      "line": 56,
      "text": "float4 fragPass2(v2f_img i) : COLOR"
    },
    {
      "line": 65,
      "text": "float4 fragPass3(v2f_img i) : COLOR"
    },
    {
      "line": 74,
      "text": "float4 fragPass4(v2f_img i) : COLOR"
    },
    {
      "line": 83,
      "text": "float fragPass5 (v2f i) : COLOR"
    }
  ]
}

```text

Shader "OverrideEye/OverrideEyeDeferred"
{
	Properties
	{
		_MainTex ("Base (RGB)", 2D) = "white" {}
		_alpha ("Blend Alpha", Range(0.0,1.0)) = 1.0
	}
	
	CGINCLUDE
	#include "UnityCG.cginc"
	sampler2D _MainTex;
	sampler2D _DeferredMaskTex;
	sampler2D _DeferredClipTex;
	float _alpha;
	
	struct BuildInGBufferOutput
	{
		float4 Col0 : COLOR0;
		float4 Col1 : COLOR1;
		float4 Col2 : COLOR2;
		float4 Col3 : COLOR3;
	};	
	
	struct v2f
	{
		float4 pos			: SV_POSITION;
	};
	
	v2f vert (appdata_full v)
	{
		v2f o;
		o.pos = mul (UNITY_MATRIX_MVP, v.vertex);
		return o;
	}		
	
	BuildInGBufferOutput fragPass0 (v2f i)
	{
		BuildInGBufferOutput o;
				
		o.Col0 = float4(0, 0, 0, 0);
		o.Col1 = float4(0, 0, 0, 0);
		o.Col2 = float4(0, 0, 0, 0);
		o.Col3 = float4(0, 0, 0, 0);
					
		return o;			
	}
	
	float4 fragPass1(v2f_img i) : COLOR
	{
		float4 _tex = tex2Dlod(_MainTex, float4(i.uv, 0, 0));
		float _mask = tex2Dlod(_DeferredMaskTex, float4(i.uv, 0, 0)).x;
		float _clip = tex2Dlod(_DeferredClipTex, float4(i.uv, 0, 0)).x;
		return float4(_tex.xyz, _mask * _clip * _alpha);
	}		
	
	float4 fragPass2(v2f_img i) : COLOR
	{
		float4 _tex = tex2Dlod(_MainTex, float4(i.uv, 0, 0));
		float _mask = tex2Dlod(_DeferredMaskTex, float4(i.uv, 0, 0)).x;
		float _clip = tex2Dlod(_DeferredClipTex, float4(i.uv, 0, 0)).x;
		clip( _mask * _clip - 0.0001f);
		return _tex;
	}	
	
	float4 fragPass3(v2f_img i) : COLOR
	{
		float4 _tex = tex2Dlod(_MainTex, float4(i.uv, 0, 0));
		float _mask = tex2Dlod(_DeferredMaskTex, float4(i.uv, 0, 0)).x;
		float _clip = tex2Dlod(_DeferredClipTex, float4(i.uv, 0, 0)).x;
		clip( _mask * _clip - 0.0001f);
		return _tex;
	}	
	
	float4 fragPass4(v2f_img i) : COLOR
	{
		float4 _tex = tex2Dlod(_MainTex, float4(i.uv, 0, 0));
		float _mask = tex2Dlod(_DeferredMaskTex, float4(i.uv, 0, 0)).x;
		float _clip = tex2Dlod(_DeferredClipTex, float4(i.uv, 0, 0)).x;
		clip( _mask * _clip - 0.0001f);
		return _tex;
	}	
	
	float fragPass5 (v2f i) : COLOR
	{
		return 1;
	}	
	
	ENDCG	
	
	SubShader
	{
		//0 Mask
		Pass
		{
			Cull off
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert
			#pragma fragment fragPass0
			ENDCG			
		}
		
	
		//1 Composit 0
		Pass
		{
			ZTest Always Cull Off ZWrite Off
			Blend SrcAlpha OneMinusSrcAlpha
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass1
			ENDCG			
		}		
		
		//2 Composit 1
		Pass
		{
			ZTest Always Cull Off ZWrite Off
			
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass2
			ENDCG			
		}
		
		//3 Composit 2
		Pass
		{
			ZTest Always Cull Off ZWrite Off
				
			
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass3
			ENDCG			
		}
		
		//4 Composit 3
		Pass
		{
			ZTest Always Cull Off ZWrite Off
			Blend SrcAlpha OneMinusSrcAlpha
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass4
			ENDCG			
		}
		
		//5 Render Clip
		Pass
		{
			Cull back
				
			CGPROGRAM
			#pragma target 3.0
			#pragma fragmentoption ARB_precision_hint_fastest
			#pragma vertex vert_img
			#pragma fragment fragPass5
			ENDCG			
		}		
	} 
	FallBack Off
}

```

## Source file: Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader

{
  "path": "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader",
  "blob_sha": "a2821efdc66aa73e88a7406ba692b32e744dd30a",
  "sha256": "e97d3bfbebb67e5b36058d34abdb30390f66fecfc718bc0292052a25bb41c546",
  "lines": 39,
  "bytes": 681,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 1,
    "shader": 2
  },
  "url": "https://github.com/apras/Unity-OverrideEye_For_UnityChan/blob/372c8239167ce3de2d89340721b1b89973a4c57c/Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye.shader",
  "function_signature_candidates": []
}

```text

Shader "UnityChan/Eye"
{
	Properties
	{
		_Color ("Main Color", Color) = (1, 1, 1, 1)
		_ShadowColor ("Shadow Color", Color) = (0.8, 0.8, 1, 1)
		
		_MainTex ("Diffuse", 2D) = "white" {}
		_FalloffSampler ("Falloff Control", 2D) = "white" {}
		_RimLightSampler ("RimLight Control", 2D) = "white" {}
	}

	SubShader
	{
		Tags
		{
			"RenderType"="Opaque"
			"Queue"="Geometry"
			"LightMode"="ForwardBase"
		}

		Pass
		{
			Cull Back
			ZTest LEqual
CGPROGRAM
#pragma multi_compile_fwdbase
#pragma target 3.0
#pragma vertex vert
#pragma fragment frag
#include "UnityCG.cginc"
#include "AutoLight.cginc"
#include "CharaSkin.cg"
ENDCG
		}
	}

	FallBack "Transparent/Cutout/Diffuse"
}


```

## Source file: Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader

{
  "path": "Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader",
  "blob_sha": "4bbd84b2708728600b5a80081635a0cb06d64481",
  "sha256": "db358328bd8e6886bb947014dd8b36ce76781c132f3a9ce28d9d28d820e37772",
  "lines": 41,
  "bytes": 788,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 1,
    "shader": 2
  },
  "url": "https://github.com/apras/Unity-OverrideEye_For_UnityChan/blob/372c8239167ce3de2d89340721b1b89973a4c57c/Assets/UnityChan/Models/UnityChanShader/Shader/Unitychan_chara_eye_blend.shader",
  "function_signature_candidates": []
}

```text

Shader "UnityChan/Eye - Transparent"
{
	Properties
	{
		_Color ("Main Color", Color) = (1, 1, 1, 1)
		_ShadowColor ("Shadow Color", Color) = (0.8, 0.8, 1, 1)
		
		_MainTex ("Diffuse", 2D) = "white" {}
		_FalloffSampler ("Falloff Control", 2D) = "white" {}
		_RimLightSampler ("RimLight Control", 2D) = "white" {}
	}

	SubShader
	{
		Blend SrcAlpha OneMinusSrcAlpha, One One 
		Tags
		{
			"Queue"="Geometry+1" // Transparent+1"
			"IgnoreProjector"="True"
			"RenderType"="Overlay"
			"LightMode"="ForwardBase"
		}

		Pass
		{
			Cull Back
			ZTest LEqual
CGPROGRAM
#pragma multi_compile_fwdbase
#pragma target 3.0
#pragma vertex vert
#pragma fragment frag
#include "UnityCG.cginc"
#include "AutoLight.cginc"
#include "CharaSkin.cg"
ENDCG
		}
	}

	FallBack "Transparent/Cutout/Diffuse"
}


```

# Facial rendering source scan: CustomPhase/CP_SSSSS

Repository: https://github.com/CustomPhase/CP_SSSSS

Pinned source revision: ac46d67bc07770964c55c2fe0976e6a28636835d

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["CP_SSSSS_BlitDepthTextureToDepth.shader", "CP_SSSSS_BlitDepthTextureToDepth.shader.meta", "CP_SSSSS_Main.cs", "CP_SSSSS_Main.cs.meta", "CP_SSSSS_Mask.shader", "CP_SSSSS_Mask.shader.meta", "CP_SSSSS_Object.cs", "CP_SSSSS_Object.cs.meta", "CP_SSSSS_Shader.shader", "CP_SSSSS_Shader.shader.meta"], "shader": ["CP_SSSSS_BlitDepthTextureToDepth.shader", "CP_SSSSS_BlitDepthTextureToDepth.shader.meta", "CP_SSSSS_Mask.shader", "CP_SSSSS_Mask.shader.meta", "CP_SSSSS_Shader.shader", "CP_SSSSS_Shader.shader.meta"]}

## Source file: CP_SSSSS_Shader.shader

{
  "path": "CP_SSSSS_Shader.shader",
  "blob_sha": "62d8201b4273b7a7fc27c26de960af34106a46f7",
  "sha256": "a83156de7928ef44877cbf8a6e139a0c5d1e5eea9fdde0586bc11f1e90e2d4a3",
  "lines": 159,
  "bytes": 3992,
  "selection_score": 42,
  "keyword_line_counts": {
    "eye": 1,
    "sss": 1,
    "shader": 2,
    "rig": 2,
    "lip": 1
  },
  "url": "https://github.com/CustomPhase/CP_SSSSS/blob/ac46d67bc07770964c55c2fe0976e6a28636835d/CP_SSSSS_Shader.shader",
  "function_signature_candidates": [
    {
      "line": 51,
      "text": "inline float SAMPLE_INVERSE_DEPTH(float2 uvs) {"
    },
    {
      "line": 61,
      "text": "inline float SAMPLE_INVERSE_DEPTH_LINEAR(float2 uvs) {"
    },
    {
      "line": 65,
      "text": "float4 frag (v2f i) : SV_Target"
    },
    {
      "line": 142,
      "text": "float4 frag(v2f i) : SV_Target"
    }
  ]
}

```text

﻿Shader "Hidden/CPSSSSSShader"
{
	Properties
	{
		_MainTex ("Texture", 2D) = "white" {}
	}

	CGINCLUDE
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
	ENDCG

	SubShader
	{
		// No culling or depth
		Cull Off ZWrite Off ZTest Always

		//Separable depth based blur
		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag
			
			#include "UnityCG.cginc"
			
			sampler2D _MainTex;
			sampler2D_float _CameraDepthTexture;
			sampler2D _MaskTex;
			fixed4 _BlurVec;
			float _BlurStr;
			float _SoftDepthBias;

			inline float SAMPLE_INVERSE_DEPTH(float2 uvs) {
				float t = unity_CameraProjection._m11;
				float z = LinearEyeDepth(tex2D(_CameraDepthTexture, uvs).r);
				/*
				Voodoo math on visible size multiplier here that i dont understand, but it works
				*/
				float size = 0.5/(z + 0.5);
				return size*t*0.6;
			}

			inline float SAMPLE_INVERSE_DEPTH_LINEAR(float2 uvs) {
				return saturate(1.0 - Linear01Depth(tex2D(_CameraDepthTexture, uvs).r));
			}

			float4 frag (v2f i) : SV_Target
			{
				//If theres nothing in the mask texture, then we dont need to blur/process it
				float4 testMask = tex2D(_MaskTex, i.uv);
				if (testMask.a < 0.003 || testMask.r + testMask.g + testMask.b < 0.005) discard;

				float4 col = tex2D(_MainTex, i.uv);
				fixed2 blurvec = _BlurVec.xy;
				float d = SAMPLE_INVERSE_DEPTH(i.uv);
				float str = _BlurStr * d * testMask.a;

				float dlin = SAMPLE_INVERSE_DEPTH_LINEAR(i.uv);
				if (dlin < 0.1) discard;

				//Gaussian blur
				/*
				col *= 0.38;
				col += tex2D(_MainTex, i.uv + blurvec * 1 * str)*0.18;
				col += tex2D(_MainTex, i.uv - blurvec * 1 * str)*0.18;
				col += tex2D(_MainTex, i.uv + blurvec * 2 * str)*0.09;
				col += tex2D(_MainTex, i.uv - blurvec * 2 * str)*0.09;
				col += tex2D(_MainTex, i.uv + blurvec * 3 * str)*0.04;
				col += tex2D(_MainTex, i.uv - blurvec * 3 * str)*0.04;
				*/

				//Blur with depth check
				/**/
				float sum = 1;
				float diff = 0;
				float cont = 0;

				//HACK:
				//We multiply the depth sample vector by a 1.06 to get rid of the 1 pixel wide lines, 
				//that show up on the edges where the depth difference between samples is significant, 
				//i have no clue whats causing it and how to fix it properly :(
				for (uint n = 1; n <= 2; n++) {
					float contrib_base = 0.99 / (n + 2);

					float4 colr = tex2D(_MainTex, i.uv + blurvec * n * str);
					float dr = SAMPLE_INVERSE_DEPTH_LINEAR(i.uv + blurvec * n * str * 1.06);
					diff = abs(dlin - dr);
					cont = 1.0-saturate(diff / _SoftDepthBias);
					cont *= contrib_base;
					col.rgb += colr.rgb*cont;
					sum += cont;

					float4 coll = tex2D(_MainTex, i.uv - blurvec * n * str);
					float dl = SAMPLE_INVERSE_DEPTH_LINEAR(i.uv - blurvec * n * str * 1.06);
					diff = abs(dlin - dl);
					cont = 1.0 - saturate(diff / _SoftDepthBias);
					cont *= contrib_base;
					col.rgb += coll.rgb*cont;
					sum += cont;
				}
				col.rgb /= sum;
				//return testMask;
				return col;
				//return pow(sum*0.5, 6);
			}
			ENDCG
		}

		//Combine pass
		Pass {
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag

			#include "UnityCG.cginc"

			sampler2D _MainTex;
			sampler2D _BlurTex;
			sampler2D _CameraDepthTexture;
			sampler2D _MaskTex;
			half _EffectStr;
			fixed _PreserveOriginal;

			float4 frag(v2f i) : SV_Target
			{
				float4 src = tex2D(_MainTex, i.uv);
				fixed4 mask = tex2D(_MaskTex, i.uv);
				float fac = 1-pow(saturate(max(max(src.r, src.g), src.b) * 1), 0.5);
				float4 blr = tex2D(_BlurTex, i.uv);
				//return blr;
				//return mask;
				//return src*0.5 + mask*0.5;
				//return src;
				blr = clamp(blr - src*_PreserveOriginal, 0, 50);
				blr.rgb *= mask.rgb;
				return src+blr*fac*_EffectStr*mask.a;
			}
			ENDCG
		}
	}
}


```

## Source file: CP_SSSSS_Mask.shader

{
  "path": "CP_SSSSS_Mask.shader",
  "blob_sha": "6cb96c28ee62cdde7b265e43f2dde25f15bc68ac",
  "sha256": "98f96394523f01efb35f7df2d3b325e84371ef16c7f0cb357f890d63d8b5a948",
  "lines": 94,
  "bytes": 1372,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 1,
    "shader": 4,
    "lip": 2
  },
  "url": "https://github.com/CustomPhase/CP_SSSSS/blob/ac46d67bc07770964c55c2fe0976e6a28636835d/CP_SSSSS_Mask.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Hidden/CPSSSSSMask"
{
	Properties
	{
		_MainTex("Texture", 2D) = "white" {}
		ScatteringColor("Color", Color) = (0,0,0,1)
	}
	SubShader
	{
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
			ZWrite off
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

			sampler2D _MainTex;
			float4 _MainTex_ST;
			fixed4 ScatteringColor;
			
			v2f vert (appdata v)
			{
				v2f o;
				o.vertex = UnityObjectToClipPos(v.vertex);
				o.uv = TRANSFORM_TEX(v.uv, _MainTex);
				return o;
			}
			
			fixed4 frag (v2f i) : SV_Target
			{
				return ScatteringColor * tex2D(_MainTex, i.uv);
			}
			ENDCG
		}
	}

	SubShader
	{
		Tags{ "RenderType" = "Transparent" }
		LOD 100

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag

			#include "UnityCG.cginc"

			struct appdata
			{
				float4 vertex : POSITION;
			};

			struct v2f
			{
				float4 vertex : SV_POSITION;
			};

			v2f vert(appdata v)
			{
				v2f o;
				o.vertex = UnityObjectToClipPos(v.vertex);
				return o;
			}

			fixed4 frag(v2f i) : SV_Target
			{
				// sample the texture
				discard;
				return 0;
			}
			ENDCG
		}
	}
	//Fallback "Legacy Shaders/Diffuse"
}


```

## Source file: CP_SSSSS_BlitDepthTextureToDepth.shader

{
  "path": "CP_SSSSS_BlitDepthTextureToDepth.shader",
  "blob_sha": "c39db47067a447d38088a4d75e43334b41b83841",
  "sha256": "a29a6c4d59a38721b8de53f482ec36420d67d3a7766c66e96e648fe95df3c692",
  "lines": 51,
  "bytes": 1467,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "sss": 1,
    "shader": 3,
    "rig": 1,
    "lip": 1
  },
  "url": "https://github.com/CustomPhase/CP_SSSSS/blob/ac46d67bc07770964c55c2fe0976e6a28636835d/CP_SSSSS_BlitDepthTextureToDepth.shader",
  "function_signature_candidates": [
    {
      "line": 41,
      "text": "float4 frag (v2f i, out float outDepth : SV_Depth) : SV_Target"
    }
  ]
}

```text

// Unity built-in shader source. Copyright (c) 2016 Unity Technologies. MIT license (see license.txt)

Shader "Hidden/CPSSSSSBlitDepthTextureToDepth" {
	SubShader { 
		Pass {
 			ZTest Always Cull Off
            ZWrite On

            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 2.0

			#include "UnityCG.cginc"

			UNITY_DECLARE_DEPTH_TEXTURE(_CameraDepthTexture);

            struct appdata_t {
                float4 vertex : POSITION;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_INPUT_INSTANCE_ID
            };

            struct v2f {
                float4 vertex : SV_POSITION;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_OUTPUT_STEREO
            };

            v2f vert (appdata_t v)
            {
                v2f o;
                UNITY_SETUP_INSTANCE_ID(v);
                UNITY_INITIALIZE_OUTPUT(v2f, o);
                UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(o);
                o.vertex = UnityObjectToClipPos(v.vertex);
				o.texcoord = v.texcoord.xy;
                return o;
            }

            float4 frag (v2f i, out float outDepth : SV_Depth) : SV_Target
            {
                UNITY_SETUP_STEREO_EYE_INDEX_POST_VERTEX(i);
                outDepth = SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, i.texcoord);
                return float4(0, 0, 0, 0);
            }
            ENDCG
		}
	}
	Fallback Off
}


```

## Source file: CP_SSSSS_Main.cs

{
  "path": "CP_SSSSS_Main.cs",
  "blob_sha": "06b7125e311d24b641f2c852d4d88679d005889b",
  "sha256": "759e01d5d54454cf434e47dd8296451127297c0d49ce540d0d41300f4906e1ab",
  "lines": 282,
  "bytes": 7953,
  "selection_score": 15,
  "keyword_line_counts": {
    "subsurface": 1,
    "sss": 24,
    "shader": 23,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/CustomPhase/CP_SSSSS/blob/ac46d67bc07770964c55c2fe0976e6a28636835d/CP_SSSSS_Main.cs",
  "function_signature_candidates": [
    {
      "line": 47,
      "text": "void OnDisable() {"
    },
    {
      "line": 62,
      "text": "void OnEnable()"
    },
    {
      "line": 86,
      "text": "private void OnPreRender()"
    },
    {
      "line": 92,
      "text": "void OnPostRender()"
    },
    {
      "line": 108,
      "text": "public static bool HasMaterialSSSParameter(Material material)"
    },
    {
      "line": 149,
      "text": "public void AddRenderer(Renderer renderer, int subMeshIndex, SSSParameter sssParameter)"
    },
    {
      "line": 168,
      "text": "void Reset()"
    },
    {
      "line": 175,
      "text": "void ApplyBuffer()"
    },
    {
      "line": 182,
      "text": "void UpdateBuffer()"
    },
    {
      "line": 240,
      "text": "void CleanupBuffer()"
    },
    {
      "line": 250,
      "text": "public void AddMakeSSSMaskCommands(CommandBuffer buffer)"
    }
  ]
}

```text

﻿using UnityEngine;
using UnityEngine.Rendering;
using System.Collections.Generic;

[RequireComponent(typeof(Camera))]
#if UNITY_5_4_OR_NEWER
[ImageEffectAllowedInSceneView]
#endif
[ImageEffectOpaque]
[ExecuteInEditMode]
public class CP_SSSSS_Main : MonoBehaviour
{
	public Shader shader;
	public Shader maskShader;
	public Shader copyDepthShader;

	CommandBuffer buffer;
	CameraEvent camEvent = CameraEvent.BeforeImageEffectsOpaque;

	private Material m_Material;
	Material material
	{
		get
		{
			if (m_Material == null && shader != null)
			{
				m_Material = new Material(shader);
				m_Material.hideFlags = HideFlags.HideAndDontSave;
			}
			return m_Material;
		}
	}
		
	[Range(1,3)]
	public int downscale = 1;
	[Range(1, 3)]
	public int blurIterations = 1;
	[Range(0.01f, 1.6f)]
	public float scatterDistance = 0.4f;
	[Range(0f,2f)]
	public float scatterIntensity = 1f;
	[Range(0.001f, 0.3f)]
	public float softDepthBias = 0.05f;
	[Range(0f, 1f)]
	public float affectDirect = 0.5f;

	void OnDisable() {
		if (m_Material)
		{
			DestroyImmediate(m_Material);
		}
		if(m_CopyDepthMaterial) DestroyImmediate(m_CopyDepthMaterial);
		foreach(var i in m_MaskMaterials) DestroyImmediate(i);

		m_Material = null;
		m_CopyDepthMaterial = null;
		m_MaskMaterials.Clear();

		CleanupBuffer();
	}

	void OnEnable()
	{
		if (!SystemInfo.supportsImageEffects)
		{
			enabled = false;
			return;
		}

		CleanupBuffer();

		// Disable the image effect if the shader can't
		// run on the users graphics card
		if(shader && !shader.isSupported)
		{
			enabled = false;
			return;
		}
		if(buffer == null) ApplyBuffer();

		// cannot call Shader.PropertyToID() at compiler time, so let's do it here
		scatteringMapID = Shader.PropertyToID("ScatteringMap");
		scatteringColorID = Shader.PropertyToID("ScatteringColor");
	}

	private void OnPreRender()
	{
		if(buffer == null) ApplyBuffer();
		if(buffer != null) UpdateBuffer();
	}

	void OnPostRender()
	{
		TargetMeshes.Clear();
	}

	public struct SSSParameter
	{
		public Texture scatteringMap;
		public Vector2 scatteringMapOffset;
		public Vector2 scatteringMapScale;
		public Color scatteringColor;
	}
	
	static int scatteringMapID; // = Shader.PropertyToID("ScatteringMap");
	static int scatteringColorID; // = Shader.PropertyToID("ScatteringColor");

	public static bool HasMaterialSSSParameter(Material material)
	{
		return material.HasProperty(scatteringColorID) && material.HasProperty(scatteringMapID);
	}

	public static SSSParameter MakeSSSParameterFromMaterial(Material material)
	{
		return new SSSParameter {
			scatteringMap = material.GetTexture(scatteringMapID),
			scatteringMapOffset = material.mainTextureOffset,
			scatteringMapScale = material.mainTextureScale,
			scatteringColor = material.GetColor(scatteringColorID),
		};
	}

	public static SSSParameter MakeSSSParameterFromTextureAndColor(TextureAndColor src)
	{
		return new SSSParameter {
			scatteringMap = src.texture,
			scatteringMapOffset = Vector2.zero,
			scatteringMapScale = Vector2.one,
			scatteringColor = src.color,
		};
	}

	public struct TargetMesh
	{
		public TargetMesh(Renderer renderer, int subMeshIndex, SSSParameter sssParameter)
		{
			this.renderer = renderer;
			this.subMeshIndex = subMeshIndex;
			this.sssParameter = sssParameter;
		}
		public Renderer renderer;
		public int subMeshIndex;
		public SSSParameter sssParameter;
	}

	List<TargetMesh> m_TargetMeshes = new List<TargetMesh>(256);
	List<TargetMesh> TargetMeshes { get { return m_TargetMeshes; } }

	public void AddRenderer(Renderer renderer, int subMeshIndex, SSSParameter sssParameter)
	{
		TargetMeshes.Add(new TargetMesh(renderer, subMeshIndex, sssParameter));
	}

	List<Material> m_MaskMaterials = new List<Material>();

	Material m_CopyDepthMaterial;
	Material CopyDepthMaterial {
		get {
			if(m_CopyDepthMaterial == null && copyDepthShader) {
				m_CopyDepthMaterial = new Material(copyDepthShader) {
					hideFlags = HideFlags.HideAndDontSave
				};
			}
			return m_CopyDepthMaterial;
		}
	}
    
	void Reset()
	{
		shader = Shader.Find("Hidden/CPSSSSSShader");
		maskShader = Shader.Find("Hidden/CPSSSSSMask");
		copyDepthShader = Shader.Find("Hidden/CPSSSSSBlitDepthTextureToDepth");
	}
	
	void ApplyBuffer()
	{
		buffer = new CommandBuffer();
		buffer.name = "Screen Space Subsurface Scattering";
		GetComponent<Camera>().AddCommandBuffer(camEvent, buffer);	
	}

	void UpdateBuffer()
	{
		buffer.Clear();
		
		if(TargetMeshes.Count == 0) return;

		AddMakeSSSMaskCommands(buffer);

		int blurRT1 = Shader.PropertyToID("_CPSSSSSBlur1");
		int blurRT2 = Shader.PropertyToID("_CPSSSSSBlur2");
		int src = Shader.PropertyToID("_CPSSSSSSource");
		int w = -1;
		int h = -1;
		Camera cam = Camera.current;
		if (cam != null)
		{
			w = cam.pixelWidth / downscale;
			h = cam.pixelHeight / downscale;
		}

		//buffer.GetTemporaryRT(blurRT1, -1, -1, 16, FilterMode.Bilinear, RenderTextureFormat.ARGBFloat);
		buffer.GetTemporaryRT(blurRT1, w, h, 16, FilterMode.Bilinear, RenderTextureFormat.ARGBFloat);
		//buffer.GetTemporaryRT(blurRT2, -1, -1, 16, FilterMode.Bilinear, RenderTextureFormat.ARGBFloat);
		buffer.GetTemporaryRT(blurRT2, w, h, 16, FilterMode.Bilinear, RenderTextureFormat.ARGBFloat);
		buffer.GetTemporaryRT(src, -1, -1, 24, FilterMode.Bilinear, RenderTextureFormat.ARGBFloat);
		buffer.SetGlobalFloat("_SoftDepthBias", softDepthBias * 0.05f * 0.2f);

		buffer.Blit(BuiltinRenderTextureType.CameraTarget, blurRT2);
		//buffer.Blit(BuiltinRenderTextureType.CurrentActive, sourceBuf);
		buffer.Blit(BuiltinRenderTextureType.CameraTarget, src);

		//multipass pass blur
		for (int k = 1; k <= blurIterations; k++)
		{
			buffer.SetGlobalFloat("_BlurStr", Mathf.Clamp01(scatterDistance * 0.08f - k * 0.022f * scatterDistance));
			buffer.SetGlobalVector("_BlurVec", new Vector4(1, 0, 0, 0));
			buffer.Blit(blurRT2, blurRT1, material, 0);
			buffer.SetGlobalVector("_BlurVec", new Vector4(0, 1, 0, 0));
			buffer.Blit(blurRT1, blurRT2, material, 0);

			buffer.SetGlobalVector("_BlurVec", new Vector4(1, 1, 0, 0).normalized);
			buffer.Blit(blurRT2, blurRT1, material, 0);
			buffer.SetGlobalVector("_BlurVec", new Vector4(-1, 1, 0, 0).normalized);
			buffer.Blit(blurRT1, blurRT2, material, 0);
		}
		
		//buffer.Blit(blurRT2, blurBuf);

		buffer.SetGlobalTexture("_BlurTex", blurRT2);
		buffer.SetGlobalFloat("_EffectStr", scatterIntensity);
		buffer.SetGlobalFloat("_PreserveOriginal", 1 - affectDirect);
		buffer.Blit(src, BuiltinRenderTextureType.CameraTarget, material, 1);

		buffer.ReleaseTemporaryRT(blurRT1);
		buffer.ReleaseTemporaryRT(blurRT2);
		buffer.ReleaseTemporaryRT(src);
	}

	void CleanupBuffer()
	{
		if (buffer!=null)
		{
			buffer.Clear();
			GetComponent<Camera>().RemoveCommandBuffer(camEvent, buffer);
			buffer = null;
		}
	}

	public void AddMakeSSSMaskCommands(CommandBuffer buffer)
	{
		if(TargetMeshes.Count == 0) return;

		for(int i = m_MaskMaterials.Count; i < TargetMeshes.Count; i++) {
			if(maskShader != null) {
				var material = new Material(maskShader) {
					hideFlags = HideFlags.HideAndDontSave
				};
				m_MaskMaterials.Add(material);
			}
		}

		if(m_MaskMaterials.Count < TargetMeshes.Count) return;

		int maskRT = Shader.PropertyToID("_MaskTex");
		buffer.GetTemporaryRT(maskRT, -1, -1, 24, FilterMode.Bilinear, RenderTextureFormat.ARGB32);
		buffer.Blit(BuiltinRenderTextureType.None, maskRT, CopyDepthMaterial);
		buffer.SetRenderTarget(maskRT);

		for(int i = 0; i < TargetMeshes.Count; i++) {
			var ii = TargetMeshes[i];
			var maskMaterial = m_MaskMaterials[i];
			maskMaterial.mainTexture = ii.sssParameter.scatteringMap;
			maskMaterial.mainTextureOffset = ii.sssParameter.scatteringMapOffset;
			maskMaterial.mainTextureScale = ii.sssParameter.scatteringMapScale;
			maskMaterial.SetColor(scatteringColorID, ii.sssParameter.scatteringColor);
			buffer.DrawRenderer(ii.renderer, maskMaterial, ii.subMeshIndex);
		}

		buffer.SetGlobalTexture("_MaskTex", maskRT);
	}
}


```

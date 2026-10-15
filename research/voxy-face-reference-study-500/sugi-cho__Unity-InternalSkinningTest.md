# Facial rendering source scan: sugi-cho/Unity-InternalSkinningTest

Repository: https://github.com/sugi-cho/Unity-InternalSkinningTest

Pinned source revision: dab7cc3808d20e71ac5035f3886f0f1f55e25ee0

Discovery topic: skin shader

Description: test code for Unity built-in shader internalskinning.compute

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/SkinningCompute.meta", "Assets/SkinningCompute/Scripts.meta", "Assets/SkinningCompute/Scripts/SetBuffer.cs", "Assets/SkinningCompute/Scripts/SetBuffer.cs.meta", "Assets/SkinningCompute/Scripts/Skinning.cs", "Assets/SkinningCompute/Scripts/Skinning.cs.meta", "Assets/SkinningCompute/Shaders.meta", "Assets/SkinningCompute/Shaders/VertOutVisualizer.shader", "Assets/SkinningCompute/Shaders/VertOutVisualizer.shader.meta", "Assets/SkinningCompute/Shaders/internalskinning.compute", "Assets/SkinningCompute/Shaders/internalskinning.compute.meta", "Assets/SkinningCompute/Shaders/skinning.compute", "Assets/SkinningCompute/Shaders/skinning.compute.meta", "Assets/SkinningCompute/Shaders/vertexSkinning.shader", "Assets/SkinningCompute/Shaders/vertexSkinning.shader.meta", "Assets/SkinningCompute/main.unity", "Assets/SkinningCompute/main.unity.meta", "Assets/SkinningCompute/showVertOut.mat", "Assets/SkinningCompute/showVertOut.mat.meta", "Assets/SkinningCompute/vertexSkinning.mat", "Assets/SkinningCompute/vertexSkinning.mat.meta"], "shader": ["Assets/SkinningCompute/Shaders.meta", "Assets/SkinningCompute/Shaders/VertOutVisualizer.shader", "Assets/SkinningCompute/Shaders/VertOutVisualizer.shader.meta", "Assets/SkinningCompute/Shaders/internalskinning.compute", "Assets/SkinningCompute/Shaders/internalskinning.compute.meta", "Assets/SkinningCompute/Shaders/skinning.compute", "Assets/SkinningCompute/Shaders/skinning.compute.meta", "Assets/SkinningCompute/Shaders/vertexSkinning.shader", "Assets/SkinningCompute/Shaders/vertexSkinning.shader.meta"]}

## Source file: Assets/SkinningCompute/Shaders/vertexSkinning.shader

{
  "path": "Assets/SkinningCompute/Shaders/vertexSkinning.shader",
  "blob_sha": "3304bd1961fdde3f02073bb3fac3e75fb196e7f0",
  "sha256": "ab6e6cdb3c7ec5b18297596a4d4f4a154c2edb882f0e0ab2ca78c6f96dec2a52",
  "lines": 71,
  "bytes": 1409,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 4,
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/sugi-cho/Unity-InternalSkinningTest/blob/dab7cc3808d20e71ac5035f3886f0f1f55e25ee0/Assets/SkinningCompute/Shaders/vertexSkinning.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Unlit/vertexSkinning"
{
	Properties
	{
		_MainTex("Texture", 2D) = "white" {}
	}
		SubShader
	{
		Tags { "RenderType" = "Opaque" }
		LOD 100

		Pass
		{
			CGPROGRAM
			#pragma vertex vert
			#pragma fragment frag

			#include "UnityCG.cginc"


			struct SVertInSkin
			{
				float weight0,weight1,weight2,weight3;
				int index0,index1,index2,index3;
			};
			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;
			};

			struct v2f
			{
				float2 uv : TEXCOORD0;
				float4 vertex : SV_POSITION;
				half4 color : COLOR;
			};

			StructuredBuffer<SVertInSkin> _Skin;
			StructuredBuffer<float4x4> _Bones;
			sampler2D _MainTex;
			float4 _MainTex_ST;

			v2f vert(appdata v, uint vIdx : SV_VertexID)
			{
				SVertInSkin si = _Skin[vIdx];

				float3 vP = v.vertex.xyz;
				float3 vPacc = float3(0, 0, 0);

				vPacc += si.weight0*mul(_Bones[si.index0], float4(vP, 1)).xyz;
				vPacc += si.weight1*mul(_Bones[si.index1], float4(vP, 1)).xyz;
				vPacc += si.weight2*mul(_Bones[si.index2], float4(vP, 1)).xyz;
				vPacc += si.weight3*mul(_Bones[si.index3], float4(vP, 1)).xyz;

				v2f o;
				o.vertex = UnityObjectToClipPos(vPacc);
				o.uv = TRANSFORM_TEX(v.uv, _MainTex);
				o.color = half4(si.index0, si.index1, si.index2, si.index3) / 43.0;
				return o;
			}

			fixed4 frag(v2f i) : SV_Target
			{
				fixed4 col = i.color;
				return col;
			}
			ENDCG
		}
	}
}


```

## Source file: Assets/SkinningCompute/Shaders/VertOutVisualizer.shader

{
  "path": "Assets/SkinningCompute/Shaders/VertOutVisualizer.shader",
  "blob_sha": "152348bce5705c7e4d5bb86dcc3f2f6ead465b7d",
  "sha256": "46d90a4d2b8a91b767550ad4fef8e3ffb16e98fb2a8e9862062ce5641a6df6f9",
  "lines": 79,
  "bytes": 2046,
  "selection_score": 18,
  "keyword_line_counts": {
    "shader": 6,
    "face": 2
  },
  "url": "https://github.com/sugi-cho/Unity-InternalSkinningTest/blob/dab7cc3808d20e71ac5035f3886f0f1f55e25ee0/Assets/SkinningCompute/Shaders/VertOutVisualizer.shader",
  "function_signature_candidates": [
    {
      "line": 58,
      "text": "void vert(inout appdata v) {"
    },
    {
      "line": 67,
      "text": "void surf (Input IN, inout SurfaceOutputStandard o) {"
    }
  ]
}

```text

﻿Shader "Custom/VertOutVisualizer" {
	Properties {
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200

		CGPROGRAM
		// Physically based Standard lighting model, and enable shadows on all light types
		#pragma surface surf Standard addshadow vertex:vert

		// Use shader model 3.0 target, to get nicer looking lighting
		#pragma target 5.0

		sampler2D _MainTex;

		struct Input {
			float2 uv_MainTex;
		};
		struct SVertOut
		{
			float3 pos;
			float3 norm;
			float4 tang;
		};

#ifdef SHADER_API_D3D11
		uniform StructuredBuffer<SVertOut> _VertIn;
#endif
		half _Glossiness;
		half _Metallic;
		fixed4 _Color;

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_BUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_BUFFER_END(Props)

		struct appdata {
			float4 vertex : POSITION;
			float4 tangent : TANGENT;
			float3 normal : NORMAL;
			float4 texcoord : TEXCOORD0;
			float4 texcoord1 : TEXCOORD1;
			float4 texcoord2 : TEXCOORD2;
			float4 texcoord3 : TEXCOORD3;
			fixed4 color : COLOR;
			UNITY_VERTEX_INPUT_INSTANCE_ID
			uint vId : SV_VertexID;
		};

		void vert(inout appdata v) {
#ifdef SHADER_API_D3D11
			SVertOut vin = _VertIn[v.vId];
			v.vertex.xyz = vin.pos;
			v.normal = vin.norm;
			v.tangent = vin.tang;
#endif
		}

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

## Source file: Assets/SkinningCompute/Scripts/Skinning.cs

{
  "path": "Assets/SkinningCompute/Scripts/Skinning.cs",
  "blob_sha": "051487695abffb9e61cbec5d65f03d27c9bf9d50",
  "sha256": "8258b3aa3cd652ccf4825c7905bc153fb2c6e08c99161a4333c11ff428d50999",
  "lines": 125,
  "bytes": 3677,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 16,
    "shader": 1
  },
  "url": "https://github.com/sugi-cho/Unity-InternalSkinningTest/blob/dab7cc3808d20e71ac5035f3886f0f1f55e25ee0/Assets/SkinningCompute/Scripts/Skinning.cs",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "void Start()"
    },
    {
      "line": 50,
      "text": "private void OnDestroy()"
    },
    {
      "line": 56,
      "text": "void Update()"
    },
    {
      "line": 62,
      "text": "void SetBoneMatrices()"
    },
    {
      "line": 69,
      "text": "void ComputeSkinning()"
    },
    {
      "line": 80,
      "text": "void Initialize()"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.Events;

public class Skinning : MonoBehaviour
{

    public ComputeShader cs;
    public ComputeBufferEvent onCreateOutBuffer;
    public DoubleComputeBufferEvent setSkinAndBone;
    [SerializeField] Transform[] bones;
    struct SVertInVBO
    {
        public Vector3 pos;
        public Vector3 norm;
        public Vector4 tang;
    }

    struct SVertInSkin
    {
        public float weight0, weight1, weight2, weight3;
        public int index0, index1, index2, index3;
    }

    struct SVertOut
    {
        Vector3 pos;
        Vector3 norm;
        Vector4 tang;
    }

    Mesh mesh;
    int vertCount;
    ComputeBuffer sourceVBO;
    ComputeBuffer sourceSkin;
    ComputeBuffer meshVertsOut;
    ComputeBuffer mBones;

    Matrix4x4[] boneMatrices;

    // Use this for initialization
    void Start()
    {
        Initialize();
    }

    private void OnDestroy()
    {
        new[] { sourceVBO, sourceSkin, meshVertsOut, mBones }.ToList().ForEach(b => b.Dispose());
    }

    // Update is called once per frame
    void Update()
    {
        SetBoneMatrices();
        ComputeSkinning();
    }

    void SetBoneMatrices()
    {
        for (var i = 0; i < boneMatrices.Length; i++)
            boneMatrices[i] = transform.worldToLocalMatrix * bones[i].localToWorldMatrix * mesh.bindposes[i];
        mBones.SetData(boneMatrices);
    }

    void ComputeSkinning()
    {
        var kernel = cs.FindKernel("main");
        cs.SetBuffer(kernel, "g_SourceVBO", sourceVBO);
        cs.SetBuffer(kernel, "g_SourceSkin", sourceSkin);
        cs.SetBuffer(kernel, "g_MeshVertsOut", meshVertsOut);
        cs.SetBuffer(kernel, "g_mBones", mBones);
        cs.SetInt("g_VertCount", vertCount);
        cs.Dispatch(kernel, vertCount / 64 + 1, 1, 1);
    }

    void Initialize()
    {
        var skin = GetComponentInChildren<SkinnedMeshRenderer>();
        mesh = skin.sharedMesh;
        vertCount = mesh.vertexCount;

        var inVBO = Enumerable.Range(0, vertCount).Select(
            idx => new SVertInVBO()
            {
                pos = mesh.vertices[idx],
                norm = mesh.normals[idx],
                tang = mesh.tangents[idx],
            }).ToArray();
        sourceVBO = new ComputeBuffer(vertCount, Marshal.SizeOf(typeof(SVertInVBO)));
        sourceVBO.SetData(inVBO);

        var inSkin = mesh.boneWeights.Select(
            w => new SVertInSkin()
            {
                weight0 = w.weight0,
                weight1 = w.weight1,
                weight2 = w.weight2,
                weight3 = w.weight3,
                index0 = w.boneIndex0,
                index1 = w.boneIndex1,
                index2 = w.boneIndex2,
                index3 = w.boneIndex3,
            }).ToArray();
        sourceSkin = new ComputeBuffer(vertCount, Marshal.SizeOf(typeof(SVertInSkin)));
        sourceSkin.SetData(inSkin);

        meshVertsOut = new ComputeBuffer(vertCount, Marshal.SizeOf(typeof(SVertOut)));
        onCreateOutBuffer.Invoke(meshVertsOut);

        bones = skin.bones;
        mBones = new ComputeBuffer(bones.Length, Marshal.SizeOf(typeof(Matrix4x4)));
        boneMatrices = bones.Select((b, idx) => transform.worldToLocalMatrix * b.localToWorldMatrix * mesh.bindposes[idx]).ToArray();

        setSkinAndBone.Invoke(sourceSkin, mBones);
    }

    [System.Serializable]
    public class ComputeBufferEvent : UnityEvent<ComputeBuffer> { }
    [System.Serializable]
    public class DoubleComputeBufferEvent : UnityEvent<ComputeBuffer, ComputeBuffer> { }
}


```

## Source file: Assets/SkinningCompute/Scripts/SetBuffer.cs

{
  "path": "Assets/SkinningCompute/Scripts/SetBuffer.cs",
  "blob_sha": "2c822944a644f122cc626e1a0a08b83ccb91bfd8",
  "sha256": "e2831ab48c7b995634381de714e58c94b91a859ca46bc1b68cdcb32599232f00",
  "lines": 25,
  "bytes": 677,
  "selection_score": 3,
  "keyword_line_counts": {
    "skin": 2
  },
  "url": "https://github.com/sugi-cho/Unity-InternalSkinningTest/blob/dab7cc3808d20e71ac5035f3886f0f1f55e25ee0/Assets/SkinningCompute/Scripts/SetBuffer.cs",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "public void Set(ComputeBuffer buffer)"
    },
    {
      "line": 16,
      "text": "public void SetSkinAndBones(ComputeBuffer skin, ComputeBuffer bones)"
    }
  ]
}

```text

﻿using UnityEngine;

public class SetBuffer : MonoBehaviour {

    public string propName = "_VertIn";

    public void Set(ComputeBuffer buffer)
    {
        var r = GetComponent<Renderer>();
        var mpb = new MaterialPropertyBlock();
        r.GetPropertyBlock(mpb);
        mpb.SetBuffer(propName, buffer);
        r.SetPropertyBlock(mpb);
    }

    public void SetSkinAndBones(ComputeBuffer skin, ComputeBuffer bones)
    {
        var r = GetComponent<Renderer>();
        var mpb = new MaterialPropertyBlock();
        r.GetPropertyBlock(mpb);
        mpb.SetBuffer("_Skin", skin);
        mpb.SetBuffer("_Bones", bones);
        r.SetPropertyBlock(mpb);
    }
}


```

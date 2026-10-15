# Facial rendering source scan: Sushiy/AGT_SSS

Repository: https://github.com/Sushiy/AGT_SSS

Pinned source revision: 72351588fd45c1753f437809d084ad5c11f7d40b

Discovery topic: subsurface scattering

Description: University Subsurface Scattering project

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"subsurface": ["Assets/Scripts/SSS_SubsurfaceScattering.cs", "Assets/Scripts/SSS_SubsurfaceScattering.cs.meta", "Assets/Shaders/SubsurfaceScattering (Transparent).shader", "Assets/Shaders/SubsurfaceScattering (Transparent).shader.meta", "Assets/Shaders/SubsurfaceScattering.shader", "Assets/Shaders/SubsurfaceScattering.shader.meta"], "sss": ["Assets/Materials/SSS_Debug.mat", "Assets/Materials/SSS_Debug.mat.meta", "Assets/Materials/SSS_Ice.mat", "Assets/Materials/SSS_Ice.mat.meta", "Assets/Materials/SSS_Jade.mat", "Assets/Materials/SSS_Jade.mat.meta", "Assets/Materials/SSS_Jelly.mat", "Assets/Materials/SSS_Jelly.mat.meta", "Assets/Materials/SSS_Waxy.mat", "Assets/Materials/SSS_Waxy.mat.meta", "Assets/Scripts/SSS_Controller.cs", "Assets/Scripts/SSS_Controller.cs.meta", "Assets/Scripts/SSS_FetchDepth.cs", "Assets/Scripts/SSS_FetchDepth.cs.meta", "Assets/Scripts/SSS_Object.cs", "Assets/Scripts/SSS_Object.cs.meta", "Assets/Scripts/SSS_SubsurfaceScattering.cs", "Assets/Scripts/SSS_SubsurfaceScattering.cs.meta"], "shader": ["Assets/Shaders.meta", "Assets/Shaders/RenderDepth.shader", "Assets/Shaders/RenderDepth.shader.meta", "Assets/Shaders/SubsurfaceScattering (Transparent).shader", "Assets/Shaders/SubsurfaceScattering (Transparent).shader.meta", "Assets/Shaders/SubsurfaceScattering.shader", "Assets/Shaders/SubsurfaceScattering.shader.meta"], "face": ["Assets/Scripts/SSS_SubsurfaceScattering.cs", "Assets/Scripts/SSS_SubsurfaceScattering.cs.meta", "Assets/Shaders/SubsurfaceScattering (Transparent).shader", "Assets/Shaders/SubsurfaceScattering (Transparent).shader.meta", "Assets/Shaders/SubsurfaceScattering.shader", "Assets/Shaders/SubsurfaceScattering.shader.meta"]}

## Source file: Assets/Scripts/SSS_SubsurfaceScattering.cs

{
  "path": "Assets/Scripts/SSS_SubsurfaceScattering.cs",
  "blob_sha": "b3c5c572136c287083376bf73efa3482579aaed5",
  "sha256": "273720cfc4a0533264a4f8d8cbcd38e0630e170d55cdc60131dd3fc619829dc2",
  "lines": 50,
  "bytes": 1974,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 4,
    "sss": 1,
    "face": 4,
    "lip": 1
  },
  "url": "https://github.com/Sushiy/AGT_SSS/blob/72351588fd45c1753f437809d084ad5c11f7d40b/Assets/Scripts/SSS_SubsurfaceScattering.cs",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "void Start ()"
    },
    {
      "line": 37,
      "text": "void OnRenderImage(RenderTexture _source, RenderTexture _destination)"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;

//Passes all necessary SceneData to the Materials 
public class SSS_SubsurfaceScattering : MonoBehaviour 
{    
    //Camera which renders the DepthMap
    [SerializeField]
    private Camera m_camDepthCamera;
    
    //Light which is represented by the camera
    [SerializeField]
    private Light m_lightScatterLight;

    //RenderTexture which receives the DepthMap
    [SerializeField]
    private RenderTexture m_texDepthMap;
    
    //Different SubsurfaceScatteringMaterials
    public Material[] m_arrmatSubSurfaceMaterials;

    // Use this for initialization
    void Start () 
    {
        if(m_camDepthCamera == null)
        {
            Debug.LogError("DepthCamera was not assigned");
        }
        if (m_texDepthMap == null)
        {
            Debug.LogError("DepthMap was not assigned");
        }
    }

    //This is on the maincamera, so this is called every frame
    void OnRenderImage(RenderTexture _source, RenderTexture _destination)
    {
        //We have several Materials to show, so we pass the values to all of them
        foreach(Material mat in m_arrmatSubSurfaceMaterials)
        {
            mat.SetTexture("_DepthMap", m_texDepthMap);                             //DeptMap Texture
            mat.SetMatrix("_LightMatrix", m_camDepthCamera.worldToCameraMatrix);    //WorldToCamera Matrix
            mat.SetMatrix("_LightProjectionMatrix", GL.GetGPUProjectionMatrix(m_camDepthCamera.projectionMatrix, false));   //CameraProjectionMatrix
            mat.SetMatrix("_LightTexMatrix", m_camDepthCamera.projectionMatrix * m_camDepthCamera.worldToCameraMatrix);     //Combined MVP Matrix
            mat.SetColor("_LightColor", m_lightScatterLight.color * m_lightScatterLight.intensity);     //The Color of the Light
            mat.SetFloat("_CameraFarPlane", m_camDepthCamera.farClipPlane);     //The cameras FarClipping plane
        }
    }
}


```

## Source file: Assets/Shaders/SubsurfaceScattering.shader

{
  "path": "Assets/Shaders/SubsurfaceScattering.shader",
  "blob_sha": "e90e0db7af69e47f7c9116f7b40390e802949066",
  "sha256": "01ef04590b7c65eb7e12ec7afecf6e1129b91aba9d0e4de0afad72c8ba362f43",
  "lines": 82,
  "bytes": 2449,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 1,
    "shader": 3,
    "face": 3
  },
  "url": "https://github.com/Sushiy/AGT_SSS/blob/72351588fd45c1753f437809d084ad5c11f7d40b/Assets/Shaders/SubsurfaceScattering.shader",
  "function_signature_candidates": [
    {
      "line": 41,
      "text": "void vert(inout appdata_full v, out Input o) {"
    },
    {
      "line": 46,
      "text": "void surf (Input IN, inout SurfaceOutputStandard o)"
    }
  ]
}

```text

﻿Shader "Subsurface Scattering/SubsurfaceScattering" {
	Properties {
		_Color ("Color", Color) = (1,1,1,1)
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		_DepthMap ("Depth", 2D) =  "white" {}
		_ScatterMap("Scatter", 2D) = "" {}

	}
	SubShader {
		Tags { "RenderType"="Opaque" }
		LOD 200
		
		CGPROGRAM
		//#include "UnityDeferredLibrary.cginc"
		#include "UnityCG.cginc"
		#include "Lighting.cginc"
		// Physically based Standard lighting model, and enable shadows on all light types
		#pragma surface surf Standard fullforwardshadows vertex:vert

		// Use shader model 3.0 target, to get nicer looking lighting
		#pragma target 5.0

		sampler2D _DepthMap;
		sampler2D _ScatterMap;
		float _CameraFarPlane;
		float4x4 _LightProjectionMatrix;
		float4x4 _LightMatrix;
		float4x4 _LightTexMatrix;
		float4 _LightColor;

		struct Input 
		{
			float3 localPos;
		};

		half _Glossiness;
		half _Metallic;
		fixed4 _Color;

		void vert(inout appdata_full v, out Input o) {
			UNITY_INITIALIZE_OUTPUT(Input, o);
			o.localPos = v.vertex.xyz;
		}

		void surf (Input IN, inout SurfaceOutputStandard o) 
		{
			//Calculate WorldSpace Position of fragment
			float4 worldPos = mul(unity_ObjectToWorld, float4(IN.localPos.xyz, 1.0f));

            //Translate point into light space
            float4 lightPos = mul(_LightMatrix, worldPos);

			//Translate point into lightProjectionSpace
			float4 lightTexCoords = mul(_LightTexMatrix, worldPos);

			//transform coordinates into texture coordinates
			lightTexCoords.xy /= lightTexCoords.w;
			lightTexCoords.x = 0.5 * lightTexCoords.x + 0.5f;
			lightTexCoords.y = 0.5 * lightTexCoords.y + 0.5f;

            //get the depth from the DepthMap and set it into the range 0-1
			float dist_IN = Linear01Depth(DecodeFloatRGBA(tex2D(_DepthMap, lightTexCoords.xy)));
            
			//calculate the distance from the Light to the current point
            float dist_OUT = length(lightPos)/ _CameraFarPlane;

			//Calculate distance traveled through the object
			float dist_THROUGH = dist_OUT - dist_IN;

			//Albedo is sampled from the ScatterMap and multiplied with the Color/Intensity of the light
			o.Albedo = tex2D(_ScatterMap, float2(dist_THROUGH, 0.0f)).rgb * _LightColor.rgb;
			
			// Metallic and smoothness come from slider variables
			o.Metallic = _Metallic;
			o.Smoothness = _Glossiness;
			o.Alpha = 1.0f;
		}
		ENDCG
	}
	FallBack "Diffuse"
}


```

## Source file: Assets/Shaders/SubsurfaceScattering (Transparent).shader

{
  "path": "Assets/Shaders/SubsurfaceScattering (Transparent).shader",
  "blob_sha": "b8ffd2e5a089a4f4e3767394b2daad66c3dee139",
  "sha256": "45e4903b76bac265cd17418558a2da94149e4e5ed1137f15d025a1bd3213e457",
  "lines": 90,
  "bytes": 2787,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 2,
    "shader": 4,
    "face": 4
  },
  "url": "https://github.com/Sushiy/AGT_SSS/blob/72351588fd45c1753f437809d084ad5c11f7d40b/Assets/Shaders/SubsurfaceScattering%20%28Transparent%29.shader",
  "function_signature_candidates": [
    {
      "line": 48,
      "text": "void vert(inout appdata_full v, out Input o) {"
    },
    {
      "line": 53,
      "text": "void surf(Input IN, inout SurfaceOutputStandard o)"
    }
  ]
}

```text

﻿Shader "Subsurface Scattering/SubsurfaceScattering(Transparent)" 
{
	//Transparent Variant of the SubSurfaceShader

	Properties{
		_Color("Color", Color) = (1,1,1,1)
		_Alpha("Alpha", Range(0,1)) = 0.5
		_Glossiness("Smoothness", Range(0,1)) = 0.5
		_Metallic("Metallic", Range(0,1)) = 0.0
		_DepthMap("Depth", 2D) = "white" {}
		_ScatterMap("Scatter", 2D) = "" {}
		_AlphaFalloffMap("AlphaFalloff", 2D) = "" {}
	}
		SubShader{
		Tags{ "Queue" = "Transparent" "RenderType" = "Transparent" }
		LOD 200

		CGPROGRAM
		//#include "UnityDeferredLibrary.cginc"
#include "UnityCG.cginc"
#include "Lighting.cginc"
		// Physically based Standard lighting model, and enable shadows on all light types
#pragma surface surf Standard alpha fullforwardshadows  vertex:vert

		// Use shader model 3.0 target, to get nicer looking lighting
#pragma target 5.0

	sampler2D _DepthMap;
	sampler2D _ScatterMap;
	sampler2D _AlphaFalloffMap;
	float _CameraFarPlane;
	float4x4 _LightProjectionMatrix;
	float4x4 _LightMatrix;
	float4x4 _LightTexMatrix;
	float4 _LightColor;

	struct Input
	{
		float3 worldPos;
		float3 localPos;
	};

	half _Glossiness;
	half _Metallic;
	float _Alpha;
	fixed4 _Color;

	void vert(inout appdata_full v, out Input o) {
		UNITY_INITIALIZE_OUTPUT(Input, o);
		o.localPos = v.vertex.xyz;
	}

	void surf(Input IN, inout SurfaceOutputStandard o)
	{
		//Calculate WorldSpace Position of fragment
		float4 worldP = mul(unity_ObjectToWorld, float4(IN.localPos.xyz, 1.0f));

		//Translate point into light space
		float4 pos_LightSpace = mul(_LightMatrix, worldP);

		//Translate point into lightProjectionSpace
		float4 lightTexCoords = mul(_LightTexMatrix, worldP);

		//transform coordinates into texture coordinates
		lightTexCoords.xy /= lightTexCoords.w;
		lightTexCoords.x = 0.5 * lightTexCoords.x + 0.5f;
		lightTexCoords.y = 0.5 * lightTexCoords.y + 0.5f;

		//get the depth from the DepthMap and set it into the range 0-1
		float dist_IN = Linear01Depth(DecodeFloatRGBA(tex2D(_DepthMap, lightTexCoords.xy)));

		//calculate the distance from the Light to the current point
		float dist_OUT = length(pos_LightSpace) / _CameraFarPlane;

		//calculate the distance that the light traveled through the object
		float dist_THROUGH = dist_OUT - dist_IN;

		//Albedo is sampled from the ScatterMap and multiplied with the Color of the light
		o.Albedo = tex2D(_ScatterMap, float2(dist_THROUGH, 0.0f)).rgb * _LightColor;

		// Metallic and smoothness come from slider variables
		o.Metallic = _Metallic;
		o.Smoothness = _Glossiness;
		//Substract the value of the AlphaFallofMap (sampled with the distance traveled through the medium) from the Alpha set in the Material
		o.Alpha = _Alpha - tex2D(_AlphaFalloffMap, float2(dist_THROUGH, 0.0f)).r;
	}
	ENDCG
	}
		FallBack "Diffuse"
}


```

## Source file: Assets/Scripts/SSS_Object.cs

{
  "path": "Assets/Scripts/SSS_Object.cs",
  "blob_sha": "f3252f575bce18072362aa020570a70404af378b",
  "sha256": "f79e5ccc8fc0587dbb77897bce0f5f486c2bce14a601da263cb852655f5dc39e",
  "lines": 83,
  "bytes": 2253,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 6
  },
  "url": "https://github.com/Sushiy/AGT_SSS/blob/72351588fd45c1753f437809d084ad5c11f7d40b/Assets/Scripts/SSS_Object.cs",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "private void Awake()"
    },
    {
      "line": 28,
      "text": "void Start ()"
    },
    {
      "line": 34,
      "text": "public void Toggle(Material[] _arrmatSSS)"
    },
    {
      "line": 63,
      "text": "void ToggleMeshes()"
    },
    {
      "line": 76,
      "text": "void SetMeshMaterial(Material _mat)"
    }
  ]
}

```text

﻿using System.Collections;
using System.Collections.Generic;
using UnityEngine;

//An Object that should receive all the different SSSMaterials
public class SSS_Object : MonoBehaviour
{
    public bool m_bEnabled = false;

    //All Meshrenderers of this Object
    [SerializeField]
    Transform m_transMeshParent;
    [SerializeField]
    Transform m_transDepthMeshParent;
    MeshRenderer[] m_arrmeshrendererRenderMeshes;
    MeshRenderer[] m_arrmeshrendererDepthMeshes;    //All DepthMesh Renderers

    int m_iMaterialIndex = 0;

    private void Awake()
    {
        //Grab all MeshRenderers
        m_arrmeshrendererRenderMeshes = m_transMeshParent.GetComponentsInChildren<MeshRenderer>();
        m_arrmeshrendererDepthMeshes = m_transDepthMeshParent.GetComponentsInChildren<MeshRenderer>();
    }

    // Use this for initialization
    void Start ()
    {
        ToggleMeshes();
	}

    //Go through different States of this Object: Disabled, Material_0, Material_1,...Material_n 
    public void Toggle(Material[] _arrmatSSS)
    {
        if(!m_bEnabled)
        {
            m_bEnabled = true;
            ToggleMeshes();
            m_iMaterialIndex = 0;
            SetMeshMaterial(_arrmatSSS[m_iMaterialIndex]);
        }

        else
        {
            int iLen = _arrmatSSS.Length;
            if(m_iMaterialIndex + 1 < iLen)
            {
                m_iMaterialIndex++;
                SetMeshMaterial(_arrmatSSS[m_iMaterialIndex]);
            }
            else
            {
                m_iMaterialIndex = 0;
                m_bEnabled = false;
                ToggleMeshes();
            }
        }

    }

    //Enable/disable all MeshRenderers (and DepthMesh Renderers)
    void ToggleMeshes()
    {
        foreach (MeshRenderer mesh in m_arrmeshrendererRenderMeshes)
        {
            mesh.enabled = m_bEnabled;
        }
        foreach (MeshRenderer mesh in m_arrmeshrendererDepthMeshes)
        {
            mesh.enabled = m_bEnabled;
        }
    }

    //Set the Material for all MeshRenderers (Not DepthMeshRenderers)
    void SetMeshMaterial(Material _mat)
    {
        foreach (MeshRenderer mesh in m_arrmeshrendererRenderMeshes)
        {
            mesh.material = _mat;
        }
    }
}


```

# Facial rendering source scan: SakhawatAkib/-character-skin-transition-shader-unity

Repository: https://github.com/SakhawatAkib/-character-skin-transition-shader-unity

Pinned source revision: d041e952df9b0b1ad72da86ba6d5c9c3433208e6

Discovery topic: skin shader

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/Materials/SamusSkinTransition.mat", "Assets/Materials/SamusSkinTransition.mat.meta", "Assets/Shaders/SkinTransition.shadergraph", "Assets/Shaders/SkinTransition.shadergraph.meta"], "shader": ["Assets/Shaders.meta", "Assets/Shaders/Fresnel.shader", "Assets/Shaders/Fresnel.shader.meta", "Assets/Shaders/SkinTransition.shadergraph", "Assets/Shaders/SkinTransition.shadergraph.meta", "Assets/Shaders/TransitionTextures.hlsl", "Assets/Shaders/TransitionTextures.hlsl.meta", "Assets/Shaders/TransitionTextures.shadersubgraph", "Assets/Shaders/TransitionTextures.shadersubgraph.meta", "ProjectSettings/ShaderGraphSettings.asset"]}

## Source file: Assets/Shaders/Fresnel.shader

{
  "path": "Assets/Shaders/Fresnel.shader",
  "blob_sha": "9e274e6ef6df810079d13865361f8670b386b751",
  "sha256": "41d526b5902f55f5e4a1f5b16c3299e35646edf12f73281135f0e059522261f4",
  "lines": 64,
  "bytes": 1720,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/SakhawatAkib/-character-skin-transition-shader-unity/blob/d041e952df9b0b1ad72da86ba6d5c9c3433208e6/Assets/Shaders/Fresnel.shader",
  "function_signature_candidates": []
}

```text

Shader "Unlit/Fresnel"
{
    Properties
    {
        _FresnelPower ("Fresnel Power", Float) = 1
        [HDR] _FresnelColor ("Fresnel Color", Color) = (1,1,1,1)
    }
    SubShader
    {
        Tags { "RenderType"="Transparent" "Queue"="Transparent" }
        ZWrite Off
        Blend SrcAlpha OneMinusSrcAlpha
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
                float4 normal : NORMAL;
            };

            struct v2f
            {
                float4 vertex : SV_POSITION;
                float4 viewDir : TEXCOORD0;
                float3 worldNormal : NORMAL;
            };

            float _FresnelPower;
            fixed4 _FresnelColor;

            v2f vert (appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);

                float4 worldPos = mul(unity_ObjectToWorld, v.vertex);
                o.viewDir = float4(_WorldSpaceCameraPos - worldPos.xyz, 0);

                o.worldNormal = UnityObjectToWorldNormal(v.normal);
                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                // fresnelDot is zero when normal is 90 deg angle from view dir
                float fresnelDot = dot(i.worldNormal, normalize(i.viewDir));

                fresnelDot = saturate(fresnelDot); // clamp to 0,1
                float fresnelPow = pow(1.0f - fresnelDot, _FresnelPower );

                return fresnelPow * _FresnelColor;
            }
            ENDCG
        }
    }
}


```

## Source file: Assets/Shaders/TransitionTextures.hlsl

{
  "path": "Assets/Shaders/TransitionTextures.hlsl",
  "blob_sha": "8fec587f5fcee6811a5f4d1fd1b5c1725f9ae148",
  "sha256": "497c214f1189087908e528f28884a111ce6d328b836a2ec27374a4628c73537c",
  "lines": 23,
  "bytes": 673,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/SakhawatAkib/-character-skin-transition-shader-unity/blob/d041e952df9b0b1ad72da86ba6d5c9c3433208e6/Assets/Shaders/TransitionTextures.hlsl",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "void TransitionTextures_half("
    }
  ]
}

```text

void TransitionTextures_half(
    in half4 primaryTextureColor,
    in half4 secondaryTextureColor,
    in float3 fragmentWorldPosition,
    in float thresholdWorldPositionY,
    in float thresholdWidth,
    in half4 thresholdColor,
    out half4 outputColor
)
{
    if(fragmentWorldPosition.y > thresholdWorldPositionY + thresholdWidth)
    {
        outputColor = secondaryTextureColor;
    } else if (
        thresholdWorldPositionY - thresholdWidth <= fragmentWorldPosition.y &&
        thresholdWorldPositionY + thresholdWidth >= fragmentWorldPosition.y)
    {
        outputColor = thresholdColor;
    } else
    {
        outputColor = primaryTextureColor;
    }
}


```

## Source file: Assets/Scripts/ThresholdPositionSetter.cs

{
  "path": "Assets/Scripts/ThresholdPositionSetter.cs",
  "blob_sha": "23fc115b21e0215e066e40d45d7a483a92768eb3",
  "sha256": "4ab136d9f11f740192aa9ffaf0c48e93c4a2d5a95fc61bcbf2c1a7e21eff9cda",
  "lines": 22,
  "bytes": 563,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 2
  },
  "url": "https://github.com/SakhawatAkib/-character-skin-transition-shader-unity/blob/d041e952df9b0b1ad72da86ba6d5c9c3433208e6/Assets/Scripts/ThresholdPositionSetter.cs",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "void Start()"
    },
    {
      "line": 18,
      "text": "void Update()"
    }
  ]
}

```text

using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[RequireComponent(typeof(SkinnedMeshRenderer))]
public class ThresholdPositionSetter : MonoBehaviour
{
    public Transform ThresholdObject;
    private Material _material;

    // Start is called before the first frame update
    void Start()
    {
        _material = GetComponent<SkinnedMeshRenderer>().material;
    }

    // Update is called once per frame
    void Update()
    {
        _material.SetFloat("_ThresholdPositionY", ThresholdObject.transform.position.y);
    }
}


```

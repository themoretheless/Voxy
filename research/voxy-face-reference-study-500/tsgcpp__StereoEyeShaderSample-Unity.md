# Facial rendering source scan: tsgcpp/StereoEyeShaderSample-Unity

Repository: https://github.com/tsgcpp/StereoEyeShaderSample-Unity

Pinned source revision: 780947b9b354c2e8f79fab8d55632ea4378db36c

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["Assets/Materials/StereoEyeMat.mat", "Assets/Materials/StereoEyeMat.mat.meta", "Assets/Resources/LeftEyeTexture.png", "Assets/Resources/LeftEyeTexture.png.meta", "Assets/Resources/RightEyeTexture.png", "Assets/Resources/RightEyeTexture.png.meta", "Assets/Scenes/StereoEyeShaderSample.meta", "Assets/Scenes/StereoEyeShaderSample.unity", "Assets/Scenes/StereoEyeShaderSample.unity.meta", "Assets/Scenes/StereoEyeShaderSample/LightingData.asset", "Assets/Scenes/StereoEyeShaderSample/LightingData.asset.meta", "Assets/Scenes/StereoEyeShaderSample/ReflectionProbe-0.exr", "Assets/Scenes/StereoEyeShaderSample/ReflectionProbe-0.exr.meta", "Assets/Shaders/StereoEyeTexture.shader", "Assets/Shaders/StereoEyeTexture.shader.meta"], "shader": ["Assets/Scenes/StereoEyeShaderSample.meta", "Assets/Scenes/StereoEyeShaderSample.unity", "Assets/Scenes/StereoEyeShaderSample.unity.meta", "Assets/Scenes/StereoEyeShaderSample/LightingData.asset", "Assets/Scenes/StereoEyeShaderSample/LightingData.asset.meta", "Assets/Scenes/StereoEyeShaderSample/ReflectionProbe-0.exr", "Assets/Scenes/StereoEyeShaderSample/ReflectionProbe-0.exr.meta", "Assets/Shaders.meta", "Assets/Shaders/StereoEyeTexture.shader", "Assets/Shaders/StereoEyeTexture.shader.meta"], "rig": ["Assets/Resources/RightEyeTexture.png", "Assets/Resources/RightEyeTexture.png.meta"]}

## Source file: Assets/Shaders/StereoEyeTexture.shader

{
  "path": "Assets/Shaders/StereoEyeTexture.shader",
  "blob_sha": "408042cd7a2400a135287270599df17691d8a067",
  "sha256": "84cedf27494911e24a28dafaca6a29bf4f07486458d871451161f8a00d3c3b30",
  "lines": 88,
  "bytes": 2420,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 11,
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/tsgcpp/StereoEyeShaderSample-Unity/blob/780947b9b354c2e8f79fab8d55632ea4378db36c/Assets/Shaders/StereoEyeTexture.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "StereoEyeTexture"
{
    SubShader
    {
        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag

			// texture arrayが有効なプラットフォームのみで子のシェーダーが使用可能であることの宣言
            #pragma require 2darray
            
            #include "UnityCG.cginc"

			/*
			unity_StereoEyeIndexについて

			unity_StereoEyeIndexはXR環境であれば基本的に`#include`経由で変数宣言される。
			実行中のシェーディング処理が左目もしくは右目のどっち向けかの判定に使用される。
			値が 0: 左目、1: 右目。
			*/

			struct appdata
			{
				float4 vertex : POSITION;
				float2 uv : TEXCOORD0;

				// instanceID変数の宣言
				UNITY_VERTEX_INPUT_INSTANCE_ID
			};

			struct v2f
			{

				float4 vertex : SV_POSITION;
				float2 uv : TEXCOORD0;

				// instanceIDをfragに送る場合は必要(今回は不要)
				// UNITY_VERTEX_INPUT_INSTANCE_ID

				// fragにunity_StereoEyeIndexなどの情報を流すための変数を追加
				UNITY_VERTEX_OUTPUT_STEREO
			};

			v2f vert (appdata v)
			{
				v2f output;

				// instanceIDおよびunity_StereoEyeIndexの取得
				UNITY_SETUP_INSTANCE_ID(v);

				// instanceIDをfragに送る場合は必要(今回は不要)
				// UNITY_TRANSFER_INSTANCE_ID(v, o);

				// outputの初期化
				UNITY_INITIALIZE_OUTPUT(v2f, output);

				// unity_StereoEyeIndexをoutput内の変数に付与
				UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(output);
				
				// vertexとuvは通常通り
				output.vertex = UnityObjectToClipPos(v.vertex);
				output.uv = v.uv;

				return output;
			}

			// _MainTexをsampler2DArrayとして宣言
			UNITY_DECLARE_TEX2DARRAY(_MainTex);

			fixed4 frag (v2f input) : SV_Target
			{
				// vertから流れてきたinputからunity_StereoEyeIndexを更新
				UNITY_SETUP_STEREO_EYE_INDEX_POST_VERTEX(input);

				// inputを使用しなくても、以下の関数でunity_StereoEyeIndexを更新可能(今回は不使用)
				// UNITY_SETUP_INSTANCE_ID(i);
				
				// unit_StereoEyeIndex番目のテクスチャをサンプリング
				return UNITY_SAMPLE_TEX2DARRAY(_MainTex,
					float3((input.uv).xy, (float)unity_StereoEyeIndex));
			}

            ENDCG
        }
    }
}

```

## Source file: Assets/Scripts/StereoTextureGenerator.cs

{
  "path": "Assets/Scripts/StereoTextureGenerator.cs",
  "blob_sha": "93d704d76be715725670ae1afde228670357f51c",
  "sha256": "169eae469b7b70aa1cffcc73bb54999ca34b1f63cce8aed326dfab27daf865f3",
  "lines": 143,
  "bytes": 5233,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 29,
    "rig": 13
  },
  "url": "https://github.com/tsgcpp/StereoEyeShaderSample-Unity/blob/780947b9b354c2e8f79fab8d55632ea4378db36c/Assets/Scripts/StereoTextureGenerator.cs",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "private void Awake()"
    },
    {
      "line": 130,
      "text": "private void OnDestroy()"
    }
  ]
}

```text

﻿using UnityEngine;
using UnityEngine.Rendering;

[RequireComponent(typeof(Renderer))]
public class StereoTextureGenerator : MonoBehaviour
{
    [SerializeField] private Texture _leftEyeTexture;
    [SerializeField] private Texture _rightEyeTexture;

    [Tooltip("Generate RenderTexture with \"vrUsage\" as \"VRTextureUsage.TwoEyes\".\nFYI: ")]
    [SerializeField] private CopyType _copyType = CopyType.VRUsageAsNone;

    private Renderer _renderer;

    private RenderTexture _stereoTexture;

    private void Awake()
    {
        _renderer = GetComponent<Renderer>();
        _stereoTexture = GenerateStereoTexture(_leftEyeTexture, _rightEyeTexture);

        // _MainTexはTex2DArrayとして受け取ることを前提
        _renderer.sharedMaterial.mainTexture = _stereoTexture;
    }

    private RenderTexture GenerateStereoTexture(
        Texture leftEyeTexture, Texture rightEyeTexture)
    {
        switch (_copyType)
        {
            case CopyType.VRUsageAsTwoEyes:
                return GenerateStereoTextureAsTwoEyes(leftEyeTexture, rightEyeTexture);
            case CopyType.VRUsageAsTwoEyesAndCopyDirect:
                return GenerateStereoTextureAsTwoEyesAndCopyDirect(leftEyeTexture, rightEyeTexture);
            default:
                return GenerateStereoTextureAsNone(leftEyeTexture, rightEyeTexture);
        }
    }

    /// <summary>
    /// Tex2DArray型のRenderTextureを生成し、
    /// 両目用テクスチャをそれぞれのテクスチャにコピーして返す
    /// </summary>
    /// <param name="leftEyeTexture">左目用テクスチャ</param>
    /// <param name="rightEyeTexture">右目用テクスチャ</param>
    /// <param name="targetRT">コピー先のRenderTexture(Tex2DArrayかつvolumeであることが前提)</param>
    private static RenderTexture GenerateStereoTextureAsNone(
        Texture leftEyeTexture, Texture rightEyeTexture)
    {
        var desc = new RenderTextureDescriptor(1024, 1024);

        // RenderTextureをTex2DArrayとして生成
        desc.dimension = TextureDimension.Tex2DArray;

        // Textureは2つ(0番目は左目用、1番目は右目用)
        desc.volumeDepth = 2;

        desc.vrUsage = VRTextureUsage.None;

        // RenderTextureDescriptorからテクスチャを生成
        var stereoTexture = RenderTexture.GetTemporary(desc);

        // 両目用テクスチャをTex2DArray型のRenderTextureにコピー
        Graphics.Blit(
            source: leftEyeTexture, dest: stereoTexture,
            sourceDepthSlice: 0, destDepthSlice: 0);
        Graphics.Blit(
            source: rightEyeTexture, dest: stereoTexture,
            sourceDepthSlice: 0, destDepthSlice: 1);

        return stereoTexture;
    }

    private static RenderTexture GenerateStereoTextureAsTwoEyes(
        Texture leftEyeTexture, Texture rightEyeTexture)
    {
        var desc = new RenderTextureDescriptor(1024, 1024);

        desc.dimension = TextureDimension.Tex2DArray;
        desc.volumeDepth = 2;
        desc.vrUsage = VRTextureUsage.TwoEyes;

        var stereoTexture = RenderTexture.GetTemporary(desc);

        // バグ対策のためのコピー用
        // 詳細: https://tsgcpp.hateblo.jp/entry/2020/06/27/200431
        desc.vrUsage = VRTextureUsage.None;
        var copyTexture = RenderTexture.GetTemporary(desc);

        // 両目用テクスチャをTex2DArray型のRenderTextureにコピー
        Graphics.Blit(
            source: leftEyeTexture, dest: copyTexture,
            sourceDepthSlice: 0, destDepthSlice: 0);
        Graphics.Blit(
            source: rightEyeTexture, dest: copyTexture,
            sourceDepthSlice: 0, destDepthSlice: 1);

        Graphics.Blit(source: copyTexture, dest: stereoTexture);

        RenderTexture.ReleaseTemporary(copyTexture);

        return stereoTexture;
    }

    /// <summary>
    /// This function will cause a bug that "leftEyeTexture" (destDepthSlice: 0) is not copied correctly.
    /// </summary>
    private static RenderTexture GenerateStereoTextureAsTwoEyesAndCopyDirect(
        Texture leftEyeTexture, Texture rightEyeTexture)
    {
        var desc = new RenderTextureDescriptor(1024, 1024);

        desc.dimension = TextureDimension.Tex2DArray;
        desc.volumeDepth = 2;
        desc.vrUsage = VRTextureUsage.TwoEyes;

        var stereoTexture = RenderTexture.GetTemporary(desc);

        // Copy textures to stereoTexture which vrUsage is TwoEyes
        Graphics.Blit(
            source: leftEyeTexture, dest: stereoTexture,
            sourceDepthSlice: 0, destDepthSlice: 0);
        Graphics.Blit(
            source: rightEyeTexture, dest: stereoTexture,
            sourceDepthSlice: 0, destDepthSlice: 1);

        return stereoTexture;
    }

    private void OnDestroy()
    {
        _renderer.sharedMaterial.mainTexture = null;
        RenderTexture.ReleaseTemporary(_stereoTexture);
        _stereoTexture = null;
    }

    public enum CopyType
    {
        VRUsageAsNone = 0,
        VRUsageAsTwoEyes = 1,
        VRUsageAsTwoEyesAndCopyDirect = 2,
    }
}


```

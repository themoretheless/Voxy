# Facial rendering source scan: hai-vr/blendshape-viewer--legacy

Repository: https://github.com/hai-vr/blendshape-viewer--legacy

Pinned source revision: 396fd2ba3d37d85c45dba0e420a7c57e681e37b3

Discovery topic: blendshape

Description: Legacy repository. This used to contain 3 projects, but they have been split into separate projects.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["Assets/Hai/BlendshapeViewer.meta", "Assets/Hai/BlendshapeViewer/Scripts.meta", "Assets/Hai/BlendshapeViewer/Scripts/Editor.meta", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerDiffCompute.cs", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerDiffCompute.cs.meta", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerEditorWindow.cs", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerEditorWindow.cs.meta", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerGenerator.cs", "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerGenerator.cs.meta", "Assets/Hai/BlendshapeViewer/Scripts/Editor/DiffCompute.compute", "Assets/Hai/BlendshapeViewer/Scripts/Editor/DiffCompute.compute.meta", "Assets/Hai/BlendshapeViewer/Shaders.meta", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader.meta", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader.meta"], "shader": ["Assets/Hai/BlendshapeViewer/Shaders.meta", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader.meta", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader", "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader.meta", "Assets/Hai/VisualExpressionsEditor/Shaders.meta", "Assets/Hai/VisualExpressionsEditor/Shaders/HaiVisualExpressionsEditor.shader", "Assets/Hai/VisualExpressionsEditor/Shaders/HaiVisualExpressionsEditor.shader.meta", "Assets/Hai/VisualExpressionsEditor/Shaders/HaiVisualExpressionsEditorRectOnly.shader", "Assets/Hai/VisualExpressionsEditor/Shaders/HaiVisualExpressionsEditorRectOnly.shader.meta"], "lip": ["Assets/Hai/VisualExpressionsEditor/Scripts/Editor/VisualExpressionsEditorGeneratorClip.cs", "Assets/Hai/VisualExpressionsEditor/Scripts/Editor/VisualExpressionsEditorGeneratorClip.cs.meta"]}

## Source file: Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader

{
  "path": "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader",
  "blob_sha": "8dfe6f6c9704207663b491ad01c57e4bbda81c8f",
  "sha256": "67930240626a3d65a2e4b65efb13ae182a249cfcad4958612a325a60711e4b52",
  "lines": 99,
  "bytes": 3307,
  "selection_score": 30,
  "keyword_line_counts": {
    "blendshape": 1,
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/hai-vr/blendshape-viewer--legacy/blob/396fd2ba3d37d85c45dba0e420a7c57e681e37b3/Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewer.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Hai/BlendshapeViewer"
{
    Properties
    {
        _MainTex ("Morphed Texture", 2D) = "white" {}
        _NeutralTex ("Neutral Texture", 2D) = "white" {}
        _Hotspots ("Hotspots", Range(0, 1)) = 0
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

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float2 uv : TEXCOORD0;
                UNITY_FOG_COORDS(1)
                float4 vertex : SV_POSITION;
                float4 diff : TEXCOORD1;
            };

            sampler2D _MainTex;
            float4 _MainTex_ST;
            float4 _MainTex_TexelSize;

            sampler2D _NeutralTex;
            float4 _NeutralTex_ST;

            float _Hotspots;

            v2f vert (appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = TRANSFORM_TEX(v.uv, _MainTex);

                float width = _MainTex_TexelSize.z;
                float height = _MainTex_TexelSize.w;
                float4 difference = float4(width, height, 0, 0);
                // TODO: There's gotta be a more clever way to do this that uses parallelism
                for (int y = 0; y < height; y++) {
                    for (int x = 0; x < width; x++) {
                        float4 sampleLocation = float4(x / width, y / height, 0, 0);
                        fixed3 neutral = tex2Dlod(_NeutralTex, sampleLocation).xyz;
                        fixed3 morphed = tex2Dlod(_MainTex, sampleLocation).xyz;
                        fixed3 v = neutral - morphed;
                        if (dot(v, v) > 0.01) {
                            difference = float4(
                                min(x, difference.x),
                                min(y, difference.y),
                                max(x, difference.z),
                                max(y, difference.w)
                            );
                        }
                    }
                }
                o.diff = difference;

                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                float width = _MainTex_TexelSize.z;
                float height = _MainTex_TexelSize.w;

                float4 difference = i.diff;
                difference = difference + float4(-1, -1, 1, 1) * 2; // Margin
                
                fixed4 col = tex2D(_MainTex, i.uv);
                if (i.uv.x < difference.x / width || i.uv.x > difference.z / width
                    || i.uv.y < difference.y / height || i.uv.y > difference.w / height) {
                        col.xyz = col.xyz * 0.2;
                }
                if (_Hotspots > 0.01) {
                    fixed3 neutral = tex2D(_NeutralTex, i.uv).xyz;
                    fixed3 morphed = tex2D(_MainTex, i.uv).xyz;
                    col = lerp(col, length(neutral - morphed) * float4(1, 0, 0, 1), _Hotspots);
                }
                return col;
            }
            ENDCG
        }
    }
}


```

## Source file: Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader

{
  "path": "Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader",
  "blob_sha": "7a9f90c7f69c8b678e1796450deaceafceb566fb",
  "sha256": "00f35157c1e611e15aac2ac5ae2f7375eeaef6056533e39cc95be6e0df02b8bf",
  "lines": 78,
  "bytes": 2210,
  "selection_score": 30,
  "keyword_line_counts": {
    "blendshape": 1,
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/hai-vr/blendshape-viewer--legacy/blob/396fd2ba3d37d85c45dba0e420a7c57e681e37b3/Assets/Hai/BlendshapeViewer/Shaders/HaiBlendshapeViewerRectOnly.shader",
  "function_signature_candidates": []
}

```text

﻿Shader "Hai/BlendshapeViewerRectOnly"
{
    Properties
    {
        _MainTex ("Morphed Texture", 2D) = "white" {}
        _NeutralTex ("Neutral Texture", 2D) = "white" {}
        _Hotspots ("Hotspots", Range(0, 1)) = 0
        _Rect ("Rect", Vector) = (0, 0, 0, 0)
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

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float2 uv : TEXCOORD0;
                UNITY_FOG_COORDS(1)
                float4 vertex : SV_POSITION;
            };

            sampler2D _MainTex;
            float4 _MainTex_ST;
            float4 _MainTex_TexelSize;

            sampler2D _NeutralTex;
            float4 _NeutralTex_ST;

            float _Hotspots;
            float4 _Rect;

            v2f vert (appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = TRANSFORM_TEX(v.uv, _MainTex);

                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                float width = _MainTex_TexelSize.z;
                float height = _MainTex_TexelSize.w;

                float4 difference = _Rect;
                difference = difference + float4(-1, -1, 1, 1) * 2; // Margin

                fixed4 col = tex2D(_MainTex, i.uv);
                if (i.uv.x < difference.x / width || i.uv.x > difference.z / width
                    || i.uv.y < difference.y / height || i.uv.y > difference.w / height) {
                        col.xyz = col.xyz * 0.2;
                }
                if (_Hotspots > 0.01) {
                    fixed3 neutral = tex2D(_NeutralTex, i.uv).xyz;
                    fixed3 morphed = tex2D(_MainTex, i.uv).xyz;
                    col = lerp(col, length(neutral - morphed) * float4(1, 0, 0, 1), _Hotspots);
                }
                return col;
            }
            ENDCG
        }
    }
}


```

## Source file: Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerGenerator.cs

{
  "path": "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerGenerator.cs",
  "blob_sha": "da684f9d996943d23991063c877a14aae2a5d137",
  "sha256": "18033a1a55c7a4c2c525649f84b03c85bbb6693798120cf2bebdd6279d610067",
  "lines": 111,
  "bytes": 4287,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 4,
    "blendshape": 5,
    "shader": 7,
    "rig": 4,
    "lip": 4
  },
  "url": "https://github.com/hai-vr/blendshape-viewer--legacy/blob/396fd2ba3d37d85c45dba0e420a7c57e681e37b3/Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerGenerator.cs",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "public void Begin(SkinnedMeshRenderer skinnedMesh, float overlay, bool useComputeShader)"
    },
    {
      "line": 41,
      "text": "public void Terminate()"
    },
    {
      "line": 51,
      "text": "public void Render(AnimationClip clip, Texture2D element)"
    },
    {
      "line": 73,
      "text": "private static void RenderCamera(RenderTexture renderTexture, Camera camera)"
    },
    {
      "line": 90,
      "text": "private static void RenderTextureTo(RenderTexture renderTexture, Texture2D texture2D)"
    },
    {
      "line": 98,
      "text": "public void Diff(Texture2D source, Texture2D neutralTexture, Texture2D newTexture)"
    }
  ]
}

```text

﻿using UnityEditor;
using UnityEngine;

namespace Hai.BlendshapeViewer.Scripts.Editor
{
    public class BlendshapeViewerGenerator
    {
        private Material _material;
        private SkinnedMeshRenderer _skinnedMesh;
        private bool _useComputeShader;
        private Camera _camera;
        private float _overlay;
        private BlendshapeViewerDiffCompute _diffCompute;

        public void Begin(SkinnedMeshRenderer skinnedMesh, float overlay, bool useComputeShader)
        {
            _skinnedMesh = skinnedMesh;
            _overlay = overlay;
            _useComputeShader = SystemInfo.supportsComputeShaders && useComputeShader;

            _material = new Material(_useComputeShader ? Shader.Find("Hai/BlendshapeViewerRectOnly") : Shader.Find("Hai/BlendshapeViewer"));
            _material.SetFloat("_Hotspots", _overlay);
            _camera = new GameObject().AddComponent<Camera>();

            var sceneCamera = SceneView.lastActiveSceneView.camera;
            _camera.transform.position = sceneCamera.transform.position;
            _camera.transform.rotation = sceneCamera.transform.rotation;
            var whRatio = (1f * sceneCamera.pixelWidth / sceneCamera.pixelHeight);
            _camera.fieldOfView = whRatio < 1 ? sceneCamera.fieldOfView * whRatio : sceneCamera.fieldOfView;
            _camera.orthographic = sceneCamera.orthographic;
            _camera.nearClipPlane = sceneCamera.nearClipPlane;
            _camera.farClipPlane = sceneCamera.farClipPlane;
            _camera.orthographicSize = sceneCamera.orthographicSize;

            if (_useComputeShader)
            {
                _diffCompute = new BlendshapeViewerDiffCompute();
            }
        }

        public void Terminate()
        {
            Object.DestroyImmediate(_material);
            Object.DestroyImmediate(_camera.gameObject);
            if (_useComputeShader)
            {
                _diffCompute.Terminate();
            }
        }

        public void Render(AnimationClip clip, Texture2D element)
        {
            try
            {
                AnimationMode.StartAnimationMode();
                AnimationMode.BeginSampling();
                AnimationMode.SampleAnimationClip(_skinnedMesh.gameObject, clip, 1 / 60f);
                AnimationMode.EndSampling();

                var renderTexture = RenderTexture.GetTemporary(element.width, element.height, 24);
                renderTexture.wrapMode = TextureWrapMode.Clamp;

                RenderCamera(renderTexture, _camera);
                RenderTextureTo(renderTexture, element);
                RenderTexture.ReleaseTemporary(renderTexture);
            }
            finally
            {
                AnimationMode.StopAnimationMode();
            }
        }

        private static void RenderCamera(RenderTexture renderTexture, Camera camera)
        {
            var originalRenderTexture = camera.targetTexture;
            var originalAspect = camera.aspect;
            try
            {
                camera.targetTexture = renderTexture;
                camera.aspect = (float) renderTexture.width / renderTexture.height;
                camera.Render();
            }
            finally
            {
                camera.targetTexture = originalRenderTexture;
                camera.aspect = originalAspect;
            }
        }

        private static void RenderTextureTo(RenderTexture renderTexture, Texture2D texture2D)
        {
            RenderTexture.active = renderTexture;
            texture2D.ReadPixels(new Rect(0, 0, renderTexture.width, renderTexture.height), 0, 0);
            texture2D.Apply();
            RenderTexture.active = null;
        }

        public void Diff(Texture2D source, Texture2D neutralTexture, Texture2D newTexture)
        {
            var diff = RenderTexture.GetTemporary(newTexture.width, newTexture.height, 24);
            _material.SetTexture("_NeutralTex", neutralTexture);
            if (_useComputeShader)
            {
                _material.SetVector("_Rect", _diffCompute.Compute(source, neutralTexture));
            }
            Graphics.Blit(source, diff, _material);
            RenderTextureTo(diff, newTexture);
            RenderTexture.ReleaseTemporary(diff);
        }
    }
}

```

## Source file: Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerDiffCompute.cs

{
  "path": "Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerDiffCompute.cs",
  "blob_sha": "d795ffc87740884854f62589420aa76d3b288050",
  "sha256": "af816235ddb3f40a43f19108afefa4baaafd875bdcfc2d1e557d4562fa743984",
  "lines": 66,
  "bytes": 3027,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 4,
    "shader": 14,
    "rig": 4
  },
  "url": "https://github.com/hai-vr/blendshape-viewer--legacy/blob/396fd2ba3d37d85c45dba0e420a7c57e681e37b3/Assets/Hai/BlendshapeViewer/Scripts/Editor/BlendshapeViewerDiffCompute.cs",
  "function_signature_candidates": [
    {
      "line": 46,
      "text": "public void Terminate()"
    }
  ]
}

```text

﻿/*
Copyright 2022 Pema Malling

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

using System.Linq;
using UnityEditor;
using UnityEngine;

namespace Hai.BlendshapeViewer.Scripts.Editor
{
    public class BlendshapeViewerDiffCompute
    {
        private readonly ComputeShader _computeShader;
        private readonly ComputeBuffer _buf;
        private readonly int _kernel;

        public BlendshapeViewerDiffCompute()
        {
            _computeShader = FindComputeShader();
            _kernel = _computeShader.FindKernel("DiffCompute");
            _buf = new ComputeBuffer(4, sizeof(int));
            _computeShader.SetBuffer(_kernel, "ResultBuffer", _buf);
        }

        public Vector4 Compute(Texture2D textureA, Texture2D textureB)
        {
            var results = new int[4];
            _buf.SetData(results);
            var computeShader = _computeShader;

            computeShader.SetTexture(_kernel, "InputA", textureA);
            computeShader.SetTexture(_kernel, "InputB", textureB);

            computeShader.Dispatch(_kernel, textureA.width / 8, textureB.height / 8, 1);

            _buf.GetData(results);
            return new Vector4(results[0], results[1], results[2], results[3]);
        }

        public void Terminate()
        {
            _buf.Release();
        }

        private static ComputeShader FindComputeShader()
        {
            var assetPathOrEmpty = AssetDatabase.GUIDToAssetPath("569e5a4e6b0efc74b93a42db6d069724");
            var defaultPath = "Assets/Hai/BlendshapeViewer/Scripts/Editor/DiffCompute.compute";
            var computeShader = AssetDatabase.LoadAssetAtPath<ComputeShader>(assetPathOrEmpty == "" ? defaultPath : assetPathOrEmpty)
                                ?? FindAmongAllComputeShaders();
            return computeShader;
        }

        private static ComputeShader FindAmongAllComputeShaders()
        {
            return Resources.FindObjectsOfTypeAll<ComputeShader>()
                .First(o => o.name.Contains("DiffCompute"));
        }
    }
}


```

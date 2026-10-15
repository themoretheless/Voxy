# Facial rendering source scan: unity3d-jp/BlendShapeBuilder

Repository: https://github.com/unity3d-jp/BlendShapeBuilder

Pinned source revision: bf04aceb946c20ec18fa9aa437b9b35102d92867

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt", "LICENSE.txt.meta"]

Topic paths: {"blendshape": [".BlendShapeBuilder/Assets/BlendShapeBuilderPackaging.cs", ".BlendShapeBuilder/Assets/BlendShapeBuilderPackaging.cs.meta", ".BlendShapeBuilder/Assets/UTJ.meta", ".BlendShapeBuilder/Assets/UTJ/BlendShapeBuilder", ".BlendShapeBuilder/Assets/UTJ/BlendShapeBuilder.meta", ".BlendShapeBuilder/Assets/UTJ/VertexTweaker", ".BlendShapeBuilder/Assets/UTJ/VertexTweaker.meta", ".BlendShapeBuilder/Packages/manifest.json", ".BlendShapeBuilder/Plugin/CMakeLists.txt", ".BlendShapeBuilder/Plugin/External/7z/7za.exe", ".BlendShapeBuilder/Plugin/External/make_archive.bat", ".BlendShapeBuilder/Plugin/MeshUtils.vcxproj", ".BlendShapeBuilder/Plugin/MeshUtils.vcxproj.filters", ".BlendShapeBuilder/Plugin/MeshUtils/CMakeLists.txt", ".BlendShapeBuilder/Plugin/MeshUtils/MeshUtils.cpp", ".BlendShapeBuilder/Plugin/MeshUtils/MeshUtils.h", ".BlendShapeBuilder/Plugin/MeshUtils/MeshUtilsCore.ispc", ".BlendShapeBuilder/Plugin/MeshUtils/MeshUtils_impl.h", ".BlendShapeBuilder/Plugin/MeshUtils/ampmath.h", ".BlendShapeBuilder/Plugin/MeshUtils/ampmath_impl.h", ".BlendShapeBuilder/Plugin/MeshUtils/ispcmath.h", ".BlendShapeBuilder/Plugin/MeshUtils/mikktspace.c", ".BlendShapeBuilder/Plugin/MeshUtils/mikktspace.h", ".BlendShapeBuilder/Plugin/MeshUtils/muAllocator.cpp", ".BlendShapeBuilder/Plugin/MeshUtils/muAllocator.h", ".BlendShapeBuilder/Plugin/MeshUtils/muConcurrency.h", ".BlendShapeBuilder/Plugin/MeshUtils/muConfig.h", ".BlendShapeBuilder/Plugin/MeshUtils/muIntrusiveArray.h", ".BlendShapeBuilder/Plugin/MeshUtils/muIterator.h", ".BlendShapeBuilder/Plugin/MeshUtils/muMath.cpp"], "shader": ["Assets/VertexTweaker/Runtime/Shaders.meta", "Assets/VertexTweaker/Runtime/Shaders/BrushShape.shader", "Assets/VertexTweaker/Runtime/Shaders/BrushShape.shader.meta", "Assets/VertexTweaker/Runtime/Shaders/Overlay.shader", "Assets/VertexTweaker/Runtime/Shaders/Overlay.shader.meta", "Assets/VertexTweaker/Runtime/Shaders/Visualizer.shader", "Assets/VertexTweaker/Runtime/Shaders/Visualizer.shader.meta"]}

## Source file: Assets/VertexTweaker/Runtime/Shaders/Overlay.shader

{
  "path": "Assets/VertexTweaker/Runtime/Shaders/Overlay.shader",
  "blob_sha": "16ca8b802092264621eee8d49f1d5acdab6f854f",
  "sha256": "669331cc4d42863645d1edf797b79c31760924fdd7729ba20c8896c52fc905ac",
  "lines": 184,
  "bytes": 4157,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 6
  },
  "url": "https://github.com/unity3d-jp/BlendShapeBuilder/blob/bf04aceb946c20ec18fa9aa437b9b35102d92867/Assets/VertexTweaker/Runtime/Shaders/Overlay.shader",
  "function_signature_candidates": [
    {
      "line": 41,
      "text": "float3 ToBaseTangentSpace(uint vid, float3 n)"
    },
    {
      "line": 99,
      "text": "float4 frag(vs_out v) : SV_Target"
    }
  ]
}

```text

Shader "Hidden/VertexTweaker/Overlay" {

CGINCLUDE
#include "UnityCG.cginc"

StructuredBuffer<float3> _BaseNormals;
StructuredBuffer<float4> _BaseTangents;
StructuredBuffer<float3> _Points;
StructuredBuffer<float3> _Normals;
StructuredBuffer<float4> _Tangents;
StructuredBuffer<float> _Selection;



struct ia_out
{
    float4 vertex : POSITION;
    float4 normal : NORMAL;
    float4 uv : TEXCOORD0;
    float4 color : COLOR;
    uint vertexID : SV_VertexID;
    uint instanceID : SV_InstanceID;
};

struct vs_out
{
    float4 vertex : SV_POSITION;
    float4 color : TEXCOORD0;
};


vs_out vert_local_space_normals_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.color.rgb = v.normal.xyz * 0.5 + 0.5;
    o.color.a = 1.0;
    return o;
}

float3 ToBaseTangentSpace(uint vid, float3 n)
{
    float3 base_normal = _BaseNormals[vid];
    float4 base_tangent = _BaseTangents[vid];
    float3 base_binormal = normalize(cross(base_normal, base_tangent.xyz) * base_tangent.w);
    float3x3 tbn = float3x3(base_tangent.xyz, base_binormal, base_normal);
    return normalize(mul(n, transpose(tbn)));
}

vs_out vert_tangent_space_normals_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.color.rgb = ToBaseTangentSpace(v.vertexID, v.normal.xyz) * 0.5 + 0.5;
    o.color.a = 1.0;
    return o;
}

vs_out vert_tangents_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);

    float4 tangent = _Tangents[v.vertexID];
    o.color.rgb = (tangent.xyz * tangent.w) * 0.5 + 0.5;
    o.color.a = 1.0;
    return o;
}

vs_out vert_binormals_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);

    float4 tangent = _Tangents[v.vertexID];
    float3 binormal = normalize(cross(v.normal.xyz, tangent.xyz * tangent.w));
    o.color.rgb = binormal * 0.5 + 0.5;
    o.color.a = 1.0;
    return o;
}

vs_out vert_uv_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.color = float4(v.uv.xy, 0.0, 1.0);
    return o;
}

vs_out vert_color_overlay(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.color = v.color;
    return o;
}


float4 frag(vs_out v) : SV_Target
{
    return v.color;
}

ENDCG

    SubShader
    {
        Tags{ "RenderType" = "Transparent" "Queue" = "Transparent+99" }
        Blend SrcAlpha OneMinusSrcAlpha
        ZWrite Off

        // pass 0: local space normals overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_local_space_normals_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 1: tangent space normals overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_tangent_space_normals_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 2: tangents overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_tangents_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }
    
        // pass 3: binormals overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_binormals_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 4: uv overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_uv_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 5: vertex color overlay
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_color_overlay
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }
    }
}


```

## Source file: Assets/VertexTweaker/Runtime/Shaders/BrushShape.shader

{
  "path": "Assets/VertexTweaker/Runtime/Shaders/BrushShape.shader",
  "blob_sha": "28ca6345a4141c8d6d2740acc4ac8d01716629ec",
  "sha256": "4410e8a78d20714e01b93c256c540116a05b55d3dbaf707c2670f5315015cab7",
  "lines": 62,
  "bytes": 1357,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2
  },
  "url": "https://github.com/unity3d-jp/BlendShapeBuilder/blob/bf04aceb946c20ec18fa9aa437b9b35102d92867/Assets/VertexTweaker/Runtime/Shaders/BrushShape.shader",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "float4 frag(v2f i) : SV_Target"
    }
  ]
}

```text

﻿Shader "Hidden/VertexTweaker/BrushShape"
{
CGINCLUDE
#include "UnityCG.cginc"


int _NumBrushSamples;
StructuredBuffer<float> _BrushSamples;

struct appdata
{
    float4 vertex : POSITION;
};

struct v2f
{
    float4 vertex : SV_POSITION;
    float4 uv : TEXCOORD0;
};


v2f vert(appdata v)
{
    v2f o;
    o.vertex = float4(v.vertex.xy, 0.0, 1.0);
    o.uv = float4(abs(v.vertex.x * 0.5 + 0.5), 1.0f - (v.vertex.y * 0.5f + 0.5f), 0, 0);
    return o;
}

float4 frag(v2f i) : SV_Target
{
    //StructuredBuffer::GetDimensions() seems not available on non-D3D11..
    int n = _NumBrushSamples;
    
    float u = 1.0 - abs(i.uv.x * 2.0 - 1.0);
    float v = _BrushSamples[(int)(u * (n-1))];

    float dy = 1.0 / (n-1) * 2.0;
    float c = 0.0;
    c += i.uv.y + dy * 2.0 < v ? 0.15 : 0.0; // 
    c += i.uv.y + dy * 1.0 < v ? 0.2  : 0.0; //
    c += i.uv.y < v ? 0.3 : 0.0;
    c += i.uv.y - dy * 1.0 < v ? 0.2  : 0.0; // 
    c += i.uv.y - dy * 2.0 < v ? 0.15 : 0.0; // antialiasing
    return float4(1,1,1,c);
}
ENDCG

    SubShader
    {
        Tags{ "RenderType" = "Transparent" "Queue" = "Transparent+1" }

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }
    }
}


```

## Source file: Assets/VertexTweaker/Runtime/Shaders/Visualizer.shader

{
  "path": "Assets/VertexTweaker/Runtime/Shaders/Visualizer.shader",
  "blob_sha": "51b2187aec38f4ff952e0de9b867901c4e338b1b",
  "sha256": "178a8eb463bccc471f4ce3113354bb3d2892461b4f2d2d82d86047057d9d6811",
  "lines": 293,
  "bytes": 7224,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 2
  },
  "url": "https://github.com/unity3d-jp/BlendShapeBuilder/blob/bf04aceb946c20ec18fa9aa437b9b35102d92867/Assets/VertexTweaker/Runtime/Shaders/Visualizer.shader",
  "function_signature_candidates": [
    {
      "line": 166,
      "text": "float4 frag(vs_out v) : SV_Target"
    },
    {
      "line": 171,
      "text": "float4 frag_brush_range(vs_out v) : SV_Target"
    }
  ]
}

```text

﻿Shader "Hidden/VertexTweaker/Visualizer" {

CGINCLUDE
#include "UnityCG.cginc"

float _VertexSize;
float _NormalSize;
float _TangentSize;
float _BinormalSize;

float4 _VertexColor;
float4 _VertexColor2;
float4 _VertexColor3;
float4 _NormalColor;
float4 _TangentColor;
float4 _BinormalColor;
float4 _BrushPos;
float4 _Direction;
int _OnlySelected = 0;

float4x4 _Transform;
StructuredBuffer<float3> _Points;
StructuredBuffer<float3> _Normals;
StructuredBuffer<float4> _Tangents;
StructuredBuffer<float> _Selection;
sampler2D _BrushSamples;


struct ia_out
{
    float4 vertex : POSITION;
    float4 normal : NORMAL;
    float4 uv : TEXCOORD0;
    float4 color : COLOR;
    uint vertexID : SV_VertexID;
    uint instanceID : SV_InstanceID;
};

struct vs_out
{
    float4 vertex : SV_POSITION;
    float4 color : TEXCOORD0;
};


vs_out vert_vertices(ia_out v)
{
    float3 pos = (mul(_Transform, float4(_Points[v.instanceID], 1.0))).xyz;

    float s = _Selection[v.instanceID];
    float4 vertex = v.vertex;
    vertex.xyz *= _VertexSize;
    vertex.xyz *= abs(mul(UNITY_MATRIX_V, float4(pos, 1.0)).z);
    vertex.xyz += pos;
    vertex = mul(UNITY_MATRIX_VP, vertex);

    vs_out o;
    o.vertex = vertex;
    o.color = lerp(_VertexColor, _VertexColor2, s);
    float d = length(pos - _BrushPos.xyz) / _BrushPos.w;
    if (d < 1) {
        o.color.rgb += _VertexColor3.rgb * tex2Dlod(_BrushSamples, float4(1 - d, 0, 0, 0)).r;
    }
    return o;
}

vs_out vert_normals(ia_out v)
{
    float3 pos = (mul(_Transform, float4(_Points[v.instanceID], 1.0))).xyz;
    float3 dir = normalize((mul(_Transform, float4(_Normals[v.instanceID], 0.0))).xyz);

    float s = _OnlySelected ? _Selection[v.instanceID] : 1.0f;
    s *= abs(mul(UNITY_MATRIX_V, float4(pos, 1.0)).z);
    float4 vertex = v.vertex;
    vertex.xyz += pos + dir * v.uv.x * _NormalSize * s;
    vertex = mul(UNITY_MATRIX_VP, vertex);

    vs_out o;
    o.vertex = vertex;
    o.color = _NormalColor;
    o.color.a = 1.0 - v.uv.x;
    return o;
}

vs_out vert_tangents(ia_out v)
{
    float3 pos = (mul(_Transform, float4(_Points[v.instanceID], 1.0))).xyz;
    float4 tangent = _Tangents[v.instanceID];
    float3 dir = normalize((mul(_Transform, float4(tangent.xyz * tangent.w, 0.0))).xyz);

    float s = _OnlySelected ? _Selection[v.instanceID] : 1.0f;
    s *= abs(mul(UNITY_MATRIX_V, float4(pos, 1.0)).z);
    float4 vertex = v.vertex;
    vertex.xyz += pos + dir * v.uv.x * _TangentSize * s;
    vertex = mul(UNITY_MATRIX_VP, vertex);

    vs_out o;
    o.vertex = vertex;
    o.color = _TangentColor;
    o.color.a = 1.0 - v.uv.x;
    return o;
}

vs_out vert_binormals(ia_out v)
{
    float3 pos = (mul(_Transform, float4(_Points[v.instanceID], 1.0))).xyz;
    float4 tangent = _Tangents[v.instanceID];
    float3 binormal = normalize(cross(_Normals[v.instanceID], tangent.xyz * tangent.w));
    float3 dir = normalize((mul(_Transform, float4(binormal, 0.0))).xyz);

    float s = _OnlySelected ? _Selection[v.instanceID] : 1.0f;
    s *= abs(mul(UNITY_MATRIX_V, float4(pos, 1.0)).z);
    float4 vertex = v.vertex;
    vertex.xyz += pos + dir * v.uv.x * _BinormalSize * s;
    vertex = mul(UNITY_MATRIX_VP, vertex);

    vs_out o;
    o.vertex = vertex;
    o.color = _BinormalColor;
    o.color.a = 1.0 - v.uv.x;
    return o;
}

vs_out vert_lasso(ia_out v)
{
    vs_out o;
    o.vertex = float4(v.vertex.xy, 0.0, 1.0);
    o.vertex.y *= -1;
    o.color = float4(1.0, 0.0, 0.0, 1.0);
    return o;
}

vs_out vert_brush_range(ia_out v)
{
    vs_out o;
    o.vertex = UnityObjectToClipPos(v.vertex);
    o.color = mul(UNITY_MATRIX_M, v.vertex);
    return o;
}

vs_out vert_ray_position(ia_out v)
{
    float z = abs(mul(UNITY_MATRIX_V, float4(_BrushPos.xyz, 1.0)).z);
    float3 pos = v.vertex.xyz * (0.01 * z) + _BrushPos.xyz;

    vs_out o;
    o.vertex = UnityObjectToClipPos(pos);
    o.color = float4(1, 0, 0, 1);
    return o;
}

vs_out vert_direction(ia_out v)
{
    float3 pos = _BrushPos.xyz;
    float3 dir = _Direction.xyz;
    float z = abs(mul(UNITY_MATRIX_V, float4(_BrushPos.xyz, 1.0)).z);
    float4 vertex = float4(pos + dir * (v.uv.x * 0.25 * z), 1.0);

    vs_out o;
    o.vertex = mul(UNITY_MATRIX_VP, vertex);
    o.color = float4(1, 0, 0, 1-v.uv.x);
    return o;
}


float4 frag(vs_out v) : SV_Target
{
    return v.color;
}

float4 frag_brush_range(vs_out v) : SV_Target
{
    float2 uv = v.color.xy / v.color.w;

    float3 pixel_pos = v.color.xyz;
    float3 brush_pos = _BrushPos.xyz;
    float distance = length(pixel_pos - brush_pos);

    float range = clamp(1.0f - distance / _BrushPos.w, 0, 1);
    float z = abs(mul(UNITY_MATRIX_V, float4(brush_pos, 1.0)).z);

    float border = 0.004 / _BrushPos.w * z;
    if (distance > _BrushPos.w || range > border) { discard; }

    float4 color = float4(1, 0, 0, 1);
    return color;
}

ENDCG

    SubShader
    {
        Tags{ "RenderType" = "Transparent" "Queue" = "Transparent+100" }
        Blend SrcAlpha OneMinusSrcAlpha
        ZWrite Off

        // pass 0: visualize vertices
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_vertices
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 1: visualize normals
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_normals
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 2: visualize tangents
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_tangents
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 3: visualize binormals
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_binormals
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 4: lasso
        Pass
        {
            ZTest Always

            CGPROGRAM
            #pragma vertex vert_lasso
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 5: brush range
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_brush_range
            #pragma fragment frag_brush_range
            #pragma target 4.5
            ENDCG
        }

        // pass 6: ray position
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_ray_position
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }

        // pass 7: direction
        Pass
        {
            ZTest LEqual

            CGPROGRAM
            #pragma vertex vert_direction
            #pragma fragment frag
            #pragma target 4.5
            ENDCG
        }
    }
}


```

## Source file: .BlendShapeBuilder/Assets/BlendShapeBuilderPackaging.cs

{
  "path": ".BlendShapeBuilder/Assets/BlendShapeBuilderPackaging.cs",
  "blob_sha": "7f5771bc76f95fbf247cd2624bd4c73f1c23f0e8",
  "sha256": "9ca2cb3665483999805617aa852058ec9f9e108693dc5a7ae18c5e18a394fef6",
  "lines": 20,
  "bytes": 509,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 4
  },
  "url": "https://github.com/unity3d-jp/BlendShapeBuilder/blob/bf04aceb946c20ec18fa9aa437b9b35102d92867/.BlendShapeBuilder/Assets/BlendShapeBuilderPackaging.cs",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "public static void MakePackage()"
    }
  ]
}

```text

﻿#if UNITY_EDITOR
using UnityEngine;
using UnityEditor;


public class BlendShapeBuilderPackaging
{
    [MenuItem("Assets/Make BlendShapeBuilder.unitypackage")]
    public static void MakePackage()
    {
        string[] files = new string[]
        {
            "Assets/UTJ/BlendShapeBuilder",
            "Assets/UTJ/VertexTweaker",
        };
        AssetDatabase.ExportPackage(files, "BlendShapeBuilder.unitypackage", ExportPackageOptions.Recurse);
    }

}
#endif // UNITY_EDITOR


```

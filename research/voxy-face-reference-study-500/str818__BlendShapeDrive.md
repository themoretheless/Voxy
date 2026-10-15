# Facial rendering source scan: str818/BlendShapeDrive

Repository: https://github.com/str818/BlendShapeDrive

Pinned source revision: 32b3724cd7d2a25236b33d331a9660e7d852b722

Discovery topic: blendshape

Description: 基于BlendShape表情基与计算着色器实现对人脸表情的控制

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": [".vs/BlendShape Drive/v15/.suo", ".vs/BlendShape Drive/v15/Browse.VC.db", ".vs/BlendShape Drive/v15/sqlite3/storage.ide", "Assets/BlendShape.cs", "Assets/BlendShape.cs.meta", "BlendShape Drive.csproj", "BlendShape Drive.sln"], "shader": ["Assets/ComputeShader.compute", "Assets/ComputeShader.compute.meta", "Assets/ComputeShaderCS.cs", "Assets/ComputeShaderCS.cs.meta", "Library/ShaderCache.db", "Library/ShaderCache/0/002b58be7c3e7374f19e6d487d762980.bin", "Library/ShaderCache/0/01af25d4589f5ada85837230535242c0.bin", "Library/ShaderCache/0/02a68e9ca4a27a34bae2293ace086f45.bin", "Library/ShaderCache/0/0a7b9d78acf7777694439441ed6086db.bin", "Library/ShaderCache/1/134bce2f382f03687517b398e8ae7787.bin", "Library/ShaderCache/1/16aa283808e8e9f6ed916590a5a71da7.bin", "Library/ShaderCache/1/1a21a34021af8c2e645f92a19a0aaf4c.bin", "Library/ShaderCache/2/26830d927e68c5dfbd37721f89b4ac66.bin", "Library/ShaderCache/3/3708aeb18161596701281f769798f12a.bin", "Library/ShaderCache/3/3a6703434c8e75dfba7bb4cfc82ef059.bin", "Library/ShaderCache/4/46835ef14ff8b76ce23a8f09a4b6ff16.bin", "Library/ShaderCache/4/47fcc257e7aeec0577cd8896bd35bc54.bin", "Library/ShaderCache/6/61b5f4bc2861714fe6058915698ef662.bin", "Library/ShaderCache/6/61c4ba8bc89b7ae74d722c635696c5ed.bin", "Library/ShaderCache/6/65d766d24340875b2091df1ab58d5907.bin", "Library/ShaderCache/6/672e171e7f41857c370565b7021e255b.bin", "Library/ShaderCache/6/69d86e0512b63da77c01e95c39c4aab1.bin", "Library/ShaderCache/6/6df9dd994bac85732efc2a415c6b2c8e.bin", "Library/ShaderCache/6/6e9f3440a522e796c27c8c2545d2b1b5.bin", "Library/ShaderCache/7/72c4ba1fb4a7a16afc9e52e7f64470ca.bin", "Library/ShaderCache/7/76a0bdd25f6a90e3bbd2927d6965746a.bin", "Library/ShaderCache/7/76af463369b54d74c3a3cd54159f8784.bin", "Library/ShaderCache/7/781198686213d8a945ce359a6979d927.bin", "Library/ShaderCache/7/7f3b0b1e58c2ed42de56a9f07487d195.bin", "Library/ShaderCache/8/8448bde5a624bdb05788943dd3ee65b7.bin"], "face": ["Assets/Materials/polySurface1Mat.mat", "Assets/Materials/polySurface1Mat.mat.meta"]}

## Source file: Assets/BlendShape.cs

{
  "path": "Assets/BlendShape.cs",
  "blob_sha": "9e7c89fd12f3133853c15c8e6227afd17a6fb02a",
  "sha256": "3d4ef44b94db854fb4a05b86288279807e6e0df55a5bd587eabac9a63ff051b2",
  "lines": 67,
  "bytes": 1934,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1
  },
  "url": "https://github.com/str818/BlendShapeDrive/blob/32b3724cd7d2a25236b33d331a9660e7d852b722/Assets/BlendShape.cs",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "void Start() {"
    },
    {
      "line": 22,
      "text": "public void SetWeight(int argIndex, float argWeight) {"
    },
    {
      "line": 27,
      "text": "void Update() {"
    },
    {
      "line": 39,
      "text": "void UpdateMesh() {"
    }
  ]
}

```text

﻿using UnityEngine;
using System.Collections;

[RequireComponent(typeof(MeshFilter))]
public class BlendShape : MonoBehaviour {
    //表情基的Mesh网格
    public Mesh[] meshs;
    //每个表情基的权重值数组
    public float[] weights;

    bool isChange = false;

    void Start() {
        //初始化权重值数组
        weights = new float[meshs.Length];
        System.Array.Clear(weights, 0, weights.Length);
        //将当前网格置为默认的模型,即第一个表情基模型
        GetComponent<MeshFilter>().mesh = meshs[0];
    }

    //设置表情基的权重值
    public void SetWeight(int argIndex, float argWeight) {
        weights[argIndex] = (argWeight > 1 ? 1 : argWeight);
        isChange = true;
    }

    void Update() {
        if (Input.GetKeyDown(KeyCode.W)) {
            SetWeight(1, weights[1] + 0.1f);
        }

        if (Input.GetKeyDown(KeyCode.S)) {
            SetWeight(1, weights[1] - 0.1f);
        }
        UpdateMesh();
    }

    //更新网格
    void UpdateMesh() {
#if UNITY_EDITOR
#else
        if (!isChange) return;
#endif
        //获取模型的网格信息
        Mesh mesh = GetComponent<MeshFilter>().mesh;
        //初始化模型的顶点数组
        Vector3[] vertices = new Vector3[meshs[0].vertices.Length];
        System.Array.Copy(meshs[0].vertices, vertices, vertices.Length);

        //根据权重值与表情基计算新的顶点位置
        for (int w = 1; w < weights.Length; w++) {
            if (weights[w] <= 0) continue;

            //获取表情基顶点
            Vector3[] verticesT = meshs[w].vertices;

            int i = 0;
            while (i < vertices.Length) {
                vertices[i] = vertices[i] + (verticesT[i] - meshs[0].vertices[i]) * weights[w];
                i++;
            }
        }
        //改变顶点位置坐标
        mesh.vertices = vertices;
        isChange = false;
    }
}


```

## Source file: Assets/ComputeShaderCS.cs

{
  "path": "Assets/ComputeShaderCS.cs",
  "blob_sha": "ec272657fee9581ffb54c0725cd9e506ecf73a73",
  "sha256": "4345c964772836728f4370cdfd4dc7cf6c11debf147fefdc7290ae6fb6286b5c",
  "lines": 83,
  "bytes": 2498,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 5,
    "shader": 11
  },
  "url": "https://github.com/str818/BlendShapeDrive/blob/32b3724cd7d2a25236b33d331a9660e7d852b722/Assets/ComputeShaderCS.cs",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "private void Start() {"
    },
    {
      "line": 32,
      "text": "private void Update() {"
    },
    {
      "line": 38,
      "text": "void RunShader() {"
    },
    {
      "line": 78,
      "text": "void OnDestroy() {"
    }
  ]
}

```text

﻿using UnityEngine;
using System.Collections;
using System.Collections.Generic;

[RequireComponent(typeof(MeshFilter))]
public class ComputeShaderCS : MonoBehaviour {
    //表情基的Mesh网格 0为natural模型
    public Mesh[] meshs;
    //每个表情基的权重值数组(限制在0-1)
    [Range(0,1)]
    public float[] weights;
    //计算着色器
    public ComputeShader shader;

    ComputeBuffer vertBuffer;       //顶点缓冲
    ComputeBuffer blendshapeBuffer; //表情基模型缓冲
    ComputeBuffer naturalBuffer;    //natural模型缓冲

    //模型网格信息
    Mesh mesh;

    private void Start() {
        //获取模型的网格信息
        mesh = GetComponent<MeshFilter>().mesh;

        //分配缓冲空间(每个float占4个字节，一个Vector3包括3个float,所占大小为4*3)
        vertBuffer = new ComputeBuffer(meshs[0].vertices.Length, 4 * 3);
        blendshapeBuffer = new ComputeBuffer(meshs[0].vertices.Length, 4 * 3);
        naturalBuffer = new ComputeBuffer(meshs[0].vertices.Length, 4 * 3);
    }

    private void Update() {
        //不必每一帧执行，此处为便于测试
        RunShader();
    }

    //执行计算着色器
    void RunShader() {

        int kernel = shader.FindKernel("Cul");

        Vector3[] vertices = new Vector3[meshs[0].vertices.Length];

        //初始化缓冲
        vertBuffer.SetData(meshs[0].vertices);
        naturalBuffer.SetData(meshs[0].vertices);

        //填充缓冲
        shader.SetBuffer(kernel, "vertBuffer", vertBuffer);
        shader.SetBuffer(kernel, "naturalBuffer", naturalBuffer);

        //根据权重值与表情基计算新的顶点位置
        for (int w = 1; w < weights.Length; w++) {

            if (weights[w] <= 0) continue;

            //初始化缓冲
            blendshapeBuffer.SetData(meshs[w].vertices);

            //填充缓冲
            shader.SetBuffer(kernel, "blendshapeBuffer", blendshapeBuffer);
            
            //设置权重
            shader.SetFloat("weight", weights[w]);

            //执行Compute Shader
            shader.Dispatch(kernel, 4, 4, 1);

        }

        //获取计算结果
        vertBuffer.GetData(vertices);

        //更改网格结构
        mesh.vertices = vertices;
    }

    void OnDestroy() {
        if (vertBuffer != null) vertBuffer.Release();
        if (blendshapeBuffer != null) blendshapeBuffer.Release();
        if (naturalBuffer != null) naturalBuffer.Release();
    }
}

```

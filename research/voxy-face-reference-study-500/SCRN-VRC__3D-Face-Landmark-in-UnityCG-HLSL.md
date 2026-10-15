# Facial rendering source scan: SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL

Repository: https://github.com/SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL

Pinned source revision: 2502c0a21c08fd82900d855ff6acc095d9b95394

Discovery topic: face model 3D

Description: 3D face landmark detection using MediaPipe's Facemesh and Iris tracking models

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset", "FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset.meta", "FaceLandmarkDetection/CRTs/Iris/GetEyes.asset", "FaceLandmarkDetection/CRTs/Iris/GetEyes.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes RR.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes RR.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes.asset.meta", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat.meta", "FaceLandmarkDetection/Materials/Iris/GetEyes.mat", "FaceLandmarkDetection/Materials/Iris/GetEyes.mat.meta", "FaceLandmarkDetection/Materials/Iris/LR_Eyes RR.mat", "FaceLandmarkDetection/Materials/Iris/LR_Eyes RR.mat.meta", "FaceLandmarkDetection/Materials/Iris/LR_Eyes.mat", "FaceLandmarkDetection/Materials/Iris/LR_Eyes.mat.meta", "FaceLandmarkDetection/Materials/World/EyeDebug.mat", "FaceLandmarkDetection/Materials/World/EyeDebug.mat.meta", "Python/data/eye1.jpg", "Python/data/eye2.jpg", "Python/data/eye3.jpg"], "blendshape": ["FaceLandmarkDetection/Editor/BakeBlendshapes.cs", "FaceLandmarkDetection/Editor/BakeBlendshapes.cs.meta"], "iris": ["C++/iris-detection/iris-detection.cpp", "FaceLandmarkDetection/CRTs/Iris.meta", "FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset", "FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset.meta", "FaceLandmarkDetection/CRTs/Iris/GetEyes.asset", "FaceLandmarkDetection/CRTs/Iris/GetEyes.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Brows RR.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Brows RR.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Brows.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Brows.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes RR.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes RR.asset.meta", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes.asset", "FaceLandmarkDetection/CRTs/Iris/LR_Eyes.asset.meta", "FaceLandmarkDetection/CRTs/Iris/Procrustes Analysis Iris.asset", "FaceLandmarkDetection/CRTs/Iris/Procrustes Analysis Iris.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L0.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L0.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L1.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L1.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L10.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L10.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L11.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L11.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L12.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L12.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L13.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L13.asset.meta", "FaceLandmarkDetection/CRTs/Iris/iris_L14.asset", "FaceLandmarkDetection/CRTs/Iris/iris_L14.asset.meta"], "shader": ["FaceLandmarkDetection/Demos/Bricks Game/Shaders.meta", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksButtons.shader", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksButtons.shader.meta", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksInc.cginc", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksInc.cginc.meta", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksRender.shader", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksRender.shader.meta", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksState.shader", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/BricksState.shader.meta", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/ShaderGames.cginc", "FaceLandmarkDetection/Demos/Bricks Game/Shaders/ShaderGames.cginc.meta", "FaceLandmarkDetection/Demos/Face Tracking/Shaders.meta", "FaceLandmarkDetection/Demos/Face Tracking/Shaders/FaceTrack.shader", "FaceLandmarkDetection/Demos/Face Tracking/Shaders/FaceTrack.shader.meta", "FaceLandmarkDetection/Demos/Face Tracking/Shaders/HeadRotate.shader", "FaceLandmarkDetection/Demos/Face Tracking/Shaders/HeadRotate.shader.meta", "FaceLandmarkDetection/Scripts/ShaderDebugging.cs", "FaceLandmarkDetection/Scripts/ShaderDebugging.cs.meta", "FaceLandmarkDetection/Shaders.meta", "FaceLandmarkDetection/Shaders/BlendValues.shader", "FaceLandmarkDetection/Shaders/BlendValues.shader.meta", "FaceLandmarkDetection/Shaders/BlendValuesInclude.cginc", "FaceLandmarkDetection/Shaders/BlendValuesInclude.cginc.meta", "FaceLandmarkDetection/Shaders/DrawLandmarks.shader", "FaceLandmarkDetection/Shaders/DrawLandmarks.shader.meta", "FaceLandmarkDetection/Shaders/FaceMeshModel.shader", "FaceLandmarkDetection/Shaders/FaceMeshModel.shader.meta", "FaceLandmarkDetection/Shaders/IrisInclude.cginc", "FaceLandmarkDetection/Shaders/IrisInclude.cginc.meta", "FaceLandmarkDetection/Shaders/IrisModel.shader"], "rig": ["FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset", "FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset.meta", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat.meta", "FaceLandmarkDetection/Textures/Dame Dane Original Template Video - Baka Mitai (full video) Tutorial in description (256p_24fps_H264-96kbit_AAC).mp4", "FaceLandmarkDetection/Textures/Dame Dane Original Template Video - Baka Mitai (full video) Tutorial in description (256p_24fps_H264-96kbit_AAC).mp4.meta"], "face": ["C++/face-landmark-detection/face-landmark-detection.cpp", "FaceLandmarkDetection/Animations.meta", "FaceLandmarkDetection/Animations/TestHead.controller", "FaceLandmarkDetection/Animations/TestHead.controller.meta", "FaceLandmarkDetection/Animations/TurnOffBlending.anim", "FaceLandmarkDetection/Animations/TurnOffBlending.anim.meta", "FaceLandmarkDetection/Animations/TurnOnBlending.anim", "FaceLandmarkDetection/Animations/TurnOnBlending.anim.meta", "FaceLandmarkDetection/CRTs.meta", "FaceLandmarkDetection/CRTs/BlendValues.asset", "FaceLandmarkDetection/CRTs/BlendValues.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L0.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L0.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L1.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L1.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L10.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L10.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L10_BN.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L10_BN.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L11.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L11.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L11_BN.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L11_BN.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L12.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L12.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L12_MP_BN.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L12_MP_BN.asset.meta", "FaceLandmarkDetection/CRTs/FaceMesh/L13.asset", "FaceLandmarkDetection/CRTs/FaceMesh/L13.asset.meta"], "lip": ["FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset", "FaceLandmarkDetection/CRTs/Iris/FlipRightEye.asset.meta", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat", "FaceLandmarkDetection/Materials/Iris/FlipRightEye.mat.meta"]}

## Source file: FaceLandmarkDetection/Shaders/IrisModel.shader

{
  "path": "FaceLandmarkDetection/Shaders/IrisModel.shader",
  "blob_sha": "0ca30ae17bc8e92b267eca3e0631bc2d78642e0d",
  "sha256": "651caae404286121b7b9fe7934063bd32a08ca1c2adbd3571e96dd3d46356351",
  "lines": 957,
  "bytes": 37572,
  "selection_score": 33,
  "keyword_line_counts": {
    "eye": 43,
    "iris": 32,
    "shader": 16,
    "rig": 13,
    "face": 3,
    "lip": 16
  },
  "url": "https://github.com/SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL/blob/2502c0a21c08fd82900d855ff6acc095d9b95394/FaceLandmarkDetection/Shaders/IrisModel.shader",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 119,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 198,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 260,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 327,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 395,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 464,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 546,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 622,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 674,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 715,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 756,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 876,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 934,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    }
  ]
}

```text

﻿Shader "FaceLandmarkDetection/Iris Model"
{
    Properties
    {
        [Header(Weights)]
        _Weights ("Baked Weights", 2D) = "black" {}
        _KernelSize ("Kernel Size", Int) = 3
        _WeightID ("Weight ID", Int) = 0
        _BiasID ("Bias ID", Int) = 0
        _MultiplierID ("Multiplier ID", Int) = 0
        [Header(Previous Layers)]
        _CamIn ("Camera Input", 2D) = "black" {}
        _Layer1 ("Layer Input 1", 2D) = "black" {}
        _Layer2 ("Layer Input 2", 2D) = "black" {}
        _Layer12WH ("[Width, Height, Width, Height]", Vector) =  (1, 1, 1, 1)
        [Header(Current Layer)]
        _WHDM ("[Width, Height, Loop Depth, Null]", Vector) = (1, 1, 0, 0)
    }

    SubShader
    {
        Tags { "Queue"="Geometry" "ForceNoShadowCasting"="True" "IgnoreProjector"="True" }
        Lighting Off
        Fog { Mode Off }
        Blend One Zero
        ZTest Never

        Pass
        {
            Name "Get Eyes"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            sampler2D         _CamIn;
            Texture2D<float>  _Layer1;
            Texture2D<float3> _Layer2;
            Texture2D<float3> _Weights; // Use this texture for the face rotations

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const float2 uv = IN.globalTexcoord.xy;

                // Swap between both eyes so only one network needed
                // for two eyes
                bool flipEye = abs(_Layer2[uint2(0, 0)] - 0.1337) < 0.001;

                // Right/Left coords for eye position from facemesh
                uint2 eyeNPos = flipEye ? uint2(9, 12) : uint2(27, 29);
                uint2 eyeSPos = flipEye ? uint2(6, 11) : uint2(30, 28);
                uint2 eyeEPos = flipEye ? uint2(9, 10) : uint2(9, 20);
                uint2 eyeWPos = flipEye ? uint2(21, 2) : uint2(33, 27);

                float2 eyeN = float2(_Layer1[eyeNPos], _Layer1[eyeNPos + uint2(1, 0)]);
                float2 eyeS = float2(_Layer1[eyeSPos], _Layer1[eyeSPos + uint2(1, 0)]);
                float2 eyeE = float2(_Layer1[eyeEPos], _Layer1[eyeEPos + uint2(1, 0)]);
                float2 eyeW = float2(_Layer1[eyeWPos], _Layer1[eyeWPos + uint2(1, 0)]);

                // Undo rotation
                float3x3 look = 0.0;

                for (uint i = 0; i < 6; i++)
                {
                    look[0] += _Weights[uint2(0, i)];
                    look[1] += _Weights[uint2(1, i)];
                    look[2] += _Weights[uint2(2, i)];
                }
                look *= 0.1667;

                // float angX = atan2(look[2][1], look[2][2]);
                float angY = atan2(-look[2][0],
                     sqrt(look[2][1] * look[2][1] + look[2][2] * look[2][2]));
                float angZ = atan2(look[1][0], look[0][0]);

                //buffer[0] = float4(dot(_Layer2[uint2(0, 0)], 0.3333.xxx).xxxx);

                // Compute center
                float2 eyeCentroid = (eyeN + eyeS + eyeE + eyeW) * 0.25 / 192.0;
                float2 uvZoom = uv - eyeCentroid.yx;
                uvZoom *= 0.3;
                uvZoom = mul(rot2(angZ), uvZoom);
                uvZoom += eyeCentroid.yx + eyeCentroid.yx * 0.15 - float2(0.12, 0.07);

                float4 col = tex2D(_CamIn, uvZoom);
                // Flip eye
                if (all(uv.xy <= 0.01.xx))
                {
                    col = flipEye ? 1..xxxx : 0.1337.xxxx;
                }
                return col;
            }
            ENDCG
        }

        Pass
        {
            Name "Camera Input"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            sampler2D        _CamIn;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                float cl = 0.0;
                const uint i0 = i * 2, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = j * 2, j1 = j0 + 1, j2 = j0 + 2;
                // 3x3 kernel
                [unroll(3)]
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl +=
                        getCameraVP(_CamIn, uint3(i0, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 0, 0), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i0, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 0, 1), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i0, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 0, 2), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 1, 0), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 1, 1), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 1, 2), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 2, 0), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 2, 1), _KernelSize, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(k, l, 2, 2), _KernelSize, _WeightID);

                        // test(i0, j0, l) * getConst(_Weights, uint4(k, l, 0, 0), _KernelSize, _WeightID) +
                        // test(i0, j1, l) * getConst(_Weights, uint4(k, l, 0, 1), _KernelSize, _WeightID) +
                        // test(i0, j2, l) * getConst(_Weights, uint4(k, l, 0, 2), _KernelSize, _WeightID) +
                        // test(i1, j0, l) * getConst(_Weights, uint4(k, l, 1, 0), _KernelSize, _WeightID) +
                        // test(i1, j1, l) * getConst(_Weights, uint4(k, l, 1, 1), _KernelSize, _WeightID) +
                        // test(i1, j2, l) * getConst(_Weights, uint4(k, l, 1, 2), _KernelSize, _WeightID) +
                        // test(i2, j0, l) * getConst(_Weights, uint4(k, l, 2, 0), _KernelSize, _WeightID) +
                        // test(i2, j1, l) * getConst(_Weights, uint4(k, l, 2, 1), _KernelSize, _WeightID) +
                        // test(i2, j2, l) * getConst(_Weights, uint4(k, l, 2, 2), _KernelSize, _WeightID);
                }

                // bias
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID);

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));
                
                //if (i == 12 && j == 2 && k == 63)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += tex2D(_CamIn, float2(0, 0)).x < 0.5 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "conv2D1x1Stride1"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                float cl = 0.0;

                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }

                // bias
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID);

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));
                
                //if (i == 1 && j == 0 && k == 62)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "depthConv2D3x3"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                const uint i0 = i, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = j, j1 = j0 + 1, j2 = j0 + 2;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                // 3x3 kernel
                float cl =
                    getLayerSP(_Layer1, uint3(i0, j0, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 0, 0), _WeightID) +
                    getLayerSP(_Layer1, uint3(i0, j1, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 0, 1), _WeightID) +
                    getLayerSP(_Layer1, uint3(i0, j2, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 0, 2), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j0, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 1, 0), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j1, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 1, 1), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j2, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 1, 2), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j0, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 2, 0), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j1, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 2, 1), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j2, k), _Layer12WH.xy, mult1) * getConst(_Weights, uint3(k, 2, 2), _WeightID);
                
                // bias
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID);
                
                //if (i == 15 && j == 14 && k == 55)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "conv2D1x1Stride1Add"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            Texture2D<float> _Layer2;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                uint2 PLD2;
                _Layer2.GetDimensions(PLD2.x, PLD2.y);
                const uint2 mult2 = PLD2 / _Layer12WH.zw;

                float cl = 0.0;

                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }

                // bias + skip in
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID) +
                    getLayerNP(_Layer2, uint3(i, j, k), _Layer12WH.zw, mult2);

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));

                //if (i == 7 && j == 6 && k == 127)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "conv2D2x2Stride2"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                const uint i0 = i * 2, i1 = i0 + 1;
                const uint j0 = j * 2, j1 = j0 + 1;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                float cl = 0.0;

                // 2x2 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl +=
                        getLayerNP(_Layer1, uint3(i0, j0, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 0, 0), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(i0, j1, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 0, 1), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j0, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 1, 0), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j1, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 1, 1), _KernelSize, _WeightID);
                }
                // bias
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID);

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));

                //if (i == 1 && j == 0 && k == 62)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "maxPool2x2Stride2Padded"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            Texture2D<float> _Layer2;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                uint2 PLD2;
                _Layer2.GetDimensions(PLD2.x, PLD2.y);
                const uint2 mult2 = PLD2 / _Layer12WH.zw;

                float cl = 0.0;

                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }

                // max pool
                const uint i0 = i * 2, i1 = i0 + 1;
                const uint j0 = j * 2, j1 = j0 + 1;

                // pad extra layers with 0
                float mp = 0.0;
                if (k < _WHDM.z)
                {
                    mp = max4(getLayerNP(_Layer2, uint3(i0, j0, k), _Layer12WH.zw, mult2),
                        getLayerNP(_Layer2, uint3(i0, j1, k), _Layer12WH.zw, mult2),
                        getLayerNP(_Layer2, uint3(i1, j0, k), _Layer12WH.zw, mult2),
                        getLayerNP(_Layer2, uint3(i1, j1, k), _Layer12WH.zw, mult2));
                }

                // bias + skip in
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID) + mp;

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));

                //if (i == 15 && j == 14 && k == 127)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "maxPool2x2Stride2NoPad"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            Texture2D<float> _Layer2;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                uint2 PLD2;
                _Layer2.GetDimensions(PLD2.x, PLD2.y);
                const uint2 mult2 = PLD2 / _Layer12WH.zw;

                float cl = 0.0;

                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }

                // max pool
                const uint i0 = i * 2, i1 = i0 + 1;
                const uint j0 = j * 2, j1 = j0 + 1;

                float mp = max4(getLayerNP(_Layer2, uint3(i0, j0, k), _Layer12WH.zw, mult2),
                    getLayerNP(_Layer2, uint3(i0, j1, k), _Layer12WH.zw, mult2),
                    getLayerNP(_Layer2, uint3(i1, j0, k), _Layer12WH.zw, mult2),
                    getLayerNP(_Layer2, uint3(i1, j1, k), _Layer12WH.zw, mult2));

                // bias + skip in
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID) + mp;

                // activation
                cl = PRelu(cl, getConst(_Weights, uint2(k, 0), _MultiplierID));

                //if (i == 15 && j == 14 && k == 127)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "conv2D2x2Stride1"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _KernelSize;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                const uint k = px.x + px.y * _WHDM.x;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                float cl = 0.0;

                // 2x2 kernel
                for (uint l = 0; l < _WHDM.z; l++)
                {
                    cl +=
                        getLayerNP(_Layer1, uint3(0, 0, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 0, 0), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(0, 1, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 0, 1), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(1, 0, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 1, 0), _KernelSize, _WeightID) +
                        getLayerNP(_Layer1, uint3(1, 1, l), _Layer12WH.xy, mult1) * getConst(_Weights, uint4(k, l, 1, 1), _KernelSize, _WeightID);
                }
                // bias
                cl = cl + getConst(_Weights, uint2(0, k), _BiasID);

                //if (k == 14)
                //    buffer[0] = cl;

                if (all(px == 0))
                {
                    // Left/Right eye flag
                    cl += _Layer1[uint2(0, 0)] < 100000.0 ? 0.0 : 10000000.0;
                }
                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "LR Eyes"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float>   _Layer1;
            Texture2D<float4>  _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                uint eyeIndex = _Layer1[uint2(0, 0)] < 100000.0 ? 0 : 1;

                float4 col = _Layer2[px];
                
                if (eyeIndex == px.x)
                {
                    col.r = _Layer1[uint2(0, px.y)];
                    col.r = col.r > 100000.0 ? col.r - 10000000.0 : col.r;
                    col.r /= 64.0;
                    col.g = _Layer1[uint2(1, px.y)] / 64.0;
                    col.b = _Layer1[uint2(2, px.y)] / 64.0;
                    // Flip the right eye back
                    col.r = eyeIndex ? col.r : 1.0 - col.r;
                }

                //if (px.x == 1)
                //buffer[0] = float4(eyeIndex.xxxx);

                return col;
            }
            ENDCG
        }

        Pass
        {
            Name "LR Brows Contours"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            Texture2D<float>   _Layer1;
            Texture2D<float4>  _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                uint eyeIndex = _Layer1[uint2(0, 0)] < 100000.0 ? 0 : 1;

                float4 col = _Layer2[px];
                
                if (eyeIndex == (px.y / 9))
                {
                    uint h = px.x + (px.y - (px.y / 9) * 9) * 8;
                    col.r = _Layer1[uint2(0, h)];
                    col.r = col.r > 100000.0 ? col.r - 10000000.0 : col.r;
                    col.r /= 64.0;
                    col.g = _Layer1[uint2(1, h)] / 64.0;
                    col.b = _Layer1[uint2(2, h)] / 64.0;
                    // Flip the right eye back
                    col.r = eyeIndex ? col.r : 1.0 - col.r;
                }

                return col;
            }
            ENDCG
        }

        Pass
        {
            Name "Procrustes Analysis"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #include "svd_hlsl.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float3> _Layer1;
            Texture2D<float3> _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                bool leftSide = px.x < 8;
                float4 col = 0.0;

                if (px.y == 0)
                {
                    float3x3 X;
                    X[0] = _Layer1[uint2(0, 7 + (leftSide ? 0 : 9))];
                    X[1] = _Layer1[uint2(5, 7 + (leftSide ? 0 : 9))];
                    X[2] = _Layer1[uint2(3, 8 + (leftSide ? 0 : 9))];

                    float3x3 Y = leftSide ? eyeLInit : eyeRInit;

                    const float3 muX = findCenteroid(X);
                    const float3 muY = findCenteroid(Y);

                    float3x3 X0;
                    float3x3 Y0;
                    uint i, j;
                    for (i = 0; i < 3; i++)
                    {
                        X0[i] = X[i] - muX;
                        Y0[i] = Y[i] - muY;
                    }

                    // squared error
                    float ssX = 0.0;
                    float ssY = 0.0;
                    [unroll]
                    for (i = 0; i < 3; i++)
                    {
                        [unroll]
                        for (j = 0; j < 3; j++)
                        {
                            ssX += X0[i][j] * X0[i][j];
                            ssY += Y0[i][j] * Y0[i][j];
                        }
                    }

                    // centred Frobenius norm
                    const float normX = sqrt(ssX);
                    const float normY = sqrt(ssY);

                    // scale to equal (unit) norm
                    [unroll]
                    for (i = 0; i < 3; i++)
                    {
                        [unroll]
                        for (j = 0; j < 3; j++)
                        {
                            X0[i][j] /= normX;
                            Y0[i][j] /= normY;
                        }
                    }

                    // Singular value decomposition of a 3x3 matrix
                    float3x3 A = mul(transpose(X0), Y0);
                    float3x3 U;
                    float3 D;
                    float3x3 Vt;
                    GetSVD3D(A, U, D, Vt);
                    
                    // solve optimum rotation matrix of Y
                    float3x3 V = transpose(Vt);
                    float3x3 T = mul(V, transpose(U));

                    bool have_reflection = determinant(T) < 0.0;
                    if (have_reflection != false)
                    {
                        V[0][2] = -V[0][2];
                        V[1][2] = -V[1][2];
                        V[2][2] = -V[2][2];
                        D[2] = -D[2];
                        T = mul(V, transpose(U));
                    }

                    const float traceTA = D[0] + D[1] + D[2];

                    StoreValue(txIrisRotation0 + uint2(leftSide ? 0 : 8, 0),
                        float4(T[0], 0.0), col, px);
                    StoreValue(txIrisRotation1 + uint2(leftSide ? 0 : 8, 0),
                        float4(T[1], 0.0), col, px);
                    StoreValue(txIrisRotation2 + uint2(leftSide ? 0 : 8, 0),
                        float4(T[2], 0.0), col, px);
                    StoreValue(txIrisScaleYNorm + uint2(leftSide ? 0 : 8, 0),
                        float4(traceTA * normX, normY, 0.0, 0.0), col, px);
                    StoreValue(txIrisXCentroid + uint2(leftSide ? 0 : 8, 0),
                        float4(muX, 0.0), col, px);
                    StoreValue(txIrisYCentroid + uint2(leftSide ? 0 : 8, 0),
                        float4(muY, 0.0), col, px);
                }
                else
                {
                    // Keep history for smoothing
                    col.rgb = _Layer2[uint2(px.x, px.y - 1)];
                }

                return col;
            }
            ENDCG
        }

        Pass
        {
            Name "Reverse Rotation"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float3> _Layer1;
            Texture2D<float3> _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                uint2 texWH;
                _Layer1.GetDimensions(texWH.x, texWH.y);

                // iris or brow texture
                bool isIris = (texWH.x == 2);
                bool leftSide = isIris ? (px.x == 0) : (px.y < 9);

                float3 pos = _Layer1[px];
                // The iris depth fucks with the SVD rotation
                pos.z = isIris ? 0.0 : pos.z;

                float3 YCentroid = 0.0;
                float3 XCentroid = 0.0;
                float2 scaleYNorm = 0.0;
                float3x3 look = 0.0;

                for (uint i = 0; i < 6; i++)
                {
                    look[0] += _Layer2[txIrisRotation0 + uint2(leftSide ? 0 : 8, 0)];
                    look[1] += _Layer2[txIrisRotation1 + uint2(leftSide ? 0 : 8, 0)];
                    look[2] += _Layer2[txIrisRotation2 + uint2(leftSide ? 0 : 8, 0)];
                    YCentroid += _Layer2[txIrisYCentroid + uint2(leftSide ? 0 : 8, 0)];
                    XCentroid += _Layer2[txIrisXCentroid + uint2(leftSide ? 0 : 8, 0)];
                    scaleYNorm += _Layer2[txIrisScaleYNorm + uint2(leftSide ? 0 : 8, 0)].xy;
                }

                look *= 0.1667;
                YCentroid *= 0.1667;
                XCentroid *= 0.1667;
                scaleYNorm *= 0.1667;

                // Reverse the rotation, scale, and translation
                pos.xyz = (pos.xyz - YCentroid) / scaleYNorm.y;
                pos.xyz = scaleYNorm.x * mul(look, pos.xyz) + XCentroid;

                return float4(pos, 0.0);
            }
            ENDCG
        }

        Pass
        {
            Name "Flip Right Eye"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "IrisInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            sampler2D  _Layer1;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                /*
                    ​Image of proportionally cropped left eye with
                    eyebrow (or horizontally flipped right eye) with a
                    25% margin on each side and size 64x64
                */
                const float2 uv = IN.globalTexcoord.xy;

                bool flipEye = abs(tex2D(_Layer1, 0..xx).x - 0.1337) < 0.001;
                float2 flippedUV = uv;
                flippedUV.y = flipEye ? flippedUV.y : 1.0 - flippedUV.y;

                float4 col = tex2D(_Layer1, flippedUV);
                if (all(uv.xy <= 0.01.xx))
                {
                    col = flipEye ? 0.1337.xxxx : 1..xxxx;
                }
                return col;
            }
            ENDCG
        }
    }
}


```

## Source file: FaceLandmarkDetection/Shaders/IrisInclude.cginc

{
  "path": "FaceLandmarkDetection/Shaders/IrisInclude.cginc",
  "blob_sha": "d8f1b0f4155e5d102b78ca7f1549423791859155",
  "sha256": "36506aee211d175ce75e10b7b2fd0952322c208e114ec0afef1b9b844d401d3f",
  "lines": 342,
  "bytes": 11496,
  "selection_score": 33,
  "keyword_line_counts": {
    "eye": 2,
    "iris": 13
  },
  "url": "https://github.com/SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL/blob/2502c0a21c08fd82900d855ff6acc095d9b95394/FaceLandmarkDetection/Shaders/IrisInclude.cginc",
  "function_signature_candidates": [
    {
      "line": 263,
      "text": "float testGen(uint3 pos)"
    },
    {
      "line": 275,
      "text": "float test(uint x, uint y, uint z)"
    },
    {
      "line": 283,
      "text": "float getLayerSP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)"
    },
    {
      "line": 296,
      "text": "float getLayerVP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)"
    },
    {
      "line": 308,
      "text": "float getLayerNP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)"
    },
    {
      "line": 319,
      "text": "float getConst(Texture2D<float> tex, uint4 off, uint kernSize, uint ID)"
    },
    {
      "line": 328,
      "text": "float getConst(Texture2D<float> tex, uint3 off, uint ID)"
    },
    {
      "line": 337,
      "text": "float getConst(Texture2D<float> tex, uint2 off, uint ID)"
    }
  ]
}

```text

﻿#ifndef __IRIS__
#define __IRIS__

#include "MLCommon.cginc"

// Texel locations for procustes analysis
#define txIrisRotation0          uint2(0, 0)
#define txIrisRotation1          uint2(1, 0)
#define txIrisRotation2          uint2(2, 0)
#define txIrisScaleYNorm         uint2(3, 0)
#define txIrisXCentroid          uint2(4, 0)
#define txIrisYCentroid          uint2(5, 0)

// The inital position of three points in the iris tracking
static const float3x3 eyeLInit =
{
    0.226, 0.794, 0.134,
    0.771, 0.753, -0.095,
    0.621, 0.176, -0.218
};

static const float3x3 eyeRInit = 
{
    0.283, 0.835, 0.201,
    0.802, 0.723, -0.122,
    0.524, 0.168, -0.282
};

// Baked weight positions

static const float4 irisWeight[227] = 
{
    411, 597, 9, 32,       // const0
    136, 597, 1, 64,       // const1
    132, 597, 1, 64,       // const2
    927, 641, 64, 1,       // const3
    116, 597, 1, 64,       // const4
    445, 597, 1, 32,       // const5
    800, 576, 32, 64,       // const6
    1013, 576, 9, 64,       // const7
    512, 384, 512, 64,       // const8
    81, 597, 9, 64,       // const9
    671, 641, 128, 1,       // const10
    102, 597, 1, 64,       // const11
    915, 653, 64, 1,       // const12
    114, 597, 1, 64,       // const13
    118, 597, 1, 64,       // const14
    120, 597, 1, 64,       // const15
    670, 642, 128, 1,       // const16
    27, 597, 9, 64,       // const17
    63, 597, 9, 64,       // const18
    192, 213, 64, 128,       // const19
    256, 213, 64, 128,       // const20
    798, 643, 128, 1,       // const21
    140, 597, 1, 64,       // const22
    851, 653, 64, 1,       // const23
    144, 597, 1, 64,       // const24
    864, 576, 32, 64,       // const25
    963, 0, 1, 128,       // const26
    210, 597, 64, 32,       // const27
    965, 0, 1, 128,       // const28
    787, 653, 64, 1,       // const29
    448, 213, 64, 128,       // const30
    723, 653, 64, 1,       // const31
    256, 533, 128, 64,       // const32
    995, 576, 9, 64,       // const33
    138, 597, 1, 64,       // const34
    670, 640, 1, 2,       // const35
    442, 597, 1, 32,       // const36
    974, 0, 1, 128,       // const37
    134, 597, 1, 64,       // const38
    131, 597, 1, 64,       // const39
    0, 405, 512, 64,       // const40
    788, 645, 128, 1,       // const41
    986, 576, 9, 64,       // const42
    851, 652, 64, 1,       // const43
    129, 597, 1, 64,       // const44
    723, 652, 64, 1,       // const45
    126, 597, 1, 64,       // const46
    128, 213, 64, 128,       // const47
    146, 597, 64, 32,       // const48
    64, 213, 64, 128,       // const49
    659, 652, 64, 1,       // const50
    980, 0, 1, 128,       // const51
    0, 597, 9, 64,       // const52
    979, 0, 1, 128,       // const53
    0, 213, 64, 128,       // const54
    438, 597, 1, 32,       // const55
    990, 642, 32, 1,       // const56
    897, 128, 64, 128,       // const57
    833, 128, 64, 128,       // const58
    640, 512, 128, 64,       // const59
    117, 597, 1, 64,       // const60
    851, 651, 64, 1,       // const61
    976, 0, 1, 128,       // const62
    659, 651, 64, 1,       // const63
    973, 0, 1, 128,       // const64
    972, 0, 1, 128,       // const65
    769, 128, 64, 128,       // const66
    113, 597, 1, 64,       // const67
    896, 576, 27, 64,       // const68
    512, 448, 256, 64,       // const69
    970, 0, 1, 128,       // const70
    1004, 576, 9, 64,       // const71
    112, 597, 1, 64,       // const72
    109, 597, 1, 64,       // const73
    108, 597, 1, 64,       // const74
    991, 640, 32, 1,       // const75
    950, 576, 9, 64,       // const76
    959, 576, 9, 64,       // const77
    420, 597, 9, 32,       // const78
    659, 640, 1, 8,       // const79
    658, 640, 1, 15,       // const80
    104, 597, 1, 64,       // const81
    141, 597, 1, 64,       // const82
    441, 597, 1, 32,       // const83
    641, 128, 64, 128,       // const84
    9, 597, 9, 64,       // const85
    274, 597, 64, 32,       // const86
    128, 469, 128, 64,       // const87
    100, 597, 1, 64,       // const88
    962, 0, 1, 128,       // const89
    915, 649, 64, 1,       // const90
    788, 647, 128, 1,       // const91
    577, 128, 64, 128,       // const92
    439, 597, 1, 32,       // const93
    90, 597, 9, 64,       // const94
    1022, 576, 1, 64,       // const95
    916, 647, 64, 1,       // const96
    916, 646, 64, 1,       // const97
    964, 0, 1, 128,       // const98
    833, 0, 64, 128,       // const99
    787, 650, 128, 1,       // const100
    769, 0, 64, 128,       // const101
    705, 0, 64, 128,       // const102
    45, 597, 9, 64,       // const103
    106, 597, 1, 64,       // const104
    926, 642, 64, 1,       // const105
    967, 0, 1, 128,       // const106
    968, 0, 1, 128,       // const107
    110, 597, 1, 64,       // const108
    897, 0, 64, 128,       // const109
    0, 533, 128, 64,       // const110
    971, 0, 1, 128,       // const111
    512, 576, 128, 64,       // const112
    659, 650, 128, 1,       // const113
    577, 0, 64, 128,       // const114
    768, 512, 128, 64,       // const115
    977, 0, 1, 128,       // const116
    119, 597, 1, 64,       // const117
    512, 320, 512, 64,       // const118
    338, 597, 64, 32,       // const119
    991, 641, 32, 1,       // const120
    123, 597, 1, 64,       // const121
    981, 0, 1, 128,       // const122
    125, 597, 1, 64,       // const123
    926, 643, 64, 1,       // const124
    916, 644, 64, 1,       // const125
    128, 597, 1, 64,       // const126
    512, 512, 128, 64,       // const127
    440, 597, 1, 32,       // const128
    977, 576, 9, 64,       // const129
    513, 128, 64, 128,       // const130
    133, 597, 1, 64,       // const131
    787, 649, 128, 1,       // const132
    787, 648, 128, 1,       // const133
    641, 0, 64, 128,       // const134
    18, 597, 9, 64,       // const135
    640, 576, 128, 64,       // const136
    139, 597, 1, 64,       // const137
    915, 648, 64, 1,       // const138
    961, 0, 1, 128,       // const139
    788, 646, 128, 1,       // const140
    143, 597, 1, 64,       // const141
    787, 652, 64, 1,       // const142
    978, 0, 1, 128,       // const143
    101, 597, 1, 64,       // const144
    915, 652, 64, 1,       // const145
    660, 645, 128, 1,       // const146
    402, 597, 9, 32,       // const147
    384, 213, 64, 128,       // const148
    256, 469, 128, 64,       // const149
    444, 597, 1, 32,       // const150
    659, 653, 64, 1,       // const151
    975, 0, 1, 128,       // const152
    660, 644, 128, 1,       // const153
    990, 643, 32, 1,       // const154
    670, 643, 128, 1,       // const155
    798, 642, 128, 1,       // const156
    105, 597, 1, 64,       // const157
    968, 576, 9, 64,       // const158
    896, 512, 128, 64,       // const159
    659, 654, 64, 1,       // const160
    320, 213, 64, 128,       // const161
    137, 597, 1, 64,       // const162
    799, 641, 128, 1,       // const163
    896, 448, 128, 64,       // const164
    932, 576, 9, 64,       // const165
    671, 640, 128, 1,       // const166
    122, 597, 1, 64,       // const167
    969, 0, 1, 128,       // const168
    384, 533, 128, 64,       // const169
    111, 597, 1, 64,       // const170
    788, 644, 128, 1,       // const171
    145, 597, 1, 64,       // const172
    941, 576, 9, 64,       // const173
    832, 576, 32, 64,       // const174
    799, 640, 128, 1,       // const175
    915, 650, 64, 1,       // const176
    0, 0, 512, 213,       // const177
    121, 597, 1, 64,       // const178
    768, 448, 128, 64,       // const179
    103, 597, 1, 64,       // const180
    429, 597, 9, 32,       // const181
    512, 0, 1, 213,       // const182
    130, 597, 1, 64,       // const183
    36, 597, 9, 64,       // const184
    443, 597, 1, 32,       // const185
    705, 128, 64, 128,       // const186
    142, 597, 1, 64,       // const187
    723, 651, 64, 1,       // const188
    0, 341, 512, 64,       // const189
    659, 648, 128, 1,       // const190
    135, 597, 1, 64,       // const191
    916, 645, 64, 1,       // const192
    128, 533, 128, 64,       // const193
    982, 0, 1, 128,       // const194
    72, 597, 9, 64,       // const195
    927, 640, 64, 1,       // const196
    513, 0, 64, 128,       // const197
    107, 597, 1, 64,       // const198
    384, 469, 128, 64,       // const199
    915, 651, 64, 1,       // const200
    659, 649, 128, 1,       // const201
    99, 597, 1, 64,       // const202
    512, 256, 512, 64,       // const203
    1023, 576, 1, 64,       // const204
    54, 597, 9, 64,       // const205
    966, 0, 1, 128,       // const206
    115, 597, 1, 64,       // const207
    660, 647, 128, 1,       // const208
    787, 651, 64, 1,       // const209
    146, 640, 512, 15,       // const210
    923, 576, 9, 64,       // const211
    124, 597, 1, 64,       // const212
    127, 597, 1, 64,       // const213
    660, 646, 128, 1,       // const214
    768, 576, 32, 64,       // const215
    0, 469, 128, 64,       // const216
    662, 640, 1, 4,       // const217
    661, 640, 1, 4,       // const218
    660, 640, 1, 4,       // const219
    667, 640, 1, 4,       // const220
    663, 640, 1, 4,       // const221
    664, 640, 1, 4,       // const222
    665, 640, 1, 4,       // const223
    666, 640, 1, 4,       // const224
    668, 640, 1, 4,       // const225
    669, 640, 1, 4,       // const226
};

/* 
float testGen(uint3 pos)
{
    float r;
    if (pos.z == 0)
        r = (pos.x / 63.0) * (pos.y / 63.0);
    else if (pos.z == 1)
        r = ((63.0 - pos.x) / 63.0) * (pos.y / 63.0);
    else
        r = (pos.x / 63.0) * ((63.0 - pos.y) / 63.0);
    return r;
}

float test(uint x, uint y, uint z)
{
    if (x >= 64 || y >= 64) return 0.0;
    return testGen(uint3(x, y, z));
}
*/

// Same padding
float getLayerSP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)
{
    if (any(off.xy == 0) || any(off.xy > lim)) return 0.0;
    off.xy -= 1;
    uint2 pos;
    pos.x = off.y + (off.z % mult.x) * lim;
    pos.y = off.x + (off.z / mult.x) * lim;
    float rtn = tex[pos];
    rtn = rtn > 100000.0 ? rtn - 10000000.0 : rtn;
    return rtn;
}

// Valid padding
float getLayerVP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)
{
    if (any(off.xy >= lim)) return 0.0;
    uint2 pos;
    pos.x = off.y + (off.z % mult.x) * lim;
    pos.y = off.x + (off.z / mult.x) * lim;
    float rtn = tex[pos];
    rtn = rtn > 100000.0 ? rtn - 10000000.0 : rtn;
    return rtn;
}

// No padding
float getLayerNP(Texture2D<float> tex, uint3 off, uint2 lim, uint2 mult)
{
    uint2 pos;
    pos.x = off.y + (off.z % mult.x) * lim;
    pos.y = off.x + (off.z / mult.x) * lim;
    float rtn = tex[pos];
    rtn = rtn > 100000.0 ? rtn - 10000000.0 : rtn;
    return rtn;
}

// Conv2D weights getter
float getConst(Texture2D<float> tex, uint4 off, uint kernSize, uint ID)
{
    uint2 pos;
    pos.x = off.w + off.z * kernSize + off.y * kernSize * kernSize;
    pos.y = off.x;
    return tex[irisWeight[ID].xy + pos];
}

// Depth-wise Conv2D weights getter
float getConst(Texture2D<float> tex, uint3 off, uint ID)
{
    uint2 pos;
    pos.x = off.z + off.y * 3;
    pos.y = off.x;
    return tex[irisWeight[ID].xy + pos];
}

// Bias/Activation slope getter
float getConst(Texture2D<float> tex, uint2 off, uint ID)
{
    return tex[irisWeight[ID].xy + off];
}

#endif

```

## Source file: FaceLandmarkDetection/Shaders/FaceMeshModel.shader

{
  "path": "FaceLandmarkDetection/Shaders/FaceMeshModel.shader",
  "blob_sha": "bbf01d571adfce87bee6bb58b11e958fd9cb6dd7",
  "sha256": "d66f4e4551fd9c26053b0de53a48ee1f2b32ff06c413072d2ab3f647be5e668d",
  "lines": 677,
  "bytes": 28970,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "deform": 1,
    "shader": 12,
    "rig": 1,
    "face": 5,
    "lip": 1
  },
  "url": "https://github.com/SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL/blob/2502c0a21c08fd82900d855ff6acc095d9b95394/FaceLandmarkDetection/Shaders/FaceMeshModel.shader",
  "function_signature_candidates": [
    {
      "line": 46,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 114,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 166,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 223,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 282,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 352,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 404,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 456,
      "text": "float frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 516,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    },
    {
      "line": 638,
      "text": "float4 frag(v2f_customrendertexture IN) : COLOR"
    }
  ]
}

```text

﻿Shader "FaceLandmarkDetection/FaceMesh Model"
{
    Properties
    {
        [Header(Weights)]
        _Weights ("Baked Weights", 2D) = "black" {}
        _WeightID ("Weight ID", Int) = 0
        _BiasID ("Bias ID", Int) = 0
        _MultiplierID ("Multiplier ID", Int) = 0
        [Header(Previous Layers)]
        _CamIn ("Camera Input", 2D) = "black" {}
        _Layer1 ("Layer Input 1", 2D) = "black" {}
        _Layer2 ("Layer Input 2", 2D) = "black" {}
        _Layer12WH ("[Width, Height, Width, Height]", Vector) =  (1, 1, 1, 1)
        [Header(Current Layer)]
        _WHDM ("[Width, Height, Loop Depth, Null]", Vector) = (1, 1, 0, 0)
    }

    SubShader
    {
        Tags { "Queue"="Geometry" "ForceNoShadowCasting"="True" "IgnoreProjector"="True" }
        Lighting Off
        Fog { Mode Off }
        Blend One Zero
        ZTest Never

        Pass
        {
            Name "Camera Input"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            sampler2D        _CamIn;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);
                
                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                // stride = 2
                const uint i0 = i * 2, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = j * 2, j1 = j0 + 1, j2 = j0 + 2;

                float cl = 0.0;
                // kernel
                [unroll(3)]
                for (uint l = 0; l < _WHDM.z; l++) {
                    cl +=
                        getCameraVP(_CamIn, uint3(i0, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(0, 0, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i0, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(0, 1, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i0, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(0, 2, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(1, 0, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(1, 1, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i1, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(1, 2, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j0, l), _Layer12WH.xy) * getConst(_Weights, uint4(2, 0, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j1, l), _Layer12WH.xy) * getConst(_Weights, uint4(2, 1, l, k), _WHDM.z, _WeightID) +
                        getCameraVP(_CamIn, uint3(i2, j2, l), _Layer12WH.xy) * getConst(_Weights, uint4(2, 2, l, k), _WHDM.z, _WeightID);

                        // test(i0, j0, l) * getConst(_Weights, uint4(0, 0, l, k), _WHDM.z, _WeightID) +
                        // test(i0, j1, l) * getConst(_Weights, uint4(0, 1, l, k), _WHDM.z, _WeightID) +
                        // test(i0, j2, l) * getConst(_Weights, uint4(0, 2, l, k), _WHDM.z, _WeightID) +
                        // test(i1, j0, l) * getConst(_Weights, uint4(1, 0, l, k), _WHDM.z, _WeightID) +
                        // test(i1, j1, l) * getConst(_Weights, uint4(1, 1, l, k), _WHDM.z, _WeightID) +
                        // test(i1, j2, l) * getConst(_Weights, uint4(1, 2, l, k), _WHDM.z, _WeightID) +
                        // test(i2, j0, l) * getConst(_Weights, uint4(2, 0, l, k), _WHDM.z, _WeightID) +
                        // test(i2, j1, l) * getConst(_Weights, uint4(2, 1, l, k), _WHDM.z, _WeightID) +
                        // test(i2, j2, l) * getConst(_Weights, uint4(2, 2, l, k), _WHDM.z, _WeightID);
                }
                // bias
                cl = cl + getConst(_Weights, k, _BiasID, false);

                // activation
                cl = relu(cl) + relu(-cl) * getConst(_Weights, k, _MultiplierID, false);

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Depth-wise Conv2D Stride 1"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);

                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                // stride = 1
                const uint i0 = i, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = j, j1 = j0 + 1, j2 = j0 + 2;

                uint2 PLD;
                _Layer1.GetDimensions(PLD.x, PLD.y);
                const uint2 mult = PLD / _Layer12WH.xy;

                float cl =
                    getLayerSP(_Layer1, uint3(i0, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 0, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i0, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 1, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i0, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 2, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 0, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 1, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i1, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 2, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 0, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 1, k), _WeightID) +
                    getLayerSP(_Layer1, uint3(i2, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 2, k), _WeightID);

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Depth-wise Conv2D Stride 2"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);

                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                // stride = 2
                const uint i0 = i * 2, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = j * 2, j1 = j0 + 1, j2 = j0 + 2;

                uint2 PLD;
                _Layer1.GetDimensions(PLD.x, PLD.y);
                const uint2 mult = PLD / _Layer12WH.xy;

                float cl =
                    getLayerVP(_Layer1, uint3(i0, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 0, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i0, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 1, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i0, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 2, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i1, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 0, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i1, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 1, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i1, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 2, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i2, j0, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 0, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i2, j1, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 1, k), _WeightID) +
                    getLayerVP(_Layer1, uint3(i2, j2, k), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 2, k), _WeightID);

                //if (i == 1 && j == 2 && k == 32) buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Batch Normalization"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            Texture2D<float> _Layer2;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);

                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                uint2 PLD2;
                _Layer2.GetDimensions(PLD2.x, PLD2.y);
                const uint2 mult2 = PLD2 / _Layer12WH.zw;

                float cl = 0.0;
                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++) {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }
                // bias + add another layer
                cl = cl + getConst(_Weights, k, _BiasID, false) +
                    getLayerNP(_Layer2, uint3(i, j, k), _Layer12WH.zw, mult2);

                // activation
                cl = relu(cl) + relu(-cl) * getConst(_Weights, k, _MultiplierID, false);

                //if (i == 2 && j == 1 && k == 31) buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Max Pool Batch Normalization"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            Texture2D<float> _Layer2;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);

                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;
                
                // stride = 2
                const uint i0 = i * 2, i1 = i0 + 1;
                const uint j0 = j * 2, j1 = j0 + 1;

                float cl = 0.0;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                uint2 PLD2;
                _Layer2.GetDimensions(PLD2.x, PLD2.y);
                const uint2 mult2 = PLD2 / _Layer12WH.zw;

                if (k < _WHDM.z) {
                    cl = max4(getLayerNP(_Layer1, uint3(i0, j0, k), _Layer12WH.xy, mult1),
                            getLayerNP(_Layer1, uint3(i0, j1, k), _Layer12WH.xy, mult1),
                            getLayerNP(_Layer1, uint3(i1, j0, k), _Layer12WH.xy, mult1),
                            getLayerNP(_Layer1, uint3(i1, j1, k), _Layer12WH.xy, mult1));
                }

                float bn = 0.0;
                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++) {
                    bn += getLayerNP(_Layer2, uint3(i, j, l), _Layer12WH.zw, mult2) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }
                // bias + add another layer
                cl = cl + bn + getConst(_Weights, k, _BiasID, false);

                // activation
                cl = relu(cl) + relu(-cl) * getConst(_Weights, k, _MultiplierID, false);

                //if (i == 1 && j == 2 && k == 31) buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "1x1 Conv2D"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;
            uint             _MultiplierID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);
                
                const uint widthMult = floor(_CustomRenderTextureWidth / _WHDM.x);

                const uint i = px.y % _WHDM.y;
                const uint j = px.x % _WHDM.x;
                const uint k = (px.x / _WHDM.x) + (px.y / _WHDM.y) * widthMult;

                uint2 PLD1;
                _Layer1.GetDimensions(PLD1.x, PLD1.y);
                const uint2 mult1 = PLD1 / _Layer12WH.xy;

                float cl = 0.0;
                // 1x1 kernel
                for (uint l = 0; l < _WHDM.z; l++) {
                    cl += getLayerNP(_Layer1, uint3(i, j, l), _Layer12WH.xy, mult1) *
                        getConst(_Weights, uint2(l, k), _WeightID);
                }
                // bias + add another layer
                cl = cl + getConst(_Weights, k, _BiasID, false);

                // activation
                cl = relu(cl) + relu(-cl) * getConst(_Weights, k, _MultiplierID, false);

                //if (i == 2 && j == 1 && k == 31) buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Face Flag Output"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                // stride = 0
                const uint i0 = 0, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = 0, j1 = j0 + 1, j2 = j0 + 2;

                float cl = 0.0;

                uint2 PLD;
                _Layer1.GetDimensions(PLD.x, PLD.y);
                const uint2 mult = PLD / _Layer12WH.xy;

                for (uint l = 0; l < _WHDM.z; l++) {
                    cl +=
                        getLayerNP(_Layer1, uint3(i0, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 0, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i0, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 1, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i0, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(0, 2, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 0, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 1, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(1, 2, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 0, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 1, l), _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint3(2, 2, l), _WeightID);
                }

                cl = sigmoid(cl + getConst(_Weights, 0, _BiasID, false));

                //buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        Pass
        {
            Name "Face Mesh Output"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Weights;
            Texture2D<float> _Layer1;
            uint4            _WHDM;
            uint4            _Layer12WH;
            uint             _WeightID;
            uint             _BiasID;

            float frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                const uint k = px.x + px.y * uint(_CustomRenderTextureWidth);

                // stride = 0
                const uint i0 = 0, i1 = i0 + 1, i2 = i0 + 2;
                const uint j0 = 0, j1 = j0 + 1, j2 = j0 + 2;

                float cl = 0.0;

                uint2 PLD;
                _Layer1.GetDimensions(PLD.x, PLD.y);
                const uint2 mult = PLD / _Layer12WH.xy;

                for (uint l = 0; l < _WHDM.z; l++) {
                    cl +=
                        getLayerNP(_Layer1, uint3(i0, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(0, 0, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i0, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(0, 1, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i0, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(0, 2, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(1, 0, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(1, 1, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i1, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(1, 2, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j0, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(2, 0, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j1, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(2, 1, l, k), _WHDM.z, _WeightID) +
                        getLayerNP(_Layer1, uint3(i2, j2, l), _Layer12WH.xy, mult) * getConst(_Weights, uint4(2, 2, l, k), _WHDM.z, _WeightID);
                }

                cl = cl + getConst(_Weights, k, _BiasID, true);

                //if (k == 1010) buffer[0] = cl.xxxx;

                return cl;
            }
            ENDCG
        }

        /*
            Python/MATLAB implementation of procrustes
            https://stackoverflow.com/questions/18925181/procrustes-analysis-with-numpy
        
            We want to undo the rotations to best measure the face mesh deformations
        */
        Pass
        {
            Name "Procrustes Analysis"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #include "svd_hlsl.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float> _Layer1;
            Texture2D<float3> _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                float4 col = 0;

                // Between eyes
                // Z + 23.0 to make the normal direction face forward
                const float3 p6 = { _Layer1[uint2(18, 0)], _Layer1[uint2(19, 0)], _Layer1[uint2(20, 0)] + 23.0};

                // Right Head
                const float3 p103 = { _Layer1[uint2(36, 7)], _Layer1[uint2(37, 7)], _Layer1[uint2(38, 7)] };

                // Left Head
                const float3 p332 = { _Layer1[uint2(21, 25)], _Layer1[uint2(22, 25)], _Layer1[uint2(23, 25)] };

                if (px.y == 0)
                {
                    // Procrustes Analysis
                    // Find the rotation/translation/scale to map Y to X
                    float3x3 X = fmInitPos;
                    float3x3 Y = { p103, p6, p332 };

                    const float3 muX = findCenteroid(X);
                    const float3 muY = findCenteroid(Y);

                    float3x3 X0;
                    float3x3 Y0;
                    uint i, j;
                    for (i = 0; i < 3; i++)
                    {
                        X0[i] = X[i] - muX;
                        Y0[i] = Y[i] - muY;
                    }

                    // squared error
                    float ssX = 0.0;
                    float ssY = 0.0;
                    [unroll]
                    for (i = 0; i < 3; i++)
                    {
                        [unroll]
                        for (j = 0; j < 3; j++)
                        {
                            ssX += X0[i][j] * X0[i][j];
                            ssY += Y0[i][j] * Y0[i][j];
                        }
                    }

                    // centred Frobenius norm
                    const float normX = sqrt(ssX);
                    const float normY = sqrt(ssY);

                    // scale to equal (unit) norm
                    [unroll]
                    for (i = 0; i < 3; i++)
                    {
                        [unroll]
                        for (j = 0; j < 3; j++)
                        {
                            X0[i][j] /= normX;
                            Y0[i][j] /= normY;
                        }
                    }

                    // Singular value decomposition of a 3x3 matrix
                    float3x3 A = mul(transpose(X0), Y0);
                    float3x3 U;
                    float3 D;
                    float3x3 Vt;
                    GetSVD3D(A, U, D, Vt);
                    
                    // solve optimum rotation matrix of Y
                    // why flipped? I don't know but it works
                    float3x3 V = transpose(Vt);
                    float3x3 T = mul(transpose(U), V);

                    bool have_reflection = determinant(T) < 0.0;
                    if (have_reflection != false)
                    {
                        V[0][2] = -V[0][2];
                        V[1][2] = -V[1][2];
                        V[2][2] = -V[2][2];
                        D[2] = -D[2];
                        T = mul(transpose(U), V);
                    }

                    const float traceTA = D[0] + D[1] + D[2];

                    StoreValue(txRotation0, float4(T[0], 0.0), col, px);
                    StoreValue(txRotation1, float4(T[1], 0.0), col, px);
                    StoreValue(txRotation2, float4(T[2], 0.0), col, px);
                    StoreValue(txScaleYNorm, float4(traceTA * normX, normY, 0.0, 0.0), col, px);
                    StoreValue(txXCentroid, float4(muX, 0.0), col, px);
                    StoreValue(txYCentroid, float4(muY, 0.0), col, px);
                }
                else
                {
                    // Keep history for smoothing
                    col.rgb = _Layer2[uint2(px.x, px.y - 1)];
                }

                return col;
            }
            ENDCG
        }

        Pass
        {
            Name "Reverse Rotation"
            CGPROGRAM
            #include "UnityCustomRenderTexture.cginc"
            #include "LandmarkInclude.cginc"
            #pragma vertex CustomRenderTextureVertexShader
            #pragma fragment frag
            #pragma target 5.0

            //RWStructuredBuffer<float4> buffer : register(u1);
            Texture2D<float>  _Layer1;
            Texture2D<float3> _Layer2;

            float4 frag(v2f_customrendertexture IN) : COLOR
            {
                const uint2 px = IN.globalTexcoord.xy *
                    float2(_CustomRenderTextureWidth, _CustomRenderTextureHeight);

                float3 pos;
                pos.x = _Layer1[uint2(px.x * 3, px.y)];
                pos.y = _Layer1[uint2(px.x * 3 + 1, px.y)];
                pos.z = _Layer1[uint2(px.x * 3 + 2, px.y)];

                float3 YCentroid = 0.0;
                float3 XCentroid = 0.0;
                float2 scaleYNorm = 0.0;
                float3x3 look = 0.0;

                for (uint i = 0; i < 6; i++)
                {
                    look[0] += _Layer2[uint2(0, i)];
                    look[1] += _Layer2[uint2(1, i)];
                    look[2] += _Layer2[uint2(2, i)];
                    YCentroid += _Layer2[uint2(5, i)];
                    XCentroid += _Layer2[uint2(4, i)];
                    scaleYNorm += _Layer2[uint2(3, i)].xy;
                }

                look *= 0.1667;
                YCentroid *= 0.1667;
                XCentroid *= 0.1667;
                scaleYNorm *= 0.1667;

                // Reverse the rotation, scale, and translation
                pos.xyz = (pos.xyz - YCentroid) / scaleYNorm.y;
                pos.xyz = scaleYNorm.x * mul(look, pos.xyz) + XCentroid;

                return float4(pos, 0.0);
            }
            ENDCG
        }
    }
}

```

## Source file: FaceLandmarkDetection/Demos/Bricks Game/Shaders/ShaderGames.cginc

{
  "path": "FaceLandmarkDetection/Demos/Bricks Game/Shaders/ShaderGames.cginc",
  "blob_sha": "929e56372e17f5af0c28a20f686a4639206c291a",
  "sha256": "d1c5464d698f1e30130810ae480677887955ca1b9973af74e2aea0840f18fc12",
  "lines": 145,
  "bytes": 4591,
  "selection_score": 28,
  "keyword_line_counts": {
    "shader": 3
  },
  "url": "https://github.com/SCRN-VRC/3D-Face-Landmark-in-UnityCG-HLSL/blob/2502c0a21c08fd82900d855ff6acc095d9b95394/FaceLandmarkDetection/Demos/Bricks%20Game/Shaders/ShaderGames.cginc",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "float Hash( sampler2D iChannel0, in float3 x )"
    },
    {
      "line": 15,
      "text": "float4 LoadValue( in Texture2D<float4> tex, in int2 re )"
    },
    {
      "line": 20,
      "text": "void StoreValue( in int2 txPos, in float4 value, inout float4 col, in int2 fragPos )"
    },
    {
      "line": 25,
      "text": "float3 TitleHexagon(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 34,
      "text": "float3 YouWin(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 43,
      "text": "float3 Score(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 52,
      "text": "float3 Ready(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 61,
      "text": "float3 Lives(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 70,
      "text": "float3 GaveOver(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 79,
      "text": "float3 CPU(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 88,
      "text": "float3 Player(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 97,
      "text": "float3 FireToJoin(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {"
    },
    {
      "line": 109,
      "text": "float PrintInt( in float2 uv, in uint value, in int maxDigits )"
    },
    {
      "line": 128,
      "text": "float PrintInt( in float2 uv, in uint value, in int maxDigits )"
    }
  ]
}

```text

﻿#ifndef GAMES_INC
#define GAMES_INC

float Hash( sampler2D iChannel0, in float3 x )
{
    float3 p = floor(x);
    float3 f = frac(x);
    f = f*f*(3.0-2.0*f);
    
    float2 uv = (p.xy+float2(37.0,17.0)*p.z) + f.xy;
    float2 rg = tex2Dlod( iChannel0, float4((uv+ 0.5)/256.0, 0.0, -100.0) ).yx;
    return lerp( rg.x, rg.y, f.z );
}

float4 LoadValue( in Texture2D<float4> tex, in int2 re )
{
    return tex.Load(int3(re, 0));
}

void StoreValue( in int2 txPos, in float4 value, inout float4 col, in int2 fragPos )
{
    col = all(fragPos == txPos) ? value : col;
}

float3 TitleHexagon(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 7 && coord.x < 64) {
        coord.y += 42;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 YouWin(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 6 && coord.x < 42) {
        coord.y += 36;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 Score(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 28) {
        coord.y += 31;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 Ready(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 29) {
        coord.y += 26;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 Lives(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 24) {
        coord.y += 21;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 GaveOver(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 6 && coord.x < 62) {
        coord.y += 15;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 CPU(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 17) {
        coord.y += 10;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 Player(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 33) {
        coord.y += 5;
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

float3 FireToJoin(in Texture2D<float4> text, in uint2 pos, in uint2 fragCoord) {
    uint2 coord = uint2(fragCoord.x - pos.x, fragCoord.y - pos.y);
    if (coord.y < 5 && coord.x < 56) {
        return text.Load(uint3(coord, 0)).xyz;
    }
    else return 0..xxx;
}

#if SHADER_API_D3D11
// Digit data by P_Malin (https://www.shadertoy.com/view/4sf3RN)
static const int font[10] = {0x75557, 0x22222, 0x74717, 0x74747, 0x11574, 0x71747, 0x71757, 0x74444, 0x75757, 0x75747};
static const uint powers[5] = {1, 10, 100, 1000, 10000};
float PrintInt( in float2 uv, in uint value, in int maxDigits )
{
    if( abs(uv.y-0.5)<0.5 )
    {
        int iu = int(floor(uv.x));
        if( iu>=0 && iu<maxDigits )
        {
            int n = (value/powers[maxDigits-iu-1]) % 10;
            uv.x = frac(uv.x);//(uv.x-float(iu)); 
            int2 p = int2(floor(uv*float2(4.0,5.0)));
            return float((font[n] >> (p.x+p.y*4)) & 1);
        }
        else return 0.0;
    } else return 0.0;
}
#else
// Digit data by P_Malin (https://www.shadertoy.com/view/4sf3RN)
static const int font[10] = {0x75557, 0x22222, 0x74717, 0x74747, 0x11574, 0x71747, 0x71757, 0x74444, 0x75757, 0x75747};
static const uint powers[5] = {1, 10, 100, 1000, 10000};
float PrintInt( in float2 uv, in uint value, in int maxDigits )
{
    if( abs(uv.y-0.5)<0.5 )
    {
        int iu = int(floor(uv.x));
        if( iu>=0 && iu<maxDigits )
        {
            uint n = (value/powers[maxDigits-iu-1]) % 10;
            uv.x = frac(uv.x);//(uv.x-float(iu)); 
            int2 p = int2(floor(uv*float2(4.0,5.0)));
            return float(round((font[n] / pow(2, p.x+p.y*5 - p.y)) % 1));
        }
        else return 0.0;
    } else return 0.0;
}
#endif

#endif

```

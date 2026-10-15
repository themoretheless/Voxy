# Facial rendering source scan: Unity-Technologies/facial-ar-remote

Repository: https://github.com/Unity-Technologies/facial-ar-remote

Pinned source revision: fb83cdaa6af4bbf3a2df742010a2cfe143a0a9cb

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md", "LICENSE.md.meta"]

Topic paths: {"blendshape": ["Editor/BlendShapeOverrideDrawer.cs", "Editor/BlendShapeOverrideDrawer.cs.meta", "Runtime/BlendShapeIndexData.cs", "Runtime/BlendShapeIndexData.cs.meta", "Runtime/BlendShapeOverride.cs", "Runtime/BlendShapeOverride.cs.meta", "Runtime/BlendShapeUtils.cs", "Runtime/BlendShapeUtils.cs.meta", "Runtime/BlendShapesController.cs", "Runtime/BlendShapesController.cs.meta", "Samples~/Sloth Example/Scenes/SlothBlendShapes.unity", "Samples~/Sloth Example/Scenes/SlothBlendShapes.unity.meta", "Samples~/Sloth Example/SlothCharacter/Meshs/sloth_head_blendshapes.fbx", "Samples~/Sloth Example/SlothCharacter/Meshs/sloth_head_blendshapes.fbx.meta", "Samples~/Sloth Example/SlothCharacter/sloth_head_blendshapes.prefab", "Samples~/Sloth Example/SlothCharacter/sloth_head_blendshapes.prefab.meta"], "shader": ["Runtime/Shaders.meta", "Runtime/Shaders/ARFaceShader.shader", "Runtime/Shaders/ARFaceShader.shader.meta"], "rig": ["Editor/CharacterRigControllerEditor.cs", "Editor/CharacterRigControllerEditor.cs.meta", "Runtime/CharacterRigController.cs", "Runtime/CharacterRigController.cs.meta"], "face": ["Runtime/Interfaces.meta", "Runtime/Interfaces/IStreamReader.cs", "Runtime/Interfaces/IStreamReader.cs.meta", "Runtime/Interfaces/IStreamRecorder.cs", "Runtime/Interfaces/IStreamRecorder.cs.meta", "Runtime/Interfaces/IStreamSettings.cs", "Runtime/Interfaces/IStreamSettings.cs.meta", "Runtime/Interfaces/IStreamSource.cs", "Runtime/Interfaces/IStreamSource.cs.meta", "Runtime/Interfaces/IUsesStreamReader.cs", "Runtime/Interfaces/IUsesStreamReader.cs.meta", "Runtime/Prefabs/AR Face.prefab", "Runtime/Prefabs/AR Face.prefab.meta", "Runtime/Prefabs/FaceLostOverlay.mat", "Runtime/Prefabs/FaceLostOverlay.mat.meta", "Runtime/Shaders/ARFaceShader.shader", "Runtime/Shaders/ARFaceShader.shader.meta", "Samples~/Sloth Example/Materials/ARKitFaceMesh.mat", "Samples~/Sloth Example/Materials/ARKitFaceMesh.mat.meta"], "lip": ["Editor/ClipBaker.cs", "Editor/ClipBaker.cs.meta"]}

## Source file: Runtime/Shaders/ARFaceShader.shader

{
  "path": "Runtime/Shaders/ARFaceShader.shader",
  "blob_sha": "05429ac86c742879ec14b56a539a00582c3fc780",
  "sha256": "8d59565464a3441f9da34d81c0385d11fc0de25cde229ae8de7cd0a330a3b909",
  "lines": 57,
  "bytes": 1786,
  "selection_score": 42,
  "keyword_line_counts": {
    "shader": 4,
    "face": 3
  },
  "url": "https://github.com/Unity-Technologies/facial-ar-remote/blob/fb83cdaa6af4bbf3a2df742010a2cfe143a0a9cb/Runtime/Shaders/ARFaceShader.shader",
  "function_signature_candidates": [
    {
      "line": 42,
      "text": "void surf (Input IN, inout SurfaceOutputStandard o) {"
    }
  ]
}

```text

﻿Shader "ARKit/AR Face" {
	Properties {
		_Color ("Color", Color) = (1,1,1,1)
		_MainTex ("Albedo (RGB)", 2D) = "white" {}
		_Glossiness ("Smoothness", Range(0,1)) = 0.5
		_Metallic ("Metallic", Range(0,1)) = 0.0
		_RimColor ("Rim Color", Color) = (0.26,0.19,0.16,0.0)
        _RimPower ("Rim Power", Range(0.5,8.0)) = 3.0
	}
	SubShader {
		Tags { "Queue"="Transparent" "RenderType"="Transparent" "IgnoreProjector"="True" }
        Blend OneMinusDstColor One
		LOD 200

		CGPROGRAM
		// Physically based Standard lighting model, and enable shadows on all light types
		#pragma surface surf Standard fullforwardshadows

		// Use shader model 3.0 target, to get nicer looking lighting
		#pragma target 3.0

		sampler2D _MainTex;

		struct Input {
			float2 uv_MainTex;
			float3 viewDir;
		};

		half _Glossiness;
		half _Metallic;
		fixed4 _Color;
		float4 _RimColor;
        float _RimPower;

		// Add instancing support for this shader. You need to check 'Enable Instancing' on materials that use the shader.
		// See https://docs.unity3d.com/Manual/GPUInstancing.html for more information about instancing.
		// #pragma instancing_options assumeuniformscaling
		UNITY_INSTANCING_BUFFER_START(Props)
			// put more per-instance properties here
		UNITY_INSTANCING_BUFFER_END(Props)

		void surf (Input IN, inout SurfaceOutputStandard o) {
			// Albedo comes from a texture tinted by color
			fixed4 c = tex2D (_MainTex, IN.uv_MainTex) * _Color;
			half rim = 1.0 - saturate(dot (normalize(IN.viewDir), o.Normal));
			rim = pow(rim, _RimPower);
			o.Albedo = c.rgb * rim;
			// Metallic and smoothness come from slider variables
			o.Metallic = _Metallic;
			o.Smoothness = _Glossiness;
			o.Emission = rim * _RimColor.rgb;
			o.Alpha = c.a * rim;
		}
		ENDCG
	}
	FallBack "Diffuse"
}


```

## Source file: Editor/ClipBaker.cs

{
  "path": "Editor/ClipBaker.cs",
  "blob_sha": "25c87676cde3d4bbb1719ec9520be0a151de87a0",
  "sha256": "c68768ccf339a037bed8d8caeb6c5d2a6153313c8333d4d0889681c53e0b60de",
  "lines": 309,
  "bytes": 12572,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 10,
    "blendshape": 23,
    "rig": 20,
    "lip": 28
  },
  "url": "https://github.com/Unity-Technologies/facial-ar-remote/blob/fb83cdaa6af4bbf3a2df742010a2cfe143a0a9cb/Editor/ClipBaker.cs",
  "function_signature_candidates": [
    {
      "line": 60,
      "text": "void StartClipBaker(Transform transform)"
    },
    {
      "line": 156,
      "text": "public void StopBake()"
    },
    {
      "line": 162,
      "text": "public void ApplyAnimationCurves()"
    },
    {
      "line": 192,
      "text": "public void BakeClipLoop()"
    },
    {
      "line": 198,
      "text": "bool BakeClipLoopInternal()"
    },
    {
      "line": 228,
      "text": "void KeyFrame(float time)"
    }
  ]
}

```text

﻿using System;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;
using UnityObject = UnityEngine.Object;

namespace Unity.Labs.FacialRemote
{
    public class ClipBaker
    {
        const int k_FramesPerStep = 16;
        const string k_BlendShapeProp = "blendShape.{0}";
        const string k_RotationX = "localRotation.x";
        const string k_RotationY = "localRotation.y";
        const string k_RotationZ = "localRotation.z";
        const string k_RotationW = "localRotation.w";
        static readonly string[] k_RotParams =
        {
            k_RotationX,
            k_RotationY,
            k_RotationZ,
            k_RotationW
        };

        AnimationClip m_Clip;
        StreamReader m_StreamReader;
        PlaybackStream m_PlaybackStream;
        CharacterRigController m_CharacterRigController;
        BlendShapesController m_BlendShapesController;

        readonly Dictionary<UnityObject, Dictionary<string, AnimationClipCurveData>> m_AnimationCurves =
            new Dictionary<UnityObject, Dictionary<string, AnimationClipCurveData>>();

        readonly List<AnimationClipCurveData> m_AnimationClipCurveData = new List<AnimationClipCurveData>();

        readonly float[] m_FrameTime = new float[1];
        float m_FirstFrameTime;

        string m_FilePath;

        public int currentFrame { get; private set; }
        public int frameCount { get; private set; }
        public bool baking { get; private set; }

        public ClipBaker(AnimationClip clip, StreamReader streamReader, PlaybackStream playbackStream,
            BlendShapesController blendShapesController, CharacterRigController characterRigController, string filePath)
        {
            m_Clip = clip;
            m_StreamReader = streamReader;
            m_PlaybackStream = playbackStream;
            m_BlendShapesController = blendShapesController;
            m_CharacterRigController = characterRigController;
            m_FilePath = filePath;

            StartClipBaker(m_BlendShapesController != null
                ? m_BlendShapesController.transform
                : m_CharacterRigController.transform);
        }

        void StartClipBaker(Transform transform)
        {
            var playbackBuffer = m_PlaybackStream.activePlaybackBuffer;
            if (playbackBuffer == null)
                return;

            if (m_CharacterRigController == null)
                Debug.LogWarning("No Character Rig Controller Found! Will not be able to bake Character Bone Animations.");

            if (m_BlendShapesController == null)
                Debug.LogWarning("No Blend Shape Controller Found! Will not be able to bake Character Blend Shape Animations.");

            m_AnimationClipCurveData.Clear();

            baking = true;

            if (m_BlendShapesController != null)
            {
                m_BlendShapesController.UpdateBlendShapeIndices(playbackBuffer);

                // Get curve data for blend shapes
                foreach (var skinnedMeshRenderer in m_BlendShapesController.skinnedMeshRenderers)
                {
                    if (skinnedMeshRenderer == null || m_AnimationCurves.ContainsKey(skinnedMeshRenderer)) // Skip duplicates
                        continue;

                    var path = AnimationUtility.CalculateTransformPath(skinnedMeshRenderer.transform, transform);
                    path = path.Replace(string.Format("{0}/", skinnedMeshRenderer.transform), "");

                    var mesh = skinnedMeshRenderer.sharedMesh;
                    var count = mesh.blendShapeCount;

                    var animationCurves = new Dictionary<string, AnimationClipCurveData>();
                    for (var i = 0; i < count; i++)
                    {
                        var shapeName = mesh.GetBlendShapeName(i);
                        var prop = string.Format(k_BlendShapeProp, shapeName);
                        var curve = new AnimationCurve();
                        var curveData = new AnimationClipCurveData
                        {
                            path = path,
                            curve = curve,
                            propertyName = prop,
                            type = skinnedMeshRenderer.GetType()
                        };

                        m_AnimationClipCurveData.Add(curveData);
                        animationCurves.Add(prop, curveData);
                    }

                    m_AnimationCurves.Add(skinnedMeshRenderer, animationCurves);
                }
            }

            if (m_CharacterRigController != null)
            {
                m_CharacterRigController.UpdateBlendShapeIndices(playbackBuffer);

                m_CharacterRigController.SetupCharacterRigController();

                // Get curve data for bone transforms
                var transformType = typeof(Transform);
                foreach (var bone in m_CharacterRigController.animatedBones)
                {
                    if (bone == null || m_AnimationCurves.ContainsKey(bone)) // Skip duplicates
                        continue;

                    var path = AnimationUtility.CalculateTransformPath(bone, transform);
                    var animationCurves = new Dictionary<string, AnimationClipCurveData>();
                    foreach (var prop in k_RotParams)
                    {
                        var curve = new AnimationCurve();
                        var curveData = new AnimationClipCurveData
                        {
                            path = path,
                            curve = curve,
                            propertyName = prop,
                            type = transformType
                        };

                        m_AnimationClipCurveData.Add(curveData);
                        animationCurves.Add(prop, curveData);
                    }

                    m_AnimationCurves.Add(bone, animationCurves);
                }
            }

            currentFrame = 0;
            var recordStream = playbackBuffer.recordStream;
            frameCount = recordStream.Length / playbackBuffer.bufferSize;

            Buffer.BlockCopy(recordStream, playbackBuffer.FrameTimeOffset, m_FrameTime, 0, playbackBuffer.FrameTimeSize);
            m_FirstFrameTime = m_FrameTime[0];
        }

        public void StopBake()
        {
            baking = false;
            m_CharacterRigController.ResetBonePoses();
        }

        public void ApplyAnimationCurves()
        {
            foreach (var curveData in m_AnimationClipCurveData)
            {
                var path = curveData.path;
                var propertyName = curveData.propertyName;
                var type = curveData.type;
                var curve = curveData.curve;
                m_Clip.SetCurve(path, type, propertyName, curve);
                AnimationUtility.SetEditorCurve(m_Clip, EditorCurveBinding.FloatCurve(path, type, propertyName), curve);
            }

            var fileClip = AssetDatabase.LoadAssetAtPath(m_FilePath, typeof(AnimationClip));
            if (fileClip == null)
            {
                AssetDatabase.CreateAsset(m_Clip, m_FilePath);
            }
            else
            {
                // This overrides the reference in the asset file
                // ReSharper disable once RedundantAssignment
                fileClip = m_Clip;
                AssetDatabase.SaveAssets();
                AssetDatabase.ImportAsset(m_FilePath);
            }

            StopBake();
            Debug.Log("End Bake");
        }

        public void BakeClipLoop()
        {
            if (!BakeClipLoopInternal())
                StopBake();
        }

        bool BakeClipLoopInternal()
        {
            if (frameCount < 1)
                return false;

            var streamSettings = m_StreamReader.streamSource.streamSettings;
            var bufferSize = streamSettings.bufferSize;
            var frameTimeOffset = streamSettings.FrameTimeOffset;
            var frameTimeSize = streamSettings.FrameTimeSize;
            var recordStream = m_PlaybackStream.activePlaybackBuffer.recordStream;
            for (var i = 0; i < k_FramesPerStep && currentFrame < frameCount; i++, currentFrame++)
            {
                Buffer.BlockCopy(recordStream, currentFrame * bufferSize + frameTimeOffset, m_FrameTime, 0, frameTimeSize);

                // Run normal playback to record transform keyframes
                m_PlaybackStream.PlayBackLoop();
                m_PlaybackStream.UpdateCurrentFrameBuffer(true);

                if (m_BlendShapesController != null)
                    m_BlendShapesController.InterpolateBlendShapes(true);

                if (m_CharacterRigController != null)
                    m_CharacterRigController.InterpolateBlendShapes(true);

                KeyFrame(m_FrameTime[0] - m_FirstFrameTime);
            }

            return true;
        }

        void KeyFrame(float time)
        {
            // Key blend shapes
            if (m_BlendShapesController != null)
            {
                foreach (var skinnedMeshRenderer in m_BlendShapesController.skinnedMeshRenderers)
                {
                    Dictionary<string, AnimationClipCurveData> animationCurves;
                    if (m_AnimationCurves.TryGetValue(skinnedMeshRenderer, out animationCurves))
                    {
                        BlendShapeIndexData[] shapeIndices;
                        if (m_BlendShapesController.blendShapeIndices.TryGetValue(skinnedMeshRenderer, out shapeIndices))
                        {
                            var length = shapeIndices.Length;
                            for (var i = 0; i < length; i++)
                            {
                                var datum = shapeIndices[i];
                                var index = datum.index;
                                if (index < 0)
                                    continue;

                                var curve = animationCurves[string.Format(k_BlendShapeProp, datum.name)].curve;
                                curve.AddKey(time, m_BlendShapesController.blendShapesScaled[index]);
                            }
                        }
                    }
                }
            }

            // Key bone rotation
            if (m_CharacterRigController)
            {
                for (var i = 0; i < m_CharacterRigController.animatedBones.Length; i++)
                {
                    // Use head
                    if (i == 0 && !m_CharacterRigController.driveHead)
                        continue;

                    // Use neck
                    if (i == 1 && !m_CharacterRigController.driveNeck)
                        continue;

                    // Use Eyes
                    if (i == 2 && !m_CharacterRigController.driveEyes)
                        continue;

                    if (i == 3 && !m_CharacterRigController.driveEyes)
                        continue;

                    var bone = m_CharacterRigController.animatedBones[i];
                    Dictionary<string, AnimationClipCurveData> animationCurves;
                    if (m_AnimationCurves.TryGetValue(bone, out animationCurves))
                    {
                        foreach (var datum in animationCurves)
                        {
                            var curveData = datum.Value;
                            var prop = curveData.propertyName;
                            var curve = curveData.curve;
                            switch (prop) {
                                case k_RotationX:
                                    curve.AddKey(time, bone.localRotation.x);
                                    break;
                                case k_RotationY:
                                    curve.AddKey(time, bone.localRotation.y);
                                    break;
                                case k_RotationZ:
                                    curve.AddKey(time, bone.localRotation.z);
                                    break;
                                case k_RotationW:
                                    curve.AddKey(time, bone.localRotation.w);
                                    break;
                                default:
                                    Debug.LogErrorFormat("Fell through on {0} : {1}", datum.Key, prop);
                                    break;
                            }
                        }
                    }
                }
            }
        }
    }
}


```

## Source file: Runtime/BlendShapeUtils.cs

{
  "path": "Runtime/BlendShapeUtils.cs",
  "blob_sha": "49860572a13b8e3a9cfbaf61718298751f98bcdb",
  "sha256": "5883694b6d7a4d093515f93adc8df8a4eea4c3c59ae5be3d3afaaa2e4bff2484",
  "lines": 226,
  "bytes": 13652,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 42,
    "blendshape": 55,
    "jaw": 12,
    "mouth": 69,
    "tongue": 3,
    "rig": 60,
    "face": 2
  },
  "url": "https://github.com/Unity-Technologies/facial-ar-remote/blob/fb83cdaa6af4bbf3a2df742010a2cfe143a0a9cb/Runtime/BlendShapeUtils.cs",
  "function_signature_candidates": [
    {
      "line": 192,
      "text": "public static int GetLocationIndex(this IStreamSettings streamSettings, string location)"
    },
    {
      "line": 202,
      "text": "public static void ArrayToPose(float[] poseArray, ref Pose pose)"
    },
    {
      "line": 213,
      "text": "public static void PoseToArray(Pose pose, float[] poseArray)"
    }
  ]
}

```text

﻿using System;
using UnityEngine;

#if INCLUDE_ARKIT_FACE_PLUGIN
using UnityEngine.XR.ARKit;
#endif

namespace Unity.Labs.FacialRemote
{
    public static class BlendShapeUtils
    {
        public const int PoseFloatCount = 7;
        public const int PoseSize = sizeof(float) * PoseFloatCount;

#if INCLUDE_ARKIT_FACE_PLUGIN
        public static readonly string  BrowDownLeft        =   ARKitBlendShapeLocation.BrowDownLeft.ToString();
        public static readonly string  BrowDownRight       =   ARKitBlendShapeLocation.BrowDownRight.ToString();
        public static readonly string  BrowInnerUp         =   ARKitBlendShapeLocation.BrowInnerUp.ToString();
        public static readonly string  BrowOuterUpLeft     =   ARKitBlendShapeLocation.BrowOuterUpLeft.ToString();
        public static readonly string  BrowOuterUpRight    =   ARKitBlendShapeLocation.BrowOuterUpRight.ToString();
        public static readonly string  CheekPuff           =   ARKitBlendShapeLocation.CheekPuff.ToString();
        public static readonly string  CheekSquintLeft     =   ARKitBlendShapeLocation.CheekSquintLeft.ToString();
        public static readonly string  CheekSquintRight    =   ARKitBlendShapeLocation.CheekSquintRight.ToString();
        public static readonly string  EyeBlinkLeft        =   ARKitBlendShapeLocation.EyeBlinkLeft.ToString();
        public static readonly string  EyeBlinkRight       =   ARKitBlendShapeLocation.EyeBlinkRight.ToString();
        public static readonly string  EyeLookDownLeft     =   ARKitBlendShapeLocation.EyeLookDownLeft.ToString();
        public static readonly string  EyeLookDownRight    =   ARKitBlendShapeLocation.EyeLookDownRight.ToString();
        public static readonly string  EyeLookInLeft       =   ARKitBlendShapeLocation.EyeLookInLeft.ToString();
        public static readonly string  EyeLookInRight      =   ARKitBlendShapeLocation.EyeLookInRight.ToString();
        public static readonly string  EyeLookOutLeft      =   ARKitBlendShapeLocation.EyeLookOutLeft.ToString();
        public static readonly string  EyeLookOutRight     =   ARKitBlendShapeLocation.EyeLookOutRight.ToString();
        public static readonly string  EyeLookUpLeft       =   ARKitBlendShapeLocation.EyeLookUpLeft.ToString();
        public static readonly string  EyeLookUpRight      =   ARKitBlendShapeLocation.EyeLookUpRight.ToString();
        public static readonly string  EyeSquintLeft       =   ARKitBlendShapeLocation.EyeSquintLeft.ToString();
        public static readonly string  EyeSquintRight      =   ARKitBlendShapeLocation.EyeSquintRight.ToString();
        public static readonly string  EyeWideLeft         =   ARKitBlendShapeLocation.EyeWideLeft.ToString();
        public static readonly string  EyeWideRight        =   ARKitBlendShapeLocation.EyeWideRight.ToString();
        public static readonly string  JawForward          =   ARKitBlendShapeLocation.JawForward.ToString();
        public static readonly string  JawLeft             =   ARKitBlendShapeLocation.JawLeft.ToString();
        public static readonly string  JawOpen             =   ARKitBlendShapeLocation.JawOpen.ToString();
        public static readonly string  JawRight            =   ARKitBlendShapeLocation.JawRight.ToString();
        public static readonly string  MouthClose          =   ARKitBlendShapeLocation.MouthClose.ToString();
        public static readonly string  MouthDimpleLeft     =   ARKitBlendShapeLocation.MouthDimpleLeft.ToString();
        public static readonly string  MouthDimpleRight    =   ARKitBlendShapeLocation.MouthDimpleRight.ToString();
        public static readonly string  MouthFrownLeft      =   ARKitBlendShapeLocation.MouthFrownLeft.ToString();
        public static readonly string  MouthFrownRight     =   ARKitBlendShapeLocation.MouthFrownRight.ToString();
        public static readonly string  MouthFunnel         =   ARKitBlendShapeLocation.MouthFunnel.ToString();
        public static readonly string  MouthLeft           =   ARKitBlendShapeLocation.MouthLeft.ToString();
        public static readonly string  MouthLowerDownLeft  =   ARKitBlendShapeLocation.MouthLowerDownLeft.ToString();
        public static readonly string  MouthLowerDownRight =   ARKitBlendShapeLocation.MouthLowerDownRight.ToString();
        public static readonly string  MouthPressLeft      =   ARKitBlendShapeLocation.MouthPressLeft.ToString();
        public static readonly string  MouthPressRight     =   ARKitBlendShapeLocation.MouthPressRight.ToString();
        public static readonly string  MouthPucker         =   ARKitBlendShapeLocation.MouthPucker.ToString();
        public static readonly string  MouthRight          =   ARKitBlendShapeLocation.MouthRight.ToString();
        public static readonly string  MouthRollLower      =   ARKitBlendShapeLocation.MouthRollLower.ToString();
        public static readonly string  MouthRollUpper      =   ARKitBlendShapeLocation.MouthRollUpper.ToString();
        public static readonly string  MouthShrugLower     =   ARKitBlendShapeLocation.MouthShrugLower.ToString();
        public static readonly string  MouthShrugUpper     =   ARKitBlendShapeLocation.MouthShrugUpper.ToString();
        public static readonly string  MouthSmileLeft      =   ARKitBlendShapeLocation.MouthSmileLeft.ToString();
        public static readonly string  MouthSmileRight     =   ARKitBlendShapeLocation.MouthSmileRight.ToString();
        public static readonly string  MouthStretchLeft    =   ARKitBlendShapeLocation.MouthStretchLeft.ToString();
        public static readonly string  MouthStretchRight   =   ARKitBlendShapeLocation.MouthStretchRight.ToString();
        public static readonly string  MouthUpperUpLeft    =   ARKitBlendShapeLocation.MouthUpperUpLeft.ToString();
        public static readonly string  MouthUpperUpRight   =   ARKitBlendShapeLocation.MouthUpperUpRight.ToString();
        public static readonly string  NoseSneerLeft       =   ARKitBlendShapeLocation.NoseSneerLeft.ToString();
        public static readonly string  NoseSneerRight      =   ARKitBlendShapeLocation.NoseSneerRight.ToString();
        public static readonly string  TongueOut           =   ARKitBlendShapeLocation.TongueOut.ToString();
#else
        public const string  BrowDownLeft        =   "BrowDownLeft";
        public const string  BrowDownRight       =   "BrowDownRight";
        public const string  BrowInnerUp         =   "BrowInnerUp";
        public const string  BrowOuterUpLeft     =   "BrowOuterUpLeft";
        public const string  BrowOuterUpRight    =   "BrowOuterUpRight";
        public const string  CheekPuff           =   "CheekPuff";
        public const string  CheekSquintLeft     =   "CheekSquintLeft";
        public const string  CheekSquintRight    =   "CheekSquintRight";
        public const string  EyeBlinkLeft        =   "EyeBlinkLeft";
        public const string  EyeBlinkRight       =   "EyeBlinkRight";
        public const string  EyeLookDownLeft     =   "EyeLookDownLeft";
        public const string  EyeLookDownRight    =   "EyeLookDownRight";
        public const string  EyeLookInLeft       =   "EyeLookInLeft";
        public const string  EyeLookInRight      =   "EyeLookInRight";
        public const string  EyeLookOutLeft      =   "EyeLookOutLeft";
        public const string  EyeLookOutRight     =   "EyeLookOutRight";
        public const string  EyeLookUpLeft       =   "EyeLookUpLeft";
        public const string  EyeLookUpRight      =   "EyeLookUpRight";
        public const string  EyeSquintLeft       =   "EyeSquintLeft";
        public const string  EyeSquintRight      =   "EyeSquintRight";
        public const string  EyeWideLeft         =   "EyeWideLeft";
        public const string  EyeWideRight        =   "EyeWideRight";
        public const string  JawForward          =   "JawForward";
        public const string  JawLeft             =   "JawLeft";
        public const string  JawOpen             =   "JawOpen";
        public const string  JawRight            =   "JawRight";
        public const string  MouthClose          =   "MouthClose";
        public const string  MouthDimpleLeft     =   "MouthDimpleLeft";
        public const string  MouthDimpleRight    =   "MouthDimpleRight";
        public const string  MouthFrownLeft      =   "MouthFrownLeft";
        public const string  MouthFrownRight     =   "MouthFrownRight";
        public const string  MouthFunnel         =   "MouthFunnel";
        public const string  MouthLeft           =   "MouthLeft";
        public const string  MouthLowerDownLeft  =   "MouthLowerDownLeft";
        public const string  MouthLowerDownRight =   "MouthLowerDownRight";
        public const string  MouthPressLeft      =   "MouthPressLeft";
        public const string  MouthPressRight     =   "MouthPressRight";
        public const string  MouthPucker         =   "MouthPucker";
        public const string  MouthRight          =   "MouthRight";
        public const string  MouthRollLower      =   "MouthRollLower";
        public const string  MouthRollUpper      =   "MouthRollUpper";
        public const string  MouthShrugLower     =   "MouthShrugLower";
        public const string  MouthShrugUpper     =   "MouthShrugUpper";
        public const string  MouthSmileLeft      =   "MouthSmileLeft";
        public const string  MouthSmileRight     =   "MouthSmileRight";
        public const string  MouthStretchLeft    =   "MouthStretchLeft";
        public const string  MouthStretchRight   =   "MouthStretchRight";
        public const string  MouthUpperUpLeft    =   "MouthUpperUpLeft";
        public const string  MouthUpperUpRight   =   "MouthUpperUpRight";
        public const string  NoseSneerLeft       =   "NoseSneerLeft";
        public const string  NoseSneerRight      =   "NoseSneerRight";
        public const string  TongueOut           =   "TongueOut";
#endif

        /// <summary>
        /// Array of the blend shape locations supported by the unity ARKit plugin.
        /// </summary>
        /// <remarks>
        /// ARKIT_2_0 is a custom scripting define symbol and will need to be enabled in
        /// 'PlayerSettings>platform>Scripting Define Symbols' for use in build
        /// </remarks>
        public static readonly string[] Locations =
        {
            BrowDownLeft,
            BrowDownRight,
            BrowInnerUp,
            BrowOuterUpLeft,
            BrowOuterUpRight,
            CheekPuff,
            CheekSquintLeft,
            CheekSquintRight,
            EyeBlinkLeft,
            EyeBlinkRight,
            EyeLookDownLeft,
            EyeLookDownRight,
            EyeLookInLeft,
            EyeLookInRight,
            EyeLookOutLeft,
            EyeLookOutRight,
            EyeLookUpLeft,
            EyeLookUpRight,
            EyeSquintLeft,
            EyeSquintRight,
            EyeWideLeft,
            EyeWideRight,
            JawForward,
            JawLeft,
            JawOpen,
            JawRight,
            MouthClose,
            MouthDimpleLeft,
            MouthDimpleRight,
            MouthFrownLeft,
            MouthFrownRight,
            MouthFunnel,
            MouthLeft,
            MouthLowerDownLeft,
            MouthLowerDownRight,
            MouthPressLeft,
            MouthPressRight,
            MouthPucker,
            MouthRight,
            MouthRollLower,
            MouthRollUpper,
            MouthShrugLower,
            MouthShrugUpper,
            MouthSmileLeft,
            MouthSmileRight,
            MouthStretchLeft,
            MouthStretchRight,
            MouthUpperUpLeft,
            MouthUpperUpRight,
            NoseSneerLeft,
            NoseSneerRight,
            TongueOut,
        };

        /// <summary>
        /// Used for mapping the the blendshape locations this returns the index of the string in the Locations array.
        /// </summary>
        /// <param name="streamSettings">Stream Setting that contains the Locations array.</param>
        /// <param name="location">Name of blendshape location you want to find.</param>
        /// <returns>Index of string in Locations array.</returns>
        public static int GetLocationIndex(this IStreamSettings streamSettings, string location)
        {
            return Array.IndexOf(streamSettings.locations, location);
        }

        /// <summary>
        /// Takes a correctly formatted array and returns a pose from that array.
        /// </summary>
        /// <param name="poseArray">Array of floats that encodes a pose.</param>
        /// <param name="pose">Pose encoded in the float array.</param>
        public static void ArrayToPose(float[] poseArray, ref Pose pose)
        {
            pose.position = new Vector3(poseArray[0], poseArray[1], poseArray[2]);
            pose.rotation = new Quaternion(poseArray[3], poseArray[4], poseArray[5], poseArray[6]);
        }

        /// <summary>
        /// Takes a pose and encodes the values to the given correctly formatted pose array.
        /// </summary>
        /// <param name="pose">Pose to encode in the float array.</param>
        /// <param name="poseArray">Float array to that the pose is encoded to.</param>
        public static void PoseToArray(Pose pose, float[] poseArray)
        {
            var position = pose.position;
            var rotation = pose.rotation;
            poseArray[0] = position.x;
            poseArray[1] = position.y;
            poseArray[2] = position.z;
            poseArray[3] = rotation.x;
            poseArray[4] = rotation.y;
            poseArray[5] = rotation.z;
            poseArray[6] = rotation.w;
        }
    }
}


```

## Source file: Runtime/BlendShapeOverride.cs

{
  "path": "Runtime/BlendShapeOverride.cs",
  "blob_sha": "fc3cee8462f25368dac4534f802701ba329becad",
  "sha256": "d9157849f171bd2fbdee8f9413ce239ec3914f4b4f3ce0d91673e905b7e5a523",
  "lines": 62,
  "bytes": 2053,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 13
  },
  "url": "https://github.com/Unity-Technologies/facial-ar-remote/blob/fb83cdaa6af4bbf3a2df742010a2cfe143a0a9cb/Runtime/BlendShapeOverride.cs",
  "function_signature_candidates": []
}

```text

﻿using System;
using UnityEngine;

namespace Unity.Labs.FacialRemote
{
    /// <summary>
    /// Stores the override values for an individual blend shape.
    /// </summary>
    [Serializable]
    public class BlendShapeOverride
    {
#pragma warning disable 649
        [SerializeField]
        [Tooltip("Enables the use of these override values on a blend shape.")]
        bool m_UseOverride;

        [SerializeField]
        [Tooltip("Blend shape name to be overridden.")]
        string m_Name;

        [Range(0f, 1)]
        [SerializeField]
        [Tooltip("Smoothing to apply to blend shape values coming from the stream reader.")]
        float m_BlendShapeSmoothing = 0.1f;

        [Range(0, 0.1f)]
        [SerializeField]
        [Tooltip("Min threshold of change to register as a new blend shape value.")]
        float m_BlendShapeThreshold = 0.01f;

        [Range(-100, 100)]
        [SerializeField]
        [Tooltip("Offsets the zero value of the blend shape.")]
        float m_BlendShapeOffset;

        [Range(0, 200f)]
        [SerializeField]
        [Tooltip("Scaling coefficient applied to the blend shape values from the stream reader.")]
        float m_BlendShapeCoefficient = 120f;

        [Range(0, 100)]
        [SerializeField]
        [Tooltip("Max value a scaled blend shape can reach.")]
        float m_BlendShapeMax = 100f;
#pragma warning restore 649

        public bool useOverride { get { return m_UseOverride; } }
        public string name { get { return m_Name; } }
        public float blendShapeSmoothing { get { return m_BlendShapeSmoothing; } }
        public float blendShapeThreshold { get { return m_BlendShapeThreshold; } }
        public float blendShapeOffset { get { return m_BlendShapeOffset; } }
        public float blendShapeCoefficient { get { return m_BlendShapeCoefficient; } }
        public float blendShapeMax { get { return m_BlendShapeMax; } }

        BlendShapeOverride(){}

        public BlendShapeOverride(string name)
        {
            m_Name = name;
        }
    }
}


```

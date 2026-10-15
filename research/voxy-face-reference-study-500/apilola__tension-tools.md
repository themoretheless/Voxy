# Facial rendering source scan: apilola/tension-tools

Repository: https://github.com/apilola/tension-tools

Pinned source revision: 2a1e0f2cb814caccf3123a30ec31072fc4ce8349

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"skin": ["Packages/com.ap.tension-tools/Samples~/Noodle/Skin.mat", "Packages/com.ap.tension-tools/Samples~/Noodle/Skin.mat.meta", "Packages/com.ap.tension-tools/Samples~/Noodle/Skin.shadergraph", "Packages/com.ap.tension-tools/Samples~/Noodle/Skin.shadergraph.meta"], "subsurface": ["Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl", "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph.meta"], "shader": ["Packages/com.ap.tension-tools/Runtime/Shaders.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/Graphs.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/Graphs/TensionVisualizer.shadergraph", "Packages/com.ap.tension-tools/Runtime/Shaders/Graphs/TensionVisualizer.shadergraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/Nodes.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/Nodes/SampleTension.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/Nodes/SampleTension.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/Nodes/SimpleSampleTension.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/Nodes/SimpleSampleTension.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl", "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl", "Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Graphs.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Graphs/TensionVisualizerURP.shadergraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Graphs/TensionVisualizerURP.shadergraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/AO Smoothness Metalic.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/AO Smoothness Metalic.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/UnpackNormal.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/UnpackNormal.shadersubgraph.meta", "Packages/com.ap.tension-tools/Samples~/Noodle/Skin.shadergraph", "Packages/com.ap.tension-tools/Samples~/Noodle/Skin.shadergraph.meta"], "face": ["Documentation/Images/FaceShowcase.jpg", "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl", "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurface.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SimpleSubsurfaceGI.shadersubgraph.meta", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph", "Packages/com.ap.tension-tools/Runtime/Shaders/URP/Nodes/SubsurfaceScattering.shadersubgraph.meta"]}

## Prior selected-function review (partial)

{"files": ["Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl"], "finding": "Per-vertex adjacency edges compare rest/current length, sum compression/stretch into separate channels and apply power/limits."}

This prior partial review may target a different pinned revision; compare hashes before reusing findings.

## Source file: Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl

{
  "path": "Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl",
  "blob_sha": "afa70c968143630dc084b1c63e9b61c708a6c913",
  "sha256": "a8e2171ce0ad756a99e6796e8537172ce96a718258c5b3cc5de733d39e514642",
  "lines": 40,
  "bytes": 2002,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 11,
    "face": 11
  },
  "url": "https://github.com/apilola/tension-tools/blob/2a1e0f2cb814caccf3123a30ec31072fc4ce8349/Packages/com.ap.tension-tools/Runtime/Shaders/SubsurfaceHelpers.hlsl",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "void GetSubsurfaceScatteringLight(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,"
    },
    {
      "line": 17,
      "text": "void GetSubsurfaceScatteringLight_float(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,"
    },
    {
      "line": 29,
      "text": "void GetSubsurfaceScatteringLight_half(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,"
    }
  ]
}

```text


#ifndef SUBSURFACE_HELPERS_INCLUDED
#define SUBSURFACE_HELPERS_INCLUDED

void GetSubsurfaceScatteringLight(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,
	float attenuation, float3 thickness, float3 indirectLight, float3 subsurfaceColour, float distortion, float power, float intensity, float ambient, out float3 light)
{
	float3 vLTLight = lightDirection + normalDirection * distortion; // Distortion
	float3 fLTDot = pow(saturate(dot(viewDirection, -vLTLight)), power)
		* intensity * 1.0 / 3.14159265359f;

	light =  lerp(1, attenuation, float(lightExists))
		* (fLTDot + ambient) * thickness
		* (lightColor + indirectLight) * subsurfaceColour;
}

void GetSubsurfaceScatteringLight_float(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,
	float attenuation, float3 thickness, float3 indirectLight, float3 subsurfaceColour, float distortion, float power, float intensity, float ambient, out float3 light)
{
	float3 vLTLight = lightDirection + normalDirection * distortion; // Distortion
	float3 fLTDot = pow(saturate(dot(viewDirection, -vLTLight)), power)
		* intensity * 1.0 / 3.14159265359f;

	light = lerp(1, attenuation, float(lightExists))
		* (fLTDot + ambient) * thickness
		* (lightColor + indirectLight) * subsurfaceColour;
}

void GetSubsurfaceScatteringLight_half(bool lightExists, float3 lightColor, float3 lightDirection, float3 normalDirection, float3 viewDirection,
	float attenuation, float3 thickness, float3 indirectLight, float3 subsurfaceColour, float distortion, float power, float intensity, float ambient, out half3 light)
{
	float3 vLTLight = lightDirection + normalDirection * distortion; // Distortion
	float3 fLTDot = pow(saturate(dot(viewDirection, -vLTLight)), power)
		* intensity * 1.0 / 3.14159265359f;

	light = lerp(1, attenuation, float(lightExists))
		* (fLTDot + ambient) * thickness
		* (lightColor + indirectLight) * subsurfaceColour;
}
#endif


```

## Source file: Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl

{
  "path": "Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl",
  "blob_sha": "c8ac38b6029a37c29d4adcec3ce133a1dcac0d66",
  "sha256": "b4c50f815f92ad89d6789faf2f42cc83415d84f5e5732ec63b0316bc0e997775",
  "lines": 73,
  "bytes": 1726,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/apilola/tension-tools/blob/2a1e0f2cb814caccf3123a30ec31072fc4ce8349/Packages/com.ap.tension-tools/Runtime/Shaders/TensionHelper.hlsl",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "float3 GetPosition(uint vertexID)"
    },
    {
      "line": 23,
      "text": "float3 GetRawEdgeDelta(uint edgeID)"
    },
    {
      "line": 39,
      "text": "void SampleTension(uint vertexID, float3 position, out float2 tension)"
    },
    {
      "line": 63,
      "text": "void SampleTension_float(uint vertexID, float3 position, out float2 tension)"
    },
    {
      "line": 68,
      "text": "void SampleTension_half(uint vertexID, float3 position, out float2 tension)"
    }
  ]
}

```text


#ifndef TENSION_HELPERS_INCLUDED
#define TENSION_HELPERS_INCLUDED

StructuredBuffer<int> _EdgeBuffer;
StructuredBuffer<float3> _RawEdgeDeltaBuffer;
ByteAddressBuffer _VertexBuffer;
int _VertexBufferStride;
int _VertexCount;
float _SquashIntensity;
float _SquashLimit;
float _SquashPower;
float _StretchIntensity;
float _StretchLimit;
float _StretchPower;
float3 _Scale;

float3 GetPosition(uint vertexID)
{
	return asfloat(_VertexBuffer.Load3((vertexID * _VertexBufferStride) << 2));// [vertexID * _VertexBufferStride] ;
}

float3 GetRawEdgeDelta(uint edgeID)
{
	return _RawEdgeDeltaBuffer[edgeID];
}

int2 GetRange(uint vertexID)
{
	int2 range;
	range.y = _EdgeBuffer[vertexID];
	if (vertexID == 0)
		range.x = 0;
	else
		range.x = _EdgeBuffer[vertexID - 1];
	return range;
}

void SampleTension(uint vertexID, float3 position, out float2 tension)
{
	int2 range = GetRange(vertexID);
	tension = float2(0, 0);
	for (int i = range.x; i < range.y; i++)
	{
		float3 rawEdgeDelta = GetRawEdgeDelta(i) * _Scale;
		float d0 = length(rawEdgeDelta);
		float3 neighborPos = GetPosition(_EdgeBuffer[i + _VertexCount]);
		float d1 = length((neighborPos - position));
		float delta = (d0 - d1);
		if (delta > 0)
		{
			tension.r += abs(delta) / d0;
		}
		else
		{
		    tension.g += (abs(delta) / d0);
		}
	}
	tension.r = min(pow(tension.r, _SquashPower) * _SquashIntensity, _SquashLimit);
	tension.g = min(pow(tension.g, _StretchPower) * _StretchIntensity, _StretchLimit);
}

void SampleTension_float(uint vertexID, float3 position, out float2 tension)
{
	SampleTension(vertexID, position, tension);
}

void SampleTension_half(uint vertexID, float3 position, out float2 tension)
{
	SampleTension(vertexID, position, tension);
}

#endif


```

## Source file: Assets/TutorialInfo/Scripts/Readme.cs

{
  "path": "Assets/TutorialInfo/Scripts/Readme.cs",
  "blob_sha": "48843dbfd7d27e9a51e5244733d010368fc5ad19",
  "sha256": "f6134a8e6818ba252d3af8102fb0f2fa1c01f11b56d120032ac3a9aa1a91dbc1",
  "lines": 14,
  "bytes": 272,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/apilola/tension-tools/blob/2a1e0f2cb814caccf3123a30ec31072fc4ce8349/Assets/TutorialInfo/Scripts/Readme.cs",
  "function_signature_candidates": []
}

```text

﻿using System;
using UnityEngine;

public class Readme : ScriptableObject {
	public Texture2D icon;
	public string title;
	public Section[] sections;
	public bool loadedLayout;
	
	[Serializable]
	public class Section {
		public string heading, text, linkText, url;
	}
}


```

## Source file: Assets/Scripts/SimpleCameraController.cs

{
  "path": "Assets/Scripts/SimpleCameraController.cs",
  "blob_sha": "f9e8f85516773e97e1ef4846b1506aa7923eed55",
  "sha256": "8f3a7a0e32f4878fb66dea5d238a4c31d7797391ffa1bb16cf68c8439e616c7e",
  "lines": 294,
  "bytes": 10139,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 16
  },
  "url": "https://github.com/apilola/tension-tools/blob/2a1e0f2cb814caccf3123a30ec31072fc4ce8349/Assets/Scripts/SimpleCameraController.cs",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "public void SetFromTransform(Transform t)"
    },
    {
      "line": 30,
      "text": "public void Translate(Vector3 translation)"
    },
    {
      "line": 39,
      "text": "public void LerpTowards(CameraState target, float positionLerpPct, float rotationLerpPct)"
    },
    {
      "line": 50,
      "text": "public void UpdateTransform(Transform t)"
    },
    {
      "line": 84,
      "text": "void Start()"
    },
    {
      "line": 119,
      "text": "void OnEnable()"
    },
    {
      "line": 162,
      "text": "void Update()"
    },
    {
      "line": 224,
      "text": "float GetBoostFactor()"
    },
    {
      "line": 242,
      "text": "bool IsBoostPressed()"
    },
    {
      "line": 254,
      "text": "bool IsEscapePressed()"
    },
    {
      "line": 263,
      "text": "bool IsCameraRotationAllowed()"
    },
    {
      "line": 274,
      "text": "bool IsRightMouseButtonDown()"
    },
    {
      "line": 283,
      "text": "bool IsRightMouseButtonUp()"
    }
  ]
}

```text

﻿#if ENABLE_INPUT_SYSTEM 
using UnityEngine.InputSystem;
#endif

using UnityEngine;

namespace UnityTemplateProjects
{
    public class SimpleCameraController : MonoBehaviour
    {
        class CameraState
        {
            public float yaw;
            public float pitch;
            public float roll;
            public float x;
            public float y;
            public float z;

            public void SetFromTransform(Transform t)
            {
                pitch = t.eulerAngles.x;
                yaw = t.eulerAngles.y;
                roll = t.eulerAngles.z;
                x = t.position.x;
                y = t.position.y;
                z = t.position.z;
            }

            public void Translate(Vector3 translation)
            {
                Vector3 rotatedTranslation = Quaternion.Euler(pitch, yaw, roll) * translation;

                x += rotatedTranslation.x;
                y += rotatedTranslation.y;
                z += rotatedTranslation.z;
            }

            public void LerpTowards(CameraState target, float positionLerpPct, float rotationLerpPct)
            {
                yaw = Mathf.Lerp(yaw, target.yaw, rotationLerpPct);
                pitch = Mathf.Lerp(pitch, target.pitch, rotationLerpPct);
                roll = Mathf.Lerp(roll, target.roll, rotationLerpPct);
                
                x = Mathf.Lerp(x, target.x, positionLerpPct);
                y = Mathf.Lerp(y, target.y, positionLerpPct);
                z = Mathf.Lerp(z, target.z, positionLerpPct);
            }

            public void UpdateTransform(Transform t)
            {
                t.eulerAngles = new Vector3(pitch, yaw, roll);
                t.position = new Vector3(x, y, z);
            }
        }
        
        CameraState m_TargetCameraState = new CameraState();
        CameraState m_InterpolatingCameraState = new CameraState();

        [Header("Movement Settings")]
        [Tooltip("Exponential boost factor on translation, controllable by mouse wheel.")]
        public float boost = 3.5f;

        [Tooltip("Time it takes to interpolate camera position 99% of the way to the target."), Range(0.001f, 1f)]
        public float positionLerpTime = 0.2f;

        [Header("Rotation Settings")]
        [Tooltip("X = Change in mouse position.\nY = Multiplicative factor for camera rotation.")]
        public AnimationCurve mouseSensitivityCurve = new AnimationCurve(new Keyframe(0f, 0.5f, 0f, 5f), new Keyframe(1f, 2.5f, 0f, 0f));

        [Tooltip("Time it takes to interpolate camera rotation 99% of the way to the target."), Range(0.001f, 1f)]
        public float rotationLerpTime = 0.01f;

        [Tooltip("Whether or not to invert our Y axis for mouse input to rotation.")]
        public bool invertY = false;

#if ENABLE_INPUT_SYSTEM
        InputAction movementAction;
        InputAction verticalMovementAction;
        InputAction lookAction;
        InputAction boostFactorAction;
        bool        mouseRightButtonPressed;

        void Start()
        {
            var map = new InputActionMap("Simple Camera Controller");

            lookAction = map.AddAction("look", binding: "<Mouse>/delta");
            movementAction = map.AddAction("move", binding: "<Gamepad>/leftStick");
            verticalMovementAction = map.AddAction("Vertical Movement");
            boostFactorAction = map.AddAction("Boost Factor", binding: "<Mouse>/scroll");

            lookAction.AddBinding("<Gamepad>/rightStick").WithProcessor("scaleVector2(x=15, y=15)");
            movementAction.AddCompositeBinding("Dpad")
                .With("Up", "<Keyboard>/w")
                .With("Up", "<Keyboard>/upArrow")
                .With("Down", "<Keyboard>/s")
                .With("Down", "<Keyboard>/downArrow")
                .With("Left", "<Keyboard>/a")
                .With("Left", "<Keyboard>/leftArrow")
                .With("Right", "<Keyboard>/d")
                .With("Right", "<Keyboard>/rightArrow");
            verticalMovementAction.AddCompositeBinding("Dpad")
                .With("Up", "<Keyboard>/pageUp")
                .With("Down", "<Keyboard>/pageDown")
                .With("Up", "<Keyboard>/e")
                .With("Down", "<Keyboard>/q")
                .With("Up", "<Gamepad>/rightshoulder")
                .With("Down", "<Gamepad>/leftshoulder");
            boostFactorAction.AddBinding("<Gamepad>/Dpad").WithProcessor("scaleVector2(x=1, y=4)");

            movementAction.Enable();
            lookAction.Enable();
            verticalMovementAction.Enable();
            boostFactorAction.Enable();
        }
#endif

        void OnEnable()
        {
            m_TargetCameraState.SetFromTransform(transform);
            m_InterpolatingCameraState.SetFromTransform(transform);
        }

        Vector3 GetInputTranslationDirection()
        {
            Vector3 direction = Vector3.zero;
#if ENABLE_INPUT_SYSTEM
            var moveDelta = movementAction.ReadValue<Vector2>();
            direction.x = moveDelta.x;
            direction.z = moveDelta.y;
            direction.y = verticalMovementAction.ReadValue<Vector2>().y;
#else
            if (Input.GetKey(KeyCode.W))
            {
                direction += Vector3.forward;
            }
            if (Input.GetKey(KeyCode.S))
            {
                direction += Vector3.back;
            }
            if (Input.GetKey(KeyCode.A))
            {
                direction += Vector3.left;
            }
            if (Input.GetKey(KeyCode.D))
            {
                direction += Vector3.right;
            }
            if (Input.GetKey(KeyCode.Q))
            {
                direction += Vector3.down;
            }
            if (Input.GetKey(KeyCode.E))
            {
                direction += Vector3.up;
            }
#endif
            return direction;
        }
        
        void Update()
        {
            // Exit Sample  

            if (IsEscapePressed())
            {
                Application.Quit();
				#if UNITY_EDITOR
				UnityEditor.EditorApplication.isPlaying = false; 
				#endif
            }

            // Hide and lock cursor when right mouse button pressed
            if (IsRightMouseButtonDown())
            {
                Cursor.lockState = CursorLockMode.Locked;
            }

            // Unlock and show cursor when right mouse button released
            if (IsRightMouseButtonUp())
            {
                Cursor.visible = true;
                Cursor.lockState = CursorLockMode.None;
            }

            // Rotation
            if (IsCameraRotationAllowed())
            {
                var mouseMovement = GetInputLookRotation() * Time.deltaTime * 5;
                if (invertY)
                    mouseMovement.y = -mouseMovement.y;
                
                var mouseSensitivityFactor = mouseSensitivityCurve.Evaluate(mouseMovement.magnitude);

                m_TargetCameraState.yaw += mouseMovement.x * mouseSensitivityFactor;
                m_TargetCameraState.pitch += mouseMovement.y * mouseSensitivityFactor;
            }
            
            // Translation
            var translation = GetInputTranslationDirection() * Time.deltaTime;

            // Speed up movement when shift key held
            if (IsBoostPressed())
            {
                translation *= 10.0f;
            }
            
            // Modify movement by a boost factor (defined in Inspector and modified in play mode through the mouse scroll wheel)
            boost += GetBoostFactor();
            translation *= Mathf.Pow(2.0f, boost);

            m_TargetCameraState.Translate(translation);

            // Framerate-independent interpolation
            // Calculate the lerp amount, such that we get 99% of the way to our target in the specified time
            var positionLerpPct = 1f - Mathf.Exp((Mathf.Log(1f - 0.99f) / positionLerpTime) * Time.deltaTime);
            var rotationLerpPct = 1f - Mathf.Exp((Mathf.Log(1f - 0.99f) / rotationLerpTime) * Time.deltaTime);
            m_InterpolatingCameraState.LerpTowards(m_TargetCameraState, positionLerpPct, rotationLerpPct);

            m_InterpolatingCameraState.UpdateTransform(transform);
        }

        float GetBoostFactor()
        {
#if ENABLE_INPUT_SYSTEM
            return boostFactorAction.ReadValue<Vector2>().y * 0.01f;
#else
            return Input.mouseScrollDelta.y * 0.2f;
#endif
        }

        Vector2 GetInputLookRotation()
        {
#if ENABLE_INPUT_SYSTEM
            return lookAction.ReadValue<Vector2>();
#else
            return new Vector2(Input.GetAxis("Mouse X"), Input.GetAxis("Mouse Y")) * 10;
#endif
        }

        bool IsBoostPressed()
        {
#if ENABLE_INPUT_SYSTEM
            bool boost = Keyboard.current != null ? Keyboard.current.leftShiftKey.isPressed : false; 
            boost |= Gamepad.current != null ? Gamepad.current.xButton.isPressed : false;
            return boost;
#else
            return Input.GetKey(KeyCode.LeftShift);
#endif

        }

        bool IsEscapePressed()
        {
#if ENABLE_INPUT_SYSTEM
            return Keyboard.current != null ? Keyboard.current.escapeKey.isPressed : false; 
#else
            return Input.GetKey(KeyCode.Escape);
#endif
        }

        bool IsCameraRotationAllowed()
        {
#if ENABLE_INPUT_SYSTEM
            bool canRotate = Mouse.current != null ? Mouse.current.rightButton.isPressed : false;
            canRotate |= Gamepad.current != null ? Gamepad.current.rightStick.ReadValue().magnitude > 0 : false;
            return canRotate;
#else
            return Input.GetMouseButton(1);
#endif
        }

        bool IsRightMouseButtonDown()
        {
#if ENABLE_INPUT_SYSTEM
            return Mouse.current != null ? Mouse.current.rightButton.isPressed : false;
#else
            return Input.GetMouseButtonDown(1);
#endif
        }

        bool IsRightMouseButtonUp()
        {
#if ENABLE_INPUT_SYSTEM
            return Mouse.current != null ? !Mouse.current.rightButton.isPressed : false;
#else
            return Input.GetMouseButtonUp(1);
#endif
        }

    }

}

```

# Facial rendering source scan: YABX-Brusk/EyeSystem

Repository: https://github.com/YABX-Brusk/EyeSystem

Pinned source revision: 891ca05f30471cb1df1b042f321b79f565056a6f

Discovery topic: eye shader

Description: Eye shader studio made in unity focused on VRchat and progams alike

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["Assets/The Yabba Store/LICENSE.txt", "Assets/The Yabba Store/LICENSE.txt.meta", "Packages/com.vrchat.core.vpm-resolver/License.md", "Packages/com.vrchat.core.vpm-resolver/License.md.meta"]

Topic paths: {"eye": ["Assets/The Yabba Store/3D Model/Eyev12.fbx", "Assets/The Yabba Store/3D Model/Eyev12.fbx.meta", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.docx", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.docx.meta", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.md", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.md.meta", "Assets/The Yabba Store/Eyeball textures.meta", "Assets/The Yabba Store/Eyeball textures/Pupil999v2.png", "Assets/The Yabba Store/Eyeball textures/Pupil999v2.png.meta", "Assets/The Yabba Store/HDR/eye_demo_studio.hdr", "Assets/The Yabba Store/HDR/eye_demo_studio.hdr.meta", "Assets/The Yabba Store/Materials/Final Eye v2.mat", "Assets/The Yabba Store/Materials/Final Eye v2.mat.meta", "Assets/The Yabba Store/Prefabs/GeneratedEye.prefab", "Assets/The Yabba Store/Prefabs/GeneratedEye.prefab.meta", "Assets/The Yabba Store/Prefabs/GeneratedEye_Material.mat", "Assets/The Yabba Store/Prefabs/GeneratedEye_Material.mat.meta", "Assets/The Yabba Store/Prefabs/LeftEye.prefab", "Assets/The Yabba Store/Prefabs/LeftEye.prefab.meta", "Assets/The Yabba Store/Prefabs/LeftEye_Material.mat", "Assets/The Yabba Store/Prefabs/LeftEye_Material.mat.meta", "Assets/The Yabba Store/Prefabs/RightEye.prefab", "Assets/The Yabba Store/Prefabs/RightEye.prefab.meta", "Assets/The Yabba Store/Prefabs/RightEye_Material.mat", "Assets/The Yabba Store/Prefabs/RightEye_Material.mat.meta", "Assets/The Yabba Store/Presets/AudioLink_Ambient.eyepreset", "Assets/The Yabba Store/Presets/AudioLink_Ambient.eyepreset.meta", "Assets/The Yabba Store/Presets/AudioLink_Heartbeat.eyepreset", "Assets/The Yabba Store/Presets/AudioLink_Heartbeat.eyepreset.meta", "Assets/The Yabba Store/Presets/AudioLink_Rave.eyepreset"], "shader": ["Assets/The Yabba Store/Documentation/EyeShader_Documentation.docx", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.docx.meta", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.md", "Assets/The Yabba Store/Documentation/EyeShader_Documentation.md.meta", "Assets/The Yabba Store/Editor/ForceShaderReimport.cs", "Assets/The Yabba Store/Editor/ForceShaderReimport.cs.meta", "Assets/The Yabba Store/Materials/Unit_Color.shader", "Assets/The Yabba Store/Materials/Unit_Color.shader.meta", "Assets/The Yabba Store/Shaders.meta", "Assets/The Yabba Store/Shaders/OjoEnergia_LayerSystem.cginc", "Assets/The Yabba Store/Shaders/OjoEnergia_LayerSystem.cginc.meta", "Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader", "Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader.meta", "Assets/The Yabba Store/Shaders/OjoEnergia_PupilaParallax_Fibrasv2.shader", "Assets/The Yabba Store/Shaders/OjoEnergia_PupilaParallax_Fibrasv2.shader.meta"], "rig": ["Assets/The Yabba Store/Prefabs/RightEye.prefab", "Assets/The Yabba Store/Prefabs/RightEye.prefab.meta", "Assets/The Yabba Store/Prefabs/RightEye_Material.mat", "Assets/The Yabba Store/Prefabs/RightEye_Material.mat.meta"]}

## Source file: Assets/The Yabba Store/Materials/Unit_Color.shader

{
  "path": "Assets/The Yabba Store/Materials/Unit_Color.shader",
  "blob_sha": "eb345438600de4eca08ac2c0bdbc532e026be6b5",
  "sha256": "4f4920a65cfab610d114df54e34ff81142f8d021a79f111a3b471ce0fc7b0411",
  "lines": 58,
  "bytes": 1347,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2,
    "lip": 1
  },
  "url": "https://github.com/YABX-Brusk/EyeSystem/blob/891ca05f30471cb1df1b042f321b79f565056a6f/Assets/The%20Yabba%20Store/Materials/Unit_Color.shader",
  "function_signature_candidates": []
}

```text

Shader "Unlit/Unit_Color"
{
    Properties
    {
        _MainTex ("Texture", 2D) = "white" {}
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
            // make fog work
            #pragma multi_compile_fog

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

            v2f vert (appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = TRANSFORM_TEX(v.uv, _MainTex);
                UNITY_TRANSFER_FOG(o,o.vertex);
                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                // sample the texture
                fixed4 col = tex2D(_MainTex, i.uv);
                // apply fog
                UNITY_APPLY_FOG(i.fogCoord, col);
                return col;
            }
            ENDCG
        }
    }
}


```

## Source file: Assets/The Yabba Store/Editor/ForceShaderReimport.cs

{
  "path": "Assets/The Yabba Store/Editor/ForceShaderReimport.cs",
  "blob_sha": "ea882a5ef564b67a2b51be684d56a7fe1db6807e",
  "sha256": "caf5e06ed5353f1302c988156bed98091a774f2bd4f92e2ebb3ccb3975d63db9",
  "lines": 84,
  "bytes": 3354,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 39
  },
  "url": "https://github.com/YABX-Brusk/EyeSystem/blob/891ca05f30471cb1df1b042f321b79f565056a6f/Assets/The%20Yabba%20Store/Editor/ForceShaderReimport.cs",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "static void ReimportShader()"
    },
    {
      "line": 41,
      "text": "static void ShowShaderStatus()"
    }
  ]
}

```text

using UnityEngine;
using UnityEditor;

public class ForceShaderReimport
{
    [MenuItem("Tools/OjoEnergia/Force Shader Reimport")]
    static void ReimportShader()
    {
        string shaderPath = "Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader";

        UnityEngine.Debug.Log("=== Force Shader Reimport ===\n");
        UnityEngine.Debug.Log($"Reimporting: {shaderPath}");

        // Force reimport
        AssetDatabase.ImportAsset(shaderPath, ImportAssetOptions.ForceUpdate);
        AssetDatabase.Refresh();

        // Wait a moment
        System.Threading.Thread.Sleep(1000);

        // Check if shader exists now
        Shader shader = Shader.Find("Custom/OjoEnergia_Layers");
        if (shader != null)
        {
            UnityEngine.Debug.Log("<color=green>✅ Shader found after reimport!</color>");
            UnityEngine.Debug.Log($"Shader: {shader.name}");
            UnityEngine.Debug.Log($"Supported: {shader.isSupported}");
        }
        else
        {
            UnityEngine.Debug.LogError("❌ Shader still not found after reimport!");
            UnityEngine.Debug.LogError("Check Unity Console for shader compilation errors.");
            UnityEngine.Debug.LogError("\nTry these steps:");
            UnityEngine.Debug.LogError("1. Look at Unity Console (not just test output) for shader errors");
            UnityEngine.Debug.LogError("2. Try: Assets > Reimport All");
            UnityEngine.Debug.LogError("3. Try deleting Library folder and reopening project");
        }
    }

    [MenuItem("Tools/OjoEnergia/Show Shader Compilation Status")]
    static void ShowShaderStatus()
    {
        UnityEngine.Debug.Log("=== Shader Compilation Status ===\n");

        string shaderPath = "Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader";
        Object shaderAsset = AssetDatabase.LoadAssetAtPath<Object>(shaderPath);

        if (shaderAsset == null)
        {
            UnityEngine.Debug.LogError($"❌ Shader asset not found at: {shaderPath}");
            return;
        }

        UnityEngine.Debug.Log($"✅ Shader asset exists: {shaderAsset.name}");

        Shader shader = shaderAsset as Shader;
        if (shader != null)
        {
            UnityEngine.Debug.Log($"✅ Asset is a valid Shader");
            UnityEngine.Debug.Log($"   Name: {shader.name}");
            UnityEngine.Debug.Log($"   Supported: {shader.isSupported}");
            UnityEngine.Debug.Log($"   Render Queue: {shader.renderQueue}");
        }
        else
        {
            UnityEngine.Debug.LogError("❌ Asset loaded but is not a valid Shader!");
            UnityEngine.Debug.LogError("This means the shader has compilation errors.");
            UnityEngine.Debug.LogError("\nCheck Unity Console for error messages starting with:");
            UnityEngine.Debug.LogError("'Shader error in'");
        }

        // Try to find shader by name
        Shader foundShader = Shader.Find("Custom/OjoEnergia_Layers");
        if (foundShader != null)
        {
            UnityEngine.Debug.Log("\n<color=green>✅ Shader.Find() successful!</color>");
        }
        else
        {
            UnityEngine.Debug.LogError("\n❌ Shader.Find('Custom/OjoEnergia_Layers') failed!");
            UnityEngine.Debug.LogError("The shader is not registered in Unity's shader database.");
        }
    }
}


```

## Source file: Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader

{
  "path": "Assets/The Yabba Store/Shaders/OjoEnergia_Layers.shader",
  "blob_sha": "7a4d5da18a37a54362199df4e1ecb2405cbef726",
  "sha256": "73f03383b1fd7288b13682903f1b5bd719c836176a938e01c9d494c3ae8025cd",
  "lines": 883,
  "bytes": 37995,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 4,
    "skin": 2,
    "deform": 4,
    "iris": 5,
    "cornea": 3,
    "shader": 6,
    "rig": 1,
    "lip": 1
  },
  "url": "https://github.com/YABX-Brusk/EyeSystem/blob/891ca05f30471cb1df1b042f321b79f565056a6f/Assets/The%20Yabba%20Store/Shaders/OjoEnergia_Layers.shader",
  "function_signature_candidates": [
    {
      "line": 151,
      "text": "float hash(float2 p)"
    },
    {
      "line": 157,
      "text": "float noise(float2 p)"
    },
    {
      "line": 178,
      "text": "float getAudioLinkValue(int band, float smoothing)"
    },
    {
      "line": 198,
      "text": "float GetSmoothedAudioValue(float bandIndex, float layerEnabled, float reactivity, float smoothing)"
    },
    {
      "line": 212,
      "text": "float4 SampleMascaraOjo(float2 uv, float textureIndex)"
    },
    {
      "line": 229,
      "text": "float4 SampleMascaraOjo2(float2 uv, float textureIndex)"
    },
    {
      "line": 246,
      "text": "float4 SampleRuidoEnergia(float2 uv, float textureIndex)"
    },
    {
      "line": 266,
      "text": "float sdfCircle(float2 p, float radius)"
    },
    {
      "line": 271,
      "text": "float sdfStar4(float2 p, float radius)"
    },
    {
      "line": 279,
      "text": "float sdfStar8(float2 p, float radius)"
    },
    {
      "line": 287,
      "text": "float sdfDiamond(float2 p, float radius)"
    },
    {
      "line": 293,
      "text": "float sdfHeart(float2 p, float radius)"
    },
    {
      "line": 305,
      "text": "float sdfShape(float2 p, float radius, int shapeType)"
    },
    {
      "line": 316,
      "text": "float2 rotate2D(float2 p, float angle)"
    },
    {
      "line": 325,
      "text": "float renderPupil(float2 uv, float2 center, LayerProperties layer, float time, float sizeOverride)"
    },
    {
      "line": 367,
      "text": "float3 renderFibers(float2 uv, float2 center, LayerProperties layer, SharedData sharedData)"
    },
    {
      "line": 500,
      "text": "float3 renderBorder(float2 uv, float2 center, LayerProperties layer, SharedData sharedData)"
    },
    {
      "line": 551,
      "text": "float3 renderSpecular(float2 uv, float3 viewDirTangent, LayerProperties layer)"
    },
    {
      "line": 636,
      "text": "float2 applyZoom(float2 uv, LayerProperties layer, float time)"
    },
    {
      "line": 655,
      "text": "float3 renderFractal(float2 uv, LayerProperties layer, float time)"
    },
    {
      "line": 690,
      "text": "float3 RenderEyeLayer(v2f i, SharedData sharedData, LayerProperties layer)"
    },
    {
      "line": 829,
      "text": "float4 frag(v2f i) : SV_Target"
    }
  ]
}

```text

Shader "Custom/OjoEnergia_Layers"
{
    Properties
    {
        // ========================================
        // LAYER SYSTEM PROPERTIES
        // ========================================
        [Header(Layer System)]
        _LayerCount ("Active Layer Count", Range(1, 8)) = 2

        // Layer data textures (256x1 RGBA Float, encoded by LayerPropertyEncoder)
        [HideInInspector] _LayerDataTexture0 ("Layer Data 0", 2D) = "black" {}
        [HideInInspector] _LayerDataTexture1 ("Layer Data 1", 2D) = "black" {}
        [HideInInspector] _LayerDataTexture2 ("Layer Data 2", 2D) = "black" {}
        [HideInInspector] _LayerDataTexture3 ("Layer Data 3", 2D) = "black" {}

        // Layer rendering order (8 layer indices in 2 Vector4s)
        [HideInInspector] _LayerOrder0 ("Layer Order 0-3", Vector) = (0,1,2,3)
        [HideInInspector] _LayerOrder1 ("Layer Order 4-7", Vector) = (4,5,6,7)

        // ========================================
        // GLOBAL SETTINGS
        // ========================================
        [Header(Global Settings)]
        _GlobalAnimationMultiplier ("Animation Speed Multiplier", Range(0, 2)) = 1.0
        _GlobalAudioLinkEnabled ("AudioLink Enabled", Range(0, 1)) = 0
        _AudioTexture ("AudioLink Texture (Auto)", 2D) = "black" {}

        // ========================================
        // PER-LAYER TEXTURES (Phase 5: Individual slots for VRChat compatibility)
        // ========================================
        [HideInInspector] _MascaraOjoL0 ("Layer 0", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL1 ("Layer 1", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL2 ("Layer 2", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL3 ("Layer 3", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL4 ("Layer 4", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL5 ("Layer 5", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL6 ("Layer 6", 2D) = "white" {}
        [HideInInspector] _MascaraOjoL7 ("Layer 7", 2D) = "white" {}

        [HideInInspector] _MascaraOjo2_0 ("Layer 0", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_1 ("Layer 1", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_2 ("Layer 2", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_3 ("Layer 3", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_4 ("Layer 4", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_5 ("Layer 5", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_6 ("Layer 6", 2D) = "white" {}
        [HideInInspector] _MascaraOjo2_7 ("Layer 7", 2D) = "white" {}

        [HideInInspector] _RuidoEnergiaL0 ("Layer 0", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL1 ("Layer 1", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL2 ("Layer 2", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL3 ("Layer 3", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL4 ("Layer 4", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL5 ("Layer 5", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL6 ("Layer 6", 2D) = "white" {}
        [HideInInspector] _RuidoEnergiaL7 ("Layer 7", 2D) = "white" {}

        // ========================================
        // SHARED TEXTURES (Fallback for layers without custom textures)
        // ========================================
        [Header(Shared Textures)]
        _MascaraOjo ("Mascara Ojo (Fallback)", 2D) = "white" {}
        _MascaraOjo2 ("Mascara Ojo 2 (Fallback)", 2D) = "white" {}
        _RuidoEnergia ("Ruido Energia (Fallback)", 2D) = "white" {}

        // ========================================
        // RENDER SETTINGS
        // ========================================
        [Header(Render Settings)]
        [Enum(UnityEngine.Rendering.CullMode)] _Cull ("Cull Mode", Float) = 2
        [Enum(Off,0,On,1)] _ZWrite ("ZWrite", Float) = 1
    }

    CustomEditor "TheYabbaStore.OjoEnergia.OjoEnergiaLayerEditor"

    SubShader
    {
        Tags { "RenderType"="Opaque" "Queue"="Geometry" }
        LOD 200

        Cull [_Cull]
        ZWrite [_ZWrite]

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 3.0  // VRChat compatible
            #include "UnityCG.cginc"
            #include "OjoEnergia_LayerSystem.cginc"

            #define PI 3.14159265359
            #define TAU 6.28318530718

            // ========================================
            // STRUCTURES
            // ========================================

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
                float3 normal : NORMAL;
                float4 tangent : TANGENT;
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float2 uv : TEXCOORD0;
                float3 viewDirTangent : TEXCOORD1;
                float3 worldNormal : TEXCOORD2;
                float3 worldViewDir : TEXCOORD3;
            };

            // ========================================
            // GLOBAL PROPERTIES
            // ========================================

            float _LayerCount;
            float4 _LayerOrder0;
            float4 _LayerOrder1;
            float _GlobalAnimationMultiplier;
            float _GlobalAudioLinkEnabled;

            // Shared textures (Phase 1 - Fallback)
            sampler2D _MascaraOjo;
            sampler2D _MascaraOjo2;
            sampler2D _RuidoEnergia;

            // Phase 5: Per-layer individual texture slots (VRChat compatible)
            sampler2D _MascaraOjoL0, _MascaraOjoL1, _MascaraOjoL2, _MascaraOjoL3;
            sampler2D _MascaraOjoL4, _MascaraOjoL5, _MascaraOjoL6, _MascaraOjoL7;

            sampler2D _MascaraOjo2_0, _MascaraOjo2_1, _MascaraOjo2_2, _MascaraOjo2_3;
            sampler2D _MascaraOjo2_4, _MascaraOjo2_5, _MascaraOjo2_6, _MascaraOjo2_7;

            sampler2D _RuidoEnergiaL0, _RuidoEnergiaL1, _RuidoEnergiaL2, _RuidoEnergiaL3;
            sampler2D _RuidoEnergiaL4, _RuidoEnergiaL5, _RuidoEnergiaL6, _RuidoEnergiaL7;

            // AudioLink (optional)
            sampler2D _AudioTexture;

            // ========================================
            // HELPER FUNCTIONS (from original shader)
            // ========================================

            // Simple noise function
            float hash(float2 p)
            {
                float h = dot(p, float2(127.1, 311.7));
                return frac(sin(h) * 43758.5453123);
            }

            float noise(float2 p)
            {
                float2 i = floor(p);
                float2 f = frac(p);
                f = f * f * (3.0 - 2.0 * f);

                float a = hash(i);
                float b = hash(i + float2(1.0, 0.0));
                float c = hash(i + float2(0.0, 1.0));
                float d = hash(i + float2(1.0, 1.0));

                return lerp(lerp(a, b, f.x), lerp(c, d, f.x), f.y);
            }

            // ========================================
            // AUDIOLINK HELPER FUNCTIONS (Phase 6)
            // Simplified for shader model 3.0 compatibility
            // ========================================

            // Get AudioLink value for specific band with smoothing
            // band: 0=Bass, 1=LowMid, 2=HighMid, 3=Treble
            float getAudioLinkValue(int band, float smoothing)
            {
                if (_GlobalAudioLinkEnabled < 0.5) return 0.0;

                band = clamp(band, 0, 3);

                // Simple UV calculation for AudioLink
                // Bass: 0.125, LowMid: 0.375, HighMid: 0.625, Treble: 0.875
                float bandY = float(band) * 0.25 + 0.125;
                float2 audioUV = float2(0.0, bandY);

                // Sample AudioLink texture
                float rawValue = tex2D(_AudioTexture, audioUV).r;

                // Apply smoothing curve
                float curve = 1.0 + smoothing * 2.0;
                return pow(saturate(rawValue), curve);
            }

            // Get smoothed audio value with reactivity
            float GetSmoothedAudioValue(float bandIndex, float layerEnabled, float reactivity, float smoothing)
            {
                if (layerEnabled < 0.5) return 0.0;

                int band = (int)bandIndex;
                float audioValue = getAudioLinkValue(band, smoothing);
                return audioValue * reactivity;
            }

            // ========================================
            // PHASE 5: PER-LAYER TEXTURE SAMPLING (VRChat Compatible)
            // ========================================

            // Sample MascaraOjo from per-layer slots or fallback
            float4 SampleMascaraOjo(float2 uv, float textureIndex)
            {
                int idx = (int)round(textureIndex);

                if (idx == 0) return tex2D(_MascaraOjoL0, uv);
                else if (idx == 1) return tex2D(_MascaraOjoL1, uv);
                else if (idx == 2) return tex2D(_MascaraOjoL2, uv);
                else if (idx == 3) return tex2D(_MascaraOjoL3, uv);
                else if (idx == 4) return tex2D(_MascaraOjoL4, uv);
                else if (idx == 5) return tex2D(_MascaraOjoL5, uv);
                else if (idx == 6) return tex2D(_MascaraOjoL6, uv);
                else if (idx == 7) return tex2D(_MascaraOjoL7, uv);

                return tex2D(_MascaraOjo, uv); // Fallback to shared texture
            }

            // Sample MascaraOjo2 from per-layer slots or fallback
            float4 SampleMascaraOjo2(float2 uv, float textureIndex)
            {
                int idx = (int)round(textureIndex);

                if (idx == 0) return tex2D(_MascaraOjo2_0, uv);
                else if (idx == 1) return tex2D(_MascaraOjo2_1, uv);
                else if (idx == 2) return tex2D(_MascaraOjo2_2, uv);
                else if (idx == 3) return tex2D(_MascaraOjo2_3, uv);
                else if (idx == 4) return tex2D(_MascaraOjo2_4, uv);
                else if (idx == 5) return tex2D(_MascaraOjo2_5, uv);
                else if (idx == 6) return tex2D(_MascaraOjo2_6, uv);
                else if (idx == 7) return tex2D(_MascaraOjo2_7, uv);

                return tex2D(_MascaraOjo2, uv); // Fallback
            }

            // Sample RuidoEnergia from per-layer slots or fallback
            float4 SampleRuidoEnergia(float2 uv, float textureIndex)
            {
                int idx = (int)round(textureIndex);

                if (idx == 0) return tex2D(_RuidoEnergiaL0, uv);
                else if (idx == 1) return tex2D(_RuidoEnergiaL1, uv);
                else if (idx == 2) return tex2D(_RuidoEnergiaL2, uv);
                else if (idx == 3) return tex2D(_RuidoEnergiaL3, uv);
                else if (idx == 4) return tex2D(_RuidoEnergiaL4, uv);
                else if (idx == 5) return tex2D(_RuidoEnergiaL5, uv);
                else if (idx == 6) return tex2D(_RuidoEnergiaL6, uv);
                else if (idx == 7) return tex2D(_RuidoEnergiaL7, uv);

                return tex2D(_RuidoEnergia, uv); // Fallback
            }

            // ========================================
            // SDF SHAPES (for pupil and specular)
            // ========================================

            float sdfCircle(float2 p, float radius)
            {
                return length(p) - radius;
            }

            float sdfStar4(float2 p, float radius)
            {
                float angle = atan2(p.y, p.x);
                float r = length(p);
                float star = radius * (1.0 + 0.3 * cos(4.0 * angle));
                return r - star;
            }

            float sdfStar8(float2 p, float radius)
            {
                float angle = atan2(p.y, p.x);
                float r = length(p);
                float star = radius * (1.0 + 0.25 * cos(8.0 * angle));
                return r - star;
            }

            float sdfDiamond(float2 p, float radius)
            {
                float2 absP = abs(p);
                return (absP.x + absP.y) - radius * 1.414;
            }

            float sdfHeart(float2 p, float radius)
            {
                p.y -= radius * 0.3;
                p.x *= 0.7;
                float a = atan2(p.x, p.y) / PI;
                float r = length(p);
                float h = abs(a);
                float d = (13.0 * h - 22.0 * h * h + 10.0 * h * h * h) / (6.0 - 5.0 * h);
                return r - radius * d;
            }

            // Shape dispatcher
            float sdfShape(float2 p, float radius, int shapeType)
            {
                if (shapeType == 0) return sdfCircle(p, radius);
                else if (shapeType == 1) return sdfStar4(p, radius);
                else if (shapeType == 2) return sdfStar8(p, radius);
                else if (shapeType == 3) return sdfDiamond(p, radius);
                else if (shapeType == 4) return sdfHeart(p, radius);
                return sdfCircle(p, radius);
            }

            // 2D rotation matrix
            float2 rotate2D(float2 p, float angle)
            {
                float s = sin(angle);
                float c = cos(angle);
                return float2(p.x * c - p.y * s, p.x * s + p.y * c);
            }

            // Pupil rendering with auto-movement and rotation
            // sizeOverride: if >= 0, use this size instead of layer.pupilaSize (for AudioLink)
            float renderPupil(float2 uv, float2 center, LayerProperties layer, float time, float sizeOverride)
            {
                // Auto-movement
                float2 finalCenter = center;
                if (layer.pupilaAutoMove > 0.5)
                {
                    float moveX = sin(time * layer.pupilaMoveSpeed) * layer.pupilaMoveAmount.x;
                    float moveY = cos(time * layer.pupilaMoveSpeed * 1.3) * layer.pupilaMoveAmount.y;
                    finalCenter += float2(moveX, moveY);
                }

                // Calculate position relative to pupil
                float2 p = uv - finalCenter;

                // Auto-rotation
                if (layer.pupilaRotacionAuto > 0.01)
                {
                    float autoRotAngle = time * layer.pupilaRotacionAuto;
                    p = rotate2D(p, autoRotAngle);
                }

                // Manual rotation
                if (layer.pupilaRotacion > 0.01)
                {
                    p = rotate2D(p, layer.pupilaRotacion * TAU);
                }

                // Apply aspect ratio
                p.x /= max(layer.pupilaAspect, 0.1);

                // Use override size if provided (for AudioLink), otherwise use layer size
                float finalSize = sizeOverride >= 0.0 ? sizeOverride : layer.pupilaSize;

                // Shape rendering
                int shapeType = (int)layer.pupilaForma;
                float dist = sdfShape(p, finalSize, shapeType);

                float pupilMask = smoothstep(layer.pupilaSuavidad, -layer.pupilaSuavidad, dist);
                return pupilMask;
            }

            // Advanced fiber rendering with Phase 2 features
            float3 renderFibers(float2 uv, float2 center, LayerProperties layer, SharedData sharedData)
            {
                if (layer.fibrasActivar < 0.5)
                    return float3(0, 0, 0);

                float time = sharedData.animatedTime;
                float2 p = uv - center;

                // Advanced parallax with curve and concave effect
                float dist = length(p);
                float parallaxFactor = layer.fibrasParallaxDepth;
                if (layer.fibrasParallaxCurve > 0.01)
                {
                    // Apply concave effect (center vs edge depth)
                    float centerDepth = layer.fibrasConcaveCenter;
                    float edgeDepth = layer.fibrasConcaveEdge;
                    float depthMix = saturate(dist * 2.0);
                    parallaxFactor *= lerp(centerDepth, edgeDepth, depthMix);
                    parallaxFactor *= pow(dist, layer.fibrasParallaxCurve);
                }

                // Apply parallax offset
                float2 parallaxP = p - sharedData.parallaxOffset * parallaxFactor;
                float angle = atan2(parallaxP.y, parallaxP.x);
                float parallaxDist = length(parallaxP);

                // Animated fiber rotation
                if (layer.fibrasAnimSpeed > 0.01)
                {
                    angle += time * layer.fibrasAnimSpeed;
                }

                // Base radial pattern
                float fiberCount = layer.fibrasCantidad;
                float fiberAngle = frac(angle / TAU * fiberCount);
                float fiber = smoothstep(0.5, 0.5 - layer.fibrasGrosor * 0.01, abs(fiberAngle - 0.5));

                // Noise modulation (Phase 2)
                if (layer.fibrasNoise > 0.01)
                {
                    float noiseVal = noise(parallaxP * layer.fibrasNoiseScale + time * 0.1);
                    fiber *= lerp(1.0, noiseVal, layer.fibrasNoise);
                }

                // Wave animation (Phase 2)
                if (layer.fibrasWave > 0.01)
                {
                    float waveOffset = sin(parallaxDist * layer.fibrasWaveFreq + time * 2.0) * layer.fibrasWave;
                    fiber *= (1.0 + waveOffset);
                    fiber = saturate(fiber);
                }

                // Multi-layer fibers (Phase 2)
                if (layer.fibrasCapas > 1.0)
                {
                    float layeredFiber = 0.0;
                    for (int i = 0; i < (int)layer.fibrasCapas && i < 4; i++)
                    {
                        float layerOffset = i * layer.fibrasCapaOffset;
                        float layerAngle = frac((angle + layerOffset) / TAU * fiberCount);
                        float layerFib = smoothstep(0.5, 0.5 - layer.fibrasGrosor * 0.01, abs(layerAngle - 0.5));
                        layeredFiber += layerFib / (i + 1.0);
                    }
                    fiber = layeredFiber / layer.fibrasCapas;
                }

                // Pulse animation (Phase 2)
                if (layer.fibrasPulso > 0.01)
                {
                    float pulse = sin(time * layer.fibrasPulsoSpeed) * 0.5 + 0.5;
                    pulse = pow(pulse, 2.0);
                    fiber *= lerp(1.0, 1.0 + pulse, layer.fibrasPulso);
                }

                // Phase 6: AudioLink fiber intensity reactivity
                if (layer.audioLinkEnabled > 0.5 && layer.fibrasAudioReactivity > 0.01)
                {
                    // Use low-mid band (index 1) for fiber intensity by default
                    float audioValue = GetSmoothedAudioValue(
                        1.0, // Low-mid band
                        layer.audioLinkEnabled,
                        layer.fibrasAudioReactivity,
                        layer.pupilReactionSpeed // Use same smoothing as pupil
                    );
                    fiber *= (1.0 + audioValue);
                }

                // Phase 6: AudioLink fiber radial pulse (outward expansion)
                float innerRadius = layer.fibrasRadioInterno;
                float outerRadius = layer.fibrasRadioExterno;

                if (layer.audioLinkEnabled > 0.5 && layer.fibrasPulseAudioReactivity > 0.01)
                {
                    // Use bass band (index 0) for radial pulse
                    float audioPulse = GetSmoothedAudioValue(
                        0.0, // Bass band for punch
                        layer.audioLinkEnabled,
                        layer.fibrasPulseAudioReactivity,
                        layer.pupilReactionSpeed
                    );

                    // Expand both radii outward for pulsing effect
                    innerRadius += audioPulse * 0.05;
                    outerRadius += audioPulse * 0.1;
                }

                // Radial masking
                float radialMask = smoothstep(outerRadius, innerRadius, parallaxDist);
                radialMask *= smoothstep(innerRadius - 0.05, innerRadius, parallaxDist);

                // Intensity masking (center vs border)
                float centerIntensity = layer.fibrasIntensidadCentro;
                float borderIntensity = layer.fibrasIntensidadBorde;
                float intensityMix = saturate((parallaxDist - innerRadius) / max(outerRadius - innerRadius, 0.01));
                float intensityMask = lerp(centerIntensity, borderIntensity, intensityMix);

                fiber *= radialMask * intensityMask;

                // Color blending with contrast
                float contrastedFiber = pow(saturate(fiber), 1.0 / max(layer.fibrasContraste, 0.1));
                float3 fiberColor = lerp(layer.fibrasColor2.rgb, layer.fibrasColor1.rgb, contrastedFiber);
                fiberColor *= layer.fibrasIntensidad;

                // Emission (HDR)
                if (layer.fibrasEmission > 0.01)
                {
                    fiberColor *= (1.0 + layer.fibrasEmission * contrastedFiber);
                }

                return fiberColor * fiber;
            }

            // Border rendering with Phase 2 animation
            float3 renderBorder(float2 uv, float2 center, LayerProperties layer, SharedData sharedData)
            {
                if (layer.bordeActivar < 0.5)
                    return float3(0, 0, 0);

                float time = sharedData.animatedTime;
                float2 p = uv - center;

                // Noise displacement (Phase 2)
                if (layer.bordeNoiseIntensidad > 0.01)
                {
                    float2 noiseOffset = float2(
                        noise(p * layer.bordeNoiseScale + time * layer.bordeNoiseSpeed),
                        noise(p * layer.bordeNoiseScale + time * layer.bordeNoiseSpeed + 100.0)
                    );
                    noiseOffset = (noiseOffset * 2.0 - 1.0) * layer.bordeNoiseIntensidad * 0.1;
                    p += noiseOffset;
                }

                float dist = length(p);
                float pupilEdge = layer.fibrasRadioInterno;

                // Base border glow
                float border = exp(-abs(dist - pupilEdge) / max(layer.bordeGrosor, 0.001));
                border = saturate(border * layer.bordeIntensidad);

                // Pulse animation (Phase 2)
                if (layer.bordePulsoIntensidad > 0.01)
                {
                    float pulse = sin(time * layer.bordePulsoSpeed) * 0.5 + 0.5;
                    pulse = pow(pulse, 2.0);
                    border *= (1.0 + pulse * layer.bordePulsoIntensidad);
                }

                // Phase 6: AudioLink border glow reactivity
                if (layer.audioLinkEnabled > 0.5 && layer.borderAudioReactivity > 0.01)
                {
                    // Use treble band (index 3) for border glow - creates high-frequency shimmer
                    float audioValue = GetSmoothedAudioValue(
                        3.0, // Treble band for sparkle
                        layer.audioLinkEnabled,
                        layer.borderAudioReactivity,
                        layer.pupilReactionSpeed
                    );
                    border *= (1.0 + audioValue);
                }

                return layer.bordeColor.rgb * border;
            }

            // Multi-layer specular rendering with Phase 2 features
            float3 renderSpecular(float2 uv, float3 viewDirTangent, LayerProperties layer)
            {
                float3 totalSpecular = float3(0, 0, 0);
                int layerCount = (int)layer.specularLayerCount;

                if (layerCount < 1)
                    return totalSpecular;

                // Global tracking and deformation
                float2 globalTracking = viewDirTangent.xy * layer.specularGlobalTracking;
                float globalDeform = layer.specularGlobalDeformation;

                // LAYER 1
                if (layer.specular1Enable > 0.5 && layerCount >= 1)
                {
                    float2 depthOffset = viewDirTangent.xy / max(viewDirTangent.z, 0.1) * layer.specular1Depth;
                    float2 specularUV = uv - depthOffset - globalTracking;
                    float2 p = specularUV - layer.specular1Pos;

                    // Apply rotation
                    if (layer.specular1Rotation > 0.01)
                    {
                        float rotRad = layer.specular1Rotation * PI / 180.0;
                        p = rotate2D(p, rotRad);
                    }

                    // Apply aspect ratio
                    p.x /= max(layer.specular1AspectRatio, 0.1);

                    // Apply global deformation
                    p += viewDirTangent.xy * globalDeform;

                    // Shape rendering
                    int shape = (int)layer.specular1Shape;
                    float dist = sdfShape(p, layer.specular1Size, shape);

                    float specular = smoothstep(layer.specular1Softness, -layer.specular1Softness, dist);
                    specular = pow(specular, 2.0);
                    specular *= layer.specular1Intensity;

                    float3 spec1 = layer.specular1Color.rgb * specular;
                    totalSpecular = spec1;
                }

                // LAYER 2
                if (layer.specular2Enable > 0.5 && layerCount >= 2)
                {
                    // For Phase 2, we have limited properties for layers 2-4
                    // Using basic rendering with available properties
                    float2 depthOffset = viewDirTangent.xy / max(viewDirTangent.z, 0.1) * 0.0; // depth not encoded
                    float2 specularUV = uv - depthOffset - globalTracking;

                    // Layer 2 position (not encoded, use default offset)
                    float2 pos2 = layer.specular1Pos + float2(0.1, -0.1);
                    float2 p = specularUV - pos2;

                    // Apply aspect ratio (using layer 1's value as fallback)
                    p.x /= max(layer.specular1AspectRatio, 0.1);

                    int shape = (int)layer.specular2Shape;
                    float dist = sdfShape(p, layer.specular2Size, shape);

                    float specular = smoothstep(0.02, -0.02, dist);
                    specular = pow(specular, 2.0);
                    specular *= layer.specular2Intensity;

                    float3 spec2 = layer.specular1Color.rgb * specular; // color not encoded, reuse

                    // Blend mode
                    int blendMode = (int)layer.specularBlendMode;
                    if (blendMode == 0) // Add
                        totalSpecular += spec2;
                    else if (blendMode == 1) // Screen
                        totalSpecular = 1.0 - (1.0 - totalSpecular) * (1.0 - spec2);
                    else // Max
                        totalSpecular = max(totalSpecular, spec2);
                }

                // Layers 3-4 would be implemented similarly when full properties are encoded
                // For now, return the first 2 layers

                return totalSpecular;
            }

            // Zoom effect
            float2 applyZoom(float2 uv, LayerProperties layer, float time)
            {
                if (layer.zoomActivar < 0.5)
                    return uv;

                float2 center = float2(0.5, 0.5);
                float2 toCenter = uv - center;

                // Animated zoom
                float zoomAmount = sin(time * layer.zoomSpeed) * 0.5 + 0.5;
                zoomAmount = pow(zoomAmount, 2.0);
                zoomAmount = 1.0 + zoomAmount * 0.3;

                toCenter /= zoomAmount;

                return center + toCenter;
            }

            // Fractal rendering (simplified Julia set)
            float3 renderFractal(float2 uv, LayerProperties layer, float time)
            {
                if (layer.fractalActivar < 0.5)
                    return float3(0, 0, 0);

                float2 c = float2(sin(time * layer.fractalSpeed * 0.5), cos(time * layer.fractalSpeed * 0.7)) * 0.7;
                float2 z = (uv - 0.5) * layer.fractalScale * 3.0;

                float iterations = 0.0;
                float maxIter = layer.fractalIterations;

                for (int i = 0; i < (int)maxIter && i < 50; i++)
                {
                    // Julia set iteration: z = z^2 + c
                    z = float2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;

                    if (dot(z, z) > 4.0)
                        break;

                    iterations += 1.0;
                }

                float fractalValue = iterations / maxIter;
                fractalValue = pow(fractalValue, layer.fractalDepth / 10.0);

                float3 fractalColor = lerp(layer.fractalColor1.rgb, float3(0, 0, 0), fractalValue);
                fractalColor *= layer.fractalIntensidad;

                return fractalColor;
            }

            // ========================================
            // RENDER EYE LAYER (Core function)
            // ========================================

            float3 RenderEyeLayer(v2f i, SharedData sharedData, LayerProperties layer)
            {
                float2 uv = i.uv;
                float3 finalColor = float3(0, 0, 0);
                float time = sharedData.animatedTime;

                // 0. Apply zoom effect (Phase 2)
                uv = applyZoom(uv, layer, time);

                // 1. Base iris color with advanced parallax
                float2 irisUV = uv - sharedData.parallaxOffset * layer.parallaxDepth;

                // Apply scrolling if enabled
                if (layer.scrollSpeed > 0.01)
                {
                    irisUV += float2(time * layer.scrollSpeed * 0.1, 0);
                }

                // Phase 5: Sample from texture array (or fallback to shared texture)
                float4 irisTex = SampleMascaraOjo(irisUV, layer.mascaraOjoIndex);
                finalColor = irisTex.rgb * layer.colorTint.rgb;

                // Apply blend intensity
                finalColor *= layer.blendIntensity;

                // 2. Pupil rendering with Phase 2 features (auto-movement, rotation)
                float2 pupilaCenter = layer.pupilaCenter;

                // Phase 6: AudioLink pupil expansion
                float audioModifiedSize = -1.0; // -1 = use default layer.pupilaSize
                if (layer.audioLinkEnabled > 0.5 && layer.pupilAudioReactivity > 0.01)
                {
                    float audioValue = GetSmoothedAudioValue(
                        layer.pupilAudioLinkBand,
                        layer.audioLinkEnabled,
                        layer.pupilAudioReactivity,
                        layer.pupilReactionSpeed
                    );

                    // Calculate audio-modified pupil size
                    audioModifiedSize = layer.pupilaSize + (audioValue * layer.pupilExpandAmount);
                }

                float pupilMask = renderPupil(uv, pupilaCenter, layer, time, audioModifiedSize);

                // Multi-layer pupil depth
                if (layer.pupilaCapas > 1.0)
                {
                    float3 pupilColor = layer.pupilaColorFondo.rgb;
                    for (int p = 0; p < (int)layer.pupilaCapas && p < 4; p++)
                    {
                        float layerDepth = p / layer.pupilaCapas;
                        float2 depthUV = uv - sharedData.parallaxOffset * layer.pupilaDepth * layerDepth;
                        float layerMask = renderPupil(depthUV, pupilaCenter, layer, time, audioModifiedSize);
                        float3 layerColor = lerp(layer.pupilaColorFondo.rgb, layer.pupilaColorMedio.rgb, layerDepth);
                        pupilColor = lerp(pupilColor, layerColor, layerMask);
                    }
                    finalColor = lerp(finalColor, pupilColor, pupilMask);
                }
                else
                {
                    finalColor = lerp(finalColor, layer.pupilaColorFondo.rgb, pupilMask);
                }

                // 3. Advanced fiber rendering (Phase 2: noise, wave, multi-layer, pulse)
                float3 fibers = renderFibers(uv, layer.fibrasCenter, layer, sharedData);

                // Blend fibers based on blend mode
                int fiberBlend = (int)layer.fibrasBlendMode;
                if (fiberBlend == 0) // Normal/Overlay
                    finalColor = lerp(finalColor, finalColor + fibers, saturate(length(fibers) * 0.5));
                else if (fiberBlend == 1) // Add
                    finalColor += fibers;
                else // Multiply
                    finalColor *= (1.0 + fibers / max(0.001, layer.fibrasIntensidad));

                // 4. Border glow with Phase 2 animation (pulse, noise)
                float3 border = renderBorder(uv, layer.fibrasCenter, layer, sharedData);
                finalColor += border;

                // 5. Multi-layer specular highlights (Phase 2: shapes, aspect, rotation)
                float3 specular = renderSpecular(uv, i.viewDirTangent, layer);
                finalColor += specular;

                // 6. Fresnel reflection with cornea curvature
                if (layer.corneaActivar > 0.5)
                {
                    float3 worldNormal = normalize(i.worldNormal);
                    float3 worldViewDir = normalize(i.worldViewDir);
                    float fresnel = pow(1.0 - saturate(dot(worldNormal, worldViewDir)), layer.fresnelPower * layer.corneaCurvature);
                    fresnel *= layer.fresnelIntensity;
                    finalColor += layer.fresnelColor.rgb * fresnel;
                }

                // 7. Fractal overlay (Phase 2)
                float3 fractal = renderFractal(uv, layer, time);
                if (layer.fractalActivar > 0.5)
                {
                    // Blend fractal based on fractalBlendMode (if encoded in future)
                    finalColor += fractal;
                }

                return finalColor;
            }

            // ========================================
            // VERTEX SHADER
            // ========================================

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;

                // World space vectors
                o.worldNormal = UnityObjectToWorldNormal(v.normal);
                float3 worldPos = mul(unity_ObjectToWorld, v.vertex).xyz;
                o.worldViewDir = _WorldSpaceCameraPos - worldPos;

                // Tangent space view direction for parallax
                float3 worldViewDir = normalize(o.worldViewDir);
                float3 worldNormal = normalize(o.worldNormal);
                float3 worldTangent = UnityObjectToWorldDir(v.tangent.xyz);
                float3 worldBinormal = cross(worldNormal, worldTangent) * v.tangent.w;

                o.viewDirTangent = float3(
                    dot(worldViewDir, worldTangent),
                    dot(worldViewDir, worldBinormal),
                    dot(worldViewDir, worldNormal)
                );

                return o;
            }

            // ========================================
            // FRAGMENT SHADER (Main Compositing)
            // ========================================

            float4 frag(v2f i) : SV_Target
            {
                // 1. Initialize
                float3 finalColor = float3(0, 0, 0);
                float animatedTime = _Time.y * _GlobalAnimationMultiplier;

                // 2. Get layer rendering order
                int layerOrder[8] = {
                    (int)_LayerOrder0.x, (int)_LayerOrder0.y,
                    (int)_LayerOrder0.z, (int)_LayerOrder0.w,
                    (int)_LayerOrder1.x, (int)_LayerOrder1.y,
                    (int)_LayerOrder1.z, (int)_LayerOrder1.w
                };

                // 3. Shared data (computed once for all layers)
                SharedData sharedData;
                sharedData.animatedTime = animatedTime;
                sharedData.audioValueBass = getAudioLinkValue(0, 0.1);
                sharedData.audioValueLowMid = getAudioLinkValue(1, 0.1);
                sharedData.audioValueHighMid = getAudioLinkValue(2, 0.1);
                sharedData.audioValueTreble = getAudioLinkValue(3, 0.1);
                sharedData.parallaxOffset = i.viewDirTangent.xy / max(i.viewDirTangent.z, 0.1);
                sharedData.viewDirTangent = i.viewDirTangent;

                // 4. Render layers bottom-to-top
                int layerCount = (int)_LayerCount;
                for (int orderIdx = 0; orderIdx < layerCount; orderIdx++)
                {
                    int layerIdx = layerOrder[orderIdx];

                    // Read layer properties from encoded textures
                    LayerProperties layer = ReadLayerData(layerIdx);

                    // Early exit: skip disabled or fully transparent layers
                    if (layer.enabled < 0.5 || layer.opacity < 0.001)
                        continue;

                    // Render complete eye layer
                    float3 layerColor = RenderEyeLayer(i, sharedData, layer);

                    // Composite with blend mode FIRST, then apply opacity
                    // This is Photoshop-correct: lerp(base, BlendMode(base, layer), opacity)
                    float3 blendedColor = BlendLayers(finalColor, layerColor, layer.blendMode);
                    finalColor = lerp(finalColor, blendedColor, layer.opacity);
                }

                return float4(finalColor, 1.0);
            }

            ENDCG
        }
    }

    Fallback "Diffuse"
}


```

## Source file: Assets/The Yabba Store/Shaders/OjoEnergia_LayerSystem.cginc

{
  "path": "Assets/The Yabba Store/Shaders/OjoEnergia_LayerSystem.cginc",
  "blob_sha": "2cc4e51998504eb1afdc5461a13cec1e9e929dcb",
  "sha256": "0b303031600288bc362bf0039b87fdfa31fafe02d5b9243174d0c0e6c9501f63",
  "lines": 479,
  "bytes": 16438,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "deform": 2,
    "cornea": 5,
    "rig": 2,
    "face": 2
  },
  "url": "https://github.com/YABX-Brusk/EyeSystem/blob/891ca05f30471cb1df1b042f321b79f565056a6f/Assets/The%20Yabba%20Store/Shaders/OjoEnergia_LayerSystem.cginc",
  "function_signature_candidates": [
    {
      "line": 188,
      "text": "float4 ReadLayerPixel(int textureIdx, int pixelX)"
    },
    {
      "line": 419,
      "text": "float3 BlendMode_Normal(float3 base, float3 blend) { return blend; }"
    },
    {
      "line": 420,
      "text": "float3 BlendMode_Add(float3 base, float3 blend) { return base + blend; }"
    },
    {
      "line": 421,
      "text": "float3 BlendMode_Multiply(float3 base, float3 blend) { return base * blend; }"
    },
    {
      "line": 422,
      "text": "float3 BlendMode_Screen(float3 base, float3 blend) { return 1.0 - (1.0 - base) * (1.0 - blend); }"
    },
    {
      "line": 425,
      "text": "float3 BlendMode_Overlay(float3 base, float3 blend)"
    },
    {
      "line": 434,
      "text": "float3 BlendMode_SoftLight(float3 base, float3 blend)"
    },
    {
      "line": 443,
      "text": "float3 BlendMode_HardLight(float3 base, float3 blend)"
    },
    {
      "line": 451,
      "text": "float3 BlendMode_ColorDodge(float3 base, float3 blend) { return base / max(1.0 - blend, 0.001); }"
    },
    {
      "line": 452,
      "text": "float3 BlendMode_LinearDodge(float3 base, float3 blend) { return base + blend; }"
    },
    {
      "line": 453,
      "text": "float3 BlendMode_Lighten(float3 base, float3 blend) { return max(base, blend); }"
    },
    {
      "line": 454,
      "text": "float3 BlendMode_Darken(float3 base, float3 blend) { return min(base, blend); }"
    },
    {
      "line": 455,
      "text": "float3 BlendMode_Difference(float3 base, float3 blend) { return abs(base - blend); }"
    },
    {
      "line": 458,
      "text": "float3 BlendLayers(float3 base, float3 blend, int mode)"
    }
  ]
}

```text

// ============================================================================
// OjoEnergia Layer System - Include File
// Phase 2: Complete 4-layer system with 150+ properties per layer
// ============================================================================

#ifndef OJOENERGIA_LAYER_SYSTEM_INCLUDED
#define OJOENERGIA_LAYER_SYSTEM_INCLUDED

// ============================================================================
// STRUCTURES
// ============================================================================

// Shared data computed once and passed to all layers
struct SharedData
{
    float animatedTime;
    float audioValueBass;
    float audioValueLowMid;
    float audioValueHighMid;
    float audioValueTreble;
    float2 parallaxOffset;
    float3 viewDirTangent;
};

// Layer properties (mirrors LayerConfig.cs)
// Phase 2: ~148 properties per layer (using 32 pixels = 128 floats)
struct LayerProperties
{
    // Metadata
    float enabled;
    float opacity;
    int blendMode;
    float locked;

    // Main Colors
    float4 colorTint;
    float blendIntensity;
    float finalBlendOpacity;
    float scrollSpeed;
    float distortionStrength;

    // Pupil
    float pupilaForma;
    float2 pupilaCenter;
    float pupilaSize;
    float pupilaAspect;
    float pupilaRotacion;
    float pupilaRotacionAuto;
    float pupilaSuavidad;
    float pupilaDepth;
    float pupilaCapas;
    float4 pupilaColorFondo;
    float4 pupilaColorMedio;
    float pupilaAutoMove;
    float pupilaMoveSpeed;
    float2 pupilaMoveAmount;

    // Fibers (basic)
    float fibrasActivar;
    float4 fibrasColor1;
    float4 fibrasColor2;
    float fibrasCantidad;
    float fibrasGrosor;
    float fibrasIntensidad;
    float fibrasContraste;
    float fibrasEmission;
    float fibrasBlendMode;

    // Fibers (parallax depth - concave effect)
    float fibrasParallaxDepth;
    float fibrasParallaxCurve;
    float fibrasConcaveCenter;
    float fibrasConcaveEdge;

    // Fibers (position & scale)
    float2 fibrasCenter;
    float fibrasRadioInterno;
    float fibrasRadioExterno;
    float fibrasSurfaceInnerMask;
    float fibrasIntensidadCentro;
    float fibrasIntensidadBorde;
    float fibrasOffset;
    float fibrasCenterShape;
    float fibrasCenterShapeRotation;

    // Fibers (variation)
    float fibrasNoise;
    float fibrasNoiseScale;
    float fibrasWave;
    float fibrasWaveFreq;

    // Fibers (layers)
    float fibrasCapas;
    float fibrasCapaOffset;

    // Fibers (animation)
    float fibrasAnimSpeed;
    float fibrasPulso;
    float fibrasPulsoSpeed;

    // Border
    float bordeActivar;
    float bordeGrosor;
    float bordeIntensidad;
    float bordeSuavidad;
    float4 bordeColor;
    float bordePulsoSpeed;
    float bordePulsoIntensidad;
    float bordeNoiseScale;
    float bordeNoiseIntensidad;
    float bordeNoiseSpeed;

    // Specular (multi-layer system)
    float specularLayerCount;
    float specularBlendMode;
    float specularGlobalTracking;
    float specularGlobalDeformation;

    // Specular Layer 1
    float specular1Enable;
    float specular1Shape;
    float specular1Size;
    float specular1Intensity;
    float4 specular1Color;
    float2 specular1Pos;
    float specular1Depth;
    float specular1Softness;
    float specular1AspectRatio;
    float specular1Rotation;

    // Specular Layer 2
    float specular2Enable;
    float specular2Shape;
    float specular2Size;
    float specular2Intensity;

    // Parallax & Fresnel
    float corneaActivar;
    float parallaxDepth;
    float corneaCurvature;
    float fresnelPower;
    float fresnelIntensity;
    float4 fresnelColor;

    // Zoom & Fractals
    float zoomActivar;
    float zoomSpeed;
    float fractalActivar;
    float fractalIntensidad;
    float4 fractalColor1;
    float fractalSpeed;
    float fractalScale;
    float fractalIterations;
    float fractalDepth;

    // AudioLink Reactivity
    float audioLinkEnabled;
    float pupilAudioLinkBand;
    float pupilAudioReactivity;
    float pupilExpandAmount;
    float pupilReactionSpeed;
    float borderAudioReactivity;
    float fibrasAudioReactivity;
    float fibrasPulseAudioReactivity;

    // Texture indices
    float mascaraOjoIndex;
    float mascaraOjo2Index;
    float ruidoEnergiaIndex;
};

// ============================================================================
// LAYER DATA READING
// ============================================================================

// Layer data textures (set by encoder)
// Format: RGBAHalf, 256x1, Linear (no sRGB), Point filtering, Clamp wrap
sampler2D _LayerDataTexture0;
sampler2D _LayerDataTexture1;
sampler2D _LayerDataTexture2;
sampler2D _LayerDataTexture3;

/// <summary>
/// Read a single pixel from layer data texture
/// CRITICAL: Sample at pixel center (pixelX + 0.5) to avoid precision issues
/// with Point filtering. Without this offset, you may read wrong pixels.
/// </summary>
float4 ReadLayerPixel(int textureIdx, int pixelX)
{
    // Sample at pixel center for precise Point-filtered lookup
    float u = (pixelX + 0.5) / 256.0; // 256.0 = TEXTURE_WIDTH
    float v = 0.5; // Center of 1-pixel-height texture

    if (textureIdx == 0)
        return tex2Dlod(_LayerDataTexture0, float4(u, v, 0, 0));
    else if (textureIdx == 1)
        return tex2Dlod(_LayerDataTexture1, float4(u, v, 0, 0));
    else if (textureIdx == 2)
        return tex2Dlod(_LayerDataTexture2, float4(u, v, 0, 0));
    else
        return tex2Dlod(_LayerDataTexture3, float4(u, v, 0, 0));
}

/// <summary>
/// Read all properties for a layer from encoded textures
/// </summary>
LayerProperties ReadLayerData(int layerIdx)
{
    LayerProperties props;

    // Determine which texture (2 layers per texture)
    int textureIdx = layerIdx / 2;
    int localLayerIdx = layerIdx % 2;
    int pixelOffset = localLayerIdx * 33; // 33 pixels per layer

    // Read pixels (0-21: Phase 1, 22-31: Phase 2 expansion, 32: Fractal extended)
    float4 pixel0 = ReadLayerPixel(textureIdx, pixelOffset + 0);
    float4 pixel1 = ReadLayerPixel(textureIdx, pixelOffset + 1);
    float4 pixel2 = ReadLayerPixel(textureIdx, pixelOffset + 2);
    float4 pixel3 = ReadLayerPixel(textureIdx, pixelOffset + 3);
    float4 pixel4 = ReadLayerPixel(textureIdx, pixelOffset + 4);
    float4 pixel5 = ReadLayerPixel(textureIdx, pixelOffset + 5);
    float4 pixel6 = ReadLayerPixel(textureIdx, pixelOffset + 6);
    float4 pixel7 = ReadLayerPixel(textureIdx, pixelOffset + 7);
    float4 pixel8 = ReadLayerPixel(textureIdx, pixelOffset + 8);
    float4 pixel9 = ReadLayerPixel(textureIdx, pixelOffset + 9);
    float4 pixel10 = ReadLayerPixel(textureIdx, pixelOffset + 10);
    float4 pixel11 = ReadLayerPixel(textureIdx, pixelOffset + 11);
    float4 pixel12 = ReadLayerPixel(textureIdx, pixelOffset + 12);
    float4 pixel13 = ReadLayerPixel(textureIdx, pixelOffset + 13);
    float4 pixel14 = ReadLayerPixel(textureIdx, pixelOffset + 14);
    float4 pixel15 = ReadLayerPixel(textureIdx, pixelOffset + 15);
    float4 pixel16 = ReadLayerPixel(textureIdx, pixelOffset + 16);
    float4 pixel17 = ReadLayerPixel(textureIdx, pixelOffset + 17);
    float4 pixel18 = ReadLayerPixel(textureIdx, pixelOffset + 18);
    float4 pixel19 = ReadLayerPixel(textureIdx, pixelOffset + 19);
    float4 pixel20 = ReadLayerPixel(textureIdx, pixelOffset + 20);
    float4 pixel21 = ReadLayerPixel(textureIdx, pixelOffset + 21);
    float4 pixel22 = ReadLayerPixel(textureIdx, pixelOffset + 22);
    float4 pixel23 = ReadLayerPixel(textureIdx, pixelOffset + 23);
    float4 pixel24 = ReadLayerPixel(textureIdx, pixelOffset + 24);
    float4 pixel25 = ReadLayerPixel(textureIdx, pixelOffset + 25);
    float4 pixel26 = ReadLayerPixel(textureIdx, pixelOffset + 26);
    float4 pixel27 = ReadLayerPixel(textureIdx, pixelOffset + 27);
    float4 pixel28 = ReadLayerPixel(textureIdx, pixelOffset + 28);
    float4 pixel29 = ReadLayerPixel(textureIdx, pixelOffset + 29);
    float4 pixel30 = ReadLayerPixel(textureIdx, pixelOffset + 30);
    float4 pixel31 = ReadLayerPixel(textureIdx, pixelOffset + 31);
    float4 pixel32 = ReadLayerPixel(textureIdx, pixelOffset + 32);

    // Decode properties
    props.enabled = pixel0.r;
    props.opacity = pixel0.g;
    props.blendMode = (int)pixel0.b;
    props.locked = pixel0.a;

    props.colorTint = pixel1;

    props.blendIntensity = pixel2.r;
    props.finalBlendOpacity = pixel2.g;
    props.scrollSpeed = pixel2.b;
    props.distortionStrength = pixel2.a;

    props.pupilaCenter = pixel3.rg;
    props.pupilaSize = pixel3.b;
    props.pupilaAspect = pixel3.a;

    props.pupilaForma = pixel4.r;
    props.pupilaRotacion = pixel4.g;
    props.pupilaSuavidad = pixel4.b;
    props.pupilaDepth = pixel4.a;

    props.pupilaColorFondo = float4(pixel5.rgb, 1.0);
    props.pupilaCapas = pixel5.a;

    props.pupilaColorMedio = float4(pixel6.rgb, 1.0);
    props.pupilReactionSpeed = pixel6.a;

    props.fibrasColor1 = float4(pixel7.rgb, 1.0);
    props.fibrasActivar = pixel7.a;

    props.fibrasColor2 = float4(pixel8.rgb, 1.0);
    props.fibrasBlendMode = pixel8.a;

    props.fibrasCantidad = pixel9.r;
    props.fibrasGrosor = pixel9.g;
    props.fibrasIntensidad = pixel9.b;
    props.fibrasContraste = pixel9.a;

    props.fibrasEmission = pixel10.r;
    props.fibrasCenter = pixel10.gb;
    props.fibrasParallaxDepth = pixel10.a;

    props.fibrasRadioInterno = pixel11.r;
    props.fibrasRadioExterno = pixel11.g;

    props.bordeActivar = pixel12.r;
    props.bordeGrosor = pixel12.g;
    props.bordeIntensidad = pixel12.b;
    props.bordeSuavidad = pixel12.a;

    props.bordeColor = float4(pixel13.rgb, 1.0);

    props.specular1Enable = pixel14.r;
    props.specular1Shape = pixel14.g;
    props.specular1Size = pixel14.b;
    props.specular1Intensity = pixel14.a;

    props.specular1Color = float4(pixel15.rgb, 1.0);
    props.specular1Depth = pixel15.a;

    props.specular1Pos = pixel16.rg;
    props.specular1Softness = pixel16.b;
    props.audioLinkEnabled = pixel16.a;

    props.parallaxDepth = pixel17.r;
    props.fresnelPower = pixel17.g;
    props.fresnelIntensity = pixel17.b;
    props.fibrasAudioReactivity = pixel17.a;

    props.fresnelColor = float4(pixel18.rgb, 1.0);
    props.borderAudioReactivity = pixel18.a;

    props.zoomActivar = pixel19.r;
    props.zoomSpeed = pixel19.g;
    props.fractalActivar = pixel19.b;
    props.fractalIntensidad = pixel19.a;

    props.fractalColor1 = float4(pixel20.rgb, 1.0);
    props.fibrasPulseAudioReactivity = pixel20.a;

    props.mascaraOjoIndex = pixel21.r;
    props.pupilAudioLinkBand = pixel21.g;
    props.pupilAudioReactivity = pixel21.b;
    props.pupilExpandAmount = pixel21.a;
    props.mascaraOjo2Index = -1.0; // Not encoded in Phase 2
    props.ruidoEnergiaIndex = -1.0; // Not encoded in Phase 2

    // ========================================
    // PHASE 2 EXPANSION (Pixels 22-31)
    // ========================================

    // Pixel 22: Pupil advanced
    props.pupilaRotacionAuto = pixel22.r;
    props.pupilaAutoMove = pixel22.g;
    props.pupilaMoveSpeed = pixel22.b;
    props.pupilaMoveAmount.x = pixel22.a;

    // Pixel 23: Pupil movement Y + fiber parallax advanced
    props.pupilaMoveAmount.y = pixel23.r;
    props.fibrasParallaxCurve = pixel23.g;
    props.fibrasConcaveCenter = pixel23.b;
    props.fibrasConcaveEdge = pixel23.a;

    // Pixel 24: Fiber position & masking
    props.fibrasSurfaceInnerMask = pixel24.r;
    props.fibrasIntensidadCentro = pixel24.g;
    props.fibrasIntensidadBorde = pixel24.b;
    props.fibrasOffset = pixel24.a;

    // Pixel 25: Fiber center shape + variation
    props.fibrasCenterShape = pixel25.r;
    props.fibrasCenterShapeRotation = pixel25.g;
    props.fibrasNoise = pixel25.b;
    props.fibrasNoiseScale = pixel25.a;

    // Pixel 26: Fiber wave + layers
    props.fibrasWave = pixel26.r;
    props.fibrasWaveFreq = pixel26.g;
    props.fibrasCapas = pixel26.b;
    props.fibrasCapaOffset = pixel26.a;

    // Pixel 27: Fiber animation
    props.fibrasAnimSpeed = pixel27.r;
    props.fibrasPulso = pixel27.g;
    props.fibrasPulsoSpeed = pixel27.b;
    props.bordePulsoSpeed = pixel27.a;

    // Pixel 28: Border animation
    props.bordePulsoIntensidad = pixel28.r;
    props.bordeNoiseScale = pixel28.g;
    props.bordeNoiseIntensidad = pixel28.b;
    props.bordeNoiseSpeed = pixel28.a;

    // Pixel 29: Specular global + Layer 1 extended
    props.specularLayerCount = pixel29.r;
    props.specularBlendMode = pixel29.g;
    props.specularGlobalTracking = pixel29.b;
    props.specularGlobalDeformation = pixel29.a;

    // Pixel 30: Specular 1 aspect/rotation + Layer 2 basic
    props.specular1AspectRatio = pixel30.r;
    props.specular1Rotation = pixel30.g;
    props.specular2Enable = pixel30.b;
    props.specular2Shape = pixel30.a;

    // Pixel 31: Specular 2 size/intensity + cornea
    props.specular2Size = pixel31.r;
    props.specular2Intensity = pixel31.g;
    props.corneaActivar = pixel31.b;
    props.corneaCurvature = pixel31.a;

    // Pixel 32: Fractal extended properties
    props.fractalSpeed = pixel32.r;
    props.fractalScale = pixel32.g;
    props.fractalIterations = pixel32.b;
    props.fractalDepth = pixel32.a;

    return props;
}

// ============================================================================
// BLEND MODES (12 Photoshop-style modes with correct alpha compositing)
// ============================================================================
// IMPORTANT: This function does NOT handle opacity - that's done in the compositor.
// It only blends RGB channels according to the selected mode.
// Opacity is applied separately: finalColor = lerp(base, BlendLayers(base, layer, mode), opacity)

float3 BlendMode_Normal(float3 base, float3 blend) { return blend; }
float3 BlendMode_Add(float3 base, float3 blend) { return base + blend; }
float3 BlendMode_Multiply(float3 base, float3 blend) { return base * blend; }
float3 BlendMode_Screen(float3 base, float3 blend) { return 1.0 - (1.0 - base) * (1.0 - blend); }

// Overlay: Vectorized, no per-channel branching
float3 BlendMode_Overlay(float3 base, float3 blend)
{
    float3 multiply = 2.0 * base * blend;
    float3 screen = 1.0 - 2.0 * (1.0 - base) * (1.0 - blend);
    float3 mask = step(0.5, base); // 1 if base >= 0.5, else 0
    return lerp(multiply, screen, mask);
}

// Soft Light: Vectorized
float3 BlendMode_SoftLight(float3 base, float3 blend)
{
    float3 dark = 2.0 * base * blend + base * base * (1.0 - 2.0 * blend);
    float3 bright = sqrt(base) * (2.0 * blend - 1.0) + 2.0 * base * (1.0 - blend);
    float3 mask = step(0.5, blend);
    return lerp(dark, bright, mask);
}

// Hard Light: Vectorized
float3 BlendMode_HardLight(float3 base, float3 blend)
{
    float3 multiply = 2.0 * base * blend;
    float3 screen = 1.0 - 2.0 * (1.0 - base) * (1.0 - blend);
    float3 mask = step(0.5, blend);
    return lerp(multiply, screen, mask);
}

float3 BlendMode_ColorDodge(float3 base, float3 blend) { return base / max(1.0 - blend, 0.001); }
float3 BlendMode_LinearDodge(float3 base, float3 blend) { return base + blend; }
float3 BlendMode_Lighten(float3 base, float3 blend) { return max(base, blend); }
float3 BlendMode_Darken(float3 base, float3 blend) { return min(base, blend); }
float3 BlendMode_Difference(float3 base, float3 blend) { return abs(base - blend); }

// Main blend function dispatcher
float3 BlendLayers(float3 base, float3 blend, int mode)
{
    // Round mode to nearest int to avoid float precision issues
    mode = (int)round((float)mode);

    if (mode == 0) return BlendMode_Normal(base, blend);
    else if (mode == 1) return BlendMode_Add(base, blend);
    else if (mode == 2) return BlendMode_Multiply(base, blend);
    else if (mode == 3) return BlendMode_Screen(base, blend);
    else if (mode == 4) return BlendMode_Overlay(base, blend);
    else if (mode == 5) return BlendMode_SoftLight(base, blend);
    else if (mode == 6) return BlendMode_HardLight(base, blend);
    else if (mode == 7) return BlendMode_ColorDodge(base, blend);
    else if (mode == 8) return BlendMode_LinearDodge(base, blend);
    else if (mode == 9) return BlendMode_Lighten(base, blend);
    else if (mode == 10) return BlendMode_Darken(base, blend);
    else if (mode == 11) return BlendMode_Difference(base, blend);

    return blend; // Fallback to Normal
}

#endif // OJOENERGIA_LAYER_SYSTEM_INCLUDED


```

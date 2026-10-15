# Facial rendering source scan: ForeverZack/Unity-Gpu-Skinning-Tool

Repository: https://github.com/ForeverZack/Unity-Gpu-Skinning-Tool

Pinned source revision: 9af9c4c4d0e1d046c9e56922ac44ce72035a5516

Discovery topic: skin shader

Description: Base on Unity 2021.3.0f1, UnityGpuSkinningTool can export bone matrix texture or vertices texture with RGBA32 format without losing precision.It also provides gpu skinning shader and supports unity built-in gpu instance.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "Packages/com.unity.mathematics@1.1.0/LICENSE.md", "Packages/com.unity.mathematics@1.1.0/LICENSE.md.meta", "Packages/com.unity.mathematics@1.1.0/Unity.Mathematics/Noise/LICENSE", "Packages/com.unity.mathematics@1.1.0/Unity.Mathematics/Noise/LICENSE.meta", "Packages/com.unity.render-pipelines.core@10.8.1/LICENSE.md", "Packages/com.unity.render-pipelines.core@10.8.1/LICENSE.md.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/LICENSE.md", "Packages/com.unity.render-pipelines.universal@10.8.1/LICENSE.md.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/External/LibTessDotNet/LICENSE.txt", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/External/LibTessDotNet/LICENSE.txt.meta", "Packages/com.unity.searcher@4.3.2/LICENSE.md", "Packages/com.unity.searcher@4.3.2/LICENSE.md.meta", "Packages/com.unity.shadergraph@10.8.1/LICENSE.md", "Packages/com.unity.shadergraph@10.8.1/LICENSE.md.meta"]

Topic paths: {"eye": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Eye-Surface-Type-Debug-Node.md"], "skin": ["Assets/Scenes/1.GpuSkinning.unity", "Assets/Scenes/1.GpuSkinning.unity.meta", "Assets/Scripts/Plugins/GpuSkinning.meta", "Assets/Scripts/Plugins/GpuSkinning/Animator.meta", "Assets/Scripts/Plugins/GpuSkinning/Animator/GPUSkinningAnimation.cs", "Assets/Scripts/Plugins/GpuSkinning/Animator/GPUSkinningAnimation.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Animator/GPUSkinningAnimator.cs", "Assets/Scripts/Plugins/GpuSkinning/Animator/GPUSkinningAnimator.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Animator/ModifyModelMatrixGPUSkinningAnimator.cs", "Assets/Scripts/Plugins/GpuSkinning/Animator/ModifyModelMatrixGPUSkinningAnimator.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Editor.meta", "Assets/Scripts/Plugins/GpuSkinning/Editor/EditorUtils.cs", "Assets/Scripts/Plugins/GpuSkinning/Editor/EditorUtils.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Editor/GpuSkinningInstGenerator.cs", "Assets/Scripts/Plugins/GpuSkinning/Editor/GpuSkinningInstGenerator.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Editor/GpuSkinningInstTools.cs", "Assets/Scripts/Plugins/GpuSkinning/Editor/GpuSkinningInstTools.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/GpuSkinningAnimData.cs", "Assets/Scripts/Plugins/GpuSkinning/GpuSkinningAnimData.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/GpuSkinningInstance.cs", "Assets/Scripts/Plugins/GpuSkinning/GpuSkinningInstance.cs.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuVerticesAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/MPBGpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/MPBGpuVerticesAnimation.shader.meta"], "tooth": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Sawtooth-Wave-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/SawtoothWaveNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Wave/SawtoothWaveNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Wave/SawtoothWaveNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Samples~/ProceduralPatterns/Houndstooth.shadersubgraph", "Packages/com.unity.shadergraph@10.8.1/Samples~/ProceduralPatterns/Houndstooth.shadersubgraph.meta"], "deform": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Compute-Deformation-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/ComputeDeformationNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Mesh Deformation.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Mesh Deformation/ComputeDeformNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Mesh Deformation/ComputeDeformNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Mesh Deformation/LinearBlendSkinningNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Mesh Deformation/LinearBlendSkinningNode.cs.meta"], "iris": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Iris-Limbal-Ring-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Iris-Offset-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Iris-Out-Of-Bound-Color-Clamp-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Iris-UV-Location-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Sclera-Iris-Blend-Node.md"], "cornea": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Cornea-Refraction-Node.md"], "shader": ["Assets/Scripts/Plugins/GpuSkinning/Shaders.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuVerticesAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/MPBGpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/MPBGpuVerticesAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/ModifyModelMatGpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/ModifyModelMatGpuVerticesAnimation.shader.meta", "Assets/Scripts/Plugins/GpuSkinning/Shaders/NoiseGpuVerticesAnimation.shader", "Assets/Scripts/Plugins/GpuSkinning/Shaders/NoiseGpuVerticesAnimation.shader.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/Gizmo/UnlitGizmo.shader", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/Gizmo/UnlitGizmo.shader.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Compositor.shader", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Compositor.shader.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/CubeToLatlong.shader", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/CubeToLatlong.shader.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/CSharpToHLSL.cs", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/CSharpToHLSL.cs.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ICSharpCode.NRefactory.dll", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ICSharpCode.NRefactory.dll.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ShaderGeneratorMenu.cs", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ShaderGeneratorMenu.cs.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ShaderTypeGeneration.cs", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/ShaderGenerator/ShaderTypeGeneration.cs.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Runtime/ShaderGenerator.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Runtime/ShaderGenerator/ShaderGeneratorAttributes.cs"], "rig": ["Packages/com.unity.mathematics@1.1.0/Tests/Tests/Shared/TestRigidTransform.cs", "Packages/com.unity.mathematics@1.1.0/Tests/Tests/Shared/TestRigidTransform.cs.meta", "Packages/com.unity.mathematics@1.1.0/Unity.Mathematics/rigid_transform.cs", "Packages/com.unity.mathematics@1.1.0/Unity.Mathematics/rigid_transform.cs.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/RightArrow.png", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/RightArrow.png.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/RightArrow@2x.png", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/RightArrow@2x.png.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/d_RightArrow.png", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/d_RightArrow.png.meta", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/d_RightArrow@2x.png", "Packages/com.unity.render-pipelines.core@10.8.1/Editor/LookDev/Icons/d_RightArrow@2x.png.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/2D/Resources/LightCapBottomRight.png", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/2D/Resources/LightCapBottomRight.png.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/2D/Resources/LightCapTopRight.png", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/2D/Resources/LightCapTopRight.png.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArccosineNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArccosineNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArcsineNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArcsineNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/Arctangent2Node.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/Arctangent2Node.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArctangentNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/ArctangentNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/CosineNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/CosineNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/DegreesToRadiansNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/DegreesToRadiansNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Math/Trigonometry/HyperbolicCosineNode.cs"], "face": ["Packages/com.unity.render-pipelines.core@10.8.1/ShaderLibrary/NormalSurfaceGradient.hlsl", "Packages/com.unity.render-pipelines.core@10.8.1/ShaderLibrary/NormalSurfaceGradient.hlsl.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/AssetPostProcessors/FBXArnoldSurfaceMaterialDescriptionPreprocessor.cs", "Packages/com.unity.render-pipelines.universal@10.8.1/Editor/AssetPostProcessors/FBXArnoldSurfaceMaterialDescriptionPreprocessor.cs.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/Materials/ArnoldStandardSurface.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/Materials/ArnoldStandardSurface/ArnoldStandardSurface.shadergraph", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/Materials/ArnoldStandardSurface/ArnoldStandardSurface.shadergraph.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/Materials/ArnoldStandardSurface/ArnoldStandardSurfaceTransparent.shadergraph", "Packages/com.unity.render-pipelines.universal@10.8.1/Runtime/Materials/ArnoldStandardSurface/ArnoldStandardSurfaceTransparent.shadergraph.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceData.hlsl", "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceData.hlsl.meta", "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceInput.hlsl", "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceInput.hlsl.meta", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Eye-Surface-Type-Debug-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Is-Front-Face.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/IsFrontFaceNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/GenerationMode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/GenerationMode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/DrawState.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/DrawState.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/GraphDrawingData.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/GraphDrawingData.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/IEdge.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/IEdge.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/INode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/INode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/IOnAssetEnabled.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Interfaces/Graph/IOnAssetEnabled.cs.meta"], "lip": ["Packages/com.unity.shadergraph@10.8.1/Documentation~/Ellipse-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Flip-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/Flipbook-Node.md", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/Blocks-Fragment-Alpha-Clip-Threshold.png", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/EllipseNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/FlipNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Documentation~/images/FlipbookNodeThumb.png", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Channel/FlipNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Channel/FlipNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Procedural/Shape/EllipseNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/Procedural/Shape/EllipseNode.cs.meta", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/UV/FlipbookNode.cs", "Packages/com.unity.shadergraph@10.8.1/Editor/Data/Nodes/UV/FlipbookNode.cs.meta"]}

## Source file: Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader

{
  "path": "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader",
  "blob_sha": "8facf3dea80f1058c6f3a7f6068328bb77517561",
  "sha256": "c7c0758aec43a2b0ce3711a32abfc1c285ba54b77243b5848958d1e466087076",
  "lines": 225,
  "bytes": 9103,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 1,
    "shader": 3,
    "lip": 1
  },
  "url": "https://github.com/ForeverZack/Unity-Gpu-Skinning-Tool/blob/9af9c4c4d0e1d046c9e56922ac44ce72035a5516/Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnim_Inst.shader",
  "function_signature_candidates": [
    {
      "line": 103,
      "text": "float4 indexToUV(float index)"
    },
    {
      "line": 111,
      "text": "float convertFloat16BytesToHalf(int data1, int data2)"
    },
    {
      "line": 123,
      "text": "float4 convertColors2Halfs(float4 color1, float4 color2)"
    }
  ]
}

```text

﻿// SRP Batch: support
// GPU Instancing: support
Shader "GPUSkin/GpuSkinningAnim_Inst" 
{
	Properties 
	{
		_BaseMap ("Albedo (RGB)", 2D) = "white" {}
		_Color ("Color", Color) = (1,1,1,1)
        _AnimationTex("Animation Texture", 2D) = "white" {}

		_BoneNum("Bone Num", Int) = 0
	}

	SubShader 
	{
	    HLSLINCLUDE
            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"
            
            struct Attributes
		    {
			    float4 positionOS : POSITION;
                half3 normal : NORMAL;
                float2 texcoord : TEXCOORD0;
				float4 boneIndices : TEXCOORD1;
				float4 boneWeights : TEXCOORD2;
				//float4 color : COLOR;
				UNITY_VERTEX_INPUT_INSTANCE_ID
		    };
		    struct Varyings
		    {
			    float4 positionCS : SV_POSITION;
                float2 uv : TEXCOORD0;
                UNITY_VERTEX_OUTPUT_STEREO
		    };
		    
            CBUFFER_START(UnityPerMaterial)
                half4 _Color;
                // 动画纹理尺寸信息
                float4 _AnimationTex_TexelSize;
                // 骨骼数量
                int _BoneNum;
            CBUFFER_END
            
            UNITY_INSTANCING_BUFFER_START(Props)
				// put more per-instance properties here
				UNITY_DEFINE_INSTANCED_PROP(int, _FrameIndex) // 当前动画第几帧			
				UNITY_DEFINE_INSTANCED_PROP(int, _BlendFrameIndex) // 下一个动画在第几帧			
				UNITY_DEFINE_INSTANCED_PROP(float, _BlendProgress) // 下一个动画的融合程度				
			UNITY_INSTANCING_BUFFER_END(Props)
            
            TEXTURE2D(_BaseMap);
            SAMPLER(sampler_BaseMap);

			//  动画纹理
			sampler2D _AnimationTex;
		
			float4x4 QuaternionToMatrix(float4 vec)
			{
				float4x4 ret;
				ret._11 = 2.0 * (vec.x * vec.x + vec.w * vec.w) - 1;
				ret._21 = 2.0 * (vec.x * vec.y + vec.z * vec.w);
				ret._31 = 2.0 * (vec.x * vec.z - vec.y * vec.w);
				ret._41 = 0.0;
				ret._12 = 2.0 * (vec.x * vec.y - vec.z * vec.w);
				ret._22 = 2.0 * (vec.y * vec.y + vec.w * vec.w) - 1;
				ret._32 = 2.0 * (vec.y * vec.z + vec.x * vec.w);
				ret._42 = 0.0;
				ret._13 = 2.0 * (vec.x * vec.z + vec.y * vec.w);
				ret._23 = 2.0 * (vec.y * vec.z - vec.x * vec.w);
				ret._33 = 2.0 * (vec.z * vec.z + vec.w * vec.w) - 1;
				ret._43 = 0.0;
				ret._14 = 0.0;
				ret._24 = 0.0;
				ret._34 = 0.0;
				ret._44 = 1.0;
				return ret;
			}

			float4x4 DualQuaternionToMatrix(float4 m_dual, float4 m_real)
			{
				float4x4 rotationMatrix = QuaternionToMatrix(float4(m_dual.x, m_dual.y, m_dual.z, m_dual.w));
				float4x4 translationMatrix;
				translationMatrix._11_12_13_14 = float4(1, 0, 0, 0);
				translationMatrix._21_22_23_24 = float4(0, 1, 0, 0);
				translationMatrix._31_32_33_34 = float4(0, 0, 1, 0);
				translationMatrix._41_42_43_44 = float4(0, 0, 0, 1);
				translationMatrix._14 = m_real.x;
				translationMatrix._24 = m_real.y;
				translationMatrix._34 = m_real.z;
				float4x4 scaleMatrix;
				scaleMatrix._11_12_13_14 = float4(1, 0, 0, 0);
				scaleMatrix._21_22_23_24 = float4(0, 1, 0, 0);
				scaleMatrix._31_32_33_34 = float4(0, 0, 1, 0);
				scaleMatrix._41_42_43_44 = float4(0, 0, 0, 1);
				scaleMatrix._11 = m_real.w;
				scaleMatrix._22 = m_real.w;
				scaleMatrix._33 = m_real.w;
				scaleMatrix._44 = 1;
				float4x4 M = mul(translationMatrix, mul(rotationMatrix, scaleMatrix));
				return M;
			}

			float4 indexToUV(float index)
			{
				int iIndex = trunc(index + 0.5);
				int row = (int)(iIndex * _AnimationTex_TexelSize.x);
				float col = iIndex - row*_AnimationTex_TexelSize.z;
				return float4((col+0.5)*_AnimationTex_TexelSize.x, (row+0.5) *_AnimationTex_TexelSize.y, 0, 0);
			}

			float convertFloat16BytesToHalf(int data1, int data2)
			{
				float f_data2 = data2;
				int flag = (data1/128);
				float result = data1-flag*128	// 整数部分
								+ f_data2/256;	// 小数部分
				
				result = result - 2*flag*result;		//1: 负  0:正

				return result;
			}

			float4 convertColors2Halfs(float4 color1, float4 color2)
			{
				return float4(convertFloat16BytesToHalf(floor(color1.r * 255 + 0.5), floor(color1.g * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color1.b * 255 + 0.5), floor(color1.a * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color2.r * 255 + 0.5), floor(color2.g * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color2.b * 255 + 0.5), floor(color2.a * 255 + 0.5)));
			}

			Varyings Vertex(Attributes input)
			{
				UNITY_SETUP_INSTANCE_ID(input);
				Varyings output;

				float4 boneIndices = input.boneIndices;
				float4 boneWeights = input.boneWeights;
				
				int frameIndex = UNITY_ACCESS_INSTANCED_PROP(Props, _FrameIndex);
				int blendFrameIndex = UNITY_ACCESS_INSTANCED_PROP(Props, _BlendFrameIndex);
                float blendProgress = UNITY_ACCESS_INSTANCED_PROP(Props, _BlendProgress);
                float4 boneUV1;
				float4 boneUV2;
				int frameDataPixelIndex;
				const int DEFAULT_PER_FRAME_BONE_DATASPACE = 2;

				// 正在播放的动画
				frameDataPixelIndex = _BoneNum * frameIndex * DEFAULT_PER_FRAME_BONE_DATASPACE;
				// bone0
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[0] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[0] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone0_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone1
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[1] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[1] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone1_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone2
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[2] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[2] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone2_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone3
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[3] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[3] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone3_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				
				// 动画Blend
				frameDataPixelIndex = _BoneNum * blendFrameIndex * DEFAULT_PER_FRAME_BONE_DATASPACE;
                // bone0
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[0]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[0]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone0_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone1
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[1]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[1]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone1_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone2
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[2]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[2]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone2_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone3
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[3]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[3]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone3_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				bone0_matrix = lerp(bone0_matrix, bone0_matrix_blend, blendProgress);
				bone1_matrix = lerp(bone1_matrix, bone1_matrix_blend, blendProgress);
				bone2_matrix = lerp(bone2_matrix, bone2_matrix_blend, blendProgress);
				bone3_matrix = lerp(bone3_matrix, bone3_matrix_blend, blendProgress);

				float4 pos =
					mul(bone0_matrix, input.positionOS) * boneWeights[0] +
					mul(bone1_matrix, input.positionOS) * boneWeights[1] +
					mul(bone2_matrix, input.positionOS) * boneWeights[2] +
					mul(bone3_matrix, input.positionOS) * boneWeights[3];
				output.positionCS = TransformObjectToHClip(pos.xyz);
				output.uv = input.texcoord;

				return output;
			}

			half4 Fragment(Varyings input) : SV_Target
			{
				half4 col = SAMPLE_TEXTURE2D(_BaseMap, sampler_BaseMap, input.uv);;
				return col;
			}
            

	    ENDHLSL
	
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
		    Tags { "RenderPipeline" = "UniversalPipeline" "LightMode"="UniversalForward" }
			HLSLPROGRAM
                #pragma target 3.0
                #pragma multi_compile_instancing
                
                #pragma vertex Vertex
                #pragma fragment Fragment
			ENDHLSL
		}

	}
}


```

## Source file: Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader

{
  "path": "Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader",
  "blob_sha": "c13f8497841db06544272c221732937f3ae84b94",
  "sha256": "33dd97650d60667bb7643768172b8ac552f9b95c72a2bd1004c44d19e429626f",
  "lines": 224,
  "bytes": 8907,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 1,
    "shader": 3,
    "lip": 1
  },
  "url": "https://github.com/ForeverZack/Unity-Gpu-Skinning-Tool/blob/9af9c4c4d0e1d046c9e56922ac44ce72035a5516/Assets/Scripts/Plugins/GpuSkinning/Shaders/GpuSkinningAnimation.shader",
  "function_signature_candidates": [
    {
      "line": 105,
      "text": "float4 indexToUV(float index)"
    },
    {
      "line": 113,
      "text": "float convertFloat16BytesToHalf(int data1, int data2)"
    },
    {
      "line": 125,
      "text": "float4 convertColors2Halfs(float4 color1, float4 color2)"
    }
  ]
}

```text

﻿// SRP Batch: support
// GPU Instancing: not support
Shader "GPUSkin/GpuSkinningAnimation" 
{
	Properties 
	{
		_BaseMap ("Albedo (RGB)", 2D) = "white" {}
		_Color ("Color", Color) = (1,1,1,1)
        _AnimationTex("Animation Texture", 2D) = "white" {}

		_BoneNum("Bone Num", Int) = 0
        _FrameIndex("Frame Index", Range(0.0, 196)) = 0.0
		_BlendFrameIndex("Blend Frame Index", Range(0.0, 282)) = 0.0
		_BlendProgress("Blend Progress", Range(0.0, 1.0)) = 0.0
	}

	SubShader 
	{
	    HLSLINCLUDE
            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"
            
            struct Attributes
		    {
			    float4 positionOS : POSITION;
                half3 normal : NORMAL;
				float2 texcoord : TEXCOORD0;
				float4 boneIndices : TEXCOORD1;
				float4 boneWeights : TEXCOORD2;
				//float4 color : COLOR;
				UNITY_VERTEX_INPUT_INSTANCE_ID
		    };
		    struct Varyings
		    {
			    float4 positionCS : SV_POSITION;
                float2 uv : TEXCOORD0;
                UNITY_VERTEX_OUTPUT_STEREO
		    };
		    
            CBUFFER_START(UnityPerMaterial)
                half4 _Color;
                // 动画纹理尺寸信息
                float4 _AnimationTex_TexelSize;
                // 骨骼数量
                int _BoneNum;
                // 当前动画第几帧
                int _FrameIndex;
                // 下一个动画在第几帧
                int _BlendFrameIndex;
                // 下一个动画的融合程度
                float _BlendProgress;
            CBUFFER_END
            
            TEXTURE2D(_BaseMap);
            SAMPLER(sampler_BaseMap);

			//  动画纹理
			sampler2D _AnimationTex;
		
			float4x4 QuaternionToMatrix(float4 vec)
			{
				float4x4 ret;
				ret._11 = 2.0 * (vec.x * vec.x + vec.w * vec.w) - 1;
				ret._21 = 2.0 * (vec.x * vec.y + vec.z * vec.w);
				ret._31 = 2.0 * (vec.x * vec.z - vec.y * vec.w);
				ret._41 = 0.0;
				ret._12 = 2.0 * (vec.x * vec.y - vec.z * vec.w);
				ret._22 = 2.0 * (vec.y * vec.y + vec.w * vec.w) - 1;
				ret._32 = 2.0 * (vec.y * vec.z + vec.x * vec.w);
				ret._42 = 0.0;
				ret._13 = 2.0 * (vec.x * vec.z + vec.y * vec.w);
				ret._23 = 2.0 * (vec.y * vec.z - vec.x * vec.w);
				ret._33 = 2.0 * (vec.z * vec.z + vec.w * vec.w) - 1;
				ret._43 = 0.0;
				ret._14 = 0.0;
				ret._24 = 0.0;
				ret._34 = 0.0;
				ret._44 = 1.0;
				return ret;
			}

			float4x4 DualQuaternionToMatrix(float4 m_dual, float4 m_real)
			{
				float4x4 rotationMatrix = QuaternionToMatrix(float4(m_dual.x, m_dual.y, m_dual.z, m_dual.w));
				float4x4 translationMatrix;
				translationMatrix._11_12_13_14 = float4(1, 0, 0, 0);
				translationMatrix._21_22_23_24 = float4(0, 1, 0, 0);
				translationMatrix._31_32_33_34 = float4(0, 0, 1, 0);
				translationMatrix._41_42_43_44 = float4(0, 0, 0, 1);
				translationMatrix._14 = m_real.x;
				translationMatrix._24 = m_real.y;
				translationMatrix._34 = m_real.z;
				float4x4 scaleMatrix;
				scaleMatrix._11_12_13_14 = float4(1, 0, 0, 0);
				scaleMatrix._21_22_23_24 = float4(0, 1, 0, 0);
				scaleMatrix._31_32_33_34 = float4(0, 0, 1, 0);
				scaleMatrix._41_42_43_44 = float4(0, 0, 0, 1);
				scaleMatrix._11 = m_real.w;
				scaleMatrix._22 = m_real.w;
				scaleMatrix._33 = m_real.w;
				scaleMatrix._44 = 1;
				float4x4 M = mul(translationMatrix, mul(rotationMatrix, scaleMatrix));
				return M;
			}

			float4 indexToUV(float index)
			{
				int iIndex = trunc(index + 0.5);
				int row = (int)(iIndex * _AnimationTex_TexelSize.x);
				float col = iIndex - row*_AnimationTex_TexelSize.z;
				return float4((col+0.5)*_AnimationTex_TexelSize.x, (row+0.5) *_AnimationTex_TexelSize.y, 0, 0);
			}

			float convertFloat16BytesToHalf(int data1, int data2)
			{
				float f_data2 = data2;
				int flag = (data1/128);
				float result = data1-flag*128	// 整数部分
								+ f_data2/256;	// 小数部分
				
				result = result - 2*flag*result;		//1: 负  0:正

				return result;
			}

			float4 convertColors2Halfs(float4 color1, float4 color2)
			{
				return float4(convertFloat16BytesToHalf(floor(color1.r * 255 + 0.5), floor(color1.g * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color1.b * 255 + 0.5), floor(color1.a * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color2.r * 255 + 0.5), floor(color2.g * 255 + 0.5))
							, convertFloat16BytesToHalf(floor(color2.b * 255 + 0.5), floor(color2.a * 255 + 0.5)));
			}

			Varyings Vertex(Attributes input)
			{
				UNITY_SETUP_INSTANCE_ID(input);
				Varyings output;

				float4 boneIndices = input.boneIndices;
				float4 boneWeights = input.boneWeights;
				
				int frameIndex = _FrameIndex;
				int blendFrameIndex = _BlendFrameIndex;
                float blendProgress = _BlendProgress;
                float4 boneUV1;
				float4 boneUV2;
				int frameDataPixelIndex;
				const int DEFAULT_PER_FRAME_BONE_DATASPACE = 2;

				// 正在播放的动画
				frameDataPixelIndex = _BoneNum * frameIndex * DEFAULT_PER_FRAME_BONE_DATASPACE;
				// bone0
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[0] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[0] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone0_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone1
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[1] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[1] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone1_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone2
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[2] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[2] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone2_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone3
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[3] * DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[3] * DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone3_matrix = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				
				// 动画Blend
				frameDataPixelIndex = _BoneNum * blendFrameIndex * DEFAULT_PER_FRAME_BONE_DATASPACE;
                // bone0
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[0]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[0]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone0_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone1
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[1]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[1]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone1_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone2
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[2]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[2]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone2_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				// bone3
				boneUV1 = indexToUV(frameDataPixelIndex + boneIndices[3]*DEFAULT_PER_FRAME_BONE_DATASPACE);
				boneUV2 = indexToUV(frameDataPixelIndex + boneIndices[3]*DEFAULT_PER_FRAME_BONE_DATASPACE + 1);
				float4x4 bone3_matrix_blend = DualQuaternionToMatrix(tex2Dlod(_AnimationTex, boneUV1), tex2Dlod(_AnimationTex, boneUV2));
				bone0_matrix = lerp(bone0_matrix, bone0_matrix_blend, blendProgress);
				bone1_matrix = lerp(bone1_matrix, bone1_matrix_blend, blendProgress);
				bone2_matrix = lerp(bone2_matrix, bone2_matrix_blend, blendProgress);
				bone3_matrix = lerp(bone3_matrix, bone3_matrix_blend, blendProgress);

				float4 pos =
					mul(bone0_matrix, input.positionOS) * boneWeights[0] +
					mul(bone1_matrix, input.positionOS) * boneWeights[1] +
					mul(bone2_matrix, input.positionOS) * boneWeights[2] +
					mul(bone3_matrix, input.positionOS) * boneWeights[3];
				output.positionCS = TransformObjectToHClip(pos.xyz);
				output.uv = input.texcoord;

				return output;
			}

			half4 Fragment(Varyings input) : SV_Target
			{
				half4 col = SAMPLE_TEXTURE2D(_BaseMap, sampler_BaseMap, input.uv);;
				return col;
			}
            

	    ENDHLSL
	
		Tags { "RenderType"="Opaque" }
		LOD 100

		Pass
		{
		    Tags { "RenderPipeline" = "UniversalPipeline" "LightMode"="UniversalForward" }
			HLSLPROGRAM
                #pragma vertex Vertex
                #pragma fragment Fragment
			ENDHLSL
		}

	}
}


```

## Source file: Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceData.hlsl

{
  "path": "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceData.hlsl",
  "blob_sha": "f241ce1bda1808764ffd78036d71afa619d163a6",
  "sha256": "789c20cdfaa4f9578770e999d75adec5dbd337d93d5718056013636ed19af039",
  "lines": 19,
  "bytes": 375,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 1,
    "face": 3
  },
  "url": "https://github.com/ForeverZack/Unity-Gpu-Skinning-Tool/blob/9af9c4c4d0e1d046c9e56922ac44ce72035a5516/Packages/com.unity.render-pipelines.universal%4010.8.1/ShaderLibrary/SurfaceData.hlsl",
  "function_signature_candidates": []
}

```text

#ifndef UNIVERSAL_SURFACE_DATA_INCLUDED
#define UNIVERSAL_SURFACE_DATA_INCLUDED

// Must match Universal ShaderGraph master node
struct SurfaceData
{
    half3 albedo;
    half3 specular;
    half  metallic;
    half  smoothness;
    half3 normalTS;
    half3 emission;
    half  occlusion;
    half  alpha;
    half  clearCoatMask;
    half  clearCoatSmoothness;
};

#endif


```

## Source file: Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceInput.hlsl

{
  "path": "Packages/com.unity.render-pipelines.universal@10.8.1/ShaderLibrary/SurfaceInput.hlsl",
  "blob_sha": "e835d9bffbf9d4b56e3967d6c5213900a2e07950",
  "sha256": "c4a6a123dd678dc2ef4f9c9770696db1c8a76a7b5f3f0fb14f3ca6f3800ebb9f",
  "lines": 59,
  "bytes": 1922,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 4,
    "face": 3,
    "lip": 1
  },
  "url": "https://github.com/ForeverZack/Unity-Gpu-Skinning-Tool/blob/9af9c4c4d0e1d046c9e56922ac44ce72035a5516/Packages/com.unity.render-pipelines.universal%4010.8.1/ShaderLibrary/SurfaceInput.hlsl",
  "function_signature_candidates": []
}

```text

#ifndef UNIVERSAL_INPUT_SURFACE_INCLUDED
#define UNIVERSAL_INPUT_SURFACE_INCLUDED

#include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"
#include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/SurfaceData.hlsl"
#include "Packages/com.unity.render-pipelines.core/ShaderLibrary/Packing.hlsl"
#include "Packages/com.unity.render-pipelines.core/ShaderLibrary/CommonMaterial.hlsl"

TEXTURE2D(_BaseMap);            SAMPLER(sampler_BaseMap);
TEXTURE2D(_BumpMap);            SAMPLER(sampler_BumpMap);
TEXTURE2D(_EmissionMap);        SAMPLER(sampler_EmissionMap);

///////////////////////////////////////////////////////////////////////////////
//                      Material Property Helpers                            //
///////////////////////////////////////////////////////////////////////////////
half Alpha(half albedoAlpha, half4 color, half cutoff)
{
#if !defined(_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A) && !defined(_GLOSSINESS_FROM_BASE_ALPHA)
    half alpha = albedoAlpha * color.a;
#else
    half alpha = color.a;
#endif

#if defined(_ALPHATEST_ON)
    clip(alpha - cutoff);
#endif

    return alpha;
}

half4 SampleAlbedoAlpha(float2 uv, TEXTURE2D_PARAM(albedoAlphaMap, sampler_albedoAlphaMap))
{
    return SAMPLE_TEXTURE2D(albedoAlphaMap, sampler_albedoAlphaMap, uv);
}

half3 SampleNormal(float2 uv, TEXTURE2D_PARAM(bumpMap, sampler_bumpMap), half scale = 1.0h)
{
#ifdef _NORMALMAP
    half4 n = SAMPLE_TEXTURE2D(bumpMap, sampler_bumpMap, uv);
    #if BUMP_SCALE_NOT_SUPPORTED
        return UnpackNormal(n);
    #else
        return UnpackNormalScale(n, scale);
    #endif
#else
    return half3(0.0h, 0.0h, 1.0h);
#endif
}

half3 SampleEmission(float2 uv, half3 emissionColor, TEXTURE2D_PARAM(emissionMap, sampler_emissionMap))
{
#ifndef _EMISSION
    return 0;
#else
    return SAMPLE_TEXTURE2D(emissionMap, sampler_emissionMap, uv).rgb * emissionColor;
#endif
}

#endif


```

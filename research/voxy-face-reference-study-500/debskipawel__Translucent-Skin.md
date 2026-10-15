# Facial rendering source scan: debskipawel/Translucent-Skin

Repository: https://github.com/debskipawel/Translucent-Skin

Pinned source revision: a5d3ab4fc4f591754f3fb908899b81ea32563bb8

Discovery topic: subsurface scattering

Description: An implementation of subsurface scattering techniques based on GPU Gems.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["BuD/Dependencies/DearImGui/LICENSE.txt", "BuD/Dependencies/DearImGui/examples/libs/glfw/COPYING.txt", "BuD/Dependencies/DirectXTK/LICENSE", "BuD/Dependencies/OBJ-Loader/LICENSE.txt", "BuD/Dependencies/entt-3.11.1/LICENSE"]

Topic paths: {"skin": ["BuD/Dependencies/DirectXTK/Src/Shaders/SkinnedEffect.fx", "BuD/Dependencies/DirectXTK/Src/Shaders/Skinning.fxh", "BuD/Dependencies/DirectXTK/Src/SkinnedEffect.cpp", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/SSSMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_BCBMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_CFULCMask.tga", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_ENMask.tga", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_MNAOMask.tga", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_NBMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/Std_Skin_Head_NMUILMask.tga", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/ambientOcclusionMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/diffuseMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/mikroNormalMap.png", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/mikroNormalMask.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/normalMap.png", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/roughnessMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/specularMap.jpg", "BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/transmissionMap.png"], "sss": ["BuD/Sandbox/Resources/textures/CC_Base_Body_CC_Base_Body_Std_Skin_Head/SSSMap.jpg"], "shader": ["BuD/BuD/Resources/Shaders/blurredIrradiance_ps.hlsl", "BuD/BuD/Resources/Shaders/depthMapAbsorption_ps.hlsl", "BuD/BuD/Resources/Shaders/depthMapAbsorption_vs.hlsl", "BuD/BuD/Resources/Shaders/f2_pos_vs.hlsl", "BuD/BuD/Resources/Shaders/gaussianBlur_ps.hlsl", "BuD/BuD/Resources/Shaders/lightMap_vs.hlsl", "BuD/BuD/Resources/Shaders/mvp_pos_vs.cso", "BuD/BuD/Resources/Shaders/mvp_pos_vs.hlsl", "BuD/BuD/Resources/Shaders/objModel_ps.hlsl", "BuD/BuD/Resources/Shaders/objModel_vs.hlsl", "BuD/BuD/Resources/Shaders/solidColor_ps.hlsl", "BuD/BuD/Resources/Shaders/solid_white_ps.cso", "BuD/BuD/Resources/Shaders/solid_white_ps.hlsl", "BuD/BuD/Resources/Shaders/uv_map_ps.hlsl", "BuD/BuD/Resources/Shaders/uv_map_vs.hlsl", "BuD/BuD/Resources/Shaders/uv_map_with_subscattering_ps.hlsl", "BuD/BuD/Shaders/DomainShader.h", "BuD/BuD/Shaders/GeometryShader.h", "BuD/BuD/Shaders/HullShader.h", "BuD/BuD/Shaders/Loader/ShaderLoader.cpp", "BuD/BuD/Shaders/Loader/ShaderLoader.h", "BuD/BuD/Shaders/PixelShader.h", "BuD/BuD/Shaders/Shader.cpp", "BuD/BuD/Shaders/Shader.h", "BuD/BuD/Shaders/VertexShader.h", "BuD/Dependencies/DearImGui/backends/vulkan/glsl_shader.frag", "BuD/Dependencies/DearImGui/backends/vulkan/glsl_shader.vert", "BuD/Dependencies/DirectXTK/Src/Shaders/AlphaTestEffect.fx", "BuD/Dependencies/DirectXTK/Src/Shaders/BasicEffect.fx", "BuD/Dependencies/DirectXTK/Src/Shaders/Common.fxh"], "face": ["BuD/BuD/Objects/Interfaces/IEmissive.h", "BuD/BuD/Objects/Interfaces/IRenderable.h", "BuD/BuD/Objects/Interfaces/IState.h", "BuD/Sandbox/DepthMapFaceMesh.cpp", "BuD/Sandbox/DepthMapFaceMesh.h", "BuD/Sandbox/FaceMesh.cpp", "BuD/Sandbox/FaceMesh.h", "BuD/Sandbox/GaussianBlurFaceMesh.cpp", "BuD/Sandbox/GaussianBlurFaceMesh.h", "BuD/Sandbox/MergedFaceMesh.cpp", "BuD/Sandbox/MergedFaceMesh.h"]}

## Source file: BuD/Sandbox/FaceMesh.h

{
  "path": "BuD/Sandbox/FaceMesh.h",
  "blob_sha": "66e3e0aadf0fc3380a98b40259d37c1ffd6861ab",
  "sha256": "8d7ec27ea7cab2a74765f2102a2579d932246731c72dbb484a7052cc523e3693",
  "lines": 41,
  "bytes": 1010,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "face": 4
  },
  "url": "https://github.com/debskipawel/Translucent-Skin/blob/a5d3ab4fc4f591754f3fb908899b81ea32563bb8/BuD/Sandbox/FaceMesh.h",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "virtual void LoadTextures(std::string meshName);"
    }
  ]
}

```text

#pragma once

#include <Scene/Scene.h>
#include <Objects/Interfaces/IRenderable.h>
#include <Objects/Interfaces/IState.h>
#include <Objects/MeshLoader/MeshLoader.h>

class FaceMesh
{
public:
	FaceMesh(BuD::Scene& scene);

	bool m_TransmissionMapOn = true;
	bool m_SpecularMapOn = true;
	bool m_SSSMapOn = true;
	bool m_RoughnessMapOn = true;
	bool m_NormalMapOn = true;
	bool m_MicronormalMapOn = true;
	bool m_MicronormalMaskOn = true;
	bool m_DiffuseMapOn = true;
	bool m_AmbientOcclussionMapOn = true;
	bool m_DepthMapAbsorptionOn = true;

protected:

	virtual void LoadTextures(std::string meshName);

	BuD::SceneEntity m_Entity;
	std::vector<BuD::RenderingPass> m_RenderingPasses;
	std::vector<BuD::MeshSegment> m_MeshSegments;

	BuD::Texture m_TransmissionMap;
	BuD::Texture m_SpecularMap;
	BuD::Texture m_SSSMap;
	BuD::Texture m_RoughnessMap;
	BuD::Texture m_NormalMap;
	BuD::Texture m_MicronormalMap;
	BuD::Texture m_MicronormalMask;
	BuD::Texture m_DiffuseMap;
	BuD::Texture m_AmbientOcclusionMap;
};


```

## Source file: BuD/BuD/Shaders/Shader.h

{
  "path": "BuD/BuD/Shaders/Shader.h",
  "blob_sha": "5e3c4b1982d4f9d1926bcad459049b7f8dfb7478",
  "sha256": "a216a09e4103df2d02bd6c357c69f6e54fc58976a61815bf0c4d766e1a764630",
  "lines": 48,
  "bytes": 1158,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 6
  },
  "url": "https://github.com/debskipawel/Translucent-Skin/blob/a5d3ab4fc4f591754f3fb908899b81ea32563bb8/BuD/BuD/Shaders/Shader.h",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "inline void AddConstantBuffer(std::shared_ptr<ConstantBuffer> constantBuffer)"
    },
    {
      "line": 20,
      "text": "inline void UpdateConstantBuffer(UINT bufferIndex, const void* data, size_t size)"
    },
    {
      "line": 26,
      "text": "inline void AddTexture(ComPtr<ID3D11ShaderResourceView> srv)"
    },
    {
      "line": 31,
      "text": "inline void UpdateTexture(UINT textureIndex, ComPtr<ID3D11ShaderResourceView> srv)"
    }
  ]
}

```text

#pragma once

#include <vector>
#include <memory>
#include <functional>

#include <Buffers/ConstantBuffer.h>
#include <Scene/SceneEntity.h>

namespace BuD
{
	class Shader
	{
	public:
		inline void AddConstantBuffer(std::shared_ptr<ConstantBuffer> constantBuffer)
		{
			m_ConstantBuffers.push_back(constantBuffer);
		}

		inline void UpdateConstantBuffer(UINT bufferIndex, const void* data, size_t size)
		{
			m_ConstantBuffers[bufferIndex]->Update(data, size);
		}
		
		// TODO: make a Texture class
		inline void AddTexture(ComPtr<ID3D11ShaderResourceView> srv)
		{
			m_Textures.push_back(srv);
		}
		
		inline void UpdateTexture(UINT textureIndex, ComPtr<ID3D11ShaderResourceView> srv)
		{
			m_Textures[textureIndex] = srv;
		}

		inline const auto& ConstantBuffers() const { return m_ConstantBuffers; }
		inline const auto& Textures() const { return m_Textures; }
		
		std::vector<ID3D11Buffer*> RawConstantBuffers();
		std::vector<ID3D11ShaderResourceView*> RawTextures();

	protected:
		explicit Shader() = default;

		std::vector<std::shared_ptr<ConstantBuffer>> m_ConstantBuffers;
		std::vector<ComPtr<ID3D11ShaderResourceView>> m_Textures;
	};
}


```

## Source file: BuD/Sandbox/FaceMesh.cpp

{
  "path": "BuD/Sandbox/FaceMesh.cpp",
  "blob_sha": "09fb39726e619978ece0e10b4b6e765b18268225",
  "sha256": "9fbde5549429f5149a062b266504a9548b002683b41852e2733174016c75fea4",
  "lines": 24,
  "bytes": 1216,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 1,
    "face": 3
  },
  "url": "https://github.com/debskipawel/Translucent-Skin/blob/a5d3ab4fc4f591754f3fb908899b81ea32563bb8/BuD/Sandbox/FaceMesh.cpp",
  "function_signature_candidates": []
}

```text

#include "FaceMesh.h"

FaceMesh::FaceMesh(BuD::Scene& scene)
{
	m_Entity = BuD::SceneEntity(scene);

	BuD::MeshLoader loader;
	m_MeshSegments = loader.LoadMesh("Resources/RogerHead.obj");

	LoadTextures(m_MeshSegments[0].SegmentName);
}

void FaceMesh::LoadTextures(std::string meshName)
{
	m_TransmissionMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/transmissionMap.png");
	m_SpecularMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/specularMap.jpg");
	m_SSSMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/SSSMap.jpg");
	m_RoughnessMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/roughnessMap.jpg");
	m_NormalMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/normalMap.png");
	m_MicronormalMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/mikroNormalMap.png");
	m_MicronormalMask = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/mikroNormalMask.jpg");
	m_DiffuseMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/diffuseMap.jpg");
	m_AmbientOcclusionMap = BuD::Texture::LoadFromFile("Resources/textures/" + meshName + "/ambientOcclusionMap.jpg");
}


```

## Source file: BuD/BuD/Shaders/Shader.cpp

{
  "path": "BuD/BuD/Shaders/Shader.cpp",
  "blob_sha": "dad6f62a9c4b773b4e6971198f46ac1171d16db3",
  "sha256": "cc646b7b616e5bc07908fa214ab990fab2137137d2bf2d56555c5753ff522e5a",
  "lines": 31,
  "bytes": 795,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 5
  },
  "url": "https://github.com/debskipawel/Translucent-Skin/blob/a5d3ab4fc4f591754f3fb908899b81ea32563bb8/BuD/BuD/Shaders/Shader.cpp",
  "function_signature_candidates": []
}

```text

#include "bud_pch.h"
#include "Shader.h"

namespace BuD
{
    std::vector<ID3D11Buffer*> Shader::RawConstantBuffers()
    {
        std::vector<ID3D11Buffer*> result(m_ConstantBuffers.size());

        std::transform(
            m_ConstantBuffers.begin(), m_ConstantBuffers.end(),
            result.begin(),
            [](std::shared_ptr<ConstantBuffer> buffer) { return buffer->Get(); }
        );

        return result;
    }

    std::vector<ID3D11ShaderResourceView*> Shader::RawTextures()
    {
        std::vector<ID3D11ShaderResourceView*> result(m_Textures.size());

        std::transform(
            m_Textures.begin(), m_Textures.end(),
            result.begin(),
            [](ComPtr<ID3D11ShaderResourceView> b) { return b.Get(); }
        );

        return result;
    }
}


```

# Facial rendering source scan: michead/vk_renderer

Repository: https://github.com/michead/vk_renderer

Pinned source revision: 071c40a7473c1aa4ae8c1dec3395a0ec6e46773f

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["compileShaders.bat", "vk_renderer/compile_shaders.bat", "vk_renderer/shaders/final/frag.spv", "vk_renderer/shaders/final/shader.frag", "vk_renderer/shaders/final/shader.vert", "vk_renderer/shaders/final/vert.spv", "vk_renderer/shaders/frag.spv", "vk_renderer/shaders/geom/frag.spv", "vk_renderer/shaders/geom/shader.frag", "vk_renderer/shaders/geom/shader.vert", "vk_renderer/shaders/geom/vert.spv", "vk_renderer/shaders/geometry/frag.spv", "vk_renderer/shaders/geometry/shader.frag", "vk_renderer/shaders/geometry/shader.vert", "vk_renderer/shaders/geometry/vert.spv", "vk_renderer/shaders/lighting/frag.spv", "vk_renderer/shaders/lighting/shader.frag", "vk_renderer/shaders/lighting/shader.vert", "vk_renderer/shaders/lighting/vert.spv", "vk_renderer/shaders/merge/frag.spv", "vk_renderer/shaders/merge/shader.frag", "vk_renderer/shaders/merge/shader.vert", "vk_renderer/shaders/merge/vert.spv", "vk_renderer/shaders/shader.frag", "vk_renderer/shaders/shader.vert", "vk_renderer/shaders/shadow/frag.spv", "vk_renderer/shaders/shadow/shader.frag", "vk_renderer/shaders/shadow/shader.vert", "vk_renderer/shaders/shadow/vert.spv", "vk_renderer/shaders/ssao-blur/frag.spv"]}

## Source file: vk_renderer/Mesh.h

{
  "path": "vk_renderer/Mesh.h",
  "blob_sha": "0207b404e62804022e4f373d7339aae5b96e3a6e",
  "sha256": "7848c74f4e9240ef6e51d262e6fc682ef3c8543ea4fbb89c784aab11d46dafcb",
  "lines": 42,
  "bytes": 941,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/michead/vk_renderer/blob/071c40a7473c1aa4ae8c1dec3395a0ec6e46773f/vk_renderer/Mesh.h",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "void initBuffers() { initVertexBuffer(); initIndexBuffer(); }"
    },
    {
      "line": 40,
      "text": "void initVertexBuffer();"
    },
    {
      "line": 41,
      "text": "void initIndexBuffer();"
    }
  ]
}

```text

#pragma once

#include "VkEngine.h"
#include "Material.h"
#include "MathUtils.h"
#include "Frame.h"


struct MeshUniformBufferObject {
	glm::mat4 model;
};

class Mesh {
	friend class VkEngine;
	friend struct Scene;
	friend class ShadowPass;
	friend class GeometryPass;

public:
	std::string getName() const { return name; }
	VkBuffer getVertexBuffer() const { return vertexBuffer; }
	VkBuffer getIndexBuffer() const { return indexBuffer; }
	Material* getMaterial() const { return material; }
	glm::mat4 getModelMatrix() const { return frame.toMatrix(); }

	void initBuffers() { initVertexBuffer(); initIndexBuffer(); }

private:
	std::string name;
	std::vector<Vertex> vertices;
	std::vector<uint32_t> indices;
	Material* material;
	Frame frame = { IDENTITY_FRAME };

	VkBuffer vertexBuffer;
	VkDeviceMemory vertexBufferMemory;
	VkBuffer indexBuffer;
	VkDeviceMemory indexBufferMemory;

	void initVertexBuffer();
	void initIndexBuffer();
};

```

## Source file: vk_renderer/Pass.h

{
  "path": "vk_renderer/Pass.h",
  "blob_sha": "638a119061f4c61230c32a4b0439da4f52d7442d",
  "sha256": "fd2a41f3def18f7df817cf2c582df48dc6e1a5e2a252c05e182a92121dbb58ea",
  "lines": 60,
  "bytes": 1491,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/michead/vk_renderer/blob/071c40a7473c1aa4ae8c1dec3395a0ec6e46773f/vk_renderer/Pass.h",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "virtual void init();"
    },
    {
      "line": 25,
      "text": "virtual void initBufferData() { /*NOP*/ }"
    },
    {
      "line": 26,
      "text": "virtual void updateBufferData() { /*NOP*/ }"
    },
    {
      "line": 43,
      "text": "virtual void initMeshBuffers();"
    },
    {
      "line": 44,
      "text": "virtual void initTextures();"
    },
    {
      "line": 45,
      "text": "virtual void initDepthResources();"
    },
    {
      "line": 46,
      "text": "virtual void initFramebuffers() { }"
    },
    {
      "line": 47,
      "text": "virtual void initUniformBuffer() { }"
    },
    {
      "line": 48,
      "text": "virtual void initSemaphores() { }"
    },
    {
      "line": 50,
      "text": "virtual void initAttachments() = 0;"
    },
    {
      "line": 51,
      "text": "virtual void initCommandBuffers() = 0;"
    },
    {
      "line": 52,
      "text": "virtual void initDescriptorSets() = 0;"
    },
    {
      "line": 53,
      "text": "virtual void initDescriptorSetLayout() = 0;"
    },
    {
      "line": 54,
      "text": "virtual void initGraphicsPipeline() = 0;"
    }
  ]
}

```text

#pragma once

#include "VkEngine.h"
#include "GBuffer.h"


#define TRANSPARENT_BLACK_CLEAR	{ 0, 0, 0, 0 }
#define OPAQUE_BLACK_CLEAR		{ 0, 0, 0, 1 }
#define DEPTH_STENCIL_CLEAR		{ 1, 0 }


struct Texture;


class Pass {
	friend class VkEngine;

public:
	Pass() { }
	Pass(std::string vsPath, std::string fsPath) : vsPath(vsPath), fsPath(fsPath) { }
	Pass(std::string vsPath, std::string gsPath, std::string fsPath) : vsPath(vsPath), gsPath(gsPath), fsPath(fsPath) { }
	~Pass() { }

	virtual void init();
	virtual void initBufferData() { /*NOP*/ }
	virtual void updateBufferData() { /*NOP*/ }
	
	virtual GBuffer* getGBuffer() { return nullptr; }

protected:
	std::string vsPath;
	std::string gsPath;
	std::string fsPath;

	VkPipelineLayout pipelineLayout;
	VkPipeline pipeline;
	VkImage depthImage;
	VkImageView depthImageView;
	VkDeviceMemory depthImageMemory;
	std::vector<VkDescriptorSet> descriptorSets;
	VkDescriptorSetLayout descriptorSetLayout;

	virtual void initMeshBuffers();
	virtual void initTextures();
	virtual void initDepthResources();
	virtual void initFramebuffers() { }
	virtual void initUniformBuffer() { }
	virtual void initSemaphores() { }

	virtual void initAttachments() = 0;
	virtual void initCommandBuffers() = 0;
	virtual void initDescriptorSets() = 0;
	virtual void initDescriptorSetLayout() = 0;
	virtual void initGraphicsPipeline() = 0;

private:
	VkRenderPass renderPass;
	std::vector<VkFramebuffer> framebuffers;
	std::vector<VkCommandBuffer> commandBuffers;
};

```

## Source file: vk_renderer/Quad.h

{
  "path": "vk_renderer/Quad.h",
  "blob_sha": "31ae628a6a12dc84930f11bbf664f4429256c78f",
  "sha256": "22fd549596a73e0ed10652c9e3f51f9fcab8235d0ff031c4c990f4d1382e0209",
  "lines": 31,
  "bytes": 674,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/michead/vk_renderer/blob/071c40a7473c1aa4ae8c1dec3395a0ec6e46773f/vk_renderer/Quad.h",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "void initVertexBuffer();"
    },
    {
      "line": 30,
      "text": "void initIndexBuffer();"
    }
  ]
}

```text

#pragma once

#include "Vertex.h"

#define GLM_FORCE_RADIANS
#define GLM_FORCE_DEPTH_ZERO_TO_ONE
#include "glm\glm.hpp"


class Quad {
public:
	Quad() { initVertexBuffer(); initIndexBuffer(); }
	~Quad() { }

	static const std::vector<glm::vec3> positions;
	static const std::vector<uint32_t> indices;
	static const std::vector<glm::vec2> uvs;
	static const std::vector<Vertex> vertices;

	VkBuffer getVertexBuffer() { return vertexBuffer; }
	VkBuffer getIndexBuffer() { return indexBuffer; }

private:
	VkBuffer vertexBuffer;
	VkDeviceMemory vertexBufferMemory;
	VkBuffer indexBuffer;
	VkDeviceMemory indexBufferMemory;

	void initVertexBuffer();
	void initIndexBuffer();
};

```

## Source file: vk_renderer/Frame.h

{
  "path": "vk_renderer/Frame.h",
  "blob_sha": "b787a56ef133f8e0db7ca7946e9de49f12fb0c21",
  "sha256": "dbcc228ad7f3548fa8b9735cf078911423b83fbdaed9e0981e4c6444d33bbfe8",
  "lines": 48,
  "bytes": 1016,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "rig": 5
  },
  "url": "https://github.com/michead/vk_renderer/blob/071c40a7473c1aa4ae8c1dec3395a0ec6e46773f/vk_renderer/Frame.h",
  "function_signature_candidates": []
}

```text

#pragma once

#include "MathUtils.h"

#define GLM_FORCE_RADIANS
#define GLM_FORCE_DEPTH_ZERO_TO_ONE
#include <glm\glm.hpp>


#define IDENTITY_FRAME {0, 0, 0}, {1, 0, 0}, {0, 1, 0}, {0, 0, 1}


struct Frame {
	glm::vec3 origin;
	glm::vec3 xAxis;
	glm::vec3 yAxis;
	glm::vec3 zAxis;

	static Frame lookAtFrame(const glm::vec3& eye, const glm::vec3& center, const glm::vec3& up)
	{
		Frame frame = {};
		frame.origin = eye;
		frame.zAxis = -normalize(center - eye);

		frame.yAxis = glm::normalize(glm::cross(up, frame.zAxis));
		frame.xAxis = glm::cross(frame.yAxis, frame.zAxis);

		return frame;
	}

	static Frame orthonormalizeF(const Frame& f)
	{
		Frame ret = f;
		ret.zAxis = normalize(f.zAxis);
		ret.xAxis = orthonormalize(f.xAxis, ret.zAxis);
		ret.yAxis = normalize(cross(ret.zAxis, ret.xAxis)); 
		return ret;
	}

	glm::mat4 toMatrix() const
	{
		return glm::mat4(
			xAxis.x, yAxis.x, zAxis.x, origin.x,
			xAxis.y, yAxis.y, zAxis.y, origin.y,
			xAxis.z, yAxis.z, zAxis.z, origin.z,
			0, 0, 0, 1);
	}
};


```

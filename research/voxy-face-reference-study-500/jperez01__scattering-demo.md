# Facial rendering source scan: jperez01/scattering-demo

Repository: https://github.com/jperez01/scattering-demo

Pinned source revision: 49e00f2295027c4feae085f9bacf42a0bb34d309

Discovery topic: subsurface scattering

Description: Subsurface Scattering Demo

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["shaders/aliasing/aliasing.fs", "shaders/animation/model.vs", "shaders/blending/blend.fs", "shaders/blending/blend.vs", "shaders/bloom/blur.fs", "shaders/bloom/blur.vs", "shaders/bloom/final.fs", "shaders/bloom/final.vs", "shaders/bloom/first.fs", "shaders/bloom/first.vs", "shaders/bloom/second.fs", "shaders/cascade/cascadeDebugF.glsl", "shaders/cascade/cascadeDebugV.glsl", "shaders/cascade/lightF.glsl", "shaders/cascade/mapDebugF.glsl", "shaders/cascade/mapDebugV.glsl", "shaders/compute/basic.glsl", "shaders/compute/tutorial.glsl", "shaders/cubemap/cb.fs", "shaders/cubemap/cb.vs", "shaders/cubemap/map.fs", "shaders/cubemap/map.vs", "shaders/default/default.fs", "shaders/default/default.vs", "shaders/default/defaultColor.fs", "shaders/default/defaultScreen.vs", "shaders/default/defaultTexture.fs", "shaders/default/defaultTexture.vs", "shaders/deferred/gbuffer.fs", "shaders/deferred/gbuffer.vs"], "rig": ["third_party/glm/glm/detail/func_trigonometric.inl", "third_party/glm/glm/detail/func_trigonometric_simd.inl", "third_party/glm/glm/ext/quaternion_trigonometric.hpp", "third_party/glm/glm/ext/quaternion_trigonometric.inl", "third_party/glm/glm/gtx/fast_trigonometry.hpp", "third_party/glm/glm/gtx/fast_trigonometry.inl", "third_party/glm/glm/simd/trigonometric.h", "third_party/glm/glm/trigonometric.hpp"], "lip": ["third_party/glm/glm/ext/matrix_clip_space.hpp", "third_party/glm/glm/ext/matrix_clip_space.inl"]}

## Source file: src/shader/shader.h

{
  "path": "src/shader/shader.h",
  "blob_sha": "a6e0bf8f2b4ff8214987f13318df48bab340e4a3",
  "sha256": "4b5b36b98ea92d3da238897974b6657bdf82bab714fffbbad496ba2409f1398e",
  "lines": 70,
  "bytes": 2674,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 16
  },
  "url": "https://github.com/jperez01/scattering-demo/blob/49e00f2295027c4feae085f9bacf42a0bb34d309/src/shader/shader.h",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "void use() const;"
    },
    {
      "line": 37,
      "text": "void reloadShader(const char* codeBuffer, GLenum shaderType);"
    },
    {
      "line": 39,
      "text": "void setBool(const std::string &name, bool value) const;"
    },
    {
      "line": 41,
      "text": "void setInt(const std::string &name, int value) const;"
    },
    {
      "line": 43,
      "text": "void setFloat(const std::string &name, float value) const;"
    },
    {
      "line": 45,
      "text": "void setVec2(const std::string &name, const glm::vec2 &value) const;"
    },
    {
      "line": 46,
      "text": "void setVec2(const std::string &name, float x, float y) const;"
    },
    {
      "line": 48,
      "text": "void setVec3(const std::string &name, const glm::vec3 &value) const;"
    },
    {
      "line": 49,
      "text": "void setVec3(const std::string &name, float x, float y, float z) const;"
    },
    {
      "line": 51,
      "text": "void setVec4(const std::string &name, const glm::vec4 &value) const;"
    },
    {
      "line": 52,
      "text": "void setVec4(const std::string &name, float x, float y, float z, float w);"
    },
    {
      "line": 54,
      "text": "void setMat2(const std::string &name, const glm::mat2 &mat) const;"
    },
    {
      "line": 56,
      "text": "void setMat3(const std::string &name, const glm::mat3 &mat) const;"
    },
    {
      "line": 58,
      "text": "void setMat4(const std::string &name, const glm::mat4 &mat) const;"
    },
    {
      "line": 61,
      "text": "void setupProgram(const vector<unsigned int>& shaderIds);"
    },
    {
      "line": 67,
      "text": "void openAndLoadShaderFile(const string& fullPath, string& codeBuffer);"
    },
    {
      "line": 68,
      "text": "unsigned int compileShader(GLint shaderType, const char* codeBuffer);"
    }
  ]
}

```text

#ifndef SHADER_H
#define SHADER_H

#include <glad/glad.h>

#include <string>
#include <fstream>
#include <sstream>
#include <iostream>
#include <map>
#include <vector>

#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>
#include <glm/gtc/type_ptr.hpp>

using namespace std;

inline std::map<GLint, std::string> shaderTypeToStringMap{
    {GL_VERTEX_SHADER, "VERTEX"},
    {GL_FRAGMENT_SHADER, "FRAGMENT"},
    {GL_PROGRAM, "PROGRAM"},
    {GL_GEOMETRY_SHADER, "GEOMETRTY"},
    {GL_COMPUTE_SHADER, "COMPUTE"}
};

class Shader {
public:
    unsigned int ID{};
    std::vector<std::pair<GLenum, std::string>> shaderTypesAndPaths;

    Shader();
    explicit Shader(const char* computePath);
    Shader(const char* vertexPath, const char* fragmentPath, const char* geoPath = nullptr);
    void use() const;

    void reloadShader(const char* codeBuffer, GLenum shaderType);

    void setBool(const std::string &name, bool value) const;
    // ------------------------------------------------------------------------
    void setInt(const std::string &name, int value) const;
    // ------------------------------------------------------------------------
    void setFloat(const std::string &name, float value) const;
    // ------------------------------------------------------------------------
    void setVec2(const std::string &name, const glm::vec2 &value) const;
    void setVec2(const std::string &name, float x, float y) const;
    // ------------------------------------------------------------------------
    void setVec3(const std::string &name, const glm::vec3 &value) const;
    void setVec3(const std::string &name, float x, float y, float z) const;
    // ------------------------------------------------------------------------
    void setVec4(const std::string &name, const glm::vec4 &value) const;
    void setVec4(const std::string &name, float x, float y, float z, float w);
    // ------------------------------------------------------------------------
    void setMat2(const std::string &name, const glm::mat2 &mat) const;
    // ------------------------------------------------------------------------
    void setMat3(const std::string &name, const glm::mat3 &mat) const;
    // ------------------------------------------------------------------------
    void setMat4(const std::string &name, const glm::mat4 &mat) const;

private:
    void setupProgram(const vector<unsigned int>& shaderIds);

    unsigned int vertexId{}, fragmentId{}, geometryId{};
    unsigned int computeId{};
};

void openAndLoadShaderFile(const string& fullPath, string& codeBuffer);
unsigned int compileShader(GLint shaderType, const char* codeBuffer);

#endif

```

## Source file: shaders/ssr/ssrF.glsl

{
  "path": "shaders/ssr/ssrF.glsl",
  "blob_sha": "1c292141ec3b9ef605b3190fc70b832a19ae51fc",
  "sha256": "d769c4aa1a219e700ef7eba206c90f68020e0e7b94b68d5a9dffbd6068527eeb",
  "lines": 143,
  "bytes": 3737,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 2
  },
  "url": "https://github.com/jperez01/scattering-demo/blob/49e00f2295027c4feae085f9bacf42a0bb34d309/shaders/ssr/ssrF.glsl",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "vec4 rayCast(in vec3 dir, inout vec3 hitCoord, out float dDepth);"
    },
    {
      "line": 32,
      "text": "vec3 binarySearch(inout vec3 dir, inout vec3 hitCoord, inout float dDepth);"
    },
    {
      "line": 33,
      "text": "vec3 hash(vec3 a);"
    },
    {
      "line": 35,
      "text": "vec3 positionFromDepth(float depth) {"
    },
    {
      "line": 45,
      "text": "bool isSignificant(float dd) {"
    },
    {
      "line": 49,
      "text": "void main() {"
    },
    {
      "line": 83,
      "text": "vec4 rayCast(in vec3 dir, inout vec3 hitCoord, out float dDepth) {"
    },
    {
      "line": 113,
      "text": "vec3 binarySearch(inout vec3 dir, inout vec3 hitCoord, inout float dDepth) {"
    },
    {
      "line": 138,
      "text": "vec3 hash(vec3 a)"
    }
  ]
}

```text

#version 460 core

layout (location = 0) out vec4 FragColor;

noperspective in vec2 TexCoords;

uniform sampler2D depthTexture;
uniform sampler2D positionTexture;
uniform sampler2D normalTexture;
uniform sampler2D colorTexture;

uniform mat4 projection;
uniform mat4 invProjection;
uniform mat4 view;
uniform mat4 invView;
uniform mat4 invTransposeView;

uniform float depthCutoff = 0.0;

const float rayStep = 0.1;
const float minRayStep = 0.1;
uniform float maxRayStep = 1.2;
const int maxSteps = 30;
const float searchDist = 5;
const int numBinarySearchSteps = 5;
const float reflectionSpecularFalloffExponent = 3.0;

#define Scale vec3(.8, .8, .8)
#define K 19.19

vec4 rayCast(in vec3 dir, inout vec3 hitCoord, out float dDepth);
vec3 binarySearch(inout vec3 dir, inout vec3 hitCoord, inout float dDepth);
vec3 hash(vec3 a);

vec3 positionFromDepth(float depth) {
	float z = depth * 2.0 - 1.0;

	vec4 clipSpacePosition = vec4(TexCoords * 2.0 - 1.0, z, 1.0);
	vec4 viewSpacePosition = invProjection * clipSpacePosition;
	viewSpacePosition /= viewSpacePosition.w;

	return viewSpacePosition.xyz;
}

bool isSignificant(float dd) {
	return dd < maxRayStep && dd > depthCutoff;
}

void main() {
	float currentDepth = texture(depthTexture, TexCoords).x;
	vec3 depthPos = positionFromDepth(currentDepth);

	if (currentDepth > 0.999999) {
		FragColor = vec4(0.0);
		return;
	}

	vec3 viewNormal = vec3(texture(normalTexture, TexCoords));
	vec3 viewPos = texture(positionTexture, TexCoords).xyz;
	vec3 albedo = texture(colorTexture, TexCoords).xyz;
	float spec = texture(colorTexture, TexCoords).w;

	vec3 reflected = normalize(reflect(normalize(viewPos), normalize(viewNormal)));
	vec3 hitPos = viewPos;
	float dDepth;

	vec3 worldPos = vec3(vec4(viewPos, 1.0) * invView);
	vec3 jitt = mix(vec3(0.0), vec3(hash(worldPos)), spec);

	vec4 coords = rayCast(normalize(jitt + reflected) * max(minRayStep, -viewPos.z), hitPos, dDepth);

	vec2 dCoords = smoothstep(0.2, 0.6, abs(vec2(0.5) - coords.xy));

	float screenEdgeFactor = clamp(1.0 - (dCoords.x + dCoords.y), 0.0, 1.0);

	float multiplier = screenEdgeFactor * -reflected.z;

	vec3 SSR = texture(colorTexture, coords.xy).rgb * clamp(multiplier, 0.0, 0.9);

	FragColor = vec4(SSR, 1.0);
}

vec4 rayCast(in vec3 dir, inout vec3 hitCoord, out float dDepth) {
	dir *= rayStep;

	float depth = 0.0;
	int steps = 0;
	vec4 projectedCoord = vec4(0.0);

	for (int i = 0; i < maxSteps; i++) {
		hitCoord += dir;
		
		projectedCoord = projection * vec4(hitCoord, 1.0);
		projectedCoord.xy /= projectedCoord.w;
		projectedCoord.xy = projectedCoord.xy * 0.5 + 0.5;

		depth = texture(positionTexture, projectedCoord.xy).z;

		if (depth > 1000.0) continue;

		dDepth = hitCoord.z - depth;
		if (dDepth <= 0.0 && (dir.z - dDepth) < maxRayStep) {
			vec4 result = vec4(binarySearch(dir, hitCoord, dDepth), 1.0);
			return result;
		}
		steps++;
	}
	if (depth < 1000.0) return vec4(0.0);

	return vec4(projectedCoord.xy, depth, 0.0);
}

vec3 binarySearch(inout vec3 dir, inout vec3 hitCoord, inout float dDepth) {
	float depth;
	vec4 projectedCoord;

	for (int i = 0; i < numBinarySearchSteps; i++) {
		projectedCoord = projection * vec4(hitCoord, 1.0);
		projectedCoord.xy /= projectedCoord.w;
		projectedCoord.xy = projectedCoord.xy * 0.5 + 0.5;

		depth = texture(positionTexture, projectedCoord.xy).z;

		dDepth = hitCoord.z - depth;

		dir *= 0.5;

		hitCoord += (dDepth > 0.0) ? dir : -dir;
	}

	projectedCoord = projection * vec4(hitCoord, 1.0);
	projectedCoord.xy /= projectedCoord.w;
	projectedCoord.xy = projectedCoord.xy * 0.5 + 0.5;

	return vec3(projectedCoord.xy, dDepth);
}

vec3 hash(vec3 a)
{
    a = fract(a * Scale);
    a += dot(a, a.yxz + K);
    return fract((a.xxy + a.yxx)*a.zyx);
}

```

## Source file: src/shader/shader.cpp

{
  "path": "src/shader/shader.cpp",
  "blob_sha": "87e1bde6a9bcfdbb2f25a642e44e142910eef182",
  "sha256": "fb283505a972fc8efaf3c6f3995d138d4be3b78eb2b3d284e5a7b4394e96917c",
  "lines": 234,
  "bytes": 7816,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 85
  },
  "url": "https://github.com/jperez01/scattering-demo/blob/49e00f2295027c4feae085f9bacf42a0bb34d309/src/shader/shader.cpp",
  "function_signature_candidates": [
    {
      "line": 109,
      "text": "void openAndLoadShaderFile(const string& fullPath, string& codeBuffer) {"
    }
  ]
}

```text

#include "shader/shader.h"

#include "utils/paths.h"

Shader::Shader() = default;

Shader::Shader(const char* computePath) {
    std::string finalComputePath = SHADER_PATH + computePath;
    std::string computeCode;
    try {
        openAndLoadShaderFile(finalComputePath, computeCode);
    } catch (ifstream::failure& e) {
        std::cout << "ERROR::SHADER::FILE_NOT_SUCCESFULLY_READ " << e.what() << std::endl;
    }

    unsigned int possibleComputeId = compileShader(GL_COMPUTE_SHADER, computeCode.c_str());
    setupProgram({possibleComputeId});

    computeId = possibleComputeId;

    shaderTypesAndPaths.emplace_back(GL_COMPUTE_SHADER, finalComputePath);
}

Shader::Shader(const char* vertexPath, const char* fragmentPath,
    const char* geoPath) {
    string finalVertexPath = SHADER_PATH + vertexPath;
    string finalFragmentPath = SHADER_PATH + fragmentPath;
    string finalGeometryPath;
    if (geoPath != nullptr) finalGeometryPath = SHADER_PATH + geoPath;

    string vertexCode;
    string fragmentCode;
    string geoCode;

    try {
        openAndLoadShaderFile(finalVertexPath, vertexCode);
        openAndLoadShaderFile(finalFragmentPath, fragmentCode);
        if (geoPath) {
            openAndLoadShaderFile(finalGeometryPath, geoCode);
        }
    } catch (ifstream::failure& e) {
        std::cout << "ERROR::SHADER::FILE_NOT_SUCCESFULLY_READ " << e.what() << std::endl;
    }

    const char* vShaderCode = vertexCode.c_str();
    const char* fShaderCode = fragmentCode.c_str();

    unsigned int vertex = 0, fragment = 0, geometry = 0;

    vertex = compileShader(GL_VERTEX_SHADER, vShaderCode);
    fragment = compileShader(GL_FRAGMENT_SHADER, fShaderCode);
    if (geoPath) {
        const char* gShaderCode = geoCode.c_str();
        geometry = compileShader(GL_GEOMETRY_SHADER, gShaderCode);
    }

    vector shaderIds{ vertex, fragment };
    if (geoPath) {
        shaderIds.push_back(geometry);
    }

    setupProgram(shaderIds);

    vertexId = vertex;
    fragmentId = fragment;
    geometryId = geometry;

    shaderTypesAndPaths = { {GL_VERTEX_SHADER, finalVertexPath}, { GL_FRAGMENT_SHADER, finalFragmentPath}};
    if (geoPath) {
        shaderTypesAndPaths.emplace_back(GL_GEOMETRY_SHADER, finalGeometryPath);
    }

}

void Shader::setupProgram(const vector<unsigned>& shaderIds) {
    GLint possibleId = glCreateProgram();
    for (auto& shaderId : shaderIds) {
        glAttachShader(possibleId, shaderId);
    }
    glLinkProgram(possibleId);

    int isLinked = 0;
    glGetProgramiv(possibleId, GL_LINK_STATUS, &isLinked);
    if (!isLinked)
    {
        GLint maxLength = 1;
        glGetProgramiv(possibleId, GL_INFO_LOG_LENGTH, &maxLength);

        std::vector<GLchar> errorLog(maxLength);
        glGetProgramInfoLog(possibleId, maxLength, &maxLength, &errorLog[0]);
        std::cout << "ERROR::PROGRAM_LINKING_ERROR of type: PROGRAM" << "\n" <<
            errorLog.data() << "\n";

        glDeleteProgram(possibleId);
        for (auto shaderId : shaderIds) {
            glDeleteShader(shaderId);
        }

        return;
    }

    for (auto shaderId : shaderIds) {
        glDetachShader(possibleId, shaderId);
    }

    ID = possibleId;
}

void openAndLoadShaderFile(const string& fullPath, string& codeBuffer) {
    ifstream shaderFile;
    shaderFile.exceptions(ifstream::failbit | ifstream::badbit);

    shaderFile.open(fullPath.c_str());
    stringstream shaderStream;
    shaderStream << shaderFile.rdbuf();
    shaderFile.close();

    codeBuffer = shaderStream.str();
}

unsigned compileShader(GLint shaderType, const char* codeBuffer) {
    unsigned int shaderId = glCreateShader(shaderType);
    glShaderSource(shaderId, 1, &codeBuffer, nullptr);
    glCompileShader(shaderId);

    const std::string shaderTypeString = shaderTypeToStringMap[shaderType];

    GLint isCompiled = 0;
    glGetShaderiv(shaderId, GL_COMPILE_STATUS, &isCompiled);

    if (!isCompiled) {
        GLint maxLength = 1;
        glGetShaderiv(shaderId, GL_INFO_LOG_LENGTH, &maxLength);

        std::vector<GLchar> errorLog(maxLength);
        glGetShaderInfoLog(shaderId, maxLength, &maxLength, &errorLog[0]);
        std::cout << "ERROR::SHADER_COMPILATION_ERROR of type: " << shaderTypeString << "\n" <<
            errorLog.data() << "\n";

        glDeleteShader(shaderId);
        return 0;
    }

    return shaderId;
}

void Shader::reloadShader(const char* codeBuffer, GLenum shaderType) {
    unsigned int newShaderId = compileShader(shaderType, codeBuffer);

    vector<unsigned int> shaderIds;
    if (shaderType == GL_VERTEX_SHADER) {
        shaderIds = { newShaderId, fragmentId};
        if (geometryId != 0) shaderIds.push_back(geometryId);
    } else if (shaderType == GL_FRAGMENT_SHADER) {
        shaderIds = {vertexId, newShaderId};
        if (geometryId != 0) shaderIds.push_back(geometryId);
    } else if (shaderType == GL_GEOMETRY_SHADER) {
        shaderIds = { vertexId, fragmentId, newShaderId};
    }

    setupProgram(shaderIds);

    if (shaderType == GL_VERTEX_SHADER) {
        vertexId = newShaderId;
    } else if (shaderType == GL_FRAGMENT_SHADER) {
        fragmentId = newShaderId;
    } else if (shaderType == GL_GEOMETRY_SHADER) {
        geometryId = newShaderId;
    }

    std::cout << "Reloaded Program with new Shader \n";
}


void Shader::use() const {
    glUseProgram(ID);
}

void Shader::setBool(const std::string &name, bool value) const
{         
    glUniform1i(glGetUniformLocation(ID, name.c_str()), (int)value); 
}
// ------------------------------------------------------------------------
void Shader::setInt(const std::string &name, int value) const
{ 
    glUniform1i(glGetUniformLocation(ID, name.c_str()), value); 
}
// ------------------------------------------------------------------------
void Shader::setFloat(const std::string &name, float value) const
{ 
    glUniform1f(glGetUniformLocation(ID, name.c_str()), value); 
}
// ------------------------------------------------------------------------
void Shader::setVec2(const std::string &name, const glm::vec2 &value) const
{ 
    glUniform2fv(glGetUniformLocation(ID, name.c_str()), 1, &value[0]); 
}
void Shader::setVec2(const std::string &name, float x, float y) const
{ 
    glUniform2f(glGetUniformLocation(ID, name.c_str()), x, y); 
}
// ------------------------------------------------------------------------
void Shader::setVec3(const std::string &name, const glm::vec3 &value) const
{ 
    glUniform3fv(glGetUniformLocation(ID, name.c_str()), 1, &value[0]); 
}
void Shader::setVec3(const std::string &name, float x, float y, float z) const
{ 
    glUniform3f(glGetUniformLocation(ID, name.c_str()), x, y, z); 
}
// ------------------------------------------------------------------------
void Shader::setVec4(const std::string &name, const glm::vec4 &value) const
{ 
    glUniform4fv(glGetUniformLocation(ID, name.c_str()), 1, &value[0]); 
}
void Shader::setVec4(const std::string &name, float x, float y, float z, float w)
{ 
    glUniform4f(glGetUniformLocation(ID, name.c_str()), x, y, z, w); 
}
// ------------------------------------------------------------------------
void Shader::setMat2(const std::string &name, const glm::mat2 &mat) const
{
    glUniformMatrix2fv(glGetUniformLocation(ID, name.c_str()), 1, GL_FALSE, &mat[0][0]);
}
// ------------------------------------------------------------------------
void Shader::setMat3(const std::string &name, const glm::mat3 &mat) const
{
    glUniformMatrix3fv(glGetUniformLocation(ID, name.c_str()), 1, GL_FALSE, &mat[0][0]);
}
// ------------------------------------------------------------------------
void Shader::setMat4(const std::string &name, const glm::mat4 &mat) const
{
    glUniformMatrix4fv(glGetUniformLocation(ID, name.c_str()), 1, GL_FALSE, &mat[0][0]);
}

```

## Source file: shaders/pbr/brdfF.glsl

{
  "path": "shaders/pbr/brdfF.glsl",
  "blob_sha": "41a540eb18bcb443cfb6046741d7383bb370ac67",
  "sha256": "1d29e8e74209832a9926456620b3a04eb3494b5bfd6f5eeccdbeb908baa9f631",
  "lines": 111,
  "bytes": 3055,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/jperez01/scattering-demo/blob/49e00f2295027c4feae085f9bacf42a0bb34d309/shaders/pbr/brdfF.glsl",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "float RadicalInverse_VdC(uint bits)"
    },
    {
      "line": 17,
      "text": "vec2 Hammersley(uint i, uint N)"
    },
    {
      "line": 22,
      "text": "vec3 ImportanceSampleGGX(vec2 Xi, vec3 N, float roughness)"
    },
    {
      "line": 45,
      "text": "float GeometrySchlickGGX(float NdotV, float roughness)"
    },
    {
      "line": 57,
      "text": "float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)"
    },
    {
      "line": 67,
      "text": "vec2 IntegrateBRDF(float NdotV, float roughness)"
    },
    {
      "line": 107,
      "text": "void main()"
    }
  ]
}

```text

#version 330 core
out vec2 FragColor;
in vec2 TexCoords;

const float PI = 3.14159265359;

float RadicalInverse_VdC(uint bits) 
{
     bits = (bits << 16u) | (bits >> 16u);
     bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
     bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
     bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
     bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
     return float(bits) * 2.3283064365386963e-10; // / 0x100000000
}

vec2 Hammersley(uint i, uint N)
{
	return vec2(float(i)/float(N), RadicalInverse_VdC(i));
}

vec3 ImportanceSampleGGX(vec2 Xi, vec3 N, float roughness)
{
	float a = roughness*roughness;
	
	float phi = 2.0 * PI * Xi.x;
	float cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y));
	float sinTheta = sqrt(1.0 - cosTheta*cosTheta);
	
	// from spherical coordinates to cartesian coordinates - halfway vector
	vec3 H;
	H.x = cos(phi) * sinTheta;
	H.y = sin(phi) * sinTheta;
	H.z = cosTheta;
	
	// from tangent-space H vector to world-space sample vector
	vec3 up          = abs(N.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
	vec3 tangent   = normalize(cross(up, N));
	vec3 bitangent = cross(N, tangent);
	
	vec3 sampleVec = tangent * H.x + bitangent * H.y + N * H.z;
	return normalize(sampleVec);
}

float GeometrySchlickGGX(float NdotV, float roughness)
{
    // note that we use a different k for IBL
    float a = roughness;
    float k = (a * a) / 2.0;

    float nom   = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return nom / denom;
}

float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = GeometrySchlickGGX(NdotV, roughness);
    float ggx1 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

vec2 IntegrateBRDF(float NdotV, float roughness)
{
    vec3 V;
    V.x = sqrt(1.0 - NdotV*NdotV);
    V.y = 0.0;
    V.z = NdotV;

    float A = 0.0;
    float B = 0.0; 

    vec3 N = vec3(0.0, 0.0, 1.0);
    
    const uint SAMPLE_COUNT = 1024u;
    for(uint i = 0u; i < SAMPLE_COUNT; ++i)
    {
        // generates a sample vector that's biased towards the
        // preferred alignment direction (importance sampling).
        vec2 Xi = Hammersley(i, SAMPLE_COUNT);
        vec3 H = ImportanceSampleGGX(Xi, N, roughness);
        vec3 L = normalize(2.0 * dot(V, H) * H - V);

        float NdotL = max(L.z, 0.0);
        float NdotH = max(H.z, 0.0);
        float VdotH = max(dot(V, H), 0.0);

        if(NdotL > 0.0)
        {
            float G = GeometrySmith(N, V, L, roughness);
            float G_Vis = (G * VdotH) / (NdotH * NdotV);
            float Fc = pow(1.0 - VdotH, 5.0);

            A += (1.0 - Fc) * G_Vis;
            B += Fc * G_Vis;
        }
    }
    A /= float(SAMPLE_COUNT);
    B /= float(SAMPLE_COUNT);
    return vec2(A, B);
}

void main() 
{
    vec2 integratedBRDF = IntegrateBRDF(TexCoords.x, TexCoords.y);
    FragColor = integratedBRDF;
}

```

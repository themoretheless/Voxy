# Facial rendering source scan: azaleostu/sss

Repository: https://github.com/azaleostu/sss

Pinned source revision: 2569d303c882ba74ba4fe6aa5676e30084df9c8d

Discovery topic: subsurface scattering

Description: Subsurface Scattering.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["assets/maps/skinLookup.png", "assets/models/james/textures/james_skin_params.png", "assets/models/james/textures/skin_spec.dds", "assets/models/james/textures/skin_stretch.dds"], "sss": ["assets/shaders/sss-blur.frag", "assets/shaders/sss-blur.vert", "notes/SSS Techniques.docx"], "shader": ["assets/shaders/cube-map.vert", "assets/shaders/env-to-cube-map.frag", "assets/shaders/env-to-cube-map.vert", "assets/shaders/final-output.frag", "assets/shaders/g-buffer.frag", "assets/shaders/g-buffer.vert", "assets/shaders/main.frag", "assets/shaders/quad.vert", "assets/shaders/shadow.frag", "assets/shaders/shadow.vert", "assets/shaders/sky-box.frag", "assets/shaders/sss-blur.frag", "assets/shaders/sss-blur.vert", "src/shader/CMakeLists.txt", "src/shader/Shader.cpp", "src/shader/Shader.h", "src/shader/ShaderProgram.cpp", "src/shader/ShaderProgram.h"]}

## Source file: src/shader/Shader.h

{
  "path": "src/shader/Shader.h",
  "blob_sha": "6aa0d366164fb153b8b5dbb09da1100083d11ed5",
  "sha256": "c1e84691b68d133da08ede08119d06feb4a85e63218cef0350fd0fc3af35b552",
  "lines": 28,
  "bytes": 378,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 4,
    "shader": 6
  },
  "url": "https://github.com/azaleostu/sss/blob/2569d303c882ba74ba4fe6aa5676e30084df9c8d/src/shader/Shader.h",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "bool compile();"
    },
    {
      "line": 18,
      "text": "void release();"
    }
  ]
}

```text

#pragma once
#ifndef SSS_SHADER_SHADER_H
#define SSS_SHADER_SHADER_H

#include <glad/glad.h>
#include <string>

namespace sss {

class Shader {
public:
  Shader() = delete;
  Shader(const char* sourceStr, GLenum type);

  ~Shader() { release(); }

  bool compile();
  void release();

  GLuint id() const { return m_id; }

private:
  GLuint m_id;
};

} // namespace sss

#endif


```

## Source file: src/shader/Shader.cpp

{
  "path": "src/shader/Shader.cpp",
  "blob_sha": "0984c9666d492fb592263633c26a03c577172015",
  "sha256": "cee3642870c82a5cf3181c10d70b24dd252eab22af3abe24db63608a98f4bac7",
  "lines": 33,
  "bytes": 722,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 2,
    "shader": 12
  },
  "url": "https://github.com/azaleostu/sss/blob/2569d303c882ba74ba4fe6aa5676e30084df9c8d/src/shader/Shader.cpp",
  "function_signature_candidates": []
}

```text

#include "Shader.h"

namespace sss {

Shader::Shader(const char* sourceStr, GLenum type)
  : m_id(glCreateShader(type)) {
  glShaderSource(m_id, 1, &sourceStr, nullptr);
}

bool Shader::compile() {
  glCompileShader(m_id);

  GLint compiled;
  glGetShaderiv(m_id, GL_COMPILE_STATUS, &compiled);
  if (compiled)
    return true;

  GLint infoLogLength;
  glGetShaderiv(m_id, GL_INFO_LOG_LENGTH, &infoLogLength);
  auto* infoLog = new GLchar[infoLogLength];
  glGetShaderInfoLog(m_id, infoLogLength, nullptr, infoLog);
  std::cout << "Failed to compile shader:\n" << infoLog << std::endl;
  release();
  delete[] infoLog;
  return false;
}

void Shader::release() {
  glDeleteShader(m_id);
  m_id = 0;
}

} // namespace sss


```

## Source file: src/shader/ShaderProgram.h

{
  "path": "src/shader/ShaderProgram.h",
  "blob_sha": "f5f99a1cd73db60407ee98d6913f696956fb2e87",
  "sha256": "2f710579054aa6543ae51d0e31ebef968f49ae94b936f97a80cd83cbdaf167bb",
  "lines": 57,
  "bytes": 1512,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 4,
    "shader": 8
  },
  "url": "https://github.com/azaleostu/sss/blob/2569d303c882ba74ba4fe6aa5676e30084df9c8d/src/shader/ShaderProgram.h",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "void init();"
    },
    {
      "line": 23,
      "text": "bool addShader(GLenum type, const std::string& file);"
    },
    {
      "line": 24,
      "text": "bool link();"
    },
    {
      "line": 25,
      "text": "void release();"
    },
    {
      "line": 27,
      "text": "bool initVertexFragment(const std::string& vertexPath, const std::string& fragmentPath);"
    },
    {
      "line": 29,
      "text": "void use() const;"
    },
    {
      "line": 34,
      "text": "void setBool(GLint loc, bool value) const;"
    },
    {
      "line": 35,
      "text": "void setInt(GLint loc, int value) const;"
    },
    {
      "line": 36,
      "text": "void setFloat(GLint loc, float value) const;"
    },
    {
      "line": 37,
      "text": "void setVec2(GLint loc, const glm::vec2& value) const;"
    },
    {
      "line": 38,
      "text": "void setVec2(GLint loc, float x, float y) const;"
    },
    {
      "line": 39,
      "text": "void setVec3(GLint loc, const glm::vec3& value) const;"
    },
    {
      "line": 40,
      "text": "void setVec3(GLint loc, float x, float y, float z) const;"
    },
    {
      "line": 41,
      "text": "void setVec4(GLint loc, const glm::vec4& value) const;"
    },
    {
      "line": 42,
      "text": "void setVec4(GLint loc, float x, float y, float z, float w) const;"
    },
    {
      "line": 43,
      "text": "void setMat2(GLint loc, const glm::mat2& mat) const;"
    },
    {
      "line": 44,
      "text": "void setMat3(GLint loc, const glm::mat3& mat) const;"
    },
    {
      "line": 45,
      "text": "void setMat4(GLint loc, const glm::mat4& mat) const;"
    },
    {
      "line": 46,
      "text": "void setVec4Array(const std::string& uniformName, const std::vector<glm::vec4>& mat) const;"
    }
  ]
}

```text

#pragma once
#ifndef SSS_SHADER_SHADERPROGRAM_H
#define SSS_SHADER_SHADERPROGRAM_H

#include "Shader.h"

#include <glad/glad.h>
#include <glm/glm.hpp>
#include <string>
#include <utility>
#include <vector>
#include <array>

namespace sss {

class ShaderProgram {
public:
  ~ShaderProgram() { release(); }

  GLuint id() const { return m_id; }

  void init();
  bool addShader(GLenum type, const std::string& file);
  bool link();
  void release();

  bool initVertexFragment(const std::string& vertexPath, const std::string& fragmentPath);

  void use() const;

public:
  GLint getUniformLocation(const char* name) const;

  void setBool(GLint loc, bool value) const;
  void setInt(GLint loc, int value) const;
  void setFloat(GLint loc, float value) const;
  void setVec2(GLint loc, const glm::vec2& value) const;
  void setVec2(GLint loc, float x, float y) const;
  void setVec3(GLint loc, const glm::vec3& value) const;
  void setVec3(GLint loc, float x, float y, float z) const;
  void setVec4(GLint loc, const glm::vec4& value) const;
  void setVec4(GLint loc, float x, float y, float z, float w) const;
  void setMat2(GLint loc, const glm::mat2& mat) const;
  void setMat3(GLint loc, const glm::mat3& mat) const;
  void setMat4(GLint loc, const glm::mat4& mat) const;
  void setVec4Array(const std::string& uniformName, const std::vector<glm::vec4>& mat) const;

private:
  static std::string s_shadersDir;

  GLuint m_id = GL_INVALID_INDEX;
  std::vector<Shader> m_shaders;
};

} // namespace sss

#endif


```

## Source file: src/shader/ShaderProgram.cpp

{
  "path": "src/shader/ShaderProgram.cpp",
  "blob_sha": "44415c640b0db82fe55042a4ab865a897ad69fbf",
  "sha256": "eaafde52a30ba829279f3916b06f9a166ca00a31749c4fe3d4d06e6bb7ef16ff",
  "lines": 109,
  "bytes": 3349,
  "selection_score": 15,
  "keyword_line_counts": {
    "sss": 3,
    "shader": 27
  },
  "url": "https://github.com/azaleostu/sss/blob/2569d303c882ba74ba4fe6aa5676e30084df9c8d/src/shader/ShaderProgram.cpp",
  "function_signature_candidates": []
}

```text

#include "ShaderProgram.h"
#include "../utils/ReadFile.h"

#include <glm/gtc/type_ptr.hpp>

namespace sss {

void ShaderProgram::init() { m_id = glCreateProgram(); }

bool ShaderProgram::addShader(GLenum type, const std::string& file) {
  std::string source;
  if (!sss::readFile(s_shadersDir + file, source))
    return false;

  m_shaders.emplace_back(source.c_str(), type);
  if (!m_shaders.back().compile()) {
    m_shaders.pop_back();
    return false;
  }
  glAttachShader(m_id, m_shaders.back().id());
  return true;
}

bool ShaderProgram::link() {
  glLinkProgram(m_id);

  GLint linked;
  glGetProgramiv(m_id, GL_LINK_STATUS, &linked);
  if (linked)
    return true;

  GLint infoLogLength;
  glGetProgramiv(m_id, GL_INFO_LOG_LENGTH, &infoLogLength);
  auto* infoLog = new GLchar[infoLogLength];
  glGetProgramInfoLog(m_id, infoLogLength, nullptr, infoLog);
  std::cout << "Failed to link program:\n" << infoLog << std::endl;
  release();
  delete[] infoLog;
  return false;
}

void ShaderProgram::release() {
  glDeleteProgram(m_id);
  m_id = 0;
}

bool ShaderProgram::initVertexFragment(const std::string& vertexPath,
                                       const std::string& fragmentPath) {
  init();
  if (!addShader(GL_VERTEX_SHADER, vertexPath) || !addShader(GL_FRAGMENT_SHADER, fragmentPath))
    return false;

  return link();
}

void ShaderProgram::use() const { glUseProgram(m_id); }

GLint ShaderProgram::getUniformLocation(const char* name) const {
  return glGetUniformLocation(m_id, name);
}

void ShaderProgram::setBool(GLint loc, bool value) const {
  glProgramUniform1i(m_id, loc, (int)value);
}

void ShaderProgram::setInt(GLint loc, int value) const { glProgramUniform1i(m_id, loc, value); }
void ShaderProgram::setFloat(GLint loc, float value) const { glProgramUniform1f(m_id, loc, value); }

void ShaderProgram::setVec2(GLint loc, const glm::vec2& value) const {
  glProgramUniform2fv(m_id, loc, 1, glm::value_ptr(value));
}
void ShaderProgram::setVec2(GLint loc, float x, float y) const {
  glProgramUniform2f(m_id, loc, x, y);
}

void ShaderProgram::setVec3(GLint loc, const glm::vec3& value) const {
  glProgramUniform3fv(m_id, loc, 1, glm::value_ptr(value));
}
void ShaderProgram::setVec3(GLint loc, float x, float y, float z) const {
  glProgramUniform3f(m_id, loc, x, y, z);
}

void ShaderProgram::setVec4(GLint loc, const glm::vec4& value) const {
  glProgramUniform4fv(m_id, loc, 1, glm::value_ptr(value));
}
void ShaderProgram::setVec4(GLint loc, float x, float y, float z, float w) const {
  glProgramUniform4f(m_id, loc, x, y, z, w);
}

void ShaderProgram::setMat2(GLint loc, const glm::mat2& mat) const {
  glProgramUniformMatrix2fv(m_id, loc, 1, GL_FALSE, glm::value_ptr(mat));
}

void ShaderProgram::setMat3(GLint loc, const glm::mat3& mat) const {
  glProgramUniformMatrix3fv(m_id, loc, 1, GL_FALSE, glm::value_ptr(mat));
}

void ShaderProgram::setMat4(GLint loc, const glm::mat4& mat) const {
  glProgramUniformMatrix4fv(m_id, loc, 1, GL_FALSE, glm::value_ptr(mat));
}

void ShaderProgram::setVec4Array(const std::string& uniformName, const std::vector<glm::vec4>& mat) const {
  for (int i = 0; i < mat.size(); ++i) {
    glProgramUniform4fv(m_id,
                        getUniformLocation((uniformName + "[" + std::to_string(i) + "]").c_str()), 1, glm::value_ptr(mat[i]));
  }
}

} // namespace sss


```

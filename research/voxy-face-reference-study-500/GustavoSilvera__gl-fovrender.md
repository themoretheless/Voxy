# Facial rendering source scan: GustavoSilvera/gl-fovrender

Repository: https://github.com/GustavoSilvera/gl-fovrender

Pinned source revision: d9a76812d84ecc0ed0c2ab3d22b4c4ea9afd2e7d

Discovery topic: eye shader

Description: OpenGL shader playground with fixed/mouse/eye-based foveated rendering. Final project for 15-469 Visual Computing Systems course at CMU.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["src/shader_utils.cpp", "src/shader_utils.h", "src/shaders/README.md", "src/shaders/fov_render_frag.glsl", "src/shaders/main/example.glsl", "src/shaders/non_fr_frag.glsl", "src/shaders/reconstruction_shader.glsl", "src/shaders/vertex_shader.glsl"]}

## Source file: src/shaders/vertex_shader.glsl

{
  "path": "src/shaders/vertex_shader.glsl",
  "blob_sha": "10d3d882ffd586f69b09fc1ec0f89085da25fe35",
  "sha256": "69c57e00772ff6ed5ebeb84a6a0ffc493e878ecb73d47f57eaa5952ff7c8bce6",
  "lines": 12,
  "bytes": 277,
  "selection_score": 27,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/GustavoSilvera/gl-fovrender/blob/d9a76812d84ecc0ed0c2ab3d22b4c4ea9afd2e7d/src/shaders/vertex_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "void main()"
    }
  ]
}

```text

#version 330 core
// a very simple vertex shader passing primitives to the fragment shader

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 inTexCoord;

out vec2 texCoord;
void main()
{
    texCoord = inTexCoord;
    gl_Position = vec4(position.xyz, 1.0);
}

```

## Source file: src/shaders/reconstruction_shader.glsl

{
  "path": "src/shaders/reconstruction_shader.glsl",
  "blob_sha": "9e93e1a0f677556f7e3bc07c37b9490cdc760cce",
  "sha256": "dd662613b54824ff456987f84bde5314f7b30268ef8551d92307c24a4efc736a",
  "lines": 146,
  "bytes": 6178,
  "selection_score": 27,
  "keyword_line_counts": {
    "shader": 1,
    "rig": 12
  },
  "url": "https://github.com/GustavoSilvera/gl-fovrender/blob/d9a76812d84ecc0ed0c2ab3d22b4c4ea9afd2e7d/src/shaders/reconstruction_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "float norm2(const vec2 a)"
    },
    {
      "line": 28,
      "text": "float sqr(const float a)"
    },
    {
      "line": 33,
      "text": "void main()"
    }
  ]
}

```text

#version 330 core

layout(location = 0) out vec4 fragColor;
uniform vec2 iResolution;
uniform float iTime;
uniform vec2 iMouse;
uniform vec2 Mouse;
uniform int iFrame;

in vec2 texCoord;
uniform sampler2D tex;

// foveated render vars
uniform int stride;
uniform float thresh1; // smallest foveal region
uniform float thresh2; // middle region
uniform float thresh3; // far region

// constant vars
int quad = stride / 2; // how wide the group of dropped pixels is
const vec4 clear = vec4(0, 0, 0, 1);

float norm2(const vec2 a)
{
    return dot(a, a);
}

float sqr(const float a)
{
    return a * a;
}

void main()
{
    vec2 coord = gl_FragCoord.xy - 0.5; // top left corner of pixel

    // which quad am on?
    float xmod = mod(coord.x, stride);
    float ymod = mod(coord.y, stride);

    // compute (boxy) distance to foveal region
    vec2 boxy_coord = floor(coord / stride) * stride;
    vec2 center = floor(vec2(Mouse.x, -Mouse.y + iResolution.y) / stride) * stride;
    float d2 = norm2(boxy_coord - center);

    // assume equal weights, though these change depending on interpolation
    float weight_x = 0.5;
    float weight_y = 0.5;

    if (xmod < quad && ymod < quad) // top left
    {
        // no op (always rendered in full in frag shader)
        discard;
    }
    else if (xmod < quad && ymod >= quad && d2 < sqr(thresh3)) // top right
    {
        if (d2 > sqr(thresh1))
        {
            weight_x = xmod / quad;          // positive is right
            weight_y = (ymod - quad) / quad; // positive is up
            fragColor = vec4(0.0);
            // always accumulate vertical pixels for interp
            fragColor += weight_y * texelFetch(tex, ivec2(coord.x, coord.y + stride - ymod), 0);           // top
            fragColor += (1.0 - weight_y) * texelFetch(tex, ivec2(coord.x, coord.y - ymod + quad - 1), 0); // bottom
            // usually accumulate horizontal pixels for interp, but not if left is unfilled
            vec4 left_colour = texelFetch(tex, ivec2(coord.x - xmod - 1, coord.y), 0);
            if (left_colour.rgb != vec3(0)) // needs to be coloured somewhat
            {
                // as long as left is good, use it for more data
                fragColor += weight_x * texelFetch(tex, ivec2(coord.x + quad - xmod, coord.y), 0); // right
                fragColor += (1.0 - weight_x) * left_colour;                                       // left
                fragColor /= 2; // average between x data and y data
            }
        }
        else
            discard;
    }
    else if (xmod >= quad && ymod < quad && d2 < sqr(thresh3)) // bottom left
    {
        if (d2 > sqr(thresh2))
        {
            weight_x = (xmod - quad) / quad; // positive is right
            weight_y = ymod / quad;          // positive is up
            fragColor = vec4(0.0);
            // always accumulate left/right data for interp
            fragColor += (1.0 - weight_x) * texelFetch(tex, ivec2(coord.x - xmod + quad - 1, coord.y), 0); // left
            fragColor += weight_x * texelFetch(tex, ivec2(coord.x + stride - xmod, coord.y), 0);           // right
            // usually accumulate vertical pixels for interp, but not if bottom is unfilled
            vec4 bottom_colour = texelFetch(tex, ivec2(coord.x, coord.y - ymod - 1), 0);
            if (bottom_colour.rgb != vec3(0)) // needs to be coloured somewhat
            {
                fragColor += weight_y * texelFetch(tex, ivec2(coord.x, coord.y + quad - ymod), 0); // top
                fragColor += (1.0 - weight_y) * bottom_colour;                                     // bottom
                fragColor /= 2; // average between x data and y data
            }
        }
        else
            discard;
    }
    else // bottom right
    {
        if (d2 >= sqr(thresh3))
        {
            fragColor = vec4(0.0);
            // need to case for horizontal, vertical, or diagonal bilinear interp.
            if (xmod < quad)
            {
                // case 1: vertical bilinear interpolation
                weight_y = (ymod - quad) / quad; // positive is up

                fragColor += weight_y * texelFetch(tex, ivec2(coord.x, coord.y + stride - ymod), 0);           // top
                fragColor += (1.0 - weight_y) * texelFetch(tex, ivec2(coord.x, coord.y - ymod + quad - 1), 0); // bottom
            }
            else
            {
                weight_x = (xmod - quad) / quad; // positive is right
                if (ymod < quad)
                {
                    // case 2: horizontal bilinear interpolation
                    fragColor += weight_x * texelFetch(tex, ivec2(coord.x + stride - xmod, coord.y), 0);           // R
                    fragColor += (1.0 - weight_x) * texelFetch(tex, ivec2(coord.x - xmod + quad - 1, coord.y), 0); // L
                }
                else
                {
                    // case 3: diagonal trilinear interpolation
                    weight_y = (ymod - quad) / quad;                        // positive is up
                    float weight_xy1 = (1.0 - weight_y) * weight_x;         // positive is bottom right
                    float weight_xy2 = weight_y * (1.0 - weight_x);         // positive is top left
                    float weight_xy3 = weight_y * weight_x;                 // positive is top right
                    float weight_xy4 = (1.0 - weight_y) * (1.0 - weight_x); // positive is bottom left

                    fragColor += // bottom right
                        weight_xy1 * texelFetch(tex, ivec2(coord.x + stride - xmod, coord.y - ymod + quad - 1), 0);
                    fragColor += // top left
                        weight_xy2 * texelFetch(tex, ivec2(coord.x - xmod + quad - 1, coord.y + stride - ymod), 0);
                    fragColor += // top right
                        weight_xy3 * texelFetch(tex, ivec2(coord.x + stride - xmod, coord.y + stride - ymod), 0);
                    fragColor += // bottom left
                        weight_xy4 * texelFetch(tex, ivec2(coord.x - xmod + quad - 1, coord.y - ymod + quad - 1), 0);
                }
            }
        }
        else
            discard;
    }
}

```

## Source file: src/shader_utils.h

{
  "path": "src/shader_utils.h",
  "blob_sha": "46e2ce9425deca1e38404f8d06e9a1d6e92389bc",
  "sha256": "07de1a854fc3aa82fe8c5ca8ade0bc3c2e239d94240eb51466d27cf6f788ea2c",
  "lines": 60,
  "bytes": 1179,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 18
  },
  "url": "https://github.com/GustavoSilvera/gl-fovrender/blob/d9a76812d84ecc0ed0c2ab3d22b4c4ea9afd2e7d/src/shader_utils.h",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "bool registerShader(Shader &S);"
    },
    {
      "line": 31,
      "text": "bool registerProgram();"
    },
    {
      "line": 32,
      "text": "void DeleteShaders();"
    },
    {
      "line": 40,
      "text": "bool loadShaders(const std::vector<Shader> &shaders);"
    },
    {
      "line": 41,
      "text": "bool Reload();"
    },
    {
      "line": 42,
      "text": "int GetProgram() const;"
    },
    {
      "line": 52,
      "text": "bool loadShaders(const ParamsStruct &P);"
    },
    {
      "line": 53,
      "text": "bool Reload(const ParamsStruct &P);"
    },
    {
      "line": 54,
      "text": "bool NextShader(const ParamsStruct &P);"
    },
    {
      "line": 55,
      "text": "bool PrevShader(const ParamsStruct &P);"
    }
  ]
}

```text

/// NOTE: this code was modified from https://github.com/k0pernicus/opengl-explorer

#ifndef _SHADER_UTILS_H
#define _SHADER_UTILS_H

#include "utils.h"
#include <string>
#include <vector>

namespace ShaderUtils
{

struct Shader
{
    Shader(const std::string &f, const std::string &t, const int ty) : file_path(f), title(t), type(ty), ShaderID(-1)
    {
    }
    std::string file_path = "";
    std::string title = "shader";
    int type;
    int ShaderID = -1; // -1 for unregistered
};

struct Program
{

  protected:
    int program = 0;

    bool registerShader(Shader &S);
    bool registerProgram();
    void DeleteShaders();

    std::vector<Shader> Shaders;

  public:
    Program();
    ~Program();

    bool loadShaders(const std::vector<Shader> &shaders);
    bool Reload();
    int GetProgram() const;
};

struct MainProgram : Program
{
  private:
    std::vector<std::string> OtherShaderPaths;
    size_t ShaderIdx = 0;

  public:
    bool loadShaders(const ParamsStruct &P);
    bool Reload(const ParamsStruct &P);
    bool NextShader(const ParamsStruct &P);
    bool PrevShader(const ParamsStruct &P);
};

} // namespace ShaderUtils

#endif /* SHADER_UTILS_H */


```

## Source file: src/shader_utils.cpp

{
  "path": "src/shader_utils.cpp",
  "blob_sha": "3e31e8380d5d86c3293ebe5e265175a8ccf1af4c",
  "sha256": "315835d3caa3ca7c55ceaf65c0c63178310d481692720dba12bba4b16f3a0cd7",
  "lines": 170,
  "bytes": 4506,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 54
  },
  "url": "https://github.com/GustavoSilvera/gl-fovrender/blob/d9a76812d84ecc0ed0c2ab3d22b4c4ea9afd2e7d/src/shader_utils.cpp",
  "function_signature_candidates": []
}

```text

/// NOTE: this code was modified from https://github.com/k0pernicus/opengl-explorer

#ifdef __APPLE__
/* Defined before OpenGL and GLUT includes to avoid deprecation messages */
#define GL_SILENCE_DEPRECATION
#include <GLFW/glfw3.h>
#endif

#include "shader_utils.h"
#include "utils.h"
#include <filesystem>
#include <iostream>
#include <optional>

namespace ShaderUtils
{
Program::Program()
{
}

Program::~Program()
{
    glDeleteProgram(program);
}

bool Program::Reload()
{
    // delete the current program
    glDeleteProgram(GetProgram());

    // load Shaders new
    return loadShaders(Shaders);
}

bool Program::loadShaders(const std::vector<Shader> &ShaderStructList)
{
    std::cout << std::endl;
    Shaders = ShaderStructList;
    for (auto &ShaderStruct : Shaders)
    {
        if (!registerShader(ShaderStruct))
        {
            std::cerr << "failed to register the " << ShaderStruct.title << " shader..." << std::endl;
            return false;
        }
    }
    if (!registerProgram())
    {
        std::cerr << "failed to register the program..." << std::endl;
        return false;
    }
    return true;
}

bool Program::registerShader(Shader &S)
{
    // first read the file
    const std::string shader_src_str = readFile(S.file_path);
    const char *shader_source = shader_src_str.c_str();

    // then create the shader and compile
    int success = {};
    char errorMessage[1024] = {};
    int shader = glCreateShader(S.type);
    glShaderSource(shader, 1, &shader_source, NULL);
    glCompileShader(shader);

    // ensure shader compilation
    glGetShaderiv(shader, GL_COMPILE_STATUS, &success);
    if (!success)
    {
        glGetShaderInfoLog(shader, 1024, NULL, errorMessage);
        std::cerr << "Shader compilation error : " << errorMessage << std::endl;
        return false;
    }
    S.ShaderID = shader;
    return true;
}

bool Program::registerProgram()
{
    int success = {};
    char errorMessage[1024] = {};

    program = glCreateProgram();

    for (auto &Shader : Shaders)
    {
        glAttachShader(program, Shader.ShaderID);
    }
    glLinkProgram(program);

    glGetProgramiv(program, GL_LINK_STATUS, &success);
    if (!success)
    {
        glGetProgramInfoLog(program, 1024, NULL, errorMessage);
        std::cerr << "Shader linking error: " << errorMessage << std::endl;
        return false;
    }

    // We can now delete our other shaders
    DeleteShaders();
    return true;
}

int Program::GetProgram() const
{
    return program;
}

void Program::DeleteShaders()
{
    for (auto &Shader : Shaders)
    {
        glDeleteShader(Shader.ShaderID);
    }
}

bool MainProgram::loadShaders(const ParamsStruct &P)
{
    Shaders.clear();
    Shaders = {
        ShaderUtils::Shader(P.MainParams.vertex_shader_path, "vertex", GL_VERTEX_SHADER),
        ShaderUtils::Shader(P.MainParams.fragment_shader_dir + P.MainParams.fragment_shader_name, "main",
                            GL_FRAGMENT_SHADER),
        ShaderUtils::Shader(P.bEnableFovRender ? P.FRParams.drop_shader : P.MainParams.non_fr_fragment_shader_path,
                            "fragment", GL_FRAGMENT_SHADER),
    };
    // read all the shaders in the FragmentShaderPath
    for (const auto &file : std::filesystem::directory_iterator(P.MainParams.fragment_shader_dir))
    {
        std::string path = file.path();
        std::cout << "Found shader: \"" << path << "\"" << std::endl;
        OtherShaderPaths.push_back(path);
    }
    std::cout << std::endl;

    // call parent load
    return Program::loadShaders(Shaders);
}

bool MainProgram::Reload(const ParamsStruct &P)
{
    // expects an up-to-date ParamsStruct instance
    if (OtherShaderPaths.size() > 0)
    {
        Shaders.clear();
        Shaders = {
            ShaderUtils::Shader(P.MainParams.vertex_shader_path, "vertex", GL_VERTEX_SHADER),
            ShaderUtils::Shader(OtherShaderPaths[ShaderIdx], "main", GL_FRAGMENT_SHADER),
            ShaderUtils::Shader(P.bEnableFovRender ? P.FRParams.drop_shader : P.MainParams.non_fr_fragment_shader_path,
                                "fragment", GL_FRAGMENT_SHADER),
        };
    }
    // call parent reload
    return Program::Reload();
}

bool MainProgram::NextShader(const ParamsStruct &P)
{
    ShaderIdx = (ShaderIdx + 1) % OtherShaderPaths.size();
    return Reload(P);
}

bool MainProgram::PrevShader(const ParamsStruct &P)
{
    ShaderIdx = (ShaderIdx - 1) % OtherShaderPaths.size();
    return Reload(P);
}
}; // namespace ShaderUtils

```

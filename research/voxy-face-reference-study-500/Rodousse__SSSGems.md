# Facial rendering source scan: Rodousse/SSSGems

Repository: https://github.com/Rodousse/SSSGems

Pinned source revision: 8e4fa443248b81348838eca30b493f362bcd3637

Discovery topic: subsurface scattering

Description: GPU Gems subsurface scattering implementation

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["resources/shaders/fragment.frag", "resources/shaders/fragmentDepth.frag", "resources/shaders/vertex.vert", "resources/shaders/vertexDepth.vert", "src/utils/utils/ShaderUtils.cpp", "src/utils/utils/ShaderUtils.h"]}

## Source file: src/utils/utils/ShaderUtils.h

{
  "path": "src/utils/utils/ShaderUtils.h",
  "blob_sha": "9e2fb71ebd0fb32635d693771fe900ca822d0bff",
  "sha256": "fcb7cc3c5960afec495d018fd70ba751d0117df298f3da13acb71ad15b59d8b4",
  "lines": 11,
  "bytes": 129,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 2
  },
  "url": "https://github.com/Rodousse/SSSGems/blob/8e4fa443248b81348838eca30b493f362bcd3637/src/utils/utils/ShaderUtils.h",
  "function_signature_candidates": []
}

```text

#pragma once

#include <string>
#include <vector>

namespace shader
{

std::vector<char> loadShader(const std::string& path);

}


```

## Source file: src/utils/utils/ShaderUtils.cpp

{
  "path": "src/utils/utils/ShaderUtils.cpp",
  "blob_sha": "5476a44914b2904e93a51947b6410309e8a62160",
  "sha256": "3c0d9b842ea99679fc3a93fc769f5d8ae1997b15887a165a088be1f91a6f4fea",
  "lines": 31,
  "bytes": 618,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 8
  },
  "url": "https://github.com/Rodousse/SSSGems/blob/8e4fa443248b81348838eca30b493f362bcd3637/src/utils/utils/ShaderUtils.cpp",
  "function_signature_candidates": []
}

```text

#include "ShaderUtils.h"
#include <fstream>
#include <algorithm>
#include <exception>

namespace shader
{

std::vector<char> loadShader(const std::string& path)
{
    std::ifstream file;
    file.open(path, std::ios::ate | std::ios::binary);

    if(!file.is_open())
    {
        throw std::runtime_error("[SHADER UTILS] : Can't open shader : " + path);
    }

    const std::streamsize fileSize = file.tellg();

    std::vector<char> shaderData;
    shaderData.resize(static_cast<std::size_t>(fileSize));

    file.seekg(0);
    file.read(shaderData.data(), fileSize);
    file.close();

    return shaderData;
}

}


```

## Source file: src/app/main.cpp

{
  "path": "src/app/main.cpp",
  "blob_sha": "4a847c7303f27be6b3427e3963d7b993cb741780",
  "sha256": "98b5ed33cf3e45c8a9c1466cb900a0e984b80006ed959461295ea27f1ecaa83b",
  "lines": 340,
  "bytes": 12105,
  "selection_score": 0,
  "keyword_line_counts": {
    "sss": 1,
    "shader": 49,
    "face": 2
  },
  "url": "https://github.com/Rodousse/SSSGems/blob/8e4fa443248b81348838eca30b493f362bcd3637/src/app/main.cpp",
  "function_signature_candidates": [
    {
      "line": 38,
      "text": "void actualizeProjection(glm::mat4& projMat, const WindowData& window);"
    },
    {
      "line": 39,
      "text": "void actualizeView(glm::mat4& viewMat);"
    },
    {
      "line": 40,
      "text": "void resizeCallback(GLFWwindow* win, int width, int height);"
    },
    {
      "line": 42,
      "text": "int main()"
    },
    {
      "line": 313,
      "text": "void actualizeProjection(glm::mat4& projMat, const WindowData& data)"
    },
    {
      "line": 319,
      "text": "void actualizeView(glm::mat4& viewMat)"
    },
    {
      "line": 334,
      "text": "void resizeCallback(GLFWwindow*, int width, int height)"
    }
  ]
}

```text

#include <array>
#include <chrono>
#include <defines.h>
#include <iostream>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>
#include <GL/glew.h>
#include <GLFW/glfw3.h>
#include "utils/ObjLoader.h"
#include "utils/ShaderUtils.h"

#define SHADOW_RES 4096


struct UniformData
{
    glm::mat4 proj;
    glm::mat4 view;
    glm::mat4 projView;
};

struct WindowData
{
    GLFWwindow* window;
    int width;
    int height;
    bool resized;
} winData;

struct DirectionnalLightInfo
{
    glm::vec3 dir;
    glm::mat4 biasProjView;
    UniformData ubo;
    float size;
};

void actualizeProjection(glm::mat4& projMat, const WindowData& window);
void actualizeView(glm::mat4& viewMat);
void resizeCallback(GLFWwindow* win, int width, int height);

int main()
{
    if(!glfwInit())
    {
        std::cerr << "ERROR: could not start GLFW" << std::endl;
        return 1;
    }

    winData.window = glfwCreateWindow(640, 480, "SSSGems", nullptr, nullptr);
    winData.width = 640;
    winData.height = 480;

    glfwSetWindowSizeCallback(winData.window, resizeCallback);

    if(!winData.window)
    {
        std::cerr << "ERROR: could not open window with GLFW3" << std::endl;
        glfwTerminate();
        return 1;
    }

    glfwMakeContextCurrent(winData.window);

    if(glewInit() != GLEW_OK)
    {
        std::cerr << "ERROR: could not start GLEW" << std::endl;
        return 1;
    }

    const GLubyte* renderer = glGetString(GL_RENDERER);
    const GLubyte* version = glGetString(GL_VERSION);
    std::cout << "Renderer: " <<  renderer << std::endl;
    std::cout << "OpenGL version supported " <<  version << std::endl;

    Mesh mesh;

    if(!loader::load(std::string(RESOURCE_PATH) + "/models/bunny.obj", mesh))
    {
        throw std::runtime_error("Can't load file : " + std::string(RESOURCE_PATH) + "/models/bunny.obj");
    }

    GLuint vbo{};
    glGenBuffers(1, &vbo);
    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glBufferData(GL_ARRAY_BUFFER, mesh.vertices.size() * sizeof(Vertex), mesh.vertices.data(),
                 GL_STATIC_DRAW);

    GLuint ebo{};
    glGenBuffers(1, &ebo);
    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo);
    glBufferData(GL_ELEMENT_ARRAY_BUFFER, mesh.indices.size() * sizeof(uint32_t), mesh.indices.data(),
                 GL_STATIC_DRAW);

    GLuint vao{};
    glGenVertexArrays(1, &vao);
    glBindVertexArray(vao);
    glEnableVertexAttribArray(0);
    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo);
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, sizeof(Vertex),
                          reinterpret_cast<void*>(offsetof(Vertex, position)));
    glEnableVertexAttribArray(1);
    glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, sizeof(Vertex),
                          reinterpret_cast<void*>(offsetof(Vertex, normal)));


    /*
     * Initialize Light informations
     */
    DirectionnalLightInfo light{};
    light.dir = glm::normalize(glm::vec3(0.0f, -1.0f, -1.0f));
    light.size = 5.0f;
    light.ubo.proj = glm::ortho(-light.size / 2.0f, light.size / 2.0f, -light.size / 2.0f,
                                light.size / 2.0f, -10.0f, 20.0f);
    light.ubo.view = glm::lookAt(-light.dir * 1.0f, glm::vec3(0.0f, 0.0f, 0.0f), glm::vec3(0.0f, 1.0f,
                                 0.0f));
    light.ubo.projView = light.ubo.proj * light.ubo.view;


    /*
     * Depth pipeline
     */
    GLuint shaderDepthProgramme{};
    {
        auto vertexDepthShader = shader::loadShader(std::string(RESOURCE_PATH) +
                                 "/shaders/vertexDepth.vert");
        GLuint vds = glCreateShader(GL_VERTEX_SHADER);
        const GLchar* vertexDepthDataPointer = vertexDepthShader.data();
        const GLint vertexDepthDataSize = static_cast<GLint>(vertexDepthShader.size());
        glShaderSource(vds, 1, &vertexDepthDataPointer, &vertexDepthDataSize);
        glCompileShader(vds);

        auto fragmentDepthShader = shader::loadShader(std::string(RESOURCE_PATH) +
                                   "/shaders/fragmentDepth.frag");
        GLuint fds = glCreateShader(GL_FRAGMENT_SHADER);
        const GLchar* fragmentDepthDataPointer = fragmentDepthShader.data();
        const GLint fragmentDepthDataSize = static_cast<GLint>(fragmentDepthShader.size());
        glShaderSource(fds, 1, &fragmentDepthDataPointer, &fragmentDepthDataSize);
        glCompileShader(fds);

        shaderDepthProgramme = glCreateProgram();
        glAttachShader(shaderDepthProgramme, fds);
        glAttachShader(shaderDepthProgramme, vds);
        glLinkProgram(shaderDepthProgramme);
    }




    const GLint projDepthID = glGetUniformLocation(shaderDepthProgramme, "camera.proj");
    const GLint viewDepthID = glGetUniformLocation(shaderDepthProgramme, "camera.view");


    glUniformMatrix4fv(projDepthID, 1, false, &light.ubo.proj[0][0]);
    glUniformMatrix4fv(viewDepthID, 1, false, &light.ubo.view[0][0]);

    //We generate a framebuffer, and will render the color rendertarget and depth buffer
    GLuint framebufferDepth{};
    glGenFramebuffers(1, &framebufferDepth);
    glBindFramebuffer(GL_FRAMEBUFFER, framebufferDepth);

    // Color
    GLuint texDepth{};
    glGenTextures(1, &texDepth);
    glBindTexture(GL_TEXTURE_2D, texDepth);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA32F, SHADOW_RES, SHADOW_RES, 0, GL_RGBA, GL_FLOAT,
                 nullptr);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);

    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texDepth, 0);

    // Depth Attachment
    GLuint texShadow{};
    glGenTextures(1, &texShadow);
    glBindTexture(GL_TEXTURE_2D, texShadow);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_DEPTH_COMPONENT16, SHADOW_RES, SHADOW_RES, 0, GL_DEPTH_COMPONENT,
                 GL_FLOAT,
                 nullptr);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);

    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_TEXTURE_2D, texShadow, 0);

    {
        const std::array<GLenum, 2> drawBuffers = {GL_COLOR_ATTACHMENT0, GL_NONE};
        glDrawBuffers(3, drawBuffers.data());
    }

    if(glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
        throw std::runtime_error("Failed to create framebuffer from depth attachment!");


    /*
     *  Main pipeline
     */
    GLuint shaderProgramme{};
    {
        auto vertexShader = shader::loadShader(std::string(RESOURCE_PATH) + "/shaders/vertex.vert");
        GLuint vs = glCreateShader(GL_VERTEX_SHADER);
        const GLchar* vertexDataPointer = vertexShader.data();
        const GLint vertexDataSize = static_cast<GLint>(vertexShader.size());
        glShaderSource(vs, 1, &vertexDataPointer, &vertexDataSize);
        glCompileShader(vs);

        auto fragmentShader = shader::loadShader(std::string(RESOURCE_PATH) + "/shaders/fragment.frag");
        GLuint fs = glCreateShader(GL_FRAGMENT_SHADER);
        const GLchar* fragmentDataPointer = fragmentShader.data();
        const GLint fragmentDataSize = static_cast<GLint>(fragmentShader.size());
        glShaderSource(fs, 1, &fragmentDataPointer, &fragmentDataSize);
        glCompileShader(fs);

        shaderProgramme = glCreateProgram();
        glAttachShader(shaderProgramme, fs);
        glAttachShader(shaderProgramme, vs);
        glLinkProgram(shaderProgramme);
    }


    UniformData ubo;
    actualizeProjection(ubo.proj, winData);
    actualizeView(ubo.view);

    const GLint projID = glGetUniformLocation(shaderProgramme, "camera.proj");
    const GLint viewID = glGetUniformLocation(shaderProgramme, "camera.view");
    const GLint lightBiasProjViewID = glGetUniformLocation(shaderProgramme, "light.biasProjView");
    const GLint lightProjViewID = glGetUniformLocation(shaderProgramme, "light.projView");
    const GLint lightViewID = glGetUniformLocation(shaderProgramme, "light.view");
    const GLint lightDirID = glGetUniformLocation(shaderProgramme, "light.dir");
    const GLint texDepthID  = glGetUniformLocation(shaderProgramme, "texDepth");
    const GLint texShadowID  = glGetUniformLocation(shaderProgramme, "texShadow");

    glm::mat4 biasMatrix(
        0.5, 0.0, 0.0, 0.0,
        0.0, 0.5, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.5, 0.5, 0.5, 1.0
    );
    light.biasProjView = biasMatrix * light.ubo.projView;


    /**********************************************************************************/

    glEnable(GL_DEPTH_TEST);
    glDepthFunc(GL_LESS);
    glEnable(GL_CULL_FACE);
    glCullFace(GL_BACK);
    glBindVertexArray(vao);

    while(!glfwWindowShouldClose(winData.window))
    {
        // Depth framebuffer computation
        glBindFramebuffer(GL_FRAMEBUFFER, framebufferDepth);
        glViewport(0, 0, SHADOW_RES, SHADOW_RES);
        glUseProgram(shaderDepthProgramme);

        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        glUniformMatrix4fv(projDepthID, 1, false, &light.ubo.proj[0][0]);
        glUniformMatrix4fv(viewDepthID, 1, false, &light.ubo.view[0][0]);
        glDrawElements(GL_TRIANGLES, static_cast<GLsizei>(mesh.indices.size()), GL_UNSIGNED_INT, nullptr);


        // Main framebuffer Computation
        glBindFramebuffer(GL_FRAMEBUFFER, 0);
        glViewport(0, 0, winData.width, winData.height);
        glUseProgram(shaderProgramme);

        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        if(winData.resized)
        {
            actualizeProjection(ubo.proj, winData);
            winData.resized = false;
        }

        actualizeView(ubo.view);
        glUniformMatrix4fv(projID, 1, false, &ubo.proj[0][0]);
        glUniformMatrix4fv(viewID, 1, false, &ubo.view[0][0]);
        glUniformMatrix4fv(lightBiasProjViewID, 1, false, &light.biasProjView[0][0]);
        glUniformMatrix4fv(lightProjViewID, 1, false, &light.ubo.projView[0][0]);
        glUniformMatrix4fv(lightViewID, 1, false, &light.ubo.view[0][0]);
        glUniform3fv(lightDirID, 1, &light.dir[0]);

        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, texDepth);
        glUniform1i(texDepthID, 0);

        glActiveTexture(GL_TEXTURE1);
        glBindTexture(GL_TEXTURE_2D, texShadow);
        glUniform1i(texShadowID, 1);
        glDrawElements(GL_TRIANGLES, static_cast<GLsizei>(mesh.indices.size()), GL_UNSIGNED_INT, nullptr);

        // update other events like input handling
        glfwPollEvents();
        // put the stuff we've been drawing onto the display
        glfwSwapBuffers(winData.window);
    }

    glDeleteFramebuffers(1, &framebufferDepth);

    glfwTerminate();
    return 0;
}

// Refresh projection infos
void actualizeProjection(glm::mat4& projMat, const WindowData& data)
{
    projMat = glm::perspective(45.0f, float(data.width) / float(data.height), 0.01f, 100.0f);
}

// New camera position and orientation
void actualizeView(glm::mat4& viewMat)
{
    static const auto startTime = std::chrono::high_resolution_clock::now();

    const auto delay = std::chrono::duration_cast<std::chrono::duration<float>>
                       (std::chrono::high_resolution_clock::now() -
                        startTime);
    const float angleRotation = delay.count() * PI / 4.0f;
    viewMat = glm::lookAt(glm::vec3(2.0f * std::cos(angleRotation), 0.0f,
                                    2.0f * std::sin(angleRotation)),
                          glm::vec3(0.0f),
                          glm::vec3(0.0f, 1.0f, 0.0f));
}

//Handle the window resizing
void resizeCallback(GLFWwindow*, int width, int height)
{
    winData.resized = true;
    winData.width = width;
    winData.height = height;
}



```

## Source file: src/data/data/Mesh.h

{
  "path": "src/data/data/Mesh.h",
  "blob_sha": "5999d788f55950d1ca7e5e5eb0e48451b186d5d8",
  "sha256": "e5f4cd774fb53ed0bd41f570ac091ee9b27438391a707f1a1111f5dd3cb6db67",
  "lines": 16,
  "bytes": 208,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Rodousse/SSSGems/blob/8e4fa443248b81348838eca30b493f362bcd3637/src/data/data/Mesh.h",
  "function_signature_candidates": []
}

```text

#pragma once
#include <glm/glm.hpp>
#include <vector>

struct Vertex
{
    glm::vec3 position;
    glm::vec3 normal;
};


struct Mesh
{
    std::vector<Vertex> vertices;
    std::vector<uint32_t> indices;
};


```

# Facial rendering source scan: MediaEyeEngineers/ShaderEye

Repository: https://github.com/MediaEyeEngineers/ShaderEye

Pinned source revision: 3b4bad5a5b1175ad056bc0732a55ddeafc7fad36

Discovery topic: eye shader

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["ShaderEye/.gitignore", "ShaderEye/AboutWindow.cpp", "ShaderEye/AboutWindow.h", "ShaderEye/AboutWindow.ui", "ShaderEye/CamerController.cpp", "ShaderEye/CameraController.h", "ShaderEye/CameraController.ui", "ShaderEye/CameraImage.cpp", "ShaderEye/CameraImage.h", "ShaderEye/CompileInfoWindow.cpp", "ShaderEye/CompileInfoWindow.h", "ShaderEye/CompileInfoWindow.ui", "ShaderEye/Component/ShaderEyeBGRACamera.cpp", "ShaderEye/Component/ShaderEyeComponent.hpp", "ShaderEye/Component/ShaderEyeRender.cpp", "ShaderEye/EyerCore/EyerCore.hpp", "ShaderEye/EyerCore/EyerLRUMap.hpp", "ShaderEye/EyerCore/EyerLinkedEle.hpp", "ShaderEye/EyerCore/EyerLinkedList.hpp", "ShaderEye/EyerCore/EyerLockQueue.hpp", "ShaderEye/EyerCore/EyerLog.cpp", "ShaderEye/EyerCore/EyerLog.hpp", "ShaderEye/EyerCore/EyerMap.hpp", "ShaderEye/EyerCore/EyerMapEle.hpp", "ShaderEye/EyerCore/EyerMat.cpp", "ShaderEye/EyerCore/EyerMat2x1.cpp", "ShaderEye/EyerCore/EyerMat3x1.cpp", "ShaderEye/EyerCore/EyerMat4x1.cpp", "ShaderEye/EyerCore/EyerMat4x4.cpp", "ShaderEye/EyerCore/EyerMath.hpp"], "shader": ["ShaderEye/.gitignore", "ShaderEye/AboutWindow.cpp", "ShaderEye/AboutWindow.h", "ShaderEye/AboutWindow.ui", "ShaderEye/CamerController.cpp", "ShaderEye/CameraController.h", "ShaderEye/CameraController.ui", "ShaderEye/CameraImage.cpp", "ShaderEye/CameraImage.h", "ShaderEye/CompileInfoWindow.cpp", "ShaderEye/CompileInfoWindow.h", "ShaderEye/CompileInfoWindow.ui", "ShaderEye/Component/ShaderEyeBGRACamera.cpp", "ShaderEye/Component/ShaderEyeComponent.hpp", "ShaderEye/Component/ShaderEyeRender.cpp", "ShaderEye/EyerCore/EyerCore.hpp", "ShaderEye/EyerCore/EyerLRUMap.hpp", "ShaderEye/EyerCore/EyerLinkedEle.hpp", "ShaderEye/EyerCore/EyerLinkedList.hpp", "ShaderEye/EyerCore/EyerLockQueue.hpp", "ShaderEye/EyerCore/EyerLog.cpp", "ShaderEye/EyerCore/EyerLog.hpp", "ShaderEye/EyerCore/EyerMap.hpp", "ShaderEye/EyerCore/EyerMapEle.hpp", "ShaderEye/EyerCore/EyerMat.cpp", "ShaderEye/EyerCore/EyerMat2x1.cpp", "ShaderEye/EyerCore/EyerMat3x1.cpp", "ShaderEye/EyerCore/EyerMat4x1.cpp", "ShaderEye/EyerCore/EyerMat4x4.cpp", "ShaderEye/EyerCore/EyerMath.hpp"]}

## Source file: ShaderEye/EyerGL/EyerGLShader.cpp

{
  "path": "ShaderEye/EyerGL/EyerGLShader.cpp",
  "blob_sha": "a20c5ab58db1ea2ce30eaecb7f58a1c8670f0e5c",
  "sha256": "4b0567dd45a490da3d897e0c6b239b93c74338c5d8a2dd9383065b0a57a98394",
  "lines": 96,
  "bytes": 2576,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 17,
    "shader": 38
  },
  "url": "https://github.com/MediaEyeEngineers/ShaderEye/blob/3b4bad5a5b1175ad056bc0732a55ddeafc7fad36/ShaderEye/EyerGL/EyerGLShader.cpp",
  "function_signature_candidates": []
}

```text

#include "EyerGL.hpp"

#include "GLHeader.h"

namespace Eyer{
    EyerGLShader::EyerGLShader(EyerGLShaderType _type, EyerString _src, EyerGLContext * _ctx)
    {
        ctx = _ctx;

        type = _type;
        src = _src;

        GLenum shaderType = GL_VERTEX_SHADER;
        if(type == EyerGLShaderType::VERTEX_SHADER){
            shaderType = GL_VERTEX_SHADER;
        }
        if(type == EyerGLShaderType::FRAGMENT_SHADER){
            shaderType = GL_FRAGMENT_SHADER;
        }

#ifdef QT_EYER_PLAYER
        shaderId = ctx->glCreateShader(shaderType);
#else
        shaderId = glCreateShader(shaderType);
#endif

    }

    EyerGLShader::~EyerGLShader()
    {
        if(shaderId != 0){
#ifdef QT_EYER_PLAYER
            ctx->glDeleteShader(shaderId);
#else
            glDeleteShader(shaderId);
#endif

            shaderId = 0;
        }
    }

    int EyerGLShader::Compile(EyerGLShaderError & shaderError)
    {
        if(shaderId == 0){
            return -1;
        }

        shaderError.shaderType = type;

#ifdef QT_EYER_PLAYER
        // EyerLog("Shader Src:\n %s\n", src.str);
        ctx->glShaderSource(shaderId, 1, &src.str , NULL);
        ctx->glCompileShader(shaderId);

        GLint Result = GL_FALSE;
        int InfoLogLength;
        ctx->glGetShaderiv(shaderId, GL_COMPILE_STATUS, &Result);
        ctx->glGetShaderiv(shaderId, GL_INFO_LOG_LENGTH, &InfoLogLength);

        shaderError.errorLen = InfoLogLength;

        if ( InfoLogLength > 0 ){
            std::vector<char> ShaderErrorMessage(InfoLogLength+1);
            ctx->glGetShaderInfoLog(shaderId, InfoLogLength, NULL, &ShaderErrorMessage[0]);
            EyerLog("%s\n", &ShaderErrorMessage[0]);

            EyerLog("Shader Src:\n %s\n", src.str);

            shaderError.error = &ShaderErrorMessage[0];
        }
#else
        // EyerLog("Shader Src:\n %s\n", src.str);
        glShaderSource(shaderId, 1, &src.str , NULL);
        glCompileShader(shaderId);

        GLint Result = GL_FALSE;
        int InfoLogLength;
        glGetShaderiv(shaderId, GL_COMPILE_STATUS, &Result);
        glGetShaderiv(shaderId, GL_INFO_LOG_LENGTH, &InfoLogLength);
        if ( InfoLogLength > 0 ){
            std::vector<char> ShaderErrorMessage(InfoLogLength+1);
            glGetShaderInfoLog(shaderId, InfoLogLength, NULL, &ShaderErrorMessage[0]);
            EyerLog("%s\n", &ShaderErrorMessage[0]);

            EyerLog("Shader Src:\n %s\n", src.str);
        }
#endif

        return 0;
    }

    unsigned int EyerGLShader::GL_GetShaderId()
    {
        return shaderId;
    }
}


```

## Source file: ShaderEye/Component/ShaderEyeRender.cpp

{
  "path": "ShaderEye/Component/ShaderEyeRender.cpp",
  "blob_sha": "5c41867ad245eac28b9496a04c2c95372821cb06",
  "sha256": "62504f3474874f8bcce732d37336a55eb67eceb090a4c107fb2dd0eae9ad6b1a",
  "lines": 80,
  "bytes": 1816,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 10,
    "shader": 9
  },
  "url": "https://github.com/MediaEyeEngineers/ShaderEye/blob/3b4bad5a5b1175ad056bc0732a55ddeafc7fad36/ShaderEye/Component/ShaderEyeRender.cpp",
  "function_signature_candidates": []
}

```text

#include "ShaderEyeComponent.hpp"
#include "EyerGLShader/Shader.hpp"

ShaderEyeRender::ShaderEyeRender(EyerGLContext * _ctx)
{
    ctx = _ctx;
    float vertex[] = {
            1.0, 1.0, 0.0,
            1.0, -1.0, 0.0,
            -1.0, -1.0, 0.0,
            -1.0, 1.0, 0.0
    };
    float coor[] = {
            1.0, 1.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 1.0, 0.0
    };
    unsigned int vertexIndex[] = {
            0, 1, 2,
            0, 2, 3
    };

    // bgra = new Eyer::EyerGLDraw(BGRA_TEXTURE_VERTEX_SHADER, BGRA_TEXTURE_FRAGMENT_SHADER, ctx);
    // bgra->Init();

    vao = new Eyer::EyerGLVAO(ctx);

    vao->AddVBO(vertex, sizeof(vertex), 0);
    vao->AddVBO(coor, sizeof(coor), 1);
    vao->SetEBO(vertexIndex, sizeof(vertexIndex));

    // bgra->SetVAO(vao);
}

ShaderEyeRender::~ShaderEyeRender()
{
    if(draw != nullptr){
        delete draw;
        draw = nullptr;
    }

    if(vao != nullptr){
        delete vao;
        vao = nullptr;
    }
}

int ShaderEyeRender::SetShader(const char * vertex, const char * fragment, Eyer::EyerGLShaderError & vertexShaderError, Eyer::EyerGLShaderError & fragmentShaderError, Eyer::EyerGLProgramError & programError)
{
    if(draw != nullptr){
        delete draw;
        draw = nullptr;
    }

    draw = new Eyer::EyerGLDraw(vertex, fragment, ctx);

    draw->Init(vertexShaderError, fragmentShaderError, programError);

    draw->SetVAO(vao);

    return 0;
}

int ShaderEyeRender::SetCameraTexture(Eyer::EyerGLTexture * _cameraTexture)
{
    cameraTexture = _cameraTexture;
    return 0;
}

int ShaderEyeRender::Draw()
{
    if(cameraTexture != nullptr){
        if(draw != nullptr){
            draw->PutTexture("cameraTex", cameraTexture, 0);
            draw->Draw();
        }
    }
    return 0;
}


```

## Source file: ShaderEye/Component/ShaderEyeComponent.hpp

{
  "path": "ShaderEye/Component/ShaderEyeComponent.hpp",
  "blob_sha": "8106bc20443b48b72f9b1c1c63e84b1637b99d63",
  "sha256": "172eaa27a3879d9a12f03f11ee926e72d0aa58cf107aa8d12856e2d64a387022",
  "lines": 47,
  "bytes": 1144,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 21,
    "shader": 9
  },
  "url": "https://github.com/MediaEyeEngineers/ShaderEye/blob/3b4bad5a5b1175ad056bc0732a55ddeafc7fad36/ShaderEye/Component/ShaderEyeComponent.hpp",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "int SetTexture(Eyer::EyerGLTexture * _frameTexture);"
    },
    {
      "line": 15,
      "text": "virtual int Draw();"
    },
    {
      "line": 33,
      "text": "int SetCameraTexture(Eyer::EyerGLTexture * _cameraTexture);"
    },
    {
      "line": 35,
      "text": "int SetShader(const char * vertex, const char * fragment, Eyer::EyerGLShaderError & vertexShaderError, Eyer::EyerGLShaderError & fragmentShaderError, Eyer::EyerGLProgramError & programError);"
    },
    {
      "line": 37,
      "text": "virtual int Draw();"
    }
  ]
}

```text

#ifndef	SHADER_EYE_CUSTOM_COMPONENT_H
#define	SHADER_EYE_CUSTOM_COMPONENT_H

#include "EyerGL/EyerGL.hpp"
#include "EyerCore/EyerCore.hpp"

class ShaderEyeBGRACamera : public Eyer::EyerGLComponent
{
public:
    ShaderEyeBGRACamera(EyerGLContext * _ctx);
    ~ShaderEyeBGRACamera();

    int SetTexture(Eyer::EyerGLTexture * _frameTexture);

    virtual int Draw();

private:
    EyerGLContext * ctx = nullptr;

    Eyer::EyerGLDraw * bgra = nullptr;
    Eyer::EyerGLVAO * vao = nullptr;

    Eyer::EyerGLTexture * frameTexture = nullptr;
};


class ShaderEyeRender : public Eyer::EyerGLComponent
{
public:
    ShaderEyeRender(EyerGLContext * _ctx);
    ~ShaderEyeRender();

    int SetCameraTexture(Eyer::EyerGLTexture * _cameraTexture);

    int SetShader(const char * vertex, const char * fragment, Eyer::EyerGLShaderError & vertexShaderError, Eyer::EyerGLShaderError & fragmentShaderError, Eyer::EyerGLProgramError & programError);

    virtual int Draw();
private:
    EyerGLContext * ctx = nullptr;
    Eyer::EyerGLTexture * cameraTexture = nullptr;

    Eyer::EyerGLDraw * draw = nullptr;
    Eyer::EyerGLVAO * vao = nullptr;
};


#endif


```

## Source file: ShaderEye/Component/ShaderEyeBGRACamera.cpp

{
  "path": "ShaderEye/Component/ShaderEyeBGRACamera.cpp",
  "blob_sha": "ca50030ec4924f0600d4618fa398046d4d6d9788",
  "sha256": "1c26a4f967a47477506acfb72e8ef3e1de104ffaa83389a5eee3fd9c370ea38f",
  "lines": 93,
  "bytes": 2229,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 11,
    "shader": 12
  },
  "url": "https://github.com/MediaEyeEngineers/ShaderEye/blob/3b4bad5a5b1175ad056bc0732a55ddeafc7fad36/ShaderEye/Component/ShaderEyeBGRACamera.cpp",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "void main(){"
    },
    {
      "line": 40,
      "text": "void main(){"
    }
  ]
}

```text

#include "ShaderEyeComponent.hpp"
#include "EyerGLShader/Shader.hpp"

ShaderEyeBGRACamera::ShaderEyeBGRACamera(EyerGLContext * _ctx)
{
    ctx = _ctx;
    float vertex[] = {
            1.0, 1.0, 0.0,
            1.0, -1.0, 0.0,
            -1.0, -1.0, 0.0,
            -1.0, 1.0, 0.0
    };
    float coor[] = {
            1.0, 1.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 1.0, 0.0
    };
    unsigned int vertexIndex[] = {
            0, 1, 2,
            0, 2, 3
    };

    char * BGRA_TEXTURE_VERTEX_SHADER = (char *)SHADER(
        layout (location = 0) in vec3 pos;
        layout (location = 1) in vec3 coor;

        out vec3 outCoor;

        void main(){
            outCoor = coor;
            gl_Position = vec4(pos, 1.0);
        }
    );

    char * BGRA_TEXTURE_FRAGMENT_SHADER = (char *)SHADER(
        out vec4 color;
        uniform sampler2D imageTex;
        in vec3 outCoor;
        void main(){
            vec2 TexCoords = vec2(1.0 - outCoor.x, 1.0 - outCoor.y);
            vec4 color_t = texture(imageTex, TexCoords);

            color.x = color_t.z;
            color.y = color_t.y;
            color.z = color_t.x;
            color.w = color_t.w;
        }
    );

    bgra = new Eyer::EyerGLDraw(BGRA_TEXTURE_VERTEX_SHADER, BGRA_TEXTURE_FRAGMENT_SHADER, ctx);

    Eyer::EyerGLShaderError vertexShaderError;
    Eyer::EyerGLShaderError fragmentShaderError;
    Eyer::EyerGLProgramError programError;
    bgra->Init(vertexShaderError, fragmentShaderError, programError);

    vao = new Eyer::EyerGLVAO(ctx);

    vao->AddVBO(vertex, sizeof(vertex), 0);
    vao->AddVBO(coor, sizeof(coor), 1);
    vao->SetEBO(vertexIndex, sizeof(vertexIndex));

    bgra->SetVAO(vao);
}

ShaderEyeBGRACamera::~ShaderEyeBGRACamera()
{
    if(bgra != nullptr){
        delete bgra;
        bgra = nullptr;
    }

    if(vao != nullptr){
        delete vao;
        vao = nullptr;
    }
}

int ShaderEyeBGRACamera::SetTexture(Eyer::EyerGLTexture * _frameTexture)
{
    frameTexture = _frameTexture;
    return 0;
}

int ShaderEyeBGRACamera::Draw()
{
    if(frameTexture != nullptr){
        bgra->PutTexture("imageTex", frameTexture, 0);
        bgra->Draw();
    }
    return 0;
}


```

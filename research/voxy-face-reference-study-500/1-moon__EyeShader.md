# Facial rendering source scan: 1-moon/EyeShader

Repository: https://github.com/1-moon/EyeShader

Pinned source revision: d3dc1e63bc13cfe07aeadde072c4e4671bf3b930

Discovery topic: eye shader

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["EyeShader/App.cpp", "EyeShader/App.hpp", "EyeShader/EyeShader.sln", "EyeShader/EyeShader.vcxproj", "EyeShader/EyeShader.vcxproj.filters", "EyeShader/Renderer.cpp", "EyeShader/Renderer.hpp", "EyeShader/Shader.cpp", "EyeShader/Shader.hpp", "EyeShader/default.frag", "EyeShader/default.vert", "EyeShader/main.cpp", "eye_components.png"], "shader": ["EyeShader/App.cpp", "EyeShader/App.hpp", "EyeShader/EyeShader.sln", "EyeShader/EyeShader.vcxproj", "EyeShader/EyeShader.vcxproj.filters", "EyeShader/Renderer.cpp", "EyeShader/Renderer.hpp", "EyeShader/Shader.cpp", "EyeShader/Shader.hpp", "EyeShader/default.frag", "EyeShader/default.vert", "EyeShader/main.cpp"]}

## Source file: EyeShader/Shader.hpp

{
  "path": "EyeShader/Shader.hpp",
  "blob_sha": "c37dbd279b7ec4e0b51dc70ca016656ba023c69f",
  "sha256": "9a42ec14ae22e544ebe1a14bb4d16698081f9fe8b0ea55e0031032644ddfb5ff",
  "lines": 25,
  "bytes": 448,
  "selection_score": 18,
  "keyword_line_counts": {
    "shader": 4
  },
  "url": "https://github.com/1-moon/EyeShader/blob/d3dc1e63bc13cfe07aeadde072c4e4671bf3b930/EyeShader/Shader.hpp",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "void use();"
    },
    {
      "line": 17,
      "text": "void setFloat(const char* name, float value);"
    },
    {
      "line": 18,
      "text": "void setVec2(const char* name, float x, float y);"
    },
    {
      "line": 19,
      "text": "void setVec4(const char* name, float x, float y, float z, float w);"
    }
  ]
}

```text

#pragma once 
#include<glad/glad.h>
#include<string>

/*
*	
*/

extern float vertices[12];

class Shader {
public: 
	Shader();
	Shader(const char* vertPath, const char* fragPath);

	void use();
	void setFloat(const char* name, float value); 
	void setVec2(const char* name, float x, float y);
	void setVec4(const char* name, float x, float y, float z, float w);
private:
	std::string readFile(const char* path); 

private:
	GLuint shaderProgram;
};

```

## Source file: EyeShader/App.hpp

{
  "path": "EyeShader/App.hpp",
  "blob_sha": "ef3b25ca67c8865a727e71a4360ed325d35903af",
  "sha256": "5abfba479b5026f2ad14d2add81350f4ecad3bace76f254f8620e30483f5c174",
  "lines": 24,
  "bytes": 348,
  "selection_score": 6,
  "keyword_line_counts": {
    "shader": 2
  },
  "url": "https://github.com/1-moon/EyeShader/blob/d3dc1e63bc13cfe07aeadde072c4e4671bf3b930/EyeShader/App.hpp",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "void render();\t// main loop"
    },
    {
      "line": 17,
      "text": "void initGLFW();"
    },
    {
      "line": 18,
      "text": "void initGLAD();"
    }
  ]
}

```text

#pragma once
#include <glad/glad.h>
#include <GLFW/glfw3.h>
#include "Shader.hpp"
#include "Renderer.hpp"

class App
{
public:
	App(int windowWidth, int windowHeight,
		const char* title); 
	~App();

	void render();	// main loop
private:
	
	void initGLFW();
	void initGLAD();
private:
	GLFWwindow* window;
	Shader* shader;
	Renderer* renderer;
};



```

## Source file: EyeShader/main.cpp

{
  "path": "EyeShader/main.cpp",
  "blob_sha": "2f6a373985ef087146633690f09998bc7db6ff03",
  "sha256": "0153ab17b5fc88ffaaa8bf0a3fa6b9bbf1d0346074dae8814f5b64aca78471dd",
  "lines": 18,
  "bytes": 305,
  "selection_score": 6,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/1-moon/EyeShader/blob/d3dc1e63bc13cfe07aeadde072c4e4671bf3b930/EyeShader/main.cpp",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "int main() {"
    }
  ]
}

```text

#include <iostream>

#include "App.hpp"

namespace 
{
	constexpr int initWindowWidth = 1280;
	constexpr int initWindowHeight = 720;
    constexpr const char* initwindowTitle = "Eyes"; 
}

int main() {
    App app(initWindowWidth, initWindowHeight, initwindowTitle);
    
    app.render();

    return 0;
}

```

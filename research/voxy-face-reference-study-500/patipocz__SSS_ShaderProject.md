# Facial rendering source scan: patipocz/SSS_ShaderProject

Repository: https://github.com/patipocz/SSS_ShaderProject

Pinned source revision: db83264f72657a354e984952083ee67e41ac5a01

Discovery topic: subsurface scattering

Description: Subsurface Scattering Shaders project

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["SSS_ShaderProject/.gitignore", "SSS_ShaderProject/CMakeLists.txt", "SSS_ShaderProject/README.md", "SSS_ShaderProject/imgui/include/ImGuiRenderer.h", "SSS_ShaderProject/imgui/include/ImGuizmo.h", "SSS_ShaderProject/imgui/include/QtImGui.h", "SSS_ShaderProject/imgui/include/imconfig.h", "SSS_ShaderProject/imgui/include/imgui.h", "SSS_ShaderProject/imgui/include/imgui_internal.h", "SSS_ShaderProject/imgui/include/stb_rect_pack.h", "SSS_ShaderProject/imgui/include/stb_textedit.h", "SSS_ShaderProject/imgui/include/stb_truetype.h", "SSS_ShaderProject/imgui/src/ColourPicker.cpp", "SSS_ShaderProject/imgui/src/ImGuiRenderer.cpp", "SSS_ShaderProject/imgui/src/ImGuizmo.cpp", "SSS_ShaderProject/imgui/src/QtImGui.cpp", "SSS_ShaderProject/imgui/src/imgui.cpp", "SSS_ShaderProject/imgui/src/imgui_draw.cpp", "SSS_ShaderProject/include/NGLScene.h", "SSS_ShaderProject/include/WindowParams.h", "SSS_ShaderProject/shaders/SSSFragment.glsl", "SSS_ShaderProject/shaders/SSSVertex.glsl", "SSS_ShaderProject/shaders/imguiFragment.glsl", "SSS_ShaderProject/shaders/imguiVertex.glsl", "SSS_ShaderProject/src/NGLScene.cpp", "SSS_ShaderProject/src/NGLSceneMouseControls.cpp", "SSS_ShaderProject/src/main.cpp"], "shader": ["ConstantShader/.gitignore", "ConstantShader/CMakeLists.txt", "ConstantShader/README.md", "ConstantShader/include/NGLScene.h", "ConstantShader/include/WindowParams.h", "ConstantShader/shaders/.vs/ProjectSettings.json", "ConstantShader/shaders/.vs/VSWorkspaceState.json", "ConstantShader/shaders/.vs/shaders/v16/.suo", "ConstantShader/shaders/.vs/slnx.sqlite", "ConstantShader/shaders/ConstantFragment.glsl", "ConstantShader/shaders/ConstantVertex.glsl", "ConstantShader/src/NGLScene.cpp", "ConstantShader/src/NGLSceneMouseControls.cpp", "ConstantShader/src/main.cpp", "SSS_ShaderProject/.gitignore", "SSS_ShaderProject/CMakeLists.txt", "SSS_ShaderProject/README.md", "SSS_ShaderProject/imgui/include/ImGuiRenderer.h", "SSS_ShaderProject/imgui/include/ImGuizmo.h", "SSS_ShaderProject/imgui/include/QtImGui.h", "SSS_ShaderProject/imgui/include/imconfig.h", "SSS_ShaderProject/imgui/include/imgui.h", "SSS_ShaderProject/imgui/include/imgui_internal.h", "SSS_ShaderProject/imgui/include/stb_rect_pack.h", "SSS_ShaderProject/imgui/include/stb_textedit.h", "SSS_ShaderProject/imgui/include/stb_truetype.h", "SSS_ShaderProject/imgui/src/ColourPicker.cpp", "SSS_ShaderProject/imgui/src/ImGuiRenderer.cpp", "SSS_ShaderProject/imgui/src/ImGuizmo.cpp", "SSS_ShaderProject/imgui/src/QtImGui.cpp"]}

## Source file: SSS_ShaderProject/shaders/SSSVertex.glsl

{
  "path": "SSS_ShaderProject/shaders/SSSVertex.glsl",
  "blob_sha": "4224b44051dcd3cc61d80d5be3f4543782d1bfa0",
  "sha256": "0e18f739addc47e99689b9ad1128d3afc237ffc56e8630f3c486d027055765d3",
  "lines": 36,
  "bytes": 843,
  "selection_score": 30,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/patipocz/SSS_ShaderProject/blob/db83264f72657a354e984952083ee67e41ac5a01/SSS_ShaderProject/shaders/SSSVertex.glsl",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "void main()"
    }
  ]
}

```text

#version 410 core

//basic vertex shader based on Jon's Simple NGL demo

/// @brief the vertex passed in
layout (location = 0) in vec3 inVert;
/// @brief the normal passed in
layout (location = 1) in vec3 inNormal;
/// @brief the in uv
layout (location = 2) in vec2 inUV;

out vec3 worldPos;
out vec3 normal;
out vec2 uv;

layout( std140) uniform TransformUBO
{
  mat4 MVP;
  mat4 normalMatrix;
  mat4 M;
} transforms;

void main()
{
	// transform the vertex from object space to world (but do not apply the camera or projection transform)
	worldPos = vec3(transforms.M * vec4(inVert, 1.0f));

	// transform the normal too
	normal = normalize(mat3(transforms.normalMatrix) * inNormal);

	// the final vertex position drawn to screen
	gl_Position = transforms.MVP*vec4(inVert,1.0);

	// TODO use these to read a thickness texture
	uv = inUV;
}


```

## Source file: SSS_ShaderProject/shaders/SSSFragment.glsl

{
  "path": "SSS_ShaderProject/shaders/SSSFragment.glsl",
  "blob_sha": "cdf74f7488b6ef494ee9bcaee47e8b8e4de9afde",
  "sha256": "b31ed3b1edfacab568bc26710c8b12371939208e4d5361878272ae6764407d71",
  "lines": 50,
  "bytes": 1555,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 2,
    "subsurface": 2,
    "sss": 1,
    "face": 3
  },
  "url": "https://github.com/patipocz/SSS_ShaderProject/blob/db83264f72657a354e984952083ee67e41ac5a01/SSS_ShaderProject/shaders/SSSFragment.glsl",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "float saturate(float x) {"
    },
    {
      "line": 33,
      "text": "void main()"
    }
  ]
}

```text

#version 330 core
layout (location =0) out vec4 fragColour;

in vec3 worldPos;
in vec3 normal;
in vec2 uv;

// light uniforms
uniform vec3 lightPosition;
uniform vec3 lightColor;

uniform vec3 camPos;

// SSS parameter uniforms are set from imgui -

// ambient value, visible from all angles, represents front & back translucency
uniform vec3 lightAmbient; 
// power value for direct translucency, view dependent
uniform float power;
// subsurface distortion, changes the *surface normal*, view dependent, fresnel-like
uniform float distortion;
// this could be a premade texture map, here just a uniform
uniform float thickness;
// direct/back translucency, view dependent
uniform float scale;
// parameter controlling the falloff of the effecton the whole surface
uniform float attenuation;

float saturate(float x) {
  return max(0, min(1, x));
}

void main()
{
    // frostbite subsurface - from https://colinbarrebrisebois.com/2011/03/07/gdc-2011-approximating-translucency-for-a-fast-cheap-and-convincing-subsurface-scattering-look/
    vec3 lightPos = lightPosition;

    // uses the same names as the presentation for easier translation
    vec3 vNormal = normalize(normal);
    vec3 vEye = normalize(camPos - worldPos);
    vec3 vLight = normalize(worldPos - lightPos);
    vec3 ambient = lightAmbient;

    vec3 vLTLight = vLight + (vNormal * distortion);
    float fLTDot = pow(saturate(dot(vEye, -vLTLight)), power) * scale;
    vec3 fLT = attenuation * (fLTDot * ambient) * thickness;

    vec3 color = fLT;
    fragColour = vec4(fLT, 1.0);
}

```

## Source file: SSS_ShaderProject/shaders/imguiVertex.glsl

{
  "path": "SSS_ShaderProject/shaders/imguiVertex.glsl",
  "blob_sha": "fbc9bf39136a7fe51f51b7aaaf35705398fa6ec0",
  "sha256": "28d2428ce764d63264987eab7241bdefe6d4c6a79f62b6176ff27e05ca8aa715",
  "lines": 14,
  "bytes": 283,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/patipocz/SSS_ShaderProject/blob/db83264f72657a354e984952083ee67e41ac5a01/SSS_ShaderProject/shaders/imguiVertex.glsl",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "void main()"
    }
  ]
}

```text

#version 330

uniform mat4 ProjMtx;
layout (location=0) in vec2 Position;
layout (location=1) in vec2 UV;
layout (location=2) in vec4 Color;
out vec2 Frag_UV;
out vec4 Frag_Color;
void main()
{
  Frag_UV = UV;
  Frag_Color = Color;
  gl_Position = ProjMtx * vec4(Position.xy,0,1);
}


```

## Source file: SSS_ShaderProject/shaders/imguiFragment.glsl

{
  "path": "SSS_ShaderProject/shaders/imguiFragment.glsl",
  "blob_sha": "903e7bb04a706ffe5afa8438b9056bf54344ec64",
  "sha256": "ef5c750879d1f3a75b3c0936a4f731bdc8db5b889e00fda06fbb1685ad29c53f",
  "lines": 9,
  "bytes": 191,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/patipocz/SSS_ShaderProject/blob/db83264f72657a354e984952083ee67e41ac5a01/SSS_ShaderProject/shaders/imguiFragment.glsl",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "void main()"
    }
  ]
}

```text

#version 330
uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;
layout (location=0) out vec4 Out_Color;
void main()
{
  Out_Color = Frag_Color * texture( Texture, Frag_UV.st);
}


```

# Facial rendering source scan: robert-leitl/toy-bricks

Repository: https://github.com/robert-leitl/toy-bricks

Pinned source revision: b0739cf017dd03a49dd7be2ed9f7efe4d4aa49c4

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"sss": ["src/app/shader/sss-blur.frag.glsl", "src/app/shader/sss-dilation.frag.glsl", "src/assets/sss-test.blend"], "shader": ["src/app/shader/composite.frag.glsl", "src/app/shader/mrt-brick.frag.glsl", "src/app/shader/mrt-brick.vert.glsl", "src/app/shader/mrt-floor.frag.glsl", "src/app/shader/mrt-floor.vert.glsl", "src/app/shader/normal-noise.glsl", "src/app/shader/quad.vert.glsl", "src/app/shader/sss-blur.frag.glsl", "src/app/shader/sss-dilation.frag.glsl", "src/app/shader/utils.glsl", "src/app/ssao/custom-ssao-shader.js"]}

## Source file: src/app/shader/sss-blur.frag.glsl

{
  "path": "src/app/shader/sss-blur.frag.glsl",
  "blob_sha": "2807aa67a99a91d276a0423a484293c6d45ce54b",
  "sha256": "cf246fd245b52031f7f4926069c11557868c970bb47e2ff1b7c55d7fb6e1d2e5",
  "lines": 106,
  "bytes": 2748,
  "selection_score": 30,
  "keyword_line_counts": {},
  "url": "https://github.com/robert-leitl/toy-bricks/blob/b0739cf017dd03a49dd7be2ed9f7efe4d4aa49c4/src/app/shader/sss-blur.frag.glsl",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "float gaussianPdf(in float x, in float sigma) {"
    },
    {
      "line": 20,
      "text": "vec3 getKernelSample("
    },
    {
      "line": 49,
      "text": "void main() {"
    }
  ]
}

```text

uniform sampler2D tDiffuse_Id;
uniform sampler2D tDepth;
uniform sampler2D tNormal_Specular;
uniform vec2 uDirection;
uniform vec2 resolution;
uniform float uFBOScale;

out vec4 outColor;

in vec2 vUv;

uint bgId = uint(0);

float gaussianPdf(in float x, in float sigma) {
    float k = .15915494; // 1 / (2 * PI)
    float pdf = (k / sigma) * exp(-(x * x) / (2. * sigma));
    return pow(pdf, 1.);
}

vec3 getKernelSample(
    in vec3 cDiffuse, 
    in float cDepth, 
    in vec3 cNormal, 
    in float cId,
    in float weight,
    in vec2 uv,
    in float depthDeltaFactor,
    in float lumDeltaFactor,
    in float normDeltaFactor
) {
    vec4 diffuseId = texture(tDiffuse_Id, uv);
    vec3 result = diffuseId.rgb;
    vec4 normSpecular = texture(tNormal_Specular, uv);
    vec3 normal = normSpecular.xyz;
    float id = diffuseId.w * 255.;
    float depth = texture(tDepth, uv).r;

    if (uint(id) != uint(cId)) return cDiffuse;

    float n = distance(cNormal, normal) * normDeltaFactor; 
    float s = min(depthDeltaFactor * abs(cDepth - depth) * (1. + n), 1.0);
    float t = distance(result, cDiffuse) * weight * .7;
    result *= (lumDeltaFactor + t);
    result = mix(result, cDiffuse, s);

    return result;
}

void main() {
    int kernelSize = 4;
    vec2 texelSize = 1. / resolution;
    float fSigma = float(kernelSize);
    float weightSum = gaussianPdf(0.0, fSigma);

    vec4 diffuseId = texture(tDiffuse_Id, vUv);
    vec3 cDiffuse = diffuseId.rgb;
    float cDepth = texture( tDepth, vUv).r;
    vec4 normSpecular = texture(tNormal_Specular, vUv);
    vec3 cNormal = normSpecular.xyz;
    float cId = diffuseId.w * 255.;
    float scale = 8.5 * uFBOScale;


    if (uint(cId) == bgId) {
        outColor = vec4(0.);
        return;
    }

    float depthDeltaFactor = 25.;
    float lumDeltaFactor = 1.25;
    float normDeltaFactor = 1.;

    vec3 diffuseSum = cDiffuse * weightSum;
    for( int i = 1; i < kernelSize; i ++ ) {
        float x = float(i);
        float w = gaussianPdf(x, fSigma);
        vec2 uvOffset = uDirection * texelSize * x * scale;

        vec3 sample1 = getKernelSample(
            cDiffuse, 
            cDepth, 
            cNormal, 
            cId,
            w,
            vUv + uvOffset,
            depthDeltaFactor,
            lumDeltaFactor,
            normDeltaFactor
        );
        vec3 sample2 = getKernelSample(
            cDiffuse, 
            cDepth, 
            cNormal, 
            cId,
            w,
            vUv - uvOffset,
            depthDeltaFactor,
            lumDeltaFactor,
            normDeltaFactor
        );

        diffuseSum += (sample1 + sample2) * w;
        weightSum += 2. * w;
    }
    outColor = vec4(diffuseSum/weightSum, diffuseId.w);
}

```

## Source file: src/app/shader/sss-dilation.frag.glsl

{
  "path": "src/app/shader/sss-dilation.frag.glsl",
  "blob_sha": "e9d859999fe554bd9295f38f04fb3c387bd58ca8",
  "sha256": "9adc076300a3cd930761864bb45470c1fb4027d70c6a3c35fbcb5bfafb6379c9",
  "lines": 39,
  "bytes": 963,
  "selection_score": 30,
  "keyword_line_counts": {},
  "url": "https://github.com/robert-leitl/toy-bricks/blob/b0739cf017dd03a49dd7be2ed9f7efe4d4aa49c4/src/app/shader/sss-dilation.frag.glsl",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "void main() {"
    }
  ]
}

```text

uniform sampler2D tDiffuse_Id;
uniform vec2 uDirection;
uniform vec2 resolution;

out vec4 outDiffuse_Id;

in vec2 vUv;

void main() {
    vec2 texelSize = 1. / resolution;
    vec4 cDiffuseId = texture(tDiffuse_Id, vUv);
    float scale = 3.;
    int kernelSize = 2;
    float weight = 1. / float(kernelSize + 1);


    float cLum = dot(cDiffuseId.rgb, cDiffuseId.rgb);
    vec3 result = cDiffuseId.rgb;
    for( int i = 1; i < kernelSize; i ++ ) {
        float x = float(i);
        vec2 uvOffset = uDirection * texelSize * x * scale;
        vec4 diffuseId = texture(tDiffuse_Id, vUv + uvOffset);
        float lum = dot(diffuseId.rgb, diffuseId.rgb);

        if (lum > cLum) {
            result = diffuseId.rgb;
            cLum = lum;
        }
        
        diffuseId = texture(tDiffuse_Id, vUv - uvOffset);

        if (lum > cLum) {
            result = diffuseId.rgb;
            cLum = lum;
        }
    }

    outDiffuse_Id = vec4(result, 1.);
}

```

## Source file: src/app/shader/utils.glsl

{
  "path": "src/app/shader/utils.glsl",
  "blob_sha": "7231820daab5d757f407fc495b739fb18595da27",
  "sha256": "48c1b2bfebf458f6e7c7e1e9e51e029b2c8c6a2a89558ff01c2a2849692b4b56",
  "lines": 27,
  "bytes": 1029,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/robert-leitl/toy-bricks/blob/b0739cf017dd03a49dd7be2ed9f7efe4d4aa49c4/src/app/shader/utils.glsl",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "float map(float value, float inMin, float inMax, float outMin, float outMax) {"
    },
    {
      "line": 5,
      "text": "float getLinearZPerspective(in float depth, in mat4 projectionMatrix) {"
    },
    {
      "line": 12,
      "text": "float getNormalizedZPerspective(in float depth, in mat4 projectionMatrix) {"
    },
    {
      "line": 19,
      "text": "float getLinearZOrtho(in float depth, in mat4 projectionMatrix) {"
    },
    {
      "line": 25,
      "text": "float getNormalizedZOrtho(in float depth) {"
    }
  ]
}

```text

float map(float value, float inMin, float inMax, float outMin, float outMax) {
  return outMin + (outMax - outMin) * (value - inMin) / (inMax - inMin);
}

float getLinearZPerspective(in float depth, in mat4 projectionMatrix) {
    float z_ndc = depth * 2.0 - 1.0;
    float A = projectionMatrix[2][2];
    float B = projectionMatrix[3][2];
    return B / (A + z_ndc);
}

float getNormalizedZPerspective(in float depth, in mat4 projectionMatrix) {
    float linearZ = getLinearZPerspective(depth, projectionMatrix);
    float near = projectionMatrix[3][2]/(projectionMatrix[2][2] - 1.);
    float far = projectionMatrix[3][2]/(projectionMatrix[2][2] + 1.);
    return map(linearZ, near, far, 0., 1.);
}

float getLinearZOrtho(in float depth, in mat4 projectionMatrix) {
    float near = (1. + projectionMatrix[3][2])/projectionMatrix[2][2];
    float far = -(1. - projectionMatrix[3][2])/projectionMatrix[2][2];
    return depth * (far - near) + near;
}

float getNormalizedZOrtho(in float depth) {
    return depth * 0.5 + 0.5;
}

```

## Source file: src/app/shader/quad.vert.glsl

{
  "path": "src/app/shader/quad.vert.glsl",
  "blob_sha": "b80763aad0bcde055c1edd635bee3af9927c714b",
  "sha256": "4b5ad9b5d853ae50e68a2cb64bfd531ba920a3cf42a92396cb2e1b2c476c6d19",
  "lines": 6,
  "bytes": 89,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/robert-leitl/toy-bricks/blob/b0739cf017dd03a49dd7be2ed9f7efe4d4aa49c4/src/app/shader/quad.vert.glsl",
  "function_signature_candidates": [
    {
      "line": 3,
      "text": "void main() {"
    }
  ]
}

```text

out vec2 vUv;

void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0., 1.);
}

```

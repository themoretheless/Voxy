# Facial rendering source scan: MaxBittker/chromatophore

Repository: https://github.com/MaxBittker/chromatophore

Pinned source revision: 2dc478ac0d6192887a77413f50454f02c59dfe6c

Discovery topic: skin shader

Description: cuttlefish skin shader

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"shader": ["src/pack.shader.js"]}

## Source file: src/pack.shader.js

{
  "path": "src/pack.shader.js",
  "blob_sha": "d71276cbf4e6c7befd5eab84c4db87f0f32ba7b0",
  "sha256": "3bce35ba8e5cc6e2ca8dbc45da8fdf6d26741d74d34449ad06a4ca8f375c5a41",
  "lines": 7,
  "bytes": 236,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/MaxBittker/chromatophore/blob/2dc478ac0d6192887a77413f50454f02c59dfe6c/src/pack.shader.js",
  "function_signature_candidates": []
}

```text

const glslify = require("glslify");
const path = require("path");

module.exports = require("shader-reload")({
  vertex: glslify(path.resolve(__dirname, "vertex.glsl")),
  fragment: glslify(path.resolve(__dirname, "fragment.glsl"))
});


```

## Source file: src/vertex.glsl

{
  "path": "src/vertex.glsl",
  "blob_sha": "615750b1468181480ea16b8410348568966ba913",
  "sha256": "c9487ca9e81e437d6a28f977735d70de73c5c9e531aad42c0def0e14b2ad36ac",
  "lines": 10,
  "bytes": 179,
  "selection_score": 12,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/MaxBittker/chromatophore/blob/2dc478ac0d6192887a77413f50454f02c59dfe6c/src/vertex.glsl",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "void main () {"
    }
  ]
}

```text


// boring "pass-through" vertex shader
precision mediump float;
attribute vec2 position;
varying vec2 uv;

void main () {
  uv = position;
  gl_Position = vec4(position, 0, 1);
}

```

## Source file: src/fragment.glsl

{
  "path": "src/fragment.glsl",
  "blob_sha": "341b5f843c2ef5233c703ece9898088d51b1bf89",
  "sha256": "f35197dd8e004336c99bef7635a7014b0af6cd49b1b34edba85fe69228a958f9",
  "lines": 202,
  "bytes": 5700,
  "selection_score": 12,
  "keyword_line_counts": {
    "eye": 9,
    "lip": 12
  },
  "url": "https://github.com/MaxBittker/chromatophore/blob/2dc478ac0d6192887a77413f50454f02c59dfe6c/src/fragment.glsl",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "float random(in vec2 _st) {"
    },
    {
      "line": 26,
      "text": "vec2 hash(vec2 p) {"
    },
    {
      "line": 32,
      "text": "vec3 voronoi(in vec2 x) {"
    },
    {
      "line": 56,
      "text": "void main() {"
    }
  ]
}

```text

precision highp float;
uniform float t;
uniform vec2 resolution;
// uniform sampler2D backBuffer;
// uniform sampler2D webcam;
// uniform vec2 videoResolution;
// uniform vec2 eyes[2];

varying vec2 uv;

// clang-format off
#pragma glslify: squareFrame = require("glsl-square-frame")
#pragma glslify: worley2D = require(glsl-worley/worley2D.glsl)
#pragma glslify: hsv2rgb = require('glsl-hsv2rgb')
#pragma glslify: luma = require(glsl-luma)
#pragma glslify: smin = require(glsl-smooth-min)
#pragma glslify: fbm3d = require('glsl-fractal-brownian-noise/3d')
#pragma glslify: noise = require('glsl-noise/simplex/3d')

// clang-format on
#define PI 3.14159265359

float random(in vec2 _st) {
  return fract(sin(dot(_st.xy, vec2(12.9898, 78.233))) * 43758.5453123);
}
vec2 hash(vec2 p) {
  p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
  return fract(sin(p) * 18.5453);
}

vec2 pixel = vec2(1.0) / resolution;
vec3 voronoi(in vec2 x) {
  vec2 n = floor(x);
  vec2 f = fract(x);

  vec3 m = vec3(8.0);
  float f2 = 80.0;
  for (int j = -1; j <= 1; j++)
    for (int i = -1; i <= 1; i++) {
      vec2 g = vec2(float(i), float(j));
      vec2 o = hash(n + g);
      // vec2  r = g - f + o;
      vec2 r = g - f + (0.5 + 0.5 * sin(6.2831 * o));
      float d = dot(r, r);
      if (d < m.x) {
        f2 = m.x;
        m = vec3(d, o);
      } else if (d < f2) {
        f2 = d;
      }
    }

  return vec3(sqrt(m.x), sqrt(f2), m.y + m.z);
}

void main() {
  vec2 pos = squareFrame(resolution);
  vec3 color;
  // vec2 webcamCoord = (uv * 0.5 + vec2(0.5)) * resolution / videoResolution;
  // vec2 flipwcord = vec2(1.) - webcamCoord;
  // vec2 eye1 = eyes[0] / videoResolution;
  // vec2 eye2 = eyes[1] / videoResolution;
  vec2 textCoord = uv * 0.5 + vec2(0.5);
  // vec2 closeEye =
  // distance(eye1, flipwcord) < distance(eye2, flipwcord) ? eye1 : eye2;
  // vec2 suck = -2.0 * pixel * normalize(flipwcord - closeEye) *
  // random(uv + t * vec2(1.0));
  // *(0.5 + noise(vec3(uv * 10., t * 5.)));

  // vec3 webcamColor = texture2D(webcam, flipwcord).rgb * 0.95;
  // float ed = smin(distance(eye1, flipwcord), distance(eye2, flipwcord),
  // 0.01);

  // float ifd = distance(eye1, eye2);

  // float s = 0.1 * ifd * 4.;
  // vec2 mid = (eye1 + eye2) * 0.5;

  color = vec3(0.09, 0.089, 0.087);

  vec3 red = vec3(0.9, 0.25, 0.);
  vec3 yellow = vec3(0.85, 0.7, 0.);
  vec3 brown = vec3(0.4, 0.2, 0.1);
  // vec3 hue = red;
  float m = 100.;
  // pos += noise(vec3(pos * 15., 0.5)) * 0.01;
  float scale =
      30. + (noise(vec3(pos * 0.5 + vec2(t * -0.001, 0.), t * 0.001)) * 1.5);
  scale /= 3.0;
  vec3 c = voronoi(scale * pos);

  // colorize
  vec3 col = red;
  float h = 0.;
  if (c.z < 1.2) {
    h = 1.;
  }
  if (c.z < 0.7) {
    h = 2.;
  }
  // col += c.x * 0.5;
  // vec3 col = hsv2rgb(vec3(hue, 0.4, 0.6));
  // 0.5 + 0.5 * cos(c.y * 6.2831 + vec3(0.0, 1.0, 2.0));
  // col *= clamp(1.0 - 0.4 * c.x * c.x, 0.0, 1.0);
  // col -= (1.0 - smoothstep(0.08, 0.09, c.x));
  // col = hsv2rgb(vec3(0.0, 1.0, 0.9));
  // if (c.z < 1.2) {
  //   col = hsv2rgb(vec3(0.1, 0.9, 0.9));
  // }
  // if (c.z < 0.7) {
  //   col = hsv2rgb(vec3(0.05, 0.9, 0.35));
  // }
  vec2 spos = pos + noise(vec3(pos * 2., 0.5 + t * 0.0005)) * 0.0;

  float dilation =
      (scale / 30.) *
          (noise(vec3((spos * vec2(1.0, 2.)) + vec2(t * -0.004, h * 5.) +
                          (vec2(0.3) * h),
                      t * 0.000)) -
           0.3) *
          1.3

      + 0.6 + ((sin((t * 0.001) + c.z * 0.4) + 0.2) * 0.2);

  // dilation = 1.0 - abs(length(webcamColor - col));
  // dilation += 1.0;
  float dd = (c.y - c.x) * (1.0 - c.x * 1.0);
  dd *= dilation;
  dd *= 1.5;
  col = hsv2rgb(vec3(0.9, 0.6 - (dilation * 0.15) - c.x * 0.2, 0.7));
  if (c.z < 1.2) {
    col = hsv2rgb(vec3(0.5, 0.6 - dilation * 0.1, 0.6));
  }
  if (c.z < 0.7) {
    col = hsv2rgb(vec3(0.7, 0.6 - dilation * 0.1, 0.95));
  }

  if (dd > 0.4 || c.x < 0.05) {
    color = col;
  } else {
    // color += col * 0.03;
    // color = col * 0.00;

    color = vec3(244.0 / 255.0, 238.0 / 255.0, 239.0 / 255.0);
  }
  // color = (c.y - c.x) * col;
  // vec3 cel = cellular(pos * 5.);
  // color = hsv2rgb(

  //     vec3(cel.z / 9., 0.5, 0.5));
  // if (cel.x < 0.05) {
  //   color = vec3(0.);
  // }
  // vec3 col = hsv2rgb(vec3(t + floor(c.y * 1.5) / 5., 0.5, 0.5));

  // if (pos.x < -1. + pixel.x * 5.) {
  //   if (mod(t, 20.) > 18.) {
  //     // color = vec3(1., 1., 1.);
  //   } else {
  //     // color = vec3(0.2, 0.3, 0.4);
  //   }
  // } else {
  //   float weight = luma(webcamColor);

  //   float edge = luma(texture2D(webcam, flipwcord - pixel.x).rgb) -
  //                luma(texture2D(webcam, flipwcord + pixel.x).rgb);

  //   edge *= 3.;

  //   if (distance(mid.x, flipwcord.x) < (ifd * 1.0) &&
  //       mid.y - flipwcord.y < ifd * 1.8 && mid.y - flipwcord.y > ifd *
  //       -2.5)
  //       {
  //   } else {
  //     weight *= 0.8;
  //   }
  //   vec2 fall = 2. * pixel * vec2(1., 0) * (0.5 + random(vec2(uv.y, t)) *
  //   0.5) *
  //               (1. - weight);

  // color = texture2D(backBuffer, textCoord - fall).rgb * 1.0;
  //   if (uv.y < 0.0) {
  //     // color =weight * vec3(1.0);
  //   }
  // }

  // if (ed < s) {
  // distance(mid,flipwcord)<0.2
  // if (distance(mid.x, flipwcord.x) < (ifd * 1.0) &&
  //     mid.y - flipwcord.y < ifd * 1.8 && mid.y - flipwcord.y > ifd * -2.5)
  //     {
  //   float weight = luma(webcamColor);
  //   // color = weight * vec3(1.0);
  // }
  // if (luma(webcamColor) <
  //  0.4) {
  // 0.5 + 0.2 * noise(vec3(uv * 200., t * 0.1))) {
  // color = vec3(1.);
  // color *= 0.25;
  // }
  gl_FragColor = vec4(color, 1.0);
}

```

## Source file: index.js

{
  "path": "index.js",
  "blob_sha": "a49f6b4ef136c827f282215d5c619a5269df8f8b",
  "sha256": "a0e1a914fc8780ac417c72d4b8a29a6e0e5036150ea1586ed2c6b6a6bae9aa20",
  "lines": 115,
  "bytes": 2853,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "shader": 10
  },
  "url": "https://github.com/MaxBittker/chromatophore/blob/2dc478ac0d6192887a77413f50454f02c59dfe6c/index.js",
  "function_signature_candidates": []
}

```text

const { setupOverlay } = require("regl-shader-error-overlay");
setupOverlay();
// let CCapture = require("ccapture.js");
// var capturer = new CCapture({ format: "webm", framerate: 60 });

function downloadURI(uri, filename) {
  var link = document.createElement("a");
  link.download = filename;
  link.href = uri;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

const regl = require("regl")({
  pixelRatio: Math.min(window.devicePixelRatio, 1)
});
// const { setupWebcam } = require("./src/regl-webcam");
let shaders = require("./src/pack.shader.js");
let vert = shaders.vertex;
let frag = shaders.fragment;

shaders.on("change", () => {
  console.log("update");
  vert = shaders.vertex;
  frag = shaders.fragment;
  let overlay = document.getElementById("regl-overlay-error");
  overlay && overlay.parentNode.removeChild(overlay);
});

// const lastFrame = regl.texture();
// const pixels = regl.texture();
// let ct;
// let last27 = [0, 0];
// let last32 = [0, 0];
// let cam = setupWebcam({
// regl,
// done: (webcam, { videoWidth, videoHeight, ctracker }) => {
// ct = ctracker;
// window.ct = ct;
// console.log(ct);
// console.log(ctracker.getCurrentPosition())
// console.log(ctracker.getCurrentPosition())
let drawTriangle = regl({
  uniforms: {
    // webcam,
    // videoResolution: [videoWidth, videoHeight],
    // Becomes `uniform float t`  and `uniform vec2 resolution` in the shader.
    t: ({ tick }) => tick,
    resolution: ({ viewportWidth, viewportHeight }) => [
      viewportWidth,
      viewportHeight
    ]
    // backBuffer: lastFrame
    // "eyes[0]": () => {
    //   // let positions = ct.getCurrentPosition();
    //   if (positions) {
    //     last27 = positions[27];
    //     return positions[27];
    //   } else {
    //     return last27;
    //   }
    // },
    // "eyes[1]": () => {
    //   // let positions = ct.getCurrentPosition();
    //   if (positions) {
    //     last32 = positions[32];
    //     return positions[32];
    //   } else {
    //     return last32;
    //   }
    // }
  },

  frag: () => shaders.fragment,
  vert: () => shaders.vertex,
  attributes: {
    // Full screen triangle
    position: [
      [-1, 4],
      [-1, -1],
      [4, -1]
    ]
  },
  // Our triangle has 3 vertices
  count: 3
});

// capturer.start();
let f = 0;

regl.frame(function(context) {
  // regl.clear({
  // color: [0, 0, 0, 0]
  // });
  drawTriangle();
  // lastFrame({
  //   copy: true
  // });
  // debugger;
  // f++;
  // if (f < 60 * 5 && f % 2) {
  // f++;
  // downloadURI(
  // regl._gl.canvas.toDataURL(),
  // `chlor-frame-${(f / 2).toString().padStart(8, "0")}.png`
  // );/
  // } else {
  // capturer.stop();
  // default save, will download automatically a file called {name}.extension (webm/gif/tar)
  // capturer.save();
  // }
});
// }
// });


```

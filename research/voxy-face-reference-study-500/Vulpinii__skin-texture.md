# Facial rendering source scan: Vulpinii/skin-texture

Repository: https://github.com/Vulpinii/skin-texture

Pinned source revision: 2b7c668d8f3958d29b7908fc62b7d4347d239ce5

Discovery topic: subsurface scattering

Description: Procedural skin texture with subsurface scattering and god rays post-process effect.

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "external/glfw/LICENSE.md", "external/glm/copying.txt"]

Topic paths: {"skin": ["images/skin.gif", "images/youtube_skin.PNG"], "shader": ["assets/shaders/depth_fragment_shader.glsl", "assets/shaders/depth_vertex_shader.glsl", "assets/shaders/fragment_shader.glsl", "assets/shaders/godrays.cs.glsl", "assets/shaders/light_fragment_shader.glsl", "assets/shaders/light_vertex_shader.glsl", "assets/shaders/quad.fs.glsl", "assets/shaders/quad.vs.glsl", "assets/shaders/vertex_shader.glsl", "include/Shader.hpp"], "rig": ["external/glm/doc/api/arrowright.png", "external/glm/glm/detail/func_trigonometric.inl", "external/glm/glm/detail/func_trigonometric_simd.inl", "external/glm/glm/ext/quaternion_trigonometric.hpp", "external/glm/glm/ext/quaternion_trigonometric.inl", "external/glm/glm/gtx/fast_trigonometry.hpp", "external/glm/glm/gtx/fast_trigonometry.inl", "external/glm/glm/simd/trigonometric.h", "external/glm/glm/trigonometric.hpp", "external/glm/test/core/core_func_trigonometric.cpp", "external/glm/test/ext/ext_quaternion_trigonometric.cpp", "external/glm/test/gtx/gtx_fast_trigonometry.cpp"], "lip": ["external/glfw/tests/clipboard.c", "external/glm/glm/ext/matrix_clip_space.hpp", "external/glm/glm/ext/matrix_clip_space.inl", "external/glm/test/ext/ext_matrix_clip_space.cpp"]}

## Source file: assets/shaders/vertex_shader.glsl

{
  "path": "assets/shaders/vertex_shader.glsl",
  "blob_sha": "70d0ec4e5a782c10939d108a5e55aed7194f8d28",
  "sha256": "c2be7b088b8d31e5b4dd444fd40e30133267c39a70bfb07c9462843bbbe5d777",
  "lines": 22,
  "bytes": 602,
  "selection_score": 27,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/Vulpinii/skin-texture/blob/2b7c668d8f3958d29b7908fc62b7d4347d239ce5/assets/shaders/vertex_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "void main(){"
    }
  ]
}

```text

#version 410 core

// Input vertex data, different for all executions of this shader.
layout(location = 0) in vec3 vertexPosition_modelspace;
layout(location = 1) in vec2 vertexUV;
layout(location = 2) in vec3 vertexNormal_modelspace;

// Values that stay constant for the whole mesh.
uniform mat4 projection;
uniform mat4 modelGlobal;
uniform mat4 view;
uniform mat4 model;

out vec3 Normal;
out vec3 FragPos;

void main(){
	gl_Position =  projection * view * model * vec4(vertexPosition_modelspace,1);
	Normal = vertexNormal_modelspace;
	FragPos = (model * vec4(vertexPosition_modelspace,1)).xyz;
}



```

## Source file: assets/shaders/fragment_shader.glsl

{
  "path": "assets/shaders/fragment_shader.glsl",
  "blob_sha": "cdb68ae7edff88422e272af78c628fdfc1db9543",
  "sha256": "c1429b6f13a588a3b01bc33f92e0a4854587fd8ecb9306b1d0bd6f624bfb07bd",
  "lines": 241,
  "bytes": 7333,
  "selection_score": 27,
  "keyword_line_counts": {
    "skin": 11,
    "subsurface": 9,
    "sss": 3,
    "face": 11
  },
  "url": "https://github.com/Vulpinii/skin-texture/blob/2b7c668d8f3958d29b7908fc62b7d4347d239ce5/assets/shaders/fragment_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "float Random( in vec3 value){"
    },
    {
      "line": 24,
      "text": "float Noise3D( in vec3 uv ){"
    },
    {
      "line": 45,
      "text": "float FBMNoise3D6( in vec3 uv){"
    },
    {
      "line": 57,
      "text": "float hash( float n )"
    },
    {
      "line": 62,
      "text": "float thickness( in vec3 p, in vec3 n, float maxDist, float falloff )"
    },
    {
      "line": 78,
      "text": "vec3 sss(vec3 skin, in vec3 p, in vec3 n, in vec3 ro, in vec3 rd )"
    },
    {
      "line": 105,
      "text": "vec2 hash( vec2 p ) // replace this by something better"
    },
    {
      "line": 112,
      "text": "float noise( in vec2 p )"
    },
    {
      "line": 131,
      "text": "float n_noise(in vec2 p)"
    },
    {
      "line": 137,
      "text": "vec4 permute(vec4 x){return mod(((x*34.0)+1.0)*x, 289.0);}"
    },
    {
      "line": 138,
      "text": "vec4 taylorInvSqrt(vec4 r){return 1.79284291400159 - 0.85373472095314 * r;}"
    },
    {
      "line": 139,
      "text": "float snoise(vec3 v){"
    },
    {
      "line": 210,
      "text": "void main()"
    }
  ]
}

```text

#version 410 core
layout (location = 0) out vec4 FragColor;
layout (location = 1) out vec4 MaskColor;

in vec3 Normal;
in vec3 FragPos;

uniform vec3 lightPos;
uniform vec3 viewPos;
uniform vec3 lightColor;
uniform vec3 objectColor; // skin color
uniform vec3 freck_col;
uniform float freck_scale;
uniform float freck_frequency;

// math
const float PI = 3.14159265359;
const float DEG_TO_RAD = PI / 180.0;

float Random( in vec3 value){
    return fract(sin(dot(value,vec3(1274.0546,1156.01549,1422.65229)))*15554.0);
}

float Noise3D( in vec3 uv ){
    vec3 index = floor(uv);
    vec3 frac = fract(uv);

    float a = Random(index);
    float b = Random(index+vec3(1.0,0.0,0.0));
    float c = Random(index+vec3(0.0,1.0,0.0));
    float d = Random(index+vec3(1.0,1.0,0.0));

    float f = Random(index+vec3(0.0,0.0,1.0));
    float g = Random(index+vec3(1.0,0.0,1.0));
    float h = Random(index+vec3(0.0,1.0,1.0));
    float i = Random(index+vec3(1.0,1.0,1.0));

    frac = frac*frac*(3.0 - 2.0 * frac);

    return mix(mix(mix(a,b,frac.x),mix(c,d,frac.x),frac.y),
    mix(mix(f,g,frac.x),mix(h,i,frac.x),frac.y),
    frac.z);
}

float FBMNoise3D6( in vec3 uv){
    float fbm = Noise3D(uv*0.1)*0.5;
    fbm += Noise3D(uv*0.2)*0.25;
    fbm += Noise3D(uv*0.4)*0.125;
    fbm += Noise3D(uv*0.8)*0.0625;
    fbm += Noise3D(uv*0.16)*0.03125;
    fbm += Noise3D(uv*0.32)*0.03125;
    return fbm;
}
// based on
// https://colinbarrebrisebois.com/2011/03/07/gdc-2011-approximating-translucency-for-a-fast-cheap-and-convincing-subsurface-scattering-look/

float hash( float n )
{
    return fract(sin(n)*3538.5453);
}

float thickness( in vec3 p, in vec3 n, float maxDist, float falloff )
{
    const int nbIte = 7;
    const float nbIteInv = 1./float(nbIte);
    float ao = 0.0;

    for( int i=0; i<nbIte; i++ )
    {
        float l = hash(float(i))*maxDist;
        vec3 rd = normalize(-n)*l;
        ao += (l) / pow(1.+l, falloff);
    }

    return clamp( 1.-ao*nbIteInv, 0., 1.);
}

vec3 sss(vec3 skin, in vec3 p, in vec3 n, in vec3 ro, in vec3 rd )
{
    p.xz = mod(p.xz+100., 200.)-100.;

    vec3 ldir1 = normalize(lightPos-p);
    float latt1 = pow( length(lightPos-p)*.15, 3. ) / (pow(1.125-FBMNoise3D6(p*100.0f), 0.25)*1.45+.35);
    float thi = thickness(p, n, 6., 0.6);
    vec3 diff1 = lightColor * (max(dot(n,ldir1),0.) ) / latt1;

    vec3 col =  diff1;
    float trans1 =  pow( clamp( dot(-rd, -ldir1+n), 0., 1.), 1.) + 1.;
    col = skin * 0.008*(trans1/latt1)*thi ;
    return col;//max(col, col * FBMNoise3D6(p*100.0f));
}


//----SKIN----
// Partially inspired by works of Joseph Kubiak
float subsurface_scale = 1.1;
float subsurface_frequency = 2.2;
float skin_scale = 0.5;
float skin_frequency = 50.0;
float base_skin_amt = 0.98;
vec3 subsurface_color = vec3(0.639, 0.058, 0);
vec3 surface_col = vec3(1.0, 1.0, 1.0);

// Simplex 2D noise by Inigo Quilez
vec2 hash( vec2 p ) // replace this by something better
{
    p = vec2( dot(p,vec2(127.1,311.7)),
    dot(p,vec2(269.5,183.3)) );

    return -1.0 + 2.0*fract(sin(p)*43758.5453123);
}
float noise( in vec2 p )
{
    const float K1 = 0.366025404; // (sqrt(3)-1)/2;
    const float K2 = 0.211324865; // (3-sqrt(3))/6;

    vec2 i = floor( p + (p.x+p.y)*K1 );

    vec2 a = p - i + (i.x+i.y)*K2;
    vec2 o = step(a.yx,a.xy);
    vec2 b = a - o + K2;
    vec2 c = a - 1.0 + 2.0*K2;

    vec3 h = max( 0.5-vec3(dot(a,a), dot(b,b), dot(c,c) ), 0.0 );

    vec3 n = h*h*h*h*vec3( dot(a,hash(i+0.0)), dot(b,hash(i+o)), dot(c,hash(i+1.0)));

    return dot( n, vec3(70.0) );

}
float n_noise(in vec2 p)
{
    return 0.5 + 0.5 * noise(p);
}

//	Simplex 3D Noise by Ian McEwan, Ashima Arts
vec4 permute(vec4 x){return mod(((x*34.0)+1.0)*x, 289.0);}
vec4 taylorInvSqrt(vec4 r){return 1.79284291400159 - 0.85373472095314 * r;}
float snoise(vec3 v){
    const vec2  C = vec2(1.0/6.0, 1.0/3.0) ;
    const vec4  D = vec4(0.0, 0.5, 1.0, 2.0);

    // First corner
    vec3 i  = floor(v + dot(v, C.yyy) );
    vec3 x0 =   v - i + dot(i, C.xxx) ;

    // Other corners
    vec3 g = step(x0.yzx, x0.xyz);
    vec3 l = 1.0 - g;
    vec3 i1 = min( g.xyz, l.zxy );
    vec3 i2 = max( g.xyz, l.zxy );

    //  x0 = x0 - 0. + 0.0 * C
    vec3 x1 = x0 - i1 + 1.0 * C.xxx;
    vec3 x2 = x0 - i2 + 2.0 * C.xxx;
    vec3 x3 = x0 - 1. + 3.0 * C.xxx;

    // Permutations
    i = mod(i, 289.0 );
    vec4 p = permute( permute( permute(
    i.z + vec4(0.0, i1.z, i2.z, 1.0 ))
    + i.y + vec4(0.0, i1.y, i2.y, 1.0 ))
    + i.x + vec4(0.0, i1.x, i2.x, 1.0 ));

    // Gradients
    // ( N*N points uniformly over a square, mapped onto an octahedron.)
    float n_ = 1.0/7.0; // N=7
    vec3  ns = n_ * D.wyz - D.xzx;

    vec4 j = p - 49.0 * floor(p * ns.z *ns.z);  //  mod(p,N*N)

    vec4 x_ = floor(j * ns.z);
    vec4 y_ = floor(j - 7.0 * x_ );    // mod(j,N)

    vec4 x = x_ *ns.x + ns.yyyy;
    vec4 y = y_ *ns.x + ns.yyyy;
    vec4 h = 1.0 - abs(x) - abs(y);

    vec4 b0 = vec4( x.xy, y.xy );
    vec4 b1 = vec4( x.zw, y.zw );

    vec4 s0 = floor(b0)*2.0 + 1.0;
    vec4 s1 = floor(b1)*2.0 + 1.0;
    vec4 sh = -step(h, vec4(0.0));

    vec4 a0 = b0.xzyw + s0.xzyw*sh.xxyy ;
    vec4 a1 = b1.xzyw + s1.xzyw*sh.zzww ;

    vec3 p0 = vec3(a0.xy,h.x);
    vec3 p1 = vec3(a0.zw,h.y);
    vec3 p2 = vec3(a1.xy,h.z);
    vec3 p3 = vec3(a1.zw,h.w);

    //Normalise gradients
    vec4 norm = taylorInvSqrt(vec4(dot(p0,p0), dot(p1,p1), dot(p2, p2), dot(p3,p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    vec4 m = max(0.6 - vec4(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), 0.0);
    m = m * m;
    return 42.0*dot( m*m, vec4( dot(p0,x0), dot(p1,x1),
    dot(p2,x2), dot(p3,x3) ) );
}



void main()
{
    //----------------------------[ HUMAN SKIN ]----------------------------//
    float subsurface_radius = subsurface_scale / 2.0;
    float freck_radius = freck_scale / 2.0;
    vec3 uv = FragPos*10.0f;
    float subsurface_distance = snoise(uv * subsurface_frequency);
    float subsurface = 1.0 - min(1.0, subsurface_distance / subsurface_radius);
    float skin_value = snoise(uv * skin_frequency)/42.0 ;//* skin_scale;
    float freck_distance = n_noise(FragPos.zy/FragPos.x * freck_frequency);
    float freck = 1.0 - min(1.0, freck_distance / freck_radius);
    vec3 col = (subsurface_color * subsurface);
    col = mix(col,  objectColor , base_skin_amt);
    col  = mix(col, surface_col, skin_value);
    col = mix(col, freck_col, freck);

    //-----------------------------[ LIGHTING ]-----------------------------//
    // basic lighgting
    float ambientStrength = 0.001; vec3 ambient = ambientStrength * lightColor;
    vec3 norm = normalize(Normal); vec3 lightDir = normalize(lightPos - FragPos);
    float diff = max(dot(norm, lightDir), 0.0); vec3 diffuse = diff * lightColor;
    float specularStrength = 0.01; vec3 viewDir = normalize(viewPos - FragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
    vec3 specular = specularStrength * spec * lightColor;
    // SubSurface Scattering lighting
    vec3 sssCol = sss(objectColor, FragPos, Normal, viewPos, normalize(viewPos - FragPos));
    FragColor = vec4(mix(sssCol, sssCol*lightColor, 0.85) + ambient + specular,1.0);

    FragColor.xyz *= col;
    MaskColor = vec4(0,0,0,1);
}

```

## Source file: assets/shaders/depth_vertex_shader.glsl

{
  "path": "assets/shaders/depth_vertex_shader.glsl",
  "blob_sha": "d5147cc705d327d3d97466a97f4c5899d01ca7d8",
  "sha256": "3a20947b0d24e4c2ffc332a8b9c04d98782f109fc1b9b87af1c65a95b4b3a5ba",
  "lines": 10,
  "bytes": 221,
  "selection_score": 27,
  "keyword_line_counts": {},
  "url": "https://github.com/Vulpinii/skin-texture/blob/2b7c668d8f3958d29b7908fc62b7d4347d239ce5/assets/shaders/depth_vertex_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "void main(){"
    }
  ]
}

```text

#version 410 core
layout(location = 0) in vec3 aPos;
uniform mat4 lightSpaceMatrix;
uniform mat4 model;
uniform mat4 modelGlobal;

void main(){
	gl_Position =  lightSpaceMatrix * modelGlobal * model * vec4(aPos, 1.0);
}



```

## Source file: assets/shaders/light_vertex_shader.glsl

{
  "path": "assets/shaders/light_vertex_shader.glsl",
  "blob_sha": "066d09fcdb4c4c8516d38d836adff3742116c9d3",
  "sha256": "37194fee8e84a7fcec7b313300ae81fd172a10c1c1fb332cec8d7355ad3fc94c",
  "lines": 11,
  "bytes": 199,
  "selection_score": 27,
  "keyword_line_counts": {},
  "url": "https://github.com/Vulpinii/skin-texture/blob/2b7c668d8f3958d29b7908fc62b7d4347d239ce5/assets/shaders/light_vertex_shader.glsl",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "void main()"
    }
  ]
}

```text

#version 410 core
layout (location = 0) in vec3 aPos;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
	gl_Position = projection * view * model * vec4(aPos*0.5, 1.0);
}

```

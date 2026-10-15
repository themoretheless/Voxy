# Facial rendering source scan: santiagoballadares/ScreenSpaceSubsurfaceScattering

Repository: https://github.com/santiagoballadares/ScreenSpaceSubsurfaceScattering

Pinned source revision: a1924d4bd5711ebe038149a0599e5240dce3b2b5

Discovery topic: subsurface scattering

Description: Dissertation Project, Computer Game Programming MSc, Newcastle University, 2012

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["SSSSS/Shaders/accumSkinFrag.glsl"], "sss": ["Debug/SSS.exe", "Debug/SSS.ilk", "Debug/SSS.pdb", "Debug/SSSSS.exe", "Debug/SSSSS.ilk", "Debug/SSSSS.pdb", "SSS/Debug/CL.read.1.tlog", "SSS/Debug/CL.write.1.tlog", "SSS/Debug/Renderer.obj", "SSS/Debug/SSS.exe.intermediate.manifest", "SSS/Debug/SSS.lastbuildstate", "SSS/Debug/SSS.log", "SSS/Debug/SSS.obj", "SSS/Debug/SSS.tlog/CL.command.1.tlog", "SSS/Debug/SSS.tlog/CL.read.1.tlog", "SSS/Debug/SSS.tlog/CL.write.1.tlog", "SSS/Debug/SSS.tlog/SSS.lastbuildstate", "SSS/Debug/SSS.tlog/link.command.1.tlog", "SSS/Debug/SSS.tlog/link.read.1.tlog", "SSS/Debug/SSS.tlog/link.write.1.tlog", "SSS/Debug/cl.command.1.tlog", "SSS/Debug/link.command.1.tlog", "SSS/Debug/link.read.1.tlog", "SSS/Debug/link.read.2.tlog", "SSS/Debug/link.write.1.tlog", "SSS/Debug/mt.command.1.tlog", "SSS/Debug/mt.read.1.tlog", "SSS/Debug/mt.write.1.tlog", "SSS/Debug/vc100.idb", "SSS/Debug/vc100.pdb"], "shader": ["Framework/Debug/Shader.obj", "Framework/Release/Shader.obj", "Framework/Shader.cpp", "Framework/Shader.h", "SSS/Shaders/basicFrag.glsl", "SSS/Shaders/basicVert.glsl", "SSS/Shaders/beckmannFrag.glsl", "SSS/Shaders/beckmannVert.glsl", "SSS/Shaders/blurFrag.glsl", "SSS/Shaders/blurVert.glsl", "SSS/Shaders/lightFrag.glsl", "SSS/Shaders/lightVert.glsl", "SSS/Shaders/mainFrag.glsl", "SSS/Shaders/mainVert.glsl", "SSS/Shaders/shadowFrag.glsl", "SSS/Shaders/shadowVert.glsl", "SSS/Shaders/stretchFrag.glsl", "SSS/Shaders/stretchVert.glsl", "SSS/Shaders/unwrapFrag.glsl", "SSS/Shaders/unwrapVert.glsl", "SSSSS/Shaders/accumMarbleFrag.glsl", "SSSSS/Shaders/accumSkinFrag.glsl", "SSSSS/Shaders/basicFrag.glsl", "SSSSS/Shaders/basicVert.glsl", "SSSSS/Shaders/beckmannFrag.glsl", "SSSSS/Shaders/beckmannVert.glsl", "SSSSS/Shaders/blurFrag.glsl", "SSSSS/Shaders/blurVert.glsl", "SSSSS/Shaders/depthFrag.glsl", "SSSSS/Shaders/depthVert.glsl"], "rig": ["Framework/RigidBody.h"], "face": ["Framework/ChildMeshInterface.h"]}

## Source file: SSSSS/Shaders/accumSkinFrag.glsl

{
  "path": "SSSSS/Shaders/accumSkinFrag.glsl",
  "blob_sha": "7724712ea63e3661db621361546fd49bdd840e56",
  "sha256": "5b08312abffeb841e0ce5f321b2a583d987a5ae47cf3d90f9c8fab4fba4d35ce",
  "lines": 50,
  "bytes": 1359,
  "selection_score": 33,
  "keyword_line_counts": {
    "skin": 1,
    "sss": 2
  },
  "url": "https://github.com/santiagoballadares/ScreenSpaceSubsurfaceScattering/blob/a1924d4bd5711ebe038149a0599e5240dce3b2b5/SSSSS/Shaders/accumSkinFrag.glsl",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "void main(void) {"
    }
  ]
}

```text

#version 150 core

uniform sampler2D diffuseTex;

uniform sampler2D blurredTex1;
uniform sampler2D blurredTex2;
uniform sampler2D blurredTex3;
uniform sampler2D blurredTex4;

uniform bool useSSS;

// SKIN
const vec3 gaussWeights1 = vec3(0.240516183695, 0.447403391891, 0.615796108321); 
const vec3 gaussWeights2 = vec3(0.115857499765, 0.366176401412, 0.343917471552);
const vec3 gaussWeights3 = vec3(0.183619017698, 0.186420206697, 0.0);
const vec3 gaussWeights4 = vec3(0.460007298842, 0.0, 0.0402864201267);


in Vertex {
	vec2 texCoord;
} IN;

out vec4 fragColor;

void main(void) {
	if (!useSSS) {
		vec4 diffuse = texture(diffuseTex, IN.texCoord);
		fragColor = diffuse;
	}
	else {
		// Total diffuse
		vec3 diffuseLight = vec3(0.0);
	
		vec4 irrad1tap = texture(blurredTex1, IN.texCoord);
		vec4 irrad2tap = texture(blurredTex2, IN.texCoord);
		vec4 irrad3tap = texture(blurredTex3, IN.texCoord);
		vec4 irrad4tap = texture(blurredTex4, IN.texCoord);
	
		diffuseLight += gaussWeights1 * irrad1tap.xyz;
		diffuseLight += gaussWeights2 * irrad2tap.xyz;
		diffuseLight += gaussWeights3 * irrad3tap.xyz;
		diffuseLight += gaussWeights4 * irrad4tap.xyz;

		// Renormalize diffusion profiles to white
		//vec3 normConst = gaussWeights1 + gaussWeights2 + gaussWeights3 + gaussWeights4;
		//diffuseLight /= normConst;

		fragColor = vec4(diffuseLight, 1.0);
	}
}

```

## Source file: SSS/Shaders/blurFrag.glsl

{
  "path": "SSS/Shaders/blurFrag.glsl",
  "blob_sha": "5f62f36e838fbd97cfec7785b9c99a20f3e786d6",
  "sha256": "d75fc09ccf47464380914a5176cd491de7e4c9395fd7094a15d240c6c761fc4e",
  "lines": 78,
  "bytes": 1878,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/santiagoballadares/ScreenSpaceSubsurfaceScattering/blob/a1924d4bd5711ebe038149a0599e5240dce3b2b5/SSS/Shaders/blurFrag.glsl",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "void main(void) {"
    }
  ]
}

```text

#version 150 core

uniform sampler2D diffuseTex;
uniform sampler2D stretchTex;
uniform vec2 pixelSize;
uniform bool xAxis;
uniform bool useStretch;

// GaussWidth should be the standard deviation.  
float GaussWidth = 1.0f;
// Gaussian curve - standard deviation of 1.0
const float weights[7] = float[](0.006, 0.061, 0.242, 0.382, 0.242, 0.061, 0.006);

in Vertex {
	vec2 texCoord;
} IN;

out vec4 fragColor;

void main(void) {
	if (useStretch) {

		vec4 stretch = texture(stretchTex, IN.texCoord);
		vec4 sum = vec4(0.0);

		if (xAxis) {
			float netFilterWidth = pixelSize.x * GaussWidth * stretch.x;
			vec2 coords = IN.texCoord - vec2(netFilterWidth * 3.0, 0.0);

			for (int i = 0; i < 7; ++i) {
				vec4 tap = texture(diffuseTex, coords);
				sum += weights[i] * tap;
				coords += vec2(netFilterWidth, 0.0);
			}
		}
		else {
			float netFilterWidth = pixelSize.y * GaussWidth * stretch.y;
			vec2 coords = IN.texCoord - vec2(0.0, netFilterWidth * 3.0);

			for (int i = 0; i < 7; ++i) {
				vec4 tap = texture(diffuseTex, coords);
				sum += weights[i] * tap;
				coords += vec2(0.0, netFilterWidth);
			}
		}

		fragColor = sum;
		
	}
	else {

		vec2 values[7];
		if (xAxis) {
			values = vec2[](vec2(-pixelSize.x * 3.0, 0.0),
							vec2(-pixelSize.x * 2.0, 0.0),
							vec2(-pixelSize.x * 1.0, 0.0),
							vec2( 				0.0, 0.0),
							vec2( pixelSize.x * 1.0, 0.0),
							vec2( pixelSize.x * 2.0, 0.0),
							vec2( pixelSize.x * 3.0, 0.0));
		}
		else {
			values = vec2[](vec2(0.0, -pixelSize.y * 3.0),
							vec2(0.0, -pixelSize.y * 2.0),
							vec2(0.0, -pixelSize.y * 1.0),
							vec2(0.0, 				 0.0),
							vec2(0.0,  pixelSize.y * 1.0),
							vec2(0.0,  pixelSize.y * 2.0),
							vec2(0.0,  pixelSize.y * 3.0));
		}

		for (int i = 0; i < 7; ++i) {
			vec4 tmp = texture(diffuseTex, IN.texCoord + values[i]);
			fragColor += tmp * weights[i];
		}

	}
}


```

## Source file: SSS/Shaders/blurVert.glsl

{
  "path": "SSS/Shaders/blurVert.glsl",
  "blob_sha": "7ff11f697b0cf54014786667848db91af9a52c62",
  "sha256": "d52f5a5b891ae6e99f48a021caf110300cd6df6dfa40ada9e86af33df63883e3",
  "lines": 19,
  "bytes": 309,
  "selection_score": 18,
  "keyword_line_counts": {},
  "url": "https://github.com/santiagoballadares/ScreenSpaceSubsurfaceScattering/blob/a1924d4bd5711ebe038149a0599e5240dce3b2b5/SSS/Shaders/blurVert.glsl",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "void main(void) {"
    }
  ]
}

```text

#version 150 core

uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projMatrix;

in vec3 position;
in vec2 texCoord;

out Vertex {
	vec2 texCoord;
} OUT;

void main(void) {
	mat4 mvp = projMatrix * viewMatrix * modelMatrix;

	OUT.texCoord = texCoord;
	gl_Position = mvp * vec4(position, 1.0);
}

```

## Source file: SSS/Shaders/mainFrag.glsl

{
  "path": "SSS/Shaders/mainFrag.glsl",
  "blob_sha": "de75030ccb2e3e9c17fe7a182e847c5ce7fddd2b",
  "sha256": "46ce286eaaeed14b45950df3385e917b3dcee7ed9b0c6cf2161eca04d93914fc",
  "lines": 144,
  "bytes": 4687,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 2,
    "sss": 1,
    "face": 1
  },
  "url": "https://github.com/santiagoballadares/ScreenSpaceSubsurfaceScattering/blob/a1924d4bd5711ebe038149a0599e5240dce3b2b5/SSS/Shaders/mainFrag.glsl",
  "function_signature_candidates": [
    {
      "line": 51,
      "text": "float fresnel(vec3 halfVector, vec3 viewVector, float f0) {"
    },
    {
      "line": 58,
      "text": "float specularKSK(sampler2D beckmannTex, vec3 normal, vec3 lightVector, vec3 viewVector, float roughness) {"
    },
    {
      "line": 72,
      "text": "void main(void) {"
    }
  ]
}

```text

#version 150 core

uniform sampler2D diffuseTex;
uniform sampler2D bumpTex;
uniform sampler2DShadow shadowTex;
uniform sampler2D beckmannTex;

uniform sampler2D blurredTex1;
uniform sampler2D blurredTex2;
uniform sampler2D blurredTex3;
uniform sampler2D blurredTex4;
uniform sampler2D blurredTex5;
uniform sampler2D blurredTex6;

uniform vec3 cameraPos;
uniform vec3 lightPos;
uniform vec4 lightColour;
uniform float lightRadius;

uniform bool useTexture;
uniform bool useLighting;
uniform vec4 colour;

const float m = 0.3;
const float mix = 0.5;

uniform bool useBlur;

// RGB Gaussian weights that define skin profiles
const vec3 gaussWeights1 = vec3(0.233, 0.455, 0.649);
const vec3 gaussWeights2 = vec3(0.100, 0.336, 0.344);
const vec3 gaussWeights3 = vec3(0.118, 0.198, 0.000);
const vec3 gaussWeights4 = vec3(0.113, 0.007, 0.007);
const vec3 gaussWeights5 = vec3(0.358, 0.004, 0.000);
const vec3 gaussWeights6 = vec3(0.078, 0.000, 0.000);


in Vertex {
	vec4 colour;	
	vec2 texCoord;
	vec3 normal;
	vec3 tangent;
	vec3 binormal;
	vec3 worldPos;
	vec4 shadowProj;
} IN;

out vec4 fragColor;

// Compute Schlick fresnel reflectance approximation:
float fresnel(vec3 halfVector, vec3 viewVector, float f0) {
    float base = 1.0 - dot(viewVector, halfVector);
    float exponential = pow(base, 5.0);
    return exponential + f0 * (1.0 - exponential);
}

// Compute Kelemen/Szirmay-Kalos specular with a beckmann texture:
float specularKSK(sampler2D beckmannTex, vec3 normal, vec3 lightVector, vec3 viewVector, float roughness) {
    vec3 halfVector = lightVector + viewVector;
    vec3 halfVectorN = normalize(halfVector);

    float NdotL = max(0.0, dot(normal, lightVector));
    float NdotH = max(0.0, dot(normal, halfVectorN));

    float ph = pow( texture(beckmannTex, vec2(NdotH, roughness)).r * 2.0, 10.0 );
    float f = fresnel(halfVectorN, viewVector, 0.028);
    float ksk = max(0.0, ph * f / dot(halfVector, halfVector));

    return NdotL * ksk;   
}

void main(void) {
// ------ blinn - phong -----
	vec4 diffuse = texture(diffuseTex, IN.texCoord);
	
	mat3 TBN = mat3(IN.tangent, IN.binormal, IN.normal);
	vec3 normal = normalize(TBN * (texture(bumpTex, IN.texCoord).rgb * 2.0 - 1.0));

	vec3 incident = normalize(lightPos - IN.worldPos);
	vec3 viewDir = normalize(cameraPos - IN.worldPos);
	vec3 halfDir = normalize(incident + viewDir);

	float dist = length(lightPos - IN.worldPos);
	float atten = 1.0 - clamp(dist / lightRadius, 0.0, 1.0);

	float lambert = max(0.0, dot(incident, normal));
	float rFactor = max(0.0, dot(halfDir, normal));
	float sFactor = pow(rFactor, 50.0);

	float shadow = 1.0;
	if(IN.shadowProj.w > 0.0) {
		shadow = textureProj(shadowTex, IN.shadowProj);
	}
	lambert *= shadow;

	vec3 colour = (diffuse.rgb * lightColour.rgb);
	colour += (lightColour.rgb * sFactor) * 0.33;
// ----------------------------------------------------------

// ----- sss ------
	// The total diffuse light exiting the surface
	vec3 diffuseLight = vec3(0.0);
	
	vec4 irrad1tap = texture( blurredTex1, IN.texCoord );
	vec4 irrad2tap = texture( blurredTex2, IN.texCoord );
	vec4 irrad3tap = texture( blurredTex3, IN.texCoord );
	vec4 irrad4tap = texture( blurredTex4, IN.texCoord );
	vec4 irrad5tap = texture( blurredTex5, IN.texCoord );
	vec4 irrad6tap = texture( blurredTex6, IN.texCoord );
	
	diffuseLight += gaussWeights1 * irrad1tap.xyz;
	diffuseLight += gaussWeights2 * irrad2tap.xyz;
	diffuseLight += gaussWeights3 * irrad3tap.xyz;
	diffuseLight += gaussWeights4 * irrad4tap.xyz;
	diffuseLight += gaussWeights5 * irrad5tap.xyz;
	diffuseLight += gaussWeights6 * irrad6tap.xyz;

	// Renormalize diffusion profiles to white
	vec3 normConst = gaussWeights1 + gaussWeights2 + gaussWeights3 + gaussWeights4 + gaussWeights5 + gaussWeights6;
	diffuseLight /= normConst; // Renormalize to white diffuse light
	
	// Calculate vectors for light shading:
	vec3 lightVector = normalize(lightPos - IN.worldPos);
	vec3 viewVector = normalize(cameraPos - IN.worldPos);
	
	// calculate specular lighting
	float specular = specularKSK(beckmannTex, normal, lightVector, viewVector, m);

	// Determine skin color from a diffuseColor map
	fragColor = vec4( (diffuseLight * pow(diffuse.rgb, vec3(1.0 - mix))) + vec3(specular), 1.0 );
	//fragColor = vec4( vec3(diffuseLight * diffuse.rgb), 1.0 );
	//fragColor = vec4( diffuseLight, 1.0 );
		
	//fragColor = texture(bluredTex1, IN.texCoord) * pow(diffuse, vec4(1.0 - mix));
	//fragColor = vec4(texture(bluredTex1, IN.texCoord).rgb * diffuse.rgb, diffuse.a);
	//fragColor.rgb += sFactor * 0.33;

//-----------------------------------------------------------

	if (!useBlur) {
		fragColor = diffuse * lambert;
		fragColor.a = 1.0f;
	}
}

```

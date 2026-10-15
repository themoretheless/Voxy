# Facial rendering source scan: lauris47/SkinShader

Repository: https://github.com/lauris47/SkinShader

Pinned source revision: 4901d00e1bcce36b1b0da5d85ac81e7f9578795f

Discovery topic: skin shader

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": [".vs/SkinShader/v14/.suo", "Assets/SkinMaterial.mat", "Assets/SkinMaterial.mat.meta", "Assets/SkinShader.shader", "Assets/SkinShader.shader.meta", "Assets/SkinShader2.shader", "Assets/SkinShader2.shader.meta", "SkinShader.sln", "SkinShader.userprefs"], "teeth": ["Assets/Materials/TeethWhite.mat", "Assets/Materials/TeethWhite.mat.meta"], "cornea": ["Assets/Materials/Cornea.mat", "Assets/Materials/Cornea.mat.meta"], "shader": [".vs/SkinShader/v14/.suo", "Assets/SkinShader.shader", "Assets/SkinShader.shader.meta", "Assets/SkinShader2.shader", "Assets/SkinShader2.shader.meta", "Library/ShaderCache.db", "Library/shadercompiler-UnityShaderCompiler.exe0.log", "Library/shadercompiler-UnityShaderCompiler.exe1.log", "Library/shadercompiler-UnityShaderCompiler.exe2.log", "Library/shadercompiler-UnityShaderCompiler.exe3.log", "Library/shadercompiler-UnityShaderCompiler.exe4.log", "SkinShader.sln", "SkinShader.userprefs"]}

## Source file: Assets/SkinShader.shader

{
  "path": "Assets/SkinShader.shader",
  "blob_sha": "751e027da385278db29c9fd9fe6a7122cb95306a",
  "sha256": "e39ba11b01b26e16a0b194995f44978cf856cf7ed7bb9534280508d234cb9761",
  "lines": 179,
  "bytes": 6409,
  "selection_score": 42,
  "keyword_line_counts": {
    "skin": 8,
    "shader": 2,
    "face": 3
  },
  "url": "https://github.com/lauris47/SkinShader/blob/4901d00e1bcce36b1b0da5d85ac81e7f9578795f/Assets/SkinShader.shader",
  "function_signature_candidates": [
    {
      "line": 72,
      "text": "float4 frog(vertexOut input) : COLOR {  //Float4 as a COLOR for unity"
    },
    {
      "line": 140,
      "text": "float4 frog(vertexOut input) : COLOR{"
    }
  ]
}

```text

﻿// Upgrade NOTE: replaced '_Object2World' with 'unity_ObjectToWorld'

// Upgrade NOTE: replaced '_Object2World' with 'unity_ObjectToWorld'
// Upgrade NOTE: replaced '_World2Object' with 'unity_WorldToObject'

// Upgrade NOTE: replaced '_Object2World' with 'unity_ObjectToWorld'

// Upgrade NOTE: replaced '_World2Object' with 'unity_WorldToObject'

// Upgrade NOTE: replaced '_World2Object' with 'unity_WorldToObject'


Shader "My Shaders/Skin Shader"
{
	Properties
	{
		_Color("Color", Color) = (1.0, 0.8, 0.6, 1) //Main color will be simmilar Skin Color by default
		_Specular("Specular Color", Color) = (0.9, 0.5, 0.4, 1) //Main color will be simmilar Skin Color by default
		_Shininess("Shininess", Range(0.1,10)) = 5
		_Transparency("Transparency", Range(0.1,10)) = 5
		_TransparetCut("TransparetCut", Range(-1,1)) = 1
		_Texture("Texture",2D) = "White" {}
	}
		SubShader{


		Pass {
			Tags {"LightMode" = "ForwardBase"}

			CGPROGRAM
			#pragma vertex vort
			#pragma fragment frog


			uniform float4 _Color, _Specular;
					float _Shininess;
			uniform float4 addShade;
			uniform sampler2D _Texture;


			float4 lightDir0;
			float lightStrength;


			//Unity will recognize this:
			float4 _LightColor0;


			struct vertexIn {
				float4 vertexPos : POSITION;
				float4 colorOfTexture : TEXCOORD0;
			};

			struct vertexOut {
				float4 pos : SV_POSITION;
				float4 tex1 : TEXCOORD0;
				float4 posWorld : TEXCOORD1;
				float4 normal: NORMAL;
			};

			vertexOut vort(vertexIn input) {
				vertexOut output;

				output.tex1 = input.colorOfTexture; //Will multiply face normal direction with world2object, to make normal vectors of object into world normals size
				output.posWorld = mul(unity_ObjectToWorld, input.vertexPos);
				output.pos = mul(UNITY_MATRIX_MVP, input.vertexPos); //Position shape verts in correcto position inside unity

				return output;
			}

			//Works per fragment/pixel, computationaly expensive in comparison to vertex program
			float4 frog(vertexOut input) : COLOR {  //Float4 as a COLOR for unity

				float distance;
				float lightStrength;
				//Vertex Vectors
				float3 normalDirection = mul(unity_ObjectToWorld, input.normal);; // input from vertex function
				float3 lightPosition = normalize(_WorldSpaceLightPos0.xyz); //_WorldSpaceLightDir0 is predefined, so no need to inniciate it at the top, it's also normalized automatically
				float3 cameraAngleToNormal = max(0.0, float3(1, 1, 1) - dot(normalDirection, _WorldSpaceCameraPos)); //Angle between face and camera. I want skin to be shiny when agnle is closer to 0, therefor I inveert it by subs, both in dot() are normalized before
				float3 cameraToVertexAngle = normalize(_WorldSpaceCameraPos - input.posWorld); // Normalized vector length from vertex to camera. Not using defined before normalDirection, becasue I don't want to normalize it alone

				//float4 textureColor = sampler2D(_Texture, input.normalDirection.xy);

				//Lighting
				float4 textureColor = tex2D(_Texture, input.tex1.xy);
				float3 diffuseShading = _LightColor0.xyzw * max(0.0, dot(normalDirection, lightPosition)); //Values from 0 to 1 with max() help. dot() will return higher value if angle is smallest, that is why objects are lit the most, in straighest line to the vertex point (they have closest to 0 angle, which will produce closest to 1 result)
				float3 specularReflection = max(0.0, dot(normalDirection, lightPosition) * pow(dot(reflect(-lightPosition, normalDirection), cameraToVertexAngle), _Shininess));

				//max(0.0, dot(normalDirection, lightPosition)) * pow(max(0.0, dot(reflect(-lightPosition, normalDirection), cameraToVertexAngle)), _Shininess);
				//To make skin shine against light source

				float3 skinAgainstLightShine = max(0.0, cameraAngleToNormal *  dot(lightPosition, normalDirection)); //Fix: shine oposite the light still appears
				float3 lightFinal = _Color * diffuseShading  + specularReflection + skinAgainstLightShine;

				return float4(lightFinal, 1.0);// + UNITY_LIGHTMODEL_AMBIENT/*+ skinAgainstLightShine Specular light does almost the same*/, 1.0); // Color is normal direction multiplied by world position of object in its local position.

			}
			ENDCG

		}
		//For spot Light
		Pass{
			Tags{ "LightMode" = "ForwardAdd" }
			Blend One One

			CGPROGRAM
			#pragma vertex vort
			#pragma fragment frog

			uniform float4 _Color, _Specular;
					float _Shininess;
			uniform float4 addShade;
			uniform sampler2D _Texture;

			float4 lightDir0;
			float lightStrength;
			float4 _LightColor0;

			struct vertexIn {
				float4 vertexPos : POSITION;
				float4 normal: NORMAL;
				float4 colorOfTexture: TEXCOORD0;
			};

			struct vertexOut {
				float4 pos : SV_POSITION;
				float4 tex1 : TEXCOORD0;
				float4 posWorld : TEXCOORD1;
			};

			vertexOut vort(vertexIn input) {
				vertexOut output;
				output.tex1 = input.colorOfTexture;
				output.posWorld = normalize(mul(unity_ObjectToWorld, input.vertexPos));
				output.pos = mul(UNITY_MATRIX_MVP, input.vertexPos);

				return output;
			}

			float4 frog(vertexOut input) : COLOR{
			
				float lightDistance;
				float lightStrength;
				float3 normalDirection = input.tex1;
				float3 lightPosition;
				float3 cameraAngleToNormal = max(0.0, float3(1, 1, 1) - normalize(dot(normalDirection, _WorldSpaceCameraPos))); 
				float3 cameraToVertexAngle = normalize(_WorldSpaceCameraPos - input.posWorld); 
				lightStrength = 0;

					
				if (!_WorldSpaceLightPos0.w == 0.0) {

					lightDistance = length(_WorldSpaceLightPos0.xyz - mul(unity_ObjectToWorld, input.posWorld));
					lightStrength = 1 / lightDistance;
					lightPosition = normalize(_WorldSpaceLightPos0.xyz - mul(unity_ObjectToWorld, input.posWorld));
				}
				else {
					//This will never execute
					lightStrength = 1;
					lightPosition = normalize(_WorldSpaceLightPos0.xyz);
				}
				float4 textureColor = tex2D(_Texture, input.tex1.xy);
				float3 diffuseShading = _LightColor0.xyzw * max(0.0, dot(normalDirection, lightPosition)) * lightStrength;
				float3 specularReflection = max(0.0, dot(normalDirection, lightPosition) * pow(dot(reflect(-lightPosition, normalDirection), cameraToVertexAngle), _Shininess));
				float3 lightFinal = diffuseShading + specularReflection;

				return float4(lightFinal,1);
			}
				ENDCG
			}
	}
		Fallback "Diffuse"
}


//Call back
//Call front
//and then -normal
// to make back faces als orenderey

```

## Source file: Assets/SkinShader2.shader

{
  "path": "Assets/SkinShader2.shader",
  "blob_sha": "d0654eaa69656b41def553544dd90e8bcffa2126",
  "sha256": "ccdd7a865d8f0415cf3c07ba11b65a0308487b2ac537128046f92a80c168b1e0",
  "lines": 260,
  "bytes": 9728,
  "selection_score": 42,
  "keyword_line_counts": {
    "skin": 15,
    "shader": 16,
    "face": 1
  },
  "url": "https://github.com/lauris47/SkinShader/blob/4901d00e1bcce36b1b0da5d85ac81e7f9578795f/Assets/SkinShader2.shader",
  "function_signature_candidates": [
    {
      "line": 102,
      "text": "float4 fragmentShader(vertexShaderOut input) : COLOR{"
    },
    {
      "line": 214,
      "text": "float4 fragmentShader(vertexShaderOut input) : COLOR{"
    }
  ]
}

```text

﻿// Upgrade NOTE: replaced '_Object2World' with 'unity_ObjectToWorld'

Shader "My Shaders/Skin Shader New" {
	Properties{
		_Texture("Texture", 2D) = "White" {}
		_BumpMap("BumpMap", 2D) = "bump" {}
		_bumpStrength("Bump Strength", Range(-1, 1)) = 0.2
		//Create if statement, if there is no texture, so it would use color instead. 
		//_Color("Color", Color) = (1,0,0,1)
		_specularColor("Specular Color", Color) = (1,0,1,1)
		_specularRollof("Specular Rollof", Range(0.1, 10)) = 5
		_specularSize("Specular Size", Range(-2, 0)) = 0.5

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

		//Look at:
		// for layers https://en.wikibooks.org/wiki/Cg_Programming/Unity/Layers_of_Textures
		// Transparency https://en.wikibooks.org/wiki/Cg_Programming/Unity/Transparencys
=======
=======
>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
>>>>>>> parent of 80ad9b7... SpecularMap Works
		_skinShineColor("Skin Shine Color", Color) = (0,0,1,1)
		_skinShinePower("Skin Shine Strength", Range(0, 1)) = 0.5

>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
=======
>>>>>>> parent of 80ad9b7... SpecularMap Works
		_skinShineColor("Skin Shine Color", Color) = (0,0,1,1)
		_skinShinePower("Skin Shine Strength", Range(0, 1)) = 0.5

>>>>>>> parent of 80ad9b7... SpecularMap Works

	}
		SubShader{


			Pass{
			Tags{ "LightMode" = "ForwardBase" }

			CGPROGRAM

			//float4 _Color;
			float4 lightDirection;
			float4 _skinShineColor;
			float4 _specularColor;
			float _specularRollof;
			float _specularSize;
			float4 _LightColor0;
			sampler2D _Texture;
			sampler2D _BumpMap;
			float lightStrength;
			float _skinShinePower;
			float _bumpStrength;

			#pragma vertex vertexShader
			#pragma fragment fragmentShader

			struct vertexShaderIn {
				float4 vertexPos : POSITION;
				float3 normal : NORMAL;
				float4 colorOfTexture : TEXCOORD0;
				float4 tangent : TANGENT; // Vector that is tangent to the face normal 


			};

			struct vertexShaderOut {
				float4 color : TEXCOORD0;
				float4 pos : SV_POSITION;
				float3 normal : NORMAL;
				float4 colorOfTexture : TEXCOORD1;
				float3 worldVertPos : TEXCOORD2;

				float3 worldTangent : TEXCOORD3; //New
				float3 worldNormal : TEXCOORD4; //New
				float3 worldbiNormal :TEXCOORD5; //New
			};


			vertexShaderOut vertexShader(vertexShaderIn input) {
				vertexShaderOut o;
				o.worldVertPos = normalize(mul(unity_ObjectToWorld, input.vertexPos));

				o.worldNormal = normalize(mul(input.normal, unity_ObjectToWorld )); //Bump map, normal as a world normal direction
				o.worldTangent = normalize(mul(unity_ObjectToWorld, input.tangent)); //Bump map, local tangent vector to world tangent of normal
				o.worldbiNormal = normalize(cross(o.worldNormal, o.worldTangent) * input.tangent.w); //Bump map, binormal vector, perpendicular to normal and tangent vectors, multiplying it by tangent, gets it to correct length

				o.colorOfTexture = input.colorOfTexture;
				o.normal = input.normal;
				o.pos = mul(UNITY_MATRIX_MVP, input.vertexPos); //Always have this, This will project correct vertex "filled siluet shape"

				return o;
			}

			float4 fragmentShader(vertexShaderOut input) : COLOR{

				//General
				float3 lightPosition = normalize(_WorldSpaceLightPos0.xyz);
				float3 cameraDirection = normalize(_WorldSpaceCameraPos);

				//Texture
				float4  colorOfTexture = tex2D(_Texture, input.colorOfTexture.xy);

				//BumMap
				float4 bumpMap = tex2D(_BumpMap, input.colorOfTexture.xy); 
				//Unpack normal 
				float3 localCoords = float3(2.0 * bumpMap.ag - float2(1.0, 1.0), 0.0);
				localCoords.z = _bumpStrength;  
				//Normal transpose matrix
				float3x3 local2WorldTranspose = float3x3(input.worldTangent, input.worldbiNormal, input.worldNormal);


				float3 normalDirection = normalize(mul(localCoords, local2WorldTranspose)); //Always have this, will covert local normal vectors to world. Without this light would casted only on predefined side


				//Difuse shading
				float4 diffuseShading = max(0.0, dot(normalDirection, lightPosition)) * _LightColor0 + UNITY_LIGHTMODEL_AMBIENT; // dot() will return higher value if angle is smallest, that is why objects are lit the most, in straighest line to the vertex point (they have closest to 0 angle, which will produce closest to 1 result)

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				//Specular shading
				float4 specularMap = tex2D(_SpecularMap, input.colorOfTexture.xy);	
				//float4 specularShading = dot(reflect(normalDirection, -lightPosition), _WorldSpaceCameraPos) * specularMap;
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) * _LightColor0  * _specularColor * specularMap * max(0.0, dot(normalDirection, lightPosition));
=======
																																 //Specular Shading
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) *  _LightColor0 * _specularColor;
>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
																																 //Specular Shading
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) *  _LightColor0 * _specularColor;
>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
																																 //Specular Shading
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) *  _LightColor0 * _specularColor;
>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
																																 //Specular Shading
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) *  _LightColor0 * _specularColor;
>>>>>>> parent of 80ad9b7... SpecularMap Works
=======
																																 //Specular Shading
				float4 specularShading = pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) *  _LightColor0 * _specularColor;
>>>>>>> parent of 80ad9b7... SpecularMap Works

				//Skin shine against light. First skin layer of "Oil"
				float4 skinShine = max(0.0, (1 - dot(lightPosition, _WorldSpaceCameraPos))) * max(0.0, dot(normalDirection, lightPosition) * _skinShineColor) * _skinShinePower;


				//Final
				float4 lightFinal = (diffuseShading * colorOfTexture) + skinShine + specularShading;

				float4 blendTest = lerp(colorOfTexture, lightFinal,  diffuseShading);

				return lightFinal;
			}
				ENDCG
			}


			//Pass for other lights than directional
			Pass{

				Tags{ "LightMode" = "ForwardAdd" }
				Blend One One
				CGPROGRAM

				//float4 _Color;
				float4 lightDirection;
				float4 _skinShineColor;
				float4 _specularColor;
				float _specularRollof;
				float _specularSize;
				float4 _LightColor0;
				sampler2D _Texture;
				float lightStrength;
				float _skinShinePower;

				#pragma vertex vertexShader
				#pragma fragment fragmentShader

				struct vertexShaderIn {
					float4 vertexPos : POSITION;
					float3 normal : NORMAL;
					float4 colorOfTexture : TEXCOORD0;
				};

				struct vertexShaderOut {
					float4 pos : SV_POSITION;
					float3 normal : NORMAL;
					float4 colorOfTexture : TEXCOORD0;
					float3 worldVertPos : TEXCOORD1;
				};

				vertexShaderOut vertexShader(vertexShaderIn input) {
					vertexShaderOut o;
					o.worldVertPos = normalize(mul(unity_ObjectToWorld, input.vertexPos));
					o.colorOfTexture = input.colorOfTexture;
					o.normal = input.normal;
					o.pos = mul(UNITY_MATRIX_MVP, input.vertexPos); //Always have this, This will project correct vertex "filled siluet shape"
					return o;
				}

				float4 fragmentShader(vertexShaderOut input) : COLOR{

					//General
					float3 normalDirection = normalize(mul(unity_ObjectToWorld, input.normal)); //Always have this, will covert local normal vectors to world. Without this light would casted only on predefined side
					float3 lightPosition = _WorldSpaceLightPos0.xyz;
					float3 cameraDirection = normalize(_WorldSpaceCameraPos);

					//Calculate strenght of other lights than directional
					if (_WorldSpaceLightPos0.w != 0.0) {
						float distance = length(lightPosition - input.worldVertPos);
						lightStrength = 1/pow(distance, 4); // pow() of 4 could be changed to user accesable variable
						lightPosition = normalize(lightPosition - input.worldVertPos);
					}
					else {
						lightStrength = 1;
					}

					//Difuse shading
					float4 diffuseShading = lightStrength * max(0.0, (dot(normalDirection, lightPosition))) * _LightColor0 ; // dot() will return higher value if angle is smallest, that is why objects are lit the most, in straighest line to the vertex point (they have closest to 0 angle, which will produce closest to 1 result)

					//Specular Shading
					float4 specularShading = lightStrength * pow(max(0.0, dot(cameraDirection, reflect(-lightPosition, normalDirection)) + _specularSize), _specularRollof) * _LightColor0;

					//Skin shine against light. First skin layer of "Oil"
					float4 skinShine = max(0.0, (1 - dot(lightPosition, _WorldSpaceCameraPos))) * max(0.0, dot(normalDirection, lightPosition) * _skinShineColor) * _skinShinePower;

					//Texture
					float4 colorOfTexture = tex2D(_Texture, input.colorOfTexture.xy);

					//Final
					float4 lightFinal = (diffuseShading * colorOfTexture) + skinShine + specularShading;

					return lightFinal ;
				}
					ENDCG
			}
			
		}
		Fallback "Diffuse"
		
}

//CG TEXCUBE

// Of screen rendering (camera rendering fx for mirror)

// Unity reflection probe

```

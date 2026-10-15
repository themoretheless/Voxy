# Facial rendering source scan: liuxq/SkinShader

Repository: https://github.com/liuxq/SkinShader

Pinned source revision: b3b567e413db1b933df1faca250834444a3a7db1

Discovery topic: skin shader

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Assets/Marmoset/Shader/Beta/Skin.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Hair IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Hair IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Cutout IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Cutout IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Detail IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Detail IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkin.cginc", "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkin.cginc.meta", "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkinInput.cginc", "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkinInput.cginc.meta"], "shader": ["Assets/Marmoset/Documentation/Shaders.txt", "Assets/Marmoset/Documentation/Shaders.txt.meta", "Assets/Marmoset/Shader/Beta.meta", "Assets/Marmoset/Shader/Beta/Editor.meta", "Assets/Marmoset/Shader/Beta/Editor/DrawLine.cs", "Assets/Marmoset/Shader/Beta/Editor/DrawLine.cs.meta", "Assets/Marmoset/Shader/Beta/Editor/LayerInspector.cs", "Assets/Marmoset/Shader/Beta/Editor/LayerInspector.cs.meta", "Assets/Marmoset/Shader/Beta/Layered.meta", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 2-Layer IBL.shader", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 2-Layer IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 2-Layer Vertex IBL.shader", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 2-Layer Vertex IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 4-Layer IBL.shader", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 4-Layer IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 4-Layer Vertex IBL.shader", "Assets/Marmoset/Shader/Beta/Layered/Marmoset 4-Layer Vertex IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Marmoset Refractive IBL.shader", "Assets/Marmoset/Shader/Beta/Marmoset Refractive IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Hair IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Hair IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Cutout IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Cutout IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Detail IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Detail IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader.meta", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader", "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader.meta"]}

## Source file: Assets/Marmoset/Shader/Beta/Skin/MarmosetSkin.cginc

{
  "path": "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkin.cginc",
  "blob_sha": "db0c1ec3aa5392d01a5ecde4bcef1386f43a04c8",
  "sha256": "3b2956c2340a5c002eeb655741da51ad8e13de37af98b64917ebf4dbc2253194",
  "lines": 383,
  "bytes": 11041,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 6,
    "skin": 19,
    "rig": 1
  },
  "url": "https://github.com/liuxq/SkinShader/blob/b3b567e413db1b933df1faca250834444a3a7db1/Assets/Marmoset/Shader/Beta/Skin/MarmosetSkin.cginc",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "inline float diffuseFresnel(float eyeDP, float scatter) {"
    },
    {
      "line": 14,
      "text": "inline float3 wrapLighting(float DP, float3 scatter) {"
    },
    {
      "line": 80,
      "text": "void MarmosetSkinVert (inout appdata_full v, out Input o) {"
    },
    {
      "line": 91,
      "text": "void MarmosetSkinSurf(Input IN, inout MarmosetSkinOutput OUT) {"
    }
  ]
}

```text

// Marmoset Skyshop
// Copyright 2013 Marmoset LLC
// http://marmoset.co

#ifndef MARMOSET_SKIN_CGINC
#define MARMOSET_SKIN_CGINC

inline float diffuseFresnel(float eyeDP, float scatter) {
	eyeDP = 1.0 - eyeDP;
	float dp4 = eyeDP * eyeDP; dp4 *= dp4;
	return lerp(dp4, eyeDP*0.4, scatter);		//0.4 is energy conserving integral
}

inline float3 wrapLighting(float DP, float3 scatter) {
	scatter *= 0.5;
	float3 integral = float3(1.0,1.0,1.0)-scatter*_ConserveEnergy;
	float3 light = saturate(DP*integral + scatter);
	float shadow = (DP*0.5+0.5);
	shadow *= shadow;
	return light * integral * shadow;
}

inline half4 LightingMarmosetSkinDirect( MarmosetSkinOutput s, half3 lightDir, half3 viewDir, half atten ) {
	half4 frag = half4(0.0,0.0,0.0,s.Alpha);
	#if defined(MARMO_DIFFUSE_DIRECT) || defined(MARMO_SPECULAR_DIRECT)
		half3 L = lightDir;
		half3 N = s.Normal;
		#ifdef MARMO_HQ
			L = normalize(L);
		#endif
	#endif
	
	float selfShadow = 1.0;
	#ifdef MARMO_DIFFUSE_DIRECT
		half dp = dot(N,L);
		half3 diff = wrapLighting(dp, s.Subdermis); //*2.0 to match Unity
		diff *= s.Albedo.rgb * 2.0;
		frag.rgb = diff;
		selfShadow = saturate(10.0*dp);
	#else
		selfShadow = saturate(10.0*dot(N,L));
	#endif
	
	#ifdef MARMO_SPECULAR_DIRECT
		half3 H = viewDir+L;
		#ifdef MARMO_SPECULAR_ANISO
			float3 T = s.anisoTangent;
			H = H - T*dot(H,T)*_Aniso;
		#endif
		H = normalize(H);
		float specRefl = saturate(dot(N,H));
		half3 spec = pow(specRefl, s.Specular*256.0);
		#ifdef MARMO_HQ
			spec *= selfShadow;			
		#endif
		frag.rgb += (0.5 * spec) * s.SpecularRGB; //*0.5 to match Unity
	#endif

	diff *= s.Albedo.rgb * 2.0;
	
	#ifdef MARMO_SKIN_DIRECT		
		#ifdef MARMO_TRANSLUCENCY_DIRECT
			//TRANSLUCENCY
			float transdp = dot(-N,L);
			half3 trans = wrapLighting(transdp, float3(1.0,1.0,1.0)); //*2.0 to match Unity
			trans *= s.Translucency;
		#else
			half3 trans = half3(0.0,0.0,0.0);
		#endif		
		//PEACH-FUZZ
		float fuzzdp = saturate(dp*_FuzzOcc + (1.0-_FuzzOcc));
		trans += s.Fuzz * _FuzzColor.rgb * fuzzdp;
				
		frag.rgb += trans * _ExposureIBL.w;
	#endif
	frag.rgb *= atten * _LightColor0.rgb;
	return frag;
}

void MarmosetSkinVert (inout appdata_full v, out Input o) {
	UNITY_INITIALIZE_OUTPUT(Input,o);
	o.texcoord = v.texcoord.xy;
	#ifdef MARMO_OCCLUSION
		o.texcoord1 = v.texcoord1.xy;
	#endif
	#ifdef MARMO_VERTEX_COLOR
		o.color = v.color;
	#endif
}

void MarmosetSkinSurf(Input IN, inout MarmosetSkinOutput OUT) {
	#define uv_diff TRANSFORM_TEX(IN.texcoord.xy, _MainTex)
	#define uv_spec TRANSFORM_TEX(IN.texcoord.xy, _SpecTex)
	#define uv_bump TRANSFORM_TEX(IN.texcoord.xy, _BumpMap)
	#define uv_glow TRANSFORM_TEX(IN.texcoord.xy, _Illum)
	#define uv_occ  TRANSFORM_TEX(IN.texcoord1.xy, _OcclusionMap)
	
	#define uv_detail    TRANSFORM_TEX(IN.texcoord.xy, _DetailMap)
	#define uv_trans     TRANSFORM_TEX(IN.texcoord.xy, _TranslucencyMap)
	#define uv_subdermis TRANSFORM_TEX(IN.texcoord.xy, _SubdermisTex)
	
	#ifdef MARMO_SKY_BLEND
		half4 exposureIBL = lerp(_ExposureIBL1, _ExposureIBL, _BlendWeightIBL);
	#else
		half4 exposureIBL = _ExposureIBL;
	#endif
	exposureIBL.xy *= _UniformOcclusion.xy;
	
	#if LIGHTMAP_ON
		exposureIBL.xy *= _ExposureLM;
	#endif
	half4 baseColor = _Color;
	#ifdef MARMO_VERTEX_COLOR
		baseColor *= IN.color;
	#endif

	#ifdef MARMO_DIFFUSE_SPECULAR_COMBINED
		half4 diffspec = half4(1.0,1.0,1.0,1.0);
	#endif
		
	//DIFFUSE
	#if defined(MARMO_DIFFUSE_DIRECT) || defined(MARMO_DIFFUSE_IBL)
		half4 diff = tex2D( _MainTex, uv_diff );
		#ifdef MARMO_DIFFUSE_SPECULAR_COMBINED
			diffspec = diff.aaaa;
		#endif
		diff *= baseColor;
		OUT.Albedo = diff.rgb;
		OUT.Alpha = diff.a;
		#ifdef MARMO_PREMULT_ALPHA
			OUT.Albedo *= diff.a;
		#endif
	#else
		OUT.Albedo = baseColor.rgb;
		OUT.Alpha = 1.0;
	#endif
	
	//AMBIENT OCC
	#if defined(MARMO_VERTEX_OCCLUSION) || defined(MARMO_OCCLUSION)
		half4 occ = half4(1.0,1.0,1.0,1.0);
		#ifdef MARMO_OCCLUSION
			occ = tex2D(_OccTex, uv_occ);
		#endif
		#ifdef MARMO_VERTEX_OCCLUSION
			occ.rg *= IN.color.rg;
		#endif
		occ = lerp(half4(1.0,1.0,1.0,1.0),occ, _OccStrength);
		//TODO: occlude lightprobe SH by diffuse AO
		exposureIBL.xy *= occ.rg;
	#endif

	//SKIN
	#if defined(MARMO_SKIN_IBL) || defined(MARMO_SKIN_DIRECT)		
		//SUBDERMIS
		float4 subdermis = _SubdermisColor;
		#ifdef MARMO_SUBDERMIS_MAP
			subdermis *= tex2D(_SubdermisTex, uv_subdermis);
			subdermis.rgb *= _Subdermis;
			float skinMask = subdermis.a;
		#else
			#define skinMask 1.0
		#endif
		OUT.Subdermis = subdermis.rgb * skinMask;
	#endif
	
	//NORMALS	
	#ifdef MARMO_NORMALMAP
		float3 localN = UnpackNormal(tex2D(_BumpMap, uv_bump));
		
		float bias = _NormalSmoothing * 4.0;
		float4 uv_blurred = float4(uv_bump.x, uv_bump.y, bias, bias);
		float3 smoothN = UnpackNormal(tex2Dbias(_BumpMap, uv_blurred));
		
		#ifdef MARMO_DETAIL
			bias = _DetailSmoothing * 8.0;
			uv_blurred = float4(uv_detail.x, uv_detail.y, bias, bias);
			float3 detailN = UnpackNormal(tex2D(_DetailMap, uv_detail.xy));
			float3 smoothDetailN = UnpackNormal(tex2Dbias(_DetailMap, uv_blurred));
			
			float detailOcc = dot(detailN, smoothDetailN) * dot(detailN, smoothN);
			
			localN += detailN * _DetailWeight * skinMask;
			smoothN.xy += smoothDetailN.xy * _DetailWeight;
		#endif
	
		//localN and viewDir are in tangent-space
		localN = normalize(localN);
		smoothN = normalize(smoothN);
		
		float specOcc = dot(localN, smoothN);
		#ifdef MARMO_DETAIL
			specOcc *= detailOcc;
			specOcc *= specOcc;
			specOcc *= specOcc;
			exposureIBL.y *= specOcc;
		#endif
		
		OUT.Normal = localN;
		float3 worldN = WorldNormalVector(IN, localN);
		smoothN = WorldNormalVector(IN, smoothN);		
	#else
		float3 worldN =  IN.worldNormal;		
		#ifdef MARMO_HQ
			worldN = normalize(worldN);
		#endif
		#if !defined(MARMO_NORMALMAP) && defined(UNITY_PASS_PREPASSFINAL)
			float3 localN = float3(0.0,0.0,1.0);
			//localN and viewDir are in tangent-space
		#else
			float3 localN = worldN;
			//localN and viewDir are in world-space
		#endif
		float3 smoothN = localN;
	#endif
		
	float3 localE = IN.viewDir;
	#ifdef MARMO_HQ
		localE = normalize(localE);
	#endif

	//SPECULAR
	#if defined(MARMO_SPECULAR_DIRECT) && defined(MARMO_SPECULAR_ANISO)
		float theta = _AnisoDir*(3.1415962/180.0);
		OUT.anisoTangent = float3(cos(theta), sin(theta), 0.0);
		#ifdef MARMO_NORMALMAP
			OUT.anisoTangent = WorldNormalVector(IN, OUT.anisoTangent);
		#endif
	#endif
	#if defined(MARMO_SPECULAR_DIRECT) || defined(MARMO_SPECULAR_IBL)
		#ifdef MARMO_DIFFUSE_SPECULAR_COMBINED
			half4 spec = diffspec;
		#else
			half4 spec = tex2D(_SpecTex, uv_spec);
		#endif

		#ifdef MARMO_HQ
			half fresnel = splineFresnel(localN, localE, _SpecInt, _Fresnel);
		#else
			half fresnel = fastFresnel(localN, localE, _SpecInt, _Fresnel);		
		#endif
			
		//camera exposure is built into OUT.Specular
		spec.rgb *= _SpecColor.rgb;

		//filter the light that reaches diffuse reflection by specular intensity		
		#ifdef MARMO_SPECULAR_FILTER
			//Light reaching diffuse is filtered by 1-specColor*specIntensity
			half3 specFilter = half3(1.0,1.0,1.0) - spec.rgb * _SpecInt;
			
			//If the material exhibits strong fresnel, bias the filter some.
			specFilter += _Fresnel.xxx*0.5;
			
			//don't let it get t crazy, clamp 0-1 and apply
			OUT.Albedo *= saturate(specFilter);
		#endif
		
		spec.rgb *= fresnel;
		half glossLod = glossLOD(spec.a, _Shininess);
		#ifdef MARMO_SPECULAR_DIRECT
			OUT.SpecularRGB = spec.rgb;
			OUT.Specular = glossExponent(glossLod);
			//conserve energy by dividing out specular integral		
			OUT.SpecularRGB *= specEnergyScalar(OUT.Specular);
			OUT.Specular *= 0.00390625; // 1/256
		#endif
	#endif

	#if defined(MARMO_SKIN_IBL) || defined(MARMO_SKIN_DIRECT)		
		//TRANSLUCENCY
		float3 trans = _TranslucencyColor.rgb;
		#ifdef MARMO_TRANSLUCENCY_MAP
			trans *= tex2D(_TranslucencyMap, uv_trans).rgb;
		#endif
		trans *= _Translucency;
		OUT.Translucency = trans.rgb * skinMask;

		//PEACH-FUZZ
		float eyeDP = dot(localE, localN);
		OUT.Fuzz = _Fuzz * diffuseFresnel(eyeDP, _FuzzScatter) * skinMask;		
		#ifdef MARMO_SPECULAR_IBL
			OUT.Fuzz *= 1.0-saturate(OUT.Specular);
		#endif
	#endif
	
	//SPECULAR IBL
	#ifdef MARMO_SPECULAR_IBL
		float3 skyR = WorldReflectionVector(IN, localN);
		#ifdef MARMO_SKY_BLEND
			float3 skyR1 = skyRotate(_SkyMatrix1, skyR); //per-fragment matrix multiply, expensive			
		#endif
		skyR = skyRotate(_SkyMatrix,skyR); //per-fragment matrix multiply, expensive
		
		#ifdef MARMO_MIP_GLOSS
			half3 specIBL = glossCubeLookup(_SpecCubeIBL, skyR, glossLod);
		#else
			half3 specIBL =  specCubeLookup(_SpecCubeIBL, skyR)*spec.a;
		#endif
		
		#ifdef MARMO_SKY_BLEND
			#ifdef MARMO_MIP_GLOSS
				half3 specIBL1 = glossCubeLookup(_SpecCubeIBL, skyR1, glossLod);
			#else
				half3 specIBL1 =  specCubeLookup(_SpecCubeIBL, skyR1)*spec.a;
			#endif
			specIBL = lerp(specIBL1, specIBL, _BlendWeightIBL);
		#endif
		
		OUT.Emission += specIBL.rgb * spec.rgb * exposureIBL.y;
	#endif
	
	//DIFFUSE IBL
	#ifdef MARMO_DIFFUSE_IBL
		#ifdef MARMO_SKIN_IBL
			worldN = lerp(worldN, smoothN, 0.5 * skinMask);
		#endif
		float3 skyN = skyRotate(_SkyMatrix,worldN); //per-fragment matrix multiply, expensive
		skyN = normalize(skyN);
		#ifdef MARMO_SKIN_IBL						
			//SH DIFFUSE			
			float3 band0, band1, band2;
			float3 unity0, unity1, unity2;
			SHLookup(skyN,band0,band1,band2);
			#ifdef MARMO_SKY_BLEND
				float3 skyN1 = skyRotate(_SkyMatrix1, worldN); //per-fragment matrix multiply, expensive
				skyN1 = normalize(skyN1);
				float3 band01, band11, band21;
				SHLookup1(skyN1,band01,band11,band21);
				band0 = lerp(band01, band0, _BlendWeightIBL);
				band1 = lerp(band11, band1, _BlendWeightIBL);
				band2 = lerp(band21, band2, _BlendWeightIBL);
			#endif
			
			SHLookupUnity(worldN,unity0,unity1,unity2);
			band0 = (band0*exposureIBL.x) + unity0;
			band1 = (band1*exposureIBL.x) + unity1;
			band2 = (band2*exposureIBL.x) + unity2;
			half3 diffIBL = SHConvolve(band0, band1, band2, subdermis.rgb*skinMask);
			diffIBL *= OUT.Albedo;
			
			//PEACH FUZZ
			diffIBL += OUT.Fuzz * _FuzzColor.rgb * (band0 + band1 + band2);

			#ifdef MARMO_TRANSLUCENCY_IBL
				//TRANSLUCENCY
				float3 diffSH = SHLookup(-smoothN);
				#ifdef MARMO_SKY_BLEND
					float3 diffSH1 = SHLookup1(-skyN1);
					diffSH = lerp(diffSH1, diffSH, _BlendWeightIBL);
				#endif
				diffSH *= exposureIBL.x;
				diffSH += SHLookupUnity(-worldN);
				diffIBL += diffSH * OUT.Translucency * _TranslucencySky;
			#endif
			
			OUT.Emission += diffIBL;
		#endif
	#endif

	
	//GLOW
	#ifdef MARMO_GLOW
		half4 glow = tex2D(_Illum, uv_glow);
		glow.rgb *= _GlowColor.rgb;
		glow.rgb *= _GlowStre4ngth;
		glow.a *= _EmissionLM;
		//NOTE: camera exposure is already in albedo from above
		glow.rgb += OUT.Albedo * glow.a;
		OUT.Emission += glow.rgb;
	#endif
	
	#ifndef MARMO_ALPHA
		OUT.Alpha = 1.0;
	#endif

	//camera exposure is built into OUT.Albedo & SpecularRGB
	OUT.Albedo *= exposureIBL.w;
	OUT.Emission *= exposureIBL.w;
	#ifdef MARMO_SPECULAR_DIRECT
		OUT.SpecularRGB *= exposureIBL.w;
	#endif
}

#endif

```

## Source file: Assets/Marmoset/Shader/Beta/Skin/MarmosetSkinInput.cginc

{
  "path": "Assets/Marmoset/Shader/Beta/Skin/MarmosetSkinInput.cginc",
  "blob_sha": "c7f1bdd13e7a44b87ea5918cebf5846b301e3a8f",
  "sha256": "ab77d52b5c8fee7f5399ec887bfbdb83fcdc14e9b7159fe69508f08f502fea7d",
  "lines": 121,
  "bytes": 2831,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 5,
    "rig": 1
  },
  "url": "https://github.com/liuxq/SkinShader/blob/b3b567e413db1b933df1faca250834444a3a7db1/Assets/Marmoset/Shader/Beta/Skin/MarmosetSkinInput.cginc",
  "function_signature_candidates": []
}

```text

// Marmoset Skyshop
// Copyright 2013 Marmoset LLC
// http://marmoset.co

#ifndef MARMOSET_SKIN_INPUT_CGINC
#define MARMOSET_SKIN_INPUT_CGINC

uniform sampler2D 	_MainTex;
uniform float4		_MainTex_ST;

#if defined(MARMO_OCCLUSION) || defined(MARMO_VERTEX_OCCLUSION)
uniform half		_OccStrength;
#endif

#ifdef MARMO_OCCLUSION
uniform sampler2D	_OccTex;
uniform float4		_OccTex_ST;
#endif

#if defined(MARMO_DIFFUSE_DIRECT) || defined(MARMO_DIFFUSE_IBL)
uniform float4		_Color;
#endif

#if defined(MARMO_SPECULAR_DIRECT) || defined(MARMO_SPECULAR_IBL)
	#ifndef MARMO_DIFFUSE_SPECULAR_COMBINED
	uniform sampler2D	_SpecTex;
	uniform float4		_SpecTex_ST;
	#endif
	//uniform float4	_SpecColor; //defined by unity
	uniform float		_SpecInt;
	uniform float		_Shininess;
	uniform float		_Fresnel;
#endif

#ifdef MARMO_NORMALMAP
uniform sampler2D 	_BumpMap;
uniform float4		_BumpMap_ST;
#endif

#ifdef MARMO_GLOW
uniform sampler2D	_Illum;
uniform float4		_GlowColor;
uniform float		_GlowStrength;
uniform float		_EmissionLM;
#endif

uniform float  _NormalSmoothing;

#ifdef MARMO_DETAIL
uniform float  		_DetailWeight;
uniform float		_DetailSmoothing;
uniform sampler2D	_DetailMap;
uniform float4		_DetailMap_ST;
#endif

uniform float	_Subdermis;
uniform float4	_SubdermisColor;
uniform float	_ConserveEnergy;

#ifdef MARMO_SUBDERMIS_MAP
uniform sampler2D _SubdermisTex;
uniform float4 	  _SubdermisTex_ST;
#endif

#ifdef MARMO_SPECULAR_ANISO
uniform float _Aniso;
uniform float _AnisoDir;
#endif

uniform float _Translucency;
uniform float _TranslucencySky;
uniform float4 _TranslucencyColor;
#ifdef MARMO_TRANSLUCENCY_MAP
uniform sampler2D _TranslucencyMap;
uniform float4	  _TranslucencyMap_ST;
#endif

uniform float   _Fuzz;	
uniform float4	_FuzzColor;
uniform float 	_FuzzScatter;
uniform float 	_FuzzOcc;
struct Input {
	float2 texcoord;

	#ifdef MARMO_OCCLUSION
		float2 texcoord1;
	#endif
	
	float3 worldNormal; //internal, required for the WorldNormalVector macro
	
	#if defined(MARMO_SPECULAR_DIRECT) || defined(MARMO_SPECULAR_IBL) || defined(MARMO_SKIN_IBL) || defined(MARMO_SKIN_DIRECT)
		float3 viewDir;
	#endif
	#ifdef MARMO_SPECULAR_IBL
		float3 worldRefl; //internal, required for the WorldReflVector macro
	#endif
	#if defined(MARMO_VERTEX_COLOR) || defined(MARMO_VERTEX_OCCLUSION)
		half4 color : COLOR;
	#endif
	INTERNAL_DATA
};

struct MarmosetSkinOutput {
	half3 Albedo;	//diffuse map RGB
	half Alpha;		//diffuse map A
	half3 Normal;	//world-space normal
	half3 Emission;	//contains IBL contribution
	half Specular;	//specular exponent (required by Unity)
	#ifdef MARMO_SPECULAR_DIRECT
		half3 SpecularRGB;	//specular mask
	#endif
	#if defined(MARMO_SKIN_DIRECT) || defined(MARMO_SKIN_IBL)
		half3 Subdermis;
		half3 Translucency;
		half  Fuzz;
	#endif
	#ifdef MARMO_SPECULAR_ANISO
		half3 anisoTangent;
	#endif
};
#endif

```

## Source file: Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader

{
  "path": "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin IBL.shader",
  "blob_sha": "0fc728f529f40bab85e44da153172ce3f210905c",
  "sha256": "837af931080dc4e83a2fdafb4872877ef2defd73fdd6d8c19b2c4ab3df925d49",
  "lines": 169,
  "bytes": 4502,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 12,
    "shader": 5,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/liuxq/SkinShader/blob/b3b567e413db1b933df1faca250834444a3a7db1/Assets/Marmoset/Shader/Beta/Skin/Marmoset%20Skin%20IBL.shader",
  "function_signature_candidates": []
}

```text

// Marmoset Skyshop
// Copyright 2013 Marmoset LLC
// http://marmoset.co

Shader "Marmoset/Beta/Skin IBL" {
	Properties {
		_Color   ("Diffuse Color", Color) = (1,1,1,1)
		_SpecColor ("Specular Color", Color) = (1,1,1,1)
		_SpecInt ("Specular Intensity", Float) = 1.0
		_Shininess ("Specular Sharpness", Range(2.0,8.0)) = 4.0
		_Fresnel ("Specular Fresnel", Range(0.0,1.0)) = 0.0		
		
		_MainTex ("Diffuse(RGB) Alpha(A)", 2D) = "white" {}
		_SpecTex ("Specular(RGB) Gloss(A)", 2D) = "white" {}
		_BumpMap ("Normal Map", 2D) 	= "bump" {}
		_NormalSmoothing ("Normal Smoothing", Range(0.0,1.0)) = 1.0
		
		_SubdermisColor ("Subdermis Color", Color) = (1,1,1,1)
		_Subdermis ("Subdermis", Range(0.0,1.0)) = 1.0
		_ConserveEnergy ("Conserve Energy", Range(0.0,1.0)) = 1.0
		_SubdermisTex("Subdermis(RGB) Skin Mask(A)", 2D) = "white" {}
		
		_TranslucencyColor ("Translucency Color", Color) = (1,0.5,0.4,1)
		_Translucency ("Translucency", Range(0.0,1.0)) = 0.0
		_TranslucencySky ("Sky Translucency", Range(0.0,1.0)) = 0.0
		_TranslucencyMap ("Translucency Map", 2D) = "white" {}
		
		_FuzzColor	("Fuzz Color", Color) = (1,1,1,1)
		_Fuzz		("Fuzz", Range(0.0,1.0)) = 0.0
		_FuzzScatter ("Fuzz Scatter", Range(0.0,1.0)) = 1.0
		_FuzzOcc 	("Fuzz Occlusion", Range(0.5,1.0)) = 0.5
		//slots for custom lighting cubemaps
		
	}
	
	SubShader {
		Tags {
			"Queue"="Geometry"
			"RenderType"="Opaque"
		}
		LOD 400
		//diffuse LOD 200
		//diffuse-spec LOD 250
		//bumped-diffuse, spec 350
		//bumped-spec 400
		
		//mac stuff
		CGPROGRAM
		#ifdef SHADER_API_OPENGL
			#pragma glsl
		#endif
				
		#pragma target 3.0
		#pragma exclude_renderers flash
		#pragma surface MarmosetSkinSurf MarmosetSkinDirect fullforwardshadows vertex:MarmosetSkinVert noambient nolightmap nodirlightmap 
				
		#pragma multi_compile MARMO_BOX_PROJECTION_OFF MARMO_BOX_PROJECTION_ON
		#if MARMO_BOX_PROJECTION_ON	
			#define MARMO_BOX_PROJECTION
		#endif
		
		#pragma multi_compile MARMO_SKY_BLEND_OFF MARMO_SKY_BLEND_ON
		
		#if MARMO_SKY_BLEND_ON
			#define MARMO_SKY_BLEND
		#endif
		
		#define MARMO_HQ
		#define MARMO_SKY_ROTATION
		#define MARMO_DIFFUSE_IBL
		#define MARMO_SPECULAR_IBL
		#define MARMO_DIFFUSE_DIRECT
		#define MARMO_SPECULAR_DIRECT		
		#define MARMO_NORMALMAP
		#define MARMO_MIP_GLOSS
		
		#define MARMO_SUBDERMIS_MAP
		#define MARMO_TRANSLUCENCY_MAP
		//#define MARMO_SPECULAR_ANISO
		#define MARMO_TRANSLUCENCY_DIRECT
		#define MARMO_TRANSLUCENCY_IBL
		
		#define MARMO_SKIN_IBL
		#define MARMO_SKIN_DIRECT
		//#define MARMO_SPECULAR_FILTER
		//#define MARMO_DIFFUSE_SPECULAR_COMBINED
				
		//#define MARMO_ALPHA
		//#define MARMO_GLOW
		//#define MARMO_PREMULT_ALPHA
		//#define MARMO_OCCLUSION
		//#define MARMO_VERTEX_OCCLUSION
		//#define MARMO_VERTEX_COLOR		
		
		#include "../../MarmosetCore.cginc"
		#include "MarmosetSkinInput.cginc"
		#include "MarmosetSkin.cginc"

		ENDCG
	}
	
	SubShader {
		Tags {
			"Queue"="Geometry"
			"RenderType"="Opaque"
		}
		LOD 400
		//diffuse LOD 200
		//diffuse-spec LOD 250
		//bumped-diffuse, spec 350
		//bumped-spec 400
		
		//mac stuff
		CGPROGRAM
		#ifdef SHADER_API_OPENGL
			#pragma glsl
		#endif
				
		#pragma target 3.0
		#pragma only_renderers gles gles3 metal d3d11_9x
		#pragma surface MarmosetSkinSurf MarmosetSkinDirect fullforwardshadows vertex:MarmosetSkinVert noambient nolightmap nodirlightmap 
				
		#pragma multi_compile MARMO_BOX_PROJECTION_OFF MARMO_BOX_PROJECTION_ON
		#if MARMO_BOX_PROJECTION_ON	
			#define MARMO_BOX_PROJECTION
		#endif
		
		#pragma multi_compile MARMO_SKY_BLEND_OFF MARMO_SKY_BLEND_ON
		
		#if MARMO_SKY_BLEND_ON
			#define MARMO_SKY_BLEND
		#endif
		
		#define MARMO_HQ
		#define MARMO_SKY_ROTATION
		#define MARMO_DIFFUSE_IBL
		#define MARMO_SPECULAR_IBL
		#define MARMO_DIFFUSE_DIRECT
		#define MARMO_SPECULAR_DIRECT		
		#define MARMO_NORMALMAP
		//#define MARMO_MIP_GLOSS
		
		#define MARMO_SUBDERMIS_MAP
		#define MARMO_TRANSLUCENCY_MAP
		//#define MARMO_SPECULAR_ANISO
		#define MARMO_TRANSLUCENCY_DIRECT
		#define MARMO_TRANSLUCENCY_IBL
		
		#define MARMO_SKIN_IBL
		#define MARMO_SKIN_DIRECT
		//#define MARMO_SPECULAR_FILTER
		//#define MARMO_DIFFUSE_SPECULAR_COMBINED
		
		//#define MARMO_ALPHA
		//#define MARMO_GLOW
		//#define MARMO_PREMULT_ALPHA
		//#define MARMO_OCCLUSION
		//#define MARMO_VERTEX_OCCLUSION
		//#define MARMO_VERTEX_COLOR
			
		#include "../../MarmosetCore.cginc"
		#include "MarmosetSkinInput.cginc"
		#include "MarmosetSkin.cginc"

		ENDCG
	}

	FallBack "Marmoset/Bumped Specular IBL"
}


```

## Source file: Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader

{
  "path": "Assets/Marmoset/Shader/Beta/Skin/Marmoset Skin Fast IBL.shader",
  "blob_sha": "326540770089badc3488fabfe247139dcfecdfe6",
  "sha256": "bd200de0c5ca822f5b93aff31a0590f25b89eab37c5216fc312109756f546544",
  "lines": 161,
  "bytes": 4258,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 12,
    "shader": 5,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/liuxq/SkinShader/blob/b3b567e413db1b933df1faca250834444a3a7db1/Assets/Marmoset/Shader/Beta/Skin/Marmoset%20Skin%20Fast%20IBL.shader",
  "function_signature_candidates": []
}

```text

// Marmoset Skyshop
// Copyright 2014 Marmoset LLC
// http://marmoset.co

Shader "Marmoset/Beta/Skin Fast IBL" {
	Properties {
		_Color   ("Diffuse Color", Color) = (1,1,1,1)
		_SpecColor ("Specular Color", Color) = (1,1,1,1)
		_SpecInt ("Specular Intensity", Float) = 1.0
		_Shininess ("Specular Sharpness", Range(2.0,8.0)) = 4.0
		_Fresnel ("Specular Fresnel", Range(0.0,1.0)) = 0.0		
		_MainTex ("Diffuse(RGB) Gloss(A)", 2D) = "white" {}
		//_SpecTex ("Specular(RGB) Gloss(A)", 2D) = "white" {}
		_BumpMap ("Normalmap", 2D) 	= "bump" {}
		
		_SubdermisColor ("Subdermis Color", Color) = (1,1,1,1)
		_ConserveEnergy ("Conserve Energy", Range(0.0,1.0)) = 1.0
		//_Subdermis ("Subdermis", Range(0.0,1.0)) = 1.0
		//_SubdermisTex("Subdermis(RGB) Skin Mask(A)", 2D) = "white" {}

		_TranslucencyColor ("Translucency Color", Color) = (1,0.5,0.4,1)
		_Translucency ("Translucency", Range(0.0,1.0)) = 1.0
		//_TranslucencySky ("Sky Translucency", Range(0.0,1.0)) = 0.0
		//_TranslucencyMap ("Translucency Map", 2D) = "white" {}

		_FuzzColor	("Fuzz Color", Color) = (1,1,1,1)
		_Fuzz		("Fuzz", Range(0.0,1.0)) = 1.0
		_FuzzScatter ("Fuzz Scatter", Range(0.0,1.0)) = 1.0
		_FuzzOcc 	("Fuzz Occlusion", Range(0.5,1.0)) = 0.5
		
		//_Aniso		("Anisotropy", Range(0.0,1.0)) = 0.0
		//_AnisoDir	("Anisotropy Direction", Range(0.0,180.0)) = 0.0		
		//_Cutoff		("Alpha Cutoff", Range(0.0,1.0)) = 0.3
		
		//slots for custom lighting cubemaps
		
	}
	
	SubShader {
		Tags {
			"Queue"="Geometry"
			"RenderType"="Opaque"
		}
		LOD 350
		//diffuse LOD 200
		//diffuse-spec LOD 250
		//bumped-diffuse, spec 350
		//bumped-spec 400
		
		//mac stuff
		CGPROGRAM
		#ifdef SHADER_API_OPENGL
			#pragma glsl
		#endif
				
		#pragma target 3.0
		#pragma exclude_renderers flash
		#pragma surface MarmosetSkinSurf MarmosetSkinDirect vertex:MarmosetSkinVert noambient noforwardadd
		
		#pragma multi_compile MARMO_SKY_BLEND_OFF MARMO_SKY_BLEND_ON
		
		#if MARMO_SKY_BLEND_ON
			#define MARMO_SKY_BLEND
		#endif
		
		#define MARMO_HQ
		#define MARMO_SKY_ROTATION
		#define MARMO_DIFFUSE_IBL
		//#define MARMO_SPECULAR_IBL
		#define MARMO_DIFFUSE_DIRECT
		#define MARMO_SPECULAR_DIRECT
		#define MARMO_NORMALMAP
		//#define MARMO_MIP_GLOSS
		
		//#define MARMO_SUBDERMIS_MAP
		//#define MARMO_TRANSLUCENCY_MAP
		//#define MARMO_SPECULAR_ANISO
		#define MARMO_TRANSLUCENCY_DIRECT
		//#define MARMO_TRANSLUCENCY_IBL
		
		#define MARMO_SKIN_IBL
		#define MARMO_SKIN_DIRECT
		//#define MARMO_SPECULAR_FILTER
		#define MARMO_DIFFUSE_SPECULAR_COMBINED
		
		//#define MARMO_ALPHA
		//#define MARMO_GLOW
		//#define MARMO_PREMULT_ALPHA
		//#define MARMO_OCCLUSION
		//#define MARMO_VERTEX_OCCLUSION
		//#define MARMO_VERTEX_COLOR		

		#include "../../MarmosetCore.cginc"
		#include "MarmosetSkinInput.cginc"
		#include "MarmosetSkin.cginc"

		ENDCG
	}
	SubShader {
		Tags {
			"Queue"="Geometry"
			"RenderType"="Opaque"
		}
		LOD 350
		//diffuse LOD 200
		//diffuse-spec LOD 250
		//bumped-diffuse, spec 350
		//bumped-spec 400
		
		//mac stuff
		CGPROGRAM
		#ifdef SHADER_API_OPENGL
			#pragma glsl
		#endif
				
		#pragma target 3.0
		#pragma only_renderers gles gles3 metal d3d11_9x
		#pragma surface MarmosetSkinSurf MarmosetSkinDirect vertex:MarmosetSkinVert noambient noforwardadd
		
		#pragma multi_compile MARMO_SKY_BLEND_OFF MARMO_SKY_BLEND_ON
		
		#if MARMO_SKY_BLEND_ON
			#define MARMO_SKY_BLEND
		#endif
		
		#define MARMO_HQ
		#define MARMO_SKY_ROTATION
		#define MARMO_DIFFUSE_IBL
		//#define MARMO_SPECULAR_IBL
		#define MARMO_DIFFUSE_DIRECT
		#define MARMO_SPECULAR_DIRECT
		#define MARMO_NORMALMAP
		//#define MARMO_MIP_GLOSS
		
		//#define MARMO_SUBDERMIS_MAP
		//#define MARMO_TRANSLUCENCY_MAP
		//#define MARMO_SPECULAR_ANISO
		#define MARMO_TRANSLUCENCY_DIRECT
		//#define MARMO_TRANSLUCENCY_IBL
		
		#define MARMO_SKIN_IBL
		#define MARMO_SKIN_DIRECT
		//#define MARMO_SPECULAR_FILTER
		#define MARMO_DIFFUSE_SPECULAR_COMBINED
		
		//#define MARMO_ALPHA
		//#define MARMO_GLOW
		//#define MARMO_PREMULT_ALPHA
		//#define MARMO_OCCLUSION
		//#define MARMO_VERTEX_OCCLUSION
		//#define MARMO_VERTEX_COLOR		

		#include "../../MarmosetCore.cginc"
		#include "MarmosetSkinInput.cginc"
		#include "MarmosetSkin.cginc"

		ENDCG
	}
	
	FallBack "Marmoset/Mobile/Diffuse IBL"
}


```

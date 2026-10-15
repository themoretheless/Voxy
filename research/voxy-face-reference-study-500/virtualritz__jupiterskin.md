# Facial rendering source scan: virtualritz/jupiterskin

Repository: https://github.com/virtualritz/jupiterskin

Pinned source revision: acff3ff84a24fc228d7f590afeea24155236ea5a

Discovery topic: skin shader

Description: /*jupiter jazz*/ RSL Skin Shader

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["code/envlight2_jjskin.sl", "code/jjskin.sl", "code/jjskin_3dfm_pragmas.h", "code/jjskin_aovs.h", "code/jjskin_utils.h", "manual/JupiterSGISkin.pdf"], "sss": ["media/sss_varying_albedo.png"], "deform": ["scenefiles/sequence_deformation_needsPTC_maya_ASCII.ma.7z"]}

## Source file: code/jjskin_aovs.h

{
  "path": "code/jjskin_aovs.h",
  "blob_sha": "af72303efce572c0a88108792f554ef2fcbd3e1f",
  "sha256": "3e653b17eb18fd54fd2d7dd623abb0c866fd1fd19534d032e17b12d03e54acbe",
  "lines": 149,
  "bytes": 5877,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 4,
    "subsurface": 4,
    "sss": 8,
    "shader": 1,
    "rig": 1,
    "face": 8
  },
  "url": "https://github.com/virtualritz/jupiterskin/blob/acff3ff84a24fc228d7f590afeea24155236ea5a/code/jjskin_aovs.h",
  "function_signature_candidates": []
}

```text

/******************************************************************************/
/* Copyright (c) 2009-2010 by Jupiter Jazz Ltd. All Rights Reserved.          */
/*                                                                            */
/******************************************************************************/

////////////////////////////////////////////////////////////////////////////////
// Set of useful AOVs for JupiterJazz JJskin shader. These although functional
// are meant as an suggestion, or departure point to further tweak this into
// your own workflow.

#ifndef JJSKIN_AOV_H
#define JJSKIN_AOV_H

#define DECLARE_AOV_PARAMETERS		\
		aov_ambient_color,			\
		aov_diffuse_color,			\
		aov_specular_color,			\
		aov_reflection_color,		\
		aov_ambient,				\
		aov_diffuse,				\
		aov_specular,				\
		aov_specularlobe1,			\
		aov_specularlobe2,			\
		aov_lobebalance,			\
		aov_reflection,				\
		aov_reflection_raytraced,	\
		aov_reflection_environment,	\
		aov_reflection_fresnel,		\
		aov_indirectdiffuse,		\
		aov_subsurfacescattering,	\
		aov_sssmixfactor,			\
		aov_diffusesssblend,		\
		aov_shadowing,				\
		aov_occlusion,				\
		aov_transparency,			\
		aov_blendfactor,			\
		aov_surfacenormal,			\
		aov_Rmap_normal,			\
		aov_Gmap_normal,			\
		aov_Bmap_normal,			\
		aov_Smap_normal,			\
		aov_Oi,						\
		aov_Ci

#define INIT_AOV_PARAMETERS \
		aov_ambient_color				= color(0);		\
		aov_diffuse_color				= color(0);		\
		aov_specular_color				= color(0);		\
		aov_reflection_color			= color(0);		\
		aov_ambient						= color(0);		\
		aov_diffuse						= color(0);		\
		aov_specular					= color(0);		\
		aov_specularlobe1				= 0;			\
		aov_specularlobe2				= 0;			\
		aov_lobebalance					= 0;			\
		aov_reflection					= color(0);		\
		aov_reflection_raytraced		= color(0);		\
		aov_reflection_environment		= color(0);		\
		aov_reflection_fresnel			= 0;			\
		aov_indirectdiffuse				= color(0);		\
		aov_subsurfacescattering		= color(0);		\
		aov_sssmixfactor				= 0;			\
		aov_diffusesssblend				= color(0);		\
		aov_shadowing					= 0;			\
		aov_occlusion					= 0;			\
		aov_transparency				= color(0);		\
		aov_blendfactor					= 0;			\
		aov_surfacenormal				= normal(0);	\
		aov_Rmap_normal					= normal(0);	\
		aov_Gmap_normal					= normal(0);	\
		aov_Bmap_normal					= normal(0);	\
		aov_Smap_normal					= normal(0);	\
		aov_Oi							= color(0);		\
		aov_Ci							= color(0);

#define DECLARE_AOV_OUTPUT_PARAMETERS					\
		output varying color aov_ambient_color;			\
		output varying color aov_diffuse_color;			\
		output varying color aov_specular_color;		\
		output varying color aov_reflection_color;		\
		output varying color aov_ambient;				\
		output varying color aov_diffuse;				\
		output varying color aov_specular;				\
		output varying float aov_specularlobe1;			\
		output varying float aov_specularlobe2;			\
		output varying float aov_lobebalance;			\
		output varying color aov_reflection;			\
		output varying color aov_reflection_raytraced;	\
		output varying color aov_reflection_environment;\
		output varying float aov_reflection_fresnel;	\
		output varying color aov_indirectdiffuse;		\
		output varying color aov_subsurfacescattering;	\
		output varying float aov_sssmixfactor;			\
		output varying color aov_diffusesssblend;		\
		output varying float aov_shadowing;				\
		output varying float aov_occlusion;				\
		output varying color aov_transparency;			\
		output varying float aov_blendfactor;			\
		output varying normal aov_surfacenormal;		\
		output varying normal aov_Rmap_normal;			\
		output varying normal aov_Gmap_normal;			\
		output varying normal aov_Bmap_normal;			\
		output varying normal aov_Smap_normal;			\
		output varying color aov_Oi;					\
		output varying color aov_Ci;					

#define DEFAULT_AOV_OUTPUT_PARAMETERS								\
		output varying color aov_ambient_color			= color(0); \
		output varying color aov_diffuse_color			= color(0); \
		output varying color aov_specular_color			= color(0); \
		output varying color aov_reflection_color		= color(0); \
		output varying color aov_ambient				= color(0); \
		output varying color aov_diffuse				= color(0); \
		output varying color aov_specular				= color(0); \
		output varying float aov_specularlobe1			= 0;		\
		output varying float aov_specularlobe2			= 0;		\
		output varying float aov_lobebalance			= 0;		\
		output varying color aov_reflection				= color(0); \
		output varying color aov_reflection_raytraced	= color(0); \
		output varying color aov_reflection_environment	= color(0); \
		output varying float aov_reflection_fresnel		= 0;		\
		output varying color aov_indirectdiffuse		= color(0); \
		output varying color aov_subsurfacescattering	= color(0); \
		output varying float aov_sssmixfactor			= 0;		\
		output varying color aov_diffusesssblend		= color(0); \
		output varying float aov_shadowing				= 0;		\
		output varying float aov_occlusion				= 0;		\
		output varying color aov_transparency			= color(0);	\
		output varying float aov_blendfactor			= 0;		\
		output varying normal aov_surfacenormal 		= normal(0);\
		output varying normal aov_Rmap_normal			= normal(0);\
		output varying normal aov_Gmap_normal			= normal(0);\
		output varying normal aov_Bmap_normal			= normal(0);\
		output varying normal aov_Smap_normal			= normal(0);\
		output varying color aov_Oi						= color(0);	\
		output varying color aov_Ci						= color(0);

// Light sources need __3dfm_shadowing, main body sets symbol to retrieve
// this value from shadow's AOV respectively.
#define GETSHADOWS( CATEGORY, PP)									\
	varying float inshadow = 0;										\
		illuminance( CATEGORY, PP ) {								\
		if (1 == lightsource("__3dfm_shadowing", inshadow) ) {		\
			aov_shadowing += (1 - inshadow);						\
			}														\
		}
////////////////////////////////////////////////////////////////////////////////
#endif // JJSKIN_AOV_H



```

## Source file: code/jjskin_utils.h

{
  "path": "code/jjskin_utils.h",
  "blob_sha": "fdedfb85c4be5c3a617b6c3ced083a3f015050e1",
  "sha256": "b25b265328dfc4790f13d68af901522ebbfb0117b8576424c83212499519e173",
  "lines": 339,
  "bytes": 11829,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 23,
    "subsurface": 2,
    "deform": 2,
    "shader": 2,
    "rig": 1,
    "face": 13
  },
  "url": "https://github.com/virtualritz/jupiterskin/blob/acff3ff84a24fc228d7f590afeea24155236ea5a/code/jjskin_utils.h",
  "function_signature_candidates": [
    {
      "line": 90,
      "text": "float gamma( float x, gamma; ) { return pow( x, 1/gamma); }"
    },
    {
      "line": 93,
      "text": "return color(\tfloat gamma( x[0], gamma),"
    },
    {
      "line": 94,
      "text": "float gamma( x[1], gamma),"
    },
    {
      "line": 95,
      "text": "float gamma( x[2], gamma) );"
    },
    {
      "line": 98,
      "text": "float remap( float value, romin, romax, rnmin, rnmax; )"
    },
    {
      "line": 149,
      "text": "float schlickfresnel(\tfloat costheta, ior; )"
    },
    {
      "line": 167,
      "text": "float beckmann(\tfloat cosalpha, roughness; )"
    },
    {
      "line": 177,
      "text": "float geomasking( float cos_theta_r, cos_alpha, cos_theta_i, cos_beta; )"
    }
  ]
}

```text

/******************************************************************************/
/* Copyright (c) 2009-2010 by Jupiter Jazz Ltd. All Rights Reserved.          */
/*                                                                            */
/******************************************************************************/

////////////////////////////////////////////////////////////////////////////////
/* Jupiter Jazz JJskin
 * author: Luis Barrancos (luis.b.barrancos_AT_gmail.com)
 *
 * set of utilities and useful constants for main JJskin
 * */

////////////////////////////////////////////////////////////////////////////////
// 3Delight for Maya's shading_utils.h, for BEGIN/END_PHOTON_CASE macros,
// and ShadingNormal(). Need to set AOVs accordingly later.
// ( include/maya/rsl )
#ifndef JJSKIN_UTILS_H
#define JJSKIN_UTILS_H	1

#include "shading_utils.h"

////////////////////////////////////////////////////////////////////////////////
// Normal maps lookup parameters
#define NORMAL_TEXLOOKUP_PARAMETERS \
		s, t, s, t, s, t, s, t, "filter", "box", "samples", 1

// texture lookup parameters (easier to set for all the lookups in main)
#define TEXLOOKUP_PARAMETERS \
		s, t, "filter", "gaussian", "blur", 0

// ptc bake macro
#define DO_BAKE_BEGIN( DOBAKE, BAKEFILE, DIFFCOLOR, NS ) \
		if (DOBAKE == 1 && BAKEFILE != "") { \
			aov_ambient += Ka * ambient(); \
			aov_diffuse += Kd * diffuse(NS); \
			aov_Ci = DIFFCOLOR * (aov_ambient + aov_diffuse) + \
				subsurface(P); \
			float A = area(P); \
			bake3d( bakefile, "", P, NS, "_radiance_t", aov_Ci, \
					"_area", A, "interpolate", 1 ); \
		} else {

// :ahem:
#define DO_BAKE_END }

// need a minimalistic version of 3dfm's photon macro
// ideally one would use AOVs here
#define BEGIN_JJSKIN_PHOTON_CASE( DIFFUSECOLOR, TRANSPARENCY ) \
	uniform string raytype = ""; \
	if (1 == rayinfo("type", raytype) && raytype == "light") { \
		aov_Ci += DIFFUSECOLOR; \
		aov_Oi += TRANSPARENCY; \
	} else {

#define END_JJSKIN_PHOTON_CASE	} // close previous brace

// useful quantities (just in case, check if definitions exist)
#ifndef PI_OVER_2
#define PI_OVER_2	1.570796326794896
#endif

#ifndef _2PI
#define _2PI		6.283185307179586
#endif

#ifndef _4PI
#define _4PI		12.56637061435917
#endif

#ifndef EPS
#define EPS			0.00001
#endif

#ifndef SQR
#define SQR(X)		( (X) * (X) )
#endif

// Skin data from "A Practical Model for Subsurface Light Transport", by
// Henrik Wann Jensen, Steve Marschner, Marc Levoy, Pat Hanrahan
// mean scattering depth = 1 / sqrt(3*(sigma_s+sigma_a)*sigma_a)
// sigma_s and a, scattering and absorption coefficients respectively
#define MND_SKIN1	color( 3.673294, 1.366533, 0.682693 )
#define ALB_SKIN1	color( 0.440000, 0.220000, 0.130000 )

#define MND_SKIN2	color( 4.821475, 1.693699, 1.089970 )
#define ALB_SKIN2	color( 0.630000, 0.440000, 0.340000 )

////////////////////////////////////////////////////////////////////////////////
// Gamma controls (bias and gain might be added later)
float gamma( float x, gamma; ) { return pow( x, 1/gamma); }

color gamma( color x; float gamma; ) {
	return color(	float gamma( x[0], gamma),
					float gamma( x[1], gamma),
					float gamma( x[2], gamma) );
}

float remap( float value, romin, romax, rnmin, rnmax; )
{
	float scale = (value - romin) / (romax-romin);
	float out = (rnmin < rnmax) ?
		rnmin + scale * (rnmax - rnmin) : rnmin - scale * (rnmin - rnmax);
	return out;
}

////////////////////////////////////////////////////////////////////////////////
// transform scanned normals
normal getnormal(
					normal Nscanned, Nref, Ns;
					point Pref, PP;
					float ss, tt;
					)
{
	// build TBN matrix of reference mesh
	normal Nsref = normalize(Nref); // it wasn't normalized when passed
	// build tangent along S direction
	float det = Du(ss)*Dv(tt) - Dv(ss)*Du(tt);
	vector Tn = normalize( (Du(Pref)*Dv(tt) - Dv(Pref)*Du(tt)) / det );
	vector Bn = normalize( Tn^Nsref ); // bitangent
	Tn = normalize( Nsref^Bn ); // tangent
	// in renderman, vector premultiplication, build TBN matrix of ref mesh
	matrix TBNref = matrix(	Tn[0], Bn[0], Nsref[0], 0,
							Tn[1], Bn[1], Nsref[1], 0,
							Tn[2], Bn[2], Nsref[2], 0,
							0, 0, 0, 1 );
	// transform scanned normals into tangent space of reference mesh
	// use "shader" fromspace, Pref and Nref shouldn't be in world or current
	normal Ntangent = normalize( transform( "shader", TBNref, Nscanned ) );

	// build TBN matrix of deformed mesh
	// NS = ShadingNormal( normalize(N) );
	Tn = normalize( (Du(PP)*Dv(tt) - Dv(PP)*Du(tt)) / det );
	Bn = normalize( Tn^Ns );
	Tn = normalize( Ns^Bn );
	// transpose
	matrix TBN = matrix(	Tn[0], Tn[1], Tn[2], 0,
							Bn[0], Bn[1], Bn[2], 0,
							Ns[0], Ns[1], Ns[2], 0,
							0, 0, 0, 1 );
	// transform tangent space normals back via TBN matrix of deformed mesh
	normal Nnew = normalize( transform( "camera", TBN, Ntangent ) );

	return Nnew;
}

////////////////////////////////////////////////////////////////////////////////
// Christophe Schlick's fast Fresnel approximation, for rays entering
// denser medium, without total internal reflection.
float schlickfresnel(	float costheta, ior; )
{
	float kr = (ior - 1) / (ior + 1);
	kr *= kr;
	return kr + (1 - kr) * pow(1 - costheta, 5);
}

////////////////////////////////////////////////////////////////////////////////
// Beckmann microfacet distribution function
// For the microfacet distribution and geometric masking/shadowing terms, also
// see the approximation suggested by Csaba Kelemen, Laszlo Szirmay-Kalos,
// in "A microfacet based coupled specular-matte BRDF model with importance
// sampling", or the approximation for the first equation of the Smith
// shadowing/masking function for a known microfacet normal, in
// "Microfacet models for refraction through rough surfaces", by Bruce Walter,
// Stephen R.Marschner, Hongsong Li, Kenneth E.Torrance, Program of Computer
// Graphics, Cornell University, Beijing Institute of Technology, 3,6-7.

float beckmann(	float cosalpha, roughness; )
{
	// (1/ cos^2(alpha) m^2) * exp( -(tan^2(alpha)/m^2)
	float m2 = SQR(roughness);
	float c2 = SQR(cosalpha);
	return exp( (c2-1) / (c2*m2)) / ( SQR(c2)*m2);
}

////////////////////////////////////////////////////////////////////////////////
// Geometric masking/shadowing (Torrance-Sparrow). /////////////////////////////
float geomasking( float cos_theta_r, cos_alpha, cos_theta_i, cos_beta; )
{
	float shadowing = (cos_alpha * cos_theta_r) / cos_beta;
	float masking = (cos_alpha * cos_theta_i) / cos_beta;
	return min(1, 2 * min( shadowing, masking));
}

////////////////////////////////////////////////////////////////////////////////
// Diffuse and specular terms. /////////////////////////////////////////////////
// If the actual surface normal is visible to viewer but angle between
// scanned normal and real surface normal > 90', use surface normal instead.
// This helps reduce artifacts in areas with small details with random-ish
// orientation (eyelashes, beard...)

color
jjskin_diffuse(
				vector Ln;
				normal Ns, ScannedNormal;
				float ns_dot_v, ns_dot_map;
				uniform float nondiff;
		)
{
	float cos_theta_i = (ns_dot_v >= 0 && ns_dot_map <= 0) ?
		Ln.Ns : Ln.ScannedNormal;
	return ((1 - nondiff) * cos_theta_i);
}

////////////////////////////////////////////////////////////////////////////////
// Specular term. //////////////////////////////////////////////////////////////
// The captured specular roughness isn't directly tied to any particular
// analytical BRDF. In any case, the best results are obtained with spatially
// varying Torrance-Sparrow lobes (the paper mentions Beckmann distribution,
// see "Analysis of Human Faces using a Measurement-Based Skin Reflectance
// Model", by Tim Weyrich, Wojciech Matusik, Hanspeter Pfister, Bernd Bickel,
// Craig Donner, Chien Tu, Jinho Lee, Janet McAndless, Addy Ngan,Markus Gross,
// Henrik Wann Jensen.

void
jjskin_specular(
					vector Ln, Vf;
					normal Ns, ScannedNormal;
					float ns_dot_v, ns_dot_map;
					float Ks1, Ks2, r1, r2, r1sqr, r2sqr, ior;
					uniform float useTSlobes;
					output varying float lobe1, lobe2;
		)
{
	float cos_theta_i = 0, cos_theta_r = 0, cos_alpha = 0, cos_beta = 0;
	float alpha = 0, D = 0, G = 0, F = 0;
	vector Hn = normalize(Ln+Vf);
	// check orientation, use Ns if needed
	if (ns_dot_v >= 0 && ns_dot_map <= 0) {
		cos_theta_r = ns_dot_v;
		cos_theta_i = Ln.Ns;
		cos_alpha = Hn.Ns;
	} else {
		cos_theta_r = max( EPS, Vf.ScannedNormal );
		cos_theta_i = Ln.ScannedNormal;
		cos_alpha = Hn.ScannedNormal;
	}
	cos_beta = Ln.Hn; // = Vf.Hn

	if (useTSlobes == 0) { // use Beckmann distribution
		// 1st lobe
		D = beckmann( cos_alpha, r1 );
		G = geomasking( cos_theta_r, cos_alpha, cos_theta_i, cos_beta );
		F = schlickfresnel( cos_beta, ior );
		lobe1 = Ks1 * (D*G*F / cos_theta_r );
		// 2nd lobe
		D = beckmann( cos_alpha, r2 );
		lobe2 = Ks2 * (D*G*F / cos_theta_r );
	} else {
		// use a normal distribution. See also: "Surface Reflection: Physical
		// and Geometrical Perspectives", by Shree K.Nayaar, Katsushi Ikeuchi,
		// Takeo Kanade
		alpha = acos( cos_alpha );
		D = exp( -( SQR(alpha) / r1sqr ));
		G = geomasking( cos_theta_r, cos_alpha, cos_theta_i, cos_beta );
		F = schlickfresnel( cos_beta, ior );
		lobe1 = Ks1 * (D*G*F / cos_theta_r );
		D = exp( -( SQR(alpha) / r2sqr ));
		lobe2 = Ks2 * (D*G*F / cos_theta_r );
	}
}

////////////////////////////////////////////////////////////////////////////////
color
get_reflection(
				vector In;
				normal Ns;
				float samplecone;
				uniform float samples, envsamples, bias, maxdist;
				float envblur;
				uniform float width;
				uniform string label, subset, hitsides, envmap, envspace;
				output varying color reflection_raytraced;
				output varying color reflection_environment;
		)
{
	color hitc = color(0), cout = color(0);
	color crayhit = color(0), craymiss = color(0);
	vector R = vector(0);
	uniform float nhitr = 0, nhite = 0;
	extern point P;

	/* The normal used is the normal corresponding to the specular normal
	 * map. In any case, using too many samples, or high maximum ray distance
	 * might be a bit of an overkill, environment mapping would do just fine,
	 * and be believable enough. */
	uniform string raytype = "none";
	rayinfo("type", raytype);

	if (raytype == "camera" || raytype == "specular" ) {

		// cut down samples at higher ray depths
		uniform float raydepth = 0;
		rayinfo("depth", raydepth);
		uniform float rsamples = (raydepth > 1) ?
			max( 1, samples / pow( 2, raydepth)) : samples;

		float idotn = In.Ns; // In = normalize(I)
		if (idotn < 0) { // when visible

			if (envmap != "") { // use environment map on ray misses

				gather( "illuminance", P, reflect( In, Ns ), samplecone,
						rsamples, "distribution", "cosine",
						"surface:Ci", hitc, "ray:direction", R,
						"label", label, "subset", subset, "bias", bias,
						"raytype", "specular", "hitsides", hitsides,
						"maxdist", maxdist )
				{
					crayhit += hitc;
					nhitr += 1;
				} else {
					craymiss += environment( envmap, transform( envspace, R),
							"blur", envblur, "width", width,
							"filter", "gaussian", "samples", envsamples );
					nhite += 1;
				}
			} else { // no env lookup on ray misses
				gather( "illuminance", P, reflect( In, Ns ), samplecone,
						rsamples, "distribution", "cosine",
						"surface:Ci", hitc, "label", label,
						"subset", subset, "bias", bias,
						"raytype", "specular", "hitsides", hitsides,
						"maxdist", maxdist)
				{
					crayhit += hitc;
					nhitr += 1;
				}
			}
			reflection_raytraced = (crayhit / nhitr);
			reflection_environment = (craymiss / nhite);
			cout = reflection_raytraced + reflection_environment;
		}
	}
	return cout;
}

////////////////////////////////////////////////////////////////////////////////
#endif // JJSKIN_UTILS_H



```

## Source file: code/jjskin_3dfm_pragmas.h

{
  "path": "code/jjskin_3dfm_pragmas.h",
  "blob_sha": "07a46cf1ba4580ee4d9660535d1336677a3536c1",
  "sha256": "fce0dfb3fafb2b3492625ab882dcddfe55a673bd42ea5f652ee74defa4c520e9",
  "lines": 160,
  "bytes": 11181,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 7,
    "subsurface": 1,
    "sss": 9,
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/virtualritz/jupiterskin/blob/acff3ff84a24fc228d7f590afeea24155236ea5a/code/jjskin_3dfm_pragmas.h",
  "function_signature_candidates": []
}

```text

/******************************************************************************/
/* Copyright (c) 2009-2010 by Jupiter Jazz Ltd. All Rights Reserved.          */
/*                                                                            */
/******************************************************************************/

////////////////////////////////////////////////////////////////////////////////
/* Jupiter Jazz JJskin
 * author: Luis Barrancos (luis.b.barrancos_AT_gmail.com)
 * */
////////////////////////////////////////////////////////////////////////////////
// set of definitions for 3Delight for Maya's UI
#ifndef JJSKIN_3DFM_PRAGMAS_H
#define JJSKIN_3DFM_PRAGMAS_H	1

#define JJSKIN_3DFM_PRAGMAS \
	_Pragma("annotation Ka \"gadgettype=floatslider;min=0;max=1;hint=Ambient coefficient\"" ) \
	_Pragma("annotation Kd \"gadgettype=floatslider;min=0;max=1;hint=Diffuse coefficient\"" ) \
	_Pragma("annotation Ks1 \"gadgettype=floatslider;min=0;max=1;hint=Specular coefficient for the first specular lobe\"" ) \
	_Pragma("annotation Ks2 \"gadgettype=floatslider;min=0;max=1;hint=Specular coefficient for the second specular lobe\"" ) \
	_Pragma("annotation Ksscale \"gadgettype=floatfield;hint=Scaling factor for specular highlights\"" ) \
	_Pragma("annotation roughness1 \"gadgettype=floatslider;min=0;max=1;hint=Specular roughness for the first specular lobe\"" ) \
	_Pragma("annotation roughness2 \"gadgettype=floatslider;min=0;max=1;hint=Specular roughness for the second specular lobe\"" ) \
	_Pragma("annotation ior \"gadgettype=floatfield;min=1;max=3;hint=Index of refraction, roughly 1.4 for skin\"" ) \
	_Pragma("annotation weight \"gadgettype=floatslider;min=0;max=1;hint=Specular lobes blending factors, 0 gives full weight to first lobe, 1 to second\"" ) \
	_Pragma("annotation diffalb \"gadgettype=inputfile;hint=Diffuse albedo map\"" ) \
	_Pragma("annotation specalb \"gadgettype=inputfile;hint=Specular albedo map\"" ) \
	_Pragma("annotation specmap \"gadgettype=inputfile;hint=Specular intensity map, might have specular roughness for first and second specular lobes in R and G channel, and mixing factor in B channel\"" ) \
	_Pragma("annotation blendmask \"gadgettype=inputfile;hint=Mask file to blend from mapped normals to surface normals;\"" ) \
	_Pragma("annotation transmask \"gadgettype=inputfile;hint=Transparency mask;\"" ) \
	_Pragma("annotation gammain \"gadgettype=floatfield;hint=Input gamma for diffuse and specular maps\"" ) \
	\
	_Pragma("annotation Rmap \"gadgettype=inputfile;hint=R wavelenght normal map\"" ) \
	_Pragma("annotation Gmap \"gadgettype=inputfile;hint=G wavelenght normal map\"" ) \
	_Pragma("annotation Bmap \"gadgettype=inputfile;hint=B wavelenght normal map\"" ) \
	_Pragma("annotation Smap \"gadgettype=inputfile;hint=Specular normal map\"" ) \
	\
	_Pragma("annotation usespecmap \"gadgettype=checkbox;hint=Toggle between using user-supplied roughness parameters, or values from specular map\"" ) \
	_Pragma("annotation useTSlobes \"gadgettype=checkbox;hint=Toggle between using the Beckmann microfacet distribution, or a Gaussian distribution\"" ) \
	_Pragma("annotation romin \"gadgettype=floatslider;min=0;max=1;hint=roughness old minimum\"" ) \
	_Pragma("annotation romax \"gadgettype=floatslider;min=0;max=1;hint=roughness old maximum\"" ) \
	_Pragma("annotation rnmin \"gadgettype=floatslider;min=0;max=1;hint=roughness new minimum\"" ) \
	_Pragma("annotation rnmax \"gadgettype=floatslider;min=0;max=1;hint=roughness new maximum\"" ) \
	_Pragma("annotation ambcolor \"gadgettype=colorslider;hint=Ambient color\"" ) \
	\
	_Pragma("annotation category \"gadgettype=textfield;hint=Light categories\"" ) \
	_Pragma("annotation useenvlight \"gadgettype=checkbox;hint=Use environment lights on/off\"" ) \
	_Pragma("annotation Ksss \"gadgettype=floatslider;min=0;max=1;hint=SSS amount\"" ) \
	_Pragma("annotation scalesss \"gadgettype=floatslider;hint=SSS scale;\"") \
	_Pragma("annotation balancesss \"gadgettype=floatslider;min=0;max=1;hint=SSS weight factor, 0 = no SSS, full diffuse term, 1 = no diffuse term and full SSS;\"") \
	_Pragma("annotation dmfp \"gadgettype=colorslider;hint=Diffuse mean free path\"" ) \
	_Pragma("annotation albedo \"gadgettype=colorslider;hint=Albedo\"" ) \
	_Pragma("annotation dobake \"gadgettype=checkbox;hint=Bake pointcloud for SSS\"" ) \
	_Pragma("annotation bakefile \"gadgettype=inputfile;hint=Point cloud file to read/bake\"" ) \
	_Pragma("annotation ptccoordsys \"gadgettype=textfield;hint=Pointcloud destination coordinate system (world)\"" ) \
	_Pragma("annotation areachannel \"gadgettype=textfield;hint=Channel name for micropolygon area, defaults to _area\"" ) \
	_Pragma("annotation radiancechannel \"gadgettype=textfield;hint=Channel name for radiance, defaults to _radiance_t\"" ) \
	_Pragma("annotation Kr \"gadgettype=floatslider;min=0;max=1;hint=Reflection amount\"") \
	_Pragma("annotation reflcolor \"gadgettype=colorslider;hint=Reflection color\"" ) \
	_Pragma("annotation envmap \"gadgettype=inputfile;hint=Environment map to lookup when there are no ray hits;\"" ) \
	_Pragma("annotation envspace \"gadgettype=textfield;hint=Environment coordinate system;\"") \
	_Pragma("annotation label \"gadgettype=textfield;hint=Ray label;\"") \
	_Pragma("annotation subset \"gadgettype=textfield;hint=Ray subset;\"") \
	_Pragma("annotation hitsides \"gadgettype=optionmenu:front:back:both;hint=Hit sides;\"") \
	_Pragma("annotation filter \"gadgettype=optionmenu:box:triangle:gaussian;hint=Environment lookup filter;\"") \
	_Pragma("annotation raysamples \"gadgettype=intfield;hint=Ray samples;\"") \
	_Pragma("annotation envsamples \"gadgettype=intfield;hint=Environment lookup samples;\"") \
	_Pragma("annotation width \"gadgettype=floatfield;hint=Environment lookup filter width;\"") \
	_Pragma("annotation envblur \"gadgettype=floatfield;hint=Environment lookup blur;\"") \
	_Pragma("annotation bias \"gadgettype=floatfield;hint=Bias;\"") \
	_Pragma("annotation maxdist \"gadgettype=floatfield;hint=Maximum distance to consider ray intersections;\"") \
	/* outputs (hidden... there has to be a better way to do this)*/ \
	_Pragma("annotation aov_ambient_color \"hide=true;\"") \
	_Pragma("annotation aov_diffuse_color \"hide=true;\"") \
	_Pragma("annotation aov_specular_color \"hide=true;\"") \
	_Pragma("annotation aov_reflection_color \"hide=true;\"") \
	_Pragma("annotation aov_ambient \"hide=true;\"") \
	_Pragma("annotation aov_diffuse \"hide=true;\"") \
	_Pragma("annotation aov_specular \"hide=true;\"") \
	_Pragma("annotation aov_specularlobe1 \"hide=true;\"") \
	_Pragma("annotation aov_specularlobe2 \"hide=true;\"") \
	_Pragma("annotation aov_lobebalance \"hide=true;\"") \
	_Pragma("annotation aov_reflection \"hide=true;\"") \
	_Pragma("annotation aov_reflection_raytraced \"hide=true;\"" ) \
	_Pragma("annotation aov_reflection_environment \"hide=true;\"" ) \
	_Pragma("annotation aov_reflection_fresnel \"hide=true;\"" ) \
	_Pragma("annotation aov_indirectdiffuse \"hide=true;\"" ) \
	_Pragma("annotation aov_subsurfacescattering \"hide=true;\"" ) \
	_Pragma("annotation aov_sssmixfactor \"hide=true;\"" ) \
	_Pragma("annotation aov_diffusesssblend \"hide=true;\"" ) \
	_Pragma("annotation aov_shadowing \"hide=true;\"" ) \
	_Pragma("annotation aov_occlusion \"hide=true;\"" ) \
	_Pragma("annotation aov_transparency \"hide=true;\"" ) \
	_Pragma("annotation aov_blendfactor \"hide=true;\"" ) \
	_Pragma("annotation aov_surfacenormal \"hide=true;\"" ) \
	_Pragma("annotation aov_Rmap_normal \"hide=true;\"" ) \
	_Pragma("annotation aov_Gmap_normal \"hide=true;\"" ) \
	_Pragma("annotation aov_Bmap_normal \"hide=true;\"" ) \
	_Pragma("annotation aov_Smap_normal \"hide=true;\"" ) \
	_Pragma("annotation aov_Oi \"hide=true;\"" ) \
	_Pragma("annotation aov_Ci \"hide=true;\"") \
	_Pragma("annotation Pref \"hide=true;\"") \
	_Pragma("annotation Nref \"hide=true;\"")

#define JJSKIN_3DFM_PRAGMA_GROUPS \
	_Pragma("annotation \"grouping\" \"Normal_maps/Rmap;\"") \
	_Pragma("annotation \"grouping\" \"Normal_maps/Gmap;\"") \
	_Pragma("annotation \"grouping\" \"Normal_maps/Bmap;\"") \
	_Pragma("annotation \"grouping\" \"Normal_maps/Smap;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/diffalb;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/specalb;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/specmap;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/blendmask;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/transmask;\"") \
	_Pragma("annotation \"grouping\" \"Texture_maps/gammain;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Ka;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/ambcolor;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Kd;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Lobe1/Ks1;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Lobe1/roughness1;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Lobe2/Ks2;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Lobe2/roughness2;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Ksscale;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/weight;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/ior;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Remap/romin;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Remap/romax;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Remap/rnmin;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/Remap/rnmax;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/usespecmap;\"") \
	_Pragma("annotation \"grouping\" \"BRDF/useTSlobes;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/Ksss;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/dmfp;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/albedo;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/scalesss;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/balancesss;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/dobake;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/bakefile;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/areachannel;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/radiancechannel;\"") \
	_Pragma("annotation \"grouping\" \"BSSRDF/ptccoordsys;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/Kr;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/reflcolor;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/envmap;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/envspace;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/envsamples;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/label;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/subset;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/hitsides;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/raysamples;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/maxdist;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/bias;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/envblur;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/filter;\"") \
	_Pragma("annotation \"grouping\" \"Reflection/width;\"") \
	_Pragma("annotation \"grouping\" \"General/category;\"") \
	_Pragma("annotation \"grouping\" \"General/useenvlight;\"") 

////////////////////////////////////////////////////////////////////////////////
#endif // JJSKIN_3DFM_PRAGMAS_H



```

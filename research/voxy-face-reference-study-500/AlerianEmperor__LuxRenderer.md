# Facial rendering source scan: AlerianEmperor/LuxRenderer

Repository: https://github.com/AlerianEmperor/LuxRenderer

Pinned source revision: 2f9a73d17f6b1e83ed8280eb2345bc813437d937

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["Lux_Renderer/fasttrigo.cpp", "Lux_Renderer/fasttrigo.h", "Lux_Renderer_For_Massive_Model/fasttrigo.cpp", "Lux_Renderer_For_Massive_Model/fasttrigo.h"], "face": ["Lux_Renderer/MicroFacet.h", "Lux_Renderer_For_Massive_Model/MicroFacet.h"]}

## Source file: Lux_Renderer/fasttrigo.h

{
  "path": "Lux_Renderer/fasttrigo.h",
  "blob_sha": "3471ab83674176af3f45d0bf8b4717816d8f9385",
  "sha256": "7bf434849269ad991f30bf23e05e4e1ed691421ca226c6fa443030c65cca456e",
  "lines": 127,
  "bytes": 5110,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/AlerianEmperor/LuxRenderer/blob/2f9a73d17f6b1e83ed8280eb2345bc813437d937/Lux_Renderer/fasttrigo.h",
  "function_signature_candidates": [
    {
      "line": 52,
      "text": "float sqrt(float sqrd);"
    },
    {
      "line": 53,
      "text": "float length(float x, float y);"
    },
    {
      "line": 54,
      "text": "float length(float x, float y, float z);"
    },
    {
      "line": 55,
      "text": "float atan2(float y, float x);"
    },
    {
      "line": 56,
      "text": "float cos(float angle);"
    },
    {
      "line": 57,
      "text": "float sin(float angle);"
    },
    {
      "line": 58,
      "text": "void  sincos(float& angle, float *sin, float *cos);"
    },
    {
      "line": 66,
      "text": "void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);"
    },
    {
      "line": 67,
      "text": "void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);"
    },
    {
      "line": 68,
      "text": "void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);"
    },
    {
      "line": 71,
      "text": "float length(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 72,
      "text": "float length(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 73,
      "text": "float angle(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 74,
      "text": "float azimuth(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 75,
      "text": "float inclination(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 76,
      "text": "float x(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 77,
      "text": "float y(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 78,
      "text": "float x(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 79,
      "text": "float y(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 80,
      "text": "float z(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 91,
      "text": "float sqrt(float sqrd);"
    },
    {
      "line": 92,
      "text": "float length(float x, float y);"
    },
    {
      "line": 93,
      "text": "float length(float x, float y, float z);"
    },
    {
      "line": 94,
      "text": "float atan2(float y, float x);"
    },
    {
      "line": 95,
      "text": "float cos(float angle);"
    },
    {
      "line": 96,
      "text": "float sin(float angle);"
    },
    {
      "line": 97,
      "text": "void  sincos(float angle, float *sin, float *cos);"
    },
    {
      "line": 105,
      "text": "void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);"
    },
    {
      "line": 106,
      "text": "void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);"
    },
    {
      "line": 107,
      "text": "void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);"
    },
    {
      "line": 110,
      "text": "float length(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 111,
      "text": "float length(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 112,
      "text": "float angle(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 113,
      "text": "float azimuth(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 114,
      "text": "float inclination(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 115,
      "text": "float x(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 116,
      "text": "float y(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 117,
      "text": "float x(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 118,
      "text": "float y(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 119,
      "text": "float z(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    }
  ]
}

```text

/* FastTrigo 1.0 (c) 2013 Robin Lobel
Fast yet accurate trigonometric functions
Each namespace (FT, FTA) has 3 sets of functions:
Scalar: standard trigonometric functions
Packed Scalar: same functions computing 4 values at the same time (using SSE/SSE2/SSE3/SSE4.1 if available)
Qt: convenience functions if using QVector2D/QVector3D classes from Qt
FT Accuracy:
FT::sqrt/sqrt_ps max error: 0.032% (average error: 0.0094%)
FT::atan2/atan2_ps max error: 0.024% (0.0015 radians, 0.086 degrees)
FT::cos/cos_ps max error: 0.06%
FT::sin/sin_ps max error: 0.06%
FT Speed up (MSVC2012 x64):
FT::sqrt speed up: x2.5 (from standard sqrt)
FT::atan2 speed up: x2.3 (from standard atan2)
FT::sin/cos speed up: x1.9 (from standard sin/cos)
FT::sincos speed up: x2.3 (from standard sin+cos)
FT::sqrt_ps speed up: x8 (from standard sqrt)
FT::atan2_ps speed up: x7.3 (from standard atan2)
FT::sin_ps/cos_ps speed up: x4.9 (from standard sin/cos)
FT::sincos_ps speed up: x6.2 (from standard sin+cos)
FTA Accuracy:
FTA::sqrt/sqrt_ps max error: 0%
FTA::atan2/atan2_ps max error: 0.0005%
FTA::cos/cos_ps max error: 0.0007%
FTA::sin/sin_ps max error: 0.0007%
FTA Speed up (MSVC2012 x64):
FTA::sqrt speed up: x1.5 (from standard sqrt)
FTA::atan2 speed up: x1.7 (from standard atan2)
FTA::sin/cos speed up: x1.6 (from standard sin/cos)
FTA::sincos speed up: x1.8 (from standard sin+cos)
FTA::sqrt_ps speed up: x4.9 (from standard sqrt)
FTA::atan2_ps speed up: x5.2 (from standard atan2)
FTA::sin_ps/cos_ps speed up: x4.3 (from standard sin/cos)
FTA::sincos_ps speed up: x5.2 (from standard sin+cos)
Distributed under Revised BSD License
*/

#ifndef FASTTRIGO_H
#define FASTTRIGO_H

#include <math.h>
#ifdef QT_GUI_LIB
#include <QtGui>
#endif
#include <intrin.h>
#include <xmmintrin.h>
#include <pmmintrin.h>

//Default accuracy
namespace FT
{
	float sqrt(float sqrd);
	float length(float x, float y);
	float length(float x, float y, float z);
	float atan2(float y, float x);
	float cos(float angle);
	float sin(float angle);
	void  sincos(float& angle, float *sin, float *cos);

	__m128 sqrt_ps(__m128 sqrd);
	__m128 length_ps(__m128 x, __m128 y);
	__m128 length_ps(__m128 x, __m128 y, __m128 z);
	__m128 atan2_ps(__m128 y, __m128 x);
	__m128 cos_ps(__m128 angle);
	__m128 sin_ps(__m128 angle);
	void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);
	void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);
	void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);

#ifdef QT_GUI_LIB
	float length(QVector2D vector); //cartesian vector(x,y)
	float length(QVector3D vector); //cartesian vector(x,y,z)
	float angle(QVector2D vector); //cartesian vector(x,y)
	float azimuth(QVector3D vector); //cartesian vector(x,y,z)
	float inclination(QVector3D vector); //cartesian vector(x,y,z)
	float x(QVector2D vector); //polar vector(length,angle)
	float y(QVector2D vector); //polar vector(length,angle)
	float x(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float y(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float z(QVector3D vector); //spherical vector(length,azimuth,inclination)
	QVector2D cartesian2polar(QVector2D vector);
	QVector2D polar2cartesian(QVector2D vector);
	QVector3D cartesian2spherical(QVector3D vector);
	QVector3D spherical2cartesian(QVector3D vector);
#endif
};

//More accurate
namespace FTA
{
	float sqrt(float sqrd);
	float length(float x, float y);
	float length(float x, float y, float z);
	float atan2(float y, float x);
	float cos(float angle);
	float sin(float angle);
	void  sincos(float angle, float *sin, float *cos);

	__m128 sqrt_ps(__m128 sqrd);
	__m128 length_ps(__m128 x, __m128 y);
	__m128 length_ps(__m128 x, __m128 y, __m128 z);
	__m128 atan2_ps(__m128 y, __m128 x);
	__m128 cos_ps(__m128 angle);
	__m128 sin_ps(__m128 angle);
	void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);
	void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);
	void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);

#ifdef QT_GUI_LIB
	float length(QVector2D vector); //cartesian vector(x,y)
	float length(QVector3D vector); //cartesian vector(x,y,z)
	float angle(QVector2D vector); //cartesian vector(x,y)
	float azimuth(QVector3D vector); //cartesian vector(x,y,z)
	float inclination(QVector3D vector); //cartesian vector(x,y,z)
	float x(QVector2D vector); //polar vector(length,angle)
	float y(QVector2D vector); //polar vector(length,angle)
	float x(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float y(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float z(QVector3D vector); //spherical vector(length,azimuth,inclination)
	QVector2D cartesian2polar(QVector2D vector);
	QVector2D polar2cartesian(QVector2D vector);
	QVector3D cartesian2spherical(QVector3D vector);
	QVector3D spherical2cartesian(QVector3D vector);
#endif
};

#endif // FASTTRIGO_H

```

## Source file: Lux_Renderer/MicroFacet.h

{
  "path": "Lux_Renderer/MicroFacet.h",
  "blob_sha": "ef2dba2b8b992ba298128f4e8a63136f7874325f",
  "sha256": "7c31853114ad6041fed18c8a6fc50f43654a65aff08ef266c754b8234ffd9984",
  "lines": 225,
  "bytes": 5522,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 13
  },
  "url": "https://github.com/AlerianEmperor/LuxRenderer/blob/2f9a73d17f6b1e83ed8280eb2345bc813437d937/Lux_Renderer/MicroFacet.h",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "float Roughness_To_Alpha(const Microfacet_Distribution& dist, const float& roughness)"
    },
    {
      "line": 29,
      "text": "float Microfacet_D(const Microfacet_Distribution& dist, const vec3& m, const vec3& n, const float& alpha)"
    },
    {
      "line": 85,
      "text": "float G1(const Microfacet_Distribution& dist, const vec3& v, const vec3& m, const vec3& n, const float& alpha)"
    },
    {
      "line": 147,
      "text": "float Microfacet_G(const Microfacet_Distribution& dist, const vec3& v_in, const vec3& v_out, const vec3& m, const vec3& n, const float& alpha)"
    },
    {
      "line": 152,
      "text": "float Microfacet_pdf(const Microfacet_Distribution& dist, const vec3& m, const vec3& n, const float& alpha)"
    },
    {
      "line": 165,
      "text": "vec3 Microfacet_Sample(const Microfacet_Distribution& dist, const vec3& n, const float& alpha)"
    }
  ]
}

```text

#ifndef _MICRO_FACET_H_
#define _MICRO_FACET_H_

#include "onb.h"
enum Microfacet_Distribution
{
	Beckmann,
	Phong,
	GGX
};

//1e-6f -> eps
float Roughness_To_Alpha(const Microfacet_Distribution& dist, const float& roughness)
{
	float minAlpha = 0.001f;
	float roughness_value = max(roughness, minAlpha);

	if (dist == Phong)
		return 2.0f / (roughness_value * roughness) - 2.0f;
	else
		return roughness_value;
}

// m = micro surface normal
// n = macro surface normal



float Microfacet_D(const Microfacet_Distribution& dist, const vec3& m, const vec3& n, const float& alpha)
{
	float cos_theta = m.dot(n);

	//0 condition
	if (cos_theta <= eps)//<=0.0f
	//if (cos_theta <= 0.0f)
		return 0.0f;//0.0f
		//return 0.0f;
	//if (cos_theta <= 0.0f)
	//	return 0.0f;

	switch (dist)
	{
		case GGX:
		{
			float alpha2 = alpha * alpha;
			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta2 = max_value(1.0f - cos_theta2, 0.0f) / cos_theta2;
			//float tan_theta2 = (1.0f - cos_theta2) / cos_theta2;
			float cos_theta4 = cos_theta2 * cos_theta2;

			//return alpha2 * ipi / (cos_theta4 * sqr(alpha2 + tan_theta2));
			
			float denom = alpha2 + tan_theta2;

			return alpha2 * ipi / (cos_theta4 * denom * denom);
			
			/*
			float cos_theta2 = cos_theta * cos_theta;

			float denom = cos_theta2 * (alpha * alpha - 1.0f) + 1.0f;

			return alpha * alpha / (pi * denom * denom);
			*/
			//return 1.0f;
		}
		case Beckmann:
		{
			float alpha2 = alpha * alpha;
			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta2 = (1.0f - cos_theta2) / cos_theta2;//maxf(1.0f - cos_theta2, 0.0f) / cos_theta2;
			float cos_theta4 = cos_theta2 * cos_theta2;

			return expf(-tan_theta2 / alpha2) / (pi * alpha2 * cos_theta4);
		}
		case Phong:	
		{
			float specular_component = powf(cos_theta, alpha);
			return (alpha + 2.0f) * specular_component * itau;
		}
		
	}
	return 0.0f;
}

float G1(const Microfacet_Distribution& dist, const vec3& v, const vec3& m, const vec3& n, const float& alpha)
{
	float cos_theta = v.dot(n);
	float cos_h = v.dot(m);

	//if (cos_theta <= 0.0f || cos_h <= 0.0f)
	//	return 1.0f;

	//0 condition
	if (cos_theta <= eps || cos_h <= eps)//<=0.0f
	//if (cos_theta <= 0.0f || cos_h <= 0.0f)
		return 0.0f;//0.0f
		//return 0.0f;
	//if (cos_theta <= 0.0f || cos_h <= 0.0f)
	//	return 0.0f;

	switch (dist)
	{
		case GGX:
		{
			/*float alpha2 = alpha * alpha;
			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta2 = max(1.0f - cos_theta2, 0.0f) / cos_theta2;

			return 2.0f / (1.0f + sqrt14(1.0f + alpha2 * tan_theta2));*/

			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta2 = max((1.0f - cos_theta2), 0.0f) / cos_theta2;//max(1.0f - cos_theta2, 0.0f) / cos_theta2;

			//if (tan_theta2 == 0.0f)
			//	return 0.0f;

			float alpha2 = alpha * alpha;

			return 2.0f / (1.0f + sqrt14(1.0f + alpha2 * tan_theta2));
		}
		case Beckmann:
		{
			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta = (sqrt14(max_value(1.0f - cos_theta2, 0.0f)) / cos_theta);
			float a = 1.0f / (alpha * tan_theta);

			if (a < 1.6f)
				return (3.535f * a + 2.181f * a * a) / (1.0f + 2.276f * a + 2.577f * a * a);
			else
				return 1.0f;
		}
		case Phong:
		{
			float cos_theta2 = cos_theta * cos_theta;
			float tan_theta = abs(sqrt14(max_value(1.0f - cos_theta2, 0.0f)) / cos_theta);
			float a = sqrt14(0.5f * alpha + 1.0f) / tan_theta;

			if(a < 1.6f)
				return (3.535f * a + 2.181f * a * a) / (1.0f + 2.276f * a + 2.577f * a * a);
			else
				return 1.0f;
		}
	}
	return 0.0f;
}

float Microfacet_G(const Microfacet_Distribution& dist, const vec3& v_in, const vec3& v_out, const vec3& m, const vec3& n, const float& alpha)
{
	return G1(dist, -v_in, m, n, alpha) * G1(dist, v_out, m, n, alpha);
}

float Microfacet_pdf(const Microfacet_Distribution& dist, const vec3& m, const vec3& n, const float& alpha)
{
	float pdf = Microfacet_D(dist, m, n, alpha) * m.dot(n);

	//0 condition
	if (pdf <= eps)//<=0.0f
		return 0.0f;
	//if(pdf <= 0.0f)
	//	return 1.0f;

	return pdf;
}

vec3 Microfacet_Sample(const Microfacet_Distribution& dist, const vec3& n, const float& alpha)
{
	float u1 = randf();
	float u2 = randf();

	float phi = 2.0f * pi * u2;
	float cos_theta;

	switch (dist)
	{
		case GGX:
		{
			//float tan_theta2 = alpha * alpha * u / (1.0f - u);
			//cos_theta = 1.0f / sqrt14(1.0f + tan_theta2);

			//float value = sqrt14(u / (1.0f - u));
			//float theta = atan(alpha * value);

			//cos_theta = cosf(theta);

			cos_theta = sqrt14((1.0f - u1) / (u1 * (alpha * alpha - 1.0f) + 1.0f));

			//float sin_theta = sqrt14(1.0f - cos_theta * cos_theta);

			break;
		}
		case Beckmann:
		{
			float tan_theta2 = -alpha * alpha * log10f(1.0f - u1);
			cos_theta = 1.0f / sqrt14(1.0f + tan_theta2);

			break;
		}
		case Phong:
		{
			cos_theta = powf(u1, 1.0f / (alpha + 2.0f));
			break;
		}
		
	}

	onb local(n);

	float sin_theta = sqrt14(max_value(1.0f - cos_theta * cos_theta, 0.0f));

	//https://gist.github.com/telegraphic/841212e8ab3252f5cffe
	//line 93 - 95
	//return vec3(cosf(phi) * sin_theta, sinf(phi) * sin_theta, cos_theta);

	//float cos_phi, sin_phi;

	//FTA::sincos(phi, &sin_phi, &cos_phi);

	vec3 h = (local.u * cosf(phi) + local.v * sinf(phi)) * sin_theta + n * cos_theta;

	return h.norm();

}

#endif // !_MICRO_FACET_H_



```

## Source file: Lux_Renderer/fasttrigo.cpp

{
  "path": "Lux_Renderer/fasttrigo.cpp",
  "blob_sha": "eb02ef6c81f7effec7b586094bdf286e9aa6e618",
  "sha256": "49b2b4bdbe6335ce61ffe1ae51698d910517992c20e62a1cb4caf8f722089bb3",
  "lines": 579,
  "bytes": 21534,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 4,
    "rig": 1
  },
  "url": "https://github.com/AlerianEmperor/LuxRenderer/blob/2f9a73d17f6b1e83ed8280eb2345bc813437d937/Lux_Renderer/fasttrigo.cpp",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "float atan(float x);"
    },
    {
      "line": 31,
      "text": "float cos_32s(float x);"
    },
    {
      "line": 306,
      "text": "float atan(float x);"
    },
    {
      "line": 307,
      "text": "float cos_52s(float x);"
    }
  ]
}

```text

#include "fasttrigo.h"

//Most ninja tricks used here:
//http://fastcpp.blogspot.fr/2011/03/changing-sign-of-float-values-using-sse.html
//http://www.songho.ca/misc/sse/sse.html
//http://markplusplus.wordpress.com/2007/03/14/fast-sse-select-operation/
//http://www.masmforum.com/board/index.php?PHPSESSID=786dd40408172108b65a5a36b09c88c0&topic=9515.0
//http://cbloomrants.blogspot.fr/2010/11/11-20-10-function-approximation-by_20.html
//http://assemblyrequired.crashworks.org/2009/10/16/timing-sqr-root/
//http://nghiaho.com/?p=997
//http://www.researchgate.net/publication/3321724_Efficient_approximations_for_the_arctangent_function
//http://www.ganssle.com/approx/approx.pdf
//http://forum.allaboutcircuits.com/newsgroups/viewtopic.php?t=68185

const float invtwopi = 0.1591549f;
const float twopi = 6.283185f;
const float threehalfpi = 4.7123889f;
const float pi = 3.141593f;
const float halfpi = 1.570796f;
const float quarterpi = 0.7853982f;
static const __m128 SIGNMASK = _mm_castsi128_ps(_mm_set1_epi32(0x80000000));

///////////////////////////////////
//FT NAMESPACE (DEFAULT ACCURACY)//
///////////////////////////////////

//SCALAR
namespace FT
{
	float atan(float x);
	float cos_32s(float x);

	__m128 atan_ps(__m128 x);
	__m128 cos_32s_ps(__m128 x);
};

float FT::sqrt(float sqrd)
{
	static int csr = 0;
	if (!csr) csr = _mm_getcsr() | 0x8040; //DAZ,FTZ (divide by zero=0)
	_mm_setcsr(csr);
	return _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(sqrd)))*sqrd;
}
float FT::length(float x, float y)
{
	return FT::sqrt(x*x + y*y);
}
float FT::length(float x, float y, float z)
{
	return FT::sqrt(x*x + y*y + z*z);
}

float FT::atan(float x)
{
	return quarterpi*x - x*(fabs(x) - 1)*(0.2447f + 0.0663f*fabs(x));
}
float FT::atan2(float y, float x)
{
	if (fabs(x)>fabs(y)) {
		float atan = FT::atan(y / x);
		if (x>0.f)
			return atan;
		else
			return y>0.f ? atan + pi : atan - pi;
	}
	else {
		float atan = FT::atan(x / y);
		if (x>0.f)
			return y>0.f ? halfpi - atan : -halfpi - atan;
		else
			return y>0.f ? halfpi + atan : -halfpi + atan;
	}
}

float FT::cos_32s(float x)
{
	const float c1 = 0.99940307f;
	const float c2 = -0.49558072f;
	const float c3 = 0.03679168f;
	float x2;      // The input argument sqrd
	x2 = x * x;
	return (c1 + x2*(c2 + c3 * x2));
}
float FT::cos(float angle) {
	//clamp to the range 0..2pi
	angle = angle - floorf(angle*invtwopi)*twopi;
	angle = angle>0.f ? angle : -angle;

	if (angle<halfpi) return FT::cos_32s(angle);
	if (angle<pi) return -FT::cos_32s(pi - angle);
	if (angle<threehalfpi) return -FT::cos_32s(angle - pi);
	return FT::cos_32s(twopi - angle);
}
float FT::sin(float angle) {
	return FT::cos(halfpi - angle);
}
void FT::sincos(float& angle, float *sin, float *cos) {
	//clamp to the range 0..2pi
	//angle = angle - floorf(angle*invtwopi)*twopi;
	
	float sinmultiplier = angle>0.f&&angle<pi ? 1.f : -1.f;
	angle = angle>0.f ? angle : -angle;

	if (angle<halfpi) {
		*cos = FT::cos_32s(angle);
		*sin = sinmultiplier*FT::sqrt(1.f - *cos**cos);
		return;
	}
	if (angle<pi) {
		*cos = -FT::cos_32s(pi - angle);
		*sin = sinmultiplier*FT::sqrt(1.f - *cos**cos);
		return;
	}
	if (angle<threehalfpi) {
		*cos = -FT::cos_32s(angle - pi);
		*sin = sinmultiplier*FT::sqrt(1.f - *cos**cos);
		return;
	}
	*cos = FT::cos_32s(twopi - angle);
	*sin = sinmultiplier*FT::sqrt(1.f - *cos**cos);
	return;
}

//PACKED SCALAR
__m128 FT::sqrt_ps(__m128 sqrd)
{
	static int csr = 0;
	if (!csr) csr = _mm_getcsr() | 0x8040; //DAZ,FTZ (divide by zero=0)
	_mm_setcsr(csr);
	return _mm_mul_ps(_mm_rsqrt_ps(sqrd), sqrd);
}
__m128 FT::length_ps(__m128 x, __m128 y)
{
	return FT::sqrt_ps(_mm_add_ps(_mm_mul_ps(x, x), _mm_mul_ps(y, y)));
}
__m128 FT::length_ps(__m128 x, __m128 y, __m128 z)
{
	return FT::sqrt_ps(_mm_add_ps(_mm_add_ps(_mm_mul_ps(x, x), _mm_mul_ps(y, y)), _mm_mul_ps(z, z)));
}

__m128 FT::atan_ps(__m128 x)
{
	//                                      quarterpi*x
	//                                      - x*(fabs(x) - 1)
	//                                      *(0.2447f+0.0663f*fabs(x));
	return _mm_sub_ps(_mm_mul_ps(_mm_set1_ps(quarterpi), x),
		_mm_mul_ps(_mm_mul_ps(x, _mm_sub_ps(_mm_andnot_ps(SIGNMASK, x), _mm_set1_ps(1.f))),
		(_mm_add_ps(_mm_set1_ps(0.2447f), _mm_mul_ps(_mm_set1_ps(0.0663f), _mm_andnot_ps(SIGNMASK, x))))));
}
__m128 FT::atan2_ps(__m128 y, __m128 x)
{
	__m128 absxgreaterthanabsy = _mm_cmpgt_ps(_mm_andnot_ps(SIGNMASK, x), _mm_andnot_ps(SIGNMASK, y));
	__m128 ratio = _mm_div_ps(_mm_add_ps(_mm_and_ps(absxgreaterthanabsy, y), _mm_andnot_ps(absxgreaterthanabsy, x)),
		_mm_add_ps(_mm_and_ps(absxgreaterthanabsy, x), _mm_andnot_ps(absxgreaterthanabsy, y)));
	__m128 atan = FT::atan_ps(ratio);

	__m128 xgreaterthan0 = _mm_cmpgt_ps(x, _mm_set1_ps(0.f));
	__m128 ygreaterthan0 = _mm_cmpgt_ps(y, _mm_set1_ps(0.f));

	atan = _mm_xor_ps(atan, _mm_andnot_ps(absxgreaterthanabsy, _mm_and_ps(xgreaterthan0, SIGNMASK))); //negate atan if absx<=absy & x>0

	__m128 shift = _mm_set1_ps(pi);
	shift = _mm_sub_ps(shift, _mm_andnot_ps(absxgreaterthanabsy, _mm_set1_ps(halfpi))); //substract halfpi if absx<=absy
	shift = _mm_xor_ps(shift, _mm_andnot_ps(ygreaterthan0, SIGNMASK)); //negate shift if y<=0
	shift = _mm_andnot_ps(_mm_and_ps(absxgreaterthanabsy, xgreaterthan0), shift); //null if abs>absy & x>0

	return _mm_add_ps(atan, shift);
}

__m128 FT::cos_32s_ps(__m128 x)
{
	const __m128 c1 = _mm_set1_ps(0.99940307f);
	const __m128 c2 = _mm_set1_ps(-0.49558072f);
	const __m128 c3 = _mm_set1_ps(0.03679168f);
	__m128 x2;      // The input argument sqrd
	x2 = _mm_mul_ps(x, x);
	//               (c1+           x2*          (c2+           c3*x2));
	return _mm_add_ps(c1, _mm_mul_ps(x2, _mm_add_ps(c2, _mm_mul_ps(c3, x2))));
}
__m128 FT::cos_ps(__m128 angle) {
	//clamp to the range 0..2pi

	//take absolute value
	angle = _mm_andnot_ps(SIGNMASK, angle);
	//fmod(angle,twopi)
	angle = _mm_sub_ps(angle, _mm_mul_ps(_mm_cvtepi32_ps(_mm_cvttps_epi32(_mm_mul_ps(angle, _mm_set1_ps(invtwopi)))), _mm_set1_ps(twopi))); //simplied SSE2 fmod, must always operate on absolute value
																																			//if SSE4.1 is always available, comment the line above and uncomment the line below
																																			//angle=_mm_sub_ps(angle,_mm_mul_ps(_mm_floor_ps(_mm_mul_ps(angle,_mm_set1_ps(invtwopi))),_mm_set1_ps(twopi))); //faster if SSE4.1 is always available

	__m128 cosangle = angle;
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(pi), angle))));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(pi)), SIGNMASK));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(threehalfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(twopi), angle))));

	__m128 result = FT::cos_32s_ps(cosangle);

	result = _mm_xor_ps(result, _mm_and_ps(_mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_cmplt_ps(angle, _mm_set1_ps(threehalfpi))), SIGNMASK));
	return result;
}
__m128 FT::sin_ps(__m128 angle) {
	return FT::cos_ps(_mm_sub_ps(_mm_set1_ps(halfpi), angle));
}
void FT::sincos_ps(__m128 angle, __m128 *sin, __m128 *cos) {
	__m128 anglesign = _mm_or_ps(_mm_set1_ps(1.f), _mm_and_ps(SIGNMASK, angle));

	//clamp to the range 0..2pi

	//take absolute value
	angle = _mm_andnot_ps(SIGNMASK, angle);
	//fmod(angle,twopi)
	angle = _mm_sub_ps(angle, _mm_mul_ps(_mm_cvtepi32_ps(_mm_cvttps_epi32(_mm_mul_ps(angle, _mm_set1_ps(invtwopi)))), _mm_set1_ps(twopi))); //simplied SSE2 fmod, must always operate on absolute value
																																			//if SSE4.1 is always available, comment the line above and uncomment the line below
																																			//angle=_mm_sub_ps(angle,_mm_mul_ps(_mm_floor_ps(_mm_mul_ps(angle,_mm_set1_ps(invtwopi))),_mm_set1_ps(twopi))); //faster if SSE4.1 is always available

	__m128 cosangle = angle;
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(pi), angle))));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(pi)), SIGNMASK));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(threehalfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(twopi), angle))));

	__m128 result = FT::cos_32s_ps(cosangle);

	result = _mm_xor_ps(result, _mm_and_ps(_mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_cmplt_ps(angle, _mm_set1_ps(threehalfpi))), SIGNMASK));
	*cos = result;

	__m128 sinmultiplier = _mm_mul_ps(anglesign, _mm_or_ps(_mm_set1_ps(1.f), _mm_and_ps(_mm_cmpgt_ps(angle, _mm_set1_ps(pi)), SIGNMASK)));
	*sin = _mm_mul_ps(sinmultiplier, FT::sqrt_ps(_mm_sub_ps(_mm_set1_ps(1.f), _mm_mul_ps(result, result))));

	return;
}

void FT::interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3)
{
	*x0y0x1y1 = _mm_unpacklo_ps(x0x1x2x3, y0y1y2y3);
	*x2y2x3y3 = _mm_unpackhi_ps(x0x1x2x3, y0y1y2y3);
}
void FT::deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3)
{
	*x0x1x2x3 = _mm_shuffle_ps(x0y0x1y1, x2y2x3y3, _MM_SHUFFLE(2, 0, 2, 0));
	*y0y1y2y3 = _mm_shuffle_ps(x0y0x1y1, x2y2x3y3, _MM_SHUFFLE(3, 1, 3, 1));
}

//QT
#ifdef QT_GUI_LIB
float FT::length(QVector2D vector) {
	return FT::length(vector.x(), vector.y());
}
float FT::length(QVector3D vector) {
	return FT::length(vector.x(), vector.y(), vector.z());
}
float FT::angle(QVector2D vector) {
	return FT::atan2(vector.y(), vector.x());
}
float FT::azimuth(QVector3D vector) {
	return FT::atan2(vector.y(), vector.x());
}
float FT::inclination(QVector3D vector) {
	float projectedlength = FT::length(vector.toVector2D());
	return FT::atan2(vector.z(), projectedlength);
}
float FT::x(QVector2D vector) {
	return vector.x()*FT::cos(vector.y());
}
float FT::y(QVector2D vector) {
	return vector.x()*FT::sin(vector.y());
}
float FT::x(QVector3D vector) {
	return vector.x()*FT::cos(vector.y())*FT::cos(vector.z());
}
float FT::y(QVector3D vector) {
	return vector.x()*FT::sin(vector.y())*FT::cos(vector.z());
}
float FT::z(QVector3D vector) {
	return vector.x()*FT::sin(vector.z());
}
QVector2D FT::cartesian2polar(QVector2D vector) {
	return QVector2D(FT::length(vector), FT::angle(vector));
}
QVector2D FT::polar2cartesian(QVector2D vector) {
	float sin, cos;
	FT::sincos(vector.y(), &sin, &cos);
	return QVector2D(vector.x()*cos,
		vector.x()*sin);
}
QVector3D FT::cartesian2spherical(QVector3D vector) {
	return QVector3D(FT::length(vector), FT::azimuth(vector), FT::inclination(vector));
}
QVector3D FT::spherical2cartesian(QVector3D vector) {
	float azimuthsin, azimuthcos;
	FT::sincos(vector.y(), &azimuthsin, &azimuthcos);
	float inclinationsin, inclinationcos;
	FT::sincos(vector.z(), &inclinationsin, &inclinationcos);
	return QVector3D(vector.x()*azimuthcos*inclinationcos,
		vector.x()*azimuthsin*inclinationcos,
		vector.x()*inclinationsin);
}
#endif


/////////////////////////////////
//FTA NAMESPACE (MORE ACCURATE)//
/////////////////////////////////

//SCALAR
namespace FTA
{
	float atan(float x);
	float cos_52s(float x);

	__m128 atan_ps(__m128 x);
	__m128 cos_52s_ps(__m128 x);
};

float FTA::sqrt(float sqrd)
{
	return _mm_cvtss_f32(_mm_sqrt_ss(_mm_set_ss(sqrd)));
}
float FTA::length(float x, float y)
{
	return FTA::sqrt(x*x + y*y);
}
float FTA::length(float x, float y, float z)
{
	return FTA::sqrt(x*x + y*y + z*z);
}

float FTA::atan(float x)
{
	float u = x*x;
	float u2 = u*u;
	float u3 = u2*u;
	float u4 = u3*u;
	float f = 1.f + 0.33288950512027f*u - 0.08467922817644f*u2 + 0.03252232640125f*u3 - 0.00749305860992f*u4;
	return x / f;
}
float FTA::atan2(float y, float x)
{
	if (fabs(x)>fabs(y)) {
		float atan = FTA::atan(y / x);
		if (x>0.f)
			return atan;
		else
			return y>0.f ? atan + pi : atan - pi;
	}
	else {
		float atan = FTA::atan(x / y);
		if (x>0.f)
			return y>0.f ? halfpi - atan : -halfpi - atan;
		else
			return y>0.f ? halfpi + atan : -halfpi + atan;
	}
}

float FTA::cos_52s(float x)
{
	const float c1 = 0.9999932946f;
	const float c2 = -0.4999124376f;
	const float c3 = 0.0414877472f;
	const float c4 = -0.0012712095f;
	float x2;      // The input argument sqrd
	x2 = x*x;
	return (c1 + x2*(c2 + x2*(c3 + c4*x2)));
}
float FTA::cos(float angle) {
	//clamp to the range 0..2pi
	angle = angle - floorf(angle*invtwopi)*twopi;
	angle = angle>0.f ? angle : -angle;

	if (angle<halfpi) return FTA::cos_52s(angle);
	if (angle<pi) return -FTA::cos_52s(pi - angle);
	if (angle<threehalfpi) return -FTA::cos_52s(angle - pi);
	return FTA::cos_52s(twopi - angle);
}
float FTA::sin(float angle) {
	return FTA::cos(halfpi - angle);
}
//konh nen them & vao float& angle
void FTA::sincos(float angle, float *sin, float *cos) {
	//clamp to the range 0..2pi
	angle = angle - floorf(angle*invtwopi)*twopi;
	float sinmultiplier = angle>0.f&&angle<pi ? 1.f : -1.f;
	angle = angle>0.f ? angle : -angle;

	if (angle<halfpi) {
		*cos = FTA::cos_52s(angle);
		*sin = sinmultiplier*FTA::sqrt(1.f - *cos**cos);
		return;
	}
	if (angle<pi) {
		*cos = -FTA::cos_52s(pi - angle);
		*sin = sinmultiplier*FTA::sqrt(1.f - *cos**cos);
		return;
	}
	if (angle<threehalfpi) {
		*cos = -FTA::cos_52s(angle - pi);
		*sin = sinmultiplier*FTA::sqrt(1.f - *cos**cos);
		return;
	}
	*cos = FTA::cos_52s(twopi - angle);
	*sin = sinmultiplier*FTA::sqrt(1.f - *cos**cos);
	return;
}

//PACKED SCALAR
__m128 FTA::sqrt_ps(__m128 sqrd)
{
	return _mm_sqrt_ps(sqrd);
}
__m128 FTA::length_ps(__m128 x, __m128 y)
{
	return FTA::sqrt_ps(_mm_add_ps(_mm_mul_ps(x, x), _mm_mul_ps(y, y)));
}
__m128 FTA::length_ps(__m128 x, __m128 y, __m128 z)
{
	return FTA::sqrt_ps(_mm_add_ps(_mm_add_ps(_mm_mul_ps(x, x), _mm_mul_ps(y, y)), _mm_mul_ps(z, z)));
}

__m128 FTA::atan_ps(__m128 x)
{
	__m128 u = _mm_mul_ps(x, x);
	__m128 u2 = _mm_mul_ps(u, u);
	__m128 u3 = _mm_mul_ps(u2, u);
	__m128 u4 = _mm_mul_ps(u3, u);
	//__m128 f=1.f+0.33288950512027f*u-0.08467922817644f*u2+0.03252232640125f*u3-0.00749305860992f*u4;

	__m128 f = _mm_add_ps(_mm_add_ps(_mm_add_ps(_mm_add_ps(_mm_set1_ps(1.f),
		_mm_mul_ps(_mm_set1_ps(0.33288950512027f), u)),
		_mm_mul_ps(_mm_set1_ps(-0.08467922817644f), u2)),
		_mm_mul_ps(_mm_set1_ps(0.03252232640125f), u3)),
		_mm_mul_ps(_mm_set1_ps(-0.00749305860992f), u4));
	return _mm_div_ps(x, f);
}
__m128 FTA::atan2_ps(__m128 y, __m128 x)
{
	__m128 absxgreaterthanabsy = _mm_cmpgt_ps(_mm_andnot_ps(SIGNMASK, x), _mm_andnot_ps(SIGNMASK, y));
	__m128 ratio = _mm_div_ps(_mm_add_ps(_mm_and_ps(absxgreaterthanabsy, y), _mm_andnot_ps(absxgreaterthanabsy, x)),
		_mm_add_ps(_mm_and_ps(absxgreaterthanabsy, x), _mm_andnot_ps(absxgreaterthanabsy, y)));
	__m128 atan = FTA::atan_ps(ratio);

	__m128 xgreaterthan0 = _mm_cmpgt_ps(x, _mm_set1_ps(0.f));
	__m128 ygreaterthan0 = _mm_cmpgt_ps(y, _mm_set1_ps(0.f));

	atan = _mm_xor_ps(atan, _mm_andnot_ps(absxgreaterthanabsy, _mm_and_ps(xgreaterthan0, SIGNMASK))); //negate atan if absx<=absy & x>0

	__m128 shift = _mm_set1_ps(pi);
	shift = _mm_sub_ps(shift, _mm_andnot_ps(absxgreaterthanabsy, _mm_set1_ps(halfpi))); //substract halfpi if absx<=absy
	shift = _mm_xor_ps(shift, _mm_andnot_ps(ygreaterthan0, SIGNMASK)); //negate shift if y<=0
	shift = _mm_andnot_ps(_mm_and_ps(absxgreaterthanabsy, xgreaterthan0), shift); //null if abs>absy & x>0

	return _mm_add_ps(atan, shift);
}

__m128 FTA::cos_52s_ps(__m128 x)
{
	const __m128 c1 = _mm_set1_ps(0.9999932946f);
	const __m128 c2 = _mm_set1_ps(-0.4999124376f);
	const __m128 c3 = _mm_set1_ps(0.0414877472f);
	const __m128 c4 = _mm_set1_ps(-0.0012712095f);
	__m128 x2;      // The input argument sqrd
	x2 = _mm_mul_ps(x, x);
	//               (c1+           x2*          (c2+           x2*          (c3+           c4*x2)));
	return _mm_add_ps(c1, _mm_mul_ps(x2, _mm_add_ps(c2, _mm_mul_ps(x2, _mm_add_ps(c3, _mm_mul_ps(c4, x2))))));
}
__m128 FTA::cos_ps(__m128 angle) {
	//clamp to the range 0..2pi

	//take absolute value
	angle = _mm_andnot_ps(SIGNMASK, angle);
	//fmod(angle,twopi)
	angle = _mm_sub_ps(angle, _mm_mul_ps(_mm_cvtepi32_ps(_mm_cvttps_epi32(_mm_mul_ps(angle, _mm_set1_ps(invtwopi)))), _mm_set1_ps(twopi))); //simplied SSE2 fmod, must always operate on absolute value
																																			//if SSE4.1 is always available, comment the line above and uncomment the line below
																																			//angle=_mm_sub_ps(angle,_mm_mul_ps(_mm_floor_ps(_mm_mul_ps(angle,_mm_set1_ps(invtwopi))),_mm_set1_ps(twopi))); //faster if SSE4.1 is always available

	__m128 cosangle = angle;
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(pi), angle))));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(pi)), SIGNMASK));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(threehalfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(twopi), angle))));

	__m128 result = FTA::cos_52s_ps(cosangle);

	result = _mm_xor_ps(result, _mm_and_ps(_mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_cmplt_ps(angle, _mm_set1_ps(threehalfpi))), SIGNMASK));
	return result;
}
__m128 FTA::sin_ps(__m128 angle) {
	return FTA::cos_ps(_mm_sub_ps(_mm_set1_ps(halfpi), angle));
}
void FTA::sincos_ps(__m128 angle, __m128 *sin, __m128 *cos) {
	__m128 anglesign = _mm_or_ps(_mm_set1_ps(1.f), _mm_and_ps(SIGNMASK, angle));

	//clamp to the range 0..2pi

	//take absolute value
	angle = _mm_andnot_ps(SIGNMASK, angle);
	//fmod(angle,twopi)
	angle = _mm_sub_ps(angle, _mm_mul_ps(_mm_cvtepi32_ps(_mm_cvttps_epi32(_mm_mul_ps(angle, _mm_set1_ps(invtwopi)))), _mm_set1_ps(twopi))); //simplied SSE2 fmod, must always operate on absolute value
																																			//if SSE4.1 is always available, comment the line above and uncomment the line below
																																			//angle=_mm_sub_ps(angle,_mm_mul_ps(_mm_floor_ps(_mm_mul_ps(angle,_mm_set1_ps(invtwopi))),_mm_set1_ps(twopi))); //faster if SSE4.1 is always available

	__m128 cosangle = angle;
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(pi), angle))));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(pi)), SIGNMASK));
	cosangle = _mm_xor_ps(cosangle, _mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(threehalfpi)), _mm_xor_ps(cosangle, _mm_sub_ps(_mm_set1_ps(twopi), angle))));

	__m128 result = FTA::cos_52s_ps(cosangle);

	result = _mm_xor_ps(result, _mm_and_ps(_mm_and_ps(_mm_cmpge_ps(angle, _mm_set1_ps(halfpi)), _mm_cmplt_ps(angle, _mm_set1_ps(threehalfpi))), SIGNMASK));
	*cos = result;

	__m128 sinmultiplier = _mm_mul_ps(anglesign, _mm_or_ps(_mm_set1_ps(1.f), _mm_and_ps(_mm_cmpgt_ps(angle, _mm_set1_ps(pi)), SIGNMASK)));
	*sin = _mm_mul_ps(sinmultiplier, FT::sqrt_ps(_mm_sub_ps(_mm_set1_ps(1.f), _mm_mul_ps(result, result))));

	return;
}

void FTA::interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3)
{
	*x0y0x1y1 = _mm_unpacklo_ps(x0x1x2x3, y0y1y2y3);
	*x2y2x3y3 = _mm_unpackhi_ps(x0x1x2x3, y0y1y2y3);
}
void FTA::deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3)
{
	*x0x1x2x3 = _mm_shuffle_ps(x0y0x1y1, x2y2x3y3, _MM_SHUFFLE(2, 0, 2, 0));
	*y0y1y2y3 = _mm_shuffle_ps(x0y0x1y1, x2y2x3y3, _MM_SHUFFLE(3, 1, 3, 1));
}

//QT
#ifdef QT_GUI_LIB
float FTA::length(QVector2D vector) {
	return FTA::length(vector.x(), vector.y());
}
float FTA::length(QVector3D vector) {
	return FTA::length(vector.x(), vector.y(), vector.z());
}
float FTA::angle(QVector2D vector) {
	return FTA::atan2(vector.y(), vector.x());
}
float FTA::azimuth(QVector3D vector) {
	return FTA::atan2(vector.y(), vector.x());
}
float FTA::inclination(QVector3D vector) {
	float projectedlength = FTA::length(vector.toVector2D());
	return FTA::atan2(vector.z(), projectedlength);
}
float FTA::x(QVector2D vector) {
	return vector.x()*FTA::cos(vector.y());
}
float FTA::y(QVector2D vector) {
	return vector.x()*FTA::sin(vector.y());
}
float FTA::x(QVector3D vector) {
	return vector.x()*FTA::cos(vector.y())*FTA::cos(vector.z());
}
float FTA::y(QVector3D vector) {
	return vector.x()*FTA::sin(vector.y())*FTA::cos(vector.z());
}
float FTA::z(QVector3D vector) {
	return vector.x()*FTA::sin(vector.z());
}
QVector2D FTA::cartesian2polar(QVector2D vector) {
	return QVector2D(FTA::length(vector), FTA::angle(vector));
}
QVector2D FTA::polar2cartesian(QVector2D vector) {
	float sin, cos;
	FTA::sincos(vector.y(), &sin, &cos);
	return QVector2D(vector.x()*cos,
		vector.x()*sin);
}
QVector3D FTA::cartesian2spherical(QVector3D vector) {
	return QVector3D(FTA::length(vector), FTA::azimuth(vector), FTA::inclination(vector));
}
QVector3D FTA::spherical2cartesian(QVector3D vector) {
	float azimuthsin, azimuthcos;
	FTA::sincos(vector.y(), &azimuthsin, &azimuthcos);
	float inclinationsin, inclinationcos;
	FTA::sincos(vector.z(), &inclinationsin, &inclinationcos);
	return QVector3D(vector.x()*azimuthcos*inclinationcos,
		vector.x()*azimuthsin*inclinationcos,
		vector.x()*inclinationsin);
}
#endif

```

## Source file: Lux_Renderer_For_Massive_Model/fasttrigo.h

{
  "path": "Lux_Renderer_For_Massive_Model/fasttrigo.h",
  "blob_sha": "3471ab83674176af3f45d0bf8b4717816d8f9385",
  "sha256": "7bf434849269ad991f30bf23e05e4e1ed691421ca226c6fa443030c65cca456e",
  "lines": 127,
  "bytes": 5110,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 6
  },
  "url": "https://github.com/AlerianEmperor/LuxRenderer/blob/2f9a73d17f6b1e83ed8280eb2345bc813437d937/Lux_Renderer_For_Massive_Model/fasttrigo.h",
  "function_signature_candidates": [
    {
      "line": 52,
      "text": "float sqrt(float sqrd);"
    },
    {
      "line": 53,
      "text": "float length(float x, float y);"
    },
    {
      "line": 54,
      "text": "float length(float x, float y, float z);"
    },
    {
      "line": 55,
      "text": "float atan2(float y, float x);"
    },
    {
      "line": 56,
      "text": "float cos(float angle);"
    },
    {
      "line": 57,
      "text": "float sin(float angle);"
    },
    {
      "line": 58,
      "text": "void  sincos(float& angle, float *sin, float *cos);"
    },
    {
      "line": 66,
      "text": "void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);"
    },
    {
      "line": 67,
      "text": "void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);"
    },
    {
      "line": 68,
      "text": "void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);"
    },
    {
      "line": 71,
      "text": "float length(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 72,
      "text": "float length(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 73,
      "text": "float angle(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 74,
      "text": "float azimuth(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 75,
      "text": "float inclination(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 76,
      "text": "float x(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 77,
      "text": "float y(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 78,
      "text": "float x(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 79,
      "text": "float y(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 80,
      "text": "float z(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 91,
      "text": "float sqrt(float sqrd);"
    },
    {
      "line": 92,
      "text": "float length(float x, float y);"
    },
    {
      "line": 93,
      "text": "float length(float x, float y, float z);"
    },
    {
      "line": 94,
      "text": "float atan2(float y, float x);"
    },
    {
      "line": 95,
      "text": "float cos(float angle);"
    },
    {
      "line": 96,
      "text": "float sin(float angle);"
    },
    {
      "line": 97,
      "text": "void  sincos(float angle, float *sin, float *cos);"
    },
    {
      "line": 105,
      "text": "void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);"
    },
    {
      "line": 106,
      "text": "void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);"
    },
    {
      "line": 107,
      "text": "void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);"
    },
    {
      "line": 110,
      "text": "float length(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 111,
      "text": "float length(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 112,
      "text": "float angle(QVector2D vector); //cartesian vector(x,y)"
    },
    {
      "line": 113,
      "text": "float azimuth(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 114,
      "text": "float inclination(QVector3D vector); //cartesian vector(x,y,z)"
    },
    {
      "line": 115,
      "text": "float x(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 116,
      "text": "float y(QVector2D vector); //polar vector(length,angle)"
    },
    {
      "line": 117,
      "text": "float x(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 118,
      "text": "float y(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    },
    {
      "line": 119,
      "text": "float z(QVector3D vector); //spherical vector(length,azimuth,inclination)"
    }
  ]
}

```text

/* FastTrigo 1.0 (c) 2013 Robin Lobel
Fast yet accurate trigonometric functions
Each namespace (FT, FTA) has 3 sets of functions:
Scalar: standard trigonometric functions
Packed Scalar: same functions computing 4 values at the same time (using SSE/SSE2/SSE3/SSE4.1 if available)
Qt: convenience functions if using QVector2D/QVector3D classes from Qt
FT Accuracy:
FT::sqrt/sqrt_ps max error: 0.032% (average error: 0.0094%)
FT::atan2/atan2_ps max error: 0.024% (0.0015 radians, 0.086 degrees)
FT::cos/cos_ps max error: 0.06%
FT::sin/sin_ps max error: 0.06%
FT Speed up (MSVC2012 x64):
FT::sqrt speed up: x2.5 (from standard sqrt)
FT::atan2 speed up: x2.3 (from standard atan2)
FT::sin/cos speed up: x1.9 (from standard sin/cos)
FT::sincos speed up: x2.3 (from standard sin+cos)
FT::sqrt_ps speed up: x8 (from standard sqrt)
FT::atan2_ps speed up: x7.3 (from standard atan2)
FT::sin_ps/cos_ps speed up: x4.9 (from standard sin/cos)
FT::sincos_ps speed up: x6.2 (from standard sin+cos)
FTA Accuracy:
FTA::sqrt/sqrt_ps max error: 0%
FTA::atan2/atan2_ps max error: 0.0005%
FTA::cos/cos_ps max error: 0.0007%
FTA::sin/sin_ps max error: 0.0007%
FTA Speed up (MSVC2012 x64):
FTA::sqrt speed up: x1.5 (from standard sqrt)
FTA::atan2 speed up: x1.7 (from standard atan2)
FTA::sin/cos speed up: x1.6 (from standard sin/cos)
FTA::sincos speed up: x1.8 (from standard sin+cos)
FTA::sqrt_ps speed up: x4.9 (from standard sqrt)
FTA::atan2_ps speed up: x5.2 (from standard atan2)
FTA::sin_ps/cos_ps speed up: x4.3 (from standard sin/cos)
FTA::sincos_ps speed up: x5.2 (from standard sin+cos)
Distributed under Revised BSD License
*/

#ifndef FASTTRIGO_H
#define FASTTRIGO_H

#include <math.h>
#ifdef QT_GUI_LIB
#include <QtGui>
#endif
#include <intrin.h>
#include <xmmintrin.h>
#include <pmmintrin.h>

//Default accuracy
namespace FT
{
	float sqrt(float sqrd);
	float length(float x, float y);
	float length(float x, float y, float z);
	float atan2(float y, float x);
	float cos(float angle);
	float sin(float angle);
	void  sincos(float& angle, float *sin, float *cos);

	__m128 sqrt_ps(__m128 sqrd);
	__m128 length_ps(__m128 x, __m128 y);
	__m128 length_ps(__m128 x, __m128 y, __m128 z);
	__m128 atan2_ps(__m128 y, __m128 x);
	__m128 cos_ps(__m128 angle);
	__m128 sin_ps(__m128 angle);
	void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);
	void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);
	void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);

#ifdef QT_GUI_LIB
	float length(QVector2D vector); //cartesian vector(x,y)
	float length(QVector3D vector); //cartesian vector(x,y,z)
	float angle(QVector2D vector); //cartesian vector(x,y)
	float azimuth(QVector3D vector); //cartesian vector(x,y,z)
	float inclination(QVector3D vector); //cartesian vector(x,y,z)
	float x(QVector2D vector); //polar vector(length,angle)
	float y(QVector2D vector); //polar vector(length,angle)
	float x(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float y(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float z(QVector3D vector); //spherical vector(length,azimuth,inclination)
	QVector2D cartesian2polar(QVector2D vector);
	QVector2D polar2cartesian(QVector2D vector);
	QVector3D cartesian2spherical(QVector3D vector);
	QVector3D spherical2cartesian(QVector3D vector);
#endif
};

//More accurate
namespace FTA
{
	float sqrt(float sqrd);
	float length(float x, float y);
	float length(float x, float y, float z);
	float atan2(float y, float x);
	float cos(float angle);
	float sin(float angle);
	void  sincos(float angle, float *sin, float *cos);

	__m128 sqrt_ps(__m128 sqrd);
	__m128 length_ps(__m128 x, __m128 y);
	__m128 length_ps(__m128 x, __m128 y, __m128 z);
	__m128 atan2_ps(__m128 y, __m128 x);
	__m128 cos_ps(__m128 angle);
	__m128 sin_ps(__m128 angle);
	void   sincos_ps(__m128 angle, __m128 *sin, __m128 *cos);
	void   interleave_ps(__m128 x0x1x2x3, __m128 y0y1y2y3, __m128 *x0y0x1y1, __m128 *x2y2x3y3);
	void   deinterleave_ps(__m128 x0y0x1y1, __m128 x2y2x3y3, __m128 *x0x1x2x3, __m128 *y0y1y2y3);

#ifdef QT_GUI_LIB
	float length(QVector2D vector); //cartesian vector(x,y)
	float length(QVector3D vector); //cartesian vector(x,y,z)
	float angle(QVector2D vector); //cartesian vector(x,y)
	float azimuth(QVector3D vector); //cartesian vector(x,y,z)
	float inclination(QVector3D vector); //cartesian vector(x,y,z)
	float x(QVector2D vector); //polar vector(length,angle)
	float y(QVector2D vector); //polar vector(length,angle)
	float x(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float y(QVector3D vector); //spherical vector(length,azimuth,inclination)
	float z(QVector3D vector); //spherical vector(length,azimuth,inclination)
	QVector2D cartesian2polar(QVector2D vector);
	QVector2D polar2cartesian(QVector2D vector);
	QVector3D cartesian2spherical(QVector3D vector);
	QVector3D spherical2cartesian(QVector3D vector);
#endif
};

#endif // FASTTRIGO_H

```

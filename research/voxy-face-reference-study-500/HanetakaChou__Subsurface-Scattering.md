# Facial rendering source scan: HanetakaChou/Subsurface-Scattering

Repository: https://github.com/HanetakaChou/Subsurface-Scattering

Pinned source revision: b4debf1d277a555d8261ef7f474a8610bf43e1b5

Discovery topic: subsurface scattering

Description: Subsurface Scattering

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"subsurface": ["Shaders/subsurface_scattering_disney_blur.hlsli", "Shaders/subsurface_scattering_disney_transmittance.hlsli", "Shaders/subsurface_scattering_texturing_mode.hlsli", "Subsurface-Scattering-OFF.png", "Subsurface-Scattering-ON.png"], "sss": ["Code/SSSBlur.cpp", "Code/SSSBlur.h", "SeparableSSS-LICENSE.txt", "SeparableSSS-README.md", "Shaders/Support/SSS_Blur.hlsli", "Shaders/Support/SSS_Blur_PS.hlsl", "Shaders/Support/SSS_Blur_VS.hlsl"], "shader": ["Shaders/Support/FilmGrain.hlsli", "Shaders/Support/FilmGrain_FilmGrainPS.hlsl", "Shaders/Support/FilmGrain_PassVS.hlsl", "Shaders/Support/Main.hlsli", "Shaders/Support/Main_RenderPS.hlsl", "Shaders/Support/Main_RenderVS.hlsl", "Shaders/Support/SSS_Blur.hlsli", "Shaders/Support/SSS_Blur_PS.hlsl", "Shaders/Support/SSS_Blur_VS.hlsl", "Shaders/Support/ShadowMap.hlsli", "Shaders/Support/ShadowMap_ShadowMapVS.hlsl", "Shaders/Support/SkyDome.hlsli", "Shaders/Support/SkyDome_SkyDomePS.hlsl", "Shaders/Support/SkyDome_SkyDomeVS.hlsl", "Shaders/brdf.hlsli", "Shaders/low_discrepancy_sequence.hlsli", "Shaders/math_consts.hlsli", "Shaders/subsurface_scattering_disney_blur.hlsli", "Shaders/subsurface_scattering_disney_transmittance.hlsli", "Shaders/subsurface_scattering_texturing_mode.hlsli"], "face": ["Shaders/subsurface_scattering_disney_blur.hlsli", "Shaders/subsurface_scattering_disney_transmittance.hlsli", "Shaders/subsurface_scattering_texturing_mode.hlsli", "Subsurface-Scattering-OFF.png", "Subsurface-Scattering-ON.png"]}

## Source file: Shaders/subsurface_scattering_disney_blur.hlsli

{
  "path": "Shaders/subsurface_scattering_disney_blur.hlsli",
  "blob_sha": "abc838160fc50d99d2177e9af815c328208878e3",
  "sha256": "9e904b8336031dd7676bab63c4942a3d22899f1855d00708d5778d587f308e99",
  "lines": 253,
  "bytes": 12202,
  "selection_score": 45,
  "keyword_line_counts": {
    "skin": 2,
    "subsurface": 5,
    "sss": 20,
    "rig": 2,
    "face": 6
  },
  "url": "https://github.com/HanetakaChou/Subsurface-Scattering/blob/b4debf1d277a555d8261ef7f474a8610bf43e1b5/Shaders/subsurface_scattering_disney_blur.hlsli",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "float diffusion_profile_sample_r(float d, float cdf);"
    },
    {
      "line": 45,
      "text": "float diffusion_profile_evaluate_cdf(float d, float r);"
    },
    {
      "line": 46,
      "text": "float diffusion_profile_evaluate_rcp_pdf(float d, float r);"
    },
    {
      "line": 47,
      "text": "float3 diffusion_profile_evaluate_pdf(float3 S, float r);"
    },
    {
      "line": 49,
      "text": "float3 subsurface_scattering_disney_blur(const float3 scattering_distance, const float world_scale, const int pixels_per_sample, const int sample_budget, const float2 center_uv)"
    },
    {
      "line": 189,
      "text": "float diffusion_profile_sample_r(float d, float cdf)"
    },
    {
      "line": 212,
      "text": "float diffusion_profile_evaluate_cdf(float d, float r)"
    },
    {
      "line": 225,
      "text": "float diffusion_profile_evaluate_rcp_pdf(float d, float r)"
    },
    {
      "line": 242,
      "text": "float3 diffusion_profile_evaluate_pdf(float3 S, float r)"
    }
  ]
}

```text

//
// Copyright (C) YuqiaoZhang(HanetakaChou)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//

// Note: Provided by the User!
// 
// SSS_ALBEDO_SOURCE_POINT
// 
// "Combining Pre-Scatter and Post-Scatter Texturing" of [Eugene dEon, David Luebke. "Advanced Techniques for Realistic Real-Time Skin Rendering." GPU Gem 3.](https://developer.nvidia.com/gpugems/gpugems3/part-iii-rendering/chapter-14-advanced-techniques-realistic-real-time-skin)
// Pre- and Post-Scatter Texturing Mode - sqrt(albedo)
// 
// "Post-Scatter Texturing" of [Eugene dEon, David Luebke. "Advanced Techniques for Realistic Real-Time Skin Rendering." GPU Gem 3.](https://developer.nvidia.com/gpugems/gpugems3/part-iii-rendering/chapter-14-advanced-techniques-realistic-real-time-skin)
// Post-Scatter Texturing Mode - albedo
//

#ifndef _SUBSURFACE_SCATTERING_DISNEY_BLUR_HLSLI_
#define _SUBSURFACE_SCATTERING_DISNEY_BLUR_HLSLI_ 1

#include "math_consts.hlsli"
#include "low_discrepancy_sequence.hlsli"

#define SSS_MIN_PIXELS_PER_SAMPLE 4
#define SSS_MAX_SAMPLE_BUDGET 80

// https://zero-radiance.github.io/post/sampling-diffusion/
// S = ShapeParam = 1 / ScatteringDistance = 1 / d
// d = ScatteringDistance = 1 / ShapeParam = 1 / S
// CDF is the random number (the value of the CDF): [0, 1).
// r is the sampled radial distance, s.t. (CDF = 0 -> r = 0) and (CDF = 1 -> r = Inf).
// PDF is the derivative of the (marginal) CDF. Actually, '2 * PI' should be multiplied to convert the normalized diffusion profile to 1D, namely, PDF = (DiffusionProfile * r / A) * (2 * PI).
float diffusion_profile_sample_r(float d, float cdf);
float diffusion_profile_evaluate_cdf(float d, float r);
float diffusion_profile_evaluate_rcp_pdf(float d, float r);
float3 diffusion_profile_evaluate_pdf(float3 S, float r);

float3 subsurface_scattering_disney_blur(const float3 scattering_distance, const float world_scale, const int pixels_per_sample, const int sample_budget, const float2 center_uv)
{
	const float dist_scale = SSS_SUBSURFACE_MASK_SOURCE(center_uv);
	// Early Out
	[branch]
	if (dist_scale < (1.0 / 255.0))
	{
		float3 total_diffuse_reflectance_pre_scatter_multiply_form_factor = SSS_TOTAL_DIFFUSE_REFLECTANCE_PRE_SCATTER_MULTIPLY_FORM_FACTOR_SOURCE(center_uv);

		float3 total_diffuse_reflectance_post_scatter = SSS_TOTAL_DIFFUSE_REFLECTANCE_POST_SCATTER_SOURCE(center_uv);
		
		float3 radiance = total_diffuse_reflectance_post_scatter * total_diffuse_reflectance_pre_scatter_multiply_form_factor;
		return radiance;
	}

	// In UE4, the "dist_scale" is applied to the "scattering_distance" rather than the "uv_per_mm".  
	// This is equivalent since the "r" evaluated by the "diffusion_profile_sample_r" is proportional to the "scattering_distance".
	const float meters_per_unit = world_scale;
#if 0
	// Unity3D
	float3 center_view_space_position = SSS_VIEW_SPACE_POSITION_SOURCE(center_uv);
	const float center_view_space_position_z = center_view_space_position.z;
	float3 corner_view_space_position = SSS_VIEW_SPACE_POSITION_SOURCE(center_uv + 0.5 * (float2(1.0, 1.0) / SSS_PIXELS_PER_UV()));
	float2 units_per_pixel = max(float2(0.0001, 0.0001), 2.0 * abs(float2(corner_view_space_position.x - center_view_space_position.x, corner_view_space_position.y - center_view_space_position.y)));
	const float mms_per_unit = 1000.0 * meters_per_unit * (1.0 / dist_scale);
	const float2 pixels_per_mm = (1.0 / units_per_pixel) * (1.0 / mms_per_unit);
	const float2 uv_per_mm = pixels_per_mm * (float2(1.0, 1.0) / SSS_PIXELS_PER_UV());
#else
	// UE4
	const float center_view_space_position_z = SSS_VIEW_SPACE_POSITION_Z_SOURCE(center_uv);
	const float mms_per_unit = 1000.0 * meters_per_unit * (1.0 / dist_scale);
	const float2 uv_per_mm = 0.5 * float2(SSS_PROJECTION_X_SOURCE(), SSS_PROJECTION_Y_SOURCE()) * (1.0 / center_view_space_position_z) * (1.0 / mms_per_unit);
	const float2 pixels_per_mm = SSS_PIXELS_PER_UV() * uv_per_mm;
#endif

	// Unity3D <=> UE4
	// ScatteringDistance = MeanFreePathColor * MeanFreePathDistance / GetScalingFactor(SurfaceAlbedo)
	const float3 S = float3(1.0, 1.0, 1.0) / scattering_distance.rgb;
	const float d = max(max(scattering_distance.r, scattering_distance.g), scattering_distance.b);

	// Center Sample Reweighting
	//
	// The original burley algorithm involes monte car sampling. Given a random variable [0,1],
	// find the distance of that point from the center using the CDF, and then divide by PDF.
	// But it is somewhat inefficient because it is weighted heavily towards the center.
	//
	// Instead, we are going to split the [0,1] random variable range. First, we figure out the
	// radius (R) of the center sample in world space. Second, we are going to determine the random
	// variable (T) such that CDF(R) = T. Then we split the range into two segments.
	//
	// 1. The center sample, which include the random variable values from [0,T].
	// 2. All other samples, which include the random variable values from [T,1].
	//
	// With the center sample is scaled the weight T and the rest of the samples are weighted
	// by (1-T). There shouldn't be any bias, except for small errors due to precision.
	const float center_sample_radius_in_mm = 0.5 * (1.0 / pixels_per_mm.x + 1.0 / pixels_per_mm.y);
	const float center_sample_cdf = diffusion_profile_evaluate_cdf(d, center_sample_radius_in_mm);

	float3 sum_numerator = float3(0.0, 0.0, 0.0);
	float3 sum_denominator = float3(0.0, 0.0, 0.0);

	// Filter radius is, strictly speaking, infinite.
	// The magnitude of the function decays exponentially, but it is never truly zero.
	// To estimate the radius, we can use adapt the "three-sigma rule" by defining
	// the radius of the kernel by the value of the CDF which corresponds to 99.7%
	// of the energy of the filter.
	const float filter_radius = diffusion_profile_sample_r(d, 0.997);
	float sample_count = int(float(PI) * (filter_radius * pixels_per_mm.x) * (filter_radius * pixels_per_mm.y) * (1.0 / float(max(pixels_per_sample, int(SSS_MIN_PIXELS_PER_SAMPLE)))));
	sample_count = min(sample_count, min(sample_budget, int(SSS_MAX_SAMPLE_BUDGET)));

	[loop]
	for (int sample_index = 0; sample_index < int(SSS_MAX_SAMPLE_BUDGET) && sample_index < sample_count; ++sample_index)
	{
		float2 xi = hammersley_2d(sample_index, sample_count);

		// Center Sample Reweighting
		xi.x = lerp(center_sample_cdf, 1.0, xi.x);

		// Sampling Diffusion Profile
		float r = diffusion_profile_sample_r(d, xi.x);
		float theta = 2.0 * float(PI) * xi.y;

		// Bilateral Filter
		float2 sample_uv = center_uv + uv_per_mm * float2(cos(theta), sin(theta)) * r;

		// The "sample_form_factor" may be zero even if the "sample_dist_scale" is NOT zero
		float sample_dist_scale = SSS_SUBSURFACE_MASK_SOURCE(sample_uv);

		[branch]
		if (sample_dist_scale >= (1.0 / 255.0))
		{
			float3 sample_total_diffuse_reflectance_pre_scatter_multiply_form_factor = SSS_TOTAL_DIFFUSE_REFLECTANCE_PRE_SCATTER_MULTIPLY_FORM_FACTOR_SOURCE(sample_uv);

			float rcp_pdf = diffusion_profile_evaluate_rcp_pdf(d, r);

			// Bilateral Filter
			float sample_view_space_position_z = SSS_VIEW_SPACE_POSITION_Z_SOURCE(sample_uv);
			float relative_position_z_mm = mms_per_unit * (sample_view_space_position_z - center_view_space_position_z);
			float r_bilateral_weight = sqrt(r * r + relative_position_z_mm * relative_position_z_mm);

			float3 pdf = diffusion_profile_evaluate_pdf(S, r_bilateral_weight);

			// normalized_diffusion_profile = pdf / r
			// (1.0 / float(N)) * total_diffuse_reflectance * (pdf / r) * form_factor * r * rcp_pdf 
			// = (1.0 / float(N)) * total_diffuse_reflectance * pdf * form_factor * rcp_pdf
			// = (1.0 / float(N)) * total_diffuse_reflectance_post_scatter * pdf * (total_diffuse_reflectance_pre_scatter * form_factor) * rcp_pdf
			// the 'total_diffuse_reflectance_post_scatter' will be calculated later while the 'total_diffuse_reflectance_pre_scatter * form_factor' is calculated here 
			float3 sample_numerator = pdf * sample_total_diffuse_reflectance_pre_scatter_multiply_form_factor * rcp_pdf;

			// (1.0 / float(N)) * pdf * rcp_pdf
			float3 sample_denominator = pdf * rcp_pdf;

			sum_numerator += sample_numerator;

			// assumed to be N since the pdf is normalized and the common divisor '1.0 / float(N)' is reduced
			sum_denominator += sample_denominator;
		}
	}

	// Center Sample Reweighting
	float3 sum_total_diffuse_reflectance_pre_scatter_multiply_form_factor = sum_numerator / max(sum_denominator, float3(FLT_MIN, FLT_MIN, FLT_MIN));
	float3 center_total_diffuse_reflectance_pre_scatter_multiply_form_factor = SSS_TOTAL_DIFFUSE_REFLECTANCE_PRE_SCATTER_MULTIPLY_FORM_FACTOR_SOURCE(center_uv);
	float3 total_diffuse_reflectance_pre_scatter_multiply_form_factor = lerp(sum_total_diffuse_reflectance_pre_scatter_multiply_form_factor, center_total_diffuse_reflectance_pre_scatter_multiply_form_factor, center_sample_cdf);

	float3 total_diffuse_reflectance_post_scatter = SSS_TOTAL_DIFFUSE_REFLECTANCE_POST_SCATTER_SOURCE(center_uv);
	
	float3 radiance = total_diffuse_reflectance_post_scatter * total_diffuse_reflectance_pre_scatter_multiply_form_factor;
	return radiance;
}

////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
//
//    IMPLEMENTATION
//
////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////

#define LOG2_E 1.44269504088896340736

float diffusion_profile_sample_r(float d, float cdf)
{
	float u = 1 - cdf; // Convert CDF to CCDF

	float g = 1 + (4 * u) * (2 * u + sqrt(1 + (4 * u) * u));

	// g^(-1/3)
	float n = exp2(log2(g) * (-1.0 / 3.0));
	// g^(+1/3)
	float p = (g * n) * n;
	// 1 + g^(+1/3) + g^(-1/3)
	float c = 1 + p + n;
	// 3 * Log[4 * u]
	float b = (3 / LOG2_E * 2) + (3 / LOG2_E) * log2(u);
	// 3 * Log[c / (4 * u)]
	float x = (3 / LOG2_E) * log2(c) - b;

	// x = S * r
	// r = x * rcpS = x * d
	float r = x * d;
	return r;
}

float diffusion_profile_evaluate_cdf(float d, float r)
{
	// x = S * r = (1.0 / d) * r
	// exp_13 = Exp[-x/3]
	float exp_13 = exp2(((LOG2_E * (-1.0 / 3.0)) * r) * (1.0 / d));
	// exp_1  = Exp[-x] = exp_13 * exp_13 * exp_13
	// exp_sum = -0.25 * exp_1 - 0.75 * exp_13 =  exp_13 * (-0.75 - 0.25 * exp_13 * exp_13)
	float exp_sum = exp_13 * (-0.75 - 0.25 * exp_13 * exp_13);
	// 1 - 0.75 * Exp[-S * r / 3]  - 0.25 * Exp[-S * r])
	float cdf = 1.0 + exp_sum;
	return cdf;
}

float diffusion_profile_evaluate_rcp_pdf(float d, float r)
{
	// x = S * r = (1.0 / d) * r
	// exp_13 = Exp[-x/3]
	float exp_13 = exp2(((LOG2_E * (-1.0 / 3.0)) * r) * (1.0 / d));
	// exp_1  = Exp[-x] = exp_13 * exp_13 * exp_13
	// exp_sum = exp_1 + exp_13 =  exp_13 * (1 + exp_13 * exp_13)
	float exp_sum = exp_13 * (1 + exp_13 * exp_13);
	// rcpExp = (1.0 / exp_sum)
	float rcpExp = (1.0 / exp_sum);

	// rcpS = d
	// (8 * PI) / S / (Exp[-S * r / 3] + Exp[-S * r])
	float rcp_pdf = (8.0 * PI) * d * rcpExp;
	return rcp_pdf;
}

float3 diffusion_profile_evaluate_pdf(float3 S, float r)
{
	// Exp[-s * r / 3]
	float3 exp_13 = exp2(((LOG2_E * (-1.0 / 3.0)) * r) * S);
	// Exp[-s * r / 3] + Exp[-S * r]
	float3 exp_sum = exp_13 * (1 + exp_13 * exp_13);
	// S / (8 * PI) * (Exp[-S * r / 3] + Exp[-S * r])
	float3 pdf = S / (8.0 * PI) * exp_sum;
	return pdf;
}

#endif

```

## Source file: Shaders/subsurface_scattering_texturing_mode.hlsli

{
  "path": "Shaders/subsurface_scattering_texturing_mode.hlsli",
  "blob_sha": "0ff364515513fec1804cea64897f221b2affdc3c",
  "sha256": "6cedadf907df8250ec1c14e044a8543e322b51be76fc54d04cbf4fb947bc9d4e",
  "lines": 47,
  "bytes": 1365,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 4,
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/HanetakaChou/Subsurface-Scattering/blob/b4debf1d277a555d8261ef7f474a8610bf43e1b5/Shaders/subsurface_scattering_texturing_mode.hlsli",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "float3 subsurface_scattering_total_diffuse_reflectance_pre_scatter_from_albedo(bool is_post_scatter_texturing_mode, float3 albedo)"
    },
    {
      "line": 34,
      "text": "float3 subsurface_scattering_total_diffuse_reflectance_post_scatter_from_albedo(bool is_post_scatter_texturing_mode, float3 albedo)"
    }
  ]
}

```text

//
// Copyright (C) YuqiaoZhang(HanetakaChou)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//

#ifndef _SUBSURFACE_SCATTERING_TEXTURING_MODE_HLSLI_
#define _SUBSURFACE_SCATTERING_TEXTURING_MODE_HLSLI_ 1

float3 subsurface_scattering_total_diffuse_reflectance_pre_scatter_from_albedo(bool is_post_scatter_texturing_mode, float3 albedo)
{
	[branch]
	if (!is_post_scatter_texturing_mode)
	{
		return sqrt(albedo);
	}
	else
	{
		return 1.0F;
	}
}

float3 subsurface_scattering_total_diffuse_reflectance_post_scatter_from_albedo(bool is_post_scatter_texturing_mode, float3 albedo)
{
	[branch]
	if (!is_post_scatter_texturing_mode)
	{
		return sqrt(albedo);
	}
	else
	{
		return albedo;
	}
}

#endif

```

## Source file: Shaders/subsurface_scattering_disney_transmittance.hlsli

{
  "path": "Shaders/subsurface_scattering_disney_transmittance.hlsli",
  "blob_sha": "3194430672f13dd3077b8fcd47eea2a6ee39d8c0",
  "sha256": "cdab643d3765bae6e96bb856980d2c36521d8770a1d8be811d0da69a7f6854f1",
  "lines": 49,
  "bytes": 2052,
  "selection_score": 45,
  "keyword_line_counts": {
    "subsurface": 4,
    "sss": 2,
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/HanetakaChou/Subsurface-Scattering/blob/b4debf1d277a555d8261ef7f474a8610bf43e1b5/Shaders/subsurface_scattering_disney_transmittance.hlsli",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "float3 EvaluateTransmittance(float3 S, float thickness)"
    },
    {
      "line": 41,
      "text": "float EvaluateTransmittanceLightAttenuation(float3 N, float3 L)"
    }
  ]
}

```text

//
// Copyright (C) YuqiaoZhang(HanetakaChou)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//

#ifndef _SUBSURFACE_SCATTERING_DISNEY_TRANSMITTANCE_HLSLI_
#define _SUBSURFACE_SCATTERING_DISNEY_TRANSMITTANCE_HLSLI_ 1

#define LOG2_E 1.44269504088896340736

// Computes the fraction of light passing through the object.
// Evaluate Int{0, inf}{2 * Pi * r * R(sqrt(r^2 + d^2))}, where R is the diffusion profile.
// Ref: Approximate Reflectance Profiles for Efficient Subsurface Scattering by Pixar (BSSRDF only).
float3 EvaluateTransmittance(float3 S, float thickness)
{
    // The transmittance coefficient T(d) is precalculated based on diffusion profile.

    // Thickness and SSS mask are decoupled for artists.
    // In theory, we should modify the thickness by the inverse of the mask scale of the profile.
    // thickness /= subsurfaceMask;

    float3 exp_13 = exp2(((LOG2_E * (-1.0 / 3.0)) * thickness) * S); // Exp[-S * t / 3]

    // T = (1/4 * A) * (e^(-S * t) + 3 * e^(-S * t / 3))
    // the 'A' is multiplied in the 'full screen sss blur pass'
    return 0.25 * (exp_13 * (exp_13 * exp_13 + 3));
}

float EvaluateTransmittanceLightAttenuation(float3 N, float3 L)
{
    // Using the transmittance coefficient T(d), we finally approximate the transmitted lighting from the back of the object
    
    float NoL = saturate(0.3 + dot(-N, L));
    return NoL;
}

#endif

```

## Source file: Shaders/Support/SSS_Blur.hlsli

{
  "path": "Shaders/Support/SSS_Blur.hlsli",
  "blob_sha": "69ea92f9d6694bcaf0a051f9cb6657f14ecc57e0",
  "sha256": "3cca6f38f97956f417da8383ac1039986cba004432c322e1ff794c4889ea7c28",
  "lines": 106,
  "bytes": 3648,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 1,
    "subsurface": 5,
    "sss": 10,
    "face": 5
  },
  "url": "https://github.com/HanetakaChou/Subsurface-Scattering/blob/b4debf1d277a555d8261ef7f474a8610bf43e1b5/Shaders/Support/SSS_Blur.hlsli",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "inline float3 SSS_TOTAL_DIFFUSE_REFLECTANCE_PRE_SCATTER_MULTIPLY_FORM_FACTOR_SOURCE(float2 pixelCoord)"
    },
    {
      "line": 31,
      "text": "inline float3 SSS_TOTAL_DIFFUSE_REFLECTANCE_POST_SCATTER_SOURCE(float2 pixelCoord)"
    },
    {
      "line": 38,
      "text": "inline float SSS_SUBSURFACE_MASK_SOURCE(float2 pixelCoord)"
    },
    {
      "line": 43,
      "text": "inline float SSS_PROJECTION_X_SOURCE()"
    },
    {
      "line": 48,
      "text": "inline float SSS_PROJECTION_Y_SOURCE()"
    },
    {
      "line": 53,
      "text": "inline float2 uv_to_ndcxy(float2 uv)"
    },
    {
      "line": 58,
      "text": "inline float ndcz_to_viewpositionz(float ndcz, float4x4 proj)"
    },
    {
      "line": 63,
      "text": "inline float3 ndc_to_view(float3 ndc, float4x4 proj)"
    },
    {
      "line": 70,
      "text": "inline float SSS_VIEW_SPACE_POSITION_Z_SOURCE(float2 pixelCoord)"
    },
    {
      "line": 79,
      "text": "inline float3 SSS_VIEW_SPACE_POSITION_SOURCE(float2 pixelCoord)"
    },
    {
      "line": 86,
      "text": "inline float2 SSS_PIXELS_PER_UV()"
    },
    {
      "line": 97,
      "text": "void SSS_Blur_VS(float4 position : POSITION, out float4 svposition : SV_POSITION, inout float2 texcoord : TEXCOORD0)"
    },
    {
      "line": 102,
      "text": "float4 SSS_Blur_PS(float4 position: SV_POSITION, float2 texcoord : TEXCOORD0) : SV_TARGET"
    }
  ]
}

```text

cbuffer UpdatedPerFrame : register(b0)
{
	row_major float4x4 currProj;
	float3 scatteringDistance;
	float padding_scatteringDistance;
	float worldScale;
	float postscatterEnabled;
	int sampleBudget;
	int pixelsPerSample;
}

Texture2D g_albedo_texture : register(t0);

// NOTE: we should output zero for non-skin material
Texture2D g_total_diffuse_reflectance_pre_scatter_multiply_form_factor_texture : register(t1);

Texture2D depthTex : register(t2);

SamplerState PointSampler : register(s1);

#include "../subsurface_scattering_texturing_mode.hlsli"

inline float3 SSS_TOTAL_DIFFUSE_REFLECTANCE_PRE_SCATTER_MULTIPLY_FORM_FACTOR_SOURCE(float2 pixelCoord)
{
	// NOTE: use SampleLevel rather than Sample
	// warning X3595: gradient instruction used in a loop with varying iteration; partial derivatives may have undefined value
	float3 total_diffuse_reflectance_pre_scatter_multiply_form_factor = g_total_diffuse_reflectance_pre_scatter_multiply_form_factor_texture.SampleLevel(PointSampler, pixelCoord, 0).rgb;
	return total_diffuse_reflectance_pre_scatter_multiply_form_factor;
}

inline float3 SSS_TOTAL_DIFFUSE_REFLECTANCE_POST_SCATTER_SOURCE(float2 pixelCoord)
{
	float3 albedo = g_albedo_texture.SampleLevel(PointSampler, pixelCoord, 0).rgb;
	float3 total_diffuse_reflectance_post_scatter = subsurface_scattering_total_diffuse_reflectance_post_scatter_from_albedo((postscatterEnabled > 0.0), albedo);
	return total_diffuse_reflectance_post_scatter;
}

inline float SSS_SUBSURFACE_MASK_SOURCE(float2 pixelCoord)
{
	return g_albedo_texture.SampleLevel(PointSampler, pixelCoord, 0).a;
}

inline float SSS_PROJECTION_X_SOURCE()
{
	return currProj[0][0];
}

inline float SSS_PROJECTION_Y_SOURCE()
{
	return currProj[1][1];
}

inline float2 uv_to_ndcxy(float2 uv)
{
	return uv * float2(2.0, -2.0) + float2(-1.0, 1.0);
}

inline float ndcz_to_viewpositionz(float ndcz, float4x4 proj)
{
	return proj[3][2] / (ndcz - proj[2][2]);
}

inline float3 ndc_to_view(float3 ndc, float4x4 proj)
{
	float view_position_z = ndcz_to_viewpositionz(ndc.z, proj);
	float2 view_position_xy = ndc.xy * view_position_z / float2(proj[0][0], proj[1][1]);
	return float3(view_position_xy, view_position_z);
}

inline float SSS_VIEW_SPACE_POSITION_Z_SOURCE(float2 pixelCoord)
{
	// NOTE: use SampleLevel rather than Sample
	// warning X3595: gradient instruction used in a loop with varying iteration; partial derivatives may have undefined value
	float depth = depthTex.SampleLevel(PointSampler, pixelCoord, 0).r;
	float view_position_z = ndcz_to_viewpositionz(depth, currProj);
	return view_position_z;
}

inline float3 SSS_VIEW_SPACE_POSITION_SOURCE(float2 pixelCoord)
{
	float depth = depthTex.SampleLevel(PointSampler, pixelCoord, 0).r;
	float3 view_position = ndc_to_view(float3(uv_to_ndcxy(pixelCoord), depth), currProj);
	return view_position;
}

inline float2 SSS_PIXELS_PER_UV()
{
	float outWidth;
	float outHeight;
	float outNumberOfLevels;
	g_total_diffuse_reflectance_pre_scatter_multiply_form_factor_texture.GetDimensions(0, outWidth, outHeight, outNumberOfLevels);
	return float2(outWidth, outHeight);
}

#include "../subsurface_scattering_disney_blur.hlsli"

void SSS_Blur_VS(float4 position : POSITION, out float4 svposition : SV_POSITION, inout float2 texcoord : TEXCOORD0)
{
	svposition = position;
}

float4 SSS_Blur_PS(float4 position: SV_POSITION, float2 texcoord : TEXCOORD0) : SV_TARGET
{
	float3 color = subsurface_scattering_disney_blur(scatteringDistance, worldScale, pixelsPerSample, sampleBudget, texcoord);
	return float4(color, 1.0);
}


```

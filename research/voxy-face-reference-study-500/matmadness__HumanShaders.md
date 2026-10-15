# Facial rendering source scan: matmadness/HumanShaders

Repository: https://github.com/matmadness/HumanShaders

Pinned source revision: 95a54cff4a9165993914143ca9047a5f5d029bce

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["EyeTest.tscn", "HumanShaders/Resources/Eye/eye_iris_diff.png", "HumanShaders/Resources/Eye/eye_iris_diff.png.import", "HumanShaders/Resources/Eye/eye_iris_nrm.png", "HumanShaders/Resources/Eye/eye_iris_nrm.png.import", "HumanShaders/Resources/Eye/eye_iris_parallax.png", "HumanShaders/Resources/Eye/eye_iris_parallax.png.import", "HumanShaders/Resources/Eye/eye_sclera_diff.png", "HumanShaders/Resources/Eye/eye_sclera_diff.png.import", "HumanShaders/Resources/Eye/eye_sclera_nrm.png", "HumanShaders/Resources/Eye/eye_sclera_nrm.png.import", "HumanShaders/eye_shader.gdshader", "HumanShaders/eye_shader.gdshader.uid", "HumanShaders/vshader_eye_wetness.tres", "Models/Eye/realistic_eye.glb", "Models/Eye/realistic_eye.glb.import", "Models/HumanHead/EmilyHeadScan/Emili_Head_Optimized_eye_iris_depth_2.png", "Models/HumanHead/EmilyHeadScan/Emili_Head_Optimized_eye_iris_depth_2.png.import", "Models/HumanHead/EmilyHeadScan/Materials/mat_emily_eyelashes.tres", "Models/HumanHead/EmilyHeadScan/Materials/smat_emily_eye.tres", "Models/HumanHead/EmilyHeadScan/Materials/vmat_generic_eye_wetness.tres"], "skin": ["HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao.png.import", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_2.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_2.png.import", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_3.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_3.png.import", "HumanShaders/Resources/lightwarp_skin1.tres", "HumanShaders/skin_shader.gdshader", "HumanShaders/skin_shader.gdshader.uid", "Models/HumanHead/EmilyHeadScan/Materials/mat_emilu_skin_standardmat.tres", "Models/HumanHead/EmilyHeadScan/Materials/smat_emily_skin.tres", "Models/HumanHead/InfiniteScan/Materials/smat_infinite_skin.tres", "SkinShaderTest.tscn"], "sss": ["Models/HumanHead/EmilyHeadScan/00_sss_unpainted_1k.png", "Models/HumanHead/EmilyHeadScan/00_sss_unpainted_1k.png.import", "Models/HumanHead/InfiniteScan/Images/Infinite-Level_02_SSS.png", "Models/HumanHead/InfiniteScan/Images/Infinite-Level_02_SSS.png.import", "Models/HumanHead/InfiniteScan/Map-SSS_COL.png", "Models/HumanHead/InfiniteScan/Map-SSS_COL.png.import"], "wrinkle": ["Models/HumanHead/EmilyHeadScan/emily_wrinkles_1_mask.png", "Models/HumanHead/EmilyHeadScan/emily_wrinkles_1_mask.png.import", "Models/HumanHead/EmilyHeadScan/emily_wrinkles_1_nrm.png", "Models/HumanHead/EmilyHeadScan/emily_wrinkles_1_nrm.png.import"], "iris": ["HumanShaders/Resources/Eye/eye_iris_diff.png", "HumanShaders/Resources/Eye/eye_iris_diff.png.import", "HumanShaders/Resources/Eye/eye_iris_nrm.png", "HumanShaders/Resources/Eye/eye_iris_nrm.png.import", "HumanShaders/Resources/Eye/eye_iris_parallax.png", "HumanShaders/Resources/Eye/eye_iris_parallax.png.import", "Models/HumanHead/EmilyHeadScan/Emili_Head_Optimized_eye_iris_depth_2.png", "Models/HumanHead/EmilyHeadScan/Emili_Head_Optimized_eye_iris_depth_2.png.import"], "shader": ["HumanShaders/Resources/Eye/eye_iris_diff.png", "HumanShaders/Resources/Eye/eye_iris_diff.png.import", "HumanShaders/Resources/Eye/eye_iris_nrm.png", "HumanShaders/Resources/Eye/eye_iris_nrm.png.import", "HumanShaders/Resources/Eye/eye_iris_parallax.png", "HumanShaders/Resources/Eye/eye_iris_parallax.png.import", "HumanShaders/Resources/Eye/eye_sclera_diff.png", "HumanShaders/Resources/Eye/eye_sclera_diff.png.import", "HumanShaders/Resources/Eye/eye_sclera_nrm.png", "HumanShaders/Resources/Eye/eye_sclera_nrm.png.import", "HumanShaders/Resources/InterleavedGradientNoise.png", "HumanShaders/Resources/InterleavedGradientNoise.png.import", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao.png.import", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_2.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_2.png.import", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_3.png", "HumanShaders/Resources/MicroDetail/skin_micro_nrm_ao_3.png.import", "HumanShaders/Resources/lightwarp_skin1.tres", "HumanShaders/Resources/translucency_gradient.tres", "HumanShaders/eye_shader.gdshader", "HumanShaders/eye_shader.gdshader.uid", "HumanShaders/skin_shader.gdshader", "HumanShaders/skin_shader.gdshader.uid", "HumanShaders/vshader_eye_wetness.tres", "Shaders/Lambert_SchlickGGX.gdshader", "Shaders/Lambert_SchlickGGX.gdshader.uid", "SkinShaderTest.tscn"], "face": ["Scripts/Visualizer/face_demo_camera.gd", "Scripts/Visualizer/face_demo_camera.gd.uid"]}

## Prior selected-function review (partial)

{"files": ["HumanShaders/eye_shader.gdshader"], "finding": "Iris heightmap parallax, separate diffuse normal from specular surface normal, SSS_STRENGTH and direct LIGHT_COLOR/ATTENUATION."}

This prior partial review may target a different pinned revision; compare hashes before reusing findings.

## Source file: HumanShaders/eye_shader.gdshader

{
  "path": "HumanShaders/eye_shader.gdshader",
  "blob_sha": "29229445445bf763bab94e03bfbfae67a59fee8b",
  "sha256": "b4f6b361b1174f19815f68483b8c7ddfd9a8c76220bde478ae938806f5b8fe5e",
  "lines": 178,
  "bytes": 6047,
  "selection_score": 42,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 1,
    "subsurface": 5,
    "sss": 7,
    "iris": 22,
    "shader": 3,
    "rig": 1,
    "face": 5,
    "lip": 2
  },
  "url": "https://github.com/matmadness/HumanShaders/blob/95a54cff4a9165993914143ca9047a5f5d029bce/HumanShaders/eye_shader.gdshader",
  "function_signature_candidates": [
    {
      "line": 50,
      "text": "void vertex() {"
    },
    {
      "line": 54,
      "text": "vec3 normal_blending(vec3 b, vec3 d, float factor) {"
    },
    {
      "line": 58,
      "text": "void fragment() {"
    },
    {
      "line": 127,
      "text": "float DistributionGGX(float cos_theta_m, float alpha){"
    },
    {
      "line": 133,
      "text": "float GeometryGGX(float NdotL, float NdotV, float alpha){"
    },
    {
      "line": 137,
      "text": "vec3 SchlickBaseReflectivity(){"
    },
    {
      "line": 142,
      "text": "float SchlickFresnel(float u){"
    },
    {
      "line": 148,
      "text": "void light()"
    }
  ]
}

```text

/*
MIT License
Copyright (c) 2025 MatMADNESS

Permission is hereby granted, free of charge, to use, copy, modify, and
distribute this software as permitted under the MIT License.

This Shader is part of the Human Shaders collection by MatMADNESS
and is built upon https://github.com/RustyRoboticsBV/GodotStandardLightShader
*/

shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx, sss_mode_skin;

uniform vec4 sclera_albedo : source_color = vec4(1.);
uniform vec4 iris_albedo : source_color = vec4(1.);
uniform sampler2D sclera_albedo_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D iris_albedo_texture : source_color, filter_linear_mipmap, repeat_enable;
uniform bool height_enabled = true;

group_uniforms normal;
uniform sampler2D sclera_normal_texture : hint_roughness_normal, filter_linear_mipmap, repeat_enable;
uniform sampler2D iris_normal_texture : hint_roughness_normal, filter_linear_mipmap, repeat_enable;
uniform float normal_strength : hint_range(-16.0, 16.0) = 1.;

uniform float iris_scale : hint_range(0.0, 10.0, 0.1) = 3.5;

uniform float roughness : hint_range(0.0, 1.0, 0.01) = 0.1;
uniform float specular : hint_range(0.0, 1.0, 0.01) = 0.5;
uniform float metallic : hint_range(0.0, 1.0, 0.01);

group_uniforms height;
uniform bool use_heightmap = true;
uniform sampler2D texture_heightmap : hint_default_black, filter_linear_mipmap, repeat_enable;
uniform float heightmap_scale : hint_range(0.0, 128.0, 0.1) = 32.0;
uniform vec2 heightmap_flip;

group_uniforms subsurface_scattering;
uniform bool subsurface_scattering = true;
uniform bool use_noise = false;
uniform float subsurface_scattering_strength : hint_range(0.0, 1.0, 0.01) = 0.4;
uniform sampler2D texture_sss_noise : filter_linear, repeat_enable, source_color;

uniform vec3 uv1_scale;
uniform vec3 uv1_offset;

varying vec3 albedo;
varying vec3 DIFFUSE_NORMAL;

void vertex() {
	UV = UV * uv1_scale.xy + uv1_offset.xy;
}

vec3 normal_blending(vec3 b, vec3 d, float factor) {
	return vec3(b.xy + mix(vec2(.5), d.xy, factor) - 0.5, b.z);
}

void fragment() {
	vec2 base_uv = UV;

	//float depth_tex = texture(depth_texture, base_uv).r * .5;
	float fresnel = clamp(dot(normalize(NORMAL), normalize(VIEW)), 0.0, 1.0 );
	fresnel = clamp(pow(1.- fresnel + 0.32,7.), 0., 1.) +1.;

	vec2 uv_center = vec2(0.5);
	vec2 iris_uv = clamp(((UV - uv_center) * iris_scale + uv_center), 0.0, 1.);
	float iris_alpha = texture(iris_albedo_texture, iris_uv).a;
	vec4 iris_depth = texture(texture_heightmap, iris_uv);
	iris_alpha = smoothstep(0.0,0.2, 1.- iris_depth.b);
	float iris_uv_size = 1.6;

//Parallax effect. Only runs of not in compatibility renderer.
#if CURRENT_RENDERER != RENDERER_COMPATIBILITY
	if (use_heightmap) {
		iris_uv_size /= (1. + heightmap_scale * 0.025);
		vec3 view_dir = normalize(normalize(-VERTEX + EYE_OFFSET) * mat3(TANGENT * heightmap_flip.x, -BINORMAL * heightmap_flip.y, NORMAL));
		float depth = 1.0 - iris_depth.b;
		vec2 ofs = iris_uv - view_dir.xy / (view_dir.z * fresnel) * depth * heightmap_scale * 0.01;
		iris_uv = clamp(ofs,0.,1.);
	}
#endif

	iris_uv = (iris_uv - uv_center) * iris_uv_size + uv_center;

	vec4 sclera_diff = texture(sclera_albedo_texture, base_uv) * sclera_albedo;
	vec4 iris_diff = texture(iris_albedo_texture, iris_uv) * iris_albedo;

	albedo = mix(sclera_diff, iris_diff, iris_alpha).rgb;

	ALBEDO = albedo;

	ROUGHNESS = roughness;

	vec3 sclera_normal_tex = textureLod(sclera_normal_texture, base_uv, 4).rgb;
	vec3 iris_normal_tex = textureLod(iris_normal_texture, iris_uv, 4).rgb;

	vec3 iris_normal_mix = normal_blending(1.5 - iris_depth.rgb * 2., iris_normal_tex, 1.0);

	mat3 TBN = mat3(TANGENT, -BINORMAL, NORMAL);
	vec3 diff_normal_map = mix(
		vec3(.5,.5,1.),
		iris_normal_mix,
		smoothstep(0.05,0.8,1. - iris_depth.b) * 5.0);
	vec3 diff_normal_tangent = vec3(
		diff_normal_map.r * 2.0 - 1.0,  // X-axis (unchanged)
		(1.0 - diff_normal_map.g) * 2.0 - 1.0,  // Y-axis (inverted)
		1   // Z-axis (solid blue)
	);
	DIFFUSE_NORMAL = normalize(TBN * diff_normal_tangent);

	NORMAL_MAP = mix(sclera_normal_tex, vec3(.5,.5,1.), iris_alpha);
	NORMAL_MAP_DEPTH = normal_strength;

	//SSS
	if (subsurface_scattering){
		float sss_strength_value = subsurface_scattering_strength;
		if (use_noise){
			vec2 tiling_screen_uv = FRAGCOORD.xy / (0.64 * -1.);
			float noise = 3.* texture(texture_sss_noise, tiling_screen_uv).r;
			sss_strength_value *= noise;
		}
		SSS_STRENGTH = sss_strength_value * (1.- iris_alpha * .5);
	}

}

float DistributionGGX(float cos_theta_m, float alpha){
	float alpha2 = alpha * alpha;
	float d = 1.0 + (alpha2 - 1.0) * cos_theta_m * cos_theta_m;
	return alpha2 / (PI * d * d);
}

float GeometryGGX(float NdotL, float NdotV, float alpha){
	return 0.5 / mix(2.0 * NdotL * NdotV, NdotL + NdotV, alpha);
}

vec3 SchlickBaseReflectivity(){
	float dielectric = 0.04 * specular * specular;
	return mix(vec3(dielectric), albedo.rgb, vec3(metallic));
}

float SchlickFresnel(float u){
	float m = 1.0 - u;
	float m2 = m * m;
	return m2 * m2 * m;
}

void light()
{
	// Calculate some vectors.
	vec3 lightColor = LIGHT_COLOR / PI;

	vec3 half = normalize(VIEW + LIGHT);

	float NdotL = max(dot(NORMAL, LIGHT), 0.0);
	float NdotV = max(dot(NORMAL, VIEW), 0.0);
	float NdotH = max(dot(NORMAL, half), 0.0);
	float LdotH = max(dot(LIGHT, half), 0.0);

	// Diffuse light (Lambert).

	DIFFUSE_LIGHT += clamp(dot(mix(DIFFUSE_NORMAL, NORMAL, 0.3), LIGHT),0.,1.) * ATTENUATION * lightColor;

	//Custom Diffuse Light.

	// Specular light (Schlick-GGX).
	float ggxAlpha = clamp(ROUGHNESS * ROUGHNESS, 0.01, 1.0);//clamping to avoid weird behavior at 0 roughness
	float D = DistributionGGX(NdotH, ggxAlpha);
	float G = GeometryGGX(NdotL, NdotV, ggxAlpha);

	vec3 f0 = SchlickBaseReflectivity();
	float LdotH5 = SchlickFresnel(LdotH);
	float f90 = clamp(50.0 * f0.g, 0.0, 1.0);
	vec3 F = f0 + (f90 - f0) * LdotH5;

	vec3 specularBRDF = max(NdotL * D * G * F, 0.0);
	SPECULAR_LIGHT += specularBRDF * LIGHT_COLOR * ATTENUATION;
}

```

## Source file: HumanShaders/skin_shader.gdshader

{
  "path": "HumanShaders/skin_shader.gdshader",
  "blob_sha": "ca411e3a39db1307e07416fb7a173703f9307ce7",
  "sha256": "b59e22741b3cc824982d8c74b4d928d26015f084f04177c1e897ed44be334737",
  "lines": 323,
  "bytes": 11317,
  "selection_score": 42,
  "keyword_line_counts": {
    "skin": 5,
    "subsurface": 6,
    "sss": 13,
    "shader": 3,
    "rig": 1,
    "face": 6
  },
  "url": "https://github.com/matmadness/HumanShaders/blob/95a54cff4a9165993914143ca9047a5f5d029bce/HumanShaders/skin_shader.gdshader",
  "function_signature_candidates": [
    {
      "line": 82,
      "text": "void vertex() {"
    },
    {
      "line": 88,
      "text": "vec3 normal_blending(vec3 b, vec3 d) {"
    },
    {
      "line": 92,
      "text": "vec3 overlay(vec3 base, vec3 blend){"
    },
    {
      "line": 97,
      "text": "vec3 custom_normal(vec3 normal_map){"
    },
    {
      "line": 107,
      "text": "float ScreenSpaceDither12(vec2 vScreenPos, float time){"
    },
    {
      "line": 111,
      "text": "void fragment() {"
    },
    {
      "line": 200,
      "text": "float DistributionGGX(float cos_theta_m, float alpha){"
    },
    {
      "line": 206,
      "text": "float GeometryGGX(float NdotL, float NdotV, float alpha){"
    },
    {
      "line": 210,
      "text": "vec3 SchlickBaseReflectivity(){"
    },
    {
      "line": 215,
      "text": "float SchlickFresnel(float u){"
    },
    {
      "line": 221,
      "text": "void light()"
    }
  ]
}

```text

/*
MIT License
Copyright (c) 2025 MatMADNESS

Permission is hereby granted, free of charge, to use, copy, modify, and
distribute this software as permitted under the MIT License.

This Shader is part of the Human Shaders collection by MatMADNESS
and is built upon https://github.com/RustyRoboticsBV/GodotStandardLightShader
*/

shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx, sss_mode_skin;

group_uniforms albedo;
uniform vec4 albedo : source_color = vec4(1.);
uniform sampler2D texture_albedo : source_color, filter_linear_mipmap, repeat_enable;

group_uniforms roughness;
uniform float roughness : hint_range(0.0, 1.0) = 0.4;
uniform float specular : hint_range(0.0, 1.0, 0.01) = 0.5;
uniform bool double_specularity = true;

uniform sampler2D texture_roughness : hint_roughness_r, filter_linear_mipmap, repeat_enable;

group_uniforms metallic;
uniform float metallic : hint_range(0.0, 1.0, 0.01);
uniform sampler2D texture_metallic : hint_default_white, filter_linear_mipmap, repeat_enable;
uniform vec4 metallic_texture_channel;

group_uniforms normal;
uniform sampler2D texture_normal : hint_roughness_normal, filter_linear_mipmap, repeat_enable;
uniform float normal_strength : hint_range(-16.0, 16.0) = 1.0;

group_uniforms micro_detail;
uniform bool use_micro_detail = false;
uniform float micro_detail_scale : hint_range(0.0, 200.0) = 25.0;
uniform float micro_normal_strength : hint_range(-1.0, 1.0) = 0.3;
uniform float micro_ao_strength : hint_range(-1.0, 1.0) = 0.4;
uniform sampler2D texture_micro_detail : hint_roughness_normal, filter_linear_mipmap_anisotropic, repeat_enable;
//uniform float soften_normal : hint_range(0.0, 1.0);

group_uniforms ambient_occlusion;
uniform bool use_ambient_occlusion = false;
uniform sampler2D ambient_occlusion_texture : hint_roughness_normal, filter_linear_mipmap, repeat_enable;
uniform float ao_strength : hint_range(0.0, 2.0) = 0.5;
uniform float ao_block_light : hint_range(0.0, 2.0) = 0.5;

group_uniforms subsurface_scattering;
uniform bool use_subsurface_scattering = true;
uniform bool use_noise = true;
uniform float subsurface_scattering_strength : hint_range(0.0, 1.0, 0.01) = 0.4;
uniform float skin_smoothness : hint_range(0.0, 10.0) = 5.0;
uniform float skin_fallof_smoothness : hint_range(0.7, 2.0, 0.01) = 1.0;
uniform bool tinted_shadow_penumbra = false;
uniform bool old_lightwarp_fallof = false;
uniform sampler2D lightwarp : source_color, filter_linear, repeat_disable;
uniform sampler2D texture_sss_noise : filter_linear, repeat_enable, source_color;


group_uniforms translucency;
uniform bool translucency = false;
uniform float translucency_strength : hint_range(0.0, 2.0, 0.01) = 0.75;
uniform sampler2D texture_translucency : hint_default_white, filter_linear_mipmap, repeat_enable;
uniform sampler2D texture_translucency_color : source_color, filter_linear, repeat_disable;

group_uniforms u_v;
uniform vec3 uv1_scale = vec3(1.);
uniform vec3 uv1_offset = vec3(0.);
uniform vec3 uv2_scale = vec3(1.);
uniform vec3 uv2_offset = vec3(0.);

varying mat3 TBN;
varying vec4 micro_detail;
varying vec3 final_albedo;
varying float final_metallic;
varying float ao;
varying float linear_depth;
varying float smooth_depth;
varying float sss_tex;

void vertex() {
	UV = UV * uv1_scale.xy + uv1_offset.xy;
	// Detail UV Layer: UV2
	UV2 = UV2 * uv2_scale.xy + uv2_offset.xy;
}

vec3 normal_blending(vec3 b, vec3 d) {
	return vec3(b.xy + d.xy - 0.5, b.z);
}

vec3 overlay(vec3 base, vec3 blend){
	vec3 limit = step(0.5, base);
	return mix(2.0 * base * blend, 1.0 - 2.0 * (1.0 - base) * (1.0 - blend), limit);
}

vec3 custom_normal(vec3 normal_map){
	normal_map = mix(vec3(0.5,0.5,1.0), normal_map, normal_strength);
	vec3 normal_tangent = vec3(
		normal_map.r * 2.0 - 1.0,  // X-axis (unchanged)
		(1.0 - normal_map.g) * 2.0 - 1.0,  // Y-axis (inverted)
		1   // Z-axis (solid blue)
		);
	return normalize(TBN * normal_tangent);
}

float ScreenSpaceDither12(vec2 vScreenPos, float time){
	float vDither = dot( vec2( 171.0, 231.0 ), vScreenPos.xy + time); 
	return fract(vDither / 53.2);
}
void fragment() {
	vec2 base_uv = UV;
	TBN = mat3(TANGENT, -BINORMAL, NORMAL);

	//Raw depth value:
	float depth_raw = FRAGCOORD.z;

	//Convert to linear:
	vec3 ndc = vec3(SCREEN_UV * 2.0 - 1.0, depth_raw);
	vec4 view = INV_PROJECTION_MATRIX * vec4(ndc, 1.0);
	view.xyz /= view.w;

	// reverse
	linear_depth = clamp(1. + (view.z / 1.0), 0., 1.);
	smooth_depth = smoothstep(0.0, 0.6, linear_depth);

	sss_tex = texture(texture_translucency, base_uv).b;

	//float fresnel = clamp(dot(normalize(NORMAL), normalize(VIEW)), 0.0, 1.0 );

	micro_detail = texture(texture_micro_detail, base_uv * micro_detail_scale);
	ao = texture(texture_normal, base_uv).b;
	ao *= texture(ambient_occlusion_texture, base_uv).r;

	float micro_overlay = mix(0.5, micro_detail.b, micro_ao_strength * 2.);
	vec4 albedo_tex = texture(texture_albedo, base_uv);
	if (use_micro_detail){
		albedo_tex = vec4(overlay(albedo_tex.rgb, vec3(micro_overlay)), albedo_tex.w);
	}

	final_albedo = albedo.rgb * albedo_tex.rgb;

	float metallic_tex = dot(texture(texture_metallic, base_uv), metallic_texture_channel);
	final_metallic = metallic_tex * metallic;
	ALBEDO = final_albedo * (1.0 - final_metallic);


	//METALLIC = metallic_tex * metallic;

	//float spec = max(dot(NORMAL, LIGHT_VERTEX * LIGHT_VERTEX), 0.0);

	//ALBEDO = vec3(1.);
	SPECULAR = clamp(specular, 0.,1.);
	if (use_ambient_occlusion){
		SPECULAR *= ao;
	}

	vec4 roughness_texture_channel = vec4(0.0, 1.0, 0.0, 0.0);
	float roughness_tex = dot(texture(texture_roughness, base_uv), roughness_texture_channel);
	ROUGHNESS = clamp(roughness_tex * roughness, 0.,1.);

	// Normal Map: Enabled
	vec3 micro_normal = mix(vec3(.5,.5,1.), vec3(micro_detail.xy, 1.), micro_normal_strength);

	//Normal Map (For Specular):
	vec3 normal_final = texture(texture_normal, base_uv).rgb;
	normal_final = normal_blending(normal_final, micro_normal);
	NORMAL_MAP = normal_final;
	NORMAL_MAP_DEPTH = normal_strength;

	// Ambient Occlusion
	if (use_ambient_occlusion){
		AO = ao * ao_strength + (1.-ao_strength);// * smoothstep(0.,1., micro_detail.z * 1.6);
		AO_LIGHT_AFFECT = 0.0;
	}

	// Subsurface Scattering: Enabled
	if (use_subsurface_scattering){
		if (smooth_depth > 0.){
			float sss_strength_value = subsurface_scattering_strength * 2.0;
			if (translucency){
				sss_strength_value += (sss_tex * 2.);
			}
			if (use_noise){
				vec2 ts = vec2(textureSize(texture_sss_noise, 0)) * 0.85;
				vec2 dither_uv = mod(FRAGCOORD.xy, ts) / ts + (TIME * 2.5);
				vec2 tiling_screen_uv = FRAGCOORD.xy / (0.64 * -1.);
				float noise = 3.* texture(texture_sss_noise, dither_uv).r;// + vec2(1000. * TIME)).r;
				//noise = 2.0 * ScreenSpaceDither12(FRAGCOORD.xy, TIME * .50);
				sss_strength_value *= noise;
				//ALBEDO = vec3(noise);
			}
			SSS_STRENGTH = sss_strength_value * texture(ambient_occlusion_texture, base_uv).r * smooth_depth;
		}
		//ALBEDO = vec3(smooth_depth);
		//float sss_depth = 1. - pow(clamp(world_camera.z,0.,1.), 0.5);
	}

}
float DistributionGGX(float cos_theta_m, float alpha){
	float alpha2 = alpha * alpha;
	float d = 1.0 + (alpha2 - 1.0) * cos_theta_m * cos_theta_m;
	return alpha2 / (PI * d * d);
}

float GeometryGGX(float NdotL, float NdotV, float alpha){
	return 0.5 / mix(2.0 * NdotL * NdotV, NdotL + NdotV, alpha);
}

vec3 SchlickBaseReflectivity(){
	float dielectric = 0.04 * specular * specular;
	return mix(vec3(dielectric), final_albedo, vec3(final_metallic));
}

float SchlickFresnel(float u){
	float m = 1.0 - u;
	float m2 = m * m;
	return m2 * m2 * m;
}

void light()
{
	// Calculate some vectors.
	vec3 lightColor = LIGHT_COLOR / PI;

	vec3 half = normalize(VIEW + LIGHT);

	float NdotL = max(dot(NORMAL, LIGHT), 0.0);
	float NdotV = max(dot(NORMAL, VIEW), 0.0);
	float NdotH = max(dot(NORMAL, half), 0.0);
	float LdotH = max(dot(LIGHT, half), 0.0);

	// Diffuse light (Lambert):
	//DIFFUSE_LIGHT += clamp(dot(mix(DIFFUSE_NORMAL, NORMAL, 0.3), LIGHT),0.,1.) * ATTENUATION * lightColor;

	//Custom wrapped diffuse:
	float wrap = 0.5;
	vec3 diff_light;
	float normal_depth = max((1.5-linear_depth), 1.);
	//normal_depth = 1.;

	if (!old_lightwarp_fallof){ //Better SSS aproximation:

		//Calculate custom normal:
		float s = skin_smoothness;
		vec3 dn_r = custom_normal(textureLod(texture_normal, UV, (1.*s) * normal_depth).rgb);
		vec3 dn_g = custom_normal(textureLod(texture_normal, UV, (0.8*s) * normal_depth).rgb);
		vec3 dn_b = custom_normal(textureLod(texture_normal, UV, (0.7*s) * normal_depth).rgb);
		
		//Include micro normals in the mix:
		vec3 m_normal = texture(texture_micro_detail, UV * micro_detail_scale).rgb;
		m_normal = mix(vec3(.5,.5,1.), vec3(m_normal.xy, 1.), micro_normal_strength);
		
		dn_g = normal_blending(dn_g, m_normal);
		dn_b = normal_blending(dn_b, m_normal);
		////
		
		float sf = skin_fallof_smoothness;
		
		float wrapped_r = smoothstep(0.4 /sf, 1.2 *sf, dot(dn_r, LIGHT) * wrap + (1.0-wrap));
		float wrapped_g = smoothstep(0.425 /sf, 1.2 *sf, dot(dn_g, LIGHT) * wrap + (1.0-wrap));
		float wrapped_b = smoothstep(0.44 /sf, 1.2 *sf, dot(dn_b, LIGHT) * wrap + (1.0-wrap));
		vec3 wrapped_rgb = vec3(wrapped_r, wrapped_g, wrapped_b);
		if (use_ambient_occlusion){
			wrapped_rgb *= (ao * ao_block_light + (1.-ao_block_light));
		}

		diff_light = wrapped_rgb;
	}
	else{ //Lightwarp technique:
		vec3 diff_normal_map = mix(
			textureLod(texture_normal, UV, 6. * normal_depth).rgb,
			textureLod(texture_normal, UV, 3. * normal_depth).rgb,
			.2);
		vec3 diff_normal = custom_normal(diff_normal_map);

		float wrapped = pow(dot(diff_normal, LIGHT) * wrap + (1.0-wrap), 2.0);
		vec3 lw = texture(lightwarp, vec2(wrapped, 0.0)).rgb;
		if (use_ambient_occlusion){
			lw = texture(lightwarp, vec2(wrapped * (ao * ao_block_light + (1.-ao_block_light)), 0.0)).rgb;
		}
		diff_light = lw;
	}

	//Tint shadow penumbra:
	vec3 att = overlay(min(vec3(ATTENUATION), 1.), vec3(1.2,.8,.6)) * max(ATTENUATION, 0.5);
	if (tinted_shadow_penumbra){att = att;} else{att = vec3(ATTENUATION);}

	DIFFUSE_LIGHT += diff_light * att * lightColor;

	if (translucency){
		float translucency_dot = dot(VIEW,-(LIGHT + NORMAL * .4));
		vec2 t_col_coord = vec2(sss_tex * translucency_dot * translucency_strength, 0.0);
		vec3 t_color = texture(texture_translucency_color, t_col_coord).rgb;
		vec3 translucent = max(translucency_dot * t_color, 0.0) * lightColor;
		//translucency *= max(translucency_strength, 1.);

		DIFFUSE_LIGHT += translucent;
	}

	vec3 f0 = SchlickBaseReflectivity();
	float LdotH5 = SchlickFresnel(LdotH);
	float f90 = clamp(50.0 * f0.g, 0.0, 1.0);
	vec3 F = f0 + (f90 - f0) * LdotH5;

	// Specular light (Schlick-GGX).
	float ggxAlpha = ROUGHNESS * ROUGHNESS;
	float D = DistributionGGX(NdotH, ggxAlpha);
	float G = GeometryGGX(NdotL, NdotV, ggxAlpha);

	vec3 specularBRDF = max(NdotL * D * G * F, 0.0);
	if (double_specularity){
		float D2 = DistributionGGX(NdotH, ggxAlpha + .3);
		float G2 = GeometryGGX(NdotL, NdotV, ggxAlpha + .3);

		vec3 specularBRDF2 = max(NdotL * D2 * G2 * F, 0.0);
		specularBRDF = mix(specularBRDF2, specularBRDF, .6) * 2.;
	}
	SPECULAR_LIGHT += specularBRDF * LIGHT_COLOR * ATTENUATION;
	if (use_ambient_occlusion){
		SPECULAR_LIGHT *= ao * ao_block_light + (1.-ao_block_light);
	}
}


```

## Source file: Shaders/Lambert_SchlickGGX.gdshader

{
  "path": "Shaders/Lambert_SchlickGGX.gdshader",
  "blob_sha": "0682e9fca8cb31656eb468e99c76a5c4d329fb40",
  "sha256": "27ca52ba7b0d5caa590086336f9b96ca502c26e6229e56935d564cb9307b2ca1",
  "lines": 96,
  "bytes": 3850,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1,
    "rig": 5
  },
  "url": "https://github.com/matmadness/HumanShaders/blob/95a54cff4a9165993914143ca9047a5f5d029bce/Shaders/Lambert_SchlickGGX.gdshader",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "void fragment()"
    },
    {
      "line": 44,
      "text": "float DistributionGGX(float cos_theta_m, float alpha)"
    },
    {
      "line": 51,
      "text": "float GeometryGGX(float NdotL, float NdotV, float alpha)"
    },
    {
      "line": 56,
      "text": "vec3 SchlickBaseReflectivity(float metallic, float specular, vec3 albedo)"
    },
    {
      "line": 62,
      "text": "float SchlickFresnel(float u)"
    },
    {
      "line": 69,
      "text": "void light()"
    }
  ]
}

```text

/**************************************************************************/
/*                         This file is part of:                          */
/*                             GODOT ENGINE                               */
/*                        https://godotengine.org                         */
/**************************************************************************/
/* Copyright (c) 2014-present Godot Engine contributors (see AUTHORS.md). */
/* Copyright (c) 2007-2014 Juan Linietsky, Ariel Manzur.                  */
/*                                                                        */
/* Permission is hereby granted, free of charge, to any person obtaining  */
/* a copy of this software and associated documentation files (the        */
/* "Software"), to deal in the Software without restriction, including    */
/* without limitation the rights to use, copy, modify, merge, publish,    */
/* distribute, sublicense, and/or sell copies of the Software, and to     */
/* permit persons to whom the Software is furnished to do so, subject to  */
/* the following conditions:                                              */
/*                                                                        */
/* The above copyright notice and this permission notice shall be         */
/* included in all copies or substantial portions of the Software.        */
/*                                                                        */
/* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,        */
/* EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF     */
/* MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. */
/* IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY   */
/* CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,   */
/* TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE      */
/* SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.                 */
/**************************************************************************/

shader_type spatial;

// Uniforms.
uniform vec3 Albedo: source_color = vec3(1, 1, 1);
uniform float Roughness : hint_range(0.0, 1.0) = 1.0;
uniform float Metallicness : hint_range(0.0, 1.0) = 0.0;

// Methods.
void fragment()
{
	ALBEDO = Albedo;
	ROUGHNESS = Roughness;
	METALLIC = Metallicness;
}

float DistributionGGX(float cos_theta_m, float alpha)
{
	float alpha2 = alpha * alpha;
	float d = 1.0 + (alpha2 - 1.0) * cos_theta_m * cos_theta_m;
	return alpha2 / (PI * d * d);
}

float GeometryGGX(float NdotL, float NdotV, float alpha)
{
	return 0.5 / mix(2.0 * NdotL * NdotV, NdotL + NdotV, alpha);
}

vec3 SchlickBaseReflectivity(float metallic, float specular, vec3 albedo)
{
	float dielectric = 0.04 * specular * specular;
	return mix(vec3(dielectric), albedo, vec3(metallic));
}
	
float SchlickFresnel(float u)
{
	float m = 1.0 - u;
	float m2 = m * m;
	return m2 * m2 * m;
}

void light()
{
	// Calculate some vectors.
	vec3 lightColor = LIGHT_COLOR / PI;
	
	vec3 half = normalize(VIEW + LIGHT);
	
	float NdotL = max(dot(NORMAL, LIGHT), 0.0);
	float NdotV = max(dot(NORMAL, VIEW), 0.0);
	float NdotH = max(dot(NORMAL, half), 0.0);
	float LdotH = max(dot(LIGHT, half), 0.0);
	
	// Diffuse light (Lambert).
	DIFFUSE_LIGHT += clamp(dot(NORMAL, LIGHT), 0.0, 1.0) * ATTENUATION * lightColor;
	
	// Specular light (Schlick-GGX).
	float ggxAlpha = ROUGHNESS * ROUGHNESS;
	float D = DistributionGGX(NdotH, ggxAlpha);
	float G = GeometryGGX(NdotL, NdotV, ggxAlpha);
	
	vec3 f0 = SchlickBaseReflectivity(METALLIC, SPECULAR_AMOUNT, ALBEDO);
	float LdotH5 = SchlickFresnel(LdotH);
	float f90 = clamp(50.0 * f0.g, 0.0, 1.0);
	vec3 F = f0 + (f90 - f0) * LdotH5;
	
	vec3 specularBRDF = max(NdotL * D * G * F, 0.0);
	SPECULAR_LIGHT += specularBRDF * LIGHT_COLOR * ATTENUATION;
}


```

## Source file: Scripts/Visualizer/face_demo_camera.gd

{
  "path": "Scripts/Visualizer/face_demo_camera.gd",
  "blob_sha": "f2f3ff6dcc1d85db9cfa296f4929d68d8642adbc",
  "sha256": "e3a5270a225fab3d62b67fe52740290be9ff19e736b480c8f7d2dd352e8b1d58",
  "lines": 61,
  "bytes": 1690,
  "selection_score": 13,
  "keyword_line_counts": {},
  "url": "https://github.com/matmadness/HumanShaders/blob/95a54cff4a9165993914143ca9047a5f5d029bce/Scripts/Visualizer/face_demo_camera.gd",
  "function_signature_candidates": []
}

```text

extends Camera3D

# Variables to define FOV limits and sensitivity
@export var min_fov: float = 10.0
@export var max_fov: float = 75.0
@export var default_fov: float = 25.0
@export var zoom_sensitivity: float = 2.0
@export var movement_sensitivity: float = 2.0

var dragging: bool = false
var previous_mouse_position: Vector2
var default_pos: Vector3
var tween: Tween

func _ready() -> void:
	default_pos = position
	fov = default_fov
	
	
#func transition(property: String, value: float) -> float:
#	tween.tween_property(self, property, )

func _input(event: InputEvent) -> void:
	
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			tween = create_tween().bind_node(self).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
			tween.tween_property(
				self, 
				"fov",
				clamp(fov - zoom_sensitivity, min_fov, max_fov),
				0.1
			)
			
		if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			tween = create_tween().bind_node(self).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
			tween.tween_property(
				self, 
				"fov",
				clamp(fov + zoom_sensitivity, min_fov, max_fov),
				0.1
			)
			
		if event.button_index == MOUSE_BUTTON_MIDDLE:
			if event.pressed:
				dragging = true
				previous_mouse_position = event.position
			else:
				dragging = false
		
	elif event is InputEventMouseMotion and dragging:
					# Calculate the drag distance
			var delta_xy = Vector3(-event.relative.x, event.relative.y, 0)
			# Rotate the model along the Y-axis
			position -= -delta_xy * movement_sensitivity * .0001


func _on_reset_camera_pressed() -> void:
	position = default_pos
	fov = default_fov
	Autoload.log_message("Reset Camera Applied")


```

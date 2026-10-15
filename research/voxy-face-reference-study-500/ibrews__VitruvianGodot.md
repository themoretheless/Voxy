# Facial rendering source scan: ibrews/VitruvianGodot

Repository: https://github.com/ibrews/VitruvianGodot

Pinned source revision: bdecdcd537b4031fdd0fb299b7e4f93f084fffa0

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "NOTICE.md", "godot_project/addons/eyeball_shader/LICENSE.txt"]

Topic paths: {"eye": ["blender_prep/_eye_probe.py", "blender_prep/_glb_eye_probe.py", "blender_prep/eyeball_src.glb", "blender_prep/vitruvian_eye_probe.py", "docs/audit-2026-06-after/eyes_closeup.png", "docs/audit-2026-06-after/z_eye_macro.png", "docs/audit-2026-06-after/z_eyes_tuned.png", "docs/audit-2026-06/eyes_closeup.png", "docs/audit-2026-06/z_eye_macro.png", "docs/audit-2026-06/z_eyes_tuned.png", "godot_project/addons/eyeball_shader/LICENSE.txt", "godot_project/addons/eyeball_shader/shaders/blend_modes.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/blend_modes.gdshaderinc.uid", "godot_project/addons/eyeball_shader/shaders/cornea.gdshader", "godot_project/addons/eyeball_shader/shaders/cornea.gdshader.uid", "godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader", "godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader.uid", "godot_project/addons/eyeball_shader/shaders/random.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/random.gdshaderinc.uid", "godot_project/addons/eyeball_shader/shaders/voronoi.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/voronoi.gdshaderinc.uid", "godot_project/scenes/eye.gdshader", "godot_project/scenes/eye.gdshader.uid"], "skin": ["blender_prep/bake_skin_textures.py", "godot_project/scenes/skin_shader_local.gdshader", "godot_project/scenes/skin_shader_local.gdshader.uid"], "blendshape": ["blender_prep/_export_head_blendshapes.py"], "jaw": ["docs/audit-2026-06-after/face_jawopen.png", "docs/audit-2026-06-after/z_mouth_jaw.png", "docs/audit-2026-06/face_jawopen.png", "docs/audit-2026-06/z_mouth_jaw.png"], "mouth": ["docs/audit-2026-06-after/z_mouth_jaw.png", "docs/audit-2026-06/z_mouth_jaw.png", "godot_project/scenes/mouth_interior.gdshader", "godot_project/scenes/mouth_interior.gdshader.uid", "godot_project/vit_mouth.png"], "facs": ["blender_prep/_facs_proof.py"], "iris": ["godot_project/vit_iris.png"], "cornea": ["godot_project/addons/eyeball_shader/shaders/cornea.gdshader", "godot_project/addons/eyeball_shader/shaders/cornea.gdshader.uid"], "shader": ["godot_project/addons/eyeball_shader/LICENSE.txt", "godot_project/addons/eyeball_shader/shaders/blend_modes.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/blend_modes.gdshaderinc.uid", "godot_project/addons/eyeball_shader/shaders/cornea.gdshader", "godot_project/addons/eyeball_shader/shaders/cornea.gdshader.uid", "godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader", "godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader.uid", "godot_project/addons/eyeball_shader/shaders/random.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/random.gdshaderinc.uid", "godot_project/addons/eyeball_shader/shaders/voronoi.gdshaderinc", "godot_project/addons/eyeball_shader/shaders/voronoi.gdshaderinc.uid", "godot_project/scenes/eye.gdshader", "godot_project/scenes/eye.gdshader.uid", "godot_project/scenes/hair_backing.gdshader", "godot_project/scenes/hair_backing.gdshader.uid", "godot_project/scenes/hair_card.gdshader", "godot_project/scenes/hair_card.gdshader.uid", "godot_project/scenes/hair_card_shadow.gdshader", "godot_project/scenes/hair_card_shadow.gdshader.uid", "godot_project/scenes/hairtool_card.gdshader", "godot_project/scenes/hairtool_card.gdshader.uid", "godot_project/scenes/hidden_discard.gdshader", "godot_project/scenes/hidden_discard.gdshader.uid", "godot_project/scenes/mouth_interior.gdshader", "godot_project/scenes/mouth_interior.gdshader.uid", "godot_project/scenes/scalp_cap.gdshader", "godot_project/scenes/scalp_cap.gdshader.uid", "godot_project/scenes/skin_shader_local.gdshader", "godot_project/scenes/skin_shader_local.gdshader.uid", "godot_project/scenes/vignette.gdshader"], "rig": ["blender_prep/_hair_springrig.py", "blender_prep/_rig_experiment.py", "blender_prep/vitruvian_rigged.blend", "godot_project/vitruvian_hair_rigged.glb"], "face": ["blender_prep/_face_probe.py", "docs/audit-2026-06-after/face_jawopen.png", "docs/audit-2026-06-after/face_neutral.png", "docs/audit-2026-06-after/face_smile.png", "docs/audit-2026-06-after/face_surprise.png", "docs/audit-2026-06/face_jawopen.png", "docs/audit-2026-06/face_neutral.png", "docs/audit-2026-06/face_smile.png", "docs/audit-2026-06/face_surprise.png", "godot_project/scenes/face_extras.gd", "godot_project/scenes/face_extras.gd.uid", "godot_project/vit_face_bc.png", "godot_project/vit_face_n.png", "godot_project/vit_face_rough.png"], "lip": ["blender_prep/_ht_clip_export.py"]}

## Source file: godot_project/scenes/skin_shader_local.gdshader

{
  "path": "godot_project/scenes/skin_shader_local.gdshader",
  "blob_sha": "290b47294ce66928d4e57408fa1b02388007e3eb",
  "sha256": "63160ea9be3f3c79558d9c311e2704d8ae50e0bc1c8f7842df89fbad8dec411b",
  "lines": 350,
  "bytes": 13148,
  "selection_score": 42,
  "keyword_line_counts": {
    "skin": 8,
    "subsurface": 6,
    "sss": 27,
    "shader": 4,
    "rig": 2,
    "face": 6,
    "lip": 1
  },
  "url": "https://github.com/ibrews/VitruvianGodot/blob/bdecdcd537b4031fdd0fb299b7e4f93f084fffa0/godot_project/scenes/skin_shader_local.gdshader",
  "function_signature_candidates": [
    {
      "line": 96,
      "text": "void vertex() {"
    },
    {
      "line": 102,
      "text": "vec3 normal_blending(vec3 b, vec3 d) {"
    },
    {
      "line": 106,
      "text": "vec3 overlay(vec3 base, vec3 blend){"
    },
    {
      "line": 111,
      "text": "vec3 custom_normal(vec3 normal_map){"
    },
    {
      "line": 121,
      "text": "float ScreenSpaceDither12(vec2 vScreenPos, float time){"
    },
    {
      "line": 125,
      "text": "void fragment() {"
    },
    {
      "line": 227,
      "text": "float DistributionGGX(float cos_theta_m, float alpha){"
    },
    {
      "line": 233,
      "text": "float GeometryGGX(float NdotL, float NdotV, float alpha){"
    },
    {
      "line": 237,
      "text": "vec3 SchlickBaseReflectivity(){"
    },
    {
      "line": 242,
      "text": "float SchlickFresnel(float u){"
    },
    {
      "line": 248,
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
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

group_uniforms albedo;
uniform vec4 albedo : source_color = vec4(1.);
uniform sampler2D texture_albedo : source_color, filter_linear_mipmap, repeat_enable;

group_uniforms roughness;
uniform float roughness : hint_range(0.0, 1.0) = 0.4;
// FIX (white-speckle/glitter): hard floor so a mis-tuned roughness of ~0 can never
// produce mirror-sharp GGX fireflies (specular aliasing = white dots) on skin.
uniform float roughness_floor : hint_range(0.0, 1.0) = 0.28;
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
// FIX (ported from skin_shader_explainer): the original linear_depth = clamp(1 + view.z/1.0)
// gates SSS off beyond ~1m (hero-closeup assumption). At portrait/turntable distance
// (~1.65m) view.z≈-1.65 → linear_depth=0 → SSS skipped. Divide view.z by this larger
// scale so SSS engages at look-dev distance. Higher = SSS reaches further from camera.
uniform float sss_depth_scale = 6.0;
// FIX: warm shadow-terminator tint (the fleshy microdancing look). Default ON now.
uniform bool tinted_shadow_penumbra = true;
uniform bool old_lightwarp_fallof = false;
uniform sampler2D lightwarp : source_color, filter_linear, repeat_disable;
uniform sampler2D texture_sss_noise : filter_linear, repeat_enable, source_color;
// MetaHuman baked Scatter map (T_Head_LOD1_Scatter_VT) — per-region SSS amount/colour
// so light passes through thin areas (ears, nostrils, lips) like real skin.
uniform bool use_scatter_map = false;
uniform sampler2D texture_scatter : source_color, filter_linear_mipmap, repeat_enable;
uniform float scatter_strength : hint_range(0.0, 3.0) = 1.4;


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

	// reverse  (FIX: divisor was 1.0 → SSS gated off past ~1m; now tunable)
	linear_depth = clamp(1. + (view.z / sss_depth_scale), 0., 1.);
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
	ROUGHNESS = clamp(max(roughness_tex * roughness, roughness_floor), 0.,1.);

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
			// FIX (AO-kills-SSS guard): only fold the cavity/AO map into SSS when AO is
			// actually enabled. The generic skin_micro_cav map is mostly black, so an
			// unconditional multiply silently zeroed SSS whenever use_ambient_occlusion=false.
			float sss_ao = use_ambient_occlusion ? texture(ambient_occlusion_texture, base_uv).r : 1.0;
			SSS_STRENGTH = sss_strength_value * sss_ao * smooth_depth;
			// MetaHuman Scatter map: drive SSS amount + fleshy transmittance per region.
			if (use_scatter_map){
				vec3 sc = texture(texture_scatter, base_uv).rgb;
				float scl = dot(sc, vec3(0.30, 0.59, 0.11));
				SSS_STRENGTH *= clamp(scl * scatter_strength, 0.25, 2.5);
				SSS_TRANSMITTANCE_COLOR = vec4(sc, 1.0);
				SSS_TRANSMITTANCE_DEPTH = 0.4;
				SSS_TRANSMITTANCE_BOOST = 0.3;
			}
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

## Source file: godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader

{
  "path": "godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader",
  "blob_sha": "977be5b59de40091a9511e012f2e340538404494",
  "sha256": "5555543b6a3c2f49261c5a5a731b8a1ec85198cb2f0855662d73676154f08643",
  "lines": 108,
  "bytes": 4708,
  "selection_score": 42,
  "keyword_line_counts": {
    "eye": 15,
    "iris": 23,
    "shader": 5,
    "rig": 4,
    "face": 2
  },
  "url": "https://github.com/ibrews/VitruvianGodot/blob/bdecdcd537b4031fdd0fb299b7e4f93f084fffa0/godot_project/addons/eyeball_shader/shaders/eyeball_shader.gdshader",
  "function_signature_candidates": [
    {
      "line": 67,
      "text": "void vertex() {"
    },
    {
      "line": 71,
      "text": "void fragment() {"
    }
  ]
}

```text

/**
* MIT License
*
* Copyright (c) 2024 Mark McKay
* https://github.com/blackears/eyeball_shader
*
* Permission is hereby granted, free of charge, to any person obtaining a copy
* of this software and associated documentation files (the "Software"), to deal
* in the Software without restriction, including without limitation the rights
* to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
* copies of the Software, and to permit persons to whom the Software is
* furnished to do so, subject to the following conditions:
*
* The above copyright notice and this permission notice shall be included in all
* copies or substantial portions of the Software.
*
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
* IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
* FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
* AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
* LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
* OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
* SOFTWARE.
*/

shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

group_uniforms color_and_size;
uniform float iris_radius:hint_range(0.0, 1.0, 0.01) = .25;
uniform float iris_margin:hint_range(0.0, 1.0, 0.01) = .02;
uniform float pupil_radius:hint_range(0.0, 1.0, 0.01) = .1;

uniform vec3 eye_white:source_color = vec3(1, 1, 1);
uniform vec3 pupil_color:source_color = vec3(0, 0, 0);
uniform sampler2D texture_iris_color : source_color, filter_linear_mipmap, repeat_disable;
uniform sampler2D texture_iris_ring_dodge : source_color, filter_linear_mipmap, repeat_disable, hint_default_black;
uniform sampler2D texture_iris_ring_mult : source_color, filter_linear_mipmap, repeat_disable, hint_default_white;


group_uniforms iris_cell_properties;
uniform float eye_cell_scale:hint_range(1.0, 20.0, 0.1) = 17;
uniform float eye_cell_jitter:hint_range(0.0, 1.0, 0.1) = .6;
uniform float iris_pinch:hint_range(0.0, 1.0, 0.01) = .6;
uniform uint iris_cells_num_octaves = 1;
uniform float iris_cells_lacunarity = 2;
uniform float iris_cells_weighted_strength = .5;

group_uniforms surface;
// Agile Lens additions: the stock shader writes only ALBEDO (fully rough, flat).
// Give the eyeball a wet glossy surface + radial sclera shading so the whites
// aren't a flat disc.
uniform float eyeball_roughness:hint_range(0.0, 1.0, 0.01) = 0.28;
uniform float eyeball_specular:hint_range(0.0, 1.0, 0.01) = 0.6;
uniform float sclera_shade:hint_range(0.0, 1.0, 0.01) = 0.45;
uniform vec3 sclera_edge_tint:source_color = vec3(0.86, 0.74, 0.66);

group_uniforms other;
uniform uint rand_seed;

uniform vec3 uv1_scale = vec3(1, 1, 1);
uniform vec3 uv1_offset;

#include "res://addons/eyeball_shader/shaders/voronoi.gdshaderinc"
#include "res://addons/eyeball_shader/shaders/blend_modes.gdshaderinc"

void vertex() {
	UV = UV * uv1_scale.xy + uv1_offset.xy;
}

void fragment() {
	
	vec2 uv_norm = UV * 2.0 - vec2(1, 1);
	
	float radial_offset = length(uv_norm);
	
	vec2 iris_uv = normalize(uv_norm) * pow(radial_offset, 1.0 - iris_pinch);
	float eye_cell_lum = voronoi_cell_2d(iris_uv * eye_cell_scale, eye_cell_jitter, rand_seed,
		iris_cells_num_octaves, iris_cells_lacunarity, iris_cells_weighted_strength);
	vec3 iris_col = texture(texture_iris_color, vec2(eye_cell_lum, 0)).xyz;

	vec3 col = mix(pupil_color, eye_white, smoothstep(0, 1, (radial_offset - iris_radius) / iris_margin));
	col = mix(iris_col, col, smoothstep(0, 1, (radial_offset - iris_radius + iris_margin) / iris_margin));
	
	col = mix(pupil_color, col, smoothstep(0, 1, (radial_offset - pupil_radius) / iris_margin));

	// Sclera shading: only the white beyond the iris. Darken toward the rim and
	// warm-tint the very edge (corner-of-eye occlusion / vascular warmth) so the
	// whites read with depth instead of flat.
	float scl = smoothstep(iris_radius, 1.0, radial_offset);
	if (radial_offset > iris_radius) {
		col *= mix(1.0, 1.0 - sclera_shade, scl);
		col = mix(col, col * sclera_edge_tint, scl * 0.6);
	}

	if (radial_offset >= pupil_radius && radial_offset <= iris_radius) {
		vec4 dodge_col = texture(texture_iris_ring_dodge, vec2((radial_offset - pupil_radius) / (iris_radius - pupil_radius), 0.0));
		vec4 mult_col = texture(texture_iris_ring_mult, vec2((radial_offset - pupil_radius) / (iris_radius - pupil_radius), 0.0));

		col = blend_color_dodge(vec4(col, 1), dodge_col).rgb;
		col = blend_multiply(vec4(col, 1), mult_col).rgb;
	}
	
	ALBEDO = col;
	ROUGHNESS = eyeball_roughness;
	SPECULAR = eyeball_specular;
	METALLIC = 0.0;
}


```

## Source file: godot_project/addons/eyeball_shader/shaders/cornea.gdshader

{
  "path": "godot_project/addons/eyeball_shader/shaders/cornea.gdshader",
  "blob_sha": "92c078c67eb0d8788345f2621016851a43ef7dd7",
  "sha256": "16b30fd1a7700c99a648f7b975222c5a65e16eaf8573a8bcadfc35e9cf1fb516",
  "lines": 48,
  "bytes": 1942,
  "selection_score": 33,
  "keyword_line_counts": {
    "eye": 1,
    "cornea": 2,
    "shader": 2,
    "rig": 5
  },
  "url": "https://github.com/ibrews/VitruvianGodot/blob/bdecdcd537b4031fdd0fb299b7e4f93f084fffa0/godot_project/addons/eyeball_shader/shaders/cornea.gdshader",
  "function_signature_candidates": [
    {
      "line": 36,
      "text": "void light() {"
    },
    {
      "line": 45,
      "text": "void fragment() {"
    }
  ]
}

```text

/**
* MIT License
*
* Copyright (c) 2024 Mark McKay
* https://github.com/blackears/eyeball_shader
*
* Permission is hereby granted, free of charge, to any person obtaining a copy
* of this software and associated documentation files (the "Software"), to deal
* in the Software without restriction, including without limitation the rights
* to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
* copies of the Software, and to permit persons to whom the Software is
* furnished to do so, subject to the following conditions:
*
* The above copyright notice and this permission notice shall be included in all
* copies or substantial portions of the Software.
*
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
* IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
* FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
* AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
* LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
* OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
* SOFTWARE.
*/

shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx, blend_add;

uniform float shininess:hint_range(0.0, 800.0, .1) = 200;
// Agile Lens addition: intensity scalar + alpha clamp so the additive cornea
// highlight from bright key/catch lights can't blow the whole cornea to a white
// disc (it did, dead-centre, before clamping).
uniform float spec_intensity:hint_range(0.0, 1.0, 0.01) = 0.4;
uniform float alpha_max:hint_range(0.0, 1.0, 0.01) = 0.85;

void light() {
	vec3 half_vec = normalize(VIEW + LIGHT);
	float n_dot_half = max(0.0, dot(NORMAL, half_vec));
	float pf = pow(n_dot_half, shininess * SPECULAR_AMOUNT) * spec_intensity;

	SPECULAR_LIGHT += pf * ATTENUATION;
	ALPHA = min(ALPHA + pf * ATTENUATION, alpha_max);
}

void fragment() {
	ALPHA = 0.0;
}



```

## Source file: godot_project/scenes/eye.gdshader

{
  "path": "godot_project/scenes/eye.gdshader",
  "blob_sha": "0cc400f2b344381ecfd9f3693485348a66b735c9",
  "sha256": "d7e9ec1cc89ccf17ba95feb5c2f9c7b003da630a663dc0b2095c5fed1f8ac58f",
  "lines": 50,
  "bytes": 2378,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 5,
    "iris": 14,
    "cornea": 1,
    "shader": 2,
    "face": 2
  },
  "url": "https://github.com/ibrews/VitruvianGodot/blob/bdecdcd537b4031fdd0fb299b7e4f93f084fffa0/godot_project/scenes/eye.gdshader",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type spatial;
// Eye shader (v2 — real MetaHuman baked eye textures).
// Composites the real iris disc (T_EyeIris*_BC, centered in UV) over the real
// veined sclera (T_EyeSclera*_BC, full eyeball UV) with their NORMAL maps for
// surface relief (iris fiber + sclera), plus a corneal clearcoat for a wet,
// reflective eye that catches the softbox/reflection-probe.
render_mode cull_back, diffuse_burley, specular_schlick_ggx;

uniform sampler2D iris_texture : source_color, filter_linear_mipmap;
uniform sampler2D sclera_texture : source_color, filter_linear_mipmap;
uniform sampler2D iris_normal : hint_normal, filter_linear_mipmap;
uniform sampler2D sclera_normal : hint_normal, filter_linear_mipmap;
uniform float iris_radius : hint_range(0.05, 0.5) = 0.27;
uniform float blend_softness : hint_range(0.0, 0.15) = 0.025;
uniform float iris_scale : hint_range(0.5, 4.0) = 1.0;
uniform float roughness_val : hint_range(0.0, 1.0) = 0.03;
uniform float specular_val : hint_range(0.0, 1.0) = 1.0;
uniform float clearcoat_val : hint_range(0.0, 1.0) = 1.0;
uniform float clearcoat_roughness_val : hint_range(0.0, 1.0) = 0.03;
uniform float normal_strength : hint_range(0.0, 2.0) = 0.6;
uniform float sclera_tint : hint_range(0.0, 1.0) = 1.0;  // 1 = full veins, lower = whiter

void fragment() {
	vec2 ctr = UV - vec2(0.5);
	float dist = length(ctr);

	// Iris disc, centered in UV (optionally zoomed).
	vec2 iris_uv = clamp(ctr * iris_scale + vec2(0.5), vec2(0.0), vec2(1.0));
	vec3 iris_rgb = texture(iris_texture, iris_uv).rgb;

	// Real veined sclera at the eyeball's native UV. Mix toward off-white so it
	// doesn't read too pink/gray; sclera_tint controls how much vein detail.
	vec3 sclera_raw = texture(sclera_texture, UV).rgb;
	vec3 sclera_rgb = mix(vec3(0.80, 0.78, 0.76), sclera_raw, sclera_tint);  // not pure white — real sclera is light grey

	float mask = smoothstep(iris_radius, iris_radius + blend_softness, dist);
	ALBEDO = mix(iris_rgb, sclera_rgb, mask);

	// Surface relief: iris fiber normal inside, sclera normal outside.
	vec3 n_iris = texture(iris_normal, iris_uv).rgb;
	vec3 n_scle = texture(sclera_normal, UV).rgb;
	NORMAL_MAP = mix(n_iris, n_scle, mask);
	NORMAL_MAP_DEPTH = normal_strength;

	ROUGHNESS = roughness_val;
	SPECULAR = specular_val;
	METALLIC = 0.0;
	CLEARCOAT = clearcoat_val;
	CLEARCOAT_ROUGHNESS = clearcoat_roughness_val;
}


```

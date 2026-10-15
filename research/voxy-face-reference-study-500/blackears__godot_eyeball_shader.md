# Facial rendering source scan: blackears/godot_eyeball_shader

Repository: https://github.com/blackears/godot_eyeball_shader

Pinned source revision: 058108056552a93f90c23420a25a9feba125e1a4

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"eye": ["docs/eyeball_shader_properties.jpg", "docs/eyeball_window.jpg", "godot/addons/eyeball_shader/eyeball_shader_plugin.gd", "godot/addons/eyeball_shader/plugin.cfg", "godot/addons/eyeball_shader/shaders/blend_modes.gdshaderinc", "godot/addons/eyeball_shader/shaders/cornea.gdshader", "godot/addons/eyeball_shader/shaders/eyeball_shader.gdshader", "godot/addons/eyeball_shader/shaders/random.gdshaderinc", "godot/addons/eyeball_shader/shaders/voronoi.gdshaderinc", "godot/art/eyeball.blend", "godot/art/eyeball.blend1", "godot/art/eyeball.glb", "godot/art/eyeball.glb.import", "godot/eye_single.tres", "godot/textures/eye_grad_blue.tres", "godot/textures/eye_grad_brown.tres"], "cornea": ["godot/addons/eyeball_shader/shaders/cornea.gdshader"], "shader": ["docs/eyeball_shader_properties.jpg", "godot/addons/eyeball_shader/eyeball_shader_plugin.gd", "godot/addons/eyeball_shader/plugin.cfg", "godot/addons/eyeball_shader/shaders/blend_modes.gdshaderinc", "godot/addons/eyeball_shader/shaders/cornea.gdshader", "godot/addons/eyeball_shader/shaders/eyeball_shader.gdshader", "godot/addons/eyeball_shader/shaders/random.gdshaderinc", "godot/addons/eyeball_shader/shaders/voronoi.gdshaderinc"]}

## Source file: godot/addons/eyeball_shader/shaders/eyeball_shader.gdshader

{
  "path": "godot/addons/eyeball_shader/shaders/eyeball_shader.gdshader",
  "blob_sha": "38183e9543b112c793322a53cdf237e46dd017a9",
  "sha256": "3ffa3a71543f7c903957a97a6d64d4f7b60737989ee25044dd947eb7ae9fc000",
  "lines": 87,
  "bytes": 3761,
  "selection_score": 42,
  "keyword_line_counts": {
    "eye": 9,
    "iris": 20,
    "shader": 4,
    "rig": 4
  },
  "url": "https://github.com/blackears/godot_eyeball_shader/blob/058108056552a93f90c23420a25a9feba125e1a4/godot/addons/eyeball_shader/shaders/eyeball_shader.gdshader",
  "function_signature_candidates": [
    {
      "line": 58,
      "text": "void vertex() {"
    },
    {
      "line": 62,
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

	if (radial_offset >= pupil_radius && radial_offset <= iris_radius) {
		vec4 dodge_col = texture(texture_iris_ring_dodge, vec2((radial_offset - pupil_radius) / (iris_radius - pupil_radius), 0.0));
		vec4 mult_col = texture(texture_iris_ring_mult, vec2((radial_offset - pupil_radius) / (iris_radius - pupil_radius), 0.0));

		col = blend_color_dodge(vec4(col, 1), dodge_col).rgb;
		col = blend_multiply(vec4(col, 1), mult_col).rgb;
	}
	
	ALBEDO = col;
}


```

## Source file: godot/addons/eyeball_shader/shaders/cornea.gdshader

{
  "path": "godot/addons/eyeball_shader/shaders/cornea.gdshader",
  "blob_sha": "1cd99ef43344ab1b689a49c996ea6f89d195a153",
  "sha256": "21634e1f1cf4ca45d290c2bd4f17e7c7a82a3e9562fe8610fc53d2a7083204e5",
  "lines": 43,
  "bytes": 1574,
  "selection_score": 33,
  "keyword_line_counts": {
    "eye": 1,
    "shader": 2,
    "rig": 4
  },
  "url": "https://github.com/blackears/godot_eyeball_shader/blob/058108056552a93f90c23420a25a9feba125e1a4/godot/addons/eyeball_shader/shaders/cornea.gdshader",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "void light() {"
    },
    {
      "line": 40,
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

void light() {
	vec3 half_vec = normalize(VIEW + LIGHT);
	float n_dot_half = max(0.0, dot(NORMAL, half_vec));
	float pf = pow(n_dot_half, shininess * SPECULAR_AMOUNT);
	
	SPECULAR_LIGHT += pf * ATTENUATION;
	ALPHA += pf * ATTENUATION;
}

void fragment() {
	ALPHA = 0.0;
}



```

## Source file: godot/addons/eyeball_shader/eyeball_shader_plugin.gd

{
  "path": "godot/addons/eyeball_shader/eyeball_shader_plugin.gd",
  "blob_sha": "45a417e6d66a682a579e90fc6948841369749fae",
  "sha256": "22c7b9e3babab1b1743b451c753f824fa82714b72f91a8477e9bff9d50c6afc4",
  "lines": 12,
  "bytes": 162,
  "selection_score": 30,
  "keyword_line_counts": {},
  "url": "https://github.com/blackears/godot_eyeball_shader/blob/058108056552a93f90c23420a25a9feba125e1a4/godot/addons/eyeball_shader/eyeball_shader_plugin.gd",
  "function_signature_candidates": []
}

```text

@tool
extends EditorPlugin


func _enter_tree():
	# Initialization of the plugin goes here.
	pass


func _exit_tree():
	# Clean-up of the plugin goes here.
	pass


```

## Source file: godot/gen_code.gd

{
  "path": "godot/gen_code.gd",
  "blob_sha": "33176d2b200e1bd9af7b33ac6fda80909baae2cd",
  "sha256": "1301fb1b68a040e1e89b9c542c5b6a0ee9f448be9e9d55d3f63b8954e4d167de",
  "lines": 35,
  "bytes": 801,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/blackears/godot_eyeball_shader/blob/058108056552a93f90c23420a25a9feba125e1a4/godot/gen_code.gd",
  "function_signature_candidates": []
}

```text

extends Control


# Called when the node enters the scene tree for the first time.
func _ready():
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass


func _on_bn_gen_code_pressed():
	var num_vec = 256;

	print("const vec2 rand_offsets_v2[] = {")
	var count:int = 0
	var text:String = ""
	for i in num_vec:
		var v:Vector2 = Vector2(randf(), randf()) * 2 - Vector2.ONE
		v = v.normalized()
		if count == 0:
			text += "    vec2(%f, %f)," % [v.x, v.y]
		elif count % 4 == 0:
			text += ",\n    vec2(%f, %f)," % [v.x, v.y]
		else:
			text += ", vec2(%f, %f)" % [v.x, v.y]
		
		count += 1
	print(text)
	print("};")
	print("const uint RAND_OFFSETS_V2_SIZE = %du;" % num_vec)
		
	pass # Replace with function body.


```

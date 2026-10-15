# Facial rendering source scan: CChapman03/Godot_Foliage_SSS_Shader

Repository: https://github.com/CChapman03/Godot_Foliage_SSS_Shader

Pinned source revision: 4eb44a703b4dca9636a2882e5877a8dab921ee0d

Discovery topic: subsurface scattering

Description: My implementation of a Foliage Subsurface Scattering effect in Godot 4.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["Materials/SSS_Foliage.gdshader", "Materials/Test_SSS_Shader.gdshader", "Textures/Tree_texture_SSS.png", "Textures/Tree_texture_SSS.png.import"], "shader": ["Materials/SSS_Foliage.gdshader", "Materials/Test_SSS_Shader.gdshader"]}

## Source file: Materials/Test_SSS_Shader.gdshader

{
  "path": "Materials/Test_SSS_Shader.gdshader",
  "blob_sha": "24cf4c360be42d3f83fc421144676b614f0b54e7",
  "sha256": "a29ec8494cafaa0d8d3f71ac8bbc7705542479dae749f7af9e453e0a21f78596",
  "lines": 136,
  "bytes": 5719,
  "selection_score": 40,
  "keyword_line_counts": {
    "skin": 1,
    "subsurface": 7,
    "sss": 18,
    "shader": 1,
    "face": 7
  },
  "url": "https://github.com/CChapman03/Godot_Foliage_SSS_Shader/blob/4eb44a703b4dca9636a2882e5877a8dab921ee0d/Materials/Test_SSS_Shader.gdshader",
  "function_signature_candidates": [
    {
      "line": 60,
      "text": "vec3 getWind(vec3 vertex, float timer) {"
    },
    {
      "line": 74,
      "text": "void vertex() {"
    },
    {
      "line": 84,
      "text": "void fragment() {"
    },
    {
      "line": 112,
      "text": "void light() {"
    }
  ]
}

```text

shader_type spatial;
render_mode blend_mix,depth_draw_always,cull_back,diffuse_burley,specular_schlick_ggx,sss_mode_skin,alpha_to_coverage_and_one;

uniform vec4 albedo : source_color = vec4(1.0);
uniform sampler2D texture_albedo : source_color,filter_linear_mipmap,repeat_enable;

uniform float alpha_scissor_threshold = 0.992;
uniform float alpha_antialiasing_edge;

uniform ivec2 albedo_texture_size;
uniform float point_size : hint_range(0,128);

uniform float roughness : hint_range(0,1) = 1.0;

uniform sampler2D texture_metallic : hint_default_white,filter_linear_mipmap,repeat_enable;
uniform vec4 metallic_texture_channel;
uniform sampler2D texture_roughness : hint_roughness_r,filter_linear_mipmap,repeat_enable;

uniform float specular = 0.0;
uniform float metallic = 0.0;

uniform sampler2D texture_normal : hint_roughness_normal,filter_linear_mipmap,repeat_enable;
uniform float normal_scale : hint_range(-16,16) = 1.0;

uniform sampler2D texture_ambient_occlusion : hint_default_white, filter_linear_mipmap,repeat_enable;
uniform vec4 ao_texture_channel;
uniform float ao_light_affect = 0.5;

uniform float subsurface_scattering_strength : hint_range(0,1) = 0.0;
uniform sampler2D texture_subsurface_scattering : hint_default_white,filter_linear_mipmap,repeat_enable;
uniform vec4 transmittance_color : source_color;
uniform float transmittance_depth = 32.0;
uniform sampler2D texture_subsurface_transmittance : hint_default_white,filter_linear_mipmap,repeat_enable;
uniform float transmittance_boost = 0.0;
uniform sampler2D sss_texture;

uniform vec3 uv1_scale;
uniform vec3 uv1_offset;
uniform vec3 uv2_scale;
uniform vec3 uv2_offset;

// sss settings
uniform vec2 sss_thickness;
uniform float sss_power = 1.0;
uniform float sss_distortion = 2.0;
uniform float sss_scale = 0.5;
uniform float sss_strength = 4.0;
uniform vec4 sss_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);

// wind settings
uniform float speed = 1.0;
uniform float min_strength : hint_range(0.0, 1.0);
uniform float max_strength : hint_range(0.0, 1.0) = 0.1;
uniform float interval = 3.5;
uniform float detail = 1.0;
uniform float distortion : hint_range(0.0, 1.0) = 0.8;
uniform vec2 direction = vec2(1.0, 1.0);
uniform float heightOffset = 0.15;

vec3 getWind(vec3 vertex, float timer) {
	vec4 pos = mix(vec4(1.0), vec4(vertex, 1.0), distortion);
	float time = timer * speed + pos.x + pos.z;
	float diff = pow(max_strength - min_strength, 2);
	float strength = clamp(min_strength + diff + sin(time / interval) * diff, min_strength, max_strength);
	float wind = (sin(time) + cos(time * detail)) * strength * max(0.0, vertex.y - heightOffset);
	vec2 dir = normalize(direction);
	
	return vec3(wind * dir.x, 0.0, wind * dir.y);
}

varying vec3 v_Normal;
varying vec3 f_Normal;

void vertex() {
	UV=UV*uv1_scale.xy+uv1_offset.xy;
	
	vec4 worldPos = vec4(VERTEX, 1.0);
	worldPos.xyz += getWind(VERTEX, TIME);
	VERTEX = (worldPos).xyz;
	
	v_Normal = NORMAL;
}

void fragment() {
	vec2 base_uv = UV;
	vec4 albedo_tex = texture(texture_albedo,base_uv);
	ALBEDO = albedo.rgb * albedo_tex.rgb;
	float metallic_tex = dot(texture(texture_metallic,base_uv),metallic_texture_channel);
	METALLIC = metallic_tex * metallic;
	vec4 roughness_texture_channel = vec4(1.0,0.0,0.0,0.0);
	float roughness_tex = dot(texture(texture_roughness,base_uv),roughness_texture_channel);
	ROUGHNESS = roughness_tex * roughness;
	SPECULAR = specular;
	NORMAL_MAP = texture(texture_normal,base_uv).rgb;
	NORMAL_MAP_DEPTH = normal_scale;
	ALPHA *= albedo.a * albedo_tex.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_scissor_threshold;
	ALPHA_ANTIALIASING_EDGE = alpha_antialiasing_edge;
	ALPHA_TEXTURE_COORDINATE = UV * vec2(albedo_texture_size);
	AO = dot(texture(texture_ambient_occlusion,base_uv),ao_texture_channel);
	AO_LIGHT_AFFECT = ao_light_affect;
	//float sss_tex = texture(texture_subsurface_scattering,base_uv).r;
	//SSS_STRENGTH=subsurface_scattering_strength*sss_tex;
	//vec4 trans_color_tex = texture(texture_subsurface_transmittance,base_uv);
	//SSS_TRANSMITTANCE_COLOR=transmittance_color*trans_color_tex;
	//SSS_TRANSMITTANCE_DEPTH=transmittance_depth;
	//SSS_TRANSMITTANCE_BOOST=transmittance_boost;
	
	f_Normal = NORMAL;
}

void light() {
	vec3 viewDir = normalize(VIEW);
	vec3 lightDir = normalize(LIGHT);
	
	// Translucency.
	vec3 transLightDir = (lightDir + f_Normal) * sss_distortion;
	float transDot = pow ( max (0, dot ( viewDir, -transLightDir ) ), sss_power ) * sss_scale;
	vec3 transLight = ( transDot ) * ALPHA * sss_color.rgb * texture(sss_texture, UV).rgb * sss_strength; // use this
	//vec3 transLight = (ATTENUATION * 1.0) * ( transDot ) * ALPHA * _SubColor.rgb * texture(sss_texture, UV).rgb;  // not this
	
	vec3 regularLight = clamp(dot(NORMAL, LIGHT), 0.0, 1.0) * ALBEDO;
	
	vec3 a = regularLight;
	vec3 b = transLight;
	
	vec3 combinedLight = regularLight + transLight; // using this blend mode
	vec3 combinedLight_softlight = (1.0 - a) * (a * b) + a * (1.0 - (1.0 - a) * (1.0 - b));
	vec3 combinedLight_hardlight = float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) > 0.5) * (1.0 - (1.0-b) * (1.0-2.0*(a-0.5))) + float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) <= 0.5) * (b * (2.0*a));
	vec3 combinedLight_overlay = float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) < 0.5) * (2.0*a*b) + float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) > 0.5) * (1.0 - 2.0 * (1.0 - a) + 2.0 * (1.0 - b));
	vec3 combinedLight_vivid = float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) > 0.5) * (b / (1.0-2.0*(a-0.5))) + float(sqrt(a.x * a.x + a.y * a.y + a.z * a.z) <= 0.5) * (1.0 - (1.0-b) / (2.0*a));
	
	// The key to subsurface scattering is to calculate ATTENUATION last
	// Otherwise, it will look noisy
	DIFFUSE_LIGHT = combinedLight * ATTENUATION * LIGHT_COLOR;
}


```

## Source file: Materials/SSS_Foliage.gdshader

{
  "path": "Materials/SSS_Foliage.gdshader",
  "blob_sha": "a45385a7e3696ac50ca8812177e889728482da75",
  "sha256": "3908ef050a0149a5dba5253f86fe0f45c5df1320036e483d63c495ccfb4a81f4",
  "lines": 170,
  "bytes": 5693,
  "selection_score": 30,
  "keyword_line_counts": {
    "subsurface": 2,
    "sss": 13,
    "shader": 1,
    "face": 2
  },
  "url": "https://github.com/CChapman03/Godot_Foliage_SSS_Shader/blob/4eb44a703b4dca9636a2882e5877a8dab921ee0d/Materials/SSS_Foliage.gdshader",
  "function_signature_candidates": [
    {
      "line": 75,
      "text": "vec3 getWind(vec3 vertex, float timer) {"
    },
    {
      "line": 86,
      "text": "vec3 get_collision(mat4 world_mat, vec3 vertex)"
    },
    {
      "line": 99,
      "text": "void vertex() {"
    },
    {
      "line": 115,
      "text": "void fragment() {"
    },
    {
      "line": 151,
      "text": "void light() {"
    }
  ]
}

```text

shader_type spatial;
render_mode blend_mix,depth_draw_always,cull_back,diffuse_burley,alpha_to_coverage_and_one;

// Material Propeties
group_uniforms Material;

group_uniforms Material.Albedo;
uniform vec4 albedo : source_color = vec4(1.0);
uniform sampler2D texture_albedo : source_color,filter_linear_mipmap,repeat_enable;
uniform ivec2 albedo_texture_size;

group_uniforms Material.Roughness;
uniform float roughness : hint_range(0,1) = 1.0;
uniform sampler2D texture_roughness : hint_roughness_r,filter_linear_mipmap,repeat_enable;

group_uniforms Material.Specular;
uniform float specular = 0.0;
uniform float metallic = 0.0;
uniform sampler2D texture_metallic : hint_default_white,filter_linear_mipmap,repeat_enable;
uniform vec4 metallic_texture_channel = vec4(0.0);

group_uniforms Material.Normals;
uniform sampler2D texture_normal : hint_roughness_normal,filter_linear_mipmap,repeat_enable;
uniform float normal_strength : hint_range(-16,16) = 0.5;

group_uniforms Material.AmbientOcclusion;
uniform sampler2D texture_ambient_occlusion : hint_default_white, filter_linear_mipmap,repeat_enable;
uniform vec4 ao_texture_channel;
uniform float ao_light_affect = 0.5;

group_uniforms Material.Transparency;
uniform float alpha_scissor_threshold = 0.992;
uniform float alpha_antialiasing_edge = 1.0;

group_uniforms Material.Uvs;
uniform vec3 uv1_scale = vec3(1.0);
uniform vec3 uv1_offset = vec3(0.0);

group_uniforms;

// sss settings
group_uniforms Subsurface_Scattering;
uniform vec2 sss_thickness = vec2(0.0);
uniform float sss_power = 1.0;
uniform float sss_distortion = 2.0;
uniform float sss_scale = 0.5;
uniform float sss_strength = 4.0;
uniform vec4 sss_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);
uniform sampler2D texture_sss;
uniform sampler2D texture_sss_mask;
group_uniforms;

// wind settings
group_uniforms Wind;
uniform float wind_speed = 1.0;
uniform float wind_min_strength : hint_range(0.0, 1.0) = 0.0;
uniform float wind_max_strength : hint_range(0.0, 1.0) = 0.1;
uniform float wind_interval = 3.5;
uniform float wind_detail = 1.0;
uniform float wind_distortion : hint_range(0.0, 1.0) = 0.8;
uniform vec2 wind_direction = vec2(0.9, 0.1);
uniform float wind_height_offset = 0.15;
group_uniforms;

// collison settings
group_uniforms Collision;
uniform float collision_strength = 1.0;
uniform vec3 playerpos;
uniform float collision_distance = 4.0;
group_uniforms;

varying vec3 v_Normal;
varying vec3 f_Normal;

vec3 getWind(vec3 vertex, float timer) {
	vec4 pos = mix(vec4(1.0), vec4(vertex, 1.0), wind_distortion);
	float time = timer * wind_speed + pos.x + pos.z;
	float diff = pow(wind_max_strength - wind_min_strength, 2);
	float strength = clamp(wind_min_strength + diff + sin(time / wind_interval) * diff, wind_min_strength, wind_max_strength);
	float wind = (sin(time) + cos(time * wind_detail)) * strength * max(0.0, vertex.y - wind_height_offset);
	vec2 dir = normalize(wind_direction);
	
	return vec3(wind * dir.x, 0.0, wind * dir.y);
}

vec3 get_collision(mat4 world_mat, vec3 vertex)
{
	vec4 v = world_mat * vec4(vertex, 1.0);
	vec3 diff = normalize(playerpos - vertex.xyz); // vec3(0, 0, 5)
	float dist = distance(playerpos, v.xyz); // 5
	vec3 strength = vec3(collision_strength, 0, collision_strength); // vec3(1.0, 0, 1.0)

	float col_dist = 1.0 - clamp(dist * collision_distance, 0.0, 1.0); // 1.0 - 5.0 / 10.0 = 1.0 - 0.5
	vec3 collision = diff * strength; // vec3(0.0, 0.0, 5.0)

	return col_dist * collision; // vec3(0.0, 0.0, 2.5)
}

void vertex() {
	// Set UV Tiling
	UV= UV * uv1_scale.xy + uv1_offset.xy;
	
	// Move vertices based on wind
	vec4 worldPos = vec4(VERTEX, 1.0);
	worldPos.xyz += getWind(VERTEX, TIME);
	// Move vertices based on collison
	worldPos.xyz += get_collision(MODEL_MATRIX, VERTEX);

	VERTEX = worldPos.xyz;

	// Varyings
	v_Normal = NORMAL;
}

void fragment() {
	// UVs
	vec2 base_uv = UV;

	// Albedo
	vec4 albedo_tex = texture(texture_albedo, base_uv);
	ALBEDO = albedo.rgb * albedo_tex.rgb;

	// Specular
	float metallic_tex = dot(texture(texture_metallic, base_uv), metallic_texture_channel);
	METALLIC = metallic_tex * metallic;
	SPECULAR = specular;
	
	// Roughness
	vec4 roughness_texture_channel = vec4(1.0,0.0,0.0,0.0);
	float roughness_tex = dot(texture(texture_roughness, base_uv), roughness_texture_channel);
	ROUGHNESS = roughness_tex * roughness;
	
	// Normals
	NORMAL_MAP = texture(texture_normal, base_uv).rgb;
	NORMAL_MAP_DEPTH = normal_strength;

	// Ambient Occlusion
	AO = dot(texture(texture_ambient_occlusion, base_uv), ao_texture_channel);
	AO_LIGHT_AFFECT = ao_light_affect;

	// Transparency
	ALPHA *= albedo.a * albedo_tex.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_scissor_threshold;
	ALPHA_ANTIALIASING_EDGE = alpha_antialiasing_edge;
	ALPHA_TEXTURE_COORDINATE = UV * vec2(albedo_texture_size);

	// Varyings
	f_Normal = NORMAL;
}

void light() {
	vec3 viewDir = normalize(VIEW);
	vec3 lightDir = normalize(LIGHT);
	
	// Regular Lighting
	vec3 regularLight = clamp(dot(NORMAL, LIGHT), 0.0, 1.0) * ALBEDO;

	// Translucent Lighting
	vec3 transLightDir = (lightDir + f_Normal) * sss_distortion;
	float transDot = pow ( max (0, dot ( viewDir, -transLightDir ) ), sss_power ) * sss_scale;
	//vec3 transLight = transDot * ALPHA * sss_color.rgb * texture(texture_sss, UV).rgb  * sss_strength;
	vec3 transLight = ( transDot ) * ALPHA * sss_color.rgb * texture(texture_sss, UV).rgb * texture(texture_sss_mask, UV).rgb * sss_strength;
	
	// Final Combined Lighting
	vec3 combinedLight = regularLight + transLight;
	
	// Don't know why, but the key to subsurface scattering is to calculate ATTENUATION last
	// Otherwise, it will look noisy
	DIFFUSE_LIGHT = combinedLight * ATTENUATION * LIGHT_COLOR;
}


```

## Source file: Scripts/Park.gd

{
  "path": "Scripts/Park.gd",
  "blob_sha": "68de534ec8da47583a87ee3bb97cbe0881a5cd0d",
  "sha256": "7bf1725062f355fe66a08288cd6861a9bbf5eaf901575dfde9d0d3557b15830e",
  "lines": 16,
  "bytes": 431,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 3,
    "rig": 1
  },
  "url": "https://github.com/CChapman03/Godot_Foliage_SSS_Shader/blob/4eb44a703b4dca9636a2882e5877a8dab921ee0d/Scripts/Park.gd",
  "function_signature_candidates": []
}

```text

extends Node3D

func _ready():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	
	set_process(true)
	
func _process(delta):
	
	# Update Player Position in shaders
	$Grass.material_override.set_shader_parameter("playerpos", $Player.transform.origin)
	#print($Grass.material_override.get_shader_parameter("playerpos"))
	
	if Input.is_action_pressed("ui_cancel"):
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
		get_tree().quit()


```

## Source file: Scripts/Player.gd

{
  "path": "Scripts/Player.gd",
  "blob_sha": "3b26b6be8818f85a661ccf025e10d1bc919df8cc",
  "sha256": "517ea68ac350a2a2046102726ca29f4215eed9f14938becdfc8eb3037ee1d586",
  "lines": 123,
  "bytes": 2701,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/CChapman03/Godot_Foliage_SSS_Shader/blob/4eb44a703b4dca9636a2882e5877a8dab921ee0d/Scripts/Player.gd",
  "function_signature_candidates": []
}

```text

extends CharacterBody3D

var camera_angle = 0
var mouse_sensitivity = 0.3

var v = Vector3()
var direction = Vector3()

# fly vars
const FLY_SPEED = 40
const FLY_ACCEL = 4

# walk vars
var gravity = -9.8 * 3
const MAX_SPEED = 4
const MAX_RUNNING_SPEED = 10
const ACCEL = 2
const DECCEL = 6

# jump vars
var jump_height = 8

var pos

# Called when the node enters the scene tree for the first time.
func _ready():
	pass # Replace with function body.

func _physics_process(delta):
	walk(delta)

func _input(event):
	if event is InputEventMouseMotion:
		$Head.rotate_y(deg_to_rad(-event.relative.x * mouse_sensitivity))
		
		var change = -event.relative.y * mouse_sensitivity
		if change + camera_angle < 90 and change + camera_angle > -90:
			$Head/Camera.rotate_x(deg_to_rad(change))
			camera_angle += change
			
func walk(delta):
	
	# reset the direction of the player
	direction = Vector3()
	
	# get the rotation of the camera
	var aim = $Head/Camera.get_camera_transform().basis
	
	# check input and change direction
	if (Input.is_action_pressed("move_forward")):
		direction -= aim.z
	if (Input.is_action_pressed("move_backward")):
		direction += aim.z
	if (Input.is_action_pressed("move_left")):
		direction -= aim.x
	if (Input.is_action_pressed("move_right")):
		direction += aim.x
		
	direction = direction.normalized()
	
	# apply gravity
	velocity.y += gravity * delta
	
	var temp_vel = velocity
	temp_vel.y = 0
	
	var speed
	if (Input.is_action_pressed("move_run")):
		speed = MAX_RUNNING_SPEED
	else:
		speed = MAX_SPEED
	
	# set max player speed
	var target = direction * speed
	
	var accelaration
	if direction.dot(temp_vel) > 0:
		accelaration = ACCEL
	else:
		accelaration = DECCEL
	
	# gradually increase velocity
	temp_vel = temp_vel.lerp(target, accelaration * delta)
	
	velocity.x = temp_vel.x
	velocity.z = temp_vel.z
	
	# move
	v = move_and_slide()
	
	if(Input.is_action_just_pressed("jump")):
		if velocity.y < jump_height and is_on_floor():
			velocity.y = jump_height
		else:
			velocity.y = 0
		
func fly(delta):
	# reset the direction of the player
	direction = Vector3()
	
	# get the rotation of the camera
	var aim = $Head/Camera.get_camera_transform().basis
	
	# check input and change direction
	if (Input.is_action_pressed("move_forward")):
		direction -= aim.z
	if (Input.is_action_pressed("move_backward")):
		direction += aim.z
	if (Input.is_action_pressed("move_left")):
		direction -= aim.x
	if (Input.is_action_pressed("move_right")):
		direction += aim.x
		
	direction = direction.normalized()
	
	# set max player speed
	var target = direction * FLY_SPEED
	
	# gradually increase velocity
	velocity = v.lerp(target, FLY_ACCEL * delta)
	
	# move
	move_and_slide()


```

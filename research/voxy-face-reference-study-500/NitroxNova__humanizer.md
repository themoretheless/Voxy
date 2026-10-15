# Facial rendering source scan: NitroxNova/humanizer

Repository: https://github.com/NitroxNova/humanizer

Pinned source revision: 21323ee75b520419f305ee042fcdb8d5c0a63f7c

Discovery topic: MakeHuman

Description: convert MakeHuman to Godot4

Reported license: AGPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"shader": ["addons/humanizer/scripts/overlay/fill_seams.gdshader", "addons/humanizer/scripts/overlay/gradient_map.gdshader", "addons/humanizer/scripts/overlay/mask.gdshader", "addons/humanizer/scripts/overlay/seeded_random.gdshaderinc", "addons/humanizer/scripts/overlay/whorley_GPU.gdshader", "addons/humanizer/scripts/shader/gradient_map.gdshader"], "rig": ["addons/humanizer/scripts/resources/equipment_rig_config.gd", "addons/humanizer/scripts/resources/humanizer_rig.gd", "addons/humanizer/scripts/services/rig.gd"], "face": ["addons/humanizer/data/animations/face_animation_tree.tscn", "addons/humanizer/data/animations/face_clip_library.tres", "addons/humanizer/data/animations/face_poses.glb", "addons/humanizer/data/animations/face_poses.glb.import", "addons/humanizer/data/animations/mh_face_poses.blend", "addons/humanizer/data/animations/mh_face_poses.blend.import", "addons/humanizer/scripts/core/surface_uv_unwrapper.gd"], "lip": ["addons/humanizer/data/animations/face_clip_library.tres"]}

## Source file: addons/humanizer/scripts/services/rig.gd

{
  "path": "addons/humanizer/scripts/services/rig.gd",
  "blob_sha": "ba5a2c07e6d5ea0deeaf5ec9069d473efe00e170",
  "sha256": "e7094976f7bc849e72999e37d789a4af010df2755194ffa759dfe80a234dc9a1",
  "lines": 146,
  "bytes": 6080,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 31
  },
  "url": "https://github.com/NitroxNova/humanizer/blob/21323ee75b520419f305ee042fcdb8d5c0a63f7c/addons/humanizer/scripts/services/rig.gd",
  "function_signature_candidates": []
}

```text

@tool
extends Resource
class_name HumanizerRigService

static func get_rig(rig_name:String)->HumanizerRig:
	return HumanizerRegistry.rigs[rig_name]

static func get_skeleton_3D(skeleton_data:Dictionary):
	var skeleton = Skeleton3D.new()
	skeleton.name = "GeneralSkeleton"
	rebuild_skeleton_3D(skeleton,skeleton_data)
	skeleton.set_unique_name_in_owner(true)
	return skeleton

static func rebuild_skeleton_3D(skeleton3D:Skeleton3D,skeleton_data:Dictionary):
	skeleton3D.clear_bones()
	for bone_name in skeleton_data:
		skeleton3D.add_bone(bone_name)
	
	#because root bone is added after
	for bone_name in skeleton_data:
		var bone_data = skeleton_data[bone_name]
		var bone_id = skeleton3D.find_bone(bone_name)
		if "parent" in bone_data:
			var parent_id = skeleton3D.find_bone(bone_data.parent)
			skeleton3D.set_bone_parent(bone_id,parent_id)
	adjust_skeleton_3D(skeleton3D,skeleton_data)

static func adjust_skeleton_3D(skeleton3D:Skeleton3D,skeleton_data:Dictionary):
	for bone_name in skeleton_data:
		var bone_id = skeleton3D.find_bone(bone_name)
		var bone_data = skeleton_data[bone_name]
		var local_pos = bone_data.global_pos
		if "parent" in bone_data:
			var parent_id = skeleton3D.find_bone(bone_data.parent)
			var parent_xform = skeleton3D.get_bone_global_rest(parent_id)
			local_pos = local_pos * parent_xform
		var bone_xform = Transform3D(bone_data.local_xform)
		bone_xform.origin = local_pos
		skeleton3D.set_bone_rest(bone_id, bone_xform)
		skeleton3D.reset_bone_pose(bone_id)
	
static func adjust_bone_positions(skeleton_data:Dictionary,rig:HumanizerRig,helper_vertex:PackedVector3Array,equipment:Dictionary,mesh_arrays:Dictionary):
	var asset_bone_positions = []
	asset_bone_positions.resize(skeleton_data.size())
	for equip in equipment.values():
		var equip_type = equip.get_type()
		if equip_type.is_rigged():
			var mhclo : MHCLO = HumanizerResourceService.load_resource(equip_type.mhclo_path)
			for bone_config in equip_type.rig_config.config:
				var bone_id = skeleton_data.keys().find(bone_config.name)
				if bone_id > -1:
					asset_bone_positions[bone_id] = get_asset_bone_position(mesh_arrays[equip.type],mhclo,bone_config)				
	
	var skeleton_config = rig.config
	var bone_id = 0
	for bone_name in skeleton_data:
		var bone_pos = Vector3.ZERO
		if bone_name == 'Root':
			bone_pos = Vector3.ZERO  # Root should always be at origin
		## manually added bones won't be in the config
		elif skeleton_config.size() < bone_id + 1:
			bone_pos = asset_bone_positions[bone_id]
		else:
			bone_pos = get_bone_position_from_config(rig,bone_id,"head",helper_vertex)
		skeleton_data[bone_name].global_pos = bone_pos
		bone_id += 1

static func get_bone_position_from_config(rig,bone_id,head:String,helper_vertex):
	var bone_data = rig.config[bone_id]
	var bone_pos = Vector3.ZERO
	for vid in bone_data[head]:
		bone_pos += helper_vertex[int(vid)]
	bone_pos /= bone_data[head].size()
	return bone_pos
				
#static func init_skeleton_data(rig: HumanizerRig,retargeted:bool)->Dictionary:
static func init_skeleton_data(skeleton:Skeleton3D)->Dictionary:
	var skeleton_data = {}
	#var skeleton : Skeleton3D
	#if retargeted:
		#skeleton = rig.load_retargeted_skeleton()
	#else:
		#skeleton = rig.load_skeleton()
	##
	for bone_id in skeleton.get_bone_count():
		var bone_name = skeleton.get_bone_name(bone_id)
		var bone_data = {}
		var parent_id = skeleton.get_bone_parent(bone_id)
		if parent_id != -1:
			bone_data.parent = skeleton.get_bone_name(parent_id)
		bone_data.local_xform = skeleton.get_bone_rest(bone_id)
		bone_data.global_pos = skeleton.get_bone_global_rest(bone_id).origin
		skeleton_data[bone_name] = bone_data
	
	return skeleton_data

static func get_motion_scale(rig_name:String, helper_vertex:PackedVector3Array):
	var base_motion_scale = _get_base_motion_scale(rig_name)
	var hips_height = HumanizerBodyService.get_hips_height(helper_vertex)
	var base_hips_height = HumanizerBodyService.get_hips_height(HumanizerTargetService.basis)
	return base_motion_scale * (hips_height  / base_hips_height)

static func _get_base_motion_scale(rig_name:String):
	var sk: Skeleton3D
	var rig = get_rig(rig_name)
	sk = rig.load_skeleton()
	return sk.motion_scale

static func skeleton_add_rigged_equipment(equipment:HumanizerEquipment, sf_arrays:Array,skeleton_data:Dictionary):
	var mhclo : MHCLO = HumanizerResourceService.load_resource(equipment.get_type().mhclo_path)
	var equip_type = equipment.get_type()
	for bone_id in equip_type.rig_config.config.size():
		var bone_config = equip_type.rig_config.config[bone_id]
		var bone_name = bone_config.name
		skeleton_data[bone_name] = {}
		skeleton_data[bone_name].global_pos = get_asset_bone_position(sf_arrays,mhclo,bone_config)
		skeleton_data[bone_name].local_xform = bone_config.transform
		var parent_name = get_rigged_parent_bone(equip_type.rig_config,bone_id,skeleton_data)
		if parent_name != null:
			skeleton_data[bone_name].parent = parent_name
		else:
			printerr(" no valid attach bones defined for " + equipment.type)
	
static func get_asset_bone_position(sf_arrays:Array,mhclo:MHCLO,bone_config:Dictionary):
	#print(bone_config.vertices.ids)
	var v1 = sf_arrays[Mesh.ARRAY_VERTEX][mhclo.mh2gd_index[bone_config.vertices.ids[0]][0]]
	var v2 = sf_arrays[Mesh.ARRAY_VERTEX][mhclo.mh2gd_index[bone_config.vertices.ids[1]][0]]
	var bone_pos = 0.5 * (v1+v2) + bone_config.vertices.offset
	return bone_pos

static func skeleton_remove_rigged_equipment(equipment:HumanizerEquipment,skeleton_data:Dictionary):
	#var mhclo : MHCLO = HumanizerResourceService.load_resource(equipment.get_type().mhclo_path)
	for bone_config in equipment.get_type().rig_config.config:
		var bone_name = bone_config.name
		skeleton_data.erase(bone_name)			
				
static func get_rigged_parent_bone(rig_config:HumanizerEquipmentRigConfig,bone_id:int,skeleton_data:Dictionary):
	var bone_config = rig_config.config[bone_id]
	if bone_config.parent == -1:
		for parent_name in rig_config.attach_bones:
			if parent_name in skeleton_data:
				return parent_name
	else:
		return rig_config.config[bone_config.parent].name
		 


```

## Source file: addons/humanizer/scripts/overlay/mask.gdshader

{
  "path": "addons/humanizer/scripts/overlay/mask.gdshader",
  "blob_sha": "81525b4e603a4aa4ba6e40d69c1aa2c6065c2c55",
  "sha256": "d8cf3d0ff8c7e55d325446ee420223bff67dd0003449c2be2126851f844401ec",
  "lines": 31,
  "bytes": 1341,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/NitroxNova/humanizer/blob/21323ee75b520419f305ee042fcdb8d5c0a63f7c/addons/humanizer/scripts/overlay/mask.gdshader",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "void fragment(){"
    }
  ]
}

```text

shader_type canvas_item;
render_mode unshaded;

//uniform sampler2D gradient: hint_default_black;
uniform float mix_amount = 1.0;
uniform sampler2D mask_image;
uniform sampler2D base_image;
//uniform sampler2D SCREEN_TEXTURE: hint_screen_texture,filter_linear_mipmap;

void fragment(){
	//vec4 input_color = texture(SCREEN_TEXTURE, SCREEN_UV);
	vec4 base_color = texture(base_image, UV);
	vec4 mask_color = texture(mask_image,UV); //should be greyscale, alpha = 1
	//vec4 overlay_color = texture(overlay_image,UV); //should be greyscale, alpha = 1

	float greyscale_value = (mask_color.r + mask_color.g + mask_color.b) / 3.0;
	//vec4 sampled_color = mix(base_color,overlay_color,greyscale_value);
	//sampled_color = (sampled_color * overlay_color.a) +  (base_color * (1.0-overlay_color.a));
	vec4 sampled_color = base_color;
	sampled_color.a = base_color.a * greyscale_value;
	COLOR = sampled_color;
	//COLOR = vec4(base_color.a,base_color.a,base_color.a,1.0);
	//COLOR = base_color;
	//vec4 sampled_color = input_color * mask_color;
	//float greyscale_value = (input_color.r + input_color.g + input_color.b) / 3.0;
	//vec3 sampled_color = texture(gradient, vec2(greyscale_value, 0.0)).rgb;
	//COLOR = mix(input_color, sampled_color, mix_amount);

	//COLOR.rgb = mix(input_color.rgb, sampled_color, mix_amount);
	//COLOR.a = input_color.a;
}

```

## Source file: addons/humanizer/scripts/resources/humanizer_rig.gd

{
  "path": "addons/humanizer/scripts/resources/humanizer_rig.gd",
  "blob_sha": "4a845a971b5fb5cca0048375388355d08b7d0dbb",
  "sha256": "318a56a9e884a5c2c39c36f38123d2c4a39cb7ef8ba6e0a3d2ea021aba977fdb",
  "lines": 25,
  "bytes": 852,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/NitroxNova/humanizer/blob/21323ee75b520419f305ee042fcdb8d5c0a63f7c/addons/humanizer/scripts/resources/humanizer_rig.gd",
  "function_signature_candidates": []
}

```text

@tool
extends Resource
class_name HumanizerRig

@export var skeleton : PackedScene #saving the skeleton3d node doesnt work, it gets reset when you open it
@export var config : Array #bone position info, from json
@export var weights : Array #weights and bones arrays, indexed to helper vertex
@export var retargeted : bool = false

func load_skeleton() -> Skeleton3D:
	return skeleton.instantiate()

func init_skeleton_data():
	var skel3D = load_skeleton()
	return HumanizerRigService.init_skeleton_data(skel3D)

#func load_bone_weights() -> Dictionary:
	#var weights: Dictionary = HumanizerResourceService.load_resource(mh_weights_path).weights
	#for in_name:String in weights.keys():
		#if ':' not in in_name:
			#continue
		#var out_name = in_name.replace(":", "_")
		#weights[out_name] = weights[in_name]
		#weights.erase(in_name)
	#return weights


```

## Source file: addons/humanizer/scripts/overlay/fill_seams.gdshader

{
  "path": "addons/humanizer/scripts/overlay/fill_seams.gdshader",
  "blob_sha": "e1e1be5eec366c45f84f09aa08b758d98e0e2dbe",
  "sha256": "003ee277c8e564ecbc3ae76c3cd8f0f400589e3ad40ffa3aec07af6ef7ea2319",
  "lines": 38,
  "bytes": 906,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/NitroxNova/humanizer/blob/21323ee75b520419f305ee042fcdb8d5c0a63f7c/addons/humanizer/scripts/overlay/fill_seams.gdshader",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "void vertex() {"
    },
    {
      "line": 9,
      "text": "void fragment() {"
    },
    {
      "line": 35,
      "text": "//void light() {"
    }
  ]
}

```text

shader_type canvas_item;

uniform int margin = 1;

void vertex() {
	// Called for every vertex the material is visible on.
}

void fragment() {
	if (COLOR.a == 0.0){
		vec2 closest_pixel = vec2(999,999);
		vec2 pixel_size = 1.0 / vec2(textureSize(TEXTURE, 0));
	    for(int y = -margin; y <= margin; y++) 
		{
		    for(int x = -margin; x <= margin; x++)
		    {
		        vec2 pixel_off = vec2(float(x), float(y));
				vec2 off_uv = UV + pixel_off * pixel_size;
		        vec4 tex = texture(TEXTURE, off_uv);
		        if (tex.a != 0.0)
				{
					if (distance(UV,off_uv) < distance(UV,closest_pixel))
					{
						closest_pixel = off_uv;
					}
				}
		    }
		}
		
		COLOR = texture(TEXTURE, closest_pixel);
		COLOR.a = 1.0;
	}
}

//void light() {
//	// Called for every pixel for every light affecting the CanvasItem.
//	// Uncomment to replace the default light processing function with this one.
//}


```

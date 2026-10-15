# Facial rendering source scan: guilhermicas/godot-3d-character-creator

Repository: https://github.com/guilhermicas/godot-3d-character-creator

Pinned source revision: 31ad3828829e66a5e4253db116b08a0e3ae72a3f

Discovery topic: character creator 3D

Description: A simple and straight forward blender -> godot character creator.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"shader": ["godot/3d-character-creator/addons/character_creator/ui_components/loading_spinner.gdshader", "godot/3d-character-creator/addons/character_creator/ui_components/loading_spinner.gdshader.uid"]}

## Source file: godot/3d-character-creator/addons/character_creator/ui_components/loading_spinner.gdshader

{
  "path": "godot/3d-character-creator/addons/character_creator/ui_components/loading_spinner.gdshader",
  "blob_sha": "a3c0f5e06899ec7e8425b96d52f79134f1544fd8",
  "sha256": "9e92012f95825d33aa8f5eed4e3d5ce6e1de238f16014f1355a573e1263f79f8",
  "lines": 37,
  "bytes": 1245,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1
  },
  "url": "https://github.com/guilhermicas/godot-3d-character-creator/blob/31ad3828829e66a5e4253db116b08a0e3ae72a3f/godot/3d-character-creator/addons/character_creator/ui_components/loading_spinner.gdshader",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "void fragment() {"
    }
  ]
}

```text

shader_type canvas_item;

// Rotating gradient spinner (3/4 circle with alpha gradient)
uniform float rotation_speed : hint_range(0.0, 10.0) = -2.0;

void fragment() {
	vec2 center = vec2(0.5, 0.5);
	vec2 pos = UV - center;

	// Calculate angle and distance from center
	float angle = atan(pos.y, pos.x);
	float dist = length(pos);

	// Normalize angle to 0-2π and rotate based on time
	float normalized_angle = angle + PI;
	float rotated_angle = mod(normalized_angle + TIME * rotation_speed, TAU);

	// Create ring (donut shape)
	float outer_radius = 0.45;
	float inner_radius = 0.30;
	float ring = smoothstep(inner_radius - 0.02, inner_radius, dist) *
	             smoothstep(outer_radius + 0.02, outer_radius, dist);

	// Create 3/4 circle (270 degrees)
	float arc_coverage = TAU * 0.75; // 3/4 of circle
	float arc = smoothstep(0.0, 0.1, mod(rotated_angle, TAU)) *
	            smoothstep(arc_coverage + 0.1, arc_coverage, mod(rotated_angle, TAU));

	// Gradient alpha along the arc
	float alpha_gradient = smoothstep(0.0, arc_coverage, mod(rotated_angle, TAU));

	// Combine ring, arc, and gradient
	float final_alpha = ring * arc * alpha_gradient;

	// Output white color with gradient alpha
	COLOR = vec4(1.0, 1.0, 1.0, final_alpha);
}


```

## Source file: blender/cc_exporter_addon.py

{
  "path": "blender/cc_exporter_addon.py",
  "blob_sha": "d4a852c6206a896cbfa15832c3ad557466fb097a",
  "sha256": "c2d8d62f59f095ff31718a0e9b7cb64e22814bb77e5b66fa65e9425ef9921718",
  "lines": 225,
  "bytes": 8567,
  "selection_score": 0,
  "keyword_line_counts": {
    "lbs": 1,
    "rig": 2
  },
  "url": "https://github.com/guilhermicas/godot-3d-character-creator/blob/31ad3828829e66a5e4253db116b08a0e3ae72a3f/blender/cc_exporter_addon.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def ensure_uuid(obj):"
    },
    {
      "line": 24,
      "text": "def selection_context(objs):"
    },
    {
      "line": 27,
      "text": "def __init__(self, target_objs):"
    },
    {
      "line": 32,
      "text": "def __enter__(self):"
    },
    {
      "line": 38,
      "text": "def __exit__(self, *args):"
    },
    {
      "line": 47,
      "text": "def gather_descendants(root, include_root=True):"
    },
    {
      "line": 55,
      "text": "def make_foldername(obj):"
    },
    {
      "line": 59,
      "text": "def export_glb(obj, folder):"
    },
    {
      "line": 96,
      "text": "def process_children(parent, folder):"
    },
    {
      "line": 110,
      "text": "def process_hierarchy(top_objects, export_root):"
    },
    {
      "line": 151,
      "text": "def execute(self, context):"
    },
    {
      "line": 192,
      "text": "def execute(self, context):"
    },
    {
      "line": 203,
      "text": "def draw(self, context):"
    },
    {
      "line": 217,
      "text": "def register():"
    },
    {
      "line": 221,
      "text": "def unregister():"
    }
  ]
}

```text

# NOTE: Windows has a max path length of 260 chars, if the tree exceeds this, export will fail. (why Windows, why)
bl_info = {
    "name": "3D Character Creator Exporter",
    "author": "Gui",
    "version": (0, 1),
    "blender": (5, 0, 0),
    "location": "Side Panel",
    "description": "Export CC_/CCC_ structured objects into folder tree of GLBs",
    "category": "Import-Export",
}

import bpy
from pathlib import Path
import uuid
from bpy.props import StringProperty, BoolProperty
from bpy.types import Operator, Panel, PropertyGroup

# ---------- Utility functions ----------

def ensure_uuid(obj):
    """Ensure object has a persistent short UUID (8 chars) in CC_id custom property."""
    if not obj.get("CC_id"): obj["CC_id"] = str(uuid.uuid4())[:8]

def selection_context(objs):
    """Context manager for safe selection state management."""
    class SelectionState:
        def __init__(self, target_objs):
            self.objs = target_objs
            self.state = {o: o.select_get() for o in bpy.context.scene.objects}
            self.active = bpy.context.view_layer.objects.active

        def __enter__(self):
            for o in bpy.context.scene.objects: o.select_set(False)
            for o in self.objs: o.select_set(True)
            if self.objs: bpy.context.view_layer.objects.active = self.objs[0]
            return self

        def __exit__(self, *args):
            for o, sel in self.state.items():
                try: o.select_set(sel)
                except: pass
            try: bpy.context.view_layer.objects.active = self.active
            except: pass

    return SelectionState(objs)

def gather_descendants(root, include_root=True):
    """Recursively gather root and non-CC_/CCC_ descendants."""
    result = [root] if include_root else []
    for child in root.children:
        if not (child.name.startswith("CC_") or child.name.startswith("CCC_")):
            result.extend(gather_descendants(child))
    return result

def make_foldername(obj):
    """Generate folder name with format: name_CC_id_shortuid"""
    return f"{obj.name}_CC_id_{obj.get('CC_id', 'unknown')}"

def export_glb(obj, folder):
    """Export object and descendants as GLB (filename is just object name)."""
    objs = gather_descendants(obj)
    if not objs: return None

    folder.mkdir(parents=True, exist_ok=True)
    out_path = folder / f"{obj.name}.glb"

    # Store original visibility states for objects and their data
    # This is necessary because hidden objects are not exported
    # So we temporarily unhide them
    visibility_state = {}

    for o in objs:
        visibility_state[o] = o.hide_get() # Store object visibility
        o.hide_set(False) # Unhide object

    try:
        with selection_context(objs):
            bpy.ops.export_scene.gltf(
                filepath=str(out_path),
                export_format='GLB',
                use_selection=True,
                export_apply=True,
                export_extras=True
            )
        print(f"[CC Exporter] Exported {obj.name} -> {out_path}")
        result = out_path
    except Exception as e:
        print(f"[CC Exporter] Export failed for {obj.name}: {e}")
        result = None
    finally:
        # Restore original visibility states
        for o, previous_hide_state in visibility_state.items(): o.hide_set(previous_hide_state)

    return result

def process_children(parent, folder):
    """Process children: CC_ objects export, CCC_ objects create subfolders."""
    for child in parent.children:
        name = child.name
        if name.startswith("CC_"):
            child_folder = folder / make_foldername(child)
            export_glb(child, child_folder)
            # Process nested CCC_ collections under this CC_
            for sub in child.children:
                if sub.name.startswith("CCC_"):
                    process_children(sub, child_folder / make_foldername(sub))
        elif name.startswith("CCC_"):
            process_children(child, folder / make_foldername(child))

def process_hierarchy(top_objects, export_root):
    """Process top-level objects into folder structure."""
    base = export_root / "root" if len(top_objects) > 1 else export_root
    base.mkdir(parents=True, exist_ok=True)

    for obj in top_objects:
        name = obj.name
        if name.startswith("CC_"):
            comp_folder = base / make_foldername(obj)
            export_glb(obj, comp_folder)
            process_children(obj, comp_folder)
        elif name.startswith("CCC_"):
            process_children(obj, base / make_foldername(obj))
        else:
            print(f"[CC Exporter] Skipping non-CC_/CCC_ top-level: {name}")

# ---------- Blender UI Components ----------

class CCExporterProperties(PropertyGroup):
    export_path: StringProperty(
        name="Export Path",
        description="Path where the CC glb folders will be created",
        default="//cc_export",
        subtype='DIR_PATH'
    )
    add_root_folder: BoolProperty(
        name="Add folder",
        description="Add a root 'character_config' folder under the chosen export path",
        default=True
    )
    delete_and_recreate: BoolProperty(
        name="Delete and recreate",
        description="Delete everything at the destination and recreate before exporting",
        default=True
    )

class CC_OT_export(Operator):
    bl_idname = "cc.export"
    bl_label = "Export"
    bl_description = "Export CC_ components into GLB folders"

    def execute(self, context):
        props = context.scene.cc_exporter_props
        base = Path(bpy.path.abspath(props.export_path))
        target = base / "character_config" if props.add_root_folder else base

        # Safety check and cleanup
        if props.delete_and_recreate:
            if len(str(target.resolve())) <= 10:
                self.report({'ERROR'}, f"Refusing to delete unsafe path: {target}")
                return {'CANCELLED'}
            if target.exists():
                try:
                    from shutil import rmtree
                    # Only delete CC_/CCC_ folders - preserve characters/, config files, etc.
                    for item in target.iterdir():
                        if item.is_dir() and (item.name.startswith("CCC_") or item.name.startswith("CC_")):
                            rmtree(item)
                except Exception as e:
                    self.report({'ERROR'}, f"Failed to delete destination: {e}")
                    return {'CANCELLED'}

        # Assign UUIDs to CC_/CCC_ objects
        for obj in context.scene.objects:
            if obj.name.startswith(("CC_", "CCC_")):
                ensure_uuid(obj)

        # Process top-level objects
        top_objects = [o for o in context.scene.objects if not o.parent]
        if not top_objects or len(top_objects) > 1 or not top_objects[0].name.startswith("CCC_"):
            self.report({'WARNING'}, "The top level object must be a single CCC_")
            return {'CANCELLED'}

        process_hierarchy(top_objects, target)
        self.report({'INFO'}, "CC export complete.")
        return {'FINISHED'}

class CC_OT_propagate_shape_keys(Operator):
    bl_idname = "cc.propagate_shape_keys"
    bl_label = "Propagate Shape Keys"
    bl_description = "Placeholder: propagate shape keys (no-op)"

    def execute(self, context):
        self.report({'INFO'}, "Propagate Shape Keys is a placeholder (no action implemented).")
        return {'FINISHED'}

class CC_PT_exporter_panel(Panel):
    bl_label = "3D Character Creator Exporter"
    bl_idname = "CC_PT_exporter_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'CC Exporter'

    def draw(self, context):
        layout = self.layout
        props = context.scene.cc_exporter_props
        layout.prop(props, "export_path")
        layout.prop(props, "add_root_folder")
        layout.prop(props, "delete_and_recreate")
        row = layout.row()
        row.operator("cc.export", text="Export", icon='EXPORT')
        row.operator("cc.propagate_shape_keys", text="Propagate Shape Keys", icon='SHAPEKEY_DATA')

# ---------- Registration ----------

classes = (CCExporterProperties, CC_OT_export, CC_OT_propagate_shape_keys, CC_PT_exporter_panel)

def register():
    for cls in classes: bpy.utils.register_class(cls)
    bpy.types.Scene.cc_exporter_props = bpy.props.PointerProperty(type=CCExporterProperties)

def unregister():
    for cls in reversed(classes): bpy.utils.unregister_class(cls)
    del bpy.types.Scene.cc_exporter_props

if __name__ == "__main__": register()

```

## Source file: godot/3d-character-creator/addons/character_creator/editor/dock.gd

{
  "path": "godot/3d-character-creator/addons/character_creator/editor/dock.gd",
  "blob_sha": "a9df28ba9888594ac0084712753b2889ea6ce83a",
  "sha256": "cc9c18f5d6f4890e9b43f349726a19438266888fce3e954e44f90cde81d1fa25",
  "lines": 228,
  "bytes": 7423,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/guilhermicas/godot-3d-character-creator/blob/31ad3828829e66a5e4253db116b08a0e3ae72a3f/godot/3d-character-creator/addons/character_creator/editor/dock.gd",
  "function_signature_candidates": []
}

```text

@tool
extends VBoxContainer

var plugin: EditorPlugin

# References
@onready var path_container: HBoxContainer = $PathConfig
@onready var path_edit: LineEdit = $PathConfig/PathEdit
@onready var browse_btn: Button = $PathConfig/BrowseButton
@onready var rescan_btn: Button = $PathConfig/RescanButton
@onready var tabs: TabBar = $Tabs
@onready var tab_container: MarginContainer = $Content
@onready var global_view: Control = $Content/GlobalView
@onready var local_view: Control = $Content/LocalView

# Current state
var global_config: CharacterComponent
var blender_export_path: String = ""

func _ready() -> void:
	tabs.tab_changed.connect(_on_tab_changed)
	browse_btn.pressed.connect(_on_browse_pressed)
	rescan_btn.pressed.connect(_on_rescan_pressed)
	path_edit.text_submitted.connect(_on_path_submitted)

	_load_saved_path()
	_refresh_ui()

	# Default to global tab
	global_view.visible = 1
	local_view.visible = 0

func _load_saved_path() -> void:
	# Load from EditorSettings (persisted across sessions)
	if plugin and plugin.get_editor_interface():
		var settings := plugin.get_editor_interface().get_editor_settings()
		if settings.has_setting("character_creator/blender_export_path"):
			blender_export_path = settings.get_setting("character_creator/blender_export_path")
			path_edit.text = blender_export_path
			_load_global_config()

func _save_path() -> void:
	# Save to EditorSettings
	if plugin and plugin.get_editor_interface():
		var settings := plugin.get_editor_interface().get_editor_settings()
		if not settings.has_setting("character_creator/blender_export_path"):
			settings.set_setting("character_creator/blender_export_path", "")
			settings.set_initial_value("character_creator/blender_export_path", "", false)
		settings.set_setting("character_creator/blender_export_path", blender_export_path)

func _on_browse_pressed() -> void:
	var file_dialog := EditorFileDialog.new()
	file_dialog.file_mode = EditorFileDialog.FILE_MODE_OPEN_DIR
	file_dialog.access = EditorFileDialog.ACCESS_RESOURCES
	file_dialog.current_dir = blender_export_path if blender_export_path != "" else "res://"
	file_dialog.dir_selected.connect(func(dir: String):
		blender_export_path = dir
		path_edit.text = dir
		_save_path()
		_load_global_config()
		_refresh_ui()
		file_dialog.queue_free()
	)
	file_dialog.canceled.connect(func(): file_dialog.queue_free())
	add_child(file_dialog)
	file_dialog.popup_centered(Vector2i(800, 600))

func _on_path_submitted(new_path: String) -> void:
	blender_export_path = new_path
	_save_path()
	_load_global_config()
	_refresh_ui()

func _on_rescan_pressed() -> void:
	if blender_export_path == "":
		push_warning("No export path set")
		return

	_scan_and_build_global_config()
	_load_global_config()
	_refresh_ui()

func _load_global_config() -> void:
	if blender_export_path == "":
		return

	var config_path := blender_export_path.path_join("global_config.tres")
	if FileAccess.file_exists(config_path):
		global_config = load(config_path)
		if global_view:
			global_view.set_config(global_config, false)
	else:
		push_warning("Global config not found at: " + config_path)

func _on_tab_changed(tab: int) -> void:
	if global_view and local_view:
		global_view.visible = (tab == 0)
		local_view.visible = (tab == 1)

func _refresh_ui() -> void:
	if global_config and global_view:
		global_view.set_config(global_config, false)

	if blender_export_path != "" and local_view:
		local_view.set_export_path(blender_export_path, global_config)

func _scan_and_build_global_config() -> void:
	if not DirAccess.dir_exists_absolute(blender_export_path):
		push_error("Blender export path does not exist: " + blender_export_path)
		return

	var config_path := blender_export_path.path_join("global_config.tres")

	# Load existing or create new
	if FileAccess.file_exists(config_path):
		global_config = load(config_path)
		if global_config == null:
			push_error("Failed to load global_config.tres")
			global_config = CharacterComponent.new()
	else:
		global_config = CharacterComponent.new()

	# Find the top-level CCC_
	var dirs := DirAccess.get_directories_at(blender_export_path)
	var first_dir := ""
	for d in dirs:
		if d.begins_with("CCC_"):
			first_dir = d
			break

	if first_dir == "":
		push_error("No top-level CCC_ folder found in: " + blender_export_path)
		return

	# Scan and merge
	var scanned := _scan_component(blender_export_path.path_join(first_dir))
	_merge_scanned_into_config(global_config, scanned)

	# Save
	var save_result := ResourceSaver.save(global_config, config_path)
	if save_result == OK:
		print("✅ Global config saved: " + config_path)
		_update_project_settings(config_path)
	else:
		push_error("Failed to save global_config.tres")

func _scan_component(path: String) -> CharacterComponent:
	var dir := DirAccess.open(path)
	if dir == null:
		push_error("Cannot open: " + path)
		return null

	var comp := CharacterComponent.new()
	var folder_name := path.get_file()

	# Parse name and ID from folder
	if folder_name.contains("_CC_id_"):
		var parts := folder_name.split("_CC_id_")
		comp.name = parts[0]
		comp.cc_id = parts[1] if parts.size() > 1 else ""
	else:
		comp.name = folder_name
		comp.cc_id = ""

	# Find GLB file
	dir.list_dir_begin()
	var f := dir.get_next()
	while f != "":
		if not dir.current_is_dir() and f.ends_with(".glb"):
			comp.glb_path = path.path_join(f)
			break
		f = dir.get_next()
	dir.list_dir_end()

	# Recursively scan children
	dir.list_dir_begin()
	f = dir.get_next()
	while f != "":
		if dir.current_is_dir() and (f.begins_with("CC_") or f.begins_with("CCC_")):
			var child := _scan_component(path.path_join(f))
			if child:
				comp.children.append(child)
		f = dir.get_next()
	dir.list_dir_end()

	return comp

func _merge_scanned_into_config(existing: CharacterComponent, scanned: CharacterComponent) -> void:
	# Update core properties from scan
	existing.name = scanned.name
	existing.glb_path = scanned.glb_path
	existing.cc_id = scanned.cc_id

	# Build map of existing children by cc_id
	var existing_map := {}
	for child in existing.children:
		if child.cc_id != "":
			existing_map[child.cc_id] = child

	# Merge scanned children with existing (preserving user edits)
	var merged: Array[CharacterComponent] = []
	for scanned_child in scanned.children:
		if scanned_child.cc_id != "" and existing_map.has(scanned_child.cc_id):
			# Child already exists, merge recursively (keeps display_name, metadata)
			var existing_child := existing_map[scanned_child.cc_id] as CharacterComponent
			_merge_scanned_into_config(existing_child, scanned_child)
			merged.append(existing_child)
			existing_map.erase(scanned_child.cc_id)
		else:
			# New child from scan
			merged.append(scanned_child)

	existing.children = merged

func _update_project_settings(_config_path: String) -> void:
	# Only store blender_export_path in ProjectSettings
	# Global config is always at {blender_export_path}/character_config/global_config.tres
	if not ProjectSettings.has_setting("character_creator/blender_export_path"):
		ProjectSettings.set_setting("character_creator/blender_export_path", "")
		ProjectSettings.set_initial_value("character_creator/blender_export_path", "")

	ProjectSettings.set_setting("character_creator/blender_export_path", blender_export_path)

	var save_result := ProjectSettings.save()
	if save_result == OK:
		print("✅ ProjectSettings updated")
	else:
		push_warning("Failed to save ProjectSettings")


```

## Source file: godot/3d-character-creator/addons/character_creator/utils/ui_utils.gd

{
  "path": "godot/3d-character-creator/addons/character_creator/utils/ui_utils.gd",
  "blob_sha": "18d506fbe40d6f2cd9754585b88273e6997b2a14",
  "sha256": "4165ee5a48a9a3516b3098b0c94784bb004d0c68ca61b149860239b521e4a5a7",
  "lines": 8,
  "bytes": 198,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/guilhermicas/godot-3d-character-creator/blob/31ad3828829e66a5e4253db116b08a0e3ae72a3f/godot/3d-character-creator/addons/character_creator/utils/ui_utils.gd",
  "function_signature_candidates": []
}

```text

class_name UIUtils

## Utility functions for UI operations

## Clear all children from a node
static func clear_children(node: Node) -> void:
	for child in node.get_children():
		child.queue_free()


```

# Facial rendering source scan: Meshcapade/SMPL_blender_addon

Repository: https://github.com/Meshcapade/SMPL_blender_addon

Pinned source revision: 09bd761956b2a8da8377a09451d6d983db8a853d

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md"]

Topic paths: {}

## Source file: reload.py

{
  "path": "reload.py",
  "blob_sha": "7eae1bd8c4d85710e6c19092a6effab0246b6d68",
  "sha256": "5a281ee92d9516ca553d7acf1ef0dd3fbb9bd5ee949e69516f8aac67ae4e3b21",
  "lines": 26,
  "bytes": 602,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Meshcapade/SMPL_blender_addon/blob/09bd761956b2a8da8377a09451d6d983db8a853d/reload.py",
  "function_signature_candidates": []
}

```text

import bpy
import importlib
import sys

addon_name = "meshcapade"
module = "meshcapade_addon"
parts_to_reload = [
    "blender",
    "globals",
    "operators",
    "properties",
    "meshcapade_addon",
    "ui",
]

for part in parts_to_reload:
    # Unregister the addon
    if addon_name in bpy.context.preferences.addons:
        bpy.ops.preferences.addon_disable(module=addon_name)

    # Reload the addon modules
    module_name = f"{addon_name}.{module}.{part}"
    importlib.reload(sys.modules[module_name])

    # Register the addon again
    bpy.ops.preferences.addon_enable(module=addon_name)

```

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "21fde6157c190cca2958a7ffde78ef2d00d18620",
  "sha256": "f8ed844e19bd31a75bf2ae77160d56e9b4d68c03be588fd5056a833cb80e70cd",
  "lines": 21,
  "bytes": 474,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Meshcapade/SMPL_blender_addon/blob/09bd761956b2a8da8377a09451d6d983db8a853d/__init__.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def register():"
    },
    {
      "line": 20,
      "text": "def unregister():"
    }
  ]
}

```text

bl_info = {
    "name": "Meshcapade Blender Tools",
    "blender": (3, 5, 1),
    "category": "Meshcapade",
}

if "bpy" in locals():
    import importlib
    if "meshcapade_addon" in locals():
        importlib.reload(meshcapade_addon)
    else:
        from .meshcapade_addon import meshcapade_addon
else:
    import bpy
    from .meshcapade_addon import meshcapade_addon

def register():
    meshcapade_addon.register()

def unregister():
    meshcapade_addon.unregister()

```

## Source file: build/make_addon.py

{
  "path": "build/make_addon.py",
  "blob_sha": "1ed076131a644120fb80fc6d8eabad9a6b452c99",
  "sha256": "96b2cdfb7d08c9c15d5d2b09ec0d8818c017db3ab42989a53da97d822f32573f",
  "lines": 31,
  "bytes": 1209,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Meshcapade/SMPL_blender_addon/blob/09bd761956b2a8da8377a09451d6d983db8a853d/build/make_addon.py",
  "function_signature_candidates": []
}

```text

from zipfile import ZipFile, ZIP_DEFLATED
from os import walk, listdir
from datetime import date
from pathlib import Path
build_script_dir = Path(__file__).parent
today = date.today()
filedate = today.strftime("%Y%m%d")
filename = '{}_{}.zip'.format("smplx_blender", filedate)
root_folder = build_script_dir.parent
addon_folder = root_folder / "smplx_blender"
data_folder = addon_folder / "data"

with ZipFile(filename, 'w') as z:

    # Adding python and data files
    for root, _, files in walk(addon_folder):
        for file in files:
            # Python files or data files
            if Path(file).suffix in ['.py', '.pyd'] or Path(root) == data_folder:
                filepath = Path(root) / Path(file)
                print("Adding '{}'".format(filepath))
                z.write(filepath, compress_type=ZIP_DEFLATED)

    # Adding README, LICENSE, and other root .md files
    for file in listdir(root_folder):
        if Path(file).suffix == ".md":
            filepath = Path(root_folder) / Path(file)
            print("Adding '{}'".format(filepath))
            z.write(filepath, compress_type=ZIP_DEFLATED)

print("Finished making release '{}', saved to current directory".format(filename))


```

## Source file: meshcapade_addon/ui.py

{
  "path": "meshcapade_addon/ui.py",
  "blob_sha": "632f6f7e8bb4b24ec6b59ead2fc96942a129caee",
  "sha256": "da36ef4edf37720e892a5f7b32abd405954380a1274f7189bb7804e144699e29",
  "lines": 202,
  "bytes": 6418,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 2,
    "blend_shape": 1,
    "rig": 1,
    "face": 3
  },
  "url": "https://github.com/Meshcapade/SMPL_blender_addon/blob/09bd761956b2a8da8377a09451d6d983db8a853d/meshcapade_addon/ui.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def draw(self, context):"
    },
    {
      "line": 38,
      "text": "def draw(self, context):"
    },
    {
      "line": 51,
      "text": "def poll(cls, context):"
    },
    {
      "line": 57,
      "text": "def draw(self, context):"
    },
    {
      "line": 94,
      "text": "def draw(self, context):"
    },
    {
      "line": 124,
      "text": "def poll(cls, context):"
    },
    {
      "line": 130,
      "text": "def draw(self, context):"
    },
    {
      "line": 157,
      "text": "def draw(self, context):"
    },
    {
      "line": 174,
      "text": "def draw(self, context):"
    }
  ]
}

```text

import bpy
from .globals import (
    VERSION,
)

class SMPL_PT_Create(bpy.types.Panel):
    bl_label = "Create"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)
        row = col.row(align=True)
        col.prop(context.window_manager.smpl_tool, "SMPL_version")

        col.separator()
        col.prop(context.window_manager.smpl_tool, "gender")

        col.separator()
        col.operator("scene.create_avatar", text="Create")

        col.separator()
        col.label(text="Texture:")
        row = col.row(align=True)
        split = row.split(factor=0.75, align=True)
        split.prop(context.window_manager.smpl_tool, "texture")
        split.operator("object.set_texture", text="Set")


class SMPL_PT_Load(bpy.types.Panel):
    bl_label = "Load"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)
        col.operator("object.load_avatar")


class SMPL_PT_Shape(bpy.types.Panel):
    bl_label = "Shape"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    @classmethod
    def poll(cls, context):
        try:
            # Enable button only if mesh is not SMPLH because we can't support that right now
            return (bpy.context.object['SMPL_version'] != 'SMPLH')
        except: return False

    def draw(self, context):
        alert = context.window_manager.smpl_tool.alert
        layout = self.layout
        col = layout.column(align=True)
        col2 = layout.column(align=True)

        col.alert = alert
        col.prop(context.window_manager.smpl_tool, "height")
        col.prop(context.window_manager.smpl_tool, "weight")
        
        if alert:
            col.label(text="Measurements are outdated.")

        row = col2.row(align=True)
        split = row.split(factor=0.5, align=True)
        split.operator("object.random_body_shape")
        split.operator("object.random_face_shape")

        row2 = col2.row(align=True)
        split2 = row2.split(factor=0.5, align=True)
        split2.operator("object.reset_body_shape")
        split2.operator("object.reset_face_shape")

        row3 = col2.row(align=True)
        split3 = row3.split(factor=0.5, align=True)
        split3.prop(context.window_manager.smpl_tool, "random_body_mult")
        split3.prop(context.window_manager.smpl_tool, "random_face_mult")

        col2.separator()


class SMPL_PT_Pose(bpy.types.Panel):
    bl_label = "Pose"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)

        col.operator("object.load_pose")
        col.operator("object.reset_pose")

        col.separator()
        row = col.row(align=True)
        split = row.split(factor=0.6666, align=True)
        split.prop(context.window_manager.smpl_tool, "hand_pose")
        split.operator("object.set_hand_pose", text="Set")
        
        col.separator()
        col.operator("object.set_pose_correctives")
        col.operator("object.set_pose_correctives_for_sequence")
        col.operator("object.zero_out_pose_correctives")

        col.separator()
        col.operator("object.snap_to_ground_plane")


class SMPL_PT_Expression(bpy.types.Panel):
    bl_label = "Facial Expressions"
    bl_idname = "OBJECT_PT_expression_presets"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Meshcapade"

    @classmethod
    def poll(cls, context):
        try:
            # Enable button only if mesh is active object
            return ((bpy.context.object['SMPL_version'] != "SMPLH") and not ((bpy.context.object['SMPL_version'] == "SUPR") and (bpy.context.object['gender'] == "neutral")))
        except: return False

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)

        row = col.row(align=True)
        row.operator("object.set_expression_preset", text="Pleasant").preset = "pleasant"
        row.operator("object.set_expression_preset", text="Happy").preset = "happy"
        row.operator("object.set_expression_preset", text="Excited").preset = "excited"

        row2 = col.row(align=True)
        row2.operator("object.set_expression_preset", text="Sad").preset = "sad"
        row2.operator("object.set_expression_preset", text="Frustrated").preset = "frustrated"
        row2.operator("object.set_expression_preset", text="Angry").preset = "angry"

        col.separator()
        row3 = col.row(align=True)
        split = row3.split(factor=0.67, align=True)
        split.operator("object.random_expression_shape")
        split.operator("object.reset_expression_shape")


class SMPL_PT_Export(bpy.types.Panel):
    bl_label = "Export"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)

        col.label(text="Shape Keys (Blendshapes):")
        col.prop(context.window_manager.smpl_tool, "export_setting_shape_keys")

        col.separator()
        col.operator("object.export_unity_fbx")


class SMPL_PT_AdditionalTools(bpy.types.Panel):
    bl_label = "Additional Tools"
    bl_category = "Meshcapade"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)

        row = col.row(align=True)
        row.operator("object.modify_avatar", text="Modify Metadata")
        row.operator("object.read_avatar", text="Read Metadata")

        col.separator()
        row2 = col.row(align=True)
        row2.operator("object.write_pose_to_console")
        row2.operator("object.write_pose_to_json")

        col.separator()
        col.operator("object.fix_blend_shape_ranges", text="Fix Blendshape Ranges")

        (year, month, day) = VERSION
        col.label(text="Version: %s-%s-%s" % (year, month, day))


UI_CLASSES = [
    SMPL_PT_Create,
    SMPL_PT_Load,
    SMPL_PT_Shape,
    SMPL_PT_Pose,
    SMPL_PT_Expression,
    # SMPL_PT_Export,  # this just doesn't seem that necissary and it's taking up space in the UI
    SMPL_PT_AdditionalTools,
]


```

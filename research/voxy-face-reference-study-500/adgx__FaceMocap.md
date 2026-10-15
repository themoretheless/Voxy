# Facial rendering source scan: adgx/FaceMocap

Repository: https://github.com/adgx/FaceMocap

Pinned source revision: 96d77e6c4a2c8cc25daacb7b0c8f42311180e0f6

Discovery topic: facial mocap

Description: An add-on for facial motion capture for Blender 4.2 and later verisions

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["core/rig.py", "operators/rigging.py"]}

## Source file: core/rig.py

{
  "path": "core/rig.py",
  "blob_sha": "6534f5c11f51f668ff403454a175175af93a7f24",
  "sha256": "4446947e4587821b802e0933e3267e8b826cff1a0fe90950155dac30e4d0fb07",
  "lines": 124,
  "bytes": 3673,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 16,
    "face": 7
  },
  "url": "https://github.com/adgx/FaceMocap/blob/96d77e6c4a2c8cc25daacb7b0c8f42311180e0f6/core/rig.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def find_rig(context=None, name_rig: str =\"\"):"
    },
    {
      "line": 29,
      "text": "def create_bones(arm_data, adapt, tail_length, advance: bool = False):"
    },
    {
      "line": 53,
      "text": "def create_landmarkers_bone(arm_data, adapt):"
    },
    {
      "line": 84,
      "text": "def apply_translation(pose_bone, vec):"
    },
    {
      "line": 88,
      "text": "def apply_rotation(pose_bone, rotation):"
    },
    {
      "line": 93,
      "text": "def validate_target(rig, mappings):"
    },
    {
      "line": 113,
      "text": "def reset_rig_pose(rig):"
    }
  ]
}

```text

import bpy

from mathutils import Vector, Matrix

BASE_RIG_NAME = "RIG-FaceMocap_base"
TARGET_RIG_NAME = "RIG-FaceMocap_advanced_face"
LANDMARKS_RIG_NAME = "RIG-FaceMocap_Landmarkers"

from .config import FACE_MAPPING, ROTATION_BONES, LANDMARKERS_FACE_MAPPING
from .solver import traslation_to_bone_space

#it could make sense to use a distintion between the sorce, and target rig   
def find_rig(context=None, name_rig: str =""):
    """L'armatura del rig, o None. Cercata per nome o per armatura attiva"""

    if not name_rig.__len__():
        obj = bpy.data.objects.get(BASE_RIG_NAME)
        if (obj is None) or obj.type != "ARMATURE":
                return None
        return obj

    obj = bpy.data.objects.get(name_rig)
    if obj is not None and obj.type == 'ARMATURE':
        return obj
    
    return None


def create_bones(arm_data, adapt, tail_length, advance: bool = False):
    """Crea ossa e parentele dell'intero rig. L'armatura dev'essere in Edit Mode."""
    if advance:
        create_landmarkers_bone(arm_data, adapt)
        return

    for name, data in FACE_MAPPING.items():
        bone = arm_data.edit_bones.new(name)

        if name in ROTATION_BONES:
            # ossa dellan mandibola: testa sul perno, coda sul mento.
            head_pos, tail_pos = ROTATION_BONES[name]
            bone.head = adapt(head_pos)
            bone.tail = adapt(tail_pos)
            continue

        x, y, z = adapt(data.position)
        bone.head = (x, y, z)
        bone.tail = (x, y, z + tail_length(name))

    for name, data in FACE_MAPPING.items():
        if data.parent_bone:
            arm_data.edit_bones[name].parent = arm_data.edit_bones[data.parent_bone]

def create_landmarkers_bone(arm_data, adapt):
    """Make the landmarkers rig"""
    #fixed properties for each bone
    tail_len = 0.008
    bbone_x = 0.004
    bbone_z = 0.003

    #make the root
    root_bone = arm_data.edit_bones.new("LMK-Root")
    root_bone.head = (0, 0, 0)
    root_bone.tail = (0, 0, tail_len)
    root_bone.bbone_x = bbone_x
    root_bone.bbone_z = bbone_z

    for name, data in LANDMARKERS_FACE_MAPPING.items():
        bone = arm_data.edit_bones.new(name)

        x, y, z = adapt(data.position)
        bone.head = (x, y, z)
        bone.tail = (x, y, z + tail_len)
        bone.bbone_x = bbone_x
        bone.bbone_z = bbone_z
        bone.parent =  root_bone

    bpy.ops.object.mode_set(mode='OBJECT')

    # Assign display color afterwards
    for bone in arm_data.bones:
        bone.color.palette = 'THEME07'

#added
def apply_translation(pose_bone, vec):
    pose_bone.location = (traslation_to_bone_space(pose_bone=pose_bone, vec=vec))

#added
def apply_rotation(pose_bone, rotation):
    pose_bone.rotation_mode = "QUATERNION"
    pose_bone.rotation_quaternion = (rotation.to_quaternion())

#added
def validate_target(rig, mappings):
    if rig is None:
        return ["Target not found"]

    missing = []

    for mapping in mappings:
        if not mapping.enabled:
            continue

        name = mapping.target_bone

        if not name:
            continue

        if rig.pose.bones.get(name) is None:
            missing.append(name)

    return sorted(set(missing))

def reset_rig_pose(rig):
    """Riporta a riposo le ossa gestite dal mocap.
       Reset the bone location position and rotation
    """
    for pose_bone in rig.pose.bones.values():
        if not pose_bone:
            continue
        pose_bone.location = (0.0, 0.0, 0.0)
        if pose_bone.rotation_mode == 'QUATERNION':
            pose_bone.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        else:
            pose_bone.rotation_euler = (0.0, 0.0, 0.0)

```

## Source file: operators/rigging.py

{
  "path": "operators/rigging.py",
  "blob_sha": "16c5ddb28fd2f29a069a341b2479e5e28969d063",
  "sha256": "e1d83b3d9bcada0396405aa06b8aa8b04f44e42284f6132a7aac55de00ef7efe",
  "lines": 92,
  "bytes": 2920,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 4,
    "face": 8
  },
  "url": "https://github.com/adgx/FaceMocap/blob/96d77e6c4a2c8cc25daacb7b0c8f42311180e0f6/operators/rigging.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def execute(self, context):"
    },
    {
      "line": 50,
      "text": "def execute(self, context):"
    }
  ]
}

```text

import bpy

from ..core.config import FACE_MAPPING, LANDMARKERS_FACE_MAPPING
from ..core.rig import BASE_RIG_NAME, LANDMARKS_RIG_NAME, create_bones

class FACEMOCAP_OT_create_armature(bpy.types.Operator):
    """Genera l'armatura facciale di riferimento per il motion capture"""
    bl_idname = "facemocap.create_armature"
    bl_label = "Crea Armatura Base"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        # Crea l'armatura
        arm_data = bpy.data.armatures.new(name="FaceMocap_Arm_Data")
        arm_obj = bpy.data.objects.new(name=BASE_RIG_NAME, object_data=arm_data)

        # collega l'armatura alla scena
        context.collection.objects.link(arm_obj)

        # seleziona armatura
        bpy.ops.object.select_all(action='DESELECT')
        arm_obj.select_set(True)
        context.view_layer.objects.active = arm_obj

        bpy.ops.object.mode_set(mode='EDIT')

        # Armatura di riferimento
        create_bones(
            arm_data,
            adapt=lambda pos: pos,
            tail_length=lambda name: 0.6 if name == "Head" else 0.1,
        )

        bpy.ops.object.mode_set(mode='OBJECT')

        arm_obj.show_in_front = True

        self.report({'INFO'}, "Armatura generata con %d ossa!" % len(FACE_MAPPING))

        return {'FINISHED'}

class FACEMOCAP_OT_create_advance_armature(bpy.types.Operator):
    """Genera l'armatura facciale di riferimento per il motion capture
       Generate the advance facial armature
    """
    bl_idname = "facemocap.create_advance_armature"
    bl_label = "Generate Advance Facial Armature"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        #create the collection
        parent_collection = context.collection

        if parent_collection is None:
            self.report({'ERROR'}, "No active collection")
            return {'CANCELLED'}
        
        FM_facial_coll_name = "FaceMocap-Facial_rig"
        FM_facial_coll = bpy.data.collections.new(FM_facial_coll_name)
        parent_collection.children.link(FM_facial_coll)

        #add armature
        arm_name = LANDMARKS_RIG_NAME
        arm_data = bpy.data.armatures.new(arm_name)
        arm_data.display_type = 'BBONE'
        arm_obj = bpy.data.objects.new(arm_name, arm_data)

        FM_facial_coll.objects.link(arm_obj)

        #select the arm
        for obj in context.view_layer.objects:
            obj.select_set(False)
            
        arm_obj.select_set(True)
        context.view_layer.objects.active = arm_obj

        bpy.ops.object.mode_set(mode='EDIT')

        create_bones(
            arm_data,
            adapt=lambda pos: pos,
            tail_length=lambda name: 0.6 if name == "Head" else 0.1,
            advance=True
        )

        bpy.ops.object.mode_set(mode='OBJECT')

        arm_obj.show_in_front = True

        self.report({'INFO'}, "Armatura generata")

        return {'FINISHED'}

```

## Source file: gui/gui.py

{
  "path": "gui/gui.py",
  "blob_sha": "8bb1879fb3e7070d0b2e901b22ae3eb870c5731b",
  "sha256": "90c680b092962539807c27dcd0d54bde3616a8fbff22001dcc72528763f6c8ef",
  "lines": 99,
  "bytes": 3959,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 5,
    "face": 17
  },
  "url": "https://github.com/adgx/FaceMocap/blob/96d77e6c4a2c8cc25daacb7b0c8f42311180e0f6/gui/gui.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def draw_item(self, context: Context | None, layout: UILayout, data: None | Any, item: None | Any, icon: int | None, active_data: Any, active_property: str | None, index: int | None, flt_flag: int | None) -> None:"
    },
    {
      "line": 35,
      "text": "def draw(self, context):"
    }
  ]
}

```text

from typing import Any

import bpy

from bpy.types import(
    Context,
    Panel,
    UILayout,
    UIList
)
from ..core import properties 

class FACEMOCAP_UL_mapping(UIList):
    bl_idname = "FACEMOCAP_UL_mapping"

    def draw_item(self, context: Context | None, layout: UILayout, data: None | Any, item: None | Any, icon: int | None, active_data: Any, active_property: str | None, index: int | None, flt_flag: int | None) -> None:
        row = layout.row(align=True)
        row.prop(item, "enabled", text="")
        row.label(text=item.role)
        row.label(text=item.target_bone)
        row.label(text=item.source_bones)
        row.prop(item, "mode", text="")
        row.prop(item, "gain", text="")

class FACEMOCAP_PT_main_panel(Panel):
    """Crea un Pannello nella barra laterale (N) della Vista 3D
       Pannel on View 3D
    """
    bl_label = "FaceMocap"
    bl_idname = "FACEMOCAP_PT_main_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'FaceMocap'

    def draw(self, context):
        layout = self.layout
        settings = (context.scene.facemocap)

        # Importazione Modello
        box_model = layout.box()
        box_model.label(text="3D Mesh", icon='MESH_MONKEY')
        box_model.operator("facemocap.import_custom_model", text="Importa Modello", icon='IMPORT')

        # Setup dell'Armatura
        box_setup = layout.box()
        box_setup.label(text="Setup Armature", icon='OUTLINER_OB_ARMATURE')

        # Generatore Armatura Standard
        box_setup.operator("facemocap.create_armature", text="Generate Base Armature", icon='BONE_DATA')
        # Generatore Armatura adattata al modello
        box_setup.operator("facemocap.create_adaptive_armature", text="Generate Armature Base On Mesh", icon='ARMATURE_DATA')
        # Operatore manuale per collegare i pesi
        box_setup.operator("facemocap.bind_model", text="Link Manually", icon='LINKED')

        # Setup source rig and target rig
        box_advance_setup = layout.box()
        box_advance_setup.label(text="Advance Setup", icon="OUTLINER_OB_ARMATURE")
        box_advance_setup.prop(settings, "source_rig_name", text="Source")
        box_advance_setup.prop(settings, "target_rig_name", text="Target")
        row = box_advance_setup.row(align=True)
        row.operator("facemocap.initialize", icon="FILE_REFRESH")
        row.operator("facemocap.validate", icon="CHECKMARK")
        # Generatore source rig
        box_advance_setup.operator("facemocap.create_advance_armature", text="Generate Source Rig", icon='BONE_DATA')
        #Mapping
        box_mapping = layout.box()
        box_mapping.label(text="Target Mapping")
        box_mapping.template_list("FACEMOCAP_UL_mapping", 
                                  "",
                                  settings,
                                  "mappings",
                                  settings,
                                  "mapping_index",
                                  rows=0)
        index = (settings.mapping_index)

        if 0 <= index < len(settings.mappings):
            item = (settings.mappings[index])
            detail = box_mapping.box()
            detail.label(text="Mapping")
            row = detail.row()
            row.enabled = False
            row.prop(item, "role")
            detail.prop(item, "target_bone")
            detail.prop(item, "source_bones")

        
        # Motion Capture
        box_mocap = layout.box()
        box_mocap.label(text="Campture", icon='ANIM')
        box_mocap.prop(settings, "camera_id")
        box_mocap.prop(settings, "mirror_x")
        box_mocap.prop(settings, "smoothing")

        box_mocap.prop(settings, "show_preview")

        box_mocap.operator("facemocap.start_capture", text="Start Motion Capture", icon='PLAY')
        box_mocap.operator("facemocap.reset_pose", text="Reset Pose", icon='LOOP_BACK')
        box_mocap.label(text="ESC = stop | C = recalibration")


```

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "d930705e9ce6531765c5e8e5d9571c2f1558105a",
  "sha256": "2532f5e4cb288979bc157b53ceff7a1e080b5d30ff9fa6242ba9f19bb14d47ff",
  "lines": 100,
  "bytes": 3598,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 18
  },
  "url": "https://github.com/adgx/FaceMocap/blob/96d77e6c4a2c8cc25daacb7b0c8f42311180e0f6/__init__.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def _run(cmd):"
    },
    {
      "line": 38,
      "text": "def install_dependencies():"
    },
    {
      "line": 93,
      "text": "def register():"
    },
    {
      "line": 98,
      "text": "def unregister():"
    }
  ]
}

```text

bl_info = {
    "name": "FaceMocap",
    "author": "adgx and F3de22",
    "blender": (4, 2, 0),
    "category": "Animation",
}

import bpy
import subprocess
import importlib
import sys
from . import auto_load
from pathlib import Path

ADDON_DIR = Path(__file__).parent
LIB_DIR = ADDON_DIR / "site-packages"
LIB_DIR.mkdir(exist_ok=True)
MODEL_DIR = LIB_DIR / "model"
MODEL_NAME = "face_landmarker_v2.task"
MODEL_DIR.mkdir(exist_ok=True)

if str(LIB_DIR) not in sys.path:
    sys.path.append(str(LIB_DIR))
 
 
def _run(cmd):
    print(f"\033[34mFaceMocap: runs -> {' '.join(cmd)}\033[0m")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr)
    if result.returncode != 0:
        print(f"\033[31mFaceMocap: Warning, command failed with exit code {result.returncode}: {' '.join(cmd)}\033[0m")
    return result.returncode == 0

#installa le dipendenze se non sono già installate
def install_dependencies():
    python_exe = sys.executable
    print(f"\033[34mFaceMocap: using Python interpreter -> {python_exe}\033[0m")
    print(f"\033[34mFaceMocap: local dependencies folder -> {LIB_DIR}\033[0m")
    missing_packages = []
 
    try:
        import cv2
        print(f"\033[32mFaceMocap: cv2 version={cv2.__version__} detected\033[0m")
    except ImportError as e:
        print(f"\033[31mFaceMocap: cv2 not found ({e})\033[0m")
        missing_packages.append("opencv-python==4.9.0.80")
 
    try:
        import mediapipe
        print(f"\033[32mFaceMocap: mediapipe version={mediapipe.__version__} detected\033[0m")
    except ImportError as e:
        print(f"\033[31mFaceMocap: mediapipe not found ({e})\033[0m")
        missing_packages.append("mediapipe==1.0.1")
 
    if missing_packages:
        print(f"\033[34mFaceMocap: Installing {missing_packages} in {LIB_DIR}. Wait...\033[0m")
        ok = _run([python_exe, "-m", "ensurepip"])
        ok = _run([python_exe, "-m", "pip", "install", "--upgrade", "pip"]) and ok
        ok = _run([
            python_exe, "-m", "pip", "install",
            "--target", str(LIB_DIR),
            "numpy<2", "protobuf<4", *missing_packages,
        ]) and ok 
 
        if not ok:
            print("\033[31mFaceMocap: ERROR - one or more installation commands failed. See the output above.\033[0m")
        else:
            print("\033[32mFaceMocap: Dependencies installed successfully.\033[0m")
 
        importlib.invalidate_caches()
 
        for pkg_import, pkg_name in [("cv2", "opencv-python"), ("mediapipe", "mediapipe")]:
            try:
                importlib.import_module(pkg_import)
                print(f"\033[32mFaceMocap: Verification OK, '{pkg_import}' can be imported after installation.\033[0m")
            except ImportError as e:
                print(f"\033[31mFaceMocap: Verification FAILED for '{pkg_import}' ({pkg_name}): {e}\033[0m")
        
    if not (MODEL_DIR / MODEL_NAME).exists:
        ok = _run([
            "wget",
            "-O", 
            str(MODEL_DIR / MODEL_NAME),
            "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/latest/face_landmarker.task"
        ])
        print(f"\033[32mFacemocap: model: {str(MODEL_NAME)} downloaded in: {str(MODEL_DIR)}\033[0m")
    else: 
        print(f"\033[32mFacemocap: model: {str(MODEL_NAME)} detected at: {str(MODEL_DIR)}\033[0m")

def register():
    install_dependencies()
    auto_load.init()
    auto_load.register()

def unregister():
    from . import auto_load
    auto_load.unregister()

```

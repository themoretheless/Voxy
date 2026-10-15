# Facial rendering source scan: elijah-atkins/ARKitBlendshapeHelper

Repository: https://github.com/elijah-atkins/ARKitBlendshapeHelper

Pinned source revision: 8629d593e596894a35959768c87b1f781894cb8c

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["facial_rig_names.py", "facial_rig_operator.py", "facial_rig_panel.py"]}

## Source file: facial_rig_names.py

{
  "path": "facial_rig_names.py",
  "blob_sha": "87ad86aa4e2994ea0117dfbb822a346b06d0596f",
  "sha256": "06518feb9b797bbe3383441b241509644aa5db802bec033b1f9b56e639d86ca5",
  "lines": 56,
  "bytes": 1578,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 14,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 21
  },
  "url": "https://github.com/elijah-atkins/ARKitBlendshapeHelper/blob/8629d593e596894a35959768c87b1f781894cb8c/facial_rig_names.py",
  "function_signature_candidates": []
}

```text

class FacialRigNames:
    names = {
        0: 'Basis',
        1: 'eyeBlinkLeft',
        2: 'eyeLookDownLeft',
        3: 'eyeLookInLeft',
        4: 'eyeLookOutLeft',
        5: 'eyeLookUpLeft',
        6: 'eyeSquintLeft',
        7: 'eyeWideLeft',
        8: 'eyeBlinkRight',
        9: 'eyeLookDownRight',
        10: 'eyeLookInRight',
        11: 'eyeLookOutRight',
        12: 'eyeLookUpRight',
        13: 'eyeSquintRight',
        14: 'eyeWideRight',
        15: 'jawForward',
        16: 'jawLeft',
        17: 'jawRight',
        18: 'jawOpen',
        19: 'mouthClose',
        20: 'mouthFunnel',
        21: 'mouthPucker',
        22: 'mouthRight',
        23: 'mouthLeft',
        24: 'mouthSmileLeft',
        25: 'mouthSmileRight',
        26: 'mouthFrownRight',
        27: 'mouthFrownLeft',
        28: 'mouthDimpleLeft',
        29: 'mouthDimpleRight',
        30: 'mouthStretchLeft',
        31: 'mouthStretchRight',
        32: 'mouthRollLower',
        33: 'mouthRollUpper',
        34: 'mouthShrugLower',
        35: 'mouthShrugUpper',
        36: 'mouthPressLeft',
        37: 'mouthPressRight',
        38: 'mouthLowerDownLeft',
        39: 'mouthLowerDownRight',
        40: 'mouthUpperUpLeft',
        41: 'mouthUpperUpRight',
        42: 'browDownLeft',
        43: 'browDownRight',
        44: 'browInnerUp',
        45: 'browOuterUpLeft',
        46: 'browOuterUpRight',
        47: 'cheekPuff',
        48: 'cheekSquintLeft',
        49: 'cheekSquintRight',
        50: 'noseSneerLeft',
        51: 'noseSneerRight',
        52: 'tongueOut'
    }


```

## Source file: facial_rig_panel.py

{
  "path": "facial_rig_panel.py",
  "blob_sha": "677d1522ee91d897e3054a06216714c31fe9008f",
  "sha256": "c7cd243d37e64dc8747ac4234b6c6a5587136ea7de612cb2cad8e76b815c8dc5",
  "lines": 24,
  "bytes": 955,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "rig": 6
  },
  "url": "https://github.com/elijah-atkins/ARKitBlendshapeHelper/blob/8629d593e596894a35959768c87b1f781894cb8c/facial_rig_panel.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def draw(self, context):"
    }
  ]
}

```text

import bpy
from .facial_rig_names import FacialRigNames

class OBJECT_PT_facial_rig_panel(bpy.types.Panel):
    bl_idname = "OBJECT_PT_facial_rig_panel"
    bl_label = "ARKit Helper"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "ARKit Helper"


    def draw(self, context):
        layout = self.layout
        # Add source object property
        layout.label(text="Set poses on frames 1-52")
        frame_number = context.scene.frame_current
        if frame_number in FacialRigNames.names:
            layout.label(text="Pose: " + FacialRigNames.names[frame_number])
        else:
            layout.label(text="No name for this frame.")
        layout.prop(context.scene, "arkit_source_object", text="Source Object")
        # Add target object property
        layout.prop(context.scene, "arkit_target_object", text="Target Object")
        layout.operator("object.facial_rig_operator", text="Create ARKit Blendshapes")


```

## Source file: facial_rig_operator.py

{
  "path": "facial_rig_operator.py",
  "blob_sha": "b34c68ca4820e71c250324ce982288c28d25bccc",
  "sha256": "5f05d33495c8dcf8f9108fbcf1d645fbfb0cfe09c95efb1f10835cbd527a55e0",
  "lines": 153,
  "bytes": 6733,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 14,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 24
  },
  "url": "https://github.com/elijah-atkins/ARKitBlendshapeHelper/blob/8629d593e596894a35959768c87b1f781894cb8c/facial_rig_operator.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def execute(self, context):"
    }
  ]
}

```text

import bpy
from .facial_rig_names import FacialRigNames

class FacialRigOperator(bpy.types.Operator):
    bl_idname = "object.facial_rig_operator"
    bl_label = "ARKit Helper Operator"
    
    def execute(self, context):
        scn = bpy.context.scene #get current scene
        scn.frame_set(0)
        # Deselect all objects
        bpy.ops.object.select_all(action='DESELECT')
        
        # Get the source and target objects from the panel UI
        source_obj = context.scene.arkit_source_object
        target_obj = context.scene.arkit_target_object
        
        # Select the source and target objects
        target_obj.select_set(True)
        source_obj.select_set(True)
        obj = bpy.context.active_object

        # Find the armature modifier by type
        armature_mod = None
        for mod in obj.modifiers:
            if mod.type == 'ARMATURE':
                armature_mod = mod
                break

        # If an armature modifier was found, remove it
        if armature_mod:
            obj.modifiers.remove(armature_mod)
        scn.frame_set(1)
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #1 eyeBlinkLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #2 eyeLookDownLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #3 eyeLookInLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #4 eyeLookOutLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #5 eyeLookUpLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #6 eyeSquintLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #7 eyeWideLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #8 eyeBlinkRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #9 eyeLookDownRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #10 eyeLookInRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #11 eyeLookOutRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #12 eyeLookUpRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #13 eyeSquintRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #14 eyeWideRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #15 jawForward
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #16 jawLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #17 jawRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #18 jawOpen
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #19 mouthClose
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #20 mouthFunnel
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #21 mouthPucker
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #22 mouthRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #23 mouthLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #24 mouthSmileLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #25 mouthSmileRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #26 mouthFrownRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #27 mouthFrownLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #28 mouthDimpleLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #29 mouthDimpleRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #30 mouthStretchLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #31 mouthStretchRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #32 mouthRollLower
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #33 mouthRollUpper
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #34 mouthShrugLower
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #35 mouthShrugUpper
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #36 mouthPressLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #37 mouthPressRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #38 mouthLowerDownLeft`
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #39 mouthLowerDownRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #40 mouthUpperUpLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #41 mouthUpperUpRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #42 browDownLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #43 browDownRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #44 browInnerUp
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #45 browOuterUpLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #46 browOuterUpRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #47 cheekPuff
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #48 cheekSquintLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #49 cheekSquintRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #50 noseSneerLeft
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #51 noseSneerRight
        bpy.ops.object.join_shapes()
        scn.frame_set(scn.frame_current + 1) #52 tongueOut


        # get the selected object
        selected_object = bpy.context.object

        # get its shapekeys
        shape_keys = selected_object.data.shape_keys.key_blocks

        # loop through shapekeys and replace the names
        for index, key in enumerate(shape_keys):
            if key.name != "Basis":
                try: key.name = FacialRigNames.names[index]
                except: pass


        return {'FINISHED'}


```

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "c86208be70b301d072978174b92492b4f7fcab10",
  "sha256": "ae8c299d0fc11f977f2ac0543949b98ae9ec5db297d34156d77046793d32d256",
  "lines": 36,
  "bytes": 930,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 1,
    "rig": 4
  },
  "url": "https://github.com/elijah-atkins/ARKitBlendshapeHelper/blob/8629d593e596894a35959768c87b1f781894cb8c/__init__.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def register():"
    },
    {
      "line": 28,
      "text": "def unregister():"
    }
  ]
}

```text

bl_info = {
    "name": "ARKit Blendshape Helper",
    "author": "Elijah Atkins",
    "description": "ARKit Helper Add-on",
    "blender": (2, 93, 0),
    "version": (1, 0, 0),
    "location": "View3D > Sidebar > ARKit Helper",
    "category": "Animation"
}

import bpy
from .facial_rig_operator import FacialRigOperator
from .facial_rig_panel import OBJECT_PT_facial_rig_panel

classes = (
    
    FacialRigOperator,
    OBJECT_PT_facial_rig_panel,
)

def register():
    for cls in classes:
        bpy.utils.register_class(cls)

    bpy.types.Scene.arkit_source_object = bpy.props.PointerProperty(type=bpy.types.Object)
    bpy.types.Scene.arkit_target_object = bpy.props.PointerProperty(type=bpy.types.Object)

def unregister():
    for cls in classes:
        bpy.utils.unregister_class(cls)

    del bpy.types.Scene.arkit_source_object
    del bpy.types.Scene.arkit_target_object

if __name__ == "__main__":
    register()


```

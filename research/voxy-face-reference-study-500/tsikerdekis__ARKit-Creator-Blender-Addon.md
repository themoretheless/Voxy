# Facial rendering source scan: tsikerdekis/ARKit-Creator-Blender-Addon

Repository: https://github.com/tsikerdekis/ARKit-Creator-Blender-Addon

Pinned source revision: 858a049993d9b6bf358264e1df1a40179bc83b50

Discovery topic: ARKit blendshape

Description: Blender addon that helps easily create ARKit blendshapes for your characters needed for face capture

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["arkit_reference_images/eyeBlinkLeft.png", "arkit_reference_images/eyeBlinkRight.png", "arkit_reference_images/eyeLookDownLeft.png", "arkit_reference_images/eyeLookDownRight.png", "arkit_reference_images/eyeLookInLeft.png", "arkit_reference_images/eyeLookInRight.png", "arkit_reference_images/eyeLookOutLeft.png", "arkit_reference_images/eyeLookOutRight.png", "arkit_reference_images/eyeLookUpLeft.png", "arkit_reference_images/eyeLookUpRight.png", "arkit_reference_images/eyeSquintLeft.png", "arkit_reference_images/eyeSquintRight.png", "arkit_reference_images/eyeWideLeft.png", "arkit_reference_images/eyeWideRight.png"], "jaw": ["arkit_reference_images/jawForward.png", "arkit_reference_images/jawLeft.png", "arkit_reference_images/jawOpen.png", "arkit_reference_images/jawRight.png"], "mouth": ["arkit_reference_images/mouthClose.png", "arkit_reference_images/mouthDimpleLeft.png", "arkit_reference_images/mouthDimpleRight.png", "arkit_reference_images/mouthFrownLeft.png", "arkit_reference_images/mouthFrownRight.png", "arkit_reference_images/mouthFunnel.png", "arkit_reference_images/mouthLeft.png", "arkit_reference_images/mouthLowerDownLeft.png", "arkit_reference_images/mouthLowerDownRight.png", "arkit_reference_images/mouthPressLeft.png", "arkit_reference_images/mouthPressRight.png", "arkit_reference_images/mouthPucker.png", "arkit_reference_images/mouthRight.png", "arkit_reference_images/mouthRollLower.png", "arkit_reference_images/mouthRollUpper.png", "arkit_reference_images/mouthShrugLower.png", "arkit_reference_images/mouthShrugUpper.png", "arkit_reference_images/mouthSmileLeft.png", "arkit_reference_images/mouthSmileRight.png", "arkit_reference_images/mouthStretchLeft.png", "arkit_reference_images/mouthStretchRight.png", "arkit_reference_images/mouthUpperUpLeft.png", "arkit_reference_images/mouthUpperUpRight.png"], "rig": ["arkit_reference_images/browDownRight.png", "arkit_reference_images/browOuterUpRight.png", "arkit_reference_images/cheekSquintRight.png", "arkit_reference_images/eyeBlinkRight.png", "arkit_reference_images/eyeLookDownRight.png", "arkit_reference_images/eyeLookInRight.png", "arkit_reference_images/eyeLookOutRight.png", "arkit_reference_images/eyeLookUpRight.png", "arkit_reference_images/eyeSquintRight.png", "arkit_reference_images/eyeWideRight.png", "arkit_reference_images/jawRight.png", "arkit_reference_images/mouthDimpleRight.png", "arkit_reference_images/mouthFrownRight.png", "arkit_reference_images/mouthLowerDownRight.png", "arkit_reference_images/mouthPressRight.png", "arkit_reference_images/mouthRight.png", "arkit_reference_images/mouthSmileRight.png", "arkit_reference_images/mouthStretchRight.png", "arkit_reference_images/mouthUpperUpRight.png", "arkit_reference_images/noseSneerRight.png"]}

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "4cb62b41d2d6c8c8d5e3d9d510c61ec408dbf45c",
  "sha256": "7fcdaa9e099460802b2a5e027f351953630b9c9bb4d54acd6c7f4ab013f6a566",
  "lines": 258,
  "bytes": 9431,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "jaw": 2,
    "mouth": 6,
    "tongue": 1,
    "rig": 11,
    "face": 1
  },
  "url": "https://github.com/tsikerdekis/ARKit-Creator-Blender-Addon/blob/858a049993d9b6bf358264e1df1a40179bc83b50/__init__.py",
  "function_signature_candidates": [
    {
      "line": 35,
      "text": "def get_joined_obj(scene):"
    },
    {
      "line": 38,
      "text": "def add_ref_image(image_path):"
    },
    {
      "line": 79,
      "text": "def clear_armature_pose(armature_obj):"
    },
    {
      "line": 101,
      "text": "def preview_set_value(scene, value):"
    },
    {
      "line": 110,
      "text": "def register_props():"
    },
    {
      "line": 123,
      "text": "def unregister_props():"
    },
    {
      "line": 133,
      "text": "def execute(self, context):"
    },
    {
      "line": 168,
      "text": "def execute(self, context):"
    },
    {
      "line": 228,
      "text": "def draw(self, context):"
    },
    {
      "line": 247,
      "text": "def register():"
    },
    {
      "line": 252,
      "text": "def unregister():"
    }
  ]
}

```text

bl_info = {
    "name": "ARKit Shape Key Baker (Always Joined)",
    "author": "Michail Tsikerdekis",
    "version": (1, 4, 6),
    "blender": (4, 4, 0),
    "location": "View3D > Sidebar > ARKit",
    "description": "Bake 52 ARKit shape keys on a permanently joined mesh with reference images facing the editor view.",
    "category": "Animation",
}

import bpy
import os
from mathutils import Vector

# --- ARKit shapes ---
ARKIT_SHAPES = [
    "browDownLeft","browDownRight","browInnerUp",
    "browOuterUpLeft","browOuterUpRight","cheekPuff","cheekSquintLeft","cheekSquintRight",
    "eyeBlinkLeft","eyeBlinkRight","eyeLookDownLeft","eyeLookDownRight","eyeLookInLeft",
    "eyeLookInRight","eyeLookOutLeft","eyeLookOutRight","eyeLookUpLeft","eyeLookUpRight",
    "eyeSquintLeft","eyeSquintRight","eyeWideLeft","eyeWideRight","jawForward","jawLeft",
    "jawRight","jawOpen","mouthClose","mouthDimpleLeft","mouthDimpleRight","mouthFrownLeft",
    "mouthFrownRight","mouthFunnel","mouthLeft","mouthRight","mouthPucker","mouthPressLeft",
    "mouthPressRight","mouthRollLower","mouthRollUpper","mouthShrugLower","mouthShrugUpper",
    "mouthSmileLeft","mouthSmileRight","mouthStretchLeft","mouthStretchRight",
    "mouthUpperUpLeft","mouthUpperUpRight","noseSneerLeft","noseSneerRight","tongueOut",
    "mouthLowerDownLeft","mouthLowerDownRight"
]

addon_dir = os.path.dirname(os.path.realpath(__file__))
DEFAULT_REF_FOLDER = os.path.join(addon_dir, "arkit_reference_images")

# --- Utilities ---

def get_joined_obj(scene):
    return bpy.data.objects.get(scene.arkit_joined_mesh)

def add_ref_image(image_path):
    """Show a floating reference image empty facing the editor."""
    # Remove old
    for o in list(bpy.data.objects):
        if o.type == 'EMPTY' and o.name.startswith("ARKit_Ref_"):
            bpy.data.objects.remove(o, do_unlink=True)

    if not os.path.exists(image_path):
        print(f"[ARKit] Missing reference image: {image_path}")
        return False

    img_name = os.path.basename(image_path)
    img = bpy.data.images.get(img_name) or bpy.data.images.load(image_path)

    joined = get_joined_obj(bpy.context.scene)
    if joined:
        loc = joined.location.copy()
        loc.x -= 0.2
        loc.z += 0.4
    else:
        loc = Vector((-1.0, 0, 1.2))

    bpy.ops.object.add(type='EMPTY', location=loc)
    empty = bpy.context.active_object
    empty.name = "ARKit_Ref_" + os.path.splitext(img_name)[0]
    empty.empty_display_type = 'IMAGE'
    empty.data = img
    empty.empty_display_size = 0.10
    empty.empty_image_depth = 'FRONT'
    empty.show_in_front = True

    # Face the view
    for area in bpy.context.screen.areas:
        if area.type == 'VIEW_3D':
            region_3d = area.spaces[0].region_3d
            view_dir = region_3d.view_rotation @ Vector((0.0, 0.0, -1.0))
            empty.rotation_euler = view_dir.to_track_quat('-Z', 'Y').to_euler()
            break

    return True

def clear_armature_pose(armature_obj):
    if not armature_obj or armature_obj.type != 'ARMATURE':
        return
    bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature_obj
    armature_obj.select_set(True)
    bpy.ops.object.mode_set(mode='POSE')
    bpy.ops.pose.select_all(action='SELECT')
    try:
        bpy.ops.pose.loc_clear()
        bpy.ops.pose.rot_clear()
        bpy.ops.pose.scale_clear()
    except:
        for pb in armature_obj.pose.bones:
            pb.location = (0, 0, 0)
            if pb.rotation_mode == 'QUATERNION':
                pb.rotation_quaternion = (1, 0, 0, 0)
            else:
                pb.rotation_euler = (0, 0, 0)
            pb.scale = (1, 1, 1)
    bpy.ops.object.mode_set(mode='OBJECT')

def preview_set_value(scene, value):
    joined = get_joined_obj(scene)
    if not joined or not joined.data.shape_keys:
        return
    sk = joined.data.shape_keys.key_blocks.get(scene.arkit_preview_shape)
    if sk:
        sk.value = value

# --- Properties ---
def register_props():
    bpy.types.Scene.arkit_current_index = bpy.props.IntProperty(default=0, min=0)
    bpy.types.Scene.arkit_joined_mesh = bpy.props.StringProperty(default="")
    bpy.types.Scene.arkit_frame_step = bpy.props.IntProperty(default=5, min=1)
    bpy.types.Scene.arkit_preview_shape = bpy.props.EnumProperty(
        items=[(n, n, "") for n in ARKIT_SHAPES],
        default=ARKIT_SHAPES[0]
    )
    bpy.types.Scene.arkit_preview_value = bpy.props.FloatProperty(
        min=0.0, max=1.0, default=0.0,
        update=lambda self, ctx: preview_set_value(ctx.scene, ctx.scene.arkit_preview_value)
    )

def unregister_props():
    for p in ("arkit_current_index","arkit_joined_mesh","arkit_frame_step","arkit_preview_shape","arkit_preview_value"):
        if hasattr(bpy.types.Scene, p):
            delattr(bpy.types.Scene, p)

# --- Operators ---
class ARKIT_OT_StartSession(bpy.types.Operator):
    bl_idname = "arkit.start_session"
    bl_label = "Start ARKit Bake Session"

    def execute(self, context):
        scene = context.scene
        sel = [o for o in context.selected_objects if o.type == 'MESH']
        if not sel:
            self.report({'ERROR'}, "Select at least one mesh")
            return {'CANCELLED'}

        bpy.ops.object.mode_set(mode='OBJECT')
        bpy.context.view_layer.objects.active = sel[0]
        sel[0].select_set(True)
        bpy.ops.object.join()
        joined = bpy.context.view_layer.objects.active
        scene.arkit_joined_mesh = joined.name
        scene.arkit_current_index = 0

        if not joined.data.shape_keys:
            joined.shape_key_add(name="Basis")

        first_path = os.path.join(DEFAULT_REF_FOLDER, ARKIT_SHAPES[0]+".png")
        add_ref_image(first_path)
        self.report({'INFO'}, f"Session started. Pose for {ARKIT_SHAPES[0]}")

        for mod in joined.modifiers:
            if mod.type == 'ARMATURE' and mod.object:
                bpy.context.view_layer.objects.active = mod.object
                bpy.ops.object.mode_set(mode='POSE')
                break

        return {'FINISHED'}


class ARKIT_OT_BakeCurrent(bpy.types.Operator):
    bl_idname = "arkit.bake_current"
    bl_label = "Bake Current Shape Key"

    def execute(self, context):
        scene = context.scene
        idx = scene.arkit_current_index
        if idx >= len(ARKIT_SHAPES):
            self.report({'INFO'}, "All shapes completed")
            return {'CANCELLED'}

        joined = get_joined_obj(scene)
        if not joined:
            self.report({'ERROR'}, "Joined mesh not found")
            return {'CANCELLED'}

        shape_name = ARKIT_SHAPES[idx]

        depsgraph = bpy.context.evaluated_depsgraph_get()
        eval_obj = joined.evaluated_get(depsgraph)
        eval_mesh = eval_obj.to_mesh()

        sk = joined.shape_key_add(name=shape_name, from_mix=False)
        sk.data.foreach_set("co", [v for vert in eval_mesh.vertices for v in vert.co])
        eval_obj.to_mesh_clear()
        sk.value = 0.0

        # reset pose if there's an armature
        active_armature = None
        for mod in joined.modifiers:
            if mod.type == 'ARMATURE' and mod.object:
                clear_armature_pose(mod.object)
                active_armature = mod.object

        # Move to next shape
        scene.arkit_current_index += 1

        if scene.arkit_current_index < len(ARKIT_SHAPES):
            scene.arkit_preview_shape = ARKIT_SHAPES[scene.arkit_current_index]
            scene.arkit_preview_value = 0.0
            next_path = os.path.join(DEFAULT_REF_FOLDER, ARKIT_SHAPES[scene.arkit_current_index]+".png")
            add_ref_image(next_path)
            self.report({'INFO'}, f"Baked {shape_name}, next: {ARKIT_SHAPES[scene.arkit_current_index]}")
        else:
            for o in list(bpy.data.objects):
                if o.type == 'EMPTY' and o.name.startswith("ARKit_Ref_"):
                    bpy.data.objects.remove(o, do_unlink=True)
            self.report({'INFO'}, "All ARKit shape keys baked!")

        if active_armature:
            bpy.context.view_layer.objects.active = active_armature
            bpy.ops.object.mode_set(mode='POSE')

        return {'FINISHED'}


# --- Panel ---
class ARKIT_PT_Panel(bpy.types.Panel):
    bl_label = "ARKit Shape Key Baker"
    bl_idname = "ARKIT_PT_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'ARKit'

    def draw(self, context):
        layout = self.layout
        scene = context.scene

        row = layout.row(align=True)
        row.operator("arkit.start_session")
        row.operator("arkit.bake_current")
        layout.separator()
        layout.label(text=f"Progress: {scene.arkit_current_index}/{len(ARKIT_SHAPES)}")
        if scene.arkit_current_index < len(ARKIT_SHAPES):
            layout.label(text=f"Current: {ARKIT_SHAPES[scene.arkit_current_index]}")
        layout.separator()
        layout.label(text="Preview baked shape:")
        layout.prop(scene, "arkit_preview_shape", text="")
        layout.prop(scene, "arkit_preview_value", slider=True)

# --- Registration ---
classes = [ARKIT_OT_StartSession, ARKIT_OT_BakeCurrent, ARKIT_PT_Panel]

def register():
    register_props()
    for cls in classes:
        bpy.utils.register_class(cls)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    unregister_props()

if __name__ == "__main__":
    register()


```

# Facial rendering source scan: BlenderDefender/MoCapkiteFA

Repository: https://github.com/BlenderDefender/MoCapkiteFA

Pinned source revision: 84b4afd80bea713d53714c9bc2af7f1f1010e126

Discovery topic: facial mocap

Description: Use the Facial Motion Capture Workflow in Blender, demonstrated by CG Matter, in 3 simple steps.

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: menus.py

{
  "path": "menus.py",
  "blob_sha": "2ad38add4b4261090f85be9df9067a68e60a9e9a",
  "sha256": "bd219f86cc54f527e96a83b6d00e510f33d6677c7796033c6f5944e86f4573c7",
  "lines": 63,
  "bytes": 1818,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 1,
    "rig": 1
  },
  "url": "https://github.com/BlenderDefender/MoCapkiteFA/blob/84b4afd80bea713d53714c9bc2af7f1f1010e126/menus.py",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "def draw(self, context: 'Context'):"
    },
    {
      "line": 43,
      "text": "def menu_func(self, context: 'Context'):"
    },
    {
      "line": 52,
      "text": "def register():"
    },
    {
      "line": 59,
      "text": "def unregister():"
    }
  ]
}

```text

# ##### BEGIN GPL LICENSE BLOCK #####
#    Copyright (C) <2020>  <Blender Defender, CG Matter>
#
#    This program is free software: you can redistribute it and/or modify
#    it under the terms of the GNU General Public License as published by
#    the Free Software Foundation, either version 3 of the License, or
#    (at your option) any later version.

#    This program is distributed in the hope that it will be useful,
#    but WITHOUT ANY WARRANTY; without even the implied warranty of
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#    GNU General Public License for more details.

#    You should have received a copy of the GNU General Public License
#    along with this program.  If not, see <https://www.gnu.org/licenses/>.

# ##### END GPL LICENSE BLOCK #####

import bpy
from bpy.types import (
    Context,
    Menu,
    UILayout
)

from .operators import (
    MOCAPKITEFA_OT_facial_mocap,
    MOCAPKITEFA_OT_pre_align
)


# The following menu appears in the "Add" menu and contains the "MOCAPKITEFA_OT_facial_mocap" Operator
class MOCAPKITEFA_MT_main_menu(Menu):
    bl_idname = 'mocapkitefa.menu'
    bl_label = 'Motion Capture'

    def draw(self, context: 'Context'):
        layout: UILayout = self.layout
        layout.operator(MOCAPKITEFA_OT_pre_align.bl_idname, icon='PLUS')
        layout.operator(MOCAPKITEFA_OT_facial_mocap.bl_idname, icon='SHADERFX')


def menu_func(self, context: 'Context'):
    self.layout.menu(MOCAPKITEFA_MT_main_menu.bl_idname)


classes = (
    MOCAPKITEFA_MT_main_menu,
)


def register():
    bpy.types.VIEW3D_MT_add.append(menu_func)

    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    bpy.types.VIEW3D_MT_add.remove(menu_func)

    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


```

## Source file: prefs.py

{
  "path": "prefs.py",
  "blob_sha": "33007a149cb36decec69e37bab5dd86d71ab4918",
  "sha256": "ebe14c853862f11235b7eb342fb5309f17c7e15d64d40a0e570d5ab39e7f1cc3",
  "lines": 107,
  "bytes": 2954,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/BlenderDefender/MoCapkiteFA/blob/84b4afd80bea713d53714c9bc2af7f1f1010e126/prefs.py",
  "function_signature_candidates": [
    {
      "line": 73,
      "text": "def draw(self, context: 'Context'):"
    },
    {
      "line": 88,
      "text": "def register_legacy(bl_info):"
    },
    {
      "line": 98,
      "text": "def register():"
    },
    {
      "line": 103,
      "text": "def unregister():"
    }
  ]
}

```text

# ##### BEGIN GPL LICENSE BLOCK #####
#    Copyright (C) <2020>  <Blender Defender, CG Matter>
#
#    This program is free software: you can redistribute it and/or modify
#    it under the terms of the GNU General Public License as published by
#    the Free Software Foundation, either version 3 of the License, or
#    (at your option) any later version.

#    This program is distributed in the hope that it will be useful,
#    but WITHOUT ANY WARRANTY; without even the implied warranty of
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#    GNU General Public License for more details.

#    You should have received a copy of the GNU General Public License
#    along with this program.  If not, see <https://www.gnu.org/licenses/>.

# ##### END GPL LICENSE BLOCK #####

import bpy
from bpy.types import (
    AddonPreferences,
    Context,
    UILayout
)
from bpy.props import (
    BoolProperty,
    IntProperty,

)

# updater ops import, all setup in this file
from . import addon_updater_ops


class MOCAPKITEFA_APT_Preferences(AddonPreferences):
    bl_idname = __package__

    # addon updater preferences

    auto_check_update: BoolProperty(
        name="Auto-check for Update",
        description="If enabled, auto-check for updates using an interval",
        default=True,
    )
    updater_intrval_months: IntProperty(
        name='Months',
        description="Number of months between checking for updates",
        default=0,
        min=0
    )
    updater_intrval_days: IntProperty(
        name='Days',
        description="Number of days between checking for updates",
        default=7,
        min=0,
        max=31
    )
    updater_intrval_hours: IntProperty(
        name='Hours',
        description="Number of hours between checking for updates",
        default=0,
        min=0,
        max=23
    )
    updater_intrval_minutes: IntProperty(
        name='Minutes',
        description="Number of minutes between checking for updates",
        default=0,
        min=0,
        max=59
    )

    def draw(self, context: 'Context'):
        layout: UILayout = self.layout

        layout.operator("wm.url_open", text="Checkout my other addons...",
                        icon='FUND').url = "https://extensions.blender-defender.com"

        if bpy.app.version < (4, 2):
            addon_updater_ops.update_settings_ui(self, context)


classes = (
    MOCAPKITEFA_APT_Preferences,
)


def register_legacy(bl_info):
    # addon updater code and configurations
    # in case of broken version, try to register the updater first
    # so that users can revert back to a working version
    addon_updater_ops.register(bl_info)

    for cls in classes:
        bpy.utils.register_class(cls)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    addon_updater_ops.unregister()

    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


```

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "a219072d13f6a1df1278d563422fa781eb9f0702",
  "sha256": "27d54f6ed758d3badebed62d4060596ffffec4680d371119586cdf9a7279d3dd",
  "lines": 54,
  "bytes": 1858,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/BlenderDefender/MoCapkiteFA/blob/84b4afd80bea713d53714c9bc2af7f1f1010e126/__init__.py",
  "function_signature_candidates": [
    {
      "line": 42,
      "text": "def register():"
    },
    {
      "line": 51,
      "text": "def unregister():"
    }
  ]
}

```text

# ##### BEGIN GPL LICENSE BLOCK #####
#    Copyright (C) <2020>  <Blender Defender, CG Matter>
#
#    This program is free software: you can redistribute it and/or modify
#    it under the terms of the GNU General Public License as published by
#    the Free Software Foundation, either version 3 of the License, or
#    (at your option) any later version.

#    This program is distributed in the hope that it will be useful,
#    but WITHOUT ANY WARRANTY; without even the implied warranty of
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#    GNU General Public License for more details.

#    You should have received a copy of the GNU General Public License
#    along with this program.  If not, see <https://www.gnu.org/licenses/>.

# ##### END GPL LICENSE BLOCK #####

import bpy

from . import (
    menus,
    operators,
    prefs,
)

bl_info = {
    "name": "MoCapkiteFA - Face Motion Capture",
    "author": "Blender Defender, CG Matter",
    "version": (1, 0, 3),
    "blender": (2, 82, 0),
    "location": "View3D > Add > Motion Capture > Set up Facial Motion Capture",
    "description": "Turns your Trackers and your head into facial motion capture!",
    "warning": "You might run in trouble with translated Blender versions. Use English version!",
    "doc_url": "https://github.com/BlenderDefender/MoCapkiteFA#mocapkitefa",
    "tracker_url": "https://github.com/BlenderDefender/MoCapkiteFA/issues",
    "endpoint_url": "https://raw.githubusercontent.com/BlenderDefender/BlenderDefender/updater_endpoints/MOCAPKITEFA.json",
    "category": "Animation"
}


def register():
    if bpy.app.version < (4, 2):
        prefs.register_legacy(bl_info)
    else:
        prefs.register()
    operators.register()
    menus.register()


def unregister():
    prefs.unregister()
    menus.unregister()
    operators.unregister()


```

## Source file: operators.py

{
  "path": "operators.py",
  "blob_sha": "f78683d6497868aa14de3cbff871a8a4f78d9c73",
  "sha256": "712b52e46298a4c5c18059930dfc06dc2685a3f6bd6998f8f3c1e11814c65fb5",
  "lines": 163,
  "bytes": 5671,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "face": 7,
    "lip": 3
  },
  "url": "https://github.com/BlenderDefender/MoCapkiteFA/blob/84b4afd80bea713d53714c9bc2af7f1f1010e126/operators.py",
  "function_signature_candidates": [
    {
      "line": 34,
      "text": "def poll(cls, context: 'Context'):"
    },
    {
      "line": 37,
      "text": "def execute(self, context: 'Context'):"
    },
    {
      "line": 70,
      "text": "def poll(cls, context: 'Context'):"
    },
    {
      "line": 73,
      "text": "def execute(self, context: 'Context'):"
    },
    {
      "line": 156,
      "text": "def register():"
    },
    {
      "line": 161,
      "text": "def unregister():"
    }
  ]
}

```text

# ##### BEGIN GPL LICENSE BLOCK #####
#    Copyright (C) <2020>  <Blender Defender, CG Matter>
#
#    This program is free software: you can redistribute it and/or modify
#    it under the terms of the GNU General Public License as published by
#    the Free Software Foundation, either version 3 of the License, or
#    (at your option) any later version.

#    This program is distributed in the hope that it will be useful,
#    but WITHOUT ANY WARRANTY; without even the implied warranty of
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#    GNU General Public License for more details.

#    You should have received a copy of the GNU General Public License
#    along with this program.  If not, see <https://www.gnu.org/licenses/>.

# ##### END GPL LICENSE BLOCK #####

import bpy
from bpy.types import (
    Context,
    Operator
)


# The following Operator does the pre-alignment (Minimal changes needed)
class MOCAPKITEFA_OT_pre_align(Operator):
    """Adds a camera and pre aligns it and the Active Object"""
    bl_idname = "mocapkitefa.pre_alignment"
    bl_label = "Pre-Align"
    bl_options = {'REGISTER'}

    @classmethod
    def poll(cls, context: 'Context'):
        return context.active_object is not None

    def execute(self, context: 'Context'):

        context.scene.frame_set(1)
        for obj in context.selected_objects:
            obj.name = "Head"
            obj.data.name = "Head"
            bpy.ops.transform.rotate(
                value=1.5708, orient_axis='X', orient_type='GLOBAL', constraint_axis=(True, False, False))

        bpy.ops.object.camera_add(location=(0, 0, 5), rotation=(0, 0, 0))
        bpy.ops.view3d.object_as_camera()
        for obj in context.selected_objects:
            obj.name = "TopCamMoCap"
            obj.data.name = "TopCamMoCap"
        bpy.ops.object.transform_apply()
        bpy.data.objects['TopCamMoCap'].select_set(False)

        context.area.ui_type = 'CLIP_EDITOR'
        bpy.ops.clip.select_all(action='SELECT')
        bpy.ops.clip.track_to_empty()
        context.area.ui_type = 'VIEW_3D'

        return {'FINISHED'}


# The following Operator set's up the motion Capture
class MOCAPKITEFA_OT_facial_mocap(Operator):
    """Setup Motion Capture (Needs Tracker Setup + Active Object)"""
    bl_idname = "mocapkitefa.facial_mocap"
    bl_label = "Set Up Facial Motion Capture"
    bl_options = {'REGISTER'}

    @classmethod
    def poll(cls, context: 'Context'):
        return context.active_object is not None

    def execute(self, context: 'Context'):

        tracker_count = 1

        # rename trackers
        face_trackers = bpy.data.collections.new('FaceTrackers')
        context.scene.collection.children.link(face_trackers)
        for trackers in bpy.data.objects:
            if trackers.type != 'EMPTY':
                continue

            tracker_count = tracker_count + 1
            face_trackers.objects.link(trackers)

        if face_trackers:
            for i, object in enumerate(face_trackers.objects):
                object.name = f"Tracker{i + 1}"

        # face mesh setup
        context.scene.frame_set(1)
        bpy.ops.object.transform_apply()

        # trackers with depth
        for value in range(1, tracker_count):
            ob = bpy.data.objects[f"Tracker{value}"]
            context.view_layer.objects.active = ob
            context.object.constraints["Follow Track"].depth_object = bpy.data.objects["Head"]

        # add armatures to trackers
        for value in range(1, tracker_count):
            ob = bpy.data.objects[f"Tracker{value}"]
            context.view_layer.objects.active = ob
            bpy.ops.object.armature_add(
                enter_editmode=False, location=ob.matrix_world.translation)

        # select armatures
        for object in bpy.data.objects:
            if object.type != 'ARMATURE':
                continue

            object.select_set(state=True)

        # join armatures in correct order
        context.view_layer.objects.active = ob = bpy.data.objects['Armature']
        bpy.ops.object.join()
        bpy.ops.object.transform_apply()

        # rename bones in armature
        for value in range(1, tracker_count):
            bone_name = "Bone." + "0" * \
                (2 - len(str(value - 1))) + str(value - 1)
            if value == 1:
                bone_name = "Bone"

            bone = bpy.data.objects['Armature'].data.bones.get(bone_name)
            bpy.data.objects['Armature'].data.bones[bone_name].name = f'Bone{value}'

        # parent face to bones
        bpy.data.objects['Head'].select_set(True)
        bpy.data.objects['Armature'].select_set(True)
        bpy.ops.object.parent_set(type='ARMATURE_AUTO')

        # parent bones to trackers
        bpy.ops.object.posemode_toggle()
        for value in range(1, tracker_count):
            tracker_name = f"Tracker{value}"
            bone_name = f"Bone{value}"
            bone = bpy.data.objects['Armature'].data.bones.get(bone_name)
            bpy.data.objects['Armature'].data.bones.active = bone
            bpy.ops.pose.constraint_add(type='COPY_LOCATION')
            context.object.pose.bones[bone_name].constraints["Copy Location"].use_z = False
            context.object.pose.bones[bone_name].constraints["Copy Location"].target = bpy.data.objects[tracker_name]
            bone.select = False

        return {'FINISHED'}


classes = (
    MOCAPKITEFA_OT_pre_align,
    MOCAPKITEFA_OT_facial_mocap,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


```

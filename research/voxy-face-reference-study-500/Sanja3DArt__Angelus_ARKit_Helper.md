# Facial rendering source scan: Sanja3DArt/Angelus_ARKit_Helper

Repository: https://github.com/Sanja3DArt/Angelus_ARKit_Helper

Pinned source revision: 4acedb0dbe695d17403f7638f7f8b4d301cceac8

Discovery topic: ARKit blendshape

Description: Facilitate the process of working with ARKit blendshapes with mirroring, setting shapes as active on multiple meshes, folders, and more!

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: __init__.py

{
  "path": "__init__.py",
  "blob_sha": "515deaf60c50ff7009086557b331864e4df3b9f2",
  "sha256": "ef13079840032cddc9b20fceee4c1ed80cc3b568ab10785887ef7c2a660ca2bf",
  "lines": 567,
  "bytes": 24728,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "blendshape": 17,
    "jaw": 1,
    "mouth": 4,
    "tongue": 1,
    "rig": 25
  },
  "url": "https://github.com/Sanja3DArt/Angelus_ARKit_Helper/blob/4acedb0dbe695d17403f7638f7f8b4d301cceac8/__init__.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def sync_all_active_indices(context, shape_name):"
    },
    {
      "line": 38,
      "text": "def autosort_shapes_logic(context):"
    },
    {
      "line": 108,
      "text": "def execute(self, context):"
    },
    {
      "line": 118,
      "text": "def execute(self, context):"
    },
    {
      "line": 144,
      "text": "def execute(self, context):"
    },
    {
      "line": 157,
      "text": "def execute(self, context):"
    },
    {
      "line": 236,
      "text": "def execute(self, context):"
    },
    {
      "line": 253,
      "text": "def execute(self, context):"
    },
    {
      "line": 278,
      "text": "def execute(self, context):"
    },
    {
      "line": 288,
      "text": "def execute(self, context):"
    },
    {
      "line": 299,
      "text": "def execute(self, context):"
    },
    {
      "line": 313,
      "text": "def execute(self, context):"
    },
    {
      "line": 322,
      "text": "def invoke(self, context, event): return context.window_manager.invoke_confirm(self, event)"
    },
    {
      "line": 328,
      "text": "def execute(self, context):"
    },
    {
      "line": 339,
      "text": "def execute(self, context):"
    },
    {
      "line": 345,
      "text": "def execute(self, context):"
    },
    {
      "line": 358,
      "text": "def execute(self, context):"
    },
    {
      "line": 380,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 391,
      "text": "def execute(self, context):"
    },
    {
      "line": 428,
      "text": "def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):"
    },
    {
      "line": 438,
      "text": "def draw(self, context):"
    },
    {
      "line": 540,
      "text": "def register():"
    },
    {
      "line": 554,
      "text": "def unregister():"
    }
  ]
}

```text

import bpy

# --------------------------------------------------
# CONSTANTS & DEFAULTS
# --------------------------------------------------

ARKIT_DEFAULTS = {
    "Brows": ["browDown","browDownLeft","browDownRight","browInnerUp","browOuterUp","browOuterUpLeft","browOuterUpRight"],
    "Eyes": ["eyeBlink","eyeBlinkLeft","eyeBlinkRight","eyeLookDown","eyeLookDownLeft","eyeLookDownRight","eyeLookIn","eyeLookInLeft","eyeLookInRight",
             "eyeLookOut","eyeLookOutLeft","eyeLookOutRight","eyeLookUp","eyeLookUpLeft","eyeLookUpRight","eyeSquint","eyeSquintLeft","eyeSquintRight","eyeWide","eyeWideLeft","eyeWideRight"],
    "Cheeks": ["cheekPuff","cheekSquint","cheekSquintLeft","cheekSquintRight"],
    "Jaw": ["jawForward","jawLeft","jawRight","jawOpen"],
    "Mouth": ["mouthClose","mouthFunnel","mouthPucker","mouthLeft","mouthRight","mouthSmile","mouthSmileLeft","mouthSmileRight",
              "mouthFrown","mouthFrownLeft","mouthFrownRight","mouthDimple","mouthDimpleLeft","mouthDimpleRight","mouthStretch","mouthStretchLeft","mouthStretchRight",
              "mouthRollLower","mouthRollUpper","mouthShrugLower","mouthShrugUpper","mouthPress","mouthPressLeft","mouthPressRight",
              "mouthLowerDown","mouthLowerDownLeft","mouthLowerDownRight","mouthUpperUp","mouthUpperUpLeft","mouthUpperUpRight", "tongueOut"],
    "Nose": ["noseSneer","noseSneerRight","noseSneerLeft"],
    "Other": []
}

# --------------------------------------------------
# UTILITIES
# --------------------------------------------------

def sync_all_active_indices(context, shape_name):
    """Ensures the blue selection bar moves on Driver and all Driven meshes."""
    scene = context.scene
    objs = [t.obj for t in scene.ak_targets if t.obj]
    if scene.ak_driver_mesh:
        objs.append(scene.ak_driver_mesh)
        
    for o in objs:
        if o and o.type == 'MESH' and o.data.shape_keys:
            idx = o.data.shape_keys.key_blocks.find(shape_name)
            if idx != -1:
                o.active_shape_key_index = idx
                
def autosort_shapes_logic(context):
    """Sorts shapes into ARKit folders by template order, then moves extras to 'Other'."""
    scene = context.scene
    master = scene.ak_driver_mesh
    
    if not master or not master.data.shape_keys:
        return

    kb = master.data.shape_keys.key_blocks
    assigned_names = set()

    # 1. Clear current CSV strings to rebuild them in the correct order
    for g in scene.ak_groups:
        g.shapes_csv = ""

    # 2. Re-populate groups based on ARKIT_DEFAULTS order
    for folder_name, shape_template_list in ARKIT_DEFAULTS.items():
        # Ensure folder exists
        group = next((g for g in scene.ak_groups if g.name == folder_name), None)
        if not group:
            group = scene.ak_groups.add()
            group.name = folder_name
        
        # Extract only shapes that exist on the mesh, in the order of the template
        found_in_mesh = [s for s in shape_template_list if s in kb and s != "Basis"]
        assigned_names.update(found_in_mesh)
        
        group.shapes_csv = ",".join(found_in_mesh)

    # 3. Combined "Refresh" Logic: Gather everything not in ARKIT_DEFAULTS
    other_group = next((g for g in scene.ak_groups if g.name == "Other"), None)
    if not other_group:
        other_group = scene.ak_groups.add()
        other_group.name = "Other"
        
    # Find every shape key that wasn't assigned to a standard folder
    unassigned = [key.name for key in kb if key.name != "Basis" and key.name not in assigned_names]
    
    # Sort custom shapes alphabetically so L/R pairs stay together in 'Other'
    unassigned.sort()
    other_group.shapes_csv = ",".join(unassigned)

    # 4. Clean up UI order (Move 'Other' to bottom)
    other_idx = scene.ak_groups.find("Other")
    if other_idx != -1:
        scene.ak_groups.move(other_idx, len(scene.ak_groups) - 1)

# --------------------------------------------------
# DATA MODELS
# --------------------------------------------------

class AK_GroupItem(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(name="Group Name")
    is_expanded: bpy.props.BoolProperty(name="Expanded", default=True)
    shapes_csv: bpy.props.StringProperty(name="Shapes", default="")

class AK_Target(bpy.types.PropertyGroup):
    obj: bpy.props.PointerProperty(type=bpy.types.Object)

# --------------------------------------------------
# OPERATORS
# --------------------------------------------------

class AK_OT_select_shape_key(bpy.types.Operator):
    bl_idname = "ak.select_shape_key"
    bl_label = "Select Blendshape"
    bl_description = "Selects this blendshape on the driver and all driven meshes"
    
    shape_name: bpy.props.StringProperty()
    
    def execute(self, context):
        sync_all_active_indices(context, self.shape_name)
        return {'FINISHED'}

class AK_OT_add_arkit_shapes(bpy.types.Operator):
    bl_idname = "ak.add_arkit_shapes"
    bl_label = "Add ARKit Shapes (Batch)"
    bl_description = "Add the 52 default ARKit blendshapes and pre-mirrored base versions."
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        scene = context.scene
        target = scene.ak_driver_mesh
            
        if not target:
            self.report({'WARNING'}, "Assign main Driver mesh first.")
            return {'CANCELLED'}

        all_arkit = [s for g in ARKIT_DEFAULTS.values() for s in g]
        
        if not target.data.shape_keys: target.shape_key_add(name="Basis")
        kb = target.data.shape_keys.key_blocks
        for s_name in all_arkit:
            if s_name not in kb: target.shape_key_add(name=s_name)
            kb[s_name].value = 0.0

        autosort_shapes_logic(context)

        return {'FINISHED'}
    
class AK_OT_autosort_shapes(bpy.types.Operator):
    bl_idname = "ak.autosort_shapes"
    bl_label = "Autosort Shapes"
    bl_description = "Sorts shapes into ARKit folders by template order, then moves extras to 'Other'."
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        autosort_shapes_logic(context)

        return {'FINISHED'}

class AK_OT_mirror_blendshape(bpy.types.Operator):
    bl_idname = "ak.mirror_blendshape"
    bl_label = "Split Left/Right"
    bl_description = "Splits the shape based on X axis across Driver and Driven meshes."
    bl_options = {'REGISTER', 'UNDO'}
    
    shape_name: bpy.props.StringProperty()
    
    def execute(self, context):
        scene = context.scene
        
        # Gather ALL meshes (Driver + Targets)
        objs = [t.obj for t in scene.ak_targets if t.obj]
        driver_obj = scene.ak_driver_mesh
        if driver_obj: objs.append(driver_obj)
        
        if not objs: return {'CANCELLED'}
        
        left_name = self.shape_name + "Left"
        right_name = self.shape_name + "Right"
        
        for obj in objs:
            if not obj or obj.type != 'MESH' or not obj.data.shape_keys: continue
            
            kb = obj.data.shape_keys.key_blocks
            target_shape = kb.get(self.shape_name)
            basis = kb.get("Basis")
            
            # Skip if this specific mesh doesn't have the shape we're trying to split
            if not target_shape or not basis: continue
            
            # 1. Delete existing L/R shapes on this mesh if they exist
            for n in (left_name, right_name):
                if n in kb:
                    obj.shape_key_remove(kb[n])
                    
            # 2. Create new Left and Right shapes
            left_shape = obj.shape_key_add(name=left_name, from_mix=False)
            right_shape = obj.shape_key_add(name=right_name, from_mix=False)
            
            # 3. Split based on X-Axis coordinates relative to THIS mesh's basis
            for i, v in enumerate(obj.data.vertices):
                b_co = basis.data[i].co
                s_co = target_shape.data[i].co
                
                if b_co.x > 0.001:
                    left_shape.data[i].co = s_co
                    right_shape.data[i].co = b_co
                elif b_co.x < -0.001:
                    left_shape.data[i].co = b_co
                    right_shape.data[i].co = s_co
                else:
                    left_shape.data[i].co = s_co
                    right_shape.data[i].co = s_co
                    
        # 4. DIRECT DRIVER LINKING
        if driver_obj and driver_obj.data.shape_keys:
            for t in scene.ak_targets:
                tar = t.obj
                if not tar or not tar.data.shape_keys: continue
                
                # Only link the two new shapes we just made
                for n in (left_name, right_name):
                    t_kb = tar.data.shape_keys.key_blocks.get(n)
                    if t_kb:
                        t_kb.driver_remove("value")
                        drv = t_kb.driver_add("value").driver
                        drv.type = 'SUM'
                        var = drv.variables.new()
                        var.name = "src_val"
                        var.type = 'SINGLE_PROP'
                        # SAFELY TARGET THE OBJECT TO AVOID CRASHES
                        var.targets[0].id_type = 'OBJECT'
                        var.targets[0].id = driver_obj
                        var.targets[0].data_path = f'data.shape_keys.key_blocks["{n}"].value'
                
        autosort_shapes_logic(context)
        
        self.report({'INFO'}, f"Split {self.shape_name} into {left_name} and {right_name} and linked drivers.")
        return {'FINISHED'}

class AK_OT_select_all_meshes(bpy.types.Operator):
    bl_idname = "ak.select_all_meshes"
    bl_label = "Select All Rig Meshes"
    bl_description = "Selects the Driver and all Target meshes in the Viewport."
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        bpy.ops.object.select_all(action='DESELECT')
        scene = context.scene
        objs = [t.obj for t in scene.ak_targets if t.obj]
        
        if scene.ak_driver_mesh:
            objs.append(scene.ak_driver_mesh)
            context.view_layer.objects.active = scene.ak_driver_mesh
            
        for o in objs:
            o.select_set(True)
            
        return {'FINISHED'}

class AK_OT_create_drivers(bpy.types.Operator):
    bl_idname = "ak.create_drivers"; bl_label = "Link Meshes"
    bl_description = "Create drivers on the secondary meshes controlled by the shared blendshapes with the main mesh."
    def execute(self, context):
        scene = context.scene
        src = scene.ak_driver_mesh
        if not src or not src.data.shape_keys: return {'CANCELLED'}
        for item in scene.ak_targets:
            tar = item.obj
            if not tar or tar == src: continue
            if not tar.data.shape_keys: tar.shape_key_add(name="Basis")
            for key in src.data.shape_keys.key_blocks:
                t_kb = tar.data.shape_keys.key_blocks.get(key.name)
                if not t_kb: continue
                t_kb.driver_remove("value")
                drv = t_kb.driver_add("value").driver
                drv.type = 'SUM'
                var = drv.variables.new()
                var.name = "src_val"
                var.type = 'SINGLE_PROP'
                var.targets[0].id_type = 'KEY'
                var.targets[0].id = src.data.shape_keys
                var.targets[0].data_path = f'key_blocks["{key.name}"].value'
        return {'FINISHED'}

class AK_OT_remove_drivers(bpy.types.Operator):
    bl_idname = "ak.remove_drivers"; bl_label = "Unlink Meshes"
    bl_description = "Remove the driver setup on the secondary meshes' blendshapes."
    def execute(self, context):
        for item in context.scene.ak_targets:
            tar = item.obj
            if not tar or not tar.data.shape_keys: continue
            for kb in tar.data.shape_keys.key_blocks: kb.driver_remove("value")
        return {'FINISHED'}

class AK_OT_select_basis(bpy.types.Operator):
    bl_idname = "ak.select_basis"; bl_label = "Select Basis"
    bl_description = 'Set all the "Basis" blendshapes as active.'
    def execute(self, context):
        scene = context.scene
        objs = [t.obj for t in scene.ak_targets if t.obj]
        if scene.ak_driver_mesh: objs.append(scene.ak_driver_mesh)
        for o in objs:
            if o.data.shape_keys: o.active_shape_key_index = 0
        return {'FINISHED'}

class AK_OT_global_zero(bpy.types.Operator):
    bl_idname = "ak.global_zero"; bl_label = "Zero Everything"
    bl_description = "Set all the blendshapes' values to 0."
    def execute(self, context):
        scene = context.scene
        objs = [t.obj for t in scene.ak_targets if t.obj]
        if scene.ak_driver_mesh: objs.append(scene.ak_driver_mesh)
        for o in objs:
            if o.data.shape_keys:
                for kb in o.data.shape_keys.key_blocks: 
                    kb.value = 0.0
        return {'FINISHED'}

class AK_OT_delete_all_shapes(bpy.types.Operator):
    bl_idname = "ak.delete_all_shapes"; bl_label = "Delete All Shapes"
    bl_description = "Deletes ALL blendshapes and folders on all the meshes, even your custom ones. Be careful."
    bl_options = {'REGISTER', 'UNDO'}
    def execute(self, context):
        obj = context.scene.ak_driver_mesh
        if obj and obj.data.shape_keys: obj.shape_key_clear()
        for t in context.scene.ak_targets:
            if t.obj and t.obj.data.shape_keys: t.obj.shape_key_clear()
        
        # Clear folders so it doesn't leave ghost items
        context.scene.ak_groups.clear()
        return {'FINISHED'}
    def invoke(self, context, event): return context.window_manager.invoke_confirm(self, event)

class AK_OT_target_add_selected(bpy.types.Operator):
    bl_idname = "ak.target_add_selected"; bl_label = "Add Selected"
    bl_description = "Adds the selected mesh or meshes to the list as secondary meshes."
    bl_options = {'REGISTER', 'UNDO'}
    def execute(self, context):
        for o in context.selected_objects:
            if o.type == 'MESH' and o != context.scene.ak_driver_mesh:
                if not any(t.obj == o for t in context.scene.ak_targets):
                    context.scene.ak_targets.add().obj = o
        return {'FINISHED'}

class AK_OT_target_remove(bpy.types.Operator):
    bl_idname = "ak.target_remove"; bl_label = "Remove"
    bl_description = "Removes the selected mesh or meshes to the list from secondary meshes."
    bl_options = {'REGISTER', 'UNDO'}
    def execute(self, context):
        if context.scene.ak_target_index >= 0: context.scene.ak_targets.remove(context.scene.ak_target_index)
        return {'FINISHED'}

class AK_OT_groups_toggle(bpy.types.Operator):
    bl_idname = "ak.groups_toggle"; bl_label = "Toggle All"
    def execute(self, context):
        s = not any(g.is_expanded for g in context.scene.ak_groups)
        for g in context.scene.ak_groups: g.is_expanded = s
        return {'FINISHED'}

class AK_OT_delete_single_shape(bpy.types.Operator):
    bl_idname = "ak.delete_single_shape"
    bl_label = "Delete Blendshape?"
    bl_description = "Delete this blendshape."
    bl_options = {'REGISTER', 'UNDO'}
    
    shape_name: bpy.props.StringProperty()

    def execute(self, context):
        scene = context.scene
        
        # 1. Clean up targets FIRST to safely kill the drivers
        for t in scene.ak_targets:
            tar = t.obj
            if tar and tar.data.shape_keys:
                kb = tar.data.shape_keys.key_blocks.get(self.shape_name)
                if kb:
                    kb.driver_remove("value")
                    tar.shape_key_remove(kb)
                    
        # 2. Clean up the master driver mesh
        master = scene.ak_driver_mesh
        if master and master.data.shape_keys:
            kb = master.data.shape_keys.key_blocks.get(self.shape_name)
            if kb:
                master.shape_key_remove(kb)
                    
        autosort_shapes_logic(context)
        return {'FINISHED'}

    def invoke(self, context, event):
        return context.window_manager.invoke_confirm(self, event)

class AK_OT_add_single_arkit_shape(bpy.types.Operator):
    bl_idname = "ak.add_single_arkit_shape"
    bl_label = "Add Shapekey"
    bl_description = "Adds the selected shapekey to all the secondary meshes, if they dont have it yet."
    bl_options = {'REGISTER', 'UNDO'}

    shape_name: bpy.props.StringProperty()

    def execute(self, context):
        scene = context.scene
        targets = [t.obj for t in scene.ak_targets if t.obj]
            
        if not targets:
            self.report({'WARNING'}, "Assign Driver/Driven meshes first.")
            return {'CANCELLED'}

        for ob in targets:
            if ob.type != 'MESH': continue
            if not ob.data.shape_keys: ob.shape_key_add(name="Basis")
            
            kb = ob.data.shape_keys.key_blocks
            if self.shape_name not in kb: ob.shape_key_add(name=self.shape_name)
            kb[self.shape_name].value = 0.0

            t_kb = kb.get(self.shape_name)
            
            t_kb.driver_remove("value")
            drv = t_kb.driver_add("value").driver
            drv.type = 'SUM'
            var = drv.variables.new()
            var.name = "src_val"
            var.type = 'SINGLE_PROP'
            # SAFELY TARGET THE OBJECT TO AVOID CRASHES
            var.targets[0].id_type = 'OBJECT'
            var.targets[0].id = scene.ak_driver_mesh
            var.targets[0].data_path = f'data.shape_keys.key_blocks["{self.shape_name}"].value'

        autosort_shapes_logic(context)
        return {'FINISHED'}

# --------------------------------------------------
# UI PANELS
# --------------------------------------------------

class AK_UL_targets(bpy.types.UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        if item.obj: layout.label(text=item.obj.name, icon='MESH_DATA')

class AK_PT_panel(bpy.types.Panel):
    bl_label = "ARKit Blendshape Helper"
    bl_idname = "AK_PT_panel"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "ARKit H"

    def draw(self, context):
        layout = self.layout
        scene = context.scene
        master_obj = scene.ak_driver_mesh

        # 1. Mesh Setup
        m_box = layout.box()
        m_box.prop(scene, "ak_show_mesh_setup", icon='TRIA_DOWN' if scene.ak_show_mesh_setup else 'TRIA_RIGHT', text="Mesh Setup", emboss=False)
        if scene.ak_show_mesh_setup:
            meshCol = m_box.column(align=True)
            meshCol.prop(scene, "ak_driver_mesh", text="Driver")
            row = meshCol.row()
            row.template_list("AK_UL_targets", "", scene, "ak_targets", scene, "ak_target_index")
            col = row.column(align=True)
            col.operator("ak.target_add_selected", icon="ADD", text="")
            col.operator("ak.target_remove", icon="REMOVE", text="")
            
            row = meshCol.row(align=True)
            row.operator("ak.add_arkit_shapes", icon='SHAPEKEY_DATA',text="Add ARKit")
            row.operator("ak.delete_all_shapes", icon='ERROR',text="")

        # 2. General Global Controls
        row = layout.row(align=True)
        row.operator("ak.create_drivers", icon='CONSTRAINT', text="Driver Link")
        row.operator("ak.remove_drivers", icon='CANCEL', text="Driver Unlink")
        row = layout.row(align=True)
        row.operator("ak.select_all_meshes", icon='RESTRICT_SELECT_OFF', text="Select")
        row.operator("ak.select_basis", icon='SHAPEKEY_DATA', text="Basis")
        row.operator("ak.global_zero", icon='FILE_REFRESH', text="Zero All")
        
        if not master_obj or not master_obj.data.shape_keys:
            layout.label(text="Assign a Driver Mesh with shapes to see folders.", icon='INFO')
            return
        
        # 3. Blendshape Folders
        f_box = layout.box()
        header = f_box.row()
        header.prop(scene, "ak_show_folders_setup", icon='TRIA_DOWN' if scene.ak_show_folders_setup else 'TRIA_RIGHT', text="Blendshape Folders", emboss=False)
        
        tools = header.row(align=True)
        tools.operator("ak.autosort_shapes", icon='FILE_REFRESH', text="") 
        
        if scene.ak_show_folders_setup:
            for i, group in enumerate(scene.ak_groups):
                g_box = f_box.box()
                header = g_box.row(align=True)
                header.prop(group, "is_expanded", icon='TRIA_DOWN' if group.is_expanded else 'TRIA_RIGHT', text=group.name, emboss=False)
                
                if group.is_expanded:
                    col = g_box.column(align=True)
                    for s_n in group.shapes_csv.split(","):
                        if not s_n: continue
                        
                        # GRAB THE NATIVE KEYBLOCK DIRECTLY
                        kb = master_obj.data.shape_keys.key_blocks.get(s_n)
                        
                        if kb:
                            # Check if it's currently selected on the driver
                            is_active = (master_obj.active_shape_key_index == master_obj.data.shape_keys.key_blocks.find(s_n))
                            
                            row = col.row(align=True)
                            
                            if s_n.endswith("Left"):
                                row.separator(factor=1.6)
                                row.label(icon='EVENT_L')
                            elif s_n.endswith("Right"):
                                row.separator(factor=1.6)
                                row.label(icon='EVENT_R')
                            else:
                                split_op = row.operator("ak.mirror_blendshape", text="", icon='MOD_MIRROR')
                                split_op.shape_name = s_n 

                            # PASS THE KEYBLOCK DIRECTLY TO THE UI
                            row.prop(kb, "value", text=s_n)
                            
                            # ADD BUTTON
                            add_op = row.operator("ak.add_single_arkit_shape", text="", icon='ADD')
                            add_op.shape_name = s_n

                            # THE SELECT BUTTON
                            sel_icon = 'RESTRICT_SELECT_OFF' if is_active else 'RESTRICT_SELECT_ON'
                            sel_op = row.operator("ak.select_shape_key", text="", icon=sel_icon)
                            sel_op.shape_name = s_n
                            
                            # THE DELETE BUTTON
                            del_op = row.operator("ak.delete_single_shape", text="", icon='X')
                            del_op.shape_name = s_n
                
# --------------------------------------------------
# REGISTER
# --------------------------------------------------

classes = [
    AK_GroupItem, AK_Target, AK_UL_targets, AK_PT_panel, 
    AK_OT_add_arkit_shapes, AK_OT_create_drivers, AK_OT_remove_drivers, 
    AK_OT_select_basis, AK_OT_global_zero, AK_OT_delete_all_shapes, 
    AK_OT_target_add_selected, AK_OT_target_remove, 
    AK_OT_groups_toggle, AK_OT_mirror_blendshape,
    AK_OT_select_all_meshes, AK_OT_autosort_shapes, AK_OT_delete_single_shape,
    AK_OT_select_shape_key, AK_OT_add_single_arkit_shape,
    ]

def register():
    for c in classes: 
        bpy.utils.register_class(c)
    
    
    s = bpy.types.Scene
    s.ak_targets = bpy.props.CollectionProperty(type=AK_Target)
    s.ak_target_index = bpy.props.IntProperty()
    s.ak_groups = bpy.props.CollectionProperty(type=AK_GroupItem)
    s.ak_driver_mesh = bpy.props.PointerProperty(type=bpy.types.Object, name="Driver")
    s.ak_show_mesh_setup = bpy.props.BoolProperty(default=True)
    s.ak_show_folders_setup = bpy.props.BoolProperty(default=True)
    

def unregister():
    s = bpy.types.Scene
    del s.ak_targets
    del s.ak_target_index
    del s.ak_groups
    del s.ak_driver_mesh
    del s.ak_show_mesh_setup
    del s.ak_show_folders_setup

    for c in classes: 
        bpy.utils.unregister_class(c)
    
if __name__ == "__main__":
    register()


```

# Facial rendering source scan: PaoloESAN/gacha-setup

Repository: https://github.com/PaoloESAN/gacha-setup

Pinned source revision: 3e40423dbec489368696c4f89a2bfb285662cdc1

Discovery topic: facial rig

Description: Blender 5.2+ add-on for automated rigging, shaders, facial rigs, and texture pipelines for Gacha game models (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, NTE, WuWa and Arknights: Endfield)

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "setup_wizard/addon_updater/LICENSE.txt"]

Topic paths: {"eye": ["docs/src/assets/content/utilities/eye-through-hair1.mp4", "docs/src/assets/content/utilities/eye-through-hair2.mp4", "docs/src/assets/content/zzz/zzz-eyeshadow1.png", "docs/src/assets/content/zzz/zzz-eyeshadow2.png", "docs/src/assets/content/zzz/zzz-eyeshadow3.png", "setup_wizard/utils/eye_through_hair.py", "tests/test_eye_through_hair.py"], "mouth": ["setup_wizard/fix_mouth_outlines.py"], "shader": ["docs/src/content/docs/es/shaders.md", "docs/src/content/docs/shaders.md", "setup_wizard/character_rig_setup/LightingPanel_Shader_v3_4.blend", "setup_wizard/character_rig_setup/RootShape_Shader_v3_4.blend", "setup_wizard/domain/shader_configurator.py", "setup_wizard/domain/shader_identifier_service.py", "setup_wizard/domain/shader_material.py", "setup_wizard/domain/shader_material_name_keywords.py", "setup_wizard/domain/shader_material_names.py", "setup_wizard/domain/shader_node_inputs.py", "setup_wizard/domain/shader_node_names.py", "setup_wizard/domain/vfx_shader_types.py", "setup_wizard/shaders/ake/AKE.blend", "setup_wizard/shaders/gi/HoYoverse - PrimoToon v4.0.blend", "setup_wizard/shaders/gi/README.md", "setup_wizard/shaders/hsr/README.md", "setup_wizard/shaders/hsr/StellarToonV1.1.blend", "setup_wizard/shaders/nte/README.md", "setup_wizard/shaders/nte/YH Shader.blend", "setup_wizard/shaders/wuwa/Gustling Waters.blend", "setup_wizard/shaders/zzz/Belt.png", "setup_wizard/shaders/zzz/Kythera's ZZZ Shader V1.5.1.blend", "setup_wizard/shaders/zzz/README.md", "setup_wizard/shaders/zzz/ZZZ Setup File V2.0.blend"], "rig": ["docs/src/assets/content/ake/ake-facerig.png", "docs/src/assets/content/genshin/genshin-rigpanel.png", "docs/src/assets/content/hsr/hsr-facerig.png", "docs/src/assets/content/hsr/hsr-manualrig.png", "docs/src/assets/content/hsr/hsr-tworigs.png", "docs/src/assets/content/nte/nte-facerig.webp", "docs/src/assets/content/nte/nte-rig.png", "docs/src/assets/content/zzz/zzz-facerig.png", "docs/src/assets/content/zzz/zzz-tworigs.png", "docs/src/content/docs/ake/rigging.mdx", "docs/src/content/docs/es/ake/rigging.mdx", "docs/src/content/docs/es/genshin/rigging.mdx", "docs/src/content/docs/es/hsr/rigging.mdx", "docs/src/content/docs/es/nte/rigging.mdx", "docs/src/content/docs/es/rigging.md", "docs/src/content/docs/es/wuwa/rigging.mdx", "docs/src/content/docs/es/zzz/rigging.mdx", "docs/src/content/docs/genshin/rigging.mdx", "docs/src/content/docs/hsr/rigging.mdx", "docs/src/content/docs/nte/rigging.mdx", "docs/src/content/docs/rigging.md", "docs/src/content/docs/wuwa/rigging.mdx", "docs/src/content/docs/zzz/rigging.mdx", "setup_wizard/camera_rig/__init__.py", "setup_wizard/camera_rig/jideeh_cam_rig.py", "setup_wizard/character_rig_setup/LightingPanel.blend", "setup_wizard/character_rig_setup/LightingPanel_Shader_v3_4.blend", "setup_wizard/character_rig_setup/RootShape.blend", "setup_wizard/character_rig_setup/RootShape_Shader_v3_4.blend", "setup_wizard/character_rig_setup/ZZZ_Facerig.blend"], "face": ["docs/src/assets/content/ake/ake-facerig.png", "docs/src/assets/content/genshin/genshin-face.mp4", "docs/src/assets/content/hsr/hsr-facerig.png", "docs/src/assets/content/nte/nte-facerig.webp", "docs/src/assets/content/zzz/zzz-facerig.png", "setup_wizard/character_rig_setup/ZZZ_Facerig.blend", "setup_wizard/character_rig_setup/ake_face_rig.py", "setup_wizard/character_rig_setup/hsr_face_rig.py", "setup_wizard/character_rig_setup/isaacfacerig.blend", "setup_wizard/character_rig_setup/nte_face_rig.py", "setup_wizard/character_rig_setup/wuwa_face_panel.py", "setup_wizard/character_rig_setup/zzz_face_rig.py", "setup_wizard/mesh_import_setup/chibi_face_setup.py", "setup_wizard/services/isolation/face_widget_manager.py", "tests/facerig/__init__.py", "tests/facerig/ake.py", "tests/facerig/facerig_assertions.py", "tests/facerig/genshin.py", "tests/facerig/hsr.py", "tests/facerig/nte.py", "tests/facerig/wuwa.py", "tests/facerig/zzz.py"]}

## Source file: setup_wizard/character_rig_setup/ake_face_rig.py

{
  "path": "setup_wizard/character_rig_setup/ake_face_rig.py",
  "blob_sha": "f7866994168568ba5121a2616d23f1b644dea209",
  "sha256": "ed4d7dfeb971bc746bbf4e3979c2aa26f2123048ab4a739a1cc38c5e71fcd6e2",
  "lines": 631,
  "bytes": 27995,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 85,
    "jaw": 4,
    "mouth": 43,
    "teeth": 4,
    "tooth": 3,
    "tongue": 5,
    "deform": 84,
    "iris": 2,
    "rig": 138,
    "face": 117,
    "lip": 23
  },
  "url": "https://github.com/PaoloESAN/gacha-setup/blob/3e40423dbec489368696c4f89a2bfb285662cdc1/setup_wizard/character_rig_setup/ake_face_rig.py",
  "function_signature_candidates": [
    {
      "line": 102,
      "text": "def setup_endfield_isaac_face_rig(body_rig, context=None):"
    },
    {
      "line": 618,
      "text": "def execute(self, context):"
    }
  ]
}

```text

# Author: michael-gh1, PaoloESAN
# Arknights: Endfield - Isaac Face Rig Integration

import os
import bpy
from bpy.types import Operator
from mathutils import Vector, Matrix

from setup_wizard.domain.game_types import GameType
from setup_wizard.setup_wizard_operator_base_classes import BasicSetupUIOperator


ENDFIELD_FACE_MAPPING = {
    # Eyebrows Left
    "browLf01Joint": {"master": "Eyebrow-Master.L", "deform": "brow_Inn_L"},
    "browLf02Joint": {"master": "Eyebrow-Master.L", "deform": "brow_Inn_L"},
    "browLf03Joint": {"master": "Eyebrow-Master.L", "deform": "brow_Mid_L"},
    "browLf04Joint": {"master": "Eyebrow-Master.L", "deform": "brow_Mid_L"},
    "browLf05Joint": {"master": "Eyebrow-Master.L", "deform": "brow_Out_L"},
    "browLineLf01Joint": {"master": "Eyebrow-Master.L", "deform": "eyebrow_Inn_L"},
    "browLineLf02Joint": {"master": "Eyebrow-Master.L", "deform": "eyebrow_Mid_L"},
    "browLineLf03Joint": {"master": "Eyebrow-Master.L", "deform": "eyebrow_Out_L"},

    # Eyebrows Right
    "browRt01Joint": {"master": "Eyebrow-Master.R", "deform": "brow_Inn_R"},
    "browRt02Joint": {"master": "Eyebrow-Master.R", "deform": "brow_Inn_R"},
    "browRt03Joint": {"master": "Eyebrow-Master.R", "deform": "brow_Mid_R"},
    "browRt04Joint": {"master": "Eyebrow-Master.R", "deform": "brow_Mid_R"},
    "browRt05Joint": {"master": "Eyebrow-Master.R", "deform": "brow_Out_R"},
    "browLineRt01Joint": {"master": "Eyebrow-Master.R", "deform": "eyebrow_Inn_R"},
    "browLineRt02Joint": {"master": "Eyebrow-Master.R", "deform": "eyebrow_Mid_R"},
    "browLineRt03Joint": {"master": "Eyebrow-Master.R", "deform": "eyebrow_Out_R"},

    # Eyes & Eyelids Left
    "eyeLf01Joint": {"master": "Eye-Master.L", "deform": "eyeconner_inn_L"},
    "eyeLf02Joint": {"master": "Eye-Master.L", "deform": "eyeup_inn_L"},
    "eyeLf03Joint": {"master": "Eye-Master.L", "deform": "eyeup_M_L"},
    "eyeLf04Joint": {"master": "Eye-Master.L", "deform": "eyeup_out_L"},
    "eyeLf05Joint": {"master": "Eye-Master.L", "deform": "eyeconner_out_L"},
    "eyeLf06Joint": {"master": "Eye-Master.L", "deform": "eyedn_out_L"},
    "eyeLf07Joint": {"master": "Eye-Master.L", "deform": "eyedn_M_L"},
    "eyeLf08Joint": {"master": "Eye-Master.L", "deform": "eyedn_inn_L"},

    # Eyes & Eyelids Right
    "eyeRt01Joint": {"master": "Eye-Master.R", "deform": "eyeconner_inn_R"},
    "eyeRt02Joint": {"master": "Eye-Master.R", "deform": "eyeup_inn_R"},
    "eyeRt03Joint": {"master": "Eye-Master.R", "deform": "eyeup_M_R"},
    "eyeRt04Joint": {"master": "Eye-Master.R", "deform": "eyeup_out_R"},
    "eyeRt05Joint": {"master": "Eye-Master.R", "deform": "eyeconner_out_R"},
    "eyeRt06Joint": {"master": "Eye-Master.R", "deform": "eyedn_out_R"},
    "eyeRt07Joint": {"master": "Eye-Master.R", "deform": "eyedn_M_R"},
    "eyeRt08Joint": {"master": "Eye-Master.R", "deform": "eyedn_inn_R"},

    # Mouth & Lips
    "lipMupJoint": {"mouth_chain": True, "deform": "lipUpper_M"},
    "lipMdnJoint": {"mouth_chain": True, "deform": "lipLower_M"},
    "lipLup1Joint": {"mouth_chain": True, "deform": "lipConner_L"},
    "lipLup2Joint": {"mouth_chain": True, "deform": "lipUpper_01_L"},
    "lipLup3Joint": {"mouth_chain": True, "deform": "lipUpper_00_L"},
    "lipLup4Joint": {"mouth_chain": True, "deform": "lipUpper_00_L"},
    "lipLdn1Joint": {"mouth_chain": True, "deform": "lipConner_L"},
    "lipLdn2Joint": {"mouth_chain": True, "deform": "lipLower_01_L"},
    "lipLdn3Joint": {"mouth_chain": True, "deform": "lipLower_00_L"},
    "lipLdn4Joint": {"mouth_chain": True, "deform": "lipLower_00_L"},
    "lipRup1Joint": {"mouth_chain": True, "deform": "lipConner_R"},
    "lipRup2Joint": {"mouth_chain": True, "deform": "lipUpper_01_R"},
    "lipRup3Joint": {"mouth_chain": True, "deform": "lipUpper_00_R"},
    "lipRup4Joint": {"mouth_chain": True, "deform": "lipUpper_00_R"},
    "lipRdn1Joint": {"mouth_chain": True, "deform": "lipConner_R"},
    "lipRdn2Joint": {"mouth_chain": True, "deform": "lipLower_01_R"},
    "lipRdn3Joint": {"mouth_chain": True, "deform": "lipLower_00_R"},
    "lipRdn4Joint": {"mouth_chain": True, "deform": "lipLower_00_R"},

    # Cheeks Left & Right
    "faceLfCheekOtJoint": {"mouth_master": True, "deform": "chhek_L"},
    "faceLfCheekOtUpJoint": {"mouth_master": True, "deform": "chhek_L"},
    "faceLfCheekOtDnJoint": {"mouth_master": True, "deform": "mouthUpper_03_L"},
    "faceLfCheekOtInJoint": {"mouth_master": True, "deform": "mouthUpper_01_L"},
    "faceRtCheekOtJoint": {"mouth_master": True, "deform": "chhek_R"},
    "faceRtCheekOtUpJoint": {"mouth_master": True, "deform": "chhek_R"},
    "faceRtCheekOtDnJoint": {"mouth_master": True, "deform": "mouthUpper_03_R"},
    "faceRtCheekOtInJoint": {"mouth_master": True, "deform": "mouthUpper_01_R"},

    # Jaw & Teeth
    "jawJoint": {"deform": "JawdnEnd_M"},
    "faceMdJawDnJoint": {"deform": "JawdnEnd_M"},
    "faceMdToothUpJoint": {"mouth_chain": True, "deform": "UpTeeth"},
    "faceMdToothDnJoint": {"mouth_chain": True, "deform": "DownTeeth"},
    "line_toothJoint": {"mouth_chain": True, "deform": "DownTeeth"},

    # Tongue
    "TongueMd01Joint": {"mouth_chain": True, "deform": "tongue_01"},
    "TongueMd02Joint": {"mouth_chain": True, "deform": "tongue_02"},
    "TongueMd03Joint": {"mouth_chain": True, "deform": "tongue_03"},
    "TongueMd04Joint": {"mouth_chain": True, "deform": "tongue_04"},

    # Nose
    "NoseMd01Joint": {"deform": "nosetip_M"},
}


def setup_endfield_isaac_face_rig(body_rig, context=None):
    """Sets up the Isaac Face Rig controls for Arknights: Endfield characters.
    
    Preserves original Endfield bone positions and geometry while binding
    face deformations to Isaac FaceRig control widgets.
    """
    if not body_rig or body_rig.type != 'ARMATURE':
        print("[AKE FACE RIG] Error: Invalid body armature provided.")
        return None

    if not context:
        context = bpy.context

    # Idempotency (HSR parity): an already-fused body rig carries the FaceRig
    # bones itself; fusing again would only spawn .001 dupes.
    try:
        if "Eye-Track-Master" in body_rig.data.bones and not bpy.data.objects.get("isaac FaceRig"):
            print("[AKE FACE RIG] FaceRig already fused into body rig, skipping.")
            return body_rig
    except Exception:
        pass

    blend_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'isaacfacerig.blend')
    if not os.path.exists(blend_path):
        print(f"[AKE FACE RIG] Error: File not found: {blend_path}")
        return None

    objects_before = set(bpy.data.objects)
    facerig_obj = None
    appended_coll = None

    # Check if isaac FaceRig already exists in the scene
    facerig_obj = bpy.data.objects.get("isaac FaceRig")
    if not facerig_obj:
        for obj in bpy.data.objects:
            if obj.type == 'ARMATURE' and any(k in obj.name.lower() for k in ['facerig', 'isaac']):
                facerig_obj = obj
                break

    # If not already in scene, append from isaacfacerig.blend
    if not facerig_obj:
        try:
            with bpy.data.libraries.load(blend_path, link=False) as (data_from, data_to):
                if data_from.collections:
                    first_coll_name = data_from.collections[0]
                    data_to.collections = [first_coll_name]
                    print(f"[AKE FACE RIG] Loading collection from blend: '{first_coll_name}'")

            for collection in data_to.collections:
                if collection:
                    appended_coll = collection
                    if collection.name not in context.scene.collection.children:
                        context.scene.collection.children.link(collection)
        except Exception as err:
            print(f"[AKE FACE RIG] Library load error: {err}")

        new_objects = set(bpy.data.objects) - objects_before
        for obj in new_objects:
            if obj.type == 'ARMATURE':
                facerig_obj = obj
                break

        if not facerig_obj:
            facerig_obj = bpy.data.objects.get("isaac FaceRig")

    if not facerig_obj:
        print("[AKE FACE RIG] Error: Could not find or append 'isaac FaceRig' armature.")
        return None

    print(f"[AKE FACE RIG] Using FaceRig armature: '{facerig_obj.name}'")

    # Organize collections: FaceRig armature to character collection,
    # widget planes to per-character WGTS_<Char> nested inside it (Append-safe)
    target_armature_coll = body_rig.users_collection[0] if body_rig.users_collection else context.scene.collection
    char_name = target_armature_coll.name if target_armature_coll != context.scene.collection else (body_rig.name.replace("Rig", "") or "Character")
    wgts_name = f"WGTS_{char_name}"
    wgt_coll = bpy.data.collections.get(wgts_name)
    if not wgt_coll:
        wgt_coll = bpy.data.collections.new(wgts_name)
    if wgt_coll.name not in target_armature_coll.children:
        try:
            target_armature_coll.children.link(wgt_coll)
        except Exception:
            pass
    if wgt_coll.name in context.scene.collection.children:
        try:
            context.scene.collection.children.unlink(wgt_coll)
        except Exception:
            pass
    try:
        wgt_coll.hide_viewport = True
        wgt_coll.hide_select = True
        wgt_coll.hide_render = True
    except Exception:
        pass

    if facerig_obj and target_armature_coll:
        if facerig_obj.name not in target_armature_coll.objects:
            target_armature_coll.objects.link(facerig_obj)
        for coll in list(facerig_obj.users_collection):
            if coll != target_armature_coll:
                coll.objects.unlink(facerig_obj)

    plane_objs = [
        obj for obj in bpy.data.objects
        if obj.type == 'MESH' and ('Plane.' in obj.name or 'wgt' in obj.name.lower()) and obj != facerig_obj
    ]
    if appended_coll:
        plane_objs.extend([obj for obj in appended_coll.objects if obj != facerig_obj and obj not in plane_objs])

    for p_obj in plane_objs:
        if p_obj.name not in wgt_coll.objects:
            wgt_coll.objects.link(p_obj)
        for coll in list(p_obj.users_collection):
            if coll != wgt_coll:
                coll.objects.unlink(p_obj)

    # Clean up appended collection if necessary
    if appended_coll:
        try:
            for parent_coll in bpy.data.collections:
                if appended_coll.name in parent_coll.children:
                    parent_coll.children.unlink(appended_coll)
            if appended_coll.name in context.scene.collection.children:
                context.scene.collection.children.unlink(appended_coll)
            bpy.data.collections.remove(appended_coll, do_unlink=True)
            print(f"[AKE FACE RIG] Cleaned up temporary collection '{appended_coll.name}'")
        except Exception as c_err:
            print(f"[AKE FACE RIG] Collection cleanup notice: {c_err}")

    # Set display properties
    facerig_obj.show_in_front = True

    # Identify head bone on body rig
    body_head_bone_name = None
    for candidate in ["DEF-spine.006", "head", "Head", "Head_M", "Bip001_Head"]:
        if candidate in body_rig.data.bones:
            body_head_bone_name = candidate
            break
    if not body_head_bone_name:
        for b in body_rig.data.bones.keys():
            if "head" in b.lower() or "spine.006" in b.lower():
                body_head_bone_name = b
                break

    if not body_head_bone_name:
        print("[AKE FACE RIG] Error: Could not find head bone on body rig.")
        return None

    facerig_head_bone_name = "DEF-spine.006" if "DEF-spine.006" in facerig_obj.data.bones else facerig_obj.data.bones[0].name

    # Align FaceRig world matrix to body head bone
    try:
        body_head_matrix_world = body_rig.matrix_world @ body_rig.pose.bones[body_head_bone_name].matrix
        facerig_head_matrix_local = facerig_obj.pose.bones[facerig_head_bone_name].matrix
        facerig_obj.matrix_world = body_head_matrix_world @ facerig_head_matrix_local.inverted()
    except Exception as e:
        print(f"[AKE FACE RIG] Matrix alignment warning: {e}")

    context.view_layer.update()

    # HSR parity: no head-follow constraint on the FaceRig (it would
    # self-target after the fuse below). DEF-spine.006 is removed from the
    # FaceRig instead, right before joining.
    context.view_layer.update()

    # Precisely align Eye-Track-Master, Eye-Track, Eye-Track-Follow, and Eye-Scale-Control to character's eye coordinates
    try:
        pb_eye_l = body_rig.pose.bones.get("eye.L") or body_rig.pose.bones.get("DEF-eye.L")
        pb_eye_r = body_rig.pose.bones.get("eye.R") or body_rig.pose.bones.get("DEF-eye.R")
        if pb_eye_l and pb_eye_r:
            eye_l_world = body_rig.matrix_world @ pb_eye_l.head
            eye_r_world = body_rig.matrix_world @ pb_eye_r.head
            eye_center_world = (eye_l_world + eye_r_world) / 2.0

            facerig_inv = facerig_obj.matrix_world.inverted()
            eye_l_local = facerig_inv @ eye_l_world
            eye_r_local = facerig_inv @ eye_r_world
            eye_center_local = (eye_l_local + eye_r_local) / 2.0

            orig_active = context.view_layer.objects.active
            orig_mode = facerig_obj.mode if facerig_obj.mode in ('OBJECT', 'EDIT', 'POSE') else 'OBJECT'
            context.view_layer.objects.active = facerig_obj
            bpy.ops.object.mode_set(mode='EDIT')
            ebs = facerig_obj.data.edit_bones

            # 1. Align Eye-Track-Master (the big eye rectangle) directly centered to eye height and midpoint
            eb_master = ebs.get("Eye-Track-Master")
            if eb_master:
                dx = eye_center_local.x - eb_master.head.x
                dz = eye_center_local.z - eb_master.head.z
                eb_master.head.x += dx
                eb_master.head.z += dz
                eb_master.tail.x += dx
                eb_master.tail.z += dz

            # 2. Align Eye-Scale-Control, Eye-Track, and Eye-Track-Follow for each eye
            for side, pos_local in [("L", eye_l_local), ("R", eye_r_local)]:
                # Eye scale control box: MUST stay at Eye-Track-Master depth Y (inside the blue rectangle)
                eb_scale = ebs.get(f"Eye-Scale-Control.{side}")
                if eb_scale and eb_master:
                    dx = pos_local.x - eb_scale.head.x
                    dz = pos_local.z - eb_scale.head.z
                    dy = eb_master.head.y - eb_scale.head.y
                    eb_scale.head.x += dx
                    eb_scale.head.z += dz
                    eb_scale.head.y += dy
                    eb_scale.tail.x += dx
                    eb_scale.tail.z += dz
                    eb_scale.tail.y += dy

                # Eye-Track gaze pivot centered on the eye, pointing straight forward with zero tilt
                eb_track = ebs.get(f"Eye-Track.{side}")
                if eb_track:
                    track_len = eb_track.length or 0.05
                    eb_track.head = pos_local.copy()
                    eb_track.tail = Vector((pos_local.x, pos_local.y - track_len, pos_local.z))
                    eb_track.roll = 0.0

                # Eye-Track-Follow target aligned in X and Z with the eye, at Eye-Track-Master depth Y
                eb_follow = ebs.get(f"Eye-Track-Follow.{side}")
                if eb_follow and eb_master:
                    dx = pos_local.x - eb_follow.head.x
                    dz = pos_local.z - eb_follow.head.z
                    dy = eb_master.head.y - eb_follow.head.y
                    eb_follow.head.x += dx
                    eb_follow.head.z += dz
                    eb_follow.head.y += dy
                    eb_follow.tail.x += dx
                    eb_follow.tail.z += dz
                    eb_follow.tail.y += dy

            bpy.ops.object.mode_set(mode=orig_mode)
            if orig_active:
                context.view_layer.objects.active = orig_active

            # NOTE: Child Of inverse on Eye-Track-Follow is fixed AFTER the fuse
            # (in body-rig space), and the lowered flag moves to the body rig.
            print("[AKE FACE RIG] Perfectly aligned Eye-Track-Master, Eye-Track, Eye-Track-Follow, and Eye-Scale-Control to character eye coordinates.")
    except Exception as align_err:
        print(f"[AKE FACE RIG] Warning while aligning eye widgets: {align_err}")

    context.view_layer.update()

    # Fuse the FaceRig into the body rig (HSR parity): a single armature
    # travels as one collection unit on Append instead of two rigs.
    # The FaceRig carries junk drivers from the template character in the
    # .blend (same file HSR fuses), which would clobber rig properties.
    try:
        facerig_obj.animation_data_clear()
    except Exception:
        pass
    try:
        prev_active = context.view_layer.objects.active
        try:
            facerig_obj.hide_viewport = False
            facerig_obj.hide_set(False)
        except Exception:
            pass
        context.view_layer.objects.active = facerig_obj
        bpy.ops.object.mode_set(mode='EDIT')
        eb_dupe = facerig_obj.data.edit_bones.get(facerig_head_bone_name)
        if eb_dupe:
            facerig_obj.data.edit_bones.remove(eb_dupe)
            print(f"[AKE FACE RIG] Removed '{facerig_head_bone_name}' from FaceRig to avoid collision on join.")
        bpy.ops.object.mode_set(mode='OBJECT')
        if prev_active:
            try:
                context.view_layer.objects.active = prev_active
            except Exception:
                pass
    except Exception as ex_undupe:
        print(f"[AKE FACE RIG] FaceRig dedupe notice: {ex_undupe}")
        try:
            bpy.ops.object.mode_set(mode='OBJECT')
        except Exception:
            pass
    try:
        bpy.ops.object.mode_set(mode='OBJECT')
    except Exception:
        pass
    try:
        for o in (facerig_obj, body_rig):
            try:
                o.hide_viewport = False
                o.hide_set(False)
            except Exception:
                pass
        bpy.ops.object.select_all(action='DESELECT')
        facerig_obj.select_set(True)
        body_rig.select_set(True)
        context.view_layer.objects.active = body_rig
        bpy.ops.object.join()
        print(f"[AKE FACE RIG] Successfully fused FaceRig into '{body_rig.name}'")
    except Exception as ex_join:
        print(f"[AKE FACE RIG] Error joining FaceRig into body rig: {ex_join}")
        return None
    facerig_obj = body_rig
    context.view_layer.update()

    # Parent the facial root to the body head bone (HSR parity).
    try:
        context.view_layer.objects.active = body_rig
        bpy.ops.object.mode_set(mode='EDIT')
        head_eb = body_rig.data.edit_bones.get(body_head_bone_name)
        if head_eb:
            for bname in ["joint_face"]:
                eb = body_rig.data.edit_bones.get(bname)
                if eb:
                    eb.parent = head_eb
                    print(f"[AKE FACE RIG] Parented '{bname}' to '{head_eb.name}'")
        bpy.ops.object.mode_set(mode='OBJECT')
    except Exception as ex_parent:
        print(f"[AKE FACE RIG] Face root parenting notice: {ex_parent}")
        try:
            bpy.ops.object.mode_set(mode='OBJECT')
        except Exception:
            pass

    # Sanity: warn about mapping bones missing after the fuse.
    try:
        wanted = {"Mouth-Master", "Lip-Master", "Eye-Track-Master"}
        for mapping in ENDFIELD_FACE_MAPPING.values():
            if mapping.get("deform"):
                wanted.add(mapping["deform"])
            if mapping.get("master"):
                wanted.add(mapping["master"])
        missing = sorted(b for b in wanted if b and b not in body_rig.data.bones)
        if missing:
            print(f"[AKE FACE RIG] Warning: fused rig is missing face bones: {missing}")
    except Exception:
        pass

    # Bind Endfield facial bones to Isaac FaceRig
    assigned_count = 0
    for bone_name, mapping in ENDFIELD_FACE_MAPPING.items():
        pb = body_rig.pose.bones.get(bone_name)
        if not pb:
            continue

        # Remove existing Isaac constraints to allow clean idempotent re-runs
        for c in list(pb.constraints):
            if c.name.startswith("Isaac_"):
                pb.constraints.remove(c)

        btype = mapping.get("type", "TRANSFORMS")

        if btype == "ROTATION":
            c = pb.constraints.new('COPY_ROTATION')
            c.name = "Isaac_CopyRotation"
            c.target = facerig_obj
            c.subtarget = mapping["deform"]
            c.owner_space = 'LOCAL'
            c.target_space = 'LOCAL_OWNER_ORIENT'
            assigned_count += 1

        elif btype == "SCALE":
            c = pb.constraints.new('COPY_SCALE')
            c.name = "Isaac_CopyScale"
            c.target = facerig_obj
            c.subtarget = mapping["deform"]
            c.owner_space = 'LOCAL'
            c.target_space = 'LOCAL'
            assigned_count += 1

        elif mapping.get("mouth_chain"):
            # Mouth chain: Mouth-Master -> Lip-Master -> Specific Deform Bone
            c1 = pb.constraints.new('COPY_TRANSFORMS')
            c1.name = "Isaac_MouthMaster"
            c1.target = facerig_obj
            c1.subtarget = "Mouth-Master"
            c1.owner_space = 'LOCAL'
            c1.target_space = 'LOCAL_OWNER_ORIENT'

            c2 = pb.constraints.new('COPY_TRANSFORMS')
            c2.name = "Isaac_LipMaster"
            c2.target = facerig_obj
            c2.subtarget = "Lip-Master"
            c2.owner_space = 'LOCAL'
            c2.target_space = 'LOCAL_OWNER_ORIENT'
            c2.mix_mode = 'AFTER'

            c3 = pb.constraints.new('COPY_TRANSFORMS')
            c3.name = f"Isaac_Deform_{mapping['deform']}"
            c3.target = facerig_obj
            c3.subtarget = mapping["deform"]
            c3.owner_space = 'LOCAL'
            c3.target_space = 'LOCAL_OWNER_ORIENT'
            c3.mix_mode = 'AFTER'
            assigned_count += 1

        elif mapping.get("mouth_master"):
            # Cheeks: Mouth-Master -> Specific Deform Bone
            c1 = pb.constraints.new('COPY_TRANSFORMS')
            c1.name = "Isaac_MouthMaster"
            c1.target = facerig_obj
            c1.subtarget = "Mouth-Master"
            c1.owner_space = 'LOCAL'
            c1.target_space = 'LOCAL_OWNER_ORIENT'

            c2 = pb.constraints.new('COPY_TRANSFORMS')
            c2.name = f"Isaac_Deform_{mapping['deform']}"
            c2.target = facerig_obj
            c2.subtarget = mapping["deform"]
            c2.owner_space = 'LOCAL'
            c2.target_space = 'LOCAL_OWNER_ORIENT'
            c2.mix_mode = 'AFTER'
            assigned_count += 1

        elif "master" in mapping:
            # Eyebrows / Eyelids: Master Bone -> Specific Deform Bone
            c1 = pb.constraints.new('COPY_TRANSFORMS')
            c1.name = f"Isaac_Master_{mapping['master']}"
            c1.target = facerig_obj
            c1.subtarget = mapping["master"]
            c1.owner_space = 'LOCAL'
            c1.target_space = 'LOCAL_OWNER_ORIENT'

            c2 = pb.constraints.new('COPY_TRANSFORMS')
            c2.name = f"Isaac_Deform_{mapping['deform']}"
            c2.target = facerig_obj
            c2.subtarget = mapping["deform"]
            c2.owner_space = 'LOCAL'
            c2.target_space = 'LOCAL_OWNER_ORIENT'
            c2.mix_mode = 'AFTER'
            assigned_count += 1

        else:
            # Jaw, Nose, etc.
            c = pb.constraints.new('COPY_TRANSFORMS')
            c.name = f"Isaac_Deform_{mapping['deform']}"
            c.target = facerig_obj
            c.subtarget = mapping["deform"]
            c.owner_space = 'LOCAL'
            c.target_space = 'LOCAL_OWNER_ORIENT'
            assigned_count += 1

    # Direct binding for eye.L and eye.R (Eye Tracking movement and Eye Scale)
    for side in ("L", "R"):
        eye_name = f"eye.{side}"
        pb_eye = body_rig.pose.bones.get(eye_name) or body_rig.pose.bones.get(f"DEF-{eye_name}")
        if pb_eye:
            for c in list(pb_eye.constraints):
                if c.name.startswith("Isaac_"):
                    pb_eye.constraints.remove(c)

            # 1. Eye Rotation (looking around): Rotates with Isaac's Eye-Track.L / Eye-Track.R
            c_rot = pb_eye.constraints.new('COPY_ROTATION')
            c_rot.name = f"Isaac_EyeRot_{side}"
            c_rot.target = facerig_obj
            c_rot.subtarget = f"Eye-Track.{side}"
            c_rot.owner_space = 'LOCAL'
            c_rot.target_space = 'LOCAL'
            c_rot.invert_x = False
            c_rot.invert_y = False
            c_rot.invert_z = False
            c_rot.influence = 0.25

            # 2. Eye Movement (translation): Slides the iris across the eye plane with Eye-Track-Master
            # Locked Y (depth) so eyes stay flat against the eye surface without sinking or popping out
            c_loc = pb_eye.constraints.new('COPY_LOCATION')
            c_loc.name = f"Isaac_EyeLoc_{side}"
            c_loc.target = facerig_obj
            c_loc.subtarget = "Eye-Track-Master"
            c_loc.owner_space = 'LOCAL'
            c_loc.target_space = 'LOCAL'
            c_loc.use_offset = True
            c_loc.use_x = True
            c_loc.use_y = False
            c_loc.use_z = True
            c_loc.invert_x = True
            c_loc.invert_z = True
            c_loc.influence = 0.25

            # 3. Eye Scale: Scale control for iris/eye size
            c_scale = pb_eye.constraints.new('COPY_SCALE')
            c_scale.name = f"Isaac_EyeScale_{side}"
            c_scale.target = facerig_obj
            c_scale.subtarget = f"Eye-Scale-Control.{side}"
            c_scale.owner_space = 'LOCAL'
            c_scale.target_space = 'LOCAL'

            assigned_count += 3

    # Post-fuse Child Of inverse fix (HSR parity): recompute in body-rig space
    # so Eye-Track-Follow rests perfectly horizontal.
    try:
        context.view_layer.update()
        for bname in ["Eye-Track-Follow.L", "Eye-Track-Follow.R"]:
            pb = body_rig.pose.bones.get(bname)
            if pb:
                c = pb.constraints.get("Child Of")
                if c and c.subtarget in body_rig.pose.bones:
                    tgt_pbone = body_rig.pose.bones[c.subtarget]
                    c.inverse_matrix = tgt_pbone.matrix.inverted()
        body_rig["ake_facerig_lowered"] = True
    except Exception as ex_inv:
        print(f"[AKE FACE RIG] Post-fuse inverse fix notice: {ex_inv}")

    # Hide the "Face" bone collection on the body rig as requested
    if hasattr(body_rig.data, "collections"):
        face_coll = body_rig.data.collections.get("Face")
        if face_coll:
            face_coll.is_visible = False
            print("[AKE FACE RIG] Hidden 'Face' bone collection on body rig.")

    print(f"[AKE FACE RIG] Successfully constrained {assigned_count} Endfield face bones to the fused rig!")
    context.view_layer.update()
    return body_rig


class AKE_OT_SetUpIsaacFaceRig(Operator, BasicSetupUIOperator):
    """Sets Up Isaac Face Rig for Arknights: Endfield Character"""
    bl_idname = "arknights_endfield.setup_face_rig"
    bl_label = "Arknights: Endfield: Set Up Isaac Face Rig"

    def execute(self, context):
        from setup_wizard.character_rig_setup.rig_ui_utils import find_target_armature
        armature = find_target_armature(context)
        if not armature:
            self.report({'ERROR'}, 'No character armature found. Please select or rig a character first.')
            return {'CANCELLED'}

        facerig = setup_endfield_isaac_face_rig(armature, context)
        if not facerig:
            self.report({'ERROR'}, 'Failed to set up Isaac Face Rig.')
            return {'CANCELLED'}

        self.report({'INFO'}, 'Successfully set up Isaac Face Rig for Arknights: Endfield character!')
        return {'FINISHED'}


```

## Source file: setup_wizard/character_rig_setup/hsr_face_rig.py

{
  "path": "setup_wizard/character_rig_setup/hsr_face_rig.py",
  "blob_sha": "ed36a534adb13cb91bd907507677df6de06edb62",
  "sha256": "4f03cca8f07a138fcead8131a1d2b2cb34129be6061b830384db7dd4ecd5c82b",
  "lines": 997,
  "bytes": 36555,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 58,
    "jaw": 1,
    "mouth": 39,
    "teeth": 1,
    "tongue": 1,
    "deform": 2,
    "rig": 63,
    "face": 91,
    "lip": 4
  },
  "url": "https://github.com/PaoloESAN/gacha-setup/blob/3e40423dbec489368696c4f89a2bfb285662cdc1/setup_wizard/character_rig_setup/hsr_face_rig.py",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "def is_blender_3():"
    },
    {
      "line": 47,
      "text": "def find_armature_and_head(mesh_obj):"
    },
    {
      "line": 78,
      "text": "def face_frame(mesh_obj):"
    },
    {
      "line": 102,
      "text": "def feature_centroid(mesh_obj, key_names):"
    },
    {
      "line": 138,
      "text": "def pick_primary_key(keyblock, candidates):"
    },
    {
      "line": 145,
      "text": "def plan_hsr_controls(mesh_obj, fwd, right, up, face_size, keyblock):"
    },
    {
      "line": 150,
      "text": "def place(feature, h=0.0, v=0.0):"
    },
    {
      "line": 243,
      "text": "def add_left_panel_grid(items_list, group_name, color, widget_type='slider'):"
    },
    {
      "line": 317,
      "text": "def add_right_panel_grid(items_list, group_name, color, widget_type='slider', is_col2=False):"
    },
    {
      "line": 457,
      "text": "def make_widget(kind, coll):"
    },
    {
      "line": 490,
      "text": "def make_text_widget(text_string, wgt_coll):"
    },
    {
      "line": 540,
      "text": "def lighten(rgb, amt):"
    },
    {
      "line": 544,
      "text": "def apply_color(armature, pb, group_name, rgb, cache):"
    },
    {
      "line": 568,
      "text": "def purge_previous(armature):"
    },
    {
      "line": 583,
      "text": "def get_widget_collection(name=\"WGTS_FaceRig\"):"
    },
    {
      "line": 618,
      "text": "def finalize_widget_collection(wgt_coll):"
    },
    {
      "line": 632,
      "text": "def setup_hsr_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size, keyblock):"
    },
    {
      "line": 638,
      "text": "def to_arm_point(p):"
    },
    {
      "line": 641,
      "text": "def to_arm_vec(v):"
    },
    {
      "line": 884,
      "text": "def setup_dynamic_eye_y_drivers(mesh_obj, armature, max_offset=-0.0040):"
    },
    {
      "line": 925,
      "text": "def attach_driver(pbone, sk_list):"
    },
    {
      "line": 964,
      "text": "def hsr_face_rig_main():"
    }
  ]
}

```text

import bpy
import math
from mathutils import Vector, Matrix

HEAD_BONE_NAME = None
CLEAN_REBUILD = True
FLIP_HORIZONTAL = True
FLIP_VERTICAL = False
FLIP_EYE_LR = False

BONE_LEN_F = 0.020
OFFSET_F = 0.120
TRAVEL_F = 0.050
SPACING_F = 0.045
WIDGET_F = 1.0
FACERIG_COLLECTION = "Face"
RIGIFY_UI_ROW = 1

EYE_HL_F = 0.013
TRI_F = 0.022
EMOTE_W_F = 0.011
EMOTE_H_F = 0.006
EMOTE_LIM_F = 0.022
EMOTE_SPACING_F = 0.040
EMOTE_FWD_F = 0.050
MOUTH_SHIFT_F = 0.55
MOUTH_RAISE_F = 0.040

COL_MOUTH = (0.90, 0.20, 0.20)
COL_CORNER = (0.15, 0.85, 0.30)
COL_VISEME = (0.95, 0.85, 0.20)
COL_EYELID = (0.90, 0.15, 0.15)
COL_EYEAIM = (0.20, 0.85, 0.90)
COL_EXPRESSION = (0.65, 0.35, 0.90)
COL_LABEL = (0.95, 0.95, 0.95)

HEAD_CANDIDATES = [
    "DEF-spine.006", "spine.006", "head", "Head", "Head_M", "head_M",
    "Bip001 Head", "Bip001_Head", "Bip001Head"
]


def is_blender_3():
    return bpy.app.version[0] == 3


def find_armature_and_head(mesh_obj):
    armature = None
    for m in mesh_obj.modifiers:
        if m.type == 'ARMATURE' and m.object:
            armature = m.object
            break
    if armature is None:
        for o in bpy.data.objects:
            if o.type == 'ARMATURE' and o.name in bpy.context.view_layer.objects:
                armature = o
                break
    if armature is None:
        raise Exception("No armature found to attach the HSR face rig to.")

    names = [HEAD_BONE_NAME] if HEAD_BONE_NAME else []
    names += HEAD_CANDIDATES
    head_name = None
    for cand in names:
        if cand and cand in armature.data.bones:
            head_name = cand
            break
    if head_name is None:
        for b in armature.data.bones:
            if 'head' in b.name.lower():
                head_name = b.name
                break
    if head_name is None:
        head_name = armature.data.bones[0].name
    return armature, head_name


def face_frame(mesh_obj):
    me = mesh_obj.data
    mw = mesh_obj.matrix_world
    fwd = Vector((0.0, 0.0, 0.0))
    for v in me.vertices:
        fwd += v.normal
    fwd = mw.to_3x3() @ fwd
    if fwd.length < 1e-6:
        fwd = Vector((0.0, -1.0, 0.0))
    fwd.normalize()

    world_up = Vector((0.0, 0.0, 1.0))
    if abs(fwd.dot(world_up)) > 0.95:
        world_up = Vector((0.0, 1.0, 0.0))
    right = world_up.cross(fwd).normalized()
    up = fwd.cross(right).normalized()

    corners = [mw @ Vector(c) for c in mesh_obj.bound_box]
    mn = Vector((min(c.x for c in corners), min(c.y for c in corners), min(c.z for c in corners)))
    mx = Vector((max(c.x for c in corners), max(c.y for c in corners), max(c.z for c in corners)))
    face_size = (mx - mn).length
    return fwd, right, up, face_size


def feature_centroid(mesh_obj, key_names):
    kb = mesh_obj.data.shape_keys.key_blocks
    basis = kb.get("Basis")
    if basis is None:
        return None
    key = None
    for kn in key_names:
        if kn in kb:
            key = kb[kn]
            break
    if key is None:
        return None

    n = len(basis.data)
    deltas = [0.0] * n
    maxd = 0.0
    for i in range(n):
        d = (key.data[i].co - basis.data[i].co).length
        deltas[i] = d
        if d > maxd:
            maxd = d
    if maxd <= 1e-9:
        return None

    thr = maxd * 0.30
    idx = [i for i in range(n) if deltas[i] >= thr]
    if not idx:
        return None

    c = Vector((0.0, 0.0, 0.0))
    for i in idx:
        c += basis.data[i].co
    c /= len(idx)
    return mesh_obj.matrix_world @ c


def pick_primary_key(keyblock, candidates):
    for c in candidates:
        if c in keyblock:
            return c
    return None


def plan_hsr_controls(mesh_obj, fwd, right, up, face_size, keyblock):
    OFFSET = face_size * OFFSET_F
    LIM = face_size * TRAVEL_F
    SPACING = face_size * SPACING_F

    def place(feature, h=0.0, v=0.0):
        return feature + fwd * OFFSET + right * h + up * v

    controls = []
    handled_keys = set()

    mouth_keys = [k for k in keyblock.keys() if "mouth" in k.lower() or "mth" in k.lower()]
    eye_keys = [k for k in keyblock.keys() if "eye" in k.lower()]
    brow_keys = [k for k in keyblock.keys() if "brow" in k.lower() or "ebr" in k.lower()]

    mouth = feature_centroid(mesh_obj, mouth_keys) or feature_centroid(mesh_obj, ["Mouth_01_A", "Mouth_00_A"])
    eyeC = feature_centroid(mesh_obj, ["00_Close01_Eye", "Eye_Close", "00_Default_Eye"]) or feature_centroid(mesh_obj, eye_keys)
    brow = feature_centroid(mesh_obj, brow_keys)

    fcx = mesh_obj.matrix_world @ (0.125 * sum((Vector(c) for c in mesh_obj.bound_box), Vector()))
    if mouth is None:
        mouth = fcx - up * face_size * 0.12
    if eyeC is None:
        eyeC = fcx + up * face_size * 0.10
    if brow is None:
        brow = eyeC + up * face_size * 0.08

    mouth = mouth + up * (face_size * MOUTH_RAISE_F)
    tri_scale = Vector((face_size * TRI_F,) * 3)

    # 0. Ignore default rest keys (Basis, Default, Rest, N, Max, Minimum)
    for k in keyblock.keys():
        kl = k.lower()
        if "default" in kl or kl in ["basis", "00_default_brow", "00_default_eye", "00_default_mouth", "mouth_00_n", "mouth_01_n", "mouth_00_max", "mouth_01_max", "mouth_00_minimum", "mouth_01_minimum"]:
            handled_keys.add(k)

    # 1. Main Direct Face Controls (Pads/Toggles right on face)
    primary_close = pick_primary_key(keyblock, ["00_Close01_Eye", "Eye_Close", "00_Close02_Eye"])
    if primary_close and primary_close not in handled_keys:
        handled_keys.add(primary_close)
        controls.append({
            'name': 'CTRL-Eye_Close',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYELID,
            'group': 'Face Eyelid',
            'head': place(eyeC, v=face_size * 0.04),
            'widget': 'triangle_down',
            'lim': LIM,
            'free': ('Z',),
            'range': 'neg',
            'shape_scale': tri_scale,
            'drivers': [{'key': primary_close, 'axis': 'Z', 'dir': -1}]
        })

    p_wide = pick_primary_key(keyblock, ["Mouth_01_Wide", "Mouth_00_Wide", "Mouth_Wide"])
    p_narrow = pick_primary_key(keyblock, ["Mouth_01_Narrow", "Mouth_00_Narrow", "Mouth_Narrow"])
    p_up = pick_primary_key(keyblock, ["Mouth_01_Up", "Mouth_00_Up", "Mouth_Up"])
    p_down = pick_primary_key(keyblock, ["Mouth_01_Down", "Mouth_00_Down", "Mouth_Down"])

    drv_shift = []
    if p_wide and p_wide not in handled_keys:
        drv_shift.append({'key': p_wide, 'axis': 'X', 'dir': +1})
        handled_keys.add(p_wide)
    if p_narrow and p_narrow not in handled_keys:
        drv_shift.append({'key': p_narrow, 'axis': 'X', 'dir': -1})
        handled_keys.add(p_narrow)
    if p_up and p_up not in handled_keys:
        drv_shift.append({'key': p_up, 'axis': 'Z', 'dir': +1})
        handled_keys.add(p_up)
    if p_down and p_down not in handled_keys:
        drv_shift.append({'key': p_down, 'axis': 'Z', 'dir': -1})
        handled_keys.add(p_down)

    if False and drv_shift:  # Disabled CTRL-Mouth-Shift bone creation per user request
        mouth_scale = Vector((LIM * 0.9, LIM, LIM * 0.4))
        controls.append({
            'name': 'CTRL-Mouth-Shift',
            'collection': FACERIG_COLLECTION,
            'color': COL_MOUTH,
            'group': 'Face Mouth',
            'head': place(mouth),
            'widget': 'pad',
            'lim': LIM,
            'free': ('X', 'Z'),
            'range': 'both',
            'shape_scale': mouth_scale,
            'drivers': drv_shift
        })

    label_scale = Vector((face_size * 0.045, face_size * 0.045, face_size * 0.045))

    # 2. Left-side Panel: Eyebrows and Eyes (A la IZQUIERDA de la cabeza)
    left_panel_origin = fcx - right * (face_size * 0.45) + up * (face_size * 0.45)
    current_left_v = 0.0
    MAX_PER_ROW = 12  # Máximo 12 controles por fila
    ROW_Z_GAP = 0.120    # Separación vertical en Z entre filas (0.120)
    ITEM_SP = 0.035

    def add_left_panel_grid(items_list, group_name, color, widget_type='slider'):
        nonlocal current_left_v
        if not items_list:
            return
        total = len(items_list)
        for i, item in enumerate(items_list):
            if isinstance(item, tuple):
                c_name, drv_list = item[0], item[1]
            else:
                c_name = item.get('name') if isinstance(item, dict) else f"CTRL-{item}"
                drv_list = item.get('drivers') if isinstance(item, dict) else [{'key': item, 'axis': 'Z', 'dir': +1}]
                if isinstance(item, str):
                    handled_keys.add(item)

            row_idx = i // MAX_PER_ROW
            col_idx = i % MAX_PER_ROW

            # Se extiende hacia la izquierda (-right)
            h = -col_idx * (face_size * ITEM_SP)
            v = current_left_v - row_idx * (face_size * ROW_Z_GAP)

            controls.append({
                'name': c_name,
                'collection': FACERIG_COLLECTION,
                'color': color,
                'group': group_name,
                'head': place(left_panel_origin, h=h, v=v),
                'widget': widget_type,
                'lim': LIM,
                'free': ('Z',),
                'range': 'pos',
                'drivers': drv_list
            })

        num_rows = (total + MAX_PER_ROW - 1) // MAX_PER_ROW
        current_left_v -= num_rows * (face_size * ROW_Z_GAP) + (face_size * 0.01)

    # A. Brow Keys (Cejas - Panel Izquierdo)
    unhandled_brows = [k for k in keyblock.keys() if k in brow_keys and k not in handled_keys]
    if unhandled_brows:
        add_left_panel_grid(unhandled_brows, 'Face Eyebrow', (0.30, 0.80, 0.35))

    # B. Eye Keys (Ojos - Panel Izquierdo)
    unhandled_eyes = [k for k in keyblock.keys() if k in eye_keys and k not in handled_keys]
    if unhandled_eyes:
        add_left_panel_grid(unhandled_eyes, 'Face Eye Expressions', COL_EYEAIM)

    # Add Left Panel Header Label (Brows & Eyes)
    if unhandled_brows or unhandled_eyes:
        total_left = len(unhandled_brows) + len(unhandled_eyes)
        num_cols_left = max(1, min(MAX_PER_ROW, total_left))
        center_h_left = - (num_cols_left - 1) / 2.0 * (face_size * ITEM_SP)
        controls.append({
            'name': 'LABEL-Brows_Eyes',
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(left_panel_origin, h=center_h_left, v=face_size * 0.075),
            'widget': 'text:BROWS & EYES',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    # 3. Right-side Panel: Mouth, Visemes, Phonemes, Extra (A la DERECHA de la cabeza)
    right_panel_origin = fcx + right * (face_size * 0.45) + up * (face_size * 0.45)
    current_right_v_col1 = 0.0
    current_right_v_col2 = 0.0

    col2_offset = face_size * (ITEM_SP * MAX_PER_ROW + 0.05)

    def add_right_panel_grid(items_list, group_name, color, widget_type='slider', is_col2=False):
        nonlocal current_right_v_col1, current_right_v_col2
        if not items_list:
            return
        col_offset = col2_offset if is_col2 else 0.0
        v_base = current_right_v_col2 if is_col2 else current_right_v_col1

        total = len(items_list)
        for i, item in enumerate(items_list):
            if isinstance(item, tuple):
                c_name, drv_list = item[0], item[1]
            else:
                c_name = item.get('name') if isinstance(item, dict) else f"CTRL-{item}"
                drv_list = item.get('drivers') if isinstance(item, dict) else [{'key': item, 'axis': 'Z', 'dir': +1}]
                if isinstance(item, str):
                    handled_keys.add(item)

            row_idx = i // MAX_PER_ROW
            col_idx = i % MAX_PER_ROW

            # Se extiende hacia la derecha (+right) con col_offset
            h = col_idx * (face_size * ITEM_SP) + col_offset
            v = v_base - row_idx * (face_size * ROW_Z_GAP)

            controls.append({
                'name': c_name,
                'collection': FACERIG_COLLECTION,
                'color': color,
                'group': group_name,
                'head': place(right_panel_origin, h=h, v=v),
                'widget': widget_type,
                'lim': LIM,
                'free': ('Z',),
                'range': 'pos',
                'drivers': drv_list
            })

        num_rows = (total + MAX_PER_ROW - 1) // MAX_PER_ROW
        v_drop = num_rows * (face_size * ROW_Z_GAP) + (face_size * 0.01)
        if is_col2:
            current_right_v_col2 -= v_drop
        else:
            current_right_v_col1 -= v_drop

    # A. Visemas (A, E, I, O, U - Panel Derecho, Columna 1)
    viseme_letters = ["A", "E", "I", "O", "U"]
    active_visemes = []
    for letter in viseme_letters:
        pk = pick_primary_key(keyblock, [f"Mouth_01_{letter}", f"Mouth_00_{letter}", f"Mouth_{letter}"])
        if pk and pk not in handled_keys:
            active_visemes.append((f"CTRL-Viseme_{letter}", [{'key': pk, 'axis': 'Z', 'dir': +1}]))
            handled_keys.add(pk)
    if active_visemes:
        add_right_panel_grid(active_visemes, 'Face Visemes', COL_VISEME, is_col2=False)

    # B. Mouth Expression Keys (Expresiones de Boca - Panel Derecho, Columna 1)
    unhandled_mouth = [k for k in keyblock.keys() if k in mouth_keys and k not in handled_keys and "phoneme" not in k.lower()]
    if unhandled_mouth:
        mouth_items = []
        for k in unhandled_mouth:
            clean_k = k.replace("Mouth_00_", "").replace("Mouth_01_", "").replace("Mouth_", "")
            c_name = f"CTRL-Expr_{clean_k}" if clean_k != k else f"CTRL-{k}"
            handled_keys.add(k)
            mouth_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(mouth_items, 'Face Expressions', COL_EXPRESSION, is_col2=False)

    # Add Right Panel Col 1 Header Label (Mouth)
    if active_visemes or unhandled_mouth:
        total_col1 = len(active_visemes) + len(unhandled_mouth)
        num_cols1 = max(1, min(MAX_PER_ROW, total_col1))
        center_h_col1 = (num_cols1 - 1) / 2.0 * (face_size * ITEM_SP)
        controls.append({
            'name': 'LABEL-Mouth',
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(right_panel_origin, h=center_h_col1, v=face_size * 0.075),
            'widget': 'text:MOUTH',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    # C. Phoneme Keys (Fonemas - Panel Derecho, COLUMNA 2 a la Derecha de las expresiones moradas)
    phoneme_keys = [k for k in keyblock.keys() if "phoneme" in k.lower() and k not in handled_keys]
    if phoneme_keys:
        ph_items = []
        for k in phoneme_keys:
            idx_p = k.lower().find("phoneme_cn_")
            sub = k[idx_p + len("phoneme_cn_"):] if idx_p != -1 else k
            c_name = f"CTRL-ph_{sub}"
            handled_keys.add(k)
            ph_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(ph_items, 'Face Phonemes', (0.30, 0.80, 0.35), is_col2=True)

    # D. Extra Keys (Dynamic Fallback - Panel Derecho, COLUMNA 2 a la Derecha)
    remaining = [k for k in keyblock.keys() if k not in handled_keys]
    if remaining:
        rem_items = []
        for k in remaining:
            handled_keys.add(k)
            rem_items.append((f"CTRL-{k}", [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(rem_items, 'Face Extra Keys', (0.25, 0.75, 0.35), is_col2=True)

    # Add Right Panel Col 2 Header Label (Phonemes & Extra)
    if phoneme_keys or remaining:
        total_col2 = len(phoneme_keys) + len(remaining)
        num_cols2 = max(1, min(MAX_PER_ROW, total_col2))
        center_h_col2 = col2_offset + (num_cols2 - 1) / 2.0 * (face_size * ITEM_SP)
        if phoneme_keys and remaining:
            lbl_text = 'PHONEMES & EXTRA'
            lbl_name = 'LABEL-Phonemes_Extra'
        elif phoneme_keys:
            lbl_text = 'PHONEMES'
            lbl_name = 'LABEL-Phonemes'
        else:
            lbl_text = 'EXTRA'
            lbl_name = 'LABEL-Extra'

        controls.append({
            'name': lbl_name,
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(right_panel_origin, h=center_h_col2, v=face_size * 0.075),
            'widget': f'text:{lbl_text}',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    return controls


def make_widget(kind, coll):
    name = "WGT-Face_" + kind
    obj = bpy.data.objects.get(name)
    if obj:
        return obj
    if kind == 'pad':
        s = 1.0
        verts = [(-s, 0, -s), (s, 0, -s), (s, 0, s), (-s, 0, s)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
    elif kind == 'slider':
        verts = [(0, 0, -1), (0, 0, 1), (-0.35, 0, 1), (0.35, 0, 1)]
        edges = [(0, 1), (2, 3)]
    elif kind == 'ring':
        verts, edges, N = [], [], 20
        for i in range(N):
            a = 2 * math.pi * i / N
            verts.append((math.cos(a), 0.0, math.sin(a)))
            edges.append((i, (i + 1) % N))
    elif kind == 'triangle':
        verts = [(0, 0, 1), (-0.9, 0, -0.7), (0.9, 0, -0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    elif kind == 'triangle_down':
        verts = [(0, 0, -1), (-0.9, 0, 0.7), (0.9, 0, 0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    else:
        verts, edges = [(0, 0, 0)], []
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, edges, [])
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return obj


def make_text_widget(text_string, wgt_coll):
    safe_name = text_string.replace(' ', '_').replace('&', 'AND').replace('/', '_')
    name = "WGT-Face_Label_" + safe_name
    obj = bpy.data.objects.get(name)
    if obj:
        return obj

    try:
        curve_data = bpy.data.curves.new(name=name + "_Curve", type='FONT')
        curve_data.body = text_string
        curve_data.size = 1.0
        curve_data.align_x = 'CENTER'
        curve_data.align_y = 'CENTER'
        curve_data.fill_mode = 'NONE'

        temp_obj = bpy.data.objects.new(name + "_Temp", curve_data)
        bpy.context.scene.collection.objects.link(temp_obj)

        depsgraph = bpy.context.evaluated_depsgraph_get()
        eval_obj = temp_obj.evaluated_get(depsgraph)
        mesh_from_eval = bpy.data.meshes.new_from_object(eval_obj)
        mesh_from_eval.name = name

        # Rotate mesh vertices so text stands upright facing the front camera
        # Text X (width) -> Bone -X (which is +X in world)
        # Text Y (height) -> Bone +Z (which is +Z in world)
        # Text Z (normal) -> Bone Y (0 depth)
        for v in mesh_from_eval.vertices:
            v.co = Vector((-v.co.x, 0.0, v.co.y))

        bpy.context.scene.collection.objects.unlink(temp_obj)
        bpy.data.objects.remove(temp_obj, do_unlink=True)
        bpy.data.curves.remove(curve_data, do_unlink=True)

        wgt_obj = bpy.data.objects.new(name, mesh_from_eval)
        wgt_coll.objects.link(wgt_obj)
        return wgt_obj
    except Exception as ex:
        # Fallback wireframe bounding box with center line
        s_w = max(1.0, len(text_string) * 0.4)
        s_h = 0.5
        verts = [(-s_w, 0, -s_h), (s_w, 0, -s_h), (s_w, 0, s_h), (-s_w, 0, s_h)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata(verts, edges, [])
        wgt_obj = bpy.data.objects.new(name, mesh)
        wgt_coll.objects.link(wgt_obj)
        return wgt_obj


def lighten(rgb, amt):
    return tuple(min(1.0, c + amt) for c in rgb)


def apply_color(armature, pb, group_name, rgb, cache):
    if is_blender_3():
        grp = cache.get(group_name)
        if grp is None:
            grp = armature.pose.bone_groups.get(group_name)
            if grp is None:
                grp = armature.pose.bone_groups.new(name=group_name)
            grp.color_set = 'CUSTOM'
            grp.colors.normal = rgb
            grp.colors.select = lighten(rgb, 0.25)
            grp.colors.active = lighten(rgb, 0.5)
            cache[group_name] = grp
        pb.bone_group = grp
    else:
        try:
            pb.color.palette = 'CUSTOM'
            cc = pb.color.custom
            cc.normal = rgb
            cc.select = lighten(rgb, 0.25)
            cc.active = lighten(rgb, 0.5)
        except Exception:
            pass


def purge_previous(armature):
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones
    for b in list(eb):
        if b.name.startswith("CTRL-") or b.name.startswith("LABEL-") or b.name == "Face-Root":
            eb.remove(b)
    bpy.ops.object.mode_set(mode='OBJECT')
    for o in list(bpy.data.objects):
        if o.name.startswith("WGT-Face_"):
            bpy.data.objects.remove(o, do_unlink=True)


def get_widget_collection(name="WGTS_FaceRig"):
    # Prefer per-character WGTS_<Char> (Append-safe): resolve from active/selected rig
    try:
        from setup_wizard.ui.character_settings_utils import resolve_settings_armature, resolve_character_name
        arm = resolve_settings_armature(bpy.context)
        if arm is not None:
            char_name = resolve_character_name(arm)
            if char_name:
                per_char = bpy.data.collections.get(f"WGTS_{char_name}")
                if per_char is not None:
                    return per_char
                # If the rig already lives in a character collection, create nested WGTS there
                try:
                    from setup_wizard.character_rig_setup.wgts_isolation import get_or_create_char_wgts, get_char_collection
                    char_coll = get_char_collection(arm, char_name)
                    if char_coll is not None:
                        return get_or_create_char_wgts(char_coll, char_name)
                except Exception:
                    pass
    except Exception:
        pass
    for name_candidate in ("WGTS", "wgt", "WGTS_FaceRig"):
        coll = bpy.data.collections.get(name_candidate)
        if coll:
            return coll

    for c in bpy.data.collections:
        c_low = c.name.lower()
        if "wgts" in c_low or "wgt" in c_low:
            return c

    coll = bpy.data.collections.new(name)
    return coll


def finalize_widget_collection(wgt_coll):
    if not wgt_coll:
        return
    try:
        wgt_coll.hide_viewport = True
    except Exception:
        pass
    if wgt_coll.name in bpy.context.scene.collection.children:
        try:
            bpy.context.scene.collection.children.unlink(wgt_coll)
        except Exception:
            pass


def setup_hsr_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size, keyblock):
    print(f"Building {len(controls)} HSR face controls on '{armature.name}' under '{head_name}'")

    amw_inv = armature.matrix_world.inverted()
    bone_len = face_size * BONE_LEN_F

    def to_arm_point(p):
        return amw_inv @ p

    def to_arm_vec(v):
        return amw_inv.to_3x3() @ v

    fwd_arm = to_arm_vec(fwd).normalized()
    up_arm = to_arm_vec(up).normalized()

    bpy.context.view_layer.objects.active = armature
    try:
        armature.data.use_mirror_x = False
    except Exception:
        pass

    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones

    root = eb.get("Face-Root") or eb.new("Face-Root")
    head_edit = eb.get(head_name)
    root.head = head_edit.head.copy() if head_edit else Vector((0, 0, 0))
    root.tail = root.head + up_arm * bone_len * 2.0
    root.use_deform = False
    if head_edit:
        root.parent = head_edit

    for c in controls:
        b = eb.get(c['name']) or eb.new(c['name'])
        h = to_arm_point(c['head'])
        b.head = h
        b.tail = h + fwd_arm * bone_len
        b.use_deform = False
        try:
            b.align_roll(up_arm)
        except Exception:
            pass
        b.parent = root
        b.use_connect = False

    bpy.ops.object.mode_set(mode='OBJECT')

    coll_names = [c['collection'] for c in controls if c.get('collection')]
    if not is_blender_3() and hasattr(armature.data, "collections"):
        for cn in set(coll_names):
            coll = armature.data.collections.get(cn) or armature.data.collections.new(cn)
            try:
                coll.is_visible = True
            except Exception:
                pass
            try:
                coll.rigify_ui_row = RIGIFY_UI_ROW
            except Exception:
                pass
        for c in controls:
            bone = armature.data.bones.get(c['name'])
            coll = armature.data.collections.get(c.get('collection', FACERIG_COLLECTION))
            if bone and coll:
                coll.assign(bone)

    wgt_coll = get_widget_collection()
    bpy.ops.object.mode_set(mode='POSE')
    color_cache = {}

    for c in controls:
        pb = armature.pose.bones.get(c['name'])
        if not pb:
            continue

        is_lbl = c.get('is_label', False)

        if is_lbl:
            # Lock all transforms for header label bones
            for i in range(3):
                pb.lock_location[i] = True
                pb.lock_rotation[i] = True
                pb.lock_scale[i] = True
            pb.lock_rotation_w = True
        else:
            lim = c['lim']
            free = c['free']
            rng = c['range']
            if rng == 'both':
                lo, hi = -lim, lim
            elif rng == 'neg':
                lo, hi = -lim, 0.0
            else:
                lo, hi = 0.0, lim

            pb.lock_location[0] = 'X' not in free
            pb.lock_location[1] = True
            pb.lock_location[2] = 'Z' not in free
            for i in range(3):
                pb.lock_rotation[i] = True
                pb.lock_scale[i] = True
            pb.lock_rotation_w = True

            con = pb.constraints.new(type='LIMIT_LOCATION')
            con.owner_space = 'LOCAL'
            con.use_transform_limit = True
            con.use_min_x = con.use_max_x = True
            con.use_min_y = con.use_max_y = True
            con.use_min_z = con.use_max_z = True
            con.min_y = con.max_y = 0.0
            con.min_x = lo if 'X' in free else 0.0
            con.max_x = hi if 'X' in free else 0.0
            con.min_z = lo if 'Z' in free else 0.0
            con.max_z = hi if 'Z' in free else 0.0

        if c['widget'].startswith('text:'):
            text_str = c['widget'].split(':', 1)[1]
            pb.custom_shape = make_text_widget(text_str, wgt_coll)
        else:
            pb.custom_shape = make_widget(c['widget'], wgt_coll)

        try:
            pb.use_custom_shape_bone_size = False
        except Exception:
            pass
        ss = c.get('shape_scale')
        pb.custom_shape_scale_xyz = ss if ss is not None else Vector((c.get('lim', 0.02) * WIDGET_F,) * 3)

        apply_color(armature, pb, c['group'], c['color'], color_cache)

    bpy.ops.object.mode_set(mode='OBJECT')

    # Build Drivers on Shape Keys
    agg = {}
    for c in controls:
        for d in c.get('drivers', []):
            agg.setdefault(d['key'], []).append(
                {'bone': c['name'], 'axis': d['axis'], 'dir': d['dir'],
                 'lim': c['lim'], 'bidir': d.get('bidir', False)})

    for key, entries in agg.items():
        sk = keyblock.get(key)
        if sk is None:
            continue
        sk.slider_min = -1.0 if any(e['bidir'] for e in entries) else 0.0
        try:
            sk.driver_remove("value")
        except Exception:
            pass
        drv = sk.driver_add("value").driver
        drv.type = 'SCRIPTED'
        terms = []
        for i, e in enumerate(entries):
            vn = f"v{i}"
            var = drv.variables.new()
            var.name = vn
            var.type = 'TRANSFORMS'
            tgt = var.targets[0]
            tgt.id = armature
            tgt.bone_target = e['bone']
            tgt.transform_type = 'LOC_' + e['axis']
            tgt.transform_space = 'LOCAL_SPACE'
            sign = '' if e['dir'] > 0 else '-'
            if e['bidir']:
                terms.append(f"{sign}{vn} / {e['lim']!r}")
            else:
                terms.append(f"max(0.0, {sign}{vn} / {e['lim']!r})")
        drv.expression = terms[0] if len(terms) == 1 else "max(" + ", ".join(terms) + ")"

    # Hide stick facial bones, extra bones, tweak collections, FK collections, Fingers (Detail), and Face-Root
    hidden_colls = [
        "DEF", "ORG", "MCH", "Face", "Face (Secondary)", "Face (Primary)", "Face (Tweaks)", "Face Bones", "Face_Bones",
        "Extra Bones", "Extra_Bones", "Extra",
        "Torso (Tweak)", "Arm.L (Tweak)", "Arm.R (Tweak)", "Leg.L (Tweak)", "Leg.R (Tweak)",
        "Arm.L (FK)", "Arm.R (FK)", "Leg.L (FK)", "Leg.R (FK)",
        "Fingers (Detail)"
    ]
    for cn in hidden_colls:
        c = armature.data.collections.get(cn)
        if c:
            try:
                c.is_visible = False
            except Exception:
                pass

    palms_coll = armature.data.collections.get("Palms")
    if palms_coll:
        palms_coll.is_visible = True

    # Create visible "Face" collection for eye/face control bones
    facerig_coll = armature.data.collections.get("Face") or armature.data.collections.get("Facerig") or armature.data.collections.new("Face")
    facerig_coll.is_visible = True

    eye_ctrl_bone_names = ["Eye-Track-Master", "Eye-Track.L", "Eye-Track.R"]
    for bname in eye_ctrl_bone_names:
        if bname in armature.data.bones:
            b = armature.data.bones[bname]
            try:
                facerig_coll.assign(b)
                b.hide = False
            except Exception:
                pass

    # Hide helper tracking bones (+EyeBone L/R A01.001)
    for bname in ["+EyeBone L A01.001", "+EyeBone R A01.001"]:
        if bname in armature.data.bones:
            try:
                armature.data.bones[bname].hide = True
            except Exception:
                pass

    face_bones_coll = armature.data.collections.get("Face Bones") or armature.data.collections.new("Face Bones")
    face_bones_coll.is_visible = False

    extra_bones_coll = armature.data.collections.get("Extra Bones") or armature.data.collections.new("Extra Bones")
    extra_bones_coll.is_visible = False

    face_keywords = [
        "joint_", "brow", "eye", "eyelid", "cheek", "nose", 
        "mouth", "lip", "jaw", "teeth", "tongue", "skn", "face-root"
    ]
    main_ctrls = ["root", "torso", "hips", "chest", "neck", "head"]

    for bone in armature.data.bones:
        b_name = bone.name
        b_low = b_name.lower()
        if b_name.startswith("CTRL-") or b_name.startswith("LABEL-") or b_name in main_ctrls or b_name in eye_ctrl_bone_names:
            continue

        is_face_bone = any(kw in b_low for kw in face_keywords)
        target_coll = face_bones_coll if is_face_bone else extra_bones_coll

        if is_face_bone or b_low.startswith("def-") or b_low.startswith("org-") or b_low.startswith("mch-") or len(bone.collections) == 0:
            try:
                target_coll.assign(bone)
            except Exception:
                pass
            for c in list(bone.collections):
                if c != target_coll and c != facerig_coll and c.name not in hidden_colls:
                    try:
                        c.unassign(bone)
                    except Exception:
                        pass
            bone.hide = True

    # Set up dynamic Y location drivers for eye bones driven by eye shape key activation (up to -0.0040 max offset)
    setup_dynamic_eye_y_drivers(mesh_obj, armature, max_offset=-0.0040)

    print("HSR Face rig build complete.")


def setup_dynamic_eye_y_drivers(mesh_obj, armature, max_offset=-0.0040):
    """
    Drives eye.L and eye.R Y location backwards up to max_offset (-0.0040)
    proportionally when eye expression/close shape keys are activated by the facial rig.
    Replaces static fixed eye location offset hack.
    """
    candidate_eye_L = ["eye.L", "DEF-eye.L", "+EyeBone L A01", "+EyeBone L A02", "EYE_L", "Eye_L"]
    candidate_eye_R = ["eye.R", "DEF-eye.R", "+EyeBone R A01", "+EyeBone R A02", "EYE_R", "Eye_R"]

    eye_L_pbone = next((armature.pose.bones.get(b) for b in candidate_eye_L if armature.pose.bones.get(b)), None)
    eye_R_pbone = next((armature.pose.bones.get(b) for b in candidate_eye_R if armature.pose.bones.get(b)), None)

    # Reset any static pose location offsets
    if eye_L_pbone:
        eye_L_pbone.location = (0.0, 0.0, 0.0)
    if eye_R_pbone:
        eye_R_pbone.location = (0.0, 0.0, 0.0)

    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return

    keyblock = mesh_obj.data.shape_keys.key_blocks
    sk_id = mesh_obj.data.shape_keys

    eye_sk_L = []
    eye_sk_R = []

    for k_name in keyblock.keys():
        kl = k_name.lower()
        if "default" in kl or kl == "basis":
            continue
        # Only select eye shape keys (excluding brow/mouth)
        if any(kw in kl for kw in ["eye", "wink"]) and not any(ex in kl for ex in ["brow", "mouth"]):
            is_r_only = kl.endswith("_r") or kl.endswith(".r") or kl.endswith("right") or "_r_" in kl
            is_l_only = kl.endswith("_l") or kl.endswith(".l") or kl.endswith("left") or "_l_" in kl

            if not is_r_only:
                eye_sk_L.append(k_name)
            if not is_l_only:
                eye_sk_R.append(k_name)

    def attach_driver(pbone, sk_list):
        if not pbone or not sk_list:
            return
        try:
            pbone.driver_remove("location", 1)
        except Exception:
            pass

        driver_obj = pbone.driver_add("location", 1)
        drv = driver_obj.driver
        drv.type = 'SCRIPTED'

        var_names = []
        for i, sk_name in enumerate(sk_list):
            vn = f"v{i}"
            var = drv.variables.new()
            var.name = vn
            var.type = 'SINGLE_PROP'
            tgt = var.targets[0]
            try:
                tgt.id_type = 'KEY'
            except Exception:
                pass
            tgt.id = sk_id
            tgt.data_path = f'key_blocks["{sk_name}"].value'
            var_names.append(vn)

        if len(var_names) == 1:
            drv.expression = f"{max_offset!r} * max(0.0, min(1.0, {var_names[0]}))"
        else:
            terms_str = ", ".join(var_names)
            drv.expression = f"{max_offset!r} * max(0.0, min(1.0, max({terms_str})))"

    if eye_L_pbone:
        attach_driver(eye_L_pbone, eye_sk_L)
    if eye_R_pbone:
        attach_driver(eye_R_pbone, eye_sk_R)


def hsr_face_rig_main():
    faceobj = None
    for obj in bpy.data.objects:
        if obj.type == 'MESH' and obj.name in bpy.context.view_layer.objects:
            n = obj.name.lower()
            if "face" in n or (obj.data.shape_keys and any(k.startswith("Mouth_") or k.startswith("Brow_") or k.startswith("Eye_") for k in obj.data.shape_keys.key_blocks.keys())):
                faceobj = obj
                break

    if faceobj is None:
        print("HSR Face Rig: No face mesh found.")
        return
    if faceobj.data.shape_keys is None:
        print("HSR Face Rig: Face mesh has no shape keys.")
        return

    keyblock = faceobj.data.shape_keys.key_blocks
    armature, head_name = find_armature_and_head(faceobj)
    fwd, right, up, face_size = face_frame(faceobj)
    controls = plan_hsr_controls(faceobj, fwd, right, up, face_size, keyblock)

    if not controls:
        print("HSR Face Rig: No drivable shape keys found.")
        return

    if CLEAN_REBUILD:
        purge_previous(armature)

    setup_hsr_face_rig(faceobj, controls, armature, head_name, fwd, up, face_size, keyblock)

    wgt_coll = get_widget_collection()
    finalize_widget_collection(wgt_coll)

    print(f"\nHSR Face Rig complete: {len(controls)} controls built on '{armature.name}'.\n")


```

## Source file: setup_wizard/character_rig_setup/nte_face_rig.py

{
  "path": "setup_wizard/character_rig_setup/nte_face_rig.py",
  "blob_sha": "853945597fc42346151655edce577415cd192d62",
  "sha256": "8dd03d457d633fb01d8f1c48e87d324dd90fb349fd398de2e335edcf3e0f8b3b",
  "lines": 1376,
  "bytes": 59573,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 90,
    "skin": 1,
    "jaw": 20,
    "mouth": 34,
    "deform": 2,
    "shader": 1,
    "rig": 96,
    "face": 117,
    "lip": 13
  },
  "url": "https://github.com/PaoloESAN/gacha-setup/blob/3e40423dbec489368696c4f89a2bfb285662cdc1/setup_wizard/character_rig_setup/nte_face_rig.py",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "def is_blender_3():"
    },
    {
      "line": 48,
      "text": "def find_face_mesh():"
    },
    {
      "line": 74,
      "text": "def find_armature_and_head(mesh_obj):"
    },
    {
      "line": 111,
      "text": "def get_face_metrics(mesh_obj, armature, head_name):"
    },
    {
      "line": 146,
      "text": "def feature_centroid(mesh_obj, key_names, side=None):"
    },
    {
      "line": 198,
      "text": "def get_shapekey_side(mesh_obj, sk_name, fcx, right):"
    },
    {
      "line": 220,
      "text": "def pick_primary_key(keyblock, candidates):"
    },
    {
      "line": 227,
      "text": "def plan_nte_controls(mesh_obj, armature, head_name, keyblock):"
    },
    {
      "line": 233,
      "text": "def place(feature, h=0.0, v=0.0):"
    },
    {
      "line": 586,
      "text": "def add_left_panel_grid(items_list, group_name, color, widget_type='slider'):"
    },
    {
      "line": 670,
      "text": "def add_right_panel_grid(items_list, group_name, color, widget_type='slider', is_col2=False):"
    },
    {
      "line": 816,
      "text": "def make_widget(kind, coll):"
    },
    {
      "line": 868,
      "text": "def make_text_widget(text_string, wgt_coll):"
    },
    {
      "line": 918,
      "text": "def get_widget_collection():"
    },
    {
      "line": 952,
      "text": "def finalize_widget_collection(wgt_coll):"
    },
    {
      "line": 966,
      "text": "def lighten(rgb, amt):"
    },
    {
      "line": 970,
      "text": "def apply_color(armature, pb, group_name, rgb, cache):"
    },
    {
      "line": 994,
      "text": "def purge_previous(armature):"
    },
    {
      "line": 1013,
      "text": "def setup_nte_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size, keyblock):"
    },
    {
      "line": 1019,
      "text": "def to_arm_point(p):"
    },
    {
      "line": 1022,
      "text": "def to_arm_vec(v):"
    },
    {
      "line": 1246,
      "text": "def setup_common_face_materials_if_present():"
    },
    {
      "line": 1263,
      "text": "def cleanup_head_driver_to_wgts_if_present(armature=None, head_name=None):"
    },
    {
      "line": 1313,
      "text": "def get_all_children(obj):"
    },
    {
      "line": 1343,
      "text": "def nte_face_rig_main():"
    }
  ]
}

```text

# Author: PaoloESAN
# Face Rig Logic for Neverness to Everness (NTE)
# Integrates Isaac/ZZZ direct face widgets and controls with HSR-style side panel expression sliders

import bpy
import math
from mathutils import Vector, Matrix

HEAD_BONE_NAME   = None
CLEAN_REBUILD    = True
FLIP_HORIZONTAL  = True
FLIP_VERTICAL    = False
FLIP_EYE_LR      = False

BONE_LEN_F   = 0.020
OFFSET_F     = 0.080
TRAVEL_F     = 0.040
SPACING_F    = 0.050
WIDGET_F     = 1.0
FACERIG_COLLECTION = "Face"
RIGIFY_UI_ROW = 1

COL_MOUTH      = (0.90, 0.20, 0.20)
COL_CORNER     = (0.15, 0.85, 0.30)
COL_VISEME     = (0.95, 0.85, 0.20)
COL_EYELID     = (0.90, 0.15, 0.15)
COL_EYEAIM     = (0.20, 0.85, 0.90)
COL_EYEGREEN   = (0.20, 0.85, 0.30)
COL_EYEMOTE    = (0.90, 0.20, 0.20)
COL_BROW       = (0.30, 0.80, 0.35)
COL_BROWSAD    = (0.95, 0.45, 0.75)
COL_EBRBONE    = (0.20, 0.55, 0.95)
COL_EBRMASTER  = (0.65, 0.35, 0.90)
COL_EXPRESSION = (0.65, 0.35, 0.90)
COL_TD_SPECIAL = (0.90, 0.40, 0.60)
COL_LABEL      = (0.95, 0.95, 0.95)

HEAD_CANDIDATES = [
    "DEF-spine.006", "ORG-spine.006", "spine.006", "head", "Head", "Head_M", "head_M",
    "Bip001-Head", "Bip001 Head", "Bip001_Head", "Bip001Head", "Bip001 头", "頭", "头",
]


def is_blender_3():
    return bpy.app.version[0] == 3


def find_face_mesh():
    nte_signatures = ["look_U", "EL_", "EB_", "jawOpen", "biyan", "TD_", "TDS_", "mouthPucker"]
    for obj in bpy.data.objects:
        if obj.type == 'MESH' and obj.data and obj.data.shape_keys:
            kb = obj.data.shape_keys.key_blocks
            if any(any(sig in k for sig in nte_signatures) for k in kb.keys()):
                return obj

    for obj in bpy.data.objects:
        if obj.type == 'MESH' and obj.data and obj.data.shape_keys:
            n = obj.name.lower()
            if any(k in n for k in ["face", "head", "skin", "player_", "npc_"]):
                if not any(ign in n for ign in ["weapon", "gun", "sword"]):
                    return obj

    for obj in bpy.context.view_layer.objects:
        if obj.type == 'MESH' and obj.data and obj.data.shape_keys:
            return obj

    for obj in bpy.data.objects:
        if obj.type == 'MESH' and obj.data and obj.data.shape_keys:
            return obj

    return None


def find_armature_and_head(mesh_obj):
    armature = None
    for m in mesh_obj.modifiers:
        if m.type == 'ARMATURE' and m.object:
            armature = m.object
            break
    if armature is None:
        for o in bpy.data.objects:
            if o.type == 'ARMATURE' and o.name in bpy.context.view_layer.objects:
                if not any(ign in o.name.lower() for ign in ['metarig', 'backup']):
                    armature = o
                    break
    if armature is None:
        for o in bpy.data.objects:
            if o.type == 'ARMATURE':
                armature = o
                break
    if armature is None:
        raise Exception("No armature found to attach the NTE face rig to.")

    names = [HEAD_BONE_NAME] if HEAD_BONE_NAME else []
    names += HEAD_CANDIDATES
    head_name = None
    for cand in names:
        if cand and cand in armature.data.bones:
            head_name = cand
            break
    if head_name is None:
        for b in armature.data.bones:
            if 'head' in b.name.lower() or 'spine.006' in b.name.lower():
                head_name = b.name
                break
    if head_name is None:
        head_name = armature.data.bones[0].name
    return armature, head_name


def get_face_metrics(mesh_obj, armature, head_name):
    head_pos = None
    head_tail = None
    head_bone = None

    if armature and head_name and hasattr(armature, 'data') and hasattr(armature.data, 'bones'):
        head_bone = armature.data.bones.get(head_name)
        if head_bone:
            head_pos = armature.matrix_world @ head_bone.head_local
            head_tail = armature.matrix_world @ head_bone.tail_local

    if head_pos and head_tail:
        head_len = (head_tail - head_pos).length
        face_size = max(0.18, min(0.28, head_len * 1.15 if head_len > 0.05 else 0.22))
        fwd_guess = Vector((0.0, -1.0, 0.0))
        up_guess = (head_tail - head_pos).normalized() if head_len > 1e-4 else Vector((0.0, 0.0, 1.0))
        fcx = head_pos + up_guess * (head_len * 0.55) + fwd_guess * (face_size * 0.25)
    else:
        face_size = 0.22
        fcx = Vector((0.0, -0.05, 1.45))

    fwd = Vector((0.0, -1.0, 0.0))
    world_up = Vector((0.0, 0.0, 1.0))

    if head_bone and head_pos and head_tail:
        bone_vec = (head_tail - head_pos).normalized() if (head_tail - head_pos).length > 1e-4 else world_up
        if abs(bone_vec.dot(world_up)) > 0.5:
            world_up = bone_vec

    right = world_up.cross(fwd).normalized()
    up = fwd.cross(right).normalized()

    return fwd, right, up, face_size, fcx


def feature_centroid(mesh_obj, key_names, side=None):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return None
    kb = mesh_obj.data.shape_keys.key_blocks
    basis = kb.get("Basis")
    if basis is None or not hasattr(basis, 'data'):
        return None
    key = None
    for kn in key_names:
        if kn in kb:
            key = kb[kn]
            break
    if key is None or not hasattr(key, 'data'):
        return None

    n = len(basis.data)
    deltas = [0.0] * n
    maxd = 0.0
    for i in range(n):
        d = (key.data[i].co - basis.data[i].co).length
        deltas[i] = d
        if d > maxd:
            maxd = d
    if maxd <= 1e-9:
        return None

    thr = maxd * 0.30
    idx = [i for i in range(n) if deltas[i] >= thr]
    if not idx:
        return None

    is_already_single_sided = False
    if key.name:
        kn_low = key.name.lower()
        if "_l" in kn_low or ".l" in kn_low or "left" in kn_low or "_r" in kn_low or ".r" in kn_low or "right" in kn_low:
            is_already_single_sided = True

    if side in ('L', 'R') and not is_already_single_sided:
        xs = sorted(basis.data[i].co.x for i in idx)
        median_x = xs[len(xs) // 2]
        hi = [i for i in idx if basis.data[i].co.x >= median_x]
        lo = [i for i in idx if basis.data[i].co.x < median_x]
        pick = hi if (side == 'L') != FLIP_EYE_LR else lo
        idx = pick or idx

    c = Vector((0.0, 0.0, 0.0))
    for i in idx:
        c += basis.data[i].co
    c /= len(idx)
    return mesh_obj.matrix_world @ c


def get_shapekey_side(mesh_obj, sk_name, fcx, right):
    """
    Returns +1 if the shapekey's vertices are on the +right side of the face (Viewer's Right / +X / Blender .L),
    -1 if on the -right side (Viewer's Left / -X / Blender .R), or 0 if centered.
    """
    if not mesh_obj or not sk_name:
        return 0
    c = feature_centroid(mesh_obj, [sk_name])
    if c is not None:
        dot = (c - fcx).dot(right)
        if dot > 0.005:
            return +1
        elif dot < -0.005:
            return -1
    name_low = sk_name.lower()
    if "_r" in name_low or ".r" in name_low or "right" in name_low:
        return +1
    elif "_l" in name_low or ".l" in name_low or "left" in name_low:
        return -1
    return 0


def pick_primary_key(keyblock, candidates):
    for c in candidates:
        if c in keyblock:
            return c
    return None


def plan_nte_controls(mesh_obj, armature, head_name, keyblock):
    fwd, right, up, face_size, fcx = get_face_metrics(mesh_obj, armature, head_name)

    OFFSET  = face_size * OFFSET_F
    LIM     = face_size * TRAVEL_F

    def place(feature, h=0.0, v=0.0):
        return feature + fwd * OFFSET + right * h + up * v

    controls = []
    handled_keys = set()

    # 0. Ignore default/basis rest keys
    for k in keyblock.keys():
        kl = k.lower()
        if kl in ["basis", "default", "rest", "00_default_brow", "00_default_eye", "00_default_mouth"]:
            handled_keys.add(k)

    # -------------------------------------------------------------------------
    # Physical Side Classification:
    # +right (+X): Viewer's RIGHT / Character's LEFT (White hair side / Blender .L)
    # -right (-X): Viewer's LEFT / Character's RIGHT (Black hair side / Blender .R)
    # -------------------------------------------------------------------------

    # 1. Classify Wink shapekeys
    wink_keys_posX = []
    wink_keys_negX = []
    for k in ["biyan_R", "biyan_r", "biyan_L", "biyan_l", "biyan", "biyan1"]:
        if k in keyblock:
            s = get_shapekey_side(mesh_obj, k, fcx, right)
            if s == +1 and k not in wink_keys_posX:
                wink_keys_posX.append(k)
            elif s == -1 and k not in wink_keys_negX:
                wink_keys_negX.append(k)

    # 2. Classify Happy / Smile close shapekeys
    happy_keys_posX = []
    happy_keys_negX = []
    for k in ["EL_Happy_r_CLO", "EL_Happy_R_CLO", "EL_Happy_l_CLO", "EL_Happy_L_CLO", "EL_Smile_01_CLO", "biyan1"]:
        if k in keyblock:
            s = get_shapekey_side(mesh_obj, k, fcx, right)
            if s == +1 and k not in happy_keys_posX:
                happy_keys_posX.append(k)
            elif s == -1 and k not in happy_keys_negX:
                happy_keys_negX.append(k)

    # 3. Classify Eyebrow Up/Down shapekeys
    brow_keys_posX = []
    brow_keys_negX = []
    for k in ["EB_UD_R", "EB_UD_r", "EB_R_UD", "EB_r_UD", "EB_UD_L", "EB_UD_l", "EB_L_UD", "EB_l_UD"]:
        if k in keyblock:
            s = get_shapekey_side(mesh_obj, k, fcx, right)
            if s == +1 and k not in brow_keys_posX:
                brow_keys_posX.append(k)
            elif s == -1 and k not in brow_keys_negX:
                brow_keys_negX.append(k)

    # 4. Compute physical positions
    pos_eye_posX = feature_centroid(mesh_obj, wink_keys_posX + happy_keys_posX) or (fcx + up * (face_size * 0.06) + right * (face_size * 0.16) + fwd * (face_size * 0.04))
    pos_eye_negX = feature_centroid(mesh_obj, wink_keys_negX + happy_keys_negX) or (fcx + up * (face_size * 0.06) - right * (face_size * 0.16) + fwd * (face_size * 0.04))

    pos_brow_posX = feature_centroid(mesh_obj, brow_keys_posX) or (pos_eye_posX + up * (face_size * 0.14))
    pos_brow_negX = feature_centroid(mesh_obj, brow_keys_negX) or (pos_eye_negX + up * (face_size * 0.14))

    eyeC = (pos_eye_posX + pos_eye_negX) * 0.5
    browCenter = (pos_brow_posX + pos_brow_negX) * 0.5

    # 5. Mouth Centroid
    mouth_keys = [k for k in keyblock.keys() if "jawopen" in k.lower() or "mouth" in k.lower()]
    mouth = feature_centroid(mesh_obj, ["jawOpen", "jawOpen_a", "jawOpen_Happy_01_OP", "mouthPucker", "mouthRollLower"] + mouth_keys)
    if mouth is None:
        mouth = fcx - up * (face_size * 0.24) + fwd * (face_size * 0.04)

    tri_scale = Vector((face_size * 0.038,) * 3)

    # =========================================================================
    # 1. DIRECT FACIAL CONTROLS
    # =========================================================================

    # 1.A Eye Aim / Look Directions (8 directions)
    look_u = pick_primary_key(keyblock, ["look_U", "Look_U", "Eye_Up"])
    look_d = pick_primary_key(keyblock, ["look_D", "Look_D", "Eye_Down"])
    look_l = pick_primary_key(keyblock, ["look_L", "Look_L", "Eye_Left"])
    look_r = pick_primary_key(keyblock, ["look_R", "Look_R", "Eye_Right"])
    look_lu = pick_primary_key(keyblock, ["look_LU", "Look_LU"])
    look_ld = pick_primary_key(keyblock, ["look_LD", "Look_LD"])
    look_ru = pick_primary_key(keyblock, ["look_RU", "Look_RU"])
    look_rd = pick_primary_key(keyblock, ["look_RD", "Look_RD"])

    aim_drv = []
    right_x_dir = +1 if FLIP_HORIZONTAL else -1
    left_x_dir = -1 if FLIP_HORIZONTAL else +1

    if look_u:
        aim_drv.append({'key': look_u, 'axis': 'Z', 'dir': +1})
        handled_keys.add(look_u)
    if look_d:
        aim_drv.append({'key': look_d, 'axis': 'Z', 'dir': -1})
        handled_keys.add(look_d)
    if look_r:
        aim_drv.append({'key': look_r, 'axis': 'X', 'dir': right_x_dir})
        handled_keys.add(look_r)
    if look_l:
        aim_drv.append({'key': look_l, 'axis': 'X', 'dir': left_x_dir})
        handled_keys.add(look_l)

    if look_ru:
        aim_drv.append({'key': look_ru, 'axis': 'Z', 'dir': +1, 'extra_axis': 'X', 'extra_dir': right_x_dir, 'is_diagonal': True})
        handled_keys.add(look_ru)
    if look_rd:
        aim_drv.append({'key': look_rd, 'axis': 'Z', 'dir': -1, 'extra_axis': 'X', 'extra_dir': right_x_dir, 'is_diagonal': True})
        handled_keys.add(look_rd)
    if look_lu:
        aim_drv.append({'key': look_lu, 'axis': 'Z', 'dir': +1, 'extra_axis': 'X', 'extra_dir': left_x_dir, 'is_diagonal': True})
        handled_keys.add(look_lu)
    if look_ld:
        aim_drv.append({'key': look_ld, 'axis': 'Z', 'dir': -1, 'extra_axis': 'X', 'extra_dir': left_x_dir, 'is_diagonal': True})
        handled_keys.add(look_ld)

    if aim_drv:
        controls.append({
            'name': 'CTRL-Eye-Aim',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYEAIM,
            'group': 'Face Eye-Aim',
            'head': place(eyeC, v=face_size * 0.015),
            'widget': 'ring',
            'lim': LIM,
            'free': ('X', 'Z'),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.050,) * 3),
            'drivers': aim_drv
        })

    # 1.B Eye Open (.L at +X / Viewer Right, .R at -X / Viewer Left)
    open_cand = pick_primary_key(keyblock, ["EL_Surprised_01_OP", "EL_Surprised_01", "EL_Up"])
    if open_cand:
        controls.append({
            'name': 'CTRL-Eye_Open.L',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYELID,
            'group': 'Face Eyelid',
            'head': place(pos_eye_posX, v=face_size * 0.045),
            'widget': 'isaac_blink_top',
            'lim': LIM,
            'free': ('Z',),
            'range': 'pos',
            'shape_scale': tri_scale,
            'drivers': [{'key': open_cand, 'axis': 'Z', 'dir': +1}]
        })
        controls.append({
            'name': 'CTRL-Eye_Open.R',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYELID,
            'group': 'Face Eyelid',
            'head': place(pos_eye_negX, v=face_size * 0.045),
            'widget': 'isaac_blink_top',
            'lim': LIM,
            'free': ('Z',),
            'range': 'pos',
            'shape_scale': tri_scale,
            'drivers': [{'key': open_cand, 'axis': 'Z', 'dir': +1}]
        })

    # 1.C Eye Wink & Smile Close (.L at +X / Viewer Right, .R at -X / Viewer Left)
    wink_k_posX = wink_keys_posX[0] if wink_keys_posX else None
    happy_k_posX = happy_keys_posX[0] if happy_keys_posX else None
    wdrv_posX = []
    if wink_k_posX:
        wdrv_posX.append({'key': wink_k_posX, 'axis': 'Z', 'dir': -1})
        handled_keys.add(wink_k_posX)
    if happy_k_posX and happy_k_posX != wink_k_posX:
        wdrv_posX.append({'key': happy_k_posX, 'axis': 'Z', 'dir': +1})
        handled_keys.add(happy_k_posX)

    if wdrv_posX:
        controls.append({
            'name': 'CTRL-Eye_Wink.L',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYELID,
            'group': 'Face Eyelid',
            'head': place(pos_eye_posX, v=-face_size * 0.045),
            'widget': 'isaac_blink_bot',
            'lim': LIM,
            'free': ('Z',),
            'range': 'both',
            'shape_scale': tri_scale,
            'drivers': wdrv_posX
        })

    wink_k_negX = wink_keys_negX[0] if wink_keys_negX else None
    happy_k_negX = happy_keys_negX[0] if happy_keys_negX else None
    wdrv_negX = []
    if wink_k_negX:
        wdrv_negX.append({'key': wink_k_negX, 'axis': 'Z', 'dir': -1})
        handled_keys.add(wink_k_negX)
    if happy_k_negX and happy_k_negX != wink_k_negX:
        wdrv_negX.append({'key': happy_k_negX, 'axis': 'Z', 'dir': +1})
        handled_keys.add(happy_k_negX)

    if wdrv_negX:
        controls.append({
            'name': 'CTRL-Eye_Wink.R',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYELID,
            'group': 'Face Eyelid',
            'head': place(pos_eye_negX, v=-face_size * 0.045),
            'widget': 'isaac_blink_bot',
            'lim': LIM,
            'free': ('Z',),
            'range': 'both',
            'shape_scale': tri_scale,
            'drivers': wdrv_negX
        })

    # 1.D Center Eye Viseme / Emote Pad
    eye_pad_drv = []
    biyan_all = pick_primary_key(keyblock, ["biyan", "TD_EyesClo", "biyan1"])
    eye_down = pick_primary_key(keyblock, ["EL_Down", "eye_SF"])
    corner_up = pick_primary_key(keyblock, ["EL_CornerUp", "EL_Light_Up"])
    corner_down = pick_primary_key(keyblock, ["EL_CornerDown", "EL_Light_L"])

    if biyan_all and biyan_all not in handled_keys:
        eye_pad_drv.append({'key': biyan_all, 'axis': 'Z', 'dir': +1})
        handled_keys.add(biyan_all)
    if eye_down and eye_down not in handled_keys:
        eye_pad_drv.append({'key': eye_down, 'axis': 'Z', 'dir': -1})
        handled_keys.add(eye_down)
    if corner_up and corner_up not in handled_keys:
        eye_pad_drv.append({'key': corner_up, 'axis': 'X', 'dir': +1})
        handled_keys.add(corner_up)
    if corner_down and corner_down not in handled_keys:
        eye_pad_drv.append({'key': corner_down, 'axis': 'X', 'dir': -1})
        handled_keys.add(corner_down)

    if eye_pad_drv:
        controls.append({
            'name': 'CTRL-Eye-Viseme-Pad',
            'collection': FACERIG_COLLECTION,
            'color': COL_EYEMOTE,
            'group': 'Face Eye-Emote',
            'head': place(eyeC),
            'widget': 'eyeblink',
            'lim': LIM,
            'free': ('X', 'Z'),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.042, face_size * 0.048, face_size * 0.032)),
            'drivers': eye_pad_drv
        })

    # 1.E Eyebrows Direct Controls (.L at +X / Viewer Right, .R at -X / Viewer Left)
    eb_k_posX = brow_keys_posX[0] if brow_keys_posX else None
    if eb_k_posX:
        handled_keys.add(eb_k_posX)
        controls.append({
            'name': 'CTRL-Brow-L_Up',
            'collection': FACERIG_COLLECTION,
            'color': COL_BROW,
            'group': 'Face Eyebrow',
            'head': place(pos_brow_posX),
            'widget': 'eyeblink',
            'lim': LIM,
            'free': ('Z',),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.040, face_size * 0.045, face_size * 0.025)),
            'drivers': [{'key': eb_k_posX, 'axis': 'Z', 'dir': +1}]
        })

    eb_k_negX = brow_keys_negX[0] if brow_keys_negX else None
    if eb_k_negX:
        handled_keys.add(eb_k_negX)
        controls.append({
            'name': 'CTRL-Brow-R_Up',
            'collection': FACERIG_COLLECTION,
            'color': COL_BROW,
            'group': 'Face Eyebrow',
            'head': place(pos_brow_negX),
            'widget': 'eyeblink',
            'lim': LIM,
            'free': ('Z',),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.040, face_size * 0.045, face_size * 0.025)),
            'drivers': [{'key': eb_k_negX, 'axis': 'Z', 'dir': +1}]
        })

    # Eyebrow Center Pad
    eb_pad_drv = []
    eb_happy = pick_primary_key(keyblock, ["EB_happy_01", "EB_Happy_01", "EB_Smile_01"])
    eb_sad = pick_primary_key(keyblock, ["EB_Sad_01", "EB_Tired_01"])
    eb_angry = pick_primary_key(keyblock, ["EB_Angry_01", "EB_Evil_01"])
    eb_surprised = pick_primary_key(keyblock, ["EB_Surprised_01", "EB_Doubt_01"])

    if eb_happy: eb_pad_drv.append({'key': eb_happy, 'axis': 'Z', 'dir': +1})
    if eb_sad: eb_pad_drv.append({'key': eb_sad, 'axis': 'Z', 'dir': -1})
    if eb_angry: eb_pad_drv.append({'key': eb_angry, 'axis': 'X', 'dir': +1})
    if eb_surprised: eb_pad_drv.append({'key': eb_surprised, 'axis': 'X', 'dir': -1})

    if eb_pad_drv:
        controls.append({
            'name': 'CTRL-Eyebrow-Viseme-Pad',
            'collection': FACERIG_COLLECTION,
            'color': COL_EBRMASTER,
            'group': 'Face Eyebrow',
            'head': place(browCenter),
            'widget': 'pad',
            'lim': LIM,
            'free': ('X', 'Z'),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.055, face_size * 0.060, face_size * 0.038)),
            'drivers': eb_pad_drv
        })

    # 1.F Mouth Main Shift (Lipsmaster)
    mouth_drv = []
    jaw_open = pick_primary_key(keyblock, ["jawOpen", "jawOpen_a"])
    jaw_yi = pick_primary_key(keyblock, ["jawOpen_yi", "jawOpen_Smile_01_OP", "jawOpen_Happy_01_OP"])
    mouth_pucker = pick_primary_key(keyblock, ["mouthPucker", "jawOpen_wu"])
    mouth_funnel = pick_primary_key(keyblock, ["mouthFunnel", "jawOpen_o"])

    if jaw_yi: mouth_drv.append({'key': jaw_yi, 'axis': 'Z', 'dir': +1})
    if jaw_open:
        mouth_drv.append({'key': jaw_open, 'axis': 'Z', 'dir': -1})
        handled_keys.add(jaw_open)
    if mouth_pucker: mouth_drv.append({'key': mouth_pucker, 'axis': 'X', 'dir': +1})
    if mouth_funnel: mouth_drv.append({'key': mouth_funnel, 'axis': 'X', 'dir': -1})

    if mouth_drv:
        mouth_scale = Vector((face_size * 0.080, face_size * 0.080, face_size * 0.045))
        controls.append({
            'name': 'CTRL-Mouth-Shift',
            'collection': FACERIG_COLLECTION,
            'color': COL_MOUTH,
            'group': 'Face Mouth',
            'head': place(mouth),
            'widget': 'lipsmaster',
            'lim': LIM,
            'free': ('X', 'Z'),
            'range': 'both',
            'shape_scale': mouth_scale,
            'drivers': mouth_drv
        })

    # =========================================================================
    # 2. SIDE PANELS: HSR-STYLE LATERAL CONTROLLERS
    # =========================================================================

    MAX_PER_ROW = 10
    ROW_Z_GAP = 0.155
    ITEM_SP = 0.075

    slider_shape_scale = Vector((face_size * 0.035, face_size * 0.035, face_size * 0.035))
    label_scale = Vector((face_size * 0.045, face_size * 0.045, face_size * 0.045))

    # -------------------------------------------------------------------------
    # LEFT PANEL (A la IZQUIERDA de la cabeza, -right)
    # -------------------------------------------------------------------------
    left_panel_origin = fcx - right * (face_size * 0.72) + up * (face_size * 0.12)
    current_left_v = 0.0

    def add_left_panel_grid(items_list, group_name, color, widget_type='slider'):
        nonlocal current_left_v
        if not items_list:
            return
        total = len(items_list)
        for i, item in enumerate(items_list):
            if isinstance(item, tuple):
                c_name, drv_list = item[0], item[1]
            else:
                c_name = item.get('name') if isinstance(item, dict) else f"CTRL-{item}"
                drv_list = item.get('drivers') if isinstance(item, dict) else [{'key': item, 'axis': 'Z', 'dir': +1}]
                if isinstance(item, str):
                    handled_keys.add(item)

            row_idx = i // MAX_PER_ROW
            col_idx = i % MAX_PER_ROW

            h = -col_idx * (face_size * ITEM_SP)
            v = current_left_v - row_idx * (face_size * ROW_Z_GAP)

            controls.append({
                'name': c_name,
                'collection': FACERIG_COLLECTION,
                'color': color,
                'group': group_name,
                'head': place(left_panel_origin, h=h, v=v),
                'widget': widget_type,
                'lim': LIM,
                'free': ('Z',),
                'range': 'pos',
                'shape_scale': slider_shape_scale,
                'drivers': drv_list
            })

        num_rows = (total + MAX_PER_ROW - 1) // MAX_PER_ROW
        current_left_v -= num_rows * (face_size * ROW_Z_GAP) + (face_size * 0.015)

    # 2.A Eyebrows Grid (Cejas - Verde)
    unhandled_eb = [k for k in keyblock.keys() if (k.startswith("EB_") or "eyebrow" in k.lower() or "brow" in k.lower()) and k not in handled_keys]
    if unhandled_eb:
        eb_items = []
        for k in unhandled_eb:
            clean_k = k.replace("EB_", "").replace("Eyebrow_", "").replace("brow_", "")
            c_name = f"CTRL-EB_{clean_k}"
            handled_keys.add(k)
            eb_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_left_panel_grid(eb_items, 'Face Eyebrows', (0.30, 0.80, 0.35))

    # 2.B Eye Expressions Grid (Ojos - Cyan / Azul)
    unhandled_el = [k for k in keyblock.keys() if (k.startswith("EL_") or "eye" in k.lower() or "biyan" in k.lower()) and k not in handled_keys]
    if unhandled_el:
        el_items = []
        for k in unhandled_el:
            clean_k = k.replace("EL_", "").replace("Eye_", "")
            c_name = f"CTRL-EL_{clean_k}"
            handled_keys.add(k)
            el_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_left_panel_grid(el_items, 'Face Eye Expressions', COL_EYEAIM)

    # Add Left Panel Header Label
    if unhandled_eb or unhandled_el:
        controls.append({
            'name': 'LABEL-Brows_Eyes',
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(left_panel_origin, h=-4.5 * (face_size * ITEM_SP), v=face_size * 0.075),
            'widget': 'text:BROWS & EYES',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    # -------------------------------------------------------------------------
    # RIGHT PANEL (A la DERECHA de la cabeza, +right)
    # -------------------------------------------------------------------------
    right_panel_origin = fcx + right * (face_size * 0.72) + up * (face_size * 0.12)
    current_right_v_col1 = 0.0
    current_right_v_col2 = 0.0
    col2_offset = face_size * (ITEM_SP * MAX_PER_ROW + 0.06)

    def add_right_panel_grid(items_list, group_name, color, widget_type='slider', is_col2=False):
        nonlocal current_right_v_col1, current_right_v_col2
        if not items_list:
            return
        col_offset = col2_offset if is_col2 else 0.0
        v_base = current_right_v_col2 if is_col2 else current_right_v_col1

        total = len(items_list)
        for i, item in enumerate(items_list):
            if isinstance(item, tuple):
                c_name, drv_list = item[0], item[1]
            else:
                c_name = item.get('name') if isinstance(item, dict) else f"CTRL-{item}"
                drv_list = item.get('drivers') if isinstance(item, dict) else [{'key': item, 'axis': 'Z', 'dir': +1}]
                if isinstance(item, str):
                    handled_keys.add(item)

            row_idx = i // MAX_PER_ROW
            col_idx = i % MAX_PER_ROW

            h = col_idx * (face_size * ITEM_SP) + col_offset
            v = v_base - row_idx * (face_size * ROW_Z_GAP)

            controls.append({
                'name': c_name,
                'collection': FACERIG_COLLECTION,
                'color': color,
                'group': group_name,
                'head': place(right_panel_origin, h=h, v=v),
                'widget': widget_type,
                'lim': LIM,
                'free': ('Z',),
                'range': 'pos',
                'shape_scale': slider_shape_scale,
                'drivers': drv_list
            })

        num_rows = (total + MAX_PER_ROW - 1) // MAX_PER_ROW
        v_drop = num_rows * (face_size * ROW_Z_GAP) + (face_size * 0.015)
        if is_col2:
            current_right_v_col2 -= v_drop
        else:
            current_right_v_col1 -= v_drop

    # 2.C Visemes (jawOpen_a, jawOpen_yi, jawOpen_wu, jawOpen_ei, jawOpen_o - Amarillo, Columna 1)
    viseme_map = [
        ("A", ["jawOpen_a", "jawOpen_A"]),
        ("I", ["jawOpen_yi", "jawOpen_i", "jawOpen_I"]),
        ("U", ["jawOpen_wu", "jawOpen_u", "jawOpen_U"]),
        ("E", ["jawOpen_ei", "jawOpen_e", "jawOpen_E"]),
        ("O", ["jawOpen_o", "jawOpen_O"]),
    ]
    viseme_items = []
    for label, cands in viseme_map:
        pk = pick_primary_key(keyblock, cands)
        if pk and pk not in handled_keys:
            handled_keys.add(pk)
            viseme_items.append((f"CTRL-Viseme_{label}", [{'key': pk, 'axis': 'Z', 'dir': +1}]))
    if viseme_items:
        add_right_panel_grid(viseme_items, 'Face Visemes', COL_VISEME, is_col2=False)

    # 2.D Mouth Expressions (jawOpen_*_OP, jawOpen_*_CLO, mouth* - Púrpura, Columna 1)
    unhandled_mouth = [k for k in keyblock.keys() if (k.startswith("jawOpen_") or "mouth" in k.lower() or "jaw" in k.lower()) and k not in handled_keys]
    if unhandled_mouth:
        m_items = []
        for k in unhandled_mouth:
            clean_k = k.replace("jawOpen_", "").replace("mouth", "")
            c_name = f"CTRL-Mth_{clean_k}" if clean_k else f"CTRL-{k}"
            handled_keys.add(k)
            m_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(m_items, 'Face Mouth Expressions', COL_EXPRESSION, is_col2=False)

    # Add Right Panel Col 1 Header Label (Mouth)
    if viseme_items or unhandled_mouth:
        controls.append({
            'name': 'LABEL-Mouth',
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(right_panel_origin, h=2.0 * (face_size * ITEM_SP), v=face_size * 0.075),
            'widget': 'text:MOUTH',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    # 2.E 2D Special Manga Symbols (TDS_ & TD_ - Naranja / Magenta, Columna 2)
    td_keys = [k for k in keyblock.keys() if (k.startswith("TDS_") or k.startswith("TD_")) and k not in handled_keys]
    if td_keys:
        td_items = []
        for k in td_keys:
            c_name = f"CTRL-{k}"
            handled_keys.add(k)
            td_items.append((c_name, [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(td_items, 'Face 2D TDS Effects', COL_TD_SPECIAL, is_col2=True)

    # 2.F Extra Shapekeys Fallback (Cualquier otro shapekey no categorizado - Columna 2)
    remaining = [k for k in keyblock.keys() if k not in handled_keys]
    if remaining:
        rem_items = []
        for k in remaining:
            handled_keys.add(k)
            rem_items.append((f"CTRL-{k}", [{'key': k, 'axis': 'Z', 'dir': +1}]))
        add_right_panel_grid(rem_items, 'Face Extra Keys', (0.25, 0.75, 0.35), is_col2=True)

    # Add Right Panel Col 2 Header Label (2D VFX)
    if td_keys or remaining:
        controls.append({
            'name': 'LABEL-2D_VFX',
            'collection': FACERIG_COLLECTION,
            'color': COL_LABEL,
            'group': 'Face Labels',
            'head': place(right_panel_origin, h=col2_offset + 4.5 * (face_size * ITEM_SP), v=face_size * 0.075),
            'widget': 'text:2D VFX',
            'lim': 0.0,
            'free': (),
            'range': 'pos',
            'shape_scale': label_scale,
            'is_label': True,
            'drivers': []
        })

    return controls


# =============================================================================
# WIDGET BUILDER AND MESH SHAPES
# =============================================================================

EYE_MASTER_VERTS = [[-0.40348, 0.0, 0.32632], [-0.28803, 0.0, 0.29715], [-0.21194, 0.0, 0.24992], [0.0, 0.0, -0.19968], [0.0, 0.0, 0.19968], [-0.21194, 0.0, -0.24992], [-0.28803, 0.0, -0.29715], [-0.40348, 0.0, -0.32632], [-0.53133, 0.0, -0.31136], [-0.64684, 0.0, -0.24534], [-0.72806, 0.0, -0.13551], [-0.7574, 0.0, 0.0], [-0.72806, 0.0, 0.13551], [-0.64684, 0.0, 0.24534], [-0.53133, 0.0, 0.31136], [-0.11372, 0.0, 0.1995], [-0.11372, 0.0, -0.1995], [-0.03778, 0.0, -0.19971], [-0.03778, 0.0, 0.19971], [-0.16207, 0.0, 0.20723], [-0.16207, 0.0, -0.20723], [-0.34203, 0.0, 0.3157], [-0.24529, 0.0, 0.2746], [-0.0037, 0.0, -0.19969], [-0.07706, 0.0, 0.1997], [-0.24529, 0.0, -0.2746], [-0.34203, 0.0, -0.3157], [-0.4676, 0.0, -0.32523], [-0.59202, 0.0, -0.28451], [-0.693, 0.0, -0.19498], [-0.74991, 0.0, -0.06945], [-0.74991, 0.0, 0.06945], [-0.693, 0.0, 0.19498], [-0.59202, 0.0, 0.28451], [-0.4676, 0.0, 0.32523], [-0.18492, 0.0, 0.22542], [-0.07706, 0.0, -0.1997], [-0.0037, 0.0, 0.19969], [-0.14053, 0.0, -0.20011], [-0.14053, 0.0, 0.20011], [-0.18492, 0.0, -0.22542], [0.40348, 0.0, 0.32632], [0.28803, 0.0, 0.29715], [0.21194, 0.0, 0.24992], [-0.0, 0.0, -0.19968], [-0.0, 0.0, 0.19968], [0.21194, 0.0, -0.24992], [0.28803, 0.0, -0.29715], [0.40348, 0.0, -0.32632], [0.53133, 0.0, -0.31136], [0.64684, 0.0, -0.24534], [0.72806, 0.0, -0.13551], [0.7574, 0.0, 0.0], [0.72806, 0.0, 0.13551], [0.64684, 0.0, 0.24534], [0.53133, 0.0, 0.31136], [0.11372, 0.0, 0.1995], [0.11372, 0.0, -0.1995], [0.03778, 0.0, -0.19971], [0.03778, 0.0, 0.19971], [0.16207, 0.0, 0.20723], [0.16207, 0.0, -0.20723], [0.34203, 0.0, 0.3157], [0.24529, 0.0, 0.2746], [0.0037, 0.0, -0.19969], [0.07706, 0.0, 0.1997], [0.24529, 0.0, -0.2746], [0.34203, 0.0, -0.3157], [0.4676, 0.0, -0.32523], [0.59202, 0.0, -0.28451], [0.693, 0.0, -0.19498], [0.74991, 0.0, -0.06945], [0.74991, 0.0, 0.06945], [0.693, 0.0, 0.19498], [0.59202, 0.0, 0.28451], [0.4676, 0.0, 0.32523], [0.18492, 0.0, 0.22542], [0.07706, 0.0, -0.1997], [0.0037, 0.0, 0.19969], [0.14053, 0.0, -0.20011], [0.14053, 0.0, 0.20011], [0.18492, 0.0, -0.22542]]
EYE_MASTER_EDGES = [[1, 21], [0, 21], [2, 22], [1, 22], [17, 23], [3, 23], [18, 24], [15, 24], [6, 25], [5, 25], [7, 26], [6, 26], [8, 27], [7, 27], [9, 28], [8, 28], [10, 29], [9, 29], [11, 30], [10, 30], [12, 31], [11, 31], [13, 32], [12, 32], [14, 33], [13, 33], [0, 34], [14, 34], [19, 35], [2, 35], [16, 36], [17, 36], [4, 37], [18, 37], [20, 38], [16, 38], [15, 39], [19, 39], [5, 40], [20, 40], [42, 62], [41, 62], [43, 63], [42, 63], [58, 64], [44, 64], [59, 65], [56, 65], [47, 66], [46, 66], [48, 67], [47, 67], [49, 68], [48, 68], [50, 69], [49, 69], [51, 70], [50, 70], [52, 71], [51, 71], [53, 72], [52, 72], [54, 73], [53, 73], [55, 74], [54, 74], [41, 75], [55, 75], [60, 76], [43, 76], [57, 77], [58, 77], [45, 78], [59, 78], [61, 79], [57, 79], [56, 80], [60, 80], [46, 81], [61, 81]]
EYE_CIRCLE_VERTS = [[0.0, 0.0, 0.5], [-0.09755, 0.0, 0.49039], [-0.19134, 0.0, 0.46194], [-0.27779, 0.0, 0.41574], [-0.35355, 0.0, 0.35355], [-0.41574, 0.0, 0.27779], [-0.46194, 0.0, 0.19134], [-0.49039, 0.0, 0.09755], [-0.5, 0.0, -0.0], [-0.49039, 0.0, -0.09755], [-0.46194, 0.0, -0.19134], [-0.41574, 0.0, -0.27779], [-0.35355, 0.0, -0.35355], [-0.27779, 0.0, -0.41574], [-0.19134, 0.0, -0.46194], [-0.09755, 0.0, -0.49039], [0.0, 0.0, -0.5], [0.09755, 0.0, -0.49039], [0.19134, 0.0, -0.46194], [0.27779, 0.0, -0.41574], [0.35355, 0.0, -0.35355], [0.41574, 0.0, -0.27779], [0.46194, 0.0, -0.19134], [0.49039, 0.0, -0.09755], [0.5, 0.0, 0.0], [0.49039, 0.0, 0.09755], [0.46194, 0.0, 0.19134], [0.41573, 0.0, 0.27779], [0.35355, 0.0, 0.35355], [0.27779, 0.0, 0.41574], [0.19134, 0.0, 0.46194], [0.09754, 0.0, 0.49039]]
EYE_CIRCLE_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [29, 30], [30, 31], [0, 31]]
LIPS_MASTER_VERTS = [[0.0, 0.19233, 0.11033], [0.13499, 0.18609, 0.10769], [0.2648, 0.15671, 0.10386], [0.38443, 0.10515, 0.09906], [0.48929, 0.03669, 0.09225], [0.57534, -0.04048, 0.08167], [0.63928, -0.11576, 0.0649], [0.67866, -0.17692, 0.03916], [0.69195, -0.21104, 0.00157], [0.67866, -0.20938, -0.04918], [0.63928, -0.17943, -0.10838], [0.57534, -0.13291, -0.1699], [0.48929, -0.08096, -0.22793], [0.38443, -0.03318, -0.27744], [0.2648, 0.00301, -0.31449], [0.13499, 0.02292, -0.33643], [-0.0, 0.02596, -0.34248], [-0.135, 0.02292, -0.33643], [-0.2648, 0.00301, -0.31449], [-0.38443, -0.03318, -0.27744], [-0.48929, -0.08096, -0.22793], [-0.57534, -0.13291, -0.1699], [-0.63928, -0.17943, -0.10838], [-0.67866, -0.20938, -0.04917], [-0.69195, -0.21104, 0.00157], [-0.67866, -0.17692, 0.03916], [-0.63928, -0.11576, 0.0649], [-0.57534, -0.04048, 0.08167], [-0.48928, 0.03669, 0.09225], [-0.38442, 0.10515, 0.09906], [-0.26479, 0.15671, 0.10386], [-0.13499, 0.1861, 0.10769]]
LIPS_MASTER_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [29, 30], [30, 31], [0, 31]]
EYEBLINK_VERTS = [[-0.42167, 0.0, -0.22917], [0.42167, 0.0, -0.22917], [-0.42167, -0.0, 0.22917], [0.42167, -0.0, 0.22917], [0.46, 0.0, -0.125], [0.46, 0.0, -0.0], [0.46, -0.0, 0.125], [-0.23, 0.0, -0.25], [-0.0, 0.0, -0.25], [0.23, 0.0, -0.25], [-0.46, 0.0, -0.125], [-0.46, -0.0, 0.0], [-0.46, -0.0, 0.125], [-0.23, -0.0, 0.25], [0.0, -0.0, 0.25], [0.23, -0.0, 0.25], [0.4594, -0.0, 0.15592], [0.45521, -0.0, 0.1849], [0.44383, -0.0, 0.20996], [0.2869, 0.0, -0.24968], [0.34021, 0.0, -0.2474], [0.38633, 0.0, -0.24121], [-0.4594, -0.0, 0.15592], [-0.45521, -0.0, 0.1849], [-0.44383, -0.0, 0.20996], [0.2869, -0.0, 0.24968], [0.34021, -0.0, 0.2474], [0.38633, -0.0, 0.24121], [0.44383, 0.0, -0.20996], [0.45521, 0.0, -0.1849], [0.4594, 0.0, -0.15592], [0.46, 0.0, -0.0625], [0.46, -0.0, 0.0625], [-0.38633, 0.0, -0.24121], [-0.34021, 0.0, -0.2474], [-0.2869, 0.0, -0.24967], [-0.115, 0.0, -0.25], [0.115, 0.0, -0.25], [-0.44383, 0.0, -0.20996], [-0.45521, 0.0, -0.1849], [-0.4594, 0.0, -0.15592], [-0.46, 0.0, -0.0625], [-0.46, -0.0, 0.0625], [-0.38633, -0.0, 0.24121], [-0.34021, -0.0, 0.2474], [-0.2869, -0.0, 0.24967], [-0.115, -0.0, 0.25], [0.115, -0.0, 0.25], [-0.39485, -0.00338, -0.20822], [0.39485, 0.00338, -0.20822], [-0.39485, -0.00338, 0.20822], [0.39485, 0.00338, 0.20822], [0.43075, 0.00369, -0.11357], [0.43075, 0.00369, 0.11358], [-0.43075, -0.00369, -0.11357], [-0.43075, -0.00369, 0.11358], [0.43018, 0.00368, 0.14167], [0.42626, 0.00365, 0.168], [0.4156, 0.00356, 0.19077], [0.31857, 0.00273, -0.22478], [0.36176, 0.0031, -0.21916], [-0.43018, -0.00368, 0.14167], [-0.42626, -0.00365, 0.168], [-0.4156, -0.00356, 0.19077], [0.31857, 0.00273, 0.22479], [0.36176, 0.0031, 0.21917], [0.4156, 0.00356, -0.19077], [0.42626, 0.00365, -0.16799], [0.43018, 0.00368, -0.14167], [-0.36176, -0.0031, -0.21916], [-0.31857, -0.00273, -0.22478], [-0.4156, -0.00356, -0.19077], [-0.42626, -0.00365, -0.16799], [-0.43018, -0.00368, -0.14167], [-0.36176, -0.0031, 0.21917], [-0.31857, -0.00273, 0.22479]]
EYEBLINK_EDGES = [[6, 16], [16, 17], [17, 18], [3, 18], [9, 19], [19, 20], [20, 21], [1, 21], [12, 22], [22, 23], [23, 24], [2, 24], [15, 25], [25, 26], [26, 27], [3, 27], [1, 28], [28, 29], [29, 30], [4, 30], [4, 31], [5, 31], [5, 32], [6, 32], [0, 33], [33, 34], [34, 35], [7, 35], [7, 36], [8, 36], [8, 37], [9, 37], [0, 38], [38, 39], [39, 40], [10, 40], [10, 41], [11, 41], [11, 42], [12, 42], [2, 43], [43, 44], [44, 45], [13, 45], [13, 46], [14, 46], [14, 47], [15, 47], [53, 56], [56, 57], [57, 58], [51, 58], [59, 60], [49, 60], [55, 61], [61, 62], [62, 63], [50, 63], [64, 65], [51, 65], [49, 66], [66, 67], [67, 68], [52, 68], [48, 69], [69, 70], [48, 71], [71, 72], [72, 73], [54, 73], [50, 74], [74, 75]]
ISAAC_BLINK_TOP_VERTS = [[1.0, 0.0, 0.5638], [-1.0, 0.0, 0.5638], [-0.0, 0.0, 0.8878], [-0.0, 0.0, -0.484], [-0.5916, 0.0, -0.0569], [0.5916, 0.0, -0.0569], [0.6306, 0.0, 0.8199], [-0.6306, 0.0, 0.8199], [-1.0, 0.0, 0.5638], [-0.0, 0.0, -0.484], [1.0, 0.0, 0.5638]]
ISAAC_BLINK_TOP_EDGES = [[7, 1], [4, 1], [3, 4], [6, 2], [5, 3], [0, 5], [0, 6], [2, 7], [1, 8], [3, 9], [0, 10]]
ISAAC_BLINK_BOT_VERTS = [[-1.0, 0.0, -0.5638], [1.0, 0.0, -0.5638], [0.0, 0.0, -0.8878], [0.0, 0.0, 0.484], [0.5916, 0.0, 0.0569], [-0.5916, 0.0, 0.0569], [-0.6306, 0.0, -0.8199], [0.6306, 0.0, -0.8199], [1.0, 0.0, -0.5638], [0.0, 0.0, 0.484], [-1.0, 0.0, -0.5638]]
ISAAC_BLINK_BOT_EDGES = [[7, 1], [4, 1], [3, 4], [6, 2], [5, 3], [0, 5], [0, 6], [2, 7], [1, 8], [3, 9], [0, 10]]


def make_widget(kind, coll):
    name = "WGT-Face_" + kind
    obj = bpy.data.objects.get(name)
    if obj:
        return obj
    if kind == 'isaac_blink_top':
        verts = ISAAC_BLINK_TOP_VERTS
        edges = ISAAC_BLINK_TOP_EDGES
    elif kind == 'isaac_blink_bot':
        verts = ISAAC_BLINK_BOT_VERTS
        edges = ISAAC_BLINK_BOT_EDGES
    elif kind == 'lipsmaster':
        verts = LIPS_MASTER_VERTS
        edges = LIPS_MASTER_EDGES
    elif kind == 'eyeblink':
        verts = EYEBLINK_VERTS
        edges = EYEBLINK_EDGES
    elif kind == 'eyemaster':
        verts = EYE_MASTER_VERTS
        edges = EYE_MASTER_EDGES
    elif kind == 'eyecircle':
        verts = EYE_CIRCLE_VERTS
        edges = EYE_CIRCLE_EDGES
    elif kind == 'pad':
        s = 1.0
        verts = [(-s, 0, -s), (s, 0, -s), (s, 0, s), (-s, 0, s)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
    elif kind == 'slider':
        verts = [(0, 0, -1), (0, 0, 1), (-0.45, 0, 1), (0.45, 0, 1)]
        edges = [(0, 1), (2, 3)]
    elif kind == 'ring':
        verts, edges, N = [], [], 20
        for i in range(N):
            a = 2 * math.pi * i / N
            verts.append((math.cos(a), 0.0, math.sin(a)))
            edges.append((i, (i + 1) % N))
    elif kind == 'triangle':
        verts = [(0, 0, 1), (-0.9, 0, -0.7), (0.9, 0, -0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    elif kind == 'triangle_down':
        verts = [(0, 0, -1), (-0.9, 0, 0.7), (0.9, 0, 0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    else:
        verts, edges = [(0, 0, 0)], []

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, edges, [])
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return obj


def make_text_widget(text_string, wgt_coll):
    safe_name = text_string.replace(' ', '_').replace('&', 'AND').replace('/', '_')
    name = "WGT-Face_Label_" + safe_name
    obj = bpy.data.objects.get(name)
    if obj:
        return obj

    try:
        curve_data = bpy.data.curves.new(name=name + "_Curve", type='FONT')
        curve_data.body = text_string
        curve_data.size = 1.0
        curve_data.align_x = 'CENTER'
        curve_data.align_y = 'CENTER'
        curve_data.fill_mode = 'NONE'

        temp_obj = bpy.data.objects.new(name + "_Temp", curve_data)
        bpy.context.scene.collection.objects.link(temp_obj)

        depsgraph = bpy.context.evaluated_depsgraph_get()
        eval_obj = temp_obj.evaluated_get(depsgraph)
        mesh_from_eval = bpy.data.meshes.new_from_object(eval_obj)
        mesh_from_eval.name = name

        # Rotate mesh vertices so text stands upright facing the front camera
        # Text X (width) -> Bone -X (which is +X in world)
        # Text Y (height) -> Bone +Z (which is +Z in world)
        # Text Z (normal) -> Bone Y (0 depth)
        for v in mesh_from_eval.vertices:
            v.co = Vector((-v.co.x, 0.0, v.co.y))

        bpy.context.scene.collection.objects.unlink(temp_obj)
        bpy.data.objects.remove(temp_obj, do_unlink=True)
        bpy.data.curves.remove(curve_data, do_unlink=True)

        wgt_obj = bpy.data.objects.new(name, mesh_from_eval)
        wgt_coll.objects.link(wgt_obj)
        return wgt_obj
    except Exception as ex:
        # Fallback wireframe bounding box with center line
        s_w = max(1.0, len(text_string) * 0.4)
        s_h = 0.5
        verts = [(-s_w, 0, -s_h), (s_w, 0, -s_h), (s_w, 0, s_h), (-s_w, 0, s_h)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata(verts, edges, [])
        wgt_obj = bpy.data.objects.new(name, mesh)
        wgt_coll.objects.link(wgt_obj)
        return wgt_obj


def get_widget_collection():
    # Prefer per-character WGTS_<Char> (Append-safe), canonical name shared with rig
    try:
        from setup_wizard.ui.character_settings_utils import resolve_settings_armature, resolve_character_name
        arm = resolve_settings_armature(bpy.context)
        if arm is not None:
            char_name = resolve_character_name(arm)
            if char_name:
                per_char = bpy.data.collections.get(f"WGTS_{char_name}")
                if per_char is not None:
                    return per_char
                try:
                    from setup_wizard.character_rig_setup.wgts_isolation import get_or_create_char_wgts, get_char_collection
                    char_coll = get_char_collection(arm, char_name)
                    if char_coll is not None:
                        return get_or_create_char_wgts(char_coll, char_name)
                except Exception:
                    pass
    except Exception:
        pass
    for name_candidate in ("WGTS", "wgt", "WGTS_FaceRig"):
        coll = bpy.data.collections.get(name_candidate)
        if coll:
            return coll

    for c in bpy.data.collections:
        c_low = c.name.lower()
        if "wgts" in c_low or "wgt" in c_low:
            return c

    coll = bpy.data.collections.new("WGTS")
    return coll


def finalize_widget_collection(wgt_coll):
    if not wgt_coll:
        return
    try:
        wgt_coll.hide_viewport = True
    except Exception:
        pass
    if wgt_coll.name in bpy.context.scene.collection.children:
        try:
            bpy.context.scene.collection.children.unlink(wgt_coll)
        except Exception:
            pass


def lighten(rgb, amt):
    return tuple(min(1.0, c + amt) for c in rgb)


def apply_color(armature, pb, group_name, rgb, cache):
    if is_blender_3():
        grp = cache.get(group_name)
        if grp is None:
            grp = armature.pose.bone_groups.get(group_name)
            if grp is None:
                grp = armature.pose.bone_groups.new(name=group_name)
            grp.color_set = 'CUSTOM'
            grp.colors.normal = rgb
            grp.colors.select = lighten(rgb, 0.25)
            grp.colors.active = lighten(rgb, 0.5)
            cache[group_name] = grp
        pb.bone_group = grp
    else:
        try:
            pb.color.palette = 'CUSTOM'
            cc = pb.color.custom
            cc.normal = rgb
            cc.select = lighten(rgb, 0.25)
            cc.active = lighten(rgb, 0.5)
        except Exception:
            pass


def purge_previous(armature):
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones
    for b in list(eb):
        if b.name.startswith("CTRL-") or b.name.startswith("LABEL-") or b.name == "Face-Root":
            eb.remove(b)
    bpy.ops.object.mode_set(mode='OBJECT')
    for o in list(bpy.data.objects):
        if o.name.startswith("WGT-Face_"):
            bpy.data.objects.remove(o, do_unlink=True)


# =============================================================================
# FACE RIG SETUP AND DRIVER GENERATION
# =============================================================================

def setup_nte_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size, keyblock):
    print(f"Building {len(controls)} NTE face controls on '{armature.name}' under '{head_name}'")

    amw_inv = armature.matrix_world.inverted()
    bone_len = face_size * BONE_LEN_F

    def to_arm_point(p):
        return amw_inv @ p

    def to_arm_vec(v):
        return amw_inv.to_3x3() @ v

    fwd_arm = to_arm_vec(fwd).normalized()
    up_arm = to_arm_vec(up).normalized()

    bpy.context.view_layer.objects.active = armature
    try:
        armature.data.use_mirror_x = False
    except Exception:
        pass

    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones

    root = eb.get("Face-Root") or eb.new("Face-Root")
    head_edit = eb.get(head_name)
    root.head = head_edit.head.copy() if head_edit else Vector((0, 0, 0))
    root.tail = root.head + up_arm * bone_len * 2.0
    root.use_deform = False
    if head_edit:
        root.parent = head_edit

    for c in controls:
        b = eb.get(c['name']) or eb.new(c['name'])
        h = to_arm_point(c['head'])
        b.head = h
        b.tail = h + fwd_arm * bone_len
        b.use_deform = False
        try:
            b.align_roll(up_arm)
        except Exception:
            pass
        b.parent = root
        b.use_connect = False

    bpy.ops.object.mode_set(mode='OBJECT')

    # Bone Collections Setup
    coll_names = [c['collection'] for c in controls if c.get('collection')]
    if not is_blender_3() and hasattr(armature.data, "collections"):
        for cn in set(coll_names):
            coll = armature.data.collections.get(cn) or armature.data.collections.new(cn)
            try:
                coll.is_visible = True
            except Exception:
                pass
            try:
                coll.rigify_ui_row = RIGIFY_UI_ROW
            except Exception:
                pass
        for c in controls:
            bone = armature.data.bones.get(c['name'])
            coll = armature.data.collections.get(c.get('collection', FACERIG_COLLECTION))
            if bone and coll:
                coll.assign(bone)

        # Place Face-Root into Face (Secondary)
        sec_coll = None
        for scn in ["Face (Secondary)", "Face(Secondary)", "Face Secondary"]:
            if scn in armature.data.collections:
                sec_coll = armature.data.collections[scn]
                break
        if not sec_coll:
            sec_coll = armature.data.collections.new("Face (Secondary)")
            try:
                sec_coll.is_visible = False
            except Exception:
                pass
            try:
                sec_coll.rigify_ui_row = RIGIFY_UI_ROW
            except Exception:
                pass

        root_bone = armature.data.bones.get("Face-Root")
        if root_bone and sec_coll:
            for c in list(root_bone.collections):
                c.unassign(root_bone)
            sec_coll.assign(root_bone)
    elif is_blender_3():
        root_bone = armature.data.bones.get("Face-Root")
        if root_bone:
            root_bone.layers = [i == 1 for i in range(32)]

    wgt_coll = get_widget_collection()
    bpy.ops.object.mode_set(mode='POSE')
    color_cache = {}

    for c in controls:
        pb = armature.pose.bones.get(c['name'])
        if not pb:
            continue

        is_lbl = c.get('is_label', False)

        if is_lbl:
            # Lock all transforms for header label bones
            for i in range(3):
                pb.lock_location[i] = True
                pb.lock_rotation[i] = True
                pb.lock_scale[i] = True
            pb.lock_rotation_w = True
        else:
            lim = c['lim']
            free = c['free']
            rng = c['range']
            if rng == 'both':
                lo, hi = -lim, lim
            elif rng == 'neg':
                lo, hi = -lim, 0.0
            else:
                lo, hi = 0.0, lim

            pb.lock_location[0] = 'X' not in free
            pb.lock_location[1] = True
            pb.lock_location[2] = 'Z' not in free
            for i in range(3):
                pb.lock_rotation[i] = True
                pb.lock_scale[i] = True
            pb.lock_rotation_w = True

            con = pb.constraints.new(type='LIMIT_LOCATION')
            con.owner_space = 'LOCAL'
            con.use_transform_limit = True
            con.use_min_x = con.use_max_x = True
            con.use_min_y = con.use_max_y = True
            con.use_min_z = con.use_max_z = True
            con.min_y = con.max_y = 0.0
            con.min_x = lo if 'X' in free else 0.0
            con.max_x = hi if 'X' in free else 0.0
            con.min_z = lo if 'Z' in free else 0.0
            con.max_z = hi if 'Z' in free else 0.0

        if c['widget'].startswith('text:'):
            text_str = c['widget'].split(':', 1)[1]
            pb.custom_shape = make_text_widget(text_str, wgt_coll)
        else:
            pb.custom_shape = make_widget(c['widget'], wgt_coll)

        try:
            pb.use_custom_shape_bone_size = False
        except Exception:
            pass
        ss = c.get('shape_scale')
        pb.custom_shape_scale_xyz = ss if ss is not None else Vector((c.get('lim', 0.02) * WIDGET_F,) * 3)

        apply_color(armature, pb, c['group'], c['color'], color_cache)

    bpy.ops.object.mode_set(mode='OBJECT')

    # Build Shape Key Drivers
    agg = {}
    for c in controls:
        for d in c['drivers']:
            agg.setdefault(d['key'], []).append({
                'bone': c['name'],
                'axis': d['axis'],
                'dir': d['dir'],
                'lim': c['lim'],
                'bidir': d.get('bidir', False),
                'gain': d.get('gain', 1.0),
                'is_diagonal': d.get('is_diagonal', False),
                'extra_axis': d.get('extra_axis'),
                'extra_dir': d.get('extra_dir', 1)
            })

    for key, entries in agg.items():
        sk = keyblock.get(key)
        if sk is None:
            continue
        sk.slider_min = -1.0 if any(e['bidir'] for e in entries) else 0.0
        try:
            sk.driver_remove("value")
        except Exception:
            pass
        drv = sk.driver_add("value").driver
        drv.type = 'SCRIPTED'
        terms = []
        var_count = 0

        for e in entries:
            vn1 = f"v{var_count}"
            var1 = drv.variables.new()
            var1.name = vn1
            var1.type = 'TRANSFORMS'
            tgt1 = var1.targets[0]
            tgt1.id = armature
            tgt1.bone_target = e['bone']
            tgt1.transform_type = 'LOC_' + e['axis']
            tgt1.transform_space = 'LOCAL_SPACE'
            var_count += 1

            sign1 = '' if e['dir'] > 0 else '-'
            lim = e['lim']
            gain = e.get('gain', 1.0)

            if e.get('is_diagonal') and e.get('extra_axis'):
                vn2 = f"v{var_count}"
                var2 = drv.variables.new()
                var2.name = vn2
                var2.type = 'TRANSFORMS'
                tgt2 = var2.targets[0]
                tgt2.id = armature
                tgt2.bone_target = e['bone']
                tgt2.transform_type = 'LOC_' + e['extra_axis']
                tgt2.transform_space = 'LOCAL_SPACE'
                var_count += 1
                sign2 = '' if e['extra_dir'] > 0 else '-'

                term = f"max(0.0, min({sign1}{vn1} / {lim!r}, {sign2}{vn2} / {lim!r}))"
            elif e.get('bidir'):
                term = f"{sign1}{vn1} / {lim!r}"
            else:
                term = f"max(0.0, {sign1}{vn1} / {lim!r})"

            if gain != 1.0:
                term = f"({gain!r} * {term})"
            terms.append(term)

        drv.expression = terms[0] if len(terms) == 1 else "max(" + ", ".join(terms) + ")"


def setup_common_face_materials_if_present():
    """
    Ensures that any common_face (or MI_common_face_mask) material in the scene
    has the correct Mix Shader + Transparent BSDF + Image Texture nodes connected.
    """
    for mat in bpy.data.materials:
        if not mat:
            continue
        mname = mat.name.lower()
        if any(k in mname for k in ["common_face", "common_face_mask", "face_mask", "facemask"]):
            try:
                from setup_wizard.replace_default_materials_setup.game_default_material_replacers import setup_common_face_material
                setup_common_face_material(mat)
            except Exception as ex:
                print(f"Notice: Setting up common_face material {mat.name}: {ex}")


def cleanup_head_driver_to_wgts_if_present(armature=None, head_name=None):
    """
    Finds Head Origin / Head Driver object hierarchy and places them into the WGTS collection,
    ensuring Child Of constraint points to the armature's head bone.
    """
    head_origin = (
        bpy.data.objects.get("Head Origin")
        or bpy.data.objects.get("Head Driver")
        or bpy.data.objects.get("Head Direction")
    )
    if not head_origin:
        for obj in bpy.data.objects:
            if obj.type == "EMPTY" and (
                obj.name.startswith("Head Origin")
                or obj.name.startswith("Head Driver")
                or obj.name.startswith("Head Direction")
            ):
                head_origin = obj
                break

    if not head_origin:
        return

    # Find WGTS collection
    wgt_coll = None
    for c in bpy.data.collections:
        if c.name.startswith("WGTS") or c.name.lower() == "wgt":
            wgt_coll = c
            break
    if not wgt_coll:
        wgt_coll = bpy.data.collections.get("WGTS") or bpy.data.collections.get("wgt")
    if not wgt_coll:
        wgt_coll = bpy.data.collections.new("WGTS")
        try:
            bpy.context.scene.collection.children.link(wgt_coll)
        except Exception:
            pass

    # Ensure Child Of constraint on Head Origin
    if armature and head_name:
        con = None
        for c in head_origin.constraints:
            if c.type == "CHILD_OF":
                con = c
                break
        if not con:
            con = head_origin.constraints.new("CHILD_OF")
        con.target = armature
        con.subtarget = head_name

    def get_all_children(obj):
        res = []
        for ch in obj.children:
            res.append(ch)
            res.extend(get_all_children(ch))
        return res

    all_objs = [head_origin] + get_all_children(head_origin)
    for o in all_objs:
        if o.name not in wgt_coll.objects:
            wgt_coll.objects.link(o)
        for col in list(o.users_collection):
            if col != wgt_coll:
                try:
                    col.objects.unlink(o)
                except Exception:
                    pass
        try:
            o.hide_viewport = True
            o.hide_render = True
        except Exception:
            pass

    try:
        wgt_coll.hide_viewport = True
        wgt_coll.hide_render = True
    except Exception:
        pass


def nte_face_rig_main():
    faceobj = find_face_mesh()
    if faceobj is None:
        print("NTE Face Rig: No face mesh found.")
        return
    if faceobj.data.shape_keys is None:
        print("NTE Face Rig: Face mesh has no shape keys.")
        return

    keyblock = faceobj.data.shape_keys.key_blocks
    armature, head_name = find_armature_and_head(faceobj)
    fwd, right, up, face_size, fcx = get_face_metrics(faceobj, armature, head_name)
    controls = plan_nte_controls(faceobj, armature, head_name, keyblock)

    if not controls:
        print("NTE Face Rig: No drivable shape keys found.")
        return

    if CLEAN_REBUILD:
        purge_previous(armature)

    setup_nte_face_rig(faceobj, controls, armature, head_name, fwd, up, face_size, keyblock)

    wgt_coll = get_widget_collection()
    finalize_widget_collection(wgt_coll)

    setup_common_face_materials_if_present()
    cleanup_head_driver_to_wgts_if_present(armature, head_name)

    print(f"\nNTE Face Rig complete: {len(controls)} controls built on '{armature.name}'.\n")


if __name__ == "__main__":
    nte_face_rig_main()


```

## Source file: setup_wizard/character_rig_setup/zzz_face_rig.py

{
  "path": "setup_wizard/character_rig_setup/zzz_face_rig.py",
  "blob_sha": "30a02e8f2c62d446bb2401fd6d5f38cbd84d9d05",
  "sha256": "7f11b94f8f7298e9809185fe10ef52e741775f1d5317bbd951e0a1cc604f9ab3",
  "lines": 2492,
  "bytes": 120580,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 237,
    "jaw": 1,
    "mouth": 81,
    "teeth": 2,
    "deform": 10,
    "iris": 2,
    "shader": 1,
    "rig": 146,
    "face": 269,
    "lip": 33
  },
  "url": "https://github.com/PaoloESAN/gacha-setup/blob/3e40423dbec489368696c4f89a2bfb285662cdc1/setup_wizard/character_rig_setup/zzz_face_rig.py",
  "function_signature_candidates": [
    {
      "line": 58,
      "text": "def is_blender_5():"
    },
    {
      "line": 62,
      "text": "def is_blender_4():"
    },
    {
      "line": 66,
      "text": "def is_blender_3():"
    },
    {
      "line": 77,
      "text": "def shapekeyrename(keyblock):"
    },
    {
      "line": 141,
      "text": "def find_armature_and_head(mesh_obj):"
    },
    {
      "line": 179,
      "text": "def find_eye_bone_pair(armature):"
    },
    {
      "line": 203,
      "text": "def find_eyebrow_bones(armature):"
    },
    {
      "line": 216,
      "text": "def find_mouth_bones(armature):"
    },
    {
      "line": 227,
      "text": "def find_extra_face_bones(armature):"
    },
    {
      "line": 238,
      "text": "def build_mouth_bone_controls(armature, fwd, up, right, face_size):"
    },
    {
      "line": 294,
      "text": "def build_extra_face_bone_controls(armature, fwd, up, right, face_size):"
    },
    {
      "line": 321,
      "text": "def eyebrow_side(name):"
    },
    {
      "line": 330,
      "text": "def is_eyebrow_key(name):"
    },
    {
      "line": 336,
      "text": "def skneyebrow_pos(armature, seg, side):"
    },
    {
      "line": 345,
      "text": "def build_brow_shapekey_controls(mesh_obj, armature, fwd, up, face_size):"
    },
    {
      "line": 352,
      "text": "def place_s(feature, h=0.0, v=0.0):"
    },
    {
      "line": 457,
      "text": "def feature_centroid(mesh_obj, key_names, side=None):"
    },
    {
      "line": 503,
      "text": "def feature_extent(mesh_obj, key_names, right, up):"
    },
    {
      "line": 532,
      "text": "def feature_extreme(mesh_obj, key_names, axis):"
    },
    {
      "line": 561,
      "text": "def face_frame(mesh_obj, armature=None):"
    },
    {
      "line": 593,
      "text": "def plan_controls(mesh_obj, fwd, right, up, face_size):"
    },
    {
      "line": 601,
      "text": "def place(feature, h=0.0, v=0.0):"
    },
    {
      "line": 604,
      "text": "def hkey(pos, neg):"
    },
    {
      "line": 607,
      "text": "def vkey(pos, neg):"
    },
    {
      "line": 641,
      "text": "def corner(side):"
    },
    {
      "line": 813,
      "text": "def get_widget_collection():"
    },
    {
      "line": 830,
      "text": "def finalize_widget_collection(wgt_coll):"
    },
    {
      "line": 909,
      "text": "def make_widget(kind, coll):"
    },
    {
      "line": 1015,
      "text": "def lighten(rgb, amt):"
    },
    {
      "line": 1019,
      "text": "def apply_color(armature, pb, group_name, rgb, cache):"
    },
    {
      "line": 1043,
      "text": "def is_face_rig_bone(bone_name):"
    },
    {
      "line": 1066,
      "text": "def purge_previous(armature):"
    },
    {
      "line": 1110,
      "text": "def make_facerig_root_widget(coll):"
    },
    {
      "line": 1134,
      "text": "def setup_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size):"
    },
    {
      "line": 1139,
      "text": "def to_arm_point(p):"
    },
    {
      "line": 1142,
      "text": "def to_arm_vec(v):"
    },
    {
      "line": 1423,
      "text": "def assign_bones_to_other_collection(armature, bone_names):"
    },
    {
      "line": 1448,
      "text": "def remove_eyetrack_bones(armature):"
    },
    {
      "line": 1466,
      "text": "def hide_mechanism_bones(armature):"
    },
    {
      "line": 1485,
      "text": "def get_facerig_bone_collection(armature, name=FACERIG_COLLECTION):"
    },
    {
      "line": 1496,
      "text": "def setup_lookat_eyes(armature, head_name, fwd, up, face_size):"
    },
    {
      "line": 1555,
      "text": "def is_valid_eye_vg(name, is_left):"
    },
    {
      "line": 1868,
      "text": "def get_facerig_detail_bone_collection(armature):"
    },
    {
      "line": 1875,
      "text": "def get_huge_facerig_widgets(wgt_coll):"
    },
    {
      "line": 1936,
      "text": "def update_rig_ui_text_for_face_detail(armature=None):"
    },
    {
      "line": 1993,
      "text": "def setup_facerig_detail_panel(faceobj, armature, head_name, keyblock):"
    },
    {
      "line": 2246,
      "text": "def _get_or_add_driver(sk):"
    },
    {
      "line": 2372,
      "text": "def zzz_face_rig_main():"
    }
  ]
}

```text

# BIG CREDITS to these people <3:
# Poke/Enthralpy | Driver Logic
# Isaac/Just_ScaasI | Facerig Logic + Widgets
# The_Crabnuts/Kan_Natto | Facerig Logic + Widgets

import bpy
import os
import tempfile
import mathutils
from mathutils import Vector, Matrix

HEAD_BONE_NAME   = None
CLEAN_REBUILD    = True
FLIP_HORIZONTAL  = True
FLIP_VERTICAL    = False
FLIP_EYE_LR      = False

BONE_LEN_F   = 0.020
OFFSET_F     = 0.120
TRAVEL_F     = 0.050
SPACING_F    = 0.055
WIDGET_F     = 1.0
EYE_LOOK_FWD_F = 0.45
SPINE_REF_Z = None
FACERIG_COLLECTION = "Face"
RIGIFY_UI_ROW = 1

EYE_HL_F     = 0.013
TRI_F        = 0.022
EMOTE_W_F    = 0.011
EMOTE_H_F    = 0.006
EMOTE_LIM_F  = 0.022
EMOTE_SPACING_F = 0.045
EMOTE_FWD_F  = 0.050
MOUTH_SHIFT_F = 0.55
MOUTH_RAISE_F = 0.040
EBR_WGT_F    = 0.016
EBR_MASTER_F = 0.030

COL_MOUTH   = (0.90, 0.20, 0.20)
COL_CORNER  = (0.15, 0.85, 0.30)
COL_VISEME  = (0.95, 0.85, 0.20)
COL_EYELID  = (0.90, 0.15, 0.15)
COL_EYEAIM  = (0.20, 0.85, 0.90)
COL_EYEGREEN = (0.20, 0.85, 0.30)
COL_EYEMOTE = (0.90, 0.20, 0.20)
COL_BROW    = (0.30, 0.80, 0.35)
COL_BROWSAD = (0.95, 0.45, 0.75)
COL_EBRBONE = (0.20, 0.55, 0.95)
COL_EBRMASTER = (0.65, 0.35, 0.90)

HEAD_CANDIDATES = [
    "DEF-spine.006", "spine.006", "head", "Head", "Head_M", "head_M",
    "Bip001 Head", "Bip001_Head", "Bip001Head", "Bip001 头", "頭", "头",
]


def is_blender_5():
    return bpy.app.version[0] == 5


def is_blender_4():
    return bpy.app.version[0] == 4


def is_blender_3():
    return bpy.app.version[0] == 3


_blender_ver = bpy.app.version[0]
if _blender_ver < 3:
    raise Exception("This script targets Blender 3.x, 4.x, or 5.x.")

ver = _blender_ver


def shapekeyrename(keyblock):
    for sk in keyblock:
        if sk.name.endswith("_Unagi") or sk.name.endswith("_Anton") or sk.name.endswith("_Corin"):
            sk.name = sk.name[:-6]
        if sk.name.endswith("_NuoCha"):
            sk.name = sk.name[:-7]
    rename_map = {
        "Mouth_↖_Ben": "Fac_Mth_R_Up",   "Mouth_↗_Ben": "Fac_Mth_L_Up",
        "Mouth_↙_Ben": "Fac_Mth_R_Down", "Mouth_↘_Ben": "Fac_Mth_L_Down",
        "Mouth_上颌↑_Ben": "Fac_Mth_Up",  "Mouth_下颌↓_Ben": "Fac_Mth_Down",
        "Mouth_呲_L_Ben": "Fac_Mth_L_In", "Mouth_呲_R_Ben": "Fac_Mth_R_In",
        "Eye_Open_↑_Ben": "Fac_Eye_R_Open", "Mouth_Oo_Ben": "Fac_Mth_UuOo",
        "Eye_Close2_Ben": "Fac_Eye_Sad",
        "Eye_Ball_↑_Ben": "Eye_Up", "Eye_Ball_↓_Ben": "Eye_Down",
        "Eye_Ball_→_Ben": "Eye_Left", "Eye_Ball_←_Ben": "Eye_Right",
        "Eye_Ball_No_Ben": "O_O",
        "Mouth_啧_R_Ben": "Fac_Mth_R_Out", "Mouth_啧_L_Ben": "Fac_Mth_L_Out",
        "Mouth_Ii1": "Fac_Mth_Ii",
        "Fac_Mth_Aa": "Fac_Mth_Aa1",
        "Fac_Mth_ooR": "Fac_Mth_R_Out", "Fac_Mth_Roo": "Fac_Mth_R_In",
        "Fac_Mth_Loo": "Fac_Mth_L_Out", "Fac_Mth_ooL": "Fac_Mth_L_In",
        "Fac_Mth_oo_RDown": "Fac_Mth_R_Down", "Fac_Mth_LDown_oo": "Fac_Mth_L_Down",
        "Fac_Mth_LUp_oo": "Fac_Mth_L_Up",
        "Fac_Eye_Open_L": "Fac_Eye_L_Open",
        "Fac_Eye_LowEyeUP": "Fac_Eye_LowlidUp",
        "Fac_Mth_Laugh1": "Fac_Mth_Laugh",
        "EB_↑": "Fac_Ebr_Up", "EB_↓": "Fac_Ebr_Down",
        "EB_Angry": "Fac_Ebr_Angry", "EB_Relax": "Fac_Ebr_Relax",
        "EB_困扰": "Fac_Ebr_Sad",
        "Fac_Eyebrow_↓": "Fac_Ebr_Down", "Fac_Eyebrow_Angry": "Fac_Ebr_Angry",
        "Fac_Eyebrow_L↑": "Fac_Ebr_L_Up", "Fac_Eyebrow_R↑": "Fac_Ebr_R_Up",
        "Fac_Eyebrow_Relax": "Fac_Ebr_Relax", "Fac_Eyebrow_困扰": "Fac_Ebr_Relax",
        "Eye_↙↘": "Fac_Eye_BLBR", "Eye_Angry": "Fac_Eye_Angry",
        "Eye_Close": "Fac_Eye_Close",
        "Eye_Open_L": "Fac_Eye_L_Open", "Eye_Open_R": "Fac_Eye_R_Open",
        "Eye_Wink_L": "Fac_Eye_L_Wink", "Eye_Wink_R": "Fac_Eye_R_Wink",
        "EYE_Wink_L": "Fac_Eye_L_Wink", "EYE_Wink_R": "Fac_Eye_R_Wink",
        "Eye_半闭": "Fac_Eye_HalfClose", "Eye_困扰": "Fac_Eye_Sad",
        "Eye_认真": "Fac_Eye_MidDown", "Eye_下眼睑↑": "Fac_Eye_LowlidUp",
        "Mouth_△": "Fac_Mth_Triangle", "Mouth_↑": "Fac_Mth_Up",
        "Mouth_→": "Fac_Mth_Left", "Mouth_↓": "Fac_Mth_Down", "Mouth_←": "Fac_Mth_Right",
        "Mouth_Aa1": "Fac_Mth_Aa1", "Mouth_Aa2": "Fac_Mth_Aa2",
        "Mouth_Aa3Shout": "Fac_Mth_Aa3Shout", "Mouth_AaTalk": "Fac_Mth_AaTalk",
        "Mouth_Ee": "Fac_Mth_Ee", "Mouth_Ii": "Fac_Mth_Ii",
        "Mouth_Uu_Ben": "Fac_Mth_Uu", "Mouth_Laugh": "Fac_Mth_Laugh",
        "Mouth_Laugh2": "Fac_Mth_Laugh2",
        "Mouth_oo←": "Fac_Mth_L_In", "Mouth_↖oo": "Fac_Mth_L_Up",
        "Mouth_←oo": "Fac_Mth_R_Out", "Mouth_↙oo": "Fac_Mth_R_Down",
        "Mouth_→oo": "Fac_Mth_R_In", "Mouth_oo↗": "Fac_Mth_R_Up",
        "Mouth_oo→": "Fac_Mth_L_Out", "Mouth_oo↘": "Fac_Mth_L_Down",
        "Mouth_Oo": "Fac_Mth_Oo", "Mouth_Uu": "Fac_Mth_Uu", "Mouth_UuOo": "Fac_Mth_UuOo",
    }
    for key in rename_map.keys():
        try:
            keyblock[key].name = rename_map[key]
        except Exception:
            for sk in keyblock:
                if key in sk.name:
                    sk.name = rename_map[key]
    for sk in keyblock:
        if sk.name.endswith("_Ben"):
            sk.name = sk.name[:-4]


def find_armature_and_head(mesh_obj):
    armature = None
    if mesh_obj:
        for m in mesh_obj.modifiers:
            if m.type == 'ARMATURE' and m.object:
                armature = m.object
                break
    if armature is None:
        for o in bpy.data.objects:
            if o.type == 'ARMATURE' and o.name in bpy.context.view_layer.objects:
                if not any(ign in o.name.lower() for ign in ['metarig', 'backup']):
                    armature = o
                    break
    if armature is None:
        for o in bpy.data.objects:
            if o.type == 'ARMATURE':
                armature = o
                break
    if armature is None:
        raise Exception("No armature found to attach the face rig to.")

    names = [HEAD_BONE_NAME] if HEAD_BONE_NAME else []
    names += HEAD_CANDIDATES
    head_name = None
    for cand in names:
        if cand and cand in armature.data.bones:
            head_name = cand
            break
    if head_name is None:
        for b in armature.data.bones:
            if 'head' in b.name.lower() or 'spine.006' in b.name.lower():
                head_name = b.name
                break
    if head_name is None and len(armature.data.bones) > 0:
        head_name = armature.data.bones[0].name
    return armature, head_name


def find_eye_bone_pair(armature):
    if not armature or not hasattr(armature, "data") or not armature.data:
        return None, None
    candidates_L = ("eye.L", "DEF-eye.L", "Skn_L_Eye", "+EyeBone L A01", "+EyeBone L A02", "EYE_L", "Eye_L", "PT_L_Eye", "Bn_Eye_L")
    candidates_R = ("eye.R", "DEF-eye.R", "Skn_R_Eye", "+EyeBone R A01", "+EyeBone R A02", "EYE_R", "Eye_R", "PT_R_Eye", "Bn_Eye_R")
    eye_L = next((c for c in candidates_L if c in armature.data.bones), None)
    eye_R = next((c for c in candidates_R if c in armature.data.bones), None)
    if not eye_L:
        for b in armature.data.bones:
            low = b.name.lower()
            if "eye" in low and any(s in low for s in ["_l", ".l", "l_", "left"]):
                if not any(ex in low for ex in ["track", "ctrl", "mch", "target", "brow", "lid", "lash", "star", "aim", "iris", "pupil"]):
                    eye_L = b.name
                    break
    if not eye_R:
        for b in armature.data.bones:
            low = b.name.lower()
            if "eye" in low and any(s in low for s in ["_r", ".r", "r_", "right"]):
                if not any(ex in low for ex in ["track", "ctrl", "mch", "target", "brow", "lid", "lash", "star", "aim", "iris", "pupil"]):
                    eye_R = b.name
                    break
    return eye_L, eye_R


def find_eyebrow_bones(armature):
    out = []
    for b in armature.data.bones:
        low = b.name.strip().lower()
        if low.startswith("ctrl-") or low.startswith("face-root") or low.startswith("mch-") or low.startswith("def-") or low.startswith("org-"):
            continue
        if low.startswith("skneyebrow_") or "eyebrow" in low or (low.startswith("ctr_") and "hair" not in low and "eyebrow" in low):
            out.append(b.name)
        elif (low.startswith("ebr_") or low.startswith("ebr ")) and low.endswith("bone"):
            out.append(b.name)
    return out


def find_mouth_bones(armature):
    out = []
    for b in armature.data.bones:
        low = b.name.strip().lower()
        if low.startswith("ctrl-") or low.startswith("face-root") or low.startswith("mch-") or low.startswith("def-") or low.startswith("org-"):
            continue
        if any(k in low for k in ["skn_l_mouth", "skn_r_mouth", "skn_m_mouth", "bdymouth", "bdy_m_mouth", "ptmouth", "pt_m_mouth", "bn_mouthcontrol", "mouth_a", "mouth_b", "mouth_c", "ctr_up_teeth", "ctr_down_teeth"]) or ("mouth" in low and ("_l" in low or "_r" in low or "_m" in low or low.startswith("skn_"))):
            out.append(b.name)
    return out


def find_extra_face_bones(armature):
    out = []
    for b in armature.data.bones:
        low = b.name.strip().lower()
        if low.startswith("ctrl-") or low.startswith("face-root") or low.startswith("mch-") or low.startswith("def-") or low.startswith("org-"):
            continue
        if any(k in low for k in ["highlight", "remimk", "facemark", "face_mark", "mole", "tear"]):
            out.append(b.name)
    return out


def build_mouth_bone_controls(armature, fwd, up, right, face_size):
    controls = []
    mouth_bones = find_mouth_bones(armature)
    if not mouth_bones:
        return controls

    positions = [armature.matrix_world @ armature.data.bones[bn].head_local for bn in mouth_bones]
    center = sum(positions, Vector((0.0, 0.0, 0.0))) / len(positions)
    master_name = 'CTRL-Mouth_Master'
    controls.append({
        'name': master_name,
        'collection': FACERIG_COLLECTION,
        'color': COL_MOUTH,
        'group': 'Face Mouth',
        'head': center + fwd * (face_size * 0.15),
        'widget': 'lipsmaster',
        'lim': face_size * TRAVEL_F,
        'free': ('X', 'Y', 'Z'),
        'range': 'both',
        'shape_scale': Vector((face_size * 0.08,) * 3),
        'kind': 'master',
        'drivers': []
    })

    for bn in mouth_bones:
        head_world = armature.matrix_world @ armature.data.bones[bn].head_local
        front = center + (head_world - center) * 0.85 + fwd * (face_size * 0.12)
        ctrl_name = 'CTRL-' + bn.strip().replace(' ', '_')
        low = bn.lower()
        if '_l' in low or '.l' in low:
            wgt = 'lip00.L'
        elif '_r' in low or '.r' in low:
            wgt = 'lip00.R'
        else:
            wgt = 'eyeblink'
        controls.append({
            'name': ctrl_name,
            'collection': FACERIG_COLLECTION,
            'color': COL_CORNER,
            'group': 'Face Mouth',
            'head': front,
            'widget': wgt,
            'lim': face_size * TRAVEL_F,
            'free': ('X', 'Y', 'Z'),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.035,) * 3),
            'kind': 'fk',
            'target_bone': bn,
            'parent': master_name,
            'hook_name': ctrl_name + '_Hook',
            'hook_head': head_world,
            'drivers': []
        })
    return controls


def build_extra_face_bone_controls(armature, fwd, up, right, face_size):
    controls = []
    extra_bones = find_extra_face_bones(armature)
    for bn in extra_bones:
        head_world = armature.matrix_world @ armature.data.bones[bn].head_local
        front = head_world + fwd * (face_size * 0.10)
        ctrl_name = 'CTRL-' + bn.strip().replace(' ', '_')
        controls.append({
            'name': ctrl_name,
            'collection': FACERIG_COLLECTION,
            'color': COL_EYEAIM,
            'group': 'Face Extra',
            'head': front,
            'widget': 'ring' if 'highlight' in bn.lower() else 'diamond',
            'lim': face_size * TRAVEL_F,
            'free': ('X', 'Y', 'Z'),
            'range': 'both',
            'shape_scale': Vector((face_size * 0.030,) * 3),
            'kind': 'fk',
            'target_bone': bn,
            'hook_name': ctrl_name + '_Hook',
            'hook_head': head_world,
            'drivers': []
        })
    return controls


def eyebrow_side(name):
    low = name.strip().lower()
    if low.endswith(".r") or low.endswith("_r") or "_r_" in low or "_r " in low or ".r " in low:
        return 'R'
    if low.endswith(".l") or low.endswith("_l") or "_l_" in low or "_l " in low or ".l " in low:
        return 'L'
    return 'C'


def is_eyebrow_key(name):
    low = name.strip().lower()
    return (low.startswith("fac_ebr_") or low.startswith("ebr_")
            or low.startswith("ebr ") or "_ebr_" in low or "eyebrow" in low)


def skneyebrow_pos(armature, seg, side):
    for b in armature.data.bones:
        low = b.name.strip().lower()
        if ("skneyebrow_" + seg) in low or ("eyebrow_" + seg) in low:
            if eyebrow_side(b.name) == side:
                return armature.matrix_world @ b.head_local
    return None


def build_brow_shapekey_controls(mesh_obj, armature, fwd, up, face_size):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return []
    kb = mesh_obj.data.shape_keys.key_blocks
    right = up.cross(fwd).normalized()
    OFFSET = face_size * OFFSET_F

    def place_s(feature, h=0.0, v=0.0):
        return feature + fwd * OFFSET + right * h + up * v

    ebkeys = [sk.name for sk in kb if is_eyebrow_key(sk.name)]

    out = []

    brow_c = feature_centroid(mesh_obj, ebkeys)
    seg_positions = [skneyebrow_pos(armature, s, sd)
                     for s in ("01", "02", "03") for sd in ("L", "R")]
    seg_positions = [p for p in seg_positions if p is not None]
    if brow_c is None and seg_positions:
        brow_c = sum(seg_positions, Vector()) / len(seg_positions)
    if brow_c is None:
        bb = [mesh_obj.matrix_world @ Vector(c) for c in mesh_obj.bound_box]
        brow_c = sum(bb, Vector()) / len(bb) + up * face_size * 0.12

    eyeL = feature_centroid(mesh_obj, ["Fac_Eye_L_Wink", "Fac_Eye_L_Open"])\
           or feature_centroid(mesh_obj, ["Fac_Eye_Close"], side='L')
    eyeR = feature_centroid(mesh_obj, ["Fac_Eye_R_Wink", "Fac_Eye_R_Open"])\
           or feature_centroid(mesh_obj, ["Fac_Eye_Close"], side='R')
    if eyeL is not None and eyeR is not None:
        eyeC = (eyeL + eyeR) * 0.5
    else:
        eyeC = brow_c

    kbnames = {sk.name for sk in kb}

    perside = any(("Fac_Ebr_%s_%s" % (s, k)) in kbnames
                  for s in ("L", "R") for k in ("Angry", "Down", "Sad"))
    if perside:
        for side, xoff in (('R', 1.0), ('L', -1.0)):
            lr = ("Fac_Ebr_%s_Sad" % side, "Fac_Ebr_%s_Angry" % side) if FLIP_HORIZONTAL \
                else ("Fac_Ebr_%s_Angry" % side, "Fac_Ebr_%s_Sad" % side)
            drv = []
            if ("Fac_Ebr_%s_Up" % side) in kbnames:
                drv.append({'key': "Fac_Ebr_%s_Up" % side, 'axis': 'Z', 'dir': +1})
            if ("Fac_Ebr_%s_Down" % side) in kbnames:
                drv.append({'key': "Fac_Ebr_%s_Down" % side, 'axis': 'Z', 'dir': -1})
            if lr[0] in kbnames:
                drv.append({'key': lr[0], 'axis': 'X', 'dir': +1})
            if lr[1] in kbnames:
                drv.append({'key': lr[1], 'axis': 'X', 'dir': -1})
            if drv:
                out.append({'name': 'CTRL-Eyebrow-Viseme-Pad.%s' % side,
                            'collection': FACERIG_COLLECTION, 'color': COL_EBRMASTER,
                            'group': 'Face Eyebrow',
                            'head': place_s(brow_c, h=xoff * face_size * 0.06),
                            'widget': 'pad', 'lim': face_size * EMOTE_LIM_F,
                            'free': ('X', 'Z'), 'range': 'both',
                            'shape_scale': Vector((face_size * 0.09, 1.0, face_size * 0.025)),
                            'shape_rotation': (0.0, -0.15 if side == 'L' else 0.15,
                                               0.43825 if side == 'L' else -0.43825),
                            'drivers': drv})
            relaxk = "Fac_Ebr_%s_Relax" % side
            if relaxk in kbnames:
                if side == 'L':
                    brow_range = 'neg'
                    brow_dir = -1
                else:
                    brow_range = 'pos'
                    brow_dir = +1
                out.append({'name': 'CTRL-Brow-%s_Up' % side,
                            'collection': FACERIG_COLLECTION, 'color': COL_BROW,
                            'group': 'Face Eyebrow',
                            'head': place_s(brow_c, h=xoff * face_size * 0.03),
                            'widget': 'eyeblink', 'lim': face_size * EMOTE_LIM_F,
                            'free': ('X',), 'range': brow_range,
                            'shape_scale': Vector((face_size * 0.028, face_size * 0.05, face_size * 0.028)),
                            'drivers': [{'key': relaxk, 'axis': 'X', 'dir': brow_dir}]})
        return out

    ebr_lr = ("Fac_Ebr_Angry", "Fac_Ebr_Sad") if FLIP_HORIZONTAL else ("Fac_Ebr_Sad", "Fac_Ebr_Angry")
    ebr_drv = []
    if "Fac_Ebr_Relax" in kbnames: ebr_drv.append({'key': "Fac_Ebr_Relax", 'axis': 'Z', 'dir': +1})
    if "Fac_Ebr_Down" in kbnames: ebr_drv.append({'key': "Fac_Ebr_Down", 'axis': 'Z', 'dir': -1})
    if ebr_lr[0] in kbnames: ebr_drv.append({'key': ebr_lr[0], 'axis': 'X', 'dir': +1})
    if ebr_lr[1] in kbnames: ebr_drv.append({'key': ebr_lr[1], 'axis': 'X', 'dir': -1})
    if ebr_drv:
        out.append({'name': 'CTRL-Eyebrow-Viseme-Pad',
                    'collection': FACERIG_COLLECTION, 'color': COL_EBRMASTER,
                    'group': 'Face Eyebrow', 'head': place_s(eyeC), 'widget': 'eyeblink',
                    'lim': face_size * EMOTE_LIM_F, 'free': ('X', 'Z'), 'range': 'both',
                    'shape_scale': Vector((face_size * 0.0455, face_size * 0.05, face_size * 0.0416)),
                    'drivers': ebr_drv})

    for side, xoff in (('R', -1.0), ('L', 1.0)):
        key = "Fac_Ebr_%s_Up" % side
        if key in kbnames:
            if side == 'L':
                brow_range = 'neg'
                brow_dir = -1
            else:
                brow_range = 'pos'
                brow_dir = +1
            out.append({'name': 'CTRL-Brow-%s_Up' % side,
                        'collection': FACERIG_COLLECTION, 'color': COL_BROW,
                        'group': 'Face Eyebrow',
                        'head': place_s(eyeC, h=xoff * face_size * 0.03), 'widget': 'eyeblink',
                        'lim': face_size * EMOTE_LIM_F, 'free': ('X',), 'range': brow_range,
                        'shape_scale': Vector((face_size * 0.028, face_size * 0.05, face_size * 0.028)),
                        'drivers': [{'key': key, 'axis': 'X', 'dir': brow_dir}]})
    return out


def feature_centroid(mesh_obj, key_names, side=None):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return None
    kb = mesh_obj.data.shape_keys.key_blocks
    basis = kb.get("Basis")
    if basis is None:
        return None
    key = None
    for kn in key_names:
        if kn in kb:
            key = kb[kn]
            break
    if key is None:
        return None

    n = len(basis.data)
    deltas = [0.0] * n
    maxd = 0.0
    for i in range(n):
        d = (key.data[i].co - basis.data[i].co).length
        deltas[i] = d
        if d > maxd:
            maxd = d
    if maxd <= 1e-9:
        return None

    thr = maxd * 0.30
    idx = [i for i in range(n) if deltas[i] >= thr]
    if not idx:
        return None

    if side in ('L', 'R'):
        xs = sorted(basis.data[i].co.x for i in idx)
        median_x = xs[len(xs) // 2]
        hi = [i for i in idx if basis.data[i].co.x >= median_x]
        lo = [i for i in idx if basis.data[i].co.x < median_x]
        pick = hi if (side == 'L') != FLIP_EYE_LR else lo
        idx = pick or idx

    c = Vector((0.0, 0.0, 0.0))
    for i in idx:
        c += basis.data[i].co
    c /= len(idx)
    return mesh_obj.matrix_world @ c


def feature_extent(mesh_obj, key_names, right, up):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return None
    kb = mesh_obj.data.shape_keys.key_blocks
    basis = kb.get("Basis")
    if basis is None:
        return None
    key = None
    for kn in key_names:
        if kn in kb:
            key = kb[kn]
            break
    if key is None:
        return None
    n = len(basis.data)
    deltas = [(key.data[i].co - basis.data[i].co).length for i in range(n)]
    maxd = max(deltas) if deltas else 0.0
    if maxd <= 1e-9:
        return None
    thr = maxd * 0.30
    mw = mesh_obj.matrix_world
    pts = [mw @ basis.data[i].co for i in range(n) if deltas[i] >= thr]
    if not pts:
        return None
    rs = [p.dot(right) for p in pts]
    us = [p.dot(up) for p in pts]
    return (max(rs) - min(rs), max(us) - min(us))


def feature_extreme(mesh_obj, key_names, axis):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return None
    kb = mesh_obj.data.shape_keys.key_blocks
    basis = kb.get("Basis")
    if basis is None:
        return None
    key = None
    for kn in key_names:
        if kn in kb:
            key = kb[kn]
            break
    if key is None:
        return None
    n = len(basis.data)
    deltas = [(key.data[i].co - basis.data[i].co).length for i in range(n)]
    maxd = max(deltas) if deltas else 0.0
    if maxd <= 1e-9:
        return None
    thr = maxd * 0.30
    mw = mesh_obj.matrix_world
    pts = [mw @ basis.data[i].co for i in range(n) if deltas[i] >= thr]
    if not pts:
        return None
    pmax = max(pts, key=lambda p: p.dot(axis))
    pmin = min(pts, key=lambda p: p.dot(axis))
    return (pmax, pmin)


def face_frame(mesh_obj, armature=None):
    mw = mesh_obj.matrix_world
    fwd = Vector((0.0, -1.0, 0.0))
    right = Vector((1.0, 0.0, 0.0))
    up = Vector((0.0, 0.0, 1.0))

    face_size = 0.20
    if armature:
        eye_L_name, eye_R_name = find_eye_bone_pair(armature)
        eb_L = armature.data.bones.get(eye_L_name) if eye_L_name else None
        eb_R = armature.data.bones.get(eye_R_name) if eye_R_name else None
        if eb_L and eb_R:
            sep = (eb_L.head_local - eb_R.head_local).length
            if 0.01 < sep < 0.3:
                face_size = sep * 3.5
        else:
            head_b = armature.data.bones.get("Head") or armature.data.bones.get("head") or armature.data.bones.get("spine.006")
            if head_b and 0.05 < head_b.length < 0.5:
                face_size = head_b.length * 1.2
    else:
        corners = [mw @ Vector(c) for c in mesh_obj.bound_box]
        mn = Vector((min(c.x for c in corners), min(c.y for c in corners), min(c.z for c in corners)))
        mx = Vector((max(c.x for c in corners), max(c.y for c in corners), max(c.z for c in corners)))
        bb_size = (mx - mn).length
        if 0.05 < bb_size < 0.45:
            face_size = bb_size

    if face_size < 1e-4 or face_size > 0.35:
        face_size = 0.20
    return fwd, right, up, face_size


def plan_controls(mesh_obj, fwd, right, up, face_size):
    if not mesh_obj or not mesh_obj.data or not mesh_obj.data.shape_keys:
        return []
    keyblock = mesh_obj.data.shape_keys.key_blocks
    OFFSET  = face_size * OFFSET_F
    LIM     = face_size * TRAVEL_F
    SPACING = face_size * SPACING_F

    def place(feature, h=0.0, v=0.0):
        return feature + fwd * OFFSET + right * h + up * v

    def hkey(pos, neg):
        return (pos, neg) if not FLIP_HORIZONTAL else (neg, pos)

    def vkey(pos, neg):
        return (pos, neg) if not FLIP_VERTICAL else (neg, pos)

    controls = []

    MOUTH_KEYS = ["Fac_Mth_Ii", "Fac_Mth_Ee", "Fac_Mth_Triangle",
                  "Fac_Mth_Left", "Fac_Mth_Right", "Fac_Mth_Up", "Fac_Mth_Down",
                  "Fac_Mth_L_Up", "Fac_Mth_R_Up", "Fac_Mth_L_Out", "Fac_Mth_R_Out",
                  "Fac_Mth_L_In", "Fac_Mth_R_In", "Fac_Mth_Aa1", "Fac_Mth_AaTalk"]

    mouth = feature_centroid(mesh_obj, MOUTH_KEYS)
    eyeL = feature_centroid(mesh_obj, ["Fac_Eye_L_Wink", "Fac_Eye_L_Open"])\
           or feature_centroid(mesh_obj, ["Fac_Eye_Close"], side='L')
    eyeR = feature_centroid(mesh_obj, ["Fac_Eye_R_Wink", "Fac_Eye_R_Open"])\
           or feature_centroid(mesh_obj, ["Fac_Eye_Close"], side='R')
    brow = feature_centroid(mesh_obj, ["Fac_Ebr_Up", "Fac_Ebr_Down", "Fac_Ebr_Angry"]
                            + [sk.name for sk in keyblock if is_eyebrow_key(sk.name)])

    fcx = mesh_obj.matrix_world @ (0.125 * sum((Vector(c) for c in mesh_obj.bound_box), Vector()))
    if mouth is None:
        mouth = fcx - up * face_size * 0.12
    if eyeL is None:
        eyeL = fcx + up * face_size * 0.10 + right * face_size * 0.10
    if eyeR is None:
        eyeR = fcx + up * face_size * 0.10 - right * face_size * 0.10
    if brow is None:
        brow = ((eyeL + eyeR) * 0.5) + up * face_size * 0.10
    eyeC = (eyeL + eyeR) * 0.5

    mouth_raise = up * (face_size * MOUTH_RAISE_F)
    mouth = mouth + mouth_raise

    mouth_edges = feature_extreme(mesh_obj, MOUTH_KEYS, right)

    def corner(side):
        cc = feature_centroid(mesh_obj,
            ["Fac_Mth_%s_Up" % side, "Fac_Mth_%s_Down" % side,
             "Fac_Mth_%s_Out" % side, "Fac_Mth_%s_In" % side])
        if mouth_edges is not None:
            pmax, pmin = mouth_edges
            if cc is not None:
                return pmax if (pmax - cc).length <= (pmin - cc).length else pmin
            return pmax if (side == 'L') != FLIP_EYE_LR else pmin
        if cc is not None:
            return cc
        s = 1.0 if (side == 'L') != FLIP_EYE_LR else -1.0
        return mouth + right * (s * face_size * 0.06)

    mouth_ext = feature_extent(mesh_obj, MOUTH_KEYS, right, up)
    if mouth_ext:
        mw2, mh2 = mouth_ext[0] * 0.5 * MOUTH_SHIFT_F, mouth_ext[1] * 0.5 * MOUTH_SHIFT_F
    else:
        mw2, mh2 = LIM * 0.9, LIM * 0.4
    mouth_scale = Vector((max(mw2, LIM * 0.3), LIM, max(mh2, LIM * 0.18)))
    hl = face_size * EYE_HL_F
    corner_scale = Vector((hl, hl, hl))
    tri = face_size * TRI_F
    tri_scale = Vector((tri, tri, tri))
    emote_scale = Vector((face_size * 0.035, LIM, face_size * 0.032))

    lr = hkey("Fac_Mth_Left", "Fac_Mth_Right")
    ud = vkey("Fac_Mth_Up", "Fac_Mth_Down")
    drv = []
    if lr[0] in keyblock: drv.append({'key': lr[0], 'axis': 'X', 'dir': +1})
    if lr[1] in keyblock: drv.append({'key': lr[1], 'axis': 'X', 'dir': -1})
    if ud[0] in keyblock: drv.append({'key': ud[0], 'axis': 'Z', 'dir': +1})
    if ud[1] in keyblock: drv.append({'key': ud[1], 'axis': 'Z', 'dir': -1})
    if drv:
        controls.append({'name': 'CTRL-Mouth-Shift', 'collection': 'Face Main',
                         'color': COL_VISEME, 'group': 'Face Mouth',
                         'head': place(mouth), 'widget': 'lipsmaster', 'lim': LIM,
                         'free': ('X', 'Z'), 'range': 'both', 'shape_scale': Vector((face_size * 0.09,) * 3),
                         'drivers': drv})

    vud = vkey("Fac_Mth_Ii", "Fac_Mth_Aa1")
    vdrv = []
    right_dir = +1 if FLIP_HORIZONTAL else -1
    left_dir = -1 if FLIP_HORIZONTAL else +1
    for k in ("Fac_Mth_L_In", "Fac_Mth_R_In"):
        if k in keyblock: vdrv.append({'key': k, 'axis': 'X', 'dir': right_dir})
    for k in ("Fac_Mth_L_Out", "Fac_Mth_R_Out"):
        if k in keyblock: vdrv.append({'key': k, 'axis': 'X', 'dir': left_dir})
    if vud[0] in keyblock: vdrv.append({'key': vud[0], 'axis': 'Z', 'dir': +1})
    if vud[1] in keyblock: vdrv.append({'key': vud[1], 'axis': 'Z', 'dir': -1, 'gain': 0.5})
    if vdrv:
        controls.append({'name': 'CTRL-Mouth-Viseme-Pad', 'collection': 'Face Main',
                         'color': COL_MOUTH, 'group': 'Face Visemes',
                         'head': place(mouth), 'widget': 'pad', 'lim': LIM,
                         'free': ('X', 'Z'), 'range': 'both',
                         'shape_scale': mouth_scale * 1.25,
                         'drivers': vdrv})

    for side in ('L', 'R'):
        out_name = "Fac_Mth_%s_Out" % side
        in_name = "Fac_Mth_%s_In" % side
        if side == 'R':
            outk = hkey(in_name, out_name)
        else:
            outk = hkey(out_name, in_name)
        udk = vkey("Fac_Mth_%s_Up" % side, "Fac_Mth_%s_Down" % side)
        drv = []
        if outk[0] in keyblock: drv.append({'key': outk[0], 'axis': 'X', 'dir': +1})
        if outk[1] in keyblock: drv.append({'key': outk[1], 'axis': 'X', 'dir': -1})
        if udk[0] in keyblock:  drv.append({'key': udk[0], 'axis': 'Z', 'dir': +1})
        if udk[1] in keyblock:  drv.append({'key': udk[1], 'axis': 'Z', 'dir': -1})
        if drv:
            controls.append({'name': 'CTRL-Mouth-Corner.%s' % side,
                             'collection': 'Face Main', 'color': COL_MOUTH,
                             'group': 'Face Corner', 'head': place(corner(side) + mouth_raise),
                             'widget': 'lip00.%s' % side, 'lim': LIM, 'free': ('X', 'Z'),
                             'range': 'both', 'shape_scale': corner_scale,
                             'drivers': drv})

    visemes = [k for k in ["Fac_Mth_Aa1", "Fac_Mth_Aa2", "Fac_Mth_Aa3Shout",
               "Fac_Mth_AaTalk", "Fac_Mth_Ee", "Fac_Mth_Ii", "Fac_Mth_Oo",
               "Fac_Mth_Uu", "Fac_Mth_UuOo", "Fac_Mth_Laugh", "Fac_Mth_Laugh2",
               "Fac_Mth_Triangle"] if k in keyblock]
    if visemes:
        n = len(visemes)
        for i, k in enumerate(visemes):
            h = (i - (n - 1) / 2.0) * SPACING
            controls.append({'name': 'CTRL-%s' % k[4:], 'collection': 'Face Visemes',
                             'color': COL_VISEME, 'group': 'Face Visemes',
                             'head': place(mouth, h=h, v=-face_size * 0.16),
                             'widget': 'eyeblink', 'lim': LIM, 'free': ('Z',),
                             'range': 'pos', 'shape_scale': Vector((face_size * 0.045,) * 3),
                             'drivers': [{'key': k, 'axis': 'Z', 'dir': +1}]})

    for side, pos in (('L', eyeL), ('R', eyeR)):
        openk = "Fac_Eye_%s_Open" % side
        winkk = "Fac_Eye_%s_Wink" % side
        if openk in keyblock:
            controls.append({'name': 'CTRL-Eye_Open.%s' % side,
                             'collection': 'Face Main', 'color': COL_EYELID,
                             'group': 'Face Eyelid',
                             'head': place(pos, v=face_size * 0.055),
                             'widget': 'isaac_blink_top', 'lim': LIM, 'free': ('Z',),
                             'range': 'pos', 'shape_scale': tri_scale,
                             'drivers': [{'key': openk, 'axis': 'Z', 'dir': +1}]})
        if winkk in keyblock:
            wdrv = [{'key': winkk, 'axis': 'Z', 'dir': -1}]
            if "Fac_Eye_Angry" in keyblock:
                wdrv.append({'key': "Fac_Eye_Angry", 'axis': 'Z', 'dir': +1})
            controls.append({'name': 'CTRL-Eye_Wink.%s' % side,
                             'collection': 'Face Main', 'color': COL_EYELID,
                             'group': 'Face Eyelid',
                             'head': place(pos, v=-face_size * 0.06),
                             'widget': 'isaac_blink_bot', 'lim': LIM, 'free': ('Z',),
                             'range': 'both', 'shape_scale': tri_scale,
                             'drivers': wdrv})

    lr = hkey("Eye_Left", "Eye_Right")
    ud = vkey("Eye_Up", "Eye_Down")
    drv = []
    if lr[0] in keyblock: drv.append({'key': lr[0], 'axis': 'X', 'dir': +1})
    if lr[1] in keyblock: drv.append({'key': lr[1], 'axis': 'X', 'dir': -1})
    if ud[0] in keyblock: drv.append({'key': ud[0], 'axis': 'Z', 'dir': +1})
    if ud[1] in keyblock: drv.append({'key': ud[1], 'axis': 'Z', 'dir': -1})
    if drv:
        controls.append({'name': 'CTRL-Eye-Aim', 'collection': 'Face Main',
                         'color': COL_EYEAIM, 'group': 'Face Eye-Aim',
                         'head': place(eyeC, v=face_size * 0.02), 'widget': 'ring',
                         'lim': LIM, 'free': ('X', 'Z'), 'range': 'both', 'drivers': drv})

    emote_lim = face_size * EMOTE_LIM_F
    emote_spacing = face_size * EMOTE_SPACING_F
    emote_fwd = fwd * (face_size * EMOTE_FWD_F)
    eye_pad_lr = hkey("Fac_Eye_MidDown", "Fac_Eye_LowlidUp")
    eye_pad_drv = []
    if "Fac_Eye_Close" in keyblock: eye_pad_drv.append({'key': "Fac_Eye_Close", 'axis': 'Z', 'dir': +1})
    if "Fac_Eye_HalfClose" in keyblock: eye_pad_drv.append({'key': "Fac_Eye_HalfClose", 'axis': 'Z', 'dir': -1})
    if eye_pad_lr[0] in keyblock: eye_pad_drv.append({'key': eye_pad_lr[0], 'axis': 'X', 'dir': +1})
    if eye_pad_lr[1] in keyblock: eye_pad_drv.append({'key': eye_pad_lr[1], 'axis': 'X', 'dir': -1})
    if eye_pad_drv:
        controls.append({'name': 'CTRL-Eye-Viseme-Pad', 'collection': 'Face Eyes',
                         'color': COL_EYEMOTE, 'group': 'Face Eye-Emote',
                         'head': place(eyeC) + emote_fwd, 'widget': 'eyeblink',
                         'lim': emote_lim, 'free': ('X', 'Z'), 'range': 'both',
                         'shape_scale': emote_scale * 1.3, 'drivers': eye_pad_drv})
    if "Fac_Eye_MidUp" in keyblock:
        controls.append({'name': 'CTRL-Eye_MidUp', 'collection': 'Face Eyes',
                         'color': COL_EYEMOTE, 'group': 'Face Eye-Emote',
                         'head': place(eyeC, v=emote_spacing) + emote_fwd, 'widget': 'eyeblink',
                         'lim': emote_lim, 'free': ('Z',), 'range': 'both',
                         'shape_scale': emote_scale,
                         'drivers': [{'key': "Fac_Eye_MidUp", 'axis': 'Z', 'dir': +1}]})
    if "Fac_Eye_Sad" in keyblock:
        controls.append({'name': 'CTRL-Eye_Sad', 'collection': 'Face Eyes',
                         'color': COL_EYEMOTE, 'group': 'Face Eye-Emote',
                         'head': place(eyeC, v=-emote_spacing) + emote_fwd, 'widget': 'eyeblink',
                         'lim': emote_lim, 'free': ('Z',), 'range': 'pos',
                         'shape_scale': emote_scale,
                         'drivers': [{'key': "Fac_Eye_Sad", 'axis': 'Z', 'dir': +1}]})
    if "O_O" in keyblock:
        controls.append({'name': 'CTRL-O_O', 'collection': 'Face Eyes',
                         'color': COL_EYEMOTE, 'group': 'Face Eye-Emote',
                         'head': place(eyeC, h=face_size * 0.14) + emote_fwd, 'widget': 'eyeblink',
                         'lim': emote_lim, 'free': ('Z',), 'range': 'pos',
                         'shape_scale': emote_scale,
                         'drivers': [{'key': "O_O", 'axis': 'Z', 'dir': +1}]})

    for c in controls:
        c['collection'] = FACERIG_COLLECTION
    return controls


def get_widget_collection():
    for name_candidate in ("WGTS", "wgt"):
        coll = bpy.data.collections.get(name_candidate)
        if coll:
            return coll

    for c in bpy.data.collections:
        c_low = c.name.lower()
        if "wgts" in c_low or "wgt" in c_low:
            return c

    coll = bpy.data.collections.get("WGTS")
    if not coll:
        coll = bpy.data.collections.new("WGTS")
    return coll


def finalize_widget_collection(wgt_coll):
    if not wgt_coll:
        return

    try:
        wgt_coll.hide_viewport = True
    except Exception:
        pass

    main_wgts = None
    for name_candidate in ("WGTS", "wgt"):
        c = bpy.data.collections.get(name_candidate)
        if c and c != wgt_coll:
            main_wgts = c
            break

    if main_wgts:
        if wgt_coll.name not in main_wgts.children:
            try:
                main_wgts.children.link(wgt_coll)
            except Exception:
                pass
        if wgt_coll.name in bpy.context.scene.collection.children:
            try:
                bpy.context.scene.collection.children.unlink(wgt_coll)
            except Exception:
                pass
    else:
        if wgt_coll.name in bpy.context.scene.collection.children:
            try:
                bpy.context.scene.collection.children.unlink(wgt_coll)
            except Exception:
                pass


EYE_MASTER_VERTS = [[-0.40348, 0.0, 0.32632], [-0.28803, 0.0, 0.29715], [-0.21194, 0.0, 0.24992], [0.0, 0.0, -0.19968], [0.0, 0.0, 0.19968], [-0.21194, 0.0, -0.24992], [-0.28803, 0.0, -0.29715], [-0.40348, 0.0, -0.32632], [-0.53133, 0.0, -0.31136], [-0.64684, 0.0, -0.24534], [-0.72806, 0.0, -0.13551], [-0.7574, 0.0, 0.0], [-0.72806, 0.0, 0.13551], [-0.64684, 0.0, 0.24534], [-0.53133, 0.0, 0.31136], [-0.11372, 0.0, 0.1995], [-0.11372, 0.0, -0.1995], [-0.03778, 0.0, -0.19971], [-0.03778, 0.0, 0.19971], [-0.16207, 0.0, 0.20723], [-0.16207, 0.0, -0.20723], [-0.34203, 0.0, 0.3157], [-0.24529, 0.0, 0.2746], [-0.0037, 0.0, -0.19969], [-0.07706, 0.0, 0.1997], [-0.24529, 0.0, -0.2746], [-0.34203, 0.0, -0.3157], [-0.4676, 0.0, -0.32523], [-0.59202, 0.0, -0.28451], [-0.693, 0.0, -0.19498], [-0.74991, 0.0, -0.06945], [-0.74991, 0.0, 0.06945], [-0.693, 0.0, 0.19498], [-0.59202, 0.0, 0.28451], [-0.4676, 0.0, 0.32523], [-0.18492, 0.0, 0.22542], [-0.07706, 0.0, -0.1997], [-0.0037, 0.0, 0.19969], [-0.14053, 0.0, -0.20011], [-0.14053, 0.0, 0.20011], [-0.18492, 0.0, -0.22542], [0.40348, 0.0, 0.32632], [0.28803, 0.0, 0.29715], [0.21194, 0.0, 0.24992], [-0.0, 0.0, -0.19968], [-0.0, 0.0, 0.19968], [0.21194, 0.0, -0.24992], [0.28803, 0.0, -0.29715], [0.40348, 0.0, -0.32632], [0.53133, 0.0, -0.31136], [0.64684, 0.0, -0.24534], [0.72806, 0.0, -0.13551], [0.7574, 0.0, 0.0], [0.72806, 0.0, 0.13551], [0.64684, 0.0, 0.24534], [0.53133, 0.0, 0.31136], [0.11372, 0.0, 0.1995], [0.11372, 0.0, -0.1995], [0.03778, 0.0, -0.19971], [0.03778, 0.0, 0.19971], [0.16207, 0.0, 0.20723], [0.16207, 0.0, -0.20723], [0.34203, 0.0, 0.3157], [0.24529, 0.0, 0.2746], [0.0037, 0.0, -0.19969], [0.07706, 0.0, 0.1997], [0.24529, 0.0, -0.2746], [0.34203, 0.0, -0.3157], [0.4676, 0.0, -0.32523], [0.59202, 0.0, -0.28451], [0.693, 0.0, -0.19498], [0.74991, 0.0, -0.06945], [0.74991, 0.0, 0.06945], [0.693, 0.0, 0.19498], [0.59202, 0.0, 0.28451], [0.4676, 0.0, 0.32523], [0.18492, 0.0, 0.22542], [0.07706, 0.0, -0.1997], [0.0037, 0.0, 0.19969], [0.14053, 0.0, -0.20011], [0.14053, 0.0, 0.20011], [0.18492, 0.0, -0.22542]]
EYE_MASTER_EDGES = [[1, 21], [0, 21], [2, 22], [1, 22], [17, 23], [3, 23], [18, 24], [15, 24], [6, 25], [5, 25], [7, 26], [6, 26], [8, 27], [7, 27], [9, 28], [8, 28], [10, 29], [9, 29], [11, 30], [10, 30], [12, 31], [11, 31], [13, 32], [12, 32], [14, 33], [13, 33], [0, 34], [14, 34], [19, 35], [2, 35], [16, 36], [17, 36], [4, 37], [18, 37], [20, 38], [16, 38], [15, 39], [19, 39], [5, 40], [20, 40], [42, 62], [41, 62], [43, 63], [42, 63], [58, 64], [44, 64], [59, 65], [56, 65], [47, 66], [46, 66], [48, 67], [47, 67], [49, 68], [48, 68], [50, 69], [49, 69], [51, 70], [50, 70], [52, 71], [51, 71], [53, 72], [52, 72], [54, 73], [53, 73], [55, 74], [54, 74], [41, 75], [55, 75], [60, 76], [43, 76], [57, 77], [58, 77], [45, 78], [59, 78], [61, 79], [57, 79], [56, 80], [60, 80], [46, 81], [61, 81]]
EYE_CIRCLE_VERTS = [[0.0, 0.0, 0.5], [-0.09755, 0.0, 0.49039], [-0.19134, 0.0, 0.46194], [-0.27779, 0.0, 0.41574], [-0.35355, 0.0, 0.35355], [-0.41574, 0.0, 0.27779], [-0.46194, 0.0, 0.19134], [-0.49039, 0.0, 0.09755], [-0.5, 0.0, -0.0], [-0.49039, 0.0, -0.09755], [-0.46194, 0.0, -0.19134], [-0.41574, 0.0, -0.27779], [-0.35355, 0.0, -0.35355], [-0.27779, 0.0, -0.41574], [-0.19134, 0.0, -0.46194], [-0.09755, 0.0, -0.49039], [0.0, 0.0, -0.5], [0.09755, 0.0, -0.49039], [0.19134, 0.0, -0.46194], [0.27779, 0.0, -0.41574], [0.35355, 0.0, -0.35355], [0.41574, 0.0, -0.27779], [0.46194, 0.0, -0.19134], [0.49039, 0.0, -0.09755], [0.5, 0.0, 0.0], [0.49039, 0.0, 0.09755], [0.46194, 0.0, 0.19134], [0.41573, 0.0, 0.27779], [0.35355, 0.0, 0.35355], [0.27779, 0.0, 0.41574], [0.19134, 0.0, 0.46194], [0.09754, 0.0, 0.49039]]
EYE_CIRCLE_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [29, 30], [30, 31], [0, 31]]

LIPS_MASTER_VERTS = [[0.0, 0.19233, 0.11033], [0.13499, 0.18609, 0.10769], [0.2648, 0.15671, 0.10386], [0.38443, 0.10515, 0.09906], [0.48929, 0.03669, 0.09225], [0.57534, -0.04048, 0.08167], [0.63928, -0.11576, 0.0649], [0.67866, -0.17692, 0.03916], [0.69195, -0.21104, 0.00157], [0.67866, -0.20938, -0.04918], [0.63928, -0.17943, -0.10838], [0.57534, -0.13291, -0.1699], [0.48929, -0.08096, -0.22793], [0.38443, -0.03318, -0.27744], [0.2648, 0.00301, -0.31449], [0.13499, 0.02292, -0.33643], [-0.0, 0.02596, -0.34248], [-0.135, 0.02292, -0.33643], [-0.2648, 0.00301, -0.31449], [-0.38443, -0.03318, -0.27744], [-0.48929, -0.08096, -0.22793], [-0.57534, -0.13291, -0.1699], [-0.63928, -0.17943, -0.10838], [-0.67866, -0.20938, -0.04917], [-0.69195, -0.21104, 0.00157], [-0.67866, -0.17692, 0.03916], [-0.63928, -0.11576, 0.0649], [-0.57534, -0.04048, 0.08167], [-0.48928, 0.03669, 0.09225], [-0.38442, 0.10515, 0.09906], [-0.26479, 0.15671, 0.10386], [-0.13499, 0.1861, 0.10769]]
LIPS_MASTER_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [29, 30], [30, 31], [0, 31]]
EYEBLINK_VERTS = [[-0.42167, 0.0, -0.22917], [0.42167, 0.0, -0.22917], [-0.42167, -0.0, 0.22917], [0.42167, -0.0, 0.22917], [0.46, 0.0, -0.125], [0.46, 0.0, -0.0], [0.46, -0.0, 0.125], [-0.23, 0.0, -0.25], [-0.0, 0.0, -0.25], [0.23, 0.0, -0.25], [-0.46, 0.0, -0.125], [-0.46, -0.0, 0.0], [-0.46, -0.0, 0.125], [-0.23, -0.0, 0.25], [0.0, -0.0, 0.25], [0.23, -0.0, 0.25], [0.4594, -0.0, 0.15592], [0.45521, -0.0, 0.1849], [0.44383, -0.0, 0.20996], [0.2869, 0.0, -0.24968], [0.34021, 0.0, -0.2474], [0.38633, 0.0, -0.24121], [-0.4594, -0.0, 0.15592], [-0.45521, -0.0, 0.1849], [-0.44383, -0.0, 0.20996], [0.2869, -0.0, 0.24968], [0.34021, -0.0, 0.2474], [0.38633, -0.0, 0.24121], [0.44383, 0.0, -0.20996], [0.45521, 0.0, -0.1849], [0.4594, 0.0, -0.15592], [0.46, 0.0, -0.0625], [0.46, -0.0, 0.0625], [-0.38633, 0.0, -0.24121], [-0.34021, 0.0, -0.2474], [-0.2869, 0.0, -0.24967], [-0.115, 0.0, -0.25], [0.115, 0.0, -0.25], [-0.44383, 0.0, -0.20996], [-0.45521, 0.0, -0.1849], [-0.4594, 0.0, -0.15592], [-0.46, 0.0, -0.0625], [-0.46, -0.0, 0.0625], [-0.38633, -0.0, 0.24121], [-0.34021, -0.0, 0.2474], [-0.2869, -0.0, 0.24967], [-0.115, -0.0, 0.25], [0.115, -0.0, 0.25], [-0.39485, -0.00338, -0.20822], [0.39485, 0.00338, -0.20822], [-0.39485, -0.00338, 0.20822], [0.39485, 0.00338, 0.20822], [0.43075, 0.00369, -0.11357], [0.43075, 0.00369, 0.11358], [-0.43075, -0.00369, -0.11357], [-0.43075, -0.00369, 0.11358], [0.43018, 0.00368, 0.14167], [0.42626, 0.00365, 0.168], [0.4156, 0.00356, 0.19077], [0.31857, 0.00273, -0.22478], [0.36176, 0.0031, -0.21916], [-0.43018, -0.00368, 0.14167], [-0.42626, -0.00365, 0.168], [-0.4156, -0.00356, 0.19077], [0.31857, 0.00273, 0.22479], [0.36176, 0.0031, 0.21917], [0.4156, 0.00356, -0.19077], [0.42626, 0.00365, -0.16799], [0.43018, 0.00368, -0.14167], [-0.36176, -0.0031, -0.21916], [-0.31857, -0.00273, -0.22478], [-0.4156, -0.00356, -0.19077], [-0.42626, -0.00365, -0.16799], [-0.43018, -0.00368, -0.14167], [-0.36176, -0.0031, 0.21917], [-0.31857, -0.00273, 0.22479]]
EYEBLINK_EDGES = [[6, 16], [16, 17], [17, 18], [3, 18], [9, 19], [19, 20], [20, 21], [1, 21], [12, 22], [22, 23], [23, 24], [2, 24], [15, 25], [25, 26], [26, 27], [3, 27], [1, 28], [28, 29], [29, 30], [4, 30], [4, 31], [5, 31], [5, 32], [6, 32], [0, 33], [33, 34], [34, 35], [7, 35], [7, 36], [8, 36], [8, 37], [9, 37], [0, 38], [38, 39], [39, 40], [10, 40], [10, 41], [11, 41], [11, 42], [12, 42], [2, 43], [43, 44], [44, 45], [13, 45], [13, 46], [14, 46], [14, 47], [15, 47], [53, 56], [56, 57], [57, 58], [51, 58], [59, 60], [49, 60], [55, 61], [61, 62], [62, 63], [50, 63], [64, 65], [51, 65], [49, 66], [66, 67], [67, 68], [52, 68], [48, 69], [69, 70], [48, 71], [71, 72], [72, 73], [54, 73], [50, 74], [74, 75]]
EBROW_MASTER_L_VERTS = [[-1.70158, 0.5298, -0.39297], [1.54361, -0.5298, -0.81676], [-1.54361, 0.5298, 0.81676], [1.70158, -0.5298, 0.39297]]
EBROW_MASTER_L_EDGES = [[2, 3], [0, 1], [0, 2], [1, 3]]
EBROW_MASTER_R_VERTS = [[-1.70158, 0.5298, -0.39297], [1.54361, -0.5298, -0.81676], [-1.54361, 0.5298, 0.81676], [1.70158, -0.5298, 0.39297]]
EBROW_MASTER_R_EDGES = [[2, 3], [0, 1], [0, 2], [1, 3]]
EBROW_TWEAK_VERTS = [[-0.8305, 0.4681, 0.52663], [0.8305, 0.4681, 0.52663], [-0.8305, -0.4681, 0.52663], [0.8305, -0.4681, 0.52663], [-0.8305, 0.4681, -0.59077], [0.8305, 0.4681, -0.59077], [-0.8305, -0.4681, -0.59077], [0.8305, -0.4681, -0.59077], [1.53746, -0.0, -0.03207], [0.8305, -0.0, 0.00804], [0.8305, -0.0, -0.07218], [1.40016, -0.0, 0.08826], [1.40016, -0.0, -0.15241], [1.40016, -0.0, 0.00804], [1.40016, -0.0, -0.07218], [0.0, 0.0, -1.29822], [0.04011, 0.0, -0.59126], [-0.04011, 0.0, -0.59126], [0.12033, 0.0, -1.16092], [-0.12033, 0.0, -1.16092], [0.04011, 0.0, -1.16092], [-0.04011, 0.0, -1.16092], [-1.53746, -0.0, -0.03207], [-0.8305, -0.0, 0.00804], [-0.8305, -0.0, -0.07218], [-1.40016, -0.0, 0.08826], [-1.40016, -0.0, -0.15241], [-1.40016, -0.0, 0.00804], [-1.40016, -0.0, -0.07218]]
EBROW_TWEAK_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]

ARROW_UP_VERTS = [[-0.55, 0.0, -0.45], [0.55, 0.0, -0.45], [0.0, 0.0, 0.6]]
ARROW_UP_EDGES = [[0, 1], [1, 2], [2, 0]]
ARROW_DOWN_VERTS = [[-0.55, 0.0, 0.45], [0.55, 0.0, 0.45], [0.0, 0.0, -0.6]]
ARROW_DOWN_EDGES = [[0, 1], [1, 2], [2, 0]]
EBROW_TWEAK2_L_VERTS = [[0.57815, -0.19227, -0.02967], [0.55841, -0.19624, -0.14754], [0.51721, -0.19268, -0.25975], [0.45613, -0.18171, -0.36197], [0.37753, -0.16376, -0.45028], [0.28442, -0.13952, -0.52129], [0.18038, -0.10992, -0.57226], [-0.04424, -0.03933, -0.60712], [-0.15618, -0.00107, -0.58967], [-0.26212, 0.03724, -0.54955], [-0.35798, 0.07411, -0.48832], [-0.44009, 0.10814, -0.40832], [-0.50529, 0.13801, -0.31263], [-0.55107, 0.16258, -0.20492], [-0.57567, 0.1809, -0.08934], [-0.57815, 0.19227, 0.02967], [-0.55841, 0.19624, 0.14754], [-0.51721, 0.19268, 0.25975], [-0.45613, 0.18171, 0.36197], [-0.37753, 0.16376, 0.45028], [-0.28442, 0.13952, 0.52129], [-0.18038, 0.10992, 0.57226], [-0.0694, 0.07609, 0.60124], [0.04424, 0.03933, 0.60712], [0.15618, 0.00107, 0.58967], [0.26212, -0.03724, 0.54955], [0.35798, -0.07411, 0.48832], [0.44009, -0.10814, 0.40832], [0.50529, -0.13801, 0.31263], [0.55107, -0.16258, 0.20492], [0.57567, -0.1809, 0.08934], [0.60157, -0.06474, 0.07763], [0.60434, 0.0539, 0.06294], [0.5839, 0.17047, 0.04583], [0.54102, 0.28049, 0.02696], [0.47734, 0.37973, 0.00705], [0.39532, 0.46438, -0.01313], [0.29811, 0.53118, -0.03281], [0.0735, 0.60176, -0.06767], [-0.04527, 0.60283, -0.08151], [-0.16231, 0.58073, -0.09223], [-0.2731, 0.53632, -0.09939], [-0.3734, 0.47129, -0.10274], [-0.45935, 0.38815, -0.10214], [-0.52765, 0.2901, -0.09762], [0.0, 0.0, 0.0], [-0.03111, 0.1873, 0.5797], [0.00838, 0.29132, 0.53587], [0.04754, 0.38414, 0.47146], [0.08488, 0.4622, 0.38892], [0.11896, 0.5225, 0.29144], [0.14846, 0.56272, 0.18276], [0.17226, 0.58132, 0.06706], [0.18945, 0.57757, -0.05122], [0.19934, 0.55163, -0.16753], [0.20158, 0.50449, -0.27741], [0.19608, 0.43796, -0.37662], [0.18303, 0.3546, -0.46136], [0.16296, 0.25762, -0.52837], [0.13662, 0.15073, -0.57508], [0.10503, 0.03805, -0.59968], [0.0694, -0.07609, -0.60124]]
EBROW_TWEAK2_L_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [30, 31], [31, 32], [32, 33], [33, 34], [34, 35], [35, 36], [36, 37], [38, 39], [39, 40], [40, 41], [41, 42], [42, 43], [43, 44], [14, 45], [30, 45], [46, 47], [47, 48], [48, 49], [49, 50], [50, 51], [51, 52], [52, 53], [53, 54], [54, 55], [55, 56], [56, 57], [57, 58], [58, 59], [59, 60], [60, 61], [0, 30], [6, 61], [7, 61], [29, 30], [37, 53], [38, 53], [14, 44], [22, 46], [45, 61], [22, 45]]
EBROW_TWEAK2_R_VERTS = [[-0.57815, -0.19227, -0.02967], [-0.55841, -0.19624, -0.14754], [-0.51721, -0.19268, -0.25975], [-0.45613, -0.18171, -0.36197], [-0.37753, -0.16376, -0.45028], [-0.28442, -0.13952, -0.52129], [-0.18038, -0.10992, -0.57226], [0.04424, -0.03933, -0.60712], [0.15618, -0.00107, -0.58967], [0.26212, 0.03724, -0.54955], [0.35798, 0.07411, -0.48832], [0.44009, 0.10814, -0.40832], [0.50529, 0.13801, -0.31263], [0.55107, 0.16258, -0.20492], [0.57567, 0.1809, -0.08934], [0.57815, 0.19227, 0.02967], [0.55841, 0.19624, 0.14754], [0.51721, 0.19268, 0.25975], [0.45613, 0.18171, 0.36197], [0.37753, 0.16376, 0.45028], [0.28442, 0.13952, 0.52129], [0.18038, 0.10992, 0.57226], [0.0694, 0.07609, 0.60124], [-0.04424, 0.03933, 0.60712], [-0.15618, 0.00107, 0.58967], [-0.26212, -0.03724, 0.54955], [-0.35798, -0.07411, 0.48832], [-0.44009, -0.10814, 0.40832], [-0.50529, -0.13801, 0.31263], [-0.55107, -0.16258, 0.20492], [-0.57567, -0.1809, 0.08934], [-0.60157, -0.06474, 0.07763], [-0.60434, 0.0539, 0.06294], [-0.5839, 0.17047, 0.04583], [-0.54102, 0.28049, 0.02696], [-0.47734, 0.37973, 0.00705], [-0.39532, 0.46438, -0.01313], [-0.29811, 0.53118, -0.03281], [-0.0735, 0.60176, -0.06767], [0.04527, 0.60283, -0.08151], [0.16231, 0.58073, -0.09223], [0.2731, 0.53632, -0.09939], [0.3734, 0.47129, -0.10274], [0.45935, 0.38815, -0.10214], [0.52765, 0.2901, -0.09762], [-0.0, 0.0, 0.0], [0.03111, 0.1873, 0.5797], [-0.00838, 0.29132, 0.53587], [-0.04754, 0.38414, 0.47146], [-0.08488, 0.4622, 0.38892], [-0.11896, 0.5225, 0.29144], [-0.14846, 0.56272, 0.18276], [-0.17226, 0.58132, 0.06706], [-0.18945, 0.57757, -0.05122], [-0.19934, 0.55163, -0.16753], [-0.20158, 0.50449, -0.27741], [-0.19608, 0.43796, -0.37662], [-0.18303, 0.3546, -0.46136], [-0.16296, 0.25762, -0.52837], [-0.13662, 0.15073, -0.57508], [-0.10503, 0.03805, -0.59968], [-0.0694, -0.07609, -0.60124]]
EBROW_TWEAK2_R_EDGES = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13], [13, 14], [14, 15], [15, 16], [16, 17], [17, 18], [18, 19], [19, 20], [20, 21], [21, 22], [22, 23], [23, 24], [24, 25], [25, 26], [26, 27], [27, 28], [28, 29], [30, 31], [31, 32], [32, 33], [33, 34], [34, 35], [35, 36], [36, 37], [38, 39], [39, 40], [40, 41], [41, 42], [42, 43], [43, 44], [14, 45], [30, 45], [46, 47], [47, 48], [48, 49], [49, 50], [50, 51], [51, 52], [52, 53], [53, 54], [54, 55], [55, 56], [56, 57], [57, 58], [58, 59], [59, 60], [60, 61], [0, 30], [6, 61], [7, 61], [29, 30], [37, 53], [38, 53], [14, 44], [22, 46], [45, 61], [22, 45]]

EBROW_TWEAK1_L_VERTS = [[0.44625, -0.70296, -0.70206], [0.25719, -0.49566, 0.93507], [0.73684, 0.18352, -0.78075], [0.54778, 0.39081, 0.85638], [-0.60831, -0.3718, -0.86578], [-0.79738, -0.1645, 0.77135], [-0.31773, 0.51467, -0.94447], [-0.50679, 0.72197, 0.69266], [-0.20527, 0.20139, 1.51067], [-0.08695, 0.10127, 0.81974], [-0.16266, 0.12504, 0.80799], [-0.07608, 0.14859, 1.39297], [-0.30321, 0.21991, 1.35771], [-0.15179, 0.17236, 1.38122], [-0.2275, 0.19614, 1.36946], [-1.22522, 0.38474, -0.19021], [-0.56258, 0.18023, -0.0471], [-0.55345, 0.17022, -0.12617], [-1.10934, 0.35907, -0.05149], [-1.08194, 0.32903, -0.2887], [-1.1002, 0.34906, -0.13056], [-1.09107, 0.33904, -0.20963], [0.14474, -0.18238, -1.52007], [0.10212, -0.10603, -0.81739], [0.02641, -0.08226, -0.82914], [0.24268, -0.2009, -1.36711], [0.01554, -0.12958, -1.40237], [0.16696, -0.17713, -1.37886], [0.09125, -0.15335, -1.39061]]
EBROW_TWEAK1_L_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]
EBROW_TWEAK1_R_VERTS = [[-0.44625, -0.70296, -0.70206], [-0.25719, -0.49566, 0.93507], [-0.73684, 0.18352, -0.78075], [-0.54778, 0.39081, 0.85638], [0.60831, -0.3718, -0.86578], [0.79738, -0.1645, 0.77135], [0.31773, 0.51467, -0.94447], [0.50679, 0.72197, 0.69266], [0.20527, 0.20139, 1.51067], [0.08695, 0.10127, 0.81974], [0.16266, 0.12504, 0.80799], [0.07608, 0.14859, 1.39297], [0.30321, 0.21991, 1.35771], [0.15179, 0.17236, 1.38122], [0.2275, 0.19614, 1.36946], [1.22522, 0.38474, -0.19021], [0.56258, 0.18023, -0.0471], [0.55345, 0.17022, -0.12617], [1.10934, 0.35907, -0.05149], [1.08194, 0.32903, -0.2887], [1.1002, 0.34906, -0.13056], [1.09107, 0.33904, -0.20963], [-0.14474, -0.18238, -1.52007], [-0.10212, -0.10603, -0.81739], [-0.02641, -0.08226, -0.82914], [-0.24268, -0.2009, -1.36711], [-0.01554, -0.12958, -1.40237], [-0.16696, -0.17713, -1.37886], [-0.09125, -0.15335, -1.39061]]
EBROW_TWEAK1_R_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]
EBROW_TWEAK3_L_VERTS = [[-0.66599, -0.3537, -0.87474], [-0.85504, -0.14651, 0.76241], [-0.3755, 0.53281, -0.95339], [-0.56454, 0.73999, 0.68376], [0.3956, -0.68696, -0.70998], [0.20655, -0.47977, 0.92717], [0.68609, 0.19955, -0.78863], [0.49705, 0.40674, 0.84852], [-0.25946, 0.2183, 1.50227], [-0.2171, 0.14207, 0.79955], [-0.14089, 0.11815, 0.81138], [-0.35816, 0.23706, 1.3492], [-0.12951, 0.16528, 1.38469], [-0.28194, 0.21313, 1.36103], [-0.20572, 0.18921, 1.37286], [1.11843, -0.3511, 0.17358], [0.44222, -0.13525, 0.10888], [0.45135, -0.14526, 0.0298], [0.97429, -0.29514, 0.27194], [1.00168, -0.32516, 0.03473], [0.98342, -0.30515, 0.19287], [0.99255, -0.31515, 0.1138], [0.09051, -0.16526, -1.52849], [-0.02806, -0.06511, -0.8376], [0.04816, -0.08904, -0.82577], [-0.03944, -0.11224, -1.41091], [0.18921, -0.18402, -1.37542], [0.03678, -0.13617, -1.39908], [0.11299, -0.16009, -1.38725]]
EBROW_TWEAK3_L_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]
EBROW_TWEAK3_R_VERTS = [[0.66599, -0.3537, -0.87474], [0.85504, -0.14651, 0.76241], [0.3755, 0.53281, -0.95339], [0.56454, 0.73999, 0.68376], [-0.3956, -0.68696, -0.70998], [-0.20655, -0.47977, 0.92717], [-0.68609, 0.19955, -0.78863], [-0.49705, 0.40674, 0.84852], [0.25946, 0.2183, 1.50227], [0.2171, 0.14207, 0.79955], [0.14089, 0.11815, 0.81138], [0.35816, 0.23706, 1.3492], [0.12951, 0.16528, 1.38469], [0.28194, 0.21313, 1.36103], [0.20572, 0.18921, 1.37286], [-1.11843, -0.3511, 0.17358], [-0.44222, -0.13525, 0.10888], [-0.45135, -0.14526, 0.0298], [-0.97429, -0.29514, 0.27194], [-1.00168, -0.32516, 0.03473], [-0.98342, -0.30515, 0.19287], [-0.99255, -0.31515, 0.1138], [-0.09051, -0.16526, -1.52849], [0.02806, -0.06511, -0.8376], [-0.04816, -0.08904, -0.82577], [0.03944, -0.11224, -1.41091], [-0.18921, -0.18402, -1.37542], [-0.03678, -0.13617, -1.39908], [-0.11299, -0.16009, -1.38725]]
EBROW_TWEAK3_R_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]
LIP00_L_VERTS = [[0.25457, -0.31, -0.55], [0.25457, -0.31, 0.55], [0.25457, 0.31, -0.55], [0.25457, 0.31, 0.55], [-0.48543, -0.31, -0.55], [-0.48543, -0.31, 0.55], [-0.48543, 0.31, -0.55], [-0.48543, 0.31, 0.55], [-0.11543, 0.0, 1.01819], [-0.08887, 0.0, 0.55], [-0.142, 0.0, 0.55], [-0.03574, 0.0, 0.92726], [-0.19512, 0.0, 0.92726], [-0.08887, 0.0, 0.92726], [-0.142, 0.0, 0.92726], [-0.95394, -0.0, 0.0], [-0.48575, -0.0, 0.02656], [-0.48575, -0.0, -0.02656], [-0.86301, -0.0, 0.07969], [-0.86301, -0.0, -0.07969], [-0.86301, -0.0, 0.02656], [-0.86301, -0.0, -0.02656], [-0.11543, 0.0, -1.01819], [-0.08887, 0.0, -0.55], [-0.142, 0.0, -0.55], [-0.03574, 0.0, -0.92726], [-0.19512, 0.0, -0.92726], [-0.08887, 0.0, -0.92726], [-0.142, 0.0, -0.92726]]
LIP00_L_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]
LIP00_R_VERTS = [[-0.25457, -0.31, -0.55], [-0.25457, -0.31, 0.55], [-0.25457, 0.31, -0.55], [-0.25457, 0.31, 0.55], [0.48543, -0.31, -0.55], [0.48543, -0.31, 0.55], [0.48543, 0.31, -0.55], [0.48543, 0.31, 0.55], [0.11543, 0.0, 1.01819], [0.08887, 0.0, 0.55], [0.142, 0.0, 0.55], [0.03574, 0.0, 0.92726], [0.19512, 0.0, 0.92726], [0.08887, 0.0, 0.92726], [0.142, 0.0, 0.92726], [0.95394, -0.0, 0.0], [0.48575, -0.0, 0.02656], [0.48575, -0.0, -0.02656], [0.86301, -0.0, 0.07969], [0.86301, -0.0, -0.07969], [0.86301, -0.0, 0.02656], [0.86301, -0.0, -0.02656], [0.11543, 0.0, -1.01819], [0.08887, 0.0, -0.55], [0.142, 0.0, -0.55], [0.03574, 0.0, -0.92726], [0.19512, 0.0, -0.92726], [0.08887, 0.0, -0.92726], [0.142, 0.0, -0.92726]]
LIP00_R_EDGES = [[0, 2], [0, 1], [1, 3], [2, 3], [2, 6], [3, 7], [6, 7], [4, 6], [5, 7], [4, 5], [0, 4], [1, 5], [8, 12], [9, 10], [9, 13], [10, 14], [11, 13], [12, 14], [8, 11], [15, 19], [16, 17], [16, 20], [17, 21], [18, 20], [19, 21], [15, 18], [22, 26], [23, 24], [23, 27], [24, 28], [25, 27], [26, 28], [22, 25]]

ISAAC_BLINK_TOP_VERTS = [[1.0, 0.0, 0.5638], [-1.0, 0.0, 0.5638], [-0.0, 0.0, 0.8878], [-0.0, 0.0, -0.484], [-0.5916, 0.0, -0.0569], [0.5916, 0.0, -0.0569], [0.6306, 0.0, 0.8199], [-0.6306, 0.0, 0.8199], [-1.0, 0.0, 0.5638], [-0.0, 0.0, -0.484], [1.0, 0.0, 0.5638]]
ISAAC_BLINK_TOP_EDGES = [[7, 1], [4, 1], [3, 4], [6, 2], [5, 3], [0, 5], [0, 6], [2, 7], [1, 8], [3, 9], [0, 10]]
ISAAC_BLINK_BOT_VERTS = [[-1.0, 0.0, -0.5638], [1.0, 0.0, -0.5638], [0.0, 0.0, -0.8878], [0.0, 0.0, 0.484], [0.5916, 0.0, 0.0569], [-0.5916, 0.0, 0.0569], [-0.6306, 0.0, -0.8199], [0.6306, 0.0, -0.8199], [1.0, 0.0, -0.5638], [0.0, 0.0, 0.484], [-1.0, 0.0, -0.5638]]
ISAAC_BLINK_BOT_EDGES = [[7, 1], [4, 1], [3, 4], [6, 2], [5, 3], [0, 5], [0, 6], [2, 7], [1, 8], [3, 9], [0, 10]]


def make_widget(kind, coll):
    name = "WGT-Face_" + kind
    obj = bpy.data.objects.get(name)
    if obj:
        return obj
    if kind == 'isaac_blink_top':
        verts = ISAAC_BLINK_TOP_VERTS
        edges = ISAAC_BLINK_TOP_EDGES
    elif kind == 'isaac_blink_bot':
        verts = ISAAC_BLINK_BOT_VERTS
        edges = ISAAC_BLINK_BOT_EDGES
    elif kind == 'ebrowtweak1.L':
        verts = EBROW_TWEAK1_L_VERTS
        edges = EBROW_TWEAK1_L_EDGES
    elif kind == 'ebrowtweak1.R':
        verts = EBROW_TWEAK1_R_VERTS
        edges = EBROW_TWEAK1_R_EDGES
    elif kind == 'ebrowtweak3.L':
        verts = EBROW_TWEAK3_L_VERTS
        edges = EBROW_TWEAK3_L_EDGES
    elif kind == 'ebrowtweak3.R':
        verts = EBROW_TWEAK3_R_VERTS
        edges = EBROW_TWEAK3_R_EDGES
    elif kind == 'lip00.L':
        verts = LIP00_L_VERTS
        edges = LIP00_L_EDGES
    elif kind == 'lip00.R':
        verts = LIP00_R_VERTS
        edges = LIP00_R_EDGES
    elif kind == 'arrow_up':
        verts = ARROW_UP_VERTS
        edges = ARROW_UP_EDGES
    elif kind == 'arrow_down':
        verts = ARROW_DOWN_VERTS
        edges = ARROW_DOWN_EDGES
    elif kind == 'ebrowtweak2.L':
        verts = EBROW_TWEAK2_L_VERTS
        edges = EBROW_TWEAK2_L_EDGES
    elif kind == 'ebrowtweak2.R':
        verts = EBROW_TWEAK2_R_VERTS
        edges = EBROW_TWEAK2_R_EDGES
    elif kind == 'lipsmaster':
        verts = LIPS_MASTER_VERTS
        edges = LIPS_MASTER_EDGES
    elif kind == 'eyeblink':
        verts = EYEBLINK_VERTS
        edges = EYEBLINK_EDGES
    elif kind == 'ebrowmaster.L':
        verts = EBROW_MASTER_L_VERTS
        edges = EBROW_MASTER_L_EDGES
    elif kind == 'ebrowmaster.R':
        verts = EBROW_MASTER_R_VERTS
        edges = EBROW_MASTER_R_EDGES
    elif kind == 'ebrowtweak':
        verts = EBROW_TWEAK_VERTS
        edges = EBROW_TWEAK_EDGES
    elif kind == 'eyemaster':
        verts = EYE_MASTER_VERTS
        edges = EYE_MASTER_EDGES
    elif kind == 'eyecircle':
        verts = EYE_CIRCLE_VERTS
        edges = EYE_CIRCLE_EDGES
    elif kind == 'pad':
        s = 1.0
        verts = [(-s, 0, -s), (s, 0, -s), (s, 0, s), (-s, 0, s)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
    elif kind == 'slider':
        verts = [(0, 0, -1), (0, 0, 1), (-0.35, 0, 1), (0.35, 0, 1)]
        edges = [(0, 1), (2, 3)]
    elif kind == 'ring':
        import math
        verts, edges, N = [], [], 20
        for i in range(N):
            a = 2 * math.pi * i / N
            verts.append((math.cos(a), 0.0, math.sin(a)))
            edges.append((i, (i + 1) % N))
    elif kind == 'triangle':
        verts = [(0, 0, 1), (-0.9, 0, -0.7), (0.9, 0, -0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    elif kind == 'triangle_down':
        verts = [(0, 0, -1), (-0.9, 0, 0.7), (0.9, 0, 0.7)]
        edges = [(0, 1), (1, 2), (2, 0)]
    elif kind == 'diamond':
        verts = [(0, 0, 1), (0.75, 0, 0), (0, 0, -1), (-0.75, 0, 0)]
        edges = [(0, 1), (1, 2), (2, 3), (3, 0)]
    elif kind == 'eyebrow':
        N = 14
        top = []
        bot = []
        for i in range(N + 1):
            x = -1.0 + 2.0 * i / N
            arch = 0.40 * (1.0 - x * x)
            ht = 0.20 * (1.0 - x * x)
            top.append((x, 0.0, arch + ht))
            bot.append((x, 0.0, arch - ht))
        verts = top + [bot[i] for i in range(N - 1, 0, -1)]
        edges = [(i, (i + 1) % len(verts)) for i in range(len(verts))]
    else:
        verts, edges = [(0, 0, 0)], []
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, edges, [])
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return obj


def lighten(rgb, amt):
    return tuple(min(1.0, c + amt) for c in rgb)


def apply_color(armature, pb, group_name, rgb, cache):
    if ver != 3:
        try:
            pb.color.palette = 'CUSTOM'
            cc = pb.color.custom
            cc.normal = rgb
            cc.select = lighten(rgb, 0.25)
            cc.active = lighten(rgb, 0.5)
        except Exception:
            pass
    else:
        grp = cache.get(group_name)
        if grp is None:
            grp = armature.pose.bone_groups.get(group_name)
            if grp is None:
                grp = armature.pose.bone_groups.new(name=group_name)
            grp.color_set = 'CUSTOM'
            grp.colors.normal = rgb
            grp.colors.select = lighten(rgb, 0.25)
            grp.colors.active = lighten(rgb, 0.5)
            cache[group_name] = grp
        pb.bone_group = grp


def is_face_rig_bone(bone_name):
    if bone_name in ("Face-Root", "Facerig Root"):
        return True
    if bone_name.endswith(" Bone"):
        return True
    if bone_name in ("MCH-EyeAim.L", "MCH-EyeAim.R", "MCH-Eyes_Parent"):
        return True
    b_low = bone_name.lower()
    if any(k in b_low for k in ("skirt", "hair", "dress", "cloth", "surfboard", "weapon", "wpn", "garape", "grape", "tail", "spine", "arm", "leg", "hand", "foot", "torso")):
        return False
    if bone_name.startswith("CTRL-Ctr_"):
        return False
    if any(bone_name.startswith(p) for p in (
        "CTRL-Mouth", "CTRL-Eye", "CTRL-Brow", "CTRL-Eyebrow",
        "CTRL-Master-Eyebrow", "CTRL-O_O", "CTRL-Fac_", "CTRL-Face",
        "MCH-EyeAim"
    )):
        return True
    if bone_name.startswith("CTRL-") and any(k in b_low for k in ("eyebrow", "ebrow", "brow", "eye", "mouth", "lip", "jaw", "highlight", "viseme")):
        return True
    return False


def purge_previous(armature):
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones
    for b in list(eb):
        if is_face_rig_bone(b.name):
            eb.remove(b)
    bpy.ops.object.mode_set(mode='OBJECT')
    for o in list(bpy.data.objects):
        if o.name.startswith("WGT-Face_"):
            bpy.data.objects.remove(o, do_unlink=True)
    facerig_col = bpy.data.collections.get("Facerig")
    if facerig_col:
        for parent_col in list(bpy.data.collections):
            if facerig_col.name in parent_col.children:
                try:
                    parent_col.children.unlink(facerig_col)
                except Exception:
                    pass
        if facerig_col.name in bpy.context.scene.collection.children:
            try:
                bpy.context.scene.collection.children.unlink(facerig_col)
            except Exception:
                pass
        for obj in list(facerig_col.objects):
            try:
                bpy.data.objects.remove(obj, do_unlink=True)
            except Exception:
                pass
        try:
            bpy.data.collections.remove(facerig_col)
        except Exception:
            pass
    border = bpy.data.objects.get("Facerig Border")
    if border:
        try:
            bpy.data.objects.remove(border, do_unlink=True)
        except Exception:
            pass



def make_facerig_root_widget(coll):
    name = "WGT-Face_FacerigRoot"
    obj = bpy.data.objects.get(name)
    if obj is not None:
        return obj
    mesh = bpy.data.meshes.new(name)
    w, h = 0.85, 0.60
    verts = [
        (-w, -h, 0.0), (w, -h, 0.0), (w, h, 0.0), (-w, h, 0.0),
        (-w*1.04, -h*1.04, 0.0), (w*1.04, -h*1.04, 0.0), (w*1.04, h*1.04, 0.0), (-w*1.04, h*1.04, 0.0)
    ]
    edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7)
    ]
    mesh.from_pydata(verts, edges, [])
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    if coll is not None and obj.name not in coll.objects:
        coll.objects.link(obj)
    return obj


def setup_face_rig(mesh_obj, controls, armature, head_name, fwd, up, face_size):
    keyblock = mesh_obj.data.shape_keys.key_blocks if (mesh_obj and mesh_obj.data and mesh_obj.data.shape_keys) else None
    amw_inv = armature.matrix_world.inverted()
    bone_len = face_size * BONE_LEN_F

    def to_arm_point(p):
        return amw_inv @ p

    def to_arm_vec(v):
        return amw_inv.to_3x3() @ v

    fwd_arm = to_arm_vec(fwd).normalized()
    up_arm = to_arm_vec(up).normalized()

    has_ebrow_sk = any(is_eyebrow_key(sk.name) for sk in keyblock) if keyblock else False
    ebrow_bones = [] if has_ebrow_sk else find_eyebrow_bones(armature)
    if has_ebrow_sk and keyblock:
        controls.extend(build_brow_shapekey_controls(mesh_obj, armature, fwd, up, face_size))
    elif ebrow_bones:
        pass

    sides = {}
    for name in ebrow_bones:
        b = armature.data.bones.get(name)
        if b is None:
            continue
        s = eyebrow_side(name)
        sides.setdefault(s, []).append((name, armature.matrix_world @ b.head_local))

    for s, items in sides.items():
        center = Vector((0.0, 0.0, 0.0))
        for _, p in items:
            center += p
        center /= len(items)
        master_name = 'CTRL-Master-Eyebrow.%s' % s
        positions = [p for _, p in items]
        ebw = 0.0
        for _ia in range(len(positions)):
            for _ib in range(_ia + 1, len(positions)):
                _d = (positions[_ia] - positions[_ib]).length
                if _d > ebw:
                    ebw = _d
        if ebw < 1e-6:
            ebw = face_size * 0.1
        controls.append({'name': master_name, 'collection': FACERIG_COLLECTION,
                         'color': COL_EBRMASTER, 'group': 'Face Eyebrow',
                         'head': center + fwd * (face_size * OFFSET_F * 0.8),
                         'widget': 'pad', 'lim': face_size * EBR_MASTER_F,
                         'free': ('X', 'Y', 'Z'), 'range': 'both',
                         'shape_scale': Vector((ebw * 0.65, 1.0, ebw * 0.14)),
                         'shape_rotation': (0.0, -0.15 if s == 'L' else 0.15, 0.43825 if s == 'L' else -0.43825),
                         'kind': 'master', 'drivers': []})
        for name, head_world in items:
            front = center + (head_world - center) * 0.75 + fwd * (face_size * OFFSET_F * 0.8)
            ctrl_name = 'CTRL-' + name.strip().replace(' ', '_')
            if '01' in name:
                tw_widget = 'ebrowtweak3.%s' % s
            elif '02' in name:
                tw_widget = 'ebrowtweak2.%s' % s
            elif '03' in name:
                tw_widget = 'ebrowtweak1.%s' % s
            else:
                tw_widget = 'ebrowtweak1.%s' % s
            controls.append({'name': ctrl_name, 'collection': FACERIG_COLLECTION,
                             'color': COL_EBRBONE, 'group': 'Face Eyebrow',
                             'head': front, 'widget': tw_widget,
                             'lim': face_size * EBR_WGT_F, 'free': ('X', 'Y', 'Z'),
                             'range': 'both',
                             'shape_scale': Vector((face_size * EBR_WGT_F,) * 3),
                             'kind': 'fk', 'target_bone': name, 'parent': master_name,
                             'hook_name': ctrl_name + '_Hook', 'hook_head': head_world,
                             'drivers': []})

    bpy.context.view_layer.objects.active = armature
    try:
        armature.data.use_mirror_x = False
    except Exception:
        pass
    try:
        armature.pose.use_mirror_x = False
    except Exception:
        pass
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones

    root = eb.get("Face-Root") or eb.new("Face-Root")
    head_edit = eb.get(head_name)
    root.head = head_edit.head.copy() if head_edit else Vector((0, 0, 0))
    root.tail = root.head + up_arm * bone_len * 2.0
    root.use_deform = False
    if head_edit:
        root.parent = head_edit

    for c in controls:
        b = eb.get(c['name']) or eb.new(c['name'])
        h = to_arm_point(c['head'])
        b.head = h
        b.tail = h + fwd_arm * bone_len
        b.use_deform = False
        try:
            b.align_roll(up_arm)
        except Exception:
            pass
        par = eb.get(c['parent']) if c.get('parent') else None
        b.parent = par if par else root
        b.use_connect = False
        if c.get('hook_name'):
            hk = eb.get(c['hook_name']) or eb.new(c['hook_name'])
            hh = to_arm_point(c['hook_head'])
            hk.head = hh
            hk.tail = hh + fwd_arm * bone_len
            hk.use_deform = False
            hk.parent = b
            hk.use_connect = False

    bpy.ops.object.mode_set(mode='OBJECT')

    coll_names = []
    for c in controls:
        if c['collection'] not in coll_names:
            coll_names.append(c['collection'])
    if ver != 3 and hasattr(armature.data, "collections"):
        for cn in coll_names:
            coll = armature.data.collections.get(cn)
            if not coll:
                coll = armature.data.collections.new(cn)
            try:
                coll.is_visible = True
            except Exception:
                pass
            try:
                coll.rigify_ui_row = RIGIFY_UI_ROW
            except Exception:
                pass
        for c in controls:
            bone = armature.data.bones.get(c['name'])
            coll = armature.data.collections.get(c['collection'])
            if bone and coll:
                coll.assign(bone)
        other_c = armature.data.collections.get("Other") or armature.data.collections.new("Other")
        face_c = armature.data.collections.get("Face")
        hook_names = [c['hook_name'] for c in controls if c.get('hook_name')]
        for hn in hook_names:
            bone = armature.data.bones.get(hn)
            if bone:
                other_c.assign(bone)
                if face_c:
                    face_c.unassign(bone)
        for b in armature.data.bones:
            if "hook" in b.name.lower():
                other_c.assign(b)
                if face_c:
                    face_c.unassign(b)

    wgt_coll = get_widget_collection()
    bpy.ops.object.mode_set(mode='POSE')
    color_cache = {}

    for c in controls:
        pb = armature.pose.bones.get(c['name'])
        if not pb:
            continue

        if c.get('kind') in ('fk', 'master'):
            is_master = c['kind'] == 'master'
            for i in range(3):
                pb.lock_location[i] = False
                pb.lock_rotation[i] = not is_master
                pb.lock_scale[i] = not is_master
            pb.lock_rotation_w = not is_master
            pb.custom_shape = make_widget(c['widget'], wgt_coll)
            try:
                pb.use_custom_shape_bone_size = False
            except Exception:
                pass
            ss = c.get('shape_scale')
            pb.custom_shape_scale_xyz = ss if ss is not None else Vector((c['lim'] * WIDGET_F,) * 3)
            sr = c.get('shape_rotation')
            if sr is not None:
                try:
                    pb.custom_shape_rotation_euler = sr
                except Exception:
                    pass
            apply_color(armature, pb, c['group'], c['color'], color_cache)
            tb = armature.pose.bones.get(c['target_bone']) if c.get('target_bone') else None
            if tb:
                for cn in [x for x in tb.constraints if x.name == 'FaceRig CopyLoc']:
                    tb.constraints.remove(cn)
                con = tb.constraints.new(type='COPY_LOCATION')
                con.name = 'FaceRig CopyLoc'
                con.target = armature
                con.subtarget = c.get('hook_name') or c['name']
            continue

        lim = c['lim']
        free = c['free']
        rng = c['range']
        if rng == 'both':
            lo, hi = -lim, lim
        elif rng == 'neg':
            lo, hi = -lim, 0.0
        else:
            lo, hi = 0.0, lim

        pb.lock_location[0] = 'X' not in free
        pb.lock_location[1] = True
        pb.lock_location[2] = 'Z' not in free
        for i in range(3):
            pb.lock_rotation[i] = True
            pb.lock_scale[i] = True
        pb.lock_rotation_w = True

        con = pb.constraints.new(type='LIMIT_LOCATION')
        con.owner_space = 'LOCAL'
        con.use_transform_limit = True
        con.use_min_x = con.use_max_x = True
        con.use_min_y = con.use_max_y = True
        con.use_min_z = con.use_max_z = True
        con.min_y = con.max_y = 0.0
        con.min_x = lo if 'X' in free else 0.0
        con.max_x = hi if 'X' in free else 0.0
        con.min_z = lo if 'Z' in free else 0.0
        con.max_z = hi if 'Z' in free else 0.0

        pb.custom_shape = make_widget(c['widget'], wgt_coll)
        try:
            pb.use_custom_shape_bone_size = False
        except Exception:
            pass
        ss = c.get('shape_scale')
        pb.custom_shape_scale_xyz = ss if ss is not None else Vector((lim * WIDGET_F,) * 3)
        sr = c.get('shape_rotation')
        if sr is not None:
            try:
                pb.custom_shape_rotation_euler = sr
            except Exception:
                pass

        apply_color(armature, pb, c['group'], c['color'], color_cache)

    bpy.ops.object.mode_set(mode='OBJECT')

    if keyblock is not None:
        agg = {}
        for c in controls:
            for d in c.get('drivers', []):
                agg.setdefault(d['key'], []).append(
                    {'bone': c['name'], 'axis': d['axis'], 'dir': d['dir'],
                     'lim': c['lim'], 'bidir': d.get('bidir', False),
                     'gain': d.get('gain', 1.0)})

        for key, entries in agg.items():
            sk = keyblock.get(key)
            if sk is None:
                continue
            sk.slider_min = -1.0 if any(e['bidir'] for e in entries) else 0.0
            try:
                sk.driver_remove("value")
            except Exception:
                pass
            drv = sk.driver_add("value").driver
            drv.type = 'SCRIPTED'
            terms = []
            for i, e in enumerate(entries):
                vn = "v%d" % i
                var = drv.variables.new()
                var.name = vn
                var.type = 'TRANSFORMS'
                tgt = var.targets[0]
                tgt.id = armature
                tgt.bone_target = e['bone']
                tgt.transform_type = 'LOC_' + e['axis']
                tgt.transform_space = 'LOCAL_SPACE'
                sign = '' if e['dir'] > 0 else '-'
                g = e.get('gain', 1.0)
                if e['bidir']:
                    base = "%s%s / %r" % (sign, vn, e['lim'])
                else:
                    base = "max(0.0, %s%s / %r)" % (sign, vn, e['lim'])
                terms.append(base if g == 1.0 else "%r * (%s)" % (g, base))
            drv.expression = terms[0] if len(terms) == 1 else "max(" + ", ".join(terms) + ")"

        _ang = keyblock.get("Fac_Ebr_Angry")
        if _ang is not None:
            _ang.slider_min = 0.0


def assign_bones_to_other_collection(armature, bone_names):
    if ver != 3 and hasattr(armature.data, "collections"):
        coll = armature.data.collections.get("Other") or armature.data.collections.new("Other")
        try:
            coll.is_visible = False
        except Exception:
            pass
        for bname in bone_names:
            b = armature.data.bones.get(bname)
            if b:
                for c in list(b.collections):
                    try:
                        c.unassign(b)
                    except Exception:
                        pass
                coll.assign(b)
    elif hasattr(armature.pose, "bone_groups"):
        bg = armature.pose.bone_groups.get("Other")
        if bg:
            for bname in bone_names:
                pb = armature.pose.bones.get(bname)
                if pb:
                    pb.bone_group = bg


def remove_eyetrack_bones(armature):
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode='EDIT')
    ebs = armature.data.edit_bones
    targets = ("eyetrack", "eyetrack_L", "eyetrack_R", "eyetrack.L", "eyetrack.R",
               "EyeTrack", "EyeTrack_L", "EyeTrack_R", "EyeTrack.L", "EyeTrack.R")
    for name in targets:
        b = ebs.get(name)
        if b:
            ebs.remove(b)
    for b in list(ebs):
        if b.name.lower().startswith("eyetrack"):
            ebs.remove(b)
    bpy.ops.object.mode_set(mode='OBJECT')


def hide_mechanism_bones(armature):
    HIDE_PREFIXES = (
        "CtrEyebrow", "SknEyebrow", "SknEyeLight", "SknMouth", "Face-Root",
        "Skn_L_Mouth", "Skn_R_Mouth", "Skn_M_Mouth", "Skn_L_Highlights",
        "Skn_R_Highlights", "Skn_RemiMk_", "BdyMouth", "PTMouth", "MCH-"
    )

    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    hidden = 0
    for b in armature.data.bones:
        if any(b.name.startswith(p) for p in HIDE_PREFIXES):
            b.hide = True
            hidden += 1

    assign_bones_to_other_collection(armature, ("MCH-EyeAim.L", "MCH-EyeAim.R", "MCH-Eyes_Parent", "Face-Root"))


def get_facerig_bone_collection(armature, name=FACERIG_COLLECTION):
    if ver != 3 and hasattr(armature.data, "collections"):
        coll = armature.data.collections.get(name) or armature.data.collections.new(name)
        try:
            coll.is_visible = True
        except Exception:
            pass
        return coll
    return None


def setup_lookat_eyes(armature, head_name, fwd, up, face_size):
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature

    eye_L_name, eye_R_name = find_eye_bone_pair(armature)
    if not eye_L_name or not eye_R_name:
        return

    amw_inv = armature.matrix_world.inverted()
    fwd_arm = (amw_inv.to_3x3() @ fwd).normalized()
    up_arm = (amw_inv.to_3x3() @ up).normalized()

    MASTER = "CTRL-Eye_Master"
    PAIRS = ((eye_L_name, "CTRL-Eye.L", "MCH-EyeAim.L"), (eye_R_name, "CTRL-Eye.R", "MCH-EyeAim.R"))

    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones
    for nm in (MASTER, "CTRL-Eye.L", "CTRL-Eye.R", "MCH-EyeAim.L", "MCH-EyeAim.R"):
        old = eb.get(nm)
        if old:
            eb.remove(old)

    e_heads = {}
    gaze_dirs = {}
    track_axes = {}
    for eye_name, ctl_name, mch_name in PAIRS:
        e = eb.get(eye_name)
        if not e:
            continue
        m = e.matrix
        cand = [(m.col[0].to_3d(), 'TRACK_X', 'TRACK_NEGATIVE_X'),
                (m.col[1].to_3d(), 'TRACK_Y', 'TRACK_NEGATIVE_Y'),
                (m.col[2].to_3d(), 'TRACK_Z', 'TRACK_NEGATIVE_Z')]
        best = None
        for vec, pax, nax in cand:
            if vec.length < 1e-9:
                continue
            v = vec.normalized()
            d = v.dot(fwd_arm)
            if best is None or abs(d) > abs(best[0]):
                best = (d, v, pax, nax)
        if best is None:
            gaze_dirs[eye_name] = fwd_arm.copy()
            track_axes[eye_name] = 'TRACK_Y'
        else:
            d, v, pax, nax = best
            if d >= 0:
                gaze_dirs[eye_name] = v
                track_axes[eye_name] = pax
            else:
                gaze_dirs[eye_name] = -v
                track_axes[eye_name] = nax
        e_heads[eye_name] = e.head.copy()

    if eye_L_name not in e_heads or eye_R_name not in e_heads:
        bpy.ops.object.mode_set(mode='OBJECT')
        return

    def is_valid_eye_vg(name, is_left):
        n = name.lower()
        if any(k in n for k in ['brow', 'lash', 'shadow', 'highlight', 'specular', 'lid', 'corner', 'tear']):
            return False
        if is_left:
            return any(k in n for k in ['skn_l_eye', 'eye.l', 'eye_l', 'l_eye', 'eyeball.l', 'def-eye.l']) or ('eye' in n and any(k in n for k in ['_l', '.l', 'left']))
        else:
            return any(k in n for k in ['skn_r_eye', 'eye.r', 'eye_r', 'r_eye', 'eyeball.r', 'def-eye.r']) or ('eye' in n and any(k in n for k in ['_r', '.r', 'right']))

    sep = (e_heads[eye_L_name] - e_heads[eye_R_name]).length
    if sep < 1e-6:
        sep = face_size * 0.1
    blen = max(sep * 0.5, face_size * BONE_LEN_F * 2.0)
    offset = min(face_size * EYE_LOOK_FWD_F, sep * 3.0)

    parent_bone = eb.get(head_name) if head_name else None
    if parent_bone is None and eye_L_name in eb:
        parent_bone = eb[eye_L_name].parent

    aim_heads = {}
    ctrl_heads = {}
    for eye_name, ctl_name, mch_name in PAIRS:
        gd = gaze_dirs.get(eye_name, fwd_arm)
        aim_heads[eye_name] = e_heads[eye_name] + gd * offset
        ctrl_heads[eye_name] = e_heads[eye_name] + fwd_arm * offset

    master_head = (ctrl_heads[eye_L_name] + ctrl_heads[eye_R_name]) * 0.5
    mb = eb.new(MASTER)
    mb.head = master_head
    mb.tail = master_head + fwd_arm * blen
    try:
        mb.align_roll(up_arm)
    except Exception:
        pass
    mb.use_deform = False
    mb.parent = parent_bone
    mb.use_connect = False

    for eye_name, ctl_name, mch_name in PAIRS:
        ch = ctrl_heads[eye_name]
        cb = eb.new(ctl_name)
        cb.head = ch
        cb.tail = ch + fwd_arm * blen
        try:
            cb.align_roll(up_arm)
        except Exception:
            pass
        cb.use_deform = False
        cb.parent = parent_bone
        cb.use_connect = False

        ah = aim_heads[eye_name]
        ab = eb.new(mch_name)
        ab.head = ah
        ab.tail = ah + fwd_arm * blen
        try:
            ab.align_roll(up_arm)
        except Exception:
            pass
        ab.use_deform = False
        ab.parent = cb
        ab.use_connect = False
        ab.inherit_scale = 'NONE'
    bpy.ops.object.mode_set(mode='OBJECT')

    wgt_coll = get_widget_collection()
    bpy.ops.object.mode_set(mode='POSE')
    sep_ctrl = (ctrl_heads[eye_L_name] - ctrl_heads[eye_R_name]).length
    sm_x = sep_ctrl / 0.86
    sm_yz = sep_ctrl / 0.96934
    pbm = armature.pose.bones.get(MASTER)
    if pbm:
        pbm.custom_shape = make_widget('eyemaster', wgt_coll)
        try:
            pbm.use_custom_shape_bone_size = False
        except Exception:
            pass
        pbm.custom_shape_scale_xyz = Vector((sm_x, sm_yz, sm_yz))
        apply_color(armature, pbm, 'Face Eye-Aim', COL_EYEGREEN, {})
    se = sm_yz * 0.5
    for eye_name, ctl_name, mch_name in PAIRS:
        pbc = armature.pose.bones.get(ctl_name)
        if pbc:
            pbc.custom_shape = make_widget('eyecircle', wgt_coll)
            try:
                pbc.use_custom_shape_bone_size = False
            except Exception:
                pass
            pbc.custom_shape_scale_xyz = Vector((se, se, se))
            apply_color(armature, pbc, 'Face Eye-Aim', COL_EYEGREEN, {})

            for con in list(pbc.constraints):
                if con.name in ("MasterLoc", "MasterRot", "MasterScale"):
                    pbc.constraints.remove(con)
            con_loc = pbc.constraints.new('COPY_LOCATION')
            con_loc.name = "MasterLoc"
            con_loc.target = armature
            con_loc.subtarget = MASTER
            con_loc.target_space = 'LOCAL'
            con_loc.owner_space = 'LOCAL'
            con_loc.use_offset = True

            con_rot = pbc.constraints.new('COPY_ROTATION')
            con_rot.name = "MasterRot"
            con_rot.target = armature
            con_rot.subtarget = MASTER
            con_rot.target_space = 'LOCAL'
            con_rot.owner_space = 'LOCAL'
            con_rot.use_offset = True

    depsgraph = bpy.context.evaluated_depsgraph_get()
    depsgraph.update()
    centers = {}
    for eye_name, ctl_name, mch_name in PAIRS:
        is_l = (eye_name == eye_L_name) or ('.l' in eye_name.lower()) or ('_l' in eye_name.lower()) or ('l_eye' in eye_name.lower())
        c_world = None
        for obj in bpy.data.objects:
            if obj.type != 'MESH':
                continue
            is_child = (obj.parent == armature)
            has_mod = any(m.type == 'ARMATURE' and m.object == armature for m in obj.modifiers)
            if not (is_child or has_mod):
                continue
            target_vg = None
            for vg in obj.vertex_groups:
                if is_valid_eye_vg(vg.name, is_l):
                    target_vg = vg
                    break
            if target_vg:
                eval_obj = obj.evaluated_get(depsgraph)
                eval_mesh = eval_obj.to_mesh()
                coords = [eval_obj.matrix_world @ eval_mesh.vertices[v.index].co
                          for v in obj.data.vertices
                          if any(g.group == target_vg.index and g.weight > 0.3 for g in v.groups)]
                eval_obj.to_mesh_clear()
                if coords:
                    c_world = sum(coords, Vector((0, 0, 0))) / len(coords)
                    break
        if c_world is None:
            c_world = armature.matrix_world @ (e_heads[eye_name] + fwd_arm * (face_size * 0.05))
        centers[eye_name] = c_world

    for eye_name, ctl_name, mch_name in PAIRS:
        pbe = armature.pose.bones.get(eye_name)
        if not pbe:
            continue
        for con in list(pbe.constraints):
            if con.name in ("CTRL-EyeAim", "CTRL-EyeScale", "CTRL-EyeScale_Master", "CTRL-EyeScalePivot", "CTRL-EyeScalePivot_Master"):
                pbe.constraints.remove(con)

        con = pbe.constraints.new('DAMPED_TRACK')
        con.name = "CTRL-EyeAim"
        con.target = armature
        con.subtarget = mch_name
        con.track_axis = track_axes[eye_name]

        H = pbe.head.copy()
        c_arm = armature.matrix_world.inverted() @ centers[eye_name]
        disp = H - c_arm

        cs_m = pbe.constraints.new('COPY_SCALE')
        cs_m.name = "CTRL-EyeScale_Master"
        cs_m.target = armature
        cs_m.subtarget = MASTER
        cs_m.target_space = 'LOCAL'
        cs_m.owner_space = 'LOCAL'

        tc_m = pbe.constraints.new('TRANSFORM')
        tc_m.name = "CTRL-EyeScalePivot_Master"
        tc_m.target = armature
        tc_m.subtarget = MASTER
        tc_m.target_space = 'LOCAL'
        tc_m.owner_space = 'POSE'
        tc_m.map_from = 'SCALE'
        tc_m.map_to = 'LOCATION'
        tc_m.use_motion_extrapolate = True
        tc_m.mix_mode = 'ADD'
        tc_m.map_to_x_from = 'X'
        tc_m.map_to_y_from = 'X'
        tc_m.map_to_z_from = 'X'
        tc_m.from_min_x_scale = 1.0
        tc_m.from_max_x_scale = 2.0
        tc_m.to_min_x = 0.0
        tc_m.to_max_x = disp.x
        tc_m.to_min_y = 0.0
        tc_m.to_max_y = disp.y
        tc_m.to_min_z = 0.0
        tc_m.to_max_z = disp.z

        cs = pbe.constraints.new('COPY_SCALE')
        cs.name = "CTRL-EyeScale"
        cs.target = armature
        cs.subtarget = ctl_name
        cs.target_space = 'LOCAL'
        cs.owner_space = 'LOCAL'
        cs.use_offset = True

        tc = pbe.constraints.new('TRANSFORM')
        tc.name = "CTRL-EyeScalePivot"
        tc.target = armature
        tc.subtarget = ctl_name
        tc.target_space = 'LOCAL'
        tc.owner_space = 'POSE'
        tc.map_from = 'SCALE'
        tc.map_to = 'LOCATION'
        tc.use_motion_extrapolate = True
        tc.mix_mode = 'ADD'
        tc.map_to_x_from = 'X'
        tc.map_to_y_from = 'X'
        tc.map_to_z_from = 'X'
        tc.from_min_x_scale = 1.0
        tc.from_max_x_scale = 2.0
        tc.to_min_x = 0.0
        tc.to_max_x = disp.x
        tc.to_min_y = 0.0
        tc.to_max_y = disp.y
        tc.to_min_z = 0.0
        tc.to_max_z = disp.z
    bpy.ops.object.mode_set(mode='OBJECT')

    fcoll = get_facerig_bone_collection(armature)
    if fcoll is not None:
        for nm in (MASTER, "CTRL-Eye.L", "CTRL-Eye.R"):
            b = armature.data.bones.get(nm)
            if not b:
                continue
            for c in list(b.collections):
                try:
                    c.unassign(b)
                except Exception:
                    pass
            fcoll.assign(b)
    else:
        for nm in (MASTER, "CTRL-Eye.L", "CTRL-Eye.R"):
            b = armature.data.bones.get(nm)
            if b and hasattr(b, "layers"):
                b.layers[0] = True
                b.layers[1] = False
    for nm in ("MCH-EyeAim.L", "MCH-EyeAim.R"):
        b = armature.data.bones.get(nm)
        if b:
            b.hide = True



FACERIG_DETAIL_COLLECTION = "Face (Detail)"

FACERIG_SLIDER_COORDS = {
    # Eyebrows
    'Ebr_Angry': (2.4779, 0.0, 1.8121),
    'Ebr_Down': (2.4779, 0.0, 1.7502),
    'Ebr_L_Up': (2.4779, 0.0, 1.6752),
    'Ebr_R_Up': (2.4779, 0.0, 1.6133),
    'Ebr_Relax': (2.4779, 0.0, 1.5497),
    'Ebr_Sad': (2.4779, 0.0, 1.4874),

    # Eyes
    'Eye_Angry': (1.7792, 0.0, 0.5454),
    'Eye_Close': (1.1063, 0.0, 0.8463),
    'Eye_HalfClose': (1.7792, 0.0, 0.8298),
    'Eye_L_Open': (1.1063, 0.0, 0.5159),
    'Eye_L_Wink': (1.1063, 0.0, 0.7094),
    'Eye_LowlidUp': (1.7792, 0.0, 0.6195),
    'Eye_MidDown': (1.7792, 0.0, 0.6929),
    'Eye_MidUp': (1.7781, 0.0, 0.7584),
    'Eye_R_Open': (1.1063, 0.0, 0.5778),
    'Eye_R_Wink': (1.1063, 0.0, 0.7713),
    'Eye_Sad': (1.7792, 0.0, 0.4816),

    # Mouth
    'Mth': (1.7768, 0.0, 1.0575),  # 2D controller
    'Mth_Aa1': (1.7824, 0.0, 1.8121),
    'Mth_AaTalk': (1.7824, 0.0, 1.4250),
    'Mth_Ee': (1.7824, 0.0, 1.6133),
    'Mth_Ii': (1.7824, 0.0, 1.7502),
    'Mth_L_Down': (1.1035, 0.0, 1.5977),
    'Mth_L_In': (1.1035, 0.0, 1.3762),
    'Mth_L_Out': (1.1035, 0.0, 1.1495),
    'Mth_L_Up': (1.1035, 0.0, 1.8121),
    'Mth_Laugh': (1.7824, 0.0, 1.3614),
    'Mth_R_Down': (1.1035, 0.0, 1.5358),
    'Mth_R_In': (1.1035, 0.0, 1.3143),
    'Mth_R_Out': (1.1035, 0.0, 1.0876),
    'Mth_R_Up': (1.1035, 0.0, 1.7502),
    'Mth_Uu': (1.7824, 0.0, 1.6752),
    'Mth_UuOo': (1.7824, 0.0, 1.5497),

    # Extra shapekeys (1..22)
    'Extra 1': (2.5308, 0.0, 1.1443),
    'Extra 2': (2.5297, 0.0, 1.0730),
    'Extra 3': (2.5308, 0.0, 1.0074),
    'Extra 4': (2.5308, 0.0, 0.9340),
    'Extra 5': (2.5308, 0.0, 0.8599),
    'Extra 6': (2.5308, 0.0, 0.7961),
    'Extra 7': (2.5308, 0.0, 0.7241),
    'Extra 8': (2.5297, 0.0, 0.6528),
    'Extra 9': (2.5308, 0.0, 0.5872),
    'Extra 10': (2.5308, 0.0, 0.5138),
    'Extra 11': (2.5308, 0.0, 0.4397),
    'Extra 12': (3.2323, 0.0, 1.1443),
    'Extra 13': (3.2312, 0.0, 1.0730),
    'Extra 14': (3.2323, 0.0, 1.0074),
    'Extra 15': (3.2323, 0.0, 0.9340),
    'Extra 16': (3.2323, 0.0, 0.8599),
    'Extra 17': (3.2323, 0.0, 0.7961),
    'Extra 18': (3.2323, 0.0, 0.7241),
    'Extra 19': (3.2312, 0.0, 0.6528),
    'Extra 20': (3.2323, 0.0, 0.5872),
    'Extra 21': (3.2323, 0.0, 0.5138),
    'Extra 22': (3.2323, 0.0, 0.4397),
}


def get_facerig_detail_bone_collection(armature):
    if hasattr(armature.data, "collections"):
        coll = armature.data.collections.get(FACERIG_DETAIL_COLLECTION) or armature.data.collections.new(FACERIG_DETAIL_COLLECTION)
        return coll
    return None


def get_huge_facerig_widgets(wgt_coll):
    panel_wgt_name = "WGT-Face_Huge_Facerig_Panel"
    controls_wgt_name = "WGT-Face_Huge_Facerig_Controls"

    panel_obj = bpy.data.objects.get(panel_wgt_name)
    controls_obj = bpy.data.objects.get(controls_wgt_name)
    if panel_obj and controls_obj:
        return panel_obj, controls_obj

    current_dir = os.path.dirname(os.path.abspath(__file__))
    blend_candidates = [
        os.path.join(current_dir, 'ZZZ_Facerig.blend'),
        r'D:\BLENDERRR\ZZZ\shaders\ZZZ Setup File V2.0.blend',
    ]
    blend_path = next((p for p in blend_candidates if os.path.isfile(p)), None)
    if not blend_path:
        print("[ZZZ Face Rig Warning] Could not find ZZZ_Facerig.blend")
        return panel_obj, controls_obj

    try:
        with bpy.data.libraries.load(blend_path, link=False) as (data_from, data_to):
            load_objs = []
            if "Huge Facerig Panel" in data_from.objects:
                load_objs.append("Huge Facerig Panel")
            elif "Huge Facerig" in data_from.objects:
                load_objs.append("Huge Facerig")
            elif "Facerig Border" in data_from.objects:
                load_objs.append("Facerig Border")

            if "Huge Facerig Controls" in data_from.objects:
                load_objs.append("Huge Facerig Controls")

            data_to.objects = load_objs

        for obj in data_to.objects:
            if not obj:
                continue
            if "Controls" in obj.name:
                obj.name = controls_wgt_name
                controls_obj = obj
            else:
                obj.name = panel_wgt_name
                panel_obj = obj
    except Exception as e:
        print(f"[ZZZ Face Rig Warning] Failed to load Huge Facerig widgets: {e}")

    for w_obj in (panel_obj, controls_obj):
        if w_obj:
            if wgt_coll and w_obj.name not in wgt_coll.objects:
                wgt_coll.objects.link(w_obj)
            if w_obj.name in bpy.context.scene.collection.objects:
                try:
                    bpy.context.scene.collection.objects.unlink(w_obj)
                except Exception:
                    pass
            w_obj.hide_viewport = True
            w_obj.hide_render = True

    return panel_obj, controls_obj


def update_rig_ui_text_for_face_detail(armature=None):
    candidates = []
    if armature:
        arm_name = armature.name.replace("RIG - ", "").replace("RIG-", "").strip()
        candidates.extend([f"{arm_name}_RigUI.py", f"{armature.name}_RigUI.py"])
    candidates.extend(["rig_ui.py", "Miyabi_RigUI.py"])

    found_texts = []
    for cand in candidates:
        t = bpy.data.texts.get(cand)
        if t and t not in found_texts:
            found_texts.append(t)

    for t in bpy.data.texts:
        if ("rigui" in t.name.lower() or "rig_ui" in t.name.lower()) and t not in found_texts:
            found_texts.append(t)

    for t in found_texts:
        content = t.as_string()
        if 'Face (Detail)' in content:
            try:
                with bpy.context.temp_override(edit_text=t):
                    bpy.ops.text.run_script()
            except Exception:
                pass
            continue

        if 'collection["Face"]' in content or "collection['Face']" in content:
            face_key = 'collection["Face"]' if 'collection["Face"]' in content else "collection['Face']"
            idx = content.find(face_key)
            end_line = content.find('\n', idx)
            next_line = content.find('\n', end_line + 1)
            line_start = content.rfind('\n', 0, idx) + 1
            indent = content[line_start:idx]
            if not indent.strip() == "":
                indent = "\t\t"

            detail_code = (
                f"\n{indent}if \"Face (Detail)\" in collection:\n"
                f"{indent}\trow = col.row(align = True)\n"
                f"{indent}\trow.prop(collection[\"Face (Detail)\"], 'is_visible', toggle=True, text='Face (Detail)')\n"
                f"{indent}\trow.prop(collection[\"Face (Detail)\"], 'is_solo', toggle=True, text=\"\", icon='SOLO_OFF', invert_checkbox=False, emboss=True)\n"
            )
            new_content = content[:next_line] + detail_code + content[next_line:]
            t.clear()
            t.write(new_content)

            try:
                with bpy.context.temp_override(edit_text=t):
                    bpy.ops.text.run_script()
            except Exception:
                try:
                    exec(compile(t.as_string(), t.name, 'exec'), {})
                except Exception as e:
                    print(f"[ZZZ Face Rig Warning] Could not reload rig UI script {t.name}: {e}")


def setup_facerig_detail_panel(faceobj, armature, head_name, keyblock):
    # Clean up any leftover legacy Facerig scene collections or standalone objects
    old_facerig_col = bpy.data.collections.get("Facerig")
    if old_facerig_col:
        for parent_col in list(bpy.data.collections):
            if old_facerig_col.name in parent_col.children:
                try:
                    parent_col.children.unlink(old_facerig_col)
                except Exception:
                    pass
        if old_facerig_col.name in bpy.context.scene.collection.children:
            try:
                bpy.context.scene.collection.children.unlink(old_facerig_col)
            except Exception:
                pass
        for obj in list(old_facerig_col.objects):
            if not obj.name.startswith("WGT-Face_Huge_Facerig"):
                try:
                    bpy.data.objects.remove(obj, do_unlink=True)
                except Exception:
                    pass
        try:
            bpy.data.collections.remove(old_facerig_col)
        except Exception:
            pass

    old_border = bpy.data.objects.get("Facerig Border")
    if old_border and not old_border.name.startswith("WGT-Face_Huge_Facerig"):
        try:
            bpy.data.objects.remove(old_border, do_unlink=True)
        except Exception:
            pass

    wgt_coll = get_widget_collection()
    panel_wgt, controls_wgt = get_huge_facerig_widgets(wgt_coll)

    base_slider_names = {
        'Ebr_Angry', 'Ebr_Down', 'Ebr_L_Up', 'Ebr_R_Up', 'Ebr_Relax', 'Ebr_Sad',
        'Eye_Angry', 'Eye_Close', 'Eye_HalfClose', 'Eye_L_Open', 'Eye_L_Wink',
        'Eye_LowlidUp', 'Eye_MidDown', 'Eye_MidUp', 'Eye_R_Open', 'Eye_R_Wink', 'Eye_Sad',
        'Mth_Aa1', 'Mth_AaTalk', 'Mth_Down', 'Mth_Ee', 'Mth_Ii', 'Mth_L_Down',
        'Mth_L_In', 'Mth_L_Out', 'Mth_L_Up', 'Mth_Laugh', 'Mth_Left', 'Mth_R_Down',
        'Mth_R_In', 'Mth_R_Out', 'Mth_R_Up', 'Mth_Right', 'Mth_Up', 'Mth_Uu', 'Mth_UuOo'
    }

    # Find character extra shapekeys
    extra_shapekeys = []
    if keyblock:
        for sk in keyblock:
            if sk.name == "Basis":
                continue
            raw_name = sk.name[4:] if sk.name.startswith("Fac_") else sk.name
            if raw_name not in base_slider_names and not raw_name.startswith("Extra") and raw_name not in extra_shapekeys:
                extra_shapekeys.append(raw_name)

    # Switch armature to EDIT mode and create detail panel bones
    bpy.ops.object.mode_set(mode='OBJECT')
    bpy.context.view_layer.objects.active = armature
    armature.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    eb = armature.data.edit_bones

    # Facerig Root aligned in +Y with roll=0 so local axes match world axes identically
    faceroot = eb.get("Facerig Root") or eb.new("Facerig Root")
    faceroot.head = Vector((1.2417, 0.0, 1.1727))
    faceroot.tail = Vector((1.2417, 0.2, 1.1727))
    faceroot.roll = 0.0
    faceroot.use_deform = False
    faceroot.parent = None

    panel_bone_names = ["Facerig Root"]

    # Facerig Controls bone for displaying the non-selectable text & slider borders
    if controls_wgt:
        facerig_ctrls = eb.get("Facerig Controls") or eb.new("Facerig Controls")
        facerig_ctrls.head = faceroot.head
        facerig_ctrls.tail = faceroot.tail
        facerig_ctrls.roll = 0.0
        facerig_ctrls.parent = faceroot
        facerig_ctrls.use_connect = False
        facerig_ctrls.use_deform = False
        panel_bone_names.append("Facerig Controls")

    # 1. Base sliders
    for key, coord in FACERIG_SLIDER_COORDS.items():
        if key.startswith("Extra "):
            continue
        if key == 'Mth':
            has_2d = any(k in keyblock for k in ("Fac_Mth_Up", "Fac_Mth_Down", "Fac_Mth_Left", "Fac_Mth_Right", "Mth_Up", "Mth_Down", "Mth_Left", "Mth_Right"))
            if not has_2d:
                continue
            bname = "Mth Bone"
            head_pos = Vector(coord)
        else:
            has_sk = (("Fac_" + key) in keyblock) or (key in keyblock)
            if not has_sk:
                continue
            bname = key + " Bone"
            # Offset to the left of the slider box so it starts at 0 and slides to the right (0 to 1)
            head_pos = Vector(coord) - Vector((0.10, 0.0, 0.0))

        b = eb.get(bname) or eb.new(bname)
        b.head = head_pos
        b.tail = b.head + Vector((0.0, 0.05, 0.0))
        b.roll = 0.0
        b.parent = faceroot
        b.use_connect = False
        b.use_deform = False
        panel_bone_names.append(bname)

    # 2. Extra sliders (sequential slots Extra 1..22 for matching extras)
    for i, sk_name in enumerate(extra_shapekeys[:22], 1):
        slot_key = f"Extra {i}"
        coord = FACERIG_SLIDER_COORDS.get(slot_key)
        if not coord:
            continue
        bname = sk_name + " Bone"
        b = eb.get(bname) or eb.new(bname)
        # Offset to the left of the slider box so it starts at 0 and slides to the right (0 to 1)
        b.head = Vector(coord) - Vector((0.10, 0.0, 0.0))
        b.tail = b.head + Vector((0.0, 0.05, 0.0))
        b.roll = 0.0
        b.parent = faceroot
        b.use_connect = False
        b.use_deform = False
        panel_bone_names.append(bname)

    bpy.ops.object.mode_set(mode='OBJECT')

    # Assign bones to Face (Detail) collection & layer
    if hasattr(armature.data, "collections"):
        colls = armature.data.collections
        detail_coll = colls.get(FACERIG_DETAIL_COLLECTION) or colls.new(FACERIG_DETAIL_COLLECTION)
        face_main_coll = colls.get(FACERIG_COLLECTION)
        for bname in panel_bone_names:
            b = armature.data.bones.get(bname)
            if b:
                detail_coll.assign(b)
                if face_main_coll:
                    face_main_coll.unassign(b)
        detail_coll.is_visible = True
    else:
        # Blender 3: assign to layer 1
        for bname in panel_bone_names:
            b = armature.data.bones.get(bname)
            if b:
                b.layers[1] = True
                b.layers[0] = False

    # Setup pose mode constraints and custom shapes
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode='POSE')

    root_pb = armature.pose.bones.get("Facerig Root")
    if root_pb:
        if panel_wgt:
            root_pb.custom_shape = panel_wgt
            root_pb.use_custom_shape_bone_size = False
        else:
            root_pb.custom_shape = make_facerig_root_widget(wgt_coll)
        root_pb.custom_shape_scale_xyz = Vector((1.0, 1.0, 1.0))
        apply_color(armature, root_pb, 'Face Detail Root', COL_EBRMASTER, {})

    ctrls_pb = armature.pose.bones.get("Facerig Controls")
    if ctrls_pb:
        if controls_wgt:
            ctrls_pb.custom_shape = controls_wgt
            ctrls_pb.use_custom_shape_bone_size = False
        ctrls_pb.custom_shape_scale_xyz = Vector((1.0, 1.0, 1.0))
        ctrls_pb.lock_location = (True, True, True)
        ctrls_pb.lock_rotation = (True, True, True)
        ctrls_pb.lock_scale = (True, True, True)

    # Make Facerig Controls unselectable so clicking the panel only selects Facerig Root or sliders
    b_ctrls = armature.data.bones.get("Facerig Controls")
    if b_ctrls:
        b_ctrls.hide_select = True

    slider_wgt = make_widget('ring', wgt_coll)

    for bname in panel_bone_names:
        if bname in ("Facerig Root", "Facerig Controls"):
            continue
        pb = armature.pose.bones.get(bname)
        if not pb:
            continue
        pb.custom_shape = slider_wgt
        pb.use_custom_shape_bone_size = False

        if bname == "Mth Bone":
            pb.custom_shape_scale_xyz = Vector((0.038, 0.038, 0.038))
            apply_color(armature, pb, 'Face Detail Mth', COL_MOUTH, {})
            # Mouth moves in 2D (X and Z). Lock Y (depth into screen)
            pb.lock_location = (False, True, False)
            pb.lock_rotation = (True, True, True)
            pb.lock_scale = (True, True, True)

            c = pb.constraints.get("Limit Location") or pb.constraints.new('LIMIT_LOCATION')
            c.name = "Limit Location"
            c.use_min_x = True; c.min_x = -0.1
            c.use_max_x = True; c.max_x = 0.1
            c.use_min_z = True; c.min_z = -0.1
            c.use_max_z = True; c.max_z = 0.1
            c.use_transform_limit = True
            c.owner_space = 'LOCAL'
        else:
            pb.custom_shape_scale_xyz = Vector((0.018, 0.018, 0.018))
            if bname.startswith("Ebr_"):
                apply_color(armature, pb, 'Face Detail Brow', COL_BROW, {})
            elif bname.startswith("Eye_"):
                apply_color(armature, pb, 'Face Detail Eye', COL_EYELID, {})
            elif bname.startswith("Mth_"):
                apply_color(armature, pb, 'Face Detail Mth', COL_VISEME, {})
            else:
                apply_color(armature, pb, 'Face Detail Extra', COL_EBRBONE, {})

            # 1D sliders move along X. Lock Y and Z
            pb.lock_location = (False, True, True)
            pb.lock_rotation = (True, True, True)
            pb.lock_scale = (True, True, True)

            c = pb.constraints.get("Limit Location") or pb.constraints.new('LIMIT_LOCATION')
            c.name = "Limit Location"
            c.use_min_x = True; c.min_x = 0.0
            c.use_max_x = True; c.max_x = 0.2
            c.use_transform_limit = True
            c.owner_space = 'LOCAL'

    # Setup Child Of constraint on Facerig Root targeting root.002
    target_root = None
    for cand in ["root.002", "root_2", "root002", "root.001", "root"]:
        if cand in armature.pose.bones:
            target_root = cand
            break

    if root_pb and target_root:
        con_root = root_pb.constraints.get("Child Of") or root_pb.constraints.new('CHILD_OF')
        con_root.name = "Child Of"
        con_root.target = armature
        con_root.subtarget = target_root
        armature.data.bones.active = armature.data.bones["Facerig Root"]
        try:
            bpy.ops.constraint.childof_set_inverse(constraint=con_root.name, owner='BONE')
        except Exception:
            root_bone = armature.data.bones.get(target_root)
            pnl_bone = armature.data.bones.get("Facerig Root")
            if root_bone and pnl_bone:
                con_root.inverse_matrix = root_bone.matrix_local.inverted() @ pnl_bone.matrix_local
        print(f"[ZZZ Face Rig] Set Child Of constraint on 'Facerig Root' targeting '{target_root}'")

    bpy.context.view_layer.update()
    bpy.ops.object.mode_set(mode='OBJECT')

    def _get_or_add_driver(sk):
        if sk.id_data.animation_data:
            target_path = f'key_blocks["{sk.name}"].value'
            for fc in sk.id_data.animation_data.drivers:
                if fc.data_path == target_path:
                    return fc
        return sk.driver_add("value")

    # Connect Shapekey Drivers
    if faceobj and faceobj.data and faceobj.data.shape_keys:
        for shapekey in faceobj.data.shape_keys.key_blocks:
            if shapekey.name == "Basis":
                continue
            sk_name = shapekey.name
            base_name = sk_name[4:] if sk_name.startswith("Fac_") else sk_name
            bone_name = base_name + " Bone"
            if bone_name in armature.data.bones:
                fcurve = _get_or_add_driver(shapekey)
                drv = fcurve.driver
                drv.type = 'SCRIPTED'
                vname = "var_pnl"
                old_var = next((v for v in drv.variables if v.name == vname), None)
                if old_var:
                    drv.variables.remove(old_var)
                var = drv.variables.new()
                var.name = vname
                var.type = 'TRANSFORMS'
                tgt = var.targets[0]
                tgt.id = armature
                tgt.bone_target = bone_name
                tgt.transform_type = 'LOC_X'
                tgt.transform_space = 'LOCAL_SPACE'

                current_expr = drv.expression.strip() if (drv.expression and drv.expression.strip()) else ""
                # Clean up legacy (0.0) placeholder if present
                if current_expr.startswith("(0.0) + "):
                    current_expr = current_expr[8:].strip()
                elif current_expr in ("0", "0.0", "0.000", "(0.0)"):
                    current_expr = ""

                if current_expr:
                    if vname not in current_expr:
                        drv.expression = f"({current_expr}) + ({vname} / 0.2)"
                else:
                    drv.expression = f"{vname} / 0.2"

        # Mouth Bone 2D drivers
        if "Mth Bone" in armature.data.bones:
            sklist = ["Fac_Mth_Up", "Fac_Mth_Left", "Fac_Mth_Right", "Fac_Mth_Down"]
            for shapekey_name in sklist:
                if shapekey_name not in keyblock:
                    continue
                sk = keyblock[shapekey_name]
                fcurve = _get_or_add_driver(sk)
                drv = fcurve.driver
                drv.type = 'SCRIPTED'
                vname = "var_mth"
                old_var = next((v for v in drv.variables if v.name == vname), None)
                if old_var:
                    drv.variables.remove(old_var)
                var = drv.variables.new()
                var.name = vname
                var.type = 'TRANSFORMS'
                tgt = var.targets[0]
                tgt.id = armature
                tgt.bone_target = "Mth Bone"
                tgt.transform_space = 'LOCAL_SPACE'
                if "_L" in shapekey_name or "_R" in shapekey_name:
                    tgt.transform_type = 'LOC_X'
                else:
                    tgt.transform_type = 'LOC_Z'

                factor = "0.1" if ("_L" in shapekey_name or "_Up" in shapekey_name) else "-0.1"
                mth_term = f"({vname} / {factor})"
                current_expr = drv.expression.strip() if (drv.expression and drv.expression.strip()) else ""
                if current_expr.startswith("(0.0) + "):
                    current_expr = current_expr[8:].strip()
                elif current_expr in ("0", "0.0", "0.000", "(0.0)"):
                    current_expr = ""

                if current_expr:
                    if vname not in current_expr:
                        drv.expression = f"({current_expr}) + {mth_term}"
                else:
                    drv.expression = mth_term

        # Lycaon mask shapekeys support
        lycaon = bpy.data.objects.get("Lycaon_Body_3")
        if lycaon and lycaon.data and lycaon.data.shape_keys:
            for sk in lycaon.data.shape_keys.key_blocks:
                if "_Body" in sk.name:
                    sk.name = sk.name[:-5]
                bname = (sk.name[4:] if sk.name.startswith("Fac_") else sk.name) + " Bone"
                if bname in armature.data.bones:
                    fcurve = _get_or_add_driver(sk)
                    drv = fcurve.driver
                    drv.type = 'SCRIPTED'
                    vname = "var_lyc"
                    old_v = next((v for v in drv.variables if v.name == vname), None)
                    if old_v:
                        drv.variables.remove(old_v)
                    var = drv.variables.new()
                    var.name = vname
                    var.type = 'TRANSFORMS'
                    tgt = var.targets[0]
                    tgt.id = armature
                    tgt.bone_target = bname
                    tgt.transform_type = 'LOC_X'
                    tgt.transform_space = 'LOCAL_SPACE'
                    current_expr = drv.expression.strip() if (drv.expression and drv.expression.strip()) else ""
                    if current_expr.startswith("(0.0) + "):
                        current_expr = current_expr[8:].strip()
                    elif current_expr in ("0", "0.0", "0.000", "(0.0)"):
                        current_expr = ""

                    if current_expr:
                        if vname not in current_expr:
                            drv.expression = f"({current_expr}) + ({vname} / 0.2)"
                    else:
                        drv.expression = f"{vname} / 0.2"

    # Update Rig UI text scripts so Face (Detail) appears in N-panel Rig Layers
    update_rig_ui_text_for_face_detail(armature)



def zzz_face_rig_main():
    try:
        if bpy.context.object and bpy.context.object.mode != 'OBJECT':
            bpy.ops.object.mode_set(mode='OBJECT')

        faceobj = None
        for obj in bpy.data.objects:
            if obj.type != 'MESH':
                continue
            n = obj.name.lower()
            if any(ign in n for ign in ["weapon_", "wpn_", "garape_", "grape_", "gun_", "sword_"]):
                continue
            if "face" in n or "_face" in n or "head" in n:
                if obj.data and obj.data.shape_keys is not None:
                    faceobj = obj
                    break
                if faceobj is None:
                    faceobj = obj

        if faceobj is None:
            for obj in bpy.data.objects:
                if obj.type == 'MESH' and ('face' in obj.name.lower() or 'head' in obj.name.lower()):
                    faceobj = obj
                    break

        if faceobj is None:
            print("[ZZZ Face Rig] No face mesh found in the scene.")
            return

        has_shapekeys = faceobj.data and faceobj.data.shape_keys is not None
        keyblock = faceobj.data.shape_keys.key_blocks if has_shapekeys else None

        if keyblock:
            shapekeyrename(keyblock)

        armature, head_name = find_armature_and_head(faceobj)
        for _hn in (
            "eye.L", "eye.R", "eye.L.001", "eye.R.001", "SknEyeStar.L", "SknEyeStar.R", "Eye Control",
            "Skn_M_Mouth", "BdyMouth.L", "BdyMouth.R", "Bdy_M_Mouth", "PTMouth.L", "PTMouth.R", "PT_M_Mouth",
            "Mouth_A", "Mouth_B", "Mouth_C", "Ctr_Up_Teeth", "Ctr_Down_Teeth", "Bn_MouthControl_L",
            "Bn_MouthControl_R", "Bn_MouthControl_M", "SknEyebrow_01.L", "SknEyebrow_02.L", "SknEyebrow_03.L",
            "SknEyebrow_01.R", "SknEyebrow_02.R", "SknEyebrow_03.R", "SknMouth.R", "SknMouth.L",
            "eyetrack", "eyetrack_L", "eyetrack_R", "EyeTrack", "EyeTrack_L", "EyeTrack_R",
            "eyetrack.L", "eyetrack.R", "EyeTrack.L", "EyeTrack.R"
        ):
            _hb = armature.data.bones.get(_hn)
            if _hb:
                _hb.hide = True

        fwd, right, up, face_size = face_frame(faceobj, armature)
        controls = []
        if keyblock:
            controls = plan_controls(faceobj, fwd, right, up, face_size)

        # Also add bone-based mouth controls if no mouth shapekeys present
        has_mouth_sk = keyblock and any(k.startswith("Fac_Mth_") for k in keyblock.keys())
        if not has_mouth_sk:
            controls.extend(build_mouth_bone_controls(armature, fwd, up, right, face_size))

        # Add extra facial bone controls (highlights, face marks)
        controls.extend(build_extra_face_bone_controls(armature, fwd, up, right, face_size))

        if any(armature.data.bones.get(_bn) for _bn in ("Mouth_A", "Mouth_B", "Mouth_C", "Bn_MouthControl_L", "Bn_MouthControl_R", "Bn_MouthControl_M")):
            controls = [c for c in controls if not c['name'].startswith("CTRL-Mouth-Corner")]

        if any(any(_p in _b.name.lower() for _p in ("bdymouth", "bdy_m_mouth", "ptmouth", "pt_m_mouth")) for _b in armature.data.bones):
            for _cn, _frm, _to in (("CTRL-Mouth-Corner.L", "_L_", "_R_"),
                                   ("CTRL-Mouth-Corner.R", "_R_", "_L_")):
                _c = next((x for x in controls if x['name'] == _cn), None)
                if _c is not None:
                    for _d in _c['drivers']:
                        _d['key'] = _d['key'].replace(_frm, _to)
            _ms = next((x for x in controls if x['name'] == 'CTRL-Mouth-Shift'), None)
            if _ms is not None:
                for _d in _ms['drivers']:
                    if _d['axis'] == 'X':
                        _d['dir'] = -_d['dir']

        _eye_L, _eye_R = find_eye_bone_pair(armature)
        has_eye_bones = (_eye_L is not None and _eye_R is not None)

        if not controls and not has_eye_bones and not keyblock:
            print("[ZZZ Face Rig] No drivable shape keys or facial bones found.")
            return

        if CLEAN_REBUILD:
            purge_previous(armature)

        remove_eyetrack_bones(armature)
        if controls:
            setup_face_rig(faceobj, controls, armature, head_name, fwd, up, face_size)
        setup_lookat_eyes(armature, head_name, fwd, up, face_size)
        hide_mechanism_bones(armature)

        wgt_coll = get_widget_collection()
        finalize_widget_collection(wgt_coll)
        old_facerig_wgt = bpy.data.collections.get("WGTS_FaceRig")
        if old_facerig_wgt:
            finalize_widget_collection(old_facerig_wgt)

        # Build Face (Detail) panel
        if keyblock:
            try:
                setup_facerig_detail_panel(faceobj, armature, head_name, keyblock)
            except Exception as e_pnl:
                print(f"[ZZZ Face Rig Warning] Failed to setup detail panel: {e_pnl}")

        print("\nZZZ Face Rig complete (Face + Face (Detail)). %d controls built on '%s'.\n"
              % (len(controls), armature.name))

    finally:
        try:
            if bpy.context.object and bpy.context.object.mode != 'OBJECT':
                bpy.ops.object.mode_set(mode='OBJECT')
            bpy.ops.object.select_all(action='DESELECT')
        except Exception:
            pass


if __name__ == "__main__":
    zzz_face_rig_main()


```

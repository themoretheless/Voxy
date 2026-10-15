# Facial rendering source scan: met4citizen/TalkingHead

Repository: https://github.com/met4citizen/TalkingHead

Pinned source revision: b3e277b3b46f88e557bf28a2c5612a5b04e075c3

Discovery topic: lip sync 3D

Description: Talking Head (3D): A JavaScript class for real-time lip-sync using full-body 3D avatars.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["blender/VRoid/build-vroid-eyes.py", "blender/build-avatarsdk-eyes.py"], "blendshape": ["examples/azure-blendshapes.html"], "rig": ["images/rig.jpg"], "face": ["blender/Faceit/FACEIT.md", "blender/Faceit/build-visemes-from-faceit-phonemes.py"], "lip": ["modules/lipsync-de.mjs", "modules/lipsync-en.mjs", "modules/lipsync-fi.mjs", "modules/lipsync-fr.mjs", "modules/lipsync-lt.mjs", "modules/lipsync-pt_br.mjs"]}

## Source file: blender/build-avatarsdk-eyes.py

{
  "path": "blender/build-avatarsdk-eyes.py",
  "blob_sha": "34685b5f5090d5545865fc1bd198fe910eae369f",
  "sha256": "59c5a3b9bb98cfa12e5e10b6bdf29f97fe20eecefdb6aff82a3a2082b5606704",
  "lines": 123,
  "bytes": 3083,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 9,
    "deform": 1,
    "rig": 5,
    "lip": 2
  },
  "url": "https://github.com/met4citizen/TalkingHead/blob/b3e277b3b46f88e557bf28a2c5612a5b04e075c3/blender/build-avatarsdk-eyes.py",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "def ensure_basis(mesh):"
    },
    {
      "line": 42,
      "text": "def remove_shape_key(mesh, name):"
    },
    {
      "line": 51,
      "text": "def reset_pose(arm):"
    },
    {
      "line": 59,
      "text": "def set_bone_rotation(pbone, axis, degrees):"
    },
    {
      "line": 72,
      "text": "def save_armature_as_shape_key(mesh, shape_name):"
    },
    {
      "line": 89,
      "text": "def bake_eye(config):"
    }
  ]
}

```text

import bpy
import math

# CONFIGURATION
ARMATURE_NAME = "Armature"
LEFT = {
    "mesh": "AvatarLeftEyeball",
    "bone": "LeftEye",
    "side": "Left",
    "rotations": {
        "Up":   -20,
        "Down":  30,
        "In":   -25,
        "Out":   25,
    }
}
RIGHT = {
    "mesh": "AvatarRightEyeball",
    "bone": "RightEye",
    "side": "Right",
    "rotations": {
        "Up":   -20,
        "Down":  30,
        "In":    25,   # flipped
        "Out":  -25,   # flipped
    }
}
UP_DOWN_AXIS = "X"
IN_OUT_AXIS = "Y"
OVERRIDE_EXISTING = True   # Set False if you want to keep existing keys

# UTILITIES
def ensure_basis(mesh):
    if not mesh.data.shape_keys:
        mesh.shape_key_add(name="Basis", from_mix=False)
        return
    keys = mesh.data.shape_keys.key_blocks
    if "Basis" not in keys:
        mesh.shape_key_add(name="Basis", from_mix=False)


def remove_shape_key(mesh, name):
    keys = mesh.data.shape_keys
    if not keys:
        return
    kb = keys.key_blocks.get(name)
    if kb:
        mesh.shape_key_remove(kb)


def reset_pose(arm):
    for pbone in arm.pose.bones:
        pbone.location = (0.0, 0.0, 0.0)
        pbone.rotation_mode = 'XYZ'
        pbone.rotation_euler = (0.0, 0.0, 0.0)
        pbone.scale = (1.0, 1.0, 1.0)


def set_bone_rotation(pbone, axis, degrees):
    radians = math.radians(degrees)
    pbone.rotation_mode = 'XYZ'
    pbone.rotation_euler = (0.0, 0.0, 0.0)

    if axis == "X":
        pbone.rotation_euler.x = radians
    elif axis == "Y":
        pbone.rotation_euler.y = radians
    elif axis == "Z":
        pbone.rotation_euler.z = radians


def save_armature_as_shape_key(mesh, shape_name):
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode='OBJECT')

    for mod in mesh.modifiers:
        if mod.type == 'ARMATURE' and mod.show_viewport:
            bpy.ops.object.modifier_apply_as_shapekey(
                modifier=mod.name,
                keep_modifier=True
            )
            mesh.data.shape_keys.key_blocks[-1].name = shape_name
            return

    raise RuntimeError(f"No active Armature modifier on {mesh.name}")


# MAIN BAKE FUNCTION
def bake_eye(config):
    arm = bpy.data.objects[ARMATURE_NAME]
    mesh = bpy.data.objects[config["mesh"]]
    pbone = arm.pose.bones[config["bone"]]

    # Ensure Basis exists
    ensure_basis(mesh)

    for direction, angle in config["rotations"].items():
        shape_name = f"eyeLook{direction}{config['side']}"

        # Override existing shape key if requested
        if OVERRIDE_EXISTING:
            remove_shape_key(mesh, shape_name)

        # Reset all bones, then pose this eye only
        reset_pose(arm)

        if direction in ("Up", "Down"):
            set_bone_rotation(pbone, UP_DOWN_AXIS, angle)
        else:
            set_bone_rotation(pbone, IN_OUT_AXIS, angle)

        bpy.context.view_layer.update()

        # Bake deformation into a new shape key
        save_armature_as_shape_key(mesh, shape_name)

    # Final cleanup
    reset_pose(arm)


# EXECUTION
bake_eye(LEFT)
bake_eye(RIGHT)

```

## Source file: blender/VRoid/build-vroid-eyes.py

{
  "path": "blender/VRoid/build-vroid-eyes.py",
  "blob_sha": "2aff5218039f40674124a9e88886fd10b5e37228",
  "sha256": "989e874fcfe98ece92356ff5a3e338d96371d91e85ce525d8333b89ae1a24873",
  "lines": 113,
  "bytes": 2888,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 7,
    "deform": 1,
    "rig": 4,
    "face": 2
  },
  "url": "https://github.com/met4citizen/TalkingHead/blob/b3e277b3b46f88e557bf28a2c5612a5b04e075c3/blender/VRoid/build-vroid-eyes.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def ensure_basis(mesh):"
    },
    {
      "line": 32,
      "text": "def remove_shape_key(mesh, name):"
    },
    {
      "line": 41,
      "text": "def reset_pose(arm):"
    },
    {
      "line": 49,
      "text": "def set_bone_rotation(pbone, axis, degrees):"
    },
    {
      "line": 62,
      "text": "def save_armature_as_shape_key(mesh, shape_name):"
    },
    {
      "line": 79,
      "text": "def bake_eye(config):"
    }
  ]
}

```text

import bpy
import math

# CONFIGURATION
ARMATURE_NAME = "Armature"
LEFT = {
    "mesh": "Face",
    "bone": "LeftEye",
    "side": "Left",
    "rotations": { "Up": -5, "Down": 5, "In": 5, "Out": -9 }
}
RIGHT = {
    "mesh": "Face",
    "bone": "RightEye",
    "side": "Right",
    "rotations": { "Up": -5, "Down": 5, "In": -5, "Out": 9 }
}
UP_DOWN_AXIS = "X"
IN_OUT_AXIS = "Z"
OVERRIDE_EXISTING = True

# UTILITIES
def ensure_basis(mesh):
    if not mesh.data.shape_keys:
        mesh.shape_key_add(name="Basis", from_mix=False)
        return
    keys = mesh.data.shape_keys.key_blocks
    if "Basis" not in keys:
        mesh.shape_key_add(name="Basis", from_mix=False)


def remove_shape_key(mesh, name):
    keys = mesh.data.shape_keys
    if not keys:
        return
    kb = keys.key_blocks.get(name)
    if kb:
        mesh.shape_key_remove(kb)


def reset_pose(arm):
    for pbone in arm.pose.bones:
        pbone.location = (0.0, 0.0, 0.0)
        pbone.rotation_mode = 'XYZ'
        pbone.rotation_euler = (0.0, 0.0, 0.0)
        pbone.scale = (1.0, 1.0, 1.0)


def set_bone_rotation(pbone, axis, degrees):
    radians = math.radians(degrees)
    pbone.rotation_mode = 'XYZ'
    pbone.rotation_euler = (0.0, 0.0, 0.0)

    if axis == "X":
        pbone.rotation_euler.x = radians
    elif axis == "Y":
        pbone.rotation_euler.y = radians
    elif axis == "Z":
        pbone.rotation_euler.z = radians


def save_armature_as_shape_key(mesh, shape_name):
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode='OBJECT')

    for mod in mesh.modifiers:
        if mod.type == 'ARMATURE' and mod.show_viewport:
            bpy.ops.object.modifier_apply_as_shapekey(
                modifier=mod.name,
                keep_modifier=True
            )
            mesh.data.shape_keys.key_blocks[-1].name = shape_name
            return

    raise RuntimeError(f"No active Armature modifier on {mesh.name}")


# MAIN BAKE FUNCTION
def bake_eye(config):
    arm = bpy.data.objects[ARMATURE_NAME]
    mesh = bpy.data.objects[config["mesh"]]
    pbone = arm.pose.bones[config["bone"]]

    # Ensure Basis exists
    ensure_basis(mesh)

    for direction, angle in config["rotations"].items():
        shape_name = f"eyeLook{direction}{config['side']}"

        # Override existing shape key if requested
        if OVERRIDE_EXISTING:
            remove_shape_key(mesh, shape_name)

        # Reset all bones, then pose this eye only
        reset_pose(arm)

        if direction in ("Up", "Down"):
            set_bone_rotation(pbone, UP_DOWN_AXIS, angle)
        else:
            set_bone_rotation(pbone, IN_OUT_AXIS, angle)

        bpy.context.view_layer.update()

        # Bake deformation into a new shape key
        save_armature_as_shape_key(mesh, shape_name)

    # Final cleanup
    reset_pose(arm)


# EXECUTION
bake_eye(LEFT)
bake_eye(RIGHT)

```

## Source file: blender/Faceit/build-visemes-from-faceit-phonemes.py

{
  "path": "blender/Faceit/build-visemes-from-faceit-phonemes.py",
  "blob_sha": "7102e03bb12d8b4374906c1086e800b9f82c7446",
  "sha256": "eb829f1b8ede27103c352e640e5c3b68a69983bb58706aac29be643faa7beddb",
  "lines": 93,
  "bytes": 3036,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1
  },
  "url": "https://github.com/met4citizen/TalkingHead/blob/b3e277b3b46f88e557bf28a2c5612a5b04e075c3/blender/Faceit/build-visemes-from-faceit-phonemes.py",
  "function_signature_candidates": [
    {
      "line": 54,
      "text": "def traverse(x):"
    },
    {
      "line": 61,
      "text": "def hasShapekeys(x):"
    }
  ]
}

```text

import bpy

# Override existing visemes?
override = False

# Oculus viseme blendshapes
shapekeys = [
    { "name": "viseme_aa", "mix": [
        { "name": "aa_02", "value": 1.0 }
    ]},
    { "name": "viseme_E", "mix": [
        { "name": "ey_eh_uh_04", "value": 1.0 }
    ]},
    { "name": "viseme_I", "mix": [
        { "name": "y_iy_ih_ix_06", "value": 1.0 }
    ]},
    { "name": "viseme_O", "mix": [
        { "name": "ow_08", "value": 1.0 },
    ]},
    { "name": "viseme_U", "mix": [
        { "name": "w_uw_07", "value": 1.0 }
    ]},
    { "name": "viseme_PP", "mix": [
        { "name": "p_b_m_21", "value": 1.0 }
    ]},
    { "name": "viseme_FF", "mix": [
        { "name": "f_v_18", "value": 1.0 }
    ]},
    { "name": "viseme_DD", "mix": [
        { "name": "d_t_n_19", "value": 1.0 }
    ]},
    { "name": "viseme_SS", "mix": [
        { "name": "s_z_15", "value": 1.0 }
    ]},
    { "name": "viseme_TH", "mix": [
        { "name": "th_dh_17", "value": 1.0 }
    ]},
    { "name": "viseme_CH", "mix": [
        { "name": "sh_ch_jh_zh_16", "value": 1.0 }
    ]},
    { "name": "viseme_RR", "mix": [
        { "name": "r_13", "value": 1.0 }
    ]},
    { "name": "viseme_kk", "mix": [
        { "name": "k_g_ng_20", "value": 1.0 }
    ]},
    { "name": "viseme_nn", "mix": [
        { "name": "d_t_n_19", "value": 1.0 }
    ]},
    { "name": "viseme_sil", "mix": [] }
]

# Recursive traverse
def traverse(x):
    yield x
    if hasattr(x, 'children'):
        for c in x.children:
            yield from traverse(c)

# Has shape keys
def hasShapekeys(x):
    return hasattr(x, 'data') and hasattr(x.data, 'shape_keys') and hasattr(x.data.shape_keys, 'key_blocks')

# Build missing blend shapes
for r in bpy.context.scene.objects:
    for o in traverse(r):
        if hasShapekeys(o):
            keys = o.data.shape_keys.key_blocks
            for b in shapekeys:
                name = b["name"]
                mix = b["mix"]
                # Override
                if override:
                    if not keys.get(name) is None:
                        o.shape_key_remove(keys.get(name))
                # Check/verify the extra doesn't already exist
                if keys.get(name) is None:
                    # Check if some component exists
                    for m in mix:
                        if not keys.get(m["name"]) is None:
                            # Reset all shapes
                            for k in keys:
                                k.value = 0
                            # Create a mixed shape
                            for m in mix:
                                if not keys.get(m["name"]) is None:
                                    keys.get(m["name"]).value = m["value"]
                            # Create a new shape key from the mix
                            o.shape_key_add(name=name,from_mix=True)
                            # Reset all shapes
                            for k in keys:
                                k.value = 0
                            break


```

## Source file: siteconfig.js

{
  "path": "siteconfig.js",
  "blob_sha": "01ce46ea6174d84d0287c5f623d100c96cb462c8",
  "sha256": "65bbac2959895e3cef7c613d5a409162d2bb42e61a822b31635a2c3012818458",
  "lines": 261,
  "bytes": 9768,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 9,
    "rig": 8
  },
  "url": "https://github.com/met4citizen/TalkingHead/blob/b3e277b3b46f88e557bf28a2c5612a5b04e075c3/siteconfig.js",
  "function_signature_candidates": []
}

```text

// Site configuration
export const site = {

  // Preset avatars
  avatars: {
    'Brunette': {
      url: './avatars/brunette.glb',
      body: 'F',
      avatarMood: 'neutral',
      fi: 'Brunetti'
},

    'MPFB': {
      url: './avatars/mpfb.glb',
      body: 'F',
      avatarMood: 'happy',
      baseline: {},
      modelDynamicBones: [
        {
          "bone":"LeftBreast",
          "type":"point",
          "stiffness":600,
          "damping":20,
          "deltaLocal":[0,0.01,0],
          "deltaWorld":[0,-0.02,0],
          "limits":[[-0.02,0.02],[-0.02,0.06],[-0.02,0.02],[-0.5,0.5]]
        },
        {
          "bone":"RightBreast",
          "type":"point",
          "stiffness":600,
          "damping":20,
          "deltaLocal":[0,0.01,0],
          "deltaWorld":[0,-0.02,0],
          "limits":[[-0.02,0.02],[-0.02,0.06],[-0.02,0.02],[-0.5,0.5]]
        },
        {
          "bone":"Ponytail1",
          "type":"mix2",
          "stiffness":100,
          "damping":4,
          "limits":[null,null,[null,0.02],null],
          "pivot":true
        },
        {
          "bone":"Ponytail2",
          "type":"mix2",
          "stiffness":150,
          "damping":4,
          "limits":[null,null,[null,0.01],null]
        },
        {
          "bone":"Ponytail3",
          "type":"mix2",
          "stiffness":200,
          "damping":4,
          "limits":[null,null,[null,0.01],null]
        }
      ],
      baseline: {
        headRotateX: -0.01,
        eyeBlinkLeft: 0.05,
        eyeBlinkRight: 0.05
      }
    },

    'Avaturn': {
      url: './avatars/avaturn.glb',
      body: 'F',
      avatarMood: 'happy',
      retarget: {
        Hips: { y: 0.03 }, Spine: { y: 0.02 }, Spine1: { y: 0.02, z: 0.01 },
        Spine2: { y: 0.02, z: 0.01 }, Neck: { z: 0.02, y: 0.01 }, Head: { z: 0.02 },
        LeftShoulder: { rx: -0.5 }, RightShoulder: { rx: -0.5 },
        scaleToHipsLevel: 1.0
      },
      baseline: {
        headRotateX: -0.05,
        eyeBlinkLeft: 0.15,
        eyeBlinkRight: 0.15
      }
    },

    'AvatarSDK': {
      url: './avatars/avatarsdk.glb',
      body: 'M',
      avatarMood: 'neutral',
      retarget: {
        Neck: { z: -0.01, rx: -0.15 }, Neck1: { z: -0.01, rx: -0.15 }, Neck2: { z: -0.01, rx: -0.15 },
        LeftShoulder: { rz: -0.3 }, RightShoulder: { rz: 0.3 },
        scaleToEyesLevel: 1.0, origin: { y: -0.1 }
      },
      baseline: {
        headRotateX: -0.04,
        eyeBlinkLeft: 0.05,
        eyeBlinkRight: 0.05
      }
    },

    'VRoid': {
      url: './avatars/vroid.glb',
      body: 'F',
      avatarMood: 'neutral',
      baseline: {
        headRotateX: -0.1,
        eyeBlinkLeft: 0.05,
        eyeBlinkRight: 0.05
      },
      modelDynamicBones: [
        {
          "bone":"J_Sec_L_Bust1",
          "type":"point",
          "stiffness":400,
          "damping":10,
          "deltaLocal":[0,0.01,0],
          "deltaWorld":[0,-0.02,0],
          "limits":[[-0.01,0.01],[-0.01,0.01],[-0.01,0.01],[-0.01,0.01]]
        },
        {
          "bone":"J_Sec_R_Bust1",
          "type":"point",
          "stiffness":400,
          "damping":10,
          "deltaLocal":[0,0.01,0],
          "deltaWorld":[0,-0.02,0],
          "limits":[[-0.01,0.01],[-0.01,0.01],[-0.01,0.01],[-0.01,0.01]]
        },
        {"bone":"J_Sec_Hair2_01","type":"mix2","stiffness":100,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair3_01","type":"mix2","stiffness":150,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair2_02","type":"mix2","stiffness":100,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair3_02","type":"mix2","stiffness":150,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair2_03","type":"mix2","stiffness":100,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair3_03","type":"mix2","stiffness":150,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair2_04","type":"mix2","stiffness":100,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair3_04","type":"mix2","stiffness":150,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair2_05","type":"mix2","stiffness":100,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair3_05","type":"mix2","stiffness":150,"damping":4,"limits":[null,null,[null,0.01],null]},
        {"bone":"J_Sec_Hair2_06","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_06","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair3_06_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_07","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_07","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair3_07_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_08","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_08","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair4_08","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair4_08_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_09","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_09","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair4_09","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair4_09_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_10","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_10","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair3_10_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_11","type":"mix2","stiffness":100,"damping":4},
        {"bone":"J_Sec_Hair3_11","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair3_11_end","type":"mix2","stiffness":200,"damping":4},
        {"bone":"J_Sec_Hair2_12","type":"mix2","stiffness":100,"damping":4,"pivot":true},
        {"bone":"J_Sec_Hair3_12","type":"mix2","stiffness":150,"damping":4},
        {"bone":"J_Sec_Hair2_13","type":"mix2","stiffness":100,"damping":4,"pivot":true},
        {"bone":"J_Sec_Hair3_13","type":"mix2","stiffness":150,"damping":4,},
      ]
    }

  },

  // Google voices
  googleVoices: {
    "fi-F": { id: "fi-FI-Standard-A" },
    "lv-M": { id: "lv-LV-Standard-A" },
    "lt-M": { id: "lt-LT-Standard-A" },
    "en-F": { id: "en-GB-Standard-A" },
    "en-M": { id: "en-GB-Standard-D" },
    "fr-F": { id: "fr-FR-Standard-A" },
    "fr-M": { id: "fr-FR-Standard-B" },
    "de-F": { id: "de-DE-Standard-G" },
    "de-M": { id: "de-DE-Standard-H" },
    "pt-F": { id: "pt-BR-Standard-A" },
    "pt-M": { id: "pt-BR-Standard-B" },
  },

  // ElevenLab voices
  elevenVoices: {
    "Bella": { id: "EXAVITQu4vr4xnSDxMaL" },
    "Elli": { id: "MF3mGyEYCl7XYWbV9V6O" },
    "Rachel": { id: "21m00Tcm4TlvDq8ikWAM" },
    "Adam": { id: "pNInz6obpgDQGcFmaJgB" },
    "Antoni": { id: "ErXwobaYiN019PkySvjV" },
    "Arnold": { id: "VR6AewLTigWG4xSOukaG" },
    "Domi": { id: "AZnzlk1XvdvUeBnXmlld" },
    "Josh": { id: "TxGEqnHWrfWFTfGW9XjX" },
    "Sam": { id: "yoZ06aMxZJJ28mfd3POQ" }
  },

  // Microsoft voices
  microsoftVoices: {
    "fi-Selma": { lang: "fi-FI", id: "fi-FI-SelmaNeural" },
    "fi-Noora": { lang: "fi-FI", id: "fi-FI-NooraNeural" },
    "fi-Harri": { lang: "fi-FI", id: "fi-FI-HarriNeural" },
    "en-Jenny": { lang: "en-US", id: "en-US-JennyNeural" },
    "en-Tony": { lang: "en-US", id: "en-US-TonyNeural" },
    "fr-Vivienne": { lang: "fr-FR", id: "fr-FR-VivienneMultilingualNeural" },
    "fr-Lucien": { lang: "fr-FR", id: "fr-FR-LucienMultilingualNeural" },
    "de-Seraphina": { lang: "de-DE", id: "de-DE-SeraphinaMultilingualNeural" },
    "de-Florian": { lang: "de-DE", id: "de-DE-FlorianMultilingualNeural" },
    "pt-Francisca": { lang: "pt-BR", id: "pt-BR-FranciscaNeural" },
    "pt-Antonio": { lang: "pt-BR", id: "pt-BR-AntonioNeural" },
  },

  // Preset views
  views: {
    'DrStrange': { url: './views/strange.jpg', type: 'image/jpg', fi: 'TohtoriOuto' },
    'Matrix': { url: './views/matrix.mp4', type: 'video/mp4' }
  },

  // Preset poses (includes internal poses)
  poses: {
    'Straight': { url: "straight", fi: 'Suora' },
    'Side': { url: "side", fi: 'Keno' },
    'Hip': { url: "hip", fi: 'Lantio' },
    'Turn': { url: "turn", fi: 'Sivu' },
    'Back': { url: "back", fi: 'Taka' },
    'Wide': { url: "wide", fi: 'Haara' },
    'OneKnee': { url: "oneknee", fi: 'Polvi' },
    'TwoKnees': { url: "kneel", fi: 'Polvet' },
    'Bend': { url: "bend", fi: 'Perä' },
    'Sitting': { url:"sitting", fi: 'Istuva' },
    'Dance': { url: './poses/dance.fbx', fi: 'Tanssi' }
  },

  // Preset gestures
  gestures: {
    'HandUp': { name: "handup", fi: 'KäsiYlös' },
    'OK': { name: "ok" },
    'Index': { name: "index", fi: 'Etusormi' },
    'ThumbUp': { name: "thumbup", fi: 'PeukaloYlös' },
    'ThumbDown': { name: "thumbdown", fi: 'PeukaloAlas' },
    'Side': { name: "side", fi: 'Sivu' },
    'Shrug': { name: "shrug", fi: 'Olankohautus' },
    'Namaste': { name: "namaste" }
  },

  // Preset animations
  animations: {
    'Walking': { url: './animations/walking.fbx', fi: 'Kävely' }
  },

  // Impulse responses
  impulses: {
    'Room': { url: './audio/ir-room.m4a', fi: 'Huone' },
    'Basement': { url: './audio/ir-basement.m4a', fi: 'Kellari' },
    'Forest': { url: './audio/ir-forest.m4a', fi: 'Metsä' },
    'Church': { url: './audio/ir-church.m4a', fi: 'Kirkko' }
  },

  // Background ambient sounds/music
  music: {
    'Murmur': { url: './audio/murmur.mp3', fi: 'Puheensorina'}
  }

};


```

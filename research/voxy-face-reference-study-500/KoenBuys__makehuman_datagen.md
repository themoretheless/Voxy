# Facial rendering source scan: KoenBuys/makehuman_datagen

Repository: https://github.com/KoenBuys/makehuman_datagen

Pinned source revision: 3cb5b218b2f2821de66e7a3a86e2703c6045e542

Discovery topic: MakeHuman

Description: Clone of the MakeHuman SVN repo to add custom DataGen code

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["license.txt", "utils/maketarget/license.txt"]

Topic paths: {"eye": ["data/eyes/brown-left.png", "data/eyes/brown-right.png", "data/eyes/brown.mhstx", "data/eyes/green-left.png", "data/eyes/green-right.png", "data/eyes/green.mhstx", "data/eyes/notfound.thumb", "data/shaders/aqsis/eyeball.sl", "data/targets/asym/asym-eye-1-l.target", "data/targets/asym/asym-eye-1-r.target", "data/targets/asym/asym-eye-2-l.target", "data/targets/asym/asym-eye-2-r.target", "data/targets/asym/asym-eye-3-l.target", "data/targets/asym/asym-eye-3-r.target", "data/targets/asym/asym-eye-4-l.target", "data/targets/asym/asym-eye-4-r.target", "data/targets/asym/asym-eye-5-l.target", "data/targets/asym/asym-eye-5-r.target", "data/targets/asym/asym-eye-6-l.target", "data/targets/asym/asym-eye-6-r.target", "data/targets/asym/asym-eye-7-l.target", "data/targets/asym/asym-eye-7-r.target", "data/targets/asym/asym-eye-8-l.target", "data/targets/asym/asym-eye-8-r.target", "data/targets/expression/units/african/female_child/eye-left-closure.target", "data/targets/expression/units/african/female_child/eye-left-opened-up.target", "data/targets/expression/units/african/female_child/eye-left-slit.target", "data/targets/expression/units/african/female_child/eye-right-closure.target", "data/targets/expression/units/african/female_child/eye-right-opened-up.target", "data/targets/expression/units/african/female_child/eye-right-slit.target"], "skin": ["data/shaders/aqsis/skin.sl", "data/shaders/aqsis/skin2.sl", "data/shaders/aqsis/skinbump.sl", "data/shaders/glsl/skin_fragment_shader.txt", "data/shaders/glsl/skin_vertex_shader.txt", "data/shaders/unused/skinbump.sl", "data/skins/bodyparts.png", "data/skins/bodyparts.thumb", "data/skins/notfound.thumb"], "sss": ["data/povray/staticcontent_mesh2only_fsss.inc"], "jaw": ["data/targets/asym/asym-jaw-1-l.target", "data/targets/asym/asym-jaw-1-r.target", "data/targets/asym/asym-jaw-2-l.target", "data/targets/asym/asym-jaw-2-r.target", "data/targets/asym/asym-jaw-3-l.target", "data/targets/asym/asym-jaw-3-r.target", "data/targets/microdetails/jaw-chin-scale-depth-decr.target", "data/targets/microdetails/jaw-chin-scale-depth-incr.target", "data/targets/microdetails/jaw-chin-scale-horiz-decr.target", "data/targets/microdetails/jaw-chin-scale-horiz-incr.target", "data/targets/microdetails/jaw-chin-scale-vert-decr.target", "data/targets/microdetails/jaw-chin-scale-vert-incr.target", "data/targets/microdetails/jaw-chin-trans-backward.target", "data/targets/microdetails/jaw-chin-trans-down.target", "data/targets/microdetails/jaw-chin-trans-forward.target", "data/targets/microdetails/jaw-chin-trans-in.target", "data/targets/microdetails/jaw-chin-trans-out.target", "data/targets/microdetails/jaw-chin-trans-up.target", "data/targets/microdetails/jaw-lower-chin-scale-depth-decr.target", "data/targets/microdetails/jaw-lower-chin-scale-depth-incr.target", "data/targets/microdetails/jaw-lower-chin-scale-horiz-decr.target", "data/targets/microdetails/jaw-lower-chin-scale-horiz-incr.target", "data/targets/microdetails/jaw-lower-chin-scale-vert-decr.target", "data/targets/microdetails/jaw-lower-chin-scale-vert-incr.target", "data/targets/microdetails/jaw-lower-chin-trans-backward.target", "data/targets/microdetails/jaw-lower-chin-trans-down.target", "data/targets/microdetails/jaw-lower-chin-trans-forward.target", "data/targets/microdetails/jaw-lower-chin-trans-in.target", "data/targets/microdetails/jaw-lower-chin-trans-out.target", "data/targets/microdetails/jaw-lower-chin-trans-up.target"], "mouth": ["data/targets/asym/asym-mouth-1-l.target", "data/targets/asym/asym-mouth-1-r.target", "data/targets/asym/asym-mouth-2-l.target", "data/targets/asym/asym-mouth-2-r.target", "data/targets/expression/units/african/female_child/mouth-compression.target", "data/targets/expression/units/african/female_child/mouth-corner-puller.target", "data/targets/expression/units/african/female_child/mouth-depression-retraction.target", "data/targets/expression/units/african/female_child/mouth-depression.target", "data/targets/expression/units/african/female_child/mouth-elevation.target", "data/targets/expression/units/african/female_child/mouth-eversion.target", "data/targets/expression/units/african/female_child/mouth-open.target", "data/targets/expression/units/african/female_child/mouth-parling.target", "data/targets/expression/units/african/female_child/mouth-part-later.target", "data/targets/expression/units/african/female_child/mouth-protusion.target", "data/targets/expression/units/african/female_child/mouth-pursing.target", "data/targets/expression/units/african/female_child/mouth-retraction.target", "data/targets/expression/units/african/female_child/mouth-upward-retraction.target", "data/targets/expression/units/african/female_old/mouth-compression.target", "data/targets/expression/units/african/female_old/mouth-corner-puller.target", "data/targets/expression/units/african/female_old/mouth-depression-retraction.target", "data/targets/expression/units/african/female_old/mouth-depression.target", "data/targets/expression/units/african/female_old/mouth-elevation.target", "data/targets/expression/units/african/female_old/mouth-eversion.target", "data/targets/expression/units/african/female_old/mouth-open.target", "data/targets/expression/units/african/female_old/mouth-parling.target", "data/targets/expression/units/african/female_old/mouth-part-later.target", "data/targets/expression/units/african/female_old/mouth-protusion.target", "data/targets/expression/units/african/female_old/mouth-pursing.target", "data/targets/expression/units/african/female_old/mouth-retraction.target", "data/targets/expression/units/african/female_old/mouth-upward-retraction.target"], "teeth": ["data/shaders/aqsis/teeth.sl"], "tongue": ["shared/mhx/targets/body_language/female-young/tongue_back_height.target", "shared/mhx/targets/body_language/female-young/tongue_depth.target", "shared/mhx/targets/body_language/female-young/tongue_height.target", "shared/mhx/targets/body_language/female-young/tongue_width.target"], "wrinkle": ["shared/mhx/targets/body_language/female-young/nose_wrinkle.target"], "deform": ["shared/io_fbx/fbx_deformer.py"], "cornea": ["data/shaders/aqsis/cornea.sl"], "shader": ["data/shaders/aqsis/bakelightmap.sl", "data/shaders/aqsis/cornea.sl", "data/shaders/aqsis/envlight.sl", "data/shaders/aqsis/eyeball.sl", "data/shaders/aqsis/hair.sl", "data/shaders/aqsis/hairpoly.sl", "data/shaders/aqsis/mixer.sl", "data/shaders/aqsis/occlmap.rib", "data/shaders/aqsis/scatteringtexture.sl", "data/shaders/aqsis/shadowspot.sl", "data/shaders/aqsis/skin.sl", "data/shaders/aqsis/skin2.sl", "data/shaders/aqsis/skinbump.sl", "data/shaders/aqsis/teeth.sl", "data/shaders/aqsis/textureonly.sl", "data/shaders/glsl/phong_fragment_shader.txt", "data/shaders/glsl/phong_vertex_shader.txt", "data/shaders/glsl/skin_fragment_shader.txt", "data/shaders/glsl/skin_vertex_shader.txt", "data/shaders/glsl/toon_fragment_shader.txt", "data/shaders/glsl/toon_vertex_shader.txt", "data/shaders/unused/onlytexture.sl", "data/shaders/unused/skinbump.sl", "lib/shader.py", "plugins/5_settings_shaders.py"], "rig": ["data/eyes/brown-right.png", "data/eyes/green-right.png", "data/fonts/Ubuntu-copyright.txt", "data/hairstyles/long2/long2.rig", "data/icons/right.png", "data/rigs/game.rig", "data/rigs/second_life.rig", "data/rigs/simple.rig", "data/rigs/soft1.rig", "data/rigs/xonotic.rig", "data/targets/expression/units/african/female_child/eye-right-closure.target", "data/targets/expression/units/african/female_child/eye-right-opened-up.target", "data/targets/expression/units/african/female_child/eye-right-slit.target", "data/targets/expression/units/african/female_child/eyebrows-right-down.target", "data/targets/expression/units/african/female_child/eyebrows-right-extern-up.target", "data/targets/expression/units/african/female_child/eyebrows-right-inner-up.target", "data/targets/expression/units/african/female_child/eyebrows-right-up.target", "data/targets/expression/units/african/female_child/nose-right-dilatation.target", "data/targets/expression/units/african/female_child/nose-right-elevation.target", "data/targets/expression/units/african/female_old/eye-right-closure.target", "data/targets/expression/units/african/female_old/eye-right-opened-up.target", "data/targets/expression/units/african/female_old/eye-right-slit.target", "data/targets/expression/units/african/female_old/eyebrows-right-down.target", "data/targets/expression/units/african/female_old/eyebrows-right-extern-up.target", "data/targets/expression/units/african/female_old/eyebrows-right-inner-up.target", "data/targets/expression/units/african/female_old/eyebrows-right-up.target", "data/targets/expression/units/african/female_old/nose-right-dilatation.target", "data/targets/expression/units/african/female_old/nose-right-elevation.target", "data/targets/expression/units/african/female_young/eye-right-closure.target", "data/targets/expression/units/african/female_young/eye-right-opened-up.target"], "face": ["data/icons/face.png", "data/landmarks/face.lmk", "data/povray/makehuman_facegroup_documentation.pov", "data/themes/makehuman/icons/face.png", "shared/mhx/rig_face_25.py", "tools/blender26x/makeface/__init__.py", "tools/blender26x/makeface/makeface.py", "tools/blender26x/makeface/mask.mhclo", "tools/blender26x/makeface/mask.obj", "tools/blender26x/mh_utils/landmarks/face.txt"], "lip": ["data/targets/microdetails/l-mouth-lower-lip-scale-depth-decr.target", "data/targets/microdetails/l-mouth-lower-lip-scale-depth-incr.target", "data/targets/microdetails/l-mouth-lower-lip-scale-horiz-decr.target", "data/targets/microdetails/l-mouth-lower-lip-scale-horiz-incr.target", "data/targets/microdetails/l-mouth-lower-lip-scale-vert-decr.target", "data/targets/microdetails/l-mouth-lower-lip-scale-vert-incr.target", "data/targets/microdetails/l-mouth-lower-lip-trans-backward.target", "data/targets/microdetails/l-mouth-lower-lip-trans-down.target", "data/targets/microdetails/l-mouth-lower-lip-trans-forward.target", "data/targets/microdetails/l-mouth-lower-lip-trans-in.target", "data/targets/microdetails/l-mouth-lower-lip-trans-out.target", "data/targets/microdetails/l-mouth-lower-lip-trans-up.target", "data/targets/microdetails/l-mouth-upper-lip-scale-depth-decr.target", "data/targets/microdetails/l-mouth-upper-lip-scale-depth-incr.target", "data/targets/microdetails/l-mouth-upper-lip-scale-horiz-decr.target", "data/targets/microdetails/l-mouth-upper-lip-scale-horiz-incr.target", "data/targets/microdetails/l-mouth-upper-lip-scale-vert-decr.target", "data/targets/microdetails/l-mouth-upper-lip-scale-vert-incr.target", "data/targets/microdetails/l-mouth-upper-lip-trans-backward.target", "data/targets/microdetails/l-mouth-upper-lip-trans-down.target", "data/targets/microdetails/l-mouth-upper-lip-trans-forward.target", "data/targets/microdetails/l-mouth-upper-lip-trans-in.target", "data/targets/microdetails/l-mouth-upper-lip-trans-out.target", "data/targets/microdetails/l-mouth-upper-lip-trans-up.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-depth-decr.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-depth-incr.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-horiz-decr.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-horiz-incr.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-vert-decr.target", "data/targets/microdetails/mouth-lower-middle-lip-scale-vert-incr.target"]}

## Source file: shared/mhx/rig_face_25.py

{
  "path": "shared/mhx/rig_face_25.py",
  "blob_sha": "2e8fa37a8bdf6f4f4a187b2fe2602c2dd8fd796b",
  "sha256": "f77a8676422e15b75cd68eb85b75a2647814e309d0c90f4529c1650656a96c07",
  "lines": 152,
  "bytes": 5133,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 24,
    "jaw": 4,
    "mouth": 3,
    "tongue": 9,
    "deform": 2,
    "rig": 1,
    "face": 11
  },
  "url": "https://github.com/KoenBuys/makehuman_datagen/blob/3cb5b218b2f2821de66e7a3a86e2703c6045e542/shared/mhx/rig_face_25.py",
  "function_signature_candidates": [
    {
      "line": 86,
      "text": "def FaceControlPoses(fp, amt):"
    },
    {
      "line": 128,
      "text": "def FaceDeformDrivers(fp, amt):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

""" 
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    http://code.google.com/p/makehuman/

**Authors:**           Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2013

**Licensing:**         AGPL3 (see also http://www.makehuman.org/node/318)

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

Face bone definitions 
"""


from .flags import *
from . import posebone
from posebone import addPoseBone

FaceJoints = [
    ('head-end',        'l', ((2.0, 'head'), (-1.0, 'neck'))),
    ('r-mouth',         'v', 2490),
    ('l-mouth',         'v', 8907),

    ('eyes',            'l', ((0.5, 'l-eye'), (0.5,'r-eye'))),
    ('gaze',            'o', ('eyes', (0,0,5))),
]

eyeOffs = (0,0,0.3)

FaceHeadsTails = [
    ('Jaw',                  'mouth', 'jaw'),
    ('TongueBase',           'tongue-1', 'tongue-2'),
    ('TongueMid',            'tongue-2', 'tongue-3'),
    ('TongueTip',            'tongue-3', 'tongue-4'),

    ('Eye_R',                'l-eye', ('l-eye', eyeOffs)),
    ('EyeParent_R',          'l-eye', ('l-eye', eyeOffs)),
    ('DfmUpLid_R',           'l-eye', 'l-upperlid'),
    ('DfmLoLid_R',           'l-eye', 'l-lowerlid'),
    
    ('Eye_L',                'r-eye', ('r-eye', eyeOffs)),
    ('EyeParent_L',          'r-eye', ('r-eye', eyeOffs)),
    ('DfmUpLid_L',           'r-eye', 'r-upperlid'),
    ('DfmLoLid_L',           'r-eye', 'r-lowerlid'),

    ('Eyes',                 'eyes', ('eyes', (0,0,1))),
    ('Gaze',                 'gaze', ('gaze', (0,0,1))),
    ('GazeParent',           'neck2', 'head-end'),
]


FaceArmature = [
    ('Jaw',              0, 'Head', F_DEF, L_HEAD, NoBB),
    ('TongueBase',       0, 'Jaw', F_DEF, L_HEAD, NoBB),
    ('TongueMid',        0, 'TongueBase', F_DEF, L_HEAD, NoBB),
    ('TongueTip',        0, 'TongueMid', F_DEF, L_HEAD, NoBB),
    ('GazeParent',       0, 'MasterFloor', 0, L_HELP, NoBB),
    ('Gaze',             pi, 'GazeParent', 0, L_HEAD, NoBB),
    ('Eyes',             0, 'Head', 0, L_HELP, NoBB),
    ('EyeParent_R',      0, 'Head', 0, L_HELP, NoBB),
    ('EyeParent_L',      0, 'Head', 0, L_HELP, NoBB),
    ('Eye_R',            0, 'EyeParent_R', F_DEF, L_HEAD+L_DEF, NoBB),
    ('Eye_L',            0, 'EyeParent_L', F_DEF, L_HEAD+L_DEF, NoBB),
    ('DfmUpLid_R',       0.279253, 'Head', F_DEF, L_DEF, NoBB),
    ('DfmLoLid_R',       0, 'Head', F_DEF, L_DEF, NoBB),
    ('DfmUpLid_L',       -0.279253, 'Head', F_DEF, L_DEF, NoBB),
    ('DfmLoLid_L',       0, 'Head', F_DEF, L_DEF, NoBB),
]


#
#    FaceControlPoses(fp, amt):
#

def FaceControlPoses(fp, amt):
    addPoseBone(fp, amt, 'Jaw', 'MHJaw', None, (1,1,1), (0,1,0), (1,1,1), (1,1,1), 0,
         [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', (-5*D,45*D, 0,0, -20*D,20*D), (1,1,1)])])

    addPoseBone(fp, amt, 'TongueBase', None, None, (1,1,1), (0,1,0), (1,0,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'TongueMid', None, None, (1,1,1), (0,1,0), (1,0,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'TongueTip', None, None, (1,1,1), (0,1,0), (1,0,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'Gaze', 'MHGaze', None, (0,0,0), (1,1,1), (0,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'GazeParent', None, None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0,
         [('CopyTrans', 0, 1, ['Head', 'Head', 0])])

    addPoseBone(fp, amt, 'DfmUpLid_R', None, None, (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'DfmLoLid_R', None, None, (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'DfmUpLid_L', None, None, (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'DfmLoLid_L', None, None, (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'Eyes', None, None, (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'Gaze', 1, None, (True, False,False), 1.0])])

    addPoseBone(fp, amt, 'Eye_R', 'MHCircle025', None, (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'Eye_L', 'MHCircle025', None, (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'EyeParent_L', None, None, (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, 
        [('CopyRot', C_LOCAL, 1, ['Eyes', 'Eyes', (1,1,1), (0,0,0), True])])

    addPoseBone(fp, amt, 'EyeParent_R', None, None, (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, 
        [('CopyRot', C_LOCAL, 1, ['Eyes', 'Eyes', (1,1,1), (0,0,0), True])])

    return

#
#    FaceDeformDrivers(fp, amt):
#

def FaceDeformDrivers(fp, amt):
    lidBones = [
    ('DfmUpLid_L', 'PUpLid_L', (0, 40*D)),
    ('DfmLoLid_L', 'PLoLid_L', (0, 20*D)),
    ('DfmUpLid_R', 'PUpLid_R', (0, 40*D)),
    ('DfmLoLid_R', 'PLoLid_R', (0, 20*D)),
    ]

    drivers = []
    for (driven, driver, coeff) in lidBones:
        drivers.append(    (driven, 'ROTQ', 'AVERAGE', None, 1, coeff,
         [("var", 'TRANSFORMS', [('OBJECT', amt.name, driver, 'LOC_Z', C_LOC)])]) )
    return drivers

#
#   FacePropDrivers
#   (Bone, Name, Props, Expr)
#

FacePropDrivers = []

SoftFacePropDrivers = [
    ('GazeParent', 'Head', ['GazeFollowsHead'], 'x1'),
]



```

## Source file: lib/shader.py

{
  "path": "lib/shader.py",
  "blob_sha": "a5fdc8eda56c4bbf6e2ffc5bd21e709c77f17655",
  "sha256": "f9bc4623471e91536f1ea8a4d74b267f7509a3457ce5d3529224bfe25e6f141b",
  "lines": 382,
  "bytes": 17260,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 64,
    "rig": 1
  },
  "url": "https://github.com/KoenBuys/makehuman_datagen/blob/3cb5b218b2f2821de66e7a3a86e2703c6045e542/lib/shader.py",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "def __init__(self, index, name, pytype, dims):"
    },
    {
      "line": 40,
      "text": "def __call__(self, index, values):"
    },
    {
      "line": 43,
      "text": "def update(self, pgm):"
    },
    {
      "line": 93,
      "text": "def check(cls, type):"
    },
    {
      "line": 99,
      "text": "def __init__(self, index, name, type):"
    },
    {
      "line": 106,
      "text": "def set(self, data):"
    },
    {
      "line": 113,
      "text": "def update(self, pgm):"
    },
    {
      "line": 201,
      "text": "def check(cls, type):"
    },
    {
      "line": 207,
      "text": "def __init__(self, index, name, type):"
    },
    {
      "line": 212,
      "text": "def set(self, data):"
    },
    {
      "line": 222,
      "text": "def reset(cls):"
    },
    {
      "line": 229,
      "text": "def supported(cls):"
    },
    {
      "line": 234,
      "text": "def __init__(self, path):"
    },
    {
      "line": 250,
      "text": "def __del__(self):"
    },
    {
      "line": 257,
      "text": "def createShader(file, type):"
    },
    {
      "line": 269,
      "text": "def delete(self):"
    },
    {
      "line": 280,
      "text": "def initShader(self):"
    },
    {
      "line": 308,
      "text": "def getUniforms(self):"
    },
    {
      "line": 323,
      "text": "def updateUniforms(self):"
    },
    {
      "line": 327,
      "text": "def setUniforms(self, params):"
    },
    {
      "line": 337,
      "text": "def getShader(path, cache=None):"
    },
    {
      "line": 375,
      "text": "def reloadShaders():"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    http://code.google.com/p/makehuman/

**Authors:**           Glynn Clements

**Copyright(c):**      MakeHuman Team 2001-2013

**Licensing:**         AGPL3 (see also http://www.makehuman.org/node/318)

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

TODO
"""

import os.path
import numpy as np
from OpenGL.GL import *
from OpenGL.GL.ARB.texture_multisample import *
import texture
import log

class Uniform(object):
    def __init__(self, index, name, pytype, dims):
        self.index = index
        self.name = name
        self.pytype = pytype
        self.dims = dims
        self.values = None

    def __call__(self, index, values):
        raise NotImplementedError

    def update(self, pgm):
        pass

class VectorUniform(Uniform):
    uniformTypes = {
        GL_FLOAT:               ((1,),  np.float32,     float,  glUniform1fv,		glGetUniformfv),
        GL_FLOAT_VEC2:          ((2,),  np.float32,     float,  glUniform2fv,		glGetUniformfv),
        GL_FLOAT_VEC3:          ((3,),  np.float32,     float,  glUniform3fv,		glGetUniformfv),
        GL_FLOAT_VEC4:          ((4,),  np.float32,     float,  glUniform4fv,		glGetUniformfv),
        GL_INT:                 ((1,),  np.int32,       int,    glUniform1iv,		glGetUniformiv),
        GL_INT_VEC2:            ((2,),  np.int32,       int,    glUniform2iv,		glGetUniformiv),
        GL_INT_VEC3:            ((3,),  np.int32,       int,    glUniform3iv,		glGetUniformiv),
        GL_INT_VEC4:            ((4,),  np.int32,       int,    glUniform4iv,		glGetUniformiv),
        GL_UNSIGNED_INT:        ((1,),  np.uint32,      int,    glUniform1uiv,		glGetUniformuiv),
        GL_UNSIGNED_INT_VEC2:   ((2,),  np.uint32,      int,    glUniform2uiv,		glGetUniformuiv),
        GL_UNSIGNED_INT_VEC3:   ((3,),  np.uint32,      int,    glUniform3uiv,		glGetUniformuiv),
        GL_UNSIGNED_INT_VEC4:   ((4,),  np.uint32,      int,    glUniform4uiv,		glGetUniformuiv),
        GL_BOOL:                ((1,),  np.int32,       bool,   glUniform1iv,		glGetUniformiv),
        GL_BOOL_VEC2:           ((2,),  np.int32,       bool,   glUniform2iv,		glGetUniformiv),
        GL_BOOL_VEC3:           ((3,),  np.int32,       bool,   glUniform3iv,		glGetUniformiv),
        GL_BOOL_VEC4:           ((4,),  np.int32,       bool,   glUniform4iv,		glGetUniformiv),
        GL_FLOAT_MAT2:          ((2,2), np.float32,     float,  glUniformMatrix2fv,	glGetUniformfv),
        GL_FLOAT_MAT2x3:        ((2,3), np.float32,     float,  glUniformMatrix2x3fv,	glGetUniformfv),
        GL_FLOAT_MAT2x4:        ((2,4), np.float32,     float,  glUniformMatrix2x4fv,	glGetUniformfv),
        GL_FLOAT_MAT3x2:        ((3,2), np.float32,     float,  glUniformMatrix3x2fv,	glGetUniformfv),
        GL_FLOAT_MAT3:          ((3,3), np.float32,     float,  glUniformMatrix3fv,	glGetUniformfv),
        GL_FLOAT_MAT3x4:        ((3,4), np.float32,     float,  glUniformMatrix3x4fv,	glGetUniformfv),
        GL_FLOAT_MAT4x2:        ((4,2), np.float32,     float,  glUniformMatrix4x2fv,	glGetUniformfv),
        GL_FLOAT_MAT4x3:        ((4,3), np.float32,     float,  glUniformMatrix4x3fv,	glGetUniformfv),
        GL_FLOAT_MAT4:          ((4,4), np.float32,     float,  glUniformMatrix4fv,	glGetUniformfv),
        }

    if 'glUniform1dv' in globals():
        uniformTypes2 = {
            GL_DOUBLE:              ((1,),  np.float64,     float,  glUniform1dv,		glGetUniformdv),
            GL_DOUBLE_VEC2:         ((2,),  np.float64,     float,  glUniform2dv,		glGetUniformdv),
            GL_DOUBLE_VEC3:         ((3,),  np.float64,     float,  glUniform3dv,		glGetUniformdv),
            GL_DOUBLE_VEC4:         ((4,),  np.float64,     float,  glUniform4dv,		glGetUniformdv),
            GL_DOUBLE_MAT2:         ((2,2), np.float64,     float,  glUniformMatrix2dv,		glGetUniformdv),
            GL_DOUBLE_MAT2x3:       ((2,3), np.float64,     float,  glUniformMatrix2x3dv,	glGetUniformdv),
            GL_DOUBLE_MAT2x4:       ((2,4), np.float64,     float,  glUniformMatrix2x4dv,	glGetUniformdv),
            GL_DOUBLE_MAT3x2:       ((3,2), np.float64,     float,  glUniformMatrix3x2dv,	glGetUniformdv),
            GL_DOUBLE_MAT3:         ((3,3), np.float64,     float,  glUniformMatrix3dv,		glGetUniformdv),
            GL_DOUBLE_MAT3x4:       ((3,4), np.float64,     float,  glUniformMatrix3x4dv,	glGetUniformdv),
            GL_DOUBLE_MAT4x2:       ((4,2), np.float64,     float,  glUniformMatrix4x2dv,	glGetUniformdv),
            GL_DOUBLE_MAT4x3:       ((4,3), np.float64,     float,  glUniformMatrix4x3dv,	glGetUniformdv),
            GL_DOUBLE_MAT4:         ((4,4), np.float64,     float,  glUniformMatrix4dv,		glGetUniformdv),
            }

    @classmethod
    def check(cls, type):
        if hasattr(cls, 'uniformTypes2') and cls.uniformTypes2:
            cls.uniformTypes.update(cls.uniformTypes2)
            cls.uniformTypes2.clear()
        return type in cls.uniformTypes

    def __init__(self, index, name, type):
        dims, dtype, pytype, glfunc, glquery = self.uniformTypes[type]
        super(VectorUniform, self).__init__(index, name, pytype, dims)
        self.dtype = dtype
        self.glfunc = glfunc
        self.glquery = glquery

    def set(self, data):
        values = np.asarray(data, dtype=self.dtype).reshape(self.dims)
        if len(self.dims) > 1:
            self.glfunc(self.index, 1, GL_TRUE, values)
        else:
            self.glfunc(self.index, len(values), values)

    def update(self, pgm):
        values = np.zeros(self.dims, dtype=self.dtype)
        self.glquery(pgm, self.index, values)
        if len(self.dims) > 1:
            values = values.T
        self.values = values
        log.debug('VectorUniform(%s) = %s', self.name, self.values)
        return self.values

class SamplerUniform(Uniform):

    textureTargets = {
        GL_SAMPLER_1D:                                  GL_TEXTURE_1D,
        GL_SAMPLER_2D:                                  GL_TEXTURE_2D,
        GL_SAMPLER_3D:                                  GL_TEXTURE_3D,
        GL_SAMPLER_CUBE:                                GL_TEXTURE_CUBE_MAP,
        GL_SAMPLER_1D_SHADOW:                           GL_TEXTURE_1D,
        GL_SAMPLER_2D_SHADOW:                           GL_TEXTURE_2D,
        GL_SAMPLER_1D_ARRAY:                            GL_TEXTURE_1D_ARRAY,
        GL_SAMPLER_2D_ARRAY:                            GL_TEXTURE_2D_ARRAY,
        GL_SAMPLER_1D_ARRAY_SHADOW:                     GL_TEXTURE_1D_ARRAY,
        GL_SAMPLER_2D_ARRAY_SHADOW:                     GL_TEXTURE_2D_ARRAY,
        GL_SAMPLER_2D_MULTISAMPLE:                      GL_TEXTURE_2D_MULTISAMPLE,
        GL_SAMPLER_2D_MULTISAMPLE_ARRAY:                GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_SAMPLER_CUBE_SHADOW:                         GL_TEXTURE_CUBE_MAP,
        GL_SAMPLER_BUFFER:                              GL_TEXTURE_BUFFER,
        GL_SAMPLER_2D_RECT:                             GL_TEXTURE_RECTANGLE,
        GL_SAMPLER_2D_RECT_SHADOW:                      GL_TEXTURE_RECTANGLE,
        GL_INT_SAMPLER_1D:                              GL_TEXTURE_1D,
        GL_INT_SAMPLER_2D:                              GL_TEXTURE_2D,
        GL_INT_SAMPLER_3D:                              GL_TEXTURE_3D,
        GL_INT_SAMPLER_CUBE:                            GL_TEXTURE_CUBE_MAP,
        GL_INT_SAMPLER_1D_ARRAY:                        GL_TEXTURE_1D_ARRAY,
        GL_INT_SAMPLER_2D_ARRAY:                        GL_TEXTURE_2D_ARRAY,
        GL_INT_SAMPLER_2D_MULTISAMPLE:                  GL_TEXTURE_2D_MULTISAMPLE,
        GL_INT_SAMPLER_2D_MULTISAMPLE_ARRAY:            GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_INT_SAMPLER_BUFFER:                          GL_TEXTURE_BUFFER,
        GL_INT_SAMPLER_2D_RECT:                         GL_TEXTURE_RECTANGLE,
        GL_UNSIGNED_INT_SAMPLER_1D:                     GL_TEXTURE_1D,
        GL_UNSIGNED_INT_SAMPLER_2D:                     GL_TEXTURE_2D,
        GL_UNSIGNED_INT_SAMPLER_3D:                     GL_TEXTURE_3D,
        GL_UNSIGNED_INT_SAMPLER_CUBE:                   GL_TEXTURE_CUBE_MAP,
        GL_UNSIGNED_INT_SAMPLER_1D_ARRAY:               GL_TEXTURE_1D_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_2D_ARRAY:               GL_TEXTURE_2D_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE:         GL_TEXTURE_2D_MULTISAMPLE,
        GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY:   GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
        GL_UNSIGNED_INT_SAMPLER_BUFFER:                 GL_TEXTURE_BUFFER,
        GL_UNSIGNED_INT_SAMPLER_2D_RECT:                GL_TEXTURE_RECTANGLE,
        }

    if 'GL_IMAGE_1D' in globals():    
        try:
            textureTargets2 = {
                GL_IMAGE_1D:                                    GL_TEXTURE_1D,
                GL_IMAGE_2D:                                    GL_TEXTURE_2D,
                GL_IMAGE_3D:                                    GL_TEXTURE_3D,
                GL_IMAGE_2D_RECT:                               GL_TEXTURE_2D_RECTANGLE,
                GL_IMAGE_CUBE:                                  GL_TEXTURE_CUBE_MAP,
                GL_IMAGE_BUFFER:                                GL_TEXTURE_BUFFER,
                GL_IMAGE_1D_ARRAY:                              GL_TEXTURE_1D_ARRAY,
                GL_IMAGE_2D_ARRAY:                              GL_TEXTURE_2D_ARRAY,
                GL_IMAGE_2D_MULTISAMPLE:                        GL_TEXTURE_2D_MULTISAMPLE,
                GL_IMAGE_2D_MULTISAMPLE_ARRAY:                  GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                GL_INT_IMAGE_1D:                                GL_TEXTURE_1D,
                GL_INT_IMAGE_2D:                                GL_TEXTURE_2D,
                GL_INT_IMAGE_3D:                                GL_TEXTURE_3D,
                GL_INT_IMAGE_2D_RECT:                           GL_TEXTURE_2D_RECTANGLE,
                GL_INT_IMAGE_CUBE:                              GL_TEXTURE_CUBE_MAP,
                GL_INT_IMAGE_BUFFER:                            GL_TEXTURE_BUFFER,
                GL_INT_IMAGE_1D_ARRAY:                          GL_TEXTURE_1D_ARRAY,
                GL_INT_IMAGE_2D_ARRAY:                          GL_TEXTURE_2D_ARRAY,
                GL_INT_IMAGE_2D_MULTISAMPLE:                    GL_TEXTURE_2D_MULTISAMPLE,
                GL_INT_IMAGE_2D_MULTISAMPLE_ARRAY:              GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                GL_UNSIGNED_INT_IMAGE_1D:                       GL_TEXTURE_1D,
                GL_UNSIGNED_INT_IMAGE_2D:                       GL_TEXTURE_2D,
                GL_UNSIGNED_INT_IMAGE_3D:                       GL_TEXTURE_3D,
                GL_UNSIGNED_INT_IMAGE_2D_RECT:                  GL_TEXTURE_2D_RECTANGLE,
                GL_UNSIGNED_INT_IMAGE_CUBE:                     GL_TEXTURE_CUBE_MAP,
                GL_UNSIGNED_INT_IMAGE_BUFFER:                   GL_TEXTURE_BUFFER,
                GL_UNSIGNED_INT_IMAGE_1D_ARRAY:                 GL_TEXTURE_1D_ARRAY,
                GL_UNSIGNED_INT_IMAGE_2D_ARRAY:                 GL_TEXTURE_2D_ARRAY,
                GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE:           GL_TEXTURE_2D_MULTISAMPLE,
                GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY:     GL_TEXTURE_2D_MULTISAMPLE_ARRAY,
                }
        except:
            pass

    @classmethod
    def check(cls, type):
        if hasattr(cls, 'textureTargets2') and cls.textureTargets2:
            cls.textureTargets.update(cls.textureTargets2)
            cls.textureTargets2.clear()
        return type in cls.textureTargets

    def __init__(self, index, name, type):
        target = self.textureTargets[type]
        super(SamplerUniform, self).__init__(index, name, str, (1,))
        self.target = target

    def set(self, data):
        cls = type(self)
        glActiveTexture(GL_TEXTURE0 + cls.currentSampler)
        tex = texture.getTexture(data)
        if tex not in (False, None):
            glBindTexture(self.target, tex.textureId)
        glUniform1i(self.index, cls.currentSampler)
        cls.currentSampler += 1

    @classmethod
    def reset(cls):
        cls.currentSampler = 1

class Shader(object):
    _supported = None

    @classmethod
    def supported(cls):
        if cls._supported is None:
            cls._supported = bool(glCreateProgram)
        return cls._supported

    def __init__(self, path):
        if not self.supported():
            raise RuntimeError("No shader support detected")

        super(Shader, self).__init__()

        self.path = path

        self.vertexId = None
        self.fragmentId = None
        self.shaderId = None
        self.modified = None
        self.uniforms = None

        self.initShader()

    def __del__(self):
        try:
            self.delete()
        except StandardError:
            pass

    @staticmethod
    def createShader(file, type):
        with open(file, 'rU') as f:
            source = f.read()
        shader = glCreateShader(type)
        glShaderSource(shader, source)
        glCompileShader(shader)
        if not glGetShaderiv(shader, GL_COMPILE_STATUS):
            logmsg = glGetShaderInfoLog(shader)
            log.error("Error compiling shader: %s", logmsg)
            return None
        return shader

    def delete(self):
        if self.vertexId:
            glDeleteShader(self.vertexId)
            self.vertexId = None
        if self.fragmentId:
            glDeleteShader(self.fragmentId)
            self.fragmentId = None
        if self.shaderId:
            glDeleteProgram(self.shaderId)
            self.shaderId = None

    def initShader(self):
        vertexSource = self.path + '_vertex_shader.txt'
        geometrySource = self.path + '_geometry_shader.txt'
        fragmentSource = self.path + '_fragment_shader.txt'

        self.shaderId = glCreateProgram()

        if os.path.isfile(vertexSource):
            self.vertexId = self.createShader(vertexSource, GL_VERTEX_SHADER)
            glAttachShader(self.shaderId, self.vertexId)

        if os.path.isfile(geometrySource) and 'GL_GEOMETRY_SHADER' in globals():
            self.geometryId = self.createShader(geometrySource, GL_GEOMETRY_SHADER)
            glAttachShader(self.shaderId, self.geometryId)

        if os.path.isfile(fragmentSource):
            self.fragmentId = self.createShader(fragmentSource, GL_FRAGMENT_SHADER)
            glAttachShader(self.shaderId, self.fragmentId)

        glLinkProgram(self.shaderId)
        if not glGetProgramiv(self.shaderId, GL_LINK_STATUS):
            log.error("Error linking shader: %s", glGetProgramInfoLog(self.shaderId))
            self.delete()
            return

        self.uniforms = None
        self.updateUniforms()

    def getUniforms(self):
        if self.uniforms is None:
            parameterCount = glGetProgramiv(self.shaderId, GL_ACTIVE_UNIFORMS)
            self.uniforms = []
            for index in xrange(parameterCount):
                name, size, type = glGetActiveUniform(self.shaderId, index)
                if VectorUniform.check(type):
                    uniform = VectorUniform(index, name, type)
                elif SamplerUniform.check(type):
                    uniform = SamplerUniform(index, name, type)
                uniform.update(self.shaderId)
                self.uniforms.append(uniform)

        return self.uniforms

    def updateUniforms(self):
        for uniform in self.getUniforms():
            uniform.update(self.shaderId)

    def setUniforms(self, params):
        SamplerUniform.reset()

        for uniform in self.getUniforms():
            value = params.get(uniform.name)
            if value is not None:
                uniform.set(value)

_shaderCache = {}

def getShader(path, cache=None):
    shader = None
    cache = cache or _shaderCache

    path1 = path + '_vertex_shader.txt'
    path2 = path + '_fragment_shader.txt'
    path3 = path + '_geometry_shader.txt'
    paths = [p for p in [path1, path2, path3] if os.path.isfile(p)]
    if not paths:
        cache[path] = False
        return False

    mtime = max(os.path.getmtime(p) for p in paths)

    if path in cache:
        shader = cache[path]
        if shader is False:
            return shader

        if mtime >= shader.modified:
            log.message('reloading %s', path)
            try:
                shader.initShader()
                shader.modified = mtime
            except RuntimeError, text:
                log.error("Error loading shader %s", path, exc_info=True)
                shader = False
    else:
        try:
            shader = Shader(path)
            shader.modified = mtime
        except RuntimeError, text:
            log.error("Error loading shader %s", path, exc_info=True)
            shader = False

    cache[path] = shader
    return shader
    
def reloadShaders():
    log.message('Reloading shaders')
    for path in _shaderCache:
        try:
            _shaderCache[path].initShader()
        except RuntimeError, text:
            log.error("Error loading shader %s", path, exc_info=True)
            _shaderCache[path] = False


```

## Source file: shared/mhx/rig_arm_25.py

{
  "path": "shared/mhx/rig_arm_25.py",
  "blob_sha": "51606f1e017ff0e510842839ae6aeb2b48538df8",
  "sha256": "416d9badb4ceb6b773e69be7f483d5284d8f476104568788b167a51a57af9c4f",
  "lines": 616,
  "bytes": 26707,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 5,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/KoenBuys/makehuman_datagen/blob/3cb5b218b2f2821de66e7a3a86e2703c6045e542/shared/mhx/rig_arm_25.py",
  "function_signature_candidates": [
    {
      "line": 326,
      "text": "def ArmControlPoses(fp, amt):"
    },
    {
      "line": 528,
      "text": "def ArmWriteActions(fp):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

""" 
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    http://code.google.com/p/makehuman/

**Authors:**           Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2013

**Licensing:**         AGPL3 (see also http://www.makehuman.org/node/318)

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

Arm bone definitions 
"""


from .flags import *
from .posebone import addPoseBone

prcUpArmVec    = 0.15
offs = (0,0.6,0)

ArmJoints = [
    ('r-shoulder-in',       'v', 14160),
    ('l-shoulder-in',       'v', 14382),
    ('r-uparm0',            'j', 'r-shoulder'),
    ('l-uparm0',            'j', 'l-shoulder'),
    
    ('r-elbow',             'j', 'r-elbow'),
    ('l-elbow',             'j', 'l-elbow'),
    
    ('hand_L_tail',         'j', 'r-finger-3-1'),
    ('hand_R_tail',         'j', 'l-finger-3-1'),

    ('r-uparm1',            'l', ((0.67, 'r-uparm0'), (0.33, 'r-elbow'))),
    ('r-uparm2',            'l', ((0.33, 'r-uparm0'), (0.67, 'r-elbow'))),
    ('r-loarm1',            'l', ((0.67, 'r-elbow'), (0.33, 'r-hand'))),
    ('r-loarm2',            'l', ((0.33, 'r-elbow'), (0.67, 'r-hand'))),

    ('l-uparm1',            'l', ((0.67, 'l-uparm0'), (0.33, 'l-elbow'))),
    ('l-uparm2',            'l', ((0.33, 'l-uparm0'), (0.67, 'l-elbow'))),
    ('l-loarm1',            'l', ((0.67, 'l-elbow'), (0.33, 'l-hand'))),
    ('l-loarm2',            'l', ((0.33, 'l-elbow'), (0.67, 'l-hand'))),

    ('r-biceps-head',       'vl', ((0.7, 2956), (0.3, 3354))),
    ('l-biceps-head',       'vl', ((0.7, 10769), (0.3, 10399))),
    ('r-biceps-tail',       'l', ((0.8, 'r-elbow'), (0.2, 'r-hand'))),
    ('l-biceps-tail',       'l', ((0.8, 'l-elbow'), (0.2, 'l-hand'))),
    
    ('r-triceps-head',       'vl', ((0.2, 2956), (0.8, 3354))),
    ('l-triceps-head',       'vl', ((0.2, 10769), (0.8, 10399))),

    ('r-armtrg',            'o', ('r-uparm0', [1,0,0])),
    ('l-armtrg',            'o', ('l-uparm0', [-1,0,0])),

    ('r-elbow-pt',          'o', ('r-elbow', [0,0,-3])),
    ('l-elbow-pt',          'o', ('l-elbow', [0,0,-3])),

    ('r-elbow-fan',         'l', ((0.25, 'r-hand'), (0.75, 'r-elbow'))),
    ('l-elbow-fan',         'l', ((0.25, 'l-hand'), (0.75, 'l-elbow'))),

    ('r-elbow-head',        'v', 2987),
    ('r-elbow-tail',        'v', 4569),
    ('l-elbow-head',        'v', 10739),
    ('l-elbow-tail',        'v', 9904),
    
]

"""
if MuscleBones:
    ArmJoints += [    

    ('r-elbow-compress1',     'l', ((0.5, 'r-elbow'), (0.5, 'r-uparm0'))),
    ('r-elbow-compress2',     'l', ((0.5, 'r-elbow'), (0.5, 'r-hand'))),
    ('l-elbow-compress1',     'l', ((0.5, 'l-elbow'), (0.5, 'l-uparm0'))),
    ('l-elbow-compress2',     'l', ((0.5, 'l-elbow'), (0.5, 'l-hand'))),
    
]
"""

ArmHeadsTails = [
    # Upper arm
    ('UpArmVec_L',            'r-uparm0', 'r-armtrg'),
    ('UpArmVecPos_L',         'r-uparm0', 'r-armtrg'),
    ('UpArmVecNeg_L',         'r-uparm0', 'r-armtrg'),
    ('UpArm_L',             'r-uparm0', 'r-elbow'),
    ('UpArmIK_L',           'r-uparm0', 'r-elbow'),
    ('UpArmRot_L',          'r-uparm0', 'r-uparm1'),
    ('DfmUpArm1_L',          'r-uparm0', 'r-uparm1'),
    ('DfmUpArm2_L',          'r-uparm1', 'r-uparm2'),
    ('DfmUpArm3_L',          'r-uparm2', 'r-elbow'),

    ('UpArmVec_R',            'l-uparm0', 'l-armtrg'),
    ('UpArmVecPos_R',         'l-uparm0', 'l-armtrg'),
    ('UpArmVecNeg_R',         'l-uparm0', 'l-armtrg'),
    ('UpArm_R',             'l-uparm0', 'l-elbow'),
    ('UpArmIK_R',           'l-uparm0', 'l-elbow'),
    ('UpArmRot_R',          'l-uparm0', 'l-uparm1'),
    ('DfmUpArm1_R',          'l-uparm0', 'l-uparm1'),
    ('DfmUpArm2_R',          'l-uparm1', 'l-uparm2'),
    ('DfmUpArm3_R',          'l-uparm2', 'l-elbow'),

    # Lower arm
    ('LoArmVec_L',          'r-elbow', 'r-loarm1'),
    ('LoArmVecPos_L',       'r-elbow', 'r-loarm1'),
    ('LoArmVecNeg_L',       'r-elbow', 'r-loarm1'),

    ('LoArm_L',             'r-elbow', 'r-hand'),
    ('LoArmIK_L',           'r-elbow', 'r-hand'),
    ('LoArmRot_L',          'r-elbow', 'r-loarm1'),
    #('HlpLoArm_L',          'r-elbow', 'r-hand'),
    ('DfmLoArm1_L',         'r-elbow', 'r-loarm1'),
    ('DfmLoArm2_L',         'r-loarm1', 'r-loarm2'),
    ('DfmLoArm3_L',         'r-loarm2', 'r-hand'),

    ('LoArmVec_R',          'l-elbow', 'l-loarm1'),
    ('LoArmVecPos_R',       'l-elbow', 'l-loarm1'),
    ('LoArmVecNeg_R',       'l-elbow', 'l-loarm1'),

    ('LoArm_R',             'l-elbow', 'l-hand'),
    ('LoArmIK_R',           'l-elbow', 'l-hand'),
    ('LoArmRot_R',          'l-elbow', 'l-loarm1'),
    #('HlpLoArm_R',          'l-elbow', 'l-hand'),
    ('DfmLoArm1_R',         'l-elbow', 'l-loarm1'),
    ('DfmLoArm2_R',         'l-loarm1', 'l-loarm2'),
    ('DfmLoArm3_R',         'l-loarm2', 'l-hand'),
    
    # Hand
    ('Wrist_L',             'r-hand', 'hand_L_tail'),
    ('Hand_L',              'r-hand', 'hand_L_tail'),
    ('DfmHand_L',           'r-hand', 'hand_L_tail'),
    ('Wrist_R',             'l-hand', 'hand_R_tail'),
    ('Hand_R',              'l-hand', 'hand_R_tail'),
    ('DfmHand_R',           'l-hand', 'hand_R_tail'),

    # Muscle
    
    ('DfmElbowFwd_L',       'r-uparm2', 'r-loarm1'),
    ('DfmElbowFwd_R',       'l-uparm2', 'l-loarm1'),
    
    ('DfmBiceps_L',          'r-biceps-head', 'r-biceps-tail'),
    ('DfmBiceps_R',          'l-biceps-head', 'l-biceps-tail'),

    ('DfmElbowFan_L',       'r-elbow', 'r-elbow-fan'),
    ('DfmElbowFan_R',       'l-elbow', 'l-elbow-fan'),

    
   # Pole Targets

    ('UpArmDir_L',         'r-uparm1', ('r-uparm1', yunit)),
    ('UpArmDir_R',         'l-uparm1', ('l-uparm1', yunit)),

    ('UpArm2PT_L',         ('r-uparm1', yunit), ('r-uparm1', ybis)),
    ('UpArm3PT_L',         ('r-uparm2', yunit), ('r-uparm2', ybis)),
    ('UpArm2PT_R',         ('l-uparm1', yunit), ('l-uparm1', ybis)),
    ('UpArm3PT_R',         ('l-uparm2', yunit), ('l-uparm2', ybis)),

    ('LoArm2PT_L',         ('r-loarm1', yunit), ('r-loarm1', ybis)),
    ('LoArm3PT_L',         ('r-loarm2', yunit), ('r-loarm2', ybis)),
    ('LoArm2PT_R',         ('l-loarm1', yunit), ('l-loarm1', ybis)),
    ('LoArm3PT_R',         ('l-loarm2', yunit), ('l-loarm2', ybis)),

    ('ElbowPT_L',         'r-elbow-pt', ('r-elbow-pt', offs)),
    ('ElbowPT_R',         'l-elbow-pt', ('l-elbow-pt', offs)),
    ('ElbowPTFK_L',       'r-elbow-pt', ('r-elbow-pt', offs)),
    ('ElbowPTFK_R',       'l-elbow-pt', ('l-elbow-pt', offs)),
    ('ElbowLinkPT_L',     'r-elbow', 'r-elbow-pt'),
    ('ElbowLinkPT_R',     'l-elbow', 'l-elbow-pt'),   
    
    # Directions
    
    #('DirUpArm_L',           'r-uparm0', ('r-uparm0', (1,0,0))),
    #('DirUpArm_R',           'l-uparm0', ('l-uparm0', (-1,0,0))),

    ('DirElbowFwd_L',        'r-elbow', ('r-elbow', (0,0,1))),
    ('DirElbowFwd_R',        'l-elbow', ('l-elbow', (0,0,1))),
    ('DirElbowInv_L',        'r-elbow', ('r-elbow', (-1,0,0))),
    ('DirElbowInv_R',        'l-elbow', ('l-elbow', (1,0,0))),
    ('DirElbowUp_L',         'r-elbow', ('r-elbow', (0,1,0))),
    ('DirElbowUp_R',         'l-elbow', ('l-elbow', (0,1,0))),
    ('DirElbowDown_L',       'r-elbow', ('r-elbow', (0,-1,0))),
    ('DirElbowDown_R',       'l-elbow', ('l-elbow', (0,-1,0))),
]

if MuscleBones:
    ArmHeadsTails = [
     # Elbow bend
    ('DfmElbowBend_L',     'r-elbow-head', 'r-elbow-tail'),
    ('ElbowBendTrg_L',     'r-elbow-tail', ('r-elbow-tail', yunit)),
    ('DfmElbowBend_R',     'l-elbow-head', 'l-elbow-tail'),
    ('ElbowBendTrg_R',     'l-elbow-tail', ('l-elbow-tail', yunit)),

    #('DfmElbowCompress_L', 'r-elbow-compress1', 'r-elbow-compress2'),
    #('ElbowCompressTrg_L', 'r-elbow-compress2', ('r-elbow-compress2', yunit)),
    #('DfmElbowCompress_R', 'l-elbow-compress1', 'l-elbow-compress2'),
    #('ElbowCompressTrg_R', 'l-elbow-compress2', ('l-elbow-compress2', yunit)),
]

ArmArmature = [
    # Arm
    ('LoArm_L',            0, 'UpArm_L', F_WIR+F_SCALE, L_LARMFK, NoBB),
    ('Hand_L',             0, 'LoArm_L', F_CON+F_WIR, L_LARMFK+L_LEXTRA, NoBB),
    
    ('LoArm_R',            0, 'UpArm_R', F_WIR+F_SCALE, L_RARMFK, NoBB),
    ('Hand_R',             0, 'LoArm_R', F_CON+F_WIR, L_RARMFK+L_REXTRA, NoBB),
    
    # IK arm
    ('UpArmIK_L',          0, 'UpArmHinge_L', 0, L_HELP2, NoBB),
    ('LoArmIK_L',          0, 'UpArmIK_L', F_CON, L_HELP2, NoBB),
    #('HlpLoArm_L',         0, 'UpArm_L', F_NOROT+F_CON+F_WIR, L_HELP2, NoBB),
    ('Wrist_L',            0, Master, F_WIR, L_LARMIK, NoBB),

    ('UpArmIK_R',          0, 'UpArmHinge_R', 0, L_HELP2, NoBB),
    ('LoArmIK_R',          0, 'UpArmIK_R', F_CON, L_HELP2, NoBB),
    #('HlpLoArm_R',         0, 'UpArm_R', F_NOROT+F_CON+F_WIR, L_HELP2, NoBB),
    ('Wrist_R',            0, Master, F_WIR, L_RARMIK, NoBB),
    
     # Arm targets
    ('UpArmVec_L',          0, 'UpArmSocket_L', 0, L_HELP, NoBB),
    ('UpArmVec_R',          0, 'UpArmSocket_R', 0, L_HELP, NoBB),
    ('UpArmVecPos_L',       60*D, 'UpArmSocket_L', 0, L_HELP, NoBB),
    ('UpArmVecPos_R',       -60*D, 'UpArmSocket_R', 0, L_HELP, NoBB),
    ('UpArmVecNeg_L',       -60*D, 'UpArmSocket_L', 0, L_HELP, NoBB),
    ('UpArmVecNeg_R',       60*D, 'UpArmSocket_R', 0, L_HELP, NoBB),

    ('LoArmVec_L',        0*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('LoArmVecPos_L',     90*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('LoArmVecNeg_L',     -90*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('LoArmVec_R',        0*D, 'UpArm_R', 0, L_HELP, NoBB),
    ('LoArmVecPos_R',     90*D, 'UpArm_R', 0, L_HELP, NoBB),
    ('LoArmVecNeg_R',     -90*D, 'UpArm_R', 0, L_HELP, NoBB),
    
    # Pole targets
    ('ElbowPT_L',         0, 'Shoulder_L', F_WIR, L_LARMIK, NoBB),
    ('ElbowPT_R',         0, 'Shoulder_R', F_WIR, L_RARMIK, NoBB),
    ('ElbowPTFK_L',       0, 'UpArm_L', 0, L_HELP2, NoBB),
    ('ElbowPTFK_R',       0, 'UpArm_R', 0, L_HELP2, NoBB),
    ('ElbowLinkPT_L',     0, 'UpArm_L', F_RES, L_LARMIK, NoBB),
    ('ElbowLinkPT_R',     0, 'UpArm_R', F_RES, L_RARMIK, NoBB),

    # Arm deform
    ('UpArmRot_L',        0, 'UpArmHinge_L', F_WIR, L_TWEAK, NoBB),
    ('DfmUpArm1_L',       0, 'UpArmRot_L', F_DEF, L_DEF, NoBB),
    ('UpArm3PT_L',        0, 'UpArm_L', 0, L_HELP, NoBB),
    ('UpArm2PT_L',        0, 'DfmUpArm1_L', 0, L_HELP, NoBB),
    ('DfmUpArm2_L',       0, 'DfmUpArm1_L', F_DEF, L_DEF, NoBB),
    ('DfmUpArm3_L',       0, 'UpArm_L', F_DEF, L_DEF, NoBB),

    ('LoArmRot_L',        0, 'UpArm_L', F_WIR, L_TWEAK, NoBB),
    ('DfmLoArm1_L',       0, 'LoArmRot_L', F_DEF, L_DEF, NoBB),
    ('LoArm3PT_L',        0, 'LoArm_L', 0, L_HELP, NoBB),
    ('LoArm2PT_L',        0, 'DfmLoArm1_L', 0, L_HELP, NoBB),
    ('DfmLoArm2_L',       0, 'DfmLoArm1_L', F_DEF+F_SCALE, L_DEF, NoBB ),
    ('DfmLoArm3_L',       0, 'LoArm_L', F_DEF, L_DEF, NoBB),
    ('DfmHand_L',         0, 'Hand_L', F_DEF, L_DEF, NoBB),
    
    ('UpArmRot_R',        0, 'UpArmHinge_R', F_WIR, L_TWEAK, NoBB),
    ('DfmUpArm1_R',       0, 'UpArmRot_R', F_DEF, L_DEF, NoBB),
    ('UpArm3PT_R',        0, 'UpArm_R', 0, L_HELP, NoBB),
    ('UpArm2PT_R',        0, 'DfmUpArm1_R', 0, L_HELP, NoBB),
    ('DfmUpArm2_R',       0, 'DfmUpArm1_R', F_DEF, L_DEF, NoBB),
    ('DfmUpArm3_R',       0, 'UpArm_R', F_DEF, L_DEF, NoBB),

    ('LoArmRot_R',        0, 'UpArm_R', F_WIR, L_TWEAK, NoBB),
    ('DfmLoArm1_R',       0, 'LoArmRot_R', F_DEF, L_DEF, NoBB),
    ('LoArm3PT_R',        0, 'LoArm_R', 0, L_HELP, NoBB),
    ('LoArm2PT_R',        0, 'DfmLoArm1_R', 0, L_HELP, NoBB),    
    ('DfmLoArm2_R',       0, 'DfmLoArm1_R', F_DEF+F_SCALE, L_DEF, NoBB ),
    ('DfmLoArm3_R',       0, 'LoArm_R', F_DEF, L_DEF, NoBB),
    ('DfmHand_R',         0, 'Hand_R', F_DEF, L_DEF, NoBB),
    
    # Muscles
    ('DfmElbowFwd_L',     0, 'UpArm_L', F_DEF, L_MSCL, NoBB),
    ('DfmElbowFwd_R',     0, 'UpArm_R', F_DEF, L_MSCL, NoBB),

    ('DfmElbowFan_L',     0, 'UpArm_L', F_DEF, L_MSCL, NoBB),
    ('DfmElbowFan_R',     0, 'UpArm_R', F_DEF, L_MSCL, NoBB),

    ('DfmBiceps_L',       0, 'UpArmHinge_L', F_DEF, L_MSCL, NoBB),
    ('DfmBiceps_R',       0, 'UpArmHinge_R', F_DEF, L_MSCL, NoBB),

    # Directions
    #('DirUpArm_L',         0, 'UpArm_L', 0, L_HELP, NoBB),
    #('DirUpArm_R',         0, 'UpArm_R', 0, L_HELP, NoBB),

    ('DirElbowFwd_L',       0*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('DirElbowFwd_R',       0*D, 'UpArm_R', 0, L_HELP, NoBB),
    ('DirElbowInv_L',       0*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('DirElbowInv_R',       0*D, 'UpArm_R', 0, L_HELP, NoBB),
    ('DirElbowUp_L',        -90*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('DirElbowUp_R',        90*D, 'UpArm_R', 0, L_HELP, NoBB),
    ('DirElbowDown_L',      90*D, 'UpArm_L', 0, L_HELP, NoBB),
    ('DirElbowDown_R',      -90*D, 'UpArm_R', 0, L_HELP, NoBB),

]

#
#
#

#limUpArm_L = (-135*D,135*D, -120*D,90*D, -135*D,135*D)
#limUpArm_R = (-135*D,135*D, -90*D,120*D, -135*D,135*D)
limUpArm_L = (-135*D,135*D, -60*D,60*D, -135*D,135*D)
limUpArm_R = (-135*D,135*D, -60*D,60*D, -135*D,135*D)

limLoArm_L = (-10*D,100*D, -178*D,150*D, -175*D,10*D)
limLoArm_R = (-10*D,100*D, -150*D,178*D, -10*D,175*D)

limHand_L = (-90*D,70*D, -10*D,10*D, -20*D,20*D)
limHand_R = (-90*D,70*D, -10*D,10*D, -20*D,20*D)


#
#    ArmControlPoses(fp, amt):
#

def ArmControlPoses(fp, amt):

    # Arm
    
    deltaElbow = 0.6*D
    deltaWrist = 0.0*D

    addPoseBone(fp, amt, 'UpArm_L', 'GZM_Circle025', 'FK_L', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limUpArm_L, (True, True, True)]),
         #('CopyTrans', 0, 0, ['ArmIK', 'UpArmIK_L', 0]),
         ('IK', 0, 0, ['ArmIK', 'LoArmIK_L', 1, None, (True, False,False)]),
         ('CopyTrans', 0, 0, ['Elbow', 'ELUpArm_L', 0])
        ])

    addPoseBone(fp, amt, 'LoArm_L', 'GZM_Circle025', 'FK_L', (1,1,1), (0,0,0), (1,0,1), (1,1,1), P_YXZ, 
        [#('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limLoArm_L, (True, True, True)]),
         #('CopyTrans', 0, 0, ['ArmIK', 'LoArmIK_L', 0]),
         ('IK', 0, 0, ['ArmIK', 'Wrist_L', 1, None, (True, False,False)]),
         ('IK', 0, 0, ['Wrist', 'Wrist_L', 1, None, (True, False,False)]),
        ])

    addPoseBone(fp, amt, 'Hand_L', 'MHHand', 'FK_L', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0,
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limHand_L, (True, True, True)]),
         ('IK', 0, 0, ['FreeIK', None, 2, None, (True, False,False)]),
         ('CopyLoc', 0, 0, ['WristLoc', 'Wrist_L', (1,1,1), (0,0,0), 0, False]),
         ('CopyRot', 0, 0, ['WristRot', 'Wrist_L', (1,1,1), (0,0,0), False])
        ])
        
        
    addPoseBone(fp, amt, 'UpArm_R', 'GZM_Circle025', 'FK_R', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limUpArm_R, (True, True, True)]),
         #('CopyTrans', 0, 0, ['ArmIK', 'UpArmIK_R', 0]),
         ('IK', 0, 0, ['ArmIK', 'LoArmIK_R', 1, None, (True, False,False)]),
         ('CopyTrans', 0, 0, ['Elbow', 'ELUpArm_R', 0])
        ])

    addPoseBone(fp, amt, 'LoArm_R', 'GZM_Circle025', 'FK_R', (1,1,1), (0,0,0), (1,0,1), (1,1,1), P_YXZ, 
        [#('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limLoArm_R, (True, True, True)]),
         #('CopyTrans', 0, 0, ['ArmIK', 'LoArmIK_R', 0]),
         ('IK', 0, 0, ['ArmIK', 'Wrist_R', 1, None, (True, False,False)]),
         ('IK', 0, 0, ['Wrist', 'Wrist_R', 1, None, (True, False,False)]),
        ])

    addPoseBone(fp, amt, 'Hand_R', 'MHHand', 'FK_R', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0,
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limHand_R, (True, True, True)]),
         ('IK', 0, 0, ['FreeIK', None, 2, None, (True, False,False)]),
         ('CopyLoc', 0, 0, ['WristLoc', 'Wrist_R', (1,1,1), (0,0,0), 0, False]),
         ('CopyRot', 0, 0, ['WristRot', 'Wrist_R', (1,1,1), (0,0,0), False])
        ])

    if True:
        # IK arm
    
        addPoseBone(fp, amt, 'UpArmIK_L', None, 'IK_L', (1,1,1), (0,0,0), (1,1,1), 
                ((1,1,1), (0,0,0), 0.05, limUpArm_L), 0, [])

        addPoseBone(fp, amt, 'LoArmIK_L', None, 'IK_L', (1,1,1), (0,0,0), (1,1,1), 
                    ((0,1,1), (0,0,0), 0.05, limLoArm_L), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['Hint', (0,0, 0,0, -18*D,-18*D), (0,0,1)]),
             ('IK', 0, 1, ['ArmIK', 'Wrist_L', 2, (pi-deltaElbow, 'ElbowPT_L'), (True, False,False)]),
             ])

        #addPoseBone(fp, amt, 'HlpLoArm_L', 'GZM_Circle025', 'IK_L', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        #    [('CopyTrans', 0, 1, ['Arm', 'LoArm_L', 0]),
        #     ('CopyRot', C_LOCAL, 0, ['WristRot', 'Wrist_L', (0,1,0), (0,0,0), False]),
        #    ])

        addPoseBone(fp, amt, 'Wrist_L', 'MHHandCtrl_L', 'IK_L', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0, 
            [#('ChildOf', C_CHILDOF, 0, ['Shoulder', 'Clavicle_L', (1,1,1), (1,1,1), (1,1,1)]),
             ('LimitDist', 0, 0, ['DistShoulder', 'Shoulder_L', 'LIMITDIST_INSIDE']),
             ('LimitDist', 0, 0, ['DistElbow', 'Elbow_L', 'LIMITDIST_ONSURFACE']),
            ])

        addPoseBone(fp, amt, 'UpArmIK_R', None, 'IK_R', (1,1,1), (0,0,0), (1,1,1), 
                ((1,1,1), (0,0,0), 0.05, limUpArm_R), 0, [])

        addPoseBone(fp, amt, 'LoArmIK_R', None, 'IK_R', (1,1,1), (0,0,0), (1,1,1), 
                ((0,1,1), (0,0,0), 0.05, limLoArm_R), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['Hint', (0,0, 0,0, 18*D,18*D), (0,0,1)]),
         ('IK', 0, 1, ['ArmIK', 'Wrist_R', 2, (deltaElbow, 'ElbowPT_R'), (True, False,False)]),        
        ])

        #addPoseBone(fp, amt, 'HlpLoArm_R', 'GZM_Circle025', 'IK_R', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        #    [('CopyTrans', 0, 1, ['Arm', 'LoArm_R', 0]),
        #     ('CopyRot', C_LOCAL, 0, ['WristRot', 'Wrist_R', (0,1,0), (0,0,0), False]),
        #    ])

        addPoseBone(fp, amt, 'Wrist_R', 'MHHandCtrl_R', 'IK_R', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0, 
            [#('ChildOf', C_CHILDOF, 0, ['Shoulder', 'Clavicle_R', (1,1,1), (1,1,1), (1,1,1)]),
            ('LimitDist', 0, 0, ['DistShoulder', 'Shoulder_R', 'LIMITDIST_INSIDE']),
            ('LimitDist', 0, 0, ['DistElbow', 'Elbow_R', 'LIMITDIST_ONSURFACE']),
            ])

        # Pole target
    
        addPoseBone(fp, amt, 'ElbowPT_L', 'MHCube025', 'IK_L', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'ElbowLinkPT_L', None, 'IK_L', (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
            [('StretchTo', 0, 1, ['Stretch', 'ElbowPT_L', 0, 1, 3.0])])

        addPoseBone(fp, amt, 'ElbowPT_R', 'MHCube025', 'IK_R', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'ElbowLinkPT_R', None, 'IK_R', (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
            [('StretchTo', 0, 1, ['Stretch', 'ElbowPT_R', 0, 1, 3.0])])

    # Targets
    
    upArmTrgIk_L = ('IK', 0, 1, ['ArmIK', 'LoArm_L', 1, None, (True, False,True)])
    addPoseBone(fp, amt, 'UpArmVec_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_L])
    addPoseBone(fp, amt, 'UpArmVecPos_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_L])
    addPoseBone(fp, amt, 'UpArmVecNeg_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_L])

    upArmTrgIk_R = ('IK', 0, 1, ['ArmIK', 'LoArm_R', 1, None, (True, False,True)])
    addPoseBone(fp, amt, 'UpArmVec_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_R])
    addPoseBone(fp, amt, 'UpArmVecPos_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_R])
    addPoseBone(fp, amt, 'UpArmVecNeg_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [upArmTrgIk_R])
    
    loArmTrgIk_L = ('IK', 0, 1, ['IK', 'Hand_L', 1, None, (True, False,True)])
    addPoseBone(fp, amt, 'LoArmVec_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_L])
    addPoseBone(fp, amt, 'LoArmVecPos_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_L])
    addPoseBone(fp, amt, 'LoArmVecNeg_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_L])
    
    loArmTrgIk_R = ('IK', 0, 1, ['IK', 'Hand_R', 1, None, (True, False,True)])
    addPoseBone(fp, amt, 'LoArmVec_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_R])
    addPoseBone(fp, amt, 'LoArmVecPos_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_R])
    addPoseBone(fp, amt, 'LoArmVecNeg_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0, [loArmTrgIk_R])
    

    deltaUpArm = 3*D
    deltaLoArm = 0*D
    
    # Upper arm deform
    
    addPoseBone(fp, amt, 'UpArm2PT_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyLoc', 0, 0.5, ['UpArm3', 'UpArm3PT_L', (1,1,1), (0,0,0), 0, False])])

    addPoseBone(fp, amt, 'UpArmRot_L', 'MHCircle05', 'FK_L', (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoArm_L', 1, None, (True, False,True)])])         

    addPoseBone(fp, amt, 'DfmUpArm2_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoArm_L', 1, (90*D, 'UpArm2PT_L'), (True, False,True)])])
        

    addPoseBone(fp, amt, 'UpArm2PT_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyLoc', 0, 0.5, ['UpArm3', 'UpArm3PT_R', (1,1,1), (0,0,0), 0, False])])

    addPoseBone(fp, amt, 'UpArmRot_R', 'MHCircle05', 'FK_R', (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoArm_R', 1, None, (True, False,True)])])

    addPoseBone(fp, amt, 'DfmUpArm2_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoArm_R', 1, (90*D, 'UpArm2PT_R'), (True, False,True)])])
        

    # Lower arm deform
    
    addPoseBone(fp, amt, 'LoArm2PT_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyLoc', 0, 0.5, ['LoArm3', 'LoArm3PT_L', (1,1,1), (0,0,0), 0, False])])

    addPoseBone(fp, amt, 'LoArmRot_L', 'MHCircle05', 'FK_L', (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'Hand_L', 1, None, (True, False,True)])])         

    addPoseBone(fp, amt, 'DfmLoArm2_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'Hand_L', 1, (90*D, 'LoArm2PT_L'), (True, False,True)])])
        

    addPoseBone(fp, amt, 'LoArm2PT_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyLoc', 0, 0.5, ['LoArm3', 'LoArm3PT_R', (1,1,1), (0,0,0), 0, False])])

    addPoseBone(fp, amt, 'LoArmRot_R', 'MHCircle05', 'FK_R', (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'Hand_R', 1, None, (True, False,True)])])

    addPoseBone(fp, amt, 'DfmLoArm2_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'Hand_R', 1, (90*D, 'LoArm2PT_R'), (True, False,True)])])
        
    # Muscles
    
    addPoseBone(fp, amt, 'DfmBiceps_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', C_STRVOL, 1, ['Stretch', 'LoArm_L', 0.2, 1])])

    addPoseBone(fp, amt, 'DfmBiceps_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', C_STRVOL, 1, ['Stretch', 'LoArm_R', 0.2, 1])])

    addPoseBone(fp, amt, 'DfmElbowFan_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'LoArm_L', (1,0,1), (0,0,0), False])])

    addPoseBone(fp, amt, 'DfmElbowFan_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'LoArm_R', (1,0,1), (0,0,0), False])])
    
    addPoseBone(fp, amt, 'DfmElbowFwd_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', C_PLANEZ, 1, ['Stretch', 'DfmLoArm1_L', 1, 1])])

    addPoseBone(fp, amt, 'DfmElbowFwd_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', C_PLANEZ, 1, ['Stretch', 'DfmLoArm1_R', 1, 1])])


    return
    

#
#    ArmWriteActions(fp)
#

def ArmWriteActions(fp):
    return

#
#    ArmFKIKDrivers
#    (Bone, FK constraint, IK constraint, driver, channel, max)
#
"""
ArmFKIKDrivers = [
    ("UpArm_L", True, [], [], "PArmIK_L", "LOC_X", 1.0),
    ("LoArm_L", True, [], ["ArmIK"], "PArmIK_L", "LOC_X", 1.0),
    ("Hand_L", True, ["FreeIK"], ["WristLoc", "WristRot"], "PArmIK_L", "LOC_X", 1.0),

    ("UpArm_R", True, [], [], "PArmIK_R", "LOC_X", 1.0),
    ("LoArm_R", True, [], ["ArmIK"], "PArmIK_R", "LOC_X", 1.0),
    ("Hand_R", True, ["FreeIK"], ["WristLoc", "WristRot"], "PArmIK_R", "LOC_X", 1.0),
]
"""
#
#   ArmPropLRDrivers
#   (Bone, Name, Props, Expr)
#

ArmPropLRDrivers = [
    ('Clavicle', 'Elbow', ['ElbowPlant'], 'x1'),
    ('UpArm', 'ArmIK', ['ArmIk', 'ElbowPlant'], 'x1*(1-x2)'),
    ('UpArm', 'Elbow', ['ElbowPlant'], 'x1'),
    #('Elbow', 'DistSternum', ['ArmStretch', 'ElbowPlant'], '(1-x1)*x2'),
    ('LoArm', 'ArmIK', ['ArmIk', 'ElbowPlant'], 'x1*(1-x2)'),
    ('LoArm', 'Wrist', ['ArmIk', 'ElbowPlant'], 'x1*x2'),
    ('Wrist', 'DistShoulder', ['ArmStretch', 'ElbowPlant'], '(1-x1)*(1-x2)'),
    ('Wrist', 'DistElbow', ['ArmStretch', 'ElbowPlant'], '(1-x1)*x2'),
    ('Hand', 'FreeIK', ['ArmIk', 'ElbowPlant'], '(1-x1)*(1-x2)'),
    ('Hand', 'WristLoc', ['ArmIk'], 'x1'),
    ('Hand', 'WristRot', ['ArmIk', 'HandFollowsWrist'], 'x1*x2'),
    #('HlpLoArm', 'WristRot', ['ArmIk', 'HandFollowsWrist'], 'x1*x2'),
]

SoftArmPropLRDrivers = [
    ('UpArmSocket', 'Hinge', ['ArmHinge'], '1-x1'),
    #('ElbowPT', 'Hand', ['ElbowFollowsWrist'], 'x1'),
    #('ElbowPT', 'Shoulder', ['ElbowFollowsWrist'], '(1-x1)'),
]

ArmPropDrivers = [
    ('UpArm_L', 'LimitRot', ['RotationLimits', 'ArmIk_L'], 'x1*(1-x2)'),
    #('LoArm_L', 'LimitRot', ['RotationLimits', 'ArmIk_L'], 'x1*(1-x2)'),    
    ('Hand_L', 'LimitRot', ['RotationLimits', 'ArmIk_L', 'HandFollowsWrist_L'], 'x1*(1-x2*x3)'),
    ('UpArm_R', 'LimitRot', ['RotationLimits', 'ArmIk_R'], 'x1*(1-x2)'),
    #('LoArm_R', 'LimitRot', ['RotationLimits', 'ArmIk_R'], 'x1*(1-x2)'),    
    ('Hand_R', 'LimitRot', ['RotationLimits', 'ArmIk_R', 'HandFollowsWrist_R'], 'x1*(1-x2*x3)'),
 ]

#
#    ArmDeformDrivers
#    (Bone, constraint, driver, rotdiff, keypoints)
#

ArmDeformDrivers = []

#
#    ArmShapeDrivers
#    Shape : (driver, rotdiff, keypoints)
#

ArmShapeDrivers = {}
'''
    'BicepFlex_L' : ( 'LoArm1_L', 'ROTATION_DIFF', 'BendLoArmForward_L',  [(0,1), (90*D,0)] ),
    'BicepFlex_R' : ( 'LoArm1_R', 'ROTATION_DIFF', 'BendLoArmForward_R',  [(0,1), (90*D,0)] ),
}
'''

expr90 = "(1-%.3f*x1)" % (2/pi)
expr135 = "2*(1-%.3f*x1)" % (2/pi)
expr90_90 = "max(1-%.3f*x1,0)*max(1-%.3f*x2,0)" % (2/pi, 2/pi)

ElbowTargetDrivers = [    
    ("elbow-up-90", "LR", expr90, 
        [("LoArmVec", "DirElbowUp")]),

    ("elbow-down-90", "LR", expr90, 
        [("LoArmVec", "DirElbowDown")]),


#    ("loarms-forward-90",  "LR", expr90,
#        [("LoArmVec", "DirElbowFwd")]),
#    ("loarms-forward-135",  "LR", expr135,
#        [("LoArmVec", "DirElbowInv")]),
]    


```

## Source file: shared/mhx/rig_leg_25.py

{
  "path": "shared/mhx/rig_leg_25.py",
  "blob_sha": "3c8ee2d3c820894b08361438d6bf05a35e172552",
  "sha256": "c690ea643134c0032d81bfbf80bf666229e66305e557653985a681ed3b0d4f57",
  "lines": 659,
  "bytes": 27550,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 6,
    "rig": 1
  },
  "url": "https://github.com/KoenBuys/makehuman_datagen/blob/3cb5b218b2f2821de66e7a3a86e2703c6045e542/shared/mhx/rig_leg_25.py",
  "function_signature_candidates": [
    {
      "line": 375,
      "text": "def LegControlPoses(fp, amt):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

""" 
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    http://code.google.com/p/makehuman/

**Authors:**           Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2013

**Licensing:**         AGPL3 (see also http://www.makehuman.org/node/318)

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

Leg bone definitions 
"""


from .flags import *
from . import posebone
from posebone import addPoseBone

offs = [0,0.6,0]
prcLegTrg = 0.2

LegJoints = [
    #('r-knee',              'f', ('r-knee-raw', 'r-upper-leg', 'r-ankle', [0,0,0.1])),
    #('l-knee',              'f', ('l-knee-raw', 'l-upper-leg', 'l-ankle', [0,0,0.1])),
    ('r-knee',              'j', 'r-knee'),
    ('l-knee',              'j', 'l-knee'),

    ('r-upleg1',            'l', ((0.75, 'r-upper-leg'), (0.25, 'r-knee'))),
    ('r-upleg2',            'l', ((0.5, 'r-upper-leg'), (0.5, 'r-knee'))),
    ('l-upleg1',            'l', ((0.75, 'l-upper-leg'), (0.25, 'l-knee'))),
    ('l-upleg2',            'l', ((0.5, 'l-upper-leg'), (0.5, 'l-knee'))),
    ('r-loleg1',            'l', ((0.75, 'r-knee'), (0.25, 'r-ankle'))),
    ('l-loleg1',            'l', ((0.75, 'l-knee'), (0.25, 'l-ankle'))),

    ('r-knee-stretch1',     'vl', ((0.25, 3985), (0.75, 3794))),
    ('r-knee-stretch2',     'vl', ((0.25, 3860), (0.75, 5710))),
    ('l-knee-stretch1',     'vl', ((0.25, 7024), (0.75, 7217))), 
    ('l-knee-stretch2',     'vl', ((0.25, 7149), (0.75, 6772))),

    ('r-legtrg',            'l', ((1-prcLegTrg, 'r-upper-leg'), (prcLegTrg, 'r-knee'))),
    ('l-legtrg',            'l', ((1-prcLegTrg, 'l-upper-leg'), (prcLegTrg, 'l-knee'))),

    ('r-midfoot',           'l', ((0.5, 'r-ankle'), (0.5, 'r-foot-1'))),
    ('r-midtoe',            'l', ((0.5, 'r-foot-1'), (0.5, 'r-foot-2'))),
    ('l-midfoot',           'l', ((0.5, 'l-ankle'), (0.5, 'l-foot-1'))),
    ('l-midtoe',            'l', ((0.5, 'l-foot-1'), (0.5, 'l-foot-2'))),

    ('r-heel0',             'v', 5721),
    #('r-heel',              'p', ['r-foot-2', 'r-foot-1', 'r-heel0']),
    ('r-heel',              'l', ((-2.5,'r-foot-2'), (3.5,'r-foot-1'))),
    ('r-ankle-tip',         'o', ('r-ankle', [0.0, 0.0, -1.0])),
    ('r-loleg-fan',         'l', ((0.75, 'r-knee'), (0.25, 'r-ankle'))),

    ('l-heel0',             'v', 13338),
    #('l-heel',              'p', ['l-foot-2', 'l-foot-1', 'l-heel0']),
    ('l-heel',              'l', ((-2.5,'l-foot-2'), (3.5,'l-foot-1'))),
    ('l-ankle-tip',         'o', ('l-ankle', [0.0, 0.0, -1.0])),
    ('l-loleg-fan',         'l', ((0.75, 'l-knee'), (0.25, 'l-ankle'))),

    ('r-knee-pt',           'o', ('r-knee', [0,0,3])),
    ('l-knee-pt',           'o', ('l-knee', [0,0,3])),

    ('r-knee-head',         'v', 4500),
    ('r-knee-tail',         'v', 5703),
    ('l-knee-head',         'v', 6865),
    ('l-knee-tail',         'v', 6779),

    ('r-legout-head',       'vl', ((0.4, 6555), (0.6, 4462))),
    ('r-legout-tail',       'vl', ((1.0, 3973), (0.0, 3832))),
    ('l-legout-head',       'vl', ((0.4, 6757), (0.6, 6903))),
    ('l-legout-tail',       'vl', ((1.0, 7035), (0.0, 7177))),

    ('r-legfront-head',     'vl', ((0.8, 6562), (0.2, 4463))),
    ('r-legfront-tail',     'vl', ((0.7, 3942), (0.3, 3841))),
    ('l-legfront-head',     'vl', ((0.8, 6750), (0.2, 6902))),
    ('l-legfront-tail',     'vl', ((0.7, 7067), (0.3, 7168))),    
]

if MuscleBones:    
    LegJoints += [
    ('r-butt',              'v', 4475),
    ('l-butt',              'v', 6890),

    ('r-legback-head',      'vl', ((0.1, 6562), (0.9, 2120))),
    ('r-legback-tail',      'vl', ((0.1, 3942), (0.9, 3841))),
    ('l-legback-head',      'vl', ((0.1, 6750), (0.9, 7481))),
    ('l-legback-tail',      'vl', ((0.1, 7067), (0.9, 7168))),

    ('r-legin-head',        'vl', ((0.5, 7298), (0.5, 7258))),
    ('r-legin-tail',        'vl', ((0.2, 3973), (0.8, 3832))),
    ('l-legin-head',        'vl', ((0.5, 7298), (0.5, 7258))),
    ('l-legin-tail',        'vl', ((0.2, 7035), (0.8, 7177))),
]

LegHeadsTails = [
    # Hip = leg location
    ('Hip_L',           'r-upper-leg', ('r-upper-leg', ysmall)),
    ('Hip_R',           'l-upper-leg', ('l-upper-leg', ysmall)),

    # Leg
    ('UpLeg_L',         'r-upper-leg', 'r-knee'),
    ('LoLeg_L',         'r-knee', 'r-ankle'),
    ('Foot_L',          'r-ankle', 'r-foot-1'),
    ('Toe_L',           'r-foot-1', 'r-foot-2'),
    ('LegFK_L',         'r-heel', 'r-foot-2'),
    
    ('UpLeg_R',         'l-upper-leg', 'l-knee'),
    ('LoLeg_R',         'l-knee', 'l-ankle'),
    ('Foot_R',          'l-ankle', 'l-foot-1'),
    ('Toe_R',           'l-foot-1', 'l-foot-2'),
    ('LegFK_R',         'l-heel', 'l-foot-2'),
    
    # IK Leg
    ('UpLegIK_L',       'r-upper-leg', 'r-knee'),
    ('LoLegIK_L',       'r-knee', 'r-ankle'),
    ('Ankle_L',         'r-ankle', 'r-ankle-tip'),
    ('AnkleIK_L',       'r-ankle', 'r-ankle-tip'),
    ('LegIK_L',         'r-heel', 'r-foot-2'),
    ('ToeRev_L',        'r-foot-2', 'r-foot-1'),
    ('FootRev_L',       'r-foot-1', 'r-ankle'),

    ('UpLegIK_R',       'l-upper-leg', 'l-knee'),
    ('LoLegIK_R',       'l-knee', 'l-ankle'),
    ('Ankle_R',         'l-ankle', 'l-ankle-tip'),
    ('AnkleIK_R',       'l-ankle', 'l-ankle-tip'),
    ('LegIK_R',         'l-heel', 'l-foot-2'),
    ('ToeRev_R',        'l-foot-2', 'l-foot-1'),
    ('FootRev_R',       'l-foot-1', 'l-ankle'),

    # Deform 
    ('DfmUpLeg1_L',      'r-upper-leg', 'r-upleg2'),
    ('DfmUpLeg2_L',      'r-upleg2', 'r-knee'),
    ('DfmLoLeg_L',      'r-knee', 'r-ankle'),
    ('DfmLoLegFan_L',   'r-knee', 'r-loleg-fan'),
    ('DfmFoot_L',       'r-ankle', 'r-foot-1'),
    ('DfmToe_L',        'r-foot-1', 'r-foot-2'),

    ('DfmUpLeg1_R',      'l-upper-leg', 'l-upleg2'),
    ('DfmUpLeg2_R',      'l-upleg2', 'l-knee'),
    ('DfmLoLeg_R',      'l-knee', 'l-ankle'),
    ('DfmLoLegFan_R',   'l-knee', 'l-loleg-fan'),
    ('DfmFoot_R',       'l-ankle', 'l-foot-1'),
    ('DfmToe_R',        'l-foot-1', 'l-foot-2'),

    # Tweaks
    ('UpLegVec_L',      'r-upper-leg', 'r-upleg1'),
    ('UpLegVec_R',      'l-upper-leg', 'l-upleg1'),
    ('UpLegVecNeg_L',    'r-upper-leg', 'r-upleg1'),
    ('UpLegVecNeg_R',    'l-upper-leg', 'l-upleg1'),

    # Pole Targets
    #('LegTrg_L',        'r-upper-leg', 'r-legtrg'),
    ('UpLeg2PT_L',      ('r-upleg1', (0,0,-1)), ('r-upleg1', (0,0,-2))),
    ('UpLeg3PT_L',      ('r-upleg2', (0,0,-1)), ('r-upleg2', (0,0,-2))),
    ('KneePT_L',        'r-knee-pt', ('r-knee-pt', offs)),
    ('KneePTFK_L',      'r-knee-pt', ('r-knee-pt', offs)),
    ('KneeLinkPT_L',    'r-knee', 'r-knee-pt'),
    ('FootPT_L',        ('r-midfoot', (0,1,0.2)), ('r-midfoot', (0,1.3,0.2))),
    ('ToePT_L',         ('r-midtoe', (0,1,0)), ('r-midtoe', (0,1.3,0))),

    #('LegTrg_R',        'l-upper-leg', 'l-legtrg'),
    ('UpLeg2PT_R',      ('l-upleg1', (0,0,-1)), ('l-upleg1', (0,0,-2))),
    ('UpLeg3PT_R',      ('l-upleg2', (0,0,-1)), ('l-upleg2', (0,0,-2))),
    ('KneePT_R',        'l-knee-pt', ('l-knee-pt', offs)),
    ('KneePTFK_R',      'l-knee-pt', ('l-knee-pt', offs)),
    ('KneeLinkPT_R',    'l-knee', 'l-knee-pt'),
    ('FootPT_R',        ('l-midfoot', (0,1,0.2)), ('l-midfoot', (0,1.3,0.2))),
    ('ToePT_R',         ('l-midtoe', (0,1,0)), ('l-midtoe', (0,1.3,0))),

    # Muscles
    ('DfmLegOut_L',     'r-legout-head', 'r-legout-tail'),
    ('DfmLegOut_R',     'l-legout-head', 'l-legout-tail'),
    #('DfmLegFront_L',   'r-legfront-head', 'r-legfront-tail'),
    #('DfmLegFront_R',   'l-legfront-head', 'l-legfront-tail'),

    ('LegOutTrg_L',     'r-knee', 'r-legout-tail'),
    ('LegOutTrg_R',     'l-knee', 'l-legout-tail'),
    ('LegFrontTrg_L',   'r-knee', 'r-legfront-tail'),
    ('LegFrontTrg_R',   'l-knee', 'l-legfront-tail'),

    # Fan bones    
    ('DfmHipFan_L',      'r-upper-leg', 'r-upleg1'),
    ('DfmHipFan_R',      'l-upper-leg', 'l-upleg1'),

    ('DfmKneeFan_L',      'r-knee', 'r-loleg1'),
    ('DfmKneeFan_R',      'l-knee', 'l-loleg1'),

    ('DfmKneeBack_L',   'r-knee-stretch1', 'r-knee-stretch2'),
    ('DfmKneeBack_R',   'l-knee-stretch1', 'l-knee-stretch2'),
    ('KneeBackTrg_L',   'r-ankle', 'r-knee-stretch2'),
    ('KneeBackTrg_R',   'l-ankle', 'l-knee-stretch2'),

    # Directions    
    ('DirUpLegFwd_L',     'r-upper-leg', ('r-upper-leg', (0,0,1))),
    ('DirUpLegFwd_R',     'l-upper-leg', ('l-upper-leg', (0,0,1))),
    ('DirUpLegBack_L',    'r-upper-leg', ('r-upper-leg', (0,0,-1))),
    ('DirUpLegBack_R',    'l-upper-leg', ('l-upper-leg', (0,0,-1))),
    ('DirUpLegOut_L',     'r-upper-leg', ('r-upper-leg', (1,0,0))),
    ('DirUpLegOut_R',     'l-upper-leg', ('l-upper-leg', (-1,0,0))),

    ('DirKneeBack_L',     'r-knee', ('r-knee', (0,0,-1))),
    ('DirKneeBack_R',     'l-knee', ('l-knee', (0,0,-1))),
    ('DirKneeInv_L',      'r-knee', ('r-knee', (0,1,0))),
    ('DirKneeInv_R',      'l-knee', ('l-knee', (0,1,0))),
]

#
#   LegArmature
#

footRoll = 3.0665693283081055
toeRoll = 0
footCtrlRoll = 0

LegArmature = [
    # Leg
    ('UpLeg_L',         0, 'Hip_L', F_WIR, L_LLEGFK, NoBB),
    ('LoLeg_L',         0, 'UpLeg_L', F_WIR+F_SCALE, L_LLEGFK, NoBB),
    ('Foot_L',          footRoll, 'LoLeg_L', F_WIR+F_CON, L_LLEGFK+L_LEXTRA, NoBB),
    ('Toe_L',           toeRoll, 'Foot_L', F_WIR, L_LLEGFK+L_LEXTRA, NoBB),
    ('LegFK_L',         footCtrlRoll, 'Toe_L', 0, L_HELP2, NoBB),

    ('UpLeg_R',         -0, 'Hip_R', F_WIR, L_RLEGFK, NoBB),
    ('LoLeg_R',         -0, 'UpLeg_R', F_WIR+F_SCALE, L_RLEGFK, NoBB),
    ('Foot_R',          -footRoll, 'LoLeg_R', F_WIR+F_CON, L_RLEGFK+L_REXTRA, NoBB),
    ('Toe_R',           -toeRoll, 'Foot_R', F_WIR, L_RLEGFK+L_REXTRA, NoBB),
    ('LegFK_R',         footCtrlRoll, 'Toe_R', 0, L_HELP2, NoBB),  
    
    # IK Leg
    
    ('UpLegIK_L',       0, 'Hip_L', 0, L_HELP2, NoBB),
    ('LoLegIK_L',       0, 'UpLegIK_L', 0, L_HELP2, NoBB),
    ('LegIK_L',         -footCtrlRoll, Master, F_WIR, L_LLEGIK, NoBB),
    ('ToeRev_L',        0, 'LegIK_L', F_WIR, L_LLEGIK, NoBB),
    ('FootRev_L',       0, 'ToeRev_L', F_WIR, L_LLEGIK, NoBB),
    ('Ankle_L',         0, Master, F_WIR, L_LEXTRA, NoBB),
    ('AnkleIK_L',       0, 'FootRev_L', 0, L_HELP2, NoBB),

    ('UpLegIK_R',       -0, 'Hip_R', 0, L_HELP2, NoBB),
    ('LoLegIK_R',       -0, 'UpLegIK_R', 0, L_HELP2, NoBB),
    ('LegIK_R',         -footCtrlRoll, Master, F_WIR, L_RLEGIK, NoBB),
    ('ToeRev_R',        0, 'LegIK_R', F_WIR, L_RLEGIK, NoBB),
    ('FootRev_R',       0, 'ToeRev_R', F_WIR, L_RLEGIK, NoBB),
    ('Ankle_R',         0, Master, F_WIR, L_REXTRA, NoBB),
    ('AnkleIK_R',       0, 'FootRev_R', 0, L_HELP2, NoBB),

    # Tweaks
    ('UpLegVec_L',      0, 'Hip_L', 0, L_HELP, NoBB),
    ('UpLegVec_R',      0, 'Hip_R', 0, L_HELP, NoBB),
    ('UpLegVecNeg_L',   -90*D, 'UpLegVec_L', 0, L_HELP, NoBB),
    ('UpLegVecNeg_R',   90*D, 'UpLegVec_R', 0, L_HELP, NoBB),
    
    # Deform
    ('DfmUpLeg1_L',      0, 'Hip_L', F_DEF, L_DEF, NoBB),
    ('DfmUpLeg2_L',      0, 'UpLeg_L', F_DEF, L_DEF, NoBB),
    ('DfmLoLeg_L',      0, 'LoLeg_L', F_DEF, L_DEF, NoBB),
    #('DfmLoLegFan_L',   0, 'UpLeg_L', F_DEF, L_DEF, NoBB),
    ('DfmFoot_L',       footRoll, 'Foot_L', F_DEF, L_DEF, NoBB),
    ('DfmToe_L',        toeRoll, 'Toe_L', F_DEF, L_DEF, NoBB),

    ('DfmUpLeg1_R',      0, 'Hip_R', F_DEF, L_DEF, NoBB),
    ('DfmUpLeg2_R',      0, 'UpLeg_R', F_DEF, L_DEF, NoBB),
    ('DfmLoLeg_R',      -0, 'LoLeg_R', F_DEF, L_DEF, NoBB),
    #('DfmLoLegFan_R',   -0, 'UpLeg_R', F_DEF, L_DEF, NoBB),
    ('DfmFoot_R',       -footRoll, 'Foot_R', F_DEF, L_DEF, NoBB),
    ('DfmToe_R',        -toeRoll, 'Toe_R', F_DEF, L_DEF, NoBB),

    # Pole targets
    #('LegTrg_L',        0.0, 'Hip_L', 0, L_HELP2, NoBB),
    ('KneePT_L',        0.0, 'FootRev_L', F_WIR, L_LLEGIK+L_LEXTRA, NoBB),
    ('KneePTFK_L',      0.0, 'UpLeg_L', 0, L_HELP2, NoBB),
    ('KneeLinkPT_L',    0.0, 'UpLeg_L', F_RES, L_LLEGIK+L_LEXTRA, NoBB),
    ('FootPT_L',        0.0, 'FootRev_L', 0, L_HELP2, NoBB),
    ('ToePT_L',         0.0, 'ToeRev_L', 0, L_HELP2, NoBB),

    #('LegTrg_R',        0.0, 'Hip_R', 0, L_HELP2, NoBB),
    ('KneePT_R',        0.0, 'FootRev_R', F_WIR, L_RLEGIK+L_REXTRA, NoBB),
    ('KneePTFK_R',      0.0, 'UpLeg_R', 0, L_HELP2, NoBB),
    ('KneeLinkPT_R',    0.0, 'UpLeg_R', F_RES, L_RLEGIK+L_REXTRA, NoBB),
    ('FootPT_R',        0.0, 'FootRev_R', 0, L_HELP2, NoBB),
    ('ToePT_R',         0.0, 'ToeRev_R', 0, L_HELP2, NoBB),

    # Muscles
    ('LegOutTrg_L',     0, 'DfmUpLeg1_L', 0, L_HELP, NoBB),
    ('LegOutTrg_R',     0, 'DfmUpLeg1_R', 0, L_HELP, NoBB),
    ('DfmLegOut_L',     0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    ('DfmLegOut_R',     0, 'DfmHips', F_DEF, L_MSCL, NoBB),

    #('DfmLegFront_L',   0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    #('DfmLegFront_R',   0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    #('LegFrontTrg_L',   0, 'DfmUpLeg1_L', 0, L_HELP, NoBB),
    #('LegFrontTrg_R',   0, 'DfmUpLeg1_R', 0, L_HELP, NoBB),

    # Fan bones
    ('DfmHipFan_L',     0, 'Hip_L', F_DEF, L_MSCL, NoBB),
    ('DfmHipFan_R',     0, 'Hip_R', F_DEF, L_MSCL, NoBB),
    
    ('DfmKneeFan_L',   0, 'DfmUpLeg2_L', F_DEF, L_MSCL, NoBB),
    ('DfmKneeFan_R',   0, 'DfmUpLeg2_R', F_DEF, L_MSCL, NoBB),

    ('KneeBackTrg_L',   0, 'DfmLoLeg_L', 0, L_HELP, NoBB),
    ('KneeBackTrg_R',   0, 'DfmLoLeg_R', 0, L_HELP, NoBB),
    ('DfmKneeBack_L',   0, 'DfmUpLeg2_L', F_DEF, L_MSCL, NoBB),
    ('DfmKneeBack_R',   0, 'DfmUpLeg2_R', F_DEF, L_MSCL, NoBB),

    # Directions
    ('DirUpLegFwd_L',       180*D, 'Hip_L', 0, L_HELP, NoBB),
    ('DirUpLegFwd_R',       180*D, 'Hip_R', 0, L_HELP, NoBB),
    ('DirUpLegBack_L',      0*D, 'Hip_L', 0, L_HELP, NoBB),
    ('DirUpLegBack_R',      0*D, 'Hip_R', 0, L_HELP, NoBB),
    ('DirUpLegOut_L',       -90*D, 'Hip_L', 0, L_HELP, NoBB), 
    ('DirUpLegOut_R',       90*D, 'Hip_R', 0, L_HELP, NoBB),

    ('DirKneeBack_L',       0*D, 'UpLeg_L', 0, L_HELP, NoBB),
    ('DirKneeBack_R',       0*D, 'UpLeg_R', 0, L_HELP, NoBB),
    ('DirKneeInv_L',        0*D, 'UpLeg_L', 0, L_HELP, NoBB),
    ('DirKneeInv_R',        0*D, 'UpLeg_R', 0, L_HELP, NoBB),
]

if MuscleBones:
    LegArmature += [

    # Pubis
    ('Pubis_L',            0, 'DfmUpLeg1_L', 0, L_HELP, NoBB),
    ('Pubis_R',            0, 'DfmUpLeg1_R', 0, L_HELP, NoBB),

    # Muscles
    ('DfmLegback_L',    0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    ('DfmLegback_R',    0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    ('DfmLegIn_L',      0, 'DfmHips', F_DEF, L_MSCL, NoBB),
    ('DfmLegIn_R',      0, 'DfmHips', F_DEF, L_MSCL, NoBB),

    ('LegbackTrg_L',    0, 'DfmUpLeg1_L', 0, L_HELP, NoBB),
    ('LegbackTrg_R',    0, 'DfmUpLeg1_R', 0, L_HELP, NoBB),
    ('LegInTrg_L',      0, 'DfmUpLeg1_L', 0, L_HELP, NoBB),
    ('LegInTrg_R',      0, 'DfmUpLeg1_R', 0, L_HELP, NoBB),
    
]

#
#   LegControlPoses(fp, amt):
#

limUpLeg_L = (-160*D,120*D, -90*D,90*D, -170*D,80*D)
limUpLeg_R = (-160*D,120*D, -90*D,90*D, -80*D,170*D)

limLoLeg_L = (-20*D,170*D,-40*D,40*D, 0,0)
limLoLeg_R = (-20*D,170*D,-40*D,40*D, 0,0)

limFoot_L = (-90*D,45*D, -30*D,15*D, 0,0)
limFoot_R = (-90*D,45*D, -15*D,30*D, 0,0)

limToe_L = (-20*D,60*D, 0,0, 0,0)
limToe_R = (-20*D,60*D, 0,0, 0,0)

limRevFoot_L = (-20*D,60*D, 0,0, 0,0)
limRevFoot_R = (-20*D,60*D, 0,0, 0,0)

limRevToe_L = (-10*D,45*D, 0,0, 0,0)
limRevToe_R = (-10*D,45*D, 0,0, 0,0)

Hint = 18*D

def LegControlPoses(fp, amt):
    deltaKnee = -2.5*D

    # Leg
    addPoseBone(fp, amt, 'Hip_L', 'MHBall025', None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'Hip_R', 'MHBall025', None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'UpLeg_L', 'GZM_Circle025', 'FK_L', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limUpLeg_L, (1,1,1)]),
         #('CopyTrans', 0, 0, ['LegIK', 'UpLegIK_L', 0]),
         ('IK', 0, 0, ['LegIK', 'LoLegIK_L', 1, None, (1,0,0)]),
        ])

    addPoseBone(fp, amt, 'UpLeg_R', 'GZM_Circle025', 'FK_R', (1,1,1), (0,0,0), (1,0,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limUpLeg_R, (1,1,1)]),
         #('CopyTrans', 0, 0, ['LegIK', 'UpLegIK_R', 0]),
         ('IK', 0, 0, ['LegIK', 'LoLegIK_R', 1, None, (1,0,0)]),
        ])

    addPoseBone(fp, amt, 'LoLeg_L', 'GZM_Circle025', 'FK_L', (1,1,1), (0,0,0), (1,0,1), (1,1,1), P_YZX, 
        [
         ('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limLoLeg_L, (1,1,1)]),
         #('CopyTrans', 0, 0, ['LegIK', 'LoLegIK_L', 0]),
         ('IK', 0, 0, ['LegIK', 'AnkleIK_L', 1, None, (1,0,0)]),
        ])

    addPoseBone(fp, amt, 'LoLeg_R', 'GZM_Circle025', 'FK_R', (1,1,1), (0,0,0), (1,0,1), (1,1,1), P_YZX, 
        [
         ('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limLoLeg_R, (1,1,1)]),
         #('CopyTrans', 0, 0, ['LegIK', 'LoLegIK_R', 0]),
         ('IK', 0, 0, ['LegIK', 'AnkleIK_R', 1, None, (1,0,0)]),
        ])

    addPoseBone(fp, amt, 'Foot_L', 'MHFoot', 'FK_L', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limFoot_L, (1,1,1)]),
         ('IK', 0, 0, ['RevIK', 'FootRev_L', 1, (90*D, 'FootPT_L'), (1,0,1)]),
         ('IK', 0, 0, ['FreeIK', None, 2, None, (True, False,True)])])

    addPoseBone(fp, amt, 'Foot_R', 'MHFoot', 'FK_R', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0, 
        [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limFoot_R, (1,1,1)]),
         ('IK', 0, 0, ['RevIK', 'FootRev_R', 1, (90*D, 'FootPT_R'), (1,0,1)]),
         ('IK', 0, 0, ['FreeIK', None, 2, None, (True, False,True)])])

    addPoseBone(fp, amt, 'Toe_L', 'MHToe_L', 'FK_L', (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, 
        [('IK', 0, 0, ['RevIK', 'ToeRev_L', 1, (90*D, 'ToePT_L'), (1,0,1)])])

    addPoseBone(fp, amt, 'Toe_R', 'MHToe_R', 'FK_R', (1,1,1), (0,1,1), (1,1,1), (1,1,1), 0, 
        [('IK', 0, 0, ['RevIK', 'ToeRev_R', 1, (90*D, 'ToePT_R'), (1,0,1)])])
    
    # Leg IK

    if True:
        addPoseBone(fp, amt, 'UpLegIK_L', None, 'IK_L', (1,1,1), (0,0,0), (1,0,1), 
                    ((1,1,1), (0,0,0), 0.05, limUpLeg_L), 0, [])

        addPoseBone(fp, amt, 'UpLegIK_R', None, 'IK_R', (1,1,1), (0,0,0), (1,0,1), 
                    ((1,1,1), (0,0,0), 0.05, limUpLeg_R), 0, [])

        addPoseBone(fp, amt, 'LoLegIK_L', None, 'IK_L', (1,1,1), (0,0,0), (1,0,1), 
                    ((1,0,0), (0,0,0), 0.05, limLoLeg_L), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['Hint', (Hint,Hint, 0,0, 0,0), (1,0,0)]),
             ('IK', 0, 1, ['LegIK', 'AnkleIK_L', 2, (-90*D+deltaKnee, 'KneePT_L'), (1,0,0)]),
            ])

        addPoseBone(fp, amt, 'LoLegIK_R', None, 'IK_R', (1,1,1), (0,0,0), (1,0,1), 
                    ((1,0,0), (0,0,0), 0.05, limLoLeg_R), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['Hint', (Hint,Hint, 0,0, 0,0), (1,0,0)]),
             ('IK', 0, 1, ['LegIK', 'AnkleIK_R', 2, (-90*D-deltaKnee, 'KneePT_R'), (1,0,0)])
            ])

        addPoseBone(fp, amt, 'LegIK_L', 'MHFootCtrl_L', 'IK_L', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0,
            [
            #('ChildOf', C_CHILDOF, 0, ['Hip', 'Hip_L', (1,1,1), (1,1,1), (1,1,1)]),
            ('LimitDist', 0, 1, ['DistHip', 'Hip_L', 'LIMITDIST_INSIDE'])])

        addPoseBone(fp, amt, 'LegIK_R', 'MHFootCtrl_R', 'IK_R', (0,0,0), (0,0,0), (1,1,1), (1,1,1), 0,
            [
            #('ChildOf', C_CHILDOF, 0, ['Hip', 'Hip_R', (1,1,1), (1,1,1), (1,1,1)]),
            ('LimitDist', 0, 1, ['DistHip', 'Hip_R', 'LIMITDIST_INSIDE'])])

        addPoseBone(fp, amt, 'FootRev_L', 'MHRevFoot', 'IK_L', (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limRevFoot_L, (1,1,1)])])

        addPoseBone(fp, amt, 'FootRev_R', 'MHRevFoot', 'IK_R', (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0,
            [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limRevFoot_R, (1,1,1)])])

        addPoseBone(fp, amt, 'ToeRev_L', 'MHRevToe', 'IK_L', (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limRevToe_L, (1,1,1)])])

        addPoseBone(fp, amt, 'ToeRev_R', 'MHRevToe', 'IK_R', (1,1,1), (0,0,0), (1,1,1), (1,1,1), 0, 
            [('LimitRot', C_OW_LOCAL, 1, ['LimitRot', limRevToe_R, (1,1,1)])])
    
        addPoseBone(fp, amt, 'Ankle_L', 'MHBall025', 'IK_L', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'Ankle_R', 'MHBall025', 'IK_R', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'AnkleIK_L', None, None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0,
            [('CopyLoc', 0, 1, ['Foot', 'FootRev_L', (1,1,1), (0,0,0), 1, False]),
             ('CopyLoc', 0, 0, ['Ankle', 'Ankle_L', (1,1,1), (0,0,0), 0, False]) ])

        addPoseBone(fp, amt, 'AnkleIK_R', None, None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0,
            [('CopyLoc', 0, 1, ['Foot', 'FootRev_R', (1,1,1), (0,0,0), 1, False]),
             ('CopyLoc', 0, 0, ['Ankle', 'Ankle_R', (1,1,1), (0,0,0), 0, False]) ])


        # Pole target

        addPoseBone(fp, amt, 'KneePT_L', 'MHCube025', 'IK_L', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'KneeLinkPT_L', None, 'IK_L', (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
            [('StretchTo', 0, 1, ['Stretch', 'KneePT_L', 0, 1, 3.0])])

        addPoseBone(fp, amt, 'KneePT_R', 'MHCube025', 'IK_R', (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

        addPoseBone(fp, amt, 'KneeLinkPT_R', None, 'IK_R', (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
            [('StretchTo', 0, 1, ['Stretch', 'KneePT_R', 0, 1, 3.0])])


    # Upper leg deform 

    addPoseBone(fp, amt, 'UpLegVec_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoLeg_L', 1, None, (True, False,True)])])

    addPoseBone(fp, amt, 'DfmUpLeg1_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoLeg_L', 1, None, (True, False,True)])])


    addPoseBone(fp, amt, 'UpLegVec_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoLeg_R', 1, None, (True, False,True)])])

    addPoseBone(fp, amt, 'DfmUpLeg1_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('IK', 0, 1, ['IK', 'LoLeg_R', 1, None, (True, False,True)])])


    # Lower leg deform
    
    addPoseBone(fp, amt, 'DfmLoLeg_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'Foot_L', 0, 1])])

    addPoseBone(fp, amt, 'DfmLoLeg_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'Foot_R', 0, 1])])


    addPoseBone(fp, amt, 'DfmLegOut_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegOutTrg_L', 1, 1])])

    addPoseBone(fp, amt, 'DfmLegOut_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegOutTrg_R', 1, 1])])

    #addPoseBone(fp, amt, 'DfmLegFront_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
    #    [('StretchTo', 0, 1, ['Stretch', 'LegFrontTrg_L', 1, 1])])

    #addPoseBone(fp, amt, 'DfmLegFront_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
    #    [('StretchTo', 0, 1, ['Stretch', 'LegFrontTrg_R', 1, 1])])

    addPoseBone(fp, amt, 'DfmKneeBack_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'KneeBackTrg_L', 1, 1])])

    addPoseBone(fp, amt, 'DfmKneeBack_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'KneeBackTrg_R', 1, 1])])

        
    # Fan bones
    
    addPoseBone(fp, amt, 'DfmHipFan_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'DfmUpLeg1_L', (1,1,1), (0,0,0), False])])

    addPoseBone(fp, amt, 'DfmHipFan_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'DfmUpLeg1_R', (1,1,1), (0,0,0), False])])


    addPoseBone(fp, amt, 'DfmKneeFan_L', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'LoLeg_L', (1,0,1), (0,0,0), False])])

    addPoseBone(fp, amt, 'DfmKneeFan_R', None, None, (1,1,1), (1,0,1), (1,1,1), (1,1,1), 0,
        [('CopyRot', C_LOCAL, 0.5, ['Rot', 'LoLeg_R', (1,0,1), (0,0,0), False])])


    if not MuscleBones:
        return

    # Tweak
    """
    addPoseBone(fp, amt, 'DfmButt_L', None, None, (1,1,1), (0,0,0), (0,0,0), (1,1,1), 0, 
        [('StretchTo', 0, 1, ['Stretch', 'Butt_L', 0, 1])])

    addPoseBone(fp, amt, 'Butt_L', 'MHCube01', None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])

    addPoseBone(fp, amt, 'DfmButt_R', None, None, (1,1,1), (0,0,0), (0,0,0), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'Butt_R', 0, 1])])

    addPoseBone(fp, amt, 'Butt_R', 'MHCube01', None, (0,0,0), (1,1,1), (1,1,1), (1,1,1), 0, [])
    """
    # Muscles

    addPoseBone(fp, amt, 'DfmLegIn_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegInTrg_L', 1, 1])])

    addPoseBone(fp, amt, 'DfmLegIn_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegInTrg_R', 1, 1])])

    addPoseBone(fp, amt, 'DfmLegback_L', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegbackTrg_L', 1, 1])])

    addPoseBone(fp, amt, 'DfmLegback_R', None, None, (1,1,1), (1,1,1), (1,1,1), (1,1,1), 0,
        [('StretchTo', 0, 1, ['Stretch', 'LegbackTrg_R', 1, 1])])

    return

#
#   LegPropLRDrivers
#   (Bone, Name, Props, Expr)
#

LegPropLRDrivers = [
    ('UpLeg', 'LegIK', ['LegIk'], 'x1'),
    ('LoLeg', 'LegIK', ['LegIk'], 'x1'),
    ('Foot', 'RevIK', ['LegIk', 'LegIkToAnkle'], 'x1*(1-x2)'),
    ('Foot', 'FreeIK', ['LegIk'], '1-x1'),
    ('Toe', 'RevIK', ['LegIk', 'LegIkToAnkle'], 'x1*(1-x2)'),
    ('LegIK', 'DistHip', ['LegStretch'], '1-x1'),
]

SoftLegPropLRDrivers = [
    #('KneePT', 'Foot', ['KneeFollowsFoot'], 'x1'),
    #('KneePT', 'Hip', ['KneeFollowsHip', 'KneeFollowsFoot'], 'x1*(1-x2)'),  
    ('AnkleIK', 'Foot', ['LegIkToAnkle'], '1-x1'),
    ('AnkleIK', 'Ankle', ['LegIkToAnkle'], 'x1'),
]

LegPropDrivers = [
    ('UpLeg_L', 'LimitRot', ['RotationLimits', 'LegIk_L'], 'x1*(1-x2)'),
    ('LoLeg_L', 'LimitRot', ['RotationLimits', 'LegIk_L'], 'x1*(1-x2)'),    
    ('Foot_L', 'LimitRot', ['RotationLimits', 'LegIk_L'], 'x1*(1-x2)'),    

    ('UpLeg_R', 'LimitRot', ['RotationLimits', 'LegIk-R'], 'x1*(1-x2)'),
    ('LoLeg_R', 'LimitRot', ['RotationLimits', 'LegIk_R'], 'x1*(1-x2)'),    
    ('Foot_R', 'LimitRot', ['RotationLimits', 'LegIk_R'], 'x1*(1-x2)'),    
]

#
#   LegDeformDrivers
#   Bone : (constraint, driver, rotdiff, keypoints)
#

LegDeformDrivers = []

#
#   LegShapeDrivers
#   Shape : (driver, rotdiff, keypoints)
#

LegShapeDrivers = {
}

expr90 = "%.3f*(1-%.3f*x1)" % (90.0/90.0, 2/pi)
expr70 = "%.3f*(1-%.3f*x1)" % (90.0/70.0, 2/pi)
expr60 = "%.3f*(1-%.3f*x1)" % (90.0/60.0, 2/pi)
expr45 = "%.3f*(1-%.3f*x1)" % (90.0/45.0, 2/pi)
expr90_90 = "%.3f*max(1-%.3f*x1,0)*max(1-%.3f*x2,0)" % (90.0/90.0, 2/pi, 2/pi)


HipTargetDrivers = []
"""
    ("legs-forward-90", "LR", expr90,
        [("UpLegVec", "DirUpLegFwd")]),
    ("legs-back-60", "LR", expr60,
        [("UpLegVec", "DirUpLegBack")]),
    ("legs-out-90", "LR", expr90_90,
        [("UpLegVec", "DirUpLegOut"),
         ("UpLeg", "UpLegVec")]),
    ("legs-out-90-neg-90", "LR", expr90_90,
        [("UpLegVec", "DirUpLegOut"),
         ("UpLeg", "UpLegVecNeg")]),
]
"""
KneeTargetDrivers = [
#    ("lolegs-back-90", "LR", expr90,
#        [("LoLeg", "DirKneeBack")]),
#    ("lolegs-back-135", "LR", expr45,
#        [("LoLeg", "DirKneeInv")]),
]




```

# Facial rendering source scan: makehumancommunity/makehuman-plugin-for-blender

Repository: https://github.com/makehumancommunity/makehuman-plugin-for-blender

Pinned source revision: a0f83ba4e9a3625de3dd6f90366dbafedb4160d8

Discovery topic: MakeHuman

Description: !DEPRECATED! Plugin for makehuman and related programs (it has been replaced with MPFB2)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["blender_source/MH_Community/operators/separateeyes.py", "blender_source/MH_Community/separate_eyes.py", "doc-assist/eye_before.png", "doc-assist/eyes_after.png"], "skin": ["blender_source/MH_Community/data/nodes/skinMaterial.json", "blender_source/MH_Community/data/nodes/skinMaterialSSS.json", "blender_source/MH_Community/data/nodes/skinOverridePresets/asian.json", "blender_source/MH_Community/data/nodes/skinOverridePresets/default.json", "blender_source/MH_Community/data/nodes/skinOverridePresets/pale.json", "blender_source/MH_Community/data/nodes/skinOverridePresets/tan.json"], "sss": ["blender_source/MH_Community/data/nodes/skinMaterialSSS.json", "blender_source/MH_Community/data/textures/sss.png"], "rig": ["blender_source/MH_Community/operators/actiontrimright.py", "blender_source/MH_Community/operators/poseright.py", "blender_source/MH_Community/operators/removefingerrig.py", "blender_source/MH_Community/operators/removeikrig.py", "blender_source/MH_Community/operators/snaponfingerrig.py", "blender_source/MH_Community/operators/snaponikrig.py", "blender_source/MH_Community/operators/toSensorRig.py", "blender_source/MH_Community/rig/__init__.py", "blender_source/MH_Community/rig/bonesurgery.py", "blender_source/MH_Community/rig/cmuriginfo.py", "blender_source/MH_Community/rig/defaultriginfo.py", "blender_source/MH_Community/rig/fingerrig.py", "blender_source/MH_Community/rig/gameriginfo.py", "blender_source/MH_Community/rig/ikrig.py", "blender_source/MH_Community/rig/kinect2riginfo.py", "blender_source/MH_Community/rig/riginfo.py", "doc-assist/rig_after.png", "doc-assist/rig_before.png"], "face": ["blender_source/MH_Community/operators/amputateface.py"]}

## Source file: blender_source/MH_Community/rig/ikrig.py

{
  "path": "blender_source/MH_Community/rig/ikrig.py",
  "blob_sha": "9ab500234ec228859aace63b2d8b5833182852f0",
  "sha256": "c14b1c40ba7a12ee6f36bdab74dad6007cc31166a7e3d089ebdd8fa6a25a1c3f",
  "lines": 238,
  "bytes": 9778,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 4,
    "rig": 30
  },
  "url": "https://github.com/makehumancommunity/makehuman-plugin-for-blender/blob/a0f83ba4e9a3625de3dd6f90366dbafedb4160d8/blender_source/MH_Community/rig/ikrig.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, rigInfo):"
    },
    {
      "line": 14,
      "text": "def add(self):"
    },
    {
      "line": 72,
      "text": "def addElbowAndHandIK(self, isLeft):"
    },
    {
      "line": 115,
      "text": "def addKneeAndFootIK(self, isLeft):"
    },
    {
      "line": 158,
      "text": "def addCopyRotation(self, boneName, subtargetName):"
    },
    {
      "line": 171,
      "text": "def addIK_Constraint(self, boneName, ikBoneName, chain_count):"
    },
    {
      "line": 185,
      "text": "def changeLocks(self, locked):"
    },
    {
      "line": 203,
      "text": "def remove(self):"
    },
    {
      "line": 215,
      "text": "def removeSide(self, isLeft):"
    },
    {
      "line": 225,
      "text": "def demolish(self, controlBoneName, boneNames):"
    }
  ]
}

```text


import bpy
from . import DefaultRigInfo
from ..util import bl28


# ===============================================================================
class IkRig():
    def __init__(self, rigInfo):
        self.rigInfo = rigInfo
        self.armature = self.rigInfo.armature

    # ===============================================================================
    def add(self):
        bpy.ops.object.mode_set(mode='POSE')
        bpy.ops.pose.select_all(action='SELECT')
        bpy.ops.pose.transforms_clear()

        if bl28():
            self.armature.data.display_type = 'BBONE'
        else:
            self.armature.data.draw_type = 'BBONE'

        # make all regular bone slightly smaller, so IK's fit around
        unitMult = 0.1 * self.rigInfo.unitMultplierToExported()
        val = 0.6 * unitMult
        bpy.ops.transform.transform(mode='BONE_SIZE', value=(val, val, val, 0))
        bpy.ops.pose.select_all(action='DESELECT')

        self.changeLocks(False)

        self.addElbowAndHandIK(True)
        self.addElbowAndHandIK(False)

        self.addKneeAndFootIK(True)
        self.addKneeAndFootIK(False)
        bpy.ops.transform.transform(mode='BONE_SIZE', value=(unitMult, unitMult, unitMult, 0))

        if isinstance(self.rigInfo, DefaultRigInfo):
            pBones = self.armature.pose.bones
            locks = self.rigInfo.additionalLocks()

            for key in locks:
                lock = locks[key]
                bone = pBones[key]
                bone.lock_ik_x = lock["lockX"]
                bone.lock_ik_y = lock["lockY"]
                bone.lock_ik_z = lock["lockZ"]

                RADIAN = 3.14 / 180.0

                if not lock["limitXMin"] is None or not lock["limitXMax"] is None:
                    bone.use_ik_limit_x = True
                    if not lock["limitXMin"] is None:
                        bone.ik_min_x = lock["limitXMin"] * RADIAN
                    if not lock["limitXMax"] is None:
                        bone.ik_max_x = lock["limitXMax"] * RADIAN
                if not lock["limitYMin"] is None or not lock["limitYMax"] is None:
                    bone.use_ik_limit_y = True
                    if not lock["limitYMin"] is None:
                        bone.ik_min_y = lock["limitYMin"] * RADIAN
                    if not lock["limitYMax"] is None:
                        bone.ik_max_y = lock["limitYMax"] * RADIAN
                if not lock["limitZMin"] is None or not lock["limitZMax"] is None:
                    bone.use_ik_limit_z = True
                    if not lock["limitZMin"] is None:
                        bone.ik_min_z = lock["limitZMin"] * RADIAN
                    if not lock["limitZMax"] is None:
                        bone.ik_max_z = lock["limitZMax"] * RADIAN

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def addElbowAndHandIK(self, isLeft):
        side = 'L' if isLeft else 'R'

        # add bones while in edit mode
        bpy.ops.object.mode_set(mode='EDIT')
        eBones = self.armature.data.edit_bones

        upperArmName = self.rigInfo.upperArm(isLeft)
        upperArm = eBones[upperArmName]

        lowerArmName = self.rigInfo.lowerArm(isLeft)

        handName = self.rigInfo.hand(isLeft)
        hand = eBones[handName]
        hand.hide = True
        # - - - - - - - -
        elbowHead = upperArm.tail.copy()
        elbowHead.y = abs(elbowHead.y) * -4  # always forward, negative
        elbowTail = elbowHead.copy()
        elbowTail.y = elbowTail.y * 2

        elbowIKName = 'elbow.ik.' + side
        elbowIK = eBones.new(elbowIKName)
        elbowIK.head = elbowHead
        elbowIK.tail = elbowTail
        #elbowIK.parent = eBones[self.rigInfo.root]
        elbowIK.use_deform = False
        elbowIK.select = True
        # - - - - - - - -
        handIKName = 'hand.ik.' + side
        handIK = eBones.new(handIKName)
        handIK.head = hand.head.copy()
        handIK.tail = hand.tail.copy()
        handIK.roll = hand.roll
        #handIK.parent = eBones[self.rigInfo.root]
        handIK.use_deform = False
        handIK.select = True
        # - - - - - - - -
        self.addIK_Constraint(upperArmName, elbowIKName, self.rigInfo.elbowIKChainLength)
        self.addIK_Constraint(lowerArmName, handIKName, self.rigInfo.handIKChainLength)
        self.addCopyRotation(handName, handIKName)

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def addKneeAndFootIK(self, isLeft):
        side = 'L' if isLeft else 'R'

        # add bones while in edit mode
        bpy.ops.object.mode_set(mode='EDIT')
        eBones = self.armature.data.edit_bones

        thighName = self.rigInfo.thigh(isLeft)
        thigh = eBones[thighName]

        calfName = self.rigInfo.calf(isLeft)

        footName = self.rigInfo.foot(isLeft)
        foot = eBones[footName]
        foot.hide = True

        kneeHead = thigh.tail.copy()
        kneeHead.y = abs(kneeHead.y) * -10  # always forward, negative
        kneeTail = kneeHead.copy()
        kneeTail.y = kneeHead.y * 1.5

        kneeIKName = 'knee.ik.' + side
        kneeIK = eBones.new(kneeIKName)
        kneeIK.head = kneeHead
        kneeIK.tail = kneeTail
        #kneeIK.parent = eBones[self.rigInfo.root]
        kneeIK.use_deform = False
        kneeIK.select = True
        # - - - - - - - -
        footIKName = 'foot.ik.' + side
        footIK = eBones.new(footIKName)
        footIK.head = foot.head.copy()
        footIK.tail = foot.tail.copy()
        footIK.roll = foot.roll
        #footIK.parent = eBones[self.rigInfo.root]
        footIK.use_deform = False
        footIK.select = True
        # - - - - - - - -
        self.addIK_Constraint(thighName, kneeIKName, self.rigInfo.kneeIKChainLength)
        self.addIK_Constraint(calfName, footIKName, self.rigInfo.footIKChainLength)
        self.addCopyRotation(footName, footIKName)

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def addCopyRotation(self, boneName, subtargetName):
        print('adding copy rotation constraint to ' + boneName + ', with sub target ' + subtargetName)
        # apply constraints to the pose bone version
        bpy.ops.object.mode_set(mode='POSE')
        pBones = self.armature.pose.bones

        pBone = pBones[boneName]
        con = pBone.constraints.new('COPY_ROTATION')
        con.target = self.armature
        con.subtarget = subtargetName
        con.name = 'IK_SNAPON_ROT'

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def addIK_Constraint(self, boneName, ikBoneName, chain_count):
        print('adding IK constraint to ' + boneName + ', with sub target ' + ikBoneName + ', and chain length ' + str(chain_count))
        # apply constraints to the pose bone version
        bpy.ops.object.mode_set(mode='POSE')
        pBones = self.armature.pose.bones

        pBone = pBones[boneName]
        con = pBone.constraints.new('IK')
        con.target = self.armature
        con.subtarget = ikBoneName
        con.chain_count = chain_count
        con.name = 'IK_SNAPON_IK'

    # ===============================================================================
    def changeLocks(self, locked):
        # take location locks off pelvis & clavicles
        pelvis = self.armature.pose.bones[self.rigInfo.pelvis]
        pelvis.lock_location[0] = locked
        pelvis.lock_location[1] = locked
        pelvis.lock_location[2] = locked

        lClavicle = self.armature.pose.bones[self.rigInfo.clavicle(True)]
        lClavicle.lock_location[0] = locked
        lClavicle.lock_location[1] = locked
        lClavicle.lock_location[2] = locked

        rClavicle = self.armature.pose.bones[self.rigInfo.clavicle(False)]
        rClavicle.lock_location[0] = locked
        rClavicle.lock_location[1] = locked
        rClavicle.lock_location[2] = locked

    # ===============================================================================
    def remove(self):
        self.changeLocks(True)
        self.removeSide(True)
        self.removeSide(False)
        # reverse making all regular bone slightly smaller, so IK's fit around; does not work; seems value cannot be > 1 in code
        unitMult = 0.1 * self.rigInfo.unitMultplierToExported()
        val = 1 / (0.6 * unitMult)
        bpy.ops.transform.transform(mode='BONE_SIZE', value=(val, val, val, 0))

        self.armature.data.display_type = 'WIRE'

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def removeSide(self, isLeft):
        side = 'L' if isLeft else 'R'
        if 'elbow.ik.L' in self.armature.pose.bones:
            self.demolish('elbow.ik.' + side, [self.rigInfo.upperArm(isLeft)])
            self.demolish('hand.ik.' + side, [self.rigInfo.lowerArm(isLeft), self.rigInfo.hand(isLeft)])
        self.demolish('knee.ik.' + side, [self.rigInfo.thigh(isLeft)])
        self.demolish('foot.ik.' + side, [self.rigInfo.calf(isLeft), self.rigInfo.foot(isLeft)])

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    # no need of BoneSurgery module, since no weights to give back
    def demolish(self, controlBoneName, boneNames):
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.armature.select_all(action='DESELECT')
        self.armature.data.edit_bones[controlBoneName].select = True
        bpy.ops.armature.delete()

        bpy.ops.object.mode_set(mode='POSE')
        for boneName in boneNames:
            self.armature.data.bones[boneName].hide = False

        for bone in self.armature.pose.bones:
            for c in bone.constraints:
                if 'IK_SNAPON_' in c.name:
                    bone.constraints.remove(c)


```

## Source file: blender_source/MH_Community/rig/riginfo.py

{
  "path": "blender_source/MH_Community/rig/riginfo.py",
  "blob_sha": "1930b34e39e2a2030bc7d574285509124fe8c6e6",
  "sha256": "65302895a2afa9a19a531168d84234df3e984aa2d3772ea70979c596e0dd8de2",
  "lines": 191,
  "bytes": 7922,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 23
  },
  "url": "https://github.com/makehumancommunity/makehuman-plugin-for-blender/blob/a0f83ba4e9a3625de3dd6f90366dbafedb4160d8/blender_source/MH_Community/rig/riginfo.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def determineRig(armature):"
    },
    {
      "line": 30,
      "text": "def __init__(self, armature, name, uniqueBoneName):"
    },
    {
      "line": 35,
      "text": "def matches(self):"
    },
    {
      "line": 49,
      "text": "def determineExportedUnits(self):"
    },
    {
      "line": 70,
      "text": "def unitMultplierToExported(self):"
    },
    {
      "line": 77,
      "text": "def hasIKRigs(self):"
    },
    {
      "line": 79,
      "text": "def hasFingerIK(self):"
    },
    {
      "line": 81,
      "text": "def hasIK(self):"
    },
    {
      "line": 85,
      "text": "def hasFingers(self):"
    },
    {
      "line": 96,
      "text": "def isExpressionCapable(self):"
    },
    {
      "line": 99,
      "text": "def isPoseCapable(self):"
    },
    {
      "line": 103,
      "text": "def getMeshesForRig(self, scene):"
    },
    {
      "line": 114,
      "text": "def isArmBone(self, boneName):"
    },
    {
      "line": 121,
      "text": "def isFinger(self, boneName):"
    },
    {
      "line": 131,
      "text": "def pelvisInWorldSpace(self):"
    },
    {
      "line": 135,
      "text": "def rootInWorldSpace(self):"
    },
    {
      "line": 139,
      "text": "def getBoneInWorldSpace(self, bone):"
    },
    {
      "line": 149,
      "text": "def getSensorMapping(self, sensorType = 'KINECT2'):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

import bpy

class RigInfo:
    @staticmethod
    def determineRig(armature):
        from .gameriginfo import GameRigInfo
        from .defaultriginfo import DefaultRigInfo
        from .cmuriginfo import CMURigInfo
        from .kinect2riginfo import Kinect2RigInfo

        # in the case where a test bone (wt dot) is not truely unique, order of tests might be important
        game = GameRigInfo(armature)
        if game.matches(): return game

        default = DefaultRigInfo(armature)
        if default.matches(): return default

        cmu = CMURigInfo(armature)
        if cmu.matches(): return cmu

        kinect2 = Kinect2RigInfo(armature)
        if kinect2.matches(): return kinect2

        return None
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    # pass not only a unique bone name, but one that has a dot.  Collada would change that to a '_'
    def __init__(self, armature, name, uniqueBoneName):
        self.armature = armature
        self.name = name
        self.uniqueBoneName = uniqueBoneName
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def matches(self):
        boneWithoutDot = self.uniqueBoneName.replace(".", "_")
        for bone in self.armature.data.bones:
            # test without dot version first in case skeleton has no dots
            if bone.name == boneWithoutDot:
                self.dot = '_'
                return True

            if bone.name == self.uniqueBoneName:
                self.dot = '.'
                return True

        return False
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def determineExportedUnits(self):
        if len(self.armature.data.exportedUnits) > 0: return self.armature.data.exportedUnits

        current_mode = bpy.context.object.mode
        bpy.ops.object.mode_set(mode='EDIT')
        eBones = self.armature.data.edit_bones

        headTail = eBones[self.head].tail.z
        footTail = eBones[self.foot(False)].tail.z
        bpy.ops.object.mode_set(mode=current_mode)

        totalHeight = headTail - footTail # done this way to be feet on ground independent
        if totalHeight < 5: ret = 'METERS' # decimeter threshold
        elif totalHeight <= 22: ret = 'DECIMETERS' # 21.7 to 23.9 is sort of no man's land of decimeters of tallest & inches of smallest
        else: ret = 'CENTIMETERS'

        print ('armature exported units is ' + ret + ', headTail: ' + str(headTail) + ', footTail: ' + str(footTail))

        self.armature.data.exportedUnits = ret
        return ret
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def unitMultplierToExported(self):
        units = self.determineExportedUnits()

        if units == 'METERS': return 1
        elif units == 'DECIMETERS': return 10
        else: return 100
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def hasIKRigs(self):
        return self.hasFingerIK() or self.hasIK()
    def hasFingerIK(self):
        return 'thumb.ik.L' in self.armature.data.bones
    def hasIK(self):
        return 'elbow.ik.L' in self.armature.data.bones

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def hasFingers(self):
        hand = self.hand(False)
        if hand not in self.armature.data.bones:
            return False

        for bone in self.armature.data.bones:
            if bone.parent is not None and bone.parent.name == hand:
                return True

        return False
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def isExpressionCapable(self):
        return 'special03' in self.armature.data.bones
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def isPoseCapable(self):
        return self.name == 'Default Rig' and not self.hasIKRigs()
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    # determine all the meshes which are controlled by skeleton,
    def getMeshesForRig(self, scene):
        meshes = []
        for object in [object for object in scene.objects]:
            if object.type == 'MESH' and len(object.vertex_groups) > 0 and self.armature == object.find_armature():
                meshes.append(object)

        return meshes

    #===========================================================================
    # for mocap support
    # the retargeting constraint space for arm bones is 'WORLD', while 'LOCAL' for rest
    def isArmBone(self, boneName):
        bones = self.clavicle(True) + self.clavicle(False) + \
                self.upperArm(True) + self.upperArm(False) + \
                self.lowerArm(True) + self.lowerArm(False) + \
                self.hand(True) + self.hand(False)
        return self.isFinger(boneName) or boneName in bones

    def isFinger(self, boneName):
        # for some skeleton, no thumbs or hand tips, so defensively coded
        if boneName == self.handTip(True ): return True
        if boneName == self.handTip(False): return True
        if boneName == self.thumb  (True ): return True
        if boneName == self.thumb  (False): return True
        return False

    # for animation scaling when compared with sensor's equivalent
    # being in world space also includes any amount the armature may have been raised / lowered to allow mesh to touch ground
    def pelvisInWorldSpace(self):
        return self.getBoneInWorldSpace(self.armature.pose.bones[self.pelvis])

    # for animation root placement
    def rootInWorldSpace(self):
        return self.getBoneInWorldSpace(self.armature.pose.bones[self.root])

    # for animation root placement
    def getBoneInWorldSpace(self, bone):
        current_mode = bpy.context.object.mode
        bpy.ops.object.mode_set(mode='POSE')

        worldMat = self.armature.convert_space(pose_bone = bone, matrix = bone.matrix, from_space = 'POSE',  to_space = 'WORLD')
        bpy.ops.object.mode_set(mode=current_mode)

        offset = worldMat.to_translation()
        return offset

    def getSensorMapping(self, sensorType = 'KINECT2'):
        if sensorType == 'KINECT2':

            return {
            # keys are kinect joints names coming from the sensor
            # values are bone names whose tail is at the joint
            'SpineBase'    : None,
            'SpineMid'     : self.pelvis,
            'SpineShoulder': self.upperSpine,

            'Neck'         : self.neckBase,
            'Head'         : self.head,

            'ShoulderLeft' : self.clavicle(True, True),
            'ElbowLeft'    : self.upperArm(True, True),
            'WristLeft'    : self.lowerArm(True, True),
            'HandLeft'     : self.hand(True, True),
            'HandTipLeft'  : self.handTip(True, True),
            'ThumbLeft'    : self.thumb(True, True),

            'ShoulderRight': self.clavicle(False, True),
            'ElbowRight'   : self.upperArm(False, True),
            'WristRight'   : self.lowerArm(False, True),
            'HandRight'    : self.hand(False, True),
            'HandTipRight' : self.handTip(False, True),
            'ThumbRight'   : self.thumb(False, True),

            'HipLeft'      : self.hip(True, True),
            'KneeLeft'     : self.thigh(True, True),
            'AnkleLeft'    : self.calf(True, True),
            'FootLeft'     : self.foot(True, True),

            'HipRight'     : self.hip(False, True),
            'KneeRight'    : self.thigh(False, True),
            'AnkleRight'   : self.calf(False, True),
            'FootRight'    : self.foot(False, True)
        }
        # add next sensor, eg., KINECT_AZURE
        elif sensorType == 'KINECT_AZURE':
            return None

        # this sensor is not supported
        else: return None


```

## Source file: blender_source/MH_Community/rig/fingerrig.py

{
  "path": "blender_source/MH_Community/rig/fingerrig.py",
  "blob_sha": "bd8d306362d068a9de9010421b9b0ac6b17403fd",
  "sha256": "b2a9c7cc415c839145b5e577d77b17d1cec3002018917bdbf481b2674bd1f489",
  "lines": 139,
  "bytes": 6205,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 19
  },
  "url": "https://github.com/makehumancommunity/makehuman-plugin-for-blender/blob/a0f83ba4e9a3625de3dd6f90366dbafedb4160d8/blender_source/MH_Community/rig/fingerrig.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self, rigInfo):"
    },
    {
      "line": 12,
      "text": "def add(self):"
    },
    {
      "line": 24,
      "text": "def buildThumb(self, isLeft):"
    },
    {
      "line": 35,
      "text": "def buildFingerPair(self, which):"
    },
    {
      "line": 39,
      "text": "def buildFinger(self, which, isLeft):"
    },
    {
      "line": 67,
      "text": "def build(self, eBones, controlBoneName, parent, fingerBoneNames, hide):"
    },
    {
      "line": 94,
      "text": "def addCopyRotation(self, boneName, subtargetName, use_z):"
    },
    {
      "line": 109,
      "text": "def remove(self):"
    },
    {
      "line": 113,
      "text": "def removeSide(self, isLeft):"
    },
    {
      "line": 122,
      "text": "def demolish(self, controlBoneName, fingerBoneNames):"
    }
  ]
}

```text


import bpy
#===============================================================================
CUSTOM_SHAPE = 'GZM_Knuckle' # name of mesh to use as custom shape if found
#===============================================================================
class FingerRig():
    def __init__(self, rigInfo):
        self.rigInfo = rigInfo
        self.armature = self.rigInfo.armature
        self.scene = bpy.context.scene
#===============================================================================
    def add(self):
        self.buildThumb(True)
        self.buildThumb(False)
        self.buildFingerPair(2)
        self.buildFingerPair(3)
        self.buildFingerPair(4)
        self.buildFingerPair(5)

        # tell BabylonJS exporter not to export IK bones
        if hasattr(self.scene, "ignoreIKBones"):
            self.scene.ignoreIKBones = True
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def buildThumb(self, isLeft):
        side = 'L' if isLeft else 'R'

        bpy.ops.object.mode_set(mode='EDIT')
        eBones = self.armature.data.edit_bones

        parent = eBones[self.rigInfo.thumbParent(isLeft)]
        thumbBoneNames = self.rigInfo.thumbBones(isLeft)

        self.build(eBones, 'thumb.ik.' + side, parent, thumbBoneNames, False)
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def buildFingerPair(self, which):
        self.buildFinger(which, True)
        self.buildFinger(which, False)
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def buildFinger(self, which, isLeft):
        side = 'L' if isLeft else 'R'

        bpy.ops.object.mode_set(mode='EDIT')
        eBones = self.armature.data.edit_bones

        if which == 2:
            name = 'index.ik.'
            parent = eBones[self.rigInfo.indexFingerParent(isLeft)]
            fingerBoneNames = self.rigInfo.indexFingerBones(isLeft)

        elif which == 3:
            name = 'middle.ik.'
            parent = eBones[self.rigInfo.middleFingerParent(isLeft)]
            fingerBoneNames = self.rigInfo.middleFingerBones(isLeft)

        elif which == 4:
            name = 'ring.ik.'
            parent = eBones[self.rigInfo.ringFingerParent(isLeft)]
            fingerBoneNames = self.rigInfo.ringFingerBones(isLeft)

        elif which == 5:
            name = 'pinky.ik.'
            parent = eBones[self.rigInfo.pinkyFingerParent(isLeft)]
            fingerBoneNames = self.rigInfo.pinkyFingerBones(isLeft)

        self.build(eBones, name + side, parent, fingerBoneNames, True)
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def build(self, eBones, controlBoneName, parent, fingerBoneNames, hide):
        lastIdx = len(fingerBoneNames) - 1
        firstFinger = eBones[fingerBoneNames[0      ]]
        lastFinger  = eBones[fingerBoneNames[lastIdx]]

        controlBone = eBones.new(controlBoneName)
        controlBone.head = firstFinger.head.copy()
        controlBone.tail = lastFinger .tail.copy()
        controlBone.parent = parent

        # make changes that must be done in pose mode
        bpy.ops.object.mode_set(mode='POSE')
        controlPBone = self.armature.pose.bones[controlBoneName]
        controlPBone.lock_location[0] = True
        controlPBone.lock_location[1] = True
        controlPBone.lock_location[2] = True

        # apply custom_shape to the pose bone version
        if CUSTOM_SHAPE in self.scene.objects:
            controlPBone.custom_shape = self.scene.objects [CUSTOM_SHAPE]

        for bIndex, boneName in enumerate(fingerBoneNames):
            self.addCopyRotation(boneName, controlBoneName, bIndex == 0)
            if hide:
                self.armature.data.bones[boneName].hide = True

    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def addCopyRotation(self, boneName, subtargetName, use_z):
        # apply constraints to the pose bone version
        bpy.ops.object.mode_set(mode='POSE')
        pBones = self.armature.pose.bones

        pBone = pBones[boneName]
        con = pBone.constraints.new('COPY_ROTATION')
        con.target = self.armature
        con.subtarget = subtargetName
        con.use_y = False
        con.use_z = use_z
        con.use_offset = True
        con.target_space = 'LOCAL'
        con.owner_space  = 'LOCAL'
#===============================================================================
    def remove(self):
        self.removeSide(True)
        self.removeSide(False)
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    def removeSide(self, isLeft):
        side = 'L' if isLeft else 'R'
        self.demolish('thumb.ik.'  + side, self.rigInfo.thumbBones       (isLeft))
        self.demolish('index.ik.'  + side, self.rigInfo.indexFingerBones (isLeft))
        self.demolish('middle.ik.' + side, self.rigInfo.middleFingerBones(isLeft))
        self.demolish('ring.ik.'   + side, self.rigInfo.ringFingerBones  (isLeft))
        self.demolish('pinky.ik.'  + side, self.rigInfo.pinkyFingerBones (isLeft))
    # - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    # no need of BoneSurgery module, since no weights to give back
    def demolish(self, controlBoneName, fingerBoneNames):
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.armature.select_all(action='DESELECT')
        self.armature.data.edit_bones[controlBoneName].select = True
        bpy.ops.armature.delete()

        bpy.ops.object.mode_set(mode='POSE')
        for bIndex, boneName in enumerate(fingerBoneNames):
            self.armature.data.bones[boneName].select = True
            self.armature.data.bones[boneName].hide = False

        for bone in bpy.context.selected_pose_bones:
            # Create a list of all the copy location constraints on this bone
            copyRotConstraints = [ c for c in bone.constraints if c.type == 'COPY_ROTATION' ]

            # Iterate over all the bone's copy location constraints and delete them all
            for c in copyRotConstraints:
                bone.constraints.remove( c ) # Remove constraint

```

## Source file: blender_source/MH_Community/separate_eyes.py

{
  "path": "blender_source/MH_Community/separate_eyes.py",
  "blob_sha": "9a6698959844b58e40d49b8d71adcfb0f1e99301",
  "sha256": "c60b91e93e2fa3c5e8776ce4a9dc2b106f47476a87e61aeeb2fd821d4fe478b3",
  "lines": 38,
  "bytes": 1683,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "rig": 5,
    "face": 1
  },
  "url": "https://github.com/makehumancommunity/makehuman-plugin-for-blender/blob/a0f83ba4e9a3625de3dd6f90366dbafedb4160d8/blender_source/MH_Community/separate_eyes.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self, combinedMesh):"
    }
  ]
}

```text

import bpy
#===============================================================================

class SeparateEyes():
    def __init__(self, combinedMesh):
        priorName = combinedMesh.name
        nVertsHalf = len(combinedMesh.data.vertices) / 2

        # switch to edit, and de-select all verts Change selection mode
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='DESELECT')
        bpy.context.tool_settings.mesh_select_mode = (True , False , False)

        #selection using code do not actually work unless in object mode
        bpy.ops.object.mode_set(mode='OBJECT')

        # select the first half of  the vertices, which is the lest side
        for vIndex, vert in enumerate(combinedMesh.data.vertices):
           if vIndex == nVertsHalf: break
           vert.select = True

        # switch back to edit & separate, which leaves the right side mesh still selected
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.separate(type='SELECTED')

        # interface is in a strange state, combinedMesh is Right side, switch to Object mode, rename, & move origin
        bpy.ops.object.mode_set(mode='OBJECT')
        combinedMesh.name = priorName + '_R'
        bpy.ops.object.origin_set(type='ORIGIN_CENTER_OF_MASS')

        # select left side, make active, rename, move origin,
        bpy.ops.object.mode_set(mode='OBJECT')
        bpy.ops.object.select_all(action='DESELECT')
        left = bpy.data.objects[priorName + '.001']
        left.select_set(True)
        bpy.context.view_layer.objects.active = left
        left.name =  priorName + '_L'
        bpy.ops.object.origin_set(type='ORIGIN_CENTER_OF_MASS')


```

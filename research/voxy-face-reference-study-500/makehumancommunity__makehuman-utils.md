# Facial rendering source scan: makehumancommunity/makehuman-utils

Repository: https://github.com/makehumancommunity/makehuman-utils

Pinned source revision: 94686ff7c743227a46ab6b35daa47c52d964437a

Discovery topic: MakeHuman

Description: Development tools for MakeHuman

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["license.txt"]

Topic paths: {"rig": ["io_mhrigging_mhskel/__init__.py", "io_mhrigging_mhskel/export_mh_rigging.py", "io_mhrigging_mhskel/import_mh_rigging.py", "io_mhrigging_mhskel/readme.txt", "io_mhrigging_mhskel/shared_mh_rigging.py", "weighting/data/vgrp_leftright.json", "weighting/genrig.py"], "face": ["weighting/gen_faceshapes.py"], "lip": ["weighting/data/vgrp_upper_lower_lip.json"]}

## Source file: weighting/genrig.py

{
  "path": "weighting/genrig.py",
  "blob_sha": "04829a4fb7bb13f6d3de8923c3fed0f0f6ebda66",
  "sha256": "7f5884d4985b3a84eda466c52f8e75af47dc3d53945bb39da385c9a818be9a6b",
  "lines": 837,
  "bytes": 22402,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 38,
    "jaw": 11,
    "mouth": 21,
    "tongue": 4,
    "rig": 72,
    "face": 13,
    "lip": 20
  },
  "url": "https://github.com/makehumancommunity/makehuman-utils/blob/94686ff7c743227a46ab6b35daa47c52d964437a/weighting/genrig.py",
  "function_signature_candidates": [
    {
      "line": 60,
      "text": "def execute(self, context):"
    },
    {
      "line": 64,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 78,
      "text": "def execute(self, context):"
    },
    {
      "line": 92,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 106,
      "text": "def execute(self, context):"
    },
    {
      "line": 110,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 124,
      "text": "def execute(self, context):"
    },
    {
      "line": 134,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 309,
      "text": "def saveAction(context, filepath):"
    },
    {
      "line": 384,
      "text": "def saveRig(context, filepath):"
    },
    {
      "line": 459,
      "text": "def makeHierarchy(rig):"
    },
    {
      "line": 471,
      "text": "def makeChildren(parent):"
    },
    {
      "line": 491,
      "text": "def writeJoints(fp, rig, ob, scn):"
    },
    {
      "line": 607,
      "text": "def getHeadLoc(bname, hjoints, connects):"
    },
    {
      "line": 618,
      "text": "def getTailLoc(bname, tjoints):"
    },
    {
      "line": 625,
      "text": "def findJoints(bname, co, joints, minDist=0.05):"
    },
    {
      "line": 636,
      "text": "def distance(co1, co2):"
    },
    {
      "line": 641,
      "text": "def findVertex(co, verts):"
    },
    {
      "line": 649,
      "text": "def getBoneName(bone):"
    },
    {
      "line": 654,
      "text": "def writeHeadsTails(fp, rig, hjoints, tjoints, connects):"
    },
    {
      "line": 664,
      "text": "def writeArmature(fp, hier, rig, ignores):"
    },
    {
      "line": 670,
      "text": "def writeHierarchy(fp, hier, parent, rig, ignores, inface):"
    },
    {
      "line": 700,
      "text": "def defineJoints(ob):"
    }
  ]
}

```text

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman-utils

**Authors:**           Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2014

**Licensing:**         AGPL3 (http://www.makehuman.org/doc/node/the_makehuman_application.html)

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

Rig

"""

import bpy
from mathutils import *
from bpy_extras.io_utils import ExportHelper
from bpy.props import *
import os
from collections import OrderedDict
from . import io_json


#------------------------------------------------------------------------
#    Buttons
#------------------------------------------------------------------------

class VIEW3D_OT_SaveRigButton(bpy.types.Operator, ExportHelper):
    bl_idname = "mhw.save_rig"
    bl_label = "Save rig"
    bl_description = "Save rig file"

    filename_ext = ".py"
    filter_glob = StringProperty(default="*.py", options={'HIDDEN'})
    filepath = StringProperty(name="File Path", maxlen=1024, default="")

    def execute(self, context):
        saveRig(context, self.filepath)
        return{'FINISHED'}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class VIEW3D_OT_SaveVertexGroupsButton(bpy.types.Operator, ExportHelper):
    bl_idname = "mhw.save_vertex_groups"
    bl_label = "Save vertex groups"
    bl_options = {'UNDO'}

    filename_ext = ".json"
    filter_glob = StringProperty(default="*.json", options={'HIDDEN'})
    filepath = StringProperty(name="File Path", maxlen=1024, default="")

    def execute(self, context):
        from .export import exportVertexGroups

        rig = context.object
        human = None
        nverts = 0
        for ob in rig.children:
            if ob.type == 'MESH' and len(ob.data.vertices) > nverts:
                human = ob
                nverts = len(ob.data.vertices)

        exportVertexGroups(context.scene, human, self.filepath)
        return{'FINISHED'}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class VIEW3D_OT_SaveActionButton(bpy.types.Operator, ExportHelper):
    bl_idname = "mhw.save_action"
    bl_label = "Save Action"
    bl_description = "Save action file"

    filename_ext = ".json"
    filter_glob = StringProperty(default="*.json", options={'HIDDEN'})
    filepath = StringProperty(name="File Path", maxlen=1024, default="")

    def execute(self, context):
        saveAction(context, self.filepath)
        return{'FINISHED'}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class VIEW3D_OT_FixJsonListButton(bpy.types.Operator, ExportHelper):
    bl_idname = "mhw.fix_json_list"
    bl_label = "Fix JSON List"
    bl_options = {'UNDO'}

    filename_ext = ".json"
    filter_glob = StringProperty(default="*.json", options={'HIDDEN'})
    filepath = StringProperty(name="File Path", maxlen=1024, default="")

    def execute(self, context):
        jlist = io_json.loadJson(self.filepath)
        string = "".join([('  ["%s", %s],\n' % tuple(elt)) for elt in jlist])
        fp = open(self.filepath, "w")
        fp.write(
            "[\n" +
            string[:-2] +
            "\n]\n")
        return{'FINISHED'}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}

#------------------------------------------------------------------------
#    Actions
#------------------------------------------------------------------------

FaceShiftNames = {
    "Neutral" : "00_neutral",
    "LeftBrowDown" : "01_BrowsD_L",
    "RightBrowDown" : "02_BrowsD_R",
    "BrowsUp" : "03_BrowsU_C",
    "LeftBrowUp" : "04_BrowsU_L",
    "RightBrowUp" : "05_BrowsU_R",
    "LeftCheekUp" : "06_CheekSquint_L",
    "RightCheekUp" : "07_CheekSquint_R",
    "ChinUp" : "08_ChinLowerRaise",
    "UpperLipUp3" : "09_ChinUpperRaise",
    "LeftEyeClose" : "10_EyeBlink_L",
    "RightEyeClose" : "11_EyeBlink_R",
    "LeftEyeDown" : "12_EyeDown_L",
    "RightEyeDown" : "13_EyeDown_R",
    "LeftEyeIn" : "14_EyeIn_L",
    "RightEyeIn" : "15_EyeIn_R",
    "LeftEyeOpen" : "16_EyeOpen_L",
    "RightEyeOpen" : "17_EyeOpen_R",
    "LeftEyeOut" : "18_EyeOout_L",
    "RightEyeOut" : "19_EyeOut_R",
    "LeftEyeLowerUp" : "20_EyeSquint_L",
    "RightEyeLowerUp" : "21_EyeSquint_R",
    "LeftEyeUp" : "22_EyeUp_L",
    "RightEyeUp" : "23_EyeUp_R",
    "JawClosedOffset" : "24_JawChew",
    "JawMoveForward" : "25_JawFwd",
    "JawMoveLeft" : "26_JawLeft",
    "JawMoveRight" : "28_JawRight",
    "JawOpen" : "27_JawOpen",
    "LipsOpenKiss" : "29_LipsFunnel",
    "LowerLipsUp" : "30_LipsLowerClose",
    "LowerLipsDown" : "31_LipsLowerDown",
    "LowerLipsDown2" : "32_LipsLowerOpen",
    "LipsKiss" : "33_LipsPucker",
    "MouthLeftSmile" : "34_LipsStretch_L",
    "MouthRightSmile" : "35_LipsStretch_R",
    "UpperLipDown" : "36_LipsUpperClose",
    "UpperLipUp" : "37_LipsUpperOpen",
    "UpperLipUp2" : "38_LipsUpperUp",
    "MouthLeftSmile2" : "39_MouthDimple_L",
    "MouthRightSmile2" : "40_MouthDimple_R",
    "MouthLeftFrown" : "41_MouthFrown_L",
    "MouthRightFrown" : "42_MouthFrown_R",
    "MouthMoveLeft" : "43_MouthLeft",
    "MouthMoveRight" : "44_MouthRight",
    "MouthLeftSmile3" : "45_MouthSmile_L",
    "MouthRightSmile3" : "46_MouthSmile_R",
    "CheeksPump" : "47_Puff",
    "FaceTension" : "48_Sneer",
}

FacePoses = [
    "LeftBrowDown",
    "RightBrowDown",
    "BrowsUp",
    "LeftBrowUp",
    "RightBrowUp",
    "LeftCheekUp",
    "RightCheekUp",
    "ChinUp",
    "UpperLipUp3",
    "LeftEyeClose",
    "RightEyeClose",
    "LeftEyeDown",
    "RightEyeDown",
    "LeftEyeIn",
    "RightEyeIn",
    "LeftEyeOpen",
    "RightEyeOpen",
    "LeftEyeOut",
    "RightEyeOut",
    "LeftEyeLowerUp",
    "RightEyeLowerUp",
    "LeftEyeUp",
    "RightEyeUp",
    "JawClosedOffset",
    "JawMoveForward",
    "JawMoveLeft",
    "JawOpen",
    "JawMoveRight",
    "LipsOpenKiss",
    "LowerLipsUp",
    "LowerLipsDown",
    "LowerLipsDown2",
    "LipsKiss",
    "MouthLeftSmile",
    "MouthRightSmile",
    "UpperLipDown",
    "UpperLipUp",
    "UpperLipUp2",
    "MouthLeftSmile2",
    "MouthRightSmile2",
    "MouthLeftFrown",
    "MouthRightFrown",
    "MouthMoveLeft",
    "MouthMoveRight",
    "MouthLeftSmile3",
    "MouthRightSmile3",
    "CheeksPump",
    "FaceTension",
]

BodyPoses = [
    "UpperArmUpLeft1",
    "UpperArmUpLeft2",
    "UpperArmForwardLeft",
    "UpperArmBackwardLeft",
    "UpperArmDownLeft",
    "UpperArmRollOutLeft",
    "UpperArmRollInLeft",
    "LowerArmBend1Left1",
    "LowerArmBend1Left2",
    "HandRollBackwardLeft",
    "HandRollForwardLeft",
    "HandBendInLeft",
    "HandBendOutLeft",
    "HandBendUpLeft",
    "HandDownLeft",
    "HeadTurnRight",
    "HeadTurnLeft",
    "HeadUp",
    "HeadDown",
    "HeadLeft",
    "HeadRight",
    "TorsoDown",
    "TorsoUp",
    "TorsoLeft",
    "TorsoRight",
    "UpperLegDownLeft",
    "UpperLegUpLeft",
    "UpperLegForwardLeft",
    "UpperLegBackwardLeft",
    "UpperLegRollOutLeft",
    "UpperLegRollInLeft",
    "LowerLegBendLeft1",
    "LowerLegBendLeft2",
    "FootTurnOutLeft",
    "FootTurnInLeft",
    "FootRollInLeft",
    "FootRollOutLeft",
    "FootDownLeft",
    "FootUpLeft",
    "Finger1CloseLeft",
    "Finger2CloseLeft",
    "Finger3CloseLeft",
    "Finger4CloseLeft",
    "Finger5CloseLeft",
    "Finger1OpenLeft",
    "Finger2OpenLeft",
    "Finger3OpenLeft",
    "Finger4OpenLeft",
    "Finger5OpenLeft",
    "Figer1RollInLeft",
    "Figer1RollOutLeft",
    "Toe1CloseLeft",
    "Toe2CloseLeft",
    "Toe3CloseLeft",
    "Toe4CloseLeft",
    "Toe5CloseLeft",
    "Toe6OpenLeft",
    "Toe7OpenLeft",
    "Toe8OpenLeft",
    "Toe9OpenLeft",
    "Toe1OpenLeft",
]

def saveAction(context, filepath):
    rig = context.object

    name = "Face"
    if name == "Face":
        poses = ["Neutral"] + FacePoses
        begin = 0
        end = 48
    elif name == "All":
        poses = ["Neutral"] + FacePoses + BodyPoses
        begin = 1
        end = 199

    fcurves = {}
    affected = {}
    times = {begin:True}

    act = rig.animation_data.action
    afcurves = []
    for fcu in act.fcurves:
        try:
            bname = fcu.data_path.split('"')[1]
        except IndexError:
            print ("Datapath '%s'" % fcu.data_path)
            continue
        if bname in ["neck", "neck2"]:
            continue
        try:
            rig.data.bones[bname]
        except KeyError:
            print("Bone %s not found" % bname)
            continue
        fcurves[bname] = {}
        affected[bname] = {}
        afcurves.append(fcu)

    print("Range", begin, end)

    for fcu in afcurves:
        bname = fcu.data_path.split('"')[1]
        values = fcurves[bname][fcu.array_index] = []
        for t in range(begin,end+1):
            y = fcu.evaluate(t)
            if fcu.array_index > 0 and abs(y) > 1e-7:
                affected[bname][t] = True
                times[t] = True
            values.append(y)

    times = list(times.keys())
    times.sort()

    struct = OrderedDict()
    struct["name"] = name
    poseTable = struct["poses"] = OrderedDict()

    for n,pname in enumerate(poses):
        t = times[n]
        quats = poseTable[pname] = {}
        for bname,aff in affected.items():
            if aff:
                fcus = fcurves[bname]
                if t in aff.keys():
                    try:
                        quat = [fcus[idx][t] for idx in range(4)]
                        quats[bname] = quat
                    except IndexError:
                        raise RuntimeError("Bug saveAction %s %s %s" % (bname, aff, t))

    io_json.saveJson(struct, filepath, maxDepth=0)


#------------------------------------------------------------------------
#    Rig
#------------------------------------------------------------------------

def saveRig(context, filepath):
    rig = context.object
    ob = rig.children[0]
    scn = context.scene
    fp = open(filepath, "w")

    fp.write(
'''#!/usr/bin/python2.7
# -*- coding: utf-8 -*-

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Authors:**           Manuel Bastioni, Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2014

**Licensing:**         AGPL3 (http://www.makehuman.org/doc/node/the_makehuman_application.html)

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

The official MakeHuman rig
"""

from .flags import *

''')

    bpy.ops.object.mode_set(mode='OBJECT')
    hjoints,tjoints,ignores,connects = writeJoints(fp, rig, ob, scn)
    writeHeadsTails(fp, rig, hjoints, tjoints, connects)
    hier = makeHierarchy(rig)
    writeArmature(fp, hier, rig, [])
    bpy.ops.object.mode_set(mode='POSE')

    fp.write(
'''
Planes = {}
Constraints = {}
LocationLimits = {}
RotationLimits = {}
CustomShapes = {}
''')

    fp.close()
    print('Rig saved to "%s"' % filepath)

    if ignores:
        print("Check the following bones manually")
        for bname in ignores:
            print("  %s" % bname)


def makeHierarchy(rig):
    roots = []
    for b in rig.data.bones:
        if b.parent is None:
            roots.append(b)

    hier = []
    for root in roots:
       hier.append((root, makeChildren(root)))
    return (None, hier)


def makeChildren(parent):
    hier = []
    for bone in parent.children:
        hier.append((bone, makeChildren(bone)))
    return hier


ManualHeads = {
    'root' : ('o', ('pelvis', (0,0,-1))),
    'special02' : ('vl', ((0.5, 5577), (0.5,12174))),
}

ManualTails = {
    'foot.L' : ('vl', ((0.5, 16839), (0.5,16851))),
    'foot.R' : ('vl', ((0.5, 15535), (0.5,15547))),
    'special01' : ('vl', ((0.5, 317), (0.5,343))),
    #'special04' : ('vl', ((0.5, 5213), (0.5,11823))),
    #'oris05' : ('vl', ((0.75, 466), (0.25,467))),
}

def writeJoints(fp, rig, ob, scn):
    fp.write("\nJoints = [\n")

    #verts = []
    #for v in ob.data.vertices:
    #    if findJoint(v, rig):
    #        verts.append(v)

    scn.objects.active = ob
    joints = defineJoints(ob)

    for j in joints:
        fp.write("  ('%s', 'j', '%s'),\n" % (j,j))
    fp.write('\n')

    scn.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    ignores = []
    dolast = []
    connects = {}
    hjoints = {}
    tjoints = {}
    for eb in rig.data.edit_bones:
        bname = getBoneName(eb)
        if (eb.parent and
            (eb.use_connect or distance(eb.head, eb.parent.tail) < 1e-3)):
            pname = getBoneName(eb.parent)
            hjoints[bname] = getTailLoc(pname, tjoints)
            connects[bname] = pname
        else:
            jname1,jname2 = findJoints(eb.name, eb.head, joints)
            if jname2 is None:
                hjoints[bname] = jname1
            else:
                hv = findVertex(eb.head, ob.data.vertices)
                if hv:
                    hjoints[bname] = ('%s_hd' % bname)
                    fp.write("  ('%s_hd', 'v', %d),\n" % (bname, hv.index))
                elif bname in ManualHeads.keys():
                    hjoints[bname] = ('%s_hd' % bname)
                    type,value = ManualHeads[bname]
                    if isinstance(value,str):
                        value = ("'%s'" % value)
                    fp.write("  ('%s_hd', '%s', %s),\n" % (bname, type, value))
                else:
                    print("Head:", bname)
                    ignores.append(bname)

        jname1,jname2 = findJoints(eb.name, eb.tail, joints)
        if jname2 is None:
            tjoints[bname] = jname1
        else:
            tv = findVertex(eb.tail, ob.data.vertices)
            if tv:
                fp.write("  ('%s_tl', 'v', %d),\n" % (bname, tv.index))
                tjoints[bname] = ('%s_tl' % bname)
            elif bname in ManualTails.keys():
                tjoints[bname] = ('%s_tl' % bname)
                type,value = ManualTails[bname]
                if isinstance(value,str):
                    value = ("'%s'" % value)
                fp.write("  ('%s_tl', '%s', %s),\n" % (bname, type, value))
                tjoints[bname] = ('%s_tl' % bname)
            elif len(eb.children) == 1:
                child = eb.children[0]
                dolast.append((bname, child.name, eb.head, eb.tail, child.tail))
            else:
                print("Tail:", bname)
                ignores.append(bname)

    print("On line:")
    fp.write("\n")
    for bname,cname,bhead,btail,ctail in dolast:
        hj = hjoints[bname]
        try:
            tj = tjoints[cname]
        except KeyError:
            tj = None
            print("No tail:", bname, cname)
        vec1 = btail - bhead
        vec2 = ctail - bhead
        frac = vec1.length / vec2.length
        offset = (vec1 - frac*vec2) / vec2.length

        if bname == 'clavicle.L':
            print(bname)
            print(bhead)
            print(btail)
            print(cname)
            print(ctail)
            print(vec1)
            print(vec2)
            print(frac)
            print(offset)

        if offset.length > 1e-3:
            #print("Unaligned:", bname, cname,vec1, vec2, norm)
            x,y,z = offset
            fp.write(
            "  ('%s_tl0', 'l', ((%.4f, '%s'), (%.4f, '%s'))),\n" %
                (bname, 1-frac, hj, frac, tj) +
            "  ('%s_tl', 'so', ('%s_tl0', '%s', '%s', (%.4f, %.4f, %.4f))),\n" %
                (bname, bname, hj, tj, x, z, -y)
            )
        else:
            print("H %s, T %s, %f" % (bname, cname, frac))
            fp.write(
            "  ('%s_tl', 'l', ((%.4f, '%s'), (%.4f, '%s'))),\n" %
                (bname, 1-frac, hjoints[bname], frac, tjoints[cname])
            )
        tjoints[bname] = ('%s_tl' % bname)

    fp.write("\n]\n")
    return hjoints, tjoints, ignores, connects


def getHeadLoc(bname, hjoints, connects):
    try:
        return hjoints[bname]
    except KeyError:
        pass
    try:
        return ('%s_tl' % connects[bname])
    except KeyError:
        return ('%s_hd' % bname)


def getTailLoc(bname, tjoints):
    try:
        return tjoints[bname]
    except KeyError:
        return ('%s_tl' % bname)


def findJoints(bname, co, joints, minDist=0.05):
    dists = []
    for jname,jco in joints.items():
        dists.append((distance(co, jco), jname))
    dists.sort()
    if abs(dists[0][0]) < minDist:
        return (dists[0][1], None)
    else:
        return (dists[0][1], dists[1][1])


def distance(co1, co2):
    vec = co1-co2
    return vec.length


def findVertex(co, verts):
    for v in verts:
        if distance(v.co, co) < 1e-4:
            return v

    return None


def getBoneName(bone):
    bname = bone.name[0].lower() + bone.name[1:]
    return bname


def writeHeadsTails(fp, rig, hjoints, tjoints, connects):
    fp.write("\nHeadsTails = {\n")
    for bone in rig.data.bones:
        bname = getBoneName(bone)
        hloc = getHeadLoc(bname, hjoints, connects)
        tloc = getTailLoc(bname, tjoints)
        fp.write("  '%s' : ('%s', '%s'),\n" % (bname, hloc, tloc))
    fp.write("\n}\n")


def writeArmature(fp, hier, rig, ignores):
    fp.write("\nArmature = {\n")
    writeHierarchy(fp, hier, None, rig, ignores, False)
    fp.write("\n}\n")


def writeHierarchy(fp, hier, parent, rig, ignores, inface):
    bone,children = hier
    if parent:
        pstring = "'%s'" % getBoneName(parent)
    else:
        pstring = "None"
        fp.write("\n")
    if bone:
        bname = getBoneName(bone)
        if bone.use_connect:
            conn = "|F_CON"
        else:
            conn = ""
        if rig.MhxExportZeroRoll:
            roll = 0
        else:
            roll = rig.data.edit_bones[bone.name].roll
        if bname not in ignores:
            if bname in ["head", "neck", "neck2", "eye.L", "eye.R"]:
                layer = "L_HEAD"
                inface = True
            elif inface:
                layer = "L_FACE"
            else:
                layer = "L_MAIN"
            fp.write("  '%s' : (%.3f, %s, F_DEF%s, %s),\n" % (bname, roll, pstring, conn, layer))
    for child in children:
        writeHierarchy(fp, child, bone, rig, ignores, inface)


def defineJoints(ob):
    jointNames = [
    "l-eye",
    "r-eye",
    "pelvis",
    "spine-4",
    "spine-3",
    "spine-2",
    "spine-1",
    "l-foot-2",
    "l-foot-1",
    "l-toe-5-3",
    "l-toe-5-4",
    "l-toe-4-3",
    "l-toe-4-4",
    "l-toe-3-3",
    "l-toe-3-4",
    "l-toe-2-4",
    "l-toe-2-3",
    "l-toe-2-2",
    "l-toe-3-2",
    "l-toe-4-2",
    "l-toe-5-2",
    "l-toe-5-1",
    "l-toe-4-1",
    "l-toe-3-1",
    "l-toe-2-1",
    "l-toe-1-3",
    "l-toe-1-2",
    "l-toe-1-1",
    "l-ankle",
    "l-knee",
    "l-upper-leg",
    "l-finger-2-4",
    "l-finger-3-4",
    "l-finger-4-4",
    "l-finger-5-4",
    "l-finger-5-3",
    "l-finger-4-3",
    "l-finger-3-3",
    "l-finger-2-3",
    "l-finger-2-2",
    "l-finger-3-2",
    "l-finger-4-2",
    "l-finger-5-2",
    "l-finger-5-1",
    "l-finger-4-1",
    "l-finger-3-1",
    "l-finger-2-1",
    "l-finger-1-4",
    "l-finger-1-3",
    "l-finger-1-2",
    "l-finger-1-1",
    "l-hand-3",
    "l-hand-2",
    "l-hand",
    "l-elbow",
    "l-shoulder",
    "l-clavicle",
    "l-scapula",
    "head",
    "l-lowerlid",
    "l-eye-target",
    "l-upperlid",
    "r-foot-2",
    "r-foot-1",
    "r-toe-5-3",
    "r-toe-5-4",
    "r-toe-4-3",
    "r-toe-4-4",
    "r-toe-3-3",
    "r-toe-3-4",
    "r-toe-2-4",
    "r-toe-2-3",
    "r-toe-2-2",
    "r-toe-3-2",
    "r-toe-4-2",
    "r-toe-5-2",
    "r-toe-5-1",
    "r-toe-4-1",
    "r-toe-3-1",
    "r-toe-2-1",
    "r-toe-1-3",
    "r-toe-1-2",
    "r-toe-1-1",
    "r-ankle",
    "r-knee",
    "r-upper-leg",
    "r-finger-2-4",
    "r-finger-3-4",
    "r-finger-4-4",
    "r-finger-5-4",
    "r-finger-5-3",
    "r-finger-4-3",
    "r-finger-3-3",
    "r-finger-2-3",
    "r-finger-2-2",
    "r-finger-3-2",
    "r-finger-4-2",
    "r-finger-5-2",
    "r-finger-5-1",
    "r-finger-4-1",
    "r-finger-3-1",
    "r-finger-2-1",
    "r-finger-1-4",
    "r-finger-1-3",
    "r-finger-1-2",
    "r-finger-1-1",
    "r-hand-3",
    "r-hand-2",
    "r-hand",
    "r-elbow",
    "r-shoulder",
    "r-clavicle",
    "r-scapula",
    "r-lowerlid",
    "r-eye-target",
    "r-upperlid",
    "neck",
    "jaw",
    "tongue-4",
    "tongue-3",
    "head-2",
    "tongue-2",
    "tongue-1",
    "mouth",
    ]

    joints = {}
    vn0 = 13606
    for name in jointNames:
        vsum = Vector((0,0,0))
        for n in range(8):
            vsum += ob.data.vertices[vn0+n].co
        joints[name] = vsum/8
        print(name, vn0, vn0+7, joints[name])
        vn0 += 8
    return joints


```

## Source file: weighting/gen_faceshapes.py

{
  "path": "weighting/gen_faceshapes.py",
  "blob_sha": "0305b97415e68a62a0a497b79358bcdf65d7acf4",
  "sha256": "1a46ed19eb1f0dba67d5f4f694248ca9b94cbca14cea41d574b27a3c030153bb",
  "lines": 152,
  "bytes": 4560,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 9,
    "tongue": 4,
    "wrinkle": 1,
    "rig": 7,
    "face": 2,
    "lip": 9
  },
  "url": "https://github.com/makehumancommunity/makehuman-utils/blob/94686ff7c743227a46ab6b35daa47c52d964437a/weighting/gen_faceshapes.py",
  "function_signature_candidates": [
    {
      "line": 93,
      "text": "def generateLRFiles(folder):"
    },
    {
      "line": 125,
      "text": "def readCoords(filepath, vgroup=None):"
    },
    {
      "line": 150,
      "text": "def execute(self, context):"
    }
  ]
}

```text

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman-utils

**Authors:**           Thomas Larsson

**Copyright(c):**      MakeHuman Team 2001-2014

**Licensing:**         AGPL3 (http://www.makehuman.org/doc/node/the_makehuman_application.html)

    This file is part of MakeHuman (www.makehuman.org).

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.

**Coding Standards:**  See http://www.makehuman.org/node/165

Abstract
--------

Project to proxy

"""

import bpy
import os
from collections import OrderedDict
from . import io_json
from maketarget.utils import round


SplitInfo = {
    "brow_mid_up"           : "LR",
    "brow_mid_down"         : "LR",
    "brow_outer_up"         : "LR",
    "brow_outer_down"       : "LR",
    "brow_squeeze"          : "Sym",

    "cheek_narrow"          : "LR",
    "cheek_balloon"         : "LR",
    "cheek_up"              : "LR",
    "cheek_squint"          : "LR",

    "lips_mid_upper_up"     : "LR",
    "lips_mid_lower_up"     : "LR",
    "lips_mid_upper_down"   : "LR",
    "lips_mid_lower_down"   : "LR",
    "lips_upper_in"         : "Sym",
    "lips_lower_in"         : "Sym",
    "lips_upper_out"        : "Sym",
    "lips_lower_out"        : "Sym",
    "lips_part"             : "Sym",

    "mouth_corner_in"       : "LR",
    "mouth_corner_out"      : "LR",
    "mouth_corner_up"       : "LR",
    "mouth_corner_down"     : "LR",
    "mouth_up"              : "LR",
    "mouth_down"            : "LR",
    "mouth_narrow"          : "LR",
    "mouth_open"            : "Sym",
    "mouth_wide"            : "LR",

    "nose_wrinkle"          : "Sym",

    "tongue_back_up"        : "Sym",
    "tongue_out"            : "Sym",
    "tongue_up"             : "Sym",
    "tongue_wide"           : "Sym",
}

Scales = {
    "base" : {
        "x" : (5399, 11998, 1.4800),
        "y" : (791, 881, 2.3298),
        "z" : (962, 5320, 1.9221),
    },
}

def generateLRFiles(folder):
    lrFile = os.path.join(os.path.dirname(__file__), "data/vgrp_leftright.json")
    lrVgroups = io_json.loadJson(lrFile)
    left = {}
    right = {}
    for n in range(19000):
        left[n] = 0.0
        right[n] = 0.0
    for n,w in lrVgroups["Left"]:
        left[n] = w
    for n,w in lrVgroups["Right"]:
        right[n] = w

    raw = os.path.join(folder, "plugins/9_export_xmhx/data/faceshapes/raw/")
    struct = OrderedDict()
    struct["scale"] = Scales["base"]
    targets = struct["targets"] = OrderedDict()

    for file in os.listdir(raw):
        fname,ext = os.path.splitext(file)
        if ext == ".target":
            filepath = os.path.join(raw, file)
            if SplitInfo[fname] == "Sym":
                targets[fname] = readCoords(filepath)
            else:
                targets[fname+"_left"] = readCoords(filepath, left)
                targets[fname+"_right"] = readCoords(filepath, right)

    filepath = os.path.join(folder, "plugins/9_export_xmhx/data/faceshapes/faceshapes.json")
    io_json.saveJson(struct, filepath, maxDepth=0)


def readCoords(filepath, vgroup=None):
    coord = []
    fp = open(filepath, "rU")
    eps = 1e-5
    for line in fp:
        words = line.split()
        if len(words) == 0 or words[0][0] == "#":
            pass
        else:
            n,x,y,z = int(words[0]), float(words[1]), float(words[2]), float(words[3])
            if vgroup:
                k = vgroup[n]
            else:
                k = 1.0
            x,y,z = k*x,k*y,k*z
            if not (abs(x) < eps and abs(y) < eps and abs(z) < eps):
                coord.append((n, (x,y,z)))
    fp.close()
    return coord


class VIEW3D_OT_GenereateLRFilesButton(bpy.types.Operator):
    bl_idname = "mhw.generate_lr_files"
    bl_label = "Generate LR Files"

    def execute(self, context):
        generateLRFiles("/home/hg/mhx_system/")
        return{'FINISHED'}


```

## Source file: io_mhrigging_mhskel/export_mh_rigging.py

{
  "path": "io_mhrigging_mhskel/export_mh_rigging.py",
  "blob_sha": "5928923ba76e2058b7b5f4a6f8d124300fd9e737",
  "sha256": "412c0a62eba8576981b3903f4774a34c6379f52ff48b2c77f4bd9eda974cf309",
  "lines": 477,
  "bytes": 16763,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 15,
    "face": 1
  },
  "url": "https://github.com/makehumancommunity/makehuman-utils/blob/94686ff7c743227a46ab6b35daa47c52d964437a/io_mhrigging_mhskel/export_mh_rigging.py",
  "function_signature_candidates": [
    {
      "line": 49,
      "text": "def boneToVertex(basemesh, point):"
    },
    {
      "line": 74,
      "text": "def getVertsFromGroup(basemesh, groupName):"
    },
    {
      "line": 104,
      "text": "def getWeightsData(basemesh, armature):"
    },
    {
      "line": 135,
      "text": "def getBonesData(basemesh, armature):"
    },
    {
      "line": 186,
      "text": "def getJointsData(basemesh, armature):"
    },
    {
      "line": 364,
      "text": "def writeRiggingFile(context, filepath):"
    },
    {
      "line": 446,
      "text": "def menu_func_export(self, context):"
    },
    {
      "line": 462,
      "text": "def execute(self, context):"
    },
    {
      "line": 465,
      "text": "def register():"
    },
    {
      "line": 469,
      "text": "def unregister():"
    }
  ]
}

```text

"""
**Project Name:**      MakeHuman

**Product Home Page:** http://www.makehuman.org/

**Code Home Page:**    https://bitbucket.org/MakeHuman/makehuman/

**Author:**            Manuel Bastioni

**Copyright(c):**      Manuel Bastioni 2014

**Licensing:**         AGPL3

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.


Abstract
--------

Export a mhskel file with a rigging designed upon the makehuman base mesh.

To use this script, you can place it in the .blender/scripts/addons dir
and then activate the script in the "Addons" tab (user preferences).
Access from the File > Import menu.

"""

import bpy
import json
from bpy_extras.io_utils import ExportHelper
from bpy.types import Operator, Panel
from bpy.props import StringProperty, BoolProperty, EnumProperty
from . import shared_mh_rigging
from . shared_mh_rigging import *
import os


def boneToVertex(basemesh, point):
    """
    This function gets the coordinates of a bone head, or bone tail,
    and look for the closer vertex in tha basemesh.

    Parameters
    ----------

    basemesh: The makehuman base mesh
    point: The coordinates of the point in as float list [x,y,z]

    Return
    ----------
    The function return a list with only one element, the point, that's list of three float.
    """
    vertPlacings = []
    for vert in basemesh.data.vertices:
        vertPlacings.append([vdist(point,vert.co),vert.index])
    closerVert = min(vertPlacings)

    if closerVert[0] > DELTAMIN:
        return None
    else:
        return [closerVert[1]]

def getVertsFromGroup(basemesh, groupName):
    """
    In Blender the vertgroups are stored in an odd way.
    So to retrieve the vertices that are in a group of vertices, it's
    needed to do a series of loops.
    This function returns all the vertices (and their weights) for the specified
    group.
    
    Parameters
    ----------

    basemesh: The makehuman base mesh.
    groupname: the name of group were extract the vertices.

    Return
    ----------
    A list of lists [[vert_index, vert_weight],..., [vert_index, vert_weight]]
    """
    vertsInGroup = []
    if groupName in basemesh.vertex_groups:
        objGroup = basemesh.vertex_groups[groupName]
        for vert in basemesh.data.vertices:
            for vertGroup in vert.groups:
                if vertGroup.group == objGroup.index:
                    weight = round(vertGroup.weight,4)
                    if weight > 0:
                        vertsInGroup.append([vert.index, weight])

    return vertsInGroup

def getWeightsData(basemesh, armature):
    """
    This function extracts the weight information.
    In Blender each bone is linked to a group with the same name.
    So for each bone we get the related group and extract the weight
    of each vert of this group.
    
    Parameters
    ----------

    basemesh: The makehuman base mesh.
    armature: The armature modelled for the MakeHuman mesh
    
    Return
    ----------
    A dictionary with weight data. The key is the bone name and the value is a
    list oc couples [vert_index, vert_weight]
    """

    objectMode = bpy.context.object.mode
    bpy.ops.object.mode_set(mode='EDIT')
    groupData = {}

    for bone in armature.data.edit_bones:
        groupName = bone.name #Because groups have the same name of bones
        verts = getVertsFromGroup(basemesh, groupName)
        groupData[bone.name] = verts

    bpy.ops.object.mode_set(mode=objectMode)
    return groupData

def getBonesData(basemesh, armature):
    """
    This function extract the main informations of each bone,
    (bone name, head name, tail name, rotation plane, parent name)
    and return them as dictionary, ready to be written
    in mhskel format. It also creates an empty dictionary of
    rotation planes, to be filled by hand, atm.
    
    Parameters
    ----------

    basemesh: The makehuman base mesh.
    armature: The armature modelled for the MakeHuman mesh

    Return
    ----------
    A dictionary with bones data. The key is the bone name, and the
    value is a sub dictionary with the bone info.
    """

    # Store the status of the object mode that will be restored at
    # the end of the function, then enter in edit mode.
    # We need to stay in edit mode in order to access to the rest position.
    # Note: bone.roll property is available only in the edit_bones object.

    objectMode = bpy.context.object.mode
    bpy.ops.object.mode_set(mode='EDIT')

    # Create and populate the dictionary.

    bones = {}
    rot_planes = {}
    for bone in armature.data.edit_bones:
        boneData = {}
        boneData["head"] = '{0}____head'.format(bone.name)
        boneData["tail"] = '{0}____tail'.format(bone.name)
        boneData["rotation_plane"] = '{0}____plane'.format(bone.name)
        #boneData["roll"] = bone.roll
        boneData["reference"] = None
        if bone.parent:
            boneData["parent"] = '{0}'.format(bone.parent.name)
        else:
            boneData["parent"] = None
        bones[bone.name] = boneData
        rot_planes['{0}____plane'.format(bone.name)] = [None, None, None] #This data will be filled by hand in the exported file.

    # Restore the initial mode
    bpy.ops.object.mode_set(mode=objectMode)
    return bones, rot_planes


def getJointsData(basemesh, armature):
    """
    An helper joint is a little cube included in the base mesh,
    that is always morphed accordingly the base mesh.
    Helper joints are used to recalculate the skeleton after the morphing.
    Each helper joint is represented by a list of eight vert indices,
    stored in JOINTS_VERT_INDICES.

    This function, for each bone, extract the coordinates of his head
    and then compare it with all the centroids of the helper joints.
    If the head is very close to a centroid, it's mapped to the centroid's
    vertices.

    When all the centroids are too far from the head, the function try to
    map it to a single vertices of the mesh, looking for it in the whole
    vertices list (not only the centroids)

    If this fails too, but the head is positioned between his bone child
    and his parent head, already mapped to an helper or a vert,
    the function try to compare the head coords with the average point
    between them.

    If this fail too, the head mapping must be done by hand, writing his
    vertices in the global MISSEDJOINT dictionary.

    The same happens for the tail of the bone.
    
    Parameters
    ----------

    basemesh: The makehuman base mesh.
    armature: The armature modelled for the MakeHuman mesh

    Return
    ----------

    The function getJointsData get the bones head and tails, and return
    a mapping with the correspondent helper joints.
    """

    global JOINTS_VERT_INDICES
    jointCentroids = []
    joints = {}

    # Store the status of the object mode that will be restored at
    # the end of the function, then enter in edit mode.
    # We need to stay in edit mode in order to access to the rest position.
    # ALso armature.data.edit_bones is empty if the armature is not in edit mode!

    objectMode = bpy.context.object.mode
    bpy.ops.object.mode_set(mode='EDIT')

    # Retrieving the vert coordinates from the vert indices stored in
    # JOINTS_VERT_INDICES, calculate their centroid, and put them in
    # jointCentroids

    for jointIndices in JOINTS_VERT_INDICES:
        vertices = []
        for index in jointIndices:
            try:
                vertices.append(basemesh.data.vertices[index])
            except:
                print("Vert n. {0} not found for centroid calculation. Probably you are exporting a mesh that's not the base human".format(index))
        jointCentroids.append(centroid(vertices))

    for bone in armature.data.edit_bones:
        headPlacings = []
        tailPlacings = []

        # headPlacings contains all the distances between the bone head
        # and the helpers centroid. It also contains, with each distance,
        # the list of the correspondent indices of the helper verts.
        # Using the function "min", it will return the closer helper,
        # and his vert indices. The same for tailPlacing.

        for i in range(len(JOINTS_VERT_INDICES)):
                headPlacings.append([vdist(bone.head,jointCentroids[i]),JOINTS_VERT_INDICES[i]])
                tailPlacings.append([vdist(bone.tail,jointCentroids[i]),JOINTS_VERT_INDICES[i]])

        # If the distance between the bone head and the closer helper
        # is less of DELTAMIN, the helper is valid. Otherwise the head
        # has not a correspodent helper. In the second case, we look for
        # a closer vertices that can be used as helper.
        # The same for tail.

        closerHead = min(headPlacings)
        closerTail = min(tailPlacings)

        # Looking for the closer helper centroid. If fails, look for the closer vert.

        if closerHead[0] > DELTAMIN:
            joints['{0}____head'.format(bone.name)] = boneToVertex(basemesh, bone.head)
        else:
            joints['{0}____head'.format(bone.name)] = closerHead[1]

        if closerTail[0] > DELTAMIN:
            joints['{0}____tail'.format(bone.name)] = boneToVertex(basemesh, bone.tail)
        else:
            joints['{0}____tail'.format(bone.name)] = closerTail[1]

    # If after the loops above, the status is still None, the function
    # look for the interpolation of the connected head and tails.
    for k, v in joints.items():
        if v == None:
            #print ("DEBUG " + k)
            # Retrieve the info about missed joint from the key

            boneName = k.split("____")[0]
            jointType = k.split("____")[1]
            
            bone = bpy.context.object.data.edit_bones[boneName]

            # Retrieve the info of the connected bones and their centroids
            # If we have a missed head, it calculate the average coordinates
            # of the bone tail and his parent head, in order to verify if they 
            # can be used to approssimate the joint position:
            #
            #     |\          |\
            #  h1 | | t1   h2 | | t2   
            #     |/          |/
            #
            #  Perhaps h2 = (h1+t2)/2
            #
            #  Why we don't get simply the t1 (parent's tail)?
            #  Because often t1 = h2 = missed helpers
            #
            #  We do a similar thing in case of a missed tail.
            
            centroid1Key = None
            centroid2Key = None
            
            if jointType == "head":
                jointToRecalculate = bone.head
                if bone.parent:
                    centroid1Key = '{0}____tail'.format(bone.name)
                    centroid2Key = '{0}____head'.format(bone.parent.name)

            elif jointType == "tail":
                jointToRecalculate = bone.tail
                if len(bone.children) != 0:
                    centroid1Key = '{0}____head'.format(bone.name)
                    centroid2Key = '{0}____tail'.format(bone.children[0].name)

            # Check if the connected joints are already mapped and then
            # interpolates them
            
            if centroid1Key in joints and centroid2Key in joints:
                if joints[centroid1Key] != None and joints[centroid2Key] != None:
                    centroid1= joints[centroid1Key]
                    centroid2= joints[centroid2Key]
                    vertices = []
                    for index in centroid1:
                        vertices.append(basemesh.data.vertices[index])
                    for index in centroid2:
                        vertices.append(basemesh.data.vertices[index])

                    # If the interpolated centroid is close enough to the joint,
                    # it's mapped to the vertices of both the connected elements.

                    if vdist(centroid(vertices),jointToRecalculate) < DELTAMIN :
                        joints[k] = centroid1 + centroid2

        # If, after the checks above, the value is still missed, print
        # a warning and try to get it from the dictionary written by hand.

        if joints[k] == None:
            if k in MISSEDJOINTS:
                joints[k] =  MISSEDJOINTS[k]
            else:
                print("{0}: MISSED".format(k))

    # Restore the initial mode
    bpy.ops.object.mode_set(mode=objectMode)
    print("Examined {0} joints".format(len(joints)))

    return joints


def writeRiggingFile(context, filepath):

    """
    This function write the data in mhskel format.
    
    Parameters
    ----------

    context: Blender context.
    filepath: The path of file to save.
    
    """
    basemesh = getObject() 
    
    if basemesh == None: 
        print("No mesh selected")
        bpy.ops.box1.message('INVOKE_DEFAULT')
        return {'FINISHED'}

  
    armature = basemesh.parent  
    if armature == None:
        print("The selected mesh is not parented with an armature")
        bpy.ops.box1.message('INVOKE_DEFAULT')
        return {'FINISHED'}
    
    # This is very important, because the armature
    # must be active in order to turn in edit mode.
    
    bpy.context.scene.objects.active = armature

    bones, rot_planes = getBonesData(basemesh, armature)
    joints = getJointsData(basemesh, armature)
    weights = getWeightsData(basemesh, armature)    

    weightsFilePath = os.path.splitext(filepath)[0]+"_weights.mhw"
    weightsFile = os.path.basename(weightsFilePath)

    dataArmature = {}
    dataArmature["name"] = armature.name
    dataArmature["version"] = VERSION #102 means 1.0.2
    dataArmature["copyright"] = "(c) Makehuman.org 2015"
    dataArmature["license"] = "GNU Affero General Public License 3"
    dataArmature["description"] = "%s skeleton" % armature.name
    dataArmature["joints"] =  joints
    dataArmature["bones"] =  bones
    dataArmature["planes"] =  rot_planes
    dataArmature["weights_file"] = weightsFile
    
    dataWeights = {}
    dataWeights["name"] = "MakeHuman weights"
    dataWeights["version"] = VERSION #102 means 1.0.2
    dataWeights["copyright"] = "(c) Makehuman.org 2014"
    dataWeights["description"] = "Very cool general-purpose skeleton"
    dataWeights["license"] = "GNU Affero General Public License 3"
    dataWeights["weights"] =  weights
    
    
    outfile = open(filepath, 'w')
    json.dump(dataArmature, outfile, sort_keys=True, indent=4, separators=(',', ': '))
    outfile.close()
        
    outfile = open(weightsFilePath, 'w')
    json.dump(dataWeights, outfile, sort_keys=True, indent=4, separators=(',', ': '))
    outfile.close()

    #Restore the initial active object
    bpy.context.scene.objects.active = basemesh
    return {'FINISHED'}

"""
BLENDER GUI
The following functions and classes are used to create the interface in Blender.
The script will create a new category tab in the Blender tools column
"""

# ImportHelper is a helper class, defines filename and
# invoke() function which calls the file selector.
from bpy_extras.io_utils import ImportHelper
from bpy.props import StringProperty, BoolProperty, EnumProperty
from bpy.types import Operator

def menu_func_export(self, context):
    self.layout.operator(ExportMHRigging.bl_idname, text="MakeHuman rigging (.mhskel)")    

class ExportMHRigging(Operator, ExportHelper):
    """This appears in the tooltip of the operator and in the generated docs"""
    bl_idname = "export_mh_rigging.data"  # important since its how bpy.ops.import_rigging.data is constructed
    bl_label = "Export MakeHuman Rigging"

    # ImportHelper mixin class uses this
    filename_ext = ".mhskel"

    filter_glob = StringProperty(
            default="*.mhskel",
            options={'HIDDEN'},
            )

    def execute(self, context):        
        return writeRiggingFile(context, self.filepath)       

def register():
    bpy.utils.register_class(ExportMHRigging)
    bpy.types.INFO_MT_file_export.append(menu_func_export)

def unregister():
    bpy.utils.unregister_class(ExportMHRigging)
    bpy.types.INFO_MT_file_export.remove(menu_func_export)


if __name__ == "__main__":
    shared_mh_rigging.register()
    register()



```

## Source file: io_mhrigging_mhskel/import_mh_rigging.py

{
  "path": "io_mhrigging_mhskel/import_mh_rigging.py",
  "blob_sha": "1ef6521ff8950a35240d1c58926fc0d7d01bf981",
  "sha256": "cb101db5b1015e44c948add2808d1601a3fde7aa82cb7c5b98d945fe67db70f5",
  "lines": 154,
  "bytes": 5032,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 15,
    "face": 1
  },
  "url": "https://github.com/makehumancommunity/makehuman-utils/blob/94686ff7c743227a46ab6b35daa47c52d964437a/io_mhrigging_mhskel/import_mh_rigging.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def createArmatureFromJsonFile(filePath):"
    },
    {
      "line": 102,
      "text": "def readRiggingFile(context, filepath):"
    },
    {
      "line": 122,
      "text": "def menu_func_import(self, context):"
    },
    {
      "line": 138,
      "text": "def execute(self, context):"
    },
    {
      "line": 142,
      "text": "def register():"
    },
    {
      "line": 146,
      "text": "def unregister():"
    }
  ]
}

```text

import bpy
import json
import os
from . import shared_mh_rigging
from . shared_mh_rigging import *

def createArmatureFromJsonFile(filePath):

    with open(filePath) as dataFile:
        armatureData = json.load(dataFile)

    basemesh = getObject()       
    joints = armatureData['joints']
    name = armatureData['name']
    jointCoordinates = {}
    bones = armatureData['bones']
    planes = armatureData['planes']
    # TODO fix if no planes defined
    #weights = armatureData['weights']

    if "weights_file" in armatureData:
        weightsFileName = armatureData["weights_file"]
        weightsFilePath = os.path.join(os.path.dirname(filePath),weightsFileName)
        weightsFile = open(weightsFilePath)
        weightsData = json.load(weightsFile)
        weights = weightsData['weights']
        weightsFile.close()

        for group, weights in weights.items():
            newGroup = basemesh.vertex_groups.new(group)
            for weightData in weights:
                newGroup.add([weightData[0]], weightData[1], 'ADD')

    for joint, vertices in joints.items():        
        jointCoordinates[joint] = vertsindexToCentroid(vertices)

    bpy.ops.object.add(
        type='ARMATURE',
        enter_editmode=True,
        location=(0,0,0))

    newArmature = bpy.context.object
    newArmature.name = name
    newArmature.show_name = True
    amt = newArmature.data
    amt.name = name+'Amt'

    # TODO store information in armature so that we can re-correct rolls after it was modified
    for boneKey, boneData in bones.items():
        # Create single bone
        newBone = amt.edit_bones.new('Bone')
        headKey = boneData['head']
        tailKey = boneData['tail']
        rollPlane = boneData.get("rotation_plane", None)

        headCoords = jointCoordinates[headKey]
        tailCoords = jointCoordinates[tailKey]

        newBone.name = boneKey
        newBone.head = headCoords
        newBone.tail = tailCoords
        #newBone.roll = ...
        # Set the roll using a reference plane
        try:
            plane = planes[rollPlane]
            plane_coords = get_plane_coords(plane, jointCoordinates)
            normal = get_normal(plane_coords)
            z_axis = normal.cross(newBone.y_axis)
            newBone.align_roll(z_axis)
        except:
            # TODO for other rigs than the default we currently cannot set the rolls correctly
            pass

    for boneName, boneData in bones.items():
        parentName = boneData.get('parent', None)
        if parentName != None:
            amt.edit_bones[boneName].parent = amt.edit_bones[parentName]

    bpy.ops.object.mode_set(mode='OBJECT')

    # Copy mesh rotation to skeleton
    # (Usually rotates rig to Z-up axis system, depending on OBJ importer config)
    basemesh.rotation_mode = 'XYZ'
    newArmature.rotation_mode = 'XYZ'
    newArmature.rotation_euler = basemesh.rotation_euler
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)

    # Give basemesh object an armature modifier, using vertex groups but
    # not envelopes
    mod = basemesh.modifiers.new('MyRigModif', 'ARMATURE')
    mod.object = newArmature
    mod.use_bone_envelopes = False
    mod.use_vertex_groups = True
    basemesh.parent = newArmature

    print ("Rig up axis: %s" % guess_up_axis(newArmature.data))
    bpy.ops.object.mode_set(mode='OBJECT')

    return newArmature


def readRiggingFile(context, filepath):
    if getObject() != None:
        createArmatureFromJsonFile(filepath)
    else:
        bpy.ops.box1.message('INVOKE_DEFAULT')
    return {'FINISHED'}


"""
BLENDER GUI
The following functions and classes are used to create the interface in Blender.
The script will create a new category tab in the Blender tools column
"""

# ImportHelper is a helper class, defines filename and
# invoke() function which calls the file selector.
from bpy_extras.io_utils import ImportHelper
from bpy.props import StringProperty, BoolProperty, EnumProperty
from bpy.types import Operator

def menu_func_import(self, context):    
    self.layout.operator(ImportMHRigging.bl_idname, text="MakeHuman rigging (.mhskel)")    

class ImportMHRigging(Operator, ImportHelper):
    """This appears in the tooltip of the operator and in the generated docs"""
    bl_idname = "import_mh_rigging.data"  # important since its how bpy.ops.import_rigging.data is constructed
    bl_label = "Import MakeHuman Rigging"

    # ImportHelper mixin class uses this
    filename_ext = ".mhskel"

    filter_glob = StringProperty(
            default="*.mhskel",
            options={'HIDDEN'},
            )

    def execute(self, context):
        return readRiggingFile(context, self.filepath)


def register():
    bpy.utils.register_class(ImportMHRigging)
    bpy.types.INFO_MT_file_import.append(menu_func_import)

def unregister():
    bpy.utils.unregister_class(ImportMHRigging)
    bpy.types.INFO_MT_file_import.remove(menu_func_import)


if __name__ == "__main__":
    shared_mh_rigging.register()
    register()



```

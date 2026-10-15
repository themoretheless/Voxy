# Facial rendering source scan: tHeBeStXu/FacialRigTool

Repository: https://github.com/tHeBeStXu/FacialRigTool

Pinned source revision: 9ea46068d8d77644c5f560fb68388de4389cfe4b

Discovery topic: facial rig

Description: Custom Facial Rigging Tool for Game(Unreal)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["FacialRigTool/rig/cartoonEyeLidRig.py"], "skin": ["FacialRigTool/skinLib/__init__.py", "FacialRigTool/skinLib/skinLib.py"], "rig": ["FacialRigTool/.gitattributes", "FacialRigTool/UI/Edit_UI.py", "FacialRigTool/UI/Main_UI.py", "FacialRigTool/UI/Rig_UI.py", "FacialRigTool/UI/Splitter_UI.py", "FacialRigTool/UI/__init__.py", "FacialRigTool/__init__.py", "FacialRigTool/base/__init__.py", "FacialRigTool/base/control.py", "FacialRigTool/base/controlShape/ArrowCurve.py", "FacialRigTool/base/controlShape/CrossControl.py", "FacialRigTool/base/controlShape/CrownCurve.py", "FacialRigTool/base/controlShape/CubeCurve.py", "FacialRigTool/base/controlShape/Diamond.py", "FacialRigTool/base/controlShape/FistCurve.py", "FacialRigTool/base/controlShape/FootControl.py", "FacialRigTool/base/controlShape/MoveControl.py", "FacialRigTool/base/controlShape/RotationControl.py", "FacialRigTool/base/controlShape/SliderControl.py", "FacialRigTool/base/controlShape/SpikeCrossControl.py", "FacialRigTool/base/controlShape/__init__.py", "FacialRigTool/base/controlShape/cubeOnBase.py", "FacialRigTool/base/controlShape/horizontalSliderControl.py", "FacialRigTool/base/controlShape/planeSliderControl.py", "FacialRigTool/base/controlShape/singleRotateControl.py", "FacialRigTool/base/controlShape/squareControl.py", "FacialRigTool/base/controlShape/unitSliderControl.py", "FacialRigTool/base/controlShape/verticalSliderControl.py", "FacialRigTool/base/module.py", "FacialRigTool/rig/__init__.py"]}

## Source file: FacialRigTool/rig/cartoonEyeLidRig.py

{
  "path": "FacialRigTool/rig/cartoonEyeLidRig.py",
  "blob_sha": "85a98d53076bb9f3e39249680209333820df1128",
  "sha256": "23082c9e08d17b082971f55c9421b9ee1eb11c9b7911bec9496a3376b268d446",
  "lines": 410,
  "bytes": 14735,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 54,
    "skin": 4,
    "deform": 2,
    "rig": 45
  },
  "url": "https://github.com/tHeBeStXu/FacialRigTool/blob/9ea46068d8d77644c5f560fb68388de4389cfe4b/FacialRigTool/rig/cartoonEyeLidRig.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def createRig(upperVertexList,"
    }
  ]
}

```text

import maya.cmds as cmds
from ..rigLib import lib
from ..utils import name
from ..base import control
from ..base import module

reload(lib)
reload(name)
reload(control)
reload(module)


def createRig(upperVertexList,
              lowerVertexList,
              prefix='L_',
              rigPartName='EyeLid',
              rigScale=1.0,
              eyeJoint='',
              numCtrl=5,
              ):

    if numCtrl < 3:
        cmds.error('numCtrl must bigger than 3!')
        return
    cmds.select(cl=1)
    # create eyeLid Module
    eyeLidRigModule = module.Module(prefix=prefix, rigPartName=rigPartName)

    # create upper eyelid Module
    upperLidRigModule = module.Module(prefix=prefix, rigPartName='upper_' + rigPartName)

    #####################
    # Upper Eyelid Part #
    #####################

    # create eyelid joint for each vertex
    upperEyeLidJointList = lib.vertex2Joints(vertexList=upperVertexList, prefix=prefix,
                                             rigPartName='upper_' + rigPartName, radius=0.05)

    # connect attr
    for joint in upperEyeLidJointList:
        if cmds.attributeQuery('slaveJoint', node=joint, exists=1):
            cmds.connectAttr(upperLidRigModule.topGrp + '.slaveJoint', joint + '.slaveJoint', f=1)

    # create eyelid parent joint for each eyelid joint
    upperEyeLidParentJntList = []
    for i in upperEyeLidJointList:
        cmds.select(cl=1)

        parentJoint = cmds.joint(n=i + '_Parent', radius=0.05)

        cmds.delete(cmds.pointConstraint(eyeJoint, parentJoint, mo=0))

        cmds.delete(cmds.aimConstraint(i, parentJoint, aimVector=(1, 0, 0), upVector=(0, -1, 0),
                                       worldUpType='scene', weight=1, offset=(0, 0, 0), mo=0))
        cmds.parent(i, parentJoint)

        cmds.joint(i, e=1, oj='none', ch=1, zso=1)

        cmds.makeIdentity(parentJoint, apply=1, t=1, r=1, s=1)

        upperEyeLidParentJntList.append(parentJoint)

    cmds.select(cl=1)

    upperEyelidLocList = []
    # create locator for each eyelid joint
    for i in upperEyeLidParentJntList:

        cmds.select(cl=1)

        eyelidJoint = cmds.listRelatives(i, c=1, type='joint', shapes=0)[0]

        ikHandle = cmds.ikHandle(n=eyelidJoint + '_IK', sj=i, ee=eyelidJoint, sol='ikSCsolver')

        eyelidLoc = cmds.spaceLocator(n=eyelidJoint + '_LOC')[0]

        cmds.delete(cmds.parentConstraint(eyelidJoint, eyelidLoc, mo=0))

        cmds.select(cl=1)

        cmds.setAttr(ikHandle[0] + '.v', 0)
        LOCShape = cmds.listRelatives(eyelidLoc, p=0, c=1, s=1)[0]
        cmds.setAttr(LOCShape + '.localScaleX', 0.1)
        cmds.setAttr(LOCShape + '.localScaleY', 0.1)
        cmds.setAttr(LOCShape + '.localScaleZ', 0.1)

        cmds.parent(ikHandle[0], eyelidLoc)

        upperEyelidLocList.append(eyelidLoc)

    cmds.select(cl=1)

    # create high definition curve
    lowerPosList = []
    for i in upperEyelidLocList:
        pos = cmds.xform(i, q=1, ws=1, t=1)
        lowerPosList.append(tuple(pos))

    upperKList = []
    for i in xrange(len(lowerPosList)):
        upperKList.append(i)

    upperHighDefCurve = cmds.curve(n=prefix + 'upper_' +rigPartName + '_HD_Crv', p=lowerPosList, k=upperKList, d=1)
    upperLowDefCurve = cmds.duplicate(upperHighDefCurve, n=prefix + 'lower_' + rigPartName + '_LD_Crv')

    upperHighDefCurveShape = cmds.listRelatives(upperHighDefCurve, p=0, c=0, s=1, path=1)[0]
    cmds.select(cl=1)

    # make each locator attach to the curve

    for i in upperEyelidLocList:
        pos = cmds.xform(i, q=1, ws=1, t=1)
        uParam = lib.getUParam(pos, upperHighDefCurveShape)

        PCI = cmds.createNode('pointOnCurveInfo', n=name.removeSuffix(i) + '_PCI')

        cmds.connectAttr(upperHighDefCurveShape + '.worldSpace', PCI + '.inputCurve', f=1)

        cmds.setAttr(PCI + '.parameter', uParam)

        cmds.connectAttr(PCI + '.position', i + '.t')

        cmds.select(cl=1)

    # make HD curve deformed by LD curve
    upperLowDefCurve = cmds.rebuildCurve(upperLowDefCurve, ch=0, rpo=1, rt=0, end=1, kr=0, kcp=0, kep=1, kt=0, s=3, d=3)
    cmds.select(cl=1)

    upperWireDefomer = cmds.wire(upperHighDefCurve, gw=0, en=1, ce=0, li=0, w=upperLowDefCurve)
    upperWireTransNode = cmds.listConnections(upperWireDefomer[0] + '.baseWire[0]', source=1, destination=0)

    cmds.select(cl=1)

    # create control joint and controls for the LD curve
    upperControlJointList = []

    eachADD = 1.0 / (numCtrl - 1)

    for i in xrange(numCtrl):
        newJnt = cmds.joint(n=prefix + 'upper_' + rigPartName + '_CtrlJnt_' + str(i), radius=0.1)
        cmds.select(cl=1)
        motionPath = cmds.pathAnimation(upperLowDefCurve, newJnt, n=prefix + rigPartName + '_MP_' + str(i), fractionMode=1,
                                        follow=1, followAxis='x', upAxis='z', worldUpType='scene',
                                        inverseUp=0, inverseFront=0, bank=0)

        cmds.cutKey(motionPath + '.u', time=())

        cmds.setAttr(motionPath + '.uValue', eachADD * float(i))

        for attr in ['t', 'r']:
            for axis in ['x', 'y', 'z']:
                cmds.delete(newJnt + '.%s%s' % (attr, axis), icn=1)

        cmds.delete(motionPath)
        cmds.select(cl=1)

        upperControlJointList.append(newJnt)

        cmds.setAttr(newJnt + '.r', 0, 0, 0)
        cmds.select(cl=1)

    # bind LD curve by control joint
    cmds.skinCluster(upperControlJointList[:], upperLowDefCurve)
    cmds.select(cl=1)

    upperJntCtrlGrpList = []
    for i in xrange(len(upperControlJointList)):
        ctrl = control.Control(prefix=upperControlJointList[i],
                               rigPartName='',
                               scale=rigScale,
                               shape='circleY',
                               translateTo=upperControlJointList[i],
                               rotateTo=upperControlJointList[i])

        cmds.pointConstraint(ctrl.C, upperControlJointList[i], mo=0)
        cmds.orientConstraint(ctrl.C, upperControlJointList[i], mo=0)

        upperJntCtrlGrpList.append(ctrl.Off)

    cmds.select(cl=1)

    # clean hierarchy
    upperParentJntGrp = cmds.group(n=prefix + 'upper_' + rigPartName + '_skinJnt_Grp', em=1)
    upperLocGrp = cmds.group(n=prefix + 'upper_' + rigPartName + '_LOC_Grp', em=1)
    upperCurveGrp = cmds.group(n=prefix + 'upper_' + rigPartName + '_Crv_Grp', em=1)
    upperCtrlJntGrp = cmds.group(n=prefix + 'upper_' + rigPartName + '_ctrlJnt_Grp', em=1)
    upperCtrlGrp = cmds.group(n=prefix + 'upper_' + rigPartName + '_CtrlGrp', em=1)

    for i in upperEyeLidParentJntList:
        cmds.parent(i, upperParentJntGrp)

    for i in upperEyelidLocList:
        cmds.parent(i, upperLocGrp)

    cmds.parent(upperLowDefCurve, upperCurveGrp)
    cmds.parent(upperHighDefCurve, upperCurveGrp)
    cmds.parent(upperWireTransNode, upperCurveGrp)

    for i in upperControlJointList:
        cmds.parent(i, upperCtrlJntGrp)

    for i in upperJntCtrlGrpList:
        cmds.parent(i, upperCtrlGrp)

    cmds.setAttr(upperLocGrp + '.v', 0)
    cmds.setAttr(upperCurveGrp + '.v', 0)
    cmds.setAttr(upperCtrlJntGrp + '.v', 0)

    cmds.parent(upperParentJntGrp, upperLidRigModule.topGrp)
    cmds.parent(upperLocGrp, upperLidRigModule.topGrp)
    cmds.parent(upperCurveGrp, upperLidRigModule.topGrp)
    cmds.parent(upperCtrlJntGrp, upperLidRigModule.topGrp)
    cmds.parent(upperCtrlGrp, upperLidRigModule.topGrp)

###################################################################################################################

    #####################
    # Lower Eyelid Part #
    #####################
    
    # create lower eyelid Module
    lowerLidRigModule = module.Module(prefix=prefix, rigPartName='lower_' + rigPartName)

    # create eyelid joint for each vertex
    lowerEyeLidJointList = lib.vertex2Joints(vertexList=lowerVertexList, prefix=prefix,
                                             rigPartName='lower_' + rigPartName, radius=0.05)

    # connect attr
    for joint in lowerEyeLidJointList:
        if cmds.attributeQuery('slaveJoint', node=joint, exists=1):
            cmds.connectAttr(lowerLidRigModule.topGrp + '.slaveJoint', joint + '.slaveJoint', f=1)

    # create eyelid parent joint for each eyelid joint
    lowerEyeLidParentJntList = []
    for i in lowerEyeLidJointList:
        cmds.select(cl=1)

        parentJoint = cmds.joint(n=i + '_Parent', radius=0.05)

        cmds.delete(cmds.pointConstraint(eyeJoint, parentJoint, mo=0))

        cmds.delete(cmds.aimConstraint(i, parentJoint, aimVector=(1, 0, 0), upVector=(0, -1, 0),
                                       worldUpType='scene', weight=1, offset=(0, 0, 0), mo=0))
        cmds.parent(i, parentJoint)

        cmds.joint(i, e=1, oj='none', ch=1, zso=1)

        cmds.makeIdentity(parentJoint, apply=1, t=1, r=1, s=1)

        lowerEyeLidParentJntList.append(parentJoint)

    cmds.select(cl=1)

    lowerEyelidLocList = []
    # create locator for each eyelid joint
    for i in lowerEyeLidParentJntList:
        cmds.select(cl=1)

        eyelidJoint = cmds.listRelatives(i, c=1, type='joint', shapes=0)[0]

        ikHandle = cmds.ikHandle(n=eyelidJoint + '_IK', sj=i, ee=eyelidJoint, sol='ikSCsolver')

        eyelidLoc = cmds.spaceLocator(n=eyelidJoint + '_LOC')[0]

        cmds.delete(cmds.parentConstraint(eyelidJoint, eyelidLoc, mo=0))

        cmds.select(cl=1)

        cmds.setAttr(ikHandle[0] + '.v', 0)
        LOCShape = cmds.listRelatives(eyelidLoc, p=0, c=1, s=1)[0]
        cmds.setAttr(LOCShape + '.localScaleX', 0.1)
        cmds.setAttr(LOCShape + '.localScaleY', 0.1)
        cmds.setAttr(LOCShape + '.localScaleZ', 0.1)

        cmds.parent(ikHandle[0], eyelidLoc)

        lowerEyelidLocList.append(eyelidLoc)

    cmds.select(cl=1)

    # create high definition curve
    lowerPosList = []
    for i in lowerEyelidLocList:
        pos = cmds.xform(i, q=1, ws=1, t=1)
        lowerPosList.append(tuple(pos))

    lowerKList = []
    for i in xrange(len(lowerPosList)):
        lowerKList.append(i)

    lowerHighDefCurve = cmds.curve(n=prefix + 'lower_' + rigPartName + '_HD_Crv', p=lowerPosList, k=lowerKList, d=1)
    lowerLowDefCurve = cmds.duplicate(lowerHighDefCurve, n=prefix + 'lower_' + rigPartName + '_LD_Crv')

    lowerHighDefCurveShape = cmds.listRelatives(lowerHighDefCurve, p=0, c=0, s=1, path=1)[0]
    cmds.select(cl=1)

    # make each locator attach to the curve

    for i in lowerEyelidLocList:
        pos = cmds.xform(i, q=1, ws=1, t=1)
        uParam = lib.getUParam(pos, lowerHighDefCurveShape)

        PCI = cmds.createNode('pointOnCurveInfo', n=name.removeSuffix(i) + '_PCI')

        cmds.connectAttr(lowerHighDefCurveShape + '.worldSpace', PCI + '.inputCurve', f=1)

        cmds.setAttr(PCI + '.parameter', uParam)

        cmds.connectAttr(PCI + '.position', i + '.t')

        cmds.select(cl=1)

    # make HD curve deformed by LD curve
    lowerLowDefCurve = cmds.rebuildCurve(lowerLowDefCurve, ch=0, rpo=1, rt=0, end=1, kr=0, kcp=0, kep=1, kt=0, s=3, d=3)
    cmds.select(cl=1)

    lowerWireDefomer = cmds.wire(lowerHighDefCurve, gw=0, en=1, ce=0, li=0, w=lowerLowDefCurve)
    lowerWireTransNode = cmds.listConnections(lowerWireDefomer[0] + '.baseWire[0]', source=1, destination=0)

    cmds.select(cl=1)

    # create control joint and controls for the LD curve
    lowerControlJointList = []

    eachADD = 1.0 / (numCtrl - 1)

    for i in xrange(numCtrl - 2):
        newJnt = cmds.joint(n=prefix + 'lower_' + rigPartName + '_CtrlJnt_' + str(i + 1), radius=0.1)
        cmds.select(cl=1)
        motionPath = cmds.pathAnimation(lowerLowDefCurve, newJnt, n=prefix + rigPartName + '_MP_' + str(i + 1),
                                        fractionMode=1,
                                        follow=1, followAxis='x', upAxis='z', worldUpType='scene',
                                        inverseUp=0, inverseFront=0, bank=0)

        cmds.cutKey(motionPath + '.u', time=())

        cmds.setAttr(motionPath + '.uValue', eachADD * float(i + 1))

        for attr in ['t', 'r']:
            for axis in ['x', 'y', 'z']:
                cmds.delete(newJnt + '.%s%s' % (attr, axis), icn=1)

        cmds.delete(motionPath)
        cmds.select(cl=1)

        lowerControlJointList.append(newJnt)

        cmds.setAttr(newJnt + '.r', 0, 0, 0)
        cmds.select(cl=1)

    lowerControlJointList.insert(0, upperControlJointList[0])
    lowerControlJointList.append(upperControlJointList[-1])

    # bind LD curve by control joint
    cmds.skinCluster(lowerControlJointList[:], lowerLowDefCurve)
    cmds.select(cl=1)

    lowerJntCtrlGrpList = []
    for i in xrange(len(lowerControlJointList[1:-1])):
        ctrl = control.Control(prefix=lowerControlJointList[i+1],
                               rigPartName='',
                               scale=rigScale,
                               shape='circleY',
                               translateTo=lowerControlJointList[i+1],
                               rotateTo=lowerControlJointList[i+1])

        cmds.pointConstraint(ctrl.C, lowerControlJointList[i+1], mo=0)
        cmds.orientConstraint(ctrl.C, lowerControlJointList[i+1], mo=0)

        lowerJntCtrlGrpList.append(ctrl.Off)

    cmds.select(cl=1)

    # clean hierarchy
    lowerParentJntGrp = cmds.group(n=prefix + 'lower_' + rigPartName + '_skinJnt_Grp', em=1)
    lowerLocGrp = cmds.group(n=prefix + 'lower_' + rigPartName + '_LOC_Grp', em=1)
    lowerCurveGrp = cmds.group(n=prefix + 'lower_' + rigPartName + '_Crv_Grp', em=1)
    lowerCtrlJntGrp = cmds.group(n=prefix + 'lower_' + rigPartName + '_ctrlJnt_Grp', em=1)
    lowerCtrlGrp = cmds.group(n=prefix + 'lower_' + rigPartName + '_CtrlGrp', em=1)

    for i in lowerEyeLidParentJntList:
        cmds.parent(i, lowerParentJntGrp)

    for i in lowerEyelidLocList:
        cmds.parent(i, lowerLocGrp)

    cmds.parent(lowerLowDefCurve, lowerCurveGrp)
    cmds.parent(lowerHighDefCurve, lowerCurveGrp)
    cmds.parent(lowerWireTransNode, lowerCurveGrp)

    for i in lowerControlJointList:
        cmds.parent(i, lowerCtrlJntGrp)

    for i in lowerJntCtrlGrpList:
        cmds.parent(i, lowerCtrlGrp)

    cmds.setAttr(lowerLocGrp + '.v', 0)
    cmds.setAttr(lowerCurveGrp + '.v', 0)
    cmds.setAttr(lowerCtrlJntGrp + '.v', 0)

    cmds.parent(lowerParentJntGrp, lowerLidRigModule.topGrp)
    cmds.parent(lowerLocGrp, lowerLidRigModule.topGrp)
    cmds.parent(lowerCurveGrp, lowerLidRigModule.topGrp)
    cmds.parent(lowerCtrlJntGrp, lowerLidRigModule.topGrp)
    cmds.parent(lowerCtrlGrp, lowerLidRigModule.topGrp)

    # final
    cmds.parent(upperLidRigModule.topGrp, eyeLidRigModule.topGrp)
    cmds.parent(lowerLidRigModule.topGrp, eyeLidRigModule.topGrp)


```

## Source file: FacialRigTool/skinLib/skinLib.py

{
  "path": "FacialRigTool/skinLib/skinLib.py",
  "blob_sha": "aac1fff055f674e956aa905aca6b481655082d5d",
  "sha256": "1b8615988696f05f18280552c8b8d4acb906a4396d92e0703a7617104e34adaa",
  "lines": 530,
  "bytes": 19265,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 52,
    "deform": 7,
    "face": 1
  },
  "url": "https://github.com/tHeBeStXu/FacialRigTool/blob/9ea46068d8d77644c5f560fb68388de4389cfe4b/FacialRigTool/skinLib/skinLib.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def show():"
    },
    {
      "line": 23,
      "text": "def getMayaWindow():"
    },
    {
      "line": 32,
      "text": "def getShape(node, intermediate=False):"
    },
    {
      "line": 72,
      "text": "def export(cls, filePath=None, shape=None):"
    },
    {
      "line": 77,
      "text": "def createAndImport(cls, filePath=None, shape=None):"
    },
    {
      "line": 159,
      "text": "def getSkinCluster(cls, shape):"
    },
    {
      "line": 177,
      "text": "def removeNamespaceFromString(cls, influenceName):"
    },
    {
      "line": 195,
      "text": "def __init__(self, shape=None):"
    },
    {
      "line": 222,
      "text": "def gatherData(self):"
    },
    {
      "line": 234,
      "text": "def __getGeometryComponents(self):"
    },
    {
      "line": 256,
      "text": "def gatherInfluenceWeights(self, dagPath, components):"
    },
    {
      "line": 280,
      "text": "def gatherBlendWeights(self, dagPath, components):"
    },
    {
      "line": 291,
      "text": "def __getCurrentWeights(self, dagPath, components):"
    },
    {
      "line": 312,
      "text": "def exportSkin(self, filePath=None):"
    },
    {
      "line": 339,
      "text": "def setData(self, data):"
    },
    {
      "line": 352,
      "text": "def setInfluenceWeights(self, dagPath, components):"
    },
    {
      "line": 402,
      "text": "def setBlendWeights(self, dagPath, components):"
    },
    {
      "line": 417,
      "text": "def __init__(self, parent=None):"
    },
    {
      "line": 465,
      "text": "def setInfluences(self, importedInfluences, existingInfluences):"
    },
    {
      "line": 492,
      "text": "def setInfluencesMapping(self, src, label):"
    },
    {
      "line": 513,
      "text": "def __init__(self, parent=None):"
    }
  ]
}

```text

import os
import cPickle as pickle
import maya.cmds as cmds
import maya.OpenMaya as openmaya
import maya.OpenMayaUI as openmayaui
import maya.OpenMayaAnim as openmayaanim
from functools import partial


from PySide2 import QtGui, QtWidgets, QtCore
from shiboken2 import wrapInstance


def show():
    """
    Show Export and Import dialog UI
    :return:
    """
    dialog = SkinIODialog(getMayaWindow())
    dialog.show()


def getMayaWindow():
    """
    Get the mayaMainWindow as parent
    :return: mayaMainWindow Ptr
    """
    ptr = openmayaui.MQtUtil.mainWindow()
    return wrapInstance(long(ptr), QtWidgets.QMainWindow)


def getShape(node, intermediate=False):
    """
    Get the shape from specified node.

    :param node: node Name of a transform or shape node.
    :param intermediate: intermediate True to get the intermediate shape, False to get the visible shape
    :return: the name of the desired shape node
    """
    if cmds.nodeType(node) == 'transform':
        shapes = cmds.listRelatives(node, shapes=1, path=1)

        # from transform node to get the shape node
        if not shapes:
            shapes = []
        for shape in shapes:
            isIntermediate = cmds.getAttr('%s.intermediateObject' % shape)
            # Sometimes there are left over intermediate shapes that are not used to(the duplicated deformed shape)
            # check the connections to make sure we get the one that is used
            # return intermediate shape which has incoming objects
            if intermediate and isIntermediate and cmds.listConnections(shape, source=0, destination=1):
                return shape

            # return visible shape
            elif not intermediate and not isIntermediate:
                return shape

        if shapes:
            return shapes[0]
    # return the shape which just is a Shape Node
    elif cmds.nodeType(node) in ['mesh', 'nurbsCurve', 'nurbsSurface']:
        return node

    return None


class SkinCluster(object):
    # global variable extension
    kFileExtension = '.skin'

    @classmethod
    def export(cls, filePath=None, shape=None):
        skin = SkinCluster(shape)
        skin.exportSkin(filePath)

    @classmethod
    def createAndImport(cls, filePath=None, shape=None):
        """
        Create a skinCluster on the specified shape if one does not already exist
        and then import the weight data.
        :param filePath: filePath of the skinWeights
        :param shape: mesh shape which skinCluster deforms
        :return:
        """

        if not shape:
            try:
                shape = cmds.ls(sl=1)[0]

            except:
                raise RuntimeError('No shape selected')

        if filePath == None:
            startDir = cmds.workspace(q=1, rootDirectory=1)
            filePath = cmds.fileDialog2(dialogStyle=2, fileMode=1, startingDirectory=startDir,
                                        fileFilter='Skin Files (*%s)' % SkinCluster.kFileExtension)

        if not filePath:
            return
        if not isinstance(filePath, basestring):
            filePath = filePath[0]

        # Read the data from the file
        fh = open(filePath, 'rb')
        data = pickle.load(fh)
        fh.close()

        # Make sure the vertex count is the same
        meshVertices = cmds.polyEvaluate(shape, vertex=1)

        importedVertices = len(data['blendWeights'])
        if meshVertices != importedVertices:
            raise RuntimeError('Vertex counts do not match. %d != %d' % (meshVertices, importedVertices))


        # check if the shape already has a skinCluster
        if SkinCluster.getSkinCluster(shape):
            skinCluster = SkinCluster(shape)
        else:
            # create a new skinCluster
            joints = data['weights'].keys()

            # Make sure all the joints exist

            unusedImports = []
            # Create a set for get which joint in the scene doesn't have weights
            noMatch = set([SkinCluster.removeNamespaceFromString(x) for x in cmds.ls(type='joint')])

            for j in joints:
                if j in noMatch:
                    noMatch.remove(j)
                else:
                    unusedImports.append(j)

            # Remapping the joints
            # if there were unmapped influences ask the user to map them
            if unusedImports and noMatch:

                mappingDialog = WeightRemapDialog(getMayaWindow())
                mappingDialog.setInfluences(unusedImports, noMatch)
                mappingDialog.exec_()

                for src, dst in mappingDialog.mapping.items():
                    # swap the mapping
                    data['weights'][dst] = data['weights'][src]
                    del data['weights'][src]

            # Create the skinCluster with post normalization so setting the weights does not
            # normalize all weights
            joints = data['weights'].keys()

            skinCluster = cmds.skinCluster(joints, shape, tsb=1, nw=2, n=data['name'])
            skinCluster = SkinCluster(shape)

        skinCluster.setData(data)
        print "Imported %s" % filePath

    @classmethod
    def getSkinCluster(cls, shape):
        """
        Get the skinCluster node attached to the specified shape.
        :param shape: Shape node name
        :return: The attached skinCluster name or None if no skinCluster is attached
        """
        shape = getShape(shape)
        history = cmds.listHistory(shape, pruneDagObjects=1, interestLevel=2)

        if not history:
            return None
        skins = [x for x in history if cmds.nodeType(x) == 'skinCluster']

        if skins:
            return skins[0]
        return None

    @classmethod
    def removeNamespaceFromString(cls, influenceName):
        """
        Remove namespaces from a string
        CHANGES NAMESPACE:  joint1 | NAMESAPCE:joint2 -> joint1 | joint2
        :param influenceName: string, name with a namespace
        :return: string ,name without a namespace
        """
        tokens = influenceName.split('|')
        result = ''

        for i, tokens in enumerate(tokens):
            if i > 0:
                result += '|'

            result += tokens.split(':')[-1]

        return result

    def __init__(self, shape=None):
        if not shape:
            try:
                shape = cmds.ls(sl=1)[0]

            except:
                raise RuntimeError('No shape selected')

        self.shape = getShape(shape)
        if not self.shape:
            raise RuntimeError('No shape connected to %s' % shape)

        # Get the skinCluster node attached to the shape
        self.node = SkinCluster.getSkinCluster(self.shape)
        if not self.node:
            raise ValueError('No skinCluster attached to %s' % self.shape)

        # Get the skinCluster MObject
        selectionList = openmaya.MSelectionList()
        selectionList.add(self.node)
        self.mobj = openmaya.MObject()
        selectionList.getDependNode(0, self.mobj)
        self.fn = openmayaanim.MFnSkinCluster(self.mobj)
        self.data = {'weights': {},
                     'blendWeights': [],
                     'name': self.node}

    def gatherData(self):
        """
        get and store the skinningMethod and normalizeWeights attributes in data dictionary
        :return: None
        """
        dagPath, components = self.__getGeometryComponents()
        self.gatherInfluenceWeights(dagPath, components)
        self.gatherBlendWeights(dagPath, components)

        for attr in ['skinningMethod', 'normalizeWeights']:
            self.data[attr] = cmds.getAttr('%s.%s' % (self.node, attr))

    def __getGeometryComponents(self):
        """
        get the dagPath of influence object(joint) and Geometry components(vertex)
        :return: dagPath, componnets
        """
        # get dagPath and member components of skined shape
        # the deformerSet pretty controls which vertex is deformed by the skinCluster
        # the deformerSet will allows us to pull out that components(vertex) mobject that we need
        fnSet = openmaya.MFnSet(self.fn.deformerSet())
        members = openmaya.MSelectionList()
        # the MSelectionList contains the vertex information in the deformerSet above
        fnSet.getMembers(members, False)

        dagPath = openmaya.MDagPath()
        components = openmaya.MObject()

        # dagPath: dagPath of influence objects(joint)
        # components: mesh components(vertex)
        members.getDagPath(0, dagPath, components)

        return dagPath, components

    def gatherInfluenceWeights(self, dagPath, components):
        """
        get and store the weights of each influence object for same ordered components(vertex) in data['weights'] dictionary
        :param dagPath:
        :param components: mesh components(vertex)
        :return: None
        """
        # Gathers all the influence weights
        weights = self.__getCurrentWeights(dagPath, components)

        influencePaths = openmaya.MDagPathArray()
        # influencePaths is the fullPath of the object(joint)
        numInfluences = self.fn.influenceObjects(influencePaths)
        # weight size = number of components(vertex) * number of influenceObjects(joints)
        numComponentsPerInfluence = weights.length() / numInfluences

        for ii in range(influencePaths.length()):
            influenceName = influencePaths[ii].partialPathName()
            # we want to store the weights by influence without the namespace so it is easier
            # to import if the namespace is different
            influenceNameWithoutNamespace = SkinCluster.removeNamespaceFromString(influenceName)
            # store the weight of each influence object(joint) for same ordered components(vertex)
            self.data['weights'][influenceNameWithoutNamespace] = [weights[jj * numInfluences + ii] for jj in range(numComponentsPerInfluence)]

    def gatherBlendWeights(self, dagPath, components):
        """
        Gather the BlendWeights
        :param dagPath: dagPath of influence objects(joint)
        :param components: mesh components(vertex)
        :return: None
        """
        weights = openmaya.MDoubleArray()
        self.fn.getBlendWeights(dagPath, components, weights)
        self.data['blendWeights'] = [weights[i] for i in range(weights.length())]

    def __getCurrentWeights(self, dagPath, components):
        """
        Get the current weights array. Be careful about the weight array, it is a giant single array.
        The order of the weights array is dependent on the indices of the joint
        :param dagPath: path to object deformed by the skinCluster
        :param components: Components to return weight for, every single vertex in the mesh
        :return: Weights
        """
        weights = openmaya.MDoubleArray()
        util = openmaya.MScriptUtil()

        util.createFromInt(0)

        pUInt = util.asUintPtr()
        # Gets the skinCluster weights for ALL influenceObjects(joints) for the specified components of the object whose dagPath is specified.
        # More details see: https://download.autodesk.com/us/maya/2011help/API/class_m_fn_skin_cluster.html#82e83fc5ab653aa15c5431710b3ac86a
        self.fn.getWeights(dagPath, components, weights, pUInt)
        # weights is a giant single Double Array
        # size = number of components(vertex) * number of influenceObjects(joints)
        return weights

    def exportSkin(self, filePath=None):
        """
        Export the skinCluster data to disk
        :param filePath: File Path
        :return:
        """
        if filePath == None:
            startDir = cmds.workspace(q=1, rootDirectory=1)
            filePath = cmds.fileDialog2(dialogStyle=2, fileMode=0, startingDirectory=startDir,
                                        fileFilter='Skin Files(*%s)' % SkinCluster.kFileExtension)

        if not filePath:
            return

        filePath = filePath[0]

        if not filePath.endswith(SkinCluster.kFileExtension):
            filePath += SkinCluster.kFileExtension

        self.gatherData()

        fh = open(filePath, 'wb')
        pickle.dump(self.data, fh, pickle.HIGHEST_PROTOCOL)

        fh.close()
        print "Exported skinCluster (%d influences, %d vertices) %s" % (len(self.data['weights'].keys()), len(self.data['blendWeights']), filePath)

    def setData(self, data):
        """
        Sets the data and stores it in the Maya skinCluster node.
        :return:
        """
        self.data = data
        dagPath, components = self.__getGeometryComponents()
        self.setInfluenceWeights(dagPath, components)
        self.setBlendWeights(dagPath, components)

        for attr in ['skinningMethod', 'normalizeWeights']:
            cmds.setAttr('%s.%s' % (self.node, attr), self.data[attr])

    def setInfluenceWeights(self, dagPath, components):
        """

        :param dagPath:
        :param components:
        :return:
        """
        # get the existing weights and fill in the new weights
        weights = self.__getCurrentWeights(dagPath, components)
        influencePaths = openmaya.MDagPathArray()
        numInfluences = self.fn.influenceObjects(influencePaths)
        numComponentsPerInfluence = weights.length() / numInfluences

        # Keep track of which imported influences aren't used
        unusedImports = []
        # Keep track of which existing influences don't get anything imported
        noMatch = [influencePaths[ii].partialPathName() for ii in xrange(influencePaths.length())]

        for importedInfluence, importedWeights in self.data['weights'].items():
            for inf_count in xrange(influencePaths.length()):
                # partialPathName used to return exclusive partial path name of the object
                influenceName = influencePaths[inf_count].partialPathName()
                influenceWithoutNamespace = SkinCluster.removeNamespaceFromString(influenceName)

                if influenceWithoutNamespace == importedInfluence:
                    # Store the imported weights into the MDoubeArray
                    for jj in xrange(numComponentsPerInfluence):
                        weights.set(importedWeights[jj], jj * numInfluences + inf_count)

                    noMatch.remove(influenceName)
                    break
            else:
                unusedImports.append(importedInfluence)

        if unusedImports and noMatch:
            mappingDialog = WeightRemapDialog(getMayaWindow())
            mappingDialog.setInfluences(unusedImports, noMatch)
            mappingDialog.exec_()
            for src, dst in mappingDialog.mapping.items():
                for ii in range(influencePaths.length()):
                    if influencePaths[ii].partialPathName() == dst:
                        for jj in range(numComponentsPerInfluence):
                            weights.set(self.data['weights'][src][jj], jj*numInfluences+ii)
                        break

        influenceIndics = openmaya.MIntArray(numInfluences)
        for ii in range(numInfluences):
            influenceIndics.set(ii, ii)
        self.fn.setWeights(dagPath, components, influenceIndics, weights, False)

    def setBlendWeights(self, dagPath, components):
        """
        Set BlendWeights
        :param dagPath:
        :param components:
        :return:
        """
        blendWeights = openmaya.MDoubleArray(len(self.data['blendWeights']))
        for i, w in enumerate(self.data['blendWeights']):
            blendWeights.set(w, i)

        self.fn.setBlendWeights(dagPath, components, blendWeights)


class WeightRemapDialog(QtWidgets.QDialog):
    def __init__(self, parent=None):
        super(WeightRemapDialog, self).__init__(parent)

        self.setWindowTitle('Remap Weights')
        self.setObjectName('remapWeightUI')

        self.setModal(True)
        self.resize(600, 400)
        self.mapping = {}

        self.mainVbox = QtWidgets.QVBoxLayout()

        self.setLayout(self.mainVbox)

        label = QtWidgets.QLabel('The following influences have no corresponding influence from the import file. '
                                 'You can either remap the influences or skip them')

        label.setWordWrap(True)

        self.mainVbox.addWidget(label)

        self.HBox = QtWidgets.QHBoxLayout()
        self.mainVbox.addLayout(self.HBox)


        self.VBox = QtWidgets.QVBoxLayout()
        self.HBox.addLayout(self.VBox)

        self.VBox.addWidget(QtWidgets.QLabel('Unmapped influences'))

        self.existingInfluences = QtWidgets.QListWidget()
        self.VBox.addWidget(self.existingInfluences)

        vbox = QtWidgets.QVBoxLayout()
        self.HBox.addLayout(vbox)
        vbox.addWidget(QtWidgets.QLabel('Available imported influence'))
        self.scrollArea = QtWidgets.QScrollArea()
        self.widget = QtWidgets.QScrollArea()
        self.importedInfluenceLayout = QtWidgets.QVBoxLayout(self.widget)
        vbox.addWidget(self.widget)

        hbox = QtWidgets.QHBoxLayout()
        self.mainVbox.addLayout(hbox)
        hbox.addStretch()
        self.btn = QtWidgets.QPushButton('OK')
        self.btn.released.connect(self.accept)
        hbox.addWidget(self.btn)

    def setInfluences(self, importedInfluences, existingInfluences):
        infs = list(existingInfluences)
        infs.sort()
        self.existingInfluences.addItems(infs)

        width = 200
        for inf in importedInfluences:
            row = QtWidgets.QHBoxLayout()
            self.importedInfluenceLayout.addLayout(row)

            label = QtWidgets.QLabel(inf)
            row.addWidget(label)

            toggleBtn = QtWidgets.QPushButton('->')
            toggleBtn.setMaximumWidth(30)
            row.addWidget(toggleBtn)

            label = QtWidgets.QLabel('')
            label.setMaximumWidth(width)
            label.setSizePolicy(QtWidgets.QSizePolicy.Fixed, QtWidgets.QSizePolicy.Fixed)
            row.addWidget(label)

            toggleBtn.released.connect(partial(self.setInfluencesMapping, src=inf, label=label))

        self.importedInfluenceLayout.addStretch()


    def setInfluencesMapping(self, src, label):
        selectedInfluence = self.existingInfluences.selectedItems()

        if not selectedInfluence:
            return

        dst = selectedInfluence[0].text()

        label.setText(dst)

        self.mapping[src] = dst

        index = self.existingInfluences.indexFromItem(selectedInfluence[0])

        item = self.existingInfluences.takeItem(index.row())

        del item


class SkinIODialog(QtWidgets.QDialog):

    def __init__(self, parent=None):
        super(SkinIODialog, self).__init__(parent)

        self.setWindowTitle('Skin IO')
        self.setObjectName('skinioWidget')
        self.setModal(False)
        self.setFixedSize(200, 80)

        self.vbox = QtWidgets.QVBoxLayout()
        self.setLayout(self.vbox)

        self.exportBtn = QtWidgets.QPushButton("Export")
        self.vbox.addWidget(self.exportBtn)
        self.exportBtn.clicked.connect(SkinCluster.export)

        self.importBtn = QtWidgets.QPushButton("Import")
        self.vbox.addWidget(self.importBtn)
        self.importBtn.clicked.connect(SkinCluster.createAndImport)


```

## Source file: FacialRigTool/UI/Rig_UI.py

{
  "path": "FacialRigTool/UI/Rig_UI.py",
  "blob_sha": "b3e7424753e5455a3accb8190de57eaab1ddb159",
  "sha256": "3cd2045fc81290298d9986a8af658e0390511551a176abf9b2c147c39ca8a142",
  "lines": 96,
  "bytes": 2973,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 2,
    "rig": 26
  },
  "url": "https://github.com/tHeBeStXu/FacialRigTool/blob/9ea46068d8d77644c5f560fb68388de4389cfe4b/FacialRigTool/UI/Rig_UI.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, rigTypeName):"
    },
    {
      "line": 32,
      "text": "def buildUI(self):"
    },
    {
      "line": 74,
      "text": "def deleteWidget(self):"
    },
    {
      "line": 79,
      "text": "def editRigPart(self):"
    }
  ]
}

```text

from PySide2 import QtWidgets, QtCore, QtGui
import logging
import Edit_UI
from ..rig import *

reload(Edit_UI)

logging.basicConfig()
logger = logging.getLogger('FacialRiggingTool')
logger.setLevel(logging.INFO)


class RigWidget(QtWidgets.QFrame):

    def __init__(self, rigTypeName):
        super(RigWidget, self).__init__()

        self.setFrameStyle(QtWidgets.QFrame.Panel)
        self.setFrameShadow(QtWidgets.QFrame.Raised)

        self.setFixedHeight(100)
        self.setFixedWidth(200)

        self.rigArgs = {}
        self.rigTypeName = rigTypeName

        self.rigPartLineEdit = None
        self.editBtn = None

        self.buildUI()

    def buildUI(self):
        """
        build the Rig UI
        :return: None
        """
        self.editWidgetLayout = QtWidgets.QVBoxLayout()
        self.setLayout(self.editWidgetLayout)

        self.editWidgetLayout.setSpacing(2)

        self.closeBtnLayout = QtWidgets.QHBoxLayout()
        self.editLineLayout = QtWidgets.QHBoxLayout()
        self.editBtnLayout = QtWidgets.QHBoxLayout()

        self.editWidgetLayout.addLayout(self.closeBtnLayout)
        self.editWidgetLayout.addLayout(self.editLineLayout)
        self.editWidgetLayout.addLayout(self.editBtnLayout)

        self.closeBtn = QtWidgets.QPushButton('X')
        self.closeBtn.clicked.connect(self.deleteWidget)
        self.closeBtn.setFixedWidth(25)

        font = QtGui.QFont()
        font.setPointSize(10)
        font.setBold(1)
        self.rigTypeLable = QtWidgets.QLabel('Type: %s' % self.rigTypeName)
        self.rigTypeLable.setFont(font)

        self.closeBtnLayout.addWidget(self.rigTypeLable)
        self.closeBtnLayout.addWidget(self.closeBtn)

        label = QtWidgets.QLabel('Part Name:  ')
        self.rigPartLineEdit = QtWidgets.QLineEdit()
        self.rigPartLineEdit.setPlaceholderText('Enter a name')
        self.editLineLayout.addWidget(label)
        self.editLineLayout.addWidget(self.rigPartLineEdit)

        self.editBtn = QtWidgets.QPushButton('Edit...')
        self.editBtn.setFixedWidth(80)
        self.editBtn.clicked.connect(self.editRigPart)
        self.editBtnLayout.addWidget(self.editBtn)

    def deleteWidget(self):
        self.setParent(None)
        self.setVisible(False)
        self.deleteLater()

    def editRigPart(self):
        """
        Set the rig part info
        :return: None
        """
        if self.rigTypeName == 'vertex2Rig':
            Edit_UI.EditWidget(self, self.rigTypeName, vertex2Rig.createRig)

        elif self.rigTypeName == 'singleLineRig':
            Edit_UI.EditWidget(self, self.rigTypeName, singleLineRig.createRig)

        elif self.rigTypeName == 'cartoonEyeLidRig':
            Edit_UI.EditWidget(self, self.rigTypeName, cartoonEyeLidRig.createRig)

        else:
            logger.info('Unknow rig type: %s' % self.rigTypeName)

        logger.info('Edit %s Rig Part... (Type: %s)' % (self.rigPartLineEdit.text(), self.rigTypeName))


```

## Source file: FacialRigTool/rig/vertex2Rig.py

{
  "path": "FacialRigTool/rig/vertex2Rig.py",
  "blob_sha": "0aee2a847eb7bffc45141a8242e1ee26d9adafab",
  "sha256": "cfde72fd77525c64e85e73870bc91ae7354e379f79e02abeaa0248d0de37bf68",
  "lines": 88,
  "bytes": 2825,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 17
  },
  "url": "https://github.com/tHeBeStXu/FacialRigTool/blob/9ea46068d8d77644c5f560fb68388de4389cfe4b/FacialRigTool/rig/vertex2Rig.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def createRig(vertexList,"
    }
  ]
}

```text

import maya.cmds as cmds
from ..base import control
from ..base import module
from ..rigLib import lib

reload(control)
reload(module)
reload(lib)


def createRig(vertexList,
              prefix='L_',
              rigPartName='',
              rigScale=1.0,
              addSliderCtrls=True,
              jointParent=''):

    cmds.select(cl=1)

    # create module
    rigModule = module.Module(prefix=prefix,
                              rigPartName=rigPartName)

    # create joint on each vertex
    jointList = lib.vertex2Joints(vertexList=vertexList,
                                  prefix=prefix,
                                  rigPartName=rigPartName,
                                  addSlaveAttr=True)

    # parent created joint to target joint
    if jointParent:
        for i in jointList:
            cmds.select(cl=1)
            cmds.parent(i, jointParent)
    cmds.select(cl=1)

    # add control for each joint
    jointCtrlList = []
    jointCtrlGrpList = []
    # create controls
    for i in xrange(len(jointList)):
        jointCtrl = control.Control(prefix=jointList[i],
                                    rigPartName='',
                                    scale=rigScale * 0.2,
                                    translateTo=jointList[i],
                                    rotateTo=jointList[i],
                                    shape='circleY')

        cmds.pointConstraint(jointCtrl.C, jointList[i], mo=0)
        cmds.orientConstraint(jointCtrl.C, jointList[i], mo=0)

        jointCtrlList.append(jointCtrl.C)
        jointCtrlGrpList.append(jointCtrl.Off)

    # connect the attribute
    for i in jointList:
        cmds.connectAttr(rigModule.topGrp + '.' + prefix + rigPartName + '_Jnt',
                         i + '.' + prefix + rigPartName, f=1)

    # slider controls
    slideCtrlList = []
    slideCtrlGrpList = []

    if addSliderCtrls:
        for i in xrange(len(jointList)):
            slideCtrl = control.Control(prefix=prefix,
                                        rigPartName=rigPartName + '_Jnt_' + str(i) + '_SLD',
                                        scale=rigScale * 0.5,
                                        shape='planeSliderControl',
                                        lockChannels=['ty', 'rx', 'ry', 'rz', 'sx', 'sy', 'sz', 'v'])

            slideCtrlList.append(slideCtrl.C)
            slideCtrlGrpList.append(slideCtrl.Off)

    # cleanHierarchy
    for i in jointCtrlGrpList:
        cmds.parent(i, rigModule.topGrp)

    if addSliderCtrls:
        rigModuleSliderGrp = cmds.group(n=prefix + rigPartName + '_SLD_Grp', em=1)
        for i in slideCtrlGrpList:
            cmds.parent(i, rigModuleSliderGrp)

        cmds.parent(rigModuleSliderGrp, rigModule.topGrp)

        # set default keyframe
        cmds.setDrivenKeyframe()



```

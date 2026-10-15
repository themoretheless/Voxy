# Facial rendering source scan: revzin/HWMToolkit

Repository: https://github.com/revzin/HWMToolkit

Pinned source revision: 5fb1376c2a1493536e059ac46e0410f9228492b1

Discovery topic: facial rig

Description: Tools to aid HWM facial rigs authoring in Blender 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["sfm_hwm_sample_project/hwmtk/facerules.py"]}

## Source file: sfm_hwm_sample_project/hwmtk/facerules.py

{
  "path": "sfm_hwm_sample_project/hwmtk/facerules.py",
  "blob_sha": "15b89d8873c002f874e7d838d69a83a3a3c2ac9a",
  "sha256": "a42f97955f2c225e8cf5d938d745aeb49805318a64cc3be29f932c23cbf42eed",
  "lines": 68,
  "bytes": 1530,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 1,
    "wrinkle": 1,
    "face": 4
  },
  "url": "https://github.com/revzin/HWMToolkit/blob/5fb1376c2a1493536e059ac46e0410f9228492b1/sfm_hwm_sample_project/hwmtk/facerules.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __ImportDatamodelPy():"
    },
    {
      "line": 29,
      "text": "def __GetRawControl(dm, name):"
    },
    {
      "line": 32,
      "text": "def __GetDmeCIC(dm, name):"
    },
    {
      "line": 35,
      "text": "def __GetDmeDR(dm, name):"
    },
    {
      "line": 38,
      "text": "def __Finalize():"
    },
    {
      "line": 42,
      "text": "def UsePassthroughs():"
    },
    {
      "line": 45,
      "text": "def LoadFaceRules(dmxName):"
    },
    {
      "line": 48,
      "text": "def SaveFaceRules(dmxName):"
    },
    {
      "line": 51,
      "text": "def NewFaceRules(HWMDefaults = True):"
    },
    {
      "line": 54,
      "text": "def AddDominationRule(listDominators, listSuppressed):"
    },
    {
      "line": 57,
      "text": "def ReorderControls(*controlNames):"
    },
    {
      "line": 61,
      "text": "def GroupControls(groupName, *rawControlNames):"
    },
    {
      "line": 65,
      "text": "def SetWrinkleScale(controlName, rawControlName, scale):"
    }
  ]
}

```text

# ====================================
# Face rules
# ====================================

import bpy, inspect

'''

def __ImportDatamodelPy():
    import os, imp
    appdata = os.getenv("APPDATA")
    blender = os.path.join(appdata, "Blender Foundation\\Blender", str(bpy.app.version[0]) + '.' + str(bpy.app.version[1]))
    dm_path = os.path.join(blender, "scripts\\addons\\io_scene_valvesource", "datamodel.py")
    print (dm_path)
    if os.path.exists(dm_path):
        datamodel = imp.load_source("datamodel", dm_path)
        return datamodel
    else:
        return None
    
datamodel = __ImportDatamodelPy()
if (not datamodel):
    raise ArgumentError('Datamodel.py not preset where it should be -- check your Blender Source Tools?')

'''
    
dm_rules = None

def __GetRawControl(dm, name):
    pass

def __GetDmeCIC(dm, name):
    pass

def __GetDmeDR(dm, name):
    pass

def __Finalize():
    dm_rules = None
    mesh = None

def UsePassthroughs():
    ''' Purpose: '''    

def LoadFaceRules(dmxName):
    pass
    
def SaveFaceRules(dmxName):
    pass
    
def NewFaceRules(HWMDefaults = True):
    pass
  
def AddDominationRule(listDominators, listSuppressed):
    pass

def ReorderControls(*controlNames):
    ''' Reorders DmeCombinationInputControls by name '''
    pass

def GroupControls(groupName, *rawControlNames):
    ''' Creates a DmeCombinationInputControl groupName for rawControlNames'''
    pass
    
def SetWrinkleScale(controlName, rawControlName, scale):
    pass

# ===========================s

```

## Source file: sfm_hwm_sample_project/hwminit.py

{
  "path": "sfm_hwm_sample_project/hwminit.py",
  "blob_sha": "9f406522e370802bc6084763503ea856f89a44d4",
  "sha256": "4d8abcd2bb060c9c01ef1675ebe888778ecb858b2536de46e03ddcbbaa0ccc58",
  "lines": 30,
  "bytes": 1088,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/revzin/HWMToolkit/blob/5fb1376c2a1493536e059ac46e0410f9228492b1/sfm_hwm_sample_project/hwminit.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def poll(cls, context):"
    },
    {
      "line": 22,
      "text": "def execute(self, context):"
    }
  ]
}

```text

absoluteMeshName = "head_abs" # Adjust this line if your character’s head mesh isn’t named ’head_abs’

import sys, bpy

# Expose the HWMTK pyfiles to the interpreter
# Path to the HWMTK folder relative to the .blend file
sys.path.append(bpy.path.abspath('//hwmtk\\'))

# Enable the Soft Blend from Shape op
import op_softblend
# Import the main toolkit module
import hwm

class HwmOps_PreprocessHeadOp(bpy.types.Operator):
    """Preprocess abs correctors to rel correctors"""
    bl_idname = "object.hwm_hwmpreproc"
    bl_label = "HWM: Preprocess Head for Export"
    
    @classmethod
    def poll(cls, context):
        return bpy.context.mode == 'OBJECT'
    def execute(self, context):
        relMesh = hwm.PreprocessMesh(absoluteMeshName)
        if (not relMesh):
            self.report({'WARNING'}, 'Failed to preprocess the %s mesh. Look in the console for now...' % absoluteMeshName)
        else:
            self.report({'INFO'}, "Successfully updated %s..." % relMesh.name)
        return {'FINISHED'}  
    
bpy.utils.register_class(HwmOps_PreprocessHeadOp)      

```

## Source file: sfm_hwm_sample_project/hwmtk/hwm.py

{
  "path": "sfm_hwm_sample_project/hwmtk/hwm.py",
  "blob_sha": "66b11c2b2f0ee55d7a81711b4bf4a26c32cb5b3d",
  "sha256": "2bbab6d1f3fbf7b63f746578c073b223cee3e2d9f396145baf6477b07a6b6eb0",
  "lines": 175,
  "bytes": 6397,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/revzin/HWMToolkit/blob/5fb1376c2a1493536e059ac46e0410f9228492b1/sfm_hwm_sample_project/hwmtk/hwm.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def PreprocessMesh(meshName, scriptFile = None):"
    },
    {
      "line": 53,
      "text": "def Execute(script_path, var_dict):"
    },
    {
      "line": 150,
      "text": "def RebuildAbsoluteMesh(mesh_in):"
    },
    {
      "line": 172,
      "text": "def EnsureCorrectorsAreUnique(mesh_in):"
    }
  ]
}

```text

import bpy, bmesh
import obtools, shapescripting, shapetools, facerules, util
import os

from shapetools import *

from bpy.props import * 

from util import DebugPrint, GetMillisecs

if (util.IsDebugging()):
    import imp
    import op_softblend
    imp.reload(obtools)
    imp.reload(shapetools)
    imp.reload(facerules)
    imp.reload(util)
    imp.reload(shapescripting)
    imp.reload(op_softblend)


# This is a dictionary that exports the internal dmxedit emulation
# methods to the executed preprocess script
ShapeInterfaceDict = {
    "Add"               :   shapescripting.Add,
    "AddCorrected"      :   shapescripting.AddCorrected,
    "GrowSelection"     :   shapescripting.GrowSelection,
    "Interp"            :   shapescripting.Interp,
    "OverrideCorrector" :   shapescripting.OverrideCorrector,
    "ResetState"        :   shapescripting.ResetState,
    "SaveDelta"         :   shapescripting.SaveDelta,
    "Select"            :   shapescripting.Select,
    "SelectHalf"        :   shapescripting.SelectHalf,
    "SetState"          :   shapescripting.SetState,
    "ShrinkSelection"   :   shapescripting.ShrinkSelection,
    "DeleteDelta"       :   shapescripting.DeleteDelta,
    "Translate"         :   shapescripting.Translate,
    "bpy"               :   bpy,
    "GetMesh"           :   shapescripting.GetMesh,
    "PrintSel"          :   shapescripting.Debug_PrintSelection,
    "VisualiseSel"      :   shapescripting.Debug_WriteDownSelection
}


def PreprocessMesh(meshName, scriptFile = None):  
    # Purpose: preprocesses a HWM mesh by name either according to the specified script,
    # or just by converting every corrector to relative mode if no script is specified
    # There must be a '_raw' postfix in the mesh name.
    # A duplicate will be created.
    
    import traceback
    
    def Execute(script_path, var_dict):
        # Executes a script file
        if not os.path.exists(script_path):
            raise ValueError("Script file does not exist.")
        with open(script_path) as f:
            code = compile(f.read(), script_path, 'exec')
            exec(code, var_dict)
           
    
    DebugPrint("hwm.PreprocessMesh: meshName = %s scriptFile = %s" % (meshName, scriptFile))
            
    mesh_in = obtools.FindObject(meshName)
    
    print ("Preprocessing mesh %s" % meshName)
    
    if (not mesh_in):
        print ('Error: mesh %s not found!' % meshName)
        return None
        
    if not (mesh_in.name.endswith('_abs')):
        print ('Error: Please add "_abs" postfix to your absolute mesh name to avoid any confusion!')
        return None
    
    if (not shapetools.HasShapes(mesh_in)):
        print ('Error: mesh %s does not have any relative shape keys!' % mesh_in.name)
        return None
    
    if (not shapetools.ValidateShapeNames(mesh_in)):
        print ('Error: mesh %s has a shape with an invalid name!' % mesh_in.name)
        return None
    
    if (not shapetools.CheckForRedundantCorrectives(mesh_in)):
        print ('Error: mesh %s has redundant corrective shapes!' % mesh_in.name)  
        return None 
    
    # Create the new mesh
    mesh_out = obtools.DuplicateObject(mesh_in.name, 
                                        mesh_in.name.replace('_abs', '_rel'))
                                        
         
    if (not mesh_out):
        print ('Failed to duplcate the mesh, aborting...')
        return None
    
    DebugPrint("Duplicated to %s" % mesh_out.name)    
    
    if scriptFile:    
        if shapescripting.OperateOnMesh(mesh_out) == None: # Set up the mesh
            print("Invalid object specified!")
            return None
        print ('Executing', scriptFile)
        failed = False
        try:
            Execute(bpy.path.abspath(scriptFile), ShapeInterfaceDict)
        except:
            traceback.print_exc()
            failed = True
        finally:
            if failed:
                print ('Script execution failed, restoring...')
                obtools.DeleteObject(mesh_out.name)
                return None
        shapescripting.Cleanup()  
        return None    
    else:
        maxRank = 1
        for shape in mesh_out.data.shape_keys.key_blocks:
            shape.value = 0.0
            if (shapescripting.SELECTOR_PREFIX in shape.name):
                DebugPrint("Removing selector %s" % shape.name)
                RemoveShapeKey(mesh_out, shape.name)
            rank = GetShapeRank(shape.name)
            if rank > maxRank:
                maxRank = rank
                
        for i in range(2, maxRank + 1):
            rankStartTime = GetMillisecs()
            rankShapeCount = 0
            for shape in mesh_out.data.shape_keys.key_blocks:
                if GetShapeRank(shape.name) == i:
                    rankShapeCount += 1
                    if (not Corr_AbsToRel(mesh_out, mesh_out, shape, shape)):
                        DebugPrint('Deleting mesh_out')
                        obtools.DeleteObject(mesh_out.name)
                        return None
                    DebugPrint('Converted %s to relative' % shape.name, 2)
            deltaTime = GetMillisecs() - rankStartTime
            DebugPrint('Rank %i took %i msec, avg %i msec' % (i, deltaTime, deltaTime / rankShapeCount))       

    for key in mesh_out.data.shape_keys.key_blocks:
        key.value = 0.0
              
    DebugPrint ('PreprocessMesh done')

    return mesh_out
    
    
def RebuildAbsoluteMesh(mesh_in):
    print ('\nRebuilding correctors mesh from', mesh_in.name)
    mesh_out = obtools.DuplicateObject(mesh_in.name, mesh_in.name + '_absolute_correctors', False)
    maxRank = 1
    for shape in mesh_out.data.shape_keys.key_blocks:
        rank = GetShapeRank(shape.name)
        if rank > maxRank:
            maxRank = rank
    if maxRank == 1:
        print ('Nothing to convert here...')
        return
    
    for i in reversed(range(2, maxRank)): 
        print ('On to rank {} shapes...'.format(i))
        for shape in mesh_out.data.shape_keys.key_blocks:
            if GetShapeRank(shape.name) == i and IsCorrectorShapeName(shape.name):
                Corr_RelToAbs(mesh_out, mesh_out, shape, shape)
                print ('Converted', shape.name)
    print ('Done converting, created', mesh_out.name)
    
     
    
def EnsureCorrectorsAreUnique(mesh_in):
    pass

DebugPrint('hwm.py reloaded...')

```

## Source file: sfm_hwm_sample_project/hwmtk/util.py

{
  "path": "sfm_hwm_sample_project/hwmtk/util.py",
  "blob_sha": "9021ea6748f23ddc35c6c047f419afd2f9cbc725",
  "sha256": "8aaf5fd4a072222ce0595a2783c39961f70cf5bd74d1fb6df2c96a0a12aea2bd",
  "lines": 18,
  "bytes": 307,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/revzin/HWMToolkit/blob/5fb1376c2a1493536e059ac46e0410f9228492b1/sfm_hwm_sample_project/hwmtk/util.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def DebugPrint(msg, level = 1):"
    },
    {
      "line": 11,
      "text": "def IsDebugging():"
    },
    {
      "line": 14,
      "text": "def GetMillisecs():"
    }
  ]
}

```text

import bpy

import time

# I only hope for more

def DebugPrint(msg, level = 1):
    if (level <= int(bpy.app.debug_value)):
        print (msg)
        
def IsDebugging():
    return bpy.app.debug_value > 0

def GetMillisecs():
    return int(round(time.time() * 1000))


DebugPrint('util.py reloaded...')


```

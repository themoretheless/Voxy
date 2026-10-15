# Facial rendering source scan: meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations

Repository: https://github.com/meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations

Pinned source revision: 3a244841e66078d4e519de08aa7fd708caba864a

Discovery topic: facial animation

Description: A Blender GUI based Interactive Genetic Algorithms to Generate Facial Expression Animations on a 3D Face Model

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["textures/eye.png", "textures/eyeCornea.png"], "mouth": ["textures/mouthLower.COL.png", "textures/mouthLower.NOR.png", "textures/mouthUpper.COL.png", "textures/mouthUpper.NOR.png"], "cornea": ["textures/eyeCornea.png"], "rig": ["docs/MISC/Sophia_rig_adjustments.blend", "rigAPI/CommandSource.py", "rigAPI/__pycache__/CommandSource.cpython-34.pyc", "rigAPI/__pycache__/rigAPI.cpython-34.pyc", "rigAPI/rigAPI.py", "rigControl/CommandListener.py", "rigControl/README.md", "rigControl/__init__.py", "rigControl/__pycache__/CommandListener.cpython-34.pyc", "rigControl/__pycache__/__init__.cpython-34.pyc", "rigControl/__pycache__/actuators.cpython-34.pyc", "rigControl/__pycache__/animationManager.cpython-34.pyc", "rigControl/__pycache__/blenderPlayback.cpython-34.pyc", "rigControl/__pycache__/blenderUI.cpython-34.pyc", "rigControl/__pycache__/commands.cpython-34.pyc", "rigControl/__pycache__/helpers.cpython-34.pyc", "rigControl/actuators.py", "rigControl/animationManager.py", "rigControl/blendedNum/README.md", "rigControl/blendedNum/__init__.py", "rigControl/blendedNum/__pycache__/__init__.cpython-34.pyc", "rigControl/blendedNum/__pycache__/plumbing.cpython-34.pyc", "rigControl/blendedNum/plumbing.py", "rigControl/blenderPlayback.py", "rigControl/blenderUI.py", "rigControl/commands.py", "rigControl/helpers.py"]}

## Source file: rigAPI/rigAPI.py

{
  "path": "rigAPI/rigAPI.py",
  "blob_sha": "fa42a98bade8d288718f9984ef3a59be48f59357",
  "sha256": "88158ab8ef4f07502935aa68cb8c60fab565ae1dac98e402b33b91d8cc1b2184",
  "lines": 109,
  "bytes": 3106,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "rig": 7,
    "face": 2
  },
  "url": "https://github.com/meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations/blob/3a244841e66078d4e519de08aa7fd708caba864a/rigAPI/rigAPI.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def getAPIVersion(self):"
    },
    {
      "line": 10,
      "text": "def isAlive(self):"
    },
    {
      "line": 15,
      "text": "def availableSomaStates(self):"
    },
    {
      "line": 18,
      "text": "def getSomaStates(self):"
    },
    {
      "line": 21,
      "text": "def setSomaState(self, emotion):"
    },
    {
      "line": 26,
      "text": "def availableEmotionStates(self):"
    },
    {
      "line": 29,
      "text": "def getEmotionStates(self):"
    },
    {
      "line": 32,
      "text": "def setEmotionState(self, emotion):"
    },
    {
      "line": 37,
      "text": "def availableGestures(self):"
    },
    {
      "line": 40,
      "text": "def getGestures(self):"
    },
    {
      "line": 43,
      "text": "def setGesture(self, name, repeat=1, speed=1, magnitude=0.5):"
    },
    {
      "line": 46,
      "text": "def stopGesture(self, gestureID, smoothing):"
    },
    {
      "line": 50,
      "text": "def availableVisemes(self):"
    },
    {
      "line": 53,
      "text": "def queueVisemes(self, vis, start=0, duration=0.5, \\"
    },
    {
      "line": 62,
      "text": "def setFaceTarget(self, location):"
    },
    {
      "line": 65,
      "text": "def setGazeTarget(self, location):"
    },
    {
      "line": 82,
      "text": "def getHeadData(self):"
    },
    {
      "line": 85,
      "text": "def getNeckData(self):"
    },
    {
      "line": 91,
      "text": "def getEyesData(self):"
    },
    {
      "line": 94,
      "text": "def getFaceData(self):"
    },
    {
      "line": 99,
      "text": "def setNeckRotation(self, pitch, roll):"
    },
    {
      "line": 102,
      "text": "def setParam(self, key, value):"
    },
    {
      "line": 105,
      "text": "def getParam(self, param):"
    },
    {
      "line": 108,
      "text": "def getAnimationLength(self, animation):"
    }
  ]
}

```text

#
# Define the blender rig API
# This is an abstract base class

class RigAPI:

    def getAPIVersion(self):
        return None

    def isAlive(self):
        return None

    # Somatic states  --------------------------------
    # awake, asleep, drunk, dazed and confused ...
    def availableSomaStates(self):
        return None

    def getSomaStates(self):
        return None

    def setSomaState(self, emotion):
        return None

    # Emotion expressions ----------------------------
    # smiling, frowning, bored ...
    def availableEmotionStates(self):
        return None

    def getEmotionStates(self):
        return None

    def setEmotionState(self, emotion):
        return None

    # Gestures --------------------------------------
    # blinking, nodding, shaking...
    def availableGestures(self):
        return None

    def getGestures(self):
        return None

    def setGesture(self, name, repeat=1, speed=1, magnitude=0.5):
        return None

    def stopGesture(self, gestureID, smoothing):
        return None

    # Visemes --------------------------------------
    def availableVisemes(self):
        return None

    def queueVisemes(self, vis, start=0, duration=0.5, \
        rampin=0.1, rampout=0.8, magnitude=1):
        return None

    # Eye look-at targets ==========================
    # The coordinate system used is head-relative, in 'engineering'
    # coordinates: 'x' is forward, 'y' to the left, and 'z' up.
    # Distances are measured in meters.  Origin of the coordinate
    # system is somewhere (where?) in the middle of the head.
    def setFaceTarget(self, location):
        return None

    def setGazeTarget(self, location):
        return None

    # ========== info dump for ROS
    # Get Head and Neck rotation quaternion in XYZ format data structure.
    # Its assumed that the robot neck assembly has two joints; the upper
    # joint is the "head" joint, the lower one is the "neck" joint.
    #
    # Pitch: Z (positive down, negative up)
    # Yaw: X (negative left to positive right)
    #
    # XXX TODO It does not really make sense to specify two quaternions,
    # without also specifying the physical distance between the two
    # joints.  Right now, this distanc is hard-coded in various random
    # places, but this should be fixed in some way.  (For example, maybe
    # the blender rig should use physical dimensions queried from the
    # actual robot being modelled??)
    def getHeadData(self):
        return None

    def getNeckData(self):
        return None

    # Get Eye rotation angles:
    # Pitch: down(negative) to up(positive)
    # Yaw: left (negative) to right(positive)
    def getEyesData(self):
        return None

    def getFaceData(self):
        return None

    # Set the lower neck rotation
    # Accepts pitch and roll as parameters for the neck
    def setNeckRotation(self, pitch, roll):
        return None

    def setParam(self, key, value):
        return None

    def getParam(self, param):
        return None

    def getAnimationLength(self, animation):
        return None

```

## Source file: rigControl/helpers.py

{
  "path": "rigControl/helpers.py",
  "blob_sha": "6b0d121ba7fc08805cb93e026c1104593a8b1983",
  "sha256": "24d41885dbe87bd7d2fe7288dbe328eadb225ce4f8ae8287f7024b95a657fc96",
  "lines": 114,
  "bytes": 3751,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 4
  },
  "url": "https://github.com/meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations/blob/3a244841e66078d4e519de08aa7fd708caba864a/rigControl/helpers.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def mix(a,b,factor):"
    },
    {
      "line": 20,
      "text": "def checkValue(var, minV, maxV):"
    },
    {
      "line": 25,
      "text": "def smoothstep(x):"
    },
    {
      "line": 30,
      "text": "def computeDistance(a, b):"
    },
    {
      "line": 41,
      "text": "def mapValue(value, leftMin, leftMax, rightMin, rightMax):"
    },
    {
      "line": 55,
      "text": "def randomSelect(probs):"
    },
    {
      "line": 64,
      "text": "def m2bu(m, scale=0.25):"
    },
    {
      "line": 75,
      "text": "def get_pose_matrix_in_other_space(mat, pose_bone, pose_refbone=None):"
    },
    {
      "line": 106,
      "text": "def get_bones_rotation_rad(armature, bone, axis, refbone=None):"
    }
  ]
}

```text

# Implements helper functions used by the rest of the module

import math, random

def mix(a,b,factor):
    '''mix two number together using a factor'''
    if type(a) is list or type(a) is tuple:
        if len(a)==len(b)==2:
            return [a[0]*factor + b[0]*(1.0-factor), a[1]*factor + b[1]*(1.0-factor)]
        elif len(a)==len(b)==3:
            return [a[0]*factor + b[0]*(1.0-factor), a[1]*factor + b[1]*(1.0-factor), a[2]*factor + b[2]*(1.0-factor)]
        elif len(a)==len(b)==4:
            return [a[0]*factor + b[0]*(1.0-factor), a[1]*factor + b[1]*(1.0-factor), a[2]*factor + b[2]*(1.0-factor), a[3]*factor + b[3]*(1.0-factor)]
        else:
            raise Exception(ArithmeticError)
    else:
        return (a*factor + b*(1.0-factor))


def checkValue(var, minV, maxV):
    if not minV <= var <= maxV:
        print('Warning:', var, 'is outside of the expected range ', ':', minV, '-' , maxV)


def smoothstep(x):
    '''returns a smoothed float given an input between 0 and 1'''
    return x * x * (3-2*(x))


def computeDistance(a, b):
    if len(a)==len(b)==2:
        return math.sqrt((a[0]-b[0])**2 + (a[1]-b[1])**2)
    elif len(a)==len(b)==3:
        return math.sqrt((a[0]-b[0])**2 + (a[1]-b[1])**2 + (a[2]-b[2])**2)
    elif len(a)==len(b)==4:
        return math.sqrt((a[0]-b[0])**2 + (a[1]-b[1])**2 + (a[2]-b[2])**2 + (a[3]-b[3])**2)
    else:
        raise Exception(ArithmeticError)


def mapValue(value, leftMin, leftMax, rightMin, rightMax):
    # map value from one range to another range

    # Figure out how 'wide' each range is
    leftSpan = leftMax - leftMin
    rightSpan = rightMax - rightMin

    # Convert the left range into a 0-1 range (float)
    valueScaled = float(value - leftMin) / float(leftSpan)

    # Convert the 0-1 range into a value in the right range.
    return rightMin + (valueScaled * rightSpan)


def randomSelect(probs):
    r = random.random()
    index = 0
    while(r >= 0 and index < len(probs)):
        r -= probs[index]
        index += 1
    return index - 1


def m2bu(m, scale=0.25):
    # converts input (assuming centimeters) to Blender Unit:
    # Beorn says in github #25 that one BU should be about 20cm.
    if type(m) is list or type(m) is tuple:
        bu = [i/scale for i in m]
    else:
        bu = m/scale
    return bu

from mathutils import Matrix

def get_pose_matrix_in_other_space(mat, pose_bone, pose_refbone=None):
    """
    Returns the transform matrix relative to pose_bone's current
    transform space. In other words, presuming that mat is in
    armature space, slapping the returned matrix onto pose_bone
    should give it the armature-space transforms of mat.

    If pose_refbone (reference bone) is set, it is used instead of pose_bone's
    parent.

    TODO: try to handle cases with axis-scaled parents better.
    """
    rest = pose_bone.bone.matrix_local.copy()
    rest_inv = rest.inverted()
    if pose_refbone == None and pose_bone.parent:
        pose_refbone = pose_bone.parent
    if pose_refbone:
        par_mat = pose_refbone.matrix.copy()
        par_inv = par_mat.inverted()
        par_rest = pose_refbone.bone.matrix_local.copy()
    else:
        par_mat = Matrix()
        par_inv = Matrix()
        par_rest = Matrix()

    # Get matrix in bone's current transform space
    smat = rest_inv * (par_rest * (par_inv * mat))

    return smat


def get_bones_rotation_rad(armature, bone, axis, refbone=None):
    if refbone:
        mat = get_pose_matrix(
            bpy.data.objects[armature].pose.bones[bone],
            bpy.data.objects[armature].pose.bones[refbone]
            )
    else:
        mat = get_local_pose_matrix(bpy.data.objects[armature].pose.bones[bone])
    return getattr(mat.to_euler(), axis)


```

## Source file: rigControl/__init__.py

{
  "path": "rigControl/__init__.py",
  "blob_sha": "b7e43961dd1167780440be7d48d849142a1703bb",
  "sha256": "8eb4643e91fbb5e606009aa73739a4d58adfb4fe507dacab5b2acb1ccb29e6dd",
  "lines": 47,
  "bytes": 1243,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 3,
    "face": 1
  },
  "url": "https://github.com/meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations/blob/3a244841e66078d4e519de08aa7fd708caba864a/rigControl/__init__.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def init():"
    }
  ]
}

```text

#
# __init__.py
#
# Initialize the rigControl module
#

def init():

    from . import animationManager
    from . import blenderUI
    from . import blenderPlayback
    from . import CommandListener

    import imp

    # Goofy hack -- It seems that we are called twice, by ../loader.py.
    # The first time by `import rigControl` at line 11, and then again by
    # imp.reload(rigControl) at line 12.  This is intentional: it allows
    # the blender tdev to reload without quitting blender.  But ... I
    # don't really want to init ROS twice, so I will hack around this.
    # The very first time through, the NameError exception is thrown, and
    # ROS gets initilized. The second time through, we return w/o the init.
    try:
        stuff = is_init
    except NameError:
        is_init = False

    if is_init:
        return

    is_init = True

    # sets up the Command listener
    imp.reload(CommandListener)
    CommandListener.refresh()

    # sets up the Blender operators
    imp.reload(blenderPlayback)
    blenderPlayback.refresh()

    # sets up the Blender interface
    imp.reload(blenderUI)
    blenderUI.refresh()

    # init animation Manager singleton
    imp.reload(animationManager)
    animationManager.init()


```

## Source file: rigControl/commands.py

{
  "path": "rigControl/commands.py",
  "blob_sha": "3796bcff40d31f67a68daef447caf2d3f69fe9a0",
  "sha256": "2a51e309e2ba3da4513f86c086d3f31a27f04eb4c9da516b3171ff792c260724",
  "lines": 561,
  "bytes": 26467,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 23,
    "jaw": 3,
    "mouth": 11,
    "gum": 1,
    "deform": 5,
    "rig": 8,
    "face": 3
  },
  "url": "https://github.com/meareghm/Blender-API-to-Evolve-Facial-Expressions-Animations/blob/3a244841e66078d4e519de08aa7fd708caba864a/rigControl/commands.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def init():"
    },
    {
      "line": 20,
      "text": "def getEnvironment():"
    },
    {
      "line": 23,
      "text": "def terminate():"
    },
    {
      "line": 32,
      "text": "def __init__(self):"
    },
    {
      "line": 36,
      "text": "def getAPIVersion(self):"
    },
    {
      "line": 39,
      "text": "def isAlive(self):"
    },
    {
      "line": 44,
      "text": "def availableSomaStates(self):"
    },
    {
      "line": 51,
      "text": "def getSomaStates(self):"
    },
    {
      "line": 62,
      "text": "def setSomaState(self, state):"
    },
    {
      "line": 73,
      "text": "def availableEmotionStates(self):"
    },
    {
      "line": 81,
      "text": "def getEmotionStates(self):"
    },
    {
      "line": 90,
      "text": "def setEmotionState(self, emotion):"
    },
    {
      "line": 98,
      "text": "def availableGestures(self):"
    },
    {
      "line": 106,
      "text": "def getGestures(self):"
    },
    {
      "line": 118,
      "text": "def setGesture(self, name, repeat=1, speed=1, magnitude=1.0):"
    },
    {
      "line": 124,
      "text": "def setGesture_IGA(self, name, repeat=1, speed=1, magnitude=1.0):"
    },
    {
      "line": 131,
      "text": "def listPopulation_IGA(self):"
    },
    {
      "line": 159,
      "text": "def importPopulation_IGA(self, name, repeat=1, speed=1, magnitude=1.0):"
    },
    {
      "line": 166,
      "text": "def select_Facialparts(self):#, selected):"
    },
    {
      "line": 216,
      "text": "def removeActions_fromIGA(self):"
    },
    {
      "line": 279,
      "text": "def saveAction_IGA(self):"
    },
    {
      "line": 301,
      "text": "def renameAction(self):"
    },
    {
      "line": 317,
      "text": "def childGenerator(self):"
    },
    {
      "line": 425,
      "text": "def stopGesture(self, gestureID, smoothing):"
    },
    {
      "line": 430,
      "text": "def availableVisemes(self):"
    },
    {
      "line": 438,
      "text": "def queueViseme(self, vis, start=0, duration=0.5, \\"
    },
    {
      "line": 449,
      "text": "def setFaceTarget(self, loc):"
    },
    {
      "line": 456,
      "text": "def setGazeTarget(self, loc):"
    },
    {
      "line": 461,
      "text": "def setBlinkRandomly(self,interval_mean,interval_variation):"
    },
    {
      "line": 465,
      "text": "def setSaccade(self,interval_mean,interval_variation,paint_scale,eye_size,eye_distance,mouth_width,mouth_height,weight_eyes,weight_mouth):"
    },
    {
      "line": 479,
      "text": "def getHeadData(self):"
    },
    {
      "line": 491,
      "text": "def getNeckData(self):"
    },
    {
      "line": 501,
      "text": "def getEyesData(self):"
    },
    {
      "line": 517,
      "text": "def getFaceData(self):"
    },
    {
      "line": 532,
      "text": "def setNeckRotation(self, pitch, roll):"
    },
    {
      "line": 535,
      "text": "def setParam(self, key, value):"
    },
    {
      "line": 545,
      "text": "def getParam(self, param):"
    },
    {
      "line": 553,
      "text": "def getAnimationLength(self, animation):"
    }
  ]
}

```text

# Implements the commands defined by the public API
import bpy
from mathutils import Matrix, Euler
from math import pi
from collections import OrderedDict
import logging
import pymsgbox
from random import randint

from rigAPI.rigAPI import RigAPI
from GA_methods import IGA

logger = logging.getLogger('hr.blender_api.rigcontrol.commands')
# ====================================================

def init():
    bpy.ops.wm.animation_playback()
    return 0

def getEnvironment():
    return None

def terminate():
    return 0


class EvaAPI(RigAPI):
    selected_actions = IGA.selected_actions
    initial_population = [] # initialize population for the IGA


    def __init__(self):
        pass

    # System control and information commands ===========
    def getAPIVersion(self):
        return 4

    def isAlive(self):
        return int(bpy.context.scene['animationPlaybackActive'])

    # Somatic states  --------------------------------
    # awake, asleep, drunk, dazed and confused ...
    def availableSomaStates(self):
        somaStates = []
        for state in bpy.data.actions:
            if state.name.startswith("CYC-"):
                somaStates.append(state.name[4:])
        return somaStates

    def getSomaStates(self):
        eva = bpy.evaAnimationManager
        somaStates = {}
        for cycle in eva.cyclesSet:
            magnitude = round(cycle.magnitude, 3)
            rate = round(cycle.rate, 3)
            ease_in = round(cycle.ease_in, 3)
            somaStates[cycle.name] = {'magnitude': magnitude, 'rate': rate,
                'ease_in': ease_in}
        return somaStates

    def setSomaState(self, state):
        name = 'CYC-' + state['name']
        rate = state['rate']
        magnitude = state['magnitude']
        ease_in = state['ease_in']
        bpy.evaAnimationManager.setCycle(name=name,
            rate=rate, magnitude=magnitude, ease_in=ease_in)
        return 0

    # Emotion expressions ----------------------------
    # smiling, frowning, bored ...
    def availableEmotionStates(self):
        emotionStates = []
        for emo in bpy.data.objects['control'].pose.bones:
            if emo.name.startswith('EMO-'):
                emotionStates.append(emo.name[4:])
        return emotionStates


    def getEmotionStates(self):
        eva = bpy.evaAnimationManager
        emotionStates = {}
        for emotion in eva.emotionsList:
            magnitude = round(emotion.magnitude.current, 3)
            emotionStates[emotion.name] = {'magnitude': magnitude}
        return emotionStates


    def setEmotionState(self, emotion):
        # TODO: expand arguments and update doc
        bpy.evaAnimationManager.setEmotion(eval(emotion))
        return 0


    # Gestures --------------------------------------
    # blinking, nodding, shaking...
    def availableGestures(self):
        emotionGestures = []
        for gesture in bpy.data.actions:
            if gesture.name.startswith("GST-"):
                emotionGestures.append(gesture.name[4:])
        return emotionGestures


    def getGestures(self):
        eva = bpy.evaAnimationManager
        emotionGestures = {}
        for gesture in eva.gesturesList:
            duration = round(gesture.duration*gesture.repeat - gesture.stripRef.strip_time, 3)
            magnitude = round(gesture.magnitude, 3)
            speed = round(gesture.speed, 3)
            emotionGestures[gesture.name] = {'duration': duration, \
                'magnitude': magnitude, 'speed': speed}
        return emotionGestures


    def setGesture(self, name, repeat=1, speed=1, magnitude=1.0):
        bpy.evaAnimationManager.newGesture(name='GST-'+name, \
            repeat=repeat, speed=speed, magnitude=magnitude)
        return 0

    # Genetic algorithm related functions   
    def setGesture_IGA(self, name, repeat=1, speed=1, magnitude=1.0):

        bpy.evaAnimationManager.newGesture_IGA(name='IGA-'+name, \
            repeat=repeat, speed=speed, magnitude=magnitude)
        return 0    

    # Add Initial population set to the IGA panel  
    def listPopulation_IGA(self):
        action_exists = 0
        action_names = bpy.data.scenes['Scene']['action_add_IGA'] # An example input can be... GST-amused : GST-thoughtful : GST-nod-1 : GST-shake-2
        action_list = action_names.split(' : ') #converts text input into lists
        print (action_list)
        
        #checked_list = []
        for i in range(0, len(action_list)):
            flag = 0
            action_exists = 0

            for j, gesture in enumerate(bpy.data.actions):  
                if gesture.name == action_list[i] and flag == 0:
                    previous_name = gesture.name[4:] # [4:] removing the 'GST-' prefix copies it to the previous_name variable.
                    gesture = gesture.copy()
                    gesture.name= 'IGA-' + previous_name # So, for example the new copy of 'GST-thought' will be 'IGA-thoughtful'
                    action_exists = 1
                    flag = 1
                    if gesture.name not in self.initial_population:
                        self.initial_population.append(gesture.name)
                        print("An action named "+ gesture.name + " is imported to the parent population set panel for IGA")
                               
            if action_exists == 0:
                print("An action named " + action_list[i] + " doesn't exist. Make sure it is imported!")
        print(self.initial_population)
        return self.initial_population   
    
    # Import population set to the IGA panel  
    def importPopulation_IGA(self, name, repeat=1, speed=1, magnitude=1.0):
        bpy.evaAnimationManager.newGesture_IGA(name= name, \
            repeat=repeat, speed=speed, magnitude=magnitude)
        return 0      



    def select_Facialparts(self):#, selected):

        action_list = ['bool_action1', 'bool_action2', 'bool_action3', 'bool_action4', 'bool_action5', 'bool_action6', 'bool_action7', 'bool_action8', 'bool_action9', 'bool_action10', 'bool_action11', 'bool_action12', 'bool_action13', 'bool_action14', 'bool_action15',  'bool_action16', 'bool_action17', 'bool_action18', 'bool_action19', 'bool_action20', 'bool_action21', 'bool_action22', 'bool_action23',  'bool_action24', 'bool_action25', 'bool_action26', 'bool_action27', 'bool_action28', 'bool_action29', 'bool_action30', 'bool_action31', 'bool_action32', 'bool_action33', 'bool_action34', 'bool_action35', 'bool_action36', 'bool_action38', 'bool_action39', 'bool_action40', 'bool_action41', 'bool_action42', 'bool_action43', 'bool_action44', 'bool_action45']

        '''selected = selected.split(',')
        sel_list = []
        for s in range(0, len(selected)):
            sel_list.append(selected[s][2])'''
        
        eyes = bpy.data.scenes['Scene'].bool_facialpart.bool_eyes
        mouth = bpy.data.scenes['Scene'].bool_facialpart.bool_mouth
        head = bpy.data.scenes['Scene'].bool_facialpart.bool_head


        ticked_boxes = bpy.data.scenes["Scene"].bool_actions.items()
        ticked_actions = sorted(ticked_boxes)

        ticked_actions_list = []
        ticked_boxes = bpy.data.scenes["Scene"].bool_actions.items()
        ticked_actions = sorted(ticked_boxes)
        print(ticked_actions)
        for i in range(0, len(ticked_actions)):
            if ticked_actions[i][1] == 1:
                for j in range(0, len(self.initial_population)):   
                    if ticked_actions[i][0] == action_list[j]:
                         if eyes == True:
                             if self.initial_population[j] + '_eyes' not in self.selected_actions:
                                 self.selected_actions.append(self.initial_population[j] +'_'+'eyes') 
                         if mouth == True:
                             if self.initial_population[j] + '_mouth' not in self.selected_actions:
                                 self.selected_actions.append(self.initial_population[j] +'_'+'mouth') 
                         if head == True:
                             if self.initial_population[j] + '_head' not in self.selected_actions:
                                 self.selected_actions.append(self.initial_population[j] +'_'+'head')  
                                 break
        print('selected facial part')
        print(self.selected_actions)
        bpy.data.scenes['Scene'].bool_facialpart.bool_eyes = False
        bpy.data.scenes['Scene'].bool_facialpart.bool_mouth = False
        bpy.data.scenes['Scene'].bool_facialpart.bool_head = False
        '''if len(selected) > 1:
            self.selected_actions = sel_list
            print(self.selected_actions)
            return self.selected_actions
        else:
            return self.selected_actions'''
        return 0

# A hack function: Renaming an action imported into the IGA panel would cause it to be removed from the list (though not from the blender instance).
    #def removeActions_fromIGA(self, ticked_actions):
    def removeActions_fromIGA(self):
        action_list = ['bool_action1', 'bool_action2', 'bool_action3', 'bool_action4', 'bool_action5', 'bool_action6', 'bool_action7', 'bool_action8', 'bool_action9', 'bool_action10', 'bool_action11', 'bool_action12', 'bool_action13', 'bool_action14', 'bool_action15',  'bool_action16', 'bool_action17', 'bool_action18', 'bool_action19', 'bool_action20', 'bool_action21', 'bool_action22', 'bool_action23',  'bool_action24', 'bool_action25', 'bool_action26', 'bool_action27', 'bool_action28', 'bool_action29', 'bool_action30', 'bool_action31', 'bool_action32', 'bool_action33', 'bool_action34', 'bool_action35', 'bool_action36', 'bool_action38', 'bool_action39', 'bool_action40', 'bool_action41', 'bool_action42', 'bool_action43', 'bool_action44', 'bool_action45']
        ticked_actions_list = []
        ticked_boxes = bpy.data.scenes["Scene"].bool_actions.items()
        ticked_actions = sorted(ticked_boxes)
        print (ticked_boxes)
        inc = 0
        for i in range(0, len(ticked_actions)):
            if ticked_actions[i][1] == 1:
                for j in range(0, len(self.initial_population)):
                    if ticked_actions[i][0] == action_list[j]:
                        ticked_actions_list.append(self.initial_population[j]) 
                        break

        print("tickedlist")
        print(ticked_actions_list)  
        parts = ['eyes', 'mouth', 'head']

 
        for i in range(0, len(ticked_actions_list)):   
            print('similarity found')
            for p in range(0, len(parts)):
                for action_index in range(0, len(self.selected_actions)):
                    print(len(self.selected_actions)) 
                    if str(ticked_actions_list[i]) == self.selected_actions[action_index].split('_')[0]:
                        print('similarity found2') 
                        if self.selected_actions[action_index].split('_')[1] == parts[0]:
                            self.selected_actions.remove(self.selected_actions[action_index].split('_')[0]+'_eyes')
                            #self.selected_actions.remove(str(ticked_actions_list[i])+'_eyes')
                            print(str(ticked_actions_list[i]))
                            print('eyes removed')
                            break
                        if self.selected_actions[action_index].split('_')[1] == parts[1]:
                            self.selected_actions.remove(self.selected_actions[action_index].split('_')[0]+'_mouth') 
                            #self.selected_actions.remove(str(ticked_actions_list[i])+'_mouth') 
                            print(str(ticked_actions_list[i]))
                            print('mouth removed')
                            break
                        if self.selected_actions[action_index].split('_')[1] == parts[2]:
                            self.selected_actions.remove(self.selected_actions[action_index].split('_')[0]+'_head')
                            #self.selected_actions.remove(str(ticked_actions_list[i])+'_head')
                            print(str(ticked_actions_list[i]))
                            print('head removed')
                            break         

        print('self.selected_actions')         
        print (self.selected_actions)  
                     
                

# A hack function: Renaming an action imported into the IGA panel would cause it to be removed from the Panel list (though not from the blender instance).

        for i in range(0, len(ticked_actions_list)):
            for j, action in enumerate(bpy.data.actions):
                if str(ticked_actions_list[i]) == action.name:
                    self.initial_population.remove(str(ticked_actions_list[i])) 
                    print("An action named "+ action.name + " is removed from the IGA panel by renaming it to" + "removed-" + action.name )
                    action.name = 'removed-' + action.name  
                    break
        return 0 

# A function to add/save a list of chosen actions into the current blend file. But the .blend file needs to be saved before closing in order to make the save permanent.
  
    def saveAction_IGA(self):
        
        action_exists = 0
        action_names = bpy.data.scenes['Scene'].action_save_IGA # An example input may look like... IGA-amused : IGA-thoughtful : IGA-nod-1 etc.(put in the list names of evolved actions you want to be saved.)
        action_list = action_names.split(' : ') #converts text input into lists
        print (action_list)
        
        #checked_list = []
        for i in range(0, len(action_list)):
            flag = 0
            action_exists = 0

            for j, gesture in enumerate(bpy.data.actions):  
                if gesture.name == action_list[i] and flag == 0:
                    bpy.data.actions[gesture.name].use_fake_user = True
                    action_exists = 1
                    flag = 1
            if action_exists == 0:
                print("An action named " + action_list[i] + " is not known!")

# A function to rename an action's current name to a new one   
    
    def renameAction(self):
        print(self.initial_population)
        action_exists = 0
        current_name = bpy.data.scenes['Scene'].action_currentname
        new_name = bpy.data.scenes['Scene'].action_newname
        for gesture in bpy.data.actions:
            if gesture.name == current_name:
                bpy.data.actions[gesture.name].name = new_name
                action_exists = 1
                print("An action named"+ gesture.name + "is renamed to" + new_name)
        if action_exists == 0:
            print("An action named" + current_name + "doesn't exit")

        return 0
 # calls and executes the d/t crossover methods defined in the animationManager.py based on the weight and boolean values.           
    #def childGeneration_uniform(self):
    def childGenerator(self):
        
        population  = self.initial_population

        bool_uniform = bpy.data.scenes['Scene'].bool_crossover.bool_uniform
        bool_cutandsplice = bpy.data.scenes['Scene'].bool_crossover.bool_cutandsplice
        bool_blend = bpy.data.scenes['Scene'].bool_crossover.bool_blend 
    
        weight_uniform = bpy.data.scenes['Scene'].weight_uniform
        weight_cutandsplice = bpy.data.scenes['Scene'].weight_cutandsplice
        weight_blend = bpy.data.scenes['Scene'].weight_blend
        weight_mutation = bpy.data.scenes['Scene'].weight_mutation

        popn_uniform = int((len(population)* weight_uniform))
        popn_cutandsplice = int((len(population)* weight_cutandsplice))
        popn_blend = int((len(population)* weight_blend))
       
        popn_range_first = randint(1, 2)
        popn_range_second = 0 
        popn_range_third = 3
        
        if popn_range_first == 1:
            popn_range_second = 2
        else:    
            popn_range_second = 1
        

              ### weight based expression generation logic ###

        # uniform, cut&splice and blend crossovers selected
        if bool_uniform  == True and bool_cutandsplice == True and bool_blend == True:
            if weight_uniform + weight_cutandsplice + weight_blend > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_uniform(self.initial_population, popn_uniform, popn_range_first, weight_mutation, self.selected_actions)
                weight_mutation=0
                bpy.evaAnimationManager.Generator_cutandsplice(self.initial_population, popn_cutandsplice, popn_range_second, weight_mutation, self.selected_actions)
                bpy.evaAnimationManager.Generator_blend(self.initial_population, popn_blend, popn_range_third, weight_mutation, self.selected_actions)

        # uniform and cut&splice crossovers selected
        if bool_uniform  == True and bool_cutandsplice == True and bool_blend == False:
            if weight_uniform + weight_cutandsplice > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_uniform(self.initial_population, popn_uniform, popn_range_first, weight_mutation, self.selected_actions)
                weight_mutation=0
                bpy.evaAnimationManager.Generator_cutandsplice(self.initial_population, popn_cutandsplice, popn_range_second, weight_mutation, self.selected_actions)

        # uniform and blend crossovers selected
        if bool_uniform  == True and bool_cutandsplice == False and bool_blend == True:
            if weight_uniform + weight_blend > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_uniform(self.initial_population, popn_uniform, popn_range_first, weight_mutation, self.selected_actions)
                weight_mutation=0
                bpy.evaAnimationManager.Generator_blend(self.initial_population, popn_blend, popn_range_first, weight_mutation, self.selected_actions)

        # cut&splice and blend crossovers selected
        if bool_uniform  == False and bool_cutandsplice == True and bool_blend == True:
            if weight_cutandsplice + weight_blend > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_cutandsplice(self.initial_population, popn_cutandsplice, popn_range_first, weight_mutation, self.selected_actions)
                weight_mutation=0
                bpy.evaAnimationManager.Generator_blend(self.initial_population, popn_blend, popn_range_second, weight_mutation, self.selected_actions)
       
        popn_range_full = 4
        # uniform crossover selected
        if bool_uniform  == True and bool_cutandsplice == False and bool_blend == False:
            if weight_uniform > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_uniform(self.initial_population, popn_uniform, popn_range_full, weight_mutation, self.selected_actions)

        # cut&splice crossover selected
        if bool_uniform  == False and bool_cutandsplice == True and bool_blend == False:
            if weight_cutandsplice > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                bpy.evaAnimationManager.Generator_cutandsplice(self.initial_population, popn_cutandsplice, popn_range_full, weight_mutation, self.selected_actions)

        # blend crossover selected
        if bool_uniform  == False and bool_cutandsplice == False and bool_blend == True:
            if weight_blend > 1.001:
                pymsgbox.alert("Total weight value exceeded 1", "Weight Limit")
                #print ("Total weight value exceeded limit of maximum value (1)")
            else:
                
                bpy.evaAnimationManager.Generator_blend(self.initial_population, popn_blend, popn_range_full, weight_mutation, self.selected_actions)

        return 0
        

   
 # calls and executes the cut and splice based GA function defined in the animationManager.py  
    '''def childGeneration_cutandsplice(self):
        population  = self.initial_population
        weight_cutandsplice =  bpy.data.scenes['Scene'].weight_cutandsplice
        bpy.evaAnimationManager.Generator_cutandsplice(population, weight_cutandsplice)
        return 0'''


    def stopGesture(self, gestureID, smoothing):
        ## TODO
        return 0

    # Visemes --------------------------------------
    def availableVisemes(self):
        visemes = []
        for viseme in bpy.data.actions:
            if viseme.name.startswith("VIS-"):
                visemes.append(viseme.name[4:])
        return visemes


    def queueViseme(self, vis, start=0, duration=0.5, \
            rampin=0.1, rampout=0.8, magnitude=1):
        return bpy.evaAnimationManager.newViseme("VIS-"+vis, duration, \
            rampin, rampout, start)

    # Eye look-at targets ==========================
    # The coordinate system used is head-relative, in 'engineering'public_ws/src/blender_api/rigControl/commands.py:135
    # coordinates: 'x' is forward, 'y' to the left, and 'z' up.
    # Distances are measured in meters.  Origin of the coordinate
    # system is somewhere (where?) in the middle of the head.

    def setFaceTarget(self, loc):
        # Eva uses y==forward x==right. Distances in meters from
        # somewhere in the middle of the head.
        mloc = [loc[1], loc[0], loc[2]]
        bpy.evaAnimationManager.setFaceTarget(mloc)
        return 0

    def setGazeTarget(self, loc):
        mloc = [loc[1],  loc[0], loc[2]]
        bpy.evaAnimationManager.setGazeTarget(mloc)
        return 0
    # ========== procedural animations with unique parameters =============
    def setBlinkRandomly(self,interval_mean,interval_variation):
        bpy.evaAnimationManager.setBlinkRandomly(interval_mean,interval_variation)
        return 0

    def setSaccade(self,interval_mean,interval_variation,paint_scale,eye_size,eye_distance,mouth_width,mouth_height,weight_eyes,weight_mouth):
        bpy.evaAnimationManager.setSaccade(interval_mean,interval_variation,paint_scale,eye_size,eye_distance,mouth_width,mouth_height,weight_eyes,weight_mouth)
        return 0

    # ========== info dump for ROS, Should return non-blender data structures

    # Gets Head rotation quaternion in XYZ format in blender independamt
    # data structure.
    # Pitch: X (positive down, negative up)?
    # Yaw: Z (negative right to positive left)
    #
    # The bones['DEF-head'].id_data.matrix_world currently return the
    # unit matrix, and so are not really needed.

    def getHeadData(self):
        bones = bpy.evaAnimationManager.deformObj.pose.bones
        rhead = bones['DEF-head'].matrix * Matrix.Rotation(-pi/2, 4, 'X')
        rneck = bones['DEF-neck'].matrix * Matrix.Rotation(-pi/2, 4, 'X')
        rneck.invert()

        # I think this is the correct order for the neck rotations.
        q = (rneck * rhead).to_quaternion()
        # q = (rhead * rneck).to_quaternion()
        return {'x':q.x, 'y':q.y, 'z':q.z, 'w':q.w}

    # Same as head, but for the lower neck joint.
    def getNeckData(self):
        bones = bpy.evaAnimationManager.deformObj.pose.bones
        rneck = bones['DEF-neck'].matrix * Matrix.Rotation(-pi/2, 4, 'X')
        q = rneck.to_quaternion()
        return {'x':q.x, 'y':q.y, 'z':q.z, 'w':q.w}

    # Gets Eye rotation angles:
    # Pitch: down(negative) to up(positive)
    # Yaw: left (negative) to right(positive)

    def getEyesData(self):
        bones = bpy.evaAnimationManager.deformObj.pose.bones
        head = (bones['DEF-head'].id_data.matrix_world*bones['DEF-head'].matrix*Matrix.Rotation(-pi/2, 4, 'X')).to_euler()
        leye = bones['eye.L'].matrix.to_euler()
        reye = bones['eye.R'].matrix.to_euler()
        # Relative to head. Head angles are inversed.
        leye_p = leye.x + head.x
        leye_y = pi - leye.z if leye.z >= 0 else -(pi+leye.z)
        reye_p = reye.x + head.x
        reye_y = pi - reye.z if reye.z >= 0 else -(pi+reye.z)
        # Add head target
        leye_y += head.z
        reye_y += head.z
        return {'l':{'p':leye_p,'y':leye_y},'r':{'p':reye_p,'y':reye_y}}


    def getFaceData(self):
        shapekeys = OrderedDict()
        for shapekeyGroup in bpy.data.shape_keys:
            # Hardcoded to find the correct group
            if shapekeyGroup.name == 'ShapeKeys':
                for kb in shapekeyGroup.key_blocks:
                    shapekeys[kb.name] = kb.value

        # Fake the jaw shapekey from its z coordinate
        jawz = bpy.evaAnimationManager.deformObj.pose.bones['chin'].location[2]
        shapekeys['jaw'] = min(max(jawz*7.142, 0), 1)

        return shapekeys


    def setNeckRotation(self, pitch, roll):
        bpy.evaAnimationManager.deformObj.pose.bones['DEF-neck'].rotation_euler = Euler((pitch, 0, roll))

    def setParam(self, key, value):
        cmd = "%s=%s" % (str(key), str(value))
        logger.info("Run %s" % cmd)
        try:
            exec(cmd)
        except Exception as ex:
            logger.error("Error %s" % ex)
            return False
        return True

    def getParam(self, param):
        param = param.strip()
        logger.info("Get %s" % param)
        try:
            return str(eval(param))
        except Exception as ex:
            logger.error("Error %s" % ex)

    def getAnimationLength(self, animation):
        animation = "GST-"+animation
        if not animation in bpy.data.actions.keys():
            return 0
        else:
            frame_range = bpy.data.actions[animation].frame_range
            frames = 1+frame_range[1]-frame_range[0]
            return frames / bpy.context.scene.render.fps



```

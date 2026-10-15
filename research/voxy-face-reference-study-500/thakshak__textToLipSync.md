# Facial rendering source scan: thakshak/textToLipSync

Repository: https://github.com/thakshak/textToLipSync

Pinned source revision: 3828550cbb437ff3cb9d2af663dbd7448c963292

Discovery topic: facial rig

Description: animates a facial rig to show as if its speaking based on the entered text in Maya.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"lip": ["maya/scripts/lipsync/__init__.py", "maya/scripts/lipsync/controller.py", "maya/scripts/lipsync/dictionary.py", "maya/scripts/lipsync/phonemes.py"]}

## Source file: maya/scripts/lipsync/__init__.py

{
  "path": "maya/scripts/lipsync/__init__.py",
  "blob_sha": "5f7975ccfaebfe40344890d837c15164b2ab2cfe",
  "sha256": "1b53c64559451716736ce94b486bcbc8d7edc0d1a848a48f4c5e9bd37ae7f464",
  "lines": 1,
  "bytes": 58,
  "selection_score": 3,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/thakshak/textToLipSync/blob/3828550cbb437ff3cb9d2af663dbd7448c963292/maya/scripts/lipsync/__init__.py",
  "function_signature_candidates": []
}

```text

# This file makes the lipsync directory a Python package.


```

## Source file: maya/scripts/lipsync/phonemes.py

{
  "path": "maya/scripts/lipsync/phonemes.py",
  "blob_sha": "f18def9f3defaa11da9600c27983ab65c6ee1074",
  "sha256": "6240568e3ec4017cf61fb1554357065ca4a369352cee38b1d888a0a4e9065707",
  "lines": 45,
  "bytes": 3111,
  "selection_score": 3,
  "keyword_line_counts": {
    "jaw": 16,
    "mouth": 4,
    "tongue": 3,
    "rig": 11,
    "lip": 11
  },
  "url": "https://github.com/thakshak/textToLipSync/blob/3828550cbb437ff3cb9d2af663dbd7448c963292/maya/scripts/lipsync/phonemes.py",
  "function_signature_candidates": []
}

```text

"""
This module contains the mapping from phonemes to facial expression data.
"""

PHONEME_MAP = {
    'AA': [('lipRightCorner2', 'translateX', 0.7), ('lipLeftCorner2', 'translateX', 0.7), ('jaw', 'translateY', -0.7)],
    'AE': [('jaw', 'translateY', -0.5)],
    'AH': [('jaw', 'translateY', -0.4)],
    'AO': 'AA',
    'AW': ['AO', 'OW'],
    'AY': ['AH', 'Y'],
    'B': [('lipRightCorner2', 'translateX', 0.1), ('lipLeftCorner2', 'translateX', 0.1), ('stickLipsLeft', 'translateY', 1.0), ('stickLipsRight', 'translateY', 1.0), ('jaw', 'translateY', -0.1)],
    'CH': [('lipRightCorner2', 'translateX', 0.3), ('lipLeftCorner2', 'translateX', 0.3), ('upperLip4', 'translateY', 0.2), ('lowerLip4', 'translateY', 0.1), ('upperLip2', 'translateY', 0.2), ('lowerLip2', 'translateY', 0.1), ('upperLipInOutLeft', 'translateY', 1.0), ('lowerLipInOutLeft', 'translateY', 1.0), ('upperLipInOutRight', 'translateY', 1.0), ('lowerLipInOutRight', 'translateY', 1.0), ('mouthInOut', 'translateY', 1.0)],
    'D': [('tongue_rotate', (0, 0, 0, 25)), ('jaw', 'translateY', -0.1)],
    'DH': [('jaw', 'translateY', -0.2), ('tongue_translate', ('translateX', 0.250)), ('tongue_translate', ('translateY', 0.600))],
    'EH': [('jaw', 'translateY', -0.3)],
    'ER': [('lipRightCorner2', 'translateX', 0.2), ('lipLeftCorner2', 'translateX', 0.2), ('upperLipInOutLeft', 'translateY', 1.0), ('lowerLipInOutLeft', 'translateY', 1.0), ('upperLipInOutRight', 'translateY', 1.0), ('lowerLipInOutRight', 'translateY', 1.0), ('mouthInOut', 'translateY', 1.0), ('jaw', 'translateY', -0.1)],
    'EY': 'AE',
    'F': [('lowerLipInOutLeft', 'translateY', -0.6), ('lowerLipInOutRight', 'translateY', -0.6), ('jaw', 'translateY', -0.15)],
    'G': [('jaw', 'translateY', -0.1)],
    'HH': [('lowerLipInOutLeft', 'translateY', 0.3), ('lowerLipInOutRight', 'translateY', 0.3), ('jaw', 'translateY', -0.1)],
    'IH': 'HH',
    'IY': [('tongue_rotate', (0,0,0,0)), ('lipRightCorner2', 'translateX', -0.2), ('lipLeftCorner2', 'translateX', -0.2), ('lowerLipInOutLeft', 'translateY', 1.0), ('lowerLipInOutRight', 'translateY', 1.0), ('jaw', 'translateY', -0.1)],
    'JH': 'CH',
    'K': 'G',
    'L': 'D',
    'M': 'B',
    'N': 'D',
    'NG': ['G', 'N'],
    'OW': [('lipRightCorner2', 'translateX', 0.9), ('lipLeftCorner2', 'translateX', 0.9), ('jaw', 'translateY', -0.5), ('mouthInOut', 'translateY', 0.25), ('upperLipInOutLeft', 'translateY', 1.0), ('lowerLipInOutLeft', 'translateY', 1.0), ('upperLipInOutRight', 'translateY', 1.0), ('lowerLipInOutRight', 'translateY', 1.0)],
    'OY': ['AO', 'Y'],
    'P': 'B',
    'R': 'D',
    'S': [('lipRightCorner2', 'translateX', -0.3), ('lipLeftCorner2', 'translateX', -0.3), ('jaw', 'translateY', -0.1)],
    'SH': 'CH',
    'T': 'D',
    'TH': 'DH',
    'UH': [('lipRightCorner2', 'translateX', 1), ('lipLeftCorner2', 'translateX', 1), ('mouthInOut', 'translateY', 0.1), ('jaw', 'translateY', -0.3)],
    'UW': 'UH',
    'V': 'F',
    'W': 'F',
    'Y': [('lipRightCorner2', 'translateX', -0.1), ('lipLeftCorner2', 'translateX', -0.1), ('jaw', 'translateY', -0.1)],
    'Z': 'S',
    'ZH': 'CH'
}


```

## Source file: maya/scripts/lipsync/controller.py

{
  "path": "maya/scripts/lipsync/controller.py",
  "blob_sha": "7a6db9aa9b938c94404136a1b7630bf114128809",
  "sha256": "164d66e4e4e8760c666fc3eeb6bd6095199137148a384840f1ed9f47080b7ba1",
  "lines": 201,
  "bytes": 9223,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 4,
    "jaw": 1,
    "mouth": 2,
    "tongue": 20,
    "rig": 9,
    "lip": 22
  },
  "url": "https://github.com/thakshak/textToLipSync/blob/3828550cbb437ff3cb9d2af663dbd7448c963292/maya/scripts/lipsync/controller.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self):"
    },
    {
      "line": 44,
      "text": "def _set_keyframe(self, control, attr, value, frame):"
    },
    {
      "line": 52,
      "text": "def _set_tongue_rotation_keyframe(self, values, frame):"
    },
    {
      "line": 59,
      "text": "def _set_tongue_translation_keyframe(self, attr, value, frame):"
    },
    {
      "line": 64,
      "text": "def reset_pose(self):"
    },
    {
      "line": 82,
      "text": "def set_idle_keyframe(self, frame):"
    },
    {
      "line": 95,
      "text": "def set_ready_keyframe(self, frame):"
    },
    {
      "line": 113,
      "text": "def express_phoneme(self, phoneme, frame, stress):"
    },
    {
      "line": 148,
      "text": "def animate_text(self, text):"
    },
    {
      "line": 185,
      "text": "def run_interactive_session(self):"
    }
  ]
}

```text

import maya.cmds as cmds
import os
from .phonemes import PHONEME_MAP
from .dictionary import load_phenome_dict

class LipSyncController:
    """
    Controls a facial rig in Maya to animate lip sync from text.
    """
    FRAME_DURATION = 8

    def __init__(self):
        self.controls = {
            'leftEyeLid': 'Mery_ac_lf_supfLid_move_sg',
            'rightEyeLid': 'Mery_ac_rg_supfLid_move_sg',
            'mouthInOut': 'Mery_ac_cn_inout_mouth',
            'jaw': 'Mery_ac_cn_jaw_control',
            'lipLeftCorner1': 'Mery_ac_lf_tinyCorner_control',
            'lipRightCorner1': 'Mery_ac_rg_tinyCorner_control',
            'lipLeftCorner2': 'Mery_ac_lf_corner_control',
            'lipRightCorner2': 'Mery_ac_rg_corner_control',
            'upperLip1': 'Mery_ac_upLip_01_control',
            'upperLip2': 'Mery_ac_upLip_02_control',
            'upperLip3': 'Mery_ac_upLip_03_control',
            'upperLip4': 'Mery_ac_upLip_04_control',
            'upperLip5': 'Mery_ac_upLip_05_control',
            'lowerLip1': 'Mery_ac_loLip_01_control',
            'lowerLip2': 'Mery_ac_loLip_02_control',
            'lowerLip3': 'Mery_ac_loLip_03_control',
            'lowerLip4': 'Mery_ac_loLip_04_control',
            'lowerLip5': 'Mery_ac_loLip_05_control',
            'stickLipsLeft': 'Mery_ac_lf_stickyLips_control',
            'stickLipsRight': 'Mery_ac_rg_stickyLips_control',
            'upperLipInOutLeft': 'Mery_ac_up_lf_lip_inout',
            'upperLipInOutRight': 'Mery_ac_up_rg_lip_inout',
            'lowerLipInOutLeft': 'Mery_ac_dw_lf_lip_inout',
            'lowerLipInOutRight': 'Mery_ac_dw_rg_lip_inout',
            'mouthMove': 'Mery_ac_cn_mouth_move',
            'tongue': ['Mery_ac_cn_tongue1', 'Mery_ac_cn_tongue2', 'Mery_ac_cn_tongue3', 'Mery_ac_cn_tongue4']
        }
        self.phoneme_map = PHONEME_MAP
        self.phenome_dict = load_phenome_dict()

    def _set_keyframe(self, control, attr, value, frame):
        """Sets a keyframe on a control attribute if it exists."""
        full_attr = "{}.{}".format(control, attr)
        if cmds.objExists(control) and cmds.attributeQuery(attr, node=control, exists=True):
            cmds.setKeyframe(full_attr, value=value, time=frame)
        else:
            cmds.warning("Attribute not found, skipping keyframe: {}".format(full_attr))

    def _set_tongue_rotation_keyframe(self, values, frame):
        """Sets keyframes for the tongue control rotations."""
        tongue_controls = self.controls['tongue']
        for i, value in enumerate(values):
            if i < len(tongue_controls):
                self._set_keyframe(tongue_controls[i], 'rotateZ', value, frame)

    def _set_tongue_translation_keyframe(self, attr, value, frame):
        """Sets a keyframe for the base of the tongue's translation."""
        tongue_base = self.controls['tongue'][0]
        self._set_keyframe(tongue_base, attr, value, frame)

    def reset_pose(self):
        """Resets all animated parts to their default positions."""
        for name, control in self.controls.items():
            if name == 'tongue':
                if cmds.objExists(control[0]):
                    cmds.setAttr(control[0] + '.translateX', 0.231)
                    cmds.setAttr(control[0] + '.translateY', 0.563)
                    cmds.setAttr(control[0] + '.translateZ', 0.014)
                for i in range(4):
                    if cmds.objExists(control[i]) and cmds.attributeQuery('rotateZ', node=control[i], exists=True):
                        cmds.setAttr(control[i] + '.rotateZ', 0)
            else:
                if cmds.objExists(control):
                    if cmds.attributeQuery('translateX', node=control, exists=True):
                        cmds.setAttr(control + '.translateX', 0)
                    if cmds.attributeQuery('translateY', node=control, exists=True):
                        cmds.setAttr(control + '.translateY', 0)

    def set_idle_keyframe(self, frame):
        """Sets a keyframe for the idle (neutral) pose."""
        for name, control in self.controls.items():
            if name == 'tongue':
                self._set_keyframe(control[0], 'translateX', 0.231, frame)
                self._set_keyframe(control[0], 'translateY', 0.563, frame)
                self._set_keyframe(control[0], 'translateZ', 0.014, frame)
                self._set_tongue_rotation_keyframe((0, 0, 0, 0), frame)
            else:
                self._set_keyframe(control, 'translateX', 0.0, frame)
                self._set_keyframe(control, 'translateY', 0.0, frame)
        return frame + self.FRAME_DURATION

    def set_ready_keyframe(self, frame):
        """Sets a keyframe for the current pose to transition from."""
        for name, control_name in self.controls.items():
            if name == 'tongue':
                t_controls = self.controls['tongue']
                if cmds.objExists(t_controls[0]):
                    for attr in ['translateX', 'translateY', 'translateZ']:
                        val = cmds.getAttr("{}.{}".format(t_controls[0], attr))
                        self._set_keyframe(t_controls[0], attr, val, frame)
                    rotations = [cmds.getAttr("{}.rotateZ".format(c)) for c in t_controls if cmds.objExists(c)]
                    self._set_tongue_rotation_keyframe(rotations, frame)
            else:
                if cmds.objExists(control_name):
                    translate = cmds.getAttr("{}.translate".format(control_name))[0]
                    self._set_keyframe(control_name, 'translateX', translate[0], frame)
                    self._set_keyframe(control_name, 'translateY', translate[1], frame)
        return frame

    def express_phoneme(self, phoneme, frame, stress):
        """Applies the facial expression for a given phoneme."""
        self.set_ready_keyframe(frame)

        phoneme_code = phoneme.strip('0123456789')
        expression_data = self.phoneme_map.get(phoneme_code)

        if not expression_data:
            cmds.warning("Phoneme '{}' not found in map.".format(phoneme_code))
            return frame + self.FRAME_DURATION - stress

        if isinstance(expression_data, str):
            expression_data = self.phoneme_map.get(expression_data, [])
        elif isinstance(expression_data, list) and expression_data and isinstance(expression_data[0], str):
            next_frame = frame
            for ph_alias in expression_data:
                next_frame = self.express_phoneme(ph_alias, next_frame, stress)
            return next_frame

        for data in expression_data:
            control_name, *rest = data

            if control_name == 'tongue_rotate':
                self._set_tongue_rotation_keyframe(rest[0], frame)
            elif control_name == 'tongue_translate':
                attr, value = rest[0]
                self._set_tongue_translation_keyframe(attr, value, frame)
            else:
                attr, value = rest
                control = self.controls.get(control_name)
                if control:
                    self._set_keyframe(control, attr, value, frame)

        return frame + self.FRAME_DURATION - stress

    def animate_text(self, text):
        """Animates the rig based on the input text."""
        if not self.phenome_dict:
            cmds.error("CMU dictionary not loaded. Cannot animate.")
            return

        words = text.upper().split()
        phonemes = []
        for word in words:
            if word in self.phenome_dict:
                phonemes.extend(self.phenome_dict[word])
            else:
                cmds.warning("Word '{}' not found in dictionary.".format(word))

        if not phonemes:
            cmds.warning("No phonemes found for the given text.")
            return

        self.reset_pose()
        frame = 0
        frame = self.set_idle_keyframe(frame)

        for ph in phonemes:
            print("Expressing: {}".format(ph))
            tmp = ph[-1:]
            stress_val = int(tmp) if tmp in ['0', '1', '2'] else 0
            stress_factor = 2 * (3 - stress_val)
            frame = self.express_phoneme(ph, frame, stress_factor)

        frame = self.set_idle_keyframe(frame)

        cmds.playbackOptions(animationStartTime=0, animationEndTime=frame, playbackSpeed=12)
        cmds.playblast(startTime=0, endTime=frame, format="image", compression="gif")
        if cmds.ls(type='animCurve'):
            cmds.cutKey(cmds.ls(type='animCurve'), time=(0, frame))
        print("Animation complete.")

    def run_interactive_session(self):
        """Starts an interactive session to animate text from user input."""
        cmds.setAttr(self.controls['leftEyeLid'] + '.translateY', 0.8)
        cmds.setAttr(self.controls['rightEyeLid'] + '.translateY', 0.8)

        while True:
            try:
                text_input = raw_input("Enter text to animate (or 'quit' to exit): ")
                if text_input.lower() == 'quit':
                    break
                if text_input:
                    self.animate_text(text_input)
            except EOFError:
                break
            except Exception as e:
                print("An error occurred: {}".format(e))
                break


```

## Source file: maya/scripts/lipsync/dictionary.py

{
  "path": "maya/scripts/lipsync/dictionary.py",
  "blob_sha": "c15d2083daca5684e73ab6a2535397c42c42db0c",
  "sha256": "5994a618b1206b7a53edbab0ecf1a6127bfff2200e70194949605dc7e10bcdeb",
  "lines": 27,
  "bytes": 953,
  "selection_score": 3,
  "keyword_line_counts": {
    "lip": 2
  },
  "url": "https://github.com/thakshak/textToLipSync/blob/3828550cbb437ff3cb9d2af663dbd7448c963292/maya/scripts/lipsync/dictionary.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def load_phenome_dict():"
    }
  ]
}

```text

"""
This module handles loading the CMU pronouncing dictionary.
"""
import os
import maya.cmds as cmds

def load_phenome_dict():
    """Loads the CMU pronouncing dictionary."""
    # This script is in maya/scripts/lipsync, so we need to go up two levels
    # to get to the maya/scripts directory where cmudict is located.
    base_dir = os.path.dirname(os.path.abspath(__file__))
    # base_dir is lipsync, parent is scripts.
    scripts_dir = os.path.dirname(base_dir)
    dict_path = os.path.join(scripts_dir, 'cmudict', 'cmudict-0.7b.txt')

    lines = {}
    try:
        with open(dict_path, "r") as f:
            for line in f:
                if not line.startswith(";;;"):
                    parts = line.strip().split()
                    if len(parts) > 1:
                        lines[parts[0]] = parts[1:]
    except IOError as e:
        cmds.warning("Could not load CMU dictionary: {}".format(e))
        return None
    return lines


```

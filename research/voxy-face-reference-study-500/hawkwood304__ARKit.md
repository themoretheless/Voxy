# Facial rendering source scan: hawkwood304/ARKit

Repository: https://github.com/hawkwood304/ARKit

Pinned source revision: 089d6dd54729b637f4e017002bde8929ebe5f955

Discovery topic: ARKit blendshape

Description: This is a combine of all the customize script I make for the work around ARKit blendshape.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["BlendShapeByTime.py", "BlendShapeControl.py"]}

## Source file: BlendShapeByTime.py

{
  "path": "BlendShapeByTime.py",
  "blob_sha": "304f44c89f8df1de49cb46faf71cd0d1048d63b7",
  "sha256": "0bea1e17e12314765b600a1185f32f429a61ec5649eb86155b448765ca357b71",
  "lines": 389,
  "bytes": 15339,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 1,
    "blendshape": 15,
    "jaw": 2,
    "mouth": 6,
    "tongue": 1,
    "rig": 9
  },
  "url": "https://github.com/hawkwood304/ARKit/blob/089d6dd54729b637f4e017002bde8929ebe5f955/BlendShapeByTime.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def __init__(self):"
    },
    {
      "line": 30,
      "text": "def create_widgets(self):"
    },
    {
      "line": 47,
      "text": "def create_layouts(self):"
    },
    {
      "line": 64,
      "text": "def create_connections(self):"
    },
    {
      "line": 70,
      "text": "def execute_mel_command(command):"
    },
    {
      "line": 77,
      "text": "def fbx_export_option(self, path):"
    },
    {
      "line": 112,
      "text": "def show_file_select_dialog(self):"
    },
    {
      "line": 117,
      "text": "def get_convert_file_path(self):"
    },
    {
      "line": 145,
      "text": "def assign_mesh_button(self):"
    },
    {
      "line": 186,
      "text": "def get_list_mesh_name(self):"
    },
    {
      "line": 212,
      "text": "def generate_bs_by_time(self, mesh):"
    },
    {
      "line": 293,
      "text": "def export_mesh(self, group_name, export_dir):"
    },
    {
      "line": 307,
      "text": "def execution_button(self):"
    },
    {
      "line": 331,
      "text": "def display(cls):"
    },
    {
      "line": 343,
      "text": "def maya_main_window(cls):"
    },
    {
      "line": 352,
      "text": "def __init__(self):"
    },
    {
      "line": 366,
      "text": "def create_widget(self):"
    },
    {
      "line": 372,
      "text": "def create_layouts(self):"
    },
    {
      "line": 377,
      "text": "def create_connections(self):"
    },
    {
      "line": 380,
      "text": "def showEvent(self, e):"
    },
    {
      "line": 386,
      "text": "def closeEvent(self, e):"
    }
  ]
}

```text

from shiboken2 import wrapInstance

import os
import maya.cmds as cm
import pymel.core as pm
import maya.OpenMaya as oMaya
import maya.OpenMayaUI as oMayaUI

from PySide2 import QtWidgets, QtCore, QtGui
from maya.mel import eval

import logging

logger = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO)


# import sys

# noinspection PyAttributeOutsideInit
class BlendShapeByTime(QtWidgets.QWidget):

    def __init__(self):
        super(BlendShapeByTime, self).__init__()

        self.create_widgets()
        self.create_layouts()
        self.create_connections()

    def create_widgets(self):
        self.file_path_lb = QtWidgets.QLabel("File path: ")
        self.file_path_le = QtWidgets.QLineEdit()

        self.select_file_path_btn = QtWidgets.QPushButton('')
        self.select_file_path_btn.setIcon(QtGui.QIcon(':fileOpen.png'))
        self.select_file_path_btn.setToolTip('Select File')

        self.mesh_name_lb = QtWidgets.QLabel("Mesh name:")
        self.mesh_name_le = QtWidgets.QLineEdit()
        self.mesh_name_btn = QtWidgets.QPushButton("Assign")

        self.execution_btn = QtWidgets.QPushButton("Execution")
        self.execution_btn.setStyleSheet(
            'QPushButton {background-color: lightyellow; color: black;}'
        )

    def create_layouts(self):
        first_layout = QtWidgets.QGridLayout()
        first_layout.addWidget(self.file_path_lb, 0, 0, 1, 1)
        first_layout.addWidget(self.file_path_le, 0, 1, 1, 5)
        first_layout.addWidget(self.select_file_path_btn, 0, 6, 1, 1)

        first_layout.addWidget(self.mesh_name_lb, 1, 0, 1, 1)
        first_layout.addWidget(self.mesh_name_le, 1, 1, 1, 5)
        first_layout.addWidget(self.mesh_name_btn, 1, 6, 1, 1)

        second_layout = QtWidgets.QHBoxLayout()
        second_layout.addWidget(self.execution_btn)

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(first_layout)
        main_layout.addLayout(second_layout)

    def create_connections(self):
        self.select_file_path_btn.clicked.connect(self.show_file_select_dialog)
        self.mesh_name_btn.clicked.connect(self.assign_mesh_button)
        self.execution_btn.clicked.connect(self.execution_button)

    @staticmethod
    def execute_mel_command(command):
        try:
            eval(command)
            # logger.info(f"MEL command executed: {command}")
        except RuntimeError as e:
            logger.error(f"Failed to execute MEL command: {command} - {e}")

    def fbx_export_option(self, path):

        self.execute_mel_command("FBXExportSmoothingGroups -v true")
        self.execute_mel_command("FBXExportHardEdges -v false")
        self.execute_mel_command("FBXExportTangents -v false")
        self.execute_mel_command("FBXExportSmoothMesh -v true")
        self.execute_mel_command("FBXExportInstances -v false")
        self.execute_mel_command("FBXExportReferencedAssetsContent -v false")

        self.execute_mel_command('FBXExportBakeComplexAnimation -v false')

        self.execute_mel_command("FBXExportUseSceneName -v false")
        self.execute_mel_command("FBXExportQuaternion -v euler")
        self.execute_mel_command("FBXExportShapes -v true")
        self.execute_mel_command("FBXExportSkins -v true")

        # Constraints
        self.execute_mel_command("FBXExportConstraints -v false")
        # Cameras
        self.execute_mel_command("FBXExportCameras -v true")
        # Lights
        self.execute_mel_command("FBXExportLights -v true")
        # Embed Media
        self.execute_mel_command("FBXExportEmbeddedTextures -v false")
        # Connections
        self.execute_mel_command("FBXExportInputConnections -v true")
        # Axis Conversion
        self.execute_mel_command("FBXExportUpAxis y")
        # Version
        self.execute_mel_command('FBXExportFileVersion -v FBX201900')

        # Export!

        self.execute_mel_command('FBXExport -f "{0}" -s'.format(path))

    def show_file_select_dialog(self):
        self.file_path = QtWidgets.QFileDialog.getExistingDirectory(self, 'Select directory')

        self.file_path_le.setText(self.file_path)

    def get_convert_file_path(self):
        """
        Converts the input file path to a normalized format and validates its existence.

        Returns:
            str: The converted file path with forward slashes if it exists.
            None: If the file path doesn't exist, displays an error and returns None.
        """
        # Get the raw file path from the input field
        raw_path = self.file_path_le.text().strip()

        if raw_path:
            # Normalize the file path (replace backslashes with forward slashes)
            normalized_path = raw_path.replace("\\", "/")

            # Check if the directory exists
            if os.path.isdir(normalized_path):
                logger.info(f"Valid file path: {normalized_path}")
                return normalized_path
            else:
                logger.error(f"File path does not exist: {normalized_path}")
                oMaya.MGlobal.displayError("File path doesn't exist. Please select a valid directory.")
                return None
        else:
            logger.warning("File path input is empty.")
            oMaya.MGlobal.displayError("File path cannot be empty.")
            return None

    def assign_mesh_button(self):
        """
        Assigns selected cameras from the Maya scene to the tool's camera name field.

        This function checks for selected objects in the Maya scene and filters out any non-camera objects.
        If no cameras are found or if nothing is selected, appropriate errors are logged and displayed.
        """
        try:
            # Get the currently selected objects in the Maya scene
            selected_objects = cm.ls(selection=True)
            if not selected_objects:
                oMaya.MGlobal.displayError("Please select at least one camera.")
                return

            # Filter out cameras from the selected objects
            selected_mesh = []
            for obj in selected_objects:
                # Check if the object or its children are cameras
                obj_children = cm.listRelatives(obj, children=True, fullPath=True) or []
                for child in obj_children or [obj]:
                    if cm.objectType(child) == 'mesh':
                        selected_mesh.append(obj)
                        break  # Only add the first valid camera per object

            if not selected_mesh:
                # Log and display an error if no cameras were found in the selection
                logger.warning("No mesh selected or found in the selected objects.")
                oMaya.MGlobal.displayError("No mesh detected in the selected objects.")
                return

            # Format the list of selected cameras as a comma-separated string
            formatted_mesh = ", ".join(selected_mesh)
            self.mesh_name_le.setText(formatted_mesh)

            # Log the successful assignment of cameras
            logger.info(f"Assigned mesh: {formatted_mesh}")

        except Exception as e:
            logger.error(f"Unexpected error in assign_mesh_button: {e}")
            oMaya.MGlobal.displayError(f"Error occurred while assigning mesh: {e}")

    def get_list_mesh_name(self):
        """
        Fetches and returns a cleaned list of mesh names from the input field.

        Returns:
            list: A list of mesh names if valid input exists.
                  An empty list if the input is empty or invalid.
        """
        # Get raw input from the mesh name field
        raw_input = self.mesh_name_le.text().strip()

        if raw_input:
            # Clean the input by removing unnecessary spaces and splitting by commas
            mesh_names = [name.strip() for name in raw_input.split(",") if name.strip()]

            # Log and return the cleaned list of mesh names
            if mesh_names:
                logger.info(f"mesh names extracted: {mesh_names}")
                return mesh_names
            else:
                logger.warning("Input contains only whitespace or invalid mesh names.")
                return []
        else:
            logger.warning("mesh name input is empty.")
            return []

    def generate_bs_by_time(self, mesh):
        """
        Generates a duplicate of a specified mesh for each frame of a predefined blendshape sequence.
        Each duplicate is named based on the corresponding blendshape name.

        Parameters:
        - mesh (str): The name of the mesh to duplicate.

        Preconditions:
        - A valid mesh name must be provided.
        - Time frames should correspond to blendshapes (sequentially named).

        Steps:
        1. Checks if the mesh exists.
        2. Creates a group for organizing all duplicates if it doesn't already exist.
        3. Iterates through blendshape names, setting time frames and creating duplicates for each.
        """

        # Predefined blendshape list
        blendshape_list = {
            i: name for i, name in enumerate([
                'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
                'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
                'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
                'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
                'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
                'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
                'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
                'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
                'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
                'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
                'noseSneerRight', 'tongueOut'
            ], start=1)
        }

        if not mesh:
            cm.warning("No mesh specified. Please provide a valid mesh name.")
            return

        try:
            # Check if the mesh exists
            if not cm.objExists(mesh):
                cm.warning(f"The specified mesh '{mesh}' does not exist.")
                return

            # Create a group to organize duplicates
            group_name = f"{mesh}_grp"
            if not cm.objExists(group_name):
                cm.group(empty=True, world=True, name=group_name)

            # Iterate through blendshape names and create duplicates
            for frame, blendshape_name in blendshape_list.items():
                # Set current time to the frame index
                cm.currentTime(frame)

                # Duplicate the mesh and rename it based on the blendshape name
                duplicate_name = f"{blendshape_name}"
                new_duplicate = cm.duplicate(mesh, name=duplicate_name, returnRootsOnly=False)

                # Parent the duplicate under the group
                cm.parent(new_duplicate, group_name)
                print(f"Created duplicate for {blendshape_name}: {duplicate_name}")

            print("Blendshape duplicates generated successfully.")

            filepath = self.get_convert_file_path()

            # Create the export directory
            export_dir = os.path.join(filepath).replace(os.sep, '/')
            export_dir = str(export_dir)
            if not os.path.isdir(export_dir):
                os.mkdir(export_dir)

            cm.select(group_name)
            self.export_mesh(group_name, export_dir)

            cm.delete(group_name)

        except Exception as e:
            cm.warning(f"Error in generate_bs_by_time: {e}")

    def export_mesh(self, group_name, export_dir):

        try:
            # Export FBX
            mesh_filename = f"{group_name}.fbx"
            mesh_output_path = os.path.join(export_dir, mesh_filename).replace(os.sep, '/')
            self.fbx_export_option(mesh_output_path)

            logger.info(f"Exported mesh {group_name} to {mesh_output_path}")

        except Exception as e:
            logger.error(f"Failed to export mesh '{group_name}': {e}")
            oMaya.MGlobal.displayError(f"Error exporting mesh '{group_name}': {e}")

    def execution_button(self):
        try:
            # Get configured paths, settings, and time ranges
            mesh = self.get_list_mesh_name()

            if mesh:
                for i in mesh:
                    self.generate_bs_by_time(i)

        except RuntimeError as e:
            logger.error(f"Runtime error encountered during FBX export: {e}")
            oMaya.MGlobal.displayError(f"FBX export failed: {e}")
        except Exception as e:
            logger.error(f"Unexpected error during FBX export: {e}")
            oMaya.MGlobal.displayError(f"Error: {e}")


# noinspection PyMethodMayBeStatic,PyAttributeOutsideInit,PyMethodOverriding
class MainWindow(QtWidgets.QDialog):
    WINDOW_TITLE = "Blend Shape By Time"

    dlg_instance = None

    @classmethod
    def display(cls):
        if not cls.dlg_instance:
            cls.dlg_instance = MainWindow()

        if cls.dlg_instance.isHidden():
            cls.dlg_instance.show()

        else:
            cls.dlg_instance.raise_()
            cls.dlg_instance.activateWindow()

    @classmethod
    def maya_main_window(cls):
        """

        Returns: The Maya main window widget as a Python object

        """
        main_window_ptr = oMayaUI.MQtUtil.mainWindow()
        return wrapInstance(int(main_window_ptr), QtWidgets.QWidget)

    def __init__(self):
        super(MainWindow, self).__init__(self.maya_main_window())

        self.setWindowTitle(self.WINDOW_TITLE)
        self.setWindowFlags(self.windowFlags() ^ QtCore.Qt.WindowContextHelpButtonHint)

        self.geometry = None

        self.setMinimumSize(400, 150)
        self.setMaximumSize(400, 150)
        self.create_widget()
        self.create_layouts()
        self.create_connections()

    def create_widget(self):
        self.content_layout = QtWidgets.QHBoxLayout()
        self.content_layout.addWidget(BlendShapeByTime())

        self.close_btn = QtWidgets.QPushButton("Close")

    def create_layouts(self):

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(self.content_layout)

    def create_connections(self):
        self.close_btn.clicked.connect(self.close)

    def showEvent(self, e):
        super(MainWindow, self).showEvent(e)

        if self.geometry:
            self.restoreGeometry(self.geometry)

    def closeEvent(self, e):
        super(MainWindow, self).closeEvent(e)

        self.geometry = self.saveGeometry()


```

## Source file: BlendShapeControl.py

{
  "path": "BlendShapeControl.py",
  "blob_sha": "79b3795fd3683cbf10a571b5a9b9f50ebdafd5b4",
  "sha256": "756374ba69a9b79e825dcf9aee9d9f4511aac65f37b3f77e71b541d6aff5dc9d",
  "lines": 644,
  "bytes": 30638,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 29,
    "blendshape": 118,
    "jaw": 17,
    "mouth": 51,
    "teeth": 4,
    "tongue": 8,
    "lbs": 1,
    "rig": 86,
    "lip": 2
  },
  "url": "https://github.com/hawkwood304/ARKit/blob/089d6dd54729b637f4e017002bde8929ebe5f955/BlendShapeControl.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def bs52_connect():"
    },
    {
      "line": 92,
      "text": "def connect_bs():"
    },
    {
      "line": 135,
      "text": "def connect_bs_custom(control, bs_node):"
    },
    {
      "line": 170,
      "text": "def connect_bs_arkit():"
    },
    {
      "line": 251,
      "text": "def bs_wrap(bs_node_name, mesh_dup):"
    },
    {
      "line": 311,
      "text": "def generate_bs_by_time(mesh):"
    },
    {
      "line": 380,
      "text": "def connect_bs_to_cube(bs_node, cube):"
    },
    {
      "line": 456,
      "text": "def select_control_m2(name_space=\"\"):"
    },
    {
      "line": 508,
      "text": "def generate_bs_by_attr(mesh, bs_node):"
    },
    {
      "line": 587,
      "text": "def generate_bs_by_attr_from_control(mesh, control_name):"
    }
  ]
}

```text

import maya.cmds as cm
import os


def bs52_connect():
    """
    Connect 52 blendshapes to a control rig using unit conversion nodes.
    """
    # Define blendshape names
    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut'
        ], start=1)
    }

    try:
        selected_mesh = cm.ls(selection=True)
        if not selected_mesh:
            raise RuntimeError("No mesh selected. Please select a mesh to connect blendshapes.")

        # Ensure the correct time unit
        cm.currentUnit(time="ntsc")

        # Load control rig if not already in the scene
        maya_script_folder = cm.internalVar(userScriptDir=True)
        rig_path = os.path.join(maya_script_folder, "ctrlARKitBS_M.ma").replace(os.sep, '/')
        if not cm.objExists("ctrlARKitBS_M"):
            cm.file(rig_path, i=True)

        # Identify the blendShape node for the selected mesh
        blendshape_node = cm.ls(cm.listHistory(selected_mesh), type='blendShape')[0]
        if not blendshape_node:
            raise RuntimeError("No blendshape node found for the selected mesh.")

        # Connect each blendshape
        for index, name_bs in blendshape_list.items():
            # Create and configure unit conversion nodes
            uc_node = cm.createNode("unitConversion", name=f"{name_bs}_UE_uC")
            cm.setAttr(f"{uc_node}.conversionFactor", 10)

            # Handle exceptions for specific blendshape names
            if name_bs in ["mouthFrownLeft", "mouthFrownRight", "cheekSquintLeft", "cheekSquintRight", "mouthClose"]:
                blendshape_attr = name_bs.replace('mouthFrown', 'mouthFrow').replace('cheekSquint', 'cheeSquint')
                if cm.objExists(f"{blendshape_node}.{blendshape_attr}"):
                    cm.connectAttr(f"{blendshape_node}.{blendshape_attr}", f"{uc_node}.input", force=True)
                    cm.connectAttr(f"{uc_node}.output", f"ctrlARKitBS_M.{name_bs}", force=True)
                else:
                    continue  # Skip if the specific attribute does not exist
            else:
                if cm.objExists(f"{blendshape_node}.{name_bs}"):
                    cm.connectAttr(f"{blendshape_node}.{name_bs}", f"{uc_node}.input", force=True)
                    cm.connectAttr(f"{uc_node}.output", f"ctrlARKitBS_M.{name_bs}", force=True)
                else:
                    continue

        # Adjust playback options based on blendshape animation time
        all_keys = sorted(cm.keyframe(f"{blendshape_node}.jawOpen", query=True))
        if all_keys:
            min_time, max_time = round(all_keys[0]), round(all_keys[-1])
            cm.playbackOptions(
                animationStartTime=min_time, animationEndTime=max_time,
                minTime=min_time, maxTime=max_time
            )
        else:
            min_time = cm.playbackOptions(q=True, min=True)
            max_time = cm.playbackOptions(q=True, max=True)

        # Bake results for control rig
        cm.select("ctrlARKitBS_M")
        cm.bakeResults(
            simulation=True, t=(min_time, max_time), sampleBy=1,
            oversamplingRate=1, disableImplicitControl=True, preserveOutsideKeys=True,
            sparseAnimCurveBake=False, removeBakedAttributeFromLayer=False,
            removeBakedAnimFromLayer=False, bakeOnOverrideLayer=False,
            minimizeRotation=True, controlPoints=False, shape=False
        )
        print("Blendshape connections completed successfully.")
    except Exception as e:
        cm.warning(f"Error in bs52_connect: {str(e)}")


def connect_bs():
    """
    Connect blendshape attributes for multiple body parts like head, eyes, and teeth.
    """
    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut'
        ], start=1)
    }

    maya_script_folder = cm.internalVar(userScriptDir=True)
    rig_path = os.path.join(maya_script_folder, "ctrlARKitBS_M.ma").replace(os.sep, '/')

    if not cm.objExists("ctrlARKitBS_M"):
        cm.file(rig_path, i=True)
        if cm.objExists("ctrlBox"):
            cm.parent("ctrlBoxARKitBS_M", "ctrlBox")

    for index, name_bs in blendshape_list.items():
        uc_node = cm.createNode("unitConversion", name=f"{name_bs}_uC")
        cm.setAttr(f"{uc_node}.conversionFactor", 0.1)
        cm.connectAttr(f"ctrlARKitBS_M.{name_bs}", f"{uc_node}.input", force=True)

        for body_part in (
                "headBS", "eyeLBS", "toungeBS", "eyeRBS", "eyeLashBS", "teethBS", "lipBS", "noseBS", "eyeBS",
                "eyebrowBS", "upperteethBS", "lowerteethBS", "mouthinsideBS", "mouthBS"):
            if cm.objExists(body_part):
                if cm.objExists(f"{body_part}.{name_bs}"):
                    cm.connectAttr(f"{uc_node}.output", f"{body_part}.{name_bs}", force=True)
                else:
                    continue


def connect_bs_custom(control, bs_node):
    """
    Connect blendshape attributes for multiple body parts like head, eyes, and teeth.
    """
    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut', 'good', 'angry', 'closedeyes', 'o', 'happy', 'l_eye_close', 'r_eye_close',
            'mischievous', 'AA', 'SS', 'OO', 'UU', 'mischiev_lipsync', 'happy_lypsync', 'mad', 'smile', 'O_cỉrcle',
            'very_angry', 'surprised', 'confident', 'determination'
        ], start=1)
    }

    for index, name_bs in blendshape_list.items():
        uc_node = cm.createNode("unitConversion", name=f"{name_bs}_uC")
        cm.setAttr(f"{uc_node}.conversionFactor", 0.1)
        if cm.objExists(f"{control}.{name_bs}"):
            cm.connectAttr(f"{control}.{name_bs}", f"{uc_node}.input", force=True)

            if cm.objExists(bs_node):
                if cm.objExists(f"{bs_node}.{name_bs}"):
                    cm.connectAttr(f"{uc_node}.output", f"{bs_node}.{name_bs}", force=True)
                else:
                    continue


def connect_bs_arkit():
    """
    Connect ARKit blendshape attributes from a LiveLink control rig to a Maya control rig
    using unit conversion nodes for proper scaling.
    """
    # ARKit / Maya blendshape mapping
    csv_blendshape_list = {
        i: name for i, name in enumerate([
            'EyeBlinkLeft', 'EyeLookDownLeft', 'EyeLookInLeft', 'EyeLookOutLeft', 'EyeLookUpLeft',
            'EyeSquintLeft', 'EyeWideLeft', 'EyeBlinkRight', 'EyeLookDownRight', 'EyeLookInRight',
            'EyeLookOutRight', 'EyeLookUpRight', 'EyeSquintRight', 'EyeWideRight', 'JawForward',
            'JawLeft', 'JawRight', 'JawOpen', 'MouthClose', 'MouthFunnel', 'MouthPucker', 'MouthLeft',
            'MouthRight', 'MouthSmileLeft', 'MouthSmileRight', 'MouthFrownLeft', 'MouthFrownRight',
            'MouthDimpleLeft', 'MouthDimpleRight', 'MouthStretchLeft', 'MouthStretchRight',
            'MouthRollLower', 'MouthRollUpper', 'MouthShrugLower', 'MouthShrugUpper', 'MouthPressLeft',
            'MouthPressRight', 'MouthLowerDownLeft', 'MouthLowerDownRight', 'MouthUpperUpLeft',
            'MouthUpperUpRight', 'BrowDownLeft', 'BrowDownRight', 'BrowInnerUp', 'BrowOuterUpLeft',
            'BrowOuterUpRight', 'CheekPuff', 'CheekSquintLeft', 'CheekSquintRight', 'NoseSneerLeft',
            'NoseSneerRight', 'TongueOut'
        ], start=1)
    }

    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut'
        ], start=1)
    }

    try:
        # Load the Maya control rig if it is not already loaded
        maya_script_folder = cm.internalVar(userScriptDir=True)
        rig_path = os.path.join(maya_script_folder, "ctrlARKitBS_M.ma").replace(os.sep, '/')
        if not cm.objExists("ctrlARKitBS_M"):
            cm.file(rig_path, i=True)

        # Connect ARKit blendshapes to Maya rig using unitConversion nodes
        for i in range(1, 53):
            # Retrieve blendshape names
            name_bs = blendshape_list.get(i)
            csv_bs = csv_blendshape_list.get(i)

            if not name_bs or not csv_bs:
                print(f"Skipping blendshape index {i}: Name not found.")
                continue

            # Create a unitConversion node
            uc_node = cm.createNode("unitConversion", name=f"{csv_bs}_csv_uC")
            cm.setAttr(f"{uc_node}.conversionFactor", 10)

            # Ensure required attributes and connections exist
            if not cm.objExists(f"T1_LiveLink.{csv_bs}"):
                cm.warning(f"LiveLink attribute for {csv_bs} does not exist. Skipping connection.")
                continue

            # Connect ARKit to the Maya control rig
            try:
                cm.connectAttr(
                    f"T1_LiveLink.{csv_bs}", f"{uc_node}.input", force=True
                )
                cm.connectAttr(
                    f"{uc_node}.output", f"ctrlARKitBS_M.{name_bs}", force=True
                )
                print(f"Connected {csv_bs} (ARKit) to {name_bs} (Maya).")
            except RuntimeError as e:
                cm.warning(f"Failed to connect {csv_bs} to {name_bs}: {e}")

        print("All ARKit blendshape connections completed.")
    except Exception as e:
        cm.warning(f"Error in connect_bs_arkit: {e}")


def bs_wrap(bs_node_name, mesh_dup):
    """
    Generates blendshape duplicates for every blendshape weight in the specified blendshape node.

    Steps:
    1. Iterates through all blendshape weights of a blendshape node.
    2. Activates each weight one at a time.
    3. Creates a mesh duplicate corresponding to each activated blendshape.
    4. Names the duplicate meshes after the respective blendshape weights.

    Requirements:
    - A blendshape node (e.g., "headBS") must exist in the scene.
    - A target mesh (e.g., "head_lod2_mesh_AS_new") must also exist.
    """

    try:
        # Create a group to store blendshape mesh in after generate
        if not cm.objExists(f"{bs_node_name}_BS_grp"):
            root_grp = cm.group(name=f"{bs_node_name}_BS_grp", world=True, relative=True, empty=True)
        else:
            root_grp = cm.group(name=f"{bs_node_name}_BS_grp", world=True, relative=True, empty=True)

        # Validate that the blendshape node exists
        if not cm.objExists(bs_node_name):
            cm.warning(f"Blendshape node '{bs_node_name}' does not exist in the scene.")
            return

        # Validate that the target mesh exists
        if not cm.objExists(mesh_dup):
            cm.warning(f"Target mesh '{mesh_dup}' does not exist in the scene.")
            return

        # Retrieve the list of blendshape attributes
        list_blendshapes = cm.listAttr(f"{bs_node_name}.w", multi=True)
        if not list_blendshapes:
            cm.warning(f"No blendshape weights found in the node '{bs_node_name}'.")
            return

        # Generate duplicates for each blendshape
        for bs_name in list_blendshapes:
            # Set the blendshape weight to 1
            cm.setAttr(f"{bs_node_name}.{bs_name}", 1)

            # Duplicate the target mesh and rename it with the blendshape name
            duplicate_name = f"{bs_name}_mesh"
            new_duplicate = cm.duplicate(mesh_dup, name=duplicate_name, rr=True)
            cm.parent(new_duplicate, root_grp)

            # Reset the blendshape weight to 0 after duplication
            cm.setAttr(f"{bs_node_name}.{bs_name}", 0)

            # Feedback
            print(f"Created duplicate for blendshape '{bs_name}': {new_duplicate[0]}")

        print("Blendshape wrapping completed successfully.")

    except Exception as e:
        cm.warning(f"Error in bs_wrap: {e}")


def generate_bs_by_time(mesh):
    """
    Generates a duplicate of a specified mesh for each frame of a predefined blendshape sequence.
    Each duplicate is named based on the corresponding blendshape name.

    Parameters:
    - mesh (str): The name of the mesh to duplicate.

    Preconditions:
    - A valid mesh name must be provided.
    - Time frames should correspond to blendshapes (sequentially named).

    Steps:
    1. Checks if the mesh exists.
    2. Creates a group for organizing all duplicates if it doesn't already exist.
    3. Iterates through blendshape names, setting time frames and creating duplicates for each.
    """

    # Predefined blendshape list
    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut'
        ], start=1)
    }

    if not mesh:
        cm.warning("No mesh specified. Please provide a valid mesh name.")
        return

    try:
        # Check if the mesh exists
        if not cm.objExists(mesh):
            cm.warning(f"The specified mesh '{mesh}' does not exist.")
            return

        # Create a group to organize duplicates
        group_name = f"{mesh}_grp"
        if not cm.objExists(group_name):
            cm.group(empty=True, world=True, name=group_name)

        # Iterate through blendshape names and create duplicates
        for frame, blendshape_name in blendshape_list.items():
            # Set current time to the frame index
            cm.currentTime(frame)

            # Duplicate the mesh and rename it based on the blendshape name
            duplicate_name = f"{blendshape_name}"
            new_duplicate = cm.duplicate(mesh, name=duplicate_name, returnRootsOnly=False)

            # Parent the duplicate under the group
            cm.parent(new_duplicate, group_name)
            print(f"Created duplicate for {blendshape_name}: {duplicate_name}")

        print("Blendshape duplicates generated successfully.")

    except Exception as e:
        cm.warning(f"Error in generate_bs_by_time: {e}")


def connect_bs_to_cube(bs_node, cube):
    """
    Connects blendshape attributes from an existing blendshape node (e.g., "headBS")
    to a "cube" (or a control object) using unit conversion nodes. This allows scaling
    or modifying the output of the blendshape weights before passing them to the cube.

    The cube acts as a visual controller for blendshape weights.

    Requirements:
    - A valid blendshape node and a cube object must exist in the Maya scene.
    - The blendshape node attributes must be connected to manipulable cube attributes.
    """
    # Predefined blendshape list
    blendshape_list = {
        i: name for i, name in enumerate([
            'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
            'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
            'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
            'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
            'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
            'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
            'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
            'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
            'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
            'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
            'noseSneerRight', 'tongueOut'
        ], start=1)
    }

    # Inputs
    blendshape_node = bs_node  # The blendshape node
    cube = cube  # The cube object to connect to

    try:
        # Validate that the blendshape node exists
        if not cm.objExists(blendshape_node):
            cm.warning(f"Blendshape node '{blendshape_node}' does not exist in the scene.")
            return

        # Validate that the cube exists
        if not cm.objExists(cube):
            cm.warning(f"Specified cube '{cube}' does not exist in the scene.")
            return

        # Connect each blendshape attribute to the cube
        for index, blendshape_name in blendshape_list.items():
            # Check if the blendshape attribute exists in the blendshape node
            if not cm.objExists(f"{blendshape_node}.{blendshape_name}"):
                cm.warning(
                    f"Blendshape attribute '{blendshape_name}' does not exist on '{blendshape_node}'. Skipping...")
                continue

            # Check if the corresponding attribute exists on the cube
            if not cm.objExists(f"{cube}.{blendshape_name}"):
                # Add the attribute if it doesn't exist
                cm.addAttr(cube, longName=blendshape_name, attributeType="float", defaultValue=0.0, minValue=0.0,
                           maxValue=1.0, keyable=True)
                print(f"Added attribute '{blendshape_name}' to '{cube}'.")

            # Create a unit conversion node for scaling (if needed)
            uc_node = cm.createNode("unitConversion", name=f"{blendshape_name}_uC")
            cm.setAttr(f"{uc_node}.conversionFactor", 10.0)

            # Connect the blendshape to the unit conversion
            cm.connectAttr(f"{blendshape_node}.{blendshape_name}", f"{uc_node}.input", force=True)

            # Connect the output of the unit conversion to the cube
            cm.connectAttr(f"{uc_node}.output", f"{cube}.{blendshape_name}", force=True)
            print(f"Connected '{blendshape_name}' from '{blendshape_node}' to '{cube}' via '{uc_node}'.")

        print("All blendshape attributes successfully connected to the cube.")

    except Exception as e:
        cm.warning(f"Error in connect_bs_to_cube: {e}")


def select_control_m2(name_space=""):
    """
    Selects specific controllers in Maya based on predefined naming conventions
    or selection rules.

    Steps:
    1. Validates the existence of the target controller group.
    2. Clears the current selection.
    3. Selects the predefined controls if they exist in the scene.

    Requirements:
    - The specified controls must exist in the Maya scene.
    - Maya's selection and object existence methods are used.

    """

    # List of predefined control names to be selected
    controls_to_select = [
        'head_ctrl',
        'neck_ctrl',
        'spine_ctrl',
        'root_ctrl',
        'arm_ctrl_L',
        'arm_ctrl_R',
        'leg_ctrl_L',
        'leg_ctrl_R'
    ]

    try:
        # Clears the current Maya selection
        cm.select(clear=True)

        # Iterate through the controls to select each one (if it exists)
        for control in controls_to_select:
            control = f"{name_space}:{control}"
            if cm.objExists(control):
                cm.select(control, add=True)
                print(f"Selected: {control}")
            else:
                print(f"Control '{control}' does not exist. Skipping...")

        # Provide feedback if the selection is empty after iteration
        current_selection = cm.ls(selection=True)
        if not current_selection:
            cm.warning("No controls were selected. Ensure that the controls exist in the scene.")
        else:
            print(f"Controls selected: {current_selection}")

    except Exception as e:
        cm.warning(f"An error occurred in select_control_m2: {e}")


def generate_bs_by_attr(mesh, bs_node):
    """
    Generates a duplicate of the specified mesh for each weight (target) in a blendshape node.
    Each duplicate is named after the corresponding blendshape weight.

    Parameters:
    - mesh (str): The name of the mesh to duplicate.
    - bs_node (str): The name of the blendshape node.

    Preconditions:
    - The mesh must exist in the Maya scene.
    - The blendshape node must be valid and have defined weights.

    Workflow:
    1. Validates the mesh and blendshape node existence.
    2. Retrieves all blendshape weight names from the blendshape node.
    3. Creates a group to organize the generated duplicates.
    4. Iterates through blendshape weights and:
        a. Activates each weight one at a time.
        b. Duplicates the mesh with the activated blendshape weight.
        c. Resets the blendshape weight to zero.
    """

    try:
        # Validate that the blendshape node exists
        if not cm.objExists(bs_node):
            cm.warning(f"Blendshape node '{bs_node}' not found.")
            return

        # Validate that the mesh exists
        if not cm.objExists(mesh):
            cm.warning(f"Mesh '{mesh}' not found in the scene.")
            return

        # Retrieve the weight names of the blendshape node
        weight_names = []
        alias_list = cm.aliasAttr(bs_node, query=True) or []
        for i in range(0, len(alias_list), 2):
            if alias_list[i + 1].startswith('weight['):
                weight_names.append(alias_list[i])

        if not weight_names:
            cm.warning(f"No weights found in blendshape node '{bs_node}'.")
            return

        # Create a group to organize duplicates
        group_name = f"{bs_node}_grp"
        if not cm.objExists(group_name):
            group_name = cm.group(name=group_name, empty=True, world=True)
        else:
            print(f"Group '{group_name}' already exists. Using existing group.")

        # Generate duplicates for each blendshape weight
        for weight in weight_names:
            if not cm.getAttr(f"{bs_node}.{weight}", lock=True):
                # Activate the blendshape weight

                cm.setAttr(f"{bs_node}.{weight}", 2)
            else:
                continue

            # Duplicate the mesh with the activated blendshape
            duplicate_name = f"{weight}"
            duplicate = cm.duplicate(mesh, name=duplicate_name, returnRootsOnly=True)[0]

            # Parent the duplicate to the group
            cm.parent(duplicate, group_name)

            # Reset the blendshape weight to zero
            cm.setAttr(f"{bs_node}.{weight}", 0)

            # Provide feedback
            print(f"Created duplicate for blendshape '{weight}': {duplicate}")

        print(f"All duplicates created successfully and stored under '{group_name}'.")

    except Exception as e:
        cm.warning(f"Error in generate_bs_by_attr: {e}")

def generate_bs_by_attr_from_control(mesh, control_name):
    try:
        # Validate that the blendshape node exists
        if not cm.objExists(control_name):
            cm.warning(f"Blendshape node '{control_name}' not found.")
            return

        # Validate that the mesh exists
        if not cm.objExists(mesh):
            cm.warning(f"Mesh '{mesh}' not found in the scene.")
            return

        # Retrieve the weight names of the blendshape node
        blendshape_list = {
            i: name for i, name in enumerate([
                'eyeBlinkLeft', 'eyeLookDownLeft', 'eyeLookInLeft', 'eyeLookOutLeft', 'eyeLookUpLeft',
                'eyeSquintLeft', 'eyeWideLeft', 'eyeBlinkRight', 'eyeLookDownRight', 'eyeLookInRight',
                'eyeLookOutRight', 'eyeLookUpRight', 'eyeSquintRight', 'eyeWideRight', 'jawForward',
                'jawLeft', 'jawRight', 'jawOpen', 'mouthClose', 'mouthFunnel', 'mouthPucker', 'mouthLeft',
                'mouthRight', 'mouthSmileLeft', 'mouthSmileRight', 'mouthFrownLeft', 'mouthFrownRight',
                'mouthDimpleLeft', 'mouthDimpleRight', 'mouthStretchLeft', 'mouthStretchRight',
                'mouthRollLower', 'mouthRollUpper', 'mouthShrugLower', 'mouthShrugUpper', 'mouthPressLeft',
                'mouthPressRight', 'mouthLowerDownLeft', 'mouthLowerDownRight', 'mouthUpperUpLeft',
                'mouthUpperUpRight', 'browDownLeft', 'browDownRight', 'browInnerUp', 'browOuterUpLeft',
                'browOuterUpRight', 'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight', 'noseSneerLeft',
                'noseSneerRight', 'tongueOut'
            ], start=1)
        }

        # Create a group to organize duplicates
        group_name = f"{control_name}_grp"
        if not cm.objExists(group_name):
            group_name = cm.group(name=group_name, empty=True, world=True)
        else:
            print(f"Group '{group_name}' already exists. Using existing group.")

        # Generate duplicates for each blendshape weight
        for index, name_bs in blendshape_list.items():
            # Create and configure unit conversion nodes
            cm.setAttr(f"{control_name}.{name_bs}", 10)

            # Duplicate the mesh with the activated blendshape
            duplicate_name = f"{name_bs}"
            duplicate = cm.duplicate(mesh, name=duplicate_name, returnRootsOnly=True)[0]

            # Parent the duplicate to the group
            cm.parent(duplicate, group_name)

            # Reset the blendshape weight to zero
            cm.setAttr(f"{control_name}.{name_bs}", 0)

            # Provide feedback
            print(f"Created duplicate for blendshape '{name_bs}': {duplicate}")

        print(f"All duplicates created successfully and stored under '{group_name}'.")

    except Exception as e:
        cm.warning(f"Error in generate_bs_by_attr: {e}")

```

## Source file: GenBS.py

{
  "path": "GenBS.py",
  "blob_sha": "f0a1df029b898bc73559147ecb453bc2993787ca",
  "sha256": "a89fa5976e1c7da6f8d7e177caa3dd591d5060af12caf244a69ebdedce6f6006",
  "lines": 244,
  "bytes": 8078,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 17,
    "blendshape": 17,
    "jaw": 4,
    "mouth": 23,
    "teeth": 1,
    "gum": 1,
    "tongue": 1,
    "lbs": 1,
    "rig": 20
  },
  "url": "https://github.com/hawkwood304/ARKit/blob/089d6dd54729b637f4e017002bde8929ebe5f955/GenBS.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self):"
    },
    {
      "line": 26,
      "text": "def __init__(self):"
    },
    {
      "line": 34,
      "text": "def __init__(self, name):"
    },
    {
      "line": 100,
      "text": "def __init__(self):"
    },
    {
      "line": 107,
      "text": "def create_widgets(self):"
    },
    {
      "line": 117,
      "text": "def create_layouts(self):"
    },
    {
      "line": 135,
      "text": "def create_connections(self):"
    },
    {
      "line": 139,
      "text": "def gen_bs(self):"
    },
    {
      "line": 156,
      "text": "def m2_connect_blendshape(self):"
    },
    {
      "line": 186,
      "text": "def display(cls):"
    },
    {
      "line": 198,
      "text": "def maya_main_window(cls):"
    },
    {
      "line": 207,
      "text": "def __init__(self):"
    },
    {
      "line": 221,
      "text": "def create_widget(self):"
    },
    {
      "line": 227,
      "text": "def create_layouts(self):"
    },
    {
      "line": 232,
      "text": "def create_connections(self):"
    },
    {
      "line": 235,
      "text": "def showEvent(self, e):"
    },
    {
      "line": 241,
      "text": "def closeEvent(self, e):"
    }
  ]
}

```text

from shiboken2 import wrapInstance

import os
import maya.cmds as cm
# import pymel.core as pm
import maya.OpenMaya as om
import maya.OpenMayaUI as omui

from PySide2 import QtWidgets, QtCore, QtGui
from maya.mel import eval


# import sys


class QHLine(QtWidgets.QFrame):

    def __init__(self):
        super(QHLine, self).__init__()
        self.setFrameShape(self.HLine)
        self.setFrameShadow(self.Sunken)


class QVLine(QtWidgets.QFrame):

    def __init__(self):
        super(QVLine, self).__init__()
        self.setFrameShape(self.VLine)
        self.setFrameShadow(self.Sunken)


class QHLineName(QtWidgets.QGridLayout):

    def __init__(self, name):
        super(QHLineName, self).__init__()
        name_lb = QtWidgets.QLabel(name)
        name_lb.setAlignment(QtCore.Qt.AlignCenter)
        name_lb.setStyleSheet("font: italic 9pt;" "color: azure;")
        self.addWidget(name_lb, 0, 0, 1, 1)
        self.addWidget(QHLine(), 0, 1, 1, 2)


# noinspection PyAttributeOutsideInit
class GenBS(QtWidgets.QWidget):
    blendshape_list = {
        1: 'eyeBlinkLeft',
        2: 'eyeLookDownLeft',
        3: 'eyeLookInLeft',
        4: 'eyeLookOutLeft',
        5: 'eyeLookUpLeft',
        6: 'eyeSquintLeft',
        7: 'eyeWideLeft',
        8: 'eyeBlinkRight',
        9: 'eyeLookDownRight',
        10: 'eyeLookInRight',
        11: 'eyeLookOutRight',
        12: 'eyeLookUpRight',
        13: 'eyeSquintRight',
        14: 'eyeWideRight',
        15: 'jawForward',
        16: 'jawLeft',
        17: 'jawRight',
        18: 'jawOpen',
        19: 'mouthClose',
        20: 'mouthFunnel',
        21: 'mouthPucker',
        22: 'mouthLeft',
        23: 'mouthRight',
        24: 'mouthSmileLeft',
        25: 'mouthSmileRight',
        26: 'mouthFrownLeft',
        27: 'mouthFrownRight',
        28: 'mouthDimpleLeft',
        29: 'mouthDimpleRight',
        30: 'mouthStretchLeft',
        31: 'mouthStretchRight',
        32: 'mouthRollLower',
        33: 'mouthRollUpper',
        34: 'mouthShrugLower',
        35: 'mouthShrugUpper',
        36: 'mouthPressLeft',
        37: 'mouthPressRight',
        38: 'mouthLowerDownLeft',
        39: 'mouthLowerDownRight',
        40: 'mouthUpperUpLeft',
        41: 'mouthUpperUpRight',
        42: 'browDownLeft',
        43: 'browDownRight',
        44: 'browInnerUp',
        45: 'browOuterUpLeft',
        46: 'browOuterUpRight',
        47: 'cheekPuff',
        48: 'cheekSquintLeft',
        49: 'cheekSquintRight',
        50: 'noseSneerLeft',
        51: 'noseSneerRight',
        52: 'tongueOut',
    }

    def __init__(self):
        super(GenBS, self).__init__()

        self.create_widgets()
        self.create_layouts()
        self.create_connections()

    def create_widgets(self):
        self.name_mesh_lb = QtWidgets.QLabel("Mesh name:")
        self.name_mesh_le = QtWidgets.QLineEdit()

        self.name_bs_node_lb = QtWidgets.QLabel("BlendShape node name:")
        self.name_bs_node_le = QtWidgets.QLineEdit()

        self.gen_bs_btn = QtWidgets.QPushButton("Gen BS")
        self.m2_bs_btn = QtWidgets.QPushButton("M2 connect")

    def create_layouts(self):
        first_layout = QtWidgets.QHBoxLayout()
        first_layout.addWidget(self.name_mesh_lb)
        first_layout.addWidget(self.name_mesh_le)

        second_layout = QtWidgets.QHBoxLayout()
        second_layout.addWidget(self.name_bs_node_lb)
        second_layout.addWidget(self.name_bs_node_le)

        third_layout = QtWidgets.QHBoxLayout()
        third_layout.addWidget(self.gen_bs_btn)
        third_layout.addWidget(self.m2_bs_btn)

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(first_layout)
        main_layout.addLayout(second_layout)
        main_layout.addLayout(third_layout)

    def create_connections(self):
        self.gen_bs_btn.clicked.connect(self.gen_bs)
        self.m2_bs_btn.clicked.connect(self.m2_connect_blendshape)

    def gen_bs(self):

        name_bs_node = self.name_bs_node_le.text()
        name_mesh = self.name_mesh_le.text()

        grp = cm.group(name=f"{name_bs_node}_grp", world=True, r=True, empty=True)
        cm.select(clear=True)
        if name_bs_node and name_mesh:
            for i in range(1, 53):
                name_bs = self.blendshape_list.get(i)
                cm.setAttr(f"{name_bs_node}.{name_bs}", 1)
                dup = cm.duplicate(name_mesh, name=name_bs, rr=False)
                cm.parent(dup, grp)
                cm.setAttr(f"{name_bs_node}.{name_bs}", 0)
        else:
            om.MGlobal_displayError("Need name blendshape node to compile")

    def m2_connect_blendshape(self):
        list_name_bs_node = ["blendShape__head_ply", "blendShape__teethGum_ply", "blendShape__eyelashesLower_ply",
                             "blendShape__eyelashesUpper_ply", "eyeLBS", "eyeRBS", "blendShape__eyebrow_ply",
                             "blendShape__eyeshell_ply", "blendShape__clothInside01_ply",
                             "blendShape__clothInside02_ply",
                             "blendShape__lowerLashes_ply", "blendShape__upperLashes_ply"]
        control_ARKit_name = "ctrlARKitBS_M"
        for my_blendShape_node in list_name_bs_node:
            if cm.objExists(my_blendShape_node):
                print(my_blendShape_node)
                for i in range(1, 53):
                    name_bs = self.blendshape_list.get(i)
                    uc_node = cm.createNode("unitConversion", n="{}_{}_uC".format(my_blendShape_node, name_bs))
                    cm.setAttr("{}.conversionFactor".format(uc_node), 0.1)
                    cm.connectAttr("{}.{}".format(control_ARKit_name, name_bs), "{}.input".format(uc_node),
                                   f=True)
                    cm.connectAttr("{}.output".format(uc_node), "{}.{}".format(my_blendShape_node, name_bs),
                                   f=True)


# noinspection PyMethodMayBeStatic,PyAttributeOutsideInit,PyMethodOverriding
class MainWindow(QtWidgets.QDialog):
    WINDOW_TITLE = "Gen BS"

    SCRIPTS_DIR = cm.internalVar(userScriptDir=True)
    ICON_DIR = os.path.join(SCRIPTS_DIR, 'Thi/Icon')

    dlg_instance = None

    @classmethod
    def display(cls):
        if not cls.dlg_instance:
            cls.dlg_instance = MainWindow()

        if cls.dlg_instance.isHidden():
            cls.dlg_instance.show()

        else:
            cls.dlg_instance.raise_()
            cls.dlg_instance.activateWindow()

    @classmethod
    def maya_main_window(cls):
        """

        Returns: The Maya main window widget as a Python object

        """
        main_window_ptr = omui.MQtUtil.mainWindow()
        return wrapInstance(int(main_window_ptr), QtWidgets.QWidget)

    def __init__(self):
        super(MainWindow, self).__init__(self.maya_main_window())

        self.setWindowTitle(self.WINDOW_TITLE)
        self.setWindowFlags(self.windowFlags() ^ QtCore.Qt.WindowContextHelpButtonHint)

        self.geometry = None

        self.setMinimumSize(400, 150)
        self.setMaximumSize(400, 150)
        self.create_widget()
        self.create_layouts()
        self.create_connections()

    def create_widget(self):
        self.content_layout = QtWidgets.QHBoxLayout()
        self.content_layout.addWidget(GenBS())

        self.close_btn = QtWidgets.QPushButton("Close")

    def create_layouts(self):

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(self.content_layout)

    def create_connections(self):
        self.close_btn.clicked.connect(self.close)

    def showEvent(self, e):
        super(MainWindow, self).showEvent(e)

        if self.geometry:
            self.restoreGeometry(self.geometry)

    def closeEvent(self, e):
        super(MainWindow, self).closeEvent(e)

        self.geometry = self.saveGeometry()


```

## Source file: ConnectBS.py

{
  "path": "ConnectBS.py",
  "blob_sha": "f0951a680383ba5fe82210226e583b2f70ab1d95",
  "sha256": "89a5c857176c40bb37b968fd105566eacc4ab40f3cbfd68f028ce8bcf87ded3b",
  "lines": 242,
  "bytes": 7497,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 14,
    "blendshape": 8,
    "jaw": 4,
    "mouth": 23,
    "tongue": 1,
    "rig": 20
  },
  "url": "https://github.com/hawkwood304/ARKit/blob/089d6dd54729b637f4e017002bde8929ebe5f955/ConnectBS.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self):"
    },
    {
      "line": 28,
      "text": "def __init__(self):"
    },
    {
      "line": 36,
      "text": "def __init__(self, name):"
    },
    {
      "line": 102,
      "text": "def __init__(self):"
    },
    {
      "line": 109,
      "text": "def create_widgets(self):"
    },
    {
      "line": 124,
      "text": "def create_layouts(self):"
    },
    {
      "line": 140,
      "text": "def create_connections(self):"
    },
    {
      "line": 143,
      "text": "def connect_blendshape(self):"
    },
    {
      "line": 184,
      "text": "def display(cls):"
    },
    {
      "line": 196,
      "text": "def maya_main_window(cls):"
    },
    {
      "line": 205,
      "text": "def __init__(self):"
    },
    {
      "line": 219,
      "text": "def create_widget(self):"
    },
    {
      "line": 225,
      "text": "def create_layouts(self):"
    },
    {
      "line": 230,
      "text": "def create_connections(self):"
    },
    {
      "line": 233,
      "text": "def showEvent(self, e):"
    },
    {
      "line": 239,
      "text": "def closeEvent(self, e):"
    }
  ]
}

```text

from symbol import factor

from shiboken2 import wrapInstance

import os
import maya.cmds as cm
# import pymel.core as pm
import maya.OpenMaya as om
import maya.OpenMayaUI as omui

from PySide2 import QtWidgets, QtCore, QtGui
from maya.mel import eval


# import sys


class QHLine(QtWidgets.QFrame):

    def __init__(self):
        super(QHLine, self).__init__()
        self.setFrameShape(self.HLine)
        self.setFrameShadow(self.Sunken)


class QVLine(QtWidgets.QFrame):

    def __init__(self):
        super(QVLine, self).__init__()
        self.setFrameShape(self.VLine)
        self.setFrameShadow(self.Sunken)


class QHLineName(QtWidgets.QGridLayout):

    def __init__(self, name):
        super(QHLineName, self).__init__()
        name_lb = QtWidgets.QLabel(name)
        name_lb.setAlignment(QtCore.Qt.AlignCenter)
        name_lb.setStyleSheet("font: italic 9pt;" "color: azure;")
        self.addWidget(name_lb, 0, 0, 1, 1)
        self.addWidget(QHLine(), 0, 1, 1, 2)


# noinspection PyAttributeOutsideInit
class ConnectBS(QtWidgets.QWidget):
    blendshape_list = {
        1: 'eyeBlinkLeft',
        2: 'eyeLookDownLeft',
        3: 'eyeLookInLeft',
        4: 'eyeLookOutLeft',
        5: 'eyeLookUpLeft',
        6: 'eyeSquintLeft',
        7: 'eyeWideLeft',
        8: 'eyeBlinkRight',
        9: 'eyeLookDownRight',
        10: 'eyeLookInRight',
        11: 'eyeLookOutRight',
        12: 'eyeLookUpRight',
        13: 'eyeSquintRight',
        14: 'eyeWideRight',
        15: 'jawForward',
        16: 'jawLeft',
        17: 'jawRight',
        18: 'jawOpen',
        19: 'mouthClose',
        20: 'mouthFunnel',
        21: 'mouthPucker',
        22: 'mouthLeft',
        23: 'mouthRight',
        24: 'mouthSmileLeft',
        25: 'mouthSmileRight',
        26: 'mouthFrownLeft',
        27: 'mouthFrownRight',
        28: 'mouthDimpleLeft',
        29: 'mouthDimpleRight',
        30: 'mouthStretchLeft',
        31: 'mouthStretchRight',
        32: 'mouthRollLower',
        33: 'mouthRollUpper',
        34: 'mouthShrugLower',
        35: 'mouthShrugUpper',
        36: 'mouthPressLeft',
        37: 'mouthPressRight',
        38: 'mouthLowerDownLeft',
        39: 'mouthLowerDownRight',
        40: 'mouthUpperUpLeft',
        41: 'mouthUpperUpRight',
        42: 'browDownLeft',
        43: 'browDownRight',
        44: 'browInnerUp',
        45: 'browOuterUpLeft',
        46: 'browOuterUpRight',
        47: 'cheekPuff',
        48: 'cheekSquintLeft',
        49: 'cheekSquintRight',
        50: 'noseSneerLeft',
        51: 'noseSneerRight',
        52: 'tongueOut',
    }

    def __init__(self):
        super(ConnectBS, self).__init__()

        self.create_widgets()
        self.create_layouts()
        self.create_connections()

    def create_widgets(self):
        self.driver_name_lb = QtWidgets.QLabel("Driver:")
        self.driver_name_lb.setToolTip("Ten node cua blendshape hoac control se dieu khien")
        self.driver_name_le = QtWidgets.QLineEdit()

        self.driven_name_lb = QtWidgets.QLabel("Driven:")
        self.driven_name_lb.setToolTip("Ten node cua blendshape hoac control se bi dieu khien")
        self.driven_name_le = QtWidgets.QLineEdit()

        self.fraction_lb = QtWidgets.QLabel("Fraction:")
        self.fraction_lb.setToolTip("Ty le quy doi")
        self.fraction_le = QtWidgets.QLineEdit()
        self.connect_bs_btn = QtWidgets.QPushButton("Connect BS")
        self.connect_bs_btn.setToolTip("Ket noi blendshape vao control hoac blenshape khac da chon")

    def create_layouts(self):
        first_layout = QtWidgets.QGridLayout()
        first_layout.addWidget(self.driver_name_lb, 0, 0)
        first_layout.addWidget(self.driver_name_le, 0, 1)
        first_layout.addWidget(self.driven_name_lb, 1, 0)
        first_layout.addWidget(self.driven_name_le, 1, 1)
        first_layout.addWidget(self.fraction_lb, 2, 0)
        first_layout.addWidget(self.fraction_le, 2, 1)

        second_layout = QtWidgets.QHBoxLayout()
        second_layout.addWidget(self.connect_bs_btn)

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(first_layout)
        main_layout.addLayout(second_layout)

    def create_connections(self):
        self.connect_bs_btn.clicked.connect(self.connect_blendshape)

    def connect_blendshape(self):
        fraction = self.fraction_le.text()
        if fraction:
            fraction = int(fraction)
        else:
            om.MGlobal_displayError("Dien ty le vao tro trong")

        driver_name = self.driver_name_le.text()
        print(driver_name)

        driven_name = self.driven_name_le.text()
        print(driven_name)

        if driven_name and driver_name:

            for i in range(1, 53):
                name_bs = self.blendshape_list.get(i)
                uc_node = cm.createNode("unitConversion", n=f"{driver_name}_{driven_name}_{name_bs}_uC")

                cm.setAttr(f"{uc_node}.conversionFactor", fraction)
                cm.connectAttr(f"{driver_name}.{name_bs}", f"{uc_node}.input",
                               f=True)
                cm.connectAttr(f"{uc_node}.output", f"{driven_name}.{name_bs}",
                               f=True)

        else:
            om.MGlobal_displayError("Lam on dien vao cho trong")


# noinspection PyMethodMayBeStatic,PyAttributeOutsideInit,PyMethodOverriding


class MainWindow(QtWidgets.QDialog):
    WINDOW_TITLE = "Connect blendshape ARKit"

    SCRIPTS_DIR = cm.internalVar(userScriptDir=True)
    ICON_DIR = os.path.join(SCRIPTS_DIR, 'Thi/Icon')

    dlg_instance = None

    @classmethod
    def display(cls):
        if not cls.dlg_instance:
            cls.dlg_instance = MainWindow()

        if cls.dlg_instance.isHidden():
            cls.dlg_instance.show()

        else:
            cls.dlg_instance.raise_()
            cls.dlg_instance.activateWindow()

    @classmethod
    def maya_main_window(cls):
        """

        Returns: The Maya main window widget as a Python object

        """
        main_window_ptr = omui.MQtUtil.mainWindow()
        return wrapInstance(int(main_window_ptr), QtWidgets.QWidget)

    def __init__(self):
        super(MainWindow, self).__init__(self.maya_main_window())

        self.setWindowTitle(self.WINDOW_TITLE)
        self.setWindowFlags(self.windowFlags() ^ QtCore.Qt.WindowContextHelpButtonHint)

        self.geometry = None

        self.setMinimumSize(400, 150)
        self.setMaximumSize(400, 150)
        self.create_widget()
        self.create_layouts()
        self.create_connections()

    def create_widget(self):
        self.content_layout = QtWidgets.QHBoxLayout()
        self.content_layout.addWidget(ConnectBS())

        self.close_btn = QtWidgets.QPushButton("Close")

    def create_layouts(self):

        main_layout = QtWidgets.QVBoxLayout(self)
        main_layout.addLayout(self.content_layout)

    def create_connections(self):
        self.close_btn.clicked.connect(self.close)

    def showEvent(self, e):
        super(MainWindow, self).showEvent(e)

        if self.geometry:
            self.restoreGeometry(self.geometry)

    def closeEvent(self, e):
        super(MainWindow, self).closeEvent(e)

        self.geometry = self.saveGeometry()


```

# Facial rendering source scan: jign/facial_rigging

Repository: https://github.com/jign/facial_rigging

Pinned source revision: 1fbd80f6f051f648f3ee7fb5980dc862233954d1

Discovery topic: facial rig

Description: Facial Rigging Toolset Demo

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["src/face_jnt_guides/FaceJointGuides/components/eye_judd/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/__pycache__/eye_judd_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/eye_judd_component.py", "src/face_jnt_guides/FaceJointGuides/components/eye_sky/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/eye_sky/eye_sky_component.py"], "skin": ["demo/head_skin.jSkin"], "blendshape": ["src/face_pose_ui/FacePoseUI/ui/corrective_blendshapes_dialog.py"], "jaw": ["src/face_jnt_guides/FaceJointGuides/components/jaw/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/jaw/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/jaw/__pycache__/jaw_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/jaw/jaw_component.py"], "rig": ["demo/face_rig.json", "src/face_pose_ui/FacePoseUI/core/rig.py", "src/face_pose_ui/FacePoseUI/core/serialization/rig_serializer.py"], "face": ["bin/FacePoseDriver.exp", "bin/FacePoseDriver.lib", "bin/FacePoseDriver.mll", "bin/FacePoseDriver.pdb", "demo/face_controls.abc", "demo/face_guides.mb", "demo/face_rig.json", "src/face_jnt_guides/FaceJointGuides/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/ear/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/ear/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/ear/__pycache__/ear_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/ear/ear_component.py", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/__pycache__/eye_judd_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/eye_judd/eye_judd_component.py", "src/face_jnt_guides/FaceJointGuides/components/eye_sky/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/eye_sky/eye_sky_component.py", "src/face_jnt_guides/FaceJointGuides/components/jaw/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/jaw/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/jaw/__pycache__/jaw_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/jaw/jaw_component.py", "src/face_jnt_guides/FaceJointGuides/components/neck/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/neck/__pycache__/__init__.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/neck/__pycache__/neck_component.cpython-310.pyc", "src/face_jnt_guides/FaceJointGuides/components/neck/neck_component.py", "src/face_jnt_guides/FaceJointGuides/components/point_control/__init__.py", "src/face_jnt_guides/FaceJointGuides/components/point_control/__pycache__/__init__.cpython-310.pyc"]}

## Source file: src/face_pose_ui/FacePoseUI/core/rig.py

{
  "path": "src/face_pose_ui/FacePoseUI/core/rig.py",
  "blob_sha": "9f32528f37e77b099fcc34d81f998fa71ed611ff",
  "sha256": "2c27fd5633274dd801f2c01a49081d41a75f8b92b5241e85437d3c371ef18c8b",
  "lines": 139,
  "bytes": 5858,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 14,
    "face": 7
  },
  "url": "https://github.com/jign/facial_rigging/blob/1fbd80f6f051f648f3ee7fb5980dc862233954d1/src/face_pose_ui/FacePoseUI/core/rig.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def __init__(self, driver: Optional[pm.PyNode]):"
    },
    {
      "line": 23,
      "text": "def set_driver(self, driver: pm.PyNode):"
    },
    {
      "line": 27,
      "text": "def init_raw_poses(self):"
    },
    {
      "line": 35,
      "text": "def add_pose(self, pose):"
    },
    {
      "line": 41,
      "text": "def delete_pose(self, pose: Pose):"
    },
    {
      "line": 45,
      "text": "def delete_subpose(self, pose: Pose, subpose: Subpose):"
    },
    {
      "line": 48,
      "text": "def add_subposes_to_pose(self, pose: Pose, attrs: Union[List[pm.Attribute], pm.Attribute]) -> List[Subpose]:"
    },
    {
      "line": 61,
      "text": "def add_attrs_to_rig(self, attrs: List[pm.Attribute]):"
    },
    {
      "line": 107,
      "text": "def is_rig_driving_attr(self, attr: pm.Attribute) -> bool:"
    },
    {
      "line": 114,
      "text": "def get_poses(self) -> List[Pose]:"
    },
    {
      "line": 117,
      "text": "def get_control_objects(self) -> List[pm.PyNode]:"
    },
    {
      "line": 123,
      "text": "def add_key(self, pose: Pose, subpose: Subpose, time, val):"
    },
    {
      "line": 129,
      "text": "def find_raw_poses(self) -> List[pm.Attribute]:"
    }
  ]
}

```text

from FacePoseUI.core.pose import Pose
from FacePoseUI.core.subpose import Subpose
import pymel.core as pm
import maya.api.OpenMaya as om
from typing import List, Set, Union, Optional
import FacePoseUI.core.core_tools as ct


# TODO try evaluateNumElements() to build the array block instead of my hacks
# TODO Maybe the Rig should keep a list of connected attributes to make it easier to filter without having to do if self.get_driven_attribute_index(attr) is not None:

class Rig:
    def __init__(self, driver: Optional[pm.PyNode]):
        # Can be None if we're rebuilding a rig from a file and we haven't created the driver node yet
        self.driver: pm.PyNode = driver

        self.poses: List[Pose] = []

        if self.driver:
            ct.force_refresh_driver_conns(self.driver)
            self.init_raw_poses()

    def set_driver(self, driver: pm.PyNode):
        # We don't refresh conns or init any poses because setting the driver means we're rebuilding the rig from scratch, so nothing to refresh.
        self.driver = driver

    def init_raw_poses(self):
        raw_poses = self.find_raw_poses()
        for raw_pose in raw_poses:
            pose = Pose()
            pose.build_from_attr(raw_pose, self)
            if pose not in self.poses:
                self.poses.append(pose)

    def add_pose(self, pose):
        if pose not in self.poses:
            self.poses.append(pose)

        ct.force_refresh_driver_conns(self.driver)

    def delete_pose(self, pose: Pose):
        self.poses.remove(pose)
        pose.delete()

    def delete_subpose(self, pose: Pose, subpose: Subpose):
        pose.delete_subpose(subpose)

    def add_subposes_to_pose(self, pose: Pose, attrs: Union[List[pm.Attribute], pm.Attribute]) -> List[Subpose]:
        if pose is None:
            om.MGlobal.displayWarning("FacePoseUI // Rig::add_subposes called with no pose selected.")
            return []
        if type(attrs) is not list:
            attrs = [attrs]
        new_subposes = []
        self.add_attrs_to_rig(attrs)
        for attr in attrs:
            new_subpose = pose.add_subpose(attr)
            new_subposes.append(new_subpose)
        return new_subposes

    def add_attrs_to_rig(self, attrs: List[pm.Attribute]):
        """
        This adds the attributes to the rig if they aren't already there.
        """
        if self.driver is None:
            om.MGlobal.displayWarning('FacePoseUI // Rig::add_attrs_to_rig Trying to add objects to the rig without a '
                                      'driver selected')
            return

        # We need to force a refresh due to the plug bug, possibly already fixed with setDependentsDirty implementation in the plugin
        ct.force_refresh_driver_conns(self.driver)

        # First make sure to filter each kind of attribute
        linear_attrs = [attr for attr in attrs if 'rotate' not in attr.name()]
        angular_attrs = [attr for attr in attrs if 'rotate' in attr.name()]
        for attr in linear_attrs:
            # Check if the attribute is already in the rig
            if ct.get_driven_linear_attribute_index(self.driver, attr) is not None:
                continue

            array_idx = ct.find_next_available_linear_index(self.driver)
            source_attr = self.driver.attr('linearOutputs')[array_idx]
            # I feel like this is unnecessary since get_driven_linear_attribute_index should take care of it
            # if source_attr.isConnected():
            #     om.MGlobal.displayError('FacePoseUI // Rig::add_attrs_to_rig trying to connect to a previously '
            #                             'connected attribute')
            source_attr.connect(attr, lock=False)

            # Important to handle the ugly plug bug, possibly already fixed with setDependentsDirty implementation in the plugin
            source_attr.get()

        for attr in angular_attrs:
            # Check if the attribute is already in the rig
            if ct.get_driven_angular_attribute_index(self.driver, attr) is not None:
                continue

            array_idx = ct.find_next_available_angular_index(self.driver)
            source_attr = self.driver.attr('angularOutputs')[array_idx]
            # if source_attr.isConnected():
            #     om.MGlobal.displayError('FacePoseUI // Rig::add_attrs_to_rig trying to connect to a previously '
            #                             'connected attribute')
            source_attr.connect(attr, lock=False)

            # Important to handle the ugly plug bug, possibly already fixed with setDependentsDirty implementation in the plugin
            source_attr.get()

    def is_rig_driving_attr(self, attr: pm.Attribute) -> bool:
        for pose in self.poses:
            for subpose in pose.subposes:
                if subpose.get_driven_attr() is attr:
                    return True
        return False      

    def get_poses(self) -> List[Pose]:
        return self.poses

    def get_control_objects(self) -> List[pm.PyNode]:
        objs: Set[pm.PyNode] = set()
        for pose in self.poses:
            objs.add(pose.get_host_obj())
        return list(objs)

    def add_key(self, pose: Pose, subpose: Subpose, time, val):
        if pose is None:
            return False
        res = pose.add_key(subpose, time, val)
        return res

    def find_raw_poses(self) -> List[pm.Attribute]:
        """
        Raw poses are attributes. The Pose() object needs to be built from them
        """
        elems = self.driver.attr('poses').elements()
        filtered_elems = [elem for elem in elems if 'poseWeight' in elem]
        out_poses: List[pm.Attribute] = []
        for pw in filtered_elems:
            if self.driver.attr(pw).isConnected():
                out_poses.append(self.driver.attr(pw).inputs(p=1)[0])
        return out_poses


```

## Source file: src/face_pose_ui/FacePoseUI/ui/corrective_blendshapes_dialog.py

{
  "path": "src/face_pose_ui/FacePoseUI/ui/corrective_blendshapes_dialog.py",
  "blob_sha": "0e4825f0e305c3b753adbb24e0985490b9797cf2",
  "sha256": "1e5f0df2cb114ab0e7af6e61add15bcd66ce9a2b5ce4aa690911ad7a271762b4",
  "lines": 311,
  "bytes": 13958,
  "selection_score": 18,
  "keyword_line_counts": {
    "sss": 2,
    "blendshape": 76,
    "blend_shape": 3,
    "rig": 23,
    "face": 2
  },
  "url": "https://github.com/jign/facial_rigging/blob/1fbd80f6f051f648f3ee7fb5980dc862233954d1/src/face_pose_ui/FacePoseUI/ui/corrective_blendshapes_dialog.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, rig: Rig, parent=smu.get_maya_main_win()):"
    },
    {
      "line": 23,
      "text": "def create_widgets(self):"
    },
    {
      "line": 48,
      "text": "def create_layouts(self):"
    },
    {
      "line": 93,
      "text": "def create_connections(self):"
    },
    {
      "line": 101,
      "text": "def refresh_all(self):"
    },
    {
      "line": 105,
      "text": "def refresh_poses_list(self):"
    },
    {
      "line": 116,
      "text": "def refresh_blendshapes_list(self):"
    },
    {
      "line": 142,
      "text": "def on_blendshape_selected(self):"
    },
    {
      "line": 146,
      "text": "def refresh_controlling_poses_list(self):"
    },
    {
      "line": 175,
      "text": "def add_poses_to_blendshape(self):"
    },
    {
      "line": 199,
      "text": "def remove_poses_from_blendshape(self):"
    },
    {
      "line": 224,
      "text": "def create_blendshape(self):"
    },
    {
      "line": 271,
      "text": "def delete_blendshape(self):"
    }
  ]
}

```text

# New file: ui/corrective_blendshapes_dialog.py
import pymel.core as pm
from PySide2 import QtCore, QtWidgets
import skylib.utils.module_utils as smu
from FacePoseUI.core.pose import Pose
from FacePoseUI.core.rig import Rig

class CorrectiveBlendshapesDialog(QtWidgets.QDialog):
    def __init__(self, rig: Rig, parent=smu.get_maya_main_win()):
        super(CorrectiveBlendshapesDialog, self).__init__(parent)
        self.rig = rig
        
        self.setWindowTitle("Corrective Blendshapes Manager")
        self.resize(800, 500)
        self.setWindowFlags(QtCore.Qt.WindowStaysOnTopHint | QtCore.Qt.Window)
        
        self.create_widgets()
        self.create_layouts()
        self.create_connections()
        
        self.refresh_all()
        
    def create_widgets(self):
        # Available poses list
        self.poses_list = QtWidgets.QListWidget()
        self.poses_list.setSelectionMode(QtWidgets.QAbstractItemView.ExtendedSelection)
        self.poses_label = QtWidgets.QLabel("Available Poses:")
        
        # Controlling poses for current blendshape
        self.controlling_poses_list = QtWidgets.QListWidget()
        self.controlling_poses_label = QtWidgets.QLabel("Poses driving selected blendshape:")
        
        # Blendshapes list
        self.blendshapes_list = QtWidgets.QListWidget()
        self.blendshapes_label = QtWidgets.QLabel("Corrective Blendshapes:")
        
        # Transfer buttons
        self.add_poses_to_blendshape_btn = QtWidgets.QPushButton("->")
        self.add_poses_to_blendshape_btn.setToolTip("Add selected poses to drive selected blendshape")
        self.remove_poses_from_blendshape_btn = QtWidgets.QPushButton("<-")
        self.remove_poses_from_blendshape_btn.setToolTip("Remove selected poses from blendshape")
        
        # Blendshape management buttons
        self.create_blendshape_btn = QtWidgets.QPushButton("Create New Blendshape")
        self.delete_blendshape_btn = QtWidgets.QPushButton("Delete Blendshape")
        self.refresh_btn = QtWidgets.QPushButton("Refresh")
        
    def create_layouts(self):
        # Poses list
        poses_layout = QtWidgets.QVBoxLayout()
        poses_layout.addWidget(self.poses_label)
        poses_layout.addWidget(self.poses_list)
        
        # Transfer buttons
        transfer_buttons_layout = QtWidgets.QVBoxLayout()
        transfer_buttons_layout.addStretch()
        transfer_buttons_layout.addWidget(self.add_poses_to_blendshape_btn)
        transfer_buttons_layout.addWidget(self.remove_poses_from_blendshape_btn)
        transfer_buttons_layout.addStretch()
        
        # Controlling poses list
        controlling_poses_layout = QtWidgets.QVBoxLayout()
        controlling_poses_layout.addWidget(self.controlling_poses_label)
        controlling_poses_layout.addWidget(self.controlling_poses_list)
        
        # Blendshapes list
        blendshapes_layout = QtWidgets.QVBoxLayout()
        blendshapes_layout.addWidget(self.blendshapes_label)
        blendshapes_layout.addWidget(self.blendshapes_list)
        
        # Blendshape buttons
        blendshape_buttons_layout = QtWidgets.QHBoxLayout()
        blendshape_buttons_layout.addWidget(self.create_blendshape_btn)
        blendshape_buttons_layout.addWidget(self.delete_blendshape_btn)
        blendshape_buttons_layout.addWidget(self.refresh_btn)
        
        # Combined left side
        left_layout = QtWidgets.QHBoxLayout()
        left_layout.addLayout(poses_layout)
        left_layout.addLayout(transfer_buttons_layout)
        left_layout.addLayout(controlling_poses_layout)
        
        # Main layout
        main_layout = QtWidgets.QVBoxLayout(self)
        
        lists_layout = QtWidgets.QHBoxLayout()
        lists_layout.addLayout(left_layout, 2)
        lists_layout.addLayout(blendshapes_layout, 1)
        
        main_layout.addLayout(lists_layout)
        main_layout.addLayout(blendshape_buttons_layout)
        
    def create_connections(self):
        self.add_poses_to_blendshape_btn.clicked.connect(self.add_poses_to_blendshape)
        self.remove_poses_from_blendshape_btn.clicked.connect(self.remove_poses_from_blendshape)
        self.create_blendshape_btn.clicked.connect(self.create_blendshape)
        self.delete_blendshape_btn.clicked.connect(self.delete_blendshape)
        self.refresh_btn.clicked.connect(self.refresh_all)
        self.blendshapes_list.itemSelectionChanged.connect(self.on_blendshape_selected)
        
    def refresh_all(self):
        self.refresh_poses_list()
        self.refresh_blendshapes_list()
        
    def refresh_poses_list(self):
        """Fill the poses list with all available poses from the rig"""
        self.poses_list.clear()
        if not self.rig:
            return
            
        for pose in self.rig.get_poses():
            item = QtWidgets.QListWidgetItem(pose.get_pose_name())
            item.setData(QtCore.Qt.UserRole, pose)
            self.poses_list.addItem(item)
            
    def refresh_blendshapes_list(self):
        """Fill the blendshapes list with corrective outputs from the rig driver"""
        self.blendshapes_list.clear()
        if not self.rig or not self.rig.driver:
            return
            
        # Get corrective outputs from the driver node
        try:
            corrective_outputs = self.rig.driver.attr('correctiveOutputs').elements()
            for i, output in enumerate(corrective_outputs):
                # Create a display name - might want to customize this
                name = f"Corrective_{i}"
                
                # Try to get target blendshape name if connected
                if output.isConnected():
                    target = output.outputs(p=True)
                    if target:
                        name = f"{target[0].name().split('.')[-1]} ({i})"
                
                item = QtWidgets.QListWidgetItem(name)
                item.setData(QtCore.Qt.UserRole, i)  # Store the index
                self.blendshapes_list.addItem(item)
        except:
            # Handle if attributes don't exist
            pass
            
    def on_blendshape_selected(self):
        """When a blendshape is selected, update the controlling poses list"""
        self.refresh_controlling_poses_list()
        
    def refresh_controlling_poses_list(self):
        """Show which poses are driving the selected blendshape"""
        self.controlling_poses_list.clear()
        
        selected_items = self.blendshapes_list.selectedItems()
        if not selected_items or not self.rig or not self.rig.driver:
            return
            
        blendshape_idx = selected_items[0].data(QtCore.Qt.UserRole)
        
        # Get poses driving this corrective
        try:
            inputs_plug = self.rig.driver.attr(f'correctiveInputs[{blendshape_idx}]')
            pose_inputs = inputs_plug.attr('correctiveInputPose').elements()
            
            for pose_input in pose_inputs:
                if pose_input.isConnected():
                    source_attr = pose_input.inputs(p=True)[0]
                    # Find the pose object that corresponds to this attribute
                    for pose in self.rig.get_poses():
                        if pose.pose_attr == source_attr:
                            item = QtWidgets.QListWidgetItem(pose.get_pose_name())
                            item.setData(QtCore.Qt.UserRole, pose)
                            self.controlling_poses_list.addItem(item)
                            break
        except:
            # Handle if attributes don't exist
            pass
            
    def add_poses_to_blendshape(self):
        """Add selected poses to drive the selected blendshape"""
        selected_poses = [item.data(QtCore.Qt.UserRole) for item in self.poses_list.selectedItems()]
        selected_blendshape_items = self.blendshapes_list.selectedItems()
        
        if not selected_poses or not selected_blendshape_items or not self.rig or not self.rig.driver:
            return
            
        blendshape_idx = selected_blendshape_items[0].data(QtCore.Qt.UserRole)
        
        # Connect selected poses to the corrective input
        for pose in selected_poses:
            # Find next available index in correctiveInputPose array
            try:
                input_plug = self.rig.driver.attr(f'correctiveInputs[{blendshape_idx}]')
                next_idx = len(input_plug.attr('correctiveInputPose').elements())
                pose_input = input_plug.attr(f'correctiveInputPose[{next_idx}]')
                pose.pose_attr.connect(pose_input)
            except Exception as e:
                pm.warning(f"Failed to connect pose: {e}")
                
        # Refresh the UI
        self.refresh_controlling_poses_list()
        
    def remove_poses_from_blendshape(self):
        """Remove selected poses from the current blendshape"""
        selected_poses = [item.data(QtCore.Qt.UserRole) for item in self.controlling_poses_list.selectedItems()]
        selected_blendshape_items = self.blendshapes_list.selectedItems()
        
        if not selected_poses or not selected_blendshape_items or not self.rig or not self.rig.driver:
            return
            
        blendshape_idx = selected_blendshape_items[0].data(QtCore.Qt.UserRole)
        
        # Disconnect selected poses from the corrective input
        for pose in selected_poses:
            try:
                input_plug = self.rig.driver.attr(f'correctiveInputs[{blendshape_idx}]')
                pose_inputs = input_plug.attr('correctiveInputPose').elements()
                
                for pose_input in pose_inputs:
                    if pose_input.isConnected() and pose_input.inputs(p=True)[0] == pose.pose_attr:
                        pose_input.disconnect()
            except Exception as e:
                pm.warning(f"Failed to disconnect pose: {e}")
                
        # Refresh the UI
        self.refresh_controlling_poses_list()
        
    def create_blendshape(self):
        """Create a new corrective blendshape"""
        if not self.rig or not self.rig.driver:
            return
            
        # Find next available index for correctiveOutputs and add an element
        try:
            next_idx = len(self.rig.driver.attr('correctiveOutputs').elements())
            # Create a new input compound element
            input_plug = self.rig.driver.attr(f'correctiveInputs[{next_idx}]')
            # Make sure output exists too (should be created automatically)
            output_plug = self.rig.driver.attr(f'correctiveOutputs[{next_idx}]')
            
            # Open dialog to connect to a blendshape target
            result = pm.confirmDialog(
                title='Connect Blendshape',
                message='Select a blendshape target, then click Connect',
                button=['Connect', 'Cancel'],
                defaultButton='Connect',
                cancelButton='Cancel',
                dismissString='Cancel'
            )
            
            if result == 'Connect':
                selection = pm.ls(sl=True)
                if selection and isinstance(selection[0], pm.nodetypes.BlendShape):
                    # Get target weight attributes
                    blend_shape = selection[0]
                    # Let user select from a list of targets
                    targets = pm.listAttr(blend_shape + '.w', multi=True)
                    if targets:
                        target = pm.confirmDialog(
                            title='Select Target', 
                            message='Select blendshape target',
                            button=targets,
                            defaultButton=targets[0]
                        )
                        if target in targets:
                            # Connect the output to the selected target
                            output_plug.connect(blend_shape.attr(target))
                
            # Refresh the UI
            self.refresh_blendshapes_list()
            
        except Exception as e:
            pm.warning(f"Failed to create corrective blendshape: {e}")
            
    def delete_blendshape(self):
        """Delete the selected corrective blendshape"""
        selected_items = self.blendshapes_list.selectedItems()
        if not selected_items or not self.rig or not self.rig.driver:
            return
            
        blendshape_idx = selected_items[0].data(QtCore.Qt.UserRole)
        
        # Confirm deletion
        result = pm.confirmDialog(
            title='Delete Corrective',
            message=f'Delete corrective blendshape {selected_items[0].text()}?',
            button=['Yes', 'No'],
            defaultButton='No',
            cancelButton='No',
            dismissString='No'
        )
        
        if result == 'Yes':
            try:
                # Disconnect any connections
                output_plug = self.rig.driver.attr(f'correctiveOutputs[{blendshape_idx}]')
                if output_plug.isConnected():
                    output_plug.disconnect()
                    
                input_plug = self.rig.driver.attr(f'correctiveInputs[{blendshape_idx}]')
                pose_inputs = input_plug.attr('correctiveInputPose').elements()
                
                for pose_input in pose_inputs:
                    if pose_input.isConnected():
                        pose_input.disconnect()
                        
                # Note: We can't actually delete array elements in Maya,
                # so we'll just disconnect everything and leave the element empty
                
                # Refresh the UI
                self.refresh_blendshapes_list()
                self.controlling_poses_list.clear()
                
            except Exception as e:
                pm.warning(f"Failed to delete corrective blendshape: {e}")

```

## Source file: src/face_pose_ui/FacePoseUI/core/serialization/rig_serializer.py

{
  "path": "src/face_pose_ui/FacePoseUI/core/serialization/rig_serializer.py",
  "blob_sha": "287a5e0723d4cdbb598a40272196dc64dbda8895",
  "sha256": "a1be2fdd5db10fbd9b2aa4c1d8c708de564d7067aed244252beeccb2f3ea7587",
  "lines": 160,
  "bytes": 5328,
  "selection_score": 18,
  "keyword_line_counts": {
    "rig": 32,
    "face": 6
  },
  "url": "https://github.com/jign/facial_rigging/blob/1fbd80f6f051f648f3ee7fb5980dc862233954d1/src/face_pose_ui/FacePoseUI/core/serialization/rig_serializer.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, path: Path, rig: Rig):"
    },
    {
      "line": 19,
      "text": "def serialize_rig(self):"
    },
    {
      "line": 34,
      "text": "def serialize_controls(self, serialized_dict) -> Dict:"
    },
    {
      "line": 47,
      "text": "def serialize_all_poses(self, serialized_dict) -> Dict:"
    },
    {
      "line": 57,
      "text": "def get_dict_for_pose(self, pose: Pose) -> Dict:"
    },
    {
      "line": 66,
      "text": "def get_subposes_dict(self, pose: Pose) -> Dict:"
    },
    {
      "line": 78,
      "text": "def get_subpose_keys(self, subpose: Subpose) -> List:"
    },
    {
      "line": 90,
      "text": "def save_to_json(self, serialized_rig):"
    },
    {
      "line": 94,
      "text": "def validate(self) -> bool:"
    },
    {
      "line": 116,
      "text": "def __init__(self, path: Path, rig: Rig):"
    },
    {
      "line": 121,
      "text": "def load_dict_from_path(path: Path) -> Dict:"
    },
    {
      "line": 125,
      "text": "def load_rig(self):"
    },
    {
      "line": 147,
      "text": "def rebuild_rig(self):"
    }
  ]
}

```text

from FacePoseUI.core.rig import Rig
from FacePoseUI.core.pose import Pose
from FacePoseUI.core.pose import Subpose
import FacePoseUI.ui_control.pose_context as posec
import FacePoseUI.core.naming.naming_tools as nt
import pymel.core as pm
from pathlib import Path
from typing import Dict, List
import json

SERIALIZATION_VERSION = "1.1.0"


class RigSerializer:
    def __init__(self, path: Path, rig: Rig):
        self.path: Path = path
        self.rig: Rig = rig

    def serialize_rig(self):
        if not self.validate():
            pm.warning("Cannot serialize rig")
            return
        
        serialized_rig = {
            'version': SERIALIZATION_VERSION,
            'driver': self.rig.driver.getName()
            }

        self.serialize_controls(serialized_rig)
        self.serialize_all_poses(serialized_rig)

        self.save_to_json(serialized_rig)

    def serialize_controls(self, serialized_dict) -> Dict:
        controls = {}
        for control_object in self.rig.get_control_objects():
            ctrl_name = control_object.getName()
            ctrl_dict = {
                "side": nt.get_side_as_str(control_object),
                "poses": {}
            }
            controls[ctrl_name] = ctrl_dict

        serialized_dict['controls'] = controls
        return serialized_dict

    def serialize_all_poses(self, serialized_dict) -> Dict:
        for pose in self.rig.get_poses():
            pose_host = pose.get_host_obj_name()
            pose_name = pose.get_pose_name()
            pose_dict = self.get_dict_for_pose(pose)

            serialized_dict['controls'][pose_host]['poses'][pose_name] = pose_dict

        return serialized_dict

    def get_dict_for_pose(self, pose: Pose) -> Dict:
        pose_dict = {
            'min_val': pose.get_min_val(),
            'max_val': pose.get_max_val(),
            'subposes': self.get_subposes_dict(pose)
        }

        return pose_dict

    def get_subposes_dict(self, pose: Pose) -> Dict:
        subposes_dict = {}

        for subpose in pose.subposes:
            subpose_name = subpose.get_name()
            subposes_dict[subpose_name] = {
                "attr_type": subpose.get_driven_attr_type(),
                "keys": self.get_subpose_keys(subpose)
            }

        return subposes_dict

    def get_subpose_keys(self, subpose: Subpose) -> List:
        keys = []

        for i in range(0, subpose.get_num_keys()):
            key_dict = {
                "pose_weight": subpose.get_key_weight(i),
                "val": subpose.get_key_val(i)
            }
            keys.append(key_dict)

        return keys

    def save_to_json(self, serialized_rig):
        with open(self.path, "w") as outfile:
            json.dump(serialized_rig, outfile, indent=4)

    def validate(self) -> bool:
        """Validate that all required components are available for serialization."""
        if not self.path:
            pm.warning("Cannot serialize: No output path specified")
            return False
            
        if not self.rig:
            pm.warning("Cannot serialize: No rig specified")
            return False
            
        if not self.rig.driver:
            pm.warning("Cannot serialize: Rig has no driver")
            return False
            
        if not self.rig.get_control_objects():
            pm.warning("Cannot serialize: Rig has no control objects")
            return False
            
        return True


class RigDeserializer:
    def __init__(self, path: Path, rig: Rig):
        self.serialized_dict = self.load_dict_from_path(path)
        self.rig = rig

    @staticmethod
    def load_dict_from_path(path: Path) -> Dict:
        with open(path, 'r') as fp:
            return json.load(fp)

    def load_rig(self):
        for control_name in self.serialized_dict["controls"]:
            host_obj = pm.PyNode(control_name)

            poses = self.serialized_dict["controls"][control_name]["poses"]
            for pose_name in poses:
                pose_dict = self.serialized_dict["controls"][control_name]["poses"][pose_name]
                min_val = pose_dict['min_val']
                max_val = pose_dict['max_val']
                pose = posec.new_pose(self.rig, host_obj, pose_name, min_val, max_val)

                for subpose_name in pose_dict['subposes']:
                    subpose_dict = pose_dict['subposes'][subpose_name]
                    driven_attr = pm.Attribute(subpose_name)
                    subpose = self.rig.add_subposes_to_pose(pose, driven_attr)[0]

                    keys = subpose_dict['keys']
                    for key in keys:
                        key_weight = key['pose_weight']
                        key_val = key['val']
                        subpose.add_key(key_weight, key_val)

    def rebuild_rig(self):
        driver_name = self.serialized_dict['driver']
            
        # Check if a Maya node with the driver name already exists
        if pm.objExists(driver_name):
            pm.warning(f"Driver node '{driver_name}' already exists. Cannot rebuild rig.")
            return
        
        # Create the driver node
        driver = pm.createNode('facePoseDriver', name=driver_name)
        self.rig.set_driver(driver)
        
        # Load the rest of the rig
        self.load_rig()

```

## Source file: src/face_jnt_guides/FaceJointGuides/components/jaw/jaw_component.py

{
  "path": "src/face_jnt_guides/FaceJointGuides/components/jaw/jaw_component.py",
  "blob_sha": "63cff38dc77cdc3bb8aff6d6353bb6c5471e7ea2",
  "sha256": "900166704f851f41f222e6f617205e5dc504cfffb32a7a13fccedfd5bb1d0eb6",
  "lines": 209,
  "bytes": 8555,
  "selection_score": 18,
  "keyword_line_counts": {
    "jaw": 61,
    "rig": 11,
    "face": 1
  },
  "url": "https://github.com/jign/facial_rigging/blob/1fbd80f6f051f648f3ee7fb5980dc862233954d1/src/face_jnt_guides/FaceJointGuides/components/jaw/jaw_component.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self):"
    },
    {
      "line": 12,
      "text": "def draw(self):"
    },
    {
      "line": 41,
      "text": "def _snap_transform(dst: pm.PyNode, src: pm.PyNode) -> None:"
    },
    {
      "line": 51,
      "text": "def _find_direct_child(parent: pm.PyNode, child_name: str) -> pm.PyNode | None:"
    },
    {
      "line": 59,
      "text": "def _iter_point_guides(controls_root: pm.PyNode) -> list[pm.PyNode]:"
    },
    {
      "line": 91,
      "text": "def build(guide: pm.PyNode):"
    }
  ]
}

```text

from FaceJointGuides.core.component import Component
import skylib.core.icon3d as i3d
import pymel.core as pm
import maya.api.OpenMaya as om2


class JawComponent(Component):
    def __init__(self):
        super().__init__()
        self._name = "jaw"

    def draw(self):
        jaw = i3d.make_crosscube(1, 'jaw_guide', [1.0, 0.0, 0.0])
        jaw_grp = pm.group(em=1, n='jaw_grp')
        self.add_global_attrs(jaw_grp)

        # organize hierarchy -- ghostlines are visuals are for development only, to help guide the placement
        ghostline_grp = pm.group(em=1, n='jaw_ghostline_grp')
        visuals_grp = pm.group(em=1, n='jaw_visuals_grp')

        # controls group for individual jaw-driven point controls, so that they not only follow the head, but also the jaw -- these should be point controls
        controls_grp = pm.group(em=1, n='jaw_controls_grp')

        pm.parent(jaw, jaw_grp)
        pm.parent(ghostline_grp, jaw_grp)
        pm.parent(visuals_grp, jaw)
        pm.parent(controls_grp, jaw_grp)

        # Visual guides (NO JOINTS should be created for these)
        visual_guide_a = i3d.make_crossball(1, 'jaw_visual_guide_a', [1.0, 1.0, 0.0])
        pm.parent(visual_guide_a, visuals_grp)
        visual_guide_b = i3d.make_crossball(1, 'jaw_visual_guide_b', [1.0, 1.0, 0.0])
        pm.parent(visual_guide_b, visuals_grp)

        ghostline = i3d.make_ghostline('jaw_ghostline', [jaw, visual_guide_a, visual_guide_b])
        pm.parent(ghostline, ghostline_grp)
        
        return jaw_grp
    
    @staticmethod
    def _snap_transform(dst: pm.PyNode, src: pm.PyNode) -> None:
        """Match dst to src (world pos + rot) without leaving constraints behind."""
        try:
            pt = pm.pointConstraint(src, dst)
            orc = pm.orientConstraint(src, dst)
            pm.delete(pt, orc)
        except Exception:
            pass

    @staticmethod
    def _find_direct_child(parent: pm.PyNode, child_name: str) -> pm.PyNode | None:
        children = parent.listRelatives(c=True, type="transform") or []
        for ch in children:
            if ch.nodeName() == child_name:
                return ch
        return None

    @staticmethod
    def _iter_point_guides(controls_root: pm.PyNode) -> list[pm.PyNode]:
        """
        Heuristic: any transform under controls_root that has shapes is treated as a jaw point guide.
        """
        nodes = controls_root.listRelatives(ad=True, type="transform") or []
        out: list[pm.PyNode] = []
        for n in nodes:
            if n is None:
                continue
            if n.nodeName().endswith("_grp"):
                continue
            # ignore any accidental visuals/ghostlines under controls
            if "ghost" in n.nodeName().lower() or "visual" in n.nodeName().lower():
                continue
            try:
                if n.getShapes():
                    out.append(n)
            except Exception:
                continue
        # also include direct children that might not be returned as ad=True in some cases
        for n in (controls_root.listRelatives(c=True, type="transform") or []):
            if n not in out:
                try:
                    if n.getShapes() and not n.nodeName().endswith("_grp"):
                        out.append(n)
                except Exception:
                    pass
        # stable order
        out.sort(key=lambda x: x.longName())
        return out    

    @staticmethod
    def build(guide: pm.PyNode):
        """
        Expected guide layout under facial_guide_grp/<bucket>/jaw_grp:
          jaw_grp
            jaw_guide               (pivot / main jaw guide)
            jaw_controls_grp        (point guides that DO create joints)
            jaw_visuals_grp         (visual-only guides; ignored)
        """
        # bucket-based intermediate parents
        target_jnts_parent = Component.get_intermediate_jnts_grp(guide)
        target_rig_parent = Component.get_intermediate_rig_grp(guide)

        jaw_guide = JawComponent._find_direct_child(guide, "jaw_guide")
        if jaw_guide is None:
            om2.MGlobal.displayError("Jaw build: expected a child named 'jaw_guide' under jaw_grp.")
            return

        controls_grp = JawComponent._find_direct_child(guide, "jaw_controls_grp")
        if controls_grp is None:
            om2.MGlobal.displayError("Jaw build: expected a child named 'jaw_controls_grp' under jaw_grp.")
            return

        # rig organization under the same bucket as the jaw
        jaw_rig_grp_name = "jaw_rig_grp"
        if pm.objExists(jaw_rig_grp_name):
            om2.MGlobal.displayError(f"Jaw build: '{jaw_rig_grp_name}' already exists. Delete it and rebuild.")
            return
        jaw_rig_grp = pm.group(em=True, n=jaw_rig_grp_name)
        pm.parent(jaw_rig_grp, target_rig_parent)

        # --- main jaw joint + jaw locator controller ---
        jaw_pos = jaw_guide.getTranslation(space="world")

        jaw_jnt_name = "jaw_jnt_anim"
        if pm.objExists(jaw_jnt_name):
            om2.MGlobal.displayError(f"Jaw build: joint '{jaw_jnt_name}' already exists. Delete it and rebuild.")
            return
        jaw_jnt = pm.joint(target_jnts_parent, n=jaw_jnt_name, p=jaw_pos)

        jaw_ctrl_name = "jaw_ctrl"
        if pm.objExists(jaw_ctrl_name):
            om2.MGlobal.displayError(f"Jaw build: locator '{jaw_ctrl_name}' already exists. Delete it and rebuild.")
            return

        jaw_ctrl = pm.spaceLocator(n=jaw_ctrl_name)
        jaw_ctrl_grp = pm.group(jaw_ctrl, n=f"{jaw_ctrl_name}_grp")

        JawComponent._snap_transform(jaw_ctrl_grp, jaw_jnt)
        try:
            pm.makeIdentity(jaw_ctrl_grp, apply=True, t=1, r=1, s=1, n=0)
        except Exception:
            pass

        pm.parent(jaw_ctrl_grp, jaw_rig_grp)

        pm.parentConstraint(jaw_ctrl, jaw_jnt, mo=True)
        pm.scaleConstraint(jaw_ctrl, jaw_jnt)

        # --- point controls: joints under bucket jnts, locators under bucket rig,
        # but locator *drivers* follow the jaw ctrl via a follow group constraint ---
        point_guides = JawComponent._iter_point_guides(controls_grp)

        if not point_guides:
            om2.MGlobal.displayWarning("Jaw build: no point guides found under jaw_controls_grp.")
            return

        for pg in point_guides:
            pg_name = pg.nodeName()
            pg_pos = pg.getTranslation(space="world")

            try:
                pg_scale = pm.xform(pg, q=True, s=True, ws=False)
            except Exception:
                pg_scale = [1.0, 1.0, 1.0]
            # use a single scalar for joint radius (average of components)
            pg_scale_avg = sum(pg_scale) / 3.0

            # joints live under bucket intermediate jnts group (NOT under jaw_jnt)
            jnt_name = f"{pg_name}_jnt_anim"
            if pm.objExists(jnt_name):
                om2.MGlobal.displayWarning(f"Jaw build: '{jnt_name}' exists; skipping.")
                continue
            jnt = pm.joint(target_jnts_parent, n=jnt_name, p=pg_pos)
            # set joint radius to match guide scale
            try:
                jnt.radius.set(pg_scale_avg)
            except Exception:
                try:
                    pm.setAttr(f"{jnt}.radius", pg_scale_avg)
                except Exception:
                    pass            

            # locator driver hierarchy under the jaw rig group
            follow_grp = pm.group(em=True, n=f"{pg_name}_follow_grp")
            pm.xform(follow_grp, ws=True, t=[pg_pos.x, pg_pos.y, pg_pos.z])
            pm.parent(follow_grp, jaw_rig_grp)

            # Make the whole point-control driver follow the jaw controller (open/close)
            pm.parentConstraint(jaw_ctrl, follow_grp, mo=True)

            # Offset group + locator that actually drives the joint
            loc_name = f"{pg_name}_ctrl"
            loc = pm.spaceLocator(n=loc_name)
            loc_grp = pm.group(loc, n=f"{loc_name}_grp")
            JawComponent._snap_transform(loc_grp, jnt)
            # set locator local scale to match guide (applied on the locator transform and shape)
            try:
                shape = loc.getShape()
                if shape and hasattr(shape, "localScale"):
                    shape.localScale.set(pg_scale)
            except Exception:
                pass

            pm.parent(loc_grp, follow_grp)

            pm.parentConstraint(loc, jnt, mo=True)
            pm.scaleConstraint(loc, jnt)

        om2.MGlobal.displayInfo("Jaw build complete.")

```

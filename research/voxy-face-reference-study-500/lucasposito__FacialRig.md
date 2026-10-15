# Facial rendering source scan: lucasposito/FacialRig

Repository: https://github.com/lucasposito/FacialRig

Pinned source revision: 0be445c7b961f5cd96672aec33f7f043fc55447b

Discovery topic: facial rig

Description: Facial Rig Prototype

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["FacialRig/man/masks/l_eyelid.json", "FacialRig/man/masks/l_eyelid_lower.json", "FacialRig/man/masks/l_eyelid_upper.json", "FacialRig/man/masks/r_eyelid.json", "FacialRig/man/masks/r_eyelid_lower.json", "FacialRig/man/masks/r_eyelid_upper.json", "FacialRig/ox/masks/l_eyelid.json", "FacialRig/ox/masks/l_eyelid_lower.json", "FacialRig/ox/masks/l_eyelid_upper.json", "FacialRig/ox/masks/r_eyelid.json", "FacialRig/ox/masks/r_eyelid_lower.json", "FacialRig/ox/masks/r_eyelid_upper.json"], "blendshape": ["FacialRig/blendshapes.json", "FacialRig/blendshapes.py"], "mouth": ["FacialRig/man/masks/mouth.json", "FacialRig/ox/masks/mouth.json"], "rig": ["FacialRig/__init__.py", "FacialRig/blendshapes.json", "FacialRig/blendshapes.py", "FacialRig/driven_keys.json", "FacialRig/driven_keys.py", "FacialRig/driven_keys_joints.json", "FacialRig/face_board.json", "FacialRig/face_board.py", "FacialRig/face_ui.py", "FacialRig/lib.py", "FacialRig/man/FaceShapes_man.fbx", "FacialRig/man/masks/l_brows.json", "FacialRig/man/masks/l_brows_inner.json", "FacialRig/man/masks/l_brows_outer.json", "FacialRig/man/masks/l_cheek.json", "FacialRig/man/masks/l_eyelid.json", "FacialRig/man/masks/l_eyelid_lower.json", "FacialRig/man/masks/l_eyelid_upper.json", "FacialRig/man/masks/l_lips.json", "FacialRig/man/masks/l_lips_lower.json", "FacialRig/man/masks/l_lips_upper.json", "FacialRig/man/masks/l_nose.json", "FacialRig/man/masks/mouth.json", "FacialRig/man/masks/r_brows.json", "FacialRig/man/masks/r_brows_inner.json", "FacialRig/man/masks/r_brows_outer.json", "FacialRig/man/masks/r_cheek.json", "FacialRig/man/masks/r_eyelid.json", "FacialRig/man/masks/r_eyelid_lower.json", "FacialRig/man/masks/r_eyelid_upper.json"], "face": ["FacialRig/face_board.json", "FacialRig/face_board.py", "FacialRig/face_ui.py", "FacialRig/man/FaceShapes_man.fbx", "FacialRig/ox/FaceShapes_ox.fbx"], "lip": ["FacialRig/man/masks/l_lips.json", "FacialRig/man/masks/l_lips_lower.json", "FacialRig/man/masks/l_lips_upper.json", "FacialRig/man/masks/r_lips.json", "FacialRig/man/masks/r_lips_lower.json", "FacialRig/man/masks/r_lips_upper.json", "FacialRig/ox/masks/l_lips.json", "FacialRig/ox/masks/l_lips_lower.json", "FacialRig/ox/masks/l_lips_upper.json", "FacialRig/ox/masks/r_lips.json", "FacialRig/ox/masks/r_lips_lower.json", "FacialRig/ox/masks/r_lips_upper.json"]}

## Source file: FacialRig/face_ui.py

{
  "path": "FacialRig/face_ui.py",
  "blob_sha": "1c3f03fb55fd98925bfb3dca7d3b4107ac99322f",
  "sha256": "3bbc20f1aa25442fc18c2bd33544e8dfb682aeff5f40e1bc684295b056adaf5d",
  "lines": 854,
  "bytes": 32480,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 9,
    "blendshape": 18,
    "jaw": 6,
    "teeth": 7,
    "tongue": 3,
    "rig": 8,
    "face": 48,
    "lip": 1
  },
  "url": "https://github.com/lucasposito/FacialRig/blob/0be445c7b961f5cd96672aec33f7f043fc55447b/FacialRig/face_ui.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def maya_main_window():"
    },
    {
      "line": 40,
      "text": "def __init__(self):"
    },
    {
      "line": 51,
      "text": "def show_ui(cls):"
    },
    {
      "line": 61,
      "text": "def __init__(self, parent=maya_main_window()):"
    },
    {
      "line": 95,
      "text": "def closeEvent(self, event):"
    },
    {
      "line": 100,
      "text": "def set_masks_layout(self):"
    },
    {
      "line": 165,
      "text": "def set_shapes_layout(self):"
    },
    {
      "line": 191,
      "text": "def button_layout(self):"
    },
    {
      "line": 223,
      "text": "def main_layout(self):"
    },
    {
      "line": 231,
      "text": "def create_connections(self):"
    },
    {
      "line": 244,
      "text": "def highlight_item(list_item_widget, exists=True):"
    },
    {
      "line": 256,
      "text": "def check_existence(list_widget, base_head):"
    },
    {
      "line": 273,
      "text": "def check_vtx_count(main_mesh, list_widget):"
    },
    {
      "line": 285,
      "text": "def lerp(start, end, t):"
    },
    {
      "line": 289,
      "text": "def get_color_ramp(value):"
    },
    {
      "line": 303,
      "text": "def generate_rom(self):"
    },
    {
      "line": 307,
      "text": "def generate_rig(self):"
    },
    {
      "line": 311,
      "text": "def toggle_mask_mode(self):"
    },
    {
      "line": 329,
      "text": "def set_base_head(self):"
    },
    {
      "line": 355,
      "text": "def set_vtx_value(self, value=None):"
    },
    {
      "line": 403,
      "text": "def load_mask(self):"
    },
    {
      "line": 426,
      "text": "def mirror_mask(self):"
    },
    {
      "line": 468,
      "text": "def import_mask(self):"
    },
    {
      "line": 492,
      "text": "def export_mask(self):"
    },
    {
      "line": 502,
      "text": "def update_values_box(self, value):"
    },
    {
      "line": 505,
      "text": "def update_slider(self, value):"
    },
    {
      "line": 508,
      "text": "def create_blendshapes(self):"
    },
    {
      "line": 555,
      "text": "def create_curve_attributes(self):"
    },
    {
      "line": 581,
      "text": "def create_comb_data(self):"
    },
    {
      "line": 625,
      "text": "def key_control(self, node, attribute, neutral_value, target_value):"
    },
    {
      "line": 636,
      "text": "def create_rom(self):"
    },
    {
      "line": 694,
      "text": "def clean_rom(self):"
    },
    {
      "line": 724,
      "text": "def create_controls(self, keep_rom=False):"
    }
  ]
}

```text

from maya.api import OpenMaya, OpenMayaAnim
from maya import OpenMayaUI, cmds, mel

from .face_board import FaceBoard
from .blendshapes import BlendShape, BlendShapeData
from .driven_keys import DrivenKeysData, load_driven_keys
from . import lib

import MayaData

import math
import json
import numpy

from pathlib import Path
from PySide2 import QtWidgets, QtCore, QtGui
from shiboken2 import wrapInstance
from functools import partial

'''
Author: Lucas Esposito
'''


def maya_main_window():
    maya_window = OpenMayaUI.MQtUtil.mainWindow()
    return wrapInstance(int(maya_window), QtWidgets.QWidget)


class SceneScale:
    unit_scale = {
        'mm': 10.0,
        'cm': 1.0,
        'm': 0.01,
        'km': 0.00001,
        'in': 0.393701,
        'ft': 0.0328084,
        'yd': 0.0109361}

    def __init__(self):
        self.factor = SceneScale.unit_scale[cmds.currentUnit(q=True, linear=True)]


class FaceUI(QtWidgets.QDialog):
    UI_INSTANCE = None
    COLOR_SET_NAME = 'FaceRigColorSet'
    FILE_FILTER = 'Json (*.json)'
    MAYA_DIALOG = QtWidgets.QDialog(maya_main_window())

    @classmethod
    def show_ui(cls):
        if not cls.UI_INSTANCE:
            cls.UI_INSTANCE = FaceUI()

        if cls.UI_INSTANCE.isHidden():
            cls.UI_INSTANCE.show()
        else:
            cls.UI_INSTANCE.raise_()
            cls.UI_INSTANCE.activateWindow()

    def __init__(self, parent=maya_main_window()):
        super(FaceUI, self).__init__(parent)

        self.setWindowTitle("Face Rig")
        self.setWindowFlags(self.windowFlags() ^ QtCore.Qt.WindowContextHelpButtonHint)

        self.data = None
        self.face_board = None
        self.base_head = None
        self.head_skeleton = None
        self.head_skin = None
        self.teeth_skin = None
        self.masks = dict()

        self.global_scale = None

        self._rom_cache = list()

        self.anim_data = dict()
        self.comb_data = dict()
        self.face_driven_keys = dict()
        self.current_frame = 1
        self.edit_mode = False

        self.default_font = QtGui.QFont()
        self.default_font.setPointSize(14)

        self.set_masks_layout()
        self.set_shapes_layout()
        self.button_layout()

        self.main_layout()
        self.create_connections()

    def closeEvent(self, event):
        if self.edit_mode:
            self.edit_mask_button.setStyleSheet("")
            self.toggle_mask_mode()

    def set_masks_layout(self):
        self.masks_layout = QtWidgets.QVBoxLayout()
        load_layout = QtWidgets.QHBoxLayout()
        weights_layout = QtWidgets.QHBoxLayout()
        slider_layout = QtWidgets.QHBoxLayout()
        import_layout = QtWidgets.QHBoxLayout()

        self.masks_widget = QtWidgets.QListWidget()
        self.load_mesh_button = QtWidgets.QPushButton('LOAD BASE HEAD')
        self.edit_mask_button = QtWidgets.QPushButton('EDIT MODE')
        load_layout.addWidget(self.load_mesh_button)
        load_layout.addWidget(self.edit_mask_button)

        for value in [0.0, 0.25, 0.5, 0.75, 1.0]:
            button_widget = QtWidgets.QPushButton(str(value))
            weights_layout.addWidget(button_widget)
            button_widget.clicked.connect(partial(self.set_vtx_value, value))

        self.values_box = QtWidgets.QDoubleSpinBox()
        self.values_box.setMinimum(0)
        self.values_box.setMaximum(1)
        self.values_box.setSingleStep(0.01)  # Set the step for decimal values
        self.values_box.setButtonSymbols(QtWidgets.QAbstractSpinBox.NoButtons)

        self.slider = QtWidgets.QSlider()
        self.slider.setOrientation(QtCore.Qt.Horizontal)
        self.slider.setMinimum(0)
        self.slider.setMaximum(100)
        self.slider.setTickInterval(10)
        self.slider.setTickPosition(QtWidgets.QSlider.TicksBelow)
        self.slider.setMaximumWidth(150)

        self.values_box.valueChanged.connect(self.update_slider)
        self.slider.valueChanged.connect(self.update_values_box)

        self.flood_button = QtWidgets.QPushButton('FLOOD')
        slider_layout.addWidget(self.values_box)
        slider_layout.addWidget(self.slider)
        slider_layout.addWidget(self.flood_button)

        self.mirror_button = QtWidgets.QPushButton('MIRROR')
        self.import_button = QtWidgets.QPushButton('IMPORT')
        self.export_button = QtWidgets.QPushButton('EXPORT')
        import_layout.addWidget(self.import_button)
        import_layout.addWidget(self.export_button)

        masks_label = QtWidgets.QLabel('MASKS')
        masks_label.setAlignment(QtCore.Qt.AlignTop | QtCore.Qt.AlignCenter)

        self.masks_layout.addWidget(masks_label)
        self.masks_layout.addWidget(self.masks_widget)

        for mask in sorted(BlendShapeData.MASKS):
            item = QtWidgets.QListWidgetItem(mask)
            item.setFont(self.default_font)
            self.masks_widget.addItem(item)

        self.masks_widget.setCurrentItem(self.masks_widget.item(0))

        self.masks_layout.addLayout(load_layout)
        self.masks_layout.addLayout(weights_layout)
        self.masks_layout.addLayout(slider_layout)
        self.masks_layout.addWidget(self.mirror_button)
        self.masks_layout.addLayout(import_layout)

    def set_shapes_layout(self):
        self.blendshapes_layout = QtWidgets.QVBoxLayout()
        self.shapes_widget = QtWidgets.QListWidget()
        self.corrective_widget = QtWidgets.QListWidget()

        shapes_label = QtWidgets.QLabel('SHAPES')
        shapes_label.setAlignment(QtCore.Qt.AlignTop | QtCore.Qt.AlignCenter)

        correctives_label = QtWidgets.QLabel('CORRECTIVES')
        correctives_label.setAlignment(QtCore.Qt.AlignTop | QtCore.Qt.AlignCenter)

        self.blendshapes_layout.addWidget(shapes_label)
        self.blendshapes_layout.addWidget(self.shapes_widget)
        self.blendshapes_layout.addWidget(correctives_label)
        self.blendshapes_layout.addWidget(self.corrective_widget)

        for shape in BlendShapeData.SHAPES:
            item = QtWidgets.QListWidgetItem(shape)
            item.setFont(self.default_font)
            self.shapes_widget.addItem(item)

        for corrective in BlendShapeData.CORRECTIVES:
            item = QtWidgets.QListWidgetItem(corrective)
            item.setFont(self.default_font)
            self.corrective_widget.addItem(item)

    def button_layout(self):
        self.buttons = QtWidgets.QVBoxLayout()

        fields = [[QtWidgets.QLabel('HEAD JOINT'), QtWidgets.QLineEdit('Head_jnt')],
                  [QtWidgets.QLabel('FACE JOINT'), QtWidgets.QLineEdit('Face_jnt')],
                  [QtWidgets.QLabel('RIGHT EYE JOINT'), QtWidgets.QLineEdit('Eye_R_jnt')],
                  [QtWidgets.QLabel('LEFT EYE JOINT'), QtWidgets.QLineEdit('Eye_L_jnt')],
                  [QtWidgets.QLabel('JAW JOINT'), QtWidgets.QLineEdit('Jaw_jnt')],
                  [QtWidgets.QLabel('TEETH MESH'), QtWidgets.QLineEdit('Teeth_Base')],
                  [QtWidgets.QLabel('NUMBER OF FACE JOINTS'), QtWidgets.QSpinBox()]]

        self.head_field, self.face_field, self.r_eye_field, self.l_eye_field, self.jaw_field, self.teeth_field, self.n_joints = [n[-1] for n in fields]

        for widgets in fields:
            if isinstance(widgets[-1], QtWidgets.QSpinBox):
                widgets[-1].setValue(81)
            base_layout = QtWidgets.QHBoxLayout()
            widgets[-1].setMinimumHeight(20)
            widgets[-1].setFont(self.default_font)
            base_layout.addWidget(widgets[0])
            base_layout.addWidget(widgets[-1])
            self.buttons.addLayout(base_layout)

        self.rig_button = QtWidgets.QPushButton('CREATE RIG')
        self.rig_button.setMinimumSize(150, 60)

        self.rom_button = QtWidgets.QPushButton('CREATE ROM')
        self.rom_button.setMinimumSize(150, 60)

        self.buttons.addWidget(self.rig_button)
        self.buttons.addWidget(self.rom_button)

    def main_layout(self):
        main_layout = QtWidgets.QHBoxLayout(self)
        main_layout.addStretch()

        main_layout.addLayout(self.masks_layout)
        main_layout.addLayout(self.blendshapes_layout)
        main_layout.addLayout(self.buttons)

    def create_connections(self):
        self.rig_button.clicked.connect(self.generate_rig)
        self.rom_button.clicked.connect(self.generate_rom)

        self.load_mesh_button.clicked.connect(self.set_base_head)
        self.edit_mask_button.clicked.connect(self.toggle_mask_mode)
        self.flood_button.clicked.connect(self.set_vtx_value)
        self.mirror_button.clicked.connect(self.mirror_mask)
        self.import_button.clicked.connect(self.import_mask)
        self.export_button.clicked.connect(self.export_mask)
        self.masks_widget.itemSelectionChanged.connect(self.load_mask)

    @staticmethod
    def highlight_item(list_item_widget, exists=True):
        red = QtGui.QColor(255, 182, 193)
        green = QtGui.QColor(168, 216, 168)
        black = QtGui.QColor(0, 0, 0)

        list_item_widget.setForeground(black)
        if not exists:
            list_item_widget.setBackground(red)
            return
        list_item_widget.setBackground(green)

    @staticmethod
    def check_existence(list_widget, base_head):
        missing_shapes = list()
        not_matching = list()
        for row in range(list_widget.count()):
            shape_item = list_widget.item(row)
            if not cmds.objExists(shape_item.text()):
                missing_shapes.append(shape_item.text())
                FaceUI.highlight_item(shape_item, False)
                continue
            FaceUI.highlight_item(shape_item)

            unmatched = FaceUI.check_vtx_count(base_head, shape_item)
            if unmatched:
                not_matching.append(unmatched)
        return missing_shapes, not_matching

    @staticmethod
    def check_vtx_count(main_mesh, list_widget):
        main_mesh = OpenMaya.MSelectionList().add(main_mesh).getDagPath(0)
        sec_mesh = OpenMaya.MSelectionList().add(list_widget.text()).getDagPath(0)

        main_count = OpenMaya.MFnMesh(main_mesh).numVertices
        sec_count = OpenMaya.MFnMesh(sec_mesh).numVertices

        if main_count != sec_count:
            FaceUI.highlight_item(list_widget, False)
            return list_widget.text()

    @staticmethod
    def lerp(start, end, t):
        return (1.0 - t) * start + t * end

    @staticmethod
    def get_color_ramp(value):
        if value <= 0.4:
            t = value / 0.4
            return 0.0, FaceUI.lerp(0.0, 1.0, t), FaceUI.lerp(1.0, 0.0, t)
        elif value <= 0.6:
            t = (value - 0.4) / 0.2
            return FaceUI.lerp(0.0, 1.0, t), 1.0, FaceUI.lerp(1.0, 0.0, t)
        elif value <= 0.8:
            t = (value - 0.6) / 0.2
            return 1.0, FaceUI.lerp(1.0, 0.5, t), FaceUI.lerp(1.0, 0.0, t)
        else:
            t = (value - 0.8) / 0.2
            return 1.0, FaceUI.lerp(0.5, 0.0, t), 0.0

    def generate_rom(self):
        self.create_blendshapes()
        self.create_controls(True)

    def generate_rig(self):
        self.create_blendshapes()
        self.create_controls()

    def toggle_mask_mode(self):
        if not self.base_head:
            return

        self.load_mask()
        self.edit_mode = not self.edit_mode

        sel_list = OpenMaya.MSelectionList().add(self.base_head)
        OpenMaya.MGlobal.setActiveSelectionList(sel_list, OpenMaya.MGlobal.kReplaceList)
        mel.eval('toggleShadeMode;')

        if not self.edit_mode:
            self.edit_mask_button.setStyleSheet("")
            return

        green = QtGui.QColor(168, 216, 168)
        self.edit_mask_button.setStyleSheet(f"background-color: {green.name()}; color: black;")

    def set_base_head(self):
        active_list = OpenMaya.MGlobal.getActiveSelectionList()

        if active_list.isEmpty():
            print('Please select the base head mesh')
            return
        base_head = active_list.getDagPath(0)

        if not OpenMaya.MFnDagNode(base_head).typeName == 'transform':
            print('Please select a mesh type object')
            return
        try:
            base_head.extendToShape().hasFn(OpenMaya.MFn.kMesh)
        except RuntimeError:
            print('Please select a mesh type object')
            return

        self.base_head = base_head.partialPathName()
        self.head_skin = MayaData.skin.get(self.base_head)
        self.head_skeleton = MayaData.skeleton.get(list(self.head_skin.keys())[0])
        if self.teeth_field.text():
            self.teeth_skin = MayaData.skin.get(self.teeth_field.text())

        self.edit_mode = False
        self.toggle_mask_mode()

    def set_vtx_value(self, value=None):
        if not self.base_head:
            return

        base_head = OpenMaya.MSelectionList().add(self.base_head).getDagPath(0)
        sel_list = OpenMaya.MGlobal.getActiveSelectionList()
        try:
            path, comp = sel_list.getComponent(0)
        except IndexError:
            return

        if not base_head == path:
            return

        if not comp.apiType() == OpenMaya.MFn.kMeshVertComponent:
            print('Please select only vertices')
            return

        if not value:
            value = self.values_box.value()

        vtx_color = self.get_color_ramp(value)

        mesh_mfn = OpenMaya.MFnMesh(path)
        comp_mfn = OpenMaya.MFnSingleIndexedComponent(comp)
        mask = self.masks_widget.currentItem()

        if mask.text() not in self.masks:
            self.masks[mask.text()] = dict()
            neutral_color = self.get_color_ramp(0)

            all_vertices = range(mesh_mfn.numVertices)
            no_color_array = OpenMaya.MColorArray([OpenMaya.MColor(neutral_color) for _ in all_vertices])

            mesh_mfn.setVertexColors(no_color_array, all_vertices)
            self.highlight_item(self.masks_widget.currentItem())

            for vtx in all_vertices:
                self.masks[mask.text()][vtx] = 0.0

        selected_vertices = comp_mfn.getElements()

        color_array = OpenMaya.MColorArray([OpenMaya.MColor(vtx_color) for _ in selected_vertices])
        mesh_mfn.setVertexColors(color_array, selected_vertices)

        for vtx in selected_vertices:
            self.masks[mask.text()][vtx] = value

    def load_mask(self):
        sel_list = OpenMaya.MSelectionList().add(self.base_head)
        mesh_mfn = OpenMaya.MFnMesh(sel_list.getDagPath(0))

        if self.COLOR_SET_NAME in mesh_mfn.getColorSetNames():
            mesh_mfn.deleteColorSet(self.COLOR_SET_NAME)

        if not self.edit_mode:
            return

        mesh_mfn.createColorSet(self.COLOR_SET_NAME, True)
        cmds.polyColorSet(self.base_head, cs=self.COLOR_SET_NAME, ccs=True)

        mask = self.masks_widget.currentItem()
        if mask.text() not in self.masks:
            return

        color_array = OpenMaya.MColorArray()
        for color in list(self.masks[mask.text()].values()):
            color_array.append(OpenMaya.MColor(self.get_color_ramp(color)))

        mesh_mfn.setVertexColors(color_array, list(map(int, self.masks[mask.text()].keys())))

    def mirror_mask(self):
        mesh_mfn = OpenMaya.MSelectionList().add(self.base_head).getDagPath(0)
        mesh_mfn = OpenMaya.MFnMesh(mesh_mfn)

        mask_items = [self.masks_widget.item(i) for i in range(self.masks_widget.count())]

        temp_node = cmds.createNode('closestPointOnMesh')
        cmds.connectAttr(f'{self.base_head}.worldMesh', f'{temp_node}.inMesh')

        for mask in BlendShapeData.MASKS:
            if mask not in self.masks:
                continue

            side = mask.split('_')
            if side[0] != 'l':
                continue

            new_values = dict(zip(range(mesh_mfn.numVertices), numpy.zeros(mesh_mfn.numVertices)))

            target = '_'.join(['r'] + side[1:])

            for vtx, value in self.masks[mask].items():
                vtx = int(vtx)

                vtx_position = list(mesh_mfn.getPoint(vtx, OpenMaya.MSpace.kWorld))[:-1]
                if value == 0.0:
                    continue

                vtx_position[0] *= -1

                cmds.setAttr(f'{temp_node}.inPosition', *vtx_position)
                closest = cmds.getAttr(f'{temp_node}.closestVertexIndex')

                new_values[closest] = value

            self.masks[target] = new_values
            mask_widget = [mask for mask in mask_items if target == mask.text()]
            if not mask_widget:
                continue
            self.highlight_item(mask_widget[0])
        cmds.delete(temp_node)

    def import_mask(self):
        if not self.base_head:
            print('The base head mesh is missing')
            return

        file_paths, selected_filter = QtWidgets.QFileDialog.getOpenFileNames(self.MAYA_DIALOG, 'Import Masks', '',
                                                                             self.FILE_FILTER)
        if not file_paths:
            return

        mask_items = [self.masks_widget.item(i) for i in range(self.masks_widget.count())]
        for file in file_paths:
            file = Path(file)
            mask = [mask for mask in mask_items if file.stem == mask.text()]
            if not mask:
                continue
            if file.stem in self.masks:
                del self.masks[file.stem]

            self.highlight_item(mask[0])

            with open(str(file), 'r') as f:
                self.masks[file.stem] = json.loads(f.read())

    def export_mask(self):
        dir_path = QtWidgets.QFileDialog.getExistingDirectory(self.MAYA_DIALOG, 'Export Masks', QtCore.QDir.homePath())
        if not dir_path:
            return

        for mask in self.masks:
            file_path = Path(dir_path) / f'{mask}.json'
            with open(file_path, 'w') as f:
                f.write(json.dumps(self.masks[mask], indent=4))

    def update_values_box(self, value):
        self.values_box.setValue(value / 100.0)

    def update_slider(self, value):
        self.slider.setValue(value * 100)

    def create_blendshapes(self):
        self.global_scale = SceneScale()
        # active_list = OpenMaya.MGlobal.getActiveSelectionList()

        if not self.base_head:
            print('The base head mesh is missing')
            return

        # Check if all masks have data
        missing_masks = list()
        for row in range(self.masks_widget.count()):
            mask_item = self.masks_widget.item(row)
            if mask_item.text() not in self.masks:
                missing_masks.append(mask_item.text())
                self.highlight_item(mask_item, False)
                continue
            self.highlight_item(mask_item)

        if missing_masks:
            print(f'The following masks don\'t contain any data:')
            print(missing_masks)
            return

        missing_shapes = list()
        not_matching = list()

        for shapes in [self.shapes_widget, self.corrective_widget]:
            a, b = self.check_existence(shapes, self.base_head)
            missing_shapes.extend(a)
            not_matching.extend(b)

        if missing_shapes:
            print(f'The following shapes weren\'t found in the scene:')
            print(missing_shapes)
            return

        if not_matching:
            print(f'The following shapes don\'t match the vtx count:')
            print(not_matching)
            return

        if self.edit_mode:
            self.toggle_mask_mode()

        self.data = BlendShape(self.base_head, self.masks, self.global_scale.factor)
        self.data.create()

    def create_curve_attributes(self):

        direction_map = {'tx': {'positive': 'east', 'negative': 'west'},
                         'ty': {'positive': 'north', 'negative': 'south'}}

        data = dict()

        for frame in range(self.current_frame + 1):
            if frame not in self.anim_data:
                continue

            driver_data = self.anim_data[frame]
            ctr_name = driver_data['node']
            attr = driver_data['attribute']

            value = driver_data['value']

            direction = 'positive' if value > 0.0 else 'negative'
            name = f"{ctr_name}_{direction_map[attr][direction]}"
            cmds.addAttr(self.face_field.text(), longName=name, attributeType='float', minValue=0.0, maxValue=1.0, k=True)

            driven = f"{self.face_field.text()}.{name}"
            data.setdefault(f'{ctr_name}.{attr}', {}).setdefault(value, {})[driven] = 1.0

        return data

    def create_comb_data(self):

        for frame, data in self.comb_data.items():
            cmds.addAttr(self.face_field.text(), ln=data['attribute'], at='float', dv=0.0, k=True)
            attr_plug = OpenMaya.MSelectionList().add(self.face_field.text()).getDependNode(0)
            attr_plug = OpenMaya.MFnTransform(attr_plug).findPlug(data['attribute'], False)

            mod = OpenMaya.MDGModifier()
            clamp_output = list()
            for each in data['nodes']:
                ctr, limit = each
                ctr_name, ctr_attr = ctr.split('.')

                ctr_node = OpenMaya.MSelectionList().add(ctr_name).getDependNode(0)
                ctr_plug = OpenMaya.MFnTransform(ctr_node).findPlug(ctr_attr, False)

                clamp_node = mod.createNode('clamp')
                clamp_mfn = OpenMaya.MFnDependencyNode(clamp_node)
                if limit < 0:
                    min_plug = clamp_mfn.findPlug('minR', False)
                    min_plug.setFloat(limit)
                else:
                    max_plug = clamp_mfn.findPlug('maxR', False)
                    max_plug.setFloat(limit)

                input_plug = clamp_mfn.findPlug('inputR', False)
                clamp_output.append(clamp_mfn.findPlug('outputR', False))

                mod.connect(ctr_plug, input_plug)

            multiply_node = mod.createNode('multiplyDivide')
            multiply_mfn = OpenMaya.MFnDependencyNode(multiply_node)
            plug_a = multiply_mfn.findPlug('input1X', False)
            plug_b = multiply_mfn.findPlug('input2X', False)
            multiply_output = multiply_mfn.findPlug('outputX', False)

            self._rom_cache.append(multiply_node)

            clamp_a, clamp_b = clamp_output
            mod.connect(clamp_a, plug_a)
            mod.connect(clamp_b, plug_b)
            mod.connect(multiply_output, attr_plug)
            mod.doIt()

    def key_control(self, node, attribute, neutral_value, target_value):

        lib.set_key(node, attribute, neutral_value, self.current_frame)
        self.current_frame += 1

        self.anim_data[self.current_frame] = {'node': node, 'attribute': attribute, 'value': target_value}
        lib.set_key(node, attribute, target_value, self.current_frame)
        self.current_frame += 1

        lib.set_key(node, attribute, neutral_value, self.current_frame)

    def create_rom(self):
        self.current_frame = 1
        self.anim_data = dict()
        self.comb_data = dict()
        # Creates animation for each blendshape face control (excluding tongue)
        OpenMaya.MTime.setUIUnit(OpenMaya.MTime.kNTSCFrame)

        # TODO: Set Maya max frame duration
        start_frame = OpenMaya.MTime(self.current_frame, OpenMaya.MTime.uiUnit())
        OpenMayaAnim.MAnimControl.setAnimationStartTime(start_frame)
        OpenMayaAnim.MAnimControl.setMinTime(start_frame)
        OpenMayaAnim.MAnimControl.setCurrentTime(start_frame)

        excluded = ['fidget_ctr', 'head_ctr', 'lipSeal_ctr', 'tongue_ctr', 'tongue_curl_ctr',
                    'tongue_forward_ctr']

        for ctr, limits in self.face_board.controls.items():
            if ctr in excluded:
                continue

            if 'txLimits' in limits:
                if limits['txLimits'][0] != 0.0:
                    neutral_value = cmds.getAttr(f'{ctr}.tx')

                    self.key_control(ctr, 'tx', neutral_value, limits['txLimits'][0])
                    self.key_control(ctr, 'tx', neutral_value, limits['txLimits'][1])

                else:
                    self.key_control(ctr, 'tx', limits['txLimits'][0], limits['txLimits'][1])

            if 'tyLimits' in limits:
                if limits['tyLimits'][0] != 0.0:
                    neutral_value = cmds.getAttr(f'{ctr}.ty')

                    self.key_control(ctr, 'ty', neutral_value, limits['tyLimits'][0])
                    self.key_control(ctr, 'ty', neutral_value, limits['tyLimits'][1])

                else:
                    self.key_control(ctr, 'ty', limits['tyLimits'][0], limits['tyLimits'][1])

        for cor_name, cor_shapes in self.data.correctives.items():
            first, second = [DrivenKeysData.SHAPES[each] for each in cor_shapes]
            first_ctr, first_attr = first[0].split('.')
            second_ctr, second_attr = second[0].split('.')

            lib.set_key(first_ctr, first_attr, 0, self.current_frame)
            lib.set_key(second_ctr, second_attr, 0, self.current_frame)

            self.current_frame += 1
            self.comb_data[self.current_frame] = {'nodes': [first, second], 'attribute': cor_name}

            lib.set_key(first_ctr, first_attr, first[-1], self.current_frame)
            lib.set_key(second_ctr, second_attr, second[-1], self.current_frame)

            self.current_frame += 1
            lib.set_key(first_ctr, first_attr, 0, self.current_frame)
            lib.set_key(second_ctr, second_attr, 0, self.current_frame)

    def clean_rom(self):
        cmds.delete([OpenMaya.MFnDependencyNode(i).name() for i in self._rom_cache])

        last_frame = max(self.anim_data.keys())
        last_shape = self.anim_data[last_frame]
        lib.set_key(last_shape['node'], last_shape['attribute'], 0, last_frame + 1)

        for frame, data in self.comb_data.items():
            first, second = data['nodes']
            first, _ = first
            second, _ = second

            first_ctr, first_attr = first.split('.')
            lib.set_key(first_ctr, first_attr, 0, frame)

            sec_ctr, sec_attr = second.split('.')
            lib.set_key(sec_ctr, sec_attr, 0, frame)

            lib.set_key(self.face_field.text(), data['attribute'], 0, frame - 1)
            lib.set_key(self.face_field.text(), data['attribute'], 1, frame)
            lib.set_key(self.face_field.text(), data['attribute'], 0, frame + 1)

        for ctr in self.face_board.controls.keys():
            [cmds.keyTangent(f'{ctr}.{attr}', edit=True, itt='linear', ott='linear') for attr in ['tx', 'ty']]

        cmds.bakeResults(self.face_field.text(), hi='below', t=(0, self.current_frame), sm=True, pok=True)
        for i in cmds.listRelatives(self.face_field.text(), ad=1) + [self.face_field.text()]:
            cmds.delete(i, sc=True)
        cmds.delete(self.face_board.base_board)

    def create_controls(self, keep_rom=False):
        if self.face_board:
            return

        if not self.data:
            print('Couldn\'t find blendshapes')
            return

        if not self.base_head:
            print('A base blendshape head needs to be generated before applying it')
            return

        joints_list = OpenMaya.MSelectionList().add(self.jaw_field.text())

        jaw_joint = OpenMaya.MFnTransform(joints_list.getDependNode(0))
        jaw_joints = [jaw_joint.child(index) for index in range(jaw_joint.childCount())]

        joints_list.add(self.face_field.text())
        face_joint = OpenMaya.MFnTransform(joints_list.getDependNode(1))
        facial_joints = [face_joint.child(index) for index in range(face_joint.childCount()) if face_joint.child(index) not in jaw_joints]

        # TODO: Find a way to load facial joints - Mediapipe?
        # TODO: Make it more obvious face_mesh will be the final output

        face_mesh = 'Face_Base'  # TODO: temporarily hard coded

        if self.teeth_skin:
            print('loading teeth skin')

        self.face_board = FaceBoard(self.head_field.text(), self.global_scale.factor)

        self.face_board.create_controls()

        load_driven_keys(DrivenKeysData.POSES, self.global_scale.factor)

        # It can happen that rom won't work at all or will give really strange results, delete Maya prefs
        self.create_rom()

        # TODO: Work on editing the drivenkeys manually
        lib.run_dembones(self.base_head, face_mesh, self.current_frame)

        joints_anim_data = dict()
        curves_anim_data = self.create_curve_attributes()

        self.create_comb_data()

        # Check what joints are being changed when a specific control is changed
        mod = OpenMaya.MDagModifier()
        for jnt in facial_joints:
            jnt_mfn = OpenMaya.MFnTransform(jnt)
            for attr in ['tx', 'ty', 'tz', 'rx', 'ry', 'rz']:
                attr_plug = jnt_mfn.findPlug(attr, False)
                anim_mfn = OpenMayaAnim.MFnAnimCurve()

                if not anim_mfn.hasObj(attr_plug.source().node()):
                    continue

                anim_mfn.setObject(attr_plug.source().node())
                neutral_value = attr_plug.asDouble()
                if attr in ['rx', 'ry', 'rz']:
                    neutral_value = math.degrees(neutral_value)

                for frame in range(self.current_frame):
                    m_frame = OpenMaya.MTime(frame, OpenMaya.MTime.uiUnit())
                    index = anim_mfn.find(m_frame)
                    if not index:
                        continue
                    pose_value = anim_mfn.value(index)
                    if attr in ['rx', 'ry', 'rz']:
                        pose_value = math.degrees(pose_value)
                    driven = f"{jnt_mfn.partialPathName()}.{attr}"

                    difference = round(pose_value - neutral_value, 5)
                    if -1e-3 <= difference <= 1e-3:
                        continue
                    if frame in self.anim_data:
                        driver_data = self.anim_data[frame]
                        driver = f"{driver_data['node']}.{driver_data['attribute']}"
                        value = driver_data['value']
                    elif frame in self.comb_data:
                        comb_attr = self.comb_data[frame]['attribute']

                        for i in self.comb_data[frame]['nodes']:
                            ctr, val = i
                            if ctr in joints_anim_data and \
                                    val in joints_anim_data[ctr] and \
                                    driven in joints_anim_data[ctr][val]:
                                difference -= joints_anim_data[ctr][val][driven]

                        driver = f"{self.face_field.text()}.{comb_attr}"
                        value = 1
                    else:
                        continue

                    if driver not in joints_anim_data:
                        joints_anim_data[driver] = dict()
                    if value not in joints_anim_data[driver]:
                        joints_anim_data[driver][value] = dict()

                    joints_anim_data[driver][value].update({driven: difference})

                mod.deleteNode(attr_plug.source().node())
        mod.doIt()

        for attr_key in set(joints_anim_data.keys()).union(curves_anim_data.keys()):
            self.face_driven_keys[attr_key] = dict()
            for inner_key in set(joints_anim_data.get(attr_key, {}).keys()).union(
                    curves_anim_data.get(attr_key, {}).keys()):
                joint_data = joints_anim_data.get(attr_key, {}).get(inner_key, {})
                curve_data = curves_anim_data.get(attr_key, {}).get(inner_key, {})
                self.face_driven_keys[attr_key][inner_key] = {**joint_data, **curve_data}

        load_driven_keys(self.face_driven_keys, self.global_scale.factor)
        load_driven_keys(DrivenKeysData.JOINTS, self.global_scale.factor)

        if keep_rom:
            self.clean_rom()
            return

        for ctr in list(self.face_board.controls.keys()) + [self.face_field.text()]:
            lib.delete_all_keys(ctr)

        # Merge the first mesh with the output one onto a copied mesh
        merged_skin = lib.merge_skin(self.base_head, face_mesh, self.face_field.text())
        MayaData.skin.load(merged_skin, face_mesh)

        cmds.select(cl=True)

        for i in range(10):
            OpenMaya.MGlobal.displayInfo('--------------------')
        OpenMaya.MGlobal.displayInfo('--Process finished--')


```

## Source file: FacialRig/face_board.py

{
  "path": "FacialRig/face_board.py",
  "blob_sha": "1e015967e25d3b5fd3b4ce8fcc586ca0ea6b6bb3",
  "sha256": "92ac6a288408c51ca1b89cfec7725e88e7c331885bc6ef55503eed101f621de9",
  "lines": 180,
  "bytes": 5855,
  "selection_score": 18,
  "keyword_line_counts": {
    "face": 9
  },
  "url": "https://github.com/lucasposito/FacialRig/blob/0be445c7b961f5cd96672aec33f7f043fc55447b/FacialRig/face_board.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def create_shape(name, shape, degree=1, global_scale=1.0):"
    },
    {
      "line": 21,
      "text": "def set_color(node, color):"
    },
    {
      "line": 36,
      "text": "def set_transformations(node, translation=None, rotation=None, scale=None, global_scale=1.0):"
    },
    {
      "line": 45,
      "text": "def set_limits(node, txLimits=None, tyLimits=None):"
    },
    {
      "line": 54,
      "text": "def lock_n_hide(node, avoid):"
    },
    {
      "line": 114,
      "text": "def __init__(self, head_joint=None, global_scale=1.0):"
    },
    {
      "line": 124,
      "text": "def create_controls(self):"
    }
  ]
}

```text

from maya import cmds

import json
from pathlib import Path


with open(str(Path(__file__).parent / 'face_board.json'), 'r') as f:
    board_shape = json.loads(f.read())


def create_shape(name, shape, degree=1, global_scale=1.0):
    shape = [[value * global_scale for value in row] for row in shape]

    transform = cmds.curve(n=name, d=degree, p=shape)
    cmds.rename(
        cmds.listRelatives(transform, s=True, f=True), "{}Shape".format(transform)
    )
    return transform


def set_color(node, color):
    node_shape = cmds.listRelatives(node, s=True, f=True)[0]

    color_type = True if isinstance(color, list) else False

    cmds.setAttr(f'{node_shape}.overrideEnabled', 1)
    cmds.setAttr(f'{node_shape}.overrideRGBColors', color_type)
    if color_type:
        for a, b in zip(('R', 'G', 'B'), color):
            cmds.setAttr(f'{node_shape}.overrideColor{a}', b)
        return

    cmds.setAttr(f'{node_shape}.overrideColor', color)


def set_transformations(node, translation=None, rotation=None, scale=None, global_scale=1.0):
    if translation:
        cmds.xform(node, t=[value * global_scale for value in translation])
    if rotation:
        cmds.xform(node, ro=rotation)
    if scale:
        cmds.xform(node, s=[value * global_scale for value in scale])


def set_limits(node, txLimits=None, tyLimits=None):
    if txLimits:
        cmds.transformLimits(node, tx=txLimits)
        cmds.transformLimits(node, etx=(True, True))
    if tyLimits:
        cmds.transformLimits(node, ty=tyLimits)
        cmds.transformLimits(node, ety=(True, True))


def lock_n_hide(node, avoid):
    if not isinstance(avoid, list):
        avoid = [avoid]

    avoid_attributes = {
        "tx": ["tx", "translateX"],
        "translateX": ["tx", "translateX"],
        "ty": ["ty", "translateY"],
        "translateY": ["ty", "translateY"],
        "tz": ["tz", "translateZ"],
        "translateZ": ["tz", "translateZ"],
        "rx": ["rx", "rotateX"],
        "rotateX": ["rx", "rotateX"],
        "ry": ["ry", "rotateY"],
        "rotateY": ["ry", "rotateY"],
        "rz": ["rz", "rotateZ"],
        "rotateZ": ["rz", "rotateZ"],
        "sx": ["sx", "scaleX"],
        "scaleX": ["sx", "scaleX"],
        "sy": ["sy", "scaleY"],
        "scaleY": ["sy", "scaleY"],
        "sz": ["sz", "scaleZ"],
        "scaleZ": ["sz", "scaleZ"],
    }

    attributes = [
        "tx",
        "ty",
        "tz",
        "rx",
        "ry",
        "rz",
        "sx",
        "sy",
        "sz",
        "translateX",
        "translateY",
        "translateZ",
        "rotateX",
        "rotateY",
        "rotateZ",
        "scaleX",
        "scaleY",
        "scaleZ",
        "visibility",
    ]

    attrs_to_avoid = list()
    for attr in avoid:
        attrs_to_avoid.extend(
            item for item in avoid_attributes[attr] if item not in attrs_to_avoid
        )

    attributes = [item for item in attributes if item not in attrs_to_avoid]

    for attr in attributes:
        cmds.setAttr(f"{node}.{attr}", k=False, l=True, cb=False)


class FaceBoard:
    def __init__(self, head_joint=None, global_scale=1.0):
        self.suffix = "ctr"
        self.base_board = "FaceBoard"
        # self.root_joint = "Root_Jnt_Exp"
        self.head_joint = head_joint
        self.face_joint = "Face_jnt"
        
        self.current_scale = global_scale
        self.controls = dict()

    def create_controls(self):
        base_board = create_shape(f"face_board_{self.suffix}", board_shape["outline_board_shape"], global_scale=self.current_scale)
        face_board = create_shape(f"face_board_{self.suffix}_crv", board_shape["face_board_shape"], 3, global_scale=self.current_scale)

        set_color(base_board, 17)
        cmds.setAttr(f"{face_board}.overrideEnabled", 1)
        cmds.setAttr(f"{face_board}.overrideDisplayType", 2)

        controls_group = list()
        for name, attr in board_shape["controls"].items():
            if not attr:
                continue

            scale = [1.0, 1.0, 1.0]
            ratio = 0.2
            if attr["shape"] == 0:
                scale = [2.0, 2.0, 2.0]
                ratio = 0.1

            transform = cmds.group(n=f"{name}_{self.suffix}_grp", em=True)
            outer_shape = create_shape(f"{name}_{self.suffix}_crv", board_shape["shapes"][str(attr["shape"])])
            inner_shape = cmds.circle(
                n=f"{name}_{self.suffix}", r=ratio, nr=(0, 0, 1), ch=False)[0]

            cmds.parent([outer_shape, inner_shape], transform)

            set_transformations(transform, attr["translate"], attr["rotate"], scale, global_scale=self.current_scale)
            set_limits(inner_shape, attr["txLimits"], attr["tyLimits"])
            lock_n_hide(inner_shape, attr["unlocked"])

            set_color(inner_shape, 17)
            cmds.setAttr(f"{outer_shape}.overrideEnabled", 1)
            cmds.setAttr(f"{outer_shape}.overrideDisplayType", 2)

            controls_group.append(transform)
            
            self.controls[inner_shape] = dict()
            if attr["txLimits"]:
                self.controls[inner_shape]['txLimits'] = attr["txLimits"]

            if attr["tyLimits"]:
                self.controls[inner_shape]['tyLimits'] = attr["tyLimits"]
        
        cmds.parent(controls_group + [face_board], base_board)

        root_group = cmds.group(base_board, n=self.base_board)

        if not cmds.objExists(self.head_joint):
            return

        offset = [value * self.current_scale for value in [20.0, 4.0, 0.0]]
        pos = cmds.xform(self.head_joint, q=True, ws=True, t=True)
        pos = [x + y for x, y in zip(pos, offset)]
        
        cmds.xform(root_group, ws=True, s=[0.6, 0.6, 0.6])
        cmds.xform(base_board, ws=True, t=pos)
        cmds.parentConstraint(self.head_joint, base_board, mo=True, weight=1)


```

## Source file: FacialRig/blendshapes.py

{
  "path": "FacialRig/blendshapes.py",
  "blob_sha": "27a5cdd10586da043600ebb3da01ef32f0c75d30",
  "sha256": "ece88ccc9d905a368be8fe55f27b340ad33fa1bb1d5eb716884ab7fd4bea2b22",
  "lines": 253,
  "bytes": 9396,
  "selection_score": 18,
  "keyword_line_counts": {
    "blendshape": 23,
    "lip": 5
  },
  "url": "https://github.com/lucasposito/FacialRig/blob/0be445c7b961f5cd96672aec33f7f043fc55447b/FacialRig/blendshapes.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def get_blendshape(mesh):"
    },
    {
      "line": 49,
      "text": "def __init__(self, main_mesh, masks, global_scale=1.0):"
    },
    {
      "line": 62,
      "text": "def flip_symmetry(mesh, global_scale=1.0):"
    },
    {
      "line": 100,
      "text": "def get_vertices_offset(base_mesh, target_mesh):"
    },
    {
      "line": 121,
      "text": "def subtract_offset(corrective_mesh, offset):"
    },
    {
      "line": 129,
      "text": "def get_mask_plug(self, shape_index):"
    },
    {
      "line": 140,
      "text": "def set_mask(self, name, shape_index):"
    },
    {
      "line": 145,
      "text": "def set_combination_shape(self, name, shape_index, driver_targets):"
    },
    {
      "line": 171,
      "text": "def create(self):"
    },
    {
      "line": 214,
      "text": "def duplicate_n_apply_masks(self):"
    }
  ]
}

```text

from maya.api import OpenMaya
from maya import cmds, mel

from .unplug_attr import Unplugged

import json
from pathlib import Path


with open(str(Path(__file__).parent / 'blendshapes.json'), 'r') as f:
    blendshapes = json.loads(f.read())


class BlendShapeData:
    NAME = 'core_blendshapes'
    BASE = blendshapes['base']
    SHAPES = blendshapes['shapes']
    CORRECTIVES = blendshapes['correctives']
    BLENDSHAPES = blendshapes['blendshapes']
    MASKS = blendshapes['masks']


def get_blendshape(mesh):
    """_summary_

    Args:
        mesh (str): Transform node or shape node name of the mesh

    Returns:
        str: Returns the blendshape node if there's any connected to the mesh
    """
    dag = OpenMaya.MSelectionList().add(mesh).getDagPath(0)
    obj = OpenMaya.MFnMesh(dag).object()

    blend_node = None
    dag_iter = OpenMaya.MItDependencyGraph(obj,
                                           OpenMaya.MItDependencyGraph.kDownstream,
                                           OpenMaya.MItDependencyGraph.kPlugLevel)
    while not dag_iter.isDone():
        current_item = dag_iter.currentNode()
        if current_item.hasFn(OpenMaya.MFn.kBlendShape):
            blend_node = OpenMaya.MFnDependencyNode(current_item).name()
            break
        dag_iter.next()
    return blend_node


class BlendShape:
    def __init__(self, main_mesh, masks, global_scale=1.0):
        main_mesh = OpenMaya.MSelectionList().add(main_mesh).getDagPath(0)
        self.main_mesh = OpenMaya.MFnMesh(main_mesh)

        self.current_scale = global_scale

        self.blend_node = get_blendshape(self.main_mesh.name())

        self.masks_data = masks
        self.shapes = dict()
        self.correctives = dict()

    @staticmethod
    def flip_symmetry(mesh, global_scale=1.0):
        mesh_list = OpenMaya.MSelectionList().add(mesh)
        mesh_mfn = OpenMaya.MFnMesh(mesh_list.getDagPath(0))

        edge_iter = OpenMaya.MItMeshEdge(mesh_list.getDependNode(0))
        central_edge = None
        central_vtx = None

        while not edge_iter.isDone():
            vtx_1, vtx_2 = edge_iter.vertexId(0), edge_iter.vertexId(1)

            point_1 = mesh_mfn.getPoint(vtx_1, OpenMaya.MSpace.kWorld)
            point_2 = mesh_mfn.getPoint(vtx_2, OpenMaya.MSpace.kWorld)

            min, max = -0.01, 0.01

            if min <= point_1[0] <= max and min <= point_2[0] <= max:
                central_edge = f'{mesh}.e[{edge_iter.index()}]'
                central_vtx = vtx_1
                break
            edge_iter.next()

        if not central_edge:
            return

        cmds.select(central_edge, r=True)
        mel.eval('activateTopoSymmetry("{0}", {{"{1}.vtx[*]"}}, {{"{2}"}}, "vertex", "dR_symmetryFlip", 1);'.format(
            central_edge, mesh, mesh))
        cmds.select(cl=True)

        offset = mesh_mfn.getPoint(central_vtx, OpenMaya.MSpace.kWorld)[0] * -1
        offset *= global_scale

        cmds.xform(mesh, t=[offset, 0, 0])
        cmds.makeIdentity(mesh, a=True, t=True)
        cmds.xform(ztp=True)

    @staticmethod
    def get_vertices_offset(base_mesh, target_mesh):
        # base_mesh and target_mesh should match vertex IDs

        base_mesh = OpenMaya.MSelectionList().add(base_mesh).getDependNode(0)
        target_mesh = OpenMaya.MSelectionList().add(target_mesh).getDependNode(0)

        offset_vtx = dict()

        base_vtx_iter = OpenMaya.MItMeshVertex(base_mesh)
        target_vtx_iter = OpenMaya.MItMeshVertex(target_mesh)

        while not base_vtx_iter.isDone() and not target_vtx_iter.isDone():
            offset = base_vtx_iter.position() - target_vtx_iter.position()
            if not all(value == 0 for value in offset):
                offset_vtx[base_vtx_iter.index()] = offset
            base_vtx_iter.next()
            target_vtx_iter.next()

        return offset_vtx

    @staticmethod
    def subtract_offset(corrective_mesh, offset):
        dag_obj = OpenMaya.MSelectionList().add(corrective_mesh).getDagPath(0)
        mfn_mesh = OpenMaya.MFnMesh(dag_obj)

        for vtx, vector in offset.items():
            result = mfn_mesh.getPoint(vtx) + vector
            mfn_mesh.setPoint(vtx, result)

    def get_mask_plug(self, shape_index):
        blend_node = OpenMaya.MSelectionList().add(self.blend_node).getDependNode(0)
        blend_node = OpenMaya.MFnDependencyNode(blend_node)

        first_plug = blend_node.findPlug('inputTarget', False)
        target_plug = first_plug.elementByLogicalIndex(0)
        group_plug = target_plug.child(0)

        shape_plug = group_plug.elementByLogicalIndex(shape_index)
        return shape_plug.child(1)

    def set_mask(self, name, shape_index):
        plug = self.get_mask_plug(shape_index)
        for vtx, value in self.masks_data[name].items():
            plug.elementByLogicalIndex(int(vtx)).setDouble(float(value))

    def set_combination_shape(self, name, shape_index, driver_targets):
        blend_node = OpenMaya.MSelectionList().add(self.blend_node).getDependNode(0)
        blend_node = OpenMaya.MFnDependencyNode(blend_node)

        blend_plug = blend_node.findPlug('weight', False)
        shape_plug = blend_plug.elementByLogicalIndex(shape_index)

        if shape_plug.isDestination:
            return

        first_target, sec_target = driver_targets

        first_target_plug = blend_plug.elementByLogicalIndex(first_target)
        sec_target_plug = blend_plug.elementByLogicalIndex(sec_target)

        for target in [first_target_plug, sec_target_plug]:
            self.subtract_offset(name, self.get_vertices_offset(self.main_mesh.name(), target.partialName(useAlias=True)))

        cmds.blendShape(self.blend_node, edit=True, t=[self.main_mesh.name(), shape_index, name, 1.0])
        cmds.combinationShape(bs=self.blend_node, cti=shape_index, cm=0, dti=[first_target, sec_target])

        self.correctives[name] = [first_target_plug.partialName(useAlias=True), sec_target_plug.partialName(useAlias=True)]

        comb_node = shape_plug.source().node()
        OpenMaya.MDGModifier().renameNode(comb_node, f'{name}_comb').doIt()

    def create(self):
        # If the mesh shape is locked, it won't work
        if not self.blend_node:
            self.blend_node = cmds.blendShape(self.main_mesh.name(), n=BlendShapeData.NAME)[0]

        temp_shapes = list()

        for index, shape in BlendShapeData.BLENDSHAPES.items():
            index = int(index)
            (shape_name,) = shape.keys()
            (values,) = shape.values()

            mask, main_shape, first_target, sec_target, flip = values
            self.shapes[shape_name] = index

            base_name = BlendShapeData.CORRECTIVES[main_shape] if sec_target else BlendShapeData.SHAPES[main_shape]
            base_shape = cmds.rename(base_name, f'{base_name}_temp')
            target_mesh = cmds.duplicate(base_shape, n=shape_name)[0]

            if flip:
                self.flip_symmetry(target_mesh, self.current_scale)

            if not sec_target:
                cmds.blendShape(self.blend_node, edit=True, t=[self.main_mesh.name(), index, target_mesh, 1.0])

                if target_mesh in temp_shapes:
                    cmds.delete(target_mesh)
                else:
                    temp_shapes.append(target_mesh)
                cmds.rename(base_shape, base_name)

                self.set_mask(BlendShapeData.MASKS[mask], index)
                continue

            self.set_combination_shape(shape_name, index, [first_target, sec_target])

            cmds.delete(target_mesh)
            cmds.rename(base_shape, base_name)

            self.set_mask(BlendShapeData.MASKS[mask], index)

        cmds.delete(temp_shapes)

    def duplicate_n_apply_masks(self):
        blend_node = OpenMaya.MSelectionList().add(self.blend_node).getDependNode(0)
        blend_node = OpenMaya.MFnDependencyNode(blend_node)

        plug = blend_node.findPlug('weight', False)

        data = dict()
        
        for j in range(plug.numElements()):
            shape = plug.elementByPhysicalIndex(j).partialName(useAlias=True)
            index = plug.elementByPhysicalIndex(j).logicalIndex()

            with Unplugged(blend_node.name(), index, plug):
                plug.elementByPhysicalIndex(j).setFloat(1)
                data[shape] = self.main_mesh.getPoints()
                plug.elementByPhysicalIndex(j).setFloat(0)

        main_mesh = OpenMaya.MFnTransform(self.main_mesh.parent(0))
        main_mesh.findPlug('visibility', False).setBool(True)

        duplicated_mesh = cmds.duplicate(main_mesh.name(), n=f'{main_mesh.name()}_copy')[0]
        temp_blend_node = cmds.blendShape(duplicated_mesh)[0]

        for i, shape in enumerate(data.items()):
            in_scene = None
            if cmds.objExists(shape[0]):
                in_scene = cmds.rename(shape[0], 'temp')

            duplicated_temp = cmds.duplicate(duplicated_mesh, n=shape[0])[0]
            temp_dag = OpenMaya.MSelectionList().add(duplicated_temp).getDagPath(0)
            temp = OpenMaya.MFnMesh(temp_dag)

            temp.setPoints(map(OpenMaya.MPoint, shape[-1]))

            cmds.blendShape(temp_blend_node, edit=True, t=[duplicated_mesh, i, temp_dag.fullPathName(), 1.0])
            cmds.delete(temp_dag.fullPathName())
            if in_scene:
                cmds.rename(in_scene, shape[0])

        return duplicated_mesh


```

## Source file: FacialRig/lib.py

{
  "path": "FacialRig/lib.py",
  "blob_sha": "5d4e2ea6b893775322e049e72cc078c788306158",
  "sha256": "6238a20d20e85c4156e110beef87dacbb5dd4c0fa2053cc9aaba9e303b59a850",
  "lines": 175,
  "bytes": 6728,
  "selection_score": 3,
  "keyword_line_counts": {
    "skin": 16,
    "blendshape": 2,
    "lip": 1
  },
  "url": "https://github.com/lucasposito/FacialRig/blob/0be445c7b961f5cd96672aec33f7f043fc55447b/FacialRig/lib.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def create_facial_joints():"
    },
    {
      "line": 30,
      "text": "def delete_all_keys(node):"
    },
    {
      "line": 47,
      "text": "def set_key(node, attribute, value=None, frame=None):"
    },
    {
      "line": 101,
      "text": "def merge_skin(base_mesh, result_mesh, mask_jnt):"
    },
    {
      "line": 132,
      "text": "def normalize_row(row):"
    },
    {
      "line": 145,
      "text": "def run_dembones(blendshape_mesh, skinned_mesh, total_frame):"
    }
  ]
}

```text

from maya.api import OpenMaya, OpenMayaAnim
from maya import cmds

import math
import pandas as pd

import MayaData
import dem_bones


def create_facial_joints():
    playblast_options = {
        "format": "image",
        "compression": "jpg",
        "quality": 100,
        "width": 1920,
        "height": 1080,
        "forceOverwrite": True,
        "viewer": False,
        "framePadding": 4,
        "startTime": 1,
        "endTime": 1,
        "filename": "playblast_output"
    }

    # Perform playblast
    playblast_path = cmds.playblast(**playblast_options)


def delete_all_keys(node):
    """Deletes all animation from a given node

    Args:
        node (str, OpenMaya.MObject): Desired node to delete animations
    """
    obj = OpenMaya.MSelectionList().add(node).getDependNode(0)
    mod = OpenMaya.MDagModifier()
    for attr in ['tx', 'ty', 'tz', 'rx', 'ry', 'rz', 'sx', 'sy', 'sz', 'visibility']:
        attr_plug = OpenMaya.MFnTransform(obj).findPlug(attr, False)
        anim_mfn = OpenMayaAnim.MFnAnimCurve()
        if not anim_mfn.hasObj(attr_plug.source().node()):
            continue
        mod.deleteNode(attr_plug.source().node())
    mod.doIt()


def set_key(node, attribute, value=None, frame=None):
    """Set a key using the Maya Api. It will delete the key if there's an existing key at the given frame.

    Args:
        node (str, OpenMaya.MObject, OpenMaya.MPlug): Transform Node, if a plug the attribute needs to be an Int index
        attribute (str or int): Name of the attribute, or Int Index if the given node is a plug
        value (float, optional): Desired value to key the given attribute. It keys the current value if None.
        frame (int, optional): Desired frame to key the given attribute. It keys the current frame if None.

    Returns:
        OpenMayaAnim.MFnAnimCurve: Returns the animation node if there's any, otherwise it returns None
    """
    unit_scale = {'mm': 0.1, 'cm': 1.0, 'm': 1e+2}
    unit_scale = unit_scale[cmds.currentUnit(q=True, linear=True)]
    
    current_time = OpenMayaAnim.MAnimControl.currentTime()
    if frame:
        current_time = OpenMaya.MTime(frame, OpenMaya.MTime.kNTSCFrame)

    if isinstance(node, OpenMaya.MPlug):
        plug = node.elementByPhysicalIndex(attribute)
    else:
        obj = OpenMaya.MSelectionList().add(node).getDependNode(0)
        plug = OpenMaya.MFnTransform(obj).findPlug(attribute, False)

    if value:
        if plug.partialName() in ['tx', 'ty', 'tz']:
            value *= unit_scale
        if plug.partialName() in ['rx', 'ry', 'rz']:
            value = math.radians(value)
        plug.setDouble(value)
    value = plug.asDouble()
    anim_obj = plug.source().node()
    anim_mfn = OpenMayaAnim.MFnAnimCurve()

    if anim_mfn.hasObj(anim_obj):
        anim_mfn.setObject(anim_obj)
    else:
        anim_mfn.create(plug, OpenMayaAnim.MFnAnimCurve.kAnimCurveUnknown)

    index = anim_mfn.find(current_time)
    if index is None:
        index = anim_mfn.insertKey(current_time)
        anim_mfn.setValue(index, value)
        return anim_mfn

    anim_mfn.remove(index)
    if anim_mfn.numKeys:
        return anim_mfn

    OpenMaya.MDagModifier().deleteNode(anim_obj).doIt()
    plug.setDouble(value)


def merge_skin(base_mesh, result_mesh, mask_jnt):
    """ This function gets the influences of a joint in the result mesh as a mask to subtract
        from every joints influence in the base mesh, where:
            - 0.0 removes the joint influence from the base mesh entirely.
            - 1.0 keeps the full joint influence from the base mesh.
        After applying the mask, the function removes the mask joint from the result mesh, combines
        the adjusted base mesh with the result mesh, and normalizes the combined weights so that
        the total influence for each vertex sums to 1.0.

    Args:
        base_mesh (str, OpenMaya.MObject): First Mesh
        result_mesh (str, OpenMaya.MObject): Second Mesh
        mask_jnt (str, OpenMaya.MObject): A joint part of First Mesh influences

    Returns:
        MayaData.skin.SkinData: mesh weights of mesh A (minus its given mask joint)
        and mesh B merged together
    """
    base_skin = pd.DataFrame(MayaData.skin.get(base_mesh))
    result_skin = pd.DataFrame(MayaData.skin.get(result_mesh))

    # Apply the mask to result_skin in a vectorized way
    mask = result_skin[mask_jnt].clip(0.0, 1.0)  # Get mask and clip values between 0.0 and 1.0
    base_skin = base_skin.multiply(mask, axis=0)  # Apply mask to all rows in result_skin

    result_skin.drop(columns=[mask_jnt], inplace=True)

    # Merge the DataFrames horizontally
    merged_skin = pd.concat([base_skin, result_skin], axis=1)

    # Normalize each row, so it sums to 1.0, adjusting the last element if necessary
    def normalize_row(row):
        total_sum = row.sum()
        if total_sum == 0:
            return row  # Skip rows that sum to 0
        normalized_row = row / total_sum  # Normalize the row
        normalized_row.iloc[-1] += 1.0 - normalized_row.sum()  # Adjust last element to sum exactly to 1.0
        return normalized_row.round(4)  # Round to 4 decimal places

    merged_skin = merged_skin.apply(normalize_row, axis=1)

    return merged_skin.to_dict(orient='list')


def run_dembones(blendshape_mesh, skinned_mesh, total_frame):
    OpenMaya.MGlobal.displayInfo('Starting Dembones')
    dembones = dem_bones.DemBones()
    dembones.compute(skinned_mesh, blendshape_mesh, start_frame=1, end_frame=total_frame)

    for frame in range(dembones.start_frame, dembones.end_frame + 1):
        for influence in dembones.influences:
            matrix = OpenMaya.MMatrix(dembones.anim_matrix(influence, frame))
            matrix = OpenMaya.MTransformationMatrix(matrix)
            translate = matrix.translation(OpenMaya.MSpace.kWorld)
            rotate = matrix.rotation().asVector()

            set_key(influence, 'tx', translate.x, frame)
            set_key(influence, 'ty', translate.y, frame)
            set_key(influence, 'tz', translate.z, frame)
            set_key(influence, 'rx', math.degrees(rotate.x), frame)
            set_key(influence, 'ry', math.degrees(rotate.y), frame)
            set_key(influence, 'rz', math.degrees(rotate.z), frame)

    skin_cluster_fn = OpenMayaAnim.MFnSkinCluster(MayaData.skin.get_skin_cluster(skinned_mesh))

    mesh_dag = OpenMaya.MSelectionList().add(skinned_mesh).getDagPath(0)
    mesh_dag.extendToShape()

    skin_cluster_fn.setWeights(
        mesh_dag,
        OpenMaya.MObject(),
        OpenMaya.MIntArray(range(len(dembones.influences))),
        OpenMaya.MDoubleArray(dembones.weights)
    )
    OpenMaya.MGlobal.displayInfo('Dembones Finished')


```

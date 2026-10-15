# Facial rendering source scan: Lemon-XQ/human_model_viewer

Repository: https://github.com/Lemon-XQ/human_model_viewer

Pinned source revision: e1c28d57764902b5e4e5e8fdc13726dca80fa67c

Discovery topic: FLAME 3D

Description: A visualization tool of some 3d human models(smpl/smplx/flame), also supports editing models' parameters

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "ui/smpl/smpl_webuser/LICENSE.txt"]

Topic paths: {"lbs": ["ui/smpl/smpl_webuser/__pycache__/lbs.cpython-35.pyc", "ui/smpl/smpl_webuser/lbs.py"]}

## Source file: ui/smpl/smpl_webuser/lbs.py

{
  "path": "ui/smpl/smpl_webuser/lbs.py",
  "blob_sha": "7798f277ab2ea0bcdf1161fdad301e52620a64d2",
  "sha256": "f9bdf0ae0294f607718577ee266e4a9003955c69d0a46ccce640ebdf0820d642",
  "lines": 80,
  "bytes": 2837,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 2,
    "blendshape": 1,
    "lbs": 1,
    "rig": 9
  },
  "url": "https://github.com/Lemon-XQ/human_model_viewer/blob/e1c28d57764902b5e4e5e8fdc13726dca80fa67c/ui/smpl/smpl_webuser/lbs.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def global_rigid_transformation(pose, J, kintree_table, xp):"
    },
    {
      "line": 63,
      "text": "def lbs_verts_core(pose, v, J, weights, kintree_table, want_Jtr=False, xp=chumpy):"
    }
  ]
}

```text

'''
Copyright 2015 Matthew Loper, Naureen Mahmood and the Max Planck Gesellschaft.  All rights reserved.
This software is provided for research purposes only.
By using this software you agree to the terms of the SMPL Model license here http://smpl.is.tue.mpg.de/license

More information about SMPL is available here http://smpl.is.tue.mpg.
For comments or questions, please email us at: smpl@tuebingen.mpg.de


About this file:
================
This file defines linear blend skinning for the SMPL loader which 
defines the effect of bones and blendshapes on the vertices of the template mesh.

Modules included:
- global_rigid_transformation: 
  computes global rotation & translation of the model
- verts_core: [overloaded function inherited from verts.verts_core]
  computes the blending of joint-influences for each vertex based on type of skinning

'''

from .posemapper import posemap
import chumpy
import numpy as np

def global_rigid_transformation(pose, J, kintree_table, xp):
    results = {}
    pose = pose.reshape((-1,3))
    id_to_col = {kintree_table[1,i] : i for i in range(kintree_table.shape[1])}
    parent = {i : id_to_col[kintree_table[0,i]] for i in range(1, kintree_table.shape[1])}

    if xp == chumpy:
        from .posemapper import Rodrigues
        rodrigues = lambda x : Rodrigues(x)
    else:
        import cv2
        rodrigues = lambda x : cv2.Rodrigues(x)[0]

    with_zeros = lambda x : xp.vstack((x, xp.array([[0.0, 0.0, 0.0, 1.0]])))
    results[0] = with_zeros(xp.hstack((rodrigues(pose[0,:]), J[0,:].reshape((3,1)))))        
        
    for i in range(1, kintree_table.shape[1]):
        results[i] = results[parent[i]].dot(with_zeros(xp.hstack((
            rodrigues(pose[i,:]),
            ((J[i,:] - J[parent[i],:]).reshape((3,1)))
            ))))

    pack = lambda x : xp.hstack([np.zeros((4, 3)), x.reshape((4,1))])
    
    results = [results[i] for i in sorted(results.keys())]
    results_global = results

    if True:
        results2 = [results[i] - (pack(
            results[i].dot(xp.concatenate( ( (J[i,:]), 0 ) )))
            ) for i in range(len(results))]
        results = results2
    result = xp.dstack(results)
    return result, results_global


def lbs_verts_core(pose, v, J, weights, kintree_table, want_Jtr=False, xp=chumpy):
    A, A_global = global_rigid_transformation(pose, J, kintree_table, xp)
    T = A.dot(weights.T)

    rest_shape_h = xp.vstack((v.T, np.ones((1, v.shape[0]))))
        
    v =(T[:,0,:] * rest_shape_h[0, :].reshape((1, -1)) + 
        T[:,1,:] * rest_shape_h[1, :].reshape((1, -1)) + 
        T[:,2,:] * rest_shape_h[2, :].reshape((1, -1)) + 
        T[:,3,:] * rest_shape_h[3, :].reshape((1, -1))).T

    v = v[:,:3] 
    
    if not want_Jtr:
        return v
    Jtr = xp.vstack([g[:3,3] for g in A_global])
    return (v, Jtr)
    


```

## Source file: ui/util.py

{
  "path": "ui/util.py",
  "blob_sha": "b410c3553dae7766cf4d75f58a250bd742fdfa4f",
  "sha256": "6100d48d472c2039b3afd9c4422f92c5af8d52a74405ae11582c69b7fa1a87d4",
  "lines": 30,
  "bytes": 1196,
  "selection_score": 0,
  "keyword_line_counts": {
    "flame": 6,
    "lbs": 2
  },
  "url": "https://github.com/Lemon-XQ/human_model_viewer/blob/e1c28d57764902b5e4e5e8fdc13726dca80fa67c/ui/util.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def load_model(model_type='smplx',gender='m'):"
    }
  ]
}

```text

from .smpl.smpl_webuser.serialization import load_model as load_smpl_model
from .smpl.smpl_webuser.serialization import load_smplx_model, load_flame_model

base_model_folder = 'E:/DataSets/human_models/'

smpl_male_model_path = base_model_folder + 'smpl/basicmodel_m_lbs_10_207_0_v1.0.0.pkl'
smpl_female_model_path = base_model_folder + 'smpl/basicModel_f_lbs_10_207_0_v1.0.0.pkl'

smplx_male_model_path = base_model_folder + 'smplx/SMPLX_MALE.pkl'
smplx_female_model_path = base_model_folder + 'smplx/SMPLX_FEMALE.pkl'

flame_male_model_path = base_model_folder + 'flame/male_model.pkl'
flame_female_model_path = base_model_folder + 'flame/female_model.pkl'

def load_model(model_type='smplx',gender='m'):
	if model_type=='smplx':
		if gender=='m':
			return load_smplx_model(smplx_male_model_path)
		else:
		    return load_smplx_model(smplx_female_model_path)
	elif model_type=='smpl':
		if gender=='m':
			return load_smpl_model(smpl_male_model_path)
		else:
			return load_smpl_model(smpl_female_model_path)
	elif model_type=='flame':
		if gender=='m':
			return load_flame_model(flame_male_model_path)
		else:
			return load_flame_model(flame_female_model_path)

```

## Source file: ui/__init__.py

{
  "path": "ui/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Lemon-XQ/human_model_viewer/blob/e1c28d57764902b5e4e5e8fdc13726dca80fa67c/ui/__init__.py",
  "function_signature_candidates": []
}

```text



```

## Source file: ui/main_window.py

{
  "path": "ui/main_window.py",
  "blob_sha": "5726bce89160e7c3964ae19586ed7bf4ba6ea3f8",
  "sha256": "b54509e01b15896b3c43491f4b1429c547f054863074d6562a5cdcf886e2cd8b",
  "lines": 514,
  "bytes": 19065,
  "selection_score": 0,
  "keyword_line_counts": {
    "flame": 4,
    "rig": 9,
    "lip": 3
  },
  "url": "https://github.com/Lemon-XQ/human_model_viewer/blob/e1c28d57764902b5e4e5e8fdc13726dca80fa67c/ui/main_window.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self):"
    },
    {
      "line": 90,
      "text": "def showEvent(self, event):"
    },
    {
      "line": 94,
      "text": "def resizeEvent(self, event):"
    },
    {
      "line": 98,
      "text": "def closeEvent(self, event):"
    },
    {
      "line": 102,
      "text": "def draw(self):"
    },
    {
      "line": 114,
      "text": "def _draw_annotations(self, img):"
    },
    {
      "line": 137,
      "text": "def _init_model(self, gender=None):"
    },
    {
      "line": 168,
      "text": "def _init_camera(self, update_camera=False):"
    },
    {
      "line": 205,
      "text": "def _save_config_dialog(self):"
    },
    {
      "line": 226,
      "text": "def _open_config_dialog(self):"
    },
    {
      "line": 264,
      "text": "def _save_screenshot_dialog(self):"
    },
    {
      "line": 272,
      "text": "def _save_mesh_dialog(self):"
    },
    {
      "line": 282,
      "text": "def _zoom(self, event):"
    },
    {
      "line": 286,
      "text": "def _mouse_begin(self, event):"
    },
    {
      "line": 293,
      "text": "def _mouse_end(self, event):"
    },
    {
      "line": 297,
      "text": "def _move(self, event):"
    },
    {
      "line": 309,
      "text": "def _show_camera_widget(self):"
    },
    {
      "line": 313,
      "text": "def _update_model(self,id):"
    },
    {
      "line": 337,
      "text": "def _update_shape(self, id, val):"
    },
    {
      "line": 342,
      "text": "def _update_exp(self, id, val):"
    },
    {
      "line": 351,
      "text": "def _reset_shape(self):"
    },
    {
      "line": 358,
      "text": "def _reset_expression(self):"
    },
    {
      "line": 367,
      "text": "def _update_pose(self, id, val):"
    },
    {
      "line": 379,
      "text": "def _reset_pose(self):"
    },
    {
      "line": 386,
      "text": "def _update_position(self, id, val):"
    },
    {
      "line": 390,
      "text": "def _reset_position(self):"
    },
    {
      "line": 398,
      "text": "def _poses(self):"
    },
    {
      "line": 474,
      "text": "def _shapes(self):"
    },
    {
      "line": 488,
      "text": "def _expressions(self):"
    },
    {
      "line": 503,
      "text": "def _to_pixmap(im):"
    }
  ]
}

```text

import sys
import numpy as np
import cv2
import configparser

from PyQt5 import QtGui, QtWidgets
from opendr.camera import ProjectPoints, Rodrigues
from opendr.renderer import ColoredRenderer
from opendr.lighting import LambertianPointLight

from .gen.main_window import Ui_MainWindow as Ui_MainWindow_Base
from .camera_widget import Ui_CameraWidget
from .util import *

model_type_list = ['smplx','smpl','flame']

class Ui_MainWindow(QtWidgets.QMainWindow, Ui_MainWindow_Base):
    def __init__(self):
        super(self.__class__, self).__init__()
        self.setupUi(self)

        self._moving = False
        self._rotating = False
        self._mouse_begin_pos = None
        self._update_canvas = False

        self.camera = ProjectPoints(rt=np.zeros(3), t=np.zeros(3))
        self.joints2d = ProjectPoints(rt=np.zeros(3), t=np.zeros(3))
        self.frustum = {'near': 0.1, 'far': 1000., 'width': 100, 'height': 30}
        self.light = LambertianPointLight(vc=np.array([0.98, 0.98, 0.98]), light_color=np.array([1., 1., 1.]))
        # self.rn = ColoredRenderer(bgcolor=np.ones(3), frustum=self.frustum, camera=self.camera, vc=self.light,
                                  # overdraw=False)
        self.rn = ColoredRenderer()
        self.rn.set(glMode='glfw',bgcolor=np.ones(3), frustum=self.frustum, camera=self.camera, vc=self.light,
                                  overdraw=False)
        self.rn.overdraw = True
        self.rn.nsamples = 8
        self.rn.msaa = True  #Without anti-aliasing optimization often does not work.
        self.rn.initGL()
        self.rn.debug = False

        self.model_type = 'smplx'
        self.model_gender = 'm'
        self.model = None
        self._init_model()
        self.model.pose[0] = np.pi

        self.camera_widget = Ui_CameraWidget(self.camera, self.frustum, self.draw)
        self.btn_camera.clicked.connect(lambda: self._show_camera_widget())

        for key, shape in self._shapes():
            shape.valueChanged[int].connect(lambda val, k=key: self._update_shape(k, val))

        for key, exp in self._expressions():
            exp.valueChanged[int].connect(lambda val, k=key: self._update_exp(k, val))

        for key, pose in self._poses():
            pose.valueChanged[int].connect(lambda val, k=key: self._update_pose(k, val))

        self.pos_0.valueChanged[float].connect(lambda val: self._update_position(0, val))
        self.pos_1.valueChanged[float].connect(lambda val: self._update_position(1, val))
        self.pos_2.valueChanged[float].connect(lambda val: self._update_position(2, val))

        self.radio_f.pressed.connect(lambda: self._init_model('f'))
        self.radio_m.pressed.connect(lambda: self._init_model('m'))

        self.model_choose.currentIndexChanged[int].connect(lambda val: self._update_model(val))

        self.reset_pose.clicked.connect(self._reset_pose)
        self.reset_shape.clicked.connect(self._reset_shape)
        self.reset_expression.clicked.connect(self._reset_expression)
        self.reset_postion.clicked.connect(self._reset_position)

        self.canvas.wheelEvent = self._zoom
        self.canvas.mousePressEvent = self._mouse_begin
        self.canvas.mouseMoveEvent = self._move
        self.canvas.mouseReleaseEvent = self._mouse_end

        self.action_save.triggered.connect(self._save_config_dialog)
        self.action_open.triggered.connect(self._open_config_dialog)
        self.action_save_screenshot.triggered.connect(self._save_screenshot_dialog)
        self.action_save_mesh.triggered.connect(self._save_mesh_dialog)

        self.view_joints.triggered.connect(self.draw)
        self.view_joint_ids.triggered.connect(self.draw)
        self.view_bones.triggered.connect(self.draw)

        self._update_canvas = True

    def showEvent(self, event):
        self._init_camera()
        super(self.__class__, self).showEvent(event)

    def resizeEvent(self, event):
        self._init_camera()
        super(self.__class__, self).resizeEvent(event)

    def closeEvent(self, event):
        self.camera_widget.close()
        super(self.__class__, self).closeEvent(event)

    def draw(self):
        # print(self._update_canvas)
        if self._update_canvas:
            # print(self.model.r)
            img = np.array(self.rn.r)

            if self.view_joints.isChecked() or self.view_joint_ids.isChecked() or self.view_bones.isChecked():
                img = self._draw_annotations(img)

            self.canvas.setScaledContents(False)
            self.canvas.setPixmap(self._to_pixmap(img))

    def _draw_annotations(self, img):
        self.joints2d.set(t=self.camera.t, rt=self.camera.rt, f=self.camera.f, c=self.camera.c, k=self.camera.k)

        height = self.canvas.height()
        if self.view_bones.isChecked():
            kintree = self.model.kintree_table[:, 1:]
            for k in range(kintree.shape[1]):
                cv2.line(img, (int(self.joints2d.r[kintree[0, k], 0]), int(height - self.joints2d.r[kintree[0, k], 1])),
                         (int(self.joints2d.r[kintree[1, k], 0]), int(height - self.joints2d.r[kintree[1, k], 1])),
                         (0.98, 0.98, 0.98), 3)

        if self.view_joints.isChecked():
            for j in self.joints2d.r:
                jj = height-j[1] # for opengl: flipx
                cv2.circle(img, (int(j[0]), int(jj)), 5, (0.38, 0.68, 0.15), -1)

        if self.view_joint_ids.isChecked():
            for k, j in enumerate(self.joints2d.r):
                jj = height-j[1]
                cv2.putText(img, str(k), (int(j[0]), int(jj)), cv2.FONT_HERSHEY_DUPLEX, 0.6, (0.3, 0.23, 0.9), 2)

        return img

    def _init_model(self, gender=None):
        pose = None
        betas = None
        trans = None

        if self.model is not None:
            pose = self.model.pose.r
            betas = self.model.betas.r
            trans = self.model.trans.r

        if gender == None: 
            gender = self.model_gender
        else:
            self.model_gender = gender
        
        self.model = load_model(model_type=self.model_type, gender=self.model_gender)
        # print(self.model.r)

        if pose is not None:
            self.model.pose[:] = pose
            self.model.betas[:] = betas
            self.model.trans[:] = trans

        self.light.set(v=self.model, f=self.model.f, num_verts=len(self.model))
        self.rn.set(v=self.model, f=self.model.f)

        self.camera.set(v=self.model)
        self.joints2d.set(v=self.model.J_transformed)

        self.draw()

    def _init_camera(self, update_camera=False):
        w = self.canvas.width()
        h = self.canvas.height()

        if update_camera or w != self.frustum['width'] and h != self.frustum['height']:
            self.camera.set(rt=np.array([self.camera_widget.rot_0.value(), self.camera_widget.rot_1.value(),
                                         self.camera_widget.rot_2.value()]),
                            t=np.array([self.camera_widget.pos_0.value(), self.camera_widget.pos_1.value(),
                                        self.camera_widget.pos_2.value()]),
                            f=np.array([w, w]) * self.camera_widget.focal_len.value(),
                            c=np.array([w, h]) / 2.,
                            k=np.array([self.camera_widget.dist_0.value(), self.camera_widget.dist_1.value(),
                                        self.camera_widget.dist_2.value(), self.camera_widget.dist_3.value(),
                                        self.camera_widget.dist_4.value()]))

            self.frustum['width'] = w
            self.frustum['height'] = h

            self.light.set(light_pos=Rodrigues(self.camera.rt).T.dot(self.camera.t) * -10.)
            self.rn.set(frustum=self.frustum, camera=self.camera)
            flipXRotation = np.array([[1.0, 0.0, 0.0, 0.0],
                [0.0, -1.0, 0., 0.0],
                [0.0, 0., -1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]])
            self.rn.camera.openglMat = flipXRotation #this is from setupcamera in utils
            self.rn.glMode = 'glfw'
            self.rn.sharedWin = None
            self.rn.overdraw = True
            self.rn.nsamples = 8
            self.rn.msaa = True  #Without anti-aliasing optimization often does not work.
            self.rn.initGL()           
            self.rn.debug = False

            # print(self.rn.r)

            self.draw()

    def _save_config_dialog(self):
        filename, _ = QtWidgets.QFileDialog.getSaveFileName(self, 'Save config', None, 'Config File (*.ini)')
        if filename:
            with open(str(filename), 'w') as fp:
                config = configparser.ConfigParser()
                config.add_section('Model')
                config.set('Model', 'gender', self.model_gender)
                config.set('Model', 'shape', ','.join(str(s) for s in self.model.betas.r))
                config.set('Model', 'pose', ','.join(str(p) for p in self.model.pose.r))
                config.set('Model', 'translation', ','.join(str(p) for p in self.model.trans.r))

                config.add_section('Camera')
                config.set('Camera', 'translation', ','.join(str(t) for t in self.camera.t.r))
                config.set('Camera', 'rotation', ','.join(str(r) for r in self.camera.rt.r))
                config.set('Camera', 'focal_length', str(self.camera_widget.focal_len.value()))
                config.set('Camera', 'center', '{},{}'.format(self.camera_widget.center_0.value(),
                                                              self.camera_widget.center_1.value()))
                config.set('Camera', 'distortion', ','.join(str(r) for r in self.camera.k.r))

                config.write(fp)

    def _open_config_dialog(self):
        filename, _ = QtWidgets.QFileDialog.getOpenFileName(self, 'Load config', None, 'Config File (*.ini)')
        if filename:
            config = configparser.ConfigParser()
            config.read(str(filename))

            self._update_canvas = False
            self._init_model(config.get('Model', 'gender'))

            shapes = np.fromstring(config.get('Model', 'shape'), dtype=np.float64, sep=',')
            poses = np.fromstring(config.get('Model', 'pose'), dtype=np.float64, sep=',')
            position = np.fromstring(config.get('Model', 'translation'), dtype=np.float64, sep=',')

            for key, shape in self._shapes():
                val = shapes[key] / 5.0 * 50.0 + 50.0
                shape.setValue(val)
            for key, pose in self._poses():
                if key == 0:
                    val = (poses[key] - np.pi) / np.pi * 50.0 + 50.0
                else:
                    val = poses[key] / np.pi * 50.0 + 50.0
                pose.setValue(val)

            self.pos_0.setValue(position[0])
            self.pos_1.setValue(position[1])
            self.pos_2.setValue(position[2])

            cam_pos = np.fromstring(config.get('Camera', 'translation'), dtype=np.float64, sep=',')
            cam_rot = np.fromstring(config.get('Camera', 'rotation'), dtype=np.float64, sep=',')
            cam_dist = np.fromstring(config.get('Camera', 'distortion'), dtype=np.float64, sep=',')
            cam_c = np.fromstring(config.get('Camera', 'center'), dtype=np.float64, sep=',')
            cam_f = config.getfloat('Camera', 'focal_length')
            print(cam_c)
            self.camera_widget.set_values(cam_pos, cam_rot, cam_f, cam_c, cam_dist)

            self._update_canvas = True
            self.draw()

    def _save_screenshot_dialog(self):
        filename, _ = QtWidgets.QFileDialog.getSaveFileName(self, 'Save screenshot', None, 'Images (*.png *.jpg *.ppm)')
        if filename:
            img = np.array(self.rn.r)
            if self.view_joints.isChecked() or self.view_joint_ids.isChecked() or self.view_bones.isChecked():
                img = self._draw_annotations(img)
            cv2.imwrite(str(filename), np.uint8(img * 255))

    def _save_mesh_dialog(self):
        filename, _ = QtWidgets.QFileDialog.getSaveFileName(self, 'Save mesh', None, 'Mesh (*.obj)')
        if filename:
            with open(filename, 'w') as fp:
                for v in self.model.r:
                    fp.write('v %f %f %f\n' % (v[0], v[1], v[2]))

                for f in self.model.f + 1:
                    fp.write('f %d %d %d\n' % (f[0], f[1], f[2]))

    def _zoom(self, event):
        delta = -event.angleDelta().y() / 1200.0
        self.camera_widget.pos_2.setValue(self.camera_widget.pos_2.value() + delta)

    def _mouse_begin(self, event):
        if event.button() == 4: # middle
            self._moving = True
        elif event.button() == 1: # left
            self._rotating = True
        self._mouse_begin_pos = event.pos()

    def _mouse_end(self, event):
        self._moving = False
        self._rotating = False

    def _move(self, event):
        if self._moving:
            delta = event.pos() - self._mouse_begin_pos
            self.camera_widget.pos_0.setValue(self.camera_widget.pos_0.value() + delta.x() / 500.)
            self.camera_widget.pos_1.setValue(self.camera_widget.pos_1.value() + delta.y() / 500.)
            self._mouse_begin_pos = event.pos()
        elif self._rotating:
            delta = event.pos() - self._mouse_begin_pos
            self.camera_widget.rot_0.setValue(self.camera_widget.rot_0.value() + delta.y() / 300.)
            self.camera_widget.rot_1.setValue(self.camera_widget.rot_1.value() - delta.x() / 300.)
            self._mouse_begin_pos = event.pos()

    def _show_camera_widget(self):
        self.camera_widget.show()
        self.camera_widget.raise_()

    def _update_model(self,id):
        print(id)
        self.model = None
        self.model_type = model_type_list[id]

        self._update_canvas = False

        self.light = LambertianPointLight(vc=np.array([0.98, 0.98, 0.98]), light_color=np.array([1., 1., 1.]))

        self.rn.set(glMode='glfw',bgcolor=np.ones(3), frustum=self.frustum, camera=self.camera, vc=self.light,
                                  overdraw=False)

        self._init_model()
        self.model.pose[0] = np.pi
        self._init_camera(update_camera=True)

        self._reset_shape()
        self._reset_expression()
        self._reset_pose()
        self._reset_position()

        self._update_canvas = True
        self.draw()

    def _update_shape(self, id, val):
        val = (val - 50) / 50.0 * 5.0
        self.model.betas[id] = val
        self.draw()

    def _update_exp(self, id, val):
        val = (val - 50) / 50.0 * 5.0
        if self.model_type=='smplx':
            self.model.betas[10 + id] = val
            self.draw()
        elif self.model_type=='flame':
            self.model.betas[10 + id] = val
            self.draw()

    def _reset_shape(self):
        self._update_canvas = False
        for key, shape in self._shapes():
            shape.setValue(50)
        self._update_canvas = True
        self.draw()

    def _reset_expression(self):
        if self.model_type != 'smplx' and self.model_type != 'flame':
            return
        self._update_canvas = False
        for key, exp in self._expressions():
            exp.setValue(50)
        self._update_canvas = True
        self.draw()

    def _update_pose(self, id, val):
        val = (val - 50) / 50.0 * np.pi

        if id == 0:
            val += np.pi

        if self.model_type == 'flame' and id>=5*3:
            return

        self.model.pose[id] = val
        self.draw()

    def _reset_pose(self):
        self._update_canvas = False
        for key, pose in self._poses():
            pose.setValue(50)
        self._update_canvas = True
        self.draw()

    def _update_position(self, id, val):
        self.model.trans[id] = val
        self.draw()

    def _reset_position(self):
        self._update_canvas = False
        self.pos_0.setValue(0)
        self.pos_1.setValue(0)
        self.pos_2.setValue(0)
        self._update_canvas = True
        self.draw()

    def _poses(self):
        return enumerate([
            self.pose_0,
            self.pose_1,
            self.pose_2,
            self.pose_3,
            self.pose_4,
            self.pose_5,
            self.pose_6,
            self.pose_7,
            self.pose_8,
            self.pose_9,
            self.pose_10,
            self.pose_11,
            self.pose_12,
            self.pose_13,
            self.pose_14,
            self.pose_15,
            self.pose_16,
            self.pose_17,
            self.pose_18,
            self.pose_19,
            self.pose_20,
            self.pose_21,
            self.pose_22,
            self.pose_23,
            self.pose_24,
            self.pose_25,
            self.pose_26,
            self.pose_27,
            self.pose_28,
            self.pose_29,
            self.pose_30,
            self.pose_31,
            self.pose_32,
            self.pose_33,
            self.pose_34,
            self.pose_35,
            self.pose_36,
            self.pose_37,
            self.pose_38,
            self.pose_39,
            self.pose_40,
            self.pose_41,
            self.pose_42,
            self.pose_43,
            self.pose_44,
            self.pose_45,
            self.pose_46,
            self.pose_47,
            self.pose_48,
            self.pose_49,
            self.pose_50,
            self.pose_51,
            self.pose_52,
            self.pose_53,
            self.pose_54,
            self.pose_55,
            self.pose_56,
            self.pose_57,
            self.pose_58,
            self.pose_59,
            self.pose_60,
            self.pose_61,
            self.pose_62,
            self.pose_63,
            self.pose_64,
            self.pose_65,
            self.pose_66,
            self.pose_67,
            self.pose_68,
            self.pose_69,
            self.pose_70,
            self.pose_71,
        ])

    def _shapes(self):
        return enumerate([
            self.shape_0,
            self.shape_1,
            self.shape_2,
            self.shape_3,
            self.shape_4,
            self.shape_5,
            self.shape_6,
            self.shape_7,
            self.shape_8,
            self.shape_9,
        ])

    def _expressions(self):
        return enumerate([
            self.shape_10,
            self.shape_11,
            self.shape_12,
            self.shape_13,
            self.shape_14,
            self.shape_15,
            self.shape_16,
            self.shape_17,
            self.shape_18,
            self.shape_19,
        ])

    @staticmethod
    def _to_pixmap(im):
        if im.dtype == np.float32 or im.dtype == np.float64:
            im = np.uint8(im * 255)

        if len(im.shape) < 3 or im.shape[-1] == 1:
            im = cv2.cvtColor(im, cv2.COLOR_GRAY2RGB)
        else:
            im = cv2.cvtColor(im, cv2.COLOR_BGR2RGB)

        qimg = QtGui.QImage(im, im.shape[1], im.shape[0], im.strides[0], QtGui.QImage.Format_RGB888)

        return QtGui.QPixmap(qimg)


```

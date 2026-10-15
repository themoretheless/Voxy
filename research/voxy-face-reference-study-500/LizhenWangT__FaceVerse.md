# Facial rendering source scan: LizhenWangT/FaceVerse

Repository: https://github.com/LizhenWangT/FaceVerse

Pinned source revision: a012b634e4e77892895660bc7164ea27f5427623

Discovery topic: face model 3D

Description: FaceVerse: a Fine-grained and Detail-controllable 3D Face Morphable Model from a Hybrid Dataset (CVPR2022)

Reported license: BSD-2-Clause

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["LICENSE-OpenSeeFace", "faceversev3_jittor/README.md", "faceversev3_jittor/data/put_model_here", "faceversev3_jittor/data_reader.py", "faceversev3_jittor/faceverse_36.ply", "faceversev3_jittor/faceverse_cuda/FaceVerseModel.py", "faceversev3_jittor/faceverse_cuda/__init__.py", "faceversev3_jittor/faceverse_cuda/losses.py", "faceversev3_jittor/faceverse_cuda/rasterizer.py", "faceversev3_jittor/faceverse_cuda/rasterizer_cuda.py", "faceversev3_jittor/faceverse_cuda/renderer.py", "faceversev3_jittor/faceverse_cuda/transform.py", "faceversev3_jittor/fit_imgs_offline_cuda.py", "faceversev3_jittor/output/images/param/00995.npy", "faceversev3_jittor/output/images/param/01066.npy", "faceversev3_jittor/output/images/param/01184.npy", "faceversev3_jittor/output/images/param/03092.npy", "faceversev3_jittor/output/images/render/00995.png", "faceversev3_jittor/output/images/render/01066.png", "faceversev3_jittor/output/images/render/01184.png", "faceversev3_jittor/output/images/render/03092.png", "faceversev3_jittor/output/output_here", "faceversev3_jittor/output/video/track.mp4", "faceversev3_jittor/rvm.py", "faceversev3_jittor/tracking_offline_cuda.py", "faceversev3_jittor/tracking_online_cuda.py", "faceversev3_jittor/util_functions.py", "model/FaceVerseModel.py", "third_libs/OpenSeeFace/__init__.py", "third_libs/OpenSeeFace/models/benchmark.bin"]}

## Source file: model/FaceVerseModel.py

{
  "path": "model/FaceVerseModel.py",
  "blob_sha": "d62c6dcaaa9a77709c33a5beb129c4d3ba6d7fd2",
  "sha256": "4234e8a4f099a938235482f4db8a1d706001f6e14f19c8fc983574682d650cb7",
  "lines": 310,
  "bytes": 13886,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 4,
    "rig": 3,
    "face": 33
  },
  "url": "https://github.com/LizhenWangT/FaceVerse/blob/a012b634e4e77892895660bc7164ea27f5427623/model/FaceVerseModel.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, model_dict, batch_size=1,"
    },
    {
      "line": 72,
      "text": "def init_coeff_tensors(self):"
    },
    {
      "line": 101,
      "text": "def get_lms(self, vs):"
    },
    {
      "line": 105,
      "text": "def split_coeffs(self, coeffs):"
    },
    {
      "line": 115,
      "text": "def merge_coeffs(self, id_coeff, exp_coeff, tex_coeff, angles, gamma, translation):"
    },
    {
      "line": 119,
      "text": "def get_packed_tensors(self):"
    },
    {
      "line": 126,
      "text": "def forward(self, coeffs, render=True, texture=True):"
    },
    {
      "line": 168,
      "text": "def get_vs(self, id_coeff, exp_coeff):"
    },
    {
      "line": 174,
      "text": "def get_vs_lms(self, id_coeff, exp_coeff):"
    },
    {
      "line": 180,
      "text": "def get_color(self, tex_coeff):"
    },
    {
      "line": 185,
      "text": "def get_skinmask(self):"
    },
    {
      "line": 188,
      "text": "def _get_camera_pose(self, device):"
    },
    {
      "line": 192,
      "text": "def _get_p_mat(self, device):"
    },
    {
      "line": 199,
      "text": "def _get_reverse_z(self, device):"
    },
    {
      "line": 203,
      "text": "def compute_norm(self, vs, tri, point_buf):"
    },
    {
      "line": 218,
      "text": "def project_vs(self, vs):"
    },
    {
      "line": 224,
      "text": "def compute_rotation_matrix(self, angles):"
    },
    {
      "line": 254,
      "text": "def add_illumination(self, face_texture, norm, gamma):"
    },
    {
      "line": 288,
      "text": "def rigid_transform(self, vs, rot, trans):"
    },
    {
      "line": 293,
      "text": "def get_rot_tensor(self):"
    },
    {
      "line": 296,
      "text": "def get_trans_tensor(self):"
    },
    {
      "line": 299,
      "text": "def get_exp_tensor(self):"
    },
    {
      "line": 302,
      "text": "def get_tex_tensor(self):"
    },
    {
      "line": 305,
      "text": "def get_id_tensor(self):"
    },
    {
      "line": 308,
      "text": "def get_gamma_tensor(self):"
    }
  ]
}

```text

import torch
from torch import nn
import numpy as np

from model.ModelRenderer import ModelRenderer
from pytorch3d.structures import Meshes
from pytorch3d.renderer import TexturesVertex

class FaceVerseModel(nn.Module):
    def __init__(self, model_dict, batch_size=1,
                 focal=1315, img_size=256, use_simplification=False, device='cuda:0'):
        super(FaceVerseModel, self).__init__()

        self.focal = focal
        self.batch_size = batch_size
        self.img_size = img_size
        self.device = torch.device(device)

        self.p_mat = self._get_p_mat(device)
        self.reverse_z = self._get_reverse_z(device)
        self.camera_pos = self._get_camera_pose(device)
        self.rotXYZ = torch.eye(3).view(1, 3, 3).repeat(3, 1, 1).view(3, 1, 3, 3).to(self.device)

        self.renderer = ModelRenderer(self.focal, self.img_size, self.device)
        
        if use_simplification:
            self.select_id = model_dict['select_id']
            self.select_id_tris = np.vstack((self.select_id * 3, self.select_id * 3 + 1, self.select_id * 3 + 2)).transpose().flatten()
            self.skinmask = torch.tensor(model_dict['skinmask_select'], requires_grad=False, device=self.device)

            self.kp_inds = torch.tensor(model_dict['keypoints_select'].reshape(-1, 1), requires_grad=False).squeeze().long().to(self.device)

            self.meanshape = torch.tensor(model_dict['meanshape'].reshape(1, -1)[:, self.select_id_tris], dtype=torch.float32, requires_grad=False, device=self.device)
            self.meantex = torch.tensor(model_dict['meantex'].reshape(1, -1)[:, self.select_id_tris], dtype=torch.float32, requires_grad=False, device=self.device)

            self.idBase = torch.tensor(model_dict['idBase'][self.select_id_tris], dtype=torch.float32, requires_grad=False, device=self.device)
            self.expBase = torch.tensor(model_dict['exBase'][self.select_id_tris], dtype=torch.float32, requires_grad=False, device=self.device)
            self.texBase = torch.tensor(model_dict['texBase'][self.select_id_tris], dtype=torch.float32, requires_grad=False, device=self.device)

            self.tri = torch.tensor(model_dict['tri_select'], dtype=torch.int64, requires_grad=False, device=self.device)
            self.point_buf = torch.tensor(model_dict['point_buf_select'], dtype=torch.int64, requires_grad=False, device=self.device)
        
        else:
            self.skinmask = torch.tensor(model_dict['skinmask'], requires_grad=False, device=self.device)

            self.kp_inds = torch.tensor(model_dict['keypoints'].reshape(-1, 1), requires_grad=False).squeeze().long().to(self.device)

            self.meanshape = torch.tensor(model_dict['meanshape'].reshape(1, -1), dtype=torch.float32, requires_grad=False, device=self.device)
            self.meantex = torch.tensor(model_dict['meantex'].reshape(1, -1), dtype=torch.float32, requires_grad=False, device=self.device)

            self.idBase = torch.tensor(model_dict['idBase'], dtype=torch.float32, requires_grad=False, device=self.device)
            self.expBase = torch.tensor(model_dict['exBase'], dtype=torch.float32, requires_grad=False, device=self.device)
            self.texBase = torch.tensor(model_dict['texBase'], dtype=torch.float32, requires_grad=False, device=self.device)

            self.tri = torch.tensor(model_dict['tri'], dtype=torch.int64, requires_grad=False, device=self.device)
            self.point_buf = torch.tensor(model_dict['point_buf'], dtype=torch.int64, requires_grad=False, device=self.device)

        self.num_vertex = self.meanshape.shape[1] // 3
        self.id_dims = self.idBase.shape[1]
        self.tex_dims = self.texBase.shape[1]
        self.exp_dims = self.expBase.shape[1]
        self.all_dims = self.id_dims + self.tex_dims + self.exp_dims

        self.init_coeff_tensors()
        
        # for tracking by landmarks
        self.kp_inds_view = torch.cat([self.kp_inds[:, None] * 3, self.kp_inds[:, None] * 3 + 1, self.kp_inds[:, None] * 3 + 2], dim=1).flatten()
        self.idBase_view = self.idBase[self.kp_inds_view, :].detach().clone()
        self.expBase_view = self.expBase[self.kp_inds_view, :].detach().clone()
        self.meanshape_view = self.meanshape[:, self.kp_inds_view].detach().clone()

    def init_coeff_tensors(self):
        self.id_tensor = torch.zeros(
            (self.batch_size, self.id_dims), dtype=torch.float32,
            requires_grad=True, device=self.device)

        self.tex_tensor = torch.zeros(
            (self.batch_size, self.tex_dims), dtype=torch.float32,
            requires_grad=True, device=self.device)

        self.exp_tensor = torch.zeros(
            (self.batch_size, self.exp_dims), dtype=torch.float32,
            requires_grad=True, device=self.device)
    
        self.gamma_tensor = torch.zeros(
            (self.batch_size, 27), dtype=torch.float32,
            requires_grad=True, device=self.device)
    
        self.trans_tensor = torch.zeros(
            (self.batch_size, 3), dtype=torch.float32,
            requires_grad=False, device=self.device)
        self.trans_tensor[:, 2] += 6
        self.trans_tensor.requires_grad = True
    
        self.rot_tensor = torch.zeros(
            (self.batch_size, 3), dtype=torch.float32,
            requires_grad=False, device=self.device)
        self.rot_tensor[:, 0] += torch.pi
        self.rot_tensor.requires_grad = True

    def get_lms(self, vs):
        lms = vs[:, self.kp_inds, :]
        return lms

    def split_coeffs(self, coeffs):
        id_coeff = coeffs[:, :self.id_dims]  # identity(shape) coeff 
        exp_coeff = coeffs[:, self.id_dims:self.id_dims+self.exp_dims]  # expression coeff 
        tex_coeff = coeffs[:, self.id_dims+self.exp_dims:self.all_dims]  # texture(albedo) coeff 
        angles = coeffs[:, self.all_dims:self.all_dims+3] # ruler angles(x,y,z) for rotation of dim 3
        gamma = coeffs[:, self.all_dims+3:self.all_dims+30] # lighting coeff for 3 channel SH function of dim 27
        translation = coeffs[:, self.all_dims+30:]  # translation coeff of dim 3

        return id_coeff, exp_coeff, tex_coeff, angles, gamma, translation

    def merge_coeffs(self, id_coeff, exp_coeff, tex_coeff, angles, gamma, translation):
        coeffs = torch.cat([id_coeff, exp_coeff, tex_coeff, angles, gamma, translation], dim=1)
        return coeffs

    def get_packed_tensors(self):
        return self.merge_coeffs(self.id_tensor,
                                 self.exp_tensor,
                                 self.tex_tensor,
                                 self.rot_tensor, self.gamma_tensor,
                                 self.trans_tensor)

    def forward(self, coeffs, render=True, texture=True):
        id_coeff, exp_coeff, tex_coeff, angles, gamma, translation = self.split_coeffs(coeffs)
        rotation = self.compute_rotation_matrix(angles)

        if render:
            vs = self.get_vs(id_coeff, exp_coeff)
            vs_t = self.rigid_transform(vs, rotation, translation)

            lms_t = self.get_lms(vs_t)
            lms_proj = self.project_vs(lms_t)
            lms_proj = torch.stack(
                [lms_proj[:, :, 0], self.img_size-lms_proj[:, :, 1]], dim=2)
            face_texture = self.get_color(tex_coeff)
            face_norm = self.compute_norm(vs, self.tri, self.point_buf)
            face_norm_r = face_norm.bmm(rotation)
            face_color = self.add_illumination(face_texture, face_norm_r, gamma)

            if texture:
                face_color_tv = TexturesVertex(face_color)
                mesh = Meshes(vs_t, self.tri.repeat(self.batch_size, 1, 1), face_color_tv)
                rendered_img = self.renderer.alb_renderer(mesh)
            else:
                face_color_tv = TexturesVertex(face_color * 0 + 200)
                mesh = Meshes(vs_t, self.tri.repeat(self.batch_size, 1, 1), face_color_tv)
                rendered_img = self.renderer.sha_renderer(mesh)

            return {'rendered_img': rendered_img,
                    'lms_proj': lms_proj,
                    'face_texture': face_texture,
                    'vs': vs_t,
                    'tri': self.tri,
                    'color': face_color}
        else:
            lms = self.get_vs_lms(id_coeff, exp_coeff)
            lms_t = self.rigid_transform(
                lms, rotation, translation)

            lms_proj = self.project_vs(lms_t)
            lms_proj = torch.stack(
                [lms_proj[:, :, 0], self.img_size-lms_proj[:, :, 1]], dim=2)
            return {'lms_proj': lms_proj}

    def get_vs(self, id_coeff, exp_coeff):
        face_shape = torch.einsum('ij,aj->ai', self.idBase, id_coeff) + \
            torch.einsum('ij,aj->ai', self.expBase, exp_coeff) + self.meanshape
        face_shape = face_shape.view(self.batch_size, -1, 3)
        return face_shape

    def get_vs_lms(self, id_coeff, exp_coeff):
        face_shape = torch.einsum('ij,aj->ai', self.idBase_view, id_coeff) + \
            torch.einsum('ij,aj->ai', self.expBase_view, torch.abs(exp_coeff)) + self.meanshape_view
        face_shape = face_shape.view(self.batch_size, -1, 3)
        return face_shape

    def get_color(self, tex_coeff):
        face_texture = torch.einsum('ij,aj->ai', self.texBase, tex_coeff) + self.meantex
        face_texture = face_texture.view(self.batch_size, -1, 3)
        return face_texture

    def get_skinmask(self):
        return self.skinmask

    def _get_camera_pose(self, device):
        camera_pos = torch.tensor([0.0, 0.0, 10.0], device=device).reshape(1, 1, 3)
        return camera_pos

    def _get_p_mat(self, device):
        half_image_width = self.img_size // 2
        p_matrix = np.array([self.focal, 0.0, half_image_width,
                             0.0, self.focal, half_image_width,
                             0.0, 0.0, 1.0], dtype=np.float32).reshape(1, 3, 3)
        return torch.tensor(p_matrix, device=device)

    def _get_reverse_z(self, device):
        reverse_z = np.reshape(np.array([1.0, 0, 0, 0, 1, 0, 0, 0, -1.0], dtype=np.float32), [1, 3, 3])
        return torch.tensor(reverse_z, device=device)

    def compute_norm(self, vs, tri, point_buf):
        face_id = tri
        point_id = point_buf
        v1 = vs[:, face_id[:, 0], :]
        v2 = vs[:, face_id[:, 1], :]
        v3 = vs[:, face_id[:, 2], :]
        e1 = v1 - v2
        e2 = v2 - v3
        face_norm = e1.cross(e2)

        v_norm = face_norm[:, point_id, :].sum(2)
        v_norm = v_norm / (v_norm.norm(dim=2).unsqueeze(2) + 1e-9)

        return v_norm

    def project_vs(self, vs):
        vs = torch.matmul(vs, self.reverse_z.repeat((self.batch_size, 1, 1))) + self.camera_pos
        aug_projection = torch.matmul(vs, self.p_mat.repeat((self.batch_size, 1, 1)).permute((0, 2, 1)))
        face_projection = aug_projection[:, :, :2] / torch.reshape(aug_projection[:, :, 2], [self.batch_size, -1, 1])
        return face_projection

    def compute_rotation_matrix(self, angles):
        sinx = torch.sin(angles[:, 0])
        siny = torch.sin(angles[:, 1])
        sinz = torch.sin(angles[:, 2])
        cosx = torch.cos(angles[:, 0])
        cosy = torch.cos(angles[:, 1])
        cosz = torch.cos(angles[:, 2])

        if self.batch_size != 1:
            rotXYZ = self.rotXYZ.repeat(1, self.batch_size, 1, 1)
        else:
            rotXYZ = self.rotXYZ.detach().clone()
        
        rotXYZ[0, :, 1, 1] = cosx
        rotXYZ[0, :, 1, 2] = -sinx
        rotXYZ[0, :, 2, 1] = sinx
        rotXYZ[0, :, 2, 2] = cosx
        rotXYZ[1, :, 0, 0] = cosy
        rotXYZ[1, :, 0, 2] = siny
        rotXYZ[1, :, 2, 0] = -siny
        rotXYZ[1, :, 2, 2] = cosy
        rotXYZ[2, :, 0, 0] = cosz
        rotXYZ[2, :, 0, 1] = -sinz
        rotXYZ[2, :, 1, 0] = sinz
        rotXYZ[2, :, 1, 1] = cosz

        rotation = rotXYZ[2].bmm(rotXYZ[1]).bmm(rotXYZ[0])

        return rotation.permute(0, 2, 1)

    def add_illumination(self, face_texture, norm, gamma):
        gamma = gamma.view(-1, 3, 9).clone()
        gamma[:, :, 0] += 0.8
        gamma = gamma.permute(0, 2, 1)

        a0 = np.pi
        a1 = 2 * np.pi / np.sqrt(3.0)
        a2 = 2 * np.pi / np.sqrt(8.0)
        c0 = 1 / np.sqrt(4 * np.pi)
        c1 = np.sqrt(3.0) / np.sqrt(4 * np.pi)
        c2 = 3 * np.sqrt(5.0) / np.sqrt(12 * np.pi)
        d0 = 0.5 / np.sqrt(3.0)

        norm = norm.view(-1, 3)
        nx, ny, nz = norm[:, 0], norm[:, 1], norm[:, 2]
        arrH = []

        arrH.append(a0 * c0 * (nx * 0 + 1))
        arrH.append(-a1 * c1 * ny)
        arrH.append(a1 * c1 * nz)
        arrH.append(-a1 * c1 * nx)
        arrH.append(a2 * c2 * nx * ny)
        arrH.append(-a2 * c2 * ny * nz)
        arrH.append(a2 * c2 * d0 * (3 * nz.pow(2) - 1))
        arrH.append(-a2 * c2 * nx * nz)
        arrH.append(a2 * c2 * 0.5 * (nx.pow(2) - ny.pow(2)))

        H = torch.stack(arrH, 1)
        Y = H.view(self.batch_size, face_texture.shape[1], 9)
        lighting = Y.bmm(gamma)

        face_color = face_texture * lighting
        return face_color

    def rigid_transform(self, vs, rot, trans):
        vs_r = torch.matmul(vs, rot)
        vs_t = vs_r + trans.view(-1, 1, 3)
        return vs_t

    def get_rot_tensor(self):
        return self.rot_tensor

    def get_trans_tensor(self):
        return self.trans_tensor

    def get_exp_tensor(self):
        return self.exp_tensor

    def get_tex_tensor(self):
        return self.tex_tensor

    def get_id_tensor(self):
        return self.id_tensor

    def get_gamma_tensor(self):
        return self.gamma_tensor



```

## Source file: third_libs/OpenSeeFace/retinaface.py

{
  "path": "third_libs/OpenSeeFace/retinaface.py",
  "blob_sha": "c97c95785989f2de65d840e8cfb9e75193080efc",
  "sha256": "3017a576408feae775f3f6a8e0d90824650f543788a459f0ebe9244e33330c4b",
  "lines": 149,
  "bytes": 4972,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "face": 5
  },
  "url": "https://github.com/LizhenWangT/FaceVerse/blob/a012b634e4e77892895660bc7164ea27f5427623/third_libs/OpenSeeFace/retinaface.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def py_cpu_nms(dets, thresh):"
    },
    {
      "line": 45,
      "text": "def decode(loc, priors, variances):"
    },
    {
      "line": 55,
      "text": "def worker_thread(rfd, frame):"
    },
    {
      "line": 62,
      "text": "def __init__(self, model_path=\"models/retinaface_640x640_opt.onnx\", json_path=\"models/priorbox_640x640.json\", threads=4, min_conf=0.4, nms_threshold=0.4, top_k=1, res=(640, 640)):"
    },
    {
      "line": 80,
      "text": "def detect_retina(self, frame, is_background=False):"
    },
    {
      "line": 116,
      "text": "def background_detect(self, frame):"
    },
    {
      "line": 124,
      "text": "def get_results(self):"
    }
  ]
}

```text

import sys
import numpy as np
import cv2
import onnxruntime
import time
import queue
import threading
import json
import copy

def py_cpu_nms(dets, thresh):
    """ Pure Python NMS baseline.
        Copyright (c) 2015 Microsoft
        Licensed under The MIT License
        Written by Ross Girshick
    """
    x1 = dets[:, 0]
    y1 = dets[:, 1]
    x2 = dets[:, 2]
    y2 = dets[:, 3]
    scores = dets[:, 4]

    areas = (x2 - x1 + 1) * (y2 - y1 + 1)
    order = scores.argsort()[::-1]

    keep = []
    while order.size > 0:
        i = order[0]
        keep.append(i)
        xx1 = np.maximum(x1[i], x1[order[1:]])
        yy1 = np.maximum(y1[i], y1[order[1:]])
        xx2 = np.minimum(x2[i], x2[order[1:]])
        yy2 = np.minimum(y2[i], y2[order[1:]])

        w = np.maximum(0.0, xx2 - xx1 + 1)
        h = np.maximum(0.0, yy2 - yy1 + 1)
        inter = w * h
        ovr = inter / (areas[i] + areas[order[1:]] - inter)

        inds = np.where(ovr <= thresh)[0]
        order = order[inds + 1]

    return keep

def decode(loc, priors, variances):
    data = (
        priors[:, :2] + loc[:, :2] * variances[0] * priors[:, 2:],
        priors[:, 2:] * np.exp(loc[:, 2:] * variances[1])
    )
    boxes = np.concatenate(data, 1)
    boxes[:, :2] -= boxes[:, 2:] / 2
    boxes[:, 2:] += boxes[:, :2]
    return boxes

def worker_thread(rfd, frame):
    results = rfd.detect_retina(frame, is_background=True)
    rfd.results.put(results, False)
    rfd.finished = True
    rfd.running = False

class RetinaFaceDetector():
    def __init__(self, model_path="models/retinaface_640x640_opt.onnx", json_path="models/priorbox_640x640.json", threads=4, min_conf=0.4, nms_threshold=0.4, top_k=1, res=(640, 640)):
        options = onnxruntime.SessionOptions()
        options.inter_op_num_threads = 1
        options.intra_op_num_threads = threads
        options.execution_mode = onnxruntime.ExecutionMode.ORT_SEQUENTIAL
        options.graph_optimization_level = onnxruntime.GraphOptimizationLevel.ORT_ENABLE_ALL
        options.log_severity_level = 3
        self.session = onnxruntime.InferenceSession(model_path, sess_options=options)
        self.res_w, self.res_h = res
        with open(json_path, "r") as prior_file:
            self.priorbox = np.array(json.loads(prior_file.read()))
        self.min_conf = min_conf
        self.nms_threshold = nms_threshold
        self.top_k = top_k
        self.finished = False
        self.running = False
        self.results = queue.Queue()

    def detect_retina(self, frame, is_background=False):
        h, w, _ = frame.shape
        im = None
        im = cv2.resize(frame, (self.res_w, self.res_h), interpolation=cv2.INTER_LINEAR)
        resize_w = w / self.res_w
        resize_w = 1 / resize_w
        resize_h = h / self.res_h
        resize_h = 1 / resize_h
        im = np.float32(im)
        scale = np.array((self.res_w / resize_w, self.res_h / resize_h, self.res_w / resize_w, self.res_h / resize_h))
        im -= (104, 117, 123)
        im = im.transpose(2, 0, 1)
        im = np.expand_dims(im, 0)
        output = self.session.run([], {"input0": im})
        loc, conf = output[0][0], output[1][0]
        boxes = decode(loc, self.priorbox, [0.1, 0.2])
        boxes = boxes * scale
        scores = conf[:, 1]

        inds = np.where(scores > self.min_conf)[0]
        boxes = boxes[inds]
        scores = scores[inds]

        dets = np.hstack((boxes, scores[:, np.newaxis])).astype(np.float32, copy=False)
        keep = py_cpu_nms(dets, self.nms_threshold)
        dets = dets[keep, :]
        dets = dets[:self.top_k, 0:4]
        dets[:, 2:4] = dets[:, 2:4] - dets[:, 0:2]

        if True:#is_background:
            upsize = dets[:, 2:4] * np.array([[0.15, 0.2]])
            dets[:, 0:2] -= upsize
            dets[:, 2:4] += upsize * 2

        return list(map(tuple, dets))

    def background_detect(self, frame):
        if self.running or self.finished:
            return
        self.running = True
        im = copy.copy(frame)
        thread = threading.Thread(target=worker_thread, args=(self, im))
        thread.start()

    def get_results(self):
        if self.finished:
            results = []
            try:
                while True:
                    detection = self.results.get(False)
                    results.append(detection)
            except:
                "No error"
            self.finished = False
            return list(*results)
        else:
            return []

if __name__== "__main__":
    retina = RetinaFaceDetector(top_k=40, min_conf=0.2)
    im = cv2.imread(sys.argv[1], cv2.IMREAD_COLOR)
    start = time.perf_counter()
    faces = retina.detect_retina(im)
    end = 1000 * (time.perf_counter() - start)
    print(f"Runtime: {end:.3f}ms")
    for (x,y,w,h) in faces:
        im = cv2.rectangle(im, (int(x),int(y)), (int(x+w),int(y+w)), (0,0,255), 1)
    cv2.imshow("Frame", im)
    while cv2.waitKey(0) & 0xff != ord('q'):
        ""


```

## Source file: faceversev3_jittor/faceverse_cuda/FaceVerseModel.py

{
  "path": "faceversev3_jittor/faceverse_cuda/FaceVerseModel.py",
  "blob_sha": "f8fe4805351c873730083186a092dbbfdb4c8ca9",
  "sha256": "e1a691f9c5305a5305729d172e86250f51a07558a93265c88c057f3a7d668ce2",
  "lines": 325,
  "bytes": 15438,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 39,
    "rig": 6,
    "face": 30
  },
  "url": "https://github.com/LizhenWangT/FaceVerse/blob/a012b634e4e77892895660bc7164ea27f5427623/faceversev3_jittor/faceverse_cuda/FaceVerseModel.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, model_dict, batch_size=1,"
    },
    {
      "line": 82,
      "text": "def init_coeff_tensors(self):"
    },
    {
      "line": 92,
      "text": "def get_lms(self, vs):"
    },
    {
      "line": 96,
      "text": "def split_coeffs(self, coeffs):"
    },
    {
      "line": 107,
      "text": "def merge_coeffs(self, id_coeff, exp_coeff, tex_coeff, angles, gamma, translation, eye_coeff):"
    },
    {
      "line": 111,
      "text": "def get_packed_tensors(self):"
    },
    {
      "line": 118,
      "text": "def execute(self, coeffs, render=False, surface=False, use_color=True, render_uv=False):"
    },
    {
      "line": 162,
      "text": "def get_vs(self, id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean):"
    },
    {
      "line": 170,
      "text": "def get_vs_lms(self, id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean):"
    },
    {
      "line": 178,
      "text": "def get_l_eye_center(self, id_coeff):"
    },
    {
      "line": 185,
      "text": "def get_r_eye_center(self, id_coeff):"
    },
    {
      "line": 192,
      "text": "def get_color(self, tex_coeff):"
    },
    {
      "line": 197,
      "text": "def _get_camera_pose(self):"
    },
    {
      "line": 201,
      "text": "def _get_p_mat(self):"
    },
    {
      "line": 208,
      "text": "def _get_camera_R(self):"
    },
    {
      "line": 212,
      "text": "def compute_norm(self, vs, tri, point_buf):"
    },
    {
      "line": 227,
      "text": "def project_vs(self, vs):"
    },
    {
      "line": 233,
      "text": "def compute_eye_rotation_matrix(self, eye):"
    },
    {
      "line": 259,
      "text": "def compute_rotation_matrix(self, angles):"
    },
    {
      "line": 287,
      "text": "def add_illumination(self, face_texture, norm, gamma):"
    },
    {
      "line": 321,
      "text": "def rigid_transform(self, vs, rot, trans):"
    }
  ]
}

```text

import numpy as np
import jittor as jt 
from jittor import nn 
jt.flags.use_cuda = 1

from faceverse_cuda.renderer import Renderer
from util_functions import get_point_buf


class FaceVerseModel(nn.Module):
    def __init__(self, model_dict, batch_size=1,
                 focal=1315, img_size=512):
        super(FaceVerseModel, self).__init__()

        self.focal = focal
        self.batch_size = batch_size
        self.img_size = img_size

        self.p_mat = self._get_p_mat()
        self.R = self._get_camera_R()
        self.camera_pos = self._get_camera_pose()
        self.rotXYZ = jt.init.eye([3, 3], dtype=jt.float32).view(1, 3, 3).repeat(3, 1, 1).view(3, 1, 3, 3)

        self.kp_inds = jt.array(model_dict['mediapipe_keypoints'].flatten(), dtype=jt.int64).stop_grad()
        self.ver_inds = model_dict['ver_inds']
        self.tri_inds = model_dict['tri_inds']

        meanshape = model_dict['meanshape'].reshape(-1, 3)
        meanshape[:, [1, 2]] *= -1
        meanshape = meanshape * 0.01
        
        self.meanshape = jt.array(meanshape.reshape(1, -1), dtype=jt.float32).stop_grad()
        self.meantex = jt.array(model_dict['meantex'].reshape(1, -1), dtype=jt.float32).stop_grad().repeat(self.batch_size, 1)
        self.parsingtex = jt.array(model_dict['parsing'].reshape(1, -1), dtype=jt.float32).stop_grad().repeat(self.batch_size, 1).reshape(self.batch_size, -1, 3)
        self.uvtexf = jt.array(model_dict['uvtex'].reshape(1, -1), dtype=jt.float32).stop_grad().repeat(self.batch_size, 1).reshape(self.batch_size, -1, 3)
        uvtex = model_dict['uvtex']
        uvtex[:self.ver_inds[0]] *= model_dict['face_mask'][:self.ver_inds[0], None]
        self.uvtex = jt.array(uvtex.reshape(1, -1), dtype=jt.float32).stop_grad().repeat(self.batch_size, 1).reshape(self.batch_size, -1, 3)

        idBase = model_dict['idBase'].reshape(-1, 3, 150)
        exBase = model_dict['exBase_52'].reshape(-1, 3, 52)
        idBase[:, [1, 2]] *= -1
        idBase = idBase * 0.01
        exBase[:, [1, 2]] *= -1
        exBase = exBase * 0.01
        idBase = idBase.reshape(-1, 150)
        exBase = exBase.reshape(-1, 52)
        texBase = model_dict['texBase'].reshape(-1, 251)

        self.idBase = jt.array(idBase, dtype=jt.float32).stop_grad().unsqueeze(0).repeat(self.batch_size, 1, 1)
        self.expBase = jt.array(exBase, dtype=jt.float32).stop_grad().unsqueeze(0).repeat(self.batch_size, 1, 1)
        self.texBase = jt.array(texBase, dtype=jt.float32).stop_grad().unsqueeze(0).repeat(self.batch_size, 1, 1)

        self.tri = jt.array(model_dict['tri'], dtype=jt.float32).stop_grad()
        self.tri = self.tri[:, [0, 2, 1]]
        self.point_buf = jt.array(model_dict['point_buf'], dtype=jt.float32).stop_grad()

        self.num_vertex = model_dict['meanshape'].shape[0]
        self.id_dims = self.idBase.shape[2]
        self.tex_dims = self.texBase.shape[2]
        self.exp_dims = self.expBase.shape[2]
        self.all_dims = self.id_dims + self.tex_dims + self.exp_dims

        self.init_coeff_tensors()

        self.renderer = Renderer(K=self.p_mat, R=self.R, t=self.camera_pos, image_size=self.img_size, 
                                 ver_num=int(self.ver_inds[2]), batch_size=self.batch_size)
        
        # for tracking by landmarks
        self.kp_inds_view = jt.concat([self.kp_inds[:, None] * 3, self.kp_inds[:, None] * 3 + 1, self.kp_inds[:, None] * 3 + 2], dim=1).flatten()
        self.idBase_view = self.idBase[:, self.kp_inds_view, :].detach().clone().stop_grad()
        self.expBase_view = self.expBase[:, self.kp_inds_view, :].detach().clone().stop_grad()
        self.meanshape_view = self.meanshape[:, self.kp_inds_view].detach().clone().stop_grad()

        # for rendering
        self.gamma_zeros = jt.zeros((self.batch_size, 27), dtype=jt.float32)
        self.gamma_zeros[:, [0, 9, 18]] += 0.1
        self.gamma_zeros[:, [1, 10, 19]] -= 0.2
        self.gamma_zeros[:, [2, 11, 20]] += 0.2
        self.gamma_zeros[:, [4, 13, 22]] -= 0.1

    def init_coeff_tensors(self):
        self.id_tensor = jt.zeros((1, self.id_dims), dtype=jt.float32)
        self.tex_tensor = jt.zeros((1, self.tex_dims), dtype=jt.float32)
        self.exp_tensor = jt.zeros((self.batch_size, self.exp_dims), dtype=jt.float32)
        self.gamma_tensor = jt.zeros((self.batch_size, 27), dtype=jt.float32)
        self.trans_tensor = jt.zeros((self.batch_size, 3), dtype=jt.float32)
        #self.trans_tensor[:, 2] -= 10 * self.focal / 1315
        self.eye_tensor = jt.zeros((self.batch_size, 4), dtype=jt.float32)
        self.rot_tensor = jt.zeros((self.batch_size, 3), dtype=jt.float32)

    def get_lms(self, vs):
        lms = vs[:, self.kp_inds, :]
        return lms

    def split_coeffs(self, coeffs):
        id_coeff = coeffs[:, :self.id_dims]  # identity(shape) coeff 
        exp_coeff = coeffs[:, self.id_dims:self.id_dims+self.exp_dims]  # expression coeff 
        tex_coeff = coeffs[:, self.id_dims+self.exp_dims:self.all_dims]  # texture(albedo) coeff 
        angles = coeffs[:, self.all_dims:self.all_dims+3] # ruler angles(x,y,z) for rotation of dim 3
        gamma = coeffs[:, self.all_dims+3:self.all_dims+30] # lighting coeff for 3 channel SH function of dim 27
        translation = coeffs[:, self.all_dims+30:self.all_dims+33]  # translation coeff of dim 3
        eye_coeff = coeffs[:, self.all_dims+33:]  # eye coeff of dim 3

        return id_coeff, exp_coeff, tex_coeff, angles, gamma, translation, eye_coeff

    def merge_coeffs(self, id_coeff, exp_coeff, tex_coeff, angles, gamma, translation, eye_coeff):
        coeffs = jt.concat([id_coeff, exp_coeff, tex_coeff, angles, gamma, translation, eye_coeff], dim=1)
        return coeffs

    def get_packed_tensors(self):
        return self.merge_coeffs(self.id_tensor.repeat(self.batch_size, 1),
                                 self.exp_tensor,
                                 self.tex_tensor.repeat(self.batch_size, 1),
                                 self.rot_tensor, self.gamma_tensor,
                                 self.trans_tensor, self.eye_tensor)

    def execute(self, coeffs, render=False, surface=False, use_color=True, render_uv=False):
        id_coeff, exp_coeff, tex_coeff, angles, gamma, translation, eye_coeff = self.split_coeffs(coeffs)
        rotation = self.compute_rotation_matrix(angles)
        l_eye_mat = self.compute_eye_rotation_matrix(eye_coeff[:, :2])
        r_eye_mat = self.compute_eye_rotation_matrix(eye_coeff[:, 2:])
        l_eye_mean = self.get_l_eye_center(id_coeff)
        r_eye_mean = self.get_r_eye_center(id_coeff)

        if render:
            vs = self.get_vs(id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean)
            vs_t = self.rigid_transform(vs, rotation, translation)

            lms_t = self.get_lms(vs_t)
            lms_proj = self.project_vs(lms_t)[:, :, :2]
            colors = self.get_color(tex_coeff)
            norm = self.compute_norm(vs, self.tri, self.point_buf)
            norm_r = nn.bmm(norm, rotation)
            if use_color:
                colors_illumin = self.add_illumination(colors, norm_r, self.gamma_zeros)
            else:
                colors_illumin = self.add_illumination(colors * 0 + 130, norm_r, self.gamma_zeros)
            
            rendered_img = self.renderer.render(vs_t[:, :self.ver_inds[2]], colors_illumin[:, :self.ver_inds[2]], self.tri[:self.tri_inds[2]])
            
            if render_uv:
                rendered_img = rendered_img.detach().clone()
                uv_img = self.renderer.render(vs_t[:, :self.ver_inds[2]], self.uvtex[:, :self.ver_inds[2]], self.tri[:self.tri_inds[2]])
                return {'rendered_img': rendered_img,
                        'uv_img': uv_img,
                        'lms_proj': lms_proj,
                        'colors': colors,
                        'vertices': vs_t}
            
            return {'rendered_img': rendered_img,
                    'lms_proj': lms_proj,
                    'colors': colors,
                    'vertices': vs_t}
        else:
            lms = self.get_vs_lms(id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean)
            lms_t = self.rigid_transform(lms, rotation, translation)

            lms_proj = self.project_vs(lms_t)[:, :, :2]
            return {'lms_proj': lms_proj}
    
    def get_vs(self, id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean):
        face_shape = jt.matmul(self.idBase, id_coeff.unsqueeze(2)).squeeze(2) + \
            jt.matmul(self.expBase, exp_coeff.unsqueeze(2)).squeeze(2) + self.meanshape
        face_shape = face_shape.view(self.batch_size, -1, 3)
        face_shape[:, self.ver_inds[0]:self.ver_inds[1]] = jt.matmul(face_shape[:, self.ver_inds[0]:self.ver_inds[1]] - l_eye_mean, l_eye_mat) + l_eye_mean + 0.01
        face_shape[:, self.ver_inds[1]:self.ver_inds[2]] = jt.matmul(face_shape[:, self.ver_inds[1]:self.ver_inds[2]] - r_eye_mean, r_eye_mat) + r_eye_mean + 0.01
        return face_shape

    def get_vs_lms(self, id_coeff, exp_coeff, l_eye_mat, r_eye_mat, l_eye_mean, r_eye_mean):
        face_shape = jt.matmul(self.idBase_view, id_coeff.unsqueeze(2)).squeeze(2) + \
            jt.matmul(self.expBase_view, exp_coeff.unsqueeze(2)).squeeze(2) + self.meanshape_view
        face_shape = face_shape.view(self.batch_size, -1, 3)
        face_shape[:, 473:478] = jt.matmul(face_shape[:, 473:478] - l_eye_mean, l_eye_mat) + l_eye_mean
        face_shape[:, 468:473] = jt.matmul(face_shape[:, 468:473] - r_eye_mean, r_eye_mat) + r_eye_mean
        return face_shape
    
    def get_l_eye_center(self, id_coeff):
        eye_shape = jt.matmul(self.idBase.reshape(self.batch_size, -1, 3, self.id_dims)[:, self.ver_inds[0]:self.ver_inds[1]].reshape(self.batch_size, -1, self.id_dims), 
                    id_coeff.unsqueeze(2)).squeeze(2) + self.meanshape.reshape(1, -1, 3)[:, self.ver_inds[0]:self.ver_inds[1]].reshape(1, -1)
        eye_shape = eye_shape.reshape(self.batch_size, -1, 3)
        eye_shape[:, :, 2] += 0.005
        return jt.mean(eye_shape, dim=1, keepdims=True).stop_grad()
    
    def get_r_eye_center(self, id_coeff):
        eye_shape = jt.matmul(self.idBase.reshape(self.batch_size, -1, 3, self.id_dims)[:, self.ver_inds[1]:self.ver_inds[2]].reshape(self.batch_size, -1, self.id_dims), 
                    id_coeff.unsqueeze(2)).squeeze(2) + self.meanshape.reshape(1, -1, 3)[:, self.ver_inds[1]:self.ver_inds[2]].reshape(1, -1)
        eye_shape = eye_shape.reshape(self.batch_size, -1, 3)
        eye_shape[:, :, 2] += 0.005
        return jt.mean(eye_shape, dim=1, keepdims=True).stop_grad()

    def get_color(self, tex_coeff):
        face_texture = jt.matmul(self.texBase, tex_coeff.unsqueeze(2)).squeeze(2) + self.meantex
        face_texture = face_texture.view(self.batch_size, -1, 3)
        return face_texture

    def _get_camera_pose(self):
        camera_pos = jt.float32([0.0, 0.0, 10.0]).reshape(1, 1, 3)
        return camera_pos.stop_grad()

    def _get_p_mat(self):
        half_image_width = self.img_size // 2
        p_matrix = np.array([self.focal, 0.0, half_image_width,
                             0.0, self.focal, half_image_width,
                             0.0, 0.0, 1.0], dtype=np.float32).reshape(1, 3, 3)
        return jt.array(p_matrix, dtype=jt.float32).stop_grad()

    def _get_camera_R(self):
        R = np.reshape(np.array([1.0, 0, 0, 0, 1.0, 0, 0, 0, 1.0], dtype=np.float32), [1, 3, 3])
        return jt.array(R, dtype=jt.float32).stop_grad()

    def compute_norm(self, vs, tri, point_buf):
        face_id = tri
        point_id = point_buf
        v1 = vs[:, face_id[:, 0], :]
        v2 = vs[:, face_id[:, 1], :]
        v3 = vs[:, face_id[:, 2], :]
        e1 = v1 - v2
        e2 = v2 - v3
        face_norm = e1.cross(e2)

        v_norm = face_norm[:, point_id, :].sum(2)
        v_norm = v_norm / (v_norm.norm(dim=2).unsqueeze(2) + 1e-9)

        return v_norm

    def project_vs(self, vs):
        vs += self.camera_pos
        aug_projection = jt.matmul(vs, self.p_mat.repeat((self.batch_size, 1, 1)).permute((0, 2, 1)))
        face_projection = aug_projection[:, :, :2] / aug_projection[:, :, 2:3]
        return jt.stack([face_projection[:, :, 0], face_projection[:, :, 1], aug_projection[:, :, 2]], dim=2)

    def compute_eye_rotation_matrix(self, eye):
        # 0 left_eye + down - up
        # 1 left_eye + right - left
        # 2 right_eye + down - up
        # 3 right_eye + right - left
        sinx = jt.sin(eye[:, 0])
        siny = jt.sin(eye[:, 1])
        cosx = jt.cos(eye[:, 0])
        cosy = jt.cos(eye[:, 1])
        if self.batch_size != 1:
            rotXYZ = self.rotXYZ.repeat(1, self.batch_size, 1, 1).detach().clone()
        else:
            rotXYZ = self.rotXYZ.detach().clone()
        rotXYZ[0, :, 1, 1] = cosx
        rotXYZ[0, :, 1, 2] = -sinx
        rotXYZ[0, :, 2, 1] = sinx
        rotXYZ[0, :, 2, 2] = cosx
        rotXYZ[1, :, 0, 0] = cosy
        rotXYZ[1, :, 0, 2] = siny
        rotXYZ[1, :, 2, 0] = -siny
        rotXYZ[1, :, 2, 2] = cosy

        rotation = nn.bmm(rotXYZ[1], rotXYZ[0])

        return rotation.permute(0, 2, 1)

    def compute_rotation_matrix(self, angles):
        sinx = jt.sin(angles[:, 0])
        siny = jt.sin(angles[:, 1])
        sinz = jt.sin(angles[:, 2])
        cosx = jt.cos(angles[:, 0])
        cosy = jt.cos(angles[:, 1])
        cosz = jt.cos(angles[:, 2])
        if self.batch_size != 1:
            rotXYZ = self.rotXYZ.repeat(1, self.batch_size, 1, 1).detach().clone()
        else:
            rotXYZ = self.rotXYZ.detach().clone()
        rotXYZ[0, :, 1, 1] = cosx
        rotXYZ[0, :, 1, 2] = -sinx
        rotXYZ[0, :, 2, 1] = sinx
        rotXYZ[0, :, 2, 2] = cosx
        rotXYZ[1, :, 0, 0] = cosy
        rotXYZ[1, :, 0, 2] = siny
        rotXYZ[1, :, 2, 0] = -siny
        rotXYZ[1, :, 2, 2] = cosy
        rotXYZ[2, :, 0, 0] = cosz
        rotXYZ[2, :, 0, 1] = -sinz
        rotXYZ[2, :, 1, 0] = sinz
        rotXYZ[2, :, 1, 1] = cosz

        rotation = nn.bmm(rotXYZ[2], nn.bmm(rotXYZ[1], rotXYZ[0]))

        return rotation.permute(0, 2, 1)

    def add_illumination(self, face_texture, norm, gamma):
        gamma = gamma.view(-1, 3, 9).clone()
        gamma[:, :, 0] += 0.8
        gamma = gamma.permute(0, 2, 1)

        a0 = np.pi
        a1 = 2 * np.pi / np.sqrt(3.0)
        a2 = 2 * np.pi / np.sqrt(8.0)
        c0 = 1 / np.sqrt(4 * np.pi)
        c1 = np.sqrt(3.0) / np.sqrt(4 * np.pi)
        c2 = 3 * np.sqrt(5.0) / np.sqrt(12 * np.pi)
        d0 = 0.5 / np.sqrt(3.0)

        norm = norm.view(-1, 3)
        nx, ny, nz = norm[:, 0], norm[:, 1], norm[:, 2]
        arrH = []

        arrH.append(a0 * c0 * (nx * 0 + 1))
        arrH.append(-a1 * c1 * ny)
        arrH.append(a1 * c1 * nz)
        arrH.append(-a1 * c1 * nx)
        arrH.append(a2 * c2 * nx * ny)
        arrH.append(-a2 * c2 * ny * nz)
        arrH.append(a2 * c2 * d0 * (3 * nz.pow(2) - 1))
        arrH.append(-a2 * c2 * nx * nz)
        arrH.append(a2 * c2 * 0.5 * (nx.pow(2) - ny.pow(2)))

        H = jt.stack(arrH, 1)
        Y = H.view(self.batch_size, face_texture.shape[1], 9)
        lighting = nn.bmm(Y, gamma)

        face_color = face_texture * lighting
        return face_color

    def rigid_transform(self, vs, rot, trans):
        vs_r = jt.matmul(vs, rot)
        vs_t = vs_r + trans.view(-1, 1, 3)
        return vs_t



```

## Source file: faceversev3_jittor/rvm.py

{
  "path": "faceversev3_jittor/rvm.py",
  "blob_sha": "c38b19a9087146e25d4a927f0c6baf5ac531b330",
  "sha256": "6a8ef5345e317c1c2b42b15c20f0d30631e78f06c74eb0b0a16616aaf65a0dc1",
  "lines": 26,
  "bytes": 962,
  "selection_score": 3,
  "keyword_line_counts": {
    "gum": 3
  },
  "url": "https://github.com/LizhenWangT/FaceVerse/blob/a012b634e4e77892895660bc7164ea27f5427623/faceversev3_jittor/rvm.py",
  "function_signature_candidates": []
}

```text

import onnxruntime as ort
import os
import cv2
import numpy as np
import argparse


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="RVM")
    parser.add_argument("--path", type=str, default='D:/test_case', help="path to the image dataset")
    parser.add_argument("--ckpt", type=str, default='data/rvm_1024_1024_32.onnx', help="path to the checkpoints to resume training")
    args = parser.parse_args()

    sess = ort.InferenceSession(args.ckpt)
    os.makedirs(os.path.join(args.path, 'back'), exist_ok=True)
    for name in os.listdir(os.path.join(args.path, 'image')):
        img = cv2.imread(os.path.join(args.path, 'image', name))[None, :, :, :].astype(np.float32)
        pha = sess.run(['out'], {
            'src': img
        })
        print(name)
        cv2.imshow('rvm', pha[0][0, 0].astype(np.uint8))
        cv2.waitKey(1)
        cv2.imwrite(os.path.join(args.path, 'back', name), pha[0][0, 0].astype(np.uint8))




```

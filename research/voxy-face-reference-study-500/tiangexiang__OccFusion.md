# Facial rendering source scan: tiangexiang/OccFusion

Repository: https://github.com/tiangexiang/OccFusion

Pinned source revision: f41e091ed1605034dcc13f7bd2cff8e9253305d1

Discovery topic: human rendering

Description: [NeurIPS 2024] OccFusion: Rendering Occluded Humans with Generative Diffusion Priors

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["submodules/diff-gaussian-rasterization/LICENSE.md", "submodules/diff-gaussian-rasterization/third_party/glm/copying.txt"]

Topic paths: {"gum": ["arguments/__init__.py"], "lbs": ["nets/mlp_delta_weight_lbs.py", "smplx/lbs.py"], "rig": ["submodules/diff-gaussian-rasterization/third_party/glm/glm/detail/func_trigonometric.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/detail/func_trigonometric_simd.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/quaternion_trigonometric.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/quaternion_trigonometric.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/gtx/fast_trigonometry.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/gtx/fast_trigonometry.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/simd/trigonometric.h", "submodules/diff-gaussian-rasterization/third_party/glm/glm/trigonometric.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/core/core_func_trigonometric.cpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/ext/ext_quaternion_trigonometric.cpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/gtx/gtx_fast_trigonometry.cpp"], "lip": ["submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/matrix_clip_space.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/matrix_clip_space.inl", "submodules/diff-gaussian-rasterization/third_party/glm/test/ext/ext_matrix_clip_space.cpp"]}

## Source file: smplx/lbs.py

{
  "path": "smplx/lbs.py",
  "blob_sha": "9d8f0619e2f321c999f08d0f96f69e8947e80fa7",
  "sha256": "cf4840e0ee1a8904eecf60270e00531ea665b1750594590fc09f33f6723f209b",
  "lines": 405,
  "bytes": 14122,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 20
  },
  "url": "https://github.com/tiangexiang/OccFusion/blob/f41e091ed1605034dcc13f7bd2cff8e9253305d1/smplx/lbs.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 108,
      "text": "def vertices2landmarks("
    },
    {
      "line": 156,
      "text": "def lbs("
    },
    {
      "line": 255,
      "text": "def vertices2joints(J_regressor: Tensor, vertices: Tensor) -> Tensor:"
    },
    {
      "line": 275,
      "text": "def blend_shapes(betas: Tensor, shape_disps: Tensor) -> Tensor:"
    },
    {
      "line": 299,
      "text": "def batch_rodrigues("
    },
    {
      "line": 336,
      "text": "def transform_mat(R: Tensor, t: Tensor) -> Tensor:"
    },
    {
      "line": 349,
      "text": "def batch_rigid_transform("
    }
  ]
}

```text

# -*- coding: utf-8 -*-

# Max-Planck-Gesellschaft zur Förderung der Wissenschaften e.V. (MPG) is
# holder of all proprietary rights on this computer program.
# You can only use this computer program if you have closed
# a license agreement with MPG or you get the right to use the computer
# program from someone who is authorized to grant you that right.
# Any use of the computer program without a valid license is prohibited and
# liable to prosecution.
#
# Copyright©2019 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: ps-license@tuebingen.mpg.de

from __future__ import absolute_import
from __future__ import print_function
from __future__ import division

from typing import Tuple, List
import numpy as np

import torch
import torch.nn.functional as F

from .utils import rot_mat_to_euler, Tensor


def find_dynamic_lmk_idx_and_bcoords(
    vertices: Tensor,
    pose: Tensor,
    dynamic_lmk_faces_idx: Tensor,
    dynamic_lmk_b_coords: Tensor,
    neck_kin_chain: List[int],
    pose2rot: bool = True,
) -> Tuple[Tensor, Tensor]:
    ''' Compute the faces, barycentric coordinates for the dynamic landmarks


        To do so, we first compute the rotation of the neck around the y-axis
        and then use a pre-computed look-up table to find the faces and the
        barycentric coordinates that will be used.

        Special thanks to Soubhik Sanyal (soubhik.sanyal@tuebingen.mpg.de)
        for providing the original TensorFlow implementation and for the LUT.

        Parameters
        ----------
        vertices: torch.tensor BxVx3, dtype = torch.float32
            The tensor of input vertices
        pose: torch.tensor Bx(Jx3), dtype = torch.float32
            The current pose of the body model
        dynamic_lmk_faces_idx: torch.tensor L, dtype = torch.long
            The look-up table from neck rotation to faces
        dynamic_lmk_b_coords: torch.tensor Lx3, dtype = torch.float32
            The look-up table from neck rotation to barycentric coordinates
        neck_kin_chain: list
            A python list that contains the indices of the joints that form the
            kinematic chain of the neck.
        dtype: torch.dtype, optional

        Returns
        -------
        dyn_lmk_faces_idx: torch.tensor, dtype = torch.long
            A tensor of size BxL that contains the indices of the faces that
            will be used to compute the current dynamic landmarks.
        dyn_lmk_b_coords: torch.tensor, dtype = torch.float32
            A tensor of size BxL that contains the indices of the faces that
            will be used to compute the current dynamic landmarks.
    '''

    dtype = vertices.dtype
    batch_size = vertices.shape[0]

    if pose2rot:
        aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
                                     neck_kin_chain)
        rot_mats = batch_rodrigues(
            aa_pose.view(-1, 3)).view(batch_size, -1, 3, 3)
    else:
        rot_mats = torch.index_select(
            pose.view(batch_size, -1, 3, 3), 1, neck_kin_chain)

    rel_rot_mat = torch.eye(
        3, device=vertices.device, dtype=dtype).unsqueeze_(dim=0).repeat(
            batch_size, 1, 1)
    for idx in range(len(neck_kin_chain)):
        rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

    y_rot_angle = torch.round(
        torch.clamp(-rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi,
                    max=39)).to(dtype=torch.long)
    neg_mask = y_rot_angle.lt(0).to(dtype=torch.long)
    mask = y_rot_angle.lt(-39).to(dtype=torch.long)
    neg_vals = mask * 78 + (1 - mask) * (39 - y_rot_angle)
    y_rot_angle = (neg_mask * neg_vals +
                   (1 - neg_mask) * y_rot_angle)

    dyn_lmk_faces_idx = torch.index_select(dynamic_lmk_faces_idx,
                                           0, y_rot_angle)
    dyn_lmk_b_coords = torch.index_select(dynamic_lmk_b_coords,
                                          0, y_rot_angle)

    return dyn_lmk_faces_idx, dyn_lmk_b_coords


def vertices2landmarks(
    vertices: Tensor,
    faces: Tensor,
    lmk_faces_idx: Tensor,
    lmk_bary_coords: Tensor
) -> Tensor:
    ''' Calculates landmarks by barycentric interpolation

        Parameters
        ----------
        vertices: torch.tensor BxVx3, dtype = torch.float32
            The tensor of input vertices
        faces: torch.tensor Fx3, dtype = torch.long
            The faces of the mesh
        lmk_faces_idx: torch.tensor L, dtype = torch.long
            The tensor with the indices of the faces used to calculate the
            landmarks.
        lmk_bary_coords: torch.tensor Lx3, dtype = torch.float32
            The tensor of barycentric coordinates that are used to interpolate
            the landmarks

        Returns
        -------
        landmarks: torch.tensor BxLx3, dtype = torch.float32
            The coordinates of the landmarks for each mesh in the batch
    '''
    # Extract the indices of the vertices for each face
    # BxLx3
    batch_size, num_verts = vertices.shape[:2]
    device = vertices.device

    lmk_faces = torch.index_select(faces, 0, lmk_faces_idx.view(-1).to(torch.long)).view(
        batch_size, -1, 3)
                        #The '.to(torch.long)'.
                        # added to make the trace work in c++,
                        # otherwise you get a runtime error in c++:
                        # 'index_select(): Expected dtype int32 or int64 for index'

    lmk_faces += torch.arange(
        batch_size, dtype=torch.long, device=device).view(-1, 1, 1) * num_verts

    lmk_vertices = vertices.view(-1, 3)[lmk_faces].view(
        batch_size, -1, 3, 3)

    landmarks = torch.einsum('blfi,blf->bli', [lmk_vertices, lmk_bary_coords])
    return landmarks


def lbs(
    betas: Tensor,
    pose: Tensor,
    v_template: Tensor,
    shapedirs: Tensor,
    posedirs: Tensor,
    J_regressor: Tensor,
    parents: Tensor,
    lbs_weights: Tensor,
    pose2rot: bool = True,
) -> Tuple[Tensor, Tensor]:
    ''' Performs Linear Blend Skinning with the given shape and pose parameters

        Parameters
        ----------
        betas : torch.tensor BxNB
            The tensor of shape parameters
        pose : torch.tensor Bx(J + 1) * 3
            The pose parameters in axis-angle format
        v_template torch.tensor BxVx3
            The template mesh that will be deformed
        shapedirs : torch.tensor 1xNB
            The tensor of PCA shape displacements
        posedirs : torch.tensor Px(V * 3)
            The pose PCA coefficients
        J_regressor : torch.tensor JxV
            The regressor array that is used to calculate the joints from
            the position of the vertices
        parents: torch.tensor J
            The array that describes the kinematic tree for the model
        lbs_weights: torch.tensor N x V x (J + 1)
            The linear blend skinning weights that represent how much the
            rotation matrix of each part affects each vertex
        pose2rot: bool, optional
            Flag on whether to convert the input pose tensor to rotation
            matrices. The default value is True. If False, then the pose tensor
            should already contain rotation matrices and have a size of
            Bx(J + 1)x9
        dtype: torch.dtype, optional

        Returns
        -------
        verts: torch.tensor BxVx3
            The vertices of the mesh after applying the shape and pose
            displacements.
        joints: torch.tensor BxJx3
            The joints of the model
    '''

    batch_size = max(betas.shape[0], pose.shape[0])
    device, dtype = betas.device, betas.dtype

    # Add shape contribution
    v_shaped = v_template + blend_shapes(betas, shapedirs)

    # Get the joints
    # NxJx3 array
    J = vertices2joints(J_regressor, v_shaped)

    # 3. Add pose blend shapes
    # N x J x 3 x 3
    ident = torch.eye(3, dtype=dtype, device=device)
    if pose2rot:
        rot_mats = batch_rodrigues(pose.view(-1, 3)).view(
            [batch_size, -1, 3, 3])

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(
            pose_feature, posedirs).view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1),
                                    posedirs).view(batch_size, -1, 3)

    v_posed = pose_offsets + v_shaped
    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)

    # 5. Do skinning:
    # W is N x V x (J + 1)
    W = lbs_weights.unsqueeze(dim=0).expand([batch_size, -1, -1])
    # (N x V x (J + 1)) x (N x (J + 1) x 16)
    num_joints = J_regressor.shape[0]
    T = torch.matmul(W, A.view(batch_size, num_joints, 16)) \
        .view(batch_size, -1, 4, 4)

    homogen_coord = torch.ones([batch_size, v_posed.shape[1], 1],
                               dtype=dtype, device=device)
    v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
    v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

    verts = v_homo[:, :, :3, 0]

    return verts, J_transformed, A, T 


def vertices2joints(J_regressor: Tensor, vertices: Tensor) -> Tensor:
    ''' Calculates the 3D joint locations from the vertices

    Parameters
    ----------
    J_regressor : torch.tensor JxV
        The regressor array that is used to calculate the joints from the
        position of the vertices
    vertices : torch.tensor BxVx3
        The tensor of mesh vertices

    Returns
    -------
    torch.tensor BxJx3
        The location of the joints
    '''

    return torch.einsum('bik,ji->bjk', [vertices, J_regressor])


def blend_shapes(betas: Tensor, shape_disps: Tensor) -> Tensor:
    ''' Calculates the per vertex displacement due to the blend shapes


    Parameters
    ----------
    betas : torch.tensor Bx(num_betas)
        Blend shape coefficients
    shape_disps: torch.tensor Vx3x(num_betas)
        Blend shapes

    Returns
    -------
    torch.tensor BxVx3
        The per-vertex displacement due to shape deformation
    '''

    # Displacement[b, m, k] = sum_{l} betas[b, l] * shape_disps[m, k, l]
    # i.e. Multiply each shape displacement by its corresponding beta and
    # then sum them.
    blend_shape = torch.einsum('bl,mkl->bmk', [betas, shape_disps])
    return blend_shape


def batch_rodrigues(
    rot_vecs: Tensor,
    epsilon: float = 1e-8,
) -> Tensor:
    ''' Calculates the rotation matrices for a batch of rotation vectors
        Parameters
        ----------
        rot_vecs: torch.tensor Nx3
            array of N axis-angle vectors
        Returns
        -------
        R: torch.tensor Nx3x3
            The rotation matrices for the given axis-angle parameters
    '''

    batch_size = rot_vecs.shape[0]
    device, dtype = rot_vecs.device, rot_vecs.dtype

    angle = torch.norm(rot_vecs + 1e-8, dim=1, keepdim=True)
    rot_dir = rot_vecs / angle

    cos = torch.unsqueeze(torch.cos(angle), dim=1)
    sin = torch.unsqueeze(torch.sin(angle), dim=1)

    # Bx1 arrays
    rx, ry, rz = torch.split(rot_dir, 1, dim=1)
    K = torch.zeros((batch_size, 3, 3), dtype=dtype, device=device)

    zeros = torch.zeros((batch_size, 1), dtype=dtype, device=device)
    K = torch.cat([zeros, -rz, ry, rz, zeros, -rx, -ry, rx, zeros], dim=1) \
        .view((batch_size, 3, 3))

    ident = torch.eye(3, dtype=dtype, device=device).unsqueeze(dim=0)
    rot_mat = ident + sin * K + (1 - cos) * torch.bmm(K, K)
    return rot_mat


def transform_mat(R: Tensor, t: Tensor) -> Tensor:
    ''' Creates a batch of transformation matrices
        Args:
            - R: Bx3x3 array of a batch of rotation matrices
            - t: Bx3x1 array of a batch of translation vectors
        Returns:
            - T: Bx4x4 Transformation matrix
    '''
    # No padding left or right, only add an extra row
    return torch.cat([F.pad(R, [0, 0, 0, 1]),
                      F.pad(t, [0, 0, 0, 1], value=1)], dim=2)


def batch_rigid_transform(
    rot_mats: Tensor,
    joints: Tensor,
    parents: Tensor,
    dtype=torch.float32
) -> Tensor:
    """
    Applies a batch of rigid transformations to the joints

    Parameters
    ----------
    rot_mats : torch.tensor BxNx3x3
        Tensor of rotation matrices
    joints : torch.tensor BxNx3
        Locations of joints
    parents : torch.tensor BxN
        The kinematic tree of each object
    dtype : torch.dtype, optional:
        The data type of the created tensors, the default is torch.float32

    Returns
    -------
    posed_joints : torch.tensor BxNx3
        The locations of the joints after applying the pose rotations
    rel_transforms : torch.tensor BxNx4x4
        The relative (with respect to the root joint) rigid transformations
        for all the joints
    """

    joints = torch.unsqueeze(joints, dim=-1)

    rel_joints = joints.clone()
    rel_joints[:, 1:] -= joints[:, parents[1:]]

    transforms_mat = transform_mat(
        rot_mats.reshape(-1, 3, 3),
        rel_joints.reshape(-1, 3, 1)).reshape(-1, joints.shape[1], 4, 4)

    transform_chain = [transforms_mat[:, 0]]
    for i in range(1, parents.shape[0]):
        # Subtract the joint location at the rest pose
        # No need for rotation, since it's identity when at rest
        curr_res = torch.matmul(transform_chain[parents[i]],
                                transforms_mat[:, i])
        transform_chain.append(curr_res)

    transforms = torch.stack(transform_chain, dim=1)

    # The last column of the transformations contains the posed joints
    posed_joints = transforms[:, :, :3, 3]

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(
        torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])

    return posed_joints, rel_transforms


```

## Source file: nets/mlp_delta_weight_lbs.py

{
  "path": "nets/mlp_delta_weight_lbs.py",
  "blob_sha": "017fb124ae906cb76674ae6fd2b68bd59b9772f3",
  "sha256": "99938d334433b0b16e2d204973ccb47e50c28fe70d38974c17b88439128224ed",
  "lines": 80,
  "bytes": 2450,
  "selection_score": 15,
  "keyword_line_counts": {
    "lbs": 2
  },
  "url": "https://github.com/tiangexiang/OccFusion/blob/f41e091ed1605034dcc13f7bd2cff8e9253305d1/nets/mlp_delta_weight_lbs.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def __init__(self, total_bones=24):"
    },
    {
      "line": 23,
      "text": "def forward(self, pts):"
    },
    {
      "line": 35,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 39,
      "text": "def create_embedding_fn(self):"
    },
    {
      "line": 64,
      "text": "def embed(self, inputs):"
    },
    {
      "line": 67,
      "text": "def get_embedder(multires, input_dims=3):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F

class LBSOffsetDecoder(nn.Module):
    def __init__(self, total_bones=24):
        super(LBSOffsetDecoder, self).__init__()

        self.total_bones = total_bones

        self.actvn = nn.ReLU()

        input_ch = 63
        D = 4
        W = 128
        self.skips = [2]
        self.bw_linears = nn.ModuleList([nn.Conv1d(input_ch, W, 1)] + [
            nn.Conv1d(W, W, 1) if i not in
            self.skips else nn.Conv1d(W + input_ch, W, 1) for i in range(D - 1)
        ])
        self.bw_fc = nn.Conv1d(W, total_bones, 1)

    def forward(self, pts):
        features = xyz_embedder(pts)
        features = features.permute(0, 2, 1)
        net = features
        for i, l in enumerate(self.bw_linears):
            net = self.actvn(self.bw_linears[i](net))
            if i in self.skips:
                net = torch.cat((features, net), dim=1)
        bw = self.bw_fc(net)   
        return bw 

class Embedder:
    def __init__(self, **kwargs):
        self.kwargs = kwargs
        self.create_embedding_fn()

    def create_embedding_fn(self):
        embed_fns = []
        d = self.kwargs['input_dims']
        out_dim = 0
        if self.kwargs['include_input']:
            embed_fns.append(lambda x: x)
            out_dim += d

        max_freq = self.kwargs['max_freq_log2']
        N_freqs = self.kwargs['num_freqs']

        if self.kwargs['log_sampling']:
            freq_bands = 2.**torch.linspace(0., max_freq, steps=N_freqs)
        else:
            freq_bands = torch.linspace(2.**0., 2.**max_freq, steps=N_freqs)

        for freq in freq_bands:
            for p_fn in self.kwargs['periodic_fns']:
                embed_fns.append(
                    lambda x, p_fn=p_fn, freq=freq: p_fn(x * freq))
                out_dim += d

        self.embed_fns = embed_fns
        self.out_dim = out_dim

    def embed(self, inputs):
        return torch.cat([fn(inputs) for fn in self.embed_fns], -1)

def get_embedder(multires, input_dims=3):
    embed_kwargs = {
        'include_input': True,
        'input_dims': input_dims,
        'max_freq_log2': multires - 1,
        'num_freqs': multires,
        'log_sampling': True,
        'periodic_fns': [torch.sin, torch.cos],
    }
    embedder_obj = Embedder(**embed_kwargs)
    embed = lambda x, eo=embedder_obj: eo.embed(x)
    return embed, embedder_obj.out_dim

xyz_embedder, xyz_dim = get_embedder(10)

```

## Source file: arguments/__init__.py

{
  "path": "arguments/__init__.py",
  "blob_sha": "37a756d2a685d02e440e481a52c86252d4ccb44c",
  "sha256": "840ed968d8024e05885d115ff88ae9bc3b87e106392998b915d3c365e98dd3ee",
  "lines": 121,
  "bytes": 4076,
  "selection_score": 3,
  "keyword_line_counts": {
    "gum": 8,
    "lbs": 1,
    "rig": 2
  },
  "url": "https://github.com/tiangexiang/OccFusion/blob/f41e091ed1605034dcc13f7bd2cff8e9253305d1/arguments/__init__.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self, parser: ArgumentParser, name : str, fill_none = False):"
    },
    {
      "line": 40,
      "text": "def extract(self, args):"
    },
    {
      "line": 48,
      "text": "def __init__(self, parser, sentinel=False):"
    },
    {
      "line": 65,
      "text": "def extract(self, args):"
    },
    {
      "line": 71,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 78,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 101,
      "text": "def get_combined_args(parser : ArgumentParser):"
    }
  ]
}

```text

#
# Copyright (C) 2023, Inria
# GRAPHDECO research group, https://team.inria.fr/graphdeco
# All rights reserved.
#
# This software is free for non-commercial, research and evaluation use 
# under the terms of the LICENSE.md file.
#
# For inquiries contact  george.drettakis@inria.fr
#

from argparse import ArgumentParser, Namespace
import sys
import os

class GroupParams:
    pass

class ParamGroup:
    def __init__(self, parser: ArgumentParser, name : str, fill_none = False):
        group = parser.add_argument_group(name)
        for key, value in vars(self).items():
            shorthand = False
            if key.startswith("_"):
                shorthand = True
                key = key[1:]
            t = type(value)
            value = value if not fill_none else None 
            if shorthand:
                if t == bool:
                    group.add_argument("--" + key, ("-" + key[0:1]), default=value, action="store_true")
                else:
                    group.add_argument("--" + key, ("-" + key[0:1]), default=value, type=t)
            else:
                if t == bool:
                    group.add_argument("--" + key, default=value, action="store_true")
                else:
                    group.add_argument("--" + key, default=value, type=t)

    def extract(self, args):
        group = GroupParams()
        for arg in vars(args).items():
            if arg[0] in vars(self) or ("_" + arg[0]) in vars(self):
                setattr(group, arg[0], arg[1])
        return group

class ModelParams(ParamGroup): 
    def __init__(self, parser, sentinel=False):
        self.sh_degree = 3
        self._source_path = ""
        self._gen_root = ""
        self._model_path = ""
        self._images = "images"
        self._resolution = -1
        self._white_background = False
        self.data_device = "cuda"
        self.eval = False
        # self.rotate_smpl = None
        self.exp_name = ""
        self.smpl_type = "smpl"
        self.actor_gender = "neutral"
        self.motion_offset_flag = False
        super().__init__(parser, "Loading Parameters", sentinel)

    def extract(self, args):
        g = super().extract(args)
        g.source_path = os.path.abspath(g.source_path)
        return g

class PipelineParams(ParamGroup):
    def __init__(self, parser):
        self.convert_SHs_python = False
        self.compute_cov3D_python = True
        self.debug = False
        super().__init__(parser, "Pipeline Parameters")

class OptimizationParams(ParamGroup):
    def __init__(self, parser):
        self.iterations = 30_000
        self.position_lr_init = 0.00016
        self.position_lr_final = 0.0000016
        self.position_lr_delay_mult = 0.01
        self.position_lr_max_steps = 30_000
        self.feature_lr = 0.0025
        self.opacity_lr = 0.05
        self.scaling_lr = 0.005
        self.rotation_lr = 0.001
        self.pose_refine_lr = 0.00005
        self.lbs_offset_lr = 0.00005
        self.percent_dense = 0.01
        self.lambda_dssim = 0.2
        self.densification_interval = 100
        self.opacity_reset_interval = 3000
        self.densify_from_iter = 400 #500
        self.densify_until_iter = 1000 #15_000
        self.densify_grad_threshold = 0.0002
        

        super().__init__(parser, "Optimization Parameters")

def get_combined_args(parser : ArgumentParser):
    cmdlne_string = sys.argv[1:]
    cfgfile_string = "Namespace()"
    args_cmdline = parser.parse_args(cmdlne_string)

    try:
        cfgfilepath = os.path.join(args_cmdline.model_path, "cfg_args")
        print("Looking for config file in", cfgfilepath)
        with open(cfgfilepath) as cfg_file:
            print("Config file found: {}".format(cfgfilepath))
            cfgfile_string = cfg_file.read()
    except TypeError:
        print("Config file not found at")
        pass
    args_cfgfile = eval(cfgfile_string)

    merged_dict = vars(args_cfgfile).copy()
    for k,v in vars(args_cmdline).items():
        if v != None:
            merged_dict[k] = v
    return Namespace(**merged_dict)


```

## Source file: render.py

{
  "path": "render.py",
  "blob_sha": "09d1353d6f5caab0b3acf2654581bb1dff44f00c",
  "sha256": "30c224c26c15069c6ce40eff6da4e9bf1a4ceb9a7eb1af3ffc4dde0f11483e57",
  "lines": 112,
  "bytes": 4172,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 10,
    "rig": 1
  },
  "url": "https://github.com/tiangexiang/OccFusion/blob/f41e091ed1605034dcc13f7bd2cff8e9253305d1/render.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def render_set(model_path, name, iteration, views, gaussians, pipeline, background, canonical=False, canonical_pose=False):"
    },
    {
      "line": 76,
      "text": "def render_sets(dataset : ModelParams, iteration : int, pipeline : PipelineParams, skip_train : bool, skip_test : bool, canonical : bool, canonical_pose : bool):"
    }
  ]
}

```text

import torch
from scene import Scene
from scene.cameras import orbit_camera
import os
import time
import pickle
from tqdm import tqdm
from os import makedirs
from gaussian_renderer import render
import torchvision
from utils.general_utils import safe_state
from argparse import ArgumentParser
from arguments import ModelParams, PipelineParams, get_combined_args
from gaussian_renderer import GaussianModel
import numpy as np
from utils.image_utils import psnr
from utils.loss_utils import ssim


def render_set(model_path, name, iteration, views, gaussians, pipeline, background, canonical=False, canonical_pose=False):
    render_path = os.path.join(model_path, name, "{}".format(iteration), "renders")
    gts_path = os.path.join(model_path, name, "{}".format(iteration), "gt")

    makedirs(render_path, exist_ok=True)
    makedirs(gts_path, exist_ok=True)

    rgbs = []
    rgbs_gt = []
    alphas = []
    elapsed_time = 0

    hors = [0, 20, 40, 60, 80, 100, 120, 140, 160, 180, -20, -40, -60, -80, -100, -120, -140, -160]
    for idx, view in enumerate(tqdm(views, desc="Rendering progress")):
        gt = view.original_image[0:3, :, :].cuda()


        frame_name = view.image_name.split('/')[-1]

        bound_mask = view.bound_mask

        # Start timer
        start_time = time.time() 

        render_output = render(view, gaussians, pipeline, background)

        rendering = render_output["render"]
        alpha = render_output["render_alpha"]
        
        # end time
        end_time = time.time()

        # Calculate elapsed time
        elapsed_time += end_time - start_time

        rendering.permute(1,2,0)

        rgbs.append(rendering)
        rgbs_gt.append(gt)
        alphas.append(alpha)

    # Calculate elapsed time
    print("Elapsed time: ", elapsed_time, " FPS: ", len(views)/elapsed_time) 

    for id in range(len(views)):
        rendering = rgbs[id]
        gt = rgbs_gt[id]
        rendering = torch.clamp(rendering, 0.0, 1.0)
        gt = torch.clamp(gt, 0.0, 1.0)
        alpha = torch.clamp(alphas[id], 0.0, 1.0)

        torchvision.utils.save_image(rendering, os.path.join(render_path, '{0:05d}'.format(id) + ".png"))
        torchvision.utils.save_image(alpha, os.path.join(render_path, '{0:05d}'.format(id) + "_alpha.png"))
        torchvision.utils.save_image(gt, os.path.join(gts_path, '{0:05d}'.format(id) + ".png"))


def render_sets(dataset : ModelParams, iteration : int, pipeline : PipelineParams, skip_train : bool, skip_test : bool, canonical : bool, canonical_pose : bool):
    with torch.no_grad():

        gaussians = GaussianModel(dataset.sh_degree, dataset.smpl_type, dataset.motion_offset_flag, dataset.actor_gender)
        scene = Scene(dataset, gaussians, load_iteration=iteration, shuffle=False)

        bg_color = [1,1,1] if dataset.white_background else [0, 0, 0]
        background = torch.tensor(bg_color, dtype=torch.float32, device="cuda")
   

        if not skip_train:
             render_set(dataset.model_path, "train", scene.loaded_iter, scene.getTrainCameras(), gaussians, pipeline, background, canonical, canonical_pose)

        if not skip_test:
             render_set(dataset.model_path, "test", scene.loaded_iter, scene.getTestCameras(), gaussians, pipeline, background, canonical, canonical_pose)




if __name__ == "__main__":
    # Set up command line argument parser
    parser = ArgumentParser(description="Testing script parameters")
    model = ModelParams(parser, sentinel=True)
    pipeline = PipelineParams(parser)
    parser.add_argument("--iteration", default=-1, type=int)
    parser.add_argument("--skip_train", action="store_true")
    parser.add_argument("--skip_test", action="store_true")
    parser.add_argument("--quiet", action="store_true")
    parser.add_argument("--canonical", action="store_true")
    parser.add_argument("--canonical_pose", action="store_true")
    args = get_combined_args(parser)
    print("Rendering " + args.model_path)

    # Initialize system state (RNG)
    safe_state(args.quiet)

    render_sets(model.extract(args), args.iteration, pipeline.extract(args), args.skip_train, args.skip_test, args.canonical, args.canonical_pose)

```

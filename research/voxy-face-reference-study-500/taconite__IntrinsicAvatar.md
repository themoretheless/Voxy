# Facial rendering source scan: taconite/IntrinsicAvatar

Repository: https://github.com/taconite/IntrinsicAvatar

Pinned source revision: b2c4fbf492b8ae2ff3ddafa8a30312a8ae06fde8

Discovery topic: human rendering

Description: [CVPR 2024] IntrinsicAvatar: Physically Based Inverse Rendering of Dynamic Humans from Monocular Videos via Explicit Ray Tracing

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md"]

Topic paths: {"deform": ["configs/deformer/non_rigid_deformer.yaml", "configs/deformer/non_rigid_deformer_long.yaml", "configs/deformer/snarf_deformer.yaml", "configs/non_rigid_deformer/dummy.yaml", "configs/non_rigid_deformer/mlp.yaml", "models/deformers/__init__.py", "models/deformers/deformer.py", "models/deformers/fast_snarf/cuda/filter/filter.cpp", "models/deformers/fast_snarf/cuda/filter/filter.cu", "models/deformers/fast_snarf/cuda/fuse_kernel/fuse_cuda.cpp", "models/deformers/fast_snarf/cuda/fuse_kernel/fuse_cuda_kernel_fast.cu", "models/deformers/fast_snarf/cuda/precompute/precompute.cpp", "models/deformers/fast_snarf/cuda/precompute/precompute.cu", "models/deformers/fast_snarf/deformer_torch.py", "models/deformers/non_rigid_deformer.py", "models/deformers/smplx/__init__.py", "models/deformers/smplx/body_models.py", "models/deformers/smplx/joint_names.py", "models/deformers/smplx/lbs.py", "models/deformers/smplx/utils.py", "models/deformers/smplx/vertex_ids.py", "models/deformers/smplx/vertex_joint_selector.py", "models/deformers/snarf_deformer.py"], "lbs": ["models/deformers/smplx/lbs.py", "scripts/easymocap/bodymodel/lbs.py", "scripts/easymocap/smplmodel/lbs.py"], "rig": ["configs/deformer/non_rigid_deformer.yaml", "configs/deformer/non_rigid_deformer_long.yaml", "configs/non_rigid_deformer/dummy.yaml", "configs/non_rigid_deformer/mlp.yaml", "models/deformers/non_rigid_deformer.py"]}

## Source file: models/deformers/non_rigid_deformer.py

{
  "path": "models/deformers/non_rigid_deformer.py",
  "blob_sha": "7d88956189b07beebf1f381a59b50ee0dfc2d466",
  "sha256": "3919b3ce962663f036fa3cf082e6d35967ed348f2791e7ccce80f088eb5656c7",
  "lines": 120,
  "bytes": 4108,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 3,
    "deform": 9,
    "rig": 6
  },
  "url": "https://github.com/taconite/IntrinsicAvatar/blob/b2c4fbf492b8ae2ff3ddafa8a30312a8ae06fde8/models/deformers/non_rigid_deformer.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self, config):"
    },
    {
      "line": 15,
      "text": "def prepare_bbox(self, bbox):"
    },
    {
      "line": 27,
      "text": "def setup(self):"
    },
    {
      "line": 30,
      "text": "def forward(self, points, cond, *args, with_jac=False):"
    },
    {
      "line": 39,
      "text": "def setup(self):"
    },
    {
      "line": 56,
      "text": "def forward(self, points, cond, *args, with_jac=False):"
    },
    {
      "line": 114,
      "text": "def update_step(self, epoch, global_step):"
    }
  ]
}

```text

import torch
import models

from torch.autograd import grad

from systems.utils import update_module_step
from models.base import BaseModel
from models.network_utils import get_encoding, get_mlp


class BaseNonRigidDeformer(BaseModel):
    def __init__(self, config):
        super().__init__(config)

    def prepare_bbox(self, bbox):
        if hasattr(self, "bbox"):
            return
        c = (bbox[0] + bbox[1]) / 2
        s = (bbox[1] - bbox[0])
        self.center = c
        self.scale = s
        self.bbox = bbox


@models.register('dummy_non_rigid_deformer')
class DummyNonRigidDeformer(BaseNonRigidDeformer):
    def setup(self):
        pass

    def forward(self, points, cond, *args, with_jac=False):
        J_inv = torch.eye(points.shape[1], device=points.device).unsqueeze(0).repeat(
            points.shape[0], 1, 1
        )
        return points, J_inv


@models.register('non-rigid-deformer')
class NonRigidDeformer(BaseNonRigidDeformer):
    def setup(self):
        self.n_input_dims = 3
        self.n_output_dims = 3
        xyz_encoding_config = self.config.get('xyz_encoding_config', None)
        xyz_encoding = (
            get_encoding(self.n_input_dims, xyz_encoding_config)
            if xyz_encoding_config is not None
            else None
        )

        network = get_mlp(xyz_encoding.n_output_dims, self.n_output_dims, self.config.mlp_network_config)
        self.xyz_encoding = xyz_encoding
        self.network = network

        self.kick_in_step = self.config.get('kick_in_step', 6000)
        self.enabled = False

    def forward(self, points, cond, *args, with_jac=False):
        if not self.enabled:
            J_inv = torch.eye(points.shape[1], device=points.device).unsqueeze(0).repeat(
                points.shape[0], 1, 1
            )
            return points, J_inv

        with torch.inference_mode(torch.is_inference_mode_enabled() and not with_jac):
            with torch.set_grad_enabled(self.training or with_jac):
                if with_jac:
                    if not self.training:
                        points = (
                            points.clone()
                        )  # points may be in inference mode, get a copy to enable grad
                    points.requires_grad_(True)

            points_ = points # points in the original scale
            if self.xyz_encoding is not None:
                points = (points - self.center) / self.scale.clone() + 0.5
                xyz_embd = self.xyz_encoding(points.view(-1, 3))
            else:
                xyz_embd = torch.empty(
                    points.shape[:1] + (0,), dtype=points.dtype, device=points.device
                )

            network_inp = xyz_embd
            deformed_points = (
                points_
                + self.network(network_inp, cond)
                .view(*points.shape[:-1], self.n_output_dims)
                .float()
            )

            if with_jac:
                # Compute inverse jacobian
                jac = torch.zeros(
                    points.shape[0],
                    points.shape[1],
                    points.shape[1],
                    device=points.device,
                )
                for i in range(points.shape[1]):
                    jac[:, i, :] = grad(
                        deformed_points[:, i],
                        points_,
                        torch.ones_like(deformed_points[:, i]),
                        create_graph=True,
                        retain_graph=True,
                    )[0]

                J_inv = jac.detach().inverse()
            else:
                J_inv = torch.eye(points.shape[1], device=points.device).unsqueeze(0).repeat(
                    points.shape[0], 1, 1
                )

            return deformed_points, J_inv

    def update_step(self, epoch, global_step):
        update_module_step(self.xyz_encoding, epoch, global_step)
        update_module_step(self.network, epoch, global_step)
        if global_step > self.kick_in_step:
            self.enabled = True
        else:
            self.enabled = False


```

## Source file: models/deformers/smplx/lbs.py

{
  "path": "models/deformers/smplx/lbs.py",
  "blob_sha": "da09262bc05d6bda20cf25c2e074cb54c8462848",
  "sha256": "75bfc58c183bba07dfb85f0e4980d36296deae769f304c7eace9401ab1a4e512",
  "lines": 401,
  "bytes": 13895,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 20
  },
  "url": "https://github.com/taconite/IntrinsicAvatar/blob/b2c4fbf492b8ae2ff3ddafa8a30312a8ae06fde8/models/deformers/smplx/lbs.py",
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
      "line": 152,
      "text": "def lbs("
    },
    {
      "line": 251,
      "text": "def vertices2joints(J_regressor: Tensor, vertices: Tensor) -> Tensor:"
    },
    {
      "line": 271,
      "text": "def blend_shapes(betas: Tensor, shape_disps: Tensor) -> Tensor:"
    },
    {
      "line": 295,
      "text": "def batch_rodrigues("
    },
    {
      "line": 332,
      "text": "def transform_mat(R: Tensor, t: Tensor) -> Tensor:"
    },
    {
      "line": 345,
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

    lmk_faces = torch.index_select(faces, 0, lmk_faces_idx.view(-1)).view(
        batch_size, -1, 3)

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
    shape_offsets = blend_shapes(betas, shapedirs)
    v_shaped = v_template + shape_offsets

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
    T = torch.matmul(W, A.view(batch_size, num_joints, 16)).view(batch_size, -1, 4, 4)

    homogen_coord = torch.ones([batch_size, v_posed.shape[1], 1],
                               dtype=dtype, device=device)
    v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
    v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

    verts = v_homo[:, :, :3, 0]

    return verts, J_transformed, A, T, shape_offsets, pose_offsets


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

## Source file: models/deformers/deformer.py

{
  "path": "models/deformers/deformer.py",
  "blob_sha": "f24432b0025c2414a90746b88a04ce97095371ce",
  "sha256": "8d6a9f8e79d256b290d3cce1e0d22af33c5d7c345a22b5edacea2034db8a910f",
  "lines": 76,
  "bytes": 2221,
  "selection_score": 15,
  "keyword_line_counts": {
    "deform": 21,
    "rig": 19
  },
  "url": "https://github.com/taconite/IntrinsicAvatar/blob/b2c4fbf492b8ae2ff3ddafa8a30312a8ae06fde8/models/deformers/deformer.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, config):"
    },
    {
      "line": 11,
      "text": "def prepare(self, batch):"
    },
    {
      "line": 17,
      "text": "def setup(self):"
    },
    {
      "line": 20,
      "text": "def forward(self, points, cond, geometry, *args, with_jac=False, eval_mode=False):"
    },
    {
      "line": 23,
      "text": "def set_initialized(self, is_initialized):"
    },
    {
      "line": 26,
      "text": "def get_rot_mats(self):"
    },
    {
      "line": 29,
      "text": "def get_joints(self):"
    },
    {
      "line": 35,
      "text": "def setup(self):"
    },
    {
      "line": 45,
      "text": "def prepare(self, batch):"
    },
    {
      "line": 49,
      "text": "def forward(self, points, cond, geometry_fn, *args, with_jac=False, eval_mode=False):"
    },
    {
      "line": 50,
      "text": "def non_rigid_geometry_fn(x):"
    },
    {
      "line": 62,
      "text": "def set_initialized(self, is_initialized):"
    },
    {
      "line": 68,
      "text": "def get_rot_mats(self):"
    },
    {
      "line": 71,
      "text": "def get_joints(self):"
    },
    {
      "line": 74,
      "text": "def update_step(self, epoch, global_step):"
    }
  ]
}

```text

import models

from systems.utils import update_module_step
from models.base import BaseModel


class BaseDeformer(BaseModel):
    def __init__(self, config):
        super().__init__(config)

    def prepare(self, batch):
        pass


@models.register("dummy_deformer")
class DummyNonRigidDeformer(BaseDeformer):
    def setup(self):
        pass

    def forward(self, points, cond, geometry, *args, with_jac=False, eval_mode=False):
        return points

    def set_initialized(self, is_initialized):
        pass

    def get_rot_mats(self):
        return None

    def get_joints(self):
        return None


@models.register("snarf_deformer")
class SNARFDeformer(BaseDeformer):
    def setup(self):
        self.n_input_dims = 3
        self.n_output_dims = 3
        self.rigid_deformer = models.make(
            self.config.rigid_deformer.name, self.config.rigid_deformer
        )
        self.non_rigid_deformer = models.make(
            self.config.non_rigid_deformer.name, self.config.non_rigid_deformer
        )

    def prepare(self, batch):
        self.rigid_deformer.prepare_deformer(batch)
        self.non_rigid_deformer.prepare_bbox(self.rigid_deformer.bbox)

    def forward(self, points, cond, geometry_fn, *args, with_jac=False, eval_mode=False):
        def non_rigid_geometry_fn(x):
            x, J_inv = self.non_rigid_deformer(x, cond=cond, with_jac=with_jac)
            ret = geometry_fn(x)

            return ret, J_inv

        return self.rigid_deformer(
            points,
            non_rigid_geometry_fn,
            eval_mode=eval_mode,
        )

    def set_initialized(self, is_initialized):
        if hasattr(self.rigid_deformer, "initialized"):
            self.rigid_deformer.initialized = is_initialized
        if hasattr(self.non_rigid_deformer, "initialized"):
            self.non_rigid_deformer.initialized = is_initialized

    def get_rot_mats(self):
        return self.rigid_deformer.rot_mats

    def get_joints(self):
        return self.rigid_deformer.basic_joints

    def update_step(self, epoch, global_step):
        update_module_step(self.rigid_deformer, epoch, global_step)
        update_module_step(self.non_rigid_deformer, epoch, global_step)


```

## Source file: models/deformers/snarf_deformer.py

{
  "path": "models/deformers/snarf_deformer.py",
  "blob_sha": "333deafa6be7b7292c2735d33957e8dad69e8720",
  "sha256": "1c4ea3eb477c39756f06c5af77743b63bc9f1ec8bdb0ffd5c79e2f126f40b3f8",
  "lines": 264,
  "bytes": 11160,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "deform": 21,
    "lbs": 3,
    "face": 3
  },
  "url": "https://github.com/taconite/IntrinsicAvatar/blob/b2c4fbf492b8ae2ff3ddafa8a30312a8ae06fde8/models/deformers/snarf_deformer.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def get_predefined_rest_pose(cano_pose, device=\"cuda\"):"
    },
    {
      "line": 24,
      "text": "def get_bbox_from_smpl(vs, factor=1.2):"
    },
    {
      "line": 40,
      "text": "def __init__(self, config) -> None:"
    },
    {
      "line": 46,
      "text": "def initialize(self, betas, device):"
    },
    {
      "line": 81,
      "text": "def prepare_deformer(self, smpl_params):"
    },
    {
      "line": 128,
      "text": "def transform_rays_w2s(self, rays):"
    },
    {
      "line": 146,
      "text": "def transform_dirs_w2s(self, d):"
    },
    {
      "line": 154,
      "text": "def transform_dirs_s2w(self, d):"
    },
    {
      "line": 160,
      "text": "def transform_rots_s2w(self, J_inv):"
    },
    {
      "line": 165,
      "text": "def get_bbox_deformed(self):"
    },
    {
      "line": 169,
      "text": "def deform_(self, pts, eval_mode):"
    },
    {
      "line": 187,
      "text": "def deform(self, pts, model, eval_mode):"
    },
    {
      "line": 263,
      "text": "def __call__(self, pts, model, eval_mode=True):"
    }
  ]
}

```text

from .fast_snarf.deformer_torch import ForwardDeformer
from .smplx import SMPL
import torch
import torch.nn.functional as F

from torchgeometry.core import conversions


def get_predefined_rest_pose(cano_pose, device="cuda"):
    body_pose_t = torch.zeros((1, 69), device=device)
    if cano_pose.lower() == "da_pose":
        body_pose_t[:, 2] = torch.pi / 6
        body_pose_t[:, 5] = -torch.pi / 6
    elif cano_pose.lower() == "a_pose":
        body_pose_t[:, 2] = 0.2
        body_pose_t[:, 5] = -0.2
        body_pose_t[:, 47] = -0.8
        body_pose_t[:, 50] = 0.8
    else:
        raise ValueError("Unknown cano_pose: {}".format(cano_pose))
    return body_pose_t


def get_bbox_from_smpl(vs, factor=1.2):
    assert vs.shape[0] == 1
    min_vert = vs.min(dim=1).values
    max_vert = vs.max(dim=1).values

    c = (max_vert + min_vert) / 2
    s = (max_vert - min_vert) / 2
    s = s.max(dim=-1).values * factor

    min_vert = c - s[:, None]
    max_vert = c + s[:, None]
    return torch.cat([min_vert, max_vert], dim=0)


class SNARFDeformer():
    # def __init__(self, model_path, gender, opt) -> None:
    def __init__(self, config) -> None:
        self.body_model = SMPL(config.model_path, gender=config.gender)
        self.deformer = ForwardDeformer(config.deformer_config)
        self.initialized = False
        self.opt = config.deformer_config

    def initialize(self, betas, device):
        if isinstance(self.opt.cano_pose, str):
            body_pose_t = get_predefined_rest_pose(self.opt.cano_pose, device=device)
        else:
            body_pose_t = torch.zeros((1, 69), device=device)
            body_pose_t[:, 2] = self.opt.cano_pose[0]
            body_pose_t[:, 5] = self.opt.cano_pose[1]
            body_pose_t[:, 47] = self.opt.cano_pose[2]
            body_pose_t[:, 50] = self.opt.cano_pose[3]

        smpl_outputs = self.body_model(betas=betas[:1], body_pose=body_pose_t)
        self.tfs_inv_t = torch.inverse(smpl_outputs.A.float().detach())
        self.vs_template = smpl_outputs.vertices
        part_ids_verts = self.body_model.lbs_weights.argmax(dim=-1)
        part_ids_faces = part_ids_verts[self.body_model.faces_tensor]
        self.faces_to_include = (part_ids_faces < 20).any(
            -1
        )  # exclude hand faces when computing inside losses

        # initialize SNARF
        self.deformer.device = device
        self.deformer.switch_to_explicit(resolution=self.opt.resolution,
                                         smpl_verts=smpl_outputs.vertices.float().detach(),
                                         smpl_weights=self.body_model.lbs_weights.clone()[None].detach(),
                                         use_smpl=True)
        self.bbox = get_bbox_from_smpl(smpl_outputs.vertices.detach())

        self.dtype = torch.float32
        self.deformer.lbs_voxel_final = self.deformer.lbs_voxel_final.type(self.dtype)
        self.deformer.grid_denorm = self.deformer.grid_denorm.type(self.dtype)
        self.deformer.scale = self.deformer.scale.type(self.dtype)
        self.deformer.offset = self.deformer.offset.type(self.dtype)
        self.deformer.scale_kernel = self.deformer.scale_kernel.type(self.dtype)
        self.deformer.offset_kernel = self.deformer.offset_kernel.type(self.dtype)

    def prepare_deformer(self, smpl_params):
        device = smpl_params["betas"].device
        if self.opt.optimize_betas:
            betas = smpl_params["betas"] + smpl_params["betas_correction"]
        else:
            betas = smpl_params["betas"]
        if next(self.body_model.parameters()).device != device:
            self.body_model = self.body_model.to(device)
        if not self.initialized:
            self.initialize(betas.detach(), device)
            self.initialized = True

        body_pose = smpl_params["body_pose"] + smpl_params["pose_correction"]
        global_orient = (
            smpl_params["global_orient"] + smpl_params["global_orient_correction"]
        )
        transl = smpl_params["transl"] + smpl_params["transl_correction"]
        smpl_outputs = self.body_model(
            betas=betas, body_pose=body_pose, global_orient=global_orient, transl=transl
        )
        s2w = smpl_outputs.A[:, 0].float()
        w2s = torch.inverse(s2w)

        tfs = (w2s[:, None] @ smpl_outputs.A.float() @ self.tfs_inv_t).type(self.dtype)
        self.deformer.precompute(tfs)

        self.w2s = w2s
        self.vertices = (smpl_outputs.vertices @ w2s[:, :3, :3].permute(0, 2, 1)) + w2s[
            :, None, :3, 3
        ]
        self.basic_joints = (
            smpl_outputs.joints[:, :24] @ w2s[:, :3, :3].permute(0, 2, 1)
        ) + w2s[:, None, :3, 3]
        rot_mats = conversions.angle_axis_to_rotation_matrix(
            smpl_params["body_pose"].reshape(-1, 3)
        )[:, :3, :3].reshape(-1, 23, 3, 3)
        self.rot_mats = torch.cat(
            [
                torch.eye(3, device=device)[None].repeat(rot_mats.shape[0], 1, 1, 1),
                rot_mats,
            ],
            dim=1,
        ).reshape(-1, 24, 9)
        self.tfs = tfs
        self.smpl_outputs = smpl_outputs
        self.smpl_params = smpl_params

    def transform_rays_w2s(self, rays):
        """transform rays from world to smpl coordinate system"""
        w2s = self.w2s.detach()
        assert (w2s.shape[0] == 1)

        # rays[:, :3] = (rays[:, :3] @ w2s[:, :3, :3].permute(0, 2, 1)) + w2s[:, None, :3, 3]
        # rays[:, 3:6] = (rays[:, 3:6] @ w2s[:, :3, :3].permute(0, 2, 1)).to(rays)
        # d = torch.norm(rays[:, :3], dim=-1)
        # rays[:, 6] = d - 1
        # rays[:, 7] = d + 1
        rays_o = (rays[:, :3] @ w2s[0, :3, :3].permute(1, 0)) + w2s[0, None, :3, 3]
        rays_d = rays[:, 3:6] @ w2s[0, :3, :3].permute(1, 0)
        d = torch.linalg.norm(rays_o, dim=-1, keepdim=True)
        near = d - 1
        far = d + 1

        return torch.cat([rays_o, rays_d, near, far], dim=-1)

    def transform_dirs_w2s(self, d):
        """transform directions from world to smpl coordinate system"""
        w2s = self.w2s.detach()
        assert (w2s.shape[0] == 1)
        return F.normalize(
            (d @ w2s[0, :3, :3].permute(1, 0)).to(d), dim=-1, eps=1e-6
        )

    def transform_dirs_s2w(self, d):
        """transform directions from smpl to world coordinate system"""
        w2s = self.w2s.detach()
        assert (w2s.shape[0] == 1)
        return F.normalize((d @ w2s[0, :3, :3]).to(d), dim=-1, eps=1e-6)

    def transform_rots_s2w(self, J_inv):
        """apply smpl->world rotation to J_inv"""
        w2s = self.w2s.detach()
        return (w2s[:, :3, :3].permute(0, 2, 1) @ J_inv).to(J_inv)

    def get_bbox_deformed(self):
        voxel = self.deformer.voxel_d[0].reshape(3, -1)
        return [voxel.min(dim=1).values, voxel.max(dim=1).values]

    def deform_(self, pts, eval_mode):
        """transform pts to canonical space"""
        point_size = pts.shape[0]
        betas = self.smpl_outputs.betas
        batch_size = betas.shape[0]
        pts = pts.reshape(batch_size, -1, 3)

        pts_cano, others = self.deformer.forward(pts, cond=None, tfs=self.tfs, eval_mode=eval_mode)
        valid = others["valid_ids"].reshape(point_size, -1)
        if self.opt.use_j_inv:
            # Use the (approximated) inverse Jacobian to transform normals
            J_inv = others["J_inv"].reshape(point_size, -1, 3, 3)
        else:
            # Use the linearly blended bone transforms to transform normals
            J_inv = others["fwd_tfs"].reshape(point_size, -1, 3, 3)

        return pts_cano.reshape(point_size, -1, 3), J_inv, valid

    def deform(self, pts, model, eval_mode):
        pts_cano_all, J_inv, valid = self.deform_(pts.type(self.dtype), eval_mode=eval_mode)

        c2w = J_inv[valid]

        sdf_cano = torch.ones_like(pts_cano_all[..., 0]).float() * 1e5  # canonical SDF
        sdf_grad = None # SDF gradient in observation space
        sdf_grad_cano = None    # SDF gradient in canonical space
        features = None # features from the canonical SDF field
        laplace = None  # Laplacian of the canonical SDF field

        if not torch.isfinite(pts_cano_all).all():
            print("WARNING: NaN found in pts_cano_all")
        # Note that model should also take care of the case where input Tensor is empty
        # (i.e. all points are invalid), and return empty tensors
        model_ret, J_inv_nr = model(pts_cano_all[valid])
        # if J_inv_nr is not None:
        c2w = c2w @ J_inv_nr
        any_valid = valid.any()
        if isinstance(model_ret, tuple) or isinstance(model_ret, list):
            if any_valid:
                sdf_cano[valid] = model_ret[0]

            if len(model_ret) > 1:
                sdf_grad_cano = torch.tensor(
                    [0, 0, 1], dtype=torch.float32, device=pts_cano_all.device
                ).repeat((*pts_cano_all.shape[:2], 1))
                if any_valid:
                    sdf_grad_cano[valid] = model_ret[1]
                assert c2w is not None
                sdf_grad = torch.tensor(
                    [0, 0, 1], dtype=torch.float32, device=pts_cano_all.device
                ).repeat((*pts_cano_all.shape[:2], 1))
                if any_valid:
                    sdf_grad[valid] = torch.einsum(
                        "bij,bj->bi", c2w, sdf_grad_cano[valid]
                    )
            if len(model_ret) > 2:
                features = torch.zeros(
                    (*pts_cano_all.shape[:2], model_ret[2].size(-1)),
                    dtype=torch.float32,
                    device=pts_cano_all.device,
                )
                if any_valid:
                    features[valid] = model_ret[2]
            if len(model_ret) > 3:
                laplace = torch.zeros_like(pts_cano_all[..., 0]).float()
                if any_valid:
                    laplace[valid] = model_ret[3]
        elif isinstance(model_ret, torch.Tensor):
            if any_valid:
                sdf_cano[valid] = model_ret
        else:
            raise ValueError("Invalid return type from model")

        sdf_cano, idx = torch.min(sdf_cano, dim=-1)
        pts_cano = torch.gather(pts_cano_all, 1, idx[:, None, None].repeat(1, 1, 3)).reshape(-1, 3)
        valid_cano = valid.any(dim=-1)
        out = [pts_cano, sdf_cano, valid_cano]
        if sdf_grad is not None:
            sdf_grad = torch.gather(sdf_grad, 1, idx[:, None, None].repeat(1, 1, 3)).reshape(-1, 3)
            out.append(sdf_grad)
        if sdf_grad_cano is not None:
            sdf_grad_cano = torch.gather(sdf_grad_cano, 1, idx[:, None, None].repeat(1, 1, 3)).reshape(-1, 3)
            out.append(sdf_grad_cano)
        if features is not None:
            features = torch.gather(
                features, 1, idx[:, None, None].repeat(1, 1, features.size(-1))
            ).reshape(-1, features.size(-1))
            out.append(features)
        if laplace is not None:
            laplace = torch.gather(laplace, 1, idx[:, None]).reshape(-1)
            out.append(laplace)

        return out

    def __call__(self, pts, model, eval_mode=True):
        return self.deform(pts, model, eval_mode)


```

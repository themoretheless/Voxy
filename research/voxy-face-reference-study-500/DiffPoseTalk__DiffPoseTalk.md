# Facial rendering source scan: DiffPoseTalk/DiffPoseTalk

Repository: https://github.com/DiffPoseTalk/DiffPoseTalk

Pinned source revision: 2c761f174776ffe380cd89b875fc8eb49e90a5a1

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"mouth": ["demo/input/style/L4H4-T0.1-BS32/iter_0034000/big_mouth.npy", "demo/input/style/L4H4-T0.1-BS32/iter_0034000/small_mouth.npy", "demo/input/style/head-L4H4-T0.1-BS32/iter_0026000/small_mouth_serious.npy"], "flame": ["models/flame.py"], "lbs": ["models/lbs.py"]}

## Source file: models/lbs.py

{
  "path": "models/lbs.py",
  "blob_sha": "fc2065fd60691fb94e29663a900c6fa94f2e51af",
  "sha256": "a39c5b8a420c2a589b11e023946986a65c2e45ad846c3899b85115fc4498a2a6",
  "lines": 371,
  "bytes": 13444,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 19
  },
  "url": "https://github.com/DiffPoseTalk/DiffPoseTalk/blob/2c761f174776ffe380cd89b875fc8eb49e90a5a1/models/lbs.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 35,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 102,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 141,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 226,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 246,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 270,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 304,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 317,
      "text": "def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):"
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
from __future__ import division
from __future__ import print_function

import numpy as np
import torch
import torch.nn.functional as F


def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] +
                    rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,
                                     dynamic_lmk_b_coords,
                                     neck_kin_chain, dtype=torch.float32):
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

    batch_size = vertices.shape[0]

    aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
                                 neck_kin_chain)
    rot_mats = batch_rodrigues(
        aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)

    rel_rot_mat = torch.eye(3, device=vertices.device,
                            dtype=dtype).unsqueeze_(dim=0)
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


def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):
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


def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,
        lbs_weights, pose2rot=True, dtype=torch.float32):
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
    device = betas.device

    # Add shape contribution
    v_shaped = v_template + blend_shapes(betas, shapedirs)

    # Get the joints
    # NxJx3 array
    J = vertices2joints(J_regressor, v_shaped)

    # 3. Add pose blend shapes
    # N x J x 3 x 3
    ident = torch.eye(3, dtype=dtype, device=device)
    if pose2rot:
        rot_mats = batch_rodrigues(
            pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])
    else:
        rot_mats = pose.view(batch_size, -1, 3, 3)

    pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
    # (N x P) x (P, V * 3) -> N x V x 3
    pose_offsets = torch.matmul(pose_feature, posedirs) \
        .view(batch_size, -1, 3)
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

    return verts, J_transformed


def vertices2joints(J_regressor, vertices):
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


def blend_shapes(betas, shape_disps):
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


def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):
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
    device = rot_vecs.device

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


def transform_mat(R, t):
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


def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):
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

    # transforms_mat = transform_mat(
    #     rot_mats.view(-1, 3, 3),
    #     rel_joints.view(-1, 3, 1)).view(-1, joints.shape[1], 4, 4)
    transforms_mat = transform_mat(
        rot_mats.view(-1, 3, 3),
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

## Source file: models/flame.py

{
  "path": "models/flame.py",
  "blob_sha": "ff505980727d64edcdc766bc33011c116dddd02e",
  "sha256": "e04dbb4c06ec597eb0d205369e986e52a1b387687ed955cbefd9ea42360ac7d0",
  "lines": 299,
  "bytes": 13891,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 18,
    "jaw": 1,
    "flame": 29,
    "lbs": 4,
    "rig": 5,
    "face": 20
  },
  "url": "https://github.com/DiffPoseTalk/DiffPoseTalk/blob/2c761f174776ffe380cd89b875fc8eb49e90a5a1/models/flame.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 33,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 40,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 64,
      "text": "def __init__(self, config):"
    },
    {
      "line": 124,
      "text": "def _find_dynamic_lmk_idx_and_bcoords(self, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 172,
      "text": "def seletec_3d68(self, vertices):"
    },
    {
      "line": 178,
      "text": "def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, pose2rot=True,"
    },
    {
      "line": 253,
      "text": "def __init__(self, config):"
    },
    {
      "line": 283,
      "text": "def forward(self, texcode):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
#
# Max-Planck-Gesellschaft zur Förderung der Wissenschaften e.V. (MPG) is
# holder of all proprietary rights on this computer program.
# Using this computer program means that you agree to the terms
# in the LICENSE file included with this software distribution.
# Any use not explicitly granted by the LICENSE is prohibited.
#
# Copyright©2019 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# For comments or questions, please email us at deca@tue.mpg.de
# For commercial licensing contact, please contact ps-license@tuebingen.mpg.de

import pickle
from pathlib import Path
from types import SimpleNamespace

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

from .lbs import lbs, batch_rodrigues, vertices2landmarks, rot_mat_to_euler


def to_tensor(array, dtype=torch.float32):
    if 'torch.tensor' not in str(type(array)):
        return torch.tensor(array, dtype=dtype)


def to_np(array, dtype=np.float32):
    if 'scipy.sparse' in str(type(array)):
        array = array.todense()
    return np.array(array, dtype=dtype)


class Struct(object):
    def __init__(self, **kwargs):
        for key, val in kwargs.items():
            setattr(self, key, val)


__dir__ = Path(__file__).parent.absolute()
FLAMEConfig = SimpleNamespace(
    flame_model_path=str(__dir__ / 'data/FLAME2020/generic_model.pkl'),
    n_shape=100,
    n_exp=50,
    n_tex=50,
    tex_type='BFM',
    tex_path=str(__dir__ / 'data/FLAME2020/FLAME_albedo_from_BFM.npz'),
    flame_lmk_embedding_path=str(__dir__ / 'data/landmark_embedding.npy')
)


class FLAME(nn.Module):
    """
    borrowed from https://github.com/soubhiksanyal/FLAME_PyTorch/blob/master/FLAME.py
    Given flame parameters this class generates a differentiable FLAME function
    which outputs the a mesh and 2D/3D facial landmarks
    """

    def __init__(self, config):
        super(FLAME, self).__init__()
        # print('creating the FLAME Decoder')
        with open(config.flame_model_path, 'rb') as f:
            ss = pickle.load(f, encoding='latin1')
            flame_model = Struct(**ss)

        self.dtype = torch.float32
        self.register_buffer('faces_tensor', to_tensor(to_np(flame_model.f, dtype=np.int64), dtype=torch.long))
        # The vertices of the template model
        self.register_buffer('v_template', to_tensor(to_np(flame_model.v_template), dtype=self.dtype))
        # The shape components and expression
        shapedirs = to_tensor(to_np(flame_model.shapedirs), dtype=self.dtype)
        shapedirs = torch.cat([shapedirs[:, :, :config.n_shape], shapedirs[:, :, 300:300 + config.n_exp]], 2)
        self.register_buffer('shapedirs', shapedirs)
        # The pose components
        num_pose_basis = flame_model.posedirs.shape[-1]
        posedirs = np.reshape(flame_model.posedirs, [-1, num_pose_basis]).T
        self.register_buffer('posedirs', to_tensor(to_np(posedirs), dtype=self.dtype))
        #
        self.register_buffer('J_regressor', to_tensor(to_np(flame_model.J_regressor), dtype=self.dtype))
        parents = to_tensor(to_np(flame_model.kintree_table[0])).long()
        parents[0] = -1
        self.register_buffer('parents', parents)
        self.register_buffer('lbs_weights', to_tensor(to_np(flame_model.weights), dtype=self.dtype))

        # Fixing Eyeball and neck rotation
        default_eyeball_pose = torch.zeros([1, 6], dtype=self.dtype, requires_grad=False)
        self.register_parameter('eye_pose', nn.Parameter(default_eyeball_pose,
                                                         requires_grad=False))
        default_eyeball_pose_mat = torch.eye(3, dtype=self.dtype, requires_grad=False).view(1, 9).repeat(1, 2)
        self.register_parameter('eye_pose_mat', nn.Parameter(default_eyeball_pose_mat,
                                                             requires_grad=False))
        default_neck_pose = torch.zeros([1, 3], dtype=self.dtype, requires_grad=False)
        self.register_parameter('neck_pose', nn.Parameter(default_neck_pose,
                                                          requires_grad=False))
        default_neck_pose_mat = torch.eye(3, dtype=self.dtype, requires_grad=False).view(1, 9)
        self.register_parameter('neck_pose_mat', nn.Parameter(default_neck_pose_mat,
                                                              requires_grad=False))

        # Static and Dynamic Landmark embeddings for FLAME
        lmk_embeddings = np.load(config.flame_lmk_embedding_path, allow_pickle=True, encoding='latin1')
        lmk_embeddings = lmk_embeddings[()]
        self.register_buffer('lmk_faces_idx', torch.from_numpy(lmk_embeddings['static_lmk_faces_idx']).long())
        self.register_buffer('lmk_bary_coords',
                             torch.from_numpy(lmk_embeddings['static_lmk_bary_coords']).to(self.dtype))
        self.register_buffer('dynamic_lmk_faces_idx', lmk_embeddings['dynamic_lmk_faces_idx'].long())
        self.register_buffer('dynamic_lmk_bary_coords', lmk_embeddings['dynamic_lmk_bary_coords'].to(self.dtype))
        self.register_buffer('full_lmk_faces_idx', torch.from_numpy(lmk_embeddings['full_lmk_faces_idx']).long())
        self.register_buffer('full_lmk_bary_coords',
                             torch.from_numpy(lmk_embeddings['full_lmk_bary_coords']).to(self.dtype))

        neck_kin_chain = []
        NECK_IDX = 1
        curr_idx = torch.tensor(NECK_IDX, dtype=torch.long)
        while curr_idx != -1:
            neck_kin_chain.append(curr_idx)
            curr_idx = self.parents[curr_idx]
        self.register_buffer('neck_kin_chain', torch.stack(neck_kin_chain))

    def _find_dynamic_lmk_idx_and_bcoords(self, pose, dynamic_lmk_faces_idx,
                                          dynamic_lmk_b_coords,
                                          neck_kin_chain, dtype=torch.float32, pose2rot=True):
        """
            Selects the face contour depending on the reletive position of the head
            Input:
                vertices: N X num_of_vertices X 3
                pose: N X full pose
                dynamic_lmk_faces_idx: The list of contour face indexes
                dynamic_lmk_b_coords: The list of contour barycentric weights
                neck_kin_chain: The tree to consider for the relative rotation
                dtype: Data type
            return:
                The contour face indexes and the corresponding barycentric weights
        """

        batch_size = pose.shape[0]

        if pose2rot:
            aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
                                         neck_kin_chain)
            rot_mats = batch_rodrigues(
                aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)
        else:
            rot_mats = torch.index_select(pose.view(batch_size, -1, 9), 1,
                                          neck_kin_chain).view(batch_size, -1, 3, 3)

        rel_rot_mat = torch.eye(3, device=pose.device,
                                dtype=dtype).unsqueeze_(dim=0).expand(batch_size, -1, -1)
        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

        y_rot_angle = torch.round(
            torch.clamp(rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi,
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

    def seletec_3d68(self, vertices):
        landmarks3d = vertices2landmarks(vertices, self.faces_tensor,
                                         self.full_lmk_faces_idx.repeat(vertices.shape[0], 1),
                                         self.full_lmk_bary_coords.repeat(vertices.shape[0], 1, 1))
        return landmarks3d

    def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, pose2rot=True,
                ignore_global_rot=False, return_lm2d=True, return_lm3d=True):
        """
            Input:
                shape_params: N X number of shape parameters
                expression_params: N X number of expression parameters
                pose_params: N X number of pose parameters (6)
            return:d
                vertices: N X V X 3
                landmarks: N X number of landmarks X 3
        """
        batch_size = shape_params.shape[0]
        betas = torch.cat([shape_params, expression_params], dim=1)

        if pose2rot:
            # The pose parameters:
            #   global rotation (3), head rotation around the neck (3), jaw rotation (3), each of the eyeballs (3+3)
            if pose_params is None:
                pose_params = self.eye_pose.expand(batch_size, -1)
            if eye_pose_params is None:
                eye_pose_params = self.eye_pose.expand(batch_size, -1)
            head_pose = pose_params[:, :3] if not ignore_global_rot else torch.zeros_like(pose_params[:, :3])
            full_pose = torch.cat(
                [head_pose, self.neck_pose.expand(batch_size, -1), pose_params[:, 3:], eye_pose_params], dim=1)
        else:
            if pose_params is None:
                pose_params = self.eye_pose_mat.expand(batch_size, -1)
            if eye_pose_params is None:
                eye_pose_params = self.eye_pose_mat.expand(batch_size, -1)
            head_pose = pose_params[:, :9] if not ignore_global_rot else self.eye_pose_mat.expand(batch_size, -1)[:, :9]
            full_pose = torch.cat(
                [head_pose, self.neck_pose_mat.expand(batch_size, -1), pose_params[:, 9:], eye_pose_params], dim=1)
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)

        vertices, _ = lbs(betas, full_pose, template_vertices,
                          self.shapedirs, self.posedirs,
                          self.J_regressor, self.parents,
                          self.lbs_weights, pose2rot, self.dtype)

        if return_lm2d:
            lmk_faces_idx = self.lmk_faces_idx.unsqueeze(dim=0).expand(batch_size, -1)
            lmk_bary_coords = self.lmk_bary_coords.unsqueeze(dim=0).expand(batch_size, -1, -1)

            dyn_lmk_faces_idx, dyn_lmk_bary_coords = self._find_dynamic_lmk_idx_and_bcoords(
                full_pose, self.dynamic_lmk_faces_idx,
                self.dynamic_lmk_bary_coords,
                self.neck_kin_chain, dtype=self.dtype, pose2rot=pose2rot)
            lmk_faces_idx = torch.cat([dyn_lmk_faces_idx, lmk_faces_idx], 1)
            lmk_bary_coords = torch.cat([dyn_lmk_bary_coords, lmk_bary_coords], 1)

            landmarks2d = vertices2landmarks(vertices, self.faces_tensor,
                                             lmk_faces_idx,
                                             lmk_bary_coords)
        else:
            landmarks2d = None

        if return_lm3d:
            bz = vertices.shape[0]
            landmarks3d = vertices2landmarks(vertices, self.faces_tensor,
                                             self.full_lmk_faces_idx.repeat(bz, 1),
                                             self.full_lmk_bary_coords.repeat(bz, 1, 1))
        else:
            landmarks3d = None

        return vertices, landmarks2d, landmarks3d


class FLAMETex(nn.Module):
    """
    FLAME texture:
    https://github.com/TimoBolkart/TF_FLAME/blob/ade0ab152300ec5f0e8555d6765411555c5ed43d/sample_texture.py#L64
    FLAME texture converted from BFM:
    https://github.com/TimoBolkart/BFM_to_FLAME
    """

    def __init__(self, config):
        super(FLAMETex, self).__init__()
        if config.tex_type == 'BFM':
            mu_key = 'MU'
            pc_key = 'PC'
            n_pc = 199
            tex_path = config.tex_path
            tex_space = np.load(tex_path)
            texture_mean = tex_space[mu_key].reshape(1, -1)
            texture_basis = tex_space[pc_key].reshape(-1, n_pc)

        elif config.tex_type == 'FLAME':
            mu_key = 'mean'
            pc_key = 'tex_dir'
            n_pc = 200
            tex_path = config.flame_tex_path
            tex_space = np.load(tex_path)
            texture_mean = tex_space[mu_key].reshape(1, -1) / 255.
            texture_basis = tex_space[pc_key].reshape(-1, n_pc) / 255.
        else:
            print('texture type ', config.tex_type, 'not exist!')
            raise NotImplementedError

        n_tex = config.n_tex
        num_components = texture_basis.shape[1]
        texture_mean = torch.from_numpy(texture_mean).float()[None, ...]
        texture_basis = torch.from_numpy(texture_basis[:, :n_tex]).float()[None, ...]
        self.register_buffer('texture_mean', texture_mean)
        self.register_buffer('texture_basis', texture_basis)

    def forward(self, texcode):
        '''
        texcode: [batchsize, n_tex]
        texture: [bz, 3, 256, 256], range: 0-1
        '''

        bs = texcode.shape[0]
        texcode = texcode[:1]

        # we use the same (first frame) texture for all frames

        texture = self.texture_mean + (self.texture_basis * texcode[:, None, :]).sum(-1)

        texture = texture.reshape(texcode.shape[0], 512, 512, 3).permute(0, 3, 1, 2)
        texture = F.interpolate(texture, [256, 256])
        texture = texture[:, [2, 1, 0], :, :].repeat(bs, 1, 1, 1)
        return texture


```

## Source file: main_se.py

{
  "path": "main_se.py",
  "blob_sha": "d05723b87ca8f09813e38b3bd74628f049a6382a",
  "sha256": "f26c48d79ecbb572bba67ddc2142aab3528e07a53efaa18e27fa467f0dc42929",
  "lines": 182,
  "bytes": 6532,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 1
  },
  "url": "https://github.com/DiffPoseTalk/DiffPoseTalk/blob/2c761f174776ffe380cd89b875fc8eb49e90a5a1/main_se.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def train(args, model: StyleEncoder, train_loader, val_loader, optimizer, save_dir,"
    },
    {
      "line": 69,
      "text": "def test(args, model: StyleEncoder, test_loader, current_iter, n_rounds=10, mode='val',"
    },
    {
      "line": 100,
      "text": "def main(args, option_text=None):"
    }
  ]
}

```text

import argparse
from collections import deque
from datetime import datetime
from pathlib import Path

import numpy as np
import torch
from colorama import Fore, Back, Style
from tensorboardX import SummaryWriter
from torch.utils import data
from tqdm import tqdm

import options.se as options
import utils
from data import LmdbDatasetForSE, infinite_data_loader
from models import StyleEncoder


def train(args, model: StyleEncoder, train_loader, val_loader, optimizer, save_dir,
          writer=None):
    device = model.device
    save_dir.mkdir(parents=True, exist_ok=True)

    model.encoder.train()
    data_loader = infinite_data_loader(train_loader)

    loss_log = deque(maxlen=args.log_smooth_win)
    pbar = tqdm(range(args.max_iter + 1), dynamic_ncols=True)
    optimizer.zero_grad()
    for it in pbar:
        # Load data
        coef_pair = next(data_loader)
        coef_pair = [coef.to(device) for coef in coef_pair]

        # Forward
        feat_a = model(coef_pair[0])
        feat_b = model(coef_pair[1])

        loss = utils.nt_xent_loss(feat_a, feat_b, args.temperature)

        # Backward
        loss.backward()
        optimizer.step()
        optimizer.zero_grad()

        # Logging
        loss_log.append(loss.item())
        description = f'Train loss: [{np.mean(loss_log):.3e}]'
        pbar.set_description(description)

        if it % args.log_iter == 0 and writer is not None:
            # write to tensorboard
            writer.add_scalar('train/loss', np.mean(loss_log), it)

        # Validation
        if (it % args.val_iter == 0 and it != 0) or it == args.max_iter:
            test(args, model, val_loader, it, 200, 'val', writer)

        # save model
        if (it % args.save_iter == 0 and it != 0) or it == args.max_iter:
            torch.save({
                'args': args,
                'encoder': model.encoder.state_dict(),
                'iter': it,
            }, save_dir / f'iter_{it:07}.pt')


@torch.no_grad()
def test(args, model: StyleEncoder, test_loader, current_iter, n_rounds=10, mode='val',
         writer=None):
    is_training = model.encoder.training
    device = model.device
    model.encoder.eval()

    loss_log = []
    for test_round in range(n_rounds):
        for coef_pair in test_loader:
            # Load data
            coef_pair = [coef.to(device) for coef in coef_pair]

            # Forward
            feat_a = model(coef_pair[0])
            feat_b = model(coef_pair[1])
            loss = utils.nt_xent_loss(feat_a, feat_b, args.temperature)

            # Logging
            loss_log.append(loss.item())

    description = f'(Iter {current_iter:>6}) {mode} loss: [{np.mean(loss_log):.3e}]'
    print(description)

    if writer is not None:
        # write to tensorboard
        writer.add_scalar(f'{mode}/loss', np.mean(loss_log), current_iter)

    if is_training:
        model.encoder.train()


def main(args, option_text=None):
    device = 'cuda' if torch.cuda.is_available() else 'cpu'

    # Load data
    data_root = Path(args.data_root)
    coef_stats_file: Path = args.stats_file
    if not coef_stats_file.is_absolute():
        coef_stats_file = data_root / coef_stats_file

    if args.mode == 'train':
        # Build model
        model = StyleEncoder(args).to(device)

        # Dataset
        train_dataset = LmdbDatasetForSE(data_root, args.data_root / 'train.txt', coef_stats_file, args.fps,
                                         args.n_motions,
                                         rot_repr=args.rot_repr, no_head_pose=args.no_head_pose)
        val_dataset = LmdbDatasetForSE(data_root, args.data_root / 'val.txt', coef_stats_file, args.fps, args.n_motions,
                                       rot_repr=args.rot_repr, no_head_pose=args.no_head_pose)
        train_loader = data.DataLoader(train_dataset, batch_size=args.batch_size, shuffle=True,
                                       num_workers=args.num_workers, pin_memory=True, drop_last=True,
                                       persistent_workers=True)
        val_loader = data.DataLoader(val_dataset, batch_size=args.batch_size, shuffle=True,
                                     num_workers=args.num_workers, drop_last=True)

        # Logging
        exp_dir = Path('experiments/SE') / f'{args.exp_name}-{datetime.now().strftime("%y%m%d_%H%M%S")}'
        log_dir = exp_dir / 'logs'
        log_dir.mkdir(parents=True, exist_ok=True)
        writer = SummaryWriter(str(log_dir))
        if option_text is not None:
            with open(log_dir / 'options.log', 'w') as f:
                f.write(option_text)
            writer.add_text('options', option_text)

        print(Back.RED + Fore.YELLOW + Style.BRIGHT + exp_dir.name + Style.RESET_ALL)
        print('model parameters: ', utils.count_parameters(model))

        # Train the model
        optimizer = torch.optim.Adam(filter(lambda p: p.requires_grad, model.parameters()), lr=args.lr)
        train(args, model, train_loader, val_loader, optimizer, exp_dir / 'checkpoints', writer)
    else:
        # Build model
        checkpoint_path, _ = utils.get_model_path(args.exp_name, args.iter, 'SE')
        model_data = torch.load(checkpoint_path, map_location=device)
        model_args = model_data['args']
        model = StyleEncoder(model_args).to(device)
        model.encoder.load_state_dict(model_data['encoder'], strict=False)
        model.eval()

        # Dataset
        test_dataset = LmdbDatasetForSE(data_root, args.data_root / 'test.txt', coef_stats_file, args.fps,
                                        args.n_motions,
                                        rot_repr=args.rot_repr, no_head_pose=args.no_head_pose)
        test_loader = data.DataLoader(test_dataset, batch_size=args.batch_size, shuffle=True,
                                      num_workers=args.num_workers)

        # Test the model
        test(args, model, test_loader, args.iter, 100, 'test')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    parser.add_argument('--mode', type=str, default='train', choices=['train', 'test'])
    parser.add_argument('--iter', type=int, default=100000, help='iteration to test')

    # Model
    options.add_model_options(parser)

    # Dataset
    options.add_data_options(parser)

    # Training
    options.add_training_options(parser)

    args = parser.parse_args()
    if args.mode == 'train':
        option_text = utils.get_option_text(args, parser)
    else:
        option_text = None

    main(args, option_text)


```

## Source file: main_dpt.py

{
  "path": "main_dpt.py",
  "blob_sha": "d57bf72433fb3ff327eae5663a7b0fdd1758ac6a",
  "sha256": "f83814bab65cba5bbadb21b4877990e32303acbd65c29c7a109104a0ac12d944",
  "lines": 535,
  "bytes": 27677,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3,
    "flame": 10,
    "rig": 1,
    "lip": 6
  },
  "url": "https://github.com/DiffPoseTalk/DiffPoseTalk/blob/2c761f174776ffe380cd89b875fc8eb49e90a5a1/main_dpt.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def train(args, model: DiffTalkingHead, style_enc: Optional[StyleEncoder], train_loader, val_loader, optimizer,"
    },
    {
      "line": 226,
      "text": "def test(args, model: DiffTalkingHead, style_enc: Optional[StyleEncoder], test_loader, current_iter, n_rounds=10,"
    },
    {
      "line": 406,
      "text": "def count_parameters(model):"
    },
    {
      "line": 410,
      "text": "def main(args, option_text=None):"
    }
  ]
}

```text

import argparse
from collections import deque, defaultdict
from datetime import datetime
from pathlib import Path
from typing import Optional

import numpy as np
import torch
import torch.optim as optim
from colorama import Fore, Back, Style
from tensorboardX import SummaryWriter
from torch.utils import data
from tqdm import tqdm

import options.dpt as options
import utils
from data import LmdbDataset, infinite_data_loader
from models import DiffTalkingHead, StyleEncoder
from models.flame import FLAME, FLAMEConfig


def train(args, model: DiffTalkingHead, style_enc: Optional[StyleEncoder], train_loader, val_loader, optimizer,
          save_dir, scheduler=None, writer=None, flame=None):
    device = model.device
    save_dir.mkdir(parents=True, exist_ok=True)

    model.train()
    data_loader = infinite_data_loader(train_loader)

    coef_stats = train_loader.dataset.coef_stats
    if coef_stats is not None:
        coef_stats = {x: coef_stats[x].to(device) for x in coef_stats}
    audio_unit = train_loader.dataset.audio_unit
    predict_head_pose = not args.no_head_pose

    loss_log = defaultdict(lambda: deque(maxlen=args.log_smooth_win))
    pbar = tqdm(range(args.max_iter + 1), dynamic_ncols=True)
    optimizer.zero_grad()
    for it in pbar:
        # Load data
        audio_pair, coef_pair, audio_stats = next(data_loader)
        audio_pair = [audio.to(device) for audio in audio_pair]
        coef_pair = [{x: coef_pair[i][x].to(device) for x in coef_pair[i]} for i in range(2)]
        motion_coef_pair = [
            utils.get_motion_coef(coef_pair[i], args.rot_repr, predict_head_pose) for i in range(2)
        ]  # (N, L, 50+x)

        # Use the shape coefficients from the first frame of the first clip as the condition
        if coef_pair[0]['shape'].ndim == 2:  # (N, 100)
            shape_coef = coef_pair[0]['shape'].clone().to(device)
        else:  # (N, L, 100)
            shape_coef = coef_pair[0]['shape'][:, 0].clone().to(device)

        # Extract style features
        if style_enc is not None:
            with torch.no_grad():
                style_pair = [style_enc(motion_coef_pair[i]) for i in range(2)]

        if args.use_context_audio_feat:
            # Extract audio features
            audio_feat = model.extract_audio_feature(torch.cat(audio_pair, dim=1), args.n_motions * 2)  # (N, 2L, :)

        loss_noise = 0
        loss_vert = 0
        loss_vel = torch.tensor(0, device=device)
        loss_smooth = torch.tensor(0, device=device)
        loss_head_angle = 0
        loss_head_vel = torch.tensor(0, device=device)
        loss_head_smooth = torch.tensor(0, device=device)
        loss_head_trans = 0
        for i in range(2):
            audio = audio_pair[i]  # (N, L_a)
            motion_coef = motion_coef_pair[i]  # (N, L, 50+x)
            style = style_pair[1 - i] if style_enc is not None else None
            batch_size = audio.shape[0]

            # truncate input audio and motion according to trunc_prob
            if (i == 0 and np.random.rand() < args.trunc_prob1) or (i != 0 and np.random.rand() < args.trunc_prob2):
                audio_in, motion_coef_in, end_idx = utils.truncate_motion_coef_and_audio(
                    audio, motion_coef, args.n_motions, audio_unit, args.pad_mode)
                if args.use_context_audio_feat and i != 0:
                    # use contextualized audio feature for the second clip
                    audio_in = model.extract_audio_feature(torch.cat([audio_pair[i - 1], audio_in], dim=1),
                                                           args.n_motions * 2)[:, -args.n_motions:]
            else:
                if args.use_context_audio_feat:
                    audio_in = audio_feat[:, i * args.n_motions:(i + 1) * args.n_motions]
                else:
                    audio_in = audio
                motion_coef_in, end_idx = motion_coef, None

            if args.use_indicator:
                if end_idx is not None:
                    indicator = torch.arange(args.n_motions, device=device).expand(batch_size, -1) < end_idx.unsqueeze(
                        1)
                else:
                    indicator = torch.ones(batch_size, args.n_motions, device=device)
            else:
                indicator = None

            # Inference
            if i == 0:
                noise, target, prev_motion_coef, prev_audio_feat = model(
                    motion_coef_in, audio_in, shape_coef, style, indicator=indicator)
                if end_idx is not None:  # was truncated, needs to use the complete feature
                    prev_motion_coef = motion_coef[:, -args.n_prev_motions:]
                    if args.use_context_audio_feat:
                        prev_audio_feat = audio_feat[:, args.n_motions - args.n_prev_motions:args.n_motions].detach()
                    else:
                        with torch.no_grad():
                            prev_audio_feat = model.extract_audio_feature(audio)[:, -args.n_prev_motions:]
                else:
                    prev_motion_coef = prev_motion_coef[:, -args.n_prev_motions:]
                    prev_audio_feat = prev_audio_feat[:, -args.n_prev_motions:]
            else:
                noise, target, _, _ = model(motion_coef_in, audio_in, shape_coef, style,
                                            prev_motion_coef, prev_audio_feat, indicator=indicator)

            loss_n, loss_v, loss_c, loss_s, loss_ha, loss_hc, loss_hs, loss_ht = utils.compute_loss(
                args, i == 0, shape_coef, motion_coef_in, noise, target, prev_motion_coef, coef_stats, flame, end_idx)
            loss_noise = loss_noise + loss_n / 2
            if args.target == 'sample' and args.l_vert > 0:
                loss_vert = loss_vert + loss_v / 2
            if args.target == 'sample' and args.l_vel > 0 and loss_c is not None:
                loss_vel = loss_vel + loss_c / 2
            if args.target == 'sample' and args.l_smooth > 0 and loss_s is not None:
                loss_smooth = loss_smooth + loss_s / 2
            if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
                loss_head_angle = loss_head_angle + loss_ha / 2
            if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0 and loss_hc is not None:
                loss_head_vel = loss_head_vel + loss_hc / 2
            if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0 and loss_hs is not None:
                loss_head_smooth = loss_head_smooth + loss_hs / 2
            if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0 and loss_ht is not None:
                # no need to divide by 2 because it only applies to the second clip
                loss_head_trans = loss_head_trans + loss_ht

        loss_log['noise'].append(loss_noise.item())
        loss = loss_noise
        if args.target == 'sample' and args.l_vert > 0:
            loss_log['vert'].append(loss_vert.item())
            loss = loss + args.l_vert * loss_vert
        if args.target == 'sample' and args.l_vel > 0:
            loss_log['vel'].append(loss_vel.item())
            loss = loss + args.l_vel * loss_vel
        if args.target == 'sample' and args.l_smooth > 0:
            loss_log['smooth'].append(loss_smooth.item())
            loss = loss + args.l_smooth * loss_smooth
        if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
            loss_log['head_angle'].append(loss_head_angle.item())
            loss = loss + args.l_head_angle * loss_head_angle
        if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
            loss_log['head_vel'].append(loss_head_vel.item())
            loss = loss + args.l_head_vel * loss_head_vel
        if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
            loss_log['head_smooth'].append(loss_head_smooth.item())
            loss = loss + args.l_head_smooth * loss_head_smooth
        if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
            loss_log['head_trans'].append(loss_head_trans.item())
            loss = loss + args.l_head_trans * loss_head_trans
        loss.backward()

        if it % args.gradient_accumulation_steps == 0:
            optimizer.step()
            optimizer.zero_grad()

        # Logging
        loss_log['loss'].append(loss.item())
        description = f'Train loss: [N: {np.mean(loss_log["noise"]):.3e}'
        if args.target == 'sample' and args.l_vert > 0:
            description += f', V: {np.mean(loss_log["vert"]):.3e}'
        if args.target == 'sample' and args.l_vel > 0:
            description += f', C: {np.mean(loss_log["vel"]):.3e}'
        if args.target == 'sample' and args.l_smooth > 0:
            description += f', S: {np.mean(loss_log["smooth"]):.3e}'
        if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
            description += f', HA: {np.mean(loss_log["head_angle"]):.3e}'
        if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
            description += f', HC: {np.mean(loss_log["head_vel"]):.3e}'
        if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
            description += f', HS: {np.mean(loss_log["head_smooth"]):.3e}'
        if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
            description += f', HT: {np.mean(loss_log["head_trans"]):.3e}'
        description += ']'
        pbar.set_description(description)

        if it % args.log_iter == 0 and writer is not None:
            # write to tensorboard
            writer.add_scalar('train/loss', np.mean(loss_log['loss']), it)
            writer.add_scalar('train/noise', np.mean(loss_log['noise']), it)
            if args.target == 'sample' and args.l_vert > 0:
                writer.add_scalar('train/vert', np.mean(loss_log['vert']), it)
            if args.target == 'sample' and args.l_vel > 0:
                writer.add_scalar('train/vel', np.mean(loss_log['vel']), it)
            if args.target == 'sample' and args.l_smooth > 0:
                writer.add_scalar('train/smooth', np.mean(loss_log['smooth']), it)
            if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
                writer.add_scalar('train/head_angle', np.mean(loss_log['head_angle']), it)
            if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
                writer.add_scalar('train/head_vel', np.mean(loss_log['head_vel']), it)
            if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
                writer.add_scalar('train/head_smooth', np.mean(loss_log['head_smooth']), it)
            if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
                writer.add_scalar('train/head_trans', np.mean(loss_log['head_trans']), it)
            writer.add_scalar('opt/lr', optimizer.param_groups[0]['lr'], it)

        # update learning rate
        if scheduler is not None:
            if args.scheduler != 'WarmupThenDecay' or (args.scheduler == 'WarmupThenDecay' and it < args.cos_max_iter):
                scheduler.step()

        # save model
        if (it % args.save_iter == 0 and it != 0) or it == args.max_iter:
            torch.save({
                'args': args,
                'model': model.state_dict(),
                'iter': it,
            }, save_dir / f'iter_{it:07}.pt')

        # validation
        if (it % args.val_iter == 0 and it != 0) or it == args.max_iter:
            test(args, model, style_enc, val_loader, it, 200, 'val', writer, flame)


@torch.no_grad()
def test(args, model: DiffTalkingHead, style_enc: Optional[StyleEncoder], test_loader, current_iter, n_rounds=10,
         mode='val', writer=None, flame=None):
    is_training = model.training
    device = model.device
    model.eval()

    coef_stats = test_loader.dataset.coef_stats
    if coef_stats is not None:
        coef_stats = {x: coef_stats[x].to(device) for x in coef_stats}
    audio_unit = test_loader.dataset.audio_unit
    predict_head_pose = not args.no_head_pose

    loss_log = defaultdict(list)
    for test_round in range(n_rounds):
        for audio_pair, coef_pair, audio_stats in test_loader:
            audio_pair = [audio.to(device) for audio in audio_pair]
            coef_pair = [{x: coef_pair[i][x].to(device) for x in coef_pair[i]} for i in range(2)]
            motion_coef_pair = [
                utils.get_motion_coef(coef_pair[i], args.rot_repr, predict_head_pose) for i in range(2)
            ]  # (N, L, 50+x)

            # Use the shape coefficients from the first frame of the first clip as the condition
            if coef_pair[0]['shape'].ndim == 2:  # (N, 100)
                shape_coef = coef_pair[0]['shape'].clone().to(device)
            else:  # (N, L, 100)
                shape_coef = coef_pair[0]['shape'][:, 0].clone().to(device)

            # Extract style features
            if style_enc is not None:
                with torch.no_grad():
                    style_pair = [style_enc(motion_coef_pair[i]) for i in range(2)]

            if args.use_context_audio_feat:
                # Extract audio features
                audio_feat = model.extract_audio_feature(torch.cat(audio_pair, dim=1), args.n_motions * 2)  # (N, 2L, :)

            loss_noise = 0
            loss_vert = 0
            loss_vel = torch.tensor(0, device=device)
            loss_smooth = torch.tensor(0, device=device)
            loss_head_angle = 0
            loss_head_vel = torch.tensor(0, device=device)
            loss_head_smooth = torch.tensor(0, device=device)
            loss_head_trans = 0
            for i in range(2):
                audio = audio_pair[i]  # (N, L_a)
                motion_coef = motion_coef_pair[i]  # (N, L, 50+x)
                style = style_pair[1 - i] if style_enc is not None else None
                batch_size = audio.shape[0]

                # truncate input audio and motion according to trunc_prob
                if (i == 0 and np.random.rand() < args.trunc_prob1) or (i != 0 and np.random.rand() < args.trunc_prob2):
                    audio_in, motion_coef_in, end_idx = utils.truncate_motion_coef_and_audio(
                        audio, motion_coef, args.n_motions, audio_unit, args.pad_mode)
                    if args.use_context_audio_feat and i != 0:
                        # use contextualized audio feature for the second clip
                        audio_in = model.extract_audio_feature(torch.cat([audio_pair[i - 1], audio_in], dim=1),
                                                               args.n_motions * 2)[:, -args.n_motions:]

                else:
                    if args.use_context_audio_feat:
                        audio_in = audio_feat[:, i * args.n_motions:(i + 1) * args.n_motions]
                    else:
                        audio_in = audio
                    motion_coef_in, end_idx = motion_coef, None

                if args.use_indicator:
                    if end_idx is not None:
                        indicator = torch.arange(args.n_motions, device=device).expand(batch_size,
                                                                                       -1) < end_idx.unsqueeze(1)
                    else:
                        indicator = torch.ones(batch_size, args.n_motions, device=device)
                else:
                    indicator = None

                # Inference
                if i == 0:
                    noise, target, prev_motion_coef, prev_audio_feat = model(
                        motion_coef_in, audio_in, shape_coef, style, indicator=indicator)
                    if end_idx is not None:  # was truncated, needs to use the complete feature
                        prev_motion_coef = motion_coef[:, -args.n_prev_motions:]
                        if args.use_context_audio_feat:
                            prev_audio_feat = audio_feat[:, args.n_motions - args.n_prev_motions:args.n_motions]
                        else:
                            with torch.no_grad():
                                prev_audio_feat = model.extract_audio_feature(audio)[:, -args.n_prev_motions:]
                    else:
                        prev_motion_coef = prev_motion_coef[:, -args.n_prev_motions:]
                        prev_audio_feat = prev_audio_feat[:, -args.n_prev_motions:]
                else:
                    noise, target, _, _ = model(motion_coef_in, audio, shape_coef, style,
                                                prev_motion_coef, prev_audio_feat, indicator=indicator)

                loss_n, loss_v, loss_c, loss_s, loss_ha, loss_hc, loss_hs, loss_ht = utils.compute_loss(
                    args, i == 0, shape_coef, motion_coef_in, noise, target, prev_motion_coef, coef_stats, flame,
                    end_idx
                )
                loss_noise = loss_noise + loss_n / 2
                if args.target == 'sample' and args.l_vert > 0:
                    loss_vert = loss_vert + loss_v / 2
                if args.target == 'sample' and args.l_vel > 0 and loss_c is not None:
                    loss_vel = loss_vel + loss_c / 2
                if args.target == 'sample' and args.l_smooth > 0 and loss_s is not None:
                    loss_smooth = loss_smooth + loss_s / 2
                if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
                    loss_head_angle = loss_head_angle + loss_ha / 2
                if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0 and loss_hc is not None:
                    loss_head_vel = loss_head_vel + loss_hc / 2
                if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0 and loss_hs is not None:
                    loss_head_smooth = loss_head_smooth + loss_hs / 2
                if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0 and loss_ht is not None:
                    # no need to divide by 2 because it only applies to the second clip
                    loss_head_trans = loss_head_trans + loss_ht

            loss_log['noise'].append(loss_noise.item())
            loss = loss_noise
            if args.target == 'sample' and args.l_vert > 0:
                loss_log['vert'].append(loss_vert.item())
                loss = loss + args.l_vert * loss_vert
            if args.target == 'sample' and args.l_vel > 0:
                loss_log['vel'].append(loss_vel.item())
                loss = loss + args.l_vel * loss_vel
            if args.target == 'sample' and args.l_smooth > 0:
                loss_log['smooth'].append(loss_smooth.item())
                loss = loss + args.l_smooth * loss_smooth
            if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
                loss_log['head_angle'].append(loss_head_angle.item())
                loss = loss + args.l_head_angle * loss_head_angle
            if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
                loss_log['head_vel'].append(loss_head_vel.item())
                loss = loss + args.l_head_vel * loss_head_vel
            if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
                loss_log['head_smooth'].append(loss_head_smooth.item())
                loss = loss + args.l_head_smooth * loss_head_smooth
            if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
                loss_log['head_trans'].append(loss_head_trans.item())
                loss = loss + args.l_head_trans * loss_head_trans
            loss_log['loss'].append(loss.item())

    description = f'(Iter {current_iter:>6}) {mode} loss: [N: {np.mean(loss_log["noise"]):.3e}'
    if args.target == 'sample' and args.l_vert > 0:
        description += f', V: {np.mean(loss_log["vert"]):.3e}'
    if args.target == 'sample' and args.l_vel > 0:
        description += f', C: {np.mean(loss_log["vel"]):.3e}'
    if args.target == 'sample' and args.l_smooth > 0:
        description += f', S: {np.mean(loss_log["smooth"]):.3e}'
    if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
        description += f', HA: {np.mean(loss_log["head_angle"]):.3e}'
    if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
        description += f', HC: {np.mean(loss_log["head_vel"]):.3e}'
    if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
        description += f', HS: {np.mean(loss_log["head_smooth"]):.3e}'
    if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
        description += f', HT: {np.mean(loss_log["head_trans"]):.3e}'
    description += ']'
    print(description)

    if writer is not None:
        # write to tensorboard
        writer.add_scalar(f'{mode}/loss', np.mean(loss_log['loss']), current_iter)
        writer.add_scalar(f'{mode}/noise', np.mean(loss_log['noise']), current_iter)
        if args.target == 'sample' and args.l_vert > 0:
            writer.add_scalar(f'{mode}/vert', np.mean(loss_log['vert']), current_iter)
        if args.target == 'sample' and args.l_vel > 0:
            writer.add_scalar(f'{mode}/vel', np.mean(loss_log['vel']), current_iter)
        if args.target == 'sample' and args.l_smooth > 0:
            writer.add_scalar(f'{mode}/smooth', np.mean(loss_log['smooth']), current_iter)
        if args.target == 'sample' and predict_head_pose and args.l_head_angle > 0:
            writer.add_scalar(f'{mode}/head_angle', np.mean(loss_log['head_angle']), current_iter)
        if args.target == 'sample' and predict_head_pose and args.l_head_vel > 0:
            writer.add_scalar(f'{mode}/head_vel', np.mean(loss_log['head_vel']), current_iter)
        if args.target == 'sample' and predict_head_pose and args.l_head_smooth > 0:
            writer.add_scalar(f'{mode}/head_smooth', np.mean(loss_log['head_smooth']), current_iter)
        if args.target == 'sample' and predict_head_pose and args.l_head_trans > 0:
            writer.add_scalar(f'{mode}/head_trans', np.mean(loss_log['head_trans']), current_iter)

    if is_training:
        model.train()


def count_parameters(model):
    return sum(p.numel() for p in model.parameters() if p.requires_grad)


def main(args, option_text=None):
    device = 'cuda' if torch.cuda.is_available() else 'cpu'

    # Load data
    data_root = args.data_root
    coef_stats_file: Path = args.stats_file
    if not coef_stats_file.is_absolute():
        coef_stats_file = data_root / coef_stats_file

    # Loss
    if args.l_vert > 0 or args.l_vel > 0:
        flame = FLAME(FLAMEConfig).to(device)
    else:
        flame = None

    if args.mode == 'train':
        # Style Encoder
        if args.style_enc_ckpt:
            # Build model
            enc_model_data = torch.load(args.style_enc_ckpt, map_location=device)
            enc_model_args = utils.NullableArgs(enc_model_data['args'])
            style_enc = StyleEncoder(enc_model_args).to(device)
            style_enc.encoder.load_state_dict(enc_model_data['encoder'], strict=False)
            style_enc.eval()
        else:
            style_enc = None

        # Build model
        model = DiffTalkingHead(args, device=device)

        # Dataset
        train_dataset = LmdbDataset(data_root, data_root / 'train.txt', coef_stats_file, args.fps, args.n_motions,
                                    rot_repr=args.rot_repr)
        val_dataset = LmdbDataset(data_root, data_root / 'val.txt', coef_stats_file, args.fps, args.n_motions,
                                  rot_repr=args.rot_repr)
        train_loader = data.DataLoader(train_dataset, batch_size=args.batch_size, shuffle=True,
                                       num_workers=args.num_workers, pin_memory=True)
        val_loader = data.DataLoader(val_dataset, batch_size=args.batch_size, shuffle=False,
                                     num_workers=args.num_workers)

        # Logging
        exp_dir = Path('experiments/DPT') / f'{args.exp_name}-{datetime.now().strftime("%y%m%d_%H%M%S")}'
        log_dir = exp_dir / 'logs'
        log_dir.mkdir(parents=True, exist_ok=True)
        writer = SummaryWriter(str(log_dir))
        if option_text is not None:
            with open(log_dir / 'options.log', 'w') as f:
                f.write(option_text)
            writer.add_text('options', option_text)

        print(Back.RED + Fore.YELLOW + Style.BRIGHT + exp_dir.name + Style.RESET_ALL)
        print('model parameters: ', count_parameters(model))

        # Train the model
        optimizer = torch.optim.Adam(filter(lambda p: p.requires_grad, model.parameters()), lr=args.lr)
        # scheduler
        if args.scheduler == 'Warmup':
            from scheduler import GradualWarmupScheduler
            scheduler = GradualWarmupScheduler(optimizer, 1, args.warm_iter)
        elif args.scheduler == 'WarmupThenDecay':
            from scheduler import GradualWarmupScheduler
            after_scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, args.cos_max_iter - args.warm_iter,
                                                                   args.lr * args.min_lr_ratio)
            scheduler = GradualWarmupScheduler(optimizer, 1, args.warm_iter, after_scheduler)
        else:
            scheduler = None

        train(args, model, style_enc, train_loader, val_loader, optimizer, exp_dir / 'checkpoints', scheduler, writer,
              flame)
    else:
        # Load model
        checkpoint_path, exp_name = utils.get_model_path(args.exp_name, args.iter)
        model_data = torch.load(checkpoint_path, map_location=device)
        model_args = utils.NullableArgs(model_data['args'])

        # Style Encoder
        if model_args.style_enc_ckpt:
            # Build model
            enc_model_data = torch.load(model_args.style_enc_ckpt, map_location=device)
            enc_model_args = utils.NullableArgs(enc_model_data['args'])
            style_enc = StyleEncoder(enc_model_args).to(device)
            style_enc.encoder.load_state_dict(enc_model_data['encoder'], strict=False)
            style_enc.eval()
        else:
            style_enc = None

        # Build model
        model = DiffTalkingHead(model_args, device=device)
        model.load_state_dict(model_data['model'])
        model.eval()

        # Dataset
        test_dataset = LmdbDataset(data_root, data_root / 'test.txt', coef_stats_file, args.fps, args.n_motions,
                                   rot_repr=args.rot_repr)
        test_loader = data.DataLoader(test_dataset, batch_size=args.batch_size, shuffle=False,
                                      num_workers=args.num_workers)

        # Test the model
        test(model_args, model, style_enc, test_loader, args.iter, 200, 'test', None, flame)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='DiffTalkingHead: Speech-Driven 3D Facial Animation')

    parser.add_argument('--mode', type=str, default='train', choices=['train', 'test'])
    parser.add_argument('--iter', type=int, default=100000, help='iteration to test')

    # Dataset
    options.add_data_options(parser)

    # Model
    options.add_model_options(parser)

    # Training
    options.add_training_options(parser)

    # Additional options depending on previous options
    options.add_additional_options(parser)

    args = parser.parse_args()
    if args.mode == 'train':
        option_text = utils.get_option_text(args, parser)
    else:
        option_text = None

    main(args, option_text)


```

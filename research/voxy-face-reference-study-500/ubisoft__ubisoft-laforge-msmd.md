# Facial rendering source scan: ubisoft/ubisoft-laforge-msmd

Repository: https://github.com/ubisoft/ubisoft-laforge-msmd

Pinned source revision: 41adbc570c237b4e3f11338acdf335c94dc27406

Discovery topic: facial animation

Description: Model See Model Do: Speech-Driven Facial Animation with Style Control

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["license.txt"]

Topic paths: {"flame": ["utils/flame.py"], "lbs": ["utils/lbs.py"], "face": ["dataset_processing/models/mediapipe_geometries/mediapipe_canonical_face_mesh.obj"]}

## Source file: utils/lbs.py

{
  "path": "utils/lbs.py",
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
  "url": "https://github.com/ubisoft/ubisoft-laforge-msmd/blob/41adbc570c237b4e3f11338acdf335c94dc27406/utils/lbs.py",
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

## Source file: utils/flame.py

{
  "path": "utils/flame.py",
  "blob_sha": "715c89fc33656f1f831c90f72b1476383cdd5fd5",
  "sha256": "b9d31961200eb309da321794ce1bc60a9409c312cea9d8549667b2c7f4de4492",
  "lines": 301,
  "bytes": 14061,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 18,
    "jaw": 1,
    "flame": 31,
    "lbs": 4,
    "rig": 5,
    "face": 20
  },
  "url": "https://github.com/ubisoft/ubisoft-laforge-msmd/blob/41adbc570c237b4e3f11338acdf335c94dc27406/utils/flame.py",
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
      "line": 66,
      "text": "def __init__(self, config):"
    },
    {
      "line": 126,
      "text": "def _find_dynamic_lmk_idx_and_bcoords(self, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 174,
      "text": "def seletec_3d68(self, vertices):"
    },
    {
      "line": 180,
      "text": "def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, pose2rot=True,"
    },
    {
      "line": 255,
      "text": "def __init__(self, config):"
    },
    {
      "line": 285,
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
    # flame_model_path=str(__dir__ / 'data/FLAME2020/generic_model.pkl'),
    flame_model_path=str("/code/models/flame_data/FLAME2020/generic_model.pkl"),
    n_shape=100,
    n_exp=50,
    n_tex=50,
    tex_type='BFM',
    tex_path=str("/code/models/flame_data/FLAME2020/FLAME_albedo_from_BFM.npz"),
    # tex_path=str(__dir__ / 'data/FLAME2020/FLAME_albedo_from_BFM.npz'),
    flame_lmk_embedding_path='/code/models/flame_data/landmark_embedding.npy'
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

## Source file: model.py

{
  "path": "model.py",
  "blob_sha": "8ce5bdf6ef88655d5788e5fb4d86425dbf108abc",
  "sha256": "b9e5b2882a101fd0d39714410b9052ea1531a94328c695b39caaf0f1229aed8b",
  "lines": 997,
  "bytes": 49968,
  "selection_score": 0,
  "keyword_line_counts": {
    "flame": 2,
    "face": 10,
    "lip": 1
  },
  "url": "https://github.com/ubisoft/ubisoft-laforge-msmd/blob/41adbc570c237b4e3f11338acdf335c94dc27406/model.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def get_diffusion_model(args, device='cuda'):"
    },
    {
      "line": 21,
      "text": "def __init__(self, num_steps, mode='linear', beta_1=1e-4, beta_T=0.02, s=0.008):"
    },
    {
      "line": 63,
      "text": "def uniform_sample_t(self, batch_size):"
    },
    {
      "line": 68,
      "text": "def get_sigmas(self, t, flexibility=0):"
    },
    {
      "line": 74,
      "text": "def __init__(self, args, device='cuda', vae_style=False, conditioned=True, denoisingnset_version=1, use_head_alpha=True, regularize_alpha=\"None\"):"
    },
    {
      "line": 143,
      "text": "def device(self):"
    },
    {
      "line": 146,
      "text": "def forward(self, motion_feat, audio_or_feat, shape_feat, style_feat=None,"
    },
    {
      "line": 250,
      "text": "def extract_audio_feature(self, audio, frame_num=None):"
    },
    {
      "line": 267,
      "text": "def extract_audio_768_feature(self, audio, frame_num=None):"
    },
    {
      "line": 283,
      "text": "def sample(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,"
    },
    {
      "line": 443,
      "text": "def sample_separate(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,"
    },
    {
      "line": 654,
      "text": "def sample_with_guide(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,"
    },
    {
      "line": 821,
      "text": "def __init__(self, args, device='cuda', motion_feat_dim=50, use_head_alpha=True, regularize_alpha=\"None\"):"
    },
    {
      "line": 911,
      "text": "def device(self):"
    },
    {
      "line": 914,
      "text": "def forward(self, motion_feat, audio_feat, person_feat, static_style_feat, prev_motion_feat, prev_audio_feat, step, indicator=None, keep_separate=False):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F
import copy
from utils.model_common import PositionalEncoding, enc_dec_mask, pad_audio

def get_diffusion_model(args, device='cuda'):
    try:
        enc_ckpt = args.style_enc_ckpt
    except:
        args.style_enc_ckpt = None
    try:
        regularizer = args.regularize_alpha
    except:
        regularizer = "None"

    return MSMD(args, device, True, use_head_alpha=False, regularize_alpha=regularizer)
   
    
class DiffusionSchedule(nn.Module):
    def __init__(self, num_steps, mode='linear', beta_1=1e-4, beta_T=0.02, s=0.008):
        # beta is generally the noise level
        # alpha is the signal level
        # here we have increasing noise level
        super().__init__()
        if mode == 'linear':
            betas = torch.linspace(beta_1, beta_T, num_steps)
        elif mode == 'quadratic':
            betas = torch.linspace(beta_1 ** 0.5, beta_T ** 0.5, num_steps) ** 2
        elif mode == 'sigmoid':
            betas = torch.sigmoid(torch.linspace(-5, 5, num_steps)) * (beta_T - beta_1) + beta_1
        elif mode == 'cosine':
            steps = num_steps + 1
            x = torch.linspace(0, num_steps, steps)
            alpha_bars = torch.cos(((x / num_steps) + s) / (1 + s) * torch.pi * 0.5) ** 2
            alpha_bars = alpha_bars / alpha_bars[0]
            betas = 1 - (alpha_bars[1:] / alpha_bars[:-1])
            betas = torch.clip(betas, 0.0001, 0.999)
        else:
            raise ValueError(f'Unknown diffusion schedule {mode}!')
        betas = torch.cat([torch.zeros(1), betas], dim=0)  # Padding beta_0 = 0

        alphas = 1 - betas
        log_alphas = torch.log(alphas)
        for i in range(1, log_alphas.shape[0]):  # 1 to T
            log_alphas[i] += log_alphas[i - 1]
        alpha_bars = log_alphas.exp()

        sigmas_flex = torch.sqrt(betas)
        sigmas_inflex = torch.zeros_like(sigmas_flex)
        for i in range(1, sigmas_flex.shape[0]):
            sigmas_inflex[i] = ((1 - alpha_bars[i - 1]) / (1 - alpha_bars[i])) * betas[i]
        sigmas_inflex = torch.sqrt(sigmas_inflex)

        self.num_steps = num_steps
        self.register_buffer('betas', betas)
        self.register_buffer('alphas', alphas)
        self.register_buffer('alpha_bars', alpha_bars)
        self.register_buffer('sigmas_flex', sigmas_flex)
        self.register_buffer('sigmas_inflex', sigmas_inflex)

    # randomly sample a time step from 1 to T (i.e. 1 to num_steps)
    def uniform_sample_t(self, batch_size):
        ts = torch.randint(1, self.num_steps + 1, (batch_size,))
        return ts.tolist()
    
    # get the noise level at time step t
    def get_sigmas(self, t, flexibility=0):
        assert 0 <= flexibility <= 1
        sigmas = self.sigmas_flex[t] * flexibility + self.sigmas_inflex[t] * (1 - flexibility)
        return sigmas

class MSMD(nn.Module):
    def __init__(self, args, device='cuda', vae_style=False, conditioned=True, denoisingnset_version=1, use_head_alpha=True, regularize_alpha="None"):
        super().__init__()

        # Model parameters
        self.target = args.target
        self.regularize_alpha = regularize_alpha
        self.architecture = args.architecture
        self.use_style = (args.style_enc_ckpt is not None) or vae_style
        self.conditioned = conditioned
        self.motion_feat_dim = 67
        self.denoisingnset_version = denoisingnset_version
        self.use_head_alpha = use_head_alpha
        self.fps = args.fps
        self.n_motions = args.n_motions
        self.n_prev_motions = args.n_prev_motions
        if self.use_style:
            self.style_feat_dim = args.d_style
        # Audio encoder
        self.audio_model = args.audio_model
        if self.audio_model == 'wav2vec2':
            from utils.wav2vec2 import Wav2Vec2Model
            self.audio_encoder = Wav2Vec2Model.from_pretrained('facebook/wav2vec2-base-960h', cache_dir="/code/models/Huggingface/hub2")
            # wav2vec 2.0 weights initialization
            self.audio_encoder.feature_extractor._freeze_parameters()
        elif self.audio_model == 'hubert':
            from utils.hubert import HubertModel
            self.audio_encoder = HubertModel.from_pretrained('facebook/hubert-base-ls960', cache_dir="/code/models/Huggingface/hub2")
            self.audio_encoder.feature_extractor._freeze_parameters()

            frozen_layers = [0, 1]
            for name, param in self.audio_encoder.named_parameters():
                if name.startswith("feature_projection"):
                    param.requires_grad = False
                if name.startswith("encoder.layers"):
                    layer = int(name.split(".")[2])
                    if layer in frozen_layers:
                        param.requires_grad = False
        else:
            raise ValueError(f'Unknown audio model {self.audio_model}!')

        if args.architecture == 'decoder':
            self.audio_feature_map = nn.Linear(768, args.feature_dim)
            self.start_audio_feat = nn.Parameter(torch.randn(1, self.n_prev_motions, args.feature_dim))
        else:
            raise ValueError(f'Unknown architecture {args.architecture}!')

        self.start_motion_feat = nn.Parameter(torch.randn(1, self.n_prev_motions, self.motion_feat_dim))

        # Diffusion model
        self.denoising_net = DenoisingNetwork_MSMD(args, device, motion_feat_dim=self.motion_feat_dim, use_head_alpha=self.use_head_alpha, regularize_alpha=self.regularize_alpha)
        # diffusion schedule
        self.diffusion_sched = DiffusionSchedule(args.n_diff_steps, args.diff_schedule)

        # Classifier-free settings
        self.cfg_mode = args.cfg_mode
        guiding_conditions = args.guiding_conditions.split(',') if args.guiding_conditions else []
        self.guiding_conditions = [cond for cond in guiding_conditions if cond in ['style', 'audio']]
        if 'style' in self.guiding_conditions:
            if not self.use_style:
                raise ValueError('Cannot use style guiding without enabling it!')
            self.null_style_feat = nn.Parameter(torch.randn(1, 1, self.style_feat_dim))
        if 'audio' in self.guiding_conditions:
            audio_feat_dim = args.feature_dim
            self.null_audio_feat = nn.Parameter(torch.randn(1, 1, audio_feat_dim))
        

        self.to(device)

    @property
    def device(self):
        return next(self.parameters()).device

    def forward(self, motion_feat, audio_or_feat, shape_feat, style_feat=None,
                prev_motion_feat=None, prev_audio_feat=None, time_step=None, indicator=None, train_with_CFG=True, keep_separate=False):
        """
        Args:
            motion_feat: (N, L, d_coef) motion coefficients or features
            audio_or_feat: (N, L_audio) raw audio or audio feature
            shape_feat: (N, d_shape) or (N, 1, d_shape)
            style_feat: (N, d_style)
            prev_motion_feat: (N, n_prev_motions, d_motion) previous motion coefficients or feature
            prev_audio_feat: (N, n_prev_motions, d_audio) previous audio features
            time_step: (N,)
            indicator: (N, L) 0/1 indicator of real (unpadded) motion coefficients

        Returns:
           motion_feat_noise: (N, L, d_motion)
        """
        if self.use_style:
            assert style_feat is not None, 'Missing style features!'

        batch_size = motion_feat.shape[0]

        if audio_or_feat.ndim == 2:
            # Extract audio features
            assert audio_or_feat.shape[1] == 16000 * self.n_motions / self.fps, \
                f'Incorrect audio length {audio_or_feat.shape[1]}'
            audio_feat_saved = self.extract_audio_feature(audio_or_feat)  # (N, L, feature_dim)
        elif audio_or_feat.ndim == 3:
            assert audio_or_feat.shape[1] == self.n_motions, f'Incorrect audio feature length {audio_or_feat.shape[1]}'
            audio_feat_saved = audio_or_feat
        else:
            raise ValueError(f'Incorrect audio input shape {audio_or_feat.shape}')
        audio_feat = audio_feat_saved.clone()

        if shape_feat.ndim == 2:
            shape_feat = shape_feat.unsqueeze(1)  # (N, 1, d_shape)
        if style_feat is not None and style_feat.ndim == 2:
            style_feat = style_feat.unsqueeze(1)  # (N, 1, d_style)

        if prev_motion_feat is None:
            prev_motion_feat = self.start_motion_feat.expand(batch_size, -1, -1)  # (N, n_prev_motions, d_motion)
        if prev_audio_feat is None:
            # (N, n_prev_motions, feature_dim)
            prev_audio_feat = self.start_audio_feat.expand(batch_size, -1, -1)

        # Classifier-free guidance
        if len(self.guiding_conditions) > 0 and train_with_CFG:
            assert len(self.guiding_conditions) <= 2, 'Only support 1 or 2 CFG conditions!'
            if len(self.guiding_conditions) == 1 or self.cfg_mode == 'independent':
                null_cond_prob = 0.5 if len(self.guiding_conditions) >= 2 else 0.1
                if 'style' in self.guiding_conditions:
                    mask_style = torch.rand(batch_size, device=self.device) < null_cond_prob
                    style_feat = torch.where(mask_style.view(-1, 1, 1),
                                             self.null_style_feat.expand(batch_size, -1, -1),
                                             style_feat)
                if 'audio' in self.guiding_conditions:
                    mask_audio = torch.rand(batch_size, device=self.device) < null_cond_prob
                    audio_feat = torch.where(mask_audio.view(-1, 1, 1),
                                             self.null_audio_feat.expand(batch_size, self.n_motions, -1),
                                             audio_feat)
            else:
                # len(self.guiding_conditions) > 1 and self.cfg_mode == 'incremental'
                # full (0.45), w/o style (0.45), w/o style or audio (0.1)
                mask_flag = torch.rand(batch_size, device=self.device)
                if 'style' in self.guiding_conditions:
                    mask_style = mask_flag > 0.55
                    style_feat = torch.where(mask_style.view(-1, 1, 1),
                                             self.null_style_feat.expand(batch_size, -1, -1),
                                             style_feat)
                if 'audio' in self.guiding_conditions:
                    mask_audio = mask_flag > 0.9
                    audio_feat = torch.where(mask_audio.view(-1, 1, 1),
                                             self.null_audio_feat.expand(batch_size, self.n_motions, -1),
                                             audio_feat)

        if style_feat is None:
            # The model only accepts audio and shape features, i.e., self.use_style = False
            person_feat = shape_feat
        else:
            person_feat = torch.cat([shape_feat, style_feat], dim=-1)

        if time_step is None:
            # Sample time step
            time_step = self.diffusion_sched.uniform_sample_t(batch_size)  # (N,)

        # The forward diffusion process
        alpha_bar = self.diffusion_sched.alpha_bars[time_step]  # (N,)
        c0 = torch.sqrt(alpha_bar).view(-1, 1, 1)  # (N, 1, 1)
        c1 = torch.sqrt(1 - alpha_bar).view(-1, 1, 1)  # (N, 1, 1)

        eps = torch.randn_like(motion_feat)  # (N, L, d_motion)
        motion_feat_noisy = c0 * motion_feat + c1 * eps

        # The reverse diffusion process
        if keep_separate:
            dynamic_features, static_features, alpha_t = self.denoising_net(motion_feat_noisy, audio_feat, person_feat, style_feat,
                                                prev_motion_feat, prev_audio_feat, time_step, indicator, keep_separate=True)
            motion_feat_target = dynamic_features + alpha_t * static_features
            return eps, motion_feat_target, motion_feat.detach(), audio_feat_saved.detach(), dynamic_features, static_features, alpha_t
        else:
            motion_feat_target = self.denoising_net(motion_feat_noisy, audio_feat, person_feat, style_feat,
                                                prev_motion_feat, prev_audio_feat, time_step, indicator)
        
            return eps, motion_feat_target, motion_feat.detach(), audio_feat_saved.detach()

    def extract_audio_feature(self, audio, frame_num=None):
        frame_num = frame_num or self.n_motions

        # # Strategy 1: resample during audio feature extraction
        # hidden_states = self.audio_encoder(pad_audio(audio), self.fps, frame_num=frame_num).last_hidden_state  # (N, L, 768)

        # Strategy 2: resample after audio feature extraction (BackResample)
        hidden_states = self.audio_encoder(pad_audio(audio), self.fps,
                                           frame_num=frame_num * 2).last_hidden_state  # (N, 2L, 768)
        hidden_states = hidden_states.transpose(1, 2)  # (N, 768, 2L)
        hidden_states = F.interpolate(hidden_states, size=frame_num, align_corners=False, mode='linear')  # (N, 768, L)
        hidden_states = hidden_states.transpose(1, 2)  # (N, L, 768)

        audio_feat = self.audio_feature_map(hidden_states)  # (N, L, feature_dim)
        return audio_feat

    @torch.no_grad()
    def extract_audio_768_feature(self, audio, frame_num=None):
        frame_num = frame_num or self.n_motions

        # # Strategy 1: resample during audio feature extraction
        # hidden_states = self.audio_encoder(pad_audio(audio), self.fps, frame_num=frame_num).last_hidden_state  # (N, L, 768)

        # Strategy 2: resample after audio feature extraction (BackResample)
        hidden_states = self.audio_encoder(pad_audio(audio), self.fps,
                                           frame_num=frame_num * 2).last_hidden_state  # (N, 2L, 768)
        hidden_states = hidden_states.transpose(1, 2)  # (N, 768, 2L)
        hidden_states = F.interpolate(hidden_states, size=frame_num, align_corners=False, mode='linear')  # (N, 768, L)
        hidden_states = hidden_states.transpose(1, 2)  # (N, L, 768)

        return hidden_states
    
    @torch.no_grad()
    def sample(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,
               motion_at_T=None, indicator=None, cfg_mode=None, cfg_cond=None, cfg_scale=1.15, flexibility=0,
               dynamic_threshold=None, ret_traj=False):
        # Check and convert inputs
        batch_size = audio_or_feat.shape[0]

        # Check CFG conditions
        if cfg_mode is None:  # Use default CFG mode
            cfg_mode = self.cfg_mode #
        if cfg_cond is None:  # Use default CFG conditions
            cfg_cond = self.guiding_conditions
        cfg_cond = [c for c in cfg_cond if c in ['audio', 'style']]

        if not isinstance(cfg_scale, list):
            cfg_scale = [cfg_scale] * len(cfg_cond)

        # sort cfg_cond and cfg_scale
        if len(cfg_cond) > 0:
            cfg_cond, cfg_scale = zip(*sorted(zip(cfg_cond, cfg_scale), key=lambda x: ['audio', 'style'].index(x[0])))
        else:
            cfg_cond, cfg_scale = [], []

        if 'style' in cfg_cond:
            assert self.use_style and style_feat is not None

        if self.use_style:
            if style_feat is None:  # use null style feature
                style_feat = self.null_style_feat.expand(batch_size, -1, -1)
        else:
            assert style_feat is None, 'This model does not support style feature input!'

        if audio_or_feat.ndim == 2:
            # Extract audio features
            assert audio_or_feat.shape[1] == 16000 * self.n_motions / self.fps, \
                f'Incorrect audio length {audio_or_feat.shape[1]}'
            audio_feat = self.extract_audio_feature(audio_or_feat)  # (N, L, feature_dim)
        elif audio_or_feat.ndim == 3:
            assert audio_or_feat.shape[1] == self.n_motions, f'Incorrect audio feature length {audio_or_feat.shape[1]}'
            audio_feat = audio_or_feat
        else:
            raise ValueError(f'Incorrect audio input shape {audio_or_feat.shape}')

        if shape_feat.ndim == 2:
            shape_feat = shape_feat.unsqueeze(1)  # (N, 1, d_shape)
        if style_feat is not None and style_feat.ndim == 2:
            style_feat = style_feat.unsqueeze(1)  # (N, 1, d_style)

        if prev_motion_feat is None:
            prev_motion_feat = self.start_motion_feat.expand(batch_size, -1, -1)  # (N, n_prev_motions, d_motion)
        if prev_audio_feat is None:
            # (N, n_prev_motions, feature_dim)
            prev_audio_feat = self.start_audio_feat.expand(batch_size, -1, -1)

        if motion_at_T is None:
            motion_at_T = torch.randn((batch_size, self.n_motions, self.motion_feat_dim)).to(self.device)

        # Prepare input for the reverse diffusion process (including optional classifier-free guidance)
        if 'audio' in cfg_cond:
            audio_feat_null = self.null_audio_feat.expand(batch_size, self.n_motions, -1)
        else:
            audio_feat_null = audio_feat

        if 'style' in cfg_cond:
            person_feat_null = torch.cat([shape_feat, self.null_style_feat.expand(batch_size, -1, -1)], dim=-1)
        else:
            if self.use_style:
                person_feat_null = torch.cat([shape_feat, style_feat], dim=-1)
            else:
                person_feat_null = shape_feat

        audio_feat_in = [audio_feat_null]
        person_feat_in = [person_feat_null]
        for cond in cfg_cond:
            if cond == 'audio':
                audio_feat_in.append(audio_feat)
                person_feat_in.append(person_feat_null)
            elif cond == 'style':
                if cfg_mode == 'independent':
                    audio_feat_in.append(audio_feat_null)
                elif cfg_mode == 'incremental':
                    audio_feat_in.append(audio_feat)
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')
                person_feat_in.append(torch.cat([shape_feat, style_feat], dim=-1))

        n_entries = len(audio_feat_in)
        audio_feat_in = torch.cat(audio_feat_in, dim=0)
        person_feat_in = torch.cat(person_feat_in, dim=0)
        prev_motion_feat_in = torch.cat([prev_motion_feat] * n_entries, dim=0)
        prev_audio_feat_in = torch.cat([prev_audio_feat] * n_entries, dim=0)
        indicator_in = torch.cat([indicator] * n_entries, dim=0) if indicator is not None else None
        style_feat = torch.cat([style_feat] * n_entries, dim=0) if style_feat is not None else None

        traj = {self.diffusion_sched.num_steps: motion_at_T}
        for t in range(self.diffusion_sched.num_steps, 0, -1):
            if t > 1:
                z = torch.randn_like(motion_at_T)
            else:
                z = torch.zeros_like(motion_at_T)

            alpha = self.diffusion_sched.alphas[t]
            alpha_bar = self.diffusion_sched.alpha_bars[t]
            alpha_bar_prev = self.diffusion_sched.alpha_bars[t - 1]
            sigma = self.diffusion_sched.get_sigmas(t, flexibility)

            motion_at_t = traj[t]
            motion_in = torch.cat([motion_at_t] * n_entries, dim=0)
            t = int(t)
            step_in = torch.tensor([t] * batch_size, device=self.device)
            step_in = torch.cat([step_in] * n_entries, dim=0)
            results = self.denoising_net(motion_in, audio_feat_in, person_feat_in, style_feat, prev_motion_feat_in,
                                         prev_audio_feat_in, step_in, indicator_in)
            # Apply thresholding if specified
            if dynamic_threshold:
                dt_ratio, dt_min, dt_max = dynamic_threshold
                abs_results = results[:, -self.n_motions:].reshape(batch_size * n_entries, -1).abs()
                s = torch.quantile(abs_results, dt_ratio, dim=1) # if dt ratio us 0, then 
                s = torch.clamp(s, min=dt_min, max=dt_max)
                s = s[..., None, None]
                results = torch.clamp(results, min=-s, max=s)

            results = results.chunk(n_entries)

            # Unconditional target (CFG) or the conditional target (non-CFG)
            target_theta = results[0][:, -self.n_motions:]
            # Classifier-free Guidance (optional)
            for i in range(0, n_entries - 1):
                if cfg_mode == 'independent':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[0][:, -self.n_motions:])
                elif cfg_mode == 'incremental':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[i][:, -self.n_motions:])
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')


            # you noise up for next step
            if self.target == 'noise':
                c0 = 1 / torch.sqrt(alpha)
                c1 = (1 - alpha) / torch.sqrt(1 - alpha_bar)
                motion_next = c0 * (motion_at_t - c1 * target_theta) + sigma * z
            elif self.target == 'sample':
                c0 = (1 - alpha_bar_prev) * torch.sqrt(alpha) / (1 - alpha_bar)
                c1 = (1 - alpha) * torch.sqrt(alpha_bar_prev) / (1 - alpha_bar)
                motion_next = c0 * motion_at_t + c1 * target_theta + sigma * z
            else:
                raise ValueError('Unknown target type: {}'.format(self.target))

            traj[t - 1] = motion_next.detach()  # Stop gradient and save trajectory.
            traj[t] = traj[t].cpu()  # Move previous output to CPU memory.
            if not ret_traj:
                del traj[t]

        if ret_traj:
            return traj, motion_at_T, audio_feat
        else:
            return traj[0], motion_at_T, audio_feat

    @torch.no_grad()
    def sample_separate(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,
               motion_at_T=None, indicator=None, cfg_mode=None, cfg_cond=None, cfg_scale=1.15, flexibility=0,
               dynamic_threshold=None, ret_traj=False, alpah_t_modification=None, return_all_alpha=False):
        # Check and convert inputs
        batch_size = audio_or_feat.shape[0]

        # Check CFG conditions
        if cfg_mode is None:  # Use default CFG mode
            cfg_mode = self.cfg_mode #
        if cfg_cond is None:  # Use default CFG conditions
            cfg_cond = self.guiding_conditions
        cfg_cond = [c for c in cfg_cond if c in ['audio', 'style']]

        if not isinstance(cfg_scale, list):
            cfg_scale = [cfg_scale] * len(cfg_cond)

        # sort cfg_cond and cfg_scale
        if len(cfg_cond) > 0:
            cfg_cond, cfg_scale = zip(*sorted(zip(cfg_cond, cfg_scale), key=lambda x: ['audio', 'style'].index(x[0])))
        else:
            cfg_cond, cfg_scale = [], []

        if 'style' in cfg_cond:
            assert self.use_style and style_feat is not None

        if self.use_style:
            if style_feat is None:  # use null style feature
                style_feat = self.null_style_feat.expand(batch_size, -1, -1)
        else:
            assert style_feat is None, 'This model does not support style feature input!'

        if audio_or_feat.ndim == 2:
            # Extract audio features
            assert audio_or_feat.shape[1] == 16000 * self.n_motions / self.fps, \
                f'Incorrect audio length {audio_or_feat.shape[1]}'
            audio_feat = self.extract_audio_feature(audio_or_feat)  # (N, L, feature_dim)
        elif audio_or_feat.ndim == 3:
            assert audio_or_feat.shape[1] == self.n_motions, f'Incorrect audio feature length {audio_or_feat.shape[1]}'
            audio_feat = audio_or_feat
        else:
            raise ValueError(f'Incorrect audio input shape {audio_or_feat.shape}')

        if shape_feat.ndim == 2:
            shape_feat = shape_feat.unsqueeze(1)  # (N, 1, d_shape)
        if style_feat is not None and style_feat.ndim == 2:
            style_feat = style_feat.unsqueeze(1)  # (N, 1, d_style)

        if prev_motion_feat is None:
            prev_motion_feat = self.start_motion_feat.expand(batch_size, -1, -1)  # (N, n_prev_motions, d_motion)
        if prev_audio_feat is None:
            # (N, n_prev_motions, feature_dim)
            prev_audio_feat = self.start_audio_feat.expand(batch_size, -1, -1)

        if motion_at_T is None:
            motion_at_T = torch.randn((batch_size, self.n_motions, self.motion_feat_dim)).to(self.device)

        # Prepare input for the reverse diffusion process (including optional classifier-free guidance)
        if 'audio' in cfg_cond:
            audio_feat_null = self.null_audio_feat.expand(batch_size, self.n_motions, -1)
        else:
            audio_feat_null = audio_feat

        if 'style' in cfg_cond:
            person_feat_null = torch.cat([shape_feat, self.null_style_feat.expand(batch_size, -1, -1)], dim=-1)
        else:
            if self.use_style:
                person_feat_null = torch.cat([shape_feat, style_feat], dim=-1)
            else:
                person_feat_null = shape_feat

        audio_feat_in = [audio_feat_null]
        person_feat_in = [person_feat_null]
        for cond in cfg_cond:
            if cond == 'audio':
                audio_feat_in.append(audio_feat)
                person_feat_in.append(person_feat_null)
            elif cond == 'style':
                if cfg_mode == 'independent':
                    audio_feat_in.append(audio_feat_null)
                elif cfg_mode == 'incremental':
                    audio_feat_in.append(audio_feat)
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')
                person_feat_in.append(torch.cat([shape_feat, style_feat], dim=-1))

        n_entries = len(audio_feat_in)
        audio_feat_in = torch.cat(audio_feat_in, dim=0)
        person_feat_in = torch.cat(person_feat_in, dim=0)
        prev_motion_feat_in = torch.cat([prev_motion_feat] * n_entries, dim=0)
        prev_audio_feat_in = torch.cat([prev_audio_feat] * n_entries, dim=0)
        indicator_in = torch.cat([indicator] * n_entries, dim=0) if indicator is not None else None

        traj = {self.diffusion_sched.num_steps: motion_at_T}
        alpha_traj = []
        cumulative_static_pose = torch.zeros_like(motion_at_T).to(self.device)
        

        for t in range(self.diffusion_sched.num_steps, 0, -1):
            if t > 1:
                z = torch.randn_like(motion_at_T)
            else:
                z = torch.zeros_like(motion_at_T)

            alpha = self.diffusion_sched.alphas[t]
            alpha_bar = self.diffusion_sched.alpha_bars[t]
            alpha_bar_prev = self.diffusion_sched.alpha_bars[t - 1]
            sigma = self.diffusion_sched.get_sigmas(t, flexibility)

            motion_at_t = traj[t]
            motion_in = torch.cat([motion_at_t] * n_entries, dim=0)
            t = int(t)
            step_in = torch.tensor([t] * batch_size, device=self.device)
            step_in = torch.cat([step_in] * n_entries, dim=0)

            dynamic_features, static_features, alpha_t = self.denoising_net(motion_in, audio_feat_in, person_feat_in, style_feat, prev_motion_feat_in,
                                         prev_audio_feat_in, step_in, indicator_in, keep_separate=True)
            # Apply modification functions if specified
            if alpah_t_modification is not None:
                alpha_t = alpah_t_modification(alpha_t)
            
            alpha_t_expanded = alpha_t.unsqueeze(-1)  # Shape: (N, L, num_of_basis, 1)
            
            if self.use_head_alpha:
                weighted_features = static_features * alpha_t_expanded  # Shape: (N, L, num_of_basis, d_motion)
                static_features = weighted_features.sum(dim=2)  # Shape: (N, L, d_motion)
            else:
                static_features_face = static_features[:, :, :, :-3] # N, L, num_of_basis, d_motion - 3
                static_features_pose = static_features[:, :, :, -3:] # N, L, num_of_basis, 3
                weighted_features = static_features_face * alpha_t_expanded  # Shape: (N, L, num_of_basis, d_motion - 3)
                sumed_static_features_face = weighted_features.sum(dim=2)  # Shape: (N, L, d_motion - 3)
                static_features_pose = static_features_pose.sum(dim=2)  # Shape: (N, L, 3)
                static_features = torch.concat([sumed_static_features_face, static_features_pose], dim=-1) # N, L, d_motion

            results = dynamic_features + static_features
            # Apply thresholding if specified
            if dynamic_threshold:
                dt_ratio, dt_min, dt_max = dynamic_threshold
                abs_results = results[:, -self.n_motions:].reshape(batch_size * n_entries, -1).abs()
                s = torch.quantile(abs_results, dt_ratio, dim=1) # if dt ratio us 0, then 
                s = torch.clamp(s, min=dt_min, max=dt_max)
                s = s[..., None, None]
                results = torch.clamp(results, min=-s, max=s)

            results = results.chunk(n_entries)
            static_features = static_features.chunk(n_entries)
            dynamic_features = dynamic_features.chunk(n_entries)
            alpha_t = alpha_t.chunk(n_entries)

            # Unconditional target (CFG) or the conditional target (non-CFG)
            target_theta = results[0][:, -self.n_motions:]
            target_theta_static = static_features[0][:, -self.n_motions:]
            target_theta_dynamic = dynamic_features[0][:, -self.n_motions:]
            target_alpha_t = alpha_t[0][:, -self.n_motions:] # has the shape of (N, L, d_motion)
            
            # Classifier-free Guidance (optional)
            for i in range(0, n_entries - 1):
                if cfg_mode == 'independent':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[0][:, -self.n_motions:])
                    target_theta_dynamic += cfg_scale[i] * (
                                dynamic_features[i + 1][:, -self.n_motions:] - dynamic_features[0][:, -self.n_motions:])
                    target_theta_static += cfg_scale[i] * (
                                static_features[i + 1][:, -self.n_motions:] - static_features[0][:, -self.n_motions:])
                    target_alpha_t += cfg_scale[i] * ( 
                                alpha_t[i + 1][:, -self.n_motions:] - alpha_t[0][:, -self.n_motions:])
                    
                elif cfg_mode == 'incremental':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[i][:, -self.n_motions:])
                    target_theta_dynamic += cfg_scale[i] * (
                                dynamic_features[i + 1][:, -self.n_motions:] - dynamic_features[i][:, -self.n_motions:])
                    target_theta_static += cfg_scale[i] * (
                                static_features[i + 1][:, -self.n_motions:] - static_features[i][:, -self.n_motions:])
                    target_alpha_t += cfg_scale[i] * (
                                alpha_t[i + 1][:, -self.n_motions:] - alpha_t[i][:, -self.n_motions:])
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')


            # you noise up for next step
            if self.target == 'noise':
                c0 = 1 / torch.sqrt(alpha)
                c1 = (1 - alpha) / torch.sqrt(1 - alpha_bar)
                motion_next = c0 * (motion_at_t - c1 * target_theta) + sigma * z
                cumulative_static_pose = cumulative_static_pose + c1 * target_theta_static

            elif self.target == 'sample':
                c0 = (1 - alpha_bar_prev) * torch.sqrt(alpha) / (1 - alpha_bar)
                c1 = (1 - alpha) * torch.sqrt(alpha_bar_prev) / (1 - alpha_bar)
                motion_next = c0 * motion_at_t + c1 * target_theta + sigma * z
                cumulative_static_pose = cumulative_static_pose + c1 * target_theta_static
            else:
                raise ValueError('Unknown target type: {}'.format(self.target))

            traj[t - 1] = motion_next.detach()  # Stop gradient and save trajectory.
            alpha_traj.append(target_alpha_t.detach())
            traj[t] = traj[t].cpu()  # Move previous output to CPU memory.
            if not ret_traj:
                del traj[t]
            
        alpha_traj = torch.cat(alpha_traj, dim=0)

        if ret_traj:
            return traj, motion_at_T, audio_feat
        else:
            if return_all_alpha:
                return traj[0], motion_at_T, audio_feat, target_theta_dynamic.detach(), cumulative_static_pose.detach(), alpha_traj
            else:
                return traj[0], motion_at_T, audio_feat, target_theta_dynamic.detach(), cumulative_static_pose.detach(), target_alpha_t.detach()

    @torch.no_grad()
    def sample_with_guide(self, audio_or_feat, shape_feat, style_feat=None, prev_motion_feat=None, prev_audio_feat=None,
               motion_at_T=None, indicator=None, cfg_mode=None, cfg_cond=None, cfg_scale=1.15, flexibility=0,
               dynamic_threshold=None, ret_traj=False, guidance_indice=None, guidance_values=None):
        # Check and convert inputs
        batch_size = audio_or_feat.shape[0]

        # Check CFG conditions
        if cfg_mode is None:  # Use default CFG mode
            cfg_mode = self.cfg_mode #
        if cfg_cond is None:  # Use default CFG conditions
            cfg_cond = self.guiding_conditions
        cfg_cond = [c for c in cfg_cond if c in ['audio', 'style']]

        if not isinstance(cfg_scale, list):
            cfg_scale = [cfg_scale] * len(cfg_cond)

        # sort cfg_cond and cfg_scale
        if len(cfg_cond) > 0:
            cfg_cond, cfg_scale = zip(*sorted(zip(cfg_cond, cfg_scale), key=lambda x: ['audio', 'style'].index(x[0])))
        else:
            cfg_cond, cfg_scale = [], []

        if 'style' in cfg_cond:
            assert self.use_style and style_feat is not None

        if self.use_style:
            if style_feat is None:  # use null style feature
                style_feat = self.null_style_feat.expand(batch_size, -1, -1)
        else:
            assert style_feat is None, 'This model does not support style feature input!'

        if audio_or_feat.ndim == 2:
            # Extract audio features
            assert audio_or_feat.shape[1] == 16000 * self.n_motions / self.fps, \
                f'Incorrect audio length {audio_or_feat.shape[1]}'
            audio_feat = self.extract_audio_feature(audio_or_feat)  # (N, L, feature_dim)
        elif audio_or_feat.ndim == 3:
            assert audio_or_feat.shape[1] == self.n_motions, f'Incorrect audio feature length {audio_or_feat.shape[1]}'
            audio_feat = audio_or_feat
        else:
            raise ValueError(f'Incorrect audio input shape {audio_or_feat.shape}')

        if shape_feat.ndim == 2:
            shape_feat = shape_feat.unsqueeze(1)  # (N, 1, d_shape)
        if style_feat is not None and style_feat.ndim == 2:
            style_feat = style_feat.unsqueeze(1)  # (N, 1, d_style)

        if prev_motion_feat is None:
            prev_motion_feat = self.start_motion_feat.expand(batch_size, -1, -1)  # (N, n_prev_motions, d_motion)
        if prev_audio_feat is None:
            # (N, n_prev_motions, feature_dim)
            prev_audio_feat = self.start_audio_feat.expand(batch_size, -1, -1)

        if motion_at_T is None:
            motion_at_T = torch.randn((batch_size, self.n_motions, self.motion_feat_dim)).to(self.device)

        # Prepare input for the reverse diffusion process (including optional classifier-free guidance)
        if 'audio' in cfg_cond:
            audio_feat_null = self.null_audio_feat.expand(batch_size, self.n_motions, -1)
        else:
            audio_feat_null = audio_feat

        if 'style' in cfg_cond:
            person_feat_null = torch.cat([shape_feat, self.null_style_feat.expand(batch_size, -1, -1)], dim=-1)
        else:
            if self.use_style:
                person_feat_null = torch.cat([shape_feat, style_feat], dim=-1)
            else:
                person_feat_null = shape_feat

        audio_feat_in = [audio_feat_null]
        person_feat_in = [person_feat_null]
        for cond in cfg_cond:
            if cond == 'audio':
                audio_feat_in.append(audio_feat)
                person_feat_in.append(person_feat_null)
            elif cond == 'style':
                if cfg_mode == 'independent':
                    audio_feat_in.append(audio_feat_null)
                elif cfg_mode == 'incremental':
                    audio_feat_in.append(audio_feat)
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')
                person_feat_in.append(torch.cat([shape_feat, style_feat], dim=-1))

        n_entries = len(audio_feat_in)
        audio_feat_in = torch.cat(audio_feat_in, dim=0)
        person_feat_in = torch.cat(person_feat_in, dim=0)
        prev_motion_feat_in = torch.cat([prev_motion_feat] * n_entries, dim=0)
        prev_audio_feat_in = torch.cat([prev_audio_feat] * n_entries, dim=0)
        indicator_in = torch.cat([indicator] * n_entries, dim=0) if indicator is not None else None

        traj = {self.diffusion_sched.num_steps: motion_at_T}
        for t in range(self.diffusion_sched.num_steps, 0, -1):
            if t > 1:
                z = torch.randn_like(motion_at_T)
            else:
                z = torch.zeros_like(motion_at_T)

            alpha = self.diffusion_sched.alphas[t]
            alpha_bar = self.diffusion_sched.alpha_bars[t]
            alpha_bar_prev = self.diffusion_sched.alpha_bars[t - 1]
            sigma = self.diffusion_sched.get_sigmas(t, flexibility)

            motion_at_t = traj[t]
            motion_in = torch.cat([motion_at_t] * n_entries, dim=0)
            step_in = torch.tensor([t] * batch_size, device=self.device)
            step_in = torch.cat([step_in] * n_entries, dim=0)

            # ============================= Add naive inpainting guidance =============================
            
            if guidance_indice is not None:
                motion_in[:, guidance_indice, :] = guidance_values

            # ============================= Add naive inpainting guidance =============================

            results = self.denoising_net(motion_in, audio_feat_in, person_feat_in, prev_motion_feat_in,
                                         prev_audio_feat_in, step_in, indicator_in)

            # Apply thresholding if specified
            if dynamic_threshold:
                dt_ratio, dt_min, dt_max = dynamic_threshold
                abs_results = results[:, -self.n_motions:].reshape(batch_size * n_entries, -1).abs()
                s = torch.quantile(abs_results, dt_ratio, dim=1) # if dt ratio us 0, then 
                s = torch.clamp(s, min=dt_min, max=dt_max)
                s = s[..., None, None]
                results = torch.clamp(results, min=-s, max=s)

            results = results.chunk(n_entries)

            # Unconditional target (CFG) or the conditional target (non-CFG)
            target_theta = results[0][:, -self.n_motions:]
            # Classifier-free Guidance (optional)
            for i in range(0, n_entries - 1):
                if cfg_mode == 'independent':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[0][:, -self.n_motions:])
                elif cfg_mode == 'incremental':
                    target_theta += cfg_scale[i] * (
                                results[i + 1][:, -self.n_motions:] - results[i][:, -self.n_motions:])
                else:
                    raise NotImplementedError(f'Unknown cfg_mode {cfg_mode}')


            # you noise up for next step
            if self.target == 'noise':
                c0 = 1 / torch.sqrt(alpha)
                c1 = (1 - alpha) / torch.sqrt(1 - alpha_bar)
                motion_next = c0 * (motion_at_t - c1 * target_theta) + sigma * z
            elif self.target == 'sample':
                c0 = (1 - alpha_bar_prev) * torch.sqrt(alpha) / (1 - alpha_bar)
                c1 = (1 - alpha) * torch.sqrt(alpha_bar_prev) / (1 - alpha_bar)
                motion_next = c0 * motion_at_t + c1 * target_theta + sigma * z
            else:
                raise ValueError('Unknown target type: {}'.format(self.target))

            traj[t - 1] = motion_next.detach()  # Stop gradient and save trajectory.
            traj[t] = traj[t].cpu()  # Move previous output to CPU memory.
            if not ret_traj:
                del traj[t]

        if ret_traj:
            return traj, motion_at_T, audio_feat
        else:
            return traj[0], motion_at_T, audio_feat

class DenoisingNetwork_MSMD(nn.Module):
    def __init__(self, args, device='cuda', motion_feat_dim=50, use_head_alpha=True, regularize_alpha="None"):
        super().__init__()

        # Model parameters
        self.regularize_alpha = regularize_alpha
        self.use_head_alpha = use_head_alpha
        self.num_of_basis = int(args.num_of_basis)
        self.use_style = args.style_enc_ckpt is not None or not args.style_enc_model_style == "diffposetalk"
        self.motion_feat_dim = motion_feat_dim # 50 / 67
        if (args.dataset_type[:9] == "HDTF_TFHP" or args.dataset_type == 'flame_mead_ravdess') and motion_feat_dim == 50:
            if args.rot_repr == 'aa':
                self.motion_feat_dim += 1 if args.no_head_pose else 4
            else:
                raise ValueError(f'Unknown rotation representation {args.rot_repr}!')
        self.shape_feat_dim = 100 # flame identity features
        if self.use_style:
            self.style_feat_dim = args.d_style
            self.person_feat_dim = self.shape_feat_dim + self.style_feat_dim
        else:
            self.person_feat_dim = self.shape_feat_dim
        self.use_indicator = args.use_indicator
        # Transformer
        self.architecture = args.architecture 
        self.feature_dim = args.feature_dim
        self.n_heads = args.n_heads
        self.n_layers = args.n_layers
        self.mlp_ratio = args.mlp_ratio
        self.align_mask_width = args.align_mask_width
        self.use_learnable_pe = not args.no_use_learnable_pe
        # sequence length
        self.n_prev_motions = args.n_prev_motions # 10
        self.n_motions = args.n_motions # 100

        # Temporal embedding for the diffusion time step
        self.TE = PositionalEncoding(self.feature_dim, max_len=args.n_diff_steps + 1)
        self.diff_step_map = nn.Sequential(
            nn.Linear(self.feature_dim, self.feature_dim),
            nn.GELU(),
            nn.Linear(self.feature_dim, self.feature_dim)
        )

        if self.use_learnable_pe:
            # Learnable positional encoding
            self.PE = nn.Parameter(torch.randn(1, 1 + self.n_prev_motions + self.n_motions, self.feature_dim))
        else:
            self.PE = PositionalEncoding(self.feature_dim)

        self.person_proj = nn.Linear(self.person_feat_dim, self.feature_dim)

        # Transformer decoder
        if self.architecture == 'decoder':
            self.feature_proj = nn.Linear(self.motion_feat_dim + (1 if self.use_indicator else 0),
                                          self.feature_dim)
            decoder_layer = nn.TransformerDecoderLayer(
                d_model=self.feature_dim, nhead=self.n_heads, dim_feedforward=self.mlp_ratio * self.feature_dim,
                activation='gelu', batch_first=True
            )
            self.transformer = nn.TransformerDecoder(decoder_layer, num_layers=self.n_layers)
            if self.align_mask_width > 0:
                motion_len = self.n_prev_motions + self.n_motions
                alignment_mask = enc_dec_mask(motion_len, motion_len, 1, self.align_mask_width - 1)
                alignment_mask = F.pad(alignment_mask, (0, 0, 1, 0), value=False)
                self.register_buffer('alignment_mask', alignment_mask)
            else:
                self.alignment_mask = None
        else:
            raise ValueError(f'Unknown architecture: {self.architecture}')


        self.static_feature_mapping = nn.ModuleList()
        for basis_i in range(0, self.num_of_basis):
            self.static_feature_mapping.append(
                nn.Sequential(
                    nn.Linear(args.d_style, self.feature_dim),
                    nn.GELU(),
                    nn.Linear(self.feature_dim, self.motion_feat_dim)
                )
            )


        # Motion decoder
        self.motion_dec = nn.Sequential(
            nn.Linear(self.feature_dim, self.feature_dim // 2),
            nn.GELU(),
            nn.Linear(self.feature_dim // 2, self.motion_feat_dim + self.num_of_basis)
        )

        self.to(device)

    @property
    def device(self):
        return next(self.parameters()).device

    def forward(self, motion_feat, audio_feat, person_feat, static_style_feat, prev_motion_feat, prev_audio_feat, step, indicator=None, keep_separate=False):
        """
        Args:
            motion_feat: (N, L, d_motion). Noisy motion feature
            audio_feat: (N, L, feature_dim)
            person_feat: (N, 1, d_person)
            static_style_feat: (N, 1, d_style): this is the same style featured concatenated into person_feat, 
                                but it will be used to generate the static offset for the motion feature
            prev_motion_feat: (N, L_p, d_motion). Padded previous motion coefficients or feature
            prev_audio_feat: (N, L_p, d_audio). Padded previous motion coefficients or feature
            step: (N,)
            indicator: (N, L). 0/1 indicator for the real (unpadded) motion feature

        Returns:
            motion_feat_target: (N, L_p + L, d_motion)
        """
        # Diffusion time step embedding
        diff_step_embedding = self.diff_step_map(self.TE.pe[0, step]).unsqueeze(1)  # (N, 1, diff_step_dim)
        person_feat = self.person_proj(person_feat)  # (N, 1, feature_dim)
        person_feat = person_feat + diff_step_embedding
        if indicator is not None:
            indicator = torch.cat([torch.zeros((indicator.shape[0], self.n_prev_motions), device=indicator.device),
                                   indicator], dim=1)  # (N, L_p + L)
            indicator = indicator.unsqueeze(-1)  # (N, L_p + L, 1)
        # Concat features and embeddings
        if self.architecture == 'decoder':
            feats_in = torch.cat([prev_motion_feat, motion_feat], dim=1)  # (N, L_p + L, d_motion)
        else:
            raise ValueError(f'Unknown architecture: {self.architecture}')
        if self.use_indicator:
            feats_in = torch.cat([feats_in, indicator], dim=-1)  # (N, L_p + L, d_motion + d_audio + 1)
        feats_in = self.feature_proj(feats_in)  # (N, L_p + L, feature_dim)
        feats_in = torch.cat([person_feat, feats_in], dim=1)  # (N, 1 + L_p + L, feature_dim)

        if self.use_learnable_pe:
            feats_in = feats_in + self.PE
        else:
            feats_in = self.PE(feats_in)

        # Transformer
        if self.architecture == 'decoder':
            audio_feat_in = torch.cat([prev_audio_feat, audio_feat], dim=1)  # (N, L_p + L, d_audio)
            feat_out = self.transformer(feats_in, audio_feat_in, memory_mask=self.alignment_mask)
        else:
            raise ValueError(f'Unknown architecture: {self.architecture}')

        # Decode predicted motion feature noise / sample
        motion_feat_target = self.motion_dec(feat_out[:, 1:])  # (N, L_p + L, d_motion)
        

        static_features = []
        for basis_i in range(0, self.num_of_basis):
            # static_style_feat is # N, 1, d_style
            basis_i = self.static_feature_mapping[basis_i](static_style_feat) # N, 1, d_motion
            basis_i = torch.tile(basis_i, (1, motion_feat_target.shape[1], 1)) # N, L, d_motion
            basis_i = basis_i.unsqueeze(2) # N, L, 1, d_motion
            static_features.append(basis_i)
        static_features = torch.concat(static_features, dim=2) # N, L, num_of_basis, d_motion 
        alphas = motion_feat_target[:, :, -self.num_of_basis:] # N, L, num_of_basis
        if self.regularize_alpha == "sigmoid":
            alphas = torch.sigmoid(alphas)
        alphas_expanded = alphas.unsqueeze(-1)  # Shape: (N, L, num_of_basis, 1)
            
        dynamic_features = motion_feat_target[:, :, :-self.num_of_basis]

        if self.use_head_alpha:
            weighted_features = static_features * alphas_expanded  # Shape: (N, L, num_of_basis, d_motion)
            sumed_static_features = weighted_features.sum(dim=2)  # Shape: (N, L, d_motion)
        else:
            if static_features.shape[0] != alphas_expanded.size()[0]:
                static_features = torch.tile(static_features, (alphas_expanded.size()[0], 1, 1, 1)) # N, L, num_of_basis, d_motion
            static_features_face = static_features[:, :, :, :-3] # N, L, num_of_basis, d_motion - 3
            static_features_pose = static_features[:, :, :, -3:] # N, L, num_of_basis, 3
            weighted_features = static_features_face * alphas_expanded  # Shape: (N, L, num_of_basis, d_motion - 3)
            sumed_static_features_face = weighted_features.sum(dim=2)  # Shape: (N, L, d_motion - 3)
            static_features_pose = static_features_pose.sum(dim=2)  # Shape: (N, L, 3)
            sumed_static_features = torch.concat([sumed_static_features_face, static_features_pose], dim=-1) # N, L, d_motion

        if keep_separate:
            return dynamic_features, static_features, alphas
        else:
            motion_feat_target = dynamic_features + sumed_static_features # N, L, d_motion
            return motion_feat_target
    

```

## Source file: datasets.py

{
  "path": "datasets.py",
  "blob_sha": "e9fb3123e796328a840c2125c609bf56c936dfdf",
  "sha256": "b59d7226f25280a2fd2316c76bd7890bbf804f9fd77db93853789c5e0f9bb550",
  "lines": 505,
  "bytes": 24666,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 17,
    "lip": 31
  },
  "url": "https://github.com/ubisoft/ubisoft-laforge-msmd/blob/41adbc570c237b4e3f11338acdf335c94dc27406/datasets.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def __init__(self, datasets, coef_stats):"
    },
    {
      "line": 27,
      "text": "def get_dataset(args, device, only_eval=False, batch_overfit_size=-1):"
    },
    {
      "line": 93,
      "text": "def incremental_mean_and_std(train_dataset, SE=False):"
    },
    {
      "line": 143,
      "text": "def load_dict_in_chunks_static(file_path):"
    },
    {
      "line": 155,
      "text": "def load_dict_in_chunks(self, file_path):"
    },
    {
      "line": 167,
      "text": "def __init__(self, pkl_file, split_file, coef_stats_file=None, original_fps=30, coef_fps=25, n_motions=100,"
    },
    {
      "line": 259,
      "text": "def __len__(self):"
    },
    {
      "line": 262,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 370,
      "text": "def get_k_indices_for_each_emotion(self, k=2):"
    },
    {
      "line": 391,
      "text": "def query_for_video(self, index):"
    },
    {
      "line": 424,
      "text": "def get_collate_fn(SE):"
    },
    {
      "line": 425,
      "text": "def pad_or_trim_audio(audio_tensor, target_length=64000):"
    },
    {
      "line": 437,
      "text": "def collate_fn(batch):"
    }
  ]
}

```text

import io
import pickle
import random
import sys
import warnings
from collections import defaultdict
from pathlib import Path
import os
import numpy as np
import torch
import torchaudio
from torch.utils import data
from scipy.interpolate import interp1d
from tqdm import tqdm
import librosa
import cv2

# https://github.com/pytorch/audio/issues/2950 , https://github.com/pytorch/audio/issues/2356
torchaudio.set_audio_backend('soundfile')
warnings.filterwarnings('ignore', message='PySoundFile failed. Trying audioread instead.')

class ConcatDataset_with_stats(data.ConcatDataset):
    def __init__(self, datasets, coef_stats):
        super(ConcatDataset_with_stats, self).__init__(datasets)
        self.coef_stats = coef_stats

def get_dataset(args, device, only_eval=False, batch_overfit_size=-1):
    print("loading dataset from ", args.data_root)
    try:
        args.data_root = Path(args.data_root)
    except:
        print("dataroot already a path")

    if batch_overfit_size > 0:
        # if we are batch overfitting, we are also not gonna random pad the audio
        do_random_pad = False
    else:
        do_random_pad = True

    if args.dataset_type == "ravdess+celebv-text-medium":
        data_root = args.data_root

        ravedess_root = Path("/data/ravdess/processed_data")

        train_dataset_ravdess = DatasetPickle(ravedess_root / 'processed_ravdess_30fps_v3.pkl', 
                                           ravedess_root / 'processed_ravdess_30fps_v3_keys_train.txt', 
                                           None, original_fps=30, coef_fps=25, no_head_pose=args.no_head_pose, SE=False, device=device, celebv_text=False, full_dataset=True)
        val_dataset_ravdess = DatasetPickle(ravedess_root / 'processed_ravdess_30fps_v3.pkl',
                                            ravedess_root / 'processed_ravdess_30fps_v3_keys_valid.txt',
                                            None, original_fps=30, coef_fps=25, no_head_pose=args.no_head_pose, SE=False, device=device, celebv_text=False, full_dataset=True)

        raw_data = {}
        for chunk in DatasetPickle.load_dict_in_chunks_static(args.data_root / "processed_data_30fps_medium_v3.pkl"):
            raw_data.update(chunk)


        train_dataset_celebv_text = DatasetPickle(args.data_root / "processed_data_30fps_medium_v3.pkl",
                                            args.data_root / 'processed_data_30fps_medium_v3_keys_train.txt',
                                            None, original_fps=30, coef_fps=25, no_head_pose=args.no_head_pose, 
                                            SE=False, pre_loaded_raw_dataset=raw_data,
                                            full_dataset=True)
        val_dataset_celebv_text = DatasetPickle(args.data_root / "processed_data_30fps_medium_v3.pkl",
                                            args.data_root / 'processed_data_30fps_medium_v3_keys_valid.txt',
                                            None, original_fps=30, coef_fps=25, no_head_pose=args.no_head_pose, 
                                            SE=False, pre_loaded_raw_dataset=raw_data, 
                                            full_dataset=True)
        
        weight_ravdess_train = 1.0 / len(train_dataset_ravdess)
        weight_celebv_text_train = 1.0 / len(train_dataset_celebv_text)
        weight_ravdess_val = 1.0 / len(val_dataset_ravdess)
        weight_celebv_text_val = 1.0 / len(val_dataset_celebv_text)

        weights_train = [weight_celebv_text_train] * len(train_dataset_celebv_text) + [weight_ravdess_train] * len(train_dataset_ravdess)
        weights_val = [weight_celebv_text_val] * len(val_dataset_celebv_text) + [weight_ravdess_val] * len(val_dataset_ravdess)

        train_dataset = data.ConcatDataset([train_dataset_celebv_text, train_dataset_ravdess])
        val_dataset = data.ConcatDataset([val_dataset_celebv_text, val_dataset_ravdess])

        sampler_train = data.WeightedRandomSampler(weights_train, num_samples=len(weights_train), replacement=True)
        sampler_val = data.WeightedRandomSampler(weights_val, num_samples=len(weights_val), replacement=True)

        train_loader = data.DataLoader(train_dataset, batch_size=args.batch_size,
                                    num_workers=args.num_workers, pin_memory=True, drop_last=True,
                                    persistent_workers=True, collate_fn=DatasetPickle.get_collate_fn(SE=False), sampler=sampler_train)
        val_loader = data.DataLoader(val_dataset, batch_size=args.batch_size,
                                    num_workers=args.num_workers, drop_last=True,
                                    collate_fn=DatasetPickle.get_collate_fn(SE=False), sampler=sampler_val)
    else:
        raise ValueError(f"Unsupported dataset type: {args.dataset_type}")

    return train_dataset, val_dataset, train_loader, val_loader

def incremental_mean_and_std(train_dataset, SE=False):
    exp_sum = 0
    exp_sum_of_squares = 0
    pose_sum = 0
    pose_sum_of_squares = 0
    num_elements = 0
    for i in tqdm(range(len(train_dataset))):
        entry_i = train_dataset[i]
        if not SE:
            # Extract expression and pose tensors for both frames
            exp_0 = entry_i[1][0]['motion'][:, :64]
            exp_1 = entry_i[1][1]['motion'][:, :64]
            pose_0 = entry_i[1][0]['motion'][:, 64:]
            pose_1 = entry_i[1][1]['motion'][:, 64:]
        if SE:
            exp_0 = entry_i[0][:, :64]
            exp_1 = entry_i[1][:, :64]
            pose_0 = entry_i[0][:, 64:]
            pose_1 = entry_i[1][:, 64:]         
        # Update sum and sum of squares for expressions
        exp_sum += exp_0.sum(dim=0)
        exp_sum_of_squares += (exp_0 ** 2).sum(dim=0)
        exp_sum += exp_1.sum(dim=0)
        exp_sum_of_squares += (exp_1 ** 2).sum(dim=0)

        # Update sum and sum of squares for poses
        pose_sum += pose_0.sum(dim=0)
        pose_sum_of_squares += (pose_0 ** 2).sum(dim=0)
        pose_sum += pose_1.sum(dim=0)
        pose_sum_of_squares += (pose_1 ** 2).sum(dim=0)
        
        # Update the total number of elements processed
        num_elements += exp_0.shape[0] + exp_1.shape[0]

    # Compute the mean for expressions and poses
    exp_mean = exp_sum / num_elements
    pose_mean = pose_sum / num_elements

    # Compute the variance for expressions and poses
    exp_var = (exp_sum_of_squares / num_elements) - (exp_mean ** 2)
    pose_var = (pose_sum_of_squares / num_elements) - (pose_mean ** 2)

    # Standard deviation is the square root of variance
    exp_std = torch.sqrt(exp_var)
    pose_std = torch.sqrt(pose_var)

    return exp_mean, exp_std, pose_mean, pose_std

class DatasetPickle(data.Dataset):
    @staticmethod
    def load_dict_in_chunks_static(file_path):
        """
        Load a dictionary in chunks from a pickle file.
        """
        with open(file_path, 'rb') as f:
            while True:
                try:
                    chunk = pickle.load(f)
                    yield chunk
                except EOFError:
                    break

    def load_dict_in_chunks(self, file_path):
        """
        Load a dictionary in chunks from a pickle file.
        """
        with open(file_path, 'rb') as f:
            while True:
                try:
                    chunk = pickle.load(f)
                    yield chunk
                except EOFError:
                    break  # End of file reached
    
    def __init__(self, pkl_file, split_file, coef_stats_file=None, original_fps=30, coef_fps=25, n_motions=100,
                 rot_repr='aa', no_head_pose=False, clip_len=100, device='cpu', SE=False, full_dataset=False, 
                 pre_loaded_raw_dataset=None, celebv_text=True, random_crop=True, batch_overfit_size=-1):
        self.split_file = split_file
        self.pkl_file = pkl_file
        self.valid_id = []
        # load the valid id file (only for celebv-text)
        if celebv_text:
            with open("/data/celebv-text/keys.txt", 'r') as f:
                for line in f:
                    self.valid_id.append(line.strip())
        # load the split file
        self.file_names = []
        with open(split_file, 'r') as f:
            for line in f:
                name = line.strip()
                # self.file_names.append(line.strip())
                if celebv_text:
                    if name in self.valid_id:
                        self.file_names.append(name)
                else:
                    self.file_names.append(name)
        # if overfit_mode is not -1, only take the first overfit_mode entries
        if batch_overfit_size > 0:
            self.file_names = self.file_names[:batch_overfit_size]

        # load the data 
        if pre_loaded_raw_dataset is not None:
            raw_data = pre_loaded_raw_dataset
        elif not full_dataset:
            raw_data = pickle.load(open(pkl_file, 'rb'))
        else:
            raw_data = {}
            for chunk in self.load_dict_in_chunks(pkl_file):
                raw_data.update(chunk)
        self.data = {}
        for key in self.file_names:
            self.data[key] = raw_data[key]
        
        # resample the data to 25 fps:
        # resample the head_orientation and expression_code to 25 fps from 30 fps
        if original_fps != coef_fps:
            for key in self.file_names:
                original_dict = self.data[key]
                original_expression_code = original_dict["expression_code"]
                original_head_orientation = original_dict["head_orientation"]
                new_dict = {"audio": original_dict["audio"]}
                # resample expressioncode to 25 fps down from 30 using interp1d
                # original_expression_code.shape
                x = np.linspace(0, 1, num=original_expression_code.shape[0])
                xnew = np.linspace(0, 1, num=int(round(original_expression_code.shape[0]/original_fps*coef_fps)))
                f_exp = interp1d(x, original_expression_code, axis=0)
                new_expression_code = f_exp(xnew)
                # also resample head_orientation to 25 fps
                f_head = interp1d(x, original_head_orientation, axis=0)
                new_head_orientation = f_head(xnew)
                new_dict["expression_code"] = new_expression_code
                new_dict["head_orientation"] = new_head_orientation
                self.data[key] = new_dict
                del original_dict
        print("finished data resampling")

        if coef_stats_file is not None:
            coef_stats = dict(np.load(coef_stats_file))
            self.coef_stats = {x: torch.tensor(coef_stats[x]) for x in coef_stats}
        else:
            self.coef_stats = None
            print('Warning: No stats file found. Coef will not be normalized.')
        self.device = device
        self.coef_fps = coef_fps
        self.clip_len = clip_len
        self.audio_unit = 16000. / self.coef_fps  # num of samples per frame
        self.n_motions = n_motions
        self.n_audio_samples = round(self.audio_unit * self.n_motions)
        self.coef_total_len = int(self.n_motions * 2.1)
        self.audio_total_len = round(self.audio_unit * self.coef_total_len)
        self.random_crop = random_crop
        self.rot_representation = rot_repr
        self.no_head_pose = no_head_pose
        self.SE = SE

        # Read split file
        self.entries = self.file_names
        if coef_stats_file is None:
            exp_mean, exp_std, pose_mean, pose_std = incremental_mean_and_std(self, self.SE)
            self.coef_stats = {}
            self.coef_stats['exp_mean'] = exp_mean   
            self.coef_stats['exp_std'] = exp_std
            self.coef_stats['pose_mean'] = pose_mean
            self.coef_stats['pose_std'] = pose_std
        self.coef_stats = {x: torch.tensor(self.coef_stats[x]).float() for x in self.coef_stats}

    def __len__(self):
        return len(self.entries)
        
    def __getitem__(self, index):
        clip_dict = self.data[self.entries[index]]
        audio = clip_dict["audio"]
        expression_code = clip_dict["expression_code"]
        head_orientation = clip_dict["head_orientation"]

        # normalize the audio
        audio_mean = audio.mean() # note these are calculated before padding to ensure that the mean and std are normalized correctly.
        audio_std = audio.std()
        audio = (audio - audio_mean) / (audio_std + 1e-5)
        
        # length of the goal clip
        goal_total_length = self.coef_total_len
        goal_each_clip_length = self.clip_len
        
        # length of the current clip
        current_clip_length = expression_code.shape[0]
        
        # select a starting frame to ensure that after cropping, the second clip will have at least half of goal_each_clip_length
        if self.random_crop:
            if current_clip_length > goal_total_length:
                start_frame1 = np.random.randint(0, current_clip_length - goal_total_length + 1)
                end_frame1 = start_frame1 + goal_each_clip_length
                start_frame2 = start_frame1 + goal_each_clip_length
                end_frame2 = start_frame2 + goal_each_clip_length
            elif current_clip_length == goal_total_length:
                start_frame1 = 0
                end_frame1 = goal_each_clip_length
                start_frame2 = goal_each_clip_length
                end_frame2 = goal_each_clip_length * 2
            else:
                frames_to_pad = goal_total_length - current_clip_length
                # split this down the middle randomly
                frames_to_pad_front = np.random.randint(0, frames_to_pad)
                frames_to_pad_back = frames_to_pad - frames_to_pad_front
                frames_to_pad_front = int(round(frames_to_pad_front))
                frames_to_pad_back = int(round(frames_to_pad_back))
                expression_code = np.pad(expression_code, ((frames_to_pad_front, frames_to_pad_back), (0, 0)), 'constant', constant_values=0)
                head_orientation = np.pad(head_orientation, ((frames_to_pad_front, frames_to_pad_back), (0, 0)), 'constant', constant_values=0)
                
                # audio frames to pad = frames_to_pad * audio_unit
                # note that the audio might be slightly shorter or longer than the video, so we need to pad the audio twice
                audio_frames_to_pad_front = int(round(frames_to_pad_front * self.audio_unit))
                audio_frames_to_pad_back = int(round(frames_to_pad_back * self.audio_unit))
                
                audio = np.pad(audio, ((int(audio_frames_to_pad_front), int(audio_frames_to_pad_back))), 'constant', constant_values=0)
                audio_length = audio.shape[0]

                # if the audio is still shorter than the goal length, pad it with zeros at the end (this is not elegant but what can we do......)
                audio_minimal_length = goal_total_length * self.audio_unit
                audio_minimal_length = int(round(audio_minimal_length))
                if audio_length < audio_minimal_length:
                    audio = np.pad(audio, (0, audio_minimal_length - audio_length), 'constant', constant_values=0)

                start_frame1 = 0
                end_frame1 = goal_each_clip_length
                start_frame2 = goal_each_clip_length
                end_frame2 = goal_each_clip_length * 2
            # Crop the audio and coef
        else:
            start_frame1 = 0
            end_frame1 = goal_each_clip_length
            start_frame2 = goal_each_clip_length
            end_frame2 = goal_each_clip_length * 2

            # pad the audio and coef at the end 
            expression_code = np.pad(expression_code, ((0, int(round(goal_total_length - current_clip_length))), (0, 0)), 'constant', constant_values=0)
            head_orientation = np.pad(head_orientation, ((0, int(round(goal_total_length - current_clip_length))), (0, 0)), 'constant', constant_values=0)
            audio = np.pad(audio, (0, int(round(goal_total_length * self.audio_unit)) - audio.shape[0]), 'constant', constant_values=0)

        expression_code_frame_0 = expression_code[start_frame1:end_frame1]
        expression_code_frame_1 = expression_code[start_frame2:end_frame2]
        head_orientation_frame_0 = head_orientation[start_frame1:end_frame1]
        head_orientation_frame_1 = head_orientation[start_frame2:end_frame2]
        audio_frame_0 = audio[int(start_frame1 * self.audio_unit):int(end_frame1 * self.audio_unit)]
        audio_frame_1 = audio[int(start_frame2 * self.audio_unit):int(end_frame2 * self.audio_unit)]

        # concatenate expression and head orientation
        expression_code_frame_0 = torch.tensor(expression_code_frame_0).float()
        expression_code_frame_1 = torch.tensor(expression_code_frame_1).float()
        head_orientation_frame_0 = torch.tensor(head_orientation_frame_0).float()
        head_orientation_frame_1 = torch.tensor(head_orientation_frame_1).float()

        # normalize coef if applicable
        if self.coef_stats is not None:
            expression_code_frame_0 = (expression_code_frame_0 - self.coef_stats['exp_mean']) / (self.coef_stats['exp_std'] + 1e-9)
            expression_code_frame_1 = (expression_code_frame_1 - self.coef_stats['exp_mean']) / (self.coef_stats['exp_std'] + 1e-9)
            head_orientation_frame_0 = (head_orientation_frame_0 - self.coef_stats['pose_mean']) / (self.coef_stats['pose_std'] + 1e-9)
            head_orientation_frame_1 = (head_orientation_frame_1 - self.coef_stats['pose_mean']) / (self.coef_stats['pose_std'] + 1e-9)

        motion_coef_frame_0 = torch.cat([expression_code_frame_0, head_orientation_frame_0], axis=-1)
        motion_coef_frame_1 = torch.cat([expression_code_frame_1, head_orientation_frame_1], axis=-1)

        shape_frame_0 = torch.zeros((motion_coef_frame_0.shape[0], 100)).float()
        shape_frame_1 = torch.zeros((motion_coef_frame_1.shape[0], 100)).float()

        coef_dict_0 = {"shape": shape_frame_0, "motion": motion_coef_frame_0}
        coef_dict_1 = {"shape": shape_frame_1, "motion": motion_coef_frame_1}

        # turning all the numpy arrays into torch tensors
        audio_frame_0 = torch.tensor(audio_frame_0).float()
        audio_frame_1 = torch.tensor(audio_frame_1).float()
        
        if self.SE:
            return [motion_coef_frame_0, motion_coef_frame_1]
        else:
            return [audio_frame_0, audio_frame_1], [coef_dict_0, coef_dict_1], (audio_mean, audio_std)
    
    def get_k_indices_for_each_emotion(self, k=2):
        # 1 to 8
        emotions = ["01", "02", "03", "04", "05", "06", "07", "08"]
        emotion_indices = {}
        for emotion in emotions:
            emotion_indices[emotion] = []
            for count in range(0, k):
                # randomly sample from the entries
                index = np.random.randint(0, len(self.entries))
                count = 0
                while self.entries[index].split("-")[2] != emotion:
                    index = np.random.randint(0, len(self.entries))
                    count += 1
                    if count >= 100:
                        break
                if self.entries[index].split("-")[2] == emotion:
                    emotion_indices[emotion].append(index)
                else:
                    continue
        return emotion_indices

    def query_for_video(self, index):
        video_name = self.entries[index]
        if not video_name in self.file_names:
            Exception("Video name not found in the dataset")
        expression_code = self.data[video_name]["expression_code"]
        pose = self.data[video_name]["head_orientation"]
        audio = self.data[video_name]["audio"]
        # normalize the audio
        audio_mean = audio.mean() # note these are calculated before padding to ensure that the mean and std are normalized correctly.
        audio_std = audio.std()
        audio = (audio - audio_mean) / (audio_std + 1e-5)
        
        # length of the current clip
        current_clip_length = expression_code.shape[0]
        
        # concatenate expression and head orientation
        expression_code = torch.tensor(expression_code).float()
        pose = torch.tensor(pose).float()
        # normalize coef if applicable
        if self.coef_stats is not None:
            expression_code = (expression_code - self.coef_stats['exp_mean']) / (self.coef_stats['exp_std'] + 1e-9)
            pose = (pose - self.coef_stats['pose_mean']) / (self.coef_stats['pose_std'] + 1e-9)
        
        motion = torch.cat([expression_code, pose], axis=-1)
        shape = torch.zeros((motion.shape[0], 100)).float()
        coef_dict = {"shape": shape, "motion": motion}
        
        # turning all the numpy arrays into torch tensors
        audio = torch.tensor(audio).float()

        return audio, coef_dict, (audio_mean, audio_std)

    @staticmethod
    def get_collate_fn(SE):
        def pad_or_trim_audio(audio_tensor, target_length=64000):
            """Helper function to ensure all audio tensors have the same length"""
            current_length = audio_tensor.size(0)
            if current_length < target_length:
                # Pad with zeros
                padding = target_length - current_length
                return torch.nn.functional.pad(audio_tensor, (0, padding), 'constant', 0)
            elif current_length > target_length:
                # Trim to target length
                return audio_tensor[:target_length]
            return audio_tensor

        def collate_fn(batch):
            if SE:
                coef_0 = []
                coef_1 = []
                for i in range(len(batch)):
                    coef_0.append(batch[i][0])
                    coef_1.append(batch[i][1])
                coef_0 = torch.stack(coef_0, dim=0)
                coef_1 = torch.stack(coef_1, dim=0)
                return [coef_0, coef_1]
            else:
                audio_0 = []
                audio_1 = []
                motion_0 = []
                motion_1 = []
                shape_0 = []
                shape_1 = []
                audio_mean = []
                audio_std = []

                # Fixed target length for audio
                target_length = 64000

                # Process each item in batch
                for i in range(len(batch)):
                    # Pad or trim audio to target length
                    audio_0_padded = pad_or_trim_audio(batch[i][0][0], target_length)
                    audio_1_padded = pad_or_trim_audio(batch[i][0][1], target_length)
                    
                    # Append processed items
                    audio_0.append(audio_0_padded)
                    audio_1.append(audio_1_padded)
                    motion_0.append(batch[i][1][0]["motion"])
                    motion_1.append(batch[i][1][1]["motion"])
                    shape_0.append(batch[i][1][0]["shape"])
                    shape_1.append(batch[i][1][1]["shape"])
                    audio_mean.append(batch[i][2][0])
                    audio_std.append(batch[i][2][1])

                try:
                    # Stack all tensors
                    audio_0 = torch.stack(audio_0, dim=0)
                    audio_1 = torch.stack(audio_1, dim=0)
                    motion_0 = torch.stack(motion_0, dim=0)
                    motion_1 = torch.stack(motion_1, dim=0)
                    shape_0 = torch.stack(shape_0, dim=0)
                    shape_1 = torch.stack(shape_1, dim=0)
                except RuntimeError as e:
                    shapes_info = {
                        'audio_0': [x.shape for x in audio_0],
                        'audio_1': [x.shape for x in audio_1],
                        'motion_0': [x.shape for x in motion_0],
                        'motion_1': [x.shape for x in motion_1],
                        'shape_0': [x.shape for x in shape_0],
                        'shape_1': [x.shape for x in shape_1]
                    }
                    raise RuntimeError(f"Failed to stack tensors. Shapes: {shapes_info}. Original error: {str(e)}")

                # Process audio statistics
                audio_mean = torch.tensor(audio_mean).float().mean()
                audio_std = torch.tensor(audio_std).float().mean()

                # Create coefficient dictionaries
                coef_0 = {"shape": shape_0, "motion": motion_0}
                coef_1 = {"shape": shape_1, "motion": motion_1}

                return [audio_0, audio_1], [coef_0, coef_1], (audio_mean, audio_std)

        return collate_fn

```

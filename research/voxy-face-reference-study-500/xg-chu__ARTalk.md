# Facial rendering source scan: xg-chu/ARTalk

Repository: https://github.com/xg-chu/ARTalk

Pinned source revision: 5571ab00382c8eec08c2743fe222cd549771dfc4

Discovery topic: lip sync 3D

Description: ARTalk generates realistic 3D head motions (lip sync, blinking, expressions, head poses) from audio in ⚡ real-time ⚡.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"flame": ["artalk/flame_model/FLAME.py", "artalk/flame_model/__init__.py", "artalk/flame_model/lbs.py", "artalk/flame_model/renderer_utils.py", "train_code/core/libs/flame_model/FLAME.py", "train_code/core/libs/flame_model/__init__.py", "train_code/core/libs/flame_model/lbs.py", "train_code/core/libs/flame_model/renderer_utils.py"], "lbs": ["artalk/flame_model/lbs.py", "train_code/core/libs/flame_model/lbs.py"]}

## Source file: artalk/flame_model/lbs.py

{
  "path": "artalk/flame_model/lbs.py",
  "blob_sha": "354014ce973bcfd0257075d65caa77e89bc90a42",
  "sha256": "895545c845d06b2b36f4ae2dba09797d0997b670d738217e2b3d29d6cb943e64",
  "lines": 383,
  "bytes": 13959,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 19
  },
  "url": "https://github.com/xg-chu/ARTalk/blob/5571ab00382c8eec08c2743fe222cd549771dfc4/artalk/flame_model/lbs.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 36,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 103,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 142,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 235,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 255,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 279,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 313,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 326,
      "text": "def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):"
    }
  ]
}

```text

# THIS FILE HAS BEEN COPIED FROM THE EMOCA TRAINING REPOSITORY

# -*- coding: utf-8 -*-

# Max-Planck-Gesellschaft zur Förderung der Wissenschaften e.V. (MPG) is
# holder of all proprietary rights on this computer program.
# You can only use this computer program if you have closed
# a license agreement with MPG or you get the right to use the computer
# program from someone who is authorized to grant you that right.
# Any use of the computer program without a valid license is prohibited and
# liable to prosecution.
#
# Copyright©2022 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: ps-license@tuebingen.mpg.de

from __future__ import absolute_import
from __future__ import print_function
from __future__ import division

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
        lbs_weights, pose2rot=True, dtype=torch.float32, detach_pose_correctives=True):
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

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs) \
            .view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1),
                                    posedirs).view(batch_size, -1, 3)

    if detach_pose_correctives:
        pose_offsets = pose_offsets.detach()

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

    # The last column of the transformations contains the posed joints
    posed_joints = transforms[:, :, :3, 3]

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(
        torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])

    return posed_joints, rel_transforms


```

## Source file: train_code/core/libs/flame_model/lbs.py

{
  "path": "train_code/core/libs/flame_model/lbs.py",
  "blob_sha": "8eba558d3bdafd348a03e603156026d5c5ce385a",
  "sha256": "43ccc7178b9c416d2496da3b2b09d0e0b3ae8381558067f4f24d094ab56c82d6",
  "lines": 375,
  "bytes": 13134,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 19
  },
  "url": "https://github.com/xg-chu/ARTalk/blob/5571ab00382c8eec08c2743fe222cd549771dfc4/train_code/core/libs/flame_model/lbs.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 37,
      "text": "def find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 96,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 132,
      "text": "def lbs("
    },
    {
      "line": 231,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 251,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 275,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 308,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 320,
      "text": "def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):"
    }
  ]
}

```text

# THIS FILE HAS BEEN COPIED FROM THE EMOCA TRAINING REPOSITORY

# -*- coding: utf-8 -*-

# Max-Planck-Gesellschaft zur Förderung der Wissenschaften e.V. (MPG) is
# holder of all proprietary rights on this computer program.
# You can only use this computer program if you have closed
# a license agreement with MPG or you get the right to use the computer
# program from someone who is authorized to grant you that right.
# Any use of the computer program without a valid license is prohibited and
# liable to prosecution.
#
# Copyright©2022 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: ps-license@tuebingen.mpg.de

from __future__ import absolute_import
from __future__ import print_function
from __future__ import division

import numpy as np

import torch
import torch.nn.functional as F


def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] + rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


def find_dynamic_lmk_idx_and_bcoords(
    vertices, pose, dynamic_lmk_faces_idx, dynamic_lmk_b_coords, neck_kin_chain, dtype=torch.float32
):
    """Compute the faces, barycentric coordinates for the dynamic landmarks


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
    """

    batch_size = vertices.shape[0]

    aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1, neck_kin_chain)
    rot_mats = batch_rodrigues(aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)

    rel_rot_mat = torch.eye(3, device=vertices.device, dtype=dtype).unsqueeze_(dim=0)
    for idx in range(len(neck_kin_chain)):
        rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

    y_rot_angle = torch.round(torch.clamp(-rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi, max=39)).to(dtype=torch.long)
    neg_mask = y_rot_angle.lt(0).to(dtype=torch.long)
    mask = y_rot_angle.lt(-39).to(dtype=torch.long)
    neg_vals = mask * 78 + (1 - mask) * (39 - y_rot_angle)
    y_rot_angle = neg_mask * neg_vals + (1 - neg_mask) * y_rot_angle

    dyn_lmk_faces_idx = torch.index_select(dynamic_lmk_faces_idx, 0, y_rot_angle)
    dyn_lmk_b_coords = torch.index_select(dynamic_lmk_b_coords, 0, y_rot_angle)

    return dyn_lmk_faces_idx, dyn_lmk_b_coords


def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):
    """Calculates landmarks by barycentric interpolation

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
    """
    # Extract the indices of the vertices for each face
    # BxLx3
    batch_size, num_verts = vertices.shape[:2]
    device = vertices.device

    lmk_faces = torch.index_select(faces, 0, lmk_faces_idx.view(-1)).view(batch_size, -1, 3)

    lmk_faces += torch.arange(batch_size, dtype=torch.long, device=device).view(-1, 1, 1) * num_verts

    lmk_vertices = vertices.view(-1, 3)[lmk_faces].view(batch_size, -1, 3, 3)

    landmarks = torch.einsum("blfi,blf->bli", [lmk_vertices, lmk_bary_coords])
    return landmarks


def lbs(
    betas,
    pose,
    v_template,
    shapedirs,
    posedirs,
    J_regressor,
    parents,
    lbs_weights,
    pose2rot=True,
    dtype=torch.float32,
    detach_pose_correctives=True,
):
    """Performs Linear Blend Skinning with the given shape and pose parameters

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
    """

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
        rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs).view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1), posedirs).view(batch_size, -1, 3)

    if detach_pose_correctives:
        pose_offsets = pose_offsets.detach()

    v_posed = pose_offsets + v_shaped
    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)

    # 5. Do skinning:
    # W is N x V x (J + 1)
    W = lbs_weights.unsqueeze(dim=0).expand([batch_size, -1, -1])
    # (N x V x (J + 1)) x (N x (J + 1) x 16)
    num_joints = J_regressor.shape[0]
    T = torch.matmul(W, A.view(batch_size, num_joints, 16)).view(batch_size, -1, 4, 4)

    homogen_coord = torch.ones([batch_size, v_posed.shape[1], 1], dtype=dtype, device=device)
    v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
    v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

    verts = v_homo[:, :, :3, 0]

    return verts, J_transformed


def vertices2joints(J_regressor, vertices):
    """Calculates the 3D joint locations from the vertices

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
    """

    return torch.einsum("bik,ji->bjk", [vertices, J_regressor])


def blend_shapes(betas, shape_disps):
    """Calculates the per vertex displacement due to the blend shapes


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
    """

    # Displacement[b, m, k] = sum_{l} betas[b, l] * shape_disps[m, k, l]
    # i.e. Multiply each shape displacement by its corresponding beta and
    # then sum them.
    blend_shape = torch.einsum("bl,mkl->bmk", [betas, shape_disps])
    return blend_shape


def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):
    """Calculates the rotation matrices for a batch of rotation vectors
    Parameters
    ----------
    rot_vecs: torch.tensor Nx3
        array of N axis-angle vectors
    Returns
    -------
    R: torch.tensor Nx3x3
        The rotation matrices for the given axis-angle parameters
    """

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
    K = torch.cat([zeros, -rz, ry, rz, zeros, -rx, -ry, rx, zeros], dim=1).view((batch_size, 3, 3))

    ident = torch.eye(3, dtype=dtype, device=device).unsqueeze(dim=0)
    rot_mat = ident + sin * K + (1 - cos) * torch.bmm(K, K)
    return rot_mat


def transform_mat(R, t):
    """Creates a batch of transformation matrices
    Args:
        - R: Bx3x3 array of a batch of rotation matrices
        - t: Bx3x1 array of a batch of translation vectors
    Returns:
        - T: Bx4x4 Transformation matrix
    """
    # No padding left or right, only add an extra row
    return torch.cat([F.pad(R, [0, 0, 0, 1]), F.pad(t, [0, 0, 0, 1], value=1)], dim=2)


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
    transforms_mat = transform_mat(rot_mats.view(-1, 3, 3), rel_joints.reshape(-1, 3, 1)).reshape(
        -1, joints.shape[1], 4, 4
    )

    transform_chain = [transforms_mat[:, 0]]
    for i in range(1, parents.shape[0]):
        # Subtract the joint location at the rest pose
        # No need for rotation, since it's identity when at rest
        curr_res = torch.matmul(transform_chain[parents[i]], transforms_mat[:, i])
        transform_chain.append(curr_res)

    transforms = torch.stack(transform_chain, dim=1)

    # The last column of the transformations contains the posed joints
    posed_joints = transforms[:, :, :3, 3]

    # The last column of the transformations contains the posed joints
    posed_joints = transforms[:, :, :3, 3]

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])

    return posed_joints, rel_transforms


```

## Source file: artalk/flame_model/FLAME.py

{
  "path": "artalk/flame_model/FLAME.py",
  "blob_sha": "df26edc4e245201a9d32e7afa1f14ce405bb8989",
  "sha256": "366f5488fe89ce5caeece6059bf55b87bbcaca211107c1103ea294fe189ecc5d",
  "lines": 204,
  "bytes": 9533,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 22,
    "flame": 27,
    "lbs": 4,
    "rig": 4,
    "face": 19
  },
  "url": "https://github.com/xg-chu/ARTalk/blob/5571ab00382c8eec08c2743fe222cd549771dfc4/artalk/flame_model/FLAME.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def __init__(self, n_shape, n_exp, scale=1.0, no_lmks=False, lmks_type='lmks70', model_path=None):"
    },
    {
      "line": 68,
      "text": "def get_faces(self, ):"
    },
    {
      "line": 71,
      "text": "def _find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 117,
      "text": "def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, verts_sclae=None):"
    },
    {
      "line": 169,
      "text": "def _vertices2landmarks(self, vertices):"
    },
    {
      "line": 180,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 185,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 194,
      "text": "def reselect_eyes(vertices, lmks70):"
    }
  ]
}

```text

"""
Author: Soubhik Sanyal
Copyright (c) 2019, Soubhik Sanyal
All rights reserved.
Modified from smplx code for FLAME by Xuangeng Chu (xg.chu@outlook.com)
"""
import os

import torch
import pickle
import numpy as np
import torch.nn as nn

from .lbs import lbs, batch_rodrigues, vertices2landmarks

class FLAMEModel(nn.Module):
    """
    Given flame parameters this class generates a differentiable FLAME function
    which outputs the a mesh and 2D/3D facial landmarks
    """
    def __init__(self, n_shape, n_exp, scale=1.0, no_lmks=False, lmks_type='lmks70', model_path=None):
        super().__init__()
        self.scale = scale
        self.no_lmks, self.lmks_type = no_lmks, lmks_type
        # print("creating the FLAME Model")
        _abs_path = os.path.dirname(os.path.abspath(__file__))
        if model_path is None:
            model_path = os.path.join(_abs_path, '../../assets', 'FLAME_with_eye.pt')
        self.flame_ckpt = torch.load(model_path, map_location='cpu', weights_only=True)
        flame_model = self.flame_ckpt['flame_model']
        flame_lmk = self.flame_ckpt['lmk_embeddings']
        flame_dense_lmk = self.flame_ckpt['lmk_embeddings_mediapipe']
        
        self.dtype = torch.float32
        self.register_buffer('faces_tensor', flame_model['f'])
        self.register_buffer('v_template', flame_model['v_template'])
        shapedirs = flame_model['shapedirs']
        self.register_buffer('shapedirs', torch.cat([shapedirs[:, :, :n_shape], shapedirs[:, :, 300:300 + n_exp]], 2))
        num_pose_basis = flame_model['posedirs'].shape[-1]
        self.register_buffer('posedirs', flame_model['posedirs'].reshape(-1, num_pose_basis).T)
        self.register_buffer('J_regressor', flame_model['J_regressor'])
        parents = flame_model['kintree_table'][0]
        parents[0] = -1
        self.register_buffer('parents', parents)
        self.register_buffer('lbs_weights', flame_model['weights'])
        # Fixing Eyeball and neck rotation
        self.register_buffer('eye_pose', torch.zeros([1, 6], dtype=torch.float32))
        self.register_buffer('neck_pose', torch.zeros([1, 3], dtype=torch.float32))
        # Static and Dynamic Landmark embeddings for FLAME
        self.register_buffer('lmk_faces_idx', flame_lmk['static_lmk_faces_idx'])
        self.register_buffer('lmk_bary_coords', flame_lmk['static_lmk_bary_coords'].to(dtype=self.dtype))
        self.register_buffer('dynamic_lmk_faces_idx', flame_lmk['dynamic_lmk_faces_idx'].to(dtype=torch.long))
        self.register_buffer('dynamic_lmk_bary_coords', flame_lmk['dynamic_lmk_bary_coords'].to(dtype=self.dtype))
        self.register_buffer('full_lmk_faces_idx', flame_lmk['full_lmk_faces_idx_with_eye'].to(dtype=torch.long))
        self.register_buffer('full_lmk_bary_coords', flame_lmk['full_lmk_bary_coords_with_eye'].to(dtype=self.dtype))
        self.register_buffer('lmk_faces_idx_mediapipe', flame_dense_lmk['lmk_face_idx'].to(dtype=torch.long))
        self.register_buffer('lmk_bary_coords_mediapipe', flame_dense_lmk['lmk_b_coords'].to(dtype=self.dtype))

        neck_kin_chain = [];
        NECK_IDX = 1
        curr_idx = torch.tensor(NECK_IDX, dtype=torch.long)
        while curr_idx != -1:
            neck_kin_chain.append(curr_idx)
            curr_idx = self.parents[curr_idx]
        self.register_buffer('neck_kin_chain', torch.stack(neck_kin_chain))
        # print("FLAME Model Done.")

    def get_faces(self, ):
        return self.faces_tensor.long()

    def _find_dynamic_lmk_idx_and_bcoords(
            self, pose, dynamic_lmk_faces_idx, dynamic_lmk_b_coords,
            neck_kin_chain, dtype=torch.float32
        ):
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

        aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
                                     neck_kin_chain)
        rot_mats = batch_rodrigues(
            aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)

        rel_rot_mat = torch.eye(3, device=pose.device,
                                dtype=dtype).unsqueeze_(dim=0).expand(batch_size, -1, -1)
        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

        y_rot_angle = torch.round(
            torch.clamp(rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi, max=39)
        ).to(dtype=torch.long)

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

    # @torch.no_grad()
    def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, verts_sclae=None):
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
        if pose_params is None:
            pose_params = self.eye_pose.expand(batch_size, -1)
        if eye_pose_params is None:
            eye_pose_params = self.eye_pose.expand(batch_size, -1)
        if expression_params is None:
            expression_params = torch.zeros(batch_size, self.cfg.n_exp).to(shape_params.device)
        if pose_params.shape[-1] == 3:
            pose_params = torch.cat([torch.zeros(batch_size, 3).to(pose_params.device), pose_params], dim=-1)
        betas = torch.cat([shape_params, expression_params], dim=1)
        full_pose = torch.cat([
                pose_params[:, :3], self.neck_pose.expand(batch_size, -1), 
                pose_params[:, 3:], eye_pose_params
            ], dim=1
        )
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)
        vertices, _ = lbs(
            betas, full_pose, template_vertices,
            self.shapedirs, self.posedirs, self.J_regressor, self.parents,
            self.lbs_weights, dtype=self.dtype, detach_pose_correctives=False
        )
        if self.no_lmks:
            return vertices * self.scale
        if self.lmks_type == 'lmks70':
            landmarks3d = vertices2landmarks(
                vertices, self.faces_tensor, 
                self.full_lmk_faces_idx.repeat(vertices.shape[0], 1),
                self.full_lmk_bary_coords.repeat(vertices.shape[0], 1, 1)
            )
            landmark_3d = reselect_eyes(vertices, landmarks3d)
        elif self.lmks_type == 'dense105':
            landmarks3d = vertices2landmarks(
                vertices, self.faces_tensor,
                self.lmk_faces_idx_mediapipe.unsqueeze(dim=0).expand(batch_size, -1).contiguous(),
                self.lmk_bary_coords_mediapipe.unsqueeze(dim=0).expand(batch_size, -1, -1).contiguous()
            )
        else:
            raise ValueError(f"Unknown lmks_type: {self.lmks_type}.")
        if verts_sclae is not None:
            return vertices * verts_sclae, landmark_3d * verts_sclae
        return vertices * self.scale, landmarks3d * self.scale

    def _vertices2landmarks(self, vertices):
        landmarks3d = vertices2landmarks(
            vertices, self.faces_tensor, 
            self.full_lmk_faces_idx.repeat(vertices.shape[0], 1),
            self.full_lmk_bary_coords.repeat(vertices.shape[0], 1, 1)
        )
        landmark_3d = reselect_eyes(vertices, landmarks3d)
        return landmark_3d


class Struct(object):
    def __init__(self, **kwargs):
        for key, val in kwargs.items():
            setattr(self, key, val)


def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] +
                    rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


def reselect_eyes(vertices, lmks70):
    lmks70 = lmks70.clone()
    eye_in_shape = [2422,2422, 2452, 2454, 2471, 3638, 2276, 2360, 3835, 1292, 1217, 1146, 1146, 999, 827, ]
    eye_in_shape_reduce = [0,2,4,5,6,7,8,9,10,11,13,14]
    cur_eye = vertices[:, eye_in_shape]
    cur_eye[:, 0] = (cur_eye[:, 0] + cur_eye[:, 1]) * 0.5
    cur_eye[:, 2] = (cur_eye[:, 2] + cur_eye[:, 3]) * 0.5
    cur_eye[:, 11] = (cur_eye[:, 11] + cur_eye[:, 12]) * 0.5
    cur_eye = cur_eye[:, eye_in_shape_reduce]
    lmks70[:, [37,38,40,41,43,44,46,47]] = cur_eye[:, [1,2,4,5,7,8,10,11]]
    return lmks70


```

## Source file: train_code/core/libs/flame_model/FLAME.py

{
  "path": "train_code/core/libs/flame_model/FLAME.py",
  "blob_sha": "4ce60a38f3573a038c2c3c804279e9a269cd442b",
  "sha256": "a9ba199e1b83d132bafb4e886c35f55ef89f0a9cce9bbf6a55700b4771741a5c",
  "lines": 233,
  "bytes": 11119,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 32,
    "jaw": 6,
    "flame": 26,
    "lbs": 4,
    "rig": 4,
    "face": 16
  },
  "url": "https://github.com/xg-chu/ARTalk/blob/5571ab00382c8eec08c2743fe222cd549771dfc4/train_code/core/libs/flame_model/FLAME.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self, n_shape, n_exp, scale=1.0, no_lmks=False, lmks_type='lmks70'):"
    },
    {
      "line": 72,
      "text": "def get_faces(self, ):"
    },
    {
      "line": 75,
      "text": "def get_colors(self, ):"
    },
    {
      "line": 78,
      "text": "def _find_dynamic_lmk_idx_and_bcoords("
    },
    {
      "line": 109,
      "text": "def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, verts_sclae=None):"
    },
    {
      "line": 161,
      "text": "def _vertices2landmarks(self, vertices):"
    },
    {
      "line": 170,
      "text": "def get_flame_verts(self, motion_code, shape_code=None, with_headpose=True):"
    },
    {
      "line": 209,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 214,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 223,
      "text": "def reselect_eyes(vertices, lmks70):"
    }
  ]
}

```text

"""
Author: Soubhik Sanyal
Copyright (c) 2019, Soubhik Sanyal
All rights reserved.
Modified from smplx code for FLAME by Xuangeng Chu (xg.chu@outlook.com)
"""
import os

import torch
import numpy as np
import torch.nn as nn

from .lbs import lbs, batch_rodrigues, vertices2landmarks

class FLAMEModel(nn.Module):
    """
    Given flame parameters this class generates a differentiable FLAME function
    which outputs the a mesh and 2D/3D facial landmarks
    """
    def __init__(self, n_shape, n_exp, scale=1.0, no_lmks=False, lmks_type='lmks70'):
        super().__init__()
        self.scale = scale
        self.no_lmks, self.lmks_type = no_lmks, lmks_type
        self.n_shape, self.n_exp = n_shape, n_exp
        _abs_path = os.path.dirname(os.path.abspath(__file__))
        self.flame_ckpt = torch.load(
            os.path.join(_abs_path, '../../../assets', 'FLAME_with_eye.pt'), map_location='cpu', weights_only=True
        )
        flame_model = self.flame_ckpt['flame_model']
        flame_lmk = self.flame_ckpt['lmk_embeddings']
        flame_dense_lmk = self.flame_ckpt['lmk_embeddings_mediapipe']

        self.dtype = torch.float32
        self.register_buffer('faces_tensor', flame_model['f'])
        self.register_buffer('v_template', flame_model['v_template'])
        shapedirs = flame_model['shapedirs']
        self.register_buffer('shapedirs', torch.cat([shapedirs[:, :, :n_shape], shapedirs[:, :, 300:300 + n_exp]], 2))
        num_pose_basis = flame_model['posedirs'].shape[-1]
        self.register_buffer('posedirs', flame_model['posedirs'].reshape(-1, num_pose_basis).T)
        self.register_buffer('J_regressor', flame_model['J_regressor'])
        parents = flame_model['kintree_table'][0]
        parents[0] = -1
        self.register_buffer('parents', parents)
        self.register_buffer('lbs_weights', flame_model['weights'])
        # Fixing Eyeball and neck rotation
        self.register_buffer('eye_pose', torch.zeros([1, 6], dtype=torch.float32))
        self.register_buffer('neck_pose', torch.zeros([1, 3], dtype=torch.float32))
        # Static and Dynamic Landmark embeddings for FLAME
        self.register_buffer('lmk_faces_idx', flame_lmk['static_lmk_faces_idx'])
        self.register_buffer('lmk_bary_coords', flame_lmk['static_lmk_bary_coords'].to(dtype=self.dtype))
        self.register_buffer('dynamic_lmk_faces_idx', flame_lmk['dynamic_lmk_faces_idx'].to(dtype=torch.long))
        self.register_buffer('dynamic_lmk_bary_coords', flame_lmk['dynamic_lmk_bary_coords'].to(dtype=self.dtype))
        self.register_buffer('full_lmk_faces_idx', flame_lmk['full_lmk_faces_idx_with_eye'].to(dtype=torch.long))
        self.register_buffer('full_lmk_bary_coords', flame_lmk['full_lmk_bary_coords_with_eye'].to(dtype=self.dtype))
        self.register_buffer('lmk_faces_idx_mediapipe', flame_dense_lmk['lmk_face_idx'].to(dtype=torch.long))
        self.register_buffer('lmk_bary_coords_mediapipe', flame_dense_lmk['lmk_b_coords'].to(dtype=self.dtype))

        neck_kin_chain = [];
        NECK_IDX = 1
        curr_idx = torch.tensor(NECK_IDX, dtype=torch.long)
        while curr_idx != -1:
            neck_kin_chain.append(curr_idx)
            curr_idx = self.parents[curr_idx]
        self.register_buffer('neck_kin_chain', torch.stack(neck_kin_chain))

        # eye vertex coloring for visualization
        eye_vertices = list(range(4477, 4594, 4)) + [4598, 4602, 4597] + list(range(3931, 4048, 4)) + [4052, 4056, 4051]
        verts_rgb = torch.ones_like(self.v_template) * torch.tensor([142, 179, 247])[None, None]
        verts_rgb[:, eye_vertices, :] = torch.tensor([21, 60, 122])[None, None].float()
        self.register_buffer("verts_rgb", verts_rgb[0])

    def get_faces(self, ):
        return self.faces_tensor.long()

    def get_colors(self, ):
        return self.verts_rgb

    def _find_dynamic_lmk_idx_and_bcoords(
            self, pose, dynamic_lmk_faces_idx, dynamic_lmk_b_coords,
            neck_kin_chain, dtype=torch.float32
        ):
        batch_size = pose.shape[0]
        aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
                                     neck_kin_chain)
        rot_mats = batch_rodrigues(
            aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)

        rel_rot_mat = torch.eye(3, device=pose.device,
                                dtype=dtype).unsqueeze_(dim=0).expand(batch_size, -1, -1)
        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

        y_rot_angle = torch.round(
            torch.clamp(rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi, max=39)
        ).to(dtype=torch.long)

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

    def forward(self, shape_params=None, expression_params=None, pose_params=None, eye_pose_params=None, verts_sclae=None):
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
        if pose_params is None:
            pose_params = self.eye_pose.expand(batch_size, -1)
        if eye_pose_params is None:
            eye_pose_params = self.eye_pose.expand(batch_size, -1)
        if expression_params is None:
            expression_params = torch.zeros(batch_size, self.cfg.n_exp).to(shape_params.device)
        if pose_params.shape[-1] == 3:
            pose_params = torch.cat([torch.zeros(batch_size, 3).to(pose_params.device), pose_params], dim=-1)
        betas = torch.cat([shape_params, expression_params], dim=1)
        full_pose = torch.cat([
                pose_params[:, :3], self.neck_pose.expand(batch_size, -1),
                pose_params[:, 3:], eye_pose_params
            ], dim=1
        )
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)
        vertices, _ = lbs(
            betas, full_pose, template_vertices,
            self.shapedirs, self.posedirs, self.J_regressor, self.parents,
            self.lbs_weights, dtype=self.dtype, detach_pose_correctives=False
        )
        if self.no_lmks:
            return vertices * self.scale
        if self.lmks_type == 'lmks70':
            landmarks3d = vertices2landmarks(
                vertices, self.faces_tensor,
                self.full_lmk_faces_idx.repeat(vertices.shape[0], 1),
                self.full_lmk_bary_coords.repeat(vertices.shape[0], 1, 1)
            )
            landmark_3d = reselect_eyes(vertices, landmarks3d)
        elif self.lmks_type == 'dense105':
            landmarks3d = vertices2landmarks(
                vertices, self.faces_tensor,
                self.lmk_faces_idx_mediapipe.unsqueeze(dim=0).expand(batch_size, -1).contiguous(),
                self.lmk_bary_coords_mediapipe.unsqueeze(dim=0).expand(batch_size, -1, -1).contiguous()
            )
        else:
            raise ValueError(f"Unknown lmks_type: {self.lmks_type}.")
        if verts_sclae is not None:
            return vertices * verts_sclae, landmark_3d * verts_sclae
        return vertices * self.scale, landmarks3d * self.scale

    def _vertices2landmarks(self, vertices):
        landmarks3d = vertices2landmarks(
            vertices, self.faces_tensor,
            self.full_lmk_faces_idx.repeat(vertices.shape[0], 1),
            self.full_lmk_bary_coords.repeat(vertices.shape[0], 1, 1)
        )
        landmark_3d = reselect_eyes(vertices, landmarks3d)
        return landmark_3d

    def get_flame_verts(self, motion_code, shape_code=None, with_headpose=True):
        """Helper method for training pipeline: motion_code [B, L, 108] -> vertices [B, L, V, 3]"""
        assert motion_code.dim() == 3
        assert motion_code.shape[-1] == 108
        batch_size, motion_length, _ = motion_code.shape
        exp_code, gpose_code, jaw_code, eyepose_code = motion_code.split([100, 3, 1, 4], dim=-1)
        if not with_headpose:
            gpose_code = torch.zeros_like(gpose_code)
        if shape_code is not None:
            shape_code = shape_code.repeat(1, motion_length, 1)
        else:
            shape_code = [None] * batch_size
        # build pose_params from gpose + jaw
        if jaw_code.shape[-1] == 1:
            jaw_code_3 = torch.cat([jaw_code, jaw_code.new_zeros(*jaw_code.shape[:-1], 2)], dim=-1)
        else:
            jaw_code_3 = jaw_code
        # build eye_pose_params from eyepose_code (4 -> 6)
        if eyepose_code.shape[-1] == 4:
            eye_zeros = eyepose_code.new_zeros(*eyepose_code.shape[:-1], 1)
            eyepose_6 = torch.cat([eyepose_code[..., :2], eye_zeros, eyepose_code[..., 2:], eye_zeros], dim=-1)
        else:
            eyepose_6 = eyepose_code
        verts = []
        for bidx in range(batch_size):
            shape = shape_code[bidx] if shape_code[bidx] is not None else motion_code.new_zeros(motion_length, self.n_shape)
            pose_params = torch.cat([gpose_code[bidx], jaw_code_3[bidx]], dim=-1)  # [L, 6]
            this_verts, _ = self.forward(
                shape_params=shape,
                expression_params=exp_code[bidx],
                pose_params=pose_params,
                eye_pose_params=eyepose_6[bidx],
            )
            verts.append(this_verts)
        verts = torch.stack(verts, dim=0)
        return verts


class Struct(object):
    def __init__(self, **kwargs):
        for key, val in kwargs.items():
            setattr(self, key, val)


def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] +
                    rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


def reselect_eyes(vertices, lmks70):
    lmks70 = lmks70.clone()
    eye_in_shape = [2422,2422, 2452, 2454, 2471, 3638, 2276, 2360, 3835, 1292, 1217, 1146, 1146, 999, 827, ]
    eye_in_shape_reduce = [0,2,4,5,6,7,8,9,10,11,13,14]
    cur_eye = vertices[:, eye_in_shape]
    cur_eye[:, 0] = (cur_eye[:, 0] + cur_eye[:, 1]) * 0.5
    cur_eye[:, 2] = (cur_eye[:, 2] + cur_eye[:, 3]) * 0.5
    cur_eye[:, 11] = (cur_eye[:, 11] + cur_eye[:, 12]) * 0.5
    cur_eye = cur_eye[:, eye_in_shape_reduce]
    lmks70[:, [37,38,40,41,43,44,46,47]] = cur_eye[:, [1,2,4,5,7,8,10,11]]
    return lmks70


```

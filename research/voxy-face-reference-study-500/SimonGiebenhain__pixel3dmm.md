# Facial rendering source scan: SimonGiebenhain/pixel3dmm

Repository: https://github.com/SimonGiebenhain/pixel3dmm

Pinned source revision: fcd1fa973c7715b02a8948dfc679dff53cf85924

Discovery topic: face reconstruction 3D

Description: [Official Code] Pixel3DMM: Versatile Screen-Space Priors for Single-Image 3D Face Reconstruction

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["assets/uv_mask_eyes.png", "assets/uv_valid_verty_noEyes.npy", "assets/uv_valid_verty_noEyes_debug.npy", "assets/uv_valid_verty_noEyes_noEyeRegion_debug_wEars.npy", "src/pixel3dmm/tracking/flame/blendshapes/l_eyelid.npy", "src/pixel3dmm/tracking/flame/blendshapes/r_eyelid.npy"], "skin": ["src/pixel3dmm/utils/masking.py"], "blendshape": ["src/pixel3dmm/tracking/flame/blendshapes/l_eyelid.npy", "src/pixel3dmm/tracking/flame/blendshapes/r_eyelid.npy"], "flame": ["assets/flame_mirror_index.npy", "assets/flame_uv_coords.npy", "assets/flame_vertex_weights.npy", "download_flame2023.sh", "src/pixel3dmm/preprocessing/replacement_code/install_mica_download_flame.sh", "src/pixel3dmm/tracking/flame/FLAME.py", "src/pixel3dmm/tracking/flame/blendshapes/l_eyelid.npy", "src/pixel3dmm/tracking/flame/blendshapes/r_eyelid.npy", "src/pixel3dmm/tracking/flame/lbs.py"], "lbs": ["src/pixel3dmm/tracking/flame/lbs.py"], "rig": ["assets/test_rigid.ply"], "face": ["scripts/run_facer_segmentation.py", "src/pixel3dmm/preprocessing/replacement_code/facer_transform.py"]}

## Source file: src/pixel3dmm/tracking/flame/lbs.py

{
  "path": "src/pixel3dmm/tracking/flame/lbs.py",
  "blob_sha": "4172f3b0989fa2afb63355518181a053b4e268b7",
  "sha256": "c1f8c1f7b5caec8f4d210ac59d43e251b85d654c9b8142f640a60359dd64df35",
  "lines": 411,
  "bytes": 15055,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 13,
    "face": 19
  },
  "url": "https://github.com/SimonGiebenhain/pixel3dmm/blob/fcd1fa973c7715b02a8948dfc679dff53cf85924/src/pixel3dmm/tracking/flame/lbs.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def stereographic_unproject_old(a):"
    },
    {
      "line": 40,
      "text": "def stereographic_unproject(a, axis=None):"
    },
    {
      "line": 57,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 66,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 131,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 170,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 266,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 286,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 310,
      "text": "def _batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 344,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 357,
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
# Copyright©2023 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: mica@tue.mpg.de

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function

import numpy as np
import torch
import torch.nn.functional as F


# in batch*5
# out batch*6
from pixel3dmm.utils.utils_3d import rotation_6d_to_matrix

def stereographic_unproject_old(a):
    s2 = torch.pow(a, 2).sum(1)  # batch
    unproj = 2 * a / (s2 + 1).view(-1, 1).repeat(1, 5)  # batch*5
    w = (s2 - 1) / (s2 + 1)  # batch
    out = torch.cat((unproj, w.view(-1, 1)), 1)  # batch*6

    return out


# in a batch*5, axis int
def stereographic_unproject(a, axis=None):
    """
	Inverse of stereographic projection: increases dimension by one.
	"""
    batch = a.shape[0]
    if axis is None:
        axis = a.shape[1]
    s2 = torch.pow(a, 2).sum(1)  # batch
    ans = torch.autograd.Variable(torch.zeros(batch, a.shape[1] + 1).cuda())  # batch*6
    unproj = 2 * a / (s2 + 1).view(batch, 1).repeat(1, a.shape[1])  # batch*5
    if (axis > 0):
        ans[:, :axis] = unproj[:, :axis]  # batch*(axis-0)
    ans[:, axis] = (s2 - 1) / (s2 + 1)  # batch
    ans[:, axis + 1:] = unproj[:, axis:]  # batch*(5-axis)		# Note that this is a no-op if the default option (last axis) is used
    return ans


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

    aa_pose = torch.index_select(pose.view(batch_size, -1, 6), 1, neck_kin_chain)
    rot_mats = rotation_6d_to_matrix(aa_pose.view(-1, 6)).view(batch_size, -1, 3, 3)

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
        # rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])
        rot_mats = rotation_6d_to_matrix(pose.view(-1, 6)).view([batch_size, -1, 3, 3])

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs) \
            .view(batch_size, -1, 3)
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

    #import pyvista as pv
    #pl = pv.Plotter()
    #pl.add_points(v_posed.detach().cpu().squeeze().numpy())
    #pl.add_point_labels(points=J_transformed.detach().cpu().squeeze().numpy(), labels=[str(i) for i in range(5)])
    #pl.add_point_labels(points=J.detach().cpu().squeeze().numpy(), labels=[str(i) for i in range(5)], text_color='green')
    #pl.show()

    return verts, A, v_shaped


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


def _batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):
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

## Source file: src/pixel3dmm/utils/masking.py

{
  "path": "src/pixel3dmm/utils/masking.py",
  "blob_sha": "588032fa1e670799ea8df88c906826423a3db089",
  "sha256": "7ebe8fc19830bdcdaed0f7b8c97a603f16727e49ff1631c1289d03062047eda7",
  "lines": 211,
  "bytes": 7090,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 21,
    "skin": 2,
    "flame": 6,
    "rig": 16,
    "face": 47,
    "lip": 2
  },
  "url": "https://github.com/SimonGiebenhain/pixel3dmm/blob/fcd1fa973c7715b02a8948dfc679dff53cf85924/src/pixel3dmm/utils/masking.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 34,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 41,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 47,
      "text": "def __init__(self):"
    },
    {
      "line": 67,
      "text": "def to_render_mask(self, mask):"
    },
    {
      "line": 72,
      "text": "def get_faces(self):"
    },
    {
      "line": 75,
      "text": "def get_color_mask(self):"
    },
    {
      "line": 78,
      "text": "def get_mask_face(self):"
    },
    {
      "line": 81,
      "text": "def get_mask_lips(self):"
    },
    {
      "line": 84,
      "text": "def get_mask_rendering(self):"
    },
    {
      "line": 96,
      "text": "def get_mask_depth(self):"
    },
    {
      "line": 103,
      "text": "def get_mask_eyes(self):"
    },
    {
      "line": 109,
      "text": "def get_mask_eyes_rendering(self):"
    },
    {
      "line": 115,
      "text": "def get_mask_eyes_region(self):"
    },
    {
      "line": 122,
      "text": "def get_mask_eyes_region_rendering(self):"
    },
    {
      "line": 132,
      "text": "def get_mask_ears(self):"
    },
    {
      "line": 138,
      "text": "def get_triangle_face_mask(self):"
    },
    {
      "line": 142,
      "text": "def get_triangle_color_face_mask(self):"
    },
    {
      "line": 146,
      "text": "def get_triangle_eyes_mask(self):"
    },
    {
      "line": 150,
      "text": "def get_triangle_whole_mask(self):"
    },
    {
      "line": 154,
      "text": "def get_triangle_mask(self, m):"
    },
    {
      "line": 168,
      "text": "def get_binary_triangle_mask(self):"
    },
    {
      "line": 181,
      "text": "def get_masked_faces(self):"
    },
    {
      "line": 191,
      "text": "def get_masked_mesh(self, vertices, triangle_mask):"
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
# Copyright©2023 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: mica@tue.mpg.de

import os
import pickle

import numpy as np
import torch
import torch.nn as nn
import trimesh
from trimesh import Trimesh

from pixel3dmm import env_paths


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


class Masking(nn.Module):
    def __init__(self):

        dir = f'{env_paths.FLAME_ASSETS}'
        super(Masking, self).__init__()
        with open(f'{dir}/FLAME2020/FLAME_masks/FLAME_masks.pkl', 'rb') as f:
            ss = pickle.load(f, encoding='latin1')
            self.masks = Struct(**ss)

        with open(f'{dir}/FLAME2020/generic_model.pkl', 'rb') as f:
            ss = pickle.load(f, encoding='latin1')
            flame_model = Struct(**ss)

        self.color_mesh = trimesh.load(f'{env_paths.head_template_color}', process=False)
        self.color_mask = (np.array(self.color_mesh.visual.vertex_colors[:, 0:3]) == [255, 0, 0])[:, 0].nonzero()[0]
        self.color_mask = np.array([i for i in self.color_mask if i not in self.get_mask_eyes()])

        self.dtype = torch.float32
        self.register_buffer('faces', to_tensor(to_np(flame_model.f, dtype=np.int64), dtype=torch.long))
        self.register_buffer('vertices', to_tensor(to_np(flame_model.v_template), dtype=self.dtype))

    def to_render_mask(self, mask):
        face_mask = torch.zeros_like(self.vertices)[None]
        face_mask[:, mask, :] = 1.0
        return face_mask

    def get_faces(self):
        return self.faces

    def get_color_mask(self):
        return self.color_mask

    def get_mask_face(self):
        return self.masks.face

    def get_mask_lips(self):
        return self.masks.lips

    def get_mask_rendering(self):
        face_mask = torch.zeros_like(self.vertices)[None]
        face_mask[:, self.masks.face, :] = 1.0
        face_mask[:, self.masks.left_eyeball, :] = 1.0
        face_mask[:, self.masks.right_eyeball, :] = 1.0

        # face_mask = torch.ones_like(self.vertices)[None]
        # face_mask[:, self.masks.boundary, :] = 0.0
        # face_mask[:, self.masks.left_ear, :] = 0.0
        # face_mask[:, self.masks.right_ear, :] = 0.0
        return face_mask

    def get_mask_depth(self):
        face_mask = torch.ones_like(self.vertices)[None]
        face_mask[:, self.masks.boundary, :] = 0.0
        face_mask[:, self.masks.left_ear, :] = 0.0
        face_mask[:, self.masks.right_ear, :] = 0.0
        return face_mask

    def get_mask_eyes(self):
        left = self.masks.left_eyeball
        right = self.masks.right_eyeball

        return np.unique(np.concatenate((left, right)))

    def get_mask_eyes_rendering(self):
        eyes_mask = torch.zeros_like(self.vertices)[None]
        eyes_mask[:, self.get_mask_eyes(), :] = 1.0

        return eyes_mask

    def get_mask_eyes_region(self):
        left = self.masks.left_eye_region
        right = self.masks.right_eye_region
        mask = np.unique(np.concatenate((left, right)))

        return mask

    def get_mask_eyes_region_rendering(self):
        left = self.masks.left_eye_region
        right = self.masks.right_eye_region

        mask = np.unique(np.concatenate((left, right)))
        eyes_mask = torch.zeros_like(self.vertices)[None]
        eyes_mask[:, mask, :] = 1.0

        return eyes_mask

    def get_mask_ears(self):
        left = self.masks.left_ear
        right = self.masks.right_ear

        return np.unique(np.concatenate((left, right)))

    def get_triangle_face_mask(self):
        m = self.color_mask
        return self.get_triangle_mask(m)

    def get_triangle_color_face_mask(self):
        m = self.masks.face
        return self.get_triangle_mask(m)

    def get_triangle_eyes_mask(self):
        m = self.get_mask_eyes()
        return self.get_triangle_mask(m)

    def get_triangle_whole_mask(self):
        m = self.get_whole_mask()
        return self.get_triangle_mask(m)

    def get_triangle_mask(self, m):
        f = self.faces.cpu().numpy()
        selected = []
        for i in range(f.shape[0]):
            l = f[i]
            valid = 0
            for j in range(3):
                if l[j] in m:
                    valid += 1
            if valid == 3:
                selected.append(i)

        return np.unique(selected)

    def get_binary_triangle_mask(self):
        mask = self.get_whole_mask()
        faces = self.faces.cpu().numpy()
        reduced_faces = []
        for f in faces:
            valid = 0
            for v in f:
                if v in mask:
                    valid += 1
            reduced_faces.append(True if valid == 3 else False)

        return reduced_faces

    def get_masked_faces(self):
        if self.masked_faces is None:
            faces = self.faces.cpu().numpy()
            vertices = self.vertices.cpu().numpy()
            m = Trimesh(vertices=vertices, faces=faces, process=False)
            m.update_faces(self.get_binary_triangle_mask())
            self.masked_faces = torch.from_numpy(np.array(m.faces)).cuda().long()[None]

        return self.masked_faces

    def get_masked_mesh(self, vertices, triangle_mask):
        if len(vertices.shape) == 2:
            vertices = vertices[None]
        B, N, V = vertices.shape
        faces = self.faces.cpu().numpy()
        masked_vertices = torch.empty(0, 0, 3).cuda()
        masked_faces = torch.empty(0, 0, 3).cuda()
        for i in range(B):
            m = Trimesh(vertices=vertices[i].detach().cpu().numpy(), faces=faces, process=False)
            m.update_faces(triangle_mask)
            m.process()
            f = torch.from_numpy(np.array(m.faces)).cuda()[None]
            v = torch.from_numpy(np.array(m.vertices)).cuda()[None].float()
            if masked_vertices.shape[1] != v.shape[1]:
                masked_vertices = torch.empty(0, v.shape[1], 3).cuda()
            if masked_faces.shape[1] != f.shape[1]:
                masked_faces = torch.empty(0, f.shape[1], 3).cuda()
            masked_vertices = torch.cat([masked_vertices, v])
            masked_faces = torch.cat([masked_faces, f])

        return masked_vertices, masked_faces


```

## Source file: scripts/run_facer_segmentation.py

{
  "path": "scripts/run_facer_segmentation.py",
  "blob_sha": "076524e51045c0a22b79b17d00a1db715b625c70",
  "sha256": "b10ffcd2e1cc0209e75304985cea890b7ad1fde76347563c89421ceb96cf4026",
  "lines": 221,
  "bytes": 6306,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 6,
    "skin": 3,
    "mouth": 3,
    "rig": 2,
    "face": 9,
    "lip": 6
  },
  "url": "https://github.com/SimonGiebenhain/pixel3dmm/blob/fcd1fa973c7715b02a8948dfc679dff53cf85924/scripts/run_facer_segmentation.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def viz_results(img, seq_classes, n_classes, suppress_plot = False):"
    },
    {
      "line": 57,
      "text": "def get_color_seg(img, seq_classes, n_classes):"
    },
    {
      "line": 92,
      "text": "def crop_gt_img(img, seq_classes, n_classes):"
    },
    {
      "line": 137,
      "text": "def main(video_name : str):"
    }
  ]
}

```text

import os
import sys
import traceback

from math import ceil

import PIL.Image
import torch
import distinctipy
import matplotlib.pyplot as plt
from PIL import Image
import numpy as np
import facer
import tyro

from pixel3dmm import env_paths

colors = distinctipy.get_colors(22, rng=0)


def viz_results(img, seq_classes, n_classes, suppress_plot = False):

    seg_img = np.zeros([img.shape[-2], img.shape[-1], 3])
    #distinctipy.color_swatch(colors)
    bad_indices = [
        0,  # background,
        1,  # neck
        # 2, skin
        3,  # cloth
        4,  # ear_r (images-space r)
        5,  # ear_l
        # 6 brow_r
        # 7 brow_l
        # 8,  # eye_r
        # 9,  # eye_l
        # 10 noise
        # 11 mouth
        # 12 lower_lip
        # 13 upper_lip
        14,  # hair,
        # 15, glasses
        16,  # ??
        17,  # earring_r
        18,  # ?
    ]
    bad_indices = []

    for i in range(n_classes):
        if i not in bad_indices:
            seg_img[seq_classes[0, :, :] == i] = np.array(colors[i])*255

    if not suppress_plot:
        plt.imshow(seg_img.astype(np.uint(8)))
        plt.show()
    return Image.fromarray(seg_img.astype(np.uint8))

def get_color_seg(img, seq_classes, n_classes):

    seg_img = np.zeros([img.shape[-2], img.shape[-1], 3])
    colors = distinctipy.get_colors(n_classes+1, rng=0)
    #distinctipy.color_swatch(colors)
    bad_indices = [
        0,  # background,
        1,  # neck
        # 2, skin
        3,  # cloth
        4,  # ear_r (images-space r)
        5,  # ear_l
        # 6 brow_r
        # 7 brow_l
        # 8,  # eye_r
        # 9,  # eye_l
        # 10 noise
        # 11 mouth
        # 12 lower_lip
        # 13 upper_lip
        14,  # hair,
        # 15, glasses
        16,  # ??
        17,  # earring_r
        18,  # ?
    ]

    for i in range(n_classes):
        if i not in bad_indices:
            seg_img[seq_classes[0, :, :] == i] = np.array(colors[i])*255


    return Image.fromarray(seg_img.astype(np.uint8))


def crop_gt_img(img, seq_classes, n_classes):

    seg_img = np.zeros([img.shape[-2], img.shape[-1], 3])
    colors = distinctipy.get_colors(n_classes+1, rng=0)
    #distinctipy.color_swatch(colors)
    bad_indices = [
        0,  # background,
        1,  # neck
        # 2, skin
        3,  # cloth
        4, #ear_r (images-space r)
        5, #ear_l
        # 6 brow_r
        # 7 brow_l
        #8,  # eye_r
        #9,  # eye_l
        # 10 noise
        # 11 mouth
        # 12 lower_lip
        # 13 upper_lip
        14,  # hair,
        # 15, glasses
        16,  # ??
        17,  # earring_r
        18,  # ?
    ]

    for i in range(n_classes):
        if i in bad_indices:
            img[seq_classes[0, :, :] == i] = 0


    #plt.imshow(img.astype(np.uint(8)))
    #plt.show()
    return img.astype(np.uint8)


device = 'cuda' if torch.cuda.is_available() else 'cpu'



face_detector = facer.face_detector('retinaface/mobilenet', device=device)
face_parser = facer.face_parser('farl/celebm/448', device=device)  # optional "farl/lapa/448"


def main(video_name : str):


    out = f'{env_paths.PREPROCESSED_DATA}/{video_name}'
    out_seg = f'{out}/seg_og/'
    out_seg_annot = f'{out}/seg_non_crop_annotations/'
    os.makedirs(out_seg, exist_ok=True)
    os.makedirs(out_seg_annot, exist_ok=True)
    folder = f'{out}/cropped/'  # '/home/giebenhain/GTA/data_kinect/color/'





    frames = [f for f in os.listdir(folder) if f.endswith('.png') or f.endswith('.jpg')]

    frames.sort()

    if len(os.listdir(out_seg)) == len(frames):
        print(f'''
                        <<<<<<<< ALREADY COMPLETED SEGMENTATION FOR {video_name}, SKIPPING >>>>>>>>
                        ''')
        return

    #for file in frames:
    batch_size = 1

    for i in range(len(frames)//batch_size):
        image_stack = []
        frame_stack = []
        original_shapes = []
        for j in range(batch_size):
            file = frames[i * batch_size + j]

            if os.path.exists(f'{out_seg_annot}/color_{file}.png'):
                print('DONE')
                continue
            img = Image.open(f'{folder}/{file}')#.resize((512, 512))

            og_size = img.size

            image = facer.hwc2bchw(torch.from_numpy(np.array(img)[..., :3])).to(device=device)  # image: 1 x 3 x h x w
            image_stack.append(image)
            frame_stack.append(file[:-4])

        for batch_idx in range(ceil(len(image_stack)/batch_size)):
            image_batch = torch.cat(image_stack[batch_idx*batch_size:(batch_idx+1)*batch_size], dim=0)
            frame_idx_batch = frame_stack[batch_idx*batch_size:(batch_idx+1)*batch_size]
            og_shape_batch = original_shapes[batch_idx*batch_size:(batch_idx+1)*batch_size]

            #if True:
            try:
                with torch.inference_mode():
                    faces = face_detector(image_batch)
                    torch.cuda.empty_cache()
                    faces = face_parser(image_batch, faces, bbox_scale_factor=1.25)
                    torch.cuda.empty_cache()

                seg_logits = faces['seg']['logits']
                back_ground = torch.all(seg_logits == 0, dim=1, keepdim=True).detach().squeeze(1).cpu().numpy()
                seg_probs = seg_logits.softmax(dim=1)  # nfaces x nclasses x h x w
                seg_classes = seg_probs.argmax(dim=1).detach().cpu().numpy().astype(np.uint8)
                seg_classes[back_ground] = seg_probs.shape[1] + 1


                for _iidx in range(seg_probs.shape[0]):
                    frame = frame_idx_batch[_iidx]
                    iidx = faces['image_ids'][_iidx].item()
                    try:
                        I_color = viz_results(image_batch[iidx:iidx+1], seq_classes=seg_classes[_iidx:_iidx+1], n_classes=seg_probs.shape[1] + 1, suppress_plot=True)
                        I_color.save(f'{out_seg_annot}/color_{frame}.png')
                    except Exception as ex:
                        pass
                    I = Image.fromarray(seg_classes[_iidx])
                    I.save(f'{out_seg}/{frame}.png')
                torch.cuda.empty_cache()
            except Exception as exx:
                traceback.print_exc()
                continue


if __name__ == '__main__':

    tyro.cli(main)



```

## Source file: src/pixel3dmm/tracking/flame/FLAME.py

{
  "path": "src/pixel3dmm/tracking/flame/FLAME.py",
  "blob_sha": "1427013152ba4cd9e759bcecafbabfae7522467d",
  "sha256": "65ae6b016341406942dfffb58282da30a969c2fec06bdb373c7d82dc70096822",
  "lines": 354,
  "bytes": 16558,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 18,
    "skin": 1,
    "blendshape": 2,
    "jaw": 11,
    "flame": 29,
    "deform": 1,
    "lbs": 6,
    "rig": 7,
    "face": 25,
    "lip": 1
  },
  "url": "https://github.com/SimonGiebenhain/pixel3dmm/blob/fcd1fa973c7715b02a8948dfc679dff53cf85924/src/pixel3dmm/tracking/flame/FLAME.py",
  "function_signature_candidates": [
    {
      "line": 34,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 39,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 46,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 51,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 67,
      "text": "def __init__(self, config):"
    },
    {
      "line": 127,
      "text": "def _find_dynamic_lmk_idx_and_bcoords(self, vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 167,
      "text": "def _vertices2landmarks(self, vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 197,
      "text": "def forward(self, shape_params,"
    },
    {
      "line": 298,
      "text": "def _register_default_params(self, param_fname, dim):"
    },
    {
      "line": 304,
      "text": "def __init__(self, config, texture_mask_index, tex_res):"
    },
    {
      "line": 333,
      "text": "def check_texture(self, config):"
    },
    {
      "line": 338,
      "text": "def forward(self, texcode, tex_offsets=None):"
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
# Copyright©2023 Max-Planck-Gesellschaft zur Förderung
# der Wissenschaften e.V. (MPG). acting on behalf of its Max Planck Institute
# for Intelligent Systems. All rights reserved.
#
# Contact: mica@tue.mpg.de

import os
import pickle

import numpy as np
# Modified from smplx code for FLAME
import torch
import torch.nn as nn
import torch.nn.functional as F
from skimage.io import imread

from pixel3dmm.utils.utils_3d import rotation_6d_to_matrix, matrix_to_rotation_6d
from pixel3dmm.tracking.flame.lbs import lbs
from pixel3dmm import env_paths

I = matrix_to_rotation_6d(torch.eye(3)[None].cuda())


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


def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] +
                    rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


class FLAME(nn.Module):
    """
    borrowed from https://github.com/soubhiksanyal/FLAME_PyTorch/blob/master/FLAME.py
    Given FLAME parameters for shape, pose, and expression, this class generates a differentiable FLAME function
    which outputs the a mesh and 2D/3D facial landmarks
    """

    def __init__(self, config):
        super(FLAME, self).__init__()
        if config.use_flame2023:
            with open(f'{env_paths.FLAME_ASSETS}/FLAME2023/flame2023_no_jaw.pkl', 'rb') as f:
                ss = pickle.load(f, encoding='latin1')
                flame_model = Struct(**ss)
            self.use_flame2023 = True
        else:
            with open(f'{env_paths.FLAME_ASSETS}/FLAME2020/generic_model.pkl', 'rb') as f:
                ss = pickle.load(f, encoding='latin1')
                flame_model = Struct(**ss)
            self.use_flame2023 = False

        self.dtype = torch.float32
        self.register_buffer('faces', to_tensor(to_np(flame_model.f, dtype=np.int64), dtype=torch.long))
        # The vertices of the template model
        self.register_buffer('v_template', to_tensor(to_np(flame_model.v_template), dtype=self.dtype))
        # The shape components and expression
        shapedirs = to_tensor(to_np(flame_model.shapedirs), dtype=self.dtype)
        shapedirs = torch.cat([shapedirs[:, :, :config.num_shape_params], shapedirs[:, :, 300:300 + config.num_exp_params]], 2)
        self.register_buffer('shapedirs', shapedirs)
        # The pose components
        num_pose_basis = flame_model.posedirs.shape[-1]
        posedirs = np.reshape(flame_model.posedirs, [-1, num_pose_basis]).T
        self.register_buffer('posedirs', to_tensor(to_np(posedirs), dtype=self.dtype))
        #
        self.register_buffer('J_regressor', to_tensor(to_np(flame_model.J_regressor), dtype=self.dtype))
        parents = to_tensor(to_np(flame_model.kintree_table[0])).long();
        parents[0] = -1
        self.register_buffer('parents', parents)
        self.register_buffer('lbs_weights', to_tensor(to_np(flame_model.weights), dtype=self.dtype))

        self.register_buffer('l_eyelid', torch.from_numpy(np.load(f'{os.path.abspath(os.path.dirname(__file__))}/blendshapes/l_eyelid.npy')).to(self.dtype)[None])
        self.register_buffer('r_eyelid', torch.from_numpy(np.load(f'{os.path.abspath(os.path.dirname(__file__))}/blendshapes/r_eyelid.npy')).to(self.dtype)[None])

        # Register default parameters
        self._register_default_params('neck_pose_params', 6)
        self._register_default_params('jaw_pose_params', 6)
        self._register_default_params('eye_pose_params', 12)
        self._register_default_params('shape_params', config.num_shape_params)
        self._register_default_params('expression_params', config.num_exp_params)

        # Static and Dynamic Landmark embeddings for FLAME
        lmk_embeddings = np.load(f'{env_paths.FLAME_ASSETS}/FLAME2020/landmark_embedding.npy', allow_pickle=True, encoding='latin1')
        lmk_embeddings = lmk_embeddings[()]


        self.register_buffer('lmk_faces_idx', torch.from_numpy(lmk_embeddings['static_lmk_faces_idx'].astype(int)).to(torch.int64))
        self.register_buffer('lmk_bary_coords', torch.from_numpy(lmk_embeddings['static_lmk_bary_coords']).to(self.dtype).float())
        self.register_buffer('dynamic_lmk_faces_idx', torch.from_numpy(np.array(lmk_embeddings['dynamic_lmk_faces_idx']).astype(int)).to(torch.int64))
        self.register_buffer('dynamic_lmk_bary_coords', torch.from_numpy(np.array(lmk_embeddings['dynamic_lmk_bary_coords'])).to(self.dtype).float())

        neck_kin_chain = []
        NECK_IDX = 1
        curr_idx = torch.tensor(NECK_IDX, dtype=torch.long)
        while curr_idx != -1:
            neck_kin_chain.append(curr_idx)
            curr_idx = self.parents[curr_idx]
        self.register_buffer('neck_kin_chain', torch.stack(neck_kin_chain))

    def _find_dynamic_lmk_idx_and_bcoords(self, vertices, pose, dynamic_lmk_faces_idx,
                                          dynamic_lmk_b_coords,
                                          neck_kin_chain, cameras, dtype=torch.float32):
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

        batch_size = vertices.shape[0]

        aa_pose = torch.index_select(pose.view(batch_size, -1, 6), 1, neck_kin_chain)
        rot_mats = rotation_6d_to_matrix(aa_pose.view(-1, 6)).view([batch_size, -1, 3, 3])

        rel_rot_mat = torch.eye(3, device=vertices.device, dtype=dtype).unsqueeze_(dim=0).expand(batch_size, -1, -1)

        for idx in range(len(neck_kin_chain)):
            rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)

        if cameras is not None:
            rel_rot_mat = cameras @ rel_rot_mat  # Cameras flips z and x, plus multiview needs different lmk sliding per view

        y_rot_angle = torch.round(torch.clamp(-rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi, max=39)).to(dtype=torch.long)
        neg_mask = y_rot_angle.lt(0).to(dtype=torch.long)
        mask = y_rot_angle.lt(-39).to(dtype=torch.long)
        neg_vals = mask * 78 + (1 - mask) * (39 - y_rot_angle)
        y_rot_angle = (neg_mask * neg_vals + (1 - neg_mask) * y_rot_angle)

        dyn_lmk_faces_idx = torch.index_select(dynamic_lmk_faces_idx, 0, y_rot_angle)
        dyn_lmk_b_coords = torch.index_select(dynamic_lmk_b_coords, 0, y_rot_angle)

        return dyn_lmk_faces_idx, dyn_lmk_b_coords

    def _vertices2landmarks(self, vertices, faces, lmk_faces_idx, lmk_bary_coords):
        """
            Calculates landmarks by barycentric interpolation
            Input:
                vertices: torch.tensor NxVx3, dtype = torch.float32
                    The tensor of input vertices
                faces: torch.tensor (N*F)x3, dtype = torch.long
                    The faces of the mesh
                lmk_faces_idx: torch.tensor N X L, dtype = torch.long
                    The tensor with the indices of the faces used to calculate the
                    landmarks.
                lmk_bary_coords: torch.tensor N X L X 3, dtype = torch.float32
                    The tensor of barycentric coordinates that are used to interpolate
                    the landmarks

            Returns:
                landmarks: torch.tensor NxLx3, dtype = torch.float32
                    The coordinates of the landmarks for each mesh in the batch
        """
        # Extract the indices of the vertices for each face
        # NxLx3
        batch_size, num_verts = vertices.shape[:2]
        device = vertices.device
        lmk_faces = torch.index_select(faces, 0, lmk_faces_idx.view(-1).to(torch.long)).view(batch_size, -1, 3)
        lmk_faces += torch.arange(batch_size, dtype=torch.long, device=device).view(-1, 1, 1) * num_verts
        lmk_vertices = vertices.view(-1, 3)[lmk_faces].view(batch_size, -1, 3, 3)
        landmarks = torch.einsum('blfi,blf->bli', [lmk_vertices, lmk_bary_coords])

        return landmarks

    def forward(self, shape_params,
                cameras,
                trans_params=None,
                rot_params=None,
                neck_pose_params=None,
                jaw_pose_params=None,
                eye_pose_params=None,
                expression_params=None,
                eyelid_params=None,
                rot_params_lmk_shift = None,
                vertex_offsets = None,

                ):

        """
            Input:
                trans_params: N X 3 global translation
                rot_params: N X 3 global rotation around the root joint of the kinematic tree (rotation is NOT around the origin!)
                neck_pose_params (optional): N X 3 rotation of the head vertices around the neck joint
                jaw_pose_params (optional): N X 3 rotation of the jaw
                eye_pose_params (optional): N X 6 rotations of left (parameters [0:3]) and right eyeball (parameters [3:6])
                shape_params (optional): N X number of shape parameters
                expression_params (optional): N X number of expression parameters
            return:d
                vertices: N X V X 3
                landmarks: N X number of landmarks X 3
        """
        if self.use_flame2023:
            jaw_pose_params = None

        batch_size = shape_params.shape[0]

        I = matrix_to_rotation_6d(torch.cat([torch.eye(3)[None]] * batch_size, dim=0).cuda())

        if trans_params is None:
            trans_params = torch.zeros(batch_size, 3).cuda()
        if rot_params is None:
            rot_params = I.clone()
        if rot_params_lmk_shift is None:
            rot_params_lmk_shift = rot_params
        if neck_pose_params is None:
            neck_pose_params = I.clone()
        if jaw_pose_params is None:
            jaw_pose_params = I.clone()
        if eye_pose_params is None:
            eye_pose_params = torch.cat([I.clone()] * 2, dim=1)
        if shape_params is None:
            shape_params = self.shape_params.expand(batch_size, -1)
        if expression_params is None:
            expression_params = self.expression_params.expand(batch_size, -1)

        # Concatenate identity shape and expression parameters
        betas = torch.cat([shape_params, expression_params], dim=1)

        # The pose vector contains global rotation, and neck, jaw, and eyeball rotations
        full_pose = torch.cat([rot_params, neck_pose_params, jaw_pose_params, eye_pose_params], dim=1)
        full_pose_no_neck = torch.cat([rot_params, I, jaw_pose_params, eye_pose_params], dim=1)
        full_pose_lmk_shift = torch.cat([rot_params_lmk_shift, neck_pose_params, jaw_pose_params, eye_pose_params], dim=1)

        # FLAME models shape and expression deformations as vertex offset from the mean face in 'zero pose', called v_template
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)

        # Use linear blendskinning to model pose roations
        vertices, joint_transforms, v_can = lbs(betas, full_pose, template_vertices,
                          self.shapedirs, self.posedirs,
                          self.J_regressor, self.parents,
                          self.lbs_weights, dtype=self.dtype)

        vertices_noneck, _, v_can = lbs(betas, full_pose_no_neck, template_vertices,
                                 self.shapedirs, self.posedirs,
                                 self.J_regressor, self.parents,
                                 self.lbs_weights, dtype=self.dtype)

        #if vertex_offsets is not None:
        #    vertices[:, self.vertex_face_mask, :] = vertices[:, self.vertex_face_mask, :] + vertex_offsets

        if eyelid_params is not None:
            vertices = vertices + self.r_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 1:2, None]
            vertices = vertices + self.l_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 0:1, None]

        lmk_faces_idx = self.lmk_faces_idx.unsqueeze(dim=0).expand(batch_size, -1).contiguous()
        lmk_bary_coords = self.lmk_bary_coords.unsqueeze(dim=0).expand(batch_size, -1, -1).contiguous()

        dyn_lmk_faces_idx, dyn_lmk_bary_coords = self._find_dynamic_lmk_idx_and_bcoords(
            vertices, full_pose_lmk_shift, self.dynamic_lmk_faces_idx,
            self.dynamic_lmk_bary_coords,
            self.neck_kin_chain, cameras, dtype=self.dtype)

        lmk_faces_idx = torch.cat([dyn_lmk_faces_idx, lmk_faces_idx], 1)
        lmk_bary_coords = torch.cat([dyn_lmk_bary_coords, lmk_bary_coords], 1)

        lmk68 = self._vertices2landmarks(vertices, self.faces, lmk_faces_idx, lmk_bary_coords)



        # always zero in this code-base
        #vertices = vertices + trans_params.unsqueeze(dim=1)
        #lmk68 = lmk68 + trans_params.unsqueeze(dim=1)

        return vertices, lmk68, joint_transforms, v_can, vertices_noneck

    def _register_default_params(self, param_fname, dim):
        default_params = torch.zeros([1, dim], dtype=self.dtype, requires_grad=False)
        self.register_parameter(param_fname, nn.Parameter(default_params, requires_grad=False))


class FLAMETex(nn.Module):
    def __init__(self, config, texture_mask_index, tex_res):
        super(FLAMETex, self).__init__()
        tex_space = np.load(config.tex_space_path)
        # FLAME texture
        if 'tex_dir' in tex_space.files:
            mu_key = 'mean'
            pc_key = 'tex_dir'
            n_pc = 200
            scale = 1
        # BFM to FLAME texture
        else:
            mu_key = 'MU'
            pc_key = 'PC'
            n_pc = 199
            scale = 255.0
        texture_mean = tex_space[mu_key].reshape(1, -1)
        texture_basis = tex_space[pc_key].reshape(-1, n_pc)
        n_tex = config.tex_params
        texture_mean = torch.from_numpy(texture_mean).float()[None, ...] * scale
        texture_basis = torch.from_numpy(texture_basis[:, :n_tex]).float()[None, ...] * scale
        self.texture = None
        self.register_buffer('texture_mean', texture_mean)
        self.register_buffer('texture_basis', texture_basis)
        self.image_size = (512, 512) #config.image_size
        #self.check_texture(config)

        self.texture_mask_index = texture_mask_index
        self.tex_res = tex_res

    def check_texture(self, config):
        path = os.path.join(config.actor, 'texture.png')
        if os.path.exists(path):
            self.texture = torch.from_numpy(imread(path)).permute(2, 0, 1).cuda()[None, 0:3, :, :] / 255.0

    def forward(self, texcode, tex_offsets=None):
        if self.texture is not None:
            return F.interpolate(self.texture, self.image_size, mode='bilinear')
        texture = self.texture_mean + (self.texture_basis * texcode[:, None, :]).sum(-1)
        texture = texture.reshape(texcode.shape[0], 512, 512, 3).permute(0, 3, 1, 2)
        texture = F.interpolate(texture, (self.tex_res, self.tex_res), mode='bilinear')

        #texture = F.interpolate(texture, (1024, 1024), mode='bilinear')

        texture = texture[:, [2, 1, 0], :, :]
        texture =  texture / 255.
        if tex_offsets is not None:
            texture[:, :, self.texture_mask_index[0], self.texture_mask_index[1]] += tex_offsets

        #texture = F.interpolate(texture, (512, 512), mode='bilinear')

        return texture


```

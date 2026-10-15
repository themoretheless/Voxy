# Facial rendering source scan: yfeng95/PIXIE

Repository: https://github.com/yfeng95/PIXIE

Pinned source revision: 5c8879b828fc78e69b8d457029b27e825eae0d1c

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["data/uv_face_eye_mask.png"], "flame": ["pixielib/models/FLAME.py"], "lbs": ["pixielib/models/lbs.py"], "face": ["Doc/images/face.gif", "TestSamples/face/2.png", "TestSamples/face/4.png", "TestSamples/face/image00213.jpg", "TestSamples/face/image02673.png", "TestSamples/face/image03906.jpg", "TestSamples/face/image03928.jpg", "TestSamples/face/image03933.jpg", "TestSamples/face/image03937.jpg", "TestSamples/face/image04375.jpg", "TestSamples/face/timo.jpg", "TestSamples/face/xiangpu.jpg", "data/uv_face_eye_mask.png", "data/uv_face_mask.png", "demos/demo_fit_face.py", "pixielib/datasets/face_datasets.py"]}

## Source file: pixielib/models/lbs.py

{
  "path": "pixielib/models/lbs.py",
  "blob_sha": "3d65c6ccc48bb078bb32545f9819f44675572829",
  "sha256": "8e5faebba45b51c275e07bafd54dbd403ab068ba537f2e55c50348e8eb247647",
  "lines": 452,
  "bytes": 16363,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 14,
    "face": 31
  },
  "url": "https://github.com/yfeng95/PIXIE/blob/5c8879b828fc78e69b8d457029b27e825eae0d1c/pixielib/models/lbs.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 37,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 108,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 147,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 237,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 257,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 281,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 315,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 328,
      "text": "def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):"
    },
    {
      "line": 385,
      "text": "def __init__(self, fname):"
    },
    {
      "line": 420,
      "text": "def extra_joint_names(self):"
    },
    {
      "line": 425,
      "text": "def forward(self, vertices, faces):"
    },
    {
      "line": 440,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 444,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 450,
      "text": "def __init__(self, **kwargs):"
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

import numpy as np
import os
import yaml
import torch
import torch.nn.functional as F
from torch import nn

def rot_mat_to_euler(rot_mats):
    # Calculates rotation matrix to euler angles
    # Careful for extreme cases of eular angles like [0.0, pi, 0.0]

    sy = torch.sqrt(rot_mats[:, 0, 0] * rot_mats[:, 0, 0] +
                    rot_mats[:, 1, 0] * rot_mats[:, 1, 0])
    return torch.atan2(-rot_mats[:, 2, 0], sy)


def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,
                                     dynamic_lmk_b_coords,
                                     head_kin_chain, dtype=torch.float32):
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
        head_kin_chain: list
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
    pose = pose.detach()
    # aa_pose = torch.index_select(pose.view(batch_size, -1, 3), 1,
    #                              head_kin_chain)
    # rot_mats = batch_rodrigues(
    #     aa_pose.view(-1, 3), dtype=dtype).view(batch_size, -1, 3, 3)
    rot_mats = torch.index_select(pose, 1, head_kin_chain)

    rel_rot_mat = torch.eye(3, device=vertices.device,
                            dtype=dtype).unsqueeze_(dim=0)
    for idx in range(len(head_kin_chain)):
        # rel_rot_mat = torch.bmm(rot_mats[:, idx], rel_rot_mat)
        rel_rot_mat = torch.matmul(rot_mats[:, idx], rel_rot_mat)

    y_rot_angle = torch.round(
        torch.clamp(-rot_mat_to_euler(rel_rot_mat) * 180.0 / np.pi,
                    max=39)).to(dtype=torch.long)
    # print(y_rot_angle[0])
    neg_mask = y_rot_angle.lt(0).to(dtype=torch.long)
    mask = y_rot_angle.lt(-39).to(dtype=torch.long)
    neg_vals = mask * 78 + (1 - mask) * (39 - y_rot_angle)
    y_rot_angle = (neg_mask * neg_vals +
                   (1 - neg_mask) * y_rot_angle)
    # print(y_rot_angle[0])

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

    # # The last column of the transformations contains the posed joints
    # posed_joints = transforms[:, :, :3, 3]

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(
        torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])

    return posed_joints, rel_transforms

class JointsFromVerticesSelector(nn.Module):
    def __init__(self, fname):
        ''' Selects extra joints from vertices
        '''
        super(JointsFromVerticesSelector, self).__init__()

        err_msg = (
            'Either pass a filename or triangle face ids, names and'
            ' barycentrics')
        assert fname is not None or (
            face_ids is not None and bcs is not None and names is not None
        ), err_msg
        if fname is not None:
            fname = os.path.expanduser(os.path.expandvars(fname))
            with open(fname, 'r') as f:
                data = yaml.load(f)
            names = list(data.keys())
            bcs = []
            face_ids = []
            for name, d in data.items():
                face_ids.append(d['face'])
                bcs.append(d['bc'])
            bcs = np.array(bcs, dtype=np.float32)
            face_ids = np.array(face_ids, dtype=np.int32)
        assert len(bcs) == len(face_ids), (
            'The number of barycentric coordinates must be equal to the faces'
        )
        assert len(names) == len(face_ids), (
            'The number of names must be equal to the number of '
        )

        self.names = names
        self.register_buffer('bcs', torch.tensor(bcs, dtype=torch.float32))
        self.register_buffer(
            'face_ids', torch.tensor(face_ids, dtype=torch.long))

    def extra_joint_names(self):
        ''' Returns the names of the extra joints
        '''
        return self.names

    def forward(self, vertices, faces):
        if len(self.face_ids) < 1:
            return []
        vertex_ids = faces[self.face_ids].reshape(-1)
        # Should be BxNx3x3
        triangles = torch.index_select(vertices, 1, vertex_ids).reshape(
            -1, len(self.bcs), 3, 3)
        return (triangles * self.bcs[None, :, :, None]).sum(dim=2)

# def to_tensor(array, dtype=torch.float32):
#     if torch.is_tensor(array):
#         return array
#     else:
#         return torch.tensor(array, dtype=dtype)

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


```

## Source file: pixielib/models/FLAME.py

{
  "path": "pixielib/models/FLAME.py",
  "blob_sha": "69a1070da3214b79456b930ed94a5e90b5458ca1",
  "sha256": "9116e8edb69e3d6a28c959bb56d13515da59ac2d829e96e59c04f6dd467a784b",
  "lines": 92,
  "bytes": 4001,
  "selection_score": 15,
  "keyword_line_counts": {
    "flame": 13,
    "rig": 3,
    "face": 1,
    "lip": 2
  },
  "url": "https://github.com/yfeng95/PIXIE/blob/5c8879b828fc78e69b8d457029b27e825eae0d1c/pixielib/models/FLAME.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def __init__(self, config):"
    },
    {
      "line": 59,
      "text": "def forward(self, texcode=None):"
    },
    {
      "line": 71,
      "text": "def texture_flame2smplx(cached_data, flame_texture, smplx_texture):"
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
# For comments or questions, please email us at pixie@tue.mpg.de
# For commercial licensing contact, please contact ps-license@tuebingen.mpg.de

import torch
import torch.nn as nn
import numpy as np
import pickle
import torch.nn.functional as F

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
            texture_mean = tex_space[mu_key].reshape(1, -1)/255.
            texture_basis = tex_space[pc_key].reshape(-1, n_pc)/255.
        else:
            print('texture type ', config.tex_type, 'not exist!')
            raise NotImplementedError

        n_tex = config.n_tex
        num_components = texture_basis.shape[1]
        texture_mean = torch.from_numpy(texture_mean).float()[None,...]
        texture_basis = torch.from_numpy(texture_basis[:,:n_tex]).float()[None,...]
        self.register_buffer('texture_mean', texture_mean)
        self.register_buffer('texture_basis', texture_basis)

    def forward(self, texcode=None):
        '''
        texcode: [batchsize, n_tex]
        texture: [bz, 3, 256, 256], range: 0-1
        '''
        texture = self.texture_mean + (self.texture_basis*texcode[:,None,:]).sum(-1)
        texture = texture.reshape(texcode.shape[0], 512, 512, 3).permute(0,3,1,2)
        texture = F.interpolate(texture, [256, 256])
        texture = texture[:,[2,1,0], :,:]
        return texture


def texture_flame2smplx(cached_data, flame_texture, smplx_texture):
    ''' Convert flame texture map (face-only) into smplx texture map (includes body texture)
    TODO: pytorch version ==> grid sample
    '''
    if smplx_texture.shape[0] != smplx_texture.shape[1]:
        print('SMPL-X texture not squared (%d != %d)' % (smplx_texture[0], smplx_texture[1]))
        return
    if smplx_texture.shape[0] != cached_data['target_resolution']:
        print('SMPL-X texture size does not match cached image resolution (%d != %d)' % (smplx_texture.shape[0], cached_data['target_resolution']))
        return
    x_coords = cached_data['x_coords']
    y_coords = cached_data['y_coords']
    target_pixel_ids = cached_data['target_pixel_ids']
    source_uv_points = cached_data['source_uv_points']

    source_tex_coords = np.zeros_like((source_uv_points)).astype(int)
    source_tex_coords[:, 0] = np.clip(flame_texture.shape[0]*(1.0-source_uv_points[:,1]), 0.0, flame_texture.shape[0]).astype(int)
    source_tex_coords[:, 1] = np.clip(flame_texture.shape[1]*(source_uv_points[:,0]), 0.0, flame_texture.shape[1]).astype(int)

    smplx_texture[y_coords[target_pixel_ids].astype(int), x_coords[target_pixel_ids].astype(int), :] = flame_texture[source_tex_coords[:,0], source_tex_coords[:,1]]

    return smplx_texture

```

## Source file: pixielib/datasets/face_datasets.py

{
  "path": "pixielib/datasets/face_datasets.py",
  "blob_sha": "cd0a410e5563efebd26d1e4f2f0524077ab741bc",
  "sha256": "128a576b5940f3bb008b53c6e50955b063db5980e97bf906b8cf995ecf060ef5",
  "lines": 418,
  "bytes": 19188,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 1,
    "mouth": 1,
    "rig": 32,
    "face": 29,
    "lip": 1
  },
  "url": "https://github.com/yfeng95/PIXIE/blob/5c8879b828fc78e69b8d457029b27e825eae0d1c/pixielib/datasets/face_datasets.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def video2sequence(video_path):"
    },
    {
      "line": 32,
      "text": "def __init__(self, testpath, iscrop=True, crop_size=224, scale=2.0, face_detector='fan'):"
    },
    {
      "line": 61,
      "text": "def __len__(self):"
    },
    {
      "line": 64,
      "text": "def bbox2point(self, left, right, top, bottom, type='bbox'):"
    },
    {
      "line": 77,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 121,
      "text": "def __init__(self, iscrop=True, crop_size=224, scale=2.0, ):"
    },
    {
      "line": 132,
      "text": "def __len__(self):"
    },
    {
      "line": 135,
      "text": "def bbox2point(self, left, right, top, bottom, type='type'):"
    },
    {
      "line": 148,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 189,
      "text": "def __init__(self, iscrop=False, crop_size=224, hd_size = 1024, scale=1.1, body_detector='rcnn', device='cuda:0'):"
    },
    {
      "line": 207,
      "text": "def __len__(self):"
    },
    {
      "line": 210,
      "text": "def bbox2point(self, left, right, top, bottom, type='type'):"
    },
    {
      "line": 223,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 283,
      "text": "def __init__(self, crop_size, scale=[1, 1], trans_scale = 0., blur_step=1, split='train'):"
    },
    {
      "line": 310,
      "text": "def __len__(self):"
    },
    {
      "line": 313,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 395,
      "text": "def load_mask(self, maskpath, h, w):"
    },
    {
      "line": 408,
      "text": "def load_gender(self, genderpath):"
    }
  ]
}

```text

import os, sys
import torch
from torch.utils.data import Dataset, DataLoader
import torchvision.transforms as transforms
import numpy as np
import cv2
import scipy
from skimage.io import imread, imsave
from skimage.transform import estimate_transform, warp, resize, rescale
from glob import glob
import scipy.io
from . import detectors

def video2sequence(video_path):
    videofolder = video_path.split('.')[0]
    util.check_mkdir(videofolder)
    video_name = video_path.split('/')[-1].split('.')[0]
    vidcap = cv2.VideoCapture(video_path)
    success,image = vidcap.read()
    count = 0
    imagepath_list = []
    while success:
        imagepath = '{}/{}_frame{:04d}.jpg'.format(videofolder, video_name, count)
        cv2.imwrite(imagepath, image)     # save frame as JPEG file
        success,image = vidcap.read()
        count += 1
        imagepath_list.append(imagepath)
    print('video frames are stored in {}'.format(videofolder))
    return imagepath_list

class TestData(Dataset):
    def __init__(self, testpath, iscrop=True, crop_size=224, scale=2.0, face_detector='fan'):
        '''
            testpath: folder, imagepath_list, image path, video path
        '''
        if isinstance(testpath, list):
            self.imagepath_list = testpath
        elif os.path.isdir(testpath): 
            self.imagepath_list = glob(testpath + '/*.jpg') +  glob(testpath + '/*.png') + glob(testpath + '/*.bmp') + glob(testpath + '/*.jpeg')
        elif os.path.isfile(testpath) and (testpath[-3:] in ['jpg', 'png', 'bmp']):
            self.imagepath_list = [testpath]
        elif os.path.isfile(testpath) and (testpath[-3:] in ['mp4', 'csv', 'vid', 'ebm']):
            self.imagepath_list = video2sequence(testpath)
        else:
            print(f'please check the test path: {testpath}')
            exit()
        print('total {} images'.format(len(self.imagepath_list)))
        self.imagepath_list = sorted(self.imagepath_list)
        self.crop_size = crop_size
        self.scale = scale
        self.iscrop = iscrop
        self.resolution_inp = crop_size
        if face_detector == 'fan':
            self.face_detector = detectors.FAN()
        # elif face_detector == 'mtcnn':
        #     self.face_detector = detectors.MTCNN()
        else:
            print(f'please check the detector: {face_detector}')
            exit()

    def __len__(self):
        return len(self.imagepath_list)

    def bbox2point(self, left, right, top, bottom, type='bbox'):
        ''' bbox from detector and landmarks are different
        '''
        if type=='kpt68':
            old_size = (right - left + bottom - top)/2*1.1
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0 ])
        elif type=='bbox':
            old_size = (right - left + bottom - top)/2
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0  + old_size*0.12])
        else:
            raise NotImplementedError
        return old_size, center

    def __getitem__(self, index):
        imagepath = self.imagepath_list[index]
        imagename = imagepath.split('/')[-1].split('.')[0]
        # print(imagepath)
        # exit()
        # import ipdb; ipdb.set_trace()
        image = np.array(cv2.imread(imagepath))[:,:,[2,1,0]]
        # image = np.array(imread(imagepath))
        if len(image.shape) == 2:
            image = image[:,:,None].repeat(1,1,3)
        if len(image.shape) == 3 and image.shape[2] > 3:
            image = image[:,:,:3]

        h, w, _ = image.shape
        if self.iscrop:
            bbox = self.face_detector.run(image)
            if len(bbox) < 4:
                print(f'no face detected! run original image: {imagepath}')
                left = 0; right = h-1; top=0; bottom=w-1
            else:
                left = bbox[0]; right=bbox[2]
                top = bbox[1]; bottom=bbox[3]
            old_size, center = self.bbox2point(left, right, top, bottom, type='kpt68')
            size = int(old_size*self.scale)
            src_pts = np.array([[center[0]-size/2, center[1]-size/2], [center[0] - size/2, center[1]+size/2], [center[0]+size/2, center[1]-size/2]])
        else:
            src_pts = np.array([[0, 0], [0, h-1], [w-1, 0]])
        
        DST_PTS = np.array([[0,0], [0,self.resolution_inp - 1], [self.resolution_inp - 1, 0]])
        tform = estimate_transform('similarity', src_pts, DST_PTS)
        
        image = image/255.

        dst_image = warp(image, tform.inverse, output_shape=(self.resolution_inp, self.resolution_inp))
        dst_image = dst_image.transpose(2,0,1)
        return {'image': torch.tensor(dst_image).float(),
                'name': imagename,
                'imagepath': imagepath
                # 'tform': tform,
                # 'original_image': torch.tensor(image.transpose(2,0,1)).float(),
                }


class NoWTest(Dataset):
    def __init__(self, iscrop=True, crop_size=224, scale=2.0, ):
        self.iscrop = iscrop
        self.scale = scale
        self.crop_size = crop_size
        # self.data_path = '/ps/scratch/face2d3d/texture_in_the_wild_code/NoW_validation/image_paths_ring_6_elements.npy'
        self.data_path = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/test_image_paths_ring_6_elements.npy'
        self.data_lines = np.load(self.data_path).astype('str').flatten()
        # import ipdb; ipdb.set_trace()
        self.imagepath = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/iphone_pictures/'
        self.bbxpath = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/detected_face/'
        self.resolution_inp = crop_size
    def __len__(self):
        return self.data_lines.shape[0]

    def bbox2point(self, left, right, top, bottom, type='type'):
        ''' bbox from detector and landmarks are different
        '''
        if type=='kpt68':
            old_size = (right - left + bottom - top)/2*1.1
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0 ])
        elif type=='bbox':
            old_size = (right - left + bottom - top)/2
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0  + old_size*0.12])
        else:
            raise NotImplementedError
        return old_size, center

    def __getitem__(self, index):
        image_th = []
        image_names =[]
        imagename = self.data_lines[index]
        imagepath = self.imagepath + self.data_lines[index] + '.jpg'
        bbx_path = self.bbxpath + self.data_lines[index] + '.npy'
        image = np.array(imread(imagepath))
        if len(image.shape) == 2:
            image = image[:,:,None].repeat(1,1,3)
        if len(image.shape) == 3 and image.shape[2] > 3:
            image = image[:,:,:3]

        h, w, _ = image.shape
        
        bbx_data = np.load(bbx_path, allow_pickle=True, encoding='latin1').item()
        # bbox = np.array([[bbx_data['left'], bbx_data['top']], [bbx_data['right'], bbx_data['bottom']]]).astype('float32')
        # import ipdb; ipdb.set_trace()
        left = bbx_data['left']; right=bbx_data['right']
        top = bbx_data['top']; bottom=bbx_data['bottom']
        old_size, center = self.bbox2point(left, right, top, bottom, type='bbox')
        size = int(old_size*self.scale)
        src_pts = np.array([[center[0]-size/2, center[1]-size/2], [center[0] - size/2, center[1]+size/2], [center[0]+size/2, center[1]-size/2]])

        DST_PTS = np.array([[0,0], [0,self.resolution_inp - 1], [self.resolution_inp - 1, 0]])
        tform = estimate_transform('similarity', src_pts, DST_PTS)
        
        image = image/255.

        dst_image = warp(image, tform.inverse, output_shape=(self.resolution_inp, self.resolution_inp))
        dst_image = dst_image.transpose(2,0,1)
        
        return {'image': torch.tensor(dst_image).float(),
                'name': imagename,
                'imagepath': imagepath
                # 'tform': tform,
                # 'original_image': torch.tensor(image.transpose(2,0,1)).float(),
                }


from . import detectors
class NoWTest_body(Dataset):
    def __init__(self, iscrop=False, crop_size=224, hd_size = 1024, scale=1.1, body_detector='rcnn', device='cuda:0'):
        self.iscrop = iscrop
        self.scale = scale
        self.crop_size = crop_size
        # self.data_path = '/ps/scratch/face2d3d/texture_in_the_wild_code/NoW_validation/image_paths_ring_6_elements.npy'
        # self.data_path = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/test_image_paths_ring_6_elements.npy'
        self.data_path = '/ps/scratch/face2d3d/texture_in_the_wild_code/NoW_validation/image_paths._ring_6_elements.npy'
        self.data_lines = np.load(self.data_path).astype('str').flatten()
        # import ipdb; ipdb.set_trace()
        self.imagepath = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/iphone_pictures/'
        self.bbxpath = '/ps/scratch/face2d3d/ringnetpp/eccv/test_data/evaluation/NoW_Dataset/final_release_version/detected_face/'
        self.resolution_inp = crop_size
        self.detector = detectors.FasterRCNN(device=device)
        self.crop_size = crop_size
        self.hd_size = hd_size
        self.scale = scale
        self.iscrop = iscrop

    def __len__(self):
        return self.data_lines.shape[0]

    def bbox2point(self, left, right, top, bottom, type='type'):
        ''' bbox from detector and landmarks are different
        '''
        if type=='kpt68':
            old_size = (right - left + bottom - top)/2*1.1
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0 ])
        elif type=='bbox':
            old_size = (right - left + bottom - top)/2
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0  + old_size*0.12])
        else:
            raise NotImplementedError
        return old_size, center

    def __getitem__(self, index):
        image_th = []
        image_names =[]
        imagename = self.data_lines[index]
        imagepath = self.imagepath + self.data_lines[index] + '.jpg'
        # bbx_path = self.bbxpath + self.data_lines[index] + '.npy'
        # image = np.array(imread(imagepath))
        # if len(image.shape) == 2:
        #     image = image[:,:,None].repeat(1,1,3)
        # if len(image.shape) == 3 and image.shape[2] > 3:
        #     image = image[:,:,:3]

        # h, w, _ = image.shape
        
        image = imread(imagepath)[:,:,:3]/255.
        h, w, _ = image.shape

        image_tensor = torch.tensor(image.transpose(2,0,1), dtype=torch.float32)[None, ...]
        if self.iscrop:
            bbox = self.detector.run(image_tensor)
            if bbox is None:
                print('no person detected! run original image')
                left = 0; right = w-1; top=0; bottom=h-1
            else:
                left = bbox[0]; right = bbox[2]; top = bbox[1]; bottom = bbox[3]
            old_size = max(right - left, bottom - top)
            center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0])
            size = int(old_size*self.scale)
            src_pts = np.array([[center[0]-size/2, center[1]-size/2], [center[0] - size/2, center[1]+size/2], [center[0]+size/2, center[1]-size/2]])
        else:
            src_pts = np.array([[0, 0], [0, h-1], [w-1, 0]])
            left = 0; right = w-1; top=0; bottom=h-1
            bbox = [left, top, right, bottom]
            
        # crop image
        DST_PTS = np.array([[0,0], [0,self.crop_size - 1], [self.crop_size - 1, 0]])
        tform = estimate_transform('similarity', src_pts, DST_PTS)
        dst_image = warp(image, tform.inverse, output_shape=(self.crop_size, self.crop_size))
        dst_image = dst_image.transpose(2,0,1)
        # hd image
        DST_PTS = np.array([[0,0], [0,self.hd_size - 1], [self.hd_size - 1, 0]])
        tform_hd = estimate_transform('similarity', src_pts, DST_PTS)
        hd_image = warp(image, tform_hd.inverse, output_shape=(self.hd_size, self.hd_size))
        hd_image = hd_image.transpose(2,0,1)
        return {'image': torch.tensor(dst_image).float(),
                'name': imagename,
                'imagepath': imagepath, 
                'image_hd': torch.tensor(hd_image).float(),
                'tform': torch.tensor(tform.params).float(),
                'original_image': torch.tensor(image.transpose(2,0,1)).float(),
                'bbox': bbox
                }

### face training data
# cfg.dataset.head.scale_min = 1.4
# cfg.dataset.head.scale_max = 1.8
# cfg.dataset.head.trans_scale = 0.3
# cfg.dataset.head.blur_step = 1
from kornia.filters import median_blur, gaussian_blur2d, motion_blur
class VGGFace2(Dataset):
    def __init__(self, crop_size, scale=[1, 1], trans_scale = 0., blur_step=1, split='train'):
        '''
        K must be less than 6
        '''
        self.image_size = crop_size
        self.imagefolder = '/ps/scratch/face2d3d/train'
        self.kptfolder = '/ps/scratch/face2d3d/train_annotated_torch7'
        self.segfolder = '/ps/scratch/face2d3d/texture_in_the_wild_code/VGGFace2_seg/test_crop_size_400_batch'
        self.attfolder = '/ps/scratch/yfeng/Data/vggface2/gender-DEX'
        # hq:
        # datafile = '/ps/scratch/face2d3d/texture_in_the_wild_code/VGGFace2_cleaning_codes/ringnetpp_training_lists/second_cleaning/vggface2_bbx_size_bigger_than_400_train_list_max_normal_100_ring_5_1_serial.npy'
        if split == 'train':
            datafile = '/ps/scratch/face2d3d/texture_in_the_wild_code/VGGFace2_cleaning_codes/ringnetpp_training_lists/second_cleaning/vggface2_train_list_max_normal_100_ring_5_1_serial.npy'
        elif split == 'eval':
            datafile = '/ps/scratch/face2d3d/texture_in_the_wild_code/VGGFace2_cleaning_codes/ringnetpp_training_lists/second_cleaning/vggface2_val_list_max_normal_100_ring_5_1_serial.npy'
            self.imagefolder = '/ps/scratch/face2d3d/test'
            self.kptfolder = '/ps/scratch/face2d3d/test_annotated_torch7'
        ## for test, no random crop
        if split != 'train':
            scale[0] = scale[1] = (scale[0] + scale[1])/2.
            trans_scale = 0.
        self.split = split
        self.data_lines = np.load(datafile).astype('str')
        self.cropper = array_cropper.Cropper(crop_size, scale, trans_scale)
        assert blur_step != 0
        self.blur_step = blur_step

    def __len__(self):
        return len(self.data_lines)

    def __getitem__(self, idx):
        # i = 0 # i: 0-6
        if self.split == 'train':
            i = np.random.randint(6)
        else:
            i = 5
        name = self.data_lines[idx, i]
        image_path = os.path.join(self.imagefolder, name + '.jpg')  
        seg_path = os.path.join(self.segfolder, name + '.npy')  
        kpt_path = os.path.join(self.kptfolder, name + '.npy')
        genderpath = os.path.join(self.attfolder, name + '_vote.txt')
        gender = self.load_gender(genderpath)

        image = imread(image_path)/255.
        kpt = np.load(kpt_path)[:,:2]
        mask = self.load_mask(seg_path, image.shape[0], image.shape[1])

        ### crop information
        arrays = np.concatenate([image, mask], axis=-1)
        cropped_tensor, tform = self.cropper.crop(arrays, kpt)
        cropped_image = cropped_tensor[:,:,:3]
        cropped_mask = cropped_tensor[:,:,-1:]
        ## transform kpt 
        cropped_kpt = np.dot(np.hstack([kpt[:,:2], np.ones([kpt.shape[0],1])]), tform) # np.linalg.inv(tform.params)

        # normalized kpt
        cropped_kpt[:,:2] = cropped_kpt[:,:2]/self.image_size * 2  - 1
        cropped_kpt[:,[2]] = np.ones([kpt.shape[0],1]) # conf is 1
        
        ###
        image = torch.from_numpy(cropped_image.transpose(2,0,1)).type(dtype = torch.float32) 
        kpt = torch.from_numpy(cropped_kpt).type(dtype = torch.float32) 
        mask = torch.from_numpy(cropped_mask.transpose(2,0,1)).type(dtype = torch.float32) 

        image_hd = image.clone()

        if idx%self.blur_step  == 0:
            ### augment image resolution
            # blur_types = ['res', 'gaussian', 'median', 'motion']
            # more gaussian blur
            blur_types = ['res', 'gaussian', 'gaussian', 'motion']
            blur_type =  blur_types[np.random.randint(len(blur_types))]
            if blur_type == 'res':
                # self.resolution_scale : 0.1 ~ 1.2
                res_scale = np.random.rand() * 0.4 + 0.1
                image = image[None]
                res_size = int(res_scale*self.image_size)
                image = F.interpolate(image, (res_size, res_size), mode='bilinear')#, align_corners=False)
                image = F.interpolate(image, (self.image_size, self.image_size), mode='bilinear')#, align_corners=False)
                image = image.squeeze()
            elif blur_type == 'gaussion':
                ks = np.random.randint(5)*2 + 9
                sigma = np.random.randint(5)*0.2 + 1.2
                image = image[None]
                image = util.gaussian_blur(image, kernel_size=(ks,ks), sigma=(sigma,sigma))
                image = image.squeeze()
            elif blur_type == 'median':
                ks = np.random.randint(3)*2 + 5
                image = image[None]
                image = median_blur(image, kernel_size=(ks,ks))
                image = image.squeeze()
            elif blur_type == 'motion':
                ks = np.random.randint(6)*2 + 7
                angle = np.random.rand()*180 - 90
                direction = np.random.randint(21)*0.1 - 1
                image = image[None]
                image = motion_blur(image, kernel_size=ks, angle=angle, direction=direction)
                image = image.squeeze()
                # for motion blur, the position will change
                image_hd = image
        ## 
        data_dict = {
            'image': image,
            'image_hd': image_hd,
            'face_kpt': kpt,
            'smplx_kpt': kpt,
            'mask': mask,
            'gender': gender
        }
        
        return data_dict

    def load_mask(self, maskpath, h, w):
        # print(maskpath)
        if os.path.isfile(maskpath):
            vis_parsing_anno = np.load(maskpath)
            # atts = ['skin', 'l_brow', 'r_brow', 'l_eye', 'r_eye', 'eye_g', 'l_ear', 'r_ear', 'ear_r',
            #     'nose', 'mouth', 'u_lip', 'l_lip', 'neck', 'neck_l', 'cloth', 'hair', 'hat']
            mask = np.zeros_like(vis_parsing_anno)
            # for i in range(1, 16):
            mask[vis_parsing_anno>0.5] = 1.
        else:
            mask = np.ones((h, w))
        return mask[:,:,None]

    def load_gender(self, genderpath):
        if os.path.exists(genderpath) is False:
            return 'N'
        with open(genderpath, 'r') as f:
            label = f.readline()
        if label == 'female' or label == 'Woman':
            return 'F'
        elif label == 'male' or label == 'Man':
            return 'M'
        else:
            return 'N'


```

## Source file: demos/demo_fit_face.py

{
  "path": "demos/demo_fit_face.py",
  "blob_sha": "58393de245fe70a0d0b23f693dec0c78fd411cf4",
  "sha256": "5e17f2e47c1252e1425ad39f1280e62950318c23d118a05aecd4749aed78c55f",
  "lines": 133,
  "bytes": 7366,
  "selection_score": 13,
  "keyword_line_counts": {
    "gum": 18,
    "flame": 4,
    "face": 5
  },
  "url": "https://github.com/yfeng95/PIXIE/blob/5c8879b828fc78e69b8d457029b27e825eae0d1c/demos/demo_fit_face.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def main(args):"
    }
  ]
}

```text

import os, sys
import numpy as np
import torch.backends.cudnn as cudnn
import torch
from tqdm import tqdm
import argparse
import cv2

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from pixielib.pixie import PIXIE
from pixielib.visualizer import Visualizer
from pixielib.datasets.face_datasets import TestData
from pixielib.utils import util
from pixielib.utils.config import cfg as pixie_cfg

def main(args):
    savefolder = args.savefolder
    device = args.device
    os.makedirs(savefolder, exist_ok=True)
    
    # check env
    if not torch.cuda.is_available():
        print('CUDA is not available! use CPU instead')
    else:
        cudnn.benchmark = True
        torch.backends.cudnn.deterministic = False
        torch.backends.cudnn.enabled = True

    # load test images 
    testdata = TestData(args.inputpath, iscrop=args.iscrop)

    #-- run PIXIE
    pixie_cfg.model.use_tex = args.useTex
    pixie = PIXIE(config = pixie_cfg, device=device)
    visualizer = Visualizer(render_size=args.render_size, config = pixie_cfg, device=device, rasterizer_type=args.rasterizer_type)
    if args.deca_path:
        # if given deca code path, run deca to get face details
        sys.path.insert(0, args.deca_path)
        from decalib.deca import DECA
        deca = DECA(device=device)
    for i, batch in enumerate(tqdm(testdata, dynamic_ncols=True)):
        util.move_dict_to_device(batch, device)
        batch['image'] = batch['image'].unsqueeze(0)
        name = batch['name']
        
        data = {
            'head': batch
        }
        param_dict = pixie.encode(data, keep_local=False, threthold=True)
        codedict = param_dict['head']
        opdict = pixie.decode(codedict, param_type='head')
        opdict['albedo'] = visualizer.tex_flame2smplx(opdict['albedo'])
        if args.saveObj or args.saveParam or args.savePred or args.saveImages:
            os.makedirs(os.path.join(savefolder, name), exist_ok=True)

        # run deca if deca is available
        if args.deca_path is not None:
            _, deca_opdict, _ = deca.run(batch['imagepath'])
            flame_displacement_map = deca_opdict['displacement_map']
            opdict['displacement_map'] = visualizer.tex_flame2smplx(flame_displacement_map)
        if args.lightTex:
            visualizer.light_albedo(opdict)
        if args.extractTex:
            visualizer.extract_texture(opdict, data['head']['image'])
        if args.saveVis:
            visdict = visualizer.render_results(opdict, data['head']['image'], overlay=True)
            if args.showBody:
                body_vertices = pixie.decode_Tpose(codedict)
                body_vertices[:,:,1:] = -body_vertices[:,:,1:]
                cam = torch.zeros([1, 3], dtype=torch.float32, device=device)
                cam[:, 0] = 1.1; cam[:, 2] = -0.45
                trans_verts = util.batch_orth_proj(body_vertices, cam)
                visdict['body_shape'] = visualizer.render.render_shape(body_vertices, trans_verts)
            cv2.imwrite(os.path.join(savefolder, f'{name}_vis.jpg'), visualizer.visualize_grid(visdict, size=args.render_size))
        if args.saveObj:
            # save unposed body mesh
            if 'Tpose_vertices' not in opdict.keys():
                opdict['Tpose_vertices'] = pixie.decode_Tpose(codedict)
            visualizer.save_obj(os.path.join(savefolder, name, f'{name}.obj'), opdict, Tpose=True)
        if args.saveParam:
            util.save_pkl(os.path.join(savefolder, name, f'{name}_param.pkl'), codedict)
        if args.savePred:
            util.save_pkl(os.path.join(savefolder, name, f'{name}_prediction.pkl'), opdict) 
        if args.saveImages:
            for vis_name in visdict.keys():
                cv2.imwrite(os.path.join(savefolder, name, f'{name}_{vis_name}.jpg'), util.tensor2image(visdict[vis_name][0]))
                  
    print(f'-- please check the results in {savefolder}')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='PIXIE')

    parser.add_argument('-i', '--inputpath', default='TestSamples/face', type=str,
                        help='path to the test data, can be image folder, image path, image list, video')
    parser.add_argument('-s', '--savefolder', default='TestSamples/face/results', type=str,
                        help='path to the output directory, where results(obj, txt files) will be stored.')
    parser.add_argument('--device', default='cuda:0', type=str,
                        help='set device, cpu for using cpu' )
    # process test images
    parser.add_argument('--iscrop', default=True, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to crop input image, set false only when the test image are well cropped' )
    # rendering option
    parser.add_argument('--render_size', default=224, type=int,
                        help='image size of renderings' )
    parser.add_argument('--rasterizer_type', default='standard', type=str,
                    help='rasterizer type: pytorch3d or standard rasterizer' )
    # save
    parser.add_argument('--deca_path', default=None, type=str,
                        help='absolute path of DECA folder, if exists, will return facial details by running DECA\
                        details of DECA: https://github.com/YadiraF/DECA' )
    parser.add_argument('--useTex', default=True, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to use FLAME texture model to generate uv texture map, \
                            set it to True only if you downloaded texture model' )
    parser.add_argument('--lightTex', default=True, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to return lit albedo: that add estimated SH lighting to albedo')
    parser.add_argument('--extractTex', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to extract texture from input image, only do this when the face is near frontal and very clean!')
    parser.add_argument('--showBody', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to show body mesh in visualization' )
    parser.add_argument('--saveVis', default=True, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to save visualization of output' )
    parser.add_argument('--saveGif', default=True, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to visualize other views of the output, save as gif' )
    parser.add_argument('--saveObj', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to save outputs as .obj, \
                            Note that saving objs could be slow' )
    parser.add_argument('--saveParam', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to save parameters as pkl file' )
    parser.add_argument('--savePred', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to save smplx prediction as pkl file' )
    parser.add_argument('--saveImages', default=False, type=lambda x: x.lower() in ['true', '1'],
                        help='whether to save visualization output as seperate images' )
    main(parser.parse_args())


```

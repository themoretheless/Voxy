# Facial rendering source scan: gapszju/RGBAvatar

Repository: https://github.com/gapszju/RGBAvatar

Pinned source revision: 903653b689a8b860df30f165e242ccd6ed3f5583

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "submodules/diff-gaussian-rasterization/LICENSE.md", "submodules/diff-gaussian-rasterization/third_party/glm/copying.txt"]

Topic paths: {"eye": ["data/FLAME2020/l_eyelid.npy", "data/FLAME2020/r_eyelid.npy"], "flame": ["data/FLAME2020/flame_uv.npz", "data/FLAME2020/l_eyelid.npy", "data/FLAME2020/r_eyelid.npy", "dataset/flame_dataset.py", "submodules/flame/__init__.py", "submodules/flame/config.py", "submodules/flame/flame.py", "submodules/flame/lbs.py"], "lbs": ["submodules/flame/lbs.py"], "rig": ["submodules/diff-gaussian-rasterization/third_party/glm/glm/detail/func_trigonometric.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/detail/func_trigonometric_simd.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/quaternion_trigonometric.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/quaternion_trigonometric.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/gtx/fast_trigonometry.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/gtx/fast_trigonometry.inl", "submodules/diff-gaussian-rasterization/third_party/glm/glm/simd/trigonometric.h", "submodules/diff-gaussian-rasterization/third_party/glm/glm/trigonometric.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/core/core_func_trigonometric.cpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/ext/ext_quaternion_trigonometric.cpp", "submodules/diff-gaussian-rasterization/third_party/glm/test/gtx/gtx_fast_trigonometry.cpp"], "face": ["submodules/diff-gaussian-rasterization/cuda_utils/face_tbn.cu"], "lip": ["submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/matrix_clip_space.hpp", "submodules/diff-gaussian-rasterization/third_party/glm/glm/ext/matrix_clip_space.inl", "submodules/diff-gaussian-rasterization/third_party/glm/test/ext/ext_matrix_clip_space.cpp"]}

## Source file: submodules/flame/lbs.py

{
  "path": "submodules/flame/lbs.py",
  "blob_sha": "f263b30b2fd441858e94598eb3cd1ed43645bbbf",
  "sha256": "62dfec70464ece85feb1cbc8b1c62aeddbcb8d34c55b7e4779f672bb88edcd85",
  "lines": 449,
  "bytes": 16126,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 3,
    "blend_shape": 5,
    "deform": 2,
    "lbs": 6,
    "rig": 14,
    "face": 19
  },
  "url": "https://github.com/gapszju/RGBAvatar/blob/903653b689a8b860df30f165e242ccd6ed3f5583/submodules/flame/lbs.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def rotation_6d_to_matrix(d6: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 52,
      "text": "def stereographic_unproject_old(a):"
    },
    {
      "line": 62,
      "text": "def stereographic_unproject(a, axis=None):"
    },
    {
      "line": 79,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 88,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 153,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 192,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 282,
      "text": "def lbs_transfer(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 307,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 327,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 351,
      "text": "def _batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 385,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 398,
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
def rotation_6d_to_matrix(d6: torch.Tensor) -> torch.Tensor:
    """
    Converts 6D rotation representation by Zhou et al. [1] to rotation matrix
    using Gram--Schmidt orthogonalization per Section B of [1].
    Args:
        d6: 6D rotation representation, of size (*, 6)

    Returns:
        batch of rotation matrices of size (*, 3, 3)

    [1] Zhou, Y., Barnes, C., Lu, J., Yang, J., & Li, H.
    On the Continuity of Rotation Representations in Neural Networks.
    IEEE Conference on Computer Vision and Pattern Recognition, 2019.
    Retrieved from http://arxiv.org/abs/1812.07035
    """

    a1, a2 = d6[..., :3], d6[..., 3:]
    b1 = F.normalize(a1, dim=-1)
    b2 = a2 - (b1 * a2).sum(-1, keepdim=True) * b1
    b2 = F.normalize(b2, dim=-1)
    b3 = torch.cross(b1, b2, dim=-1)
    return torch.stack((b1, b2, b3), dim=-2)


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
    ans = torch.autograd.Variable(torch.zeros(batch, a.shape[1] + 1, device=a.device))  # batch*6
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
    T = torch.matmul(W, A.view(batch_size, num_joints, 16)) \
        .view(batch_size, -1, 4, 4)

    homogen_coord = torch.ones([batch_size, v_posed.shape[1], 1],
                               dtype=dtype, device=device)
    v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
    v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

    verts = v_homo[:, :, :3, 0]

    return verts, J_transformed


def lbs_transfer(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,
    lbs_weights, pose2rot=True, dtype=torch.float32):
    batch_size = max(betas.shape[0], pose.shape[0])
    device = betas.device

    # Add shape contribution
    v_shaped = v_template + blend_shapes(betas, shapedirs)

    # Get the joints
    # NxJx3 array
    J = vertices2joints(J_regressor, v_shaped)

    # 3. Add pose blend shapes
    # N x J x 3 x 3
    if pose2rot:
        rot_mats = rotation_6d_to_matrix(pose.view(-1, 6)).view([batch_size, -1, 3, 3])

    else:
        rot_mats = pose.view(batch_size, -1, 3, 3)

    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)
    return A


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

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(
        torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])

    return posed_joints, rel_transforms


```

## Source file: dataset/flame_dataset.py

{
  "path": "dataset/flame_dataset.py",
  "blob_sha": "88f519721505f8948b8456dea4872151b61dfaff",
  "sha256": "b59f6dda46be1b1a2869a4e679ea4f0633981bebdbc7126b3d08e05c8a370ceb",
  "lines": 194,
  "bytes": 8501,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 15,
    "blendshape": 1,
    "jaw": 7,
    "flame": 30
  },
  "url": "https://github.com/gapszju/RGBAvatar/blob/903653b689a8b860df30f165e242ccd6ed3f5583/dataset/flame_dataset.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def __init__(self,"
    },
    {
      "line": 55,
      "text": "def __len__(self):"
    },
    {
      "line": 58,
      "text": "def get_image(self, index):"
    },
    {
      "line": 66,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 78,
      "text": "def get_roll_yaw_pitch(self, index):"
    },
    {
      "line": 84,
      "text": "def load_images(self):"
    },
    {
      "line": 93,
      "text": "def load_camera_params(self, path: str):"
    },
    {
      "line": 106,
      "text": "def load_track_params(self):"
    },
    {
      "line": 153,
      "text": "def transform_flame_params(self, flame_model: FLAME):"
    },
    {
      "line": 167,
      "text": "def precompute_bind_mesh(self, flame_model: FLAME):"
    },
    {
      "line": 180,
      "text": "def precompute_blend_weight(self, use_shape_weight: bool, use_pose_weight: bool):"
    }
  ]
}

```text

import os
import torch
import numpy as np
from PIL import Image
from tqdm import tqdm
from torch.utils.data import Dataset
from roma import rotmat_to_euler
from submodules.flame import FLAME
from utils import rotation_6d_to_matrix


class FLAMEDataset(Dataset):
    def __init__(self, 
        flame_model: FLAME,
        path: str, 
        split: str = 'train', 
        use_shape_weight: bool = True,
        use_pose_weight: bool = True,
        preload: bool = False,
        pin_memory: bool = False,
        retarget_neck: bool = True,
        HQ: bool = False
    ):
        self.preload = preload
        self.pin_memory = pin_memory
        self.HQ = HQ
        if self.HQ: image_folder_path = os.path.join(path, "images.HQ")
        else: image_folder_path = os.path.join(path, "images")
        # image_folder_path = os.path.join(path, "matted")
        flame_folder_path = os.path.join(path, "checkpoint")

        self.image_paths = [os.path.join(image_folder_path, f) for f in sorted(os.listdir(image_folder_path))]
        self.flame_param_paths = [os.path.join(flame_folder_path, f) for f in sorted(os.listdir(flame_folder_path))]

        if split == 'train':
            self.image_paths = self.image_paths[:-350]
            self.flame_param_paths = self.flame_param_paths[:-350]
        elif split == 'test':
            self.image_paths = self.image_paths[-350:]
            self.flame_param_paths = self.flame_param_paths[-350:]
        elif split == 'all':
            pass
        else:
            raise ValueError("Invalid split")

        self.load_camera_params(flame_folder_path) # caution: use frame 0 camera pose
        self.load_track_params()
        if retarget_neck:
            self.transform_flame_params(flame_model) # retarget neck rotation to global rotation
        self.precompute_bind_mesh(flame_model)
        self.precompute_blend_weight(use_shape_weight, use_pose_weight)
        if preload: self.load_images()
        else: self.images = None

    def __len__(self):
        return len(self.image_paths)

    def get_image(self, index):
        if self.images is None:
            image_np = np.array(Image.open(self.image_paths[index]))
        else:
            image_tf = self.images[index]
            image_np = image_tf.permute(1, 2, 0).numpy()
        return image_np
    
    def __getitem__(self, index):
        if self.images is None:
            image_np = np.array(Image.open(self.image_paths[index]))
            image_tf = torch.from_numpy(image_np).permute(2, 0, 1)
        else:
            image_tf = self.images[index]
        return {
            'image': image_tf,
            'mesh': self.mesh_verts[index],
            'blend_weight': self.blend_weight[index]
        }
    
    def get_roll_yaw_pitch(self, index):
        global_rots = self.neck_poses if self.pose_transformed else self.global_rots
        global_rot = rotmat_to_euler("zyx", global_rots[index], degrees=True)
        roll, yaw, pitch = global_rot[0], global_rot[1], global_rot[2]
        return roll, yaw, pitch
    
    def load_images(self):
        images = []
        for img_path in tqdm(self.image_paths, desc="Loading images"):
            img = np.array(Image.open(img_path))
            images.append(img)
        self.images = torch.from_numpy(np.stack(images, axis=0)).permute(0, 3, 1, 2).contiguous() # caution: pre contiguous here
        if self.pin_memory: self.images = self.images.pin_memory() # use pinned memory
        print("Load images")

    def load_camera_params(self, path: str):
        flame_data = torch.load(os.path.join(path, "00000.frame"), weights_only=False)
        self.camera_intri = flame_data['opencv']['K'][0]
        self.camera_extri = np.diag([1.0, -1.0, -1.0, 1.0]) # adjust the camera coordinate
        self.camera_extri[:3,  3] = flame_data['opencv']['t'][0]
        self.image_width = flame_data['img_size'][0]
        self.image_height = flame_data['img_size'][1]
        print("Load camera params with frame 0")
        if self.HQ:
            self.camera_intri[0:2] *= 2
            self.image_width *= 2
            self.image_height *= 2

    def load_track_params(self):
        shapes = []
        exps = []
        jaw_poses = []
        eye_poses = []
        eyelid_params = []
        global_rots = []
        global_transls = []

        inv_cam_extri = np.linalg.inv(self.camera_extri)
        for flame_param_path in tqdm(self.flame_param_paths, desc="Loading flame params"):
            flame_data = torch.load(flame_param_path, weights_only=False)
            jaw_pose = torch.from_numpy(flame_data['flame']['jaw']).to(torch.float64)
            eye_pose = torch.from_numpy(flame_data['flame']['eyes']).to(torch.float64)
            eyelid_param = torch.from_numpy(flame_data['flame']['eyelids']).to(torch.float64)
            shape = torch.from_numpy(flame_data['flame']['shape']).to(torch.float64)
            exp = torch.from_numpy(flame_data['flame']['exp']).to(torch.float64)

            transform_mat = np.eye(4)
            transform_mat[:3, :3] = flame_data['opencv']['R'][0]
            transform_mat[:3,  3] = flame_data['opencv']['t'][0]
            transform_mat = inv_cam_extri @ transform_mat # transform under frame0
            global_rot = torch.from_numpy(transform_mat[:3, :3]).unsqueeze(0)
            global_transl = torch.from_numpy(transform_mat[:3,  3]).unsqueeze(0)

            shapes.append(shape)
            exps.append(exp)
            jaw_poses.append(jaw_pose)
            eye_poses.append(eye_pose)
            eyelid_params.append(eyelid_param)
            global_rots.append(global_rot)
            global_transls.append(global_transl)
        
        self.shapes = torch.cat(shapes, dim=0)
        self.exps = torch.cat(exps, dim=0)
        self.eyelid_params = torch.cat(eyelid_params, dim=0)
        self.global_rots = torch.cat(global_rots, dim=0)
        self.global_transls = torch.cat(global_transls, dim=0)

        jaw_poses = torch.cat(jaw_poses, dim=0)
        eye_poses = torch.cat(eye_poses, dim=0)
        self.jaw_poses = rotation_6d_to_matrix(jaw_poses.cuda()).cpu()
        self.eye_poses = rotation_6d_to_matrix(eye_poses.reshape(-1, 2, 6).cuda()).cpu()
        self.neck_poses = torch.eye(3, dtype=torch.float64).unsqueeze(0).repeat(self.exps.shape[0], 1, 1)
        self.pose_transformed = False
        print("Load FLAME params")

    def transform_flame_params(self, flame_model: FLAME):
        shapes = self.shapes.cuda()
        exps = self.exps.cuda()

        joints = flame_model.compute_joint_location(shapes, exps)
        neck_joints = joints[:, 1]
        global_transls = torch.matmul(self.global_rots.cuda(), neck_joints.unsqueeze(-1)).squeeze(-1) - neck_joints
        global_transls += self.global_transls.cuda()

        self.global_transls = global_transls.cpu()
        self.global_rots, self.neck_poses = self.neck_poses, self.global_rots
        self.pose_transformed = True
        print("Transform FLAME params")

    def precompute_bind_mesh(self, flame_model: FLAME):
        verts = flame_model( # blendshape & articulate transform
            shape_params=self.shapes.cuda(),
            expression_params=self.exps.cuda(),
            neck_pose_params=self.neck_poses.cuda(),
            jaw_pose_params=self.jaw_poses.cuda(),
            eye_pose_params=self.eye_poses.cuda(),
            eyelid_params=self.eyelid_params.cuda()
        )
        verts = torch.matmul(verts, self.global_rots.transpose(-1, -2).cuda()) # global transform
        verts += self.global_transls.unsqueeze(1).cuda()
        self.mesh_verts = verts.to(dtype=torch.float32, device='cpu')

    def precompute_blend_weight(self, use_shape_weight: bool, use_pose_weight: bool):
        poses = self.neck_poses if self.pose_transformed else self.global_rots
        # shape_weight = self.exps[:, :50]
        shape_weight = self.exps[:, :100] # extra exp coeffs
        pose_weight = torch.cat([
            poses[:, :2, :].reshape(-1, 6), # matrix to rot6d
            self.jaw_poses[:, :2, :].reshape(-1, 6), # matrix to rot6d
            self.eye_poses[:, :, :2, :].reshape(-1, 12), # matrix to rot6d
            self.eyelid_params,
            self.global_transls
        ], dim=1)
        blend_weights = []
        if use_shape_weight: blend_weights.append(shape_weight)
        if use_pose_weight: blend_weights.append(pose_weight)
        self.blend_weight = torch.cat(blend_weights, dim=1).to(dtype=torch.float32, device='cpu')


```

## Source file: submodules/flame/flame.py

{
  "path": "submodules/flame/flame.py",
  "blob_sha": "bf48125a6d06ac72dfd339ef496db142d6d07812",
  "sha256": "155248608f75ced0195f30cde6278b5c85cdbd17e482ec980db24772c39dc7f0",
  "lines": 457,
  "bytes": 20154,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 23,
    "skin": 1,
    "blend_shape": 2,
    "jaw": 7,
    "mouth": 6,
    "teeth": 45,
    "flame": 20,
    "deform": 1,
    "lbs": 11,
    "rig": 19,
    "face": 25,
    "lip": 10
  },
  "url": "https://github.com/gapszju/RGBAvatar/blob/903653b689a8b860df30f165e242ccd6ed3f5583/submodules/flame/flame.py",
  "function_signature_candidates": [
    {
      "line": 59,
      "text": "def rotation_6d_to_matrix(d6: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 83,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 88,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 95,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 100,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 116,
      "text": "def __init__(self, config):"
    },
    {
      "line": 174,
      "text": "def add_teeth(self):"
    },
    {
      "line": 324,
      "text": "def _vertices2landmarks(self, vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 354,
      "text": "def forward(self,"
    },
    {
      "line": 408,
      "text": "def compute_transfer(self,"
    },
    {
      "line": 439,
      "text": "def compute_keypoints(self, vertices: torch.Tensor):"
    },
    {
      "line": 449,
      "text": "def compute_joint_location(self, shape_params: torch.Tensor, expression_params: torch.Tensor):"
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

from typing import Optional

import pickle
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

from .lbs import lbs, blend_shapes, vertices2joints, lbs_transfer


mouth_v = torch.tensor([
    [0.034501, -0.07, 0.029227],
    [-0.034501, -0.07, 0.029227],
    [0.034501, -0.04, 0.029227],
    [-0.034501, -0.04, 0.029227]
], dtype=torch.float32)

mouth_uv = torch.tensor([
    [0.750000, 0.955000],
    [0.250000, 0.995000],
    [0.250000, 0.955000],
    [0.750000, 0.995000]
    # [0.750000, 0.955000],
    # [0.250000, 0.955000],
    # [0.750000, 0.995000],
    # [0.250000, 0.995000]
], dtype=torch.float32)

mouth_faces = torch.tensor([
    [3, 2, 1],
    [3, 4, 2]
], dtype=torch.int64) + 5022

mouth_uv_faces = torch.tensor([
    [1, 2, 3],
    [1, 4, 2]
    # [3, 2, 1],
    # [3, 4, 2]
], dtype=torch.int64) + 5117


def rotation_6d_to_matrix(d6: torch.Tensor) -> torch.Tensor:
    """
    Converts 6D rotation representation by Zhou et al. [1] to rotation matrix
    using Gram--Schmidt orthogonalization per Section B of [1].
    Args:
        d6: 6D rotation representation, of size (*, 6)

    Returns:
        batch of rotation matrices of size (*, 3, 3)

    [1] Zhou, Y., Barnes, C., Lu, J., Yang, J., & Li, H.
    On the Continuity of Rotation Representations in Neural Networks.
    IEEE Conference on Computer Vision and Pattern Recognition, 2019.
    Retrieved from http://arxiv.org/abs/1812.07035
    """

    a1, a2 = d6[..., :3], d6[..., 3:]
    b1 = F.normalize(a1, dim=-1)
    b2 = a2 - (b1 * a2).sum(-1, keepdim=True) * b1
    b2 = F.normalize(b2, dim=-1)
    b3 = torch.cross(b1, b2, dim=-1)
    return torch.stack((b1, b2, b3), dim=-2)


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
        print(f"[FLAME] Creating the 3DMM from {config.flame_geom_path}")
        with open(config.flame_geom_path, 'rb') as f:
            ss = pickle.load(f, encoding='latin1')
            self.flame_model = Struct(**ss)

        self.config = config
        dtype = config.dtype

        verts = torch.from_numpy(np.array(self.flame_model.v_template)).to(dtype)
        self.register_buffer('v_template', verts)
        self.register_buffer('mouth_v', mouth_v.to(dtype))

        faces = torch.from_numpy(np.array(self.flame_model.f).astype(np.int64)).to(torch.int64)
        self.register_buffer('faces', faces)

        # The shape components and expression
        shapedirs = torch.from_numpy(np.array(self.flame_model.shapedirs)).to(dtype)
        shapedirs = torch.cat([shapedirs[:, :, :config.num_shape_params], shapedirs[:, :, 300:300 + config.num_exp_params]], 2)
        self.register_buffer('shapedirs', shapedirs)
        # The pose components
        num_pose_basis = self.flame_model.posedirs.shape[-1]
        posedirs = np.reshape(self.flame_model.posedirs, [-1, num_pose_basis]).T
        self.register_buffer('posedirs', torch.from_numpy(np.array(posedirs)).to(dtype))
        #
        self.register_buffer('J_regressor', torch.from_numpy(np.array(self.flame_model.J_regressor.todense())).to(dtype))
        parents = torch.from_numpy(np.array(self.flame_model.kintree_table[0])).to(torch.long)
        parents[0] = -1
        self.register_buffer('parents', parents)
        self.register_buffer('lbs_weights', torch.from_numpy(np.array(self.flame_model.weights)).to(dtype))

        # Eyelid data
        self.register_buffer('l_eyelid', torch.from_numpy(np.load(config.l_eyelid_path)).to(dtype)[None])
        self.register_buffer('r_eyelid', torch.from_numpy(np.load(config.r_eyelid_path)).to(dtype)[None])

        # UV data
        flame_uv_data = np.load(config.flame_uv_path)
        uvs = torch.from_numpy(np.array(flame_uv_data['vt'])).to(dtype)
        self.register_buffer("uvs", uvs)
        uv_faces = torch.from_numpy(np.array(flame_uv_data['ft']).astype(np.int64)).to(torch.int64)
        self.register_buffer("uv_faces", uv_faces)

        # Landmark embeddings
        # lmk_embeddings = np.load(config.flame_lmk_path, allow_pickle=True, encoding='latin1')
        # lmk_embeddings = lmk_embeddings[()]
        # self.register_buffer('lmk_faces_idx', torch.from_numpy(np.array(lmk_embeddings['static_lmk_faces_idx'])).to(torch.int64))
        # self.register_buffer('lmk_bary_coords', torch.from_numpy(np.array(lmk_embeddings['static_lmk_bary_coords'])).to(dtype))
        # self.register_buffer('dynamic_lmk_faces_idx', torch.from_numpy(np.array(lmk_embeddings['dynamic_lmk_faces_idx'])).to(torch.int64))
        # self.register_buffer('dynamic_lmk_bary_coords', torch.from_numpy(np.array(lmk_embeddings['dynamic_lmk_bary_coords'])).to(dtype))

        if self.config.add_mouth: self.add_teeth()

        self.num_vertices = self.v_template.shape[0]
        self.num_faces = self.faces.shape[0]
        self.num_blend_basis = 129


    def add_teeth(self):
        device = self.v_template.device
        dtype = self.v_template.dtype

        vid_lip_outside_ring_upper = torch.tensor([
            1713, 1715, 1716, 1735, 1696, 1694, 1657, 3543, 2774, 2811, 2813, 2850, 2833, 2832, 2830
        ], device=device, dtype=torch.int64)
        vid_lip_outside_ring_lower = torch.tensor([
            1576, 1577, 1773, 1774, 1795, 1802, 1865, 3503, 2948, 2905, 2898, 2881, 2880, 2713, 2712
        ], device=device, dtype=torch.int64)

        v_lip_upper = self.v_template[vid_lip_outside_ring_upper]
        v_lip_lower = self.v_template[vid_lip_outside_ring_lower]

        # construct vertices for teeth
        mean_dist = (v_lip_upper - v_lip_lower).norm(dim=-1, keepdim=True).mean()
        v_teeth_middle = (v_lip_upper + v_lip_lower) / 2
        v_teeth_middle[:, 1] = v_teeth_middle[:, 1:2].mean(dim=0, keepdim=True)
        v_teeth_middle[:, 2] -= mean_dist * 2.0  # how far the teeth are from the lips

        # upper, front
        v_teeth_upper_edge = v_teeth_middle.clone() - torch.tensor([[0, mean_dist, 0]], device=device, dtype=dtype) * 1.0
        v_teeth_upper_root = v_teeth_upper_edge + torch.tensor([[0, mean_dist, 0]], device=device, dtype=dtype) * 1.0  # scale the height of teeth

        # lower, front
        v_teeth_lower_edge = v_teeth_middle.clone() + torch.tensor([[0, mean_dist, 0]], device=device, dtype=dtype) * 1.0
        v_teeth_lower_edge -= torch.tensor([[0, 0, mean_dist]], device=device, dtype=dtype) * 0.4  # slightly move the lower teeth to the back
        v_teeth_lower_root = v_teeth_lower_edge - torch.tensor([[0, mean_dist, 0]], device=device, dtype=dtype) * 1.0  # scale the height of teeth

        # concatenate to v_template
        num_verts_orig = self.v_template.shape[0]
        v_teeth = torch.cat([
            v_teeth_upper_root,  # num_verts_orig + 0-14 
            v_teeth_lower_root,  # num_verts_orig + 15-29
            v_teeth_upper_edge,  # num_verts_orig + 30-44
            v_teeth_lower_edge,  # num_verts_orig + 45-59
        ], dim=0)
        num_verts_teeth = v_teeth.shape[0]
        self.v_template = torch.cat([self.v_template, v_teeth], dim=0)

        vid_teeth_upper_root = torch.arange(0, 15, device=device, dtype=torch.int64) + num_verts_orig
        vid_teeth_lower_root = torch.arange(15, 30, device=device, dtype=torch.int64) + num_verts_orig
        vid_teeth_upper_edge = torch.arange(30, 45, device=device, dtype=torch.int64) + num_verts_orig
        vid_teeth_lower_edge = torch.arange(45, 60, device=device, dtype=torch.int64) + num_verts_orig
        
        vid_teeth_upper = torch.cat([vid_teeth_upper_root, vid_teeth_upper_edge], dim=0)
        vid_teeth_lower = torch.cat([vid_teeth_lower_root, vid_teeth_lower_edge], dim=0)
        vid_teeth = torch.cat([vid_teeth_upper, vid_teeth_lower], dim=0)

        # shapedirs copy from lips
        self.shapedirs = torch.cat([self.shapedirs, torch.zeros_like(self.shapedirs[:num_verts_teeth])], dim=0)
        shape_dirs_mean = (
            self.shapedirs[vid_lip_outside_ring_upper, :, :self.config.num_shape_params] + 
            self.shapedirs[vid_lip_outside_ring_lower, :, :self.config.num_shape_params]
        ) / 2
        self.shapedirs[vid_teeth_upper_root, :, :self.config.num_shape_params] = shape_dirs_mean
        self.shapedirs[vid_teeth_lower_root, :, :self.config.num_shape_params] = shape_dirs_mean
        self.shapedirs[vid_teeth_upper_edge, :, :self.config.num_shape_params] = shape_dirs_mean
        self.shapedirs[vid_teeth_lower_edge, :, :self.config.num_shape_params] = shape_dirs_mean

        # eyelid set to zero
        self.r_eyelid = torch.cat([self.r_eyelid, torch.zeros_like(self.r_eyelid[:, :num_verts_teeth])], dim=1)
        self.l_eyelid = torch.cat([self.l_eyelid, torch.zeros_like(self.l_eyelid[:, :num_verts_teeth])], dim=1)

        # posedirs set to zero
        posedirs = self.posedirs.reshape(len(self.parents) - 1, 9, num_verts_orig, 3)  # (J*9, V*3) -> (J, 9, V, 3)
        posedirs = torch.cat([posedirs, torch.zeros_like(posedirs[:, :, :num_verts_teeth])], dim=2)  # (J, 9, V+num_verts_teeth, 3)
        self.posedirs = posedirs.reshape((len(self.parents) - 1) * 9, (num_verts_orig + num_verts_teeth) * 3)  # (J*9, (V+num_verts_teeth)*3)

        # J_regressor set to zero
        self.J_regressor = torch.cat([self.J_regressor, torch.zeros_like(self.J_regressor[:, :num_verts_teeth])], dim=1)  # (5, J) -> (5, J+num_verts_teeth)

        # lbs_weights manually set
        self.lbs_weights = torch.cat([self.lbs_weights, torch.zeros_like(self.lbs_weights[:num_verts_teeth])], dim=0)  # (V, 5) -> (V+num_verts_teeth, 5)
        self.lbs_weights[vid_teeth_upper, 1] += 1  # move with neck
        self.lbs_weights[vid_teeth_lower, 2] += 1  # move with jaw

        # add faces for teeth
        f_teeth_upper = torch.tensor([
            [0, 31, 30],  #0
            [0, 1, 31],  #1
            [1, 32, 31],  #2
            [1, 2, 32],  #3
            [2, 33, 32],  #4
            [2, 3, 33],  #5
            [3, 34, 33],  #6
            [3, 4, 34],  #7
            [4, 35, 34],  #8
            [4, 5, 35],  #9
            [5, 36, 35],  #10
            [5, 6, 36],  #11
            [6, 37, 36],  #12
            [6, 7, 37],  #13
            [7, 8, 37],  #14
            [8, 38, 37],  #15
            [8, 9, 38],  #16
            [9, 39, 38],  #17
            [9, 10, 39],  #18
            [10, 40, 39],  #19
            [10, 11, 40],  #20
            [11, 41, 40],  #21
            [11, 12, 41],  #22
            [12, 42, 41],  #23
            [12, 13, 42],  #24
            [13, 43, 42],  #25
            [13, 14, 43],  #26
            [14, 44, 43],  #27
        ], device=device, dtype=torch.int64)
        f_teeth_lower = torch.tensor([
            [45, 46, 15],  # 28           
            [46, 16, 15],  # 29
            [46, 47, 16],  # 30
            [47, 17, 16],  # 31
            [47, 48, 17],  # 32
            [48, 18, 17],  # 33
            [48, 49, 18],  # 34
            [49, 19, 18],  # 35
            [49, 50, 19],  # 36
            [50, 20, 19],  # 37
            [50, 51, 20],  # 38
            [51, 21, 20],  # 39
            [51, 52, 21],  # 40
            [52, 22, 21],  # 41
            [52, 23, 22],  # 42
            [52, 53, 23],  # 43
            [53, 24, 23],  # 44
            [53, 54, 24],  # 45
            [54, 25, 24],  # 46
            [54, 55, 25],  # 47
            [55, 26, 25],  # 48
            [55, 56, 26],  # 49
            [56, 27, 26],  # 50
            [56, 57, 27],  # 51
            [57, 28, 27],  # 52
            [57, 58, 28],  # 53
            [58, 29, 28],  # 54
            [58, 59, 29],  # 55
        ], device=device, dtype=torch.int64)
        self.faces = torch.cat([self.faces, f_teeth_upper + num_verts_orig, f_teeth_lower + num_verts_orig], dim=0)

        # add uv for teeth
        num_verts_uv_orig = self.uvs.shape[0]
        u = torch.linspace(0.75, 0.25, 15, device=device, dtype=dtype)
        v = torch.linspace(0.995, 0.955, 3, device=device, dtype=dtype)
        v = v[[0, 2, 1, 1]]
        uv = torch.stack(torch.meshgrid(u, v, indexing='ij'), dim=-1).permute(1, 0, 2).reshape(num_verts_teeth, 2)
        self.uvs = torch.cat([self.uvs, uv], dim=0)
        self.uv_faces = torch.cat([self.uv_faces, f_teeth_upper + num_verts_uv_orig, f_teeth_lower + num_verts_uv_orig], dim=0)


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

    def forward(self, 
        shape_params: torch.Tensor, 
        expression_params: torch.Tensor, 
        neck_pose_params: torch.Tensor,
        jaw_pose_params: torch.Tensor, 
        eye_pose_params: torch.Tensor, 
        eyelid_params: Optional[torch.Tensor] = None,
        root_pose_params: Optional[torch.Tensor] = None,
        static_offsets: Optional[torch.Tensor] = None
    ):
        batch_size = shape_params.shape[0]

        if root_pose_params is None:
            root_pose_params = torch.eye(3, device=shape_params.device, dtype=self.v_template.dtype).unsqueeze(0).unsqueeze(0).repeat(batch_size, 1, 1, 1)

        # Concatenate identity shape and expression parameters
        betas = torch.cat([shape_params, expression_params], dim=1)

        # The pose vector contains global rotation, and neck, jaw, and eyeball rotations
        # full_pose = torch.cat([rot_params, neck_pose_params, jaw_pose_params, eye_pose_params], dim=1)
        full_pose = torch.cat([
            root_pose_params.reshape(-1, 1, 3, 3), 
            neck_pose_params.reshape(-1, 1, 3, 3),
            jaw_pose_params.reshape(-1, 1, 3, 3), 
            eye_pose_params.reshape(-1, 2, 3, 3)
        ], dim=1)

        # FLAME models shape and expression deformations as vertex offset from the mean face in 'zero pose', called v_template
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)

        if static_offsets is not None:
            template_vertices = template_vertices.contiguous()
            template_vertices[:, :5023] += static_offsets[:, :5023]

        # add eyelid before LBS
        if eyelid_params is not None:
            template_vertices = template_vertices + self.r_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 1:2, None]
            template_vertices = template_vertices + self.l_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 0:1, None]

        # Use linear blendskinning to model pose roations
        vertices, J = lbs(betas, full_pose, template_vertices,
                          self.shapedirs, self.posedirs,
                          self.J_regressor, self.parents,
                          self.lbs_weights, dtype=self.v_template.dtype,
                          pose2rot=False)

        # FIXME
        # if eyelid_params is not None:
        #     vertices = vertices + self.r_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 1:2, None]
        #     vertices = vertices + self.l_eyelid.expand(batch_size, -1, -1) * eyelid_params[:, 0:1, None]

        return vertices
    

    def compute_transfer(self,
        shape_params: torch.Tensor, 
        expression_params: torch.Tensor, 
        neck_pose_params: torch.Tensor,
        jaw_pose_params: torch.Tensor, 
        eye_pose_params: torch.Tensor, 
        eyelid_params: torch.Tensor
    ):
        batch_size = shape_params.shape[0]
        I = torch.eye(3, device=shape_params.device, dtype=self.v_template.dtype).unsqueeze(0).unsqueeze(0).repeat(batch_size, 1, 1, 1)

        betas = torch.cat([shape_params, expression_params], dim=1)

        full_pose = torch.cat([
            I, 
            neck_pose_params.reshape(-1, 1, 3, 3),
            jaw_pose_params.reshape(-1, 1, 3, 3), 
            eye_pose_params.reshape(-1, 2, 3, 3)
        ], dim=1)

        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)

        A = lbs_transfer(betas, full_pose, template_vertices,
            self.shapedirs, self.posedirs,
            self.J_regressor, self.parents,
            self.lbs_weights, dtype=self.v_template.dtype,
            pose2rot=False
        )
        return A


    def compute_keypoints(self, vertices: torch.Tensor):
        batch_size = vertices.shape[0]
        lmk_faces_idx = torch.cat([self.dynamic_lmk_faces_idx[0], self.lmk_faces_idx], 0)
        lmk_bary_coords = torch.cat([self.dynamic_lmk_bary_coords[0], self.lmk_bary_coords], 0)
        lmk_faces_idx = lmk_faces_idx.unsqueeze(dim=0).expand(batch_size, -1).contiguous()
        lmk_bary_coords = lmk_bary_coords.unsqueeze(dim=0).expand(batch_size, -1, -1).contiguous()
        lmk68 = self._vertices2landmarks(vertices, self.faces, lmk_faces_idx, lmk_bary_coords)
        return lmk68
    

    def compute_joint_location(self, shape_params: torch.Tensor, expression_params: torch.Tensor):
        betas = torch.cat([shape_params, expression_params], dim=1)
        template_vertices = self.v_template.unsqueeze(0).expand(betas.shape[0], -1, -1)
        v_shaped = template_vertices + blend_shapes(betas, self.shapedirs)
        J = vertices2joints(self.J_regressor, v_shaped)
        return J





```

## Source file: submodules/diff-gaussian-rasterization/cuda_utils/face_tbn.cu

{
  "path": "submodules/diff-gaussian-rasterization/cuda_utils/face_tbn.cu",
  "blob_sha": "52227cf35675f0e9bc9d5e6d8a15a2a97b883da2",
  "sha256": "6d180281a43c111d3b36d0f8487a79f5f417d9a15ff1cdb9d83d2fbf81c6bb60",
  "lines": 56,
  "bytes": 1612,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 16
  },
  "url": "https://github.com/gapszju/RGBAvatar/blob/903653b689a8b860df30f165e242ccd6ed3f5583/submodules/diff-gaussian-rasterization/cuda_utils/face_tbn.cu",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "__global__ void ComputeFaceTBNCuda("
    }
  ]
}

```text

#include "utils.h"
#include <cooperative_groups.h>
namespace cg = cooperative_groups;


__global__ void ComputeFaceTBNCuda(
    const int B, const int F,
    const glm::vec3* face_verts, // [B, F, 3, 3]
    const glm::vec2* face_uvs,   // [F, 3, 2]
    glm::mat3* TBNs
) {
    auto idx = cg::this_grid().thread_rank();
    if (idx >= B * F)
        return;

    glm::vec3 v0 = face_verts[3 * idx];
    glm::vec3 v1 = face_verts[3 * idx + 1];
    glm::vec3 v2 = face_verts[3 * idx + 2];

    int face_index = idx % F;
    glm::vec2 uv0 = face_uvs[3 * face_index];
    glm::vec2 uv1 = face_uvs[3 * face_index + 1];
    glm::vec2 uv2 = face_uvs[3 * face_index + 2];

    glm::vec3 edge1 = v1 - v0;
    glm::vec3 edge2 = v2 - v0;

    glm::vec2 deltaUV1 = uv1 - uv0;
    glm::vec2 deltaUV2 = uv2 - uv0;

    float f = 1.0f / (deltaUV1.x * deltaUV2.y - deltaUV1.y * deltaUV2.x);
    glm::vec3 normal = glm::cross(edge1, edge2);
    glm::vec3 tangent = (edge1 * deltaUV2.y - edge2 * deltaUV1.y) * f;
    glm::vec3 bitangent = (edge2 * deltaUV1.x - edge1 * deltaUV2.x) * f;

    normal = glm::normalize(normal);
    tangent = glm::normalize(tangent);
    bitangent = glm::normalize(bitangent);
    TBNs[idx] = glm::transpose(glm::mat3(tangent, bitangent, normal)); // glm uses column-major
}


void CudaUtils::ComputeFaceTBN(
    const int B, const int F,
    const float* face_verts,
    const float* face_uvs,
    float* TBNs
) {
    const int P = B * F;
    ComputeFaceTBNCuda<< <(P + 255) / 256, 256 >> >(
        B, F,
        (glm::vec3*)face_verts,
        (glm::vec2*)face_uvs,
        (glm::mat3*)TBNs
    );
}

```

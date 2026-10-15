# Facial rendering source scan: zjumsj/GaussianBlendshapes

Repository: https://github.com/zjumsj/GaussianBlendshapes

Pinned source revision: 501712451535b43d44ee7a37b266d2fd1f5e2a0a

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "submodules/diff-gaussian-rasterization/LICENSE.md"]

Topic paths: {"eye": ["FLAME/blendshapes/l_eyelid.npy", "FLAME/blendshapes/r_eyelid.npy"], "blendshape": ["FLAME/blendshapes/l_eyelid.npy", "FLAME/blendshapes/r_eyelid.npy", "config/config_blendshapes.py"], "mouth": ["FLAME/mouth_gs_model.py"], "flame": ["FLAME/FLAME.py", "FLAME/adamacc.py", "FLAME/blendshapes/l_eyelid.npy", "FLAME/blendshapes/r_eyelid.npy", "FLAME/dataset.py", "FLAME/dataset_dyn.py", "FLAME/dataset_nerfbs.py", "FLAME/face_gs_model.py", "FLAME/face_renderer.py", "FLAME/lbs.py", "FLAME/mediapipe/landmarks.py", "FLAME/mediapipe/mediapipe_landmark_embedding.npz", "FLAME/mediapipe/mediapipe_readme.txt", "FLAME/mouth_gs_model.py", "FLAME/rigid_transform_3D.py", "FLAME/rot.py", "FLAME/transforms.py", "FLAME/weight_volume.py", "data/FLAME2020/back_of_head.txt", "data/FLAME2020/neck_exclude_vertex.txt"], "lbs": ["FLAME/lbs.py"], "rig": ["FLAME/rigid_transform_3D.py"], "face": ["FLAME/face_gs_model.py", "FLAME/face_renderer.py"]}

## Source file: FLAME/lbs.py

{
  "path": "FLAME/lbs.py",
  "blob_sha": "9acd3f510c8909a31a2cadd7951bb82a1bd6c182",
  "sha256": "d0ff4e3843241d53a3bc2dcace1dcc42187aa9de8b89cbd7bd7651de7ef713d6",
  "lines": 476,
  "bytes": 17497,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 5,
    "skin": 3,
    "blend_shape": 6,
    "jaw": 2,
    "deform": 2,
    "lbs": 8,
    "rig": 17,
    "face": 19
  },
  "url": "https://github.com/zjumsj/GaussianBlendshapes/blob/501712451535b43d44ee7a37b266d2fd1f5e2a0a/FLAME/lbs.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def stereographic_unproject_old(a):"
    },
    {
      "line": 41,
      "text": "def stereographic_unproject(a, axis=None):"
    },
    {
      "line": 58,
      "text": "def rot_mat_to_euler(rot_mats):"
    },
    {
      "line": 67,
      "text": "def find_dynamic_lmk_idx_and_bcoords(vertices, pose, dynamic_lmk_faces_idx,"
    },
    {
      "line": 132,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 171,
      "text": "def lbs_trans(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 207,
      "text": "def lbs_jaw(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 243,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    },
    {
      "line": 331,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 351,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 375,
      "text": "def _batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 409,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 422,
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
from pytorch3d.transforms import rotation_6d_to_matrix


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

# get trans
def lbs_trans(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,
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
    ident = torch.eye(3, dtype=dtype, device=device)
    if pose2rot:
        # rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])
        rot_mats = rotation_6d_to_matrix(pose.view(-1, 6)).view([batch_size, -1, 3, 3])

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs).view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1), posedirs).view(batch_size, -1, 3)

    v_posed = pose_offsets + v_shaped
    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)
    # J_transformed 1x5x3
    # A 1x5x4x4
    return J_transformed, A

# get jaw trans
def lbs_jaw(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,
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
    ident = torch.eye(3, dtype=dtype, device=device)
    if pose2rot:
        # rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])
        rot_mats = rotation_6d_to_matrix(pose.view(-1, 6)).view([batch_size, -1, 3, 3])

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs).view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1), posedirs).view(batch_size, -1, 3)

    v_posed = pose_offsets + v_shaped
    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)

    final_trans = A[0,0] * 1.30787707e-02 + A[0,1] * 4.26467828e-02 + A[0,2] * 9.44269622e-01
    return final_trans

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
        pose_offsets = torch.matmul(pose_feature, posedirs).view(batch_size, -1, 3)
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

## Source file: FLAME/face_gs_model.py

{
  "path": "FLAME/face_gs_model.py",
  "blob_sha": "f94ae59e049e2caf1435dc19a6369d0b20b287cf",
  "sha256": "358f463b819f4ae288837f72ed4b424ad2546ade404515ff70b797ae227a670c",
  "lines": 1236,
  "bytes": 55364,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 32,
    "skin": 4,
    "blendshape": 38,
    "jaw": 7,
    "flame": 10,
    "lbs": 3,
    "shader": 3,
    "rig": 3,
    "face": 38
  },
  "url": "https://github.com/zjumsj/GaussianBlendshapes/blob/501712451535b43d44ee7a37b266d2fd1f5e2a0a/FLAME/face_gs_model.py",
  "function_signature_candidates": [
    {
      "line": 54,
      "text": "def __init__(self):"
    },
    {
      "line": 70,
      "text": "def setup_functions(self):"
    },
    {
      "line": 71,
      "text": "def build_covariance_from_scaling_rotation(scaling, scaling_modifier, rotation):"
    },
    {
      "line": 77,
      "text": "def inv_softplus(x, beta=1., thres=20.):"
    },
    {
      "line": 95,
      "text": "def __init__(self, sh_degree : int):"
    },
    {
      "line": 118,
      "text": "def capture(self):"
    },
    {
      "line": 143,
      "text": "def restore(self, model_args, training_args):"
    },
    {
      "line": 171,
      "text": "def sample_possion_points(self,v,f,N):"
    },
    {
      "line": 179,
      "text": "def get_processed(self):"
    },
    {
      "line": 200,
      "text": "def get_processed_acc(self):"
    },
    {
      "line": 217,
      "text": "def run_blendshape(self, xyz, params):"
    },
    {
      "line": 304,
      "text": "def prepare_xyz(self, params , args):"
    },
    {
      "line": 328,
      "text": "def prepare_merge(self, params):"
    },
    {
      "line": 333,
      "text": "def get_scaling(self):"
    },
    {
      "line": 342,
      "text": "def get_rotation(self):"
    },
    {
      "line": 346,
      "text": "def get_xyz(self):"
    },
    {
      "line": 351,
      "text": "def get_features(self):"
    },
    {
      "line": 355,
      "text": "def get_opacity(self):"
    },
    {
      "line": 366,
      "text": "def oneupSHdegree(self):"
    },
    {
      "line": 370,
      "text": "def render_shape(self, vertices, faces=None, white=True):"
    },
    {
      "line": 390,
      "text": "def setup_renderer(self, args, img_size):"
    },
    {
      "line": 410,
      "text": "def vis(self, params,  args, visualization): # []"
    },
    {
      "line": 450,
      "text": "def create_from_face(self, params, args, spatial_lr_scale: float, replace_v = True):"
    },
    {
      "line": 587,
      "text": "def training_setup(self, training_args, id = 0):"
    },
    {
      "line": 624,
      "text": "def update_learning_rate(self, iteration):"
    },
    {
      "line": 632,
      "text": "def construct_list_of_attributes(self):"
    },
    {
      "line": 646,
      "text": "def update_from_nn(self, update_acc_dict = False):"
    },
    {
      "line": 670,
      "text": "def compute_blendshape_init(self):"
    },
    {
      "line": 751,
      "text": "def save_acc_dict(self, filename):"
    },
    {
      "line": 758,
      "text": "def load_acc_dict(self, filename):"
    },
    {
      "line": 768,
      "text": "def save_npy_forviewer(self, path):"
    },
    {
      "line": 837,
      "text": "def reset_opacity(self):"
    },
    {
      "line": 842,
      "text": "def load_ply(self, path):"
    },
    {
      "line": 885,
      "text": "def replace_tensor_to_optimizer(self, tensor, name):"
    },
    {
      "line": 900,
      "text": "def _prune_optimizer(self, mask):"
    },
    {
      "line": 918,
      "text": "def prune_points(self, mask):"
    },
    {
      "line": 1002,
      "text": "def cat_tensors_to_optimizer(self, tensors_dict):"
    },
    {
      "line": 1024,
      "text": "def densification_postfix(self, new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation,"
    },
    {
      "line": 1123,
      "text": "def densify_and_split(self, grads, grad_threshold, scene_extent, N=2):"
    },
    {
      "line": 1173,
      "text": "def densify_and_clone(self, grads, grad_threshold, scene_extent):"
    },
    {
      "line": 1209,
      "text": "def densify_and_prune(self, max_grad, min_opacity, extent, max_screen_size):"
    },
    {
      "line": 1229,
      "text": "def add_densification_stats(self, viewspace_point_tensor, update_filter):"
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

# Blend Shape

import torch
from torch import nn
import torch.nn.functional as F

import numpy as np
from utils.general_utils import inverse_sigmoid, get_expon_lr_func, build_rotation

import os
from utils.system_utils import mkdir_p
from plyfile import PlyData, PlyElement
from simple_knn._C import distCUDA2
from utils.general_utils import strip_symmetric, build_scaling_rotation

from pytorch3d.io import load_obj

from pytorch3d.renderer import RasterizationSettings, PointLights, MeshRenderer, MeshRasterizer, TexturesVertex, SoftPhongShader, look_at_view_transform, PerspectiveCameras, BlendParams
from pytorch3d.structures import Meshes
from pytorch3d.transforms import matrix_to_rotation_6d, rotation_6d_to_matrix

#from .FLAME import FLAME, FLAMETex
from .FLAME import FLAME
from .rot import matrix_to_quaternion, quaternion_raw_multiply, quaternion_to_matrix
from diff_gaussian_rasterization import RotSH3_C3, CompositeBlendShape
#from diff_gaussian_rasterization import RotSH3_C3, RotSHBlendShape
from .transforms import normalize_transform
from .weight_volume import WeightVolume, WeightVolumeIndex
from .adamacc import AdamAcc

from enum import Enum

use_feature_expr_mask = True
use_rot_expr_mask = True
use_scale_expr_mask = True
use_opacity_expr_mask = True
use_xyz_expr_mask = True

use_trans_rot = True
use_trans_rot_sh = True

class DummyObj:
    def __init__(self):
        pass


class View(Enum):
    GROUND_TRUTH = 1
    COLOR_OVERLAY = 2
    SHAPE_OVERLAY = 4
    SHAPE = 8
    LANDMARKS = 16
    HEATMAP = 32
    DEPTH = 64
    MASK = 128

class GaussianModel:

    def setup_functions(self):
        def build_covariance_from_scaling_rotation(scaling, scaling_modifier, rotation):
            L = build_scaling_rotation(scaling_modifier * scaling, rotation)
            actual_covariance = L @ L.transpose(1, 2)
            symm = strip_symmetric(actual_covariance)
            return symm

        def inv_softplus(x, beta=1., thres=20.):
            y = beta * x
            unsafe_mask = y > thres
            safe_y = torch.where(unsafe_mask, torch.full_like(y, thres), y)  # avoid nan
            z = torch.where(unsafe_mask, x, torch.log(torch.exp(safe_y) - 1 + 1e-12))
            return z

        self.scaling_activation = torch.exp
        self.scaling_inverse_activation = torch.log

        self.covariance_activation = build_covariance_from_scaling_rotation

        self.opacity_activation = torch.sigmoid
        self.inverse_opacity_activation = inverse_sigmoid

        self.rotation_activation = torch.nn.functional.normalize


    def __init__(self, sh_degree : int):


        self.acc_dict = None
        #self.opt_id = 0

        self.active_sh_degree = 0
        self.max_sh_degree = sh_degree
        self._xyz = torch.empty(0)
        self._features_dc = torch.empty(0)
        self._features_rest = torch.empty(0)
        self._scaling = torch.empty(0)
        self._rotation = torch.empty(0)
        self._opacity = torch.empty(0)
        self.max_radii2D = torch.empty(0)
        self.xyz_gradient_accum = torch.empty(0)
        self.denom = torch.empty(0)
        self.optimizer = None
        self.percent_dense = 0
        self.spatial_lr_scale = 0

        self.setup_functions()

    def capture(self):
        return (
            self.active_sh_degree,
            self._xyz,
            self._features_dc,
            self._features_rest,
            self._scaling,
            self._rotation,
            self._opacity,
            self.max_radii2D,
            self.xyz_gradient_accum,
            self.denom,
            # new
            self._xyz_b,
            self._features_b,
            self._scaling_b,
            self._rotation_b,
            self._opacity_b,
            self.face_id,
            self.barycentric_coord,
            #
            self.optimizer.state_dict(),
            self.spatial_lr_scale,
        )

    def restore(self, model_args, training_args):
        (self.active_sh_degree, 
        self._xyz,
        self._features_dc, 
        self._features_rest,
        self._scaling, 
        self._rotation, 
        self._opacity,
        self.max_radii2D, 
        xyz_gradient_accum, 
        denom,
        # new
        self._xyz_b,
        self._features_b,
        self._scaling_b,
        self._rotation_b,
        self._opacity_b,
        self.face_id,
        self.barycentric_coord,
        #
        opt_dict,
        self.spatial_lr_scale) = model_args
        self.training_setup(training_args)
        self.xyz_gradient_accum = xyz_gradient_accum
        self.denom = denom
        #if self.optimizer is not None:
        #    self.optimizer.load_state_dict(opt_dict)

    def sample_possion_points(self,v,f,N):
        import point_cloud_utils as pcu
        f_i, bc = pcu.sample_mesh_poisson_disk(v,f,N)
        print('expect point number %d, get %d' % (N, bc.shape[0]))
        return f_i, bc

    ########################

    def get_processed(self):
        #----
        _rotation = self._rotation
        init_rot = self.acc_dict["init_rot"]
        _rotation = _rotation + torch.sum(init_rot * self._mf, dim=-1)
        _rotation_b = self._rotation_b
        if use_rot_expr_mask:
            _rotation_b = _rotation_b * self.expr_mask_tensor.unsqueeze(1)
        _rotation = _rotation + torch.sum(_rotation_b * self._mf, dim=-1)
        processed_rotation = self.rotation_activation(_rotation)
        #----
        _features = self.processed_features
        init_sh = self.acc_dict["init_sh"]
        _features = _features + torch.sum(init_sh * self._mf, dim=-1)
        _features_b = self._features_b
        if use_feature_expr_mask:
            _features_b = _features_b * self.expr_mask_tensor.unsqueeze(1).unsqueeze(1)
        _features = _features + torch.sum(_features_b * self._mf, dim=-1)
        processed_features = _features
        return processed_rotation, processed_features

    def get_processed_acc(self):
        #----
        _rotation = self._rotation
        _rotation_b = self._rotation_b
        init_rot = self.acc_dict["init_rot"]
        _mask = self.expr_mask_tensor if use_rot_expr_mask else None
        _rotation = CompositeBlendShape(_rotation, _rotation_b, self._mf, init_rot, _mask)
        processed_rotation = self.rotation_activation(_rotation)
        #----
        _features = self.processed_features
        _features_b = self._features_b
        init_sh = self.acc_dict["init_sh"]
        _mask = self.expr_mask_tensor if use_feature_expr_mask else None
        processed_features = CompositeBlendShape(_features, _features_b, self._mf, init_sh, _mask)
        return processed_rotation, processed_features


    def run_blendshape(self, xyz, params):

        shape_params = params.shape
        trans_params = None
        rot_params = None
        neck_pose_params = None
        jaw_pose_params = params.jaw
        eye_pose_params = params.eyes
        expression_params = params.exp
        eyelid_params = params.eyelids

        batch_size = shape_params.shape[0]
        I = matrix_to_rotation_6d(torch.cat([torch.eye(3)[None]] * batch_size, dim=0).cuda())

        if trans_params is None:
            trans_params = torch.zeros(batch_size, 3).cuda()
        if rot_params is None:
            rot_params = I.clone()
        if neck_pose_params is None:
            neck_pose_params = I.clone()
        if jaw_pose_params is None:
            jaw_pose_params = I.clone()
        if eye_pose_params is None:
            eye_pose_params = torch.cat([I.clone()] * 2, dim=1)

        betas = torch.cat([shape_params, expression_params], dim=1) # 300 + 100
        full_pose = torch.cat([rot_params, neck_pose_params, jaw_pose_params, eye_pose_params], dim=1)

        #######################
        batch_size = 1
        device = betas.device
        dtype = torch.float32
        A = params.A
        A_diff = params.A_diff

        v_biased = xyz
        #n_gs_point = v_biased.shape[0]

        # Compute expr_mask_tensor
        self.expr_mask_tensor = self.acc_dict["expr_mask_tensor"]

        xyz_blendshape = self.acc_dict["init_xyz"] # Fx3x{50}
        v_shaped = v_biased + (xyz_blendshape * betas[...,300:350]).sum(-1) # Px3

        _xyz_b = self._xyz_b # Px3x50
        if use_xyz_expr_mask:
            _xyz_b = _xyz_b * self.expr_mask_tensor.unsqueeze(1)
        v_shaped = v_shaped + (_xyz_b * betas[...,300:350]).sum(-1)

        #processed_rotation, processed_features = self.get_processed()
        processed_rotation, processed_features = self.get_processed_acc()

        # Add pose blend shapes
        # N x J x 3 x 3
        ident = torch.eye(3, dtype=dtype, device=device)
        rot_mats = rotation_6d_to_matrix(full_pose.view(-1, 6)).view([batch_size, -1, 3, 3])
        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])

        pose_f_tensor_ = self.acc_dict["pose_blendshape"]
        pose_f_tensor_ = pose_f_tensor_.view(36, -1)
        pose_offsets = torch.matmul(pose_feature, pose_f_tensor_).view(batch_size, -1, 3)
        v_posed = pose_offsets + v_shaped

        # fetch skinning weights
        W = self.acc_dict["W"]
        W = W.unsqueeze(dim=0).expand([batch_size, -1, -1])

        num_joints = W.shape[-1]
        T = torch.matmul(W, A.view(batch_size, num_joints, 16)).view(batch_size, -1, 4, 4)
        T_diff = torch.matmul(W, A_diff.view(batch_size, num_joints, 16)).view(batch_size, -1, 4, 4)

        homogen_coord = torch.ones([batch_size, v_posed.shape[1], 1], dtype=dtype, device=device)
        v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
        v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

        verts = v_homo[:, :, :3, 0]

        ### eyelid
        eye_blendshape = self.acc_dict["eye_blendshape"]
        # eyelid_params 1x2
        verts = verts + (eye_blendshape * eyelid_params).sum(-1)

        vertices = verts
        vertices = vertices + trans_params.unsqueeze(dim=1)
        return vertices, T_diff, processed_rotation, processed_features


    def prepare_xyz(self, params , args):

        features_dc = self._features_dc
        features_rest = self._features_rest
        _features = torch.cat((features_dc, features_rest), dim=1)
        self.processed_features = _features

        vertices, T_diff, processed_rotation, processed_features = self.run_blendshape(
            self._xyz, params
        )
        self.processed_xyz = vertices[0]
        self.processed_rotation = processed_rotation
        self.processed_features = processed_features

        ## Rot gaussians and SH according to LBS (linear blend skinning) transform.
        if use_trans_rot or use_trans_rot_sh:
            rot_apply = normalize_transform(T_diff[0, :, :3, :3])

        if use_trans_rot:
            rot_q = matrix_to_quaternion(rot_apply)
            self.processed_rotation = quaternion_raw_multiply(rot_q, self.processed_rotation)
        if use_trans_rot_sh:
            self.processed_features = RotSH3_C3(rot_apply, self.processed_features)

    def prepare_merge(self, params):
        merged_f = params.exp[...,:self.n_expr_params]
        self._mf = merged_f[0]

    @property
    def get_scaling(self):
        _scaling = self._scaling
        _scaling_b = self._scaling_b
        if use_scale_expr_mask:
            _scaling_b = _scaling_b * self.expr_mask_tensor.unsqueeze(1)
        _scaling = _scaling + torch.sum(_scaling_b * self._mf, dim=-1)
        return self.scaling_activation(_scaling)

    @property
    def get_rotation(self):
        return self.processed_rotation

    @property
    def get_xyz(self):
        # WARNING! YOU SHOULD CALL PREPARE_XYZ BEFORE THIS FUNCTION
        return self.processed_xyz

    @property
    def get_features(self):
        return self.processed_features

    @property
    def get_opacity(self):
        _opacity = self._opacity
        _opacity_b = self._opacity_b
        if use_opacity_expr_mask:
            _opacity_b = _opacity_b * self.expr_mask_tensor.unsqueeze(1)
        _opacity = _opacity + torch.sum(_opacity_b * self._mf, dim=-1)
        return self.opacity_activation(_opacity)

    # def get_covariance(self, scaling_modifier = 1):
    #     return self.covariance_activation(self.get_scaling, scaling_modifier, self._rotation)

    def oneupSHdegree(self):
        if self.active_sh_degree < self.max_sh_degree:
            self.active_sh_degree += 1

    def render_shape(self, vertices, faces=None, white=True):
        B = vertices.shape[0]
        V = vertices.shape[1]
        if faces is None:
            #faces = self.faces.verts_idx.cuda()[None].repeat(B, 1, 1)
            faces = self.faces[None].repeat(B,1,1)
        if not white:
            verts_rgb = torch.from_numpy(np.array([80, 140, 200]) / 255.).cuda().float()[None, None, :].repeat(B, V, 1)
        else:
            verts_rgb = torch.from_numpy(np.array([1.0, 1.0, 1.0])).cuda().float()[None, None, :].repeat(B, V, 1)
        textures = TexturesVertex(verts_features=verts_rgb.cuda())
        meshes_world = Meshes(verts=[vertices[i] for i in range(B)], faces=[faces[i] for i in range(B)], textures=textures)

        blend = BlendParams(background_color=(1.0, 1.0, 1.0))

        fragments = self.mesh_rasterizer(meshes_world, cameras=self.cameras)
        rendering = self.debug_renderer.shader(fragments, meshes_world, cameras=self.cameras, blend_params=blend)
        rendering = rendering.permute(0, 3, 1, 2).detach()
        return rendering[:, 0:3, :, :]

    def setup_renderer(self, args, img_size):
        raster_settings = RasterizationSettings(
            image_size=img_size,
            faces_per_pixel=1,
            cull_backfaces=True,
            perspective_correct=True
        )

        self.lights = PointLights(
            device=args.data_device,
            location=((0.0, 0.0, 5.0),),
            ambient_color=((0.5, 0.5, 0.5),),
            diffuse_color=((0.5, 0.5, 0.5),)
        )
        self.mesh_rasterizer = MeshRasterizer(raster_settings=raster_settings)
        self.debug_renderer = MeshRenderer(
            rasterizer=self.mesh_rasterizer,
            shader=SoftPhongShader(device=args.data_device, lights=self.lights)
        )

    def vis(self, params,  args, visualization): # []

        with torch.no_grad():
            self.cameras = params.cameras
            self.debug_renderer.rasterizer.raster_settings.image_size = params.image_size

            vertices, _, _ = self.flame(
                cameras=torch.inverse(params.cameras.R.to(args.data_device)),
                shape_params=params.shape,
                expression_params=params.exp,
                eye_pose_params=params.eyes, # 12
                jaw_pose_params=params.jaw, # 6
                eyelid_params=params.eyelids # 2
            )

            #albedos = self.albedos

            final_views = []
            for view in visualization:
                if view == View.COLOR_OVERLAY:
                    raise NotImplementedError
                if view == View.GROUND_TRUTH:
                    final_views.append(params.original_image.cpu().numpy())
                if view == View.MASK:
                    mask = params.mask.cpu().numpy()[...,None]
                    mask = np.repeat(mask,3,axis=-1)
                    final_views.append(mask)
                if view == View.SHAPE:
                    shape = self.render_shape(vertices, white=False)[0].cpu().numpy()
                    shape = np.transpose(shape,[1,2,0]) # CHW->HWC
                    final_views.append(shape)
                if view == View.LANDMARKS:
                    raise NotImplementedError
                if view == View.SHAPE_OVERLAY:
                    raise NotImplementedError
                if view == View.HEATMAP:
                    raise NotImplementedError

            return final_views

    def create_from_face(self, params, args, spatial_lr_scale: float, replace_v = True):

        self.spatial_lr_scale = spatial_lr_scale
        #fused_point_cloud = torch.tensor(np.asarray(pcd.points)).float().cuda()

        ## load canonical object
        mesh_file = args.flame_template_path #should be canonical mesh
        faces = load_obj(mesh_file) # Nx3 id int64
        canonical_verts = faces[0]
        self.faces = faces[1].verts_idx.to(args.data_device)

        ## load flame
        self.flame = FLAME(args).to(args.data_device)
        args.image_size = params.image_size
        args.actor = args.source_path
        #flametex = FLAMETex(args).to(args.data_device)
        #self.diff_renderer = Renderer(params.image_size, obj_fielname=mesh_file).to(self.data_device)

        ## load renderer
        self.setup_renderer(args, params.image_size)

        ## create init points

        if replace_v:
            from .transforms import get_rest_pose_vertices
            v = get_rest_pose_vertices(self,params,args).cpu().numpy()
        else:
            v = canonical_verts.cpu().numpy()
        f = faces[1].verts_idx.cpu().numpy()
        face_id, barycentric_coord = self.sample_possion_points(v,f, args.init_face_point_number)
        self.face_id = torch.from_numpy(face_id).to(args.data_device)
        self.barycentric_coord = torch.from_numpy(barycentric_coord).to(args.data_device)

        self.n_pose_params = 0
        self.n_expr_params = 50
        self.n_total_params = self.n_expr_params + self.n_pose_params

        with torch.no_grad():

            zeros_exp = torch.zeros_like(params.exp)
            zeros_eyes = torch.tensor([[1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0]], dtype=torch.float32).to(args.data_device)
            zeros_jaw = torch.tensor([[1, 0, 0, 0, 1, 0]], dtype=torch.float32).to(args.data_device)
            zeros_eyelids = torch.tensor([[0, 0]], dtype=torch.float32).to(args.data_device)

            vertices, _, _ = self.flame(
                cameras=torch.inverse(params.cameras.R.to(args.data_device)),
                shape_params=params.shape,
                expression_params=zeros_exp,
                eye_pose_params=zeros_eyes, # 12
                jaw_pose_params=zeros_jaw, # 6
                eyelid_params=zeros_eyelids # 2
            )

            #### Create grid for finding nearest triangle
            N_res1 = 256
            #N_res1 = 512
            path1 = os.path.join(args.source_path,"WeightID_%d.pt" % N_res1)
            if not os.path.exists(path1):
                print('gen weight volume id')
                path1 = None
            self.weight_volume_index = WeightVolumeIndex(
                path1,
                vertices[0],
                self.faces,
                N=N_res1
            )
            if path1 is None:
                print('done')
                path1 = os.path.join(args.source_path,"WeightID_%d.pt" % N_res1)
                self.weight_volume_index.dump(path1)

            #### Create grid for fetching LBS weights
            N_res2 = 256
            #N_res2 = 512
            path2 = os.path.join(args.source_path, "Weight5node_%d.pt" % N_res2)
            if not os.path.exists(path2):
                print('gen weight volume node')
                path2 = None
            self.weight_volume = WeightVolume(
                path2,
                vertices[0],
                self.flame.lbs_weights,
                self.faces,
                N=N_res2
            )
            if path2 is None:
                print('done')
                path2 = os.path.join(args.source_path, "Weight5node_%d.pt" % N_res2)
                self.weight_volume.dump(path2)

            ####

            vertices = vertices[0] # BxPx3 -> Px3

            p_face_id = self.faces[self.face_id]
            v_id_xyz = vertices[p_face_id]
            point_vertices = (v_id_xyz * self.barycentric_coord[..., None]).sum(-2)

            albedos = (torch.rand(point_vertices.shape[0],3)/255.).float().cuda()
            #fused_color = RGB2SH(albedos)
            fused_color = albedos
            features = torch.zeros((fused_color.shape[0], 3, (self.max_sh_degree + 1) ** 2)).float().cuda()
            features_b = torch.zeros((fused_color.shape[0], (self.max_sh_degree + 1) ** 2, 3, self.n_total_params)).float().cuda()
            features[:, :3, 0 ] = fused_color
            features[:, 3:, 1:] = 0.0

            #dist2 = torch.clamp_min(distCUDA2(torch.from_numpy(np.asarray(pcd.points)).float().cuda()), 0.0000001)
            dist2 = torch.clamp_min(distCUDA2(point_vertices), 0.0000001)
            scales = torch.log(torch.sqrt(dist2))[...,None].repeat(1, 3)
            rots = torch.zeros((point_vertices.shape[0], 4), device="cuda")
            rots[:, 0] = 1
            scales_b = torch.zeros(list(scales.shape) + [self.n_total_params],dtype=torch.float,device="cuda")
            rots_b = torch.zeros(list(rots.shape) + [self.n_total_params],dtype=torch.float,device="cuda")

            #opacities = inverse_sigmoid(0.1 * torch.ones((fused_point_cloud.shape[0], 1), dtype=torch.float, device="cuda"))
            opacities = inverse_sigmoid(0.1 * torch.ones((point_vertices.shape[0], 1), dtype=torch.float, device="cuda"))
            opacities_b = torch.zeros(list(opacities.shape)+[self.n_total_params],dtype=torch.float,device="cuda")

            xyz_b = torch.zeros((fused_color.shape[0], 3, self.n_total_params)).float().cuda()

        ## neutral model
        self._xyz = nn.Parameter(point_vertices.requires_grad_(True))
        self._features_dc = nn.Parameter(features[:,:,0:1].transpose(1, 2).contiguous().requires_grad_(True))
        self._features_rest = nn.Parameter(features[:,:,1:].transpose(1, 2).contiguous().requires_grad_(True))
        self._scaling = nn.Parameter(scales.requires_grad_(True))
        self._rotation = nn.Parameter(rots.requires_grad_(True))
        self._opacity = nn.Parameter(opacities.requires_grad_(True))
        self.max_radii2D = torch.zeros((self._scaling.shape[0]), device="cuda")
        ## blendshapes
        self._xyz_b = nn.Parameter(xyz_b.requires_grad_(True))
        self._features_b = nn.Parameter(features_b.requires_grad_(True))
        self._scaling_b = nn.Parameter(scales_b.requires_grad_(True))
        self._rotation_b = nn.Parameter(rots_b.requires_grad_(True))
        self._opacity_b = nn.Parameter(opacities_b.requires_grad_(True))

        #flametex = None

    def training_setup(self, training_args, id = 0):
        # id = 0 full params
        # id = 1 neutral head params only
        # id = 2 blendshape diff only
        self.percent_dense = training_args.percent_dense
        self.xyz_gradient_accum = torch.zeros((self._scaling.shape[0], 1), device="cuda")
        self.denom = torch.zeros((self._scaling.shape[0], 1), device="cuda")

        l = []
        if id == 0 or id == 1:
            l += [
                {'params': [self._xyz], 'lr': training_args.position_lr_init * self.spatial_lr_scale, "name": "xyz"},
                {'params': [self._features_dc], 'lr': training_args.feature_lr, "name": "f_dc"},
                {'params': [self._features_rest], 'lr': training_args.feature_lr / 20.0, "name": "f_rest"},
                {'params': [self._opacity], 'lr': training_args.opacity_lr, "name": "opacity"},
                {'params': [self._scaling], 'lr': training_args.scaling_lr, "name": "scaling"},
                {'params': [self._rotation], 'lr': training_args.rotation_lr, "name": "rotation"}
            ]

        if id == 0 or id == 2:
            weight_decay = training_args.weight_decay
            l += [
                {'params': [self._xyz_b], 'lr': 0.2 * 1.6e-6, "weight_decay": weight_decay, "name": "xyz_b"},
                {'params': [self._features_b], 'lr': 0.5 * training_args.feature_lr * training_args.basis_lr_decay, "weight_decay":weight_decay, "name": "f_b"},
                {'params': [self._opacity_b], 'lr': 0.001 * training_args.opacity_lr * training_args.basis_lr_decay, "weight_decay":weight_decay, "name": "opacity_b"},
                {'params': [self._scaling_b], 'lr': 0.1 * training_args.scaling_lr * training_args.basis_lr_decay, "weight_decay":weight_decay, "name": "scaling_b"},
                {'params': [self._rotation_b], 'lr': 0.1 * training_args.rotation_lr * training_args.basis_lr_decay, "weight_decay":weight_decay, "name": "rotation_b"}
            ]

        #self.opt_id = id
        #self.optimizer = torch.optim.Adam(l, lr=0.0, eps=1e-15)
        self.optimizer = AdamAcc(l, lr=0.0, eps=1e-15) # ACC_FLAG
        self.xyz_scheduler_args = get_expon_lr_func(lr_init=training_args.position_lr_init*self.spatial_lr_scale,
                                                    lr_final=training_args.position_lr_final*self.spatial_lr_scale,
                                                    lr_delay_mult=training_args.position_lr_delay_mult,
                                                    max_steps=training_args.position_lr_max_steps)

    def update_learning_rate(self, iteration):
        ''' Learning rate scheduling per step '''
        for param_group in self.optimizer.param_groups:
            if param_group["name"] == "xyz":
                lr = self.xyz_scheduler_args(iteration)
                param_group['lr'] = lr
                return lr

    def construct_list_of_attributes(self):
        l = ['x', 'y', 'z', 'nx', 'ny', 'nz']
        # All channels except the 3 DC
        for i in range(self._features_dc.shape[1]*self._features_dc.shape[2]):
            l.append('f_dc_{}'.format(i))
        for i in range(self._features_rest.shape[1]*self._features_rest.shape[2]):
            l.append('f_rest_{}'.format(i))
        l.append('opacity')
        for i in range(self._scaling.shape[1]):
            l.append('scale_{}'.format(i))
        for i in range(self._rotation.shape[1]):
            l.append('rot_{}'.format(i))
        return l

    def update_from_nn(self, update_acc_dict = False):

        v_biased = self._xyz
        # fetch skinning weights
        W = self.weight_volume(v_biased)
        W = W / torch.sum(W, dim=-1, keepdim=True)
        #W = W.unsqueeze(dim=0).expand([batch_size, -1, -1])

        # Compute expr_mask_tensor
        face_id, face_w = self.weight_volume_index(v_biased)  # Px3 -> Px8
        expr_mask_tensor_face = self.expr_mask_tensor_face[..., :50]  # Fx50
        face_id = face_id.long()
        tmp1 = expr_mask_tensor_face[face_id]  # Px8x50
        expr_mask_tensor = (tmp1 * face_w[..., None]).sum(1)

        if update_acc_dict:
            self.acc_dict.update({
                "expr_mask_tensor" : expr_mask_tensor.contiguous().detach(), # Px50?
                "W": W.contiguous().detach() # Px5
            })

        return expr_mask_tensor, W

    # Initialize difference of Gaussian blendshapes
    def compute_blendshape_init(self):
    #def extract_acc(self):
        # rot_tensor = Fx4x100
        # faceR_tensor = Fx100x3x3
        # facet_tensor = Fx100x3

        faceR_tensor = self.faceR_tensor
        facet_tensor = self.facet_tensor
        rot_tensor = self.rot_tensor
        pose_f_tensor = self.pose_f_tensor
        l_eyelid_offset = self.l_eyelid_offset
        r_eyelid_offset = self.r_eyelid_offset
        face_id = self.face_id # P

        v_biased = self._xyz

        ## Initialize rotation difference
        tmp1 = rot_tensor[face_id] # Px4x100
        tmp1 = tmp1[..., :50]
        tmp1_bak = tmp1 # Px4xN
        tmp1 = tmp1.permute(0, 2, 1)  # PxNx4
        _rotation = self._rotation  # Px4
        _rotation = torch.stack([_rotation] * 50, dim=-2)  # PxNx4
        _rot = quaternion_raw_multiply(tmp1, _rotation)
        _rot = _rot.permute(0, 2, 1)  # ->Px4xN
        init_rot = _rot

        ## Initialize SH difference
        # delta_sh = rot(qi,SH)-SH
        features_dc = self._features_dc
        features_rest = self._features_rest
        _features = torch.cat((features_dc, features_rest), dim=1) # Px16x3
        _rotation = self._rotation # Px4x100
        sh_blendshapes = []
        for ii in range(50):
            # Px4x100
            loc_rot = tmp1_bak[...,ii] # Px4
            # Get full rot
            loc_rot = loc_rot + torch.tensor([1.,0.,0.,0.],dtype=loc_rot.dtype,device=loc_rot.device)
            loc_mat = quaternion_to_matrix(loc_rot) # Px3x3
            rot_features = RotSH3_C3(loc_mat, _features)
            delta_  = rot_features - _features
            sh_blendshapes.append(delta_) # Px16x3
        init_sh = torch.stack(sh_blendshapes,dim=-1)

        ## Initialize pos. difference
        rd_faceR_tensor = faceR_tensor[:, :50, ...]  # Fx50x3x3
        rd_facet_tensor = facet_tensor[:, :50, ...]  # Fx50x3
        tmp1 = rd_faceR_tensor[face_id]  # Px50x3x3
        tmp2 = rd_facet_tensor[face_id]  # Px50x3
        v_biased_ = v_biased.unsqueeze(1).repeat(1, 50, 1)  # Px3 -> Px50x3
        tmp1 = tmp1.view(tmp1.shape[0] * 50, 3, 3)
        v_biased_ = v_biased_.view(tmp1.shape[0], 3, 1)
        tmp1 = torch.bmm(tmp1, v_biased_).squeeze(-1).view(tmp1.shape[0] // 50, 50, 3)
        xyz_blendshape = tmp1 + tmp2
        # Px50x3 -> Px3x50
        init_xyz = xyz_blendshape.permute(0, 2, 1)

        pose_f_tensor_ = pose_f_tensor[:,face_id] # 36xPx3
        pose_blendshape = pose_f_tensor_  # 36xPx3

        ## Initialize eyelid difference
        eye_offset = r_eyelid_offset[face_id] # Px3
        r_eye_offset = eye_offset
        eye_offset = l_eyelid_offset[face_id] # Px3
        l_eye_offset = eye_offset
        eye_blendshape = torch.stack([l_eye_offset, r_eye_offset], dim=-1)  # Px3x2

        expr_mask_tensor, W = self.update_from_nn()

        self.acc_dict = {
            "init_rot" : init_rot.contiguous().detach(), # Px4x50
            "init_sh" : init_sh.contiguous().detach(), # Px16x3x50
            "init_xyz" : init_xyz.contiguous().detach(), # Px3x50
            "pose_blendshape" : pose_blendshape.detach(), # 36xPx3
            "eye_blendshape" : eye_blendshape.detach(), # Px3x2
            # updatable
            "expr_mask_tensor" : expr_mask_tensor.contiguous().detach(), # Px50?
            "W": W.contiguous().detach() # Px5
        }

    def save_acc_dict(self, filename):
        acc_dict = self.acc_dict
        np_acc_dict = {}
        for k,v in acc_dict.items():
            np_acc_dict[k] = v.detach().cpu().numpy()
        np.save(filename,np_acc_dict)

    def load_acc_dict(self, filename):
        np_acc_dict = np.load(filename,allow_pickle=True).item()
        acc_dict = {}
        for k, v in np_acc_dict.items():
            acc_dict[k] = torch.from_numpy(v).cuda()
        self.acc_dict = acc_dict
        #self.flatten_rot = acc_dict["flatten_rot"]


    # dump for C++ viewer
    def save_npy_forviewer(self, path):

        mkdir_p(path)
        features_dc = self._features_dc
        features_reset = self._features_rest
        _xyz = self._xyz
        _features = torch.cat((features_dc, features_reset),dim=1)
        _scaling = self._scaling
        _rotation = self._rotation
        _opacity = self._opacity

        assert(self.acc_dict is not None)
        extract_params = self.acc_dict

        _xyz = _xyz.cpu().detach().numpy()
        _features = _features.cpu().detach().numpy()
        #n_points = _xyz.shape[0]

        np.save(os.path.join(path, "pos.npy"),_xyz)
        np.save(os.path.join(path, "shs.npy"),_features)
        np.save(os.path.join(path, "scale.npy"), _scaling.cpu().detach().numpy())  # 71600x3
        np.save(os.path.join(path, "rot.npy"), _rotation.cpu().detach().numpy())  # 71600x4
        np.save(os.path.join(path, "opacity.npy"), _opacity.cpu().detach().numpy())  # 71600x1

        #v_biased = self._xyz
        # fetch skinning weights
        W = self.acc_dict["W"]

        # Compute expr_mask_tensor
        expr_mask_tensor = self.acc_dict["expr_mask_tensor"]

        init_rot = extract_params["init_rot"]
        init_sh = extract_params["init_sh"]
        init_xyz = extract_params["init_xyz"]
        pose_blendshape = extract_params["pose_blendshape"]
        eye_blendshape = extract_params["eye_blendshape"]

        _features_b = self._features_b
        if use_feature_expr_mask:
            _features_b = _features_b * expr_mask_tensor.unsqueeze(1).unsqueeze(1)
        _scaling_b = self._scaling_b
        if use_scale_expr_mask:
            _scaling_b = _scaling_b * expr_mask_tensor.unsqueeze(1)
        _rotation_b = self._rotation_b
        if use_rot_expr_mask:
            _rotation_b = _rotation_b * expr_mask_tensor.unsqueeze(1)
        _opacity_b = self._opacity_b
        if use_opacity_expr_mask:
            _opacity_b = _opacity_b * expr_mask_tensor.unsqueeze(1)

        _xyz_b = self._xyz_b
        if use_xyz_expr_mask:
            _xyz_b = _xyz_b * expr_mask_tensor.unsqueeze(1)
        xyz_blendshape = _xyz_b + init_xyz

        _rotation_b = _rotation_b + init_rot
        _features_b = _features_b + init_sh

        np.save(os.path.join(path,"xyz_t.npy"),xyz_blendshape.cpu().detach().numpy()) # Px3x100
        np.save(os.path.join(path,"shs_t.npy"),_features_b.cpu().detach().numpy()) # PxLx3x100
        np.save(os.path.join(path,"scale_t.npy"),_scaling_b.cpu().detach().numpy()) # Px3x100
        np.save(os.path.join(path,"rot_t.npy"),_rotation_b.cpu().detach().numpy()) # Px4x100
        np.save(os.path.join(path,"opacity_t.npy"),_opacity_b.cpu().detach().numpy()) # Px1x100

        np.save(os.path.join(path, "pos_t.npy"), pose_blendshape.cpu().detach().numpy())  # 36xPx3
        np.save(os.path.join(path, "W.npy"), W.cpu().detach().numpy()) # Px5
        np.save(os.path.join(path, "eyelid.npy"), eye_blendshape.cpu().detach().numpy()) # Px3x2


    def reset_opacity(self):
        opacities_new = inverse_sigmoid(torch.min(self.get_opacity, torch.ones_like(self.get_opacity)*0.01))
        optimizable_tensors = self.replace_tensor_to_optimizer(opacities_new, "opacity")
        self._opacity = optimizable_tensors["opacity"]

    def load_ply(self, path):
        plydata = PlyData.read(path)

        xyz = np.stack((np.asarray(plydata.elements[0]["x"]),
                        np.asarray(plydata.elements[0]["y"]),
                        np.asarray(plydata.elements[0]["z"])),  axis=1)
        opacities = np.asarray(plydata.elements[0]["opacity"])[..., np.newaxis]

        features_dc = np.zeros((xyz.shape[0], 3, 1))
        features_dc[:, 0, 0] = np.asarray(plydata.elements[0]["f_dc_0"])
        features_dc[:, 1, 0] = np.asarray(plydata.elements[0]["f_dc_1"])
        features_dc[:, 2, 0] = np.asarray(plydata.elements[0]["f_dc_2"])

        extra_f_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("f_rest_")]
        extra_f_names = sorted(extra_f_names, key = lambda x: int(x.split('_')[-1]))
        assert len(extra_f_names)==3*(self.max_sh_degree + 1) ** 2 - 3
        features_extra = np.zeros((xyz.shape[0], len(extra_f_names)))
        for idx, attr_name in enumerate(extra_f_names):
            features_extra[:, idx] = np.asarray(plydata.elements[0][attr_name])
        # Reshape (P,F*SH_coeffs) to (P, F, SH_coeffs except DC)
        features_extra = features_extra.reshape((features_extra.shape[0], 3, (self.max_sh_degree + 1) ** 2 - 1))

        scale_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("scale_")]
        scale_names = sorted(scale_names, key = lambda x: int(x.split('_')[-1]))
        scales = np.zeros((xyz.shape[0], len(scale_names)))
        for idx, attr_name in enumerate(scale_names):
            scales[:, idx] = np.asarray(plydata.elements[0][attr_name])

        rot_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("rot")]
        rot_names = sorted(rot_names, key = lambda x: int(x.split('_')[-1]))
        rots = np.zeros((xyz.shape[0], len(rot_names)))
        for idx, attr_name in enumerate(rot_names):
            rots[:, idx] = np.asarray(plydata.elements[0][attr_name])

        self._xyz = nn.Parameter(torch.tensor(xyz, dtype=torch.float, device="cuda").requires_grad_(True))
        self._features_dc = nn.Parameter(torch.tensor(features_dc, dtype=torch.float, device="cuda").transpose(1, 2).contiguous().requires_grad_(True))
        self._features_rest = nn.Parameter(torch.tensor(features_extra, dtype=torch.float, device="cuda").transpose(1, 2).contiguous().requires_grad_(True))
        self._opacity = nn.Parameter(torch.tensor(opacities, dtype=torch.float, device="cuda").requires_grad_(True))
        self._scaling = nn.Parameter(torch.tensor(scales, dtype=torch.float, device="cuda").requires_grad_(True))
        self._rotation = nn.Parameter(torch.tensor(rots, dtype=torch.float, device="cuda").requires_grad_(True))

        self.active_sh_degree = self.max_sh_degree

    def replace_tensor_to_optimizer(self, tensor, name):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            if group["name"] == name:
                stored_state = self.optimizer.state.get(group['params'][0], None)
                stored_state["exp_avg"] = torch.zeros_like(tensor)
                stored_state["exp_avg_sq"] = torch.zeros_like(tensor)

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter(tensor.requires_grad_(True))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
        return optimizable_tensors

    def _prune_optimizer(self, mask):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            stored_state = self.optimizer.state.get(group['params'][0], None)
            if stored_state is not None:
                stored_state["exp_avg"] = stored_state["exp_avg"][mask]
                stored_state["exp_avg_sq"] = stored_state["exp_avg_sq"][mask]

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter((group["params"][0][mask].requires_grad_(True)))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
            else:
                group["params"][0] = nn.Parameter(group["params"][0][mask].requires_grad_(True))
                optimizable_tensors[group["name"]] = group["params"][0]
        return optimizable_tensors

    def prune_points(self, mask):
        valid_points_mask = ~mask
        optimizable_tensors = self._prune_optimizer(valid_points_mask)

        if "xyz" in optimizable_tensors:
            self._xyz = optimizable_tensors["xyz"]
        else:
            tmp = self._xyz[valid_points_mask].detach()
            self._xyz = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_dc" in optimizable_tensors:
            self._features_dc = optimizable_tensors["f_dc"]
        else:
            tmp = self._features_dc[valid_points_mask].detach()
            self._features_dc = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_rest" in optimizable_tensors:
            self._features_rest = optimizable_tensors["f_rest"]
        else:
            tmp = self._features_rest[valid_points_mask].detach()
            self._features_rest = nn.Parameter(tmp.requires_grad_(True))
        #
        if "opacity" in optimizable_tensors:
            self._opacity = optimizable_tensors["opacity"]
        else:
            tmp = self._opacity[valid_points_mask].detach()
            self._opacity = nn.Parameter(tmp.requires_grad_(True))
        #
        if "scaling" in optimizable_tensors:
            self._scaling = optimizable_tensors["scaling"]
        else:
            tmp = self._scaling[valid_points_mask].detach()
            self._scaling = nn.Parameter(tmp.requires_grad_(True))
        #
        if "rotation" in optimizable_tensors:
            self._rotation = optimizable_tensors["rotation"]
        else:
            tmp = self._rotation[valid_points_mask].detach()
            self._rotation = nn.Parameter(tmp.requires_grad_(True))

        if "xyz_b" in optimizable_tensors:
            self._xyz_b = optimizable_tensors["xyz_b"]
        else:
            tmp = self._xyz_b[valid_points_mask].detach()
            self._xyz_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_b" in optimizable_tensors:
            self._features_b = optimizable_tensors["f_b"]
        else:
            tmp = self._features_b[valid_points_mask].detach()
            self._features_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "opacity_b" in optimizable_tensors:
            self._opacity_b = optimizable_tensors["opacity_b"]
        else:
            tmp = self._opacity_b[valid_points_mask].detach()
            self._opacity_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "scaling_b" in optimizable_tensors:
            self._scaling_b = optimizable_tensors["scaling_b"]
        else:
            tmp = self._scaling_b[valid_points_mask].detach()
            self._scaling_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "rotation_b" in optimizable_tensors:
            self._rotation_b = optimizable_tensors["rotation_b"]
        else:
            tmp = self._rotation_b[valid_points_mask].detach()
            self._rotation_b = nn.Parameter(tmp.requires_grad_(True))

        if self.acc_dict is not None:
            self.acc_dict["init_rot"] = self.acc_dict["init_rot"][valid_points_mask].detach()
            self.acc_dict["init_sh"] = self.acc_dict["init_sh"][valid_points_mask].detach()
            self.acc_dict["init_xyz"] = self.acc_dict["init_xyz"][valid_points_mask].detach()
            self.acc_dict["pose_blendshape"] = self.acc_dict["pose_blendshape"][:, valid_points_mask].detach()
            self.acc_dict["eye_blendshape"] = self.acc_dict["eye_blendshape"][valid_points_mask].detach()
            self.acc_dict["expr_mask_tensor"] = self.acc_dict["expr_mask_tensor"][valid_points_mask].detach()
            self.acc_dict["W"] = self.acc_dict["W"][valid_points_mask].detach()

        self.xyz_gradient_accum = self.xyz_gradient_accum[valid_points_mask]

        self.denom = self.denom[valid_points_mask]
        self.max_radii2D = self.max_radii2D[valid_points_mask]

    def cat_tensors_to_optimizer(self, tensors_dict):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            assert len(group["params"]) == 1
            extension_tensor = tensors_dict[group["name"]]
            stored_state = self.optimizer.state.get(group['params'][0], None)
            if stored_state is not None:

                stored_state["exp_avg"] = torch.cat((stored_state["exp_avg"], torch.zeros_like(extension_tensor)), dim=0)
                stored_state["exp_avg_sq"] = torch.cat((stored_state["exp_avg_sq"], torch.zeros_like(extension_tensor)), dim=0)

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter(torch.cat((group["params"][0], extension_tensor), dim=0).requires_grad_(True))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
            else:
                group["params"][0] = nn.Parameter(torch.cat((group["params"][0], extension_tensor), dim=0).requires_grad_(True))
                optimizable_tensors[group["name"]] = group["params"][0]

        return optimizable_tensors

    def densification_postfix(self, new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation,
                              new_xyz_b,new_features_b, new_opacity_b, new_scaling_b, new_rotation_b, new_acc_dict):
        d = {"xyz": new_xyz,
             "f_dc": new_features_dc,
             "f_rest": new_features_rest,
             "opacity": new_opacities,
             "scaling": new_scaling,
             "rotation": new_rotation,
             #
             "xyz_b": new_xyz_b,
             "f_b": new_features_b,
             "opacity_b": new_opacity_b,
             "scaling_b": new_scaling_b,
             "rotation_b": new_rotation_b
             }

        optimizable_tensors = self.cat_tensors_to_optimizer(d)
        #
        if "xyz" in optimizable_tensors:
            self._xyz = optimizable_tensors["xyz"]
        else:
            tmp = torch.cat([self._xyz, new_xyz],dim=0).detach()
            self._xyz = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_dc" in optimizable_tensors:
            self._features_dc = optimizable_tensors["f_dc"]
        else:
            tmp = torch.cat([self._features_dc, new_features_dc],dim=0).detach()
            self._features_dc = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_rest" in optimizable_tensors:
            self._features_rest = optimizable_tensors["f_rest"]
        else:
            tmp = torch.cat([self._features_rest, new_features_rest],dim=0).detach()
            self._features_rest = nn.Parameter(tmp.requires_grad_(True))
        #
        if "opacity" in optimizable_tensors:
            self._opacity = optimizable_tensors["opacity"]
        else:
            tmp = torch.cat([self._opacity, new_opacities], dim = 0).detach()
            self._opacity = nn.Parameter(tmp.requires_grad_(True))
        #
        if "scaling" in optimizable_tensors:
            self._scaling = optimizable_tensors["scaling"]
        else:
            tmp = torch.cat([self._scaling, new_scaling], dim = 0).detach()
            self._scaling = nn.Parameter(tmp.requires_grad_(True))
        #
        if "rotation" in optimizable_tensors:
            self._rotation = optimizable_tensors["rotation"]
        else:
            tmp = torch.cat([self._rotation, new_rotation], dim = 0).detach()
            self._rotation = nn.Parameter(tmp.requires_grad_(True))

        #
        if "xyz_b" in optimizable_tensors:
            self._xyz_b = optimizable_tensors["xyz_b"]
        else:
            tmp = torch.cat([self._xyz_b,new_xyz_b],dim=0).detach()
            self._xyz_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "f_b" in optimizable_tensors:
            self._features_b = optimizable_tensors["f_b"]
        else:
            tmp = torch.cat([self._features_b,new_features_b],dim=0).detach()
            self._features_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "opacity_b" in optimizable_tensors:
            self._opacity_b = optimizable_tensors["opacity_b"]
        else:
            tmp = torch.cat([self._opacity_b, new_opacity_b],dim=0).detach()
            self._opacity_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "scaling_b" in optimizable_tensors:
            self._scaling_b = optimizable_tensors["scaling_b"]
        else:
            tmp = torch.cat([self._scaling_b, new_scaling_b],dim=0).detach()
            self._scaling_b = nn.Parameter(tmp.requires_grad_(True))
        #
        if "rotation_b" in optimizable_tensors:
            self._rotation_b = optimizable_tensors["rotation_b"]
        else:
            tmp = torch.cat([self._rotation_b, new_rotation_b],dim=0).detach()
            self._rotation_b = nn.Parameter(tmp.requires_grad_(True))

        self.xyz_gradient_accum = torch.zeros((self._scaling.shape[0], 1), device="cuda")
        self.denom = torch.zeros((self._scaling.shape[0], 1), device="cuda")
        self.max_radii2D = torch.zeros((self._scaling.shape[0]), device="cuda")

        with torch.no_grad():
            if self.acc_dict is not None:
                self.acc_dict["init_rot"] = torch.cat([self.acc_dict["init_rot"], new_acc_dict["init_rot"]],dim=0)
                self.acc_dict["init_sh"] = torch.cat([self.acc_dict["init_sh"], new_acc_dict["init_sh"]], dim=0)
                self.acc_dict["init_xyz"] = torch.cat([self.acc_dict["init_xyz"], new_acc_dict["init_xyz"]], dim=0)
                self.acc_dict["pose_blendshape"] = torch.cat([self.acc_dict["pose_blendshape"], new_acc_dict["pose_blendshape"]],dim=1)
                self.acc_dict["eye_blendshape"] = torch.cat([self.acc_dict["eye_blendshape"], new_acc_dict["eye_blendshape"]],dim=0)
                self.acc_dict["expr_mask_tensor"] = torch.cat([self.acc_dict["expr_mask_tensor"], new_acc_dict["expr_mask_tensor"]],dim=0)
                self.acc_dict["W"] = torch.cat([self.acc_dict["W"], new_acc_dict["W"]],dim=0)

    def densify_and_split(self, grads, grad_threshold, scene_extent, N=2):

        _scaling = self.scaling_activation(self._scaling)
        n_init_points = self._scaling.shape[0]
        # Extract points that satisfy the gradient condition
        padded_grad = torch.zeros((n_init_points), device="cuda")
        padded_grad[:grads.shape[0]] = grads.squeeze()
        selected_pts_mask = torch.where(padded_grad >= grad_threshold, True, False)
        #selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(self.get_scaling, dim=1).values > self.percent_dense * scene_extent)
        selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(_scaling, dim=1).values > self.percent_dense * scene_extent)

        #stds = self.get_scaling[selected_pts_mask].repeat(N, 1)
        stds = _scaling[selected_pts_mask].repeat(N,1)
        means = torch.zeros((stds.size(0), 3), device="cuda")
        samples = torch.normal(mean=means, std=stds)
        rots = build_rotation(self._rotation[selected_pts_mask]).repeat(N, 1, 1)
        #new_xyz = torch.bmm(rots, samples.unsqueeze(-1)).squeeze(-1) + self.get_xyz[selected_pts_mask].repeat(N, 1)
        new_xyz = torch.bmm(rots, samples.unsqueeze(-1)).squeeze(-1) + self._xyz[selected_pts_mask].repeat(N, 1)
        #new_scaling = self.scaling_inverse_activation(self.get_scaling[selected_pts_mask].repeat(N, 1) / (0.8 * N))
        new_scaling = self.scaling_inverse_activation(_scaling[selected_pts_mask].repeat(N, 1) / (0.8 * N))
        new_rotation = self._rotation[selected_pts_mask].repeat(N, 1)
        new_features_dc = self._features_dc[selected_pts_mask].repeat(N, 1, 1)
        new_features_rest = self._features_rest[selected_pts_mask].repeat(N, 1, 1)
        new_opacity = self._opacity[selected_pts_mask].repeat(N, 1)

        new_xyz_b = self._xyz_b[selected_pts_mask].repeat(N,1,1)
        new_features_b = self._features_b[selected_pts_mask].repeat(N,1,1,1)
        new_opacity_b = self._opacity_b[selected_pts_mask].repeat(N,1,1)
        new_scaling_b = self._scaling_b[selected_pts_mask].repeat(N,1,1)
        new_rotation_b = self._rotation_b[selected_pts_mask].repeat(N,1,1)

        with torch.no_grad():
            new_acc_dict = None
            if self.acc_dict is not None:
                new_acc_dict = {
                    "init_rot": self.acc_dict["init_rot"][selected_pts_mask].repeat(N,1,1),
                    "init_sh": self.acc_dict["init_sh"][selected_pts_mask].repeat(N,1,1,1),
                    "init_xyz": self.acc_dict["init_xyz"][selected_pts_mask].repeat(N,1,1),
                    "pose_blendshape": self.acc_dict["pose_blendshape"][:, selected_pts_mask].repeat(1,N,1),
                    "eye_blendshape": self.acc_dict["eye_blendshape"][selected_pts_mask].repeat(N,1,1),
                    "expr_mask_tensor": self.acc_dict["expr_mask_tensor"][selected_pts_mask].repeat(N,1),
                    "W": self.acc_dict["W"][selected_pts_mask].repeat(N,1)
                }

        self.densification_postfix(new_xyz, new_features_dc, new_features_rest, new_opacity, new_scaling, new_rotation,
                                    new_xyz_b,new_features_b, new_opacity_b, new_scaling_b, new_rotation_b, new_acc_dict)

        prune_filter = torch.cat((selected_pts_mask, torch.zeros(N * selected_pts_mask.sum(), device="cuda", dtype=bool)))
        self.prune_points(prune_filter)

    def densify_and_clone(self, grads, grad_threshold, scene_extent):
        # Extract points that satisfy the gradient condition
        selected_pts_mask = torch.where(torch.norm(grads, dim=-1) >= grad_threshold, True, False)
        _scaling = self.scaling_activation(self._scaling)
        #selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(self.get_scaling, dim=1).values <= self.percent_dense * scene_extent)
        selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(_scaling,  dim=1).values <= self.percent_dense * scene_extent)

        new_xyz = self._xyz[selected_pts_mask]
        new_features_dc = self._features_dc[selected_pts_mask]
        new_features_rest = self._features_rest[selected_pts_mask]
        new_opacities = self._opacity[selected_pts_mask]
        new_scaling = self._scaling[selected_pts_mask]
        new_rotation = self._rotation[selected_pts_mask]

        new_xyz_b = self._xyz_b[selected_pts_mask]
        new_features_b = self._features_b[selected_pts_mask]
        new_opacity_b = self._opacity_b[selected_pts_mask]
        new_scaling_b = self._scaling_b[selected_pts_mask]
        new_rotation_b = self._rotation_b[selected_pts_mask]

        with torch.no_grad():
            new_acc_dict = None
            if self.acc_dict is not None:
                new_acc_dict = {
                    "init_rot": self.acc_dict["init_rot"][selected_pts_mask],
                    "init_sh": self.acc_dict["init_sh"][selected_pts_mask],
                    "init_xyz": self.acc_dict["init_xyz"][selected_pts_mask],
                    "pose_blendshape": self.acc_dict["pose_blendshape"][:, selected_pts_mask],
                    "eye_blendshape": self.acc_dict["eye_blendshape"][selected_pts_mask],
                    "expr_mask_tensor": self.acc_dict["expr_mask_tensor"][selected_pts_mask],
                    "W": self.acc_dict["W"][selected_pts_mask]
                }

        self.densification_postfix(new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation,
                                   new_xyz_b,new_features_b, new_opacity_b, new_scaling_b, new_rotation_b, new_acc_dict)

    def densify_and_prune(self, max_grad, min_opacity, extent, max_screen_size):
        grads = self.xyz_gradient_accum / self.denom
        grads[grads.isnan()] = 0.0

        self.densify_and_clone(grads, max_grad, extent)
        self.densify_and_split(grads, max_grad, extent)

        _scaling = self.scaling_activation(self._scaling)

        #prune_mask = (self.get_opacity < min_opacity).squeeze()
        prune_mask = (self.opacity_activation(self._opacity) < min_opacity).squeeze()
        if max_screen_size:
            big_points_vs = self.max_radii2D > max_screen_size
            #big_points_ws = self.get_scaling.max(dim=1).values > 0.1 * extent
            big_points_ws = _scaling.max(dim=1).values > 0.1 * extent
            prune_mask = torch.logical_or(torch.logical_or(prune_mask, big_points_vs), big_points_ws)
        self.prune_points(prune_mask)

        torch.cuda.empty_cache()

    def add_densification_stats(self, viewspace_point_tensor, update_filter):
        self.xyz_gradient_accum[update_filter] += torch.norm(viewspace_point_tensor.grad[update_filter, :2], dim=-1, keepdim=True)
        self.denom[update_filter] += 1







```

## Source file: FLAME/face_renderer.py

{
  "path": "FLAME/face_renderer.py",
  "blob_sha": "9bc1c8c450efdc975353756a0cdcbd316ea6fa16",
  "sha256": "5b1df9d7098b58638f42a8c8f1d1f735a12d1f0c232931f7ee0305669679aacf",
  "lines": 263,
  "bytes": 9869,
  "selection_score": 18,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/zjumsj/GaussianBlendshapes/blob/501712451535b43d44ee7a37b266d2fd1f5e2a0a/FLAME/face_renderer.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def render(viewpoint_camera, pcs: List[GaussianModel], pipe,"
    },
    {
      "line": 129,
      "text": "def render_alpha(viewpoint_camera, pcs: List[GaussianModel], pipe,"
    },
    {
      "line": 252,
      "text": "def get_depth(viewpoint_cameras, pcs: List[GaussianModel]):"
    }
  ]
}

```text

import torch
import math

from diff_gaussian_rasterization import GaussianRasterizationSettings, GaussianRasterizer
from .face_gs_model import GaussianModel
from utils.sh_utils import eval_sh

from typing import List

def render(viewpoint_camera, pcs: List[GaussianModel], pipe,
           bg_color: torch.Tensor, scaling_modifier = 1.0, override_color = None):
    """
    Render the scene.

    Background tensor (bg_color) must be on GPU!
    """

    ## merge sub point cloud to form full point clouds
    n = len(pcs)
    xyz = []
    features = []
    scaling = []
    rotation = []
    opacity = []
    cov3D_precomp_ = []
    for ii in range(n):
        xyz.append(pcs[ii].get_xyz)
        features.append(pcs[ii].get_features)
        opacity.append(pcs[ii].get_opacity)
        # If precomputed 3d covariance is provided, use it. If not, then it will be computed from
        # scaling / rotation by the rasterizer.
        if pipe.compute_cov3D_python:
            cov3D_precomp_.append(pcs[ii].get_covariance(scaling_modifier))
        else:
            scaling.append(pcs[ii].get_scaling)
            rotation.append(pcs[ii].get_rotation)

    if override_color is not None:
        if isinstance(override_color,list) or isinstance(override_color,tuple):
            override_color = torch.cat(override_color,dim=0)

    scales = None
    rotations = None
    cov3D_precomp = None

    xyz = torch.cat(xyz,dim=0)
    features = torch.cat(features,dim=0)
    if pipe.compute_cov3D_python:
        cov3D_precomp = torch.cat(cov3D_precomp_, dim=0)
    else:
        scales = torch.cat(scaling,dim=0)
        rotations = torch.cat(rotation,dim=0)

    opacity = torch.cat(opacity,dim=0)

    # Create zero tensor. We will use it to make pytorch return gradients of the 2D (screen-space) means
    screenspace_points = torch.zeros_like(xyz, dtype=xyz.dtype, requires_grad=True, device="cuda") + 0
    try:
        screenspace_points.retain_grad()
    except:
        pass

    # Set up rasterization configuration
    tanfovx = math.tan(viewpoint_camera.FoVx * 0.5)
    tanfovy = math.tan(viewpoint_camera.FoVy * 0.5)

    raster_settings = GaussianRasterizationSettings(
        image_height=int(viewpoint_camera.image_height),
        image_width=int(viewpoint_camera.image_width),
        tanfovx=tanfovx,
        tanfovy=tanfovy,
        bg=bg_color,
        scale_modifier=scaling_modifier,
        viewmatrix=viewpoint_camera.world_view_transform,
        projmatrix=viewpoint_camera.full_proj_transform,
        sh_degree=pcs[0].active_sh_degree, # TODO, all pc should agree
        campos=viewpoint_camera.camera_center,
        prefiltered=False,
        debug=pipe.debug
    )

    rasterizer = GaussianRasterizer(raster_settings=raster_settings)

    means3D = xyz
    means2D = screenspace_points
    #opacity = opacity

    # If precomputed colors are provided, use them. Otherwise, if it is desired to precompute colors
    # from SHs in Python, do it. If not, then SH -> RGB conversion will be done by rasterizer.
    shs = None
    colors_precomp = None
    if override_color is None:
        if pipe.convert_SHs_python:
            shs_view = features.transpose(1,2).view(-1,3, (pcs[0].max_sh_degree + 1) ** 2)
            dir_pp = (xyz - viewpoint_camera.camera_center.repeat(features.shape[0], 1))
            dir_pp_normalized = dir_pp / dir_pp.norm(dim=1,keepdim=True)
            sh2rgb = eval_sh(pcs[0].active_sh_degree, shs_view, dir_pp_normalized)
            colors_precomp = torch.clamp_min(sh2rgb + 0.5, 0.0)

            # shs_view = pc.get_features.transpose(1, 2).view(-1, 3, (pc.max_sh_degree + 1) ** 2)
            # dir_pp = (pc.get_xyz - viewpoint_camera.camera_center.repeat(pc.get_features.shape[0], 1))
            # dir_pp_normalized = dir_pp / dir_pp.norm(dim=1, keepdim=True)
            # sh2rgb = eval_sh(pc.active_sh_degree, shs_view, dir_pp_normalized)
            # colors_precomp = torch.clamp_min(sh2rgb + 0.5, 0.0)
        else:
            shs = features
    else:
        colors_precomp = override_color

    # Rasterize visible Gaussians to image, obtain their radii (on screen).
    rendered_image, radii = rasterizer(
        means3D = means3D,
        means2D = means2D,
        shs = shs,
        colors_precomp = colors_precomp,
        opacities = opacity,
        scales = scales,
        rotations = rotations,
        cov3D_precomp = cov3D_precomp)

    # Those Gaussians that were frustum culled or had a radius of 0 were not visible.
    # They will be excluded from value updates used in the splitting criteria.
    return {"render": rendered_image,
            "viewspace_points": screenspace_points,
            "visibility_filter" : radii > 0,
            "radii": radii}


def render_alpha(viewpoint_camera, pcs: List[GaussianModel], pipe,
           bg_color:torch.Tensor, scaling_modifier = 1.0, override_color = None):
    """
        Render the scene with alpha

        Background tensor (bg_color) must be on GPU!
    """

    ## merge sub point cloud to form full point clouds
    n = len(pcs)
    xyz = []
    features = []
    scaling = []
    rotation = []
    opacity = []
    cov3D_precomp_ = []
    point_numbers = []
    for ii in range(n):
        xyz.append(pcs[ii].get_xyz)
        point_numbers.append(xyz[ii].shape[0])
        features.append(pcs[ii].get_features)
        opacity.append(pcs[ii].get_opacity)
        # If precomputed 3d covariance is provided, use it. If not, then it will be computed from
        # scaling / rotation by the rasterizer.
        if pipe.compute_cov3D_python:
            cov3D_precomp_.append(pcs[ii].get_covariance(scaling_modifier))
        else:
            scaling.append(pcs[ii].get_scaling)
            rotation.append(pcs[ii].get_rotation)

    if override_color is not None:
        if isinstance(override_color, list) or isinstance(override_color, tuple):
            override_color = torch.cat(override_color, dim=0)

    scales = None
    rotations = None
    cov3D_precomp = None

    xyz = torch.cat(xyz, dim=0)
    features = torch.cat(features, dim=0)
    if pipe.compute_cov3D_python:
        cov3D_precomp = torch.cat(cov3D_precomp_, dim=0)
    else:
        scales = torch.cat(scaling, dim=0)
        rotations = torch.cat(rotation, dim=0)

    opacity = torch.cat(opacity, dim=0)

    # Create zero tensor. We will use it to make pytorch return gradients of the 2D (screen-space) means
    screenspace_points = torch.zeros_like(xyz, dtype=xyz.dtype, requires_grad=True, device="cuda") + 0
    try:
        screenspace_points.retain_grad()
    except:
        pass

    # Set up rasterization configuration
    tanfovx = math.tan(viewpoint_camera.FoVx * 0.5)
    tanfovy = math.tan(viewpoint_camera.FoVy * 0.5)

    raster_settings = GaussianRasterizationSettings(
        image_height=int(viewpoint_camera.image_height),
        image_width=int(viewpoint_camera.image_width),
        tanfovx=tanfovx,
        tanfovy=tanfovy,
        bg=bg_color,
        scale_modifier=scaling_modifier,
        viewmatrix=viewpoint_camera.world_view_transform,
        projmatrix=viewpoint_camera.full_proj_transform,
        sh_degree=pcs[0].active_sh_degree,
        campos=viewpoint_camera.camera_center,
        prefiltered=False,
        debug=pipe.debug
    )

    rasterizer = GaussianRasterizer(raster_settings=raster_settings)

    means3D = xyz
    means2D = screenspace_points
    # opacity = opacity

    # If precomputed colors are provided, use them. Otherwise, if it is desired to precompute colors
    # from SHs in Python, do it. If not, then SH -> RGB conversion will be done by rasterizer.
    shs = None
    colors_precomp = None
    if override_color is None:
        if pipe.convert_SHs_python:
            shs_view = features.transpose(1, 2).view(-1, 3, (pcs[0].max_sh_degree + 1) ** 2)
            dir_pp = (xyz - viewpoint_camera.camera_center.repeat(features.shape[0], 1))
            dir_pp_normalized = dir_pp / dir_pp.norm(dim=1, keepdim=True)
            sh2rgb = eval_sh(pcs[0].active_sh_degree, shs_view, dir_pp_normalized)
            colors_precomp = torch.clamp_min(sh2rgb + 0.5, 0.0)

            # shs_view = pc.get_features.transpose(1, 2).view(-1, 3, (pc.max_sh_degree + 1) ** 2)
            # dir_pp = (pc.get_xyz - viewpoint_camera.camera_center.repeat(pc.get_features.shape[0], 1))
            # dir_pp_normalized = dir_pp / dir_pp.norm(dim=1, keepdim=True)
            # sh2rgb = eval_sh(pc.active_sh_degree, shs_view, dir_pp_normalized)
            # colors_precomp = torch.clamp_min(sh2rgb + 0.5, 0.0)
        else:
            shs = features
    else:
        colors_precomp = override_color

    # Rasterize visible Gaussians to image, obtain their radii (on screen).
    rendered_image, alpha0, radii = rasterizer( # DEBUG
        means3D = means3D,
        means2D = means2D,
        shs = shs,
        colors_precomp = colors_precomp,
        opacities = opacity,
        scales = scales,
        rotations = rotations,
        cov3D_precomp = cov3D_precomp,
        alpha_point_id = 0
    )

    # Those Gaussians that were frustum culled or had a radius of 0 were not visible.
    # They will be excluded from value updates used in the splitting criteria.
    return {"render": rendered_image,
            "alpha0": alpha0,
            "viewspace_points": screenspace_points,
            "visibility_filter" : radii > 0,
            "radii": radii}

def get_depth(viewpoint_cameras, pcs: List[GaussianModel]):
    n = len(pcs)
    depth_map = []
    for ii in range(n):
        xyz = pcs[ii].get_xyz
        w2c_T = viewpoint_cameras.world_view_transform
        # Nx4
        homo_xyz = torch.cat([xyz,torch.ones_like(xyz[...,0:1])],dim=-1)
        c_xyzw = torch.matmul(homo_xyz, w2c_T)
        depth_c3 = c_xyzw[...,[2,2,2]]
        depth_map.append(depth_c3)
    return depth_map


```

## Source file: FLAME/mouth_gs_model.py

{
  "path": "FLAME/mouth_gs_model.py",
  "blob_sha": "61528ee80d1e52736a442a614c0be096ee594ad8",
  "sha256": "643a68c6fd1d8240a245f40b7e0f95783403414cd8deef2502dc1987c31ec9cb",
  "lines": 610,
  "bytes": 28764,
  "selection_score": 18,
  "keyword_line_counts": {
    "jaw": 7,
    "teeth": 2,
    "face": 5,
    "lip": 4
  },
  "url": "https://github.com/zjumsj/GaussianBlendshapes/blob/501712451535b43d44ee7a37b266d2fd1f5e2a0a/FLAME/mouth_gs_model.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def __init__(self):"
    },
    {
      "line": 39,
      "text": "def __init__(self, center, R, half_H):"
    },
    {
      "line": 48,
      "text": "def compute_loss(self, pos):"
    },
    {
      "line": 60,
      "text": "def hard_constraint(self, pos):"
    },
    {
      "line": 82,
      "text": "def setup_functions(self):"
    },
    {
      "line": 83,
      "text": "def build_covariance_from_scaling_rotation(scaling, scaling_modifier, rotation):"
    },
    {
      "line": 99,
      "text": "def __init__(self, sh_degree : int, model_type: int):"
    },
    {
      "line": 120,
      "text": "def capture(self):"
    },
    {
      "line": 136,
      "text": "def restore(self, model_args, training_args):"
    },
    {
      "line": 155,
      "text": "def sample_possion_points(self,v,f,N):"
    },
    {
      "line": 163,
      "text": "def prepare_xyz(self, params, args):"
    },
    {
      "line": 204,
      "text": "def get_scaling(self):"
    },
    {
      "line": 208,
      "text": "def get_rotation(self):"
    },
    {
      "line": 213,
      "text": "def get_xyz(self):"
    },
    {
      "line": 218,
      "text": "def get_features(self):"
    },
    {
      "line": 225,
      "text": "def get_opacity(self):"
    },
    {
      "line": 231,
      "text": "def oneupSHdegree(self):"
    },
    {
      "line": 260,
      "text": "def create_from_face(self, mesh_file, offset,"
    },
    {
      "line": 298,
      "text": "def training_setup(self, training_args):"
    },
    {
      "line": 329,
      "text": "def update_learning_rate(self, iteration):"
    },
    {
      "line": 337,
      "text": "def construct_list_of_attributes(self):"
    },
    {
      "line": 352,
      "text": "def save_npy_forviewer(self, path):"
    },
    {
      "line": 356,
      "text": "def save_npy(self, path):"
    },
    {
      "line": 392,
      "text": "def reset_opacity(self):"
    },
    {
      "line": 397,
      "text": "def load_ply(self, path):"
    },
    {
      "line": 440,
      "text": "def replace_tensor_to_optimizer(self, tensor, name):"
    },
    {
      "line": 455,
      "text": "def _prune_optimizer(self, mask):"
    },
    {
      "line": 473,
      "text": "def prune_points(self, mask):"
    },
    {
      "line": 494,
      "text": "def cat_tensors_to_optimizer(self, tensors_dict):"
    },
    {
      "line": 516,
      "text": "def densification_postfix(self, new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation,"
    },
    {
      "line": 547,
      "text": "def densify_and_split(self, grads, grad_threshold, scene_extent, N=2):"
    },
    {
      "line": 573,
      "text": "def densify_and_clone(self, grads, grad_threshold, scene_extent):"
    },
    {
      "line": 587,
      "text": "def densify_and_prune(self, max_grad, min_opacity, extent, max_screen_size):"
    },
    {
      "line": 603,
      "text": "def add_densification_stats(self, viewspace_point_tensor, update_filter):"
    }
  ]
}

```text


import torch
from torch import nn
import torch.nn.functional as F

import numpy as np
from utils.general_utils import inverse_sigmoid, get_expon_lr_func, build_rotation

import os
from utils.system_utils import mkdir_p
from plyfile import PlyData, PlyElement
from simple_knn._C import distCUDA2
from utils.general_utils import strip_symmetric, build_scaling_rotation

from .rot import matrix_to_quaternion, quaternion_raw_multiply
from diff_gaussian_rasterization import RotSH3_C3

from .adamacc import AdamAcc

from enum import Enum

only_SH0 = False

class DummyObj:
    def __init__(self):
        pass

class View(Enum):
    GROUND_TRUTH = 1
    COLOR_OVERLAY = 2
    SHAPE_OVERLAY = 4
    SHAPE = 8
    LANDMARKS = 16
    HEATMAP = 32
    DEPTH = 64
    MASK = 128

class BoundingCylinder:
    def __init__(self, center, R, half_H):
        self.center = center
        self.R = R
        self.half_H = half_H
        if isinstance(center, list):
            self.center = torch.tensor(center).float().cuda()
        if isinstance(center, np.ndarray):
            self.center = torch.from_numpy(center).cuda()

    def compute_loss(self, pos):
        dist = (pos - self.center)
        #dist_xy = (dist[...,:2] * dist[...,:2]).sum(-1)
        dist_xy = dist[...,0] * dist[...,0] + dist[...,2] * dist[...,2]
        dist_xy = torch.sqrt(dist_xy + 1e-8)
        dist_xy = torch.clip(dist_xy - self.R, 0.)
        #dist_z = torch.abs(dist[...,2])
        dist_z = torch.abs(dist[...,1])
        dist_z = torch.clip(dist_z - self.half_H, 0.)
        L2 = dist_xy * dist_xy + dist_z * dist_z
        return L2

    def hard_constraint(self, pos):
        dist = (pos - self.center)
        #dist_xy = (dist[...,:2] * dist[...,:2]).sum(-1)
        dist_xy = dist[...,0] * dist[...,0] + dist[...,2] * dist[...,2]
        dist_xy = torch.sqrt(dist_xy + 1e-8)
        k_xy = torch.clip(self.R / dist_xy, 0., 1.)
        #dist_z = torch.abs(dist[...,2])
        dist_z = torch.abs(dist[...,1])
        k_z = torch.clip(self.half_H / dist_z, 0., 1.)
        # pos2 = self.center + torch.cat([
        #     dist[...,:2] * k_xy,
        #     dist[...,2:3] * k_z
        # ],dim=-1)
        pos2 = self.center + torch.cat([
            dist[...,0:1] * k_xy[...,None],
            dist[...,1:2] * k_z[...,None],
            dist[...,2:3] * k_xy[...,None]
        ],dim=-1)
        pos.data = pos2.detach()

class GaussianModel:

    def setup_functions(self):
        def build_covariance_from_scaling_rotation(scaling, scaling_modifier, rotation):
            L = build_scaling_rotation(scaling_modifier * scaling, rotation)
            actual_covariance = L @ L.transpose(1, 2)
            symm = strip_symmetric(actual_covariance)
            return symm

        self.scaling_activation = torch.exp
        self.scaling_inverse_activation = torch.log

        self.covariance_activation = build_covariance_from_scaling_rotation

        self.opacity_activation = torch.sigmoid
        self.inverse_opacity_activation = inverse_sigmoid

        self.rotation_activation = torch.nn.functional.normalize

    def __init__(self, sh_degree : int, model_type: int):
        self.active_sh_degree = 0
        self.max_sh_degree = sh_degree
        self._xyz = torch.empty(0)
        self._features_dc = torch.empty(0)
        self._features_rest = torch.empty(0)
        self._scaling = torch.empty(0)
        self._rotation = torch.empty(0)
        self._opacity = torch.empty(0)
        self.max_radii2D = torch.empty(0)
        self.xyz_gradient_accum = torch.empty(0)
        self.denom = torch.empty(0)
        self.optimizer = None
        self.percent_dense = 0
        self.spatial_lr_scale = 0
        # model_type=0 upper teeth, move with back head anchor
        # model_type=1 lower teeth, move with jaw
        self.model_type = model_type

        self.setup_functions()

    def capture(self):
        return (
            self.active_sh_degree,
            self._xyz,
            self._features_dc,
            self._features_rest,
            self._scaling,
            self._rotation,
            self._opacity,
            self.max_radii2D,
            self.xyz_gradient_accum,
            self.denom,
            self.optimizer.state_dict(),
            self.spatial_lr_scale,
        )

    def restore(self, model_args, training_args):
        (self.active_sh_degree,
         self._xyz,
         self._features_dc,
         self._features_rest,
         self._scaling,
         self._rotation,
         self._opacity,
         self.max_radii2D,
         xyz_gradient_accum,
         denom,
         opt_dict,
         self.spatial_lr_scale) = model_args
        self.training_setup(training_args)
        self.xyz_gradient_accum = xyz_gradient_accum
        self.denom = denom
        # if self.optimizer is not None: # for test case ?
        #    self.optimizer.load_state_dict(opt_dict)

    def sample_possion_points(self,v,f,N):
        import point_cloud_utils as pcu
        f_i, bc = pcu.sample_mesh_poisson_disk(v,f,N)
        print('expect point number %d, get %d' % (N, bc.shape[0]))
        return f_i, bc

    ########################

    def prepare_xyz(self, params, args):

        pos_before = self._xyz
        if self.model_type == 0:
            ###### Move with anchors on the back of the head
            ## transfer position
            frameR = params.frameR.to(args.data_device).repeat(pos_before.shape[0],1,1)
            framet = params.framet.to(args.data_device)
            frameNR = params.frameNR.to(args.data_device).repeat(pos_before.shape[0],1,1)
            pos_after = torch.bmm(frameR, pos_before.unsqueeze(-1)).squeeze(-1) + framet[...,0]
            self.processed_xyz = pos_after
            ## rotate gaussians
            rotation = self.rotation_activation(self._rotation)
            rot_q = matrix_to_quaternion(frameNR)
            rot_q = quaternion_raw_multiply(rot_q, rotation)
            self.processed_rotation = rot_q
            ## rotate SH
            features_dc = self._features_dc
            features_rest = self._features_rest
            _features = torch.cat((features_dc, features_rest), dim=1)  # BxLx3
            self.processed_features = RotSH3_C3(frameNR, _features)

        elif self.model_type == 1:
            ###### Move with jaw
            ## transfer position
            rjawTrans = params.rjawTrans.to(args.data_device).repeat(pos_before.shape[0],1,1)
            rjawTransN = params.rjawTransN.to(args.data_device).repeat(pos_before.shape[0],1,1)
            pos_after = torch.bmm(rjawTrans[:,:3,:3], pos_before.unsqueeze(-1)).squeeze(-1) + rjawTrans[:,:3,3]
            self.processed_xyz = pos_after
            ## rotate gaussians
            rotation = self.rotation_activation(self._rotation)
            rot_q = matrix_to_quaternion(rjawTransN)
            rot_q = quaternion_raw_multiply(rot_q, rotation)
            self.processed_rotation = rot_q
            ## rotate SH
            features_dc = self._features_dc
            features_rest = self._features_rest
            _features = torch.cat((features_dc, features_rest), dim=1)  # BxLx3
            self.processed_features = RotSH3_C3(rjawTransN, _features)

    @property
    def get_scaling(self):
        return self.scaling_activation(self._scaling)

    @property
    def get_rotation(self):
        #return self.rotation_activation(self._rotation)
        return self.processed_rotation

    @property
    def get_xyz(self):
        return self.processed_xyz
        #return self._xyz

    @property
    def get_features(self):
        return self.processed_features
        #features_dc = self._features_dc
        #features_rest = self._features_rest
        #return torch.cat((features_dc, features_rest), dim=1)

    @property
    def get_opacity(self):
        return self.opacity_activation(self._opacity)

    # def get_covariance(self, scaling_modifier = 1):
    #     return self.covariance_activation(self.get_scaling, scaling_modifier, self._rotation)

    def oneupSHdegree(self):
        if self.active_sh_degree < self.max_sh_degree:
            self.active_sh_degree += 1

    # def create_from_pcd(self, pcd : BasicPointCloud, spatial_lr_scale : float):
    #     self.spatial_lr_scale = spatial_lr_scale
    #     fused_point_cloud = torch.tensor(np.asarray(pcd.points)).float().cuda()
    #     fused_color = RGB2SH(torch.tensor(np.asarray(pcd.colors)).float().cuda())
    #     features = torch.zeros((fused_color.shape[0], 3, (self.max_sh_degree + 1) ** 2)).float().cuda()
    #     features[:, :3, 0 ] = fused_color
    #     features[:, 3:, 1:] = 0.0
    #
    #     print("Number of points at initialisation : ", fused_point_cloud.shape[0])
    #
    #     dist2 = torch.clamp_min(distCUDA2(torch.from_numpy(np.asarray(pcd.points)).float().cuda()), 0.0000001)
    #     scales = torch.log(torch.sqrt(dist2))[...,None].repeat(1, 3)
    #     rots = torch.zeros((fused_point_cloud.shape[0], 4), device="cuda")
    #     rots[:, 0] = 1
    #
    #     opacities = inverse_sigmoid(0.1 * torch.ones((fused_point_cloud.shape[0], 1), dtype=torch.float, device="cuda"))
    #
    #     self._xyz = nn.Parameter(fused_point_cloud.requires_grad_(True))
    #     self._features_dc = nn.Parameter(features[:,:,0:1].transpose(1, 2).contiguous().requires_grad_(True))
    #     self._features_rest = nn.Parameter(features[:,:,1:].transpose(1, 2).contiguous().requires_grad_(True))
    #     self._scaling = nn.Parameter(scales.requires_grad_(True))
    #     self._rotation = nn.Parameter(rots.requires_grad_(True))
    #     self._opacity = nn.Parameter(opacities.requires_grad_(True))
    #     self.max_radii2D = torch.zeros((self.get_xyz.shape[0]), device="cuda")

    def create_from_face(self, mesh_file, offset,
                         args, spatial_lr_scale: float, replace_v = None):

        self.spatial_lr_scale = spatial_lr_scale

        import point_cloud_utils as pcu

        v, f = pcu.load_mesh_vf(mesh_file)
        v = v + offset

        #face_id, barycentric_coord = pcu.sample_mesh_poisson_disk(v,f,int(args.init_face_point_number * 0.1))
        face_id, barycentric_coord = self.sample_possion_points(v, f, int(args.init_face_point_number * 0.1))

        p_face_id = f[face_id]
        v_id_xyz = v[p_face_id]
        point_vertices = (v_id_xyz * barycentric_coord[...,None]).sum(-2)
        point_vertices = torch.from_numpy(point_vertices).float().cuda()

        fused_color = (torch.rand(point_vertices.shape[0],3)/255.).float().cuda()
        features = torch.zeros((fused_color.shape[0], 3, (self.max_sh_degree + 1) ** 2)).float().cuda()
        features[:, :3, 0] = fused_color
        features[:, 3:, 1:] = 0.0

        dist2 = torch.clamp_min(distCUDA2(point_vertices), 0.0000001)
        scales = torch.log(torch.sqrt(dist2))[..., None].repeat(1, 3)
        rots = torch.zeros((point_vertices.shape[0], 4), device="cuda")
        rots[:, 0] = 1

        opacities = inverse_sigmoid(0.1 * torch.ones((point_vertices.shape[0], 1), dtype=torch.float, device="cuda"))

        self._xyz = nn.Parameter(point_vertices.requires_grad_(True))
        self._features_dc = nn.Parameter(features[:, :, 0:1].transpose(1, 2).contiguous().requires_grad_(True))
        self._features_rest = nn.Parameter(features[:, :, 1:].transpose(1, 2).contiguous().requires_grad_(True))
        self._scaling = nn.Parameter(scales.requires_grad_(True))
        self._rotation = nn.Parameter(rots.requires_grad_(True))
        self._opacity = nn.Parameter(opacities.requires_grad_(True))
        self.max_radii2D = torch.zeros((self._xyz.shape[0]), device="cuda")

    def training_setup(self, training_args):
        self.percent_dense = training_args.percent_dense
        self.xyz_gradient_accum = torch.zeros((self._xyz.shape[0], 1), device="cuda")
        self.denom = torch.zeros((self._xyz.shape[0], 1), device="cuda")

        if only_SH0:
            l = [
                {'params': [self._xyz], 'lr': training_args.position_lr_init * self.spatial_lr_scale, "name": "xyz"},
                {'params': [self._features_dc], 'lr': training_args.feature_lr, "name": "f_dc"},
                #{'params': [self._features_rest], 'lr': training_args.feature_lr / 20.0, "name": "f_rest"},
                {'params': [self._opacity], 'lr': training_args.opacity_lr, "name": "opacity"},
                {'params': [self._scaling], 'lr': training_args.scaling_lr, "name": "scaling"},
                {'params': [self._rotation], 'lr': training_args.rotation_lr, "name": "rotation"}
            ]
        else:
            l = [
                {'params': [self._xyz], 'lr': training_args.position_lr_init * self.spatial_lr_scale, "name": "xyz"},
                {'params': [self._features_dc], 'lr': training_args.feature_lr, "name": "f_dc"},
                {'params': [self._features_rest], 'lr': training_args.feature_lr / 20.0, "name": "f_rest"},
                {'params': [self._opacity], 'lr': training_args.opacity_lr, "name": "opacity"},
                {'params': [self._scaling], 'lr': training_args.scaling_lr, "name": "scaling"},
                {'params': [self._rotation], 'lr': training_args.rotation_lr, "name": "rotation"}
            ]

        #self.optimizer = torch.optim.Adam(l, lr=0.0, eps=1e-15)
        self.optimizer = AdamAcc(l, lr=0.0, eps=1e-15)
        self.xyz_scheduler_args = get_expon_lr_func(lr_init=training_args.position_lr_init * self.spatial_lr_scale,
                                                    lr_final=training_args.position_lr_final * self.spatial_lr_scale,
                                                    lr_delay_mult=training_args.position_lr_delay_mult,
                                                    max_steps=training_args.position_lr_max_steps)

    def update_learning_rate(self, iteration):
        ''' Learning rate scheduling per step '''
        for param_group in self.optimizer.param_groups:
            if param_group["name"] == "xyz":
                lr = self.xyz_scheduler_args(iteration)
                param_group['lr'] = lr
                return lr

    def construct_list_of_attributes(self):
        l = ['x', 'y', 'z', 'nx', 'ny', 'nz']
        # All channels except the 3 DC
        for i in range(self._features_dc.shape[1]*self._features_dc.shape[2]):
            l.append('f_dc_{}'.format(i))
        for i in range(self._features_rest.shape[1]*self._features_rest.shape[2]):
            l.append('f_rest_{}'.format(i))
        l.append('opacity')
        for i in range(self._scaling.shape[1]):
            l.append('scale_{}'.format(i))
        for i in range(self._rotation.shape[1]):
            l.append('rot_{}'.format(i))
        return l

    # dump for C++ viewer
    def save_npy_forviewer(self, path):
        # seems no more things to do
        return self.save_npy(path)

    def save_npy(self, path):
        mkdir_p(path)
        features_dc = self._features_dc
        features_reset = self._features_rest
        _xyz = self._xyz
        _features = torch.cat((features_dc, features_reset),dim=1)
        _scaling = self._scaling
        _rotation = self._rotation
        _opacity = self._opacity

        np.save(os.path.join(path, "pos.npy"), _xyz.cpu().detach().numpy())
        np.save(os.path.join(path, "shs.npy"), _features.cpu().detach().numpy())  # 71600x16x3
        np.save(os.path.join(path, "scale.npy"), _scaling.cpu().detach().numpy())  # 71600x3
        np.save(os.path.join(path, "rot.npy"), _rotation.cpu().detach().numpy())  # 71600x4
        np.save(os.path.join(path, "opacity.npy"), _opacity.cpu().detach().numpy())  # 71600x1

    # def save_ply(self, path):
    #     mkdir_p(os.path.dirname(path))
    #
    #     xyz = self._xyz.detach().cpu().numpy()
    #
    #     normals = np.zeros_like(xyz)
    #     f_dc = self._features_dc.detach().transpose(1, 2).flatten(start_dim=1).contiguous().cpu().numpy()
    #     f_rest = self._features_rest.detach().transpose(1, 2).flatten(start_dim=1).contiguous().cpu().numpy()
    #     opacities = self._opacity.detach().cpu().numpy()
    #     scale = self._scaling.detach().cpu().numpy()
    #     rotation = self._rotation.detach().cpu().numpy()
    #
    #     dtype_full = [(attribute, 'f4') for attribute in self.construct_list_of_attributes()]
    #
    #     elements = np.empty(xyz.shape[0], dtype=dtype_full)
    #     attributes = np.concatenate((xyz, normals, f_dc, f_rest, opacities, scale, rotation), axis=1)
    #     elements[:] = list(map(tuple, attributes))
    #     el = PlyElement.describe(elements, 'vertex')
    #     PlyData([el]).write(path)

    def reset_opacity(self):
        opacities_new = inverse_sigmoid(torch.min(self.get_opacity, torch.ones_like(self.get_opacity) * 0.01))
        optimizable_tensors = self.replace_tensor_to_optimizer(opacities_new, "opacity")
        self._opacity = optimizable_tensors["opacity"]

    def load_ply(self, path):
        plydata = PlyData.read(path)

        xyz = np.stack((np.asarray(plydata.elements[0]["x"]),
                        np.asarray(plydata.elements[0]["y"]),
                        np.asarray(plydata.elements[0]["z"])), axis=1)
        opacities = np.asarray(plydata.elements[0]["opacity"])[..., np.newaxis]

        features_dc = np.zeros((xyz.shape[0], 3, 1))
        features_dc[:, 0, 0] = np.asarray(plydata.elements[0]["f_dc_0"])
        features_dc[:, 1, 0] = np.asarray(plydata.elements[0]["f_dc_1"])
        features_dc[:, 2, 0] = np.asarray(plydata.elements[0]["f_dc_2"])

        extra_f_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("f_rest_")]
        extra_f_names = sorted(extra_f_names, key=lambda x: int(x.split('_')[-1]))
        assert len(extra_f_names) == 3 * (self.max_sh_degree + 1) ** 2 - 3
        features_extra = np.zeros((xyz.shape[0], len(extra_f_names)))
        for idx, attr_name in enumerate(extra_f_names):
            features_extra[:, idx] = np.asarray(plydata.elements[0][attr_name])
        # Reshape (P,F*SH_coeffs) to (P, F, SH_coeffs except DC)
        features_extra = features_extra.reshape((features_extra.shape[0], 3, (self.max_sh_degree + 1) ** 2 - 1))

        scale_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("scale_")]
        scale_names = sorted(scale_names, key=lambda x: int(x.split('_')[-1]))
        scales = np.zeros((xyz.shape[0], len(scale_names)))
        for idx, attr_name in enumerate(scale_names):
            scales[:, idx] = np.asarray(plydata.elements[0][attr_name])

        rot_names = [p.name for p in plydata.elements[0].properties if p.name.startswith("rot")]
        rot_names = sorted(rot_names, key=lambda x: int(x.split('_')[-1]))
        rots = np.zeros((xyz.shape[0], len(rot_names)))
        for idx, attr_name in enumerate(rot_names):
            rots[:, idx] = np.asarray(plydata.elements[0][attr_name])

        self._xyz = nn.Parameter(torch.tensor(xyz, dtype=torch.float, device="cuda").requires_grad_(True))
        self._features_dc = nn.Parameter(torch.tensor(features_dc, dtype=torch.float, device="cuda").transpose(1, 2).contiguous().requires_grad_(True))
        self._features_rest = nn.Parameter(torch.tensor(features_extra, dtype=torch.float, device="cuda").transpose(1, 2).contiguous().requires_grad_(True))
        self._opacity = nn.Parameter(torch.tensor(opacities, dtype=torch.float, device="cuda").requires_grad_(True))
        self._scaling = nn.Parameter(torch.tensor(scales, dtype=torch.float, device="cuda").requires_grad_(True))
        self._rotation = nn.Parameter(torch.tensor(rots, dtype=torch.float, device="cuda").requires_grad_(True))

        self.active_sh_degree = self.max_sh_degree

    def replace_tensor_to_optimizer(self, tensor, name):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            if group["name"] == name:
                stored_state = self.optimizer.state.get(group['params'][0], None)
                stored_state["exp_avg"] = torch.zeros_like(tensor)
                stored_state["exp_avg_sq"] = torch.zeros_like(tensor)

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter(tensor.requires_grad_(True))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
        return optimizable_tensors

    def _prune_optimizer(self, mask):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            stored_state = self.optimizer.state.get(group['params'][0], None)
            if stored_state is not None:
                stored_state["exp_avg"] = stored_state["exp_avg"][mask]
                stored_state["exp_avg_sq"] = stored_state["exp_avg_sq"][mask]

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter((group["params"][0][mask].requires_grad_(True)))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
            else:
                group["params"][0] = nn.Parameter(group["params"][0][mask].requires_grad_(True))
                optimizable_tensors[group["name"]] = group["params"][0]
        return optimizable_tensors

    def prune_points(self, mask):
        valid_points_mask = ~mask
        optimizable_tensors = self._prune_optimizer(valid_points_mask)

        self._xyz = optimizable_tensors["xyz"]
        self._features_dc = optimizable_tensors["f_dc"]
        #self._features_rest = optimizable_tensors["f_rest"]
        if "f_rest" in optimizable_tensors:
            self._features_rest = optimizable_tensors["f_rest"]
        else:
            tmp = self._features_rest[valid_points_mask].detach()
            self._features_rest = nn.Parameter(tmp.requires_grad_(True))
        self._opacity = optimizable_tensors["opacity"]
        self._scaling = optimizable_tensors["scaling"]
        self._rotation = optimizable_tensors["rotation"]

        self.xyz_gradient_accum = self.xyz_gradient_accum[valid_points_mask]

        self.denom = self.denom[valid_points_mask]
        self.max_radii2D = self.max_radii2D[valid_points_mask]

    def cat_tensors_to_optimizer(self, tensors_dict):
        optimizable_tensors = {}
        for group in self.optimizer.param_groups:
            assert len(group["params"]) == 1
            extension_tensor = tensors_dict[group["name"]]
            stored_state = self.optimizer.state.get(group['params'][0], None)
            if stored_state is not None:

                stored_state["exp_avg"] = torch.cat((stored_state["exp_avg"], torch.zeros_like(extension_tensor)), dim=0)
                stored_state["exp_avg_sq"] = torch.cat((stored_state["exp_avg_sq"], torch.zeros_like(extension_tensor)), dim=0)

                del self.optimizer.state[group['params'][0]]
                group["params"][0] = nn.Parameter(torch.cat((group["params"][0], extension_tensor), dim=0).requires_grad_(True))
                self.optimizer.state[group['params'][0]] = stored_state

                optimizable_tensors[group["name"]] = group["params"][0]
            else:
                group["params"][0] = nn.Parameter(torch.cat((group["params"][0], extension_tensor), dim=0).requires_grad_(True))
                optimizable_tensors[group["name"]] = group["params"][0]

        return optimizable_tensors

    def densification_postfix(self, new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation,
                              new_xyz_gradient_accum = None, new_denom = None, new_max_radii2D = None):
        d = {"xyz": new_xyz,
             "f_dc": new_features_dc,
             "f_rest": new_features_rest,
             "opacity": new_opacities,
             "scaling": new_scaling,
             "rotation": new_rotation}

        optimizable_tensors = self.cat_tensors_to_optimizer(d)
        self._xyz = optimizable_tensors["xyz"]
        self._features_dc = optimizable_tensors["f_dc"]
        #self._features_rest = optimizable_tensors["f_rest"]
        if "f_rest" in optimizable_tensors:
            self._features_rest = optimizable_tensors["f_rest"]
        else:
            tmp = torch.cat([self._features_rest, new_features_rest],dim=0).detach()
            self._features_rest = nn.Parameter(tmp.requires_grad_(True))
        self._opacity = optimizable_tensors["opacity"]
        self._scaling = optimizable_tensors["scaling"]
        self._rotation = optimizable_tensors["rotation"]

        if new_xyz_gradient_accum is not None:
            self.xyz_gradient_accum = torch.cat([self.xyz_gradient_accum, new_xyz_gradient_accum],dim=0)
            self.denom = torch.cat([self.denom, new_denom],dim=0)
            self.max_radii2D = torch.cat([self.max_radii2D, new_max_radii2D], dim=0)
        else:
            self.xyz_gradient_accum = torch.zeros((self._scaling.shape[0], 1), device="cuda")
            self.denom = torch.zeros((self._scaling.shape[0], 1), device="cuda")
            self.max_radii2D = torch.zeros((self._scaling.shape[0]), device="cuda")

    def densify_and_split(self, grads, grad_threshold, scene_extent, N=2):
        #n_init_points = self.get_xyz.shape[0]
        n_init_points = self.get_scaling.shape[0]
        # Extract points that satisfy the gradient condition
        padded_grad = torch.zeros((n_init_points), device="cuda")
        padded_grad[:grads.shape[0]] = grads.squeeze()
        selected_pts_mask = torch.where(padded_grad >= grad_threshold, True, False)
        selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(self.get_scaling, dim=1).values > self.percent_dense * scene_extent)

        stds = self.get_scaling[selected_pts_mask].repeat(N, 1)
        means = torch.zeros((stds.size(0), 3), device="cuda")
        samples = torch.normal(mean=means, std=stds)
        rots = build_rotation(self._rotation[selected_pts_mask]).repeat(N, 1, 1)
        #new_xyz = torch.bmm(rots, samples.unsqueeze(-1)).squeeze(-1) + self.get_xyz[selected_pts_mask].repeat(N, 1)
        new_xyz = torch.bmm(rots, samples.unsqueeze(-1)).squeeze(-1) + self._xyz[selected_pts_mask].repeat(N, 1)
        new_scaling = self.scaling_inverse_activation(self.get_scaling[selected_pts_mask].repeat(N, 1) / (0.8 * N))
        new_rotation = self._rotation[selected_pts_mask].repeat(N, 1)
        new_features_dc = self._features_dc[selected_pts_mask].repeat(N, 1, 1)
        new_features_rest = self._features_rest[selected_pts_mask].repeat(N, 1, 1)
        new_opacity = self._opacity[selected_pts_mask].repeat(N, 1)

        self.densification_postfix(new_xyz, new_features_dc, new_features_rest, new_opacity, new_scaling, new_rotation)

        prune_filter = torch.cat((selected_pts_mask, torch.zeros(N * selected_pts_mask.sum(), device="cuda", dtype=bool)))
        self.prune_points(prune_filter)

    def densify_and_clone(self, grads, grad_threshold, scene_extent):
        # Extract points that satisfy the gradient condition
        selected_pts_mask = torch.where(torch.norm(grads, dim=-1) >= grad_threshold, True, False)
        selected_pts_mask = torch.logical_and(selected_pts_mask, torch.max(self.get_scaling, dim=1).values <= self.percent_dense * scene_extent)

        new_xyz = self._xyz[selected_pts_mask]
        new_features_dc = self._features_dc[selected_pts_mask]
        new_features_rest = self._features_rest[selected_pts_mask]
        new_opacities = self._opacity[selected_pts_mask]
        new_scaling = self._scaling[selected_pts_mask]
        new_rotation = self._rotation[selected_pts_mask]

        self.densification_postfix(new_xyz, new_features_dc, new_features_rest, new_opacities, new_scaling, new_rotation)

    def densify_and_prune(self, max_grad, min_opacity, extent, max_screen_size):
        grads = self.xyz_gradient_accum / self.denom
        grads[grads.isnan()] = 0.0

        self.densify_and_clone(grads, max_grad, extent)
        self.densify_and_split(grads, max_grad, extent)

        prune_mask = (self.get_opacity < min_opacity).squeeze()
        if max_screen_size:
            big_points_vs = self.max_radii2D > max_screen_size  # have BUG !!
            big_points_ws = self.get_scaling.max(dim=1).values > 0.1 * extent
            prune_mask = torch.logical_or(torch.logical_or(prune_mask, big_points_vs), big_points_ws)
        self.prune_points(prune_mask)

        torch.cuda.empty_cache()

    def add_densification_stats(self, viewspace_point_tensor, update_filter):
        self.xyz_gradient_accum[update_filter] += torch.norm(viewspace_point_tensor.grad[update_filter, :2], dim=-1, keepdim=True)
        self.denom[update_filter] += 1







```

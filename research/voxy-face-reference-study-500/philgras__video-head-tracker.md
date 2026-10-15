# Facial rendering source scan: philgras/video-head-tracker

Repository: https://github.com/philgras/video-head-tracker

Pinned source revision: 57b9ebb8e25386751c7c5ab42596d62ee169adcd

Discovery topic: FLAME 3D

Description: Python library for tracking human heads with FLAME (a 3D morphable head model)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["assets/flame/landmark_embedding_with_eyes.npy"], "flame": ["assets/flame/landmark_embedding_with_eyes.npy", "assets/flame/lower_neck_face_idcs.npy", "vht/model/flame.py"], "lbs": ["vht/model/lbs.py"], "face": ["assets/flame/lower_neck_face_idcs.npy"]}

## Source file: vht/model/lbs.py

{
  "path": "vht/model/lbs.py",
  "blob_sha": "da67e063618af0ef7ed00fca4990b71b2218e066",
  "sha256": "a140c90c1bd6f52f0382a0f1e8ef4d68afa126b9cc55398d37e894970f4aa9ea",
  "lines": 310,
  "bytes": 9986,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 2,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 12,
    "face": 10
  },
  "url": "https://github.com/philgras/video-head-tracker/blob/57b9ebb8e25386751c7c5ab42596d62ee169adcd/vht/model/lbs.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 60,
      "text": "def vertices2landmarks(vertices, faces, lmk_faces_idx, lmk_bary_coords):"
    },
    {
      "line": 101,
      "text": "def lbs("
    },
    {
      "line": 204,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 224,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 248,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 260,
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
from __future__ import print_function
from __future__ import division

import torch
import torch.nn.functional as F


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
    K = torch.cat([zeros, -rz, ry, rz, zeros, -rx, -ry, rx, zeros], dim=1).view(
        (batch_size, 3, 3)
    )

    ident = torch.eye(3, dtype=dtype, device=device).unsqueeze(dim=0)
    rot_mat = ident + sin * K + (1 - cos) * torch.bmm(K, K)
    return rot_mat


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

    lmk_faces = torch.index_select(faces, 0, lmk_faces_idx.view(-1)).view(
        batch_size, -1, 3
    )

    lmk_faces += (
        torch.arange(batch_size, dtype=torch.long, device=device).view(-1, 1, 1)
        * num_verts
    )

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
    faces=None,
):
    """Performs Linear Blend Skinning with the given shape and pose parameters

    Parameters
    ----------
    betas : torch.tensor BxNB
        The tensor of shape parameters
    pose : torch.tensor Bx(J + 1) * 3
        The pose parameters in axis-angle format
    v_template: torch.tensor BxVx3
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
        rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view(
            [batch_size, -1, 3, 3]
        )

        pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1])
        # (N x P) x (P, V * 3) -> N x V x 3
        pose_offsets = torch.matmul(pose_feature, posedirs).view(batch_size, -1, 3)
    else:
        pose_feature = pose[:, 1:].view(batch_size, -1, 3, 3) - ident
        rot_mats = pose.view(batch_size, -1, 3, 3)

        pose_offsets = torch.matmul(pose_feature.view(batch_size, -1), posedirs).view(
            batch_size, -1, 3
        )

    v_posed = pose_offsets + v_shaped

    # 4. Get the global joint location
    J_transformed, A = batch_rigid_transform(rot_mats, J, parents, dtype=dtype)

    # 5. Do skinning:
    # W is N x V x (J + 1)
    W = lbs_weights.unsqueeze(dim=0).expand([batch_size, -1, -1])
    # (N x V x (J + 1)) x (N x (J + 1) x 16)
    num_joints = J_regressor.shape[0]
    T = torch.matmul(W, A.view(batch_size, num_joints, 16)).view(batch_size, -1, 4, 4)

    homogen_coord = torch.ones(
        [batch_size, v_posed.shape[1], 1], dtype=dtype, device=device
    )
    v_posed_homo = torch.cat([v_posed, homogen_coord], dim=2)
    v_homo = torch.matmul(T, torch.unsqueeze(v_posed_homo, dim=-1))

    verts = v_homo[:, :, :3, 0]

    return verts, J_transformed, A[:, 1]


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

    rel_joints = joints.clone().contiguous()
    rel_joints[:, 1:] = rel_joints[:, 1:] - joints[:, parents[1:]]

    transforms_mat = transform_mat(rot_mats.view(-1, 3, 3), rel_joints.view(-1, 3, 1))
    transforms_mat = transforms_mat.view(-1, joints.shape[1], 4, 4)

    transform_chain = [transforms_mat[:, 0]]
    for i in range(1, parents.shape[0]):
        # Subtract the joint location at the rest pose
        # No need for rotation, since it's identity when at rest
        curr_res = torch.matmul(transform_chain[parents[i]], transforms_mat[:, i])
        transform_chain.append(curr_res)

    transforms = torch.stack(transform_chain, dim=1)

    # The last column of the transformations contains the posed joints
    posed_joints = transforms[:, :, :3, 3]

    joints_homogen = F.pad(joints, [0, 0, 0, 1])

    rel_transforms = transforms - F.pad(
        torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0]
    )

    return posed_joints, rel_transforms


```

## Source file: vht/model/flame.py

{
  "path": "vht/model/flame.py",
  "blob_sha": "689e6220ea43721adc12239e60411bb5dfe4dc29",
  "sha256": "504f1800de0c656de451b857d3e96fa557a27de3a05537e45bbcef33842a5107",
  "lines": 319,
  "bytes": 12019,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 18,
    "jaw": 15,
    "flame": 25,
    "lbs": 4,
    "rig": 8,
    "face": 14,
    "lip": 1
  },
  "url": "https://github.com/philgras/video-head-tracker/blob/57b9ebb8e25386751c7c5ab42596d62ee169adcd/vht/model/flame.py",
  "function_signature_candidates": [
    {
      "line": 40,
      "text": "def to_tensor(array, dtype=torch.float32):"
    },
    {
      "line": 45,
      "text": "def to_np(array, dtype=np.float32):"
    },
    {
      "line": 52,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 63,
      "text": "def __init__("
    },
    {
      "line": 159,
      "text": "def _apply_rotation_limit(self, rotation, limit):"
    },
    {
      "line": 164,
      "text": "def apply_rotation_limits(self, neck, jaw, eyes):"
    },
    {
      "line": 181,
      "text": "def _revert_rotation_limit(self, rotation, limit):"
    },
    {
      "line": 200,
      "text": "def revert_rotation_limits(self, neck, jaw, eyes):"
    },
    {
      "line": 215,
      "text": "def get_neutral_joint_rotations(self):"
    },
    {
      "line": 227,
      "text": "def forward("
    },
    {
      "line": 299,
      "text": "def __init__(self, tex_params, tex_size=256, tex_space_path=FLAME_TEX_PATH):"
    },
    {
      "line": 313,
      "text": "def forward(self, texcode):"
    }
  ]
}

```text

# Code heavily inspired by https://github.com/HavenFeng/photometric_optimization/blob/master/models/FLAME.py.
# Please consider citing their work if you find this code useful. The code is subject to the license available via
# https://github.com/vchoutas/smplx/edit/master/LICENSE

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


from vht.model.lbs import lbs, vertices2landmarks
from vht.util.graphics import face_vertices
from vht.util.log import get_logger
from pytorch3d.io import load_obj

import torch
import torch.nn as nn
import numpy as np
import pickle
import torch.nn.functional as F

logger = get_logger(__name__)

FLAME_MODEL_PATH = "assets/flame/generic_model.pkl"
FLAME_MESH_PATH = "assets/flame/head_template_mesh.obj"
FLAME_PARTS_PATH = "assets/flame/FLAME_masks.pkl"
FLAME_LMK_PATH = "assets/flame/landmark_embedding_with_eyes.npy"
FLAME_TEX_PATH = "assets/flame/FLAME_texture.npz"


def to_tensor(array, dtype=torch.float32):
    if "torch.tensor" not in str(type(array)):
        return torch.tensor(array, dtype=dtype)


def to_np(array, dtype=np.float32):
    if "scipy.sparse" in str(type(array)):
        array = array.todense()
    return np.array(array, dtype=dtype)


class Struct(object):
    def __init__(self, **kwargs):
        for key, val in kwargs.items():
            setattr(self, key, val)


class FlameHead(nn.Module):
    """
    Given flame parameters this class generates a differentiable FLAME function
    which outputs the a mesh and 2D/3D facial landmarks
    """

    def __init__(
        self,
        shape_params,
        expr_params,
        flame_model_path=FLAME_MODEL_PATH,
        flame_lmk_embedding_path=FLAME_LMK_PATH,
        flame_template_mesh_path=FLAME_MESH_PATH,
        eye_limits=((-50, 50), (-50, 50), (-0.1, 0.1)),
        neck_limits=((-90, 90), (-60, 60), (-80, 80)),
        jaw_limits=((-5, 60), (-0.1, 0.1), (-0.1, 0.1)),
    ):
        super().__init__()

        self.n_shape_params = shape_params
        self.n_expr_params = expr_params

        with open(flame_model_path, "rb") as f:
            ss = pickle.load(f, encoding="latin1")
            flame_model = Struct(**ss)

        self.dtype = torch.float32
        # The vertices of the template model
        self.register_buffer(
            "v_template", to_tensor(to_np(flame_model.v_template), dtype=self.dtype)
        )

        # The shape components and expression
        shapedirs = to_tensor(to_np(flame_model.shapedirs), dtype=self.dtype)
        shapedirs = torch.cat(
            [shapedirs[:, :, :shape_params], shapedirs[:, :, 300 : 300 + expr_params]],
            2,
        )
        self.register_buffer("shapedirs", shapedirs)

        # The pose components
        num_pose_basis = flame_model.posedirs.shape[-1]
        posedirs = np.reshape(flame_model.posedirs, [-1, num_pose_basis]).T
        self.register_buffer("posedirs", to_tensor(to_np(posedirs), dtype=self.dtype))
        #
        self.register_buffer(
            "J_regressor", to_tensor(to_np(flame_model.J_regressor), dtype=self.dtype)
        )
        parents = to_tensor(to_np(flame_model.kintree_table[0])).long()
        parents[0] = -1
        self.register_buffer("parents", parents)
        self.register_buffer(
            "lbs_weights", to_tensor(to_np(flame_model.weights), dtype=self.dtype)
        )

        # rotation limits for the respective joints
        neck_limits = torch.tensor(neck_limits).float() / 180 * np.pi
        self.register_buffer("neck_limits", neck_limits)
        jaw_limits = torch.tensor(jaw_limits).float() / 180 * np.pi
        self.register_buffer("jaw_limits", jaw_limits)
        eye_limits = torch.tensor(eye_limits).float() / 180 * np.pi
        self.register_buffer("eye_limits", eye_limits)

        # Landmark embeddings for FLAME
        lmk_embeddings = np.load(
            flame_lmk_embedding_path, allow_pickle=True, encoding="latin1"
        )
        lmk_embeddings = lmk_embeddings[()]
        self.register_buffer(
            "full_lmk_faces_idx",
            torch.tensor(lmk_embeddings["full_lmk_faces_idx"], dtype=torch.long),
        )
        self.register_buffer(
            "full_lmk_bary_coords",
            torch.tensor(lmk_embeddings["full_lmk_bary_coords"], dtype=self.dtype),
        )

        neck_kin_chain = []
        NECK_IDX = 1
        curr_idx = torch.tensor(NECK_IDX, dtype=torch.long)
        while curr_idx != -1:
            neck_kin_chain.append(curr_idx)
            curr_idx = self.parents[curr_idx]
        self.register_buffer("neck_kin_chain", torch.stack(neck_kin_chain))

        # add faces and uvs
        _, faces, aux = load_obj(flame_template_mesh_path, load_textures=False)
        vertex_uvs = aux.verts_uvs
        face_uvs_idx = faces.textures_idx  # index into verts_uvs

        # create uvcoords per face --> this is what you can use for uv map rendering
        # range from -1 to 1 (-1, -1) = left top; (+1, +1) = right bottom
        # pad 1 to the end
        pad = torch.ones(vertex_uvs.shape[0], 1)
        vertex_uvs = torch.cat([vertex_uvs, pad], dim=-1)
        vertex_uvs = vertex_uvs * 2 - 1
        vertex_uvs[..., 1] = -vertex_uvs[..., 1]

        face_uv_coords = face_vertices(vertex_uvs[None], face_uvs_idx[None])[0]
        self.register_buffer("face_uvcoords", face_uv_coords, persistent=False)
        self.register_buffer("faces", faces.verts_idx, persistent=False)

    def _apply_rotation_limit(self, rotation, limit):
        r_min, r_max = limit[:, 0].view(1, 3), limit[:, 1].view(1, 3)
        diff = r_max - r_min
        return r_min + (torch.tanh(rotation) + 1) / 2 * diff

    def apply_rotation_limits(self, neck, jaw, eyes):
        """
        method to call for applying rotation limits. Don't use _apply_rotation_limit() in other methods as this
        might cause some bugs if we change which poses are affected by rotation limits. For this reason, in this method,
        all affected poses are limited within one function so that if we add more restricted poses, they can just be
        updated here
        :param neck:
        :param jaw:
        :return:
        """
        neck = self._apply_rotation_limit(neck, self.neck_limits)
        jaw = self._apply_rotation_limit(jaw, self.jaw_limits)
        eye_r = self._apply_rotation_limit(eyes[:, :3], self.eye_limits)
        eye_l = self._apply_rotation_limit(eyes[:, 3:], self.eye_limits)
        eyes = torch.cat([eye_r, eye_l], dim=1)
        return neck, jaw, eyes

    def _revert_rotation_limit(self, rotation, limit):
        """
        inverse function of _apply_rotation_limit()
        from rotation angle vector (rodriguez) -> scalars from -inf ... inf
        :param rotation: tensor of shape N x 3
        :param limit: tensor of shape 3 x 2 (min, max)
        :return:
        """
        r_min, r_max = limit[:, 0].view(1, 3), limit[:, 1].view(1, 3)
        diff = r_max - r_min
        rotation = rotation.clone()
        for i in range(3):
            rotation[:, i] = torch.clip(
                rotation[:, i],
                min=r_min[0, i] + diff[0, i] * 0.01,
                max=r_max[0, i] - diff[0, i] * 0.01,
            )
        return torch.atanh((rotation - r_min) / diff * 2 - 1)

    def revert_rotation_limits(self, neck, jaw, eyes):
        """
        inverse function of apply_rotation_limits()
        from rotation angle vector (rodriguez) -> scalars from -inf ... inf
        :param rotation:
        :param limit:
        :return:
        """
        neck = self._revert_rotation_limit(neck, self.neck_limits)
        jaw = self._revert_rotation_limit(jaw, self.jaw_limits)
        eye_r = self._revert_rotation_limit(eyes[:, :3], self.eye_limits)
        eye_l = self._revert_rotation_limit(eyes[:, 3:], self.eye_limits)
        eyes = torch.cat([eye_r, eye_l], dim=1)
        return neck, jaw

    def get_neutral_joint_rotations(self):
        res = {}
        for name, limit in zip(
            ["neck", "jaw", "eyes"],
            [self.neck_limits, self.jaw_limits, self.eye_limits],
        ):
            r_min, r_max = limit[:, 0], limit[:, 1]
            diff = r_max - r_min
            res[name] = torch.atanh(-2 * r_min / diff - 1)
            # assert (r_min + (torch.tanh(res[name]) + 1) / 2 * diff) < 1e-7
        return res

    def forward(
        self,
        shape,
        expr,
        rotation,
        neck,
        jaw,
        eyes,
        translation,
        zero_centered=True,
        use_rotation_limits=True,
        return_landmarks=True,
    ):
        """
        Input:
            shape_params: N X number of shape parameters
            expression_params: N X number of expression parameters
            pose_params: N X number of pose parameters (6)
        return:d
            vertices: N X V X 3
            landmarks: N X number of landmarks X 3
        """
        batch_size = shape.shape[0]

        # apply limits to joint rotations
        if use_rotation_limits:
            neck, jaw, eyes = self.apply_rotation_limits(neck, jaw, eyes)

        betas = torch.cat([shape, expr], dim=1)
        full_pose = torch.cat([rotation, neck, jaw, eyes], dim=1)
        template_vertices = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)

        faces = self.faces.unsqueeze(0).expand(batch_size, -1, -1)
        vertices, J, mat_rot = lbs(
            betas,
            full_pose,
            template_vertices,
            self.shapedirs,
            self.posedirs,
            self.J_regressor,
            self.parents,
            self.lbs_weights,
            dtype=self.dtype,
            faces=faces,
        )

        if zero_centered:
            vertices = vertices - J[:, [0]]
            J = J - J[:, [0]]

        vertices = vertices + translation[:, None, :]
        J = J + translation[:, None, :]
        ret_vals = [vertices]

        # compute landmarks if desired
        if return_landmarks:
            bz = vertices.shape[0]
            landmarks = vertices2landmarks(
                vertices,
                self.faces,
                self.full_lmk_faces_idx.repeat(bz, 1),
                self.full_lmk_bary_coords.repeat(bz, 1, 1),
            )
            ret_vals.append(landmarks)

        if len(ret_vals) > 1:
            return ret_vals
        else:
            return ret_vals[0]


class FlameTex(nn.Module):
    def __init__(self, tex_params, tex_size=256, tex_space_path=FLAME_TEX_PATH):
        super().__init__()
        self._tex_size = tex_size
        tex_params = tex_params
        tex_space = np.load(tex_space_path)
        texture_mean = tex_space["mean"].reshape(1, -1)
        texture_basis = tex_space["tex_dir"].reshape(-1, 200)
        texture_mean = torch.from_numpy(texture_mean).float()[None, ...]
        texture_basis = torch.from_numpy(texture_basis[:, :tex_params]).float()[
            None, ...
        ]
        self.register_buffer("texture_mean", texture_mean)
        self.register_buffer("texture_basis", texture_basis)

    def forward(self, texcode):
        texture = self.texture_mean + (self.texture_basis * texcode[:, None, :]).sum(-1)
        texture = texture.reshape(texcode.shape[0], 512, 512, 3).permute(0, 3, 1, 2)
        texture = F.interpolate(texture, [self._tex_size, self._tex_size])
        texture = texture[:, [2, 1, 0], :, :]
        texture = texture / 255.0
        return texture.clamp(0, 1)


```

## Source file: setup.py

{
  "path": "setup.py",
  "blob_sha": "8f699d362cb1113bcf2047bbb6d5861c79fcfed0",
  "sha256": "ebc1bc575c1fd7ffbdf1330694e77a635cdeb7d7f431bce0f9f0bce444e1bbff",
  "lines": 21,
  "bytes": 731,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/philgras/video-head-tracker/blob/57b9ebb8e25386751c7c5ab42596d62ee169adcd/setup.py",
  "function_signature_candidates": []
}

```text

from setuptools import setup, find_packages

setup(
    name="vht",
    version="0.1.0",
    packages=find_packages(),
    python_requires=">=3.9",
    install_requires=[
        "ConfigArgParse",
        "torch@https://download.pytorch.org/whl/cu113/torch-1.10.2%2Bcu113-cp39-cp39-linux_x86_64.whl",
        "torchvision@https://download.pytorch.org/whl/cu113/torchvision-0.11.3%2Bcu113-cp39-cp39-linux_x86_64.whl",
        "pytorch3d@https://dl.fbaipublicfiles.com/pytorch3d/packaging/wheels/py39_cu113_pyt1100/pytorch3d-0.6.1-cp39-cp39-linux_x86_64.whl",
        "matplotlib",
        "tensorboard",
        "scipy",
        "opencv-python",
        "chumpy",
        "face-alignment",
        "face-detection-tflite",
    ],
)


```

## Source file: vht/util/log.py

{
  "path": "vht/util/log.py",
  "blob_sha": "a6c84881942c02d71344120b3f96b1e0ff618cdc",
  "sha256": "7fd28b43e112ec3dcdbe7add25c7024abb0e3e68f9a2b96ef298c000491519fe",
  "lines": 78,
  "bytes": 2860,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/philgras/video-head-tracker/blob/57b9ebb8e25386751c7c5ab42596d62ee169adcd/vht/util/log.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def _colored(msg, color):"
    },
    {
      "line": 19,
      "text": "def formatMessage(self, record):"
    },
    {
      "line": 35,
      "text": "def get_logger(name, level=logging.DEBUG, root=False, log_dir=None):"
    }
  ]
}

```text

import logging
import sys
from datetime import datetime
import atexit
from pathlib import Path


def _colored(msg, color):
    colors = {'red': '\033[91m', 'green': '\033[92m', 'yellow': '\033[93m', 'normal': '\033[0m'}
    return colors[color] + msg + colors["normal"]


class ColorFormatter(logging.Formatter):
    """
    Class to make command line log entries more appealing
    Inspired by https://github.com/facebookresearch/detectron2
    """

    def formatMessage(self, record):
        """
        Print warnings yellow and errors red
        :param record:
        :return:
        """
        log = super().formatMessage(record)
        if record.levelno == logging.WARNING:
            prefix = _colored("WARNING", "yellow")
        elif record.levelno == logging.ERROR or record.levelno == logging.CRITICAL:
            prefix = _colored("ERROR", "red")
        else:
            return log
        return prefix + " " + log


def get_logger(name, level=logging.DEBUG, root=False, log_dir=None):
    """
    Replaces the standard library logging.getLogger call in order to make some configuration
    for all loggers.
    :param name: pass the __name__ variable
    :param level: the desired log level
    :param root: call only once in the program
    :param log_dir: if root is set to True, this defines the directory where a log file is going
                    to be created that contains all logging output
    :return: the logger object
    """
    logger = logging.getLogger(name)
    logger.setLevel(level)

    if root:
        # create handler for console
        console_handler = logging.StreamHandler(sys.stdout)
        console_handler.setLevel(level)
        formatter = ColorFormatter(_colored("[%(asctime)s %(name)s]: ", "green") + "%(message)s",
                                   datefmt="%m/%d %H:%M:%S")
        console_handler.setFormatter(formatter)
        logger.addHandler(console_handler)

        if log_dir is not None:
            # add handler to log to a file
            log_dir = Path(log_dir)
            if not log_dir.exists():
                logger.info(f"Logging directory {log_dir} does not exist and will be created")
                log_dir.mkdir(parents=True)
            timestamp = datetime.now().strftime("%d-%m-%Y_%H-%M-%S")
            log_file = log_dir / f"{timestamp}.log"

            # open stream and make sure it will be closed
            stream = log_file.open(mode="w")
            atexit.register(stream.close)

            formatter = logging.Formatter("[%(asctime)s] %(name)s %(levelname)s: %(message)s",
                                          datefmt="%m/%d %H:%M:%S")
            file_handler = logging.StreamHandler(stream)
            file_handler.setLevel(level)
            file_handler.setFormatter(formatter)
            logger.addHandler(file_handler)

    return logger


```

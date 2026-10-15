# Facial rendering source scan: shivangi-aneja/ClipFace

Repository: https://github.com/shivangi-aneja/ClipFace

Pinned source revision: 95b977a6e413ca9c59f0b2f8d93b591c30cc1937

Discovery topic: face model 3D

Description: [SIGGRAPH 2023] ClipFace: Text-guided Editing of Textured 3D Morphable Models

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt"]

Topic paths: {"flame": ["model/flame/flame_model.py", "model/flame/lbs.py"], "lbs": ["model/flame/lbs.py"], "shader": ["model/renderer/illumination/shader.py"], "face": ["config/clipface.yaml", "dataset/face_image.py", "dataset/face_mesh.py", "dataset/face_mesh_video.py"], "lip": ["config/clipface.yaml", "criteria/clip_loss.py"]}

## Source file: model/flame/lbs.py

{
  "path": "model/flame/lbs.py",
  "blob_sha": "d23e00ee6826a5a27b4d0318910dd84fd0c2cd70",
  "sha256": "190618b45edcf9e52ba573a2a3b0d7e5b009d4148ae2d2afbdfb1b011074424b",
  "lines": 223,
  "bytes": 7850,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 2,
    "skin": 3,
    "blend_shape": 4,
    "deform": 2,
    "lbs": 4,
    "rig": 7
  },
  "url": "https://github.com/shivangi-aneja/ClipFace/blob/95b977a6e413ca9c59f0b2f8d93b591c30cc1937/model/flame/lbs.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def vertices2joints(J_regressor, vertices):"
    },
    {
      "line": 25,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 59,
      "text": "def blend_shapes(betas, shape_disps):"
    },
    {
      "line": 83,
      "text": "def transform_mat(R, t):"
    },
    {
      "line": 96,
      "text": "def batch_rigid_transform(rot_mats, joints, parents, dtype=torch.float32):"
    },
    {
      "line": 143,
      "text": "def lbs(betas, pose, v_template, shapedirs, posedirs, J_regressor, parents,"
    }
  ]
}

```text

import torch
import torch.nn.functional as F


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

    transforms_mat = transform_mat(rot_mats.view(-1, 3, 3), rel_joints.contiguous().view(-1, 3, 1)).view(-1, joints.shape[1], 4, 4)

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
    rel_transforms = transforms - F.pad(torch.matmul(transforms, joints_homogen), [3, 0, 0, 0, 0, 0, 0, 0])
    return posed_joints, rel_transforms


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
        rot_mats = batch_rodrigues(pose.view(-1, 3), dtype=dtype).view([batch_size, -1, 3, 3])
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

```

## Source file: dataset/face_mesh.py

{
  "path": "dataset/face_mesh.py",
  "blob_sha": "37a23dcd8be5804d55d05aabc066881e1f99200e",
  "sha256": "b777301a45a9ee6c3775a73bab3697a8ca235a0d6df0b6e88af4db37667e4223",
  "lines": 78,
  "bytes": 3113,
  "selection_score": 15,
  "keyword_line_counts": {
    "flame": 11,
    "face": 4
  },
  "url": "https://github.com/shivangi-aneja/ClipFace/blob/95b977a6e413ca9c59f0b2f8d93b591c30cc1937/dataset/face_mesh.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, w_codes_pth, mesh_path, flame_params, verts_pth, deca_warped_path, cam_mode='orthographic', viewpoint_x=None, mode='train'):"
    },
    {
      "line": 41,
      "text": "def set_device(self, device):"
    },
    {
      "line": 48,
      "text": "def __len__(self):"
    },
    {
      "line": 51,
      "text": "def __getitem__(self, index):"
    }
  ]
}

```text

import random
import torch
import pickle
import numpy as np
from glob import glob
from util.misc import get_random_perspective_view, load_mesh, get_orthographic_view


class FaceMesh(torch.utils.data.Dataset):

    def __init__(self, w_codes_pth, mesh_path, flame_params, verts_pth, deca_warped_path, cam_mode='orthographic', viewpoint_x=None, mode='train'):
        super().__init__()
        self.device = None
        self.mode = mode
        self.cam_mode = cam_mode
        self.viewpoint_x = viewpoint_x
        _, faces, uvs, uv_indices = load_mesh(mesh_path)
        self.faces = torch.from_numpy(faces).int()
        self.uvs = torch.from_numpy(uvs).float()
        self.uvs = torch.cat([self.uvs[:, 0:1], 1 - self.uvs[:, 1:2]], dim=1)
        self.uv_indices = torch.from_numpy(uv_indices).int()

        # The list of latent codes
        self.w_codes = np.array(sorted(glob(w_codes_pth + '*.pt')))

        # Get the tform from deca
        with open(deca_warped_path, 'rb') as f:
            data_dict = pickle.load(f)
        self.tform = torch.tensor(data_dict['tform'].params).float()

        # Get the vertices
        with open(verts_pth, 'rb') as f:
            verts = pickle.load(f)
        self.vertices = torch.from_numpy(verts).float()

        # Get Flame Vertices
        with open(flame_params, 'rb') as f:
            flame_params = pickle.load(f)
        self.flame_params = flame_params

    def set_device(self, device):
        self.device = device
        self.vertices = self.vertices.to(device)
        self.uvs = self.uvs.to(device)
        self.faces = self.faces.to(device)
        self.uv_indices = self.uv_indices.to(device)

    def __len__(self):
        return len(self.w_codes)

    def __getitem__(self, index):
        projection_matrix, view_matrix = None, None
        if self.cam_mode == 'perspective':
            projection_matrix, view_matrix = get_random_perspective_view()
        elif self.cam_mode == 'orthographic':
            if self.mode == 'val':
                pos_x = 0
            else:
                pos_x = random.uniform(-2, 2)
            pos_y = 0
            if self.viewpoint_x is not None:
                projection_matrix, view_matrix = get_orthographic_view(pos_x=self.viewpoint_x[index], pos_y=pos_y)
            else:
                projection_matrix, view_matrix = get_orthographic_view(pos_x=pos_x, pos_y=pos_y)
        w_code = torch.load(self.w_codes[index])

        return {
            'name': f"{index:06d}",
            'w_code': w_code,
            'flame_shape': torch.from_numpy(self.flame_params['shape']).float(),
            'flame_exp': torch.from_numpy(self.flame_params['expression']).float(),
            'flame_pose': torch.from_numpy(self.flame_params['pose']).float(),
            'flame_tex': torch.from_numpy(self.flame_params['tex']).float(),
            'flame_cam': torch.from_numpy(self.flame_params['camera']).float(),
            'flame_light': torch.from_numpy(self.flame_params['light']).float(),
            "projection_matrix": projection_matrix,
            "view_matrix": view_matrix,
        }


```

## Source file: criteria/clip_loss.py

{
  "path": "criteria/clip_loss.py",
  "blob_sha": "4e86031d7efa6f41becc10449e644578657b199d",
  "sha256": "0c14f25c639eacc266f395c4dc04cdee5045473536eda9ae4b07ab23825de813",
  "lines": 75,
  "bytes": 3663,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 18
  },
  "url": "https://github.com/shivangi-aneja/ClipFace/blob/95b977a6e413ca9c59f0b2f8d93b591c30cc1937/criteria/clip_loss.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, config):"
    },
    {
      "line": 24,
      "text": "def set_device(self, device):"
    },
    {
      "line": 27,
      "text": "def get_text_features(self, text_prompt: str, norm: bool = True) -> torch.Tensor:"
    },
    {
      "line": 34,
      "text": "def get_image_features(self, img: torch.Tensor, norm: bool = True) -> torch.Tensor:"
    },
    {
      "line": 41,
      "text": "def compute_text_direction(self, source_text: str, target_text: str) -> torch.Tensor:"
    },
    {
      "line": 48,
      "text": "def compute_img2img_direction(self, source_img: torch.Tensor, target_imgs: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 55,
      "text": "def forward(self, altered_images, init_images=None, weight=1):"
    }
  ]
}

```text

import torch
import clip
from torchvision.transforms import transforms


class ClipLoss(torch.nn.Module):

    def __init__(self, config):
        super(ClipLoss, self).__init__()
        self.base_prompt = config.base_prompt
        self.altered_prompt = config.altered_prompt
        self.loss_type = config.clip_loss_type
        self.clip_model, _ = clip.load('ViT-B/32', jit=False)
        self.directional = config.clip_directional
        self.loss_func = {
            'mse':    torch.nn.MSELoss(),
            'cosine': torch.nn.CosineSimilarity(dim=1, eps=1e-6),
            'mae':    torch.nn.L1Loss()
        }[self.loss_type]
        clip_normalizer = transforms.Normalize((0.48145466, 0.4578275, 0.40821073),
                                               (0.26862954, 0.26130258, 0.27577711))
        self.clip_transform = transforms.Compose([transforms.Resize((224, 224)), clip_normalizer])

    def set_device(self, device):
        self.device = device

    def get_text_features(self, text_prompt: str, norm: bool = True) -> torch.Tensor:
        tokens = clip.tokenize([text_prompt]).to(self.device)
        text_features = self.clip_model.encode_text(tokens).detach()
        if norm:
            text_features /= text_features.norm(dim=-1, keepdim=True)
        return text_features

    def get_image_features(self, img: torch.Tensor, norm: bool = True) -> torch.Tensor:
        clip_image = self.clip_transform(torch.clamp(img * 0.5 + 0.5, 0, 1))  # Clip model expects image to be normalized to (0, 1)
        image_features = self.clip_model.encode_image(clip_image)
        if norm:
            image_features /= image_features.clone().norm(dim=-1, keepdim=True)
        return image_features

    def compute_text_direction(self, source_text: str, target_text: str) -> torch.Tensor:
        source_features = self.get_text_features(source_text)
        target_features = self.get_text_features(target_text)
        text_direction = (target_features - source_features).mean(axis=0, keepdim=True)
        text_direction /= text_direction.norm(dim=-1, keepdim=True)
        return text_direction

    def compute_img2img_direction(self, source_img: torch.Tensor, target_imgs: torch.Tensor) -> torch.Tensor:
        src_encoding = self.get_image_features(source_img)
        target_encoding = self.get_image_features(target_imgs)
        img_direction = (target_encoding - src_encoding)
        img_direction /= (img_direction.clone().norm(dim=-1, keepdim=True) + 1e-6)
        return img_direction

    def forward(self, altered_images, init_images=None, weight=1):
        if self.directional:
            # Use directional clip loss
            img_direction = self.compute_img2img_direction(init_images, altered_images)
            text_direction = weight*self.compute_text_direction(self.base_prompt, self.altered_prompt)
            # Check for loss type
            if self.loss_type == "cosine":
                loss = 1. - self.loss_func(img_direction.double(), text_direction)
            else:
                loss = self.loss_func(img_direction, text_direction)
            return loss.mean()
        else:
            # Undirectional Clip loss
            prompt_token = clip.tokenize([self.altered_prompt]).to(self.device)
            encoded_text = self.clip_model.encode_text(prompt_token)
            clip_images = self.clip_transform(torch.clamp(altered_images * 0.5 + 0.5, 0, 1))
            encoded_renders = self.clip_model.encode_image(clip_images)

            # Check for loss type
            if self.loss_type == "cosine":
                return - torch.mean(torch.cosine_similarity(encoded_renders, encoded_text))

```

## Source file: dataset/face_image.py

{
  "path": "dataset/face_image.py",
  "blob_sha": "3462d6421c1e01654775aed43ae5a88d6dc72cd3",
  "sha256": "cd21c97d7c4b6ec23657e780a68e103fda48d5fb8df67342d76ce324ce695bce",
  "lines": 66,
  "bytes": 2405,
  "selection_score": 15,
  "keyword_line_counts": {
    "flame": 1,
    "face": 4
  },
  "url": "https://github.com/shivangi-aneja/ClipFace/blob/95b977a6e413ca9c59f0b2f8d93b591c30cc1937/dataset/face_image.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, config):"
    },
    {
      "line": 31,
      "text": "def set_renderer(self, renderer):"
    },
    {
      "line": 34,
      "text": "def __len__(self):"
    },
    {
      "line": 37,
      "text": "def set_device(self, device):"
    },
    {
      "line": 44,
      "text": "def __getitem__(self, index):"
    }
  ]
}

```text

import cv2
import torch
import numpy as np
from glob import glob
from util.misc import load_mesh
from torch.utils.data import DataLoader
from util.misc import get_orthographic_view


class FaceImage(torch.utils.data.Dataset):
    def __init__(self, config):
        self.config = config
        self.image_size = config.image_size
        self.deca_size = config.deca_size
        self.scale = 1.25

        mesh_path = config.flame_obj
        vertices, faces, uvs, uv_indices = load_mesh(mesh_path)
        vertex_bounds = (vertices.min(axis=0), vertices.max(axis=0))
        self.vertices = vertices - (vertex_bounds[0] + vertex_bounds[1]) / 2
        self.vertices = self.vertices / (vertex_bounds[1] - vertex_bounds[0]).max()
        self.vertices = torch.from_numpy(self.vertices).float()
        self.faces = torch.from_numpy(faces).int()
        self.uvs = torch.from_numpy(uvs).float()
        self.uvs = torch.cat([self.uvs[:, 0:1], 1 - self.uvs[:, 1:2]], dim=1)
        self.uv_indices = torch.from_numpy(uv_indices).int()

        self.image_paths = np.array(sorted(glob(config.ffhq_dir_512 + '/*.png')))
        self.vertices_paths = np.array(sorted(glob(config.ffhq_verts_dir + '/*.pt')))

    def set_renderer(self, renderer):
        self.renderer = renderer

    def __len__(self):
        return len(self.image_paths)

    def set_device(self, device):
        self.device = device
        self.vertices = self.vertices.to(device)
        self.uvs = self.uvs.to(device)
        self.faces = self.faces.to(device)
        self.uv_indices = self.uv_indices.to(device)

    def __getitem__(self, index):

        projection_matrix, view_matrix = get_orthographic_view()

        # # Image Paths
        image_path = self.image_paths[index]
        vertex_path = self.vertices_paths[index]

        # Corresponding Real Image for StyleGAN
        image_cv = cv2.cvtColor(cv2.imread(image_path), cv2.COLOR_BGR2RGB)
        image_np = cv2.resize(image_cv, (self.image_size, self.image_size), cv2.INTER_AREA)
        image = torch.from_numpy(image_np).float() / 127.5 - 1

        # Load the vertex
        vertex = torch.load(vertex_path, map_location='cpu')

        return {
            "name": f"{index:06d}",
            "image": image.permute(2, 0, 1),
            "projection_matrix": projection_matrix,
            "view_matrix": view_matrix,
            "vertices":  vertex,
        }


```

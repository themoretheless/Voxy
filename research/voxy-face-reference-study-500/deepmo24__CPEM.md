# Facial rendering source scan: deepmo24/CPEM

Repository: https://github.com/deepmo24/CPEM

Pinned source revision: cdaa86a9e1fb7ccf975eb6ad1cb4b910bbb29405

Discovery topic: facial retargeting

Description: PyTorch implementation of "Towards Accurate Facial Motion Retargeting with Identity-Consistent and Expression-Exclusive Constraints" (AAAI2022)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["assets/blendshape_name.md"], "face": ["FaceBoxes/.gitignore", "FaceBoxes/FaceBoxes.py", "FaceBoxes/FaceBoxes_ONNX.py", "FaceBoxes/__init__.py", "FaceBoxes/build_cpu_nms.sh", "FaceBoxes/models/__init__.py", "FaceBoxes/models/faceboxes.py", "FaceBoxes/onnx.py", "FaceBoxes/readme.md", "FaceBoxes/utils/.gitignore", "FaceBoxes/utils/__init__.py", "FaceBoxes/utils/box_utils.py", "FaceBoxes/utils/build.py", "FaceBoxes/utils/config.py", "FaceBoxes/utils/functions.py", "FaceBoxes/utils/nms/.gitignore", "FaceBoxes/utils/nms/__init__.py", "FaceBoxes/utils/nms/cpu_nms.cp38-win_amd64.pyd", "FaceBoxes/utils/nms/cpu_nms.pyx", "FaceBoxes/utils/nms/py_cpu_nms.py", "FaceBoxes/utils/nms_wrapper.py", "FaceBoxes/utils/prior_box.py", "FaceBoxes/utils/timer.py", "FaceBoxes/weights/.gitignore", "FaceBoxes/weights/FaceBoxesProd.pth", "FaceBoxes/weights/readme.md", "core/face_decoder.py", "data/demo_dataset/300w_lp/face_mask/AFW_1130084326_2/001/AFW_1130084326_2_0.jpg", "data/demo_dataset/300w_lp/face_mask/AFW_1130084326_2/001/AFW_1130084326_2_1.jpg", "data/demo_dataset/300w_lp/face_mask/AFW_1130084326_2/001/AFW_1130084326_2_10.jpg"]}

## Source file: core/face_decoder.py

{
  "path": "core/face_decoder.py",
  "blob_sha": "8a8c200fe18a0d05e5ce97b67cad4e405c5a73b6",
  "sha256": "42a31258efb0f98fb6fc20ab06992b6dfa9cf67221b79a7442966d1d2ea475a1",
  "lines": 473,
  "bytes": 17791,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 8,
    "shader": 2,
    "rig": 4,
    "face": 112
  },
  "url": "https://github.com/deepmo24/CPEM/blob/cdaa86a9e1fb7ccf975eb6ad1cb4b910bbb29405/core/face_decoder.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def __init__(self, model_path, delta_bs_path, device=torch.device('cuda')):"
    },
    {
      "line": 59,
      "text": "def get_neutral_face(self, id_coeff):"
    },
    {
      "line": 67,
      "text": "def get_blendshapes(self, id_coeff, add_neutral_face=False):"
    },
    {
      "line": 84,
      "text": "def to_device(self, device):"
    },
    {
      "line": 97,
      "text": "def __init__(self, model_path, delta_bs_path, device, batch_size,"
    },
    {
      "line": 115,
      "text": "def build_constant(self):"
    },
    {
      "line": 153,
      "text": "def get_renderer(self, device):"
    },
    {
      "line": 183,
      "text": "def Split_coeff(self, coeff):"
    },
    {
      "line": 202,
      "text": "def Shape_formation(self, id_coeff, bs_coeff):"
    },
    {
      "line": 222,
      "text": "def Texture_formation(self, tex_coeff):"
    },
    {
      "line": 238,
      "text": "def Compute_norm(self, face_shape):"
    },
    {
      "line": 260,
      "text": "def Projection_block(self, face_shape):"
    },
    {
      "line": 283,
      "text": "def Projection_block_3d(self, face_shape):"
    },
    {
      "line": 315,
      "text": "def Illumination_layer(self, face_texture, norm, gamma):"
    },
    {
      "line": 352,
      "text": "def Compute_rotation_matrix(self, angles):"
    },
    {
      "line": 383,
      "text": "def Rigid_transform_block(self, face_shape, rotation, translation):"
    },
    {
      "line": 390,
      "text": "def get_landmarks(self, face_shape, kp_inds):"
    },
    {
      "line": 395,
      "text": "def decode_face(self, coeff, return_coeffs=False, return_neutral_face=False):"
    },
    {
      "line": 451,
      "text": "def get_face_on_2d_plane(self, coeff):"
    }
  ]
}

```text

import numpy as np
from scipy.io import loadmat, savemat
import torch
from pytorch3d.structures import Meshes
from pytorch3d.renderer import (
    look_at_view_transform,
    FoVPerspectiveCameras,
    PointLights,
    RasterizationSettings,
    MeshRenderer,
    MeshRasterizer,
    SoftPhongShader,
    TexturesVertex,
    blending
)

class FaceModel():
    def __init__(self, model_path, delta_bs_path, device=torch.device('cuda')):
        model = loadmat(model_path)
        # mean face shape. [1, N*3]
        self.meanshape = torch.from_numpy(model['meanshape'])
        # identity basis. [N*3,80]
        self.idBase = torch.from_numpy(model['idBase'])
        # mean face texture. [1, N*3] (0-255)
        self.meantex = torch.from_numpy(model['meantex'])
        # texture basis. [N*3,80]
        self.texBase = torch.from_numpy(model['texBase'])
        # triangle indices for each vertex that lies in. starts from 1. [N,8]
        self.point_buf = torch.from_numpy(model['point_buf']).long()
        # vertex indices in each triangle. starts from 1. [F,3]
        self.tri = torch.from_numpy(model['tri']).long()
        # vertex indices of 68 facial landmarks. starts from 0. [68]
        self.kp_inds = torch.tensor(model['keypoints'] - 1).squeeze().long()
        # delta blendshapes. [N*3, 46]
        delta_bs = np.load(delta_bs_path)
        self.deltaB = torch.from_numpy(delta_bs).float() # to float32

        self.n_ver = int(self.idBase.size(0) / 3)
        self.n_id_para = self.idBase.size(1)
        self.n_bs_para = self.deltaB.size(1)
        self.n_tex_para = self.texBase.size(1)
        # predefine
        self.n_rot_para = 3
        self.n_light_para = 27
        self.n_tran_para = 3

        # print('3DMM parameter')
        # print('n_id_para: ', self.n_id_para)
        # print('n_bs_para: ', self.n_bs_para)
        # print('n_tex_para: ', self.n_tex_para)
        # print('n_rot_para: ', self.n_rot_para)
        # print('n_light_para: ', self.n_light_para)
        # print('n_tran_para: ', self.n_tran_para)

        # init device
        self.device = device
        self.to_device(device)

    def get_neutral_face(self, id_coeff):
        '''
        get neutral face shape
        '''
        # [bs, nverx3]
        neutral_face = self.meanshape + torch.einsum('ij,aj->ai', self.idBase, id_coeff)
        return neutral_face

    def get_blendshapes(self, id_coeff, add_neutral_face=False):
        neutral_face = self.get_neutral_face(id_coeff)

        blendshapes = []
        if add_neutral_face:
            blendshapes.append(neutral_face)

        for bs_idx in range(self.n_bs_para):
            bs_coeff = torch.zeros(1, self.n_bs_para)
            bs_coeff[0, bs_idx] = 1.0

            B = neutral_face + torch.einsum('ij,aj->ai', self.deltaB, bs_coeff)
            blendshapes.append(B)

        return blendshapes


    def to_device(self, device):
        self.meanshape = self.meanshape.to(device)
        self.idBase = self.idBase.to(device)
        self.meantex = self.meantex.to(device)
        self.texBase = self.texBase.to(device)
        self.deltaB = self.deltaB.to(device)
        self.point_buf = self.point_buf.to(device)
        self.tri = self.tri.to(device)
        self.kp_inds =self.kp_inds.to(device)



class FaceDecoder():
    def __init__(self, model_path, delta_bs_path, device, batch_size,
                 focal=1015, img_size=224):

        facemodel = FaceModel(model_path, delta_bs_path, device)
        self.facemodel = facemodel

        self.focal = focal
        self.img_size = img_size
        # self.img_size = 300
        self.n_b = batch_size

        self.device = device

        self.renderer = self.get_renderer(self.device)
        self.renderer.to(self.device)

        self.build_constant()

    def build_constant(self):

        # SH illumination
        a0 = np.pi
        a1 = 2 * np.pi / np.sqrt(3.0)
        a2 = 2 * np.pi / np.sqrt(8.0)
        c0 = 1 / np.sqrt(4 * np.pi)
        c1 = np.sqrt(3.0) / np.sqrt(4 * np.pi)
        c2 = 3 * np.sqrt(5.0) / np.sqrt(12 * np.pi)
        d0 = 0.5 / np.sqrt(3.0)

        self.a0 = torch.tensor(a0, dtype=torch.float32, device=self.device)
        self.a1 = torch.tensor(a1, dtype=torch.float32, device=self.device)
        self.a2 = torch.tensor(a2, dtype=torch.float32, device=self.device)
        self.c0 = torch.tensor(c0, dtype=torch.float32, device=self.device)
        self.c1 = torch.tensor(c1, dtype=torch.float32, device=self.device)
        self.c2 = torch.tensor(c2, dtype=torch.float32, device=self.device)
        self.d0 = torch.tensor(d0, dtype=torch.float32, device=self.device)

        num_vertex = self.facemodel.n_ver
        n_v_full = self.n_b * num_vertex

        Y0 = torch.ones(n_v_full).to(self.device).float() * self.a0 * self.c0
        self.Y0 = Y0

        # Projection
        half_image_width = self.img_size // 2

        camera_pos = torch.tensor([0.0, 0.0, 10.0], device=self.device).reshape(1, 1, 3)

        p_matrix = torch.tensor([self.focal, 0.0, half_image_width,
                             0.0, self.focal, half_image_width,
                             0.0, 0.0, 1.0], dtype=torch.float32, device=self.device)

        self.camera_pos = camera_pos
        self.p_matrix = p_matrix


    def get_renderer(self, device):
        R, T = look_at_view_transform(10, 0, 0)
        cameras = FoVPerspectiveCameras(device=device, R=R, T=T, znear=0.01, zfar=50,
                                        fov=2 * np.arctan(self.img_size // 2 / self.focal) * 180. / np.pi)

        lights = PointLights(device=device, location=[[0.0, 0.0, 1e5]], ambient_color=[[1, 1, 1]],
                             specular_color=[[0., 0., 0.]], diffuse_color=[[0., 0., 0.]])

        raster_settings = RasterizationSettings(
            image_size=self.img_size,
            blur_radius=0.0,
            faces_per_pixel=1,
        )
        blend_params = blending.BlendParams(background_color=[0, 0, 0])

        renderer = MeshRenderer(
            rasterizer=MeshRasterizer(
                cameras=cameras,
                raster_settings=raster_settings
            ),
            shader=SoftPhongShader(
                device=device,
                cameras=cameras,
                lights=lights,
                blend_params=blend_params
            )
        )
        return renderer


    def Split_coeff(self, coeff):
        to_id = self.facemodel.n_id_para
        to_bs = to_id + self.facemodel.n_bs_para
        to_tex = to_bs + self.facemodel.n_tex_para
        to_angle = to_tex + self.facemodel.n_rot_para
        to_gamma = to_angle + self.facemodel.n_light_para
        to_trans = to_gamma + self.facemodel.n_tran_para

        id_coeff = coeff[:, :to_id]
        bs_coeff = coeff[:, to_id:to_bs]  # blendshape coeff
        tex_coeff = coeff[:, to_bs:to_tex]  # texture(albedo) coeff
        angles = coeff[:, to_tex:to_angle]  # ruler angles(x,y,z) for rotation
        gamma = coeff[:, to_angle:to_gamma]  # lighting coeff for 3 channel SH function
        translation = coeff[:, to_gamma:to_trans]  # translation coeff


        return id_coeff, bs_coeff, tex_coeff, angles, gamma, translation


    def Shape_formation(self, id_coeff, bs_coeff):
        # compute face shape with blendshape coeff, based on BFM model
        # input: bs_coeff with shape [bs,46], bs is batch size
        #        id_coeff with shape [bs,80]
        # output: face_shape with shape [bs,N,3], N is number of vertices

        '''
            S = neutral_shape + \alpha * deltaB
        '''
        n_b = bs_coeff.size(0)

        B0 = self.facemodel.get_neutral_face(id_coeff) # [bs, nverx3]

        # [bs, nverx3]
        face_shape = B0 + torch.einsum('ij,aj->ai', self.facemodel.deltaB, bs_coeff)
        face_shape = face_shape.view(n_b, -1, 3)

        return face_shape


    def Texture_formation(self, tex_coeff):
        # compute vertex texture(albedo) with tex_coeff
        # input: tex_coeff with shape [bs,N,3]
        # output: face_texture with shape [bs,N,3], RGB order, range from 0-255

        '''
            T = mean_texture + \lambda * B_texture
        '''
        n_b = tex_coeff.size(0)
        # [bs, nverx3]
        face_texture = self.facemodel.meantex + torch.einsum('ij,aj->ai', self.facemodel.texBase, tex_coeff)
        face_texture = face_texture.view(n_b, -1, 3)

        return face_texture


    def Compute_norm(self, face_shape):
        # compute vertex normal using one-ring neighborhood (8 points)
        # input: face_shape with shape [bs,N,3]
        # output: v_norm with shape [bs,N,3]

        face_id = self.facemodel.tri - 1
        point_id = self.facemodel.point_buf - 1
        shape = face_shape
        v1 = shape[:, face_id[:, 0], :]
        v2 = shape[:, face_id[:, 1], :]
        v3 = shape[:, face_id[:, 2], :]
        e1 = v1 - v2
        e2 = v2 - v3
        face_norm = e1.cross(e2)
        empty = torch.zeros((face_norm.size(0), 1, 3), dtype=face_norm.dtype, device=face_norm.device)
        face_norm = torch.cat((face_norm, empty), 1)
        v_norm = face_norm[:, point_id, :].sum(2)
        v_norm = v_norm / v_norm.norm(dim=2).unsqueeze(2)

        return v_norm


    def Projection_block(self, face_shape):
        # we choose the focal length and camera position empirically
        # project 3D face onto image plane
        # input: face_shape with shape [bs,N,3]
        # output: face_projection with shape [bs,N,2]
        batchsize = face_shape.shape[0]

        camera_pos = self.camera_pos

        p_matrix = self.p_matrix.view(1,3,3).repeat(batchsize, 1, 1)

        reverse_z = torch.tensor([1.0, 0, 0, 0, 1, 0, 0, 0, -1.0], dtype=torch.float32, device=face_shape.device)
        reverse_z = reverse_z.view(1, 3, 3).repeat(batchsize, 1, 1)


        face_shape = torch.matmul(face_shape, reverse_z) + camera_pos
        aug_projection = torch.matmul(face_shape, p_matrix.permute((0, 2, 1)))

        face_projection = aug_projection[:, :, :2] / \
                          torch.reshape(aug_projection[:, :, 2], [batchsize, -1, 1])
        return face_projection


    def Projection_block_3d(self, face_shape):
        # we choose the focal length and camera position empirically
        # project 3D face onto image plane
        # input: face_shape with shape [bs,N,3]
        # output: face_projection with shape [bs,N,2]
        batchsize = face_shape.shape[0]

        camera_pos = self.camera_pos

        p_matrix = self.p_matrix.view(1,3,3).repeat(batchsize, 1, 1)

        reverse_z = torch.tensor([1.0, 0, 0, 0, 1, 0, 0, 0, -1.0], dtype=torch.float32, device=face_shape.device)
        reverse_z = reverse_z.view(1, 3, 3).repeat(batchsize, 1, 1)


        face_shape = torch.matmul(face_shape, reverse_z) + camera_pos
        aug_projection = torch.matmul(face_shape, p_matrix.permute((0, 2, 1)))

        # face_projection = aug_projection[:, :, :2] / \
        #                   torch.reshape(aug_projection[:, :, 2], [batchsize, -1, 1])

        aug_projection[:, :, :2] = aug_projection[:, :, :2] / \
                          torch.reshape(aug_projection[:, :, 2], [batchsize, -1, 1])

        face_projection = aug_projection

        face_projection[:, :, 2] =  face_projection[:, :, 2] - face_projection[:, :, 2].min()
        z_scale = 160 / face_projection[:, :, 2].max()
        face_projection[:, :, 2] = face_projection[:, :, 2] * z_scale

        return face_projection

    def Illumination_layer(self, face_texture, norm, gamma):
        # compute vertex color using face_texture and SH function lighting approximation
        # input: face_texture with shape [1,N,3]
        #          norm with shape [1,N,3]
        #         gamma with shape [1,27]
        # output: face_color with shape [1,N,3], RGB order, range from 0-255

        n_b, num_vertex, _ = face_texture.size()
        n_v_full = n_b * num_vertex
        gamma = gamma.view(-1, 3, 9).clone()
        gamma[:, :, 0] += 0.8

        gamma = gamma.permute(0, 2, 1)

        # Y0 = torch.ones(n_v_full).to(gamma.device).float() * self.a0 * self.c0
        norm = norm.view(-1, 3)
        nx, ny, nz = norm[:, 0], norm[:, 1], norm[:, 2]
        arrH = []

        arrH.append(self.Y0)
        arrH.append(-self.a1 * self.c1 * ny)
        arrH.append(self.a1 * self.c1 * nz)
        arrH.append(-self.a1 * self.c1 * nx)
        arrH.append(self.a2 * self.c2 * nx * ny)
        arrH.append(-self.a2 * self.c2 * ny * nz)
        arrH.append(self.a2 * self.c2 * self.d0 * (3 * nz.pow(2) - 1))
        arrH.append(-self.a2 * self.c2 * nx * nz)
        arrH.append(self.a2 * self.c2 * 0.5 * (nx.pow(2) - ny.pow(2)))

        H = torch.stack(arrH, 1)
        Y = H.view(n_b, num_vertex, 9)
        lighting = Y.bmm(gamma)

        face_color = face_texture * lighting
        return face_color


    def Compute_rotation_matrix(self, angles):
        n_b = angles.size(0)
        sinx = torch.sin(angles[:, 0])
        siny = torch.sin(angles[:, 1])
        sinz = torch.sin(angles[:, 2])
        cosx = torch.cos(angles[:, 0])
        cosy = torch.cos(angles[:, 1])
        cosz = torch.cos(angles[:, 2])

        rotXYZ = torch.eye(3).view(1, 3, 3).repeat(n_b * 3, 1, 1).view(3, n_b, 3, 3)

        if angles.is_cuda: rotXYZ = rotXYZ.cuda()

        rotXYZ[0, :, 1, 1] = cosx
        rotXYZ[0, :, 1, 2] = -sinx
        rotXYZ[0, :, 2, 1] = sinx
        rotXYZ[0, :, 2, 2] = cosx
        rotXYZ[1, :, 0, 0] = cosy
        rotXYZ[1, :, 0, 2] = siny
        rotXYZ[1, :, 2, 0] = -siny
        rotXYZ[1, :, 2, 2] = cosy
        rotXYZ[2, :, 0, 0] = cosz
        rotXYZ[2, :, 0, 1] = -sinz
        rotXYZ[2, :, 1, 0] = sinz
        rotXYZ[2, :, 1, 1] = cosz

        rotation = rotXYZ[2].bmm(rotXYZ[1]).bmm(rotXYZ[0])

        return rotation.permute(0, 2, 1)


    def Rigid_transform_block(self, face_shape, rotation, translation):
        face_shape_r = torch.matmul(face_shape, rotation)
        face_shape_t = face_shape_r + translation.view(-1, 1, 3)

        return face_shape_t


    def get_landmarks(self, face_shape, kp_inds):
        lms = face_shape[:, kp_inds, :]
        return lms


    def decode_face(self, coeff, return_coeffs=False, return_neutral_face=False):


        id_coeff, bs_coeff, tex_coeff, angles, gamma, translation = self.Split_coeff(coeff)


        batch_num = bs_coeff.shape[0]

        face_shape = self.Shape_formation(id_coeff, bs_coeff)
        face_texture = self.Texture_formation(tex_coeff)
        face_norm = self.Compute_norm(face_shape)
        rotation = self.Compute_rotation_matrix(angles)
        face_norm_r = face_norm.bmm(rotation)
        face_shape_t = self.Rigid_transform_block(face_shape, rotation, translation)
        face_color = self.Illumination_layer(face_texture, face_norm_r, gamma)
        landmarks_3d = self.get_landmarks(face_shape_t, self.facemodel.kp_inds)
        landmarks_2d = self.Projection_block(landmarks_3d)
        landmarks_2d = torch.stack([landmarks_2d[:, :, 0], self.img_size - 1.0 - landmarks_2d[:, :, 1]], dim=2)

        if return_neutral_face:
            face_shape_neutral = self.facemodel.get_neutral_face(id_coeff)
            face_shape_neutral = face_shape_neutral.view(1, -1, 3)
            face_shape_neutral_t = self.Rigid_transform_block(face_shape_neutral, rotation, translation)
            landmarks_3d_neutral = self.get_landmarks(face_shape_neutral_t, self.facemodel.kp_inds)
            landmarks_2d_neutral = self.Projection_block(landmarks_3d_neutral)
            landmarks_2d_neutral = torch.stack([landmarks_2d_neutral[:, :, 0], self.img_size - 1.0 - landmarks_2d_neutral[:, :, 1]], dim=2)

        tri = self.facemodel.tri - 1
        face_color_pytorch3d = TexturesVertex(face_color)
        mesh = Meshes(face_shape_t, tri.repeat(batch_num, 1, 1), face_color_pytorch3d)
        rendered_img = self.renderer(mesh)
        rendered_img = torch.clamp(rendered_img, 0, 255)


        if return_coeffs:
            coeffs = {
                'coeff_id': id_coeff,
                'coeff_bs': bs_coeff,
                'coeff_angle': angles,
                'coeff_translation': translation,
                'coeff_tex': tex_coeff,
                'coeff_light': gamma
            }

            mesh_tuple = (face_shape_t, tri.repeat(batch_num, 1, 1), torch.clamp(face_color, 0, 255))

            if not return_neutral_face:
                return rendered_img, landmarks_2d, coeffs, mesh_tuple
            else:
                neutral_tuple = (face_shape_neutral, face_shape_neutral_t, landmarks_2d_neutral)
                return rendered_img, landmarks_2d, coeffs, mesh_tuple, neutral_tuple

        else:
            return rendered_img, landmarks_2d, bs_coeff, id_coeff, tex_coeff


    def get_face_on_2d_plane(self, coeff):

        id_coeff, bs_coeff, tex_coeff, angles, gamma, translation = self.Split_coeff(coeff)

        batch_num = bs_coeff.shape[0]

        face_shape = self.Shape_formation(id_coeff, bs_coeff)
        face_texture = self.Texture_formation(tex_coeff)
        face_norm = self.Compute_norm(face_shape)
        rotation = self.Compute_rotation_matrix(angles)
        face_norm_r = face_norm.bmm(rotation)
        face_shape_t = self.Rigid_transform_block(face_shape, rotation, translation)
        # face_color = self.Illumination_layer(face_texture, face_norm_r, gamma)
        # landmarks_3d = self.get_landmarks(face_shape_t, self.facemodel.kp_inds)
        # landmarks_2d = self.Projection_block(landmarks_3d)
        # landmarks_2d = torch.stack([landmarks_2d[:, :, 0], self.img_size - 1.0 - landmarks_2d[:, :, 1]], dim=2)

        face_shape_2d = self.Projection_block_3d(face_shape_t)

        face_shape_2d[:, :, 1] = self.img_size - 1.0 - face_shape_2d[:, :, 1]

        return face_shape_2d



```

## Source file: FaceBoxes/FaceBoxes.py

{
  "path": "FaceBoxes/FaceBoxes.py",
  "blob_sha": "785455843a017907f5244a7cad74cdef923f9365",
  "sha256": "c7b5df2d328a84fba909380de16b294d6c0267e6083e2b45e283874f7062378b",
  "lines": 162,
  "bytes": 4748,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 7
  },
  "url": "https://github.com/deepmo24/CPEM/blob/cdaa86a9e1fb7ccf975eb6ad1cb4b910bbb29405/FaceBoxes/FaceBoxes.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def viz_bbox(img, dets, wfp='out.jpg'):"
    },
    {
      "line": 48,
      "text": "def __init__(self, timer_flag=False):"
    },
    {
      "line": 58,
      "text": "def __call__(self, img_):"
    },
    {
      "line": 141,
      "text": "def main():"
    },
    {
      "line": 144,
      "text": "fn = 'trump_hillary.jpg'"
    }
  ]
}

```text

# coding: utf-8

import os.path as osp

import torch
import numpy as np
import cv2

from .utils.prior_box import PriorBox
from .utils.nms_wrapper import nms
from .utils.box_utils import decode
from .utils.timer import Timer
from .utils.functions import check_keys, remove_prefix, load_model
from .utils.config import cfg
from .models.faceboxes import FaceBoxesNet

# some global configs
confidence_threshold = 0.05
top_k = 5000
keep_top_k = 750
nms_threshold = 0.3
vis_thres = 0.5
resize = 1

scale_flag = True
HEIGHT, WIDTH = 720, 1080

make_abs_path = lambda fn: osp.join(osp.dirname(osp.realpath(__file__)), fn)
pretrained_path = make_abs_path('weights/FaceBoxesProd.pth')


def viz_bbox(img, dets, wfp='out.jpg'):
    # show
    for b in dets:
        if b[4] < vis_thres:
            continue
        text = "{:.4f}".format(b[4])
        b = list(map(int, b))
        cv2.rectangle(img, (b[0], b[1]), (b[2], b[3]), (0, 0, 255), 2)
        cx = b[0]
        cy = b[1] + 12
        cv2.putText(img, text, (cx, cy), cv2.FONT_HERSHEY_DUPLEX, 0.5, (255, 255, 255))
    cv2.imwrite(wfp, img)
    print(f'Viz bbox to {wfp}')


class FaceBoxes:
    def __init__(self, timer_flag=False):
        torch.set_grad_enabled(False)

        net = FaceBoxesNet(phase='test', size=None, num_classes=2)  # initialize detector
        self.net = load_model(net, pretrained_path=pretrained_path, load_to_cpu=True)
        self.net.eval()
        # print('Finished loading model!')

        self.timer_flag = timer_flag

    def __call__(self, img_):
        img_raw = img_.copy()

        # scaling to speed up
        scale = 1
        if scale_flag:
            h, w = img_raw.shape[:2]
            if h > HEIGHT:
                scale = HEIGHT / h
            if w * scale > WIDTH:
                scale *= WIDTH / (w * scale)
            # print(scale)
            if scale == 1:
                img_raw_scale = img_raw
            else:
                h_s = int(scale * h)
                w_s = int(scale * w)
                # print(h_s, w_s)
                img_raw_scale = cv2.resize(img_raw, dsize=(w_s, h_s))
                # print(img_raw_scale.shape)

            img = np.float32(img_raw_scale)
        else:
            img = np.float32(img_raw)

        # forward
        _t = {'forward_pass': Timer(), 'misc': Timer()}
        im_height, im_width, _ = img.shape
        scale_bbox = torch.Tensor([img.shape[1], img.shape[0], img.shape[1], img.shape[0]])
        img -= (104, 117, 123)
        img = img.transpose(2, 0, 1)
        img = torch.from_numpy(img).unsqueeze(0)

        _t['forward_pass'].tic()
        loc, conf = self.net(img)  # forward pass
        _t['forward_pass'].toc()
        _t['misc'].tic()
        priorbox = PriorBox(image_size=(im_height, im_width))
        priors = priorbox.forward()
        prior_data = priors.data
        boxes = decode(loc.data.squeeze(0), prior_data, cfg['variance'])
        if scale_flag:
            boxes = boxes * scale_bbox / scale / resize
        else:
            boxes = boxes * scale_bbox / resize

        boxes = boxes.cpu().numpy()
        scores = conf.squeeze(0).data.cpu().numpy()[:, 1]

        # ignore low scores
        inds = np.where(scores > confidence_threshold)[0]
        boxes = boxes[inds]
        scores = scores[inds]

        # keep top-K before NMS
        order = scores.argsort()[::-1][:top_k]
        boxes = boxes[order]
        scores = scores[order]

        # do NMS
        dets = np.hstack((boxes, scores[:, np.newaxis])).astype(np.float32, copy=False)
        keep = nms(dets, nms_threshold)
        dets = dets[keep, :]

        # keep top-K faster NMS
        dets = dets[:keep_top_k, :]
        _t['misc'].toc()

        if self.timer_flag:
            print('Detection: {:d}/{:d} forward_pass_time: {:.4f}s misc: {:.4f}s'.format(1, 1, _t[
                'forward_pass'].average_time, _t['misc'].average_time))

        # filter using vis_thres
        det_bboxes = []
        for b in dets:
            if b[4] > vis_thres:
                xmin, ymin, xmax, ymax, score = b[0], b[1], b[2], b[3], b[4]
                bbox = [xmin, ymin, xmax, ymax, score]
                det_bboxes.append(bbox)

        return det_bboxes


def main():
    face_boxes = FaceBoxes(timer_flag=True)

    fn = 'trump_hillary.jpg'
    img_fp = f'../examples/inputs/{fn}'
    img = cv2.imread(img_fp)
    print(f'input shape: {img.shape}')
    dets = face_boxes(img)  # xmin, ymin, w, h
    # print(dets)

    # repeating inference for `n` times
    n = 10
    for i in range(n):
        dets = face_boxes(img)

    wfn = fn.replace('.jpg', '_det.jpg')
    wfp = osp.join('../examples/results', wfn)
    viz_bbox(img, dets, wfp)


if __name__ == '__main__':
    main()


```

## Source file: FaceBoxes/FaceBoxes_ONNX.py

{
  "path": "FaceBoxes/FaceBoxes_ONNX.py",
  "blob_sha": "619f50953b843db07b9c9b9842a60cab5c285d6c",
  "sha256": "f5dc5808355a056fdb805c17ee6f07d74dc8e4b312b2e29a84f96bc91230a5da",
  "lines": 168,
  "bytes": 4817,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 5
  },
  "url": "https://github.com/deepmo24/CPEM/blob/cdaa86a9e1fb7ccf975eb6ad1cb4b910bbb29405/FaceBoxes/FaceBoxes_ONNX.py",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "def viz_bbox(img, dets, wfp='out.jpg'):"
    },
    {
      "line": 49,
      "text": "def __init__(self, timer_flag=False):"
    },
    {
      "line": 56,
      "text": "def __call__(self, img_):"
    },
    {
      "line": 147,
      "text": "def main():"
    },
    {
      "line": 150,
      "text": "fn = 'trump_hillary.jpg'"
    }
  ]
}

```text

# coding: utf-8

import os.path as osp

import torch
import numpy as np
import cv2

from .utils.prior_box import PriorBox
from .utils.nms_wrapper import nms
from .utils.box_utils import decode
from .utils.timer import Timer
from .utils.config import cfg
from .onnx import convert_to_onnx

import onnxruntime

# some global configs
confidence_threshold = 0.05
top_k = 5000
keep_top_k = 750
nms_threshold = 0.3
vis_thres = 0.5
resize = 1

scale_flag = True
HEIGHT, WIDTH = 720, 1080

make_abs_path = lambda fn: osp.join(osp.dirname(osp.realpath(__file__)), fn)
onnx_path = make_abs_path('weights/FaceBoxesProd.onnx')


def viz_bbox(img, dets, wfp='out.jpg'):
    # show
    for b in dets:
        if b[4] < vis_thres:
            continue
        text = "{:.4f}".format(b[4])
        b = list(map(int, b))
        cv2.rectangle(img, (b[0], b[1]), (b[2], b[3]), (0, 0, 255), 2)
        cx = b[0]
        cy = b[1] + 12
        cv2.putText(img, text, (cx, cy), cv2.FONT_HERSHEY_DUPLEX, 0.5, (255, 255, 255))
    cv2.imwrite(wfp, img)
    print(f'Viz bbox to {wfp}')


class FaceBoxes_ONNX(object):
    def __init__(self, timer_flag=False):
        if not osp.exists(onnx_path):
            convert_to_onnx(onnx_path)
        self.session = onnxruntime.InferenceSession(onnx_path, None)

        self.timer_flag = timer_flag

    def __call__(self, img_):
        img_raw = img_.copy()

        # scaling to speed up
        scale = 1
        if scale_flag:
            h, w = img_raw.shape[:2]
            if h > HEIGHT:
                scale = HEIGHT / h
            if w * scale > WIDTH:
                scale *= WIDTH / (w * scale)
            # print(scale)
            if scale == 1:
                img_raw_scale = img_raw
            else:
                h_s = int(scale * h)
                w_s = int(scale * w)
                # print(h_s, w_s)
                img_raw_scale = cv2.resize(img_raw, dsize=(w_s, h_s))
                # print(img_raw_scale.shape)

            img = np.float32(img_raw_scale)
        else:
            img = np.float32(img_raw)

        # forward
        _t = {'forward_pass': Timer(), 'misc': Timer()}
        im_height, im_width, _ = img.shape
        scale_bbox = torch.Tensor([img.shape[1], img.shape[0], img.shape[1], img.shape[0]])

        img -= (104, 117, 123)
        img = img.transpose(2, 0, 1)
        # img = torch.from_numpy(img).unsqueeze(0)
        img = img[np.newaxis, ...]

        _t['forward_pass'].tic()
        # loc, conf = self.net(img)  # forward pass
        out = self.session.run(None, {'input': img})
        loc, conf = out[0], out[1]
        # for compatibility, may need to optimize
        loc = torch.from_numpy(loc)
        _t['forward_pass'].toc()
        _t['misc'].tic()

        priorbox = PriorBox(image_size=(im_height, im_width))
        priors = priorbox.forward()
        prior_data = priors.data
        boxes = decode(loc.data.squeeze(0), prior_data, cfg['variance'])
        if scale_flag:
            boxes = boxes * scale_bbox / scale / resize
        else:
            boxes = boxes * scale_bbox / resize

        boxes = boxes.cpu().numpy()
        scores = conf[0][:, 1]
        # scores = conf.squeeze(0).data.cpu().numpy()[:, 1]

        # ignore low scores
        inds = np.where(scores > confidence_threshold)[0]
        boxes = boxes[inds]
        scores = scores[inds]

        # keep top-K before NMS
        order = scores.argsort()[::-1][:top_k]
        boxes = boxes[order]
        scores = scores[order]

        # do NMS
        dets = np.hstack((boxes, scores[:, np.newaxis])).astype(np.float32, copy=False)
        keep = nms(dets, nms_threshold)
        dets = dets[keep, :]

        # keep top-K faster NMS
        dets = dets[:keep_top_k, :]
        _t['misc'].toc()

        if self.timer_flag:
            print('Detection: {:d}/{:d} forward_pass_time: {:.4f}s misc: {:.4f}s'.format(1, 1, _t[
                'forward_pass'].average_time, _t['misc'].average_time))

        # filter using vis_thres
        det_bboxes = []
        for b in dets:
            if b[4] > vis_thres:
                xmin, ymin, xmax, ymax, score = b[0], b[1], b[2], b[3], b[4]
                bbox = [xmin, ymin, xmax, ymax, score]
                det_bboxes.append(bbox)

        return det_bboxes


def main():
    face_boxes = FaceBoxes_ONNX(timer_flag=True)

    fn = 'trump_hillary.jpg'
    img_fp = f'../examples/inputs/{fn}'
    img = cv2.imread(img_fp)
    print(f'input shape: {img.shape}')
    dets = face_boxes(img)  # xmin, ymin, w, h
    # print(dets)

    # repeating inference for `n` times
    n = 10
    for i in range(n):
        dets = face_boxes(img)

    wfn = fn.replace('.jpg', '_det.jpg')
    wfp = osp.join('../examples/results', wfn)
    viz_bbox(img, dets, wfp)


if __name__ == '__main__':
    main()


```

## Source file: FaceBoxes/models/faceboxes.py

{
  "path": "FaceBoxes/models/faceboxes.py",
  "blob_sha": "83ca7f0da1d0b3e5f04218d6b279e5e53b3a6505",
  "sha256": "844f5a8afe395e1e74277de332adf2321c8167cf9c3dbdaccdb4b8dbb44e315c",
  "lines": 150,
  "bytes": 5407,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/deepmo24/CPEM/blob/cdaa86a9e1fb7ccf975eb6ad1cb4b910bbb29405/FaceBoxes/models/faceboxes.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, in_channels, out_channels, **kwargs):"
    },
    {
      "line": 15,
      "text": "def forward(self, x):"
    },
    {
      "line": 22,
      "text": "def __init__(self):"
    },
    {
      "line": 32,
      "text": "def forward(self, x):"
    },
    {
      "line": 51,
      "text": "def __init__(self, in_channels, out_channels, **kwargs):"
    },
    {
      "line": 56,
      "text": "def forward(self, x):"
    },
    {
      "line": 66,
      "text": "def __init__(self, phase, size, num_classes):"
    },
    {
      "line": 102,
      "text": "def multibox(self, num_classes):"
    },
    {
      "line": 113,
      "text": "def forward(self, x):"
    }
  ]
}

```text

# coding: utf-8

import torch
import torch.nn as nn
import torch.nn.functional as F


class BasicConv2d(nn.Module):

    def __init__(self, in_channels, out_channels, **kwargs):
        super(BasicConv2d, self).__init__()
        self.conv = nn.Conv2d(in_channels, out_channels, bias=False, **kwargs)
        self.bn = nn.BatchNorm2d(out_channels, eps=1e-5)

    def forward(self, x):
        x = self.conv(x)
        x = self.bn(x)
        return F.relu(x, inplace=True)


class Inception(nn.Module):
    def __init__(self):
        super(Inception, self).__init__()
        self.branch1x1 = BasicConv2d(128, 32, kernel_size=1, padding=0)
        self.branch1x1_2 = BasicConv2d(128, 32, kernel_size=1, padding=0)
        self.branch3x3_reduce = BasicConv2d(128, 24, kernel_size=1, padding=0)
        self.branch3x3 = BasicConv2d(24, 32, kernel_size=3, padding=1)
        self.branch3x3_reduce_2 = BasicConv2d(128, 24, kernel_size=1, padding=0)
        self.branch3x3_2 = BasicConv2d(24, 32, kernel_size=3, padding=1)
        self.branch3x3_3 = BasicConv2d(32, 32, kernel_size=3, padding=1)

    def forward(self, x):
        branch1x1 = self.branch1x1(x)

        branch1x1_pool = F.avg_pool2d(x, kernel_size=3, stride=1, padding=1)
        branch1x1_2 = self.branch1x1_2(branch1x1_pool)

        branch3x3_reduce = self.branch3x3_reduce(x)
        branch3x3 = self.branch3x3(branch3x3_reduce)

        branch3x3_reduce_2 = self.branch3x3_reduce_2(x)
        branch3x3_2 = self.branch3x3_2(branch3x3_reduce_2)
        branch3x3_3 = self.branch3x3_3(branch3x3_2)

        outputs = [branch1x1, branch1x1_2, branch3x3, branch3x3_3]
        return torch.cat(outputs, 1)


class CRelu(nn.Module):

    def __init__(self, in_channels, out_channels, **kwargs):
        super(CRelu, self).__init__()
        self.conv = nn.Conv2d(in_channels, out_channels, bias=False, **kwargs)
        self.bn = nn.BatchNorm2d(out_channels, eps=1e-5)

    def forward(self, x):
        x = self.conv(x)
        x = self.bn(x)
        x = torch.cat([x, -x], 1)
        x = F.relu(x, inplace=True)
        return x


class FaceBoxesNet(nn.Module):

    def __init__(self, phase, size, num_classes):
        super(FaceBoxesNet, self).__init__()
        self.phase = phase
        self.num_classes = num_classes
        self.size = size

        self.conv1 = CRelu(3, 24, kernel_size=7, stride=4, padding=3)
        self.conv2 = CRelu(48, 64, kernel_size=5, stride=2, padding=2)

        self.inception1 = Inception()
        self.inception2 = Inception()
        self.inception3 = Inception()

        self.conv3_1 = BasicConv2d(128, 128, kernel_size=1, stride=1, padding=0)
        self.conv3_2 = BasicConv2d(128, 256, kernel_size=3, stride=2, padding=1)

        self.conv4_1 = BasicConv2d(256, 128, kernel_size=1, stride=1, padding=0)
        self.conv4_2 = BasicConv2d(128, 256, kernel_size=3, stride=2, padding=1)

        self.loc, self.conf = self.multibox(self.num_classes)

        if self.phase == 'test':
            self.softmax = nn.Softmax(dim=-1)

        if self.phase == 'train':
            for m in self.modules():
                if isinstance(m, nn.Conv2d):
                    if m.bias is not None:
                        nn.init.xavier_normal_(m.weight.data)
                        m.bias.data.fill_(0.02)
                    else:
                        m.weight.data.normal_(0, 0.01)
                elif isinstance(m, nn.BatchNorm2d):
                    m.weight.data.fill_(1)
                    m.bias.data.zero_()

    def multibox(self, num_classes):
        loc_layers = []
        conf_layers = []
        loc_layers += [nn.Conv2d(128, 21 * 4, kernel_size=3, padding=1)]
        conf_layers += [nn.Conv2d(128, 21 * num_classes, kernel_size=3, padding=1)]
        loc_layers += [nn.Conv2d(256, 1 * 4, kernel_size=3, padding=1)]
        conf_layers += [nn.Conv2d(256, 1 * num_classes, kernel_size=3, padding=1)]
        loc_layers += [nn.Conv2d(256, 1 * 4, kernel_size=3, padding=1)]
        conf_layers += [nn.Conv2d(256, 1 * num_classes, kernel_size=3, padding=1)]
        return nn.Sequential(*loc_layers), nn.Sequential(*conf_layers)

    def forward(self, x):

        detection_sources = list()
        loc = list()
        conf = list()

        x = self.conv1(x)
        x = F.max_pool2d(x, kernel_size=3, stride=2, padding=1)
        x = self.conv2(x)
        x = F.max_pool2d(x, kernel_size=3, stride=2, padding=1)
        x = self.inception1(x)
        x = self.inception2(x)
        x = self.inception3(x)
        detection_sources.append(x)

        x = self.conv3_1(x)
        x = self.conv3_2(x)
        detection_sources.append(x)

        x = self.conv4_1(x)
        x = self.conv4_2(x)
        detection_sources.append(x)

        for (x, l, c) in zip(detection_sources, self.loc, self.conf):
            loc.append(l(x).permute(0, 2, 3, 1).contiguous())
            conf.append(c(x).permute(0, 2, 3, 1).contiguous())

        loc = torch.cat([o.view(o.size(0), -1) for o in loc], 1)
        conf = torch.cat([o.view(o.size(0), -1) for o in conf], 1)

        if self.phase == "test":
            output = (loc.view(loc.size(0), -1, 4),
                      self.softmax(conf.view(conf.size(0), -1, self.num_classes)))
        else:
            output = (loc.view(loc.size(0), -1, 4),
                      conf.view(conf.size(0), -1, self.num_classes))

        return output


```

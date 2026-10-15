# Facial rendering source scan: gaoxiangjun/MPS-NeRF

Repository: https://github.com/gaoxiangjun/MPS-NeRF

Pinned source revision: 02a2842cff8bca4206d0cae9339fa15e9694de1f

Discovery topic: human rendering

Description: [TPAMI' 2022'] MPS-NeRF: Generalizable 3D Human Rendering from Multiview Images

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["lib/skinnning_batch.py"]}

## Source file: lib/skinnning_batch.py

{
  "path": "lib/skinnning_batch.py",
  "blob_sha": "35bf626d6912992909745ebd97f34547c1eab70a",
  "sha256": "90b98a037d519c0ae96b9361988cf4b7c4d0eba11dd9c66817c98dcf997a7a35",
  "lines": 2042,
  "bytes": 105208,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 11,
    "skin": 16,
    "deform": 67,
    "lbs": 17,
    "rig": 11,
    "face": 20
  },
  "url": "https://github.com/gaoxiangjun/MPS-NeRF/blob/02a2842cff8bca4206d0cae9339fa15e9694de1f/lib/skinnning_batch.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def normalize_v3(arr):"
    },
    {
      "line": 28,
      "text": "def compute_normal(vertices, faces):"
    },
    {
      "line": 43,
      "text": "def batch_rodrigues(rot_vecs, epsilon=1e-8, dtype=torch.float32):"
    },
    {
      "line": 79,
      "text": "def __init__(self, D=8, W=256, input_ch=3, output_ch=3, skips=[4], deform_type=\"weights\"):"
    },
    {
      "line": 93,
      "text": "def forward(self, x):"
    },
    {
      "line": 110,
      "text": "def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False,"
    },
    {
      "line": 166,
      "text": "def normalize_pts(self, source_pts, bounds):"
    },
    {
      "line": 177,
      "text": "def projection(self, query_pts, R, T, K):"
    },
    {
      "line": 186,
      "text": "def encode_images(self, images):"
    },
    {
      "line": 193,
      "text": "def big_pose_params(self, params):"
    },
    {
      "line": 203,
      "text": "def coarse_deform_target2c(self, params, vertices,  query_pts):"
    },
    {
      "line": 253,
      "text": "def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):"
    },
    {
      "line": 302,
      "text": "def forward_fusion(self, query_pts, params, point_3d_feature, agg_feature=None):"
    },
    {
      "line": 311,
      "text": "def backward_fusion(self, canonical_pts, embedding=None):"
    },
    {
      "line": 318,
      "text": "def prepare_spconv(self, tp_input, t_vertices=False):"
    },
    {
      "line": 330,
      "text": "def set_extract_mesh(self, flag):"
    },
    {
      "line": 333,
      "text": "def forward(self, sp_input, tp_input, world_query_pts, viewdir):"
    },
    {
      "line": 518,
      "text": "def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False,"
    },
    {
      "line": 576,
      "text": "def normalize_pts(self, source_pts, bounds):"
    },
    {
      "line": 587,
      "text": "def projection(self, query_pts, R, T, K):"
    },
    {
      "line": 597,
      "text": "def encode_images(self, images):"
    },
    {
      "line": 604,
      "text": "def big_pose_params(self, params):"
    },
    {
      "line": 614,
      "text": "def coarse_deform_target2c(self, params, vertices,  query_pts):"
    },
    {
      "line": 664,
      "text": "def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):"
    },
    {
      "line": 713,
      "text": "def forward_fusion(self, query_pts, params, point_3d_feature, agg_feature=None):"
    },
    {
      "line": 722,
      "text": "def backward_fusion(self, canonical_pts, embedding=None):"
    },
    {
      "line": 729,
      "text": "def prepare_spconv(self, tp_input, t_vertices=False):"
    },
    {
      "line": 741,
      "text": "def set_extract_mesh(self, flag):"
    },
    {
      "line": 744,
      "text": "def forward(self, sp_input, tp_input, world_query_pts, viewdir):"
    },
    {
      "line": 884,
      "text": "def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False,"
    },
    {
      "line": 944,
      "text": "def normalize_pts(self, source_pts, bounds):"
    },
    {
      "line": 955,
      "text": "def projection(self, query_pts, R, T, K):"
    },
    {
      "line": 965,
      "text": "def encode_images(self, images):"
    },
    {
      "line": 972,
      "text": "def big_pose_params(self, params):"
    },
    {
      "line": 982,
      "text": "def coarse_deform_target2c(self, params, vertices,  query_pts):"
    },
    {
      "line": 1032,
      "text": "def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):"
    },
    {
      "line": 1081,
      "text": "def forward_fusion(self, query_pts, params, embedding, point_3d_feature, agg_feature=None):"
    },
    {
      "line": 1092,
      "text": "def backward_fusion(self, query_pts, params, embedding, point_3d_feature):"
    },
    {
      "line": 1100,
      "text": "def prepare_spconv(self, tp_input, t_vertices=False):"
    },
    {
      "line": 1112,
      "text": "def set_extract_mesh(self, flag):"
    },
    {
      "line": 1115,
      "text": "def forward(self, sp_input, tp_input, world_query_pts, viewdir):"
    },
    {
      "line": 1293,
      "text": "def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False,"
    },
    {
      "line": 1353,
      "text": "def normalize_pts(self, source_pts, bounds):"
    },
    {
      "line": 1364,
      "text": "def projection(self, query_pts, R, T, K):"
    },
    {
      "line": 1374,
      "text": "def encode_images(self, images):"
    },
    {
      "line": 1381,
      "text": "def big_pose_params(self, params):"
    },
    {
      "line": 1391,
      "text": "def coarse_deform_target2c(self, params, vertices,  query_pts):"
    },
    {
      "line": 1441,
      "text": "def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):"
    },
    {
      "line": 1490,
      "text": "def set_extract_mesh(self, flag):"
    },
    {
      "line": 1493,
      "text": "def forward(self, sp_input, tp_input, world_query_pts, viewdir):"
    }
  ]
}

```text

from random import sample
from lib.run_nerf_helpers import PositionalEncoding, wide_sigmoid
import torch.nn as nn
from pytorch3d.ops.knn import knn_points
import spconv.pytorch as spconv
import torch.nn.functional as F
import torch
import cv2
from lib.encoder import *
from lib.base_utils import read_pickle
# from h36m_blend_weights import *
from lib.transformer import Transformer
from lib.run_nerf_helpers import *
import copy


def normalize_v3(arr):
    ''' Normalize a numpy array of 3 component vectors shape=(n,3) '''
    lens = torch.sqrt(arr[:, 0] ** 2 + arr[:, 1] ** 2 + arr[:, 2] ** 2)
    eps = 0.00000001
    lens[lens < eps] = eps
    arr[:, 0] /= lens
    arr[:, 1] /= lens
    arr[:, 2] /= lens
    return arr


def compute_normal(vertices, faces):
    # norm = np.zeros(vertices.shape, dtype=vertices.dtype)
    norm = torch.zeros(vertices.shape, dtype=vertices.dtype).cuda()
    tris = vertices[faces]
    # n = np.cross(tris[::, 1] - tris[::, 0], tris[::, 2] - tris[::, 0])
    n = torch.cross(tris[::, 1] - tris[::, 0], tris[::, 2] - tris[::, 0])
    normalize_v3(n)
    norm[faces[:, 0]] += n
    norm[faces[:, 1]] += n
    norm[faces[:, 2]] += n
    normalize_v3(norm)

    return norm


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
    

class DeformField(nn.Module):

    def __init__(self, D=8, W=256, input_ch=3, output_ch=3, skips=[4], deform_type="weights"):
        super(DeformField, self).__init__()
        self.D = D
        self.W = W
        self.skips = skips
        
        self.pts_time_linears = nn.ModuleList(
            [nn.Linear(input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + input_ch, W) for i in range(D-1)])
        self.output_linear = nn.Linear(W, output_ch)
        if deform_type=='correction':
            torch.nn.init.constant_(self.output_linear.weight, 0)
            torch.nn.init.constant_(self.output_linear.bias, 0)
        self.deform_type = deform_type
    
    def forward(self, x):
        input_point_time = x
        h = input_point_time
        for i, l in enumerate(self.pts_time_linears):
            h = self.pts_time_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([input_point_time, h], -1)

        outputs = self.output_linear(h)
        if self.deform_type=='weights':
            outputs = F.softmax(outputs, dim=1)

        return outputs


class SKinningBatch(nn.Module):
    def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False, 
                smooth_loss=False, num_instances=1, mean_shape=1, correction_field=1, skinning_field=1, data_set_type="H36M_B", append_rgb="False"):
        super(SKinningBatch, self).__init__()
        self.encoder_3d = SparseConvNet(num_layers=2)
        self.encoder_2d = SpatialEncoder(num_layers=2)# no smooth loss

        self.forward_deform = DeformField(D=2, input_ch=(39+32+128), output_ch=3, deform_type='correction')
        self.backward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')

        self.with_viewdirs = with_viewdirs
        self.pos_enc = PositionalEncoding(num_freqs=6)
        self.view_enc = PositionalEncoding(num_freqs=4)
        
        male_smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl')
        female_smpl_path = os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')
        neutral_smpl_path = os.path.join('assets', 'SMPL_NEUTRAL.pkl')
        self.SMPL_MALE = SMPL_to_tensor(read_pickle(male_smpl_path))
        self.SMPL_FEMALE = SMPL_to_tensor(read_pickle(female_smpl_path))
        self.SMPL_NEU = SMPL_to_tensor(read_pickle(neutral_smpl_path))
        self.SMPL_NEUTRAL = self.SMPL_NEU # SMPL_to_tensor(read_pickle(smpl_path))

        self.faces = self.SMPL_NEUTRAL['f'] #.astype(np.int32)
        self.smpl_pts_normal = None #compute_normal(t_vertices, faces)
        W = 256
        self.image_shape = torch.zeros((2,))

        self.actvn = nn.ReLU()
        self.skips = [4]
        nerf_input_ch = (39+128) if not append_rgb else (39+128+27)#if smooth_loss else (39+32+128)
        self.pts_linears = nn.ModuleList(
            [nn.Linear(nerf_input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + nerf_input_ch, W) for i in range(7)])
        self.alpha_linear = nn.Linear(W, 1)
        self.feature_linear = nn.Linear(W, W)
        self.rgb_linear = nn.Linear(W//2, 3)
        
        self.extract_mesh = False
        self.mesh_animation = False
        self.use_agg = use_agg
        self.data_set_type = data_set_type
        self.human_sample = human_sample
        self.density_loss = density_loss
        self.use_f2d = use_f2d
        self.use_trans = use_trans
        self.smooth_loss = smooth_loss
        self.mean_shape = mean_shape
        self.correction_field = correction_field
        self.skinning_field = skinning_field
        self.append_rgb = append_rgb

        nerf_input_ch_2 = 384 if not append_rgb else (128+256+27) # 128 fused feature + 256 alpha feature + rgb code 
        self.views_linear = nn.Linear((nerf_input_ch_2+27) if self.with_viewdirs else nerf_input_ch_2, W//2)
        self.transformer = None if not use_trans else Transformer(128 if not append_rgb else (128+27))
        self.latent_codes = nn.Embedding(num_instances, 128)
        self.extract_mesh = False
        nn.init.normal_(self.latent_codes.weight, mean=0, std=0.01)

    def normalize_pts(self, source_pts, bounds):
        # bounds 2 * 3
        # source_pts N * 3
        mean_plus = 0.5 * (bounds[0] + bounds[1])
        mean_plus = mean_plus
        mean_sub = 0.5 * (bounds[1] - bounds[0])
        mean_sub = mean_sub

        normalized_pts = ((source_pts - mean_plus) / mean_sub)
        return normalized_pts

    def projection(self, query_pts, R, T, K):
        RT = torch.cat([R, T], 2)
        xyz = torch.repeat_interleave(query_pts.unsqueeze(dim=0), repeats=RT.shape[0], dim=0)
        xyz = torch.bmm(xyz.float(), RT[:, :, :3].transpose(1, 2).float()) + RT[:, :, 3:].transpose(1, 2).float()
        xyz = torch.bmm(xyz, K.transpose(1, 2).float())
        xy = xyz[:, :, :2] / (xyz[:, :, 2:] + 1e-5)
        
        return xy
    
    def encode_images(self, images):
        self.images = images
        self.image_shape[0] = images.shape[-1]
        self.image_shape[1] = images.shape[-2]

        self.encoder_2d(images)

    def big_pose_params(self, params):
        big_pose_params = copy.deepcopy(params)
        big_pose_params['poses'] = torch.zeros((1,72))
        big_pose_params['poses'][0, 5] = 45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 8] = -45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 23] = -30/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 26] = 30/180*torch.tensor(np.pi)

        return big_pose_params

    def coarse_deform_target2c(self, params, vertices,  query_pts):

        # joints transformation
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)        
        J = torch.hstack((joints, torch.ones([joints.shape[0], 1])))
        # self.c_joints = joints # smpl space
        # self.t_A = A # target to canonical space transformation
        # self.t_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]

        # transform smpl vertices from world space to smpl space
        smpl_pts = torch.mm((vertices - Th), R)
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # From smpl space target pose to smpl space canonical pose
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        can_pts = torch.sum(R_inv * can_pts[:, None], dim=2)
        
        if self.mean_shape:
            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            can_pts = can_pts - pose_offsets[vert_ids.squeeze(0).reshape(-1)]

            # To mean shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            # shape_offset = shapedirs.dot(torch.reshape(params['shapes'], (10,)))
            # shape_offset = torch.einsum('ijk, k -> ij', shapedirs, torch.reshape(params['shapes'], (10,)))
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            can_pts = can_pts - shape_offset[vert_ids.squeeze(0).reshape(-1)]

        # From T To Big Pose        
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        # J = torch.hstack((self.c_joints, torch.ones([self.c_joints.shape[0], 1])))
        # self.c_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * can_pts[:, None], dim=2)
        can_pts = can_pts + A[:, :3, 3]

        return can_pts 

    def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):
        # Find nearest smpl vertex
        smpl_pts = t_vertices
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # add weights_correction, normalize weights
        # bweights = F.softmax(bweights + 0.2*weights_correction, dim=1)
        bweights = bweights + 0.2 * weights_correction
        bweights = bweights / torch.sum(bweights, dim=-1, keepdim=True)

        ### From Big To T Pose
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        query_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        query_pts = torch.sum(R_inv * query_pts[:, None], dim=2)

        if self.mean_shape:
            # From mean shape to normal shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            query_pts = query_pts + shape_offset[vert_ids.squeeze(0).reshape(-1)]

            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            query_pts = query_pts + pose_offsets[vert_ids.squeeze(0).reshape(-1)]

        # get tar_pts, smpl space source pose
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)
        self.s_A = A
        A = torch.mm(bweights, self.s_A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * query_pts[:, None], dim=2)
        smpl_src_pts = can_pts + A[:, :3, 3]
        
        # transform points from the smpl space to the world space
        R_inv = torch.inverse(R)
        world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th
        
        return smpl_src_pts, world_src_pts, bweights

    def forward_fusion(self, query_pts, params, point_3d_feature, agg_feature=None):
        """(x, x-joint_t, target_f_3d) → deformation Net 1 → X_c"""
        query_pts_code = self.pos_enc(query_pts)
        if agg_feature != None:
            f = torch.cat([query_pts_code, point_3d_feature, agg_feature], dim=-1) # 39 + 72 + 32 + 128 = 271
        else:
            f = torch.cat([query_pts_code, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def backward_fusion(self, canonical_pts, embedding=None):
        """(X_c, Pose_s, shape, x-joint_c, f_agg) → deformation Net 2 → X_s"""
        canonical_pts_code = self.pos_enc(canonical_pts) # torch.Size([16384, 3])
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), canonical_pts_code.shape[0], dim=0) # torch.Size([16384, 24, 3])
        f = torch.cat([canonical_pts_code, embedding], dim=-1) # torch.Size([16384, 669])
        return f
    
    def prepare_spconv(self, tp_input, t_vertices=False):
        pre = 't_' if t_vertices else ''
        xyz_input_feature = tp_input[pre+'feature'] #torch.from_numpy(tp_input['feature']).cuda().float()
        voxel_integer_coord = tp_input[pre+'coord'].int() #torch.from_numpy(tp_input['coord']).cuda().float().int()
        coord = torch.zeros_like(voxel_integer_coord[:,0:1])
        voxel_integer_coord = torch.cat((coord, voxel_integer_coord), dim=1)
        out_sh = tp_input[pre+'out_sh'].cpu().numpy().astype(np.int32)
        batch_size = 1
        sparse_smpl_vertices = spconv.SparseConvTensor(xyz_input_feature, voxel_integer_coord, out_sh, batch_size)

        return sparse_smpl_vertices

    def set_extract_mesh(self, flag):
        self.extract_mesh = flag

    def forward(self, sp_input, tp_input, world_query_pts, viewdir):

        if sp_input["gender"][0] == 1:
            self.SMPL_NEUTRAL = self.SMPL_MALE
        elif sp_input["gender"][0] == 0:
            self.SMPL_NEUTRAL = self.SMPL_FEMALE
        else:
            self.SMPL_NEUTRAL = self.SMPL_NEU
        ## translate query points from world space target pose to smpl space target pose
        world_query_pts = world_query_pts.squeeze(0)
        viewdir = viewdir.squeeze(0)
        sp_input, tp_input = sequeeze_0(sp_input, tp_input)
        R = tp_input['params']['R']
        Th = tp_input['params']['Th'] #.astype(np.float32)
        smpl_query_pts = torch.mm(world_query_pts - Th, R)
        
        ## encode images
        img_all = sp_input['img_all']
        self.encode_images(img_all)
        
        ## human region sample
        if self.human_sample:
            tar_smpl_pts = tp_input['vertices']
            tar_smpl_pts = torch.mm(tar_smpl_pts - Th, R)
            distance, _, _ = knn_points(smpl_query_pts.unsqueeze(0).float(), tar_smpl_pts.unsqueeze(0).float(), K=1)
            distance = distance.squeeze(0).view(-1)
            pts_mask = torch.zeros_like(smpl_query_pts[:,0]).cuda().int()
            threshold = 0.05 ** 2
            pts_mask[distance < threshold] = 1
            
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            smpl_query_pts = smpl_query_pts[pts_mask==1]
            viewdir = viewdir[pts_mask==1]
        else:
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
        
        ## coarse deform target to caonical
        coarse_canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts)        
        
        ## Calculate correction
        if self.correction_field or self.skinning_field:
            sparse_smpl_vertices = self.prepare_spconv(tp_input, t_vertices=False)
            normalized_source_pts = self.normalize_pts(smpl_query_pts, tp_input['bounds'])
            point_3d_feature_0 = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts.detach())
            if self.data_set_type in ["H36M_P", "THuman_P"]:
                _, coarse_world_src_pts, _ = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], query_pts=coarse_canonical_pts, weights_correction=0.)
                uv = self.projection(coarse_world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
            else:
                uv = self.projection(world_query_pts[pts_mask==1].detach(), sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
            agg_feature = torch.mean(self.encoder_2d.index(uv, self.image_shape), dim=0).transpose(0,1)        
            fused_feature = self.forward_fusion(smpl_query_pts, tp_input['params'], point_3d_feature_0, agg_feature)
        
        ## More Accurate canonical_pts
        if self.correction_field:
            correction = self.forward_deform(fused_feature.float())
            canonical_pts = coarse_canonical_pts + correction
        else:
            canonical_pts = coarse_canonical_pts.requires_grad_()

        ## Extract Mesh
        if self.extract_mesh or self.mesh_animation:
            canonical_pts = world_query_pts
            pts_mask = torch.ones_like(canonical_pts[:,0]).cuda().int()

        ## Use Skinning weights Field to do deform
        instance_idx = tp_input['instance_idx'].long()
        embedding = self.latent_codes(instance_idx)
        fused_feature = self.backward_fusion(canonical_pts.clone(), embedding)
        if self.skinning_field:
            weights_correction = self.backward_deform(fused_feature.float())
        else:
            weights_correction = 0.
        smpl_src_pts, world_src_pts, bweights = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], canonical_pts, weights_correction)
        
        if self.smooth_loss and self.training:
            _, vert_ids, _ = knn_points(canonical_pts.unsqueeze(0).float(), sp_input['t_vertices'].unsqueeze(0).float(), K=1)
            if self.smpl_pts_normal==None:
                self.smpl_pts_normal = compute_normal(sp_input['t_vertices'], self.faces)
            nearest_smpl_normal = self.smpl_pts_normal[vert_ids.squeeze(0)].reshape(-1, 3)

        ## Get canonical 3d geometry aligned feature
        # sparse_smpl_vertices = self.prepare_spconv(tp_input, t_vertices=True)
        # normalized_source_pts = self.normalize_pts(canonical_pts, sp_input['t_bounds'])
        # point_3d_feature = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts)
        
        ## Get mean pixel-aligned feature four view
        uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        # if self.correction_field or tp_input['pose_index']!=sp_input['pose_index']:
        #     uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        # else:
        #     uv = self.projection(world_query_pts[pts_mask==1], sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        point_2d_feature_0 = self.encoder_2d.index(uv, self.image_shape)
        point_2d_feature_0 = point_2d_feature_0.transpose(1,2)
        
        if self.append_rgb:
            img = sp_input['img_all']
            uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
            samples = grid_sample(img, uv_)
            samples = samples.squeeze(-1).transpose(1,2)
            sh = samples.shape
            samples = self.view_enc(samples.reshape(-1,3)).reshape(sh[0],sh[1],27)
            point_2d_feature_0 = torch.cat((point_2d_feature_0, samples), dim=-1)

        ## Transformer Fusion of multiview Feature
        if self.use_trans and point_2d_feature_0.shape[1]!=0:
            point_2d_feature = point_2d_feature_0.transpose(0,1)
            point_2d_feature = self.transformer(point_2d_feature)
            point_2d_feature_1 = point_2d_feature[:,0,:]
            point_2d_feature_2 = point_2d_feature[:,1,:]
        else:
            point_2d_feature = torch.mean(point_2d_feature_0, dim=0)
            point_2d_feature_1 = point_2d_feature
            point_2d_feature_2 = point_2d_feature

        # Run in Nerf
        x = self.pos_enc(canonical_pts)
        # if not self.smooth_loss:
        #     x = torch.cat((x, point_3d_feature, point_2d_feature_1), dim=1)
        # else:
        #     x = torch.cat((x, point_2d_feature_1), dim=1)
        x = torch.cat((x, point_2d_feature_1), dim=1)
        h = x
        for i, l in enumerate(self.pts_linears):
            h = self.pts_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([x, h], -1)

        alpha = self.alpha_linear(h)
        feature = self.feature_linear(h)
        
        if self.with_viewdirs:
            viewdir = self.view_enc(viewdir)
            h = torch.cat([feature, viewdir, point_2d_feature_2], -1)
        else:
            h = torch.cat([feature, point_2d_feature_2], -1)

        h = self.views_linear(h)
        h = F.relu(h)
        rgb = self.rgb_linear(h)
        
        if self.mesh_animation:
            raw = torch.cat([world_src_pts, bweights, rgb, alpha], dim=-1)
            return raw
        if self.extract_mesh:
            raw = torch.cat([rgb, alpha], -1).unsqueeze(0)
            return raw

        if self.human_sample:
            ret_smpl_query_pts, ret_smpl_src_pts = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            ret_smpl_query_pts[pts_mask==1], ret_smpl_src_pts[pts_mask==1] = smpl_query_pts, smpl_src_pts
            ret_correction, ret_correction_ = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            all_occ_normal, all_nearest_smpl_normal = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            
            if self.correction_field:
                ret_correction[pts_mask==1] = correction

            raw = torch.zeros(zero_pts.shape[0],4).cuda().float()
            raw[pts_mask==1] = torch.cat([rgb, alpha], -1)
            raw[pts_mask==0] = -80 # softplus and sigmoid is outside
            raw = torch.cat([raw, torch.unsqueeze(pts_mask, -1), ret_correction, ret_correction_, ret_smpl_query_pts, ret_smpl_src_pts], -1)

            if self.smooth_loss and self.training:
                intv_flag = (int(sp_input['global_step'].item()) % int(sp_input['smooth_interval'].item()))==0
                if intv_flag:
                    occ_normal = torch.autograd.grad(wide_sigmoid(alpha), [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = occ_normal / (torch.norm(occ_normal, dim=-1, keepdim=True) + 1e-8)
                    all_occ_normal[pts_mask==1] = occ_normal
                    all_nearest_smpl_normal[pts_mask==1] = nearest_smpl_normal
                    # raw = torch.cat([raw, all_occ_normal], -1)
                    raw = torch.cat([raw, all_occ_normal, all_nearest_smpl_normal], -1)

                    del x, h, point_2d_feature, point_2d_feature_0, weights_correction, fused_feature
                    del feature, canonical_pts, smpl_src_pts, world_src_pts, instance_idx
                    del coarse_canonical_pts, pts_mask, sp_input, tp_input, world_query_pts, viewdir, occ_normal
        else:
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
            pts_mask = torch.unsqueeze(pts_mask, -1)
            raw = torch.cat([rgb, alpha, pts_mask, correction], -1)
        
        return raw.unsqueeze(0)


class DirectDeform(nn.Module):
    def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False, 
                smooth_loss=False, num_instances=1, mean_shape=1, correction_field=1, skinning_field=1, data_set_type="H36M_B", append_rgb="False"):
        super(DirectDeform, self).__init__()
        self.encoder_3d = SparseConvNet(num_layers=2)
        self.encoder_2d = SpatialEncoder(num_layers=2)# no smooth loss
        # self.encoder_2d = ImageViewEncoder(num_layers=2)# smooth loss
        self.forward_deform = DeformField(D=2, input_ch=(39+32+128), output_ch=3, deform_type='correction')
        self.backward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        self.use_agg = use_agg
        self.with_viewdirs = with_viewdirs
        self.pos_enc = PositionalEncoding(num_freqs=6)
        self.view_enc = PositionalEncoding(num_freqs=4)
        
        male_smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl')
        female_smpl_path = os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')
        neutral_smpl_path = os.path.join('assets', 'SMPL_NEUTRAL.pkl')
        self.SMPL_MALE = SMPL_to_tensor(read_pickle(male_smpl_path))
        self.SMPL_FEMALE = SMPL_to_tensor(read_pickle(female_smpl_path))
        self.SMPL_NEU = SMPL_to_tensor(read_pickle(neutral_smpl_path))

        smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl') if data_set_type[0]=="T" else os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')
        self.data_set_type = data_set_type
        self.SMPL_NEUTRAL = self.SMPL_NEU # SMPL_to_tensor(read_pickle(smpl_path))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')))
        self.faces = self.SMPL_NEUTRAL['f'] #.astype(np.int32)
        self.smpl_pts_normal = None #compute_normal(t_vertices, faces)
        W = 256
        self.image_shape = torch.zeros((2,))

        # xc + f3d + f2d
        self.actvn = nn.ReLU()
        self.skips = [4]
        nerf_input_ch = (128) if not append_rgb else (128+27)#if smooth_loss else (39+32+128)
        self.pts_linears = nn.ModuleList(
            [nn.Linear(nerf_input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + nerf_input_ch, W) for i in range(7)])
        self.alpha_linear = nn.Linear(W, 1)
        self.feature_linear = nn.Linear(W, W)
        self.rgb_linear = nn.Linear(W//2, 3)
        self.extract_mesh = False
        self.mesh_animation = False
        self.human_sample = human_sample
        self.density_loss = density_loss
        self.use_f2d = use_f2d
        self.use_trans = use_trans
        self.smooth_loss = smooth_loss
        self.mean_shape = mean_shape
        self.correction_field = correction_field
        self.skinning_field = skinning_field
        self.append_rgb = append_rgb

        nerf_input_ch_2 = 384 if not append_rgb else (128+256+27) # 128 fused feature + 256 alpha feature + rgb code 
        self.views_linear = nn.Linear((nerf_input_ch_2+27) if self.with_viewdirs else nerf_input_ch_2, W//2)
        self.transformer = None if not use_trans else Transformer(128 if not append_rgb else (128+27))
        self.latent_codes = nn.Embedding(num_instances, 128)
        self.extract_mesh = False
        nn.init.normal_(self.latent_codes.weight, mean=0, std=0.01)

    def normalize_pts(self, source_pts, bounds):
        # bounds 2 * 3
        # source_pts N * 3
        mean_plus = 0.5 * (bounds[0] + bounds[1])
        mean_plus = mean_plus
        mean_sub = 0.5 * (bounds[1] - bounds[0])
        mean_sub = mean_sub

        normalized_pts = ((source_pts - mean_plus) / mean_sub)
        return normalized_pts

    def projection(self, query_pts, R, T, K):
        # https://github.com/pengsida/if-nerf/blob/492e765423fcbfb9aeca345286a49074cc4e5a90/lib/utils/render_utils.py#L45
        RT = torch.cat([R, T], 2)
        xyz = torch.repeat_interleave(query_pts.unsqueeze(dim=0), repeats=RT.shape[0], dim=0)
        xyz = torch.bmm(xyz.float(), RT[:, :, :3].transpose(1, 2).float()) + RT[:, :, 3:].transpose(1, 2).float()
        xyz = torch.bmm(xyz, K.transpose(1, 2).float())
        xy = xyz[:, :, :2] / (xyz[:, :, 2:] + 1e-5)
        
        return xy
    
    def encode_images(self, images):
        self.images = images
        self.image_shape[0] = images.shape[-1]
        self.image_shape[1] = images.shape[-2]

        self.encoder_2d(images)

    def big_pose_params(self, params):
        big_pose_params = copy.deepcopy(params)
        big_pose_params['poses'] = torch.zeros((1,72))
        big_pose_params['poses'][0, 5] = 45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 8] = -45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 23] = -30/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 26] = 30/180*torch.tensor(np.pi)

        return big_pose_params

    def coarse_deform_target2c(self, params, vertices,  query_pts):

        # joints transformation
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)        
        J = torch.hstack((joints, torch.ones([joints.shape[0], 1])))
        # self.c_joints = joints # smpl space
        # self.t_A = A # target to canonical space transformation
        # self.t_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]

        # transform smpl vertices from world space to smpl space
        smpl_pts = torch.mm((vertices - Th), R)
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # From smpl space target pose to smpl space canonical pose
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        can_pts = torch.sum(R_inv * can_pts[:, None], dim=2)
        
        if self.mean_shape:
            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            can_pts = can_pts - pose_offsets[vert_ids.squeeze(0).reshape(-1)]

            # To mean shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            # shape_offset = shapedirs.dot(torch.reshape(params['shapes'], (10,)))
            # shape_offset = torch.einsum('ijk, k -> ij', shapedirs, torch.reshape(params['shapes'], (10,)))
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            can_pts = can_pts - shape_offset[vert_ids.squeeze(0).reshape(-1)]

        # From T To Big Pose        
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        # J = torch.hstack((self.c_joints, torch.ones([self.c_joints.shape[0], 1])))
        # self.c_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * can_pts[:, None], dim=2)
        can_pts = can_pts + A[:, :3, 3]

        return can_pts 

    def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):
        # Find nearest smpl vertex
        smpl_pts = t_vertices
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # add weights_correction, normalize weights
        # bweights = F.softmax(bweights + 0.2*weights_correction, dim=1)
        bweights = bweights + 0.2 * weights_correction
        bweights = bweights / torch.sum(bweights, dim=-1, keepdim=True)

        ### From Big To T Pose
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        query_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        query_pts = torch.sum(R_inv * query_pts[:, None], dim=2)

        if self.mean_shape:
            # From mean shape to normal shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            query_pts = query_pts + shape_offset[vert_ids.squeeze(0).reshape(-1)]

            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            query_pts = query_pts + pose_offsets[vert_ids.squeeze(0).reshape(-1)]

        # get tar_pts, smpl space source pose
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)
        self.s_A = A
        A = torch.mm(bweights, self.s_A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * query_pts[:, None], dim=2)
        smpl_src_pts = can_pts + A[:, :3, 3]
        
        # transform points from the smpl space to the world space
        R_inv = torch.inverse(R)
        world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th
        
        return smpl_src_pts, world_src_pts, bweights

    def forward_fusion(self, query_pts, params, point_3d_feature, agg_feature=None):
        """(x, x-joint_t, target_f_3d) → deformation Net 1 → X_c"""
        query_pts_code = self.pos_enc(query_pts)
        if agg_feature != None:
            f = torch.cat([query_pts_code, point_3d_feature, agg_feature], dim=-1) # 39 + 72 + 32 + 128 = 271
        else:
            f = torch.cat([query_pts_code, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def backward_fusion(self, canonical_pts, embedding=None):
        """(X_c, Pose_s, shape, x-joint_c, f_agg) → deformation Net 2 → X_s"""
        canonical_pts_code = self.pos_enc(canonical_pts) # torch.Size([16384, 3])
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), canonical_pts_code.shape[0], dim=0) # torch.Size([16384, 24, 3])
        f = torch.cat([canonical_pts_code, embedding], dim=-1) # torch.Size([16384, 669])
        return f
    
    def prepare_spconv(self, tp_input, t_vertices=False):
        pre = 't_' if t_vertices else ''
        xyz_input_feature = tp_input[pre+'feature'] #torch.from_numpy(tp_input['feature']).cuda().float()
        voxel_integer_coord = tp_input[pre+'coord'].int() #torch.from_numpy(tp_input['coord']).cuda().float().int()
        coord = torch.zeros_like(voxel_integer_coord[:,0:1])
        voxel_integer_coord = torch.cat((coord, voxel_integer_coord), dim=1)
        out_sh = tp_input[pre+'out_sh'].cpu().numpy().astype(np.int32)
        batch_size = 1
        sparse_smpl_vertices = spconv.SparseConvTensor(xyz_input_feature, voxel_integer_coord, out_sh, batch_size)

        return sparse_smpl_vertices

    def set_extract_mesh(self, flag):
        self.extract_mesh = flag

    def forward(self, sp_input, tp_input, world_query_pts, viewdir):

        if sp_input["gender"][0] == 1:
            self.SMPL_NEUTRAL = self.SMPL_MALE
        elif sp_input["gender"][0] == 0:
            self.SMPL_NEUTRAL = self.SMPL_FEMALE
        else:
            self.SMPL_NEUTRAL = self.SMPL_NEU
        ## translate query points from world space target pose to smpl space target pose
        world_query_pts = world_query_pts.squeeze(0)
        viewdir = viewdir.squeeze(0)
        sp_input, tp_input = sequeeze_0(sp_input, tp_input)
        R = tp_input['params']['R']
        Th = tp_input['params']['Th'] #.astype(np.float32)
        smpl_query_pts = torch.mm(world_query_pts - Th, R)
        
        ## encode images
        img_all = sp_input['img_all']
        self.encode_images(img_all)
        
        ## human region sample
        if self.human_sample:
            tar_smpl_pts = tp_input['vertices']
            tar_smpl_pts = torch.mm(tar_smpl_pts - Th, R)
            distance, _, _ = knn_points(smpl_query_pts.unsqueeze(0).float(), tar_smpl_pts.unsqueeze(0).float(), K=1)
            distance = distance.squeeze(0).view(-1)
            pts_mask = torch.zeros_like(smpl_query_pts[:,0]).cuda().int()
            threshold = 0.05 ** 2
            pts_mask[distance < threshold] = 1
            
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            smpl_query_pts = smpl_query_pts[pts_mask==1]
            viewdir = viewdir[pts_mask==1]
        else:
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
        
        ## coarse deform target to caonical
        coarse_canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts)        
        canonical_pts = coarse_canonical_pts #.requires_grad_()
        

        ## Extract Mesh
        if self.extract_mesh or self.mesh_animation:
            canonical_pts = world_query_pts
            pts_mask = torch.ones_like(canonical_pts[:,0]).cuda().int()

        ## Use Skinning weights Field to do deform
        weights_correction = 0.
        smpl_src_pts, world_src_pts, bweights = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], canonical_pts, weights_correction)
        
        if self.smooth_loss and self.training:
            _, vert_ids, _ = knn_points(canonical_pts.unsqueeze(0).float(), sp_input['t_vertices'].unsqueeze(0).float(), K=1)
            if self.smpl_pts_normal==None:
                self.smpl_pts_normal = compute_normal(sp_input['t_vertices'], self.faces)
            nearest_smpl_normal = self.smpl_pts_normal[vert_ids.squeeze(0)].reshape(-1, 3)
        
        uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        point_2d_feature_0 = self.encoder_2d.index(uv, self.image_shape)
        point_2d_feature_0 = point_2d_feature_0.transpose(1,2)
        
        if self.append_rgb:
            img = sp_input['img_all']
            uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
            samples = grid_sample(img, uv_)
            samples = samples.squeeze(-1).transpose(1,2)
            sh = samples.shape
            samples = self.view_enc(samples.reshape(-1,3)).reshape(sh[0],sh[1],27)
            point_2d_feature_0 = torch.cat((point_2d_feature_0, samples), dim=-1)

        ## Transformer Fusion of multiview Feature
        if self.use_trans and point_2d_feature_0.shape[1]!=0:
            point_2d_feature = point_2d_feature_0.transpose(0,1)
            point_2d_feature = self.transformer(point_2d_feature)
            point_2d_feature_1 = point_2d_feature[:,0,:]
            point_2d_feature_2 = point_2d_feature[:,1,:]
        else:
            point_2d_feature = torch.mean(point_2d_feature_0, dim=0)
            point_2d_feature_1 = point_2d_feature
            point_2d_feature_2 = point_2d_feature

        # Run in Nerf
        # x = self.pos_enc(canonical_pts)
        # x = torch.cat((x, point_2d_feature_1), dim=1)
        x = point_2d_feature_1
        h = x
        for i, l in enumerate(self.pts_linears):
            h = self.pts_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([x, h], -1)

        alpha = self.alpha_linear(h)
        feature = self.feature_linear(h)
        
        if self.with_viewdirs:
            viewdir = self.view_enc(viewdir)
            h = torch.cat([feature, viewdir, point_2d_feature_2], -1)
        else:
            h = torch.cat([feature, point_2d_feature_2], -1)

        h = self.views_linear(h)
        h = F.relu(h)
        rgb = self.rgb_linear(h)
        
        if self.mesh_animation:
            raw = torch.cat([world_src_pts, bweights, rgb, alpha], dim=-1)
            return raw
        if self.extract_mesh:
            raw = torch.cat([rgb, alpha], -1).unsqueeze(0)
            return raw

        if self.human_sample:
            ret_smpl_query_pts, ret_smpl_src_pts = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            ret_smpl_query_pts[pts_mask==1], ret_smpl_src_pts[pts_mask==1] = smpl_query_pts, smpl_src_pts
            ret_correction, ret_correction_ = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            all_occ_normal, all_nearest_smpl_normal = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            raw = torch.zeros(zero_pts.shape[0],4).cuda().float()
            raw[pts_mask==1] = torch.cat([rgb, alpha], -1)
            raw[pts_mask==0] = -80 # softplus and sigmoid is outside
            raw = torch.cat([raw, torch.unsqueeze(pts_mask, -1), ret_correction, ret_correction_, ret_smpl_query_pts, ret_smpl_src_pts], -1)

            if self.smooth_loss and self.training:
                intv_flag = (int(sp_input['global_step'].item()) % int(sp_input['smooth_interval'].item()))==0
                if intv_flag:
                    occ_normal = torch.autograd.grad(wide_sigmoid(alpha), [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = occ_normal / (torch.norm(occ_normal, dim=-1, keepdim=True) + 1e-8)
                    all_occ_normal[pts_mask==1] = occ_normal
                    all_nearest_smpl_normal[pts_mask==1] = nearest_smpl_normal
                    # raw = torch.cat([raw, all_occ_normal], -1)
                    raw = torch.cat([raw, all_occ_normal, all_nearest_smpl_normal], -1)
        else:
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
            pts_mask = torch.unsqueeze(pts_mask, -1)
            raw = torch.cat([rgb, alpha, pts_mask], -1)
        
        return raw.unsqueeze(0)


class CorrectionBatch(nn.Module):
    def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False, 
                smooth_loss=False, num_instances=1, mean_shape=1, correction_field=1, skinning_field=1, data_set_type="H36M_B", append_rgb="False"):
        super(CorrectionBatch, self).__init__()
        self.encoder_3d = SparseConvNet(num_layers=2)
        self.encoder_2d = SpatialEncoder(num_layers=2)# no smooth loss
        # self.encoder_2d = ImageViewEncoder(num_layers=2)# smooth loss
        self.forward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        self.backward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        # self.forward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        # self.backward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        self.use_agg = use_agg
        self.with_viewdirs = with_viewdirs
        self.pos_enc = PositionalEncoding(num_freqs=6)
        self.view_enc = PositionalEncoding(num_freqs=4)

        male_smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl')
        female_smpl_path = os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')
        neutral_smpl_path = os.path.join('assets', 'SMPL_NEUTRAL.pkl')
        self.SMPL_MALE = SMPL_to_tensor(read_pickle(male_smpl_path))
        self.SMPL_FEMALE = SMPL_to_tensor(read_pickle(female_smpl_path))
        self.SMPL_NEU = SMPL_to_tensor(read_pickle(neutral_smpl_path))

        smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl') if data_set_type[0]=="T" else os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')
        self.data_set_type = data_set_type
        self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(smpl_path))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')))
        self.faces = self.SMPL_NEUTRAL['f'] #.astype(np.int32)
        self.smpl_pts_normal = None #compute_normal(t_vertices, faces)
        W = 256
        self.image_shape = torch.zeros((2,))

        # xc + f3d + f2d
        self.actvn = nn.ReLU()
        self.skips = [4]
        nerf_input_ch = (39+128) if not append_rgb else (39+128+27)#if smooth_loss else (39+32+128)
        self.pts_linears = nn.ModuleList(
            [nn.Linear(nerf_input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + nerf_input_ch, W) for i in range(7)])
        self.alpha_linear = nn.Linear(W, 1)
        self.feature_linear = nn.Linear(W, W)
        self.rgb_linear = nn.Linear(W//2, 3)
        self.extract_mesh = False
        self.mesh_animation = False
        self.human_sample = human_sample
        self.density_loss = density_loss
        self.use_f2d = use_f2d
        self.use_trans = use_trans
        self.smooth_loss = smooth_loss
        self.mean_shape = mean_shape
        self.correction_field = correction_field
        self.skinning_field = skinning_field
        self.append_rgb = append_rgb

        nerf_input_ch_2 = 384 if not append_rgb else (128+256+27) # 128 fused feature + 256 alpha feature + rgb code 
        self.views_linear = nn.Linear((nerf_input_ch_2+27) if self.with_viewdirs else nerf_input_ch_2, W//2)
        self.transformer = None if not use_trans else Transformer(128 if not append_rgb else (128+27))
        self.latent_codes = nn.Embedding(num_instances, 128)
        self.extract_mesh = False
        nn.init.normal_(self.latent_codes.weight, mean=0, std=0.01)

    def normalize_pts(self, source_pts, bounds):
        # bounds 2 * 3
        # source_pts N * 3
        mean_plus = 0.5 * (bounds[0] + bounds[1])
        mean_plus = mean_plus
        mean_sub = 0.5 * (bounds[1] - bounds[0])
        mean_sub = mean_sub

        normalized_pts = ((source_pts - mean_plus) / mean_sub)
        return normalized_pts

    def projection(self, query_pts, R, T, K):
        # https://github.com/pengsida/if-nerf/blob/492e765423fcbfb9aeca345286a49074cc4e5a90/lib/utils/render_utils.py#L45
        RT = torch.cat([R, T], 2)
        xyz = torch.repeat_interleave(query_pts.unsqueeze(dim=0), repeats=RT.shape[0], dim=0)
        xyz = torch.bmm(xyz.float(), RT[:, :, :3].transpose(1, 2).float()) + RT[:, :, 3:].transpose(1, 2).float()
        xyz = torch.bmm(xyz, K.transpose(1, 2).float())
        xy = xyz[:, :, :2] / (xyz[:, :, 2:] + 1e-5)
        
        return xy
    
    def encode_images(self, images):
        self.images = images
        self.image_shape[0] = images.shape[-1]
        self.image_shape[1] = images.shape[-2]

        self.encoder_2d(images)

    def big_pose_params(self, params):
        big_pose_params = copy.deepcopy(params)
        big_pose_params['poses'] = torch.zeros((1,72))
        big_pose_params['poses'][0, 5] = 45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 8] = -45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 23] = -30/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 26] = 30/180*torch.tensor(np.pi)

        return big_pose_params

    def coarse_deform_target2c(self, params, vertices,  query_pts):

        # joints transformation
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)        
        J = torch.hstack((joints, torch.ones([joints.shape[0], 1])))
        # self.c_joints = joints # smpl space
        # self.t_A = A # target to canonical space transformation
        # self.t_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]

        # transform smpl vertices from world space to smpl space
        smpl_pts = torch.mm((vertices - Th), R)
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # From smpl space target pose to smpl space canonical pose
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        can_pts = torch.sum(R_inv * can_pts[:, None], dim=2)
        
        if self.mean_shape:
            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            can_pts = can_pts - pose_offsets[vert_ids.squeeze(0).reshape(-1)]

            # To mean shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            # shape_offset = shapedirs.dot(torch.reshape(params['shapes'], (10,)))
            # shape_offset = torch.einsum('ijk, k -> ij', shapedirs, torch.reshape(params['shapes'], (10,)))
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            can_pts = can_pts - shape_offset[vert_ids.squeeze(0).reshape(-1)]

        # From T To Big Pose        
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        # J = torch.hstack((self.c_joints, torch.ones([self.c_joints.shape[0], 1])))
        # self.c_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * can_pts[:, None], dim=2)
        can_pts = can_pts + A[:, :3, 3]

        return can_pts 

    def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):
        # Find nearest smpl vertex
        smpl_pts = t_vertices
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # add weights_correction, normalize weights
        # bweights = F.softmax(bweights + 0.2*weights_correction, dim=1)
        bweights = bweights + 0.2 * weights_correction
        bweights = bweights / torch.sum(bweights, dim=-1, keepdim=True)

        ### From Big To T Pose
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        query_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        query_pts = torch.sum(R_inv * query_pts[:, None], dim=2)

        if self.mean_shape:
            # From mean shape to normal shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            query_pts = query_pts + shape_offset[vert_ids.squeeze(0).reshape(-1)]

            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            query_pts = query_pts + pose_offsets[vert_ids.squeeze(0).reshape(-1)]

        # get tar_pts, smpl space source pose
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)
        self.s_A = A
        A = torch.mm(bweights, self.s_A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * query_pts[:, None], dim=2)
        smpl_src_pts = can_pts + A[:, :3, 3]
        
        # transform points from the smpl space to the world space
        R_inv = torch.inverse(R)
        world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th
        
        return smpl_src_pts, world_src_pts, bweights

    def forward_fusion(self, query_pts, params, embedding, point_3d_feature, agg_feature=None):
        """(x, x-joint_t, target_f_3d) → deformation Net 1 → X_c""" #39+72+128+32
        query_pts_code = self.pos_enc(query_pts)
        poses = torch.repeat_interleave(params['poses'].reshape(1,72), query_pts_code.shape[0], 0)
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), query_pts_code.shape[0], 0)
        if agg_feature != None:
            f = torch.cat([query_pts_code, poses, embedding, point_3d_feature, agg_feature], dim=-1) # 39 + 72 + 32 + 128 = 271
        else:
            f = torch.cat([query_pts_code, poses, embedding, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def backward_fusion(self, query_pts, params, embedding, point_3d_feature):
        """(X_c, Pose_s, shape, x-joint_c, f_agg) → deformation Net 2 → X_s"""
        query_pts_code = self.pos_enc(query_pts) # torch.Size([16384, 3])
        poses = torch.repeat_interleave(params['poses'].reshape(1,72), query_pts_code.shape[0], 0)
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), query_pts_code.shape[0], dim=0) # torch.Size([16384, 24, 3])
        f = torch.cat([query_pts_code, poses, embedding, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def prepare_spconv(self, tp_input, t_vertices=False):
        pre = 't_' if t_vertices else ''
        xyz_input_feature = tp_input[pre+'feature'] #torch.from_numpy(tp_input['feature']).cuda().float()
        voxel_integer_coord = tp_input[pre+'coord'].int() #torch.from_numpy(tp_input['coord']).cuda().float().int()
        coord = torch.zeros_like(voxel_integer_coord[:,0:1])
        voxel_integer_coord = torch.cat((coord, voxel_integer_coord), dim=1)
        out_sh = tp_input[pre+'out_sh'].cpu().numpy().astype(np.int32)
        batch_size = 1
        sparse_smpl_vertices = spconv.SparseConvTensor(xyz_input_feature, voxel_integer_coord, out_sh, batch_size)

        return sparse_smpl_vertices

    def set_extract_mesh(self, flag):
        self.extract_mesh = flag

    def forward(self, sp_input, tp_input, world_query_pts, viewdir):

        ## translate query points from world space target pose to smpl space target pose
        world_query_pts = world_query_pts.squeeze(0)
        viewdir = viewdir.squeeze(0)
        sp_input, tp_input = sequeeze_0(sp_input, tp_input)
        R = tp_input['params']['R']
        Th = tp_input['params']['Th'] #.astype(np.float32)
        smpl_query_pts = torch.mm(world_query_pts - Th, R)
        
        if sp_input["gender"] == "M":
            self.SMPL_NEUTRAL = self.SMPL_MALE
        elif sp_input["gender"] == "F":
            self.SMPL_NEUTRAL = self.SMPL_FEMALE
        else:
            self.SMPL_NEUTRAL = self.SMPL_NEU

        ## encode images
        img_all = sp_input['img_all']
        self.encode_images(img_all)
        
        ## human region sample
        if self.human_sample:
            tar_smpl_pts = tp_input['vertices']
            tar_smpl_pts = torch.mm(tar_smpl_pts - Th, R)
            distance, _, _ = knn_points(smpl_query_pts.unsqueeze(0).float(), tar_smpl_pts.unsqueeze(0).float(), K=1)
            distance = distance.squeeze(0).view(-1)
            pts_mask = torch.zeros_like(smpl_query_pts[:,0]).cuda().int()
            threshold = 0.05 ** 2
            pts_mask[distance < threshold] = 1
            
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            smpl_query_pts = smpl_query_pts[pts_mask==1]
            viewdir = viewdir[pts_mask==1]
        else:
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
        
        ## More Accurate canonical_pts
        if self.correction_field:
            sparse_smpl_vertices = self.prepare_spconv(tp_input, t_vertices=False)
            normalized_source_pts = self.normalize_pts(smpl_query_pts, tp_input['bounds'])
            point_3d_feature_0 = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts.detach())
            
            instance_idx = tp_input['instance_idx'].long()
            embedding = self.latent_codes(instance_idx)
            fused_feature = self.forward_fusion(smpl_query_pts, tp_input['params'], embedding, point_3d_feature_0)
            
            correction = self.forward_deform(fused_feature.float())
            # canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts + correction)
            canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts)
            canonical_pts = canonical_pts + correction
        else:
            canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts).requires_grad_()

        ## Extract Mesh
        if self.extract_mesh or self.mesh_animation:
            canonical_pts = world_query_pts
            pts_mask = torch.ones_like(canonical_pts[:,0]).cuda().int()

        smpl_src_pts, world_src_pts, bweights = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], canonical_pts, weights_correction=0.)
        
        if self.correction_field:
            sparse_smpl_vertices = self.prepare_spconv(sp_input, t_vertices=False)
            normalized_source_pts = self.normalize_pts(smpl_src_pts, sp_input['bounds'])
            point_3d_feature_0 = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts.detach())
            
            instance_idx = sp_input['instance_idx'].long()
            embedding = self.latent_codes(instance_idx)
            fused_feature = self.backward_fusion(smpl_src_pts.clone(), sp_input['params'], embedding, point_3d_feature_0)
            
            correction_ = self.backward_deform(fused_feature.float())
            smpl_src_pts = smpl_src_pts + correction_
            A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, sp_input['params'])
            R_inv = torch.inverse(R)
            world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th

        if self.smooth_loss and self.training:
            _, vert_ids, _ = knn_points(canonical_pts.unsqueeze(0).float(), sp_input['t_vertices'].unsqueeze(0).float(), K=1)
            if self.smpl_pts_normal==None:
                self.smpl_pts_normal = compute_normal(sp_input['t_vertices'], self.faces)
            nearest_smpl_normal = self.smpl_pts_normal[vert_ids.squeeze(0)].reshape(-1, 3)

        ## Get multi-view pixel-aligned feature four view
        if self.correction_field or tp_input['pose_index']!=sp_input['pose_index']:
            uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        else:
            uv = self.projection(world_query_pts[pts_mask==1], sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        point_2d_feature_0 = self.encoder_2d.index(uv, self.image_shape)
        point_2d_feature_0 = point_2d_feature_0.transpose(1,2)
        
        if self.append_rgb:
            img = sp_input['img_all']
            uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
            samples = grid_sample(img, uv_)
            samples = samples.squeeze(-1).transpose(1,2)
            sh = samples.shape
            samples = self.view_enc(samples.reshape(-1,3)).reshape(sh[0],sh[1],27)
            point_2d_feature_0 = torch.cat((point_2d_feature_0, samples), dim=-1)

        ## Transformer Fusion of multiview Feature
        if self.use_trans and point_2d_feature_0.shape[1]!=0:
            point_2d_feature = point_2d_feature_0.transpose(0,1)
            point_2d_feature = self.transformer(point_2d_feature)
            point_2d_feature_1 = point_2d_feature[:,0,:]
            point_2d_feature_2 = point_2d_feature[:,1,:]
        else:
            point_2d_feature = torch.mean(point_2d_feature_0, dim=0)
            point_2d_feature_1 = point_2d_feature
            point_2d_feature_2 = point_2d_feature

        # Run in Nerf
        x = self.pos_enc(canonical_pts)
        x = torch.cat((x, point_2d_feature_1), dim=1)
        h = x
        for i, l in enumerate(self.pts_linears):
            h = self.pts_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([x, h], -1)

        alpha = self.alpha_linear(h)
        feature = self.feature_linear(h)
        
        if self.with_viewdirs:
            viewdir = self.view_enc(viewdir)
            h = torch.cat([feature, viewdir, point_2d_feature_2], -1)
        else:
            h = torch.cat([feature, point_2d_feature_2], -1)

        h = self.views_linear(h)
        h = F.relu(h)
        rgb = self.rgb_linear(h)
        
        if self.mesh_animation:
            raw = torch.cat([world_src_pts, bweights, rgb, alpha], dim=-1)
            return raw
        if self.extract_mesh:
            raw = torch.cat([rgb, alpha], -1).unsqueeze(0)
            return raw

        if self.human_sample:
            ret_smpl_query_pts, ret_smpl_src_pts = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            ret_smpl_query_pts[pts_mask==1], ret_smpl_src_pts[pts_mask==1] = smpl_query_pts, smpl_src_pts
            ret_correction, ret_correction_ = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            all_occ_normal, all_nearest_smpl_normal = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            if self.correction_field:
                ret_correction[pts_mask==1] = correction
                ret_correction_[pts_mask==1] = correction_

            raw = torch.zeros(zero_pts.shape[0],4).cuda().float()
            raw[pts_mask==1] = torch.cat([rgb, alpha], -1)
            raw[pts_mask==0] = -80 # softplus and sigmoid is outside
            raw = torch.cat([raw, torch.unsqueeze(pts_mask, -1), ret_correction, ret_correction_, ret_smpl_query_pts, ret_smpl_src_pts], -1)

            if self.smooth_loss and self.training:
                intv_flag = (int(sp_input['global_step'].item()) % int(sp_input['smooth_interval'].item()))==0
                if intv_flag:
                    # TODO wide_sigmoid(alpha)
                    # occ_normal = torch.autograd.grad(alpha, [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = torch.autograd.grad(wide_sigmoid(alpha), [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = occ_normal / (torch.norm(occ_normal, dim=-1, keepdim=True) + 1e-8)
                    all_occ_normal[pts_mask==1] = occ_normal
                    all_nearest_smpl_normal[pts_mask==1] = nearest_smpl_normal
                    raw = torch.cat([raw, all_occ_normal, all_nearest_smpl_normal], -1)

                    del x, h, point_2d_feature, point_2d_feature_0, fused_feature
                    del feature, point_3d_feature_0, canonical_pts
                    del pts_mask, sp_input, tp_input, world_query_pts, viewdir, occ_normal
        else:
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
            pts_mask = torch.unsqueeze(pts_mask, -1)
            raw = torch.cat([rgb, alpha, pts_mask, correction], -1)
        
        return raw.unsqueeze(0)


class CoarseBatch(nn.Module):
    def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False, 
                smooth_loss=False, num_instances=1, mean_shape=1, correction_field=1, skinning_field=1, data_set_type="H36M_B", append_rgb="False"):
        super(CorrectionBatch, self).__init__()
        # self.encoder_3d = SparseConvNet(num_layers=2)
        self.encoder_2d = SpatialEncoder(num_layers=2)# no smooth loss
        # self.encoder_2d = ImageViewEncoder(num_layers=2)# smooth loss
        # self.forward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        # self.backward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        # self.forward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        # self.backward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        self.use_agg = use_agg
        self.with_viewdirs = with_viewdirs
        self.pos_enc = PositionalEncoding(num_freqs=6)
        self.view_enc = PositionalEncoding(num_freqs=4)

        male_smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl')
        female_smpl_path = os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')
        neutral_smpl_path = os.path.join('assets', 'SMPL_NEUTRAL.pkl')
        self.SMPL_MALE = SMPL_to_tensor(read_pickle(male_smpl_path))
        self.SMPL_FEMALE = SMPL_to_tensor(read_pickle(female_smpl_path))
        self.SMPL_NEU = SMPL_to_tensor(read_pickle(neutral_smpl_path))

        smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl') if data_set_type[0]=="T" else os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')
        self.data_set_type = data_set_type
        self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(smpl_path))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')))
        # self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')))
        self.faces = self.SMPL_NEUTRAL['f'] #.astype(np.int32)
        self.smpl_pts_normal = None #compute_normal(t_vertices, faces)
        W = 256
        self.image_shape = torch.zeros((2,))

        # xc + f3d + f2d
        self.actvn = nn.ReLU()
        self.skips = [4]
        nerf_input_ch = (39+128) if not append_rgb else (39+128+27)#if smooth_loss else (39+32+128)
        self.pts_linears = nn.ModuleList(
            [nn.Linear(nerf_input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + nerf_input_ch, W) for i in range(7)])
        self.alpha_linear = nn.Linear(W, 1)
        self.feature_linear = nn.Linear(W, W)
        self.rgb_linear = nn.Linear(W//2, 3)
        self.extract_mesh = False
        self.mesh_animation = False
        self.human_sample = human_sample
        self.density_loss = density_loss
        self.use_f2d = use_f2d
        self.use_trans = use_trans
        self.smooth_loss = smooth_loss
        self.mean_shape = mean_shape
        self.correction_field = correction_field
        self.skinning_field = skinning_field
        self.append_rgb = append_rgb

        nerf_input_ch_2 = 384 if not append_rgb else (128+256+27) # 128 fused feature + 256 alpha feature + rgb code 
        self.views_linear = nn.Linear((nerf_input_ch_2+27) if self.with_viewdirs else nerf_input_ch_2, W//2)
        self.transformer = None if not use_trans else Transformer(128 if not append_rgb else (128+27))
        self.latent_codes = nn.Embedding(num_instances, 128)
        self.extract_mesh = False
        nn.init.normal_(self.latent_codes.weight, mean=0, std=0.01)

    def normalize_pts(self, source_pts, bounds):
        # bounds 2 * 3
        # source_pts N * 3
        mean_plus = 0.5 * (bounds[0] + bounds[1])
        mean_plus = mean_plus
        mean_sub = 0.5 * (bounds[1] - bounds[0])
        mean_sub = mean_sub

        normalized_pts = ((source_pts - mean_plus) / mean_sub)
        return normalized_pts

    def projection(self, query_pts, R, T, K):
        # https://github.com/pengsida/if-nerf/blob/492e765423fcbfb9aeca345286a49074cc4e5a90/lib/utils/render_utils.py#L45
        RT = torch.cat([R, T], 2)
        xyz = torch.repeat_interleave(query_pts.unsqueeze(dim=0), repeats=RT.shape[0], dim=0)
        xyz = torch.bmm(xyz.float(), RT[:, :, :3].transpose(1, 2).float()) + RT[:, :, 3:].transpose(1, 2).float()
        xyz = torch.bmm(xyz, K.transpose(1, 2).float())
        xy = xyz[:, :, :2] / (xyz[:, :, 2:] + 1e-5)
        
        return xy
    
    def encode_images(self, images):
        self.images = images
        self.image_shape[0] = images.shape[-1]
        self.image_shape[1] = images.shape[-2]

        self.encoder_2d(images)

    def big_pose_params(self, params):
        big_pose_params = copy.deepcopy(params)
        big_pose_params['poses'] = torch.zeros((1,72))
        big_pose_params['poses'][0, 5] = 45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 8] = -45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 23] = -30/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 26] = 30/180*torch.tensor(np.pi)

        return big_pose_params

    def coarse_deform_target2c(self, params, vertices,  query_pts):

        # joints transformation
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)        
        J = torch.hstack((joints, torch.ones([joints.shape[0], 1])))
        # self.c_joints = joints # smpl space
        # self.t_A = A # target to canonical space transformation
        # self.t_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]

        # transform smpl vertices from world space to smpl space
        smpl_pts = torch.mm((vertices - Th), R)
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # From smpl space target pose to smpl space canonical pose
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        can_pts = torch.sum(R_inv * can_pts[:, None], dim=2)
        
        if self.mean_shape:
            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            can_pts = can_pts - pose_offsets[vert_ids.squeeze(0).reshape(-1)]

            # To mean shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            # shape_offset = shapedirs.dot(torch.reshape(params['shapes'], (10,)))
            # shape_offset = torch.einsum('ijk, k -> ij', shapedirs, torch.reshape(params['shapes'], (10,)))
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            can_pts = can_pts - shape_offset[vert_ids.squeeze(0).reshape(-1)]

        # From T To Big Pose        
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        # J = torch.hstack((self.c_joints, torch.ones([self.c_joints.shape[0], 1])))
        # self.c_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * can_pts[:, None], dim=2)
        can_pts = can_pts + A[:, :3, 3]

        return can_pts 

    def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):
        # Find nearest smpl vertex
        smpl_pts = t_vertices
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # add weights_correction, normalize weights
        # bweights = F.softmax(bweights + 0.2*weights_correction, dim=1)
        bweights = bweights + 0.2 * weights_correction
        bweights = bweights / torch.sum(bweights, dim=-1, keepdim=True)

        ### From Big To T Pose
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        query_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        query_pts = torch.sum(R_inv * query_pts[:, None], dim=2)

        if self.mean_shape:
            # From mean shape to normal shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            query_pts = query_pts + shape_offset[vert_ids.squeeze(0).reshape(-1)]

            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            query_pts = query_pts + pose_offsets[vert_ids.squeeze(0).reshape(-1)]

        # get tar_pts, smpl space source pose
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)
        self.s_A = A
        A = torch.mm(bweights, self.s_A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * query_pts[:, None], dim=2)
        smpl_src_pts = can_pts + A[:, :3, 3]
        
        # transform points from the smpl space to the world space
        R_inv = torch.inverse(R)
        world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th
        
        return smpl_src_pts, world_src_pts, bweights

    def set_extract_mesh(self, flag):
        self.extract_mesh = flag

    def forward(self, sp_input, tp_input, world_query_pts, viewdir):

        ## translate query points from world space target pose to smpl space target pose
        world_query_pts = world_query_pts.squeeze(0)
        viewdir = viewdir.squeeze(0)
        sp_input, tp_input = sequeeze_0(sp_input, tp_input)
        R = tp_input['params']['R']
        Th = tp_input['params']['Th'] #.astype(np.float32)
        smpl_query_pts = torch.mm(world_query_pts - Th, R)
        
        if sp_input["gender"] == "M":
            self.SMPL_NEUTRAL = self.SMPL_MALE
        elif sp_input["gender"] == "F":
            self.SMPL_NEUTRAL = self.SMPL_FEMALE
        else:
            self.SMPL_NEUTRAL = self.SMPL_NEU

        ## encode images
        img_all = sp_input['img_all']
        self.encode_images(img_all)
        
        ## human region sample
        if self.human_sample:
            tar_smpl_pts = tp_input['vertices']
            tar_smpl_pts = torch.mm(tar_smpl_pts - Th, R)
            distance, _, _ = knn_points(smpl_query_pts.unsqueeze(0).float(), tar_smpl_pts.unsqueeze(0).float(), K=1)
            distance = distance.squeeze(0).view(-1)
            pts_mask = torch.zeros_like(smpl_query_pts[:,0]).cuda().int()
            threshold = 0.05 ** 2
            pts_mask[distance < threshold] = 1
            
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            smpl_query_pts = smpl_query_pts[pts_mask==1]
            viewdir = viewdir[pts_mask==1]
        else:
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
        
        ## More Accurate canonical_pts
        canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts).requires_grad_()

        ## Extract Mesh
        if self.extract_mesh or self.mesh_animation:
            canonical_pts = world_query_pts
            pts_mask = torch.ones_like(canonical_pts[:,0]).cuda().int()

        smpl_src_pts, world_src_pts, bweights = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], canonical_pts, weights_correction=0.)

        if self.smooth_loss and self.training:
            _, vert_ids, _ = knn_points(canonical_pts.unsqueeze(0).float(), sp_input['t_vertices'].unsqueeze(0).float(), K=1)
            if self.smpl_pts_normal==None:
                self.smpl_pts_normal = compute_normal(sp_input['t_vertices'], self.faces)
            nearest_smpl_normal = self.smpl_pts_normal[vert_ids.squeeze(0)].reshape(-1, 3)

        ## Get multi-view pixel-aligned feature four view
        if self.correction_field or tp_input['pose_index']!=sp_input['pose_index']:
            uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        else:
            uv = self.projection(world_query_pts[pts_mask==1], sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        point_2d_feature_0 = self.encoder_2d.index(uv, self.image_shape)
        point_2d_feature_0 = point_2d_feature_0.transpose(1,2)
        
        if self.append_rgb:
            img = sp_input['img_all']
            uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
            samples = grid_sample(img, uv_)
            samples = samples.squeeze(-1).transpose(1,2)
            sh = samples.shape
            samples = self.view_enc(samples.reshape(-1,3)).reshape(sh[0],sh[1],27)
            point_2d_feature_0 = torch.cat((point_2d_feature_0, samples), dim=-1)

        ## Transformer Fusion of multiview Feature
        if self.use_trans and point_2d_feature_0.shape[1]!=0:
            point_2d_feature = point_2d_feature_0.transpose(0,1)
            point_2d_feature = self.transformer(point_2d_feature)
            point_2d_feature_1 = point_2d_feature[:,0,:]
            point_2d_feature_2 = point_2d_feature[:,1,:]
        else:
            point_2d_feature = torch.mean(point_2d_feature_0, dim=0)
            point_2d_feature_1 = point_2d_feature
            point_2d_feature_2 = point_2d_feature

        # Run in Nerf
        x = self.pos_enc(canonical_pts)
        x = torch.cat((x, point_2d_feature_1), dim=1)
        h = x
        for i, l in enumerate(self.pts_linears):
            h = self.pts_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([x, h], -1)

        alpha = self.alpha_linear(h)
        feature = self.feature_linear(h)
        
        if self.with_viewdirs:
            viewdir = self.view_enc(viewdir)
            h = torch.cat([feature, viewdir, point_2d_feature_2], -1)
        else:
            h = torch.cat([feature, point_2d_feature_2], -1)

        h = self.views_linear(h)
        h = F.relu(h)
        rgb = self.rgb_linear(h)
        
        if self.mesh_animation:
            raw = torch.cat([world_src_pts, bweights, rgb, alpha], dim=-1)
            return raw
        if self.extract_mesh:
            raw = torch.cat([rgb, alpha], -1).unsqueeze(0)
            return raw

        if self.human_sample:
            ret_smpl_query_pts, ret_smpl_src_pts = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            ret_smpl_query_pts[pts_mask==1], ret_smpl_src_pts[pts_mask==1] = smpl_query_pts, smpl_src_pts
            ret_correction, ret_correction_ = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            all_occ_normal, all_nearest_smpl_normal = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()

            raw = torch.zeros(zero_pts.shape[0],4).cuda().float()
            raw[pts_mask==1] = torch.cat([rgb, alpha], -1)
            raw[pts_mask==0] = -80 # softplus and sigmoid is outside
            raw = torch.cat([raw, torch.unsqueeze(pts_mask, -1), ret_correction, ret_correction_, ret_smpl_query_pts, ret_smpl_src_pts], -1)

            if self.smooth_loss and self.training:
                intv_flag = (int(sp_input['global_step'].item()) % int(sp_input['smooth_interval'].item()))==0
                if intv_flag:
                    # TODO wide_sigmoid(alpha)
                    # occ_normal = torch.autograd.grad(alpha, [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = torch.autograd.grad(wide_sigmoid(alpha), [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = occ_normal / (torch.norm(occ_normal, dim=-1, keepdim=True) + 1e-8)
                    all_occ_normal[pts_mask==1] = occ_normal
                    all_nearest_smpl_normal[pts_mask==1] = nearest_smpl_normal
                    raw = torch.cat([raw, all_occ_normal, all_nearest_smpl_normal], -1)

                    del x, h, point_2d_feature, feature, canonical_pts, point_2d_feature_0
                    del pts_mask, sp_input, tp_input, world_query_pts, viewdir, occ_normal
        else:
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
            pts_mask = torch.unsqueeze(pts_mask, -1)
            raw = torch.cat([rgb, alpha, pts_mask, correction], -1)
        
        return raw.unsqueeze(0)


class CorrectionBatchBlend(nn.Module):
    def __init__(self, use_agg=False, human_sample=True, density_loss=False, with_viewdirs=False, use_f2d=True, use_trans=False, 
                smooth_loss=False, num_instances=1, mean_shape=1, correction_field=1, skinning_field=1, data_set_type="H36M_B", append_rgb="False"):
        super(CorrectionBatchBlend, self).__init__()
        self.encoder_3d = SparseConvNet(num_layers=2)
        self.encoder_2d = SpatialEncoder(num_layers=2)# no smooth loss
        # self.encoder_2d = ImageViewEncoder(num_layers=2)# smooth loss
        self.forward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        self.backward_deform = DeformField(D=4, input_ch=(271), output_ch=3, deform_type='correction')
        # self.forward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        # self.backward_deform = DeformField(D=4, input_ch=(39 + 128), output_ch=24, deform_type='weights')
        self.use_agg = use_agg
        self.with_viewdirs = with_viewdirs
        self.pos_enc = PositionalEncoding(num_freqs=6)
        self.view_enc = PositionalEncoding(num_freqs=4)

        male_smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl')
        female_smpl_path = os.path.join('assets', 'basicmodel_f_lbs_10_207_0_v1.0.0.pkl')
        neutral_smpl_path = os.path.join('assets', 'SMPL_NEUTRAL.pkl')
        self.SMPL_MALE = SMPL_to_tensor(read_pickle(male_smpl_path))
        self.SMPL_FEMALE = SMPL_to_tensor(read_pickle(female_smpl_path))
        self.SMPL_NEU = SMPL_to_tensor(read_pickle(neutral_smpl_path))

        smpl_path = os.path.join('assets', 'basicmodel_m_lbs_10_207_0_v1.0.0.pkl') if data_set_type[0]=="T" else os.path.join('msra_h36m', 'smplx/smpl/SMPL_NEUTRAL.pkl')
        self.data_set_type = data_set_type
        self.SMPL_NEUTRAL = SMPL_to_tensor(read_pickle(smpl_path))

        self.faces = self.SMPL_NEUTRAL['f'] #.astype(np.int32)
        self.smpl_pts_normal = None #compute_normal(t_vertices, faces)
        W = 256
        self.image_shape = torch.zeros((2,))

        # xc + f3d + f2d
        self.actvn = nn.ReLU()
        self.skips = [4]
        nerf_input_ch = (39+128) if not append_rgb else (39+128+27)#if smooth_loss else (39+32+128)
        self.pts_linears = nn.ModuleList(
            [nn.Linear(nerf_input_ch, W)] + [nn.Linear(W, W) if i not in self.skips else nn.Linear(W + nerf_input_ch, W) for i in range(7)])
        self.alpha_linear = nn.Linear(W, 1)
        self.feature_linear = nn.Linear(W, W)
        self.rgb_linear = nn.Linear(W//2, 4)
        self.extract_mesh = False
        self.mesh_animation = False
        self.human_sample = human_sample
        self.density_loss = density_loss
        self.use_f2d = use_f2d
        self.use_trans = use_trans
        self.smooth_loss = smooth_loss
        self.mean_shape = mean_shape
        self.correction_field = correction_field
        self.skinning_field = skinning_field
        self.append_rgb = append_rgb

        nerf_input_ch_2 = 384 if not append_rgb else (128+256+27) # 128 fused feature + 256 alpha feature + rgb code 
        self.views_linear = nn.Linear((nerf_input_ch_2+27) if self.with_viewdirs else nerf_input_ch_2, W//2)
        self.transformer = None if not use_trans else Transformer(128 if not append_rgb else (128+27))
        self.latent_codes = nn.Embedding(num_instances, 128)
        self.extract_mesh = False
        nn.init.normal_(self.latent_codes.weight, mean=0, std=0.01)

    def normalize_pts(self, source_pts, bounds):
        # bounds 2 * 3
        # source_pts N * 3
        mean_plus = 0.5 * (bounds[0] + bounds[1])
        mean_plus = mean_plus
        mean_sub = 0.5 * (bounds[1] - bounds[0])
        mean_sub = mean_sub

        normalized_pts = ((source_pts - mean_plus) / mean_sub)
        return normalized_pts

    def projection(self, query_pts, R, T, K):
        # https://github.com/pengsida/if-nerf/blob/492e765423fcbfb9aeca345286a49074cc4e5a90/lib/utils/render_utils.py#L45
        RT = torch.cat([R, T], 2)
        xyz = torch.repeat_interleave(query_pts.unsqueeze(dim=0), repeats=RT.shape[0], dim=0)
        xyz = torch.bmm(xyz.float(), RT[:, :, :3].transpose(1, 2).float()) + RT[:, :, 3:].transpose(1, 2).float()
        xyz = torch.bmm(xyz, K.transpose(1, 2).float())
        xy = xyz[:, :, :2] / (xyz[:, :, 2:] + 1e-5)
        
        return xy
    
    def encode_images(self, images):
        self.images = images
        self.image_shape[0] = images.shape[-1]
        self.image_shape[1] = images.shape[-2]

        self.encoder_2d(images)

    def big_pose_params(self, params):
        big_pose_params = copy.deepcopy(params)
        big_pose_params['poses'] = torch.zeros((1,72))
        big_pose_params['poses'][0, 5] = 45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 8] = -45/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 23] = -30/180*torch.tensor(np.pi)
        big_pose_params['poses'][0, 26] = 30/180*torch.tensor(np.pi)

        return big_pose_params

    def coarse_deform_target2c(self, params, vertices,  query_pts):

        # joints transformation
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)        
        J = torch.hstack((joints, torch.ones([joints.shape[0], 1])))
        # self.c_joints = joints # smpl space
        # self.t_A = A # target to canonical space transformation
        # self.t_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]

        # transform smpl vertices from world space to smpl space
        smpl_pts = torch.mm((vertices - Th), R)
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # From smpl space target pose to smpl space canonical pose
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        can_pts = torch.sum(R_inv * can_pts[:, None], dim=2)
        
        if self.mean_shape:
            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            can_pts = can_pts - pose_offsets[vert_ids.squeeze(0).reshape(-1)]

            # To mean shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            # shape_offset = shapedirs.dot(torch.reshape(params['shapes'], (10,)))
            # shape_offset = torch.einsum('ijk, k -> ij', shapedirs, torch.reshape(params['shapes'], (10,)))
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            can_pts = can_pts - shape_offset[vert_ids.squeeze(0).reshape(-1)]

        # From T To Big Pose        
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        # J = torch.hstack((self.c_joints, torch.ones([self.c_joints.shape[0], 1])))
        # self.c_joints = torch.matmul(A, J.reshape([-1, 4, 1])).reshape([-1, 4])[:, :3]
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * can_pts[:, None], dim=2)
        can_pts = can_pts + A[:, :3, 3]

        return can_pts 

    def coarse_deform_c2source(self, params, t_vertices, query_pts, weights_correction):
        # Find nearest smpl vertex
        smpl_pts = t_vertices
        _, vert_ids, _ = knn_points(query_pts.unsqueeze(0).float(), smpl_pts.unsqueeze(0).float(), K=1)
        bweights = self.SMPL_NEUTRAL['weights'][vert_ids.squeeze(0)].view(-1,24).cuda()

        # add weights_correction, normalize weights
        # bweights = F.softmax(bweights + 0.2*weights_correction, dim=1)
        bweights = bweights + 0.2 * weights_correction
        bweights = bweights / torch.sum(bweights, dim=-1, keepdim=True)

        ### From Big To T Pose
        big_pose_params = self.big_pose_params(params)
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, big_pose_params)
        A = torch.mm(bweights, A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        query_pts = query_pts - A[:, :3, 3]
        R_inv = torch.inverse(A[:, :3, :3].float())
        query_pts = torch.sum(R_inv * query_pts[:, None], dim=2)

        if self.mean_shape:
            # From mean shape to normal shape
            shapedirs = self.SMPL_NEUTRAL['shapedirs'].cuda()
            shape_offset = (shapedirs * torch.reshape(params['shapes'].cuda(), (10,))).sum(dim=-1)
            query_pts = query_pts + shape_offset[vert_ids.squeeze(0).reshape(-1)]

            posedirs = self.SMPL_NEUTRAL['posedirs'].cuda().float()
            pose_ = params['poses']
            ident = torch.eye(3).cuda().float()
            batch_size = 1
            rot_mats = batch_rodrigues(pose_.view(-1, 3)).view([batch_size, -1, 3, 3])
            pose_feature = (rot_mats[:, 1:, :, :] - ident).view([batch_size, -1]).cuda()
            pose_offsets = torch.matmul(pose_feature, posedirs.view(6890*3, -1).transpose(1,0)).view(-1, 3)
            query_pts = query_pts + pose_offsets[vert_ids.squeeze(0).reshape(-1)]

        # get tar_pts, smpl space source pose
        A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, params)
        self.s_A = A
        A = torch.mm(bweights, self.s_A.reshape(24, -1))
        A = torch.reshape(A, (-1, 4, 4))
        can_pts = torch.sum(A[:, :3, :3] * query_pts[:, None], dim=2)
        smpl_src_pts = can_pts + A[:, :3, 3]
        
        # transform points from the smpl space to the world space
        R_inv = torch.inverse(R)
        world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th
        
        return smpl_src_pts, world_src_pts, bweights

    def forward_fusion(self, query_pts, params, embedding, point_3d_feature, agg_feature=None):
        """(x, x-joint_t, target_f_3d) → deformation Net 1 → X_c""" #39+72+128+32
        query_pts_code = self.pos_enc(query_pts)
        poses = torch.repeat_interleave(params['poses'].reshape(1,72), query_pts_code.shape[0], 0)
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), query_pts_code.shape[0], 0)
        if agg_feature != None:
            f = torch.cat([query_pts_code, poses, embedding, point_3d_feature, agg_feature], dim=-1) # 39 + 72 + 32 + 128 = 271
        else:
            f = torch.cat([query_pts_code, poses, embedding, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def backward_fusion(self, query_pts, params, embedding, point_3d_feature):
        """(X_c, Pose_s, shape, x-joint_c, f_agg) → deformation Net 2 → X_s"""
        query_pts_code = self.pos_enc(query_pts) # torch.Size([16384, 3])
        poses = torch.repeat_interleave(params['poses'].reshape(1,72), query_pts_code.shape[0], 0)
        embedding = torch.repeat_interleave(embedding.unsqueeze(0), query_pts_code.shape[0], dim=0) # torch.Size([16384, 24, 3])
        f = torch.cat([query_pts_code, poses, embedding, point_3d_feature], dim=-1) # torch.Size([16384, 669])
        return f
    
    def prepare_spconv(self, tp_input, t_vertices=False):
        pre = 't_' if t_vertices else ''
        xyz_input_feature = tp_input[pre+'feature'] #torch.from_numpy(tp_input['feature']).cuda().float()
        voxel_integer_coord = tp_input[pre+'coord'].int() #torch.from_numpy(tp_input['coord']).cuda().float().int()
        coord = torch.zeros_like(voxel_integer_coord[:,0:1])
        voxel_integer_coord = torch.cat((coord, voxel_integer_coord), dim=1)
        out_sh = tp_input[pre+'out_sh'].cpu().numpy().astype(np.int32)
        batch_size = 1
        sparse_smpl_vertices = spconv.SparseConvTensor(xyz_input_feature, voxel_integer_coord, out_sh, batch_size)

        return sparse_smpl_vertices

    def set_extract_mesh(self, flag):
        self.extract_mesh = flag

    def forward(self, sp_input, tp_input, world_query_pts, viewdir):

        ## translate query points from world space target pose to smpl space target pose
        world_query_pts = world_query_pts.squeeze(0)
        viewdir = viewdir.squeeze(0)
        sp_input, tp_input = sequeeze_0(sp_input, tp_input)
        R = tp_input['params']['R']
        Th = tp_input['params']['Th'] #.astype(np.float32)
        smpl_query_pts = torch.mm(world_query_pts - Th, R)
        
        ## encode images
        img_all = sp_input['img_all']
        self.encode_images(img_all)
        
        ## human region sample
        if self.human_sample:
            tar_smpl_pts = tp_input['vertices']
            tar_smpl_pts = torch.mm(tar_smpl_pts - Th, R)
            distance, _, _ = knn_points(smpl_query_pts.unsqueeze(0).float(), tar_smpl_pts.unsqueeze(0).float(), K=1)
            distance = distance.squeeze(0).view(-1)
            pts_mask = torch.zeros_like(smpl_query_pts[:,0]).cuda().int()
            threshold = 0.05 ** 2
            pts_mask[distance < threshold] = 1
            
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            smpl_query_pts = smpl_query_pts[pts_mask==1]
            viewdir = viewdir[pts_mask==1]
        else:
            zero_pts = torch.zeros(smpl_query_pts.shape).cuda().float()
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
        
        ## More Accurate canonical_pts
        if self.correction_field:
            sparse_smpl_vertices = self.prepare_spconv(tp_input, t_vertices=False)
            normalized_source_pts = self.normalize_pts(smpl_query_pts, tp_input['bounds'])
            point_3d_feature_0 = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts.detach())
            
            instance_idx = tp_input['instance_idx'].long()
            embedding = self.latent_codes(instance_idx)
            fused_feature = self.forward_fusion(smpl_query_pts, tp_input['params'], embedding, point_3d_feature_0)
            
            correction = self.forward_deform(fused_feature.float())
            # canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts + correction)
            canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts)
            canonical_pts = canonical_pts + correction
        else:
            canonical_pts = self.coarse_deform_target2c(tp_input['params'], tp_input['vertices'], smpl_query_pts).requires_grad_()

        ## Extract Mesh
        if self.extract_mesh or self.mesh_animation:
            canonical_pts = world_query_pts
            pts_mask = torch.ones_like(canonical_pts[:,0]).cuda().int()

        smpl_src_pts, world_src_pts, bweights = self.coarse_deform_c2source(sp_input['params'], sp_input['t_vertices'], canonical_pts, weights_correction=0.)
        
        if self.correction_field:
            sparse_smpl_vertices = self.prepare_spconv(sp_input, t_vertices=False)
            normalized_source_pts = self.normalize_pts(smpl_src_pts, sp_input['bounds'])
            point_3d_feature_0 = self.encoder_3d(sparse_smpl_vertices, normalized_source_pts.detach())
            
            instance_idx = sp_input['instance_idx'].long()
            embedding = self.latent_codes(instance_idx)
            fused_feature = self.backward_fusion(smpl_src_pts.clone(), sp_input['params'], embedding, point_3d_feature_0)
            
            correction_ = self.backward_deform(fused_feature.float())
            smpl_src_pts = smpl_src_pts + correction_
            A, R, Th, joints = get_transform_params_torch(self.SMPL_NEUTRAL, sp_input['params'])
            R_inv = torch.inverse(R)
            world_src_pts = torch.mm(smpl_src_pts, R_inv) + Th

        if self.smooth_loss and self.training:
            _, vert_ids, _ = knn_points(canonical_pts.unsqueeze(0).float(), sp_input['t_vertices'].unsqueeze(0).float(), K=1)
            if self.smpl_pts_normal==None:
                self.smpl_pts_normal = compute_normal(sp_input['t_vertices'], self.faces)
            nearest_smpl_normal = self.smpl_pts_normal[vert_ids.squeeze(0)].reshape(-1, 3)

        ## Get multi-view pixel-aligned feature four view
        if self.correction_field or tp_input['pose_index']!=sp_input['pose_index']:
            uv = self.projection(world_src_pts, sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        else:
            uv = self.projection(world_query_pts[pts_mask==1], sp_input['R_all'], sp_input['T_all'], sp_input['K_all'])
        point_2d_feature_0 = self.encoder_2d.index(uv, self.image_shape)
        point_2d_feature_0 = point_2d_feature_0.transpose(1,2)
        
        if self.append_rgb:
            img = sp_input['img_all']
            uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
            samples = grid_sample(img, uv_)
            samples = samples.squeeze(-1).transpose(1,2)
            sh = samples.shape
            samples = self.view_enc(samples.reshape(-1,3)).reshape(sh[0],sh[1],27)
            point_2d_feature_0 = torch.cat((point_2d_feature_0, samples), dim=-1)

        ## Transformer Fusion of multiview Feature
        if self.use_trans and point_2d_feature_0.shape[1]!=0:
            point_2d_feature = point_2d_feature_0.transpose(0,1)
            point_2d_feature = self.transformer(point_2d_feature)
            point_2d_feature_1 = point_2d_feature[:,0,:]
            point_2d_feature_2 = point_2d_feature[:,1,:]
        else:
            point_2d_feature = torch.mean(point_2d_feature_0, dim=0)
            point_2d_feature_1 = point_2d_feature
            point_2d_feature_2 = point_2d_feature

        # Run in Nerf
        x = self.pos_enc(canonical_pts)
        x = torch.cat((x, point_2d_feature_1), dim=1)
        h = x
        for i, l in enumerate(self.pts_linears):
            h = self.pts_linears[i](h)
            h = F.relu(h)
            if i in self.skips:
                h = torch.cat([x, h], -1)

        alpha = self.alpha_linear(h)
        feature = self.feature_linear(h)
        
        if self.with_viewdirs:
            viewdir = self.view_enc(viewdir)
            h = torch.cat([feature, viewdir, point_2d_feature_2], -1)
        else:
            h = torch.cat([feature, point_2d_feature_2], -1)

        h = self.views_linear(h)
        h = F.relu(h)
        # rgb = self.rgb_linear(h)
        rgb_weights = F.softmax(self.rgb_linear(h),dim=-1).unsqueeze(1)
        img = sp_input['img_all']
        uv_ = 2.0 * uv.unsqueeze(2).type(torch.float32)  / self.image_shape.clone().detach().to(uv.device) - 1.0
        samples = grid_sample(img, uv_)
        samples = samples.squeeze(-1).transpose(1,2).transpose(0,1)
        rgb = torch.bmm(rgb_weights, samples).squeeze(1)
        
        if self.mesh_animation:
            raw = torch.cat([world_src_pts, bweights, rgb, alpha], dim=-1)
            return raw
        if self.extract_mesh:
            raw = torch.cat([rgb, alpha], -1).unsqueeze(0)
            return raw

        if self.human_sample:
            ret_smpl_query_pts, ret_smpl_src_pts = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            ret_smpl_query_pts[pts_mask==1], ret_smpl_src_pts[pts_mask==1] = smpl_query_pts, smpl_src_pts
            ret_correction, ret_correction_ = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            all_occ_normal, all_nearest_smpl_normal = torch.zeros(zero_pts.shape).cuda().float(), torch.zeros(zero_pts.shape).cuda().float()
            if self.correction_field:
                ret_correction[pts_mask==1] = correction
                ret_correction_[pts_mask==1] = correction_

            raw = torch.zeros(zero_pts.shape[0],4).cuda().float()
            raw[pts_mask==1] = torch.cat([rgb, alpha], -1)
            raw[pts_mask==0] = -80 # softplus and sigmoid is outside
            raw = torch.cat([raw, torch.unsqueeze(pts_mask, -1), ret_correction, ret_correction_, ret_smpl_query_pts, ret_smpl_src_pts], -1)

            if self.smooth_loss and self.training:
                intv_flag = (int(sp_input['global_step'].item()) % int(sp_input['smooth_interval'].item()))==0
                if intv_flag:
                    # TODO wide_sigmoid(alpha)
                    # occ_normal = torch.autograd.grad(alpha, [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = torch.autograd.grad(wide_sigmoid(alpha), [canonical_pts], grad_outputs=torch.ones_like(alpha), create_graph=True)[0]
                    occ_normal = occ_normal / (torch.norm(occ_normal, dim=-1, keepdim=True) + 1e-8)
                    all_occ_normal[pts_mask==1] = occ_normal
                    all_nearest_smpl_normal[pts_mask==1] = nearest_smpl_normal
                    raw = torch.cat([raw, all_occ_normal, all_nearest_smpl_normal], -1)

                    del x, h, point_2d_feature, point_2d_feature_0
                    del feature, point_3d_feature_0, canonical_pts
                    del pts_mask, sp_input, tp_input, world_query_pts, viewdir, occ_normal
        else:
            pts_mask = torch.ones_like(smpl_query_pts[:,0]).cuda().int()
            pts_mask = torch.unsqueeze(pts_mask, -1)
            raw = torch.cat([rgb, alpha, pts_mask, correction], -1)
        
        return raw.unsqueeze(0)



```

## Source file: lib/encoder.py

{
  "path": "lib/encoder.py",
  "blob_sha": "7b302030ecc51a31a808f2cf475dc142de949ae3",
  "sha256": "27b4b12930ecf1fd107a4913f6f20b2e8e2321d7e12217f1480f3729fc192f11",
  "lines": 527,
  "bytes": 18525,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/gaoxiangjun/MPS-NeRF/blob/02a2842cff8bca4206d0cae9339fa15e9694de1f/lib/encoder.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def grid_sample(image, optical):"
    },
    {
      "line": 70,
      "text": "def __init__(self, backbone=\"resnet34\", pretrained=True, num_layers=4,"
    },
    {
      "line": 105,
      "text": "def index(self, uv, image_size=(200, 200)):"
    },
    {
      "line": 133,
      "text": "def forward(self, x):"
    },
    {
      "line": 191,
      "text": "def __init__(self, backbone=\"resnet34\", pretrained=True, num_layers=4,"
    },
    {
      "line": 225,
      "text": "def index(self, uv, image_size=(512, 512)):"
    },
    {
      "line": 256,
      "text": "def forward(self, x):"
    },
    {
      "line": 314,
      "text": "def __init__(self, backbone=\"resnet34\", pretrained=True, latent_size=512):"
    },
    {
      "line": 332,
      "text": "def index(self, uv, image_size=()):"
    },
    {
      "line": 340,
      "text": "def forward(self, x):"
    },
    {
      "line": 374,
      "text": "def __init__(self, num_layers=2):"
    },
    {
      "line": 394,
      "text": "def forward(self, x, point_normalied_coords):"
    },
    {
      "line": 455,
      "text": "def single_conv(in_channels, out_channels, indice_key=None):"
    },
    {
      "line": 467,
      "text": "def double_conv(in_channels, out_channels, indice_key=None):"
    },
    {
      "line": 491,
      "text": "def triple_conv(in_channels, out_channels, indice_key=None):"
    },
    {
      "line": 517,
      "text": "def stride_conv(in_channels, out_channels, indice_key=None):"
    }
  ]
}

```text

"""
Implements image encoders
"""
import torch
from torch import nn
import torch.nn.functional as F
import torchvision
import torch.autograd.profiler as profiler
import spconv.pytorch as spconv


def grid_sample(image, optical):
    N, C, IH, IW = image.shape
    _, H, W, _ = optical.shape

    ix = optical[..., 0]
    iy = optical[..., 1]

    ix = ((ix + 1) / 2) * (IW-1);
    iy = ((iy + 1) / 2) * (IH-1);
    with torch.no_grad():
        ix_nw = torch.floor(ix);
        iy_nw = torch.floor(iy);
        ix_ne = ix_nw + 1;
        iy_ne = iy_nw;
        ix_sw = ix_nw;
        iy_sw = iy_nw + 1;
        ix_se = ix_nw + 1;
        iy_se = iy_nw + 1;

    nw = (ix_se - ix)    * (iy_se - iy)
    ne = (ix    - ix_sw) * (iy_sw - iy)
    sw = (ix_ne - ix)    * (iy    - iy_ne)
    se = (ix    - ix_nw) * (iy    - iy_nw)
    
    with torch.no_grad():
        torch.clamp(ix_nw, 0, IW-1, out=ix_nw)
        torch.clamp(iy_nw, 0, IH-1, out=iy_nw)

        torch.clamp(ix_ne, 0, IW-1, out=ix_ne)
        torch.clamp(iy_ne, 0, IH-1, out=iy_ne)
 
        torch.clamp(ix_sw, 0, IW-1, out=ix_sw)
        torch.clamp(iy_sw, 0, IH-1, out=iy_sw)
 
        torch.clamp(ix_se, 0, IW-1, out=ix_se)
        torch.clamp(iy_se, 0, IH-1, out=iy_se)

    image = image.view(N, C, IH * IW)


    nw_val = torch.gather(image, 2, (iy_nw * IW + ix_nw).long().view(N, 1, H * W).repeat(1, C, 1))
    ne_val = torch.gather(image, 2, (iy_ne * IW + ix_ne).long().view(N, 1, H * W).repeat(1, C, 1))
    sw_val = torch.gather(image, 2, (iy_sw * IW + ix_sw).long().view(N, 1, H * W).repeat(1, C, 1))
    se_val = torch.gather(image, 2, (iy_se * IW + ix_se).long().view(N, 1, H * W).repeat(1, C, 1))

    out_val = (nw_val.view(N, C, H, W) * nw.view(N, 1, H, W) + 
               ne_val.view(N, C, H, W) * ne.view(N, 1, H, W) +
               sw_val.view(N, C, H, W) * sw.view(N, 1, H, W) +
               se_val.view(N, C, H, W) * se.view(N, 1, H, W))

    return out_val


class ImageViewEncoder(nn.Module):
    """
    2D (Spatial/Pixel-aligned/local) image encoder
    """

    def __init__(self, backbone="resnet34", pretrained=True, num_layers=4, 
                index_interp="bilinear",
                feature_scale=0.5, use_first_pool=False):
        """Initialization
        
        Args:
            backbone: backbone Backbone network.
            num_layers: number of resnet layers to use, 1-5
            index_interp: Interpolation to use for indexing
            feature_scale: factor to scale all latent by. Useful (<1) if image
                is extremely large, to fit in memory.
            use_first_pool: if false, skips first maxpool layer to avoid downscaling image
                features too much (ResNet only)
        """
        super(ImageViewEncoder, self).__init__()

        self.feature_scale = feature_scale
        self.use_first_pool = use_first_pool
        self.norm_type = nn.BatchNorm2d

        # print("Using torchvision", backbone, "encoder")
        self.model = getattr(torchvision.models, backbone)(pretrained=pretrained, norm_layer=self.norm_type)
        self.model.conv1 = nn.Conv2d(6, 64, kernel_size=7, stride=2, padding=3, bias=False)
        self.latent_size = [0, 64, 128, 256, 512, 1024][num_layers]

        self.num_layers = num_layers
        self.index_interp = index_interp
        self.latent = None
        # self.register_buffer("latent", torch.empty(1, 1, 1, 1), persistent=False)
        # self.register_buffer(
        #     "latent_scaling", torch.empty(2, dtype=torch.float32), persistent=False
        # )
        # self.latent (B, L, H, W)


    def index(self, uv, image_size=(200, 200)):
        """Get pixel-aligned image features at 2D image coordinates
        
        Args:
            :param uv (B, N, 2) image points (x,y)  # uv.shape torch.Size([4, 16384, 2])
            :param image_size image size, either (width, height) or single int.
                if not specified, assumes coords are in [-1, 1]
        
        Returns:
            :return (B, L, N) L is latent size
        """
        with profiler.record_function("encoder_index"):

            uv = uv.unsqueeze(2).type(torch.float32)  # (B, N, 1, 2)
            uv = 2.0 * uv / torch.tensor(image_size).to(uv.device) - 1.0

            # samples = F.grid_sample(
            #     self.latent.float(), # torch.Size([4, 512, 501, 500]) (B, C, H, W)
            #     uv,
            #     align_corners=True,
            #     mode="bilinear",
            #     padding_mode='zeros', 
            #     )# torch.Size([4, 512, 16384, 1])
            samples = grid_sample(self.latent.float(), uv)

            return samples[:, :, :, 0]  # [4, 512, 16384] (B, C, N)


    def forward(self, x):
        """For extracting ResNet's features.
        
            :param x image (B, C, H, W)
            :return latent (B, latent_size, H, W)
        """
        if self.feature_scale != 1.0:
            x = F.interpolate(
                x,
                scale_factor=self.feature_scale,
                mode="bilinear" if self.feature_scale > 1.0 else "area",
                align_corners=True if self.feature_scale > 1.0 else None,
                recompute_scale_factor=True,
            )
        # x = x.to(device=self.latent.device)

        x = self.model.conv1(x)
        x = self.model.bn1(x)
        x = self.model.relu(x)

        latents = [x]
        if self.num_layers > 1:
            if self.use_first_pool:
                x = self.model.maxpool(x)
            x = self.model.layer1(x)
            latents.append(x)
        if self.num_layers > 2:
            x = self.model.layer2(x)
            latents.append(x)
        if self.num_layers > 3:
            x = self.model.layer3(x)
            latents.append(x)
        if self.num_layers > 4:
            x = self.model.layer4(x)
            latents.append(x)   # x.shape  torch.Size([4, 256, 63, 63])

        self.latents = latents
        align_corners = None if self.index_interp == "nearest " else True
        latent_sz = latents[0].shape[-2:]
        for i in range(len(latents)):
            latents[i] = F.interpolate(
                latents[i],
                latent_sz,
                mode="bilinear",
                align_corners=align_corners,
            ) # torch.Size([4, 64, 501, 500])

        # torch.Size([4, 512, 501, 500])
        self.latent = torch.cat(latents, dim=1) 

        return self.latent


class SpatialEncoder(nn.Module):
    """
    2D (Spatial/Pixel-aligned/local) image encoder
    """

    def __init__(self, backbone="resnet34", pretrained=True, num_layers=4, 
                index_interp="bilinear",
                feature_scale=0.5, use_first_pool=False):
        """Initialization
        
        Args:
            backbone: backbone Backbone network.
            num_layers: number of resnet layers to use, 1-5
            index_interp: Interpolation to use for indexing
            feature_scale: factor to scale all latent by. Useful (<1) if image
                is extremely large, to fit in memory.
            use_first_pool: if false, skips first maxpool layer to avoid downscaling image
                features too much (ResNet only)
        """
        super().__init__()

        self.feature_scale = feature_scale
        self.use_first_pool = use_first_pool
        self.norm_type = nn.BatchNorm2d

        # print("Using torchvision", backbone, "encoder")
        self.model = getattr(torchvision.models, backbone)(pretrained=pretrained, norm_layer=self.norm_type)
        self.latent_size = [0, 64, 128, 256, 512, 1024][num_layers]

        self.num_layers = num_layers
        self.index_interp = index_interp
        self.latent = None
        # self.register_buffer("latent", torch.empty(1, 1, 1, 1), persistent=False)
        # self.register_buffer(
        #     "latent_scaling", torch.empty(2, dtype=torch.float32), persistent=False
        # )
        # self.latent (B, L, H, W)


    def index(self, uv, image_size=(512, 512)):
        """Get pixel-aligned image features at 2D image coordinates
        
        Args:
            :param uv (B, N, 2) image points (x,y)  # uv.shape torch.Size([4, 16384, 2])
            :param image_size image size, either (width, height) or single int.
                if not specified, assumes coords are in [-1, 1]
        
        Returns:
            :return (B, L, N) L is latent size
        """
        with profiler.record_function("encoder_index"):

            uv = uv.unsqueeze(2).type(torch.float32)  # (B, N, 1, 2)
            # uv = 2.0 * uv / image_size.cuda().float() - 1.0
            uv = 2.0 * uv / image_size.clone().detach().to(uv.device) - 1.0 #torch.tensor(image_size).to(uv.device) - 1.0
            # uv = torch.flip(uv, (-1,))
            # uv[:,:,:,0] = 2 * uv[:,:,:,0] / image_size[0] - 1.
            # uv[:,:,:,1] = 2 * uv[:,:,:,1] / image_size[1] - 1.
            samples = grid_sample(self.latent.float(), uv)
            # samples = F.grid_sample(
            #     self.latent.float(), # torch.Size([4, 512, 501, 500]) (B, C, H, W)
            #     uv,
            #     align_corners=True,
            #     mode="bilinear",
            #     padding_mode='zeros', 
            #     )# torch.Size([4, 512, 16384, 1])

            return samples[:, :, :, 0]  # [4, 512, 16384] (B, C, N)


    def forward(self, x):
        """For extracting ResNet's features.
        
            :param x image (B, C, H, W)
            :return latent (B, latent_size, H, W)
        """
        if self.feature_scale != 1.0:
            x = F.interpolate(
                x,
                scale_factor=self.feature_scale,
                mode="bilinear" if self.feature_scale > 1.0 else "area",
                align_corners=True if self.feature_scale > 1.0 else None,
                recompute_scale_factor=True,
            )
        # x = x.to(device=self.latent.device)

        x = self.model.conv1(x)
        x = self.model.bn1(x)
        x = self.model.relu(x)

        latents = [x]
        if self.num_layers > 1:
            if self.use_first_pool:
                x = self.model.maxpool(x)
            x = self.model.layer1(x)
            latents.append(x)
        if self.num_layers > 2:
            x = self.model.layer2(x)
            latents.append(x)
        if self.num_layers > 3:
            x = self.model.layer3(x)
            latents.append(x)
        if self.num_layers > 4:
            x = self.model.layer4(x)
            latents.append(x)   # x.shape  torch.Size([4, 256, 63, 63])

        self.latents = latents
        align_corners = None if self.index_interp == "nearest " else True
        latent_sz = latents[0].shape[-2:]
        for i in range(len(latents)):
            latents[i] = F.interpolate(
                latents[i],
                latent_sz,
                mode="bilinear",
                align_corners=align_corners,
            ) # torch.Size([4, 64, 501, 500])

        # torch.Size([4, 512, 501, 500])
        self.latent = torch.cat(latents, dim=1) 

        return self.latent


class ImageEncoder(nn.Module):
    """
    Global image encoder
    """

    def __init__(self, backbone="resnet34", pretrained=True, latent_size=512):
        """Initialization

        Args: 
            :param backbone Backbone network. Assumes it is resnet*
                e.g. resnet34 | resnet50
            :param num_layers number of resnet layers to use, 1-5
            :param pretrained Whether to use model pretrained on ImageNet
        """
        super().__init__()
        self.model = getattr(torchvision.models, backbone)(pretrained=pretrained)
        self.model.fc = nn.Sequential()
        # self.register_buffer("latent", torch.empty(1, 1), persistent=False)
        self.latent = None
        self.latent_size = latent_size
        if latent_size != 512:
            self.fc = nn.Linear(latent_size, 512)

    def index(self, uv, image_size=()):
        """
        Params ignored (compatibility)
        :param uv (B, N, 2) only used for shape
        :return latent vector (B, L, N)
        """
        return self.latent.unsqueeze(-1).expand(-1, -1, uv.shape[1])

    def forward(self, x):
        """
        For extracting ResNet's features.
        :param x image (B, C, H, W)
        :return latent (B, latent_size)
        """
        # x = x.to(device=self.latent.device)
        x = self.model.conv1(x)
        x = self.model.bn1(x)
        x = self.model.relu(x)

        x = self.model.maxpool(x)
        x = self.model.layer1(x)
        x = self.model.layer2(x)
        x = self.model.layer3(x)
        x = self.model.layer4(x)

        x = self.model.avgpool(x)
        x = torch.flatten(x, 1)

        if self.latent_size != 512:
            x = self.fc(x)

        self.latent = x  # (B, latent_size)
        return self.latent


class SparseConvNet(nn.Module):
    """Find the corresponding 3D feature of query point along the ray
    
    Attributes:
        conv: sparse convolutional layer 
        down: sparse convolutional layer with downsample 
    """
    def __init__(self, num_layers=2):
        super(SparseConvNet, self).__init__()
        self.num_layers = num_layers

        self.conv0 = double_conv(3, 16, 'subm0')
        self.down0 = stride_conv(16, 32, 'down0')

        self.conv1 = double_conv(32, 32, 'subm1')
        self.down1 = stride_conv(32, 64, 'down1')

        self.conv2 = triple_conv(64, 64, 'subm2')
        self.down2 = stride_conv(64, 128, 'down2')

        self.conv3 = triple_conv(128, 128, 'subm3')
        self.down3 = stride_conv(128, 128, 'down3')

        self.conv4 = triple_conv(128, 128, 'subm4')

        self.channel = 32

    def forward(self, x, point_normalied_coords):
        """Find the corresponding 3D feature of query point along the ray.

        Args:
            x: Sparse Conv Tensor
            point_normalied_coords: Voxel grid coordinate, integer normalied to [-1, 1]
        
        Returns:
            features: Corresponding 3D feature of query point along the ray
        """
        features = []

        net = self.conv0(x)
        net = self.down0(net)

        point_normalied_coords = point_normalied_coords.unsqueeze(0).unsqueeze(0).unsqueeze(0)

        if self.num_layers > 1:
            net = self.conv1(net)
            net1 = net.dense()
            # torch.Size([1, 32, 1, 1, 4096])
            feature_1 = F.grid_sample(net1, point_normalied_coords, padding_mode='zeros', align_corners=True)
            features.append(feature_1)
            self.channel = 32
            net = self.down1(net)
        
        if self.num_layers > 2:
            net = self.conv2(net)
            net2 = net.dense()
            # torch.Size([1, 64, 1, 1, 4096])
            feature_2 = F.grid_sample(net2, point_normalied_coords, padding_mode='zeros', align_corners=True)
            features.append(feature_2)
            self.channel = 64
            net = self.down2(net)
        
        if self.num_layers > 3:
            net = self.conv3(net)
            net3 = net.dense()
            # 128
            feature_3 = F.grid_sample(net3, point_normalied_coords, padding_mode='zeros', align_corners=True)
            features.append(feature_3)
            self.channel = 128
            net = self.down3(net)
        
        if self.num_layers > 4:
            net = self.conv4(net)
            net4 = net.dense()
            # 256
            feature_4 = F.grid_sample(net4, point_normalied_coords, padding_mode='zeros', align_corners=True)
            features.append(feature_4)

        features = torch.cat(features, dim=1)
        # TODO, why so much dimention ???
        # B*C*D*H*W or B*D*H*W*C
        # B*N*C or B*C*N(num of query point)
        # torch.Size([1, 352, 4096])
        features = features.view(features.size(0), self.channel, features.size(4)).squeeze(0).transpose(0,1)

        return features


def single_conv(in_channels, out_channels, indice_key=None):
    return spconv.SparseSequential(
        spconv.SubMConv3d(in_channels,
                          out_channels,
                          1,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
    )


def double_conv(in_channels, out_channels, indice_key=None):
    tmp = spconv.SubMConv3d(in_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key)
    return spconv.SparseSequential(
        spconv.SubMConv3d(in_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
        spconv.SubMConv3d(out_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
    )


def triple_conv(in_channels, out_channels, indice_key=None):
    return spconv.SparseSequential(
        spconv.SubMConv3d(in_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
        spconv.SubMConv3d(out_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
        spconv.SubMConv3d(out_channels,
                          out_channels,
                          3,
                          bias=False,
                          indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01),
        nn.ReLU(),
    )


def stride_conv(in_channels, out_channels, indice_key=None):
    return spconv.SparseSequential(
        spconv.SparseConv3d(in_channels,
                            out_channels,
                            3,
                            2,
                            padding=1,
                            bias=False,
                            indice_key=indice_key),
        nn.BatchNorm1d(out_channels, eps=1e-3, momentum=0.01), nn.ReLU())



```

## Source file: parser_config.py

{
  "path": "parser_config.py",
  "blob_sha": "28046255e7df401202e67f3df5e5e6eb5cb58802",
  "sha256": "7b15697df37b34a796c873a4a406d065252c842d2af2f1a556292801eea2fa4c",
  "lines": 113,
  "bytes": 6624,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 1,
    "gum": 63
  },
  "url": "https://github.com/gaoxiangjun/MPS-NeRF/blob/02a2842cff8bca4206d0cae9339fa15e9694de1f/parser_config.py",
  "function_signature_candidates": [
    {
      "line": 3,
      "text": "def config_parser():"
    },
    {
      "line": 109,
      "text": "def print_args(args):"
    }
  ]
}

```text

import configargparse

def config_parser():
    parser = configargparse.ArgumentParser()
    parser.add_argument('--config', is_config_file=True, 
                        help='config file path')
    parser.add_argument("--expname", type=str, 
                        help='experiment name')
    parser.add_argument("--basedir", type=str, default='./logs/', 
                        help='where to store ckpts and logs')

    # training options
    parser.add_argument("--N_rand", type=int, default=1024*32, 
                        help='batch size (number of random rays per gradient step)')
    parser.add_argument("--lrate", type=float, default=5e-4, 
                        help='learning rate')
    parser.add_argument("--decay_steps", type=int, default=10000, 
                        help='exponential learning rate decay (in 1000 steps)')
    parser.add_argument("--chunk", type=int, default=1024*64, 
                        help='number of rays processed in parallel, decrease if running out of memory')
    parser.add_argument("--netchunk_per_gpu", type=int, default=1024*64*64, 
                        help='number of pts sent through network in parallel, decrease if running out of memory')
    parser.add_argument("--no_reload", action='store_true', 
                        help='do not reload weights from saved ckpt')
    parser.add_argument("--ft_path", type=str, default=None, 
                        help='specific weights npy file to reload for coarse network')

    # rendering options
    parser.add_argument("--N_samples", type=int, default=64, 
                        help='number of coarse samples per ray')
    parser.add_argument("--N_importance", type=int, default=0,
                        help='number of additional fine samples per ray')
    parser.add_argument("--perturb", type=float, default=1.,
                        help='set to 0. for no jitter, 1. for jitter')
    parser.add_argument("--use_viewdirs", action='store_true', 
                        help='use full 5D input instead of 3D')
    parser.add_argument("--with_viewdirs", type=int, default=1, 
                        help='use full 5D input instead of 3D')

    # dataset options
    parser.add_argument("--data_root", type=str, default='msra_h36m/S9/Posing', 
                        help='Dataset root dir')
    parser.add_argument("--data_set_type", type=str, default='multi_pair', 
                        help='Dataset root dir')
    parser.add_argument("--train_split", type=str, default="test", 
                        help='training dataloader type, choose whole image or random sample')
    parser.add_argument("--test_split", type=str, default="test", 
                        help='test dataloader type, choose whole image or random sample')
    parser.add_argument("--image_scaling", type=float, default="0.4", 
                        help='down sample factor')
    parser.add_argument("--model", type=str, default="correction_by_f3d", 
                        help='test dataloader type, choose whole image or random sample')
    parser.add_argument("--N_iteration",   type=int, default=48001, 
                        help='frequency of console printout and metric loggin')
    parser.add_argument("--white_bkgd", action='store_true', 
                        help='set to render synthetic data on a white bkgd (always use for dvoxels)')
    
    parser.add_argument("--use_os_env", type=int, default=0, help="--- ")
    parser.add_argument("--multi_person", type=int, default=1, help="--- ")
    
    parser.add_argument("--density_loss", type=int, default=0, help="--- ")
    parser.add_argument("--correction_loss", type=int, default=0, help="--- ")
    parser.add_argument("--acc_loss", type=int, default=1, help="--- ")
    parser.add_argument("--T_loss", type=int, default=1, help="--- ")
    parser.add_argument("--smooth_loss", type=int, default=1, help="--- ")
    parser.add_argument("--consistency_loss", type=int, default=0, help="--- ")
    
    parser.add_argument("--half_acc", type=int, default=0, help="--- ")
    parser.add_argument("--human_sample", type=int, default=0, help="--- ")
    parser.add_argument("--num_worker", type=int, default=8, help="--- ")
    parser.add_argument("--start", type=int, default=0, help="--- ")
    parser.add_argument("--interval", type=int, default=10, help="--- ")
    parser.add_argument("--poses_num", type=int, default=100, help="--- ")
    parser.add_argument("--num_instance", type=int, default=100, help="--- ")
    parser.add_argument("--test_num_instance", type=int, default=1, help="--- ")
    parser.add_argument("--random_pair", type=int, default=1, help="--- ")
    
    parser.add_argument("--use_f2d", type=int, default=0, help="--- ")
    parser.add_argument("--use_trans", type=int, default=0, help="--- ")
    parser.add_argument("--save_weights", type=int, default=1, help="--- ")
    parser.add_argument("--view_num",   type=int, default=3,  help='num of view, 3 for training, 4 for test')
    parser.add_argument("--border",   type=int, default=5, help='num of view, 3 for training, 4 for test')
    parser.add_argument("--batch_size", type=int, default=1)

    parser.add_argument("--local_rank", type=int, default=0)
    parser.add_argument("--ddp", type=int, default=0)
    parser.add_argument("--occupancy", type=int, default=0)
    parser.add_argument("--mean_shape", type=int, default=1)
    parser.add_argument("--correction_field", type=int, default=0)
    parser.add_argument("--skinning_field", type=int, default=0)
    parser.add_argument("--smooth_interval", type=int, default=4)
    parser.add_argument("--append_rgb", type=int, default=1)
    parser.add_argument("--male", type=int, default=0)
    parser.add_argument("--new_mask", type=int, default=0)
    parser.add_argument("--test_persons", type=int, default=2)
    parser.add_argument("--ani_nerf_ft", type=int, default=0)
    
    # logging/saving options
    parser.add_argument("--i_print",   type=int, default=120, help='frequency of console printout and metric loggin')
    parser.add_argument("--i_weights", type=int, default=12000, help='frequency of weight ckpt saving')
    parser.add_argument("--i_testset", type=int, default=3000, help='frequency of testset saving')

    parser.add_argument("--smpl_shape_loss", type=int, default=1)


    return parser


def print_args(args):
    print('--------args----------')
    for k in list(vars(args).keys()):
        print('%s: %s' % (k, vars(args)[k]))
    print('--------args----------\n')

```

## Source file: lib/base_utils.py

{
  "path": "lib/base_utils.py",
  "blob_sha": "84a9a9b36c4c1d5b2ff075644501daa026ba357a",
  "sha256": "941e72ce4d2650d2de14cd9be014b9469ba8340cd4ceec63013a6780a89851b6",
  "lines": 48,
  "bytes": 1322,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/gaoxiangjun/MPS-NeRF/blob/02a2842cff8bca4206d0cae9339fa15e9694de1f/lib/base_utils.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def read_pickle(pkl_path):"
    },
    {
      "line": 13,
      "text": "def save_pickle(data, pkl_path):"
    },
    {
      "line": 19,
      "text": "def project(xyz, K, RT):"
    },
    {
      "line": 31,
      "text": "def write_K_pose_inf(K, poses, img_root):"
    }
  ]
}

```text

import pickle
import os
import numpy as np


def read_pickle(pkl_path):
    with open(pkl_path, 'rb') as f:
        u = pickle._Unpickler(f)
        u.encoding = 'latin1'
        return u.load()


def save_pickle(data, pkl_path):
    os.system('mkdir -p {}'.format(os.path.dirname(pkl_path)))
    with open(pkl_path, 'wb') as f:
        pickle.dump(data, f)


def project(xyz, K, RT):
    """
    xyz: [N, 3]
    K: [3, 3]
    RT: [3, 4]
    """
    xyz = np.dot(xyz, RT[:, :3].T) + RT[:, 3:].T
    xyz = np.dot(xyz, K.T)
    xy = xyz[:, :2] / xyz[:, 2:]
    return xy


def write_K_pose_inf(K, poses, img_root):
    K = K.copy()
    K[:2] = K[:2] * 8
    K_inf = os.path.join(img_root, 'Intrinsic.inf')
    os.system('mkdir -p {}'.format(os.path.dirname(K_inf)))
    with open(K_inf, 'w') as f:
        for i in range(len(poses)):
            f.write('%d\n'%i)
            f.write('%f %f %f\n %f %f %f\n %f %f %f\n' % tuple(K.reshape(9).tolist()))
            f.write('\n')

    pose_inf = os.path.join(img_root, 'CamPose.inf')
    with open(pose_inf, 'w') as f:
        for pose in poses:
            pose = np.linalg.inv(pose)
            A = pose[0:3,:]
            tmp = np.concatenate([A[0:3,2].T, A[0:3,0].T,A[0:3,1].T,A[0:3,3].T])
            f.write('%f %f %f %f %f %f %f %f %f %f %f %f\n' % tuple(tmp.tolist()))


```

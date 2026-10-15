# Facial rendering source scan: Namkwangwoon/Rotation-Invariant-Tooth-Seg

Repository: https://github.com/Namkwangwoon/Rotation-Invariant-Tooth-Seg

Pinned source revision: 60174c7c9bd6cf03472e589adacfbf30cce34ec3

Discovery topic: teeth 3D

Description: 3D Teeth Scan Segmentation via Rotation-Invariant Descriptor

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["model/RIGA_v2.py", "snapshot/fdmatch_ripoint_transformer/model/RIGA_v2.py", "snapshot/tdmatch_ripoint_transformer/model/RIGA_v2.py"]}

## Source file: model/RIGA_v2.py

{
  "path": "model/RIGA_v2.py",
  "blob_sha": "422198c77ac32e8ee87e2b29df9103f1278fe995",
  "sha256": "8d9fdcb2cca9ef111bc732213676a514945ec525ba5e6a67fa8e392cc655f209",
  "lines": 181,
  "bytes": 9370,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/Namkwangwoon/Rotation-Invariant-Tooth-Seg/blob/60174c7c9bd6cf03472e589adacfbf30cce34ec3/model/RIGA_v2.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self, config):"
    },
    {
      "line": 58,
      "text": "def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):"
    },
    {
      "line": 178,
      "text": "def create_model(config):"
    }
  ]
}

```text

import torch

from model.model import *
from model.modules import CoarseMatching, AdaptiveSuperPointMatching, GTCoarseCorrGenerator, FineMatching, LearnableLogOptimalTransport
import torch.nn.functional as F
from lib.utils import to_o3d_pcd
import open3d as o3d


class RIGA_v2(nn.Module):
    '''
    The RoITr pipeline
    '''
    def __init__(self, config):
        super(RIGA_v2, self).__init__()
        self.config = config
        # backbone network
        self.with_cross_pos_embed = config.with_cross_pos_embed
        self.benchmark = config.benchmark

        if self.benchmark in ['3DMatch', '3DLoMatch']:

            self.coarse_matching = CoarseMatching(num_correspondences=config.num_est_coarse_corr, dual_normalization=True)
            self.factor = 1
        else:
            
            self.coarse_matching = AdaptiveSuperPointMatching(min_num_correspondences=config.num_est_coarse_corr, similarity_threshold=0.75)
            self.factor = 2

        self.backbone = RIPointTransformer(transformer_architecture=config.transformer_architecture, with_cross_pos_embed=self.with_cross_pos_embed, factor=self.factor)
        # learnable Optimal Transport Layer
        self.OT = LearnableLogOptimalTransport(num_iter=100)
        # the number of correspondences used for each point cloud pair during training

        self.mode = config.mode # current phase, should be in ['train', 'val', 'test']
        self.point_per_patch = config.point_per_patch
        self.matching_radius = config.matching_radius
        
        # coarse level final descriptor projection
        self.coarse_proj = nn.Linear(256*self.factor, 256*self.factor)
        # fine level final descriptor projection
        self.fine_proj = nn.Linear(64*self.factor, 256*self.factor)



        self.coarse_generator = GTCoarseCorrGenerator(num_targets=config.num_gt_coarse_corr, overlap_threshold=config.coarse_overlap_threshold)
        self.fine_matching = FineMatching(config.fine_matching_topk,
                                          mutual=config.fine_matching_mutual, confidence_threshold=config.fine_matching_confidence_threshold,
                                          use_dustbin=config.fine_matching_use_dustbin,
                                          use_global_score=config.fine_matching_use_global_score,
                                          correspondence_threshold=config.fine_matching_correspondence_threshold)

        self.fine_matching_use_dustbin = config.fine_matching_use_dustbin

        self.optimal_transport = LearnableLogOptimalTransport(num_iter=100)


    def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):
        src_o, tgt_o = torch.from_numpy(np.array([src_raw_pcd.shape[0]])).to(src_raw_pcd).int(), torch.from_numpy(np.array([tgt_pcd.shape[0]])).to(tgt_pcd).int()
        output_dict = {}
        # 1. get descriptors
        src_node_xyz, src_node_feats, src_pcd, src_point_feats, tgt_node_xyz, tgt_node_feats, tgt_pcd, tgt_point_feats = self.backbone([src_raw_pcd, src_feats, src_o, src_normals], [tgt_pcd, tgt_feats, tgt_o, tgt_normals], src_pcd)

        src_node_feats = F.normalize(self.coarse_proj(src_node_feats), p=2, dim=1)
        tgt_node_feats = F.normalize(self.coarse_proj(tgt_node_feats), p=2, dim=1)

        src_point_feats = self.fine_proj(src_point_feats)
        tgt_point_feats = self.fine_proj(tgt_point_feats)

        output_dict['src_points'] = src_pcd
        output_dict['tgt_points'] = tgt_pcd
        output_dict['src_nodes'] = src_node_xyz
        output_dict['tgt_nodes'] = tgt_node_xyz

        output_dict['src_point_feats'] = src_point_feats
        output_dict['tgt_point_feats'] = tgt_point_feats
        output_dict['src_node_feats'] = src_node_feats
        output_dict['tgt_node_feats'] = tgt_node_feats


        # 2. get ground truth node correspondences
        _, src_node_masks, src_node_knn_indices, src_node_knn_masks = point_to_node_partition(src_pcd, src_node_xyz, point_limit=self.point_per_patch)
        _, tgt_node_masks, tgt_node_knn_indices, tgt_node_knn_masks = point_to_node_partition(tgt_pcd, tgt_node_xyz, point_limit=self.point_per_patch)


        src_padded_points = torch.cat([src_pcd, torch.zeros_like(src_pcd[:1])], dim=0)
        tgt_padded_points = torch.cat([tgt_pcd, torch.zeros_like(tgt_pcd[:1])], dim=0)
        src_node_knn_points = index_select(src_padded_points, src_node_knn_indices, dim=0)
        tgt_node_knn_points = index_select(tgt_padded_points, tgt_node_knn_indices, dim=0)

        gt_node_corr_indices, gt_node_corr_overlaps = get_node_correspondences(
            tgt_node_xyz,
            src_node_xyz,
            tgt_node_knn_points,
            src_node_knn_points,
            rot, trans,
            self.matching_radius,
            ref_masks=tgt_node_masks,
            src_masks=src_node_masks,
            ref_knn_masks=tgt_node_knn_masks,
            src_knn_masks=src_node_knn_masks
        )
        gt_tgt_node_occ_score, gt_src_node_occ_score = get_node_occlusion_score(tgt_node_knn_indices,
                                                                                src_node_knn_indices,
                                                                                tgt_padded_points,
                                                                                src_padded_points,
                                                                                rot, trans,
                                                                                ref_masks=tgt_node_masks,
                                                                                src_masks=src_node_masks,
                                                                                ref_knn_masks=tgt_node_knn_masks,
                                                                                src_knn_masks=src_node_knn_masks)

        output_dict['gt_node_corr_indices'] = gt_node_corr_indices
        output_dict['gt_node_corr_overlaps'] = gt_node_corr_overlaps
        output_dict['gt_tgt_node_occ'] = gt_tgt_node_occ_score
        output_dict['gt_src_node_occ'] = gt_src_node_occ_score

        # 3. select topk node correspondences
        with torch.no_grad():

            tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_matching(tgt_node_feats, src_node_feats, tgt_node_masks, src_node_masks)
            output_dict['src_node_corr_indices'] = src_node_corr_indices
            output_dict['tgt_node_corr_indices'] = tgt_node_corr_indices

            if self.training:
                tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_generator(gt_node_corr_indices, gt_node_corr_overlaps.contiguous())

        # 4. Generate batched node points & feats
        src_node_corr_knn_indices = src_node_knn_indices[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_indices = tgt_node_knn_indices[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_masks = src_node_knn_masks[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_masks = tgt_node_knn_masks[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_points = src_node_knn_points[src_node_corr_indices]  # (P, K, 3)
        tgt_node_corr_knn_points = tgt_node_knn_points[tgt_node_corr_indices]  # (P, K, 3)

        src_padded_point_feats = torch.cat([src_point_feats, torch.zeros_like(src_point_feats[:1])], dim=0)
        tgt_padded_point_feats = torch.cat([tgt_point_feats, torch.zeros_like(tgt_point_feats[:1])], dim=0)

        src_node_corr_knn_feats = index_select(src_padded_point_feats, src_node_corr_knn_indices, dim=0)  # (P, K, C)
        tgt_node_corr_knn_feats = index_select(tgt_padded_point_feats, tgt_node_corr_knn_indices, dim=0)  # (P, K, C)

        output_dict['src_node_corr_knn_points'] = src_node_corr_knn_points
        output_dict['tgt_node_corr_knn_points'] = tgt_node_corr_knn_points
        output_dict['src_node_corr_knn_masks'] = src_node_corr_knn_masks
        output_dict['tgt_node_corr_knn_masks'] = tgt_node_corr_knn_masks

        # 5. Optimal transport
        matching_scores = torch.einsum('bnd,bmd->bnm', tgt_node_corr_knn_feats,
                                       src_node_corr_knn_feats)  # (P, K, K)
        matching_scores = matching_scores / src_point_feats.shape[1] ** 0.5
        matching_scores = self.optimal_transport(matching_scores, tgt_node_corr_knn_masks, src_node_corr_knn_masks)

        output_dict['matching_scores'] = matching_scores # row: tgt, col: src

        # 6. Generate final correspondences during testing
        with torch.no_grad():
            if not self.fine_matching_use_dustbin:
                matching_scores = matching_scores[:, :-1, :-1]

            tgt_corr_points, src_corr_points, corr_scores = self.fine_matching(
                tgt_node_corr_knn_points,
                src_node_corr_knn_points,
                tgt_node_corr_knn_masks,
                src_node_corr_knn_masks,
                matching_scores,
                node_corr_scores
            )

            output_dict['tgt_corr_points'] = tgt_corr_points
            output_dict['src_corr_points'] = src_corr_points
            output_dict['corr_scores'] = corr_scores

        return output_dict


def create_model(config):
    model = RIGA_v2(config)
    return model



```

## Source file: snapshot/fdmatch_ripoint_transformer/model/RIGA_v2.py

{
  "path": "snapshot/fdmatch_ripoint_transformer/model/RIGA_v2.py",
  "blob_sha": "422198c77ac32e8ee87e2b29df9103f1278fe995",
  "sha256": "8d9fdcb2cca9ef111bc732213676a514945ec525ba5e6a67fa8e392cc655f209",
  "lines": 181,
  "bytes": 9370,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/Namkwangwoon/Rotation-Invariant-Tooth-Seg/blob/60174c7c9bd6cf03472e589adacfbf30cce34ec3/snapshot/fdmatch_ripoint_transformer/model/RIGA_v2.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self, config):"
    },
    {
      "line": 58,
      "text": "def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):"
    },
    {
      "line": 178,
      "text": "def create_model(config):"
    }
  ]
}

```text

import torch

from model.model import *
from model.modules import CoarseMatching, AdaptiveSuperPointMatching, GTCoarseCorrGenerator, FineMatching, LearnableLogOptimalTransport
import torch.nn.functional as F
from lib.utils import to_o3d_pcd
import open3d as o3d


class RIGA_v2(nn.Module):
    '''
    The RoITr pipeline
    '''
    def __init__(self, config):
        super(RIGA_v2, self).__init__()
        self.config = config
        # backbone network
        self.with_cross_pos_embed = config.with_cross_pos_embed
        self.benchmark = config.benchmark

        if self.benchmark in ['3DMatch', '3DLoMatch']:

            self.coarse_matching = CoarseMatching(num_correspondences=config.num_est_coarse_corr, dual_normalization=True)
            self.factor = 1
        else:
            
            self.coarse_matching = AdaptiveSuperPointMatching(min_num_correspondences=config.num_est_coarse_corr, similarity_threshold=0.75)
            self.factor = 2

        self.backbone = RIPointTransformer(transformer_architecture=config.transformer_architecture, with_cross_pos_embed=self.with_cross_pos_embed, factor=self.factor)
        # learnable Optimal Transport Layer
        self.OT = LearnableLogOptimalTransport(num_iter=100)
        # the number of correspondences used for each point cloud pair during training

        self.mode = config.mode # current phase, should be in ['train', 'val', 'test']
        self.point_per_patch = config.point_per_patch
        self.matching_radius = config.matching_radius
        
        # coarse level final descriptor projection
        self.coarse_proj = nn.Linear(256*self.factor, 256*self.factor)
        # fine level final descriptor projection
        self.fine_proj = nn.Linear(64*self.factor, 256*self.factor)



        self.coarse_generator = GTCoarseCorrGenerator(num_targets=config.num_gt_coarse_corr, overlap_threshold=config.coarse_overlap_threshold)
        self.fine_matching = FineMatching(config.fine_matching_topk,
                                          mutual=config.fine_matching_mutual, confidence_threshold=config.fine_matching_confidence_threshold,
                                          use_dustbin=config.fine_matching_use_dustbin,
                                          use_global_score=config.fine_matching_use_global_score,
                                          correspondence_threshold=config.fine_matching_correspondence_threshold)

        self.fine_matching_use_dustbin = config.fine_matching_use_dustbin

        self.optimal_transport = LearnableLogOptimalTransport(num_iter=100)


    def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):
        src_o, tgt_o = torch.from_numpy(np.array([src_raw_pcd.shape[0]])).to(src_raw_pcd).int(), torch.from_numpy(np.array([tgt_pcd.shape[0]])).to(tgt_pcd).int()
        output_dict = {}
        # 1. get descriptors
        src_node_xyz, src_node_feats, src_pcd, src_point_feats, tgt_node_xyz, tgt_node_feats, tgt_pcd, tgt_point_feats = self.backbone([src_raw_pcd, src_feats, src_o, src_normals], [tgt_pcd, tgt_feats, tgt_o, tgt_normals], src_pcd)

        src_node_feats = F.normalize(self.coarse_proj(src_node_feats), p=2, dim=1)
        tgt_node_feats = F.normalize(self.coarse_proj(tgt_node_feats), p=2, dim=1)

        src_point_feats = self.fine_proj(src_point_feats)
        tgt_point_feats = self.fine_proj(tgt_point_feats)

        output_dict['src_points'] = src_pcd
        output_dict['tgt_points'] = tgt_pcd
        output_dict['src_nodes'] = src_node_xyz
        output_dict['tgt_nodes'] = tgt_node_xyz

        output_dict['src_point_feats'] = src_point_feats
        output_dict['tgt_point_feats'] = tgt_point_feats
        output_dict['src_node_feats'] = src_node_feats
        output_dict['tgt_node_feats'] = tgt_node_feats


        # 2. get ground truth node correspondences
        _, src_node_masks, src_node_knn_indices, src_node_knn_masks = point_to_node_partition(src_pcd, src_node_xyz, point_limit=self.point_per_patch)
        _, tgt_node_masks, tgt_node_knn_indices, tgt_node_knn_masks = point_to_node_partition(tgt_pcd, tgt_node_xyz, point_limit=self.point_per_patch)


        src_padded_points = torch.cat([src_pcd, torch.zeros_like(src_pcd[:1])], dim=0)
        tgt_padded_points = torch.cat([tgt_pcd, torch.zeros_like(tgt_pcd[:1])], dim=0)
        src_node_knn_points = index_select(src_padded_points, src_node_knn_indices, dim=0)
        tgt_node_knn_points = index_select(tgt_padded_points, tgt_node_knn_indices, dim=0)

        gt_node_corr_indices, gt_node_corr_overlaps = get_node_correspondences(
            tgt_node_xyz,
            src_node_xyz,
            tgt_node_knn_points,
            src_node_knn_points,
            rot, trans,
            self.matching_radius,
            ref_masks=tgt_node_masks,
            src_masks=src_node_masks,
            ref_knn_masks=tgt_node_knn_masks,
            src_knn_masks=src_node_knn_masks
        )
        gt_tgt_node_occ_score, gt_src_node_occ_score = get_node_occlusion_score(tgt_node_knn_indices,
                                                                                src_node_knn_indices,
                                                                                tgt_padded_points,
                                                                                src_padded_points,
                                                                                rot, trans,
                                                                                ref_masks=tgt_node_masks,
                                                                                src_masks=src_node_masks,
                                                                                ref_knn_masks=tgt_node_knn_masks,
                                                                                src_knn_masks=src_node_knn_masks)

        output_dict['gt_node_corr_indices'] = gt_node_corr_indices
        output_dict['gt_node_corr_overlaps'] = gt_node_corr_overlaps
        output_dict['gt_tgt_node_occ'] = gt_tgt_node_occ_score
        output_dict['gt_src_node_occ'] = gt_src_node_occ_score

        # 3. select topk node correspondences
        with torch.no_grad():

            tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_matching(tgt_node_feats, src_node_feats, tgt_node_masks, src_node_masks)
            output_dict['src_node_corr_indices'] = src_node_corr_indices
            output_dict['tgt_node_corr_indices'] = tgt_node_corr_indices

            if self.training:
                tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_generator(gt_node_corr_indices, gt_node_corr_overlaps.contiguous())

        # 4. Generate batched node points & feats
        src_node_corr_knn_indices = src_node_knn_indices[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_indices = tgt_node_knn_indices[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_masks = src_node_knn_masks[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_masks = tgt_node_knn_masks[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_points = src_node_knn_points[src_node_corr_indices]  # (P, K, 3)
        tgt_node_corr_knn_points = tgt_node_knn_points[tgt_node_corr_indices]  # (P, K, 3)

        src_padded_point_feats = torch.cat([src_point_feats, torch.zeros_like(src_point_feats[:1])], dim=0)
        tgt_padded_point_feats = torch.cat([tgt_point_feats, torch.zeros_like(tgt_point_feats[:1])], dim=0)

        src_node_corr_knn_feats = index_select(src_padded_point_feats, src_node_corr_knn_indices, dim=0)  # (P, K, C)
        tgt_node_corr_knn_feats = index_select(tgt_padded_point_feats, tgt_node_corr_knn_indices, dim=0)  # (P, K, C)

        output_dict['src_node_corr_knn_points'] = src_node_corr_knn_points
        output_dict['tgt_node_corr_knn_points'] = tgt_node_corr_knn_points
        output_dict['src_node_corr_knn_masks'] = src_node_corr_knn_masks
        output_dict['tgt_node_corr_knn_masks'] = tgt_node_corr_knn_masks

        # 5. Optimal transport
        matching_scores = torch.einsum('bnd,bmd->bnm', tgt_node_corr_knn_feats,
                                       src_node_corr_knn_feats)  # (P, K, K)
        matching_scores = matching_scores / src_point_feats.shape[1] ** 0.5
        matching_scores = self.optimal_transport(matching_scores, tgt_node_corr_knn_masks, src_node_corr_knn_masks)

        output_dict['matching_scores'] = matching_scores # row: tgt, col: src

        # 6. Generate final correspondences during testing
        with torch.no_grad():
            if not self.fine_matching_use_dustbin:
                matching_scores = matching_scores[:, :-1, :-1]

            tgt_corr_points, src_corr_points, corr_scores = self.fine_matching(
                tgt_node_corr_knn_points,
                src_node_corr_knn_points,
                tgt_node_corr_knn_masks,
                src_node_corr_knn_masks,
                matching_scores,
                node_corr_scores
            )

            output_dict['tgt_corr_points'] = tgt_corr_points
            output_dict['src_corr_points'] = src_corr_points
            output_dict['corr_scores'] = corr_scores

        return output_dict


def create_model(config):
    model = RIGA_v2(config)
    return model



```

## Source file: snapshot/tdmatch_ripoint_transformer/model/RIGA_v2.py

{
  "path": "snapshot/tdmatch_ripoint_transformer/model/RIGA_v2.py",
  "blob_sha": "422198c77ac32e8ee87e2b29df9103f1278fe995",
  "sha256": "8d9fdcb2cca9ef111bc732213676a514945ec525ba5e6a67fa8e392cc655f209",
  "lines": 181,
  "bytes": 9370,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/Namkwangwoon/Rotation-Invariant-Tooth-Seg/blob/60174c7c9bd6cf03472e589adacfbf30cce34ec3/snapshot/tdmatch_ripoint_transformer/model/RIGA_v2.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self, config):"
    },
    {
      "line": 58,
      "text": "def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):"
    },
    {
      "line": 178,
      "text": "def create_model(config):"
    }
  ]
}

```text

import torch

from model.model import *
from model.modules import CoarseMatching, AdaptiveSuperPointMatching, GTCoarseCorrGenerator, FineMatching, LearnableLogOptimalTransport
import torch.nn.functional as F
from lib.utils import to_o3d_pcd
import open3d as o3d


class RIGA_v2(nn.Module):
    '''
    The RoITr pipeline
    '''
    def __init__(self, config):
        super(RIGA_v2, self).__init__()
        self.config = config
        # backbone network
        self.with_cross_pos_embed = config.with_cross_pos_embed
        self.benchmark = config.benchmark

        if self.benchmark in ['3DMatch', '3DLoMatch']:

            self.coarse_matching = CoarseMatching(num_correspondences=config.num_est_coarse_corr, dual_normalization=True)
            self.factor = 1
        else:
            
            self.coarse_matching = AdaptiveSuperPointMatching(min_num_correspondences=config.num_est_coarse_corr, similarity_threshold=0.75)
            self.factor = 2

        self.backbone = RIPointTransformer(transformer_architecture=config.transformer_architecture, with_cross_pos_embed=self.with_cross_pos_embed, factor=self.factor)
        # learnable Optimal Transport Layer
        self.OT = LearnableLogOptimalTransport(num_iter=100)
        # the number of correspondences used for each point cloud pair during training

        self.mode = config.mode # current phase, should be in ['train', 'val', 'test']
        self.point_per_patch = config.point_per_patch
        self.matching_radius = config.matching_radius
        
        # coarse level final descriptor projection
        self.coarse_proj = nn.Linear(256*self.factor, 256*self.factor)
        # fine level final descriptor projection
        self.fine_proj = nn.Linear(64*self.factor, 256*self.factor)



        self.coarse_generator = GTCoarseCorrGenerator(num_targets=config.num_gt_coarse_corr, overlap_threshold=config.coarse_overlap_threshold)
        self.fine_matching = FineMatching(config.fine_matching_topk,
                                          mutual=config.fine_matching_mutual, confidence_threshold=config.fine_matching_confidence_threshold,
                                          use_dustbin=config.fine_matching_use_dustbin,
                                          use_global_score=config.fine_matching_use_global_score,
                                          correspondence_threshold=config.fine_matching_correspondence_threshold)

        self.fine_matching_use_dustbin = config.fine_matching_use_dustbin

        self.optimal_transport = LearnableLogOptimalTransport(num_iter=100)


    def forward(self, src_pcd, tgt_pcd, src_feats, tgt_feats, src_normals, tgt_normals, rot, trans, src_raw_pcd):
        src_o, tgt_o = torch.from_numpy(np.array([src_raw_pcd.shape[0]])).to(src_raw_pcd).int(), torch.from_numpy(np.array([tgt_pcd.shape[0]])).to(tgt_pcd).int()
        output_dict = {}
        # 1. get descriptors
        src_node_xyz, src_node_feats, src_pcd, src_point_feats, tgt_node_xyz, tgt_node_feats, tgt_pcd, tgt_point_feats = self.backbone([src_raw_pcd, src_feats, src_o, src_normals], [tgt_pcd, tgt_feats, tgt_o, tgt_normals], src_pcd)

        src_node_feats = F.normalize(self.coarse_proj(src_node_feats), p=2, dim=1)
        tgt_node_feats = F.normalize(self.coarse_proj(tgt_node_feats), p=2, dim=1)

        src_point_feats = self.fine_proj(src_point_feats)
        tgt_point_feats = self.fine_proj(tgt_point_feats)

        output_dict['src_points'] = src_pcd
        output_dict['tgt_points'] = tgt_pcd
        output_dict['src_nodes'] = src_node_xyz
        output_dict['tgt_nodes'] = tgt_node_xyz

        output_dict['src_point_feats'] = src_point_feats
        output_dict['tgt_point_feats'] = tgt_point_feats
        output_dict['src_node_feats'] = src_node_feats
        output_dict['tgt_node_feats'] = tgt_node_feats


        # 2. get ground truth node correspondences
        _, src_node_masks, src_node_knn_indices, src_node_knn_masks = point_to_node_partition(src_pcd, src_node_xyz, point_limit=self.point_per_patch)
        _, tgt_node_masks, tgt_node_knn_indices, tgt_node_knn_masks = point_to_node_partition(tgt_pcd, tgt_node_xyz, point_limit=self.point_per_patch)


        src_padded_points = torch.cat([src_pcd, torch.zeros_like(src_pcd[:1])], dim=0)
        tgt_padded_points = torch.cat([tgt_pcd, torch.zeros_like(tgt_pcd[:1])], dim=0)
        src_node_knn_points = index_select(src_padded_points, src_node_knn_indices, dim=0)
        tgt_node_knn_points = index_select(tgt_padded_points, tgt_node_knn_indices, dim=0)

        gt_node_corr_indices, gt_node_corr_overlaps = get_node_correspondences(
            tgt_node_xyz,
            src_node_xyz,
            tgt_node_knn_points,
            src_node_knn_points,
            rot, trans,
            self.matching_radius,
            ref_masks=tgt_node_masks,
            src_masks=src_node_masks,
            ref_knn_masks=tgt_node_knn_masks,
            src_knn_masks=src_node_knn_masks
        )
        gt_tgt_node_occ_score, gt_src_node_occ_score = get_node_occlusion_score(tgt_node_knn_indices,
                                                                                src_node_knn_indices,
                                                                                tgt_padded_points,
                                                                                src_padded_points,
                                                                                rot, trans,
                                                                                ref_masks=tgt_node_masks,
                                                                                src_masks=src_node_masks,
                                                                                ref_knn_masks=tgt_node_knn_masks,
                                                                                src_knn_masks=src_node_knn_masks)

        output_dict['gt_node_corr_indices'] = gt_node_corr_indices
        output_dict['gt_node_corr_overlaps'] = gt_node_corr_overlaps
        output_dict['gt_tgt_node_occ'] = gt_tgt_node_occ_score
        output_dict['gt_src_node_occ'] = gt_src_node_occ_score

        # 3. select topk node correspondences
        with torch.no_grad():

            tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_matching(tgt_node_feats, src_node_feats, tgt_node_masks, src_node_masks)
            output_dict['src_node_corr_indices'] = src_node_corr_indices
            output_dict['tgt_node_corr_indices'] = tgt_node_corr_indices

            if self.training:
                tgt_node_corr_indices, src_node_corr_indices, node_corr_scores = self.coarse_generator(gt_node_corr_indices, gt_node_corr_overlaps.contiguous())

        # 4. Generate batched node points & feats
        src_node_corr_knn_indices = src_node_knn_indices[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_indices = tgt_node_knn_indices[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_masks = src_node_knn_masks[src_node_corr_indices]  # (P, K)
        tgt_node_corr_knn_masks = tgt_node_knn_masks[tgt_node_corr_indices]  # (P, K)

        src_node_corr_knn_points = src_node_knn_points[src_node_corr_indices]  # (P, K, 3)
        tgt_node_corr_knn_points = tgt_node_knn_points[tgt_node_corr_indices]  # (P, K, 3)

        src_padded_point_feats = torch.cat([src_point_feats, torch.zeros_like(src_point_feats[:1])], dim=0)
        tgt_padded_point_feats = torch.cat([tgt_point_feats, torch.zeros_like(tgt_point_feats[:1])], dim=0)

        src_node_corr_knn_feats = index_select(src_padded_point_feats, src_node_corr_knn_indices, dim=0)  # (P, K, C)
        tgt_node_corr_knn_feats = index_select(tgt_padded_point_feats, tgt_node_corr_knn_indices, dim=0)  # (P, K, C)

        output_dict['src_node_corr_knn_points'] = src_node_corr_knn_points
        output_dict['tgt_node_corr_knn_points'] = tgt_node_corr_knn_points
        output_dict['src_node_corr_knn_masks'] = src_node_corr_knn_masks
        output_dict['tgt_node_corr_knn_masks'] = tgt_node_corr_knn_masks

        # 5. Optimal transport
        matching_scores = torch.einsum('bnd,bmd->bnm', tgt_node_corr_knn_feats,
                                       src_node_corr_knn_feats)  # (P, K, K)
        matching_scores = matching_scores / src_point_feats.shape[1] ** 0.5
        matching_scores = self.optimal_transport(matching_scores, tgt_node_corr_knn_masks, src_node_corr_knn_masks)

        output_dict['matching_scores'] = matching_scores # row: tgt, col: src

        # 6. Generate final correspondences during testing
        with torch.no_grad():
            if not self.fine_matching_use_dustbin:
                matching_scores = matching_scores[:, :-1, :-1]

            tgt_corr_points, src_corr_points, corr_scores = self.fine_matching(
                tgt_node_corr_knn_points,
                src_node_corr_knn_points,
                tgt_node_corr_knn_masks,
                src_node_corr_knn_masks,
                matching_scores,
                node_corr_scores
            )

            output_dict['tgt_corr_points'] = tgt_corr_points
            output_dict['src_corr_points'] = src_corr_points
            output_dict['corr_scores'] = corr_scores

        return output_dict


def create_model(config):
    model = RIGA_v2(config)
    return model



```

## Source file: eval.py

{
  "path": "eval.py",
  "blob_sha": "ca8ef16fb6d289bfde39b95fac5b571a6f776b66",
  "sha256": "633c63be5029cc49743502973d3c378787cffe51b3e4dc6ed26b6b9f569f600e",
  "lines": 174,
  "bytes": 6977,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3
  },
  "url": "https://github.com/Namkwangwoon/Rotation-Invariant-Tooth-Seg/blob/60174c7c9bd6cf03472e589adacfbf30cce34ec3/eval.py",
  "function_signature_candidates": []
}

```text

import torch
from model.backbone_only import RIPointTransformer
from model.pointnet_sem_seg import get_model
from model.pointtransformer_seg import PointTransformerSeg, PointTransformerBlock

import os
import gen_utils as gu
import numpy as np

import pyvista as pv
import open3d as o3d

import pymeshlab
import augmentator as aug
from dataset.common import normal_redirect

import argparse
from dataset.tdmatch import DentalMeshDataset
from torch.utils.data import DataLoader

from lib.evaluation import cal_metric

palet = np.array([
    [255,153,153],

    [153,76,0],
    [153,153,0],
    [76,153,0],
    [0,153,153],
    [0,0,153],
    [153,0,153],
    [153,0,76],
    [64,64,64],

    [20, 10, 0],
    [10, 10, 0],
    [10, 20, 0],
    [0, 10, 20],
    [0, 0, 20],
    [10, 0, 10],
    [10, 0, 0],
    [10, 10, 10],
    ])/255


Y_AXIS_MAX = 33.15232091532151
Y_AXIS_MIN = -36.9843781139949

view_point = np.array([0., 0., 0.])

aug_obj = aug.Augmentator([aug.Rotation([-180,180], 'rand')])

parser = argparse.ArgumentParser()
parser.add_argument('--model', default=None, type=str)
parser.add_argument('--lr_head', action='store_true')
args = parser.parse_args()



# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead_multiscale/epoch77_val1.0147_cls_acc0.7454_mask_acc0.9737.pth'
# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead_multiscale_vertexnorm/epoch83_val0.9012_cls_acc0.8159_mask_acc0.9616.pth'
# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead_multiscale_vertexnorm_focal/epoch88_val0.9179_cls_acc0.7738_mask_acc0.9678_.pth'
# checkpoint_path = 'checkpoints_rollback/fps_saved/epoch62_val0.4737_cls_acc0.7195.pth'
checkpoint_path = 'checkpoints_rollback/simplification_saved/epoch65_val0.4956_cls_acc0.7171.pth'
# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead/epoch79_val0.5562_cls_acc0.7180_mask_acc0.9841.pth'
# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead_multiscale_focal/epoch85_val0.8152_cls_acc0.7560_mask_acc0.9757.pth'
# checkpoint_path = 'checkpoints_rollback/simplification_saved_maskhead_multiscale_vertexnorm_normredirect/epoch82_val0.7044_cls_acc0.8453_mask_acc0.9674_.pth'

# checkpoint_path = 'checkpoints_rotate/vertexnorm/epoch66_val0.8429_cls_acc0.7969_mask_acc0.9573_.pth'
# checkpoint_path = 'checkpoints_rotate/aligned_val/vertexnorm_normredir/epoch87_val0.7268_cls_acc0.8255_mask_acc0.9676_.pth'

## PointNet
# checkpoint_path = 'checkpoints_pointnet/vertexnorm/epoch97_val1.1880_cls_acc0.7050_mask_acc0.0000.pth'
# checkpoint_path = 'checkpoints_pointnet/rotate_aligned_vertexnorm/epoch58_val2.2611_cls_acc0.5248_mask_acc0.0000.pth'

## Point Transformer
# checkpoint_path = 'checkpoints_pointtransformer/vertexnorm/epoch90_val1.2929_cls_acc0.8183_mask_acc0.0000.pth'
# checkpoint_path = 'checkpoints_pointtransformer/rotate_aligned_vertexnorm/epoch74_val2.9434_cls_acc0.5932_mask_acc0.0000.pth'

print(checkpoint_path)

os.environ["CUDA_DEVICE_ORDER"]="PCI_BUS_ID"
# os.environ["CUDA_VISIBLE_DEVICES"]= "0, 1"
device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

print('Device:', device)  # 출력결과: cuda 
print('Count of using GPUs:', torch.cuda.device_count())   #출력결과: 2 (0, 1 두개 사용하므로)
print('Current cuda device:', torch.cuda.current_device())  # 출력결과: 0 (0, 1 중 앞의 GPU #0 의미)


if args.model == None:
    model = RIPointTransformer(transformer_architecture=['self', 'cross', 'self', 'cross', 'self', 'cross'], with_cross_pos_embed=True, factor=1)
elif args.model == 'pointnet':
    model = get_model(17)
elif args.model == 'pointtransformer':
    model = PointTransformerSeg(PointTransformerBlock, [2, 3, 4, 6, 3], c=1, k=17)
model.cuda()
model.load_state_dict(torch.load(checkpoint_path))
# model.load_state_dict(torch.load(checkpoint_path)['model_state_dict'])
model.eval()

batch_size = 1

validation_set = DataLoader(DentalMeshDataset(split_with_txt_path='base_name_test_fold.txt', augmentation=False), batch_size=batch_size, shuffle=False, num_workers=0)

with torch.no_grad():
    
    val_loss = 0.0
    # val_acc = 0.0
    val_total_cls_acc = 0.0
    val_total_mask_acc = 0.0
    # val_total_sem_acc = 0.0
    val_total_lr_acc = 0.0
    
    val_total_iou = 0.0
    val_total_f1 = 0.0
    val_total_acc = 0.0
    val_total_sem_acc = 0.0
    
    model.eval()
        
    print()
    print("Start Validation!")
    for i, data in enumerate(validation_set):
        
        src_pcd, src_normals, src_feats, src_raw_pcd, labels = data
        src_pcd, src_normals, src_feats, src_raw_pcd, labels = src_pcd[0], src_normals[0], src_feats[0], src_raw_pcd[0], labels[0]
        src_o = torch.tensor([src_raw_pcd.shape[0]]).to(src_raw_pcd).int()
        labels = labels.reshape(-1)
        
        mask_labels = labels.clone()
        mask_labels[mask_labels>0] = 1
    
        sem_labels = labels.clone()
        sem_labels[sem_labels>8] = sem_labels[sem_labels>8] - 8
        
        lr_labels = labels.clone()
        lr_labels[(lr_labels>=1) & (lr_labels<=8)] = 1
        lr_labels[(lr_labels>=9) & (lr_labels<=16)] = 2
        
        
        if args.model == None:
            # cls_output, mask_output, sem_output = model([src_raw_pcd, src_feats, src_o, src_normals])
            cls_output, mask_output, lr_output = model([src_raw_pcd, src_feats, src_o, src_normals])
            # mask_output = mask_output.argmax(-1)
        elif args.model == 'pointnet':
            src_raw_pcd = src_raw_pcd.unsqueeze(0).permute(0, 2, 1)
            cls_output = model(src_raw_pcd)[0]
        elif args.model == 'pointtransformer':
            cls_output = model([src_raw_pcd, src_feats, src_o])
            
        cls_output = cls_output.argmax(-1)
        
        sem_output = cls_output.clone()
        sem_output[sem_output>8] = sem_output[sem_output>8] - 8
        
        iou, f1, acc, sem_acc, iou_arr = cal_metric(labels.detach().cpu(), cls_output.detach().cpu(), cls_output.detach().cpu())
        
        
        if (i+1) % 10 == 0:
            print("[{}/{}]  IOU: {iou:.6f}   F1: {f1:.6f}   ACC: {acc:.6f}   SEM_ACC: {sem_acc:.6f}"
                .format(i+1, len(validation_set), iou=iou, f1=f1, acc=acc, sem_acc=sem_acc))
            
        val_total_iou += iou
        val_total_f1 += f1
        val_total_acc += acc
        val_total_sem_acc += sem_acc

print()
print()
print("[Results]  IOU: {iou:.6f}   F1: {f1:.6f}   ACC: {acc:.6f}   SEM_ACC: {sem_acc:.6f}".format(iou=val_total_iou/len(validation_set),
                                                                                                  f1=val_total_f1/len(validation_set),
                                                                                                  acc=val_total_acc/len(validation_set),
                                                                                                  sem_acc=val_total_sem_acc/len(validation_set)))


```

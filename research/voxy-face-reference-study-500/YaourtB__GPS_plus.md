# Facial rendering source scan: YaourtB/GPS_plus

Repository: https://github.com/YaourtB/GPS_plus

Pinned source revision: 9469ee1c7c6532d2c20cbbc458dc11c63fea086b

Discovery topic: human rendering

Description: [T-PAMI 2025] The official repo for “GPS-Gaussian+: Generalizable Pixel-Wise 3D Gaussian Splatting for Real-Time Human-Scene Rendering from Sparse Views”

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "c1bf5492208b6421fe5a04de6b086836494bf053",
  "sha256": "a6aac5ee8fc5db1b9704ae44a25f0a8ce264da68e30ccf619b055b466827e3af",
  "lines": 263,
  "bytes": 10743,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 1
  },
  "url": "https://github.com/YaourtB/GPS_plus/blob/9469ee1c7c6532d2c20cbbc458dc11c63fea086b/train.py",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "def __init__(self, cfg_file):"
    },
    {
      "line": 64,
      "text": "def train(self):"
    },
    {
      "line": 159,
      "text": "def run_eval(self):"
    },
    {
      "line": 193,
      "text": "def fetch_data(self, phase):"
    },
    {
      "line": 212,
      "text": "def load_ckpt(self, load_path, load_optimizer=True, strict=True):"
    },
    {
      "line": 225,
      "text": "def save_ckpt(self, save_path, show_log=True):"
    }
  ]
}

```text

from __future__ import print_function, division

import argparse
import logging

import numpy as np
import cv2
import os
import random
from pathlib import Path
from tqdm import tqdm
from datetime import datetime
from lib.human_loader import StereoHumanDataset
from lib.network import RtStereoHumanModel
from config.stereo_human_config import ConfigStereoHuman as config
from lib.train_recoder import Logger, file_backup
from lib.GaussianRender import pts2render
from lib.gs_utils.loss_utils import l1_loss, ssim
from lib.gs_utils.image_utils import psnr
from pytorch3d.loss import chamfer_distance

import trimesh 
import torch
import torch.nn as nn
import torch.optim as optim
from torch.cuda.amp import GradScaler
from torch.utils.data import DataLoader
import warnings
from copy import deepcopy
warnings.filterwarnings("ignore", category=UserWarning)

class Trainer:
    def __init__(self, cfg_file):
        self.cfg = cfg_file
        self.bs = self.cfg.batch_size



        self.model = RtStereoHumanModel(self.cfg, with_gs_render=True)
        self.train_set = StereoHumanDataset(self.cfg.dataset, phase='train')
        self.train_loader = DataLoader(self.train_set, batch_size=self.bs, shuffle=True, num_workers=8, pin_memory=True)
        self.train_iterator = iter(self.train_loader)
        self.val_set = StereoHumanDataset(self.cfg.dataset, phase='val')
        self.val_loader = DataLoader(self.val_set, batch_size=1, shuffle=False, num_workers=8, pin_memory=True)
        self.len_val = int(len(self.val_loader) / self.val_set.val_boost)  # real length of val set
        self.val_iterator = iter(self.val_loader)
        self.optimizer = optim.AdamW(self.model.parameters(), lr=self.cfg.lr, weight_decay=self.cfg.wdecay, eps=1e-8)
        self.scheduler = optim.lr_scheduler.OneCycleLR(self.optimizer, self.cfg.lr, self.cfg.num_steps + 100,
                                                       pct_start=0.01, cycle_momentum=False, anneal_strategy='linear')

        self.logger = Logger(self.scheduler, cfg.record)
        self.total_steps = 0

        self.model.cuda()
        if self.cfg.restore_ckpt:
            self.load_ckpt(self.cfg.restore_ckpt)
        elif self.cfg.stage1_ckpt:
            logging.info(f"Using checkpoint from stage1")
            self.load_ckpt(self.cfg.stage1_ckpt, load_optimizer=False, strict=False)
        self.model.train()
        self.model.raft_stereo.freeze_bn()  # We keep BatchNorm frozen in Raft-Stereo
        self.scaler = GradScaler(enabled=self.cfg.raft.mixed_precision)

    def train(self):
        log_l1 = 0
        log_ssim = 0
        log_chamfer = 0
        log_scale = 0
        if_chamfer = True   
        if_scale = False  
        iter_from = -1 
        for itr_ in tqdm(range(self.total_steps, self.cfg.num_steps)):
            self.optimizer.zero_grad()
            data = self.fetch_data(phase='train')

            #  Raft Stereo
            data, _, metrics = self.model(data, is_train=True)
            #  Gaussian Render
            data = pts2render(data, bg_color=self.cfg.dataset.bg_color)

            # Loss
            render_novel = data['novel_view']['img_pred']
            gt_novel = data['novel_view']['img'].cuda()

            l_xyz = data['lmain']['xyz']
            r_xyz = data['rmain']['xyz']
            chamfer_loss = 0
            if if_chamfer and itr_ > iter_from:
                for b_i in range(self.bs):
                    l_valid_i = data['lmain']['pts_valid'][b_i, :]  # [S*S]
                    l_xyz_i = l_xyz[b_i, :, :]
                    l_xyz_i = l_xyz_i[l_valid_i].view(1, -1, 3).contiguous()
                    
                    r_valid_i = data['rmain']['pts_valid'][b_i, :]  # [S*S]
                    r_xyz_i = r_xyz[b_i, :, :]
                    r_xyz_i = r_xyz_i[r_valid_i].view(1, -1, 3).contiguous()
                    
                    sample_l = np.random.choice(l_xyz_i.shape[1], 10000, replace = False)
                    sample_r = np.random.choice(r_xyz_i.shape[1], 10000, replace = False)
                    chamfer_loss_i, _ = chamfer_distance(l_xyz_i[:, sample_l], r_xyz_i[:, sample_r])
                    chamfer_loss += chamfer_loss_i
                
                chamfer_loss /= self.bs

            Ll1 = l1_loss(render_novel, gt_novel)
            Lssim = 1.0 - ssim(render_novel, gt_novel)
            loss = 0.8 * Ll1 + 0.2 * Lssim + 0.5 * chamfer_loss 

            log_l1 += 0.8 * Ll1.item()
            log_ssim += 0.2 * Lssim.item()
            log_chamfer += 0.5 * chamfer_loss.item() if if_chamfer and itr_>iter_from else 0 
            log_scale += 0.5 * data['novel_view']['scale_regular'].item() if if_scale else 0

            if self.total_steps and self.total_steps % self.cfg.record.loss_freq == 0:
                self.logger.writer.add_scalar(f'lr', self.optimizer.param_groups[0]['lr'], self.total_steps)
                self.save_ckpt(save_path=Path('%s/%s_latest.pth' % (cfg.record.ckpt_path, cfg.name)), show_log=False)
            metrics.update({
                'l1': Ll1.item(),
                'ssim': Lssim.item(),
                'chamfer': 0.5*chamfer_loss.item() if if_chamfer and itr_>iter_from else 0
            })
            self.logger.push(metrics)

            self.scaler.scale(loss).backward()
            self.scaler.unscale_(self.optimizer)
            torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)

            self.scaler.step(self.optimizer)
            self.scheduler.step()
            self.scaler.update()

            if self.total_steps and self.total_steps % self.cfg.record.eval_freq == 0:
                self.model.eval()
                self.run_eval()
                self.model.train()
                self.model.raft_stereo.freeze_bn()
                
            if self.total_steps in self.cfg.record.save_iter:
                self.save_ckpt(save_path=Path('%s/iter%d.pth' % (cfg.record.ckpt_path, self.total_steps)))


            self.total_steps += 1
            if self.total_steps % 100 == 99:
                print(
                    'l1 ', log_l1/ 100,
                    'ssim', log_ssim/100,
                    'chamfer', log_chamfer/100,
                    'scale', log_scale/100,
                    )
                log_l1 = 0
                log_ssim = 0
                log_chamfer = 0
                log_scale = 0

        print("FINISHED TRAINING")
        self.logger.close()
        self.save_ckpt(save_path=Path('%s/%s_final.pth' % (cfg.record.ckpt_path, cfg.name)))

    def run_eval(self):
        logging.info(f"Doing validation ...")
        torch.cuda.empty_cache()
        psnr_list = []
        show_idx = np.random.choice(list(range(self.len_val)), 1)
 
        for idx in range(self.len_val):
            data = self.fetch_data(phase='val')
            with torch.no_grad():
                data, _, _ = self.model(data, is_train=False)
                data = pts2render(data, bg_color=self.cfg.dataset.bg_color)

                render_novel = data['novel_view']['img_pred']
                gt_novel = data['novel_view']['img'].cuda()
                psnr_value = psnr(render_novel, gt_novel).mean().double()
                psnr_list.append(psnr_value.item())

                if idx == show_idx:
                    tmp_novel = data['novel_view']['img_pred'][0].detach()
                    tmp_novel *= 255
                    tmp_novel = tmp_novel.permute(1, 2, 0).cpu().numpy()
                    tmp_img_name = '%s/%s.jpg' % (cfg.record.show_path, self.total_steps)
                    cv2.imwrite(tmp_img_name, tmp_novel[:, :, ::-1].astype(np.uint8))

        val_psnr = np.round(np.mean(np.array(psnr_list)), 4)
        if val_psnr < 10:
            print('something wrong during training, please change random seed and re-train')
            exit()

            
        logging.info(f"Validation Metrics ({self.total_steps}): psnr {val_psnr}")
        self.logger.write_dict({'val_psnr': val_psnr}, write_step=self.total_steps)
        torch.cuda.empty_cache()

    def fetch_data(self, phase):
        if phase == 'train':
            try:
                data = next(self.train_iterator)
            except:
                self.train_iterator = iter(self.train_loader)
                data = next(self.train_iterator)
        elif phase == 'val':
            try:
                data = next(self.val_iterator)
            except:
                self.val_iterator = iter(self.val_loader)
                data = next(self.val_iterator)

        for view in ['lmain', 'rmain']:
            for item in data[view].keys():
                data[view][item] = data[view][item].cuda()
        return data

    def load_ckpt(self, load_path, load_optimizer=True, strict=True):
        assert os.path.exists(load_path)
        logging.info(f"Loading checkpoint from {load_path} ...")
        ckpt = torch.load(load_path, map_location='cuda')
        self.model.load_state_dict(ckpt['network'], strict=strict)
        logging.info(f"Parameter loading done")
        if load_optimizer:
            self.total_steps = ckpt['total_steps'] + 1
            self.logger.total_steps = self.total_steps
            self.optimizer.load_state_dict(ckpt['optimizer'])
            self.scheduler.load_state_dict(ckpt['scheduler'])
            logging.info(f"Optimizer loading done")

    def save_ckpt(self, save_path, show_log=True):
        if show_log:
            logging.info(f"Save checkpoint to {save_path} ...")
        torch.save({
            'total_steps': self.total_steps,
            'network': self.model.state_dict(),
            'optimizer': self.optimizer.state_dict(),
            'scheduler': self.scheduler.state_dict()
        }, save_path)


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO,
                        format='%(asctime)s %(levelname)-8s [%(filename)s:%(lineno)d] %(message)s')

    cfg = config()
    cfg.load("config/stage.yaml")
    cfg = cfg.get_cfg()

    cfg.defrost()
    dt = datetime.today()
    cfg.exp_name = '%s_%s%s' % (cfg.name, str(dt.month).zfill(2), str(dt.day).zfill(2))
    cfg.record.ckpt_path = "experiments/%s/ckpt" % cfg.exp_name
    cfg.record.show_path = "experiments/%s/show" % cfg.exp_name
    cfg.record.logs_path = "experiments/%s/logs" % cfg.exp_name
    cfg.record.file_path = "experiments/%s/file" % cfg.exp_name
    cfg.freeze()


    for path in [cfg.record.ckpt_path, cfg.record.show_path, cfg.record.logs_path, cfg.record.file_path]:
        Path(path).mkdir(exist_ok=True, parents=True)
    
    file_backup(cfg.record.file_path, cfg, train_script=os.path.basename(__file__))

    torch.manual_seed(1314)
    np.random.seed(1314)

    trainer = Trainer(cfg)
    trainer.train()


```

## Source file: lib/loss.py

{
  "path": "lib/loss.py",
  "blob_sha": "9455a82067e69dbce05a7d89e59043602e8adffe",
  "sha256": "65ea34f884485f273762e323c182b337767b286dc562d0d668a093b27958e0bf",
  "lines": 31,
  "bytes": 1069,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/YaourtB/GPS_plus/blob/9469ee1c7c6532d2c20cbbc458dc11c63fea086b/lib/loss.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def sequence_loss(flow_preds, flow_gt, valid, loss_gamma=0.9):"
    }
  ]
}

```text


import torch


def sequence_loss(flow_preds, flow_gt, valid, loss_gamma=0.9):
    """ Loss function defined over sequence of flow predictions """

    n_predictions = len(flow_preds)
    flow_loss = 0.0

    valid = (valid >= 0.5)
    assert not torch.isinf(flow_gt[valid.bool()]).any()

    for i in range(n_predictions):
        # assert not torch.isnan(flow_preds[i]).any() and not torch.isinf(flow_preds[i]).any()  # 
        # We adjust the loss_gamma so it is consistent for any number of RAFT-Stereo iterations
        adjusted_loss_gamma = loss_gamma**(15/(n_predictions - 1))
        i_weight = adjusted_loss_gamma**(n_predictions - i - 1)
        i_loss = (flow_preds[i] - flow_gt).abs()
        flow_loss += i_weight * i_loss[valid.bool()].mean()

    epe = torch.sum((flow_preds[-1] - flow_gt)**2, dim=1).sqrt()  # 
    epe = epe.view(-1)[valid.view(-1)]

    metrics = {
        'train_epe': epe.mean().item(),
        'train_1px': (epe < 1).float().mean().item(),
        'train_3px': (epe < 3).float().mean().item()
    }

    return flow_loss, metrics


```

## Source file: core/corr.py

{
  "path": "core/corr.py",
  "blob_sha": "97e79e6ac41e7f14b55a5848ea084b01a017daa3",
  "sha256": "cf3a9dacbfbfd28656f793a9857d066c7ce2a05fbceb3a3f3206522649c7be19",
  "lines": 188,
  "bytes": 6628,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/YaourtB/GPS_plus/blob/9469ee1c7c6532d2c20cbbc458dc11c63fea086b/core/corr.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def forward(ctx, volume, coords, radius):"
    },
    {
      "line": 25,
      "text": "def backward(ctx, grad_output):"
    },
    {
      "line": 32,
      "text": "def __init__(self, fmap1, fmap2, num_levels=4, radius=4):"
    },
    {
      "line": 44,
      "text": "def __call__(self, coords):"
    },
    {
      "line": 54,
      "text": "def corr(fmap1, fmap2):"
    },
    {
      "line": 65,
      "text": "def __init__(self, fmap1, fmap2, num_levels=4, radius=4):"
    },
    {
      "line": 72,
      "text": "def corr(self, fmap1, fmap2, coords):"
    },
    {
      "line": 89,
      "text": "def __call__(self, coords):"
    },
    {
      "line": 111,
      "text": "def __init__(self, fmap1, fmap2, num_levels=4, radius=4):"
    },
    {
      "line": 127,
      "text": "def __call__(self, coords):"
    },
    {
      "line": 149,
      "text": "def corr(fmap1, fmap2):"
    },
    {
      "line": 160,
      "text": "def __init__(self, fmap1, fmap2, num_levels=4, radius=4):"
    },
    {
      "line": 171,
      "text": "def __call__(self, coords):"
    }
  ]
}

```text

import torch
import torch.nn.functional as F
from core.utils.utils import bilinear_sampler

try:
    import corr_sampler
except:
    pass

try:
    import alt_cuda_corr
except:
    # alt_cuda_corr is not compiled
    pass


class CorrSampler(torch.autograd.Function):
    @staticmethod
    def forward(ctx, volume, coords, radius):
        ctx.save_for_backward(volume,coords)
        ctx.radius = radius
        corr, = corr_sampler.forward(volume, coords, radius)
        return corr
    @staticmethod
    def backward(ctx, grad_output):
        volume, coords = ctx.saved_tensors
        grad_output = grad_output.contiguous()
        grad_volume, = corr_sampler.backward(volume, coords, grad_output, ctx.radius)
        return grad_volume, None, None

class CorrBlockFast1D:
    def __init__(self, fmap1, fmap2, num_levels=4, radius=4):
        self.num_levels = num_levels
        self.radius = radius
        self.corr_pyramid = []
        # all pairs correlation
        corr = CorrBlockFast1D.corr(fmap1, fmap2)
        batch, h1, w1, dim, w2 = corr.shape
        corr = corr.reshape(batch*h1*w1, dim, 1, w2)
        for i in range(self.num_levels):
            self.corr_pyramid.append(corr.view(batch, h1, w1, -1, w2//2**i))
            corr = F.avg_pool2d(corr, [1,2], stride=[1,2])

    def __call__(self, coords):
        out_pyramid = []
        bz, _, ht, wd = coords.shape
        coords = coords[:, [0]]
        for i in range(self.num_levels):
            corr = CorrSampler.apply(self.corr_pyramid[i].squeeze(3), coords/2**i, self.radius)
            out_pyramid.append(corr.view(bz, -1, ht, wd))
        return torch.cat(out_pyramid, dim=1)

    @staticmethod
    def corr(fmap1, fmap2):
        B, D, H, W1 = fmap1.shape
        _, _, _, W2 = fmap2.shape
        fmap1 = fmap1.view(B, D, H, W1)
        fmap2 = fmap2.view(B, D, H, W2)
        corr = torch.einsum('aijk,aijh->ajkh', fmap1, fmap2)
        corr = corr.reshape(B, H, W1, 1, W2).contiguous()
        return corr / torch.sqrt(torch.tensor(D).float())


class PytorchAlternateCorrBlock1D:
    def __init__(self, fmap1, fmap2, num_levels=4, radius=4):
        self.num_levels = num_levels
        self.radius = radius
        self.corr_pyramid = []
        self.fmap1 = fmap1
        self.fmap2 = fmap2

    def corr(self, fmap1, fmap2, coords):
        B, D, H, W = fmap2.shape
        # map grid coordinates to [-1,1]
        xgrid, ygrid = coords.split([1,1], dim=-1)
        xgrid = 2*xgrid/(W-1) - 1
        ygrid = 2*ygrid/(H-1) - 1

        grid = torch.cat([xgrid, ygrid], dim=-1)
        output_corr = []
        for grid_slice in grid.unbind(3):
            fmapw_mini = F.grid_sample(fmap2, grid_slice, align_corners=True)
            corr = torch.sum(fmapw_mini * fmap1, dim=1)
            output_corr.append(corr)
        corr = torch.stack(output_corr, dim=1).permute(0,2,3,1)

        return corr / torch.sqrt(torch.tensor(D).float())

    def __call__(self, coords):
        r = self.radius
        coords = coords.permute(0, 2, 3, 1)
        batch, h1, w1, _ = coords.shape
        fmap1 = self.fmap1
        fmap2 = self.fmap2
        out_pyramid = []
        for i in range(self.num_levels):
            dx = torch.zeros(1)
            dy = torch.linspace(-r, r, 2*r+1)
            delta = torch.stack(torch.meshgrid(dy, dx), axis=-1).to(coords.device)
            centroid_lvl = coords.reshape(batch, h1, w1, 1, 2).clone()
            centroid_lvl[...,0] = centroid_lvl[...,0] / 2**i
            coords_lvl = centroid_lvl + delta.view(-1, 2)
            corr = self.corr(fmap1, fmap2, coords_lvl)
            fmap2 = F.avg_pool2d(fmap2, [1, 2], stride=[1, 2])
            out_pyramid.append(corr)
        out = torch.cat(out_pyramid, dim=-1)
        return out.permute(0, 3, 1, 2).contiguous().float()


class CorrBlock1D:
    def __init__(self, fmap1, fmap2, num_levels=4, radius=4):
        self.num_levels = num_levels
        self.radius = radius
        self.corr_pyramid = []

        # all pairs correlation
        corr = CorrBlock1D.corr(fmap1, fmap2)

        batch, h1, w1, _, w2 = corr.shape
        corr = corr.reshape(batch*h1*w1, 1, 1, w2)

        self.corr_pyramid.append(corr)
        for i in range(self.num_levels):
            corr = F.avg_pool2d(corr, [1,2], stride=[1,2])
            self.corr_pyramid.append(corr)

    def __call__(self, coords):
        r = self.radius
        coords = coords[:, :1].permute(0, 2, 3, 1)
        batch, h1, w1, _ = coords.shape

        out_pyramid = []
        for i in range(self.num_levels):
            corr = self.corr_pyramid[i]
            dx = torch.linspace(-r, r, 2*r+1)
            dx = dx.view(2*r+1, 1).to(coords.device)
            x0 = dx + coords.reshape(batch*h1*w1, 1, 1, 1) / 2**i
            y0 = torch.zeros_like(x0)

            coords_lvl = torch.cat([x0,y0], dim=-1)
            corr = bilinear_sampler(corr, coords_lvl)
            corr = corr.view(batch, h1, w1, -1)
            out_pyramid.append(corr)

        out = torch.cat(out_pyramid, dim=-1)
        return out.permute(0, 3, 1, 2).contiguous().float()

    @staticmethod
    def corr(fmap1, fmap2):
        B, D, H, W1 = fmap1.shape
        _, _, _, W2 = fmap2.shape
        fmap1 = fmap1.view(B, D, H, W1)
        fmap2 = fmap2.view(B, D, H, W2)
        corr = torch.einsum('aijk,aijh->ajkh', fmap1, fmap2)
        corr = corr.reshape(B, H, W1, 1, W2).contiguous()
        return corr / torch.sqrt(torch.tensor(D).float())


class AlternateCorrBlock:
    def __init__(self, fmap1, fmap2, num_levels=4, radius=4):
        raise NotImplementedError
        self.num_levels = num_levels
        self.radius = radius

        self.pyramid = [(fmap1, fmap2)]
        for i in range(self.num_levels):
            fmap1 = F.avg_pool2d(fmap1, 2, stride=2)
            fmap2 = F.avg_pool2d(fmap2, 2, stride=2)
            self.pyramid.append((fmap1, fmap2))

    def __call__(self, coords):
        coords = coords.permute(0, 2, 3, 1)
        B, H, W, _ = coords.shape
        dim = self.pyramid[0][0].shape[1]

        corr_list = []
        for i in range(self.num_levels):
            r = self.radius
            fmap1_i = self.pyramid[0][0].permute(0, 2, 3, 1).contiguous()
            fmap2_i = self.pyramid[i][1].permute(0, 2, 3, 1).contiguous()

            coords_i = (coords / 2**i).reshape(B, 1, H, W, 2).contiguous()
            corr, = alt_cuda_corr.forward(fmap1_i, fmap2_i, coords_i, r)
            corr_list.append(corr.squeeze(1))

        corr = torch.stack(corr_list, dim=1)
        corr = corr.reshape(B, -1, H, W)
        return corr / torch.sqrt(torch.tensor(dim).float())


```

## Source file: lib/utils.py

{
  "path": "lib/utils.py",
  "blob_sha": "4ac4f851c100d71c7c2839146881acbd3c149f68",
  "sha256": "387524fd8bd54056caed26fed8d46554cc0fd8d8af8e76ebfaf5fdf608318642",
  "lines": 282,
  "bytes": 10891,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 2,
    "rig": 5,
    "face": 1
  },
  "url": "https://github.com/YaourtB/GPS_plus/blob/9469ee1c7c6532d2c20cbbc458dc11c63fea086b/lib/utils.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def modify_theta_phi(xyz, delta_phi=0, delta_theta=0):"
    },
    {
      "line": 26,
      "text": "def lookat(pos, target, up):"
    },
    {
      "line": 45,
      "text": "def modify_pitch(extr, look_at_center, down_direction, p=0):"
    },
    {
      "line": 57,
      "text": "def get_novel_calib_opencv(data, look_at_center=[0.0, 0.8, 0.0], down_direction=[0, -1, 0], ratio=0.5, p=0, intr_key='intr', extr_key='extr'):"
    },
    {
      "line": 88,
      "text": "def get_novel_calib_ndc(data, opt, look_at_center=[0.0, 0.8, 0.0], down_direction=[0, -1, 0], ratio=0.5, p=0, intr_key='intr', extr_key='extr'):"
    },
    {
      "line": 141,
      "text": "def depth2mesh(pts, depth):"
    },
    {
      "line": 165,
      "text": "def depth2pc(depth, extrinsic, intrinsic):"
    },
    {
      "line": 190,
      "text": "def flow2depth(data):"
    },
    {
      "line": 200,
      "text": "def depth_filter(depth):"
    },
    {
      "line": 210,
      "text": "def perspective(pts, calibs):"
    },
    {
      "line": 221,
      "text": "def depth_projection(pts, valid, target_intr, target_extr, S=1024):"
    },
    {
      "line": 243,
      "text": "def stereo_rectify(RS, extr0, intr0, extr1, intr1):"
    }
  ]
}

```text


import torch
import trimesh
import numpy as np
import cv2
from scipy.spatial.transform import Rotation as Rot
from scipy.spatial.transform import Slerp
import kornia
from lib.gs_utils.graphics_utils import getWorld2View2, getProjectionMatrix, focal2fov


def modify_theta_phi(xyz, delta_phi=0, delta_theta=0):

    x, y, z = xyz
    r = np.sqrt(x**2 + y**2 + z**2)
    phi = np.arctan2(y, (x**2 + z**2)**0.5) - delta_phi
    theta = np.arctan2(x, z) + delta_theta

    #  Spherical to Cartesian
    x_new = r * np.cos(phi) * np.cos(theta)
    y_new = r * np.sin(phi)
    z_new = r * np.cos(phi) * np.sin(theta)
    return np.array([x_new, y_new, z_new])


def lookat(pos, target, up):
    pos, target, up = np.array(pos), np.array(target), np.array(up)
    fwd = target - pos
    fwd_len = np.sqrt(np.sum(np.power(fwd, 2)))
    fwd = fwd / fwd_len

    right = np.cross(up, fwd)
    right_len = np.sqrt(np.sum(np.power(right, 2)))
    right = right / right_len
    up = np.cross(fwd, right)

    trans = np.array([right, up, fwd])
    pos = -trans.dot(np.array(pos))
    extr = np.zeros((3, 4))
    extr[:3, :3] = trans
    extr[:3, 3] = pos
    return extr


def modify_pitch(extr, look_at_center, down_direction, p=0):
    R = np.linalg.inv(extr[:3, :3])
    T = -np.matmul(R, extr[:3, 3])  # camera position

    T = modify_theta_phi(T-look_at_center, delta_phi=p*np.pi/180) + np.array(look_at_center)


    lookat_mat = lookat(pos=T, target=look_at_center, up=down_direction)

    return lookat_mat


def get_novel_calib_opencv(data, look_at_center=[0.0, 0.8, 0.0], down_direction=[0, -1, 0], ratio=0.5, p=0, intr_key='intr', extr_key='extr'):
    bs = data['lmain'][intr_key].shape[0]
    intr_list, extr_list = [], []
    data['novel_view'] = {}
    for i in range(bs):
        intr0 = data['lmain'][intr_key][i, ...].cpu().numpy()
        intr1 = data['rmain'][intr_key][i, ...].cpu().numpy()
        extr0 = data['lmain'][extr_key][i, ...].cpu().numpy()
        extr1 = data['rmain'][extr_key][i, ...].cpu().numpy()

        rot0 = extr0[:3, :3]
        rot1 = extr1[:3, :3]
        rots = Rot.from_matrix(np.stack([rot0, rot1]))
        key_times = [0, 1]
        slerp = Slerp(key_times, rots)
        rot = slerp(ratio)
        npose = np.diag([1.0, 1.0, 1.0, 1.0])
        npose = npose.astype(np.float32)
        npose[:3, :3] = rot.as_matrix()
        npose[:3, 3] = ((1.0 - ratio) * extr0 + ratio * extr1)[:3, 3]
        extr_new = npose[:3, :]
        if p != 0.0:
            extr_new = modify_pitch(extr_new, look_at_center=look_at_center, down_direction=down_direction, p=p)
        intr_new = ((1.0 - ratio) * intr0 + ratio * intr1)
        intr_list.append(intr_new)
        extr_list.append(extr_new)
    data['novel_view']['intr'] = torch.FloatTensor(np.array(intr_list)).cuda()
    data['novel_view']['extr'] = torch.FloatTensor(np.array(extr_list)).cuda()
    return data


def get_novel_calib_ndc(data, opt, look_at_center=[0.0, 0.8, 0.0], down_direction=[0, -1, 0], ratio=0.5, p=0, intr_key='intr', extr_key='extr'):
    bs = data['lmain'][intr_key].shape[0]
    fovx_list, fovy_list, world_view_transform_list, full_proj_transform_list, camera_center_list = [], [], [], [], []
    for i in range(bs):
        intr0 = data['lmain'][intr_key][i, ...].cpu().numpy()
        intr1 = data['rmain'][intr_key][i, ...].cpu().numpy()
        extr0 = data['lmain'][extr_key][i, ...].cpu().numpy()
        extr1 = data['rmain'][extr_key][i, ...].cpu().numpy()

        rot0 = extr0[:3, :3]
        rot1 = extr1[:3, :3]
        rots = Rot.from_matrix(np.stack([rot0, rot1]))
        key_times = [0, 1]
        slerp = Slerp(key_times, rots)
        rot = slerp(ratio)
        npose = np.diag([1.0, 1.0, 1.0, 1.0])
        npose = npose.astype(np.float32)
        npose[:3, :3] = rot.as_matrix()
        npose[:3, 3] = ((1.0 - ratio) * extr0 + ratio * extr1)[:3, 3]
        extr_new = npose[:3, :]
        if p != 0.0:
            extr_new = modify_pitch(extr_new, look_at_center=look_at_center, down_direction=down_direction, p=p)
        intr_new = ((1.0 - ratio) * intr0 + ratio * intr1)

        if opt.use_hr_img:
            intr_new[:2] *= 2
        width, height = data['novel_view']['width'][i], data['novel_view']['height'][i]
        R = np.array(extr_new[:3, :3], np.float32).reshape(3, 3).transpose(1, 0)
        T = np.array(extr_new[:3, 3], np.float32)

        FovX = focal2fov(intr_new[0, 0], width)
        FovY = focal2fov(intr_new[1, 1], height)
        projection_matrix = getProjectionMatrix(znear=opt.znear, zfar=opt.zfar, fovX=FovX, fovY=FovY,
                                                K=intr_new, h=height, w=width).transpose(0, 1)
        world_view_transform = torch.tensor(getWorld2View2(R, T, np.array(opt.trans), opt.scale)).transpose(0, 1)
        full_proj_transform = (world_view_transform.unsqueeze(0).bmm(projection_matrix.unsqueeze(0))).squeeze(0)
        camera_center = world_view_transform.inverse()[3, :3]

        fovx_list.append(FovX)
        fovy_list.append(FovY)
        world_view_transform_list.append(world_view_transform.unsqueeze(0))
        full_proj_transform_list.append(full_proj_transform.unsqueeze(0))
        camera_center_list.append(camera_center.unsqueeze(0))

    data['novel_view']['FovX'] = torch.FloatTensor(np.array(fovx_list)).cuda()
    data['novel_view']['FovY'] = torch.FloatTensor(np.array(fovy_list)).cuda()
    data['novel_view']['world_view_transform'] = torch.concat(world_view_transform_list).cuda()
    data['novel_view']['full_proj_transform'] = torch.concat(full_proj_transform_list).cuda()
    data['novel_view']['camera_center'] = torch.concat(camera_center_list).cuda()
    return data


# https://github.com/yasaminjafarian/HDNet_TikTok/blob/main/utils.py
def depth2mesh(pts, depth):
    S = depth.shape[3]

    pts = pts.view(S, S, 3).detach().cpu().numpy()
    diff_depth = depth_filter(depth)
    depth[diff_depth > 0.04] = 0
    valid_depth = depth != 0
    valid_depth = kornia.morphology.erosion(valid_depth.float(), torch.ones(3, 3).cuda())
    valid_depth = valid_depth.squeeze().detach().cpu().numpy()

    f_list = []
    for i in range(S-1):
        for j in range(S-1):
            if valid_depth[i, j] and valid_depth[i+1, j+1]:
                if valid_depth[i+1, j]:
                    f_list.append([int(i*S+j+1), int((i+1)*S+j+1), int((i+1)*S+j+2)])
                if valid_depth[i, j+1]:
                    f_list.append([int(i*S+j+1), int((i+1)*S+j+2), int(i*S+j+2)])

    obj_out = trimesh.Trimesh(vertices=pts.reshape(-1, 3), faces=np.array(f_list))

    return obj_out


def depth2pc(depth, extrinsic, intrinsic):
    B, C, H, W = depth.shape
    depth = depth[:, 0, :, :]
    rot = extrinsic[:, :3, :3]
    trans = extrinsic[:, :3, 3:]

    y, x = torch.meshgrid(torch.linspace(0.5, H-0.5, H, device=depth.device), torch.linspace(0.5, W-0.5, W, device=depth.device))
    pts_2d = torch.stack([x, y, torch.ones_like(x)], dim=-1).unsqueeze(0).repeat(B, 1, 1, 1)  # B S S 3

    pts_2d[..., 2] = 1.0 / (depth + 1e-8)
    pts_2d[:, :, :, 0] -= intrinsic[:, None, None, 0, 2]
    pts_2d[:, :, :, 1] -= intrinsic[:, None, None, 1, 2]
    pts_2d_xy = pts_2d[:, :, :, :2] * pts_2d[:, :, :, 2:]
    pts_2d = torch.cat([pts_2d_xy, pts_2d[..., 2:]], dim=-1)

    pts_2d[..., 0] /= intrinsic[:, 0, 0][:, None, None]
    pts_2d[..., 1] /= intrinsic[:, 1, 1][:, None, None]

    pts_2d = pts_2d.view(B, -1, 3).permute(0, 2, 1)
    rot_t = rot.permute(0, 2, 1)
    pts = torch.bmm(rot_t, pts_2d) - torch.bmm(rot_t, trans)

    return pts.permute(0, 2, 1)


def flow2depth(data):
    offset = data['ref_intr'][:, 0, 2] - data['intr'][:, 0, 2]
    offset = torch.broadcast_to(offset[:, None, None, None], data['flow_pred'].shape)
    disparity = offset - data['flow_pred']
    depth = -disparity / data['Tf_x'][:, None, None, None]
    #depth *= data['mask'][:, :1, :, :]

    return depth


def depth_filter(depth):
    diff_depth = torch.zeros_like(depth)
    diff_depth[..., 1:-1, 1:-1] += torch.abs(depth[..., 1:, :] - depth[..., :-1, :])[..., :-1, 1:-1]
    diff_depth[..., 1:-1, 1:-1] += torch.abs(depth[..., :-1, :] - depth[..., 1:, :])[..., 1:, 1:-1]
    diff_depth[..., 1:-1, 1:-1] += torch.abs(depth[..., :, 1:] - depth[..., :, :-1])[..., 1:-1, :-1]
    diff_depth[..., 1:-1, 1:-1] += torch.abs(depth[..., :, :-1] - depth[..., :, 1:])[..., 1:-1, 1:]

    return diff_depth


def perspective(pts, calibs):
    # pts: [B, N, 3]
    # calibs: [B, 3, 4]
    pts = pts.permute(0, 2, 1)
    pts = torch.bmm(calibs[:, :3, :3], pts)
    pts = pts + calibs[:, :3, 3:4]
    pts[:, :2, :] /= pts[:, 2:, :]
    pts = pts.permute(0, 2, 1)
    return pts


def depth_projection(pts, valid, target_intr, target_extr, S=1024):
    '''
    input:
        pts: [B, S*S, 3]
        valid: [B, S*S]
    output:
        depth_out: [B, 1, S, S]
    '''
    target_calib = torch.matmul(target_intr, target_extr)
    pts_valid = torch.zeros_like(pts)
    pts_valid[valid] = pts[valid]

    pts_valid = perspective(pts_valid, target_calib)
    pts_valid[:, :, 2:] = 1.0 / (pts_valid[:, :, 2:] + 1e-8)
    pts_valid[:, :, :2] = torch.clamp(pts_valid[:, :, :2], 0, S-1)

    B = pts.shape[0]
    depth_out = torch.zeros([B, 1, S, S], device=pts.device)
    depth_out[:, 0, pts_valid[:, :, 1].int(), pts_valid[:, :, 0].int()] = pts_valid[:, :, 2]

    return depth_out.detach()

def stereo_rectify(RS, extr0, intr0, extr1, intr1):
    # return extr0, intr0, extr1, intr1, image_list0, image_list1, np.zeros((3, 4))
    H, W, _ = RS, RS, 3
    r0 = extr0[:3, :3]
    t0 = extr0[:3, 3:]
    r1 = extr1[:3, :3]
    t1 = extr1[:3, 3:]
    inv_r0 = r0.T
    inv_t0 = - r0.T @ t0
    E0 = np.eye(4)
    E0[:3, :3] = inv_r0
    E0[:3, 3:] = inv_t0
    E1 = np.eye(4)
    E1[:3, :3] = r1
    E1[:3, 3:] = t1
    E = E1 @ E0
    R = E[:3, :3]
    T = E[:3, 3]
    dist0 = np.zeros(4)
    dist1 = np.zeros(4)
    
    R0, R1, P0, P1, Q, validPixROI0, validPixROI1 = cv2.stereoRectify(intr0, dist0, intr1, dist1, (W, H), R, T, flags=0)

    #print(R0, R1, P0, P1, Q, validPixROI0, validPixROI1)

    new_extr0 = R0 @ extr0
    new_intr0 = P0[:3, :3]
    new_extr1 = R1 @ extr1
    new_intr1 = P1[:3, :3]

    # import matplotlib.pyplot as plt
    recity_mat0_x, recity_mat0_y = cv2.initUndistortRectifyMap(intr0, dist0, R0, P0, (W, H), cv2.CV_32FC1)
    # plt.imshow(recity_mat0_x)
    # plt.show()
    # for i in range(len(image_list0)):
    #     image_list0[i] = cv2.remap(image_list0[i], recity_mat0_x, recity_mat0_y, cv2.INTER_LINEAR)
    recity_mat1_x, recity_mat1_y = cv2.initUndistortRectifyMap(intr1, dist1, R1, P1, (W, H), cv2.CV_32FC1)
    # for i in range(len(image_list0)):
    #     image_list1[i] = cv2.remap(image_list1[i], recity_mat1_x, recity_mat1_y, cv2.INTER_LINEAR)
    return new_extr0, new_intr0, new_extr1, new_intr1, recity_mat0_x, recity_mat0_y, recity_mat1_x, recity_mat1_y, P1


```

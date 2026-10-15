# Facial rendering source scan: CRazorback/LDDMM-Face

Repository: https://github.com/CRazorback/LDDMM-Face

Pinned source revision: cb45e96abbd40dc57560a9f44550e697a5d4f878

Discovery topic: face deformation

Description: [PR'24] "LDDMM-Face: Large deformation diffeomorphic metric learning for cross-annotation face alignment".

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hourglass.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hourglass_pip.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt44.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt50.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt50_beta1e-2.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt50_beta1e0.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_cl01.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_cosine.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_cosine_large.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_curve11_softmax.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_curve9.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_finetune.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt68_large.yaml", "experiments/300w/hrnet_lddmm/face_alignment_300w_lddmm_hrnet_w18_pt72.yaml", "experiments/300w/hrnet_pretrained/face_alignment_300w_hrnet_w18_pt44.yaml", "experiments/300w/preliminary/face_alignment_300w_coordinate_hourglass.yaml", "experiments/300w/preliminary/face_alignment_300w_coordinate_hrnet_w18.yaml", "experiments/300w/preliminary/face_alignment_300w_dan.yaml", "experiments/300w/preliminary/face_alignment_300w_hourglass.yaml", "experiments/300w/preliminary/face_alignment_300w_hrnet_w18.yaml", "experiments/300w/preliminary/face_alignment_300w_hrnet_w18_pt50.yaml", "experiments/300w/preliminary/face_alignment_300w_lddmm_dan.yaml", "experiments/aflw/face_alignment_aflw_hrnet_w18.yaml", "experiments/cofw/face_alignment_cofw_hrnet_w18.yaml", "experiments/helen/face_alignment_helen_coordinate_hourglass.yaml", "experiments/helen/face_alignment_helen_coordinate_hrnet_w18.yaml", "experiments/helen/face_alignment_helen_dan.yaml", "experiments/helen/face_alignment_helen_hourglass_pt194.yaml", "experiments/helen/face_alignment_helen_hrnet_w18_pt194.yaml"]}

## Source file: lib/datasets/face300w.py

{
  "path": "lib/datasets/face300w.py",
  "blob_sha": "25efbb747cfd12e9a0e05dffdbd203e204469c99",
  "sha256": "a40489a4ae1b7bb5b71b651d5b6f02263e5847adfec6f4f533d155f39cd4b680",
  "lines": 163,
  "bytes": 6399,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 2,
    "rig": 3,
    "face": 4,
    "lip": 5
  },
  "url": "https://github.com/CRazorback/LDDMM-Face/blob/cb45e96abbd40dc57560a9f44550e697a5d4f878/lib/datasets/face300w.py",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "def __init__(self, cfg, is_train=True, transform=None):"
    },
    {
      "line": 53,
      "text": "def __len__(self):"
    },
    {
      "line": 59,
      "text": "def __getitem__(self, idx):"
    }
  ]
}

```text

# ------------------------------------------------------------------------------
# Copyright (c) Microsoft
# Licensed under the MIT License.
# Created by Tianheng Cheng(tianhengcheng@gmail.com), Yang Zhao
# ------------------------------------------------------------------------------

import os
import random
import itertools
import imageio

import torch
import torch.nn as nn
import torch.utils.data as data
import pandas as pd
from PIL import Image
import numpy as np

from lib.utils.transforms import fliplr_joints, crop, generate_target, transform_pixel
from lib.utils.lddmm_params import get_index


class Face300W(data.Dataset):
    def __init__(self, cfg, is_train=True, transform=None):
        # specify annotation file for dataset
        if is_train:
            self.csv_file = cfg.DATASET.TRAINSET
        else:
            self.csv_file = cfg.DATASET.TESTSET

        self.is_train = is_train
        self.transform = transform
        self.data_root = cfg.DATASET.ROOT
        self.dataset_name = '300W'
        self.input_size = cfg.MODEL.IMAGE_SIZE
        self.output_size = cfg.MODEL.HEATMAP_SIZE
        self.points = cfg.MODEL.NUM_JOINTS if is_train else cfg.TEST.NUM_JOINTS
        self.sigma = cfg.MODEL.SIGMA
        self.scale_factor = cfg.DATASET.SCALE_FACTOR
        self.bounding_box_scale_factor = cfg.DATASET.BOUNDINGBOX_SCALE_FACTOR
        self.rot_factor = cfg.DATASET.ROT_FACTOR
        self.label_type = cfg.MODEL.TARGET_TYPE
        self.flip = cfg.DATASET.FLIP

        # load annotations
        self.landmarks_frame = pd.read_csv(self.csv_file)

        self.mean = np.array([0.485, 0.456, 0.406], dtype=np.float32)
        self.std = np.array([0.229, 0.224, 0.225], dtype=np.float32)

        self.index = get_index('300W', self.points)

    def __len__(self):
        if self.is_train:
            return len(self.landmarks_frame) * 1
        else:
            return len(self.landmarks_frame)

    def __getitem__(self, idx):
        # idx = 2341
        # idx = idx % len(self.landmarks_frame)
        
        image_path = os.path.join(self.data_root,
                                  self.landmarks_frame.iloc[idx, 0])
        scale = self.landmarks_frame.iloc[idx, 1]

        center_w = self.landmarks_frame.iloc[idx, 2]
        center_h = self.landmarks_frame.iloc[idx, 3]
        center = torch.Tensor([center_w, center_h])

        pts = self.landmarks_frame.iloc[idx, 4:].values
        pts = pts.astype('float').reshape(-1, 2)

        # DAN scale or HRNet scale
        scale *= self.bounding_box_scale_factor
        nparts = pts.shape[0]
        img = np.array(Image.open(image_path).convert('RGB'), dtype=np.float32)

        r = 0
        if self.is_train:
            scale = scale * (random.uniform(1 - self.scale_factor,
                                            1 + self.scale_factor))
            r = random.uniform(-self.rot_factor, self.rot_factor) \
                if random.random() <= 0.6 else 0
            if random.random() <= 0.5 and self.flip:
                img = np.fliplr(img)
                pts = fliplr_joints(pts, width=img.shape[1], dataset='300W')
                center[0] = img.shape[1] - center[0]

        img = crop(img, center, scale, self.input_size, rot=r)
        if self.label_type == 'Gaussian':
            target = np.zeros((nparts, self.output_size[0], self.output_size[1]))
        tpts = pts.copy()

        for i in range(nparts):
            if tpts[i, 1] > 0:
                tpts[i, 0:2] = transform_pixel(tpts[i, 0:2]+1, center,
                                               scale, self.output_size, rot=r)
                if self.label_type == 'Gaussian':                               
                    target[i] = generate_target(target[i], tpts[i]-1, self.sigma,
                                                label_type=self.label_type)

        # draw figure    
        # if 'image_0324.png' in image_path:
        #     import imageio
        #     import cv2
        #     img1, img2 = img.copy(), img.copy()
        #     tpts = tpts.astype(np.uint8)
        #     for k in range(tpts.shape[0]):
        #         cv2.circle(img1, (tpts[k, 0], tpts[k, 1]), 3, [0, 255, 0], -1)
        #     imageio.imwrite('keypoint_img.jpg'.format(idx), img1)
        #     tpts_ = tpts[self.index]
        #     for k in range(tpts_.shape[0]):
        #         cv2.circle(img2, (tpts_[k, 0], tpts_[k, 1]), 3, [0, 255, 0], -1)
        #     imageio.imwrite('keypoint_img_weak.jpg'.format(idx), img2)
        #     print(idx)

        img = img.astype(np.float32)
        img = (img / 255.0 - self.mean) / self.std
        img = img.transpose([2, 0, 1])
        if self.label_type == 'Gaussian':
            target = torch.Tensor(target)
        tpts = torch.Tensor(tpts)
        center = torch.Tensor(center)
        origin_pts = torch.Tensor(pts)

        # weak-supervised
        if self.points < 68:
            tpts = tpts[self.index]
            pts = pts[self.index]
            if self.label_type == 'Gaussian':
                target = target[self.index]

        meta = {'index': idx, 'center': center, 'scale': scale, 'dataset_name': self.dataset_name,
                'pts': torch.Tensor(pts), 'tpts': tpts, 'origin_pts': origin_pts}

        if self.label_type == 'Gaussian':
            return img, target, meta
        else:
            return img, tpts, meta


if __name__ == '__main__':
    import argparse
    from lib.config import config, update_config
    parser = argparse.ArgumentParser(description='Train Face Alignment')
    parser.add_argument('--cfg', help='experiment configuration filename',
                        required=True, type=str)
    args = parser.parse_args()
    update_config(config, args)
    config.DATASET.TESTSET = 'data/300w/face_landmarks_300w_train.csv'
    config.MODEL.NUM_JOINTS = 68
    config.MODEL.HEATMAP_SIZE = 112
    config.MODEL.TARGET_TYPE = 'Landmark'
    dataset = Face300W(config, is_train=False)
    landmarks = torch.zeros([len(dataset), 68, 2])
    sigmaV = torch.zeros([12])
    sigmaW = torch.zeros([12])
    for i, (a, b, c) in enumerate(dataset):
        for curve_idx, landmark_idxs in dataset.curve2landmark.items():
            sigmaV[curve_idx] += torch.max(b[landmark_idxs, 0]) - torch.min(b[landmark_idxs, 0])
            sigmaW[curve_idx] += torch.max(b[landmark_idxs, 1]) - torch.min(b[landmark_idxs, 1])
    

```

## Source file: invoker.py

{
  "path": "invoker.py",
  "blob_sha": "f94fe126d304d176b74a644cb4f2a68e31f323b6",
  "sha256": "0bdb151db436e44ecb2f1217e3c37e4bd53f32b9baea95fcbf8dbf98fc68e2f6",
  "lines": 83,
  "bytes": 2701,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 2,
    "face": 3
  },
  "url": "https://github.com/CRazorback/LDDMM-Face/blob/cb45e96abbd40dc57560a9f44550e697a5d4f878/invoker.py",
  "function_signature_candidates": []
}

```text

import argparse
from lib.config import config, update_config
import sys
import numpy as np
import scipy.io
from lib.datasets import WFLW, Face300W

# parser = argparse.ArgumentParser(description='Train Face Alignment')
# parser.add_argument('--cfg', help='experiment configuration filename',
#                     required=True, type=str)
# args = parser.parse_args()
# update_config(config, args)
# # dataset = WFLW(config, is_train=False)
# # landmarks = np.zeros([len(dataset), 98, 2])
# dataset = Face300W(config, is_train=False)
# landmarks = np.zeros([3149, 68, 2])
# for i, (a, b, c) in enumerate(dataset):
#     landmarks[i] = c['tpts']

# mean_shape = np.mean(landmarks, dim=0)
# print(mean_shape)

# np.save('./data/300w/init_landmark.npy', mean_shape.numpy())
# curve2landmark = {
#     0: np.arange(0, 9),
#     1: np.arange(9, 17),
#     2: np.arange(17, 22),
#     3: np.arange(22, 27),
#     4: np.arange(27, 31),
#     5: np.arange(31, 36),
#     6: np.arange(36, 42),
#     7: np.arange(42, 48),
#     8: np.arange(48, 55),
#     9: np.arange(55, 62),
#     10: np.arange(62, 67),
#     11: np.arange(67, 72)}
# init_landmarks = np.load('./data/init_landmark.npy')
# index = list(np.arange(0, 48)) + list(np.arange(48, 55)) + list(np.arange(54, 60)) +\
#                     [48] + list(np.arange(60, 65)) + list(np.arange(64, 68)) + [60]
# init_landmarks = init_landmarks[index]
# curve2landmark = {
#     0: np.arange(0, 17),
#     1: np.arange(17, 33),
#     2: np.arange(33, 42),
#     3: np.arange(42, 51),
#     4: np.arange(51, 55),
#     5: np.arange(55, 60),
#     6: np.arange(60, 68),
#     7: np.arange(68, 76),
#     8: np.arange(76, 83),
#     9: np.arange(83, 88),
#     10: np.arange(88, 93),
#     11: np.arange(93, 96)
# }
# init_landmarks = np.load('data/wflw/init_landmark.npy')
curve2landmark = {
    0: np.arange(0, 21),
    1: np.arange(21, 41),
    2: np.arange(41, 58),
    3: np.arange(58, 72),
    4: np.arange(72, 86),
    5: np.arange(86, 100),
    6: np.arange(100, 114),
    7: np.arange(114, 134),
    8: np.arange(134, 154),
    9: np.arange(154, 174),
    10: np.arange(174, 194)}
init_landmarks = scipy.io.loadmat('data/300w/images/helen/Helen_meanShape_256_1_5x.mat')['Helen_meanShape_256_1_5x']
init_landmarks *= (112 / 256)

sigmaV = np.zeros([11])
sigmaW = np.zeros([11])
for curve_idx, landmark_idxs in curve2landmark.items():
    landmarks = init_landmarks[landmark_idxs]
    x = landmarks[:, 0]
    y = landmarks[:, 1]
    max_x = np.max(landmarks[:, 0])
    min_x = np.min(landmarks[:, 0])
    max_y = np.max(landmarks[:, 1])
    min_y = np.min(landmarks[:, 1])
    sigmaV[curve_idx] += np.mean([max_x - min_x, max_y - min_y])

print(sigmaV)

```

## Source file: tools/train.py

{
  "path": "tools/train.py",
  "blob_sha": "851ac982b15357bc2f9ca190c66c9a7ee642c16e",
  "sha256": "febbfab7dde08839ffdfa981d906f887f2c2b6d91489f2da9ce8c31fb05ccd0c",
  "lines": 160,
  "bytes": 4979,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 2,
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/CRazorback/LDDMM-Face/blob/cb45e96abbd40dc57560a9f44550e697a5d4f878/tools/train.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def parse_args():"
    },
    {
      "line": 39,
      "text": "def main():"
    }
  ]
}

```text

# ------------------------------------------------------------------------------
# Copyright (c) Microsoft
# Licensed under the MIT License.
# Created by Tianheng Cheng(tianhengcheng@gmail.com)
# ------------------------------------------------------------------------------

import os
import pprint
import argparse

import torch
import torch.nn as nn
import torch.optim as optim
import torch.backends.cudnn as cudnn
from tensorboardX import SummaryWriter
from torch.utils.data import DataLoader
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import lib.models as models
from lib.config import config, update_config
from lib.datasets import get_dataset
from lib.core import function
from lib.utils import utils
from lib.core.evaluation import LDDMMError


def parse_args():

    parser = argparse.ArgumentParser(description='Train Face Alignment')

    parser.add_argument('--cfg', help='experiment configuration filename',
                        required=True, type=str)

    args = parser.parse_args()
    update_config(config, args)
    return args


def main():

    args = parse_args()

    logger, final_output_dir, tb_log_dir = \
        utils.create_logger(config, args.cfg, 'train')

    logger.info(pprint.pformat(args))
    logger.info(pprint.pformat(config))

    cudnn.benchmark = config.CUDNN.BENCHMARK
    cudnn.determinstic = config.CUDNN.DETERMINISTIC
    cudnn.enabled = config.CUDNN.ENABLED

    model = models.get_face_alignment_net(config)

    # copy model files
    writer_dict = {
        'writer': SummaryWriter(log_dir=tb_log_dir),
        'train_global_steps': 0,
        'valid_global_steps': 0,
    }

    gpus = list(config.GPUS)
    model = nn.DataParallel(model, device_ids=gpus).cuda()

    # loss
    if 'lddmm' in config.MODEL['NAME']:
        criterion = LDDMMError(config).cuda()
    elif 'coord' in config.MODEL['NAME'] or 'dan' in config.MODEL['NAME']:
        criterion = LDDMMError(config, curve=False).cuda()
    else:
        criterion = nn.MSELoss()

    optimizer = utils.get_optimizer(config, model)
    best_nme = 100
    last_epoch = config.TRAIN.BEGIN_EPOCH
    if config.TRAIN.RESUME:
        model_state_file = config.TRAIN.CHECKPOINT
        if os.path.isfile(model_state_file):
            checkpoint = torch.load(model_state_file)
            last_epoch = checkpoint['epoch']
            best_nme = checkpoint['best_nme']
            model.load_state_dict(checkpoint['state_dict'].state_dict())
            optimizer.load_state_dict(checkpoint['optimizer'])
            print("=> loaded checkpoint (epoch {})"
                  .format(checkpoint['epoch']))
        else:
            print("=> no checkpoint found")

    if isinstance(config.TRAIN.LR_STEP, list):
        lr_scheduler = torch.optim.lr_scheduler.MultiStepLR(
            optimizer, config.TRAIN.LR_STEP,
            config.TRAIN.LR_FACTOR, last_epoch-1
        )
    else:
        lr_scheduler = torch.optim.lr_scheduler.StepLR(
            optimizer, config.TRAIN.LR_STEP,
            config.TRAIN.LR_FACTOR, last_epoch-1
        )
    dataset_type = get_dataset(config)

    train_loader = DataLoader(
        dataset=dataset_type(config,
                             is_train=True),
        batch_size=config.TRAIN.BATCH_SIZE_PER_GPU*len(gpus),
        shuffle=config.TRAIN.SHUFFLE,
        num_workers=config.WORKERS,
        pin_memory=config.PIN_MEMORY)

    val_loader = DataLoader(
        dataset=dataset_type(config,
                             is_train=False),
        batch_size=config.TEST.BATCH_SIZE_PER_GPU*len(gpus),
        shuffle=False,
        num_workers=config.WORKERS,
        pin_memory=config.PIN_MEMORY
    )

    for epoch in range(last_epoch, config.TRAIN.END_EPOCH):
        lr_scheduler.step()
        # utils.adjust_learning_rate(optimizer, last_epoch, config.TRAIN.END_EPOCH, config.TRAIN.LR)

        function.train(config, train_loader, model, criterion,
                       optimizer, epoch, writer_dict)

        # evaluate
        nme, predictions = function.validate(config, val_loader, model,
                                             criterion, epoch, writer_dict)

        is_best = nme < best_nme
        best_nme = min(nme, best_nme)

        logger.info('=> saving checkpoint to {}'.format(final_output_dir))
        print("best:", is_best)
        utils.save_checkpoint(
            {"state_dict": model,
             "epoch": epoch + 1,
             "best_nme": best_nme,
             "optimizer": optimizer.state_dict(),
             }, predictions, is_best, final_output_dir, 'checkpoint_{}.pth'.format(epoch))

    final_model_state_file = os.path.join(final_output_dir,
                                          'final_state.pth')
    logger.info('saving final model state to {}'.format(
        final_model_state_file))
    torch.save(model.module.state_dict(), final_model_state_file)
    writer_dict['writer'].close()


if __name__ == '__main__':
    main()












```

## Source file: lib/__init__.py

{
  "path": "lib/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/CRazorback/LDDMM-Face/blob/cb45e96abbd40dc57560a9f44550e697a5d4f878/lib/__init__.py",
  "function_signature_candidates": []
}

```text



```

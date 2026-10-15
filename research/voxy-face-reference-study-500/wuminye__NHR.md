# Facial rendering source scan: wuminye/NHR

Repository: https://github.com/wuminye/NHR

Pinned source revision: e9ef87603b153e58a75f1b166a4c0687c547d223

Discovery topic: human rendering

Description: Pytorch implementation of  Multi-view Neural Human Rendering.

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.md"]

Topic paths: {}

## Source file: data/build.py

{
  "path": "data/build.py",
  "blob_sha": "534baf4f630f944e4faa13caf1cc7f305ebd869b",
  "sha256": "0d5acf648b6a1829713cfab661ced81de892b689f53debdb6882b132f6a948c2",
  "lines": 52,
  "bytes": 1623,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/wuminye/NHR/blob/e9ef87603b153e58a75f1b166a4c0687c547d223/data/build.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def build_dataset(cfg, transforms,  is_train=True, use_mask = False, is_need_all_data = False):"
    },
    {
      "line": 34,
      "text": "def make_data_loader(cfg, is_train=True, is_need_all_data = False, is_center =False):"
    }
  ]
}

```text

# encoding: utf-8
import os
import torch
from torch.utils import data

import logging


from .datasets import IBRDynamicDataset
from .datasets import IBRDynamicTotalDataset
from .transforms import build_transforms
from .collate_batch import static_collate

def build_dataset(cfg, transforms,  is_train=True, use_mask = False, is_need_all_data = False):

    if is_train:
        datasets = IBRDynamicTotalDataset(cfg, transforms,is_train)
        if cfg.DATASETS.RANDOM_NOISY>0:
            if cfg.MODEL.RESUME_EPOCH > 0 :
                datasets.load_vs(cfg)
            else:
                datasets.save_vs(cfg)
    else:
        datasets = IBRDynamicTotalDataset(cfg, transforms,is_train)
        if cfg.DATASETS.RANDOM_NOISY>0:
            datasets.load_vs(cfg)


    logger = logging.getLogger("rendering_model.train")
    logger.info("Load {} datasets.".format(len(datasets.datasets)))
    return datasets


def make_data_loader(cfg, is_train=True, is_need_all_data = False, is_center =False):
    
    batch_size = cfg.SOLVER.IMS_PER_BATCH


    transforms = build_transforms(cfg, is_train,is_center = is_center)
    datasets = build_dataset(cfg, transforms,  is_train, is_need_all_data)

    num_workers = cfg.DATALOADER.NUM_WORKERS
    if is_train:
        data_loader = data.DataLoader(
            datasets, collate_fn=static_collate, batch_size=batch_size, shuffle=True, num_workers=num_workers
        )
    else:
        data_loader = data.DataLoader(
            datasets, collate_fn=static_collate, batch_size=batch_size, shuffle=True, num_workers=num_workers
        )

    return data_loader, datasets


```

## Source file: layers/build.py

{
  "path": "layers/build.py",
  "blob_sha": "532729ea928c341c4ef085f20fdbef5ee806bda0",
  "sha256": "744d347dc5bc63fc6990a5061854f88e779f52d2ed0561cd6777b97a321ff71e",
  "lines": 6,
  "bytes": 112,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/wuminye/NHR/blob/e9ef87603b153e58a75f1b166a4c0687c547d223/layers/build.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def make_loss(cfg):"
    }
  ]
}

```text

import torch
from .perceptual_loss import Perceptual_loss


def make_loss(cfg):
    return Perceptual_loss(cfg)


```

## Source file: solver/build.py

{
  "path": "solver/build.py",
  "blob_sha": "287f57481bec722b171ead66d51006f379372807",
  "sha256": "6a9d4c2110528dc7c3925bd09250f45c9546a1ef3738b4a3f963c9c38515c4a4",
  "lines": 42,
  "bytes": 1487,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/wuminye/NHR/blob/e9ef87603b153e58a75f1b166a4c0687c547d223/solver/build.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def find_pretrain_file(path, resume_point):"
    },
    {
      "line": 18,
      "text": "def make_optimizer(cfg, model, pre_load = True):"
    }
  ]
}

```text

# encoding: utf-8


import torch
import logging
import os

def find_pretrain_file(path, resume_point):
    files = os.listdir(path)
    files = [i for i in files if 'nr_optimizer' in i]
    files.sort()

    if ('nr_optimizer_%d.pth' %resume_point) not in files:
        return None
    return 'nr_optimizer_%d.pth' % resume_point


def make_optimizer(cfg, model, pre_load = True):
    params = []
    for key, value in model.named_parameters():
        if not value.requires_grad:
            continue
        lr = cfg.SOLVER.BASE_LR
        weight_decay = cfg.SOLVER.WEIGHT_DECAY
        if "bias" in key:
            lr = cfg.SOLVER.BASE_LR * cfg.SOLVER.BIAS_LR_FACTOR
            weight_decay = cfg.SOLVER.WEIGHT_DECAY_BIAS
        params += [{"params": [value], "lr": lr, "weight_decay": weight_decay}]
        
    if cfg.SOLVER.OPTIMIZER_NAME == 'SGD':
        optimizer = getattr(torch.optim, cfg.SOLVER.OPTIMIZER_NAME)(params, momentum=cfg.SOLVER.MOMENTUM)
    else:
        optimizer = getattr(torch.optim, cfg.SOLVER.OPTIMIZER_NAME)(params)

    if pre_load:
        pretrain_optimizer = find_pretrain_file(cfg.OUTPUT_DIR, cfg.MODEL.RESUME_EPOCH)
        if pretrain_optimizer is not None:
            logger = logging.getLogger("rendering_model.train")
            logger.info("Load pretrain optimizer {}.".format(pretrain_optimizer))
            optimizer.load_state_dict(torch.load(os.path.join(cfg.OUTPUT_DIR,pretrain_optimizer),map_location='cpu'))

    return optimizer


```

## Source file: utils/logger.py

{
  "path": "utils/logger.py",
  "blob_sha": "a9c32e71701a780dcbd856b064d4f7c580afacc1",
  "sha256": "07dc09ab6da13f55ec4c27e1253b92becbe477ffa369665626a8a471b61a383a",
  "lines": 30,
  "bytes": 783,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/wuminye/NHR/blob/e9ef87603b153e58a75f1b166a4c0687c547d223/utils/logger.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def setup_logger(name, save_dir, distributed_rank):"
    }
  ]
}

```text

# encoding: utf-8
"""
@author:  sherlock
@contact: sherlockliao01@gmail.com
"""

import logging
import os
import sys


def setup_logger(name, save_dir, distributed_rank):
    logger = logging.getLogger(name)
    logger.setLevel(logging.DEBUG)
    # don't log results for the non-master process
    if distributed_rank > 0:
        return logger
    ch = logging.StreamHandler(stream=sys.stdout)
    ch.setLevel(logging.DEBUG)
    formatter = logging.Formatter("%(asctime)s %(name)s %(levelname)s: %(message)s")
    ch.setFormatter(formatter)
    logger.addHandler(ch)

    if save_dir:
        fh = logging.FileHandler(os.path.join(save_dir, "log.txt"), mode='w')
        fh.setLevel(logging.DEBUG)
        fh.setFormatter(formatter)
        logger.addHandler(fh)

    return logger


```

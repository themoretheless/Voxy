# Facial rendering source scan: huifu99/Mimic

Repository: https://github.com/huifu99/Mimic

Pinned source revision: 9e71299fc041a232e37ac79fbb4dff0b0552c20e

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "external/spectre/LICENSE"]

Topic paths: {"flame": ["external/spectre/src/models/FLAME.py"], "lbs": ["external/spectre/src/models/lbs.py"], "lip": ["external/spectre/configs/lipread_config.ini", "external/spectre/utils/lipread_utils.py"]}

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "c4d59daaf37c52795a490c79c6342888ae049219",
  "sha256": "f43d2bfda356ddaf4cbff4f93071359c09a962f88d9a7087889012d1ed392439",
  "lines": 150,
  "bytes": 5911,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/huifu99/Mimic/blob/9e71299fc041a232e37ac79fbb4dff0b0552c20e/train.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def main():"
    }
  ]
}

```text

import os
# set device
os.environ['CUDA_VISIBLE_DEVICES'] = '0'  # single GPU for training
# os.environ['TRANSFORMERS_OFFLINE'] = '1'
import datetime
import numpy as np
import copy
import cv2
import torch
import torch.nn as nn
from torch.utils.tensorboard import SummaryWriter
from base.utilities import get_logger, get_parser, fixed_seed
"model"
from models.network import DisNetAutoregCycle as Model
"data"
from data.dataloader_HDTF import get_dataloaders
"loss"
from losses.loss_collections import ComposeCycleLoss as ComposeLoss
"val"
# from val import Val, ValTrainset


def main():
    fixed_seed(seed=42)
    cfg, params_dict = get_parser()
    device = 'cuda:0'
    output_dir = os.path.join(cfg.output, cfg.dataset+'_train{}_val-shape1'.format(str(cfg.train_ids)), cfg.exp_name)
    
    "log"
    logger = get_logger()
    log_dir = os.path.join(output_dir, 'log')
    timestamp = datetime.datetime.now().strftime('%Y-%m-%d-%H-%M-%S')
    if cfg.continue_ckpt is None:
        writer = SummaryWriter(f'{log_dir}/runs/{timestamp}')

    "output"
    save_dir = os.path.join(output_dir, 'ckpt')
    os.makedirs(save_dir, exist_ok=True)

    "model"
    model = Model(cfg)
    if cfg.continue_ckpt is None:
        model = model.to(device)

    "data"
    dataset = get_dataloaders(cfg)
    train_loader = dataset['train']
    # val_loader = dataset['valid']

    "val"
    # val = Val(cfg, device)
    # val_trainset = ValTrainset(cfg, device)

    "optimizer"
    optimizer = torch.optim.Adam(filter(lambda p: p.requires_grad, model.parameters()), lr=float(cfg.base_lr), betas=(0.9, 0.999))

    "lr_scheduler"
    scheduler = torch.optim.lr_scheduler.ExponentialLR(optimizer, gamma=cfg.lr_sch_gamma)

    "loss"
    criterion = ComposeLoss(cfg).to(device)

    iteration = 0
    "continue train"
    if cfg.continue_ckpt is not None:
        ckpt_dict = torch.load(cfg.continue_ckpt, map_location='cpu')
        cfg.start_epoch = ckpt_dict['start_epoch']
        weights = ckpt_dict['model']
        model.load_state_dict(weights)
        model.to(device)
        optim = ckpt_dict['optimizer']
        # optimizer = torch.optim.Adam(filter(lambda p: p.requires_grad, model.parameters()), lr=ckpt_dict['optimizer']['param_groups'][0]['lr'], betas=(0.9, 0.999))
        optimizer.load_state_dict(optim)
        scheduler = torch.optim.lr_scheduler.ExponentialLR(optimizer, gamma=cfg.lr_sch_gamma, last_epoch=cfg.start_epoch-1)
        timestamp = '2023-07-10-23-27-58'
        writer = SummaryWriter(f'{log_dir}/runs/{timestamp}')
        iteration = cfg.start_epoch*len(train_loader)

    "train"
    for epoch in range(cfg.start_epoch, cfg.epochs):

        model.train()
        optimizer.zero_grad()
        if cfg.lr_sch_epoch is not None and (epoch+1) % cfg.lr_sch_epoch == 0:
             scheduler.step()
        loss_train_list = []
        for b, data in enumerate(train_loader):
            audio, vertices, template, one_hot, subject_id, init_state = data['audio'], data['vertices'], data['template'], data['one_hot'], data['subject_id'], data['init_state']
            vertices = vertices.to(device)
            label = vertices
            audio = audio.to(device)
            # text_label = text_label.to(device)
            template = template.to(device)
            one_hot = one_hot.to(device)
            id_label = torch.argmax(one_hot, dim=1)
            init_state = init_state.to(device)
            output = model(vertices, audio, template, init_state, id_label=id_label)
            # output = model(vertices, audio, template, one_hot)
            loss_dict = criterion(output, label, text_label=None, id_label=id_label)
            loss = loss_dict['loss']
            loss.backward()
            optimizer.step()
            optimizer.zero_grad()
            loss_train_list.append(loss.item())
            lr = optimizer.state_dict()['param_groups'][0]['lr']
            log = "(Epoch {}/{}, Batch {}/{}), lr {}, TRAIN LOSS:{:.9f}".format((epoch+1), cfg.epochs, b+1, len(train_loader), lr, loss)
            logger.info(log)
            iteration += 1
            if iteration%cfg.print_freq==0:
                for loss_item in loss_dict:
                    if 'acc' not in loss_item:
                        writer.add_scalar('train/{}'.format(loss_item), loss_dict[loss_item].item(), iteration)
                # debug
                # if loss_dict['content_contrastive_loss'].item() > 4.5e-6:
                #     print(subject_id)
            # cal loss for epoch
            if b==0:
                loss_dict_epoch = {}
                for key in loss_dict:
                    loss_dict_epoch[key] = loss_dict[key]
            else:
                for key in loss_dict_epoch:
                    loss_dict_epoch[key] += loss_dict[key]

        for loss_item in loss_dict_epoch:
            writer.add_scalar('train/{}_epoch'.format(loss_item), loss_dict_epoch[loss_item].item()/(b+1), epoch+1)
        # writer.add_scalar('train/loss_epoch', np.mean(loss_train_list), epoch+1)
        
        if (epoch+1)%cfg.save_freq==0:
            state = {'params': params_dict['params'],
                    'model': model.state_dict(),
                    'optimizer': optimizer.state_dict(),
                    'start_epoch': epoch+1,
                    }
            ckpt_dir = os.path.join(save_dir, 'Epoch_{}.pth'.format(epoch+1))
            torch.save(state, ckpt_dir)

        # if cfg.content_grl_loss.w_decay is not None and (epoch+1)%cfg.content_grl_loss.w_decay == 0:
        #     cfg.content_grl_loss.w = cfg.content_grl_loss.w*0.1
        # debug
        if cfg.content_grl_loss.w_decay is not None:
            if epoch+1 == 20:
                cfg.content_grl_loss.w = cfg.content_grl_loss.w*0.1
            elif epoch+1 == 40:
                cfg.content_grl_loss.w = cfg.content_grl_loss.w*0.5
                


if __name__ == '__main__':
    main()

```

## Source file: base/config.py

{
  "path": "base/config.py",
  "blob_sha": "62f93b9cb9e9b71301e26e27900b26467a119636",
  "sha256": "5813a009521cc94fa4d92949a052208330b80d79d530d77199e4eb8c03f9812f",
  "lines": 165,
  "bytes": 5444,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "rig": 8
  },
  "url": "https://github.com/huifu99/Mimic/blob/9e71299fc041a232e37ac79fbb4dff0b0552c20e/base/config.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, init_dict=None, key_list=None, new_allowed=False):"
    },
    {
      "line": 26,
      "text": "def __getattr__(self, name):"
    },
    {
      "line": 32,
      "text": "def __setattr__(self, name, value):"
    },
    {
      "line": 35,
      "text": "def __str__(self):"
    },
    {
      "line": 36,
      "text": "def _indent(s_, num_spaces):"
    },
    {
      "line": 56,
      "text": "def __repr__(self):"
    },
    {
      "line": 60,
      "text": "def load_cfg_from_cfg_file(file):"
    },
    {
      "line": 76,
      "text": "def merge_cfg_from_list(cfg, cfg_list):"
    },
    {
      "line": 91,
      "text": "def _decode_cfg_value(v):"
    },
    {
      "line": 120,
      "text": "def _check_and_coerce_cfg_value_type(replacement, original, key, full_key):"
    },
    {
      "line": 134,
      "text": "def conditional_cast(from_type, to_type):"
    },
    {
      "line": 162,
      "text": "def _assert_with_logging(cond, msg):"
    }
  ]
}

```text

# -----------------------------------------------------------------------------
# Functions for parsing args
# -----------------------------------------------------------------------------
import yaml
import os
from ast import literal_eval
import copy


class CfgNode(dict):
    """
    CfgNode represents an internal node in the configuration tree. It's a simple
    dict-like container that allows for attribute-based access to keys.
    """

    def __init__(self, init_dict=None, key_list=None, new_allowed=False):
        # Recursively convert nested dictionaries in init_dict into CfgNodes
        init_dict = {} if init_dict is None else init_dict
        key_list = [] if key_list is None else key_list
        for k, v in init_dict.items():
            if type(v) is dict:
                # Convert dict to CfgNode
                init_dict[k] = CfgNode(v, key_list=key_list + [k])
        super(CfgNode, self).__init__(init_dict)

    def __getattr__(self, name):
        if name in self:
            return self[name]
        else:
            raise AttributeError(name)

    def __setattr__(self, name, value):
        self[name] = value

    def __str__(self):
        def _indent(s_, num_spaces):
            s = s_.split("\n")
            if len(s) == 1:
                return s_
            first = s.pop(0)
            s = [(num_spaces * " ") + line for line in s]
            s = "\n".join(s)
            s = first + "\n" + s
            return s

        r = ""
        s = []
        for k, v in sorted(self.items()):
            seperator = "\n" if isinstance(v, CfgNode) else " "
            attr_str = "{}:{}{}".format(str(k), seperator, str(v))
            attr_str = _indent(attr_str, 2)
            s.append(attr_str)
        r += "\n".join(s)
        return r

    def __repr__(self):
        return "{}({})".format(self.__class__.__name__, super(CfgNode, self).__repr__())


def load_cfg_from_cfg_file(file):
    cfg = {}
    assert os.path.isfile(file) and file.endswith('.yaml'), \
        '{} is not a yaml file'.format(file)

    with open(file, 'r') as f:
        cfg_from_file = yaml.safe_load(f)

    for key in cfg_from_file:
        for k, v in cfg_from_file[key].items():
            cfg[k] = v

    cfg = CfgNode(cfg)
    return cfg


def merge_cfg_from_list(cfg, cfg_list):
    new_cfg = copy.deepcopy(cfg)
    assert len(cfg_list) % 2 == 0
    for full_key, v in zip(cfg_list[0::2], cfg_list[1::2]):
        subkey = full_key.split('.')[-1]
        assert subkey in cfg, 'Non-existent key: {}'.format(full_key)
        value = _decode_cfg_value(v)
        value = _check_and_coerce_cfg_value_type(
            value, cfg[subkey], subkey, full_key
        )
        setattr(new_cfg, subkey, value)

    return new_cfg


def _decode_cfg_value(v):
    """Decodes a raw config value (e.g., from a yaml config files or command
    line argument) into a Python object.
    """
    # All remaining processing is only applied to strings
    if not isinstance(v, str):
        return v
    # Try to interpret `v` as a:
    #   string, number, tuple, list, dict, boolean, or None
    try:
        v = literal_eval(v)
    # The following two excepts allow v to pass through when it represents a
    # string.
    #
    # Longer explanation:
    # The type of v is always a string (before calling literal_eval), but
    # sometimes it *represents* a string and other times a data structure, like
    # a list. In the case that v represents a string, what we got back from the
    # yaml parser is 'foo' *without quotes* (so, not '"foo"'). literal_eval is
    # ok with '"foo"', but will raise a ValueError if given 'foo'. In other
    # cases, like paths (v = 'foo/bar' and not v = '"foo/bar"'), literal_eval
    # will raise a SyntaxError.
    except ValueError:
        pass
    except SyntaxError:
        pass
    return v


def _check_and_coerce_cfg_value_type(replacement, original, key, full_key):
    """Checks that `replacement`, which is intended to replace `original` is of
    the right type. The type is correct if it matches exactly or is one of a few
    cases in which the type can be easily coerced.
    """
    original_type = type(original)
    replacement_type = type(replacement)

    # The types must match (with some exceptions)
    if replacement_type == original_type or original is None:
        return replacement

    # Cast replacement from from_type to to_type if the replacement and original
    # types match from_type and to_type
    def conditional_cast(from_type, to_type):
        if replacement_type == from_type and original_type == to_type:
            return True, to_type(replacement)
        else:
            return False, None

    # Conditionally casts
    # list <-> tuple
    casts = [(tuple, list), (list, tuple)]
    # For py2: allow converting from str (bytes) to a unicode string
    try:
        casts.append((str, unicode))  # noqa: F821
    except Exception:
        pass

    for (from_type, to_type) in casts:
        converted, converted_value = conditional_cast(from_type, to_type)
        if converted:
            return converted_value

    raise ValueError(
        "Type mismatch ({} vs. {}) with values ({} vs. {}) for config "
        "key: {}".format(
            original_type, replacement_type, original, replacement, full_key
        )
    )


def _assert_with_logging(cond, msg):
    if not cond:
        logger.debug(msg)
    assert cond, msg


```

## Source file: base/__init__.py

{
  "path": "base/__init__.py",
  "blob_sha": "4253df0556a265578b44dd0b95a2e7b246988656",
  "sha256": "4e78027adc2162220e000c58b7274f8d5a30b9d0e88f48eda24c87ce5cf81624",
  "lines": 1,
  "bytes": 25,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/huifu99/Mimic/blob/9e71299fc041a232e37ac79fbb4dff0b0552c20e/base/__init__.py",
  "function_signature_candidates": []
}

```text

from .base_model import *

```

## Source file: base/utilities.py

{
  "path": "base/utilities.py",
  "blob_sha": "e4d4c6b8a644774a004c359e22b24ded834b58a6",
  "sha256": "003295973cd5165794e212dec05b62375eb9371b4df77feec2fe9275f79361e7",
  "lines": 89,
  "bytes": 2297,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 3
  },
  "url": "https://github.com/huifu99/Mimic/blob/9e71299fc041a232e37ac79fbb4dff0b0552c20e/base/utilities.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def yaml2json(yaml_dir):"
    },
    {
      "line": 22,
      "text": "def get_parser():"
    },
    {
      "line": 36,
      "text": "def get_logger():"
    },
    {
      "line": 50,
      "text": "def __init__(self):"
    },
    {
      "line": 53,
      "text": "def reset(self):"
    },
    {
      "line": 59,
      "text": "def update(self, val, n=1):"
    },
    {
      "line": 66,
      "text": "def check_mkdir(dir_name):"
    },
    {
      "line": 71,
      "text": "def check_makedirs(dir_name):"
    },
    {
      "line": 76,
      "text": "def main_process(args):"
    },
    {
      "line": 81,
      "text": "def fixed_seed(seed=1):"
    }
  ]
}

```text

#!/usr/bin/env python
import argparse
import os
import random
import time
import logging
import yaml
import numpy as np
import torch
from base import config


def yaml2json(yaml_dir):
    f = open(yaml_dir, 'r')
    ystr = f.read()
    params = yaml.load(ystr, Loader=yaml.FullLoader)
    info_dict={}
    info_dict['params']=params
    return info_dict


def get_parser():
    parser = argparse.ArgumentParser(description=' ')
    parser.add_argument('--config', type=str, default='./config/HDTF/config.yaml', help='config file')
    parser.add_argument('opts', help=' ', default=None,
                        nargs=argparse.REMAINDER)
    args = parser.parse_args()
    assert args.config is not None
    cfg = config.load_cfg_from_cfg_file(args.config)
    if args.opts is not None:
        cfg = config.merge_cfg_from_list(cfg, args.opts)
    params_dict = yaml2json(args.config)
    return cfg, params_dict


def get_logger():
    logger_name = "main-logger"
    logger = logging.getLogger(logger_name)
    logger.setLevel(logging.INFO)
    handler = logging.StreamHandler()
    fmt = "[%(asctime)s %(levelname)s %(filename)s line %(lineno)d %(process)d]=>%(message)s"
    handler.setFormatter(logging.Formatter(fmt))
    logger.addHandler(handler)
    return logger


class AverageMeter(object):
    """Computes and stores the average and current value"""

    def __init__(self):
        self.reset()

    def reset(self):
        self.val = 0
        self.avg = 0
        self.sum = 0
        self.count = 0

    def update(self, val, n=1):
        self.val = val
        self.sum += val * n
        self.count += n
        self.avg = self.sum / self.count


def check_mkdir(dir_name):
    if not os.path.exists(dir_name):
        os.mkdir(dir_name)


def check_makedirs(dir_name):
    if not os.path.exists(dir_name):
        os.makedirs(dir_name)


def main_process(args):
    return not args.multiprocessing_distributed or (
            args.multiprocessing_distributed and args.rank % args.ngpus_per_node == 0)


def fixed_seed(seed=1):
    # seed = 1
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    torch.backends.cudnn.deterministic = True
    torch.backends.cudnn.benchmark = False

```

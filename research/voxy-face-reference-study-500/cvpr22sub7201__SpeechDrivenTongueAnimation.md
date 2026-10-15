# Facial rendering source scan: cvpr22sub7201/SpeechDrivenTongueAnimation

Repository: https://github.com/cvpr22sub7201/SpeechDrivenTongueAnimation

Pinned source revision: 82caf9d7f4331e039e3b2f0d31df6393d24ccb1c

Discovery topic: tongue animation

Description: This repository has the code for submission 7201

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"tongue": ["configs/training/transformer/ds/train_tongueformer_v106_ds_pose.yaml", "configs/training/transformer/ds/train_tongueformer_v107_ds_pose.yaml", "configs/training/transformer/mfa/train_tongueformer_v101_mfa_pose.yaml", "configs/training/transformer/mfa/train_tongueformer_v105_a_mfa_pose.yaml", "configs/training/transformer/mfa/train_tongueformer_v105_b_mfa_pose.yaml", "configs/training/transformer/mfa/train_tongueformer_v105_mfa_pose.yaml", "configs/training/transformer/mfcc/train_tongueformer_v100_mfcc_pose.yaml", "configs/training/transformer/mfcc/train_tongueformer_v104_a_mfcc_pose.yaml", "configs/training/transformer/mfcc/train_tongueformer_v104_mfcc_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_s2f_comparison_v2_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_s2f_comparison_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v001_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v002_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v003_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v004_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v005_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v006_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v007_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v008_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v009_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v010_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v011_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v012_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v013_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v014_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v015_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v016_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v017_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v018_w2v_c_pose.yaml", "configs/training/transformer/w2v_1_c/train_tongueformer_v019_w2v_c_pose.yaml"]}

## Source file: train_tongueformer.py

{
  "path": "train_tongueformer.py",
  "blob_sha": "63204d2b30abc6c1c5b74aabde8e164e015d574d",
  "sha256": "06a0e4616627768a50c2b94c742704c53276ae1ffb0e16eab59abb0c036fd4a0",
  "lines": 395,
  "bytes": 15333,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 5,
    "tongue": 7
  },
  "url": "https://github.com/cvpr22sub7201/SpeechDrivenTongueAnimation/blob/82caf9d7f4331e039e3b2f0d31df6393d24ccb1c/train_tongueformer.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def parse_args():"
    },
    {
      "line": 40,
      "text": "def print_training_header(training):"
    },
    {
      "line": 49,
      "text": "def get_criterion(loss):"
    },
    {
      "line": 74,
      "text": "def get_optimizer(model, optim):"
    },
    {
      "line": 89,
      "text": "def get_scheduler(scheduler, optimizer):"
    },
    {
      "line": 102,
      "text": "def train(model, model_optimizer, criterion, scheduler, dataloaders, save_dir,"
    },
    {
      "line": 237,
      "text": "def main(local_config, config):"
    }
  ]
}

```text

import argparse
from io import open
import random
import time
import os.path as osp
from pathlib import Path

import numpy as np
from ruamel.yaml import YAML
from easydict import EasyDict as edict

import torch
import torch.nn as nn
from torch import optim
import torch.nn.functional as F
from torch.utils.data import DataLoader

from dataset import TongueMocapDataset
from models import TongueFormer, save_checkpoint
from losses import L2L1Loss, HuberLoss, ShrinkageLoss
from logger.model_logger import ModelLogger


random.seed(78373)
torch.manual_seed(78373)
np.random.seed(78373)


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('-l', '--local_config_path', type=str,
                        help='Path to the local machine configuration')
    parser.add_argument('-c', '--config_path', type=str,
                        help='Path to the experiment configuration')
    parser.add_argument('-gid', '--gpu_id', type=int, default=0,
                        help='GPU to be used for training')
    return parser.parse_args()


def print_training_header(training):
    print('\nTraining Model')
    print('=' * 70)
    print(f'Num Epochs: {training.num_epochs}\nbatch size: {training.batch_sz}')
    print(f'Loss: {training.loss.label}  Params: {training.loss.params}')
    print('=' * 70)
    print()


def get_criterion(loss):
    """Creates a criterion from a label for MSE, Huber, and L1, otherwise None
    Args:
        loss (dict): dictionary with the loss label and params to construct the criterion
    Returns:
        nn.criterion: pytorch loss calculator
    """
    if loss.label == 'mse':
        return nn.MSELoss()
    elif loss.label == 'huber':
        return HuberLoss(delta=loss.params.delta)
    elif loss.label == 'smooth_l1':
        return nn.SmoothL1Loss(beta=loss.params.beta)
    elif loss.label == 'l1':
        return nn.L1Loss()
    elif loss.label == 'cross_entropy':
        return nn.CrossEntropyLoss()
    elif loss.label == 'l2l1':
        return L2L1Loss(betas=loss.params.betas)
    elif loss.label == 'shrinkage':
        return ShrinkageLoss(speed=loss.params.speed, loc=loss.params.loc)
    
    return None


def get_optimizer(model, optim):
    """Creates an optimizer from a label, if not in the list returns None
    Args:
        optim (dict): dictionary with the params to construct the optimizer
    Returns:
        nn.optim: pytorch optimizer
    """
    if optim.label == 'adam':
        return torch.optim.Adam(model.parameters(), lr=optim.params.lr, weight_decay=optim.params.weight_decay)
    if optim.label == 'adamw':
        return torch.optim.AdamW(model.parameters(), lr=optim.params.lr, weight_decay=optim.params.weight_decay)
    
    return None


def get_scheduler(scheduler, optimizer):
    """Creates an optimizer from a label, if not in the list returns None
    Args:
        optim (dict): dictionary with the params to construct the optimizer
    Returns:
        nn.optim: pytorch optimizer
    """
    if scheduler.label == 'exp_lr':
        return torch.optim.lr_scheduler.ExponentialLR(optimizer=optimizer, gamma=scheduler.gamma)

    return None


def train(model, model_optimizer, criterion, scheduler, dataloaders, save_dir,
            model_logger, log_start_step,
            batch_size, n_epochs, device, optim,
            output_idx, output_full=False,
            start_epoch=1, early_stop=10,
            multi_gpu=False,
            print_every=200):

    best_score = edict(value=1e9, epoch=0)
    iter_times_list = list()
    global_step = log_start_step
    last_val_loss = 1e9
    for epoch in range(start_epoch, start_epoch+n_epochs):
        print(f'Epoch {epoch}/{start_epoch+n_epochs-1}')
        print('-' * 70)
        
        ###------ Train ------###
        print('Training phase')
        phase = 'train'
        model.train(True)
        epoch_start_time = time.time()
        iter_start_time = time.time()

        dataloader_iter = iter(dataloaders['train'])
        running_loss = 0.0
        batch_idx = 0
        train_loss = 0.0
        for source_tensor, target_pos_tensor, _ in dataloader_iter:
            # skip the batch that is not complete
            if source_tensor.shape[0] != batch_size:
                continue
            
            global_step += 1
            batch_idx += 1

            # Pass to device
            source_tensor = source_tensor.to(device).float()            
            output_tensor = target_pos_tensor if output_full else target_pos_tensor[:, output_idx, :]

            pos_pred = model(source_tensor)

            loss = criterion(pos_pred, output_tensor.to(device).float())
            if batch_idx % print_every == 0:
                model_logger.train.add_scalar('loss/iter', loss, global_step)
            
            # We can perfectly fit a batch in a single pass
            model_optimizer.zero_grad()
            loss.backward()
            model_optimizer.step()
            
            running_loss += loss.item()
            
            if batch_idx % print_every == 0:
                iter_time = time.time() - iter_start_time
                iter_times_list.append(iter_time)
                iter_start_time = time.time()
                print(f'[train] Epoch {epoch}  Iter Time:{iter_time:.3f}  Step:{batch_idx}/{len(dataloaders["train"]):<8}  l: {running_loss/batch_idx:<10}')
        
        train_loss = running_loss/len(dataloaders[phase])
        epoch_time_str = time.strftime("%H:%M:%S", time.gmtime(time.time() - epoch_start_time))
        print(f'Training totals:  loss: {train_loss}  time: {epoch_time_str}')
        print()

        ###------ Validate every 10 epochs ------###
        if epoch % 10 == 0:
            print('Validation phase')
            phase = 'valid'
            model.eval()
        
            with torch.no_grad():
                running_loss = 0.0
                epoch_start_time = time.time()
                iter_start_time = time.time()
                
                dataloader_iter = iter(dataloaders['valid'])
                running_loss = 0.0
                batch_idx = 0
                val_loss = 0.0
                for source_tensor, target_pos_tensor, _ in dataloader_iter:
                    # skip the batch that is not complete
                    if source_tensor.shape[0] != batch_size:
                        continue

                    batch_idx += 1

                    # Pass to device
                    source_tensor = source_tensor.to(device).float()
                    output_tensor = target_pos_tensor if output_full else target_pos_tensor[:, output_idx, :]

                    pos_pred = model(source_tensor)
                    loss = criterion(pos_pred, output_tensor.to(device).float())
                    running_loss += loss.item()
                    
                    if batch_idx % print_every == 0:
                        iter_time = time.time() - iter_start_time
                        iter_times_list.append(iter_time)
                        iter_start_time = time.time()
                        print(f'[valid] Epoch {epoch}  Iter Time:{iter_time:.3f}  Step:{batch_idx}/{len(dataloaders["valid"]):<8}  l: {running_loss/batch_idx:<10}')
                
                val_loss = running_loss/len(dataloaders['valid'])
                epoch_time_str = time.strftime("%H:%M:%S", time.gmtime(time.time() - epoch_start_time))
                print(f'Validation totals:  loss: {val_loss}  time: {epoch_time_str}')
                print()
                model_logger.val.add_scalar('loss/iter', val_loss, global_step)

                # -- Early Stop ---
                if val_loss < best_score.value:
                    best_score.value = val_loss
                    best_score.epoch = epoch
                
                if (epoch - best_score.epoch) >= early_stop:
                    print(f'Early stop at epoch {epoch}, previous best: {best_score.value} @ {best_score.epoch}')
                    break

                last_val_loss = val_loss

        # TODO: HACK! add as a config parameter
        if (last_val_loss < 2.0) or (epoch % 10 == 0):
            save_checkpoint(epoch=epoch, 
                            model=model, 
                            model_params=model.module.params if multi_gpu else model.params, 
                            optimizer=model_optimizer, 
                            optimizer_params=dict(optim.params), 
                            loss=train_loss, 
                            global_step=global_step,
                            save_path=osp.join(save_dir, f'{epoch:02d}.pt'))
        
        if scheduler is not None:
            scheduler.step()
        
    total_train_time_str = time.strftime("%H:%M:%S", time.gmtime(sum(iter_times_list)))
    print(f'Total training time: {total_train_time_str}')
    return train_loss, val_loss


def main(local_config, config):
    # Training params
    start_epoch = 1
    
    # Data loading params
    train_dataset_path = osp.join(local_config.datasets_dir, config.data.train.path)
    valid_dataset_path = osp.join(local_config.datasets_dir, config.data.valid.path)

    # Training save dir
    model_save_dir = osp.join(local_config.models_dir, config.model.save_dir)
    Path(model_save_dir).mkdir(parents=True, exist_ok=True)

    # Log
    log_dir = osp.join(local_config.logs_dir, config.log.save_dir)
    Path(log_dir).mkdir(parents=True, exist_ok=True)
    log_start_step = 0
    model_logger = ModelLogger(log_dir, config.name)
    
    # Create device under which the model will be trained
    device_str = 'cpu'
    if torch.cuda.is_available():
        num_gpus = torch.cuda.device_count()
        target_gid = int(args.gpu_id)
        if target_gid >= 0 and target_gid < num_gpus:
            device_str = f'cuda:{target_gid}'
    
    device = torch.device(device_str)
    print(f'Training on device: {device}')
    
    # Create dataset
    print('Loading Training data')
    train_dataset = TongueMocapDataset(train_dataset_path,
                                        num_files=config.data.train.num_files,
                                        win_sz=config.data.train.win_sz,
                                        stride=config.data.train.win_stride,
                                        pose_only=config.data.train.pose_only)
    print(f'Training samples:    {len(train_dataset)}')
    
    print('Loading Validation data')
    valid_dataset = TongueMocapDataset(valid_dataset_path,
                                        num_files=config.data.valid.num_files,
                                        win_sz=config.data.valid.win_sz,
                                        stride=config.data.valid.win_stride,
                                        pose_only=config.data.valid.pose_only)
    print(f'Validation samples:  {len(valid_dataset)}')
    
    # Create dataloaders
    train_dataloader = DataLoader(train_dataset, 
                                    batch_size=config.training.batch_sz, 
                                    shuffle=True, 
                                    num_workers=config.data.train.num_workers, 
                                    pin_memory=True)
    valid_dataloader = DataLoader(valid_dataset, 
                                    batch_size=config.training.batch_sz, 
                                    shuffle=False, 
                                    num_workers=config.data.valid.num_workers, 
                                    pin_memory=True)
    dataloaders = dict(train=train_dataloader, 
                        valid=valid_dataloader)

    # Build if no checkpoint is given
    if 'checkpoint' not in config:
        print('Building new model')
        model = TongueFormer(**config.model.params)
        model.to(device)

        print('Building new optimizer')
        model_optimizer = get_optimizer(model, config.optim)
    else:
        print('Loading model checkpoint')
        checkpoint_path = osp.join(local_config.models_dir, config.checkpoint.path)
        checkpoint = torch.load(checkpoint_path)
        model = TongueFormer(**config.model.params)
        trained_dict = checkpoint['model_state_dict']
        model_dict = model.state_dict()

        diff = set(model_dict.keys()) - set(trained_dict.keys())
        froce_reset_optim = False
        if not diff:
            model.load_state_dict(trained_dict)
        else:
            if 'deeper_fc' in config.model.params:
                if config.model.params['deeper_fc']:
                    froce_reset_optim = True
                    # let's remove head.1 since it's size is now
                    # different and not matching the checkpoint
                    for i in range(2):
                        del trained_dict['head.{}.weight'.format(i)]
                        del trained_dict['head.{}.bias'.format(i)]

            model_dict.update(trained_dict)
            model.load_state_dict(model_dict)

        model.to(device)

        reset = False
        if 'reset' in config.checkpoint:
            reset = config.checkpoint.reset

        if not (reset or froce_reset_optim): 
            print('Loading optimizer checkpoint')
            model_optimizer = optim.Adam(model.parameters(), lr=checkpoint['optimizer_params']['lr'])
            model_optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
            start_epoch = int(Path(checkpoint_path).stem) + 1
            log_start_step = checkpoint['global_step']
        else:
            print('RESET is TRUE - Building new optimizer')
            model_optimizer = get_optimizer(model, config.optim)

    
    # Criterion
    criterion = get_criterion(config.training.loss)
    criterion.to(device)

    # Scheduler
    scheduler = get_scheduler(config.scheduler, model_optimizer) if 'scheduler' in config else None

    print_training_header(config.training)
    if start_epoch > 1:
        print(f'Resuming training from epoch {start_epoch}')
    print(f'Checkpoints save dir: {model_save_dir}')
    print(f'Log save dir:         {log_dir}')
    print()
    
    train(model=model, 
            model_optimizer=model_optimizer,
            criterion=criterion,
            scheduler=scheduler,
            dataloaders=dataloaders, 
            save_dir=model_save_dir, 
            model_logger=model_logger,
            log_start_step=log_start_step,
            batch_size=config.training.batch_sz, 
            n_epochs=config.training.num_epochs, 
            device=device,
            optim=config.optim,
            output_idx=config.training.output_idx,
            output_full=config.training.output_full, # TODO: add to all the tongueformer training config files
            start_epoch=start_epoch,
            early_stop=config.training.early_stop,
            multi_gpu=config.model.multi_gpu,
            print_every=200)


if __name__ == '__main__':
    # Parse input arguments
    args = parse_args()
    # Load configurations
    yaml = YAML(typ='safe')
    # -- machine configuration
    local_config = edict(yaml.load(open(args.local_config_path)))
    # -- training configuration
    config = edict(yaml.load(open(args.config_path)))

    # TODO: fix None load
    config.model.params.qk_scale = None
    config.model.params.norm_layer = None
    
    main(local_config, config)

```

## Source file: utils/io.py

{
  "path": "utils/io.py",
  "blob_sha": "8f7384075225f289d1950446a008f43e63e876a2",
  "sha256": "2f665b264be96b9a9a6383b741e6cd95c159c48a82109b1678f69bd16a49e122",
  "lines": 394,
  "bytes": 8617,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1
  },
  "url": "https://github.com/cvpr22sub7201/SpeechDrivenTongueAnimation/blob/82caf9d7f4331e039e3b2f0d31df6393d24ccb1c/utils/io.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def get_checkpoint(resume_path: str) -> str:"
    },
    {
      "line": 75,
      "text": "def ensure_dir(path: str) -> str:"
    },
    {
      "line": 91,
      "text": "def get_filename_from_path(path: str) -> str:"
    },
    {
      "line": 110,
      "text": "def file_exists(path: str) -> bool:"
    },
    {
      "line": 124,
      "text": "def write_json(path: str, data: dict) -> None:"
    },
    {
      "line": 137,
      "text": "def read_from_json(path: str) -> dict:"
    },
    {
      "line": 160,
      "text": "def abs_path(path: str) -> str:"
    },
    {
      "line": 180,
      "text": "def get_dir(path: str) -> str:"
    },
    {
      "line": 210,
      "text": "def get_format(path: str) -> str:"
    },
    {
      "line": 232,
      "text": "def write_h5(path: str, data: dict) -> None:"
    },
    {
      "line": 263,
      "text": "def read_h5(path: str) -> dict:"
    },
    {
      "line": 288,
      "text": "def get_sub_dirs(path: str):"
    },
    {
      "line": 313,
      "text": "def get_files(path: str, file_format: str, keep_format: bool = True):"
    },
    {
      "line": 365,
      "text": "def remove_files(paths: list) -> None:"
    },
    {
      "line": 377,
      "text": "def make_relative(path: str, root_path: str) -> str:"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
IO Utils
"""

__version__ = "0.2.0"
__all__ = [
    'get_checkpoint',
    'ensure_dir',
    'get_filename_from_path',
    'file_exists',
    'get_format',
    'write_json',
    'read_from_json',
    'abs_path',
    'get_dir',
    'write_h5',
    'read_h5',
    'get_sub_dirs',
    'get_files',
    'remove_files',
    'make_relative',
]

import re
import os
import json
import h5py
import numpy as np


def get_checkpoint(resume_path: str) -> str:
    """Get checkpoint file from dir with multiple checkpoints

    Args:
        resume_path (str): path to the dir containing the checkpoint or
                           partially defined path ('*' only allowed at the end)

    Raises:
        IOError: No checkpoint has been found

    Returns:
        str: path to the checkpoint file
    """

    if not os.path.isdir(resume_path):

        assert '.pth.tar' not in resume_path

        # path needs to be completed
        partial_name = resume_path.split('/')[-1]
        path_dir = resume_path.replace(partial_name, '')

        models_list = [f for f in os.listdir(path_dir) if partial_name in f]

        if models_list:
            return os.path.join(path_dir, models_list[-1])

        # let's get a model in the same directory
        resume_path = path_dir

    models_list = [f for f in os.listdir(
        resume_path) if f.endswith(".pth.tar")]
    models_list.sort()

    if not models_list:
        raise IOError(
            'Directory {} does not contain any model'.format(resume_path))

    model_name = models_list[-2]

    return os.path.join(resume_path, model_name)


def ensure_dir(path: str) -> str:
    """Make sure directory exists, otherwise create it.

    Args:
        path (str): path to the directory.

    Returns:
        str: path to the directory.
    """

    dir_path = get_dir(path)
    if not os.path.exists(dir_path):
        os.makedirs(dir_path)
    return path


def get_filename_from_path(path: str) -> str:
    """Get name of a file given the absolute or
    relative path.

    Args:
        path (str): path to the file.

    Returns:
        str: file name without format.
    """

    assert isinstance(path, str)
    split = os.path.split(path)
    if len(split) == 1:
        return split

    return split[-1]


def file_exists(path: str) -> bool:
    """Check if file exists

    Args:
        path (str): path to the file.

    Returns:
        bool: True if file exists.
    """

    assert isinstance(path, str)
    return os.path.exists(path)


def write_json(path: str, data: dict) -> None:
    """Save data into a json file

    Args:
        path (str): path where to save the file
        data (dict): data to be stored
    """

    assert isinstance(path, str)
    with open(path, 'w') as out_file:
        json.dump(data, out_file, indent=2)


def read_from_json(path: str) -> dict:
    """Read data from json file

    Args:
        path (str): path to json file

    Raises:
        IOError: File not found

    Returns:
        dict: dictionary containing data
    """

    assert isinstance(path, str)
    if '.json' not in path:
        raise IOError('Path does not point to a json file')

    with open(path, 'r') as in_file:
        data = json.load(in_file)

    return data


def abs_path(path: str) -> str:
    """Get absolute path of a relative one

    Args:
        path (str): relative path

    Raises:
        NameError: string is empty

    Returns:
        str: absolute path
    """

    assert isinstance(path, str)
    if path:
        return os.path.expanduser(path)

    raise NameError('Path is empty...')


def get_dir(path: str) -> str:
    """Get file path from absolute or relative path without
    file name.

    Args:
        path (str): path to directory.

    Returns:
        str: path.
    """

    assert isinstance(path, str), "Path must be a string"

    # check if exists
    if not os.path.exists(path):
        if '.' not in path[:-5]:
            return path
    else:
        # check if it's arready a dir
        if os.path.isdir(path):
            return path

    split = os.path.split(path)
    # check if the path is only the file name
    if len(split) == 1:
        return './'

    return split[0]


def get_format(path: str) -> str:
    """Get file format from path

    Args:
        path (str): file path.

    Raises:
        RuntimeError: File path has no format.

    Returns:
        str: file format.
    """

    file_name = get_filename_from_path(path)
    split_str = file_name.split('.')

    if len(split_str) <= 1:
        raise RuntimeError('File name has no format')

    return split_str[-1]


def write_h5(path: str, data: dict) -> None:
    """Write h5 file

    Args:
        path (str): file path where to save the data
        data (dict): data to be saved

    Raises:
        NotImplementedError: non serializable data to save
    """

    if '.h5' not in path[-3:]:
        path += '.h5'

    hf = h5py.File(path, mode='w')

    if isinstance(data, dict):
        for k, v in data.items():
            if isinstance(v[0], str):
                v = [a.encode('utf8') for a in v]
            hf.create_dataset(k, data=v)
    elif isinstance(data, list):
        hf.create_dataset('val', data=data)
    elif isinstance(data, np.ndarray):
        hf.create_dataset('val', data=data)
    else:
        raise NotImplementedError

    hf.close()


def read_h5(path: str) -> dict:
    """Load data from h5 file

    Args:
        path (str): file path

    Raises:
        FileNotFoundError: path not pointing to a file

    Returns:
        dict: dictionary containing the data
    """

    data_files = dict()
    h5_data = h5py.File(path, mode='r')

    tags = list(h5_data.keys())
    for tag in tags:
        tag_data = np.asarray(h5_data[tag]).copy()
        data_files.update({tag: tag_data})
    h5_data.close()

    return data_files


def get_sub_dirs(path: str):
    """Get sub-directories contained in a specified directory

    Args:
        path (str): path to directory

    Returns:
        str: lists of absolute paths
        str: list of dir names
    """

    try:
        dirs = os.walk(path).next()[1]
    except AttributeError:
        try:
            dirs = next(os.walk(path))[1]
        except StopIteration:
            dirs = []

    dirs.sort()
    dir_paths = [os.path.abspath(os.path.join(path, dir)) for dir in dirs]

    return dirs, dir_paths


def get_files(path: str, file_format: str, keep_format: bool = True):
    """Get file paths of files contained in
    a given directory according to the format

    Args:
        path (str): path to the directory containing the files
        file_format (list): list or single format
        keep_format (bool, optional): keep format in file names.
                                      Defualts to True.

    Returns:
        str: lists of absolute paths
        str: list of file names
    """

    if isinstance(file_format, str):
        file_format = [file_format]
    else:
        assert isinstance(file_format, list)

    # get sub-directories files
    _, sub_dirs = get_sub_dirs(path)
    if sub_dirs:
        list_paths = []
        list_names = []
        for sub_dir in sub_dirs:
            paths, names = get_files(sub_dir, file_format)
            list_paths.extend(paths)
            list_names.extend(names)
        return list_paths, list_names

    # get current files
    file_names = []
    file_paths = []
    for f_format in file_format:
        if f_format != '*':
            files = [f for f in os.listdir(path)
                     if re.match(r'.*\.{}'.format(f_format), f)]
        else:
            files = os.listdir(path)
        files.sort()

        if not keep_format:
            file_names.extend([f.replace('.{}'.format(f_format), '')
                               for f in files])
        else:
            file_names.extend(files)
        file_paths.extend([os.path.join(path, f) for f in files])

    return file_paths, file_names


def remove_files(paths: list) -> None:
    """Delete files

    Args:
        paths (list): list of paths
    """

    assert isinstance(paths, list)
    for path in paths:
        os.remove(path)


def make_relative(path: str, root_path: str) -> str:
    """Make path relative with respect to a
    root directory

    Arguments:
        path (str): current path.
        root_path (str): root directory path.

    Returns:
        str: relative path.
    """

    r_path = path.replace(root_path, '')
    if r_path:
        if r_path[0] == '/':
            r_path = r_path[1:]

    return r_path


```

## Source file: train_gru.py

{
  "path": "train_gru.py",
  "blob_sha": "723abfebce339d4385f785472187b42f399dac7a",
  "sha256": "1b169a7b32621c67f0ea3725741b69405e0db5e7b6848c542faad8e4b8e15725",
  "lines": 329,
  "bytes": 12590,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "tongue": 3
  },
  "url": "https://github.com/cvpr22sub7201/SpeechDrivenTongueAnimation/blob/82caf9d7f4331e039e3b2f0d31df6393d24ccb1c/train_gru.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def parse_args():"
    },
    {
      "line": 38,
      "text": "def print_training_header(learning_rate, dropout_rate, batch_size, rnn, model_save_dir, log_dir):"
    },
    {
      "line": 46,
      "text": "def get_criterion(loss_label):"
    },
    {
      "line": 65,
      "text": "def train(gru, gru_optimizer, dataloaders, save_dir, log_dir,"
    },
    {
      "line": 183,
      "text": "def main(local_config, config):"
    }
  ]
}

```text

import argparse
import os
import os.path as osp
import random
import time
from io import open
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from easydict import EasyDict as edict
from ruamel.yaml import YAML
from torch import optim
from torch.utils.data import DataLoader

from dataset import  TongueMocapDataset
from logger.model_logger import ModelLogger
from models import GRU, save_checkpoint

random.seed(78373)
torch.manual_seed(78373)
np.random.seed(78373)


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('-l', '--local_config_path', type=str,
                        help='Path to the local machine configuration')
    parser.add_argument('-c', '--config_path', type=str,
                        help='Path to the experiment configuration')
    parser.add_argument('-gid', '--gpu_id', type=int, default=0,
                        help='GPU to be used for training')
    return parser.parse_args()


def print_training_header(learning_rate, dropout_rate, batch_size, rnn, model_save_dir, log_dir):
    print('\nTraining Model')
    print('=' * 70)
    print(f'LR:{learning_rate}\ndropout: {dropout_rate}\nbatch size: {batch_size}\nmodel:{rnn}\nmodel dir:{model_save_dir}\nlog dir:{log_dir}')
    print('=' * 70)
    print()


def get_criterion(loss_label):
    """Creates a criterion from a label for MSE, Huber, and L1, otherwise None

    Args:
        loss_label (str): loss label string

    Returns:
        nn.criterion: pytorch loss calculator
    """
    if loss_label == 'mse':
        return nn.MSELoss()
    elif loss_label == 'huber':
        return nn.SmoothL1Loss()
    elif loss_label == 'l1':
        return nn.L1Loss()

    return None


def train(gru, gru_optimizer, dataloaders, save_dir, log_dir,
          batch_size, n_epochs, device, start_epoch=1, early_stop=10,
          multi_gpu=False, learning_rate=0.01, dropout=0.2,
          loss_str='mse', log_start_step=0, print_every=200):

    model_logger = ModelLogger(log_dir, f'test')
    criterion = get_criterion(loss_str)
    criterion.to(device)

    iter_times_list = list()

    best = edict(loss=1e9, epoch=-1, path='best.pt')
    global_step = log_start_step
    for epoch in range(start_epoch, start_epoch+n_epochs):
        print(f'Epoch {epoch}/{start_epoch+n_epochs-1}')
        print('-' * 70)

        for phase in ['train', 'valid']:
            if phase == 'train':
                gru.train(True)
                print('Training phase')
            else:
                gru.eval()
                print('Validation phase')

            epoch_start_time = time.time()
            iter_start_time = time.time()

            if not multi_gpu:
                h0 = gru.init_hidden(batch_size).to(device)

            dataloader_iter = iter(dataloaders[phase])
            running_loss = 0.
            batch_idx = 0
            total_loss = 0.0
            for input_tensor, target_tensor in dataloader_iter:
                # skip the batch that is not complete
                if input_tensor.shape[0] != batch_size:
                    continue

                if phase == 'train':
                    global_step += 1
                batch_idx += 1

                if multi_gpu:
                    pos_pred = gru(x=input_tensor.to(device).float())
                else:
                    h = h0.data
                    pos_pred, _ = gru(x=input_tensor.to(device).float(), h=h)

                loss = criterion(pos_pred, target_tensor.to(device).float())

                # We can perfectly fit a batch in a single pass
                gru_optimizer.zero_grad()
                if phase == 'train':
                    if batch_idx % print_every == 0:
                        model_logger.train.add_scalar('loss/iter',
                                                      loss,
                                                      global_step)
                    loss.backward()
                    gru_optimizer.step()

                running_loss += loss.item()

                if batch_idx % print_every == 0:
                    iter_time = time.time() - iter_start_time
                    iter_times_list.append(iter_time)
                    iter_start_time = time.time()
                    print(f'[{phase}] Epoch {epoch}  Iter Time:{iter_time:.3f}  Step:{batch_idx}/{len(dataloaders[phase]):<8}  l: {running_loss/batch_idx:<10}')

            total_loss = running_loss/len(dataloaders[phase])
            epoch_time_str = time.strftime("%H:%M:%S",
                                           time.gmtime(time.time() - epoch_start_time))
            print(f'Totals: {phase}  loss: {total_loss}  time: {epoch_time_str}')
            print()

            if phase == 'train':
                save_checkpoint(epoch=epoch,
                                model=gru,
                                model_params=gru.module.params if multi_gpu else gru.params,
                                optimizer=gru_optimizer,
                                optimizer_params=dict(lr=learning_rate),
                                loss=total_loss,
                                global_step=global_step,
                                save_path=osp.join(save_dir, f'{epoch:02d}.pt'))
            else:
                model_logger.val.add_scalar('loss/iter',
                                            total_loss,
                                            global_step)
                if total_loss < best.loss:
                    best.loss = total_loss
                    best.epoch = epoch
                    if osp.exists(best.path):
                        os.remove(best.path)
                    best.path = osp.join(save_dir, f'best_{epoch:02d}.pt')
                    save_checkpoint(epoch=epoch,
                                    model=gru,
                                    model_params=gru.module.params if multi_gpu else gru.params,
                                    optimizer=gru_optimizer,
                                    optimizer_params=dict(lr=learning_rate),
                                    loss=total_loss,
                                    global_step=global_step,
                                    save_path=best.path)
                else:
                    if (epoch - best.epoch) >= early_stop:
                        print(f'Early Stop @ Epoch {epoch}')
                        print(f'Best model:')
                        print(f'    Epoch:    {best.epoch}')
                        print(f'    Val Loss: {best.loss}')
                        print(f'    Path:     {best.path}')
                        exit(0)

    total_train_time_str = time.strftime("%H:%M:%S",
                                         time.gmtime(sum(iter_times_list)))
    print(f'Total training time: {total_train_time_str}')
    return total_loss


def main(local_config, config):
    # Training params
    start_epoch = 1
    num_epochs = config.training.num_epochs
    batch_size = config.training.batch_sz
    dropout_rate = config.training.dropout
    learning_rate = config.training.learning_rate
    early_stop = config.training.early_stop

    # Data loading params
    num_audio_files = config.data.num_train_files
    dataload_workers = config.data.num_dataload_workers
    data_win_sz = config.data.win_sz
    data_win_stride = config.data.win_stride
    train_dataset_path = osp.join(local_config.datasets_dir,
                                  config.data.train_path)
    valid_dataset_path = osp.join(local_config.datasets_dir,
                                  config.data.valid_path)

    # Model params
    input_size = config.model.input_sz
    hidden_size = config.model.hidden_sz
    output_size = config.model.output_sz
    num_layers = config.model.num_layers
    bidirectional = config.model.bidirectional
    model_save_dir = osp.join(local_config.models_dir, config.model.save_dir)
    Path(model_save_dir).mkdir(parents=True, exist_ok=True)

    # Log params
    log_dir = osp.join(local_config.logs_dir, config.log.save_dir)
    Path(log_dir).mkdir(parents=True, exist_ok=True)
    log_start_step = 0

    # Create device under which the model will be trained
    cuda_str = 'cpu'
    if torch.cuda.is_available():
        num_gpus = torch.cuda.device_count()
        if config.training.multi_gpu and num_gpus > 1:
            cuda_str = 'cuda'
        else:
            target_gid = int(args.gpu_id)
            if target_gid >= 0 and target_gid < num_gpus:
                cuda_str = f'cuda:{target_gid}'

    print(f'Training on device: {cuda_str}')
    device = torch.device(cuda_str)

    # Create dataset
    if config.data.type == 'mocap':
        print('Loading Training data')
        train_dataset = TongueMocapDataset(train_dataset_path,
                                           num_files=num_audio_files,
                                           win_sz=data_win_sz,
                                           stride=data_win_stride,
                                           pose_only=True)
        print(f'Training samples:    {len(train_dataset)}')

        print('Loading Validation data')
        valid_dataset = TongueMocapDataset(valid_dataset_path,
                                           num_files=num_audio_files,
                                           win_sz=data_win_sz,
                                           stride=data_win_stride,
                                           pose_only=True)
        print(f'Validation samples:  {len(valid_dataset)}')
    else:
        print('ERROR: Unknown dataset type')
        exit(-1)

    # Create dataloaders
    train_dataloader = DataLoader(train_dataset,
                                  batch_size=batch_size,
                                  shuffle=True,
                                  num_workers=dataload_workers,
                                  pin_memory=True)
    valid_dataloader = DataLoader(valid_dataset,
                                  batch_size=batch_size,
                                  shuffle=False,
                                  num_workers=dataload_workers,
                                  pin_memory=True)
    dataloaders = dict(train=train_dataloader,
                       valid=valid_dataloader)

    # Build if no checkpoint is given
    if 'checkpoint' not in config:
        print('Building new model')
        rnn = GRU(input_size, hidden_size, output_size,
                  n_layers=num_layers,
                  dropout=dropout_rate,
                  bidirectional=bidirectional)
        if config.training.multi_gpu:
            rnn = nn.DataParallel(rnn)
        rnn.to(device)

        print('Building new optimizer')
        rnn_optimizer = optim.Adam(rnn.parameters(), lr=learning_rate)
    else:
        print('Loading model checkpoint')
        checkpoint_path = osp.join(local_config.models_dir,
                                   config.checkpoint.path)
        checkpoint = torch.load(checkpoint_path)
        rnn = GRU(**checkpoint['model_params'])
        rnn.load_state_dict(checkpoint['model_state_dict'])
        rnn.to(device)

        print('Loading optimizer checkpoint')
        rnn_optimizer = optim.Adam(rnn.parameters(),
                                   lr=checkpoint['optimizer_params']['lr'])
        rnn_optimizer.load_state_dict(checkpoint['optimizer_state_dict'])

        start_epoch = int(Path(checkpoint_path).stem) + 1
        log_start_step = checkpoint['global_step']

    print_training_header(learning_rate,
                          dropout_rate,
                          batch_size,
                          rnn,
                          model_save_dir,
                          log_dir)

    train(rnn, rnn_optimizer,
          dataloaders=dataloaders,
          save_dir=model_save_dir,
          log_dir=log_dir,
          batch_size=batch_size,
          learning_rate=learning_rate,
          dropout=dropout_rate,
          start_epoch=start_epoch,
          n_epochs=num_epochs,
          early_stop=early_stop,
          loss_str=config.training.loss,
          device=device,
          multi_gpu=config.training.multi_gpu,
          log_start_step=log_start_step,
          print_every=200)


if __name__ == '__main__':
    # Parse input arguments
    args = parse_args()
    # Load configurations
    yaml = YAML(typ='safe')
    # -- machine configuration
    local_config = edict(yaml.load(open(args.local_config_path)))
    # -- training configuration
    config = edict(yaml.load(open(args.config_path)))

    main(local_config, config)


```

## Source file: train_mlp.py

{
  "path": "train_mlp.py",
  "blob_sha": "a013aef7226313180131746bfe45f636cb9e6744",
  "sha256": "5f76655825252bf0c5224395beffac2d61841ccb8a3ca42aa7773d84fafbbd9c",
  "lines": 273,
  "bytes": 10512,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "tongue": 3
  },
  "url": "https://github.com/cvpr22sub7201/SpeechDrivenTongueAnimation/blob/82caf9d7f4331e039e3b2f0d31df6393d24ccb1c/train_mlp.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def parse_args():"
    },
    {
      "line": 38,
      "text": "def print_training_header(learning_rate, dropouts, batch_size, model, model_save_dir, log_dir):"
    },
    {
      "line": 46,
      "text": "def print_checkpoint(name, params_dict):"
    },
    {
      "line": 52,
      "text": "def get_criterion(loss_label):"
    },
    {
      "line": 71,
      "text": "def train(model, model_optimizer, dataloaders, save_dir, log_dir,"
    },
    {
      "line": 156,
      "text": "def main(local_config, config):"
    }
  ]
}

```text

import argparse
from io import open
import random
import time
import os.path as osp
from pathlib import Path

import numpy as np
from ruamel.yaml import YAML
from easydict import EasyDict as edict

import torch
import torch.nn as nn
from torch import optim
from torch.utils.data import DataLoader

from dataset import CausalTongueMocapDataset
from models import MLP, save_checkpoint, load_checkpoint
from logger.model_logger import ModelLogger


random.seed(78373)
torch.manual_seed(78373)
np.random.seed(78373)


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('-l', '--local_config_path', type=str,
                        help='Path to the local machine configuration')
    parser.add_argument('-c', '--config_path', type=str,
                        help='Path to the experiment configuration')
    parser.add_argument('-gid', '--gpu_id', type=int, default=0,
                        help='GPU to be used for training')
    return parser.parse_args()


def print_training_header(learning_rate, dropouts, batch_size, model, model_save_dir, log_dir):
    print('\nTraining Model')
    print('=' * 70)
    print(f'LR:{learning_rate}\ndropout: {dropouts}\nbatch size: {batch_size}\nmodel:{model}\nmodel dir:{model_save_dir}\nlog dir:{log_dir}')
    print('=' * 70)
    print()


def print_checkpoint(name, params_dict):
    print(f'{"Name":>13}: {name}')
    for k, v in params_dict.items():
        print(f'{k:>13}: {v}')


def get_criterion(loss_label):
    """Creates a criterion from a label for MSE, Huber, and L1, otherwise None

    Args:
        loss_label (str): loss label string

    Returns:
        nn.criterion: pytorch loss calculator
    """
    if loss_label == 'mse':
        return nn.MSELoss()
    elif loss_label == 'huber':
        return nn.SmoothL1Loss()
    elif loss_label == 'l1':
        return nn.L1Loss()

    return None


def train(model, model_optimizer, dataloaders, save_dir, log_dir,
          batch_size, n_epochs, device, start_epoch=1, multi_gpu=False, learning_rate=0.01,
          loss_str='mse', print_every=200, log_start_step=0, label='training'):

    model_logger = ModelLogger(log_dir, label)
    criterion = get_criterion(loss_str)
    criterion.to(device)

    iter_times_list = list()
    global_step = log_start_step
    for epoch in range(start_epoch, start_epoch+n_epochs):
        print(f'Epoch {epoch}/{start_epoch+n_epochs-1}')
        print('-' * 70)

        for phase in ['train', 'valid']:
            if phase == 'train':
                model.train(True)
                print('Training phase')
            else:
                model.eval()
                print('Validation phase')

            dataloader_iter = iter(dataloaders[phase])
            running_loss = 0.

            epoch_start_time = time.time()
            iter_start_time = time.time()

            batch_idx = 0
            total_loss = 0.0
            for feats_tensor, pos_tensor, _ in dataloader_iter:
                # skip the batch that is not complete
                if feats_tensor.shape[0] != batch_size:
                    continue

                # Flatten for causal inference
                input_tensor = torch.flatten(feats_tensor, 1, 2).to(device).float()
                target_pos_tensor = torch.flatten(pos_tensor, 1, 2).to(device).float()

                pos_pred = model(x=input_tensor)
                loss = criterion(pos_pred, target_pos_tensor)

                # We can perfectly fit a batch in a single pass
                model_optimizer.zero_grad()
                if phase == 'train':
                    if (batch_idx > 0) and (batch_idx % print_every == 0):
                        model_logger.train.add_scalar(
                            'loss/iter', loss, global_step)
                    loss.backward()
                    model_optimizer.step()

                running_loss += loss.item()

                if (batch_idx > 0) and (batch_idx % print_every == 0):
                    iter_time = time.time() - iter_start_time
                    iter_times_list.append(iter_time)
                    iter_start_time = time.time()
                    print(f'[{phase}] Epoch {epoch}  Iter Time:{iter_time:.3f}  Step:{batch_idx}/{len(dataloaders[phase]):<8}  l: {running_loss/batch_idx:<10}')

                batch_idx += 1
                if phase == 'train':
                    global_step += 1

            total_loss = running_loss/len(dataloaders[phase])
            epoch_time_str = time.strftime("%H:%M:%S", time.gmtime(time.time() - epoch_start_time))
            print(f'Totals: {phase}  loss: {total_loss}  time: {epoch_time_str}')
            print()

            if phase == 'train':
                save_checkpoint(epoch=epoch,
                                model=model,
                                model_params=model.module.params if multi_gpu else model.params,
                                optimizer=model_optimizer,
                                optimizer_params=dict(lr=learning_rate),
                                loss=total_loss,
                                global_step=global_step,
                                save_path=osp.join(save_dir, f'{epoch:02d}.pt'))
            else:
                model_logger.val.add_scalar(
                    'loss/iter', total_loss, global_step)

    total_train_time_str = time.strftime("%H:%M:%S", time.gmtime(sum(iter_times_list)))
    print(f'Total training time: {total_train_time_str}')


def main(local_config, config):
    # Training params
    start_epoch = 1
    num_epochs = config.training.num_epochs
    batch_size = config.training.batch_sz
    dropouts = config.training.dropouts
    learning_rate = config.training.learning_rate

    # Data loading params
    num_audio_files = config.data.num_train_files
    dataload_workers = config.data.num_dataload_workers
    data_input_win_sz = config.data.input_win_sz
    data_output_win_sz = config.data.output_win_sz
    data_win_stride = config.data.win_stride
    train_dataset_path = osp.join(local_config.datasets_dir, config.data.train_path)
    valid_dataset_path = osp.join(local_config.datasets_dir, config.data.valid_path)

    # Model params
    input_size = config.model.input_sz
    hidden_sizes = config.model.hidden_szs
    output_size = config.model.output_sz
    model_save_dir = osp.join(local_config.models_dir, config.model.save_dir)
    Path(model_save_dir).mkdir(parents=True, exist_ok=True)

    # Log params
    log_dir = osp.join(local_config.logs_dir, config.log.save_dir)
    Path(log_dir).mkdir(parents=True, exist_ok=True)
    log_start_step = 0

    # Create device under which the model will be trained
    device_str = 'cpu'
    if torch.cuda.is_available():
        num_gpus = torch.cuda.device_count()
        if config.training.multi_gpu and num_gpus > 1:
            device_str = 'cuda'
        else:
            target_gid = int(args.gpu_id)
            if target_gid >= 0 and target_gid < num_gpus:
                device_str = f'cuda:{target_gid}'

    print(f'Training on device: {device_str}')
    device = torch.device(device_str)

    # Create dataset
    print('Loading Training data')
    train_dataset = CausalTongueMocapDataset(train_dataset_path,
                                             num_files=num_audio_files,
                                             input_win_sz=data_input_win_sz,
                                             output_win_sz=data_output_win_sz,
                                             stride=data_win_stride,
                                             pose_only=False)
    print(f'Training samples:    {len(train_dataset)}')

    print('Loading Validation data')
    valid_dataset = CausalTongueMocapDataset(valid_dataset_path,
                                             num_files=num_audio_files,
                                             input_win_sz=data_input_win_sz,
                                             output_win_sz=data_output_win_sz,
                                             stride=data_win_stride,
                                             pose_only=False)
    print(f'Validation samples:  {len(valid_dataset)}')

    # Create dataloaders
    train_dataloader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=dataload_workers, pin_memory=True)
    valid_dataloader = DataLoader(valid_dataset, batch_size=batch_size, shuffle=False, num_workers=dataload_workers, pin_memory=True)
    dataloaders = dict(train=train_dataloader,
                       valid=valid_dataloader)

    # Build if no checkpoint is given
    model = None
    if 'checkpoint' not in config:
        print('Building new model')
        model = MLP(input_dim=input_size,
                    hidden_dims=hidden_sizes,
                    output_dim=output_size,
                    dropouts=dropouts)
        if config.training.multi_gpu:
            model = nn.DataParallel(model)

        print('Building new optimizer')
        model_optimizer = optim.Adam(model.parameters(), lr=learning_rate)
    else:
        print('Loading checkpoint')
        checkpoint_path = osp.join(local_config.models_dir, config.checkpoint.path)
        model, model_optimizer, log_start_step = load_checkpoint(checkpoint_path=checkpoint_path,
                                                                 model_class=MLP,
                                                                 optimizer_class=optim.Adam,
                                                                 device=device)
        start_epoch = int(Path(checkpoint_path).stem) + 1

    model.to(device)

    print_training_header(learning_rate, dropouts,
                          batch_size, model, model_save_dir, log_dir)

    train(model, model_optimizer,
          dataloaders=dataloaders,
          save_dir=model_save_dir, log_dir=log_dir,
          batch_size=batch_size, learning_rate=learning_rate,
          start_epoch=start_epoch, n_epochs=num_epochs,
          loss_str=config.training.loss,
          device=device,
          multi_gpu=config.training.multi_gpu,
          log_start_step=log_start_step,
          print_every=200)


if __name__ == '__main__':
    # Parse input arguments
    args = parse_args()
    # Load configurations
    yaml = YAML(typ='safe')
    # -- machine configuration
    local_config = edict(yaml.load(open(args.local_config_path)))
    # -- training configuration
    config = edict(yaml.load(open(args.config_path)))

    main(local_config, config)


```

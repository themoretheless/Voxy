# Facial rendering source scan: salmedina/SpeechDrivenTongueAnimation

Repository: https://github.com/salmedina/SpeechDrivenTongueAnimation

Pinned source revision: c578efe92d1ef2e2c5f89dcf2817d0592fea2741

Discovery topic: tongue animation

Description: ML-driven tongue animation (CVPR'22)

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: src/utils.py

{
  "path": "src/utils.py",
  "blob_sha": "ca2ae8961319b185f076b6fe7b89d3d8fd70ea37",
  "sha256": "50855fe94a756010c829a6b851e3102395b668a17e4b52d9930e3cca39474722",
  "lines": 86,
  "bytes": 2398,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/salmedina/SpeechDrivenTongueAnimation/blob/c578efe92d1ef2e2c5f89dcf2817d0592fea2741/src/utils.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def seed_everything(seed: int) -> None:"
    },
    {
      "line": 25,
      "text": "def init_torch_device(gpu_id: int=-1) -> torch.device:"
    },
    {
      "line": 40,
      "text": "def load_audio(path: str) -> tuple:"
    },
    {
      "line": 60,
      "text": "def calc_seq_dist(gt_tensor: torch.Tensor,"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
'''
author: Salvador Medina
'''
import os
import random
import numpy as np
import torch
import soundfile as sf


def seed_everything(seed: int) -> None:
    """ Seed all random number generators

    Args:
        seed (int): Seed number for all random numbergenerators
    """
    os.environ["PL_GLOBAL_SEED"] = str(seed)
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)


def init_torch_device(gpu_id: int=-1) -> torch.device:
    """ Creates torch device based on the GPU ID

    Args:
        gpu_id (int, optional): CUDA device index, CPU if < 0
    """
    device_str = 'cpu'
    if torch.cuda.is_available():
        num_gpus = torch.cuda.device_count()
        if gpu_id >= 0 and gpu_id < num_gpus:
            device_str = f'cuda:{gpu_id}'

    return torch.device(device_str)


def load_audio(path: str) -> tuple:
    """ Load and normalize audio file

    Args:
        path (str): Path to the audio file

    Returns:
        np.array, int: audio signal array, sample rate
    """
    audio_signal, sample_rate = sf.read(path, dtype='int16')
    # Audio normalization
    audio_signal = audio_signal.astype('float32') / 32767 
    if len(audio_signal.shape) > 1:
        if audio_signal.shape[1] == 1:
            audio_signal = audio_signal.squeeze()
        else:
            audio_signal = audio_signal.mean(axis=1)  # mean value across all channels
    return audio_signal, sample_rate


def calc_seq_dist(gt_tensor: torch.Tensor,
                  pred_tensor: torch.Tensor) -> float:
    """ Calculates the sequence MSE from tensors

    Args:
        gt_tensor (torch.Tensor): ground truth
        pred_tensor (torch.Tensor): predictions values

    Returns:
        float: MSE
    """
    # Reshape the tensors to 3D coords
    gt_tensor = gt_tensor.reshape(-1, 10, 3)
    pred_tensor = pred_tensor.reshape(-1, 10, 3)

    # Calculate batch L2 error
    # -- Axis; 0: frames, 1:joints, 2:coords
    pose_delta = gt_tensor - pred_tensor
    error_tensor = torch.sqrt(torch.sum(
                              torch.mul(pose_delta, pose_delta),
                              axis=2))
    # mean across the joints through the sequence,
    # then the mean across the joints
    joints_error = error_tensor.mean(axis=0)
    sample_error = joints_error.mean().cpu().item()

    return sample_error

```

## Source file: src/train_gru.py

{
  "path": "src/train_gru.py",
  "blob_sha": "25af4a6d03209c02171bb8b303514a1eb16113e0",
  "sha256": "8b62ac064892ee23a14720eea568554d2674b2ba8558219ce9bb1a5f615e909f",
  "lines": 384,
  "bytes": 14305,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 5,
    "tongue": 3
  },
  "url": "https://github.com/salmedina/SpeechDrivenTongueAnimation/blob/c578efe92d1ef2e2c5f89dcf2817d0592fea2741/src/train_gru.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def parse_args():"
    },
    {
      "line": 37,
      "text": "def print_training_header(model, config):"
    },
    {
      "line": 50,
      "text": "def get_criterion(loss):"
    },
    {
      "line": 69,
      "text": "def train_epoch(epoch, model, optimizer, criterion, data_loader,"
    },
    {
      "line": 131,
      "text": "def validate_epoch(epoch, model, optimizer, criterion, data_loader, dataset,"
    },
    {
      "line": 226,
      "text": "def run(model, optimizer, dataloaders, valid_dataset, config,"
    },
    {
      "line": 270,
      "text": "def main(local_config, config):"
    }
  ]
}

```text

import argparse
import os
import os.path as osp
import time
from io import open
from pathlib import Path

import torch
import torch.nn as nn
from easydict import EasyDict as edict
from ruamel.yaml import YAML
from torch import optim
from torch.utils.data import DataLoader
from tqdm import tqdm

from dataset import TongueMocapDataset
from logger.model_logger import ModelLogger
from models import GRU, save_checkpoint
from utils import calc_seq_dist, init_torch_device, seed_everything

seed_everything(78373)

loss_iter_str = 'loss/iter'


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('-l', '--local_config_path', type=str,
                        help='Path to the local machine configuration')
    parser.add_argument('-c', '--config_path', type=str,
                        help='Path to the experiment configuration')
    parser.add_argument('-gid', '--gpu_id', type=int, default=0,
                        help='GPU to be used for training')
    return parser.parse_args()


def print_training_header(model, config):
    print('\nTraining Model')
    print('=' * 70)
    print(f'LR:         {config.training.learning_rate}')
    print(f'dropout:    {config.training.dropout}')
    print(f'batch size: {config.training.batch_sz}')
    print(f'model dir:  {config.paths.save_dir}')
    print(f'log dir:    {config.paths.log_dir}')
    print(f'model:{model}')
    print('=' * 70)
    print()


def get_criterion(loss):
    """Creates a criterion from a label for MSE, Huber, and L1, otherwise None

    Args:
        loss_label (str): loss label string

    Returns:
        nn.criterion: pytorch loss calculator
    """
    if loss.label == 'mse':
        return nn.MSELoss()
    elif loss.label == 'huber':
        return nn.SmoothL1Loss()
    elif loss.label == 'l1':
        return nn.L1Loss()

    return None


def train_epoch(epoch, model, optimizer, criterion, data_loader,
                config, model_logger, global_step, device, print_every):
    print('Training phase')
    model.train()

    epoch_start_time = time.time()
    iter_start_time = time.time()
    dataloader_iter = iter(data_loader)
    running_loss = 0.
    batch_idx = 0
    total_loss = 0.0
    iter_times_list = list()
    h0 = model.init_hidden(config.training.batch_sz).to(device)

    for input_tensor, output_tensor in dataloader_iter:
        # skip the batch that is not complete
        if input_tensor.shape[0] != config.training.batch_sz:
            continue
        global_step += 1
        batch_idx += 1

        h = h0.data
        y_hat, _ = model(x=input_tensor.to(device).float(), h=h)
        loss = criterion(y_hat, output_tensor.to(device).float())
        optimizer.zero_grad()
        if batch_idx % print_every == 0:
            model_logger.train.add_scalar(loss_iter_str, loss, global_step)
        loss.backward()
        optimizer.step()

        running_loss += loss.item()

        if batch_idx % print_every == 0:
            iter_time = time.time() - iter_start_time
            iter_times_list.append(iter_time)
            iter_start_time = time.time()
            print(f'[Training] Epoch {epoch}', end='')
            print(f'  Iter Time:{iter_time:.3f}', end='')
            print(f'  Step:{batch_idx}/{len(data_loader):<8}', end='')
            print(f'  l: {running_loss/batch_idx:<10}')

    total_loss = running_loss/len(data_loader)
    epoch_time_str = time.strftime("%H:%M:%S",
                                   time.gmtime(time.time() - epoch_start_time))
    print(f'Totals: Training  loss: {total_loss}  time: {epoch_time_str}')
    print()

    model_save_path = osp.join(config.paths.save_dir,
                               f'{epoch:02d}_{total_loss:06f}.pt')

    save_checkpoint(epoch=epoch,
                    model=model,
                    model_params=model.params,
                    optimizer=optimizer,
                    optimizer_params=dict(lr=config.training.learning_rate),
                    loss=total_loss,
                    global_step=global_step,
                    save_path=model_save_path)

    return global_step


def validate_epoch(epoch, model, optimizer, criterion, data_loader, dataset,
                   config, model_logger, global_step, best, device,
                   print_every):
    with torch.no_grad():
        print('Validation phase')
        model.eval()

        epoch_start_time = time.time()
        iter_start_time = time.time()

        dataloader_iter = iter(data_loader)
        running_loss = 0.
        batch_idx = 0
        total_loss = 0.0
        iter_times_list = list()
        h0 = model.init_hidden(config.training.batch_sz).to(device)

        # --- Report Validation Loss
        for input_tensor, output_tensor in dataloader_iter:
            if input_tensor.shape[0] != config.training.batch_sz:
                continue

            batch_idx += 1
            h = h0.data
            pos_pred, _ = model(x=input_tensor.to(device).float(), h=h)
            loss = criterion(pos_pred, output_tensor.to(device).float())
            optimizer.zero_grad()

            running_loss += loss.item()

            if batch_idx % print_every == 0:
                iter_time = time.time() - iter_start_time
                iter_times_list.append(iter_time)
                iter_start_time = time.time()
                print(f'[Validation] Epoch {epoch}', end='  ')
                print(f'Iter Time:{iter_time:.3f}', end='  ')
                print(f'Step:{batch_idx}/{len(data_loader):<8}', end='  ')
                print(f'l: {running_loss/batch_idx:<10}')

        total_loss = running_loss/len(data_loader)
        epoch_time_str = time.strftime("%H:%M:%S",
                                       time.gmtime(time.time() - epoch_start_time))
        print(f'Totals: Validation  loss: {total_loss}  time: {epoch_time_str}')
        print()

        # --- Report Error (L2 dist)
        error_tensor = torch.zeros(dataset.get_num_files())
        test_pbar = tqdm(list(range(dataset.get_num_files())))
        for sid in test_pbar:
            test_pbar.set_description(f'Sample Num: {sid}')
            sample_feats, sample_pos, _ = dataset.get_sample_by_index(sid)
            sample_feats = torch.Tensor(sample_feats).unsqueeze(0).to(device).float()
            sample_pos = torch.Tensor(sample_pos).to(device)
            pred_pos = model.infer(sample_feats).squeeze(0)

            seq_len = min(len(pred_pos), len(sample_pos))
            error_tensor[sid] = calc_seq_dist(pred_pos[:seq_len],
                                              sample_pos[:seq_len])

        total_error = error_tensor.mean().item()
        print(f'Totals: Validation error: {total_error:06f}')
        print()

        model_logger.val.add_scalar(loss_iter_str, total_loss, global_step)
        model_logger.test.add_scalar(loss_iter_str, total_error, global_step)
        
        if total_error < best.error:
            best.loss = total_loss
            best.error = total_error
            best.epoch = epoch
            if osp.exists(best.path):
                os.remove(best.path)
            best.path = osp.join(config.paths.save_dir,
                                 f'best_{epoch:02d}_{total_error:06f}.pt')
            save_checkpoint(epoch=epoch,
                            model=model,
                            model_params=model.params,
                            optimizer=optimizer,
                            optimizer_params=dict(lr=config.training.learning_rate),
                            loss=total_loss,
                            global_step=global_step,
                            save_path=best.path)
        else:
            if (epoch - best.epoch) >= config.training.early_stop:
                print(f'Early Stop @ Epoch {epoch}')
                print('Best model:')
                print(f'    Epoch:      {best.epoch}')
                print(f'    Val Loss:   {best.loss}')
                print(f'    Val Error:  {best.error}')
                print(f'    Path:       {best.path}')
                exit(0)

    return best


def run(model, optimizer, dataloaders, valid_dataset, config,
        start_epoch, log_start_step, device, print_every):

    model_logger = ModelLogger(config.paths.log_dir, f'test')
    criterion = get_criterion(config.training.loss)
    criterion.to(device)

    n_epochs = config.training.num_epochs
    start_time = time.time()
    global_step = log_start_step
    best = edict(loss=1e9, error=1e9, epoch=-1, path='best.pt')
    for epoch in range(start_epoch, start_epoch + n_epochs):
        print(f'Epoch {epoch}/{start_epoch + n_epochs - 1}')
        print('-' * 70)

        global_step = train_epoch(epoch,
                                  model,
                                  optimizer,
                                  criterion,
                                  dataloaders['train'],
                                  config,
                                  model_logger,
                                  global_step,
                                  device,
                                  print_every)
        best = validate_epoch(epoch,
                              model,
                              optimizer,
                              criterion,
                              dataloaders['valid'],
                              valid_dataset,
                              config,
                              model_logger,
                              global_step,
                              best,
                              device,
                              print_every)

    # --- Print total time
    total_time = time.time() - start_time
    total_train_time_str = time.strftime("%H:%M:%S", time.gmtime(total_time))
    print(f'Total training time: {total_train_time_str}')


def main(local_config, config):
    # --- Init params
    start_epoch = 1
    log_start_step = 0

    # --- Build paths
    train_dataset_path = osp.join(local_config.datasets_dir,
                                  config.data.train_path)
    valid_dataset_path = osp.join(local_config.datasets_dir,
                                  config.data.valid_path)

    model_save_dir = osp.join(local_config.models_dir,
                              config.model.save_dir)
    Path(model_save_dir).mkdir(parents=True, exist_ok=True)

    log_dir = osp.join(local_config.logs_dir,
                       config.log.save_dir)
    Path(log_dir).mkdir(parents=True, exist_ok=True)

    config['paths'] = dict()
    config.paths.save_dir = model_save_dir
    config.paths.log_dir = log_dir
    config.paths.train_dataset = train_dataset_path
    config.paths.valid_dataset = valid_dataset_path

    # --- Create torch device
    device = init_torch_device(args.gpu_id)

    # --- Create datasets
    print('Loading Training data')
    train_dataset = TongueMocapDataset(train_dataset_path,
                                       num_files=config.data.num_train_files,
                                       win_sz=config.data.win_sz,
                                       stride=config.data.win_stride,
                                       pose_only=True)
    print(f'Training samples:    {len(train_dataset)}')

    print('Loading Validation data')
    valid_dataset = TongueMocapDataset(valid_dataset_path,
                                       num_files=config.data.num_train_files,
                                       win_sz=config.data.win_sz,
                                       stride=config.data.win_stride,
                                       pose_only=True)
    print(f'Validation samples:  {len(valid_dataset)}')

    # --- Build data loaders
    train_dataloader = DataLoader(train_dataset,
                                  batch_size=config.training.batch_sz,
                                  shuffle=True,
                                  num_workers=config.data.num_dataload_workers,
                                  pin_memory=True)
    valid_dataloader = DataLoader(valid_dataset,
                                  batch_size=config.training.batch_sz,
                                  shuffle=False,
                                  num_workers=config.data.num_dataload_workers,
                                  pin_memory=True)
    dataloaders = dict(train=train_dataloader,
                       valid=valid_dataloader)

    # --- Build model from checkpoint or scratch
    if 'checkpoint' not in config:
        print('Building new model')
        rnn = GRU(config.model.input_sz,
                  config.model.hidden_sz,
                  config.model.output_sz,
                  n_layers=config.model.num_layers,
                  dropout=config.training.dropout,
                  bidirectional=config.model.bidir,
                  embedding_dim=config.model.embedding_sz)
        rnn.to(device)

        print('Building new optimizer')
        rnn_optimizer = optim.Adam(rnn.parameters(),
                                   lr=config.training.learning_rate)
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

    print_training_header(rnn, config)

    run(rnn,
        rnn_optimizer,
        dataloaders,
        valid_dataset,
        config,
        start_epoch,
        log_start_step,
        device,
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

## Source file: src/models/gru.py

{
  "path": "src/models/gru.py",
  "blob_sha": "7be87dabce4ffb0809fb5bb6f2b6f585ecac6cff",
  "sha256": "b25990456a42205360f79b94cf881314760d4f1390212996d2b684323865c7c5",
  "lines": 79,
  "bytes": 2624,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/salmedina/SpeechDrivenTongueAnimation/blob/c578efe92d1ef2e2c5f89dcf2817d0592fea2741/src/models/gru.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, input_dim, hidden_dim, output_dim, n_layers,"
    },
    {
      "line": 48,
      "text": "def forward(self, x, h):"
    },
    {
      "line": 55,
      "text": "def infer(self, x):"
    },
    {
      "line": 60,
      "text": "def init_hidden(self, batch_size):"
    },
    {
      "line": 67,
      "text": "def save_model(self, save_path):"
    },
    {
      "line": 73,
      "text": "def load_model(checkpoint_path, device):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
@author: Salvador Medina
"""

import torch
import torch.nn as nn

__all__ = [
    'GRU'
]


class GRU(nn.Module):
    def __init__(self, input_dim, hidden_dim, output_dim, n_layers,
                 dropout=0.2, bidirectional=False, embedding_dim=0):
        super(GRU, self).__init__()
        self.input_dim = input_dim
        self.hidden_dim = hidden_dim
        self.output_dim = output_dim
        self.embedding_dim = embedding_dim
        self.n_layers = n_layers
        self.dropout = dropout
        self.bidirectional = bidirectional
        self.bidir_factor = 2 if bidirectional else 1

        self.feat_fc = None
        if embedding_dim > 0:
            self.feat_fc = nn.Linear(input_dim, embedding_dim)
        gru_input_dim = embedding_dim if embedding_dim > 0 else input_dim
        self.gru = nn.GRU(gru_input_dim,
                          hidden_dim,
                          n_layers,
                          batch_first=True,
                          dropout=dropout,
                          bidirectional=bidirectional)
        self.relu = nn.ReLU()
        self.fc = nn.Linear(hidden_dim * self.bidir_factor, output_dim)

        self.params = dict(input_dim=self.input_dim,
                           hidden_dim=self.hidden_dim,
                           output_dim=self.output_dim,
                           embedding_dim=self.embedding_dim,
                           n_layers=self.n_layers,
                           dropout=self.dropout,
                           bidirectional=self.bidirectional)

    def forward(self, x, h):
        if self.feat_fc is not None:
            x = self.feat_fc(x)
        out, h = self.gru(x, h)
        out = self.fc(self.relu(out))
        return out, h

    def infer(self, x):
        h0 = self.init_hidden(x.shape[0])
        out, _ = self.forward(x, h0)
        return out

    def init_hidden(self, batch_size):
        weight = next(self.parameters()).data
        hidden = weight.new(self.n_layers * self.bidir_factor,
                            batch_size,
                            self.hidden_dim).zero_()
        return hidden

    def save_model(self, save_path):
        torch.save({'model_params': self.params,
                    'model_state_dict': self.state_dict()
                    }, save_path)

    @staticmethod
    def load_model(checkpoint_path, device):
        checkpoint = torch.load(checkpoint_path, map_location=str(device))
        model = GRU(**checkpoint['model_params'])
        model.load_state_dict(checkpoint['model_state_dict'])
        model.to(device)

        return model


```

## Source file: src/models/utils.py

{
  "path": "src/models/utils.py",
  "blob_sha": "630459f99f63ad640a8b898f94cedfbd6b3784cb",
  "sha256": "5f1833c80e26ca5da6440a49ffe556dc5574eadb4fe911531251f015506eeaef",
  "lines": 80,
  "bytes": 2608,
  "selection_score": 0,
  "keyword_line_counts": {
    "tongue": 1
  },
  "url": "https://github.com/salmedina/SpeechDrivenTongueAnimation/blob/c578efe92d1ef2e2c5f89dcf2817d0592fea2741/src/models/utils.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def save_checkpoint(epoch, model, model_params, optimizer, optimizer_params,"
    },
    {
      "line": 26,
      "text": "def load_checkpoint(checkpoint_path, model_class, optimizer_class, device):"
    },
    {
      "line": 44,
      "text": "def save_model(epoch, model, model_params, save_path):"
    },
    {
      "line": 52,
      "text": "def load_model(checkpoint_path, model_class, device):"
    },
    {
      "line": 67,
      "text": "def print_checkpoint(checkpoint_path, name, params_dict):"
    },
    {
      "line": 75,
      "text": "def count_total_params(model):"
    },
    {
      "line": 79,
      "text": "def count_trainable_params(model):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""
@author: Salvador Medina
"""

import torch

__all__ = ['save_checkpoint', 'load_checkpoint', 'print_checkpoint',
           'save_model', 'load_model',
           'count_total_params', 'count_trainable_params']

def save_checkpoint(epoch, model, model_params, optimizer, optimizer_params,
                    loss, global_step, save_path):
    torch.save({'epoch': epoch,
                'model_class': model.__class__.__name__,
                'model_params': model_params,
                'model_state_dict': model.state_dict(),
                'optimizer_class': optimizer.__class__.__name__,
                'optimizer_params': optimizer_params,
                'optimizer_state_dict': optimizer.state_dict(),
                'loss': loss,
                'global_step': global_step
                }, save_path)


def load_checkpoint(checkpoint_path, model_class, optimizer_class, device):
    checkpoint = torch.load(checkpoint_path, map_location=str(device))

    # Load model
    model = model_class(**checkpoint['model_params'])
    model.load_state_dict(checkpoint['model_state_dict'])
    model.to(device)

    # Load optimizer
    optimizer = optimizer_class(model.parameters(), **checkpoint['optimizer_params'])
    optimizer.load_state_dict(checkpoint['optimizer_state_dict'])

    # Load log status
    log_step = checkpoint['global_step']

    return model, optimizer, log_step


def save_model(epoch, model, model_params, save_path):
    torch.save({'epoch': epoch,
                'model_class': model.__class__.__name__,
                'model_params': model_params,
                'model_state_dict': model.state_dict()
                }, save_path)


def load_model(checkpoint_path, model_class, device):
    checkpoint = torch.load(checkpoint_path, map_location=str(device))

    # TODO: Hack to remove drop path rate in tongueformer
    if 'drop_path_rate' in checkpoint['model_params']:
        checkpoint['model_params']['drop_path_rate'] = 0.0

    # Load model
    model = model_class(**checkpoint['model_params'])
    model.load_state_dict(checkpoint['model_state_dict'])
    model.to(device)

    return model


def print_checkpoint(checkpoint_path, name, params_dict):
    checkpoint = torch.load(checkpoint_path)

    print(f'{"Name":>13}: {checkpoint["model_name"]}')
    for k, v in checkpoint['model_params'].items():
        print(f'{k:>13}: {v}')


def count_total_params(model):
    return sum(p.numel() for p in model.parameters())


def count_trainable_params(model):
    return sum(p.numel() for p in model.parameters() if p.requires_grad)

```

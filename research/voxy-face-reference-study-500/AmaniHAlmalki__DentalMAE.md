# Facial rendering source scan: AmaniHAlmalki/DentalMAE

Repository: https://github.com/AmaniHAlmalki/DentalMAE

Pinned source revision: a4860fe3df349815e2af6c29b786dfd059fdfcdc

Discovery topic: teeth 3D

Description: Code for WACV 2024 paper "Self-Supervised Learning with Masked Autoencoders for Teeth Segmentation from Intra-oral 3D Scans"

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: train_seg.py

{
  "path": "train_seg.py",
  "blob_sha": "51752e8f5aafa8c21d797fee888377f414123b43",
  "sha256": "5b019c304221c5e43fa306704d35c5012e4b77d1e166399aefdd57dea61b6279",
  "lines": 239,
  "bytes": 9840,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 1,
    "gum": 35,
    "deform": 3,
    "face": 20
  },
  "url": "https://github.com/AmaniHAlmalki/DentalMAE/blob/a4860fe3df349815e2af6c29b786dfd059fdfcdc/train_seg.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def seed_torch(seed=42):"
    },
    {
      "line": 32,
      "text": "def train(net, optim, criterion, train_dataset, epoch, args):"
    },
    {
      "line": 81,
      "text": "def test(net, criterion, test_dataset, epoch, args):"
    }
  ]
}

```text

import torch
import os
import sys
from torch.autograd import Variable
import argparse
from tensorboardX import SummaryWriter
import copy
import torch.optim as optim
from torch.optim.lr_scheduler import MultiStepLR, CosineAnnealingLR
import numpy as np
import torch.nn as nn
import torch.utils.data as data
import random
# os.environ['CUDA_VISIBLE_DEVICES'] = '0'
from model.dataset import SegmentationDataset
from model.dentalmae import Mesh_baseline_seg
from model.reconstruction import save_results
import sys

sys.setrecursionlimit(3000)


def seed_torch(seed=42):
    random.seed(seed)
    os.environ['PYTHONHASHSEED'] = str(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed(seed)
    torch.backends.cudnn.deterministic = True


def train(net, optim, criterion, train_dataset, epoch, args):
    net.train()
    running_loss = 0
    running_corrects = 0
    n_samples = 0
    patch_size = 64
    num_of_patch = 0
    for i, (face_patch, feats_patch, np_Fs, center_patch, coordinate_patch, labels) in enumerate(
            train_dataset):
        optim.zero_grad()
        faces = face_patch.cuda()
        patch_size = faces.size(2)
        num_of_patch = faces.size(1)

        feats = feats_patch.to(torch.float32).cuda()
        centers = center_patch.to(torch.float32).cuda()
        Fs = np_Fs.cuda()
        cordinates = coordinate_patch.to(torch.float32).cuda()
        labels = labels.to(torch.long).cuda()

        labels = labels.reshape(faces.shape[0], -1)
        n_samples += faces.shape[0]

        outputs, outputs_seg = net(faces, feats, centers, Fs, cordinates)
        outputs = outputs.reshape(faces.shape[0], -1, args.seg_parts).permute(0, 2, 1)
        outputs_seg = outputs_seg.reshape(faces.shape[0], -1, args.seg_parts).permute(0, 2, 1)

        dim = outputs.shape[1]

        loss = criterion(outputs, labels)
        loss_seg = criterion(outputs_seg, labels)
        loss = args.lw1 * loss + args.lw2 * loss_seg

        _, preds = torch.max(outputs_seg, 1)
        running_corrects += torch.sum(preds == labels.data)

        loss.backward()
        optim.step()
        running_loss += loss.item() * faces.size(0)
    epoch_loss = running_loss / n_samples
    epoch_acc = running_corrects / n_samples / num_of_patch / patch_size
    print('epoch: {:} Train Loss: {:.4f} Acc: {:.4f}'.format(epoch, epoch_loss, epoch_acc))
    message = 'epoch: {:} Train Loss: {:.4f} Acc: {:.4f}\n'.format(epoch, epoch_loss, epoch_acc)
    with open(os.path.join('checkpoints', name, 'log.txt'), 'a') as f:
        f.write(message)




def test(net, criterion, test_dataset, epoch, args):
    net.eval()
    acc = 0
    running_loss = 0
    running_corrects = 0
    n_samples = 0
    for i, (face_patch, feats_patch, np_Fs, center_patch, coordinate_patch, labels) in enumerate(
            test_dataset):

        faces = face_patch.cuda()
        feats = feats_patch.to(torch.float32).cuda()
        centers = center_patch.to(torch.float32).cuda()
        Fs = np_Fs.cuda()
        cordinates = coordinate_patch.to(torch.float32).cuda()

        labels = labels.to(torch.long).cuda()
        labels = labels.reshape(faces.shape[0], -1)
        n_samples += faces.shape[0]
        with torch.no_grad():
            outputs, outputs_seg = net(faces, feats, centers, Fs, cordinates)
        outputs = outputs.reshape(faces.shape[0], -1, args.seg_parts).permute(0, 2, 1)
        outputs_seg = outputs_seg.reshape(faces.shape[0], -1, args.seg_parts).permute(0, 2, 1)

        loss = criterion(outputs, labels)
        loss_seg = criterion(outputs_seg, labels)
        loss = 0.5 * loss + 0.5 * loss_seg
        _, preds = torch.max(outputs_seg, 1)

        running_corrects += torch.sum(preds == labels.data)
        running_loss += loss.item() * faces.size(0)

    epoch_acc = running_corrects.double() / n_samples / 16384
    epoch_loss = running_loss / n_samples

    if test.best_acc < epoch_acc:
        test.best_acc = epoch_acc
        best_model_wts = copy.deepcopy(net.state_dict())
        torch.save(best_model_wts, os.path.join('checkpoints', name, 'best.pkl'))
    print('epoch: {:} test Loss: {:.4f} Acc: {:.4f} Best: {:.4f}'.format(epoch, epoch_loss, epoch_acc,test.best_acc))
    message = 'epoch: {:} test Loss: {:.4f} Acc: {:.4f} Best: {:.4f}\n'.format(epoch, epoch_loss, epoch_acc,
                                                                               test.best_acc)
    with open(os.path.join('checkpoints', name, 'log.txt'), 'a') as f:
        f.write(message)




if __name__ == '__main__':
    seed_torch(seed=42)
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['train', 'test'])
    parser.add_argument('--name', type=str, required=True)
    parser.add_argument('--dataroot', type=str, required=True)
    parser.add_argument('--checkpoint', type=str)
    parser.add_argument('--optim', choices=['adam', 'sgd', 'adamw'], default='adam')
    parser.add_argument('--lr', type=float, default=1e-3)
    parser.add_argument('--lr_milestones', type=int, default=None, nargs='+',)
    parser.add_argument('--heads', type=int, required=True)
    parser.add_argument('--dim', type=int, default=384)
    parser.add_argument('--encoder_depth', type=int, default=6)
    parser.add_argument('--weight_decay', type=float, default=0)
    parser.add_argument('--decoder_depth', type=int, default=6)
    parser.add_argument('--decoder_dim', type=int, default=512)
    parser.add_argument('--decoder_num_heads', type=int, default=6)
    parser.add_argument('--patch_size', type=int, required=True)
    parser.add_argument('--batch_size', type=int, default=48)
    parser.add_argument('--n_epoch', type=int, default=100)
    parser.add_argument('--max_epoch', type=int, default=300)
    parser.add_argument('--drop_path', type=float, default=0)
    parser.add_argument('--seed', type=int, default=None)
    parser.add_argument('--n_worker', type=int, default=4)
    parser.add_argument('--channels', type=int, default=10)
    parser.add_argument('--augment_scale', action='store_true')
    parser.add_argument('--augment_orient', action='store_true')
    parser.add_argument('--augment_deformation', action='store_true')
    parser.add_argument('--lw1', type=float, default=0.5)
    parser.add_argument('--lw2', type=float, default=0.5)
    parser.add_argument('--fpn', action='store_true')
    parser.add_argument('--face_pos', action='store_true')
    parser.add_argument('--lr_min', type=float, default=1e-5)
    parser.add_argument('--gamma', type=float, default=0.1)
    parser.add_argument('--dataset_name', type=str, default='alien', choices=['alien', 'human'])
    parser.add_argument('--seg_parts', type=int, default=4)
    parser.add_argument('--mask_ratio', type=float, default=0.25)
    args = parser.parse_args()
    mode = args.mode
    name = args.name
    dataroot = args.dataroot
    # ========== Dataset ==========
    augments = []
    if args.augment_scale:
        augments.append('scale')
    if args.augment_orient:
        augments.append('orient')
    if args.augment_deformation:
        augments.append('deformation')
    train_dataset = SegmentationDataset(dataroot, train=True, augments=augments)
    test_dataset = SegmentationDataset(dataroot, train=False)
    print(len(train_dataset))
    print(len(test_dataset))

    train_data_loader = data.DataLoader(train_dataset, num_workers=args.n_worker, batch_size=args.batch_size,
                                        shuffle=True, pin_memory=True)
    test_data_loader = data.DataLoader(test_dataset, num_workers=args.n_worker, batch_size=args.batch_size,
                                       shuffle=False, pin_memory=True)
    # ========== Network ==========

    net = Mesh_baseline_seg(masking_ratio=args.mask_ratio,
                            channels=args.channels,
                            num_heads=args.heads,
                            encoder_depth=args.encoder_depth,
                            embed_dim=args.dim,
                            decoder_num_heads=args.decoder_num_heads,
                            decoder_depth=args.decoder_depth,
                            decoder_embed_dim=args.decoder_dim,
                            patch_size=args.patch_size,
                            drop_path=args.drop_path,
                            fpn=args.fpn,
                            face_pos=args.face_pos,
                            seg_part=args.seg_parts)

    device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
    net.to(device)
    # ========== Optimizer ==========
    if args.optim == 'adamw':
        optim = optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.weight_decay)

    if args.lr_milestones is not None:
        scheduler = MultiStepLR(optim, milestones=args.lr_milestones, gamma=args.gamma)
    else:

        scheduler = CosineAnnealingLR(optim, T_max=args.max_epoch, eta_min=args.lr_min, last_epoch=-1)

    criterion = nn.CrossEntropyLoss()
    checkpoint_path = os.path.join('checkpoints', name)
    #checkpoint_name = os.path.join(checkpoint_path, name + '-latest.pkl')

    os.makedirs(checkpoint_path, exist_ok=True)

    if args.checkpoint is not None:
        net.load_state_dict(torch.load(args.checkpoint), strict=False)

    train.step = 0
    test.best_acc = 0

    if args.mode == 'train':
        for epoch in range(args.n_epoch):
            # train_data_loader.dataset.set_epoch()
            print('iteration', epoch)
            train(net, optim, criterion, train_data_loader, epoch, args)
            print('train finished')
            test(net, criterion, test_data_loader, epoch, args)
            print('test finished')
            scheduler.step()
            print(optim.param_groups[0]['lr'])


    else:
        test(net, criterion, test_data_loader, 0, args)


```

## Source file: model/utils.py

{
  "path": "model/utils.py",
  "blob_sha": "c9b0bedf9ba4a9b37d392c8556def77e88cd6af2",
  "sha256": "eac0fdeb54a9a02fee1e98cc5f61ddf16ec77c9f703875081443b6bd705c8756",
  "lines": 34,
  "bytes": 1005,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/AmaniHAlmalki/DentalMAE/blob/a4860fe3df349815e2af6c29b786dfd059fdfcdc/model/utils.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, nclass):"
    },
    {
      "line": 13,
      "text": "def vote(self, mesh_paths, preds, labels):"
    },
    {
      "line": 29,
      "text": "def compute_accuracy(self):"
    }
  ]
}

```text

import os
import json
from pathlib import Path

import numpy as np
import trimesh
import torch
class ClassificationMajorityVoting:
    def __init__(self, nclass):
        self.votes = {}
        self.nclass = nclass

    def vote(self, mesh_paths, preds, labels):
        
        if isinstance(preds, torch.Tensor):
            preds = preds.data
        if isinstance(labels, torch.Tensor):
            labels = labels.data

        for i in range(preds.shape[0]):
            name = (Path(mesh_paths[i]).stem).split('-')[0]
            if not name in self.votes:
                self.votes[name] = {
                    'polls': np.zeros(self.nclass, dtype=int),
                    'label': labels[i]
                }
            self.votes[name]['polls'][preds[i]] += 1

    def compute_accuracy(self):
        sum_acc = 0
        for name, vote in self.votes.items():
            pred = np.argmax(vote['polls'])
            sum_acc += pred == vote['label']
        return sum_acc / len(self.votes)


```

## Source file: model/__init__.py

{
  "path": "model/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/AmaniHAlmalki/DentalMAE/blob/a4860fe3df349815e2af6c29b786dfd059fdfcdc/model/__init__.py",
  "function_signature_candidates": []
}

```text



```

## Source file: train_pretrain.py

{
  "path": "train_pretrain.py",
  "blob_sha": "7e0c954358bac7639d47cfa7b96c576778c70f2c",
  "sha256": "f34e76ea42e6045153504c5d033a2ae3c77a69a3f526924e5984706deb8f5cda",
  "lines": 189,
  "bytes": 7838,
  "selection_score": 0,
  "keyword_line_counts": {
    "skin": 1,
    "gum": 32,
    "deform": 3,
    "face": 8
  },
  "url": "https://github.com/AmaniHAlmalki/DentalMAE/blob/a4860fe3df349815e2af6c29b786dfd059fdfcdc/train_pretrain.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def train(net, optim, scheduler, names, train_dataset, epoch, args):"
    },
    {
      "line": 54,
      "text": "def test(net,  names, test_dataset, epoch, args):"
    }
  ]
}

```text

import torch
import os
import sys
from torch.autograd import Variable
import argparse
from tensorboardX import SummaryWriter
import copy
import torch.optim as optim
from torch.optim.lr_scheduler import MultiStepLR, CosineAnnealingLR
import numpy as np
import torch.nn as nn
import torch.utils.data as data
from model.dataset import ClassificationDataset
from model.dataset import ShapeNetDataset
from model.dentalmae import Mesh_mae
from model.reconstruction import save_results
from model.utils import ClassificationMajorityVoting
from transformers import AdamW, get_linear_schedule_with_warmup, get_constant_schedule, get_cosine_schedule_with_warmup
import time


def train(net, optim, scheduler, names, train_dataset, epoch, args):
    net.train()
    running_loss = 0

    n_samples = 0

    for it, (feats_patch, center_patch,coordinate_patch, face_patch,  np_Fs, label, mesh_paths) in enumerate(
            train_dataset):
        optim.zero_grad()
        faces = face_patch.to(torch.float32).cuda()
        feats = feats_patch.to(torch.float32).cuda()
        centers = center_patch.to(torch.float32).cuda()
        Fs = np_Fs.cuda()
        cordinates = coordinate_patch.to(torch.float32).cuda()
        n_samples += faces.shape[0]
        loss = net(faces, feats, centers, Fs, cordinates)
        loss.backward()
        optim.step()
        running_loss += loss.item() * faces.size(0)

    epoch_loss = running_loss / n_samples


    scheduler.step()
    if train.best_loss > epoch_loss:
        train.best_loss = epoch_loss
        train.best_epoch = epoch
        best_model_wts = copy.deepcopy(net.state_dict())
        torch.save(best_model_wts, os.path.join('checkpoints', names, f'loss-{epoch_loss:.4f}-{epoch:.4f}.pkl'))
    print('epoch ({:}): {:} Train Loss: {:.4f}'.format(names, epoch, epoch_loss))


def test(net,  names, test_dataset, epoch, args):
    #######################################################################
    # if you are going to show the reconstruct shape, please using the following codes
    #######################################################################
    net.eval()
    for it, (feats_patch, center_patch, coordinate_patch, face_patch, np_Fs, label, mesh_paths) in enumerate(
            test_dataset):

        faces = face_patch.cuda()
        feats = feats_patch.cuda()
        centers = center_patch.cuda()
        Fs = np_Fs.cuda()
        cordinates = coordinate_patch.cuda()

        with torch.no_grad():
            loss, masked_indices, unmasked_indices, pred_vertices_coordinates, cordinates = net(faces, feats, centers, Fs, cordinates)
        save_results(masked_indices, unmasked_indices, pred_vertices_coordinates, cordinates, mesh_paths)





if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['train', 'test'])
    parser.add_argument('--name', type=str, required=True)
    parser.add_argument('--checkpoint', type=str, default=None)
    parser.add_argument('--lr_milestones', type=str, default=None)
    parser.add_argument('--num_warmup_steps', type=str, default=None)

    parser.add_argument('--depth', type=int, required=True)
    parser.add_argument('--lr', type=float, default=1e-3)
    parser.add_argument('--weight_decay', type=float, default=0)
    parser.add_argument('--n_dropout', type=int, default=1)
    parser.add_argument('--encoder_depth', type=int, default=6)
    parser.add_argument('--decoder_depth', type=int, default=6)
    parser.add_argument('--decoder_dim', type=int, default=512)
    parser.add_argument('--decoder_num_heads', type=int, default=6)
    parser.add_argument('--dim', type=int, default=384)
    parser.add_argument('--weight', type=float, default=0.2)
    parser.add_argument('--optim', type=str, default='adam')
    parser.add_argument('--heads', type=int, required=True)
    parser.add_argument('--patch_size', type=int, required=True)
    parser.add_argument('--batch_size', type=int, default=32)
    parser.add_argument('--n_epoch', type=int, default=500)
    parser.add_argument('--max_epoch', type=int, default=300)
    parser.add_argument('--dataroot', type=str, required=True)
    parser.add_argument('--n_classes', type=int)
    parser.add_argument('--no_center_diff', action='store_true')
    parser.add_argument('--seed', type=int, default=None)
    parser.add_argument('--n_worker', type=int, default=8)
    parser.add_argument('--augment_scale', action='store_true')
    parser.add_argument('--augment_orient', action='store_true')
    parser.add_argument('--augment_deformation', action='store_true')
    parser.add_argument('--channels', type=int, default=10)
    parser.add_argument('--mask_ratio', type=float, default=0.25)

    args = parser.parse_args()
    mode = args.mode
    dataroot = args.dataroot

    # ========== Dataset ==========
    augments = []
    if args.augment_scale:
        augments.append('scale')
    if args.augment_orient:
        augments.append('orient')
    if args.augment_deformation:
        augments.append('deformation')

    if 'ShapeNet' in dataroot:
        train_dataset = ShapeNetDataset(dataroot, train=True, augment=augments)

    else:
        train_dataset = ClassificationDataset(dataroot, train=True, augment=augments)
        test_dataset = ClassificationDataset(dataroot, train=False)
        print(len(test_dataset))
        test_data_loader = data.DataLoader(test_dataset, num_workers=args.n_worker, batch_size=args.batch_size,
                                           shuffle=True, pin_memory=True)
    print(len(train_dataset))
    train_data_loader = data.DataLoader(train_dataset, num_workers=args.n_worker, batch_size=args.batch_size,
                                        shuffle=True, pin_memory=True)

    # ========== Network ==========
    device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
    net = Mesh_mae(masking_ratio=args.mask_ratio,
                   channels=args.channels,
                   num_heads=args.heads,
                   encoder_depth=args.encoder_depth,
                   embed_dim=args.dim,
                   decoder_num_heads=args.decoder_num_heads,
                   decoder_depth=args.decoder_depth,
                   decoder_embed_dim=args.decoder_dim,
                   patch_size=args.patch_size,
                   weight=args.weight
                   ).to(device)

    # ========== Optimizer ==========
    if args.optim.lower() == 'adamw':
        optim = optim.AdamW(net.parameters(), lr=args.lr, weight_decay=args.weight_decay)

    if args.lr_milestones.lower() != 'none':
        ms = args.lr_milestones
        ms = ms.split()
        ms = [int(j) for j in ms]
        scheduler = MultiStepLR(optim, milestones=ms, gamma=0.1)
    else:
        scheduler = get_cosine_schedule_with_warmup(optim, num_warmup_steps=int(args.num_warmup_steps),
                                                    num_training_steps=args.max_epoch + 1)

    print(scheduler)

    # ========== MISC ==========

    checkpoint_names = []
    checkpoint_path = os.path.join('checkpoints', args.name)
    os.makedirs(checkpoint_path, exist_ok=True)

    if args.checkpoint.lower() != 'none':
        net.load_state_dict(torch.load(args.checkpoint), strict=True)


    train.best_loss = 999
    train.best_epoch = 0
    # ========== Start Training ==========

    if args.mode == 'train':
        for epoch in range(args.n_epoch):
            print('epoch', epoch)
            train(net, optim, scheduler, args.name, train_data_loader, epoch, args)
            print('train finished')



    else:
        test(net,  args.name, test_data_loader, 0, args)


```

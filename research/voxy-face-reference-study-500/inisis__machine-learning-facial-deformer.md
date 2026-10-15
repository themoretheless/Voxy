# Facial rendering source scan: inisis/machine-learning-facial-deformer

Repository: https://github.com/inisis/machine-learning-facial-deformer

Pinned source revision: 2956804d893f073d77cf7224d99b73a708d7d816

Discovery topic: facial rig

Description: Machine Learning, Facial Rigger

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["model/neural_facial_rigger.py"]}

## Source file: model/neural_facial_rigger.py

{
  "path": "model/neural_facial_rigger.py",
  "blob_sha": "b64e4e47342dbce7da86f3062cd2de81259b2c55",
  "sha256": "2400c64bfcd99d3e9e6149b771b0886c01d2d61d8cf9c10a78392dc62b26a92a",
  "lines": 43,
  "bytes": 1338,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 5
  },
  "url": "https://github.com/inisis/machine-learning-facial-deformer/blob/2956804d893f073d77cf7224d99b73a708d7d816/model/neural_facial_rigger.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def __init__(self, cfg, in_dim = 256):"
    },
    {
      "line": 10,
      "text": "def forward(self, x):"
    },
    {
      "line": 21,
      "text": "def __init__(self, cfg):"
    },
    {
      "line": 31,
      "text": "def _make_layer(self, num_rig_param, depth, block):"
    },
    {
      "line": 39,
      "text": "def forward(self, ziva):"
    }
  ]
}

```text

from torch import nn


class BasicBlock(nn.Module):
    def __init__(self, cfg, in_dim = 256):
        super(BasicBlock, self).__init__()
        self.leaky_relu = nn.LeakyReLU(inplace=True)
        self.fc = nn.Linear(in_features=in_dim, out_features=256)

    def forward(self, x):
        identity = x
        out = self.leaky_relu(x)
        out = self.fc(out)
        out = out + identity

        return out


class NeuralFacialRigger(nn.Module):

    def __init__(self, cfg):
        super(NeuralFacialRigger, self).__init__()
        self.cfg = cfg
        self.layer1 = self._make_layer(self.cfg.TRAIN.NUM_RIG_PARAM, self.cfg.TRAIN.BASE_DEPTH, BasicBlock)
        self.layer2 = nn.Linear(in_features=256,
                                out_features=self.cfg.TRAIN.EMBEDDING_SIZE)
        self.layer3 = nn.Linear(
            in_features=self.cfg.TRAIN.EMBEDDING_SIZE,
            out_features=self.cfg.TRAIN.MESH_VERTEX_SIZE)

    def _make_layer(self, num_rig_param, depth, block):
        layers = []
        layers.append(nn.Linear(in_features=num_rig_param, out_features=256))
        for _ in range(1, depth):
            layers.append(block(self.cfg))

        return nn.Sequential(*layers)

    def forward(self, ziva):
        x = self.layer1(ziva)
        x = self.layer2(x)
        x = self.layer3(x)
        return x


```

## Source file: bin/pca.py

{
  "path": "bin/pca.py",
  "blob_sha": "e335a78e828efcd4a5a2334b3120b8b52ba10180",
  "sha256": "2e04bdcc57a83fca1f6aa866d335d64d8e1f41e4fed3080704a4bf109267ec41",
  "lines": 62,
  "bytes": 1619,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 4
  },
  "url": "https://github.com/inisis/machine-learning-facial-deformer/blob/2956804d893f073d77cf7224d99b73a708d7d816/bin/pca.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def get_parser():"
    },
    {
      "line": 28,
      "text": "def run(args):"
    },
    {
      "line": 56,
      "text": "def main():"
    }
  ]
}

```text

import os
import time
import argparse
import numpy as np

from sklearn.decomposition import PCA


def get_parser():
    parser = argparse.ArgumentParser(description='Principal Component Analysis')

    parser.add_argument('--train_file', type=str,
                        default='D:/data/csv/pca.csv',
                        help='The file path of the train data file')

    parser.add_argument('--n_components', type=int,
                        default=300,
                        help='The max epoch to train the network')

    parser.add_argument('--save_path', type=str,
                        default='D:/data/pca',
                        help='The max epoch to train the network')

    args = parser.parse_args()
    return args


def run(args):
    start_time = time.time()

    mesh_list = []
    if args.save_path is None:
        save_path = os.path.dirname(args.train_file)
    else:
        save_path = args.save_path

    with open(args.train_file) as f:
        for line in f:
            fields = line.strip('\n').split(',')
            mesh_vertex_file = fields[0]
            mesh_vertex = np.load(mesh_vertex_file)
            mesh_list.append(mesh_vertex)

    data = np.array(mesh_list)

    mean = np.mean(data, axis=0)
    np.save(os.path.join(save_path, "mean.npy"), mean)
    pca = PCA(n_components=args.n_components)
    pca.fit(data)
    end_time = time.time()
    print("Time:%d" % (end_time - start_time))
    np.save(os.path.join(save_path, "featVec.npy"), pca.components_)
    print("Done")


def main():
    args = get_parser()
    run(args)


if __name__ == '__main__':
    main()


```

## Source file: bin/train.py

{
  "path": "bin/train.py",
  "blob_sha": "56507b72865cf0da2d7f0e439a0cdfdca5d53b0a",
  "sha256": "e3a579cc5780f2a246b71ba17a54cdb2fa56048a46cbbaa122df1d7f9f4a90e2",
  "lines": 81,
  "bytes": 1951,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 7,
    "rig": 1,
    "face": 1
  },
  "url": "https://github.com/inisis/machine-learning-facial-deformer/blob/2956804d893f073d77cf7224d99b73a708d7d816/bin/train.py",
  "function_signature_candidates": [
    {
      "line": 42,
      "text": "def run(args):"
    },
    {
      "line": 75,
      "text": "def main():"
    }
  ]
}

```text

import os
import sys
import argparse

sys.path.append(os.path.dirname(os.path.abspath(__file__)) + '/../')

from engine.trainer import Trainer

parser = argparse.ArgumentParser(description='Face rigger trainer')
parser.add_argument(
    'cfg_file',
    default=None,
    type=str,
    help='Model config in yaml format')
parser.add_argument(
    'save_path',
    default=None,
    type=str,
    help='Path to save model')
parser.add_argument(
    '--num_workers',
    default=1,
    type=int,
    help='Number of workers for each dataloader')
parser.add_argument(
    '--device_ids',
    default='0',
    type=str,
    help='GPU indices comma separated')
parser.add_argument(
    '--logtofile',
    default=False,
    type=bool,
    help='Save log to save_path/log.txt if set True')
parser.add_argument(
    '--local_rank',
    default=0,
    type=int,
    help='DDP parameter, do not modify')


def run(args):
    trainer = Trainer(
        args.cfg_file,
        args.save_path,
        num_workers=args.num_workers,
        device_ids=args.device_ids,
        logtofile=args.logtofile,
        local_rank=args.local_rank)
    cfg = trainer.cfg

    epoch_steps = len(trainer.dataloader_train)
    total_steps = epoch_steps * cfg.TRAIN.EPOCH
    start_step = trainer.summary['step']

    for step in range(start_step, total_steps):
        trainer.train_step()

        if (step + 1) % cfg.TRAIN.LOG_EVERY == 0:
            if args.local_rank == 0:
                trainer.logging('Train')
                trainer.write_summary('Train')
            trainer.log_init()

        if (step + 1) % cfg.TRAIN.DEV_EVERY == 0 and args.local_rank == 0:
            trainer.dev_epoch()
            trainer.logging('Dev')
            trainer.write_summary('Dev')
            trainer.save_model('Train')
            trainer.save_model('Dev')

    trainer.close()


def main():
    args = parser.parse_args()
    run(args)


if __name__ == '__main__':
    main()

```

## Source file: model/utils.py

{
  "path": "model/utils.py",
  "blob_sha": "fb2795207b1b50cb424fa1471e1247402abc3b7f",
  "sha256": "52da3664a21bab33d4cd855e0ad145ef1a9ae2fd5efdbf8029621ab203e60e52",
  "lines": 32,
  "bytes": 1327,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/inisis/machine-learning-facial-deformer/blob/2956804d893f073d77cf7224d99b73a708d7d816/model/utils.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def lr_schedule(lr, lr_factor, epoch_now, lr_epochs):"
    },
    {
      "line": 12,
      "text": "def get_optimizer(params, cfg):"
    }
  ]
}

```text

import numpy as np
from torch.optim import SGD, Adadelta, Adagrad, Adam, AdamW, RMSprop


def lr_schedule(lr, lr_factor, epoch_now, lr_epochs):

    count = sum(epoch_now >= epoch for epoch in lr_epochs)

    return lr * np.power(lr_factor, count)


def get_optimizer(params, cfg):
    if cfg.SOLVER.OPTIMIZER == 'SGD':
        return SGD(params, lr=cfg.SOLVER.LR, momentum=cfg.SOLVER.MOMENTUM,
                   weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    elif cfg.SOLVER.OPTIMIZER == 'Adadelta':
        return Adadelta(params, lr=cfg.SOLVER.LR,
                        weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    elif cfg.SOLVER.OPTIMIZER == 'Adagrad':
        return Adagrad(params, lr=cfg.SOLVER.LR,
                       weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    elif cfg.SOLVER.OPTIMIZER == 'Adam':
        return Adam(params, lr=cfg.SOLVER.LR,
                    weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    elif cfg.SOLVER.OPTIMIZER == 'AdamW':
        return AdamW(params, lr=cfg.SOLVER.LR,
                     weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    elif cfg.SOLVER.OPTIMIZER == 'RMSprop':
        return RMSprop(params, lr=cfg.SOLVER.LR, momentum=cfg.SOLVER.MOMENTUM,
                       weight_decay=cfg.SOLVER.WEIGHT_DECAY)
    else:
        raise Exception('Unknown optimizer: {}'.format(cfg.SOLVER.OPTIMIZER))

```

# Facial rendering source scan: hippolytemayard/teeth-3d-mesh-segmentation

Repository: https://github.com/hippolytemayard/teeth-3d-mesh-segmentation

Pinned source revision: dc2f0c233c19e3cb7e2cae67becc3715a00d4a93

Discovery topic: teeth 3D

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"teeth": ["teeth_3d_seg/configs/inference/inference_config.yaml", "teeth_3d_seg/configs/label_decoder.json", "teeth_3d_seg/configs/label_encoder.json", "teeth_3d_seg/configs/settings.py", "teeth_3d_seg/configs/training/training_config.yaml", "teeth_3d_seg/dataset/.ipynb_checkpoints/mesh_dataset-checkpoint.py", "teeth_3d_seg/dataset/mesh_dataset.py", "teeth_3d_seg/dataset/mesh_loader.py", "teeth_3d_seg/inference/predict.py", "teeth_3d_seg/metrics/metrics.py", "teeth_3d_seg/model/meshsegnet.py", "teeth_3d_seg/training/losses.py", "teeth_3d_seg/training/train.py", "teeth_3d_seg/training/training.py", "teeth_3d_seg/training/validation.py", "teeth_3d_seg/utils/file.py"]}

## Source file: teeth_3d_seg/utils/file.py

{
  "path": "teeth_3d_seg/utils/file.py",
  "blob_sha": "95da7f29cef43ffce39ebb0b9a7108b3f7d977f2",
  "sha256": "f9164c8b8b4a2e17492870a6c9ef916a880ec7c4372b8f358ceb094024b70c61",
  "lines": 34,
  "bytes": 619,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/hippolytemayard/teeth-3d-mesh-segmentation/blob/dc2f0c233c19e3cb7e2cae67becc3715a00d4a93/teeth_3d_seg/utils/file.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def load_json(file_path: str) -> dict:"
    },
    {
      "line": 26,
      "text": "def load_yaml(path: str) -> DictConfig:"
    },
    {
      "line": 32,
      "text": "def make_exists(file_path):"
    }
  ]
}

```text

import json
import os

from omegaconf import DictConfig, OmegaConf


def load_json(file_path: str) -> dict:
    """Loading a json file

    Parameters
    ----------
    file_path : str
        path to the json file

    Returns
    -------
    dict
        json file
    """
    with open(file_path, "r") as f:
        json_file = json.load(f)

    return json_file


def load_yaml(path: str) -> DictConfig:
    """Load yaml into DictConfig omegaconf format"""
    yaml_file = OmegaConf.load(path)
    return yaml_file


def make_exists(file_path):
    if not os.path.exists(file_path):
        os.makedirs(file_path)


```

## Source file: teeth_3d_seg/training/train.py

{
  "path": "teeth_3d_seg/training/train.py",
  "blob_sha": "95df28cfe7e534146167b1ec1eedf7ed26f7b175",
  "sha256": "5fc8af3ab698b4551bb1db6b78ff061d464e50c95ba257345c28c44993bdcb36",
  "lines": 74,
  "bytes": 2510,
  "selection_score": 3,
  "keyword_line_counts": {
    "teeth": 5
  },
  "url": "https://github.com/hippolytemayard/teeth-3d-mesh-segmentation/blob/dc2f0c233c19e3cb7e2cae67becc3715a00d4a93/teeth_3d_seg/training/train.py",
  "function_signature_candidates": []
}

```text

import logging
from pathlib import Path

import torch
from torch import optim
from torch.utils.tensorboard import SummaryWriter

from teeth_3d_seg.configs.settings import CONFIG_PATH, DATA_PATH
from teeth_3d_seg.dataset.mesh_loader import get_mesh_loader
from teeth_3d_seg.model.meshsegnet import MeshSegNet
from teeth_3d_seg.training.training import train
from teeth_3d_seg.utils.file import load_yaml

if __name__ == "__main__":
    torch.multiprocessing.set_start_method("spawn")

    logging.basicConfig(level=logging.INFO)
    config = load_yaml(path=CONFIG_PATH)

    # TODO include validation to loader
    val_dir = "/data/ubuntu/data/test_set/"
    save_dir = Path(config.save_dir)
    experiment_dir = save_dir / f"experiment{config.experiment}"
    save_dir.mkdir(exist_ok=True)
    experiment_dir.mkdir(exist_ok=True)

    logging.info("Training loader initialization")

    train_loader = get_mesh_loader(
        data_folder=DATA_PATH,
        label_encoder=config.data.label_encoder,
        batch_size=config.data.loader.train.batch_size,
        shuffle=config.data.loader.train.shuffle,
        patch_size=config.data.dataset.patch_size,
        num_workers=config.data.loader.num_workers,
    )

    logging.info("Validation loader initialization")
    validation_loader = get_mesh_loader(
        data_folder=val_dir,
        label_encoder=config.data.label_encoder,
        batch_size=config.data.loader.validation.batch_size,
        shuffle=config.data.loader.validation.shuffle,
        patch_size=config.data.dataset.patch_size,
        num_workers=config.data.loader.num_workers,
    )

    logging.info(f"Training loader has size : {len(train_loader)}")
    logging.info(f"Validation loader has size : {len(validation_loader)}")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    model = MeshSegNet(
        num_classes=config.model.num_classes,
        num_channels=config.model.num_channels,
        with_dropout=config.model.dropout,
        dropout_p=config.model.dropout_proba,
    ).to(device, dtype=torch.float)

    optimizer = optim.Adam(params=model.parameters(), lr=config.train.lr, amsgrad=True)

    writer = SummaryWriter(log_dir=experiment_dir) if config.train.tensorboard else None

    logging.info("Starting training")

    train(
        model=model,
        optimizer=optimizer,
        train_loader=train_loader,
        validation_loader=validation_loader,
        config=config,
        device=device,
        writer=writer,
    )


```

## Source file: teeth_3d_seg/metrics/metrics.py

{
  "path": "teeth_3d_seg/metrics/metrics.py",
  "blob_sha": "2c041dd7a2561c76687364bba62ec79d035eb7ad",
  "sha256": "56df1d65ec7402dddd691700e56b9a4d1c9c8390fb70275b0cea1c2be81767fe",
  "lines": 170,
  "bytes": 5321,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/hippolytemayard/teeth-3d-mesh-segmentation/blob/dc2f0c233c19e3cb7e2cae67becc3715a00d4a93/teeth_3d_seg/metrics/metrics.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def weighting_DSC(y_pred, y_true, class_weights, smooth=1.0):"
    },
    {
      "line": 32,
      "text": "def weighting_SEN(y_pred, y_true, class_weights, smooth=1.0):"
    },
    {
      "line": 59,
      "text": "def weighting_PPV(y_pred, y_true, class_weights, smooth=1.0):"
    },
    {
      "line": 86,
      "text": "def DSC(y_pred, y_true, ignore_background=True, smooth=1.0):"
    },
    {
      "line": 115,
      "text": "def SEN(y_pred, y_true, ignore_background=True, smooth=1.0):"
    },
    {
      "line": 144,
      "text": "def PPV(y_pred, y_true, ignore_background=True, smooth=1.0):"
    }
  ]
}

```text

import torch
import numpy as np


def weighting_DSC(y_pred, y_true, class_weights, smooth=1.0):
    """
    inputs:
        y_pred [n_classes, x, y, z] probability
        y_true [n_classes, x, y, z] one-hot code
        class_weights
        smooth = 1.0
    """
    smooth = 1.0
    mdsc = 0.0
    n_classes = y_pred.shape[-1]

    # convert probability to one-hot code
    max_idx = torch.argmax(y_pred, dim=-1, keepdim=True)
    one_hot = torch.zeros_like(y_pred)
    one_hot.scatter_(-1, max_idx, 1)

    for c in range(0, n_classes):
        pred_flat = one_hot[:, :, c].reshape(-1)
        true_flat = y_true[:, :, c].reshape(-1)
        intersection = (pred_flat * true_flat).sum()
        w = class_weights[c] / class_weights.sum()
        mdsc += w * ((2.0 * intersection + smooth) / (pred_flat.sum() + true_flat.sum() + smooth))

    return mdsc


def weighting_SEN(y_pred, y_true, class_weights, smooth=1.0):
    """
    inputs:
        y_pred [n_classes, x, y, z] probability
        y_true [n_classes, x, y, z] one-hot code
        class_weights
        smooth = 1.0
    """
    smooth = 1.0
    msen = 0.0
    n_classes = y_pred.shape[-1]

    # convert probability to one-hot code
    max_idx = torch.argmax(y_pred, dim=-1, keepdim=True)
    one_hot = torch.zeros_like(y_pred)
    one_hot.scatter_(-1, max_idx, 1)

    for c in range(0, n_classes):
        pred_flat = one_hot[:, :, c].reshape(-1)
        true_flat = y_true[:, :, c].reshape(-1)
        intersection = (pred_flat * true_flat).sum()
        w = class_weights[c] / class_weights.sum()
        msen += w * ((intersection + smooth) / (true_flat.sum() + smooth))

    return msen


def weighting_PPV(y_pred, y_true, class_weights, smooth=1.0):
    """
    inputs:
        y_pred [n_classes, x, y, z] probability
        y_true [n_classes, x, y, z] one-hot code
        class_weights
        smooth = 1.0
    """
    smooth = 1.0
    mppv = 0.0
    n_classes = y_pred.shape[-1]

    # convert probability to one-hot code
    max_idx = torch.argmax(y_pred, dim=-1, keepdim=True)
    one_hot = torch.zeros_like(y_pred)
    one_hot.scatter_(-1, max_idx, 1)

    for c in range(0, n_classes):
        pred_flat = one_hot[:, :, c].reshape(-1)
        true_flat = y_true[:, :, c].reshape(-1)
        intersection = (pred_flat * true_flat).sum()
        w = class_weights[c] / class_weights.sum()
        mppv += w * ((intersection + smooth) / (pred_flat.sum() + smooth))

    return mppv


def DSC(y_pred, y_true, ignore_background=True, smooth=1.0):
    """
    inputs:
        y_pred [npts, n_classes] one-hot code
        y_true [npts, n_classes] one-hot code
    """
    smooth = 1.0
    n_classes = y_pred.shape[-1]
    dsc = []
    if ignore_background:
        for c in range(1, n_classes):  # pass 0 because 0 is background
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            dsc.append(((2.0 * intersection + smooth) / (pred_flat.sum() + true_flat.sum() + smooth)))

        dsc = np.asarray(dsc)
    else:
        for c in range(0, n_classes):
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            dsc.append(((2.0 * intersection + smooth) / (pred_flat.sum() + true_flat.sum() + smooth)))

        dsc = np.asarray(dsc)

    return dsc


def SEN(y_pred, y_true, ignore_background=True, smooth=1.0):
    """
    inputs:
        y_pred [npts, n_classes] one-hot code
        y_true [npts, n_classes] one-hot code
    """
    smooth = 1.0
    n_classes = y_pred.shape[-1]
    sen = []
    if ignore_background:
        for c in range(1, n_classes):  # pass 0 because 0 is background
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            sen.append(((intersection + smooth) / (true_flat.sum() + smooth)))

        sen = np.asarray(sen)
    else:
        for c in range(0, n_classes):
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            sen.append(((intersection + smooth) / (true_flat.sum() + smooth)))

        sen = np.asarray(sen)

    return sen


def PPV(y_pred, y_true, ignore_background=True, smooth=1.0):
    """
    inputs:
        y_pred [npts, n_classes] one-hot code
        y_true [npts, n_classes] one-hot code
    """
    smooth = 1.0
    n_classes = y_pred.shape[-1]
    ppv = []
    if ignore_background:
        for c in range(1, n_classes):  # pass 0 because 0 is background
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            ppv.append(((intersection + smooth) / (pred_flat.sum() + smooth)))

        ppv = np.asarray(ppv)
    else:
        for c in range(0, n_classes):
            pred_flat = y_pred[:, c].reshape(-1)
            true_flat = y_true[:, c].reshape(-1)
            intersection = (pred_flat * true_flat).sum()
            ppv.append(((intersection + smooth) / (pred_flat.sum() + smooth)))

        ppv = np.asarray(ppv)

    return ppv


```

## Source file: teeth_3d_seg/training/losses.py

{
  "path": "teeth_3d_seg/training/losses.py",
  "blob_sha": "b3c7e856ba2ce09cc2e5ac05799caa61a722b7db",
  "sha256": "746498b3a01417b9a3f6649432cc2cef844e8e40fdc7c22ad4f9695268c49956",
  "lines": 22,
  "bytes": 683,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/hippolytemayard/teeth-3d-mesh-segmentation/blob/dc2f0c233c19e3cb7e2cae67becc3715a00d4a93/teeth_3d_seg/training/losses.py",
  "function_signature_candidates": [
    {
      "line": 1,
      "text": "def Generalized_Dice_Loss(y_pred, y_true, class_weights, smooth=1.0):"
    }
  ]
}

```text

def Generalized_Dice_Loss(y_pred, y_true, class_weights, smooth=1.0):
    """
    inputs:
        y_pred [n_classes, x, y, z] probability
        y_true [n_classes, x, y, z] one-hot code
        class_weights
        smooth = 1.0
    """
    smooth = 1.0
    loss = 0.0
    n_classes = y_pred.shape[-1]

    for c in range(0, n_classes):
        pred_flat = y_pred[:, :, c].reshape(-1)
        true_flat = y_true[:, :, c].reshape(-1)
        intersection = (pred_flat * true_flat).sum()

        # with weight
        w = class_weights[c] / class_weights.sum()
        loss += w * (1 - ((2.0 * intersection + smooth) / (pred_flat.sum() + true_flat.sum() + smooth)))

    return loss


```

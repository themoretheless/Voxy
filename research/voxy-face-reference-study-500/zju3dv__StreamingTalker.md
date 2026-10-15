# Facial rendering source scan: zju3dv/StreamingTalker

Repository: https://github.com/zju3dv/StreamingTalker

Pinned source revision: 1f6452752dbd6c11f4bb1e94982e000e68d8f76d

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: eval.py

{
  "path": "eval.py",
  "blob_sha": "ef6c8bad43ca99652237f6c0a9c95207044b2eb3",
  "sha256": "f91b2401170038494a730d2ebc3de983413ff20306b02bcf48fa6a691cac5698",
  "lines": 134,
  "bytes": 4160,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/zju3dv/StreamingTalker/blob/1f6452752dbd6c11f4bb1e94982e000e68d8f76d/eval.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def print_table(title, metrics):"
    },
    {
      "line": 34,
      "text": "def get_metric_statistics(values, replication_times):"
    },
    {
      "line": 40,
      "text": "def main():"
    }
  ]
}

```text

import os
import pickle
import torch
import json
import os
import pytorch_lightning as pl
import numpy as np
from pathlib import Path
from omegaconf import OmegaConf
from rich import get_console
from rich.table import Table
from tqdm import tqdm

from utils.logger_utils import create_logger
from utils.progress import ProgressLogger
from configs.config import parse_args
from utils.demo_utils import animate

from algorithms.models import get_model
from datasets import get_datasets

def print_table(title, metrics):
    table = Table(title=title)

    table.add_column("Metrics", style="cyan", no_wrap=True)
    table.add_column("Value", style="magenta")

    for key, value in metrics.items():
        table.add_row(key, str(value))

    console = get_console()
    console.print(table, justify="center")

def get_metric_statistics(values, replication_times):
    mean = np.mean(values, axis=0)
    std = np.std(values, axis=0)
    conf_interval = 1.96 * std / np.sqrt(replication_times)
    return mean, conf_interval

def main():
    # parse options
    cfg = parse_args(phase="test") #
    cfg.FOLDER = cfg.TEST.FOLDER
    cfg.Name = "demo--" + cfg.NAME
    # set up the logger
    dataset = 'vocaset' # TODO
    logger = create_logger(cfg, phase="test")
    output_dir = Path(
        os.path.join(cfg.FOLDER, str(cfg.MODEL.TYPE), str(cfg.NAME),
                     "samples_" + cfg.TIME))
    output_dir.mkdir(parents=True, exist_ok=True)
    # logger.info(OmegaConf.to_yaml(cfg))

    # set seed
    pl.seed_everything(cfg.SEED_VALUE)
    
    # create dataset
    datasets = get_datasets(cfg, phase="test")
    logger.info("datasets module {} initialized".format("".join(
        cfg.DATASET.NAME)))

    # create model
    model = get_model(cfg)
    logger.info("model {} loaded".format(cfg.MODEL.TYPE))

    # monitor
    metric_monitor = {
        'none': None # TODO
    }

    # callbacks
    callbacks = [
        pl.callbacks.RichProgressBar(), # type: ignore
        ProgressLogger(metric_monitor=metric_monitor),
    ]

    # trainer
    trainer = pl.Trainer(
        benchmark=False,
        max_epochs=cfg.TRAIN.END_EPOCH,
        accelerator=cfg.ACCELERATOR,
        devices=cfg.DEVICE,
        default_root_dir=cfg.FOLDER_EXP,
        reload_dataloaders_every_n_epochs=1,
        log_every_n_steps=cfg.LOGGER.LOG_EPOCH,
        deterministic=False,
        detect_anomaly=False,
        enable_progress_bar=True,
        logger=None,
        callbacks=callbacks,
    )

    # load model weights
    logger.info("Loading checkpoints from {}".format(cfg.TEST.CHECKPOINTS))
    state_dict = torch.load(cfg.TEST.CHECKPOINTS, map_location="cpu")["state_dict"]
    
    # model.load_state_dict(state_dict, strict=True)
    model.load_state_dict(state_dict, strict=False)   

    all_metrics = {}
    replication_times = cfg.TEST.REPLICATION_TIMES
    seeds = np.arange(0, replication_times * 10, replication_times)
    # calculate metrics
    for i, seed in zip(tqdm(range(replication_times), desc="Evaluation among replications"), seeds):
        logger.info(f"Evaluation Replication {i}")
        metrics = trainer.test(model, datamodule=datasets)[0]

        # set seed
        pl.seed_everything(seed)

        # save metrics
        for key, item in metrics.items():
            if key not in all_metrics:
                all_metrics[key] = [item]
            else:
                all_metrics[key] += [item]

    all_metrics_new = {}
    for key, item in all_metrics.items():
        mean, conf_interval = get_metric_statistics(np.array(item),
                                                    replication_times)
        
        all_metrics_new[key + "/mean"] = mean
        all_metrics_new[key + "/conf_interval"] = conf_interval
    print_table(f"Mean Metrics", all_metrics_new)
    all_metrics_new.update(all_metrics)
    # save metrics to file
    metric_file = output_dir.parent / f"metrics_{cfg.TIME}.json"
    with open(metric_file, "w", encoding="utf-8") as f:
        json.dump(all_metrics_new, f, indent=4)
    logger.info(f"Testing done, the metrics are saved to {str(metric_file)}")

if __name__ == "__main__":
    main()

```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "fbc636d3f49f26d48c432953ea48c04b8a60cfad",
  "sha256": "7ab09bc0a97b8b0bb316981216de978847c53b704d07c76e98d84e7dcbfd0799",
  "lines": 174,
  "bytes": 5949,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1,
    "lip": 4
  },
  "url": "https://github.com/zju3dv/StreamingTalker/blob/1f6452752dbd6c11f4bb1e94982e000e68d8f76d/train.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def train():"
    }
  ]
}

```text

import os
import torch
import pytorch_lightning as pl

from omegaconf import OmegaConf
from collections import OrderedDict
from pytorch_lightning import loggers as pl_loggers
from pytorch_lightning.callbacks import ModelCheckpoint

from datasets import get_datasets
from utils.logger_utils import create_logger
from utils.progress import ProgressLogger
from configs.config import parse_args
from algorithms.models import get_model

def train():
    cfg = parse_args()

    # create logger
    logger = create_logger(cfg, phase="train")

    # resume
    if cfg.TRAIN.RESUME:
        resume = cfg.TRAIN.RESUME
        backcfg = cfg.TRAIN.copy()
        losscfg = cfg.LOSS.copy()
        if os.path.exists(resume):
            file_list = sorted(os.listdir(resume), reverse=True)
            for item in file_list:
                if item.endswith(".yaml"):
                    cfg = OmegaConf.load(os.path.join(resume, item))
                    cfg.TRAIN = backcfg
                    cfg.LOSS = losscfg
                    break
            checkpoints = sorted(os.listdir(os.path.join(
                resume, "checkpoints")),
                                 key=lambda x: int(x[6:-5]),
                                 reverse=True)
            for checkpoint in checkpoints:
                if "epoch=" in checkpoint:
                    cfg.TRAIN.PRETRAINED = os.path.join(
                        resume, "checkpoints", checkpoint)
                    break
            if os.path.exists(os.path.join(resume, "wandb")):
                wandb_list = sorted(os.listdir(os.path.join(resume, "wandb")),
                                    reverse=True)
                for item in wandb_list:
                    if "run-" in item:
                        cfg.LOGGER.WANDB.RESUME_ID = item.split("-")[-1]
            logger.info("Resume from {}".format(resume))
        else:
            raise ValueError("Resume path is not right.")
        
    # set seed
    pl.seed_everything(cfg.SEED_VALUE)

    # gpu setting
    if cfg.ACCELERATOR == "gpu":
        os.environ["PYTHONWARNINGS"] = "ignore"
        os.environ["TOKENIZERS_PARALLELISM"] = "false"

    # tensorboard logger and wandb logger
    loggers = []
    if cfg.LOGGER.WANDB.PROJECT:
        wandb_logger = pl_loggers.WandbLogger(
            project=cfg.LOGGER.WANDB.PROJECT,
            offline=cfg.LOGGER.WANDB.OFFLINE,
            id=cfg.LOGGER.WANDB.RESUME_ID,
            save_dir=cfg.FOLDER_EXP,
            version="",
            name=cfg.NAME,
            anonymous=False,
            log_model=False,
        )
        loggers.append(wandb_logger)
    if cfg.LOGGER.TENSORBOARD:
        tb_logger = pl_loggers.TensorBoardLogger(save_dir=cfg.FOLDER_EXP,
                                                 sub_dir="tensorboard",
                                                 version="",
                                                 name="")
        loggers.append(tb_logger)
    # logger.info(OmegaConf.to_yaml(cfg))

    # create datamodule
    datamodule = get_datasets(cfg=cfg)
    logger.info("{} Datamodule initialized".format("".join(
        cfg.DATASET.NAME)))

    # create model
    model = get_model(cfg=cfg)
    logger.info("{} Model initialized".format(cfg.MODEL.TYPE))

    metric_monitor = {
        "Diff_loss_Train": "diff_loss/train",
        "Diff_loss_Val":"diff_loss/val",
        "Train_vertice_recon": "vertice/enc/train",
        "Train_vertice_reconv": "vertice/encv/train",
        "Train_lip_recon": "lip/enc/train",
        "Train_lip_reconv": "lip/encv/train",
        "Val_vertice_recon": "vertice/enc/val",
        "Val_vertice_reconv": "vertice/encv/val",
        "Val_lip_recon": "lip/enc/val",
        "Val_lip_reconv": "lip/encv/val",
    }

    # callbacks
    callbacks = [
        pl.callbacks.RichProgressBar(),
        ProgressLogger(metric_monitor=metric_monitor),
        ModelCheckpoint(
            dirpath=os.path.join(cfg.FOLDER_EXP, "checkpoints"),
            filename="{epoch:02d}",
            monitor="step",
            mode="max",
            every_n_epochs=cfg.LOGGER.SAVE_CKPT_EPOCH,
            save_top_k=-1,
            save_last=False,
            save_on_train_epoch_end=True,
        ),
    ]
    logger.info("Callbacks initialized")

    if len(cfg.DEVICE) > 1:
        ddp_strategy = "ddp"
        logger.info("Trained with DDP")
    else:
        ddp_strategy = None

    # trainer
    trainer = pl.Trainer(
        benchmark=False,
        max_epochs=cfg.TRAIN.END_EPOCH,
        accelerator=cfg.ACCELERATOR,
        devices=cfg.DEVICE,
        strategy=ddp_strategy,
        default_root_dir=cfg.FOLDER_EXP,
        log_every_n_steps=cfg.LOGGER.LOG_EPOCH,
        deterministic=False,
        detect_anomaly=False,
        enable_progress_bar=True,
        logger=loggers,
        callbacks=callbacks,
        check_val_every_n_epoch=cfg.LOGGER.VAL_EPOCH,
        num_sanity_val_steps=0,
    )
    logger.info("Trainer initialized")

    if cfg.TRAIN.PRETRAINED:
        logger.info("resume from {}".format(
            cfg.TRAIN.PRETRAINED))
        logger.info("Attention! VAE will be recovered")
        state_dict = torch.load(cfg.TRAIN.PRETRAINED,
                                map_location="cpu")["state_dict"]

        new_state_dict = OrderedDict()
        for k, v in state_dict.items():
            new_state_dict[k] = v
        model.load_state_dict(new_state_dict, strict=False)

    # fitting
    if cfg.TRAIN.RESUME:
        trainer.fit(model,
                    datamodule = datamodule,
                    ckpt_path = cfg.TRAIN.PRETRAINED)
    else:
        trainer.fit(model, datamodule = datamodule)

    checkpoint_folder = trainer.checkpoint_callback.dirpath
    logger.info(f"The checkpoints are stored in {checkpoint_folder}")
    logger.info(f"The outputs of this experiment are stored in {cfg.FOLDER_EXP}")
    logger.info("Training ends!")

if __name__=="__main__":
    train()

```

## Source file: eval_vae.py

{
  "path": "eval_vae.py",
  "blob_sha": "c5d139c297612bca8ed537e31cba21f7fe96b597",
  "sha256": "59337919cd15301b8582396827f06162e264281060d3567c91657b2dcb76ad4d",
  "lines": 73,
  "bytes": 2654,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/zju3dv/StreamingTalker/blob/1f6452752dbd6c11f4bb1e94982e000e68d8f76d/eval_vae.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def main():"
    }
  ]
}

```text

import os
import pickle
import torch
import time
import numpy as np
import torch.nn.functional as F

from configs.config import parse_args
from algorithms.models.vae_trainer import VAETrainer
from utils.logger_utils import create_logger
from utils.demo_utils import animate


def main():
    cfg = parse_args(phase="demo")
    cfg.FOLDER = cfg.TEST.FOLDER

    if cfg.ACCELERATOR == "gpu":
        device = torch.device("cuda")
    else:
        device = torch.device("cpu")

    # create logger
    dataset = cfg.DATASET.NAME
    logger = create_logger(cfg, phase="demo")

    # create model & load ckpt
    model = VAETrainer(cfg)
    logger.info("Loading checkpoints from {}".format(cfg.DEMO.CHECKPOINTS))
    state_dict = torch.load(cfg.DEMO.CHECKPOINTS, map_location="cpu")["state_dict"]
    
    model.load_state_dict(state_dict, strict=False)
    model.to(device)
    model.eval()

    # load the template
    logger.info("Loading template mesh from {}".format(cfg.DEMO.TEMPLATE))
    template_file = cfg.DEMO.TEMPLATE
    with open(template_file, 'rb') as fin:
        template = pickle.load(fin,encoding='latin1')
        subject_id = cfg.DEMO.ID
        assert subject_id in template, f'{subject_id} is not a subject included'
        template = torch.Tensor(template[subject_id].reshape(-1))

    # load vertices
    vertice_path = cfg.DEMO.VERTICE
    if cfg.DATASET.NAME == 'biwi':
        vertices = torch.FloatTensor(np.load(vertice_path,allow_pickle=True)).to(device)
    elif cfg.DATASET.NAME == 'vocaset':
        vertices = torch.FloatTensor(np.load(vertice_path,allow_pickle=True)[::2,:]).to(device)

    logger.info("Start making predictions")
    
    with torch.no_grad():
        vertices_pred, quant_loss, info = model.autoencoder(vertices[None,...], template[None,...].to(device))
        vertices_pred = vertices_pred.squeeze().cpu().numpy()
        vertices = vertices.cpu().numpy()
            
    output_dir = os.path.join(cfg.FOLDER, str(cfg.MODEL.TYPE), str(cfg.NAME), "samples_" + cfg.TIME)
    pred_file_name = os.path.join(output_dir, 'vertice_pred' + '.mp4')
    gt_file_name = os.path.join(output_dir, 'vertice_gt' + '.mp4')
    output_file = os.path.join(output_dir,'output.mp4')

    wav_path = None
    
    animate(vertices_pred, wav_path, pred_file_name, cfg.DEMO.PLY, fps=30, use_tqdm=True, multi_process=True)
    animate(vertices, wav_path, gt_file_name, cfg.DEMO.PLY, fps=30, use_tqdm=True, multi_process=True)

    cmd = " ".join(['ffmpeg', '-i', gt_file_name , '-i', pred_file_name , '-filter_complex "[0:v][1:v]hstack" -c:a copy', output_file, ])
    os.system(cmd)

if __name__ == "__main__":
    main()

```

## Source file: train_vae.py

{
  "path": "train_vae.py",
  "blob_sha": "d8e19ced9c79b82f68c26319ba8d5c589e15f4c0",
  "sha256": "cc67f2a8504b2291d41f90dfd762fcafce95c8f0f59e7fcffd45077f8165cbd5",
  "lines": 153,
  "bytes": 5085,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/zju3dv/StreamingTalker/blob/1f6452752dbd6c11f4bb1e94982e000e68d8f76d/train_vae.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def train():"
    }
  ]
}

```text

import os
import torch
import pytorch_lightning as pl

from omegaconf import OmegaConf
from collections import OrderedDict
from pytorch_lightning import loggers as pl_loggers
from pytorch_lightning.callbacks import ModelCheckpoint

from datasets import get_datasets
from utils.logger_utils import create_logger
from utils.progress import ProgressLogger
from configs.config import parse_args
from algorithms.models import get_model

def train():
    cfg = parse_args()

    # create logger
    logger = create_logger(cfg, phase="train")

    # resume
    if cfg.TRAIN.RESUME:
        resume = cfg.TRAIN.RESUME
        backcfg = cfg.TRAIN.copy()
        losscfg = cfg.LOSS.copy()
        if os.path.exists(resume):
            file_list = sorted(os.listdir(resume), reverse=True)
            for item in file_list:
                if item.endswith(".yaml"):
                    cfg = OmegaConf.load(os.path.join(resume, item))
                    cfg.TRAIN = backcfg
                    cfg.LOSS = losscfg
                    break
            checkpoints = sorted(os.listdir(os.path.join(
                resume, "checkpoints")),
                                 key=lambda x: int(x[6:-5]),
                                 reverse=True)
            for checkpoint in checkpoints:
                if "epoch=" in checkpoint:
                    cfg.TRAIN.PRETRAINED = os.path.join(
                        resume, "checkpoints", checkpoint)
                    break
            if os.path.exists(os.path.join(resume, "wandb")):
                wandb_list = sorted(os.listdir(os.path.join(resume, "wandb")),
                                    reverse=True)
                for item in wandb_list:
                    if "run-" in item:
                        cfg.LOGGER.WANDB.RESUME_ID = item.split("-")[-1]
            logger.info("Resume from {}".format(resume))
        else:
            raise ValueError("Resume path is not right.")
        
    # set seed
    pl.seed_everything(cfg.SEED_VALUE)

    # gpu setting
    if cfg.ACCELERATOR == "gpu":
        os.environ["PYTHONWARNINGS"] = "ignore"
        os.environ["TOKENIZERS_PARALLELISM"] = "false"

    # tensorboard logger
    loggers = []
    tb_logger = pl_loggers.TensorBoardLogger(save_dir=cfg.FOLDER_EXP,
                                                sub_dir="tensorboard",
                                                version="",
                                                name="")
    loggers.append(tb_logger)

    # create datamodule
    datamodule = get_datasets(cfg=cfg)
    logger.info("{} Datamodule initialized".format("".join(
        cfg.DATASET.NAME)))

    # create model
    model = get_model(cfg=cfg)
    logger.info("{} Model initialized".format(cfg.MODEL.TYPE))

    metric_monitor = {
        "Train_recon_loss": "recon/train",
        "Train_embed_loss": "embed/train",
        "Val_recon_loss": "recon/val",
        "Val_embed_loss": "embed/val",
    }

    # callbacks
    callbacks = [
        pl.callbacks.RichProgressBar(),
        ProgressLogger(metric_monitor=metric_monitor),
        ModelCheckpoint(
            dirpath=os.path.join(cfg.FOLDER_EXP, "checkpoints"),
            filename="{epoch:02d}",
            monitor="step",
            mode="max",
            every_n_epochs=cfg.LOGGER.SAVE_CKPT_EPOCH,
            save_top_k=-1,
            save_last=False,
            save_on_train_epoch_end=True,
        ),
    ]
    logger.info("Callbacks initialized")

    if len(cfg.DEVICE) > 1:
        ddp_strategy = "ddp"
        logger.info("Trained with DDP")
    else:
        ddp_strategy = None

    # trainer
    trainer = pl.Trainer(
        benchmark=False,
        max_epochs=cfg.TRAIN.END_EPOCH,
        accelerator=cfg.ACCELERATOR,
        devices=cfg.DEVICE,
        strategy=ddp_strategy,
        default_root_dir=cfg.FOLDER_EXP,
        log_every_n_steps=cfg.LOGGER.LOG_EPOCH,
        deterministic=False,
        detect_anomaly=False,
        enable_progress_bar=True,
        logger=loggers,
        callbacks=callbacks,
        check_val_every_n_epoch=cfg.LOGGER.VAL_EPOCH,
        num_sanity_val_steps=0,
    )
    logger.info("Trainer initialized")

    if cfg.TRAIN.PRETRAINED:
        logger.info("resume from {}".format(
            cfg.TRAIN.PRETRAINED))
        state_dict = torch.load(cfg.TRAIN.PRETRAINED,
                                map_location="cpu")["state_dict"]

        new_state_dict = OrderedDict()
        for k, v in state_dict.items():
            new_state_dict[k] = v
        model.load_state_dict(new_state_dict, strict=False)

    # fitting
    if cfg.TRAIN.RESUME:
        trainer.fit(model,
                    datamodule = datamodule,
                    ckpt_path = cfg.TRAIN.PRETRAINED)
    else:
        trainer.fit(model, datamodule = datamodule)

    checkpoint_folder = trainer.checkpoint_callback.dirpath
    logger.info(f"The checkpoints are stored in {checkpoint_folder}")
    logger.info(f"The outputs of this experiment are stored in {cfg.FOLDER_EXP}")
    logger.info("Training ends!")

if __name__=="__main__":
    train()

```

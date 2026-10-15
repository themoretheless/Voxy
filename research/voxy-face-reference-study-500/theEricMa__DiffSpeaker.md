# Facial rendering source scan: theEricMa/DiffSpeaker

Repository: https://github.com/theEricMa/DiffSpeaker

Pinned source revision: 0f2ae2d4e88fe78bff8bb7a1fdbc628e8733f120

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["datasets/vocaset/FLAME_masks.pkl", "datasets/vocaset/templates/FLAME_sample.ply"]}

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "8db5722dda3a382427b00d65feac366429c2ce7f",
  "sha256": "322ec5ad23eb3cf47fed6a91fe090a545ad0ebafdecacb8be85fb6fc9e64efcd",
  "lines": 232,
  "bytes": 8560,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "rig": 1,
    "lip": 4
  },
  "url": "https://github.com/theEricMa/DiffSpeaker/blob/0f2ae2d4e88fe78bff8bb7a1fdbc628e8733f120/train.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def main():"
    }
  ]
}

```text

import os
from pprint import pformat

import pytorch_lightning as pl
import torch
from omegaconf import OmegaConf
from pytorch_lightning import loggers as pl_loggers
from pytorch_lightning.callbacks import ModelCheckpoint
from pytorch_lightning.strategies.ddp import DDPStrategy

from alm.callback import ProgressLogger
from alm.config import parse_args
from alm.data.get_data import get_datasets
from alm.models.get_model import get_model
from alm.utils.logger import create_logger


def main():
    # parse options
    cfg = parse_args()  # parse config file

    # create logger
    logger = create_logger(cfg, phase="train")

    # resume
    if cfg.TRAIN.RESUME:
        resume = cfg.TRAIN.RESUME
        backcfg = cfg.TRAIN.copy()
        if os.path.exists(resume):
            file_list = sorted(os.listdir(resume), reverse=True)
            for item in file_list:
                if item.endswith(".yaml"):
                    cfg = OmegaConf.load(os.path.join(resume, item))
                    cfg.TRAIN = backcfg
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
        # os.environ['CUDA_VISIBLE_DEVICES'] = ",".join(str(x) for x in cfg.DEVICE)

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
    logger.info(OmegaConf.to_yaml(cfg))

    # create dataset
    datasets = get_datasets(cfg, logger=logger)
    logger.info("datasets module {} initialized".format("".join(
        cfg.TRAIN.DATASETS)))

    # tmp = next(iter(datasets[0].train_dataset)) #TODO: remove this line
    # tmp = next(iter(datasets[0].train_dataloader()))

    # create model
    model = get_model(cfg, datasets[0])
    logger.info("model {} loaded".format(cfg.model.model_type))

    # optimizer
    # metric_monitor = {
    #     # baseline experiment for training
    #     "Train_vertice_recon": "vertice/enc/train",
    #     "Train_vertice_reconv": "vertice/encv/train",
    #     # diffusion-denoising experiment for training
    #     "Train_vertice_deno": "vertice/den/train",
    #     "Train_vertice_denov": "vertice/denv/train",
    #     "Train_latent": "latent/consist/train",
    #     # baseline experiment for validation
    #     "Val_vertice_recon": "vertice/enc/val",
    #     "Val_vertice_reconv": "vertice/encv/val",
    #     # diffusion-denoising experiment for validation
    #     "Val_vertice_deno": "vertice/den/val",
    #     "Val_vertice_denov": "vertice/denv/val",
    #     "Val_latent": "latent/consist/val"
    # }

    metric_monitor = {
        "Train_vertice_recon": "vertice/enc/train",
        "Train_vertice_reconv": "vertice/encv/train",
        "Train_lip_recon": "lip/enc/train",
        "Train_lip_reconv": "lip/encv/train",
        "Val_vertice_recon": "vertice/enc/val",
        "Val_vertice_reconv": "vertice/encv/val",
        "Val_lip_recon": "lip/enc/val",
        "Val_lip_reconv": "lip/encv/val",
    }

    # metric_monitor = {
    #     # baseline experiment for training
    #     "Train_vertice_recon": "vertice/enc/train",
    #     "Train_vertice_reconv": "vertice/encv/train",
    #     "Train_exp_recon": "exp/enc/train",
    #     "Train_exp_reconv": "exp/encv/train",
    #     "Train_pose_recon": "pose/enc/train",
    #     "Train_pose_reconv": "pose/encv/train",
    #     "Train_eye_recon": "eye/enc/train",
    #     "Train_eye_reconv": "eye/encv/train",
    #     # "Train_lmk": "lmk/enc/train",
    #     # "Train_lmkv": "lmk/encv/train",
    #     # "Train_sc_lmk": "lmk/sc/train",
    #     # "Train_sc_vertice": "vertice/sc/train",
    #     # baseline experiment for validation
    #     "Val_vertice_recon": "vertice/enc/val",
    #     "Val_vertice_reconv": "vertice/encv/val",
    #     "Val_exp_recon": "exp/enc/val",
    #     "Val_exp_reconv": "exp/encv/val",
    #     "Val_pose_recon": "pose/enc/val",
    #     "Val_pose_reconv": "pose/encv/val",
    #     "Val_eye_recon": "eye/enc/val",
    #     "Val_eye_reconv": "eye/encv/val",
    #     # "Val_lmk": "lmk/enc/val",
    #     # "Val_lmkv": "lmk/encv/val",
    #     # "Val_sc_lmk": "lmk/sc/val",
    #     # "Val_sc_vertice": "vertice/sc/val",
    # }

    # callbacks
    callbacks = [
        pl.callbacks.RichProgressBar(),
        ProgressLogger(metric_monitor=metric_monitor),
        # ModelCheckpoint(dirpath=os.path.join(cfg.FOLDER_EXP,'checkpoints'),filename='latest-{epoch}',every_n_epochs=1,save_top_k=1,save_last=True,save_on_train_epoch_end=True),
        ModelCheckpoint(
            dirpath=os.path.join(cfg.FOLDER_EXP, "checkpoints"),
            filename="{epoch:02d}",
            monitor="step",
            mode="max",
            every_n_epochs=cfg.LOGGER.SACE_CHECKPOINT_EPOCH,
            save_top_k=-1,
            save_last=False,
            save_on_train_epoch_end=True,
        ),
    ]
    logger.info("Callbacks initialized")

    if len(cfg.DEVICE) > 1:
        ddp_strategy = "ddp" #DDPStrategy(find_unused_parameters=False)
    else:
        ddp_strategy = None

    # trainer
    trainer = pl.Trainer(
        benchmark=False,
        max_epochs=cfg.TRAIN.END_EPOCH,
        accelerator=cfg.ACCELERATOR,
        devices=cfg.DEVICE,
        strategy=ddp_strategy,
        # move_metrics_to_cpu=True,
        default_root_dir=cfg.FOLDER_EXP,
        log_every_n_steps=cfg.LOGGER.VAL_EVERY_STEPS,
        deterministic=False,
        detect_anomaly=False,
        enable_progress_bar=True,
        logger=loggers,
        callbacks=callbacks,
        check_val_every_n_epoch=cfg.LOGGER.VAL_EVERY_STEPS,
        num_sanity_val_steps=0, #0
    )
    logger.info("Trainer initialized")

    if cfg.TRAIN.PRETRAINED:
        logger.info("Loading pretrain mode from {}".format(
            cfg.TRAIN.PRETRAINED))
        logger.info("Attention! VAE will be recovered")
        state_dict = torch.load(cfg.TRAIN.PRETRAINED,
                                map_location="cpu")["state_dict"]
        # remove mismatched and unused params
        from collections import OrderedDict

        new_state_dict = OrderedDict()
        for k, v in state_dict.items():
            if k not in ["denoiser.sequence_pos_encoding.pe"]:
                new_state_dict[k] = v
        model.load_state_dict(new_state_dict, strict=False)

    # fitting
    if cfg.TRAIN.RESUME:
        trainer.fit(model,
                    datamodule=datasets[0],
                    ckpt_path=cfg.TRAIN.PRETRAINED)
    else:
        trainer.fit(model, datamodule=datasets[0])

    # checkpoint
    checkpoint_folder = trainer.checkpoint_callback.dirpath
    logger.info(f"The checkpoints are stored in {checkpoint_folder}")
    logger.info(
        f"The outputs of this experiment are stored in {cfg.FOLDER_EXP}")

    # end
    logger.info("Training ends!")


if __name__ == "__main__":
    main()


```

## Source file: eval_biwi.py

{
  "path": "eval_biwi.py",
  "blob_sha": "243ae0db5c570dd5d80587bd45b1c6842de64aa7",
  "sha256": "e7719183a37e8169de8a3199f7a6e161ef8559f5d1e008a83fe42d9e465ab72b",
  "lines": 169,
  "bytes": 5370,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 3,
    "face": 1
  },
  "url": "https://github.com/theEricMa/DiffSpeaker/blob/0f2ae2d4e88fe78bff8bb7a1fdbc628e8733f120/eval_biwi.py",
  "function_signature_candidates": [
    {
      "line": 54,
      "text": "def print_table(title, metrics):"
    },
    {
      "line": 66,
      "text": "def get_metric_statistics(values, replication_times):"
    },
    {
      "line": 72,
      "text": "def main():"
    }
  ]
}

```text

#############################################################################################################
# this file is for evaluation of the vocaset dataset
#############################################################################################################
import os
import pickle
import torch
import json
import os
from pathlib import Path
from omegaconf import OmegaConf

import numpy as np
from alm.config import parse_args
from alm.models.get_model import get_model
from alm.utils.logger import create_logger
from alm.utils.demo_utils import animate

from rich import get_console
from rich.table import Table

import pytorch_lightning as pl
from alm.data.get_data import get_datasets
from alm.callback import ProgressLogger

from tqdm import tqdm

# create a list namly vertices_pred_list, if it does not exist
try:
    eval('vertices_pred_list')
    eval('vertices_gt_list')
    eval('motion_std_difference')
except NameError: # if it does not exist, create it
    vertices_gt_all = []
    vertices_pred_all = []
    motion_std_difference = []

# load the mouth and upper face region
try:
    eval('mouth_map')
except NameError:
    with open(os.path.join('datasets/biwi/regions', "lve.txt")) as f:
        maps = f.read().split(", ")
        mouth_map = [int(i) for i in maps]
                
try:
    eval('upper_map')
except NameError:
    with open(os.path.join('datasets/biwi/regions', "fdd.txt")) as f:
        maps = f.read().split(", ")
        upper_map = [int(i) for i in maps]
                            


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
        os.path.join(cfg.FOLDER, str(cfg.model.model_type), str(cfg.NAME),
                     "samples_" + cfg.TIME))
    output_dir.mkdir(parents=True, exist_ok=True)
    logger.info(OmegaConf.to_yaml(cfg))

    # set seed
    pl.seed_everything(cfg.SEED_VALUE)
    
    # create dataset
    datasets = get_datasets(cfg, logger=logger, phase="test")[0]
    logger.info("datasets module {} initialized".format("".join(
        cfg.TRAIN.DATASETS)))

    # create model
    model = get_model(cfg, datasets)
    logger.info("model {} loaded".format(cfg.model.model_type))

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
        devices=list(range(len(cfg.DEVICE))),
        default_root_dir=cfg.FOLDER_EXP,
        reload_dataloaders_every_n_epochs=1,
        log_every_n_steps=cfg.LOGGER.LOG_EVERY_STEPS,
        deterministic=False,
        detect_anomaly=False,
        enable_progress_bar=True,
        logger=None,
        callbacks=callbacks,
    )


    # load model weights
    logger.info("Loading checkpoints from {}".format(cfg.TEST.CHECKPOINTS))
    state_dict = torch.load(cfg.TEST.CHECKPOINTS, map_location="cpu")["state_dict"]
    state_dict.pop("denoiser.PPE.pe") # this is not needed, since the sequence length can be any flexiable
    model.load_state_dict(state_dict, strict=False)    

    all_metrics = {}
    replication_times = cfg.TEST.REPLICATION_TIMES
    seeds = np.arange(0, replication_times * 10, replication_times)
    # calculate metrics
    for i, seed in zip(tqdm(range(replication_times), desc="Evaluation among replications"), seeds):
        logger.info(f"Evaluation Replication {i}")
        metrics = trainer.test(model, datamodule=datasets)[0]#[0] # TODO

        # set seed
        pl.seed_everything(seed)

        # save metrics
        for key, item in metrics.items():
            if key not in all_metrics:
                all_metrics[key] = [item]
            else:
                all_metrics[key] += [item]

        # change the seed


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

## Source file: alm/config.py

{
  "path": "alm/config.py",
  "blob_sha": "cc8f523a80a183fee0bf8862a33bd0b249d1eee2",
  "sha256": "f4571ecbd8d4a46cc386055e57405e9403eba456cce0fbe891bbe7eb575844e0",
  "lines": 204,
  "bytes": 6915,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "gum": 24
  },
  "url": "https://github.com/theEricMa/DiffSpeaker/blob/0f2ae2d4e88fe78bff8bb7a1fdbc628e8733f120/alm/config.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def get_module_config(cfg_model, path=\"modules\"):"
    },
    {
      "line": 16,
      "text": "def get_obj_from_str(string, reload=False):"
    },
    {
      "line": 24,
      "text": "def instantiate_from_config(config):"
    },
    {
      "line": 34,
      "text": "def parse_args(phase=\"train\"):"
    }
  ]
}

```text

import importlib
from argparse import ArgumentParser
from omegaconf import OmegaConf
import os


def get_module_config(cfg_model, path="modules"):
    files = os.listdir(f'./configs/{path}/')
    for file in files:
        if file.endswith('.yaml'):
            with open(f'./configs/{path}/' + file, 'r') as f:
                cfg_model.merge_with(OmegaConf.load(f))
    return cfg_model


def get_obj_from_str(string, reload=False):
    module, cls = string.rsplit(".", 1)
    if reload:
        module_imp = importlib.import_module(module)
        importlib.reload(module_imp)
    return getattr(importlib.import_module(module, package=None), cls)


def instantiate_from_config(config):
    if not "target" in config:
        if config == '__is_first_stage__':
            return None
        elif config == "__is_unconditional__":
            return None
        raise KeyError("Expected key `target` to instantiate.")
    return get_obj_from_str(config["target"])(**config.get("params", dict()))


def parse_args(phase="train"):
    parser = ArgumentParser()

    group = parser.add_argument_group("Training options")
    if phase in ["train", "test", "demo"]:
        group.add_argument(
            "--cfg",
            type=str,
            required=False,
            default="./configs/config.yaml",
            help="config file",
        )
        group.add_argument(
            "--cfg_assets",
            type=str,
            required=False,
            default="./configs/assets.yaml",
            help="config file for asset paths",
        )
        group.add_argument(
            "--frame_rate",
            type=float,
            default=30,
            help="the frame rate for the input/output motion",
        )
        group.add_argument(
            "--resume",
            type=str,
            required=False,
            help="resume from a checkpoint",
        )
        group.add_argument("--batch_size",
                           type=int,
                           required=False,
                           help="training batch size")
        group.add_argument("--device",
                           type=int,
                           nargs="+",
                           required=False,
                           help="training device")
        group.add_argument("--nodebug",
                           action="store_true",
                           required=False,
                           help="debug or not")
        group.add_argument("--dir",
                           type=str,
                           required=False,
                           help="evaluate existing npys")


    if phase == "demo":
        # group.add_argument("--motion_transfer", action='store_true', help="Motion Distribution Transfer")
        group.add_argument("--render",
                           action="store_true",
                           help="Render visulizaed figures")
        group.add_argument("--render_mode", type=str, help="video or sequence")
        group.add_argument(
            "--example",
            type=str,
            required=False,
            help="input text and lengths with txt format",
        )
        group.add_argument(
            "--out_dir",
            type=str,
            required=False,
            help="output dir",
        )
        group.add_argument(
            "--template",
            type=str,
            required=False,
            help="template path",
        )
        group.add_argument(
            "--checkpoint",
            type=str,
            required=True,
            help="output seperate or combined npy file",
        )
        group.add_argument(
            "--id",
            type=str,
            required=True,
            help="the candiate subect identity",
        )
        group.add_argument(
            "--ply",
            type=str,
            required=True,
            help="the candiate subect identity",
        )

    if phase == "render":
        group.add_argument(
            "--cfg",
            type=str,
            required=False,
            default="./configs/render.yaml",
            help="config file",
        )
        group.add_argument(
            "--cfg_assets",
            type=str,
            required=False,
            default="./configs/assets.yaml",
            help="config file for asset paths",
        )
        # group.add_argument("--motion_transfer", action='store_true', help="Motion Distribution Transfer")
        group.add_argument("--dir",
                           type=str,
                           required=False,
                           default=None,
                           help="npy motion folder")

    # remove None params, and create a dictionnary
    params = parser.parse_args()
    # params = {key: val for key, val in vars(opt).items() if val is not None}

    # update config from files
    cfg_base = OmegaConf.load('./configs/base.yaml')
    cfg_exp = OmegaConf.merge(cfg_base, OmegaConf.load(params.cfg))
    cfg_assets = OmegaConf.load(params.cfg_assets)
    cfg_model = get_module_config(cfg_exp.model, cfg_exp.model.target)
    cfg = OmegaConf.merge(cfg_exp, cfg_model, cfg_assets)

    if phase in ["train", "test"]:
        cfg.TRAIN.BATCH_SIZE = (params.batch_size
                                if params.batch_size else cfg.TRAIN.BATCH_SIZE)
        cfg.DEVICE = params.device if params.device else cfg.DEVICE
        cfg.DEBUG = not params.nodebug if params.nodebug is not None else cfg.DEBUG
        
        # resume from a checkpoint, added by Zhiyuan Ma
        cfg.TRAIN.RESUME = params.resume if params.resume else cfg.TRAIN.RESUME
        
        # no debug in test
        cfg.DEBUG = False if phase == "test" else cfg.DEBUG
        if phase == "test":
            cfg.DEBUG = False
            cfg.DEVICE = [0]
            print("Force no debugging and one gpu when testing")
        cfg.TEST.TEST_DIR = params.dir if params.dir else cfg.TEST.TEST_DIR

    if phase == "demo":
        # cfg.DEMO.MOTION_TRANSFER = params.motion_transfer
        cfg.DEMO.RENDER = params.render
        cfg.DEMO.FRAME_RATE = params.frame_rate
        cfg.DEMO.EXAMPLE = params.example
        cfg.DEMO.CHECKPOINTS = params.checkpoint
        cfg.DEMO.TEMPLATE = params.template
        cfg.DEMO.ID = params.id
        cfg.DEMO.PLY = params.ply
        cfg.TEST.FOLDER = params.out_dir if params.dir else cfg.TEST.FOLDER

    if phase == "render":
        if params.npy:
            cfg.RENDER.NPY = params.npy
            cfg.RENDER.INPUT_MODE = "npy"
        if params.dir:
            cfg.RENDER.DIR = params.dir
            cfg.RENDER.INPUT_MODE = "dir"
        cfg.RENDER.JOINT_TYPE = params.joint_type
        cfg.RENDER.MODE = params.mode

    # debug mode
    if cfg.DEBUG:
        cfg.NAME = "debug--" + cfg.NAME
        cfg.LOGGER.WANDB.OFFLINE = True
        cfg.LOGGER.VAL_EVERY_STEPS = 1

    return cfg


```

## Source file: eval_vocaset.py

{
  "path": "eval_vocaset.py",
  "blob_sha": "9d8012dffe1352e50e8b2c86a1f3df309f35105f",
  "sha256": "4c3f6afdc84262adf686551b6a11fe1a3b980525180d4d25514bf1cd60701406",
  "lines": 141,
  "bytes": 4626,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/theEricMa/DiffSpeaker/blob/0f2ae2d4e88fe78bff8bb7a1fdbc628e8733f120/eval_vocaset.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def print_table(title, metrics):"
    },
    {
      "line": 39,
      "text": "def get_metric_statistics(values, replication_times):"
    },
    {
      "line": 45,
      "text": "def main():"
    }
  ]
}

```text

#############################################################################################################
# this file is for evaluation of the vocaset dataset
#############################################################################################################
import os
import pickle
import torch
import json
import os
from pathlib import Path
from omegaconf import OmegaConf

import numpy as np
from alm.config import parse_args
from alm.models.get_model import get_model
from alm.utils.logger import create_logger
from alm.utils.demo_utils import animate

from rich import get_console
from rich.table import Table

import pytorch_lightning as pl
from alm.data.get_data import get_datasets
from alm.callback import ProgressLogger

from tqdm import tqdm

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
        os.path.join(cfg.FOLDER, str(cfg.model.model_type), str(cfg.NAME),
                     "samples_" + cfg.TIME))
    output_dir.mkdir(parents=True, exist_ok=True)
    logger.info(OmegaConf.to_yaml(cfg))

    # set seed
    pl.seed_everything(cfg.SEED_VALUE)
    
    # create dataset
    datasets = get_datasets(cfg, logger=logger, phase="test")[0]
    logger.info("datasets module {} initialized".format("".join(
        cfg.TRAIN.DATASETS)))

    # create model
    model = get_model(cfg, datasets)
    logger.info("model {} loaded".format(cfg.model.model_type))

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
        devices=list(range(len(cfg.DEVICE))),
        default_root_dir=cfg.FOLDER_EXP,
        reload_dataloaders_every_n_epochs=1,
        log_every_n_steps=cfg.LOGGER.LOG_EVERY_STEPS,
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
    state_dict.pop("denoiser.PPE.pe") # this is not needed, since the sequence length can be any flexiable
    model.load_state_dict(state_dict, strict=False)   

    all_metrics = {}
    replication_times = cfg.TEST.REPLICATION_TIMES
    seeds = np.arange(0, replication_times * 10, replication_times)
    # calculate metrics
    for i, seed in zip(tqdm(range(replication_times), desc="Evaluation among replications"), seeds):
        logger.info(f"Evaluation Replication {i}")
        metrics = trainer.test(model, datamodule=datasets)[0]#[0] # TODO

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

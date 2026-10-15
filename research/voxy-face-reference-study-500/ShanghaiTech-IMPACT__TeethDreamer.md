# Facial rendering source scan: ShanghaiTech-IMPACT/TeethDreamer

Repository: https://github.com/ShanghaiTech-IMPACT/TeethDreamer

Pinned source revision: a40073360a51b80b2fb5018c2618810a3bfab350

Discovery topic: teeth 3D

Description: [MICCAI 2024] TeethDreamer: 3D Teeth Reconstruction from Five Intra-oral Photographs

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"teeth": ["TeethDreamer.py", "configs/TeethDreamer.yaml", "example/teeth/1832_lower/000.png", "example/teeth/1832_lower/001.png", "example/teeth/1832_lower/002.png", "example/teeth/1832_lower/003.png", "example/teeth/1832_upper/000.png", "example/teeth/1832_upper/001.png", "example/teeth/1832_upper/002.png", "example/teeth/1832_upper/003.png", "ldm/data/teeth_dreamer.py", "ldm/models/diffusion/__pycache__/teeth_dreamer.cpython-38.pyc", "ldm/models/diffusion/teeth_dreamer.py", "seg_teeth.py"], "face": ["imgs/surface.png"]}

## Source file: seg_teeth.py

{
  "path": "seg_teeth.py",
  "blob_sha": "ff1173219833a2e6f50b854fc7bec767ae9be11f",
  "sha256": "50be5193eaa4a9858fd1cd590ea8c2c323924d6e9bf7ecbe1359358eac9524aa",
  "lines": 130,
  "bytes": 4516,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 3,
    "tooth": 7,
    "gum": 4,
    "face": 1
  },
  "url": "https://github.com/ShanghaiTech-IMPACT/TeethDreamer/blob/a40073360a51b80b2fb5018c2618810a3bfab350/seg_teeth.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def show_mask(mask, ax, random_color=False):"
    },
    {
      "line": 25,
      "text": "def show_points(coords, labels, ax, marker_size=175):"
    },
    {
      "line": 34,
      "text": "def show_box(box, ax):"
    },
    {
      "line": 40,
      "text": "def on_move(event):"
    },
    {
      "line": 47,
      "text": "def on_click(event):"
    },
    {
      "line": 65,
      "text": "def click_seg(image_path, mask_path):"
    }
  ]
}

```text

import numpy as np
import torch
import matplotlib.pyplot as plt
import cv2
from segment_anything import sam_model_registry, SamPredictor
import torchvision
import os
from skimage.io import imsave, imread
from PIL import Image
from matplotlib.pyplot import imshow
from matplotlib import pyplot as plt
import argparse
import sys

def show_mask(mask, ax, random_color=False):
    if random_color:
        color = np.concatenate([np.random.random(3), np.array([0.6])], axis=0)
    else:
        color = np.array([30 / 255, 144 / 255, 255 / 255, 0.6])
    h, w = mask.shape[-2:]
    mask_image = mask.reshape(h, w, 1) * color.reshape(1, 1, -1)
    ax.imshow(mask_image)


def show_points(coords, labels, ax, marker_size=175):
    pos_points = coords[labels == 1]
    neg_points = coords[labels == 0]
    ax.scatter(pos_points[:, 0], pos_points[:, 1], color='green', marker='*', s=marker_size, edgecolor='white',
               linewidth=1.25)
    ax.scatter(neg_points[:, 0], neg_points[:, 1], color='red', marker='*', s=marker_size, edgecolor='white',
               linewidth=1.25)


def show_box(box, ax):
    x0, y0 = box[0], box[1]
    w, h = box[2] - box[0], box[3] - box[1]
    ax.add_patch(plt.Rectangle((x0, y0), w, h, edgecolor='green', facecolor=(0, 0, 0, 0), lw=2))


def on_move(event):
    if event.inaxes:
        x = event.xdata
        y = event.ydata
        ax.set_title('Mouse Position: (%.2f, %.2f)' % (x, y))
        plt.draw()

def on_click(event):
    global masks_tot, tooth_mask
    if event.inaxes:
        ix, iy = float(event.xdata), float(event.ydata)
        clicks.append([ix, iy])
        if event.button==1:
            labels.append(1)
        else:
            labels.append(0)
        masks, scores, logits = predictor.predict(point_coords=np.array(clicks),
                                                    point_labels=np.array(labels),
                                                    multimask_output=False,
                                                    )
        plt.imshow(image)
        show_mask(masks[0], plt.gca())
        tooth_mask=masks[0].copy()
        plt.show()

def click_seg(image_path, mask_path):
    global out_name, clicks, labels, image, masks_tot, tooth_mask
    out_name = mask_path
    #print(image_path)
    image = cv2.imread(image_path)
    image = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
    h, w, c = image.shape
    x = max(h, w)
    padding = [(x - h) // 2, x - h - (x - h) // 2, (x - w) // 2, x - w - (x - w) // 2]
    image = cv2.copyMakeBorder(image, *padding, cv2.BORDER_CONSTANT, value=(0,) * 3)
    masks_tot = np.zeros((image.shape[0],image.shape[1])).astype(np.float32)

    clicks = []
    labels = []
    predictor.set_image(image)
    fig, ax = plt.subplots()
    plt.imshow(image)
    plt.axis('on')
    last_mask = masks_tot.copy()
    tooth_mask=np.array([])
    cid = fig.canvas.mpl_connect('button_press_event', on_click)
    plt.show()
    mask = tooth_mask[:,:,None].repeat(3,-1)
    img = np.asarray(image.data)
    img[mask < 0.5] = 0
    image = cv2.cvtColor(img, cv2.COLOR_RGB2BGR)
    cv2.imwrite(out_name, image)


if __name__=="__main__":
    sys.path.append(os.getcwd())
    parser = argparse.ArgumentParser()
    parser.add_argument('--img', required=True, help='The directory of your intra-oral images')
    parser.add_argument('--seg', required=True, help='The directory to store segmented images of teeth')
    parser.add_argument('--suffix', default='jpg', help='The suffix of your image files')
    args = parser.parse_args()
    mark_every_tooth=False
    image=None
    out_name=None
    clicks=None
    labels=None
    masks_tot=None
    tooth_mask=None
    sam_checkpoint = "ckpt/sam_vit_b_01ec64.pth"
    model_type = "vit_b"
    device = "cuda"
    sam = sam_model_registry[model_type](checkpoint=sam_checkpoint)
    sam.to(device=device)
    predictor = SamPredictor(sam)

    uid = os.path.basename(args.img)
    ### Segment Upper Teeth
    os.makedirs(os.path.join(args.seg, f"{uid}_upper"), exist_ok=True)
    img_id = [0, 1, 2, 3]
    for i in range(4):
        click_seg(os.path.join(args.img, f"{img_id[i]}.{args.suffix}"),
                  os.path.join(args.seg, f"{uid}_upper", f"{i:03d}.png"))

    ### Segment Lower Teeth
    os.makedirs(os.path.join(args.seg, f"{uid}_lower"), exist_ok=True)
    img_id = [0, 1, 2, 4]
    for i in range(4):
        click_seg(os.path.join(args.img, f"{img_id[i]}.{args.suffix}"),
                  os.path.join(args.seg, f"{uid}_lower", f"{i:03d}.png"))




```

## Source file: TeethDreamer.py

{
  "path": "TeethDreamer.py",
  "blob_sha": "d0e6169c3966922619d044fe4ab4e1a948f7f435",
  "sha256": "7c361f287f3e4e0fa5dfeb8890e368e38a4c314c5e51372b10a696a8840f9f8f",
  "lines": 342,
  "bytes": 15203,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 3,
    "gum": 13
  },
  "url": "https://github.com/ShanghaiTech-IMPACT/TeethDreamer/blob/a40073360a51b80b2fb5018c2618810a3bfab350/TeethDreamer.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def rank_zero_print(*args):"
    },
    {
      "line": 25,
      "text": "def get_parser(**parser_kwargs):"
    },
    {
      "line": 26,
      "text": "def str2bool(v):"
    },
    {
      "line": 49,
      "text": "def trainer_args(opt):"
    },
    {
      "line": 56,
      "text": "def __init__(self, resume, logdir, ckptdir, cfgdir, config):"
    },
    {
      "line": 64,
      "text": "def on_fit_start(self, trainer, pl_module):"
    },
    {
      "line": 78,
      "text": "def __init__(self, batch_frequency, max_images, log_images_kwargs=None):"
    },
    {
      "line": 85,
      "text": "def log_to_logger(self, pl_module, images, split):"
    },
    {
      "line": 94,
      "text": "def log_to_file(self, save_dir, split, images, global_step, current_epoch):"
    },
    {
      "line": 108,
      "text": "def log_img(self, pl_module, batch, split=\"train\"):"
    },
    {
      "line": 131,
      "text": "def check_frequency(self, check_idx):"
    },
    {
      "line": 137,
      "text": "def on_train_batch_end(self, trainer, pl_module, outputs, batch, batch_idx):"
    },
    {
      "line": 141,
      "text": "def on_validation_batch_end(self, trainer, pl_module, outputs, batch, batch_idx, dataloader_idx=0):"
    },
    {
      "line": 149,
      "text": "def on_train_epoch_start(self, trainer, pl_module):"
    },
    {
      "line": 155,
      "text": "def on_train_epoch_end(self, trainer, pl_module):"
    },
    {
      "line": 169,
      "text": "def get_node_name(name, parent_name):"
    },
    {
      "line": 178,
      "text": "def on_train_start(self, trainer, pl_module):"
    },
    {
      "line": 188,
      "text": "def load_pretrain_stable_diffusion(new_model, finetune_from):"
    },
    {
      "line": 221,
      "text": "def get_optional_dict(name, config):"
    }
  ]
}

```text

import argparse, os, sys
import numpy as np
import time
import torch
import torch.nn as nn
import torchvision
import pytorch_lightning as pl

from omegaconf import OmegaConf
from PIL import Image

from pytorch_lightning import seed_everything
from pytorch_lightning.strategies import DDPStrategy
from pytorch_lightning.trainer import Trainer
from pytorch_lightning.callbacks import ModelCheckpoint, Callback, LearningRateMonitor
from pytorch_lightning.utilities import rank_zero_info, rank_zero_only

from ldm.util import instantiate_from_config, load_config


@rank_zero_only
def rank_zero_print(*args):
    print(*args)

def get_parser(**parser_kwargs):
    def str2bool(v):
        if isinstance(v, bool):
            return v
        if v.lower() in ("yes", "true", "t", "y", "1"):
            return True
        elif v.lower() in ("no", "false", "f", "n", "0"):
            return False
        else:
            raise argparse.ArgumentTypeError("Boolean value expected.")

    parser = argparse.ArgumentParser(**parser_kwargs)
    parser.add_argument("-r", "--resume", dest='resume', action='store_true', default=False)
    parser.add_argument("-b", "--base", type=str, default='',)
    parser.add_argument("-l", "--logdir", type=str, default="logs", help="directory for logging data", )
    parser.add_argument("-c", "--ckptdir", type=str, default="models", help="directory for checkpoint data", )
    parser.add_argument("-s", "--seed", type=int, default=6033, help="seed for seed_everything", )
    parser.add_argument("--finetune_from", type=str, default="", help="path to checkpoint to load model state from" )
    parser.add_argument("--resume_from", type=str, default="", help="path to checkpoint to resume training")
    parser.add_argument("--output", type=str, default="", help="path to store generated images")
    parser.add_argument("--test", type=str, default="", help="path to checkpoint for inference")
    parser.add_argument("--gpus", type=str, default='0,')
    return parser

def trainer_args(opt):
    parser = argparse.ArgumentParser()
    parser = Trainer.add_argparse_args(parser)
    args = parser.parse_args([])
    return sorted(k for k in vars(args) if hasattr(opt, k))

class SetupCallback(Callback):
    def __init__(self, resume, logdir, ckptdir, cfgdir, config):
        super().__init__()
        self.resume = resume
        self.logdir = logdir
        self.ckptdir = ckptdir
        self.cfgdir = cfgdir
        self.config = config

    def on_fit_start(self, trainer, pl_module):
        if trainer.global_rank == 0:
            # Create logdirs and save configs
            os.makedirs(self.logdir, exist_ok=True)
            os.makedirs(self.ckptdir, exist_ok=True)
            os.makedirs(self.cfgdir, exist_ok=True)

            rank_zero_print(OmegaConf.to_yaml(self.config))
            OmegaConf.save(self.config, os.path.join(self.cfgdir, "configs.yaml"))

            if not self.resume and os.path.exists(os.path.join(self.logdir,'checkpoints','last.ckpt')):
                raise RuntimeError(f"checkpoint {os.path.join(self.logdir,'checkpoints','last.ckpt')} existing")

class ImageLogger(Callback):
    def __init__(self, batch_frequency, max_images, log_images_kwargs=None):
        super().__init__()
        self.batch_freq = batch_frequency
        self.max_images = max_images
        self.log_images_kwargs = log_images_kwargs if log_images_kwargs else {}

    @rank_zero_only
    def log_to_logger(self, pl_module, images, split):
        for k in images:
            grid = torchvision.utils.make_grid(images[k])
            grid = (grid + 1.0) / 2.0  # -1,1 -> 0,1; c,h,w

            tag = f"{split}/{k}"
            pl_module.logger.experiment.add_image(tag, grid, global_step=pl_module.global_step)

    @rank_zero_only
    def log_to_file(self, save_dir, split, images, global_step, current_epoch):
        root = os.path.join(save_dir, "images", split)
        for k in images:
            grid = torchvision.utils.make_grid(images[k], nrow=4)
            grid = (grid + 1.0) / 2.0  # -1,1 -> 0,1; c,h,w
            grid = grid.transpose(0, 1).transpose(1, 2).squeeze(-1)
            grid = grid.numpy()
            grid = (grid * 255).astype(np.uint8)
            filename = "{:06}-{:06}-{}.jpg".format(global_step, current_epoch, k)
            path = os.path.join(root, filename)
            os.makedirs(os.path.split(path)[0], exist_ok=True)
            Image.fromarray(grid).save(path)

    @rank_zero_only
    def log_img(self, pl_module, batch, split="train"):
        if split == "val": should_log = True
        else: should_log = self.check_frequency(pl_module.global_step)

        if should_log:
            is_train = pl_module.training
            if is_train: pl_module.eval()

            with torch.no_grad():
                images = pl_module.log_images(batch, split=split, **self.log_images_kwargs)

            for k in images:
                N = min(images[k].shape[0], self.max_images)
                images[k] = images[k][:N]
                if isinstance(images[k], torch.Tensor):
                    images[k] = images[k].detach().cpu()
                    images[k] = torch.clamp(images[k], -1., 1.)

            self.log_to_file(pl_module.logger.save_dir, split, images, pl_module.global_step, pl_module.current_epoch)
            # self.log_to_logger(pl_module, images, split)

            if is_train: pl_module.train()

    def check_frequency(self, check_idx):
        if (check_idx % self.batch_freq) == 0 and check_idx > 0:
            return True
        else:
            return False

    def on_train_batch_end(self, trainer, pl_module, outputs, batch, batch_idx):
        self.log_img(pl_module, batch, split="train")

    @rank_zero_only
    def on_validation_batch_end(self, trainer, pl_module, outputs, batch, batch_idx, dataloader_idx=0):
        # print('validation ....')
        # print(dataloader_idx)
        # print(batch_idx)
        if batch_idx==0: self.log_img(pl_module, batch, split="val")

class CUDACallback(Callback):
    # see https://github.com/SeanNaren/minGPT/blob/master/mingpt/callback.py
    def on_train_epoch_start(self, trainer, pl_module):
        # Reset the memory use counter
        torch.cuda.reset_peak_memory_stats(trainer.strategy.root_device.index)
        torch.cuda.synchronize(trainer.strategy.root_device.index)
        self.start_time = time.time()

    def on_train_epoch_end(self, trainer, pl_module):
        torch.cuda.synchronize(trainer.strategy.root_device.index)
        max_memory = torch.cuda.max_memory_allocated(trainer.strategy.root_device.index) / 2 ** 20
        epoch_time = time.time() - self.start_time

        try:
            max_memory = trainer.strategy.reduce(max_memory)
            epoch_time = trainer.strategy.reduce(epoch_time)

            rank_zero_info(f"Average Epoch time: {epoch_time:.2f} seconds")
            rank_zero_info(f"Average Peak memory {max_memory:.2f}MiB")
        except AttributeError:
            pass

def get_node_name(name, parent_name):
    if len(name) <= len(parent_name):
        return False, ''
    p = name[:len(parent_name)]
    if p != parent_name:
        return False, ''
    return True, name[len(parent_name):]

class ResumeCallBacks(Callback):
    def on_train_start(self, trainer, pl_module):
        pl_module.optimizers().param_groups = pl_module.optimizers()._optimizer.param_groups
        # print(trainer.optimizers)
        for i in range(len(trainer.optimizers[0].param_groups)):
            trainer.optimizers[0].param_groups[i]['capturable'] = True
        
        # trainer.optimizers[0].param_groups[0]['capturable'] = True
        # trainer.optimizers[0].param_groups[1]['capturable'] = True
        # trainer.optimizers[0].param_groups[2]['capturable'] = True

def load_pretrain_stable_diffusion(new_model, finetune_from):
    rank_zero_print(f"Attempting to load state from {finetune_from}")
    old_state = torch.load(finetune_from, map_location="cpu")
    if "state_dict" in old_state: old_state = old_state["state_dict"]

    in_filters_load = old_state["model.diffusion_model.input_blocks.0.0.weight"]
    new_state = new_model.state_dict()
    if "model.diffusion_model.input_blocks.0.0.weight" in new_state:
        in_filters_current = new_state["model.diffusion_model.input_blocks.0.0.weight"]
        in_shape = in_filters_current.shape
        ## because the model adopts additional inputs as conditions.
        if in_shape != in_filters_load.shape:
            input_keys = ["model.diffusion_model.input_blocks.0.0.weight", "model_ema.diffusion_modelinput_blocks00weight",]
            for input_key in input_keys:
                if input_key not in old_state or input_key not in new_state:
                    continue
                input_weight = new_state[input_key]
                if input_weight.size() != old_state[input_key].size():
                    print(f"Manual init: {input_key}")
                    input_weight.zero_()
                    #print(old_state[input_key].shape)
                    input_weight[:, :4, :, :].copy_(old_state[input_key][:, :4, :, :])
                old_state[input_key] = torch.nn.parameter.Parameter(input_weight)
    
    if 'cc_projection.weight' in new_state and 'cc_projection.weight' in old_state:
        if new_state['cc_projection.weight'].size() != old_state['cc_projection.weight']:
            input_weight = new_state['cc_projection.weight']
            input_weight.zero_()
            input_weight[:, :772].copy_(old_state['cc_projection.weight'][:, :772])
            old_state['cc_projection.weight'] = torch.nn.parameter.Parameter(input_weight)
    
    new_model.load_state_dict(old_state, strict=False)

def get_optional_dict(name, config):
    if name in config:
        cfg = config[name]
    else:
        cfg =  OmegaConf.create()
    return cfg

if __name__ == "__main__":

    torch.set_float32_matmul_precision('medium')
    os.environ['TORCH_DISTRIBUTED_DEBUG']='DETAIL'
    # now = datetime.datetime.now().strftime("%Y-%m-%dT%H-%M-%S")
    sys.path.append(os.getcwd())
    opt, extras = get_parser().parse_known_args()

    assert opt.base != ''
    name = os.path.split(opt.base)[-1]
    name = os.path.splitext(name)[0]
    logdir = os.path.join(opt.logdir, name)

    # logdir: checkpoints+configs
    ckptdir = os.path.join(opt.ckptdir, name)
    cfgdir = os.path.join(logdir, "configs")

    if opt.resume:
        ckpt = os.path.join(ckptdir, "last.ckpt")
        opt.resume_from_checkpoint = ckpt
        opt.finetune_from = "" # disable finetune checkpoint
    
    if opt.resume_from!="":
        opt.resume_from_checkpoint = opt.resume_from
    
    seed_everything(opt.seed)

    ###################config#####################
    #config = OmegaConf.load(opt.base)  # loade default configs
    config = load_config(opt.base, cli_args=extras)
    
    lightning_config = config.lightning
    trainer_config = config.lightning.trainer
    for k in trainer_args(opt): # overwrite trainer configs
        trainer_config[k] = getattr(opt, k)

    ###################trainer#####################
    # training framework
    gpuinfo = trainer_config["gpus"]
    rank_zero_print(f"Running on GPUs {gpuinfo}")
    ngpu = len(trainer_config.gpus.strip(",").split(','))
    trainer_config['devices'] = ngpu

    ###################model#####################
    model = instantiate_from_config(config.model)
    model.cpu()
    # load stable diffusion parameters
    if opt.finetune_from != "":
        load_pretrain_stable_diffusion(model, opt.finetune_from)

    ###################logger#####################
    # default logger configs
    default_logger_cfg = {"target": "pytorch_lightning.loggers.TensorBoardLogger",
                          "params": {"save_dir": logdir, "name": "tensorboard_logs", }}
    logger_cfg = OmegaConf.create(default_logger_cfg)
    logger = instantiate_from_config(logger_cfg)

    ###################callbacks#####################
    # default ckpt callbacks
    default_modelckpt_cfg = {"target": "pytorch_lightning.callbacks.ModelCheckpoint",
                             "params": {"dirpath": ckptdir, "filename": "{epoch:06}", "verbose": True, "save_last": True, "every_n_train_steps": 5000}}
    modelckpt_cfg = OmegaConf.merge(default_modelckpt_cfg, get_optional_dict("modelcheckpoint", lightning_config))  # overwrite checkpoint configs
    default_modelckpt_cfg_repeat = {"target": "pytorch_lightning.callbacks.ModelCheckpoint",
                                    "params": {"dirpath": ckptdir, "filename": "{step:08}", "verbose": True, "save_last": False, "every_n_train_steps": 5000, "save_top_k": -1}}
    modelckpt_cfg_repeat = OmegaConf.merge(default_modelckpt_cfg_repeat)

    # add callback which sets up log directory
    default_callbacks_cfg = {
        "setup_callback": {
            "target": "TeethDreamer.SetupCallback",
            "params": {"resume": opt.resume, "logdir": logdir, "ckptdir": ckptdir, "cfgdir": cfgdir, "config": config}
        },
        "learning_rate_logger": {
            "target": "TeethDreamer.LearningRateMonitor",
            "params": {"logging_interval": "step"}
        },
        "cuda_callback": {"target": "TeethDreamer.CUDACallback"},
    }
    callbacks_cfg = OmegaConf.merge(default_callbacks_cfg, get_optional_dict("callbacks", lightning_config))
    callbacks_cfg['model_ckpt'] = modelckpt_cfg  # add checkpoint
    callbacks_cfg['model_ckpt_repeat'] = modelckpt_cfg_repeat  # add checkpoint
    callbacks = [instantiate_from_config(callbacks_cfg[k]) for k in callbacks_cfg]  # construct all callbacks
    if opt.resume:
        callbacks.append(ResumeCallBacks())

    #trainer = Trainer.from_argparse_args(args=argparse.Namespace(), **trainer_config,
    #                                     accelerator='cuda', strategy=DDPStrategy(find_unused_parameters=False), logger=logger, callbacks=callbacks)
    trainer = Trainer.from_argparse_args(args=argparse.Namespace(), **trainer_config,
                                         accelerator='cuda', logger=logger, callbacks=callbacks)
    trainer.logdir = logdir

    ###################data#####################
    config.data.params.seed = opt.seed
    data = instantiate_from_config(config.data)
    data.prepare_data()

    ####################lr#####################
    bs, base_lr = config.data.params.batch_size, config.model.base_learning_rate
    accumulate_grad_batches = trainer_config.accumulate_grad_batches if hasattr(trainer_config, "trainer_config") else 1
    rank_zero_print(f"accumulate_grad_batches = {accumulate_grad_batches}")
    model.learning_rate = base_lr
    rank_zero_print("++++ NOT USING LR SCALING ++++")
    rank_zero_print(f"Setting learning rate to {model.learning_rate:.2e}")
    model.image_dir = logdir # used in output images during training
    model.output_dir = opt.output
    
    # trainer.optimizers().param_groups[0]['capturable'] = True
    # run
    if opt.test=="":
        data.setup('fit')
        trainer.fit(model, data)
    else:
        data.setup('test')
        model.names = os.listdir(data.test_dir)
        trainer.test(model, data, ckpt_path=opt.test)


```

## Source file: ldm/data/teeth_dreamer.py

{
  "path": "ldm/data/teeth_dreamer.py",
  "blob_sha": "9346eefb728314c7b4f14c9efcdb6a6d2663e32a",
  "sha256": "fa1df6c286d222b6bb21f3802001eb8d03c6c283171b3dcd65b9d78850f2dc68",
  "lines": 314,
  "bytes": 15039,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 7,
    "rig": 6
  },
  "url": "https://github.com/ShanghaiTech-IMPACT/TeethDreamer/blob/a40073360a51b80b2fb5018c2618810a3bfab350/ldm/data/teeth_dreamer.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def __init__(self, target_dir, input_dir, uid_set_pkl, normal_predict=False, image_size=256, bg_color='white'):"
    },
    {
      "line": 42,
      "text": "def __len__(self):"
    },
    {
      "line": 46,
      "text": "def get_bg_color(self):"
    },
    {
      "line": 66,
      "text": "def load_im(self, path, bg_color, normal=False, **kwargs):"
    },
    {
      "line": 83,
      "text": "def process_im(self, im):"
    },
    {
      "line": 88,
      "text": "def load_index(self, filename, index, bg_color, normal=False, **kwargs):"
    },
    {
      "line": 96,
      "text": "def get_data_for_index(self, index):"
    },
    {
      "line": 152,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 157,
      "text": "def __init__(self, image_dir, uid_set_pkl, image_size, crop_size, normal_predict):"
    },
    {
      "line": 182,
      "text": "def __len__(self):"
    },
    {
      "line": 185,
      "text": "def get_data_for_index(self, index):"
    },
    {
      "line": 215,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 220,
      "text": "def __init__(self, image_dir, uid_set_npy, image_size, crop_size, normal_predict=False, single_cond=True):"
    },
    {
      "line": 234,
      "text": "def __len__(self):"
    },
    {
      "line": 237,
      "text": "def get_data_for_index(self, index):"
    },
    {
      "line": 273,
      "text": "def __getitem__(self, index):"
    },
    {
      "line": 279,
      "text": "def __init__(self, target_dir, input_dir, validation_dir, test_dir, batch_size, uid_set_pkl, test_set_npy, num_workers=4, normal_predict=False, single_cond=True, image_size=256, val_crop_size=200, seed=0, bg_color='white', **kwargs):"
    },
    {
      "line": 297,
      "text": "def setup(self, stage):"
    },
    {
      "line": 304,
      "text": "def train_dataloader(self):"
    },
    {
      "line": 309,
      "text": "def val_dataloader(self):"
    },
    {
      "line": 313,
      "text": "def test_dataloader(self):"
    }
  ]
}

```text

import pytorch_lightning as pl
import numpy as np
import torch
import PIL
import os
from skimage.io import imread
import webdataset as wds
import PIL.Image as Image
from torch.utils.data import Dataset
from torch.utils.data.distributed import DistributedSampler
from pathlib import Path

from ldm.base_utils import read_pickle, pose_inverse, trans_normal
import torchvision.transforms as transforms
import torchvision
from einops import rearrange

from ldm.util import prepare_inputs
import random

class TeethDreamerTrainData(Dataset):
    def __init__(self, target_dir, input_dir, uid_set_pkl, normal_predict=False, image_size=256, bg_color='white'):
        self.default_image_size = 256
        self.image_size = image_size
        self.target_dir = Path(target_dir)
        self.input_dir = Path(input_dir)
        self.normal_predict = normal_predict
        self.bg_color = bg_color
        if 'mv' in uid_set_pkl:
            self.single_cond=False
        else:
            self.single_cond=True
        self.uids = read_pickle(uid_set_pkl)['train']
        
        print('============= length of train_dataset %d =============' % len(self.uids))

        image_transforms = []
        image_transforms.extend([transforms.ToTensor(), transforms.Lambda(lambda x: rearrange(x * 2. - 1., 'c h w -> h w c'))])
        self.image_transforms = torchvision.transforms.Compose(image_transforms)
        self.num_images = 16

    def __len__(self):
        # return len(self.uids)
        return self.uids.shape[0]
    
    def get_bg_color(self):
        if self.bg_color == 'white':
            bg_color = np.array([1., 1., 1.], dtype=np.float32)
        elif self.bg_color == 'black':
            bg_color = np.array([0., 0., 0.], dtype=np.float32)
        elif self.bg_color == 'gray':
            bg_color = np.array([0.5, 0.5, 0.5], dtype=np.float32)
        elif self.bg_color == 'random':
            bg_color = np.random.rand(3)
        elif self.bg_color == 'three_choices':
            white = np.array([1., 1., 1.], dtype=np.float32)
            black = np.array([0., 0., 0.], dtype=np.float32)
            gray = np.array([0.5, 0.5, 0.5], dtype=np.float32)
            bg_color = random.choice([white, black, gray])
        elif isinstance(self.bg_color, float):
            bg_color = np.array([self.bg_color] * 3, dtype=np.float32)
        else:
            raise NotImplementedError
        return bg_color
    
    def load_im(self, path, bg_color, normal=False, **kwargs):
        if normal:
            img = imread(path)
            img = img.astype(np.float32) / 255.0
            mask = img[:,:,3:].copy()
            normal = trans_normal(2*img[:,:,:3]-1, kwargs['RT_w2c_src'], kwargs['RT_w2c_tgt'])
            img = (normal*0.5 + 0.5).astype(np.float32)  # [0, 1]
            img = img * mask + (1 - mask) * bg_color 
            img = Image.fromarray(np.uint8(img * 255.))
        else:
            img = imread(path)
            img = img.astype(np.float32) / 255.0
            mask = img[:,:,3:]
            img[:,:,:3] = img[:,:,:3] * mask + (1 - mask) * bg_color 
            img = Image.fromarray(np.uint8(img[:, :, :3] * 255.))
        return img, mask

    def process_im(self, im):
        im = im.convert("RGB")
        im = im.resize((self.image_size, self.image_size), resample=PIL.Image.BICUBIC)
        return self.image_transforms(im)

    def load_index(self, filename, index, bg_color, normal=False, **kwargs):
        if normal:
            img, _ = self.load_im(os.path.join(filename, '%03d_normal.png' % index), bg_color, normal=True, **kwargs)
        else:
            img, _ = self.load_im(os.path.join(filename, '%03d.png' % index), bg_color)
        img = self.process_im(img)
        return img
   
    def get_data_for_index(self, index):
        if self.single_cond:
            target_dir = os.path.join(self.target_dir, self.uids[index])
            input_dir = os.path.join(self.input_dir, self.uids[index])
        else:
            target_dir = os.path.join(self.target_dir, self.uids[index]+'_front')
            if 'lower' in self.uids[index]:
                ext=['_front', '_left', '_right', '_up']
            else:
                ext=['_front', '_left', '_right', '_down']
            input_dir = [os.path.join(self.input_dir, self.uids[index]+ext[i]) for i in range(4)]

        views = np.arange(0, self.num_images)
        bg_color = self.get_bg_color()
        # start_view_index = np.random.randint(0, self.num_images)
        # views = (views + start_view_index) % self.num_images
        target_images = []
        target_K, target_az, target_el, target_dis, target_pose = read_pickle(os.path.join(target_dir, f'meta.pkl'))
        
        if self.normal_predict:
            views = np.arange(8, self.num_images)
            for si, target_index in enumerate(views):
                img = self.load_index(target_dir, target_index, bg_color)
                target_images.append(img)
            
            for si, target_index in enumerate(views):
                img = self.load_index(target_dir.replace('target','normal'), target_index, bg_color, normal=True, RT_w2c_src=target_pose[target_index], RT_w2c_tgt=target_pose[views[0]])
                target_images.append(img)
            
            target_K, target_az, target_el, target_dis, target_pose = target_K, target_az[views], target_el[views], target_dis[views], target_pose[views]
        else:
            for si, target_index in enumerate(views):
                img = self.load_index(target_dir, target_index, bg_color)
                target_images.append(img)
        
        target_images = torch.stack(target_images, 0)
        # input_img = self.load_index(input_dir, start_view_index)
        if self.single_cond:
            idx = np.random.choice(self.num_images,1)
            input_img = self.load_index(input_dir, idx, bg_color)
        else:
            idx = np.random.choice(self.num_images,4,replace=False)
            input_img = torch.cat([self.load_index(input_dir[i], idx[i], bg_color) for i in range(4)],-1)

        # K, azimuths, elevations, distances, cam_poses = read_pickle(os.path.join(input_dir, f'meta.pkl'))
        if self.single_cond:
            meta_path=os.path.join(input_dir, f'meta.pkl')
        else:
            meta_path=os.path.join(input_dir[-1], f'meta.pkl')
        meta = read_pickle(meta_path)
        input_az=np.array([meta[1][idx[-1]]]).astype(np.float32)
        input_el=np.array([meta[2][idx[-1]]]).astype(np.float32)
        input_dis=np.array([meta[3][idx[-1]]]).astype(np.float32)
        
        return {"target_image": target_images, "input_image": input_img, "input_az":input_az, "input_el":input_el, "input_dis":input_dis, "target_K":target_K, "target_az":target_az, "target_el":target_el, "target_dis":target_dis, "target_pose":target_pose}

    def __getitem__(self, index):
        data = self.get_data_for_index(index)
        return data

class TeethDreamerEvalData(Dataset):
    def __init__(self, image_dir, uid_set_pkl, image_size, crop_size, normal_predict):
        self.image_size = image_size
        self.image_dir = Path(image_dir)
        self.crop_size = crop_size
        self.uids = read_pickle(uid_set_pkl)['val']
        
        self.num_images = 16
        self.normal_predict = normal_predict
        if 'mv' in uid_set_pkl:
            self.view_idx = np.random.randint(0, self.num_images, (self.uids.shape[0], 4))

        else:
            self.view_idx = np.random.randint(0, self.num_images, (self.uids.shape[0], 1))

        self.target_cams = [read_pickle(f'meta_info/upper.pkl'), read_pickle(f'meta_info/lower.pkl')]
        
        if self.normal_predict and (not 'mv' in uid_set_pkl):
            occ=[]
            for i in self.uids:
                if i.split('_')[-1]=='down' or i.split('_')[-1]=='up':
                    occ.append(i)
            self.uids=np.array(occ)

        print('============= length of val_dataset %d =============' % self.uids.shape[0])

    def __len__(self):
        return self.uids.shape[0]

    def get_data_for_index(self, index):
        name = self.uids[index]
        if self.view_idx.shape[1] == 4:
            if 'lower' in name:
                ext=['_front', '_left', '_right', '_up']
            else:
                ext=['_front', '_left', '_right', '_down']
            input_img_fn = [os.path.join(self.image_dir, name+ext[i], f'{self.view_idx[index][i]:03d}.png') for i in range(4)]
            meta = read_pickle((self.image_dir) / (name+ext[-1]) / 'meta.pkl')
        else:
            input_img_fn = [(self.image_dir) / name / f"{self.view_idx[index][-1]:03d}.png"]
            meta = read_pickle((self.image_dir) / name / 'meta.pkl')
        input_az=np.array([meta[1][self.view_idx[index][-1]]]).astype(np.float32)
        input_el=np.array([meta[2][self.view_idx[index]][-1]]).astype(np.float32)
        input_dis=np.array([meta[3][self.view_idx[index][-1]]]).astype(np.float32)
        input_cam = {"input_az":input_az, "input_el":input_el, "input_dis":input_dis}
        target_K, target_az, target_el, target_dis, target_pose = self.target_cams[0] if 'upper' in name else self.target_cams[1]
        
        if self.normal_predict:
            views = np.arange(8, self.num_images)
            #views = np.arange(0, self.num_images)
            target_K, target_az, target_el, target_dis, target_pose = target_K, target_az[views], target_el[views], target_dis[views], target_pose[views]
        
        target_cams = {"target_K":target_K, "target_az":target_az, "target_el":target_el, "target_dis":target_dis, "target_pose":target_pose}
        data=prepare_inputs(input_img_fn, input_cam, target_cams, crop_size=self.crop_size, image_size=self.image_size)
        
        #data.update({'name':np.array([index],dtype=np.int64), 'cond':self.view_idx[index].astype(dtype=np.int64)})
        
        return data 

    def __getitem__(self, index):
        return self.get_data_for_index(index)
        
        
class TeethDreamerTestData(Dataset):
    def __init__(self, image_dir, uid_set_npy, image_size, crop_size, normal_predict=False, single_cond=True):
        self.image_size = image_size
        self.image_dir = Path(image_dir)
        self.crop_size = crop_size

        self.uids = np.array(os.listdir(image_dir))
        self.num_images = 16
        self.normal_predict = normal_predict
        self.single_cond = single_cond
        self.view_idx = np.zeros((self.uids.shape[0], 4)).astype(np.int32)
        self.target_cams = [read_pickle(f'meta_info/upper.pkl'), read_pickle(f'meta_info/lower.pkl')]
        
        print('============= length of test_dataset %d =============' % self.uids.shape[0])

    def __len__(self):
        return self.uids.shape[0]

    def get_data_for_index(self, index):
        name = self.uids[index]
        
        if 'lower' in name:
            ext=['_front', '_left', '_right', '_up']
        else:
            ext=['_front', '_left', '_right', '_down']

        input_img_fn = [os.path.join(self.image_dir, name, f'{i:03d}.png') for i in range(4)]
        meta = read_pickle('meta_info/cond.pkl')
        if 'upper' in name:
            input_az=np.array([meta[1][3]]).astype(np.float32)
            input_el=np.array([meta[2][3]]).astype(np.float32)
            input_dis=np.array([meta[3][3]]).astype(np.float32)
        else:
            input_az=np.array([meta[1][4]]).astype(np.float32)
            input_el=np.array([meta[2][4]]).astype(np.float32)
            input_dis=np.array([meta[3][4]]).astype(np.float32)
        
        input_cam = {"input_az":input_az, "input_el":input_el, "input_dis":input_dis}
        target_K, target_az, target_el, target_dis, target_pose = self.target_cams[0] if 'upper' in name else self.target_cams[1]
        
        if self.normal_predict:
            views = np.arange(8, self.num_images)
            target_K, target_az, target_el, target_dis, target_pose = target_K, target_az[views], target_el[views], target_dis[views], target_pose[views]
        
        target_cams = {"target_K":target_K, "target_az":target_az, "target_el":target_el, "target_dis":target_dis, "target_pose":target_pose}
        data=prepare_inputs(input_img_fn, input_cam, target_cams, crop_size=self.crop_size, image_size=self.image_size)
        
        if not self.single_cond:
            data.update({'name':np.array([index],dtype=np.int64), 'cond':self.view_idx[index].astype(dtype=np.int64)})
        else:
            data.update({'name':np.array([index],dtype=np.int64), 'cond':self.view_idx[index][[-1]].astype(dtype=np.int64)})
        
        return data 

    def __getitem__(self, index):
        return self.get_data_for_index(index)
                


class TeethDreamerDataset(pl.LightningDataModule):
    def __init__(self, target_dir, input_dir, validation_dir, test_dir, batch_size, uid_set_pkl, test_set_npy, num_workers=4, normal_predict=False, single_cond=True, image_size=256, val_crop_size=200, seed=0, bg_color='white', **kwargs):
        super().__init__()
        self.target_dir = target_dir
        self.input_dir = input_dir
        self.validation_dir = validation_dir
        self.batch_size = batch_size
        self.num_workers = num_workers
        self.normal_predict = normal_predict
        self.uid_set_pkl = uid_set_pkl
        self.seed = seed
        self.additional_args = kwargs
        self.image_size = image_size
        self.val_crop_size = val_crop_size
        self.bg_color = bg_color
        self.test_dir = test_dir
        self.test_set_npy = test_set_npy
        self.single_cond = single_cond

    def setup(self, stage):
        if stage in ['fit']:
            self.train_dataset = TeethDreamerTrainData(self.target_dir, self.input_dir, uid_set_pkl=self.uid_set_pkl, normal_predict=self.normal_predict, image_size=self.image_size, bg_color=self.bg_color)
            self.val_dataset = TeethDreamerEvalData(image_dir=self.validation_dir, uid_set_pkl=self.uid_set_pkl, image_size=self.image_size, crop_size=self.val_crop_size, normal_predict=self.normal_predict)
        else:
            self.test_dataset = TeethDreamerTestData(image_dir=self.test_dir, uid_set_npy=self.test_set_npy, image_size=self.image_size, crop_size=self.val_crop_size, normal_predict=self.normal_predict, single_cond=self.single_cond)

    def train_dataloader(self):
        #sampler = DistributedSampler(self.train_dataset, seed=self.seed)
        #return wds.WebLoader(self.train_dataset, batch_size=self.batch_size, num_workers=self.num_workers, shuffle=False, sampler=sampler)
        return wds.WebLoader(self.train_dataset, batch_size=self.batch_size, num_workers=self.num_workers, shuffle=True)

    def val_dataloader(self):
        loader = wds.WebLoader(self.val_dataset, batch_size=self.batch_size, num_workers=self.num_workers, shuffle=False)
        return loader

    def test_dataloader(self):
        return wds.WebLoader(self.test_dataset, batch_size=self.batch_size, num_workers=self.num_workers, shuffle=False)


```

## Source file: ldm/models/diffusion/teeth_dreamer.py

{
  "path": "ldm/models/diffusion/teeth_dreamer.py",
  "blob_sha": "433c12ac8d7cd685a7d92ab9f61d8ae34024d1bf",
  "sha256": "7ffb845de36a1b63f776933b82ca71f60d2d37c5d7e6a0d12dbb11e7f3262941",
  "lines": 766,
  "bytes": 37378,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "teeth": 4,
    "rig": 3,
    "lip": 54
  },
  "url": "https://github.com/ShanghaiTech-IMPACT/TeethDreamer/blob/a40073360a51b80b2fb5018c2618810a3bfab350/ldm/models/diffusion/teeth_dreamer.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def disabled_train(self, mode=True):"
    },
    {
      "line": 24,
      "text": "def disable_training_module(module: nn.Module):"
    },
    {
      "line": 31,
      "text": "def repeat_to_batch(tensor, B, VN):"
    },
    {
      "line": 38,
      "text": "def __init__(self, diff_model_config, drop_conditions=False, drop_scheme='default', use_zero_123=True):"
    },
    {
      "line": 46,
      "text": "def drop(self, cond, mask):"
    },
    {
      "line": 52,
      "text": "def get_trainable_parameters(self):"
    },
    {
      "line": 55,
      "text": "def get_drop_scheme(self, B, device):"
    },
    {
      "line": 66,
      "text": "def forward(self, x, t, clip_embed, volume_feats, x_concat, is_train=False, **kwargs):"
    },
    {
      "line": 103,
      "text": "def predict_with_unconditional_scale(self, x, t, clip_embed, volume_feats, x_concat, unconditional_scale, **kwargs):"
    },
    {
      "line": 133,
      "text": "def __init__(self, time_dim, view_dim, view_num,"
    },
    {
      "line": 154,
      "text": "def construct_spatial_volume(self, x, t_embed, v_embed, target_poses, target_Ks):"
    },
    {
      "line": 203,
      "text": "def construct_view_frustum_volume(self, spatial_volume, t_embed, v_embed, poses, Ks, target_indices, **kwargs):"
    },
    {
      "line": 246,
      "text": "def __init__(self, unet_config, scheduler_config,"
    },
    {
      "line": 292,
      "text": "def _init_clip_projection(self):"
    },
    {
      "line": 301,
      "text": "def get_viewpoint_embedding(self, dp):"
    },
    {
      "line": 315,
      "text": "def _init_first_stage(self):"
    },
    {
      "line": 340,
      "text": "def _init_clip_image_encoder(self):"
    },
    {
      "line": 344,
      "text": "def _init_schedule(self):"
    },
    {
      "line": 368,
      "text": "def _init_time_step_embedding(self):"
    },
    {
      "line": 376,
      "text": "def encode_first_stage(self, x, sample=True):"
    },
    {
      "line": 384,
      "text": "def decode_first_stage(self, z):"
    },
    {
      "line": 389,
      "text": "def prepare(self, batch):"
    },
    {
      "line": 422,
      "text": "def embed_time(self, t):"
    },
    {
      "line": 427,
      "text": "def get_target_view_feats(self, x_input, spatial_volume, clip_embed, t_embed, v_embed, target_index, normal=False, **kwargs):"
    },
    {
      "line": 455,
      "text": "def training_step(self, batch):"
    },
    {
      "line": 502,
      "text": "def add_noise(self, x_start, t):"
    },
    {
      "line": 518,
      "text": "def sample(self, sampler, batch, cfg_scale, batch_view_num, return_inter_results=False, inter_interval=50, inter_view_interval=2):"
    },
    {
      "line": 543,
      "text": "def log_image(self,  x_sample, batch, step, output_dir):"
    },
    {
      "line": 557,
      "text": "def validation_step(self, batch, batch_idx):"
    },
    {
      "line": 570,
      "text": "def save_image(self,  x_sample, batch, output_dir):"
    },
    {
      "line": 585,
      "text": "def test_step(self, batch, batch_idx):"
    },
    {
      "line": 594,
      "text": "def configure_optimizers(self):"
    },
    {
      "line": 620,
      "text": "def __init__(self, model: TeethDreamer, ddim_num_steps, ddim_discretize=\"uniform\", ddim_eta=1.0, latent_size=32):"
    },
    {
      "line": 628,
      "text": "def _make_schedule(self,  ddim_num_steps, ddim_discretize=\"uniform\", ddim_eta=0., verbose=True):"
    },
    {
      "line": 646,
      "text": "def denoise_apply_impl(self, x_target_noisy, index, noise_pred, is_step0=False):"
    },
    {
      "line": 672,
      "text": "def denoise_apply(self, x_target_noisy, input_info, clip_embed, time_steps, index, unconditional_scale, batch_view_num=1, is_step0=False):"
    },
    {
      "line": 737,
      "text": "def sample(self, input_info, clip_embed, unconditional_scale=1.0, log_every_t=50, batch_view_num=1):"
    }
  ]
}

```text

from pathlib import Path

import pytorch_lightning as pl
import torch
import torch.nn as nn
import torch.nn.functional as F
import numpy as np
from skimage.io import imsave
from torch.optim.lr_scheduler import LambdaLR
from tqdm import tqdm

from ldm.base_utils import read_pickle, concat_images_list, camNormal2worldNormal
from ldm.models.diffusion.sync_dreamer_utils import get_warp_coordinates, create_target_volume
from ldm.models.diffusion.sync_dreamer_network import NoisyTargetViewEncoder, SpatialTime3DNet, FrustumTV3DNet
from ldm.modules.diffusionmodules.util import make_ddim_timesteps, timestep_embedding
from ldm.modules.encoders.modules import FrozenCLIPImageEmbedder
from ldm.util import instantiate_from_config

def disabled_train(self, mode=True):
    """Overwrite model.train with this function to make sure train/eval mode
    does not change anymore."""
    return self

def disable_training_module(module: nn.Module):
    module = module.eval()
    module.train = disabled_train
    for para in module.parameters():
        para.requires_grad = False
    return module

def repeat_to_batch(tensor, B, VN):
    t_shape = tensor.shape
    ones = [1 for _ in range(len(t_shape)-1)]
    tensor_new = tensor.view(B,1,*t_shape[1:]).repeat(1,VN,*ones).view(B*VN,*t_shape[1:])
    return tensor_new

class UNetWrapper(nn.Module):
    def __init__(self, diff_model_config, drop_conditions=False, drop_scheme='default', use_zero_123=True):
        super().__init__()
        self.diffusion_model = instantiate_from_config(diff_model_config)
        self.drop_conditions = drop_conditions
        self.drop_scheme=drop_scheme
        self.use_zero_123 = use_zero_123


    def drop(self, cond, mask):
        shape = cond.shape
        B = shape[0]
        cond = mask.view(B,*[1 for _ in range(len(shape)-1)]) * cond
        return cond

    def get_trainable_parameters(self):
        return self.diffusion_model.get_trainable_parameters()

    def get_drop_scheme(self, B, device):
        if self.drop_scheme=='default':
            random = torch.rand(B, dtype=torch.float32, device=device)
            drop_clip = (random > 0.15) & (random <= 0.2)
            drop_volume = (random > 0.1) & (random <= 0.15)
            drop_concat = (random > 0.05) & (random <= 0.1)
            drop_all = random <= 0.05
        else:
            raise NotImplementedError
        return drop_clip, drop_volume, drop_concat, drop_all

    def forward(self, x, t, clip_embed, volume_feats, x_concat, is_train=False, **kwargs):
        """

        @param x:             B,4,H,W
        @param t:             B,
        @param clip_embed:    B,M,768
        @param volume_feats:  B,C,D,H,W
        @param x_concat:      B,C,H,W
        @param is_train:
        @return:
        """
        if self.drop_conditions and is_train:
            B = x.shape[0]
            drop_clip, drop_volume, drop_concat, drop_all = self.get_drop_scheme(B, x.device)

            clip_mask = 1.0 - (drop_clip | drop_all).float()
            clip_embed = self.drop(clip_embed, clip_mask)

            volume_mask = 1.0 - (drop_volume | drop_all).float()
            for k, v in volume_feats.items():
                volume_feats[k] = self.drop(v, mask=volume_mask)

            concat_mask = 1.0 - (drop_concat | drop_all).float()
            x_concat = self.drop(x_concat, concat_mask)

        if self.use_zero_123:
            # zero123 does not multiply this when encoding, maybe a bug for zero123
            first_stage_scale_factor = 0.18215
            x_concat_ = x_concat * 1.0 / first_stage_scale_factor     #!!!!!
            #x_concat_[:, :4] = x_concat_[:, :4] / first_stage_scale_factor
        else:
            x_concat_ = x_concat

        x = torch.cat([x, x_concat_], 1)
        pred = self.diffusion_model(x, t, clip_embed, source_dict=volume_feats, **kwargs)
        return pred

    def predict_with_unconditional_scale(self, x, t, clip_embed, volume_feats, x_concat, unconditional_scale, **kwargs):
        x_ = torch.cat([x] * 2, 0)
        t_ = torch.cat([t] * 2, 0)
        clip_embed_ = torch.cat([clip_embed, torch.zeros_like(clip_embed)], 0)
        if 'camera_embedding' in kwargs:
            kwargs['camera_embedding'] = torch.cat([kwargs['camera_embedding']] * 2, 0)
        
        v_ = {}
        for k, v in volume_feats.items():
            v_[k] = torch.cat([v, torch.zeros_like(v)], 0)
            #v_[k] = torch.cat([torch.zeros_like(v), torch.zeros_like(v)], 0)

        x_concat_ = torch.cat([x_concat, torch.zeros_like(x_concat)], 0)
        if self.use_zero_123:
            # zero123 does not multiply this when encoding, maybe a bug for zero123
            first_stage_scale_factor = 0.18215
            x_concat_ = x_concat_ * 1.0 / first_stage_scale_factor      #!!!!!
            #x_concat_[:, :4] = x_concat_[:, :4] / first_stage_scale_factor
        
        #print(x_.shape)
        #print(x_concat_.shape)
        
        x_ = torch.cat([x_, x_concat_], 1)
        # s, s_uc = self.diffusion_model(x_, t_, clip_embed_, source_dict=v_).chunk(2)
        s, s_uc = self.diffusion_model(x_, t_, clip_embed_, source_dict=v_, **kwargs).chunk(2)
        s = s_uc + unconditional_scale * (s - s_uc)
        return s


class SpatialVolumeNet(nn.Module):
    def __init__(self, time_dim, view_dim, view_num,
                 input_image_size=256, frustum_volume_depth=48,
                 spatial_volume_size=32, spatial_volume_length=0.5,
                 frustum_volume_length=0.86603 # sqrt(3)/2
                 ):
        super().__init__()
        self.target_encoder = NoisyTargetViewEncoder(time_dim, view_dim, output_dim=16)
        self.spatial_volume_feats = SpatialTime3DNet(input_dim=16 * view_num, time_dim=time_dim, dims=(64, 128, 256, 512))
        self.frustum_volume_feats = FrustumTV3DNet(64, time_dim, view_dim, dims=(64, 128, 256, 512))

        self.frustum_volume_length = frustum_volume_length
        self.input_image_size = input_image_size
        self.spatial_volume_size = spatial_volume_size
        self.spatial_volume_length = spatial_volume_length

        self.frustum_volume_size = self.input_image_size // 8
        self.frustum_volume_depth = frustum_volume_depth
        self.time_dim = time_dim
        self.view_dim = view_dim
        self.default_origin_depth = 1.5 # our rendered images are 1.5 away from the origin, we assume camera is 1.5 away from the origin

    def construct_spatial_volume(self, x, t_embed, v_embed, target_poses, target_Ks):
        """
        @param x:            B,N,4,H,W
        @param t_embed:      B,t_dim
        @param v_embed:      B,N,v_dim
        @param target_poses: N,3,4
        @param target_Ks:    N,3,3
        @return:
        """
        B, N, _, H, W = x.shape
        V = self.spatial_volume_size
        device = x.device

        spatial_volume_verts = torch.linspace(-self.spatial_volume_length, self.spatial_volume_length, V, dtype=torch.float32, device=device)
        spatial_volume_verts = torch.stack(torch.meshgrid(spatial_volume_verts, spatial_volume_verts, spatial_volume_verts), -1)
        spatial_volume_verts = spatial_volume_verts.reshape(1, V ** 3, 3)[:, :, (2, 1, 0)]
        spatial_volume_verts = spatial_volume_verts.view(1, V, V, V, 3).permute(0, 4, 1, 2, 3).repeat(B, 1, 1, 1, 1)

        # encode source features
        t_embed_ = t_embed.view(B, 1, self.time_dim).repeat(1, N, 1).view(B, N, self.time_dim)
        # v_embed_ = v_embed.view(1, N, self.view_dim).repeat(B, 1, 1).view(B, N, self.view_dim)
        v_embed_ = v_embed 
        # target_Ks = target_Ks.unsqueeze(0).repeat(B, 1, 1, 1)
        # target_poses = target_poses.unsqueeze(0).repeat(B, 1, 1, 1)

        # extract 2D image features
        spatial_volume_feats = []
        # project source features
        for ni in range(0, N):
            pose_source_ = target_poses[:, ni]
            K_source_ = target_Ks[:, ni]
            # print(pose_source_.shape)
            # print(K_source_.shape)
            
            x_ = self.target_encoder(x[:, ni], t_embed_[:, ni], v_embed_[:, ni])
            C = x_.shape[1]

            coords_source = get_warp_coordinates(spatial_volume_verts, x_.shape[-1], self.input_image_size, K_source_, pose_source_).view(B, V, V * V, 2)
            unproj_feats_ = F.grid_sample(x_, coords_source, mode='bilinear', padding_mode='zeros', align_corners=True)
            unproj_feats_ = unproj_feats_.view(B, C, V, V, V)
            spatial_volume_feats.append(unproj_feats_)

        spatial_volume_feats = torch.stack(spatial_volume_feats, 1) # B,N,C,V,V,V
        N = spatial_volume_feats.shape[1]
        spatial_volume_feats = spatial_volume_feats.view(B, N*C, V, V, V)

        spatial_volume_feats = self.spatial_volume_feats(spatial_volume_feats, t_embed)  # b,64,32,32,32
        return spatial_volume_feats

    def construct_view_frustum_volume(self, spatial_volume, t_embed, v_embed, poses, Ks, target_indices, **kwargs):
        """
        @param spatial_volume:    B,C,V,V,V
        @param t_embed:           B,t_dim
        @param v_embed:           B,N,v_dim
        @param poses:             N,3,4
        @param Ks:                N,3,3
        @param target_indices:    B,TN
        @return: B*TN,C,H,W
        """
        B, TN = target_indices.shape
        H, W = self.frustum_volume_size, self.frustum_volume_size
        D = self.frustum_volume_depth
        V = self.spatial_volume_size

        near = torch.ones(B * TN, 1, H, W, dtype=spatial_volume.dtype, device=spatial_volume.device) * self.default_origin_depth - self.frustum_volume_length
        far = torch.ones(B * TN, 1, H, W, dtype=spatial_volume.dtype, device=spatial_volume.device) * self.default_origin_depth + self.frustum_volume_length

        # target_indices = target_indices.view(B*TN) # B*TN
        # poses_ = poses[target_indices] # B*TN,3,4
        # Ks_ = Ks[target_indices] # B*TN,3,3
        batch_idices = torch.arange(0, target_indices.shape[0])[:,None].repeat(1, target_indices.shape[1]).long().to(target_indices.device).flatten()
        target_indices = target_indices.flatten()
        poses_ = poses[batch_idices, target_indices]
        Ks_ = Ks[batch_idices, target_indices]
        
        volume_xyz, volume_depth = create_target_volume(D, self.frustum_volume_size, self.input_image_size, poses_, Ks_, near, far) # B*TN,3 or 1,D,H,W

        volume_xyz_ = volume_xyz / self.spatial_volume_length  # since the spatial volume is constructed in [-spatial_volume_length,spatial_volume_length]
        volume_xyz_ = volume_xyz_.permute(0, 2, 3, 4, 1)  # B*TN,D,H,W,3
        spatial_volume_ = spatial_volume.unsqueeze(1).repeat(1, TN, 1, 1, 1, 1).view(B * TN, -1, V, V, V)
        volume_feats = F.grid_sample(spatial_volume_, volume_xyz_, mode='bilinear', padding_mode='zeros', align_corners=True) # B*TN,C,D,H,W
        #print(H,W)
        v_embed_ = v_embed[torch.arange(B)[:,None], target_indices.view(B,TN)].view(B*TN, -1) # B*TN
        t_embed_ = t_embed.unsqueeze(1).repeat(1,TN,1).view(B*TN,-1)
        if 'frustum_volume_feats' in kwargs:
            volume_feats_dict = kwargs['frustum_volume_feats'](volume_feats, t_embed_, v_embed_)
        else:
            volume_feats_dict = self.frustum_volume_feats(volume_feats, t_embed_, v_embed_)
        #for k,v in volume_feats_dict.items(): print(k,v.shape)
        return volume_feats_dict, volume_depth
        
class TeethDreamer(pl.LightningModule):
    def __init__(self, unet_config, scheduler_config,
                 finetune_unet=False, finetune_projection=True,
                 view_num=16, image_size=256,
                 cfg_scale=3.0, output_num=8, batch_view_num=4,
                 drop_conditions=False, drop_scheme='default',
                 clip_image_encoder_path="/apdcephfs/private_rondyliu/projects/clip/ViT-L-14.pt",
                 sample_type='ddim', sample_steps=200):
        super().__init__()

        self.finetune_unet = finetune_unet
        self.finetune_projection = finetune_projection

        self.view_num = view_num
        self.viewpoint_dim = 6
        self.output_num = output_num
        self.image_size = image_size

        self.batch_view_num = batch_view_num
        self.cfg_scale = cfg_scale

        self.clip_image_encoder_path = clip_image_encoder_path

        self._init_time_step_embedding()
        self._init_first_stage()
        self._init_schedule()
        self._init_clip_image_encoder()
        self._init_clip_projection()
        
        self.normal_volume = SpatialVolumeNet(self.time_embed_dim, self.viewpoint_dim, self.view_num, input_image_size=self.image_size)
        self.volume_merge = SpatialTime3DNet(input_dim=128, time_dim=self.time_embed_dim, dims=(64, 128, 256, 512))
        self.volume_frustum = FrustumTV3DNet(64, self.time_embed_dim, self.viewpoint_dim, dims=(64, 128, 256, 512))
        
        self.spatial_volume = SpatialVolumeNet(self.time_embed_dim, self.viewpoint_dim, self.view_num, input_image_size=self.image_size)
        
        del self.spatial_volume.frustum_volume_feats
        del self.normal_volume.frustum_volume_feats
        
        self.model = UNetWrapper(unet_config, drop_conditions=drop_conditions, drop_scheme=drop_scheme, use_zero_123=True)
        self.scheduler_config = scheduler_config
        
        latent_size = image_size//8
        if sample_type=='ddim':
            self.sampler = TeethDDIMSampler(self, sample_steps , "uniform", 1.0, latent_size=latent_size)
        else:
            raise NotImplementedError
    
    def _init_clip_projection(self):
        self.cc_projection = nn.Linear(768+self.viewpoint_dim, 768)
        nn.init.eye_(list(self.cc_projection.parameters())[0][:768, :768])
        nn.init.zeros_(list(self.cc_projection.parameters())[1])
        self.cc_projection.requires_grad_(True)

        if not self.finetune_projection:
            disable_training_module(self.cc_projection)

    def get_viewpoint_embedding(self, dp):
        """
        @param dp {da,de,dz}
        @return: cam_pose_embedding
        """
        embedding = torch.stack([dp['de'], torch.sin(dp['da']), torch.cos(dp['da']), dp['dz']], -1) # B,N,4
        embed = torch.tensor([1,0], dtype=embedding.dtype, device=embedding.device)
        embed = embed[None,None].repeat(embedding.shape[0], 2*embedding.shape[1], 1)
        embed[:,embedding.shape[1]:,0] = 0
        embed[:,embedding.shape[1]:,1] = 1
        embedding = torch.cat([embedding.repeat(1,2,1), embed], -1) # B,2*N,6
        return embedding
    
    
    def _init_first_stage(self):
        first_stage_config={
            "target": "ldm.models.autoencoder.AutoencoderKL",
            "params": {
                "embed_dim": 4,
                "monitor": "val/rec_loss",
                "ddconfig":{
                  "double_z": True,
                  "z_channels": 4,
                  "resolution": self.image_size,
                  "in_channels": 3,
                  "out_ch": 3,
                  "ch": 128,
                  "ch_mult": [1,2,4,4],
                  "num_res_blocks": 2,
                  "attn_resolutions": [],
                  "dropout": 0.0
                },
                "lossconfig": {"target": "torch.nn.Identity"},
            }
        }
        self.first_stage_scale_factor = 0.18215
        self.first_stage_model = instantiate_from_config(first_stage_config)
        self.first_stage_model = disable_training_module(self.first_stage_model)

    def _init_clip_image_encoder(self):
        self.clip_image_encoder = FrozenCLIPImageEmbedder(model=self.clip_image_encoder_path)
        self.clip_image_encoder = disable_training_module(self.clip_image_encoder)

    def _init_schedule(self):
        self.num_timesteps = 1000
        linear_start = 0.00085
        linear_end = 0.0120
        num_timesteps = 1000
        betas = torch.linspace(linear_start ** 0.5, linear_end ** 0.5, num_timesteps, dtype=torch.float32) ** 2 # T
        assert betas.shape[0] == self.num_timesteps

        # all in float64 first
        alphas = 1. - betas
        alphas_cumprod = torch.cumprod(alphas, dim=0) # T
        alphas_cumprod_prev = torch.cat([torch.ones(1, dtype=torch.float64), alphas_cumprod[:-1]], 0)
        posterior_variance = betas * (1. - alphas_cumprod_prev) / (1. - alphas_cumprod) # T
        posterior_log_variance_clipped = torch.log(torch.clamp(posterior_variance, min=1e-20))
        posterior_log_variance_clipped = torch.clamp(posterior_log_variance_clipped, min=-10)

        self.register_buffer("betas", betas.float())
        self.register_buffer("alphas", alphas.float())
        self.register_buffer("alphas_cumprod", alphas_cumprod.float())
        self.register_buffer("sqrt_alphas_cumprod", torch.sqrt(alphas_cumprod).float())
        self.register_buffer("sqrt_one_minus_alphas_cumprod", torch.sqrt(1 - alphas_cumprod).float())
        self.register_buffer("posterior_variance", posterior_variance.float())
        self.register_buffer('posterior_log_variance_clipped', posterior_log_variance_clipped.float())

    def _init_time_step_embedding(self):
        self.time_embed_dim = 256
        self.time_embed = nn.Sequential(
            nn.Linear(self.time_embed_dim, self.time_embed_dim),
            nn.SiLU(True),
            nn.Linear(self.time_embed_dim, self.time_embed_dim),
        )

    def encode_first_stage(self, x, sample=True):
        with torch.no_grad():
            posterior = self.first_stage_model.encode(x)  # b,4,h//8,w//8
            if sample:
                return posterior.sample().detach() * self.first_stage_scale_factor
            else:
                return posterior.mode().detach() * self.first_stage_scale_factor

    def decode_first_stage(self, z):
        with torch.no_grad():
            z = 1. / self.first_stage_scale_factor * z
            return self.first_stage_model.decode(z)

    def prepare(self, batch):
        # encode target
        if 'target_image' in batch:
            image_target = batch['target_image'].permute(0, 1, 4, 2, 3) # b,n,3,h,w
            N = image_target.shape[1]
            x = [self.encode_first_stage(image_target[:,ni], True) for ni in range(N)]
            x = torch.stack(x, 1) # b,n,4,h//8,w//8
        else:
            x = None
        
        image_input = batch['input_image'].permute(0, 3, 1, 2)
        x_input = torch.cat([self.encode_first_stage(image_input[:,i*3:(i+1)*3,:,:]) for i in range(image_input.shape[1]//3)],1)


        azimuth_input = batch['input_az']
        azimuth_target = batch['target_az']
        elevation_input = -batch['input_el']
        elevation_target = -batch['target_el']
        distance_input = batch['input_dis']
        distance_target = batch['target_dis']
        N = azimuth_target.shape[1]
        

        d_e = elevation_target - elevation_input.repeat(1, N)
        d_a = azimuth_target - azimuth_input.repeat(1, N)
        d_z = distance_target - distance_input.repeat(1, N)
        d_p = {'da': d_a, 'de': d_e, 'dz': d_z}
        
        input_info = {'image': image_input, 'dp': d_p, 'x': x_input}
        with torch.no_grad():
            clip_embed = torch.cat([self.clip_image_encoder.encode(image_input[:,i*3:(i+1)*3,:,:]) for i in range(image_input.shape[1]//3)],1)
        return x, clip_embed, input_info

    def embed_time(self, t):
        t_embed = timestep_embedding(t, self.time_embed_dim, repeat_only=False) # B,TED
        t_embed = self.time_embed(t_embed) # B,TED
        return t_embed

    def get_target_view_feats(self, x_input, spatial_volume, clip_embed, t_embed, v_embed, target_index, normal=False, **kwargs):
        """
        @param x_input:        B,4,H,W
        @param spatial_volume: B,C,V,V,V
        @param clip_embed:     B,1,768
        @param t_embed:        B,t_dim
        @param v_embed:        B,N,v_dim
        @param target_index:   B,TN
        @return:
            tensors of size B*TN,*
        """
        B, _, H, W = x_input.shape
        if normal:
            frustum_volume_feats, frustum_volume_depth = self.normal_volume.construct_view_frustum_volume(spatial_volume, t_embed, v_embed, self.poses, self.Ks, target_index, **kwargs)
        else:
            frustum_volume_feats, frustum_volume_depth = self.spatial_volume.construct_view_frustum_volume(spatial_volume, t_embed, v_embed, self.poses, self.Ks, target_index, **kwargs)

        # clip
        TN = target_index.shape[1]
        clip_embed_ = clip_embed.unsqueeze(1).repeat(1,TN,1,1).view(B*TN,clip_embed.shape[1],768)
        
        v_embed_ = v_embed[torch.arange(B)[:,None], target_index].view(B*TN, self.viewpoint_dim) # B*TN,v_dim
        clip_embed_ = self.cc_projection(torch.cat([clip_embed_, v_embed_.unsqueeze(1).repeat(1,clip_embed.shape[1],1)], -1))  # B*TN,1,768

        x_input_ = x_input.unsqueeze(1).repeat(1, TN, 1, 1, 1).view(B * TN, _, H, W)
        x_concat = x_input_
        return clip_embed_, frustum_volume_feats, x_concat

    def training_step(self, batch):

        B = batch['target_image'].shape[0]
        
        self.poses = batch['target_pose'].to(torch.float32)
        self.Ks = batch['target_K'][:,None].repeat(1,self.view_num,1,1).to(torch.float32)
        
        time_steps = torch.randint(0, self.num_timesteps, (B,), device=self.device).long()

        x, clip, input_info = self.prepare(batch)
        x_noisy, noise = self.add_noise(x, time_steps)  # B,N,4,H,W

        N = self.view_num
        target_index = torch.randint(0, N, (B, 1), device=self.device).long() # B, 1
        v_embed = self.get_viewpoint_embedding(input_info['dp']) # B,N,v_dim
        t_embed = self.embed_time(time_steps)
        
        volume = self.volume_merge(torch.cat([self.spatial_volume.construct_spatial_volume(x_noisy[:,:N], t_embed, v_embed[:,:N], self.poses, self.Ks),
                                           self.normal_volume.construct_spatial_volume(x_noisy[:,N:], t_embed, v_embed[:,N:], self.poses, self.Ks)], 1), t_embed)
        
            
        clip_embed, volume_feats, x_concat = self.get_target_view_feats(input_info['x'], volume, clip, t_embed, v_embed[:,:N], target_index, normal=False, frustum_volume_feats=self.volume_frustum)
        x_noisy_ = x_noisy[torch.arange(B)[:,None],target_index][:,0] # B,4,H,W
        noise_predict = self.model(x_noisy_, time_steps, clip_embed, volume_feats, x_concat, is_train=True)
        noise_target = noise[torch.arange(B)[:,None],target_index][:,0] # B,4,H,W
            
        clip_embed_, volume_feats_, x_concat_ = self.get_target_view_feats(input_info['x'], volume, clip, t_embed, v_embed[:,N:], target_index, normal=True, frustum_volume_feats=self.volume_frustum)
        n_noisy_ = x_noisy[torch.arange(B)[:,None],target_index+N][:,0] # B,4,H,W
        noise_predict_ = self.model(n_noisy_, time_steps, clip_embed_, volume_feats_, x_concat_, is_train=True)
        noise_target_ = noise[torch.arange(B)[:,None],target_index+N][:,0] # B,4,H,W
            
        rgb_loss = torch.nn.functional.mse_loss(noise_target, noise_predict, reduction='none').mean()
        normal_loss = torch.nn.functional.mse_loss(noise_target_, noise_predict_, reduction='none').mean()
        loss = rgb_loss + normal_loss
            
        self.log('rgb_loss', rgb_loss, prog_bar=True, logger=True, on_step=True, on_epoch=True, rank_zero_only=True)
        self.log('normal_loss', normal_loss, prog_bar=True, logger=True, on_step=True, on_epoch=True, rank_zero_only=True)
        
        self.log('sim', loss, prog_bar=True, logger=True, on_step=True, on_epoch=True, rank_zero_only=True)

        # log others
        lr = self.optimizers().param_groups[0]['lr']
        self.log('lr', lr, prog_bar=True, logger=True, on_step=True, on_epoch=False, rank_zero_only=True)
        self.log("step", self.global_step, prog_bar=True, logger=True, on_step=True, on_epoch=False, rank_zero_only=True)
            
        return loss

    def add_noise(self, x_start, t):
        """
        @param x_start: B,*
        @param t:       B,
        @return:
        """
        B = x_start.shape[0]
        noise = torch.randn_like(x_start) # B,*

        sqrt_alphas_cumprod_  = self.sqrt_alphas_cumprod[t] # B,
        sqrt_one_minus_alphas_cumprod_ = self.sqrt_one_minus_alphas_cumprod[t] # B
        sqrt_alphas_cumprod_ = sqrt_alphas_cumprod_.view(B, *[1 for _ in range(len(x_start.shape)-1)])
        sqrt_one_minus_alphas_cumprod_ = sqrt_one_minus_alphas_cumprod_.view(B, *[1 for _ in range(len(x_start.shape)-1)])
        x_noisy = sqrt_alphas_cumprod_ * x_start + sqrt_one_minus_alphas_cumprod_ * noise
        return x_noisy, noise

    def sample(self, sampler, batch, cfg_scale, batch_view_num, return_inter_results=False, inter_interval=50, inter_view_interval=2):
        _, clip_embed, input_info = self.prepare(batch)
        self.poses = batch['target_pose'].to(torch.float32)
        self.Ks = batch['target_K'][:,None].repeat(1,self.view_num,1,1).to(torch.float32)
        
        x_sample, inter = sampler.sample(input_info, clip_embed, unconditional_scale=cfg_scale, log_every_t=inter_interval, batch_view_num=batch_view_num)

        N = x_sample.shape[1]
        x_sample = torch.stack([self.decode_first_stage(x_sample[:, ni]) for ni in range(N)], 1)
        if return_inter_results:
            torch.cuda.synchronize()
            torch.cuda.empty_cache()
            inter = torch.stack(inter['x_inter'], 2) # # B,N,T,C,H,W
            B,N,T,C,H,W = inter.shape
            inter_results = []
            for ni in tqdm(range(0, N, inter_view_interval)):
                inter_results_ = []
                for ti in range(T):
                    inter_results_.append(self.decode_first_stage(inter[:, ni, ti]))
                inter_results.append(torch.stack(inter_results_, 1)) # B,T,3,H,W
            inter_results = torch.stack(inter_results,1) # B,N,T,3,H,W
            return x_sample, inter_results
        else:
            return x_sample

    def log_image(self,  x_sample, batch, step, output_dir):
        process = lambda x: ((torch.clip(x, min=-1, max=1).cpu().numpy() * 0.5 + 0.5) * 255).astype(np.uint8)
        B = x_sample.shape[0]
        N = x_sample.shape[1]
        image_cond = []
        output_dir = Path(output_dir)

        for bi in range(B):
            img_pr_ = concat_images_list(*[process(batch['input_image'][bi,:,:,i*3:(i+1)*3]) for i in range(batch['input_image'].shape[-1]//3)],*[process(x_sample[bi, ni].permute(1, 2, 0)) for ni in range(N)])
            image_cond.append(img_pr_)

        imsave(str(output_dir/f'{step}.jpg'), concat_images_list(*image_cond, vert=True))

    @torch.no_grad()
    def validation_step(self, batch, batch_idx):
        if batch_idx==0 and self.global_rank==0:
            self.eval()
            step = self.global_step
            batch_ = {}
            for k, v in batch.items(): batch_[k] = v[:self.output_num]
            self.poses = batch_['target_pose'].to(torch.float32)
            self.Ks = batch_['target_K'][:,None].repeat(1,self.view_num,1,1).to(torch.float32)
            x_sample = self.sample(self.sampler, batch_, self.cfg_scale, self.batch_view_num)
            output_dir = Path(self.image_dir) / 'images' / 'val'
            output_dir.mkdir(exist_ok=True, parents=True)
            self.log_image(x_sample, batch, step, output_dir=output_dir)
     
    def save_image(self,  x_sample, batch, output_dir):
        process = lambda x: ((torch.clip(x, min=-1, max=1).cpu().numpy() * 0.5 + 0.5) * 255).astype(np.uint8)
        B = x_sample.shape[0]
        N = x_sample.shape[1]
        image_cond = []
        output_dir = Path(output_dir)
        
        for bi in range(B):
            img_pr_ = concat_images_list(*[process(x_sample[bi, ni].permute(1, 2, 0)) for ni in range(N)])
            name = self.names[batch['name'][bi][0]]
            conds = '_'.join([f'{cond:03d}' for cond in batch['cond'][bi]])
            output_fn = output_dir / f'{name}_cond_{conds}.png'
            imsave(output_fn, img_pr_)
           
    @torch.no_grad()
    def test_step(self, batch, batch_idx):
        self.eval()
        self.poses = batch['target_pose'].to(torch.float32)
        self.Ks = batch['target_K'][:,None].repeat(1,self.view_num,1,1).to(torch.float32)
        x_sample = self.sample(self.sampler, batch, self.cfg_scale, self.batch_view_num)
        output_dir = Path(self.output_dir)
        output_dir.mkdir(exist_ok=True, parents=True)
        self.save_image(x_sample, batch, output_dir=output_dir)

    def configure_optimizers(self):
        lr = self.learning_rate
        print(f'setting learning rate to {lr:.4f} ...')
        paras = []
        if self.finetune_projection:
            paras.append({"params": self.cc_projection.parameters(), "lr": 10*lr},)
        if self.finetune_unet:
            paras.append({"params": self.model.parameters(), "lr": lr},)
        else:
            paras.append({"params": self.model.get_trainable_parameters(), "lr": lr},)
            
        paras.append({"params": self.normal_volume.parameters(), "lr": lr*10.0},)
        paras.append({"params": self.volume_merge.parameters(), "lr": lr*10.0},)
        paras.append({"params": self.volume_frustum.parameters(), "lr": lr*10.0},)
            
        paras.append({"params": self.time_embed.parameters(), "lr": lr*10.0},)
        paras.append({"params": self.spatial_volume.parameters(), "lr": lr*10.0},)
        
        opt = torch.optim.AdamW(paras, lr=lr)

        scheduler = instantiate_from_config(self.scheduler_config)
        print("Setting up LambdaLR scheduler...")
        scheduler = [{'scheduler': LambdaLR(opt, lr_lambda=scheduler.schedule), 'interval': 'step', 'frequency': 1}]
        return [opt], scheduler

class TeethDDIMSampler:
    def __init__(self, model: TeethDreamer, ddim_num_steps, ddim_discretize="uniform", ddim_eta=1.0, latent_size=32):
        super().__init__()
        self.model = model
        self.ddpm_num_timesteps = model.num_timesteps
        self.latent_size = latent_size
        self._make_schedule(ddim_num_steps, ddim_discretize, ddim_eta)
        self.eta = ddim_eta

    def _make_schedule(self,  ddim_num_steps, ddim_discretize="uniform", ddim_eta=0., verbose=True):
        self.ddim_timesteps = make_ddim_timesteps(ddim_discr_method=ddim_discretize, num_ddim_timesteps=ddim_num_steps, num_ddpm_timesteps=self.ddpm_num_timesteps, verbose=verbose) # DT
        ddim_timesteps_ = torch.from_numpy(self.ddim_timesteps.astype(np.int64)) # DT

        alphas_cumprod = self.model.alphas_cumprod # T
        assert alphas_cumprod.shape[0] == self.ddpm_num_timesteps, 'alphas have to be defined for each timestep'
        self.ddim_alphas = alphas_cumprod[ddim_timesteps_].double() # DT
        self.ddim_alphas_prev = torch.cat([alphas_cumprod[0:1], alphas_cumprod[ddim_timesteps_[:-1]]], 0) # DT
        self.ddim_sigmas = ddim_eta * torch.sqrt((1 - self.ddim_alphas_prev) / (1 - self.ddim_alphas) * (1 - self.ddim_alphas / self.ddim_alphas_prev))

        self.ddim_alphas_raw = self.model.alphas[ddim_timesteps_].float() # DT
        self.ddim_sigmas = self.ddim_sigmas.float()
        self.ddim_alphas = self.ddim_alphas.float()
        self.ddim_alphas_prev = self.ddim_alphas_prev.float()
        self.ddim_sqrt_one_minus_alphas = torch.sqrt(1. - self.ddim_alphas).float()


    @torch.no_grad()
    def denoise_apply_impl(self, x_target_noisy, index, noise_pred, is_step0=False):
        """
        @param x_target_noisy: B,N,4,H,W
        @param index:          index
        @param noise_pred:     B,N,4,H,W
        @param is_step0:       bool
        @return:
        """
        device = x_target_noisy.device
        B,N,_,H,W = x_target_noisy.shape

        # apply noise
        a_t = self.ddim_alphas[index].to(device).float().view(1,1,1,1,1)
        a_prev = self.ddim_alphas_prev[index].to(device).float().view(1,1,1,1,1)
        sqrt_one_minus_at = self.ddim_sqrt_one_minus_alphas[index].to(device).float().view(1,1,1,1,1)
        sigma_t = self.ddim_sigmas[index].to(device).float().view(1,1,1,1,1)

        pred_x0 = (x_target_noisy - sqrt_one_minus_at * noise_pred) / a_t.sqrt()
        dir_xt = torch.clamp(1. - a_prev - sigma_t**2, min=1e-7).sqrt() * noise_pred
        x_prev = a_prev.sqrt() * pred_x0 + dir_xt
        if not is_step0:
            noise = sigma_t * torch.randn_like(x_target_noisy)
            x_prev = x_prev + noise
        return x_prev

    @torch.no_grad()
    def denoise_apply(self, x_target_noisy, input_info, clip_embed, time_steps, index, unconditional_scale, batch_view_num=1, is_step0=False):
        """
        @param x_target_noisy:   B,N,4,H,W
        @param input_info:
        @param clip_embed:       B,M,768
        @param time_steps:       B,
        @param index:            int
        @param unconditional_scale:
        @param batch_view_num:   int
        @param is_step0:         bool
        @return:
        """
        x_input, dp = input_info['x'], input_info['dp']
        B, _, C, H, W = x_target_noisy.shape
        N = self.model.view_num

        # construct source data
        v_embed = self.model.get_viewpoint_embedding(dp) # B,N,v_dim
        t_embed = self.model.embed_time(time_steps)  # B,t_dim
        
        volume = self.model.volume_merge(torch.cat([self.model.spatial_volume.construct_spatial_volume(x_target_noisy[:,:N], t_embed, v_embed[:,:N], self.model.poses, self.model.Ks),
                                                self.model.normal_volume.construct_spatial_volume(x_target_noisy[:,N:], t_embed, v_embed[:,N:], self.model.poses, self.model.Ks)], 1), t_embed) 
        
        e_t = []
        target_indices = torch.arange(N) # N
        
        #predict color
        for ni in range(0, N, batch_view_num):
            x_target_noisy_ = x_target_noisy[:, ni:ni + batch_view_num]
            VN = x_target_noisy_.shape[1]
            x_target_noisy_ = x_target_noisy_.reshape(B*VN,C,H,W)

            time_steps_ = repeat_to_batch(time_steps, B, VN)
            target_indices_ = target_indices[ni:ni+batch_view_num].unsqueeze(0).repeat(B,1)
            clip_embed_, volume_feats_, x_concat_ = self.model.get_target_view_feats(x_input, volume, clip_embed, t_embed, v_embed[:,:N], target_indices_, normal=False, frustum_volume_feats=self.model.volume_frustum)
            
            if unconditional_scale!=1.0:
                noise = self.model.model.predict_with_unconditional_scale(x_target_noisy_, time_steps_, clip_embed_, volume_feats_, x_concat_, unconditional_scale)
            else:
                noise = self.model.model(x_target_noisy_, time_steps_, clip_embed_, volume_feats_, x_concat_, is_train=False)
            
            e_t.append(noise.view(B,VN,4,H,W))
        
        #predict normal
        for ni in range(0, N, batch_view_num):
            x_target_noisy_ = x_target_noisy[:, N + ni:N + ni + batch_view_num]
            VN = x_target_noisy_.shape[1]
            x_target_noisy_ = x_target_noisy_.reshape(B*VN,C,H,W)

            time_steps_ = repeat_to_batch(time_steps, B, VN)
            target_indices_ = target_indices[ni:ni+batch_view_num].unsqueeze(0).repeat(B,1)
            clip_embed_, volume_feats_, x_concat_ = self.model.get_target_view_feats(x_input, volume, clip_embed, t_embed, v_embed[:,N:], target_indices_, normal=True, frustum_volume_feats=self.model.volume_frustum)

            if unconditional_scale!=1.0:
                noise = self.model.model.predict_with_unconditional_scale(x_target_noisy_, time_steps_, clip_embed_, volume_feats_, x_concat_, unconditional_scale)
            else:
                noise = self.model.model(x_target_noisy_, time_steps_, clip_embed_, volume_feats_, x_concat_, is_train=False)
            
            e_t.append(noise.view(B,VN,4,H,W))

        e_t = torch.cat(e_t, 1)
        x_prev = self.denoise_apply_impl(x_target_noisy, index, e_t, is_step0)
        return x_prev

    @torch.no_grad()
    def sample(self, input_info, clip_embed, unconditional_scale=1.0, log_every_t=50, batch_view_num=1):
        """
        @param input_info:      x, elevation
        @param clip_embed:      B,M,768
        @param unconditional_scale:
        @param log_every_t:
        @param batch_view_num:
        @return:
        """
        print(f"unconditional scale {unconditional_scale:.1f}")
        C, H, W = 4, self.latent_size, self.latent_size
        B = clip_embed.shape[0]
        N = self.model.view_num
        device = self.model.device
        x_target_noisy = torch.randn([B, 2*N, C, H, W], device=device)

        timesteps = self.ddim_timesteps
        intermediates = {'x_inter': []}
        time_range = np.flip(timesteps)
        total_steps = timesteps.shape[0]

        iterator = tqdm(time_range, desc='DDIM Sampler', total=total_steps)
        for i, step in enumerate(iterator):
            index = total_steps - i - 1 # index in ddim state
            time_steps = torch.full((B,), step, device=device, dtype=torch.long)
            x_target_noisy = self.denoise_apply(x_target_noisy, input_info, clip_embed, time_steps, index, unconditional_scale, batch_view_num=batch_view_num, is_step0=index==0)
            if index % log_every_t == 0 or index == total_steps - 1:
                intermediates['x_inter'].append(x_target_noisy)

        return x_target_noisy, intermediates

```

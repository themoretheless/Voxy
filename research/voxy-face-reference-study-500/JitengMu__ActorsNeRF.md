# Facial rendering source scan: JitengMu/ActorsNeRF

Repository: https://github.com/JitengMu/ActorsNeRF

Pinned source revision: e09f2dfc182015934354c7ee839ede5144b1f05c

Discovery topic: human rendering

Description: ActorsNeRF: Animatable Few-shot Human Rendering with Generalizable NeRFs (ICCV 2023)

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "third_parties/lpips/LICENSE", "third_parties/yacs/LICENSE", "third_parties/yacs/NOTICE"]

Topic paths: {"deform": ["core/nets/actorsnerf/deformation_network/deformation_network.py"]}

## Source file: core/nets/actorsnerf/deformation_network/deformation_network.py

{
  "path": "core/nets/actorsnerf/deformation_network/deformation_network.py",
  "blob_sha": "d52566f3d882f473352379297617c0279cd76ddc",
  "sha256": "38feae284ad5783db6fd7b129981cf1eb03f84e52f3dbdc1e202832cfee23d27",
  "lines": 75,
  "bytes": 2519,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/JitengMu/ActorsNeRF/blob/e09f2dfc182015934354c7ee839ede5144b1f05c/core/nets/actorsnerf/deformation_network/deformation_network.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self,"
    },
    {
      "line": 53,
      "text": "def forward(self, pos_embed, pos_xyz, condition_code, local_feat, viewdirs=None, **_):"
    }
  ]
}

```text

import torch
import torch.nn as nn

from core.utils.network_util import initseq
from configs import cfg


class NonRigidMotionMLP(nn.Module):
    def __init__(self,
                 pos_embed_size=3, 
                 condition_code_size=69,
                 mlp_width=128,
                 mlp_depth=6,
                 skips=None):
        super(NonRigidMotionMLP, self).__init__()

        # self.latents = nn.Parameter(
        #     torch.randn(cfg.num_sub, 512), requires_grad=True 
        # )

        self.pixel_feat_fc = nn.Linear(256, 64)
        self.skips = [4] if skips is None else skips
        
        if cfg.use_enc:
            if cfg.use_data_cross_pose or cfg.use_data_cross_view:
                block_mlps = [nn.Linear(pos_embed_size+condition_code_size+192, mlp_width), nn.ReLU()]
 
        layers_to_cat_inputs = []
        for i in range(1, mlp_depth):
            if i in self.skips:
                layers_to_cat_inputs.append(len(block_mlps))
                if cfg.use_enc:
                    if cfg.use_data_cross_pose or cfg.use_data_cross_view:
                        block_mlps += [nn.Linear(mlp_width + pos_embed_size + condition_code_size + 192, mlp_width), nn.ReLU()]
            else:
                block_mlps += [nn.Linear(mlp_width, mlp_width), nn.ReLU()]

        block_mlps += [nn.Linear(mlp_width, 3)]

        self.block_mlps = nn.ModuleList(block_mlps)
        initseq(self.block_mlps)

        self.layers_to_cat_inputs = layers_to_cat_inputs

        # init the weights of the last layer as very small value
        # -- at the beginning, we hope non-rigid offsets are zeros 
        init_val = 1e-5
        last_layer = self.block_mlps[-1]
        last_layer.weight.data.uniform_(-init_val, init_val)
        last_layer.bias.data.zero_()


    def forward(self, pos_embed, pos_xyz, condition_code, local_feat, viewdirs=None, **_):

        B = pos_embed.shape[0]

        latent = self.pixel_feat_fc(local_feat)
        latent = latent.view(latent.shape[0], -1)

        h = torch.cat([pos_embed, condition_code, latent], dim=-1)
        if viewdirs is not None:
            h = torch.cat([h, viewdirs], dim=-1)

        for i in range(len(self.block_mlps)):
            if i in self.layers_to_cat_inputs:
                h = torch.cat([h, pos_embed, condition_code, latent], dim=-1)
            h = self.block_mlps[i](h)
        trans = h

        result = {
            'xyz': pos_xyz + trans,
            'offsets': trans
        }
        
        return result


```

## Source file: eval.py

{
  "path": "eval.py",
  "blob_sha": "b4d786a439f185bf5ef8926daf945cc45e8b6583",
  "sha256": "bbf79b46234b4d5e7d3a3f1bde28cf338588fb03ddcf0e4c30e7e77116588c6e",
  "lines": 241,
  "bytes": 7906,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2,
    "lip": 2
  },
  "url": "https://github.com/JitengMu/ActorsNeRF/blob/e09f2dfc182015934354c7ee839ede5144b1f05c/eval.py",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "def load_network():"
    },
    {
      "line": 36,
      "text": "def unpack_alpha_map(alpha_vals, ray_mask, width, height):"
    },
    {
      "line": 42,
      "text": "def unpack_to_image(width, height, ray_mask, bgcolor,"
    },
    {
      "line": 71,
      "text": "def _eval("
    },
    {
      "line": 153,
      "text": "def _eval_freeview("
    },
    {
      "line": 222,
      "text": "def run_eval_novel_view():"
    },
    {
      "line": 228,
      "text": "def run_tpose():"
    },
    {
      "line": 234,
      "text": "def run_eval_freeview():"
    }
  ]
}

```text

import os

import torch
import numpy as np
from tqdm import tqdm

from core.data import create_dataloader
from core.nets import create_network
from core.utils.train_util import cpu_data_to_gpu
from core.utils.image_util import ImageWriter_Category, to_8b_image, to_8b3ch_image

from configs import cfg, args
from core.evaluator import Evaluator

from moviepy.editor import *
# import kornia
import cv2

EXCLUDE_KEYS_TO_GPU = ['frame_name', 'sub_idx', 'cam_idx',
                       'img_width', 'img_height', 'ray_mask']

MAKE_VIDEO = False

def load_network():
    model = create_network()
    ckpt_path = os.path.join(cfg.logdir, f'{cfg.load_net}.tar')
    ckpt = torch.load(ckpt_path, map_location='cuda:0')
    for m1, _ in model.named_parameters():
        if m1 not in ckpt['network'].keys():
            raise Exception("model not match")
    model.load_state_dict(ckpt['network'], strict=False)
    print('load network from ', ckpt_path)
    return model.cuda().deploy_mlps_to_secondary_gpus()


def unpack_alpha_map(alpha_vals, ray_mask, width, height):
    alpha_map = np.zeros((height * width), dtype='float32')
    alpha_map[ray_mask] = alpha_vals
    return alpha_map.reshape((height, width))


def unpack_to_image(width, height, ray_mask, bgcolor,
                    rgb, alpha, truth=None):
    
    rgb_image = np.full((height * width, 3), bgcolor, dtype='float32')
    truth_image = np.full((height * width, 3), bgcolor, dtype='float32')

    rgb_image[ray_mask] = rgb
    rgb_image = to_8b_image(rgb_image.reshape((height, width, 3)))

    if truth is not None:
        truth_image[ray_mask] = truth
        truth_image = to_8b_image(truth_image.reshape((height, width, 3)))

    alpha_map = unpack_alpha_map(alpha, ray_mask, width, height)
    alpha_image  = to_8b3ch_image(alpha_map)

    return rgb_image, alpha_image, truth_image


# def img_process(img, mask, bg_color):
#     mask = mask / 255.
#     img = mask * img + (1.0 - mask) * bg_color[None, None, :]
#     img = torch.tensor(img[None, ...]).permute(0, 3, 1, 2)
#     if cfg.resize_img_scale != 1.:
#         H, W = int(img.shape[-2]*cfg.resize_img_scale), int(img.shape[-1]*cfg.resize_img_scale)
#         img = kornia.geometry.transform.resize(img, (H, W), interpolation='bilinear')
#     img = img / 255.0
#     return img.permute(0, 2, 3, 1).contiguous()[0].float()

def _eval(
        data_type=None,
        dataset_mode='ins_level',
        folder_name=None):
    cfg.perturb = 0.

    model = load_network()
    test_loader = create_dataloader(data_type, dataset_mode)

    writer = ImageWriter_Category(
                output_dir=os.path.join(cfg.logdir, cfg.load_net),
                exp_name=folder_name)

    # if data_type in ['eval_novel_view', 'eval_novel_pose']:
    if data_type in ['eval_novel_view']:
        evaluator = Evaluator()
    model.eval()
    for batch in tqdm(test_loader):
        batch = batch[0]
        for k, v in batch.items():
            if k=="data_enc" or k=='txyz_dic' or k=='pxyz_dic' or k=='dst_bbox':
                batch[k] = v
            else:
                batch[k] = v[0]

        data = cpu_data_to_gpu(
                    batch,
                    exclude_keys=EXCLUDE_KEYS_TO_GPU)

        data['iter_val'] = torch.full((1,), 1000000).cuda()
        with torch.no_grad():
            net_output = model(**data)

        rgb = net_output['rgb']
        alpha = net_output['alpha']

        width = batch['img_width']
        height = batch['img_height']
        ray_mask = batch['ray_mask']
        target_rgbs = batch.get('target_rgbs', None)

        rgb_img, alpha_img, _ = unpack_to_image(
            width, height, ray_mask, np.array(cfg.bgcolor) / 255.,
            rgb.data.cpu().numpy(),
            alpha.data.cpu().numpy())

        # # define the alpha and beta
        # alpha = 1.2 # Contrast control
        # beta = 10 # Brightness control
        
        # call convertScaleAbs function
        # rgb_img = cv2.convertScaleAbs(rgb_img, alpha=alpha, beta=beta)

        imgs = [rgb_img]
        if cfg.show_truth and target_rgbs is not None:
            target_rgbs = to_8b_image(target_rgbs.numpy())
            imgs.append(target_rgbs)
        if cfg.show_alpha:
            imgs.append(alpha_img)

        img_out = np.concatenate(imgs, axis=1)
        writer.append(batch, img_out)

        # if data_type in ['eval_novel_view', 'eval_novel_pose']:
        if data_type in ['eval_novel_view']:
            H, W, _ = batch['target_rgbs'].shape
            mask = ray_mask.detach().cpu().numpy().reshape(H, W)
            if cfg.task=='zju_mocap':
                eval_pred_rgb = np.zeros((H, W, 3))
            if cfg.task=='AIST_mocap':
                eval_pred_rgb = np.ones((H, W, 3))
            eval_pred_rgb[mask>0] = rgb.detach().cpu().numpy()
            eval_gt_rgb = batch['target_rgbs'].detach().cpu().numpy()
            evaluator.evaluate(eval_pred_rgb, eval_gt_rgb)

    # if data_type in ['eval_novel_view', 'eval_novel_pose']:
    if data_type in ['eval_novel_view']:
        evaluator.summarize()

    writer.finalize()


def _eval_freeview(
        data_type=None,
        dataset_mode='ins_level',
        folder_name=None):
    cfg.perturb = 0.

    model = load_network()
    test_loader = create_dataloader(data_type, dataset_mode)

    writer = ImageWriter_Category(
                output_dir=os.path.join(cfg.logdir, cfg.load_net),
                exp_name=folder_name+'_'+test_loader.dataset.target_person, MAKE_VIDEO=MAKE_VIDEO)

    model.eval()
    imgs = []
    for idx, batch in tqdm(enumerate(test_loader)):
        batch = batch[0]
        for k, v in batch.items():
            if k=="data_enc" or k=='txyz_dic' or k=='pxyz_dic' or k=='dst_bbox':
                batch[k] = v
            else:
                batch[k] = v[0]

        data = cpu_data_to_gpu(
                    batch,
                    exclude_keys=EXCLUDE_KEYS_TO_GPU)

        data['iter_val'] = torch.full((1,), 1000000).cuda()
        with torch.no_grad():
            net_output = model(**data)

        rgb = net_output['rgb']
        alpha = net_output['alpha']

        width = batch['img_width']
        height = batch['img_height']
        ray_mask = batch['ray_mask']
        target_rgbs = batch.get('target_rgbs', None)

        rgb_img, alpha_img, _ = unpack_to_image(
            width, height, ray_mask, np.array(cfg.bgcolor) / 255.,
            rgb.data.cpu().numpy(),
            alpha.data.cpu().numpy())

        # define the alpha and beta
        alpha = 1.2 # Contrast control
        beta = 10 # Brightness control
        
        # call convertScaleAbs function
        rgb_img = cv2.convertScaleAbs(rgb_img, alpha=alpha, beta=beta)
        #if cfg.task=='AIST_mocap':
        #    rgb_img = rgb_img[100:500, 240:640]

        if MAKE_VIDEO:
            writer.append(batch, rgb_img, MAKE_VIDEO, idx)
        else:
            writer.append(batch, rgb_img)
        imgs.append(rgb_img)
        img_out = np.stack(imgs, axis=0)

    writer.finalize()
    save_path = os.path.join(cfg.logdir, cfg.load_net, folder_name+'_'+test_loader.dataset.target_person)
    import torchvision
    torchvision.io.write_video(save_path+'/result.mp4', torch.tensor(img_out), fps=10, video_codec='h264', options={'crf': '10'})
    videoClip  = VideoFileClip(save_path+'/result.mp4')
    videoClip.write_gif(save_path+'/result.gif')



def run_eval_novel_view():
    _eval(
        data_type='eval_novel_view',
        dataset_mode='cat_level',
        folder_name=f"eval_novel_view")

def run_tpose():
    _eval(
        data_type='tpose',
        dataset_mode='cat_level',
        folder_name=f"tpose")

def run_eval_freeview():
    _eval_freeview(
        data_type='eval_freeview',
        dataset_mode='cat_level',
        folder_name=f"eval_freeview")
        
if __name__ == '__main__':
    globals()[f'run_{args.type}']()


```

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "985ffe11aea4ddada82f836feff06bf32270e6c5",
  "sha256": "5f11baa56bde60f7976f1916a9975a0a7ef5bfbf3e8757279265c2226e5a1cc1",
  "lines": 33,
  "bytes": 813,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/JitengMu/ActorsNeRF/blob/e09f2dfc182015934354c7ee839ede5144b1f05c/train.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def main():"
    }
  ]
}

```text

from configs import cfg

from core.utils.log_util import Logger
from core.data import create_dataloader
from core.nets import create_network
from core.train import create_trainer, create_optimizer


def main():
    log = Logger()
    log.print_config()

    model = create_network()
    optimizer = create_optimizer(model)
    trainer = create_trainer(model, optimizer)

    train_loader = create_dataloader(data_type='train', dataset_mode='cat_level')

    # estimate start epoch
    epoch = trainer.iter // len(train_loader) + 1
    while True:
        if trainer.iter > cfg.train.maxiter:
            break
        
        trainer.train(epoch=epoch,
                      train_dataloader=train_loader,
                      )
        epoch += 1

    trainer.finalize()

if __name__ == '__main__':
    main()


```

## Source file: core/__init__.py

{
  "path": "core/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/JitengMu/ActorsNeRF/blob/e09f2dfc182015934354c7ee839ede5144b1f05c/core/__init__.py",
  "function_signature_candidates": []
}

```text



```

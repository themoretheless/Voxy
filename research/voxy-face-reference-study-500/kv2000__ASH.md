# Facial rendering source scan: kv2000/ASH

Repository: https://github.com/kv2000/ASH

Pinned source revision: 0aea45c5f9646f32fde2eb69b05ab31aa044462e

Discovery topic: human rendering

Description: The Training and Demo code for: Ash: Animatable gaussian splats for efficient and photoreal human rendering (CVPR 2024)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"gum": ["arguments/__init__.py"]}

## Source file: arguments/__init__.py

{
  "path": "arguments/__init__.py",
  "blob_sha": "1169fd444a738859d1c25c465560b3b6918354d5",
  "sha256": "a9ecf59947b4ed534213f285bf0f466f890d203a0985c6235002b0e482a0c2b3",
  "lines": 120,
  "bytes": 4035,
  "selection_score": 3,
  "keyword_line_counts": {
    "gum": 8,
    "rig": 2
  },
  "url": "https://github.com/kv2000/ASH/blob/0aea45c5f9646f32fde2eb69b05ab31aa044462e/arguments/__init__.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def __init__(self, parser: ArgumentParser, name : str, fill_none = False):"
    },
    {
      "line": 40,
      "text": "def extract(self, args):"
    },
    {
      "line": 49,
      "text": "def __init__(self, parser, sentinel=False):"
    },
    {
      "line": 60,
      "text": "def extract(self, args):"
    },
    {
      "line": 66,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 74,
      "text": "def __init__(self):"
    },
    {
      "line": 79,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 100,
      "text": "def get_combined_args(parser : ArgumentParser):"
    }
  ]
}

```text

#
# Copyright (C) 2023, Inria
# GRAPHDECO research group, https://team.inria.fr/graphdeco
# All rights reserved.
#
# This software is free for non-commercial, research and evaluation use 
# under the terms of the LICENSE.md file.
#
# For inquiries contact  george.drettakis@inria.fr
#

from argparse import ArgumentParser, Namespace
import sys
import os

class GroupParams:
    pass

class ParamGroup:
    def __init__(self, parser: ArgumentParser, name : str, fill_none = False):
        group = parser.add_argument_group(name)
        for key, value in vars(self).items():
            shorthand = False
            if key.startswith("_"):
                shorthand = True
                key = key[1:]
            t = type(value)
            value = value if not fill_none else None 
            if shorthand:
                if t == bool:
                    group.add_argument("--" + key, ("-" + key[0:1]), default=value, action="store_true")
                else:
                    group.add_argument("--" + key, ("-" + key[0:1]), default=value, type=t)
            else:
                if t == bool:
                    group.add_argument("--" + key, default=value, action="store_true")
                else:
                    group.add_argument("--" + key, default=value, type=t)

    def extract(self, args):
        group = GroupParams()
        for arg in vars(args).items():
            if arg[0] in vars(self) or ("_" + arg[0]) in vars(self):
                setattr(group, arg[0], arg[1])
        return group


class ModelParams(ParamGroup): 
    def __init__(self, parser, sentinel=False):
        self.sh_degree = 3
        self._source_path = ""
        self._model_path = ""
        self._images = "images"
        self._resolution = -1
        self._white_background = False
        self.data_device = "cuda"
        self.eval = False
        super().__init__(parser, "Loading Parameters", sentinel)

    def extract(self, args):
        g = super().extract(args)
        g.source_path = os.path.abspath(g.source_path)
        return g

class PipelineParams(ParamGroup):
    def __init__(self, parser):
        self.convert_SHs_python = False
        self.compute_cov3D_python = False
        self.debug = False
        super().__init__(parser, "Pipeline Parameters")

# a random mocked/hacked pipline which is already enough for my code base :D 
class Mock_PipelineParams:
    def __init__(self):
        self.convert_SHs_python = False
        self.compute_cov3D_python = False

class OptimizationParams(ParamGroup):
    def __init__(self, parser):
        self.iterations = 30_000
        self.position_lr_init = 0.00016
        self.position_lr_final = 0.0000016
        self.position_lr_delay_mult = 0.01
        self.position_lr_max_steps = 30_000
        self.feature_lr = 0.0025
        self.opacity_lr = 0.05
        self.scaling_lr = 0.005
        self.rotation_lr = 0.001
        self.percent_dense = 0.01
        self.lambda_dssim = 0.2
        self.densification_interval = 100
        self.opacity_reset_interval = 3000
        self.densify_from_iter = 500
        self.densify_until_iter = 15_000
        self.densify_grad_threshold = 0.0002
        self.random_background = False
        super().__init__(parser, "Optimization Parameters")


def get_combined_args(parser : ArgumentParser):
    cmdlne_string = sys.argv[1:]
    cfgfile_string = "Namespace()"
    args_cmdline = parser.parse_args(cmdlne_string)

    try:
        cfgfilepath = os.path.join(args_cmdline.model_path, "cfg_args")
        print("Looking for config file in", cfgfilepath)
        with open(cfgfilepath) as cfg_file:
            print("Config file found: {}".format(cfgfilepath))
            cfgfile_string = cfg_file.read()
    except TypeError:
        print("Config file not found at")
        pass
    args_cfgfile = eval(cfgfile_string)

    merged_dict = vars(args_cfgfile).copy()
    for k,v in vars(args_cmdline).items():
        if v != None:
            merged_dict[k] = v
    return Namespace(**merged_dict)


```

## Source file: scene/unet.py

{
  "path": "scene/unet.py",
  "blob_sha": "9f929a628003fe1c9807a30396be7d28152d7b73",
  "sha256": "1e9946ecdf4e6bafde91cc6064791ef250d5c79df571e3ab32a790721bf82c04",
  "lines": 108,
  "bytes": 3959,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kv2000/ASH/blob/0aea45c5f9646f32fde2eb69b05ab31aa044462e/scene/unet.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self, n_channels, n_classes, bilinear=False):"
    },
    {
      "line": 25,
      "text": "def forward(self, x):     # x (bs, c, h, w)"
    },
    {
      "line": 42,
      "text": "def __init__(self, in_channels, out_channels, mid_channels=None):"
    },
    {
      "line": 55,
      "text": "def forward(self, x):"
    },
    {
      "line": 62,
      "text": "def __init__(self, in_channels, out_channels):"
    },
    {
      "line": 69,
      "text": "def forward(self, x):"
    },
    {
      "line": 76,
      "text": "def __init__(self, in_channels, out_channels, bilinear=True):"
    },
    {
      "line": 87,
      "text": "def forward(self, x1, x2):"
    },
    {
      "line": 103,
      "text": "def __init__(self, in_channels, out_channels):"
    },
    {
      "line": 107,
      "text": "def forward(self, x):"
    }
  ]
}

```text

# Source: https://github.com/milesial/Pytorch-UNet/blob/master/unet/unet_parts.py
import torch
import torch.nn as nn
import torch.nn.functional as F

class UNet(nn.Module):
    def __init__(self, n_channels, n_classes, bilinear=False):
        super(UNet, self).__init__()
        self.n_channels = n_channels
        self.n_classes = n_classes
        self.bilinear = bilinear

        self.inc = (DoubleConv(n_channels, 64))
        self.down1 = (Down(64, 128))
        self.down2 = (Down(128, 256))
        self.down3 = (Down(256, 512))
        factor = 2 if bilinear else 1
        self.down4 = (Down(512, 1024 // factor))
        self.up1 = (Up(1024, 512 // factor, bilinear))
        self.up2 = (Up(512, 256 // factor, bilinear))
        self.up3 = (Up(256, 128 // factor, bilinear))
        self.up4 = (Up(128, 64, bilinear))
        self.outc = (OutConv(64, n_classes))

    def forward(self, x):     # x (bs, c, h, w)
        x1 = self.inc(x)      # x (bs, 64, h, w)
        x2 = self.down1(x1)   # x (bs, 128, h/2, w/2)
        x3 = self.down2(x2)   # x (bs, 256, h/4, w/4)
        x4 = self.down3(x3)   # x (bs, 512, h/8, w/8)
        x5 = self.down4(x4)   # x (bs, 1024, h/16, w/16)
        x = self.up1(x5, x4)  # x (bs, 512, h/8, w/8)
        x = self.up2(x, x3)   # x (bs, 256, h/4, w/4)
        x = self.up3(x, x2)   # x (bs, 128, h/2, w/2)
        x = self.up4(x, x1)   # x (bs, 64, h, w)
        logits = self.outc(x)
        return logits


class DoubleConv(nn.Module):
    """(convolution => [BN] => ReLU) * 2"""

    def __init__(self, in_channels, out_channels, mid_channels=None):
        super().__init__()
        if not mid_channels:
            mid_channels = out_channels
        self.double_conv = nn.Sequential(
            nn.Conv2d(in_channels, mid_channels, kernel_size=3, padding=1, bias=False),
            nn.BatchNorm2d(mid_channels),
            nn.ReLU(inplace=True),
            nn.Conv2d(mid_channels, out_channels, kernel_size=3, padding=1, bias=False),
            nn.BatchNorm2d(out_channels),
            nn.ReLU(inplace=True)
        )

    def forward(self, x):
        return self.double_conv(x)


class Down(nn.Module):
    """Downscaling with maxpool then double conv"""

    def __init__(self, in_channels, out_channels):
        super().__init__()
        self.maxpool_conv = nn.Sequential(
            nn.MaxPool2d(2),
            DoubleConv(in_channels, out_channels)
        )

    def forward(self, x):
        return self.maxpool_conv(x)


class Up(nn.Module):
    """Upscaling then double conv"""

    def __init__(self, in_channels, out_channels, bilinear=True):
        super().__init__()

        # if bilinear, use the normal convolutions to reduce the number of channels
        if bilinear:
            self.up = nn.Upsample(scale_factor=2, mode='bilinear', align_corners=True)
            self.conv = DoubleConv(in_channels, out_channels, in_channels // 2)
        else:
            self.up = nn.ConvTranspose2d(in_channels, in_channels // 2, kernel_size=2, stride=2)
            self.conv = DoubleConv(in_channels, out_channels)

    def forward(self, x1, x2):
        x1 = self.up(x1)
        # input is CHW
        diffY = x2.size()[2] - x1.size()[2]
        diffX = x2.size()[3] - x1.size()[3]

        x1 = F.pad(x1, [diffX // 2, diffX - diffX // 2,
                        diffY // 2, diffY - diffY // 2])
        # if you have padding issues, see
        # https://github.com/HaiyongJiang/U-Net-Pytorch-Unstructured-Buggy/commit/0e854509c2cea854e247a9c615f175f76fbb2e3a
        # https://github.com/xiaopeng-liao/Pytorch-UNet/commit/8ebac70e633bac59fc22bb5195e513d5832fb3bd
        x = torch.cat([x2, x1], dim=1)
        return self.conv(x)


class OutConv(nn.Module):
    def __init__(self, in_channels, out_channels):
        super(OutConv, self).__init__()
        self.conv = nn.Conv2d(in_channels, out_channels, kernel_size=1)

    def forward(self, x):
        return self.conv(x)


```

## Source file: models/utils.py

{
  "path": "models/utils.py",
  "blob_sha": "ae5b707078f9387b63d4cc5964b60dc426c96de9",
  "sha256": "52e78af821f8e833508f3c20e214585e7072d2fa3e2328cb3d26d7ea94868cab",
  "lines": 49,
  "bytes": 2260,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 15,
    "lip": 4
  },
  "url": "https://github.com/kv2000/ASH/blob/0aea45c5f9646f32fde2eb69b05ab31aa044462e/models/utils.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def gen_uv_barycentric(face_idx, face_texture_coords, resolution=512):"
    }
  ]
}

```text

"""
@File: utils.py
@Author: Heming Zhu
@Email: hezhu@mpi-inf.mpg.de
@Date: 2024-3-1
@Desc: Trimmed version for the dataset utils.
"""

import numpy as np

def gen_uv_barycentric(face_idx, face_texture_coords, resolution=512):
    uv_face_id = np.ones(shape=(resolution,resolution)) * (-1.0)
    uv_bary_weights = np.zeros(shape=(resolution,resolution,3))

    for i in range(face_idx.shape[0]):
        cur_face_uv_coords = face_texture_coords[i]
        uu_min = np.clip(np.min(cur_face_uv_coords[:,0]) * resolution - 2, 0, resolution - 1)
        uu_max = np.clip(np.max(cur_face_uv_coords[:,0]) * resolution + 2, 0, resolution - 1)
        vv_min = np.clip(np.min(cur_face_uv_coords[:,1]) * resolution - 2, 0, resolution - 1)
        vv_max = np.clip(np.max(cur_face_uv_coords[:,1]) * resolution + 2, 0, resolution - 1)
        uu_min, uu_max, vv_min, vv_max = int(uu_min), int(uu_max), int(vv_min), int(vv_max)

        for xx in range(uu_min, uu_max + 1):
            for yy in range(vv_min, vv_max + 1):
                fin_x, fin_y = xx, yy
                if uv_face_id[fin_x,fin_y] == -1:
                    px, py = (xx)/resolution, (yy)/resolution
                    
                    p0x, p0y = cur_face_uv_coords[0, 0], cur_face_uv_coords[0, 1]
                    p1x, p1y = cur_face_uv_coords[1, 0], cur_face_uv_coords[1, 1]
                    p2x, p2y = cur_face_uv_coords[2, 0], cur_face_uv_coords[2, 1]
                    
                    signed_area = 0.5 * (-p1y * p2x + p0y * (-p1x + p2x) + p0x * (p1y - p2y) + p1x * p2y)

                    w_1 = 1 / (2 * signed_area) * (p0y * p2x - p0x * p2y + (p2y - p0y) * px + (p0x - p2x) * py)
                    w_2 = 1 / (2 * signed_area) * (p0x * p1y - p0y * p1x + (p0y - p1y) * px + (p1x - p0x) * py)
                    w_0 = 1 - w_1 - w_2

                    if (w_0 >= 0) and (w_1 >= 0) and (w_2 >= 0):
                        uv_face_id[fin_x, fin_y] = i
                        uv_bary_weights[fin_x, fin_y, 0] = w_0
                        uv_bary_weights[fin_x, fin_y, 1] = w_1
                        uv_bary_weights[fin_x, fin_y, 2] = w_2
                else:
                    pass

    uv_face_id = uv_face_id.astype(np.int32)

    return uv_face_id, uv_bary_weights

```

## Source file: scene/unet_2.py

{
  "path": "scene/unet_2.py",
  "blob_sha": "70f79758a541d38979485ad6459a6c34e16b9803",
  "sha256": "f68986c667e1617edce97e1ae0e2bcd806c71b097fb2f7203d6f18ddb980396d",
  "lines": 241,
  "bytes": 7948,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kv2000/ASH/blob/0aea45c5f9646f32fde2eb69b05ab31aa044462e/scene/unet_2.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, **kwargs):"
    },
    {
      "line": 14,
      "text": "def create_embedding_fn(self):"
    },
    {
      "line": 38,
      "text": "def embed(self, inputs):"
    },
    {
      "line": 42,
      "text": "def get_embedder(multires, input_dims=3):"
    },
    {
      "line": 54,
      "text": "def embed(x, eo=embedder_obj): return eo.embed(x)"
    },
    {
      "line": 64,
      "text": "def __init__(self, input_dim):"
    },
    {
      "line": 74,
      "text": "def forward(self, x):"
    },
    {
      "line": 80,
      "text": "def __init__(self, n_channels, n_classes, num_frequency, bilinear=False):"
    },
    {
      "line": 104,
      "text": "def forward(self, x, x_global):     # x (bs, c, h, w)  x_global (bs, 3)"
    },
    {
      "line": 127,
      "text": "def __init__(self, in_channels, out_channels, mid_channels=None):"
    },
    {
      "line": 140,
      "text": "def forward(self, x):"
    },
    {
      "line": 147,
      "text": "def __init__(self, in_channels, out_channels):"
    },
    {
      "line": 154,
      "text": "def forward(self, x):"
    },
    {
      "line": 161,
      "text": "def __init__(self, in_channels, out_channels, bilinear=True):"
    },
    {
      "line": 172,
      "text": "def forward(self, x1, x2):"
    },
    {
      "line": 191,
      "text": "def __init__(self, in_channels, mid_channels, out_channels, bilinear=True):"
    },
    {
      "line": 202,
      "text": "def forward(self, x1, x2):"
    },
    {
      "line": 219,
      "text": "def __init__(self, in_channels, out_channels):"
    },
    {
      "line": 223,
      "text": "def forward(self, x):"
    }
  ]
}

```text

# Source: https://github.com/milesial/Pytorch-UNet/blob/master/unet/unet_parts.py
import torch
import torch.nn as nn
import torch.nn.functional as F


# Positional encoding embedding. Code was taken from https://github.com/bmild/nerf.
class Embedder:

    def __init__(self, **kwargs):
        self.kwargs = kwargs
        self.create_embedding_fn()

    def create_embedding_fn(self):
        embed_fns = []
        d = self.kwargs['input_dims']  # 3
        out_dim = 0
        if self.kwargs['include_input']:
            embed_fns.append(lambda x: x)
            out_dim += d

        max_freq = self.kwargs['max_freq_log2']
        N_freqs = self.kwargs['num_freqs']

        if self.kwargs['log_sampling']:
            freq_bands = 2. ** torch.linspace(0., max_freq, N_freqs)
        else:
            freq_bands = torch.linspace(2. ** 0., 2. ** max_freq, N_freqs)

        for freq in freq_bands:
            for p_fn in self.kwargs['periodic_fns']:
                embed_fns.append(lambda x, p_fn=p_fn, freq=freq: p_fn(x * freq))
                out_dim += d

        self.embed_fns = embed_fns
        self.out_dim = out_dim

    def embed(self, inputs):
        return torch.cat([fn(inputs) for fn in self.embed_fns], -1)


def get_embedder(multires, input_dims=3):
    embed_kwargs = {
        'include_input': True,
        'input_dims': input_dims,
        'max_freq_log2': multires - 1,
        'num_freqs': multires,
        'log_sampling': True,
        'periodic_fns': [torch.sin, torch.cos],
    }

    embedder_obj = Embedder(**embed_kwargs)

    def embed(x, eo=embedder_obj): return eo.embed(x)

    return embed, embedder_obj.out_dim


class MLP(nn.Module):
    '''
    Multilayer Perceptron.
  '''

    def __init__(self, input_dim):
        super().__init__()
        self.layers = nn.Sequential(
            nn.Linear(input_dim, 64),
            nn.ReLU(),
            nn.Linear(64, 32),
            nn.ReLU(),
            nn.Linear(32, 16)
        )

    def forward(self, x):
        '''Forward pass'''
        return self.layers(x)

# UNet with gloal postion encoding, from haokai
class UNet(nn.Module):
    def __init__(self, n_channels, n_classes, num_frequency, bilinear=False):
        super(UNet, self).__init__()
        self.n_channels = n_channels
        self.n_classes = n_classes
        self.bilinear = bilinear

        self.num_freq = num_frequency
        embedder, embedder_out_dim = get_embedder(self.num_freq)
        self.embedder = embedder
        self.embedder_out_dim = embedder_out_dim
        self.mlp = MLP(embedder_out_dim)

        self.inc = (DoubleConv(n_channels, 64))
        self.down1 = (Down(64, 128))
        self.down2 = (Down(128, 256))
        self.down3 = (Down(256, 512))
        factor = 2 if bilinear else 1
        self.down4 = (Down(512, 1024 // factor))
        self.up1 = (Up_bottle(1024 + 16, 512 + 520, 512 // factor, bilinear))
        self.up2 = (Up(512, 256 // factor, bilinear))
        self.up3 = (Up(256, 128 // factor, bilinear))
        self.up4 = (Up(128, 64, bilinear))
        self.outc = (OutConv(64, n_classes))

    def forward(self, x, x_global):     # x (bs, c, h, w)  x_global (bs, 3)
        x1 = self.inc(x)      # x (bs, 64, h, w)
        x2 = self.down1(x1)   # x (bs, 128, h/2, w/2)
        x3 = self.down2(x2)   # x (bs, 256, h/4, w/4)
        x4 = self.down3(x3)   # x (bs, 512, h/8, w/8)
        x5 = self.down4(x4)   # x (bs, 1024, h/16, w/16)

        _, _, h, w = x5.shape
        x_global = self.mlp(self.embedder(x_global))  # bs, 16
        x_global = x_global.unsqueeze(2).unsqueeze(3).repeat(1, 1, h, w)
        x5 = torch.cat((x5, x_global), dim=1)

        x = self.up1(x5, x4)  # x (bs, 512, h/8, w/8)
        x = self.up2(x, x3)   # x (bs, 256, h/4, w/4)
        x = self.up3(x, x2)   # x (bs, 128, h/2, w/2)
        x = self.up4(x, x1)   # x (bs, 64, h, w)
        logits = self.outc(x)
        return logits


class DoubleConv(nn.Module):
    """(convolution => [BN] => ReLU) * 2"""

    def __init__(self, in_channels, out_channels, mid_channels=None):
        super().__init__()
        if not mid_channels:
            mid_channels = out_channels
        self.double_conv = nn.Sequential(
            nn.Conv2d(in_channels, mid_channels, kernel_size=3, padding=1, bias=False),
            nn.BatchNorm2d(mid_channels),
            nn.ReLU(inplace=True),
            nn.Conv2d(mid_channels, out_channels, kernel_size=3, padding=1, bias=False),
            nn.BatchNorm2d(out_channels),
            nn.ReLU(inplace=True)
        )

    def forward(self, x):
        return self.double_conv(x)


class Down(nn.Module):
    """Downscaling with maxpool then double conv"""

    def __init__(self, in_channels, out_channels):
        super().__init__()
        self.maxpool_conv = nn.Sequential(
            nn.MaxPool2d(2),
            DoubleConv(in_channels, out_channels)
        )

    def forward(self, x):
        return self.maxpool_conv(x)


class Up(nn.Module):
    """Upscaling then double conv"""

    def __init__(self, in_channels, out_channels, bilinear=True):
        super().__init__()

        # if bilinear, use the normal convolutions to reduce the number of channels
        if bilinear:
            self.up = nn.Upsample(scale_factor=2, mode='bilinear', align_corners=True)
            self.conv = DoubleConv(in_channels, out_channels, in_channels // 2)
        else:
            self.up = nn.ConvTranspose2d(in_channels, in_channels // 2, kernel_size=2, stride=2)
            self.conv = DoubleConv(in_channels, out_channels)

    def forward(self, x1, x2):
        x1 = self.up(x1)

        # input is CHW
        diffY = x2.size()[2] - x1.size()[2]
        diffX = x2.size()[3] - x1.size()[3]

        x1 = F.pad(x1, [diffX // 2, diffX - diffX // 2,
                        diffY // 2, diffY - diffY // 2])
        # if you have padding issues, see
        # https://github.com/HaiyongJiang/U-Net-Pytorch-Unstructured-Buggy/commit/0e854509c2cea854e247a9c615f175f76fbb2e3a
        # https://github.com/xiaopeng-liao/Pytorch-UNet/commit/8ebac70e633bac59fc22bb5195e513d5832fb3bd
        x = torch.cat([x2, x1], dim=1)
        return self.conv(x)


class Up_bottle(nn.Module):
    """Upscaling then double conv"""

    def __init__(self, in_channels, mid_channels, out_channels, bilinear=True):
        super().__init__()

        # if bilinear, use the normal convolutions to reduce the number of channels
        if bilinear:
            self.up = nn.Upsample(scale_factor=2, mode='bilinear', align_corners=True)
            self.conv = DoubleConv(in_channels, out_channels, in_channels // 2)
        else:
            self.up = nn.ConvTranspose2d(in_channels, in_channels // 2, kernel_size=2, stride=2)
            self.conv = DoubleConv(mid_channels, out_channels)

    def forward(self, x1, x2):
        x1 = self.up(x1)

        # input is CHW
        diffY = x2.size()[2] - x1.size()[2]
        diffX = x2.size()[3] - x1.size()[3]

        x1 = F.pad(x1, [diffX // 2, diffX - diffX // 2,
                        diffY // 2, diffY - diffY // 2])
        # if you have padding issues, see
        # https://github.com/HaiyongJiang/U-Net-Pytorch-Unstructured-Buggy/commit/0e854509c2cea854e247a9c615f175f76fbb2e3a
        # https://github.com/xiaopeng-liao/Pytorch-UNet/commit/8ebac70e633bac59fc22bb5195e513d5832fb3bd
        x = torch.cat([x2, x1], dim=1)
        return self.conv(x)


class OutConv(nn.Module):
    def __init__(self, in_channels, out_channels):
        super(OutConv, self).__init__()
        self.conv = nn.Conv2d(in_channels, out_channels, kernel_size=1)

    def forward(self, x):
        return self.conv(x)


'''
import pickle

path = r'Z:\work\subj0003_ddc\vlad_global.pkl'

with open(path, 'rb') as file:
    data = pickle.load(file)

a = torch.tensor(data[0], dtype=torch.float).unsqueeze(0)

x = torch.rand([1, 3, 256, 256])
x_2 = a
unet = UNet(3, 10, 6)
print(unet(x, x_2).shape)
'''

```

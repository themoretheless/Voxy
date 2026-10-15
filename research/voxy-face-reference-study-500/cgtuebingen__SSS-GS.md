# Facial rendering source scan: cgtuebingen/SSS-GS

Repository: https://github.com/cgtuebingen/SSS-GS

Pinned source revision: ba3d44ee4e4afa17edf8c27553e47a96af8377bc

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["submodules/diff-gaussian-rasterization/LICENSE.md"]

Topic paths: {"sss": ["gs_sss_rasterization/CMakeLists.txt", "gs_sss_rasterization/README.md", "gs_sss_rasterization/cuda_rasterizer/auxiliary.h", "gs_sss_rasterization/cuda_rasterizer/backward.cu", "gs_sss_rasterization/cuda_rasterizer/backward.h", "gs_sss_rasterization/cuda_rasterizer/config.h", "gs_sss_rasterization/cuda_rasterizer/forward.cu", "gs_sss_rasterization/cuda_rasterizer/forward.h", "gs_sss_rasterization/cuda_rasterizer/rasterizer.h", "gs_sss_rasterization/cuda_rasterizer/rasterizer_impl.cu", "gs_sss_rasterization/cuda_rasterizer/rasterizer_impl.h", "gs_sss_rasterization/ext.cpp", "gs_sss_rasterization/gs_sss_rasterization/__init__.py", "gs_sss_rasterization/rasterize_points.cu", "gs_sss_rasterization/rasterize_points.h", "gs_sss_rasterization/setup.py", "gs_sss_rasterization/third_party/stbi_image_write.h", "scene/sss_model.py"], "gum": ["arguments/__init__.py"]}

## Source file: scene/sss_model.py

{
  "path": "scene/sss_model.py",
  "blob_sha": "312af9169b7f1dbfb94c0cf338d43e77cabfd037",
  "sha256": "bfa0416f083db145b1ca96b7f3394cae1c8032122c8327f032a617bb9b811b42",
  "lines": 130,
  "bytes": 4672,
  "selection_score": 15,
  "keyword_line_counts": {
    "subsurface": 3,
    "sss": 5,
    "face": 3
  },
  "url": "https://github.com/cgtuebingen/SSS-GS/blob/ba3d44ee4e4afa17edf8c27553e47a96af8377bc/scene/sss_model.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__("
    },
    {
      "line": 52,
      "text": "def positional_encoding(self, x: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 68,
      "text": "def forward("
    }
  ]
}

```text

""" Subsurface Scattering (SSS) model. """
import math
from typing import Tuple
import torch
from torch import nn

class SSS(nn.Module):
    """ Subsurface Scattering (SSS) model. """
    def __init__(
        self,
        net_width: int = 32,
        factor: int = 2,
        degree: int = 4
    ):
        """ Initializes the Subsurface Scattering (SSS) model.

        Args:
            net_width (int): The width of the neural network layers. Default is 32.
            factor (int): The factor for the network width. Default is 2.
            degree (int): The degree for the positional encoding. Default is 4.
        """
        super().__init__()
        
        # Model parameters
        self.factor = factor
        self.net_width = net_width

        # Positional encoding parameters
        self.degree = degree

        self.main = nn.Sequential(
            nn.Linear(18 + 3 * 2 * self.degree, self.factor * self.net_width),
            nn.LeakyReLU(),
            nn.Linear(self.net_width * self.factor, self.net_width),
            nn.LeakyReLU(),
            nn.Linear(self.net_width, self.net_width),
            nn.LeakyReLU(),
        )

        self.residual = nn.Sequential(
            nn.Linear(self.net_width, 3),
            nn.Sigmoid(),
        )

        self.incident_light = nn.Sequential(
            nn.Linear(self.net_width, self.net_width),
            nn.LeakyReLU(),
            nn.Linear(self.net_width, 1),
            nn.ReLU()
        )

    def positional_encoding(self, x: torch.Tensor) -> torch.Tensor:
        """ Applies positional encoding to the input.

        Args:
            x (torch.Tensor): Input tensor to be positionally encoded.

        Returns:
            torch.Tensor: Positionally encoded tensor.
        """
        result = []
        for d in range(self.degree):
            for fn in [torch.sin, torch.cos]:
                result.append(fn(2.0 ** d * math.pi * x))
        return torch.cat(result, dim=-1)


    def forward(
            self, 
            positions: torch.Tensor,
            rotations: torch.Tensor,
            scales: torch.Tensor,
            view_dirs: torch.Tensor,
            light_dirs: torch.Tensor,
            normals: torch.Tensor,
            visibilities: torch.Tensor,
            light_distances: torch.Tensor, 
            iteration: int = -1
        ) -> Tuple[torch.Tensor, torch.Tensor]: 
        """ Forward pass of the SSS model.

        Args:
            positions (torch.Tensor): Positions of the points. Shape: [batch, 3]
            rotations (torch.Tensor): Rotations of the points. Shape: [batch, 2]
            scales (torch.Tensor): Scales of the points. Shape: [batch, 3]
            view_dirs (torch.Tensor): View directions. Shape: [batch, 3]
            light_dirs (torch.Tensor): Light directions. Shape: [batch, 3]
            normals (torch.Tensor): Normal vectors. Shape: [batch, 3]
            visibilities (torch.Tensor): Visibility values. Shape: [batch, 1]
            light_distances (torch.Tensor): Distances to light sources. Shape: [batch, 1]
            iteration (int): Current iteration number. Default is -1.

        Returns:
            tuple: A tuple containing:
                - residual (torch.Tensor): Residual values. Shape: [batch, 3]
                - incident_light (torch.Tensor): Incident light values. Shape: [batch, 3]
        """

        encoded_positions = self.positional_encoding(positions)

        x = torch.concat([encoded_positions, rotations, scales, view_dirs, light_dirs, normals, visibilities, light_distances], dim=-1)
        x = self.main(x)

        residual = self.residual(x)
        incident_light = self.incident_light(x).repeat(1, 3)
 
        # Linear decay the incident light
        # TODO: This should be moved to args 
        start_iteration = 0
        stop_iteration = 14_000
        max_incident_light = 4.0

        # Calculate midpoint
        midpoint_iteration = (start_iteration + stop_iteration) // 2


        # FIXME: Remove or fix this
        incident_light_addition = 0.0
        if start_iteration <= iteration <= stop_iteration:
            if iteration <= midpoint_iteration:
                # Increasing phase
                incident_light_addition = max_incident_light * (iteration - start_iteration) / (midpoint_iteration - start_iteration)
            else:
                # Decreasing phase
                # incident_light_addition = max_incident_light * (stop_iteration - iteration) / (stop_iteration - midpoint_iteration)
                pass
            
        incident_light = incident_light + incident_light_addition

        return residual, incident_light

```

## Source file: arguments/__init__.py

{
  "path": "arguments/__init__.py",
  "blob_sha": "53f3cb74c04ee464ad950b2b6157eccc224468d3",
  "sha256": "9c1159f41c42a9d5354c8cd8ccf986b492c7683eba91bbd53d23a46170c9f791",
  "lines": 144,
  "bytes": 4937,
  "selection_score": 3,
  "keyword_line_counts": {
    "subsurface": 2,
    "sss": 2,
    "gum": 8,
    "rig": 2,
    "face": 2
  },
  "url": "https://github.com/cgtuebingen/SSS-GS/blob/ba3d44ee4e4afa17edf8c27553e47a96af8377bc/arguments/__init__.py",
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
      "line": 48,
      "text": "def __init__(self, parser, sentinel=False):"
    },
    {
      "line": 59,
      "text": "def extract(self, args):"
    },
    {
      "line": 69,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 76,
      "text": "def __init__(self, parser):"
    },
    {
      "line": 124,
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
        self._source_path = ""
        self._model_path = ""
        self.alternative_transforms_path = ""
        self._images = "images"
        self._resolution = -1
        self._white_background = False
        self.data_device = "cuda"
        self.eval = False
        super().__init__(parser, "Loading Parameters", sentinel)

    def extract(self, args):
        g = super().extract(args)
        g.source_path = os.path.abspath(g.source_path)
        if g.alternative_transforms_path is not "" and g.alternative_transforms_path is not None:
            g.alternative_transforms_path = os.path.abspath(g.alternative_transforms_path)
        else: 
            g.alternative_transforms_path = None
        return g

class PipelineParams(ParamGroup):
    def __init__(self, parser):
        self.compute_cov3D_python = False
        self.debug = False

        super().__init__(parser, "Pipeline Parameters")

class OptimizationParams(ParamGroup):
    def __init__(self, parser):
        self.iterations = 60_000
        self.batch_size = 100
        self.position_lr_init = 0.00016
        self.position_lr_final = 0.0000016
        self.position_lr_delay_mult = 0.01
        self.position_lr_max_steps = 30_000
        self.feature_lr = 0.0025
        self.opacity_lr = 0.05
        self.color_lr = 0.0025
        self.scaling_lr = 0.005
        self.rotation_lr = 0.001
        self.normals_lr = 0.01

        self.base_color_lr = 0.01
        self.roughness_lr = 0.01
        self.metallic_lr = 0.01
        self.subsurfaceness_lr = 0.01
        self.light_lr = 0.001
        self.light_rest_lr = -1.0
        self.visibility_lr = 0.0025
        self.visibility_rest_lr = 0.0025

        self.percent_dense = 0.01
        self.lambda_dssim = 0.2
        self.lambda_lpips = 0.2
        self.lambda_normal = 0.02
        self.lambda_visibility = 0.01
        self.lambda_incident_light = 0.02
        self.lambda_mask_entropy = 0.1

        self.lambda_base_color = 0.005
        self.lambda_base_color_smooth = 0.006
        self.lambda_metallic_smooth = 0.002
        self.lambda_roughness_smooth = 0.002
        self.lambda_subsurfaceness_smooth = 0.002
        self.lambda_pbr_regularizer = 100.0

        self.densification_interval = 100
        self.opacity_reset_interval = 3000
        self.densify_from_iter = 500
        self.densify_until_iter = 15_000
        self.densify_grad_threshold = 0.0002
        self.random_background = False
        self.sss_width = 32
        self.sss_lr = 0.001
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

## Source file: gs_sss_rasterization/ext.cpp

{
  "path": "gs_sss_rasterization/ext.cpp",
  "blob_sha": "d768779579761238347972a973fbd1603d44235e",
  "sha256": "102d6ca2a07cf57717e5828c49a7100841cc0147f7373de768070f06bc4909ef",
  "lines": 19,
  "bytes": 571,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/cgtuebingen/SSS-GS/blob/ba3d44ee4e4afa17edf8c27553e47a96af8377bc/gs_sss_rasterization/ext.cpp",
  "function_signature_candidates": []
}

```text

/*
 * Copyright (C) 2023, Inria
 * GRAPHDECO research group, https://team.inria.fr/graphdeco
 * All rights reserved.
 *
 * This software is free for non-commercial, research and evaluation use 
 * under the terms of the LICENSE.md file.
 *
 * For inquiries contact  george.drettakis@inria.fr
 */

#include <torch/extension.h>
#include "rasterize_points.h"

PYBIND11_MODULE(TORCH_EXTENSION_NAME, m) {
  m.def("rasterize_gaussians", &RasterizeGaussiansCUDA);
  m.def("rasterize_gaussians_backward", &RasterizeGaussiansBackwardCUDA);
  m.def("mark_visible", &markVisible);
}

```

## Source file: gs_sss_rasterization/setup.py

{
  "path": "gs_sss_rasterization/setup.py",
  "blob_sha": "83d25dba8948231e68112bf4efc7962d321fb57f",
  "sha256": "bbcebb54bf7d03fcf750d6e46a4348b430c2e38930f0f3d6d452ff7790fcb1c5",
  "lines": 37,
  "bytes": 1102,
  "selection_score": 3,
  "keyword_line_counts": {
    "sss": 3,
    "rig": 2
  },
  "url": "https://github.com/cgtuebingen/SSS-GS/blob/ba3d44ee4e4afa17edf8c27553e47a96af8377bc/gs_sss_rasterization/setup.py",
  "function_signature_candidates": []
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

from setuptools import setup
from torch.utils.cpp_extension import CUDAExtension, BuildExtension
import os
os.path.dirname(os.path.abspath(__file__))

setup(
    name="gs_sss_rasterization",
    packages=['gs_sss_rasterization'],
    ext_modules=[
        CUDAExtension(
            name="gs_sss_rasterization._C",
            sources=[
            "cuda_rasterizer/rasterizer_impl.cu",
            "cuda_rasterizer/forward.cu",
            "cuda_rasterizer/backward.cu",
            "rasterize_points.cu",
            "ext.cpp"],
            extra_compile_args={
                "nvcc": ["-I" + os.path.join(os.path.dirname(os.path.abspath(__file__)), "third_party/glm/"),
                         "-O3", "-v"],
                "cxx": ["-O3"]})
        ],
    cmdclass={
        'build_ext': BuildExtension
    }
)


```

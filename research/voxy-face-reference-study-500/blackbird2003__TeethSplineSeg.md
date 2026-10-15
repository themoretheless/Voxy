# Facial rendering source scan: blackbird2003/TeethSplineSeg

Repository: https://github.com/blackbird2003/TeethSplineSeg

Pinned source revision: 238b74c26833703b281685a3f152f133ba49369c

Discovery topic: teeth 3D

Description: A model for 3D tooth segmentation. Official PyTorch implementation of B-Spline Embedded Structure Learning for 3D Tooth Segmentation.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"tooth": ["models/ptv3/ptv3_model_tooth.py", "models/ptv3/ptv3_tooth_seg_network_arch_cac_final.py"]}

## Source file: models/ptv3/ptv3_model_tooth.py

{
  "path": "models/ptv3/ptv3_model_tooth.py",
  "blob_sha": "ebcc8dd040d5119c47d2e54bb8422f79cfc89bb2",
  "sha256": "c80069c7166c90ba7d990ec5be722dd6fe11a2ed671b28ce22aae27915b6d896",
  "lines": 1323,
  "bytes": 54344,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "tooth": 103,
    "rig": 1
  },
  "url": "https://github.com/blackbird2003/TeethSplineSeg/blob/238b74c26833703b281685a3f152f133ba49369c/models/ptv3/ptv3_model_tooth.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def offset2bincount(offset):"
    },
    {
      "line": 33,
      "text": "def offset2batch(offset):"
    },
    {
      "line": 41,
      "text": "def batch2offset(batch):"
    },
    {
      "line": 68,
      "text": "def __init__(self, *args, **kwargs):"
    },
    {
      "line": 76,
      "text": "def serialization(self, order=\"z\", depth=None, shuffle_orders=False):"
    },
    {
      "line": 133,
      "text": "def sparsify(self, pad=96):"
    },
    {
      "line": 177,
      "text": "def __init__(self, *args, **kwargs):"
    },
    {
      "line": 187,
      "text": "def __init__(self, *args, **kwargs):"
    },
    {
      "line": 202,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 212,
      "text": "def __len__(self):"
    },
    {
      "line": 215,
      "text": "def add(self, module, name=None):"
    },
    {
      "line": 222,
      "text": "def forward(self, input):"
    },
    {
      "line": 251,
      "text": "def __init__("
    },
    {
      "line": 273,
      "text": "def forward(self, point):"
    },
    {
      "line": 293,
      "text": "def __init__(self, patch_size, num_heads):"
    },
    {
      "line": 302,
      "text": "def forward(self, coord):"
    },
    {
      "line": 315,
      "text": "def __init__("
    },
    {
      "line": 384,
      "text": "def get_t_guided_order_and_inverse(self, point):"
    },
    {
      "line": 418,
      "text": "def get_t_block_order_and_inverse(self, point):"
    },
    {
      "line": 459,
      "text": "def should_use_t_guided_patching(self, point):"
    },
    {
      "line": 470,
      "text": "def should_use_t_block_patching(self, point):"
    },
    {
      "line": 482,
      "text": "def get_rel_pos(self, point, order, order_tag=\"xyz\"):"
    },
    {
      "line": 492,
      "text": "def get_padding_and_inverse(self, point):"
    },
    {
      "line": 549,
      "text": "def forward(self, point):"
    },
    {
      "line": 614,
      "text": "def __init__("
    },
    {
      "line": 630,
      "text": "def forward(self, x):"
    },
    {
      "line": 640,
      "text": "def __init__("
    },
    {
      "line": 745,
      "text": "def _build_tooth_t_feat(self, t_cond):"
    },
    {
      "line": 752,
      "text": "def forward(self, point: Point):"
    },
    {
      "line": 815,
      "text": "def __init__("
    },
    {
      "line": 843,
      "text": "def forward(self, point: Point):"
    },
    {
      "line": 927,
      "text": "def __init__("
    },
    {
      "line": 950,
      "text": "def forward(self, point):"
    },
    {
      "line": 979,
      "text": "def __init__("
    },
    {
      "line": 1005,
      "text": "def forward(self, point: Point):"
    },
    {
      "line": 1011,
      "text": "def __init__("
    },
    {
      "line": 1267,
      "text": "def _attach_tooth_t(self, point: Point, coarse_t: torch.Tensor):"
    },
    {
      "line": 1275,
      "text": "def forward(self, data_dict):"
    }
  ]
}

```text

"""
PTV3 Model 
modified for tooth segmentation task
"""

import sys
from functools import partial
from addict import Dict
import math
import torch
import torch.nn as nn
import spconv.pytorch as spconv
import torch_scatter
from timm.layers import DropPath
from collections import OrderedDict

try:
    import flash_attn
except ImportError:
    flash_attn = None

from .serialization import encode


@torch.inference_mode()
def offset2bincount(offset):
    return torch.diff(
        offset, prepend=torch.tensor([0], device=offset.device, dtype=torch.long)
    )


@torch.inference_mode()
def offset2batch(offset):
    bincount = offset2bincount(offset)
    return torch.arange(
        len(bincount), device=offset.device, dtype=torch.long
    ).repeat_interleave(bincount)


@torch.inference_mode()
def batch2offset(batch):
    return torch.cumsum(batch.bincount(), dim=0).long()


class Point(Dict):
    """
    Point-cloud data container.

    Stores the tensors and serialization metadata required by the backbone.

    - "coord": original coordinate of point cloud;
    - "grid_coord": grid coordinate for specific grid size (related to GridSampling);
    Point also support the following optional attributes:
    - "offset": if not exist, initialized as batch size is 1;
    - "batch": if not exist, initialized as batch size is 1;
    - "feat": feature of point cloud, default input of model;
    - "grid_size": Grid size of point cloud (related to GridSampling);
    (related to Serialization)
    - "serialized_depth": depth of serialization, 2 ** depth * grid_size describe the maximum of point cloud range;
    - "serialized_code": a list of serialization codes;
    - "serialized_order": a list of serialization order determined by code;
    - "serialized_inverse": a list of inverse mapping determined by code;
    (related to Sparsify: SpConv)
    - "sparse_shape": Sparse shape for Sparse Conv Tensor;
    - "sparse_conv_feat": SparseConvTensor init with information provide by Point;
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # If one of "offset" or "batch" do not exist, generate by the existing one
        if "batch" not in self.keys() and "offset" in self.keys():
            self["batch"] = offset2batch(self.offset)
        elif "offset" not in self.keys() and "batch" in self.keys():
            self["offset"] = batch2offset(self.batch)

    def serialization(self, order="z", depth=None, shuffle_orders=False):
        """
        Point Cloud Serialization

        relay on ["grid_coord" or "coord" + "grid_size", "batch", "feat"]
        """
        assert "batch" in self.keys()
        if "grid_coord" not in self.keys():
            # if you don't want to operate GridSampling in data augmentation,
            # please add the following augmentation into your pipline:
            # dict(type="Copy", keys_dict={"grid_size": 0.01}),
            # (adjust `grid_size` to what your want)
            assert {"grid_size", "coord"}.issubset(self.keys())
            self["grid_coord"] = torch.div(
                self.coord - self.coord.min(0)[0], self.grid_size, rounding_mode="trunc"
            ).int()

        if depth is None:
            # Adaptive measure the depth of serialization cube (length = 2 ^ depth)
            depth = int(self.grid_coord.max()).bit_length()
        self["serialized_depth"] = depth
        # Maximum bit length for serialization code is 63 (int64)
        assert depth * 3 + len(self.offset).bit_length() <= 63
        # Limit the depth to 16 (48 bits) for point positions.
        # Although depth is limited to less than 16, we can encode a 655.36^3 (2^16 * 0.01) meter^3
        # cube with a grid size of 0.01 meter. We consider it is enough for the current stage.
        # We can unlock the limitation by optimizing the z-order encoding function if necessary.
        assert depth <= 16

        # The serialization codes are arranged as following structures:
        # [Order1 ([n]),
        #  Order2 ([n]),
        #   ...
        #  OrderN ([n])] (k, n)
        code = [
            encode(self.grid_coord, self.batch, depth, order=order_) for order_ in order
        ]
        code = torch.stack(code)
        order = torch.argsort(code)
        inverse = torch.zeros_like(order).scatter_(
            dim=1,
            index=order,
            src=torch.arange(0, code.shape[1], device=order.device).repeat(
                code.shape[0], 1
            ),
        )

        if shuffle_orders:
            perm = torch.randperm(code.shape[0])
            code = code[perm]
            order = order[perm]
            inverse = inverse[perm]

        self["serialized_code"] = code
        self["serialized_order"] = order
        self["serialized_inverse"] = inverse

    def sparsify(self, pad=96):
        """
        Point Cloud Serialization

        Point cloud is sparse, here we use "sparsify" to specifically refer to
        preparing "spconv.SparseConvTensor" for SpConv.

        relay on ["grid_coord" or "coord" + "grid_size", "batch", "feat"]

        pad: padding sparse for sparse shape.
        """
        assert {"feat", "batch"}.issubset(self.keys())
        if "grid_coord" not in self.keys():
            # if you don't want to operate GridSampling in data augmentation,
            # please add the following augmentation into your pipline:
            # dict(type="Copy", keys_dict={"grid_size": 0.01}),
            # (adjust `grid_size` to what your want)
            assert {"grid_size", "coord"}.issubset(self.keys())
            self["grid_coord"] = torch.div(
                self.coord - self.coord.min(0)[0], self.grid_size, rounding_mode="trunc"
            ).int()
        if "sparse_shape" in self.keys():
            sparse_shape = self.sparse_shape
        else:
            sparse_shape = torch.add(
                torch.max(self.grid_coord, dim=0).values, pad
            ).tolist()
        sparse_conv_feat = spconv.SparseConvTensor(
            features=self.feat,
            indices=torch.cat(
                [self.batch.unsqueeze(-1).int(), self.grid_coord.int()], dim=1
            ).contiguous(),
            spatial_shape=sparse_shape,
            batch_size=self.batch[-1].tolist() + 1,
        )
        self["sparse_shape"] = sparse_shape
        self["sparse_conv_feat"] = sparse_conv_feat


class PointModule(nn.Module):
    r"""PointModule
    placeholder, all module subclass from this will take Point in PointSequential.
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)


class PointSequential(PointModule):
    r"""A sequential container.
    Modules will be added to it in the order they are passed in the constructor.
    Alternatively, an ordered dict of modules can also be passed in.
    """

    def __init__(self, *args, **kwargs):
        super().__init__()
        if len(args) == 1 and isinstance(args[0], OrderedDict):
            for key, module in args[0].items():
                self.add_module(key, module)
        else:
            for idx, module in enumerate(args):
                self.add_module(str(idx), module)
        for name, module in kwargs.items():
            if sys.version_info < (3, 6):
                raise ValueError("kwargs only supported in py36+")
            if name in self._modules:
                raise ValueError("name exists.")
            self.add_module(name, module)

    def __getitem__(self, idx):
        if not (-len(self) <= idx < len(self)):
            raise IndexError("index {} is out of range".format(idx))
        if idx < 0:
            idx += len(self)
        it = iter(self._modules.values())
        for i in range(idx):
            next(it)
        return next(it)

    def __len__(self):
        return len(self._modules)

    def add(self, module, name=None):
        if name is None:
            name = str(len(self._modules))
            if name in self._modules:
                raise KeyError("name exists")
        self.add_module(name, module)

    def forward(self, input):
        for k, module in self._modules.items():
            # Point module
            if isinstance(module, PointModule):
                input = module(input)
            # Spconv module
            elif spconv.modules.is_spconv_module(module):
                if isinstance(input, Point):
                    input.sparse_conv_feat = module(input.sparse_conv_feat)
                    input.feat = input.sparse_conv_feat.features
                else:
                    input = module(input)
            # PyTorch module
            else:
                if isinstance(input, Point):
                    input.feat = module(input.feat)
                    if "sparse_conv_feat" in input.keys():
                        input.sparse_conv_feat = input.sparse_conv_feat.replace_feature(
                            input.feat
                        )
                elif isinstance(input, spconv.SparseConvTensor):
                    if input.indices.shape[0] != 0:
                        input = input.replace_feature(module(input.features))
                else:
                    input = module(input)
        return input


class PDNorm(PointModule):
    def __init__(
        self,
        num_features,
        norm_layer,
        context_channels=256,
        conditions=("ScanNet", "S3DIS", "Structured3D"),
        decouple=True,
        adaptive=False,
    ):
        super().__init__()
        self.conditions = conditions
        self.decouple = decouple
        self.adaptive = adaptive
        if self.decouple:
            self.norm = nn.ModuleList([norm_layer(num_features) for _ in conditions])
        else:
            self.norm = norm_layer
        if self.adaptive:
            self.modulation = nn.Sequential(
                nn.SiLU(), nn.Linear(context_channels, 2 * num_features, bias=True)
            )

    def forward(self, point):
        assert {"feat", "condition"}.issubset(point.keys())
        if isinstance(point.condition, str):
            condition = point.condition
        else:
            condition = point.condition[0]
        if self.decouple:
            assert condition in self.conditions
            norm = self.norm[self.conditions.index(condition)]
        else:
            norm = self.norm
        point.feat = norm(point.feat)
        if self.adaptive:
            assert "context" in point.keys()
            shift, scale = self.modulation(point.context).chunk(2, dim=1)
            point.feat = point.feat * (1.0 + scale) + shift
        return point


class RPE(torch.nn.Module):
    def __init__(self, patch_size, num_heads):
        super().__init__()
        self.patch_size = patch_size
        self.num_heads = num_heads
        self.pos_bnd = int((4 * patch_size) ** (1 / 3) * 2)
        self.rpe_num = 2 * self.pos_bnd + 1
        self.rpe_table = torch.nn.Parameter(torch.zeros(3 * self.rpe_num, num_heads))
        torch.nn.init.trunc_normal_(self.rpe_table, std=0.02)

    def forward(self, coord):
        idx = (
            coord.clamp(-self.pos_bnd, self.pos_bnd)  # clamp into bnd
            + self.pos_bnd  # relative position to positive index
            + torch.arange(3, device=coord.device) * self.rpe_num  # x, y, z stride
        )
        out = self.rpe_table.index_select(0, idx.reshape(-1))
        out = out.view(idx.shape + (-1,)).sum(3)
        out = out.permute(0, 3, 1, 2)  # (N, K, K, H) -> (N, H, K, K)
        return out


class SerializedAttention(PointModule):
    def __init__(
        self,
        channels,
        num_heads,
        patch_size,
        qkv_bias=True,
        qk_scale=None,
        attn_drop=0.0,
        proj_drop=0.0,
        order_index=0,
        enable_rpe=False,
        enable_flash=True,
        upcast_attention=True,
        upcast_softmax=True,
        use_t_guided_patching=False,
        t_guided_patching_prob=1.0,
        use_t_block_patching=False,
        t_block_patching_k=8,
        t_block_patching_prob=1.0,
    ):
        super().__init__()
        assert channels % num_heads == 0
        self.channels = channels
        self.num_heads = num_heads
        self.scale = qk_scale or (channels // num_heads) ** -0.5
        self.order_index = order_index
        self.upcast_attention = upcast_attention
        self.upcast_softmax = upcast_softmax
        self.enable_rpe = enable_rpe
        self.enable_flash = enable_flash
        self.use_t_guided_patching = bool(use_t_guided_patching)
        self.t_guided_patching_prob = float(t_guided_patching_prob)
        if self.t_guided_patching_prob < 0.0 or self.t_guided_patching_prob > 1.0:
            raise ValueError(f"t_guided_patching_prob must be in [0, 1], got {self.t_guided_patching_prob}")
        self.use_t_block_patching = bool(use_t_block_patching)
        self.t_block_patching_k = int(t_block_patching_k)
        self.t_block_patching_prob = float(t_block_patching_prob)
        if self.t_block_patching_k <= 0:
            raise ValueError(f"t_block_patching_k must be positive, got {self.t_block_patching_k}")
        if self.t_block_patching_prob < 0.0 or self.t_block_patching_prob > 1.0:
            raise ValueError(f"t_block_patching_prob must be in [0, 1], got {self.t_block_patching_prob}")
        if enable_flash:
            assert (
                enable_rpe is False
            ), "Set enable_rpe to False when enable Flash Attention"
            assert (
                upcast_attention is False
            ), "Set upcast_attention to False when enable Flash Attention"
            assert (
                upcast_softmax is False
            ), "Set upcast_softmax to False when enable Flash Attention"
            assert flash_attn is not None, "Make sure flash_attn is installed."
            self.patch_size = patch_size
            self.attn_drop = attn_drop
        else:
            # when disable flash attention, we still don't want to use mask
            # consequently, patch size will auto set to the
            # min number of patch_size_max and number of points
            self.patch_size_max = patch_size
            self.patch_size = 0
            self.attn_drop = torch.nn.Dropout(attn_drop)

        self.qkv = torch.nn.Linear(channels, channels * 3, bias=qkv_bias)
        self.proj = torch.nn.Linear(channels, channels)
        self.proj_drop = torch.nn.Dropout(proj_drop)
        self.softmax = torch.nn.Softmax(dim=-1)
        self.rpe = RPE(patch_size, num_heads) if self.enable_rpe else None

    @torch.no_grad()
    def get_t_guided_order_and_inverse(self, point):
        order_key = "t_guided_serialized_order"
        inverse_key = "t_guided_serialized_inverse"
        if order_key not in point.keys() or inverse_key not in point.keys():
            t_values = point.t_cond.detach().reshape(-1).float()
            t_values = torch.nan_to_num(t_values, nan=0.0, posinf=1.0, neginf=0.0)
            t_values = t_values.clamp(0.0, 1.0)

            offset = point.offset
            padded_offset = nn.functional.pad(offset, (1, 0))
            order = torch.empty_like(point.batch, dtype=torch.long)
            for batch_idx in range(len(offset)):
                start = int(padded_offset[batch_idx].item())
                end = int(padded_offset[batch_idx + 1].item())
                point_idx = torch.arange(start, end, device=t_values.device, dtype=torch.long)
                sorted_local = torch.argsort(t_values[start:end], stable=True)
                order[start:end] = point_idx[sorted_local]

            inverse = torch.empty_like(order)
            inverse.scatter_(
                dim=0,
                index=order,
                src=torch.arange(order.numel(), device=order.device, dtype=torch.long),
            )

            num_orders = point.serialized_order.shape[0]
            point[order_key] = order.unsqueeze(0).repeat(num_orders, 1)
            point[inverse_key] = inverse.unsqueeze(0).repeat(num_orders, 1)

        order = point[order_key][self.order_index % point[order_key].shape[0]]
        inverse = point[inverse_key][self.order_index % point[inverse_key].shape[0]]
        return order, inverse

    @torch.no_grad()
    def get_t_block_order_and_inverse(self, point):
        order_key = f"t_block_serialized_order_{self.t_block_patching_k}"
        inverse_key = f"t_block_serialized_inverse_{self.t_block_patching_k}"
        if order_key not in point.keys() or inverse_key not in point.keys():
            t_values = point.t_block_cond.detach().reshape(-1).float()
            t_values = torch.nan_to_num(t_values, nan=0.0, posinf=1.0, neginf=0.0)
            t_values = t_values.clamp(0.0, 1.0)
            block_ids = torch.clamp(
                torch.floor(t_values * float(self.t_block_patching_k)).long(),
                min=0,
                max=self.t_block_patching_k - 1,
            )

            num_orders, num_points = point.serialized_order.shape
            block_orders = []
            batch_count = int(point.offset.numel())
            for order_idx in range(num_orders):
                base_order = point.serialized_order[order_idx]
                chunks = []
                for batch_idx in range(batch_count):
                    batch_order = base_order[point.batch[base_order] == batch_idx]
                    for block_idx in range(self.t_block_patching_k):
                        block_order = batch_order[block_ids[batch_order] == block_idx]
                        if block_order.numel() > 0:
                            chunks.append(block_order)
                block_orders.append(torch.cat(chunks, dim=0) if chunks else base_order)

            order = torch.stack(block_orders, dim=0)
            inverse = torch.empty_like(order)
            inverse.scatter_(
                dim=1,
                index=order,
                src=torch.arange(num_points, device=order.device, dtype=torch.long).repeat(num_orders, 1),
            )
            point[order_key] = order
            point[inverse_key] = inverse

        order = point[order_key][self.order_index % point[order_key].shape[0]]
        inverse = point[inverse_key][self.order_index % point[inverse_key].shape[0]]
        return order, inverse

    def should_use_t_guided_patching(self, point):
        if not self.use_t_guided_patching:
            return False
        if "t_cond" not in point.keys() or point.t_cond is None:
            return False
        if self.t_guided_patching_prob <= 0.0:
            return False
        if self.t_guided_patching_prob >= 1.0:
            return True
        return bool(torch.rand((), device=point.feat.device).item() < self.t_guided_patching_prob)

    def should_use_t_block_patching(self, point):
        if not self.use_t_block_patching:
            return False
        if "t_block_cond" not in point.keys() or point.t_block_cond is None:
            return False
        if self.t_block_patching_prob <= 0.0:
            return False
        if self.t_block_patching_prob >= 1.0:
            return True
        return bool(torch.rand((), device=point.feat.device).item() < self.t_block_patching_prob)

    @torch.no_grad()
    def get_rel_pos(self, point, order, order_tag="xyz"):
        K = self.patch_size
        rel_pos_key = f"rel_pos_{order_tag}_{self.order_index}"
        if rel_pos_key not in point.keys():
            grid_coord = point.grid_coord[order]
            grid_coord = grid_coord.reshape(-1, K, 3)
            point[rel_pos_key] = grid_coord.unsqueeze(2) - grid_coord.unsqueeze(1)
        return point[rel_pos_key]

    @torch.no_grad()
    def get_padding_and_inverse(self, point):
        pad_key = "pad"
        unpad_key = "unpad"
        cu_seqlens_key = "cu_seqlens_key"
        if (
            pad_key not in point.keys()
            or unpad_key not in point.keys()
            or cu_seqlens_key not in point.keys()
        ):
            offset = point.offset
            bincount = offset2bincount(offset)
            bincount_pad = (
                torch.div(
                    bincount + self.patch_size - 1,
                    self.patch_size,
                    rounding_mode="trunc",
                )
                * self.patch_size
            )
            # only pad point when num of points larger than patch_size
            mask_pad = bincount > self.patch_size
            bincount_pad = ~mask_pad * bincount + mask_pad * bincount_pad
            _offset = nn.functional.pad(offset, (1, 0))
            _offset_pad = nn.functional.pad(torch.cumsum(bincount_pad, dim=0), (1, 0))
            pad = torch.arange(_offset_pad[-1], device=offset.device)
            unpad = torch.arange(_offset[-1], device=offset.device)
            cu_seqlens = []
            for i in range(len(offset)):
                unpad[_offset[i] : _offset[i + 1]] += _offset_pad[i] - _offset[i]
                if bincount[i] != bincount_pad[i]:
                    pad[
                        _offset_pad[i + 1]
                        - self.patch_size
                        + (bincount[i] % self.patch_size) : _offset_pad[i + 1]
                    ] = pad[
                        _offset_pad[i + 1]
                        - 2 * self.patch_size
                        + (bincount[i] % self.patch_size) : _offset_pad[i + 1]
                        - self.patch_size
                    ]
                pad[_offset_pad[i] : _offset_pad[i + 1]] -= _offset_pad[i] - _offset[i]
                cu_seqlens.append(
                    torch.arange(
                        _offset_pad[i],
                        _offset_pad[i + 1],
                        step=self.patch_size,
                        dtype=torch.int32,
                        device=offset.device,
                    )
                )
            point[pad_key] = pad
            point[unpad_key] = unpad
            point[cu_seqlens_key] = nn.functional.pad(
                torch.concat(cu_seqlens), (0, 1), value=_offset_pad[-1]
            )
        return point[pad_key], point[unpad_key], point[cu_seqlens_key]

    def forward(self, point):
        if not self.enable_flash:
            self.patch_size = min(
                offset2bincount(point.offset).min().tolist(), self.patch_size_max
            )

        H = self.num_heads
        K = self.patch_size
        C = self.channels

        pad, unpad, cu_seqlens = self.get_padding_and_inverse(point)

        order_tag = "xyz"
        if self.should_use_t_block_patching(point):
            serialized_order, serialized_inverse = self.get_t_block_order_and_inverse(point)
            order_tag = f"tblock{self.t_block_patching_k}"
        elif self.should_use_t_guided_patching(point):
            serialized_order, serialized_inverse = self.get_t_guided_order_and_inverse(point)
            order_tag = "t"
        else:
            serialized_order = point.serialized_order[self.order_index]
            serialized_inverse = point.serialized_inverse[self.order_index]

        order = serialized_order[pad]
        inverse = unpad[serialized_inverse]

        # padding and reshape feat and batch for serialized point patch
        qkv = self.qkv(point.feat)[order]

        if not self.enable_flash:
            # encode and reshape qkv: (N', K, 3, H, C') => (3, N', H, K, C')
            q, k, v = (
                qkv.reshape(-1, K, 3, H, C // H).permute(2, 0, 3, 1, 4).unbind(dim=0)
            )
            # attn
            if self.upcast_attention:
                q = q.float()
                k = k.float()
            attn = (q * self.scale) @ k.transpose(-2, -1)  # (N', H, K, K)
            if self.enable_rpe:
                attn = attn + self.rpe(self.get_rel_pos(point, order, order_tag=order_tag))
            if self.upcast_softmax:
                attn = attn.float()
            attn = self.softmax(attn)
            attn = self.attn_drop(attn).to(qkv.dtype)
            feat = (attn @ v).transpose(1, 2).reshape(-1, C)
        else:
            feat = flash_attn.flash_attn_varlen_qkvpacked_func(
                qkv.half().reshape(-1, 3, H, C // H),
                cu_seqlens,
                max_seqlen=self.patch_size,
                dropout_p=self.attn_drop if self.training else 0,
                softmax_scale=self.scale,
            ).reshape(-1, C)
            feat = feat.to(qkv.dtype)
        feat = feat[inverse]

        # ffn
        feat = self.proj(feat)
        feat = self.proj_drop(feat)
        point.feat = feat
        return point


class MLP(nn.Module):
    def __init__(
        self,
        in_channels,
        hidden_channels=None,
        out_channels=None,
        act_layer=nn.GELU,
        drop=0.0,
    ):
        super().__init__()
        out_channels = out_channels or in_channels
        hidden_channels = hidden_channels or in_channels
        self.fc1 = nn.Linear(in_channels, hidden_channels)
        self.act = act_layer()
        self.fc2 = nn.Linear(hidden_channels, out_channels)
        self.drop = nn.Dropout(drop)

    def forward(self, x):
        x = self.fc1(x)
        x = self.act(x)
        x = self.drop(x)
        x = self.fc2(x)
        x = self.drop(x)
        return x


class Block(PointModule):
    def __init__(
        self,
        channels,
        num_heads,
        patch_size=48,
        mlp_ratio=4.0,
        qkv_bias=True,
        qk_scale=None,
        attn_drop=0.0,
        proj_drop=0.0,
        drop_path=0.0,
        norm_layer=nn.LayerNorm,
        act_layer=nn.GELU,
        pre_norm=True,
        order_index=0,
        cpe_indice_key=None,
        enable_rpe=False,
        enable_flash=True,
        upcast_attention=True,
        upcast_softmax=True,
        use_tooth_t_symmetry=False,
        tooth_t_dynamic_gate=False,
        tooth_t_dynamic_gate_hidden_dim=16,
        tooth_t_guided_patching=False,
        tooth_t_guided_patching_prob=1.0,
        tooth_t_block_patching=False,
        tooth_t_block_patching_k=8,
        tooth_t_block_patching_prob=1.0,
    ):
        super().__init__()
        self.channels = channels
        self.pre_norm = pre_norm
        self.use_tooth_t_symmetry = use_tooth_t_symmetry
        self.tooth_t_dynamic_gate = bool(tooth_t_dynamic_gate)
        tooth_t_input_dim = 2 if use_tooth_t_symmetry else 1
        # 用一个轻量 MLP 将中间预测得到的牙弓参数 t 映射到通道空间，
        # 以逐点加性偏置的方式增强 xCPE，避免直接改注意力带来的训练不稳定。
        # 可选地在原始 t 之外拼接 min(t, 1-t)，用于编码关于 0.5 的对称位置信息。(默认关闭，效果不明显)
        self.tooth_t_xcpe = nn.Sequential(
            nn.Linear(tooth_t_input_dim, channels),
            nn.SiLU(),
            nn.Linear(channels, channels),
        )
        self.tooth_t_xcpe_gate = None
        if self.tooth_t_dynamic_gate:
            gate_hidden_dim = int(tooth_t_dynamic_gate_hidden_dim)
            if gate_hidden_dim > 0:
                self.tooth_t_xcpe_gate = nn.Sequential(
                    nn.Linear(channels + tooth_t_input_dim, gate_hidden_dim),
                    nn.SiLU(),
                    nn.Linear(gate_hidden_dim, 1),
                    nn.Sigmoid(),
                )
            else:
                self.tooth_t_xcpe_gate = nn.Sequential(
                    nn.Linear(channels + tooth_t_input_dim, 1),
                    nn.Sigmoid(),
                )

        self.cpe = PointSequential(
            spconv.SubMConv3d(
                channels,
                channels,
                kernel_size=3,
                bias=True,
                indice_key=cpe_indice_key,
            ),
            nn.Linear(channels, channels),
            norm_layer(channels),
        )

        self.norm1 = PointSequential(norm_layer(channels))
        self.attn = SerializedAttention(
            channels=channels,
            patch_size=patch_size,
            num_heads=num_heads,
            qkv_bias=qkv_bias,
            qk_scale=qk_scale,
            attn_drop=attn_drop,
            proj_drop=proj_drop,
            order_index=order_index,
            enable_rpe=enable_rpe,
            enable_flash=enable_flash,
            upcast_attention=upcast_attention,
            upcast_softmax=upcast_softmax,
            use_t_guided_patching=tooth_t_guided_patching,
            t_guided_patching_prob=tooth_t_guided_patching_prob,
            use_t_block_patching=tooth_t_block_patching,
            t_block_patching_k=tooth_t_block_patching_k,
            t_block_patching_prob=tooth_t_block_patching_prob,
        )
        self.norm2 = PointSequential(norm_layer(channels))
        self.mlp = PointSequential(
            MLP(
                in_channels=channels,
                hidden_channels=int(channels * mlp_ratio),
                out_channels=channels,
                act_layer=act_layer,
                drop=proj_drop,
            )
        )
        self.drop_path = PointSequential(
            DropPath(drop_path) if drop_path > 0.0 else nn.Identity()
        )

    def _build_tooth_t_feat(self, t_cond):
        # 在保留绝对 t 的同时，可选加入关于牙弓中线对称的 min(t, 1-t) 特征。（默认关闭，效果不明显）
        if not self.use_tooth_t_symmetry:
            return t_cond
        t_symmetry = torch.minimum(t_cond, 1.0 - t_cond)
        return torch.cat([t_cond, t_symmetry], dim=-1)

    def forward(self, point: Point):
        shortcut = point.feat

        point = self.cpe(point)
        point.feat = shortcut + point.feat

        # 仅在 backbone 中后段拿到 t_cond 后才启用牙弓位置调制
        if "t_cond" in point.keys() and point.t_cond is not None:
            shortcut = point.feat
            tooth_t_feat = self._build_tooth_t_feat(point.t_cond.float())
            tooth_t_bias = self.tooth_t_xcpe(tooth_t_feat)
            if self.tooth_t_xcpe_gate is not None:
                gate_input = torch.cat(
                    [point.feat, tooth_t_feat.to(device=point.feat.device, dtype=point.feat.dtype)],
                    dim=-1,
                )
                tooth_t_bias = self.tooth_t_xcpe_gate(gate_input) * tooth_t_bias
            point.feat = shortcut + tooth_t_bias

        shortcut = point.feat

        if self.pre_norm:
            point = self.norm1(point)
        point = self.drop_path(self.attn(point))
        point.feat = shortcut + point.feat
        if not self.pre_norm:
            point = self.norm1(point)

        shortcut = point.feat
        if self.pre_norm:
            point = self.norm2(point)
        point = self.drop_path(self.mlp(point))
        point.feat = shortcut + point.feat
        if not self.pre_norm:
            point = self.norm2(point)
        point.sparse_conv_feat = point.sparse_conv_feat.replace_feature(point.feat)
        return point
    
    # ptv3的原本实现
    # def forward(self, point: Point):
    #     shortcut = point.feat
    #     point = self.cpe(point)
    #     point.feat = shortcut + point.feat
    #     shortcut = point.feat
    #     if self.pre_norm:
    #         point = self.norm1(point)
    #     point = self.drop_path(self.attn(point))
    #     point.feat = shortcut + point.feat
    #     if not self.pre_norm:
    #         point = self.norm1(point)

    #     shortcut = point.feat
    #     if self.pre_norm:
    #         point = self.norm2(point)
    #     point = self.drop_path(self.mlp(point))
    #     point.feat = shortcut + point.feat
    #     if not self.pre_norm:
    #         point = self.norm2(point)
    #     point.sparse_conv_feat = point.sparse_conv_feat.replace_feature(point.feat)
    #     return point


class SerializedPooling(PointModule):
    def __init__(
        self,
        in_channels,
        out_channels,
        stride=2,
        norm_layer=None,
        act_layer=None,
        reduce="max",
        shuffle_orders=True,
        traceable=True,  # record parent and cluster
    ):
        super().__init__()
        self.in_channels = in_channels
        self.out_channels = out_channels

        assert stride == 2 ** (math.ceil(stride) - 1).bit_length()  # 2, 4, 8
        self.stride = stride
        assert reduce in ["sum", "mean", "min", "max"]
        self.reduce = reduce
        self.shuffle_orders = shuffle_orders
        self.traceable = traceable

        self.proj = nn.Linear(in_channels, out_channels)
        if norm_layer is not None:
            self.norm = PointSequential(norm_layer(out_channels))
        if act_layer is not None:
            self.act = PointSequential(act_layer())

    def forward(self, point: Point):
        pooling_depth = (math.ceil(self.stride) - 1).bit_length()
        if pooling_depth > point.serialized_depth:
            pooling_depth = 0
        assert {
            "serialized_code",
            "serialized_order",
            "serialized_inverse",
            "serialized_depth",
        }.issubset(
            point.keys()
        ), "Run point.serialization() point cloud before SerializedPooling"

        code = point.serialized_code >> pooling_depth * 3
        code_, cluster, counts = torch.unique(
            code[0],
            sorted=True,
            return_inverse=True,
            return_counts=True,
        )
        # indices of point sorted by cluster, for torch_scatter.segment_csr
        _, indices = torch.sort(cluster)
        # index pointer for sorted point, for torch_scatter.segment_csr
        idx_ptr = torch.cat([counts.new_zeros(1), torch.cumsum(counts, dim=0)])
        # head_indices of each cluster, for reduce attr e.g. code, batch
        head_indices = indices[idx_ptr[:-1]]
        # generate down code, order, inverse
        code = code[:, head_indices]
        order = torch.argsort(code)
        inverse = torch.zeros_like(order).scatter_(
            dim=1,
            index=order,
            src=torch.arange(0, code.shape[1], device=order.device).repeat(
                code.shape[0], 1
            ),
        )

        if self.shuffle_orders:
            perm = torch.randperm(code.shape[0])
            code = code[perm]
            order = order[perm]
            inverse = inverse[perm]

        # collect information
        point_dict = Dict(
            feat=torch_scatter.segment_csr(
                self.proj(point.feat)[indices], idx_ptr, reduce=self.reduce
            ),
            coord=torch_scatter.segment_csr(
                point.coord[indices], idx_ptr, reduce="mean"
            ),
            grid_coord=point.grid_coord[head_indices] >> pooling_depth,
            serialized_code=code,
            serialized_order=order,
            serialized_inverse=inverse,
            serialized_depth=point.serialized_depth - pooling_depth,
            batch=point.batch[head_indices],
        )

        if "condition" in point.keys():
            point_dict["condition"] = point.condition
        if "context" in point.keys():
            point_dict["context"] = point.context
        if "t_block_cond" in point.keys() and point.t_block_cond is not None:
            t_block_cond = point.t_block_cond
            if t_block_cond.dim() == 1:
                t_block_cond = t_block_cond.unsqueeze(-1)
            point_dict["t_block_cond"] = torch_scatter.segment_csr(
                t_block_cond[indices], idx_ptr, reduce="mean"
            )

        if self.traceable:
            point_dict["pooling_inverse"] = cluster
            point_dict["pooling_parent"] = point
        point = Point(point_dict)
        if self.norm is not None:
            point = self.norm(point)
        if self.act is not None:
            point = self.act(point)
        point.sparsify()
        return point


class SerializedUnpooling(PointModule):
    def __init__(
        self,
        in_channels,
        skip_channels,
        out_channels,
        norm_layer=None,
        act_layer=None,
        traceable=False,  # record parent and cluster
    ):
        super().__init__()
        self.proj = PointSequential(nn.Linear(in_channels, out_channels))
        self.proj_skip = PointSequential(nn.Linear(skip_channels, out_channels))

        if norm_layer is not None:
            self.proj.add(norm_layer(out_channels))
            self.proj_skip.add(norm_layer(out_channels))

        if act_layer is not None:
            self.proj.add(act_layer())
            self.proj_skip.add(act_layer())

        self.traceable = traceable

    def forward(self, point):
        assert "pooling_parent" in point.keys()
        assert "pooling_inverse" in point.keys()
        parent = point.pop("pooling_parent")
        inverse = point.pop("pooling_inverse")
        point = self.proj(point)
        parent = self.proj_skip(parent)
        parent.feat = parent.feat + point.feat[inverse]
        # 将粗粒度 t 通过 unpooling 的逆映射传播到更高分辨率，
        # 这样后续 decoder block 可以持续使用同一个 coarse t 来增强 xCPE。
        if "t_cond" in point.keys() and point.t_cond is not None:
            parent["t_cond"] = point.t_cond[inverse]
        if (
            "t_block_cond" in point.keys()
            and point.t_block_cond is not None
            and ("t_block_cond" not in parent.keys() or parent.t_block_cond is None)
        ):
            parent["t_block_cond"] = point.t_block_cond[inverse]
        # 训练监督使用单独的可求导副本，避免为了稳定 xCPE 而 detach 后，
        # 中间 coarse t 分支本身失去梯度。
        if "t_coarse_supervision" in point.keys() and point.t_coarse_supervision is not None:
            parent["t_coarse_supervision"] = point.t_coarse_supervision[inverse]

        if self.traceable:
            parent["unpooling_parent"] = point
        return parent


class Embedding(PointModule):
    def __init__(
        self,
        in_channels,
        embed_channels,
        norm_layer=None,
        act_layer=None,
    ):
        super().__init__()
        self.in_channels = in_channels
        self.embed_channels = embed_channels

        self.stem = PointSequential(
            conv=spconv.SubMConv3d(
                in_channels,
                embed_channels,
                kernel_size=5,
                padding=1,
                bias=False,
                indice_key="stem",
            )
        )
        if norm_layer is not None:
            self.stem.add(norm_layer(embed_channels), name="norm")
        if act_layer is not None:
            self.stem.add(act_layer(), name="act")

    def forward(self, point: Point):
        point = self.stem(point)
        return point


class PointTransformerV3(PointModule):
    def __init__(
        self,
        in_channels=6,
        order=("z", "z-trans", "hilbert", "hilbert-trans"),
        stride=(2, 2, 2, 2),
        enc_depths=(2, 2, 2, 6, 2),
        enc_channels=(32, 64, 128, 256, 512),
        enc_num_head=(2, 4, 8, 16, 32),
        enc_patch_size=(1024, 1024, 1024, 1024, 1024),
        dec_depths=(2, 2, 2, 2),
        dec_channels=(64, 64, 128, 256),
        dec_num_head=(4, 4, 8, 16),
        dec_patch_size=(1024, 1024, 1024, 1024),
        mlp_ratio=4,
        qkv_bias=True,
        qk_scale=None,
        attn_drop=0.0,
        proj_drop=0.0,
        drop_path=0.3,
        pre_norm=True,
        shuffle_orders=True,
        enable_rpe=False,
        enable_flash=True,
        upcast_attention=False,
        upcast_softmax=False,
        cls_mode=False,
        pdnorm_bn=False,
        pdnorm_ln=False,
        pdnorm_decouple=True,
        pdnorm_adaptive=False,
        pdnorm_affine=True,
        pdnorm_conditions=("ScanNet", "S3DIS", "Structured3D"),
        use_tooth_t_stage2=False,
        tooth_t_mid_stage=0,
        tooth_t_detach_for_xcpe=True,
        tooth_t_inject_decoder=True,
        use_tooth_t_symmetry=False,
        tooth_t_dynamic_gate=False,
        tooth_t_dynamic_gate_hidden_dim=16,
        tooth_t_guided_patching=False,
        tooth_t_guided_patching_prob=1.0,
        tooth_t_block_patching=False,
        tooth_t_block_patching_source="pred",
        tooth_t_block_patching_k=8,
        tooth_t_block_patching_prob=1.0,
    ):
        super().__init__()
        self.num_stages = len(enc_depths)
        self.order = [order] if isinstance(order, str) else order
        self.cls_mode = cls_mode
        self.shuffle_orders = shuffle_orders
        # 这几个开关用于两阶段 tooth backbone。
        self.use_tooth_t_stage2 = use_tooth_t_stage2 and (not cls_mode)
        self.tooth_t_mid_stage = int(tooth_t_mid_stage)
        self.tooth_t_detach_for_xcpe = tooth_t_detach_for_xcpe
        self.tooth_t_inject_decoder = bool(tooth_t_inject_decoder)
        self.use_tooth_t_symmetry = use_tooth_t_symmetry
        self.tooth_t_dynamic_gate = bool(tooth_t_dynamic_gate)
        self.tooth_t_dynamic_gate_hidden_dim = int(tooth_t_dynamic_gate_hidden_dim)
        self.tooth_t_guided_patching = bool(tooth_t_guided_patching)
        self.tooth_t_guided_patching_prob = float(tooth_t_guided_patching_prob)
        if self.tooth_t_guided_patching_prob < 0.0 or self.tooth_t_guided_patching_prob > 1.0:
            raise ValueError(
                f"tooth_t_guided_patching_prob must be in [0, 1], got {self.tooth_t_guided_patching_prob}"
            )
        self.tooth_t_block_patching = bool(tooth_t_block_patching)
        self.tooth_t_block_patching_source = str(tooth_t_block_patching_source).lower()
        if self.tooth_t_block_patching_source not in {"pred", "gt", "pred_t"}:
            raise ValueError(
                f"tooth_t_block_patching_source must be 'pred', 'gt', or 'pred_t', got {tooth_t_block_patching_source}"
            )
        self.tooth_t_block_patching_k = int(tooth_t_block_patching_k)
        self.tooth_t_block_patching_prob = float(tooth_t_block_patching_prob)
        if self.tooth_t_block_patching_k <= 0:
            raise ValueError(f"tooth_t_block_patching_k must be positive, got {self.tooth_t_block_patching_k}")
        if self.tooth_t_block_patching_prob < 0.0 or self.tooth_t_block_patching_prob > 1.0:
            raise ValueError(
                f"tooth_t_block_patching_prob must be in [0, 1], got {self.tooth_t_block_patching_prob}"
            )

        assert self.num_stages == len(stride) + 1
        assert self.num_stages == len(enc_depths)
        assert self.num_stages == len(enc_channels)
        assert self.num_stages == len(enc_num_head)
        assert self.num_stages == len(enc_patch_size)
        assert self.cls_mode or self.num_stages == len(dec_depths) + 1
        assert self.cls_mode or self.num_stages == len(dec_channels) + 1
        assert self.cls_mode or self.num_stages == len(dec_num_head) + 1
        assert self.cls_mode or self.num_stages == len(dec_patch_size) + 1

        # norm layers
        if pdnorm_bn:
            bn_layer = partial(
                PDNorm,
                norm_layer=partial(
                    nn.BatchNorm1d, eps=1e-3, momentum=0.01, affine=pdnorm_affine
                ),
                conditions=pdnorm_conditions,
                decouple=pdnorm_decouple,
                adaptive=pdnorm_adaptive,
            )
        else:
            bn_layer = partial(nn.BatchNorm1d, eps=1e-3, momentum=0.01)
        if pdnorm_ln:
            ln_layer = partial(
                PDNorm,
                norm_layer=partial(nn.LayerNorm, elementwise_affine=pdnorm_affine),
                conditions=pdnorm_conditions,
                decouple=pdnorm_decouple,
                adaptive=pdnorm_adaptive,
            )
        else:
            ln_layer = nn.LayerNorm
        # activation layers
        act_layer = nn.GELU

        self.embedding = Embedding(
            in_channels=in_channels,
            embed_channels=enc_channels[0],
            norm_layer=bn_layer,
            act_layer=act_layer,
        )

        # encoder
        enc_drop_path = [
            x.item() for x in torch.linspace(0, drop_path, sum(enc_depths))
        ]
        self.enc = PointSequential()
        for s in range(self.num_stages):
            enc_drop_path_ = enc_drop_path[
                sum(enc_depths[:s]) : sum(enc_depths[: s + 1])
            ]
            enc = PointSequential()
            if s > 0:
                enc.add(
                    SerializedPooling(
                        in_channels=enc_channels[s - 1],
                        out_channels=enc_channels[s],
                        stride=stride[s - 1],
                        norm_layer=bn_layer,
                        act_layer=act_layer,
                    ),
                    name="down",
                )
            for i in range(enc_depths[s]):
                enc.add(
                    Block(
                        channels=enc_channels[s],
                        num_heads=enc_num_head[s],
                        patch_size=enc_patch_size[s],
                        mlp_ratio=mlp_ratio,
                        qkv_bias=qkv_bias,
                        qk_scale=qk_scale,
                        attn_drop=attn_drop,
                        proj_drop=proj_drop,
                        drop_path=enc_drop_path_[i],
                        norm_layer=ln_layer,
                        act_layer=act_layer,
                        pre_norm=pre_norm,
                        order_index=i % len(self.order),
                        cpe_indice_key=f"stage{s}",
                        enable_rpe=enable_rpe,
                        enable_flash=enable_flash,
                        upcast_attention=upcast_attention,
                        upcast_softmax=upcast_softmax,
                        use_tooth_t_symmetry=use_tooth_t_symmetry,
                        tooth_t_block_patching=tooth_t_block_patching,
                        tooth_t_block_patching_k=tooth_t_block_patching_k,
                        tooth_t_block_patching_prob=tooth_t_block_patching_prob,
                    ),
                    name=f"block{i}",
                )
            if len(enc) != 0:
                self.enc.add(module=enc, name=f"enc{s}")

        # decoder
        if not self.cls_mode:
            dec_drop_path = [
                x.item() for x in torch.linspace(0, drop_path, sum(dec_depths))
            ]
            self.dec = PointSequential()
            # 保存原始 decoder 每个 stage 的输出通道，用于在指定位置
            # 挂一个 coarse t 预测头。tooth_t_mid_stage=-1 表示 encoder bottleneck；
            # 0..len(dec)-1 表示对应 decoder stage 输出之后。
            self.dec_stage_channels = list(dec_channels)
            dec_channels = list(dec_channels) + [enc_channels[-1]]
            for s in reversed(range(self.num_stages - 1)):
                dec_drop_path_ = dec_drop_path[
                    sum(dec_depths[:s]) : sum(dec_depths[: s + 1])
                ]
                dec_drop_path_.reverse()
                dec = PointSequential()
                dec.add(
                    SerializedUnpooling(
                        in_channels=dec_channels[s + 1],
                        skip_channels=enc_channels[s],
                        out_channels=dec_channels[s],
                        norm_layer=bn_layer,
                        act_layer=act_layer,
                    ),
                    name="up",
                )
                for i in range(dec_depths[s]):
                    dec.add(
                        Block(
                            channels=dec_channels[s],
                            num_heads=dec_num_head[s],
                            patch_size=dec_patch_size[s],
                            mlp_ratio=mlp_ratio,
                            qkv_bias=qkv_bias,
                            qk_scale=qk_scale,
                            attn_drop=attn_drop,
                            proj_drop=proj_drop,
                            drop_path=dec_drop_path_[i],
                            norm_layer=ln_layer,
                            act_layer=act_layer,
                            pre_norm=pre_norm,
                            order_index=i % len(self.order),
                            cpe_indice_key=f"stage{s}",
                            enable_rpe=enable_rpe,
                            enable_flash=enable_flash,
                            upcast_attention=upcast_attention,
                            upcast_softmax=upcast_softmax,
                            use_tooth_t_symmetry=use_tooth_t_symmetry,
                            tooth_t_dynamic_gate=tooth_t_dynamic_gate,
                            tooth_t_dynamic_gate_hidden_dim=tooth_t_dynamic_gate_hidden_dim,
                            tooth_t_guided_patching=tooth_t_guided_patching,
                            tooth_t_guided_patching_prob=tooth_t_guided_patching_prob,
                            tooth_t_block_patching=tooth_t_block_patching,
                            tooth_t_block_patching_k=tooth_t_block_patching_k,
                            tooth_t_block_patching_prob=tooth_t_block_patching_prob,
                        ),
                        name=f"block{i}",
                    )
                self.dec.add(module=dec, name=f"dec{s}")

            # 在 encoder bottleneck 或 decoder 的某个 stage 之后预测 coarse t。
            # 默认 tooth_t_mid_stage=0，即第一个 decoder stage 输出后就产生 coarse t。
            if self.use_tooth_t_stage2:
                max_dec_stage = len(self.dec_stage_channels) - 1
                if self.tooth_t_mid_stage < -1 or self.tooth_t_mid_stage > max_dec_stage:
                    raise ValueError(
                        f"tooth_t_mid_stage must be -1 or in [0, {max_dec_stage}], "
                        f"got {self.tooth_t_mid_stage}"
                    )
                if self.tooth_t_mid_stage == -1:
                    mid_channels = enc_channels[-1]
                else:
                    mid_channels = self.dec_stage_channels[::-1][self.tooth_t_mid_stage]
                self.tooth_t_coarse_head = nn.Sequential(
                    nn.Linear(mid_channels, mid_channels),
                    nn.SiLU(),
                    nn.Linear(mid_channels, 1),
                    nn.Sigmoid(),
                )

    def _attach_tooth_t(self, point: Point, coarse_t: torch.Tensor):
        point["t_coarse_supervision"] = coarse_t
        if self.tooth_t_block_patching and self.tooth_t_block_patching_source == "pred":
            point["t_block_cond"] = coarse_t.detach()
        if self.tooth_t_inject_decoder:
            point["t_cond"] = coarse_t.detach() if self.tooth_t_detach_for_xcpe else coarse_t
        return point

    def forward(self, data_dict):
        """
        A data_dict is a dictionary containing properties of a batched point cloud.
        It should contain the following properties for PTv3:
        1. "feat": feature of point cloud
        2. "grid_coord": discrete coordinate after grid sampling (voxelization) or "coord" + "grid_size"
        3. "offset" or "batch": batch membership metadata.
        """
        point = Point(data_dict)
        if self.tooth_t_block_patching and self.tooth_t_block_patching_source in {"gt", "pred_t"}:
            if "t_block_cond" not in point.keys() or point.t_block_cond is None:
                raise RuntimeError(
                    f"tooth_t_block_patching_source='{self.tooth_t_block_patching_source}' "
                    "requires data_dict['t_block_cond']."
                )
            if point.t_block_cond.dim() == 1:
                point["t_block_cond"] = point.t_block_cond.unsqueeze(-1)
        point.serialization(order=self.order, shuffle_orders=self.shuffle_orders)
        point.sparsify()

        point = self.embedding(point)
        for i in range(len(self.enc)):
            point = self.enc[i](point)

        # 手动执行 decoder，便于在 encoder bottleneck 或中间 decoder stage
        # 产生 coarse t，并按需传给后续 decoder block 做 xCPE conditioning。
        if not self.cls_mode:
            coarse_t = None
            if self.use_tooth_t_stage2 and self.tooth_t_mid_stage == -1:
                coarse_t = self.tooth_t_coarse_head(point.feat)
                point = self._attach_tooth_t(point, coarse_t)

            for dec_idx in range(len(self.dec)):
                point = self.dec[dec_idx](point)
                if (
                    self.use_tooth_t_stage2
                    and coarse_t is None
                    and dec_idx == self.tooth_t_mid_stage
                ):
                    coarse_t = self.tooth_t_coarse_head(point.feat)
                    point = self._attach_tooth_t(point, coarse_t)

            if coarse_t is not None and "t_coarse_supervision" in point.keys():
                point["t_coarse"] = point.t_coarse_supervision
            elif coarse_t is not None and "t_cond" in point.keys():
                point["t_coarse"] = point.t_cond
            elif coarse_t is not None:
                point["t_coarse"] = coarse_t
        return point


```

## Source file: models/ptv3/ptv3_tooth_seg_network_arch_cac_final.py

{
  "path": "models/ptv3/ptv3_tooth_seg_network_arch_cac_final.py",
  "blob_sha": "fffb1888b457091c4eedfcf98479916358bc5395",
  "sha256": "2a340f22aed0a7e6d0153f64878335b94d925008947c7bdf2ced60176c41179a",
  "lines": 772,
  "bytes": 32557,
  "selection_score": 15,
  "keyword_line_counts": {
    "teeth": 3,
    "tooth": 60
  },
  "url": "https://github.com/blackbird2003/TeethSplineSeg/blob/238b74c26833703b281685a3f152f133ba49369c/models/ptv3/ptv3_tooth_seg_network_arch_cac_final.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def __init__("
    },
    {
      "line": 106,
      "text": "def get_pred(x: torch.Tensor, proto: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 111,
      "text": "def _project_point_features(self, feat: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 117,
      "text": "def _merge_teeth_only_logits(base_logits: torch.Tensor, cac_logits: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 122,
      "text": "def _compute_t_prior_gate(self, t_values: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 131,
      "text": "def _compute_slot_t_values(self, weights: torch.Tensor, t_values: torch.Tensor | None) -> torch.Tensor:"
    },
    {
      "line": 143,
      "text": "def _apply_tooth_slot_attention(self, proto: torch.Tensor, slot_t: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 187,
      "text": "def post_refine_proto_batch("
    },
    {
      "line": 218,
      "text": "def get_adaptive_perspective("
    },
    {
      "line": 268,
      "text": "def get_distill_loss("
    },
    {
      "line": 309,
      "text": "def forward(self, x, pos, target=None, return_aux=False, t_gt=None, t_block_cond=None):"
    },
    {
      "line": 377,
      "text": "def __init__("
    },
    {
      "line": 423,
      "text": "def _unpack_batch(self, batch):"
    },
    {
      "line": 437,
      "text": "def _prepare_targets(x, y, t):"
    },
    {
      "line": 445,
      "text": "def _select_t_block_cond(self, gt_t, pred_t):"
    },
    {
      "line": 458,
      "text": "def _compute_arch_loss(self, t_values, t_gt, y):"
    },
    {
      "line": 473,
      "text": "def _compute_arch_abs_error(self, t_values, t_gt, y):"
    },
    {
      "line": 488,
      "text": "def _extract_instances_from_labels_np(labels: np.ndarray) -> dict[int, np.ndarray]:"
    },
    {
      "line": 496,
      "text": "def _compute_tooth_miou_np(pred: np.ndarray, gt: np.ndarray) -> float:"
    },
    {
      "line": 512,
      "text": "def _match_instances_by_centroid_np("
    },
    {
      "line": 527,
      "text": "def _compute_tir_np(self, pred: np.ndarray, gt: np.ndarray, pos: np.ndarray, eps: float = 1e-12) -> float:"
    },
    {
      "line": 561,
      "text": "def _relabel_pred_instances_by_t_np(pred: np.ndarray, t_values: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 575,
      "text": "def _compute_tir_arch_np("
    },
    {
      "line": 587,
      "text": "def _compute_seg_quality_metrics_per_sample("
    },
    {
      "line": 623,
      "text": "def _shared_step(self, batch, stage=\"train\"):"
    },
    {
      "line": 708,
      "text": "def training_step(self, batch, batch_idx):"
    },
    {
      "line": 711,
      "text": "def validation_step(self, batch, batch_idx):"
    },
    {
      "line": 714,
      "text": "def test_step(self, batch, batch_idx):"
    },
    {
      "line": 717,
      "text": "def predict_labels(self, data, return_t=False):"
    },
    {
      "line": 745,
      "text": "def configure_optimizers(self):"
    }
  ]
}

```text

from torch import nn
import numpy as np
import torch
import torch.nn.functional as F
import torchmetrics as tm
import lightning as L
from scipy.optimize import linear_sum_assignment

from .ptv3_model_tooth import PointTransformerV3


class PTV3ToothSegmentationNetwork_Arch_CAC_Final(nn.Module):
    def __init__(
        self,
        num_classes=17,
        feature_dim=7,
        grid_size=0.01,
        ptv3_kwargs=None,
        dropout=0.0,
        cac_cos_temp=15.0,
        cac_conf_thresh=0.75,
        cac_t_prior_sigma=(2.0 / 17.0),
        cac_tooth_attention_heads=4,
    ):
        super().__init__()
        self.num_classes = int(num_classes)
        self.feature_dim = int(feature_dim)
        self.grid_size = float(grid_size)
        self.cac_cos_temp = float(cac_cos_temp)
        self.cac_conf_thresh = float(cac_conf_thresh)
        self.cac_t_prior_sigma = float(cac_t_prior_sigma)
        self.cac_tooth_attention_heads = int(cac_tooth_attention_heads)
        if self.cac_t_prior_sigma <= 0:
            raise ValueError(f"cac_t_prior_sigma must be > 0, got {self.cac_t_prior_sigma}")

        ptv3_kwargs = ptv3_kwargs or {}
        ptv3_kwargs.setdefault("use_tooth_t_stage2", True)
        self.backbone = PointTransformerV3(
            in_channels=self.feature_dim,
            cls_mode=False,
            **ptv3_kwargs,
        )

        backbone_out_dim = 64
        self.seg_proj = nn.Sequential(
            nn.Linear(backbone_out_dim, backbone_out_dim),
            nn.ReLU(inplace=True),
            nn.Dropout(dropout),
        )
        self.seg_classifier = nn.Linear(backbone_out_dim, self.num_classes)
        self.proj = nn.Sequential(
            nn.Linear(backbone_out_dim * 2, backbone_out_dim * 2, bias=False),
            nn.ReLU(inplace=True),
            nn.Linear(backbone_out_dim * 2, backbone_out_dim),
        )
        self.apd_proj = nn.Sequential(
            nn.Linear(backbone_out_dim * 2, backbone_out_dim * 2, bias=False),
            nn.ReLU(inplace=True),
            nn.Linear(backbone_out_dim * 2, backbone_out_dim),
        )
        self.feat_proj_layer = nn.Sequential(
            nn.Linear(backbone_out_dim, backbone_out_dim, bias=False),
            nn.BatchNorm1d(backbone_out_dim),
            nn.ReLU(inplace=True),
            nn.Linear(backbone_out_dim, backbone_out_dim),
        )

        if backbone_out_dim % self.cac_tooth_attention_heads != 0:
            raise ValueError(
                f"backbone_out_dim={backbone_out_dim} must be divisible by "
                f"cac_tooth_attention_heads={self.cac_tooth_attention_heads}"
            )
        self.cac_tooth_attention_scale = (backbone_out_dim // self.cac_tooth_attention_heads) ** -0.5
        self.tooth_attn_q = nn.Linear(backbone_out_dim, backbone_out_dim, bias=False)
        self.tooth_attn_k = nn.Linear(backbone_out_dim, backbone_out_dim, bias=False)
        self.tooth_attn_v = nn.Linear(backbone_out_dim, backbone_out_dim, bias=False)
        self.tooth_attn_out = nn.Linear(backbone_out_dim, backbone_out_dim, bias=False)
        self.tooth_attn_norm1 = nn.LayerNorm(backbone_out_dim)
        self.tooth_attn_ffn = nn.Sequential(
            nn.Linear(backbone_out_dim, backbone_out_dim * 2),
            nn.ReLU(inplace=True),
            nn.Dropout(dropout),
            nn.Linear(backbone_out_dim * 2, backbone_out_dim),
        )
        self.tooth_attn_norm2 = nn.LayerNorm(backbone_out_dim)
        self.tooth_attn_dropout = nn.Dropout(dropout)
        self.tooth_relation_bias = nn.Sequential(
            nn.Linear(5, 32),
            nn.ReLU(inplace=True),
            nn.Linear(32, self.cac_tooth_attention_heads),
        )

        class_centers = torch.zeros((self.num_classes,), dtype=torch.float32)
        tooth_slot_ids = torch.zeros((self.num_classes,), dtype=torch.float32)
        tooth_class_count = self.num_classes - 1
        denominator = float(self.num_classes)
        for cls_id in range(1, self.num_classes):
            tooth_id = cls_id if cls_id <= tooth_class_count else cls_id - tooth_class_count
            class_centers[cls_id] = float(tooth_id) / denominator
            tooth_slot_ids[cls_id] = float(tooth_id)
        self.register_buffer("cac_t_prior_centers", class_centers, persistent=False)
        self.register_buffer("cac_tooth_slot_ids", tooth_slot_ids, persistent=False)


    @staticmethod
    def get_pred(x: torch.Tensor, proto: torch.Tensor) -> torch.Tensor:
        x = F.normalize(x, p=2, dim=1)
        proto = F.normalize(proto, p=2, dim=1)
        return x @ proto.transpose(0, 1)

    def _project_point_features(self, feat: torch.Tensor) -> torch.Tensor:
        b, n, c = feat.shape
        projected = self.feat_proj_layer(feat.reshape(b * n, c))
        return projected.reshape(b, n, c)

    @staticmethod
    def _merge_teeth_only_logits(base_logits: torch.Tensor, cac_logits: torch.Tensor) -> torch.Tensor:
        merged_logits = base_logits.clone()
        merged_logits[..., 1:] = cac_logits[..., 1:]
        return merged_logits

    def _compute_t_prior_gate(self, t_values: torch.Tensor) -> torch.Tensor:
        t_values = t_values.detach().float()
        centers = self.cac_t_prior_centers.to(device=t_values.device, dtype=t_values.dtype).unsqueeze(1)
        normalized_delta = (t_values.unsqueeze(0) - centers) / self.cac_t_prior_sigma
        gate = torch.exp(-0.5 * normalized_delta.pow(2))
        gingiva_gate = torch.ones_like(gate[:1])
        gate = torch.cat([gingiva_gate, gate[1:]], dim=0)
        return gate

    def _compute_slot_t_values(self, weights: torch.Tensor, t_values: torch.Tensor | None) -> torch.Tensor:
        centers = self.cac_t_prior_centers.to(device=weights.device, dtype=weights.dtype)
        if t_values is None:
            return centers
        t_values = t_values.detach().to(device=weights.device, dtype=weights.dtype)
        denom = weights.sum(dim=-1)
        weighted_t = (weights @ t_values) / (denom + 1e-7)
        valid = denom > 1e-7
        slot_t = torch.where(valid, weighted_t, centers)
        slot_t = torch.cat([centers[:1], slot_t[1:]], dim=0)
        return slot_t

    def _apply_tooth_slot_attention(self, proto: torch.Tensor, slot_t: torch.Tensor) -> torch.Tensor:
        if proto.shape[0] <= 2:
            return proto

        tooth_proto = proto[1:]
        slot_t = slot_t[1:].to(device=tooth_proto.device, dtype=tooth_proto.dtype)
        tooth_ids = self.cac_tooth_slot_ids[1:1 + tooth_proto.shape[0]].to(
            device=tooth_proto.device,
            dtype=tooth_proto.dtype,
        )
        tooth_count, hidden_dim = tooth_proto.shape
        num_heads = self.cac_tooth_attention_heads
        head_dim = hidden_dim // num_heads

        q = self.tooth_attn_q(tooth_proto).reshape(tooth_count, num_heads, head_dim).permute(1, 0, 2)
        k = self.tooth_attn_k(tooth_proto).reshape(tooth_count, num_heads, head_dim).permute(1, 0, 2)
        v = self.tooth_attn_v(tooth_proto).reshape(tooth_count, num_heads, head_dim).permute(1, 0, 2)

        abs_t_delta = torch.abs(slot_t[:, None] - slot_t[None, :])
        neighbor_t_delta = torch.abs(abs_t_delta - (1.0 / float(self.num_classes)))
        symmetry_t_delta = torch.abs((slot_t[:, None] + slot_t[None, :]) - 1.0)
        tooth_id_delta = torch.abs(tooth_ids[:, None] - tooth_ids[None, :])
        adjacent_hint = (tooth_id_delta == 1).to(tooth_proto.dtype)
        symmetry_hint = torch.zeros_like(adjacent_hint)
        adjacent_hint = torch.zeros_like(adjacent_hint)
        relation_features = torch.stack(
            [abs_t_delta, neighbor_t_delta, symmetry_t_delta, adjacent_hint, symmetry_hint],
            dim=-1,
        )
        relation_bias = self.tooth_relation_bias(relation_features).permute(2, 0, 1)

        attn_scores = torch.matmul(q, k.transpose(-2, -1)) * self.cac_tooth_attention_scale
        attn_scores = attn_scores + relation_bias
        attn_weights = F.softmax(attn_scores, dim=-1)
        attn_weights = self.tooth_attn_dropout(attn_weights)

        attn_output = torch.matmul(attn_weights, v).permute(1, 0, 2).reshape(tooth_count, hidden_dim)
        tooth_proto = self.tooth_attn_norm1(tooth_proto + self.tooth_attn_out(attn_output))
        tooth_proto = self.tooth_attn_norm2(tooth_proto + self.tooth_attn_ffn(tooth_proto))

        updated_proto = proto.clone()
        updated_proto[1:] = tooth_proto
        return updated_proto

    def post_refine_proto_batch(
        self,
        feat: torch.Tensor,
        pred: torch.Tensor,
        proto: torch.Tensor,
        t_values: torch.Tensor | None = None,
    ) -> torch.Tensor:
        raw_feat = feat
        raw_pred = pred.detach()
        projected_feat = self._project_point_features(raw_feat)
        refine_logits = []

        t_iter = [None] * len(raw_feat) if t_values is None else t_values
        for raw_feat_b, projected_feat_b, pred_b, t_b in zip(raw_feat, projected_feat, raw_pred, t_iter):
            pred_prob = F.softmax(pred_b, dim=-1).transpose(0, 1)
            if self.cac_conf_thresh > 0:
                keep_mask = (pred_prob.max(dim=0).values >= self.cac_conf_thresh).float().unsqueeze(0)
                pred_prob = pred_prob * keep_mask
            if t_b is None:
                raise RuntimeError("The final model requires the backbone's intermediate t prediction.")
            pred_prob = pred_prob * self._compute_t_prior_gate(t_b).to(pred_prob.dtype)

            denom = pred_prob.sum(dim=-1, keepdim=True) + 1e-7
            pred_proto = (pred_prob / denom) @ raw_feat_b
            pred_proto = self.proj(torch.cat([pred_proto, proto], dim=-1))
            pred_slot_t = self._compute_slot_t_values(pred_prob, t_b)
            pred_proto = self._apply_tooth_slot_attention(pred_proto, pred_slot_t)
            refine_logits.append(self.get_pred(projected_feat_b, pred_proto))

        return torch.stack(refine_logits, dim=0) * self.cac_cos_temp

    def get_adaptive_perspective(
        self,
        feat: torch.Tensor,
        target: torch.Tensor,
        new_proto: torch.Tensor,
        proto: torch.Tensor,
        t_values: torch.Tensor | None = None,
    ) -> torch.Tensor:
        raw_feat = feat
        projected_feat = self._project_point_features(raw_feat)
        oracle_logits = []

        t_iter = [None] * len(raw_feat) if t_values is None else t_values
        for raw_feat_b, projected_feat_b, target_b, t_b in zip(raw_feat, projected_feat, target, t_iter):
            sample_proto = new_proto.clone()
            oracle_weights = torch.zeros(
                (sample_proto.shape[0], target_b.shape[0]),
                device=raw_feat_b.device,
                dtype=raw_feat_b.dtype,
            )
            if t_b is None:
                raise RuntimeError("The final model requires the backbone's intermediate t prediction.")
            t_gate = self._compute_t_prior_gate(t_b).to(raw_feat_b.dtype)

            for cls_id in target_b.unique():
                cls_value = int(cls_id.item())
                if cls_value < 0:
                    continue
                cls_mask = target_b == cls_value
                if not torch.any(cls_mask):
                    continue
                cls_weights = t_gate[cls_value, cls_mask]
                weight_sum = cls_weights.sum()
                if float(weight_sum) <= 1e-7:
                    sample_proto[cls_value] = raw_feat_b[cls_mask].mean(dim=0)
                    oracle_weights[cls_value, cls_mask] = 1.0
                else:
                    sample_proto[cls_value] = (
                        raw_feat_b[cls_mask] * cls_weights.unsqueeze(-1)
                    ).sum(dim=0) / weight_sum
                    oracle_weights[cls_value, cls_mask] = cls_weights

            sample_proto = self.apd_proj(torch.cat([sample_proto, proto], dim=-1))
            sample_slot_t = self._compute_slot_t_values(oracle_weights, t_b)
            sample_proto = self._apply_tooth_slot_attention(sample_proto, sample_slot_t)
            oracle_logits.append(self.get_pred(projected_feat_b, sample_proto))

        return torch.stack(oracle_logits, dim=0) * self.cac_cos_temp

    @staticmethod
    def get_distill_loss(
        pred: torch.Tensor,
        soft: torch.Tensor,
        target: torch.Tensor,
        smoothness: float = 0.5,
        eps: float = 0.0,
    ) -> torch.Tensor:
        pred = pred.reshape(-1, pred.shape[-1])
        soft = soft.detach().reshape(-1, soft.shape[-1])
        target = target.reshape(-1)
        device = pred.device

        n, c = soft.shape
        onehot_index = target.unsqueeze(-1)
        ignore_mask = (onehot_index == -1).float()
        sm_soft = F.softmax(soft, dim=1)

        onehot_index = onehot_index * (1 - ignore_mask.long())
        onehot = torch.zeros(n, c, device=device, dtype=pred.dtype).scatter_(1, onehot_index.long(), 1)
        smoothed_label = smoothness * sm_soft + (1 - smoothness) * onehot
        if eps > 0:
            smoothed_label = smoothed_label * (1 - eps) + (1 - smoothed_label) * eps / (c - 1)

        loss = (-F.log_softmax(pred, dim=1) * smoothed_label).sum(dim=1)
        entropy_mask = -(sm_soft * torch.log(sm_soft + 1e-4)).sum(dim=1)

        valid_mask = (target != -1).float()
        entropy_mask = entropy_mask * valid_mask
        unique_classes = [int(cls.item()) for cls in target.unique() if int(cls.item()) != -1]

        loss_list = []
        for cls_value in unique_classes:
            cls_mask = (target == cls_value).float()
            cls_entropy = entropy_mask * cls_mask
            cls_loss = (loss * cls_entropy).sum() / (cls_entropy.sum() + 1e-4)
            loss_list.append(cls_loss)

        if not loss_list:
            return pred.new_tensor(0.0)
        return torch.stack(loss_list).mean()

    def forward(self, x, pos, target=None, return_aux=False, t_gt=None, t_block_cond=None):
        assert x.dim() == 3 and pos.dim() == 3
        b, n, c_in = x.shape
        assert c_in >= self.feature_dim, f"Expected input feature dim >= {self.feature_dim}, got {c_in}"
        if c_in > self.feature_dim:
            x = x[:, :, :self.feature_dim]

        device = x.device
        x = x.float()
        pos = pos.float()
        feat = x.reshape(b * n, self.feature_dim)
        coord = pos.reshape(b * n, 3)
        batch = torch.arange(b, device=device, dtype=torch.long).repeat_interleave(n)

        data_dict = {
            "feat": feat,
            "coord": coord,
            "batch": batch,
            "grid_size": float(self.grid_size),
        }
        if t_block_cond is None:
            t_block_cond = t_gt
        if t_block_cond is not None:
            data_dict["t_block_cond"] = t_block_cond.reshape(b * n, 1).float()

        point = self.backbone(data_dict)
        point_feat = point.feat
        t_coarse = point.get("t_coarse", None)
        if t_coarse is None:
            raise RuntimeError("The final model requires the backbone's intermediate t prediction.")

        feat_for_cac = self.seg_proj(point_feat)
        base_logits = self.seg_classifier(feat_for_cac)
        feat_for_cac = feat_for_cac.reshape(b, n, -1)
        base_logits = base_logits.reshape(b, n, self.num_classes)
        t_coarse_batched = None if t_coarse is None else t_coarse.squeeze(-1).reshape(b, n)

        refine_logits = self.post_refine_proto_batch(
            feat=feat_for_cac,
            pred=base_logits,
            proto=self.seg_classifier.weight,
            t_values=t_coarse_batched,
        )
        refine_logits = self._merge_teeth_only_logits(base_logits, refine_logits)

        oracle_logits = None
        if target is not None:
            oracle_logits = self.get_adaptive_perspective(
                feat=feat_for_cac,
                target=target.long(),
                new_proto=self.seg_classifier.weight.detach().clone(),
                proto=self.seg_classifier.weight,
                t_values=t_coarse_batched,
            )
            oracle_logits = self._merge_teeth_only_logits(base_logits, oracle_logits)

        if return_aux:
            return {
                "base_logits": base_logits,
                "refine_logits": refine_logits,
                "oracle_logits": oracle_logits,
                "t_coarse": t_coarse_batched,
                "feat_for_cac": feat_for_cac,
            }
        return refine_logits, t_coarse_batched, t_coarse_batched


class LitPTV3ToothSegmentationNetwork_Arch_CAC_Final(L.LightningModule):
    def __init__(
        self,
        grid_size=0.01,
        feature_dim=7,
        num_classes=17,
        ptv3_kwargs=None,
        coarse_arch_loss_weight=100.0,
        gingiva_ignore_for_pos=True,
        arch_loss_type="smooth_l1",
        dropout=0.0,
        cac_main_weight=1.0,
        cac_pre_weight=1.0,
        cac_pre_self_weight=1.0,
        cac_kl_weight=0.5,
        cac_cos_temp=15.0,
        cac_conf_thresh=0.75,
        cac_t_prior_sigma=(2.0 / 17.0),
        cac_tooth_attention_heads=4,
    ):
        super().__init__()
        self.model = PTV3ToothSegmentationNetwork_Arch_CAC_Final(
            num_classes=num_classes,
            feature_dim=feature_dim,
            grid_size=grid_size,
            ptv3_kwargs=ptv3_kwargs,
            dropout=dropout,
            cac_cos_temp=cac_cos_temp,
            cac_conf_thresh=cac_conf_thresh,
            cac_t_prior_sigma=cac_t_prior_sigma,
            cac_tooth_attention_heads=cac_tooth_attention_heads,
        )

        self.seg_loss = nn.CrossEntropyLoss()
        if arch_loss_type == "mse":
            self.arch_loss_fn = nn.MSELoss(reduction="none")
        else:
            self.arch_loss_fn = nn.SmoothL1Loss(reduction="none")

        self.train_acc = tm.Accuracy(task="multiclass", num_classes=num_classes)
        self.val_acc = tm.Accuracy(task="multiclass", num_classes=num_classes)
        self.test_acc = tm.Accuracy(task="multiclass", num_classes=num_classes)
        self.train_miou = tm.JaccardIndex(task="multiclass", num_classes=num_classes)
        self.val_miou = tm.JaccardIndex(task="multiclass", num_classes=num_classes)
        self.test_miou = tm.JaccardIndex(task="multiclass", num_classes=num_classes)
        self.save_hyperparameters(ignore=["unused_hparams"])

    def _unpack_batch(self, batch):
        pred_t = None
        if len(batch) == 3:
            pos, x, y = batch
            t = None
        elif len(batch) == 4:
            pos, x, y, t = batch
        elif len(batch) == 5:
            pos, x, y, t, pred_t = batch
        else:
            raise ValueError(f"Unexpected batch format, expected 3, 4, or 5 items, got {len(batch)}")
        return pos, x, y, t, pred_t

    @staticmethod
    def _prepare_targets(x, y, t):
        b, n, _ = x.shape
        x = x.float()
        y = y.view(b, n).float()
        if t is not None:
            t = t.view(b, n).float().clamp(0.0, 1.0)
        return x, y, t

    def _select_t_block_cond(self, gt_t, pred_t):
        backbone = getattr(self.model, "backbone", None)
        if backbone is None or not getattr(backbone, "tooth_t_block_patching", False):
            return None
        source = getattr(backbone, "tooth_t_block_patching_source", "pred")
        if source == "gt":
            return gt_t
        if source == "pred_t":
            if pred_t is None:
                raise RuntimeError("tooth_t_block_patching_source='pred_t' requires pred_t in the dataset.")
            return pred_t
        return None

    def _compute_arch_loss(self, t_values, t_gt, y):
        if t_gt is None or t_values is None:
            return y.new_tensor(0.0, dtype=torch.float32)

        if self.hparams.gingiva_ignore_for_pos:
            valid_mask = y.long() > 0
        else:
            valid_mask = torch.ones_like(y, dtype=torch.bool)

        if valid_mask.sum() == 0:
            return t_values.new_tensor(0.0)

        loss_map = self.arch_loss_fn(t_values, t_gt)
        return loss_map[valid_mask].mean()

    def _compute_arch_abs_error(self, t_values, t_gt, y):
        if t_gt is None or t_values is None:
            return y.new_tensor(0.0, dtype=torch.float32)

        if self.hparams.gingiva_ignore_for_pos:
            valid_mask = y.long() > 0
        else:
            valid_mask = torch.ones_like(y, dtype=torch.bool)

        if valid_mask.sum() == 0:
            return t_values.new_tensor(0.0)

        return torch.abs(t_values - t_gt)[valid_mask].mean()

    @staticmethod
    def _extract_instances_from_labels_np(labels: np.ndarray) -> dict[int, np.ndarray]:
        inst_dict = {}
        for c in np.unique(labels):
            if c > 0:
                inst_dict[int(c)] = np.where(labels == c)[0]
        return inst_dict

    @staticmethod
    def _compute_tooth_miou_np(pred: np.ndarray, gt: np.ndarray) -> float:
        gt_labels = np.unique(gt)
        gt_labels = gt_labels[gt_labels > 0]
        if len(gt_labels) == 0:
            return 0.0

        ious = []
        for c in gt_labels:
            g = gt == c
            p = pred == c
            inter = np.sum(g & p)
            union = np.sum(g | p)
            ious.append(float(inter / union) if union else 0.0)
        return float(np.mean(ious)) if ious else 0.0

    @staticmethod
    def _match_instances_by_centroid_np(
        gt_dict: dict[int, dict],
        pred_dict: dict[int, dict],
    ) -> dict[int, int]:
        gt_keys = list(gt_dict.keys())
        pred_keys = list(pred_dict.keys())
        if not gt_keys or not pred_keys:
            return {}

        gt_cent = np.stack([gt_dict[k]["centroid"] for k in gt_keys], axis=0)
        pred_cent = np.stack([pred_dict[k]["centroid"] for k in pred_keys], axis=0)
        dist = np.linalg.norm(gt_cent[:, None, :] - pred_cent[None, :, :], axis=2)
        row_ind, col_ind = linear_sum_assignment(dist)
        return {gt_keys[i]: pred_keys[j] for i, j in zip(row_ind, col_ind)}

    def _compute_tir_np(self, pred: np.ndarray, gt: np.ndarray, pos: np.ndarray, eps: float = 1e-12) -> float:
        gt_inst = self._extract_instances_from_labels_np(gt)
        pred_inst = self._extract_instances_from_labels_np(pred)

        gt_dict = {}
        for k, idx in gt_inst.items():
            pts = pos[idx]
            centroid = pts.mean(axis=0)
            tooth_size = np.sqrt(np.sum((centroid - pts) ** 2, axis=0))
            gt_dict[k] = {
                "label": k,
                "centroid": centroid,
                "tooth_size": tooth_size,
            }

        pred_dict = {}
        for k, idx in pred_inst.items():
            pts = pos[idx]
            pred_dict[k] = {
                "label": k,
                "centroid": pts.mean(axis=0),
            }

        matching = self._match_instances_by_centroid_np(gt_dict, pred_dict)
        tir = 0
        for gt_k, pred_k in matching.items():
            info = gt_dict[gt_k]
            den = np.where(np.abs(info["tooth_size"]) < eps, eps, info["tooth_size"])
            dist = np.linalg.norm((info["centroid"] - pred_dict[pred_k]["centroid"]) / den)
            if dist < 0.5 and gt_k == pred_k:
                tir += 1
        return float(tir / max(1, len(matching)))

    @staticmethod
    def _relabel_pred_instances_by_t_np(pred: np.ndarray, t_values: np.ndarray) -> np.ndarray:
        relabeled = pred.copy()
        anchors = np.arange(1, 17, dtype=np.float32) / 17.0
        pred_labels = np.unique(pred)
        pred_labels = pred_labels[pred_labels > 0]
        for label in pred_labels:
            mask = pred == label
            if not np.any(mask):
                continue
            t_avg = float(np.mean(t_values[mask]))
            new_label = int(np.argmin(np.abs(anchors - t_avg)) + 1)
            relabeled[mask] = new_label
        return relabeled

    def _compute_tir_arch_np(
        self,
        pred: np.ndarray,
        gt: np.ndarray,
        pos: np.ndarray,
        t_values: np.ndarray | None,
    ) -> float:
        if t_values is None:
            return 0.0
        pred_arch = self._relabel_pred_instances_by_t_np(pred, t_values)
        return self._compute_tir_np(pred_arch, gt, pos)

    def _compute_seg_quality_metrics_per_sample(
        self,
        seg_logits: torch.Tensor,
        y: torch.Tensor,
        pos: torch.Tensor,
        t_values: torch.Tensor | None = None,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        pred = torch.argmax(seg_logits, dim=-1).detach().cpu().numpy()
        gt = y.detach().cpu().numpy()
        pos_np = pos.detach().cpu().numpy()
        t_np = None if t_values is None else t_values.detach().cpu().numpy()

        tooth_mious = []
        tirs = []
        tir_archs = []
        batch_size = pred.shape[0]
        for b in range(batch_size):
            tooth_mious.append(self._compute_tooth_miou_np(pred[b], gt[b]))
            tirs.append(self._compute_tir_np(pred[b], gt[b], pos_np[b]))
            tir_archs.append(
                self._compute_tir_arch_np(
                    pred[b],
                    gt[b],
                    pos_np[b],
                    None if t_np is None else t_np[b],
                )
            )

        device = seg_logits.device
        dtype = seg_logits.dtype
        return (
            torch.tensor(tooth_mious, device=device, dtype=dtype),
            torch.tensor(tirs, device=device, dtype=dtype),
            torch.tensor(tir_archs, device=device, dtype=dtype),
        )

    def _shared_step(self, batch, stage="train"):
        pos, x, y, t, pred_t = self._unpack_batch(batch)
        x, y, t = self._prepare_targets(x, y, t)
        if pred_t is not None:
            pred_t = pred_t.view(x.shape[0], x.shape[1]).float().clamp(0.0, 1.0)
        pos = pos.float()
        target = y.long() if stage == "train" else None

        t_block_cond = self._select_t_block_cond(t, pred_t)
        outputs = self.model(x, pos, target=target, return_aux=True, t_gt=t, t_block_cond=t_block_cond)
        base_logits = outputs["base_logits"]
        refine_logits = outputs["refine_logits"]
        oracle_logits = outputs["oracle_logits"]
        t_coarse = outputs["t_coarse"]

        refine_logits_ch_first = refine_logits.transpose(2, 1)
        base_logits_ch_first = base_logits.transpose(2, 1)
        target_long = y.long()

        refine_seg_loss = self.seg_loss(refine_logits_ch_first, target_long) * self.hparams.cac_main_weight
        pre_self_loss = self.seg_loss(base_logits_ch_first, target_long) * self.hparams.cac_pre_self_weight
        if oracle_logits is not None:
            oracle_logits_ch_first = oracle_logits.transpose(2, 1)
            pre_loss = self.seg_loss(oracle_logits_ch_first, target_long) * self.hparams.cac_pre_weight
            kl_loss = (
                self.model.get_distill_loss(refine_logits, oracle_logits.detach(), target_long)
                * self.hparams.cac_kl_weight
            )
        else:
            pre_loss = refine_logits.new_tensor(0.0)
            kl_loss = refine_logits.new_tensor(0.0)

        coarse_arch_loss = (
            self._compute_arch_loss(t_coarse, t, y) * self.hparams.coarse_arch_loss_weight
            if t_coarse is not None
            else refine_logits.new_tensor(0.0)
        )
        coarse_t_abs_error = self._compute_arch_abs_error(t_coarse, t, y)
        coarse_t_error_rate = coarse_t_abs_error * 100.0

        total_loss = refine_seg_loss + pre_self_loss + pre_loss + kl_loss + coarse_arch_loss

        prefix = "train" if stage == "train" else "val" if stage == "val" else "test"
        metric_acc = self.train_acc if stage == "train" else self.val_acc if stage == "val" else self.test_acc
        metric_miou = self.train_miou if stage == "train" else self.val_miou if stage == "val" else self.test_miou
        metric_acc(refine_logits_ch_first, target_long)
        metric_miou(refine_logits_ch_first, target_long)

        t_miou_per_sample, tir_per_sample, tir_arch_per_sample = self._compute_seg_quality_metrics_per_sample(
            refine_logits,
            target_long,
            pos,
            t_coarse,
        )
        base_t_miou_per_sample, base_tir_per_sample, _ = self._compute_seg_quality_metrics_per_sample(
            base_logits,
            target_long,
            pos,
            t_coarse,
        )
        t_miou = t_miou_per_sample.mean()
        tir = tir_per_sample.mean()
        tir_arch = tir_arch_per_sample.mean()
        base_t_miou = base_t_miou_per_sample.mean()
        base_tir = base_tir_per_sample.mean()

        self.log(f"{prefix}_acc", metric_acc, prog_bar=True, on_step=False, on_epoch=True)
        self.log(f"{prefix}_miou", metric_miou, prog_bar=True, on_step=False, on_epoch=True)
        self.log(f"{prefix}_seg_loss", refine_seg_loss, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_base_seg_loss", pre_self_loss, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_cac_pre_loss", pre_loss, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_cac_kl_loss", kl_loss, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_coarse_arch_loss", coarse_arch_loss, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_coarse_t_abs_error", coarse_t_abs_error, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_coarse_t_error_rate", coarse_t_error_rate, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_t_miou", t_miou, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_tir", tir, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_tir_arch", tir_arch, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_base_t_miou", base_t_miou, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_base_tir", base_tir, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_cac_delta_t_miou", t_miou - base_t_miou, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_cac_delta_tir", tir - base_tir, prog_bar=False, on_step=False, on_epoch=True)
        self.log(f"{prefix}_loss", total_loss, prog_bar=True, on_step=False, on_epoch=True)
        return total_loss

    def training_step(self, batch, batch_idx):
        return self._shared_step(batch, stage="train")

    def validation_step(self, batch, batch_idx):
        return self._shared_step(batch, stage="val")

    def test_step(self, batch, batch_idx):
        return self._shared_step(batch, stage="test")

    def predict_labels(self, data, return_t=False):
        device = next(self.parameters()).device
        with torch.no_grad():
            t_gt = None
            pred_t = None
            if len(data) == 2:
                pos, x = data
            elif len(data) == 3:
                pos, x, _ = data
            elif len(data) == 4:
                pos, x, _, t_gt = data
            elif len(data) == 5:
                pos, x, _, t_gt, pred_t = data
            else:
                raise ValueError(f"Unexpected predict input length: {len(data)}")

            pos = pos.unsqueeze(0).to(device)
            x = x.unsqueeze(0).to(device).float()
            t_gt = None if t_gt is None else t_gt.unsqueeze(0).to(device).float()
            pred_t = None if pred_t is None else pred_t.unsqueeze(0).to(device).float()
            t_block_cond = self._select_t_block_cond(t_gt, pred_t)
            seg_logits, position_param, _ = self.model(x, pos, t_gt=t_gt, t_block_cond=t_block_cond)
            pred = torch.argmax(seg_logits.transpose(2, 1), dim=1).squeeze(0)

            if return_t:
                return pred, position_param.squeeze(0)
            return pred

    def configure_optimizers(self):
        optimizer = torch.optim.Adam(
            self.parameters(),
            lr=1e-3,
            betas=(0.9, 0.999),
            weight_decay=1e-5,
        )

        warmup_scheduler = torch.optim.lr_scheduler.LinearLR(
            optimizer,
            start_factor=0.1,
            end_factor=1.0,
            total_iters=5,
        )
        cosine_scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
            optimizer,
            T_max=95,
            eta_min=1e-5,
        )
        sch = torch.optim.lr_scheduler.SequentialLR(
            optimizer,
            schedulers=[warmup_scheduler, cosine_scheduler],
            milestones=[5],
        )
        return {
            "optimizer": optimizer,
            "lr_scheduler": {"scheduler": sch},
        }


```

## Source file: bspline.py

{
  "path": "bspline.py",
  "blob_sha": "824cd3c6281088cf898bcce4a01e6dbe944290b5",
  "sha256": "9cabf44a07a8176ba79de82f5cdeb10a24f238497eae0f125fa11032dd4b9ccd",
  "lines": 114,
  "bytes": 3488,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 1,
    "tooth": 2,
    "rig": 2
  },
  "url": "https://github.com/blackbird2003/TeethSplineSeg/blob/238b74c26833703b281685a3f152f133ba49369c/bspline.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def compute_centroids_with_extension("
    },
    {
      "line": 60,
      "text": "def interpolate_bspline(params: np.ndarray, centroids: np.ndarray):"
    },
    {
      "line": 64,
      "text": "def sample_curve_with_params(spline, num_samples: int = 2000):"
    },
    {
      "line": 71,
      "text": "def compute_vertex_parameters("
    },
    {
      "line": 84,
      "text": "def build_vertex_arch_parameters("
    },
    {
      "line": 94,
      "text": "def ensure_arch_parameter_file("
    }
  ]
}

```text

"""Tooth-centroid-anchored B-spline arch parameterization."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from scipy.interpolate import make_interp_spline
from scipy.spatial import cKDTree


def compute_centroids_with_extension(
    vertices: np.ndarray,
    labels: np.ndarray,
) -> tuple[np.ndarray, np.ndarray]:
    """Build tooth-centroid anchors and two half-slot endpoint anchors."""
    existing_ids = [label for label in range(1, 17) if np.any(labels == label)]
    if len(existing_ids) < 4:
        raise ValueError("At least four teeth are required to fit a cubic B-spline")

    centroids = np.stack(
        [vertices[labels == label].mean(axis=0) for label in existing_ids],
        axis=0,
    )
    params = np.asarray(existing_ids, dtype=np.float64) / 17.0

    first_label = existing_ids[0]
    last_label = existing_ids[-1]
    first_points = vertices[labels == first_label]
    last_points = vertices[labels == last_label]
    left_point = np.array(
        [
            first_points[:, 0].mean(),
            first_points[:, 1].max(),
            first_points[:, 2].mean(),
        ],
        dtype=np.float64,
    )
    right_point = np.array(
        [
            last_points[:, 0].mean(),
            last_points[:, 1].max(),
            last_points[:, 2].mean(),
        ],
        dtype=np.float64,
    )

    params = np.concatenate(
        (
            [(first_label - 0.5) / 17.0],
            params,
            [(last_label + 0.5) / 17.0],
        )
    )
    centroids = np.vstack((left_point, centroids, right_point))
    order = np.argsort(params)
    return params[order], centroids[order]


def interpolate_bspline(params: np.ndarray, centroids: np.ndarray):
    return make_interp_spline(params, centroids, k=3)


def sample_curve_with_params(spline, num_samples: int = 2000):
    t_min = spline.t[spline.k]
    t_max = spline.t[-spline.k - 1]
    t_values = np.linspace(t_min, t_max, num_samples)
    return t_values, spline(t_values)


def compute_vertex_parameters(
    vertices: np.ndarray,
    spline,
    num_curve_samples: int = 2000,
) -> np.ndarray:
    t_samples, curve_samples = sample_curve_with_params(
        spline,
        num_samples=num_curve_samples,
    )
    nearest_curve_indices = cKDTree(curve_samples).query(vertices, k=1)[1]
    return t_samples[nearest_curve_indices].astype(np.float32, copy=False)


def build_vertex_arch_parameters(
    vertices: np.ndarray,
    labels: np.ndarray,
    num_curve_samples: int = 2000,
) -> np.ndarray:
    params, centroids = compute_centroids_with_extension(vertices, labels)
    spline = interpolate_bspline(params, centroids)
    return compute_vertex_parameters(vertices, spline, num_curve_samples)


def ensure_arch_parameter_file(
    vertices: np.ndarray,
    labels: np.ndarray,
    output_path: Path,
    overwrite: bool = False,
) -> np.ndarray:
    """Load an existing t target or construct and persist it when absent."""
    if output_path.exists() and not overwrite:
        values = np.asarray(
            np.loadtxt(output_path, dtype=np.float32)
        ).reshape(-1)
        if len(values) != len(vertices):
            raise ValueError(
                f"Vertex/arch-parameter count mismatch in {output_path}: "
                f"{len(vertices)} vs {len(values)}"
            )
        return values

    values = build_vertex_arch_parameters(vertices, labels)
    np.savetxt(output_path, values, fmt="%.8f")
    return values


```

## Source file: evaluate.py

{
  "path": "evaluate.py",
  "blob_sha": "ed362d5462b16491df6aa252b08f23068e80c5a2",
  "sha256": "f77778dbfa5f4433053d5ccecb3a99da280e95e8595145ee38ec65e75c15749a",
  "lines": 249,
  "bytes": 10600,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 3,
    "tooth": 12,
    "gum": 8,
    "rig": 2
  },
  "url": "https://github.com/blackbird2003/TeethSplineSeg/blob/238b74c26833703b281685a3f152f133ba49369c/evaluate.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def load_obj_vertices(path: Path) -> np.ndarray:"
    },
    {
      "line": 35,
      "text": "def load_ground_truth(path: Path) -> np.ndarray:"
    },
    {
      "line": 44,
      "text": "def load_prediction(path: Path, pred_format: str) -> np.ndarray:"
    },
    {
      "line": 52,
      "text": "def align_prediction(prediction: np.ndarray, ground_truth: np.ndarray) -> np.ndarray:"
    },
    {
      "line": 61,
      "text": "def boundary_vertices(vertices: np.ndarray, labels: np.ndarray, neighbors: int) -> np.ndarray:"
    },
    {
      "line": 70,
      "text": "def basic_metrics(prediction: np.ndarray, ground_truth: np.ndarray, vertices: np.ndarray, neighbors: int) -> dict[str, float]:"
    },
    {
      "line": 98,
      "text": "def tooth_side(label: int) -> str:"
    },
    {
      "line": 106,
      "text": "def tir_metric(prediction: np.ndarray, ground_truth: np.ndarray, vertices: np.ndarray, eps: float = 1e-12) -> float:"
    },
    {
      "line": 142,
      "text": "def first_match(root: Path, name: str, suffix: str) -> Path | None:"
    },
    {
      "line": 147,
      "text": "def evaluate_case(task: tuple[str, str, str, str, int]) -> tuple[str, dict[str, float]]:"
    },
    {
      "line": 160,
      "text": "def parse_args() -> argparse.Namespace:"
    },
    {
      "line": 172,
      "text": "def os_cpu_count() -> int | None:"
    },
    {
      "line": 178,
      "text": "def collect_tasks(args: argparse.Namespace) -> list[tuple[str, str, str, str, int]]:"
    },
    {
      "line": 204,
      "text": "def format_case(case_name: str, metrics: dict[str, float]) -> str:"
    },
    {
      "line": 211,
      "text": "def write_report(results: list[tuple[str, dict[str, float]]], log_file: Path | None) -> str:"
    },
    {
      "line": 234,
      "text": "def main() -> None:"
    }
  ]
}

```text

"""Evaluate tooth segmentation predictions with the paper's six reported metrics."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import math
from pathlib import Path

import numpy as np
from scipy.optimize import linear_sum_assignment
from sklearn.neighbors import NearestNeighbors
from tqdm import tqdm


UPPER_TEETH_FDI = [18, 17, 16, 15, 14, 13, 12, 11, 21, 22, 23, 24, 25, 26, 27, 28]
LOWER_TEETH_FDI = [38, 37, 36, 35, 34, 33, 32, 31, 41, 42, 43, 44, 45, 46, 47, 48]
FDI_TO_LABEL = {
    fdi: index + 1
    for index, pair in enumerate(zip(UPPER_TEETH_FDI, LOWER_TEETH_FDI))
    for fdi in pair
}


def load_obj_vertices(path: Path) -> np.ndarray:
    vertices = []
    with path.open() as handle:
        for line in handle:
            if line.startswith("v "):
                vertices.append([float(value) for value in line.split()[1:4]])
    return np.asarray(vertices, dtype=np.float32)


def load_ground_truth(path: Path) -> np.ndarray:
    with path.open() as handle:
        raw_labels = np.asarray(json.load(handle)["labels"])
    # Accept already-binary labels for compatibility with binary inputs.
    if 1 in raw_labels:
        return raw_labels.astype(np.int64, copy=False)
    return np.asarray([FDI_TO_LABEL.get(int(label), 0) for label in raw_labels], dtype=np.int64)


def load_prediction(path: Path, pred_format: str) -> np.ndarray:
    if pred_format == "txt":
        return np.asarray(np.loadtxt(path, dtype=np.float64), dtype=np.int64).reshape(-1)
    with path.open() as handle:
        raw_labels = json.load(handle)["labels"]
    return np.asarray([FDI_TO_LABEL.get(int(label), 0) for label in raw_labels], dtype=np.int64)


def align_prediction(prediction: np.ndarray, ground_truth: np.ndarray) -> np.ndarray:
    prediction = np.asarray(prediction).reshape(-1)
    if prediction.shape == ground_truth.shape:
        return prediction
    aligned = np.zeros_like(ground_truth)
    aligned[: min(len(prediction), len(ground_truth))] = prediction[: len(ground_truth)]
    return aligned


def boundary_vertices(vertices: np.ndarray, labels: np.ndarray, neighbors: int) -> np.ndarray:
    if len(vertices) == 0:
        return np.empty((0,), dtype=np.int64)
    knn = NearestNeighbors(n_neighbors=min(neighbors + 1, len(vertices))).fit(vertices)
    _, indices = knn.kneighbors(vertices)
    neighbor_labels = labels[indices]
    return np.flatnonzero(np.any(neighbor_labels != neighbor_labels[:, [0]], axis=1))


def basic_metrics(prediction: np.ndarray, ground_truth: np.ndarray, vertices: np.ndarray, neighbors: int) -> dict[str, float]:
    oa = float(np.mean(prediction == ground_truth))
    gt_boundary = boundary_vertices(vertices, ground_truth, neighbors)
    pred_boundary = boundary_vertices(vertices, prediction, neighbors)
    union_boundary = np.union1d(gt_boundary, pred_boundary)
    biou = float(len(np.intersect1d(gt_boundary, pred_boundary)) / max(1, len(union_boundary)))

    tooth_labels = np.unique(ground_truth)
    tooth_labels = tooth_labels[tooth_labels > 0]
    if len(tooth_labels) == 0:
        return {"miou": 0.0, "dice": 0.0, "oa": oa, "biou": biou}

    ious, dices = [], []
    for label in tooth_labels:
        gt_mask = ground_truth == label
        pred_mask = prediction == label
        intersection = np.sum(gt_mask & pred_mask)
        union = np.sum(gt_mask | pred_mask)
        ious.append(intersection / union if union else 0.0)
        dices.append((2.0 * intersection) / (np.sum(gt_mask) + np.sum(pred_mask) + 1e-6))
    return {
        "miou": float(np.mean(ious)),
        "dice": float(np.mean(dices)),
        "oa": oa,
        "biou": biou,
    }


def tooth_side(label: int) -> str:
    if 1 <= label <= 8:
        return "right"
    if 9 <= label <= 16:
        return "left"
    return "unknown"


def tir_metric(prediction: np.ndarray, ground_truth: np.ndarray, vertices: np.ndarray, eps: float = 1e-12) -> float:
    """Compute TIR using the official centroid matching convention."""
    gt_labels = [int(label) for label in np.unique(ground_truth) if label > 0]
    pred_labels = [int(label) for label in np.unique(prediction) if label > 0]
    if not gt_labels or not pred_labels:
        return 0.0

    gt_info = {}
    for label in gt_labels:
        points = vertices[ground_truth == label]
        centroid = points.mean(axis=0)
        # Keep the original evaluator's axis-wise tooth-size normalization.
        tooth_size = np.sqrt(np.sum((centroid - points) ** 2, axis=0))
        gt_info[label] = (centroid, tooth_size)
    pred_centroids = {
        label: vertices[prediction == label].mean(axis=0)
        for label in pred_labels
    }

    gt_centroids = np.stack([gt_info[label][0] for label in gt_labels])
    pred_centroid_array = np.stack([pred_centroids[label] for label in pred_labels])
    cost = np.linalg.norm(gt_centroids[:, None, :] - pred_centroid_array[None, :, :], axis=2)
    rows, columns = linear_sum_assignment(cost)

    correct = 0
    for row, column in zip(rows, columns):
        gt_label = gt_labels[row]
        pred_label = pred_labels[column]
        centroid, tooth_size = gt_info[gt_label]
        denominator = np.where(np.abs(tooth_size) < eps, eps, tooth_size)
        distance = np.linalg.norm((centroid - pred_centroids[pred_label]) / denominator)
        if distance < 0.5 and gt_label == pred_label:
            correct += 1
    return correct / max(1, len(rows))


def first_match(root: Path, name: str, suffix: str) -> Path | None:
    matches = sorted(root.rglob(f"{name}{suffix}"))
    return matches[0] if matches else None


def evaluate_case(task: tuple[str, str, str, str, int]) -> tuple[str, dict[str, float]]:
    case_name, pred_path, obj_path, json_path, neighbors = task
    prediction = load_prediction(Path(pred_path), "json" if pred_path.endswith(".json") else "txt")
    ground_truth = load_ground_truth(Path(json_path))
    prediction = align_prediction(prediction, ground_truth)
    vertices = load_obj_vertices(Path(obj_path))
    if len(vertices) != len(ground_truth):
        raise ValueError(f"{case_name}: OBJ has {len(vertices)} vertices but JSON has {len(ground_truth)} labels.")
    metrics = basic_metrics(prediction, ground_truth, vertices, neighbors)
    metrics["tir"] = tir_metric(prediction, ground_truth, vertices)
    return case_name, metrics


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Evaluate tooth segmentation predictions.")
    parser.add_argument("--pred_dir", required=True, type=Path, help="Directory containing predicted TXT or JSON labels.")
    parser.add_argument("--gt_dir", required=True, type=Path, help="Raw dataset directory containing OBJ and JSON files.")
    parser.add_argument("--pred_format", choices=("txt", "json"), default="txt")
    parser.add_argument("--knn", type=int, default=10, help="KNN neighborhood size for B-IoU.")
    parser.add_argument("--workers", type=int, default=max(1, min(16, (os_cpu_count() or 1))))
    parser.add_argument("--log_file", type=Path, default=None, help="Optional path for detailed per-case results and summary.")
    parser.add_argument("--samples", nargs="*", default=None, help="Optional case names for a subset evaluation.")
    return parser.parse_args()


def os_cpu_count() -> int | None:
    import os

    return os.cpu_count()


def collect_tasks(args: argparse.Namespace) -> list[tuple[str, str, str, str, int]]:
    suffix = ".txt" if args.pred_format == "txt" else ".json"
    prediction_paths = sorted(
        path for path in args.pred_dir.rglob(f"*{suffix}")
        if not path.name.endswith("_pos_params.txt")
    )
    if args.samples is not None:
        wanted = set(args.samples)
        prediction_paths = [path for path in prediction_paths if path.stem in wanted]
        missing = wanted - {path.stem for path in prediction_paths}
        if missing:
            raise FileNotFoundError(f"Predictions not found for: {', '.join(sorted(missing))}")
    if not prediction_paths:
        raise FileNotFoundError(f"No {suffix} prediction files found under {args.pred_dir}")

    tasks = []
    for prediction_path in prediction_paths:
        case_name = prediction_path.stem
        obj_path = first_match(args.gt_dir, case_name, ".obj")
        json_path = first_match(args.gt_dir, case_name, ".json")
        if obj_path is None or json_path is None:
            raise FileNotFoundError(f"Missing OBJ or JSON ground truth for {case_name} under {args.gt_dir}")
        tasks.append((case_name, str(prediction_path), str(obj_path), str(json_path), args.knn))
    return tasks


def format_case(case_name: str, metrics: dict[str, float]) -> str:
    return (
        f"{case_name}: miou={metrics['miou']:.4f}, dice={metrics['dice']:.4f}, "
        f"oa={metrics['oa']:.4f}, biou={metrics['biou']:.4f}, tir={metrics['tir']:.4f}"
    )


def write_report(results: list[tuple[str, dict[str, float]]], log_file: Path | None) -> str:
    values = [metrics for _, metrics in results]
    tir_eq_1_count = sum(np.isclose(metrics["tir"], 1.0) for metrics in values)
    lines = [format_case(case_name, metrics) for case_name, metrics in results]
    lines.extend(
        [
            "",
            "==== SUMMARY ====",
            f"miou: {np.mean([metrics['miou'] for metrics in values]):.4f}",
            f"dice: {np.mean([metrics['dice'] for metrics in values]):.4f}",
            f"oa: {np.mean([metrics['oa'] for metrics in values]):.4f}",
            f"biou: {np.mean([metrics['biou'] for metrics in values]):.4f}",
            f"tir: {np.mean([metrics['tir'] for metrics in values]):.4f}",
            f"tir_eq_1: {tir_eq_1_count}/{len(values)} ({tir_eq_1_count / len(values) * 100:.2f}%)",
        ]
    )
    report = "\n".join(lines) + "\n"
    if log_file is not None:
        log_file.parent.mkdir(parents=True, exist_ok=True)
        log_file.write_text(report)
    return "\n".join(lines[-8:])


def main() -> None:
    args = parse_args()
    tasks = collect_tasks(args)
    results = []
    with concurrent.futures.ProcessPoolExecutor(max_workers=max(1, args.workers)) as executor:
        futures = [executor.submit(evaluate_case, task) for task in tasks]
        for future in tqdm(concurrent.futures.as_completed(futures), total=len(futures), desc="Evaluating"):
            results.append(future.result())
    results.sort(key=lambda item: item[0])
    print(write_report(results, args.log_file))
    if args.log_file is not None:
        print(f"Evaluation log saved to {args.log_file}")


if __name__ == "__main__":
    main()


```

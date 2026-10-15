# Facial rendering source scan: TencentYoutuResearch/3DFaceReconstruction-LAP

Repository: https://github.com/TencentYoutuResearch/3DFaceReconstruction-LAP

Pinned source revision: 1c92070d74d834e994f90404d0dfab8863a93d57

Discovery topic: face reconstruction 3D

Description: The code of paper 'Learning to Aggregate and Personalize 3D Face from In-the-Wild Photo Collection'

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["License.txt"]

Topic paths: {}

## Source file: lap/utils.py

{
  "path": "lap/utils.py",
  "blob_sha": "91131de489949dc6c16b4d672d09a42dfa24f623",
  "sha256": "fd2f2511c5b5ff8421da3bba99a3df0acf6f914e73a63dcf06a8f82127d2b6e0",
  "lines": 183,
  "bytes": 6026,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/TencentYoutuResearch/3DFaceReconstruction-LAP/blob/1c92070d74d834e994f90404d0dfab8863a93d57/lap/utils.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def setup_runtime(args):"
    },
    {
      "line": 45,
      "text": "def load_yaml(path):"
    },
    {
      "line": 51,
      "text": "def dump_yaml(path, cfgs):"
    },
    {
      "line": 58,
      "text": "def xmkdir(path):"
    },
    {
      "line": 63,
      "text": "def clean_checkpoint(checkpoint_dir, keep_num=2):"
    },
    {
      "line": 74,
      "text": "def archive_code(arc_path, filetypes=['.py', '.yml']):"
    },
    {
      "line": 89,
      "text": "def get_model_device(model):"
    },
    {
      "line": 93,
      "text": "def set_requires_grad(nets, requires_grad=False):"
    },
    {
      "line": 102,
      "text": "def draw_bbox(im, size):"
    },
    {
      "line": 114,
      "text": "def save_videos(out_fold, imgs, prefix='', suffix='', sep_folder=True, ext='.mp4', cycle=False):"
    },
    {
      "line": 134,
      "text": "def save_images(out_fold, imgs, prefix='', suffix='', sep_folder=True, ext='.png'):"
    },
    {
      "line": 151,
      "text": "def save_txt(out_fold, data, prefix='', suffix='', sep_folder=True, ext='.txt'):"
    },
    {
      "line": 163,
      "text": "def compute_sc_inv_err(d_pred, d_gt, mask=None):"
    },
    {
      "line": 176,
      "text": "def compute_angular_distance(n1, n2, mask=None):"
    },
    {
      "line": 181,
      "text": "def save_scores(out_path, scores, header=''):"
    }
  ]
}

```text

import os
import glob
import random
import zipfile
import yaml
import numpy as np
import cv2
import torch


def setup_runtime(args):
    """Load configs, initialize CUDA, CuDNN and the random seeds."""

    # Setup CUDA
    cuda_device_id = args.gpu
    if cuda_device_id is not None:
        os.environ["CUDA_DEVICE_ORDER"] = "PCI_BUS_ID"
        os.environ["CUDA_VISIBLE_DEVICES"] = str(cuda_device_id)
    if torch.cuda.is_available():
        torch.backends.cudnn.enabled = True
        torch.backends.cudnn.benchmark = True
        torch.backends.cudnn.deterministic = True

    # Setup random seeds for reproducibility
    random.seed(args.seed)
    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(args.seed)

    # Load config
    cfgs = {}
    if args.config is not None and os.path.isfile(args.config):
        cfgs = load_yaml(args.config)

    cfgs['config'] = args.config
    cfgs['seed'] = args.seed
    cfgs['num_workers'] = args.num_workers
    cfgs['device'] = 'cuda:0' if torch.cuda.is_available() and cuda_device_id is not None else 'cpu'

    print(f"Environment: GPU {cuda_device_id} seed {args.seed} number of workers {args.num_workers}")
    return cfgs


def load_yaml(path):
    print(f"Loading configs from {path}")
    with open(path, 'r') as f:
        return yaml.safe_load(f)


def dump_yaml(path, cfgs):
    print(f"Saving configs to {path}")
    xmkdir(os.path.dirname(path))
    with open(path, 'w') as f:
        return yaml.safe_dump(cfgs, f)


def xmkdir(path):
    """Create directory PATH recursively if it does not exist."""
    os.makedirs(path, exist_ok=True)


def clean_checkpoint(checkpoint_dir, keep_num=2):
    if keep_num > 0:
        names = list(sorted(
            glob.glob(os.path.join(checkpoint_dir, 'checkpoint*.pth'))
        ))
        if len(names) > keep_num:
            for name in names[:-keep_num]:
                print(f"Deleting obslete checkpoint file {name}")
                os.remove(name)


def archive_code(arc_path, filetypes=['.py', '.yml']):
    if '.py' and '.yml' not in filetypes:
        print("no matching file types")
        return None
    print(f"Archiving code to {arc_path}")
    xmkdir(os.path.dirname(arc_path))
    zipf = zipfile.ZipFile(arc_path, 'w', zipfile.ZIP_DEFLATED)
    cur_dir = os.getcwd()
    flist = []
    for ftype in filetypes:
        flist.extend(glob.glob(os.path.join(cur_dir, '**', '*'+ftype), recursive=True))
    [zipf.write(f, arcname = f.replace(cur_dir, 'archived_code', 1)) for f in flist]
    zipf.close()


def get_model_device(model):
    return next(model.parameters()).device


def set_requires_grad(nets, requires_grad=False):
    if not isinstance(nets, list):
        nets = [nets]
    for net in nets:
        if net is not None:
            for param in net.parameters():
                param.requires_grad = requires_grad


def draw_bbox(im, size):
    _, _, h, w = im.shape
    h2, w2 = (h-size)//2, (w-size)//2
    marker = np.tile(np.array([[1.], [0.], [0.]]), (1, size))
    marker = torch.FloatTensor(marker)
    im[:, :, h2, w2:w2+size] = marker
    im[:, :, h2+size, w2:w2+size] = marker
    im[:, :, h2:h2+size, w2] = marker
    im[:, :, h2:h2+size, w2+size] = marker
    return im


def save_videos(out_fold, imgs, prefix='', suffix='', sep_folder=True, ext='.mp4', cycle=False):
    if sep_folder:
        out_fold = os.path.join(out_fold, suffix)
    xmkdir(out_fold)
    prefix = prefix + '_' if prefix else ''
    suffix = '_' + suffix if suffix else ''
    offset = len(glob.glob(os.path.join(out_fold, prefix+'*'+suffix+ext))) + 1

    imgs = imgs.transpose(0, 1, 3, 4, 2)  # BxTxCxHxW -> BxTxHxWxC
    for i, fs in enumerate(imgs):
        if cycle:
            fs = np.concatenate([fs, fs[::-1]], 0)
        fourcc = cv2.VideoWriter_fourcc(*'mp4v')
        # fourcc = cv2.VideoWriter_fourcc(*'avc1')
        vid = cv2.VideoWriter(os.path.join(out_fold, prefix+'%05d'%(i+offset)+suffix+ext), 
        fourcc, 5, (fs.shape[2], fs.shape[1]))
        [vid.write(np.uint8(f[..., ::-1]*255.)) for f in fs]
        vid.release()


def save_images(out_fold, imgs, prefix='', suffix='', sep_folder=True, ext='.png'):
    if sep_folder:
        out_fold = os.path.join(out_fold, suffix)
    xmkdir(out_fold)
    prefix = prefix + '_' if prefix else ''
    suffix = '_' + suffix if suffix else ''
    offset = len(glob.glob(os.path.join(out_fold, prefix+'*'+suffix+ext))) + 1

    imgs = imgs.transpose(0, 2, 3, 1)
    for i, img in enumerate(imgs):
        if 'depth' in suffix:
            im_out = np.uint16(img[..., ::-1]*65535.)
        else:
            im_out = np.uint8(img[..., ::-1]*255.)
        cv2.imwrite(os.path.join(out_fold, prefix+'%05d'%(i+offset)+suffix+ext), im_out)


def save_txt(out_fold, data, prefix='', suffix='', sep_folder=True, ext='.txt'):
    if sep_folder:
        out_fold = os.path.join(out_fold, suffix)
    xmkdir(out_fold)
    prefix = prefix + '_' if prefix else ''
    suffix = '_' + suffix if suffix else ''
    offset = len(glob.glob(os.path.join(out_fold, prefix + '*' + suffix+ext))) + 1

    [np.savetxt(os.path.join(out_fold, prefix+'%05d'%(i+offset)+suffix+ext), d, fmt='%.6f', delimiter=', ') 
    for i, d in enumerate(data)]


def compute_sc_inv_err(d_pred, d_gt, mask=None):
    b = d_pred.size(0)
    diff = d_pred - d_gt
    if mask is not None:
        diff = diff * mask
        avg = diff.view(b, -1).sum(1) / (mask.view(b, -1).sum(1))
        score = (diff - avg.view(b, 1, 1))**2 * mask
    else:
        avg = diff.view(b, -1).mean(1)
        score = (diff - avg.view(b, 1, 1))**2
    return score  # masked error maps


def compute_angular_distance(n1, n2, mask=None):
    dist = (n1 * n2).sum(3).clamp(-1, 1).acos() / np.pi * 180
    return dist * mask if mask is not None else dist


def save_scores(out_path, scores, header=''):
    print('Saving scores to %s' % out_path)
    np.savetxt(out_path, scores, fmt='%.8f', delimiter=',\t', header=header)


```

## Source file: lap/__init__.py

{
  "path": "lap/__init__.py",
  "blob_sha": "afb46721956e8a2b77a7422c966e69d7b7df12b7",
  "sha256": "5201881d158e749efcd876cea130727fe3439ba828e6ba372d4e7216df602297",
  "lines": 1,
  "bytes": 33,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/TencentYoutuResearch/3DFaceReconstruction-LAP/blob/1c92070d74d834e994f90404d0dfab8863a93d57/lap/__init__.py",
  "function_signature_candidates": []
}

```text

from .utils import setup_runtime


```

## Source file: lap/renderer/utils.py

{
  "path": "lap/renderer/utils.py",
  "blob_sha": "9c43fc626e8366f23e46eb5ab371b9eb09be8299",
  "sha256": "5012c98cc3f19de2e085e96760f39ab7563c6078a1f396a92753bf65a05c8c4b",
  "lines": 107,
  "bytes": 3639,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4,
    "lip": 1
  },
  "url": "https://github.com/TencentYoutuResearch/3DFaceReconstruction-LAP/blob/1c92070d74d834e994f90404d0dfab8863a93d57/lap/renderer/utils.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def mm_normalize(x, min=0, max=1):"
    },
    {
      "line": 13,
      "text": "def rand_range(size, min, max):"
    },
    {
      "line": 17,
      "text": "def rand_posneg_range(size, min, max):"
    },
    {
      "line": 22,
      "text": "def get_grid(b, height, width, normalize=True):"
    },
    {
      "line": 33,
      "text": "def get_rotation_matrix(tx, ty, tz):"
    },
    {
      "line": 52,
      "text": "def get_transform_matrices(view):"
    },
    {
      "line": 74,
      "text": "def get_face_idx(b, h, w):"
    },
    {
      "line": 81,
      "text": "def vcolor_to_texture_cube(vcolors):"
    },
    {
      "line": 96,
      "text": "def get_textures_from_im(im, tx_size=1):"
    }
  ]
}

```text

import torch


def mm_normalize(x, min=0, max=1):
    x_min = x.min()
    x_max = x.max()
    x_range = x_max - x_min
    x_z = (x - x_min) / x_range
    x_out = x_z * (max - min) + min
    return x_out


def rand_range(size, min, max):
    return torch.rand(size)*(max-min)+min


def rand_posneg_range(size, min, max):
    i = (torch.rand(size) > 0.5).type(torch.float)*2.-1.
    return i*rand_range(size, min, max)


def get_grid(b, height, width, normalize=True):
    if normalize:
        h_range = torch.linspace(-1, 1, H)
        w_range = torch.linspace(-1, 1, W)
    else:
        h_range = torch.arange(0, height)
        w_range = torch.arange(0, width)
    grid = torch.stack(torch.meshgrid([h_range, w_range]), -1).repeat(b, 1, 1, 1).flip(3).float()  # flip h,w to x,y
    return grid


def get_rotation_matrix(tx, ty, tz):
    m_x = torch.zeros((len(tx), 3, 3)).to(tx.device)
    m_y = torch.zeros((len(tx), 3, 3)).to(tx.device)
    m_z = torch.zeros((len(tx), 3, 3)).to(tx.device)

    m_x[:, 1, 1], m_x[:, 1, 2] = tx.cos(), -tx.sin()
    m_x[:, 2, 1], m_x[:, 2, 2] = tx.sin(), tx.cos()
    m_x[:, 0, 0] = 1

    m_y[:, 0, 0], m_y[:, 0, 2] = ty.cos(), ty.sin()
    m_y[:, 2, 0], m_y[:, 2, 2] = -ty.sin(), ty.cos()
    m_y[:, 1, 1] = 1

    m_z[:, 0, 0], m_z[:, 0, 1] = tz.cos(), -tz.sin()
    m_z[:, 1, 0], m_z[:, 1, 1] = tz.sin(), tz.cos()
    m_z[:, 2, 2] = 1
    return torch.matmul(m_z, torch.matmul(m_y, m_x))


def get_transform_matrices(view):
    b = view.size(0)
    if view.size(1) == 6:
        rx = view[:, 0]
        ry = view[:, 1]
        rz = view[:, 2]
        trans_xyz = view[:, 3:].reshape(b, 1, 3)
    elif view.size(1) == 5:
        rx = view[:, 0]
        ry = view[:, 1]
        rz = view[:, 2]
        delta_xy = view[:, 3:].reshape(b, 1, 2)
        trans_xyz = torch.cat([delta_xy, torch.zeros(b, 1, 1).to(view.device)], 2)
    elif view.size(1) == 3:
        rx = view[:, 0]
        ry = view[:, 1]
        rz = view[:, 2]
        trans_xyz = torch.zeros(b, 1, 3).to(view.device)
    rot_mat = get_rotation_matrix(rx, ry, rz)
    return rot_mat, trans_xyz


def get_face_idx(b, h, w):
    idx_map = torch.arange(h * w).reshape(h, w)
    faces1 = torch.stack([idx_map[:h-1, :w-1], idx_map[1:, :w-1], idx_map[:h-1, 1:]], -1).reshape(-1, 3)
    faces2 = torch.stack([idx_map[:h-1, 1:], idx_map[1:, :w-1], idx_map[1:, 1:]], -1).reshape(-1, 3)
    return torch.cat([faces1, faces2], 0).repeat(b, 1, 1).int()


def vcolor_to_texture_cube(vcolors):
    # input bxcxnx3
    b, c, n, _ = vcolors.shape
    coeffs = torch.FloatTensor(
        [[ 0.5,  0.5,  0.5],
         [ 0. ,  0. ,  1. ],
         [ 0. ,  1. ,  0. ],
         [-0.5,  0.5,  0.5],
         [ 1. ,  0. ,  0. ],
         [ 0.5, -0.5,  0.5],
         [ 0.5,  0.5, -0.5],
         [ 0. ,  0. ,  0. ]]).to(vcolors.device)
    return coeffs.matmul(vcolors.permute(0, 2, 3, 1)).reshape(b, n, 2, 2, 2, c)


def get_textures_from_im(im, tx_size=1):
    b, c, h, w = im.shape
    if tx_size == 1:
        textures = torch.cat([im[:, :, :h-1, :w-1].reshape(b, c, -1), im[:, :, 1:, 1:].reshape(b, c, -1)], 2)
        textures = textures.transpose(2, 1).reshape(b, -1, 1, 1, 1, c)
    elif tx_size == 2:
        textures1 = torch.stack([im[:, :, :h-1, :w-1], im[:, :, :h-1, 1:], im[:, :, 1:, :w-1]], -1).reshape(b, c, -1, 3)
        textures2 = torch.stack([im[:, :, 1:, :w-1], im[:, :, :h-1, 1:], im[:, :, 1:, 1:]], -1).reshape(b, c, -1, 3)
        textures = vcolor_to_texture_cube(torch.cat([textures1, textures2], 2))  # bxnx2x2x2xc
    else:
        raise NotImplementedError("Currently support texture size of 1 or 2 only.")
    return textures


```

## Source file: lap/renderer/__init__.py

{
  "path": "lap/renderer/__init__.py",
  "blob_sha": "4581c8ec991a741a0c347eed1ba326d49f9964c9",
  "sha256": "044c07909b4dfda8edb8155ce5628e7ac89c424dc075984f2b4223a97adabd1f",
  "lines": 1,
  "bytes": 34,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/TencentYoutuResearch/3DFaceReconstruction-LAP/blob/1c92070d74d834e994f90404d0dfab8863a93d57/lap/renderer/__init__.py",
  "function_signature_candidates": []
}

```text

from .renderer_mr import Renderer


```

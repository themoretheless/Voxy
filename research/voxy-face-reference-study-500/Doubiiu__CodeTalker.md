# Facial rendering source scan: Doubiiu/CodeTalker

Repository: https://github.com/Doubiiu/CodeTalker

Pinned source revision: 1b7cb95a637bda670bd39369bae1bf10299b5339

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["BIWI/data_preprocess/Utils/+MESH/+SEG/private/MeshMapperSurfaceIdx.m", "BIWI/data_preprocess/Utils/+MESH/+SEG/private/lb_basis_surface.m", "BIWI/data_preprocess/Utils/+MESH/compute_vtx_and_face_normals.m", "BIWI/data_preprocess/Utils/+MESH/private/compute_vtx_and_face_normals.m", "BIWI/data_preprocess/Utils/+MESH/surfaceArea.m", "BIWI/data_preprocess/Utils/func_orientationOp/private/face_grads.m", "BIWI/data_preprocess/Utils/func_orientationOp/private/face_grads_mat.m", "demo/output/man_FaceTalk_170809_00138_TA_condition_FaceTalk_170725_00137_TA_audio.mp4"]}

## Source file: utils/util.py

{
  "path": "utils/util.py",
  "blob_sha": "a669fcbeb9829fbf4cb429c421dbd81e826c344a",
  "sha256": "6eb4c03d67ba940ff4945f32dcbb4677afe5a1ce95263677bdc9929f0109a684",
  "lines": 84,
  "bytes": 1911,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 7
  },
  "url": "https://github.com/Doubiiu/CodeTalker/blob/1b7cb95a637bda670bd39369bae1bf10299b5339/utils/util.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def ensure_dir(path):"
    },
    {
      "line": 13,
      "text": "def imagesc(nd_array):"
    },
    {
      "line": 20,
      "text": "def imread(img):"
    },
    {
      "line": 31,
      "text": "def imwrite(img, name):"
    },
    {
      "line": 36,
      "text": "def stereoRead(img: str):"
    },
    {
      "line": 45,
      "text": "def stereoWright(left: np.ndarray, right: np.ndarray, name: str):"
    },
    {
      "line": 50,
      "text": "def exeCmd(cmd=None):"
    },
    {
      "line": 55,
      "text": "def img2tensor(img, cuda=True):"
    },
    {
      "line": 63,
      "text": "def tensor2img(img_t):"
    },
    {
      "line": 74,
      "text": "def cal_padding(L, window_size):"
    },
    {
      "line": 82,
      "text": "def requires_grad(model, flag=True):"
    }
  ]
}

```text

import os, subprocess
import numpy as np
from matplotlib import pyplot as plt
import torch
import cv2


def ensure_dir(path):
    if not os.path.exists(path):
        os.makedirs(path)


def imagesc(nd_array):
    plt.figure(figsize=(10, 10))
    plt.imshow(nd_array)
    plt.colorbar()
    plt.show()


def imread(img):
    # return RGB image
    try:
        im = cv2.imread(img)
    except Exception as e:
        print(e)
        return None
    im = im[..., ::-1] / 255.
    return im


def imwrite(img, name):
    # write RGB image, img is in RGB format
    cv2.imwrite(name, img[..., ::-1] * 255.)


def stereoRead(img: str):
    im = imread(img)
    H, W, _ = im.shape
    assert W % 2 == 0, '%s is not a side-by-side stereo image' % img
    left = im[:, :W // 2, ...]
    right = im[:, W // 2:, ...]
    return left, right


def stereoWright(left: np.ndarray, right: np.ndarray, name: str):
    im = np.concatenate((left, right), axis=1)
    imwrite(im, name)


def exeCmd(cmd=None):
    print(cmd)
    return subprocess.check_call(cmd, shell=True)


def img2tensor(img, cuda=True):
    img_t = np.expand_dims(img.transpose(2, 0, 1), axis=0)
    img_t = torch.from_numpy(img_t.astype(np.float32))
    if cuda:
        img_t = img_t.cuda(non_blocking=True)
    return img_t


def tensor2img(img_t):
    if len(img_t.shape) == 4:
        img = img_t[0].detach().cpu().numpy()
    elif len(img_t.shape) == 3:
        img = img_t.detach().cpu().numpy()
    else:
        raise NotImplementedError
    img = img.transpose(1, 2, 0)
    return img


def cal_padding(L, window_size):
    padding_left, padding_right = 0, 0
    if L%window_size != 0:
        total = window_size-L%window_size
        padding_left = total//2
        padding_right = total - padding_left
    return padding_left, padding_right

def requires_grad(model, flag=True):
    for p in model.parameters():
        p.requires_grad = flag


```

## Source file: base/config.py

{
  "path": "base/config.py",
  "blob_sha": "62f93b9cb9e9b71301e26e27900b26467a119636",
  "sha256": "5813a009521cc94fa4d92949a052208330b80d79d530d77199e4eb8c03f9812f",
  "lines": 165,
  "bytes": 5444,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "rig": 8
  },
  "url": "https://github.com/Doubiiu/CodeTalker/blob/1b7cb95a637bda670bd39369bae1bf10299b5339/base/config.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, init_dict=None, key_list=None, new_allowed=False):"
    },
    {
      "line": 26,
      "text": "def __getattr__(self, name):"
    },
    {
      "line": 32,
      "text": "def __setattr__(self, name, value):"
    },
    {
      "line": 35,
      "text": "def __str__(self):"
    },
    {
      "line": 36,
      "text": "def _indent(s_, num_spaces):"
    },
    {
      "line": 56,
      "text": "def __repr__(self):"
    },
    {
      "line": 60,
      "text": "def load_cfg_from_cfg_file(file):"
    },
    {
      "line": 76,
      "text": "def merge_cfg_from_list(cfg, cfg_list):"
    },
    {
      "line": 91,
      "text": "def _decode_cfg_value(v):"
    },
    {
      "line": 120,
      "text": "def _check_and_coerce_cfg_value_type(replacement, original, key, full_key):"
    },
    {
      "line": 134,
      "text": "def conditional_cast(from_type, to_type):"
    },
    {
      "line": 162,
      "text": "def _assert_with_logging(cond, msg):"
    }
  ]
}

```text

# -----------------------------------------------------------------------------
# Functions for parsing args
# -----------------------------------------------------------------------------
import yaml
import os
from ast import literal_eval
import copy


class CfgNode(dict):
    """
    CfgNode represents an internal node in the configuration tree. It's a simple
    dict-like container that allows for attribute-based access to keys.
    """

    def __init__(self, init_dict=None, key_list=None, new_allowed=False):
        # Recursively convert nested dictionaries in init_dict into CfgNodes
        init_dict = {} if init_dict is None else init_dict
        key_list = [] if key_list is None else key_list
        for k, v in init_dict.items():
            if type(v) is dict:
                # Convert dict to CfgNode
                init_dict[k] = CfgNode(v, key_list=key_list + [k])
        super(CfgNode, self).__init__(init_dict)

    def __getattr__(self, name):
        if name in self:
            return self[name]
        else:
            raise AttributeError(name)

    def __setattr__(self, name, value):
        self[name] = value

    def __str__(self):
        def _indent(s_, num_spaces):
            s = s_.split("\n")
            if len(s) == 1:
                return s_
            first = s.pop(0)
            s = [(num_spaces * " ") + line for line in s]
            s = "\n".join(s)
            s = first + "\n" + s
            return s

        r = ""
        s = []
        for k, v in sorted(self.items()):
            seperator = "\n" if isinstance(v, CfgNode) else " "
            attr_str = "{}:{}{}".format(str(k), seperator, str(v))
            attr_str = _indent(attr_str, 2)
            s.append(attr_str)
        r += "\n".join(s)
        return r

    def __repr__(self):
        return "{}({})".format(self.__class__.__name__, super(CfgNode, self).__repr__())


def load_cfg_from_cfg_file(file):
    cfg = {}
    assert os.path.isfile(file) and file.endswith('.yaml'), \
        '{} is not a yaml file'.format(file)

    with open(file, 'r') as f:
        cfg_from_file = yaml.safe_load(f)

    for key in cfg_from_file:
        for k, v in cfg_from_file[key].items():
            cfg[k] = v

    cfg = CfgNode(cfg)
    return cfg


def merge_cfg_from_list(cfg, cfg_list):
    new_cfg = copy.deepcopy(cfg)
    assert len(cfg_list) % 2 == 0
    for full_key, v in zip(cfg_list[0::2], cfg_list[1::2]):
        subkey = full_key.split('.')[-1]
        assert subkey in cfg, 'Non-existent key: {}'.format(full_key)
        value = _decode_cfg_value(v)
        value = _check_and_coerce_cfg_value_type(
            value, cfg[subkey], subkey, full_key
        )
        setattr(new_cfg, subkey, value)

    return new_cfg


def _decode_cfg_value(v):
    """Decodes a raw config value (e.g., from a yaml config files or command
    line argument) into a Python object.
    """
    # All remaining processing is only applied to strings
    if not isinstance(v, str):
        return v
    # Try to interpret `v` as a:
    #   string, number, tuple, list, dict, boolean, or None
    try:
        v = literal_eval(v)
    # The following two excepts allow v to pass through when it represents a
    # string.
    #
    # Longer explanation:
    # The type of v is always a string (before calling literal_eval), but
    # sometimes it *represents* a string and other times a data structure, like
    # a list. In the case that v represents a string, what we got back from the
    # yaml parser is 'foo' *without quotes* (so, not '"foo"'). literal_eval is
    # ok with '"foo"', but will raise a ValueError if given 'foo'. In other
    # cases, like paths (v = 'foo/bar' and not v = '"foo/bar"'), literal_eval
    # will raise a SyntaxError.
    except ValueError:
        pass
    except SyntaxError:
        pass
    return v


def _check_and_coerce_cfg_value_type(replacement, original, key, full_key):
    """Checks that `replacement`, which is intended to replace `original` is of
    the right type. The type is correct if it matches exactly or is one of a few
    cases in which the type can be easily coerced.
    """
    original_type = type(original)
    replacement_type = type(replacement)

    # The types must match (with some exceptions)
    if replacement_type == original_type or original is None:
        return replacement

    # Cast replacement from from_type to to_type if the replacement and original
    # types match from_type and to_type
    def conditional_cast(from_type, to_type):
        if replacement_type == from_type and original_type == to_type:
            return True, to_type(replacement)
        else:
            return False, None

    # Conditionally casts
    # list <-> tuple
    casts = [(tuple, list), (list, tuple)]
    # For py2: allow converting from str (bytes) to a unicode string
    try:
        casts.append((str, unicode))  # noqa: F821
    except Exception:
        pass

    for (from_type, to_type) in casts:
        converted, converted_value = conditional_cast(from_type, to_type)
        if converted:
            return converted_value

    raise ValueError(
        "Type mismatch ({} vs. {}) with values ({} vs. {}) for config "
        "key: {}".format(
            original_type, replacement_type, original, replacement, full_key
        )
    )


def _assert_with_logging(cond, msg):
    if not cond:
        logger.debug(msg)
    assert cond, msg


```

## Source file: main/render.py

{
  "path": "main/render.py",
  "blob_sha": "ee39ee7ffaa8976f84ae6efc981a9bc9e6431758",
  "sha256": "aafc9c2cbc3565b46a768627dce6262895e1e28c06a7e5624fc52aeed38cc780",
  "lines": 204,
  "bytes": 8653,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "gum": 8,
    "flame": 1,
    "rig": 10,
    "face": 2
  },
  "url": "https://github.com/Doubiiu/CodeTalker/blob/1b7cb95a637bda670bd39369bae1bf10299b5339/main/render.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def add_text_to_image(img, text):"
    },
    {
      "line": 43,
      "text": "def render_mesh_helper(args,mesh, t_center, rot=np.zeros(3), tex_img=None, z_offset=0):"
    },
    {
      "line": 124,
      "text": "def render_sequence_meshes(args, sequence_vertices, template, out_path, predicted_vertices_path, vt, ft ,tex_img):"
    },
    {
      "line": 159,
      "text": "def main():"
    }
  ]
}

```text

'''
Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this
computer program.
You can only use this computer program if you have closed a license agreement with MPG or you get the right to use
the computer program from someone who is authorized to grant you that right.
Any use of the computer program without a valid license is prohibited and liable to prosecution.
Copyright 2019 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its
Max Planck Institute for Intelligent Systems and the Max Planck Institute for Biological Cybernetics.
All rights reserved.
More information about VOCA is available at http://voca.is.tue.mpg.de.
For comments or questions, please email us at voca@tue.mpg.de
'''

import os, shutil
import cv2
import tempfile
import numpy as np
from subprocess import call
import argparse
os.environ['PYOPENGL_PLATFORM'] = 'osmesa' #egl
import pyrender
import trimesh
from psbody.mesh import Mesh


def add_text_to_image(img, text):
    # setup text
    font = cv2.FONT_HERSHEY_SIMPLEX
    text = text

    # get boundary of this text
    textsize = cv2.getTextSize(text, font, 1, 2)[0]

    # get coords based on boundary
    textX = (img.shape[1] - textsize[0]) // 2
    textY = img.shape[0]- (img.shape[0] + textsize[1]) // 16

    # add text centered on image
    cv2.putText(img, text, (textX, textY), font, 1, (255, 255, 255), 2)
    return img

# The implementation of rendering is borrowed from VOCA: https://github.com/TimoBolkart/voca/blob/master/utils/rendering.py
def render_mesh_helper(args,mesh, t_center, rot=np.zeros(3), tex_img=None, z_offset=0):
    if args.dataset == "BIWI":
        camera_params = {'c': np.array([400, 400]),
                         'k': np.array([-0.19816071, 0.92822711, 0, 0, 0]),
                         'f': np.array([4754.97941935 / 8, 4754.97941935 / 8])}
    elif args.dataset == "vocaset":
        camera_params = {'c': np.array([400, 400]),
                         'k': np.array([-0.19816071, 0.92822711, 0, 0, 0]),
                         'f': np.array([4754.97941935 / 2, 4754.97941935 / 2])}

    frustum = {'near': 0.01, 'far': 3.0, 'height': 800, 'width': 800}

    mesh_copy = Mesh(mesh.v, mesh.f)
    mesh_copy.v[:] = cv2.Rodrigues(rot)[0].dot((mesh_copy.v-t_center).T).T+t_center
    
    intensity = 2.0

    primitive_material = pyrender.material.MetallicRoughnessMaterial(
                alphaMode='BLEND',
                baseColorFactor=[0.3, 0.3, 0.3, 1.0],
                metallicFactor=0.8, 
                roughnessFactor=0.8 
            )


    tri_mesh = trimesh.Trimesh(vertices=mesh_copy.v, faces=mesh_copy.f)

    render_mesh = pyrender.Mesh.from_trimesh(tri_mesh, material=primitive_material,smooth=True)

    if args.background_black:
        scene = pyrender.Scene(ambient_light=[.2, .2, .2], bg_color=[0, 0, 0])
    else:
        scene = pyrender.Scene(ambient_light=[.2, .2, .2], bg_color=[255, 255, 255])
    
    camera = pyrender.IntrinsicsCamera(fx=camera_params['f'][0],
                                      fy=camera_params['f'][1],
                                      cx=camera_params['c'][0],
                                      cy=camera_params['c'][1],
                                      znear=frustum['near'],
                                      zfar=frustum['far'])

    scene.add(render_mesh, pose=np.eye(4))

    camera_pose = np.eye(4)
    camera_pose[:3,3] = np.array([0, 0, 1.0-z_offset])
    scene.add(camera, pose=[[1, 0, 0, 0],
                            [0, 1, 0, 0],
                            [0, 0, 1, 1],
                            [0, 0, 0, 1]])

    angle = np.pi / 6.0
    pos = camera_pose[:3,3]
    light_color = np.array([1., 1., 1.])
    light = pyrender.DirectionalLight(color=light_color, intensity=intensity)

    light_pose = np.eye(4)
    light_pose[:3,3] = pos
    scene.add(light, pose=light_pose.copy())
    
    light_pose[:3,3] = cv2.Rodrigues(np.array([angle, 0, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] =  cv2.Rodrigues(np.array([-angle, 0, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] = cv2.Rodrigues(np.array([0, -angle, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    light_pose[:3,3] = cv2.Rodrigues(np.array([0, angle, 0]))[0].dot(pos)
    scene.add(light, pose=light_pose.copy())

    flags = pyrender.RenderFlags.SKIP_CULL_FACES
    try:
        r = pyrender.OffscreenRenderer(viewport_width=frustum['width'], viewport_height=frustum['height'])
        color, _ = r.render(scene, flags=flags)
    except:
        print('pyrender: Failed rendering frame')
        color = np.zeros((frustum['height'], frustum['width'], 3), dtype='uint8')

    return color[..., ::-1]

def render_sequence_meshes(args, sequence_vertices, template, out_path, predicted_vertices_path, vt, ft ,tex_img):
    num_frames = sequence_vertices.shape[0]
    file_name_pred = predicted_vertices_path.split('/')[-1].split('.')[0]
    file_name_wav = os.path.join(args.dataset_dir, 'wav', file_name_pred.split('_condition_')[0]+'.wav')

    tmp_video_file_pred = tempfile.NamedTemporaryFile('w', suffix='.mp4', dir=out_path)
    writer_pred = cv2.VideoWriter(tmp_video_file_pred.name, cv2.VideoWriter_fourcc(*'mp4v'), args.fps, (800, 800), True) # w, h

    center = np.mean(sequence_vertices[0], axis=0)

    video_fname_pred = os.path.join(out_path, file_name_pred+'.mp4')
    for i_frame in range(num_frames):
        render_mesh = Mesh(sequence_vertices[i_frame], template.f)
        if vt is not None and ft is not None:
            render_mesh.vt, render_mesh.ft = vt, ft

        pred_img = render_mesh_helper(args,render_mesh, center, tex_img=tex_img)
        pred_img = pred_img.astype(np.uint8)
  
        writer_pred.write(pred_img)

    writer_pred.release()

    ### render without audio
    cmd = ('ffmpeg' + ' -i {0} -pix_fmt yuv420p -qscale 0 {1}'.format(
       tmp_video_file_pred.name, video_fname_pred)).split()
    call(cmd)

    ### render with audio
    cmd = ('ffmpeg' + ' -i {0} -i {1} -vcodec h264 -ac 2 -channel_layout stereo -qscale 0 {2}'.format(
       file_name_wav, video_fname_pred, video_fname_pred.replace('.mp4', '_audio.mp4'))).split()
    call(cmd)
    if os.path.exists(video_fname_pred):
        os.remove(video_fname_pred)

def main():
    parser = argparse.ArgumentParser(description='CodeTalker: Speech-Driven 3D Facial Animation with Discrete Motion Prior')
    parser.add_argument("--dataset", type=str, default="vocaset", help='vocaset or BIWI')
    parser.add_argument("--dataset_dir", type=str, default="./")
    parser.add_argument('--background_black', type=bool, default=True, help='whether to use black background')
    parser.add_argument('--fps', type=int,default=30, help='frame rate - 30 for vocaset; 25 for BIWI')
    parser.add_argument("--vertice_dim", type=int, default=5023*3, help='number of vertices - 5023*3 for vocaset; 23370*3 for BIWI')
    parser.add_argument("--pred_path", type=str, default='')
    parser.add_argument("--output_path", type=str, default='')
    args = parser.parse_args()
    if args.dataset == 'BIWI':
        args.fps=25
        args.vertice_dim=70110

    args.dataset_dir = args.dataset_dir + args.dataset

    assert os.path.exists(args.pred_path)
    if not os.path.exists(args.output_path):
        os.makedirs(args.output_path)
    else:
        pass
        # shutil.rmtree(args.output_path)
        # os.makedirs(args.output_path)


    for file in os.listdir(args.pred_path):
        if file.endswith("npy"): 
            predicted_vertices_path = os.path.join(args.pred_path, file)
            if args.dataset == "BIWI":
                template_file = os.path.join(args.dataset_dir, "BIWI.ply")
            elif args.dataset == "vocaset":
                template_file = os.path.join(args.dataset_dir, "FLAME_sample.ply")
            print("rendering: ", file)
        
            template = Mesh(filename=template_file)
            vt, ft = None, None
            tex_img = None

            predicted_vertices = np.load(predicted_vertices_path)
            predicted_vertices = np.reshape(predicted_vertices,(-1,args.vertice_dim//3,3))
            
            render_sequence_meshes(args, predicted_vertices, template, args.output_path, predicted_vertices_path, vt, ft ,tex_img)


if __name__=="__main__":
    main()

```

## Source file: metrics/loss.py

{
  "path": "metrics/loss.py",
  "blob_sha": "f7146f331355707f35be5e2e77f0595dd028fc2a",
  "sha256": "b97a909724ff8afdfeb38672ae2a98e1434b2e1a5637549aaa7cb0fb1a9f9208",
  "lines": 16,
  "bytes": 628,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Doubiiu/CodeTalker/blob/1b7cb95a637bda670bd39369bae1bf10299b5339/metrics/loss.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def calc_vq_loss(pred, target, quant_loss, quant_loss_weight=1.0, alpha=1.0):"
    },
    {
      "line": 13,
      "text": "def calc_logit_loss(pred, target):"
    }
  ]
}

```text

#!/usr/bin/env python
import torch.nn.functional as F
import torch.nn as nn


def calc_vq_loss(pred, target, quant_loss, quant_loss_weight=1.0, alpha=1.0):
    """ function that computes the various components of the VQ loss """
    rec_loss = nn.L1Loss()(pred, target)
    ## loss is VQ reconstruction + weighted pre-computed quantization loss
    quant_loss = quant_loss.mean()
    return quant_loss * quant_loss_weight + rec_loss, [rec_loss, quant_loss]

def calc_logit_loss(pred, target):
    """ Cross entropy loss wrapper """
    loss = F.cross_entropy(pred.reshape(-1, pred.size(-1)), target.reshape(-1))
    return loss


```

# Facial rendering source scan: barisgecer/facegan

Repository: https://github.com/barisgecer/facegan

Pinned source revision: 7df369afab50a26994d7e09ef428f54f7358b617

Discovery topic: face model 3D

Description: TF implementation of our ECCV 2018 paper:  Semi-supervised Adversarial Learning to Generate Photorealistic Face Images of New Identities from 3D Morphable Model

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["figures/DrosteEffect-BrewerMap-213e65d/license.txt"]

Topic paths: {"face": ["facenet_model/20170511-185253.pb", "facenet_model/model-20170511-185253.ckpt-80000.data-00000-of-00001", "facenet_model/model-20170511-185253.ckpt-80000.index", "facenet_model/model-20170511-185253.meta"]}

## Source file: ops.py

{
  "path": "ops.py",
  "blob_sha": "023b9964bec3285f8d555a617e609c6641834246",
  "sha256": "2300bee5f5ee6503423bf0695368c4319e49e849dc3c6808f80c41f4e5751513",
  "lines": 265,
  "bytes": 8607,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/barisgecer/facegan/blob/7df369afab50a26994d7e09ef428f54f7358b617/ops.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def c7s1_k(input, k, reuse=False, norm='instance', activation='relu', is_training=True, name='c7s1_k'):"
    },
    {
      "line": 35,
      "text": "def c3s1_k(input, k, reuse=False, norm='instance', activation='relu', is_training=True, name='c7s1_k'):"
    },
    {
      "line": 65,
      "text": "def dk(input, k, reuse=False, norm='instance', is_training=True, name=None):"
    },
    {
      "line": 88,
      "text": "def Rk(input, k,  reuse=False, norm='instance', is_training=True, name=None):"
    },
    {
      "line": 120,
      "text": "def n_res_blocks(input, reuse, norm='instance', is_training=True, n=6, drop_keep=1.0):"
    },
    {
      "line": 128,
      "text": "def uk(input, k, reuse=False, norm='instance', is_training=True, name=None, output_size=None):"
    },
    {
      "line": 159,
      "text": "def Ck(input, k, slope=0.2, stride=2, reuse=False, norm='instance', is_training=True, name=None):"
    },
    {
      "line": 184,
      "text": "def last_conv(input, reuse=False, use_sigmoid=False, name=None):"
    },
    {
      "line": 206,
      "text": "def _weights(name, shape, mean=0.0, stddev=0.02):"
    },
    {
      "line": 222,
      "text": "def _biases(name, shape, constant=0.0):"
    },
    {
      "line": 228,
      "text": "def _leaky_relu(input, slope):"
    },
    {
      "line": 231,
      "text": "def _norm(input, is_training, norm='instance'):"
    },
    {
      "line": 241,
      "text": "def _batch_norm(input, is_training):"
    },
    {
      "line": 251,
      "text": "def _instance_norm(input):"
    },
    {
      "line": 264,
      "text": "def safe_log(x, eps=1e-12):"
    }
  ]
}

```text

import tensorflow as tf

## Layers: follow the naming convention used in the original paper
### Generator layers
def c7s1_k(input, k, reuse=False, norm='instance', activation='relu', is_training=True, name='c7s1_k'):
  """ A 7x7 Convolution-BatchNorm-ReLU layer with k filters and stride 1
  Args:
    input: 4D tensor
    k: integer, number of filters (output depth)
    norm: 'instance' or 'batch' or None
    activation: 'relu' or 'tanh'
    name: string, e.g. 'c7sk-32'
    is_training: boolean or BoolTensor
    name: string
    reuse: boolean
  Returns:
    4D tensor
  """
  with tf.variable_scope(name, reuse=reuse):
    weights = _weights("weights",
      shape=[7, 7, input.get_shape()[3], k])

    padded = tf.pad(input, [[0,0],[3,3],[3,3],[0,0]], 'REFLECT')
    conv = tf.nn.conv2d(padded, weights,
        strides=[1, 1, 1, 1], padding='VALID')

    normalized = _norm(conv, is_training, norm)
    output = normalized
    if activation == 'relu':
      output = tf.nn.relu(normalized)
    if activation == 'tanh':
      output = tf.nn.tanh(normalized)
    return output

def c3s1_k(input, k, reuse=False, norm='instance', activation='relu', is_training=True, name='c7s1_k'):
  """ A 7x7 Convolution-BatchNorm-ReLU layer with k filters and stride 1
  Args:
    input: 4D tensor
    k: integer, number of filters (output depth)
    norm: 'instance' or 'batch' or None
    activation: 'relu' or 'tanh'
    name: string, e.g. 'c7sk-32'
    is_training: boolean or BoolTensor
    name: string
    reuse: boolean
  Returns:
    4D tensor
  """
  with tf.variable_scope(name, reuse=reuse):
    weights = _weights("weights",
      shape=[3, 3, input.get_shape()[3], k])

    padded = tf.pad(input, [[0,0],[1,1],[1,1],[0,0]], 'REFLECT')
    conv = tf.nn.conv2d(padded, weights,
        strides=[1, 1, 1, 1], padding='VALID')

    normalized = _norm(conv, is_training, norm)
    output = normalized
    if activation == 'relu':
      output = tf.nn.relu(normalized)
    if activation == 'tanh':
      output = tf.nn.tanh(normalized)
    return output

def dk(input, k, reuse=False, norm='instance', is_training=True, name=None):
  """ A 3x3 Convolution-BatchNorm-ReLU layer with k filters and stride 2
  Args:
    input: 4D tensor
    k: integer, number of filters (output depth)
    norm: 'instance' or 'batch' or None
    is_training: boolean or BoolTensor
    name: string
    reuse: boolean
    name: string, e.g. 'd64'
  Returns:
    4D tensor
  """
  with tf.variable_scope(name, reuse=reuse):
    weights = _weights("weights",
      shape=[3, 3, input.get_shape()[3], k])

    conv = tf.nn.conv2d(input, weights,
        strides=[1, 2, 2, 1], padding='SAME')
    normalized = _norm(conv, is_training, norm)
    output = tf.nn.relu(normalized)
    return output

def Rk(input, k,  reuse=False, norm='instance', is_training=True, name=None):
  """ A residual block that contains two 3x3 convolutional layers
      with the same number of filters on both layer
  Args:
    input: 4D Tensor
    k: integer, number of filters (output depth)
    reuse: boolean
    name: string
  Returns:
    4D tensor (same shape as input)
  """
  with tf.variable_scope(name, reuse=reuse):
    with tf.variable_scope('layer1', reuse=reuse):
      weights1 = _weights("weights1",
        shape=[3, 3, input.get_shape()[3], k])
      padded1 = tf.pad(input, [[0,0],[1,1],[1,1],[0,0]], 'REFLECT')
      conv1 = tf.nn.conv2d(padded1, weights1,
          strides=[1, 1, 1, 1], padding='VALID')
      normalized1 = _norm(conv1, is_training, norm)
      relu1 = tf.nn.relu(normalized1)

    with tf.variable_scope('layer2', reuse=reuse):
      weights2 = _weights("weights2",
        shape=[3, 3, relu1.get_shape()[3], k])

      padded2 = tf.pad(relu1, [[0,0],[1,1],[1,1],[0,0]], 'REFLECT')
      conv2 = tf.nn.conv2d(padded2, weights2,
          strides=[1, 1, 1, 1], padding='VALID')
      normalized2 = _norm(conv2, is_training, norm)
    output = input+normalized2
    return output

def n_res_blocks(input, reuse, norm='instance', is_training=True, n=6, drop_keep=1.0):
  depth = input.get_shape()[3]
  for i in range(1,n+1):
    output = Rk(input, depth, reuse, norm, is_training, 'R{}_{}'.format(depth, i))
    output = tf.nn.dropout(output, drop_keep)
    input = output
  return output

def uk(input, k, reuse=False, norm='instance', is_training=True, name=None, output_size=None):
  """ A 3x3 fractional-strided-Convolution-BatchNorm-ReLU layer
      with k filters, stride 1/2
  Args:
    input: 4D tensor
    k: integer, number of filters (output depth)
    norm: 'instance' or 'batch' or None
    is_training: boolean or BoolTensor
    reuse: boolean
    name: string, e.g. 'c7sk-32'
    output_size: integer, desired output size of layer
  Returns:
    4D tensor
  """
  with tf.variable_scope(name, reuse=reuse):
    input_shape = input.get_shape().as_list()

    weights = _weights("weights",
      shape=[3, 3, k, input_shape[3]])

    if not output_size:
      output_size = input_shape[1]*2
    output_shape = [input_shape[0], output_size, output_size, k]
    fsconv = tf.nn.conv2d_transpose(input, weights,
        output_shape=output_shape,
        strides=[1, 2, 2, 1], padding='SAME')
    normalized = _norm(fsconv, is_training, norm)
    output = tf.nn.relu(normalized)
    return output

### Discriminator layers
def Ck(input, k, slope=0.2, stride=2, reuse=False, norm='instance', is_training=True, name=None):
  """ A 4x4 Convolution-BatchNorm-LeakyReLU layer with k filters and stride 2
  Args:
    input: 4D tensor
    k: integer, number of filters (output depth)
    slope: LeakyReLU's slope
    stride: integer
    norm: 'instance' or 'batch' or None
    is_training: boolean or BoolTensor
    reuse: boolean
    name: string, e.g. 'C64'
  Returns:
    4D tensor
  """
  with tf.variable_scope(name, reuse=reuse):
    weights = _weights("weights",
      shape=[4, 4, input.get_shape()[3], k])

    conv = tf.nn.conv2d(input, weights,
        strides=[1, stride, stride, 1], padding='SAME')

    normalized = _norm(conv, is_training, norm)
    output = _leaky_relu(normalized, slope)
    return output

def last_conv(input, reuse=False, use_sigmoid=False, name=None):
  """ Last convolutional layer of discriminator network
      (1 filter with size 4x4, stride 1)
  Args:
    input: 4D tensor
    reuse: boolean
    use_sigmoid: boolean (False if use lsgan)
    name: string, e.g. 'C64'
  """
  with tf.variable_scope(name, reuse=reuse):
    weights = _weights("weights",
      shape=[4, 4, input.get_shape()[3], 1])
    biases = _biases("biases", [1])

    conv = tf.nn.conv2d(input, weights,
        strides=[1, 1, 1, 1], padding='SAME')
    output = conv + biases
    if use_sigmoid:
      output = tf.sigmoid(output)
    return output

### Helpers
def _weights(name, shape, mean=0.0, stddev=0.02):
  """ Helper to create an initialized Variable
  Args:
    name: name of the variable
    shape: list of ints
    mean: mean of a Gaussian
    stddev: standard deviation of a Gaussian
  Returns:
    A trainable variable
  """
  var = tf.get_variable(
    name, shape,
    initializer=tf.random_normal_initializer(
      mean=mean, stddev=stddev, dtype=tf.float32))
  return var

def _biases(name, shape, constant=0.0):
  """ Helper to create an initialized Bias with constant
  """
  return tf.get_variable(name, shape,
            initializer=tf.constant_initializer(constant))

def _leaky_relu(input, slope):
  return tf.maximum(slope*input, input)

def _norm(input, is_training, norm='instance'):
  """ Use Instance Normalization or Batch Normalization or None
  """
  if norm == 'instance':
    return _instance_norm(input)
  elif norm == 'batch':
    return _batch_norm(input, is_training)
  else:
    return input

def _batch_norm(input, is_training):
  """ Batch Normalization
  """
  with tf.variable_scope("batch_norm"):
    return tf.contrib.layers.batch_norm(input,
                                        decay=0.9,
                                        scale=True,
                                        updates_collections=None,
                                        is_training=is_training)

def _instance_norm(input):
  """ Instance Normalization
  """
  with tf.variable_scope("instance_norm"):
    depth = input.get_shape()[3]
    scale = _weights("scale", [depth], mean=1.0)
    offset = _biases("offset", [depth])
    mean, variance = tf.nn.moments(input, axes=[1,2], keep_dims=True)
    epsilon = 1e-5
    inv = tf.rsqrt(variance + epsilon)
    normalized = (input-mean)*inv
    return scale*normalized + offset

def safe_log(x, eps=1e-12):
  return tf.log(x + eps)

```

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "c15c496a39dc7dabfcf30bedd09331cef6bc4612",
  "sha256": "44afe3169e3563ea46224a1344d9df8dd046eb26c78ab27db95e766ff47aa4fc",
  "lines": 75,
  "bytes": 2764,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/barisgecer/facegan/blob/7df369afab50a26994d7e09ef428f54f7358b617/main.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def main(config):"
    }
  ]
}

```text

import numpy as np
import tensorflow as tf

from trainer import Trainer
from config import get_config
from data_loader import *
from utils import prepare_dirs_and_logger, save_config

def main(config):
    #if config.train_renderer:
    #    config.task = "{}_{}".format(config.task, 'ren')
    #if config.train_regressor:
    #    config.task = "{}_{}".format(config.task, 'reg')
    #    if config.pretrain_generator:
    #        config.task = "{}_{}".format(config.task, 'pregen')
    if config.train_generator:
        config.task = "{}_{}".format(config.task, 'gen')

    prepare_dirs_and_logger(config)

    rng = np.random.RandomState(config.random_seed)
    tf.set_random_seed(config.random_seed)

    if config.is_train:
        data_path = config.data_path
        batch_size = config.batch_size
        do_shuffle = True
    else:
        setattr(config, 'batch_size', 64)
        if config.test_data_path is None:
            data_path = config.data_path
        else:
            data_path = config.test_data_path
        batch_size = config.sample_per_image
        do_shuffle = False

    with tf.device('/cpu:0'):
        data_loader = get_loader(
                data_path, config.batch_size*config.num_gpu, config.input_scale_size,
                config.data_format, config)

        syn_image, syn_label, config.n_id = get_syn_loader(
                config.syn_data_dir, config.batch_size*config.num_gpu, config.syn_scale_size,
                config.data_format, config)

        image_3dmm, annot_3dmm = get_3dmm_loader(
                config.dataset_3dmm_dir, config.batch_size*config.num_gpu, config.syn_scale_size,
                config.data_format, config)

    #image_3dmm_test, annot_3dmm_test, latent_3dmm_test = get_3dmm_loader(
    #        config.dataset_3dmm_test_dir, config.batch_size, config.syn_scale_size,
    #        config.data_format, config.split)

        trainer = Trainer(config, data_loader,syn_image,syn_label, image_3dmm, annot_3dmm) #image_3dmm_test, annot_3dmm_test, latent_3dmm_test )

    if config.is_train:
        save_config(config)
        #if config.train_renderer | (config.cont == 'ren'):
        #    trainer.train_renderer()
        #if config.train_regressor | (config.cont == 'reg'):
        #    trainer.train_regressor()
        if config.train_generator | (config.cont == 'gen'):
            trainer.train()
        elif config.generate_dataset:
            trainer.generate_dataset()
        elif config.fit_dataset:
            trainer.fit_dataset()
    else:
        if not config.load_path:
            raise Exception("[!] You should specify `load_path` to load a pretrained model")
        trainer.test()

if __name__ == "__main__":
    config, unparsed = get_config()
    main(config)


```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "bdee23d4cef2161165181509120993a6d75872b3",
  "sha256": "3e2af6500ac4ca91180f69c4a6c6fc2578c08fb5c15081aa1540c7a70baf9bb7",
  "lines": 97,
  "bytes": 3368,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/barisgecer/facegan/blob/7df369afab50a26994d7e09ef428f54f7358b617/utils.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def prepare_dirs_and_logger(config):"
    },
    {
      "line": 53,
      "text": "def get_time():"
    },
    {
      "line": 56,
      "text": "def save_config(config):"
    },
    {
      "line": 65,
      "text": "def rank(array):"
    },
    {
      "line": 68,
      "text": "def make_grid(tensor, nrow=8, padding=2,"
    },
    {
      "line": 88,
      "text": "def save_image(tensor, filename, nrow=8, padding=2,"
    },
    {
      "line": 95,
      "text": "def save_one_image(tensor, filename):"
    }
  ]
}

```text

from __future__ import print_function

import os
import math
import json
import logging
import numpy as np
from PIL import Image
from datetime import datetime
from shutil import copy2
from glob import glob

def prepare_dirs_and_logger(config):
    formatter = logging.Formatter("%(asctime)s:%(levelname)s::%(message)s")
    logger = logging.getLogger()

    for hdlr in logger.handlers:
        logger.removeHandler(hdlr)

    handler = logging.StreamHandler()
    handler.setFormatter(formatter)

    logger.addHandler(handler)

    # if config.load_path:
    #     if config.load_path.startswith(config.log_dir):
    #         config.model_dir = config.load_path
    #     else:
    #         if config.load_path.startswith(config.task):
    #             config.model_name = config.load_path
    #         else:
    #             config.model_name = "{}_{}".format(config.task, config.load_path)
    # else:
    config.model_name = "{}_{}".format(config.task, get_time())

    if not hasattr(config, 'model_dir'):
        config.model_dir = os.path.join(config.log_dir, config.model_name)
    config.data_path = os.path.join(config.data_dir, config.dataset)
    config.syn_data_dir = os.path.join(config.data_dir, config.syn_dataset)
    config.dataset_3dmm_dir = os.path.join(config.data_dir, config.dataset_3dmm)
    config.dataset_3dmm_test_dir = os.path.join(config.data_dir, config.dataset_3dmm_test)

    for path in [config.log_dir, config.data_dir, config.model_dir]:
        if not os.path.exists(path):
            os.makedirs(path)

    src_path = os.path.join(config.model_dir, 'src')
    if not os.path.exists(src_path):
        os.makedirs(src_path)
    for path in glob("*.py"):
        copy2(path, src_path)

def get_time():
    return datetime.now().strftime("%m%d_%H%M%S")

def save_config(config):
    param_path = os.path.join(config.model_dir, "params.json")

    print("[*] MODEL dir: %s" % config.model_dir)
    print("[*] PARAM path: %s" % param_path)

    with open(param_path, 'w') as fp:
        json.dump(config.__dict__, fp, indent=4, sort_keys=True)

def rank(array):
    return len(array.shape)

def make_grid(tensor, nrow=8, padding=2,
              normalize=False, scale_each=False):
    """Code based on https://github.com/pytorch/vision/blob/master/torchvision/utils.py"""
    nmaps = tensor.shape[0]
    xmaps = min(nrow, nmaps)
    ymaps = int(math.ceil(float(nmaps) / xmaps))
    height, width = int(tensor.shape[1] + padding), int(tensor.shape[2] + padding)
    grid = np.zeros([height * ymaps + 1 + padding // 2, width * xmaps + 1 + padding // 2, 3], dtype=np.uint8)
    k = 0
    for y in range(ymaps):
        for x in range(xmaps):
            if k >= nmaps:
                break
            h, h_width = y * height + 1 + padding // 2, height - padding
            w, w_width = x * width + 1 + padding // 2, width - padding

            grid[h:h+h_width, w:w+w_width] = tensor[k]
            k = k + 1
    return grid

def save_image(tensor, filename, nrow=8, padding=2,
               normalize=False, scale_each=False):
    ndarr = make_grid(tensor, nrow=nrow, padding=padding,
                            normalize=normalize, scale_each=scale_each)
    im = Image.fromarray(ndarr)
    im.save(filename, quality=100)

def save_one_image(tensor, filename):
    im = Image.fr (tensor)
    im.save(filename, quality=100)


```

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "059f9eda36df0761e09ad21cdcef3ba5e65f4a47",
  "sha256": "9d5fdf398ab6e96b3cda9a0c1dd00d5d5e04e66c4c580460f1c1d1835d083285",
  "lines": 197,
  "bytes": 11577,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 103,
    "face": 7,
    "lip": 2
  },
  "url": "https://github.com/barisgecer/facegan/blob/7df369afab50a26994d7e09ef428f54f7358b617/config.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def str2bool(v):"
    },
    {
      "line": 12,
      "text": "def add_argument_group(name):"
    },
    {
      "line": 17,
      "text": "def get_branch_name():"
    },
    {
      "line": 194,
      "text": "def get_config():"
    }
  ]
}

```text

#-*- coding: utf-8 -*-
import argparse
from pygit2 import Repository
import os.path

def str2bool(v):
    return v.lower() in ('true', '1')

arg_lists = []
parser = argparse.ArgumentParser()

def add_argument_group(name):
    arg = parser.add_argument_group(name)
    arg_lists.append(arg)
    return arg

def get_branch_name():
    if (os.name!='nt') & (os.path.isfile("branchname.txt")):
        with open("branchname.txt", "rb") as file:
            branchname = str(file.readline().decode("utf-8"))
    else:
        branchname = Repository('.').head.shorthand
        with open("branchname.txt", "w") as file:
            file.write(branchname)
    return branchname

# Network
net_arg = add_argument_group('Network')
net_arg.add_argument('--input_scale_size', type=int, default=64,#64
                     help='input image will be resized with the given value as width and height')
net_arg.add_argument('--conv_hidden_num', type=int, default=64,
                     choices=[64, 128],help='n in the paper')
net_arg.add_argument('--conv_hidden_num_res', type=int, default=32,
                     choices=[64, 128],help='n in the paper')
net_arg.add_argument('--num_res_rep', type=int, default=3,help='number of residueal blocks')
net_arg.add_argument('--z_num', type=int, default=64, choices=[64, 128])

# Data
data_arg = add_argument_group('Data')
data_arg.add_argument('--dataset', type=str, default='vgg-aligned-64')
data_arg.add_argument('--num_gpu', type=int, default=1)
data_arg.add_argument('--split', type=str, default='train')
data_arg.add_argument('--batch_size', type=int, default=16)
data_arg.add_argument('--grayscale', type=str2bool, default=False)
data_arg.add_argument('--num_worker', type=int, default=4)

# Training / test parameters
train_arg = add_argument_group('Training')

train_arg.add_argument('--cont', type=str, default="",choices=['','ren','reg','gen']) # If set, rest should be False
#train_arg.add_argument('--train_renderer', type=str2bool, default=False)
#train_arg.add_argument('--train_regressor', type=str2bool, default=False)
#train_arg.add_argument('--pretrain_generator', type=str2bool, default=True) # True if not pretrained (when train_reg is False)
train_arg.add_argument('--train_generator', type=str2bool, default=True)
train_arg.add_argument('--generate_dataset', type=str2bool, default=True)
train_arg.add_argument('--fit_dataset', type=str2bool, default=False)

#train_arg.add_argument('--pretrained_ren', type=str, default='pretrained_models/ren')
#train_arg.add_argument('--pretrained_reg', type=str, default='pretrained_models/reg')
train_arg.add_argument('--pretrained_gen', type=str, default='pretrained_models/gen')
train_arg.add_argument('--save_syn_dataset', type=str, default='generated')
train_arg.add_argument('--save_fitting', type=str, default='fitting')
train_arg.add_argument('--pretrained_rec', type=str, default='facenet_model/model-20171214-014943.ckpt-80000', help='Pretrained facenet model')

train_arg.add_argument('--task', type=str, default=get_branch_name(), help='default branch name')
train_arg.add_argument('--is_train', type=str2bool, default=True)
train_arg.add_argument('--optimizer', type=str, default='adam')
train_arg.add_argument('--max_step', type=int, default=248000)
train_arg.add_argument('--lr_update_step', type=int, default=128000)
train_arg.add_argument('--warm_up', type=int, default=5000)
train_arg.add_argument('--d_lr', type=float, default=0.00008)
train_arg.add_argument('--g_lr', type=float, default=0.00008)
train_arg.add_argument('--ren_lr', type=float, default=0.00008)
train_arg.add_argument('--reg_lr', type=float, default=0.00008)
train_arg.add_argument('--lr_lower_boundary', type=float, default=0.000005)
train_arg.add_argument('--beta1', type=float, default=0.5)
train_arg.add_argument('--beta2', type=float, default=0.999)
train_arg.add_argument('--gamma', type=float, default=0.5)
train_arg.add_argument('--lambda_k', type=float, default=0.001)
train_arg.add_argument('--use_gpu', type=str2bool, default=True)
train_arg.add_argument('--lambda_d', type=float, default=0.5, help='')
train_arg.add_argument('--lambda_s', type=float, default=0.5, help='')
train_arg.add_argument('--lambda_p', type=float, default=0.1, help='')
train_arg.add_argument('--lambda_c', type=float, default=0.001, help='')
train_arg.add_argument('--lambda_c_upper', type=float, default=0.001, help='')

train_arg.add_argument('--method_c', type=str, default="magnet",choices=['none','magnet','softmax','center'])
train_arg.add_argument('--relax_c', type=str2bool, default=False)
# Misc
misc_arg = add_argument_group('Misc')
misc_arg.add_argument('--load_path', type=str, default='')
misc_arg.add_argument('--log_step', type=int, default=50)
misc_arg.add_argument('--save_step', type=int, default=20)
misc_arg.add_argument('--num_log_id', type=int, default=16)
misc_arg.add_argument('--num_log_samples', type=int, default=8)
misc_arg.add_argument('--log_level', type=str, default='INFO', choices=['INFO', 'DEBUG', 'WARN'])
misc_arg.add_argument('--log_dir', type=str, default='logs')
misc_arg.add_argument('--data_dir', type=str, default='c:/data')
misc_arg.add_argument('--test_data_path', type=str, default=None,
                      help='directory with images which will be used in test sample generation')
misc_arg.add_argument('--sample_per_image', type=int, default=64,
                      help='# of sample per image during test sample generation')
misc_arg.add_argument('--random_seed', type=int, default=123)

# FaceGAN
misc_arg.add_argument('--n_id', type=int, default=10, help='Number of different identities to be generated')
misc_arg.add_argument('--facenet_scope', type=str, default='InceptionResnetV1', help='Facenet model scope name defined in --model_def')
parser.add_argument('--syn_dataset', type=str,
                    help='Directory where synthetic images generated by 3dmm', default='syn_lsfm-64')
parser.add_argument('--dataset_3dmm_test', type=str,
                    help='Directory where synthetic images generated by 3dmm', default='AFLW-test')
net_arg.add_argument('--syn_scale_size', type=int, default=64,
                     help='input image will be resized with the given value as width and height')
parser.add_argument('--dataset_3dmm', type=str,
                    help='A real dataset annotated with 3dmm renderings', default='paired_data-64')



# FaceNet
parser.add_argument('--logs_base_dir', type=str,
                    help='Directory where to write event logs.', default='~/logs/facenet')
parser.add_argument('--models_base_dir', type=str,
                    help='Directory where to write trained models and checkpoints.', default='~/models/facenet')
parser.add_argument('--gpu_memory_fraction', type=float,
                    help='Upper bound on the amount of GPU memory that will be used by the process.', default=1.0)
parser.add_argument('--pretrained_model', type=str,
                    help='Load a pretrained model before training starts.')
parser.add_argument('--model_def', type=str,
                    help='Model definition. Points to a module containing the definition of the inference graph.',
                    default='models.nn4')
parser.add_argument('--max_nrof_epochs', type=int,
                    help='Number of epochs to run.', default=500)
parser.add_argument('--image_size', type=int,
                    help='Image size (height, width) in pixels.', default=160)
parser.add_argument('--epoch_size', type=int,
                    help='Number of batches per epoch.', default=1000)
parser.add_argument('--embedding_size', type=int,
                    help='Dimensionality of the embedding.', default=128)
parser.add_argument('--random_crop',
                    help='Performs random cropping of training images. If false, the center image_size pixels from the training images are used. ' +
                         'If the size of the images in the data directory is equal to image_size no cropping is performed',
                    action='store_true')
parser.add_argument('--random_flip',
                    help='Performs random horizontal flipping of training images.', action='store_true')
parser.add_argument('--random_rotate',
                    help='Performs random rotations of training images.', action='store_true')
parser.add_argument('--keep_probability', type=float,
                    help='Keep probability of dropout for the fully connected layer(s).', default=1.0)
parser.add_argument('--weight_decay', type=float,
                    help='L2 weight regularization.', default=5e-5)
parser.add_argument('--center_loss_factor', type=float,
                    help='Center loss factor.', default=0.0)
parser.add_argument('--center_loss_alfa', type=float,
                    help='Center update rate for center loss.', default=0.95)
parser.add_argument('--learning_rate', type=float,
                    help='Initial learning rate. If set to a negative value a learning rate ' +
                         'schedule can be specified in the file "learning_rate_schedule.txt"', default=0.1)
parser.add_argument('--learning_rate_decay_epochs', type=int,
                    help='Number of epochs between learning rate decay.', default=100)
parser.add_argument('--learning_rate_decay_factor', type=float,
                    help='Learning rate decay factor.', default=1.0)
parser.add_argument('--moving_average_decay', type=float,
                    help='Exponential decay for tracking of training parameters.', default=0.9999)
parser.add_argument('--seed', type=int,
                    help='Random seed.', default=666)
parser.add_argument('--nrof_preprocess_threads', type=int,
                    help='Number of preprocessing (data loading and augmentation) threads.', default=4)
parser.add_argument('--log_histograms',
                    help='Enables logging of weight/bias histograms in tensorboard.', action='store_true')
parser.add_argument('--learning_rate_schedule_file', type=str,
                    help='File containing the learning rate schedule that is used when learning_rate is set to to -1.',
                    default='data/learning_rate_schedule.txt')
parser.add_argument('--filter_filename', type=str,
                    help='File containing image data used for dataset filtering', default='')
parser.add_argument('--filter_percentile', type=float,
                    help='Keep only the percentile images closed to its class center', default=100.0)
parser.add_argument('--filter_min_nrof_images_per_class', type=int,
                    help='Keep only the classes with this number of examples or more', default=0)

# Parameters for validation on LFW
parser.add_argument('--lfw_pairs', type=str,
                    help='The file containing the pairs to use for validation.', default='data/pairs.txt')
parser.add_argument('--lfw_file_ext', type=str,
                    help='The file extension for the LFW dataset.', default='jpg', choices=['jpg', 'png'])
parser.add_argument('--lfw_dir', type=str,
                    help='Path to the data directory containing aligned face patches.', default='')
parser.add_argument('--lfw_batch_size', type=int,
                    help='Number of images to process in a batch in the LFW test set.', default=100)
parser.add_argument('--lfw_nrof_folds', type=int,
                    help='Number of folds to use for cross validation. Mainly used for testing.', default=10)



def get_config():
    config, unparsed = parser.parse_known_args()
    setattr(config, 'data_format', 'NHWC')
    return config, unparsed


```

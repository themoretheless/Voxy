# Facial rendering source scan: tranluan/Nonlinear_Face_3DMM

Repository: https://github.com/tranluan/Nonlinear_Face_3DMM

Pinned source revision: 662098a602d542c3505cd16ba01dd302f33eeee8

Discovery topic: face model 3D

Description: Source code for "Nonlinear 3D Face Morphable Model"

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: ops.py

{
  "path": "ops.py",
  "blob_sha": "a98025c03ab46bdb0e2ca240b7ad8bf22d88533d",
  "sha256": "a35f11f6021051851ff0f99f056c467d94b773590526636d691a61cde7ced37a",
  "lines": 262,
  "bytes": 10263,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/tranluan/Nonlinear_Face_3DMM/blob/662098a602d542c3505cd16ba01dd302f33eeee8/ops.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def make_parallel(fn, num_gpus, **kwargs):"
    },
    {
      "line": 35,
      "text": "def __init__(self, epsilon=1e-5, momentum = 0.9, name=\"batch_norm\"):"
    },
    {
      "line": 41,
      "text": "def __call__(self, x, train=True, reuse=False, trainable=True ):"
    },
    {
      "line": 53,
      "text": "def get_shape(tensor):"
    },
    {
      "line": 60,
      "text": "def binary_cross_entropy(preds, targets, name=None):"
    },
    {
      "line": 78,
      "text": "def conv_cond_concat(x, y):"
    },
    {
      "line": 84,
      "text": "def conv2d(input_, output_dim,"
    },
    {
      "line": 98,
      "text": "def deconv2d(input_, output_shape,"
    },
    {
      "line": 130,
      "text": "def maxpool2d(x, k=2, padding='VALID'):"
    },
    {
      "line": 134,
      "text": "def prelu(x, name, reuse = False):"
    },
    {
      "line": 143,
      "text": "def relu(x, name='relu'):"
    },
    {
      "line": 146,
      "text": "def lrelu(x, leak=0.2, name=\"lrelu\"):"
    },
    {
      "line": 149,
      "text": "def elu(x, name='elu'):"
    },
    {
      "line": 152,
      "text": "def linear(input_, output_size, scope=\"Linear\", reuse = False, stddev=0.02, bias_start=0.0, with_w=False):"
    },
    {
      "line": 166,
      "text": "def triplet_loss(anchor_output, positive_output, negative_output, margin = 0.2 ):"
    },
    {
      "line": 174,
      "text": "def cosine_loss(anchor_output, positive_output):"
    },
    {
      "line": 181,
      "text": "def cosine_triplet_loss(anchor_output, positive_output, negative_output, margin = 0.2 ):"
    },
    {
      "line": 193,
      "text": "def norm_loss(predictions, labels, mask = None, loss_type = 'l1', reduce_mean = True, p=1):"
    },
    {
      "line": 226,
      "text": "def total_variation(images, mask, name=None):"
    }
  ]
}

```text

import math
import numpy as np 
import tensorflow as tf

from tensorflow.python.framework import ops

from utils import *

def make_parallel(fn, num_gpus, **kwargs):
    print("Making make_parallel for %d gpu(s)" % num_gpus)
    in_splits = {}
    for k, v in kwargs.items():
        if type(v) == list:
            in_splits[k] = zip(*[iter(v)]*( int( len(v) / num_gpus ) ))
        else:
            in_splits[k] = tf.split(v, num_gpus)

    out_split = []
    for i in range(num_gpus):
        with tf.device(tf.DeviceSpec(device_type="GPU", device_index=i)):
            with tf.variable_scope(tf.get_variable_scope(), reuse= i!=0):# tf.AUTO_REUSE):
                out_split.append(fn(**{k : v[i] for k, v in in_splits.items()}))

    output_num = len(out_split[0])
    output = []
    for i in range(output_num):
        output.append([])
        for j in range(num_gpus):
            output[i].append(out_split[j][i])

    return output

class batch_norm(object):
    """Code modification of http://stackoverflow.com/a/33950177"""
    def __init__(self, epsilon=1e-5, momentum = 0.9, name="batch_norm"):
        with tf.variable_scope(name):
          self.epsilon  = epsilon
          self.momentum = momentum
          self.name = name

    def __call__(self, x, train=True, reuse=False, trainable=True ):
        return tf.contrib.layers.batch_norm(x,
                      decay=self.momentum, 
                      updates_collections=None,
                      epsilon=self.epsilon,
                      fused=True,
                      scale=True,
                      trainable=trainable,
                      reuse = reuse,
                      is_training=train,
                      scope=self.name)

def get_shape(tensor):
    static_shape = tensor.shape.as_list()
    dynamic_shape = tf.unstack(tf.shape(tensor))
    dims = [s[1] if s[0] is None else s[0]
            for s in zip(static_shape, dynamic_shape)]
    return dims

def binary_cross_entropy(preds, targets, name=None):
    """Computes binary cross entropy given `preds`.

    For brevity, let `x = `, `z = targets`.  The logistic loss is

        loss(x, z) = - sum_i (x[i] * log(z[i]) + (1 - x[i]) * log(1 - z[i]))

    Args:
        preds: A `Tensor` of type `float32` or `float64`.
        targets: A `Tensor` of the same type and shape as `preds`.
    """
    eps = 1e-12
    with ops.op_scope([preds, targets], name, "bce_loss") as name:
        preds = ops.convert_to_tensor(preds, name="preds")
        targets = ops.convert_to_tensor(targets, name="targets")
        return tf.reduce_mean(-(targets * tf.log(preds + eps) +
                              (1. - targets) * tf.log(1. - preds + eps)))

def conv_cond_concat(x, y):
    """Concatenate conditioning vector on feature map axis."""
    x_shapes = x.get_shape()
    y_shapes = y.get_shape()
    return tf.concat(axis=3, values=[x, y*tf.ones([x_shapes[0], x_shapes[1], x_shapes[2], y_shapes[3]])])

def conv2d(input_, output_dim, 
           k_h=3, k_w=3, d_h=2, d_w=2, use_bias=True, stddev=0.02,
           name="conv2d", reuse = False):
    with tf.variable_scope(name, reuse = reuse):
        w = tf.get_variable('w', [k_h, k_w, input_.get_shape()[-1], output_dim],
                            initializer=tf.truncated_normal_initializer(stddev=stddev))
        conv = tf.nn.conv2d(input_, w, strides=[1, d_h, d_w, 1], padding='SAME')

        if use_bias:
            biases = tf.get_variable('biases', [output_dim], initializer=tf.constant_initializer(0.0))
            conv = tf.reshape(tf.nn.bias_add(conv, biases), get_shape(conv))

        return conv

def deconv2d(input_, output_shape,
             kernel_size=(3,3), strides=(2,2), stddev=0.02, use_bias = True,
             name="deconv2d", with_w=False, reuse = False):
    '''
    with tf.variable_scope(name, reuse = reuse):
        # filter : [height, width, output_channels, in_channels]
        w = tf.get_variable('w', [k_h, k_h, output_shape[-1], input_.get_shape()[-1]],
                            initializer=tf.random_normal_initializer(stddev=stddev))
        
        try:
            deconv = tf.nn.conv2d_transpose(input_, w, output_shape=output_shape,
                                strides=[1, d_h, d_w, 1])

        # Support for verisons of TensorFlow before 0.7.0
        except AttributeError:
            deconv = tf.nn.deconv2d(input_, w, output_shape=output_shape,
                                strides=[1, d_h, d_w, 1])

        biases = tf.get_variable('biases', [output_shape[-1]], initializer=tf.constant_initializer(0.0))
        deconv = tf.reshape(tf.nn.bias_add(deconv, biases), deconv.get_shape())

        if with_w:
            return deconv, w, biases
        else:
            return deconv
    '''
    if type(output_shape) == list():
        output_shape = output_shape[-1]
    return tf.layers.conv2d_transpose(input_, output_shape, kernel_size, strides, padding='SAME', data_format='channels_last', activation=None, use_bias=use_bias, 
                                        kernel_initializer=tf.random_normal_initializer(stddev=stddev), bias_initializer=tf.zeros_initializer(),
                                        trainable=True, name=name, reuse=reuse)

def maxpool2d(x, k=2, padding='VALID'):
    return tf.nn.max_pool(x, ksize=[1, k, k, 1], strides=[1, k, k, 1], padding=padding)

       
def prelu(x, name, reuse = False):
    shape = x.get_shape().as_list()[-1:]

    with tf.variable_scope(name, reuse = reuse):
        alphas = tf.get_variable('alpha', shape, tf.float32,
                            initializer=tf.constant_initializer(value=0.2))

        return tf.nn.relu(x) + tf.multiply(alphas, (x - tf.abs(x))) * 0.5

def relu(x, name='relu'):
    return tf.nn.relu(x, name)  

def lrelu(x, leak=0.2, name="lrelu"):
  return tf.maximum(x, leak*x)

def elu(x, name='elu'):
  return tf.nn.elu(x, name)

def linear(input_, output_size, scope="Linear", reuse = False, stddev=0.02, bias_start=0.0, with_w=False):
    shape = input_.get_shape().as_list()
    print(shape)

    with tf.variable_scope(scope or "Linear", reuse = reuse):
        matrix = tf.get_variable("Matrix", [shape[1], output_size], tf.float32,
                                 tf.random_normal_initializer(stddev=stddev))
        bias = tf.get_variable("bias", [output_size],
            initializer=tf.constant_initializer(bias_start))
        if with_w:
            return tf.matmul(input_, matrix) + bias, matrix, bias
        else:
            return tf.matmul(input_, matrix) + bias

def triplet_loss(anchor_output, positive_output, negative_output, margin = 0.2 ):
    d_pos = tf.reduce_mean(tf.square(anchor_output - positive_output), 1)
    d_neg = tf.reduce_mean(tf.square(anchor_output - negative_output), 1)

    loss = tf.maximum(0., margin + d_pos - d_neg)
    
    return loss

def cosine_loss(anchor_output, positive_output):
    anchor_output_norm = tf.nn.l2_normalize(anchor_output, 1)
    positive_output_norm = tf.nn.l2_normalize(positive_output, 1)
    loss = 1 - tf.reduce_sum(tf.multiply(anchor_output_norm, positive_output_norm), 1)

    return loss

def cosine_triplet_loss(anchor_output, positive_output, negative_output, margin = 0.2 ):
    anchor_output_norm = tf.nn.l2_normalize(anchor_output, 1)
    positive_output_norm = tf.nn.l2_normalize(positive_output, 1)
    negative_output_norm = tf.nn.l2_normalize(negative_output, 1)

    sim_pos = tf.reduce_sum(tf.multiply(anchor_output_norm, positive_output_norm), 1)
    sim_neg = tf.reduce_sum(tf.multiply(anchor_output_norm, negative_output_norm), 1)

    loss = tf.maximum(0., margin - sim_pos + sim_neg)

    return loss

def norm_loss(predictions, labels, mask = None, loss_type = 'l1', reduce_mean = True, p=1):
    from tensorflow.python.ops import array_ops

    assert (loss_type in ['l1', 'l2', 'l2,1']), "Suporting loss type is ['l1', 'l2', 'l2,1']"

    diff = predictions - labels
    if mask != None:
        diff = tf.multiply(diff, mask)

    inputs_shape = array_ops.shape(diff)

    if loss_type == 'l1':
        loss = tf.reduce_sum( tf.abs(diff) )

    elif loss_type == 'l2':
        loss = tf.nn.l2_loss(diff)

    elif loss_type == 'l2,1':
        #inputs_rank = tf.cast(labels.get_shape().ndims, tf.int32)
        #spatial_dims = array_ops.slice(inputs_shape, [1], [2])
        #batch_dim = array_ops.slice(inputs_shape, [0], [1])

        loss = tf.sqrt( tf.reduce_sum ( tf.square (diff) + 1e-16, axis = [-1] ) )
        if p!= 1:
            loss = tf.pow(loss, p)
        loss = tf.reduce_sum(loss)

    if reduce_mean:
        numel = tf.cast(tf.reduce_prod(inputs_shape), diff.dtype)
        loss = tf.div(loss, numel)

    return loss
        
def total_variation(images, mask, name=None):

    ndims = images.get_shape().ndims
    if ndims == 3:
        # The input is a single image with shape [height, width, channels].

        # Calculate the difference of neighboring pixel-values.
        # The images are shifted one pixel along the height and width by slicing.
        pixel_dif1 = images[1:, :, :] - images[:-1, :, :]
        pixel_dif2 = images[:, 1:, :] - images[:, :-1, :]

        # Sum for all axis. (None is an alias for all axis.)
        sum_axis = None
    elif ndims == 4:
        # The input is a batch of images with shape:
        # [batch, height, width, channels].

        # Calculate the difference of neighboring pixel-values.
        # The images are shifted one pixel along the height and width by slicing.
        pixel_dif1 = images[:, 1:, :, :] - images[:, :-1, :, :]
        pixel_dif2 = images[:, :, 1:, :] - images[:, :, :-1, :]

        # Only sum for the last 3 axis.
        # This results in a 1-D tensor with the total variation for each image.
        sum_axis = [1, 2, 3]
    else:
        raise ValueError('\'images\' must be either 3 or 4-dimensional.')

    pixel_dif1 = tf.multiply(pixel_dif1, mask[:, 1:,  :, :])
    pixel_dif2 = tf.multiply(pixel_dif2, mask[:,  :, 1:, :])

    # Calculate the total variation by taking the absolute value of the
    # pixel-differences and summing over the appropriate axis.
    tot_var = (tf.reduce_sum(tf.abs(pixel_dif1), axis=sum_axis) +
               tf.reduce_sum(tf.abs(pixel_dif2), axis=sum_axis))

    return tot_var 

```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "6df08c046fe763d861c1d877d248552945a9dc4f",
  "sha256": "21b26bc3fff6a56c598a6be03db3e8a0c61b75e7263ab4438026a6f61b150bd2",
  "lines": 304,
  "bytes": 10028,
  "selection_score": 0,
  "keyword_line_counts": {
    "lip": 2
  },
  "url": "https://github.com/tranluan/Nonlinear_Face_3DMM/blob/662098a602d542c3505cd16ba01dd302f33eeee8/utils.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def get_image(image_path, image_size, is_crop=True, is_random_crop = False, resize_w=64, is_grayscale = False):"
    },
    {
      "line": 25,
      "text": "def save_images(images, size, image_path, inverse = True):"
    },
    {
      "line": 37,
      "text": "def imread(path, is_grayscale = False):"
    },
    {
      "line": 43,
      "text": "def merge_images(images, size):"
    },
    {
      "line": 46,
      "text": "def merge(images, size):"
    },
    {
      "line": 68,
      "text": "def imresize(img, sz):"
    },
    {
      "line": 71,
      "text": "def imsave(images, size, path):"
    },
    {
      "line": 79,
      "text": "def center_crop(x, crop_h, crop_w=None, resize_w=64):"
    },
    {
      "line": 88,
      "text": "def random_crop(x, crop_h, crop_w=None, with_crop_size=None ):"
    },
    {
      "line": 103,
      "text": "def crop(x, crop_h, crop_w, j, i):"
    },
    {
      "line": 112,
      "text": "def transform(image, npx=64, is_crop=True, is_random_crop=True, resize_w=64):"
    },
    {
      "line": 123,
      "text": "def inverse_transform(images):"
    },
    {
      "line": 127,
      "text": "def to_json(output_path, *layers):"
    },
    {
      "line": 190,
      "text": "def make_gif(images, fname, duration=2, true_image=False):"
    },
    {
      "line": 193,
      "text": "def make_frame(t):"
    },
    {
      "line": 207,
      "text": "def visualize(sess, dcgan, config, option):"
    },
    {
      "line": 262,
      "text": "def load_300W_LP_dataset(dataset):"
    },
    {
      "line": 299,
      "text": "def image2texture_fn(image_fn):"
    }
  ]
}

```text

"""
Some codes from https://github.com/Newmu/dcgan_code
"""
from __future__ import division
import math
#import csv
import json
import random
import pprint
import scipy.misc
import numpy as np
from glob import glob
import os
#import matplotlib.pyplot as plt
from time import gmtime, strftime
from config import _300W_LP_DIR

pp = pprint.PrettyPrinter()

get_stddev = lambda x, k_h, k_w: 1/math.sqrt(k_w*k_h*x.get_shape()[-1])

def get_image(image_path, image_size, is_crop=True, is_random_crop = False, resize_w=64, is_grayscale = False):
    return transform(imread(image_path, is_grayscale), image_size, is_crop, is_random_crop, resize_w)

def save_images(images, size, image_path, inverse = True):
    if len(size) == 1:
        size= [size, -1]
    if size[1] == -1:
        size[1] = int(math.ceil(images.shape[0]/size[0]))
    if size[0] == -1:
        size[0] = int(math.ceil(images.shape[0]/size[1]))        
    if (inverse):
        images = inverse_transform(images)

    return imsave(images, size, image_path)

def imread(path, is_grayscale = False):
    if (is_grayscale):
        return scipy.misc.imread(path, flatten = True).astype(np.float)
    else:
        return scipy.misc.imread(path).astype(np.float)

def merge_images(images, size):
    return inverse_transform(images)

def merge(images, size):
    h, w = images.shape[1], images.shape[2]
    nn = images.shape[0]

    if size[1] < 0:
        size[1] = int(math.ceil(nn/size[0]))
    if size[0] < 0:
        size[0] = int(math.ceil(nn/size[1]))


    if (images.ndim == 4):
        img = np.zeros((h * size[0], w * size[1], 3))
        for idx, image in enumerate(images):
            i = idx % size[1]
            j = idx // size[1]
            img[j*h:j*h+h, i*w:i*w+w, :] = image
    else:
        img = images


    return img

def imresize(img, sz):
    return scipy.misc.imresize(img, sz)

def imsave(images, size, path):
    img = merge(images, size)

    #plt.imshow(img)
    #plt.show()
    
    return scipy.misc.imsave(path, img)

def center_crop(x, crop_h, crop_w=None, resize_w=64):
    if crop_w is None:
        crop_w = crop_h
    h, w = x.shape[:2]
    j = int(round((h - crop_h)/2.))
    i = int(round((w - crop_w)/2.))
    return scipy.misc.imresize(x[j:j+crop_h, i:i+crop_w],
                               [resize_w, resize_w])

def random_crop(x, crop_h, crop_w=None, with_crop_size=None ):
    if crop_w is None:
        crop_w = crop_h
    if with_crop_size is None:
        with_crop_size = False
    h, w = x.shape[:2]

    j = random.randint(0, h - crop_h)
    i = random.randint(0, w - crop_w)

    if with_crop_size:
        return x[j:j+crop_h, i:i+crop_w,:], j, i
    else:
        return x[j:j+crop_h, i:i+crop_w,:]

def crop(x, crop_h, crop_w, j, i):
    if crop_w is None:
        crop_w = crop_h
    
    return x[j:j+crop_h, i:i+crop_w]


    #return scipy.misc.imresize(x, [96, 96] )

def transform(image, npx=64, is_crop=True, is_random_crop=True, resize_w=64):
    # npx : # of pixels width/height of image
    if is_crop:
        if is_random_crop:
            cropped_image = random_crop(image, npx)
        else:  
            cropped_image = center_crop(image, npx, resize_w=resize_w)
    else:
        cropped_image = image
    return np.array(cropped_image)/127.5 - 1.

def inverse_transform(images):
    return (images+1.)/2.


def to_json(output_path, *layers):
    with open(output_path, "w") as layer_f:
        lines = ""
        for w, b, bn in layers:
            layer_idx = w.name.split('/')[0].split('h')[1]

            B = b.eval()

            if "lin/" in w.name:
                W = w.eval()
                depth = W.shape[1]
            else:
                W = np.rollaxis(w.eval(), 2, 0)
                depth = W.shape[0]

            biases = {"sy": 1, "sx": 1, "depth": depth, "w": ['%.2f' % elem for elem in list(B)]}
            if bn != None:
                gamma = bn.gamma.eval()
                beta = bn.beta.eval()

                gamma = {"sy": 1, "sx": 1, "depth": depth, "w": ['%.2f' % elem for elem in list(gamma)]}
                beta = {"sy": 1, "sx": 1, "depth": depth, "w": ['%.2f' % elem for elem in list(beta)]}
            else:
                gamma = {"sy": 1, "sx": 1, "depth": 0, "w": []}
                beta = {"sy": 1, "sx": 1, "depth": 0, "w": []}

            if "lin/" in w.name:
                fs = []
                for w in W.T:
                    fs.append({"sy": 1, "sx": 1, "depth": W.shape[0], "w": ['%.2f' % elem for elem in list(w)]})

                lines += """
                    var layer_%s = {
                        "layer_type": "fc", 
                        "sy": 1, "sx": 1, 
                        "out_sx": 1, "out_sy": 1,
                        "stride": 1, "pad": 0,
                        "out_depth": %s, "in_depth": %s,
                        "biases": %s,
                        "gamma": %s,
                        "beta": %s,
                        "filters": %s
                    };""" % (layer_idx.split('_')[0], W.shape[1], W.shape[0], biases, gamma, beta, fs)
            else:
                fs = []
                for w_ in W:
                    fs.append({"sy": 5, "sx": 5, "depth": W.shape[3], "w": ['%.2f' % elem for elem in list(w_.flatten())]})

                lines += """
                    var layer_%s = {
                        "layer_type": "deconv", 
                        "sy": 5, "sx": 5,
                        "out_sx": %s, "out_sy": %s,
                        "stride": 2, "pad": 1,
                        "out_depth": %s, "in_depth": %s,
                        "biases": %s,
                        "gamma": %s,
                        "beta": %s,
                        "filters": %s
                    };""" % (layer_idx, 2**(int(layer_idx)+2), 2**(int(layer_idx)+2),
                             W.shape[0], W.shape[3], biases, gamma, beta, fs)
        layer_f.write(" ".join(lines.replace("'","").split()))

def make_gif(images, fname, duration=2, true_image=False):
  import moviepy.editor as mpy

  def make_frame(t):
    try:
      x = images[int(len(images)/duration*t)]
    except:
      x = images[-1]

    if true_image:
      return x.astype(np.uint8)
    else:
      return ((x+1)/2*255).astype(np.uint8)

  clip = mpy.VideoClip(make_frame, duration=duration)
  clip.write_gif(fname, fps = len(images) / duration)

def visualize(sess, dcgan, config, option):
  if option == 0:
    z_sample = np.random.uniform(-0.5, 0.5, size=(config.batch_size, dcgan.z_dim))
    samples = sess.run(dcgan.sampler, feed_dict={dcgan.z: z_sample})
    save_images(samples, [8, 8], './samples/test_%s.png' % strftime("%Y-%m-%d %H:%M:%S", gmtime()))
  elif option == 1:
    values = np.arange(0, 1, 1./config.batch_size)
    for idx in xrange(100):
      print(" [*] %d" % idx)
      z_sample = np.zeros([config.batch_size, dcgan.z_dim])
      for kdx, z in enumerate(z_sample):
        z[idx] = values[kdx]

      samples = sess.run(dcgan.sampler, feed_dict={dcgan.z: z_sample})
      save_images(samples, [8, 8], './samples/test_arange_%s.png' % (idx))
  elif option == 2:
    values = np.arange(0, 1, 1./config.batch_size)
    for idx in [random.randint(0, 99) for _ in xrange(100)]:
      print(" [*] %d" % idx)
      z = np.random.uniform(-0.2, 0.2, size=(dcgan.z_dim))
      z_sample = np.tile(z, (config.batch_size, 1))
      #z_sample = np.zeros([config.batch_size, dcgan.z_dim])
      for kdx, z in enumerate(z_sample):
        z[idx] = values[kdx]

      samples = sess.run(dcgan.sampler, feed_dict={dcgan.z: z_sample})
      make_gif(samples, './samples/test_gif_%s.gif' % (idx))
  elif option == 3:
    values = np.arange(0, 1, 1./config.batch_size)
    for idx in xrange(100):
      print(" [*] %d" % idx)
      z_sample = np.zeros([config.batch_size, dcgan.z_dim])
      for kdx, z in enumerate(z_sample):
        z[idx] = values[kdx]

      samples = sess.run(dcgan.sampler, feed_dict={dcgan.z: z_sample})
      make_gif(samples, './samples/test_gif_%s.gif' % (idx))
  elif option == 4:
    image_set = []
    values = np.arange(0, 1, 1./config.batch_size)

    for idx in xrange(100):
      print(" [*] %d" % idx)
      z_sample = np.zeros([config.batch_size, dcgan.z_dim])
      for kdx, z in enumerate(z_sample): z[idx] = values[kdx]

      image_set.append(sess.run(dcgan.sampler, feed_dict={dcgan.z: z_sample}))
      make_gif(image_set[-1], './samples/test_gif_%s.gif' % (idx))

    new_image_set = [merge(np.array([images[idx] for images in image_set]), [10, 10]) \
        for idx in range(64) + range(63, -1, -1)]
    make_gif(new_image_set, './samples/test_gif_merged.gif', duration=8)



def load_300W_LP_dataset(dataset):
    print 'Loading ' + dataset +' ...'
   
    fd = open(_300W_LP_DIR+'/filelist/'+dataset+'_filelist.txt', 'r')
    all_images = []
    for line in fd:
        all_images.append(line.strip())
    fd.close()
    print '    DONE. Finish loading ' + dataset +' with ' + str(len(all_images)) + ' images' 

    fd = open(_300W_LP_DIR+'/filelist/'+dataset+'_param.dat')
    all_paras = np.fromfile(file=fd, dtype=np.float32)
    fd.close()

    idDim = 1
    mDim  = idDim + 8
    poseDim = mDim + 7
    shapeDim = poseDim + 199
    expDim = shapeDim + 29
    texDim = expDim + 40
    ilDim  = texDim + 10
    #colorDim  = ilDim + 7

    all_paras = all_paras.reshape((-1,ilDim)).astype(np.float32)
    pid = all_paras[:,0:idDim]
    m = all_paras[:,idDim:mDim]
    pose = all_paras[:,mDim:poseDim]
    shape = all_paras[:,poseDim:shapeDim]
    exp = all_paras[:,shapeDim:expDim]
    tex = all_paras[:,expDim:texDim]
    il  = all_paras[:,texDim:ilDim]
    #color = all_paras[:,ilDim:colorDim]

    assert (len(all_images) == all_paras.shape[0]),"Number of samples must be the same between images and paras"

    return all_images, pid, m, pose, shape, exp, tex, il

def image2texture_fn(image_fn):
    last = image_fn[-7:].find('_')
    if (last < 0):
        return image_fn
    else:
        return image_fn[:-7 + last] + '_0.png'


```

## Source file: config.py

{
  "path": "config.py",
  "blob_sha": "c601572c5f50b918307ef4080525d912f2921ba9",
  "sha256": "8991d7432e086fe75ccc72a302b95047d9a59689785cfc18cb8cd8a962eac74a",
  "lines": 2,
  "bytes": 79,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/tranluan/Nonlinear_Face_3DMM/blob/662098a602d542c3505cd16ba01dd302f33eeee8/config.py",
  "function_signature_candidates": []
}

```text

_3DMM_DEFINITION_DIR = './3DMM_definition/'
_300W_LP_DIR='./data/300W_LP_crop/'

```

## Source file: _3dmm_utils.py

{
  "path": "_3dmm_utils.py",
  "blob_sha": "0d28638b0337c352eb33f7507d42f1eb92135fb9",
  "sha256": "e1943e2a58288511a7f4cddc4dcf8e7b73828d602dc7a3f1ed3f1d0e65fab039",
  "lines": 130,
  "bytes": 3310,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/tranluan/Nonlinear_Face_3DMM/blob/662098a602d542c3505cd16ba01dd302f33eeee8/_3dmm_utils.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def load_3DMM_tri():"
    },
    {
      "line": 33,
      "text": "def load_3DMM_vertex_tri():"
    },
    {
      "line": 50,
      "text": "def load_3DMM_vt2pixel():"
    },
    {
      "line": 65,
      "text": "def load_3DMM_kpts():"
    },
    {
      "line": 77,
      "text": "def load_3DMM_tri_2d(with_mask = False):"
    },
    {
      "line": 94,
      "text": "def load_Basel_basic(element, is_reduce = False):"
    },
    {
      "line": 95,
      "text": "fn = _3DMM_DEFINITION_DIR + '3DMM_'+element+'_basis.dat'"
    },
    {
      "line": 113,
      "text": "def load_const_alb_mask():"
    },
    {
      "line": 122,
      "text": "def load_3DMM_tri_2d_barycoord():"
    }
  ]
}

```text

"""
Notes: Many of .dat files are written using Matlab. 
Hence, there are "-1" subtraction to Python 0-based indexing
"""
from __future__ import division
import math
import numpy as np
from config import _3DMM_DEFINITION_DIR


VERTEX_NUM = 53215
TRI_NUM = 105840
N = VERTEX_NUM * 3


def load_3DMM_tri():
    # Triangle definition (i.e. from Basel model)

    print 'Loading 3DMM tri ...'
   
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_tri.dat')
    tri = np.fromfile(file=fd, dtype=np.int32)
    fd.close()
    #print tri

    tri = tri.reshape((3,-1)).astype(np.int32)
    tri = tri - 1
    tri = np.append(tri, [[ VERTEX_NUM], [VERTEX_NUM], [VERTEX_NUM]], axis = 1 )

    print '    DONE'
    return tri

def load_3DMM_vertex_tri():
    # Vertex to triangle mapping (list of all trianlge containing the cureent vertex)

    print 'Loading 3DMM vertex tri ...'
   
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_vertex_tri.dat')
    vertex_tri = np.fromfile(file=fd, dtype=np.int32)
    fd.close()

    vertex_tri = vertex_tri.reshape((8,-1)).astype(np.int32)
    #vertex_tri = np.append(vertex_tri, np.zeros([8,1]), 1)
    vertex_tri[vertex_tri == 0] = TRI_NUM + 1
    vertex_tri = vertex_tri - 1

    print '    DONE'
    return vertex_tri

def load_3DMM_vt2pixel():
    # Mapping in UV space

    fd = open(_3DMM_DEFINITION_DIR + 'vertices_2d_u.dat')
    vt2pixel_u = np.fromfile(file=fd, dtype=np.float32)
    vt2pixel_u = np.append(vt2pixel_u - 1, 0)
    fd.close()

    fd = open(_3DMM_DEFINITION_DIR + 'vertices_2d_v.dat')
    vt2pixel_v = np.fromfile(file=fd, dtype=np.float32)
    vt2pixel_v = np.append(vt2pixel_v - 1, 0)
    fd.close()

    return vt2pixel_u, vt2pixel_v

def load_3DMM_kpts():
    # 68 keypoints indices

    print 'Loading 3DMM keypoints ...'
   
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_keypoints.dat')
    kpts = np.fromfile(file=fd, dtype=np.int32)
    kpts = kpts.reshape((-1, 1))
    fd.close()

    return kpts - 1

def load_3DMM_tri_2d(with_mask = False):
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_tri_2d.dat')
    tri_2d = np.fromfile(file=fd, dtype=np.int32)
    fd.close()

    tri_2d = tri_2d.reshape(192, 224)

    tri_mask = tri_2d != 0

    tri_2d[tri_2d == 0] = TRI_NUM+1 #VERTEX_NUM + 1
    tri_2d = tri_2d - 1

    if with_mask:
        return tri_2d, tri_mask

    return tri_2d

def load_Basel_basic(element, is_reduce = False):
    fn = _3DMM_DEFINITION_DIR + '3DMM_'+element+'_basis.dat'
    print 'Loading ' + fn + ' ...'


   
    fd = open(fn)
    all_paras = np.fromfile(file=fd, dtype=np.float32)
    fd.close()
 
    all_paras = np.transpose(all_paras.reshape((-1,N)).astype(np.float32))
 
    mu = all_paras[:,0]
    w  = all_paras[:,1:]

    print '    DONE'

    return mu, w

def load_const_alb_mask():
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_const_alb_mask.dat')
    const_alb_mask = np.fromfile(file=fd, dtype=np.uint8)
    fd.close()
    const_alb_mask = const_alb_mask - 1
    const_alb_mask = const_alb_mask.reshape((-1,2)).astype(np.uint8)

    return const_alb_mask

def load_3DMM_tri_2d_barycoord():
    fd = open(_3DMM_DEFINITION_DIR + '3DMM_tri_2d_barycoord.dat')
    tri_2d_barycoord = np.fromfile(file=fd, dtype=np.float32)
    fd.close()

    tri_2d_barycoord = tri_2d_barycoord.reshape(192, 224, 3)
    

    return tri_2d_barycoord

```

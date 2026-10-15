# Facial rendering source scan: xiaoyiT/Course-Project---Speech-Driven-Facial-Animation

Repository: https://github.com/xiaoyiT/Course-Project---Speech-Driven-Facial-Animation

Pinned source revision: 6fbfb4debfb54fb08a5750ec61b745f523ac208a

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: DCGAN/ops.py

{
  "path": "DCGAN/ops.py",
  "blob_sha": "3b0441c9c47daf60bd4733b59a5aaeb733eb88f7",
  "sha256": "f7f46786e3db3c9421a61fcaa87f06b18e75ea105abd8ddd3893e5f9e0c07f40",
  "lines": 121,
  "bytes": 4367,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/xiaoyiT/Course-Project---Speech-Driven-Facial-Animation/blob/6fbfb4debfb54fb08a5750ec61b745f523ac208a/DCGAN/ops.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def concat(tensors, axis, *args, **kwargs):"
    },
    {
      "line": 26,
      "text": "def concat(tensors, axis, *args, **kwargs):"
    },
    {
      "line": 30,
      "text": "def __init__(self, epsilon=1e-5, momentum = 0.9, name=\"batch_norm\"):"
    },
    {
      "line": 36,
      "text": "def __call__(self, x, train=True):"
    },
    {
      "line": 45,
      "text": "def conv_cond_concat(x, y):"
    },
    {
      "line": 52,
      "text": "def conv2d(input_, output_dim,"
    },
    {
      "line": 65,
      "text": "def unpool(value, name='unpool'):"
    },
    {
      "line": 82,
      "text": "def deconv2d(input_, output_shape,"
    },
    {
      "line": 107,
      "text": "def lrelu(x, leak=0.2, name=\"lrelu\"):"
    },
    {
      "line": 110,
      "text": "def linear(input_, output_size, scope=None, stddev=0.02, bias_start=0.0, with_w=False):"
    }
  ]
}

```text

import math
import numpy as np 
import tensorflow as tf

from tensorflow.python.framework import ops

from utils import *

try:
  image_summary = tf.image_summary
  scalar_summary = tf.scalar_summary
  histogram_summary = tf.histogram_summary
  merge_summary = tf.merge_summary
  SummaryWriter = tf.train.SummaryWriter
except:
  image_summary = tf.summary.image
  scalar_summary = tf.summary.scalar
  histogram_summary = tf.summary.histogram
  merge_summary = tf.summary.merge
  SummaryWriter = tf.summary.FileWriter

if "concat_v2" in dir(tf):
  def concat(tensors, axis, *args, **kwargs):
    return tf.concat_v2(tensors, axis, *args, **kwargs)
else:
  def concat(tensors, axis, *args, **kwargs):
    return tf.concat(tensors, axis, *args, **kwargs)

class batch_norm(object):
  def __init__(self, epsilon=1e-5, momentum = 0.9, name="batch_norm"):
    with tf.variable_scope(name):
      self.epsilon  = epsilon
      self.momentum = momentum
      self.name = name

  def __call__(self, x, train=True):
    return tf.contrib.layers.batch_norm(x,
                      decay=self.momentum, 
                      updates_collections=None,
                      epsilon=self.epsilon,
                      scale=True,
                      is_training=train,
                      scope=self.name)

def conv_cond_concat(x, y):
  """Concatenate conditioning vector on feature map axis."""
  x_shapes = x.get_shape()
  y_shapes = y.get_shape()
  return concat([
    x, y*tf.ones([x_shapes[0], x_shapes[1], x_shapes[2], y_shapes[3]])], 3)

def conv2d(input_, output_dim, 
       k_h=5, k_w=5, d_h=2, d_w=2, stddev=0.02,
       name="conv2d"):
  with tf.variable_scope(name):
    w = tf.get_variable('w', [k_h, k_w, input_.get_shape()[-1], output_dim],
              initializer=tf.truncated_normal_initializer(stddev=stddev))
    conv = tf.nn.conv2d(input_, w, strides=[1, d_h, d_w, 1], padding='SAME')

    biases = tf.get_variable('biases', [output_dim], initializer=tf.constant_initializer(0.0))
    conv = tf.reshape(tf.nn.bias_add(conv, biases), conv.get_shape())

    return conv

def unpool(value, name='unpool'):
    """N-dimensional version of the unpooling operation from
    https://www.robots.ox.ac.uk/~vgg/rg/papers/Dosovitskiy_Learning_to_Generate_2015_CVPR_paper.pdf

    :param value: A Tensor of shape [b, d0, d1, ..., dn, ch]
    :return: A Tensor of shape [b, 2*d0, 2*d1, ..., 2*dn, ch]
    """
    with tf.name_scope(name) as scope:
        sh = value.get_shape().as_list()
        dim = len(sh[1:-1])
        out = (tf.reshape(value, [-1] + sh[-dim:]))
        for i in range(dim, 0, -1):
            out = tf.concat([out, tf.zeros_like(out)],i)
        out_size = [-1] + [s * 2 for s in sh[1:-1]] + [sh[-1]]
        out = tf.reshape(out, out_size, name=scope)
    return out

def deconv2d(input_, output_shape,
       k_h=5, k_w=5, d_h=2, d_w=2, stddev=0.02,
       name="deconv2d", with_w=False):
  with tf.variable_scope(name):
    # filter : [height, width, output_channels, in_channels]
    w = tf.get_variable('w', [k_h, k_w, output_shape[-1], input_.get_shape()[-1]],
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
     
def lrelu(x, leak=0.2, name="lrelu"):
  return tf.maximum(x, leak*x)

def linear(input_, output_size, scope=None, stddev=0.02, bias_start=0.0, with_w=False):
  shape = input_.get_shape().as_list()

  with tf.variable_scope(scope or "Linear"):
    matrix = tf.get_variable("Matrix", [shape[1], output_size], tf.float32,
                 tf.random_normal_initializer(stddev=stddev))
    bias = tf.get_variable("bias", [output_size],
      initializer=tf.constant_initializer(bias_start))
    if with_w:
      return tf.matmul(input_, matrix) + bias, matrix, bias
    else:
      return tf.matmul(input_, matrix) + bias


```

## Source file: DCGAN/main.py

{
  "path": "DCGAN/main.py",
  "blob_sha": "697b19484b16143cf99ddc0d816b75253395a8cb",
  "sha256": "e17bd080b4375ced27fc8a3274afe4ae970c43cbfafc9a1c6d073d8658093776",
  "lines": 78,
  "bytes": 3334,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/xiaoyiT/Course-Project---Speech-Driven-Facial-Animation/blob/6fbfb4debfb54fb08a5750ec61b745f523ac208a/DCGAN/main.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def main(_):"
    }
  ]
}

```text

import os
import scipy.misc
import numpy as np

from mygan import DCGAN
from utils import pp, visualize, to_json, show_all_variables

import tensorflow as tf

flags = tf.app.flags
flags.DEFINE_integer("epoch", 25, "Epoch to train [25]")
flags.DEFINE_float("learning_rate", 0.0002, "Learning rate of for adam [0.0002]")
flags.DEFINE_float("beta1", 0.5, "Momentum term of adam [0.5]")
flags.DEFINE_integer("train_size", np.inf, "The size of train images [np.inf]")
flags.DEFINE_integer("batch_size", 32, "The size of batch images [64]")
flags.DEFINE_integer("input_height", 256, "The size of image to use (will be center cropped). [108]")
flags.DEFINE_integer("input_width", 256, "The size of image to use (will be center cropped). If None, same value as input_height [None]")
flags.DEFINE_integer("output_height", 256, "The size of the output images to produce [64]")
flags.DEFINE_integer("output_width", 256, "The size of the output images to produce. If None, same value as output_height [None]")
flags.DEFINE_string("dataset", "fr", "The name of dataset [celebA, mnist, lsun]")
flags.DEFINE_string("input_fname_pattern", "*.jpg", "Glob pattern of filename of input images [*]")
flags.DEFINE_string("checkpoint_dir", "checkpoint", "Directory name to save the checkpoints [checkpoint]")
flags.DEFINE_string("sample_dir", "samples", "Directory name to save the image samples [samples]")
flags.DEFINE_boolean("train", False, "True for training, False for testing [False]")
flags.DEFINE_boolean("crop", False, "True for training, False for testing [False]")
flags.DEFINE_boolean("visualize", False, "True for visualizing, False for nothing [False]")
flags.DEFINE_integer("generate_test_images", 100, "Number of images to generate during test. [100]")
FLAGS = flags.FLAGS

def main(_):
      pp.pprint(flags.FLAGS.__flags)
    
      if FLAGS.input_width is None:
        FLAGS.input_width = FLAGS.input_height
      if FLAGS.output_width is None:
        FLAGS.output_width = FLAGS.output_height
    
      if not os.path.exists(FLAGS.checkpoint_dir):
        os.makedirs(FLAGS.checkpoint_dir)
      if not os.path.exists(FLAGS.sample_dir):
        os.makedirs(FLAGS.sample_dir)
    
      #gpu_options = tf.GPUOptions(per_process_gpu_memory_fraction=0.333)
      run_config = tf.ConfigProto()
      run_config.gpu_options.allow_growth=True

  
      with tf.Session(config=run_config) as sess:  
	    dcgan = DCGAN(
		  sess,
		  input_width=FLAGS.input_width,
		  input_height=FLAGS.input_height,
		  output_width=FLAGS.output_width,
		  output_height=FLAGS.output_height,
		  batch_size=FLAGS.batch_size,
		  sample_num=FLAGS.batch_size,
		  y_dim=136,
		  z_dim=FLAGS.generate_test_images,
		  dataset_name=FLAGS.dataset,
		  input_fname_pattern=FLAGS.input_fname_pattern,
		  crop=FLAGS.crop,
		  checkpoint_dir=FLAGS.checkpoint_dir,
		  sample_dir=FLAGS.sample_dir)
    
        show_all_variables()
    
        if FLAGS.train:
          dcgan.train(FLAGS)
        else:
          if not dcgan.load(FLAGS.checkpoint_dir)[0]:
            raise Exception("[!] Train a model first, then run test mode")

        # Below is codes for visualization
        OPTION = 5
        visualize(sess, dcgan, FLAGS, OPTION)

if __name__ == '__main__':
  tf.app.run()


```

## Source file: CNN_RNN/CNN.py

{
  "path": "CNN_RNN/CNN.py",
  "blob_sha": "46d29b4457b29060898fdfc2abb748c3213a701a",
  "sha256": "b54f0d659c89a78bc2c85dc62f92b79c891cee5b3a60522a675ed8eae7c48404",
  "lines": 42,
  "bytes": 2422,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/xiaoyiT/Course-Project---Speech-Driven-Facial-Animation/blob/6fbfb4debfb54fb08a5750ec61b745f523ac208a/CNN_RNN/CNN.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def cnn_model_fn(features, labels, mode):"
    }
  ]
}

```text

import tensorflow as tf
import numpy as np

def cnn_model_fn(features, labels, mode):

    input_layer = tf.reshape(features["x"],[-1,128,23,1])

    conv1 = tf.layers.conv2d(inputs=input_layer,filters=32,kernel_size=[3, 1],padding="same",activation=tf.nn.relu,strides=[2,1])
    pool1 = tf.layers.max_pooling2d(inputs=conv1, pool_size=[2, 1], strides=[2,1])
    conv2 = tf.layers.conv2d(inputs=pool1,filters=32,kernel_size=[3, 1],padding="same",activation=tf.nn.relu,strides=[2,1])
    pool2 = tf.layers.max_pooling2d(inputs=conv2, pool_size=[2, 1], strides=[2,1])
    conv3 = tf.layers.conv2d(inputs=pool2,filters=64,kernel_size=[2, 1],padding="same",activation=tf.nn.relu,strides=[2,1])
    conv4 = tf.layers.conv2d(inputs=conv3,filters=64,kernel_size=[3, 1],padding="same",activation=tf.nn.relu,strides=[2,1])
    conv5 = tf.layers.conv2d(inputs=conv4,filters=128,kernel_size=[3, 1],padding="same",activation=tf.nn.relu,strides=[2,1])
    

    pool3 = tf.layers.max_pooling2d(inputs=conv5, pool_size=[1, 2], strides=[1,2])
    conv6 = tf.layers.conv2d(inputs=pool3,filters=128,kernel_size=[1, 3],padding="same",activation=tf.nn.relu,strides=[1,2])
    conv7 = tf.layers.conv2d(inputs=conv6,filters=136,kernel_size=[1, 3],padding="same",activation=tf.nn.relu,strides=[1,2])
    conv8 = tf.layers.conv2d(inputs=conv7,filters=136,kernel_size=[1, 4],padding="same",activation=tf.nn.relu,strides=[1,4])


    conv8_flat = tf.reshape(conv8, [-1,136])
    dense1 = tf.layers.dense(inputs=conv8_flat, units=136 , activation=tf.nn.tanh)
    rnn = tf.nn.rnn_cell.GRUCell(dense1)
    dense2 = tf.layers.dense(inputs=dense1, units=136 , activation=tf.nn.tanh)
    output = tf.layers.dense(inputs=dense2, units=136 , activation=tf.nn.sigmoid)

    predictions = {"classes": output}

    if mode == tf.estimator.ModeKeys.PREDICT:
        return tf.estimator.EstimatorSpec(mode=mode, predictions=predictions)

    loss = tf.losses.mean_squared_error(labels,output)

    if mode == tf.estimator.ModeKeys.TRAIN:
        optimizer = tf.train.AdamOptimizer()
        train_op = optimizer.minimize(loss=loss,global_step=tf.train.get_global_step())
        return tf.estimator.EstimatorSpec(mode=mode, loss=loss, train_op=train_op)

    eval_metric_ops = {"error":tf.metrics.mean_squared_error(labels=labels, predictions=predictions["classes"])}
    return tf.estimator.EstimatorSpec(mode=mode, loss=loss, eval_metric_ops=eval_metric_ops)

```

## Source file: DCGAN/mygan.py

{
  "path": "DCGAN/mygan.py",
  "blob_sha": "264ee1499d962d1a14e6a97128f80734c2350911",
  "sha256": "2ca67b60743a2a265a31315848f35574178f3c364176938aecaa040665badb8d",
  "lines": 345,
  "bytes": 13251,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/xiaoyiT/Course-Project---Speech-Driven-Facial-Animation/blob/6fbfb4debfb54fb08a5750ec61b745f523ac208a/DCGAN/mygan.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def conv_out_size_same(size, stride):"
    },
    {
      "line": 17,
      "text": "def __init__(self, sess, input_height=108, input_width=108, crop=True,"
    },
    {
      "line": 82,
      "text": "def build_model(self):"
    },
    {
      "line": 108,
      "text": "def sigmoid_cross_entropy_with_logits(x, y):"
    },
    {
      "line": 136,
      "text": "def train(self, config):"
    },
    {
      "line": 226,
      "text": "def discriminator(self, image,y , reuse=False):"
    },
    {
      "line": 247,
      "text": "def generator(self, z, y):"
    },
    {
      "line": 275,
      "text": "def sampler(self, y):"
    },
    {
      "line": 308,
      "text": "def load_landmarks(self):"
    },
    {
      "line": 315,
      "text": "def model_dir(self):"
    },
    {
      "line": 320,
      "text": "def save(self, checkpoint_dir, step):"
    },
    {
      "line": 331,
      "text": "def load(self, checkpoint_dir):"
    }
  ]
}

```text

from __future__ import division
import os
import time
import math
from glob import glob
import tensorflow as tf
import numpy as np
from six.moves import xrange

from ops import *
from utils import *

def conv_out_size_same(size, stride):
  return int(math.ceil(float(size) / float(stride)))

class DCGAN(object):
  def __init__(self, sess, input_height=108, input_width=108, crop=True,
         batch_size=64, sample_num = 64, output_height=64, output_width=64,
         y_dim=None, z_dim=100, gf_dim=64, df_dim=64,
         gfc_dim=1024, dfc_dim=1024, c_dim=3, dataset_name='default',
         input_fname_pattern='*.jpg', checkpoint_dir=None, sample_dir=None):
    """

    Args:
      sess: TensorFlow session
      batch_size: The size of batch. Should be specified before training.
      y_dim: (optional) Dimension of dim for y. [None]
      z_dim: (optional) Dimension of dim for Z. [100]
      gf_dim: (optional) Dimension of gen filters in first conv layer. [64]
      df_dim: (optional) Dimension of discrim filters in first conv layer. [64]
      gfc_dim: (optional) Dimension of gen units for for fully connected layer. [1024]
      dfc_dim: (optional) Dimension of discrim units for fully connected layer. [1024]
      c_dim: (optional) Dimension of image color. For grayscale input, set to 1. [3]
    """
    self.sess = sess
    self.crop = crop

    self.batch_size = batch_size
    self.sample_num = sample_num

    self.input_height = input_height
    self.input_width = input_width
    self.output_height = output_height
    self.output_width = output_width

    self.z_dim = z_dim
    self.y_dim = y_dim
    
    self.init_pic = tf.placeholder(tf.float32, [self.batch_size,self.input_height, self.input_width, 3], name='ip')

    self.gf_dim = gf_dim
    self.df_dim = df_dim

    # batch normalization : deals with poor initialization helps gradient flow
    self.d_bn1 = batch_norm(name='d_bn1')
    self.d_bn2 = batch_norm(name='d_bn2')
    self.d_bn3 = batch_norm(name='d_bn3')
    
    self.g_bn0 = batch_norm(name='g_bn0')
    self.g_bn1 = batch_norm(name='g_bn1')
    self.g_bn2 = batch_norm(name='g_bn2')
    self.g_bn3 = batch_norm(name='g_bn3')


    self.dataset_name = dataset_name
    self.input_fname_pattern = input_fname_pattern
    self.checkpoint_dir = checkpoint_dir
    
    self.data = glob(os.path.join("./data",self.dataset_name, self.input_fname_pattern))
    imreadImg = imread(self.data[0])
    if len(imreadImg.shape) >= 3:
        self.c_dim = imread(self.data[0]).shape[-1]
    else:
        self.c_dim = 1

    self.input_y = self.load_landmarks()
    
    self.grayscale = (self.c_dim == 1)

    self.build_model()

  def build_model(self):
    #self.y = None
    self.y = tf.placeholder(tf.float32, [self.batch_size, self.y_dim], name='y')
    if self.crop:
      image_dims = [self.output_height, self.output_width, self.c_dim]
    else:
      image_dims = [self.input_height, self.input_width, self.c_dim]

    self.inputs = tf.placeholder(
      tf.float32, [self.batch_size] + image_dims, name='real_images')

    inputs = self.inputs

    self.z = tf.placeholder(
      tf.float32, [None, self.z_dim], name='z')
    self.z_sum = histogram_summary("z", self.z)

    self.G                  = self.generator(self.z,self.y)
    self.D, self.D_logits   = self.discriminator(inputs,self.y, reuse=False)
    self.sampler            = self.sampler(self.y)
    self.D_, self.D_logits_ = self.discriminator(self.G,self.y, reuse=True)
    
    self.d_sum = histogram_summary("d", self.D)
    self.d__sum = histogram_summary("d_", self.D_)
    self.G_sum = image_summary("G", self.G)

    def sigmoid_cross_entropy_with_logits(x, y):
      try:
        return tf.nn.sigmoid_cross_entropy_with_logits(logits=x, labels=y)
      except:
        return tf.nn.sigmoid_cross_entropy_with_logits(logits=x, targets=y)

    self.d_loss_real = tf.reduce_mean(
      sigmoid_cross_entropy_with_logits(self.D_logits, tf.ones_like(self.D)))
    self.d_loss_fake = tf.reduce_mean(
      sigmoid_cross_entropy_with_logits(self.D_logits_, tf.zeros_like(self.D_)))
    self.g_loss = tf.reduce_mean(
      sigmoid_cross_entropy_with_logits(self.D_logits_, tf.ones_like(self.D_)))

    self.d_loss_real_sum = scalar_summary("d_loss_real", self.d_loss_real)
    self.d_loss_fake_sum = scalar_summary("d_loss_fake", self.d_loss_fake)
                          
    self.d_loss = self.d_loss_real + self.d_loss_fake

    self.g_loss_sum = scalar_summary("g_loss", self.g_loss)
    self.d_loss_sum = scalar_summary("d_loss", self.d_loss)

    t_vars = tf.trainable_variables()

    self.d_vars = [var for var in t_vars if 'd_' in var.name]
    self.g_vars = [var for var in t_vars if 'g_' in var.name]

    self.saver = tf.train.Saver()

  def train(self, config):
    d_optim = tf.train.AdamOptimizer(config.learning_rate, beta1=config.beta1) \
              .minimize(self.d_loss, var_list=self.d_vars)
    g_optim = tf.train.AdamOptimizer(config.learning_rate, beta1=config.beta1) \
              .minimize(self.g_loss, var_list=self.g_vars)
    try:
      tf.global_variables_initializer().run()
    except:
      tf.initialize_all_variables().run()

    self.g_sum = merge_summary([self.z_sum, self.d__sum,
      self.G_sum, self.d_loss_fake_sum, self.g_loss_sum])
    self.d_sum = merge_summary(
        [self.z_sum, self.d_sum, self.d_loss_real_sum, self.d_loss_sum])
    self.writer = SummaryWriter("./logs", self.sess.graph)

    sample_files = self.data[0:self.sample_num]
    sample = [
    get_image(sample_file,
              input_height=self.input_height,
              input_width=self.input_width,
              resize_height=self.output_height,
              resize_width=self.output_width,
              crop=self.crop,
              grayscale=self.grayscale) for sample_file in sample_files]
    if (self.grayscale):
        sample_inputs = np.array(sample).astype(np.float32)[:, :, :, None]
    else:
        sample_inputs = np.array(sample).astype(np.float32)
    
    counter = 1
    start_time = time.time()
    could_load, checkpoint_counter = self.load(self.checkpoint_dir)
    if could_load:
      counter = checkpoint_counter
      print(" [*] Load SUCCESS")
    else:
      print(" [!] Load failed...")

    for epoch in xrange(config.epoch):
      self.data = glob(os.path.join( "./data", config.dataset, self.input_fname_pattern))
      batch_idxs = min(int(len(self.data)/self.batch_size), config.train_size)
      
      shift_id = np.arange(len(self.data))
      np.random.shuffle(shift_id)

      for idx in xrange(0, batch_idxs):
        batch_files = []
        batch_y = []
        batch_z = np.random.uniform(-1, 1, [config.batch_size, self.z_dim]) \
              .astype(np.float32)
        for i in shift_id[idx*config.batch_size:(idx+1)*config.batch_size]:
          batch_files.append(self.data[i])
          batch_y.append(self.input_y[i])
        batch = [
          get_image(batch_file,
                      input_height=self.input_height,
                      input_width=self.input_width,
                      resize_height=self.output_height,
                      resize_width=self.output_width,
                      crop=self.crop,
                      grayscale=self.grayscale) for batch_file in batch_files]
        if self.grayscale:
            batch_images = np.array(batch).astype(np.float32)[:, :, :, None]
        else:
            batch_images = np.array(batch).astype(np.float32)
			
        # Update D network
        _, summary_str = self.sess.run([d_optim, self.d_sum],
          feed_dict={ self.inputs: batch_images, self.z: batch_z, self.y: batch_y })
        self.writer.add_summary(summary_str, counter)      
        # Update G network
        _, summary_str = self.sess.run([g_optim, self.g_sum],
          feed_dict={ self.z: batch_z, self.y: batch_y })
        self.writer.add_summary(summary_str, counter)      
        # Run g_optim twice to make sure that d_loss does not go to zero (different from paper)
        _, summary_str = self.sess.run([g_optim, self.g_sum],
          feed_dict={ self.z: batch_z, self.y: batch_y })
        self.writer.add_summary(summary_str, counter)        
        errD_fake = self.d_loss_fake.eval({ self.z: batch_z , self.y: batch_y})
        errD_real = self.d_loss_real.eval({ self.inputs: batch_images, self.y: batch_y})
        errG = self.g_loss.eval({self.z: batch_z, self.y: batch_y})

        counter += 1
        print("Epoch: [%2d] [%4d/%4d] time: %4.4f, d_loss: %.8f, g_loss: %.8f" \
          % (epoch, idx, batch_idxs,
            time.time() - start_time, errD_fake+errD_real, errG))
        if np.mod(counter, 50) == 2:
          self.save(config.checkpoint_dir, counter)

  def discriminator(self, image,y , reuse=False):
    with tf.variable_scope("discriminator") as scope:
      if reuse:
        scope.reuse_variables()
      
      yb = tf.reshape(y, [self.batch_size, 1, 1, self.y_dim])
      
      h0 = lrelu(conv2d(image, 64, name='d_h0_conv'))
      h1 = tf.nn.max_pool(h0,[1,2,2,1],[1,2,2,1],padding='SAME',name='d_h1_pool')
      h1c = conv_cond_concat(h1, yb)
      h2 = lrelu(self.d_bn1(conv2d(h1c, 128, name='d_h2_conv')))
      h3 = tf.nn.max_pool(h2,[1,2,2,1],[1,2,2,1],padding='SAME',name='d_h3_pool')
      h4 = lrelu(self.d_bn2(conv2d(h3, 256, name='d_h4_conv')))
      h5 = lrelu(self.d_bn3(conv2d(h4, 1024, name='d_h5_conv')))
      h5 = tf.reshape(h5, [self.batch_size, -1])
      h6 = linear(h5, self.z_dim, 'd_h6_lin')
        
      return tf.nn.tanh(h6), h6      



  def generator(self, z, y):
    with tf.variable_scope("generator") as scope:
      yb = tf.reshape(y, [self.batch_size, 1, 1, self.y_dim])

      self.z_, self.h0_w, self.h0_b = linear(z, 1024*4*4, 'g_h0_lin',with_w=True)     
      
      self.h0 = tf.reshape(self.z_,[-1,4,4,1024])
      h0 = tf.nn.relu(self.g_bn0(self.h0))
       
      h1 = deconv2d(h0,[self.batch_size,8,8,256],name='g_h1_dconv')
      h1 = tf.nn.relu(self.g_bn1(h1))
        
      h2 = deconv2d(h1,[self.batch_size,16,16,128],name='g_h2_dconv')
      h2 = tf.nn.relu(self.g_bn2(h2))
        
      h3 = unpool(h2,name='g_h3_unpool')
        
      h4 = deconv2d(h3,[self.batch_size,64,64,64],name='g_h4_dconv')
      h4 = tf.nn.relu(self.g_bn3(h4))
      print(h4.get_shape())
      h4c = conv_cond_concat(h4, yb)
      print(h4c.get_shape())  
      h5 = unpool(h4c,name='g_h5_unpool')
      print(h5.get_shape())  
      h6 = deconv2d(h5,[self.batch_size,256,256,3],name='g_h6_dconv')
      print(h6.get_shape())  
      return tf.nn.tanh(h6)

  def sampler(self, y):
    z = tf.placeholder(tf.float32, [self.batch_size, self.z_dim], name='z')
    _, z = self.discriminator(self.init_pic, y, True)
        
    with tf.variable_scope("generator") as scope:
      scope.reuse_variables()
      yb = tf.reshape(y, [self.batch_size, 1, 1, self.y_dim])
      self.z_, self.h0_w, self.h0_b = linear(z, 1024*4*4, 'g_h0_lin',with_w=True)
        
      self.h0 = tf.reshape(self.z_,[-1,4,4,1024])
      h0 = tf.nn.relu(self.g_bn0(self.h0))
        
      h1 = deconv2d(h0,[self.batch_size,8,8,256],name='g_h1_dconv')
      h1 = tf.nn.relu(self.g_bn1(h1))
       
      h2 = deconv2d(h1,[self.batch_size,16,16,128],name='g_h2_dconv')
      h2 = tf.nn.relu(self.g_bn2(h2))
        
      h3 = unpool(h2,name='g_h3_unpool')
        
      h4 = deconv2d(h3,[self.batch_size,64,64,64],name='g_h4_dconv')
      h4 = tf.nn.relu(self.g_bn3(h4))
       
      h4c = conv_cond_concat(h4, yb)
        
      h5 = unpool(h4c,name='g_h5_unpool')
        
      h6 = deconv2d(h5,[self.batch_size,256,256,3],name='g_h6_dconv')
        
      return tf.nn.tanh(h6)



  def load_landmarks(self):         
    z = np.load('./data/landmarks.npy')    
    return z  



  @property
  def model_dir(self):
    return "{}_{}_{}_{}".format(
        self.dataset_name, self.batch_size,
        self.output_height, self.output_width)
      
  def save(self, checkpoint_dir, step):
    model_name = "DCGAN.model"
    checkpoint_dir = os.path.join(checkpoint_dir, self.model_dir)

    if not os.path.exists(checkpoint_dir):
      os.makedirs(checkpoint_dir)

    self.saver.save(self.sess,
            os.path.join(checkpoint_dir, model_name),
            global_step=step)

  def load(self, checkpoint_dir):
    import re
    print(" [*] Reading checkpoints...")
    checkpoint_dir = os.path.join(checkpoint_dir, self.model_dir)

    ckpt = tf.train.get_checkpoint_state(checkpoint_dir)
    if ckpt and ckpt.model_checkpoint_path:
      ckpt_name = os.path.basename(ckpt.model_checkpoint_path)
      self.saver.restore(self.sess, os.path.join(checkpoint_dir, ckpt_name))
      counter = int(next(re.finditer("(\d+)(?!.*\d)",ckpt_name)).group(0))
      print(" [*] Success to read {}".format(ckpt_name))
      return True, counter
    else:
      print(" [*] Failed to find a checkpoint")
      return False, 0


```

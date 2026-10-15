# Facial rendering source scan: huybery/GDPnet

Repository: https://github.com/huybery/GDPnet

Pinned source revision: 8b84010bc043dc6e53b0d3ff20e6ee2407d5eeb9

Discovery topic: facial animation

Description: :astonished: GDPnet: "Geometry-guided Dense Perspective Network for Speech-Driven Facial Animation." (TVCG 2021)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["smpl_webuser/LICENSE.txt"]

Topic paths: {"flame": ["flame/readme.txt", "template/FLAME_sample.ply"], "lbs": ["smpl_webuser/lbs.py"]}

## Source file: smpl_webuser/lbs.py

{
  "path": "smpl_webuser/lbs.py",
  "blob_sha": "8fa18be4a543f4426321dc6bd4a4c67160839815",
  "sha256": "5c9f0e32736782e15eac2dce418b21de25d3f30e2282801a934e09bc5849460c",
  "lines": 80,
  "bytes": 2857,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 2,
    "blendshape": 1,
    "rig": 9
  },
  "url": "https://github.com/huybery/GDPnet/blob/8b84010bc043dc6e53b0d3ff20e6ee2407d5eeb9/smpl_webuser/lbs.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def global_rigid_transformation(pose, J, kintree_table, xp):"
    },
    {
      "line": 63,
      "text": "def verts_core(pose, v, J, weights, kintree_table, want_Jtr=False, xp=chumpy):"
    }
  ]
}

```text

'''
Copyright 2015 Matthew Loper, Naureen Mahmood and the Max Planck Gesellschaft.  All rights reserved.
This software is provided for research purposes only.
By using this software you agree to the terms of the SMPL Model license here http://smpl.is.tue.mpg.de/license

More information about SMPL is available here http://smpl.is.tue.mpg.
For comments or questions, please email us at: smpl@tuebingen.mpg.de


About this file:
================
This file defines linear blend skinning for the SMPL loader which 
defines the effect of bones and blendshapes on the vertices of the template mesh.

Modules included:
- global_rigid_transformation: 
  computes global rotation & translation of the model
- verts_core: [overloaded function inherited from verts.verts_core]
  computes the blending of joint-influences for each vertex based on type of skinning

'''

from smpl_webuser.posemapper import posemap
import chumpy
import numpy as np

def global_rigid_transformation(pose, J, kintree_table, xp):
    results = {}
    pose = pose.reshape((-1,3))
    id_to_col = {kintree_table[1,i] : i for i in range(kintree_table.shape[1])}
    parent = {i : id_to_col[kintree_table[0,i]] for i in range(1, kintree_table.shape[1])}

    if xp == chumpy:
        from smpl_webuser.posemapper import Rodrigues
        rodrigues = lambda x : Rodrigues(x)
    else:
        import cv2
        rodrigues = lambda x : cv2.Rodrigues(x)[0]

    with_zeros = lambda x : xp.vstack((x, xp.array([[0.0, 0.0, 0.0, 1.0]])))
    results[0] = with_zeros(xp.hstack((rodrigues(pose[0,:]), J[0,:].reshape((3,1)))))        
        
    for i in range(1, kintree_table.shape[1]):
        results[i] = results[parent[i]].dot(with_zeros(xp.hstack((
            rodrigues(pose[i,:]),
            ((J[i,:] - J[parent[i],:]).reshape((3,1)))
            ))))

    pack = lambda x : xp.hstack([np.zeros((4, 3)), x.reshape((4,1))])
    
    results = [results[i] for i in sorted(results.keys())]
    results_global = results

    if True:
        results2 = [results[i] - (pack(
            results[i].dot(xp.concatenate( ( (J[i,:]), 0 ) )))
            ) for i in range(len(results))]
        results = results2
    result = xp.dstack(results)
    return result, results_global


def verts_core(pose, v, J, weights, kintree_table, want_Jtr=False, xp=chumpy):
    A, A_global = global_rigid_transformation(pose, J, kintree_table, xp)
    T = A.dot(weights.T)

    rest_shape_h = xp.vstack((v.T, np.ones((1, v.shape[0]))))
        
    v =(T[:,0,:] * rest_shape_h[0, :].reshape((1, -1)) + 
        T[:,1,:] * rest_shape_h[1, :].reshape((1, -1)) + 
        T[:,2,:] * rest_shape_h[2, :].reshape((1, -1)) + 
        T[:,3,:] * rest_shape_h[3, :].reshape((1, -1))).T

    v = v[:,:3] 
    
    if not want_Jtr:
        return v
    Jtr = xp.vstack([g[:3,3] for g in A_global])
    return (v, Jtr)
    


```

## Source file: utils/ops.py

{
  "path": "utils/ops.py",
  "blob_sha": "d1fde0e37b5f40a16c57a6b8030412f177b24166",
  "sha256": "a43ae4bba64f928af264c39476bea20f740b2a2b1faa5d3ae12151b232e3be38",
  "lines": 168,
  "bytes": 6512,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "rig": 5
  },
  "url": "https://github.com/huybery/GDPnet/blob/8b84010bc043dc6e53b0d3ff20e6ee2407d5eeb9/utils/ops.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def fc_layer(inputs,"
    },
    {
      "line": 55,
      "text": "def custom_fc_layer(inputs,"
    },
    {
      "line": 96,
      "text": "def conv2d(inputs, n_filters,"
    },
    {
      "line": 153,
      "text": "def __init__(self, epsilon=1e-5, momentum=0.9, name=\"batch_norm\"):"
    },
    {
      "line": 159,
      "text": "def __call__(self, x, reuse=False, is_training=True):"
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

import tensorflow as tf

def fc_layer(inputs,
             num_units_in,
             num_units_out,
             init_weights=None,
             weightini=0.1,
             bias=0.,
             trainable=True,
             scope=None,
             regularisation_constant=0.0):
    with tf.variable_scope(scope):
        weights_shape = [num_units_in, num_units_out]

        if init_weights is not None:
            weights_initializer = tf.constant(init_weights, dtype=tf.float32)
            weights_shape = None
        elif weightini == 0.:
            weights_initializer = tf.constant_initializer(weightini)
        else:
            weights_initializer = tf.truncated_normal_initializer(stddev=weightini)

        weights = tf.get_variable('weights',
                                  shape=weights_shape,
                                  initializer=weights_initializer,
                                  trainable=trainable,
                                  regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))

        bias_shape = [num_units_out, ]
        bias_initializer = tf.constant_initializer(bias)
        biases = tf.get_variable('biases',
                                 shape=bias_shape,
                                 initializer=bias_initializer,
                                 trainable=trainable)
        outputs = tf.nn.xw_plus_b(inputs, weights, biases)
    return outputs

def custom_fc_layer(inputs,
             num_units_in,
             num_units_out,
             init_weights=None,
             weightini=0.1,
             bias=0.,
             trainable_weights=True,
             trainable_bias=True,
             regularisation_constant=0.0,
             output_weights=False,
              scope=None):
    with tf.variable_scope(scope):
        weights_shape = [num_units_in, num_units_out]

        if init_weights is not None:
            weights_initializer = tf.constant(init_weights, dtype=tf.float32)
            weights_shape = None
        elif weightini == 0.:
            weights_initializer = tf.constant_initializer(weightini)
        else:
            weights_initializer = tf.truncated_normal_initializer(stddev=weightini)

        weights = tf.get_variable('weights',
                                  shape=weights_shape,
                                  initializer=weights_initializer,
                                  trainable=trainable_weights,
                                  regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))

        bias_shape = [num_units_out, ]
        bias_initializer = tf.constant_initializer(bias)
        biases = tf.get_variable('biases',
                                 shape=bias_shape,
                                 initializer=bias_initializer,
                                 trainable=trainable_bias)
        outputs = tf.nn.xw_plus_b(inputs, weights, biases)
    if not output_weights:
        return outputs
    else:
        return outputs, weights


def conv2d(inputs, n_filters,
           k_h=5, k_w=5,
           stride_h=2, stride_w=2,
           stddev=0.02,
           activation=None,
           bias=True,
           padding='SAME',
           scope=None,
           regularisation_constant=0.0):
    """2D Convolution with options for kernel size, stride, and init deviation.
    Parameters
    ----------
    inputs : Tensor
        Input tensor to convolve.
    n_filters : int
        Number of filters to apply.
    k_h : int, optional
        Kernel height.
    k_w : int, optional
        Kernel width.
    stride_h : int, optional
        Stride in rows.
    stride_w : int, optional
        Stride in cols.
    stddev : float, optional
        Initialization's standard deviation.
    activation : arguments, optional
        Function which applies a nonlinearity
    padding : str, optional
        'SAME' or 'VALID'
    scope : str, optional
        Variable scope to use.
    Returns
    -------
    x : Tensor
        Convolved input.
    """
    with tf.variable_scope(scope):
        w = tf.get_variable('weights',
                            [k_h, k_w, inputs.get_shape()[-1], n_filters],
                            initializer=tf.truncated_normal_initializer(stddev=stddev),
                            regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))
        conv = tf.nn.conv2d(inputs,
                            w,
                            strides=[1, stride_h, stride_w, 1],
                            padding=padding)
        if bias:
            b = tf.get_variable('biases',
                                [n_filters],
                                initializer=tf.truncated_normal_initializer(stddev=stddev))
            conv = tf.nn.bias_add(conv, b)
        if activation:
            conv = activation(conv)
        return conv


class BatchNorm(object):
    def __init__(self, epsilon=1e-5, momentum=0.9, name="batch_norm"):
        with tf.variable_scope(name):
            self.epsilon = epsilon
            self.momentum = momentum
            self.name = name

    def __call__(self, x, reuse=False, is_training=True):
        return tf.contrib.layers.batch_norm(x,
                                            decay=self.momentum,
                                            updates_collections=None,
                                            epsilon=self.epsilon,
                                            center=True,
                                            scale=True,
                                            is_training=is_training,
                                            reuse=reuse)
                                            # scope=self.name)

```

## Source file: utils/ops2.py

{
  "path": "utils/ops2.py",
  "blob_sha": "f0644d853dee91a923cb7a6e61beb085a9fbba6e",
  "sha256": "b3c086af00f6d4bba9e810ed1040392c9eb0c60750fd6f6ffcf45a149533a1ca",
  "lines": 168,
  "bytes": 6511,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "rig": 5
  },
  "url": "https://github.com/huybery/GDPnet/blob/8b84010bc043dc6e53b0d3ff20e6ee2407d5eeb9/utils/ops2.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def fc_layer(inputs,"
    },
    {
      "line": 55,
      "text": "def custom_fc_layer(inputs,"
    },
    {
      "line": 96,
      "text": "def conv2d(inputs, n_filters,"
    },
    {
      "line": 153,
      "text": "def __init__(self, epsilon=1e-5, momentum=0.9, name=\"batch_norm\"):"
    },
    {
      "line": 159,
      "text": "def __call__(self, x, reuse=False, is_training=True):"
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

import tensorflow as tf

def fc_layer(inputs,
             num_units_in,
             num_units_out,
             init_weights=None,
             weightini=0.1,
             bias=0.,
             trainable=True,
             scope=None,
             regularisation_constant=0.0):
    with tf.variable_scope(scope):
        weights_shape = [num_units_in, num_units_out]

        if init_weights is not None:
            weights_initializer = tf.constant(init_weights, dtype=tf.float32)
            weights_shape = None
        elif weightini == 0.:
            weights_initializer = tf.constant_initializer(weightini)
        else:
            weights_initializer = tf.truncated_normal_initializer(stddev=weightini)

        weights = tf.get_variable('weights',
                                  shape=weights_shape,
                                  initializer=weights_initializer,
                                  trainable=trainable,
                                  regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))

        bias_shape = [num_units_out, ]
        bias_initializer = tf.constant_initializer(bias)
        biases = tf.get_variable('biases',
                                 shape=bias_shape,
                                 initializer=bias_initializer,
                                 trainable=trainable)
        outputs = tf.nn.xw_plus_b(inputs, weights, biases)
    return outputs

def custom_fc_layer(inputs,
             num_units_in,
             num_units_out,
             init_weights=None,
             weightini=0.1,
             bias=0.,
             trainable_weights=True,
             trainable_bias=True,
             regularisation_constant=0.0,
             output_weights=False,
              scope=None):
    with tf.variable_scope(scope):
        weights_shape = [num_units_in, num_units_out]

        if init_weights is not None:
            weights_initializer = tf.constant(init_weights, dtype=tf.float32)
            weights_shape = None
        elif weightini == 0.:
            weights_initializer = tf.constant_initializer(weightini)
        else:
            weights_initializer = tf.truncated_normal_initializer(stddev=weightini)

        weights = tf.get_variable('weights',
                                  shape=weights_shape,
                                  initializer=weights_initializer,
                                  trainable=trainable_weights,
                                  regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))

        bias_shape = [num_units_out, ]
        bias_initializer = tf.constant_initializer(bias)
        biases = tf.get_variable('biases',
                                 shape=bias_shape,
                                 initializer=bias_initializer,
                                 trainable=trainable_bias)
        outputs = tf.nn.xw_plus_b(inputs, weights, biases)
    if not output_weights:
        return outputs
    else:
        return outputs, weights


def conv2d(inputs, n_filters,
           k_h=5, k_w=5,
           stride_h=2, stride_w=2,
           stddev=0.02,
           activation=None,
           bias=True,
           padding='SAME',
           scope=None,
           regularisation_constant=0.0):
    """2D Convolution with options for kernel size, stride, and init deviation.
    Parameters
    ----------
    inputs : Tensor
        Input tensor to convolve.
    n_filters : int
        Number of filters to apply.
    k_h : int, optional
        Kernel height.
    k_w : int, optional
        Kernel width.
    stride_h : int, optional
        Stride in rows.
    stride_w : int, optional
        Stride in cols.
    stddev : float, optional
        Initialization's standard deviation.
    activation : arguments, optional
        Function which applies a nonlinearity
    padding : str, optional
        'SAME' or 'VALID'
    scope : str, optional
        Variable scope to use.
    Returns
    -------
    x : Tensor
        Convolved input.
    """
    with tf.variable_scope(scope):
        w = tf.get_variable('weights',
                            [k_h, k_w, inputs.get_shape()[-1], n_filters],
                            initializer=tf.truncated_normal_initializer(stddev=stddev),
                            regularizer=tf.contrib.layers.l2_regularizer(scale=regularisation_constant))
        conv = tf.nn.conv2d(inputs,
                            w,
                            strides=[1, stride_h, stride_w, 1],
                            padding=padding)
        if bias:
            b = tf.get_variable('biases',
                                [n_filters],
                                initializer=tf.truncated_normal_initializer(stddev=stddev))
            conv = tf.nn.bias_add(conv, b)
        if activation:
            conv = activation(conv)
        return conv


class BatchNorm(object):
    def __init__(self, epsilon=1e-5, momentum=0.9, name="batch_norm"):
        with tf.variable_scope(name):
            self.epsilon = epsilon
            self.momentum = momentum
            self.name = name

    def __call__(self, x, reuse=False, is_training=True):
        return tf.contrib.layers.batch_norm(x,
                                            decay=self.momentum,
                                            updates_collections=None,
                                            epsilon=self.epsilon,
                                            center=True,
                                            scale=True,
                                            is_training=is_training,
                                            reuse=reuse,
                                            scope=self.name)


```

## Source file: utils/losses.py

{
  "path": "utils/losses.py",
  "blob_sha": "f1b5b58a0f07bf07352500cca5a16e60cc7e5d68",
  "sha256": "7aafa61af2d2be70a5e5b9fdc6bfa6552c8dda2f17c43e1c9eb24f199d7234f6",
  "lines": 64,
  "bytes": 2853,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "rig": 5
  },
  "url": "https://github.com/huybery/GDPnet/blob/8b84010bc043dc6e53b0d3ff20e6ee2407d5eeb9/utils/losses.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def reconstruction_loss(predicted, real, want_absolute_loss=True, want_in_mm=False, weights=None):"
    },
    {
      "line": 37,
      "text": "def wing_reconstruction_loss(predicted, real):"
    },
    {
      "line": 40,
      "text": "def edge_reconstruction_loss(predicted, real, num_vertices, mesh_f, want_absolute_loss=True):"
    },
    {
      "line": 49,
      "text": "def orthogonality_loss(W, want_absolute_loss=True, no_normalize=False):"
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

from __future__ import absolute_import
from __future__ import division

import tensorflow as tf

def reconstruction_loss(predicted, real, want_absolute_loss=True, want_in_mm=False, weights=None):
    if weights is not None:
        assert predicted.shape[1] == real.shape[1] == weights.shape[0]
        tf_weights = tf.constant(weights, dtype=tf.float32)
        predicted = tf.einsum('abcd,bd->abcd', predicted, tf_weights)
        real = tf.einsum('abcd,bd->abcd', real, tf_weights)

    if want_in_mm:
        predicted, real = predicted * 1000, real * 1000
    if want_absolute_loss:
        return tf.reduce_mean(tf.reduce_sum(tf.abs(tf.subtract(predicted, real)), axis=2))
    else:
        return tf.reduce_mean(tf.reduce_sum(tf.squared_difference(predicted, real), axis=2))

def wing_reconstruction_loss(predicted, real):
    pass

def edge_reconstruction_loss(predicted, real, num_vertices, mesh_f, want_absolute_loss=True):
    from tfbody.mesh.utils import get_vertices_per_edge
    vpe = get_vertices_per_edge(num_vertices, mesh_f)
    edges_for = lambda x: tf.gather(x, vpe[:, 0], axis=1) - tf.gather(x, vpe[:, 1], axis=1)
    if want_absolute_loss:
        return tf.reduce_mean(tf.reduce_sum(tf.abs(tf.subtract(edges_for(predicted), edges_for(real))), axis=2))
    else:
        return tf.reduce_mean(tf.reduce_sum(tf.squared_difference(edges_for(predicted), edges_for(real)), axis=2))

def orthogonality_loss(W, want_absolute_loss=True, no_normalize=False):
    '''Computes the loss to a column-wise orthogonality. If no_normalize is true, the loss reflects
    the distance to orthogonalize the matrix columns <a,b> = 0 but the columnts are not orthonormal,
    i.e. <a,a> != 1'''
    wtw = tf.matmul(tf.transpose(W), W)

    if no_normalize:
        diag = tf.diag(tf.diag_part(wtw))
    else:
        num_rows = wtw.get_shape().as_list()[0]
        diag = tf.eye(num_rows)

    if want_absolute_loss:
        return tf.reduce_sum(tf.abs(tf.subtract(wtw, diag)))
    else:
        return tf.reduce_sum(tf.squared_difference((wtw, diag)))

```

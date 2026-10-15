# Facial rendering source scan: TimoBolkart/TF_FLAME

Repository: https://github.com/TimoBolkart/TF_FLAME

Pinned source revision: d4dab55213a93851f09689016be32d7fa10a6ce1

Discovery topic: FLAME 3D

Description: Tensorflow framework for the FLAME 3D head model. The code demonstrates how to sample 3D heads from the model, fit the model to 2D or 3D keypoints, and how to generate textured head meshes from Images.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["tf_smpl/LICENSE"]

Topic paths: {"flame": ["data/flame_static_embedding.pkl", "data/flame_static_embedding_68.pkl", "sample_FLAME.py"], "lbs": ["tf_smpl/batch_lbs.py"]}

## Source file: sample_FLAME.py

{
  "path": "sample_FLAME.py",
  "blob_sha": "0df2ba724b88881d6cf354a652ce0580ebfb1629",
  "sha256": "35a363cf68c61ad3d60565cab767184c3af17ffaec4d5a2b44ec0f9a565c77e4",
  "lines": 115,
  "bytes": 5861,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "jaw": 1,
    "mouth": 1,
    "gum": 6,
    "flame": 14,
    "rig": 5,
    "face": 1
  },
  "url": "https://github.com/TimoBolkart/TF_FLAME/blob/d4dab55213a93851f09689016be32d7fa10a6ce1/sample_FLAME.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def str2bool(val):"
    },
    {
      "line": 41,
      "text": "def sample_FLAME(model_fname, num_samples, out_path, visualize, sample_VOCA_template=False):"
    },
    {
      "line": 94,
      "text": "def main(args):"
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

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''


import os
import six
import argparse
import numpy as np
import tensorflow as tf
from psbody.mesh import Mesh
from psbody.mesh.meshviewer import MeshViewer
from utils.landmarks import load_binary_pickle, load_embedding, tf_get_model_lmks, create_lmk_spheres

from tf_smpl.batch_smpl import SMPL
from tensorflow.contrib.opt import ScipyOptimizerInterface as scipy_pt

def str2bool(val):
    if isinstance(val, bool):
        return val
    elif isinstance(val, str):
        if val.lower() in ['true', 't', 'yes', 'y']:
            return True
        elif val.lower() in ['false', 'f', 'no', 'n']:
            return False
    return False

def sample_FLAME(model_fname, num_samples, out_path, visualize, sample_VOCA_template=False):
    '''
    Sample the FLAME model to demonstrate how to vary the model parameters.FLAME has parameters to
        - model identity-dependent shape variations (paramters: shape),
        - articulation of neck (paramters: pose[0:3]), jaw (paramters: pose[3:6]), and eyeballs (paramters: pose[6:12])
        - model facial expressions, i.e. all expression motion that does not involve opening the mouth (paramters: exp)
        - global translation (paramters: trans)
        - global rotation (paramters: rot)
    :param model_fname              saved FLAME model
    :param num_samples              number of samples
    :param out_path                 output path to save the generated templates (no templates are saved if path is empty)
    :param visualize                visualize samples
    :param sample_VOCA_template     sample template in 'zero pose' that can be used e.g. for speech-driven animation in VOCA
    '''

    tf_trans = tf.Variable(np.zeros((1,3)), name="trans", dtype=tf.float64, trainable=True)
    tf_rot = tf.Variable(np.zeros((1,3)), name="pose", dtype=tf.float64, trainable=True)
    tf_pose = tf.Variable(np.zeros((1,12)), name="pose", dtype=tf.float64, trainable=True)
    tf_shape = tf.Variable(np.zeros((1,300)), name="shape", dtype=tf.float64, trainable=True)
    tf_exp = tf.Variable(np.zeros((1,100)), name="expression", dtype=tf.float64, trainable=True)
    smpl = SMPL(model_fname)
    tf_model = tf.squeeze(smpl(tf_trans,
                               tf.concat((tf_shape, tf_exp), axis=-1),
                               tf.concat((tf_rot, tf_pose), axis=-1)))

    with tf.Session() as session:
        session.run(tf.global_variables_initializer())

        if visualize:
            mv = MeshViewer()
        for i in range(num_samples):
            if sample_VOCA_template:
                assign_shape = tf.assign(tf_shape, np.hstack((np.random.randn(100), np.zeros(200)))[np.newaxis,:])
                session.run([assign_shape])
                out_fname = os.path.join(out_path, 'VOCA_template_%02d.ply' % (i+1))
            else:
                # assign_trans = tf.assign(tf_trans, np.random.randn(3)[np.newaxis,:])
                assign_rot = tf.assign(tf_rot, np.random.randn(3)[np.newaxis,:] * 0.03)
                assign_pose = tf.assign(tf_pose, np.random.randn(12)[np.newaxis,:] * 0.02)
                assign_shape = tf.assign(tf_shape, np.hstack((np.random.randn(100), np.zeros(200)))[np.newaxis,:])
                assign_exp = tf.assign(tf_exp, np.hstack((0.5*np.random.randn(50), np.zeros(50)))[np.newaxis,:])
                session.run([assign_rot, assign_pose, assign_shape, assign_exp])
                out_fname = os.path.join(out_path, 'FLAME_sample_%02d.ply' % (i+1))

            sample_mesh = Mesh(session.run(tf_model), smpl.f)
            if visualize:
                mv.set_dynamic_meshes([sample_mesh], blocking=True)
                key = six.moves.input('Press (s) to save sample, any other key to continue ')
                if key == 's':
                    sample_mesh.write_ply(out_fname)
            else:
                sample_mesh.write_ply(out_fname)

def main(args):
    if not os.path.exists(args.model_fname):
        print('FLAME model not found - %s' % args.model_fname)
        return
    if not os.path.exists(args.out_path):
        os.makedirs(args.out_path)
    if args.option == 'sample_FLAME':
        sample_FLAME(args.model_fname, int(args.num_samples), args.out_path, str2bool(args.visualize), sample_VOCA_template=False)
    else:
        sample_FLAME(args.model_fname, int(args.num_samples), args.out_path, str2bool(args.visualize), sample_VOCA_template=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Sample FLAME shape space')
    parser.add_argument('--option', default='sample_FLAME', help='sample random FLAME meshes or VOCA templates')
    parser.add_argument('--model_fname', default='./models/generic_model.pkl', help='Path of the FLAME model')
    parser.add_argument('--num_samples', default='5', help='Number of samples')
    parser.add_argument('--out_path', default='./FLAME_samples', help='Output path')
    parser.add_argument('--visualize', default='True', help='Visualize fitting progress and final fitting result')
    args = parser.parse_args()
    main(args)



```

## Source file: tf_smpl/batch_lbs.py

{
  "path": "tf_smpl/batch_lbs.py",
  "blob_sha": "5e48df08d13f58edb53d1acbce92bf29778ee890",
  "sha256": "54525cd929f2d4a7b997c6bc43ee8b9e45a89b2a865dc11454f26127fa6e1dc4",
  "lines": 145,
  "bytes": 5000,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 3,
    "skin": 1,
    "lbs": 1,
    "rig": 7,
    "lip": 2
  },
  "url": "https://github.com/TimoBolkart/TF_FLAME/blob/d4dab55213a93851f09689016be32d7fa10a6ce1/tf_smpl/batch_lbs.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def batch_skew(vec, batch_size=None, name=''):"
    },
    {
      "line": 42,
      "text": "def batch_rodrigues(theta, name='', dtype=tf.float64):"
    },
    {
      "line": 61,
      "text": "def batch_lrotmin(theta, name=''):"
    },
    {
      "line": 85,
      "text": "def batch_global_rigid_transformation(Rs, Js, parent, rotate_base=False, name=''):"
    },
    {
      "line": 116,
      "text": "def make_A(R, t, name=''):"
    }
  ]
}

```text

""" Util functions for SMPL
@@batch_skew
@@batch_rodrigues
@@batch_lrotmin
@@batch_global_rigid_transformation
"""

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function

import tensorflow as tf


def batch_skew(vec, batch_size=None, name=''):
    """
    vec is N x 3, batch_size is int

    returns N x 3 x 3. Skew_sym version of each matrix.
    """
    with tf.name_scope(name, "batch_skew", [vec]):
        if batch_size is None:
            batch_size = vec.shape.as_list()[0]
        col_inds = tf.constant([1, 2, 3, 5, 6, 7])
        indices = tf.reshape(
            tf.reshape(tf.range(0, batch_size) * 9, [-1, 1]) + col_inds,
            [-1, 1])
        updates = tf.reshape(
            tf.stack(
                [
                    -vec[:, 2], vec[:, 1], vec[:, 2], -vec[:, 0], -vec[:, 1],
                    vec[:, 0]
                ],
                axis=1), [-1])
        out_shape = [batch_size * 9]
        res = tf.scatter_nd(indices, updates, out_shape)
        res = tf.reshape(res, [batch_size, 3, 3])

        return res


def batch_rodrigues(theta, name='', dtype=tf.float64):
    """
    Theta is N x 3
    """
    with tf.name_scope(name, "batch_rodrigues", [theta]):
        batch_size = theta.shape.as_list()[0]
        angle = tf.expand_dims(tf.clip_by_value(tf.norm(theta, axis=1), 1e-16, 1e16), -1)
        r = tf.expand_dims(tf.math.divide_no_nan(theta, angle), -1)

        angle = tf.expand_dims(angle, -1)
        cos = tf.cos(angle)
        sin = tf.sin(angle)

        outer = tf.matmul(r, r, transpose_b=True, name="outer")
        eyes = tf.tile(tf.expand_dims(tf.eye(3, dtype=dtype), 0), [batch_size, 1, 1])
        R = cos * eyes + (1 - cos) * outer + sin * batch_skew(r, batch_size=batch_size)
        return R


def batch_lrotmin(theta, name=''):
    """ NOTE: not used bc I want to reuse R and this is simple.
    Output of this is used to compute joint-to-pose blend shape mapping.
    Equation 9 in SMPL paper.


    Args:
      pose: `Tensor`, N x 72 vector holding the axis-angle rep of K joints.
            This includes the global rotation so K=24

    Returns
      diff_vec : `Tensor`: N x 9*(num_joints-1) rotation matrix of (K-1) joints with identity subtracted.,
    """
    with tf.name_scope(name, "batch_lrotmin", [theta]):
        with tf.name_scope("ignore_global"):
            theta = theta[:, 3:]

        # N*23 x 3 x 3
        Rs = batch_rodrigues(tf.reshape(theta, [-1, 3]))
        lrotmin = tf.reshape(Rs - tf.eye(3), [-1, 207])

        return lrotmin


def batch_global_rigid_transformation(Rs, Js, parent, rotate_base=False, name=''):
    """
    Computes absolute joint locations given pose.

    rotate_base: if True, rotates the global rotation by 90 deg in x axis.
    if False, this is the original SMPL coordinate.

    Args:
      Rs: N x num_joints x 3 x 3 rotation vector of K joints
      Js: N x num_joints x 3, joint locations before posing
      parent: num_joints holding the parent id for each index

    Returns
      new_J : `Tensor`: N x num_joints x 3 location of absolute joints
      A     : `Tensor`: N x num_joints 4 x 4 relative joint transformations for LBS.
    """

    num_joints = parent.shape[0]
    with tf.name_scope(name, "batch_forward_kinematics", [Rs, Js]):
        N = Rs.shape[0].value
        if rotate_base:
            print('Flipping the SMPL coordinate frame!!!!')
            rot_x = tf.constant([[1, 0, 0], [0, -1, 0], [0, 0, -1]], dtype=Rs.dtype)
            rot_x = tf.reshape(tf.tile(rot_x, [N, 1]), [N, 3, 3])
            root_rotation = tf.matmul(Rs[:, 0, :, :], rot_x)
        else:
            root_rotation = Rs[:, 0, :, :]

        # Now Js is N x num_joints x 3 x 1
        Js = tf.expand_dims(Js, -1)

        def make_A(R, t, name=''):
            # Rs is N x 3 x 3, ts is N x 3 x 1
            with tf.name_scope(name, "Make_A", [R, t]):
                R_homo = tf.pad(R, [[0, 0], [0, 1], [0, 0]])
                t_homo = tf.concat([t, tf.ones([N, 1, 1], dtype=t.dtype)], 1)
                return tf.concat([R_homo, t_homo], 2)

        A0 = make_A(root_rotation, Js[:, 0])
        results = [A0]
        for i in range(1, parent.shape[0]):
            j_here = Js[:, i] - Js[:, parent[i]]
            A_here = make_A(Rs[:, i], j_here)
            res_here = tf.matmul(results[parent[i]], A_here, name="propA%d" % i)
            results.append(res_here)

        results = tf.stack(results, axis=1)

        new_J = results[:, :, :3, 3]

        # --- Compute relative A: Skinning is based on
        # how much the bone moved (not the final location of the bone)
        # but (final_bone - init_bone)
        # ---
        Js_w0 = tf.concat([Js, tf.zeros([N, num_joints, 1, 1], dtype=Js.dtype)], 2)
        init_bone = tf.matmul(results, Js_w0)
        # Append empty 4 x 3:
        init_bone = tf.pad(init_bone, [[0, 0], [0, 0], [0, 0], [3, 0]])
        A = results - init_bone

        return new_J, A


```

## Source file: fit_3D_mesh.py

{
  "path": "fit_3D_mesh.py",
  "blob_sha": "e283e9838954bc9bc7da15c5b73f5a03ccb02100",
  "sha256": "5fad96cc59e8e41e93ca23671d8f9ad4fb471903226fcdbe4f817fa2d84c9234",
  "lines": 127,
  "bytes": 5526,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "jaw": 4,
    "mouth": 1,
    "flame": 5,
    "rig": 6,
    "face": 1
  },
  "url": "https://github.com/TimoBolkart/TF_FLAME/blob/d4dab55213a93851f09689016be32d7fa10a6ce1/fit_3D_mesh.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def fit_3D_mesh(target_3d_mesh_fname, model_fname, weights, show_fitting=True):"
    },
    {
      "line": 90,
      "text": "def run_corresponding_mesh_fitting():"
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

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''


import os
import six
import numpy as np
import tensorflow as tf
from psbody.mesh import Mesh
from psbody.mesh.meshviewer import MeshViewer

from tf_smpl.batch_smpl import SMPL
from tensorflow.contrib.opt import ScipyOptimizerInterface as scipy_pt

def fit_3D_mesh(target_3d_mesh_fname, model_fname, weights, show_fitting=True):
    '''
    Fit FLAME to 3D mesh in correspondence to the FLAME mesh (i.e. same number of vertices, same mesh topology)
    :param target_3d_mesh_fname:    target 3D mesh filename
    :param model_fname:             saved FLAME model
    :param weights:             weights of the individual objective functions
    :return: a mesh with the fitting results
    '''

    target_mesh = Mesh(filename=target_3d_mesh_fname)

    tf_trans = tf.Variable(np.zeros((1,3)), name="trans", dtype=tf.float64, trainable=True)
    tf_rot = tf.Variable(np.zeros((1,3)), name="pose", dtype=tf.float64, trainable=True)
    tf_pose = tf.Variable(np.zeros((1,12)), name="pose", dtype=tf.float64, trainable=True)
    tf_shape = tf.Variable(np.zeros((1,300)), name="shape", dtype=tf.float64, trainable=True)
    tf_exp = tf.Variable(np.zeros((1,100)), name="expression", dtype=tf.float64, trainable=True)
    smpl = SMPL(model_fname)
    tf_model = tf.squeeze(smpl(tf_trans,
                               tf.concat((tf_shape, tf_exp), axis=-1),
                               tf.concat((tf_rot, tf_pose), axis=-1)))

    with tf.Session() as session:
        session.run(tf.global_variables_initializer())

        mesh_dist = tf.reduce_sum(tf.square(tf.subtract(tf_model, target_mesh.v)))
        neck_pose_reg = tf.reduce_sum(tf.square(tf_pose[:,:3]))
        jaw_pose_reg = tf.reduce_sum(tf.square(tf_pose[:,3:6]))
        eyeballs_pose_reg = tf.reduce_sum(tf.square(tf_pose[:,6:]))
        shape_reg = tf.reduce_sum(tf.square(tf_shape))
        exp_reg = tf.reduce_sum(tf.square(tf_exp))

        # Optimize global transformation first
        vars = [tf_trans, tf_rot]
        loss = mesh_dist
        optimizer = scipy_pt(loss=loss, var_list=vars, method='BFGS', options={'disp': 1})
        print('Optimize rigid transformation')
        optimizer.minimize(session)

        # Optimize for the model parameters
        vars = [tf_trans, tf_rot, tf_pose, tf_shape, tf_exp]
        loss = weights['data'] * mesh_dist + weights['shape'] * shape_reg + weights['expr'] * exp_reg + \
               weights['neck_pose'] * neck_pose_reg + weights['jaw_pose'] * jaw_pose_reg + weights['eyeballs_pose'] * eyeballs_pose_reg

        optimizer = scipy_pt(loss=loss, var_list=vars, method='BFGS', options={'disp': 1})
        print('Optimize model parameters')
        optimizer.minimize(session)

        print('Fitting done')

        if show_fitting:
            # Visualize fitting
            mv = MeshViewer()
            fitting_mesh = Mesh(session.run(tf_model), smpl.f)
            fitting_mesh.set_vertex_colors('light sky blue')

            mv.set_static_meshes([target_mesh, fitting_mesh])
            six.moves.input('Press key to continue')

        return Mesh(session.run(tf_model), smpl.f)


def run_corresponding_mesh_fitting():
    # Path of the FLAME model
    model_fname = './models/generic_model.pkl'
    # model_fname = '/models/female_model.pkl'
    # model_fname = '/models/male_model.pkl'

    # target 3D mesh in dense vertex-correspondence to the model
    target_mesh_path = './data/registered_mesh.ply'

    # Output filename
    out_mesh_fname = './results/mesh_fitting.ply'

    weights = {}
    # Weight of the data term
    weights['data'] = 1000.0
    # Weight of the shape regularizer (the lower, the less shape is constrained)
    weights['shape'] = 1e-4
    # Weight of the expression regularizer (the lower, the less expression is constrained)
    weights['expr']  = 1e-4
    # Weight of the neck pose (i.e. neck rotationh around the neck) regularizer (the lower, the less neck pose is constrained)
    weights['neck_pose'] = 1e-4
    # Weight of the jaw pose (i.e. jaw rotation for opening the mouth) regularizer (the lower, the less jaw pose is constrained)
    weights['jaw_pose'] = 1e-4
    # Weight of the eyeball pose (i.e. eyeball rotations) regularizer (the lower, the less eyeballs pose is constrained)
    weights['eyeballs_pose'] = 1e-4
    # Show landmark fitting (default: red = target landmarks, blue = fitting landmarks)
    show_fitting = True

    result_mesh = fit_3D_mesh(target_mesh_path, model_fname, weights, show_fitting=show_fitting)

    if not os.path.exists(os.path.dirname(out_mesh_fname)):
        os.makedirs(os.path.dirname(out_mesh_fname))

    result_mesh.write_ply(out_mesh_fname)


if __name__ == '__main__':
    run_corresponding_mesh_fitting()

```

## Source file: sample_texture.py

{
  "path": "sample_texture.py",
  "blob_sha": "2aa42fa38dce595e5dc66ed642810db9f1c81b0a",
  "sha256": "2830bb0a946661c33b12a4424390201276595a477cb2125bf1917b56932c6c84",
  "lines": 132,
  "bytes": 6764,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "jaw": 1,
    "mouth": 1,
    "gum": 5,
    "flame": 8,
    "rig": 5,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/TimoBolkart/TF_FLAME/blob/d4dab55213a93851f09689016be32d7fa10a6ce1/sample_texture.py",
  "function_signature_candidates": [
    {
      "line": 32,
      "text": "def sample_texture(model_fname, texture_fname, num_samples, out_path):"
    },
    {
      "line": 114,
      "text": "def main(args):"
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

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''


import os
import cv2
import six
import argparse
import numpy as np
import tensorflow as tf
from psbody.mesh import Mesh
from psbody.mesh.meshviewer import MeshViewer
from utils.landmarks import load_binary_pickle, load_embedding, tf_get_model_lmks, create_lmk_spheres

from tf_smpl.batch_smpl import SMPL
from tensorflow.contrib.opt import ScipyOptimizerInterface as scipy_pt

def sample_texture(model_fname, texture_fname, num_samples, out_path):
    '''
    Sample the FLAME model to demonstrate how to vary the model parameters.FLAME has parameters to
        - model identity-dependent shape variations (paramters: shape),
        - articulation of neck (paramters: pose[0:3]), jaw (paramters: pose[3:6]), and eyeballs (paramters: pose[6:12])
        - model facial expressions, i.e. all expression motion that does not involve opening the mouth (paramters: exp)
        - global translation (paramters: trans)
        - global rotation (paramters: rot)
    :param model_fname          saved FLAME model
    :param num_samples          number of samples
    :param out_path             output path to save the generated templates (no templates are saved if path is empty)
    '''

    tf_trans = tf.Variable(np.zeros((1,3)), name="trans", dtype=tf.float64, trainable=True)
    tf_rot = tf.Variable(np.zeros((1,3)), name="pose", dtype=tf.float64, trainable=True)
    tf_pose = tf.Variable(np.zeros((1,12)), name="pose", dtype=tf.float64, trainable=True)
    tf_shape = tf.Variable(np.zeros((1,300)), name="shape", dtype=tf.float64, trainable=True)
    tf_exp = tf.Variable(np.zeros((1,100)), name="expression", dtype=tf.float64, trainable=True)
    smpl = SMPL(model_fname)
    tf_model = tf.squeeze(smpl(tf_trans,
                               tf.concat((tf_shape, tf_exp), axis=-1),
                               tf.concat((tf_rot, tf_pose), axis=-1)))

    texture_model = np.load(texture_fname)
    if ('MU' in texture_model) and ('PC' in texture_model) and ('specMU' in texture_model) and ('specPC' in texture_model):
        b_albedoMM = True
    elif ('mean' in texture_model) and ('tex_dir' in texture_model):
        b_albedoMM = False
    else:
        print('Unknown texture model - %s' % texture_fname)
        return
 
    if b_albedoMM:
        # Albedo Morphable Model 
        num_tex_pc = texture_model['PC'].shape[-1]
        tex_shape = texture_model['MU'].shape

        tf_tex_params = tf.Variable(np.zeros((1,num_tex_pc)), name="params", dtype=tf.float64, trainable=True)
        
        tf_MU = tf.Variable(np.reshape(texture_model['MU'], (1,-1)), name='MU', dtype=tf.float64, trainable=False)
        tf_PC = tf.Variable(np.reshape(texture_model['PC'], (-1, num_tex_pc)).T, name='PC', dtype=tf.float64, trainable=False)
        tf_specMU = tf.Variable(np.reshape(texture_model['specMU'], (1,-1)), name='specMU', dtype=tf.float64, trainable=False)
        tf_specPC = tf.Variable(np.reshape(texture_model['specPC'], (-1, num_tex_pc)).T, name='specPC', dtype=tf.float64, trainable=False)

        tf_diff_albedo = tf.add(tf_MU, tf.matmul(tf_tex_params, tf_PC))
        tf_spec_albedo = tf.add(tf_specMU, tf.matmul(tf_tex_params, tf_specPC))
        tf_tex = 255*tf.math.pow(0.6*tf.add(tf_diff_albedo, tf_spec_albedo), 1.0/2.2)
    else:
        # MPI texture space or equivalent
        num_tex_pc = texture_model['tex_dir'].shape[-1]
        tex_shape = texture_model['mean'].shape

        tf_tex_params = tf.Variable(np.zeros((1,num_tex_pc)), name="params", dtype=tf.float64, trainable=True)
        tf_tex_mean = tf.Variable(np.reshape(texture_model['mean'], (1,-1)), name='tex_mean', dtype=tf.float64, trainable=False)
        tf_tex_dir = tf.Variable(np.reshape(texture_model['tex_dir'], (-1, num_tex_pc)).T, name='tex_dir', dtype=tf.float64, trainable=False)
        tf_tex = tf.add(tf_tex_mean, tf.matmul(tf_tex_params, tf_tex_dir))

    tf_tex = tf.reshape(tf_tex, (tex_shape[0], tex_shape[1], tex_shape[2]))
    tf_tex = tf.cast(tf.clip_by_value(tf_tex, 0.0, 255.0), tf.int64)

    with tf.Session() as session:
        session.run(tf.global_variables_initializer())
        mv = MeshViewer()

        for i in range(num_samples):
            assign_tex = tf.assign(tf_tex_params, np.random.randn(num_tex_pc)[np.newaxis,:])
            session.run([assign_tex])

            v, tex = session.run([tf_model, tf_tex])
            out_mesh = Mesh(v, smpl.f)
            out_mesh.vt = texture_model['vt']
            out_mesh.ft = texture_model['ft']

            mv.set_dynamic_meshes([out_mesh], blocking=True)
            key = six.moves.input('Press (s) to save sample, any other key to continue ')
            if key == 's':
                out_mesh_fname = os.path.join(out_path, 'tex_sample_%02d.obj' % (i+1))
                out_tex_fname = out_mesh_fname.replace('obj', 'png')
                cv2.imwrite(out_tex_fname, tex)
                out_mesh.set_texture_image(out_tex_fname)
                out_mesh.write_obj(out_mesh_fname)

def main(args):
    if not os.path.exists(args.model_fname):
        print('FLAME model not found - %s' % args.model_fname)
        return
    if not os.path.exists(args.texture_fname):
        print('Texture model not found - %s' % args.texture_fname)
        return
    if not os.path.exists(args.out_path):
        os.makedirs(args.out_path)
    sample_texture(args.model_fname, args.texture_fname, int(args.num_samples), args.out_path)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Sample FLAME shape space')
    parser.add_argument('--model_fname', default='./models/generic_model.pkl', help='Path of the FLAME model')
    parser.add_argument('--texture_fname', default='./models/FLAME_texture.npz', help='Path of the texture model')
    parser.add_argument('--num_samples', default='5', help='Number of samples')
    parser.add_argument('--out_path', default='./texture_samples', help='Output path')
    args = parser.parse_args()
    main(args)


```

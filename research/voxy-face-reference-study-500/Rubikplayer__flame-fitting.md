# Facial rendering source scan: Rubikplayer/flame-fitting

Repository: https://github.com/Rubikplayer/flame-fitting

Pinned source revision: ca806ce13a8964231136bd226bf3255fc2e476de

Discovery topic: FLAME 3D

Description: Example code for the FLAME 3D head model. The code demonstrates how to sample 3D heads from the model, fit the model to 3D keypoints and 3D scans.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["fetch_FLAME.sh", "models/flame_static_embedding.pkl"], "lbs": ["smpl_webuser/lbs.py"]}

## Source file: smpl_webuser/lbs.py

{
  "path": "smpl_webuser/lbs.py",
  "blob_sha": "7977683851d293e7fdaeec94e70e4740212e3d2b",
  "sha256": "f0687e51d390fe7d1ee47ac950644cfe08112f5babeb7ca0eb8705b93e62c593",
  "lines": 81,
  "bytes": 2890,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 2,
    "blendshape": 1,
    "rig": 9
  },
  "url": "https://github.com/Rubikplayer/flame-fitting/blob/ca806ce13a8964231136bd226bf3255fc2e476de/smpl_webuser/lbs.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def global_rigid_transformation(pose, J, kintree_table, xp):"
    },
    {
      "line": 64,
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

# from posemapper import posemap
import chumpy
import numpy as np
from smpl_webuser.posemapper import posemap

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

## Source file: fit_scan.py

{
  "path": "fit_scan.py",
  "blob_sha": "5caec9cb0f8c20ba9a7d68abd7fa953eae265a0c",
  "sha256": "0e47bce39e7b534348c7a25a07815600567b7427522b3e2f5fd5b1a871f811f1",
  "lines": 261,
  "bytes": 11066,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 1,
    "flame": 9,
    "rig": 8,
    "face": 14
  },
  "url": "https://github.com/Rubikplayer/flame-fitting/blob/ca806ce13a8964231136bd226bf3255fc2e476de/fit_scan.py",
  "function_signature_candidates": [
    {
      "line": 27,
      "text": "def compute_approx_scale(lmk_3d, model, lmk_face_idx, lmk_b_coords, opt_options=None):"
    },
    {
      "line": 61,
      "text": "def on_step(_):"
    },
    {
      "line": 73,
      "text": "def fit_scan(  scan,                        # input scan"
    },
    {
      "line": 140,
      "text": "def on_step(_):"
    },
    {
      "line": 172,
      "text": "def run_fitting():"
    }
  ]
}

```text

'''
Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this computer program. 
Using this computer program means that you agree to the terms in the LICENSE file (https://flame.is.tue.mpg.de/modellicense) included 
with the FLAME model. Any use not explicitly granted by the LICENSE is prohibited.

Copyright 2020 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its 
Max Planck Institute for Intelligent Systems. All rights reserved.

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''

import numpy as np
import chumpy as ch
from os.path import join

from psbody.mesh import Mesh
from smpl_webuser.serialization import load_model
from sbody.mesh_distance import ScanToMesh
from sbody.robustifiers import GMOf
from sbody.alignment.objectives import sample_from_mesh
from fitting.landmarks import load_embedding, landmark_error_3d, mesh_points_by_barycentric_coordinates
from fitting.util import load_binary_pickle, write_simple_obj, safe_mkdir, get_unit_factor

# -----------------------------------------------------------------------------

def compute_approx_scale(lmk_3d, model, lmk_face_idx, lmk_b_coords, opt_options=None):
    """ function: compute approximate scale to align scan and model

    input: 
        lmk_3d: input landmark 3d, in shape (N,3)
        model: FLAME face model
        lmk_face_idx, lmk_b_coords: landmark embedding, in face indices and barycentric coordinates
        opt_options: optimizaton options

    output:
        model.r: fitted result vertices
        model.f: fitted result triangulations (fixed in this code)
        parms: fitted model parameters

    """

    scale = ch.ones(1)
    scan_lmks = scale*ch.array(lmk_3d)
    model_lmks = mesh_points_by_barycentric_coordinates( model, model.f, lmk_face_idx, lmk_b_coords )
    lmk_err = scan_lmks-model_lmks

    # options
    if opt_options is None:
        print("fit_lmk3d(): no 'opt_options' provided, use default settings.")
        import scipy.sparse as sp
        opt_options = {}
        opt_options['disp']    = 1
        opt_options['delta_0'] = 0.1
        opt_options['e_3']     = 1e-4
        opt_options['maxiter'] = 2000
        sparse_solver = lambda A, x: sp.linalg.cg(A, x, maxiter=opt_options['maxiter'])[0]
        opt_options['sparse_solver'] = sparse_solver

    # on_step callback
    def on_step(_):
        pass

    ch.minimize( fun      = lmk_err,
                 x0       = [ scale, model.trans, model.pose[:3] ],
                 method   = 'dogleg',
                 callback = on_step,
                 options  = opt_options )
    return scale.r

# -----------------------------------------------------------------------------

def fit_scan(  scan,                        # input scan
               lmk_3d,                      # input scan landmarks
               model,                       # model
               lmk_face_idx, lmk_b_coords,  # landmark embedding
               weights,                     # weights for the objectives
               gmo_sigma,                   # weight of the robustifier
               shape_num=300, expr_num=100, opt_options=None ):
    
    """ function: fit FLAME model to a 3D scan

    input: 
        scan: input scan
        lmk_3d: input landmark 3d, in shape (N,3)
        model: FLAME face model
        lmk_face_idx, lmk_b_coords: landmark embedding, in face indices and barycentric coordinates
        weights: weights for each objective
        shape_num, expr_num: numbers of shape and expression compoenents used
        opt_options: optimizaton options

    output:
        model.r: fitted result vertices
        model.f: fitted result triangulations (fixed in this code)
        parms: fitted model parameters

    """

    # variables
    shape_idx      = np.arange( 0, min(300,shape_num) )        # valid shape component range in "betas": 0-299
    expr_idx       = np.arange( 300, 300+min(100,expr_num) )   # valid expression component range in "betas": 300-399
    used_idx       = np.union1d( shape_idx, expr_idx )
    model.betas[:] = np.random.rand( model.betas.size ) * 0.0  # initialized to zero
    model.pose[:]  = np.random.rand( model.pose.size ) * 0.0   # initialized to zero
    free_variables = [model.trans, model.pose, model.betas[used_idx]] 
    
    # weights
    print("fit_scan(): use the following weights:")
    for kk in weights.keys():
        print("fit_scan(): weights['%s'] = %f" % ( kk, weights[kk] ))

    # objectives
    # landmark error
    lmk_err = landmark_error_3d(mesh_verts=model, mesh_faces=model.f,  lmk_3d=lmk_3d, lmk_face_idx=lmk_face_idx, lmk_b_coords=lmk_b_coords)

    # scan-to-mesh distance, measuring the distance between scan vertices and the closest points in the model's surface
    sampler = sample_from_mesh(scan, sample_type='vertices')
    s2m = ScanToMesh(scan, model, model.f, scan_sampler=sampler, rho=lambda x: GMOf(x, sigma=gmo_sigma))

    # regularizer
    shape_err = weights['shape'] * model.betas[shape_idx] 
    expr_err  = weights['expr']  * model.betas[expr_idx] 
    pose_err  = weights['pose']  * model.pose[3:] # exclude global rotation

    objectives = {'s2m': weights['s2m']*s2m, 'lmk': weights['lmk']*lmk_err, 'shape': shape_err, 'expr': expr_err, 'pose': pose_err} 

    # options
    if opt_options is None:
        print("fit_lmk3d(): no 'opt_options' provided, use default settings.")
        import scipy.sparse as sp
        opt_options = {}
        opt_options['disp']    = 1
        opt_options['delta_0'] = 0.1
        opt_options['e_3']     = 1e-4
        opt_options['maxiter'] = 2000
        sparse_solver = lambda A, x: sp.linalg.cg(A, x, maxiter=opt_options['maxiter'])[0]
        opt_options['sparse_solver'] = sparse_solver

    # on_step callback
    def on_step(_):
        pass
        
    # optimize
    # step 1: rigid alignment
    from time import time
    timer_start = time()
    print("\nstep 1: start rigid fitting...")
    ch.minimize( fun      = lmk_err,
                 x0       = [ model.trans, model.pose[:3] ],
                 method   = 'dogleg',
                 callback = on_step,
                 options  = opt_options )
    timer_end = time()
    print("step 1: fitting done, in %f sec\n" % ( timer_end - timer_start ))

    # step 2: non-rigid alignment
    timer_start = time()
    print("step 2: start non-rigid fitting...")    
    ch.minimize( fun      = objectives,
                 x0       = free_variables,
                 method   = 'dogleg',
                 callback = on_step,
                 options  = opt_options )
    timer_end = time()
    print("step 2: fitting done, in %f sec\n" % ( timer_end - timer_start ))

    # return results
    parms = { 'trans': model.trans.r, 'pose': model.pose.r, 'betas': model.betas.r }
    return model.r, model.f, parms


def run_fitting():
    # input scan
    scan_path = './data/scan.obj'

    # landmarks of the scan
    scan_lmk_path = './data/scan_lmks.npy'

    # measurement unit of landmarks ['m', 'cm', 'mm', 'NA'] 
    # When using option 'NA', the scale of the scan will be estimated by rigidly aligning model and scan landmarks
    scan_unit = 'm' 

    scan = Mesh(filename=scan_path)
    print("loaded scan from:", scan_path)

    lmk_3d = np.load(scan_lmk_path)
    print("loaded scan landmark from:", scan_lmk_path)

    # model
    model_path = './models/generic_model.pkl' # change to 'female_model.pkl' or 'male_model.pkl', if gender is known
    model = load_model(model_path)       # the loaded model object is a 'chumpy' object, check https://github.com/mattloper/chumpy for details
    print("loaded model from:", model_path)

    # landmark embedding
    lmk_emb_path = './models/flame_static_embedding.pkl' 
    lmk_face_idx, lmk_b_coords = load_embedding(lmk_emb_path)
    print("loaded lmk embedding")

    # scale scans and scan landmarks to be in the same local coordinate systems as the FLAME model
    if scan_unit.lower() == 'na':
        print('No scale specifiec - compute approximate scale based on the landmarks')
        scale_factor = compute_approx_scale(lmk_3d, model, lmk_face_idx, lmk_b_coords)
        print('Scale factor: %f' % scale_factor)
    else:
        scale_factor = get_unit_factor('m') / get_unit_factor(scan_unit)
        print('Scale factor: %f' % scale_factor)        

    scan.v[:] *= scale_factor
    lmk_3d[:] *= scale_factor

    # output
    output_dir = './output'
    safe_mkdir(output_dir)

    # weights
    weights = {}
    # scan vertex to model surface distance term
    weights['s2m']   = 2.0   
    # landmark term
    weights['lmk']   = 1e-2
    # shape regularizer (weight higher to regularize face shape more towards the mean)
    weights['shape'] = 1e-4
    # expression regularizer (weight higher to regularize facial expression more towards the mean)
    weights['expr']  = 1e-4
    # regularization of head rotation around the neck and jaw opening (weight higher for more regularization)
    weights['pose']  = 1e-3
    # Parameter of the Geman-McClure robustifier (higher weight for a larger bassin of attraction which makes it less robust to outliers)
    gmo_sigma = 1e-4

    # optimization options
    import scipy.sparse as sp
    opt_options = {}
    opt_options['disp']    = 1
    opt_options['delta_0'] = 0.1
    opt_options['e_3']     = 1e-4
    opt_options['maxiter'] = 2000
    sparse_solver = lambda A, x: sp.linalg.cg(A, x, maxiter=opt_options['maxiter'])[0]
    opt_options['sparse_solver'] = sparse_solver

    # run fitting
    mesh_v, mesh_f, parms = fit_scan(   scan=scan,                                             # input scan
                                        lmk_3d=lmk_3d,                                         # input landmark 3d
                                        model=model,                                           # model
                                        lmk_face_idx=lmk_face_idx, lmk_b_coords=lmk_b_coords,  # landmark embedding
                                        weights=weights,                                       # weights for the objectives
                                        gmo_sigma=gmo_sigma,                                   # parameter of the regularizer
                                        shape_num=300, expr_num=100, opt_options=opt_options ) # options

    # write result
    output_path = join( output_dir, 'fit_scan_result.obj' )
    write_simple_obj( mesh_v=mesh_v, mesh_f=mesh_f, filepath=output_path, verbose=False )
    print('output mesh saved to: ', output_path) 

    # output scaled scan for reference (output scan fit and the scan should be spatially aligned)
    output_path = join( output_dir, 'scan_scaled.obj' )    
    write_simple_obj( mesh_v=scan.v, mesh_f=scan.f, filepath=output_path, verbose=False )

# -----------------------------------------------------------------------------

if __name__ == '__main__':
    run_fitting()


```

## Source file: fit_lmk3d.py

{
  "path": "fit_lmk3d.py",
  "blob_sha": "26c00afb6d2c307d5ac93066ccb9bbc3a2b25f57",
  "sha256": "a8ffe1625ae8e304c73166acc0c466375a71c66cd7c1ce80365fab722e97b7a5",
  "lines": 185,
  "bytes": 7816,
  "selection_score": 0,
  "keyword_line_counts": {
    "jaw": 2,
    "flame": 7,
    "rig": 7,
    "face": 8
  },
  "url": "https://github.com/Rubikplayer/flame-fitting/blob/ca806ce13a8964231136bd226bf3255fc2e476de/fit_lmk3d.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def fit_lmk3d( lmk_3d,                      # input landmark 3d"
    },
    {
      "line": 88,
      "text": "def on_step(_):"
    },
    {
      "line": 121,
      "text": "def run_fitting():"
    }
  ]
}

```text

'''
Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this computer program. 
Using this computer program means that you agree to the terms in the LICENSE file (https://flame.is.tue.mpg.de/modellicense) included 
with the FLAME model. Any use not explicitly granted by the LICENSE is prohibited.

Copyright 2020 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its 
Max Planck Institute for Intelligent Systems. All rights reserved.

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''

import numpy as np
import chumpy as ch
from os.path import join

from smpl_webuser.serialization import load_model
from fitting.landmarks import load_embedding, landmark_error_3d
from fitting.util import load_binary_pickle, write_simple_obj, safe_mkdir, get_unit_factor

# -----------------------------------------------------------------------------

def fit_lmk3d( lmk_3d,                      # input landmark 3d
               model,                       # model
               lmk_face_idx, lmk_b_coords,  # landmark embedding
               weights,                     # weights for the objectives
               shape_num=300, expr_num=100, opt_options=None ):
    
    """ function: fit FLAME model to 3D landmarks

    input: 
        lmk_3d: input landmark 3D, in shape (N,3)
        model: FLAME face model
        lmk_face_idx, lmk_b_coords: landmark embedding, in face indices and barycentric coordinates
        weights: weights for each objective
        shape_num, expr_num: numbers of shape and expression compoenents used
        opt_options: optimizaton options

    output:
        model.r: fitted result vertices
        model.f: fitted result triangulations (fixed in this code)
        parms: fitted model parameters

    """

    # variables
    pose_idx       = np.union1d(np.arange(3), np.arange(6,9)) # global rotation and jaw rotation
    shape_idx      = np.arange( 0, min(300,shape_num) )        # valid shape component range in "betas": 0-299
    expr_idx       = np.arange( 300, 300+min(100,expr_num) )   # valid expression component range in "betas": 300-399
    used_idx       = np.union1d( shape_idx, expr_idx )
    model.betas[:] = np.random.rand( model.betas.size ) * 0.0  # initialized to zero
    model.pose[:]  = np.random.rand( model.pose.size ) * 0.0   # initialized to zero
    free_variables = [ model.trans, model.pose[pose_idx], model.betas[used_idx] ] 
    
    # weights
    print("fit_lmk3d(): use the following weights:")
    for kk in weights.keys():
        print("fit_lmk3d(): weights['%s'] = %f" % ( kk, weights[kk] ))

    # objectives
    # lmk
    lmk_err = landmark_error_3d( mesh_verts=model, 
                                 mesh_faces=model.f, 
                                 lmk_3d=lmk_3d, 
                                 lmk_face_idx=lmk_face_idx, 
                                 lmk_b_coords=lmk_b_coords, 
                                 weight=weights['lmk'] )
    # regularizer
    shape_err = weights['shape'] * model.betas[shape_idx] 
    expr_err  = weights['expr']  * model.betas[expr_idx] 
    pose_err  = weights['pose']  * model.pose[3:] # exclude global rotation
    objectives = {}
    objectives.update( { 'lmk': lmk_err, 'shape': shape_err, 'expr': expr_err, 'pose': pose_err } ) 

    # options
    if opt_options is None:
        print("fit_lmk3d(): no 'opt_options' provided, use default settings.")
        import scipy.sparse as sp
        opt_options = {}
        opt_options['disp']    = 1
        opt_options['delta_0'] = 0.1
        opt_options['e_3']     = 1e-4
        opt_options['maxiter'] = 2000
        sparse_solver = lambda A, x: sp.linalg.cg(A, x, maxiter=opt_options['maxiter'])[0]
        opt_options['sparse_solver'] = sparse_solver

    # on_step callback
    def on_step(_):
        pass
        
    # optimize
    # step 1: rigid alignment
    from time import time
    timer_start = time()
    print("\nstep 1: start rigid fitting...")
    ch.minimize( fun      = lmk_err,
                 x0       = [ model.trans, model.pose[0:3] ],
                 method   = 'dogleg',
                 callback = on_step,
                 options  = opt_options )
    timer_end = time()
    print("step 1: fitting done, in %f sec\n" % ( timer_end - timer_start ))

    # step 2: non-rigid alignment
    timer_start = time()
    print("step 2: start non-rigid fitting...")    
    ch.minimize( fun      = objectives,
                 x0       = free_variables,
                 method   = 'dogleg',
                 callback = on_step,
                 options  = opt_options )
    timer_end = time()
    print("step 2: fitting done, in %f sec\n" % ( timer_end - timer_start ))

    # return results
    parms = { 'trans': model.trans.r, 'pose': model.pose.r, 'betas': model.betas.r }
    return model.r, model.f, parms

# -----------------------------------------------------------------------------

def run_fitting():
    # input landmarks
    lmk_path = './data/scan_lmks.npy'
    # measurement unit of landmarks ['m', 'cm', 'mm']
    unit = 'm' 

    scale_factor = get_unit_factor('m') / get_unit_factor(unit)
    lmk_3d = scale_factor*np.load(lmk_path)
    print("loaded 3d landmark from:", lmk_path)

    # model
    model_path = './models/generic_model.pkl' # change to 'female_model.pkl' or 'male_model.pkl', if gender is known
    model = load_model(model_path)       # the loaded model object is a 'chumpy' object, check https://github.com/mattloper/chumpy for details
    print("loaded model from:", model_path)

    # landmark embedding
    lmk_emb_path = './models/flame_static_embedding.pkl' 
    lmk_face_idx, lmk_b_coords = load_embedding(lmk_emb_path)
    print("loaded lmk embedding")

    # output
    output_dir = './output'
    safe_mkdir(output_dir)

    # weights
    weights = {}
    # landmark term
    weights['lmk']   = 1.0   
    # shape regularizer (weight higher to regularize face shape more towards the mean)
    weights['shape'] = 1e-3
    # expression regularizer (weight higher to regularize facial expression more towards the mean)
    weights['expr']  = 1e-3
    # regularization of head rotation around the neck and jaw opening (weight higher for more regularization)
    weights['pose']  = 1e-2
    
    # number of shape and expression parameters (we do not recommend using too many parameters for fitting to sparse keypoints)
    shape_num = 100
    expr_num = 50

    # optimization options
    import scipy.sparse as sp
    opt_options = {}
    opt_options['disp']    = 1
    opt_options['delta_0'] = 0.1
    opt_options['e_3']     = 1e-4
    opt_options['maxiter'] = 2000
    sparse_solver = lambda A, x: sp.linalg.cg(A, x, maxiter=opt_options['maxiter'])[0]
    opt_options['sparse_solver'] = sparse_solver

    # run fitting
    mesh_v, mesh_f, parms = fit_lmk3d( lmk_3d=lmk_3d,                                         # input landmark 3d
                                       model=model,                                           # model
                                       lmk_face_idx=lmk_face_idx, lmk_b_coords=lmk_b_coords,  # landmark embedding
                                       weights=weights,                                       # weights for the objectives
                                       shape_num=shape_num, expr_num=expr_num, opt_options=opt_options ) # options

    # write result
    output_path = join( output_dir, 'fit_lmk3d_result.obj' )
    write_simple_obj( mesh_v=mesh_v, mesh_f=mesh_f, filepath=output_path, verbose=False )

# -----------------------------------------------------------------------------

if __name__ == '__main__':
    run_fitting()



```

## Source file: hello_world.py

{
  "path": "hello_world.py",
  "blob_sha": "c05912d84df7a5bc43a8b95c3468a49eace7467b",
  "sha256": "b1a6bdae682c1d2382bccaf52a31cf35355677fb54bca19738a848d4589651ba",
  "lines": 57,
  "bytes": 2734,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 1,
    "flame": 8,
    "rig": 3
  },
  "url": "https://github.com/Rubikplayer/flame-fitting/blob/ca806ce13a8964231136bd226bf3255fc2e476de/hello_world.py",
  "function_signature_candidates": []
}

```text

'''
Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG) is holder of all proprietary rights on this computer program. 
Using this computer program means that you agree to the terms in the LICENSE file (https://flame.is.tue.mpg.de/modellicense) included 
with the FLAME model. Any use not explicitly granted by the LICENSE is prohibited.

Copyright 2020 Max-Planck-Gesellschaft zur Foerderung der Wissenschaften e.V. (MPG). acting on behalf of its 
Max Planck Institute for Intelligent Systems. All rights reserved.

More information about FLAME is available at http://flame.is.tue.mpg.de.
For comments or questions, please email us at flame@tue.mpg.de
'''

# This script is based on the hello-world script from SMPL python code http://smpl.is.tue.mpg.de/downloads

import numpy as np
from os.path import join
from smpl_webuser.serialization import load_model
from fitting.util import write_simple_obj, safe_mkdir

# -----------------------------------------------------------------------------

if __name__ == '__main__':

    # Load FLAME model (here we load the generic model)
    # Make sure path is correct
    model_path = './models/generic_model.pkl'
    model = load_model( model_path )           # the loaded model object is a 'chumpy' object, check https://github.com/mattloper/chumpy for details
    print("loaded model from:", model_path)

    # Show component number
    print("\nFLAME coefficients:")
    print("shape (identity) coefficient shape =", model.betas[0:300].shape) # valid shape component range in "betas": 0-299
    print("expression coefficient shape       =", model.betas[300:].shape)  # valid expression component range in "betas": 300-399
    print("pose coefficient shape             =", model.pose.shape)

    print("\nFLAME model components:")
    print("shape (identity) component shape =", model.shapedirs[:,:,0:300].shape)
    print("expression component shape       =", model.shapedirs[:,:,300:].shape)
    print("pose corrective blendshape shape =", model.posedirs.shape)
    print("\n")

    # -----------------------------------------------------------------------------

    # Assign random pose and shape parameters
    model.pose[:]  = np.random.randn(model.pose.size) * 0.02
    model.betas[:100] = np.random.randn(100) * 0.6
    model.betas[300:350] = np.random.randn(50) * 0.6
    # model.trans[:] = np.random.randn( model.trans.size ) * 0.01   # you may also manipulate the translation of mesh

    # Write to an .obj file
    outmesh_dir = './output'
    safe_mkdir( outmesh_dir )
    outmesh_path = join( outmesh_dir, 'hello_flame.obj')
    write_simple_obj( mesh_v=model.r, mesh_f=model.f, filepath=outmesh_path )

    print('output mesh saved to: ', outmesh_path) 



```

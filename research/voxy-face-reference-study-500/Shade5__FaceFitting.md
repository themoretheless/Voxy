# Facial rendering source scan: Shade5/FaceFitting

Repository: https://github.com/Shade5/FaceFitting

Pinned source revision: b8106ae90f3fc369c3ab092adae11f78dfcc6e2e

Discovery topic: face model 3D

Description: Fits a 3D model to a 2D image of a face

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["data/face.jpeg", "face_space.py", "models/shape_predictor_68_face_landmarks.dat", "results/face_fit.png", "results/face_ori.png"]}

## Source file: face_space.py

{
  "path": "face_space.py",
  "blob_sha": "84d28c32cf327a00e453f9fd19d95eb59ff0d41c",
  "sha256": "60231bdc57c8102d1c5ff65bfedd66482fc0c3240b7b64cc348887a2ccf2ffb0",
  "lines": 33,
  "bytes": 1210,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/Shade5/FaceFitting/blob/b8106ae90f3fc369c3ab092adae11f78dfcc6e2e/face_space.py",
  "function_signature_candidates": []
}

```text

import numpy as np
from MorphabelModel import MorphabelModel
import mesh
import imageio

bfm = MorphabelModel('models/BFM.mat')

shape_coefficients = np.zeros((bfm.n_shape_para, 1))
expr_coefficients = np.zeros((bfm.n_exp_para, 1))
color_coefficients = np.zeros((bfm.n_tex_para, 1))
images = []

for i in range(50):
    shape_coefficients += 0.2*np.vstack((np.random.randn(10, 1), np.zeros((bfm.n_shape_para - 10, 1))))
    expr_coefficients += 0.0001*np.vstack((np.random.randn(5, 1), np.zeros((bfm.n_exp_para - 5, 1))))
    color_coefficients += 0.2*np.vstack((np.random.randn(10, 1), np.zeros((bfm.n_tex_para - 10, 1))))
    vertices = bfm.generate_vertices(shape_coefficients, expr_coefficients)
    colors = bfm.generate_colors(color_coefficients)

    s = 2e-03
    angles = [0, 0, 0]
    t = [0, 0, 0]
    transformed_vertices = bfm.transform(vertices, s, angles, t)
    projected_vertices = transformed_vertices.copy()

    h = w = 512
    c = 3
    image_vertices = mesh.transform.to_image(projected_vertices, h, w)
    image = mesh.render.render_colors(image_vertices, bfm.triangles, colors, h, w)
    images.append(image)

images.extend(reversed(images))
imageio.mimsave('results/movie.gif', images)

```

## Source file: fit.py

{
  "path": "fit.py",
  "blob_sha": "c0aa30e5df7231ff28224600431673f3806e7c18",
  "sha256": "a0c8db747f76fbe604652afdfd6631b964a65e52dc5a4d0bcf0b52db4ba0cb2c",
  "lines": 283,
  "bytes": 8134,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 4
  },
  "url": "https://github.com/Shade5/FaceFitting/blob/b8106ae90f3fc369c3ab092adae11f78dfcc6e2e/fit.py",
  "function_signature_candidates": [
    {
      "line": 56,
      "text": "def estimate_shape(x, shapeMU, shapePC, shapeEV, expression, s, R, t2d, lamb=3000):"
    },
    {
      "line": 110,
      "text": "def estimate_expression(x, shapeMU, expPC, expEV, shape, s, R, t2d, lamb=2000):"
    },
    {
      "line": 164,
      "text": "def fit_points(x, X_ind, model, n_sp, n_ep, max_iter=4):"
    },
    {
      "line": 217,
      "text": "def fit_points_for_show(x, X_ind, model, n_sp, n_ep, max_iter=4):"
    }
  ]
}

```text

'''
Estimating parameters about vertices: shape para, exp para, pose para(s, R, t)
'''
import numpy as np
import mesh

''' TODO: a clear document. 
Given: image_points, 3D Model, Camera Matrix(s, R, t2d)
Estimate: shape parameters, expression parameters

Inference: 

    projected_vertices = s*P*R(mu + shape + exp) + t2d  --> image_points
    s*P*R*shape + s*P*R(mu + exp) + t2d --> image_poitns

    # Define:
    X = vertices
    x_hat = projected_vertices
    x = image_points
    A = s*P*R
    b = s*P*R(mu + exp) + t2d
    ==>
    x_hat = A*shape + b  (2 x n)

    A*shape (2 x n)
    shape = reshape(shapePC * sp) (3 x n)
    shapePC*sp : (3n x 1)

    * flatten:
    x_hat_flatten = A*shape + b_flatten  (2n x 1)
    A*shape (2n x 1)
    --> A*shapePC (2n x 199)  sp: 199 x 1

    # Define:
    pc_2d = A* reshape(shapePC)
    pc_2d_flatten = flatten(pc_2d) (2n x 199)

    =====>
    x_hat_flatten = pc_2d_flatten * sp + b_flatten ---> x_flatten (2n x 1)

    Goals:
    (ignore flatten, pc_2d-->pc)
    min E = || x_hat - x || + lambda*sum(sp/sigma)^2
          = || pc * sp + b - x || + lambda*sum(sp/sigma)^2

    Solve:
    d(E)/d(sp) = 0
    2 * pc' * (pc * sp + b - x) + 2 * lambda * sp / (sigma' * sigma) = 0

    Get:
    (pc' * pc + lambda / (sigma'* sigma)) * sp  = pc' * (x - b)

'''


def estimate_shape(x, shapeMU, shapePC, shapeEV, expression, s, R, t2d, lamb=3000):
    '''
    Args:
        x: (2, n). image points (to be fitted)
        shapeMU: (3n, 1)
        shapePC: (3n, n_sp)
        shapeEV: (n_sp, 1)
        expression: (3, n)
        s: scale
        R: (3, 3). rotation matrix
        t2d: (2,). 2d translation
        lambda: regulation coefficient

    Returns:
        shape_para: (n_sp, 1) shape parameters(coefficients)
    '''
    x = x.copy()
    assert (shapeMU.shape[0] == shapePC.shape[0])
    assert (shapeMU.shape[0] == x.shape[1] * 3)

    dof = shapePC.shape[1]

    n = x.shape[1]
    sigma = shapeEV
    t2d = np.array(t2d)
    P = np.array([[1, 0, 0], [0, 1, 0]], dtype=np.float32)
    A = s * P.dot(R)

    # --- calc pc
    pc_3d = np.resize(shapePC.T, [dof, n, 3])  # 199 x n x 3
    pc_3d = np.reshape(pc_3d, [dof * n, 3])
    pc_2d = pc_3d.dot(A.T.copy())  # 199 x n x 2

    pc = np.reshape(pc_2d, [dof, -1]).T  # 2n x 199

    # --- calc b
    # shapeMU
    mu_3d = np.resize(shapeMU, [n, 3]).T  # 3 x n
    # expression
    exp_3d = expression
    #
    b = A.dot(mu_3d + exp_3d) + np.tile(t2d[:, np.newaxis], [1, n])  # 2 x n
    b = np.reshape(b.T, [-1, 1])  # 2n x 1

    # --- solve
    equation_left = np.dot(pc.T, pc) + lamb * np.diagflat(1 / sigma ** 2)
    x = np.reshape(x.T, [-1, 1])
    equation_right = np.dot(pc.T, x - b)

    shape_para = np.dot(np.linalg.inv(equation_left), equation_right)

    return shape_para


def estimate_expression(x, shapeMU, expPC, expEV, shape, s, R, t2d, lamb=2000):
    '''
    Args:
        x: (2, n). image points (to be fitted)
        shapeMU: (3n, 1)
        expPC: (3n, n_ep)
        expEV: (n_ep, 1)
        shape: (3, n)
        s: scale
        R: (3, 3). rotation matrix
        t2d: (2,). 2d translation
        lambda: regulation coefficient

    Returns:
        exp_para: (n_ep, 1) shape parameters(coefficients)
    '''
    x = x.copy()
    assert (shapeMU.shape[0] == expPC.shape[0])
    assert (shapeMU.shape[0] == x.shape[1] * 3)

    dof = expPC.shape[1]

    n = x.shape[1]
    sigma = expEV
    t2d = np.array(t2d)
    P = np.array([[1, 0, 0], [0, 1, 0]], dtype=np.float32)
    A = s * P.dot(R)

    # --- calc pc
    pc_3d = np.resize(expPC.T, [dof, n, 3])
    pc_3d = np.reshape(pc_3d, [dof * n, 3])
    pc_2d = pc_3d.dot(A.T)
    pc = np.reshape(pc_2d, [dof, -1]).T  # 2n x 29

    # --- calc b
    # shapeMU
    mu_3d = np.resize(shapeMU, [n, 3]).T  # 3 x n
    # expression
    shape_3d = shape
    #
    b = A.dot(mu_3d + shape_3d) + np.tile(t2d[:, np.newaxis], [1, n])  # 2 x n
    b = np.reshape(b.T, [-1, 1])  # 2n x 1

    # --- solve
    equation_left = np.dot(pc.T, pc) + lamb * np.diagflat(1 / sigma ** 2)
    x = np.reshape(x.T, [-1, 1])
    equation_right = np.dot(pc.T, x - b)

    exp_para = np.dot(np.linalg.inv(equation_left), equation_right)

    return exp_para


# ---------------- fit
def fit_points(x, X_ind, model, n_sp, n_ep, max_iter=4):
    '''
    Args:
        x: (n, 2) image points
        X_ind: (n,) corresponding Model vertex indices
        model: 3DMM
        max_iter: iteration
    Returns:
        sp: (n_sp, 1). shape parameters
        ep: (n_ep, 1). exp parameters
        s, R, t
    '''
    x = x.copy().T

    # -- init
    sp = np.zeros((n_sp, 1), dtype=np.float32)
    ep = np.zeros((n_ep, 1), dtype=np.float32)

    # -------------------- estimate
    X_ind_all = np.tile(X_ind[np.newaxis, :], [3, 1]) * 3
    X_ind_all[1, :] += 1
    X_ind_all[2, :] += 2
    valid_ind = X_ind_all.flatten('F')

    shapeMU = model['shapeMU'][valid_ind, :]
    shapePC = model['shapePC'][valid_ind, :n_sp]
    expPC = model['expPC'][valid_ind, :n_ep]

    for i in range(max_iter):
        X = shapeMU + shapePC.dot(sp) + expPC.dot(ep)
        X = np.reshape(X, [int(len(X) / 3), 3]).T

        # ----- estimate pose
        P = mesh.transform.estimate_affine_matrix_3d22d(X.T, x.T)
        s, R, t = mesh.transform.P2sRt(P)
        rx, ry, rz = mesh.transform.matrix2angle(R)
        # print('Iter:{}; estimated pose: s {}, rx {}, ry {}, rz {}, t1 {}, t2 {}'.format(i, s, rx, ry, rz, t[0], t[1]))

        # ----- estimate shape
        # expression
        shape = shapePC.dot(sp)
        shape = np.reshape(shape, [int(len(shape) / 3), 3]).T
        ep = estimate_expression(x, shapeMU, expPC, model['expEV'][:n_ep, :], shape, s, R, t[:2], lamb=20)

        # shape
        expression = expPC.dot(ep)
        expression = np.reshape(expression, [int(len(expression) / 3), 3]).T
        sp = estimate_shape(x, shapeMU, shapePC, model['shapeEV'][:n_sp, :], expression, s, R, t[:2], lamb=40)

    return sp, ep, s, R, t


# ---------------- fitting process
def fit_points_for_show(x, X_ind, model, n_sp, n_ep, max_iter=4):
    '''
    Args:
        x: (n, 2) image points
        X_ind: (n,) corresponding Model vertex indices
        model: 3DMM
        max_iter: iteration
    Returns:
        sp: (n_sp, 1). shape parameters
        ep: (n_ep, 1). exp parameters
        s, R, t
    '''
    x = x.copy().T

    # -- init
    sp = np.zeros((n_sp, 1), dtype=np.float32)
    ep = np.zeros((n_ep, 1), dtype=np.float32)

    # -------------------- estimate
    X_ind_all = np.tile(X_ind[np.newaxis, :], [3, 1]) * 3
    X_ind_all[1, :] += 1
    X_ind_all[2, :] += 2
    valid_ind = X_ind_all.flatten('F')

    shapeMU = model['shapeMU'][valid_ind, :]
    shapePC = model['shapePC'][valid_ind, :n_sp]
    expPC = model['expPC'][valid_ind, :n_ep]

    s = 4e-04
    R = mesh.transform.angle2matrix([0, 0, 0])
    t = [0, 0, 0]
    lsp = [];
    lep = [];
    ls = [];
    lR = [];
    lt = []
    for i in range(max_iter):
        X = shapeMU + shapePC.dot(sp) + expPC.dot(ep)
        X = np.reshape(X, [int(len(X) / 3), 3]).T
        lsp.append(sp);
        lep.append(ep);
        ls.append(s), lR.append(R), lt.append(t)

        # ----- estimate pose
        P = mesh.transform.estimate_affine_matrix_3d22d(X.T, x.T)
        s, R, t = mesh.transform.P2sRt(P)
        lsp.append(sp);
        lep.append(ep);
        ls.append(s), lR.append(R), lt.append(t)

        # ----- estimate shape
        # expression
        shape = shapePC.dot(sp)
        shape = np.reshape(shape, [int(len(shape) / 3), 3]).T
        ep = estimate_expression(x, shapeMU, expPC, model['expEV'][:n_ep, :], shape, s, R, t[:2], lamb=20)
        lsp.append(sp);
        lep.append(ep);
        ls.append(s), lR.append(R), lt.append(t)

        # shape
        expression = expPC.dot(ep)
        expression = np.reshape(expression, [int(len(expression) / 3), 3]).T
        sp = estimate_shape(x, shapeMU, shapePC, model['shapeEV'][:n_sp, :], expression, s, R, t[:2], lamb=40)

    # print('ls', ls)
    # print('lR', lR)
    return np.array(lsp), np.array(lep), np.array(ls), np.array(lR), np.array(lt)

```

## Source file: mesh/io.py

{
  "path": "mesh/io.py",
  "blob_sha": "4bb146012ecd8f78b7485153bafab00af092fe16",
  "sha256": "1204bdfb31fe07872633b22724c13f956e7f79c28fddad51a72d038e189e90db",
  "lines": 142,
  "bytes": 4938,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 6
  },
  "url": "https://github.com/Shade5/FaceFitting/blob/b8106ae90f3fc369c3ab092adae11f78dfcc6e2e/mesh/io.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def read_obj(obj_name):"
    },
    {
      "line": 20,
      "text": "def write_asc(path, vertices):"
    },
    {
      "line": 30,
      "text": "def write_obj_with_colors(obj_name, vertices, triangles, colors):"
    },
    {
      "line": 61,
      "text": "def write_obj_with_texture(obj_name, vertices, triangles, texture, uv_coords):"
    },
    {
      "line": 112,
      "text": "def write_obj_with_colors_texture(obj_name, vertices, triangles, colors, texture, uv_coords):"
    }
  ]
}

```text

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function

import numpy as np
import os
from skimage import io
from time import time

from .cython import mesh_core_cython

## TODO
## TODO: c++ version
def read_obj(obj_name):
	''' read mesh
	'''
	return 0

# ------------------------- write
def write_asc(path, vertices):
    '''
    Args:
        vertices: shape = (nver, 3)
    '''
    if path.split('.')[-1] == 'asc':
        np.savetxt(path, vertices)
    else:
        np.savetxt(path + '.asc', vertices)

def write_obj_with_colors(obj_name, vertices, triangles, colors):
    ''' Save 3D face model with texture represented by colors.
    Args:
        obj_name: str
        vertices: shape = (nver, 3)
        triangles: shape = (ntri, 3)
        colors: shape = (nver, 3)
    '''
    triangles = triangles.copy()
    triangles += 1 # meshlab start with 1
    
    if obj_name.split('.')[-1] != 'obj':
        obj_name = obj_name + '.obj'
        
    # write obj
    with open(obj_name, 'w') as f:
        
        # write vertices & colors
        for i in range(vertices.shape[0]):
            # s = 'v {} {} {} \n'.format(vertices[0,i], vertices[1,i], vertices[2,i])
            s = 'v {} {} {} {} {} {}\n'.format(vertices[i, 0], vertices[i, 1], vertices[i, 2], colors[i, 0], colors[i, 1], colors[i, 2])
            f.write(s)

        # write f: ver ind/ uv ind
        [k, ntri] = triangles.shape
        for i in range(triangles.shape[0]):
            # s = 'f {} {} {}\n'.format(triangles[i, 0], triangles[i, 1], triangles[i, 2])
            s = 'f {} {} {}\n'.format(triangles[i, 2], triangles[i, 1], triangles[i, 0])
            f.write(s)

## TODO: c++ version
def write_obj_with_texture(obj_name, vertices, triangles, texture, uv_coords):
    ''' Save 3D face model with texture represented by texture map.
    Ref: https://github.com/patrikhuber/eos/blob/bd00155ebae4b1a13b08bf5a991694d682abbada/include/eos/core/Mesh.hpp
    Args:
        obj_name: str
        vertices: shape = (nver, 3)
        triangles: shape = (ntri, 3)
        texture: shape = (256,256,3)
        uv_coords: shape = (nver, 3) max value<=1
    '''
    if obj_name.split('.')[-1] != 'obj':
        obj_name = obj_name + '.obj'
    mtl_name = obj_name.replace('.obj', '.mtl')
    texture_name = obj_name.replace('.obj', '_texture.png')
    
    triangles = triangles.copy()
    triangles += 1 # mesh lab start with 1
    
    # write obj
    with open(obj_name, 'w') as f:
        # first line: write mtlib(material library)
        s = "mtllib {}\n".format(os.path.abspath(mtl_name))
        f.write(s)

        # write vertices
        for i in range(vertices.shape[0]):
            s = 'v {} {} {}\n'.format(vertices[i, 0], vertices[i, 1], vertices[i, 2])
            f.write(s)
        
        # write uv coords
        for i in range(uv_coords.shape[0]):
            s = 'vt {} {}\n'.format(uv_coords[i,0], 1 - uv_coords[i,1])
            f.write(s)

        f.write("usemtl FaceTexture\n")

        # write f: ver ind/ uv ind
        for i in range(triangles.shape[0]):
            s = 'f {}/{} {}/{} {}/{}\n'.format(triangles[i,2], triangles[i,2], triangles[i,1], triangles[i,1], triangles[i,0], triangles[i,0])
            f.write(s)

    # write mtl
    with open(mtl_name, 'w') as f:
        f.write("newmtl FaceTexture\n")
        s = 'map_Kd {}\n'.format(os.path.abspath(texture_name)) # map to image
        f.write(s)

    # write texture as png
    imsave(texture_name, texture)

# c++ version
def write_obj_with_colors_texture(obj_name, vertices, triangles, colors, texture, uv_coords):
    ''' Save 3D face model with texture. 
    Ref: https://github.com/patrikhuber/eos/blob/bd00155ebae4b1a13b08bf5a991694d682abbada/include/eos/core/Mesh.hpp
    Args:
        obj_name: str
        vertices: shape = (nver, 3)
        triangles: shape = (ntri, 3)
        colors: shape = (nver, 3)
        texture: shape = (256,256,3)
        uv_coords: shape = (nver, 3) max value<=1
    '''
    if obj_name.split('.')[-1] != 'obj':
        obj_name = obj_name + '.obj'
    mtl_name = obj_name.replace('.obj', '.mtl')
    texture_name = obj_name.replace('.obj', '_texture.png')
    
    triangles = triangles.copy()
    triangles += 1 # mesh lab start with 1
    
    # write obj
    vertices, colors, uv_coords = vertices.astype(np.float32).copy(), colors.astype(np.float32).copy(), uv_coords.astype(np.float32).copy()
    mesh_core_cython.write_obj_with_colors_texture_core(str.encode(obj_name), str.encode(os.path.abspath(mtl_name)), vertices, triangles, colors, uv_coords, vertices.shape[0], triangles.shape[0], uv_coords.shape[0])
   
    # write mtl
    with open(mtl_name, 'w') as f:
        f.write("newmtl FaceTexture\n")
        s = 'map_Kd {}\n'.format(os.path.abspath(texture_name)) # map to image
        f.write(s)

    # write texture as png
    io.imsave(texture_name, texture)

```

## Source file: mesh/vis.py

{
  "path": "mesh/vis.py",
  "blob_sha": "9db972f2c463a310fa4abde16d6a7baf02797c90",
  "sha256": "44a3679936d9e1982c69a4e37cb09cafa6ad5dd950f026bc6ad22e9970c29d46",
  "lines": 24,
  "bytes": 791,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Shade5/FaceFitting/blob/b8106ae90f3fc369c3ab092adae11f78dfcc6e2e/mesh/vis.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def plot_mesh(vertices, triangles, subplot = [1,1,1], title = 'mesh', el = 90, az = -90, lwdt=.1, dist = 6, color = \"grey\"):"
    }
  ]
}

```text

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function

import numpy as np
import matplotlib.pyplot as plt
from skimage import measure
from mpl_toolkits.mplot3d import Axes3D

def plot_mesh(vertices, triangles, subplot = [1,1,1], title = 'mesh', el = 90, az = -90, lwdt=.1, dist = 6, color = "grey"):
	'''
	plot the mesh 
	Args:
		vertices: [nver, 3]
		triangles: [ntri, 3]
	'''
	ax = plt.subplot(subplot[0], subplot[1], subplot[2], projection = '3d')
	ax.plot_trisurf(vertices[:, 0], vertices[:, 1], vertices[:, 2], triangles = triangles, lw = lwdt, color = color, alpha = 1)
	ax.axis("off")
	ax.view_init(elev = el, azim = az)
	ax.dist = dist
	plt.title(title)

### -------------- Todo: use vtk to visualize mesh? or visvis? or VisPy?


```

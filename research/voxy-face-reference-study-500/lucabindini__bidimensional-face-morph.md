# Facial rendering source scan: lucabindini/bidimensional-face-morph

Repository: https://github.com/lucabindini/bidimensional-face-morph

Pinned source revision: fac5d0f94ee962059cf0b576ba7218b7e97c01e4

Discovery topic: face deformation

Description: A tool to measure facial deformation from 2D images

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "ad021c841c8c354beabc8c445e3072d5559c5ec7",
  "sha256": "1b559435fb5aa7f06fcd165d4a468b51e3dfa891da021b4a628a805dd165bd99",
  "lines": 99,
  "bytes": 3790,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 6
  },
  "url": "https://github.com/lucabindini/bidimensional-face-morph/blob/fac5d0f94ee962059cf0b576ba7218b7e97c01e4/main.py",
  "function_signature_candidates": []
}

```text

#!/usr/bin/python3

import os
import sys

import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

import fitting
import landmarks

FRAMES_PATH = sys.argv[1]
DEF_SHAPE = 'defShape'
OUTPUT_DIR_3D_FINAL = 'final_deformation_heatmaps'
OUTPUT_DIR_3D_INTERMEDIATE = 'intermediate_deformation_heatmaps'
OUTPUT_DIR_PLOT = 'temporal_deformation_plots'
N = 4  # number of frames analysed in each sequence
MEDIAN_ITER = 1  # number of iteration to get median
VMAX = 4  # saturation value for uniform 3d model scaling
LR = True  # set to True if using the LR dataset (due to additional initial frames issue)

for directory in {OUTPUT_DIR_3D_FINAL, OUTPUT_DIR_3D_INTERMEDIATE, landmarks.OUTPUT_DIR_2D, OUTPUT_DIR_PLOT}:
    if not os.access(directory, os.F_OK):
        os.mkdir(directory)

fig_all, ax_all = plt.subplots()
ax_all.set_xlabel('frame fraction')
ax_all.set_ylabel('deformation index')

for i in range(1, len(os.listdir(FRAMES_PATH)) + 1):

    path = f'{FRAMES_PATH}/{i:04}'
    start = int(
        os.path.splitext(min(os.listdir(path), key=lambda s: int(os.path.splitext(s)[0])))[0])
    if LR:
        start += i - 1
    def_shapes = []
    def_shapes0 = []
    for k in range(start, start + MEDIAN_ITER):
        img_path = f'{path}/{k}.jpg'
        preds0 = landmarks.lm_dir(img_path, True)
        def_shape00 = fitting.fit_3dmm(preds0)[DEF_SHAPE]
        def_shape0 = def_shape00.copy()
        def_shape00 = fitting.fit_3dmm(preds0, def_shape00)[DEF_SHAPE]
        def_shapes.append(def_shape00)
        def_shapes0.append(def_shape0)
    def_shapes_a = np.array(def_shapes)
    def_shape00 = np.median(def_shapes_a, axis=0)
    def_shapes0_a = np.array(def_shapes0)
    def_shape0 = np.median(def_shapes0_a, axis=0)

    y = [0]
    fig_intermediate = plt.figure(figsize=plt.figaspect(N))

    for j in range(1, N + 1):
        def_shapes = []
        end = int(os.path.splitext(max(os.listdir(path), key=lambda s: int(os.path.splitext(s)[0])))[0])
        n = start + int((end - start) * j / N)
        for k in range(n + 1 - MEDIAN_ITER, n + 1):
            img_path = f'{path}/{k}.jpg'
            preds1 = landmarks.lm_dir(img_path, True)
            def_shape01 = def_shape0.copy()
            def_shape01 = fitting.fit_3dmm(preds1, def_shape01)[DEF_SHAPE]
            def_shapes.append(def_shape01)
        def_shapes_a = np.array(def_shapes)
        def_shape_median = np.median(def_shapes_a, axis=0)

        def_indices = landmarks.models_deformation(def_shape00, def_shape_median)

        y.append(def_indices.mean())

        ax_intermediate = fig_intermediate.add_subplot(N, 1, j, projection='3d', elev=90, azim=-90)
        ax_intermediate.scatter3D(*def_shape_median.transpose(), s=1, c=def_indices, cmap='jet', vmin=0, vmax=VMAX)
        ax_intermediate.axis(False)
        ax_intermediate.set_title(f'{y[-1]:.2f}')

    fig_final = plt.figure()
    ax_final = fig_final.add_subplot(1, 1, 1, projection='3d', elev=90, azim=-90)
    ax_final.scatter3D(*def_shape_median.transpose(), s=1, c=def_indices, cmap='jet', vmin=0, vmax=VMAX)
    ax_final.axis(False)
    fig_final.savefig(f'{OUTPUT_DIR_3D_FINAL}/{i:04}.png', bbox_inches='tight', pad_inches=0)
    plt.close(fig_final)

    fig_intermediate.savefig(f'{OUTPUT_DIR_3D_INTERMEDIATE}/{i:04}.png', bbox_inches='tight', pad_inches=0)
    plt.close(fig_intermediate)

    fig, ax = plt.subplots()
    ax.plot(np.arange(len(y)) / N, y)
    if not LR or i > 1:
        ax_all.plot(np.arange(len(y)) / N, y, color='C0')
    ax.set_xlabel('frame fraction')
    ax.set_ylabel('deformation index')
    fig.savefig(f'{OUTPUT_DIR_PLOT}/{i:04}.png')
    plt.close(fig)

    print(f'{i} / {len(os.listdir(FRAMES_PATH))} expressions done')

fig_all.savefig(f'{OUTPUT_DIR_PLOT}/all.png')


```

## Source file: fitting.py

{
  "path": "fitting.py",
  "blob_sha": "4fc4d320eaf7a59d04a487d18d085ac47570876f",
  "sha256": "3afa6ebe7c3e7363c7a308fda6b4f5e1de9a620454e785a40aa7ab9bce026a72",
  "lines": 60,
  "bytes": 2007,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/lucabindini/bidimensional-face-morph/blob/fac5d0f94ee962059cf0b576ba7218b7e97c01e4/fitting.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def fit_3dmm(lm, avg_model=None):"
    }
  ]
}

```text

import numpy as np
import numpy.matlib as npm
import h5py

from deep_3dmm_refinement._3DMM import _3DMM
from SLC_3DMM import Matrix_operations


def fit_3dmm(lm, avg_model=None):
    # Fixed Params
    rounds = 1
    r = 3.8
    c_dist = 400

    lm = lm[:, :2]
    lm = np.delete(lm, 64, axis=0)
    lm = np.delete(lm, 60, axis=0)

    # Load 3D data
    avgfile = h5py.File('data3dmm/avgModel_bh_1779_NE_mediumBound.mat', 'r')
    idx_landmarks_3D = np.transpose(np.array(avgfile['idxLandmarks3D']))
    idx_landmarks_3D -= 1
    landmarks_3D = np.transpose(np.array(avgfile['landmarks3D']))

    if avg_model is None:
        lambda_ = 0.15
        componentsfile_path = 'data3dmm/components_DL_300_1779.mat'
    else:
        lambda_ = 64
        componentsfile_path = 'SLC_3DMM/data/SLC_300_1_1.mat'

    # Load 3DMM params and dictionary
    componentsfile = h5py.File(componentsfile_path, 'r')
    Components = np.transpose(np.array(componentsfile['Components']))
    Weights = np.array(componentsfile['Weights'])
    if avg_model is None:
        avg_model = np.transpose(np.array(avgfile['avgModel']))
        Weights = np.transpose(Weights)
        Components_res = np.transpose(np.array(componentsfile['Components_res']))
    else:
        aligned_models_data = None
        components_R = Matrix_operations.Matrix_op(Components, aligned_models_data)
        components_R.reshape(Components)
        Components_res = components_R.X_res
    print('Data Loaded.')

    # Center to zero
    baric_3dmm = np.mean(avg_model, axis=0)
    avg_model = avg_model - npm.repmat(baric_3dmm, avg_model.shape[0], 1)
    landmarks_3D = landmarks_3D - npm.repmat(baric_3dmm, landmarks_3D.shape[0], 1)

    # Create 3DMM object
    _3DMM_obj = _3DMM()

    # Fit the 3DMM
    result = _3DMM_obj.opt_3DMM_fast(Weights, Components, Components_res,
                                     landmarks_3D, idx_landmarks_3D, lm, avg_model, lambda_, rounds, r, c_dist)
    print('3DMM Fitting Completed')

    return result


```

## Source file: landmarks.py

{
  "path": "landmarks.py",
  "blob_sha": "26e43f7736141bae44089e66419373bf3ac77e43",
  "sha256": "4dcf633296ec7b17be44e4989c05e81dffec267275cc7584476969f540f6a8c0",
  "lines": 65,
  "bytes": 2166,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "teeth": 1,
    "deform": 1,
    "face": 3,
    "lip": 1
  },
  "url": "https://github.com/lucabindini/bidimensional-face-morph/blob/fac5d0f94ee962059cf0b576ba7218b7e97c01e4/landmarks.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def lm_dir(path, plot=False):"
    },
    {
      "line": 64,
      "text": "def models_deformation(shape1, shape2):"
    }
  ]
}

```text

import os
from collections import namedtuple

import matplotlib.pyplot as plt
import matplotlib.image as mpimg
import face_alignment

OUTPUT_DIR_2D = 'landmarks_2d'


def lm_dir(path, plot=False):
    fa = face_alignment.FaceAlignment(face_alignment.LandmarksType._2D, device='cpu')

    input_img = mpimg.imread(path)

    preds = fa.get_landmarks_from_image(input_img)[-1]

    if plot:
        # 2D-Plot
        pred_type = namedtuple('prediction_type', ['slice', 'color'])
        pred_types = {'face': pred_type(slice(0, 17), 'b'),
                      'eyebrow1': pred_type(slice(17, 22), 'g'),
                      'eyebrow2': pred_type(slice(22, 27), 'g'),
                      'nose': pred_type(slice(27, 31), 'r'),
                      'nostril': pred_type(slice(31, 36), 'c'),
                      'eye1': pred_type(slice(36, 42), 'm'),
                      'eye2': pred_type(slice(42, 48), 'm'),
                      'lips': pred_type(slice(48, 60), 'y'),
                      'teeth': pred_type(slice(60, 68), 'k')
                      }

        fig, ax = plt.subplots()
        ax.imshow(input_img)
        ax.axis(False)

        for pred_type in pred_types.values():
            ax.plot(preds[pred_type.slice, 0],
                    preds[pred_type.slice, 1],
                    color=pred_type.color,
                    marker='o',
                    markersize=1)

        fig.savefig(f'{OUTPUT_DIR_2D}/{os.path.basename(os.path.dirname(path))}_with_image.png', bbox_inches='tight',
                    pad_inches=0)
        plt.close(fig)

        fig, ax = plt.subplots()
        ax.axis(False)
        ax.set_aspect('equal')
        ax.invert_yaxis()

        for pred_type in pred_types.values():
            ax.plot(preds[pred_type.slice, 0],
                    preds[pred_type.slice, 1],
                    color=pred_type.color,
                    marker='o')

        fig.savefig(f'{OUTPUT_DIR_2D}/{os.path.basename(os.path.dirname(path))}.png', bbox_inches='tight', pad_inches=0)
        plt.close(fig)

    return preds


def models_deformation(shape1, shape2):
    return ((shape1 - shape2) ** 2).sum(axis=1) ** (1 / 2)


```

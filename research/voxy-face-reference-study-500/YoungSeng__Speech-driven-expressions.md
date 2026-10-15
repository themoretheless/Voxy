# Facial rendering source scan: YoungSeng/Speech-driven-expressions

Repository: https://github.com/YoungSeng/Speech-driven-expressions

Pinned source revision: 5e7c35aaf47f23bc183beec684af4ed95a5e1fd9

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["data_process/add_eye.py"], "face": ["data/训练集/audio2face_data_for_train/Readme.txt", "data/训练集/audio2face_text/Readme.txt"]}

## Source file: data_process/add_eye.py

{
  "path": "data_process/add_eye.py",
  "blob_sha": "a7158ed4ed57007ef29fe91d1149dc40e3da7804",
  "sha256": "caff8d8bbef1706b807c648466cd595e62aa1c400b2b9a92ca18ddf7cab4e20b",
  "lines": 76,
  "bytes": 3255,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 6,
    "jaw": 1,
    "mouth": 6,
    "rig": 7
  },
  "url": "https://github.com/YoungSeng/Speech-driven-expressions/blob/5e7c35aaf47f23bc183beec684af4ed95a5e1fd9/data_process/add_eye.py",
  "function_signature_candidates": []
}

```text

import os
import pdb
import csv
import pandas as pd
import numpy as np

root = '... your path/AIWIN/data/trn+val/'

bs_path = root + 'bs/'
dictionary = {}
for item in os.listdir(bs_path):
    if item in ['trn_b24.csv', 'trn_b23.csv', 'trn_b22.csv', 'trn_b21.csv', 'trn_b20.csv', 'trn_b19.csv']:
        print('continue')
        continue
    data = pd.read_csv(os.path.join(bs_path, item))
    data = np.array(data)[:, :14]
    if data.shape[0] not in dictionary:
        dictionary[data.shape[0]] = data

print(sorted(dictionary.keys()), len(dictionary))

'''
[86, 90, 92, 94, 95, 98, 99, 100, 103, 105, 106, 108, 109, 110, 111, 112, 114, 115, 116,
 118, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 133, 135, 136, 137, 13
8, 140, 141, 142, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 158, 159,
160, 161, 162, 163, 164, 165, 166, 167, 170, 171, 172, 173, 174, 175, 176, 177, 178, 180
, 181, 183, 184, 186, 188, 189, 190, 192, 193, 195, 196, 197, 198, 199, 200, 202, 203, 2
04, 205, 206, 207, 208, 210, 212, 214, 216, 219, 221, 223, 226, 227, 230, 231, 235, 239,
 241, 244, 246, 250, 251, 256, 260, 267, 268, 271, 279, 280, 291, 296, 310, 321, 341, 41
0]
'''

# 52减去眼睛、舌头blendshape
keys = ['JawForward', 'JawLeft', 'JawRight', 'JawOpen', 'MouthClose', 'MouthFunnel', 'MouthPucker', 'MouthLeft',
        'MouthRight', 'MouthSmileLeft', 'MouthSmileRight', 'MouthFrownLeft', 'MouthFrownRight', 'MouthDimpleLeft',
        'MouthDimpleRight', 'MouthStretchLeft', 'MouthStretchRight', 'MouthRollLower', 'MouthRollUpper',
        'MouthShrugLower', 'MouthShrugUpper', 'MouthPressLeft', 'MouthPressRight', 'MouthLowerDownLeft',
        'MouthLowerDownRight', 'MouthUpperUpLeft',
        'MouthUpperUpRight', 'BrowDownLeft', 'BrowDownRight', 'BrowInnerUp', 'BrowOuterUpLeft', 'BrowOuterUpRight',
        'CheekPuff', 'CheekSquintLeft', 'CheekSquintRight', 'NoseSneerLeft', 'NoseSneerRight']

tst_B_bs = "... your path/AIWIN/result/output/infer_sample/my_tst_A_post/"
output_bs = "... your path/AIWIN/result/output/infer_sample/my_tst_A_post_addeye/"

os.makedirs(output_bs, exist_ok=True)

blendshape_example = []
blendshape_example_path = "... your path/AIWIN/data/训练集/arkit.csv"
with open(blendshape_example_path, "r") as f:
    reader = csv.reader(f)
    for row in reader:
        blendshape_example = row
        break

for item in os.listdir(tst_B_bs):
    data = pd.read_csv(os.path.join(tst_B_bs, item), usecols=keys)
    data = np.array(data)
    assert data.shape[1] == 37
    if data.shape[0] in dictionary:
        data = np.concatenate((dictionary[data.shape[0]], data, np.zeros((data.shape[0], 1))), axis=1)
    else:
        i = 1
        while True:
            frames = data.shape[0] + i
            if frames in dictionary:
                break
            i += 1
        data = np.concatenate((dictionary[frames][:data.shape[0], :], data, np.zeros((data.shape[0], 1))), axis=1)
    assert data.shape[1] == 52
    out_bvh_path = os.path.join(output_bs, item)
    with open(out_bvh_path, "w") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow(blendshape_example)
        writer.writerows(data)




```

## Source file: blender.py

{
  "path": "blender.py",
  "blob_sha": "5c4a3a19b51ef3245bd6737860b6183a3a0a2207",
  "sha256": "c2834497837cdaaffcb1f3c4bc8350a68ca9a8390a0582e8782168179fe8c900",
  "lines": 69,
  "bytes": 2357,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "blendshape": 2,
    "rig": 1
  },
  "url": "https://github.com/YoungSeng/Speech-driven-expressions/blob/5e7c35aaf47f23bc183beec684af4ed95a5e1fd9/blender.py",
  "function_signature_candidates": []
}

```text

import bpy
import os
import math
import csv
#import pandas as pd

## Step 1
### 清除场景中多余的组件.
#bpy.data.objects.remove(bpy.data.objects['Camera'])
##bpy.data.objects.remove(bpy.data.objects['Cube'])
##bpy.data.objects.remove(bpy.data.objects['Light'])
#fbx_path = r"D:\Downloads\new\POLYWINK_AIX_52_SAMPLE\POLYWINK_AIX_SAMPLE\POLYWINK_AIX_SAMPLE.fbx"
#bpy.ops.import_scene.fbx(filepath=fbx_path)

# Step 2
output_dir = r'D:\Downloads\new\POLYWINK_AIX_52_SAMPLE\POLYWINK_AIX_SAMPLE\result_'

obj = bpy.context.active_object
#shape_key = obj.data.shape_keys.key_blocks["eyeBlinkLeft"]
#keys = obj.data.shape_keys.key_blocks.keys()
#shape_key_index = keys.index(shape_key.name)
#print(shape_key, keys, shape_key_index)
#print(bpy.data.objects["POLYWINK_Bella"].data)

obj.data.shape_keys.key_blocks["eyeBlinkLeft"].value=1.0
print(obj.data.shape_keys.key_blocks["eyeBlinkLeft"].value)
#print(obj.data.shape_keys.key_blocks["eyeBlinkRight"])


bpy.context.scene.render.engine = 'BLENDER_WORKBENCH'
bpy.context.scene.display.shading.light = 'MATCAP'
bpy.context.scene.display.render_aa = 'FXAA'
bpy.context.scene.render.resolution_x=int(1024)
bpy.context.scene.render.resolution_y=int(768)
bpy.context.scene.render.fps = 25
bpy.context.scene.render.image_settings.file_format='PNG'
bpy.context.scene.render.filepath=os.path.join(output_dir, '{}.png'.format('Test'))

## Camera
bpy.ops.object.camera_add(enter_editmode=False, location=[0, -1, 1.1], rotation=[math.radians(90), 0, 0])
cam = bpy.data.objects['Camera']
cam.scale = [2, 2, 2]
bpy.context.scene.camera = cam # add cam so it's rendered

bpy.ops.render.render(write_still=True)


#blendshape_example_path = r"D:\Code\AIWIN\TEST\a10.csv"
#result = []
#with open(blendshape_example_path, "r") as f:
#    reader = csv.reader(f)
#    for row in reader:
#        result.append(row)
#head = [str.lower(item[0]) + item[1:] for item in result[0]]
#print(head)
#result = result[1:]

#print(head, len(result))


#for i in range(len(result)):
#    for j in range(52):
#        obj.data.shape_keys.key_blocks[head[j]].value = eval(result[i][j])
#    bpy.context.scene.render.filepath=os.path.join(output_dir, '{}.png'.format(i))
#    bpy.ops.render.render(write_still=True)

# Step 3
# Blender将图片序列+声音合成为视频
# https://jingyan.baidu.com/article/375c8e198d4a1464f2a2299c.html


```

## Source file: evaluate.py

{
  "path": "evaluate.py",
  "blob_sha": "8c82b46c63ab5b5406414f4460c20ecc4909ffbb",
  "sha256": "7891f09d06a628f5e0942371fd7468164e175b5a7d879102ec9d9b50aaa6c676",
  "lines": 82,
  "bytes": 5109,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 5,
    "blendshape": 3,
    "jaw": 3,
    "mouth": 12,
    "gum": 2,
    "tongue": 2,
    "rig": 16
  },
  "url": "https://github.com/YoungSeng/Speech-driven-expressions/blob/5e7c35aaf47f23bc183beec684af4ed95a5e1fd9/evaluate.py",
  "function_signature_candidates": []
}

```text

import csv
import os
import pdb

import numpy as np
# from fastdtw import fastdtw
import torch
from myfastdtw import myfastdtw
from sklearn.metrics import mean_squared_error

# 61点blendshape
# keys = ['EyeBlinkLeft', 'EyeLookDownLeft', 'EyeLookInLeft', 'EyeLookOutLeft', 'EyeLookUpLeft', 'EyeSquintLeft', 'EyeWideLeft', 'EyeBlinkRight', 'EyeLookDownRight', 'EyeLookInRight', 'EyeLookOutRight', 'EyeLookUpRight', 'EyeSquintRight',
#         'EyeWideRight', 'JawForward', 'JawRight', 'JawLeft', 'JawOpen', 'MouthClose', 'MouthFunnel', 'MouthPucker', 'MouthRight', 'MouthLeft', 'MouthSmileLeft', 'MouthSmileRight', 'MouthFrownLeft', 'MouthFrownRight', 'MouthDimpleLeft',
#         'MouthDimpleRight', 'MouthStretchLeft', 'MouthStretchRight', 'MouthRollLower', 'MouthRollUpper', 'MouthShrugLower', 'MouthShrugUpper', 'MouthPressLeft', 'MouthPressRight', 'MouthLowerDownLeft', 'MouthLowerDownRight', 'MouthUpperUpLeft',
#         'MouthUpperUpRight', 'BrowDownLeft', 'BrowDownRight', 'BrowInnerUp', 'BrowOuterUpLeft', 'BrowOuterUpRight', 'CheekPuff', 'CheekSquintLeft', 'CheekSquintRight', 'NoseSneerLeft', 'NoseSneerRight', 'TongueOut', 'HeadYaw', 'HeadPitch',
#         'HeadRoll', 'LeftEyeYaw', 'LeftEyePitch', 'LeftEyeRoll', 'RightEyeYaw', 'RightEyePitch', 'RightEyeRoll']

# 52点blendshape
# keys = ['EyeBlinkLeft', 'EyeLookDownLeft', 'EyeLookInLeft', 'EyeLookOutLeft', 'EyeLookUpLeft', 'EyeSquintLeft', 'EyeWideLeft', 'EyeBlinkRight', 'EyeLookDownRight', 'EyeLookInRight', 'EyeLookOutRight', 'EyeLookUpRight', 'EyeSquintRight',
#         'EyeWideRight', 'JawForward', 'JawLeft', 'JawRight', 'JawOpen', 'MouthClose', 'MouthFunnel', 'MouthPucker', 'MouthLeft', 'MouthRight', 'MouthSmileLeft', 'MouthSmileRight', 'MouthFrownLeft', 'MouthFrownRight', 'MouthDimpleLeft',
#         'MouthDimpleRight', 'MouthStretchLeft', 'MouthStretchRight', 'MouthRollLower', 'MouthRollUpper', 'MouthShrugLower', 'MouthShrugUpper', 'MouthPressLeft', 'MouthPressRight', 'MouthLowerDownLeft', 'MouthLowerDownRight', 'MouthUpperUpLeft',
#         'MouthUpperUpRight', 'BrowDownLeft', 'BrowDownRight', 'BrowInnerUp', 'BrowOuterUpLeft', 'BrowOuterUpRight', 'CheekPuff', 'CheekSquintLeft', 'CheekSquintRight', 'NoseSneerLeft', 'NoseSneerRight', 'TongueOut']

# 52减去眼睛、舌头blendshape
keys = ['JawForward', 'JawLeft', 'JawRight', 'JawOpen', 'MouthClose', 'MouthFunnel', 'MouthPucker', 'MouthLeft',
        'MouthRight', 'MouthSmileLeft', 'MouthSmileRight', 'MouthFrownLeft', 'MouthFrownRight', 'MouthDimpleLeft',
        'MouthDimpleRight', 'MouthStretchLeft', 'MouthStretchRight', 'MouthRollLower', 'MouthRollUpper',
        'MouthShrugLower', 'MouthShrugUpper', 'MouthPressLeft', 'MouthPressRight', 'MouthLowerDownLeft',
        'MouthLowerDownRight', 'MouthUpperUpLeft',
        'MouthUpperUpRight', 'BrowDownLeft', 'BrowDownRight', 'BrowInnerUp', 'BrowOuterUpLeft', 'BrowOuterUpRight',
        'CheekPuff', 'CheekSquintLeft', 'CheekSquintRight', 'NoseSneerLeft', 'NoseSneerRight']

'''
python evaluate.py --path "... your path/AIWIN/result/output/infer_sample/output_myfastdtw_batchfist_interpolate_normalize_dropout_data_decoder_val3_5_4_fix/multimodal_context_checkpoint_189/"
python evaluate.py --path "... your path/AIWIN/result/output/infer_sample/"
'''

import argparse
parser = argparse.ArgumentParser(description='Process some integers.')
parser.add_argument('--path', type=str)
args = parser.parse_args()

# 读取预测文件和参考答案文件
# root = "... your path/AIWIN/result/output/infer_sample/output_myfastdtw_batchfist_interpolate_normalize_dropout_data_decoder_val3_5_1/multimodal_context_checkpoint_226/"
root = args.path
# predict_dir = root + "val/"
# predict_dir = root + "val_post2/"
predict_dir = root + "my_val_post/"

truth_dir = '... your path/AIWIN/data/val/bs/'

n = 0
score = 0
with open(root + 'score.txt', 'w') as f_result:
    for prediction in os.listdir(predict_dir):
        if prediction[0] == '.':continue
        y_pred = []
        y_true = []

        with open(predict_dir + prediction, 'r') as csv_pred:
            reader_pred = csv.DictReader(csv_pred)
            for row in reader_pred:
                r_list = []
                for key in keys:
                    r_list.append(float(row[key]))
                y_pred.append(r_list)

        with open(truth_dir + prediction, 'r') as csv_gt:
            reader_gt = csv.DictReader(csv_gt)
            for row in reader_gt:
                r_list = []
                for key in keys:
                    r_list.append(float(row[key]))
                y_true.append(r_list)
        n += 1
        # distance, _ = fastdtw(np.asarray(y_true), np.asarray(y_pred), dist=mean_squared_error)
        distance= myfastdtw(torch.as_tensor(np.asarray(y_true)).unsqueeze(0), torch.as_tensor(np.asarray(y_pred)).unsqueeze(0)).item()
        score += distance
        print(prediction, ':', distance)
        f_result.write(prediction + ': ' + str(distance) + '\n')
    print('score:', score / n)
    f_result.write('score: ' + str(score / n))


```

## Source file: myfastdtw.py

{
  "path": "myfastdtw.py",
  "blob_sha": "117fdfe7fff6ba5217a3b20f5f395356e3858df0",
  "sha256": "35beb96d28ecb1e1aedba0773550931954f00600c9b5fe0e0626862af3a1a5d9",
  "lines": 294,
  "bytes": 9209,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/YoungSeng/Speech-driven-expressions/blob/5e7c35aaf47f23bc183beec684af4ed95a5e1fd9/myfastdtw.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def fastdtw(x, y, radius=1, dist=None):"
    },
    {
      "line": 59,
      "text": "def __difference(a, b):"
    },
    {
      "line": 63,
      "text": "def __norm(p):"
    },
    {
      "line": 67,
      "text": "def __fastdtw(x, y, radius, dist):"
    },
    {
      "line": 81,
      "text": "def __prep_inputs(x, y, dist):"
    },
    {
      "line": 106,
      "text": "def dtw(x, y, dist=None):"
    },
    {
      "line": 141,
      "text": "def __dtw(x, y, window, dist):"
    },
    {
      "line": 161,
      "text": "def __reduce_by_half(x):"
    },
    {
      "line": 165,
      "text": "def __expand_window(path, len_x, len_y, radius):"
    },
    {
      "line": 195,
      "text": "def mymean_squared_error("
    },
    {
      "line": 261,
      "text": "def myfastdtw(x, y, dist=mymean_squared_error):"
    }
  ]
}

```text

#!/usr/bin/env python
# -*- coding: utf-8 -*-

from __future__ import absolute_import, division
import numbers
import pdb

import torch
import numpy as np
from collections import defaultdict

try:
    range = xrange
except NameError:
    pass


def fastdtw(x, y, radius=1, dist=None):
    ''' return the approximate distance between 2 time series with O(N)
        time and memory complexity

        Parameters
        ----------
        x : array_like
            input array 1
        y : array_like
            input array 2
        radius : int
            size of neighborhood when expanding the path. A higher value will
            increase the accuracy of the calculation but also increase time
            and memory consumption. A radius equal to the size of x and y will
            yield an exact dynamic time warping calculation.
        dist : function or int
            The method for calculating the distance between x[i] and y[j]. If
            dist is an int of value p > 0, then the p-norm will be used. If
            dist is a function then dist(x[i], y[j]) will be used. If dist is
            None then abs(x[i] - y[j]) will be used.

        Returns
        -------
        distance : float
            the approximate distance between the 2 time series
        path : list
            list of indexes for the inputs x and y

        Examples
        --------
        >>> import numpy as np
        >>> import fastdtw
        >>> x = np.array([1, 2, 3, 4, 5], dtype='float')
        >>> y = np.array([2, 3, 4], dtype='float')
        >>> fastdtw.fastdtw(x, y)
        (2.0, [(0, 0), (1, 0), (2, 1), (3, 2), (4, 2)])
    '''
    x, y, dist = __prep_inputs(x, y, dist)
    return __fastdtw(x, y, radius, dist)


def __difference(a, b):
    return abs(a - b)


def __norm(p):
    return lambda a, b: torch.linalg.norm(torch.atleast_1d(a) - torch.atleast_1d(b), p)


def __fastdtw(x, y, radius, dist):
    min_time_size = radius + 2

    if len(x) < min_time_size or len(y) < min_time_size:
        return dtw(x, y, dist=dist)

    x_shrinked = __reduce_by_half(x)
    y_shrinked = __reduce_by_half(y)
    distance, path = \
        __fastdtw(x_shrinked, y_shrinked, radius=radius, dist=dist)
    window = __expand_window(path, len(x), len(y), radius)
    return __dtw(x, y, window, dist=dist)


def __prep_inputs(x, y, dist):
    # x = np.asanyarray(x, dtype='float')
    # y = np.asanyarray(y, dtype='float')

    if type(x) == list:
        x = torch.stack(x)
    if type(y) == list:
        y = torch.stack(y)

    if x.ndim == y.ndim > 1 and x.shape[1] != y.shape[1]:
        raise ValueError('second dimension of x and y must be the same')
    if isinstance(dist, numbers.Number) and dist <= 0:
        raise ValueError('dist cannot be a negative integer')

    if dist is None:
        if x.ndim == 1:
            dist = __difference
        else:
            dist = __norm(p=1)
    elif isinstance(dist, numbers.Number):
        dist = __norm(p=dist)

    return x, y, dist


def dtw(x, y, dist=None):
    ''' return the distance between 2 time series without approximation

        Parameters
        ----------
        x : array_like
            input array 1
        y : array_like
            input array 2
        dist : function or int
            The method for calculating the distance between x[i] and y[j]. If
            dist is an int of value p > 0, then the p-norm will be used. If
            dist is a function then dist(x[i], y[j]) will be used. If dist is
            None then abs(x[i] - y[j]) will be used.

        Returns
        -------
        distance : float
            the approximate distance between the 2 time series
        path : list
            list of indexes for the inputs x and y

        Examples
        --------
        >>> import numpy as np
        >>> import fastdtw
        >>> x = np.array([1, 2, 3, 4, 5], dtype='float')
        >>> y = np.array([2, 3, 4], dtype='float')
        >>> fastdtw.dtw(x, y)
        (2.0, [(0, 0), (1, 0), (2, 1), (3, 2), (4, 2)])
    '''
    x, y, dist = __prep_inputs(x, y, dist)
    return __dtw(x, y, None, dist)


def __dtw(x, y, window, dist):
    len_x, len_y = len(x), len(y)
    if window is None:
        window = [(i, j) for i in range(len_x) for j in range(len_y)]
    window = ((i + 1, j + 1) for i, j in window)
    D = defaultdict(lambda: (float('inf'),))
    D[0, 0] = (0, 0, 0)
    for i, j in window:
        dt = dist(x[i - 1], y[j - 1])
        D[i, j] = min((D[i - 1, j][0] + dt, i - 1, j), (D[i, j - 1][0] + dt, i, j - 1),
                      (D[i - 1, j - 1][0] + dt, i - 1, j - 1), key=lambda a: a[0])
    path = []
    i, j = len_x, len_y
    while not (i == j == 0):
        path.append((i - 1, j - 1))
        i, j = D[i, j][1], D[i, j][2]
    path.reverse()
    return (D[len_x, len_y][0], path)


def __reduce_by_half(x):
    return [(x[i] + x[1 + i]) / 2 for i in range(0, len(x) - len(x) % 2, 2)]


def __expand_window(path, len_x, len_y, radius):
    path_ = set(path)
    for i, j in path:
        for a, b in ((i + a, j + b)
                     for a in range(-radius, radius + 1)
                     for b in range(-radius, radius + 1)):
            path_.add((a, b))

    window_ = set()
    for i, j in path_:
        for a, b in ((i * 2, j * 2), (i * 2, j * 2 + 1),
                     (i * 2 + 1, j * 2), (i * 2 + 1, j * 2 + 1)):
            window_.add((a, b))

    window = []
    start_j = 0
    for i in range(0, len_x):
        new_start_j = None
        for j in range(start_j, len_y):
            if (i, j) in window_:
                window.append((i, j))
                if new_start_j is None:
                    new_start_j = j
            elif new_start_j is not None:
                break
        start_j = new_start_j

    return window


def mymean_squared_error(
        y_true, y_pred, *, sample_weight=None, multioutput="uniform_average", squared=True
):
    """Mean squared error regression loss.

    Read more in the :ref:`User Guide <mean_squared_error>`.

    Parameters
    ----------
    y_true : array-like of shape (n_samples,) or (n_samples, n_outputs)
        Ground truth (correct) target values.

    y_pred : array-like of shape (n_samples,) or (n_samples, n_outputs)
        Estimated target values.

    sample_weight : array-like of shape (n_samples,), default=None
        Sample weights.

    multioutput : {'raw_values', 'uniform_average'} or array-like of shape \
            (n_outputs,), default='uniform_average'
        Defines aggregating of multiple output values.
        Array-like value defines weights used to average errors.

        'raw_values' :
            Returns a full set of errors in case of multioutput input.

        'uniform_average' :
            Errors of all outputs are averaged with uniform weight.

    squared : bool, default=True
        If True returns MSE value, if False returns RMSE value.

    Returns
    -------
    loss : float or ndarray of floats
        A non-negative floating point value (the best value is 0.0), or an
        array of floating point values, one for each individual target.

    Examples
    --------
    >>> from sklearn.metrics import mean_squared_error
    >>> y_true = [3, -0.5, 2, 7]
    >>> y_pred = [2.5, 0.0, 2, 8]
    >>> mean_squared_error(y_true, y_pred)
    0.375
    >>> y_true = [3, -0.5, 2, 7]
    >>> y_pred = [2.5, 0.0, 2, 8]
    >>> mean_squared_error(y_true, y_pred, squared=False)
    0.612...
    >>> y_true = [[0.5, 1],[-1, 1],[7, -6]]
    >>> y_pred = [[0, 2],[-1, 2],[8, -5]]
    >>> mean_squared_error(y_true, y_pred)
    0.708...
    >>> mean_squared_error(y_true, y_pred, squared=False)
    0.822...
    >>> mean_squared_error(y_true, y_pred, multioutput='raw_values')
    array([0.41666667, 1.        ])
    >>> mean_squared_error(y_true, y_pred, multioutput=[0.3, 0.7])
    0.825...
    """

    output_errors = torch.mean((y_true - y_pred) ** 2, axis=0)

    return torch.mean(output_errors)


def myfastdtw(x, y, dist=mymean_squared_error):
    assert x.ndim == 3
    assert y.ndim == 3
    batch_size = x.size()[0]
    # print(batch_size)
    loss = 0
    for i in range(batch_size):
        distance, _ = fastdtw(x[i, :, :], y[i, :, :], dist=dist)
        loss += distance
    return loss / batch_size


if __name__ == '__main__':
    '''
    import torch
    x = torch.rand((2,3), requires_grad=True)
    y = torch.rand(2,3)
    # dist = lambda x,y:torch.sqrt(torch.sum((x-y)**2, axis=1))
    distance, _ = fastdtw(x, y, dist=mymean_squared_error)
    print(distance)
    pdb.set_trace()
    '''

    x = torch.rand((128, 55, 37), requires_grad=True)
    y = torch.rand(128, 55, 37)
    loss = myfastdtw(x, y)
    pdb.set_trace()
    loss.backward()

    # x = torch.rand((4, 5), requires_grad=True)
    # y = torch.rand(4, 5)
    # loss, _ = fastdtw(x, y, dist=mymean_squared_error)
    # pdb.set_trace()
    # loss.backward()


```

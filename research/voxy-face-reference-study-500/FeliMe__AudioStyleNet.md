# Facial rendering source scan: FeliMe/AudioStyleNet

Repository: https://github.com/FeliMe/AudioStyleNet

Pinned source revision: 070e7214ec73f5d81c77d367eb656469d1a1ad84

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["align_face.py", "utils/compute_face_mask.py", "utils/download_faceforensics.py"], "lip": ["utils/lipnet.py"]}

## Source file: align_face.py

{
  "path": "align_face.py",
  "blob_sha": "562abb603feef1febf452c4b379ac1788470db85",
  "sha256": "1c897a6fcd7324eb890366ddd5a0a69fdf07ff2132acdeee9b40d537433378f1",
  "lines": 54,
  "bytes": 1531,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 6
  },
  "url": "https://github.com/FeliMe/AudioStyleNet/blob/070e7214ec73f5d81c77d367eb656469d1a1ad84/align_face.py",
  "function_signature_candidates": []
}

```text

import argparse
import os

from glob import glob
from utils.utils import VideoAligner


# Parse arguments
parser = argparse.ArgumentParser()
parser.add_argument('--files', type=str)
parser.add_argument('--out_dir', type=str)
parser.add_argument('--filetype', type=str, choices=['image', 'video'], default='image')
parser.add_argument('--gpu', type=int, default=0)
args = parser.parse_args()


# Load target image
path = args.files
if os.path.isdir(path):
    if args.filetype == 'image':
        files = glob(path + '*.png')
        files += glob(path + '*.jpg')
    elif args.filetype == 'video':
        files = glob(path + '*.mp4')
        files += glob(path + '*.avi')
    else:
        raise NotImplementedError
else:
    files = [path]

# Select filetype
ext = files[0].split('.')[-1]
print(ext)
if ext.lower() in ['avi', 'mp4', 'flv', 'mpg']:
    filetype = 'video'
elif ext.lower() in ['jpg', 'jpeg', 'png']:
    filetype = 'image'
else:
    print("Unknown file type")
    raise NotImplementedError

aligner = VideoAligner(device=f'cuda:{args.gpu}')
if not os.path.exists(args.out_dir):
    os.makedirs(args.outdir, exist_ok=True)

for file in files:
    if filetype == 'image':
        save_path = args.out_dir + file.split('/')[-1].split('.')[0] + '.png'
        print(f"Saving to {save_path}")
        aligner.align_single_image(file, save_path)
    else:
        save_dir = os.path.join(args.out_dir, file.split('/')[-1].split('.')[0])
        print(f"Saving to {save_dir}")
        aligner.align_video(file, save_dir)


```

## Source file: utils/lipnet.py

{
  "path": "utils/lipnet.py",
  "blob_sha": "8ac3454711d9542529590cee6365e8d3af05dfae",
  "sha256": "545cc5d58aca665f01ff14ae4b6a662add3304454702392183cf2dc4d439b17f",
  "lines": 228,
  "bytes": 7543,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 3,
    "lip": 6
  },
  "url": "https://github.com/FeliMe/AudioStyleNet/blob/070e7214ec73f5d81c77d367eb656469d1a1ad84/utils/lipnet.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self, dropout_p=0.5):"
    },
    {
      "line": 36,
      "text": "def _init(self):"
    },
    {
      "line": 62,
      "text": "def forward(self, x):"
    },
    {
      "line": 97,
      "text": "def get_position(size, padding=0.25):"
    },
    {
      "line": 124,
      "text": "def transformation_from_points(points1, points2):"
    },
    {
      "line": 144,
      "text": "def prepare_video(array, device, verbose=False):"
    },
    {
      "line": 178,
      "text": "def ctc_arr2txt(arr, start):"
    },
    {
      "line": 193,
      "text": "def decode_sentence(y):"
    },
    {
      "line": 198,
      "text": "def lipnet_predict(video, model):"
    },
    {
      "line": 207,
      "text": "def read_transcript(file):"
    },
    {
      "line": 213,
      "text": "def get_model(device):"
    }
  ]
}

```text

import cv2
import face_alignment
import numpy as np
import os
import torch
import torch.nn as nn
import torch.nn.init as init
import math

RAIDROOT = os.environ.get('RAIDROOT')


class LipNet(torch.nn.Module):
    def __init__(self, dropout_p=0.5):
        super(LipNet, self).__init__()
        self.conv1 = nn.Conv3d(3, 32, (3, 5, 5), (1, 2, 2), (1, 2, 2))
        self.pool1 = nn.MaxPool3d((1, 2, 2), (1, 2, 2))

        self.conv2 = nn.Conv3d(32, 64, (3, 5, 5), (1, 1, 1), (1, 2, 2))
        self.pool2 = nn.MaxPool3d((1, 2, 2), (1, 2, 2))

        self.conv3 = nn.Conv3d(64, 96, (3, 3, 3), (1, 1, 1), (1, 1, 1))
        self.pool3 = nn.MaxPool3d((1, 2, 2), (1, 2, 2))

        self.gru1 = nn.GRU(96 * 4 * 8, 256, 1, bidirectional=True)
        self.gru2 = nn.GRU(512, 256, 1, bidirectional=True)

        self.FC = nn.Linear(512, 27 + 1)
        self.dropout_p = dropout_p

        self.relu = nn.ReLU(inplace=True)
        self.dropout = nn.Dropout(self.dropout_p)
        self.dropout3d = nn.Dropout3d(self.dropout_p)
        self._init()

    def _init(self):

        init.kaiming_normal_(self.conv1.weight, nonlinearity='relu')
        init.constant_(self.conv1.bias, 0)

        init.kaiming_normal_(self.conv2.weight, nonlinearity='relu')
        init.constant_(self.conv2.bias, 0)

        init.kaiming_normal_(self.conv3.weight, nonlinearity='relu')
        init.constant_(self.conv3.bias, 0)

        init.kaiming_normal_(self.FC.weight, nonlinearity='sigmoid')
        init.constant_(self.FC.bias, 0)

        for m in (self.gru1, self.gru2):
            stdv = math.sqrt(2 / (96 * 3 * 6 + 256))
            for i in range(0, 256 * 3, 256):
                init.uniform_(m.weight_ih_l0[i: i + 256],
                              -math.sqrt(3) * stdv, math.sqrt(3) * stdv)
                init.orthogonal_(m.weight_hh_l0[i: i + 256])
                init.constant_(m.bias_ih_l0[i: i + 256], 0)
                init.uniform_(m.weight_ih_l0_reverse[i: i + 256],
                              -math.sqrt(3) * stdv, math.sqrt(3) * stdv)
                init.orthogonal_(m.weight_hh_l0_reverse[i: i + 256])
                init.constant_(m.bias_ih_l0_reverse[i: i + 256], 0)

    def forward(self, x):

        x = self.conv1(x)
        x = self.relu(x)
        x = self.dropout3d(x)
        x = self.pool1(x)

        x = self.conv2(x)
        x = self.relu(x)
        x = self.dropout3d(x)
        x = self.pool2(x)

        x = self.conv3(x)
        x = self.relu(x)
        x = self.dropout3d(x)
        x = self.pool3(x)

        # (B, C, T, H, W)->(T, B, C, H, W)
        x = x.permute(2, 0, 1, 3, 4).contiguous()
        # (B, C, T, H, W)->(T, B, C*H*W)
        x = x.view(x.size(0), x.size(1), -1)

        self.gru1.flatten_parameters()
        self.gru2.flatten_parameters()

        x, h = self.gru1(x)
        x = self.dropout(x)
        x, h = self.gru2(x)
        x = self.dropout(x)

        x = self.FC(x)
        x = x.permute(1, 0, 2).contiguous()
        return x


def get_position(size, padding=0.25):

    x = [0.000213256, 0.0752622, 0.18113, 0.29077, 0.393397, 0.586856, 0.689483, 0.799124,
         0.904991, 0.98004, 0.490127, 0.490127, 0.490127, 0.490127, 0.36688, 0.426036,
         0.490127, 0.554217, 0.613373, 0.121737, 0.187122, 0.265825, 0.334606, 0.260918,
         0.182743, 0.645647, 0.714428, 0.793132, 0.858516, 0.79751, 0.719335, 0.254149,
         0.340985, 0.428858, 0.490127, 0.551395, 0.639268, 0.726104, 0.642159, 0.556721,
         0.490127, 0.423532, 0.338094, 0.290379, 0.428096, 0.490127, 0.552157, 0.689874,
         0.553364, 0.490127, 0.42689]

    y = [0.106454, 0.038915, 0.0187482, 0.0344891, 0.0773906, 0.0773906, 0.0344891,
         0.0187482, 0.038915, 0.106454, 0.203352, 0.307009, 0.409805, 0.515625, 0.587326,
         0.609345, 0.628106, 0.609345, 0.587326, 0.216423, 0.178758, 0.179852, 0.231733,
         0.245099, 0.244077, 0.231733, 0.179852, 0.178758, 0.216423, 0.244077, 0.245099,
         0.780233, 0.745405, 0.727388, 0.742578, 0.727388, 0.745405, 0.780233, 0.864805,
         0.902192, 0.909281, 0.902192, 0.864805, 0.784792, 0.778746, 0.785343, 0.778746,
         0.784792, 0.824182, 0.831803, 0.824182]

    x, y = np.array(x), np.array(y)

    x = (x + padding) / (2 * padding + 1)
    y = (y + padding) / (2 * padding + 1)
    x = x * size
    y = y * size
    return np.array(list(zip(x, y)))


def transformation_from_points(points1, points2):
    points1 = points1.astype(np.float64)
    points2 = points2.astype(np.float64)

    c1 = np.mean(points1, axis=0)
    c2 = np.mean(points2, axis=0)
    points1 -= c1
    points2 -= c2
    s1 = np.std(points1)
    s2 = np.std(points2)
    points1 /= s1
    points2 /= s2

    U, S, Vt = np.linalg.svd(points1.T * points2)
    R = (U * Vt).T
    return np.vstack([np.hstack(((s2 / s1) * R,
                                 c2.T - (s2 / s1) * R * c1.T)),
                      np.matrix([0., 0., 1.])])


def prepare_video(array, device, verbose=False):
    fa = face_alignment.FaceAlignment(
        face_alignment.LandmarksType._2D, flip_input=False, device=device)
    points = [fa.get_landmarks(I) for I in array]

    front256 = get_position(256)
    video = []
    for point, scene in zip(points, array):
        if point is not None:
            shape = np.array(point[0])
            shape = shape[17:]
            M = transformation_from_points(
                np.matrix(shape), np.matrix(front256))

            img = cv2.warpAffine(scene, M[:2], (256, 256))
            (x, y) = front256[-20:].mean(0).astype(np.int32)
            w = 160 // 2
            img = img[y - w // 2:y + w // 2, x - w:x + w, ...]
            img = cv2.resize(img, (128, 64))

            # Visualize
            if verbose:
                cv2.imshow("", cv2.cvtColor(img, cv2.COLOR_RGB2BGR))
                cv2.waitKey(0)
                1 / 0

            video.append(img)

    video = np.stack(video, axis=0).astype(np.float32)
    video = torch.FloatTensor(video.transpose(3, 0, 1, 2)) / 255.0

    return video


def ctc_arr2txt(arr, start):
    letters = [' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
               'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z']
    pre = -1
    txt = []
    for n in arr:
        if(pre != n and n >= start):
            if(len(txt) > 0 and txt[-1] == ' ' and letters[n - start] == ' '):
                pass
            else:
                txt.append(letters[n - start])
        pre = n
    return ''.join(txt).strip()


def decode_sentence(y):
    y = y.argmax(-1)
    return ctc_arr2txt(y, start=1).lower()


def lipnet_predict(video, model):
    if video is None:
        return None
    y = model(video[None, ...].cuda())
    txt = decode_sentence(y[0])

    return txt


def read_transcript(file):
    with open(file, 'r') as f:
        text = f.readline()
    return text.lower()


def get_model(device):
    model = LipNet()
    model = model.to(device)

    pretrained_dict = torch.load(RAIDROOT + 'Networks/lipnet.pt')
    model_dict = model.state_dict()
    pretrained_dict = {k: v for k, v in pretrained_dict.items(
    ) if k in model_dict.keys() and v.size() == model_dict[k].size()}
    missed_params = [k for k, v in model_dict.items(
    ) if k not in pretrained_dict.keys()]
    print('loaded params/tot params:{}/{}'.format(len(pretrained_dict), len(model_dict)))
    print('miss matched params:{}'.format(missed_params))
    model_dict.update(pretrained_dict)
    model.load_state_dict(model_dict)

    return model


```

## Source file: utils/compute_face_mask.py

{
  "path": "utils/compute_face_mask.py",
  "blob_sha": "84bc3823cfb998be149c20504f67ba0fc20f33d7",
  "sha256": "cfb9b7860f33024d3b2ddcbe7a23e47a025f51766ca81db874b5548c257a17c3",
  "lines": 128,
  "bytes": 4006,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 20,
    "mouth": 11,
    "gum": 8,
    "rig": 6
  },
  "url": "https://github.com/FeliMe/AudioStyleNet/blob/070e7214ec73f5d81c77d367eb656469d1a1ad84/utils/compute_face_mask.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def show_landmarks(lm, image=None):"
    },
    {
      "line": 19,
      "text": "def points_to_mask(lm):"
    },
    {
      "line": 29,
      "text": "def get_mouth_mask(landmarks, std_factor):"
    },
    {
      "line": 61,
      "text": "def get_eyes_mask(landmarks, std_factor):"
    },
    {
      "line": 62,
      "text": "def get_eye_mask(mean, std):"
    }
  ]
}

```text

import argparse
import cv2
import numpy as np
import os
import torch

DATAROOT = os.environ['DATAROOT']


def show_landmarks(lm, image=None):
    if image is None:
        image = np.zeros((256, 256, 3))
    for (x, y) in lm:
        image = cv2.circle(image, (x, y), 1, (255, 255, 255), 1)
    cv2.imshow("", image)
    cv2.waitKey(0)


def points_to_mask(lm):
    hull = cv2.convexHull(lm.astype(np.int64))
    mask = np.zeros((256, 256, 3), dtype='uint8')
    mask = cv2.drawContours(
        mask, [hull], -1, (255, 255, 255), thickness=cv2.FILLED)
    mask = cv2.cvtColor(mask, cv2.COLOR_BGR2GRAY)
    _, mask = cv2.threshold(mask, 10, 255, cv2.THRESH_BINARY)
    return mask


def get_mouth_mask(landmarks, std_factor):
    mouth_lm = landmarks[:, 48:60]

    means = mouth_lm.mean(axis=0)
    std = mouth_lm.std(axis=0) * std_factor

    mask = means.copy()
    mask[0, 0] -= std[0, 0]
    mask[1] -= std[1]
    mask[2] -= std[2]
    mask[3, 1] -= std[3, 1]
    mask[4, 0] += std[4, 0]
    mask[4, 1] -= std[4, 1]
    mask[5, 0] += std[5, 0]
    mask[5, 1] -= std[5, 1]
    mask[6, 0] += std[6, 0]
    mask[7] += std[7]
    mask[8] += std[8]
    mask[9, 1] += std[9, 1]
    mask[10, 0] -= std[10, 0]
    mask[10, 1] += std[10, 1]
    mask[11, 0] -= std[11, 0]
    mask[11, 1] += std[11, 1]

    mask = points_to_mask(mask)

    # Convert to torch tensor
    mask = torch.tensor(mask, dtype=torch.float32).unsqueeze(0)

    return mask


def get_eyes_mask(landmarks, std_factor):
    def get_eye_mask(mean, std):
        mask = mean.copy()
        mask[0] -= std[0]
        mask[1] -= std[1]
        mask[2, 1] -= std[2, 1]
        mask[3, 0] += std[3, 0]
        mask[3, 1] -= std[3, 1]
        mask[4, 0] += std[4, 0]
        mask[4, 1] -= std[4, 1]
        mask[5, 0] -= std[5, 0]
        mask[5, 1] += std[5, 1]
        mask[6] += std[6]
        mask[7] += std[7]
        mask[8, 0] -= std[8, 0]
        mask[8, 1] += std[8, 1]
        return mask

    # Right eye
    right_eye = np.concatenate(
        (landmarks[:, 17:22], landmarks[:, 36].reshape(-1, 1, 2), landmarks[:, 39:42]), axis=1)
    right_eye_mask = get_eye_mask(right_eye.mean(
        axis=0), right_eye.std(axis=0) * std_factor)
    right_eye_mask = points_to_mask(right_eye_mask)

    # Left eye
    left_eye = np.concatenate(
        (landmarks[:, 22:27], landmarks[:, 42].reshape(-1, 1, 2), landmarks[:, 45:48]), axis=1)
    left_eye_mask = get_eye_mask(left_eye.mean(
        axis=0), left_eye.std(axis=0) * std_factor)
    left_eye_mask = points_to_mask(left_eye_mask)

    # Combine bitwise
    mask = cv2.bitwise_or(right_eye_mask, left_eye_mask)

    # Convert to torch tensor
    mask = torch.tensor(mask, dtype=torch.float32).unsqueeze(0)

    return mask


if __name__ == '__main__':
    # Parse arguments
    parser = argparse.ArgumentParser()
    parser.add_argument('--data_path', type=str,
                        default=DATAROOT + 'AudioVisualDataset/Aligned256/latent_data.pt')
    parser.add_argument('--output_dir', type=str, default='../saves/pre-trained/')
    parser.add_argument('--mouth', action='store_true')
    parser.add_argument('--std_factor_mouth', type=float, default=3.)
    parser.add_argument('--eyes', action='store_true')
    parser.add_argument('--std_factor_eyes', type=float, default=3.)
    args = parser.parse_args()

    # Load data
    data = torch.load(args.data_path)
    landmarks = data['landmarks'].numpy()
    del data

    if args.mouth:
        mouth_mask = get_mouth_mask(landmarks, std_factor=args.std_factor_mouth)
        save_path = args.output_dir + f"tagesschau_mouth_mask_{int(args.std_factor_mouth)}std.pt"
        print(mouth_mask.shape, save_path)
        torch.save(mouth_mask, save_path)
    if args.eyes:
        eyes_mask = get_eyes_mask(landmarks, std_factor=args.std_factor_eyes)
        save_path = args.output_dir + f"tagesschau_eyes_mask_{int(args.std_factor_eyes)}std.pt"
        print(eyes_mask.shape, save_path)
        torch.save(eyes_mask, save_path)


```

## Source file: utils/download_faceforensics.py

{
  "path": "utils/download_faceforensics.py",
  "blob_sha": "cf9a53baa7334397258c8c74d93ab1c3ead62006",
  "sha256": "1ec0c2c21b8600eae507c74cbb0c5565e75ca7557da6df4f8a8d5b4720532584",
  "lines": 257,
  "bytes": 10624,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 9,
    "rig": 14,
    "face": 9
  },
  "url": "https://github.com/FeliMe/AudioStyleNet/blob/070e7214ec73f5d81c77d367eb656469d1a1ad84/utils/download_faceforensics.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def parse_args():"
    },
    {
      "line": 98,
      "text": "def download_files(filenames, base_url, output_path, report_progress=True):"
    },
    {
      "line": 106,
      "text": "def reporthook(count, block_size, total_size):"
    },
    {
      "line": 120,
      "text": "def download_file(url, out_file, report_progress=False):"
    },
    {
      "line": 136,
      "text": "def main(args):"
    }
  ]
}

```text

#!/usr/bin/env python
""" Downloads FaceForensics++ and Deep Fake Detection public data release
Example usage:
    see -h or https://github.com/ondyari/FaceForensics
"""
# -*- coding: utf-8 -*-
import argparse
import os
import urllib
import urllib.request
import tempfile
import time
import sys
import json
import random
from tqdm import tqdm
from os.path import join


# URLs and filenames
FILELIST_URL = 'misc/filelist.json'
DEEPFEAKES_DETECTION_URL = 'misc/deepfake_detection_filenames.json'
DEEPFAKES_MODEL_NAMES = ['decoder_A.h5', 'decoder_B.h5', 'encoder.h5', ]

# Parameters
DATASETS = {
    'original_youtube_videos': 'misc/downloaded_youtube_videos.zip',
    'original_youtube_videos_info': 'misc/downloaded_youtube_videos_info.zip',
    'original': 'original_sequences/youtube',
    'DeepFakeDetection_original': 'original_sequences/actors',
    'Deepfakes': 'manipulated_sequences/Deepfakes',
    'DeepFakeDetection': 'manipulated_sequences/DeepFakeDetection',
    'Face2Face': 'manipulated_sequences/Face2Face',
    'FaceSwap': 'manipulated_sequences/FaceSwap',
    'NeuralTextures': 'manipulated_sequences/NeuralTextures'
}
ALL_DATASETS = ['original', 'DeepFakeDetection_original', 'Deepfakes',
                'DeepFakeDetection', 'Face2Face', 'FaceSwap',
                'NeuralTextures']
COMPRESSION = ['raw', 'c23', 'c40']
TYPE = ['videos', 'masks', 'models']
SERVERS = ['EU', 'EU2', 'CA']


def parse_args():
    parser = argparse.ArgumentParser(
        description='Downloads FaceForensics v2 public data release.',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument('output_path', type=str, help='Output directory.')
    parser.add_argument('-d', '--dataset', type=str, default='original',
                        help='Which dataset to download, either pristine or '
                             'manipulated data or the downloaded youtube '
                             'videos.',
                        choices=list(DATASETS.keys()) + ['all']
                        )
    parser.add_argument('-c', '--compression', type=str, default='raw',
                        help='Which compression degree. All videos '
                             'have been generated with h264 with a varying '
                             'codec. Raw (c0) videos are lossless compressed.',
                        choices=COMPRESSION
                        )
    parser.add_argument('-t', '--type', type=str, default='videos',
                        help='Which file type, i.e. videos, masks, for our '
                             'manipulation methods, models, for Deepfakes.',
                        choices=TYPE
                        )
    parser.add_argument('-n', '--num_videos', type=int, default=None,
                        help='Select a number of videos number to '
                             "download if you don't want to download the full"
                             ' dataset.')
    parser.add_argument('--server', type=str, default='EU',
                        help='Server to download the data from. If you '
                             'encounter a slow download speed, consider '
                             'changing the server.',
                        choices=SERVERS
                        )
    args = parser.parse_args()

    # URLs
    server = args.server
    if server == 'EU':
        server_url = 'http://canis.vc.in.tum.de:8100/'
    elif server == 'EU2':
        server_url = 'http://kaldir.vc.in.tum.de/faceforensics/'
    elif server == 'CA':
        server_url = 'http://falas.cmpt.sfu.ca:8100/'
    else:
        raise Exception('Wrong server name. Choices: {}'.format(str(SERVERS)))
    args.tos_url = server_url + 'webpage/FaceForensics_TOS.pdf'
    args.base_url = server_url + 'v3/'
    args.deepfakes_model_url = server_url + 'v3/manipulated_sequences/' + \
        'Deepfakes/models/'

    return args


def download_files(filenames, base_url, output_path, report_progress=True):
    os.makedirs(output_path, exist_ok=True)
    if report_progress:
        filenames = tqdm(filenames)
    for filename in filenames:
        download_file(base_url + filename, join(output_path, filename))


def reporthook(count, block_size, total_size):
    global start_time
    if count == 0:
        start_time = time.time()
        return
    duration = time.time() - start_time
    progress_size = int(count * block_size)
    speed = int(progress_size / (1024 * duration))
    percent = int(count * block_size * 100 / total_size)
    sys.stdout.write("\rProgress: %d%%, %d MB, %d KB/s, %d seconds passed" %
                     (percent, progress_size / (1024 * 1024), speed, duration))
    sys.stdout.flush()


def download_file(url, out_file, report_progress=False):
    out_dir = os.path.dirname(out_file)
    if not os.path.isfile(out_file):
        fh, out_file_tmp = tempfile.mkstemp(dir=out_dir)
        f = os.fdopen(fh, 'w')
        f.close()
        if report_progress:
            urllib.request.urlretrieve(url, out_file_tmp,
                                       reporthook=reporthook)
        else:
            urllib.request.urlretrieve(url, out_file_tmp)
        os.rename(out_file_tmp, out_file)
    else:
        tqdm.write('WARNING: skipping download of existing file ' + out_file)


def main(args):
    # TOS
    print('By pressing any key to continue you confirm that you have agreed '
          'to the FaceForensics terms of use as described at:')
    print(args.tos_url)
    print('***')
    print('Press any key to continue, or CTRL-C to exit.')
    _ = input('')

    # Extract arguments
    c_datasets = [args.dataset] if args.dataset != 'all' else ALL_DATASETS
    c_type = args.type
    c_compression = args.compression
    num_videos = args.num_videos
    output_path = args.output_path
    os.makedirs(output_path, exist_ok=True)

    # Check for special dataset cases
    for dataset in c_datasets:
        dataset_path = DATASETS[dataset]
        # Special cases
        if 'original_youtube_videos' in dataset:
            # Here we download the original youtube videos zip file
            print('Downloading original youtube videos.')
            if not 'info' in dataset_path:
                print('Please be patient, this may take a while (~40gb)')
                suffix = ''
            else:
            	suffix = 'info'
            download_file(args.base_url + '/' + dataset_path,
                          out_file=join(output_path,
                                        'downloaded_videos{}.zip'.format(
                                            suffix)),
                          report_progress=True)
            return

        # Else: regular datasets
        print('Downloading {} of dataset "{}"'.format(
            c_type, dataset_path
        ))

        # Get filelists and video lenghts list from server
        if 'DeepFakeDetection' in dataset_path or 'actors' in dataset_path:
        	filepaths = json.loads(urllib.request.urlopen(args.base_url + '/' +
                                                       DEEPFEAKES_DETECTION_URL).read().decode("utf-8"))
        	if 'actors' in dataset_path:
        		filelist = filepaths['actors']
        	else:
        		filelist = filepaths['DeepFakesDetection']
        elif 'original' in dataset_path:
            # Load filelist from server
            file_pairs = json.loads(urllib.request.urlopen(args.base_url + '/' +
                                                           FILELIST_URL).read().decode("utf-8"))
            filelist = []
            for pair in file_pairs:
            	filelist += pair
        else:
            # Load filelist from server
            file_pairs = json.loads(urllib.request.urlopen(args.base_url + '/' +
                                                           FILELIST_URL).read().decode("utf-8"))
            # Get filelist
            filelist = []
            for pair in file_pairs:
                filelist.append('_'.join(pair))
                if c_type != 'models':
                    filelist.append('_'.join(pair[::-1]))
        # Maybe limit number of videos for download
        if num_videos is not None and num_videos > 0:
        	print('Downloading the first {} videos'.format(num_videos))
        	filelist = filelist[:num_videos]

        # Server and local paths
        dataset_videos_url = args.base_url + '{}/{}/{}/'.format(
            dataset_path, c_compression, c_type)
        dataset_mask_url = args.base_url + '{}/{}/videos/'.format(
            dataset_path, 'masks', c_type)

        if c_type == 'videos':
            dataset_output_path = join(output_path, dataset_path, c_compression,
                                       c_type)
            print('Output path: {}'.format(dataset_output_path))
            filelist = [filename + '.mp4' for filename in filelist]
            download_files(filelist, dataset_videos_url, dataset_output_path)
        elif c_type == 'masks':
            dataset_output_path = join(output_path, dataset_path, c_type,
                                       'videos')
            print('Output path: {}'.format(dataset_output_path))
            if 'original' in dataset:
                if args.dataset != 'all':
                    print('Only videos available for original data. Aborting.')
                    return
                else:
                    print('Only videos available for original data. '
                          'Skipping original.\n')
                    continue
            filelist = [filename + '.mp4' for filename in filelist]
            download_files(filelist, dataset_mask_url, dataset_output_path)

        # Else: models for deepfakes
        else:
            if dataset != 'Deepfakes' and c_type == 'models':
                print('Models only available for Deepfakes. Aborting')
                return
            dataset_output_path = join(output_path, dataset_path, c_type)
            print('Output path: {}'.format(dataset_output_path))

            # Get Deepfakes models
            for folder in tqdm(filelist):
                folder_filelist = DEEPFAKES_MODEL_NAMES

                # Folder paths
                folder_base_url = args.deepfakes_model_url + folder + '/'
                folder_dataset_output_path = join(dataset_output_path,
                                                  folder)
                download_files(folder_filelist, folder_base_url,
                               folder_dataset_output_path,
                               report_progress=False)   # already done


if __name__ == "__main__":
    args = parse_args()
    main(args)


```

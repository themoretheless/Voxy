# Facial rendering source scan: Fanziapril/mvfnet

Repository: https://github.com/Fanziapril/mvfnet

Pinned source revision: bb7ce9b53368cd213489e41f29bf6ce6ae200f9d

Discovery topic: face model 3D

Description: Pytorch code for paper: MVF-Net: Multi-View 3D Face Morphable Model Regression

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["data/imgs/right.jpg"]}

## Source file: model.py

{
  "path": "model.py",
  "blob_sha": "6736901c06173e7aae16a2494ba8f08569716242",
  "sha256": "46a423be54be2a4bccbe34ca5abf863a2cd89a1f04cb6ab0e8a54eac2fde9467",
  "lines": 95,
  "bytes": 3659,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/Fanziapril/mvfnet/blob/bb7ce9b53368cd213489e41f29bf6ce6ae200f9d/model.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def reset_params(net):"
    },
    {
      "line": 24,
      "text": "def __init__(self):"
    },
    {
      "line": 78,
      "text": "def forward(self, x):"
    }
  ]
}

```text

import torch
import torch.nn as nn
from torch.nn import functional as F
from collections import OrderedDict
import torchvision.models as tvmodel

def reset_params(net):
    for m in net.modules():
        if isinstance(m, nn.Conv2d):
            nn.init.normal(m.weight, 0.0, 0.02)
            if m.bias is not None:
                nn.init.constant(m.bias, 0)
        elif isinstance(m, nn.Linear):
            nn.init.normal(m.weight, 0.0, 0.0001)
            if m.bias is not None:
                nn.init.constant(m.bias, 0)
        elif isinstance(m, nn.BatchNorm2d):
            nn.init.constant(m.weight, 1)
            nn.init.normal(m.weight, 1.0, 0.02)
            nn.init.constant(m.bias, 0)


class VggEncoder(nn.Module):
    def __init__(self):
        super(VggEncoder, self).__init__()

        self.featChannel = 512
        self.layer1 = tvmodel.vgg16_bn(pretrained=True).features
        self.layer1 = nn.Sequential(OrderedDict([
            ('conv1',  nn.Conv2d(3, 64, (3, 3), (1, 1), (1, 1))),
            ('bn1',  nn.BatchNorm2d(64)),
            ('relu1', nn.ReLU(True)),
            ('pool1',  nn.MaxPool2d((2, 2), (2, 2), (0, 0), ceil_mode=True)),
       
            ('conv2',  nn.Conv2d(64, 128, (3, 3), (1, 1), (1, 1))),
            ('bn2',  nn.BatchNorm2d(128)),
            ('relu2', nn.ReLU(True)),
            ('pool2',  nn.MaxPool2d((2, 2), (2, 2), (0, 0), ceil_mode=True)),
        
            ('conv3',  nn.Conv2d(128, 256, (3, 3), (1, 1), (1, 1))),
            ('bn3',  nn.BatchNorm2d(256)),
            ('relu3', nn.ReLU(True)),
        
            ('conv4', nn.Conv2d(256, 256, (3, 3), (1, 1), (1, 1))),
            ('bn4',  nn.BatchNorm2d(256)),
            ('relu4', nn.ReLU(True)),
            ('pool3',  nn.MaxPool2d((2, 2), (2, 2), (0, 0), ceil_mode=True)),
        
            ('conv5',  nn.Conv2d(256, 512, (3, 3), (1, 1), 1)),
            ('bn5',  nn.BatchNorm2d(512)),
            ('relu5', nn.ReLU(True)),
            ('pool4',  nn.MaxPool2d((2, 2), (2, 2), (0, 0), ceil_mode=True)),
        
            ('conv6',  nn.Conv2d(512, 512, (3, 3), stride=1, padding=1)),
            ('bn6',  nn.BatchNorm2d(512)),
            ('relu6', nn.ReLU(True)),
        
            ('conv7',  nn.Conv2d(512, 512, (3, 3), (1, 1), 1)),
            ('bn7',  nn.BatchNorm2d(512)),
            ('relu7', nn.ReLU(True)),
            ('pool5',  nn.MaxPool2d((2, 2), (2, 2), (0, 0), ceil_mode=True)),
            ]))

        
            
        self.fc_3dmm = nn.Sequential(OrderedDict([
            ('fc1', nn.Linear(self.featChannel*3, 256*3)),
            ('relu1', nn.ReLU(True)),
            ('fc2', nn.Linear(256*3, 228))]))
        
        self.fc_pose = nn.Sequential(OrderedDict([
           ('fc3', nn.Linear(512, 256)),
           ('relu2', nn.ReLU(True)),
           ('fc4', nn.Linear(256, 7))]))
        reset_params(self.fc_3dmm)
        reset_params(self.fc_pose)

    def forward(self, x):
        imga = x[:, 0:3, :, :]
        feata = self.layer1(imga)
        feata = F.avg_pool2d(feata, feata.size()[2:]).view(feata.size(0), feata.size(1))
        posea = self.fc_pose(feata)
        imgb = x[:, 3:6, :, :]
        featb = self.layer1(imgb)
        featb = F.avg_pool2d(featb, featb.size()[2:]).view(featb.size(0), featb.size(1))
        poseb = self.fc_pose(featb)
        imgc = x[:, 6:9, :, :]
        featc = self.layer1(imgc)
        featc = F.avg_pool2d(featc, featc.size()[2:]).view(featc.size(0), featc.size(1))
        posec = self.fc_pose(featc)
        para = self.fc_3dmm(torch.cat([feata, featb, featc], dim=1))
        out = torch.cat([para, posea, poseb, posec], dim=1)
        return out




```

## Source file: tools.py

{
  "path": "tools.py",
  "blob_sha": "2faf54a4335f46821d1d327b12116509aadd08b0",
  "sha256": "6e2ac0214433c1c83ff2b6cb9f5798dccd41f80760b7549069da1d6eba4217e4",
  "lines": 192,
  "bytes": 6722,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "face": 11,
    "lip": 1
  },
  "url": "https://github.com/Fanziapril/mvfnet/blob/bb7ce9b53368cd213489e41f29bf6ce6ae200f9d/tools.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def angle_to_rotation(angles):"
    },
    {
      "line": 42,
      "text": "def preds_to_pose(preds):"
    },
    {
      "line": 49,
      "text": "def preds_to_shape(preds):"
    },
    {
      "line": 69,
      "text": "def crop_image(image, res=224):"
    },
    {
      "line": 104,
      "text": "def write_ply(filename, points=None, mesh=None, colors=None, as_text=True):"
    },
    {
      "line": 170,
      "text": "def describe_element(name, df):"
    }
  ]
}

```text

import face_alignment
import numpy as np
import scipy.io as io
import torchvision.transforms as transforms
from PIL import Image
import pandas as pd
import sys
import math

model_shape = io.loadmat('data/Model_Shape.mat')
kpt_index = np.reshape(model_shape['keypoints'], 68).astype(np.int32) - 1
model_exp = io.loadmat('data/Model_Expression.mat')
data = io.loadmat('data/sigma_exp.mat')
pose_mean = np.array([0,0,0,112,112,0,0]).astype(np.float32)
pose_std = np.array([math.pi/2.0,math.pi/2.0,math.pi/2.0,56,56,1,224.0 / (2 * 180000.0)]).astype(np.float32)

def angle_to_rotation(angles):
    phi = angles[0]
    gamma = angles[1]
    theta = angles[2]
    
    R_x = np.eye(3)
    R_x[1, 1] = math.cos(phi)
    R_x[1, 2] = math.sin(phi)
    R_x[2, 1] = - math.sin(phi)
    R_x[2, 2] = math.cos(phi)

    R_y = np.eye(3)
    R_y[0, 0] = math.cos(gamma)
    R_y[0, 2] = - math.sin(gamma)
    R_y[2, 0] = math.sin(gamma)
    R_y[2, 2] = math.cos(gamma)

    R_z = np.eye(3)
    R_z[0, 0] = math.cos(theta)
    R_z[0, 1] = math.sin(theta)
    R_z[1, 0] = - math.sin(theta)
    R_z[1, 1] = math.cos(theta)

    return np.matmul(np.matmul(R_x, R_y), R_z)

def preds_to_pose(preds):
    pose = preds * pose_std + pose_mean
    R = angle_to_rotation(pose[:3])
    t2d = pose[3:5]
    s = pose[6]
    return R, t2d, s

def preds_to_shape(preds):
    # paras = torch.mul(preds[:228, :], label_std[:199+29, :])
    alpha = np.reshape(preds[:199], [199,1]) * np.reshape(model_shape['sigma'], [199,1])
    beta = np.reshape(preds[199:228], [29, 1]) * 1.0/(1000.0 * np.reshape(data['sigma_exp'], [29, 1]))
    face_shape = np.matmul(model_shape['w'], alpha) + np.matmul(model_exp['w_exp'], beta) + model_shape['mu_shape']
    face_shape = face_shape.reshape(-1, 3)
    
    R, t, s = preds_to_pose(preds[228:228+7])
    kptA = np.matmul(face_shape[kpt_index], s*R[:2].transpose()) + np.repeat(np.reshape(t,[1,2]), 68, axis=0) 
    kptA[:, 1] = 224 - kptA[:, 1]
    R, t, s = preds_to_pose(preds[228+7:228+14])
    kptB = np.matmul(face_shape[kpt_index], s*R[:2].transpose()) + np.repeat(np.reshape(t,[1,2]), 68, axis=0)
    kptB[:, 1] = 224 - kptB[:, 1]
    
    R, t, s = preds_to_pose(preds[228+14:])
    kptC = np.matmul(face_shape[kpt_index], s*R[:2].transpose()) + np.repeat(np.reshape(t,[1,2]), 68, axis=0)
    kptC[:, 1] = 224 - kptC[:, 1]
    return [face_shape, model_shape['tri'].astype(np.int64).transpose() - 1, kptA, kptB, kptC]
    
    
def crop_image(image, res=224):
    fa = face_alignment.FaceAlignment(face_alignment.LandmarksType._3D, flip_input=False)
    pts = fa.get_landmarks(np.array(image))
    if len(pts) < 1:
        assert "No face detected!"
    pts = np.array(pts[0]).astype(np.int32)
        
    h = image.size[1]
    w = image.size[0]
        # x-width-pts[0,:], y-height-pts[1,:]
    x_max = np.max(pts[:68, 0])
    x_min = np.min(pts[:68, 0])
    y_max = np.max(pts[:68, 1])
    y_min = np.min(pts[:68, 1])
    bbox = [y_min, x_min, y_max, x_max]
    # c (cy, cx)
    c = [bbox[2] - (bbox[2] - bbox[0]) / 2, bbox[3] - (bbox[3] - bbox[1]) / 2.0]
    c[0] = c[0] - (bbox[2] - bbox[0]) * 0.12
    s = (max(bbox[2] - bbox[0], bbox[3] - bbox[1]) * 1.5).astype(np.int32)
    old_bb = np.array([c[0] - s / 2, c[1] - s / 2, c[0] + s / 2, c[1] + s / 2]).astype(np.int32)
    crop_img = Image.new('RGB', (s, s))
    #crop_img = torch.zeros(image.shape[0], s, s, dtype=torch.float32)

    shift_x = 0 - old_bb[1]
    shift_y = 0 - old_bb[0]
    old_bb = np.array([max(0, old_bb[0]), max(0, old_bb[1]),
              min(h, old_bb[2]), min(w, old_bb[3])]).astype(np.int32)
    hb = old_bb[2] - old_bb[0]
    wb = old_bb[3] - old_bb[1]
    new_bb = np.array([max(0, shift_y), max(0, shift_x), max(0, shift_y) + hb, max(0, shift_x) + wb]).astype(np.int32)
    cache = image.crop((old_bb[1], old_bb[0], old_bb[3], old_bb[2]))
    crop_img.paste(cache, (new_bb[1], new_bb[0], new_bb[3], new_bb[2]))
    crop_img = crop_img.resize((res, res), Image.BICUBIC)
    return crop_img

def write_ply(filename, points=None, mesh=None, colors=None, as_text=True):
    points = pd.DataFrame(points, columns=["x", "y", "z"])
    mesh = pd.DataFrame(mesh, columns=["v1", "v2", "v3"])
    if colors is not None:
        colors = pd.DataFrame(colors, columns=["red", "green", "blue"])
        points = pd.concat([points, colors], axis=1)
    """
 
    Parameters
    ----------
    filename: str
        The created file will be named with this
    points: ndarray
    mesh: ndarray
    as_text: boolean
        Set the write mode of the file. Default: binary
 
    Returns
    -------
    boolean
        True if no problems
 
    """
    if not filename.endswith('ply'):
        filename += '.ply'

    # open in text mode to write the header
    with open(filename, 'w') as ply:
        header = ['ply']

        if as_text:
            header.append('format ascii 1.0')
        else:
            header.append('format binary_' + sys.byteorder + '_endian 1.0')

        if points is not None:
            header.extend(describe_element('vertex', points))
        if mesh is not None:
            mesh = mesh.copy()
            mesh.insert(loc=0, column="n_points", value=3)
            mesh["n_points"] = mesh["n_points"].astype("u1")
            header.extend(describe_element('face', mesh))

        header.append('end_header')

        for line in header:
            ply.write("%s\n" % line)

    if as_text:
        if points is not None:
            points.to_csv(filename, sep=" ", index=False, header=False, mode='a',
                          encoding='ascii')
        if mesh is not None:
            mesh.to_csv(filename, sep=" ", index=False, header=False, mode='a',
                        encoding='ascii')

    else:
        # open in binary/append to use tofile
        with open(filename, 'ab') as ply:
            if points is not None:
                points.to_records(index=False).tofile(ply)
            if mesh is not None:
                mesh.to_records(index=False).tofile(ply)

    return True

def describe_element(name, df):
    """ Takes the columns of the dataframe and builds a ply-like description
    Parameters
    ----------
    name: str
    df: pandas DataFrame
    Returns
    -------
    element: list[str]
    """
    property_formats = {'f': 'float', 'u': 'uchar', 'i': 'int'}
    element = ['element ' + name + ' ' + str(len(df))]

    if name == 'face':
        element.append("property list uchar int vertex_indices")

    else:
        for i in range(len(df.columns)):
            # get first letter of dtype to infer format
            f = property_formats[str(df.dtypes[i])[0]]
            element.append('property ' + f + ' ' + str(df.columns.values[i]))

    return element


```

## Source file: test_img.py

{
  "path": "test_img.py",
  "blob_sha": "a52c77324fcd68cf83e21858d2141622dc136943",
  "sha256": "e82cd0fbb337a660494f9ae93d93dfcc19cd38516a0b31c78523c6a6f4775b93",
  "lines": 36,
  "bytes": 1458,
  "selection_score": -2,
  "keyword_line_counts": {
    "gum": 3,
    "rig": 2,
    "face": 3
  },
  "url": "https://github.com/Fanziapril/mvfnet/blob/bb7ce9b53368cd213489e41f29bf6ce6ae200f9d/test_img.py",
  "function_signature_candidates": []
}

```text

import torch
import argparse
import os
from PIL import Image
import tools
import torchvision.transforms as transforms
from model import VggEncoder
import time

parser = argparse.ArgumentParser()
parser.add_argument('--image_path', type=str, default=None, help='path to load images. It should include image name with: front|left|right')
parser.add_argument('--save_dir', type=str, default='./result', help='path to save 3D face shapes')

options = parser.parse_args()
crop_opt = True # change to True if you want to crop the image
imgA = Image.open(os.path.join(options.image_path, 'front.jpg')).convert('RGB')
imgB = Image.open(os.path.join(options.image_path, 'left.jpg')).convert('RGB')
imgC = Image.open(os.path.join(options.image_path, 'right.jpg')).convert('RGB')
if crop_opt:
    imgA = tools.crop_image(imgA)
    imgB = tools.crop_image(imgB)
    imgC = tools.crop_image(imgC)
imgA = transforms.functional.to_tensor(imgA)
imgB = transforms.functional.to_tensor(imgB)
imgC = transforms.functional.to_tensor(imgC)
model = VggEncoder()
model = torch.nn.DataParallel(model).cuda()
ckpt = torch.load('data/net.pth')
model.load_state_dict(ckpt)
#print model
input_tensor = torch.cat([imgA, imgB, imgC], 0).view(1, 9, 224, 224).cuda()
start = time.time()
preds = model(input_tensor)
print(time.time() -start)
faces3d = tools.preds_to_shape(preds[0].detach().cpu().numpy())
tools.write_ply(os.path.join(options.save_dir, 'shape.ply'), faces3d[0], faces3d[1])


```

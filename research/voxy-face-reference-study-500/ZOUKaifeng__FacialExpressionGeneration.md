# Facial rendering source scan: ZOUKaifeng/FacialExpressionGeneration

Repository: https://github.com/ZOUKaifeng/FacialExpressionGeneration

Pinned source revision: b87720365dc259cb6d9595255f7e2dd7d0366139

Discovery topic: facial animation

Description: Disentangled 3D face animation generator

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"mouth": ["Results/diversity_mouth_open.jpg", "Results/gif/MouthExtrem.gif", "Results/gif/MouthUp.gif", "Results/gif/mouth_open.gif"], "teeth": ["Results/diversity_bareteeth.jpg", "Results/gif/BareTeeth.gif", "Results/gif/bareteeth_diversity.gif", "Results/gif/div_bareteeth.gif"], "face": ["Results/Face 3D.png"], "lip": ["Results/gif/LipsUp.gif"]}

## Source file: evaluate/fid.py

{
  "path": "evaluate/fid.py",
  "blob_sha": "fb32d1844d109c2ddc9c9de1f4f9cf5851f6f6e4",
  "sha256": "fd2cb686a98808fb711c9bf553f4d764b05286870fdb7c41fa3a198594a660a2",
  "lines": 58,
  "bytes": 2246,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/ZOUKaifeng/FacialExpressionGeneration/blob/b87720365dc259cb6d9595255f7e2dd7d0366139/evaluate/fid.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def calculate_frechet_distance(act1, act2, eps=1e-6):"
    }
  ]
}

```text

import numpy as np
from scipy import linalg
from numpy import cov


def calculate_frechet_distance(act1, act2, eps=1e-6):
    """Numpy implementation of the Frechet Distance.
    The Frechet distance between two multivariate Gaussians X_1 ~ N(mu_1, C_1)
    and X_2 ~ N(mu_2, C_2) is
            d^2 = ||mu_1 - mu_2||^2 + Tr(C_1 + C_2 - 2*sqrt(C_1*C_2)).
    Stable version by Dougal J. Sutherland.
    Params:
    -- mu1   : Numpy array containing the activations of a layer of the
               inception net (like returned by the function 'get_predictions')
               for generated samples.
    -- mu2   : The sample mean over activations, precalculated on an
               representative data set.
    -- sigma1: The covariance matrix over activations for generated samples.
    -- sigma2: The covariance matrix over activations, precalculated on an
               representative data set.
    Returns:
    --   : The Frechet Distance.
    """

    mu1, sigma1 = act1.mean(axis=0), cov(act1, rowvar=False)
    mu2, sigma2 = act2.mean(axis=0), cov(act2, rowvar=False)
    mu1 = np.atleast_1d(mu1)
    mu2 = np.atleast_1d(mu2)

    sigma1 = np.atleast_2d(sigma1)
    sigma2 = np.atleast_2d(sigma2)

    assert mu1.shape == mu2.shape, \
        'Training and test mean vectors have different lengths'
    assert sigma1.shape == sigma2.shape, \
        'Training and test covariances have different dimensions'

    diff = mu1 - mu2

    # Product might be almost singular
    covmean, _ = linalg.sqrtm(sigma1.dot(sigma2), disp=False)

    if not np.isfinite(covmean).all():
        msg = ('fid calculation produces singular product; adding %s to diagonal of cov estimates') % eps
        print(msg)
        offset = np.eye(sigma1.shape[0]) * eps
        covmean = linalg.sqrtm((sigma1 + offset).dot(sigma2 + offset))

    # Numerical error might give slight imaginary component
    if np.iscomplexobj(covmean):
        if not np.allclose(np.diagonal(covmean).imag, 0, atol=1e-3):
            m = np.max(np.abs(covmean.imag))
            raise ValueError('Imaginary component {}'.format(m))
        covmean = covmean.real

    tr_covmean = np.trace(covmean)

    return diff.dot(diff) + np.trace(sigma1) + np.trace(sigma2) - 2 * tr_covmean

```

## Source file: evaluate/__init__.py

{
  "path": "evaluate/__init__.py",
  "blob_sha": "6b261d36a739e9d9567d1e26370f909274639fb6",
  "sha256": "bcd8e788b8aba7bf43ed92757f5bfd47d856128e372040557a24190ab7cea890",
  "lines": 2,
  "bytes": 99,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/ZOUKaifeng/FacialExpressionGeneration/blob/b87720365dc259cb6d9595255f7e2dd7d0366139/evaluate/__init__.py",
  "function_signature_candidates": []
}

```text

from evaluate.fid import calculate_frechet_distance
from evaluate.classifier import LSTMClassifier


```

## Source file: evaluate/classifier.py

{
  "path": "evaluate/classifier.py",
  "blob_sha": "ed3fabfafa3767e0e74f81c5a284e91b6f701518",
  "sha256": "aedde060c5f293ee91758cb053e2fe8b4a80989b844edc78059d6f27d071addd",
  "lines": 233,
  "bytes": 7005,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 4
  },
  "url": "https://github.com/ZOUKaifeng/FacialExpressionGeneration/blob/b87720365dc259cb6d9595255f7e2dd7d0366139/evaluate/classifier.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def __init__(self, data, label):"
    },
    {
      "line": 35,
      "text": "def __len__(self):"
    },
    {
      "line": 39,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 54,
      "text": "def __init__(self, num_pts, num_frame, label_size):"
    },
    {
      "line": 67,
      "text": "def forward(self, x):"
    },
    {
      "line": 88,
      "text": "def train_step(net,  dataloader, optimizer, criterion):"
    },
    {
      "line": 116,
      "text": "def valid_step(net,  dataloader, criterion):"
    }
  ]
}

```text

import os

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader
import math
from sklearn.model_selection import train_test_split, RepeatedStratifiedKFold
import matplotlib.pyplot as plt


import numpy as np
import scipy.sparse as sp
import  open3d as o3d
from scipy.linalg import orthogonal_procrustes


data_list = ["Angry", "Disgust", "Fear", "Happy", "Sad", "Surprise"]
labels_map = {}
for i in range(6):
    labels_map[i] = data_list[i]

device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
EPS = 1e-6


class FaceData(Dataset):
    def __init__(self, data, label): 
        self.pts = data
        self.labels = label
        self.length = len(data)

        
        
    def __len__(self):

        return self.length

    def __getitem__(self, idx):
        pc = self.pts[idx]
     
     #   pc = np.reshape(self.pts[idx], (100, 83,3))  # 100x249 => 100x83x3
        # pc, coeff = on_unit_cube(pc)
        pc = np.reshape(pc, (100, 249))
        pc = torch.FloatTensor(pc)
        # s = torch.FloatTensor([coeff['scale']])
        # m = torch.FloatTensor(coeff['min'])
        y = self.labels[idx]
        return pc, y


class LSTMClassifier(nn.Module):

    def __init__(self, num_pts, num_frame, label_size):
        super(LSTMClassifier, self).__init__()
       # self.encoding_conv = nn.Linear(num_pts, 16)
        self.lstm_1 = nn.LSTM(249, 128, batch_first=True, bidirectional=True)
      #  self.lstm_2 = nn.LSTM(128, 128, batch_first=True, bidirectional=False)

       # self.w = nn.Parameter(torch.zeros(1024))
      #  self.linear = nn.Linear(128*2, 6)
        self.cls = nn.Linear(128*2, label_size)
        self.tanh1 = nn.Tanh()



    def forward(self, x):

        bs = x.shape[0]
 
      #  x = F.relu(self.encoding_conv(x))
        
        x, (hidden, cell) = self.lstm_1(x)
        
       # m = self.tanh1(lstm_out)
      #  alpha = F.softmax(torch.matmul(m, self.w), dim=1).unsqueeze(-1)
        hidden = torch.cat((hidden[-2,:,:], hidden[-1,:,:]), dim = 1)
        # out = lstm_out * alpha 
        # out = F.relu(torch.sum(out, 1))
      #  x = F.relu(self.linear(hidden))
        hidden = hidden.view(bs, -1)
        logits  = self.cls(hidden)
        return logits, hidden




def train_step(net,  dataloader, optimizer, criterion):
    net.train()
    total_loss = []
    correct = 0
    total_data = 0
    for data in dataloader:

        pts, label = data
        pts, label = pts.to(device), label.to(device)
        bs = pts.shape[0]
        total_data += bs
        logits, _ = net(pts)
     
        loss = criterion(logits, label)
        loss.backward()
        optimizer.step()
        optimizer.zero_grad()

        index_pred = torch.argmax(F.softmax(logits),  dim = 1)
       
        correct += torch.sum(index_pred == label).item()
        total_loss.append(loss.item())


    return np.mean(total_loss), correct/total_data



def valid_step(net,  dataloader, criterion):
    net.eval()
    total_loss = []
    correct = 0
    total_data = 0
    with torch.no_grad():
        for data in dataloader:
            pts, label = data
            pts, label = pts.to(device), label.to(device)
            bs = pts.shape[0]
            total_data += bs
            logits, _ = net(pts)
            loss = criterion(logits, label)



            index_pred = torch.argmax(F.softmax(logits),  dim = 1)
            correct += torch.sum(index_pred == label).item()
            total_loss.append(loss.item())

    return np.mean(total_loss), correct/total_data

num_frame = 100
num_points = 83
z = 50
c = 6
epochs = 30
lr = 0.0001
weight_decay = 0.0001

'''
data_path = '../dataset.pkl'
dataset = np.load(data_path, allow_pickle=True)
subject = np.array(dataset["data"])
labels =  np.array(dataset["labels"])
#subject[:, 0, :] = np.zeros((606,249))
subject_ = np.reshape(subject, (606*100 , 249))
m = subject_.mean(0)
std = subject_.std(0)

subject = (subject_-m)/std
subject = np.reshape(subject, (606, 100 , 249))
# data_pro = []
# for ani in subject:
#     sbj = []
#     for frame in ani:
#         if frame.max()-frame.min() == 0:
#             mtx2 = frame
#         else:
#             mtx1, mtx2, disparity = procrustes(m, frame)
#         sbj.append(mtx2)

#     data_pro.append(np.array(sbj))
# subject = np.array(data_pro)
# print(subject.shape)
# subject = np.reshape(subject, (60600,83, 3))
# subject = (subject-subject.mean(0))/subject.std(0)
# subject = np.reshape(subject, (606, 100,83, 3))

#subject = np.reshape(subject, (606,100,249))
random_seeds = 2021
test_size = 0.2
path = "classifier/"
if not os.path.exists(path):
    os.mkdir(path)

net = LSTMClassifier(3*num_points, num_frame, c).to(device)
criterion = nn.CrossEntropyLoss()
skf = RepeatedStratifiedKFold(n_splits=5, n_repeats=1, random_state = random_seeds) 
n = 0
results = []

for train_index, test_index in skf.split(subject, labels):
    n+=1
    
    train, valid= train_test_split(train_index, test_size=test_size, random_state = random_seeds)
    
    
    train_data = subject[train]
    train_labels = labels[train]

    print("number of train data:", train_data.shape[0])
    #mean, std, train_data = normalize(train_data)
    train_dataset = FaceData(train_data, train_labels)
    train_loader = DataLoader(train_dataset, batch_size=16, shuffle=True, num_workers=4)
    
    valid_data = subject[valid]
    valid_labels = labels[valid]
    print("number of valid data:", valid_data.shape[0])
   # _, _, valid_data = normalize(valid_data, mean = mean, std = std)
    valid_dataset = FaceData(valid_data, valid_labels)
    valid_loader = DataLoader(valid_dataset, batch_size=16, shuffle=True, num_workers=4)
    optimizer = torch.optim.Adam(net.parameters(), lr=lr, weight_decay=weight_decay)
#    std = torch.FloatTensor(std).unsqueeze(0).unsqueeze(0).to(device)
    std = 1
    best_loss = 1000
    for ep in range(epochs):
        train_loss, train_acc = train_step(net,  train_loader, optimizer, criterion)
        valid_loss, valid_acc = valid_step(net,  valid_loader, criterion)
    
        print("epoch {}, training: acc {}, loss {}".format(ep,   train_acc,  train_loss))
        print(" validation: acc {}, loss {}".format(   valid_acc, valid_loss))
        if best_loss > valid_loss:

            torch.save(net, 'classfier_'+str(n)+ '.pt')
    
    test_dataset = torch.load( 'classfier_'+str(n)+ '.pt')
    test_data = subject[test_index]
    test_labels = labels[test_index]
    print("number of test data:", test_data.shape[0])
   # _, _, valid_data = normalize(valid_data, mean = mean, std = std)
    test_dataset = FaceData(test_data, test_labels)
    test_loader = DataLoader(test_dataset, batch_size=16, shuffle=True, num_workers=4)
    test_loss, test_acc = valid_step(net,  test_loader, criterion)
    print(" test: acc {}, loss {}".format(   test_acc, test_loss))
    results.append(test_acc)
print("Final results: mean {}, std {}".format(np.mean(results), np.std(results)))
'''

```

## Source file: utils/visualization.py

{
  "path": "utils/visualization.py",
  "blob_sha": "1f8a0aac1fc3481c2feb9327ee62dc56802b5c76",
  "sha256": "aafeb6bb7609a789ef2ecac36a5ce5c08209f60c517901e086b8939863faf74b",
  "lines": 159,
  "bytes": 5658,
  "selection_score": 0,
  "keyword_line_counts": {
    "shader": 3,
    "face": 9
  },
  "url": "https://github.com/ZOUKaifeng/FacialExpressionGeneration/blob/b87720365dc259cb6d9595255f7e2dd7d0366139/utils/visualization.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def img_to_gif(image, path):"
    },
    {
      "line": 52,
      "text": "def set_render():"
    }
  ]
}

```text

import open3d as o3d
from PIL import Image, ImageDraw
import os
import numpy as np
from tqdm import tqdm
import os
import torch
import matplotlib.pyplot as plt

# Util function for loading meshes
from pytorch3d.io import load_objs_as_meshes, load_obj

# Data structures and functions for rendering
from pytorch3d.structures import Meshes
from pytorch3d.vis.plotly_vis import AxisArgs, plot_batch_individually, plot_scene
from pytorch3d.vis.texture_vis import texturesuv_image_matplotlib
from pytorch3d.renderer import (
    look_at_view_transform,
    FoVPerspectiveCameras, 
    PointLights, 
    DirectionalLights, 
    Materials, 
    RasterizationSettings, 
    MeshRenderer, 
    MeshRasterizer,  
    SoftPhongShader,
    TexturesUV,
    TexturesVertex
)
from pytorch3d.renderer.mesh import Textures
# add path for demo utils functions 
import sys
import os
sys.path.append(os.path.abspath(''))



if torch.cuda.is_available():
    device = torch.device("cuda:0")
    torch.cuda.set_device(device)
else:
    device = torch.device("cpu")


def img_to_gif(image, path):
    images = []
    for img in image:
        img = Image.fromarray(img, 'RGB')
        images.append(img)
    images[0].save(path+"/expression.gif", save_all=True, append_images=images[1:], optimize=False, duration=40, loop=0)

def set_render():
    # Initialize a camera.
    # With world coordinates +Y up, +X left and +Z in, the front of the cow is facing the -Z direction. 
    # So we move the camera by 180 in the azimuth direction so it is facing the front of the cow. 
    R, T = look_at_view_transform(2.7, 0, 0) 
    cameras = FoVPerspectiveCameras(device=device, R=R, T=T)

    # Define the settings for rasterization and shading. Here we set the output image to be of size
    # 512x512. As we are rendering images for visualization purposes only we will set faces_per_pixel=1
    # and blur_radius=0.0. We also set bin_size and max_faces_per_bin to None which ensure that 
    # the faster coarse-to-fine rasterization method is used. Refer to rasterize_meshes.py for 
    # explanations of these parameters. Refer to docs/notes/renderer.md for an explanation of 
    # the difference between naive and coarse-to-fine rasterization. 
    raster_settings = RasterizationSettings(
        image_size=512, 
        blur_radius=0.0, 
        faces_per_pixel=1, 
    )

    # Place a point light in front of the object. As mentioned above, the front of the cow is facing the 
    # -z direction. 
    lights = PointLights(device=device, location=[[0.0, 0.0, -3.0]])

    # Create a Phong renderer by composing a rasterizer and a shader. The textured Phong shader will 
    # interpolate the texture uv coordinates for each vertex, sample from a texture image and 
    # apply the Phong lighting model
    renderer = MeshRenderer(
        rasterizer=MeshRasterizer(
            cameras=cameras, 
            raster_settings=raster_settings
        ),
        shader=SoftPhongShader(
            device=device, 
            cameras=cameras,
            lights=lights
        )
    )

    return renderer

if __name__ == '__main__':
    renderer = set_render()

    data = {}
    from PIL import Image, ImageDraw
    from tqdm import tqdm
    data_path = "../data/BU_FACE/"
    BU_path = "/media/kzou/LaCie/FACE3D_DB/BU/Preprocessed/"
    if not os.path.exists("data"):
        os.mkdir("data")
    if not os.path.exists(data_path):
        os.mkdir(data_path)
    subject_list = os.listdir(BU_path)
    for sub in subject_list:
        subject_path = os.path.join(BU_path, sub)
        if sub == "LM_indices.txt":
            continue
        subjects = os.listdir(subject_path)
        

        for s in tqdm(subjects):
            data[s] = {}
            exp_path = os.path.join(subject_path, s)
            exp_lists = os.listdir(exp_path)
            
            save_sub_path = os.path.join(data_path, s)
            if not os.path.exists(save_sub_path):
                os.mkdir(save_sub_path)
            

            print(exp_path)
            for exp in exp_lists:
                data[s][exp] = {}
                frames_path = os.path.join(exp_path, exp)
                frame =  os.listdir(frames_path)
                LM = np.zeros((100, 68, 3))
                vert = []
            
            
                save_exp_path = os.path.join(save_sub_path, exp)
                if not os.path.exists(save_exp_path):
                    os.mkdir(save_exp_path) 

                count = 0
                imgs = []
                for f in frame:
                    f_ = f.split(".")
                    if len(f_)<2:
                        continue
                    if f_[1] == "_mesh":
                        count += 1
                        num = int(f_[0])
                        if num != count-1:
                            print(frames_path)
                        if count > 100:
                            break
                      
                        verts, faces, _ = load_obj(os.path.join(frames_path, f))
                        faces = faces.verts_idx
                        texture = loadmat(os.path.join(frames_path, f_[0]+'.mat'))
                        verts_tex = torch.tensor(texture["face_texture"]/255)[None]
                        textures = Textures(verts_rgb=verts_tex).to(device)
                        test_mesh = Meshes([verts], [faces], textures).cuda()
                        images = renderer(test_mesh)
                        images[images<0] = 0
                        imgs.append(images[0, ..., :3].cpu().numpy())
                print(save_exp_path)
                img_to_gif(imgs, save_exp_path)


```

# Facial rendering source scan: HuayuanSong/TeethSegModelling

Repository: https://github.com/HuayuanSong/TeethSegModelling

Pinned source revision: ce74e878976db7fcaf5b1ebc415bbb679ef37d99

Discovery topic: teeth 3D

Description: Repo containing modelling code for 3D Teeth Scan Segmentation project

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: train.py

{
  "path": "train.py",
  "blob_sha": "3356b8cab8fa1bc0a8a4082f85388d4a79b416d6",
  "sha256": "b0428d4101b0b520678d93423a2db635d6a2d15aad1d47714906721acdcada17",
  "lines": 166,
  "bytes": 6448,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/HuayuanSong/TeethSegModelling/blob/ce74e878976db7fcaf5b1ebc415bbb679ef37d99/train.py",
  "function_signature_candidates": []
}

```text

import os

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader

from mesh_dataset import *
from dl_models import *
from loss_func import *
import utils

if __name__ == '__main__':
    torch.cuda.set_device(utils.get_avail_gpu()) # assign which gpu will be used (only linux works)
    use_visdom = True # if you don't use visdom, please set to False

    train_list = './train_list.csv' # use 1-fold as example
    val_list = './val_list.csv' # use 1-fold as example

    model_path = 'models/'
    model_name = 'model' # need to define

    num_classes = 15
    num_channels = 15 #number of features
    num_epochs = 200
    num_workers = 0
    train_batch_size = 10
    val_batch_size = 10
    num_batches_to_print = 20

    if use_visdom:
        # set plotter
        global plotter
        plotter = utils.VisdomLinePlotter(env_name=model_name)

    # mkdir 'models'
    if not os.path.exists(model_path):
        os.mkdir(model_path)

    # set dataset
    training_dataset = Mesh_Dataset(data_list_path=train_list,
                                    num_classes=num_classes,
                                    patch_size=6000)
    val_dataset = Mesh_Dataset(data_list_path=val_list,
                               num_classes=num_classes,
                               patch_size=6000)

    train_loader = DataLoader(dataset=training_dataset,
                              batch_size=train_batch_size,
                              shuffle=True,
                              num_workers=num_workers)

    val_loader = DataLoader(dataset=val_dataset,
                            batch_size=val_batch_size,
                            shuffle=False,
                            num_workers=num_workers)

    # set model
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')

    # Option: Change to PointNet
    model = MeshSegNet(num_classes=num_classes, num_channels=num_channels, with_dropout=True, dropout_p=0.5).to(device, dtype=torch.float)
    opt = optim.Adam(model.parameters(), amsgrad=True)

    losses = []
    val_losses = []

    #cudnn
    torch.backends.cudnn.benchmark = True
    torch.backends.cudnn.enabled = True

    print('Training model...')
    class_weights = torch.ones(15).to(device, dtype=torch.float)
    for epoch in range(num_epochs):

        # training
        model.train()
        running_loss = 0.0
        loss_epoch = 0.0
        for i_batch, batched_sample in enumerate(train_loader):

            # send mini-batch to device
            inputs = batched_sample['cells'].to(device, dtype=torch.float)
            labels = batched_sample['labels'].to(device, dtype=torch.long)
            A_S = batched_sample['A_S'].to(device, dtype=torch.float)
            A_L = batched_sample['A_L'].to(device, dtype=torch.float)
            one_hot_labels = nn.functional.one_hot(labels[:, 0, :], num_classes=num_classes)

            # zero the parameter gradients
            opt.zero_grad()

            # forward + backward + optimize
            outputs = model(inputs, A_S, A_L)
            loss = Generalized_Dice_Loss(outputs, one_hot_labels, class_weights)
            loss.backward()
            opt.step()

            # print statistics
            running_loss += loss.item()
            loss_epoch += loss.item()
            if i_batch % num_batches_to_print == num_batches_to_print-1:  # print every N mini-batches
                print('[Epoch: {0}/{1}, Batch: {2}/{3}] loss: {4}'.format(epoch+1, num_epochs, i_batch+1, len(train_loader), running_loss/num_batches_to_print))
                if use_visdom:
                    plotter.plot('loss', 'train', 'Loss', epoch+(i_batch+1)/len(train_loader), running_loss/num_batches_to_print)
                running_loss = 0.0

        # record losses and metrics
        losses.append(loss_epoch/len(train_loader))

        #reset
        loss_epoch = 0.0

        # validation
        model.eval()
        with torch.no_grad():
            running_val_loss = 0.0
            val_loss_epoch = 0.0

            for i_batch, batched_val_sample in enumerate(val_loader):

                # send mini-batch to device
                inputs = batched_val_sample['cells'].to(device, dtype=torch.float)
                labels = batched_val_sample['labels'].to(device, dtype=torch.long)
                A_S = batched_val_sample['A_S'].to(device, dtype=torch.float)
                A_L = batched_val_sample['A_L'].to(device, dtype=torch.float)
                one_hot_labels = nn.functional.one_hot(labels[:, 0, :], num_classes=num_classes)

                outputs = model(inputs, A_S, A_L)
                loss = Generalized_Dice_Loss(outputs, one_hot_labels, class_weights)

                running_val_loss += loss.item()
                val_loss_epoch += loss.item()

                if i_batch % num_batches_to_print == num_batches_to_print-1:  # print every N mini-batches
                    print('[Epoch: {0}/{1}, Val batch: {2}/{3}] val_loss: {4}'.format(epoch+1, num_epochs, i_batch+1, len(val_loader), running_val_loss/num_batches_to_print))
                    running_val_loss = 0.0

            # record losses and metrics
            val_losses.append(val_loss_epoch/len(val_loader))

            # reset
            val_loss_epoch = 0.0

            # output current status
            print('*****\nEpoch: {}/{}, loss: {}\n         val_loss: {}\n*****'.format(epoch+1, num_epochs, losses[-1], val_losses[-1]))
            if use_visdom:
                plotter.plot('loss', 'train', 'Loss', epoch+1, losses[-1])
                plotter.plot('loss', 'val', 'Loss', epoch+1, val_losses[-1])

        # save the checkpoint
        #torch.save({'epoch': epoch+1,
                    #'model_state_dict': model.state_dict(),
                    #'optimizer_state_dict': opt.state_dict(),
                    #'losses': losses,
                    #'val_losses': val_losses},
                    #model_path+"checkpoint1")

        # save the best model
        if best_val_losses < val_losses[-1]:
            best_val_losses = val_losses[-1]
            torch.save({'epoch': epoch+1,
                        'model_state_dict': model.state_dict(),
                        'optimizer_state_dict': opt.state_dict(),
                        'losses': losses,
                        'val_losses': val_losses},
                        model_path+'{}_best.tar'.format(model_name))


```

## Source file: utils.py

{
  "path": "utils.py",
  "blob_sha": "58530ac521c6836cd9544cbdaed0ceb05b621ded",
  "sha256": "2b49828e91ae85989b07ae89122f54e30008c47264ad45a781f3dbe4645d4b23",
  "lines": 90,
  "bytes": 2647,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/HuayuanSong/TeethSegModelling/blob/ce74e878976db7fcaf5b1ebc415bbb679ef37d99/utils.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self, env_name='main'):"
    },
    {
      "line": 26,
      "text": "def plot(self, var_name, split_name, title_name, x, y):"
    },
    {
      "line": 49,
      "text": "def get_avail_gpu():"
    },
    {
      "line": 78,
      "text": "def load_json(file_path: str) -> dict:"
    }
  ]
}

```text

import os
import json

import numpy as np
import visdom

class VisdomLinePlotter(object):
    """
    A class for plotting data using Visdom.

    Attributes:
    env_name (str): Name of the Visdom environment.
    """

    def __init__(self, env_name='main'):
        """
        Initialize the VisdomLinePlotter.

        Args:
        env_name (str): The environment name for Visdom.
        """
        self.viz = visdom.Visdom()
        self.env = env_name
        self.plots = {}

    def plot(self, var_name, split_name, title_name, x, y):
        """
        Plot or update a line in a Visdom plot.

        Args:
        var_name (str): Name of the variable to plot.
        split_name (str): Name of the split (e.g., 'train', 'val').
        title_name (str): Title of the plot.
        x (int or float): X-coordinate.
        y (int or float): Y-coordinate.
        """
        if var_name not in self.plots:
            # Create a new plot if it doesn't exist
            self.plots[var_name] = self.viz.line(X=np.array([x,x]), Y=np.array([y,y]), env=self.env, opts=dict(
                legend=[split_name],
                title=title_name,
                xlabel='Epochs',
                ylabel=var_name
            ))
        else:
            # Update the existing plot
            self.viz.line(X=np.array([x]), Y=np.array([y]), env=self.env, win=self.plots[var_name], name=split_name, update='append')

def get_avail_gpu():
    """
    Detect the first available GPU on a Linux system using nvidia-smi.

    Returns:
    int: The index of the first available GPU, or 0 if none are available.
    """
    result = os.popen("nvidia-smi").readlines()
    try:
        # Find the line indicating GPU processes
        for i in range(len(result)):
            if 'Processes' in result[i]:
                process_idx = i
        # Count the number of GPUs
        num_gpu = 0
        for i in range(process_idx+1):
            if 'MiB' in result[i]:
                num_gpu += 1
        gpu_list = list(range(num_gpu))

        # Detect busy GPUs and remove them from the list
        for i in range(process_idx, len(result)):
            if result[i][22] == 'C':
                gpu_list.remove(int(result[i][5]))   
        return gpu_list[0]
    except:
        print('No GPU available, returning 0')
        return 0

def load_json(file_path: str) -> dict:
    """
    Load and return the contents of a JSON file.

    Parameters:
    file_path (str): Path to the JSON file.

    Returns:
    dict: The contents of the JSON file.
    """
    with open(file_path, "r") as f:
        json_file = json.load(f)
    return json_file


```

## Source file: dl_models.py

{
  "path": "dl_models.py",
  "blob_sha": "e3f53869117eafd4a7835db442ad9d0e5529bac1",
  "sha256": "feb7b35febd2a2787e8ab58cd515f60a57f05d7bb98a639b0609e03c1cd82124",
  "lines": 277,
  "bytes": 9444,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/HuayuanSong/TeethSegModelling/blob/ce74e878976db7fcaf5b1ebc415bbb679ef37d99/dl_models.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, channel):"
    },
    {
      "line": 34,
      "text": "def forward(self, x):"
    },
    {
      "line": 69,
      "text": "def __init__(self, k=64):"
    },
    {
      "line": 95,
      "text": "def forward(self, x):"
    },
    {
      "line": 131,
      "text": "def __init__(self, num_classes=15, num_channels=15, with_dropout=True, dropout_p=0.5):"
    },
    {
      "line": 199,
      "text": "def forward(self, x, a_s, a_l):"
    }
  ]
}

```text

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.autograd import Variable
from torchsummary import summary

## PointNet ##
class STN3d(nn.Module):
    def __init__(self, channel):
        """
        Initialize the STN3d module.

        Parameters:
        channel: The number of input channels.
        """
        super(STN3d, self).__init__()
        # Define convolutional and linear layers with batch normalization and ReLU activation
        self.conv1 = torch.nn.Conv1d(channel, 64, 1)
        self.conv2 = torch.nn.Conv1d(64, 128, 1)
        self.conv3 = torch.nn.Conv1d(128, 1024, 1)
        self.fc1 = nn.Linear(1024, 512)
        self.fc2 = nn.Linear(512, 256)
        self.fc3 = nn.Linear(256, 9)
        self.relu = nn.ReLU()

        # Batch normalization layers
        self.bn1 = nn.BatchNorm1d(64)
        self.bn2 = nn.BatchNorm1d(128)
        self.bn3 = nn.BatchNorm1d(1024)
        self.bn4 = nn.BatchNorm1d(512)
        self.bn5 = nn.BatchNorm1d(256)

    def forward(self, x):
        """
        Forward pass of the STN3d module.

        Parameters:
        x: Input tensor.

        Returns:
        Transformed tensor.
        """
        batchsize = x.size()[0]
        # Apply convolutional layers with ReLU and batch normalization
        x = F.relu(self.bn1(self.conv1(x)))
        x = F.relu(self.bn2(self.conv2(x)))
        x = F.relu(self.bn3(self.conv3(x)))

        # Apply max pooling and reshape
        x = torch.max(x, 2, keepdim=True)[0]
        x = x.view(-1, 1024)

        # Apply linear layers with ReLU and batch normalization
        x = F.relu(self.bn4(self.fc1(x)))
        x = F.relu(self.bn5(self.fc2(x)))
        x = self.fc3(x)

        # Create identity matrix and add to the tensor
        iden = Variable(torch.from_numpy(np.array([1, 0, 0, 0, 1, 0, 0, 0, 1]).astype(np.float32))).view(1, 9).repeat(batchsize, 1)
        if x.is_cuda:
            iden = iden.to(x.get_device())
        x = x + iden
        x = x.view(-1, 3, 3)

        return x

class STNkd(nn.Module):
    def __init__(self, k=64):
        """
        Initialize the STNkd module.

        Parameters:
        k: Dimension size of the transformation matrix.
        """
        super(STNkd, self).__init__()
        # Define convolutional and linear layers with batch normalization and ReLU activation
        self.conv1 = torch.nn.Conv1d(k, 64, 1)
        self.conv2 = torch.nn.Conv1d(64, 128, 1)
        self.conv3 = torch.nn.Conv1d(128, 512, 1)
        self.fc1 = nn.Linear(512, 256)
        self.fc2 = nn.Linear(256, 128)
        self.fc3 = nn.Linear(128, k * k)
        self.relu = nn.ReLU()

        # Batch normalization layers
        self.bn1 = nn.BatchNorm1d(64)
        self.bn2 = nn.BatchNorm1d(128)
        self.bn3 = nn.BatchNorm1d(512)
        self.bn4 = nn.BatchNorm1d(256)
        self.bn5 = nn.BatchNorm1d(128)

        self.k = k

    def forward(self, x):
        """
        Forward pass of the STNkd module.

        Parameters:
        x: Input tensor.

        Returns:
        Transformed tensor.
        """
        batchsize = x.size()[0]
        # Apply convolutional layers with ReLU and batch normalization
        x = F.relu(self.bn1(self.conv1(x)))
        x = F.relu(self.bn2(self.conv2(x)))
        x = F.relu(self.bn3(self.conv3(x)))

        # Apply max pooling and reshape
        x = torch.max(x, 2, keepdim=True)[0]
        x = x.view(-1, 512)

        # Apply linear layers with ReLU and batch normalization
        x = F.relu(self.bn4(self.fc1(x)))
        x = F.relu(self.bn5(self.fc2(x)))
        x = self.fc3(x)

        # Create identity matrix and add to the tensor
        iden = Variable(torch.from_numpy(np.eye(self.k).flatten().astype(np.float32))).view(1, self.k * self.k).repeat(batchsize, 1)
        if x.is_cuda:
            iden = iden.to(x.get_device())
        x = x + iden
        x = x.view(-1, self.k, self.k)

        return x

## MeshSegNet ##
class MeshSegNet(nn.Module):
    def __init__(self, num_classes=15, num_channels=15, with_dropout=True, dropout_p=0.5):
        """
        Initialize the MeshSegNet module.

        Parameters:
        num_classes: Number of output classes.
        num_channels: Number of input channels.
        with_dropout: Indicates whether to use dropout.
        dropout_p: Dropout probability.
        """
        super(MeshSegNet, self).__init__()
        self.num_classes = num_classes
        self.num_channels = num_channels
        self.with_dropout = with_dropout
        self.dropout_p = dropout_p

        # Define layers for MLP-1 with batch normalization
        self.mlp1_conv1 = torch.nn.Conv1d(self.num_channels, 64, 1)
        self.mlp1_conv2 = torch.nn.Conv1d(64, 64, 1)
        self.mlp1_bn1 = nn.BatchNorm1d(64)
        self.mlp1_bn2 = nn.BatchNorm1d(64)

        # Feature-transformer module (FTM)
        self.fstn = STNkd(k=64)

        # Graph-constrained learning module GLM-1
        self.glm1_conv1_1 = torch.nn.Conv1d(64, 32, 1)
        self.glm1_conv1_2 = torch.nn.Conv1d(64, 32, 1)
        self.glm1_bn1_1 = nn.BatchNorm1d(32)
        self.glm1_bn1_2 = nn.BatchNorm1d(32)
        self.glm1_conv2 = torch.nn.Conv1d(32+32, 64, 1)
        self.glm1_bn2 = nn.BatchNorm1d(64)

        # Define layers for MLP-2 with batch normalization
        self.mlp2_conv1 = torch.nn.Conv1d(64, 64, 1)
        self.mlp2_bn1 = nn.BatchNorm1d(64)
        self.mlp2_conv2 = torch.nn.Conv1d(64, 128, 1)
        self.mlp2_bn2 = nn.BatchNorm1d(128)
        self.mlp2_conv3 = torch.nn.Conv1d(128, 512, 1)
        self.mlp2_bn3 = nn.BatchNorm1d(512)

        # Graph-constrained learning module GLM-2
        self.glm2_conv1_1 = torch.nn.Conv1d(512, 128, 1)
        self.glm2_conv1_2 = torch.nn.Conv1d(512, 128, 1)
        self.glm2_conv1_3 = torch.nn.Conv1d(512, 128, 1)
        self.glm2_bn1_1 = nn.BatchNorm1d(128)
        self.glm2_bn1_2 = nn.BatchNorm1d(128)
        self.glm2_bn1_3 = nn.BatchNorm1d(128)
        self.glm2_conv2 = torch.nn.Conv1d(128*3, 512, 1)
        self.glm2_bn2 = nn.BatchNorm1d(512)

        # Define layers for MLP-3 with batch normalization
        self.mlp3_conv1 = torch.nn.Conv1d(64+512+512+512, 256, 1)
        self.mlp3_conv2 = torch.nn.Conv1d(256, 256, 1)
        self.mlp3_bn1_1 = nn.BatchNorm1d(256)
        self.mlp3_bn1_2 = nn.BatchNorm1d(256)
        self.mlp3_conv3 = torch.nn.Conv1d(256, 128, 1)
        self.mlp3_conv4 = torch.nn.Conv1d(128, 128, 1)
        self.mlp3_bn2_1 = nn.BatchNorm1d(128)
        self.mlp3_bn2_2 = nn.BatchNorm1d(128)

        # Output convolutional layer
        self.output_conv = torch.nn.Conv1d(128, self.num_classes, 1)

        # Dropout layer
        if self.with_dropout:
            self.dropout = nn.Dropout(p=self.dropout_p)

    def forward(self, x, a_s, a_l):
        """
        Forward pass of the MeshSegNet module.

        Parameters:
        x: Input tensor.
        a_s: Adjacency matrix for small radius.
        a_l: Adjacency matrix for
        """
        batchsize = x.size()[0]
        n_pts = x.size()[2]

        # MLP-1
        x = F.relu(self.mlp1_bn1(self.mlp1_conv1(x)))
        x = F.relu(self.mlp1_bn2(self.mlp1_conv2(x)))

        # FTM
        trans_feat = self.fstn(x)
        x = x.transpose(2, 1)
        x_ftm = torch.bmm(x, trans_feat)

        # GLM-1
        sap = torch.bmm(a_s, x_ftm)
        sap = sap.transpose(2, 1)
        x_ftm = x_ftm.transpose(2, 1)
        x = F.relu(self.glm1_bn1_1(self.glm1_conv1_1(x_ftm)))
        glm_1_sap = F.relu(self.glm1_bn1_2(self.glm1_conv1_2(sap)))
        x = torch.cat([x, glm_1_sap], dim=1)
        x = F.relu(self.glm1_bn2(self.glm1_conv2(x)))

        # MLP-2
        x = F.relu(self.mlp2_bn1(self.mlp2_conv1(x)))
        x = F.relu(self.mlp2_bn2(self.mlp2_conv2(x)))
        x_mlp2 = F.relu(self.mlp2_bn3(self.mlp2_conv3(x)))
        if self.with_dropout:
            x_mlp2 = self.dropout(x_mlp2)

        # GLM-2
        x_mlp2 = x_mlp2.transpose(2, 1)
        sap_1 = torch.bmm(a_s, x_mlp2)
        sap_2 = torch.bmm(a_l, x_mlp2)
        x_mlp2 = x_mlp2.transpose(2, 1)
        sap_1 = sap_1.transpose(2, 1)
        sap_2 = sap_2.transpose(2, 1)
        x = F.relu(self.glm2_bn1_1(self.glm2_conv1_1(x_mlp2)))
        glm_2_sap_1 = F.relu(self.glm2_bn1_2(self.glm2_conv1_2(sap_1)))
        glm_2_sap_2 = F.relu(self.glm2_bn1_3(self.glm2_conv1_3(sap_2)))
        x = torch.cat([x, glm_2_sap_1, glm_2_sap_2], dim=1)
        x_glm2 = F.relu(self.glm2_bn2(self.glm2_conv2(x)))

        # GMP
        x = torch.max(x_glm2, 2, keepdim=True)[0]

        # Upsample
        x = torch.nn.Upsample(n_pts)(x)

        # Dense fusion
        x = torch.cat([x, x_ftm, x_mlp2, x_glm2], dim=1)

        # MLP-3
        x = F.relu(self.mlp3_bn1_1(self.mlp3_conv1(x)))
        x = F.relu(self.mlp3_bn1_2(self.mlp3_conv2(x)))
        x = F.relu(self.mlp3_bn2_1(self.mlp3_conv3(x)))
        if self.with_dropout:
            x = self.dropout(x)
        x = F.relu(self.mlp3_bn2_2(self.mlp3_conv4(x)))

        # output
        x = self.output_conv(x)
        x = x.transpose(2,1).contiguous()
        x = torch.nn.Softmax(dim=-1)(x.view(-1, self.num_classes))
        x = x.view(batchsize, n_pts, self.num_classes)

        return x

if __name__ == '__main__':
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    model = MeshSegNet().to(device)
    summary(model, [(15, 6000), (6000, 6000), (6000, 6000)])


```

## Source file: loss_func.py

{
  "path": "loss_func.py",
  "blob_sha": "3a82f2ea2a0feb74c4a7701467d33036f56fdcfa",
  "sha256": "533d75d5b6e9aeedd5582fc0734755fe37ed7cfc917194bd5d5a95bce1814e1b",
  "lines": 150,
  "bytes": 5436,
  "selection_score": 0,
  "keyword_line_counts": {
    "tooth": 11
  },
  "url": "https://github.com/HuayuanSong/TeethSegModelling/blob/ce74e878976db7fcaf5b1ebc415bbb679ef37d99/loss_func.py",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "def points_distance(points, labels, n_classes):"
    },
    {
      "line": 43,
      "text": "def distance_count(points, y_pre_max):"
    },
    {
      "line": 69,
      "text": "def smooth_Loss(y_pred, inputs):"
    },
    {
      "line": 90,
      "text": "def Generalized_Dice_Loss(y_pred, y_true, class_weights, inputs, smooth=1.0):"
    }
  ]
}

```text

import numpy as np
import torch

def points_distance(points, labels, n_classes):
    """
    Calculate the maximum distance of points in each class from their mean.

    Parameters:
    points: A numpy array of points.
    labels: Corresponding labels for each point.
    n_classes: The number of unique classes.

    Returns:
    A dictionary with the class label as key and the maximum distance from the mean as value.
    """
    label_list = [n for n in range(0, n_classes)]
    tooth_dic = {}
    tooth_distance = {}

    # Loop through each class label and calculate distances.
    for i in label_list:
        key = i
        tooth_list = []
        distance = []

        # Collect points belonging to the current class label.
        for k in range(len(labels)):
            if labels[k] == i:
                tooth_list.append(points[k])

        # If the class label is present, calculate the maximum distance from mean.
        if tooth_list != []:
            mean = np.mean(tooth_list, axis=0)
            tooth_dic[key] = mean
            for point in tooth_list:
                distance.append(np.linalg.norm(point - mean, ord=2))
            distance.sort(reverse=True)
            tooth_distance[key] = distance[0]
        else:
            tooth_distance[key] = 0  # Assign 0 if the class label is missing.
    return tooth_distance

def distance_count(points, y_pre_max):
    """
    Compute the smoothness loss for points based on their distances.

    Parameters:
    points: A tensor of points.
    y_pre_max: Tensor representing the maximum predicted values.

    Returns:
    The smoothness loss for the given points.
    """
    r = 0.2
    L_smooth = 0.
    device = points.device
    one_array_points = torch.ones(points.shape).to(device)
    one_array_pred = torch.ones(y_pre_max.values.shape).to(device)

    # Calculate the smoothness loss.
    for i in range(0, points.shape[0]):
        distance_array = torch.linalg.norm((points - one_array_points * points[i]), ord=2, axis=1, keepdims=True)
        distance_sort, idx = torch.sort(distance_array)
        distance_array_pred = torch.le(distance_array, distance_sort[15])
        n = torch.count_nonzero(distance_sort[0:15])
        L_smooth += torch.linalg.norm((y_pre_max.values.reshape(distance_array_pred.shape) * distance_array_pred - y_pre_max.values[i] * one_array_pred.reshape(distance_array_pred.shape) * distance_array_pred), ord=2)/n
    return L_smooth/points.shape[0]

def smooth_Loss(y_pred, inputs):
    """
    Calculate the smoothness loss for a batch of predictions and inputs.

    Parameters:
    y_pred: Predicted values.
    inputs: Input values.

    Returns:
    The total smoothness loss for the batch.
    """
    batch_size = y_pred.shape[0]
    L_smooth = 0.

    # Accumulate the smoothness loss for each item in the batch.
    for number_patient in range(0, batch_size):
        y_pre_max = torch.max(y_pred[number_patient, :], 1)
        points = inputs[number_patient, 9:12, :].T
        L_smooth += distance_count(points, y_pre_max)
    return L_smooth

def Generalized_Dice_Loss(y_pred, y_true, class_weights, inputs, smooth=1.0):
    """
    Calculate the Generalized Dice Loss for given predictions and true values.

    Parameters:
    y_pred: Predicted probabilities [n_classes, x, y, z].
    y_true: True one-hot encoded values [n_classes, x, y, z].
    class_weights: Weights for each class.
    smooth: Smoothing parameter to avoid division by zero.

    Returns:
    The total loss combining dice loss, size constraint, and smoothness loss.
    """
    smooth = 1e-7
    Lambda_size = 0.1
    Lambda_smooth = 2.5
    weight_smooth = 1000
    size_loss = 0.
    loss = 0.
    n_classes = y_pred.shape[-1]
    batch_size = y_pred.shape[0]
    y_pre_np = y_pred.cpu().detach().numpy()
    y_tru_np = y_true.cpu().detach().numpy()

    # Calculate smoothness loss.
    L_smooth = smooth_Loss(y_pred, inputs)
    L_smooth = L_smooth * weight_smooth

    # Calculate size constraint loss.
    L_pre = []
    L_gt = []
    predicted_labels = np.zeros((y_pred.shape[1]))
    true_labels = np.zeros((y_true.shape[1]))
    for number_patient in range(0, batch_size):
        for i_label in range(n_classes):
            predicted_labels[np.argmax(y_pre_np[number_patient, :], axis=-1) == i_label] = i_label
            true_labels[np.argmax(y_tru_np[number_patient, :], axis=-1) == i_label] = i_label
        points = inputs[number_patient, 9:12, :].cpu().detach().numpy().T
        L_pre_dic = points_distance(points, predicted_labels, n_classes)
        L_gt_dic = points_distance(points, true_labels, n_classes)
        for elem in L_gt_dic:
            if elem == 0:
                continue
            else:
                L_pre.append(L_pre_dic[elem])
                L_gt.append(L_gt_dic[elem])
        size_loss += np.linalg.norm(np.array(L_gt) - np.array(L_pre), ord=2)
    size_loss = Lambda_size * size_loss

    # Calculate dice loss with weight.
    for c in range(0, n_classes):
        pred_flat = y_pred[:, :, c].reshape(-1)
        true_flat = y_true[:, :, c].reshape(-1)
        intersection = (pred_flat * true_flat).sum()
        w = class_weights[c] / class_weights.sum()
        loss += w * (1 - ((2. * intersection + smooth) /
                          (pred_flat.sum() + true_flat.sum() + smooth)))

    # Combine all losses.
    all_loss = (loss + size_loss + L_smooth)/2
    return all_loss


```

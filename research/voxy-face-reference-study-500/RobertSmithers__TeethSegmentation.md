# Facial rendering source scan: RobertSmithers/TeethSegmentation

Repository: https://github.com/RobertSmithers/TeethSegmentation

Pinned source revision: e435da664fde5e7dfb1f7a032d1c026f9ea6f075

Discovery topic: teeth 3D

Description: Identify teeth in 3D using segmentation and labelling.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"teeth": ["X-Ray Teeth Segmentation.pptx"]}

## Source file: model.py

{
  "path": "model.py",
  "blob_sha": "cb784ad474412cdea8dd5c3a75dc4a088cb33d69",
  "sha256": "5b0c75107f78990e98d72ae1fe3803d75d6e610c20529696345198f53a0f05ec",
  "lines": 196,
  "bytes": 8244,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RobertSmithers/TeethSegmentation/blob/e435da664fde5e7dfb1f7a032d1c026f9ea6f075/model.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, in_ch, out_ch, kernel_size, batch_norm=True):"
    },
    {
      "line": 32,
      "text": "def forward(self, x):"
    },
    {
      "line": 39,
      "text": "def __init__(self, pool_kernel_size=(2,2), conv_in_ch=32, conv_out_ch=32, conv_kernel_size=(3,3), batch_norm=True):"
    },
    {
      "line": 44,
      "text": "def forward(self, x):"
    },
    {
      "line": 52,
      "text": "def __init__(self, in_ch, out_ch, kernel_size, stride=None):"
    },
    {
      "line": 59,
      "text": "def forward(self, x, skip):"
    },
    {
      "line": 65,
      "text": "def __init__(self, input_shape=(512,512,1)):"
    },
    {
      "line": 149,
      "text": "def forward(self, x):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F

# NOTE: On torch version 1.7 (which I was previously on), the "same" padding option was not valid for Conv2d. Requires torch v1.10 and up

class DoubleConvBlock(nn.Module):
  # (conv) => (dropout+ReLU) => (conv) => (bn+ReLU)

  def __init__(self, in_ch, out_ch, kernel_size, batch_norm=True):
    super(DoubleConvBlock, self).__init__()

    if batch_norm:
      self.conv = nn.Sequential(
          nn.Conv2d(in_ch, out_ch, 3, padding="same"),
          nn.Dropout2d(0.1),
          nn.BatchNorm2d(out_ch),
          # nn.LeakyReLU(inplace=True),
          nn.Conv2d(out_ch, out_ch, 3, padding="same"),
          nn.BatchNorm2d(out_ch),             # Heavily debated, but have decided to normalize before activation
          nn.LeakyReLU(inplace=True)
      )
    else:
      self.conv = nn.Sequential(
          nn.Conv2d(in_ch, out_ch, 3, padding="same"),
          nn.Dropout2d(0.1),
          nn.LeakyReLU(inplace=True),
          nn.Conv2d(out_ch, out_ch, 3, padding="same"),
          nn.LeakyReLU(inplace=True)
      )

  def forward(self, x):
      return self.conv(x)

# This is our encoder
class PoolConvBlock(nn.Module):
  # (Pool) => (DoubleConvBlock)

  def __init__(self, pool_kernel_size=(2,2), conv_in_ch=32, conv_out_ch=32, conv_kernel_size=(3,3), batch_norm=True):
    super(PoolConvBlock, self).__init__()
    self.pool = nn.MaxPool2d(pool_kernel_size)
    self.conv = DoubleConvBlock(conv_in_ch, conv_out_ch, conv_kernel_size, batch_norm=batch_norm)

  def forward(self, x):
    p = self.pool(x)
    x = self.conv(p)
    return x, p

class TConv(nn.Module):
  # (ConvTranspose) => (ReLU)

  def __init__(self, in_ch, out_ch, kernel_size, stride=None):
    super(TConv, self).__init__()
    self.conv = nn.Sequential(
          nn.ConvTranspose2d(in_ch, out_ch, kernel_size, padding=1, stride=stride),
          # nn.LeakyReLU(inplace=True)
      )
  
  def forward(self, x, skip):
    x = self.conv(x)
    x = torch.cat([x, skip], axis=1)
    return x

class UNet(nn.Module):
    def __init__(self, input_shape=(512,512,1)):
        super(UNet, self).__init__()
        
        # conv1 = Conv2D(32,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(inputs)
        # d1=Dropout(0.1)(conv1)
        # conv2 = Conv2D(32,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d1)
        # b=BatchNormalization()(conv2)
        self.conv = DoubleConvBlock(1, 32, (3,3))
        
        # pool1 = MaxPooling2D(pool_size=(2, 2))(b)
        # conv3 = Conv2D(64,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(pool1)
        # d2=Dropout(0.2)(conv3)
        # conv4 = Conv2D(64,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d2)
        # b1=BatchNormalization()(conv4)
        self.pconv1 = PoolConvBlock(conv_in_ch=32, conv_out_ch=64)

        # pool2 = MaxPooling2D(pool_size=(2, 2))(b1)
        # conv5 = Conv2D(128,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(pool2)
        # d3=Dropout(0.3)(conv5)
        # conv6 = Conv2D(128,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d3)
        # b2=BatchNormalization()(conv6)
        self.pconv2 = PoolConvBlock(conv_in_ch=64, conv_out_ch=128)
        
        # pool3 = MaxPooling2D(pool_size=(2, 2))(b2)
        # conv7 = Conv2D(256,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(pool3)
        # d4=Dropout(0.4)(conv7)
        # conv8 = Conv2D(256,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d4)
        # b3=BatchNormalization()(conv8)
        self.pconv3 = PoolConvBlock(conv_in_ch=128, conv_out_ch=256)
        
        # pool4 = MaxPooling2D(pool_size=(2, 2))(b3)
        # conv9 = Conv2D(512,(3,3),activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(pool4)
        # d5=Dropout(0.5)(conv9)
        # conv10 = Conv2D(512,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d5)
        # b4=BatchNormalization()(conv10)
        self.pconv4 = PoolConvBlock(conv_in_ch=256, conv_out_ch=512)
        
        # conv11 = Conv2DTranspose(512,(4,4), activation = 'relu', padding = 'same', strides=(2,2),kernel_initializer = 'he_normal')(b4)
        self.tconv1 = TConv(512, 256, (4,4), stride=(2,2))
        # x= concatenate([conv11,conv8])

        # conv12 = Conv2D(256,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(x)
        # d6=Dropout(0.4)(conv12)
        # conv13 = Conv2D(256,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d6)
        # b5=BatchNormalization()(conv13)
        self.conv1 = DoubleConvBlock(512, 256, (3,3))
        
        # conv14 = Conv2DTranspose(256,(4,4), activation = 'relu', padding = 'same', strides=(2,2),kernel_initializer = 'he_normal')(b5)
        self.tconv2 = TConv(256, 128, (4,4), stride=(2,2))
        # x1=concatenate([conv14,conv6])

        # conv15 = Conv2D(128,3, activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(x1)
        # d7=Dropout(0.3)(conv15)
        # conv16 = Conv2D(128,3, activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d7)
        # b6=BatchNormalization()(conv16)
        self.conv2 = DoubleConvBlock(256, 128, 3)
        
        # conv17 = Conv2DTranspose(128,(4,4), activation = 'relu', padding = 'same',strides=(2,2), kernel_initializer = 'he_normal')(b6)
        self.tconv3 = TConv(128, 64, (4,4), stride=(2,2))
        
        # x2=concatenate([conv17,conv4])

        # conv18 = Conv2D(64,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(x2)
        # d8=Dropout(0.2)(conv18)
        # conv19 = Conv2D(64,(3,3) ,activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d8)
        # b7=BatchNormalization()(conv19)
        self.conv3 = DoubleConvBlock(128, 64, 3)

        # conv20 = Conv2DTranspose(64,(4,4), activation = 'relu', padding = 'same',strides=(2,2), kernel_initializer = 'he_normal')(b7)
        self.tconv4 = TConv(64, 32, (4,4), stride=(2,2))
        
        # x3=concatenate([conv20,conv2])

        # conv21 = Conv2D(32,(3,3) ,activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(x3)
        # d9=Dropout(0.1)(conv21)
        # conv22 = Conv2D(32,(3,3), activation = 'relu', padding = 'same', kernel_initializer = 'he_normal')(d9)
        self.conv4 = DoubleConvBlock(64, 32, (3,3), batch_norm=False)

        self.conv5 = nn.Sequential(
            nn.Conv2d(32, 1, (1,1), padding="same"),
            nn.ReLU()
        )
        # outputs = Conv2D(1,(1,1), activation = last_activation, padding = 'same', kernel_initializer = 'he_normal')(conv22)

    def forward(self, x):

      ### Reduce ###
      
      # Outputs 512 x 512 x 32
      skip1 = self.conv(x)
      # print("level 1 layer+skip", skip1.shape)

      # Outputs 256 x 256 x 64
      skip2, layer2 = self.pconv1(skip1)
      # print("level 2 layer/skip", layer2.shape, skip2.shape)

      # Outputs 128 x 128 x 128
      skip3, layer3 = self.pconv2(skip2)
      # print("level 3 layer/skip", layer3.shape, skip3.shape)

      # Outputs 64 x 64 x 256
      skip4, layer4 = self.pconv3(skip3)
      # print("level 4 layer/skip", layer4.shape, skip4.shape)
      
      # Outputs 32 x 32 x 512
      skip5, x = self.pconv4(skip4)
      # print("level 5 channels", x.shape)

      ### Expand ###
      
      # Outputs 64 x 64 x 256
      expand4 = self.tconv1(skip5, skip4)
      expand4 = self.conv1(expand4)

      # Outputs 128 x 128 x 128
      expand3 = self.tconv2(expand4, skip3)
      expand3 = self.conv2(expand3)

      # Outputs 256 x 256 x 64
      expand2 = self.tconv3(expand3, skip2)
      expand2 = self.conv3(expand2)

      # Outputs 512 x 512 x 32
      expand1 = self.tconv4(expand2, skip1)
      expand1 = self.conv4(expand1)

      # Outputs 512 x 512 x 1
      out = self.conv5(expand1)

      # return torch.sigmoid(x)
      # No sigmoid because using BCEWithLogistLoss which is more numerically stable (using log-sum-exp trick)
      return out

```

## Source file: loader.py

{
  "path": "loader.py",
  "blob_sha": "7ac885b805824ff2720c9a4e561f8fe41cd86c88",
  "sha256": "4233bc007037a3461f7081dcd4ee6c9832bdf9a7efd9ca2610b92bbbe92044d2",
  "lines": 166,
  "bytes": 6192,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 6,
    "lip": 1
  },
  "url": "https://github.com/RobertSmithers/TeethSegmentation/blob/e435da664fde5e7dfb1f7a032d1c026f9ea6f075/loader.py",
  "function_signature_candidates": [
    {
      "line": 77,
      "text": "def get_fnames(root):"
    },
    {
      "line": 88,
      "text": "def __init__(self, data, root_dir='data', train=False, transformX = None, transformY = None, batch_transforms = 3):"
    },
    {
      "line": 97,
      "text": "def __len__(self):"
    },
    {
      "line": 100,
      "text": "def __getitem__(self, index):"
    }
  ]
}

```text

import os
import numpy as np
import cv2
from PIL import Image

import matplotlib.pyplot as plt

from sklearn.model_selection import train_test_split

from torch.utils.data  import Dataset, DataLoader
from torchvision import transforms
import torchvision.transforms.functional as TF
import torch.nn as nn
import torch.nn.functional as F

# Extra augmentation (blur, noise, brightness)
# import albumentations as A


###Pre-Process Images###
# def convert_one_channel(img):
#     #some images have 3 channels , although they are grayscale image
#     if len(img.shape)>2:
#         img=img[:,:,0]
#         return img
#     else:
#         return img

# def pre_images(resize_shape,path):
#     img=Image.open(path)
#     img=img.resize((resize_shape),Image.ANTIALIAS)
#     img=convert_one_channel(np.asarray(img))
#     cv2.imwrite(path,img)
#     return img

# def show_imgs():
#     fig = plt.figure(figsize = (30,7))
#     for index in range(7):
#       file_path1 = os.path.join('original_img', str(index+1)+'.png')
#       file_path2 = os.path.join('masked_img', str(index+1)+'.png')
#       # print(file_path)
#       ax = fig.add_subplot(2, 7, index+1)
#       plt.imshow(pre_images((512,512),file_path1))  #show result of converting every img to one color channel

#       ax = fig.add_subplot(2, 7, index+8)
#       plt.imshow(cv2.imread(file_path2))


# def gen_csv():
#     ###generate file names to keep track of file for later usage
#     arr1=np.arange(1,117)
#     # print(arr1.dtype)
#     arr1=arr1.astype(str)
#     # print(type(arr1))
#     df=pd.DataFrame(arr1)
#     df[1]=df[0]
#     df[0]=df[0]+'.png'
#     df.to_csv('data/sample.csv',index=False)
###Load DATA

# def rename():
#     ###rename all files in masked_img folder
#     folder='data/masked_img'
#     for file_name in os.listdir(folder):
#         source = folder+'/'+file_name
#         destination = source.replace('_m.png','.png')
#         os.rename(source, destination)

# def mergefiles():
#     ###move renamed files to Images folder
#     folder='data/masked_img'
#     for file_name in os.listdir(folder):
#       source = folder+'/'+file_name
#       destination = 'data/original_img/'+file_name
#       os.rename(source,destination)

def get_fnames(root):
  xs, ys = os.listdir(os.path.join(root, 'original_img')), os.listdir(os.path.join(root, 'masked_img'))
  f = lambda fname: int(fname.split('.png')[0])
  xs = sorted(xs, key=f)
#   ys = sorted(ys, key=f)
  return xs

# our dataset class
rest_set_size = 0.3
test_set_size = 0.5
class dset(Dataset):
    def __init__(self, data, root_dir='data', train=False, transformX = None, transformY = None, batch_transforms = 3):

      self.root_dir = root_dir
      self.transformX = transformX
      self.transformY = transformY
      self.train = train
      self.data = data
      self.batch_transforms = batch_transforms

    def __len__(self):
        return len(self.data)

    def __getitem__(self, index):
        fname = self.data[index]

        imx_name = os.path.join(self.root_dir, 'original_img', fname)
        imy_name = os.path.join(self.root_dir, 'masked_img', fname)
        imx = Image.open(imx_name)
        imy = Image.open(imy_name).convert('L')
        
        # 3104 and 1200 need to be divisible by 16 to work in our model.
        imx=imx.resize((512,512), Image.ANTIALIAS)
        imy=imy.resize((512,512), Image.ANTIALIAS)

        ##data augmentation
#         if self.train:
            
#           for _ in range(self.batch_transforms):
#             # Set of augmentations shown to be successful in SerdarHelli's usage in X-Ray imagery
#             aug = A.Compose([
#                     A.OneOf([A.RandomCrop(height=512, width=512),
#                                 A.PadIfNeeded(min_height=512, min_width=512, p=0.5)],p=0.4),
#                     A.RandomBrightnessContrast(brightness_limit=0.25, contrast_limit=0.25,p=0.5),
#                     A.Compose([A.RandomScale(scale_limit=(-0.15, 0.15), p=1, interpolation=1),
#                                             A.PadIfNeeded(min_height=512, min_width=512, border_mode=cv2.BORDER_CONSTANT), 
#                                             A.Resize(512, 512, cv2.INTER_NEAREST), ],p=0.5),
#                     A.ShiftScaleRotate (shift_limit=0.325, scale_limit=0.15, rotate_limit=15,border_mode=cv2.BORDER_CONSTANT, p=1),
#                     A.Rotate(15,p=0.5),
#                     A.Blur(blur_limit=1, p=0.5),
#                     A.Downscale(scale_min=0.15, scale_max=0.25,  always_apply=False, p=0.5),
#                     A.GaussNoise(var_limit=(0.05, 0.1), mean=0, per_channel=True, always_apply=False, p=0.5),
#                     A.HorizontalFlip(p=0.25),
#             ])
            
#             augd = aug(image=np.array(imx), mask=np.array(imy))
#             imx, imy = augd['image'], augd['mask']
            
#             del augd
#             del aug

        if self.transformX:
            imx = self.transformX(imx)
            imy = self.transformY(imy)

        sample = {'image': imx, 'annotation': imy}
        return sample

tx_X = transforms.Compose([
                              transforms.ToTensor(),
                              transforms.Normalize((0.5,), (0.5,))
                              ])
tx_Y = transforms.Compose([
                              transforms.ToTensor()
                              ])

all_data = get_fnames(root='data')

# split the dataset to train and rest
# split the rest to validation and test
train_data, other_data = train_test_split(all_data, test_size = rest_set_size, random_state = 5)
val_data, test_data = train_test_split(other_data, test_size = test_set_size, random_state = 5)

train_set = dset(train_data, 'data', train = True, transformX = tx_X, transformY = tx_Y)
val_set = dset(val_data, 'data', transformX = tx_X, transformY = tx_Y)
test_set = dset(test_data, 'data', transformX = tx_X, transformY = tx_Y)

train_loader = DataLoader(dataset=train_set, batch_size=2, shuffle=True, num_workers=1)
val_loader = DataLoader(dataset=val_set, batch_size=1, shuffle=True, num_workers=1)
test_loader = DataLoader(dataset=test_set, batch_size=1, shuffle=True, num_workers=1)


```

## Source file: Res_Unet.py

{
  "path": "Res_Unet.py",
  "blob_sha": "318dc3d60d1ff7178844812bc3ab9e8ed9886fb2",
  "sha256": "fa2708bfa914b81fa6fad6a7b39d693ae9422a57ba3cffbf6848282177700ad9",
  "lines": 89,
  "bytes": 3227,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/RobertSmithers/TeethSegmentation/blob/e435da664fde5e7dfb1f7a032d1c026f9ea6f075/Res_Unet.py",
  "function_signature_candidates": [
    {
      "line": 2,
      "text": "def __init__(self, input_channels, output_channels, down=True):"
    },
    {
      "line": 14,
      "text": "def forward(self, x):"
    },
    {
      "line": 20,
      "text": "def __init__(self, input_channels, output_channels):"
    },
    {
      "line": 24,
      "text": "def forward(self, x):"
    },
    {
      "line": 30,
      "text": "def __init__(self, input_channels, output_channels):"
    },
    {
      "line": 34,
      "text": "def forward(self,x):"
    },
    {
      "line": 39,
      "text": "def __init__(self, input_channels, output_channels):"
    },
    {
      "line": 44,
      "text": "def forward(self, x1, x2):"
    },
    {
      "line": 55,
      "text": "def __init__(self, input_channels, output_channels):"
    },
    {
      "line": 60,
      "text": "def forward(self, x):"
    },
    {
      "line": 65,
      "text": "def __init__(self):"
    },
    {
      "line": 78,
      "text": "def forward(self, x):"
    }
  ]
}

```text

class res_conv(nn.Module):
    def __init__(self, input_channels, output_channels, down=True):
        super(res_conv, self).__init__()
        self.conv1 = nn.Sequential(nn.Conv2d(input_channels, output_channels, kernel_size=3, padding=1),
                                   nn.BatchNorm2d(output_channels),
                                   nn.LeakyReLU(inplace = True),
                                   nn.Dropout(0.1),
                                 )
        self.conv2 = nn.Sequential(nn.Conv2d(output_channels, output_channels, kernel_size=3, padding=1),
                                   nn.BatchNorm2d(output_channels),
                                   nn.LeakyReLU(inplace = True),
                                   nn.Dropout(0.1),
                                  )
    def forward(self, x):
        x1 = self.conv1(x)
        x2 = self.conv2(x1)+x1
        return x2

class start_conv(nn.Module):
    def __init__(self, input_channels, output_channels):
        super(start_conv, self).__init__()
        self.conv = res_conv(input_channels, output_channels)

    def forward(self, x):
        x = self.conv(x)
        return x


class down_conv(nn.Module):
    def __init__(self, input_channels, output_channels):
        super(down_conv, self).__init__()
        self.conv = nn.Sequential(nn.MaxPool2d(2),
                                  res_conv(input_channels, output_channels),)
    def forward(self,x):
        x = self.conv(x)
        return x

class up_conv(nn.Module):
    def __init__(self, input_channels, output_channels):
        super(up_conv, self).__init__()
        self.up = nn.ConvTranspose2d(input_channels//2, input_channels//2, kernel_size=2, stride=2)
        self.conv = res_conv(input_channels, output_channels)

    def forward(self, x1, x2):
        x1 = self.up(x1)
        diff1 = x2.shape[2]-x1.shape[2]
        diff2 = x2.shape[3]-x1.shape[3]
        x1 = F.pad(x1, pad=(diff1//2, diff1-diff1//2, diff2//2, diff2-diff2//2))
        x = torch.cat([x2, x1], dim=1)
        x = self.conv(x)
        return x


class stop_conv(nn.Module):
    def __init__(self, input_channels, output_channels):
        super(stop_conv, self).__init__()
        self.conv = nn.Sequential(nn.Conv2d(input_channels, output_channels, kernel_size=1),
                                 nn.Sigmoid())

    def forward(self, x):
        x = self.conv(x)
        return x

class ResUnet(nn.Module):
    def __init__(self):
        super(ResUnet, self).__init__()
        self.inc = start_conv(1, 64)
        self.down1 = down_conv(64, 128)
        self.down2 = down_conv(128, 256)
        self.down3 = down_conv(256, 512)
        self.down4 = down_conv(512, 512)
        self.up1 = up_conv(1024, 256)
        self.up2 = up_conv(512, 128)
        self.up3 = up_conv(256, 64)
        self.up4 = up_conv(128, 64)
        self.outc = stop_conv(64, 1)

    def forward(self, x):
        xin = self.inc(x)
        xd1 = self.down1(xin)
        xd2 = self.down2(xd1)
        xd3 = self.down3(xd2)
        xd4 = self.down4(xd3)
        xu1 = self.up1(xd4, xd3)
        xu2 = self.up2(xu1, xd2)
        xu3 = self.up3(xu2, xd1)
        xu4 = self.up4(xu3, xin)
        out = self.outc(xu4)
        return out


```

## Source file: pre_process.py

{
  "path": "pre_process.py",
  "blob_sha": "8b570d51fee6c3ae1b4b47b72f14836cb6fad7dd",
  "sha256": "1593ee10f2c1425d5cc7535d8dbb589493d6f11b2e14daece1667de0a343e423",
  "lines": 50,
  "bytes": 1288,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 5,
    "rig": 3
  },
  "url": "https://github.com/RobertSmithers/TeethSegmentation/blob/e435da664fde5e7dfb1f7a032d1c026f9ea6f075/pre_process.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "def convert_one_channel(img):"
    },
    {
      "line": 25,
      "text": "def pre_process_images(resize_shape,path):"
    }
  ]
}

```text

# -*- coding: utf-8 -*-
"""data_preparation.ipynb

Automatically generated by Colaboratory.

Original file is located at
    https://colab.research.google.com/drive/1cWWsndFakCCQ7gB5Y8LtyttlDEoQCQ68

#  Original and Mask Images Prepare
"""

from PIL import Image
import os

import numpy as np

def convert_one_channel(img):
    #some images have 3 channels , although they are grayscale image
    if len(img.shape) > 2:
        img=img[:,:,0]
        return img
    else:
        return img

def pre_process_images(resize_shape,path):
    dirs=os.listdir(path)
    l = []
    dirs.sort()
    for i in range (1,len(dirs)+1):
        img=Image.open(path+str(i)+'.png')
        img=img.resize((resize_shape),Image.ANTIALIAS)
        img=convert_one_channel(np.asarray(img))
        l.append(img)
    return l

! git clone https://github.com/RobertSmithers/TeethSegmentation.git

import sys
sys.path.append("/content/TeethSegmentation")

# Commented out IPython magic to ensure Python compatibility.
# %cd '/content/TeethSegmentation/'

og_path = '/content/TeethSegmentation/data/original_img/'
masked_path = '/content/TeethSegmentation/data/masked_img/'

resize_shape = (3104,1200)
og_img = pre_process_images(resize_shape, og_path)

mask_img = pre_process_images(resize_shape, masked_path)


```

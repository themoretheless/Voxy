# Facial rendering source scan: alexandru-dinu/3D-face-reconstruction

Repository: https://github.com/alexandru-dinu/3D-face-reconstruction

Pinned source revision: bec912c581ad0698d13d76efbe70ac12116d2fe0

Discovery topic: face reconstruction 3D

Description: 3D Face Reconstruction from a Single Image using Direct Volumetric CNN Regression.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: src/utils.py

{
  "path": "src/utils.py",
  "blob_sha": "e1d32bc54d0b009f18b3e1ec6efbce0a0c616d38",
  "sha256": "d57adea8e5f39f7e2a3bf9cc17b49462dfa8867a7f5e3a411bdfd8cf642b9452",
  "lines": 42,
  "bytes": 1157,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 9
  },
  "url": "https://github.com/alexandru-dinu/3D-face-reconstruction/blob/bec912c581ad0698d13d76efbe70ac12116d2fe0/src/utils.py",
  "function_signature_candidates": [
    {
      "line": 5,
      "text": "def get_args():"
    },
    {
      "line": 23,
      "text": "def gaussian_distribution(center_x, center_y, size=400):"
    },
    {
      "line": 34,
      "text": "def to_cuda(data, use_cuda):"
    },
    {
      "line": 41,
      "text": "def num_flat_features(x):"
    }
  ]
}

```text

import argparse
import numpy as np


def get_args():
    parser = argparse.ArgumentParser()

    parser.add_argument("--images_dir", type=str)
    parser.add_argument("--mats_dir", type=str)
    parser.add_argument("--lands_dir", type=str)
    parser.add_argument("--transform", action="store_true")

    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--start_epoch", type=int, default=0)

    parser.add_argument("--test_img", type=str)

    parser.add_argument("--checkpoint", type=str, help="path to saved model")

    return parser.parse_args()


def gaussian_distribution(center_x, center_y, size=400):
    img = np.zeros((size, size))
    for i in range(center_x - 10, center_x + 10):
        for j in range(center_y - 10, center_y + 10):
            exp_fact = ((i - center_x) ** 2) / 2 + ((j - center_y) ** 2) / 2
            img[i, size - 1 - j] = 1 / np.sqrt(2 * np.pi) * np.exp(-exp_fact)

    img /= np.sum(img)
    return img


def to_cuda(data, use_cuda):
    input_ = data.float()
    if use_cuda:
        input_ = input_.cuda()
    return input_


def num_flat_features(x):
    return np.product(x.size()[1:])


```

## Source file: src/pipeline.py

{
  "path": "src/pipeline.py",
  "blob_sha": "40a3e5d8f001126ef5314e3ae6bca67d151afcf0",
  "sha256": "088bc1785106890ff873691b6d9c9a2b6e608210083b900fb44d83ef875caa3c",
  "lines": 98,
  "bytes": 2915,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/alexandru-dinu/3D-face-reconstruction/blob/bec912c581ad0698d13d76efbe70ac12116d2fe0/src/pipeline.py",
  "function_signature_candidates": [
    {
      "line": 33,
      "text": "def train():"
    }
  ]
}

```text

import torch
import torch.nn.functional as F
import torch.optim as optim

import data_loader
from model_hourglass import StackedHourGlass
from utils import get_args, to_cuda

args = get_args()

trainset = data_loader.FacesWith3DCoords(
    images_dir=args.images_dir, mats_dir=args.mats_dir, transform=args.transform
)

trainloader = torch.utils.data.DataLoader(
    trainset, batch_size=2, shuffle=True, num_workers=2
)

model = StackedHourGlass(
    nChannels=224, nStack=2, nModules=2, numReductions=4, nOutputs=200
)
model.cuda()
model.train()
print("Set-up model")

# criterion & optimizer
criterion = torch.nn.CrossEntropyLoss()
# optimizer = optim.SGD(net.parameters(), lr=0.1, momentum=0.9)
optimizer = optim.RMSprop(model.parameters(), lr=0.001)
scheduler = optim.lr_scheduler.StepLR(optimizer, step_size=5, gamma=0.9)


def train():
    if args.resume:
        model.load_state_dict(torch.load(args.checkpoint))

    for epoch in range(args.start_epoch, 1000):  # loop over the dataset multiple times
        print("=== Epoch", epoch, "===")
        scheduler.step()

        running_loss, epoch_avg = 0.0, 0.0

        for i, data in enumerate(trainloader, start=1):
            # get the inputs
            images, volumes, landmarks = data

            images = to_cuda(images, True)
            volumes = to_cuda(volumes, True)
            # landmarks = to_cuda(landmarks, True)

            # l_shape = landmarks.shape
            # m = landmarks.view(l_shape[0], l_shape[1], l_shape[2] * l_shape[3]).argmax(2)
            # idx_gt = to_cuda(torch.stack((m // 128, m % 128), dim=2), True) / 128.0

            # zero the parameter gradients
            optimizer.zero_grad()

            # forward + backward + optimize
            # out_volumes, landmarks_predictions = net(images, landmarks)

            out_volumes = F.sigmoid(model(images))

            loss = F.binary_cross_entropy(out_volumes, volumes)
            # loss2 = F.mse_loss(landmarks_predictions, idx_gt)
            # loss = loss1 + 0.5 * loss2

            loss.backward()

            torch.nn.utils.clip_grad_value_(model.parameters(), 5)
            optimizer.step()

            # print statistics
            running_loss += loss.item()
            epoch_avg += loss.item()

            if i % 1 == 0:
                print(
                    "[%2d, %5d/%5d] loss: %.8f lr %.8f"
                    % (
                        epoch,
                        i,
                        len(trainloader),
                        running_loss / 1,
                        scheduler.get_lr()[0],
                    )
                )
                running_loss = 0.0

        print("EPOCH AVG", epoch_avg / len(trainloader))

        if epoch % 5 == 0:
            torch.save(
                model.state_dict(), "../checkpoints/2hourglass_%d_schd_with_aug" % epoch
            )


if __name__ == "__main__":
    train()


```

## Source file: src/visualize.py

{
  "path": "src/visualize.py",
  "blob_sha": "5785917c25e31676311677f401dadef1e54f87b9",
  "sha256": "1fdb3a34ccecbd53354d4fdcf6939d82a023f887062eb06acb6a28c1e44e55e7",
  "lines": 61,
  "bytes": 1567,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/alexandru-dinu/3D-face-reconstruction/blob/bec912c581ad0698d13d76efbe70ac12116d2fe0/src/visualize.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def visualize(im, vol, sz=0.25, thr=0.99):"
    }
  ]
}

```text

import cv2
import numpy as np
import visvis as vv

import data_loader
from utils import get_args


def visualize(im, vol, sz=0.25, thr=0.99):
    im, vol = im.numpy(), vol.numpy()
    print("Image shape:", im.shape)
    print("Volume shape:", vol.shape)

    # overlap with 3d representation + BGR->RGB
    im = im.transpose(1, 2, 0)  # H,W,C
    im = im[:, :, ::-1]
    im = cv2.resize(im, (128, 128))

    t = vv.imshow(im)
    t.interpolate = True  # interpolate pixels

    # volshow will use volshow3 and rendering the isosurface if OpenGL version is >= 2.0
    # Otherwise, it will show slices with bars that you can move (much less useful).
    im = (im * 128 + 128).astype(np.uint8)
    # im = np.ones_like(im)

    volRGB = np.stack(
        (
            (vol >= thr) * im[:, :, 0],
            (vol >= thr) * im[:, :, 1],
            (vol >= thr) * im[:, :, 2],
        ),
        axis=3,
    )

    v = vv.volshow(volRGB, renderStyle="iso")
    v.transformations[1].sz = sz  # Z rescaling

    l0 = vv.gca()
    l0.light0.ambient = 0.9  # 0.2 is default for light 0
    l0.light0.diffuse = 1.0  # 1.0 is default

    a = vv.gca()
    a.axis.visible = 0
    a.camera.fov = 0  # orthographic
    vv.use().Run()


if __name__ == "__main__":
    args = get_args()

    trainset = data_loader.FacesWith3DCoords(
        images_dir=args.images_dir,
        mats_dir=args.mats_dir,
        landmarks_dir=args.lands_dir,
        transform=args.transform,
    )

    i = np.random.randint(len(trainset))
    img, volume, _ = trainset[i]
    visualize(img, volume)


```

## Source file: src/view_model.py

{
  "path": "src/view_model.py",
  "blob_sha": "cf3401cb42bd4491750248ca51f90f312b57db16",
  "sha256": "40ed92d74d620e279a1cbfa24bc84e07ca1fb06468e9517cf6bfc04417db327f",
  "lines": 13,
  "bytes": 303,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alexandru-dinu/3D-face-reconstruction/blob/bec912c581ad0698d13d76efbe70ac12116d2fe0/src/view_model.py",
  "function_signature_candidates": []
}

```text

import torchfile
import ast

model = torchfile.load("../pre_trained_models/vrn-unguided.t7")

assert model is not None

print("vars:", list(vars(model)))
print("_obj:", model._obj.keys())

print("modules:")
for i, m in enumerate(model._obj[b"modules"], start=1):
    print("\t", i, "->", m._obj.keys())


```

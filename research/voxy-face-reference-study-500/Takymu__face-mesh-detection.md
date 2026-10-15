# Facial rendering source scan: Takymu/face-mesh-detection

Repository: https://github.com/Takymu/face-mesh-detection

Pinned source revision: 5e3f1a6fc7d04e0655634551698f8e727e5d9dcc

Discovery topic: facial mocap

Description: markerless facial mocap technology, course project.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["model/notebooks/mainnofc_eye_pos.ipynb", "model/tests/models_testing/eye_area_detector.py", "model/tests/models_testing/eye_area_detector_fc.py", "model/tests/models_testing/eye_area_detector_good.py", "model/tests/models_testing/eye_area_detector_multiflow.py", "model/tests/models_testing/eyes_window_detector.py"], "flame": ["reconstruction/flame_tests/main.py"], "face": ["dataset_creation/create_coords_from_video/demoface.obj", "dataset_creation/create_coords_from_video/mmod_human_face_detector.dat.txt", "dataset_creation/create_coords_from_video/shape_predictor_68_face_landmarks.dat.txt", "dataset_creation/point_adding/demoface.obj", "model/teach_face_det.py", "model/tests/models_testing/face_det.py", "reconstruction/face_viewer.py"]}

## Source file: model/teach_face_det.py

{
  "path": "model/teach_face_det.py",
  "blob_sha": "611ab1016d15be84f595a94df3f558de856523a1",
  "sha256": "3bc15aac049378b50aeb34c1a37cb179d92653590168146f741108ad6bedf530",
  "lines": 110,
  "bytes": 3747,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 14
  },
  "url": "https://github.com/Takymu/face-mesh-detection/blob/5e3f1a6fc7d04e0655634551698f8e727e5d9dcc/model/teach_face_det.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def interactor(signal, frame):"
    },
    {
      "line": 29,
      "text": "def model_test(look=False):"
    },
    {
      "line": 46,
      "text": "def look_predict(imgtens: torch.Tensor, predict: torch.Tensor):"
    }
  ]
}

```text

import cv2
import os
import sys
import signal
import torch
import torch.nn as nn
import numpy as np

from data_process import augment_gen, make_highlighter, face_coord
from data_process import load
from cascade_detector import FaceDetector


def interactor(signal, frame):
    print('SIGINT was received.')
    cmd = input('Choose action: save/test/exit/look (continue by default): ')
    match cmd:
        case 'save':
            path = weight_save_path
            print(f'saving to {path}')
            torch.save(model.state_dict(), path)
        case 'test':
            model_test(look=False)
        case 'exit':
            sys.exit(0)
        case 'look':
            model_test(look=True)

def model_test(look=False):
    with torch.no_grad():
        iteration = 0
        loss = 0.0
        for bt_images, bt_coords in augment_gen(dataset, epochs=1, device=device, 
                                                noise=0, part=-0.1, verbose=True):
            
            truth = highlight_face(bt_coords)
            ans = model(bt_images)
            loss += criterion(ans, truth)
            iteration += 1
            if look:
                look_predict(bt_images, ans)
        
        print(f'total loss {loss / iteration}')
        print(f'total iterations count {iteration}')

def look_predict(imgtens: torch.Tensor, predict: torch.Tensor):
    predict = predict[0]

    newimg = (imgtens[0,[2,1,0], :, :].cpu().numpy().transpose(1,2,0) * 255).astype(np.uint8)

    newimg = np.ascontiguousarray(newimg)
    for coord in predict:
        truth = face_coord(bt_coords)
        ans = model(bt_images)
        print(truth.shape, ans.shape)
        loss = criterion(ans, truth)
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        if(iteration % 20 == 2):
            face = bt_images[0].permute(1, 2, 0)[:, :, [2, 1, 0]].cpu().numpy()
            cv2.circle(face, (int(ans[0][0] * face.shape[0]), int(ans[0][1] * face.shape[1])), 5, 1)
            cv2.imshow('face', face)
            cv2.waitKey(3000)
            cv2.destroyAllWindows() 
        cv2.imshow("img", newimg)
        cv2.waitKey(0)

# establishing paths and loading
current_path = os.path.dirname(os.path.abspath(__file__))
registry_path = os.path.join(current_path, 'registry')
weight_save_path = os.path.join(registry_path, 'weights', 'face_det.pth')
dataset = load(2000, 10, os.path.join(current_path, registry_path, 'dataset'))

# establishing devices and signal handler
device = torch.device("cuda:0" if torch.cuda.is_available() else "cpu")
print("devise is: ", device)
signal.signal(signal.SIGINT, interactor)


model = FaceDetector(device).to(device)
# if input('load weigths from selected weight save path? (y/n) ') == 'y':
    # lvl1det.load_state_dict(torch.load(weight_save_path))

optimizer = torch.optim.Adam(model.parameters(), 0.001, betas=(0.9, 0.95))
criterion = nn.MSELoss().to(device)
highlight_face = make_highlighter(64, 20, 15, device)
# learning cycle
iteration = 0
for bt_images, bt_coords in augment_gen(dataset, epochs=10, device=device,
                                        noise=0.1, part=0.9, displace=128, rotate=20):
    
    truth = face_coord(bt_coords)
    ans = model(bt_images)
    print(truth.shape, ans.shape)
    loss = criterion(ans, truth)
    optimizer.zero_grad()
    loss.backward()
    optimizer.step()

    if(iteration % 20 == 2):
        face = bt_images[0].permute(1, 2, 0)[:, :, [2, 1, 0]].cpu().numpy()
        cv2.circle(face, (int(ans[0][0] * face.shape[0]), int(ans[0][1] * face.shape[1])), 5, 1)
        cv2.imshow('face', face)
        cv2.waitKey(3000)
        cv2.destroyAllWindows()
    
    iteration += 1
    print(f'loss {loss:.5f}, iteration: {iteration}')


```

## Source file: reconstruction/face_viewer.py

{
  "path": "reconstruction/face_viewer.py",
  "blob_sha": "34dd58c601979096fb3abb3c2973b55b05a22460",
  "sha256": "5bebb3496abd1779b2d3f6cb30e42d41b461cc7d63a07c06e82fc521ec184a19",
  "lines": 45,
  "bytes": 1438,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/Takymu/face-mesh-detection/blob/5e3f1a6fc7d04e0655634551698f8e727e5d9dcc/reconstruction/face_viewer.py",
  "function_signature_candidates": []
}

```text

import torch
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D  # Импорт для 3D проекций

DOTS = 68

cordpath = '../../experiments/condcords00003_3d.txt'
cordfile = open(cordpath, 'r')
ts = torch.zeros(DOTS, 3)
for i, line in enumerate(cordfile):
    ts[i] = torch.tensor(list(map(float, line.split())))
ts[:, 1] = ts[:, 1] * ts[:, 2]
ts[:, 0] = ts[:, 0] * ts[:, 2]

points = ts

# 2. Создание фигуры и 3D проекции:
fig = plt.figure(figsize=(8, 8))
ax = fig.add_subplot(111, projection='3d')

# 3. Извлечение координат:
x = points[:, 0].numpy()
y = points[:, 2].numpy()
z = points[:, 1].numpy()

# 4. Построение графика:
ax.scatter(x, y, z, c='r', marker='o')

# Добавление номеров точек
for i in range(len(x)):
    ax.text(x[i], y[i], z[i],  '%s' % (str(i)), size=8, zorder=1,  color='k')

# 5. Настройка осей (необязательно, но рекомендуется):
ax.set_xlabel('X')
ax.set_ylabel('Y')
ax.set_zlabel('Z')

# Можно установить пределы осей, если нужно:
ax.set_xlim([0.5, 2])  # Пример: пределы для оси X
ax.set_ylim([1.5, 3])  # Пример: пределы для оси Y
ax.set_zlim([0.8, 2.3])  # Пример: пределы для оси Z

# 6. Отображение графика:
plt.title('face points')
plt.show()


```

## Source file: model/tests/models_testing/face_det.py

{
  "path": "model/tests/models_testing/face_det.py",
  "blob_sha": "1d2ab4831a71c5c62e8c05a918278dcdf1d54ee9",
  "sha256": "96e9db6ff02756c11751ad59b7b5f25409838be92048218ec9d025fae1911e26",
  "lines": 97,
  "bytes": 3731,
  "selection_score": 13,
  "keyword_line_counts": {
    "eye": 1,
    "face": 5
  },
  "url": "https://github.com/Takymu/face-mesh-detection/blob/5e3f1a6fc7d04e0655634551698f8e727e5d9dcc/model/tests/models_testing/face_det.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self, device):"
    },
    {
      "line": 28,
      "text": "def forward(self, x):"
    },
    {
      "line": 55,
      "text": "def test():"
    }
  ]
}

```text


import cv2
import torch
from torch import device, nn
import numpy as np
class FaceDetector(nn.Module):
    def __init__(self, device):
        super(FaceDetector, self).__init__()
        self.adpool = nn.AdaptiveAvgPool2d((128, 128)).to(device)
        self.pool = nn.MaxPool2d(2, 2).to(device)
        self.mid_depth = mid_depth = 16
        self.conv1 = nn.Conv2d(3, mid_depth, 3, padding = 1).to(device) 
        self.conv2 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv3 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv4 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv5 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv6 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv7 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv8 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv9 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv10 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv11 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv12 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv13 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv14 = nn.Conv2d(mid_depth, 1, 3, padding = 1).to(device)
        self.act = nn.ReLU().to(device)

    def forward(self, x):
        # Input: [batch, 3, H, W]
        x = self.adpool(x)
        print(x.shape)
        x1 = x.clone()
        x1 = x1.repeat(1, 6, 1, 1)[:, :self.mid_depth, :, :]
        x = self.act(self.conv1(x))
        x = self.pool(self.act(self.conv2(x)) + x1)
        x1 = x.clone()
        x = self.act(self.conv3(x))
        x = self.act(self.conv4(x)) + x1 # add skip connections (+x.copy()) if needed
        x1 = x.clone()
        x = self.act(self.conv5(x))
        x = self.act(self.conv6(x)) + x1 
        x1 = x.clone()
        x = self.act(self.conv7(x))
        x = self.act(self.conv8(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv9(x))
        x = self.act(self.conv10(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv11(x))
        x = self.act(self.conv12(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv14(x))
        return x

def test():
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    newmodel = FaceDetector(device).to(device)
    newmodel.load_state_dict(torch.load("registry/weights/face_det_works.pth"))
    newmodel.eval()

    cap = cv2.VideoCapture(0)

    while True:
        # Capture frame-by-frame
        ret, frame = cap.read()
        # frame = cv2.resize(frame, (256, 256))
        
        frame = torch.from_numpy(frame.astype(np.float32)).to(device) / 255
        frame = frame.permute(2, 0, 1)
        height = frame.shape[1]
        width = frame.shape[2]
        dif = width - height
        frame = frame[:, 100:356, 100:356]
        
        if not ret:
            break
        
        # Detect and visualize eye areas
        result_frame = newmodel(frame)
        
        result_frame = result_frame[0]
        result_frame = result_frame.permute(1, 2, 0).cpu().detach().numpy()
        thr, result_frame = cv2.threshold(result_frame, 0.2, 1, cv2.THRESH_BINARY)
        result_frame = cv2.resize(result_frame, (512, 512))

        # Display the resulting frame
        cv2.imshow('Face Area Detection', result_frame)
        
        # Exit on 'q' key press
        if cv2.waitKey(1) & 0xFF == ord('q'):
            break

    # Release resources
    cap.release()
    cv2.destroyAllWindows()

test()

```

## Source file: model/tests/models_testing/eye_area_detector.py

{
  "path": "model/tests/models_testing/eye_area_detector.py",
  "blob_sha": "cf4a2a76830afccbd21203e9a2489935a36d7e5d",
  "sha256": "fadc173a1499109e2628dc5accd6662b80eb140a68de19bc54f71247b28a5a39",
  "lines": 95,
  "bytes": 3631,
  "selection_score": 13,
  "keyword_line_counts": {
    "eye": 3
  },
  "url": "https://github.com/Takymu/face-mesh-detection/blob/5e3f1a6fc7d04e0655634551698f8e727e5d9dcc/model/tests/models_testing/eye_area_detector.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def __init__(self, device):"
    },
    {
      "line": 29,
      "text": "def forward(self, x):"
    },
    {
      "line": 57,
      "text": "def test():"
    }
  ]
}

```text


import cv2
import torch
from torch import device, nn
import numpy as np

class CnnDetector(nn.Module):
    def __init__(self, device):
        super(CnnDetector, self).__init__()
        self.adpool = nn.AdaptiveAvgPool2d((128, 128)).to(device)
        self.pool = nn.MaxPool2d(2, 2).to(device)
        self.mid_depth = mid_depth = 7
        self.conv1 = nn.Conv2d(3, mid_depth, 7, padding = 3).to(device) 
        self.conv2 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv3 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv4 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv5 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv6 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv7 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv8 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv9 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv10 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv11 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv12 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv13 = nn.Conv2d(mid_depth, mid_depth, 3, padding = 1).to(device)
        self.conv14 = nn.Conv2d(mid_depth, 1, 3, padding = 1).to(device)
        self.act = nn.ReLU().to(device)

    def forward(self, x):
        # Input: [batch, 3, H, W]
        x = self.adpool(x)
        print(x.shape)
        x1 = x.clone()
        x1 = x1.repeat(1, 3, 1, 1)[:, :self.mid_depth, :, :]
        x = self.act(self.conv1(x))
        x = self.pool(self.act(self.conv2(x)) + x1)
        x1 = x.clone()
        x = self.act(self.conv3(x))
        x = self.act(self.conv4(x)) + x1 # add skip connections (+x.copy()) if needed
        x1 = x.clone()
        x = self.act(self.conv5(x))
        x = self.act(self.conv6(x)) + x1 
        x1 = x.clone()
        x = self.act(self.conv7(x))
        x = self.act(self.conv8(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv9(x))
        x = self.act(self.conv10(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv11(x))
        x = self.act(self.conv12(x)) + x1
        x1 = x.clone()
        x = self.act(self.conv13(x))
        x = x1 * 0.3 + self.act(self.conv14(x))
        return x

def test():
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    newmodel = CnnDetector(device).to(device)
    newmodel.load_state_dict(torch.load("eye_area_detector.pth"))
    newmodel.eval()

    cap = cv2.VideoCapture(0)

    while True:
        # Capture frame-by-frame
        ret, frame = cap.read()
        frame = torch.from_numpy(frame.astype(np.float32)).to(device) / 255
        frame = frame.permute(2, 0, 1)
        height = frame.shape[1]
        width = frame.shape[2]
        dif = width - height
        frame = frame[:, :height, dif//2:width - dif//2]
        if not ret:
            break
        
        # Detect and visualize eye areas
        result_frame = newmodel(frame)
        
        result_frame = result_frame[0][1:4]
        result_frame = result_frame.permute(1, 2, 0).cpu().detach().numpy()
        result_frame = cv2.resize(result_frame, (512, 512))

        # Display the resulting frame
        cv2.imshow('Eye Area Detection', result_frame)
        
        # Exit on 'q' key press
        if cv2.waitKey(1) & 0xFF == ord('q'):
            break

    # Release resources
    cap.release()
    cv2.destroyAllWindows()

test()

```

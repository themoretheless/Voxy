# Facial rendering source scan: Boese0601/MagicDance

Repository: https://github.com/Boese0601/MagicDance

Pinned source revision: a515a1f4a5fceb9abb3d2cabbc48958ba249e560

Discovery topic: facial retargeting

Description: [ICML 2024] MagicPose(also known as MagicDance): Realistic Human Poses and Facial Expressions Retargeting with Identity-aware Diffusion 

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE.txt", "model_lib/ControlNet/annotator/openpose/LICENSE", "model_lib/ControlNet/annotator/zoe/LICENSE", "model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/LICENSE", "model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/mobile/android/LICENSE", "model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/mobile/ios/LICENSE", "model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/ros/LICENSE"]

Topic paths: {"deform": ["model_lib/ControlNet/annotator/uniformer/mmcv/ops/deform_conv.py", "model_lib/ControlNet/annotator/uniformer/mmcv/ops/deform_roi_pool.py", "model_lib/ControlNet/annotator/uniformer/mmcv/ops/modulated_deform_conv.py", "model_lib/ControlNet/annotator/uniformer/mmcv/ops/multi_scale_deform_attn.py"], "rig": ["model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/ros/midas_cpp/scripts/listener_original.py"], "face": ["misc_scripts/keypoint/face_keypoint.py", "misc_scripts/keypoint/idls/faceattr.thrift", "model_lib/ControlNet/annotator/openpose/face.py", "model_lib/ControlNet/annotator/zoe/zoedepth/models/base_models/midas_repo/mobile/ios/Midas.xcodeproj/project.xcworkspace/xcuserdata/admin.xcuserdatad/UserInterfaceState.xcuserstate"], "lip": ["model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/config.json", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/merges.txt", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/preprocessor_config.json", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/special_tokens_map.json", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/tokenizer.json", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/tokenizer_config.json", "model_lib/ControlNet/annotator/ckpts/blip2-opt-2.7b/vocab.json", "tool/ClipScore.py"]}

## Source file: tool/ClipScore.py

{
  "path": "tool/ClipScore.py",
  "blob_sha": "8cf7d7803b547ad3f9876ed05c8b39e7cb711d6a",
  "sha256": "1ab6ab10d107a6f576d1ebec4a3bb82197e878c39e75005d8ecc6d65368ecda6",
  "lines": 95,
  "bytes": 3269,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 11
  },
  "url": "https://github.com/Boese0601/MagicDance/blob/a515a1f4a5fceb9abb3d2cabbc48958ba249e560/tool/ClipScore.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self, model_filename):"
    },
    {
      "line": 15,
      "text": "def forward(self, text, image, batch_size=None):"
    },
    {
      "line": 50,
      "text": "def __init__(self, img_list, txt_list):"
    },
    {
      "line": 63,
      "text": "def __len__(self):"
    },
    {
      "line": 66,
      "text": "def __getitem__(self, i):"
    },
    {
      "line": 76,
      "text": "def compute_clip_score(model_pth, img_list, txt_list, batch_size=32, cuda_device=0):"
    }
  ]
}

```text

import sys
import os
sys.path.append(os.path.dirname(__file__) + os.sep + '../')
import clip
from config import *


class CLIPSimilarity(nn.Module):
    def __init__(self, model_filename):
        super(CLIPSimilarity, self).__init__()
        self.model_filename = model_filename
        self.model, self.p = clip.load(model_filename, device='cpu')
        self.model = self.model.eval()

    def forward(self, text, image, batch_size=None):
        '''
        text: [X]
          for example,  ['str_1', 'str_2', ..., 'str_X']
        image: [Y, c, w, h]

        return: [X, Y]
        '''
        device = image.device
        img_input = F.interpolate(image, size=224)
        image_mean = torch.tensor([0.48145466, 0.4578275, 0.40821073]).to(device)
        image_std = torch.tensor([0.26862954, 0.26130258, 0.27577711]).to(device)
        img_input = (img_input.clamp(min=-1.0, max=1.0) + 1.0) / 2.0
        img_input -= image_mean[:, None, None]
        img_input /= image_std[:, None, None]
        text_input = clip.tokenize(text, truncate=True).to(device)
        with torch.no_grad():
            text_features = self.model.encode_text(text_input).float()
            image_features = self.model.encode_image(img_input).float()

            image_features /= image_features.norm(dim=-1, keepdim=True)
            text_features /= text_features.norm(dim=-1, keepdim=True)

            logit_scale = 1.0  # Default value is models.logit_scale.exp()
            if batch_size:
                text_features = rearrange(text_features, '(b x) d -> b x d', b=batch_size)
                image_features = rearrange(image_features, '(b y) d -> b d y', b=batch_size)
            else:
                image_features = image_features.t()
            logits_per_text = logit_scale * torch.matmul(text_features, image_features)
        similarity = logits_per_text
        return similarity


class BaseDataset(Dataset):
    def __init__(self, img_list, txt_list):

        self.img_list = img_list
        self.txt_list = txt_list

        assert len(img_list) == len(txt_list), 'len_img must equal len_txt!'

        self.transform = transforms.Compose([
            transforms.Resize((224, 224)),
            transforms.ToTensor(),
            transforms.Normalize([0.5, 0.5, 0.5], [0.5, 0.5, 0.5]),
        ])

    def __len__(self):
        return len(self.img_list)

    def __getitem__(self, i):
        img_path = self.img_list[i]
        caption = self.txt_list[i]
        img = Image.open(img_path).convert('RGB')
        image = self.transform(img)

        sample = {"label_imgs": image, 'input_text': caption}
        return sample


def compute_clip_score(model_pth, img_list, txt_list, batch_size=32, cuda_device=0):

    device = torch.device(f'cuda:{cuda_device}')
    model = CLIPSimilarity(model_pth).to(device)

    data = BaseDataset(img_list, txt_list)
    loader = DataLoader(data, batch_size=batch_size, shuffle=False, num_workers=32)

    clip_sim_total = 0
    num = 0
    for sample in tqdm(loader):
        imgs = sample['label_imgs'].to(device)
        caps = sample['input_text']

        clip_sim = model(caps, imgs, batch_size=len(caps))

        clip_sim_total += clip_sim.mean().item()
        num += 1

    return clip_sim_total/num


```

## Source file: misc_scripts/keypoint/face_keypoint.py

{
  "path": "misc_scripts/keypoint/face_keypoint.py",
  "blob_sha": "d5a3b31fb4729595b3f2d783ad69e6b0045c7fe0",
  "sha256": "585b8cbf37e7da159b7358e1223df530dd896c08c7ec3b45a334075d1f6c4dcd",
  "lines": 66,
  "bytes": 2491,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 2,
    "face": 7
  },
  "url": "https://github.com/Boese0601/MagicDance/blob/a515a1f4a5fceb9abb3d2cabbc48958ba249e560/misc_scripts/keypoint/face_keypoint.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self):"
    },
    {
      "line": 27,
      "text": "def predict(self, img, pt='106', return_bbox=False):"
    }
  ]
}

```text

#!/usr/bin/python
# -*- coding: utf-8 -*-

import euler
import os
import io
import thriftpy2
import numpy as np

class FaceKeypointDetector(object):
    def __init__(self):
        module_path = os.path.dirname(__file__)
        self.common_thrift = thriftpy2.load(
            module_path + '/idls/common.thrift', 
            module_name='common_thrift'
        )
        self.server_thrift = thriftpy2.load(
            module_path + '/idls/faceattr.thrift', 
            module_name='faceattr_thrift'
        )
        self.client = euler.Client(
            self.server_thrift.FaceAttr,
            'sd://labcv.smash.serving?cluster=default',
            timeout=10
        )

    def predict(self, img, pt='106', return_bbox=False):
        # pt: 106->正常人脸关键点, 44->外轮廓关键点
        # https://bytedance.feishu.cn/wiki/wikcn1Y8WcbOFBHfVjNj0e6LVId
        # https://bytedance.feishu.cn/docs/doccnvCM1LYPhsMPfNRmZwQ371d#1LwosE
        if pt not in ['106', '44']:
            raise ValueError('%s keypoint type not supported yet.' % pt)

        buffer = io.BytesIO()
        img.save(buffer, format='PNG')
        img_data = buffer.getvalue()

        req = self.server_thrift.FaceAttrReq()
        base_info = self.common_thrift.BaseInfo()
        base_info.image_name = 'data'
        base_info.image_data = img_data
        req.base_info = base_info
        rsp = self.client.PredictFaceAttr(req)
        attr_infos = rsp.face_attr_infos
        
        h = rsp.base_meta.image_height
        w = rsp.base_meta.image_width

        keypoints = []
        bboxes = []
        if pt == '106':
            for attr_info in attr_infos:
                bbox = [attr_info.rect.left * w, attr_info.rect.top * h, attr_info.rect.right * w, attr_info.rect.bottom * h]
                bboxes.append(np.array(bbox))
                cur = [[p.x * w, p.y * h] for p in attr_info.landmark106s]
                keypoints.append(np.array(cur, dtype=np.float32).reshape((-1, 2)))
        else:
            for attr_info in attr_infos:
                bbox = [attr_info.rect.left * w, attr_info.rect.top * h, attr_info.rect.right * w, attr_info.rect.bottom * h]
                bboxes.append(np.array(bbox))
                cur = [[p.x * w, p.y * h] for p in attr_info.landmark_outlines][:44]
                keypoints.append(np.array(cur, dtype=np.float32).reshape((-1, 2)))
        if return_bbox:
            return keypoints, bboxes
        else:
            return keypoints


```

## Source file: model_lib/ControlNet/annotator/openpose/face.py

{
  "path": "model_lib/ControlNet/annotator/openpose/face.py",
  "blob_sha": "24969e620186a5183da0cb9fae6ca8db62fe57fa",
  "sha256": "335c091f97a807696695bd95ed1812224cd0ec0fb6d3df0300fbd2e6d6c4c97d",
  "lines": 363,
  "bytes": 13510,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 10
  },
  "url": "https://github.com/Boese0601/MagicDance/blob/a515a1f4a5fceb9abb3d2cabbc48958ba249e560/model_lib/ControlNet/annotator/openpose/face.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def __init__(self):"
    },
    {
      "line": 191,
      "text": "def forward(self, x):"
    },
    {
      "line": 317,
      "text": "def __init__(self, face_model_path,"
    },
    {
      "line": 331,
      "text": "def __call__(self, face_img):"
    },
    {
      "line": 348,
      "text": "def compute_peaks_from_heatmaps(self, heatmaps):"
    }
  ]
}

```text

import logging
import numpy as np
from torchvision.transforms import ToTensor, ToPILImage
import torch
import torch.nn.functional as F
import cv2

from . import util
from torch.nn import Conv2d, Module, ReLU, MaxPool2d, init


class FaceNet(Module):
    """Model the cascading heatmaps. """
    def __init__(self):
        super(FaceNet, self).__init__()
        # cnn to make feature map
        self.relu = ReLU()
        self.max_pooling_2d = MaxPool2d(kernel_size=2, stride=2)
        self.conv1_1 = Conv2d(in_channels=3, out_channels=64,
                              kernel_size=3, stride=1, padding=1)
        self.conv1_2 = Conv2d(
            in_channels=64, out_channels=64, kernel_size=3, stride=1,
            padding=1)
        self.conv2_1 = Conv2d(
            in_channels=64, out_channels=128, kernel_size=3, stride=1,
            padding=1)
        self.conv2_2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=3, stride=1,
            padding=1)
        self.conv3_1 = Conv2d(
            in_channels=128, out_channels=256, kernel_size=3, stride=1,
            padding=1)
        self.conv3_2 = Conv2d(
            in_channels=256, out_channels=256, kernel_size=3, stride=1,
            padding=1)
        self.conv3_3 = Conv2d(
            in_channels=256, out_channels=256, kernel_size=3, stride=1,
            padding=1)
        self.conv3_4 = Conv2d(
            in_channels=256, out_channels=256, kernel_size=3, stride=1,
            padding=1)
        self.conv4_1 = Conv2d(
            in_channels=256, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv4_2 = Conv2d(
            in_channels=512, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv4_3 = Conv2d(
            in_channels=512, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv4_4 = Conv2d(
            in_channels=512, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv5_1 = Conv2d(
            in_channels=512, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv5_2 = Conv2d(
            in_channels=512, out_channels=512, kernel_size=3, stride=1,
            padding=1)
        self.conv5_3_CPM = Conv2d(
            in_channels=512, out_channels=128, kernel_size=3, stride=1,
            padding=1)

        # stage1
        self.conv6_1_CPM = Conv2d(
            in_channels=128, out_channels=512, kernel_size=1, stride=1,
            padding=0)
        self.conv6_2_CPM = Conv2d(
            in_channels=512, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        # stage2
        self.Mconv1_stage2 = Conv2d(
            in_channels=199, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv2_stage2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv3_stage2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv4_stage2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv5_stage2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv6_stage2 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=1, stride=1,
            padding=0)
        self.Mconv7_stage2 = Conv2d(
            in_channels=128, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        # stage3
        self.Mconv1_stage3 = Conv2d(
            in_channels=199, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv2_stage3 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv3_stage3 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv4_stage3 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv5_stage3 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv6_stage3 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=1, stride=1,
            padding=0)
        self.Mconv7_stage3 = Conv2d(
            in_channels=128, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        # stage4
        self.Mconv1_stage4 = Conv2d(
            in_channels=199, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv2_stage4 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv3_stage4 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv4_stage4 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv5_stage4 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv6_stage4 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=1, stride=1,
            padding=0)
        self.Mconv7_stage4 = Conv2d(
            in_channels=128, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        # stage5
        self.Mconv1_stage5 = Conv2d(
            in_channels=199, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv2_stage5 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv3_stage5 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv4_stage5 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv5_stage5 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv6_stage5 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=1, stride=1,
            padding=0)
        self.Mconv7_stage5 = Conv2d(
            in_channels=128, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        # stage6
        self.Mconv1_stage6 = Conv2d(
            in_channels=199, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv2_stage6 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv3_stage6 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv4_stage6 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv5_stage6 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=7, stride=1,
            padding=3)
        self.Mconv6_stage6 = Conv2d(
            in_channels=128, out_channels=128, kernel_size=1, stride=1,
            padding=0)
        self.Mconv7_stage6 = Conv2d(
            in_channels=128, out_channels=71, kernel_size=1, stride=1,
            padding=0)

        for m in self.modules():
            if isinstance(m, Conv2d):
                init.constant_(m.bias, 0)

    def forward(self, x):
        """Return a list of heatmaps."""
        heatmaps = []

        h = self.relu(self.conv1_1(x))
        h = self.relu(self.conv1_2(h))
        h = self.max_pooling_2d(h)
        h = self.relu(self.conv2_1(h))
        h = self.relu(self.conv2_2(h))
        h = self.max_pooling_2d(h)
        h = self.relu(self.conv3_1(h))
        h = self.relu(self.conv3_2(h))
        h = self.relu(self.conv3_3(h))
        h = self.relu(self.conv3_4(h))
        h = self.max_pooling_2d(h)
        h = self.relu(self.conv4_1(h))
        h = self.relu(self.conv4_2(h))
        h = self.relu(self.conv4_3(h))
        h = self.relu(self.conv4_4(h))
        h = self.relu(self.conv5_1(h))
        h = self.relu(self.conv5_2(h))
        h = self.relu(self.conv5_3_CPM(h))
        feature_map = h

        # stage1
        h = self.relu(self.conv6_1_CPM(h))
        h = self.conv6_2_CPM(h)
        heatmaps.append(h)

        # stage2
        h = torch.cat([h, feature_map], dim=1)  # channel concat
        h = self.relu(self.Mconv1_stage2(h))
        h = self.relu(self.Mconv2_stage2(h))
        h = self.relu(self.Mconv3_stage2(h))
        h = self.relu(self.Mconv4_stage2(h))
        h = self.relu(self.Mconv5_stage2(h))
        h = self.relu(self.Mconv6_stage2(h))
        h = self.Mconv7_stage2(h)
        heatmaps.append(h)

        # stage3
        h = torch.cat([h, feature_map], dim=1)  # channel concat
        h = self.relu(self.Mconv1_stage3(h))
        h = self.relu(self.Mconv2_stage3(h))
        h = self.relu(self.Mconv3_stage3(h))
        h = self.relu(self.Mconv4_stage3(h))
        h = self.relu(self.Mconv5_stage3(h))
        h = self.relu(self.Mconv6_stage3(h))
        h = self.Mconv7_stage3(h)
        heatmaps.append(h)

        # stage4
        h = torch.cat([h, feature_map], dim=1)  # channel concat
        h = self.relu(self.Mconv1_stage4(h))
        h = self.relu(self.Mconv2_stage4(h))
        h = self.relu(self.Mconv3_stage4(h))
        h = self.relu(self.Mconv4_stage4(h))
        h = self.relu(self.Mconv5_stage4(h))
        h = self.relu(self.Mconv6_stage4(h))
        h = self.Mconv7_stage4(h)
        heatmaps.append(h)

        # stage5
        h = torch.cat([h, feature_map], dim=1)  # channel concat
        h = self.relu(self.Mconv1_stage5(h))
        h = self.relu(self.Mconv2_stage5(h))
        h = self.relu(self.Mconv3_stage5(h))
        h = self.relu(self.Mconv4_stage5(h))
        h = self.relu(self.Mconv5_stage5(h))
        h = self.relu(self.Mconv6_stage5(h))
        h = self.Mconv7_stage5(h)
        heatmaps.append(h)

        # stage6
        h = torch.cat([h, feature_map], dim=1)  # channel concat
        h = self.relu(self.Mconv1_stage6(h))
        h = self.relu(self.Mconv2_stage6(h))
        h = self.relu(self.Mconv3_stage6(h))
        h = self.relu(self.Mconv4_stage6(h))
        h = self.relu(self.Mconv5_stage6(h))
        h = self.relu(self.Mconv6_stage6(h))
        h = self.Mconv7_stage6(h)
        heatmaps.append(h)

        return heatmaps


LOG = logging.getLogger(__name__)
TOTEN = ToTensor()
TOPIL = ToPILImage()


params = {
    'gaussian_sigma': 2.5,
    'inference_img_size': 736,  # 368, 736, 1312
    'heatmap_peak_thresh': 0.1,
    'crop_scale': 1.5,
    'line_indices': [
        [0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 6],
        [6, 7], [7, 8], [8, 9], [9, 10], [10, 11], [11, 12], [12, 13],
        [13, 14], [14, 15], [15, 16],
        [17, 18], [18, 19], [19, 20], [20, 21],
        [22, 23], [23, 24], [24, 25], [25, 26],
        [27, 28], [28, 29], [29, 30],
        [31, 32], [32, 33], [33, 34], [34, 35],
        [36, 37], [37, 38], [38, 39], [39, 40], [40, 41], [41, 36],
        [42, 43], [43, 44], [44, 45], [45, 46], [46, 47], [47, 42],
        [48, 49], [49, 50], [50, 51], [51, 52], [52, 53], [53, 54],
        [54, 55], [55, 56], [56, 57], [57, 58], [58, 59], [59, 48],
        [60, 61], [61, 62], [62, 63], [63, 64], [64, 65], [65, 66],
        [66, 67], [67, 60]
    ],
}


class Face(object):
    """
    The OpenPose face landmark detector model.

    Args:
        inference_size: set the size of the inference image size, suggested:
            368, 736, 1312, default 736
        gaussian_sigma: blur the heatmaps, default 2.5
        heatmap_peak_thresh: return landmark if over threshold, default 0.1

    """
    def __init__(self, face_model_path,
                 inference_size=None,
                 gaussian_sigma=None,
                 heatmap_peak_thresh=None):
        self.inference_size = inference_size or params["inference_img_size"]
        self.sigma = gaussian_sigma or params['gaussian_sigma']
        self.threshold = heatmap_peak_thresh or params["heatmap_peak_thresh"]
        self.model = FaceNet()
        self.model.load_state_dict(torch.load(face_model_path))
        if torch.cuda.is_available():
            self.model = self.model.cuda()
            print('cuda')
        self.model.eval()

    def __call__(self, face_img):
        H, W, C = face_img.shape

        w_size = 384
        x_data = torch.from_numpy(util.smart_resize(face_img, (w_size, w_size))).permute([2, 0, 1]) / 256.0 - 0.5

        if torch.cuda.is_available():
            x_data = x_data.cuda()

        with torch.no_grad():
            hs = self.model(x_data[None, ...])
            heatmaps = F.interpolate(
                hs[-1],
                (H, W),
                mode='bilinear', align_corners=True).cpu().numpy()[0]
        return heatmaps

    def compute_peaks_from_heatmaps(self, heatmaps):
        all_peaks = []
        for part in range(heatmaps.shape[0]):
            map_ori = heatmaps[part].copy()
            binary = np.ascontiguousarray(map_ori > 0.05, dtype=np.uint8)

            if np.sum(binary) == 0:
                continue

            positions = np.where(binary > 0.5)
            intensities = map_ori[positions]
            mi = np.argmax(intensities)
            y, x = positions[0][mi], positions[1][mi]
            all_peaks.append([x, y])

        return np.array(all_peaks)


```

## Source file: model_lib/ControlNet/annotator/uniformer/mmcv/ops/deform_conv.py

{
  "path": "model_lib/ControlNet/annotator/uniformer/mmcv/ops/deform_conv.py",
  "blob_sha": "a3f8c75ee774823eea334e3b3732af6a18f55038",
  "sha256": "d3a1ceebd5438f4e3ff94df6078297cea8ac7123f17e47050a3b9a193999d7b8",
  "lines": 405,
  "bytes": 15603,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 1,
    "deform": 44,
    "rig": 1
  },
  "url": "https://github.com/Boese0601/MagicDance/blob/a515a1f4a5fceb9abb3d2cabbc48958ba249e560/model_lib/ControlNet/annotator/uniformer/mmcv/ops/deform_conv.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def symbolic(g,"
    },
    {
      "line": 50,
      "text": "def forward(ctx,"
    },
    {
      "line": 114,
      "text": "def backward(ctx, grad_output):"
    },
    {
      "line": 173,
      "text": "def _output_size(ctx, input, weight):"
    },
    {
      "line": 228,
      "text": "def __init__(self,"
    },
    {
      "line": 269,
      "text": "def reset_parameters(self):"
    },
    {
      "line": 276,
      "text": "def forward(self, x: Tensor, offset: Tensor) -> Tensor:"
    },
    {
      "line": 315,
      "text": "def __repr__(self):"
    },
    {
      "line": 358,
      "text": "def __init__(self, *args, **kwargs):"
    },
    {
      "line": 370,
      "text": "def init_offset(self):"
    },
    {
      "line": 374,
      "text": "def forward(self, x):"
    },
    {
      "line": 380,
      "text": "def _load_from_state_dict(self, state_dict, prefix, local_metadata, strict,"
    }
  ]
}

```text

# Copyright (c) OpenMMLab. All rights reserved.
from typing import Tuple, Union

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch import Tensor
from torch.autograd import Function
from torch.autograd.function import once_differentiable
from torch.nn.modules.utils import _pair, _single

from annotator.uniformer.mmcv.utils import deprecated_api_warning
from ..cnn import CONV_LAYERS
from ..utils import ext_loader, print_log

ext_module = ext_loader.load_ext('_ext', [
    'deform_conv_forward', 'deform_conv_backward_input',
    'deform_conv_backward_parameters'
])


class DeformConv2dFunction(Function):

    @staticmethod
    def symbolic(g,
                 input,
                 offset,
                 weight,
                 stride,
                 padding,
                 dilation,
                 groups,
                 deform_groups,
                 bias=False,
                 im2col_step=32):
        return g.op(
            'mmcv::MMCVDeformConv2d',
            input,
            offset,
            weight,
            stride_i=stride,
            padding_i=padding,
            dilation_i=dilation,
            groups_i=groups,
            deform_groups_i=deform_groups,
            bias_i=bias,
            im2col_step_i=im2col_step)

    @staticmethod
    def forward(ctx,
                input,
                offset,
                weight,
                stride=1,
                padding=0,
                dilation=1,
                groups=1,
                deform_groups=1,
                bias=False,
                im2col_step=32):
        if input is not None and input.dim() != 4:
            raise ValueError(
                f'Expected 4D tensor as input, got {input.dim()}D tensor \
                  instead.')
        assert bias is False, 'Only support bias is False.'
        ctx.stride = _pair(stride)
        ctx.padding = _pair(padding)
        ctx.dilation = _pair(dilation)
        ctx.groups = groups
        ctx.deform_groups = deform_groups
        ctx.im2col_step = im2col_step

        # When pytorch version >= 1.6.0, amp is adopted for fp16 mode;
        # amp won't cast the type of model (float32), but "offset" is cast
        # to float16 by nn.Conv2d automatically, leading to the type
        # mismatch with input (when it is float32) or weight.
        # The flag for whether to use fp16 or amp is the type of "offset",
        # we cast weight and input to temporarily support fp16 and amp
        # whatever the pytorch version is.
        input = input.type_as(offset)
        weight = weight.type_as(input)
        ctx.save_for_backward(input, offset, weight)

        output = input.new_empty(
            DeformConv2dFunction._output_size(ctx, input, weight))

        ctx.bufs_ = [input.new_empty(0), input.new_empty(0)]  # columns, ones

        cur_im2col_step = min(ctx.im2col_step, input.size(0))
        assert (input.size(0) %
                cur_im2col_step) == 0, 'im2col step must divide batchsize'
        ext_module.deform_conv_forward(
            input,
            weight,
            offset,
            output,
            ctx.bufs_[0],
            ctx.bufs_[1],
            kW=weight.size(3),
            kH=weight.size(2),
            dW=ctx.stride[1],
            dH=ctx.stride[0],
            padW=ctx.padding[1],
            padH=ctx.padding[0],
            dilationW=ctx.dilation[1],
            dilationH=ctx.dilation[0],
            group=ctx.groups,
            deformable_group=ctx.deform_groups,
            im2col_step=cur_im2col_step)
        return output

    @staticmethod
    @once_differentiable
    def backward(ctx, grad_output):
        input, offset, weight = ctx.saved_tensors

        grad_input = grad_offset = grad_weight = None

        cur_im2col_step = min(ctx.im2col_step, input.size(0))
        assert (input.size(0) % cur_im2col_step
                ) == 0, 'batch size must be divisible by im2col_step'

        grad_output = grad_output.contiguous()
        if ctx.needs_input_grad[0] or ctx.needs_input_grad[1]:
            grad_input = torch.zeros_like(input)
            grad_offset = torch.zeros_like(offset)
            ext_module.deform_conv_backward_input(
                input,
                offset,
                grad_output,
                grad_input,
                grad_offset,
                weight,
                ctx.bufs_[0],
                kW=weight.size(3),
                kH=weight.size(2),
                dW=ctx.stride[1],
                dH=ctx.stride[0],
                padW=ctx.padding[1],
                padH=ctx.padding[0],
                dilationW=ctx.dilation[1],
                dilationH=ctx.dilation[0],
                group=ctx.groups,
                deformable_group=ctx.deform_groups,
                im2col_step=cur_im2col_step)

        if ctx.needs_input_grad[2]:
            grad_weight = torch.zeros_like(weight)
            ext_module.deform_conv_backward_parameters(
                input,
                offset,
                grad_output,
                grad_weight,
                ctx.bufs_[0],
                ctx.bufs_[1],
                kW=weight.size(3),
                kH=weight.size(2),
                dW=ctx.stride[1],
                dH=ctx.stride[0],
                padW=ctx.padding[1],
                padH=ctx.padding[0],
                dilationW=ctx.dilation[1],
                dilationH=ctx.dilation[0],
                group=ctx.groups,
                deformable_group=ctx.deform_groups,
                scale=1,
                im2col_step=cur_im2col_step)

        return grad_input, grad_offset, grad_weight, \
            None, None, None, None, None, None, None

    @staticmethod
    def _output_size(ctx, input, weight):
        channels = weight.size(0)
        output_size = (input.size(0), channels)
        for d in range(input.dim() - 2):
            in_size = input.size(d + 2)
            pad = ctx.padding[d]
            kernel = ctx.dilation[d] * (weight.size(d + 2) - 1) + 1
            stride_ = ctx.stride[d]
            output_size += ((in_size + (2 * pad) - kernel) // stride_ + 1, )
        if not all(map(lambda s: s > 0, output_size)):
            raise ValueError(
                'convolution input is too small (output would be ' +
                'x'.join(map(str, output_size)) + ')')
        return output_size


deform_conv2d = DeformConv2dFunction.apply


class DeformConv2d(nn.Module):
    r"""Deformable 2D convolution.

    Applies a deformable 2D convolution over an input signal composed of
    several input planes. DeformConv2d was described in the paper
    `Deformable Convolutional Networks
    <https://arxiv.org/pdf/1703.06211.pdf>`_

    Note:
        The argument ``im2col_step`` was added in version 1.3.17, which means
        number of samples processed by the ``im2col_cuda_kernel`` per call.
        It enables users to define ``batch_size`` and ``im2col_step`` more
        flexibly and solved `issue mmcv#1440
        <https://github.com/open-mmlab/mmcv/issues/1440>`_.

    Args:
        in_channels (int): Number of channels in the input image.
        out_channels (int): Number of channels produced by the convolution.
        kernel_size(int, tuple): Size of the convolving kernel.
        stride(int, tuple): Stride of the convolution. Default: 1.
        padding (int or tuple): Zero-padding added to both sides of the input.
            Default: 0.
        dilation (int or tuple): Spacing between kernel elements. Default: 1.
        groups (int): Number of blocked connections from input.
            channels to output channels. Default: 1.
        deform_groups (int): Number of deformable group partitions.
        bias (bool): If True, adds a learnable bias to the output.
            Default: False.
        im2col_step (int): Number of samples processed by im2col_cuda_kernel
            per call. It will work when ``batch_size`` > ``im2col_step``, but
            ``batch_size`` must be divisible by ``im2col_step``. Default: 32.
            `New in version 1.3.17.`
    """

    @deprecated_api_warning({'deformable_groups': 'deform_groups'},
                            cls_name='DeformConv2d')
    def __init__(self,
                 in_channels: int,
                 out_channels: int,
                 kernel_size: Union[int, Tuple[int, ...]],
                 stride: Union[int, Tuple[int, ...]] = 1,
                 padding: Union[int, Tuple[int, ...]] = 0,
                 dilation: Union[int, Tuple[int, ...]] = 1,
                 groups: int = 1,
                 deform_groups: int = 1,
                 bias: bool = False,
                 im2col_step: int = 32) -> None:
        super(DeformConv2d, self).__init__()

        assert not bias, \
            f'bias={bias} is not supported in DeformConv2d.'
        assert in_channels % groups == 0, \
            f'in_channels {in_channels} cannot be divisible by groups {groups}'
        assert out_channels % groups == 0, \
            f'out_channels {out_channels} cannot be divisible by groups \
              {groups}'

        self.in_channels = in_channels
        self.out_channels = out_channels
        self.kernel_size = _pair(kernel_size)
        self.stride = _pair(stride)
        self.padding = _pair(padding)
        self.dilation = _pair(dilation)
        self.groups = groups
        self.deform_groups = deform_groups
        self.im2col_step = im2col_step
        # enable compatibility with nn.Conv2d
        self.transposed = False
        self.output_padding = _single(0)

        # only weight, no bias
        self.weight = nn.Parameter(
            torch.Tensor(out_channels, in_channels // self.groups,
                         *self.kernel_size))

        self.reset_parameters()

    def reset_parameters(self):
        # switch the initialization of `self.weight` to the standard kaiming
        # method described in `Delving deep into rectifiers: Surpassing
        # human-level performance on ImageNet classification` - He, K. et al.
        # (2015), using a uniform distribution
        nn.init.kaiming_uniform_(self.weight, nonlinearity='relu')

    def forward(self, x: Tensor, offset: Tensor) -> Tensor:
        """Deformable Convolutional forward function.

        Args:
            x (Tensor): Input feature, shape (B, C_in, H_in, W_in)
            offset (Tensor): Offset for deformable convolution, shape
                (B, deform_groups*kernel_size[0]*kernel_size[1]*2,
                H_out, W_out), H_out, W_out are equal to the output's.

                An offset is like `[y0, x0, y1, x1, y2, x2, ..., y8, x8]`.
                The spatial arrangement is like:

                .. code:: text

                    (x0, y0) (x1, y1) (x2, y2)
                    (x3, y3) (x4, y4) (x5, y5)
                    (x6, y6) (x7, y7) (x8, y8)

        Returns:
            Tensor: Output of the layer.
        """
        # To fix an assert error in deform_conv_cuda.cpp:128
        # input image is smaller than kernel
        input_pad = (x.size(2) < self.kernel_size[0]) or (x.size(3) <
                                                          self.kernel_size[1])
        if input_pad:
            pad_h = max(self.kernel_size[0] - x.size(2), 0)
            pad_w = max(self.kernel_size[1] - x.size(3), 0)
            x = F.pad(x, (0, pad_w, 0, pad_h), 'constant', 0).contiguous()
            offset = F.pad(offset, (0, pad_w, 0, pad_h), 'constant', 0)
            offset = offset.contiguous()
        out = deform_conv2d(x, offset, self.weight, self.stride, self.padding,
                            self.dilation, self.groups, self.deform_groups,
                            False, self.im2col_step)
        if input_pad:
            out = out[:, :, :out.size(2) - pad_h, :out.size(3) -
                      pad_w].contiguous()
        return out

    def __repr__(self):
        s = self.__class__.__name__
        s += f'(in_channels={self.in_channels},\n'
        s += f'out_channels={self.out_channels},\n'
        s += f'kernel_size={self.kernel_size},\n'
        s += f'stride={self.stride},\n'
        s += f'padding={self.padding},\n'
        s += f'dilation={self.dilation},\n'
        s += f'groups={self.groups},\n'
        s += f'deform_groups={self.deform_groups},\n'
        # bias is not supported in DeformConv2d.
        s += 'bias=False)'
        return s


@CONV_LAYERS.register_module('DCN')
class DeformConv2dPack(DeformConv2d):
    """A Deformable Conv Encapsulation that acts as normal Conv layers.

    The offset tensor is like `[y0, x0, y1, x1, y2, x2, ..., y8, x8]`.
    The spatial arrangement is like:

    .. code:: text

        (x0, y0) (x1, y1) (x2, y2)
        (x3, y3) (x4, y4) (x5, y5)
        (x6, y6) (x7, y7) (x8, y8)

    Args:
        in_channels (int): Same as nn.Conv2d.
        out_channels (int): Same as nn.Conv2d.
        kernel_size (int or tuple[int]): Same as nn.Conv2d.
        stride (int or tuple[int]): Same as nn.Conv2d.
        padding (int or tuple[int]): Same as nn.Conv2d.
        dilation (int or tuple[int]): Same as nn.Conv2d.
        groups (int): Same as nn.Conv2d.
        bias (bool or str): If specified as `auto`, it will be decided by the
            norm_cfg. Bias will be set as True if norm_cfg is None, otherwise
            False.
    """

    _version = 2

    def __init__(self, *args, **kwargs):
        super(DeformConv2dPack, self).__init__(*args, **kwargs)
        self.conv_offset = nn.Conv2d(
            self.in_channels,
            self.deform_groups * 2 * self.kernel_size[0] * self.kernel_size[1],
            kernel_size=self.kernel_size,
            stride=_pair(self.stride),
            padding=_pair(self.padding),
            dilation=_pair(self.dilation),
            bias=True)
        self.init_offset()

    def init_offset(self):
        self.conv_offset.weight.data.zero_()
        self.conv_offset.bias.data.zero_()

    def forward(self, x):
        offset = self.conv_offset(x)
        return deform_conv2d(x, offset, self.weight, self.stride, self.padding,
                             self.dilation, self.groups, self.deform_groups,
                             False, self.im2col_step)

    def _load_from_state_dict(self, state_dict, prefix, local_metadata, strict,
                              missing_keys, unexpected_keys, error_msgs):
        version = local_metadata.get('version', None)

        if version is None or version < 2:
            # the key is different in early versions
            # In version < 2, DeformConvPack loads previous benchmark models.
            if (prefix + 'conv_offset.weight' not in state_dict
                    and prefix[:-1] + '_offset.weight' in state_dict):
                state_dict[prefix + 'conv_offset.weight'] = state_dict.pop(
                    prefix[:-1] + '_offset.weight')
            if (prefix + 'conv_offset.bias' not in state_dict
                    and prefix[:-1] + '_offset.bias' in state_dict):
                state_dict[prefix +
                           'conv_offset.bias'] = state_dict.pop(prefix[:-1] +
                                                                '_offset.bias')

        if version is not None and version > 1:
            print_log(
                f'DeformConv2dPack {prefix.rstrip(".")} is upgraded to '
                'version 2.',
                logger='root')

        super()._load_from_state_dict(state_dict, prefix, local_metadata,
                                      strict, missing_keys, unexpected_keys,
                                      error_msgs)


```

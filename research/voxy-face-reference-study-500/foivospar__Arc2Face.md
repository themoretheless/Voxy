# Facial rendering source scan: foivospar/Arc2Face

Repository: https://github.com/foivospar/Arc2Face

Pinned source revision: 8f3acd701d17fda7fde4c9f1d170fc88fecbe9ad

Discovery topic: blendshape

Description: [ECCV 2024 Oral 🔥] Arc2Face: A Foundation Model for ID-Consistent Human Faces ------------------------ [ICCVW 2025] ID-Consistent, Precise Expression Generation with Blendshape-Guided Diffusion

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["arc2face/__init__.py", "arc2face/exp_utils.py", "arc2face/models.py", "arc2face/ref_adapter.py", "arc2face/utils.py", "assets/arc2face_exp.jpg"]}

## Source file: arc2face/utils.py

{
  "path": "arc2face/utils.py",
  "blob_sha": "7c6a1ab17316a0c3063b6bbcaf8c8cff728eb604",
  "sha256": "307a89cdc8898f2c8d887f192b1402123c808adfcf1b3c0f791a7e6516b00439",
  "lines": 124,
  "bytes": 4600,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 10,
    "mouth": 8,
    "rig": 6,
    "face": 8,
    "lip": 5
  },
  "url": "https://github.com/foivospar/Arc2Face/blob/8f3acd701d17fda7fde4c9f1d170fc88fecbe9ad/arc2face/utils.py",
  "function_signature_candidates": [
    {
      "line": 8,
      "text": "def project_face_embs(pipeline, face_embs):"
    },
    {
      "line": 36,
      "text": "def image_align(img,"
    }
  ]
}

```text

import scipy
import PIL
import numpy as np
import torch
import torch.nn.functional as F

@torch.no_grad()
def project_face_embs(pipeline, face_embs):

    '''
    face_embs: (N, 512) normalized ArcFace embeddings
    '''

    arcface_token_id = pipeline.tokenizer.encode("id", add_special_tokens=False)[0]

    input_ids = pipeline.tokenizer(
            "photo of a id person",
            truncation=True,
            padding="max_length",
            max_length=pipeline.tokenizer.model_max_length,
            return_tensors="pt",
        ).input_ids.to(pipeline.device)

    face_embs_padded = F.pad(face_embs, (0, pipeline.text_encoder.config.hidden_size-512), "constant", 0)
    token_embs = pipeline.text_encoder(input_ids=input_ids.repeat(len(face_embs), 1), return_token_embs=True)
    token_embs[input_ids==arcface_token_id] = face_embs_padded

    prompt_embeds = pipeline.text_encoder(
        input_ids=input_ids,
        input_token_embs=token_embs
    )[0]

    return prompt_embeds


def image_align(img,
                face_landmarks,
                output_size=1024,
                transform_size=4096,
                enable_padding=True):
    # Align function from FFHQ dataset pre-processing step
    # https://github.com/NVlabs/ffhq-dataset/blob/master/download_ffhq.py

    lm = face_landmarks
    lm_eye_left = lm[36:42]  # left-clockwise
    lm_eye_right = lm[42:48]  # left-clockwise
    lm_mouth_outer = lm[48:60]  # left-clockwise

    # Calculate auxiliary vectors.
    eye_left = np.mean(lm_eye_left, axis=0)
    eye_right = np.mean(lm_eye_right, axis=0)
    eye_avg = (eye_left + eye_right) * 0.5
    eye_to_eye = eye_right - eye_left
    mouth_left = lm_mouth_outer[0]
    mouth_right = lm_mouth_outer[6]
    mouth_avg = (mouth_left + mouth_right) * 0.5
    eye_to_mouth = mouth_avg - eye_avg

    # Choose oriented crop rectangle.
    x = eye_to_eye - np.flipud(eye_to_mouth) * [-1, 1]
    x /= np.hypot(*x)
    x *= max(np.hypot(*eye_to_eye) * 2.0, np.hypot(*eye_to_mouth) * 1.8)
    y = np.flipud(x) * [-1, 1]
    c = eye_avg + eye_to_mouth * 0.1
    quad = np.stack([c - x - y, c - x + y, c + x + y, c + x - y])
    qsize = np.hypot(*x) * 2
    
    img = img.convert('RGB')

    # Shrink.
    shrink = int(np.floor(qsize / output_size * 0.5))
    if shrink > 1:
        rsize = (int(np.rint(float(img.size[0]) / shrink)),
                 int(np.rint(float(img.size[1]) / shrink)))
        img = img.resize(rsize, PIL.Image.LANCZOS)
        quad /= shrink
        qsize /= shrink

    # Crop.
    border = max(int(np.rint(qsize * 0.1)), 3)
    crop = (int(np.floor(min(quad[:, 0]))), int(np.floor(min(quad[:, 1]))),
            int(np.ceil(max(quad[:, 0]))), int(np.ceil(max(quad[:, 1]))))
    crop = (max(crop[0] - border, 0), max(crop[1] - border, 0),
            min(crop[2] + border,
                img.size[0]), min(crop[3] + border, img.size[1]))
    if crop[2] - crop[0] < img.size[0] or crop[3] - crop[1] < img.size[1]:
        img = img.crop(crop)
        quad -= crop[0:2]

    # Pad.
    pad = (int(np.floor(min(quad[:, 0]))), int(np.floor(min(quad[:, 1]))),
           int(np.ceil(max(quad[:, 0]))), int(np.ceil(max(quad[:, 1]))))
    pad = (max(-pad[0] + border,
               0), max(-pad[1] + border,
                       0), max(pad[2] - img.size[0] + border,
                               0), max(pad[3] - img.size[1] + border, 0))
    if enable_padding and max(pad) > border - 4:
        pad = np.maximum(pad, int(np.rint(qsize * 0.3)))
        img = np.pad(np.float32(img),
                     ((pad[1], pad[3]), (pad[0], pad[2]), (0, 0)), 'reflect')
        h, w, _ = img.shape
        y, x, _ = np.ogrid[:h, :w, :1]
        mask = np.maximum(
            1.0 -
            np.minimum(np.float32(x) / pad[0],
                       np.float32(w - 1 - x) / pad[2]), 1.0 -
            np.minimum(np.float32(y) / pad[1],
                       np.float32(h - 1 - y) / pad[3]))
        blur = qsize * 0.02
        img += (scipy.ndimage.gaussian_filter(img, [blur, blur, 0]) -
                img) * np.clip(mask * 3.0 + 1.0, 0.0, 1.0)
        img += (np.median(img, axis=(0, 1)) - img) * np.clip(mask, 0.0, 1.0)
        img = PIL.Image.fromarray(np.uint8(np.clip(np.rint(img), 0, 255)),
                                  'RGB')
        quad += pad[:2]


    # Transform.
    img = img.transform((transform_size, transform_size), PIL.Image.QUAD,
                        (quad + 0.5).flatten(), PIL.Image.BILINEAR)
    if output_size < transform_size:
        img = img.resize((output_size, output_size), PIL.Image.LANCZOS)

    return img

```

## Source file: arc2face/models.py

{
  "path": "arc2face/models.py",
  "blob_sha": "fe9d5df40c374c694591bc745b2e012e5a04cf2b",
  "sha256": "b7ab5373bae3582583dbc924eba6c91ab5e4743316a609f69859787e79f89021",
  "lines": 92,
  "bytes": 4985,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 1,
    "lip": 7
  },
  "url": "https://github.com/foivospar/Arc2Face/blob/8f3acd701d17fda7fde4c9f1d170fc88fecbe9ad/arc2face/models.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def forward("
    }
  ]
}

```text

import torch
from transformers import CLIPTextModel
from typing import Any, Callable, Dict, Optional, Tuple, Union, List
from transformers.modeling_outputs import BaseModelOutputWithPooling
#from transformers.models.clip.modeling_clip import _make_causal_mask, _expand_mask
from transformers.modeling_attn_mask_utils import _create_4d_causal_attention_mask, _prepare_4d_attention_mask  # transformers 4.36.0


class CLIPTextModelWrapper(CLIPTextModel):
    # Adapted from https://github.com/huggingface/transformers/blob/v4.34.1/src/transformers/models/clip/modeling_clip.py#L812
    # Modified to accept precomputed token embeddings "input_token_embs" as input or calculate them from input_ids and return them.
    def forward(
        self,
        input_ids: Optional[torch.Tensor] = None,
        attention_mask: Optional[torch.Tensor] = None,
        position_ids: Optional[torch.Tensor] = None,
        output_attentions: Optional[bool] = None,
        output_hidden_states: Optional[bool] = None,
        return_dict: Optional[bool] = None,
        input_token_embs: Optional[torch.Tensor] = None,
        return_token_embs: Optional[bool] = False,
    ) -> Union[Tuple, torch.Tensor, BaseModelOutputWithPooling]:

        if return_token_embs:
            return self.text_model.embeddings.token_embedding(input_ids)
    
        return_dict = return_dict if return_dict is not None else self.config.use_return_dict
    
        output_attentions = output_attentions if output_attentions is not None else self.text_model.config.output_attentions
        output_hidden_states = (
            output_hidden_states if output_hidden_states is not None else self.text_model.config.output_hidden_states
        )
        return_dict = return_dict if return_dict is not None else self.text_model.config.use_return_dict
    
        if input_ids is None:
            raise ValueError("You have to specify input_ids")
    
        input_shape = input_ids.size()
        input_ids = input_ids.view(-1, input_shape[-1])
    
        hidden_states = self.text_model.embeddings(input_ids=input_ids, position_ids=position_ids, inputs_embeds=input_token_embs)
    
        # CLIP's text model uses causal mask, prepare it here.
        # https://github.com/openai/CLIP/blob/cfcffb90e69f37bf2ff1e988237a0fbe41f33c04/clip/model.py#L324
        causal_attention_mask = _create_4d_causal_attention_mask(input_shape, hidden_states.dtype, device=hidden_states.device)
        # expand attention_mask
        if attention_mask is not None:
            # [bsz, seq_len] -> [bsz, 1, tgt_seq_len, src_seq_len]
            attention_mask = _prepare_4d_attention_mask(attention_mask, hidden_states.dtype)
    
        encoder_outputs = self.text_model.encoder(
            inputs_embeds=hidden_states,
            attention_mask=attention_mask,
            causal_attention_mask=causal_attention_mask,
            output_attentions=output_attentions,
            output_hidden_states=output_hidden_states,
            return_dict=return_dict,
        )
    
        last_hidden_state = encoder_outputs[0]
        last_hidden_state = self.text_model.final_layer_norm(last_hidden_state)
    
        if self.text_model.eos_token_id == 2:
            # The `eos_token_id` was incorrect before PR #24773: Let's keep what have been done here.
            # A CLIP model with such `eos_token_id` in the config can't work correctly with extra new tokens added
            # ------------------------------------------------------------
            # text_embeds.shape = [batch_size, sequence_length, transformer.width]
            # take features from the eot embedding (eot_token is the highest number in each sequence)
            # casting to torch.int for onnx compatibility: argmax doesn't support int64 inputs with opset 14
            pooled_output = last_hidden_state[
                torch.arange(last_hidden_state.shape[0], device=last_hidden_state.device),
                input_ids.to(dtype=torch.int, device=last_hidden_state.device).argmax(dim=-1),
            ]
        else:
            # The config gets updated `eos_token_id` from PR #24773 (so the use of exta new tokens is possible)
            pooled_output = last_hidden_state[
                torch.arange(last_hidden_state.shape[0], device=last_hidden_state.device),
                # We need to get the first position of `eos_token_id` value (`pad_token_ids` might equal to `eos_token_id`)
                (input_ids.to(dtype=torch.int, device=last_hidden_state.device) == self.text_model.eos_token_id)
                .int()
                .argmax(dim=-1),
            ]
    
        if not return_dict:
            return (last_hidden_state, pooled_output) + encoder_outputs[1:]
    
        return BaseModelOutputWithPooling(
            last_hidden_state=last_hidden_state,
            pooler_output=pooled_output,
            hidden_states=encoder_outputs.hidden_states,
            attentions=encoder_outputs.attentions,
        )

```

## Source file: arc2face/__init__.py

{
  "path": "arc2face/__init__.py",
  "blob_sha": "b1088fd6de9d94c7e02f757bda233a2f9359b6b5",
  "sha256": "bc08cc546e35475c097312a6d47b4eace72ffe4258eca82a186012108cbc5dce",
  "lines": 3,
  "bytes": 132,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/foivospar/Arc2Face/blob/8f3acd701d17fda7fde4c9f1d170fc88fecbe9ad/arc2face/__init__.py",
  "function_signature_candidates": []
}

```text

from .models import CLIPTextModelWrapper
from .utils import project_face_embs, image_align
from .ref_adapter import ReferenceAdapter

```

## Source file: arc2face/exp_utils.py

{
  "path": "arc2face/exp_utils.py",
  "blob_sha": "20ca71eec06d4a4b187f46e3200c6dbb8330d0d0",
  "sha256": "5a4cf4e5353116dbfb8cc8d528cb0dfc62212150555ac020837079f923941b54",
  "lines": 131,
  "bytes": 4770,
  "selection_score": 3,
  "keyword_line_counts": {
    "eye": 2,
    "blendshape": 1,
    "jaw": 2,
    "flame": 1,
    "rig": 3,
    "face": 17
  },
  "url": "https://github.com/foivospar/Arc2Face/blob/8f3acd701d17fda7fde4c9f1d170fc88fecbe9ad/arc2face/exp_utils.py",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "def run_mediapipe(image):"
    },
    {
      "line": 54,
      "text": "def crop_face(frame, landmarks, scale=1.0, image_size=224):"
    },
    {
      "line": 74,
      "text": "def create_backbone(backbone_name, pretrained=True):"
    },
    {
      "line": 82,
      "text": "def __init__(self, n_exp=50) -> None:"
    },
    {
      "line": 95,
      "text": "def init_weights(self):"
    },
    {
      "line": 100,
      "text": "def forward(self, img):"
    },
    {
      "line": 117,
      "text": "def run_smirk(encoder, image, device='cuda'):"
    }
  ]
}

```text

''' 
FLAME regression code borrowed from SMIRK:
    - https://github.com/georgeretsi/smirk/blob/main/utils/mediapipe_utils.py
    - https://github.com/georgeretsi/smirk/blob/main/src/smirk_encoder.py
    - https://github.com/georgeretsi/smirk/blob/main/demo.py
'''

import mediapipe as mp
from mediapipe.tasks import python
from mediapipe.tasks.python import vision
import cv2
import numpy as np
from skimage.transform import estimate_transform, warp

import torch
import torch.nn.functional as F
from torch import nn
import timm

base_options = python.BaseOptions(model_asset_path='models/smirk/face_landmarker.task')
options = vision.FaceLandmarkerOptions(base_options=base_options,
                                    output_face_blendshapes=True,
                                    output_facial_transformation_matrixes=True,
                                    num_faces=1,
                                    min_face_detection_confidence=0.1,
                                    min_face_presence_confidence=0.1
                                    )
detector = vision.FaceLandmarker.create_from_options(options)

def run_mediapipe(image):
    # print(image.shape)    
    image_numpy = cv2.cvtColor(image,cv2.COLOR_BGR2RGB)

    # STEP 3: Load the input image.
    image = mp.Image(image_format=mp.ImageFormat.SRGB, data=image_numpy)


    # STEP 4: Detect face landmarks from the input image.
    detection_result = detector.detect(image)

    if len (detection_result.face_landmarks) == 0:
        print('No face detected')
        return None
    
    face_landmarks = detection_result.face_landmarks[0]

    face_landmarks_numpy = np.zeros((478, 3))

    for i, landmark in enumerate(face_landmarks):
        face_landmarks_numpy[i] = [landmark.x*image.width, landmark.y*image.height, landmark.z]

    return face_landmarks_numpy

def crop_face(frame, landmarks, scale=1.0, image_size=224):
    left = np.min(landmarks[:, 0])
    right = np.max(landmarks[:, 0])
    top = np.min(landmarks[:, 1])
    bottom = np.max(landmarks[:, 1])

    h, w, _ = frame.shape
    old_size = (right - left + bottom - top) / 2
    center = np.array([right - (right - left) / 2.0, bottom - (bottom - top) / 2.0])

    size = int(old_size * scale)

    # crop image
    src_pts = np.array([[center[0] - size / 2, center[1] - size / 2], [center[0] - size / 2, center[1] + size / 2],
                        [center[0] + size / 2, center[1] - size / 2]])
    DST_PTS = np.array([[0, 0], [0, image_size - 1], [image_size - 1, 0]])
    tform = estimate_transform('similarity', src_pts, DST_PTS)

    return tform

def create_backbone(backbone_name, pretrained=True):
    backbone = timm.create_model(backbone_name, 
                        pretrained=pretrained,
                        features_only=True)
    feature_dim = backbone.feature_info[-1]['num_chs']
    return backbone, feature_dim

class ExpressionEncoder(nn.Module):
    def __init__(self, n_exp=50) -> None:
        super().__init__()

        self.encoder, feature_dim = create_backbone('tf_mobilenetv3_large_minimal_100')
        
        self.expression_layers = nn.Sequential( 
            nn.Linear(feature_dim, n_exp+2+3) # num expressions + jaw + eyelid
        )

        self.n_exp = n_exp
        self.init_weights()


    def init_weights(self):
        self.expression_layers[-1].weight.data *= 0.1
        self.expression_layers[-1].bias.data *= 0.1


    def forward(self, img):
        features = self.encoder(img)[-1]
            
        features = F.adaptive_avg_pool2d(features, (1, 1)).squeeze(-1).squeeze(-1)


        parameters = self.expression_layers(features).reshape(img.size(0), -1)

        outputs = {}

        outputs['expression_params'] = parameters[...,:self.n_exp]
        outputs['eyelid_params'] = torch.clamp(parameters[...,self.n_exp:self.n_exp+2], 0, 1)
        outputs['jaw_params'] = torch.cat([F.relu(parameters[...,self.n_exp+2].unsqueeze(-1)), 
                                           torch.clamp(parameters[...,self.n_exp+3:self.n_exp+5], -.2, .2)], dim=-1)

        return outputs

def run_smirk(encoder, image, device='cuda'):

    kpt_mediapipe = run_mediapipe(image[:,:,::-1])
    if kpt_mediapipe is None:
        return None
    kpt_mediapipe = kpt_mediapipe[..., :2]
    tform = crop_face(image, kpt_mediapipe, scale=1.4, image_size=224)
    cropped_image = warp(image, tform.inverse, output_shape=(224, 224), preserve_range=True).astype(np.uint8)
    cropped_image = cv2.cvtColor(cropped_image, cv2.COLOR_BGR2RGB)
    cropped_image = cv2.resize(cropped_image, (224,224))
    cropped_image = torch.tensor(cropped_image).permute(2,0,1).unsqueeze(0).float()/255.0
    with torch.no_grad():
        outputs = encoder(cropped_image.to(device))

    return outputs

```

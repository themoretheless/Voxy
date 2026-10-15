# Facial rendering source scan: host2india/Lip2Sync-3D-Talker

Repository: https://github.com/host2india/Lip2Sync-3D-Talker

Pinned source revision: 00223506993a4355ad3b677e5f81760fcac7669c

Discovery topic: lip sync 3D

Description: Lip2Sync 3D Talking Face Engine — integrated with SadTalker, GFPGAN, RealESRGAN, and 3DMM (BFM). Custom engine for high-quality audio-driven face animation.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["models/wav2lip/face_detection/README.md", "models/wav2lip/face_detection/__init__.py", "models/wav2lip/face_detection/api.py", "models/wav2lip/face_detection/detection/__init__.py", "models/wav2lip/face_detection/detection/core.py", "models/wav2lip/face_detection/detection/sfd/__init__.py", "models/wav2lip/face_detection/detection/sfd/bbox.py", "models/wav2lip/face_detection/detection/sfd/detect.py", "models/wav2lip/face_detection/detection/sfd/net_s3fd.py", "models/wav2lip/face_detection/detection/sfd/sfd_detector.py", "models/wav2lip/face_detection/models.py", "models/wav2lip/face_detection/utils.py", "models/wav2lip_src/face_detection/README.md", "models/wav2lip_src/face_detection/__init__.py", "models/wav2lip_src/face_detection/api.py", "models/wav2lip_src/face_detection/detection/__init__.py", "models/wav2lip_src/face_detection/detection/core.py", "models/wav2lip_src/face_detection/detection/sfd/__init__.py", "models/wav2lip_src/face_detection/detection/sfd/bbox.py", "models/wav2lip_src/face_detection/detection/sfd/detect.py", "models/wav2lip_src/face_detection/detection/sfd/net_s3fd.py", "models/wav2lip_src/face_detection/detection/sfd/sfd_detector.py", "models/wav2lip_src/face_detection/models.py", "models/wav2lip_src/face_detection/utils.py"], "lip": ["Lip2Sync_GPU_Template.zip", "Lip2Sync_clean.zip", "Lip2Sync_clean_ready.zip", "app/engines/wav2lip/README.md", "app/engines/wav2lip/engine.py", "app/engines/wav2lip/utils.py", "app/engines/wav2lip_single_image/README.md", "app/engines/wav2lip_single_image/__init__.py", "app/engines/wav2lip_single_image/engine.py", "app/engines/wav2lip_single_image/utils.py", "app/routes/wav2lip.py", "app/routes/wav2lip_single_image.py", "app/wav2lip_engine.py", "models/wav2lip/README.md", "models/wav2lip/checkpoints/README.md", "models/wav2lip/checkpoints/s3fd.pth", "models/wav2lip/checkpoints/wav2lip.pth", "models/wav2lip/checkpoints/wav2lip_gan.pth", "models/wav2lip/face_detection/README.md", "models/wav2lip/face_detection/__init__.py", "models/wav2lip/face_detection/api.py", "models/wav2lip/face_detection/detection/__init__.py", "models/wav2lip/face_detection/detection/core.py", "models/wav2lip/face_detection/detection/sfd/__init__.py", "models/wav2lip/face_detection/detection/sfd/bbox.py", "models/wav2lip/face_detection/detection/sfd/detect.py", "models/wav2lip/face_detection/detection/sfd/net_s3fd.py", "models/wav2lip/face_detection/detection/sfd/sfd_detector.py", "models/wav2lip/face_detection/models.py", "models/wav2lip/face_detection/utils.py"]}

## Source file: app/routes/wav2lip.py

{
  "path": "app/routes/wav2lip.py",
  "blob_sha": "8485623843c4a67762202486d6c3928488a35a04",
  "sha256": "e541f0d70c706db301c61b7808fb2b6809233caac98f46350a3db6f509677570",
  "lines": 24,
  "bytes": 1053,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 7
  },
  "url": "https://github.com/host2india/Lip2Sync-3D-Talker/blob/00223506993a4355ad3b677e5f81760fcac7669c/app/routes/wav2lip.py",
  "function_signature_candidates": [
    {
      "line": 17,
      "text": "async def sync_wav2lip(video: UploadFile = File(...), audio: UploadFile = File(None)):"
    }
  ]
}

```text

# app/routes/wav2lip.py
import os
from fastapi import APIRouter, UploadFile, File
from fastapi.responses import FileResponse, JSONResponse
from app.engines.wav2lip.engine import Wav2LipEngine

router = APIRouter()

# Use WORKSPACE env var when running in Pod; locally it will default to "."
workspace_env = os.environ.get("WORKSPACE", ".")
# model path (if your repo contains Wav2Lip folder, use "Wav2Lip")
model_path = os.environ.get("WAV2LIP_MODEL_PATH", "Wav2Lip")

engine = Wav2LipEngine(model_path=model_path, workspace=workspace_env)

@router.post("/sync/wav2lip")
async def sync_wav2lip(video: UploadFile = File(...), audio: UploadFile = File(None)):
    video_bytes = await video.read()
    audio_bytes = await audio.read() if audio else None

    result = engine.run(video_bytes, audio_bytes)
    if result.get("status") == "success":
        return FileResponse(result["output_path"], media_type="video/mp4", filename="video_sync.mp4")
    return JSONResponse({"status": "error", "details": result.get("details", "unknown")}, status_code=500)


```

## Source file: app/wav2lip_engine.py

{
  "path": "app/wav2lip_engine.py",
  "blob_sha": "0db637cc489907af68003825555ecd8806130a5a",
  "sha256": "f0cf60973d6de60b52476a3c52e27e6dd8446628715eceb1fc02dcef21d956dc",
  "lines": 69,
  "bytes": 2478,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 1,
    "lip": 8
  },
  "url": "https://github.com/host2india/Lip2Sync-3D-Talker/blob/00223506993a4355ad3b677e5f81760fcac7669c/app/wav2lip_engine.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def ffmpeg_merge_image_audio(image_path, audio_path, out_path):"
    },
    {
      "line": 27,
      "text": "def synthesize_lips(input_video_path, input_audio_path, output_path=\"uploads/wav2lip_output.mp4\"):"
    }
  ]
}

```text

import os, subprocess, shutil
from pathlib import Path

MODELS_DIR = Path("models/wav2lip")
WAV2LIP_SCRIPT = Path("wav2lip/inference.py")  # if you add real inference here, engine will call it

def ffmpeg_merge_image_audio(image_path, audio_path, out_path):
    # Make a simple video from the image + audio (loop image, add audio)
    # uses ffmpeg, expects it to be installed on the system
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "ffmpeg", "-y",
        "-loop", "1",
        "-i", str(image_path),
        "-i", str(audio_path),
        "-c:v", "libx264",
        "-c:a", "aac",
        "-b:a", "192k",
        "-shortest",
        "-pix_fmt", "yuv420p",
        str(out_path)
    ]
    subprocess.run(cmd, check=True)
    return str(out_path)

def synthesize_lips(input_video_path, input_audio_path, output_path="uploads/wav2lip_output.mp4"):
    """
    Prefer real inference script: wav2lip/inference.py
    If it's not present, fallback to a simple ffmpeg image+audio merge.
    - If `input_video_path` is an image (jpg/png) it will use image fallback.
    - If it's a video and real inference is missing, we will copy video and replace audio.
    """
    input_video = Path(input_video_path)
    output = Path(output_path)
    output.parent.mkdir(parents=True, exist_ok=True)

    # If real inference exists, call it (expects an API similar to Wav2Lip repo)
    if WAV2LIP_SCRIPT.exists():
        cmd = [
            "python", str(WAV2LIP_SCRIPT),
            "--checkpoint_path", str(MODELS_DIR / "wav2lip.pth"),
            "--face", str(input_video_path),
            "--audio", str(input_audio_path),
            "--outfile", str(output)
        ]
        subprocess.run(cmd, check=True)
        return str(output)

    # Fallback behavior:
    # If provided a video, replace its audio using ffmpeg
    suffix = input_video.suffix.lower()
    if suffix in [".jpg", ".jpeg", ".png", ".webp"]:
        return ffmpeg_merge_image_audio(input_video, input_audio_path, output)
    else:
        # replace audio of the input video (quick fallback)
        cmd = [
            "ffmpeg", "-y",
            "-i", str(input_video),
            "-i", str(input_audio_path),
            "-c:v", "copy",
            "-c:a", "aac",
            "-map", "0:v:0",
            "-map", "1:a:0",
            "-shortest",
            str(output)
        ]
        subprocess.run(cmd, check=True)
        return str(output)


```

## Source file: models/wav2lip/wav2lip/wav2lip.py

{
  "path": "models/wav2lip/wav2lip/wav2lip.py",
  "blob_sha": "ae5d6919169ec497f0f0815184f5db8ba9108fbd",
  "sha256": "a02b54288f8200332ee4f0ea0c8013bca542b94d8149e514ac25a742a7a70e66",
  "lines": 184,
  "bytes": 8579,
  "selection_score": 15,
  "keyword_line_counts": {
    "face": 25,
    "lip": 4
  },
  "url": "https://github.com/host2india/Lip2Sync-3D-Talker/blob/00223506993a4355ad3b677e5f81760fcac7669c/models/wav2lip/wav2lip/wav2lip.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def __init__(self):"
    },
    {
      "line": 87,
      "text": "def forward(self, audio_sequences, face_sequences):"
    },
    {
      "line": 128,
      "text": "def __init__(self):"
    },
    {
      "line": 155,
      "text": "def get_lower_half(self, face_sequences):"
    },
    {
      "line": 158,
      "text": "def to_2d(self, face_sequences):"
    },
    {
      "line": 163,
      "text": "def perceptual_forward(self, false_face_sequences):"
    },
    {
      "line": 176,
      "text": "def forward(self, face_sequences):"
    }
  ]
}

```text

import torch
from torch import nn
from torch.nn import functional as F
import math

from .conv import Conv2dTranspose, Conv2d, nonorm_Conv2d

class Wav2Lip(nn.Module):
    def __init__(self):
        super(Wav2Lip, self).__init__()

        self.face_encoder_blocks = nn.ModuleList([
            nn.Sequential(Conv2d(6, 16, kernel_size=7, stride=1, padding=3)), # 96,96

            nn.Sequential(Conv2d(16, 32, kernel_size=3, stride=2, padding=1), # 48,48
            Conv2d(32, 32, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(32, 32, kernel_size=3, stride=1, padding=1, residual=True)),

            nn.Sequential(Conv2d(32, 64, kernel_size=3, stride=2, padding=1),    # 24,24
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True)),

            nn.Sequential(Conv2d(64, 128, kernel_size=3, stride=2, padding=1),   # 12,12
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True)),

            nn.Sequential(Conv2d(128, 256, kernel_size=3, stride=2, padding=1),       # 6,6
            Conv2d(256, 256, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(256, 256, kernel_size=3, stride=1, padding=1, residual=True)),

            nn.Sequential(Conv2d(256, 512, kernel_size=3, stride=2, padding=1),     # 3,3
            Conv2d(512, 512, kernel_size=3, stride=1, padding=1, residual=True),),
            
            nn.Sequential(Conv2d(512, 512, kernel_size=3, stride=1, padding=0),     # 1, 1
            Conv2d(512, 512, kernel_size=1, stride=1, padding=0)),])

        self.audio_encoder = nn.Sequential(
            Conv2d(1, 32, kernel_size=3, stride=1, padding=1),
            Conv2d(32, 32, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(32, 32, kernel_size=3, stride=1, padding=1, residual=True),

            Conv2d(32, 64, kernel_size=3, stride=(3, 1), padding=1),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),

            Conv2d(64, 128, kernel_size=3, stride=3, padding=1),
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True),

            Conv2d(128, 256, kernel_size=3, stride=(3, 2), padding=1),
            Conv2d(256, 256, kernel_size=3, stride=1, padding=1, residual=True),

            Conv2d(256, 512, kernel_size=3, stride=1, padding=0),
            Conv2d(512, 512, kernel_size=1, stride=1, padding=0),)

        self.face_decoder_blocks = nn.ModuleList([
            nn.Sequential(Conv2d(512, 512, kernel_size=1, stride=1, padding=0),),

            nn.Sequential(Conv2dTranspose(1024, 512, kernel_size=3, stride=1, padding=0), # 3,3
            Conv2d(512, 512, kernel_size=3, stride=1, padding=1, residual=True),),

            nn.Sequential(Conv2dTranspose(1024, 512, kernel_size=3, stride=2, padding=1, output_padding=1),
            Conv2d(512, 512, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(512, 512, kernel_size=3, stride=1, padding=1, residual=True),), # 6, 6

            nn.Sequential(Conv2dTranspose(768, 384, kernel_size=3, stride=2, padding=1, output_padding=1),
            Conv2d(384, 384, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(384, 384, kernel_size=3, stride=1, padding=1, residual=True),), # 12, 12

            nn.Sequential(Conv2dTranspose(512, 256, kernel_size=3, stride=2, padding=1, output_padding=1),
            Conv2d(256, 256, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(256, 256, kernel_size=3, stride=1, padding=1, residual=True),), # 24, 24

            nn.Sequential(Conv2dTranspose(320, 128, kernel_size=3, stride=2, padding=1, output_padding=1), 
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(128, 128, kernel_size=3, stride=1, padding=1, residual=True),), # 48, 48

            nn.Sequential(Conv2dTranspose(160, 64, kernel_size=3, stride=2, padding=1, output_padding=1),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),
            Conv2d(64, 64, kernel_size=3, stride=1, padding=1, residual=True),),]) # 96,96

        self.output_block = nn.Sequential(Conv2d(80, 32, kernel_size=3, stride=1, padding=1),
            nn.Conv2d(32, 3, kernel_size=1, stride=1, padding=0),
            nn.Sigmoid()) 

    def forward(self, audio_sequences, face_sequences):
        # audio_sequences = (B, T, 1, 80, 16)
        B = audio_sequences.size(0)

        input_dim_size = len(face_sequences.size())
        if input_dim_size > 4:
            audio_sequences = torch.cat([audio_sequences[:, i] for i in range(audio_sequences.size(1))], dim=0)
            face_sequences = torch.cat([face_sequences[:, :, i] for i in range(face_sequences.size(2))], dim=0)

        audio_embedding = self.audio_encoder(audio_sequences) # B, 512, 1, 1

        feats = []
        x = face_sequences
        for f in self.face_encoder_blocks:
            x = f(x)
            feats.append(x)

        x = audio_embedding
        for f in self.face_decoder_blocks:
            x = f(x)
            try:
                x = torch.cat((x, feats[-1]), dim=1)
            except Exception as e:
                print(x.size())
                print(feats[-1].size())
                raise e
            
            feats.pop()

        x = self.output_block(x)

        if input_dim_size > 4:
            x = torch.split(x, B, dim=0) # [(B, C, H, W)]
            outputs = torch.stack(x, dim=2) # (B, C, T, H, W)

        else:
            outputs = x
            
        return outputs

class Wav2Lip_disc_qual(nn.Module):
    def __init__(self):
        super(Wav2Lip_disc_qual, self).__init__()

        self.face_encoder_blocks = nn.ModuleList([
            nn.Sequential(nonorm_Conv2d(3, 32, kernel_size=7, stride=1, padding=3)), # 48,96

            nn.Sequential(nonorm_Conv2d(32, 64, kernel_size=5, stride=(1, 2), padding=2), # 48,48
            nonorm_Conv2d(64, 64, kernel_size=5, stride=1, padding=2)),

            nn.Sequential(nonorm_Conv2d(64, 128, kernel_size=5, stride=2, padding=2),    # 24,24
            nonorm_Conv2d(128, 128, kernel_size=5, stride=1, padding=2)),

            nn.Sequential(nonorm_Conv2d(128, 256, kernel_size=5, stride=2, padding=2),   # 12,12
            nonorm_Conv2d(256, 256, kernel_size=5, stride=1, padding=2)),

            nn.Sequential(nonorm_Conv2d(256, 512, kernel_size=3, stride=2, padding=1),       # 6,6
            nonorm_Conv2d(512, 512, kernel_size=3, stride=1, padding=1)),

            nn.Sequential(nonorm_Conv2d(512, 512, kernel_size=3, stride=2, padding=1),     # 3,3
            nonorm_Conv2d(512, 512, kernel_size=3, stride=1, padding=1),),
            
            nn.Sequential(nonorm_Conv2d(512, 512, kernel_size=3, stride=1, padding=0),     # 1, 1
            nonorm_Conv2d(512, 512, kernel_size=1, stride=1, padding=0)),])

        self.binary_pred = nn.Sequential(nn.Conv2d(512, 1, kernel_size=1, stride=1, padding=0), nn.Sigmoid())
        self.label_noise = .0

    def get_lower_half(self, face_sequences):
        return face_sequences[:, :, face_sequences.size(2)//2:]

    def to_2d(self, face_sequences):
        B = face_sequences.size(0)
        face_sequences = torch.cat([face_sequences[:, :, i] for i in range(face_sequences.size(2))], dim=0)
        return face_sequences

    def perceptual_forward(self, false_face_sequences):
        false_face_sequences = self.to_2d(false_face_sequences)
        false_face_sequences = self.get_lower_half(false_face_sequences)

        false_feats = false_face_sequences
        for f in self.face_encoder_blocks:
            false_feats = f(false_feats)

        false_pred_loss = F.binary_cross_entropy(self.binary_pred(false_feats).view(len(false_feats), -1), 
                                        torch.ones((len(false_feats), 1)).cuda())

        return false_pred_loss

    def forward(self, face_sequences):
        face_sequences = self.to_2d(face_sequences)
        face_sequences = self.get_lower_half(face_sequences)

        x = face_sequences
        for f in self.face_encoder_blocks:
            x = f(x)

        return self.binary_pred(x).view(len(x), -1)


```

## Source file: app/routes/wav2lip_single_image.py

{
  "path": "app/routes/wav2lip_single_image.py",
  "blob_sha": "88afdfcb55968ce80399993a480ec262e592afd3",
  "sha256": "0983c737d7748e9531de56bcf36766da51f44c3e3785c9eab275f23fce0cf058",
  "lines": 19,
  "bytes": 835,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 3
  },
  "url": "https://github.com/host2india/Lip2Sync-3D-Talker/blob/00223506993a4355ad3b677e5f81760fcac7669c/app/routes/wav2lip_single_image.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "async def sync_single_image(image: UploadFile = File(...), audio: UploadFile = File(...)):"
    }
  ]
}

```text

# app/routes/wav2lip_single_image.py
from fastapi import APIRouter, UploadFile, File
from fastapi.responses import FileResponse, JSONResponse
from app.engines.wav2lip_single_image.engine import Wav2LipSingleImageEngine

router = APIRouter()
engine = Wav2LipSingleImageEngine(model_path="Wav2Lip", workspace=".")

@router.post("/sync/single_image")
async def sync_single_image(image: UploadFile = File(...), audio: UploadFile = File(...)):
    # Read bytes
    image_bytes = await image.read()
    audio_bytes = await audio.read()

    result = engine.run(image_bytes, audio_bytes)

    if result.get("status") == "success":
        return FileResponse(result["output_path"], media_type="video/mp4", filename="single_image.mp4")
    return JSONResponse({"status": "error", "details": result.get("details", "unknown")}, status_code=500)


```

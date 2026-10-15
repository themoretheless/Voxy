# Facial rendering source scan: SoheeLeeanim/lprr_project

Repository: https://github.com/SoheeLeeanim/lprr_project

Pinned source revision: b7f26b090d53754e3da4756c07b89ffd00ef8487

Discovery topic: facial retargeting

Description: Implementation of "Deep-Learning-Based Facial Retargeting Using Local Patches"

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["checkpoints/rm_eye_l/rm_epoch_001.pt", "checkpoints/rm_eye_l/rm_epoch_002.pt", "checkpoints/rm_eye_l/rm_epoch_003.pt", "checkpoints/rm_eye_l/rm_epoch_004.pt", "checkpoints/rm_eye_l/rm_epoch_005.pt", "checkpoints/rm_eye_l/rm_epoch_006.pt", "checkpoints/rm_eye_l/rm_epoch_007.pt", "checkpoints/rm_eye_l/rm_epoch_008.pt", "checkpoints/rm_eye_l/rm_epoch_009.pt", "checkpoints/rm_eye_l/rm_epoch_010.pt", "checkpoints/rm_eye_r/rm_epoch_001.pt", "checkpoints/rm_eye_r/rm_epoch_002.pt", "checkpoints/rm_eye_r/rm_epoch_003.pt", "checkpoints/rm_eye_r/rm_epoch_004.pt", "checkpoints/rm_eye_r/rm_epoch_005.pt", "checkpoints/rm_eye_r/rm_epoch_006.pt", "checkpoints/rm_eye_r/rm_epoch_007.pt", "checkpoints/rm_eye_r/rm_epoch_008.pt", "checkpoints/rm_eye_r/rm_epoch_009.pt", "checkpoints/rm_eye_r/rm_epoch_010.pt", "code/rm/train_eye_l.py", "code/rm/train_eye_r.py", "data/source/patch_eye_l/frame_000000.png", "data/source/patch_eye_l/frame_000001.png", "data/source/patch_eye_l/frame_000002.png", "data/source/patch_eye_l/frame_000003.png", "data/source/patch_eye_l/frame_000004.png", "data/source/patch_eye_l/frame_000005.png", "data/source/patch_eye_l/frame_000006.png", "data/source/patch_eye_l/frame_000007.png"], "face": ["landmark_images/facemesh_landmarks_index.jpg"], "lip": ["checkpoints/rm_lip/rm_epoch_001.pt", "checkpoints/rm_lip/rm_epoch_002.pt", "checkpoints/rm_lip/rm_epoch_003.pt", "checkpoints/rm_lip/rm_epoch_004.pt", "checkpoints/rm_lip/rm_epoch_005.pt", "checkpoints/rm_lip/rm_epoch_006.pt", "checkpoints/rm_lip/rm_epoch_007.pt", "checkpoints/rm_lip/rm_epoch_008.pt", "checkpoints/rm_lip/rm_epoch_009.pt", "checkpoints/rm_lip/rm_epoch_010.pt", "code/rm/train_lip.py", "data/source/patch_lip/frame_000000.png", "data/source/patch_lip/frame_000001.png", "data/source/patch_lip/frame_000002.png", "data/source/patch_lip/frame_000003.png", "data/source/patch_lip/frame_000004.png", "data/source/patch_lip/frame_000005.png", "data/source/patch_lip/frame_000006.png", "data/source/patch_lip/frame_000007.png", "data/source/patch_lip/frame_000008.png", "data/source/patch_lip/frame_000009.png", "data/source/patch_lip/frame_000010.png", "data/source/patch_lip/frame_000011.png", "data/source/patch_lip/frame_000012.png", "data/source/patch_lip/frame_000013.png", "data/source/patch_lip/frame_000014.png", "data/source/patch_lip/frame_000015.png", "data/source/patch_lip/frame_000016.png", "data/source/patch_lip/frame_000017.png", "data/source/patch_lip/frame_000018.png"]}

## Source file: code/rm/train_lip.py

{
  "path": "code/rm/train_lip.py",
  "blob_sha": "84bb8e258105e7b228b8e6a0dd1ae7023b385655",
  "sha256": "1de5ec723be3e92e1938a16763e41cac818b0f2dbca0e54f6a34dcc14d9dcb9f",
  "lines": 180,
  "bytes": 5935,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 17
  },
  "url": "https://github.com/SoheeLeeanim/lprr_project/blob/b7f26b090d53754e3da4756c07b89ffd00ef8487/code/rm/train_lip.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def _gaussian(window_size=11, sigma=1.5, device=\"cpu\"):"
    },
    {
      "line": 29,
      "text": "def _create_window(window_size=11, channel=3, device=\"cpu\"):"
    },
    {
      "line": 36,
      "text": "def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):"
    },
    {
      "line": 65,
      "text": "def main():"
    }
  ]
}

```text

import os
import sys

# Add project root to sys.path to allow running as script
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
from tqdm import tqdm
import torchvision.utils as vutils
from code.rm.datasets.patch_dataset import LipPatchDataset
from code.rm.models.autoencoder import RM_AutoEncoder
import random


# -----------------------
# SSIM (simple implementation)
# -----------------------
def _gaussian(window_size=11, sigma=1.5, device="cpu"):
    coords = torch.arange(window_size, device=device).float() - window_size // 2
    g = torch.exp(-(coords ** 2) / (2 * sigma ** 2))
    g = g / g.sum()
    return g

def _create_window(window_size=11, channel=3, device="cpu"):
    g = _gaussian(window_size, 1.5, device=device).unsqueeze(1)
    window_2d = g @ g.t()
    window_2d = window_2d.unsqueeze(0).unsqueeze(0)  # (1,1,ws,ws)
    window = window_2d.repeat(channel, 1, 1, 1)      # (C,1,ws,ws)
    return window

def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):
    """
    Calculates the Structural Similarity Index (SSIM) between two images.
    :param img1, img2: Tensors of shape (B, C, H, W) with values in [0, 1].
    :return: Mean SSIM value over the batch.
    """
    assert img1.shape == img2.shape
    B, C, H, W = img1.shape
    device = img1.device
    window = _create_window(window_size, C, device=device)

    mu1 = torch.nn.functional.conv2d(img1, window, padding=window_size//2, groups=C)
    mu2 = torch.nn.functional.conv2d(img2, window, padding=window_size//2, groups=C)

    mu1_sq = mu1.pow(2)
    mu2_sq = mu2.pow(2)
    mu1_mu2 = mu1 * mu2

    sigma1_sq = torch.nn.functional.conv2d(img1 * img1, window, padding=window_size//2, groups=C) - mu1_sq
    sigma2_sq = torch.nn.functional.conv2d(img2 * img2, window, padding=window_size//2, groups=C) - mu2_sq
    sigma12   = torch.nn.functional.conv2d(img1 * img2, window, padding=window_size//2, groups=C) - mu1_mu2

    ssim_map = ((2 * mu1_mu2 + C1) * (2 * sigma12 + C2)) / ((mu1_sq + mu2_sq + C1) * (sigma1_sq + sigma2_sq + C2))
    return ssim_map.mean()


# -----------------------
# Train
# -----------------------
def main():
    # ====== directory setting ======
    # e.g., data/source/patch_lip, data/target/patch_lip
    source_root = os.path.join("data", "source", "patch_lip") 
    target_root = os.path.join("data", "target", "patch_lip")
    source_base = os.path.join("data", "source")
    target_base = os.path.join("data", "target")

    frame_offset_target = -10

    # ====== Hyperparameters ======
    batch_size = 16
    num_workers = 0 # set to 0 for Windows compatibility
    lr = 1e-4
    epochs = 10
 
    # loss weights
    w_l1 = 1.0
    w_ssim = 1.0

    device = "cuda"
    print("[Device]", device)

    # ====== Dataset / Loader ======
    ds = LipPatchDataset(source_root, target_root, frame_offset_target=frame_offset_target)
    dl = DataLoader(ds, batch_size=batch_size, shuffle=True, num_workers=num_workers, drop_last=True)
    print("[Dataset length]", len(ds))

    # ====== Model ======
    model = RM_AutoEncoder(in_ch=3, base_ch=64, latent_ch=256).to(device)

    # ====== Optimizer ======
    opt = optim.Adam(model.parameters(), lr=lr)

    # L1 Loss
    l1 = nn.L1Loss()

    #  Create directories for saving checkpoints and samples
    ckpt_dir = os.path.join("checkpoints", "rm_lip")
    os.makedirs(ckpt_dir, exist_ok=True)
    sample_dir = os.path.join("samples", "rm_lip")
    os.makedirs(sample_dir, exist_ok=True)


    # ====== Train Loop ======
    model.train()
    global_step = 0

    for epoch in range(1, epochs + 1):
        running = 0.0

        pbar = tqdm(dl, desc=f"Epoch {epoch}/{epochs}", total=len(dl))
        for (src, tgt) in pbar: 
            src = src.to(device)  # Shape: (B, 3, 128, 128)
            tgt = tgt.to(device)

            src_rec, tgt_pred = model(src)
            loss_src = w_l1 * l1(src_rec, src) + w_ssim * (1.0 - ssim(src_rec, src))
            loss_tgt = w_l1 * l1(tgt_pred, tgt) + w_ssim * (1.0 - ssim(tgt_pred, tgt))

            loss = loss_src + loss_tgt

            opt.zero_grad()
            loss.backward()
            opt.step()


            running += loss.item()
            pbar.set_postfix(loss=f"{loss.item():.4f}", avg=f"{(running / (pbar.n + 1)):.4f}")

            global_step += 1


        avg = running / max(1, len(dl))
        print(f"[Epoch {epoch}/{epochs}] loss={avg:.6f}")

        # ---- Save sample images ----
        dl_sample = DataLoader(ds, batch_size=batch_size, shuffle=True, num_workers=0, drop_last=False)
        # ---- Save sample images (once per epoch): RANDOM ----
        model.eval()
        with torch.no_grad():
            rand_indices = random.sample(range(len(ds)), k=4)

            src_lips, tgt_lips = [], []

            for i in rand_indices:
                src_lip, tgt_lip = ds[i]
                src_lips.append(src_lip); tgt_lips.append(tgt_lip)

            src_lip = torch.stack(src_lips).to(device)
            tgt_lip = torch.stack(tgt_lips).to(device)
            src_rec_lip, tgt_pred_lip = model(src_lip)

            n = src_lip.size(0)
            out_lip = os.path.join(sample_dir, f"epoch_{epoch:03d}_lip_rand.png")
            vutils.save_image(torch.cat([src_lip, src_rec_lip, tgt_lip, tgt_pred_lip], dim=0), out_lip, nrow=n)

            print("  saved sample:", out_lip)

        model.train()

        
        # ---- Save checkpoint ----
        ckpt_path = os.path.join(ckpt_dir, f"rm_epoch_{epoch:03d}.pt")
        torch.save({
            "epoch": epoch,
            "model": model.state_dict(),
            "opt": opt.state_dict(),
        }, ckpt_path)
        print("  saved:", ckpt_path)

    print("Done.")


if __name__ == "__main__":
    main()


```

## Source file: code/rm/train_eye_l.py

{
  "path": "code/rm/train_eye_l.py",
  "blob_sha": "2ca09721e121d706a5fff4f23bddebc239011d00",
  "sha256": "53f0ab3069a76f8c4c6c579fd2cc02a283637778e542eaab41cb2c73b6344a3d",
  "lines": 162,
  "bytes": 5466,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 6,
    "lip": 3
  },
  "url": "https://github.com/SoheeLeeanim/lprr_project/blob/b7f26b090d53754e3da4756c07b89ffd00ef8487/code/rm/train_eye_l.py",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "def _gaussian(window_size=11, sigma=1.5, device=\"cpu\"):"
    },
    {
      "line": 28,
      "text": "def _create_window(window_size=11, channel=3, device=\"cpu\"):"
    },
    {
      "line": 35,
      "text": "def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):"
    },
    {
      "line": 59,
      "text": "def main():"
    }
  ]
}

```text

import os
import sys

# Add project root to sys.path to allow running as script
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
from tqdm import tqdm
import torchvision.utils as vutils
from code.rm.datasets.patch_dataset import LipPatchDataset
from code.rm.models.autoencoder import RM_AutoEncoder
import random

# -----------------------
# SSIM (simple implementation)
# -----------------------
def _gaussian(window_size=11, sigma=1.5, device="cpu"):
    coords = torch.arange(window_size, device=device).float() - window_size // 2
    g = torch.exp(-(coords ** 2) / (2 * sigma ** 2))
    g = g / g.sum()
    return g

def _create_window(window_size=11, channel=3, device="cpu"):
    g = _gaussian(window_size, 1.5, device=device).unsqueeze(1)
    window_2d = g @ g.t()
    window_2d = window_2d.unsqueeze(0).unsqueeze(0)  # (1,1,ws,ws)
    window = window_2d.repeat(channel, 1, 1, 1)      # (C,1,ws,ws)
    return window

def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):
    assert img1.shape == img2.shape
    B, C, H, W = img1.shape
    device = img1.device
    window = _create_window(window_size, C, device=device)

    mu1 = torch.nn.functional.conv2d(img1, window, padding=window_size//2, groups=C)
    mu2 = torch.nn.functional.conv2d(img2, window, padding=window_size//2, groups=C)

    mu1_sq = mu1.pow(2)
    mu2_sq = mu2.pow(2)
    mu1_mu2 = mu1 * mu2

    sigma1_sq = torch.nn.functional.conv2d(img1 * img1, window, padding=window_size//2, groups=C) - mu1_sq
    sigma2_sq = torch.nn.functional.conv2d(img2 * img2, window, padding=window_size//2, groups=C) - mu2_sq
    sigma12   = torch.nn.functional.conv2d(img1 * img2, window, padding=window_size//2, groups=C) - mu1_mu2

    ssim_map = ((2 * mu1_mu2 + C1) * (2 * sigma12 + C2)) / ((mu1_sq + mu2_sq + C1) * (sigma1_sq + sigma2_sq + C2))
    return ssim_map.mean()


# -----------------------
# Train
# -----------------------
def main():
    # ====== directory setting ======
    source_root = os.path.join("data", "source", "patch_eye_l") 
    target_root = os.path.join("data", "target", "patch_eye_l")

    frame_offset_target = -10

    # ====== Hyperparameters ======
    batch_size = 16
    num_workers = 0 # set to 0 for Windows compatibility
    lr = 1e-4
    epochs = 10
 
    # loss weights
    w_l1 = 1.0
    w_ssim = 1.0

    device = "cuda"
    print("[Device]", device)

    # ====== Dataset / Loader ======
    # LipPatchDataset is generic enough to handle eye patches given the correct roots
    ds = LipPatchDataset(source_root, target_root, frame_offset_target=frame_offset_target)
    dl = DataLoader(ds, batch_size=batch_size, shuffle=True, num_workers=num_workers, drop_last=True)
    print("[Dataset length]", len(ds))

    # ====== Model ======
    model = RM_AutoEncoder(in_ch=3, base_ch=64, latent_ch=256).to(device)

    # ====== Optimizer ======
    opt = optim.Adam(model.parameters(), lr=lr)

    # L1 Loss
    l1 = nn.L1Loss()

    #  Create directories for saving checkpoints and samples
    ckpt_dir = os.path.join("checkpoints", "rm_eye_l")
    os.makedirs(ckpt_dir, exist_ok=True)
    sample_dir = os.path.join("samples", "rm_eye_l")
    os.makedirs(sample_dir, exist_ok=True)


    # ====== Train Loop ======
    model.train()
    global_step = 0

    for epoch in range(1, epochs + 1):
        running = 0.0

        pbar = tqdm(dl, desc=f"Epoch {epoch}/{epochs}", total=len(dl))
        for (src, tgt) in pbar: 
            src = src.to(device)
            tgt = tgt.to(device)

            src_rec, tgt_pred = model(src)
            loss_src = w_l1 * l1(src_rec, src) + w_ssim * (1.0 - ssim(src_rec, src))
            loss_tgt = w_l1 * l1(tgt_pred, tgt) + w_ssim * (1.0 - ssim(tgt_pred, tgt))

            loss = loss_src + loss_tgt

            opt.zero_grad()
            loss.backward()
            opt.step()

            running += loss.item()
            pbar.set_postfix(loss=f"{loss.item():.4f}", avg=f"{(running / (pbar.n + 1)):.4f}")
            global_step += 1

        avg = running / max(1, len(dl))
        print(f"[Epoch {epoch}/{epochs}] loss={avg:.6f}")

        # ---- Save sample images (once per epoch): RANDOM ----
        model.eval()
        with torch.no_grad():
            rand_indices = random.sample(range(len(ds)), k=4)
            src_list, tgt_list = [], []
            for i in rand_indices:
                s, t = ds[i]
                src_list.append(s); tgt_list.append(t)

            src_batch = torch.stack(src_list).to(device)
            tgt_batch = torch.stack(tgt_list).to(device)
            src_rec, tgt_pred = model(src_batch)

            n = src_batch.size(0)
            out_sample = os.path.join(sample_dir, f"epoch_{epoch:03d}_eye_l_rand.png")
            vutils.save_image(torch.cat([src_batch, src_rec, tgt_batch, tgt_pred], dim=0), out_sample, nrow=n)
            print("  saved sample:", out_sample)

        model.train()
        
        # ---- Save checkpoint ----
        ckpt_path = os.path.join(ckpt_dir, f"rm_epoch_{epoch:03d}.pt")
        torch.save({
            "epoch": epoch,
            "model": model.state_dict(),
            "opt": opt.state_dict(),
        }, ckpt_path)
        print("  saved:", ckpt_path)

    print("Done.")

if __name__ == "__main__":
    main()

```

## Source file: code/rm/train_eye_r.py

{
  "path": "code/rm/train_eye_r.py",
  "blob_sha": "ba6fcaac30cfeeb73daac8479d80ad4414a58341",
  "sha256": "32b99325c003d0e6c46f808c90f05aa0e034ed38b2e4c31f452fd9d9453f4e3d",
  "lines": 163,
  "bytes": 5486,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 7,
    "lip": 3
  },
  "url": "https://github.com/SoheeLeeanim/lprr_project/blob/b7f26b090d53754e3da4756c07b89ffd00ef8487/code/rm/train_eye_r.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def _gaussian(window_size=11, sigma=1.5, device=\"cpu\"):"
    },
    {
      "line": 29,
      "text": "def _create_window(window_size=11, channel=3, device=\"cpu\"):"
    },
    {
      "line": 36,
      "text": "def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):"
    },
    {
      "line": 60,
      "text": "def main():"
    }
  ]
}

```text

# rm_train_eye_r.py
import os
import sys

# Add project root to sys.path to allow running as script
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
from tqdm import tqdm
import torchvision.utils as vutils
from code.rm.datasets.patch_dataset import LipPatchDataset
from code.rm.models.autoencoder import RM_AutoEncoder
import random

# -----------------------
# SSIM (simple implementation)
# -----------------------
def _gaussian(window_size=11, sigma=1.5, device="cpu"):
    coords = torch.arange(window_size, device=device).float() - window_size // 2
    g = torch.exp(-(coords ** 2) / (2 * sigma ** 2))
    g = g / g.sum()
    return g

def _create_window(window_size=11, channel=3, device="cpu"):
    g = _gaussian(window_size, 1.5, device=device).unsqueeze(1)
    window_2d = g @ g.t()
    window_2d = window_2d.unsqueeze(0).unsqueeze(0)  # (1,1,ws,ws)
    window = window_2d.repeat(channel, 1, 1, 1)      # (C,1,ws,ws)
    return window

def ssim(img1, img2, window_size=11, C1=0.01**2, C2=0.03**2):
    assert img1.shape == img2.shape
    B, C, H, W = img1.shape
    device = img1.device
    window = _create_window(window_size, C, device=device)

    mu1 = torch.nn.functional.conv2d(img1, window, padding=window_size//2, groups=C)
    mu2 = torch.nn.functional.conv2d(img2, window, padding=window_size//2, groups=C)

    mu1_sq = mu1.pow(2)
    mu2_sq = mu2.pow(2)
    mu1_mu2 = mu1 * mu2

    sigma1_sq = torch.nn.functional.conv2d(img1 * img1, window, padding=window_size//2, groups=C) - mu1_sq
    sigma2_sq = torch.nn.functional.conv2d(img2 * img2, window, padding=window_size//2, groups=C) - mu2_sq
    sigma12   = torch.nn.functional.conv2d(img1 * img2, window, padding=window_size//2, groups=C) - mu1_mu2

    ssim_map = ((2 * mu1_mu2 + C1) * (2 * sigma12 + C2)) / ((mu1_sq + mu2_sq + C1) * (sigma1_sq + sigma2_sq + C2))
    return ssim_map.mean()


# -----------------------
# Train
# -----------------------
def main():
    # ====== directory setting ======
    source_root = os.path.join("data", "source", "patch_eye_r") 
    target_root = os.path.join("data", "target", "patch_eye_r")

    frame_offset_target = -10

    # ====== Hyperparameters ======
    batch_size = 16
    num_workers = 0 # set to 0 for Windows compatibility
    lr = 1e-4
    epochs = 10
 
    # loss weights
    w_l1 = 1.0
    w_ssim = 1.0

    device = "cuda"
    print("[Device]", device)

    # ====== Dataset / Loader ======
    # LipPatchDataset is generic enough to handle eye patches given the correct roots
    ds = LipPatchDataset(source_root, target_root, frame_offset_target=frame_offset_target)
    dl = DataLoader(ds, batch_size=batch_size, shuffle=True, num_workers=num_workers, drop_last=True)
    print("[Dataset length]", len(ds))

    # ====== Model ======
    model = RM_AutoEncoder(in_ch=3, base_ch=64, latent_ch=256).to(device)

    # ====== Optimizer ======
    opt = optim.Adam(model.parameters(), lr=lr)

    # L1 Loss
    l1 = nn.L1Loss()

    #  Create directories for saving checkpoints and samples
    ckpt_dir = os.path.join("checkpoints", "rm_eye_r")
    os.makedirs(ckpt_dir, exist_ok=True)
    sample_dir = os.path.join("samples", "rm_eye_r")
    os.makedirs(sample_dir, exist_ok=True)


    # ====== Train Loop ======
    model.train()
    global_step = 0

    for epoch in range(1, epochs + 1):
        running = 0.0

        pbar = tqdm(dl, desc=f"Epoch {epoch}/{epochs}", total=len(dl))
        for (src, tgt) in pbar: 
            src = src.to(device)
            tgt = tgt.to(device)

            src_rec, tgt_pred = model(src)
            loss_src = w_l1 * l1(src_rec, src) + w_ssim * (1.0 - ssim(src_rec, src))
            loss_tgt = w_l1 * l1(tgt_pred, tgt) + w_ssim * (1.0 - ssim(tgt_pred, tgt))

            loss = loss_src + loss_tgt

            opt.zero_grad()
            loss.backward()
            opt.step()

            running += loss.item()
            pbar.set_postfix(loss=f"{loss.item():.4f}", avg=f"{(running / (pbar.n + 1)):.4f}")
            global_step += 1

        avg = running / max(1, len(dl))
        print(f"[Epoch {epoch}/{epochs}] loss={avg:.6f}")

        # ---- Save sample images (once per epoch): RANDOM ----
        model.eval()
        with torch.no_grad():
            rand_indices = random.sample(range(len(ds)), k=4)
            src_list, tgt_list = [], []
            for i in rand_indices:
                s, t = ds[i]
                src_list.append(s); tgt_list.append(t)

            src_batch = torch.stack(src_list).to(device)
            tgt_batch = torch.stack(tgt_list).to(device)
            src_rec, tgt_pred = model(src_batch)

            n = src_batch.size(0)
            out_sample = os.path.join(sample_dir, f"epoch_{epoch:03d}_eye_r_rand.png")
            vutils.save_image(torch.cat([src_batch, src_rec, tgt_batch, tgt_pred], dim=0), out_sample, nrow=n)
            print("  saved sample:", out_sample)

        model.train()
        
        # ---- Save checkpoint ----
        ckpt_path = os.path.join(ckpt_dir, f"rm_epoch_{epoch:03d}.pt")
        torch.save({
            "epoch": epoch,
            "model": model.state_dict(),
            "opt": opt.state_dict(),
        }, ckpt_path)
        print("  saved:", ckpt_path)

    print("Done.")

if __name__ == "__main__":
    main()

```

## Source file: reorganize.py

{
  "path": "reorganize.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/SoheeLeeanim/lprr_project/blob/b7f26b090d53754e3da4756c07b89ffd00ef8487/reorganize.py",
  "function_signature_candidates": []
}

```text



```

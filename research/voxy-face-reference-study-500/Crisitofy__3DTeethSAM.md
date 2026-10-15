# Facial rendering source scan: Crisitofy/3DTeethSAM

Repository: https://github.com/Crisitofy/3DTeethSAM

Pinned source revision: 4845f4132cbbeca2ebc456d33de7e0c358385615

Discovery topic: teeth 3D

Description: Official PyTorch implementation of 3DTeethSAM: Taming SAM2 for 3D Teeth Segmentation (AAAI 2026, oral).

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"jaw": ["model/dataset/hdf5dataset_aug_jaw.py"]}

## Source file: model/dataset/hdf5dataset_aug_jaw.py

{
  "path": "model/dataset/hdf5dataset_aug_jaw.py",
  "blob_sha": "115a7791ccd7bb50c22172f14df186ebd8d86397",
  "sha256": "b0382b85ce380129bf51a8435b7090b1fff3468b1be8f9e25ef8ae74eb733d87",
  "lines": 226,
  "bytes": 8676,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 7,
    "teeth": 3,
    "tooth": 3,
    "rig": 8
  },
  "url": "https://github.com/Crisitofy/3DTeethSAM/blob/4845f4132cbbeca2ebc456d33de7e0c358385615/model/dataset/hdf5dataset_aug_jaw.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self, config=None):"
    },
    {
      "line": 29,
      "text": "def __call__(self, image, gt_masks):"
    },
    {
      "line": 69,
      "text": "def _rotate(self, image, masks, angle):"
    },
    {
      "line": 116,
      "text": "def __init__(self, hdf5_file, transform=False, mode=None, augment_config=None, jaw_type=None, split_file_path=None, view_indices=None):"
    },
    {
      "line": 188,
      "text": "def __len__(self):"
    },
    {
      "line": 191,
      "text": "def __getitem__(self, idx):"
    },
    {
      "line": 224,
      "text": "def __del__(self):"
    }
  ]
}

```text

import h5py
import torch
import torch.nn.functional as F
from torch.utils.data import Dataset
import json
import numpy as np
import random

class TeethAugmentation:
    def __init__(self, config=None):
        if config is None:
            config = {}
        
        # Brightness and contrast adjustment
        self.brightness_prob = config.get('brightness_prob', 0.5)
        self.brightness_range = config.get('brightness_range', 0.15)
        
        self.contrast_prob = config.get('contrast_prob', 0.5)
        self.contrast_range = config.get('contrast_range', 0.15)
        
        # Add noise
        self.noise_prob = config.get('noise_prob', 0.3)
        self.noise_std = config.get('noise_std', 0.01)
        
        # Small rotation (optional)
        self.rotate_prob = config.get('rotate_prob', 0.0)  # Disabled by default
        self.rotate_range = config.get('rotate_range', 5)  # ±5 degrees
    
    def __call__(self, image, gt_masks):
        """
        Apply data augmentation.
        Args:
            image: [3, H, W] image tensor.
            gt_masks: [17, H, W] mask tensor (including background).
        Returns:
            augmented_image, augmented_masks
        """
        # Clone inputs to avoid modifying original data
        image = image.clone()
        gt_masks = gt_masks.clone()
        
        # 1. Brightness adjustment
        if random.random() < self.brightness_prob:
            brightness_factor = 1 + random.uniform(-self.brightness_range, self.brightness_range)
            image = image * brightness_factor
        
        # 2. Contrast adjustment
        if random.random() < self.contrast_prob:
            contrast_factor = 1 + random.uniform(-self.contrast_range, self.contrast_range)
            mean = image.mean(dim=[1, 2], keepdim=True)
            image = (image - mean) * contrast_factor + mean
        
        # 3. Add Gaussian noise
        if random.random() < self.noise_prob:
            noise = torch.randn_like(image) * self.noise_std
            image = image + noise
        
        # 4. Small rotation (applied to both image and masks)
        if self.rotate_prob > 0 and random.random() < self.rotate_prob:
            angle = random.uniform(-self.rotate_range, self.rotate_range)
            image, gt_masks = self._rotate(image, gt_masks, angle)
        
        # Ensure values are within a valid range
        image = torch.clamp(image, 0, 1)
        gt_masks = torch.clamp(gt_masks, 0, 1)
        
        return image, gt_masks
    
    def _rotate(self, image, masks, angle):
        """Rotation implementation using PyTorch."""
        import torch.nn.functional as F
        
        # Convert angle to radians
        theta = angle * np.pi / 180
        
        # Create rotation matrix
        cos_val = np.cos(theta)
        sin_val = np.sin(theta)
        
        # 2x3 affine transformation matrix
        rotation_matrix = torch.tensor([
            [cos_val, -sin_val, 0],
            [sin_val, cos_val, 0]
        ], dtype=torch.float32)
        
        # Add batch dimension
        image_batch = image.unsqueeze(0)
        masks_batch = masks.unsqueeze(0)
        
        # Create grid
        grid = F.affine_grid(
            rotation_matrix.unsqueeze(0),
            image_batch.size(),
            align_corners=False
        )
        
        # Apply rotation
        rotated_image = F.grid_sample(
            image_batch, grid,
            mode='bilinear',
            padding_mode='reflection',
            align_corners=False
        )
        
        rotated_masks = F.grid_sample(
            masks_batch, grid,
            mode='nearest',
            padding_mode='zeros',
            align_corners=False
        )
        
        return rotated_image.squeeze(0), rotated_masks.squeeze(0)

class HDF5TeethDataset(Dataset):
    """HDF5 dataset for tooth segmentation."""
    def __init__(self, hdf5_file, transform=False, mode=None, augment_config=None, jaw_type=None, split_file_path=None, view_indices=None):
        """
        Args:
            hdf5_file: Path to the HDF5 file.
            transform: Whether to apply data augmentation.
            mode: 'train', 'val', 'test', 'test_all'.
            augment_config: Dictionary of augmentation configurations.
            split_file_path: Path to the dataset split file.
        """
        self.transform = transform
        self.file_path = hdf5_file
        self.mode = mode
        self.jaw_type = jaw_type

        # View filtering list, None means keep all views (for ablation)
        if view_indices is not None:
            # Supports comma-separated string or iterable
            if isinstance(view_indices, str):
                view_indices = [int(v) for v in view_indices.split(',') if v.strip()]
            self.view_indices = set(view_indices)
        else:
            self.view_indices = None

        # Open HDF5 file (read-only mode)
        self.h5_file = h5py.File(hdf5_file, 'r')

        # Load all sample indices
        all_samples = json.loads(self.h5_file['sample_index'][()])
        
        # Filter samples based on the dataset split file
        if split_file_path:
            with open(split_file_path, 'r') as f:
                split_case_names = {line.strip() for line in f if line.strip()}
            
            self.samples = []
            for sample in all_samples:
                # Assuming case_name format is P001_upper or P001_lower
                case_name = sample['case_name'].split('_')[0]+'_'+sample['case_name'].split('_')[1]
                if case_name in split_case_names:
                    self.samples.append(sample)
            print(f"Loaded {len(self.samples)} samples according to {split_file_path}")
        else:
            self.samples = all_samples
            print(f"Warning: No split file provided, loaded all {len(self.samples)} samples")

        # Filter samples by jaw_type (if needed)
        if jaw_type is not None:
            jaw_samples = [s for s in self.samples if f"_{jaw_type}_" in s['case_name']]
            print(f"Filtered {len(jaw_samples)} {jaw_type} jaw samples from the split")
            self.samples = jaw_samples

        # Filter samples by view indices (if needed)
        if self.view_indices is not None:
            view_filtered_samples = [s for s in self.samples if s['view_idx'] in self.view_indices]
            print(f"Filtered to {len(view_filtered_samples)} samples based on view indices {sorted(self.view_indices)}")
            self.samples = view_filtered_samples
        
        # Check available fields in the dataset
        self.available_fields = list(self.h5_file.keys())
        required_fields = ['images', 'gt_masks']
            
        # Check if all required fields are available
        missing_fields = [field for field in required_fields if field not in self.available_fields]
        if missing_fields:
            raise ValueError(f"HDF5 file is missing required fields: {missing_fields}")
        
        # Apply augmentation only in training mode
        if mode == 'train' and transform:
            self.augmentation = TeethAugmentation(augment_config)
        else:
            self.augmentation = None
            
    def __len__(self):
        return len(self.samples)
    
    def __getitem__(self, idx):
        sample = self.samples[idx]
        sample_id = sample['id']
        
        # Get basic data
        image = np.array(self.h5_file['images'][sample_id])
        gt_masks = np.array(self.h5_file['gt_masks'][sample_id])
        
        # Get metadata
        valid_masks = np.array(self.h5_file['metadata'][f"{sample_id}_valid_masks"])
        tooth_ids = np.array(self.h5_file['metadata'][f"{sample_id}_tooth_ids"])
        
        # Convert to tensors
        image_tensor = torch.from_numpy(image).float().permute(2, 0, 1)  # [3, H, W]
        gt_masks_tensor = torch.from_numpy(gt_masks).float()  # [17, H, W]
        valid_masks_tensor = torch.from_numpy(valid_masks).long()  # [max_masks]
        
        # Apply data augmentation
        if self.augmentation is not None:
            image_tensor, gt_masks_tensor = self.augmentation(image_tensor, gt_masks_tensor)
        
        # Build the basic result dictionary
        result = {
            'case_name': sample['case_name'],
            'image': image_tensor,
            'gt_masks': gt_masks_tensor,
            'tooth_ids': tooth_ids,
            'view_idx': sample['view_idx'],
            'valid_masks': valid_masks_tensor
        }
            
        return result
    
    def __del__(self):
        if hasattr(self, 'h5_file'):
            self.h5_file.close()

```

## Source file: model/PEG.py

{
  "path": "model/PEG.py",
  "blob_sha": "f99ac445dad32d0be6e9cb4676c7f11900e84b51",
  "sha256": "f763c2f2b2df434b9ee13c9fedc73035e63bbfbc1fe3a916a88c1c4c0b3af92f",
  "lines": 333,
  "bytes": 12423,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 4,
    "rig": 10
  },
  "url": "https://github.com/Crisitofy/3DTeethSAM/blob/4845f4132cbbeca2ebc456d33de7e0c358385615/model/PEG.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, d_model, dropout=0.1):"
    },
    {
      "line": 70,
      "text": "def forward(self, low_res_feat, high_res_feats):"
    },
    {
      "line": 112,
      "text": "def __init__(self, d_model=256, num_queries=16, num_layers=5,"
    },
    {
      "line": 135,
      "text": "def forward(self, image_embed):"
    },
    {
      "line": 162,
      "text": "def __init__(self, config):"
    },
    {
      "line": 216,
      "text": "def forward(self, images):"
    },
    {
      "line": 244,
      "text": "def process_images(self, images):"
    },
    {
      "line": 279,
      "text": "def generate_masks(self, prompt_embed, image_embed, high_res_feats, orig_hw):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F
import math

from model.sam2.utils.transforms import SAM2Transforms
from model.sam2.build_sam import build_sam2 
from model.sam2.modeling.sam2_utils import LayerNorm2d

from model.Mask_Refiner import ResUNet

class MultiLevelFeatureFusion(nn.Module):
    """
    Multi-level feature fusion
    """
    def __init__(self, d_model, dropout=0.1):
        super().__init__()
        self.d_model = d_model
        
        self.s0_processor = nn.Sequential(
            nn.Conv2d(d_model // 8, d_model // 4, kernel_size=2, stride=2),
            LayerNorm2d(d_model // 4),
            nn.GELU(),
            nn.Conv2d(d_model // 4, d_model, kernel_size=2, stride=2),
            LayerNorm2d(d_model),
            nn.GELU()
        )
        self.s1_processor = nn.Sequential(
            nn.Conv2d(d_model // 4, d_model, kernel_size=2, stride=2),
            LayerNorm2d(d_model),
            nn.GELU()
        )
        self.low_res_processor = nn.Sequential(
            nn.Conv2d(d_model, d_model, kernel_size=1),
            LayerNorm2d(d_model),
            nn.GELU()
        )
        self.concat_fusion_conv = nn.Conv2d(d_model * 3, d_model, kernel_size=1)

        # channel attention
        self.channel_attention = nn.Sequential(
            nn.AdaptiveAvgPool2d(1),
            nn.Conv2d(d_model, d_model//16, 1),
            nn.GELU(),
            nn.Conv2d(d_model//16, d_model, 1),
            nn.Sigmoid()
        )
        
        # spatial attention
        self.spatial_attention = nn.Sequential(
            nn.Conv2d(2, 1, kernel_size=7, padding=3),
            nn.Sigmoid()
        )
        
        # feature processing and residual connection
        self.refine = nn.Sequential(
            nn.Conv2d(d_model, d_model, kernel_size=3, padding=1),
            LayerNorm2d(d_model),
            nn.GELU(),
            nn.Conv2d(d_model, d_model, kernel_size=3, padding=1),
            LayerNorm2d(d_model),
            nn.GELU()
        )
        
        # final projection to embedding space
        self.final_proj = nn.Linear(d_model, d_model)
        self.dropout = nn.Dropout(dropout)
        self.norm = nn.LayerNorm(d_model)
    
    def forward(self, low_res_feat, high_res_feats):
        """
        Args:
            low_res_feat: [B, 256, 64, 64] - low-resolution features
            high_res_feats: List[[B, 32, 256, 256], [B, 64, 128, 128]] - high-resolution features

        Returns:
            fused_feat: [B, 4096, 256] - fused features, ready for transformer input, [B, HW, C]
            fused_features: [B, C, H, W] - fused features, ready for refinement
        """
        feat_s0, feat_s1 = high_res_feats  # feat_s0 is highest resolution
        
        s0_processed = self.s0_processor(feat_s0)
        s1_processed = self.s1_processor(feat_s1)
        low_res_processed = self.low_res_processor(low_res_feat)
        concatenated = torch.cat([s0_processed, s1_processed, low_res_processed], dim=1)
        fused = self.concat_fusion_conv(concatenated)

        # CBAM
        channel_attn = self.channel_attention(fused)
        fused = fused * channel_attn
        avg_feat = torch.mean(fused, dim=1, keepdim=True)
        max_feat, _ = torch.max(fused, dim=1, keepdim=True)
        spatial_feat = torch.cat([avg_feat, max_feat], dim=1)
        spatial_attn = self.spatial_attention(spatial_feat)
        fused = fused * spatial_attn

        # refine fused features
        fused_features = self.refine(fused)  # [B, C, H, W]

        # reshape to sequence format, suitable for transformer processing
        seq_feat = fused_features.flatten(2).transpose(1, 2)  # [B, 4096, 256]

        # final projection and normalization
        out = self.final_proj(seq_feat)
        out = self.dropout(out)
        out = self.norm(out + seq_feat)  # residual connection

        return out, fused_features

class AutoPromptGenerator(nn.Module):
    """APGnet, based on Transformer decoder"""
    def __init__(self, d_model=256, num_queries=16, num_layers=5, 
                 dropout=0.15, nhead=8):
        super().__init__()
        self.d_model = d_model
        self.num_queries = num_queries
        
        # learnable query vectors, shape [num_queries, d_model]
        self.query_embed = nn.Parameter(torch.empty(num_queries, d_model))
        nn.init.kaiming_uniform_(self.query_embed, a=math.sqrt(5))
        
        # Transformer decoder
        decoder_layer = nn.TransformerDecoderLayer(
            d_model=d_model, 
            nhead=nhead, 
            batch_first=True,
            dropout=dropout
        )

        self.transformer_decoder = nn.TransformerDecoder(decoder_layer, num_layers=num_layers)

        # existence head, output embedding confidence
        self.label_head = nn.Linear(d_model, 1)

    def forward(self, image_embed):
        """
        Args:
            image_embed: [B, H*W, C] - fused image embeddings
        Returns:
            prompt_embed: [B, num_queries, d_model] - generated prompt embeddings
            labels: [B, num_queries] - confidence for each prompt embedding, range [0, 1]
        """
        B, _, _ = image_embed.shape
        device = image_embed.device
        
        # prepare query embedding
        query_embed = self.query_embed.to(device)
        query_embed = query_embed.unsqueeze(0).expand(B, -1, -1)

        # Transformer decoder
        prompt_embed = self.transformer_decoder(query_embed, image_embed)

        # predict labels
        labels = self.label_head(prompt_embed).squeeze(-1)  # [B, num_queries]
        
        return prompt_embed, labels

class DentalSegmentationSystem(nn.Module):
    """
    End-to-end dental segmentation system, using SAM2 and Transformer-based prompt generation network
    """
    def __init__(self, config):
        super().__init__()
        
        # model parameters
        self.d_model = config.get('embed_dim', 256)
        self.num_queries = config.get('num_teeth', 16) 

        # class head predicts 17 classes (16 teeth + 1 background)
        self.num_classes_for_head = 17
        self.num_layers = config.get('num_layers', 6) 
        self.dropout_rate = config.get('dropout_rate', 0.1)
        
        # multi-level feature fusion
        self.feature_fusion = MultiLevelFeatureFusion(
            d_model=self.d_model,
            dropout=self.dropout_rate
        )

        self.auto_prompt_generator = AutoPromptGenerator(
            d_model=self.d_model,
            num_queries=self.num_queries,
            num_layers=self.num_layers,
            dropout=self.dropout_rate, 
            nhead=config.get('nhead', 8),
        )

        # SAM model
        self.sam_model = build_sam2(
            config_file=config.get('sam_config', 'configs/sam2.1/sam2.1_hiera_l.yaml'),
            ckpt_path=config.get('sam_checkpoint', 'checkpoints/sam2.1_hiera_large.pt')
        )
        
        # SAM2 feature sizes
        self.bb_feat_sizes = [(256, 256), (128, 128), (64, 64)]
        
        # get transforms
        self._transforms = SAM2Transforms(
            resolution=self.sam_model.image_size,
            mask_threshold=0.0,
            max_hole_area=0.0,
            max_sprinkle_area=0.0,
        )
        
        # refine_net
        self.refine_net = ResUNet(
            num_classes=config.get('num_classes', 17),
            dropout_rate=self.dropout_rate
        )

        # freeze SAM
        if not config.get('finetune_sam', False):
            for param in self.sam_model.parameters():
                param.requires_grad = False
    
    def forward(self, images):
        """
        Args:
            images: input images [B, 3, H, W]
        Returns:
            sam_masks: SAM generated masks [B, num_classes, H, W]
            refined_masks: refined masks [B, num_classes, H, W]
            confidece: existence probabilities [B, num_queries]
        """
        # process tensor images to get image embeddings
        image_embed, high_res_feats, orig_hw = self.process_images(images)

        # multi-level feature fusion
        seq_features, fused_features = self.feature_fusion(image_embed, high_res_feats) # [B, HW, C], [B, C, H, W]

        # generate prompt embeddings and confidence
        prompt_embed, confidence = self.auto_prompt_generator(seq_features)  

        # generate masks
        sam_masks = self.generate_masks(prompt_embed, image_embed, high_res_feats, orig_hw)

        sam_probs = torch.sigmoid(sam_masks)  # [B, num_classes, H, W]

        # refined_masks, input is original images and coarse segmentation results (sam_probs)
        refined_masks = self.refine_net(images, sam_probs, fused_features)
        
        return sam_masks, refined_masks, confidence
        
    def process_images(self, images):
        """
        Args:
            images: image tensor [B, 3, H, W]
            
        Returns:
            image_embed: image embedding
            high_res_feats: high-resolution features
            orig_hw: original image size
        """
        # save original image size
        B, _, H, W = images.shape
        orig_hw = [(H, W) for _ in range(B)]
        
        img_batch= self._transforms.transforms(images)
        
        # get image embeddings
        backbone_out = self.sam_model.forward_image(img_batch)
        _, vision_feats, _, _ = self.sam_model._prepare_backbone_features(backbone_out)
        
        # add no_mem_embed
        if self.sam_model.directly_add_no_mem_embed:
            vision_feats[-1] = vision_feats[-1] + self.sam_model.no_mem_embed
        
        # prepare features
        feats = [
            feat.permute(1, 2, 0).view(B, -1, *feat_size)
            for feat, feat_size in zip(vision_feats[::-1], self.bb_feat_sizes[::-1])
        ][::-1]
        
        # return image embeddings and high-resolution features
        image_embed = feats[-1]
        high_res_feats = feats[:-1]
        return image_embed, high_res_feats, orig_hw

    def generate_masks(self, prompt_embed, image_embed, high_res_feats, orig_hw):
        """
        Args:
            prompt_embed: prompt embeddings [B, num_queries, d_model]
            image_embed: image embedding [B, H'*W', C]
            high_res_feats: high-resolution features list
            orig_hw: original image size
            
        Returns:
            sam_masks: SAM generated masks [B, num_classes, H, W]
        """
        B = prompt_embed.shape[0]

        image_embeddings = torch.repeat_interleave(image_embed, self.num_queries, dim=0) # (B*n_teeth, C, H', W')
        high_res_features = [torch.repeat_interleave(feats, self.num_queries, dim=0) for feats in high_res_feats]
        
        # flatten prompt_embed to match repeated image_embeddings
        sparse_embeddings = prompt_embed.reshape(-1, prompt_embed.shape[-1]).unsqueeze(1)  # [B*n_teeth, 1, embed_dim]
        
        # get dense_embeddings
        dense_embeddings = self.sam_model.sam_prompt_encoder.no_mask_embed.weight.reshape(1, -1, 1, 1).expand(
            B * self.num_queries, 
            -1, 
            self.sam_model.sam_prompt_encoder.image_embedding_size[0], 
            self.sam_model.sam_prompt_encoder.image_embedding_size[1]
        )

        low_res_masks, _, _, _ = self.sam_model.sam_mask_decoder(
            image_embeddings=image_embeddings,
            image_pe=self.sam_model.sam_prompt_encoder.get_dense_pe(),
            sparse_prompt_embeddings=sparse_embeddings,
            dense_prompt_embeddings=dense_embeddings,
            multimask_output=False,  
            repeat_image=False,  
            high_res_features=high_res_features,
        )

        masks = F.interpolate(low_res_masks, orig_hw[0], mode="bilinear", align_corners=False)
        masks = masks.view(B, self.num_queries, masks.shape[2], masks.shape[3])

        # Calculate background mask in probability space for numerical stability
        fg_probs = torch.sigmoid(masks)
        bg_prob = 1.0 - torch.clamp(fg_probs.sum(dim=1, keepdim=True), 0, 1)
        
        # Convert background probability back to logits
        # Add a small epsilon to prevent log(0)
        eps = 1e-6
        bg_masks = torch.log(bg_prob / (1 - bg_prob + eps) + eps)

        # merge background and reordered masks
        sam_masks = torch.cat([bg_masks, masks], dim=1)  # [B, 17, H, W]

        return sam_masks
        
  

```

## Source file: model/DGAP.py

{
  "path": "model/DGAP.py",
  "blob_sha": "af2ccd7b22611903b25fb18d52b78f984624c109",
  "sha256": "ac7b2d11fc81fe8077e2fb976fa2ca9cd0584f8acd18e24738cc2b0ad9b7bbd8",
  "lines": 139,
  "bytes": 4483,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 3
  },
  "url": "https://github.com/Crisitofy/3DTeethSAM/blob/4845f4132cbbeca2ebc456d33de7e0c358385615/model/DGAP.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__("
    },
    {
      "line": 31,
      "text": "def forward(self, x):"
    },
    {
      "line": 35,
      "text": "def __init__("
    },
    {
      "line": 111,
      "text": "def forward(self, x):"
    }
  ]
}

```text

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.cuda.amp import autocast

class ConvBNReLU(nn.Module):
    def __init__(
        self, 
        in_channels, 
        out_channels, 
        kernel_size=3,         
        stride=1, 
        groups=1,
        use_bn=True,
        use_relu=True
    ):
        super().__init__()
        padding = (kernel_size - 1) // 2
        self.conv = nn.Conv2d(
            in_channels, 
            out_channels,
            kernel_size=kernel_size, 
            stride=stride,
            padding=padding,
            groups=groups,
            bias=not use_bn
        )
        self.bn = nn.BatchNorm2d(out_channels) if use_bn else nn.Identity()
        self.relu = nn.ReLU(inplace=True) if use_relu else nn.Identity()

    def forward(self, x):
        return self.relu(self.bn(self.conv(x)))

class DSPWrapper(nn.Module):
    def __init__(
        self, 
        in_channels, 
        deformable_groups=1, 
        scale_factor=1.0,
        offset_channels=None,  
        conv_channels=None,   
        kernel_size=3,       
        use_residual=True,     
        use_bn=True,          
        use_relu=True   
    ):
        super().__init__()
        self.in_channels = int(in_channels)
        self.deformable_groups = int(deformable_groups)
        self.scale_factor = float(scale_factor)
        self.kernel_size = int(kernel_size)
        self.use_residual = bool(use_residual)
        self.use_bn = bool(use_bn)
        self.use_relu = bool(use_relu)
        
        if offset_channels is None:
            self.offset_channels = self.in_channels // 4
        else:
            self.offset_channels = int(offset_channels)
        
        if conv_channels is None:
            self.conv_channels = self.in_channels
        else:
            self.conv_channels = int(conv_channels)

        self.offset_net = nn.Sequential(
            ConvBNReLU(
                self.in_channels, 
                self.offset_channels, 
                kernel_size=1, 
                use_bn=self.use_bn, 
                use_relu=self.use_relu
            ),
            ConvBNReLU(
                self.offset_channels, 
                self.offset_channels, 
                kernel_size=self.kernel_size, 
                groups=self.offset_channels, 
                use_bn=self.use_bn, 
                use_relu=self.use_relu
            ),
            ConvBNReLU(
                self.offset_channels, 
                self.offset_channels // 2, 
                kernel_size=1, 
                use_bn=self.use_bn, 
                use_relu=self.use_relu
            ),
            nn.Conv2d(self.offset_channels // 2, 2, kernel_size=1)
        )
        self.conv = nn.Sequential(
            ConvBNReLU(
                self.in_channels, 
                self.conv_channels, 
                kernel_size=self.kernel_size, 
                groups=self.deformable_groups, 
                use_bn=self.use_bn, 
                use_relu=self.use_relu
            ),
            ConvBNReLU(
                self.conv_channels, 
                self.in_channels, 
                kernel_size=1, 
                use_bn=self.use_bn, 
                use_relu=self.use_relu
            )
        )
        for p in self.offset_net[-1].parameters():
            nn.init.zeros_(p)
            
    def forward(self, x):
        B, H, W, C = x.shape

        x_in = x.permute(0, 3, 1, 2).contiguous()
        identity = x_in if self.use_residual else None
        offset = self.offset_net(x_in)
        grid_h, grid_w = torch.meshgrid(
            torch.linspace(-1, 1, H, device=x.device),
            torch.linspace(-1, 1, W, device=x.device),
            indexing='ij'
        )

        grid = torch.stack([grid_w, grid_h], dim=-1)
        grid = grid.unsqueeze(0).expand(B, -1, -1, -1)
        offset_permuted = offset.permute(0, 2, 3, 1).contiguous()
        offset_scaled = offset_permuted * self.scale_factor / torch.tensor([W/2., H/2.], device=offset.device)
        grid_with_offset = grid + offset_scaled
        x_sampled = F.grid_sample(
            x_in, grid_with_offset, mode='bilinear', padding_mode='zeros', align_corners=True
        )

        x_conv = self.conv(x_sampled)
        if self.use_residual and identity is not None:
            x_out_feat = x_conv + identity
        else:
            x_out_feat = x_conv
        x_out = x_out_feat.permute(0, 2, 3, 1).contiguous()

        return x_out

```

## Source file: lift/lift3d.py

{
  "path": "lift/lift3d.py",
  "blob_sha": "f1d6fdd30ecb182a889d9620668a73afa4dafa94",
  "sha256": "da247554c231c17bacdb7e5aab42ea4090462d83f1840fb983132c4732d78217",
  "lines": 288,
  "bytes": 12513,
  "selection_score": 0,
  "keyword_line_counts": {
    "teeth": 8,
    "tooth": 21,
    "face": 35
  },
  "url": "https://github.com/Crisitofy/3DTeethSAM/blob/4845f4132cbbeca2ebc456d33de7e0c358385615/lift/lift3d.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def __init__(self, device='cuda', image_size=512, camera_distance=70):"
    },
    {
      "line": 47,
      "text": "def project(self, vertices, faces=None):"
    },
    {
      "line": 103,
      "text": "def assign_masks(self, projected_points, visibility, masks, valid_masks):"
    },
    {
      "line": 144,
      "text": "def __init__(self, device='cuda'):"
    },
    {
      "line": 147,
      "text": "def vote(self, assignments, tooth_ids, visibility=None, depths=None, view_weights=None, tooth_bias=2.0):"
    },
    {
      "line": 197,
      "text": "def build_adjacency_matrix(self, vertices, faces, device):"
    },
    {
      "line": 222,
      "text": "def retain_largest_component_per_tooth(self, vertices, faces, labels, logger=None):"
    },
    {
      "line": 255,
      "text": "def apply_graph_cut_smoothing(self, vertices, faces, initial_labels, logger=None):"
    },
    {
      "line": 279,
      "text": "def complete_segmentation_pipeline(self, vertices, faces, initial_labels, logger=None):"
    }
  ]
}

```text

import numpy as np
import torch
from pytorch3d.renderer import (
    FoVPerspectiveCameras,
    look_at_view_transform,
    MeshRasterizer,
    RasterizationSettings
)
from pytorch3d.structures import Meshes
import torch.nn.functional as F
from scipy.sparse import csr_matrix
from scipy.sparse.csgraph import connected_components
from collections import Counter

class Projector:
    def __init__(self, device='cuda', image_size=512, camera_distance=70):
        self.device = device
        self.image_size = image_size
        self.camera_distance = camera_distance
        self.views = [
            [10, 0],  
            [10, 45],  
            [10, 315], 
            [320, 0],  
            [315, 45], 
            [315, 315], 
            [60, 0],    
        ]
        self.cameras = []
        for view in self.views:
            R, T = look_at_view_transform(
                dist=self.camera_distance,
                elev=view[0],
                azim=view[1],
                device=self.device
            )
            self.cameras.append(FoVPerspectiveCameras(
                device=self.device,
                R=R,
                T=T,
                znear=0.01,
                zfar=100.0,
                aspect_ratio=1.0,
                fov=60.0
            ))
    
    def project(self, vertices, faces=None):
        vertices = torch.tensor(vertices, device=self.device)
        faces = torch.tensor(faces, device=self.device) if faces is not None else None
        num_vertices = len(vertices)
        num_views = len(self.views)

        projected_points = torch.zeros((num_views, num_vertices, 2), device=self.device)
        visibility = torch.zeros((num_views, num_vertices), dtype=torch.bool, device=self.device)
        depths = torch.zeros((num_views, num_vertices), device=self.device)
        
        if faces is not None:
            mesh = Meshes(verts=[vertices], faces=[faces])
            face_vertices = faces.flatten()
            face_indices = torch.arange(len(faces), device=self.device).repeat_interleave(3)
            indices = torch.stack([face_vertices, face_indices])
            values = torch.ones(len(face_vertices), device=self.device)
            vert_to_faces_map = torch.sparse_coo_tensor(
                indices, values, 
                (num_vertices, len(faces))
            ).coalesce()
        
        for view_idx, camera in enumerate(self.cameras):
            projected = camera.transform_points_screen(
                vertices.unsqueeze(0),  # [1, N, 3]
                image_size=(self.image_size, self.image_size)
            )
            projected_points[view_idx] = projected[0, :, :2]
            depths[view_idx] = projected[0, :, 2]

            if faces is None:
                visibility[view_idx] = torch.ones(num_vertices, dtype=torch.bool, device=self.device)
                continue
            raster_settings = RasterizationSettings(
                image_size=self.image_size,
                blur_radius=0.0,
                faces_per_pixel=1,
                cull_backfaces=True,
            )
            rasterizer = MeshRasterizer(
                cameras=camera,
                raster_settings=raster_settings
            )
            fragments = rasterizer(mesh)
            face_idxs = fragments.pix_to_face[0, :, :, 0]  # [H, W]
            visible_face_idxs = face_idxs[face_idxs >= 0].unique()

            if len(visible_face_idxs) > 0:
                visible_faces_mask = torch.zeros(len(faces), dtype=torch.float32, device=self.device)
                visible_faces_mask[visible_face_idxs] = 1.0
                vert_visibility = torch.sparse.mm(
                    vert_to_faces_map, 
                    visible_faces_mask.unsqueeze(1)
                ).squeeze(1)
                visibility[view_idx] = (vert_visibility > 0)
        return projected_points, visibility, depths, None

    def assign_masks(self, projected_points, visibility, masks, valid_masks):

        num_views, num_points, _ = projected_points.shape
        num_masks = masks.shape[1]
        assignments = torch.zeros((num_views, num_points, num_masks), dtype=torch.float32, device=self.device)
        
        for view_idx in range(num_views):
            view_points = projected_points[view_idx]  # [N, 2]
            view_masks = torch.tensor(masks[view_idx], device=self.device)  # [M, H, W]
            view_valid = valid_masks[view_idx]  # [M]
            view_visibility = visibility[view_idx]  # [N]
            
            valid_points = (view_points[:, 0] >= 0) & (view_points[:, 0] < self.image_size - 1) & \
                          (view_points[:, 1] >= 0) & (view_points[:, 1] < self.image_size - 1) & \
                          view_visibility
            valid_points_idx = valid_points.nonzero().squeeze(1)
            
            if len(valid_points_idx) > 0:
                valid_coords = view_points[valid_points_idx]  # [K, 2]
                scaled_coords = valid_coords.clone()
                scaled_coords[:, 0] = 2.0 * (scaled_coords[:, 0] / (self.image_size - 1)) - 1.0
                scaled_coords[:, 1] = 2.0 * (scaled_coords[:, 1] / (self.image_size - 1)) - 1.0
                grid = scaled_coords.view(1, -1, 1, 2)  # [1, K, 1, 2]

                for mask_idx in range(num_masks):
                    if not view_valid[mask_idx]:
                        continue
                    mask = view_masks[mask_idx].unsqueeze(0).unsqueeze(0).float()  # [1, 1, H, W]
                    sampled_mask = F.grid_sample(
                        mask, 
                        grid, 
                        mode='bilinear', 
                        padding_mode='border', 
                        align_corners=True
                    )  # [1, 1, K, 1]
                    mask_values = sampled_mask.view(-1)
                    assignments[view_idx, valid_points_idx, mask_idx] = (mask_values > 0.5).float()

        return assignments
    
class Labeler:
    def __init__(self, device='cuda'):
        self.device = device
        
    def vote(self, assignments, tooth_ids, visibility=None, depths=None, view_weights=None, tooth_bias=2.0):
        num_views, num_points, num_masks = assignments.shape
        device = self.device
        
        tooth_votes = torch.zeros((num_points, 16), device=device)
        background_votes = torch.zeros(num_points, device=device)

        base_weights = torch.ones((num_views, num_points), device=device)
        if view_weights is None:
            view_weights_tensor = torch.ones(num_views, device=device)
        else:
            view_weights_tensor = torch.tensor(view_weights, dtype=torch.float32, device=device)
        base_weights *= view_weights_tensor.view(-1, 1)

        if depths is not None:
            depth_weights = 1.0 / depths.clamp(min=1e-6)
            for v_idx in range(num_views):
                v_depths = depth_weights[v_idx]
                min_d, max_d = v_depths.min(), v_depths.max()
                if max_d > min_d:
                    norm_depth_weights = 1.0 + (v_depths - min_d) / (max_d - min_d)
                    base_weights[v_idx] *= norm_depth_weights
        
        if visibility is not None:
            base_weights *= visibility.float()

        for view_idx in range(num_views):
            view_assignments = assignments[view_idx]  # [N, M]
            view_tooth_ids = tooth_ids[view_idx]      # [M]
            view_weights = base_weights[view_idx]    # [N]

            prob_sum_per_point = view_assignments.sum(dim=1)  # [N]
            background_prob = (1.0 - prob_sum_per_point).clamp(min=0.0)
            background_votes += view_weights * background_prob
            vote_divisor = prob_sum_per_point.clamp(min=1e-6)

            for mask_idx in range(num_masks):
                tooth_id = view_tooth_ids[mask_idx].item()
                if tooth_id > 0:
                    prob_this_mask = view_assignments[:, mask_idx] # [N]
                    vote_for_this_tooth = view_weights * (prob_this_mask / vote_divisor)
                    tooth_votes[:, tooth_id - 1] += vote_for_this_tooth
        
        max_tooth_votes, best_tooth_indices = torch.max(tooth_votes, dim=1)
        is_tooth_mask = (max_tooth_votes * tooth_bias) > background_votes
        final_labels = torch.zeros(num_points, dtype=torch.long, device=device)
        final_labels[is_tooth_mask] = best_tooth_indices[is_tooth_mask] + 1
        
        return final_labels.cpu().numpy()
    
    def build_adjacency_matrix(self, vertices, faces, device):
        num_vertices = len(vertices)
        faces_tensor = torch.tensor(faces, device=device) if not isinstance(faces, torch.Tensor) else faces
        i = torch.cat([
            faces_tensor[:, 0], faces_tensor[:, 0], 
            faces_tensor[:, 1], faces_tensor[:, 1], 
            faces_tensor[:, 2], faces_tensor[:, 2]
        ])
        j = torch.cat([
            faces_tensor[:, 1], faces_tensor[:, 2], 
            faces_tensor[:, 0], faces_tensor[:, 2], 
            faces_tensor[:, 0], faces_tensor[:, 1]
        ])
        indices = torch.stack([i, j])
        values = torch.ones(indices.shape[1], device=device)
        adj_matrix_sparse = torch.sparse_coo_tensor(indices, values, (num_vertices, num_vertices)).coalesce()
        adj_matrix_sparse = (adj_matrix_sparse + adj_matrix_sparse.transpose(0,1)).coalesce()
        adj_matrix_sparse = torch.sparse_coo_tensor(
            adj_matrix_sparse.indices(), 
            torch.ones_like(adj_matrix_sparse.values()), 
            adj_matrix_sparse.shape
        ).coalesce()

        return adj_matrix_sparse

    def retain_largest_component_per_tooth(self, vertices, faces, labels, logger=None):
        cleaned_labels = labels.copy()
        unique_labels = np.unique(labels)
        unique_labels = unique_labels[unique_labels > 0]
        
        if len(unique_labels) == 0:
            return cleaned_labels
        adj_matrix_torch = self.build_adjacency_matrix(vertices, faces, self.device)
        indices_cpu = adj_matrix_torch._indices().cpu().numpy()
        values_cpu = adj_matrix_torch._values().cpu().numpy()
        adj_scipy = csr_matrix(
            (values_cpu, (indices_cpu[0], indices_cpu[1])), 
            shape=(len(vertices), len(vertices))
        )
        for tooth_label in unique_labels:
            tooth_mask = (labels == tooth_label)
            tooth_indices = np.where(tooth_mask)[0]
            if len(tooth_indices) < 10:
                continue
            tooth_adj_subgraph = adj_scipy[tooth_indices, :][:, tooth_indices]
            n_components, component_labels = connected_components(
                tooth_adj_subgraph, directed=False
            )
            if n_components > 1:
                component_sizes = Counter(component_labels)
                main_component_id = max(component_sizes, key=component_sizes.get)
                for comp_id, comp_size in component_sizes.items():
                    if comp_id != main_component_id:
                        comp_local_indices = np.where(component_labels == comp_id)[0]
                        comp_global_indices = tooth_indices[comp_local_indices]
                        cleaned_labels[comp_global_indices] = 0
        return cleaned_labels

    def apply_graph_cut_smoothing(self, vertices, faces, initial_labels, logger=None):  
        smoothed_labels = initial_labels.copy()
        try:
            from lift.graph_cuts.smooth import Teeth
            tooth_labels_to_process = list(range(1, 17))
            teeth = Teeth(vertices, faces)
            teeth.v_labels = smoothed_labels
            teeth.update_t_labels()

            for lbl in tooth_labels_to_process:
                if lbl not in teeth.labels:
                    continue
                teeth.fuzzy_cluster(lbl)

            teeth.update_v_labels()
            smoothed_labels = teeth.v_labels

        except Exception as e:
            if logger:
                logger.error(f"graph_cuts error: {e}", exc_info=True)
            return initial_labels
        
        return smoothed_labels

    def complete_segmentation_pipeline(self, vertices, faces, initial_labels, logger=None):
        current_labels = initial_labels.copy()
        current_labels = self.retain_largest_component_per_tooth(
            vertices, faces, current_labels, logger
        )
        final_labels = self.apply_graph_cut_smoothing(
            vertices, faces, current_labels, logger
        )
        return final_labels
    

```

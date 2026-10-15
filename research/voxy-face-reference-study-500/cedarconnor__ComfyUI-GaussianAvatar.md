# Facial rendering source scan: cedarconnor/ComfyUI-GaussianAvatar

Repository: https://github.com/cedarconnor/ComfyUI-GaussianAvatar

Pinned source revision: 01b38e7fa590517772a4a4747fed0e1db88acd14

Discovery topic: FLAME 3D

Description: FLAME-based 3D Gaussian head avatar creation in ComfyUI

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"flame": ["lib/tracking/flame_fitter.py", "lib/types/flame.py", "lib/utils/flame_deformation.py"], "deform": ["lib/utils/flame_deformation.py"]}

## Source file: lib/utils/flame_deformation.py

{
  "path": "lib/utils/flame_deformation.py",
  "blob_sha": "ba13df2d3cbb065ee4845032abe5fa186ddcf109",
  "sha256": "1ba6b28449591c442969c3846b1e35022f1a4d7e879f3ef73a8a04a2e6ddf117",
  "lines": 307,
  "bytes": 10545,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 1,
    "skin": 4,
    "blendshape": 7,
    "flame": 22,
    "deform": 16,
    "lbs": 39,
    "rig": 2
  },
  "url": "https://github.com/cedarconnor/ComfyUI-GaussianAvatar/blob/01b38e7fa590517772a4a4747fed0e1db88acd14/lib/utils/flame_deformation.py",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "def batch_rodrigues(rot_vecs: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 50,
      "text": "def lbs_forward("
    },
    {
      "line": 105,
      "text": "def apply_expression_blendshapes("
    },
    {
      "line": 128,
      "text": "def compute_per_gaussian_lbs_weights("
    },
    {
      "line": 184,
      "text": "def __init__("
    },
    {
      "line": 219,
      "text": "def get_canonical_vertices(self, shape_params: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 225,
      "text": "def deform_gaussians("
    },
    {
      "line": 304,
      "text": "def invalidate_cache(self):"
    }
  ]
}

```text

"""
FLAME Linear Blend Skinning (LBS) deformation for Gaussian Splatting.

This module provides per-Gaussian deformation based on FLAME expression
and pose parameters, enabling animatable 3D avatar rendering.
"""

import torch
import torch.nn.functional as F
import numpy as np
from typing import Dict, Optional, Tuple

def batch_rodrigues(rot_vecs: torch.Tensor) -> torch.Tensor:
    """
    Convert axis-angle rotations to rotation matrices.
    
    Args:
        rot_vecs: (N, 3) axis-angle vectors
        
    Returns:
        Rotation matrices (N, 3, 3)
    """
    batch_size = rot_vecs.shape[0]
    device = rot_vecs.device
    dtype = rot_vecs.dtype
    
    angle = torch.norm(rot_vecs + 1e-8, dim=1, keepdim=True)
    rot_dir = rot_vecs / angle
    
    cos = torch.cos(angle).unsqueeze(2)
    sin = torch.sin(angle).unsqueeze(2)
    
    # Bx1 arrays
    rx, ry, rz = rot_dir[:, 0:1], rot_dir[:, 1:2], rot_dir[:, 2:3]
    
    K = torch.zeros((batch_size, 3, 3), dtype=dtype, device=device)
    K[:, 0, 1] = -rz.squeeze()
    K[:, 0, 2] = ry.squeeze()
    K[:, 1, 0] = rz.squeeze()
    K[:, 1, 2] = -rx.squeeze()
    K[:, 2, 0] = -ry.squeeze()
    K[:, 2, 1] = rx.squeeze()
    
    ident = torch.eye(3, dtype=dtype, device=device).unsqueeze(0)
    rot_mat = ident + sin * K + (1 - cos) * torch.bmm(K, K)
    
    return rot_mat


def lbs_forward(
    vertices: torch.Tensor,
    lbs_weights: torch.Tensor,
    pose_params: torch.Tensor,
    parents: torch.Tensor,
    n_joints: int = 5
) -> torch.Tensor:
    """
    Apply Linear Blend Skinning to deform vertices.
    
    Args:
        vertices: (N, 3) canonical vertices
        lbs_weights: (N, J) skinning weights for N vertices across J joints
        pose_params: (J*3,) axis-angle pose parameters
        parents: (J,) kinematic tree parent indices
        n_joints: Number of joints in the kinematic tree
        
    Returns:
        Deformed vertices (N, 3)
    """
    device = vertices.device
    dtype = vertices.dtype
    n_verts = vertices.shape[0]
    
    # Reshape pose to joint rotations
    pose_params = pose_params.reshape(-1, 3)[:n_joints]  # (J, 3)
    rot_mats = batch_rodrigues(pose_params)  # (J, 3, 3)
    
    # Build kinematic chain transforms
    transforms = []
    for j in range(n_joints):
        parent = parents[j].item() if parents[j] >= 0 else -1
        if parent < 0:
            transforms.append(rot_mats[j])
        else:
            transforms.append(transforms[parent] @ rot_mats[j])
    
    transforms = torch.stack(transforms, dim=0)  # (J, 3, 3)
    
    # Apply LBS
    # Ensure weights sum to 1 (if not already normalized)
    lbs_weights = lbs_weights[:, :n_joints]  # Truncate to number of joints
    
    # Transform vertices by each joint and weight
    # v' = sum_j (w_j * R_j * v)
    verts_homog = vertices.unsqueeze(1)  # (N, 1, 3)
    transformed = torch.einsum('njk,nk->nj', transforms.unsqueeze(0).expand(n_verts, -1, -1, -1).reshape(n_verts, n_joints, 3, 3), verts_homog.expand(-1, n_joints, -1).reshape(n_verts, n_joints, 3))
    
    # Weight and sum
    weighted = transformed * lbs_weights.unsqueeze(-1)  # (N, J, 3)
    deformed = weighted.sum(dim=1)  # (N, 3)
    
    return deformed


def apply_expression_blendshapes(
    vertices: torch.Tensor,
    expr_params: torch.Tensor,
    expr_dirs: torch.Tensor
) -> torch.Tensor:
    """
    Apply expression blendshapes to vertices.
    
    Args:
        vertices: (N, 3) base vertices
        expr_params: (E,) expression coefficients
        expr_dirs: (N, 3, E) expression blendshape directions
        
    Returns:
        Vertices with expression applied (N, 3)
    """
    # expr_dirs: (N, 3, E)
    # expr_params: (E,)
    # Result: (N, 3)
    blend = torch.einsum('ijk,k->ij', expr_dirs, expr_params)
    return vertices + blend


def compute_per_gaussian_lbs_weights(
    gaussian_xyz: torch.Tensor,
    flame_vertices: torch.Tensor,
    flame_lbs_weights: torch.Tensor,
    k: int = 4
) -> torch.Tensor:
    """
    Compute LBS weights for Gaussians based on nearest FLAME vertices.
    
    Uses K-nearest neighbor interpolation to assign skinning weights
    to Gaussians that may not be 1:1 with mesh vertices.
    
    Args:
        gaussian_xyz: (N, 3) Gaussian positions
        flame_vertices: (V, 3) FLAME template vertices
        flame_lbs_weights: (V, J) LBS weights for FLAME vertices
        k: Number of nearest neighbors for interpolation
        
    Returns:
        (N, J) LBS weights for each Gaussian
    """
    device = gaussian_xyz.device
    n_gaussians = gaussian_xyz.shape[0]
    n_verts = flame_vertices.shape[0]
    n_joints = flame_lbs_weights.shape[1]
    
    # Compute pairwise distances (N, V)
    dists = torch.cdist(gaussian_xyz, flame_vertices)
    
    # Find K nearest vertices
    _, knn_indices = torch.topk(dists, k, largest=False, dim=1)  # (N, k)
    
    # Get distances to K nearest
    knn_dists = torch.gather(dists, 1, knn_indices)  # (N, k)
    
    # Inverse distance weighting
    weights = 1.0 / (knn_dists + 1e-8)
    weights = weights / weights.sum(dim=1, keepdim=True)  # Normalize  (N, k)
    
    # Get LBS weights for KNN vertices
    knn_lbs = flame_lbs_weights[knn_indices]  # (N, k, J)
    
    # Weighted average of LBS weights
    gaussian_lbs = torch.einsum('nk,nkj->nj', weights, knn_lbs)  # (N, J)
    
    return gaussian_lbs


class FLAMEDeformer:
    """
    Deforms Gaussian positions based on FLAME expression and pose.
    
    Stores the FLAME model parameters needed for deformation and caches
    per-Gaussian LBS weights for efficient animation.
    """
    
    def __init__(
        self,
        v_template: torch.Tensor,
        shapedirs: torch.Tensor,
        exprdirs: torch.Tensor,
        lbs_weights: torch.Tensor,
        parents: torch.Tensor,
        device: torch.device = None
    ):
        """
        Initialize deformer with FLAME model data.
        
        Args:
            v_template: (V, 3) FLAME template vertices
            shapedirs: (V, 3, S) shape blendshape directions
            exprdirs: (V, 3, E) expression blendshape directions
            lbs_weights: (V, J) LBS weights
            parents: (J,) kinematic tree parents
            device: Target device
        """
        if device is None:
            device = v_template.device
            
        self.device = device
        self.v_template = v_template.to(device)
        self.shapedirs = shapedirs.to(device)
        self.exprdirs = exprdirs.to(device)
        self.lbs_weights = lbs_weights.to(device)
        self.parents = parents.to(device)
        self.n_joints = lbs_weights.shape[1]
        
        # Cache for per-Gaussian LBS weights (computed on first use)
        self._gaussian_lbs_cache: Dict[int, torch.Tensor] = {}
        self._gaussian_binding_cache: Dict[int, torch.Tensor] = {}
    
    def get_canonical_vertices(self, shape_params: torch.Tensor) -> torch.Tensor:
        """Get shaped FLAME vertices (identity applied, no expression/pose)."""
        # Apply shape blendshapes
        shape_blend = torch.einsum('ijk,k->ij', self.shapedirs, shape_params.to(self.device))
        return self.v_template + shape_blend
    
    def deform_gaussians(
        self,
        gaussian_xyz: torch.Tensor,
        shape_params: torch.Tensor,
        expression_params: torch.Tensor,
        pose_params: torch.Tensor,
        canonical_verts: Optional[torch.Tensor] = None
    ) -> torch.Tensor:
        """
        Deform Gaussian positions based on FLAME parameters.
        
        Args:
            gaussian_xyz: (N, 3) canonical Gaussian positions
            shape_params: (S,) FLAME shape coefficients
            expression_params: (E,) FLAME expression coefficients
            pose_params: (P,) FLAME pose parameters (axis-angle)
            canonical_verts: Optional precomputed canonical vertices
            
        Returns:
            Deformed Gaussian positions (N, 3)
        """
        device = gaussian_xyz.device
        n_gaussians = gaussian_xyz.shape[0]
        
        # Get canonical FLAME vertices
        if canonical_verts is None:
            canonical_verts = self.get_canonical_vertices(shape_params)
        
        # Compute or retrieve per-Gaussian LBS weights
        # Cache key is n_gaussians count - recompute when count changes (after densify/prune)
        cache_key = n_gaussians
        if cache_key not in self._gaussian_lbs_cache:
            self._gaussian_lbs_cache[cache_key] = compute_per_gaussian_lbs_weights(
                gaussian_xyz.to(self.device),
                canonical_verts,
                self.lbs_weights,
                k=4
            )
            # Also clear old cache entries if size changed
            keys_to_remove = [k for k in self._gaussian_lbs_cache if k != cache_key]
            for k in keys_to_remove:
                del self._gaussian_lbs_cache[k]
        
        gaussian_lbs = self._gaussian_lbs_cache[cache_key].to(device)
        
        # Apply expression blendshapes to get target pose vertices
        expr_params = expression_params.to(self.device)
        pose_params = pose_params.to(self.device)
        
        # For Gaussians, we apply a simplified deformation:
        # 1. Expression offset from nearest vertices
        # 2. Pose rotation via LBS
        
        # Expression: find offset per-Gaussian from expression on mesh
        expr_blend = torch.einsum('ijk,k->ij', self.exprdirs, expr_params)  # (V, 3)
        
        # Interpolate expression offsets to Gaussians (use LBS weights as proxy)
        # Compute nearest vertex indices for each Gaussian
        if cache_key not in self._gaussian_binding_cache:
            dists = torch.cdist(gaussian_xyz.to(self.device), canonical_verts)
            self._gaussian_binding_cache[cache_key] = dists.argmin(dim=1)
        
        nearest_idx = self._gaussian_binding_cache[cache_key]
        expr_offsets = expr_blend[nearest_idx].to(device)  # (N, 3)
        
        # Apply expression offset
        posed_gaussians = gaussian_xyz + expr_offsets
        
        # Apply pose via LBS
        deformed = lbs_forward(
            posed_gaussians,
            gaussian_lbs,
            pose_params,
            self.parents,
            n_joints=self.n_joints
        )
        
        return deformed
    
    def invalidate_cache(self):
        """Clear cached LBS weights (call after densification)."""
        self._gaussian_lbs_cache.clear()
        self._gaussian_binding_cache.clear()


```

## Source file: lib/types/flame.py

{
  "path": "lib/types/flame.py",
  "blob_sha": "bd2964c99d33a30fdf81efe4a6c42ca70fa41fe1",
  "sha256": "e9023aade88cd4d2704eb6666fe53beb492f5019b2f7cc5e6079a782742f7761",
  "lines": 125,
  "bytes": 4381,
  "selection_score": 15,
  "keyword_line_counts": {
    "flame": 9
  },
  "url": "https://github.com/cedarconnor/ComfyUI-GaussianAvatar/blob/01b38e7fa590517772a4a4747fed0e1db88acd14/lib/types/flame.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def __post_init__(self):"
    },
    {
      "line": 26,
      "text": "def save(self, path: str, source_image: Optional[torch.Tensor] = None):"
    },
    {
      "line": 44,
      "text": "def load(cls, path: str) -> 'FLAMEIdentity':"
    },
    {
      "line": 61,
      "text": "def _create_thumbnail(self, image: torch.Tensor, size: int) -> bytes:"
    },
    {
      "line": 69,
      "text": "def get_thumbnail_image(self) -> Optional[np.ndarray]:"
    },
    {
      "line": 85,
      "text": "def frame_count(self) -> int:"
    },
    {
      "line": 88,
      "text": "def __post_init__(self):"
    },
    {
      "line": 97,
      "text": "def neutral(cls, num_frames: int = 1, expression_dim: int = 100, pose_dim: int = 15) -> 'FLAMEMotion':"
    },
    {
      "line": 103,
      "text": "def get_frame(self, idx: int) -> 'FLAMEMotion':"
    },
    {
      "line": 109,
      "text": "def save(self, path: str):"
    },
    {
      "line": 117,
      "text": "def load(cls, path: str) -> 'FLAMEMotion':"
    }
  ]
}

```text

from dataclasses import dataclass
import torch
import numpy as np
import io
from PIL import Image
from typing import Optional

@dataclass
class FLAMEIdentity:
    """
    FLAME identity parameters (shape coefficients).
    Constant across all frames for one person.
    """
    shape: torch.Tensor  # (1, 300) or (1, 100)
    
    # Metadata
    source_image_hash: Optional[str] = None
    extraction_method: str = "MICA"
    thumbnail: Optional[bytes] = None
    
    def __post_init__(self):
        self.shape = self.shape.to(torch.float32)
        if self.shape.dim() == 1:
            self.shape = self.shape.unsqueeze(0)
            
    def save(self, path: str, source_image: Optional[torch.Tensor] = None):
        """Save to .npz"""
        data = {
            'schema_version': np.array([1]),
            'shape': self.shape.cpu().numpy(),
            'extraction_method': np.array([self.extraction_method], dtype=object),
            'source_image_hash': np.array([self.source_image_hash or ""], dtype=object),
        }
        
        if source_image is not None:
            thumb = self._create_thumbnail(source_image, size=128)
            data['thumbnail'] = np.frombuffer(thumb, dtype=np.uint8)
        elif self.thumbnail is not None:
            data['thumbnail'] = np.frombuffer(self.thumbnail, dtype=np.uint8)
            
        np.savez(path, **data)
        
    @classmethod
    def load(cls, path: str) -> 'FLAMEIdentity':
        data = np.load(path, allow_pickle=True)
        version = int(data['schema_version'][0])
        if version != 1:
            raise ValueError(f"Unsupported schema version: {version}")
            
        thumbnail = None
        if 'thumbnail' in data:
            thumbnail = bytes(data['thumbnail'])
            
        return cls(
            shape=torch.from_numpy(data['shape']).to(torch.float32),
            extraction_method=str(data['extraction_method'][0]),
            source_image_hash=str(data['source_image_hash'][0]) or None,
            thumbnail=thumbnail
        )

    def _create_thumbnail(self, image: torch.Tensor, size: int) -> bytes:
        img = (image[0].cpu().numpy() * 255).astype(np.uint8)
        pil_img = Image.fromarray(img)
        pil_img.thumbnail((size, size))
        buffer = io.BytesIO()
        pil_img.save(buffer, format='JPEG', quality=85)
        return buffer.getvalue()
        
    def get_thumbnail_image(self) -> Optional[np.ndarray]:
        if self.thumbnail is None:
            return None
        buffer = io.BytesIO(self.thumbnail)
        pil_img = Image.open(buffer)
        return np.array(pil_img)

@dataclass
class FLAMEMotion:
    """
    FLAME motion parameters (expression + pose).
    """
    expression: torch.Tensor  # (N, 100)
    pose: torch.Tensor        # (N, 15)
    
    @property
    def frame_count(self) -> int:
        return self.expression.shape[0]
        
    def __post_init__(self):
        self.expression = self.expression.to(torch.float32)
        self.pose = self.pose.to(torch.float32)
        if self.expression.dim() == 1:
            self.expression = self.expression.unsqueeze(0)
        if self.pose.dim() == 1:
            self.pose = self.pose.unsqueeze(0)
            
    @classmethod
    def neutral(cls, num_frames: int = 1, expression_dim: int = 100, pose_dim: int = 15) -> 'FLAMEMotion':
        return cls(
            expression=torch.zeros(num_frames, expression_dim, dtype=torch.float32),
            pose=torch.zeros(num_frames, pose_dim, dtype=torch.float32)
        )
        
    def get_frame(self, idx: int) -> 'FLAMEMotion':
        return FLAMEMotion(
            expression=self.expression[idx:idx+1],
            pose=self.pose[idx:idx+1]
        )
        
    def save(self, path: str):
        np.savez(path,
            schema_version=np.array([1]),
            expression=self.expression.cpu().numpy(),
            pose=self.pose.cpu().numpy()
        )
        
    @classmethod
    def load(cls, path: str) -> 'FLAMEMotion':
        data = np.load(path, allow_pickle=True)
        version = int(data['schema_version'][0])
        if version != 1:
            raise ValueError(f"Unsupported schema version: {version}")
        return cls(
            expression=torch.from_numpy(data['expression']).to(torch.float32),
            pose=torch.from_numpy(data['pose']).to(torch.float32)
        )


```

## Source file: lib/tracking/flame_fitter.py

{
  "path": "lib/tracking/flame_fitter.py",
  "blob_sha": "7c813e6e47567fdfe3458d04016e57e50b92bf0e",
  "sha256": "e54505d65d1166cf0ecbc3043715f500c5f64dfe8bdc259a5f78a9eb0ecf412f",
  "lines": 434,
  "bytes": 17288,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "blendshape": 1,
    "flame": 51,
    "deform": 3,
    "lbs": 6,
    "rig": 1
  },
  "url": "https://github.com/cedarconnor/ComfyUI-GaussianAvatar/blob/01b38e7fa590517772a4a4747fed0e1db88acd14/lib/tracking/flame_fitter.py",
  "function_signature_candidates": [
    {
      "line": 18,
      "text": "def _to_numpy(x) -> np.ndarray:"
    },
    {
      "line": 47,
      "text": "def __init__(self,"
    },
    {
      "line": 73,
      "text": "def _load_flame(self, path: str):"
    },
    {
      "line": 149,
      "text": "def _verify_gradients(self):"
    },
    {
      "line": 173,
      "text": "def get_deformer_data(self) -> Dict[str, torch.Tensor]:"
    },
    {
      "line": 188,
      "text": "def forward(self,"
    },
    {
      "line": 220,
      "text": "def get_landmarks(self, vertices: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 232,
      "text": "def fit_identity(self,"
    },
    {
      "line": 270,
      "text": "def objective(shape_params):"
    },
    {
      "line": 310,
      "text": "def fit_expression(self,"
    },
    {
      "line": 374,
      "text": "def objective(expr_params):"
    },
    {
      "line": 409,
      "text": "def get_flame_path() -> str:"
    }
  ]
}

```text

"""
FLAME Parameter Fitter.

Fits FLAME parameters (shape, expression, pose) to 2D/3D landmarks using
PyTorch optimization.
"""

import os
import pickle
import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
from typing import Tuple, Optional, Dict, Any
from .landmark_mapping import get_flame_landmark_indices


def _to_numpy(x) -> np.ndarray:
    """
    Convert chumpy object or numpy array to numpy array.
    
    Chumpy is an auto-differentiation library used in older FLAME models.
    It's incompatible with newer NumPy versions, so we need to extract
    the underlying data.
    """
    if hasattr(x, 'r'):
        # Chumpy object - get the underlying numpy array
        return np.array(x.r, dtype=np.float32)
    elif hasattr(x, '__array__'):
        return np.asarray(x, dtype=np.float32)
    else:
        return np.array(x, dtype=np.float32)


class FLAMEFitter:
    """
    Fit FLAME parameters to facial landmarks via differentiable optimization.
    
    Uses PyTorch autograd for gradient computation. Requires FLAME model file.
    
    Usage:
        fitter = FLAMEFitter("/path/to/generic_model.pkl")
        shape = fitter.fit_identity(landmarks_68)
        expr, pose = fitter.fit_expression(landmarks_68, shape)
    """
    
    def __init__(self,
                 flame_model_path: str,
                 device: str = "cuda",
                 num_shape_params: int = 100,
                 num_expression_params: int = 50):
        """
        Initialize FLAME fitter.
        
        Args:
            flame_model_path: Path to FLAME generic_model.pkl
            device: 'cuda' or 'cpu'
            num_shape_params: Number of shape (identity) coefficients to use
            num_expression_params: Number of expression coefficients to use
        """
        self.device = torch.device(device if torch.cuda.is_available() else "cpu")
        self.num_shape = num_shape_params
        self.num_expr = num_expression_params
        
        # Load FLAME model
        self._load_flame(flame_model_path)
        
        # Get landmark indices
        self.landmark_indices = torch.from_numpy(
            get_flame_landmark_indices()
        ).long().to(self.device)
    
    def _load_flame(self, path: str):
        """Load FLAME model from pickle."""
        if not os.path.exists(path):
            raise FileNotFoundError(
                f"FLAME model not found at {path}. "
                "Please download from https://flame.is.tue.mpg.de/"
            )
        
        with open(path, 'rb') as f:
            flame_data = pickle.load(f, encoding='latin1')
        
        # Mean template vertices (5023, 3)
        # Ensure contiguous float32 for gradient propagation
        v_template_np = _to_numpy(flame_data['v_template']).copy()
        self.v_template = torch.from_numpy(v_template_np).float().contiguous().to(self.device)
        
        # Shape blend shapes (5023, 3, num_betas)
        shapedirs = _to_numpy(flame_data['shapedirs']).copy()
        if shapedirs.shape[2] < self.num_shape:
            self.num_shape = shapedirs.shape[2]
        shapedirs = shapedirs[:, :, :self.num_shape].copy()  # Ensure contiguous
        self.shapedirs = torch.from_numpy(shapedirs).float().contiguous().to(self.device)
        
        # Expression blend shapes (if available)
        self.exprdirs = None
        if 'shapedirs' in flame_data:
            # Some FLAME versions have expression blendshapes in shapedirs
            # after the shape components, or in a separate array
            pass
        
        # Check for separate expression data
        expr_key = None
        for key in ['exprbetas', 'expr_dirs', 'expression']:
            if key in flame_data:
                expr_key = key
                break
        
        if expr_key:
            exprdirs = _to_numpy(flame_data[expr_key]).copy()
            if exprdirs.shape[2] < self.num_expr:
                self.num_expr = exprdirs.shape[2]
            exprdirs = exprdirs[:, :, :self.num_expr].copy()
            self.exprdirs = torch.from_numpy(exprdirs).float().contiguous().to(self.device)
        else:
            # Fallback: use last shape components as expression
            # (FLAME 2020 has 400 total blend shapes: 300 shape + 100 expression)
            # shapedirs already converted above, but need the original full array
            shapedirs_full = _to_numpy(flame_data['shapedirs']).copy()
            total_blends = shapedirs_full.shape[2]
            if total_blends > 300:
                expr_start = 300
                expr_count = min(self.num_expr, total_blends - 300)
                exprdirs = shapedirs_full[:, :, expr_start:expr_start+expr_count].copy()
                self.exprdirs = torch.from_numpy(exprdirs).float().contiguous().to(self.device)
                self.num_expr = expr_count
        
        # LBS (Linear Blend Skinning) weights for pose deformation
        # 'weights' in FLAME pickle: (V, J) where J is number of joints
        self.lbs_weights = None
        if 'weights' in flame_data:
            lbs_weights = _to_numpy(flame_data['weights']).copy()
            self.lbs_weights = torch.from_numpy(lbs_weights).float().contiguous().to(self.device)
        
        # Kinematic tree for joint hierarchy
        # 'kintree_table' in FLAME pickle: (2, J) where row 0 is parent indices
        self.kintree_parents = None
        if 'kintree_table' in flame_data:
            kintree = _to_numpy(flame_data['kintree_table']).copy()
            # Row 0 contains parent indices (-1 for root)
            parents = kintree[0].astype(np.int64)
            parents[0] = -1  # Root has no parent
            self.kintree_parents = torch.from_numpy(parents).long().to(self.device)
        
        # Verify gradient flow works
        self._verify_gradients()
    
    def _verify_gradients(self):
        """Test that gradient flows properly through forward pass."""
        test_shape = torch.randn(1, self.num_shape, device=self.device, requires_grad=True)
        
        # Test einsum directly
        test_blend = torch.einsum('vcs,bs->bvc', self.shapedirs, test_shape)
        
        if not test_blend.requires_grad:
            print(f"[!] GRADIENT DEBUG:")
            print(f"    shapedirs dtype: {self.shapedirs.dtype}")
            print(f"    shapedirs device: {self.shapedirs.device}")
            print(f"    shapedirs requires_grad: {self.shapedirs.requires_grad}")
            print(f"    test_shape dtype: {test_shape.dtype}")
            print(f"    test_shape device: {test_shape.device}")
            print(f"    test_shape requires_grad: {test_shape.requires_grad}")
            print(f"    test_blend requires_grad: {test_blend.requires_grad}")
            
            # Try alternative - mm should always work
            flat = self.shapedirs.reshape(-1, self.num_shape)
            test_mm = torch.mm(test_shape, flat.T)
            print(f"    torch.mm result requires_grad: {test_mm.requires_grad}")
        else:
            print("[OK] Gradient flow verified through FLAME forward pass")
    
    def get_deformer_data(self) -> Dict[str, torch.Tensor]:
        """
        Get FLAME model data needed for FLAMEDeformer initialization.
        
        Returns:
            Dict with v_template, shapedirs, exprdirs, lbs_weights, parents
        """
        return {
            'v_template': self.v_template,
            'shapedirs': self.shapedirs,
            'exprdirs': self.exprdirs,
            'lbs_weights': self.lbs_weights,
            'parents': self.kintree_parents
        }
    
    def forward(self,
                shape: torch.Tensor,
                expression: Optional[torch.Tensor] = None) -> torch.Tensor:
        """
        Forward FLAME model to get vertices (shape + expression only).
        
        Args:
            shape: (B, num_shape) identity coefficients
            expression: (B, num_expr) expression coefficients (optional)
        
        Returns:
            vertices: (B, 5023, 3) FLAME mesh vertices
        """
        batch_size = shape.shape[0]
        
        # Start with template - expand for batch
        v = self.v_template.unsqueeze(0).expand(batch_size, -1, -1)
        
        # Add shape blend using einsum for proper gradient flow
        # shapedirs: (V, 3, S) where V=5023, S=num_shape
        # shape: (B, S)
        # result: (B, V, 3)
        shape_blend = torch.einsum('vcs,bs->bvc', self.shapedirs, shape)
        v = v + shape_blend
        
        # Add expression blend
        if expression is not None and self.exprdirs is not None:
            expr_blend = torch.einsum('vcs,bs->bvc', self.exprdirs, expression)
            v = v + expr_blend
        
        return v
    
    def get_landmarks(self, vertices: torch.Tensor) -> torch.Tensor:
        """
        Extract 68 landmarks from FLAME vertices.
        
        Args:
            vertices: (B, 5023, 3) FLAME mesh vertices
            
        Returns:
            landmarks: (B, 68, 3) landmark positions
        """
        return vertices[:, self.landmark_indices, :]
    
    def fit_identity(self,
                     target_landmarks: torch.Tensor,
                     num_iterations: int = 200,
                     lr: float = 0.05,
                     verbose: bool = False) -> torch.Tensor:
        """
        Fit FLAME shape (identity) parameters to landmarks.
        
        Args:
            target_landmarks: (68, 3) target landmarks (normalized or pixel coords)
            num_iterations: Number of optimization steps
            lr: Learning rate
            verbose: Print progress
        
        Returns:
            shape: (1, num_shape) optimized identity parameters
        """
        # Prepare target
        target = target_landmarks.to(self.device).float()
        if target.dim() == 2:
            target = target.unsqueeze(0)
        
        # Center target
        target_center = target.mean(dim=1, keepdim=True)
        target_centered = target - target_center
        
        # Use scipy optimization since ComfyUI disables PyTorch gradients
        from scipy.optimize import minimize
        
        # Convert target to numpy for scipy
        target_np = target_centered.squeeze(0).cpu().numpy()  # (68, 3)
        shapedirs_np = self.shapedirs.cpu().numpy()  # (5023, 3, num_shape)
        v_template_np = self.v_template.cpu().numpy()  # (5023, 3)
        landmark_indices = self.landmark_indices.cpu().numpy()  # (68,)
        
        # Target scale for normalization
        target_scale = np.sqrt((target_np ** 2).sum())
        
        def objective(shape_params):
            """Compute loss for given shape parameters."""
            # shape_params: (num_shape,)
            # Apply blend shapes
            blend = np.einsum('vcs,s->vc', shapedirs_np, shape_params)  # (5023, 3)
            verts = v_template_np + blend  # (5023, 3)
            
            # Get landmarks
            landmarks = verts[landmark_indices]  # (68, 3)
            
            # Center
            landmarks_centered = landmarks - landmarks.mean(axis=0, keepdims=True)
            
            # Scale normalize
            pred_scale = np.sqrt((landmarks_centered ** 2).sum()) + 1e-8
            landmarks_normalized = landmarks_centered / pred_scale * target_scale
            
            # L2 loss
            loss = np.mean((landmarks_normalized - target_np) ** 2)
            
            # Regularization
            reg = 0.001 * np.mean(shape_params ** 2)
            
            return loss + reg
        
        # Initialize
        x0 = np.zeros(self.num_shape, dtype=np.float64)
        
        # Optimize
        result = minimize(objective, x0, method='L-BFGS-B', 
                         options={'maxiter': num_iterations, 'disp': verbose})
        
        if verbose:
            print(f"  Identity fit: final loss = {result.fun:.6f}, success = {result.success}")
        
        # Convert result back to torch
        best_shape = torch.from_numpy(result.x.astype(np.float32)).unsqueeze(0).to(self.device)
        
        return best_shape
    
    def fit_expression(self,
                       target_landmarks: torch.Tensor,
                       shape: torch.Tensor,
                       num_iterations: int = 100,
                       lr: float = 0.1,
                       expression_prior: Optional[torch.Tensor] = None,
                       verbose: bool = False) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Fit FLAME expression and pose to landmarks with fixed identity.
        
        Args:
            target_landmarks: (68, 3) target landmarks
            shape: (1, num_shape) fixed identity parameters
            num_iterations: Number of optimization steps
            lr: Learning rate
            expression_prior: Optional (num_expr,) initial expression guess
            verbose: Print progress
        
        Returns:
            expression: (1, num_expr) expression parameters
            pose: (1, 15) pose parameters (placeholder zeros for now)
        """
        # Prepare target
        target = target_landmarks.to(self.device).float()
        if target.dim() == 2:
            target = target.unsqueeze(0)
        
        # Center target
        target_center = target.mean(dim=1, keepdim=True)
        target_centered = target - target_center
        
        # Fixed shape
        shape_fixed = shape.to(self.device).detach()
        
        # Initialize expression (with prior if available)
        if expression_prior is not None:
            expr_init = expression_prior.to(self.device).float()
            if expr_init.dim() == 1:
                expr_init = expr_init.unsqueeze(0)
            # Pad or truncate to match our dimension
            if expr_init.shape[1] < self.num_expr:
                expr_init = torch.cat([
                    expr_init,
                    torch.zeros(1, self.num_expr - expr_init.shape[1], device=self.device)
                ], dim=1)
            elif expr_init.shape[1] > self.num_expr:
                expr_init = expr_init[:, :self.num_expr]
            x0 = expr_init.squeeze(0).cpu().numpy().astype(np.float64)
        else:
            x0 = np.zeros(self.num_expr, dtype=np.float64)
        
        # Use scipy optimization since ComfyUI disables PyTorch gradients
        from scipy.optimize import minimize
        
        # Convert to numpy
        target_np = target_centered.squeeze(0).cpu().numpy()  # (68, 3)
        shapedirs_np = self.shapedirs.cpu().numpy()  # (5023, 3, num_shape)
        exprdirs_np = self.exprdirs.cpu().numpy() if self.exprdirs is not None else None
        v_template_np = self.v_template.cpu().numpy()  # (5023, 3)
        shape_np = shape_fixed.squeeze(0).cpu().numpy()  # (num_shape,)
        landmark_indices = self.landmark_indices.cpu().numpy()  # (68,)
        
        target_scale = np.sqrt((target_np ** 2).sum())
        
        def objective(expr_params):
            """Compute loss for given expression parameters."""
            # Apply shape + expression blend
            shape_blend = np.einsum('vcs,s->vc', shapedirs_np, shape_np)
            verts = v_template_np + shape_blend
            
            if exprdirs_np is not None:
                expr_blend = np.einsum('vcs,s->vc', exprdirs_np, expr_params)
                verts = verts + expr_blend
            
            # Get landmarks
            landmarks = verts[landmark_indices]
            landmarks_centered = landmarks - landmarks.mean(axis=0, keepdims=True)
            
            # Scale normalize
            pred_scale = np.sqrt((landmarks_centered ** 2).sum()) + 1e-8
            landmarks_normalized = landmarks_centered / pred_scale * target_scale
            
            # Loss
            loss = np.mean((landmarks_normalized - target_np) ** 2)
            reg = 0.0001 * np.mean(expr_params ** 2)
            
            return loss + reg
        
        # Optimize
        result = minimize(objective, x0, method='L-BFGS-B', 
                         options={'maxiter': num_iterations, 'disp': False})
        
        # Return expression and placeholder pose
        expression = torch.from_numpy(result.x.astype(np.float32)).unsqueeze(0).to(self.device)
        pose = torch.zeros(1, 15, device=self.device)
        
        return expression, pose


def get_flame_path() -> str:
    """
    Get path to FLAME model file.
    
    Checks standard locations:
    1. ComfyUI/models/flame/generic_model.pkl
    2. Environment variable FLAME_MODEL_PATH
    """
    # Try ComfyUI models dir
    try:
        import folder_paths
        flame_dir = os.path.join(folder_paths.models_dir, "flame")
        flame_path = os.path.join(flame_dir, "generic_model.pkl")
        if os.path.exists(flame_path):
            return flame_path
    except ImportError:
        pass
    
    # Try environment variable
    env_path = os.environ.get("FLAME_MODEL_PATH")
    if env_path and os.path.exists(env_path):
        return env_path
    
    # Fallback to relative path
    base_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__))))
    return os.path.join(base_dir, "models", "flame", "generic_model.pkl")


```

## Source file: install.py

{
  "path": "install.py",
  "blob_sha": "2e0be0cfb124b01a75c8c3285b16650b75c568f3",
  "sha256": "2998b4d7c390457777309ed97e82365b235de395e08c630a1c808b17e571068b",
  "lines": 396,
  "bytes": 14615,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 8
  },
  "url": "https://github.com/cedarconnor/ComfyUI-GaussianAvatar/blob/01b38e7fa590517772a4a4747fed0e1db88acd14/install.py",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "def get_python_version():"
    },
    {
      "line": 30,
      "text": "def get_cuda_version():"
    },
    {
      "line": 57,
      "text": "def get_torch_version():"
    },
    {
      "line": 67,
      "text": "def install_requirements():"
    },
    {
      "line": 81,
      "text": "def check_cuda_extensions():"
    },
    {
      "line": 97,
      "text": "def download_wheel(url, dest_dir):"
    },
    {
      "line": 111,
      "text": "def get_release_wheels():"
    },
    {
      "line": 130,
      "text": "def find_matching_wheel(wheels, package_name, python_ver, cuda_ver):"
    },
    {
      "line": 151,
      "text": "def get_community_wheel_urls(package_name, python_ver, cuda_ver, torch_ver):"
    },
    {
      "line": 171,
      "text": "def install_wheels_from_release(missing_extensions, python_ver, cuda_ver):"
    },
    {
      "line": 196,
      "text": "def install_wheels_from_community(missing_extensions, python_ver, cuda_ver):"
    },
    {
      "line": 230,
      "text": "def install_from_source(missing_extensions):"
    },
    {
      "line": 246,
      "text": "def install_external_dependencies():"
    },
    {
      "line": 334,
      "text": "def main():"
    }
  ]
}

```text

"""
ComfyUI-GaussianAvatar Installer
Installs required Python dependencies and CUDA extensions.
Will attempt to download pre-built wheels from GitHub releases.
"""
import sys
import os
import subprocess
import platform
import urllib.request
import tempfile
import re
import json
import tarfile
import shutil

REPO_OWNER = "cedarconnor"
REPO_NAME = "ComfyUI-GaussianAvatar"
RELEASES_API = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest"

# Community-maintained wheels for ComfyUI 3D projects
COMMUNITY_WHEELS_BASE_URL = "https://github.com/MrForExample/Comfy3D_Pre_Builds/raw/main/_Build_Wheels/"


def get_python_version():
    """Get python version as string like '3.12'"""
    return f"{sys.version_info.major}.{sys.version_info.minor}"


def get_cuda_version():
    """Get CUDA version from PyTorch."""
    try:
        import torch
        if torch.cuda.is_available():
            # Returns string like "12.1"
            version = torch.version.cuda
            if version:
                return version
    except ImportError:
        pass
    
    # Fallback: try nvcc
    try:
        result = subprocess.run(
            ["nvcc", "--version"],
            capture_output=True, text=True, check=False
        )
        match = re.search(r"release (\d+\.\d+)", result.stdout)
        if match:
            return match.group(1)
    except FileNotFoundError:
        pass
    
    return None


def get_torch_version():
    """Get PyTorch version."""
    try:
        import torch
        # Return base version like "2.4.0" (strip +cu121 etc)
        return torch.__version__.split('+')[0]
    except ImportError:
        return None


def install_requirements():
    """Install core python dependencies from requirements.txt"""
    req_file = os.path.join(os.path.dirname(__file__), "requirements.txt")
    if os.path.exists(req_file):
        print(f"[i] Installing dependencies from {req_file}...")
        try:
            subprocess.check_call([sys.executable, "-m", "pip", "install", "-r", req_file])
            print("[OK] Core dependencies installed")
        except subprocess.CalledProcessError as e:
            print(f"[X] Failed to install dependencies: {e}")
            return False
    return True


def check_cuda_extensions():
    """Check if CUDA extensions are importable."""
    missing = []
    try:
        import diff_gaussian_rasterization
    except ImportError:
        missing.append("diff_gaussian_rasterization")
    
    try:
        import simple_knn
    except ImportError:
        missing.append("simple_knn")
        
    return missing


def download_wheel(url, dest_dir):
    """Download a wheel file from URL to destination directory."""
    filename = url.split("/")[-1]
    dest_path = os.path.join(dest_dir, filename)
    print(f"    Downloading {filename}...")
    try:
        # Some GitHub URLs are redirects, urllib handles them
        urllib.request.urlretrieve(url, dest_path)
        return dest_path
    except Exception as e:
        print(f"    [X] Download failed: {e}")
        return None


def get_release_wheels():
    """Get list of wheel URLs from latest GitHub release."""
    try:
        req = urllib.request.Request(
            RELEASES_API,
            headers={"User-Agent": "ComfyUI-GaussianAvatar-Installer"}
        )
        with urllib.request.urlopen(req, timeout=30) as response:
            data = json.loads(response.read().decode())
            return [
                asset["browser_download_url"]
                for asset in data.get("assets", [])
                if asset["name"].endswith(".whl")
            ]
    except Exception as e:
        # Don't print error if it's just 404 (no release yet)
        return []


def find_matching_wheel(wheels, package_name, python_ver, cuda_ver):
    """Find a wheel matching our Python and CUDA version."""
    # Normalize versions for matching
    py_tag = f"cp{python_ver.replace('.', '')}"  # e.g., cp312
    cuda_tag = f"cu{cuda_ver.replace('.', '')}"  # e.g., cu121
    
    for wheel_url in wheels:
        wheel_name = wheel_url.split("/")[-1].lower()
        # Check if this wheel matches our package
        if package_name.replace("_", "-") not in wheel_name.replace("_", "-"):
            continue
        # Check Python version tag
        if py_tag not in wheel_name:
            continue
        # Check CUDA version tag
        if cuda_tag in wheel_name or f"+{cuda_tag}" in wheel_name:
            return wheel_url
    
    return None


def get_community_wheel_urls(package_name, python_ver, cuda_ver, torch_ver):
    """Construct URLs for community-maintained wheels."""
    # Example directory: _Wheels_win_py312_torch2.4.0_cu121
    # Example file: diff_gaussian_rasterization-0.0.0-cp312-cp312-win_amd64.whl
    
    if platform.system() != "Windows":
        return []
    
    py_tag = f"py{python_ver.replace('.', '')}"  # py312
    cuda_tag = f"cu{cuda_ver.replace('.', '')}"  # cu121
    
    # Community repo uses specific torch versions in folder names
    # We'll try the exact torch version first
    folder_name = f"_Wheels_win_{py_tag}_torch{torch_ver}_{cuda_tag}"
    
    base_filename = f"{package_name.replace('-', '_')}-0.0.0-cp{python_ver.replace('.', '')}-cp{python_ver.replace('.', '')}-win_amd64.whl"
    
    return [f"{COMMUNITY_WHEELS_BASE_URL}{folder_name}/{base_filename}"]


def install_wheels_from_release(missing_extensions, python_ver, cuda_ver):
    """Attempt to install pre-built wheels from internal GitHub releases."""
    wheels = get_release_wheels()
    if not wheels:
        return False
    
    installed_count = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        for ext in missing_extensions:
            wheel_url = find_matching_wheel(wheels, ext, python_ver, cuda_ver)
            if not wheel_url:
                continue
            
            wheel_path = download_wheel(wheel_url, tmpdir)
            if wheel_path:
                print(f"    Installing {os.path.basename(wheel_path)}...")
                try:
                    subprocess.check_call([sys.executable, "-m", "pip", "install", wheel_path], stdout=subprocess.DEVNULL)
                    installed_count += 1
                except subprocess.CalledProcessError:
                    pass
    
    return installed_count == len(missing_extensions)


def install_wheels_from_community(missing_extensions, python_ver, cuda_ver):
    """Attempt to install pre-built wheels from community sources."""
    torch_ver = get_torch_version()
    if not torch_ver:
        return False
        
    print(f"[i] Checking community fallback sources (Python {python_ver}, CUDA {cuda_ver}, Torch {torch_ver})...")
    
    installed_count = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        for ext in missing_extensions:
            # Try a few likely torch version folders if exact match fails
            v_parts = torch_ver.split('.')
            search_versions = [torch_ver, f"{v_parts[0]}.{v_parts[1]}.0", "2.4.0", "2.5.1", "2.6.0"]
            
            success = False
            for v in search_versions:
                urls = get_community_wheel_urls(ext, python_ver, cuda_ver, v)
                for wheel_url in urls:
                    wheel_path = download_wheel(wheel_url, tmpdir)
                    if wheel_path:
                        print(f"    Installing {os.path.basename(wheel_path)}...")
                        try:
                            subprocess.check_call([sys.executable, "-m", "pip", "install", wheel_path], stdout=subprocess.DEVNULL)
                            success = True
                            installed_count += 1
                            break
                        except subprocess.CalledProcessError:
                            continue
                if success: break
                
    return installed_count == len(missing_extensions)


def install_from_source(missing_extensions):
    """Fallback: attempt to compile from source."""
    print("[i] Attempting to compile from source (requires build tools)...")
    
    for ext in missing_extensions:
        url = f"git+https://github.com/graphdeco-inria/{ext.replace('_', '-')}.git"
        print(f"    Building {ext}...")
        try:
            subprocess.check_call([sys.executable, "-m", "pip", "install", url], stdout=subprocess.DEVNULL)
        except subprocess.CalledProcessError:
            print(f"    [X] Failed to build {ext}")
            return False
    
    return True


def install_external_dependencies():
    """Clone MICA and EMOCA repositories."""
    print("[i] Checking external research repositories (MICA, EMOCA)...")
    
    root_dir = os.path.dirname(__file__)
    external_dir = os.path.join(root_dir, "external")
    os.makedirs(external_dir, exist_ok=True)
    
    # 1. MICA
    mica_dir = os.path.join(external_dir, "mica")
    if not os.path.exists(mica_dir):
        print("    Cloning MICA...")
        try:
            subprocess.check_call(["git", "clone", "https://github.com/Zielon/MICA.git", mica_dir], stdout=subprocess.DEVNULL)
        except subprocess.CalledProcessError:
            print("    [X] Failed to clone MICA")
    else:
        print("    [OK] MICA already present")
        
    # 2. EMOCA
    emoca_dir = os.path.join(external_dir, "emoca")
    if not os.path.exists(emoca_dir):
        print("    Cloning EMOCA...")
        try:
            subprocess.check_call(["git", "clone", "--recursive", "https://github.com/radekd91/emoca.git", emoca_dir], stdout=subprocess.DEVNULL)
        except subprocess.CalledProcessError:
            print("    [X] Failed to clone EMOCA")
    else:
        print("    [OK] EMOCA already present")

    # 3. Face Parsing (BiSeNet)
    parsing_dir = os.path.join(external_dir, "face_parsing")
    if not os.path.exists(parsing_dir):
        print("    Cloning Face Parsing...")
        try:
             subprocess.check_call(["git", "clone", "https://github.com/zllrunning/face-parsing.PyTorch.git", parsing_dir], stdout=subprocess.DEVNULL)
        except subprocess.CalledProcessError:
            print("    [X] Failed to clone Face Parsing")
    else:
        print("    [OK] Face Parsing already present")

    # 4. Download MICA Model
    # We need to replicate the path logic from model_loading.py:
    # models/mica/mica.pth
    # But wait, MICA zip has 'data/pretrained/mica.tar' structure usually.
    # Let's download to external/mica first then move? Or just download directly?
    # Simple approach: Download mica.tar to a temp dir, extract, and move to ComfyUI/models/mica
    
    # Try to find ComfyUI models dir
    # Assuming we are in custom_nodes/ComfyUI-GaussianAvatar
    comfy_root = os.path.dirname(os.path.dirname(root_dir))
    models_dir = os.path.join(comfy_root, "models", "mica")
    os.makedirs(models_dir, exist_ok=True)
    
    mica_model_path = os.path.join(models_dir, "mica.tar")
    if not os.path.exists(mica_model_path):
        print("    Downloading MICA model from Google Drive...")
        # MICA model is hosted on Google Drive, not GitHub releases
        # File ID: 1bYsI_spptzyuFmfLYqYkcJA6GZWZViNt
        mica_gdrive_id = "1bYsI_spptzyuFmfLYqYkcJA6GZWZViNt"
        try:
            import gdown
            gdown.download(id=mica_gdrive_id, output=mica_model_path, quiet=False)
            if os.path.exists(mica_model_path):
                print("    [OK] MICA model downloaded")
            else:
                print("    [X] MICA download completed but file not found")
        except Exception as e:
            print(f"    [X] Failed to download MICA model: {e}")
            print("    Please download manually from:")
            print("    https://drive.google.com/file/d/1bYsI_spptzyuFmfLYqYkcJA6GZWZViNt/view")
            print(f"    And place at: {mica_model_path}")
    else:
        print("    [OK] MICA model already present")

    # Install Python deps for them
    # MICA needs scipy
    # EMOCA needs face_alignment, gdown, munch
    print("[i] Installing research dependencies...")
    deps = ["scipy", "face_alignment", "gdown", "munch", "ninja", "adabound", "pytorch-lightning", "pandas", "compress-pickle", "hickle", "imgaug", "sk-video", "facenet_pytorch", "wandb"]
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install"] + deps, stdout=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        print("    [X] Failed to install some research dependencies")
        
    return True


def main():
    print("=" * 60)
    print("ComfyUI-GaussianAvatar Installer")
    print("=" * 60)
    
    # Step 1: Install Python requirements
    if not install_requirements():
        return 1
    
    # Step 2: Check for missing CUDA extensions
    missing = check_cuda_extensions()
    if not missing:
        print("[OK] All CUDA extensions already installed!")
        install_external_dependencies()
        print("[OK] Installation complete!")
        return 0
    
    print(f"[!] Missing CUDA extensions: {missing}")
    
    # Step 3: Detect environment
    python_ver = get_python_version()
    cuda_ver = get_cuda_version()
    
    print(f"[i] Detected: Python {python_ver}, CUDA {cuda_ver or 'not found'}")
    
    if not cuda_ver:
        print("[X] Could not detect CUDA version. Please ensure PyTorch with CUDA is installed.")
        return 1
    
    # Step 4: Try to install from internal releases
    print("[i] Searching for pre-built wheels in repository releases...")
    if install_wheels_from_release(missing, python_ver, cuda_ver):
        if not check_cuda_extensions():
            install_external_dependencies()
            print("[OK] Installation complete!")
            return 0
    
    # Step 5: Try community fallbacks
    print("[i] Searching for community-maintained wheels...")
    if install_wheels_from_community(missing, python_ver, cuda_ver):
        if not check_cuda_extensions():
            # SUCCESS
            install_external_dependencies()
            print("[OK] Installation complete!")
            return 0

    # Step 6: Fallback to source compilation
    if install_from_source(missing):
        print("[OK] Installation complete!")
        return 0
    
    # All methods failed
    print("\n[X] Installation failed.")
    print("\nOptions to fix this:")
    print("1. Install Visual Studio Build Tools (Desktop development with C++).")
    print("2. Download compatible wheels manually from:")
    print("   https://github.com/MrForExample/Comfy3D_Pre_Builds")
    
    return 1


if __name__ == "__main__":
    sys.exit(main())


```

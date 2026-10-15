# Facial rendering source scan: facebookresearch/MHR

Repository: https://github.com/facebookresearch/MHR

Pinned source revision: d96fafa33bbf018647c70c3525e91f53e79d2a14

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": ["web-viewer/tools/inspect_blender_rig.py"], "face": ["docs/face-expressions.md", "mhr/face_expression.py", "tests/test_face_expression.py", "tools/mhr_smpl_conversion/assets/mhr_face_mask.ply"]}

## Source file: mhr/face_expression.py

{
  "path": "mhr/face_expression.py",
  "blob_sha": "5981f7176307c57d6ccbe29ec6108688c0443d69",
  "sha256": "3a6ae979e5cdf5a82842f71c2067ec48ed5a2393555f303d12324bf437316055",
  "lines": 81,
  "bytes": 1786,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 10,
    "jaw": 4,
    "mouth": 2,
    "wrinkle": 2,
    "rig": 4,
    "face": 3,
    "lip": 28
  },
  "url": "https://github.com/facebookresearch/MHR/blob/d96fafa33bbf018647c70c3525e91f53e79d2a14/mhr/face_expression.py",
  "function_signature_candidates": []
}

```text

"""Semantic names for MHR facial expression coefficients."""


# Position ``i`` names ``face_expr_coeffs[:, i]`` in MHR v1.x assets.
FACE_EXPRESSION_NAMES: tuple[str, ...] = (
    "browLowerer_L",
    "browLowerer_R",
    "cheekPuff_L",
    "cheekPuff_R",
    "cheekRaiser_L",
    "cheekRaiser_R",
    "cheekSuck_L",
    "cheekSuck_R",
    "chinRaiser_B",
    "chinRaiser_T",
    "dimpler_L",
    "dimpler_R",
    "eyesClosed_L",
    "eyesClosed_R",
    "eyesLookDown_L",
    "eyesLookDown_R",
    "eyesLookLeft_L",
    "eyesLookLeft_R",
    "eyesLookRight_L",
    "eyesLookRight_R",
    "eyesLookUp_L",
    "eyesLookUp_R",
    "innerBrowRaiser_L",
    "innerBrowRaiser_R",
    "jawDrop",
    "jawSidewaysLeft",
    "jawSidewaysRight",
    "jawThrust",
    "lidTightener_L",
    "lidTightener_R",
    "lipCornerDepressor_L",
    "lipCornerDepressor_R",
    "lipCornerPuller_L",
    "lipCornerPuller_R",
    "lipFunneler_LB",
    "lipFunneler_LT",
    "lipFunneler_RB",
    "lipFunneler_RT",
    "lipPressor_L",
    "lipPressor_R",
    "lipPucker_L",
    "lipPucker_R",
    "lipStretcher_L",
    "lipStretcher_R",
    "lipSuck_LB",
    "lipSuck_LT",
    "lipSuck_RB",
    "lipSuck_RT",
    "lipTightener_L",
    "lipTightener_R",
    "lipsToward_LB",
    "lipsToward_LT",
    "lipsToward_RB",
    "lipsToward_RT",
    "lowerLipDepressor_L",
    "lowerLipDepressor_R",
    "mouthLeft",
    "mouthRight",
    "nasolabialFurrow_L",
    "nasolabialFurrow_R",
    "noseWrinkler_L",
    "noseWrinkler_R",
    "nostrilCompressor_L",
    "nostrilCompressor_R",
    "nostrilDilator_L",
    "nostrilDilator_R",
    "outerBrowRaiser_L",
    "outerBrowRaiser_R",
    "upperLidRaiser_L",
    "upperLidRaiser_R",
    "upperLipRaiser_L",
    "upperLipRaiser_R",
)


__all__ = ["FACE_EXPRESSION_NAMES"]


```

## Source file: web-viewer/tools/inspect_blender_rig.py

{
  "path": "web-viewer/tools/inspect_blender_rig.py",
  "blob_sha": "98b94885d0ae371942bf8664b0607bcac76dfd97",
  "sha256": "f7e6d1f76d04f242d8dab2a6d56b10097715ae61b70dbfc820007b0aa305cfd3",
  "lines": 128,
  "bytes": 4013,
  "selection_score": 15,
  "keyword_line_counts": {
    "gum": 2,
    "rig": 2
  },
  "url": "https://github.com/facebookresearch/MHR/blob/d96fafa33bbf018647c70c3525e91f53e79d2a14/web-viewer/tools/inspect_blender_rig.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def _arguments() -> tuple[bool, list[str]]:"
    },
    {
      "line": 21,
      "text": "def _vector(values: object) -> list[float]:"
    },
    {
      "line": 25,
      "text": "def inspect_file("
    },
    {
      "line": 115,
      "text": "def main() -> None:"
    }
  ]
}

```text

"""Inspect a GLB or USD rig after importing it into Blender."""

from __future__ import annotations

import json
from pathlib import Path
import sys

import bpy


def _arguments() -> tuple[bool, list[str]]:
    if "--" not in sys.argv:
        raise SystemExit("Pass one or more model paths after --.")
    values = sys.argv[sys.argv.index("--") + 1 :]
    disable_glb_bone_shapes = "--disable-glb-bone-shapes" in values
    paths = [value for value in values if value != "--disable-glb-bone-shapes"]
    return disable_glb_bone_shapes, paths


def _vector(values: object) -> list[float]:
    return [float(value) for value in values]


def inspect_file(
    path: Path,
    *,
    disable_glb_bone_shapes: bool = False,
) -> dict[str, object]:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    if path.suffix.lower() == ".glb":
        bpy.ops.import_scene.gltf(
            filepath=str(path),
            disable_bone_shape=disable_glb_bone_shapes,
        )
    elif path.suffix.lower() in {".usd", ".usda", ".usdc", ".usdz"}:
        bpy.ops.wm.usd_import(filepath=str(path))
    else:
        raise ValueError(f"Unsupported file: {path}")

    armatures = []
    for armature_object in (
        item for item in bpy.data.objects if item.type == "ARMATURE"
    ):
        bones = []
        for bone in armature_object.data.bones:
            bones.append(
                {
                    "name": bone.name,
                    "parent": bone.parent.name if bone.parent else None,
                    "head": _vector(bone.head_local),
                    "tail": _vector(bone.tail_local),
                    "length": float(bone.length),
                }
            )
        bones.sort(key=lambda item: item["length"], reverse=True)
        custom_shapes = [
            pose_bone.custom_shape.name
            for pose_bone in armature_object.pose.bones
            if pose_bone.custom_shape is not None
        ]
        armatures.append(
            {
                "name": armature_object.name,
                "displayType": armature_object.data.display_type,
                "boneCount": len(bones),
                "customShapeCount": len(custom_shapes),
                "customShapeObjectNames": sorted(set(custom_shapes)),
                "longestBones": bones[:12],
            }
        )

    meshes = []
    for mesh_object in (item for item in bpy.data.objects if item.type == "MESH"):
        meshes.append(
            {
                "name": mesh_object.name,
                "vertexCount": len(mesh_object.data.vertices),
                "modifiers": [
                    {
                        "name": modifier.name,
                        "type": modifier.type,
                        "object": (
                            modifier.object.name
                            if getattr(modifier, "object", None)
                            else None
                        ),
                    }
                    for modifier in mesh_object.modifiers
                ],
                "vertexGroupCount": len(mesh_object.vertex_groups),
            }
        )

    empties = [
        {
            "name": item.name,
            "displayType": item.empty_display_type,
            "displaySize": float(item.empty_display_size),
        }
        for item in bpy.data.objects
        if item.type == "EMPTY"
    ]
    return {
        "file": str(path),
        "disableGlbBoneShapes": disable_glb_bone_shapes,
        "objects": len(bpy.data.objects),
        "armatures": armatures,
        "meshes": meshes,
        "emptyCount": len(empties),
        "empties": empties[:12],
    }


def main() -> None:
    disable_glb_bone_shapes, paths = _arguments()
    reports = [
        inspect_file(
            Path(value).resolve(),
            disable_glb_bone_shapes=disable_glb_bone_shapes,
        )
        for value in paths
    ]
    print("MHR_RIG_INSPECTION=" + json.dumps(reports, separators=(",", ":")))


if __name__ == "__main__":
    main()


```

## Source file: tests/test_face_expression.py

{
  "path": "tests/test_face_expression.py",
  "blob_sha": "a5b59adad0d03ee7dcb4b8c91856d24bda01e392",
  "sha256": "e917a4a23d2e9ce0cef0184bd7a40ea8aff459aa3d7bfda6999b7e761a535ad4",
  "lines": 49,
  "bytes": 1575,
  "selection_score": 13,
  "keyword_line_counts": {
    "blendshape": 2,
    "jaw": 1,
    "rig": 1,
    "face": 9,
    "lip": 16
  },
  "url": "https://github.com/facebookresearch/MHR/blob/d96fafa33bbf018647c70c3525e91f53e79d2a14/tests/test_face_expression.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def test_face_expression_names() -> None:"
    }
  ]
}

```text

# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.


from mhr import FACE_EXPRESSION_NAMES
from mhr.mhr import NUM_FACE_EXPRESSION_BLENDSHAPES


def test_face_expression_names() -> None:
    assert len(FACE_EXPRESSION_NAMES) == NUM_FACE_EXPRESSION_BLENDSHAPES == 72
    assert len(set(FACE_EXPRESSION_NAMES)) == len(FACE_EXPRESSION_NAMES)
    assert FACE_EXPRESSION_NAMES[:4] == (
        "browLowerer_L",
        "browLowerer_R",
        "cheekPuff_L",
        "cheekPuff_R",
    )
    assert FACE_EXPRESSION_NAMES[24] == "jawDrop"
    assert FACE_EXPRESSION_NAMES[40:54] == (
        "lipPucker_L",
        "lipPucker_R",
        "lipStretcher_L",
        "lipStretcher_R",
        "lipSuck_LB",
        "lipSuck_LT",
        "lipSuck_RB",
        "lipSuck_RT",
        "lipTightener_L",
        "lipTightener_R",
        "lipsToward_LB",
        "lipsToward_LT",
        "lipsToward_RB",
        "lipsToward_RT",
    )
    assert FACE_EXPRESSION_NAMES[-2:] == (
        "upperLipRaiser_L",
        "upperLipRaiser_R",
    )


```

## Source file: mhr/io.py

{
  "path": "mhr/io.py",
  "blob_sha": "8325a469edc6c00dd317d8320fc0c29e52fb57dc",
  "sha256": "504281d489677f98709cc004f6452a24f046d468672433bc867cfb9cd8b3ba99",
  "lines": 107,
  "bytes": 3446,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 9,
    "rig": 1
  },
  "url": "https://github.com/facebookresearch/MHR/blob/d96fafa33bbf018647c70c3525e91f53e79d2a14/mhr/io.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def get_default_asset_folder() -> Path:"
    },
    {
      "line": 35,
      "text": "def get_mhr_fbx_path(folder: Path, lod: int) -> str:"
    },
    {
      "line": 42,
      "text": "def get_mhr_model_path(folder: Path) -> str:"
    },
    {
      "line": 49,
      "text": "def get_mhr_blendshapes_path(folder: Path, lod: int) -> str:"
    },
    {
      "line": 56,
      "text": "def get_corrective_activation_path(folder: Path) -> str:"
    },
    {
      "line": 63,
      "text": "def load_pose_dirs_predictor("
    },
    {
      "line": 104,
      "text": "def has_pose_corrective_blendshapes(data: Dict[str, np.ndarray]) -> bool:"
    }
  ]
}

```text

# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.


from pathlib import Path
from typing import Dict

import numpy as np
import torch

from .utils import SparseLinear


POSE_CORRECTIVES_SPARSE_MASK_NAME = "posedirs_sparse_mask"
POSE_CORRECTIVES_COMPONENTS_NAME = "corrective_blendshapes"


def get_default_asset_folder() -> Path:
    """Return the path to the default MHR asset folder."""

    return Path(__file__).parent.parent / "assets"


def get_mhr_fbx_path(folder: Path, lod: int) -> str:
    """Return the path to the MHR fbx file."""

    asset_path = folder / f"lod{lod}.fbx"
    return str(asset_path)


def get_mhr_model_path(folder: Path) -> str:
    """Return the path to the MHR model definition file (same across LODs)."""

    asset_path = folder / "compact_v6_1.model"
    return str(asset_path)


def get_mhr_blendshapes_path(folder: Path, lod: int) -> str:
    """Return the path to the file storing pose-dependent blendshapes."""

    asset_path = folder / f"corrective_blendshapes_lod{lod}.npz"
    return str(asset_path)


def get_corrective_activation_path(folder: Path) -> str:
    """Return the path to the file storing activations for the pose-dependent correctives."""

    asset_path = folder / "corrective_activation.npz"
    return str(asset_path)


def load_pose_dirs_predictor(
    blendshapes_data: Dict[str, np.ndarray],
    corrective_activation_data: Dict[str, np.ndarray],
    load_with_cuda: bool,
) -> torch.nn.Sequential:
    """Extract pose correctives data and build the pose correctives predictor."""

    n_components = blendshapes_data[POSE_CORRECTIVES_COMPONENTS_NAME].shape[0]
    n_verts = blendshapes_data[POSE_CORRECTIVES_COMPONENTS_NAME].shape[1]
    state_dict = {
        "0.sparse_indices": torch.from_numpy(
            corrective_activation_data["0.sparse_indices"]
        ),
        "0.sparse_weight": torch.from_numpy(
            corrective_activation_data["0.sparse_weight"]
        ),
    }
    state_dict["2.weight"] = torch.from_numpy(
        blendshapes_data[POSE_CORRECTIVES_COMPONENTS_NAME].reshape((n_components, -1)).T
    )

    posedirs = torch.nn.Sequential(
        SparseLinear(
            125
            * 6,  # num joints minus the 2 global ones (125) x 6D rotation representation
            125 * 24,  # 24 is a hyperparameter
            torch.from_numpy(corrective_activation_data["posedirs_sparse_mask"]),
            bias=False,
            load_with_cuda=load_with_cuda,
        ),
        torch.nn.ReLU(),
        torch.nn.Linear(125 * 24, n_verts * 3, bias=False),
    )

    posedirs.load_state_dict(state_dict)
    for posedir in posedirs.parameters():
        posedir.requires_grad = False

    return posedirs


def has_pose_corrective_blendshapes(data: Dict[str, np.ndarray]) -> bool:
    """Check if the data contains pose-dependent correctives."""

    return POSE_CORRECTIVES_COMPONENTS_NAME in data


```

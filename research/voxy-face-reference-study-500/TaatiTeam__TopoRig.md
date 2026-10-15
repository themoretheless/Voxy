# Facial rendering source scan: TaatiTeam/TopoRig

Repository: https://github.com/TaatiTeam/TopoRig

Pinned source revision: 4b97e0770eb29bea90235641f62940e6a583bb1c

Discovery topic: facial rig

Description: Official repository of the paper "TopoRig: Topology-Agnostic Facial Rigging via Multi-Source Supervision".

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"eye": ["scripts/animate_glb_eye_aus.py", "scripts/cache_validated_welded_eyelid_landmarks.py", "utils/eye_gaze_surface_anchors.py", "utils/eye_gaze_texture_mask.py", "utils/eye_gaze_uv_warp.py", "utils/eye_geometry.py", "utils/eyelid_landmarks.py", "utils/welded_eyelid_landmarks.py"], "jaw": ["scripts/test_lip_split_jaw_open.py"], "mouth": ["utils/gradio_mouth.py", "utils/mouth_landmark_validation.py"], "rig": ["TopoRig_LICENSE.md", "model/toporig.py", "scripts/aggregate_toporig_physical_evaluations.py", "scripts/evaluate_toporig_checkpoint_physical.py"], "face": ["utils/eye_gaze_surface_anchors.py"], "lip": ["scripts/export_lip_split_to_textured_fbx.py", "scripts/prepare_lip_split.py", "scripts/test_lip_split_jaw_open.py", "utils/lip_split.py"]}

## Source file: utils/eye_gaze_surface_anchors.py

{
  "path": "utils/eye_gaze_surface_anchors.py",
  "blob_sha": "61f94cc07b2f3eead20212017e0877d7992c9826",
  "sha256": "e3e3752a21961132efa1583ffcb49e2715ddac3b86d3744b47d19c021e8092c4",
  "lines": 277,
  "bytes": 11900,
  "selection_score": 30,
  "keyword_line_counts": {
    "eye": 51,
    "rig": 11,
    "face": 32
  },
  "url": "https://github.com/TaatiTeam/TopoRig/blob/4b97e0770eb29bea90235641f62940e6a583bb1c/utils/eye_gaze_surface_anchors.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def eye_capability_name(value: int) -> str:"
    },
    {
      "line": 30,
      "text": "def load_eye_gaze_surface_anchor_payload("
    },
    {
      "line": 211,
      "text": "def select_surface_anchors("
    },
    {
      "line": 251,
      "text": "def interpolate_surface_anchors("
    }
  ]
}

```text

from __future__ import annotations

from pathlib import Path
from typing import Any, Mapping, Sequence

import torch

from utils.eye_geometry import (
    EYE_CAPABILITY_LAYERED_MESH,
    EYE_CAPABILITY_RIGID_MESH,
    EYE_CAPABILITY_TEXTURE_ONLY,
)


EYE_GAZE_SURFACE_ANCHOR_VERSION = 3
SUPPORTED_EYE_GAZE_SURFACE_ANCHOR_VERSIONS = (2, 3)


def eye_capability_name(value: int) -> str:
    names = {
        EYE_CAPABILITY_TEXTURE_ONLY: "texture_only",
        EYE_CAPABILITY_RIGID_MESH: "rigid_mesh",
        EYE_CAPABILITY_LAYERED_MESH: "layered_mesh",
    }
    if int(value) not in names:
        raise ValueError(f"Unknown eye geometry capability code: {value}.")
    return names[int(value)]


def load_eye_gaze_surface_anchor_payload(
    path: Path,
    *,
    vertex_count: int | None = None,
) -> dict[str, torch.Tensor]:
    try:
        payload = torch.load(path, map_location="cpu", weights_only=True)
    except TypeError:
        payload = torch.load(path, map_location="cpu")
    if not isinstance(payload, Mapping):
        raise ValueError(f"Expected eye-gaze anchor cache {path} to be a mapping.")

    version_value = payload.get("version")
    if isinstance(version_value, torch.Tensor):
        version = int(version_value.item())
    else:
        version = int(version_value or -1)
    if version not in SUPPORTED_EYE_GAZE_SURFACE_ANCHOR_VERSIONS:
        raise ValueError(
            f"Eye-gaze anchor cache {path} has version {version}; expected "
            f"one of {SUPPORTED_EYE_GAZE_SURFACE_ANCHOR_VERSIONS}."
        )

    required = (
        "mediapipe_ids",
        "face_vertex_ids",
        "barycentric_weights",
        "neutral_positions",
        "rigid_vertex_ids",
        "roi_vertex_ids",
        "component_id",
    )
    missing = [name for name in required if not isinstance(payload.get(name), torch.Tensor)]
    if missing:
        raise ValueError(f"Eye-gaze anchor cache {path} is missing tensors: {missing}.")

    result = {
        name: value.detach().cpu().contiguous()
        for name, value in payload.items()
        if isinstance(value, torch.Tensor)
    }
    mediapipe_ids = result["mediapipe_ids"].long().flatten()
    face_vertex_ids = result["face_vertex_ids"].long()
    barycentric_weights = result["barycentric_weights"].float()
    neutral_positions = result["neutral_positions"].float()
    rigid_vertex_ids = result["rigid_vertex_ids"].long().flatten()
    roi_vertex_ids = result["roi_vertex_ids"].long().flatten()
    if face_vertex_ids.shape != (mediapipe_ids.numel(), 3):
        raise ValueError("Eye-gaze face_vertex_ids must have shape [L, 3].")
    if barycentric_weights.shape != face_vertex_ids.shape:
        raise ValueError("Eye-gaze barycentric_weights must match face_vertex_ids.")
    if neutral_positions.shape != (mediapipe_ids.numel(), 3):
        raise ValueError("Eye-gaze neutral_positions must have shape [L, 3].")
    if mediapipe_ids.unique().numel() != mediapipe_ids.numel():
        raise ValueError("Eye-gaze mediapipe_ids must be unique.")
    if not torch.isfinite(barycentric_weights).all():
        raise ValueError("Eye-gaze barycentric weights must be finite.")
    if not torch.allclose(
        barycentric_weights.sum(dim=-1),
        torch.ones(mediapipe_ids.numel()),
        atol=1.0e-4,
        rtol=1.0e-4,
    ):
        raise ValueError("Eye-gaze barycentric weights must sum to one.")
    if (barycentric_weights < -1.0e-4).any():
        raise ValueError("Eye-gaze barycentric weights must be non-negative.")
    if rigid_vertex_ids.numel() < 3:
        raise ValueError("Eye-gaze rigid region must contain at least three vertices.")
    if roi_vertex_ids.numel() < 3:
        raise ValueError("Eye-gaze ROI must contain at least three vertices.")
    if vertex_count is not None:
        for name, values in (
            ("face_vertex_ids", face_vertex_ids),
            ("rigid_vertex_ids", rigid_vertex_ids),
            ("roi_vertex_ids", roi_vertex_ids),
        ):
            if values.numel() and (
                int(values.min()) < 0 or int(values.max()) >= int(vertex_count)
            ):
                raise ValueError(
                    f"Eye-gaze {name} contains an ID outside [0, {vertex_count})."
                )

    result["mediapipe_ids"] = mediapipe_ids
    result["face_vertex_ids"] = face_vertex_ids
    result["barycentric_weights"] = barycentric_weights
    result["neutral_positions"] = neutral_positions
    result["rigid_vertex_ids"] = rigid_vertex_ids.unique(sorted=True)
    result["roi_vertex_ids"] = roi_vertex_ids.unique(sorted=True)
    result["component_id"] = result["component_id"].long().reshape(())
    result["cache_version"] = torch.tensor(version, dtype=torch.long)

    if version < 3:
        # Version 2 frequently selected the face as its rigid region. Keep its
        # legacy anchor/loss fields readable, but never advertise it as a
        # physical eye assembly.
        result["eye_capability"] = torch.tensor(
            EYE_CAPABILITY_TEXTURE_ONLY,
            dtype=torch.long,
        )
        result["eye_index"] = torch.tensor(-1, dtype=torch.long)
        result["pivot_component_id"] = torch.tensor(-1, dtype=torch.long)
        result["assembly_component_ids"] = torch.empty(0, dtype=torch.long)
        result["assembly_vertex_ids"] = torch.empty(0, dtype=torch.long)
        result["rotation_center"] = torch.full((3,), float("nan"))
        result["rotation_radius"] = torch.tensor(float("nan"))
        result["fit_confidence"] = torch.tensor(0.0)
        result["selection_score"] = torch.tensor(float("inf"))
        return result

    physical_required = (
        "eye_capability",
        "eye_index",
        "pivot_component_id",
        "assembly_component_ids",
        "assembly_vertex_ids",
        "rotation_center",
        "rotation_radius",
        "fit_confidence",
        "selection_score",
    )
    missing_physical = [
        name for name in physical_required if not isinstance(result.get(name), torch.Tensor)
    ]
    if missing_physical:
        raise ValueError(
            f"Eye-gaze anchor cache {path} is missing v3 tensors: "
            f"{missing_physical}."
        )
    capability = int(result["eye_capability"].item())
    eye_capability_name(capability)
    eye_index = int(result["eye_index"].item())
    if eye_index not in {-1, 0, 1}:
        raise ValueError("Eye-gaze eye_index must be -1, 0 (left), or 1 (right).")
    assembly_component_ids = (
        result["assembly_component_ids"].long().flatten().unique(sorted=True)
    )
    assembly_vertex_ids = result["assembly_vertex_ids"].long().flatten().unique(sorted=True)
    rotation_center = result["rotation_center"].float().flatten()
    rotation_radius = result["rotation_radius"].float().reshape(())
    fit_confidence = result["fit_confidence"].float().reshape(())
    selection_score = result["selection_score"].float().reshape(())
    if rotation_center.shape != (3,):
        raise ValueError("Eye-gaze rotation_center must have shape [3].")
    if not 0.0 <= float(fit_confidence) <= 1.0:
        raise ValueError("Eye-gaze fit_confidence must lie in [0, 1].")
    is_physical = capability in {
        EYE_CAPABILITY_RIGID_MESH,
        EYE_CAPABILITY_LAYERED_MESH,
    }
    if is_physical:
        if int(result["pivot_component_id"]) < 0:
            raise ValueError("A physical eye cache requires a pivot component.")
        if assembly_component_ids.numel() < 1 or assembly_vertex_ids.numel() < 4:
            raise ValueError("A physical eye cache requires a non-empty assembly.")
        if not torch.isfinite(rotation_center).all():
            raise ValueError("A physical eye cache requires a finite rotation center.")
        if not torch.isfinite(rotation_radius) or float(rotation_radius) <= 0.0:
            raise ValueError("A physical eye cache requires a positive rotation radius.")
    elif assembly_component_ids.numel() or assembly_vertex_ids.numel():
        raise ValueError("A texture-only eye cache must not expose a physical assembly.")
    if vertex_count is not None and assembly_vertex_ids.numel() and (
        int(assembly_vertex_ids.min()) < 0
        or int(assembly_vertex_ids.max()) >= int(vertex_count)
    ):
        raise ValueError(
            "Eye-gaze assembly_vertex_ids contains an ID outside "
            f"[0, {vertex_count})."
        )
    result["eye_capability"] = torch.tensor(capability, dtype=torch.long)
    result["eye_index"] = torch.tensor(eye_index, dtype=torch.long)
    result["pivot_component_id"] = result["pivot_component_id"].long().reshape(())
    result["assembly_component_ids"] = assembly_component_ids
    result["assembly_vertex_ids"] = assembly_vertex_ids
    result["rotation_center"] = rotation_center
    result["rotation_radius"] = rotation_radius
    result["fit_confidence"] = fit_confidence
    result["selection_score"] = selection_score
    return result


def select_surface_anchors(
    payload: Mapping[str, Any],
    required_ids: Sequence[int],
    *,
    device: torch.device,
    dtype: torch.dtype,
) -> tuple[torch.Tensor, torch.Tensor] | None:
    mediapipe_ids = payload.get("gaze_mediapipe_ids")
    face_vertex_ids = payload.get("gaze_face_vertex_ids")
    barycentric_weights = payload.get("gaze_barycentric_weights")
    if not all(
        isinstance(value, torch.Tensor)
        for value in (mediapipe_ids, face_vertex_ids, barycentric_weights)
    ):
        return None

    ids = mediapipe_ids.detach().cpu().long().flatten().tolist()
    index_by_id = {int(value): index for index, value in enumerate(ids)}
    if any(int(value) not in index_by_id for value in required_ids):
        return None
    indices = torch.tensor(
        [index_by_id[int(value)] for value in required_ids],
        dtype=torch.long,
    )
    triangles = face_vertex_ids.detach().cpu().long()
    weights = barycentric_weights.detach().cpu().float()
    if triangles.dim() == 3 and triangles.shape[0] == 1:
        triangles = triangles[0]
    if weights.dim() == 3 and weights.shape[0] == 1:
        weights = weights[0]
    if triangles.dim() != 2 or triangles.shape[-1] != 3:
        raise ValueError("gaze_face_vertex_ids must have shape [L, 3].")
    if weights.shape != triangles.shape:
        raise ValueError("gaze_barycentric_weights must match gaze_face_vertex_ids.")
    return (
        triangles.index_select(0, indices).to(device=device),
        weights.index_select(0, indices).to(device=device, dtype=dtype),
    )


def interpolate_surface_anchors(
    vertices: torch.Tensor,
    face_vertex_ids: torch.Tensor,
    barycentric_weights: torch.Tensor,
) -> torch.Tensor:
    if vertices.dim() == 2:
        vertices = vertices.unsqueeze(0)
    if vertices.dim() != 3 or vertices.shape[-1] != 3:
        raise ValueError("vertices must have shape [V, 3] or [B, V, 3].")
    if face_vertex_ids.dim() == 2:
        face_vertex_ids = face_vertex_ids.unsqueeze(0).expand(vertices.shape[0], -1, -1)
    if barycentric_weights.dim() == 2:
        barycentric_weights = barycentric_weights.unsqueeze(0).expand(
            vertices.shape[0], -1, -1
        )
    if face_vertex_ids.shape != barycentric_weights.shape:
        raise ValueError("Surface-anchor faces and weights must have matching shapes.")
    if face_vertex_ids.shape[0] != vertices.shape[0] or face_vertex_ids.shape[-1] != 3:
        raise ValueError("Surface anchors must have shape [B, L, 3].")
    if face_vertex_ids.numel() and (
        int(face_vertex_ids.min()) < 0 or int(face_vertex_ids.max()) >= vertices.shape[1]
    ):
        raise ValueError("Surface-anchor face vertices are outside the mesh range.")

    batch = torch.arange(vertices.shape[0], device=vertices.device).view(-1, 1, 1)
    triangle_vertices = vertices[batch, face_vertex_ids]
    return (triangle_vertices * barycentric_weights.unsqueeze(-1)).sum(dim=2)


```

## Source file: scripts/test_lip_split_jaw_open.py

{
  "path": "scripts/test_lip_split_jaw_open.py",
  "blob_sha": "20f98c8e1509e4de2fb188ab98dcdf40db5a1789",
  "sha256": "5deb1c671d16de9788ca1f2a1250b99c6eab85707545858dafb9162db7875b5d",
  "lines": 3298,
  "bytes": 126337,
  "selection_score": 28,
  "keyword_line_counts": {
    "skin": 1,
    "jaw": 33,
    "mouth": 270,
    "teeth": 22,
    "tooth": 11,
    "gum": 187,
    "tongue": 20,
    "deform": 74,
    "rig": 16,
    "face": 291,
    "lip": 233
  },
  "url": "https://github.com/TaatiTeam/TopoRig/blob/4b97e0770eb29bea90235641f62940e6a583bb1c/scripts/test_lip_split_jaw_open.py",
  "function_signature_candidates": [
    {
      "line": 62,
      "text": "def positive_int(value: str) -> int:"
    },
    {
      "line": 69,
      "text": "def rgb_channel(value: str) -> int:"
    },
    {
      "line": 76,
      "text": "def should_build_proxy_before_split("
    },
    {
      "line": 96,
      "text": "def main() -> None:"
    },
    {
      "line": 1672,
      "text": "def parse_args() -> argparse.Namespace:"
    },
    {
      "line": 2771,
      "text": "def resolve_input_landmarks("
    },
    {
      "line": 2808,
      "text": "def resolve_lip_regions("
    },
    {
      "line": 2858,
      "text": "def resolve_extra_removed_face_ids("
    },
    {
      "line": 2881,
      "text": "def resolve_reference_landmarks(args: argparse.Namespace) -> Optional[dict[int, int]]:"
    },
    {
      "line": 2896,
      "text": "def build_inference_proxy("
    },
    {
      "line": 3002,
      "text": "def smooth_proxy_displacement_transfer("
    },
    {
      "line": 3064,
      "text": "def preprocess_split_mesh_for_inference("
    },
    {
      "line": 3149,
      "text": "def validation_colors_with_cavity("
    },
    {
      "line": 3168,
      "text": "def export_colors_with_cavity("
    },
    {
      "line": 3177,
      "text": "def apply_cavity_color("
    },
    {
      "line": 3188,
      "text": "def fit_requested_oral_assembly("
    },
    {
      "line": 3252,
      "text": "def apply_oral_vertex_colors("
    },
    {
      "line": 3272,
      "text": "def apply_component_vertex_color("
    },
    {
      "line": 3290,
      "text": "def write_json(path: Path, payload: Mapping[str, Any]) -> None:"
    }
  ]
}

```text

#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any, Mapping, Optional

import numpy as np
import torch

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

import demo
import train
from utils.fbx_to_tensor import AU_NAME
from utils.lip_split import (
    LipRegions,
    _mesh_boundary_edges,
    bridge_excess_mouth_corner_gap,
    build_mediapipe_lip_annulus,
    build_mouth_boundary_lining,
    build_mouth_cavity_disk,
    clear_source_faces_under_lip_annulus,
    clip_mesh_to_smooth_mouth_contour,
    detect_jaw_open_aperture_faces,
    detect_stretched_mouth_faces,
    fill_secondary_mouth_boundary_holes,
    lip_regions_from_mediapipe_mapping,
    lip_regions_from_mediapipe_paths,
    load_lip_regions,
    load_vertex_ids_arg,
    mouth_band_face_ids_from_mediapipe,
    orient_front_surface_faces,
    repair_mediapipe_lip_landmarks_to_dominant_component,
    retain_aperture_connected_face_removals,
    remap_lower_lip_landmarks,
    remap_vertex_mapping,
    smooth_mouth_boundary_loops,
    split_lip_connections,
    split_lips_along_mediapipe_seam,
    weld_coincident_vertices,
)
from utils.metahuman_oral import (
    fit_oral_assembly,
    remove_disconnected_head_mouth_components,
    remove_head_faces_behind_outer_lips,
)


JAW_OPEN_AU_ID = next(
    action_unit_id
    for action_unit_id, action_unit_name in AU_NAME.items()
    if action_unit_name == "jawOpen"
)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be a positive integer")
    return parsed


def rgb_channel(value: str) -> int:
    parsed = int(value)
    if not 0 <= parsed <= 255:
        raise argparse.ArgumentTypeError("RGB channels must be in [0, 255]")
    return parsed


def should_build_proxy_before_split(
    source_face_count: int,
    target_face_count: int,
    maximum_source_ratio: float,
) -> bool:
    """Keep seam construction after decimation only for moderate reductions."""

    if source_face_count <= 0 or target_face_count <= 0:
        raise ValueError("Proxy source and target face counts must be positive.")
    if maximum_source_ratio <= 1.0:
        raise ValueError(
            "Proxy-before-split maximum source ratio must be greater than one."
        )
    return (
        source_face_count > target_face_count
        and float(source_face_count) / float(target_face_count)
        <= float(maximum_source_ratio)
    )


def main() -> None:
    pipeline_started = time.monotonic()
    args = parse_args()
    mesh_path = args.mesh.expanduser().resolve()
    checkpoint_path = demo.resolve_checkpoint_path(args.checkpoint)
    output_dir = (
        args.output_dir.expanduser().resolve()
        if args.output_dir is not None
        else mesh_path.parent / f"{mesh_path.stem}_lip_split_jaw_open"
    )
    output_dir.mkdir(parents=True, exist_ok=True)
    prefix = args.prefix or mesh_path.stem

    checkpoint = demo.load_checkpoint(checkpoint_path)
    config = demo.checkpoint_config(checkpoint, args.config)
    device = demo.resolve_device(args.device, config)
    model = demo.load_model(checkpoint, config, device)

    raw_mesh = demo.load_mesh_for_model(mesh_path)
    input_landmarks = resolve_input_landmarks(args, mesh_path)
    weld_report = None
    if args.split_mode in {"mediapipe-seam", "weld-only"} and not args.no_weld_coincident_vertices:
        raw_mesh, weld_report, original_to_welded = weld_coincident_vertices(raw_mesh)
        if input_landmarks is not None:
            input_landmarks = remap_vertex_mapping(
                input_landmarks,
                original_to_welded,
            )
    lip_landmark_topology_repair_report = None
    if (
        args.split_mode == "mediapipe-seam"
        and input_landmarks is not None
        and not args.no_repair_mediapipe_lip_landmarks
    ):
        (
            input_landmarks,
            lip_landmark_topology_repair_report,
        ) = repair_mediapipe_lip_landmarks_to_dominant_component(
            raw_mesh,
            input_landmarks,
            maximum_snap_distance_ratio=(
                args.lip_landmark_maximum_snap_distance_ratio
            ),
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
        )
    proxy_report = None
    proxy_built_before_split = False
    if args.proxy_before_split_maximum_source_ratio <= 1.0:
        raise ValueError(
            "Proxy-before-split maximum source ratio must be greater than one."
        )
    proxy_source_reduction_ratio = (
        float(raw_mesh["faces"].shape[0]) / float(args.proxy_target_faces)
        if args.proxy_target_faces is not None
        else 0.0
    )
    build_proxy_before_split = (
        args.proxy_before_split
        and args.proxy_target_faces is not None
        and should_build_proxy_before_split(
            int(raw_mesh["faces"].shape[0]),
            int(args.proxy_target_faces),
            float(args.proxy_before_split_maximum_source_ratio),
        )
    )
    if build_proxy_before_split:
        if args.split_mode != "mediapipe-seam" or input_landmarks is None:
            raise ValueError(
                "Pre-split proxy construction requires MediaPipe seam splitting."
            )
        if args.proxy_output_resolution != "proxy":
            raise ValueError(
                "Pre-split proxy construction only supports proxy-resolution output."
            )
        (
            raw_mesh,
            input_landmarks,
            _pre_split_dense_to_proxy,
            proxy_report,
        ) = build_inference_proxy(
            raw_mesh,
            input_landmarks,
            target_faces=args.proxy_target_faces,
            aggression=args.proxy_aggression,
        )
        proxy_report["build_order"] = "before-lip-split"
        proxy_built_before_split = True
        print(
            "Pre-split proxy:   "
            f"{proxy_report['original_face_count']} -> "
            f"{proxy_report['proxy_face_count']} faces in "
            f"{proxy_report['build_seconds']:.2f}s",
            flush=True,
        )
    elif (
        args.proxy_before_split
        and args.proxy_target_faces is not None
        and int(raw_mesh["faces"].shape[0]) > args.proxy_target_faces
    ):
        print(
            "Pre-split proxy:   deferred until after lip split "
            f"(source_ratio={proxy_source_reduction_ratio:.2f} > "
            f"{args.proxy_before_split_maximum_source_ratio:.2f})",
            flush=True,
        )
    lip_regions: Optional[LipRegions] = None
    split_lip_regions: Optional[LipRegions] = None
    if args.split_mode == "mediapipe-seam":
        if input_landmarks is None:
            raise ValueError(
                "--split-mode mediapipe-seam requires MediaPipe landmarks."
            )
        split_mesh, split_report = split_lips_along_mediapipe_seam(
            raw_mesh,
            input_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            depth_tolerance=args.seam_depth_tolerance,
            vertical_tolerance=args.seam_vertical_tolerance,
            horizontal_padding=args.seam_horizontal_padding,
            corner_inset_ratio=args.seam_corner_inset_ratio,
            preopen_distance=args.seam_preopen_distance,
        )
    elif args.split_mode == "region":
        lip_regions = resolve_lip_regions(args, raw_mesh, input_landmarks)
        split_mesh, split_report, split_lip_regions = split_lip_connections(
            raw_mesh,
            lip_regions,
            remove_mixed_faces=not args.keep_mixed_faces,
            extra_remove_face_ids=resolve_extra_removed_face_ids(
                args,
                raw_mesh,
                input_landmarks,
            ),
        )
    else:
        split_mesh = raw_mesh
        split_report = None
    if input_landmarks is None:
        split_landmarks = None
    elif split_report is None:
        split_landmarks = input_landmarks
    else:
        split_landmarks = remap_lower_lip_landmarks(input_landmarks, split_report)

    outputs: dict[str, str] = {}
    if not args.no_raw_split_mesh:
        raw_split_path = output_dir / f"{prefix}_raw_lip_split.{args.output_format}"
        demo.export_mesh(
            vertices=split_mesh["vertices"],
            faces=split_mesh["faces"],
            normals=split_mesh["normals"],
            path=raw_split_path,
        )
        outputs["raw_split_mesh"] = str(raw_split_path)

    processed_mesh = preprocess_split_mesh_for_inference(
        split_mesh=split_mesh,
        config=config,
        mesh_path=mesh_path,
        input_convention=args.input_convention,
        input_landmarks=split_landmarks,
        reference_mesh_path=demo.metahuman_sample_mesh(
            args.metahuman_sample_dir.expanduser()
        ),
        reference_landmarks=resolve_reference_landmarks(args),
        reference_input_convention=args.reference_input_convention,
        min_alignment_landmarks=args.min_alignment_landmarks,
        alignment_mode=args.alignment_mode,
        reference_mesh_cache_path=args.reference_processed_cache,
    )
    dense_processed_mesh = processed_mesh
    dense_probe_source_faces = processed_mesh["faces"].clone()
    dense_to_proxy = None
    probe_landmarks = split_landmarks
    if (
        not proxy_built_before_split
        and args.proxy_target_faces is not None
        and int(processed_mesh["faces"].shape[0]) > args.proxy_target_faces
    ):
        if split_landmarks is None:
            raise ValueError("Proxy inference requires MediaPipe landmarks.")
        (
            processed_mesh,
            probe_landmarks,
            dense_to_proxy,
            proxy_report,
        ) = build_inference_proxy(
            processed_mesh,
            split_landmarks,
            target_faces=args.proxy_target_faces,
            aggression=args.proxy_aggression,
        )
        proxy_report["build_order"] = "after-lip-split"
        print(
            "Inference proxy:   "
            f"{proxy_report['original_face_count']} -> "
            f"{proxy_report['proxy_face_count']} faces in "
            f"{proxy_report['build_seconds']:.2f}s",
            flush=True,
        )
    probe_source_faces = processed_mesh["faces"].clone()

    aperture_cut_enabled = (
        args.split_mode == "mediapipe-seam"
        and not args.no_probe_aperture_cut
        and probe_landmarks is not None
    )
    aperture_pass_reports: list[dict[str, Any]] = []
    stretch_pass_reports: list[dict[str, Any]] = []
    connectivity_pass_reports: list[dict[str, Any]] = []
    aperture_total_removed_faces = 0
    stretch_total_detected_faces = 0
    probe_total_removed_faces = 0
    aperture_remaining_report = None
    stretch_remaining_report = None
    dense_connectivity_report = None
    remaining_connectivity_report = None
    probe_predicted_delta = None
    probe_face_set_converged = not aperture_cut_enabled

    if aperture_cut_enabled:
        source_face_count = int(probe_source_faces.shape[0])
        current_keep_faces = torch.ones(source_face_count, dtype=torch.bool)
        safe_streak = torch.zeros(source_face_count, dtype=torch.int16)
        restored_once = torch.zeros(source_face_count, dtype=torch.bool)
        restore_blocked = torch.zeros(source_face_count, dtype=torch.bool)
        for pass_index in range(args.probe_aperture_max_passes):
            current_deformed_vertices, current_predicted_delta = demo.run_model(
                model=model,
                mesh=processed_mesh,
                action_unit_id=JAW_OPEN_AU_ID,
                config=config,
                device=device,
            )
            if probe_predicted_delta is None:
                probe_predicted_delta = current_predicted_delta

            aperture_cut_report = detect_jaw_open_aperture_faces(
                current_deformed_vertices,
                probe_source_faces,
                probe_landmarks,
                horizontal_axis=args.seam_horizontal_axis,
                depth_axis=args.seam_depth_axis,
                vertical_axis=args.seam_vertical_axis,
                aperture_scale=args.probe_aperture_scale,
                overlap_inset_scale=args.probe_aperture_overlap_inset_scale,
                depth_padding=args.probe_aperture_depth_padding,
            )
            pass_report = aperture_cut_report.to_dict()
            pass_report["pass_index"] = pass_index + 1
            stretch_report = detect_stretched_mouth_faces(
                processed_mesh["vertices"],
                current_deformed_vertices,
                probe_source_faces,
                probe_landmarks,
                horizontal_axis=args.seam_horizontal_axis,
                depth_axis=args.seam_depth_axis,
                vertical_axis=args.seam_vertical_axis,
                edge_ratio_threshold=args.probe_stretch_edge_ratio,
                edge_growth_threshold=args.probe_stretch_edge_growth,
                region_scale=args.probe_stretch_region_scale,
                depth_padding=args.probe_stretch_depth_padding,
            )
            connectivity_report = retain_aperture_connected_face_removals(
                probe_source_faces,
                aperture_cut_report.removed_face_ids,
                stretch_report.removed_face_ids,
            )
            removed_face_ids = connectivity_report.removed_face_ids
            unsafe_faces = torch.zeros(source_face_count, dtype=torch.bool)
            if removed_face_ids:
                unsafe_faces[
                    torch.tensor(removed_face_ids, dtype=torch.long)
                ] = True

            removed_but_safe = (
                ~current_keep_faces & ~unsafe_faces & ~restore_blocked
            )
            safe_streak[removed_but_safe] += 1
            safe_streak[~removed_but_safe] = 0

            next_keep_faces = current_keep_faces.clone()
            newly_unsafe = current_keep_faces & unsafe_faces
            restore_blocked[newly_unsafe & restored_once] = True
            next_keep_faces[newly_unsafe] = False
            safe_to_restore = (
                ~current_keep_faces
                & ~unsafe_faces
                & ~restore_blocked
                & (safe_streak >= args.probe_restore_safe_passes)
            )
            next_keep_faces[safe_to_restore] = True
            restored_once[safe_to_restore] = True
            safe_streak[safe_to_restore] = 0

            newly_removed_count = int(
                (current_keep_faces & ~next_keep_faces).sum().item()
            )
            restored_count = int(
                (~current_keep_faces & next_keep_faces).sum().item()
            )
            face_set_matches = torch.equal(next_keep_faces, current_keep_faces)
            pending_restore_count = int(
                (
                    ~next_keep_faces
                    & ~unsafe_faces
                    & ~restore_blocked
                ).sum().item()
            )
            face_set_stable = face_set_matches and pending_restore_count == 0
            pass_report["action"] = (
                "converged" if face_set_stable else "reconcile"
            )
            pass_report["newly_removed_face_count"] = newly_removed_count
            pass_report["restored_face_count"] = restored_count
            pass_report["restore_blocked_face_count"] = int(
                restore_blocked.sum().item()
            )
            pass_report["pending_restore_face_count"] = pending_restore_count
            pass_report["target_face_count"] = int(next_keep_faces.sum().item())
            aperture_pass_reports.append(pass_report)
            stretch_pass_report = stretch_report.to_dict()
            stretch_pass_report["pass_index"] = pass_index + 1
            stretch_pass_report["action"] = pass_report["action"]
            stretch_pass_reports.append(stretch_pass_report)
            connectivity_pass_report = connectivity_report.to_dict()
            connectivity_pass_report["pass_index"] = pass_index + 1
            connectivity_pass_report["action"] = pass_report["action"]
            connectivity_pass_reports.append(connectivity_pass_report)

            if args.proxy_one_shot_probe:
                processed_mesh = dict(processed_mesh)
                processed_mesh["faces"] = probe_source_faces[
                    next_keep_faces
                ].contiguous()
                current_keep_faces = next_keep_faces
                deformed_vertices = current_deformed_vertices
                predicted_delta = current_predicted_delta
                probe_face_set_converged = True
                pass_report["action"] = "one-shot"
                stretch_pass_report["action"] = "one-shot"
                break

            if face_set_stable:
                deformed_vertices = current_deformed_vertices
                predicted_delta = current_predicted_delta
                probe_face_set_converged = True
                break

            processed_mesh = dict(processed_mesh)
            processed_mesh["faces"] = probe_source_faces[next_keep_faces].contiguous()
            current_keep_faces = next_keep_faces
        else:
            # Return a prediction for the final bounded face set even when the
            # reconciliation limit is reached.
            deformed_vertices, predicted_delta = demo.run_model(
                model=model,
                mesh=processed_mesh,
                action_unit_id=JAW_OPEN_AU_ID,
                config=config,
                device=device,
            )

        aperture_total_removed_faces = (
            aperture_pass_reports[-1]["removed_face_count"]
            if aperture_pass_reports
            else 0
        )
        stretch_total_detected_faces = (
            stretch_pass_reports[-1]["removed_face_count"]
            if stretch_pass_reports
            else 0
        )
        probe_total_removed_faces = int((~current_keep_faces).sum().item())
        aperture_remaining_report = detect_jaw_open_aperture_faces(
            deformed_vertices,
            processed_mesh["faces"],
            probe_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            aperture_scale=args.probe_aperture_scale,
            overlap_inset_scale=args.probe_aperture_overlap_inset_scale,
            depth_padding=args.probe_aperture_depth_padding,
        )
        stretch_remaining_report = detect_stretched_mouth_faces(
            processed_mesh["vertices"],
            deformed_vertices,
            processed_mesh["faces"],
            probe_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            edge_ratio_threshold=args.probe_stretch_edge_ratio,
            edge_growth_threshold=args.probe_stretch_edge_growth,
            region_scale=args.probe_stretch_region_scale,
            depth_padding=args.probe_stretch_depth_padding,
        )
        remaining_connectivity_report = retain_aperture_connected_face_removals(
            processed_mesh["faces"],
            aperture_remaining_report.removed_face_ids,
            stretch_remaining_report.removed_face_ids,
        )
    else:
        deformed_vertices, predicted_delta = demo.run_model(
            model=model,
            mesh=processed_mesh,
            action_unit_id=JAW_OPEN_AU_ID,
            config=config,
            device=device,
        )
        probe_predicted_delta = predicted_delta

    if dense_to_proxy is not None and args.proxy_output_resolution == "dense":
        transfer_start = time.monotonic()
        proxy_predicted_delta = predicted_delta
        predicted_delta = proxy_predicted_delta.index_select(0, dense_to_proxy)
        if args.proxy_transfer_smoothing_iterations > 0:
            predicted_delta = smooth_proxy_displacement_transfer(
                predicted_delta,
                dense_processed_mesh["faces"],
                pinned_vertex_ids=split_landmarks.values(),
                iterations=args.proxy_transfer_smoothing_iterations,
                blend=args.proxy_transfer_smoothing_blend,
                device=device,
            )
        deformed_vertices = dense_processed_mesh["vertices"] + predicted_delta
        probe_predicted_delta = predicted_delta
        processed_mesh = dict(dense_processed_mesh)
        probe_source_faces = dense_probe_source_faces

        dense_aperture_report = detect_jaw_open_aperture_faces(
            deformed_vertices,
            probe_source_faces,
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            aperture_scale=args.probe_aperture_scale,
            overlap_inset_scale=args.probe_aperture_overlap_inset_scale,
            depth_padding=args.probe_aperture_depth_padding,
        )
        dense_stretch_report = detect_stretched_mouth_faces(
            processed_mesh["vertices"],
            deformed_vertices,
            probe_source_faces,
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            edge_ratio_threshold=args.probe_stretch_edge_ratio,
            edge_growth_threshold=args.probe_stretch_edge_growth,
            region_scale=args.probe_stretch_region_scale,
            depth_padding=args.probe_stretch_depth_padding,
        )
        dense_connectivity_report = retain_aperture_connected_face_removals(
            probe_source_faces,
            dense_aperture_report.removed_face_ids,
            dense_stretch_report.removed_face_ids,
        )
        dense_removed_ids = dense_connectivity_report.removed_face_ids
        dense_keep = torch.ones(probe_source_faces.shape[0], dtype=torch.bool)
        if dense_removed_ids:
            dense_keep[torch.tensor(dense_removed_ids, dtype=torch.long)] = False
        processed_mesh["faces"] = probe_source_faces[dense_keep].contiguous()
        aperture_total_removed_faces = len(dense_aperture_report.removed_face_ids)
        stretch_total_detected_faces = len(dense_stretch_report.removed_face_ids)
        probe_total_removed_faces = len(dense_removed_ids)
        aperture_remaining_report = detect_jaw_open_aperture_faces(
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            aperture_scale=args.probe_aperture_scale,
            overlap_inset_scale=args.probe_aperture_overlap_inset_scale,
            depth_padding=args.probe_aperture_depth_padding,
        )
        stretch_remaining_report = detect_stretched_mouth_faces(
            processed_mesh["vertices"],
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            edge_ratio_threshold=args.probe_stretch_edge_ratio,
            edge_growth_threshold=args.probe_stretch_edge_growth,
            region_scale=args.probe_stretch_region_scale,
            depth_padding=args.probe_stretch_depth_padding,
        )
        remaining_connectivity_report = retain_aperture_connected_face_removals(
            processed_mesh["faces"],
            aperture_remaining_report.removed_face_ids,
            stretch_remaining_report.removed_face_ids,
        )
        probe_face_set_converged = (
            remaining_connectivity_report.removed_face_count == 0
        )
        proxy_report["transfer_seconds"] = time.monotonic() - transfer_start
        proxy_report["dense_removed_face_count"] = probe_total_removed_faces
        proxy_report["transfer_mode"] = "collapse-cluster-displacement"
        proxy_report["transfer_smoothing_iterations"] = int(
            args.proxy_transfer_smoothing_iterations
        )
        proxy_report["transfer_smoothing_blend"] = float(
            args.proxy_transfer_smoothing_blend
        )
    elif dense_to_proxy is not None:
        split_landmarks = probe_landmarks
        proxy_report["transfer_mode"] = "proxy-resolution-output"
        proxy_report["transfer_seconds"] = 0.0
        proxy_report["dense_removed_face_count"] = 0
        proxy_report["transfer_smoothing_iterations"] = 0
        proxy_report["transfer_smoothing_blend"] = 0.0

    if probe_predicted_delta is None:
        raise RuntimeError("The jawOpen probe did not produce a prediction.")

    probe_source_boundary_edges = (
        _mesh_boundary_edges(probe_source_faces)
        if args.split_mode == "mediapipe-seam"
        else None
    )

    mouth_contour_clip_report = None
    if args.smooth_contour_clip:
        if args.split_mode != "mediapipe-seam" or split_landmarks is None:
            raise ValueError(
                "Smooth contour clipping requires MediaPipe seam splitting."
            )
        print("Smooth contour clip: reconstructing boundary faces...", flush=True)
        (
            clipped_neutral,
            deformed_vertices,
            clipped_faces,
            mouth_contour_clip_report,
        ) = clip_mesh_to_smooth_mouth_contour(
            processed_mesh["vertices"],
            deformed_vertices,
            probe_source_faces,
            processed_mesh["faces"],
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            contour_scale=args.smooth_contour_scale,
            depth_padding=args.smooth_contour_depth_padding,
            edge_ratio_threshold=args.probe_stretch_edge_ratio,
            edge_growth_threshold=args.probe_stretch_edge_growth,
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = clipped_neutral
        processed_mesh["faces"] = clipped_faces
        print(
            "Smooth contour clip: "
            f"restored={mouth_contour_clip_report.restored_boundary_face_count} "
            f"clipped={mouth_contour_clip_report.clipped_face_count} "
            f"added_vertices={mouth_contour_clip_report.added_vertex_count}",
            flush=True,
        )

    mouth_boundary_smooth_report = None
    mouth_hole_fill_reports = []
    mouth_hole_fill_report = None
    if args.split_mode == "mediapipe-seam" and not args.no_fill_secondary_mouth_holes:
        if split_landmarks is None:
            raise ValueError("Mouth hole filling requires MediaPipe landmarks.")
        (
            filled_neutral,
            deformed_vertices,
            filled_faces,
            mouth_hole_fill_report,
        ) = fill_secondary_mouth_boundary_holes(
            processed_mesh["vertices"],
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            reference_faces=probe_source_faces,
            reference_boundary_edges=probe_source_boundary_edges,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            maximum_area_ratio=args.mouth_hole_max_area_ratio,
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = filled_neutral
        processed_mesh["faces"] = filled_faces
        mouth_hole_fill_reports.append(mouth_hole_fill_report)

    if args.split_mode == "mediapipe-seam" and not args.no_smooth_mouth_boundary:
        if split_landmarks is None:
            raise ValueError("Mouth boundary smoothing requires MediaPipe landmarks.")
        smoothed_neutral, deformed_vertices, mouth_boundary_smooth_report = (
            smooth_mouth_boundary_loops(
                processed_mesh["vertices"],
                deformed_vertices,
                processed_mesh["faces"],
                split_landmarks,
                reference_faces=probe_source_faces,
                reference_boundary_edges=probe_source_boundary_edges,
                horizontal_axis=args.seam_horizontal_axis,
                depth_axis=args.seam_depth_axis,
                vertical_axis=args.seam_vertical_axis,
                maximum_displacement=args.mouth_boundary_max_displacement,
                blend_rings=args.mouth_boundary_blend_rings,
            )
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = smoothed_neutral

    if (
        mouth_hole_fill_report is not None
        and mouth_hole_fill_report.rejected_hole_cycle_count > 0
    ):
        (
            filled_neutral,
            deformed_vertices,
            filled_faces,
            mouth_hole_fill_report,
        ) = fill_secondary_mouth_boundary_holes(
            processed_mesh["vertices"],
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            reference_faces=probe_source_faces,
            reference_boundary_edges=probe_source_boundary_edges,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            maximum_area_ratio=args.mouth_hole_max_area_ratio,
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = filled_neutral
        processed_mesh["faces"] = filled_faces
        mouth_hole_fill_reports.append(mouth_hole_fill_report)

    mouth_corner_bridge_report = None
    if args.split_mode == "mediapipe-seam" and not args.no_repair_mouth_corners:
        if split_landmarks is None:
            raise ValueError("Mouth corner repair requires MediaPipe landmarks.")
        (
            repaired_neutral,
            deformed_vertices,
            repaired_faces,
            mouth_corner_bridge_report,
        ) = bridge_excess_mouth_corner_gap(
            processed_mesh["vertices"],
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            reference_faces=probe_source_faces,
            reference_boundary_edges=probe_source_boundary_edges,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            corner_inset_ratio=args.mouth_corner_inset_ratio,
            minimum_lateral_overshoot_ratio=(
                args.mouth_corner_minimum_overshoot_ratio
            ),
            maximum_repaired_sides=args.mouth_corner_maximum_repaired_sides,
            chain_padding_vertices=args.mouth_corner_chain_padding_vertices,
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = repaired_neutral
        processed_mesh["faces"] = repaired_faces

    post_bridge_smooth_report = None
    if args.post_bridge_smoothing:
        if args.split_mode != "mediapipe-seam" or split_landmarks is None:
            raise ValueError(
                "Post-bridge smoothing requires MediaPipe seam splitting and landmarks."
            )
        weighted_unbridged_smoothing = (
            mouth_corner_bridge_report is None
            or mouth_corner_bridge_report.added_face_count == 0
        )
        smoothed_neutral, deformed_vertices, post_bridge_smooth_report = (
            smooth_mouth_boundary_loops(
                processed_mesh["vertices"],
                deformed_vertices,
                processed_mesh["faces"],
                split_landmarks,
                reference_faces=probe_source_faces,
                reference_boundary_edges=probe_source_boundary_edges,
                horizontal_axis=args.seam_horizontal_axis,
                depth_axis=args.seam_depth_axis,
                vertical_axis=args.seam_vertical_axis,
                maximum_displacement=args.post_bridge_max_displacement,
                blend_rings=args.post_bridge_blend_rings,
                smoothing_cycles=args.post_bridge_smoothing_cycles,
                preserve_boundary_junctions=weighted_unbridged_smoothing,
                arc_length_weighted=weighted_unbridged_smoothing,
            )
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = smoothed_neutral

    oral_fit = None
    oral_cleanup_maximum_depth = None
    if args.metahuman_oral_asset is not None:
        if split_landmarks is None:
            raise ValueError("Oral assembly fitting requires mouth landmarks.")
        oral_fit = fit_requested_oral_assembly(
            args,
            processed_mesh["vertices"],
            deformed_vertices,
            split_landmarks,
        )
        oral_cleanup_maximum_depth = float(
            oral_fit[1][:, args.seam_depth_axis].amax().item()
        )
    oral_head_aperture_report = None
    oral_head_component_report = None
    post_cleanup_smooth_report = None
    if (
        args.metahuman_oral_asset is not None
        and not args.no_oral_head_aperture_cleanup
    ):
        if split_landmarks is None:
            raise ValueError("Oral head-aperture cleanup requires mouth landmarks.")
        cleaned_faces, oral_head_aperture_report = (
            remove_head_faces_behind_outer_lips(
                deformed_vertices,
                processed_mesh["faces"],
                split_landmarks,
                horizontal_axis=args.seam_horizontal_axis,
                depth_axis=args.seam_depth_axis,
                vertical_axis=args.seam_vertical_axis,
                scale=args.oral_head_aperture_scale,
                depth_padding_ratio=(
                    args.oral_head_depth_padding_ratio
                ),
                maximum_depth=oral_cleanup_maximum_depth,
                boundary_distance_ratio=(
                    args.oral_head_boundary_distance_ratio
                ),
                boundary_blend_rings=args.oral_head_boundary_blend_rings,
                maximum_protected_edge_ratio=(
                    args.oral_head_maximum_protected_edge_ratio
                ),
                protect_lower_lip=(
                    not args.no_oral_head_lower_lip_protection
                ),
                lower_lip_depth_tolerance_ratio=(
                    args.oral_head_lower_lip_depth_tolerance_ratio
                ),
            )
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["faces"] = cleaned_faces

        cleaned_faces, oral_head_component_report = (
            remove_disconnected_head_mouth_components(
                deformed_vertices,
                processed_mesh["faces"],
                torch.tensor(oral_head_aperture_report.aperture_polygon),
                mapping=split_landmarks,
                depth_axis=args.seam_depth_axis,
                maximum_depth=oral_cleanup_maximum_depth,
                depth_padding_ratio=args.oral_head_depth_padding_ratio,
                horizontal_axis=args.seam_horizontal_axis,
                vertical_axis=args.seam_vertical_axis,
                margin_ratio=args.oral_head_component_margin_ratio,
                maximum_component_faces=args.oral_head_component_maximum_faces,
            )
        )
        processed_mesh["faces"] = cleaned_faces

        if args.post_cleanup_mouth_boundary_smoothing:
            smoothed_neutral, deformed_vertices, post_cleanup_smooth_report = (
                smooth_mouth_boundary_loops(
                    processed_mesh["vertices"],
                    deformed_vertices,
                    processed_mesh["faces"],
                    split_landmarks,
                    reference_faces=probe_source_faces,
                    reference_boundary_edges=probe_source_boundary_edges,
                    horizontal_axis=args.seam_horizontal_axis,
                    depth_axis=args.seam_depth_axis,
                    vertical_axis=args.seam_vertical_axis,
                    maximum_displacement=(
                        args.post_cleanup_boundary_max_displacement
                    ),
                    blend_rings=args.post_cleanup_boundary_blend_rings,
                    smoothing_cycles=args.post_cleanup_boundary_smoothing_cycles,
                    preserve_boundary_junctions=True,
                    arc_length_weighted=True,
                    minimum_component_span=(
                        args.post_cleanup_boundary_minimum_span
                    ),
                    minimum_component_vertices=(
                        args.post_cleanup_boundary_minimum_vertices
                    ),
                )
            )
            processed_mesh = dict(processed_mesh)
            processed_mesh["vertices"] = smoothed_neutral

    lip_annulus_repair_report = None
    lip_annulus_head_cleanup_report = None
    lip_annulus_vertex_count = 0
    if args.reconstruct_lip_annulus:
        if args.split_mode != "mediapipe-seam" or split_landmarks is None:
            raise ValueError(
                "Lip annulus reconstruction requires MediaPipe seam splitting."
            )
        (
            neutral_lip_annulus,
            deformed_lip_annulus,
            lip_annulus_faces,
            lip_annulus_repair_report,
        ) = build_mediapipe_lip_annulus(
            processed_mesh["vertices"],
            deformed_vertices,
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            segments=args.lip_annulus_segments,
            radial_rings=args.lip_annulus_radial_rings,
            outer_scale=args.lip_annulus_outer_scale,
            thickness_ratio=args.lip_annulus_thickness_ratio,
            side_thickness_factor=args.lip_annulus_side_thickness_factor,
            maximum_radial_inset_fraction=(
                args.lip_annulus_maximum_radial_inset_fraction
            ),
            inner_recess_depth=args.lip_annulus_inner_recess_depth,
            front_offset=args.lip_annulus_front_offset,
            bulge_depth=args.lip_annulus_bulge_depth,
            upper_smoothing_window=args.lip_annulus_upper_smoothing_window,
            lower_smoothing_window=args.lip_annulus_lower_smoothing_window,
        )
        if args.clear_head_under_lip_annulus:
            cleaned_faces, lip_annulus_head_cleanup_report = (
                clear_source_faces_under_lip_annulus(
                    deformed_vertices,
                    processed_mesh["faces"],
                    deformed_lip_annulus,
                    segments=args.lip_annulus_segments,
                    horizontal_axis=args.seam_horizontal_axis,
                    depth_axis=args.seam_depth_axis,
                    vertical_axis=args.seam_vertical_axis,
                    depth_padding_ratio=(
                        args.lip_annulus_cleanup_depth_padding_ratio
                    ),
                )
            )
            processed_mesh = dict(processed_mesh)
            processed_mesh["faces"] = cleaned_faces
        lip_annulus_vertex_count = int(neutral_lip_annulus.shape[0])
        annulus_offset = int(processed_mesh["vertices"].shape[0])
        processed_mesh = dict(processed_mesh)
        processed_mesh["vertices"] = torch.cat(
            (processed_mesh["vertices"], neutral_lip_annulus),
            dim=0,
        ).contiguous()
        deformed_vertices = torch.cat(
            (deformed_vertices, deformed_lip_annulus),
            dim=0,
        ).contiguous()
        processed_mesh["faces"] = torch.cat(
            (
                processed_mesh["faces"],
                lip_annulus_faces + annulus_offset,
            ),
            dim=0,
        ).contiguous()
        print(
            "Lip annulus:       "
            f"vertices={lip_annulus_repair_report.vertex_count} "
            f"faces={lip_annulus_repair_report.face_count} "
            f"thickness_ratio={lip_annulus_repair_report.thickness_ratio:.3f} "
            f"capped_segments="
            f"{lip_annulus_repair_report.capped_deformed_segment_count} "
            f"cleared_head_faces="
            f"{lip_annulus_head_cleanup_report.removed_face_count if lip_annulus_head_cleanup_report is not None else 0}",
            flush=True,
        )

    front_surface_orientation_report = None
    if args.orient_front_surface_faces:
        if split_landmarks is None:
            raise ValueError(
                "Front-surface orientation requires MediaPipe landmarks."
            )
        oriented_faces, front_surface_orientation_report = (
            orient_front_surface_faces(
                processed_mesh["vertices"],
                deformed_vertices,
                processed_mesh["faces"],
                split_landmarks,
                depth_axis=args.seam_depth_axis,
                region_depth_ratio=args.front_surface_depth_ratio,
                normal_tolerance=args.front_surface_normal_tolerance,
            )
        )
        processed_mesh = dict(processed_mesh)
        processed_mesh["faces"] = oriented_faces
        print(
            "Face orientation:  "
            f"eligible={front_surface_orientation_report.eligible_face_count} "
            f"flipped={front_surface_orientation_report.flipped_face_count}",
            flush=True,
        )

    head_neutral_normals = demo.recompute_vertex_normals(
        processed_mesh["vertices"],
        processed_mesh["faces"],
    )
    head_deformed_normals = demo.recompute_vertex_normals(
        deformed_vertices,
        processed_mesh["faces"],
    )
    mouth_cavity_report = None
    neutral_cavity_report = None
    cavity_vertex_count = 0
    oral_assembly_report = None
    oral_vertex_count = 0
    oral_vertex_colors = None
    mouth_boundary_lining_report = None
    lining_vertex_count = 0
    vertex_offset = int(processed_mesh["vertices"].shape[0])
    output_face_parts = [processed_mesh["faces"]]
    neutral_vertex_parts = [processed_mesh["vertices"]]
    deformed_vertex_parts = [deformed_vertices]
    neutral_normal_parts = [head_neutral_normals]
    deformed_normal_parts = [head_deformed_normals]

    if oral_fit is not None:
        (
            neutral_oral_vertices,
            deformed_oral_vertices,
            oral_faces,
            oral_vertex_colors,
            oral_assembly_report,
        ) = oral_fit
        neutral_oral_normals = demo.recompute_vertex_normals(
            neutral_oral_vertices,
            oral_faces,
        )
        deformed_oral_normals = demo.recompute_vertex_normals(
            deformed_oral_vertices,
            oral_faces,
        )
        oral_vertex_count = int(neutral_oral_vertices.shape[0])
        output_face_parts.append(oral_faces + vertex_offset)
        neutral_vertex_parts.append(neutral_oral_vertices)
        deformed_vertex_parts.append(deformed_oral_vertices)
        neutral_normal_parts.append(neutral_oral_normals)
        deformed_normal_parts.append(deformed_oral_normals)
        vertex_offset += oral_vertex_count

    full_metahuman_oral_shell = (
        oral_assembly_report is not None
        and oral_assembly_report.geometry_mode == "full_shell"
    )
    add_mouth_boundary_lining = (
        args.mouth_boundary_lining
        or (
            args.metahuman_oral_asset is not None
            and not full_metahuman_oral_shell
        )
    ) and not args.no_mouth_boundary_lining
    if add_mouth_boundary_lining:
        if split_landmarks is None:
            raise ValueError("Mouth boundary lining requires MediaPipe landmarks.")
        (
            neutral_lining_vertices,
            lining_faces,
            neutral_lining_normals,
            neutral_lining_report,
        ) = build_mouth_boundary_lining(
            processed_mesh["vertices"],
            processed_mesh["faces"],
            split_landmarks,
            reference_vertices=deformed_vertices,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            depth=args.mouth_boundary_lining_depth,
            rim_depth=args.mouth_boundary_lining_rim_depth,
            side_fraction=args.mouth_boundary_lining_side_fraction,
            inner_scale=args.mouth_boundary_lining_inner_scale,
            radial_rings=args.mouth_boundary_lining_radial_rings,
            smoothing_iterations=args.mouth_boundary_lining_smoothing_iterations,
            analytic_segments=args.mouth_boundary_lining_segments,
            opening_scale=args.mouth_boundary_lining_opening_scale,
            shape_exponent=args.mouth_boundary_lining_shape_exponent,
            cap_back=not args.no_mouth_boundary_lining_cap,
            maximum_cycles=args.mouth_boundary_lining_maximum_cycles,
        )
        (
            deformed_lining_vertices,
            deformed_lining_faces,
            deformed_lining_normals,
            mouth_boundary_lining_report,
        ) = build_mouth_boundary_lining(
            deformed_vertices,
            processed_mesh["faces"],
            split_landmarks,
            reference_vertices=deformed_vertices,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            depth=args.mouth_boundary_lining_depth,
            rim_depth=args.mouth_boundary_lining_rim_depth,
            side_fraction=args.mouth_boundary_lining_side_fraction,
            inner_scale=args.mouth_boundary_lining_inner_scale,
            radial_rings=args.mouth_boundary_lining_radial_rings,
            smoothing_iterations=args.mouth_boundary_lining_smoothing_iterations,
            analytic_segments=args.mouth_boundary_lining_segments,
            opening_scale=args.mouth_boundary_lining_opening_scale,
            shape_exponent=args.mouth_boundary_lining_shape_exponent,
            cap_back=not args.no_mouth_boundary_lining_cap,
            maximum_cycles=args.mouth_boundary_lining_maximum_cycles,
        )
        if not torch.equal(lining_faces, deformed_lining_faces):
            raise RuntimeError("Neutral and jawOpen mouth linings have mismatched topology.")
        if neutral_lining_report.boundary_vertex_count != mouth_boundary_lining_report.boundary_vertex_count:
            raise RuntimeError("Neutral and jawOpen mouth linings use different boundaries.")
        lining_vertex_count = int(neutral_lining_vertices.shape[0])
        output_face_parts.append(lining_faces + vertex_offset)
        neutral_vertex_parts.append(neutral_lining_vertices)
        deformed_vertex_parts.append(deformed_lining_vertices)
        neutral_normal_parts.append(neutral_lining_normals)
        deformed_normal_parts.append(deformed_lining_normals)
        vertex_offset += lining_vertex_count

    capped_mouth_compartment = (
        add_mouth_boundary_lining and not args.no_mouth_boundary_lining_cap
    )
    add_mouth_cavity = (
        args.mouth_cavity
        or (
            args.metahuman_oral_asset is not None
            and not capped_mouth_compartment
            and not full_metahuman_oral_shell
        )
    ) and not args.no_mouth_cavity
    if add_mouth_cavity:
        if split_landmarks is None:
            raise ValueError("Mouth cavity generation requires MediaPipe landmarks.")
        (
            neutral_cavity_vertices,
            cavity_faces,
            neutral_cavity_normals,
            neutral_cavity_report,
        ) = build_mouth_cavity_disk(
            processed_mesh["vertices"],
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            scale=args.mouth_cavity_scale,
            shape_exponent=args.mouth_cavity_shape_exponent,
            depth_offset=args.mouth_cavity_depth_offset,
            segments=args.mouth_cavity_segments,
            radial_rings=args.mouth_cavity_radial_rings,
            curvature_depth=args.mouth_cavity_curvature_depth,
            inner_radius_ratio=args.mouth_cavity_inner_radius_ratio,
            side_wall_min_horizontal_ratio=(
                args.mouth_cavity_side_wall_min_horizontal_ratio
            ),
        )
        (
            deformed_cavity_vertices,
            deformed_cavity_faces,
            deformed_cavity_normals,
            mouth_cavity_report,
        ) = build_mouth_cavity_disk(
            deformed_vertices,
            split_landmarks,
            horizontal_axis=args.seam_horizontal_axis,
            depth_axis=args.seam_depth_axis,
            vertical_axis=args.seam_vertical_axis,
            scale=args.mouth_cavity_scale,
            shape_exponent=args.mouth_cavity_shape_exponent,
            depth_offset=args.mouth_cavity_depth_offset,
            segments=args.mouth_cavity_segments,
            radial_rings=args.mouth_cavity_radial_rings,
            curvature_depth=args.mouth_cavity_curvature_depth,
            inner_radius_ratio=args.mouth_cavity_inner_radius_ratio,
            side_wall_min_horizontal_ratio=(
                args.mouth_cavity_side_wall_min_horizontal_ratio
            ),
        )
        if not torch.equal(cavity_faces, deformed_cavity_faces):
            raise RuntimeError("Neutral and jawOpen mouth cavities have mismatched topology.")
        cavity_vertex_count = int(neutral_cavity_vertices.shape[0])
        output_face_parts.append(cavity_faces + vertex_offset)
        neutral_vertex_parts.append(neutral_cavity_vertices)
        deformed_vertex_parts.append(deformed_cavity_vertices)
        neutral_normal_parts.append(neutral_cavity_normals)
        deformed_normal_parts.append(deformed_cavity_normals)

    output_faces = torch.cat(output_face_parts, dim=0)
    render_neutral_vertices = torch.cat(neutral_vertex_parts, dim=0)
    render_deformed_vertices = torch.cat(deformed_vertex_parts, dim=0)
    render_neutral_normals = torch.cat(neutral_normal_parts, dim=0)
    render_deformed_normals = torch.cat(deformed_normal_parts, dim=0)

    cavity_rgb = torch.tensor(args.mouth_cavity_color, dtype=torch.float32) / 255.0
    lining_rgb = torch.tensor(
        args.mouth_boundary_lining_color,
        dtype=torch.float32,
    ) / 255.0
    render_colors = validation_colors_with_cavity(
        render_neutral_vertices,
        render_neutral_normals,
        config,
        cavity_vertex_count,
        cavity_rgb,
    )
    render_colors = apply_oral_vertex_colors(
        render_colors,
        oral_vertex_count,
        oral_vertex_colors,
        trailing_vertex_count=lining_vertex_count + cavity_vertex_count,
    )
    render_colors = apply_component_vertex_color(
        render_colors,
        lining_vertex_count,
        lining_rgb,
        trailing_vertex_count=cavity_vertex_count,
    )

    neutral_path = output_dir / f"{prefix}_canonical_lip_split_neutral.{args.output_format}"
    neutral_vertices = demo.output_vertices_for_convention(
        render_neutral_vertices,
        args.output_convention,
    )
    neutral_normals = demo.output_vertices_for_convention(
        render_neutral_normals,
        args.output_convention,
    )
    demo.export_mesh(
        vertices=neutral_vertices,
        faces=output_faces,
        normals=neutral_normals,
        path=neutral_path,
        vertex_colors=apply_component_vertex_color(
            apply_oral_vertex_colors(
                export_colors_with_cavity(
                    neutral_vertices,
                    cavity_vertex_count,
                    cavity_rgb,
                ),
                oral_vertex_count,
                oral_vertex_colors,
                trailing_vertex_count=lining_vertex_count + cavity_vertex_count,
            ),
            lining_vertex_count,
            lining_rgb,
            trailing_vertex_count=cavity_vertex_count,
        ),
    )
    outputs["canonical_lip_split_neutral_mesh"] = str(neutral_path)
    if split_landmarks is not None:
        canonical_landmark_path = output_dir / (
            f"{prefix}_canonical_lip_split_landmarks.json"
        )
        write_json(
            canonical_landmark_path,
            {
                str(int(mediapipe_id)): int(vertex_id)
                for mediapipe_id, vertex_id in sorted(split_landmarks.items())
            },
        )
        outputs["canonical_lip_split_landmarks"] = str(
            canonical_landmark_path
        )

    image = demo.render_output_image(
        neutral_vertices=render_neutral_vertices,
        deformed_vertices=render_deformed_vertices,
        normals=render_neutral_normals,
        faces=output_faces,
        action_unit_id=JAW_OPEN_AU_ID,
        config=config,
        device=device,
        vertex_colors=render_colors,
    )
    image_path = output_dir / f"{prefix}_jawOpen_lip_split.jpg"
    image.save(image_path, quality=95)
    outputs["jaw_open_image"] = str(image_path)

    jaw_open_path = output_dir / f"{prefix}_jawOpen_lip_split.{args.output_format}"
    export_vertices = demo.output_vertices_for_convention(
        render_deformed_vertices,
        args.output_convention,
    )
    export_normals = demo.output_vertices_for_convention(
        render_deformed_normals,
        args.output_convention,
    )
    demo.export_mesh(
        vertices=export_vertices,
        faces=output_faces,
        normals=export_normals,
        path=jaw_open_path,
        vertex_colors=apply_component_vertex_color(
            apply_oral_vertex_colors(
                export_colors_with_cavity(
                    export_vertices,
                    cavity_vertex_count,
                    cavity_rgb,
                ),
                oral_vertex_count,
                oral_vertex_colors,
                trailing_vertex_count=lining_vertex_count + cavity_vertex_count,
            ),
            lining_vertex_count,
            lining_rgb,
            trailing_vertex_count=cavity_vertex_count,
        ),
    )
    outputs["jaw_open_mesh"] = str(jaw_open_path)

    # Keep an exact, lossless copy of the component topology for downstream
    # appearance-preserving export.  GLB importers are allowed to split or
    # merge vertices at material/attribute boundaries, which makes component
    # ranges and source-UV transfer ambiguous after the fact.  The NPZ stores
    # the canonical neutral and jawOpen meshes before GLB serialization.
    component_data_path = output_dir / f"{prefix}_lip_split_components.npz"
    np.savez_compressed(
        component_data_path,
        neutral_vertices=neutral_vertices.detach().cpu().float().numpy(),
        jaw_open_vertices=export_vertices.detach().cpu().float().numpy(),
        faces=output_faces.detach().cpu().long().numpy(),
        vertex_colors=apply_component_vertex_color(
            apply_oral_vertex_colors(
                export_colors_with_cavity(
                    export_vertices,
                    cavity_vertex_count,
                    cavity_rgb,
                ),
                oral_vertex_count,
                oral_vertex_colors,
                trailing_vertex_count=lining_vertex_count + cavity_vertex_count,
            ),
            lining_vertex_count,
            lining_rgb,
            trailing_vertex_count=cavity_vertex_count,
        ).detach().cpu().float().numpy(),
        head_vertex_count=np.asarray(
            int(processed_mesh["vertices"].shape[0]), dtype=np.int64
        ),
        lip_annulus_vertex_count=np.asarray(lip_annulus_vertex_count, dtype=np.int64),
        oral_vertex_count=np.asarray(oral_vertex_count, dtype=np.int64),
        lining_vertex_count=np.asarray(lining_vertex_count, dtype=np.int64),
        cavity_vertex_count=np.asarray(cavity_vertex_count, dtype=np.int64),
    )
    outputs["component_data"] = str(component_data_path)

    report_path = output_dir / f"{prefix}_lip_split_jaw_open_report.json"
    outputs["report"] = str(report_path)
    report = {
        "mesh": str(mesh_path),
        "checkpoint": str(checkpoint_path),
        "split_mode": args.split_mode,
        "coincident_vertex_weld": (
            weld_report.to_dict() if weld_report is not None else None
        ),
        "lip_landmark_topology_repair": (
            lip_landmark_topology_repair_report.to_dict()
            if lip_landmark_topology_repair_report is not None
            else None
        ),
        "action_unit_id": JAW_OPEN_AU_ID,
        "action_unit_name": AU_NAME[JAW_OPEN_AU_ID],
        "lip_regions": lip_regions.to_dict() if lip_regions is not None else None,
        "split_lip_regions": (
            split_lip_regions.to_dict() if split_lip_regions is not None else None
        ),
        "split": split_report.to_dict() if split_report is not None else None,
        "inference_proxy": proxy_report,
        "probe_aperture_cut": (
            aperture_pass_reports[0] if aperture_pass_reports else None
        ),
        "probe_aperture_passes": aperture_pass_reports,
        "probe_aperture_total_removed_faces": aperture_total_removed_faces,
        "probe_stretch_passes": stretch_pass_reports,
        "probe_stretch_total_detected_faces": stretch_total_detected_faces,
        "probe_removal_connectivity_passes": connectivity_pass_reports,
        "probe_dense_removal_connectivity": (
            dense_connectivity_report.to_dict()
            if dense_connectivity_report is not None
            else None
        ),
        "probe_remaining_removal_connectivity": (
            remaining_connectivity_report.to_dict()
            if remaining_connectivity_report is not None
            else None
        ),
        "probe_total_removed_faces": probe_total_removed_faces,
        "probe_face_set_converged": probe_face_set_converged,
        "probe_stretch_remaining": (
            stretch_remaining_report.to_dict()
            if stretch_remaining_report is not None
            else None
        ),
        "probe_aperture_remaining": (
            aperture_remaining_report.to_dict()
            if aperture_remaining_report is not None
            else None
        ),
        "probe_aperture_converged": (
            probe_face_set_converged
            and remaining_connectivity_report is not None
            and remaining_connectivity_report.removed_face_count == 0
        ),
        "probe_converged": (
            probe_face_set_converged
            and remaining_connectivity_report is not None
            and remaining_connectivity_report.removed_face_count == 0
        ),
        "mouth_cavity": (
            {
                "neutral": neutral_cavity_report.to_dict(),
                "jaw_open": mouth_cavity_report.to_dict(),
                "color_rgb": [int(value) for value in args.mouth_cavity_color],
            }
            if mouth_cavity_report is not None
            else None
        ),
        "mouth_boundary_lining": (
            {
                "neutral": neutral_lining_report.to_dict(),
                "jaw_open": mouth_boundary_lining_report.to_dict(),
                "color_rgb": [
                    int(value) for value in args.mouth_boundary_lining_color
                ],
            }
            if mouth_boundary_lining_report is not None
            else None
        ),
        "mouth_boundary_smoothing": (
            mouth_boundary_smooth_report.to_dict()
            if mouth_boundary_smooth_report is not None
            else None
        ),
        "mouth_contour_clip": (
            mouth_contour_clip_report.to_dict()
            if mouth_contour_clip_report is not None
            else None
        ),
        "lip_annulus_repair": (
            lip_annulus_repair_report.to_dict()
            if lip_annulus_repair_report is not None
            else None
        ),
        "lip_annulus_head_cleanup": (
            lip_annulus_head_cleanup_report.to_dict()
            if lip_annulus_head_cleanup_report is not None
            else None
        ),
        "front_surface_orientation": (
            front_surface_orientation_report.to_dict()
            if front_surface_orientation_report is not None
            else None
        ),
        "post_bridge_mouth_boundary_smoothing": (
            post_bridge_smooth_report.to_dict()
            if post_bridge_smooth_report is not None
            else None
        ),
        "post_cleanup_mouth_boundary_smoothing": (
            post_cleanup_smooth_report.to_dict()
            if post_cleanup_smooth_report is not None
            else None
        ),
        "metahuman_oral_assembly": (
            oral_assembly_report.to_dict()
            if oral_assembly_report is not None
            else None
        ),
        "output_components": {
            "head_vertex_count": int(processed_mesh["vertices"].shape[0]),
            "lip_annulus_vertex_count": lip_annulus_vertex_count,
            "oral_vertex_count": oral_vertex_count,
            "lining_vertex_count": lining_vertex_count,
            "cavity_vertex_count": cavity_vertex_count,
            "total_vertex_count": int(render_neutral_vertices.shape[0]),
            "total_face_count": int(output_faces.shape[0]),
        },
        "oral_head_aperture_cleanup": (
            oral_head_aperture_report.to_dict()
            if oral_head_aperture_report is not None
            else None
        ),
        "oral_head_component_cleanup": (
            oral_head_component_report.to_dict()
            if oral_head_component_report is not None
            else None
        ),
        "mouth_hole_fill": (
            {
                "pass_count": len(mouth_hole_fill_reports),
                "initial_candidate_hole_cycle_count": (
                    mouth_hole_fill_reports[0].candidate_hole_cycle_count
                ),
                "total_filled_hole_cycle_count": sum(
                    item.filled_hole_cycle_count for item in mouth_hole_fill_reports
                ),
                "remaining_hole_cycle_count": (
                    mouth_hole_fill_reports[-1].candidate_hole_cycle_count
                    - mouth_hole_fill_reports[-1].filled_hole_cycle_count
                ),
                "total_added_face_count": sum(
                    item.added_face_count for item in mouth_hole_fill_reports
                ),
                "reoriented_face_count": sum(
                    item.reoriented_face_count for item in mouth_hole_fill_reports
                ),
                "boundary_edge_count_before": (
                    mouth_hole_fill_reports[0].boundary_edge_count_before
                ),
                "boundary_edge_count_after": (
                    mouth_hole_fill_reports[-1].boundary_edge_count_after
                ),
            }
            if mouth_hole_fill_reports
            else None
        ),
        "mouth_hole_fill_passes": [
            item.to_dict() for item in mouth_hole_fill_reports
        ],
        "mouth_corner_bridge": (
            mouth_corner_bridge_report.to_dict()
            if mouth_corner_bridge_report is not None
            else None
        ),
        "probe_predicted_delta_mean_norm": float(
            probe_predicted_delta.norm(dim=-1).mean().item()
        ),
        "probe_predicted_delta_max_norm": float(
            probe_predicted_delta.norm(dim=-1).max().item()
        ),
        "predicted_delta_mean_norm": float(predicted_delta.norm(dim=-1).mean().item()),
        "predicted_delta_max_norm": float(predicted_delta.norm(dim=-1).max().item()),
        "elapsed_seconds": time.monotonic() - pipeline_started,
        "outputs": outputs,
    }
    write_json(report_path, report)

    print(f"Loaded checkpoint: {checkpoint_path}")
    print(f"Loaded mesh:       {mesh_path}")
    if weld_report is not None:
        print(
            "Coincident weld:   "
            f"merged_vertices={weld_report.merged_vertex_count} "
            f"removed_degenerate_faces={weld_report.removed_degenerate_face_count}"
        )
    if lip_landmark_topology_repair_report is not None:
        print(
            "Lip landmarks:     "
            f"invalid={lip_landmark_topology_repair_report.invalid_lip_landmark_count} "
            f"detached={lip_landmark_topology_repair_report.off_component_lip_landmark_count} "
            f"outliers={lip_landmark_topology_repair_report.spatial_outlier_lip_landmark_count} "
            f"snapped={lip_landmark_topology_repair_report.snapped_lip_landmark_count} "
            f"skipped={lip_landmark_topology_repair_report.skipped_lip_landmark_count}"
        )
    if args.split_mode == "mediapipe-seam":
        print(
            "Lip seam split:    "
            f"candidate_faces={split_report.candidate_face_count} "
            f"seam_vertices={split_report.seam_vertex_count} "
            f"duplicated_vertices={split_report.duplicated_vertex_count} "
            f"mixed_faces={split_report.mixed_original_duplicate_face_count}"
        )
        if aperture_pass_reports:
            print(
                "Probe aperture cut: "
                f"passes={len(aperture_pass_reports)} "
                f"removed_faces={aperture_total_removed_faces} "
                "remaining_front_cover_faces="
                f"{aperture_remaining_report.removed_face_count} "
                f"final_faces={processed_mesh['faces'].shape[0]}"
            )
            print(
                "Probe stretch cut: "
                f"detected_faces={stretch_total_detected_faces} "
                "remaining_severe_faces="
                f"{stretch_remaining_report.removed_face_count}"
            )
        if mouth_cavity_report is not None:
            print(
                "Mouth cavity:      "
                f"vertices={mouth_cavity_report.vertex_count} "
                f"faces={mouth_cavity_report.face_count} "
                f"depth={mouth_cavity_report.center[args.seam_depth_axis]:.6f}"
            )
        if mouth_boundary_lining_report is not None:
            print(
                "Mouth edge lining: "
                f"cycles={mouth_boundary_lining_report.boundary_cycle_count} "
                f"vertices={mouth_boundary_lining_report.vertex_count} "
                f"faces={mouth_boundary_lining_report.face_count}"
            )
        if mouth_boundary_smooth_report is not None:
            print(
                "Boundary smoothing:"
                f" components={mouth_boundary_smooth_report.smoothed_component_count} "
                f"vertices={mouth_boundary_smooth_report.smoothed_boundary_vertex_count} "
                f"max_move={mouth_boundary_smooth_report.max_boundary_displacement:.6f}"
            )
        if mouth_hole_fill_reports:
            print(
                "Mouth hole fill:   "
                f"cycles={sum(item.filled_hole_cycle_count for item in mouth_hole_fill_reports)}/"
                f"{mouth_hole_fill_reports[0].candidate_hole_cycle_count} "
                f"added_faces={sum(item.added_face_count for item in mouth_hole_fill_reports)} "
                "remaining_holes="
                f"{mouth_hole_fill_reports[-1].candidate_hole_cycle_count - mouth_hole_fill_reports[-1].filled_hole_cycle_count} "
                "remaining_boundary_edges="
                f"{mouth_hole_fill_reports[-1].boundary_edge_count_after}"
            )
        if mouth_corner_bridge_report is not None:
            print(
                "Mouth corner repair: "
                f"sides={mouth_corner_bridge_report.repaired_sides} "
                f"added_faces={mouth_corner_bridge_report.added_face_count} "
                "remaining_boundary_edges="
                f"{mouth_corner_bridge_report.boundary_edge_count_after} "
                f"severe_faces={mouth_corner_bridge_report.severe_added_face_count} "
                f"flipped_faces={mouth_corner_bridge_report.flipped_added_face_count}"
            )
        if post_bridge_smooth_report is not None:
            print(
                "Post-bridge smooth:"
                f" vertices={post_bridge_smooth_report.smoothed_boundary_vertex_count} "
                f"max_move={post_bridge_smooth_report.max_boundary_displacement:.6f} "
                f"rejected_faces={post_bridge_smooth_report.quality_rejected_face_count}"
            )
        if post_cleanup_smooth_report is not None:
            print(
                "Post-cleanup smooth:"
                f" components={post_cleanup_smooth_report.smoothed_component_count} "
                f"vertices={post_cleanup_smooth_report.smoothed_boundary_vertex_count} "
                f"max_move={post_cleanup_smooth_report.max_boundary_displacement:.6f} "
                f"rejected_faces={post_cleanup_smooth_report.quality_rejected_face_count}"
            )
        if oral_assembly_report is not None:
            print(
                "MetaHuman mouth:   "
                f"vertices={oral_assembly_report.vertex_count} "
                f"faces={oral_assembly_report.face_count} "
                f"teeth_vertices={oral_assembly_report.tooth_vertex_count} "
                f"mode={oral_assembly_report.geometry_mode}"
            )
    elif args.split_mode == "region":
        print(
            "Lip region split:  "
            f"removed_faces={split_report.removed_face_count} "
            f"duplicated_vertices={split_report.duplicated_vertex_count} "
            f"remaining_direct_faces={split_report.direct_lip_connection_faces_after}"
        )
    else:
        print("Lip seam split:    disabled (weld-only baseline)")
    print(
        "JawOpen mean/max:  "
        f"{report['predicted_delta_mean_norm']:.9f} / "
        f"{report['predicted_delta_max_norm']:.9f}"
    )
    print(f"Wrote report:      {report_path}")
    for label, path in outputs.items():
        if label != "report":
            print(f"Wrote {label}: {path}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Split connected upper/lower lip topology, then run TopoRig jawOpen "
            "inference for visual inspection."
        )
    )
    parser.add_argument("--mesh", type=Path, required=True)
    parser.add_argument(
        "--checkpoint",
        type=Path,
        help="TopoRig checkpoint. Defaults to the newest runs/**/checkpoints/*.pt.",
    )
    parser.add_argument("--config", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--prefix", default=None)
    parser.add_argument("--device", default=None)
    parser.add_argument(
        "--proxy-target-faces",
        type=positive_int,
        default=None,
        help=(
            "Run TopoRig and iterative probe cleanup on a compiled decimation "
            "proxy, then transfer displacement back to the dense mesh."
        ),
    )
    parser.add_argument(
        "--proxy-aggression",
        type=float,
        default=7.0,
        help="Compiled proxy decimation aggression from 0 (careful) to 10 (fast).",
    )
    parser.add_argument(
        "--proxy-transfer-smoothing-iterations",
        type=int,
        default=6,
        help="Dense topology smoothing iterations after proxy displacement transfer.",
    )
    parser.add_argument(
        "--proxy-transfer-smoothing-blend",
        type=float,
        default=0.5,
        help="Neighbor-average blend per proxy transfer smoothing iteration.",
    )
    parser.add_argument(
        "--proxy-output-resolution",
        choices=("dense", "proxy"),
        default="dense",
        help=(
            "Transfer deformation back to the dense mesh or export the proxy "
            "directly. Proxy output is substantially faster."
        ),
    )
    parser.add_argument(
        "--proxy-one-shot-probe",
        action="store_true",
        help=(
            "Remove faces detected by the first jawOpen prediction without "
            "rerunning the model. This applies with or without decimation."
        ),
    )
    parser.add_argument(
        "--proxy-before-split",
        action="store_true",
        help=(
            "Build the fast inference proxy before constructing the lip seam so "
            "decimation cannot collapse the newly separated upper/lower boundary."
        ),
    )
    parser.add_argument(
        "--proxy-before-split-maximum-source-ratio",
        type=float,
        default=3.5,
        help=(
            "Maximum source/target face ratio for proxy-before-split. More "
            "aggressive reductions preserve the dense seam before proxying. "
            "Default: 3.5."
        ),
    )
    parser.add_argument("--output-format", choices=("glb", "obj"), default="glb")
    parser.add_argument(
        "--split-mode",
        choices=("mediapipe-seam", "weld-only", "region"),
        default="mediapipe-seam",
        help=(
            "mediapipe-seam welds exact duplicates and separates the closed-mouth "
            "seam; weld-only diagnoses duplicate topology; region retains the "
            "older Sapiens/vertex-region behavior."
        ),
    )
    parser.add_argument(
        "--output-convention",
        choices=("metahuman", "model"),
        default="metahuman",
    )
    parser.add_argument("--seam-horizontal-axis", type=int, default=0)
    parser.add_argument("--seam-depth-axis", type=int, default=1)
    parser.add_argument("--seam-vertical-axis", type=int, default=2)
    parser.add_argument(
        "--seam-depth-tolerance",
        type=float,
        default=0.02,
        help="Maximum depth from the MediaPipe inner-lip centerline to split.",
    )
    parser.add_argument(
        "--seam-vertical-tolerance",
        type=float,
        default=0.012,
        help="Vertical band around the inner-lip centerline used to find the seam.",
    )
    parser.add_argument(
        "--seam-horizontal-padding",
        type=float,
        default=0.0,
        help="Optional horizontal extension beyond the two inner mouth corners.",
    )
    parser.add_argument(
        "--seam-corner-inset-ratio",
        type=float,
        default=0.05,
        help=(
            "Fraction of mouth width left connected at each seam endpoint. "
            "Default: 0.05."
        ),
    )
    parser.add_argument(
        "--seam-preopen-distance",
        type=float,
        default=0.002,
        help=(
            "Total neutral gap inserted between duplicated upper/lower seam "
            "vertices before inference. Default: 0.002."
        ),
    )
    parser.add_argument(
        "--no-weld-coincident-vertices",
        action="store_true",
        help=(
            "Do not merge exact duplicate vertices before the MediaPipe seam split."
        ),
    )
    parser.add_argument(
        "--no-repair-mediapipe-lip-landmarks",
        action="store_true",
        help=(
            "Do not repair invalid or detached lip landmarks onto the dominant "
            "connected mouth surface before seam construction."
        ),
    )
    parser.add_argument(
        "--lip-landmark-maximum-snap-distance-ratio",
        type=float,
        default=0.25,
        help=(
            "Maximum repair distance relative to the mapped mouth span. "
            "Default: 0.25."
        ),
    )
    parser.add_argument(
        "--reconstruct-lip-annulus",
        action="store_true",
        help=(
            "Add a continuous skin strip derived from the repaired outer "
            "MediaPipe lip contour after mouth cleanup."
        ),
    )
    parser.add_argument(
        "--lip-annulus-segments",
        type=positive_int,
        default=64,
        help="Even segment count around the reconstructed lip rim. Default: 64.",
    )
    parser.add_argument(
        "--lip-annulus-radial-rings",
        type=positive_int,
        default=3,
        help="Radial subdivisions across the reconstructed lip rim. Default: 3.",
    )
    parser.add_argument(
        "--lip-annulus-outer-scale",
        type=float,
        default=1.03,
        help="Scale applied to the outer lip repair curve. Default: 1.03.",
    )
    parser.add_argument(
        "--lip-annulus-thickness-ratio",
        type=float,
        default=0.055,
        help="Uniform lip-rim width relative to mouth span. Default: 0.055.",
    )
    parser.add_argument(
        "--lip-annulus-maximum-radial-inset-fraction",
        type=float,
        default=0.45,
        help=(
            "Maximum fraction of the local mouth radius used by the lip rim. "
            "Default: 0.45."
        ),
    )
    parser.add_argument(
        "--lip-annulus-side-thickness-factor",
        type=float,
        default=0.45,
        help="Rim thickness at mouth corners relative to upper/lower lips. Default: 0.45.",
    )
    parser.add_argument(
        "--lip-annulus-inner-recess-depth",
        type=float,
        default=0.003,
        help="Depth that recesses the inner rim behind the outer lip. Default: 0.003.",
    )
    parser.add_argument(
        "--lip-annulus-front-offset",
        type=float,
        default=0.002,
        help=(
            "Small frontward offset that prevents overlap with damaged source "
            "lip triangles. Default: 0.002."
        ),
    )
    parser.add_argument(
        "--lip-annulus-bulge-depth",
        type=float,
        default=0.0025,
        help="Frontward curvature across the repaired lip strip. Default: 0.0025.",
    )
    parser.add_argument(
        "--lip-annulus-upper-smoothing-window",
        type=positive_int,
        default=9,
        help="Odd smoothing window for the upper repair curve. Default: 9.",
    )
    parser.add_argument(
        "--lip-annulus-lower-smoothing-window",
        type=positive_int,
        default=17,
        help="Odd smoothing window for the lower repair curve. Default: 17.",
    )
    parser.add_argument(
        "--clear-head-under-lip-annulus",
        action="store_true",
        help="Remove source-head triangles replaced by the generated lip rim.",
    )
    parser.add_argument(
        "--lip-annulus-cleanup-depth-padding-ratio",
        type=float,
        default=0.30,
        help="Depth padding for source faces cleared under the lip rim. Default: 0.30.",
    )
    parser.add_argument(
        "--orient-front-surface-faces",
        action="store_true",
        help=(
            "Correct reversed triangle winding on the external facial shell "
            "before appending the oral assembly."
        ),
    )
    parser.add_argument(
        "--front-surface-depth-ratio",
        type=float,
        default=0.18,
        help="Depth of the external facial orientation band. Default: 0.18.",
    )
    parser.add_argument(
        "--front-surface-normal-tolerance",
        type=float,
        default=0.01,
        help="Minimum reversed normal component required for a flip. Default: 0.01.",
    )
    parser.add_argument(
        "--no-probe-aperture-cut",
        action="store_true",
        help="Disable the probe jawOpen pass that removes faces covering the aperture.",
    )
    parser.add_argument(
        "--probe-aperture-scale",
        type=float,
        default=1.0,
        help="Scale of the opened inner-lip polygon used by the probe cut.",
    )
    parser.add_argument(
        "--probe-aperture-depth-padding",
        type=float,
        default=0.01,
        help="Depth padding around the deformed inner lips for the probe cut.",
    )
    parser.add_argument(
        "--probe-aperture-overlap-inset-scale",
        type=float,
        default=0.9,
        help=(
            "Inset scale used to catch triangles crossing the aperture when their "
            "centroids remain outside it. Default: 0.9."
        ),
    )
    parser.add_argument(
        "--probe-aperture-max-passes",
        type=positive_int,
        default=10,
        help="Maximum number of probe/cut passes before final verification.",
    )
    parser.add_argument(
        "--probe-restore-safe-passes",
        type=positive_int,
        default=2,
        help=(
            "Consecutive safe predictions required before restoring a removed "
            "source face. Default: 2."
        ),
    )
    parser.add_argument("--probe-stretch-edge-ratio", type=float, default=3.0)
    parser.add_argument("--probe-stretch-edge-growth", type=float, default=0.02)
    parser.add_argument("--probe-stretch-region-scale", type=float, default=1.5)
    parser.add_argument("--probe-stretch-depth-padding", type=float, default=0.08)
    parser.add_argument(
        "--smooth-contour-clip",
        action="store_true",
        help=(
            "Reconstruct cut boundary triangles against a smooth MediaPipe "
            "inner-lip contour. Disabled by default."
        ),
    )
    parser.add_argument(
        "--smooth-contour-scale",
        type=float,
        default=1.0,
        help="Scale of the smooth inner-lip contour. Default: 1.0.",
    )
    parser.add_argument(
        "--smooth-contour-depth-padding",
        type=float,
        default=0.08,
        help="Depth band in which removed boundary faces may be reconstructed.",
    )
    parser.add_argument(
        "--no-fill-secondary-mouth-holes",
        action="store_true",
        help="Do not cap secondary mouth-area boundary cycles after probe cleanup.",
    )
    parser.add_argument(
        "--mouth-hole-max-area-ratio",
        type=float,
        default=0.15,
        help="Largest secondary cycle area to cap relative to the main aperture.",
    )
    parser.add_argument(
        "--no-smooth-mouth-boundary",
        action="store_true",
        help="Disable bounded smoothing of the final mouth cut loops.",
    )
    parser.add_argument(
        "--mouth-boundary-max-displacement",
        type=float,
        default=0.006,
        help="Maximum projected movement of a mouth boundary vertex.",
    )
    parser.add_argument(
        "--mouth-boundary-blend-rings",
        type=positive_int,
        default=5,
        help="Adjacent face rings used to blend the boundary correction.",
    )
    parser.add_argument(
        "--post-cleanup-mouth-boundary-smoothing",
        action="store_true",
        help=(
            "Smooth substantial aperture fragments after oral head cleanup. "
            "This changes head vertices only and is disabled by default."
        ),
    )
    parser.add_argument(
        "--post-cleanup-boundary-minimum-span",
        type=float,
        default=0.15,
        help=(
            "Minimum horizontal or vertical span of a cleanup-created boundary "
            "fragment relative to the inner lips. Default: 0.15."
        ),
    )
    parser.add_argument(
        "--post-cleanup-boundary-minimum-vertices",
        type=positive_int,
        default=8,
        help="Minimum vertices in a smoothed cleanup boundary. Default: 8.",
    )
    parser.add_argument(
        "--post-cleanup-boundary-max-displacement",
        type=float,
        default=0.003,
        help="Maximum movement of a final head-boundary vertex. Default: 0.003.",
    )
    parser.add_argument(
        "--post-cleanup-boundary-blend-rings",
        type=positive_int,
        default=3,
        help="Adjacent head rings blended by final boundary smoothing. Default: 3.",
    )
    parser.add_argument(
        "--post-cleanup-boundary-smoothing-cycles",
        type=positive_int,
        default=4,
        help="Taubin cycles used by final boundary smoothing. Default: 4.",
    )
    parser.add_argument(
        "--post-bridge-smoothing",
        action="store_true",
        help="Apply a second conservative boundary pass after mouth-corner repair.",
    )
    parser.add_argument(
        "--post-bridge-max-displacement",
        type=float,
        default=0.0025,
        help="Maximum movement in the post-bridge smoothing pass.",
    )
    parser.add_argument(
        "--post-bridge-blend-rings",
        type=positive_int,
        default=3,
        help="Adjacent rings blended by the post-bridge smoothing pass.",
    )
    parser.add_argument(
        "--post-bridge-smoothing-cycles",
        type=positive_int,
        default=4,
        help="Taubin cycles used by the conservative post-bridge pass.",
    )
    parser.add_argument(
        "--no-repair-mouth-corners",
        action="store_true",
        help="Disable localized bridging of a laterally overextended mouth corner.",
    )
    parser.add_argument(
        "--mouth-corner-inset-ratio",
        type=float,
        default=0.13,
        help=(
            "Fraction of lip width included in a localized corner bridge. "
            "Default: 0.13."
        ),
    )
    parser.add_argument(
        "--mouth-corner-minimum-overshoot-ratio",
        type=float,
        default=0.12,
        help=(
            "Minimum boundary overshoot relative to lip width before repair. "
            "Default: 0.12."
        ),
    )
    parser.add_argument(
        "--mouth-corner-maximum-repaired-sides",
        type=positive_int,
        choices=(1, 2),
        default=1,
        help="Maximum number of overshooting mouth corners to bridge. Default: 1.",
    )
    parser.add_argument(
        "--mouth-corner-chain-padding-vertices",
        type=positive_int,
        default=1,
        help=(
            "Boundary vertices included beyond each detected corner chain end. "
            "Default: 1."
        ),
    )
    parser.add_argument(
        "--mouth-cavity",
        action="store_true",
        help=(
            "Add the inner-mouth occluder. This is enabled automatically when "
            "a dental-only oral asset is fitted."
        ),
    )
    parser.add_argument(
        "--mouth-boundary-lining",
        action="store_true",
        help=(
            "Extrude the actual mouth cut loop into an oral compartment. This "
            "is enabled automatically when a dental-only oral asset is fitted."
        ),
    )
    parser.add_argument(
        "--no-mouth-boundary-lining",
        action="store_true",
        help="Disable the automatic cut-boundary oral compartment.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-depth",
        type=float,
        default=0.24,
        help="Depth of the oral compartment behind the cut edge. Default: 0.24.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-rim-depth",
        type=float,
        default=0.002,
        help="Small recession applied at the visible lining rim. Default: 0.002.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-side-fraction",
        type=float,
        default=1.0,
        help=(
            "Fraction of lip width retained at each side of the boundary lining. "
            "One keeps the complete loops."
        ),
    )
    parser.add_argument(
        "--mouth-boundary-lining-inner-scale",
        type=float,
        default=0.95,
        help=(
            "Scale the rear compartment ring toward the mouth center. Default: 0.95."
        ),
    )
    parser.add_argument(
        "--mouth-boundary-lining-radial-rings",
        type=positive_int,
        default=6,
        help="Depth sections used by the oral compartment. Default: 6.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-segments",
        type=positive_int,
        default=96,
        help="Ordered superellipse segments around the full oral opening. Default: 96.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-opening-scale",
        type=float,
        default=1.08,
        help="Scale of the compartment rim relative to detected opening bounds.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-shape-exponent",
        type=float,
        default=2.8,
        help="Superellipse exponent of the oral compartment rim. Default: 2.8.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-smoothing-iterations",
        type=int,
        default=12,
        help="Cyclic smoothing passes for recessed compartment rings. Default: 12.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-maximum-cycles",
        type=positive_int,
        default=1,
        help="Maximum dominant mouth openings enclosed. Default: 1.",
    )
    parser.add_argument(
        "--no-mouth-boundary-lining-cap",
        action="store_true",
        help="Leave the rear of the mouth boundary lining open.",
    )
    parser.add_argument(
        "--mouth-boundary-lining-color",
        type=rgb_channel,
        nargs=3,
        default=(87, 19, 22),
        metavar=("R", "G", "B"),
        help="Mouth boundary lining RGB color. Default: 87 19 22.",
    )
    parser.add_argument(
        "--oral-asset",
        "--metahuman-oral-asset",
        dest="metahuman_oral_asset",
        type=Path,
        help="Fitted teeth, tongue, gums, palate, and oral-cavity NPZ.",
    )
    parser.add_argument(
        "--oral-assembly-scale",
        type=float,
        default=1.0,
        help="Scale the fitted oral assembly around the mouth center.",
    )
    parser.add_argument(
        "--no-oral-head-aperture-cleanup",
        action="store_true",
        help="Keep source head surfaces projected inside the inserted oral aperture.",
    )
    parser.add_argument(
        "--oral-head-aperture-scale",
        type=float,
        default=0.90,
        help="Inset outer-lip scale used to clear source mouth interiors. Default: 0.90.",
    )
    parser.add_argument(
        "--oral-head-depth-padding-ratio",
        type=float,
        default=0.15,
        help="Front-surface cleanup depth padding relative to mouth span. Default: 0.15.",
    )
    parser.add_argument(
        "--oral-head-component-margin-ratio",
        type=float,
        default=0.25,
        help="Extra mouth-bounds margin for disconnected source interiors.",
    )
    parser.add_argument(
        "--oral-head-boundary-distance-ratio",
        type=float,
        default=0.125,
        help="Distance from the outer contour protected as visible lip boundary.",
    )
    parser.add_argument(
        "--oral-head-boundary-blend-rings",
        type=int,
        default=2,
        help="Adjacent locally sized lip rings protected from cleanup. Default: 2.",
    )
    parser.add_argument(
        "--oral-head-maximum-protected-edge-ratio",
        type=float,
        default=0.25,
        help="Largest protected adjacent edge relative to mouth span. Default: 0.25.",
    )
    parser.add_argument(
        "--oral-head-component-maximum-faces",
        type=int,
        default=5000,
        help="Largest disconnected source-mouth component to remove. Default: 5000.",
    )
    parser.add_argument(
        "--no-oral-head-lower-lip-protection",
        action="store_true",
        help="Disable preservation of locally sized faces in the lower-lip landmark ribbon.",
    )
    parser.add_argument(
        "--oral-head-lower-lip-depth-tolerance-ratio",
        type=float,
        default=0.12,
        help="Maximum lower-lip face depth deviation relative to mouth span.",
    )
    parser.add_argument(
        "--oral-depth-offset",
        type=float,
        default=0.012,
        help=(
            "Move the oral assembly behind the lips in model units to prevent "
            "gum intersections. Default: 0.012."
        ),
    )
    parser.add_argument(
        "--no-oral-level-target-frame",
        action="store_true",
        help="Allow mouth-corner slope to rotate the fitted oral assembly.",
    )
    parser.add_argument(
        "--oral-auto-center-teeth",
        action="store_true",
        help=(
            "Center visible tooth bounds horizontally after landmark fitting."
        ),
    )
    parser.add_argument(
        "--oral-center-teeth-vertically",
        action="store_true",
        help=(
            "Also center tooth bounds vertically. Disabled by default because the "
            "source upper/lower jaw offset should normally be preserved."
        ),
    )
    parser.add_argument(
        "--oral-maximum-center-offset-ratio",
        type=float,
        default=0.25,
        help=(
            "Maximum tooth-centering correction as a fraction of target mouth "
            "span. Default: 0.25."
        ),
    )
    parser.add_argument(
        "--oral-tooth-depth-offset",
        type=float,
        default=0.0,
        help=(
            "Additional model-depth offset applied to tooth crowns with a local "
            "falloff. Negative values move teeth forward for the default axes."
        ),
    )
    parser.add_argument(
        "--oral-tooth-depth-blend-rings",
        type=int,
        default=2,
        help="Mesh rings used to taper tooth-only depth movement. Default: 2.",
    )
    parser.add_argument(
        "--oral-auto-center-lower-teeth",
        action="store_true",
        help="Center only the moving lower dental region horizontally.",
    )
    parser.add_argument(
        "--oral-lower-teeth-depth-offset",
        type=float,
        default=0.0,
        help="Additional depth offset for lower crowns and adjacent moving gum.",
    )
    parser.add_argument(
        "--oral-lower-teeth-blend-rings",
        type=int,
        default=2,
        help="Mesh rings used to taper lower dental movement. Default: 2.",
    )
    parser.add_argument(
        "--no-oral-containment",
        action="store_true",
        help="Disable the default deformation that keeps oral geometry inside the lips.",
    )
    parser.add_argument(
        "--oral-containment-scale",
        type=float,
        default=0.96,
        help="Scale of the outer-lip polygon used for oral containment. Default: 0.96.",
    )
    parser.add_argument(
        "--oral-containment-depth-inset",
        type=float,
        default=0.025,
        help="Depth recession applied to oral vertices outside the lip contour.",
    )
    parser.add_argument(
        "--oral-containment-blend-rings",
        type=int,
        default=2,
        help="Adjacent gum rings blended into oral containment. Default: 2.",
    )
    parser.add_argument(
        "--oral-rim-recession-ratio",
        type=float,
        default=0.08,
        help=(
            "Minimum gum recession behind the nearest inner-lip segment as a "
            "fraction of mouth span. Teeth and tongue are unchanged. Default: 0.08."
        ),
    )
    parser.add_argument(
        "--oral-recessed-shell-horizontal-scale",
        type=float,
        default=1.0,
        help=(
            "Horizontal scale for recessed oral-wall tissue only. Teeth, tongue, "
            "and front gum remain fixed. Default: 1.0."
        ),
    )
    parser.add_argument(
        "--oral-recessed-shell-start-depth-ratio",
        type=float,
        default=0.06,
        help="Recessed depth where oral-wall widening begins. Default: 0.06.",
    )
    parser.add_argument(
        "--oral-recessed-shell-blend-depth-ratio",
        type=float,
        default=0.28,
        help="Depth interval used to blend in oral-wall widening. Default: 0.28.",
    )
    parser.add_argument(
        "--no-oral-rim-transition-trim",
        action="store_true",
        help="Keep exposed gum/root transition triangles outside the inner lips.",
    )
    parser.add_argument(
        "--oral-rim-transition-distance-ratio",
        type=float,
        default=0.08,
        help="Maximum transition-face distance from the inner-lip rim. Default: 0.08.",
    )
    parser.add_argument(
        "--oral-rim-transition-depth-tolerance-ratio",
        type=float,
        default=0.01,
        help="Local lip-depth tolerance for oral rim trimming. Default: 0.01.",
    )
    parser.add_argument(
        "--oral-upper-crown-quantile",
        type=float,
        default=0.50,
        help=(
            "Fraction of the static upper tooth column colored and moved as "
            "visible crown. Lower values hide more root. Default: 0.50."
        ),
    )
    parser.add_argument(
        "--oral-upper-teeth-vertical-offset",
        type=float,
        default=0.0,
        help=(
            "Vertical offset for static upper crowns only. Negative values move "
            "them downward for the default axes."
        ),
    )
    parser.add_argument(
        "--oral-upper-teeth-vertical-blend-rings",
        type=int,
        default=2,
        help="Mesh rings used to taper upper-crown vertical movement. Default: 2.",
    )
    oral_geometry_group = parser.add_mutually_exclusive_group()
    oral_geometry_group.add_argument(
        "--oral-full-shell",
        "--no-oral-wall-uv-removal",
        dest="oral_full_shell",
        action="store_true",
        help=(
            "Retain the complete MetaHuman palate, floor, side walls, gums, "
            "teeth, and tongue. This is the default."
        ),
    )
    oral_geometry_group.add_argument(
        "--oral-dental-only",
        action="store_true",
        help=(
            "Keep only the MetaHuman dental arches and tongue, then use the "
            "generated oral compartment fallback."
        ),
    )
    parser.add_argument(
        "--oral-remove-tongue-uv-island",
        action="store_true",
        help=(
            "Remove the broad MetaHuman tongue UV island while retaining both "
            "dental arches and oral-wall geometry."
        ),
    )
    parser.add_argument(
        "--oral-tongue-planar-scale",
        type=float,
        default=1.40,
        help="Scale tongue width and visible height around its center. Default: 1.40.",
    )
    parser.add_argument(
        "--oral-tongue-depth-offset",
        type=float,
        default=0.0,
        help="Additional model-depth offset for the UV-defined tongue island.",
    )
    parser.add_argument(
        "--oral-tongue-vertical-offset",
        type=float,
        default=0.0,
        help="Additional vertical offset for the UV-defined tongue island.",
    )
    parser.add_argument(
        "--oral-tongue-offset-blend-rings",
        type=int,
        default=2,
        help="Mesh rings used to taper tongue repositioning. Default: 2.",
    )
    parser.add_argument(
        "--oral-aperture-crop-scale",
        type=float,
        default=None,
        help=(
            "Optional outer-lip crop scale for debugging. Disabled by default "
            "because cropping can remove teeth and gum geometry."
        ),
    )
    parser.add_argument(
        "--no-oral-aperture-crop",
        action="store_true",
        help="Keep the full oral assembly without outer-lip cropping.",
    )
    parser.add_argument(
        "--no-mouth-cavity",
        action="store_true",
        help="Disable the inner-mouth cavity generated for the oral assembly.",
    )
    parser.add_argument(
        "--mouth-cavity-scale",
        type=float,
        default=1.05,
        help="Scale of the cavity disk relative to the opened inner lips.",
    )
    parser.add_argument(
        "--mouth-cavity-depth-offset",
        type=float,
        default=0.03,
        help="Distance behind the rearmost inner-lip point for the cavity rim.",
    )
    parser.add_argument(
        "--mouth-cavity-shape-exponent",
        type=float,
        default=2.4,
        help="Superellipse exponent for the elliptical cavity rim.",
    )
    parser.add_argument(
        "--mouth-cavity-segments",
        type=positive_int,
        default=64,
        help="Number of radial segments in the cavity disk.",
    )
    parser.add_argument(
        "--mouth-cavity-radial-rings",
        type=positive_int,
        default=8,
        help="Concentric rings used to form the curved cavity lining.",
    )
    parser.add_argument(
        "--mouth-cavity-curvature-depth",
        type=float,
        default=0.20,
        help="Additional recession at the center of the cavity lining.",
    )
    parser.add_argument(
        "--mouth-cavity-inner-radius-ratio",
        type=float,
        default=0.0,
        help=(
            "Leave the center open and generate only an annular side lining. "
            "Zero retains the filled cavity."
        ),
    )
    parser.add_argument(
        "--mouth-cavity-side-wall-min-horizontal-ratio",
        type=float,
        default=0.0,
        help=(
            "Keep only left/right annular segments whose normalized horizontal "
            "position reaches this ratio. Zero keeps every segment."
        ),
    )
    parser.add_argument(
        "--mouth-cavity-color",
        type=rgb_channel,
        nargs=3,
        default=(105, 32, 45),
        metavar=("R", "G", "B"),
        help="Mouth cavity RGB color. Default: 105 32 45.",
    )
    parser.add_argument(
        "--sapiens-lip-regions",
        "--lip-regions",
        dest="lip_regions",
        type=Path,
        help=(
            "Projected Sapiens lip regions. JSON/NPZ may contain "
            "upper_lip_vertex_ids and lower_lip_vertex_ids, boolean masks, "
            "or per-vertex labels."
        ),
    )
    parser.add_argument(
        "--upper-lip-vertices",
        help="Comma/space-separated vertex ids, or a text file containing them.",
    )
    parser.add_argument(
        "--lower-lip-vertices",
        help="Comma/space-separated vertex ids, or a text file containing them.",
    )
    parser.add_argument(
        "--upper-label-ids",
        type=int,
        nargs="*",
        default=(),
        help="Label ids to treat as upper lip when --lip-regions is a label array.",
    )
    parser.add_argument(
        "--lower-label-ids",
        type=int,
        nargs="*",
        default=(),
        help="Label ids to treat as lower lip when --lip-regions is a label array.",
    )
    parser.add_argument(
        "--lip-mask-rings",
        type=int,
        default=1,
        help="Fallback MediaPipe lip seed expansion in mesh edge rings.",
    )
    parser.add_argument(
        "--include-outer-lip-landmarks",
        action="store_true",
        help="Use outer MediaPipe lip landmarks in the fallback lip mask.",
    )
    parser.add_argument(
        "--use-mediapipe-lip-paths",
        action="store_true",
        help=(
            "Build fallback lip masks from mesh paths between MediaPipe lip "
            "landmarks. This is more reliable on closed-mouth meshes than "
            "isolated landmark seeds."
        ),
    )
    parser.add_argument(
        "--keep-mixed-faces",
        action="store_true",
        help="Duplicate shared seam vertices but do not remove upper/lower bridge faces.",
    )
    parser.add_argument(
        "--mouth-band-cut",
        action="store_true",
        help=(
            "Also remove faces whose centroids lie in the MediaPipe inner-mouth "
            "lip polygon. Useful for closed-mouth meshes with a broad welded "
            "patch between lips."
        ),
    )
    parser.add_argument(
        "--mouth-band-padding",
        type=float,
        default=0.0,
        help="Scale the mouth-band polygon outward by this fraction before cutting.",
    )
    parser.add_argument(
        "--mouth-band-axes",
        type=int,
        nargs=2,
        default=(0, 2),
        metavar=("AXIS_A", "AXIS_B"),
        help="Coordinate axes used for the 2D mouth-band polygon. Default: 0 2.",
    )
    parser.add_argument(
        "--mouth-band-depth-axis",
        type=int,
        default=1,
        help="Coordinate axis used to constrain the mouth-band depth. Default: 1.",
    )
    parser.add_argument(
        "--mouth-band-depth-padding",
        type=float,
        default=0.006,
        help=(
            "Padding around inner-lip landmarks on the depth axis. Lower values "
            "remove fewer off-surface face patches. Default: 0.006."
        ),
    )
    parser.add_argument("--no-raw-split-mesh", action="store_true")
    parser.add_argument(
        "--input-convention",
        choices=(
            "auto",
            "fbx",
            "custom-glb",
            "custom-glb-y-front",
            "custom-glb-neg-y-front",
            "custom-glb-y-up-z-front",
            "custom-glb-y-up-neg-z-front",
        ),
        default="auto",
    )
    parser.add_argument(
        "--reference-input-convention",
        choices=(
            "auto",
            "fbx",
            "custom-glb",
            "custom-glb-y-front",
            "custom-glb-neg-y-front",
            "custom-glb-y-up-z-front",
            "custom-glb-y-up-neg-z-front",
        ),
        default="auto",
    )
    parser.add_argument(
        "--alignment-mode",
        choices=("none", "similarity", "scale_translation"),
        default="similarity",
    )
    parser.add_argument(
        "--landmark-json",
        type=Path,
        help="Optional precomputed MediaPipe mesh landmark mapping for the input mesh.",
    )
    parser.add_argument(
        "--metahuman-sample-dir",
        type=Path,
        default=demo.DEFAULT_METAHUMAN_SAMPLE_DIR,
    )
    parser.add_argument(
        "--landmark-cache-dir",
        type=Path,
        default=demo.DEFAULT_LANDMARK_CACHE_DIR,
    )
    parser.add_argument(
        "--mediapipe-mapper",
        type=Path,
        default=demo.DEFAULT_MEDIAPIPE_MAPPER,
    )
    parser.add_argument("--blender", default=None)
    parser.add_argument("--refresh-landmarks", action="store_true")
    parser.add_argument("--min-alignment-landmarks", type=int, default=32)
    parser.add_argument(
        "--reference-processed-cache",
        type=Path,
        default=ROOT / "runs" / "cache" / "metahuman_reference_preprocessed.pt",
        help="Cached preprocessed reference mesh used for landmark alignment.",
    )
    parser.add_argument(
        "--no-mediapipe-landmarks",
        action="store_true",
        help=(
            "Do not generate MediaPipe landmarks. Requires explicit lip regions "
            "and --alignment-mode none."
        ),
    )
    return parser.parse_args()


def resolve_input_landmarks(
    args: argparse.Namespace,
    mesh_path: Path,
) -> Optional[dict[int, int]]:
    if args.no_mediapipe_landmarks:
        return None
    if args.landmark_json is not None:
        return demo.load_landmark_mapping(args.landmark_json.expanduser())

    needs_fallback_lips = (
        args.split_mode == "mediapipe-seam"
        or (
            args.lip_regions is None
            and not (args.upper_lip_vertices and args.lower_lip_vertices)
        )
    )
    needs_alignment = args.alignment_mode != "none"
    if not needs_fallback_lips and not needs_alignment:
        return None

    if mesh_path.suffix.lower() == ".obj":
        raise ValueError(
            "MediaPipe fallback landmark generation supports FBX/GLB/GLTF in "
            "map_mediapipe_landmarks.py. For OBJ input, pass --lip-regions or "
            "--upper-lip-vertices/--lower-lip-vertices and use "
            "--alignment-mode none, or pass --landmark-json."
        )
    landmark_path = demo.ensure_mediapipe_landmarks(
        mesh_path=mesh_path,
        cache_dir=args.landmark_cache_dir.expanduser(),
        mapper_path=args.mediapipe_mapper.expanduser(),
        blender=args.blender,
        force=args.refresh_landmarks,
    )
    return demo.load_landmark_mapping(landmark_path)


def resolve_lip_regions(
    args: argparse.Namespace,
    raw_mesh: Mapping[str, torch.Tensor],
    input_landmarks: Optional[Mapping[int, int]],
) -> LipRegions:
    vertex_count = int(raw_mesh["vertices"].shape[0])
    explicit_upper = load_vertex_ids_arg(args.upper_lip_vertices, vertex_count)
    explicit_lower = load_vertex_ids_arg(args.lower_lip_vertices, vertex_count)
    if explicit_upper.numel() > 0 or explicit_lower.numel() > 0:
        if explicit_upper.numel() == 0 or explicit_lower.numel() == 0:
            raise ValueError(
                "Pass both --upper-lip-vertices and --lower-lip-vertices, or neither."
            )
        return LipRegions(explicit_upper, explicit_lower, source="cli")

    if args.use_mediapipe_lip_paths:
        if input_landmarks is None:
            raise ValueError(
                "--use-mediapipe-lip-paths requires MediaPipe landmarks. Pass "
                "--landmark-json or allow landmark generation."
            )
        return lip_regions_from_mediapipe_paths(
            raw_mesh["vertices"],
            raw_mesh["faces"],
            input_landmarks,
            rings=args.lip_mask_rings,
            include_outer_lip=args.include_outer_lip_landmarks,
        )

    if args.lip_regions is not None:
        return load_lip_regions(
            args.lip_regions,
            vertex_count,
            upper_label_ids=args.upper_label_ids,
            lower_label_ids=args.lower_label_ids,
        )

    if input_landmarks is None:
        raise ValueError(
            "No lip regions were supplied and MediaPipe landmarks are disabled."
        )
    return lip_regions_from_mediapipe_mapping(
        raw_mesh["vertices"],
        raw_mesh["faces"],
        input_landmarks,
        rings=args.lip_mask_rings,
        include_outer_lip=args.include_outer_lip_landmarks,
    )


def resolve_extra_removed_face_ids(
    args: argparse.Namespace,
    raw_mesh: Mapping[str, torch.Tensor],
    input_landmarks: Optional[Mapping[int, int]],
) -> torch.Tensor:
    if not args.mouth_band_cut:
        return torch.empty(0, dtype=torch.long)
    if input_landmarks is None:
        raise ValueError(
            "--mouth-band-cut requires MediaPipe landmarks. Pass --landmark-json "
            "or allow landmark generation."
        )
    return mouth_band_face_ids_from_mediapipe(
        raw_mesh["vertices"],
        raw_mesh["faces"],
        input_landmarks,
        axes=(int(args.mouth_band_axes[0]), int(args.mouth_band_axes[1])),
        padding=float(args.mouth_band_padding),
        depth_axis=int(args.mouth_band_depth_axis),
        depth_padding=float(args.mouth_band_depth_padding),
    )


def resolve_reference_landmarks(args: argparse.Namespace) -> Optional[dict[int, int]]:
    if args.alignment_mode == "none":
        return None
    reference_mesh_path = demo.metahuman_sample_mesh(args.metahuman_sample_dir.expanduser())
    reference_landmark_json = demo.ensure_mediapipe_landmarks(
        mesh_path=reference_mesh_path,
        cache_dir=args.landmark_cache_dir.expanduser(),
        mapper_path=args.mediapipe_mapper.expanduser(),
        blender=args.blender,
        force=args.refresh_landmarks,
        prefer_sidecar=True,
    )
    return demo.load_landmark_mapping(reference_landmark_json)


def build_inference_proxy(
    mesh: Mapping[str, torch.Tensor],
    landmarks: Mapping[int, int],
    *,
    target_faces: int,
    aggression: float,
) -> tuple[
    dict[str, torch.Tensor],
    dict[int, int],
    torch.Tensor,
    dict[str, Any],
]:
    """Decimate a canonical mesh and retain an exact dense-to-proxy map."""

    if not 0.0 <= aggression <= 10.0:
        raise ValueError("Proxy aggression must be in [0, 10].")
    try:
        import fast_simplification
        from fast_simplification.replay import replay_simplification
    except ImportError as exc:
        raise ImportError(
            "Proxy inference requires the fast-simplification package."
        ) from exc

    vertices = mesh["vertices"].detach().cpu().float().numpy()
    faces = mesh["faces"].detach().cpu().long().numpy()
    original_vertex_count = int(vertices.shape[0])
    referenced_vertex_ids = np.unique(faces.reshape(-1))
    unreferenced_vertex_count = original_vertex_count - int(
        referenced_vertex_ids.shape[0]
    )
    source_to_compact = np.full(original_vertex_count, -1, dtype=np.int64)
    source_to_compact[referenced_vertex_ids] = np.arange(
        referenced_vertex_ids.shape[0],
        dtype=np.int64,
    )
    if unreferenced_vertex_count:
        from scipy.spatial import cKDTree

        unreferenced_vertex_ids = np.flatnonzero(source_to_compact < 0)
        compact_tree = cKDTree(vertices[referenced_vertex_ids])
        _distances, nearest_compact = compact_tree.query(
            vertices[unreferenced_vertex_ids],
            k=1,
        )
        source_to_compact[unreferenced_vertex_ids] = nearest_compact.astype(
            np.int64,
            copy=False,
        )
    compact_vertices = vertices[referenced_vertex_ids]
    compact_faces = source_to_compact[faces]
    started = time.monotonic()
    _vertices, _faces, collapses = fast_simplification.simplify(
        compact_vertices,
        compact_faces,
        target_count=int(target_faces),
        agg=float(aggression),
        return_collapses=True,
    )
    proxy_vertices, proxy_faces, compact_to_proxy = replay_simplification(
        compact_vertices,
        compact_faces,
        collapses,
    )
    original_to_proxy = compact_to_proxy[source_to_compact]
    proxy_vertices_tensor = torch.from_numpy(proxy_vertices).float().contiguous()
    proxy_faces_tensor = torch.from_numpy(proxy_faces).long().contiguous()
    dense_to_proxy = torch.from_numpy(original_to_proxy).long().contiguous()
    proxy_landmarks = {
        int(landmark_id): int(dense_to_proxy[int(vertex_id)].item())
        for landmark_id, vertex_id in landmarks.items()
        if 0 <= int(vertex_id) < dense_to_proxy.numel()
    }
    proxy_mesh = {
        "vertices": proxy_vertices_tensor,
        "faces": proxy_faces_tensor,
        "normals": demo.recompute_vertex_normals(
            proxy_vertices_tensor,
            proxy_faces_tensor,
        ),
    }
    proxy_mesh["landmarks_3d"] = train.landmark_payload_from_mapping(
        proxy_mesh,
        proxy_landmarks,
    )
    cluster_sizes = torch.bincount(
        dense_to_proxy,
        minlength=proxy_vertices_tensor.shape[0],
    )
    report = {
        "target_face_count": int(target_faces),
        "aggression": float(aggression),
        "original_vertex_count": original_vertex_count,
        "original_face_count": int(mesh["faces"].shape[0]),
        "referenced_vertex_count": int(referenced_vertex_ids.shape[0]),
        "unreferenced_vertex_count": int(unreferenced_vertex_count),
        "proxy_vertex_count": int(proxy_vertices_tensor.shape[0]),
        "proxy_face_count": int(proxy_faces_tensor.shape[0]),
        "landmark_count": len(proxy_landmarks),
        "unique_landmark_vertex_count": len(set(proxy_landmarks.values())),
        "maximum_dense_cluster_size": int(cluster_sizes.max().item()),
        "build_seconds": time.monotonic() - started,
    }
    return proxy_mesh, proxy_landmarks, dense_to_proxy, report


def smooth_proxy_displacement_transfer(
    displacement: torch.Tensor,
    faces: torch.Tensor,
    *,
    pinned_vertex_ids: Any,
    iterations: int,
    blend: float,
    device: Optional[torch.device] = None,
) -> torch.Tensor:
    """Remove collapse-cluster steps without smoothing across the split seam."""

    if iterations < 0:
        raise ValueError("Proxy transfer smoothing iterations cannot be negative.")
    if not 0.0 <= blend <= 1.0:
        raise ValueError("Proxy transfer smoothing blend must be in [0, 1].")
    if iterations == 0 or blend == 0.0:
        return displacement.contiguous()

    work_device = device or displacement.device
    faces = faces.detach().to(device=work_device, dtype=torch.long).contiguous()
    displacement = displacement.detach().to(
        device=work_device,
        dtype=torch.float32,
    ).contiguous()
    source = torch.cat(
        (
            faces[:, 0], faces[:, 1], faces[:, 2],
            faces[:, 1], faces[:, 2], faces[:, 0],
        )
    )
    target = torch.cat(
        (
            faces[:, 1], faces[:, 2], faces[:, 0],
            faces[:, 0], faces[:, 1], faces[:, 2],
        )
    )
    degree = torch.bincount(target, minlength=displacement.shape[0]).clamp_min(1)
    pinned = torch.zeros(
        displacement.shape[0],
        dtype=torch.bool,
        device=work_device,
    )
    pinned_ids = torch.tensor(
        sorted({int(value) for value in pinned_vertex_ids}),
        dtype=torch.long,
        device=work_device,
    )
    pinned_ids = pinned_ids[
        (pinned_ids >= 0) & (pinned_ids < displacement.shape[0])
    ]
    pinned[pinned_ids] = True
    fixed = displacement.clone()

    for _ in range(iterations):
        neighbor_sum = torch.zeros_like(displacement)
        neighbor_sum.index_add_(0, target, displacement.index_select(0, source))
        neighbor_average = neighbor_sum / degree.to(displacement.dtype).unsqueeze(1)
        displacement = torch.lerp(displacement, neighbor_average, float(blend))
        displacement[pinned] = fixed[pinned]
    return displacement.detach().cpu().contiguous()


def preprocess_split_mesh_for_inference(
    *,
    split_mesh: Mapping[str, torch.Tensor],
    config: Mapping[str, Any],
    mesh_path: Path,
    input_convention: str,
    input_landmarks: Optional[Mapping[int, int]],
    reference_mesh_path: Path,
    reference_landmarks: Optional[Mapping[int, int]],
    reference_input_convention: str,
    min_alignment_landmarks: int,
    alignment_mode: str,
    reference_mesh_cache_path: Optional[Path] = None,
) -> dict[str, torch.Tensor]:
    processed_mesh = train.preprocess_custom_visualization_mesh(
        split_mesh,
        config,
        input_suffix=mesh_path.suffix.lower(),
        input_convention=input_convention,
    )
    if alignment_mode == "none":
        processed_mesh["landmarks_3d"] = train.landmark_payload_from_mapping(
            processed_mesh,
            input_landmarks or {},
        )
        return processed_mesh

    if input_landmarks is None or reference_landmarks is None:
        raise ValueError("Landmark alignment requires input and reference landmarks.")
    reference_path = reference_mesh_path.expanduser().resolve()
    reference_stat = reference_path.stat()
    cache_metadata = {
        "source": str(reference_path),
        "source_size": int(reference_stat.st_size),
        "source_mtime_ns": int(reference_stat.st_mtime_ns),
        "input_convention": str(reference_input_convention),
        "preprocess_settings": demo.mesh_preprocess_settings(config),
    }
    reference_mesh = None
    cache_path = (
        reference_mesh_cache_path.expanduser().resolve()
        if reference_mesh_cache_path is not None
        else None
    )
    if cache_path is not None and cache_path.exists():
        cached = torch.load(cache_path, map_location="cpu", weights_only=False)
        if cached.get("metadata") == cache_metadata:
            reference_mesh = cached.get("mesh")
    if reference_mesh is None:
        reference_mesh = train.preprocess_custom_visualization_mesh(
            demo.load_mesh_for_model(reference_path),
            config,
            input_suffix=reference_path.suffix.lower(),
            input_convention=reference_input_convention,
        )
        if cache_path is not None:
            cache_path.parent.mkdir(parents=True, exist_ok=True)
            temporary_path = cache_path.with_suffix(cache_path.suffix + ".tmp")
            torch.save(
                {
                    "metadata": cache_metadata,
                    "mesh": {
                        key: value.detach().cpu().contiguous()
                        for key, value in reference_mesh.items()
                        if isinstance(value, torch.Tensor)
                    },
                },
                temporary_path,
            )
            temporary_path.replace(cache_path)
    aligned_mesh = train.align_mesh_to_reference_landmarks(
        mesh=processed_mesh,
        input_landmarks=input_landmarks,
        reference_mesh=reference_mesh,
        reference_landmarks=reference_landmarks,
        min_landmarks=min_alignment_landmarks,
        alignment_mode=alignment_mode,
    )
    aligned_mesh["landmarks_3d"] = train.landmark_payload_from_mapping(
        aligned_mesh,
        input_landmarks,
    )
    return aligned_mesh


def validation_colors_with_cavity(
    vertices: torch.Tensor,
    normals: torch.Tensor,
    config: Mapping[str, Any],
    cavity_vertex_count: int,
    cavity_rgb: torch.Tensor,
) -> torch.Tensor:
    batched_vertices = vertices.detach().cpu().float().unsqueeze(0)
    batched_normals = normals.detach().cpu().float().unsqueeze(0)
    render_up_axis = train.resolve_validation_render_up_axis(config, batched_vertices)
    render_vertices = train.vertices_to_render_axes(batched_vertices, render_up_axis)
    render_normals = train.vertices_to_render_axes(batched_normals, render_up_axis)
    colors = train.validation_vertex_colors(
        render_vertices,
        normals=render_normals,
    )[0]
    return apply_cavity_color(colors, cavity_vertex_count, cavity_rgb)


def export_colors_with_cavity(
    vertices: torch.Tensor,
    cavity_vertex_count: int,
    cavity_rgb: torch.Tensor,
) -> torch.Tensor:
    colors = train.validation_vertex_colors(vertices.detach().cpu().float().unsqueeze(0))[0]
    return apply_cavity_color(colors, cavity_vertex_count, cavity_rgb)


def apply_cavity_color(
    colors: torch.Tensor,
    cavity_vertex_count: int,
    cavity_rgb: torch.Tensor,
) -> torch.Tensor:
    colors = colors.clone()
    if cavity_vertex_count > 0:
        colors[-cavity_vertex_count:] = cavity_rgb.to(dtype=colors.dtype)
    return colors.contiguous()


def fit_requested_oral_assembly(
    args: argparse.Namespace,
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    split_landmarks: Mapping[int, int],
):
    return fit_oral_assembly(
        args.metahuman_oral_asset,
        neutral_vertices,
        deformed_vertices,
        split_landmarks,
        horizontal_axis=args.seam_horizontal_axis,
        depth_axis=args.seam_depth_axis,
        vertical_axis=args.seam_vertical_axis,
        assembly_scale=args.oral_assembly_scale,
        depth_offset=args.oral_depth_offset,
        level_target_frame=not args.no_oral_level_target_frame,
        aperture_crop_scale=(
            None if args.no_oral_aperture_crop else args.oral_aperture_crop_scale
        ),
        auto_center_teeth=args.oral_auto_center_teeth,
        maximum_center_offset_ratio=args.oral_maximum_center_offset_ratio,
        center_teeth_vertically=args.oral_center_teeth_vertically,
        tooth_depth_offset=args.oral_tooth_depth_offset,
        tooth_depth_blend_rings=args.oral_tooth_depth_blend_rings,
        auto_center_lower_teeth=args.oral_auto_center_lower_teeth,
        lower_teeth_depth_offset=args.oral_lower_teeth_depth_offset,
        lower_teeth_blend_rings=args.oral_lower_teeth_blend_rings,
        contain_inside_lips=not args.no_oral_containment,
        containment_scale=args.oral_containment_scale,
        containment_depth_inset=args.oral_containment_depth_inset,
        containment_blend_rings=args.oral_containment_blend_rings,
        rim_recession_ratio=args.oral_rim_recession_ratio,
        recessed_shell_horizontal_scale=(
            args.oral_recessed_shell_horizontal_scale
        ),
        recessed_shell_start_depth_ratio=(
            args.oral_recessed_shell_start_depth_ratio
        ),
        recessed_shell_blend_depth_ratio=(
            args.oral_recessed_shell_blend_depth_ratio
        ),
        trim_rim_transition_faces=(
            not args.no_oral_rim_transition_trim
        ),
        rim_transition_maximum_distance_ratio=(
            args.oral_rim_transition_distance_ratio
        ),
        rim_transition_depth_tolerance_ratio=(
            args.oral_rim_transition_depth_tolerance_ratio
        ),
        upper_crown_quantile=args.oral_upper_crown_quantile,
        upper_teeth_vertical_offset=args.oral_upper_teeth_vertical_offset,
        upper_teeth_vertical_blend_rings=(
            args.oral_upper_teeth_vertical_blend_rings
        ),
        remove_oral_wall_uv_faces=args.oral_dental_only,
        remove_tongue_uv_island=args.oral_remove_tongue_uv_island,
        tongue_planar_scale=args.oral_tongue_planar_scale,
        tongue_depth_offset=args.oral_tongue_depth_offset,
        tongue_vertical_offset=args.oral_tongue_vertical_offset,
        tongue_offset_blend_rings=args.oral_tongue_offset_blend_rings,
    )

def apply_oral_vertex_colors(
    colors: torch.Tensor,
    oral_vertex_count: int,
    oral_vertex_colors: Optional[torch.Tensor],
    *,
    trailing_vertex_count: int = 0,
) -> torch.Tensor:
    if oral_vertex_count <= 0:
        return colors.contiguous()
    if oral_vertex_colors is None or oral_vertex_colors.shape != (oral_vertex_count, 3):
        raise ValueError("Oral vertex colors must have shape [oral_vertex_count, 3].")
    if trailing_vertex_count < 0 or oral_vertex_count + trailing_vertex_count > colors.shape[0]:
        raise ValueError("Oral/trailing vertex counts exceed the color array.")
    colors = colors.clone()
    oral_end = colors.shape[0] - trailing_vertex_count
    oral_start = oral_end - oral_vertex_count
    colors[oral_start:oral_end] = oral_vertex_colors.to(dtype=colors.dtype)
    return colors.contiguous()


def apply_component_vertex_color(
    colors: torch.Tensor,
    vertex_count: int,
    rgb: torch.Tensor,
    *,
    trailing_vertex_count: int = 0,
) -> torch.Tensor:
    if vertex_count <= 0:
        return colors.contiguous()
    if trailing_vertex_count < 0 or vertex_count + trailing_vertex_count > colors.shape[0]:
        raise ValueError("Component/trailing vertex counts exceed the color array.")
    colors = colors.clone()
    component_end = colors.shape[0] - trailing_vertex_count
    component_start = component_end - vertex_count
    colors[component_start:component_end] = rgb.to(dtype=colors.dtype)
    return colors.contiguous()


def write_json(path: Path, payload: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


if __name__ == "__main__":
    main()


```

## Source file: model/toporig.py

{
  "path": "model/toporig.py",
  "blob_sha": "c628b1e5afcfdd44fd04f06fb47c150140c720de",
  "sha256": "48d3c47777ea188ac993fb443e4a80067f4bdc0943385557af74491d86e37037",
  "lines": 730,
  "bytes": 25891,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 1,
    "deform": 8,
    "facs": 30,
    "rig": 7,
    "face": 46
  },
  "url": "https://github.com/TaatiTeam/TopoRig/blob/4b97e0770eb29bea90235641f62940e6a583bb1c/model/toporig.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def _faces_to_edges("
    },
    {
      "line": 46,
      "text": "def _edge_mean("
    },
    {
      "line": 61,
      "text": "def _dense_adj_mean(x: torch.Tensor, adjacency: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 70,
      "text": "def _neighbor_mean("
    },
    {
      "line": 103,
      "text": "def __init__("
    },
    {
      "line": 130,
      "text": "def forward(self, x: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 137,
      "text": "def __init__(self, feature_dim: int, dropout: float = 0.0) -> None:"
    },
    {
      "line": 148,
      "text": "def forward("
    },
    {
      "line": 169,
      "text": "def __init__("
    },
    {
      "line": 192,
      "text": "def forward("
    },
    {
      "line": 217,
      "text": "def __init__("
    },
    {
      "line": 243,
      "text": "def forward("
    },
    {
      "line": 290,
      "text": "def __init__("
    },
    {
      "line": 396,
      "text": "def forward("
    },
    {
      "line": 512,
      "text": "def _forward_single_pose("
    },
    {
      "line": 565,
      "text": "def _parse_position_encoding("
    },
    {
      "line": 590,
      "text": "def _vertex_features("
    },
    {
      "line": 631,
      "text": "def _normalized_vertices(vertices: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 640,
      "text": "def _fourier_position_features("
    },
    {
      "line": 659,
      "text": "def _initialize_output_layer("
    },
    {
      "line": 686,
      "text": "def _test() -> None:"
    }
  ]
}

```text

from __future__ import annotations

import math
from typing import Any, Mapping, Optional, Tuple

import torch
from torch import nn
import torch.nn.functional as F


def _faces_to_edges(
    faces: torch.Tensor,
    num_vertices: int,
    device: torch.device,
) -> Tuple[torch.Tensor, torch.Tensor]:
    faces = faces.to(device=device, dtype=torch.long)
    if faces.numel() == 0:
        idx = torch.arange(num_vertices, device=device)
        return idx, idx

    valid = (faces >= 0).all(dim=-1)
    faces = faces[valid]
    if faces.numel() == 0:
        idx = torch.arange(num_vertices, device=device)
        return idx, idx

    i, j, k = faces.unbind(dim=-1)
    src = torch.cat([i, j, j, k, k, i], dim=0)
    dst = torch.cat([j, i, k, j, i, k], dim=0)

    in_range = (
        (src >= 0)
        & (src < num_vertices)
        & (dst >= 0)
        & (dst < num_vertices)
    )
    src = src[in_range]
    dst = dst[in_range]

    self_edges = torch.arange(num_vertices, device=device)
    src = torch.cat([src, self_edges], dim=0)
    dst = torch.cat([dst, self_edges], dim=0)
    return src, dst


def _edge_mean(
    x: torch.Tensor,
    edge_index: Tuple[torch.Tensor, torch.Tensor],
) -> torch.Tensor:
    src, dst = edge_index
    batch_size, num_vertices, channels = x.shape

    out = x.new_zeros(batch_size, num_vertices, channels)
    out.index_add_(1, dst, x[:, src, :])

    degree = x.new_zeros(num_vertices)
    degree.index_add_(0, dst, torch.ones_like(dst, dtype=x.dtype))
    return out / degree.clamp_min(1.0).view(1, num_vertices, 1)


def _dense_adj_mean(x: torch.Tensor, adjacency: torch.Tensor) -> torch.Tensor:
    if adjacency.dim() == 2:
        adjacency = adjacency.unsqueeze(0).expand(x.shape[0], -1, -1)

    adjacency = adjacency.to(device=x.device, dtype=x.dtype)
    degree = adjacency.sum(dim=-1, keepdim=True).clamp_min(1.0)
    return torch.bmm(adjacency / degree, x)


def _neighbor_mean(
    x: torch.Tensor,
    faces: Optional[torch.Tensor] = None,
    adjacency: Optional[torch.Tensor] = None,
    edge_index: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
) -> torch.Tensor:
    if adjacency is not None:
        return _dense_adj_mean(x, adjacency)

    if edge_index is not None:
        return _edge_mean(x, edge_index)

    if faces is None:
        return x

    if faces.dim() == 2:
        edge_index = _faces_to_edges(faces, x.shape[1], x.device)
        return _edge_mean(x, edge_index)

    if faces.dim() != 3:
        raise ValueError("faces must have shape [F, 3] or [B, F, 3].")

    if faces.shape[0] != x.shape[0]:
        raise ValueError("batched faces must have the same batch size as x.")

    outputs = []
    for batch_idx in range(x.shape[0]):
        batch_edges = _faces_to_edges(faces[batch_idx], x.shape[1], x.device)
        outputs.append(_edge_mean(x[batch_idx : batch_idx + 1], batch_edges))
    return torch.cat(outputs, dim=0)


class MLP(nn.Module):
    def __init__(
        self,
        in_dim: int,
        hidden_dim: int,
        out_dim: int,
        num_layers: int = 2,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        if num_layers < 1:
            raise ValueError("num_layers must be at least 1.")

        if num_layers == 1:
            dims = [in_dim, out_dim]
        else:
            dims = [in_dim] + [hidden_dim] * (num_layers - 1) + [out_dim]

        layers = []
        for idx in range(len(dims) - 1):
            layers.append(nn.Linear(dims[idx], dims[idx + 1]))
            if idx < len(dims) - 2:
                layers.append(nn.LayerNorm(dims[idx + 1]))
                layers.append(nn.SiLU())
                if dropout > 0:
                    layers.append(nn.Dropout(dropout))
        self.net = nn.Sequential(*layers)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


class SurfaceDiffusionBlock(nn.Module):
    """A lightweight DiffusionNet-like block for per-vertex mesh features."""

    def __init__(self, feature_dim: int, dropout: float = 0.0) -> None:
        super().__init__()
        self.spatial_mlp = MLP(
            in_dim=feature_dim * 3,
            hidden_dim=feature_dim,
            out_dim=feature_dim,
            num_layers=2,
            dropout=dropout,
        )
        self.norm = nn.LayerNorm(feature_dim)

    def forward(
        self,
        x: torch.Tensor,
        faces: Optional[torch.Tensor] = None,
        adjacency: Optional[torch.Tensor] = None,
        edge_index: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
    ) -> torch.Tensor:
        diffused = _neighbor_mean(
            x,
            faces=faces,
            adjacency=adjacency,
            edge_index=edge_index,
        )
        gradient = diffused - x
        update = self.spatial_mlp(torch.cat([x, diffused, gradient], dim=-1))
        return self.norm(x + update)


class ConditionalDiffusionBlock(nn.Module):
    """Diffusion block with FACS/global conditioning injected per vertex."""

    def __init__(
        self,
        feature_dim: int,
        condition_dim: int,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        self.spatial_mlp = MLP(
            in_dim=feature_dim * 3,
            hidden_dim=feature_dim,
            out_dim=feature_dim,
            num_layers=2,
            dropout=dropout,
        )
        self.condition_mlp = MLP(
            in_dim=feature_dim + condition_dim,
            hidden_dim=feature_dim,
            out_dim=feature_dim,
            num_layers=2,
            dropout=dropout,
        )
        self.norm = nn.LayerNorm(feature_dim)

    def forward(
        self,
        x: torch.Tensor,
        condition: torch.Tensor,
        faces: Optional[torch.Tensor] = None,
        adjacency: Optional[torch.Tensor] = None,
        edge_index: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
    ) -> torch.Tensor:
        diffused = _neighbor_mean(
            x,
            faces=faces,
            adjacency=adjacency,
            edge_index=edge_index,
        )
        gradient = diffused - x
        spatial = self.spatial_mlp(torch.cat([x, diffused, gradient], dim=-1))

        condition = condition.unsqueeze(1).expand(-1, x.shape[1], -1)
        update = self.condition_mlp(torch.cat([spatial, condition], dim=-1))
        return self.norm(x + update)


class GlobalEncoder(nn.Module):
    """Two-layer mesh encoder with global average pooling, as in Fig. 3."""

    def __init__(
        self,
        input_dim: int,
        hidden_dim: int,
        global_dim: int,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        self.input_mlp = MLP(
            in_dim=input_dim,
            hidden_dim=hidden_dim,
            out_dim=hidden_dim,
            num_layers=2,
            dropout=dropout,
        )
        self.blocks = nn.ModuleList(
            [SurfaceDiffusionBlock(hidden_dim, dropout=dropout) for _ in range(2)]
        )
        self.output_mlp = MLP(
            in_dim=hidden_dim,
            hidden_dim=hidden_dim,
            out_dim=global_dim,
            num_layers=2,
            dropout=dropout,
        )

    def forward(
        self,
        vertex_features: torch.Tensor,
        faces: Optional[torch.Tensor] = None,
        adjacency: Optional[torch.Tensor] = None,
        edge_index: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
    ) -> torch.Tensor:
        x = self.input_mlp(vertex_features)
        for block in self.blocks:
            x = block(
                x,
                faces=faces,
                adjacency=adjacency,
                edge_index=edge_index,
            )
        return self.output_mlp(x).mean(dim=1)


class TopoRig(nn.Module):
    """
    Mesh deformation network inspired by the RigAnyFace/RAF Fig. 3 architecture.

    Args:
        facs_dim: Number of FACS/action-unit controls in the pose vector.
        hidden_dim: Width of the main conditional diffusion stack.
        num_blocks: Number of conditional diffusion blocks in the main network.
        use_global_encoder: If True, encode the neutral mesh with a 2-layer
            global encoder and concatenate that embedding with the FACS vector.
        global_dim: Size of the optional global mesh embedding.
        condition_dim: Projected latent size injected into each diffusion block.
        input_dim: Per-vertex input feature size before positional encoding. The
            default expects position and normal concatenated as
            [x, y, z, nx, ny, nz]. Larger values reserve the remaining channels
            for optional auxiliary inputs such as landmark-relative features.
        position_encoding: Optional topology-agnostic positional encoding config.
            Set ``{"type": "fourier", "num_frequencies": 8}`` to concatenate
            Fourier features computed from per-mesh normalized coordinates.
        use_conditioned_output_head: If True, concatenate condition features into
            the final output MLP. Set False only for loading legacy checkpoints
            trained before the output head received condition features.
        output_scale: Multiplier applied to predicted displacements.
        output_init: Initialization for the final displacement layer. ``"zero"``
            starts new models at neutral displacement, which is usually the
            right prior for small blendshape deltas.
        output_init_std: Standard deviation used when ``output_init="normal"``.
    """

    def __init__(
        self,
        facs_dim: int = 96,
        hidden_dim: int = 256,
        num_blocks: int = 6,
        use_global_encoder: bool = False,
        global_dim: int = 128,
        condition_dim: int = 128,
        input_dim: int = 6,
        dropout: float = 0.0,
        output_scale: float = 1.0,
        output_init: str = "zero",
        output_init_std: float = 1.0e-5,
        position_encoding: Optional[Mapping[str, Any] | str] = None,
        use_conditioned_output_head: bool = True,
    ) -> None:
        super().__init__()
        input_dim = int(input_dim)
        if input_dim < 6:
            raise ValueError(
                "TopoRig requires input_dim >= 6 for base vertex features "
                "[x, y, z, nx, ny, nz]."
            )
        self.input_dim = input_dim
        self.auxiliary_input_dim = input_dim - 6
        self.facs_dim = facs_dim
        self.use_global_encoder = use_global_encoder
        self.use_conditioned_output_head = bool(use_conditioned_output_head)
        self.output_scale = float(output_scale)
        (
            self.position_encoding_type,
            self.position_encoding_num_frequencies,
            self.position_encoding_include_raw,
            self.normalize_base_positions,
        ) = self._parse_position_encoding(position_encoding)
        self.position_encoding_frequency_scale = math.pi

        fourier_dim = 0
        if self.position_encoding_type == "fourier":
            fourier_dim = 3 * 2 * self.position_encoding_num_frequencies
            if self.position_encoding_include_raw:
                fourier_dim += 3
            frequencies = torch.pow(
                2.0,
                torch.arange(self.position_encoding_num_frequencies).float(),
            )
        else:
            frequencies = torch.empty(0)
        self.register_buffer(
            "position_encoding_frequencies",
            frequencies,
            persistent=False,
        )
        model_input_dim = input_dim + fourier_dim

        self.vertex_mlp = MLP(
            in_dim=model_input_dim,
            hidden_dim=hidden_dim,
            out_dim=hidden_dim,
            num_layers=3,
            dropout=dropout,
        )

        self.global_encoder: Optional[GlobalEncoder]
        if use_global_encoder:
            self.global_encoder = GlobalEncoder(
                input_dim=model_input_dim,
                hidden_dim=max(hidden_dim // 2, condition_dim),
                global_dim=global_dim,
                dropout=dropout,
            )
            latent_dim = facs_dim + global_dim
        else:
            self.global_encoder = None
            latent_dim = facs_dim

        self.condition_mlp = MLP(
            in_dim=latent_dim,
            hidden_dim=condition_dim,
            out_dim=condition_dim,
            num_layers=2,
            dropout=dropout,
        )

        self.blocks = nn.ModuleList(
            [
                ConditionalDiffusionBlock(
                    feature_dim=hidden_dim,
                    condition_dim=condition_dim,
                    dropout=dropout,
                )
                for _ in range(num_blocks)
            ]
        )
        output_input_dim = hidden_dim
        if self.use_conditioned_output_head:
            output_input_dim += condition_dim
        self.output_mlp = MLP(
            in_dim=output_input_dim,
            hidden_dim=hidden_dim,
            out_dim=3,
            num_layers=3,
            dropout=dropout,
        )
        self._initialize_output_layer(output_init, output_init_std)

    def forward(
        self,
        vertices: torch.Tensor,
        facs: torch.Tensor,
        normals: Optional[torch.Tensor] = None,
        faces: Optional[torch.Tensor] = None,
        adjacency: Optional[torch.Tensor] = None,
        landmark_features: Optional[torch.Tensor] = None,
        return_deformed: bool = False,
    ) -> torch.Tensor | Tuple[torch.Tensor, torch.Tensor]:
        """
        Predict per-vertex 3D displacement.

        Args:
            vertices: Neutral mesh vertices with shape [B, V, 3].
            facs: FACS pose vector with shape [B, C] or [B, P, C].
            normals: Optional per-vertex normals with shape [B, V, 3].
            faces: Optional triangle indices with shape [F, 3] or [B, F, 3].
            adjacency: Optional dense adjacency with shape [V, V] or [B, V, V].
            landmark_features: Optional auxiliary per-vertex features with shape
                [B, V, input_dim - 6].
            return_deformed: If True, return (deformed_vertices, displacement).

        Returns:
            Displacement with shape [B, V, 3] for one pose or [B, P, V, 3]
            for multiple poses. If return_deformed=True, the first item is
            vertices + displacement with the same output shape.
        """
        if vertices.dim() != 3 or vertices.shape[-1] != 3:
            raise ValueError("vertices must have shape [B, V, 3].")

        if normals is None:
            normals = torch.zeros_like(vertices)
        elif normals.shape != vertices.shape:
            raise ValueError("normals must have the same shape as vertices.")
        else:
            normals = normals.to(device=vertices.device, dtype=vertices.dtype)
            normals = F.normalize(normals, dim=-1, eps=1e-6)

        if facs.dim() == 2:
            displacement = self._forward_single_pose(
                vertices=vertices,
                normals=normals,
                facs=facs,
                faces=faces,
                adjacency=adjacency,
                landmark_features=landmark_features,
            )
            if return_deformed:
                return vertices + displacement, displacement
            return displacement

        if facs.dim() != 3:
            raise ValueError("facs must have shape [B, C] or [B, P, C].")

        batch_size, num_poses, facs_dim = facs.shape
        if batch_size != vertices.shape[0]:
            raise ValueError("facs and vertices must have the same batch size.")

        vertices_rep = (
            vertices.unsqueeze(1)
            .expand(batch_size, num_poses, -1, -1)
            .reshape(batch_size * num_poses, vertices.shape[1], 3)
        )
        normals_rep = (
            normals.unsqueeze(1)
            .expand(batch_size, num_poses, -1, -1)
            .reshape(batch_size * num_poses, normals.shape[1], 3)
        )
        landmark_features_rep = None
        if landmark_features is not None:
            if landmark_features.dim() != 3 or landmark_features.shape[:2] != vertices.shape[:2]:
                raise ValueError(
                    "landmark_features must have shape [B, V, input_dim - 6]."
                )
            landmark_features_rep = (
                landmark_features.unsqueeze(1)
                .expand(batch_size, num_poses, -1, -1)
                .reshape(
                    batch_size * num_poses,
                    landmark_features.shape[1],
                    landmark_features.shape[2],
                )
            )
        facs_rep = facs.reshape(batch_size * num_poses, facs_dim)

        faces_rep = faces
        if faces is not None and faces.dim() == 3:
            faces_rep = (
                faces.unsqueeze(1)
                .expand(batch_size, num_poses, -1, -1)
                .reshape(batch_size * num_poses, faces.shape[1], 3)
            )

        adjacency_rep = adjacency
        if adjacency is not None and adjacency.dim() == 3:
            adjacency_rep = (
                adjacency.unsqueeze(1)
                .expand(batch_size, num_poses, -1, -1)
                .reshape(batch_size * num_poses, adjacency.shape[1], adjacency.shape[2])
            )

        displacement = self._forward_single_pose(
            vertices=vertices_rep,
            normals=normals_rep,
            facs=facs_rep,
            faces=faces_rep,
            adjacency=adjacency_rep,
            landmark_features=landmark_features_rep,
        ).reshape(batch_size, num_poses, vertices.shape[1], 3)

        if return_deformed:
            deformed = vertices.unsqueeze(1) + displacement
            return deformed, displacement
        return displacement

    def _forward_single_pose(
        self,
        vertices: torch.Tensor,
        normals: torch.Tensor,
        facs: torch.Tensor,
        faces: Optional[torch.Tensor],
        adjacency: Optional[torch.Tensor],
        landmark_features: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        if facs.dim() != 2 or facs.shape[-1] != self.facs_dim:
            raise ValueError(f"facs must have shape [B, {self.facs_dim}].")
        if facs.shape[0] != vertices.shape[0]:
            raise ValueError("facs and vertices must have the same batch size.")

        faces = faces.to(vertices.device) if faces is not None else None
        adjacency = adjacency.to(vertices.device) if adjacency is not None else None
        edge_index = None
        if adjacency is None and faces is not None and faces.dim() == 2:
            edge_index = _faces_to_edges(faces, vertices.shape[1], vertices.device)

        vertex_features = self._vertex_features(
            vertices,
            normals,
            landmark_features=landmark_features,
        )
        x = self.vertex_mlp(vertex_features)

        latent = facs.to(device=vertices.device, dtype=vertices.dtype)
        if self.global_encoder is not None:
            global_embedding = self.global_encoder(
                vertex_features,
                faces=faces,
                adjacency=adjacency,
                edge_index=edge_index,
            )
            latent = torch.cat([latent, global_embedding], dim=-1)
        condition = self.condition_mlp(latent)

        for block in self.blocks:
            x = block(
                x,
                condition=condition,
                faces=faces,
                adjacency=adjacency,
                edge_index=edge_index,
            )
        output_features = x
        if self.use_conditioned_output_head:
            condition_features = condition.unsqueeze(1).expand(-1, x.shape[1], -1)
            output_features = torch.cat((x, condition_features), dim=-1)
        return self.output_mlp(output_features) * self.output_scale

    @staticmethod
    def _parse_position_encoding(
        config: Optional[Mapping[str, Any] | str],
    ) -> tuple[str, int, bool, bool]:
        if config is None:
            return "none", 0, False, False
        if isinstance(config, str):
            config = {"type": config}
        if not isinstance(config, Mapping):
            raise ValueError("position_encoding must be a mapping, string, or null.")

        encoding_type = str(config.get("type", "none")).lower()
        if encoding_type in {"none", "off", "disabled", "false"}:
            return "none", 0, False, bool(config.get("normalize_positions", False))
        if encoding_type not in {"fourier", "sinusoidal"}:
            raise ValueError(
                "position_encoding.type must be one of: none, fourier, sinusoidal."
            )

        num_frequencies = int(config.get("num_frequencies", 8))
        if num_frequencies < 1:
            raise ValueError("position_encoding.num_frequencies must be at least 1.")
        include_raw = bool(config.get("include_raw", False))
        normalize_positions = bool(config.get("normalize_positions", True))
        return "fourier", num_frequencies, include_raw, normalize_positions

    def _vertex_features(
        self,
        vertices: torch.Tensor,
        normals: torch.Tensor,
        landmark_features: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        normalized_vertices = self._normalized_vertices(vertices)
        positions = normalized_vertices if self.normalize_base_positions else vertices
        features = [positions, normals]
        if self.auxiliary_input_dim > 0:
            if landmark_features is None:
                landmark_features = vertices.new_zeros(
                    vertices.shape[0],
                    vertices.shape[1],
                    self.auxiliary_input_dim,
                )
            elif landmark_features.shape != (
                vertices.shape[0],
                vertices.shape[1],
                self.auxiliary_input_dim,
            ):
                raise ValueError(
                    "landmark_features must have shape "
                    f"[B, V, {self.auxiliary_input_dim}]."
                )
            else:
                landmark_features = landmark_features.to(
                    device=vertices.device,
                    dtype=vertices.dtype,
                )
            features.append(landmark_features)
        elif landmark_features is not None and landmark_features.numel() > 0:
            raise ValueError(
                "landmark_features were provided, but input_dim does not reserve "
                "auxiliary channels."
            )
        if self.position_encoding_type == "fourier":
            features.append(self._fourier_position_features(normalized_vertices))
        return torch.cat(features, dim=-1)

    @staticmethod
    def _normalized_vertices(vertices: torch.Tensor) -> torch.Tensor:
        center = vertices.mean(dim=1, keepdim=True)
        centered = vertices - center
        scale = torch.linalg.vector_norm(centered, dim=-1).amax(
            dim=1,
            keepdim=True,
        )
        return centered / scale.clamp_min(1.0e-6).unsqueeze(-1)

    def _fourier_position_features(
        self,
        normalized_vertices: torch.Tensor,
    ) -> torch.Tensor:
        frequencies = self.position_encoding_frequencies.to(
            device=normalized_vertices.device,
            dtype=normalized_vertices.dtype,
        )
        angles = (
            normalized_vertices.unsqueeze(-2)
            * frequencies.view(1, 1, -1, 1)
            * self.position_encoding_frequency_scale
        )
        encoded = torch.cat((torch.sin(angles), torch.cos(angles)), dim=-2)
        encoded = encoded.flatten(start_dim=-2)
        if self.position_encoding_include_raw:
            encoded = torch.cat((normalized_vertices, encoded), dim=-1)
        return encoded

    def _initialize_output_layer(
        self,
        output_init: str,
        output_init_std: float,
    ) -> None:
        output_init = output_init.lower()
        if output_init == "default":
            return

        final_linear = None
        for module in self.output_mlp.net.modules():
            if isinstance(module, nn.Linear):
                final_linear = module
        if final_linear is None:
            raise RuntimeError("TopoRig output MLP does not contain a Linear layer.")

        if output_init == "zero":
            nn.init.zeros_(final_linear.weight)
        elif output_init == "normal":
            nn.init.normal_(final_linear.weight, mean=0.0, std=float(output_init_std))
        else:
            raise ValueError(
                "output_init must be one of: 'zero', 'normal', or 'default'."
            )
        nn.init.zeros_(final_linear.bias)


def _test() -> None:
    torch.manual_seed(7)

    batch_size = 2
    num_vertices = 128
    facs_dim = 96

    vertices = torch.randn(batch_size, num_vertices, 3)
    normals = F.normalize(torch.randn(batch_size, num_vertices, 3), dim=-1)
    facs = torch.randn(batch_size, facs_dim)

    face_ids = torch.arange(num_vertices - 2)
    faces = torch.stack([face_ids, face_ids + 1, face_ids + 2], dim=-1)

    model = TopoRig(
        facs_dim=facs_dim,
        hidden_dim=64,
        num_blocks=2,
        use_global_encoder=False,
    )
    output = model(vertices, facs, normals=normals, faces=faces)

    model_with_global = TopoRig(
        facs_dim=facs_dim,
        hidden_dim=64,
        num_blocks=2,
        use_global_encoder=True,
    )
    output_with_global = model_with_global(
        vertices,
        facs,
        normals=normals,
        faces=faces,
    )

    print(f"vertices input shape: {tuple(vertices.shape)}")
    print(f"normals input shape:  {tuple(normals.shape)}")
    print(f"facs input shape:     {tuple(facs.shape)}")
    print(f"faces input shape:    {tuple(faces.shape)}")
    print(f"output shape:         {tuple(output.shape)}")
    print(f"global output shape:  {tuple(output_with_global.shape)}")


if __name__ == "__main__":
    _test()


```

## Source file: utils/lip_split.py

{
  "path": "utils/lip_split.py",
  "blob_sha": "6f094a8cda3de0f38198b7ea1272a83577f2f528",
  "sha256": "21fbae5589f83be36c50a1cf23ffc584154daba909e57e9526f6daa798ae918a",
  "lines": 5440,
  "bytes": 217107,
  "selection_score": 15,
  "keyword_line_counts": {
    "jaw": 10,
    "mouth": 156,
    "teeth": 1,
    "tooth": 1,
    "tongue": 2,
    "deform": 190,
    "iris": 1,
    "rig": 84,
    "face": 550,
    "lip": 374
  },
  "url": "https://github.com/TaatiTeam/TopoRig/blob/4b97e0770eb29bea90235641f62940e6a583bb1c/utils/lip_split.py",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 76,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 106,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 119,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 139,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 162,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 181,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 205,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 228,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 247,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 266,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 286,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 312,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 328,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 347,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 364,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 390,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 415,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 435,
      "text": "def to_dict(self) -> dict[str, Any]:"
    },
    {
      "line": 439,
      "text": "def detect_jaw_open_aperture_faces("
    },
    {
      "line": 596,
      "text": "def build_mouth_cavity_disk("
    },
    {
      "line": 791,
      "text": "def build_mouth_boundary_lining("
    },
    {
      "line": 937,
      "text": "def projected_component_area(component: list[int]) -> float:"
    },
    {
      "line": 1145,
      "text": "def build_mediapipe_lip_annulus("
    },
    {
      "line": 1233,
      "text": "def sample_curve("
    },
    {
      "line": 1267,
      "text": "def closed_ring(upper: torch.Tensor, lower: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 1288,
      "text": "def build_ring_pair("
    },
    {
      "line": 1348,
      "text": "def surface_rows(outer: torch.Tensor, inner: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 1417,
      "text": "def clear_source_faces_under_lip_annulus("
    },
    {
      "line": 1499,
      "text": "def orient_front_surface_faces("
    },
    {
      "line": 1595,
      "text": "def detect_stretched_mouth_faces("
    },
    {
      "line": 1750,
      "text": "def retain_aperture_connected_face_removals("
    },
    {
      "line": 1799,
      "text": "def find(index: int) -> int:"
    },
    {
      "line": 1805,
      "text": "def union(left: int, right: int) -> None:"
    },
    {
      "line": 1884,
      "text": "def _undirected_cycle_basis("
    },
    {
      "line": 1920,
      "text": "def _ear_clip_projected_cycle("
    },
    {
      "line": 1939,
      "text": "def cross(first: int, second: int, third: int) -> float:"
    },
    {
      "line": 1946,
      "text": "def inside_triangle("
    },
    {
      "line": 1983,
      "text": "def _mesh_boundary_edges(faces: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 2009,
      "text": "def _new_boundary_edge_mask("
    },
    {
      "line": 2048,
      "text": "def fill_secondary_mouth_boundary_holes("
    },
    {
      "line": 2457,
      "text": "def bridge_excess_mouth_corner_gap("
    },
    {
      "line": 2552,
      "text": "def ordered_cycle(component: list[int]) -> list[int]:"
    },
    {
      "line": 2586,
      "text": "def projected_cycle_area(cycle: list[int]) -> float:"
    },
    {
      "line": 2630,
      "text": "def longest_corner_chain(cycle: list[int], side: str) -> list[int]:"
    },
    {
      "line": 2681,
      "text": "def triangle_quality(vertex_ids: list[int]) -> tuple[float, bool, bool, bool]:"
    },
    {
      "line": 2877,
      "text": "def smooth_mouth_boundary_loops("
    },
    {
      "line": 3045,
      "text": "def boundary_displacement(vertices: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 3155,
      "text": "def diffuse(seed: torch.Tensor) -> torch.Tensor:"
    },
    {
      "line": 3264,
      "text": "def clip_mesh_to_smooth_mouth_contour("
    }
  ]
}

```text

from __future__ import annotations

import json
import math
from collections import Counter, deque
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence

import numpy as np
import torch
import torch.nn.functional as F


INNER_UPPER_LIP_MEDIAPIPE_IDS = (78, 191, 80, 81, 82, 13, 312, 311, 310, 415, 308)
INNER_LOWER_LIP_MEDIAPIPE_IDS = (78, 95, 88, 178, 87, 14, 317, 402, 318, 324, 308)
OUTER_UPPER_LIP_MEDIAPIPE_IDS = (61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291)
OUTER_LOWER_LIP_MEDIAPIPE_IDS = (61, 146, 91, 181, 84, 17, 314, 405, 321, 375, 291)

# Corresponding points along the closed inner-mouth contact line.  Averaging
# each upper/lower pair is more stable than choosing either semantic mask on a
# neutral mesh whose lips touch or share vertices.
INNER_LIP_CENTER_PAIRS = (
    (78, 78),
    (191, 95),
    (80, 88),
    (81, 178),
    (82, 87),
    (13, 14),
    (312, 317),
    (311, 402),
    (310, 318),
    (415, 324),
    (308, 308),
)


@dataclass(frozen=True)
class LipRegions:
    upper_vertex_ids: torch.Tensor
    lower_vertex_ids: torch.Tensor
    source: str = "unknown"

    def to_dict(self) -> dict[str, Any]:
        return {
            "source": self.source,
            "upper_vertex_count": int(self.upper_vertex_ids.numel()),
            "lower_vertex_count": int(self.lower_vertex_ids.numel()),
            "upper_vertex_ids": self.upper_vertex_ids.detach().cpu().long().tolist(),
            "lower_vertex_ids": self.lower_vertex_ids.detach().cpu().long().tolist(),
        }


@dataclass(frozen=True)
class LipSplitReport:
    original_vertex_count: int
    original_face_count: int
    final_vertex_count: int
    final_face_count: int
    upper_vertex_count: int
    lower_vertex_count: int
    shared_vertex_count: int
    upper_face_count: int
    lower_face_count: int
    neutral_face_count: int
    mixed_face_count: int
    removed_face_count: int
    duplicated_vertex_count: int
    direct_lip_connection_faces_before: int
    direct_lip_connection_edges_before: int
    direct_lip_connection_faces_after: int
    direct_lip_connection_edges_after: int
    lower_duplicate_vertex_ids: dict[int, int]
    removed_face_ids: list[int]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class GeometricLipSeamReport:
    original_vertex_count: int
    original_face_count: int
    final_vertex_count: int
    final_face_count: int
    curve_point_count: int
    candidate_face_count: int
    upper_candidate_face_count: int
    lower_candidate_face_count: int
    seam_vertex_count: int
    duplicated_vertex_count: int
    remapped_lower_face_count: int
    mixed_original_duplicate_face_count: int
    horizontal_axis: int
    depth_axis: int
    vertical_axis: int
    depth_tolerance: float
    vertical_tolerance: float
    horizontal_padding: float
    corner_inset_ratio: float
    preopen_distance: float
    curve_points: list[list[float]]
    seam_vertex_ids: list[int]
    lower_duplicate_vertex_ids: dict[int, int]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class CoincidentVertexWeldReport:
    original_vertex_count: int
    final_vertex_count: int
    merged_vertex_count: int
    original_face_count: int
    final_face_count: int
    removed_degenerate_face_count: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class LipLandmarkTopologyRepairReport:
    input_landmark_count: int
    lip_landmark_count: int
    valid_lip_landmark_count: int
    invalid_lip_landmark_count: int
    connected_component_count: int
    dominant_component_vertex_count: int
    off_component_lip_landmark_count: int
    spatial_outlier_lip_landmark_count: int
    snapped_lip_landmark_count: int
    interpolated_lip_landmark_count: int
    skipped_lip_landmark_count: int
    maximum_snap_distance: float
    mean_snap_distance: float

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class LipLandmarkSurfaceSnapReport:
    input_landmark_count: int
    lip_landmark_count: int
    connected_component_count: int
    facial_component_id: int
    facial_component_vertex_count: int
    facial_component_landmark_votes: int
    off_component_lip_landmark_count: int
    duplicate_lip_landmark_count_before: int
    duplicate_lip_landmark_count_after: int
    snapped_lip_landmark_count: int
    skipped_lip_landmark_count: int
    maximum_projected_distance_ratio: float
    mean_projected_distance_ratio: float
    minimum_upper_lower_gap: float
    front_axis: int
    front_direction: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class JawOpenApertureCutReport:
    aperture_scale: float
    overlap_inset_scale: float
    depth_padding: float
    front_direction: int
    polygon_point_count: int
    inside_aperture_face_count: int
    overlap_aperture_face_count: int
    removed_face_count: int
    original_face_count: int
    final_face_count: int
    aperture_polygon: list[list[float]]
    removed_face_ids: list[int]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthCavityReport:
    vertex_count: int
    face_count: int
    segment_count: int
    horizontal_axis: int
    depth_axis: int
    vertical_axis: int
    front_direction: int
    scale: float
    shape_exponent: float
    depth_offset: float
    radial_ring_count: int
    curvature_depth: float
    inner_radius_ratio: float
    side_wall_min_horizontal_ratio: float
    center: list[float]
    radius_horizontal: float
    radius_vertical: float

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthBoundaryLiningReport:
    vertex_count: int
    face_count: int
    boundary_cycle_count: int
    boundary_vertex_count: int
    depth: float
    front_direction: int
    side_fraction: float
    inner_scale: float
    radial_ring_count: int
    smoothing_iterations: int
    analytic_segments: int | None
    opening_scale: float
    shape_exponent: float
    boundary_source: str
    cap_back: bool
    maximum_cycles: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class StretchedMouthFaceReport:
    edge_ratio_threshold: float
    edge_growth_threshold: float
    neighbor_growth_rings: int
    neighbor_edge_ratio_threshold: float
    neighbor_edge_growth_threshold: float
    region_scale: float
    depth_padding: float
    candidate_face_count: int
    strict_removed_face_count: int
    grown_removed_face_count: int
    removed_face_count: int
    removed_face_ids: list[int]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthRemovalConnectivityReport:
    aperture_seed_face_count: int
    stretch_candidate_face_count: int
    candidate_face_count: int
    connected_component_count: int
    seeded_component_count: int
    significant_seeded_component_count: int
    discarded_aperture_component_count: int
    discarded_aperture_face_count: int
    removed_face_count: int
    restored_disconnected_component_count: int
    restored_disconnected_face_count: int
    removed_face_ids: list[int]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthContourClipReport:
    source_face_count: int
    input_kept_face_count: int
    final_face_count: int
    original_vertex_count: int
    final_vertex_count: int
    clipped_face_count: int
    restored_boundary_face_count: int
    removed_inside_face_count: int
    retained_outside_face_count: int
    added_vertex_count: int
    smooth_curve_sample_count: int
    contour_scale: float
    depth_padding: float

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class LipAnnulusRepairReport:
    vertex_count: int
    face_count: int
    segment_count: int
    radial_ring_count: int
    outer_scale: float
    thickness_ratio: float
    side_thickness_factor: float
    maximum_radial_inset_fraction: float
    inner_recess_depth: float
    front_offset: float
    bulge_depth: float
    capped_neutral_segment_count: int
    capped_deformed_segment_count: int
    upper_smoothing_window: int
    lower_smoothing_window: int
    front_direction: int
    horizontal_axis: int
    depth_axis: int
    vertical_axis: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class FrontSurfaceOrientationReport:
    face_count: int
    eligible_face_count: int
    flipped_face_count: int
    front_direction: int
    mouth_depth: float
    front_depth_limit: float
    region_depth_ratio: float
    normal_tolerance: float
    depth_axis: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class LipAnnulusHeadCleanupReport:
    original_face_count: int
    removed_face_count: int
    final_face_count: int
    projected_overlap_face_count: int
    depth_rejected_overlap_face_count: int
    polygon_point_count: int
    depth_padding_ratio: float
    depth_min: float
    depth_max: float
    horizontal_axis: int
    depth_axis: int
    vertical_axis: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthBoundarySnapReport:
    boundary_edge_count: int
    mouth_boundary_edge_count: int
    mouth_boundary_component_count: int
    snapped_component_count: int
    snapped_vertex_count: int
    blended_vertex_count: int
    blend_rings: int
    contour_point_count: int
    max_projected_displacement: float
    mean_projected_displacement: float

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthBoundarySmoothReport:
    boundary_edge_count: int
    mouth_boundary_edge_count: int
    mouth_boundary_component_count: int
    smoothed_component_count: int
    smoothed_boundary_vertex_count: int
    blended_vertex_count: int
    blend_rings: int
    smoothing_cycles: int
    maximum_displacement: float
    mean_boundary_displacement: float
    max_boundary_displacement: float
    quality_rejected_face_count: int
    quality_restored_vertex_count: int
    preserve_boundary_junctions: bool
    preserved_boundary_junction_count: int
    arc_length_weighted: bool
    fragmented_selection_enabled: bool
    minimum_component_span: float | None
    minimum_component_vertices: int

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthHoleFillReport:
    boundary_edge_count_before: int
    reference_boundary_edge_count: int
    new_boundary_edge_count: int
    mouth_boundary_edge_count_before: int
    mouth_cycle_count: int
    preexisting_cycle_count: int
    preserved_aperture_cycle_count: int
    candidate_hole_cycle_count: int
    filled_hole_cycle_count: int
    rejected_hole_cycle_count: int
    added_face_count: int
    added_vertex_count: int
    reoriented_face_count: int
    boundary_edge_count_after: int
    maximum_area_ratio: float
    minimum_new_boundary_edge_fraction: float
    largest_cycle_area: float
    filled_cycle_areas: list[float]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class MouthCornerBridgeReport:
    boundary_edge_count_before: int
    boundary_edge_count_after: int
    mouth_boundary_component_count: int
    candidate_side_count: int
    repaired_sides: list[str]
    added_face_count: int
    reoriented_face_count: int
    severe_added_face_count: int
    flipped_added_face_count: int
    corner_inset_ratio: float
    chain_padding_vertices: int
    minimum_lateral_overshoot_ratio: float
    lateral_overshoot_ratios: dict[str, float]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def detect_jaw_open_aperture_faces(
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    aperture_scale: float = 1.0,
    overlap_inset_scale: float = 0.9,
    depth_padding: float = 0.01,
) -> JawOpenApertureCutReport:
    """Find front-surface faces covering the opened inner-lip aperture."""

    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    axes = (int(horizontal_axis), int(depth_axis), int(vertical_axis))
    if sorted(axes) != [0, 1, 2]:
        raise ValueError(
            "horizontal_axis, depth_axis, and vertical_axis must be distinct "
            "coordinate indices in [0, 2]."
        )
    if aperture_scale <= 0.0:
        raise ValueError("Jaw-open aperture scale must be positive.")
    if not 0.0 < overlap_inset_scale <= 1.0:
        raise ValueError("Jaw-open overlap inset scale must be in (0, 1].")
    if depth_padding < 0.0:
        raise ValueError("Jaw-open aperture depth padding cannot be negative.")
    if faces.ndim != 2 or faces.shape[-1] != 3:
        raise ValueError("detect_jaw_open_aperture_faces expects triangular faces.")

    vertex_count = int(deformed_vertices.shape[0])
    upper_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_UPPER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    lower_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_LOWER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    if len(upper_ids) < 3 or len(lower_ids) < 3:
        raise ValueError(
            "Jaw-open aperture detection requires mapped upper and lower inner lips."
        )
    ordered_ids = upper_ids + list(reversed(lower_ids))
    polygon = deformed_vertices[
        torch.tensor(ordered_ids, dtype=torch.long)
    ][:, (horizontal_axis, vertical_axis)]
    polygon = _compact_polygon_points(polygon)
    if polygon.shape[0] < 3:
        raise ValueError("Jaw-open inner-lip aperture polygon is degenerate.")
    polygon_center = polygon.mean(dim=0, keepdim=True)
    polygon = polygon_center + (polygon - polygon_center) * float(aperture_scale)

    face_centers = deformed_vertices.index_select(
        0,
        faces.reshape(-1),
    ).reshape(-1, 3, 3).mean(dim=1)
    inside_aperture = points_in_polygon_2d(
        face_centers[:, horizontal_axis],
        face_centers[:, vertical_axis],
        polygon,
    )
    lip_depths = deformed_vertices[
        torch.tensor(upper_ids + lower_ids, dtype=torch.long),
        depth_axis,
    ]
    lip_depth_center = float(lip_depths.mean().item())
    mesh_depth_min = float(deformed_vertices[:, depth_axis].amin().item())
    mesh_depth_max = float(deformed_vertices[:, depth_axis].amax().item())
    front_direction = (
        -1
        if lip_depth_center - mesh_depth_min < mesh_depth_max - lip_depth_center
        else 1
    )
    if front_direction < 0:
        front_depth = face_centers[:, depth_axis] <= (
            lip_depths.amax() + float(depth_padding)
        )
    else:
        front_depth = face_centers[:, depth_axis] >= (
            lip_depths.amin() - float(depth_padding)
        )
    overlap_aperture = torch.zeros_like(inside_aperture)
    front_face_ids = torch.nonzero(front_depth, as_tuple=False).flatten().long()
    if front_face_ids.numel() > 0:
        inset_polygon = polygon_center + (
            polygon - polygon_center
        ) * float(overlap_inset_scale)
        front_triangles = deformed_vertices.index_select(
            0,
            faces.index_select(0, front_face_ids).reshape(-1),
        ).reshape(-1, 3, 3)
        edge_midpoints = 0.5 * (
            front_triangles + front_triangles.roll(shifts=-1, dims=1)
        )
        overlap_samples = torch.cat((front_triangles, edge_midpoints), dim=1)
        sample_inside = points_in_polygon_2d(
            overlap_samples[..., horizontal_axis].reshape(-1),
            overlap_samples[..., vertical_axis].reshape(-1),
            inset_polygon,
        ).reshape(front_face_ids.numel(), -1)
        triangle_2d = front_triangles[..., (horizontal_axis, vertical_axis)]
        triangle_origin = triangle_2d[:, :1, :]
        triangle_u = triangle_2d[:, 1:2, :] - triangle_origin
        triangle_v = triangle_2d[:, 2:3, :] - triangle_origin
        polygon_relative = inset_polygon.unsqueeze(0) - triangle_origin
        dot_uu = (triangle_u * triangle_u).sum(dim=2)
        dot_uv = (triangle_u * triangle_v).sum(dim=2)
        dot_vv = (triangle_v * triangle_v).sum(dim=2)
        dot_up = (polygon_relative * triangle_u).sum(dim=2)
        dot_vp = (polygon_relative * triangle_v).sum(dim=2)
        denominator = dot_uu * dot_vv - dot_uv.square()
        valid_triangle = denominator.abs() > 1.0e-12
        safe_denominator = torch.where(
            valid_triangle,
            denominator,
            torch.ones_like(denominator),
        )
        barycentric_u = (
            dot_vv * dot_up - dot_uv * dot_vp
        ) / safe_denominator
        barycentric_v = (
            dot_uu * dot_vp - dot_uv * dot_up
        ) / safe_denominator
        polygon_inside_triangle = (
            valid_triangle
            & (barycentric_u >= -1.0e-6)
            & (barycentric_v >= -1.0e-6)
            & (barycentric_u + barycentric_v <= 1.0 + 1.0e-6)
        ).any(dim=1)
        overlap_aperture[front_face_ids] = (
            sample_inside.any(dim=1) | polygon_inside_triangle
        )
    removed_mask = front_depth & (inside_aperture | overlap_aperture)
    removed_face_ids = torch.nonzero(
        removed_mask,
        as_tuple=False,
    ).flatten().long()
    return JawOpenApertureCutReport(
        aperture_scale=float(aperture_scale),
        overlap_inset_scale=float(overlap_inset_scale),
        depth_padding=float(depth_padding),
        front_direction=int(front_direction),
        polygon_point_count=int(polygon.shape[0]),
        inside_aperture_face_count=int(inside_aperture.sum().item()),
        overlap_aperture_face_count=int(overlap_aperture.sum().item()),
        removed_face_count=int(removed_face_ids.numel()),
        original_face_count=int(faces.shape[0]),
        final_face_count=int(faces.shape[0] - removed_face_ids.numel()),
        aperture_polygon=polygon.tolist(),
        removed_face_ids=[int(face_id) for face_id in removed_face_ids.tolist()],
    )


def build_mouth_cavity_disk(
    vertices: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    scale: float = 1.12,
    shape_exponent: float = 4.0,
    depth_offset: float = 0.02,
    segments: int = 64,
    radial_rings: int = 1,
    curvature_depth: float = 0.0,
    inner_radius_ratio: float = 0.0,
    side_wall_min_horizontal_ratio: float = 0.0,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, MouthCavityReport]:
    """Build a recessed mouth-shaped lining behind the inner lips."""

    vertices = vertices.detach().cpu().float().contiguous()
    axes = (int(horizontal_axis), int(depth_axis), int(vertical_axis))
    if sorted(axes) != [0, 1, 2]:
        raise ValueError(
            "horizontal_axis, depth_axis, and vertical_axis must be distinct "
            "coordinate indices in [0, 2]."
        )
    if scale <= 0.0:
        raise ValueError("Mouth cavity scale must be positive.")
    if shape_exponent < 2.0:
        raise ValueError("Mouth cavity shape exponent must be at least 2.0.")
    if depth_offset <= 0.0:
        raise ValueError("Mouth cavity depth offset must be positive.")
    if segments < 8:
        raise ValueError("Mouth cavity requires at least eight segments.")
    if radial_rings < 1:
        raise ValueError("Mouth cavity requires at least one radial ring.")
    if curvature_depth < 0.0:
        raise ValueError("Mouth cavity curvature depth cannot be negative.")
    if not 0.0 <= inner_radius_ratio < 1.0:
        raise ValueError("Mouth cavity inner radius ratio must be in [0, 1).")
    if inner_radius_ratio > 0.0 and radial_rings < 2:
        raise ValueError("Annular mouth lining requires at least two radial rings.")
    if not 0.0 <= side_wall_min_horizontal_ratio < 1.0:
        raise ValueError("Mouth side-wall horizontal ratio must be in [0, 1).")

    lip_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
        int(vertices.shape[0]),
    )
    lip_ids = list(dict.fromkeys(lip_ids))
    if len(lip_ids) < 4:
        raise ValueError("Mouth cavity requires at least four mapped inner-lip vertices.")
    lip_points = vertices[torch.tensor(lip_ids, dtype=torch.long)]

    projected = lip_points[:, (horizontal_axis, vertical_axis)]
    projected_min = projected.amin(dim=0)
    projected_max = projected.amax(dim=0)
    projected_center = 0.5 * (projected_min + projected_max)
    projected_radius = (
        0.5 * (projected_max - projected_min) * float(scale)
    ).clamp_min(1.0e-4)

    lip_depths = lip_points[:, depth_axis]
    mesh_depth_min = vertices[:, depth_axis].amin()
    mesh_depth_max = vertices[:, depth_axis].amax()
    lip_depth_center = lip_depths.mean()
    front_direction = (
        -1
        if lip_depth_center - mesh_depth_min < mesh_depth_max - lip_depth_center
        else 1
    )
    if front_direction < 0:
        cavity_depth = lip_depths.amax() + float(depth_offset)
    else:
        cavity_depth = lip_depths.amin() - float(depth_offset)

    center = torch.zeros(3, dtype=torch.float32)
    center[horizontal_axis] = projected_center[0]
    center[vertical_axis] = projected_center[1]
    center[depth_axis] = (
        cavity_depth - float(front_direction) * float(curvature_depth)
    )
    angles = torch.arange(segments, dtype=torch.float32) * (
        2.0 * torch.pi / float(segments)
    )
    cosine = torch.cos(angles)
    sine = torch.sin(angles)
    shape_power = 2.0 / float(shape_exponent)
    unit_horizontal = cosine.sign() * cosine.abs().pow(shape_power)
    unit_vertical = sine.sign() * sine.abs().pow(shape_power)
    rings: list[torch.Tensor] = []
    radius_fractions = (
        torch.linspace(float(inner_radius_ratio), 1.0, radial_rings).tolist()
        if inner_radius_ratio > 0.0
        else [
            float(ring_index) / float(radial_rings)
            for ring_index in range(1, radial_rings + 1)
        ]
    )
    for radius_fraction in radius_fractions:
        ring = center.repeat(segments, 1)
        ring[:, horizontal_axis] += (
            projected_radius[0] * radius_fraction * unit_horizontal
        )
        ring[:, vertical_axis] += (
            projected_radius[1] * radius_fraction * unit_vertical
        )
        ring[:, depth_axis] = (
            cavity_depth
            - float(front_direction)
            * float(curvature_depth)
            * (1.0 - radius_fraction * radius_fraction)
        )
        rings.append(ring)
    if inner_radius_ratio > 0.0:
        cavity_vertices = torch.cat(rings, dim=0).contiguous()
        face_blocks: list[torch.Tensor] = []
        ring_vertex_offset = 0
    else:
        cavity_vertices = torch.cat((center.unsqueeze(0), *rings), dim=0).contiguous()
        first_ring_ids = torch.arange(1, segments + 1, dtype=torch.long)
        next_first_ring_ids = first_ring_ids.roll(shifts=-1)
        center_ids = torch.zeros(segments, dtype=torch.long)
        if front_direction < 0:
            face_blocks = [
                torch.stack((center_ids, first_ring_ids, next_first_ring_ids), dim=1)
            ]
        else:
            face_blocks = [
                torch.stack((center_ids, next_first_ring_ids, first_ring_ids), dim=1)
            ]
        ring_vertex_offset = 1
    for ring_index in range(radial_rings - 1):
        inner = (
            ring_vertex_offset
            + ring_index * segments
            + torch.arange(segments, dtype=torch.long)
        )
        outer = inner + segments
        inner_next = inner.roll(shifts=-1)
        outer_next = outer.roll(shifts=-1)
        if front_direction < 0:
            face_blocks.extend(
                (
                    torch.stack((inner, outer, outer_next), dim=1),
                    torch.stack((inner, outer_next, inner_next), dim=1),
                )
            )
        else:
            face_blocks.extend(
                (
                    torch.stack((inner, outer_next, outer), dim=1),
                    torch.stack((inner, inner_next, outer_next), dim=1),
                )
            )
    if side_wall_min_horizontal_ratio > 0.0:
        segment_mid_angles = angles + torch.pi / float(segments)
        side_segments = (
            torch.cos(segment_mid_angles).abs()
            >= float(side_wall_min_horizontal_ratio)
        )
        face_blocks = [block[side_segments] for block in face_blocks]
    cavity_faces = torch.cat(face_blocks, dim=0).contiguous()
    triangles = cavity_vertices[cavity_faces]
    face_normals = torch.cross(
        triangles[:, 1] - triangles[:, 0],
        triangles[:, 2] - triangles[:, 0],
        dim=1,
    )
    cavity_normals = torch.zeros_like(cavity_vertices)
    for corner in range(3):
        cavity_normals.index_add_(0, cavity_faces[:, corner], face_normals)
    cavity_normals = F.normalize(cavity_normals, dim=1, eps=1.0e-8)
    report = MouthCavityReport(
        vertex_count=int(cavity_vertices.shape[0]),
        face_count=int(cavity_faces.shape[0]),
        segment_count=int(segments),
        horizontal_axis=int(horizontal_axis),
        depth_axis=int(depth_axis),
        vertical_axis=int(vertical_axis),
        front_direction=int(front_direction),
        scale=float(scale),
        shape_exponent=float(shape_exponent),
        depth_offset=float(depth_offset),
        radial_ring_count=int(radial_rings),
        curvature_depth=float(curvature_depth),
        inner_radius_ratio=float(inner_radius_ratio),
        side_wall_min_horizontal_ratio=float(side_wall_min_horizontal_ratio),
        center=[float(value) for value in center.tolist()],
        radius_horizontal=float(projected_radius[0].item()),
        radius_vertical=float(projected_radius[1].item()),
    )
    return cavity_vertices, cavity_faces.contiguous(), cavity_normals, report


def build_mouth_boundary_lining(
    vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    reference_vertices: torch.Tensor | None = None,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth: float = 0.06,
    rim_depth: float = 0.002,
    detection_depth_padding: float = 0.12,
    side_fraction: float = 1.0,
    inner_scale: float = 1.0,
    radial_rings: int = 1,
    smoothing_iterations: int = 0,
    analytic_segments: int | None = None,
    opening_scale: float = 1.0,
    shape_exponent: float = 2.8,
    cap_back: bool = False,
    maximum_cycles: int = 2,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, MouthBoundaryLiningReport]:
    """Extrude real mouth boundary loops into a continuous oral compartment."""

    vertices = vertices.detach().cpu().float().contiguous()
    reference = (
        vertices
        if reference_vertices is None
        else reference_vertices.detach().cpu().float().contiguous()
    )
    faces = faces.detach().cpu().long().contiguous()
    if reference.shape != vertices.shape:
        raise ValueError("Mouth lining reference vertices must match target vertices.")
    if depth <= 0.0:
        raise ValueError("Mouth boundary lining depth must be positive.")
    if rim_depth < 0.0 or rim_depth >= depth:
        raise ValueError("Mouth lining rim depth must be in [0, depth).")
    if not 0.0 < side_fraction <= 1.0:
        raise ValueError("Mouth lining side fraction must be in (0, 1].")
    if not 0.0 < inner_scale <= 1.0:
        raise ValueError("Mouth lining inner scale must be in (0, 1].")
    if radial_rings < 1:
        raise ValueError("Mouth lining requires at least one radial ring.")
    if smoothing_iterations < 0:
        raise ValueError("Mouth lining smoothing iterations cannot be negative.")
    if analytic_segments is not None and analytic_segments < 8:
        raise ValueError("Analytic mouth compartment requires at least eight segments.")
    if opening_scale <= 0.0:
        raise ValueError("Mouth compartment opening scale must be positive.")
    if shape_exponent < 2.0:
        raise ValueError("Mouth compartment shape exponent must be at least 2.0.")
    if analytic_segments is not None and maximum_cycles != 1:
        raise ValueError("Analytic mouth compartment supports exactly one opening.")
    if maximum_cycles < 1:
        raise ValueError("Mouth lining maximum cycles must be positive.")
    if cap_back and side_fraction < 1.0:
        raise ValueError("A capped mouth compartment requires complete side walls.")

    lip_ids = list(
        dict.fromkeys(
            _ordered_mapped_vertex_ids(
                mapping,
                INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
                int(reference.shape[0]),
            )
        )
    )
    if len(lip_ids) < 4:
        raise ValueError("Mouth boundary lining requires mapped inner lips.")
    lip_points = reference[torch.tensor(lip_ids, dtype=torch.long)]
    target_lip_points = vertices[torch.tensor(lip_ids, dtype=torch.long)]
    target_projected_center = 0.5 * (
        target_lip_points[:, (horizontal_axis, vertical_axis)].amin(dim=0)
        + target_lip_points[:, (horizontal_axis, vertical_axis)].amax(dim=0)
    )
    projected = lip_points[:, (horizontal_axis, vertical_axis)]
    projected_min = projected.amin(dim=0)
    projected_max = projected.amax(dim=0)
    projected_span = (projected_max - projected_min).clamp_min(1.0e-6)

    boundary_edges = _mesh_boundary_edges(faces)
    boundary_midpoints = reference[boundary_edges].mean(dim=1)
    mouth_mask = (
        (boundary_midpoints[:, horizontal_axis] >= projected_min[0] - 0.45 * projected_span[0])
        & (boundary_midpoints[:, horizontal_axis] <= projected_max[0] + 0.45 * projected_span[0])
        & (boundary_midpoints[:, vertical_axis] >= projected_min[1] - 0.45 * projected_span[1])
        & (boundary_midpoints[:, vertical_axis] <= projected_max[1] + 0.45 * projected_span[1])
        & (
            boundary_midpoints[:, depth_axis]
            >= lip_points[:, depth_axis].amin() - float(detection_depth_padding)
        )
        & (
            boundary_midpoints[:, depth_axis]
            <= lip_points[:, depth_axis].amax() + float(detection_depth_padding)
        )
    )
    adjacency: dict[int, list[int]] = {}
    for first, second in boundary_edges[mouth_mask].tolist():
        adjacency.setdefault(int(first), []).append(int(second))
        adjacency.setdefault(int(second), []).append(int(first))
    cycles = [cycle for cycle in _undirected_cycle_basis(adjacency) if len(cycle) >= 3]

    candidates: list[tuple[float, list[int]]] = []
    for cycle in cycles:
        points = reference[torch.tensor(cycle, dtype=torch.long)]
        horizontal_span = points[:, horizontal_axis].amax() - points[:, horizontal_axis].amin()
        vertical_span = points[:, vertical_axis].amax() - points[:, vertical_axis].amin()
        if (
            horizontal_span < 0.8 * projected_span[0]
            or vertical_span < 0.8 * projected_span[1]
        ):
            continue
        points_2d = points[:, (horizontal_axis, vertical_axis)]
        area = abs(
            0.5
            * float(
                (
                    points_2d[:, 0] * torch.roll(points_2d[:, 1], shifts=-1)
                    - points_2d[:, 1] * torch.roll(points_2d[:, 0], shifts=-1)
                ).sum().item()
            )
        )
        candidates.append((area, cycle))
    selected_cycles = [
        item[1] for item in sorted(candidates, reverse=True)[:maximum_cycles]
    ]
    boundary_source = "closed_cycle"
    if not selected_cycles and analytic_segments is not None:
        remaining = set(adjacency)
        components: list[list[int]] = []
        while remaining:
            root = min(remaining)
            stack = [root]
            component: list[int] = []
            remaining.remove(root)
            while stack:
                current = stack.pop()
                component.append(current)
                for neighbor in adjacency[current]:
                    if neighbor in remaining:
                        remaining.remove(neighbor)
                        stack.append(neighbor)
            if len(component) >= 3:
                components.append(component)

        if components:
            def projected_component_area(component: list[int]) -> float:
                points = reference[torch.tensor(component, dtype=torch.long)]
                projected_points = points[:, (horizontal_axis, vertical_axis)]
                span = projected_points.amax(dim=0) - projected_points.amin(dim=0)
                return float(span.prod().item())

            selected_cycles = [max(components, key=projected_component_area)]
            boundary_source = "open_component"
        else:
            selected_cycles = [lip_ids]
            boundary_source = "lip_landmarks"
    if not selected_cycles:
        raise ValueError("No dominant mouth boundary loops were found for lining.")

    lip_depth_center = lip_points[:, depth_axis].mean()
    front_direction = (
        -1
        if lip_depth_center - reference[:, depth_axis].amin()
        < reference[:, depth_axis].amax() - lip_depth_center
        else 1
    )
    vertex_parts: list[torch.Tensor] = []
    face_parts: list[torch.Tensor] = []
    local_offset = 0
    boundary_vertex_count = 0
    for cycle in selected_cycles:
        ids = torch.tensor(cycle, dtype=torch.long)
        cycle_points = vertices[ids]
        if analytic_segments is None:
            base = cycle_points.clone()
            ring_center = target_projected_center
        else:
            bounds_points = torch.cat((cycle_points, target_lip_points), dim=0)
            cycle_projected = bounds_points[:, (horizontal_axis, vertical_axis)]
            cycle_min = cycle_projected.amin(dim=0)
            cycle_max = cycle_projected.amax(dim=0)
            ring_center = 0.5 * (cycle_min + cycle_max)
            ring_radius = (
                0.5 * (cycle_max - cycle_min) * float(opening_scale)
            ).clamp_min(1.0e-5)
            angles = torch.arange(analytic_segments, dtype=torch.float32) * (
                2.0 * torch.pi / float(analytic_segments)
            )
            shape_power = 2.0 / float(shape_exponent)
            unit_horizontal = (
                torch.cos(angles).sign()
                * torch.cos(angles).abs().pow(shape_power)
            )
            unit_vertical = (
                torch.sin(angles).sign()
                * torch.sin(angles).abs().pow(shape_power)
            )
            base = vertices.new_zeros((analytic_segments, 3))
            base[:, horizontal_axis] = (
                ring_center[0] + ring_radius[0] * unit_horizontal
            )
            base[:, vertical_axis] = (
                ring_center[1] + ring_radius[1] * unit_vertical
            )
            cycle_depths = bounds_points[:, depth_axis]
            base[:, depth_axis] = (
                cycle_depths.amax()
                if front_direction < 0
                else cycle_depths.amin()
            )
        count = int(base.shape[0])
        base_projected = base[:, (horizontal_axis, vertical_axis)]
        smooth_projected = base_projected.clone()
        for _ in range(smoothing_iterations):
            smooth_projected = (
                0.5 * smooth_projected
                + 0.25 * torch.roll(smooth_projected, shifts=1, dims=0)
                + 0.25 * torch.roll(smooth_projected, shifts=-1, dims=0)
            )
        if smoothing_iterations > 0:
            base_min = base_projected.amin(dim=0)
            base_max = base_projected.amax(dim=0)
            smooth_min = smooth_projected.amin(dim=0)
            smooth_max = smooth_projected.amax(dim=0)
            base_center = 0.5 * (base_min + base_max)
            smooth_center = 0.5 * (smooth_min + smooth_max)
            span_scale = (base_max - base_min) / (
                smooth_max - smooth_min
            ).clamp_min(1.0e-6)
            smooth_projected = (
                base_center + (smooth_projected - smooth_center) * span_scale
            )

        ring_parts: list[torch.Tensor] = []
        front_depths = (
            base[:, depth_axis] - float(front_direction) * float(rim_depth)
        )
        back_depth = (
            base[:, depth_axis].mean()
            - float(front_direction) * float(depth)
        )
        for ring_index in range(radial_rings + 1):
            fraction = ring_index / float(radial_rings)
            ring = base.clone()
            ring[:, depth_axis] = (
                front_depths * (1.0 - fraction) + back_depth * fraction
            )
            ring_scale = 1.0 - (1.0 - float(inner_scale)) * fraction
            ring_projected = (
                base_projected * (1.0 - fraction)
                + smooth_projected * fraction
            )
            ring_projected = ring_center + (
                ring_projected - ring_center
            ) * ring_scale
            ring[:, horizontal_axis] = ring_projected[:, 0]
            ring[:, vertical_axis] = ring_projected[:, 1]
            ring_parts.append(ring)

        reference_points = (
            reference[ids]
            if analytic_segments is None
            else base
        )
        edge_horizontal = 0.5 * (
            reference_points[:, horizontal_axis]
            + torch.roll(reference_points[:, horizontal_axis], shifts=-1)
        )
        side_width = float(side_fraction) * projected_span[0]
        side_edges = (
            (edge_horizontal <= projected_min[0] + side_width)
            | (edge_horizontal >= projected_max[0] - side_width)
        )

        for ring_index in range(radial_rings):
            outer_ids = (
                local_offset
                + ring_index * count
                + torch.arange(count, dtype=torch.long)
            )
            inner_ids = outer_ids + count
            outer_next = outer_ids.roll(shifts=-1)
            inner_next = inner_ids.roll(shifts=-1)
            first_block = torch.stack((outer_ids, inner_ids, inner_next), dim=1)
            second_block = torch.stack((outer_ids, inner_next, outer_next), dim=1)
            if side_fraction < 1.0:
                first_block = first_block[side_edges]
                second_block = second_block[side_edges]
            face_parts.extend((first_block, second_block))

        vertex_parts.extend(ring_parts)
        cycle_vertex_count = (radial_rings + 1) * count
        if cap_back:
            back_ring_ids = (
                local_offset
                + radial_rings * count
                + torch.arange(count, dtype=torch.long)
            )
            back_next = back_ring_ids.roll(shifts=-1)
            cap_center = ring_parts[-1].mean(dim=0)
            cap_center[horizontal_axis] = ring_center[0]
            cap_center[vertical_axis] = ring_center[1]
            center_id = torch.full(
                (count,),
                local_offset + cycle_vertex_count,
                dtype=torch.long,
            )
            face_parts.append(
                torch.stack((back_ring_ids, center_id, back_next), dim=1)
            )
            vertex_parts.append(cap_center.unsqueeze(0))
            cycle_vertex_count += 1

        local_offset += cycle_vertex_count
        boundary_vertex_count += count

    lining_vertices = torch.cat(vertex_parts, dim=0).contiguous()
    lining_faces = torch.cat(face_parts, dim=0).contiguous()
    triangles = lining_vertices[lining_faces]
    face_normals = torch.cross(
        triangles[:, 1] - triangles[:, 0],
        triangles[:, 2] - triangles[:, 0],
        dim=1,
    )
    lining_normals = torch.zeros_like(lining_vertices)
    for corner in range(3):
        lining_normals.index_add_(0, lining_faces[:, corner], face_normals)
    lining_normals = F.normalize(lining_normals, dim=1, eps=1.0e-8)
    report = MouthBoundaryLiningReport(
        vertex_count=int(lining_vertices.shape[0]),
        face_count=int(lining_faces.shape[0]),
        boundary_cycle_count=(
            len(selected_cycles) if boundary_source == "closed_cycle" else 0
        ),
        boundary_vertex_count=boundary_vertex_count,
        depth=float(depth),
        front_direction=int(front_direction),
        side_fraction=float(side_fraction),
        inner_scale=float(inner_scale),
        radial_ring_count=int(radial_rings),
        smoothing_iterations=int(smoothing_iterations),
        analytic_segments=(
            int(analytic_segments) if analytic_segments is not None else None
        ),
        opening_scale=float(opening_scale),
        shape_exponent=float(shape_exponent),
        boundary_source=boundary_source,
        cap_back=bool(cap_back),
        maximum_cycles=int(maximum_cycles),
    )
    return lining_vertices, lining_faces, lining_normals, report


def build_mediapipe_lip_annulus(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    segments: int = 64,
    radial_rings: int = 3,
    outer_scale: float = 1.03,
    thickness_ratio: float = 0.055,
    side_thickness_factor: float = 0.45,
    maximum_radial_inset_fraction: float = 0.45,
    inner_recess_depth: float = 0.003,
    front_offset: float = 0.002,
    bulge_depth: float = 0.0025,
    upper_smoothing_window: int = 9,
    lower_smoothing_window: int = 17,
) -> tuple[
    torch.Tensor,
    torch.Tensor,
    torch.Tensor,
    LipAnnulusRepairReport,
]:
    """Build a continuous lip strip from a uniformly inset outer contour.

    MediaPipe's inner and outer lip curves do not have dependable point-wise
    correspondence after projection onto fragmented meshes.  Pairing those
    curves can therefore bridge unrelated points across the mouth.  This
    repair samples only the repaired outer contour and derives a narrow inner
    edge geometrically, leaving the predicted mouth opening unobstructed.
    """

    from scipy.interpolate import PchipInterpolator

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    if neutral_vertices.shape != deformed_vertices.shape:
        raise ValueError("Lip annulus poses must have matching vertex arrays.")
    if segments < 16 or segments % 2 != 0:
        raise ValueError("Lip annulus segments must be an even integer of at least 16.")
    if radial_rings < 1:
        raise ValueError("Lip annulus requires at least one radial ring.")
    if outer_scale <= 0.0:
        raise ValueError("Lip annulus outer scale must be positive.")
    if thickness_ratio <= 0.0:
        raise ValueError("Lip annulus thickness ratio must be positive.")
    if not 0.0 < side_thickness_factor <= 1.0:
        raise ValueError("Lip annulus side thickness factor must be in (0, 1].")
    if not 0.0 < maximum_radial_inset_fraction < 1.0:
        raise ValueError(
            "Lip annulus maximum radial inset fraction must be between 0 and 1."
        )
    if inner_recess_depth < 0.0:
        raise ValueError("Lip annulus inner recess depth cannot be negative.")
    if front_offset < 0.0:
        raise ValueError("Lip annulus front offset cannot be negative.")
    if bulge_depth < 0.0:
        raise ValueError("Lip annulus bulge depth cannot be negative.")
    side_samples = segments // 2 + 1
    for name, window in (
        ("upper", upper_smoothing_window),
        ("lower", lower_smoothing_window),
    ):
        if window < 3 or window % 2 == 0 or window > side_samples:
            raise ValueError(
                f"Lip annulus {name} smoothing window must be an odd integer "
                f"between 3 and {side_samples}."
            )
    axes = {int(horizontal_axis), int(depth_axis), int(vertical_axis)}
    if axes != {0, 1, 2}:
        raise ValueError("Lip annulus axes must be a permutation of [0, 1, 2].")

    vertex_count = int(neutral_vertices.shape[0])
    curve_landmark_ids = (
        OUTER_UPPER_LIP_MEDIAPIPE_IDS,
        OUTER_LOWER_LIP_MEDIAPIPE_IDS,
    )
    curve_vertex_ids = [
        _ordered_mapped_vertex_ids(mapping, curve, vertex_count)
        for curve in curve_landmark_ids
    ]
    if any(len(ids) < 4 for ids in curve_vertex_ids):
        raise ValueError(
            "Lip annulus repair requires mapped outer upper/lower curves."
        )

    def sample_curve(
        vertices: torch.Tensor,
        ids: list[int],
        smoothing_window: int,
    ) -> torch.Tensor:
        from scipy.signal import savgol_filter

        points = vertices[torch.tensor(ids, dtype=torch.long)].double().numpy()
        projected = points[:, (horizontal_axis, vertical_axis)]
        distances = np.linalg.norm(np.diff(projected, axis=0), axis=1)
        parameter = np.concatenate(([0.0], np.cumsum(distances)))
        keep = np.concatenate(([True], np.diff(parameter) > 1.0e-10))
        parameter = parameter[keep]
        points = points[keep]
        if parameter.shape[0] < 2 or parameter[-1] <= 1.0e-10:
            raise ValueError("A lip annulus curve collapsed to one point.")
        parameter /= parameter[-1]
        samples = PchipInterpolator(
            parameter,
            points,
            axis=0,
            extrapolate=False,
        )(np.linspace(0.0, 1.0, side_samples))
        endpoints = samples[[0, -1]].copy()
        for axis in (depth_axis, vertical_axis):
            samples[:, axis] = savgol_filter(
                samples[:, axis],
                window_length=int(smoothing_window),
                polyorder=3,
                mode="interp",
            )
        samples[[0, -1]] = endpoints
        return torch.from_numpy(samples.astype(np.float32, copy=False))

    def closed_ring(upper: torch.Tensor, lower: torch.Tensor) -> torch.Tensor:
        return torch.cat((upper, torch.flip(lower[1:-1], dims=(0,))), dim=0)

    neutral_outer_upper = sample_curve(
        neutral_vertices, curve_vertex_ids[0], upper_smoothing_window
    )
    neutral_outer_lower = sample_curve(
        neutral_vertices, curve_vertex_ids[1], lower_smoothing_window
    )
    deformed_outer_upper = sample_curve(
        deformed_vertices, curve_vertex_ids[0], upper_smoothing_window
    )
    deformed_outer_lower = sample_curve(
        deformed_vertices, curve_vertex_ids[1], lower_smoothing_window
    )

    neutral_outer = closed_ring(neutral_outer_upper, neutral_outer_lower)
    deformed_outer = closed_ring(deformed_outer_upper, deformed_outer_lower)
    if any(int(ring.shape[0]) != segments for ring in (neutral_outer, deformed_outer)):
        raise RuntimeError("Lip annulus curve resampling produced inconsistent rings.")

    def build_ring_pair(
        outer: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, int]:
        projected = outer[:, (horizontal_axis, vertical_axis)]
        center = 0.5 * (projected.amin(dim=0) + projected.amax(dim=0))
        scaled_outer = outer.clone()
        scaled_outer[:, horizontal_axis] = center[0] + float(outer_scale) * (
            outer[:, horizontal_axis] - center[0]
        )
        scaled_outer[:, vertical_axis] = center[1] + float(outer_scale) * (
            outer[:, vertical_axis] - center[1]
        )
        scaled_projected = scaled_outer[:, (horizontal_axis, vertical_axis)]
        radial_delta = scaled_projected - center
        radial_distance = radial_delta.norm(dim=1).clamp_min(1.0e-8)
        projected_span = scaled_projected.amax(dim=0) - scaled_projected.amin(dim=0)
        base_inset = max(
            float(thickness_ratio) * float(projected_span.max().item()),
            1.0e-6,
        )
        vertical_fraction = (
            radial_delta[:, 1].abs() / radial_distance
        ).clamp(0.0, 1.0)
        thickness_weight = float(side_thickness_factor) + (
            1.0 - float(side_thickness_factor)
        ) * vertical_fraction.pow(1.5)
        requested_inset = base_inset * thickness_weight
        inset = torch.minimum(
            requested_inset,
            radial_distance * float(maximum_radial_inset_fraction),
        )
        inner_projected = scaled_projected - (
            radial_delta / radial_distance.unsqueeze(1)
        ) * inset.unsqueeze(1)
        inner = scaled_outer.clone()
        inner[:, horizontal_axis] = inner_projected[:, 0]
        inner[:, vertical_axis] = inner_projected[:, 1]
        capped_count = int((inset < requested_inset - 1.0e-8).sum().item())
        return scaled_outer, inner, capped_count

    (
        neutral_outer,
        neutral_inner,
        capped_neutral_segment_count,
    ) = build_ring_pair(neutral_outer)
    (
        deformed_outer,
        deformed_inner,
        capped_deformed_segment_count,
    ) = build_ring_pair(deformed_outer)
    lip_depth_center = float(neutral_outer[:, depth_axis].mean().item())
    front_direction = (
        -1
        if lip_depth_center - float(neutral_vertices[:, depth_axis].amin().item())
        < float(neutral_vertices[:, depth_axis].amax().item()) - lip_depth_center
        else 1
    )
    neutral_inner[:, depth_axis] -= float(front_direction) * float(inner_recess_depth)
    deformed_inner[:, depth_axis] -= float(front_direction) * float(inner_recess_depth)

    def surface_rows(outer: torch.Tensor, inner: torch.Tensor) -> torch.Tensor:
        rows = []
        for ring_index in range(radial_rings + 1):
            fraction = ring_index / float(radial_rings)
            smooth_fraction = fraction * fraction * (3.0 - 2.0 * fraction)
            row = outer * (1.0 - smooth_fraction) + inner * smooth_fraction
            # Keep the repair just in front of the damaged source lip. The
            # offset grows toward the cut edge and avoids coplanar flicker.
            offset_weight = 0.25 + 0.75 * smooth_fraction
            row[:, depth_axis] += (
                float(front_direction)
                * (
                    float(front_offset) * offset_weight
                    + float(bulge_depth) * math.sin(math.pi * fraction)
                )
            )
            rows.append(row)
        return torch.cat(rows, dim=0).contiguous()

    neutral_annulus = surface_rows(neutral_outer, neutral_inner)
    deformed_annulus = surface_rows(deformed_outer, deformed_inner)
    face_parts: list[torch.Tensor] = []
    ring_ids = torch.arange(segments, dtype=torch.long)
    next_ids = torch.roll(ring_ids, shifts=-1)
    for ring_index in range(radial_rings):
        outer_ids = ring_index * segments + ring_ids
        outer_next = ring_index * segments + next_ids
        inner_ids = outer_ids + segments
        inner_next = outer_next + segments
        face_parts.extend(
            (
                torch.stack((outer_ids, outer_next, inner_next), dim=1),
                torch.stack((outer_ids, inner_next, inner_ids), dim=1),
            )
        )
    annulus_faces = torch.cat(face_parts, dim=0).contiguous()
    triangles = neutral_annulus[annulus_faces]
    normals = torch.cross(
        triangles[:, 1] - triangles[:, 0],
        triangles[:, 2] - triangles[:, 0],
        dim=1,
    )
    if float(normals[:, depth_axis].mean().item()) * front_direction < 0.0:
        annulus_faces = annulus_faces[:, (0, 2, 1)].contiguous()

    report = LipAnnulusRepairReport(
        vertex_count=int(neutral_annulus.shape[0]),
        face_count=int(annulus_faces.shape[0]),
        segment_count=int(segments),
        radial_ring_count=int(radial_rings),
        outer_scale=float(outer_scale),
        thickness_ratio=float(thickness_ratio),
        side_thickness_factor=float(side_thickness_factor),
        maximum_radial_inset_fraction=float(maximum_radial_inset_fraction),
        inner_recess_depth=float(inner_recess_depth),
        front_offset=float(front_offset),
        bulge_depth=float(bulge_depth),
        capped_neutral_segment_count=int(capped_neutral_segment_count),
        capped_deformed_segment_count=int(capped_deformed_segment_count),
        upper_smoothing_window=int(upper_smoothing_window),
        lower_smoothing_window=int(lower_smoothing_window),
        front_direction=int(front_direction),
        horizontal_axis=int(horizontal_axis),
        depth_axis=int(depth_axis),
        vertical_axis=int(vertical_axis),
    )
    return neutral_annulus, deformed_annulus, annulus_faces, report


def clear_source_faces_under_lip_annulus(
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    deformed_annulus: torch.Tensor,
    *,
    segments: int,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth_padding_ratio: float = 0.30,
) -> tuple[torch.Tensor, LipAnnulusHeadCleanupReport]:
    """Clear source triangles replaced by the generated lip annulus."""

    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    deformed_annulus = deformed_annulus.detach().cpu().float().contiguous()
    if segments < 3 or int(deformed_annulus.shape[0]) < segments:
        raise ValueError("Lip annulus cleanup requires one complete outer ring.")
    if depth_padding_ratio < 0.0:
        raise ValueError("Lip annulus cleanup depth padding cannot be negative.")
    axes = {int(horizontal_axis), int(depth_axis), int(vertical_axis)}
    if axes != {0, 1, 2}:
        raise ValueError("Lip annulus cleanup axes must be a permutation of [0, 1, 2].")

    outer_ring = deformed_annulus[:segments]
    polygon_stride = max(1, int(segments) // 32)
    polygon = outer_ring[::polygon_stride, (horizontal_axis, vertical_axis)]
    triangles = deformed_vertices[faces]
    samples = torch.cat(
        (
            triangles.mean(dim=1, keepdim=True),
            0.5 * (triangles[:, 0:1] + triangles[:, 1:2]),
            0.5 * (triangles[:, 1:2] + triangles[:, 2:3]),
            0.5 * (triangles[:, 2:3] + triangles[:, 0:1]),
        ),
        dim=1,
    )
    projected_samples = samples[:, :, (horizontal_axis, vertical_axis)]
    polygon_min = polygon.amin(dim=0)
    polygon_max = polygon.amax(dim=0)
    triangle_projected = triangles[:, :, (horizontal_axis, vertical_axis)]
    bbox_overlap = (
        (triangle_projected.amax(dim=1) >= polygon_min).all(dim=1)
        & (triangle_projected.amin(dim=1) <= polygon_max).all(dim=1)
    )
    overlap = torch.zeros(int(faces.shape[0]), dtype=torch.bool)
    candidate_ids = torch.nonzero(bbox_overlap, as_tuple=False).flatten()
    if candidate_ids.numel() > 0:
        candidate_samples = projected_samples[candidate_ids]
        candidate_overlap = points_in_polygon_2d(
            candidate_samples[:, :, 0].reshape(-1),
            candidate_samples[:, :, 1].reshape(-1),
            polygon,
        ).reshape(int(candidate_ids.shape[0]), -1).any(dim=1)
        overlap[candidate_ids] = candidate_overlap
    projected_span = float(
        (polygon.amax(dim=0) - polygon.amin(dim=0)).amax().item()
    )
    depth_padding = float(depth_padding_ratio) * projected_span
    depth_min = float(outer_ring[:, depth_axis].amin().item()) - depth_padding
    depth_max = float(outer_ring[:, depth_axis].amax().item()) + depth_padding
    face_depth = triangles[:, :, depth_axis].mean(dim=1)
    within_depth = (face_depth >= depth_min) & (face_depth <= depth_max)
    removed = overlap & within_depth
    kept_faces = faces[~removed].contiguous()
    report = LipAnnulusHeadCleanupReport(
        original_face_count=int(faces.shape[0]),
        removed_face_count=int(removed.sum().item()),
        final_face_count=int(kept_faces.shape[0]),
        projected_overlap_face_count=int(overlap.sum().item()),
        depth_rejected_overlap_face_count=int((overlap & ~within_depth).sum().item()),
        polygon_point_count=int(polygon.shape[0]),
        depth_padding_ratio=float(depth_padding_ratio),
        depth_min=float(depth_min),
        depth_max=float(depth_max),
        horizontal_axis=int(horizontal_axis),
        depth_axis=int(depth_axis),
        vertical_axis=int(vertical_axis),
    )
    return kept_faces, report


def orient_front_surface_faces(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    depth_axis: int = 1,
    region_depth_ratio: float = 0.18,
    normal_tolerance: float = 0.01,
) -> tuple[torch.Tensor, FrontSurfaceOrientationReport]:
    """Orient the external facial shell toward the viewer.

    Reconstructed scan meshes can contain many disconnected triangles whose
    winding is inconsistent even when their positions form a usable surface.
    Restricting the repair to a depth band around the lips avoids changing the
    rear head or the separately appended oral assembly.
    """

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    if neutral_vertices.shape != deformed_vertices.shape:
        raise ValueError("Front-surface orientation poses must have matching vertices.")
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Front-surface orientation faces must have shape [F, 3].")
    if not 0 <= int(depth_axis) <= 2:
        raise ValueError("Front-surface orientation depth axis must be 0, 1, or 2.")
    if region_depth_ratio <= 0.0:
        raise ValueError("Front-surface orientation depth ratio must be positive.")
    if normal_tolerance < 0.0:
        raise ValueError("Front-surface orientation normal tolerance cannot be negative.")
    vertex_count = int(neutral_vertices.shape[0])
    mouth_ids = _ordered_mapped_vertex_ids(
        mapping,
        OUTER_UPPER_LIP_MEDIAPIPE_IDS + OUTER_LOWER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    if len(mouth_ids) < 4:
        raise ValueError("Front-surface orientation requires mapped outer lip points.")

    mouth_index = torch.tensor(mouth_ids, dtype=torch.long)
    neutral_mouth_depth = float(
        neutral_vertices[mouth_index, depth_axis].mean().item()
    )
    neutral_depth_min = float(neutral_vertices[:, depth_axis].amin().item())
    neutral_depth_max = float(neutral_vertices[:, depth_axis].amax().item())
    front_direction = (
        -1
        if neutral_mouth_depth - neutral_depth_min
        < neutral_depth_max - neutral_mouth_depth
        else 1
    )

    deformed_mouth_depth = float(
        deformed_vertices[mouth_index, depth_axis].mean().item()
    )
    deformed_depth_min = float(deformed_vertices[:, depth_axis].amin().item())
    deformed_depth_max = float(deformed_vertices[:, depth_axis].amax().item())
    depth_span = max(deformed_depth_max - deformed_depth_min, 1.0e-6)
    front_depth_limit = deformed_mouth_depth - (
        float(front_direction) * float(region_depth_ratio) * depth_span
    )

    triangles = deformed_vertices[faces]
    face_centers = triangles.mean(dim=1)
    face_normals = torch.cross(
        triangles[:, 1] - triangles[:, 0],
        triangles[:, 2] - triangles[:, 0],
        dim=1,
    )
    normal_magnitude = face_normals.norm(dim=1)
    eligible = (
        float(front_direction)
        * (face_centers[:, depth_axis] - float(front_depth_limit))
        >= 0.0
    )
    reversed_winding = eligible & (
        float(front_direction) * face_normals[:, depth_axis]
        < -float(normal_tolerance) * normal_magnitude
    )
    oriented_faces = faces.clone()
    oriented_faces[reversed_winding] = oriented_faces[reversed_winding][:, (0, 2, 1)]
    report = FrontSurfaceOrientationReport(
        face_count=int(faces.shape[0]),
        eligible_face_count=int(eligible.sum().item()),
        flipped_face_count=int(reversed_winding.sum().item()),
        front_direction=int(front_direction),
        mouth_depth=float(deformed_mouth_depth),
        front_depth_limit=float(front_depth_limit),
        region_depth_ratio=float(region_depth_ratio),
        normal_tolerance=float(normal_tolerance),
        depth_axis=int(depth_axis),
    )
    return oriented_faces.contiguous(), report


def detect_stretched_mouth_faces(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    edge_ratio_threshold: float = 3.0,
    edge_growth_threshold: float = 0.02,
    neighbor_growth_rings: int = 2,
    neighbor_edge_ratio_threshold: float = 1.35,
    neighbor_edge_growth_threshold: float = 0.006,
    region_scale: float = 1.5,
    depth_padding: float = 0.08,
) -> StretchedMouthFaceReport:
    """Find strict edge-stretch outliers near the opened inner lips."""

    neutral_vertices = neutral_vertices.detach().cpu().float()
    deformed_vertices = deformed_vertices.detach().cpu().float()
    faces = faces.detach().cpu().long()
    lip_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
        int(deformed_vertices.shape[0]),
    )
    lip_ids = list(dict.fromkeys(lip_ids))
    if len(lip_ids) < 4:
        raise ValueError("Stretch detection requires mapped inner-lip vertices.")
    lip_points = deformed_vertices[torch.tensor(lip_ids, dtype=torch.long)]
    projected = lip_points[:, (horizontal_axis, vertical_axis)]
    center = 0.5 * (projected.amin(dim=0) + projected.amax(dim=0))
    radius = 0.5 * (projected.amax(dim=0) - projected.amin(dim=0)) * float(region_scale)

    neutral_triangles = neutral_vertices[faces]
    deformed_triangles = deformed_vertices[faces]
    neutral_edges = torch.stack(
        tuple(
            (neutral_triangles[:, (corner + 1) % 3] - neutral_triangles[:, corner]).norm(
                dim=1
            )
            for corner in range(3)
        ),
        dim=1,
    )
    deformed_edges = torch.stack(
        tuple(
            (deformed_triangles[:, (corner + 1) % 3] - deformed_triangles[:, corner]).norm(
                dim=1
            )
            for corner in range(3)
        ),
        dim=1,
    )
    max_ratio = (deformed_edges / neutral_edges.clamp_min(1.0e-8)).amax(dim=1)
    max_growth = (deformed_edges - neutral_edges).amax(dim=1)
    face_centers = deformed_triangles.mean(dim=1)
    neutral_lip_depth = neutral_vertices[
        torch.tensor(lip_ids, dtype=torch.long), depth_axis
    ].mean()
    neutral_depth_min = neutral_vertices[:, depth_axis].amin()
    neutral_depth_max = neutral_vertices[:, depth_axis].amax()
    front_direction = (
        -1
        if float((neutral_lip_depth - neutral_depth_min).item())
        < float((neutral_depth_max - neutral_lip_depth).item())
        else 1
    )
    if front_direction < 0:
        front_depth = face_centers[:, depth_axis] <= (
            lip_points[:, depth_axis].amax() + float(depth_padding)
        )
    else:
        front_depth = face_centers[:, depth_axis] >= (
            lip_points[:, depth_axis].amin() - float(depth_padding)
        )
    candidate = (
        ((face_centers[:, horizontal_axis] - center[0]).abs() <= radius[0])
        & ((face_centers[:, vertical_axis] - center[1]).abs() <= radius[1])
        & front_depth
    )
    strict_removed = candidate & (max_ratio > float(edge_ratio_threshold)) & (
        max_growth > float(edge_growth_threshold)
    )
    if neighbor_growth_rings < 0:
        raise ValueError("Stretch-neighbor growth rings cannot be negative.")
    if neighbor_edge_ratio_threshold <= 1.0:
        raise ValueError("Stretch-neighbor edge ratio must be greater than one.")
    if neighbor_edge_growth_threshold < 0.0:
        raise ValueError("Stretch-neighbor edge growth cannot be negative.")
    soft_candidate = (
        candidate
        & (max_ratio > float(neighbor_edge_ratio_threshold))
        & (max_growth > float(neighbor_edge_growth_threshold))
    )
    landmark_vertex_mask = torch.zeros(
        deformed_vertices.shape[0],
        dtype=torch.bool,
    )
    mouth_landmark_ids = set(
        INNER_UPPER_LIP_MEDIAPIPE_IDS
        + INNER_LOWER_LIP_MEDIAPIPE_IDS
        + OUTER_UPPER_LIP_MEDIAPIPE_IDS
        + OUTER_LOWER_LIP_MEDIAPIPE_IDS
    )
    mapped_mouth_vertex_ids = [
        int(vertex_id)
        for landmark_id, vertex_id in mapping.items()
        if int(landmark_id) in mouth_landmark_ids
        and 0 <= int(vertex_id) < deformed_vertices.shape[0]
    ]
    if mapped_mouth_vertex_ids:
        landmark_vertex_mask[
            torch.tensor(mapped_mouth_vertex_ids, dtype=torch.long)
        ] = True
    landmark_faces = landmark_vertex_mask[faces].any(dim=1)
    removed = strict_removed.clone()
    frontier = removed.clone()
    for _ring in range(int(neighbor_growth_rings)):
        if not bool(frontier.any().item()):
            break
        frontier_vertices = torch.zeros(
            deformed_vertices.shape[0],
            dtype=torch.bool,
        )
        frontier_vertices[faces[frontier].reshape(-1)] = True
        touching_frontier = frontier_vertices[faces].any(dim=1)
        grown = (
            touching_frontier
            & soft_candidate
            & ~removed
            & ~landmark_faces
        )
        if not bool(grown.any().item()):
            break
        removed |= grown
        frontier = grown
    removed_ids = torch.nonzero(removed, as_tuple=False).flatten().long()
    return StretchedMouthFaceReport(
        edge_ratio_threshold=float(edge_ratio_threshold),
        edge_growth_threshold=float(edge_growth_threshold),
        neighbor_growth_rings=int(neighbor_growth_rings),
        neighbor_edge_ratio_threshold=float(neighbor_edge_ratio_threshold),
        neighbor_edge_growth_threshold=float(neighbor_edge_growth_threshold),
        region_scale=float(region_scale),
        depth_padding=float(depth_padding),
        candidate_face_count=int(candidate.sum().item()),
        strict_removed_face_count=int(strict_removed.sum().item()),
        grown_removed_face_count=int((removed & ~strict_removed).sum().item()),
        removed_face_count=int(removed_ids.numel()),
        removed_face_ids=[int(value) for value in removed_ids.tolist()],
    )


def retain_aperture_connected_face_removals(
    faces: torch.Tensor,
    aperture_face_ids: Sequence[int],
    stretch_face_ids: Sequence[int],
) -> MouthRemovalConnectivityReport:
    """Keep only the dominant aperture-connected removal component."""

    cpu_faces = faces.detach().cpu().long().contiguous()
    if cpu_faces.ndim != 2 or cpu_faces.shape[1] != 3:
        raise ValueError(
            "Mouth-removal connectivity filtering expects triangular faces."
        )
    face_count = int(cpu_faces.shape[0])
    aperture_ids = sorted(set(int(value) for value in aperture_face_ids))
    stretch_ids = sorted(set(int(value) for value in stretch_face_ids))
    invalid_ids = [
        value
        for value in aperture_ids + stretch_ids
        if value < 0 or value >= face_count
    ]
    if invalid_ids:
        raise ValueError(
            "Mouth-removal face ids are outside the source face range: "
            + ", ".join(str(value) for value in invalid_ids[:8])
        )

    candidate_ids = sorted(set(aperture_ids) | set(stretch_ids))
    if not candidate_ids:
        return MouthRemovalConnectivityReport(
            aperture_seed_face_count=0,
            stretch_candidate_face_count=0,
            candidate_face_count=0,
            connected_component_count=0,
            seeded_component_count=0,
            significant_seeded_component_count=0,
            discarded_aperture_component_count=0,
            discarded_aperture_face_count=0,
            removed_face_count=0,
            restored_disconnected_component_count=0,
            restored_disconnected_face_count=0,
            removed_face_ids=[],
        )

    candidate_faces = cpu_faces[
        torch.tensor(candidate_ids, dtype=torch.long)
    ].numpy()
    parent = np.arange(len(candidate_ids), dtype=np.int64)
    component_size = np.ones(len(candidate_ids), dtype=np.int64)

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = int(parent[index])
        return index

    def union(left: int, right: int) -> None:
        left_root = find(left)
        right_root = find(right)
        if left_root == right_root:
            return
        if component_size[left_root] < component_size[right_root]:
            left_root, right_root = right_root, left_root
        parent[right_root] = left_root
        component_size[left_root] += component_size[right_root]

    edge_owner: dict[tuple[int, int], int] = {}
    for local_face_id, face in enumerate(candidate_faces):
        for corner in range(3):
            start = int(face[corner])
            end = int(face[(corner + 1) % 3])
            edge = (start, end) if start < end else (end, start)
            previous_owner = edge_owner.setdefault(edge, local_face_id)
            if previous_owner != local_face_id:
                union(local_face_id, previous_owner)

    roots = np.asarray([find(index) for index in range(len(candidate_ids))])
    component_roots = set(int(value) for value in roots.tolist())
    candidate_to_local = {
        face_id: local_id for local_id, face_id in enumerate(candidate_ids)
    }
    seeded_roots = {
        int(roots[candidate_to_local[face_id]]) for face_id in aperture_ids
    }
    seed_counts = {
        root: sum(
            int(roots[candidate_to_local[face_id]]) == root
            for face_id in aperture_ids
        )
        for root in seeded_roots
    }
    component_counts = Counter(int(root) for root in roots.tolist())
    if seeded_roots:
        significant_seeded_roots = {
            max(
                seeded_roots,
                key=lambda root: (
                    seed_counts[root],
                    component_counts[root],
                    -root,
                ),
            )
        }
    else:
        significant_seeded_roots = set()
    retained_local = np.asarray(
        [int(root) in significant_seeded_roots for root in roots],
        dtype=bool,
    )
    removed_face_ids = [
        candidate_ids[index]
        for index in np.flatnonzero(retained_local).tolist()
    ]
    discarded_seeded_roots = seeded_roots - significant_seeded_roots
    restored_roots = component_roots - significant_seeded_roots
    return MouthRemovalConnectivityReport(
        aperture_seed_face_count=len(aperture_ids),
        stretch_candidate_face_count=len(stretch_ids),
        candidate_face_count=len(candidate_ids),
        connected_component_count=len(component_roots),
        seeded_component_count=len(seeded_roots),
        significant_seeded_component_count=len(significant_seeded_roots),
        discarded_aperture_component_count=len(discarded_seeded_roots),
        discarded_aperture_face_count=sum(
            seed_counts[root] for root in discarded_seeded_roots
        ),
        removed_face_count=len(removed_face_ids),
        restored_disconnected_component_count=len(restored_roots),
        restored_disconnected_face_count=(
            len(candidate_ids) - len(removed_face_ids)
        ),
        removed_face_ids=removed_face_ids,
    )


def _undirected_cycle_basis(
    adjacency: Mapping[int, Sequence[int]],
) -> list[list[int]]:
    """Return edge-ordered Paton cycle-basis paths."""

    cycles: list[list[int]] = []
    remaining = set(adjacency)
    while remaining:
        root = min(remaining)
        stack = [root]
        predecessor = {root: root}
        used: dict[int, set[int]] = {root: set()}
        while stack:
            current = stack.pop()
            current_used = used[current]
            for neighbor in adjacency[current]:
                if neighbor not in used:
                    predecessor[neighbor] = current
                    stack.append(neighbor)
                    used[neighbor] = {current}
                elif neighbor == current:
                    cycles.append([current])
                elif neighbor not in current_used:
                    neighbor_used = used[neighbor]
                    cycle = [neighbor, current]
                    parent = predecessor[current]
                    while parent not in neighbor_used:
                        cycle.append(parent)
                        parent = predecessor[parent]
                    cycle.append(parent)
                    cycles.append(cycle)
                    used[neighbor].add(current)
        remaining.difference_update(predecessor)
    return cycles


def _ear_clip_projected_cycle(
    cycle: Sequence[int],
    projected_vertices: torch.Tensor,
) -> list[list[int]]:
    """Triangulate a simple projected cycle while retaining its winding."""

    points = projected_vertices[torch.tensor(cycle, dtype=torch.long)].double()
    signed_area_twice = float(
        (
            points[:, 0] * torch.roll(points[:, 1], shifts=-1)
            - points[:, 1] * torch.roll(points[:, 0], shifts=-1)
        ).sum().item()
    )
    if abs(signed_area_twice) <= 1.0e-12:
        return []
    orientation = 1.0 if signed_area_twice > 0.0 else -1.0
    remaining = list(range(len(cycle)))
    triangles: list[list[int]] = []

    def cross(first: int, second: int, third: int) -> float:
        first_edge = points[second] - points[first]
        second_edge = points[third] - points[first]
        return float(
            (first_edge[0] * second_edge[1] - first_edge[1] * second_edge[0]).item()
        )

    def inside_triangle(
        point: int,
        first: int,
        second: int,
        third: int,
    ) -> bool:
        return min(
            orientation * cross(first, second, point),
            orientation * cross(second, third, point),
            orientation * cross(third, first, point),
        ) >= -1.0e-12

    while len(remaining) > 3:
        found_ear = False
        for offset, current in enumerate(remaining):
            previous = remaining[offset - 1]
            following = remaining[(offset + 1) % len(remaining)]
            if orientation * cross(previous, current, following) <= 1.0e-12:
                continue
            if any(
                inside_triangle(point, previous, current, following)
                for point in remaining
                if point not in (previous, current, following)
            ):
                continue
            triangles.append(
                [int(cycle[previous]), int(cycle[current]), int(cycle[following])]
            )
            remaining.pop(offset)
            found_ear = True
            break
        if not found_ear:
            return []
    triangles.append([int(cycle[index]) for index in remaining])
    return triangles


def _mesh_boundary_edges(faces: torch.Tensor) -> torch.Tensor:
    faces = faces.detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Boundary detection expects triangular faces.")
    if faces.numel() == 0:
        return torch.empty((0, 2), dtype=torch.long)
    face_array = faces.numpy()
    edges = np.concatenate(
        (
            face_array[:, (0, 1)],
            face_array[:, (1, 2)],
            face_array[:, (2, 0)],
        ),
        axis=0,
    )
    edges.sort(axis=1)
    order = np.lexsort((edges[:, 1], edges[:, 0]))
    ordered_edges = edges[order]
    group_start = np.ones(len(ordered_edges), dtype=bool)
    group_start[1:] = np.any(ordered_edges[1:] != ordered_edges[:-1], axis=1)
    group_ids = np.flatnonzero(group_start)
    group_counts = np.diff(np.append(group_ids, len(ordered_edges)))
    boundary = ordered_edges[group_ids[group_counts == 1]]
    return torch.from_numpy(boundary.copy()).long().contiguous()


def _new_boundary_edge_mask(
    boundary_edges: torch.Tensor,
    reference_faces: torch.Tensor | None,
    *,
    reference_boundary_edges: torch.Tensor | None = None,
) -> tuple[torch.Tensor, int]:
    """Mark boundary edges that were not boundaries in the reference topology."""

    boundary_edges = boundary_edges.detach().cpu().long().contiguous()
    if reference_faces is None and reference_boundary_edges is None:
        return torch.ones(boundary_edges.shape[0], dtype=torch.bool), 0
    if reference_boundary_edges is None:
        assert reference_faces is not None
        reference_boundary_edges = _mesh_boundary_edges(reference_faces)
    else:
        reference_boundary_edges = (
            reference_boundary_edges.detach().cpu().long().contiguous()
        )
    if boundary_edges.numel() == 0 or reference_boundary_edges.numel() == 0:
        return (
            torch.ones(boundary_edges.shape[0], dtype=torch.bool),
            int(reference_boundary_edges.shape[0]),
        )
    maximum_vertex_id = max(
        int(boundary_edges.max().item()),
        int(reference_boundary_edges.max().item()),
    )
    key_base = maximum_vertex_id + 1
    boundary_keys = boundary_edges[:, 0] * key_base + boundary_edges[:, 1]
    reference_keys = (
        reference_boundary_edges[:, 0] * key_base
        + reference_boundary_edges[:, 1]
    )
    return (
        ~torch.isin(boundary_keys, reference_keys),
        int(reference_boundary_edges.shape[0]),
    )


def fill_secondary_mouth_boundary_holes(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    reference_faces: torch.Tensor | None = None,
    reference_boundary_edges: torch.Tensor | None = None,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth_padding: float = 0.12,
    maximum_area_ratio: float = 0.15,
    minimum_aperture_horizontal_span: float = 0.8,
    minimum_aperture_vertical_span: float = 0.8,
    edge_ratio_threshold: float = 3.0,
    edge_growth_threshold: float = 0.02,
    maximum_ear_clip_shifts: int = 8,
    minimum_new_boundary_edge_fraction: float = 0.5,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, MouthHoleFillReport]:
    """Cap small cut-created mouth cycles while preserving existing boundaries."""

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    if neutral_vertices.shape != deformed_vertices.shape:
        raise ValueError("Neutral and deformed vertices must have matching shapes.")
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Mouth hole filling expects triangular faces.")
    if depth_padding < 0.0:
        raise ValueError("Mouth hole depth padding cannot be negative.")
    if not 0.0 < maximum_area_ratio < 1.0:
        raise ValueError("Mouth hole maximum area ratio must be in (0, 1).")
    if maximum_ear_clip_shifts <= 0:
        raise ValueError("Maximum ear-clip shifts must be positive.")
    if not 0.0 <= minimum_new_boundary_edge_fraction <= 1.0:
        raise ValueError("Minimum new-boundary edge fraction must be in [0, 1].")

    vertex_count = int(deformed_vertices.shape[0])
    lip_ids = list(
        dict.fromkeys(
            _ordered_mapped_vertex_ids(
                mapping,
                INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
                vertex_count,
            )
        )
    )
    if len(lip_ids) < 4:
        raise ValueError("Mouth hole filling requires mapped inner-lip vertices.")

    boundary_edges = _mesh_boundary_edges(faces)
    new_boundary_mask, reference_boundary_edge_count = _new_boundary_edge_mask(
        boundary_edges,
        reference_faces,
        reference_boundary_edges=reference_boundary_edges,
    )
    new_boundary_edges = boundary_edges[new_boundary_mask]
    new_boundary_edge_keys = {
        (int(first), int(second))
        for first, second in new_boundary_edges.tolist()
    }
    boundary_midpoints = deformed_vertices[boundary_edges].mean(dim=1)
    lip_points = deformed_vertices[torch.tensor(lip_ids, dtype=torch.long)]
    projected_lips = lip_points[:, (horizontal_axis, vertical_axis)]
    projected_min = projected_lips.amin(dim=0)
    projected_max = projected_lips.amax(dim=0)
    projected_span = (projected_max - projected_min).clamp_min(1.0e-6)
    mouth_edge_mask = (
        (boundary_midpoints[:, horizontal_axis] >= projected_min[0] - 0.45 * projected_span[0])
        & (boundary_midpoints[:, horizontal_axis] <= projected_max[0] + 0.45 * projected_span[0])
        & (boundary_midpoints[:, vertical_axis] >= projected_min[1] - 0.45 * projected_span[1])
        & (boundary_midpoints[:, vertical_axis] <= projected_max[1] + 0.45 * projected_span[1])
        & (boundary_midpoints[:, depth_axis] >= lip_points[:, depth_axis].amin() - float(depth_padding))
        & (boundary_midpoints[:, depth_axis] <= lip_points[:, depth_axis].amax() + float(depth_padding))
    )
    mouth_boundary_edges = boundary_edges[mouth_edge_mask]
    adjacency: dict[int, list[int]] = {}
    for first, second in mouth_boundary_edges.tolist():
        adjacency.setdefault(int(first), []).append(int(second))
        adjacency.setdefault(int(second), []).append(int(first))
    cycles = [cycle for cycle in _undirected_cycle_basis(adjacency) if len(cycle) >= 3]

    cycle_metrics: list[tuple[list[int], float, float, float, float]] = []
    for cycle in cycles:
        points = deformed_vertices[torch.tensor(cycle, dtype=torch.long)]
        projected = points[:, (horizontal_axis, vertical_axis)]
        signed_area = 0.5 * float(
            (
                projected[:, 0] * torch.roll(projected[:, 1], shifts=-1)
                - projected[:, 1] * torch.roll(projected[:, 0], shifts=-1)
            ).sum().item()
        )
        horizontal_span = float(
            (points[:, horizontal_axis].amax() - points[:, horizontal_axis].amin()).item()
        )
        vertical_span = float(
            (points[:, vertical_axis].amax() - points[:, vertical_axis].amin()).item()
        )
        cycle_edge_count = len(cycle)
        new_cycle_edge_count = sum(
            (
                min(int(cycle[index]), int(cycle[(index + 1) % cycle_edge_count])),
                max(int(cycle[index]), int(cycle[(index + 1) % cycle_edge_count])),
            )
            in new_boundary_edge_keys
            for index in range(cycle_edge_count)
        )
        cycle_metrics.append(
            (
                cycle,
                abs(signed_area),
                horizontal_span,
                vertical_span,
                float(new_cycle_edge_count) / float(cycle_edge_count),
            )
        )

    ranked_cycles = sorted(
        (
            index
            for index, metrics in enumerate(cycle_metrics)
            if metrics[4] >= float(minimum_new_boundary_edge_fraction)
        ),
        key=lambda index: cycle_metrics[index][1],
        reverse=True,
    )
    primary_cycle_ids = set(ranked_cycles[:1])
    largest_cycle_area = (
        cycle_metrics[ranked_cycles[0]][1] if ranked_cycles else 0.0
    )
    minimum_area = float(projected_span.prod().item()) * 1.0e-7
    candidate_cycles: list[tuple[list[int], float]] = []
    preserved_count = 0
    preexisting_count = 0
    for index, (
        cycle,
        area,
        horizontal_span,
        vertical_span,
        new_boundary_fraction,
    ) in enumerate(cycle_metrics):
        spans_aperture = (
            horizontal_span
            >= float(minimum_aperture_horizontal_span) * float(projected_span[0].item())
            and vertical_span
            >= float(minimum_aperture_vertical_span) * float(projected_span[1].item())
        )
        is_secondary = (
            largest_cycle_area > 0.0
            and area >= minimum_area
            and area <= float(maximum_area_ratio) * largest_cycle_area
        )
        is_cut_created = (
            new_boundary_fraction >= float(minimum_new_boundary_edge_fraction)
        )
        if not is_cut_created:
            preexisting_count += 1
            preserved_count += 1
        elif index in primary_cycle_ids or spans_aperture or not is_secondary:
            preserved_count += 1
        else:
            candidate_cycles.append((cycle, area))

    lip_depth_center = lip_points[:, depth_axis].mean()
    front_direction = (
        -1.0
        if lip_depth_center - deformed_vertices[:, depth_axis].amin()
        < deformed_vertices[:, depth_axis].amax() - lip_depth_center
        else 1.0
    )
    projected_deformed = deformed_vertices[:, (horizontal_axis, vertical_axis)]
    added_faces: list[list[int]] = []
    added_neutral_vertices: list[torch.Tensor] = []
    added_deformed_vertices: list[torch.Tensor] = []
    filled_areas: list[float] = []
    rejected_count = 0
    reoriented_count = 0

    for cycle, area in candidate_cycles:
        cycle_points = deformed_vertices[torch.tensor(cycle, dtype=torch.long)]
        cycle_center = cycle_points.mean(dim=0)
        _, _, local_axes = torch.linalg.svd(
            cycle_points - cycle_center,
            full_matrices=False,
        )
        local_projection = (deformed_vertices - cycle_center) @ local_axes[:2].T
        best_faces: list[list[int]] = []
        best_score: tuple[int, float, float] | None = None
        face_deformed_vertices = deformed_vertices
        for winding in (cycle, list(reversed(cycle))):
            for projection in (projected_deformed, local_projection):
                shift_count = min(len(winding), int(maximum_ear_clip_shifts))
                shifts = sorted(
                    {
                        (index * len(winding)) // shift_count
                        for index in range(shift_count)
                    }
                )
                for shift in shifts:
                    rotated = winding[shift:] + winding[:shift]
                    candidate_faces = _ear_clip_projected_cycle(rotated, projection)
                    if not candidate_faces:
                        continue
                    candidate_tensor = torch.tensor(candidate_faces, dtype=torch.long)
                    neutral_triangles = neutral_vertices[candidate_tensor]
                    deformed_triangles = deformed_vertices[candidate_tensor]
                    neutral_normals = torch.cross(
                        neutral_triangles[:, 1] - neutral_triangles[:, 0],
                        neutral_triangles[:, 2] - neutral_triangles[:, 0],
                        dim=1,
                    )
                    deformed_normals = torch.cross(
                        deformed_triangles[:, 1] - deformed_triangles[:, 0],
                        deformed_triangles[:, 2] - deformed_triangles[:, 0],
                        dim=1,
                    )
                    neutral_lengths = torch.stack(
                        tuple(
                            (
                                neutral_triangles[:, (corner + 1) % 3]
                                - neutral_triangles[:, corner]
                            ).norm(dim=1)
                            for corner in range(3)
                        ),
                        dim=1,
                    )
                    deformed_lengths = torch.stack(
                        tuple(
                            (
                                deformed_triangles[:, (corner + 1) % 3]
                                - deformed_triangles[:, corner]
                            ).norm(dim=1)
                            for corner in range(3)
                        ),
                        dim=1,
                    )
                    severe_stretch = (
                        (deformed_lengths / neutral_lengths.clamp_min(1.0e-8)).amax(dim=1)
                        > float(edge_ratio_threshold)
                    ) & (
                        (deformed_lengths - neutral_lengths).amax(dim=1)
                        > float(edge_growth_threshold)
                    )
                    orientation = (
                        F.normalize(neutral_normals, dim=1, eps=1.0e-8)
                        * F.normalize(deformed_normals, dim=1, eps=1.0e-8)
                    ).sum(dim=1)
                    degenerate = (
                        (neutral_normals.norm(dim=1) <= 1.0e-10)
                        | (deformed_normals.norm(dim=1) <= 1.0e-10)
                    )
                    bad_count = int(
                        (severe_stretch | (orientation < 0.0) | degenerate).sum().item()
                    )
                    score = (
                        bad_count,
                        -float(orientation.min().item()),
                        float(deformed_lengths.max().item()),
                    )
                    if best_score is None or score < best_score:
                        best_score = score
                        best_faces = candidate_faces
                    if bad_count == 0 and float(orientation.min().item()) >= 0.25:
                        break
                if best_score is not None and best_score[0] == 0 and -best_score[1] >= 0.25:
                    break
            if best_score is not None and best_score[0] == 0 and -best_score[1] >= 0.25:
                break

        if best_score is None or best_score[0] > 0:
            neutral_center = neutral_vertices[
                torch.tensor(cycle, dtype=torch.long)
            ].mean(dim=0)
            deformed_center = deformed_vertices[
                torch.tensor(cycle, dtype=torch.long)
            ].mean(dim=0)
            center_id = vertex_count + len(added_neutral_vertices)
            fan_faces = [
                [cycle[index], cycle[(index + 1) % len(cycle)], center_id]
                for index in range(len(cycle))
            ]
            candidate_neutral = torch.cat(
                (
                    neutral_vertices,
                    torch.stack((*added_neutral_vertices, neutral_center)),
                ),
                dim=0,
            )
            candidate_deformed = torch.cat(
                (
                    deformed_vertices,
                    torch.stack((*added_deformed_vertices, deformed_center)),
                ),
                dim=0,
            )
            candidate_tensor = torch.tensor(fan_faces, dtype=torch.long)
            neutral_triangles = candidate_neutral[candidate_tensor]
            deformed_triangles = candidate_deformed[candidate_tensor]
            neutral_normals = torch.cross(
                neutral_triangles[:, 1] - neutral_triangles[:, 0],
                neutral_triangles[:, 2] - neutral_triangles[:, 0],
                dim=1,
            )
            deformed_normals = torch.cross(
                deformed_triangles[:, 1] - deformed_triangles[:, 0],
                deformed_triangles[:, 2] - deformed_triangles[:, 0],
                dim=1,
            )
            neutral_lengths = torch.stack(
                tuple(
                    (
                        neutral_triangles[:, (corner + 1) % 3]
                        - neutral_triangles[:, corner]
                    ).norm(dim=1)
                    for corner in range(3)
                ),
                dim=1,
            )
            deformed_lengths = torch.stack(
                tuple(
                    (
                        deformed_triangles[:, (corner + 1) % 3]
                        - deformed_triangles[:, corner]
                    ).norm(dim=1)
                    for corner in range(3)
                ),
                dim=1,
            )
            severe_stretch = (
                (deformed_lengths / neutral_lengths.clamp_min(1.0e-8)).amax(dim=1)
                > float(edge_ratio_threshold)
            ) & (
                (deformed_lengths - neutral_lengths).amax(dim=1)
                > float(edge_growth_threshold)
            )
            orientation = (
                F.normalize(neutral_normals, dim=1, eps=1.0e-8)
                * F.normalize(deformed_normals, dim=1, eps=1.0e-8)
            ).sum(dim=1)
            degenerate = (
                (neutral_normals.norm(dim=1) <= 1.0e-10)
                | (deformed_normals.norm(dim=1) <= 1.0e-10)
            )
            if bool(
                (severe_stretch | (orientation < 0.0) | degenerate).any().item()
            ):
                rejected_count += 1
                continue
            added_neutral_vertices.append(neutral_center)
            added_deformed_vertices.append(deformed_center)
            best_faces = fan_faces
            face_deformed_vertices = candidate_deformed
        oriented_faces = []
        for first, second, third in best_faces:
            normal = torch.cross(
                face_deformed_vertices[second] - face_deformed_vertices[first],
                face_deformed_vertices[third] - face_deformed_vertices[first],
                dim=0,
            )
            if float(normal[depth_axis].item()) * front_direction < 0.0:
                second, third = third, second
                reoriented_count += 1
            oriented_faces.append([first, second, third])
        best_faces = oriented_faces
        added_faces.extend(best_faces)
        filled_areas.append(float(area))

    final_neutral_vertices = (
        torch.cat((neutral_vertices, torch.stack(added_neutral_vertices)), dim=0)
        if added_neutral_vertices
        else neutral_vertices
    )
    final_deformed_vertices = (
        torch.cat((deformed_vertices, torch.stack(added_deformed_vertices)), dim=0)
        if added_deformed_vertices
        else deformed_vertices
    )
    final_faces = (
        torch.cat((faces, torch.tensor(added_faces, dtype=torch.long)), dim=0).contiguous()
        if added_faces
        else faces
    )
    final_boundary_edge_count = int(_mesh_boundary_edges(final_faces).shape[0])
    report = MouthHoleFillReport(
        boundary_edge_count_before=int(boundary_edges.shape[0]),
        reference_boundary_edge_count=int(reference_boundary_edge_count),
        new_boundary_edge_count=int(new_boundary_edges.shape[0]),
        mouth_boundary_edge_count_before=int(mouth_boundary_edges.shape[0]),
        mouth_cycle_count=len(cycles),
        preexisting_cycle_count=int(preexisting_count),
        preserved_aperture_cycle_count=preserved_count,
        candidate_hole_cycle_count=len(candidate_cycles),
        filled_hole_cycle_count=len(filled_areas),
        rejected_hole_cycle_count=rejected_count,
        added_face_count=len(added_faces),
        added_vertex_count=len(added_neutral_vertices),
        reoriented_face_count=reoriented_count,
        boundary_edge_count_after=final_boundary_edge_count,
        maximum_area_ratio=float(maximum_area_ratio),
        minimum_new_boundary_edge_fraction=float(
            minimum_new_boundary_edge_fraction
        ),
        largest_cycle_area=float(largest_cycle_area),
        filled_cycle_areas=filled_areas,
    )
    return final_neutral_vertices, final_deformed_vertices, final_faces, report


def bridge_excess_mouth_corner_gap(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    reference_faces: torch.Tensor | None = None,
    reference_boundary_edges: torch.Tensor | None = None,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth_padding: float = 0.12,
    corner_inset_ratio: float = 0.13,
    minimum_lateral_overshoot_ratio: float = 0.12,
    maximum_repaired_sides: int = 1,
    chain_padding_vertices: int = 1,
    minimum_horizontal_span: float = 0.8,
    minimum_vertical_span: float = 0.8,
    edge_ratio_threshold: float = 3.0,
    edge_growth_threshold: float = 0.02,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, MouthCornerBridgeReport]:
    """Bridge only a mouth corner whose cut loops extend past the lip contour."""

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    if neutral_vertices.shape != deformed_vertices.shape:
        raise ValueError("Neutral and deformed vertices must have matching shapes.")
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Mouth corner bridging expects triangular faces.")
    if not 0.0 < corner_inset_ratio < 0.5:
        raise ValueError("Mouth corner inset ratio must be in (0, 0.5).")
    if minimum_lateral_overshoot_ratio < 0.0:
        raise ValueError("Minimum lateral overshoot ratio cannot be negative.")
    if maximum_repaired_sides < 1:
        raise ValueError("At least one mouth side must be eligible for repair.")
    if chain_padding_vertices < 1:
        raise ValueError("Mouth corner chain padding must be at least one vertex.")

    vertex_count = int(deformed_vertices.shape[0])
    lip_ids = list(
        dict.fromkeys(
            _ordered_mapped_vertex_ids(
                mapping,
                INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
                vertex_count,
            )
        )
    )
    if len(lip_ids) < 4:
        raise ValueError("Mouth corner bridging requires mapped inner-lip vertices.")
    lip_points = deformed_vertices[torch.tensor(lip_ids, dtype=torch.long)]
    projected_lips = lip_points[:, (horizontal_axis, vertical_axis)]
    projected_min = projected_lips.amin(dim=0)
    projected_max = projected_lips.amax(dim=0)
    projected_span = (projected_max - projected_min).clamp_min(1.0e-6)

    boundary_edges = _mesh_boundary_edges(faces)
    new_boundary_mask, _ = _new_boundary_edge_mask(
        boundary_edges,
        reference_faces,
        reference_boundary_edges=reference_boundary_edges,
    )
    boundary_midpoints = deformed_vertices[boundary_edges].mean(dim=1)
    mouth_edge_mask = (
        (boundary_midpoints[:, horizontal_axis] >= projected_min[0] - 0.45 * projected_span[0])
        & (boundary_midpoints[:, horizontal_axis] <= projected_max[0] + 0.45 * projected_span[0])
        & (boundary_midpoints[:, vertical_axis] >= projected_min[1] - 0.45 * projected_span[1])
        & (boundary_midpoints[:, vertical_axis] <= projected_max[1] + 0.45 * projected_span[1])
        & (boundary_midpoints[:, depth_axis] >= lip_points[:, depth_axis].amin() - float(depth_padding))
        & (boundary_midpoints[:, depth_axis] <= lip_points[:, depth_axis].amax() + float(depth_padding))
    )
    mouth_boundary_edges = boundary_edges[mouth_edge_mask & new_boundary_mask]
    adjacency: dict[int, list[int]] = {}
    for first, second in mouth_boundary_edges.tolist():
        adjacency.setdefault(int(first), []).append(int(second))
        adjacency.setdefault(int(second), []).append(int(first))

    components: list[list[int]] = []
    visited: set[int] = set()
    for start in adjacency:
        if start in visited:
            continue
        stack = [start]
        visited.add(start)
        component: list[int] = []
        while stack:
            current = stack.pop()
            component.append(current)
            for neighbor in adjacency[current]:
                if neighbor not in visited:
                    visited.add(neighbor)
                    stack.append(neighbor)
        components.append(component)

    def ordered_cycle(component: list[int]) -> list[int]:
        if len(component) < 3 or any(len(adjacency[value]) != 2 for value in component):
            return []
        start = min(component)
        order = [start]
        previous = -1
        current = start
        while True:
            candidates = [value for value in adjacency[current] if value != previous]
            if not candidates:
                return []
            following = candidates[0]
            if following == start:
                break
            if following in order:
                return []
            order.append(following)
            previous, current = current, following
        return order if len(order) == len(component) else []

    mouth_cycles: list[list[int]] = []
    for component in components:
        cycle = ordered_cycle(component)
        if not cycle:
            continue
        points = deformed_vertices[torch.tensor(cycle, dtype=torch.long)]
        horizontal_span = points[:, horizontal_axis].amax() - points[:, horizontal_axis].amin()
        vertical_span = points[:, vertical_axis].amax() - points[:, vertical_axis].amin()
        if (
            horizontal_span >= float(minimum_horizontal_span) * projected_span[0]
            and vertical_span >= float(minimum_vertical_span) * projected_span[1]
        ):
            mouth_cycles.append(cycle)

    def projected_cycle_area(cycle: list[int]) -> float:
        points = deformed_vertices[
            torch.tensor(cycle, dtype=torch.long)
        ][:, (horizontal_axis, vertical_axis)]
        return abs(
            0.5
            * float(
                (
                    points[:, 0] * torch.roll(points[:, 1], shifts=-1)
                    - points[:, 1] * torch.roll(points[:, 0], shifts=-1)
                ).sum().item()
            )
        )

    mouth_cycles = sorted(mouth_cycles, key=projected_cycle_area, reverse=True)[:2]
    lateral_overshoot_ratios = {"left": 0.0, "right": 0.0}
    if len(mouth_cycles) == 2:
        cycle_ids = torch.tensor(
            mouth_cycles[0] + mouth_cycles[1],
            dtype=torch.long,
        )
        cycle_horizontal = deformed_vertices[cycle_ids, horizontal_axis]
        lateral_overshoot_ratios = {
            "left": max(
                0.0,
                float((projected_min[0] - cycle_horizontal.amin()).item())
                / float(projected_span[0].item()),
            ),
            "right": max(
                0.0,
                float((cycle_horizontal.amax() - projected_max[0]).item())
                / float(projected_span[0].item()),
            ),
        }
    candidate_sides = [
        side
        for side in sorted(
            lateral_overshoot_ratios,
            key=lateral_overshoot_ratios.get,
            reverse=True,
        )
        if lateral_overshoot_ratios[side] >= float(minimum_lateral_overshoot_ratio)
    ][: int(maximum_repaired_sides)]

    def longest_corner_chain(cycle: list[int], side: str) -> list[int]:
        if side == "left":
            threshold = projected_min[0] + float(corner_inset_ratio) * projected_span[0]
            selected = [
                bool((deformed_vertices[value, horizontal_axis] <= threshold).item())
                for value in cycle
            ]
        else:
            threshold = projected_max[0] - float(corner_inset_ratio) * projected_span[0]
            selected = [
                bool((deformed_vertices[value, horizontal_axis] >= threshold).item())
                for value in cycle
            ]
        starts = [
            index
            for index, value in enumerate(selected)
            if value and not selected[index - 1]
        ]
        runs: list[list[int]] = []
        for start in starts:
            run = []
            index = start
            while selected[index]:
                run.append(cycle[index])
                index = (index + 1) % len(cycle)
                if index == start:
                    break
            runs.append(run)
        if not runs:
            return []
        chain = max(runs, key=len)
        positions = {value: index for index, value in enumerate(cycle)}
        start_index = positions[chain[0]]
        end_index = positions[chain[-1]]
        padded = [
            cycle[(start_index - offset) % len(cycle)]
            for offset in range(chain_padding_vertices, 0, -1)
        ]
        padded.extend(chain)
        padded.extend(
            cycle[(end_index + offset) % len(cycle)]
            for offset in range(1, chain_padding_vertices + 1)
        )
        chain = list(dict.fromkeys(padded))
        if (
            deformed_vertices[chain[0], vertical_axis]
            < deformed_vertices[chain[-1], vertical_axis]
        ):
            chain.reverse()
        return chain

    def triangle_quality(vertex_ids: list[int]) -> tuple[float, bool, bool, bool]:
        index = torch.tensor(vertex_ids, dtype=torch.long)
        neutral_triangle = neutral_vertices[index]
        deformed_triangle = deformed_vertices[index]
        neutral_normal = torch.cross(
            neutral_triangle[1] - neutral_triangle[0],
            neutral_triangle[2] - neutral_triangle[0],
            dim=0,
        )
        deformed_normal = torch.cross(
            deformed_triangle[1] - deformed_triangle[0],
            deformed_triangle[2] - deformed_triangle[0],
            dim=0,
        )
        neutral_lengths = torch.stack(
            tuple(
                (
                    neutral_triangle[(corner + 1) % 3]
                    - neutral_triangle[corner]
                ).norm()
                for corner in range(3)
            )
        )
        deformed_lengths = torch.stack(
            tuple(
                (
                    deformed_triangle[(corner + 1) % 3]
                    - deformed_triangle[corner]
                ).norm()
                for corner in range(3)
            )
        )
        degenerate = bool(
            (neutral_normal.norm() <= 1.0e-10).item()
            or (deformed_normal.norm() <= 1.0e-10).item()
        )
        orientation = float(
            (
                F.normalize(neutral_normal, dim=0, eps=1.0e-8)
                * F.normalize(deformed_normal, dim=0, eps=1.0e-8)
            ).sum().item()
        )
        flipped = orientation < 0.0
        severe = bool(
            (
                (deformed_lengths / neutral_lengths.clamp_min(1.0e-8)).amax()
                > float(edge_ratio_threshold)
            ).item()
            and (
                (deformed_lengths - neutral_lengths).amax()
                > float(edge_growth_threshold)
            ).item()
        )
        bad_count = int(degenerate) + int(flipped) + int(severe)
        scale = float(projected_span.max().item())
        cost = (
            bad_count * 1.0e6
            + max(0.0, 0.25 - orientation) * 1.0e4
            + float(deformed_lengths.max().item()) / scale * 10.0
            + float(neutral_lengths.max().item()) / scale * 5.0
        )
        return cost, severe, flipped, degenerate

    added_faces: list[list[int]] = []
    repaired_sides: list[str] = []
    reoriented_count = 0
    lip_depth_center = lip_points[:, depth_axis].mean()
    front_direction = (
        -1.0
        if lip_depth_center - deformed_vertices[:, depth_axis].amin()
        < deformed_vertices[:, depth_axis].amax() - lip_depth_center
        else 1.0
    )
    for side in candidate_sides:
        chains = [longest_corner_chain(cycle, side) for cycle in mouth_cycles]
        if len(chains) != 2 or any(len(chain) < 3 for chain in chains):
            continue
        first, second = chains
        same_direction_cost = float(
            (
                deformed_vertices[first[0]] - deformed_vertices[second[0]]
            ).norm().item()
            + (
                deformed_vertices[first[-1]] - deformed_vertices[second[-1]]
            ).norm().item()
        )
        reversed_cost = float(
            (
                deformed_vertices[first[0]] - deformed_vertices[second[-1]]
            ).norm().item()
            + (
                deformed_vertices[first[-1]] - deformed_vertices[second[0]]
            ).norm().item()
        )
        if reversed_cost < same_direction_cost:
            second = list(reversed(second))

        first_count = len(first)
        second_count = len(second)
        costs = [[float("inf")] * second_count for _ in range(first_count)]
        previous: list[list[tuple[int, int, list[int]] | None]] = [
            [None] * second_count for _ in range(first_count)
        ]
        costs[0][0] = 0.0
        for first_index in range(first_count):
            for second_index in range(second_count):
                current_cost = costs[first_index][second_index]
                if not np.isfinite(current_cost):
                    continue
                if first_index + 1 < first_count:
                    triangle = [
                        first[first_index],
                        first[first_index + 1],
                        second[second_index],
                    ]
                    candidate_cost = current_cost + triangle_quality(triangle)[0]
                    if candidate_cost < costs[first_index + 1][second_index]:
                        costs[first_index + 1][second_index] = candidate_cost
                        previous[first_index + 1][second_index] = (
                            first_index,
                            second_index,
                            triangle,
                        )
                if second_index + 1 < second_count:
                    triangle = [
                        first[first_index],
                        second[second_index + 1],
                        second[second_index],
                    ]
                    candidate_cost = current_cost + triangle_quality(triangle)[0]
                    if candidate_cost < costs[first_index][second_index + 1]:
                        costs[first_index][second_index + 1] = candidate_cost
                        previous[first_index][second_index + 1] = (
                            first_index,
                            second_index,
                            triangle,
                        )

        side_faces: list[list[int]] = []
        first_index = first_count - 1
        second_index = second_count - 1
        while first_index > 0 or second_index > 0:
            step = previous[first_index][second_index]
            if step is None:
                side_faces = []
                break
            prior_first, prior_second, triangle = step
            side_faces.append(triangle)
            first_index, second_index = prior_first, prior_second
        if not side_faces:
            continue
        side_faces.reverse()
        for first_vertex, second_vertex, third_vertex in side_faces:
            normal = torch.cross(
                deformed_vertices[second_vertex] - deformed_vertices[first_vertex],
                deformed_vertices[third_vertex] - deformed_vertices[first_vertex],
                dim=0,
            )
            if float(normal[depth_axis].item()) * front_direction < 0.0:
                second_vertex, third_vertex = third_vertex, second_vertex
                reoriented_count += 1
            added_faces.append([first_vertex, second_vertex, third_vertex])
        repaired_sides.append(side)

    severe_count = 0
    flipped_count = 0
    if added_faces:
        for triangle in added_faces:
            _, severe, flipped, _ = triangle_quality(triangle)
            severe_count += int(severe)
            flipped_count += int(flipped)
        final_faces = torch.cat(
            (faces, torch.tensor(added_faces, dtype=torch.long)),
            dim=0,
        ).contiguous()
    else:
        final_faces = faces
    final_boundary_edge_count = int(_mesh_boundary_edges(final_faces).shape[0])
    report = MouthCornerBridgeReport(
        boundary_edge_count_before=int(boundary_edges.shape[0]),
        boundary_edge_count_after=final_boundary_edge_count,
        mouth_boundary_component_count=len(mouth_cycles),
        candidate_side_count=len(candidate_sides),
        repaired_sides=repaired_sides,
        added_face_count=len(added_faces),
        reoriented_face_count=reoriented_count,
        severe_added_face_count=severe_count,
        flipped_added_face_count=flipped_count,
        corner_inset_ratio=float(corner_inset_ratio),
        chain_padding_vertices=int(chain_padding_vertices),
        minimum_lateral_overshoot_ratio=float(minimum_lateral_overshoot_ratio),
        lateral_overshoot_ratios=lateral_overshoot_ratios,
    )
    return neutral_vertices, deformed_vertices, final_faces, report


def smooth_mouth_boundary_loops(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    reference_faces: torch.Tensor | None = None,
    reference_boundary_edges: torch.Tensor | None = None,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth_padding: float = 0.12,
    minimum_horizontal_span: float = 0.8,
    minimum_vertical_span: float = 0.8,
    smoothing_cycles: int = 8,
    taubin_lambda: float = 0.4,
    taubin_mu: float = -0.42,
    maximum_displacement: float = 0.006,
    blend_rings: int = 5,
    blend_iterations: int = 35,
    preserve_boundary_junctions: bool = False,
    arc_length_weighted: bool = False,
    minimum_component_span: float | None = None,
    minimum_component_vertices: int = 3,
) -> tuple[torch.Tensor, torch.Tensor, MouthBoundarySmoothReport]:
    """Smooth full mouth cut loops and diffuse the correction into nearby faces."""

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    vertex_count = int(deformed_vertices.shape[0])
    lip_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_UPPER_LIP_MEDIAPIPE_IDS + INNER_LOWER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    lip_ids = list(dict.fromkeys(lip_ids))
    if len(lip_ids) < 4:
        raise ValueError("Boundary smoothing requires mapped inner-lip vertices.")
    if smoothing_cycles < 1 or blend_rings < 1 or blend_iterations < 1:
        raise ValueError("Boundary smoothing iteration and ring counts must be positive.")
    if maximum_displacement <= 0.0:
        raise ValueError("Boundary smoothing maximum displacement must be positive.")
    if minimum_component_span is not None and minimum_component_span <= 0.0:
        raise ValueError("Boundary component span must be positive.")
    if minimum_component_vertices < 3:
        raise ValueError("Boundary components must contain at least three vertices.")

    boundary_edges = _mesh_boundary_edges(faces)
    new_boundary_mask, _ = _new_boundary_edge_mask(
        boundary_edges,
        reference_faces,
        reference_boundary_edges=reference_boundary_edges,
    )
    boundary_midpoints = deformed_vertices[boundary_edges].mean(dim=1)
    lip_points = deformed_vertices[torch.tensor(lip_ids, dtype=torch.long)]
    projected_lips = lip_points[:, (horizontal_axis, vertical_axis)]
    projected_min = projected_lips.amin(dim=0)
    projected_max = projected_lips.amax(dim=0)
    projected_span = (projected_max - projected_min).clamp_min(1.0e-6)
    mouth_edge_mask = (
        (boundary_midpoints[:, horizontal_axis] >= projected_min[0] - 0.45 * projected_span[0])
        & (boundary_midpoints[:, horizontal_axis] <= projected_max[0] + 0.45 * projected_span[0])
        & (boundary_midpoints[:, vertical_axis] >= projected_min[1] - 0.45 * projected_span[1])
        & (boundary_midpoints[:, vertical_axis] <= projected_max[1] + 0.45 * projected_span[1])
        & (boundary_midpoints[:, depth_axis] >= lip_points[:, depth_axis].amin() - float(depth_padding))
        & (boundary_midpoints[:, depth_axis] <= lip_points[:, depth_axis].amax() + float(depth_padding))
    )
    mouth_boundary_edges = boundary_edges[mouth_edge_mask & new_boundary_mask]
    adjacency: dict[int, list[int]] = {}
    for first, second in mouth_boundary_edges.tolist():
        adjacency.setdefault(int(first), []).append(int(second))
        adjacency.setdefault(int(second), []).append(int(first))
    components: list[list[int]] = []
    visited: set[int] = set()
    for start in adjacency:
        if start in visited:
            continue
        stack = [start]
        visited.add(start)
        component: list[int] = []
        while stack:
            current = stack.pop()
            component.append(current)
            for neighbor in adjacency[current]:
                if neighbor not in visited:
                    visited.add(neighbor)
                    stack.append(neighbor)
        components.append(component)
    selected_components = []
    for component in components:
        points = deformed_vertices[torch.tensor(component, dtype=torch.long)]
        horizontal_span = points[:, horizontal_axis].amax() - points[:, horizontal_axis].amin()
        vertical_span = points[:, vertical_axis].amax() - points[:, vertical_axis].amin()
        spans_full_aperture = (
            horizontal_span >= float(minimum_horizontal_span) * projected_span[0]
            and vertical_span >= float(minimum_vertical_span) * projected_span[1]
        )
        horizontal_ratio = float((horizontal_span / projected_span[0]).item())
        vertical_ratio = float((vertical_span / projected_span[1]).item())
        spans_fragment = (
            minimum_component_span is not None
            and len(component) >= int(minimum_component_vertices)
            and max(horizontal_ratio, vertical_ratio)
            >= float(minimum_component_span)
        )
        if spans_full_aperture or spans_fragment:
            selected_components.append(component)
    boundary_ids = sorted({value for component in selected_components for value in component})
    smoothed_neutral = neutral_vertices.clone()
    smoothed_deformed = deformed_vertices.clone()
    blended_vertex_count = 0
    displacement_norms = torch.empty(0)
    quality_rejected_faces: set[int] = set()
    quality_restored_vertices: set[int] = set()
    preserved_boundary_junction_count = 0
    if boundary_ids:
        boundary_tensor = torch.tensor(boundary_ids, dtype=torch.long)
        if preserve_boundary_junctions:
            preserved_boundary_junction_count = sum(
                len(adjacency[vertex_id]) != 2 for vertex_id in boundary_ids
            )
        boundary_local_by_global = {
            vertex_id: local_id
            for local_id, vertex_id in enumerate(boundary_ids)
        }
        boundary_edge_sources: list[int] = []
        boundary_edge_targets: list[int] = []
        two_neighbor_vertices: list[int] = []
        two_neighbor_first: list[int] = []
        two_neighbor_second: list[int] = []
        boundary_degrees = torch.empty(len(boundary_ids), dtype=torch.long)
        for local_id, vertex_id in enumerate(boundary_ids):
            neighbor_ids = [
                boundary_local_by_global[value]
                for value in adjacency[vertex_id]
            ]
            boundary_degrees[local_id] = len(neighbor_ids)
            boundary_edge_sources.extend([local_id] * len(neighbor_ids))
            boundary_edge_targets.extend(neighbor_ids)
            if len(neighbor_ids) == 2:
                two_neighbor_vertices.append(local_id)
                two_neighbor_first.append(neighbor_ids[0])
                two_neighbor_second.append(neighbor_ids[1])
        boundary_edge_sources_tensor = torch.tensor(
            boundary_edge_sources,
            dtype=torch.long,
        )
        boundary_edge_targets_tensor = torch.tensor(
            boundary_edge_targets,
            dtype=torch.long,
        )
        boundary_movable = boundary_degrees > 0
        if preserve_boundary_junctions:
            boundary_movable &= boundary_degrees == 2
        two_neighbor_vertices_tensor = torch.tensor(
            two_neighbor_vertices,
            dtype=torch.long,
        )
        two_neighbor_first_tensor = torch.tensor(
            two_neighbor_first,
            dtype=torch.long,
        )
        two_neighbor_second_tensor = torch.tensor(
            two_neighbor_second,
            dtype=torch.long,
        )

        def boundary_displacement(vertices: torch.Tensor) -> torch.Tensor:
            original = vertices[boundary_tensor][
                :, (horizontal_axis, vertical_axis)
            ]
            projected = original.clone()
            for _ in range(int(smoothing_cycles)):
                for factor in (float(taubin_lambda), float(taubin_mu)):
                    previous = projected.clone()
                    updated = previous.clone()
                    neighbor_totals = torch.zeros_like(previous)
                    neighbor_totals.index_add_(
                        0,
                        boundary_edge_sources_tensor,
                        previous[boundary_edge_targets_tensor],
                    )
                    neighbor_target = neighbor_totals / boundary_degrees.clamp_min(
                        1
                    ).unsqueeze(1)
                    if arc_length_weighted and two_neighbor_vertices:
                        centers = previous[two_neighbor_vertices_tensor]
                        first_points = previous[two_neighbor_first_tensor]
                        second_points = previous[two_neighbor_second_tensor]
                        first_distances = (first_points - centers).norm(dim=1)
                        second_distances = (second_points - centers).norm(dim=1)
                        distance_sum = first_distances + second_distances
                        weighted = (
                            first_points * second_distances.unsqueeze(1)
                            + second_points * first_distances.unsqueeze(1)
                        ) / distance_sum.clamp_min(1.0e-8).unsqueeze(1)
                        valid_weight = distance_sum > 1.0e-8
                        neighbor_target[
                            two_neighbor_vertices_tensor[valid_weight]
                        ] = weighted[valid_weight]
                    updated[boundary_movable] = previous[boundary_movable] + factor * (
                        neighbor_target[boundary_movable]
                        - previous[boundary_movable]
                    )
                    projected = updated
            displacement = projected - original
            lengths = displacement.norm(dim=1, keepdim=True)
            displacement *= (
                float(maximum_displacement) / lengths.clamp_min(float(maximum_displacement))
            ).clamp_max(1.0)
            return displacement

        deformed_seed = boundary_displacement(deformed_vertices)
        neutral_seed = boundary_displacement(neutral_vertices)
        displacement_norms = deformed_seed.norm(dim=1)
        ring_distance = torch.full((vertex_count,), -1, dtype=torch.long)
        ring_distance[boundary_tensor] = 0
        frontier = torch.zeros(vertex_count, dtype=torch.bool)
        frontier[boundary_tensor] = True
        for ring in range(1, int(blend_rings) + 1):
            touching_faces = frontier[faces].any(dim=1)
            neighbors = faces[touching_faces].reshape(-1)
            fresh = torch.unique(neighbors[ring_distance[neighbors] < 0])
            if fresh.numel() == 0:
                break
            frontier = torch.zeros_like(frontier)
            frontier[fresh] = True
            ring_distance[fresh] = ring
        blend_region = (ring_distance >= 0) & (ring_distance < int(blend_rings))
        local_faces = faces[blend_region[faces].any(dim=1)]
        local_directed_edges = torch.cat(
            (
                local_faces[:, (0, 1)], local_faces[:, (1, 0)],
                local_faces[:, (1, 2)], local_faces[:, (2, 1)],
                local_faces[:, (2, 0)], local_faces[:, (0, 2)],
            ),
            dim=0,
        )
        local_edges = local_directed_edges[
            blend_region[local_directed_edges[:, 0]]
        ]
        blend_ids = torch.nonzero(blend_region, as_tuple=False).flatten()
        blend_local_by_global = torch.full(
            (vertex_count,),
            -1,
            dtype=torch.long,
        )
        blend_local_by_global[blend_ids] = torch.arange(len(blend_ids))
        local_sources = blend_local_by_global[local_edges[:, 0]]
        local_targets = blend_local_by_global[local_edges[:, 1]]
        local_target_mask = local_targets >= 0
        local_counts = torch.zeros(len(blend_ids), dtype=torch.float32)
        local_counts.index_add_(
            0,
            local_sources,
            torch.ones(local_sources.shape[0], dtype=torch.float32),
        )
        local_boundary_ids = blend_local_by_global[boundary_tensor]
        local_movable = ring_distance[blend_ids] > 0
        from scipy import sparse

        valid_sources = local_sources[local_target_mask].numpy()
        valid_targets = local_targets[local_target_mask].numpy()
        averaging_weights = (
            1.0 / local_counts[local_sources[local_target_mask]].numpy()
        ).astype(np.float32, copy=False)
        averaging_matrix = sparse.coo_matrix(
            (
                averaging_weights,
                (valid_sources, valid_targets),
            ),
            shape=(len(blend_ids), len(blend_ids)),
            dtype=np.float32,
        ).tocsr()
        local_boundary_array = local_boundary_ids.numpy()
        local_movable_array = local_movable.numpy()

        def diffuse(seed: torch.Tensor) -> torch.Tensor:
            displacement = np.zeros((len(blend_ids), 2), dtype=np.float32)
            seed_array = seed.numpy()
            displacement[local_boundary_array] = seed_array
            for _ in range(int(blend_iterations)):
                average = averaging_matrix @ displacement
                displacement[local_movable_array] = average[local_movable_array]
                displacement[local_boundary_array] = seed_array
            return torch.from_numpy(displacement)

        deformed_displacement = diffuse(deformed_seed)
        neutral_displacement = diffuse(neutral_seed)
        smoothed_deformed[blend_ids, horizontal_axis] += deformed_displacement[:, 0]
        smoothed_deformed[blend_ids, vertical_axis] += deformed_displacement[:, 1]
        smoothed_neutral[blend_ids, horizontal_axis] += neutral_displacement[:, 0]
        smoothed_neutral[blend_ids, vertical_axis] += neutral_displacement[:, 1]
        blended_vertex_count = int(blend_ids.numel())

        quality_face_ids = torch.nonzero(
            blend_region[faces].any(dim=1),
            as_tuple=False,
        ).flatten()
        quality_faces = faces[quality_face_ids]
        original_triangles = deformed_vertices[quality_faces]
        original_normals = torch.cross(
            original_triangles[:, 1] - original_triangles[:, 0],
            original_triangles[:, 2] - original_triangles[:, 0],
            dim=1,
        )
        for _ in range(6):
            neutral_triangles = smoothed_neutral[quality_faces]
            deformed_triangles = smoothed_deformed[quality_faces]
            smoothed_normals = torch.cross(
                deformed_triangles[:, 1] - deformed_triangles[:, 0],
                deformed_triangles[:, 2] - deformed_triangles[:, 0],
                dim=1,
            )
            orientation_dot = (
                F.normalize(original_normals, dim=1, eps=1.0e-8)
                * F.normalize(smoothed_normals, dim=1, eps=1.0e-8)
            ).sum(dim=1)
            neutral_edges = torch.stack(
                tuple(
                    (
                        neutral_triangles[:, (corner + 1) % 3]
                        - neutral_triangles[:, corner]
                    ).norm(dim=1)
                    for corner in range(3)
                ),
                dim=1,
            )
            deformed_edges = torch.stack(
                tuple(
                    (
                        deformed_triangles[:, (corner + 1) % 3]
                        - deformed_triangles[:, corner]
                    ).norm(dim=1)
                    for corner in range(3)
                ),
                dim=1,
            )
            severe = (
                (deformed_edges / neutral_edges.clamp_min(1.0e-8)).amax(dim=1) > 3.0
            ) & ((deformed_edges - neutral_edges).amax(dim=1) > 0.02)
            rejected = torch.nonzero(
                (orientation_dot < 0.0) | severe,
                as_tuple=False,
            ).flatten()
            if rejected.numel() == 0:
                break
            quality_rejected_faces.update(
                int(value) for value in quality_face_ids[rejected].tolist()
            )
            restore_ids = torch.unique(quality_faces[rejected].reshape(-1))
            neighbor_mask = torch.isin(local_directed_edges[:, 0], restore_ids)
            restore_ids = torch.unique(
                torch.cat((restore_ids, local_directed_edges[neighbor_mask, 1]))
            )
            quality_restored_vertices.update(int(value) for value in restore_ids.tolist())
            smoothed_neutral[restore_ids] = neutral_vertices[restore_ids]
            smoothed_deformed[restore_ids] = deformed_vertices[restore_ids]
    report = MouthBoundarySmoothReport(
        boundary_edge_count=int(boundary_edges.shape[0]),
        mouth_boundary_edge_count=int(mouth_boundary_edges.shape[0]),
        mouth_boundary_component_count=int(len(components)),
        smoothed_component_count=int(len(selected_components)),
        smoothed_boundary_vertex_count=int(len(boundary_ids)),
        blended_vertex_count=int(blended_vertex_count),
        blend_rings=int(blend_rings),
        smoothing_cycles=int(smoothing_cycles),
        maximum_displacement=float(maximum_displacement),
        mean_boundary_displacement=float(displacement_norms.mean().item()) if displacement_norms.numel() else 0.0,
        max_boundary_displacement=float(displacement_norms.max().item()) if displacement_norms.numel() else 0.0,
        quality_rejected_face_count=int(len(quality_rejected_faces)),
        quality_restored_vertex_count=int(len(quality_restored_vertices)),
        preserve_boundary_junctions=bool(preserve_boundary_junctions),
        preserved_boundary_junction_count=int(preserved_boundary_junction_count),
        arc_length_weighted=bool(arc_length_weighted),
        fragmented_selection_enabled=minimum_component_span is not None,
        minimum_component_span=(
            float(minimum_component_span)
            if minimum_component_span is not None
            else None
        ),
        minimum_component_vertices=int(minimum_component_vertices),
    )
    return smoothed_neutral.contiguous(), smoothed_deformed.contiguous(), report


def clip_mesh_to_smooth_mouth_contour(
    neutral_vertices: torch.Tensor,
    deformed_vertices: torch.Tensor,
    source_faces: torch.Tensor,
    kept_faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    contour_scale: float = 1.0,
    depth_padding: float = 0.08,
    edge_ratio_threshold: float = 3.0,
    edge_growth_threshold: float = 0.02,
    curve_samples: int = 128,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, MouthContourClipReport]:
    """Replace a whole-face mouth cut with a spline-clipped triangle boundary."""

    neutral_vertices = neutral_vertices.detach().cpu().float().contiguous()
    deformed_vertices = deformed_vertices.detach().cpu().float().contiguous()
    source_faces = source_faces.detach().cpu().long().contiguous()
    kept_faces = kept_faces.detach().cpu().long().contiguous()
    if neutral_vertices.shape != deformed_vertices.shape:
        raise ValueError("Neutral and deformed vertices must have matching shapes.")
    if source_faces.ndim != 2 or source_faces.shape[1] != 3:
        raise ValueError("source_faces must contain triangles [F, 3].")
    if kept_faces.ndim != 2 or kept_faces.shape[1] != 3:
        raise ValueError("kept_faces must contain triangles [F, 3].")
    if contour_scale <= 0.0:
        raise ValueError("Mouth contour scale must be positive.")
    if depth_padding < 0.0:
        raise ValueError("Mouth contour depth padding cannot be negative.")
    if curve_samples < 16:
        raise ValueError("Mouth contour requires at least 16 curve samples.")

    vertex_count = int(deformed_vertices.shape[0])
    upper_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_UPPER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    lower_ids = _ordered_mapped_vertex_ids(
        mapping,
        INNER_LOWER_LIP_MEDIAPIPE_IDS,
        vertex_count,
    )
    if len(upper_ids) < 3 or len(lower_ids) < 3:
        raise ValueError("Smooth contour clipping requires both inner-lip curves.")
    upper_points = deformed_vertices[torch.tensor(upper_ids, dtype=torch.long)]
    lower_points = deformed_vertices[torch.tensor(lower_ids, dtype=torch.long)]
    contour = _smooth_mouth_contour_field(
        deformed_vertices[:, horizontal_axis],
        deformed_vertices[:, vertical_axis],
        upper_points[:, (horizontal_axis, vertical_axis)],
        lower_points[:, (horizontal_axis, vertical_axis)],
        contour_scale=float(contour_scale),
        curve_samples=int(curve_samples),
    )

    source_neutral_triangles = neutral_vertices[source_faces]
    source_deformed_triangles = deformed_vertices[source_faces]
    face_centers = source_deformed_triangles.mean(dim=1)
    lip_depths = torch.cat((upper_points[:, depth_axis], lower_points[:, depth_axis]))
    front_depth = (
        (face_centers[:, depth_axis] >= lip_depths.amin() - float(depth_padding))
        & (face_centers[:, depth_axis] <= lip_depths.amax() + float(depth_padding))
    )
    neutral_edges = torch.stack(
        tuple(
            (
                source_neutral_triangles[:, (corner + 1) % 3]
                - source_neutral_triangles[:, corner]
            ).norm(dim=1)
            for corner in range(3)
        ),
        dim=1,
    )
    deformed_edges = torch.stack(
        tuple(
            (
                source_deformed_triangles[:, (corner + 1) % 3]
                - source_deformed_triangles[:, corner]
            ).norm(dim=1)
            for corner in range(3)
        ),
        dim=1,
    )
    severe_stretch = (
        (deformed_edges / neutral_edges.clamp_min(1.0e-8)).amax(dim=1)
        > float(edge_ratio_threshold)
    ) & (
        (deformed_edges - neutral_edges).amax(dim=1)
        > float(edge_growth_threshold)
    )

    face_key_base = int(neutral_vertices.shape[0]) + 1
    source_keys = (
        (source_faces[:, 0] * face_key_base + source_faces[:, 1])
        * face_key_base
        + source_faces[:, 2]
    )
    kept_keys = (
        (kept_faces[:, 0] * face_key_base + kept_faces[:, 1])
        * face_key_base
        + kept_faces[:, 2]
    )
    kept_mask = torch.from_numpy(
        np.isin(
            source_keys.numpy(),
            kept_keys.numpy(),
            assume_unique=True,
        )
    )
    face_inside = contour[source_faces] > 1.0e-8
    any_inside = face_inside.any(dim=1)
    all_inside = face_inside.all(dim=1)
    crossing = front_depth & any_inside & ~all_inside
    clip_mask = crossing & (kept_mask | ~severe_stretch)
    unchanged_mask = kept_mask & (~front_depth | ~any_inside)

    output_faces: list[list[int]] = source_faces[unchanged_mask].tolist()
    neutral_output = [point.clone() for point in neutral_vertices]
    deformed_output = [point.clone() for point in deformed_vertices]
    edge_intersections: dict[tuple[int, int], int] = {}
    clipped_face_count = int(clip_mask.sum().item())
    restored_boundary_face_count = int((clip_mask & ~kept_mask).sum().item())
    removed_inside_face_count = int(
        (kept_mask & front_depth & all_inside).sum().item()
    )
    retained_outside_face_count = int(
        (kept_mask & front_depth & ~any_inside).sum().item()
    )

    def intersection_vertex(start: int, end: int) -> int:
        key = (min(start, end), max(start, end))
        cached = edge_intersections.get(key)
        if cached is not None:
            return cached
        start_value = float(contour[start].item())
        end_value = float(contour[end].item())
        denominator = start_value - end_value
        if abs(denominator) <= 1.0e-12:
            fraction = 0.5
        else:
            fraction = max(0.0, min(1.0, start_value / denominator))
        neutral_point = neutral_vertices[start] + fraction * (
            neutral_vertices[end] - neutral_vertices[start]
        )
        deformed_point = deformed_vertices[start] + fraction * (
            deformed_vertices[end] - deformed_vertices[start]
        )
        vertex_id = len(neutral_output)
        neutral_output.append(neutral_point)
        deformed_output.append(deformed_point)
        edge_intersections[key] = vertex_id
        return vertex_id

    for face_index in torch.nonzero(clip_mask, as_tuple=False).flatten().tolist():
        face_tensor = source_faces[face_index]
        face = [int(value) for value in face_tensor.tolist()]
        values = [float(contour[vertex_id].item()) for vertex_id in face]
        inside = [value > 1.0e-8 for value in values]

        clipped_polygon: list[int] = []
        for corner in range(3):
            start = face[corner]
            end = face[(corner + 1) % 3]
            start_inside = inside[corner]
            end_inside = inside[(corner + 1) % 3]
            if not start_inside:
                clipped_polygon.append(start)
            if start_inside != end_inside:
                clipped_polygon.append(intersection_vertex(start, end))
        clipped_polygon = list(dict.fromkeys(clipped_polygon))
        if len(clipped_polygon) < 3:
            continue
        for corner in range(1, len(clipped_polygon) - 1):
            triangle = [
                clipped_polygon[0],
                clipped_polygon[corner],
                clipped_polygon[corner + 1],
            ]
            if len(set(triangle)) == 3:
                output_faces.append(triangle)
    final_neutral = torch.stack(neutral_output, dim=0).contiguous()
    final_deformed = torch.stack(deformed_output, dim=0).contiguous()
    final_faces = torch.tensor(output_faces, dtype=torch.long).contiguous()
    report = MouthContourClipReport(
        source_face_count=int(source_faces.shape[0]),
        input_kept_face_count=int(kept_faces.shape[0]),
        final_face_count=int(final_faces.shape[0]),
        original_vertex_count=vertex_count,
        final_vertex_count=int(final_neutral.shape[0]),
        clipped_face_count=int(clipped_face_count),
        restored_boundary_face_count=int(restored_boundary_face_count),
        removed_inside_face_count=int(removed_inside_face_count),
        retained_outside_face_count=int(retained_outside_face_count),
        added_vertex_count=int(final_neutral.shape[0] - vertex_count),
        smooth_curve_sample_count=int(curve_samples),
        contour_scale=float(contour_scale),
        depth_padding=float(depth_padding),
    )
    return final_neutral, final_deformed, final_faces, report




def _smooth_mouth_contour_field(
    horizontal: torch.Tensor,
    vertical: torch.Tensor,
    upper_points: torch.Tensor,
    lower_points: torch.Tensor,
    *,
    contour_scale: float,
    curve_samples: int,
) -> torch.Tensor:
    from scipy.interpolate import PchipInterpolator

    horizontal_np = horizontal.detach().cpu().double().numpy()
    vertical_np = vertical.detach().cpu().double().numpy()
    upper_np = upper_points.detach().cpu().double().numpy()
    lower_np = lower_points.detach().cpu().double().numpy()
    contour = _smooth_mouth_contour_samples_numpy(
        upper_np,
        lower_np,
        contour_scale=float(contour_scale),
        curve_samples=int(curve_samples),
    )
    half = contour.shape[0] // 2
    sample_x = contour[:half, 0]
    top_curve = contour[:half, 1]
    bottom_curve = contour[half:, 1][::-1]
    horizontal_min = float(sample_x.min())
    horizontal_max = float(sample_x.max())
    top = np.interp(horizontal_np, sample_x, top_curve)
    bottom = np.interp(horizontal_np, sample_x, bottom_curve)
    field = np.minimum.reduce(
        (
            horizontal_np - horizontal_min,
            horizontal_max - horizontal_np,
            vertical_np - bottom,
            top - vertical_np,
        )
    )
    return torch.from_numpy(field.astype(np.float32, copy=False))


def _smooth_mouth_contour_samples_numpy(
    upper_points: np.ndarray,
    lower_points: np.ndarray,
    *,
    contour_scale: float,
    curve_samples: int,
) -> np.ndarray:
    from scipy.interpolate import PchipInterpolator

    all_points = np.concatenate((upper_points, lower_points), axis=0)
    center = 0.5 * (all_points.min(axis=0) + all_points.max(axis=0))
    upper_points = center + (upper_points - center) * float(contour_scale)
    lower_points = center + (lower_points - center) * float(contour_scale)
    upper_x, upper_z = _merge_curve_samples_numpy(upper_points)
    lower_x, lower_z = _merge_curve_samples_numpy(lower_points)
    horizontal_min = max(float(upper_x.min()), float(lower_x.min()))
    horizontal_max = min(float(upper_x.max()), float(lower_x.max()))
    sample_x = np.linspace(horizontal_min, horizontal_max, int(curve_samples))
    upper_curve = PchipInterpolator(upper_x, upper_z, extrapolate=False)(sample_x)
    lower_curve = PchipInterpolator(lower_x, lower_z, extrapolate=False)(sample_x)
    top_curve = np.maximum(upper_curve, lower_curve)
    bottom_curve = np.minimum(upper_curve, lower_curve)
    upper_contour = np.stack((sample_x, top_curve), axis=1)
    lower_contour = np.stack((sample_x[::-1], bottom_curve[::-1]), axis=1)
    return np.concatenate((upper_contour, lower_contour), axis=0)


def _merge_curve_samples_numpy(points: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    order = np.argsort(points[:, 0], kind="stable")
    points = points[order]
    horizontal, inverse = np.unique(points[:, 0], return_inverse=True)
    vertical = np.zeros_like(horizontal)
    counts = np.zeros_like(horizontal)
    np.add.at(vertical, inverse, points[:, 1])
    np.add.at(counts, inverse, 1.0)
    vertical /= np.maximum(counts, 1.0)
    if horizontal.shape[0] < 3:
        raise ValueError("A smooth lip curve requires at least three distinct x positions.")
    return horizontal, vertical


def weld_coincident_vertices(
    mesh: Mapping[str, torch.Tensor],
) -> tuple[dict[str, torch.Tensor], CoincidentVertexWeldReport, torch.Tensor]:
    """Merge exactly coincident vertices while preserving first-seen ordering."""

    vertices = mesh["vertices"].detach().cpu().float().contiguous()
    faces = mesh["faces"].detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[-1] != 3:
        raise ValueError("weld_coincident_vertices expects triangular faces [F, 3].")

    vertex_array = vertices.numpy()
    _unique_sorted, first_indices, inverse_sorted = np.unique(
        vertex_array,
        axis=0,
        return_index=True,
        return_inverse=True,
    )
    stable_order = np.argsort(first_indices)
    sorted_to_stable = np.empty_like(stable_order)
    sorted_to_stable[stable_order] = np.arange(stable_order.shape[0])
    stable_first_indices = first_indices[stable_order]
    original_to_welded = torch.from_numpy(
        sorted_to_stable[inverse_sorted].astype(np.int64, copy=False)
    ).long()
    welded_vertices = vertices.index_select(
        0,
        torch.from_numpy(stable_first_indices.astype(np.int64, copy=False)).long(),
    ).contiguous()

    welded_faces = original_to_welded.index_select(0, faces.reshape(-1)).reshape_as(faces)
    nondegenerate = (
        (welded_faces[:, 0] != welded_faces[:, 1])
        & (welded_faces[:, 1] != welded_faces[:, 2])
        & (welded_faces[:, 2] != welded_faces[:, 0])
    )
    welded_faces = welded_faces[nondegenerate].contiguous()

    original_normals = mesh.get("normals")
    if not isinstance(original_normals, torch.Tensor) or original_normals.shape != vertices.shape:
        original_normals = compute_vertex_normals(vertices, faces)
    else:
        original_normals = original_normals.detach().cpu().float().contiguous()
    welded_normals = torch.zeros_like(welded_vertices)
    welded_normals.index_add_(0, original_to_welded, original_normals)
    welded_normals = F.normalize(welded_normals, dim=-1, eps=1.0e-6).contiguous()

    welded_mesh = dict(mesh)
    welded_mesh["vertices"] = welded_vertices
    welded_mesh["faces"] = welded_faces
    welded_mesh["normals"] = welded_normals
    report = CoincidentVertexWeldReport(
        original_vertex_count=int(vertices.shape[0]),
        final_vertex_count=int(welded_vertices.shape[0]),
        merged_vertex_count=int(vertices.shape[0] - welded_vertices.shape[0]),
        original_face_count=int(faces.shape[0]),
        final_face_count=int(welded_faces.shape[0]),
        removed_degenerate_face_count=int((~nondegenerate).sum().item()),
    )
    return welded_mesh, report, original_to_welded.contiguous()


def remap_vertex_mapping(
    mapping: Mapping[int, int],
    original_to_updated: torch.Tensor,
) -> dict[int, int]:
    original_to_updated = original_to_updated.detach().cpu().long().flatten()
    remapped: dict[int, int] = {}
    for raw_landmark_id, raw_vertex_id in mapping.items():
        vertex_id = int(raw_vertex_id)
        if 0 <= vertex_id < original_to_updated.numel():
            remapped[int(raw_landmark_id)] = int(original_to_updated[vertex_id].item())
    return remapped


def snap_mediapipe_lip_landmarks_to_face_surface(
    mesh: Mapping[str, torch.Tensor],
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    front_axis: int = 1,
    vertical_axis: int = 2,
    front_direction: int = -1,
    maximum_projected_distance_ratio: float = 0.20,
    local_candidate_radius_ratio: float = 0.04,
    minimum_upper_lower_gap_ratio: float = 0.012,
    front_depth_weight: float = 0.18,
) -> tuple[dict[int, int], LipLandmarkSurfaceSnapReport]:
    """Snap lip landmarks to one coherent, front-facing facial surface.

    Multi-part character FBXs can produce a plausible 2D MediaPipe detection
    whose selected mesh vertices lie on teeth, tongue, hair, or a rear surface.
    The 2D lip locations are still useful.  This routine chooses the facial
    component by landmark votes, then assigns every lip landmark to a unique
    vertex near its projected location on the front of that component.

    Upper/lower curve pairs receive a small minimum vertical separation before
    assignment.  Consequently, touching lips do not collapse landmarks such as
    MediaPipe 13 and 14 onto one vertex when the mesh already has separate lip
    rows.
    """

    from scipy import sparse
    from scipy.optimize import linear_sum_assignment
    from scipy.sparse.csgraph import connected_components

    vertices = mesh["vertices"].detach().cpu().float().contiguous()
    faces = mesh["faces"].detach().cpu().long().contiguous()
    axes = {int(horizontal_axis), int(front_axis), int(vertical_axis)}
    if axes != {0, 1, 2}:
        raise ValueError("Lip landmark snap axes must be a permutation of [0, 1, 2].")
    if int(front_direction) not in {-1, 1}:
        raise ValueError("front_direction must be either -1 or 1.")
    if maximum_projected_distance_ratio <= 0.0:
        raise ValueError("maximum_projected_distance_ratio must be positive.")
    if local_candidate_radius_ratio < 0.0:
        raise ValueError("local_candidate_radius_ratio must be non-negative.")
    if minimum_upper_lower_gap_ratio < 0.0:
        raise ValueError("minimum_upper_lower_gap_ratio must be non-negative.")
    if front_depth_weight < 0.0:
        raise ValueError("front_depth_weight must be non-negative.")
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Lip landmark surface snap expects triangular faces.")
    if faces.numel() and (
        int(faces.min().item()) < 0
        or int(faces.max().item()) >= vertices.shape[0]
    ):
        raise ValueError("Lip landmark surface snap faces contain invalid vertex IDs.")

    repaired = {int(key): int(value) for key, value in mapping.items()}
    original_mapping = dict(repaired)
    vertex_count = int(vertices.shape[0])
    lip_curves = (
        INNER_UPPER_LIP_MEDIAPIPE_IDS,
        INNER_LOWER_LIP_MEDIAPIPE_IDS,
        OUTER_UPPER_LIP_MEDIAPIPE_IDS,
        OUTER_LOWER_LIP_MEDIAPIPE_IDS,
    )
    lip_landmark_ids = tuple(
        dict.fromkeys(value for curve in lip_curves for value in curve)
    )
    present_ids = [
        landmark_id
        for landmark_id in lip_landmark_ids
        if 0 <= repaired.get(landmark_id, -1) < vertex_count
    ]

    def empty_report(*, component_count: int = 0) -> LipLandmarkSurfaceSnapReport:
        return LipLandmarkSurfaceSnapReport(
            input_landmark_count=len(repaired),
            lip_landmark_count=len(lip_landmark_ids),
            connected_component_count=int(component_count),
            facial_component_id=-1,
            facial_component_vertex_count=0,
            facial_component_landmark_votes=0,
            off_component_lip_landmark_count=0,
            duplicate_lip_landmark_count_before=0,
            duplicate_lip_landmark_count_after=0,
            snapped_lip_landmark_count=0,
            skipped_lip_landmark_count=len(lip_landmark_ids),
            maximum_projected_distance_ratio=0.0,
            mean_projected_distance_ratio=0.0,
            minimum_upper_lower_gap=0.0,
            front_axis=int(front_axis),
            front_direction=int(front_direction),
        )

    if not present_ids or faces.numel() == 0:
        return repaired, empty_report(component_count=vertex_count)

    face_array = faces.numpy()
    graph_rows = np.concatenate(
        (face_array[:, 0], face_array[:, 1], face_array[:, 2])
    )
    graph_columns = np.concatenate(
        (face_array[:, 1], face_array[:, 2], face_array[:, 0])
    )
    graph = sparse.coo_matrix(
        (
            np.ones(graph_rows.shape[0], dtype=np.uint8),
            (graph_rows, graph_columns),
        ),
        shape=(vertex_count, vertex_count),
    ).tocsr()
    component_count, component_labels = connected_components(
        graph,
        directed=False,
    )
    component_sizes = np.bincount(component_labels, minlength=component_count)

    # Face Mesh IDs below the iris extension (468+) provide a much stronger
    # facial-surface vote than mouth landmarks alone when several lips happen
    # to land on the same tongue or tooth component.
    component_votes = Counter(
        int(component_labels[vertex_id])
        for landmark_id, vertex_id in repaired.items()
        if 0 <= int(landmark_id) < 468 and 0 <= int(vertex_id) < vertex_count
    )
    if not component_votes:
        return repaired, empty_report(component_count=component_count)
    facial_component = max(
        component_votes,
        key=lambda component: (
            int(component_votes[component]),
            int(component_sizes[component]),
            -int(component),
        ),
    )
    facial_vertex_ids = np.flatnonzero(
        component_labels == facial_component
    ).astype(np.int64, copy=False)
    if facial_vertex_ids.size == 0:
        return repaired, empty_report(component_count=component_count)

    source_vertex_ids = torch.tensor(
        [repaired[landmark_id] for landmark_id in present_ids],
        dtype=torch.long,
    )
    targets = vertices.index_select(0, source_vertex_ids).clone()
    target_index = {
        landmark_id: index for index, landmark_id in enumerate(present_ids)
    }
    projection_axes = (int(horizontal_axis), int(vertical_axis))

    if all(value in repaired for value in (61, 291)):
        corner_ids = torch.tensor([repaired[61], repaired[291]], dtype=torch.long)
        if bool(((corner_ids >= 0) & (corner_ids < vertex_count)).all()):
            corner_points = vertices.index_select(0, corner_ids)[:, projection_axes]
            mouth_width = float(
                torch.linalg.vector_norm(corner_points[1] - corner_points[0]).item()
            )
        else:
            mouth_width = 0.0
    else:
        mouth_width = 0.0
    projected_targets = targets[:, projection_axes]
    projected_span = projected_targets.amax(dim=0) - projected_targets.amin(dim=0)
    mouth_width = max(mouth_width, float(projected_span.max().item()), 1.0e-8)
    minimum_gap = float(minimum_upper_lower_gap_ratio) * mouth_width

    # MediaPipe defines corresponding upper and lower curves in the same
    # left-to-right order.  Preserve their observed center while separating
    # coincident or inverted samples just enough to select different lip rows.
    for upper_curve, lower_curve in (
        (INNER_UPPER_LIP_MEDIAPIPE_IDS, INNER_LOWER_LIP_MEDIAPIPE_IDS),
        (OUTER_UPPER_LIP_MEDIAPIPE_IDS, OUTER_LOWER_LIP_MEDIAPIPE_IDS),
    ):
        for upper_id, lower_id in zip(upper_curve, lower_curve):
            if (
                upper_id == lower_id
                or upper_id not in target_index
                or lower_id not in target_index
            ):
                continue
            upper_index = target_index[upper_id]
            lower_index = target_index[lower_id]
            upper_height = targets[upper_index, int(vertical_axis)]
            lower_height = targets[lower_index, int(vertical_axis)]
            if float(upper_height - lower_height) >= minimum_gap:
                continue
            center_height = 0.5 * (upper_height + lower_height)
            targets[upper_index, int(vertical_axis)] = center_height + 0.5 * minimum_gap
            targets[lower_index, int(vertical_axis)] = center_height - 0.5 * minimum_gap

    facial_ids_tensor = torch.from_numpy(facial_vertex_ids).long()
    facial_vertices = vertices.index_select(0, facial_ids_tensor)
    projected_targets = targets[:, projection_axes]
    projected_candidates = facial_vertices[:, projection_axes]
    projected_distances = torch.cdist(projected_targets, projected_candidates)
    nearest_distances = projected_distances.amin(dim=1)
    maximum_distance = float(maximum_projected_distance_ratio) * mouth_width
    candidate_slack = float(local_candidate_radius_ratio) * mouth_width
    surface_depth_tolerance = 0.12 * mouth_width
    directed_depth = (
        facial_vertices[:, int(front_axis)] * float(front_direction)
    )
    facial_face_mask = component_labels[face_array[:, 0]] == int(facial_component)
    facial_faces = faces[torch.from_numpy(facial_face_mask)]
    facial_triangles = vertices.index_select(
        0,
        facial_faces.reshape(-1),
    ).reshape(-1, 3, 3)
    projected_triangles = facial_triangles[:, :, projection_axes]
    triangle_first = projected_triangles[:, 0]
    triangle_first_edge = projected_triangles[:, 1] - triangle_first
    triangle_second_edge = projected_triangles[:, 2] - triangle_first
    triangle_denominator = (
        triangle_first_edge[:, 0] * triangle_second_edge[:, 1]
        - triangle_second_edge[:, 0] * triangle_first_edge[:, 1]
    )
    valid_projected_triangle = triangle_denominator.abs() > 1.0e-12
    safe_triangle_denominator = torch.where(
        valid_projected_triangle,
        triangle_denominator,
        torch.ones_like(triangle_denominator),
    )
    facial_local_by_global = np.full(vertex_count, -1, dtype=np.int64)
    facial_local_by_global[facial_vertex_ids] = np.arange(
        facial_vertex_ids.size,
        dtype=np.int64,
    )
    unavailable_cost = 1.0e6 * max(mouth_width, 1.0)
    costs = torch.full_like(projected_distances, unavailable_cost)
    assignable_rows: list[int] = []
    for row_index in range(len(present_ids)):
        nearest_distance = float(nearest_distances[row_index].item())
        if nearest_distance > maximum_distance:
            continue
        point_offset = projected_targets[row_index] - triangle_first
        first_weight = (
            point_offset[:, 0] * triangle_second_edge[:, 1]
            - triangle_second_edge[:, 0] * point_offset[:, 1]
        ) / safe_triangle_denominator
        second_weight = (
            triangle_first_edge[:, 0] * point_offset[:, 1]
            - point_offset[:, 0] * triangle_first_edge[:, 1]
        ) / safe_triangle_denominator
        zeroth_weight = 1.0 - first_weight - second_weight
        ray_hits = (
            valid_projected_triangle
            & (zeroth_weight >= -1.0e-5)
            & (first_weight >= -1.0e-5)
            & (second_weight >= -1.0e-5)
        )
        hit_face_ids = torch.where(ray_hits)[0]
        hit_local_ids: torch.Tensor | None = None
        hit_directed_depth: torch.Tensor | None = None
        if hit_face_ids.numel() > 0:
            hit_depths = (
                zeroth_weight[hit_face_ids]
                * facial_triangles[hit_face_ids, 0, int(front_axis)]
                + first_weight[hit_face_ids]
                * facial_triangles[hit_face_ids, 1, int(front_axis)]
                + second_weight[hit_face_ids]
                * facial_triangles[hit_face_ids, 2, int(front_axis)]
            ) * float(front_direction)
            front_hit_id = hit_face_ids[int(torch.argmax(hit_depths).item())]
            hit_global_ids = facial_faces[front_hit_id]
            hit_local_ids = torch.from_numpy(
                facial_local_by_global[hit_global_ids.numpy()]
            ).long()
            hit_directed_depth = hit_depths.max()

        if hit_local_ids is not None and hit_directed_depth is not None:
            hit_distance = projected_distances[
                row_index,
                hit_local_ids,
            ].amin()
            allowed_distance = min(
                float(hit_distance.item()) + candidate_slack,
                maximum_distance,
            )
            depth_difference = (directed_depth - hit_directed_depth).abs()
            allowed = (
                (projected_distances[row_index] <= allowed_distance)
                & (depth_difference <= surface_depth_tolerance)
            )
            allowed[hit_local_ids] = True
            depth_penalty = depth_difference
            broad_allowed = (
                (projected_distances[row_index] <= maximum_distance)
                & (depth_difference <= 0.30 * mouth_width)
            )
        else:
            allowed_distance = min(
                nearest_distance + candidate_slack,
                maximum_distance,
            )
            broad_allowed = projected_distances[row_index] <= maximum_distance
            if not bool(broad_allowed.any()):
                continue
            frontmost_depth = directed_depth[broad_allowed].amax()
            depth_penalty = (frontmost_depth - directed_depth).clamp_min(0.0)
            allowed = projected_distances[row_index] <= allowed_distance

        row_cost = projected_distances[row_index] + float(
            front_depth_weight
        ) * depth_penalty
        # Dense or low-resolution lip meshes can leave several landmark rows
        # competing for the same tiny set of ideal vertices.  Keep a wider,
        # depth-constrained fallback set at a deliberately higher cost so the
        # global assignment remains unique without preferring a rear surface.
        costs[row_index, broad_allowed] = (
            row_cost[broad_allowed] + maximum_distance
        )
        costs[row_index, allowed] = row_cost[allowed]
        assignable_rows.append(row_index)

    assigned_rows: set[int] = set()
    if assignable_rows:
        row_ids, column_ids = linear_sum_assignment(
            costs[assignable_rows].numpy()
        )
        for local_row, column_index in zip(row_ids.tolist(), column_ids.tolist()):
            row_index = assignable_rows[int(local_row)]
            if float(costs[row_index, column_index].item()) >= unavailable_cost:
                continue
            landmark_id = present_ids[row_index]
            repaired[landmark_id] = int(facial_vertex_ids[column_index])
            assigned_rows.add(row_index)

    # A lip sample can fall just inside the aperture and ray-hit a rear facial
    # wall.  Repair isolated depth jumps from its two curve neighbors while
    # retaining the unique-vertex assignment.
    for _pass in range(2):
        used_vertex_ids = {int(repaired[value]) for value in present_ids}
        changed_outlier = False
        for curve in lip_curves:
            for curve_index in range(1, len(curve) - 1):
                landmark_id = curve[curve_index]
                previous_id = curve[curve_index - 1]
                next_id = curve[curve_index + 1]
                if not all(
                    value in target_index
                    and target_index[value] in assigned_rows
                    for value in (previous_id, landmark_id, next_id)
                ):
                    continue
                current_vertex_id = int(repaired[landmark_id])
                previous_depth = (
                    vertices[int(repaired[previous_id]), int(front_axis)]
                    * float(front_direction)
                )
                next_depth = (
                    vertices[int(repaired[next_id]), int(front_axis)]
                    * float(front_direction)
                )
                if float((previous_depth - next_depth).abs()) > 0.30 * mouth_width:
                    continue
                expected_depth = 0.5 * (previous_depth + next_depth)
                current_depth = (
                    vertices[current_vertex_id, int(front_axis)]
                    * float(front_direction)
                )
                if float((current_depth - expected_depth).abs()) <= 0.30 * mouth_width:
                    continue

                row_index = target_index[landmark_id]
                depth_difference = (directed_depth - expected_depth).abs()
                allowed = (
                    (projected_distances[row_index] <= maximum_distance)
                    & (depth_difference <= 0.15 * mouth_width)
                )
                for used_vertex_id in used_vertex_ids - {current_vertex_id}:
                    local_id = int(facial_local_by_global[used_vertex_id])
                    if local_id >= 0:
                        allowed[local_id] = False
                candidate_local_ids = torch.where(allowed)[0]
                if candidate_local_ids.numel() == 0:
                    continue
                candidate_cost = (
                    projected_distances[row_index, candidate_local_ids]
                    + float(front_depth_weight)
                    * depth_difference[candidate_local_ids]
                )
                replacement_local_id = int(
                    candidate_local_ids[int(torch.argmin(candidate_cost).item())].item()
                )
                replacement_vertex_id = int(facial_vertex_ids[replacement_local_id])
                if replacement_vertex_id == current_vertex_id:
                    continue
                repaired[landmark_id] = replacement_vertex_id
                used_vertex_ids.discard(current_vertex_id)
                used_vertex_ids.add(replacement_vertex_id)
                changed_outlier = True
        if not changed_outlier:
            break

    original_vertex_ids = [int(original_mapping[value]) for value in present_ids]
    final_vertex_ids = [int(repaired[value]) for value in present_ids]
    projected_ratios = [
        float(
            torch.linalg.vector_norm(
                vertices[repaired[landmark_id], projection_axes]
                - targets[target_index[landmark_id], projection_axes]
            ).item()
        )
        / mouth_width
        for landmark_id in present_ids
        if target_index[landmark_id] in assigned_rows
    ]
    original_duplicates = len(original_vertex_ids) - len(set(original_vertex_ids))
    final_duplicates = len(final_vertex_ids) - len(set(final_vertex_ids))
    off_component_count = sum(
        int(component_labels[int(original_mapping[value])]) != int(facial_component)
        for value in present_ids
    )
    report = LipLandmarkSurfaceSnapReport(
        input_landmark_count=len(repaired),
        lip_landmark_count=len(lip_landmark_ids),
        connected_component_count=int(component_count),
        facial_component_id=int(facial_component),
        facial_component_vertex_count=int(facial_vertex_ids.size),
        facial_component_landmark_votes=int(component_votes[facial_component]),
        off_component_lip_landmark_count=int(off_component_count),
        duplicate_lip_landmark_count_before=int(original_duplicates),
        duplicate_lip_landmark_count_after=int(final_duplicates),
        snapped_lip_landmark_count=sum(
            original != final
            for original, final in zip(original_vertex_ids, final_vertex_ids)
        ),
        skipped_lip_landmark_count=(
            len(lip_landmark_ids) - len(present_ids)
            + len(present_ids) - len(assigned_rows)
        ),
        maximum_projected_distance_ratio=max(projected_ratios, default=0.0),
        mean_projected_distance_ratio=(
            float(np.mean(projected_ratios)) if projected_ratios else 0.0
        ),
        minimum_upper_lower_gap=float(minimum_gap),
        front_axis=int(front_axis),
        front_direction=int(front_direction),
    )
    return repaired, report


def repair_mediapipe_lip_landmarks_to_dominant_component(
    mesh: Mapping[str, torch.Tensor],
    mapping: Mapping[int, int],
    *,
    maximum_snap_distance_ratio: float = 0.25,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
) -> tuple[dict[int, int], LipLandmarkTopologyRepairReport]:
    """Move invalid or detached lip landmarks onto one coherent face surface."""

    from scipy import sparse
    from scipy.sparse.csgraph import connected_components
    from scipy.spatial import cKDTree

    vertices = mesh["vertices"].detach().cpu().float().contiguous()
    faces = mesh["faces"].detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("Lip landmark topology repair expects triangular faces.")
    if maximum_snap_distance_ratio <= 0.0:
        raise ValueError("Maximum lip-landmark snap distance ratio must be positive.")
    if {
        int(horizontal_axis),
        int(depth_axis),
        int(vertical_axis),
    } != {0, 1, 2}:
        raise ValueError("Lip landmark repair axes must be a permutation of [0, 2].")

    input_landmark_count = len(mapping)
    repaired = {int(key): int(value) for key, value in mapping.items()}
    vertex_count = int(vertices.shape[0])
    lip_curves = (
        INNER_UPPER_LIP_MEDIAPIPE_IDS,
        INNER_LOWER_LIP_MEDIAPIPE_IDS,
        OUTER_UPPER_LIP_MEDIAPIPE_IDS,
        OUTER_LOWER_LIP_MEDIAPIPE_IDS,
    )
    lip_landmark_ids = tuple(
        dict.fromkeys(value for curve in lip_curves for value in curve)
    )
    valid_ids = [
        landmark_id
        for landmark_id in lip_landmark_ids
        if 0 <= repaired.get(landmark_id, -1) < vertex_count
    ]
    invalid_ids = set(lip_landmark_ids) - set(valid_ids)
    if not valid_ids or faces.numel() == 0:
        report = LipLandmarkTopologyRepairReport(
            input_landmark_count=input_landmark_count,
            lip_landmark_count=len(lip_landmark_ids),
            valid_lip_landmark_count=len(valid_ids),
            invalid_lip_landmark_count=len(invalid_ids),
            connected_component_count=vertex_count,
            dominant_component_vertex_count=0,
            off_component_lip_landmark_count=0,
            spatial_outlier_lip_landmark_count=0,
            snapped_lip_landmark_count=0,
            interpolated_lip_landmark_count=0,
            skipped_lip_landmark_count=len(invalid_ids),
            maximum_snap_distance=0.0,
            mean_snap_distance=0.0,
        )
        return repaired, report

    face_array = faces.numpy()
    graph_rows = np.concatenate(
        (
            face_array[:, 0], face_array[:, 1], face_array[:, 2],
            face_array[:, 1], face_array[:, 2], face_array[:, 0],
        )
    )
    graph_columns = np.concatenate(
        (
            face_array[:, 1], face_array[:, 2], face_array[:, 0],
            face_array[:, 0], face_array[:, 1], face_array[:, 2],
        )
    )
    graph = sparse.coo_matrix(
        (
            np.ones(graph_rows.shape[0], dtype=np.uint8),
            (graph_rows, graph_columns),
        ),
        shape=(vertex_count, vertex_count),
    ).tocsr()
    component_count, component_labels = connected_components(
        graph,
        directed=False,
    )
    component_sizes = np.bincount(component_labels, minlength=component_count)
    landmark_component_counts = np.bincount(
        np.asarray(
            [component_labels[repaired[landmark_id]] for landmark_id in valid_ids],
            dtype=np.int64,
        ),
        minlength=component_count,
    )
    candidate_components = np.flatnonzero(landmark_component_counts > 0)
    dominant_component = max(
        candidate_components.tolist(),
        key=lambda component: (
            int(landmark_component_counts[component]),
            int(component_sizes[component]),
        ),
    )
    dominant_vertex_ids = np.flatnonzero(
        component_labels == dominant_component
    ).astype(np.int64, copy=False)
    dominant_vertices = vertices.numpy()[dominant_vertex_ids]
    dominant_tree = cKDTree(dominant_vertices)

    valid_points = vertices[
        torch.tensor([repaired[value] for value in valid_ids], dtype=torch.long)
    ]
    projected_span = (
        valid_points[:, (int(horizontal_axis), int(vertical_axis))].amax(dim=0)
        - valid_points[:, (int(horizontal_axis), int(vertical_axis))].amin(dim=0)
    )
    mouth_scale = max(float(projected_span.max().item()), 1.0e-6)
    maximum_snap_distance = float(maximum_snap_distance_ratio) * mouth_scale
    snap_distances: list[float] = []
    snapped_ids: set[int] = set()
    interpolated_ids: set[int] = set()
    off_component_ids = {
        landmark_id
        for landmark_id in valid_ids
        if component_labels[repaired[landmark_id]] != dominant_component
    }

    def snap_landmark(landmark_id: int, target: np.ndarray) -> bool:
        distance, local_index = dominant_tree.query(target, k=1)
        if not np.isfinite(distance) or float(distance) > maximum_snap_distance:
            return False
        repaired[int(landmark_id)] = int(dominant_vertex_ids[int(local_index)])
        snapped_ids.add(int(landmark_id))
        snap_distances.append(float(distance))
        return True

    for landmark_id in sorted(off_component_ids):
        source_vertex_id = repaired[landmark_id]
        snap_landmark(landmark_id, vertices[source_vertex_id].numpy())

    resolved_ids = [
        landmark_id
        for landmark_id in lip_landmark_ids
        if (
            0 <= repaired.get(landmark_id, -1) < vertex_count
            and component_labels[repaired[landmark_id]] == dominant_component
        )
    ]
    resolved_points = vertices[
        torch.tensor([repaired[value] for value in resolved_ids], dtype=torch.long)
    ]
    resolved_depths = resolved_points[:, int(depth_axis)].double().numpy()
    median_depth = float(np.median(resolved_depths))
    median_depth_deviation = float(
        np.median(np.abs(resolved_depths - median_depth))
    )
    depth_outlier_threshold = max(
        0.12 * mouth_scale,
        8.0 * median_depth_deviation,
        1.0e-5,
    )
    spatial_outlier_ids = {
        landmark_id
        for landmark_id in resolved_ids
        if abs(
            float(vertices[repaired[landmark_id], int(depth_axis)].item())
            - median_depth
        )
        > depth_outlier_threshold
    }
    for curve in lip_curves:
        for index, landmark_id in enumerate(curve):
            if landmark_id not in spatial_outlier_ids:
                continue
            neighbor_ids = []
            for neighbor_index in (index - 1, index + 1):
                if not 0 <= neighbor_index < len(curve):
                    continue
                neighbor_id = curve[neighbor_index]
                if (
                    neighbor_id not in spatial_outlier_ids
                    and 0 <= repaired.get(neighbor_id, -1) < vertex_count
                    and component_labels[repaired[neighbor_id]]
                    == dominant_component
                ):
                    neighbor_ids.append(repaired[neighbor_id])
            if not neighbor_ids:
                continue
            target = vertices[
                torch.tensor(neighbor_ids, dtype=torch.long)
            ].mean(dim=0).numpy()
            snap_landmark(landmark_id, target)

    unresolved_ids = {
        landmark_id
        for landmark_id in lip_landmark_ids
        if not (
            0 <= repaired.get(landmark_id, -1) < vertex_count
            and component_labels[repaired[landmark_id]] == dominant_component
        )
    }
    for curve in lip_curves:
        known_indices = [
            index
            for index, landmark_id in enumerate(curve)
            if landmark_id not in unresolved_ids
        ]
        if not known_indices:
            continue
        known_points = vertices[
            torch.tensor(
                [repaired[curve[index]] for index in known_indices],
                dtype=torch.long,
            )
        ].numpy()
        for index, landmark_id in enumerate(curve):
            if landmark_id not in unresolved_ids:
                continue
            insertion = int(np.searchsorted(known_indices, index))
            if insertion <= 0:
                target = known_points[0]
            elif insertion >= len(known_indices):
                target = known_points[-1]
            else:
                first_index = known_indices[insertion - 1]
                second_index = known_indices[insertion]
                fraction = float(index - first_index) / float(
                    second_index - first_index
                )
                target = known_points[insertion - 1] + fraction * (
                    known_points[insertion] - known_points[insertion - 1]
                )
            if snap_landmark(landmark_id, target):
                interpolated_ids.add(int(landmark_id))
                unresolved_ids.discard(landmark_id)

    # Closed-mouth upper and lower curves are close enough that one can seed
    # the other when an FBX sidecar refers entirely to a different mesh object.
    for first_curve, second_curve in (
        (INNER_UPPER_LIP_MEDIAPIPE_IDS, INNER_LOWER_LIP_MEDIAPIPE_IDS),
        (OUTER_UPPER_LIP_MEDIAPIPE_IDS, OUTER_LOWER_LIP_MEDIAPIPE_IDS),
    ):
        for source_curve, target_curve in (
            (first_curve, second_curve),
            (second_curve, first_curve),
        ):
            for landmark_id, counterpart_id in zip(source_curve, target_curve):
                if landmark_id not in unresolved_ids:
                    continue
                counterpart_vertex_id = repaired.get(counterpart_id, -1)
                if not (
                    0 <= counterpart_vertex_id < vertex_count
                    and component_labels[counterpart_vertex_id]
                    == dominant_component
                ):
                    continue
                if snap_landmark(
                    landmark_id,
                    vertices[counterpart_vertex_id].numpy(),
                ):
                    interpolated_ids.add(int(landmark_id))
                    unresolved_ids.discard(landmark_id)

    report = LipLandmarkTopologyRepairReport(
        input_landmark_count=input_landmark_count,
        lip_landmark_count=len(lip_landmark_ids),
        valid_lip_landmark_count=len(valid_ids),
        invalid_lip_landmark_count=len(invalid_ids),
        connected_component_count=int(component_count),
        dominant_component_vertex_count=int(component_sizes[dominant_component]),
        off_component_lip_landmark_count=len(off_component_ids),
        spatial_outlier_lip_landmark_count=len(spatial_outlier_ids),
        snapped_lip_landmark_count=len(snapped_ids),
        interpolated_lip_landmark_count=len(interpolated_ids),
        skipped_lip_landmark_count=len(unresolved_ids),
        maximum_snap_distance=max(snap_distances, default=0.0),
        mean_snap_distance=(
            float(np.mean(snap_distances)) if snap_distances else 0.0
        ),
    )
    return repaired, report


def split_lips_along_mediapipe_seam(
    mesh: Mapping[str, torch.Tensor],
    mapping: Mapping[int, int],
    *,
    horizontal_axis: int = 0,
    depth_axis: int = 1,
    vertical_axis: int = 2,
    depth_tolerance: float = 0.02,
    vertical_tolerance: float = 0.012,
    horizontal_padding: float = 0.0,
    corner_inset_ratio: float = 0.0,
    preopen_distance: float = 0.0,
) -> tuple[dict[str, torch.Tensor], GeometricLipSeamReport]:
    """Cut a closed-mouth mesh along its MediaPipe inner-lip centerline.

    Faces are retained.  The function labels local faces above and below a
    piecewise-linear inner-lip curve, finds vertices shared by both sides, and
    duplicates those vertices for lower-side faces.  The resulting coincident
    boundaries look unchanged in neutral pose but are disconnected in the
    graph supplied to TopoRig inference.
    """

    vertices = mesh["vertices"].detach().cpu().float().contiguous()
    faces = mesh["faces"].detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[-1] != 3:
        raise ValueError(
            "split_lips_along_mediapipe_seam expects triangular faces [F, 3]."
        )
    axes = (int(horizontal_axis), int(depth_axis), int(vertical_axis))
    if sorted(axes) != [0, 1, 2]:
        raise ValueError(
            "horizontal_axis, depth_axis, and vertical_axis must be distinct "
            "coordinate indices in [0, 2]."
        )
    if depth_tolerance <= 0.0 or vertical_tolerance <= 0.0:
        raise ValueError("Lip seam depth and vertical tolerances must be positive.")
    if horizontal_padding < 0.0:
        raise ValueError("Lip seam horizontal padding cannot be negative.")
    if not 0.0 <= corner_inset_ratio < 0.5:
        raise ValueError("Lip seam corner inset ratio must be in [0, 0.5).")
    if preopen_distance < 0.0:
        raise ValueError("Lip seam pre-open distance cannot be negative.")

    curve_points = _inner_lip_center_curve(vertices, mapping)
    if curve_points.shape[0] < 2:
        raise ValueError(
            "At least two usable MediaPipe inner-lip center points are required."
        )
    curve_points = curve_points[
        torch.argsort(curve_points[:, horizontal_axis])
    ].contiguous()
    curve_points = _merge_curve_points_with_duplicate_horizontal_values(
        curve_points,
        horizontal_axis=horizontal_axis,
    )
    if curve_points.shape[0] < 2:
        raise ValueError(
            "MediaPipe inner-lip center points do not span the horizontal axis."
        )

    face_centers = vertices.index_select(0, faces.reshape(-1)).reshape(-1, 3, 3).mean(dim=1)
    center_horizontal = face_centers[:, horizontal_axis]
    curve_vertical = _piecewise_linear_interpolate(
        center_horizontal,
        curve_points[:, horizontal_axis],
        curve_points[:, vertical_axis],
    )
    curve_depth = _piecewise_linear_interpolate(
        center_horizontal,
        curve_points[:, horizontal_axis],
        curve_points[:, depth_axis],
    )
    vertical_offset = face_centers[:, vertical_axis] - curve_vertical
    depth_offset = face_centers[:, depth_axis] - curve_depth
    curve_horizontal_min = curve_points[:, horizontal_axis].amin()
    curve_horizontal_max = curve_points[:, horizontal_axis].amax()
    corner_inset = (
        curve_horizontal_max - curve_horizontal_min
    ) * float(corner_inset_ratio)
    horizontal_min = (
        curve_horizontal_min + corner_inset - float(horizontal_padding)
    )
    horizontal_max = (
        curve_horizontal_max - corner_inset + float(horizontal_padding)
    )
    candidate_faces = (
        (center_horizontal >= horizontal_min)
        & (center_horizontal <= horizontal_max)
        & (depth_offset.abs() <= float(depth_tolerance))
        & (vertical_offset.abs() <= float(vertical_tolerance))
    )
    upper_candidate_faces = candidate_faces & (vertical_offset >= 0.0)
    lower_candidate_faces = candidate_faces & (vertical_offset < 0.0)
    if not bool(upper_candidate_faces.any().item()):
        raise ValueError("No upper-side faces were found near the MediaPipe lip seam.")
    if not bool(lower_candidate_faces.any().item()):
        raise ValueError("No lower-side faces were found near the MediaPipe lip seam.")

    vertex_count = int(vertices.shape[0])
    upper_vertices = torch.zeros(vertex_count, dtype=torch.bool)
    lower_vertices = torch.zeros(vertex_count, dtype=torch.bool)
    upper_vertices[faces[upper_candidate_faces].reshape(-1)] = True
    lower_vertices[faces[lower_candidate_faces].reshape(-1)] = True
    seam_vertex_ids = torch.nonzero(
        upper_vertices & lower_vertices,
        as_tuple=False,
    ).flatten().long()
    if seam_vertex_ids.numel() < 2:
        raise ValueError(
            "The MediaPipe lip curve did not produce a usable mesh seam. "
            "Increase the seam depth or vertical tolerance."
        )

    duplicate_ids = torch.arange(
        vertex_count,
        vertex_count + seam_vertex_ids.numel(),
        dtype=torch.long,
    )
    lower_duplicate_vertex_ids = {
        int(original): int(duplicate)
        for original, duplicate in zip(seam_vertex_ids.tolist(), duplicate_ids.tolist())
    }
    updated_vertices = torch.cat(
        (vertices, vertices.index_select(0, seam_vertex_ids)),
        dim=0,
    ).contiguous()
    if preopen_distance > 0.0:
        half_distance = float(preopen_distance) * 0.5
        updated_vertices[seam_vertex_ids, vertical_axis] += half_distance
        updated_vertices[duplicate_ids, vertical_axis] -= half_distance
    lower_remap = torch.arange(updated_vertices.shape[0], dtype=torch.long)
    lower_remap[seam_vertex_ids] = duplicate_ids

    seam_mask = torch.zeros(vertex_count, dtype=torch.bool)
    seam_mask[seam_vertex_ids] = True
    face_has_seam_vertex = seam_mask.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=1)
    remapped_lower_faces = face_has_seam_vertex & (vertical_offset < 0.0)
    updated_faces = faces.clone()
    updated_faces[remapped_lower_faces] = lower_remap.index_select(
        0,
        updated_faces[remapped_lower_faces].reshape(-1),
    ).reshape(-1, 3)

    original_in_face = torch.zeros(updated_vertices.shape[0], dtype=torch.bool)
    duplicate_in_face = torch.zeros_like(original_in_face)
    original_in_face[seam_vertex_ids] = True
    duplicate_in_face[duplicate_ids] = True
    flat_faces = updated_faces.reshape(-1)
    face_has_original = original_in_face.index_select(0, flat_faces).reshape_as(updated_faces).any(dim=1)
    face_has_duplicate = duplicate_in_face.index_select(0, flat_faces).reshape_as(updated_faces).any(dim=1)
    mixed_faces = face_has_original & face_has_duplicate

    updated_mesh = dict(mesh)
    updated_mesh["vertices"] = updated_vertices
    updated_mesh["faces"] = updated_faces.contiguous()
    original_normals = mesh.get("normals")
    if not isinstance(original_normals, torch.Tensor) or original_normals.shape != vertices.shape:
        original_normals = compute_vertex_normals(vertices, faces)
    else:
        original_normals = original_normals.detach().cpu().float().contiguous()
    updated_mesh["normals"] = torch.cat(
        (original_normals, original_normals.index_select(0, seam_vertex_ids)),
        dim=0,
    ).contiguous()
    report = GeometricLipSeamReport(
        original_vertex_count=vertex_count,
        original_face_count=int(faces.shape[0]),
        final_vertex_count=int(updated_vertices.shape[0]),
        final_face_count=int(updated_faces.shape[0]),
        curve_point_count=int(curve_points.shape[0]),
        candidate_face_count=int(candidate_faces.sum().item()),
        upper_candidate_face_count=int(upper_candidate_faces.sum().item()),
        lower_candidate_face_count=int(lower_candidate_faces.sum().item()),
        seam_vertex_count=int(seam_vertex_ids.numel()),
        duplicated_vertex_count=int(seam_vertex_ids.numel()),
        remapped_lower_face_count=int(remapped_lower_faces.sum().item()),
        mixed_original_duplicate_face_count=int(mixed_faces.sum().item()),
        horizontal_axis=horizontal_axis,
        depth_axis=depth_axis,
        vertical_axis=vertical_axis,
        depth_tolerance=float(depth_tolerance),
        vertical_tolerance=float(vertical_tolerance),
        horizontal_padding=float(horizontal_padding),
        corner_inset_ratio=float(corner_inset_ratio),
        preopen_distance=float(preopen_distance),
        curve_points=curve_points.tolist(),
        seam_vertex_ids=[int(vertex_id) for vertex_id in seam_vertex_ids.tolist()],
        lower_duplicate_vertex_ids=lower_duplicate_vertex_ids,
    )
    return updated_mesh, report


def split_lip_connections(
    mesh: Mapping[str, torch.Tensor],
    lip_regions: LipRegions,
    *,
    remove_mixed_faces: bool = True,
    extra_remove_face_ids: torch.Tensor | Sequence[int] | None = None,
) -> tuple[dict[str, torch.Tensor], LipSplitReport, LipRegions]:
    """Remove upper/lower bridge faces and duplicate shared lower-lip vertices.

    Sapiens-style dense face parsing is expected to have already been projected
    to mesh vertices. Vertices that are marked as both upper and lower lip are
    treated as closed-mouth seam vertices: upper-side faces keep the original
    vertex id while lower-side faces receive a duplicate.
    """

    vertices = mesh["vertices"].detach().cpu().float().contiguous()
    faces = mesh["faces"].detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[-1] != 3:
        raise ValueError("split_lip_connections expects triangular faces [F, 3].")

    vertex_count = int(vertices.shape[0])
    upper_ids = normalize_vertex_ids(lip_regions.upper_vertex_ids, vertex_count)
    lower_ids = normalize_vertex_ids(lip_regions.lower_vertex_ids, vertex_count)
    if upper_ids.numel() == 0:
        raise ValueError("Upper lip region is empty after filtering vertex ids.")
    if lower_ids.numel() == 0:
        raise ValueError("Lower lip region is empty after filtering vertex ids.")

    upper_mask = vertex_id_mask(upper_ids, vertex_count)
    lower_mask = vertex_id_mask(lower_ids, vertex_count)
    shared_mask = upper_mask & lower_mask
    exclusive_upper = upper_mask & ~lower_mask
    exclusive_lower = lower_mask & ~upper_mask

    before_faces, before_edges = count_direct_lip_connections(
        faces,
        upper_ids,
        lower_ids,
        vertex_count,
    )

    face_exclusive_upper = (
        exclusive_upper.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=-1)
    )
    face_exclusive_lower = (
        exclusive_lower.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=-1)
    )
    face_shared = shared_mask.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=-1)

    mixed_faces = face_exclusive_upper & face_exclusive_lower
    lower_faces = face_exclusive_lower & ~face_exclusive_upper
    upper_faces = face_exclusive_upper & ~face_exclusive_lower
    neutral_faces = ~(lower_faces | upper_faces | mixed_faces | face_shared)

    keep_face_mask = torch.ones(faces.shape[0], dtype=torch.bool)
    if remove_mixed_faces:
        keep_face_mask &= ~mixed_faces
    if extra_remove_face_ids is not None:
        extra_face_ids = normalize_index_ids(extra_remove_face_ids, int(faces.shape[0]))
        if extra_face_ids.numel() > 0:
            keep_face_mask[extra_face_ids] = False
    kept_faces = faces[keep_face_mask].clone()
    kept_lower_faces = lower_faces[keep_face_mask]

    lower_shared_vertex_ids = _lower_shared_vertex_ids(
        faces=kept_faces,
        lower_face_mask=kept_lower_faces,
        shared_mask=shared_mask,
    )
    lower_duplicate_vertex_ids: dict[int, int] = {}
    updated_vertices = vertices
    if lower_shared_vertex_ids.numel() > 0:
        duplicate_ids = torch.arange(
            vertex_count,
            vertex_count + lower_shared_vertex_ids.numel(),
            dtype=torch.long,
        )
        lower_duplicate_vertex_ids = {
            int(old_id): int(new_id)
            for old_id, new_id in zip(lower_shared_vertex_ids.tolist(), duplicate_ids.tolist())
        }
        updated_vertices = torch.cat(
            (vertices, vertices.index_select(0, lower_shared_vertex_ids)),
            dim=0,
        ).contiguous()
        lower_remap = torch.arange(updated_vertices.shape[0], dtype=torch.long)
        lower_remap[lower_shared_vertex_ids] = duplicate_ids
        kept_faces[kept_lower_faces] = lower_remap.index_select(
            0,
            kept_faces[kept_lower_faces].reshape(-1),
        ).reshape(-1, 3)

    updated_upper_ids = upper_ids
    lower_exclusive_ids = lower_ids[~upper_mask.index_select(0, lower_ids)]
    if lower_duplicate_vertex_ids:
        duplicate_tensor = torch.tensor(
            list(lower_duplicate_vertex_ids.values()),
            dtype=torch.long,
        )
        updated_lower_ids = torch.cat((lower_exclusive_ids, duplicate_tensor), dim=0)
    else:
        updated_lower_ids = lower_exclusive_ids
    updated_lower_ids = updated_lower_ids.unique(sorted=True).contiguous()

    updated_mesh = dict(mesh)
    updated_mesh["vertices"] = updated_vertices
    updated_mesh["faces"] = kept_faces.contiguous()
    updated_mesh["normals"] = compute_vertex_normals(updated_vertices, kept_faces)

    after_faces, after_edges = count_direct_lip_connections(
        kept_faces,
        updated_upper_ids,
        updated_lower_ids,
        int(updated_vertices.shape[0]),
    )
    removed_face_ids = torch.nonzero(~keep_face_mask, as_tuple=False).flatten().tolist()

    report = LipSplitReport(
        original_vertex_count=vertex_count,
        original_face_count=int(faces.shape[0]),
        final_vertex_count=int(updated_vertices.shape[0]),
        final_face_count=int(kept_faces.shape[0]),
        upper_vertex_count=int(upper_ids.numel()),
        lower_vertex_count=int(lower_ids.numel()),
        shared_vertex_count=int(shared_mask.sum().item()),
        upper_face_count=int(upper_faces.sum().item()),
        lower_face_count=int(lower_faces.sum().item()),
        neutral_face_count=int(neutral_faces.sum().item()),
        mixed_face_count=int(mixed_faces.sum().item()),
        removed_face_count=int((~keep_face_mask).sum().item()),
        duplicated_vertex_count=int(len(lower_duplicate_vertex_ids)),
        direct_lip_connection_faces_before=int(before_faces),
        direct_lip_connection_edges_before=int(before_edges),
        direct_lip_connection_faces_after=int(after_faces),
        direct_lip_connection_edges_after=int(after_edges),
        lower_duplicate_vertex_ids=lower_duplicate_vertex_ids,
        removed_face_ids=[int(face_id) for face_id in removed_face_ids],
    )
    return (
        updated_mesh,
        report,
        LipRegions(
            upper_vertex_ids=updated_upper_ids,
            lower_vertex_ids=updated_lower_ids,
            source=f"{lip_regions.source}:split",
        ),
    )


def count_direct_lip_connections(
    faces: torch.Tensor,
    upper_vertex_ids: torch.Tensor | Sequence[int],
    lower_vertex_ids: torch.Tensor | Sequence[int],
    vertex_count: int,
) -> tuple[int, int]:
    faces = faces.detach().cpu().long().contiguous()
    upper_mask = vertex_id_mask(normalize_vertex_ids(upper_vertex_ids, vertex_count), vertex_count)
    lower_mask = vertex_id_mask(normalize_vertex_ids(lower_vertex_ids, vertex_count), vertex_count)
    if faces.numel() == 0:
        return 0, 0

    upper_face = upper_mask.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=-1)
    lower_face = lower_mask.index_select(0, faces.reshape(-1)).reshape_as(faces).any(dim=-1)
    connection_faces = upper_face & lower_face

    edges = torch.cat(
        (
            faces[:, (0, 1)],
            faces[:, (1, 2)],
            faces[:, (2, 0)],
        ),
        dim=0,
    )
    edge_upper = upper_mask.index_select(0, edges.reshape(-1)).reshape_as(edges)
    edge_lower = lower_mask.index_select(0, edges.reshape(-1)).reshape_as(edges)
    connection_edges = (edge_upper[:, 0] & edge_lower[:, 1]) | (
        edge_lower[:, 0] & edge_upper[:, 1]
    )
    if connection_edges.any():
        sorted_edges = edges[connection_edges].sort(dim=-1).values
        edge_count = int(torch.unique(sorted_edges, dim=0).shape[0])
    else:
        edge_count = 0
    return int(connection_faces.sum().item()), edge_count


def lip_regions_from_mediapipe_mapping(
    vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    rings: int = 1,
    include_outer_lip: bool = False,
) -> LipRegions:
    vertex_count = int(vertices.shape[0])
    upper_landmarks = list(INNER_UPPER_LIP_MEDIAPIPE_IDS)
    lower_landmarks = list(INNER_LOWER_LIP_MEDIAPIPE_IDS)
    upper_seed_ids = _vertex_ids_for_landmarks(mapping, upper_landmarks, vertex_count)
    lower_seed_ids = _vertex_ids_for_landmarks(mapping, lower_landmarks, vertex_count)
    if include_outer_lip or _needs_outer_lip_landmarks(upper_seed_ids, lower_seed_ids):
        upper_landmarks.extend(OUTER_UPPER_LIP_MEDIAPIPE_IDS)
        lower_landmarks.extend(OUTER_LOWER_LIP_MEDIAPIPE_IDS)
        upper_seed_ids = _vertex_ids_for_landmarks(mapping, upper_landmarks, vertex_count)
        lower_seed_ids = _vertex_ids_for_landmarks(mapping, lower_landmarks, vertex_count)
    if rings > 0:
        upper_seed_ids = expand_vertex_ids_by_rings(
            faces,
            upper_seed_ids,
            vertex_count=vertex_count,
            rings=rings,
        )
        lower_seed_ids = expand_vertex_ids_by_rings(
            faces,
            lower_seed_ids,
            vertex_count=vertex_count,
            rings=rings,
        )
    return LipRegions(
        upper_vertex_ids=upper_seed_ids,
        lower_vertex_ids=lower_seed_ids,
        source="mediapipe",
    )


def lip_regions_from_mediapipe_paths(
    vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    rings: int = 2,
    include_outer_lip: bool = True,
) -> LipRegions:
    """Build lip bands by tracing mesh paths between MediaPipe lip landmarks."""

    vertex_count = int(vertices.shape[0])
    upper_landmarks = list(INNER_UPPER_LIP_MEDIAPIPE_IDS)
    lower_landmarks = list(INNER_LOWER_LIP_MEDIAPIPE_IDS)
    if include_outer_lip:
        upper_landmarks.extend(OUTER_UPPER_LIP_MEDIAPIPE_IDS)
        lower_landmarks.extend(OUTER_LOWER_LIP_MEDIAPIPE_IDS)

    upper_seed_ids = _vertex_ids_for_landmarks(mapping, upper_landmarks, vertex_count)
    lower_seed_ids = _vertex_ids_for_landmarks(mapping, lower_landmarks, vertex_count)
    if upper_seed_ids.numel() == 0:
        raise ValueError("No usable upper MediaPipe lip landmarks were found.")
    if lower_seed_ids.numel() == 0:
        raise ValueError("No usable lower MediaPipe lip landmarks were found.")

    neighbors = build_vertex_neighbors(faces, vertex_count)
    upper_path_ids = _landmark_path_vertex_ids(
        mapping,
        upper_landmarks,
        vertex_count,
        neighbors,
    )
    lower_path_ids = _landmark_path_vertex_ids(
        mapping,
        lower_landmarks,
        vertex_count,
        neighbors,
    )
    upper_ids = normalize_vertex_ids(
        list(upper_seed_ids.tolist()) + sorted(upper_path_ids),
        vertex_count,
    )
    lower_ids = normalize_vertex_ids(
        list(lower_seed_ids.tolist()) + sorted(lower_path_ids),
        vertex_count,
    )
    if rings > 0:
        upper_ids = expand_vertex_ids_by_rings(
            faces,
            upper_ids,
            vertex_count=vertex_count,
            rings=rings,
            neighbors=neighbors,
        )
        lower_ids = expand_vertex_ids_by_rings(
            faces,
            lower_ids,
            vertex_count=vertex_count,
            rings=rings,
            neighbors=neighbors,
        )
    return LipRegions(
        upper_vertex_ids=upper_ids,
        lower_vertex_ids=lower_ids,
        source="mediapipe_paths",
    )


def mouth_band_face_ids_from_mediapipe(
    vertices: torch.Tensor,
    faces: torch.Tensor,
    mapping: Mapping[int, int],
    *,
    axes: tuple[int, int] = (0, 2),
    padding: float = 0.0,
    depth_axis: int | None = 1,
    depth_padding: float = 0.006,
) -> torch.Tensor:
    """Faces whose centroids lie in the inner-mouth MediaPipe lip band."""

    vertices = vertices.detach().cpu().float().contiguous()
    faces = faces.detach().cpu().long().contiguous()
    if faces.ndim != 2 or faces.shape[-1] != 3:
        raise ValueError("mouth_band_face_ids_from_mediapipe expects triangular faces.")
    axis_x, axis_y = int(axes[0]), int(axes[1])
    if axis_x == axis_y or axis_x not in {0, 1, 2} or axis_y not in {0, 1, 2}:
        raise ValueError("axes must contain two different coordinate indices in [0, 2].")
    if depth_axis is not None:
        depth_axis = int(depth_axis)
        if depth_axis not in {0, 1, 2}:
            raise ValueError("depth_axis must be a coordinate index in [0, 2].")

    lip_landmark_ids = list(INNER_UPPER_LIP_MEDIAPIPE_IDS) + list(INNER_LOWER_LIP_MEDIAPIPE_IDS)
    polygon = _lip_landmark_polygon(
        vertices,
        mapping,
        list(INNER_UPPER_LIP_MEDIAPIPE_IDS),
        list(reversed(INNER_LOWER_LIP_MEDIAPIPE_IDS)),
        axes=(axis_x, axis_y),
    )
    if polygon.shape[0] < 3:
        raise ValueError("Not enough MediaPipe lip landmarks to build a mouth band.")
    if padding > 0:
        center = polygon.mean(dim=0, keepdim=True)
        polygon = center + (polygon - center) * (1.0 + float(padding))

    centroids = vertices.index_select(0, faces.reshape(-1)).reshape(-1, 3, 3).mean(dim=1)
    mask = points_in_polygon_2d(
        centroids[:, axis_x],
        centroids[:, axis_y],
        polygon,
    )
    if depth_axis is not None:
        depth_values = _landmark_axis_values(
            vertices,
            mapping,
            lip_landmark_ids,
            axis=depth_axis,
        )
        if depth_values.numel() > 0:
            low = depth_values.min() - float(depth_padding)
            high = depth_values.max() + float(depth_padding)
            mask &= (centroids[:, depth_axis] >= low) & (centroids[:, depth_axis] <= high)
    return torch.nonzero(mask, as_tuple=False).flatten().long().contiguous()


def remap_lower_lip_landmarks(
    mapping: Mapping[int, int],
    split_report: LipSplitReport | GeometricLipSeamReport,
) -> dict[int, int]:
    remapped = {int(key): int(value) for key, value in mapping.items()}
    duplicate_by_original = split_report.lower_duplicate_vertex_ids
    if not duplicate_by_original:
        return remapped
    for mediapipe_id in INNER_LOWER_LIP_MEDIAPIPE_IDS + OUTER_LOWER_LIP_MEDIAPIPE_IDS:
        vertex_id = remapped.get(int(mediapipe_id))
        if vertex_id in duplicate_by_original:
            remapped[int(mediapipe_id)] = duplicate_by_original[int(vertex_id)]
    return remapped


def _inner_lip_center_curve(
    vertices: torch.Tensor,
    mapping: Mapping[int, int],
) -> torch.Tensor:
    points: list[torch.Tensor] = []
    vertex_count = int(vertices.shape[0])
    for upper_landmark_id, lower_landmark_id in INNER_LIP_CENTER_PAIRS:
        upper_vertex_id = mapping.get(int(upper_landmark_id))
        lower_vertex_id = mapping.get(int(lower_landmark_id))
        if upper_vertex_id is None or lower_vertex_id is None:
            continue
        upper_vertex_id = int(upper_vertex_id)
        lower_vertex_id = int(lower_vertex_id)
        if not (
            0 <= upper_vertex_id < vertex_count
            and 0 <= lower_vertex_id < vertex_count
        ):
            continue
        points.append(
            (vertices[upper_vertex_id] + vertices[lower_vertex_id]) * 0.5
        )
    if not points:
        return torch.empty(0, 3, dtype=vertices.dtype)
    return torch.stack(points, dim=0).contiguous()


def _merge_curve_points_with_duplicate_horizontal_values(
    points: torch.Tensor,
    *,
    horizontal_axis: int,
    epsilon: float = 1.0e-7,
) -> torch.Tensor:
    groups: list[list[torch.Tensor]] = []
    for point in points:
        if not groups or abs(
            float(point[horizontal_axis] - groups[-1][-1][horizontal_axis])
        ) > epsilon:
            groups.append([point])
        else:
            groups[-1].append(point)
    return torch.stack(
        [torch.stack(group, dim=0).mean(dim=0) for group in groups],
        dim=0,
    ).contiguous()


def _piecewise_linear_interpolate(
    query: torch.Tensor,
    knot_x: torch.Tensor,
    knot_y: torch.Tensor,
) -> torch.Tensor:
    knot_x = knot_x.contiguous()
    knot_y = knot_y.contiguous()
    indices = torch.searchsorted(knot_x, query.contiguous(), right=True)
    right = indices.clamp(min=1, max=int(knot_x.numel()) - 1)
    left = right - 1
    x0 = knot_x.index_select(0, left)
    x1 = knot_x.index_select(0, right)
    y0 = knot_y.index_select(0, left)
    y1 = knot_y.index_select(0, right)
    weight = (query - x0) / (x1 - x0).clamp_min(1.0e-12)
    return y0 + (y1 - y0) * weight


def load_lip_regions(
    path: Path,
    vertex_count: int,
    *,
    upper_label_ids: Sequence[int] = (),
    lower_label_ids: Sequence[int] = (),
) -> LipRegions:
    path = Path(path).expanduser()
    suffix = path.suffix.lower()
    if suffix == ".json":
        with path.open("r", encoding="utf-8") as handle:
            payload = json.load(handle)
    elif suffix == ".npz":
        payload = dict(np.load(path, allow_pickle=True))
    elif suffix == ".npy":
        payload = {"vertex_labels": np.load(path, allow_pickle=True)}
    elif suffix in {".pt", ".pth"}:
        loaded = torch.load(path, map_location="cpu")
        payload = loaded if isinstance(loaded, Mapping) else {"vertex_labels": loaded}
    else:
        raise ValueError(
            f"Unsupported lip region file {path.suffix!r}. "
            "Use JSON, NPZ, NPY, PT, or PTH."
        )
    if not isinstance(payload, Mapping):
        raise ValueError(f"Expected {path} to contain a mapping-like payload.")

    upper = _extract_region_ids(
        payload,
        vertex_count,
        keys=(
            "upper_lip_vertex_ids",
            "upper_vertex_ids",
            "upper_lip_ids",
            "upper_ids",
            "upper_lip",
            "upper",
            "upper_lip_mask",
            "upper_mask",
        ),
    )
    lower = _extract_region_ids(
        payload,
        vertex_count,
        keys=(
            "lower_lip_vertex_ids",
            "lower_vertex_ids",
            "lower_lip_ids",
            "lower_ids",
            "lower_lip",
            "lower",
            "lower_lip_mask",
            "lower_mask",
        ),
    )
    if upper is None or lower is None:
        upper, lower = _extract_regions_from_labels(
            payload,
            vertex_count,
            upper_label_ids=upper_label_ids,
            lower_label_ids=lower_label_ids,
        )
    if upper is None or lower is None:
        raise ValueError(
            f"{path} must provide upper/lower lip vertex ids, masks, or labels."
        )
    return LipRegions(
        upper_vertex_ids=normalize_vertex_ids(upper, vertex_count),
        lower_vertex_ids=normalize_vertex_ids(lower, vertex_count),
        source=str(path),
    )


def load_vertex_ids_arg(value: str | None, vertex_count: int) -> torch.Tensor:
    if value is None or str(value).strip() == "":
        return torch.empty(0, dtype=torch.long)
    candidate = Path(str(value)).expanduser()
    if candidate.is_file():
        text = candidate.read_text(encoding="utf-8")
    else:
        text = str(value)
    ids: list[int] = []
    for token in text.replace(",", " ").split():
        stripped = token.strip()
        if stripped:
            ids.append(int(stripped))
    return normalize_vertex_ids(ids, vertex_count)


def expand_vertex_ids_by_rings(
    faces: torch.Tensor,
    vertex_ids: torch.Tensor | Sequence[int],
    *,
    vertex_count: int,
    rings: int,
    neighbors: list[set[int]] | None = None,
) -> torch.Tensor:
    selected = set(normalize_vertex_ids(vertex_ids, vertex_count).tolist())
    if not selected or rings <= 0:
        return torch.tensor(sorted(selected), dtype=torch.long)
    if neighbors is None:
        neighbors = build_vertex_neighbors(faces, vertex_count)

    frontier = set(selected)
    for _ in range(int(rings)):
        grown: set[int] = set()
        for vertex_id in frontier:
            grown.update(neighbors[vertex_id])
        grown -= selected
        if not grown:
            break
        selected.update(grown)
        frontier = grown
    return torch.tensor(sorted(selected), dtype=torch.long)


def build_vertex_neighbors(
    faces: torch.Tensor,
    vertex_count: int,
) -> list[set[int]]:
    neighbors: list[set[int]] = [set() for _ in range(vertex_count)]
    for a, b, c in faces.detach().cpu().long().tolist():
        if 0 <= a < vertex_count and 0 <= b < vertex_count:
            neighbors[a].add(b)
            neighbors[b].add(a)
        if 0 <= b < vertex_count and 0 <= c < vertex_count:
            neighbors[b].add(c)
            neighbors[c].add(b)
        if 0 <= c < vertex_count and 0 <= a < vertex_count:
            neighbors[c].add(a)
            neighbors[a].add(c)
    return neighbors


def normalize_vertex_ids(
    vertex_ids: torch.Tensor | Sequence[int] | np.ndarray,
    vertex_count: int,
) -> torch.Tensor:
    if isinstance(vertex_ids, torch.Tensor):
        ids = vertex_ids.detach().cpu().long().flatten()
    else:
        ids = torch.as_tensor(vertex_ids, dtype=torch.long).flatten()
    if ids.numel() == 0:
        return torch.empty(0, dtype=torch.long)
    ids = ids[(ids >= 0) & (ids < int(vertex_count))]
    return ids.unique(sorted=True).contiguous()


def normalize_index_ids(
    index_ids: torch.Tensor | Sequence[int] | np.ndarray,
    count: int,
) -> torch.Tensor:
    if isinstance(index_ids, torch.Tensor):
        ids = index_ids.detach().cpu().long().flatten()
    else:
        ids = torch.as_tensor(index_ids, dtype=torch.long).flatten()
    if ids.numel() == 0:
        return torch.empty(0, dtype=torch.long)
    ids = ids[(ids >= 0) & (ids < int(count))]
    return ids.unique(sorted=True).contiguous()


def vertex_id_mask(vertex_ids: torch.Tensor, vertex_count: int) -> torch.Tensor:
    mask = torch.zeros(int(vertex_count), dtype=torch.bool)
    if vertex_ids.numel() > 0:
        mask[vertex_ids.long()] = True
    return mask


def compute_vertex_normals(vertices: torch.Tensor, faces: torch.Tensor) -> torch.Tensor:
    if faces.numel() == 0:
        return torch.zeros_like(vertices)
    triangles = vertices[faces]
    face_normals = torch.cross(
        triangles[:, 1] - triangles[:, 0],
        triangles[:, 2] - triangles[:, 0],
        dim=-1,
    )
    normals = torch.zeros_like(vertices)
    for corner in range(3):
        normals.index_add_(0, faces[:, corner], face_normals)
    return F.normalize(normals, dim=-1, eps=1.0e-6).contiguous()


def _lower_shared_vertex_ids(
    *,
    faces: torch.Tensor,
    lower_face_mask: torch.Tensor,
    shared_mask: torch.Tensor,
) -> torch.Tensor:
    if faces.numel() == 0 or not bool(lower_face_mask.any().item()):
        return torch.empty(0, dtype=torch.long)
    lower_faces = faces[lower_face_mask]
    shared_in_lower_faces = shared_mask.index_select(0, lower_faces.reshape(-1))
    if not bool(shared_in_lower_faces.any().item()):
        return torch.empty(0, dtype=torch.long)
    return lower_faces.reshape(-1)[shared_in_lower_faces].unique(sorted=True).contiguous()


def _vertex_ids_for_landmarks(
    mapping: Mapping[int, int],
    landmark_ids: Sequence[int],
    vertex_count: int,
) -> torch.Tensor:
    ids = []
    for landmark_id in landmark_ids:
        vertex_id = mapping.get(int(landmark_id))
        if vertex_id is not None and 0 <= int(vertex_id) < vertex_count:
            ids.append(int(vertex_id))
    return normalize_vertex_ids(ids, vertex_count)


def _ordered_mapped_vertex_ids(
    mapping: Mapping[int, int],
    landmark_ids: Sequence[int],
    vertex_count: int,
) -> list[int]:
    ids: list[int] = []
    for landmark_id in landmark_ids:
        vertex_id = mapping.get(int(landmark_id))
        if vertex_id is None:
            continue
        vertex_id = int(vertex_id)
        if 0 <= vertex_id < vertex_count:
            ids.append(vertex_id)
    return ids


def _compact_polygon_points(points: torch.Tensor) -> torch.Tensor:
    compact: list[torch.Tensor] = []
    for point in points:
        if not compact or not bool(torch.equal(point, compact[-1])):
            compact.append(point)
    if len(compact) > 1 and bool(torch.equal(compact[0], compact[-1])):
        compact.pop()
    if not compact:
        return torch.empty(0, 2, dtype=points.dtype)
    return torch.stack(compact, dim=0).contiguous()


def _landmark_path_vertex_ids(
    mapping: Mapping[int, int],
    landmark_ids: Sequence[int],
    vertex_count: int,
    neighbors: Sequence[set[int]],
) -> set[int]:
    path_ids: set[int] = set()
    previous: int | None = None
    for landmark_id in landmark_ids:
        vertex_id = mapping.get(int(landmark_id))
        if vertex_id is None:
            continue
        vertex_id = int(vertex_id)
        if vertex_id < 0 or vertex_id >= vertex_count:
            continue
        path_ids.add(vertex_id)
        if previous is not None and previous != vertex_id:
            path_ids.update(_shortest_vertex_path(previous, vertex_id, neighbors))
        previous = vertex_id
    return path_ids


def _shortest_vertex_path(
    start: int,
    target: int,
    neighbors: Sequence[set[int]],
) -> list[int]:
    if start == target:
        return [int(start)]
    parents: dict[int, int] = {int(start): -1}
    queue: deque[int] = deque([int(start)])
    while queue:
        current = queue.popleft()
        for neighbor in neighbors[current]:
            neighbor = int(neighbor)
            if neighbor in parents:
                continue
            parents[neighbor] = current
            if neighbor == target:
                path = [target]
                while path[-1] != start:
                    path.append(parents[path[-1]])
                path.reverse()
                return path
            queue.append(neighbor)
    return []


def _lip_landmark_polygon(
    vertices: torch.Tensor,
    mapping: Mapping[int, int],
    first_curve: Sequence[int],
    second_curve: Sequence[int],
    *,
    axes: tuple[int, int],
) -> torch.Tensor:
    points: list[list[float]] = []
    vertex_count = int(vertices.shape[0])
    for landmark_id in list(first_curve) + list(second_curve):
        vertex_id = mapping.get(int(landmark_id))
        if vertex_id is None:
            continue
        vertex_id = int(vertex_id)
        if 0 <= vertex_id < vertex_count:
            points.append(
                [
                    float(vertices[vertex_id, axes[0]].item()),
                    float(vertices[vertex_id, axes[1]].item()),
                ]
            )
    if not points:
        return torch.empty(0, 2, dtype=torch.float32)
    compact: list[list[float]] = []
    for point in points:
        if not compact or point != compact[-1]:
            compact.append(point)
    if len(compact) > 1 and compact[0] == compact[-1]:
        compact.pop()
    return torch.tensor(compact, dtype=torch.float32)


def _landmark_axis_values(
    vertices: torch.Tensor,
    mapping: Mapping[int, int],
    landmark_ids: Sequence[int],
    *,
    axis: int,
) -> torch.Tensor:
    values: list[float] = []
    vertex_count = int(vertices.shape[0])
    for landmark_id in landmark_ids:
        vertex_id = mapping.get(int(landmark_id))
        if vertex_id is None:
            continue
        vertex_id = int(vertex_id)
        if 0 <= vertex_id < vertex_count:
            values.append(float(vertices[vertex_id, int(axis)].item()))
    if not values:
        return torch.empty(0, dtype=torch.float32)
    return torch.tensor(values, dtype=torch.float32)


def points_in_polygon_2d(
    xs: torch.Tensor,
    ys: torch.Tensor,
    polygon: torch.Tensor,
) -> torch.Tensor:
    if polygon.ndim != 2 or polygon.shape[0] < 3 or polygon.shape[1] != 2:
        return torch.zeros_like(xs, dtype=torch.bool)
    xs = xs.detach().cpu().float()
    ys = ys.detach().cpu().float()
    poly = polygon.detach().cpu().float()
    inside = torch.zeros_like(xs, dtype=torch.bool)
    previous = int(poly.shape[0]) - 1
    for current in range(int(poly.shape[0])):
        xi = poly[current, 0]
        yi = poly[current, 1]
        xj = poly[previous, 0]
        yj = poly[previous, 1]
        crosses = (yi > ys) != (yj > ys)
        intersect_x = (xj - xi) * (ys - yi) / (yj - yi + 1.0e-12) + xi
        inside ^= crosses & (xs < intersect_x)
        previous = current
    return inside


def _needs_outer_lip_landmarks(
    upper_seed_ids: torch.Tensor,
    lower_seed_ids: torch.Tensor,
) -> bool:
    if upper_seed_ids.numel() == 0 or lower_seed_ids.numel() == 0:
        return True
    upper = set(int(vertex_id) for vertex_id in upper_seed_ids.tolist())
    lower = set(int(vertex_id) for vertex_id in lower_seed_ids.tolist())
    return not (upper - lower) or not (lower - upper)


def _extract_region_ids(
    payload: Mapping[str, Any],
    vertex_count: int,
    *,
    keys: Sequence[str],
) -> torch.Tensor | None:
    for key in keys:
        if key not in payload:
            continue
        value = payload[key]
        array = _to_numpy(value)
        if array is None:
            continue
        flat = array.reshape(-1)
        if flat.size == vertex_count and (
            flat.dtype == np.bool_ or np.all((flat == 0) | (flat == 1))
        ):
            return torch.from_numpy(np.nonzero(flat.astype(bool))[0]).long()
        return normalize_vertex_ids(flat.astype(np.int64), vertex_count)
    return None


def _extract_regions_from_labels(
    payload: Mapping[str, Any],
    vertex_count: int,
    *,
    upper_label_ids: Sequence[int],
    lower_label_ids: Sequence[int],
) -> tuple[torch.Tensor | None, torch.Tensor | None]:
    labels = None
    for key in ("vertex_labels", "labels", "sapiens_labels", "label_ids"):
        if key in payload:
            labels = _to_numpy(payload[key])
            break
    if labels is None:
        return None, None
    labels = labels.reshape(-1)
    if labels.size != vertex_count:
        raise ValueError(
            "Per-vertex lip label arrays must have one label per mesh vertex."
        )
    upper_values = set(int(value) for value in upper_label_ids)
    lower_values = set(int(value) for value in lower_label_ids)
    if not upper_values:
        upper_values = _label_ids_for_names(
            payload,
            ("upper_lip", "upper lip", "upperlip"),
        )
    if not lower_values:
        lower_values = _label_ids_for_names(
            payload,
            ("lower_lip", "lower lip", "lowerlip"),
        )
    if not upper_values or not lower_values:
        return None, None
    upper_mask = np.isin(labels.astype(np.int64), list(upper_values))
    lower_mask = np.isin(labels.astype(np.int64), list(lower_values))
    return (
        torch.from_numpy(np.nonzero(upper_mask)[0]).long(),
        torch.from_numpy(np.nonzero(lower_mask)[0]).long(),
    )


def _label_ids_for_names(payload: Mapping[str, Any], names: Sequence[str]) -> set[int]:
    label_map = payload.get("label_map", payload.get("labels_map"))
    if not isinstance(label_map, Mapping):
        return set()
    wanted = {name.lower().replace("-", "_") for name in names}
    ids: set[int] = set()
    for raw_key, raw_value in label_map.items():
        try:
            label_id = int(raw_key)
            label_name = str(raw_value)
        except (TypeError, ValueError):
            try:
                label_id = int(raw_value)
                label_name = str(raw_key)
            except (TypeError, ValueError):
                continue
        normalized_name = label_name.lower().replace("-", "_")
        if normalized_name in wanted:
            ids.add(label_id)
    return ids


def _to_numpy(value: Any) -> np.ndarray | None:
    if isinstance(value, torch.Tensor):
        return value.detach().cpu().numpy()
    if isinstance(value, np.ndarray):
        return value
    if isinstance(value, (list, tuple)):
        return np.asarray(value)
    return None


```

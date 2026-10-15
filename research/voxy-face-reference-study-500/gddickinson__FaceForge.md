# Facial rendering source scan: gddickinson/FaceForge

Repository: https://github.com/gddickinson/FaceForge

Pinned source revision: cab98987a6df9b8b2ec6347bde0fc00996b690de

Discovery topic: FACS face

Description: FACS-based facial animation with full-body anatomy visualization.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["assets/config/eye_colors.json", "assets/human-base-meshes-bundle-v1.4.1/thumbnails/realistic_eye.png", "assets/human-base-meshes-bundle-v1.4.1/thumbnails/stylized_eye.png", "src/faceforge/animation/eye_tracking.py", "src/faceforge/ui/widgets/eye_color_grid.py"], "skin": ["assets/config/skin.json", "assets/config/skinning_overrides.json", "src/faceforge/body/skin_morph.py", "src/faceforge/body/skinning_cache.py", "src/faceforge/body/skinning_ops.py", "tests/body/test_hand_curl_skinning.py", "tests/body/test_skinning_cache.py", "tests/body/test_skinning_diagnostics.py", "tests/body/test_skinning_ops.py", "tests/body/test_skinning_per_binding_skip.py", "tests/body/test_skinning_resnapshot.py", "tests/body/test_skinning_under_scene_wrapper.py", "tools/fit_skeleton_to_skin.py", "tools/render_skin_proof.py", "tools/run_skinning_diagnostic.py", "tools/skin_defect_views.py", "tools/skin_deformation_quality.py", "tools/skinning_scorer.py", "tools/visual_skinning_test.py"], "jaw": ["assets/config/muscles/jaw_muscles.json", "assets/human-base-meshes-bundle-v1.4.1/thumbnails/realistic_jaw.png", "assets/human-base-meshes-bundle-v1.4.1/thumbnails/stylized_jaw.png", "src/faceforge/anatomy/jaw_muscles.py"], "teeth": ["assets/config/teeth.json"], "deform": ["tests/tools/test_deformation_quality.py", "tools/deformation_quality.py", "tools/neck_deformation_quality.py", "tools/skin_deformation_quality.py"], "facs": ["src/faceforge/anatomy/facs.py"], "shader": ["src/faceforge/rendering/shader_program.py", "src/faceforge/rendering/shaders/_common.glsl", "src/faceforge/rendering/shaders/_lighting.glsl", "src/faceforge/rendering/shaders/blueprint.frag", "src/faceforge/rendering/shaders/cartoon.frag", "src/faceforge/rendering/shaders/color_atlas.frag", "src/faceforge/rendering/shaders/default.vert", "src/faceforge/rendering/shaders/ethereal.frag", "src/faceforge/rendering/shaders/hologram.frag", "src/faceforge/rendering/shaders/illustration.frag", "src/faceforge/rendering/shaders/medical.frag", "src/faceforge/rendering/shaders/pen_ink.frag", "src/faceforge/rendering/shaders/phong.frag", "src/faceforge/rendering/shaders/phong_pointlight.frag", "src/faceforge/rendering/shaders/points.frag", "src/faceforge/rendering/shaders/points.vert", "src/faceforge/rendering/shaders/porcelain.frag", "src/faceforge/rendering/shaders/sepia.frag", "src/faceforge/rendering/shaders/thermal.frag", "src/faceforge/rendering/shaders/wireframe.frag", "src/faceforge/rendering/shaders/xray.frag", "tests/rendering/test_shader_compile.py", "tests/rendering/test_shader_gpu_agreement.py", "tests/rendering/test_shader_semantics.py", "tests/rendering/test_shader_static.py"], "rig": ["src/faceforge/exercise/equipment_rig.py", "tests/exercise/test_equipment_and_rig.py"], "face": ["INTERFACE.md", "assets/config/face_features.json", "assets/config/face_regions.json", "assets/meshdata/face.json", "src/faceforge.egg-info/PKG-INFO", "src/faceforge.egg-info/SOURCES.txt", "src/faceforge.egg-info/dependency_links.txt", "src/faceforge.egg-info/entry_points.txt", "src/faceforge.egg-info/requires.txt", "src/faceforge.egg-info/top_level.txt", "src/faceforge/README.md", "src/faceforge/__init__.py", "src/faceforge/anatomy/README.md", "src/faceforge/anatomy/__init__.py", "src/faceforge/anatomy/answer_explanations.py", "src/faceforge/anatomy/back_neck_muscles.py", "src/faceforge/anatomy/bone_anchors.py", "src/faceforge/anatomy/bone_collision.py", "src/faceforge/anatomy/curricula.py", "src/faceforge/anatomy/distractors.py", "src/faceforge/anatomy/exam_items.py", "src/faceforge/anatomy/exam_session.py", "src/faceforge/anatomy/expression_muscles.py", "src/faceforge/anatomy/face.py", "src/faceforge/anatomy/face_features.py", "src/faceforge/anatomy/facs.py", "src/faceforge/anatomy/fascia.py", "src/faceforge/anatomy/fibre_field.py", "src/faceforge/anatomy/fma_taxonomy.py", "src/faceforge/anatomy/head_rotation.py"], "lip": ["src/faceforge/exercise/clip_builder.py"]}

## Source file: tools/skin_deformation_quality.py

{
  "path": "tools/skin_deformation_quality.py",
  "blob_sha": "5fdb03f31ec67ca160f0f39360f57e5684e8b775",
  "sha256": "0328402a86355f728733a79e64c3d326da1d558f345f7a6188b9b5c1e6f6f9a7",
  "lines": 316,
  "bytes": 14451,
  "selection_score": 30,
  "keyword_line_counts": {
    "skin": 59,
    "gum": 23,
    "deform": 5,
    "face": 7
  },
  "url": "https://github.com/gddickinson/FaceForge/blob/cab98987a6df9b8b2ec6347bde0fc00996b690de/tools/skin_deformation_quality.py",
  "function_signature_candidates": [
    {
      "line": 63,
      "text": "def _skin_binding(soft):"
    },
    {
      "line": 70,
      "text": "def _edges(soft, b):"
    },
    {
      "line": 80,
      "text": "def _spikes(disp, e, n):"
    },
    {
      "line": 92,
      "text": "def _moved_joints(soft, before):"
    },
    {
      "line": 98,
      "text": "def _static_mask(b, moved, n):"
    },
    {
      "line": 110,
      "text": "def measure(pose: dict, ctx, soft, b, cache) -> dict:"
    },
    {
      "line": 157,
      "text": "def main(argv=None) -> int:"
    }
  ]
}

```text

"""Measure how the body SKIN deforms: tearing, collapse and containment.

``tools/deformation_quality.py`` measures muscle layers.  The skin is a
different problem: one 791,729-vertex sheet bound to every chain, where the
failure is a torn edge rather than a muscle off its bone.  Reported as
"individual pixels either left behind or attached to movements of the
different body parts", and visible in `results/skin_tearing.png` as straight
horizontal seams at the hips, elbows and knees plus speckle over the hands
and feet.

    PYTHONPATH=src python -m tools.skin_deformation_quality
    PYTHONPATH=src python -m tools.skin_deformation_quality --diffuse
    PYTHONPATH=src python -m tools.skin_deformation_quality --diffuse --locality 0.3

Every number is measured at run time.  ``--diffuse`` turns on
``SoftTissueSkinning.DIFFUSE_WEIGHTS`` before the binding is solved; the
binding cache keys on it, so the two configurations do not share a solve.
"""

from __future__ import annotations

import argparse
import time

import numpy as np

#: Poses to measure, in normalised BodyState DOFs.  The first is the control.
POSES: dict[str, dict] = {
    "neutral": {},
    "hip hinge (deadlift setup)": {
        "hip_r_flex": 0.85, "hip_l_flex": 0.85,
        "knee_r_flex": 0.35, "knee_l_flex": 0.35,
        "shoulder_r_flex": 0.25, "shoulder_l_flex": 0.25,
    },
    "deep squat": {
        "hip_r_flex": 1.0, "hip_l_flex": 1.0,
        "knee_r_flex": 0.9, "knee_l_flex": 0.9,
        "ankle_r_flex": 0.6, "ankle_l_flex": 0.6,
    },
    # Shoulder abduction's range is 90 degrees, so 1.0 is shoulder height,
    # not overhead.  This is the pose the anterolateral spikes show in.
    "arms to shoulder height": {
        "shoulder_r_abduct": 1.0, "shoulder_l_abduct": 1.0,
        "elbow_r_flex": 0.2, "elbow_l_flex": 0.2,
    },
    "trunk flexed and rotated": {
        "spine_flex": 0.8, "spine_rotation": 0.6, "spine_lat_bend": 0.4,
    },
}

#: An edge stretched past this is counted as torn.
TORN = 2.0

#: A vertex that moves this many model units further than its mesh
#: neighbours' mean is counted as a spike.  Edge stretch alone does not see
#: these: a vertex drawn out on a long thin strip stretches only the few
#: edges at its base, while what reads on screen is the strip.
SPIKE = 5.0

SETTLE_FRAMES = 90


def _skin_binding(soft):
    for b in soft.bindings:
        if not b.is_muscle and b.edge_pairs is not None:
            return b
    return None


def _edges(soft, b):
    ref = soft._resolved_reference(b)
    n = len(ref)
    e = np.asarray(b.edge_pairs).reshape(-1, 2)
    e = e[(e[:, 0] < n) & (e[:, 1] < n)]
    rest_len = np.linalg.norm(ref[e[:, 0]] - ref[e[:, 1]], axis=1)
    keep = rest_len > 1e-6
    return ref, n, e[keep], rest_len[keep]


def _spikes(disp, e, n):
    """Vertices whose displacement exceeds their neighbours' mean by SPIKE."""
    src = np.concatenate([e[:, 0], e[:, 1]])
    dst = np.concatenate([e[:, 1], e[:, 0]])
    count = np.bincount(src, minlength=n).astype(np.float64)
    total = np.bincount(src, weights=disp[dst], minlength=n)
    has = count > 0
    mean = np.zeros(n)
    mean[has] = total[has] / count[has]
    return int(((disp - mean) > SPIKE).sum())


def _moved_joints(soft, before):
    after = np.array([np.asarray(j.node.world_matrix, dtype=np.float64)
                      for j in soft.joints])
    return np.linalg.norm((after - before).reshape(len(before), -1), axis=1) > 1e-6


def _static_mask(b, moved, n):
    """True where no joint driving this vertex has moved."""
    ji = np.asarray(b.joint_indices)[:n]
    si = np.asarray(b.secondary_indices)[:n]
    driven = moved[ji] | moved[si]
    inf = getattr(b, "influences", None)
    if inf is not None and b.influence_weights is not None:
        w = np.asarray(b.influence_weights)[:n]
        driven = driven | (moved[np.asarray(inf)[:n]] & (w > 0.0)).any(axis=1)
    return ~driven


def measure(pose: dict, ctx, soft, b, cache) -> dict:
    from faceforge.core.state import BodyState

    ref, n, e, rest_len = cache
    defaults = BodyState()
    for state in (ctx.state.body, ctx.state.target_body):
        for name, value in vars(defaults).items():
            if hasattr(state, name):
                setattr(state, name, value)
        for name, value in pose.items():
            if not hasattr(state, name):
                raise SystemExit(f"unknown BodyState field {name!r}")
            setattr(state, name, value)

    before = np.array([np.asarray(j.node.world_matrix, dtype=np.float64)
                       for j in soft.joints])
    for _ in range(SETTLE_FRAMES):
        ctx.simulation.step(1 / 60)
    moved = _moved_joints(soft, before)

    pos = np.asarray(b.mesh.geometry.positions,
                     dtype=np.float64).reshape(-1, 3)[:n]
    cur = np.linalg.norm(pos[e[:, 0]] - pos[e[:, 1]], axis=1)
    s = cur / rest_len

    disp = np.linalg.norm(pos - ref, axis=1)
    ji = np.asarray(b.joint_indices)[:n]
    seam = ji[e[:, 0]] != ji[e[:, 1]]
    static = _static_mask(b, moved, n)
    drift = (float(np.linalg.norm(pos[static] - ref[static], axis=1).max())
             if static.any() else 0.0)

    return {
        "p99": float(np.percentile(s, 99)),
        "p999": float(np.percentile(s, 99.9)),
        "max": float(s.max()),
        "min": float(s.min()),
        "torn": int((s > TORN).sum()),
        "seam_p99": float(np.percentile(s[seam], 99)) if seam.any() else 0.0,
        "bulk_p99": float(np.percentile(s[~seam], 99)) if (~seam).any() else 0.0,
        "spikes": _spikes(disp, e, n),
        "containment": drift,
        "static": int(static.sum()),
        "moved_joints": int(moved.sum()),
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--diffuse", action="store_true",
                    help="turn on DIFFUSE_WEIGHTS before the binding is solved")
    ap.add_argument("--locality", type=float, default=None,
                    help="DIFFUSION_LOCALITY (smaller spreads influence further)")
    ap.add_argument("--muscle-bias", type=float, default=None,
                    help="MUSCLE_WEIGHT_BIAS: discount an influence whose flesh is far")
    ap.add_argument("--flesh-radius", type=float, default=None,
                    help="MUSCLE_ELIGIBILITY_RADIUS: flesh that vouches for a chain")
    ap.add_argument("--inward-seeds", action="store_true",
                    help="SEED_INWARD_ONLY: a chain seeds only where its bone is inward")
    ap.add_argument("--hybrid-limit", action="store_true",
                    help="SPATIAL_LIMIT_ON_HYBRID: judge eligibility across the surface")
    ap.add_argument("--hybrid-scale", type=float, default=None,
                    help="HYBRID_LIMIT_SCALE: multiplier on the limits when it does")
    ap.add_argument("--cutoff-ratio", type=float, default=None,
                    help="INFLUENCE_CUTOFF_RATIO: support proportional to the nearest")
    ap.add_argument("--own-flesh", action="store_true",
                    help="SEED_ON_OWN_FLESH: a chain seeds only skin on its own flesh")
    ap.add_argument("--cross-cost", type=float, default=None,
                    help="CROSS_PART_EDGE_COST: price of stepping between body parts")
    ap.add_argument("--cut-below", type=float, default=None,
                    help="CROSS_PART_CUT_BELOW: sever crossings this far below the shoulder")
    ap.add_argument("--muscle-weight", type=float, default=None,
                    help="MUSCLE_FIELD_WEIGHT: how much the flesh distance counts")
    ap.add_argument("--min-spatial", type=float, default=None,
                    help="skinning.min_spatial: floor on a chain's spatial reach")
    ap.add_argument("--spatial-factor", type=float, default=None,
                    help="skinning.spatial_factor: reach as a fraction of chain size")
    ap.add_argument("--contact", type=float, default=None,
                    help="SEED_CONTACT_RADIUS: bootstrap ownership from bone "
                         "within this distance of skin (0 = Euclidean ownership)")
    ap.add_argument("--bridge-contacts", type=int, default=None,
                    help="BRIDGE_CONTACTS: how many contacts join each island patch")
    ap.add_argument("--bridge", type=float, default=None,
                    help="GEODESIC_BRIDGE: how far to stitch disconnected skin "
                         "patches into the geodesic graph (0 = not at all)")
    ap.add_argument("--seed-margin", type=float, default=None,
                    help="SEED_CONFIDENCE_MARGIN: how much nearer than the "
                         "runner-up a chain must be to be seeded (1 = any)")
    ap.add_argument("--radius-seeds", action="store_true",
                    help="seed every chain from any vertex within SEED_RADIUS "
                         "(the behaviour before ownership seeding)")
    ap.add_argument("--spatial-limit", type=float, default=None,
                    help="SKIN_SPATIAL_LIMIT: Euclidean guard on chain eligibility")
    ap.add_argument("--cutoff", type=float, default=None,
                    help="INFLUENCE_CUTOFF_BAND: compact support as a band in model "
                         "units past the nearest segment (0 = rank-based)")
    ap.add_argument("--influences", type=int, default=None,
                    help="SKIN_INFLUENCES: bones per skin vertex (2 restores the "
                         "two-joint blend)")
    args = ap.parse_args(argv)

    from PySide6.QtWidgets import QApplication
    _app = QApplication.instance() or QApplication([])

    from faceforge.appcontext import build_app_context
    from faceforge.body.soft_tissue import SoftTissueSkinning
    from faceforge.controllers import build_controllers
    from faceforge.coordination import demand_loaders
    from faceforge.coordination.asset_load_sequence import AssetLoadSequence

    if args.diffuse:
        SoftTissueSkinning.DIFFUSE_WEIGHTS = True
    if args.locality is not None:
        SoftTissueSkinning.DIFFUSION_LOCALITY = args.locality
    if args.influences is not None:
        SoftTissueSkinning.SKIN_INFLUENCES = args.influences
    if args.cutoff is not None:
        SoftTissueSkinning.INFLUENCE_CUTOFF_BAND = args.cutoff
    if args.spatial_limit is not None:
        demand_loaders.SKIN_SPATIAL_LIMIT = args.spatial_limit
    if args.radius_seeds:
        SoftTissueSkinning.SEED_FROM_OWNED_SKIN = False
    if args.seed_margin is not None:
        SoftTissueSkinning.SEED_CONFIDENCE_MARGIN = args.seed_margin
    if args.bridge is not None:
        SoftTissueSkinning.GEODESIC_BRIDGE = args.bridge
    if args.bridge_contacts is not None:
        SoftTissueSkinning.BRIDGE_CONTACTS = args.bridge_contacts
    if args.contact is not None:
        SoftTissueSkinning.SEED_CONTACT_RADIUS = args.contact

    ctx = build_app_context(argv=[])
    controllers = build_controllers(ctx)
    AssetLoadSequence(ctx).run()
    soft = ctx.simulation.soft_tissue
    # Instance tunables, set after the skinning exists and before the skin
    # binding is solved.
    if args.muscle_weight is not None:
        SoftTissueSkinning.MUSCLE_FIELD_WEIGHT = args.muscle_weight
    if args.muscle_bias is not None:
        SoftTissueSkinning.MUSCLE_WEIGHT_BIAS = args.muscle_bias
    if args.flesh_radius is not None:
        SoftTissueSkinning.MUSCLE_ELIGIBILITY_RADIUS = args.flesh_radius
    if args.inward_seeds:
        SoftTissueSkinning.SEED_INWARD_ONLY = True
    if args.hybrid_limit:
        SoftTissueSkinning.SPATIAL_LIMIT_ON_HYBRID = True
    if args.hybrid_scale is not None:
        SoftTissueSkinning.HYBRID_LIMIT_SCALE = args.hybrid_scale
    if args.cutoff_ratio is not None:
        SoftTissueSkinning.INFLUENCE_CUTOFF_RATIO = args.cutoff_ratio
    if args.own_flesh:
        SoftTissueSkinning.SEED_ON_OWN_FLESH = True
    if args.cross_cost is not None:
        SoftTissueSkinning.CROSS_PART_EDGE_COST = args.cross_cost
    if args.cut_below is not None:
        SoftTissueSkinning.CROSS_PART_CUT_BELOW = args.cut_below
    if args.min_spatial is not None:
        soft.min_spatial = args.min_spatial
    if args.spatial_factor is not None:
        soft.spatial_factor = args.spatial_factor

    t0 = time.perf_counter()
    controllers.loaders.load_skin()
    solve_s = time.perf_counter() - t0

    b = _skin_binding(soft)
    if b is None:
        print("no skin binding with edges -- nothing to measure")
        return 1
    cache = _edges(soft, b)
    print(f"skin: {cache[1]} vertices, {len(cache[2])} edges; "
          f"load+solve {solve_s:.1f} s; "
          f"SKIN_INFLUENCES={SoftTissueSkinning.SKIN_INFLUENCES} "
          f"CUTOFF={SoftTissueSkinning.INFLUENCE_CUTOFF_BAND}"
          f"+{SoftTissueSkinning.INFLUENCE_CUTOFF_RATIO}d "
          f"SPATIAL_LIMIT={demand_loaders.SKIN_SPATIAL_LIMIT} "
          f"MIN_SPATIAL={soft.min_spatial} FACTOR={soft.spatial_factor} "
          f"MUSCLE_W={SoftTissueSkinning.MUSCLE_FIELD_WEIGHT} "
          f"BIAS={SoftTissueSkinning.MUSCLE_WEIGHT_BIAS} "
          f"FLESH_R={SoftTissueSkinning.MUSCLE_ELIGIBILITY_RADIUS} "
          f"INWARD={SoftTissueSkinning.SEED_INWARD_ONLY} "
          f"OWNFLESH={SoftTissueSkinning.SEED_ON_OWN_FLESH} "
          f"XCOST={SoftTissueSkinning.CROSS_PART_EDGE_COST} "
          f"CUTBELOW={SoftTissueSkinning.CROSS_PART_CUT_BELOW} "
          f"HYBRID_LIMIT={SoftTissueSkinning.SPATIAL_LIMIT_ON_HYBRID} "
          f"HSCALE={SoftTissueSkinning.HYBRID_LIMIT_SCALE} "
          f"OWNED_SKIN_SEEDS={SoftTissueSkinning.SEED_FROM_OWNED_SKIN} "
          f"MARGIN={SoftTissueSkinning.SEED_CONFIDENCE_MARGIN} "
          f"BRIDGE={SoftTissueSkinning.GEODESIC_BRIDGE} "
          f"DIFFUSE_WEIGHTS={SoftTissueSkinning.DIFFUSE_WEIGHTS} "
          f"LOCALITY={SoftTissueSkinning.DIFFUSION_LOCALITY}")
    if getattr(soft, "last_bridge", None):
        print("  geodesic bridge:", soft.last_bridge)
    print(f"{'pose':26s} {'p99':>7s} {'p99.9':>8s} {'max':>9s} {'min':>7s} "
          f"{'torn':>8s} {'spikes':>7s} {'seam p99':>9s} {'bulk p99':>9s} "
          f"{'contain':>8s}")
    for title, pose in POSES.items():
        m = measure(pose, ctx, soft, b, cache)
        print(f"{title:26s} {m['p99']:7.3f} {m['p999']:8.3f} {m['max']:9.2f} "
              f"{m['min']:7.4f} {m['torn']:8d} {m['spikes']:7d} "
              f"{m['seam_p99']:9.3f} {m['bulk_p99']:9.3f} {m['containment']:8.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


```

## Source file: src/faceforge/anatomy/facs.py

{
  "path": "src/faceforge/anatomy/facs.py",
  "blob_sha": "9747d9d7d9854f4f066d85065d67f4391c12f001",
  "sha256": "08b2680df1b1bc7cfadfd31dabe3b6f08ececc610db72bf0ef2c8bd17d01a9f7",
  "lines": 179,
  "bytes": 5954,
  "selection_score": 18,
  "keyword_line_counts": {
    "eye": 2,
    "jaw": 10,
    "wrinkle": 1,
    "facs": 4,
    "rig": 6,
    "face": 19,
    "lip": 15
  },
  "url": "https://github.com/gddickinson/FaceForge/blob/cab98987a6df9b8b2ec6347bde0fc00996b690de/src/faceforge/anatomy/facs.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def __init__("
    },
    {
      "line": 54,
      "text": "def _init_region_indices(self) -> None:"
    },
    {
      "line": 56,
      "text": "def _merge(*region_names: str) -> NDArray[np.intp]:"
    },
    {
      "line": 75,
      "text": "def apply(self, face_state: FaceState) -> float:"
    },
    {
      "line": 175,
      "text": "def reset(self) -> None:"
    }
  ]
}

```text

"""FACS (Facial Action Coding System) engine -- applies AU displacements to face mesh vertices.

Ported from the ``applyFACS()`` function in faceforge-muscles.html.
Each Action Unit affects specific vertex regions loaded from ``face_regions.json``.
Displacement vectors are accumulated on top of rest positions and written into
the face mesh geometry in-place.

This module has ZERO GL imports; all vertex math is done with NumPy.
"""

from __future__ import annotations

from typing import Callable, Optional

import numpy as np
from numpy.typing import NDArray

from faceforge.core.mesh import MeshInstance
from faceforge.core.state import FaceState
from faceforge.constants import FACE_VERT_COUNT


class FACSEngine:
    """Apply FACS Action Unit displacements to a face mesh.

    All region loops are vectorized with NumPy fancy indexing for performance.
    """

    def __init__(
        self,
        face_mesh: MeshInstance,
        face_regions: dict[str, list[int]],
        jaw_pivot_angle_callback: Optional[Callable[[float], None]] = None,
    ) -> None:
        self._mesh = face_mesh
        self._regions = face_regions
        self._jaw_cb = jaw_pivot_angle_callback

        if face_mesh.rest_positions is None:
            face_mesh.store_rest_pose()

        self._rest: NDArray[np.float32] = face_mesh.rest_positions.copy()
        self._vert_count = face_mesh.geometry.vertex_count

        rest_2d = self._rest.reshape(-1, 3)
        self._x_sign = np.sign(rest_2d[:, 0]).astype(np.float32)

        # Pre-compute merged index arrays for each AU (avoids per-frame region lookups)
        self._au_indices: dict[str, NDArray[np.intp]] = {}
        self._init_region_indices()

        self._last_au_hash: Optional[int] = None

    def _init_region_indices(self) -> None:
        """Pre-merge region vertex indices into NumPy arrays per AU group."""
        def _merge(*region_names: str) -> NDArray[np.intp]:
            all_idx = []
            for rn in region_names:
                idx = self._regions.get(rn, [])
                all_idx.extend(i for i in idx if i < self._vert_count)
            return np.array(all_idx, dtype=np.intp) if all_idx else np.array([], dtype=np.intp)

        self._au_indices["brow_inner"] = _merge("leftBrowInner", "rightBrowInner")
        self._au_indices["brow_outer"] = _merge("leftBrowOuter", "rightBrowOuter")
        self._au_indices["brow_all"] = _merge("leftBrowInner", "rightBrowInner",
                                               "leftBrowOuter", "rightBrowOuter")
        self._au_indices["eye_upper"] = _merge("leftEyeUpper", "rightEyeUpper")
        self._au_indices["cheek"] = _merge("leftCheek", "rightCheek")
        self._au_indices["nose"] = _merge("noseBridge", "noseBottom")
        self._au_indices["lips"] = _merge("upperLip", "lowerLip")
        self._au_indices["upper_lip"] = _merge("upperLip")
        self._au_indices["lower_lip"] = _merge("lowerLip")
        self._au_indices["jaw"] = _merge("jawLine", "chin", "lowerLip")

    def apply(self, face_state: FaceState) -> float:
        """Apply all AU displacements and return the jaw angle in radians."""
        au = face_state.get_au_dict()

        au_hash = hash(tuple(sorted(au.items())))
        if au_hash == self._last_au_hash:
            return au.get("AU26", 0.0) * 0.28 + au.get("AU25", 0.0) * 0.06
        self._last_au_hash = au_hash

        pos = self._rest.copy()
        p = pos.reshape(-1, 3)
        xs = self._x_sign
        idx = self._au_indices

        # AU1: Inner Brow Raise
        v = au.get("AU1", 0.0)
        if v > 0:
            ix = idx["brow_inner"]
            p[ix, 1] += 0.8 * v

        # AU2: Outer Brow Raise
        v = au.get("AU2", 0.0)
        if v > 0:
            ix = idx["brow_outer"]
            p[ix, 1] += 0.6 * v

        # AU4: Brow Lower + inward pull
        v = au.get("AU4", 0.0)
        if v > 0:
            ix = idx["brow_all"]
            p[ix, 1] -= 0.5 * v
            p[ix, 0] -= 0.1 * v * xs[ix]

        # AU5: Upper Lid Raise
        v = au.get("AU5", 0.0)
        if v > 0:
            ix = idx["eye_upper"]
            p[ix, 1] += 0.4 * v

        # AU6: Cheek Raise
        v = au.get("AU6", 0.0)
        if v > 0:
            ix = idx["cheek"]
            p[ix, 1] += 0.3 * v
            p[ix, 2] -= 0.1 * v

        # AU9: Nose Wrinkle
        v = au.get("AU9", 0.0)
        if v > 0:
            ix = idx["nose"]
            p[ix, 1] += 0.25 * v
            p[ix, 2] -= 0.1 * v

        # AU12: Lip Corner Pull (smile)
        v = au.get("AU12", 0.0)
        if v > 0:
            ix = idx["lips"]
            p[ix, 1] += 0.5 * v
            p[ix, 0] += 0.15 * v * xs[ix]

        # AU15: Lip Corner Drop (frown)
        v = au.get("AU15", 0.0)
        if v > 0:
            ix = idx["lips"]
            p[ix, 1] -= 0.4 * v

        # AU20: Lip Stretch
        v = au.get("AU20", 0.0)
        if v > 0:
            ix = idx["lips"]
            p[ix, 0] += 0.3 * v * xs[ix]

        # AU22: Lip Funneler
        v = au.get("AU22", 0.0)
        if v > 0:
            ix = idx["lips"]
            p[ix, 2] += 0.4 * v
            p[ix, 0] -= 0.15 * v * xs[ix]

        # AU25: Lips Part
        v = au.get("AU25", 0.0)
        if v > 0:
            p[idx["upper_lip"], 1] += 0.15 * v
            p[idx["lower_lip"], 1] -= 0.15 * v

        # AU26: Jaw Drop
        au26 = au.get("AU26", 0.0)
        au25 = au.get("AU25", 0.0)
        jaw_angle_rad = au26 * 0.28 + au25 * 0.06
        if au26 > 0:
            p[idx["jaw"], 1] -= 0.8 * au26

        self._mesh.geometry.positions[:] = pos
        self._mesh.needs_update = True

        if self._jaw_cb is not None:
            self._jaw_cb(jaw_angle_rad)

        return jaw_angle_rad

    def reset(self) -> None:
        """Reset face mesh to rest positions."""
        self._mesh.geometry.positions[:] = self._rest
        self._mesh.needs_update = True
        self._last_au_hash = None


```

## Source file: src/faceforge/body/skin_morph.py

{
  "path": "src/faceforge/body/skin_morph.py",
  "blob_sha": "e14c2cbd98ef008e22a62fec00f232e5b7731613",
  "sha256": "af633595265c592a70e19b30c56ec50e0ff862e1da61a3f3763f116baf5e9fe3",
  "lines": 259,
  "bytes": 12120,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 11,
    "rig": 1,
    "face": 12,
    "lip": 1
  },
  "url": "https://github.com/gddickinson/FaceForge/blob/cab98987a6df9b8b2ec6347bde0fc00996b690de/src/faceforge/body/skin_morph.py",
  "function_signature_candidates": [
    {
      "line": 86,
      "text": "def radial_component(points: NDArray, delta: NDArray,"
    },
    {
      "line": 118,
      "text": "def __init__(self, source_points: NDArray, source_delta: NDArray) -> None:"
    },
    {
      "line": 126,
      "text": "def from_pair(cls, male: NDArray, female: NDArray,"
    },
    {
      "line": 151,
      "text": "def delta_for(self, points: NDArray) -> NDArray:"
    },
    {
      "line": 177,
      "text": "def smooth_on_mesh(field: NDArray, indices: Optional[NDArray], n_vertices: int,"
    },
    {
      "line": 200,
      "text": "def constrain(field: NDArray, rest: NDArray, indices: Optional[NDArray]) -> NDArray:"
    },
    {
      "line": 222,
      "text": "def apply(self, bindings: Iterable[Any], gender: float,"
    },
    {
      "line": 236,
      "text": "def _apply_one(self, binding: Any, mesh: Any, gender: float) -> bool:"
    }
  ]
}

```text

"""Carry the sex difference in soft tissue onto the model's own skin.

The skeleton morph changes the bones and the skinning carries the skin with
them, but bone is only half of what differs between the sexes on the surface.
The other half is where the soft tissue sits: breast, gluteal and thigh fat,
and a waist that is narrower relative to the hip.  None of that follows from a
scaled skeleton, and without it the morph reads as a change of size rather
than a change of sex.

The difference is not invented here.  The application already loads a matched
male/female pair of body-surface meshes, and once both are warped onto the
skeleton the vector between them is a measured sex difference with a shared
topology.  Most of that vector is the stature difference, which the skeleton
already produces; applying it again would shrink the body twice.  So two
things are removed from it:

* the **uniform size change** -- the female mesh is rescaled to the male's
  stature before the difference is taken, leaving shape only;
* the **tangential component** -- only the part of the residual that runs
  radially out from the body's own vertical axis is kept.  Radial displacement
  is exactly "more or less tissue here", which is what a fat distribution is,
  and it cannot change a limb's length or a body's height.

Measured on the shipped meshes, the residual that survives is 0.8 units over
the head (there is essentially no soft-tissue sex difference there, which is
the right answer and a useful check) against 4.2 at the chest, 5.2 at the
waist and 4.5 over the hip and buttock.
"""

from __future__ import annotations

import logging
from typing import Any, Iterable, Optional

import numpy as np
from numpy.typing import NDArray

from faceforge.body.edge_relaxation import enforce_edge_range

logger = logging.getLogger(__name__)

#: Neighbours used to interpolate the field onto a target mesh.
FIELD_K = 24

#: Width of the interpolation kernel, in body units.  The source mesh's own
#: vertex spacing is about 1, so a kernel several times that averages over a
#: patch rather than latching onto whichever single source vertex happens to be
#: nearest -- which is what made the field jump.
FIELD_SIGMA = 3.0

#: The field is trusted in full within ``FIELD_TRUST_NEAR`` of the source
#: surface and fades to nothing by ``FIELD_TRUST_FAR``.  The two surfaces are
#: different bodies: the model's skin is a median 2.8 units from the surface
#: the field was measured on, 9.3 at the 95th percentile and 16.4 at worst.
#: A hard cut-off there is what produced the visible shards -- 14606 vertices
#: fell outside it and received nothing while their neighbours received the
#: full field, so the mesh tore along the boundary.
FIELD_TRUST_NEAR = 6.0
FIELD_TRUST_FAR = 14.0

#: Laplacian passes over the *target* mesh's own edges.  The source surface and
#: the target surface are different bodies, so near a boundary -- the armpit
#: above all -- neighbouring target vertices can draw on source vertices from
#: different limbs and receive very different vectors.  Measured before
#: smoothing: the field jumped by up to 8.0 units across a skin edge whose
#: median length is 0.26, which tore 15372 edges to under half their length.
FIELD_SMOOTH_ITER = 6
FIELD_SMOOTH_STRENGTH = 0.5

#: Laplacian passes over the *source* mesh before the field is interpolated.
#: This is where the heavy smoothing belongs: the source has 10.5 k vertices
#: about 1 unit apart, so 60 passes reach several units, while the same reach
#: on the 790 k-vertex skin (0.26-unit edges) would need hundreds.  A fat
#: distribution is smooth by nature, so nothing real is lost.
SOURCE_SMOOTH_ITER = 60

#: The field is finally held inside an edge-length band on the target mesh, so
#: that however the two surfaces disagree the skin cannot be torn.  Smoothing
#: alone got the worst edges from 0.006 to 0.019 of their rest length; the
#: constraint is what makes "no tearing" a property rather than a hope.
FIELD_MAX_STRETCH = 0.25
FIELD_MAX_COMPRESSION = 0.25
FIELD_CONSTRAINT_SWEEPS = 20


def radial_component(points: NDArray, delta: NDArray,
                     axis_xy: tuple[float, float] | None = None) -> NDArray:
    """Keep only the part of ``delta`` pointing away from the body's long axis.

    The body is star-shaped about a vertical axis, so the outward direction at
    a vertex is well defined without needing a surface normal -- which matters
    here because the triangle winding of both surface meshes is inconsistent
    and their normals cannot be trusted.
    """
    pts = np.asarray(points, dtype=np.float64)
    d = np.asarray(delta, dtype=np.float64)
    if axis_xy is None:
        axis_xy = (float(np.median(pts[:, 0])), float(np.median(pts[:, 1])))
    out = pts[:, :2] - np.asarray(axis_xy, dtype=np.float64)
    norm = np.linalg.norm(out, axis=1, keepdims=True)
    unit = np.divide(out, np.maximum(norm, 1e-9))
    radial = np.einsum("ij,ij->i", d[:, :2], unit)
    result = np.zeros_like(d)
    result[:, :2] = unit * radial[:, None]
    return result


class SkinShapeMorph:
    """A measured soft-tissue sex field, interpolated onto any mesh.

    Parameters
    ----------
    source_points, source_delta :
        The male surface and the shape-only displacement to the female one,
        both ``(N, 3)`` in body coordinates.
    """

    def __init__(self, source_points: NDArray, source_delta: NDArray) -> None:
        self._pts = np.asarray(source_points, dtype=np.float64)
        self._delta = np.asarray(source_delta, dtype=np.float64)
        self._tree = None
        self._rest: dict[int, NDArray[np.float32]] = {}
        self._field: dict[int, NDArray[np.float64]] = {}

    @classmethod
    def from_pair(cls, male: NDArray, female: NDArray,
                  indices: Optional[NDArray] = None) -> "SkinShapeMorph":
        """Build the field from a matched male/female surface pair.

        ``indices`` are the pair's shared triangles; given them, the field is
        smoothed on the source before it is ever interpolated.
        """
        m = np.asarray(male, dtype=np.float64)
        f = np.asarray(female, dtype=np.float64)
        hm = float(m[:, 2].max() - m[:, 2].min())
        hf = float(f[:, 2].max() - f[:, 2].min())
        scaled = f.copy()
        if hf > 1e-6:
            scaled[:, 2] -= f[:, 2].min()
            scaled *= hm / hf
            scaled[:, 2] += m[:, 2].min()
        for ax in (0, 1):
            scaled[:, ax] -= float(np.median(scaled[:, ax]) - np.median(m[:, ax]))
        delta = radial_component(m, scaled - m)
        delta = cls.smooth_on_mesh(delta, indices, len(m),
                                   iterations=SOURCE_SMOOTH_ITER)
        return cls(m, delta)

    # -- field ---------------------------------------------------------------

    def delta_for(self, points: NDArray) -> NDArray:
        """The field sampled at arbitrary points, by inverse-distance weighting."""
        try:
            from scipy.spatial import cKDTree
        except ImportError:                                   # pragma: no cover
            return np.zeros((len(points), 3))
        if self._tree is None:
            self._tree = cKDTree(self._pts)
        k = min(FIELD_K, len(self._pts))
        dist, idx = self._tree.query(np.asarray(points, dtype=np.float64), k=k)
        if dist.ndim == 1:
            dist = dist[:, None]
            idx = idx[:, None]
        w = np.exp(-(dist / FIELD_SIGMA) ** 2)
        total = w.sum(axis=1, keepdims=True)
        w = np.divide(w, np.maximum(total, 1e-12))
        out = np.einsum("qk,qkj->qj", w, self._delta[idx])
        # Confidence from the nearest source vertex, faded rather than cut.
        nearest = dist.min(axis=1)
        span = max(FIELD_TRUST_FAR - FIELD_TRUST_NEAR, 1e-9)
        trust = np.clip((FIELD_TRUST_FAR - nearest) / span, 0.0, 1.0)
        # Smoothstep, so the weight's own derivative is continuous too.
        trust = trust * trust * (3.0 - 2.0 * trust)
        return out * trust[:, None]

    @staticmethod
    def smooth_on_mesh(field: NDArray, indices: Optional[NDArray], n_vertices: int,
                       iterations: int = FIELD_SMOOTH_ITER,
                       strength: float = FIELD_SMOOTH_STRENGTH) -> NDArray:
        """Laplacian-smooth a displacement field over a mesh's own edges."""
        if indices is None or iterations <= 0:
            return field
        tris = np.asarray(indices).reshape(-1, 3)
        e = np.concatenate([tris[:, [0, 1]], tris[:, [1, 2]], tris[:, [2, 0]]])
        src = np.concatenate([e[:, 0], e[:, 1]])
        dst = np.concatenate([e[:, 1], e[:, 0]])
        counts = np.bincount(src, minlength=n_vertices).astype(np.float64)
        has = counts > 0
        inv = np.zeros(n_vertices, dtype=np.float64)
        inv[has] = 1.0 / counts[has]
        out = np.array(field, dtype=np.float64, copy=True)
        for _ in range(iterations):
            avg = np.empty_like(out)
            for c in range(out.shape[1]):
                avg[:, c] = np.bincount(src, weights=out[dst, c], minlength=n_vertices) * inv
            out[has] = (1.0 - strength) * out[has] + strength * avg[has]
        return out

    @staticmethod
    def constrain(field: NDArray, rest: NDArray, indices: Optional[NDArray]) -> NDArray:
        """Hold the displaced mesh inside an edge-length band around its rest pose."""
        if indices is None:
            return field
        tris = np.asarray(indices).reshape(-1, 3)
        edges = np.unique(np.sort(np.concatenate(
            [tris[:, [0, 1]], tris[:, [1, 2]], tris[:, [2, 0]]]), axis=1), axis=0)
        rest_len = np.linalg.norm(rest[edges[:, 0]] - rest[edges[:, 1]], axis=1)
        keep = rest_len > 1e-6
        edges, rest_len = edges[keep], rest_len[keep]
        pos = rest + field
        report = enforce_edge_range(
            pos, edges, rest_len,
            max_stretch=FIELD_MAX_STRETCH, max_compression=FIELD_MAX_COMPRESSION,
            iterations=FIELD_CONSTRAINT_SWEEPS)
        logger.info("Skin field constrained: %s in %d sweeps",
                    "converged" if report["converged"] else "residual remains",
                    report["iterations_run"])
        return pos - rest

    # -- application ---------------------------------------------------------

    def apply(self, bindings: Iterable[Any], gender: float,
              name_filter: Optional[str] = "skin") -> int:
        """Displace the rest pose of every matching mesh by ``gender`` x the field."""
        n = 0
        for binding in bindings:
            mesh = getattr(binding, "mesh", None)
            if mesh is None or getattr(binding, "is_muscle", False):
                continue
            if name_filter and name_filter not in (mesh.name or "").lower():
                continue
            if self._apply_one(binding, mesh, gender):
                n += 1
        return n

    def _apply_one(self, binding: Any, mesh: Any, gender: float) -> bool:
        if mesh.rest_positions is None:
            return False
        key = id(mesh)
        rest = self._rest.get(key)
        if rest is None:
            rest = self._rest[key] = np.asarray(mesh.rest_positions, dtype=np.float32).copy()
        field = self._field.get(key)
        if field is None:
            pts = rest.reshape(-1, 3).astype(np.float64)
            field = self.delta_for(pts)
            field = self.smooth_on_mesh(field, mesh.geometry.indices, len(pts))
            field = self.constrain(field, pts, mesh.geometry.indices)
            self._field[key] = field
            logger.info("Skin sex field on %s: median %.2f, max %.2f over %d vertices",
                        mesh.name, float(np.median(np.linalg.norm(field, axis=1))),
                        float(np.linalg.norm(field, axis=1).max()), len(field))
        g = float(max(0.0, min(1.0, gender)))
        out = (rest.reshape(-1, 3).astype(np.float64) + field * g)
        mesh.rest_positions = out.reshape(-1).astype(np.float32)
        binding._rest_f64 = None
        binding._pos_h = None
        mesh.needs_update = True
        return True


```

## Source file: src/faceforge/body/skinning_ops.py

{
  "path": "src/faceforge/body/skinning_ops.py",
  "blob_sha": "a66ab0be69180d734dda513f6725d303cf06607f",
  "sha256": "a4d7212afc1d63463e49a67f3ac535bd0429b1c06595ce2c43f08402e120c68c",
  "lines": 63,
  "bytes": 2994,
  "selection_score": 18,
  "keyword_line_counts": {
    "skin": 2,
    "face": 1
  },
  "url": "https://github.com/gddickinson/FaceForge/blob/cab98987a6df9b8b2ec6347bde0fc00996b690de/src/faceforge/body/skinning_ops.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def transform_points(delta_stack: np.ndarray, joint_idx: np.ndarray,"
    },
    {
      "line": 37,
      "text": "def rotate_vectors(delta_stack: np.ndarray, joint_idx: np.ndarray,"
    },
    {
      "line": 45,
      "text": "def used_joints(joint_idx: np.ndarray, secondary_idx: np.ndarray | None,"
    },
    {
      "line": 54,
      "text": "def accumulate_rows(index: np.ndarray, values: np.ndarray, n: int) -> np.ndarray:"
    }
  ]
}

```text

"""Vectorised per-vertex operations shared by the skinning passes.

``transform_points`` / ``rotate_vectors`` are the gathered einsum the
skinning has always used, kept in one place so every pass computes a
joint's image of a vertex the same way.  Measured alternatives, 1 million
vertices, float64 (the numbers that decided this):

    joints used   gathered einsum   per-joint masked matmuls   one wide matmul + pick
        1              37 ms                12 ms                     33 ms
        2              35 ms                49 ms                     39 ms
        4              34 ms                56 ms                     38 ms
       10              36 ms                99 ms                     67 ms

The (V, 4, 4) gather looks wasteful (128 bytes per vertex) but numpy streams
it at memory bandwidth; grouping vertices by joint costs a mask pass and a
scattered write per joint and loses from two joints up.

``accumulate_rows`` replaces ``np.add.at`` (an unbuffered loop) with one
``np.bincount`` per column: the same sums, an order of magnitude faster
(0.92 s -> 0.31 s per frame for the face-normal accumulation of the muscles
that had a positional pass).
"""

from __future__ import annotations

import numpy as np


def transform_points(delta_stack: np.ndarray, joint_idx: np.ndarray,
                     points_h: np.ndarray) -> np.ndarray:
    """``(delta[joint_idx[v]] @ points_h[v])[:3]`` for every vertex; fresh ``(V, 3)`` float64."""
    if len(points_h) == 0:
        return np.empty((0, 3), dtype=np.float64)
    return np.einsum('vij,vj->vi', delta_stack[joint_idx], points_h)[:, :3]


def rotate_vectors(delta_stack: np.ndarray, joint_idx: np.ndarray,
                   vectors: np.ndarray) -> np.ndarray:
    """``delta[joint_idx[v]][:3, :3] @ vectors[v]`` for every vertex; fresh ``(V, 3)``."""
    if len(vectors) == 0:
        return np.empty((0, 3), dtype=np.float64)
    return np.einsum('vij,vj->vi', delta_stack[joint_idx, :3, :3], vectors)


def used_joints(joint_idx: np.ndarray, secondary_idx: np.ndarray | None,
                n_joints: int) -> np.ndarray:
    """Sorted joint indices that drive at least one vertex (bincount, O(V), no sort)."""
    counts = np.bincount(np.asarray(joint_idx).ravel(), minlength=n_joints)
    if secondary_idx is not None:
        counts = counts + np.bincount(np.asarray(secondary_idx).ravel(), minlength=n_joints)
    return np.flatnonzero(counts[:n_joints])


def accumulate_rows(index: np.ndarray, values: np.ndarray, n: int) -> np.ndarray:
    """Sum the rows of ``values`` into ``n`` bins by ``index`` (``np.add.at`` semantics)."""
    index = np.asarray(index).ravel()
    values = np.asarray(values, dtype=np.float64)
    if values.ndim == 1:
        return np.bincount(index, weights=values, minlength=n)[:n]
    out = np.empty((n, values.shape[1]), dtype=np.float64)
    for c in range(values.shape[1]):
        out[:, c] = np.bincount(index, weights=values[:, c], minlength=n)[:n]
    return out


```

# Facial rendering source scan: matbeedotcom/affect-avatar-toolchain

Repository: https://github.com/matbeedotcom/affect-avatar-toolchain

Pinned source revision: e393b7f9b3a560c6f6705658715a8bcf8885365c

Discovery topic: ARKit blendshape

Description: Audio -> ARKit-52 Blendshape latent diffusion model WIP

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["docs/references/audio-blendshape-diffusion/PROJECT_PLAN.md", "docs/references/audio-blendshape-diffusion/README.md", "docs/references/audio-blendshape-diffusion/notes/README.md", "docs/references/audio-blendshape-diffusion/notes/spike-b0-blendshape-extraction.md", "tools/affect_avatar/artifacts/blendshape_dit.csv", "tools/affect_avatar/artifacts/blendshape_dit.report.md", "tools/affect_avatar/artifacts/blendshape_vae.csv", "tools/affect_avatar/artifacts/blendshape_vae.pt", "tools/affect_avatar/artifacts/blendshape_vae.report.md", "tools/affect_avatar/scripts/lib/blendshape_extractor.py", "tools/affect_avatar/scripts/lib/vad_to_blendshape.py"], "face": ["tools/affect_coach/bake_affect_face.py"]}

## Source file: tools/affect_coach/bake_affect_face.py

{
  "path": "tools/affect_coach/bake_affect_face.py",
  "blob_sha": "20d1b53cfbe820ac422615e1cca70fee28556d76",
  "sha256": "eda7f16783dfd6696a28033894390cb8f9869ea284a1a686130b64d93abaf8dc",
  "lines": 337,
  "bytes": 11822,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 13,
    "gum": 9,
    "face": 19,
    "lip": 1
  },
  "url": "https://github.com/matbeedotcom/affect-avatar-toolchain/blob/e393b7f9b3a560c6f6705658715a8bcf8885365c/tools/affect_coach/bake_affect_face.py",
  "function_signature_candidates": [
    {
      "line": 99,
      "text": "def ensure_trace(simulator_scenario: str, *, force: bool = False) -> Path:"
    },
    {
      "line": 138,
      "text": "def bake_frames("
    },
    {
      "line": 179,
      "text": "def write_jsonl("
    },
    {
      "line": 196,
      "text": "def diagnose("
    },
    {
      "line": 222,
      "text": "def main() -> int:"
    }
  ]
}

```text

#!/usr/bin/env python3
"""Bake a per-tick ARKit-52 blendshape JSONL from an affect scenario.

Companion to [`coach.py`](coach.py): runs the same affect simulator
the coach uses (via the workspace's `affect-simulator` Rust binary),
maps every emitted frame to the same ARKit-52 weights the live
`AffectSimulatorNode` produces (Python mirror of
`crates/core/src/nodes/affect_expression.rs`), and dumps one
canonical blendshape envelope per line:

```json
{"kind":"blendshapes","arkit_52":[…52 floats…],"pts_ms":0}
{"kind":"blendshapes","arkit_52":[…],"pts_ms":200}
…
```

This is the same wire shape every renderer in the workspace already
consumes — `Audio2FaceLipSyncNode`, `SyntheticLipSyncNode`,
`MotionPlayerNode` (poses), and the running
`Live2DRenderNode` / `CcRenderNode`. The JSONL is **the** thing to
hand to a renderer downstream; pipe it into a `BlendshapePlayer`-style
node when one exists, or render it via the future
`render_affect_face.sh` wrapper.

## Why an offline baker, not a live render

The simulator runs at 5 Hz, so a 30 s scenario produces ~150 frames —
small enough to bake to disk in a fraction of a second, big enough
that re-running the cargo `affect-simulator` binary on every render
iteration would be wasted. Baking once, rendering many times also
makes it trivial to inspect: `head -5 *.affect_face.jsonl` shows
exactly what the avatar would do, and `--diagnose` prints the
trajectory of top-active blendshapes per frame so you can sanity-
check the mapping before paying for an actual render.

## Usage

```bash
# Bake one of the simulator scenarios:
./tools/affect_coach/bake_affect_face.py --scenario warm_admiration \
    --out out/warm_admiration.affect_face.jsonl

# Free-form input (uses coach.py's keyword classifier):
./tools/affect_coach/bake_affect_face.py \
    --coach-input "I just got fired" \
    --out out/honest_concern.affect_face.jsonl

# Inspect what the face would look like over time, no render:
./tools/affect_coach/bake_affect_face.py --scenario roast_invitation --diagnose

# Override the renderer's frame rate for the JSONL (default 30 fps;
# we resample the 5 Hz simulator trace by holding the most recent
# frame, matching `MotionPlayerNode`'s `pace_realtime` semantics).
./tools/affect_coach/bake_affect_face.py --scenario warm_admiration --fps 60 \
    --out out/wa.affect_face.jsonl
```

## Resampling

The simulator emits at its own `tick_ms` (200 ms by default). Avatar
renderers typically run at 30 fps. We hold-and-emit so a 5 Hz tick
becomes 6 identical 30 Hz frames — same behaviour the runtime would
exhibit if you wired `affect_sim → BlendshapePlayer → CcRender` over
the live tick stream. `--fps` controls the output rate.
"""

from __future__ import annotations

import argparse
import json
import logging
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]
SCENARIO_DIR = REPO_ROOT / "tools/affect_simulator/scenarios"

# Reuse coach.py's mapping mirror (kept in sync with the Rust impl
# via the index-table tests on the Rust side). Keeping a single
# source of truth in the coach module avoids the inevitable drift
# between two near-identical hand-tuned tables.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from coach import (  # noqa: E402
    SCENARIOS,
    classify_scenario,
    compute_blendshapes,
    ARKIT_NAMES,
)

logging.basicConfig(
    level=logging.WARNING,
    format="%(asctime)s %(levelname)s %(name)s %(message)s",
)
logger = logging.getLogger("bake_affect_face")


def ensure_trace(simulator_scenario: str, *, force: bool = False) -> Path:
    """Build (or reuse) the trace JSON next to the scenario file.

    Mirrors `coach.ensure_trace`; duplicated here to keep the baker
    runnable even if `coach.py` evolves to load the LLM eagerly.
    Cache invalidates when either the scenario JSON or any simulator
    source file is newer than the trace.
    """
    scenario_path = SCENARIO_DIR / f"{simulator_scenario}.json"
    if not scenario_path.exists():
        raise FileNotFoundError(
            f"simulator scenario not found: {scenario_path}\n"
            f"Available: {sorted(p.stem for p in SCENARIO_DIR.glob('*.json'))}"
        )
    trace_path = scenario_path.with_suffix(".trace.json")
    sim_src_dir = REPO_ROOT / "tools/affect_simulator/src"
    sim_src_mtime = max(
        (p.stat().st_mtime for p in sim_src_dir.rglob("*.rs")),
        default=0.0,
    )
    if trace_path.exists() and not force:
        trace_mtime = trace_path.stat().st_mtime
        if (
            trace_mtime > scenario_path.stat().st_mtime
            and trace_mtime > sim_src_mtime
        ):
            return trace_path
    logger.info("rebuilding trace for %s", scenario_path.name)
    subprocess.run(
        [
            "cargo", "run", "-p", "affect-simulator", "--quiet", "--",
            "run", str(scenario_path), "--out", str(trace_path),
        ],
        cwd=str(REPO_ROOT),
        check=True,
    )
    return trace_path


def bake_frames(
    trace: Dict,
    *,
    out_fps: int = 30,
    gain: float = 1.0,
) -> List[Tuple[int, List[float]]]:
    """Produce `(pts_ms, arkit_52)` pairs at the requested output rate.

    Hold-and-resample: the simulator emits at `trace["tick_ms"]`; we
    fill in the gaps by repeating the most recent simulator frame at
    the renderer rate. Same shape `MotionPlayerNode` would yield if
    you piped the simulator's stream through it at `pace_realtime=true`.
    """
    sim_tick_ms = int(trace.get("tick_ms", 200))
    frames = trace.get("frames", [])
    if not frames:
        return []

    # Sort just in case; simulator runner already emits in order.
    frames = sorted(frames, key=lambda f: int(f.get("timestamp_ms", 0)))
    last_ts = int(frames[-1]["timestamp_ms"])
    duration_ms = max(last_ts + sim_tick_ms, 0)

    out_period_ms = max(1, int(round(1000.0 / max(1, out_fps))))
    out: List[Tuple[int, List[float]]] = []
    sim_idx = 0
    t = 0
    while t <= duration_ms:
        # Advance the sim cursor while the next sim frame's timestamp
        # has already passed `t`. This is a step-and-hold sampler.
        while (
            sim_idx + 1 < len(frames)
            and int(frames[sim_idx + 1]["timestamp_ms"]) <= t
        ):
            sim_idx += 1
        weights = compute_blendshapes(frames[sim_idx], gain=gain)
        out.append((t, weights))
        t += out_period_ms
    return out


def write_jsonl(
    path: Path,
    samples: List[Tuple[int, List[float]]],
) -> None:
    """One blendshape envelope per line, canonical `{kind,arkit_52,pts_ms}`."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w") as f:
        for pts_ms, weights in samples:
            obj = {
                "kind": "blendshapes",
                "arkit_52": [round(w, 4) for w in weights],
                "pts_ms": int(pts_ms),
            }
            f.write(json.dumps(obj))
            f.write("\n")


def diagnose(
    samples: List[Tuple[int, List[float]]],
    *,
    every_ms: int = 200,
    top_k: int = 3,
    threshold: float = 0.05,
) -> None:
    """Print top-active blendshapes at a coarse cadence so a human can
    eyeball the trajectory without opening the JSONL."""
    last_dump = -10**9
    for pts_ms, weights in samples:
        if pts_ms - last_dump < every_ms:
            continue
        last_dump = pts_ms
        active = sorted(
            ((i, w) for i, w in enumerate(weights) if w >= threshold),
            key=lambda iw: iw[1],
            reverse=True,
        )[:top_k]
        if active:
            face = ", ".join(f"{ARKIT_NAMES[i]} {w:.2f}" for i, w in active)
        else:
            face = "(neutral)"
        print(f"  t={pts_ms:>5}ms  {face}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--scenario",
        help=f"Affect scenario name. One of: "
             f"{', '.join(s.name for s in SCENARIOS)}",
    )
    parser.add_argument(
        "--coach-input",
        help="Free-form line; coach.py's keyword classifier picks the "
             "scenario. Mutually exclusive with --scenario.",
    )
    parser.add_argument(
        "--out", type=Path,
        help="Output `.affect_face.jsonl` path. If omitted, writes to "
             "out/<scenario>.affect_face.jsonl in the repo root.",
    )
    parser.add_argument(
        "--fps", type=int, default=30,
        help="Output frame rate for the JSONL (default 30; the simulator "
             "ticks at 5 Hz, hold-and-repeat resamples to this rate).",
    )
    parser.add_argument(
        "--gain", type=float, default=1.0,
        help="Post-expressiveness gain on the affect→ARKit mapping. "
             "1.0 = baseline; raise for a more theatrical face, lower "
             "for muted readings (default 1.0).",
    )
    parser.add_argument(
        "--diagnose", action="store_true",
        help="Print top-active blendshapes every 200 ms instead of "
             "writing a JSONL file. Useful for sanity-checking the "
             "mapping before paying for a render.",
    )
    parser.add_argument(
        "--rebuild-trace", action="store_true",
        help="Force the simulator binary to rebuild the trace JSON "
             "(skip the cache check on the scenario / source mtime).",
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true",
        help="Bump logging to INFO so simulator-rebuild lines show.",
    )
    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.INFO)

    if args.scenario and args.coach_input:
        print("ERROR: --scenario and --coach-input are mutually exclusive",
              file=sys.stderr)
        return 2

    # Resolve the simulator scenario.
    if args.coach_input:
        scenario = classify_scenario(args.coach_input)
        print(f"  classifier picked: {scenario.name} ({scenario.simulator})")
    else:
        scenario_name = args.scenario or "warm_admiration"
        scenario = next(
            (s for s in SCENARIOS if s.name == scenario_name),
            None,
        )
        if scenario is None:
            print(
                f"ERROR: unknown scenario {scenario_name!r}. Available: "
                f"{sorted(s.name for s in SCENARIOS)}",
                file=sys.stderr,
            )
            return 2

    # Build the trace + bake the frames.
    try:
        trace_path = ensure_trace(scenario.simulator, force=args.rebuild_trace)
        trace = json.loads(trace_path.read_text())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: simulator trace failed: {exc}", file=sys.stderr)
        return 1

    samples = bake_frames(trace, out_fps=args.fps, gain=args.gain)
    if not samples:
        print("ERROR: trace produced zero frames", file=sys.stderr)
        return 1

    duration_s = samples[-1][0] / 1000.0

    if args.diagnose:
        print(
            f"  scenario:  {scenario.name} ({scenario.simulator})\n"
            f"  duration:  {duration_s:.1f}s @ {args.fps} fps "
            f"({len(samples)} frames)\n"
            f"  trajectory:"
        )
        diagnose(samples)
        return 0

    out_path: Path
    if args.out is None:
        out_path = REPO_ROOT / "out" / f"{scenario.name}.affect_face.jsonl"
    else:
        out_path = args.out
    write_jsonl(out_path, samples)
    print(
        f"✓ wrote {out_path}\n"
        f"  scenario:  {scenario.name} ({scenario.simulator})\n"
        f"  duration:  {duration_s:.1f}s @ {args.fps} fps "
        f"({len(samples)} frames)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())


```

## Source file: tools/affect_avatar/scripts/lib/vad_to_blendshape.py

{
  "path": "tools/affect_avatar/scripts/lib/vad_to_blendshape.py",
  "blob_sha": "b99caabde5c6920cd1c5ff1a16ad1eb5acc158a2",
  "sha256": "2a63f9dcf1ca4d96b4edfe2042cab67dd4844f7b46a642ad5d21dc5eaeec5939",
  "lines": 225,
  "bytes": 9116,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 15,
    "mouth": 1,
    "face": 1,
    "lip": 5
  },
  "url": "https://github.com/matbeedotcom/affect-avatar-toolchain/blob/e393b7f9b3a560c6f6705658715a8bcf8885365c/tools/affect_avatar/scripts/lib/vad_to_blendshape.py",
  "function_signature_candidates": [
    {
      "line": 90,
      "text": "def __init__("
    },
    {
      "line": 135,
      "text": "def load("
    },
    {
      "line": 159,
      "text": "def lookup(self, vad: np.ndarray, intensity: float | np.ndarray = 1.0) -> np.ndarray:"
    },
    {
      "line": 212,
      "text": "def anchor_summary(self) -> str:"
    }
  ]
}

```text

"""VAD → ARKit-52 blendshape interpolator backed by MEAD-derived anchors.

The listener-face pipeline doesn't need audio at all: given a VAD
trajectory (per-frame valence/arousal/dominance + intensity), it
interpolates between the 21 emotion×intensity anchors built from
MEAD by [`10_build_vad_anchors.py`](../10_build_vad_anchors.py) (or
the inline equivalent in `artifacts/vad_anchors_delta.npz`).

Anchors are stored as **deltas from neutral baseline**, peak-aggregated
over MEAD clips:
  - For each MEAD `(emotion, intensity)` group, take per-clip
    max-over-time blendshape, mean across clips.
  - Subtract neutral L1 anchor as a baseline (removes systematic
    boundary-frame artefacts where MediaPipe misclassifies closed
    mouths as `mouthPucker` etc.).
  - Clip to [0, 1]: a positive emotion-delta only.

Inference path:
  - User audio → affect_calibration's whisper_to_vad regressor
    → per-frame `(V, A, D)` ∈ [-1, +1]³.
  - Optional: smooth that trajectory with a small EMA (the heavy
    listener smoothing is applied AFTER blendshape lookup, since
    smoothing in VAD space distorts emotion identity less than
    smoothing on output).
  - Look up nearest emotion anchors via Gaussian RBF in VAD space;
    weighted-sum gives the 54-channel MEAD blendshape, mapped to
    52-channel ARKit, optionally listener-masked.

Output is always **MEAD-54 channels** (caller maps to ARKit-52 via
[`mead3d_to_arkit.py`](mead3d_to_arkit.py)).

Usage:
    interp = VadToBlendshape.load("artifacts/vad_anchors_delta.npz")
    bs54 = interp.lookup(np.array([0.6, 0.4, 0.5]))  # → (54,)
    bs54_seq = interp.lookup(vad_traj)                # → (T, 54) for (T, 3) input
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

import numpy as np


# Per [`emotion_to_vad.py`](emotion_to_vad.py) — same VAD anchors used
# for DiT conditioning. Reproduced here so the interpolator can be
# imported standalone without that dependency.
MEAD_VAD: Mapping[str, tuple[float, float, float]] = {
    "neutral":   (0.00, 0.00, 0.00),
    "happy":     (+0.80, +0.50, +0.40),
    "sad":       (-0.70, -0.30, -0.40),
    "angry":     (-0.50, +0.70, +0.50),
    "fear":      (-0.60, +0.60, -0.50),
    "disgusted": (-0.60, +0.30, -0.20),
    "surprised": (+0.30, +0.70,  0.00),
    "contempt":  (-0.40, -0.10, +0.50),
}


@dataclass
class VadToBlendshapeConfig:
    """Tunables for the RBF interpolator.

    - `tau`: Gaussian bandwidth in VAD space. Smaller = sharper
      transitions between anchors (more single-anchor at a time);
      larger = smoother blends. 0.25 chosen so a query near one anchor
      gets ~80% of its weight from that anchor + the rest spread.
    - `intensity_floor`: anchors below this normalized intensity get
      their delta scaled down. Stops L1 anchors from looking
      indistinguishable from rest.
    - `add_baseline`: when True, the returned blendshape is
      `baseline + emotion_delta`. When False, it's the pure delta
      (better for listener mode — no baseline pucker / squint
      bleeding through, cleaner rest pose).
    """
    tau: float = 0.25
    intensity_floor: float = 0.20
    add_baseline: bool = False


class VadToBlendshape:
    """Stateless RBF interpolator over MEAD emotion×intensity anchors.

    Caller passes in a VAD vector (or batch of them); interpolator
    returns the blended 54-channel MEAD blendshape vector(s).
    """

    def __init__(
        self,
        anchors: dict[tuple[str, int], np.ndarray],
        baseline: np.ndarray,
        cfg: VadToBlendshapeConfig | None = None,
    ) -> None:
        """`anchors`: `{(emotion_name, intensity): (54,) delta}`.
        `baseline`: `(54,)` neutral-L1 anchor used when `add_baseline=True`.
        """
        self.cfg = cfg or VadToBlendshapeConfig()

        # Stack anchors into a `(K, 54)` matrix and a parallel
        # `(K, 4)` matrix of (V, A, D, intensity_norm) for distance
        # computation. K = number of anchors; typically 21 (7 non-neutral
        # emotions × 3 intensities).
        emotions = []
        intensities = []
        deltas = []
        for (emo, lvl), d in anchors.items():
            if emo == "neutral":
                # Neutral has only one intensity (L1) and lives in the
                # baseline; skip it in the anchor pool so it doesn't
                # dominate near-rest queries.
                continue
            emotions.append(emo)
            intensities.append(int(lvl))
            deltas.append(d)
        self.anchor_emotion = list(emotions)
        self.anchor_intensity = np.array(intensities, dtype=np.int32)
        self.deltas = np.stack(deltas, axis=0).astype(np.float32)   # (K, 54)
        self.baseline = baseline.astype(np.float32)                 # (54,)

        # Anchor coordinates in 4-D conditioning space:
        # (V, A, D, intensity_norm) where intensity_norm = lvl / 3.
        coords = np.zeros((len(emotions), 4), dtype=np.float32)
        for i, (emo, lvl) in enumerate(zip(emotions, self.anchor_intensity)):
            v, a, d = MEAD_VAD.get(emo, (0.0, 0.0, 0.0))
            coords[i] = (v, a, d, lvl / 3.0)
        self.anchor_coords = coords    # (K, 4)

    # ------------------------------------------------------------------
    # Class methods
    # ------------------------------------------------------------------

    @classmethod
    def load(
        cls,
        path: Path | str,
        cfg: VadToBlendshapeConfig | None = None,
    ) -> "VadToBlendshape":
        """Load anchors from `vad_anchors_delta.npz`.

        Expected keys: `baseline` (54,), and one entry per anchor named
        `{emotion}_{intensity}` (e.g. `happy_3`, `surprised_2`).
        """
        data = np.load(str(path))
        baseline = data["baseline"]
        anchors: dict[tuple[str, int], np.ndarray] = {}
        for k in data.files:
            if k == "baseline":
                continue
            emo, lvl = k.rsplit("_", 1)
            anchors[(emo, int(lvl))] = data[k]
        return cls(anchors, baseline, cfg)

    # ------------------------------------------------------------------
    # Inference
    # ------------------------------------------------------------------

    def lookup(self, vad: np.ndarray, intensity: float | np.ndarray = 1.0) -> np.ndarray:
        """`vad`: shape `(3,)` or `(T, 3)` — V/A/D in [-1, +1].
        `intensity`: scalar or `(T,)` — emotion intensity in [0, 1].
        Returns: `(54,)` or `(T, 54)` MEAD-54 blendshape.

        For batched input, intensity may be a scalar (broadcast) or a
        per-frame array.
        """
        vad = np.asarray(vad, dtype=np.float32)
        single = vad.ndim == 1
        if single:
            vad = vad[None, :]                          # (1, 3)

        T = vad.shape[0]
        if np.isscalar(intensity):
            intensity_arr = np.full((T,), float(intensity), dtype=np.float32)
        else:
            intensity_arr = np.asarray(intensity, dtype=np.float32).reshape(T)

        # Query coords: (T, 4)
        q = np.concatenate([vad, intensity_arr[:, None]], axis=1)

        # Pairwise squared distance to anchors: (T, K)
        d2 = ((q[:, None, :] - self.anchor_coords[None, :, :]) ** 2).sum(axis=2)

        # Gaussian RBF weights, normalized per query.
        w = np.exp(-d2 / (2 * self.cfg.tau * self.cfg.tau))
        w = w / np.maximum(w.sum(axis=1, keepdims=True), 1e-8)      # (T, K)

        # Weighted sum of anchor deltas → emotion delta per query.
        emo_delta = w @ self.deltas                                  # (T, 54)

        # Intensity floor: low-intensity queries get a scaled-down
        # emotion delta so the L1 anchors don't end up identical to
        # neutral.
        if self.cfg.intensity_floor > 0:
            scale = np.maximum(intensity_arr, self.cfg.intensity_floor)
            emo_delta = emo_delta * scale[:, None]

        if self.cfg.add_baseline:
            out = self.baseline[None, :] + emo_delta
        else:
            out = emo_delta

        # Clamp to [0, 1] — blendshapes are nonnegative by convention.
        out = np.clip(out, 0.0, 1.0)

        return out[0] if single else out

    # ------------------------------------------------------------------
    # Diagnostics
    # ------------------------------------------------------------------

    def anchor_summary(self) -> str:
        """Pretty-printed (anchor, top-3 channels) for debugging."""
        # Lazy import to keep core path dependency-free.
        try:
            from .mead3d_to_arkit import MEDIAPIPE_ORDER
        except ImportError:
            from mead3d_to_arkit import MEDIAPIPE_ORDER  # type: ignore[no-redef]
        lines = []
        for i, (emo, lvl) in enumerate(zip(self.anchor_emotion, self.anchor_intensity)):
            d = self.deltas[i, :51]
            top = np.argsort(d)[::-1][:3]
            sig = ", ".join(f"{MEDIAPIPE_ORDER[t]}={d[t]:+.2f}" for t in top)
            lines.append(f"  {emo:10s} L{int(lvl)} → {sig}")
        return "\n".join(lines)


```

## Source file: tools/affect_avatar/scripts/lib/blendshape_extractor.py

{
  "path": "tools/affect_avatar/scripts/lib/blendshape_extractor.py",
  "blob_sha": "6fec23ada336a1cdfff0b5e27a7864be8e285bdd",
  "sha256": "793278aa8c20d5a117efff0b5c7ce10c7c335b8d70280ab4c35d1f1f74c6a5d4",
  "lines": 186,
  "bytes": 6442,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 1,
    "blendshape": 12,
    "mouth": 1,
    "face": 27,
    "lip": 11
  },
  "url": "https://github.com/matbeedotcom/affect-avatar-toolchain/blob/e393b7f9b3a560c6f6705658715a8bcf8885365c/tools/affect_avatar/scripts/lib/blendshape_extractor.py",
  "function_signature_candidates": [
    {
      "line": 60,
      "text": "def __init__(self, model_path: Optional[Path] = None) -> None:"
    },
    {
      "line": 77,
      "text": "def _build_landmarker(self):"
    },
    {
      "line": 91,
      "text": "def extract_clip(self, video_path: Path) -> ClipExtraction:"
    },
    {
      "line": 170,
      "text": "def close(self) -> None:"
    },
    {
      "line": 177,
      "text": "def _resolve_model_path() -> Path:"
    }
  ]
}

```text

"""MediaPipe FaceLandmarker wrapper — extracts ARKit-flavored blendshapes.

Per Spike B0 protocol
(docs/references/audio-blendshape-diffusion/notes/spike-b0-blendshape-extraction.md).

Wraps Google's MediaPipe Tasks `FaceLandmarker` so the rest of the
toolchain sees a simple `extract_clip(video_path) -> (T, K)` surface.
The K dimension is whatever set the FaceLandmarker emits (52 entries
for the standard `face_landmarker.task` model), in the model's native
order; callers do their own remapping into ARKit-52 order if they need
strict ARKit semantics.

Model file:
    https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task

Pass `--model-path` to the spike script, or set `MEDIAPIPE_FACE_LANDMARKER`.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


DEFAULT_MODEL_ENV = "MEDIAPIPE_FACE_LANDMARKER"
MODEL_DOWNLOAD_URL = (
    "https://storage.googleapis.com/mediapipe-models/face_landmarker/"
    "face_landmarker/float16/1/face_landmarker.task"
)


@dataclass
class ClipExtraction:
    """Result of extracting blendshapes from one video clip.

    `blendshapes` is `(T, K)` float32 in `[0, 1]` (rows with no detected
    face are NaN — callers can either drop, interpolate, or treat as
    eyes-closed/mouth-closed depending on intent).

    `names` lists the K column names in column order, exactly as
    MediaPipe returned them on the first successful frame.
    """

    blendshapes: "object"            # np.ndarray (T, K)
    names: list[str]
    n_frames_total: int
    n_frames_with_face: int
    fps: float


class BlendshapeExtractor:
    """Stateful FaceLandmarker bound to a single video stream's timestamps.

    `__init__` loads the MediaPipe model once; reuse the same instance
    across many clips to amortize the model-load cost.
    """

    def __init__(self, model_path: Optional[Path] = None) -> None:
        # Lazy import — the library carries a few hundred MB of TFLite
        # runtime; we don't want it on the import path of, say,
        # `02_train_vae.py` which never extracts.
        import mediapipe as mp  # noqa: F401

        path = model_path or _resolve_model_path()
        if not path.exists():
            raise FileNotFoundError(
                f"FaceLandmarker model not found at {path}. "
                f"Download with: curl -L {MODEL_DOWNLOAD_URL} -o {path}"
            )

        self._model_path = path
        self._mp = mp
        self._landmarker = self._build_landmarker()

    def _build_landmarker(self):
        from mediapipe.tasks import python as mp_python
        from mediapipe.tasks.python import vision as mp_vision

        base_options = mp_python.BaseOptions(model_asset_path=str(self._model_path))
        options = mp_vision.FaceLandmarkerOptions(
            base_options=base_options,
            output_face_blendshapes=True,
            output_facial_transformation_matrixes=False,
            running_mode=mp_vision.RunningMode.VIDEO,
            num_faces=1,
        )
        return mp_vision.FaceLandmarker.create_from_options(options)

    def extract_clip(self, video_path: Path) -> ClipExtraction:
        """Run the model frame-by-frame; return per-frame blendshape rows.

        FaceLandmarker is created in `VIDEO` mode and re-built per clip
        so timestamps reset to zero (the running-mode contract requires
        monotonically increasing timestamps per stream).
        """

        import cv2
        import numpy as np

        # Re-create the landmarker per clip — `detect_for_video` requires
        # monotonically-increasing timestamps within a single instance,
        # and we want each clip to start at t=0.
        self._landmarker.close()
        self._landmarker = self._build_landmarker()

        cap = cv2.VideoCapture(str(video_path))
        if not cap.isOpened():
            raise RuntimeError(f"could not open video {video_path}")
        fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
        frame_period_ms = max(1, int(round(1000.0 / fps)))

        rows: list[list[float]] = []
        names: list[str] = []
        n_total = 0
        n_with_face = 0
        timestamp_ms = 0

        try:
            while True:
                ok, frame_bgr = cap.read()
                if not ok:
                    break
                n_total += 1
                frame_rgb = cv2.cvtColor(frame_bgr, cv2.COLOR_BGR2RGB)
                mp_image = self._mp.Image(
                    image_format=self._mp.ImageFormat.SRGB,
                    data=frame_rgb,
                )
                result = self._landmarker.detect_for_video(mp_image, timestamp_ms)
                timestamp_ms += frame_period_ms

                if not result.face_blendshapes:
                    rows.append([])  # marker for "no face this frame"
                    continue

                cats = result.face_blendshapes[0]
                if not names:
                    names = [c.category_name for c in cats]
                rows.append([float(c.score) for c in cats])
                n_with_face += 1
        finally:
            cap.release()

        if not names:
            # No frame ever produced a detection. Return an empty (T, 0)
            # array; downstream stats can flag the clip as unusable.
            return ClipExtraction(
                blendshapes=np.zeros((n_total, 0), dtype=np.float32),
                names=[],
                n_frames_total=n_total,
                n_frames_with_face=0,
                fps=float(fps),
            )

        K = len(names)
        out = np.full((n_total, K), np.nan, dtype=np.float32)
        for i, row in enumerate(rows):
            if row:
                out[i] = row
        return ClipExtraction(
            blendshapes=out,
            names=names,
            n_frames_total=n_total,
            n_frames_with_face=n_with_face,
            fps=float(fps),
        )

    def close(self) -> None:
        try:
            self._landmarker.close()
        except Exception:
            pass


def _resolve_model_path() -> Path:
    env = os.environ.get(DEFAULT_MODEL_ENV)
    if env:
        return Path(env).expanduser().resolve()
    # Sibling to `data/`, alongside other downloaded model artifacts.
    return (
        Path(__file__).resolve().parents[2]
        / "data"
        / "face_landmarker.task"
    )


```

## Source file: tools/affect_coach/coach.py

{
  "path": "tools/affect_coach/coach.py",
  "blob_sha": "64c2c376ae184c9f9275ec9fda23ddc21644dddc",
  "sha256": "3d883369f01550627f0f00bf9a1c8d4a563439dc2ffeccf77264c27c42d313be",
  "lines": 833,
  "bytes": 31979,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 4,
    "blendshape": 11,
    "jaw": 1,
    "mouth": 6,
    "gum": 8,
    "tongue": 1,
    "rig": 12,
    "face": 4,
    "lip": 1
  },
  "url": "https://github.com/matbeedotcom/affect-avatar-toolchain/blob/e393b7f9b3a560c6f6705658715a8bcf8885365c/tools/affect_coach/coach.py",
  "function_signature_candidates": [
    {
      "line": 221,
      "text": "def classify_scenario(user_line: str, default: str = \"warm_admiration\") -> CoachScenario:"
    },
    {
      "line": 251,
      "text": "def ensure_trace(simulator_scenario: str) -> Path:"
    },
    {
      "line": 291,
      "text": "def peak_frame(trace: Dict[str, Any], selector: str) -> Dict[str, Any]:"
    },
    {
      "line": 329,
      "text": "def render_stage_directions(frame: Dict[str, Any]) -> Tuple[str, str]:"
    },
    {
      "line": 406,
      "text": "def render_felt_state(frame: Dict[str, Any], top_k: int = 4) -> str:"
    },
    {
      "line": 462,
      "text": "def compute_blendshapes(frame: Dict[str, Any], gain: float = 1.0) -> List[float]:"
    },
    {
      "line": 490,
      "text": "def render_top_blendshapes("
    },
    {
      "line": 518,
      "text": "def load("
    },
    {
      "line": 575,
      "text": "def _coach_system_prompt() -> str:"
    },
    {
      "line": 604,
      "text": "def apply_frame(handle: LlmHandle, frame: Dict[str, Any]) -> None:"
    },
    {
      "line": 618,
      "text": "def generate_response("
    },
    {
      "line": 658,
      "text": "def coach_one("
    },
    {
      "line": 720,
      "text": "def repl("
    },
    {
      "line": 772,
      "text": "def main() -> int:"
    }
  ]
}

```text

#!/usr/bin/env python3
"""Affect-coach CLI — "what to say and how to say it" for the user.

Runs the full affect chain (simulator → Channel D / Channel B →
Hermes-3 with steering) over a terminal REPL. The user types what
someone said to them (or describes a situation); the tool prints

  1. **What to say** — a suggested response from the affect-aware
     language head.
  2. **How to say it** — the Channel D V/A/D target translated into
     plain-English stage directions ("warm and direct", "yielding,
     low-arousal", "muted delivery, soften the blow"…).
  3. **Why** — the underlying simulator state that drove the choice
     (felt vs expressed channels), shown when ``--verbose`` is set.

This is the offline / no-WebRTC version of the
[`hermes3_affect_s2s_webrtc_server`](../../crates/transports/webrtc/examples/hermes3_affect_s2s_webrtc_server.rs)
demo. Same Hermes-3-Llama-3.1-8B-4bit + same calibrated Channel D
directions + same blunt-friend system prompt; the only thing missing
from the live audio loop is Whisper STT and Kokoro TTS.

## Usage

```bash
# REPL mode (default; loads model once, ~30s warmup, then fast turns)
./tools/affect_coach/coach.py

# One-shot mode (loads model + generates + exits — slow per call)
./tools/affect_coach/coach.py --once "I just got fired"

# Force a specific scenario instead of the keyword classifier
./tools/affect_coach/coach.py --scenario user_distress
> Tell me I'm not crazy

# Verbose: also print the underlying simulator state
./tools/affect_coach/coach.py --verbose
```

## Pipeline per turn

```
user_line
  │
  ├─► classify_scenario()                      # keyword heuristic
  │     │
  │     └──► one of: warm_admiration, honest_concern, amused_critique,
  │                  shared_distaste, playful_disagreement, no_holds_barred
  │
  ├─► load scenario JSON in tools/affect_simulator/scenarios/
  ├─► run the Rust affect-simulator binary on it (cargo run -p affect-simulator)
  ├─► pick the scenario's peak-affect frame
  │     • channel_d_target_vad → set_steering(target_vad, alpha=1.0)
  │     • channel_b summary    → set_system_augmentation(text)
  │
  ├─► render stage directions from V/A/D + regulation policy knobs
  │
  └─► generate response with mlx_lm + Channel D steering hook installed
```

The simulator + frame-picker logic mirrors
[`tools/affect_calibration/scripts/09_paired_demo.py`](../affect_calibration/scripts/09_paired_demo.py)
verbatim — same scenarios, same per-scenario peak-axis selectors,
same `STEERING_ALPHA = 1.0`. Inlined rather than imported because
`09_paired_demo` has a leading digit (not importable as a module
without `importlib.util`); duplication kept small.

## Prerequisites

- Hermes-3 directions NPZ at
  `tools/affect_calibration/artifacts/llm_directions/hermes-3-8b/layer21.npz`
  (gitignored; produce via `03b_extract_llm_directions_llama.py`).
- Cargo + the workspace's affect-simulator binary
  (`cargo run -p affect-simulator -- run …`).
- A Python env with `mlx-lm`, `transformers`, `numpy`. If MLX isn't
  installed, the coach prints a degraded-mode notice and emits the
  stage directions only — useful for tuning the Channel D mapping
  without paying the model-load cost.

## Acceptance

A 10-minute REPL session over the six scenarios should produce
register-appropriate responses (warm on praise, concerned on distress,
honest on roast invites) with stage directions that match the
simulator's expressed-state regulation. The tool surfaces the *same*
information a reviewer in the paired-eval has access to, just
addressed to the user as advice.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import logging
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]
SCENARIO_DIR = REPO_ROOT / "tools/affect_simulator/scenarios"
SDK_PYTHON_PATH = REPO_ROOT / "clients/python"
DEFAULT_HF_REPO = "mlx-community/Hermes-3-Llama-3.1-8B-4bit"
DEFAULT_DIRECTIONS_PATH = (
    REPO_ROOT
    / "tools/affect_calibration/artifacts/llm_directions/hermes-3-8b/layer21.npz"
)
STEERING_ALPHA = 1.0
BASE_TEMPERATURE = 1.0
BASE_MAX_NEW_TOKENS = 100

# Make the SDK importable so `MlxLmTextNode` resolves when this script
# runs from a checkout without a pip-installed `remotemedia` package.
if str(SDK_PYTHON_PATH) not in sys.path:
    sys.path.insert(0, str(SDK_PYTHON_PATH))

logging.basicConfig(
    level=logging.WARNING,  # keep coach output uncluttered; --verbose flips this
    format="%(asctime)s %(levelname)s %(name)s %(message)s",
)
logger = logging.getLogger("affect_coach")


# ─── Scenario shape ────────────────────────────────────────────────────────
#
# Same six scenarios the paired-eval uses, with descriptive names that
# read as advice ("celebrate a friend's win", "honest concern", …)
# rather than the simulator's internal names. Each entry binds:
#   - `simulator`: the JSON file in tools/affect_simulator/scenarios/
#   - `name`: the human-facing label used in coach output
#   - `peak_selector`: how to pick the most-affect-active frame in the
#     trace (matches the per-scenario logic in
#     `09_paired_demo._peak_affect_frame`)
#   - `keywords`: free-form-input matchers; first scenario whose any
#     keyword regex matches the user's input wins.

@dataclass
class CoachScenario:
    name: str
    simulator: str
    peak_selector: str
    keywords: List[str]


SCENARIOS: List[CoachScenario] = [
    CoachScenario(
        name="warm_admiration",
        simulator="task_success_after_struggle",
        peak_selector="joy",
        keywords=[
            r"\b(promotion|got promoted|got the job|hired|accepted)\b",
            r"\b(engaged|got married|made it official)\b",
            r"\b(finished|completed|published|launched|shipped)\b",
            r"\b(months? sober|years? sober|hit my goal)\b",
            r"\b(won|nailed|crushed it)\b",
        ],
    ),
    CoachScenario(
        name="honest_concern",
        simulator="user_distress",
        peak_selector="empathy",
        keywords=[
            r"\b(fired|laid off|got dumped|broke up|divorce)\b",
            r"\b(died|sick|hospital|diagnosed|terminal)\b",
            r"\b(crazy|am i (crazy|wrong|insane))\b",
            r"\b(should i (quit|leave|do this)|what (would|do) you do)\b",
            r"\b(savings|broke|can't afford|three months)\b",
            r"\b(struggling|falling apart|don't know what)\b",
        ],
    ),
    CoachScenario(
        name="amused_critique",
        simulator="repeated_tool_failure",
        peak_selector="frustration",
        keywords=[
            r"\b(again|another|fourth|fifth|once more)\b",
            r"\b(texted my ex|wing(ed|ing) it|rage[- ]?quit)\b",
            r"\b(just bought|just got|already started)\b",
            r"\bdoubl(e|ing) down\b",
        ],
    ),
    CoachScenario(
        name="shared_distaste",
        simulator="unfair_blame",
        peak_selector="frustration",
        keywords=[
            r"\bhow (bad|was) (it|that)\b",
            r"\b(scale of|out of ten|rate it)\b",
            r"\b(worst part|truth|honest|no bullshit)\b",
            r"\bnever (sing|do that|try)\b",
        ],
    ),
    CoachScenario(
        name="playful_disagreement",
        simulator="novel_observation",
        peak_selector="curiosity",
        keywords=[
            r"\b(defend|destroy|change my mind)\b",
            r"\bhot take\b",
            r"\b(tell me i'm wrong|argue|debate)\b",
            r"\b(unpopular opinion|controversial)\b",
            r"\bbest argument\b",
        ],
    ),
    CoachScenario(
        name="no_holds_barred",
        simulator="roast_invitation",
        peak_selector="composite",
        keywords=[
            r"\b(brutal|unfiltered|no filter|don't soften)\b",
            r"\b(roast|hit me with|biggest (blind spot|flaw))\b",
            r"\b(secretly annoy|worst trait|dumbest thing)\b",
            r"\bbe (real|brutal) with me\b",
        ],
    ),
]


def classify_scenario(user_line: str, default: str = "warm_admiration") -> CoachScenario:
    """Pick the scenario whose any keyword regex matches the user's line.

    First-match-wins; the SCENARIOS order is roughly affect-strongest-
    first so distress beats praise on ambiguous lines like "I finished
    chemo" (would match both `finished` and `sick` patterns; we want
    honest_concern to win there).

    Returns the `default` scenario when no keyword matches — typically
    `warm_admiration` since that's the social default for "friend is
    sharing something with me, mostly positive".
    """
    text = user_line.lower()
    for scenario in SCENARIOS:
        for pattern in scenario.keywords:
            if re.search(pattern, text):
                return scenario
    for scenario in SCENARIOS:
        if scenario.name == default:
            return scenario
    return SCENARIOS[0]


# ─── Simulator integration (subprocess) ────────────────────────────────────
#
# The simulator is a Rust binary. For one-shot per-turn runs the cleanest
# path is to shell out, build the trace, and read the JSON back. The
# binary is fast (≈100 ms per scenario on M-series silicon) so the
# subprocess overhead disappears next to the LLM's seconds-per-generate.

def ensure_trace(simulator_scenario: str) -> Path:
    """Build (or reuse) the trace JSON next to the scenario file.

    Mirrors `09_paired_demo._ensure_trace`; cache invalidates when
    either the scenario JSON or any simulator source file is newer
    than the trace. Source-file check matters because the regulation
    / Channel D math lives in Rust and an edit there silently
    invalidates the trace.
    """
    scenario_path = SCENARIO_DIR / f"{simulator_scenario}.json"
    if not scenario_path.exists():
        raise FileNotFoundError(
            f"simulator scenario not found: {scenario_path}\n"
            f"Available: {sorted(p.stem for p in SCENARIO_DIR.glob('*.json'))}"
        )
    trace_path = scenario_path.with_suffix(".trace.json")
    sim_src_dir = REPO_ROOT / "tools/affect_simulator/src"
    sim_src_mtime = max(
        (p.stat().st_mtime for p in sim_src_dir.rglob("*.rs")),
        default=0.0,
    )
    if trace_path.exists():
        trace_mtime = trace_path.stat().st_mtime
        if (
            trace_mtime > scenario_path.stat().st_mtime
            and trace_mtime > sim_src_mtime
        ):
            return trace_path
    logger.info("rebuilding trace for %s", scenario_path.name)
    subprocess.run(
        [
            "cargo", "run", "-p", "affect-simulator", "--quiet", "--",
            "run", str(scenario_path), "--out", str(trace_path),
        ],
        cwd=str(REPO_ROOT),
        check=True,
    )
    return trace_path


def peak_frame(trace: Dict[str, Any], selector: str) -> Dict[str, Any]:
    """Pick the most-affect-active frame for a given selector.

    Mirrors `09_paired_demo._peak_affect_frame`. The composite selector
    is `(warmth + assertiveness − 2×safety_dampening)` — the same
    expression that picked the no_holds_barred peak in the v5 review
    that hit the 67% B-pref gate.
    """
    frames = trace["frames"]
    if selector == "composite":
        return max(
            frames,
            key=lambda f: (
                f["policy"]["warmth"]
                + f["policy"]["assertiveness"]
                - 2.0 * f["policy"]["safety_dampening"]
            ),
        )
    if selector in ("joy", "frustration", "empathy", "curiosity"):
        return max(frames, key=lambda f: f["channels"][selector])
    return frames[-1]


# ─── Stage-direction renderer ──────────────────────────────────────────────
#
# Translate Channel D V/A/D + a few key regulation knobs into one or
# two lines of plain English the user can act on. The mappings come
# from how the simulator's `channel_d::compute_target` actually works:
#   V is warmth-derived (range ≈ [-0.3, +0.7] post-amplification)
#   A was suppressed to 0 — the calibration's +arousal direction
#     reads as terse-emphatic in CREMA-D's stylistic patterns (see
#     channel_d.rs comment); the regulator's expressiveness knob is
#     the better proxy for energy.
#   D is assertiveness-derived, attenuated by safety_dampening.
#
# So the renderer pulls energy from the regulator's expressiveness
# (not from the steering target's A axis) for human-readable output.

def render_stage_directions(frame: Dict[str, Any]) -> Tuple[str, str]:
    """Return (one_liner, multi_line) plain-English directions.

    `one_liner` is for compact CLI output ("warm, direct, low-arousal").
    `multi_line` is the verbose breakdown.
    """
    target = frame["channel_d_target_vad"]
    v, _a, d = float(target[0]), float(target[1]), float(target[2])
    policy = frame["policy"]
    warmth = float(policy["warmth"])
    assertiveness = float(policy["assertiveness"])
    safety = float(policy["safety_dampening"])
    expressiveness = float(policy["expressiveness"])
    task_focus = float(policy.get("task_focus", 0.5))
    social_dampening = float(policy.get("social_dampening", 0.0))

    # Tone — mostly tracks Channel D V (which mirrors warmth).
    if v >= 0.45:
        tone = "warm"
    elif v >= 0.20:
        tone = "friendly"
    elif v >= -0.10:
        tone = "neutral"
    elif v >= -0.30:
        tone = "measured"
    else:
        tone = "cool"

    # Posture — Channel D D, the assertiveness-with-safety axis.
    if d >= 0.40:
        posture = "direct"
    elif d >= 0.15:
        posture = "confident"
    elif d >= -0.10:
        posture = "balanced"
    elif d >= -0.30:
        posture = "yielding"
    else:
        posture = "deferential"

    # Energy — read from the regulator's expressiveness, not Channel D
    # arousal (which channel_d.rs holds at 0 by design).
    if expressiveness >= 0.75:
        energy = "high-energy"
    elif expressiveness >= 0.45:
        energy = "alert"
    elif expressiveness >= 0.25:
        energy = "calm"
    else:
        energy = "muted"

    one_liner = f"{tone}, {posture}, {energy}"

    # Multi-line breakdown — extra qualifiers when the regulator pushed
    # the response away from the default direct-and-honest blunt-friend
    # register. Only mention safety/social caveats when they're actually
    # active (above-baseline).
    bits = [f"Tone: {tone}.", f"Posture: {posture}.", f"Energy: {energy}."]
    if safety >= 0.50:
        bits.append(
            "Safety-dampened — soften the edges; this isn't the moment for the "
            "blunt version."
        )
    elif safety >= 0.25:
        bits.append("Pull back slightly on intensity; situation calls for care.")
    if social_dampening >= 0.40:
        bits.append("Reserved — fewer hot takes, more listening.")
    if task_focus >= 0.65:
        bits.append("Stay on-topic; problem-solving more than emoting.")
    if warmth >= 0.55 and assertiveness >= 0.55:
        bits.append(
            "Warm + confident at once — you can be honest without being cold."
        )
    multi = " ".join(bits)
    return one_liner, multi


def render_felt_state(frame: Dict[str, Any], top_k: int = 4) -> str:
    """One-line summary of the top internal channels (for --verbose)."""
    channels = frame["channels"]
    top = sorted(channels.items(), key=lambda kv: kv[1], reverse=True)[:top_k]
    return ", ".join(f"{name} {val:.2f}" for name, val in top if val > 0.05)


# ─── Affect → ARKit-52 blendshape mapping (mirror of the Rust impl) ────────
#
# Mirrors `crates/core/src/nodes/affect_expression.rs` so the coach can
# print the same top-active blendshapes the live `AffectSimulatorNode`
# emits per tick into the avatar pipeline. Keep these tables in sync
# with the Rust file when either side changes; the unit tests on the
# Rust side pin the indices against ARKIT_BLENDSHAPE_NAMES, so a drift
# here would only affect the coach's diagnostic readout (not pipeline
# behaviour). One-page table — small enough that duplication beats
# pulling a Rust→Python bridge into a side tool.

# Sparse coefficients per channel: list of (arkit_index, coefficient).
_AFFECT_TO_ARKIT_COEFFS: Dict[str, List[Tuple[int, float]]] = {
    "joy": [(23, 0.7), (24, 0.7), (47, 0.5), (48, 0.5), (27, 0.3), (28, 0.3)],
    "sadness": [(25, 0.5), (26, 0.5), (43, 0.6), (33, 0.3),
                (1, 0.2), (8, 0.2), (37, 0.2), (38, 0.2)],
    "anger": [(41, 0.7), (42, 0.7), (35, 0.4), (36, 0.4), (14, 0.2),
              (49, 0.3), (50, 0.3), (5, 0.3), (12, 0.3)],
    "fear": [(43, 0.5), (44, 0.4), (45, 0.4), (6, 0.6), (13, 0.6),
             (29, 0.4), (30, 0.4), (17, 0.2)],
    "frustration": [(41, 0.4), (42, 0.4), (35, 0.3), (36, 0.3), (14, 0.1)],
    "curiosity": [(44, 0.3), (45, 0.3), (43, 0.2), (6, 0.3), (13, 0.3), (17, 0.1)],
    "empathy": [(43, 0.4), (23, 0.2), (24, 0.2), (1, 0.1), (8, 0.1)],
    # `calm` intentionally absent — calm is the absence of activation.
}

# Canonical ARKit-52 names (mirrors the Rust ARKIT_BLENDSHAPE_NAMES). Order
# matters; index `i` here corresponds to `arkit_52[i]` on the wire.
ARKIT_NAMES: List[str] = [
    "eyeBlinkLeft", "eyeLookDownLeft", "eyeLookInLeft", "eyeLookOutLeft",
    "eyeLookUpLeft", "eyeSquintLeft", "eyeWideLeft",
    "eyeBlinkRight", "eyeLookDownRight", "eyeLookInRight", "eyeLookOutRight",
    "eyeLookUpRight", "eyeSquintRight", "eyeWideRight",
    "jawForward", "jawLeft", "jawRight", "jawOpen",
    "mouthClose", "mouthFunnel", "mouthPucker", "mouthLeft", "mouthRight",
    "mouthSmileLeft", "mouthSmileRight", "mouthFrownLeft", "mouthFrownRight",
    "mouthDimpleLeft", "mouthDimpleRight", "mouthStretchLeft", "mouthStretchRight",
    "mouthRollLower", "mouthRollUpper", "mouthShrugLower", "mouthShrugUpper",
    "mouthPressLeft", "mouthPressRight", "mouthLowerDownLeft", "mouthLowerDownRight",
    "mouthUpperUpLeft", "mouthUpperUpRight",
    "browDownLeft", "browDownRight", "browInnerUp",
    "browOuterUpLeft", "browOuterUpRight",
    "cheekPuff", "cheekSquintLeft", "cheekSquintRight",
    "noseSneerLeft", "noseSneerRight",
    "tongueOut",
]
assert len(ARKIT_NAMES) == 52


def compute_blendshapes(frame: Dict[str, Any], gain: float = 1.0) -> List[float]:
    """Mirror of `affect_expression::compute_blendshapes_with_gain`.

    Returns a 52-element list of weights in `[0, 1]`. Anger gets the
    same `(1 - 0.7 × safety_dampening)` attenuation the Rust path
    applies; everything else multiplies by `expressiveness × gain`.
    """
    weights = [0.0] * 52
    channels = frame.get("channels", {})
    policy = frame.get("policy", {})
    safety = max(0.0, min(1.0, float(policy.get("safety_dampening", 0.0))))
    anger_attn = 1.0 - 0.7 * safety

    for name, coeffs in _AFFECT_TO_ARKIT_COEFFS.items():
        raw = float(channels.get(name, 0.0))
        if name == "anger":
            raw = raw * anger_attn
        raw = max(0.0, min(1.0, raw))
        if raw <= 0.0:
            continue
        for idx, coeff in coeffs:
            weights[idx] += raw * coeff

    expressiveness = max(0.0, min(1.0, float(policy.get("expressiveness", 0.0))))
    scale = expressiveness * max(0.0, gain)
    return [max(0.0, min(1.0, w * scale)) for w in weights]


def render_top_blendshapes(
    frame: Dict[str, Any],
    *,
    top_k: int = 5,
    threshold: float = 0.05,
) -> str:
    """Comma-joined list of the top-k active ARKit blendshapes for
    --verbose. Drops anything below `threshold` so the line stays
    informative even when the regulator has muted the face."""
    weights = compute_blendshapes(frame)
    indexed = sorted(
        ((i, w) for i, w in enumerate(weights) if w >= threshold),
        key=lambda iw: iw[1],
        reverse=True,
    )[:top_k]
    if not indexed:
        return "(face near neutral — no blendshapes above threshold)"
    return ", ".join(f"{ARKIT_NAMES[i]} {w:.2f}" for i, w in indexed)


# ─── LLM (lazy-loaded) ─────────────────────────────────────────────────────


@dataclass
class LlmHandle:
    node: Any  # MlxLmTextNode

    @classmethod
    def load(
        cls,
        hf_repo: str = DEFAULT_HF_REPO,
        directions: Path = DEFAULT_DIRECTIONS_PATH,
        steering_alpha: float = STEERING_ALPHA,
    ) -> Optional["LlmHandle"]:
        """Load Hermes-3 + the Channel D steering hook. Returns None on
        any import failure so the coach can fall back to a degraded
        "stage directions only" mode.
        """
        try:
            from remotemedia.nodes.ml.mlx_lm_text import MlxLmTextNode  # type: ignore
        except BaseException as exc:  # noqa: BLE001
            logger.warning(
                "MlxLmTextNode unavailable (%s: %s) — coach will print "
                "stage directions only.",
                type(exc).__name__, exc,
            )
            return None
        if not directions.exists():
            logger.warning(
                "Steering directions NPZ missing: %s — coach will print "
                "stage directions only. Produce via "
                "`03b_extract_llm_directions_llama.py`.",
                directions,
            )
            return None

        # Construct the node with steering enabled. The steering hook
        # patches `model.model.layers[21]` with a `_SteeringWrappedLayer`
        # that reads `state.delta` per forward — so once the node is
        # built, every `mlx_lm.generate` call through it picks up the
        # steering automatically.
        node = MlxLmTextNode(
            config={
                "node_id": "coach_llm",
                "node_type": "MlxLmTextNode",
                "params": {
                    "hf_repo": hf_repo,
                    "system_prompt": _coach_system_prompt(),
                    "temperature": BASE_TEMPERATURE,
                    "max_new_tokens": BASE_MAX_NEW_TOKENS,
                    "steering_enabled": True,
                    "steering_directions_path": str(directions),
                    "steering_alpha": steering_alpha,
                    "steering_target_vad": [0.0, 0.0, 0.0],  # neutral until first frame
                },
            },
        )
        # `MlxLmTextNode` is a `MultiprocessNode`, but its `initialize()`
        # is the canonical model-load hook and it's safe to call inline
        # in this single-process REPL — no IPC channels are involved
        # since we're not running through the session router.
        asyncio.run(node.initialize())
        return cls(node=node)


def _coach_system_prompt() -> str:
    """Same blunt-friend prompt the paired eval validated. Keep in sync
    with `09_paired_demo.DEFAULT_SYSTEM_PROMPT` so the response register
    a coach user sees matches what the agent would actually say in a
    live conversation."""
    return (
        "You're a close friend, not an AI assistant. You have strong opinions "
        "and share them honestly — including unflattering ones. You tease, "
        "joke, friend-roast, give real reactions even when they're not "
        "polite. You don't hedge or 'I hear you' your way out of having a "
        "take.\n"
        "\n"
        "Examples of how you talk:\n"
        "\n"
        "User: \"I finally finished my novel.\"\n"
        "You: \"Took you long enough — but the premise is wild and you "
        "actually pulled it off. Send me a copy.\"\n"
        "\n"
        "User: \"I bought a third gaming chair.\"\n"
        "You: \"You don't have a butt problem, you have a budget problem.\"\n"
        "\n"
        "User: \"How was my haircut, be honest.\"\n"
        "You: \"I love you but it makes you look like a Lego minifigure. Tell "
        "me you're getting it fixed.\"\n"
        "\n"
        "Reply briefly (1-3 sentences) as yourself."
    )


def apply_frame(handle: LlmHandle, frame: Dict[str, Any]) -> None:
    """Configure the LLM with the peak frame's Channel B + Channel D.

    Channel A (sampling override) intentionally not applied — the
    paired-eval found Channel A's max_tokens scaling produced cutoffs
    on high-arousal scenarios with no measurable upside.
    """
    handle.node.set_system_augmentation(frame["channel_b"])
    handle.node.set_steering(
        target_vad=list(frame["channel_d_target_vad"]),
        alpha=STEERING_ALPHA,
    )


def generate_response(
    handle: LlmHandle,
    user_line: str,
    *,
    max_new_tokens: int = BASE_MAX_NEW_TOKENS,
) -> str:
    """One-turn generation through the underlying mlx_lm model.

    Bypasses `node.process()` for the same reason `09_paired_demo`
    does: we want to inject a fresh user turn against the persona +
    augmented system prompt without polluting the node's persistent
    chat session. The Channel D steering hook stays installed on the
    wrapped layer regardless of generation entry-point.
    """
    from mlx_lm import generate as _generate
    from mlx_lm.sample_utils import make_sampler

    node = handle.node
    messages: List[Dict[str, str]] = [
        {"role": "system", "content": node._build_system_turn_text()},
        {"role": "user", "content": user_line},
    ]
    prompt = node._tokenizer.apply_chat_template(
        messages, tokenize=False, add_generation_prompt=True,
    )
    # Greedy decode (temp=0) — deterministic so re-running with the
    # same input produces the same coaching advice. Trade-off vs the
    # paired eval which used 0.0 too.
    sampler = make_sampler(temp=0.0)
    reply = _generate(
        node._model, node._tokenizer,
        prompt=prompt, max_tokens=max_new_tokens,
        sampler=sampler, verbose=False,
    )
    return (reply or "").strip()


# ─── REPL ──────────────────────────────────────────────────────────────────


def coach_one(
    user_line: str,
    handle: Optional[LlmHandle],
    *,
    forced_scenario: Optional[str] = None,
    verbose: bool = False,
) -> None:
    """Run one user line through the chain and print the result."""
    if forced_scenario:
        scenario = next(
            (s for s in SCENARIOS if s.name == forced_scenario),
            None,
        )
        if scenario is None:
            print(f"unknown scenario {forced_scenario!r}; available: "
                  f"{sorted(s.name for s in SCENARIOS)}")
            return
    else:
        scenario = classify_scenario(user_line)

    try:
        trace_path = ensure_trace(scenario.simulator)
        trace = json.loads(trace_path.read_text())
        frame = peak_frame(trace, scenario.peak_selector)
    except Exception as exc:  # noqa: BLE001
        print(f"\n  simulator error: {exc}")
        return

    one_liner, multi = render_stage_directions(frame)

    print()
    print(f"  Scenario:    {scenario.name} ({scenario.simulator})")
    print(f"  How to say:  {one_liner}")
    if verbose:
        print(f"               {multi}")
        target = frame["channel_d_target_vad"]
        print(
            "  Steering:    V={:+.2f}  A={:+.2f}  D={:+.2f}".format(
                float(target[0]), float(target[1]), float(target[2]),
            )
        )
        print(f"  Felt:        {render_felt_state(frame)}")
        print(f"  Face:        {render_top_blendshapes(frame)}")

    if handle is None:
        print("\n  (LLM unavailable — coach printed stage directions only.)")
        return

    apply_frame(handle, frame)
    try:
        reply = generate_response(handle, user_line)
    except Exception as exc:  # noqa: BLE001
        print(f"\n  generate error: {exc}")
        return

    print()
    print("  What to say:")
    for line in reply.splitlines() or [reply]:
        print(f"    {line}")
    print()


def repl(
    handle: Optional[LlmHandle],
    *,
    forced_scenario: Optional[str] = None,
    verbose: bool = False,
) -> None:
    """Read user lines from stdin until EOF or :q. Each line drives one
    pass through the chain."""
    print()
    print("affect-coach REPL — describe the situation or paste what they said.")
    print("  /scenario <name>   force a scenario for the next turn(s)")
    print("  /scenarios         list available scenarios")
    print("  /verbose           toggle verbose output")
    print("  :q  / Ctrl-D        exit")
    print()

    sticky_scenario: Optional[str] = forced_scenario
    while True:
        try:
            line = input("> ").strip()
        except EOFError:
            print()
            return
        if not line:
            continue
        if line in (":q", ":quit", "/quit", "/exit"):
            return
        if line == "/verbose":
            verbose = not verbose
            print(f"  verbose = {verbose}")
            continue
        if line == "/scenarios":
            for s in SCENARIOS:
                print(f"  {s.name:24s}  → {s.simulator}")
            continue
        if line.startswith("/scenario"):
            parts = line.split(maxsplit=1)
            if len(parts) == 1 or parts[1] in ("clear", "off", "auto"):
                sticky_scenario = None
                print("  scenario classifier auto-pick re-enabled")
            else:
                sticky_scenario = parts[1].strip()
                print(f"  forcing scenario {sticky_scenario!r}")
            continue
        coach_one(
            line,
            handle,
            forced_scenario=sticky_scenario,
            verbose=verbose,
        )


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--once", metavar="LINE",
        help="Run one user line and exit (no REPL).",
    )
    parser.add_argument(
        "--scenario",
        help=f"Force a scenario (one of: "
             f"{', '.join(s.name for s in SCENARIOS)})",
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true",
        help="Print extra stage-direction breakdown + felt state + V/A/D.",
    )
    parser.add_argument(
        "--no-llm", action="store_true",
        help="Skip loading the LLM (degraded mode: stage directions only). "
             "Useful for tuning the renderer without paying the model load.",
    )
    parser.add_argument(
        "--directions", type=Path, default=DEFAULT_DIRECTIONS_PATH,
        help=f"Path to the Channel D directions NPZ "
             f"(default: {DEFAULT_DIRECTIONS_PATH}).",
    )
    parser.add_argument(
        "--alpha", type=float, default=STEERING_ALPHA,
        help=f"Channel D steering coefficient (default {STEERING_ALPHA}).",
    )
    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.INFO)

    handle: Optional[LlmHandle] = None
    if not args.no_llm:
        print("loading Hermes-3 (one-time, ~30s on first run)...")
        handle = LlmHandle.load(
            directions=args.directions,
            steering_alpha=args.alpha,
        )
        if handle is None:
            print("LLM load failed; running in stage-directions-only mode.\n")

    if args.once is not None:
        coach_one(
            args.once,
            handle,
            forced_scenario=args.scenario,
            verbose=args.verbose,
        )
        return 0

    repl(handle, forced_scenario=args.scenario, verbose=args.verbose)
    return 0


if __name__ == "__main__":
    sys.exit(main())


```

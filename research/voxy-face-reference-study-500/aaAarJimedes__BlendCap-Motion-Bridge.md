# Facial rendering source scan: aaAarJimedes/BlendCap-Motion-Bridge

Repository: https://github.com/aaAarJimedes/BlendCap-Motion-Bridge

Pinned source revision: 61c114894c5a9b5d4bb63badb1d73375f050bab1

Discovery topic: facial retargeting

Description: Blender bridge for retargeting BlendCap motion and facial capture to MMD characters.

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["blendcap_motion_bridge/face_mapper.py", "test_face_mapper.py"]}

## Source file: blendcap_motion_bridge/face_mapper.py

{
  "path": "blendcap_motion_bridge/face_mapper.py",
  "blob_sha": "31bb3d0f586e96542ae462b432eb4bd081203a5f",
  "sha256": "60d1ff621b3bb8d1156c5af6a03dbb0b16ad07a20883ae89edaee72e614f481c",
  "lines": 259,
  "bytes": 9173,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 21,
    "blendshape": 4,
    "jaw": 3,
    "mouth": 15,
    "tongue": 1,
    "rig": 42,
    "face": 6,
    "lip": 3
  },
  "url": "https://github.com/aaAarJimedes/BlendCap-Motion-Bridge/blob/61c114894c5a9b5d4bb63badb1d73375f050bab1/blendcap_motion_bridge/face_mapper.py",
  "function_signature_candidates": [
    {
      "line": 130,
      "text": "def _norm(name):"
    },
    {
      "line": 134,
      "text": "def build_face_channels(bs_names, shape_key_names):"
    },
    {
      "line": 188,
      "text": "def compose_values(blendshapes, channels):"
    },
    {
      "line": 207,
      "text": "def _resolve_eye_conflicts(vals, channels):"
    }
  ]
}

```text

"""Map BlendCap's ARKit face weights onto MMD Tools shape keys.

BlendCap's own ARKit apply expects the mesh to already carry ARKit-named
shape keys.  MMD models often only carry Japanese keys (まばたき, あ, ...),
so this module first matches ARKit names directly, then falls back to
deterministic compositions for the common MMD expressions.  Direct and
fallback channels are independent: a model with a few ARKit keys can still
use its Japanese morphs for every other expression.
"""

import dataclasses
import unicodedata

import numpy as np


@dataclasses.dataclass(frozen=True)
class FaceChannel:
    target: str
    sources: tuple
    combine: str = "sum"


# Each rule maps one or more MMD shape-key names to a weighted ARKit
# composition.  "max" picks the strongest source; "sum" adds weighted
# sources and clips the final value to [0, 1].
MMD_FALLBACK_RULES = (
    # Eyes: two-eye and one-eye blinks.
    (("まばたき", "blink", "まばた"), "max",
     (("eyeBlinkLeft", 1.0), ("eyeBlinkRight", 1.0))),
    (("ウィンク", "wink", "wink left", "winkL"), "max",
     (("eyeBlinkLeft", 1.0),)),
    (("ウィンク右", "wink right", "winkR"), "max",
     (("eyeBlinkRight", 1.0),)),
    (("ウィンク２", "ウィンク2", "wink2", "wink left2", "winkL2"), "max",
     (("eyeBlinkLeft", 1.0),)),
    (("ｳｨﾝｸ２右", "ウィンク2右", "wink right2", "winkR2"), "max",
     (("eyeBlinkRight", 1.0),)),

    # Other eye expressions and gaze.
    (("じと目", "deadpan"), "max",
     (("eyeSquintLeft", 1.0), ("eyeSquintRight", 1.0))),
    (("びっくり", "surprised"), "max",
     (("eyeWideLeft", 1.0), ("eyeWideRight", 1.0))),
    (("上", "look up", "lookup"), "max",
     (("eyeLookUpLeft", 1.0), ("eyeLookUpRight", 1.0))),
    (("下", "look down", "lookdown"), "max",
     (("eyeLookDownLeft", 1.0), ("eyeLookDownRight", 1.0))),
    (("左", "look left", "lookleft"), "max",
     (("eyeLookOutLeft", 1.0), ("eyeLookInRight", 1.0))),
    (("右", "look right", "lookright"), "max",
     (("eyeLookInLeft", 1.0), ("eyeLookOutRight", 1.0))),
    (("瞳左", "pupil left"), "max",
     (("eyeLookOutLeft", 1.0), ("eyeLookInRight", 1.0))),
    (("瞳右", "pupil right"), "max",
     (("eyeLookInLeft", 1.0), ("eyeLookOutRight", 1.0))),

    # Brows and mood expressions that mainly move the brows.
    (("まゆげ1", "眉上げ", "まゆ上げ", "brow up"), "max",
     (("BrowInnerUp", 1.0), ("BrowOuterUpLeft", 1.0),
      ("BrowOuterUpRight", 1.0))),
    (("眉下げ", "まゆ下げ", "brow down"), "max",
     (("BrowDownLeft", 1.0), ("BrowDownRight", 1.0))),
    (("困る", "troubled"), "max",
     (("BrowDownLeft", 1.0), ("BrowDownRight", 1.0))),
    (("怒り", "angry"), "max",
     (("BrowDownLeft", 1.0), ("BrowDownRight", 1.0),
      ("mouthFrownLeft", 0.5), ("mouthFrownRight", 0.5))),

    # Vowel morphs.
    (("あ", "a", "A"), "sum", (("jawOpen", 1.0),)),
    (("い", "i", "I"), "sum",
     (("mouthStretchLeft", 0.5), ("mouthStretchRight", 0.5))),
    (("う", "u", "U"), "sum",
     (("mouthFunnel", 0.8), ("mouthPucker", 0.2))),
    (("え", "e", "E"), "sum",
     (("mouthSmileLeft", 0.5), ("mouthSmileRight", 0.5))),
    (("お", "o", "O"), "sum",
     (("mouthPucker", 0.7), ("jawOpen", 0.3))),

    # Mouth expressions.
    (("笑い", "笑い1", "にこり", "にやり", "smile", "ha"), "max",
     (("mouthSmileLeft", 1.0), ("mouthSmileRight", 1.0),
      ("cheekSquintLeft", 1.0), ("cheekSquintRight", 1.0))),
    (("口角上げ", "mouth corner up"), "sum",
     (("mouthSmileLeft", 0.5), ("mouthSmileRight", 0.5))),
    (("口角上げ1",), "sum",
     (("mouthSmileLeft", 0.5), ("mouthSmileRight", 0.5))),
    (("口角下げ", "mouth corner down"), "sum",
     (("mouthFrownLeft", 0.5), ("mouthFrownRight", 0.5))),
    (("口真一文字", "mouth straight"), "max",
     (("mouthPressLeft", 1.0), ("mouthPressRight", 1.0))),
    (("頬", "cheek"), "sum", (("cheekPuff", 1.0),)),
    (("ぺろっ", "tongue"), "sum", (("tongueOut", 1.0),)),
    (("△", "triangle"), "sum",
     (("mouthFunnel", 0.5), ("jawOpen", 0.5))),
)


BLINK_LEFT_KEYS = (
    "eyeBlinkLeft",
    "ウィンク",
    "ウィンク２",
    "ウィンク2",
    "wink",
    "wink left",
    "winkL",
    "wink2",
    "winkL2",
)

BLINK_RIGHT_KEYS = (
    "eyeBlinkRight",
    "ウィンク右",
    "ｳｨﾝｸ２右",
    "ウィンク2右",
    "wink right",
    "winkR",
    "wink right2",
    "winkR2",
)

BLINK_BOTH_KEYS = (
    "まばたき",
    "blink",
    "まばた",
)


def _norm(name):
    return unicodedata.normalize("NFKC", name or "").strip().lower()


def build_face_channels(bs_names, shape_key_names):
    """Return FaceChannel list for the mesh's shape keys."""
    source_index = {}
    for i, name in enumerate(bs_names):
        source_index.setdefault(_norm(name), i)

    target_lookup = {}
    for name in shape_key_names:
        target_lookup.setdefault(_norm(name), name)

    channels = []
    seen_targets = set()
    direct_source_names = set()

    # ARKit-named shape keys are always driven directly with their source
    # value.  MMD Tools sometimes prefixes imported ARKit keys with "_".
    for i, bs_name in enumerate(bs_names):
        key = _norm(bs_name)
        target = target_lookup.get(key) or target_lookup.get(f"_{key}")
        if target is None:
            continue
        channels.append(FaceChannel(target, ((i, 1.0),), "sum"))
        seen_targets.add(_norm(target))
        direct_source_names.add(key)

    # Japanese/English MMD morphs are filled in as separate channels.  A rule
    # only runs when its morph exists on the mesh, has not already been driven,
    # and at least one ARKit source is missing from the direct set.  The last
    # condition prevents a mixed model from double-driving the same expression
    # through an ARKit key and its Japanese duplicate.
    for sk_names, combine, sources in MMD_FALLBACK_RULES:
        target = None
        for name in sk_names:
            key = _norm(name)
            if key in target_lookup:
                target = target_lookup[key]
                break
        if target is None or _norm(target) in seen_targets:
            continue
        if all(_norm(arkit) in direct_source_names for arkit, _weight in sources):
            continue
        resolved = tuple(
            (source_index[_norm(arkit)], weight)
            for arkit, weight in sources
            if _norm(arkit) in source_index
        )
        if not resolved:
            continue
        channels.append(FaceChannel(target, resolved, combine))
        seen_targets.add(_norm(target))

    return channels


def compose_values(blendshapes, channels):
    """Return (N, len(channels)) float32 weights clipped to [0, 1]."""
    vals = np.zeros((len(blendshapes), len(channels)), dtype=np.float32)
    for col, channel in enumerate(channels):
        if channel.combine == "max":
            for bs_idx, weight in channel.sources:
                np.maximum(
                    vals[:, col],
                    blendshapes[:, bs_idx] * weight,
                    out=vals[:, col],
                )
        else:
            for bs_idx, weight in channel.sources:
                vals[:, col] += blendshapes[:, bs_idx] * weight
    np.clip(vals, 0.0, 1.0, out=vals)
    _resolve_eye_conflicts(vals, channels)
    return vals


def _resolve_eye_conflicts(vals, channels):
    """Keep blink/wink channels exclusive so one-eye and two-eye closures
    do not stack on the same eye geometry."""
    left_cols = [
        i for i, channel in enumerate(channels)
        if _norm(channel.target) in {_norm(n) for n in BLINK_LEFT_KEYS}
    ]
    right_cols = [
        i for i, channel in enumerate(channels)
        if _norm(channel.target) in {_norm(n) for n in BLINK_RIGHT_KEYS}
    ]
    both_cols = [
        i for i, channel in enumerate(channels)
        if _norm(channel.target) in {_norm(n) for n in BLINK_BOTH_KEYS}
    ]

    if not left_cols or not right_cols:
        return

    left = np.maximum.reduce([vals[:, i] for i in left_cols])
    right = np.maximum.reduce([vals[:, i] for i in right_cols])

    for frame in range(len(vals)):
        l = float(left[frame])
        r = float(right[frame])
        peak = max(l, r)
        if peak <= 0.05:
            for i in left_cols + right_cols + both_cols:
                vals[frame, i] = 0.0
            continue
        if min(l, r) >= peak * 0.7:
            if both_cols:
                for i in both_cols:
                    vals[frame, i] = peak
                for i in left_cols + right_cols:
                    vals[frame, i] = 0.0
            else:
                for i in left_cols + right_cols:
                    vals[frame, i] = peak
            continue
        if both_cols:
            for i in both_cols:
                vals[frame, i] = 0.0
        if l >= r:
            for i in left_cols:
                vals[frame, i] = l
            for i in right_cols:
                vals[frame, i] = 0.0
        else:
            for i in right_cols:
                vals[frame, i] = r
            for i in left_cols:
                vals[frame, i] = 0.0


```

## Source file: test_face_mapper.py

{
  "path": "test_face_mapper.py",
  "blob_sha": "e0743eace9f8b29bb29308f4b91c9074b062a9be",
  "sha256": "bee7290145b028450f10924ad9553703d338db6611551826680fa4958846fa2f",
  "lines": 153,
  "bytes": 6477,
  "selection_score": 13,
  "keyword_line_counts": {
    "eye": 24,
    "jaw": 4,
    "mouth": 9,
    "tongue": 1,
    "rig": 23,
    "face": 16
  },
  "url": "https://github.com/aaAarJimedes/BlendCap-Motion-Bridge/blob/61c114894c5a9b5d4bb63badb1d73375f050bab1/test_face_mapper.py",
  "function_signature_candidates": [
    {
      "line": 42,
      "text": "def test_direct_arkit_mapping():"
    },
    {
      "line": 49,
      "text": "def test_mmd_fallback_mapping():"
    },
    {
      "line": 62,
      "text": "def test_mixed_model_avoids_duplicate_driving():"
    },
    {
      "line": 83,
      "text": "def test_expanded_mmd_rule_coverage():"
    },
    {
      "line": 102,
      "text": "def test_compose_values():"
    },
    {
      "line": 127,
      "text": "def test_direct_arkit_eyes_are_exclusive():"
    },
    {
      "line": 142,
      "text": "def main():"
    }
  ]
}

```text

"""Pure-Python unit tests for ARKit -> MMD face mapping."""

import importlib.util
import os
import sys

import numpy as np


DEV = os.path.dirname(os.path.abspath(__file__))
module_path = os.path.join(DEV, "blendcap_motion_bridge", "face_mapper.py")
spec = importlib.util.spec_from_file_location("face_mapper", module_path)
face_mapper = importlib.util.module_from_spec(spec)
sys.modules["face_mapper"] = face_mapper
spec.loader.exec_module(face_mapper)


ARKIT_NAMES = [
    "BrowInnerUp", "BrowDownLeft", "BrowDownRight", "BrowOuterUpLeft",
    "BrowOuterUpRight", "eyeLookUpLeft", "eyeLookUpRight",
    "eyeLookDownLeft", "eyeLookDownRight", "eyeLookInLeft",
    "eyeLookInRight", "eyeLookOutLeft", "eyeLookOutRight",
    "eyeBlinkLeft", "eyeBlinkRight", "eyeSquintLeft", "eyeSquintRight",
    "eyeWideLeft", "eyeWideRight", "cheekPuff", "cheekSquintLeft",
    "cheekSquintRight", "noseSneerLeft", "noseSneerRight", "jawOpen",
    "jawForward", "jawLeft", "jawRight", "mouthFunnel", "mouthPucker",
    "mouthLeft", "mouthRight", "mouthRollUpper", "mouthRollLower",
    "mouthShrugUpper", "mouthShrugLower", "mouthClose", "mouthSmileLeft",
    "mouthSmileRight", "mouthFrownLeft", "mouthFrownRight",
    "mouthDimpleLeft", "mouthDimpleRight", "mouthUpperUpLeft",
    "mouthUpperUpRight", "mouthLowerDownLeft", "mouthLowerDownRight",
    "mouthPressLeft", "mouthPressRight", "mouthStretchLeft",
    "mouthStretchRight", "tongueOut",
]

MMD_KEYS = [
    "Basis", "う", "まばたき", "笑い", "笑い1", "ウィンク", "ウィンク右",
    "ウィンク２", "あ", "い", "E", "お",
]


def test_direct_arkit_mapping():
    channels = face_mapper.build_face_channels(ARKIT_NAMES, ARKIT_NAMES)
    assert len(channels) == len(ARKIT_NAMES)
    assert all(c.target == ARKIT_NAMES[i] for i, c in enumerate(channels))
    assert all(c.combine == "sum" for c in channels)


def test_mmd_fallback_mapping():
    channels = face_mapper.build_face_channels(ARKIT_NAMES, MMD_KEYS)
    by_target = {c.target: c for c in channels}
    blink = by_target["まばたき"]
    assert blink.combine == "max"
    blink_sources = {ARKIT_NAMES[i] for i, _w in blink.sources}
    assert blink_sources == {"eyeBlinkLeft", "eyeBlinkRight"}
    assert by_target["あ"].sources[0][0] == ARKIT_NAMES.index("jawOpen")
    assert "E" in by_target
    assert "ウィンク" in by_target
    assert "ウィンク右" in by_target


def test_mixed_model_avoids_duplicate_driving():
    channels = face_mapper.build_face_channels(ARKIT_NAMES, ARKIT_NAMES + MMD_KEYS)
    targets = [c.target for c in channels]
    assert len(targets) == len(set(targets))
    assert "eyeBlinkLeft" in targets
    assert "eyeBlinkRight" in targets
    # Fully covered fallbacks are skipped so the same expression is not
    # double-driven through an ARKit key and its Japanese duplicate.
    assert "まばたき" not in targets
    assert "あ" not in targets
    assert "E" not in targets

    # A fallback still runs when one of its ARKit sources is unavailable.
    missing = [name for name in ARKIT_NAMES if name != "mouthPucker"]
    channels = face_mapper.build_face_channels(missing, ARKIT_NAMES + MMD_KEYS)
    targets = [c.target for c in channels]
    assert len(targets) == len(set(targets))
    assert "う" in targets
    assert "お" in targets


def test_expanded_mmd_rule_coverage():
    white_like_keys = [
        "Basis", "まゆげ1", "まばたき", "笑い", "ウィンク", "ウィンク右",
        "ウィンク２", "ｳｨﾝｸ２右", "じと目", "びっくり", "困る", "上", "下",
        "瞳左", "瞳右", "あ", "い", "う", "え", "お", "口角上げ",
        "口角上げ1", "△",
    ]
    channels = face_mapper.build_face_channels(ARKIT_NAMES, white_like_keys)
    targets = [c.target for c in channels]
    assert len(targets) == len(set(targets))
    for expected in (
        "まゆげ1", "まばたき", "笑い", "ウィンク", "ウィンク右",
        "ウィンク２", "ｳｨﾝｸ２右", "じと目", "びっくり", "困る", "上",
        "下", "瞳左", "瞳右", "あ", "い", "う", "え", "お", "口角上げ",
        "口角上げ1", "△",
    ):
        assert expected in targets, expected


def test_compose_values():
    channels = face_mapper.build_face_channels(ARKIT_NAMES, MMD_KEYS)
    data = np.zeros((4, len(ARKIT_NAMES)), dtype=np.float32)
    data[0, ARKIT_NAMES.index("eyeBlinkLeft")] = 0.7
    data[0, ARKIT_NAMES.index("eyeBlinkRight")] = 0.3
    data[1, ARKIT_NAMES.index("eyeBlinkLeft")] = 0.2
    data[1, ARKIT_NAMES.index("eyeBlinkRight")] = 0.8
    data[2, ARKIT_NAMES.index("eyeBlinkLeft")] = 0.7
    data[2, ARKIT_NAMES.index("eyeBlinkRight")] = 0.65
    data[3, ARKIT_NAMES.index("jawOpen")] = 0.8
    vals = face_mapper.compose_values(data, channels)
    by_target = {c.target: i for i, c in enumerate(channels)}
    assert vals[0, by_target["まばたき"]] == 0.0
    assert vals[0, by_target["ウィンク"]] == 0.7
    assert vals[0, by_target["ウィンク右"]] == 0.0
    assert vals[1, by_target["まばたき"]] == 0.0
    assert vals[1, by_target["ウィンク"]] == 0.0
    assert vals[1, by_target["ウィンク右"]] == 0.8
    assert vals[2, by_target["まばたき"]] == 0.7
    assert vals[2, by_target["ウィンク"]] == 0.0
    assert vals[2, by_target["ウィンク右"]] == 0.0
    assert vals[3, by_target["あ"]] == 0.8
    assert vals.shape == (4, len(channels))


def test_direct_arkit_eyes_are_exclusive():
    channels = face_mapper.build_face_channels(ARKIT_NAMES, ARKIT_NAMES)
    data = np.zeros((2, len(ARKIT_NAMES)), dtype=np.float32)
    data[0, ARKIT_NAMES.index("eyeBlinkLeft")] = 0.8
    data[0, ARKIT_NAMES.index("eyeBlinkRight")] = 0.3
    data[1, ARKIT_NAMES.index("eyeBlinkLeft")] = 0.7
    data[1, ARKIT_NAMES.index("eyeBlinkRight")] = 0.65
    vals = face_mapper.compose_values(data, channels)
    by_target = {c.target: i for i, c in enumerate(channels)}
    assert vals[0, by_target["eyeBlinkLeft"]] == 0.8
    assert vals[0, by_target["eyeBlinkRight"]] == 0.0
    assert vals[1, by_target["eyeBlinkLeft"]] == 0.7
    assert vals[1, by_target["eyeBlinkRight"]] == 0.7


def main():
    test_direct_arkit_mapping()
    test_mmd_fallback_mapping()
    test_mixed_model_avoids_duplicate_driving()
    test_expanded_mmd_rule_coverage()
    test_compose_values()
    test_direct_arkit_eyes_are_exclusive()
    print("FACE_MAPPER_OK")


if __name__ == "__main__":
    main()


```

## Source file: build_zip.py

{
  "path": "build_zip.py",
  "blob_sha": "e2c63ebcbe619022646ba88317e59e4f25c296dd",
  "sha256": "531a1c96dacaa6efab5f825c510d6e336d9b3e4f02639e79bb2a80a366f87016",
  "lines": 41,
  "bytes": 1077,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/aaAarJimedes/BlendCap-Motion-Bridge/blob/61c114894c5a9b5d4bb63badb1d73375f050bab1/build_zip.py",
  "function_signature_candidates": [
    {
      "line": 20,
      "text": "def _version():"
    },
    {
      "line": 29,
      "text": "def main():"
    }
  ]
}

```text

"""Build the installable BlendCap Motion Bridge zip."""

import os
import zipfile


DEV = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(DEV, "blendcap_motion_bridge")
PACKAGE_FILES = (
    "__init__.py",
    "blender_manifest.toml",
    "face_mapper.py",
    "mmd_mapper.py",
    "operators.py",
    "panels.py",
    "preset_writer.py",
)


def _version():
    manifest = os.path.join(PKG, "blender_manifest.toml")
    with open(manifest, encoding="utf-8") as f:
        for line in f:
            if line.startswith("version ="):
                return line.split("=", 1)[1].strip().strip('"')
    raise RuntimeError("version missing from blender_manifest.toml")


def main():
    out = os.path.join(DEV, f"blendcap_motion_bridge-{_version()}.zip")
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        for name in PACKAGE_FILES:
            zf.write(
                os.path.join(PKG, name),
                arcname=os.path.join("blendcap_motion_bridge", name),
            )
    print("ZIP_OK", out)


if __name__ == "__main__":
    main()


```

## Source file: blendcap_motion_bridge/panels.py

{
  "path": "blendcap_motion_bridge/panels.py",
  "blob_sha": "3f2c5eb7724e7e2a94c50678496f41f33b0a55b7",
  "sha256": "f5f3acebb838c0a9858b5db9b53f4e131685cda6d03ef6a39fceb4946b2558bc",
  "lines": 187,
  "bytes": 8843,
  "selection_score": 0,
  "keyword_line_counts": {
    "deform": 3,
    "face": 9
  },
  "url": "https://github.com/aaAarJimedes/BlendCap-Motion-Bridge/blob/61c114894c5a9b5d4bb63badb1d73375f050bab1/blendcap_motion_bridge/panels.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def _section(layout, title, icon):"
    },
    {
      "line": 12,
      "text": "def _draw_mapping_result(layout, scene):"
    },
    {
      "line": 39,
      "text": "def _draw_face(layout, scene):"
    },
    {
      "line": 54,
      "text": "def _draw_vmd_import(layout, scene):"
    },
    {
      "line": 85,
      "text": "def draw(self, context):"
    }
  ]
}

```text

"""Single, workflow-ordered panel for BlendCap Motion Bridge."""

import bpy


def _section(layout, title, icon):
    box = layout.box()
    box.label(text=title, icon=icon)
    return box


def _draw_mapping_result(layout, scene):
    status = layout.row()
    status.alert = scene.blendcap_motion_bridge_status_level == "ERROR"
    status.label(
        text=scene.blendcap_motion_bridge_status,
        icon=(
            "ERROR"
            if status.alert
            else ("CHECKMARK" if scene.blendcap_motion_bridge_status_level == "READY" else "INFO")
        ),
    )
    if not scene.blendcap_motion_bridge_total:
        return

    result = layout.column(align=True)
    result.label(text=f"映射 {scene.blendcap_motion_bridge_matched}/{scene.blendcap_motion_bridge_total}")
    if scene.blendcap_motion_bridge_critical_missing:
        result.label(text=f"关键缺失：{scene.blendcap_motion_bridge_critical_missing}", icon="ERROR")
    unmatched = [item for item in scene.blendcap_motion_bridge_unmatched.split(",") if item]
    if unmatched:
        result.label(text="可选未匹配：", icon="INFO")
        for item in unmatched[:6]:
            result.label(text=item)
        if len(unmatched) > 6:
            result.label(text=f"…另有 {len(unmatched) - 6} 项")


def _draw_face(layout, scene):
    face = _section(layout, "3 · MMD 表情", "SHAPEKEY_DATA")
    face.prop(scene, "blendcap_motion_bridge_face_npz", text="表情数据")
    face.prop_search(
        scene,
        "blendcap_motion_bridge_face_mesh",
        bpy.data,
        "objects",
        text="目标网格",
    )
    face.operator("blendcap_motion_bridge.apply_face", text="应用表情", icon="SHAPEKEY_DATA")
    if scene.blendcap_motion_bridge_face_total:
        face.label(text=f"匹配 {scene.blendcap_motion_bridge_face_matched}/{scene.blendcap_motion_bridge_face_total}")


def _draw_vmd_import(layout, scene):
    vmd = _section(layout, "4 · VMD 与烘焙后处理", "CONSTRAINT_BONE")
    vmd.prop_search(
        scene,
        "blendcap_motion_bridge_vmd_target",
        bpy.data,
        "objects",
        text="MMD 骨架",
    )
    vmd.label(text="导入 VMD 后", icon="IMPORT")
    row = vmd.row(align=True)
    row.operator("blendcap_motion_bridge.disable_leg_overrides", text="关闭腿部覆盖")
    row.operator("blendcap_motion_bridge.restore_leg_overrides", text="恢复腿 IK")
    vmd.separator()
    vmd.label(text="烘焙动作 / 姿态后", icon="RENDER_ANIMATION")
    vmd.operator("blendcap_motion_bridge.relink_leg_deform", text="重连腿部形变", icon="CONSTRAINT_BONE")
    if scene.blendcap_motion_bridge_vmd_ik_disabled:
        vmd.label(text=f"已关闭腿 IK：{scene.blendcap_motion_bridge_vmd_ik_disabled}", icon="INFO")
    if scene.blendcap_motion_bridge_vmd_cancel_disabled:
        vmd.label(text=f"已关闭腰取消：{scene.blendcap_motion_bridge_vmd_cancel_disabled}", icon="INFO")
    if scene.blendcap_motion_bridge_vmd_deform_relinked:
        vmd.label(text=f"已重连腿形变：{scene.blendcap_motion_bridge_vmd_deform_relinked}", icon="CHECKMARK")


class BCMB_PT_main(bpy.types.Panel):
    bl_label = "BlendCap Motion Bridge"
    bl_idname = "BCMB_PT_main"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "BlendCap"

    def draw(self, context):
        layout = self.layout
        layout.use_property_split = True
        layout.use_property_decorate = False
        scene = context.scene
        blendcap_ready = hasattr(bpy.types.Scene, "blendcap_retarget_source")

        source = _section(layout, "1 · 动作来源与角色", "ARMATURE_DATA")
        if blendcap_ready:
            source.prop(scene, "blendcap_retarget_source", text="BVH 来源")
            source.prop(scene, "blendcap_retarget_target", text="MMD 目标")
            source.operator("blendcap_motion_bridge.auto_select", text="自动识别来源与目标", icon="VIEWZOOM")
        else:
            warning = source.row()
            warning.alert = True
            warning.label(text="请先启用 BlendCap", icon="ERROR")
        source.label(text="传统 BlendCap BVH；SOMA 请使用 Proscenium Motion Bridge", icon="INFO")

        retarget = _section(layout, "2 · 动作重定向", "ACTION")
        preroll = retarget.box()
        preroll.prop(scene, "blendcap_motion_bridge_preroll_enabled")
        if scene.blendcap_motion_bridge_preroll_enabled:
            settings = preroll.column()
            settings.enabled = not scene.blendcap_motion_bridge_preroll_pending
            settings.prop(scene, "blendcap_motion_bridge_preroll_pose_source")
            if scene.blendcap_motion_bridge_preroll_pose_source == "ACTION":
                settings.prop(scene, "blendcap_motion_bridge_preroll_pose_action")
                settings.prop(scene, "blendcap_motion_bridge_preroll_pose_frame")
            timing = settings.row(align=True)
            timing.prop(scene, "blendcap_motion_bridge_preroll_hold_frames")
            timing.prop(scene, "blendcap_motion_bridge_preroll_transition_frames")
            total = (
                scene.blendcap_motion_bridge_preroll_hold_frames
                + scene.blendcap_motion_bridge_preroll_transition_frames
            )
            settings.label(text=f"负帧预滚动共 {total} 帧；正式动作时间码不移动", icon="TIME")
            if scene.blendcap_motion_bridge_preroll_pending:
                preroll.separator()
                preroll.label(
                    text=(
                        f"预滚动：{scene.blendcap_motion_bridge_preroll_start}"
                        f"～{scene.blendcap_motion_bridge_preroll_motion_start - 1}"
                    ),
                    icon="PREVIEW_RANGE",
                )
                preroll.label(
                    text=f"正式动作首帧：{scene.blendcap_motion_bridge_preroll_motion_start}",
                    icon="KEYFRAME_HLT",
                )
                run_preroll = preroll.column()
                run_preroll.scale_y = 1.15
                run_preroll.operator(
                    "blendcap_motion_bridge.run_preroll",
                    text="运行负帧预滚动",
                    icon="PHYSICS",
                )
                if scene.blendcap_motion_bridge_preroll_simulated:
                    preroll.label(text="预滚动已求值；现在烘焙裙发物理", icon="CHECKMARK")
                confirm = preroll.row()
                confirm.alert = not scene.blendcap_motion_bridge_preroll_cleanup_confirmed
                confirm.prop(scene, "blendcap_motion_bridge_preroll_cleanup_confirmed")
                cleanup = preroll.column()
                cleanup.enabled = scene.blendcap_motion_bridge_preroll_cleanup_confirmed
                cleanup.operator(
                    "blendcap_motion_bridge.cleanup_preroll",
                    text="完成并清理预滚动",
                    icon="TRASH",
                )
                preroll.label(text="清理只删除首帧之前的键，不移动正式动作", icon="LOCKED")
        run = retarget.column()
        run.enabled = blendcap_ready and not scene.blendcap_motion_bridge_preroll_pending
        run.scale_y = 1.4
        run.operator("blendcap_motion_bridge.quick_retarget", text="自动准备并安全重定向", icon="PLAY")
        retarget.label(text="内存映射 · 独立 Action · 不写 preset", icon="LOCKED")
        _draw_mapping_result(retarget, scene)

        retarget.separator()
        retarget.label(text="手动映射与 Preset（可选）", icon="PREFERENCES")
        manual = retarget.column()
        manual.enabled = blendcap_ready
        row = manual.row(align=True)
        row.operator("blendcap_motion_bridge.detect", text="只检查")
        row.operator("blendcap_motion_bridge.prepare_in_memory", text="载入内存表")
        manual.operator("blendcap_motion_bridge.write_preset", text="原子保存 Preset", icon="FILE_TICK")
        retarget.label(text="同名旧文件保留单份 .bak", icon="LOCKED")
        if scene.blendcap_motion_bridge_last_preset:
            retarget.label(text=f"Preset：{scene.blendcap_motion_bridge_last_preset}", icon="FILE")
        if scene.blendcap_motion_bridge_last_backup:
            retarget.label(text=f"备份：{scene.blendcap_motion_bridge_last_backup}", icon="RECOVER_LAST")

        if (
            scene.blendcap_motion_bridge_previous_state_available
            or scene.blendcap_motion_bridge_constraint_snapshot
            or scene.blendcap_motion_bridge_previous_table_json
        ):
            retarget.separator()
            retarget.operator("blendcap_motion_bridge.restore_previous_state", icon="LOOP_BACK")

        _draw_face(layout, scene)
        _draw_vmd_import(layout, scene)


CLASSES = (BCMB_PT_main,)


```

# Facial rendering source scan: digitalp/arkit-blendshape-tool

Repository: https://github.com/digitalp/arkit-blendshape-tool

Pinned source revision: 0adb9a2c65550e2f9bcb67013ced918aa6b23733

Discovery topic: ARKit blendshape

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"blendshape": ["__pycache__/blendshape_names.cpython-314.pyc", "blendshape_names.py"]}

## Source file: blendshape_names.py

{
  "path": "blendshape_names.py",
  "blob_sha": "335623700af1f5f1b1f4a32fb4afd1eb015b5a46",
  "sha256": "ce325555441175c413e82926517494824793356e8b69c978587e262692d93f2b",
  "lines": 105,
  "bytes": 2420,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 20,
    "blendshape": 4,
    "jaw": 4,
    "mouth": 25,
    "teeth": 2,
    "tongue": 1,
    "rig": 25,
    "face": 3
  },
  "url": "https://github.com/digitalp/arkit-blendshape-tool/blob/0adb9a2c65550e2f9bcb67013ced918aa6b23733/blendshape_names.py",
  "function_signature_candidates": []
}

```text

"""
ARKit 52 blendshape names and Oculus 15 viseme names.
"""

ARKIT_BLENDSHAPES = [
    "eyeBlinkLeft",
    "eyeLookDownLeft",
    "eyeLookInLeft",
    "eyeLookOutLeft",
    "eyeLookUpLeft",
    "eyeSquintLeft",
    "eyeWideLeft",
    "eyeBlinkRight",
    "eyeLookDownRight",
    "eyeLookInRight",
    "eyeLookOutRight",
    "eyeLookUpRight",
    "eyeSquintRight",
    "eyeWideRight",
    "jawForward",
    "jawLeft",
    "jawRight",
    "jawOpen",
    "mouthClose",
    "mouthFunnel",
    "mouthPucker",
    "mouthLeft",
    "mouthRight",
    "mouthSmileLeft",
    "mouthSmileRight",
    "mouthFrownLeft",
    "mouthFrownRight",
    "mouthDimpleLeft",
    "mouthDimpleRight",
    "mouthStretchLeft",
    "mouthStretchRight",
    "mouthRollLower",
    "mouthRollUpper",
    "mouthShrugLower",
    "mouthShrugUpper",
    "mouthPressLeft",
    "mouthPressRight",
    "mouthLowerDownLeft",
    "mouthLowerDownRight",
    "mouthUpperUpLeft",
    "mouthUpperUpRight",
    "browDownLeft",
    "browDownRight",
    "browInnerUp",
    "browOuterUpLeft",
    "browOuterUpRight",
    "cheekPuff",
    "cheekSquintLeft",
    "cheekSquintRight",
    "noseSneerLeft",
    "noseSneerRight",
    "tongueOut",
]

OCULUS_VISEMES = [
    "viseme_sil",
    "viseme_PP",
    "viseme_FF",
    "viseme_TH",
    "viseme_DD",
    "viseme_kk",
    "viseme_CH",
    "viseme_SS",
    "viseme_nn",
    "viseme_RR",
    "viseme_aa",
    "viseme_E",
    "viseme_I",
    "viseme_O",
    "viseme_U",
]

# Extra blendshapes TalkingHead auto-generates from ARKit if missing,
# but including them explicitly ensures maximum compatibility.
TALKINGHEAD_EXTRAS = [
    "mouthOpen",
    "mouthSmile",
    "eyesClosed",
    "eyesLookUp",
    "eyesLookDown",
]

ALL_BLENDSHAPES = ARKIT_BLENDSHAPES + OCULUS_VISEMES + TALKINGHEAD_EXTRAS

# Bone names TalkingHead requires (Mixamo-compatible skeleton)
REQUIRED_BONES = [
    "Hips", "Spine", "Spine1", "Spine2", "Neck", "Head",
    "LeftEye", "RightEye",
    "LeftShoulder", "LeftArm", "LeftForeArm", "LeftHand",
    "RightShoulder", "RightArm", "RightForeArm", "RightHand",
    "LeftUpLeg", "LeftLeg", "LeftFoot",
    "RightUpLeg", "RightLeg", "RightFoot",
]

# Mesh name patterns that should receive morph targets
# (TalkingHead expects targets on all face-related meshes)
FACE_MESH_KEYWORDS = [
    "head", "face", "teeth", "eyeleft", "eyeright",
    "eye_left", "eye_right", "wolf3d_head", "wolf3d_teeth",
]


```

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "f9c46164fd85d7d71087a2c815073a607d5200de",
  "sha256": "1b67d15f8ab5447a4b8d95d623d8a1df44c532a286d09c99215a56ae91492335",
  "lines": 259,
  "bytes": 8720,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 22,
    "mouth": 4,
    "gum": 9,
    "face": 34
  },
  "url": "https://github.com/digitalp/arkit-blendshape-tool/blob/0adb9a2c65550e2f9bcb67013ced918aa6b23733/main.py",
  "function_signature_candidates": [
    {
      "line": 37,
      "text": "def cmd_inspect(args):"
    },
    {
      "line": 94,
      "text": "def cmd_transfer(args):"
    },
    {
      "line": 203,
      "text": "def main():"
    }
  ]
}

```text

#!/usr/bin/env python3
"""
ARKit Blendshape + Oculus Viseme Injector for GLB Avatars

Transfers ARKit 52 blendshapes, 15 Oculus visemes, and 5 TalkingHead
extras from a reference GLB onto a target GLB.

Usage:
    python main.py transfer --reference ref.glb --target target.glb --output output.glb
    python main.py inspect --input model.glb
"""

import argparse
import sys

import numpy as np

from blendshape_names import (
    ARKIT_BLENDSHAPES, OCULUS_VISEMES, TALKINGHEAD_EXTRAS, ALL_BLENDSHAPES,
)
from glb_utils import (
    load_glb,
    save_glb,
    find_face_mesh,
    find_all_face_meshes,
    get_accessor_data,
    get_existing_morph_target_names,
    get_morph_target_data,
    validate_skeleton,
)
from transfer import transfer_all_blendshapes
from inject import inject_morph_targets
from extras import synthesize_extras
from skeleton_fix import add_missing_bones


def cmd_inspect(args):
    """Inspect a GLB file for mesh info and existing morph targets."""
    gltf = load_glb(args.input)

    print(f"\nFile: {args.input}")
    print(f"Meshes: {len(gltf.meshes)}")
    print(f"Nodes: {len(gltf.nodes)}")

    # Skeleton validation
    skel = validate_skeleton(gltf)
    if skel["valid"]:
        print(f"Skeleton: OK ({len(skel['present'])} required bones found)")
    else:
        print(f"Skeleton: INCOMPLETE — missing {len(skel['missing'])} bones:")
        for b in skel["missing"]:
            print(f"    - {b}")

    # Face meshes
    all_face = find_all_face_meshes(gltf)
    if all_face:
        print(f"Face meshes: {len(all_face)}")
        for mi, pi in all_face:
            print(f"    {gltf.meshes[mi].name}")

    print()
    for mi, mesh in enumerate(gltf.meshes):
        print(f"  Mesh {mi}: '{mesh.name or '(unnamed)'}'")
        names = get_existing_morph_target_names(gltf, mi)

        for pi, prim in enumerate(mesh.primitives):
            vcount = 0
            if prim.attributes.POSITION is not None:
                vcount = gltf.accessors[prim.attributes.POSITION].count

            n_targets = len(prim.targets) if prim.targets else 0
            print(f"    Primitive {pi}: {vcount} vertices, "
                  f"{n_targets} morph targets")

            if names:
                arkit = [n for n in names if n in ARKIT_BLENDSHAPES]
                visemes = [n for n in names if n in OCULUS_VISEMES]
                extras = [n for n in names if n in TALKINGHEAD_EXTRAS]
                other = [n for n in names if n not in ALL_BLENDSHAPES]

                print(f"      ARKit: {len(arkit)}/52, "
                      f"Visemes: {len(visemes)}/15, "
                      f"Extras: {len(extras)}/5")
                if other:
                    print(f"      Other: {', '.join(other)}")

    face = find_face_mesh(gltf)
    if face:
        mi, pi = face
        print(f"\n  Primary face mesh: Mesh {mi} "
              f"('{gltf.meshes[mi].name or '(unnamed)'}')")


def cmd_transfer(args):
    """Transfer blendshapes from reference to target GLB."""
    print(f"Loading reference: {args.reference}")
    ref_gltf = load_glb(args.reference)

    print(f"Loading target: {args.target}")
    target_gltf = load_glb(args.target)

    # Validate target skeleton
    skel = validate_skeleton(target_gltf)
    if not skel["valid"]:
        print(f"\nWARNING: Target is missing {len(skel['missing'])} bones "
              f"required by TalkingHead:")
        for b in skel["missing"]:
            print(f"    - {b}")
        print("The output may not work in TalkingHead.\n")

    # Find reference face mesh
    ref_face = find_face_mesh(ref_gltf, args.face_mesh_ref)
    if ref_face is None:
        print("ERROR: Could not find face mesh in reference GLB.")
        print("Available meshes:")
        for i, m in enumerate(ref_gltf.meshes):
            print(f"  {i}: '{m.name}'")
        sys.exit(1)

    ref_mi, ref_pi = ref_face
    print(f"\nReference face: '{ref_gltf.meshes[ref_mi].name}'")

    ref_prim = ref_gltf.meshes[ref_mi].primitives[ref_pi]
    ref_positions = get_accessor_data(ref_gltf, ref_prim.attributes.POSITION)
    ref_blendshapes = get_morph_target_data(ref_gltf, ref_mi, ref_pi)

    if not ref_blendshapes:
        print("ERROR: Reference GLB has no morph targets on the face mesh.")
        sys.exit(1)

    print(f"Reference: {ref_positions.shape[0]} verts, "
          f"{len(ref_blendshapes)} blendshapes")

    # Fix skeleton: add missing bones
    target_gltf, added_bones = add_missing_bones(target_gltf)
    if added_bones:
        print(f"Added missing bones: {', '.join(added_bones)}")

    # Find ALL face-related meshes in target
    target_face_meshes = find_all_face_meshes(target_gltf)
    if not target_face_meshes:
        # Single-mesh avatar (like Avaturn T1) — use primary face mesh
        target_face = find_face_mesh(target_gltf, args.face_mesh_target)
        if target_face is None:
            print("ERROR: Could not find face mesh in target GLB.")
            for i, m in enumerate(target_gltf.meshes):
                print(f"  {i}: '{m.name}'")
            sys.exit(1)
        target_face_meshes = [target_face]

    print(f"Target face meshes: {len(target_face_meshes)}")
    for mi, pi in target_face_meshes:
        print(f"  '{target_gltf.meshes[mi].name}'")

    # Build canonical blendshape order
    order = []
    if "mouthOpen" in ref_blendshapes:
        order.append("mouthOpen")
    for name in OCULUS_VISEMES:
        if name not in order:
            order.append(name)
    if "mouthSmile" not in order:
        order.append("mouthSmile")
    for name in ARKIT_BLENDSHAPES:
        if name not in order:
            order.append(name)
    for name in TALKINGHEAD_EXTRAS:
        if name not in order:
            order.append(name)
    for name in ref_blendshapes:
        if name not in order:
            order.append(name)

    # Transfer and inject onto each face mesh
    for tgt_mi, tgt_pi in target_face_meshes:
        tgt_prim = target_gltf.meshes[tgt_mi].primitives[tgt_pi]
        tgt_positions = get_accessor_data(
            target_gltf, tgt_prim.attributes.POSITION
        )
        mesh_name = target_gltf.meshes[tgt_mi].name or "(unnamed)"
        print(f"\nTransferring to '{mesh_name}' ({tgt_positions.shape[0]} verts)...")

        transferred = transfer_all_blendshapes(
            ref_positions, tgt_positions, ref_blendshapes,
            max_distance=args.max_distance,
            falloff_distance=args.falloff_distance,
        )

        # Synthesize extra blendshapes
        extras = synthesize_extras(transferred)
        transferred.update(extras)

        print(f"  Injecting {len(order)} morph targets...")
        target_gltf = inject_morph_targets(
            target_gltf, tgt_mi, tgt_pi, transferred, order
        )

    save_glb(target_gltf, args.output)
    print(f"\nSaved: {args.output}")
    print("Done.")


def main():
    parser = argparse.ArgumentParser(
        description="Add ARKit blendshapes and Oculus visemes to GLB avatars"
    )
    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    p_inspect = subparsers.add_parser(
        "inspect", help="Inspect a GLB for mesh info and morph targets"
    )
    p_inspect.add_argument("--input", "-i", required=True, help="Input GLB file")

    p_transfer = subparsers.add_parser(
        "transfer",
        help="Transfer blendshapes from a reference GLB to a target GLB",
    )
    p_transfer.add_argument(
        "--reference", "-r", required=True,
        help="Reference GLB with existing ARKit blendshapes",
    )
    p_transfer.add_argument(
        "--target", "-t", required=True,
        help="Target GLB without blendshapes",
    )
    p_transfer.add_argument(
        "--output", "-o", required=True,
        help="Output GLB path",
    )
    p_transfer.add_argument(
        "--max-distance", type=float, default=0.15,
        help="Max distance for vertex correspondence (default: 0.15)",
    )
    p_transfer.add_argument(
        "--falloff-distance", type=float, default=0.08,
        help="Distance falloff for displacement blending (default: 0.08)",
    )
    p_transfer.add_argument(
        "--face-mesh-ref", type=str, default=None,
        help="Name of the face mesh in reference (auto-detected if omitted)",
    )
    p_transfer.add_argument(
        "--face-mesh-target", type=str, default=None,
        help="Name of the face mesh in target (auto-detected if omitted)",
    )

    args = parser.parse_args()

    if args.command == "inspect":
        cmd_inspect(args)
    elif args.command == "transfer":
        cmd_transfer(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()


```

## Source file: extras.py

{
  "path": "extras.py",
  "blob_sha": "242150a3cd870930e60d198b8380b9e4ff00ff60",
  "sha256": "6a87b53b102e413b05ceca00f57dfbbfe247eb515993c4bf5d0bb1fbc941e24f",
  "lines": 64,
  "bytes": 2357,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 18,
    "blendshape": 22,
    "jaw": 3,
    "mouth": 10,
    "rig": 16
  },
  "url": "https://github.com/digitalp/arkit-blendshape-tool/blob/0adb9a2c65550e2f9bcb67013ced918aa6b23733/extras.py",
  "function_signature_candidates": [
    {
      "line": 16,
      "text": "def synthesize_extras(blendshapes: dict) -> dict:"
    }
  ]
}

```text

"""
Synthesize extra blendshapes that TalkingHead needs from ARKit blendshapes.

TalkingHead expects these 5 additional morph targets beyond the standard
52 ARKit + 15 Oculus visemes:
  - mouthOpen:    derived from jawOpen
  - mouthSmile:   average of mouthSmileLeft + mouthSmileRight
  - eyesClosed:   average of eyeBlinkLeft + eyeBlinkRight
  - eyesLookUp:   average of eyeLookUpLeft + eyeLookUpRight
  - eyesLookDown: average of eyeLookDownLeft + eyeLookDownRight
"""

import numpy as np


def synthesize_extras(blendshapes: dict) -> dict:
    """
    Generate the 5 extra blendshapes from existing ARKit blendshapes.
    Only creates them if they don't already exist in the input dict.

    Args:
        blendshapes: dict of {name: (V, 3) displacement array}

    Returns:
        dict of newly synthesized blendshapes (may be empty if all exist)
    """
    extras = {}

    # Get vertex count from any existing blendshape
    if not blendshapes:
        return extras
    sample = next(iter(blendshapes.values()))
    V = sample.shape[0]
    zero = np.zeros((V, 3), dtype=np.float32)

    # mouthOpen: same as jawOpen
    if "mouthOpen" not in blendshapes:
        extras["mouthOpen"] = blendshapes.get("jawOpen", zero).copy()

    # mouthSmile: average of left + right smile
    if "mouthSmile" not in blendshapes:
        left = blendshapes.get("mouthSmileLeft", zero)
        right = blendshapes.get("mouthSmileRight", zero)
        extras["mouthSmile"] = ((left + right) * 0.5).astype(np.float32)

    # eyesClosed: average of left + right blink
    if "eyesClosed" not in blendshapes:
        left = blendshapes.get("eyeBlinkLeft", zero)
        right = blendshapes.get("eyeBlinkRight", zero)
        extras["eyesClosed"] = ((left + right) * 0.5).astype(np.float32)

    # eyesLookUp: average of left + right look up
    if "eyesLookUp" not in blendshapes:
        left = blendshapes.get("eyeLookUpLeft", zero)
        right = blendshapes.get("eyeLookUpRight", zero)
        extras["eyesLookUp"] = ((left + right) * 0.5).astype(np.float32)

    # eyesLookDown: average of left + right look down
    if "eyesLookDown" not in blendshapes:
        left = blendshapes.get("eyeLookDownLeft", zero)
        right = blendshapes.get("eyeLookDownRight", zero)
        extras["eyesLookDown"] = ((left + right) * 0.5).astype(np.float32)

    return extras


```

## Source file: inject.py

{
  "path": "inject.py",
  "blob_sha": "c19bc49aaacbd76cd16992434374f0535365b8e7",
  "sha256": "2d11bbbafc3d8e9e4c5fda9773d1bfacf196a172e5adfb721b8825de04ad0582",
  "lines": 131,
  "bytes": 3498,
  "selection_score": 0,
  "keyword_line_counts": {
    "blendshape": 8
  },
  "url": "https://github.com/digitalp/arkit-blendshape-tool/blob/0adb9a2c65550e2f9bcb67013ced918aa6b23733/inject.py",
  "function_signature_candidates": [
    {
      "line": 19,
      "text": "def _pack_vec3_array(data: np.ndarray) -> bytes:"
    },
    {
      "line": 24,
      "text": "def _compute_bounds(data: np.ndarray):"
    },
    {
      "line": 31,
      "text": "def _align_to_4(offset: int) -> int:"
    },
    {
      "line": 36,
      "text": "def inject_morph_targets("
    }
  ]
}

```text

"""
Inject morph targets (blendshapes) into a GLB file.

This module handles the low-level glTF binary manipulation needed to
add new morph targets to an existing mesh primitive.
"""

import numpy as np
from pygltflib import (
    GLTF2,
    Accessor,
    Attributes,
    BufferView,
)

FLOAT = 5126


def _pack_vec3_array(data: np.ndarray) -> bytes:
    """Pack an (N, 3) float32 array into raw bytes."""
    return data.astype(np.float32).tobytes()


def _compute_bounds(data: np.ndarray):
    """Compute min/max for an (N, 3) array."""
    mins = data.min(axis=0).tolist()
    maxs = data.max(axis=0).tolist()
    return mins, maxs


def _align_to_4(offset: int) -> int:
    """Round up to next 4-byte boundary (glTF spec requirement)."""
    return (offset + 3) & ~3


def inject_morph_targets(
    gltf: GLTF2,
    mesh_index: int,
    prim_index: int,
    blendshapes: dict,
    blendshape_order: list = None,
) -> GLTF2:
    """
    Inject morph target displacements into a GLB mesh primitive.

    Preserves all existing binary data and appends new morph target
    data with proper 4-byte alignment per the glTF spec.
    """
    if blendshape_order is None:
        blendshape_order = list(blendshapes.keys())

    mesh = gltf.meshes[mesh_index]
    prim = mesh.primitives[prim_index]

    # Get current binary blob
    blob = gltf.binary_blob()
    if blob is None:
        blob = b""

    # Pad existing blob to 4-byte alignment before appending
    aligned_start = _align_to_4(len(blob))
    padding = aligned_start - len(blob)

    new_data = bytearray(b'\x00' * padding)
    new_targets = []
    target_names = []

    for name in blendshape_order:
        if name not in blendshapes:
            continue

        disp = blendshapes[name]
        raw = _pack_vec3_array(disp)
        mins, maxs = _compute_bounds(disp)

        # Current offset into the combined buffer
        current_offset = len(blob) + len(new_data)

        # Ensure 4-byte alignment for this buffer view
        aligned_offset = _align_to_4(current_offset)
        if aligned_offset > current_offset:
            new_data.extend(b'\x00' * (aligned_offset - current_offset))

        bv_offset = len(blob) + len(new_data)

        # Create buffer view
        bv_index = len(gltf.bufferViews)
        bv = BufferView(
            buffer=0,
            byteOffset=bv_offset,
            byteLength=len(raw),
        )
        gltf.bufferViews.append(bv)

        # Create accessor
        acc_index = len(gltf.accessors)
        acc = Accessor(
            bufferView=bv_index,
            byteOffset=0,
            componentType=FLOAT,
            count=disp.shape[0],
            type="VEC3",
            max=maxs,
            min=mins,
        )
        gltf.accessors.append(acc)

        # pygltflib expects Attributes objects for morph targets
        target = Attributes(POSITION=acc_index)
        new_targets.append(target)
        target_names.append(name)

        new_data.extend(raw)

    # Set the targets on the primitive
    prim.targets = new_targets

    # Set default weights (all zero)
    mesh.weights = [0.0] * len(new_targets)

    # Store target names in mesh extras (standard glTF convention)
    if mesh.extras is None:
        mesh.extras = {}
    mesh.extras["targetNames"] = target_names

    # Update the buffer size and binary blob
    combined = bytearray(blob) + new_data
    gltf.buffers[0].byteLength = len(combined)
    gltf.set_binary_blob(bytes(combined))

    return gltf


```

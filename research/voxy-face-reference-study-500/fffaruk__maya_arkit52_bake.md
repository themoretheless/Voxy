# Facial rendering source scan: fffaruk/maya_arkit52_bake

Repository: https://github.com/fffaruk/maya_arkit52_bake

Pinned source revision: 0c16f4cf9a84a8417ae9735a5344b7df35e1209f

Discovery topic: ARKit blendshape

Description: Convert Metahuman Control Rig Driven 52arkit To 52arkit Blendshapes on its head mesh.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: maya_arkit52_bake.py

{
  "path": "maya_arkit52_bake.py",
  "blob_sha": "f8f3237f20c37e2ee467666ad091f5f61305cdf2",
  "sha256": "ff6dbe316324af6d2d324f36f0e1b8d22c6d7d838d5d04a73d6346d53aad039d",
  "lines": 751,
  "bytes": 27390,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 16,
    "skin": 11,
    "blendshape": 44,
    "jaw": 4,
    "mouth": 28,
    "tongue": 3,
    "deform": 9,
    "rig": 22,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/fffaruk/maya_arkit52_bake/blob/0c16f4cf9a84a8417ae9735a5344b7df35e1209f/maya_arkit52_bake.py",
  "function_signature_candidates": [
    {
      "line": 64,
      "text": "def _get_transform(node):"
    },
    {
      "line": 73,
      "text": "def _is_mesh_transform(node):"
    },
    {
      "line": 80,
      "text": "def _duplicate_baked_mesh(src_transform, name):"
    },
    {
      "line": 94,
      "text": "def _duplicate_target_with_transfer(source_transform, target_transform, name):"
    },
    {
      "line": 132,
      "text": "def _layout_in_grid("
    },
    {
      "line": 175,
      "text": "def _sanitize_node_name(desired_name):"
    },
    {
      "line": 190,
      "text": "def _ensure_time_unit_24fps():"
    },
    {
      "line": 197,
      "text": "def bake_arkit_to_blendshapes("
    },
    {
      "line": 375,
      "text": "def quick_run():"
    },
    {
      "line": 410,
      "text": "def quick_run_multi():"
    },
    {
      "line": 458,
      "text": "def _list_blendshape_aliases(bs_node):"
    },
    {
      "line": 467,
      "text": "def build_arkit_controller(controller_name=\"ARKit52_CTRL\", source_bs_node=None):"
    },
    {
      "line": 498,
      "text": "def connect_controller_to_blendshape(controller, bs_node):"
    },
    {
      "line": 530,
      "text": "def wire_selected_to_controller(controller_name=\"ARKit52_CTRL\"):"
    },
    {
      "line": 560,
      "text": "def _find_arkit_bs_on_mesh(transform):"
    },
    {
      "line": 573,
      "text": "def wire_selected_to_first_bs():"
    },
    {
      "line": 621,
      "text": "def _get_blendshape_indices(bs_node):"
    },
    {
      "line": 635,
      "text": "def bake_weights_to_targets(start_frame=None, end_frame=None, driver_mesh=None):"
    },
    {
      "line": 698,
      "text": "def enforce_deformer_order(mesh_list=None):"
    },
    {
      "line": 740,
      "text": "def export_prep(driver_mesh=None, start_frame=None, end_frame=None):"
    }
  ]
}

```text

import maya.cmds as cmds
import math


ARKIT52_NAMES = [
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

# Default frame indices: consecutive 1..52 (skip frame 0)
ARKIT52_DEFAULT_FRAMES = list(range(1, 53))


def _get_transform(node):
    if not node:
        return None
    if cmds.nodeType(node) == "transform":
        return node
    parents = cmds.listRelatives(node, parent=True, fullPath=False) or []
    return parents[0] if parents else None


def _is_mesh_transform(node):
    if not node or cmds.nodeType(node) != "transform":
        return False
    shapes = cmds.listRelatives(node, shapes=True, noIntermediate=True) or []
    return any(cmds.nodeType(s) == "mesh" for s in shapes)


def _duplicate_baked_mesh(src_transform, name):
    cmds.refresh(force=True)
    dup = cmds.duplicate(src_transform, name=name, smartTransform=False)[0]
    # Delete history to freeze deformations at the current frame
    cmds.delete(dup, constructionHistory=True)
    # Unlock transforms for safe layout
    for axis in ("t", "r", "s"):
        for c in ("x", "y", "z"):
            attr = f"{dup}.{axis}{c}"
            if cmds.getAttr(attr, lock=True):
                cmds.setAttr(attr, lock=False)
    return dup


def _duplicate_target_with_transfer(source_transform, target_transform, name):
    """
    Duplicates the TARGET topology and transfers the current frame deformation
    from SOURCE -> duplicate via transferAttributes (positions only), then bakes history.
    Ensures the baked duplicate matches TARGET topology so it can be added as a
    blendShape target.
    """
    cmds.refresh(force=True)
    dup = cmds.duplicate(target_transform, name=name, smartTransform=False)[0]
    try:
        # Transfer vertex positions in world space using closest point
        # This works across different topologies
        cmds.transferAttributes(
            source_transform,
            dup,
            transferPositions=True,
            transferNormals=False,
            transferUVs=False,
            transferColors=False,
            sampleSpace=3,  # 3 = world space
            searchMethod=3,  # 3 = closest point on surface
            flipUVs=False,
            colorBorders=True,
        )
        # Bake the transferred result by deleting history
        cmds.delete(dup, constructionHistory=True)
    except Exception:
        # If transfer fails, still try to continue with the duplicate as-is
        pass
    # Unlock transforms for safe layout
    for axis in ("t", "r", "s"):
        for c in ("x", "y", "z"):
            attr = f"{dup}.{axis}{c}"
            if cmds.getAttr(attr, lock=True):
                cmds.setAttr(attr, lock=False)
    return dup


def _layout_in_grid(
    nodes,
    columns=10,
    spacing=10.0,
    group_name="ARKit_Poses_GRP",
    plane="XZ",
    aspect_w=16,
    aspect_h=9,
    spacing_x=None,
    spacing_y=None,
    group_offset=(0.0, 0.0, 0.0),
):
    if not nodes:
        return None
    grp = cmds.group(empty=True, name=group_name)
    n = len(nodes)
    cols = int(columns) if columns is not None else 0
    if cols <= 0:
        # Auto-compute columns for requested aspect ratio (width:height)
        ratio = float(aspect_w) / float(aspect_h) if aspect_h else 1.0
        cols = int(math.ceil(math.sqrt(n * ratio)))
        cols = max(cols, 1)
    # Derive per-axis spacing
    sx = float(spacing_x) if spacing_x is not None else float(spacing)
    sy = float(spacing_y) if spacing_y is not None else float(spacing)
    for i, nnode in enumerate(nodes):
        row = i // cols
        col = i % cols
        if str(plane).upper() == "XY":
            t = (col * sx, row * sy, 0.0)
        else:
            t = (col * sx, 0.0, row * sy)
        cmds.xform(nnode, worldSpace=True, translation=t)
        cmds.parent(nnode, grp)
    # Apply group offset
    try:
        gx, gy, gz = group_offset if group_offset else (0.0, 0.0, 0.0)
        cmds.xform(grp, worldSpace=True, translation=(float(gx), float(gy), float(gz)))
    except Exception:
        pass
    return grp


def _sanitize_node_name(desired_name):
    """
    Make sure Maya node name is valid. If it starts with a digit, prefix with 'BS_'.
    Replace invalid characters with underscore.
    """
    if not desired_name:
        return "ARKit52_BS"
    name = "".join(ch if (ch.isalnum() or ch == "_") else "_" for ch in str(desired_name))
    if not name:
        name = "ARKit52_BS"
    if name[0].isdigit():
        name = f"BS_{name}"
    return name


def _ensure_time_unit_24fps():
    try:
        cmds.currentUnit(time="film")  # 24 fps
    except Exception:
        pass


def bake_arkit_to_blendshapes(
    arkit_names=None,
    start_frame=None,
    count=52,
    columns=10,
    spacing=10.0,
    keep_duplicates=True,
    apply_to_second_selection=True,
    blendshape_node_name="ARKit52_BS",
    group_plane="XZ",
    aspect_w=16,
    aspect_h=9,
    frame_indices=None,
    spacing_x=None,
    spacing_y=None,
    group_offset=(0.0, 0.0, 0.0),
    include_neutral=False,
    neutral_frame=0,
    neutral_name="neutral",
    group_name="ARKit_Poses_GRP",
):
    """
    Workflow:
      - Select animated source head mesh first (with mh_arkit_mapping_anim applied)
      - Optionally select a second target head mesh to receive the blendShape
      - This will sample frames [start_frame .. start_frame + count-1], duplicate and freeze each as a baked mesh,
        name them using ARKit names, lay them out in a grid, and build a blendShape on the target.

    Args:
      arkit_names: list of 52 ARKit names (defaults to ARKIT52_NAMES)
      start_frame: frame to start sampling (defaults to playback minTime)
      count: number of frames to capture (default 52)
      columns: grid columns for layout (visual QA)
      spacing: grid spacing in world units
      keep_duplicates: keep baked meshes in scene for QA
      apply_to_second_selection: if True and two nodes selected, apply to second as target; otherwise apply to source
      blendshape_node_name: name for the created blendShape node
    """
    _ensure_time_unit_24fps()

    arkit = list(arkit_names) if arkit_names else list(ARKIT52_NAMES)
    # Build alias list: optional neutral first, then ARKit names in order
    aliases = []
    if include_neutral:
        aliases.append(str(neutral_name) if neutral_name else "neutral")
    aliases.extend(arkit)
    # Determine how many we plan to bake
    if frame_indices:
        planned_targets = len(frame_indices) + (1 if include_neutral else 0)
    else:
        planned_targets = (1 if include_neutral else 0) + (count if count is not None else len(arkit))

    sel = cmds.ls(selection=True, long=False) or []
    if not sel:
        cmds.error("Select the animated source head transform (and optionally the target head) and run again.")
        return

    source = _get_transform(sel[0])
    if not _is_mesh_transform(source):
        cmds.error("First selection must be a mesh transform (animated MetaHuman head).")
        return

    target = None
    if apply_to_second_selection and len(sel) >= 2:
        target = _get_transform(sel[1])
        if not _is_mesh_transform(target):
            cmds.error("Second selection must be a mesh transform (target head).")
            return
    else:
        target = source

    if frame_indices:
        # Use explicit frames for ARKit names only
        if len(frame_indices) < max(0, planned_targets - (1 if include_neutral else 0)):
            cmds.warning("frame_indices shorter than required; missing shapes will reuse last frame.")
    else:
        if start_frame is None:
            start_frame = int(round(cmds.playbackOptions(query=True, min=True)))
        start_frame = int(start_frame)
        end_frame_needed = start_frame + (planned_targets - (1 if include_neutral else 0)) - 1
        scene_max = int(round(cmds.playbackOptions(query=True, max=True)))
        if end_frame_needed > scene_max:
            cmds.warning(
                f"Timeline max ({scene_max}) is less than needed end frame ({end_frame_needed}). Proceeding anyway."
            )

    print(f"[ARKitBake] Source: {source} | Target: {target} | Start: {start_frame} | Count: {count}")

    # Construct list of frames to bake: optional neutral, then ARKit frames
    frames_to_bake = []
    names_to_use = []
    if include_neutral:
        frames_to_bake.append(int(neutral_frame))
        names_to_use.append(aliases[0])
    # ARKit frames
    for i in range(min(len(arkit), planned_targets - (1 if include_neutral else 0))):
        if frame_indices and i < len(frame_indices):
            frames_to_bake.append(int(frame_indices[i]))
        else:
            frames_to_bake.append(int(start_frame + i))
        names_to_use.append(arkit[i])

    baked_transforms = []
    for i, frame in enumerate(frames_to_bake):
        name = names_to_use[i] if i < len(names_to_use) else f"frame_{frame}"
        try:
            cmds.currentTime(frame, edit=True)
            if source == target:
                dup = _duplicate_baked_mesh(source, name)
            else:
                # Different meshes (likely different topology). Bake onto a duplicate
                # of the TARGET and transfer positions from SOURCE at this frame.
                dup = _duplicate_target_with_transfer(source, target, name)
            baked_transforms.append(dup)
            print(f"[ARKitBake] Baked frame {frame} -> {dup}")
        except Exception as exc:
            cmds.warning(f"Failed baking frame {frame}: {exc}")

    if not baked_transforms:
        cmds.error("No baked meshes created. Aborting.")
        return

    # Layout duplicates for quick visual QA
    grp = _layout_in_grid(
        baked_transforms,
        columns=columns,
        spacing=spacing,
        group_name=group_name,
        plane=group_plane,
        aspect_w=aspect_w,
        aspect_h=aspect_h,
        spacing_x=spacing_x,
        spacing_y=spacing_y,
        group_offset=group_offset,
    )
    if grp:
        print(f"[ARKitBake] Laid out {len(baked_transforms)} meshes under group: {grp}")

    # Build blendShape on target
    bs_node_name = _sanitize_node_name(blendshape_node_name)
    if bs_node_name != blendshape_node_name:
        print(f"[ARKitBake] Renamed blendShape node to valid Maya name: {bs_node_name}")
    try:
        bs_node = cmds.blendShape(target, name=bs_node_name, origin="local")[0]
    except Exception:
        # If node exists, append to it
        existing = cmds.ls(bs_node_name, type="blendShape")
        bs_node = existing[0] if existing else cmds.blendShape(target, origin="local")[0]
        if existing:
            try:
                bs_node = cmds.rename(bs_node, bs_node_name)
            except Exception:
                pass

    for i, dup in enumerate(baked_transforms):
        alias = names_to_use[i] if i < len(names_to_use) else f"shape_{i:02d}"
        try:
            cmds.blendShape(bs_node, edit=True, t=(target, i, dup, 1.0))
            # Set user-friendly alias on the weight slot
            try:
                cmds.aliasAttr(alias, f"{bs_node}.w[{i}]")
            except Exception:
                pass
            print(f"[ARKitBake] Added target {alias} at index {i}")
        except Exception as exc:
            cmds.warning(f"Failed adding {dup} to blendShape: {exc}")

    # Optionally delete duplicates (keep grid for QA by default)
    if not keep_duplicates:
        try:
            cmds.delete(baked_transforms)
            print("[ARKitBake] Deleted baked duplicates after creating blendShape.")
        except Exception:
            pass

    print(f"[ARKitBake] Done. BlendShape node: {bs_node} on {target}")


def quick_run():
    """
    Convenience entry point with sensible defaults.
    Usage in Maya Script Editor:
        import sys
        sys.path.append(r"D:/DXG/CG/Maya/Plugins/MH_mh_arkit_mapping_anim_To_BlendShape")
        import maya_arkit52_bake as ab
        ab.quick_run()
    """
    # Omit one ARKit key to fit frames 1..51; here we drop 'mouthClose'
    custom_arkit = [n for n in ARKIT52_NAMES if n != "mouthClose"]
    bake_arkit_to_blendshapes(
        arkit_names=custom_arkit,
        start_frame=0,
        count=51,
        columns=0,  # auto-compute columns for 16:9
        spacing=10.0,
        keep_duplicates=True,
        apply_to_second_selection=True,
        blendshape_node_name="ARKit52_BS",
        group_plane="XY",
        aspect_w=16,
        aspect_h=9,
        # Map 51 ARKit names to frames 1..50 and put the last (tongueOut) at frame 52
        frame_indices=(list(range(1, 51)) + [52]),
        spacing_x=15.0,   # distance on X
        spacing_y=40.0,   # distance on Y (your horizontal view)
        group_offset=(-300.0, 100.0, 0.0),
        include_neutral=True,
        neutral_frame=0,
        neutral_name="neutral",
        group_name="ARKit_Poses_GRP_head",
    )


def quick_run_multi():
    """
    Bake neutral@0 + ARKit 51 (omit mouthClose) for each selected mesh separately.
    Creates unique blendShape node names and unique layout groups per mesh.
    """
    sel = cmds.ls(selection=True, long=False) or []
    if not sel:
        cmds.error("Select one or more animated mesh transforms and run again.")
        return
    custom_arkit = [n for n in ARKIT52_NAMES if n != "mouthClose"]
    frames = list(range(1, 51)) + [52]
    base_offset = (-300.0, 100.0, 0.0)
    offset_step_x = 400.0
    for idx, tr in enumerate(sel):
        tr_xformed = _get_transform(tr)
        if not _is_mesh_transform(tr_xformed):
            cmds.warning(f"Skipping non-mesh selection: {tr}")
            continue
        node_name = _sanitize_node_name(f"ARKit52_BS_{tr_xformed}")
        group_name = _sanitize_node_name(f"ARKit_Poses_GRP_{tr_xformed}")
        # Make this mesh both source and target
        try:
            cmds.select(tr_xformed, r=True)
            bake_arkit_to_blendshapes(
                arkit_names=custom_arkit,
                start_frame=0,
                count=51,
                columns=0,
                spacing=10.0,
                keep_duplicates=True,
                apply_to_second_selection=False,
                blendshape_node_name=node_name,
                group_plane="XY",
                aspect_w=16,
                aspect_h=9,
                frame_indices=frames,
                spacing_x=15.0,
                spacing_y=40.0,
                group_offset=(base_offset[0] + idx * offset_step_x, base_offset[1], base_offset[2]),
                include_neutral=True,
                neutral_frame=0,
                neutral_name="neutral",
                group_name=group_name,
            )
        except Exception as exc:
            cmds.warning(f"Failed on {tr_xformed}: {exc}")


def _list_blendshape_aliases(bs_node):
    pairs = cmds.aliasAttr(bs_node, query=True) or []
    # pairs like [alias0, 'weight[0]', alias1, 'weight[1]', ...]
    aliases = []
    for i in range(0, len(pairs), 2):
        aliases.append(pairs[i])
    return aliases


def build_arkit_controller(controller_name="ARKit52_CTRL", source_bs_node=None):
    """
    Create a central controller transform with one channel per BS alias.
    If source_bs_node is provided, build attributes from its existing aliases
    (e.g., ['neutral', 'eyeBlinkLeft', ..., 'tongueOut']).
    Returns the controller transform name and the alias list.
    """
    ctrl = controller_name
    if not cmds.objExists(ctrl):
        ctrl = cmds.createNode("transform", name=controller_name)
    aliases = []
    if source_bs_node and cmds.objExists(source_bs_node):
        aliases = _list_blendshape_aliases(source_bs_node)
    if not aliases:
        # Fallback: neutral + ARKIT52_NAMES (current profile omits mouthClose elsewhere)
        aliases = ["neutral"] + [n for n in ARKIT52_NAMES]
    # Add attributes if missing
    for a in aliases:
        safe = _sanitize_node_name(a)
        if not cmds.attributeQuery(safe, node=ctrl, exists=True):
            try:
                cmds.addAttr(ctrl, longName=safe, attributeType="double", min=0.0, max=1.0, defaultValue=0.0, keyable=True)
            except Exception:
                # If name collides, append suffix
                try:
                    cmds.addAttr(ctrl, longName=f"{safe}_attr", attributeType="double", min=0.0, max=1.0, defaultValue=0.0, keyable=True)
                except Exception:
                    pass
    return ctrl, aliases


def connect_controller_to_blendshape(controller, bs_node):
    """
    Wire controller.<alias> to bs_node.<alias> for all aliases present on bs_node.
    """
    if not (cmds.objExists(controller) and cmds.objExists(bs_node)):
        cmds.warning("Controller or blendShape node does not exist.")
        return
    pairs = cmds.aliasAttr(bs_node, query=True) or []
    for i in range(0, len(pairs), 2):
        alias = pairs[i]
        safe = _sanitize_node_name(alias)
        # attribute may have been created as exact or with _attr suffix
        src_attr = f"{controller}.{safe}"
        if not cmds.objExists(src_attr):
            alt = f"{controller}.{safe}_attr"
            if cmds.objExists(alt):
                src_attr = alt
            else:
                # Create it on the fly
                try:
                    cmds.addAttr(controller, longName=safe, attributeType="double", min=0.0, max=1.0, defaultValue=0.0, keyable=True)
                except Exception:
                    continue
        dst_attr = f"{bs_node}.{alias}"
        # Avoid double connections
        if not cmds.listConnections(dst_attr, source=True, destination=False):
            try:
                cmds.connectAttr(src_attr, dst_attr, force=True)
            except Exception:
                pass


def wire_selected_to_controller(controller_name="ARKit52_CTRL"):
    """
    For each selected transform, find ARKit blendShape nodes in history (name starts with ARKit52_BS)
    and wire them to the central controller. If controller lacks channels, build from the first BS.
    """
    sel = cmds.ls(selection=True, long=False) or []
    if not sel:
        cmds.error("Select one or more meshes that have ARKit blendShapes and run again.")
        return
    controller = controller_name
    # Find first bs to derive aliases if controller missing
    first_bs = None
    bs_nodes = []
    for tr in sel:
        hist = cmds.listHistory(tr, pruneDagObjects=True) or []
        for h in hist:
            if cmds.nodeType(h) == "blendShape" and h.startswith("ARKit52_BS"):
                bs_nodes.append(h)
                if not first_bs:
                    first_bs = h
                break
    if not bs_nodes:
        cmds.error("No ARKit52_BS blendShape nodes found on selection.")
        return
    if not cmds.objExists(controller):
        controller, _ = build_arkit_controller(controller, source_bs_node=first_bs)
    for bs in bs_nodes:
        connect_controller_to_blendshape(controller, bs)


def _find_arkit_bs_on_mesh(transform):
    """Return the first ARKit blendShape node on a mesh history, preferring names starting with ARKit52_BS."""
    hist = cmds.listHistory(transform, pruneDagObjects=True) or []
    preferred = None
    for h in hist:
        if cmds.nodeType(h) == "blendShape":
            if h.startswith("ARKit52_BS"):
                return h
            if preferred is None:
                preferred = h
    return preferred


def wire_selected_to_first_bs():
    """
    Use the first selected mesh's ARKit52_BS node as the central driver.
    Connect its aliases to identically named aliases on the ARKit52_BS nodes
    of the remaining selected meshes.
    """
    sel = cmds.ls(selection=True, long=False) or []
    if not sel or len(sel) < 2:
        cmds.error("Select at least two meshes: source first, then targets.")
        return
    src_tr = _get_transform(sel[0])
    if not _is_mesh_transform(src_tr):
        cmds.error("First selection must be a mesh transform.")
        return
    src_bs = _find_arkit_bs_on_mesh(src_tr)
    if not src_bs:
        cmds.error("No ARKit blendShape found on the first selected mesh.")
        return
    # Gather source aliases
    src_pairs = cmds.aliasAttr(src_bs, query=True) or []
    src_aliases = [src_pairs[i] for i in range(0, len(src_pairs), 2)]
    # Wire each target
    for tr in sel[1:]:
        tgt_tr = _get_transform(tr)
        if not _is_mesh_transform(tgt_tr):
            cmds.warning(f"Skipping non-mesh: {tr}")
            continue
        tgt_bs = _find_arkit_bs_on_mesh(tgt_tr)
        if not tgt_bs:
            cmds.warning(f"No ARKit blendShape on {tgt_tr}; skipping.")
            continue
        tgt_pairs = cmds.aliasAttr(tgt_bs, query=True) or []
        tgt_aliases = set(tgt_pairs[i] for i in range(0, len(tgt_pairs), 2))
        # Connect matching aliases
        for alias in src_aliases:
            if alias not in tgt_aliases:
                continue
            src_attr = f"{src_bs}.{alias}"
            dst_attr = f"{tgt_bs}.{alias}"
            try:
                # avoid duplicate connections
                existing = cmds.listConnections(dst_attr, source=True, destination=False) or []
                if src_bs not in existing:
                    cmds.connectAttr(src_attr, dst_attr, force=True)
            except Exception:
                pass


def _get_blendshape_indices(bs_node):
    pairs = cmds.aliasAttr(bs_node, query=True) or []
    result = []
    for i in range(0, len(pairs), 2):
        alias = pairs[i]
        plug = pairs[i + 1]  # e.g., 'weight[0]'
        try:
            idx = int(plug.split('[')[1].split(']')[0])
        except Exception:
            idx = i // 2
        result.append((alias, idx))
    return result


def bake_weights_to_targets(start_frame=None, end_frame=None, driver_mesh=None):
    """
    Bake per-mesh ARKit weights by sampling the head's (driver) ARKit52_BS values
    and keying the corresponding weights on each selected mesh's ARKit52_BS.
    After this, FBX export will preserve the keyed morph animation per mesh.

    Select target meshes (including driver if desired). Optionally pass driver_mesh name.
    """
    sel = cmds.ls(selection=True, long=False) or []
    if not sel:
        cmds.error("Select one or more meshes to bake weights on.")
        return
    if start_frame is None:
        start_frame = int(round(cmds.playbackOptions(q=True, min=True)))
    if end_frame is None:
        end_frame = int(round(cmds.playbackOptions(q=True, max=True)))
    # Determine driver
    driver = driver_mesh or sel[0]
    driver_tr = _get_transform(driver)
    driver_bs = _find_arkit_bs_on_mesh(driver_tr)
    if not driver_bs:
        cmds.error("Driver mesh lacks ARKit blendShape.")
        return
    driver_pairs = _get_blendshape_indices(driver_bs)
    # Prepare targets (exclude driver if in sel)
    targets = []
    for tr in sel:
        ttr = _get_transform(tr)
        if not _is_mesh_transform(ttr):
            continue
        if ttr == driver_tr:
            continue
        tbs = _find_arkit_bs_on_mesh(ttr)
        if not tbs:
            cmds.warning(f"Skipping {ttr}: no ARKit blendShape found.")
            continue
        targets.append((ttr, tbs))
    if not targets:
        cmds.warning("No targets to bake.")
        return
    # Bake
    for f in range(int(start_frame), int(end_frame) + 1):
        cmds.currentTime(f, edit=True)
        for alias, idx in driver_pairs:
            src_attr = f"{driver_bs}.{alias}"
            val = 0.0
            try:
                val = cmds.getAttr(src_attr)
            except Exception:
                pass
            for (ttr, tbs) in targets:
                # ensure alias exists on target; if not, skip
                if not cmds.attributeQuery(alias, node=tbs, exists=True):
                    continue
                dst_attr = f"{tbs}.{alias}"
                try:
                    cmds.setAttr(dst_attr, val)
                    cmds.setKeyframe(tbs, attribute=alias, time=f, value=val)
                except Exception:
                    pass
    print(f"[ARKitBake] Baked weights from {driver_tr} to {len(targets)} targets over {start_frame}-{end_frame}.")


def enforce_deformer_order(mesh_list=None):
    """
    Ensure skinCluster remains before our ARKit52_BS and any MH default blendShape nodes are preserved.
    Reorders only when safe. Returns a dict of mesh -> new order list.
    """
    if mesh_list is None:
        mesh_list = cmds.ls(selection=True, long=False) or []
    report = {}
    for tr in mesh_list:
        ttr = _get_transform(tr)
        if not _is_mesh_transform(ttr):
            continue
        hist = cmds.listHistory(ttr, pruneDagObjects=True) or []
        skin = None
        mh_bs = None
        arkit_bs = None
        for h in hist:
            t = cmds.nodeType(h)
            if t == "skinCluster" and not skin:
                skin = h
            if t == "blendShape":
                if h.startswith("ARKit52_BS"):
                    arkit_bs = h
                else:
                    # heuristics: treat first non-arkit blendShape as MH node
                    if not mh_bs:
                        mh_bs = h
        # If both exist, try to enforce: skin -> mh_bs -> arkit_bs
        try:
            if skin and mh_bs and arkit_bs:
                cmds.reorderDeformers(skin, mh_bs, ttr)
                cmds.reorderDeformers(mh_bs, arkit_bs, ttr)
            elif skin and arkit_bs:
                cmds.reorderDeformers(skin, arkit_bs, ttr)
        except Exception:
            pass
        # Record order
        new_hist = cmds.listHistory(ttr, pruneDagObjects=True) or []
        report[ttr] = [h for h in new_hist if cmds.nodeType(h) in ("skinCluster", "blendShape")]
    return report


def export_prep(driver_mesh=None, start_frame=None, end_frame=None):
    """
    One-shot prep: bake weights from head ARKit52_BS to selected meshes' ARKit52_BS,
    then enforce safe deformer order. After this, export FBX with Skins+Blend Shapes+Animation.
    """
    bake_weights_to_targets(start_frame=start_frame, end_frame=end_frame, driver_mesh=driver_mesh)
    rep = enforce_deformer_order()
    print("[ARKitBake] Export prep done. Deformer order:")
    for mesh, nodes in rep.items():
        print(f"  {mesh}: {nodes}")




```

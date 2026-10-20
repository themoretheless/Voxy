#!/usr/bin/env python3
"""Validate a complete diagnostic snapshot without treating it as FPS proof."""
import collections, hashlib, json, math, struct, sys
from pathlib import Path
root = Path(sys.argv[1])
m = json.loads((root / "metadata.json").read_text())
assert m["schema"] == "voxy-ray-chunk-v1" and m["byte_order"] == "little"
assert m["segment_stride"] == 176 and m["hit_stride"] == 32
w, h = m["viewport"]
x, y, tw, th = m["tile"]
n = m["ray_count_per_kind"]
assert n == tw * th and 0 <= x < w and 0 <= y < h
assert x + tw <= w and y + th <= h
files = {}
for f in m["files"]:
    name = f["name"]
    assert Path(name).name == name and name not in files
    data = (root / name).read_bytes()
    assert len(data) == f["bytes"]
    files[name] = data
assert len(files["camera.bin"]) == m["camera_bytes"]
assert struct.unpack("<4I", files["tile.bin"]) == tuple(m["tile"])
assert len(files["depth.bin"]) == w * h * 4
assert all(math.isfinite(z) and 0 <= z <= 1 for (z,) in struct.iter_unpack("<f", files["depth.bin"]))
results = {}
for kind in m["ray_kinds"]:
    seg, hit, order = (files[kind + "." + part + ".bin"] for part in ["segments", "hits", "order"])
    assert len(seg) == n * 176 and len(hit) == n * 32 and len(order) == n * 4
    ids = [v[0] for v in struct.iter_unpack("<I", order)]
    assert len(set(ids)) == n and min(ids) == 0 and max(ids) == n - 1
    states = collections.Counter()
    hit_count = 0
    low_words_nonzero = 0
    for i in range(n):
        state = struct.unpack_from("<I", seg, i * 176 + 160)[0]
        assert state in [0, 1, 2]
        states[state] += 1
        if state == 1:
            assert all(math.isfinite(v) for v in struct.unpack_from("<40f", seg, i * 176))
            low_words_nonzero += sum(v != 0 for v in struct.unpack_from("<8f", seg, i * 176 + 96))
        hx, hy, hi, lo, visited, p0, p1, p2 = struct.unpack_from("<2i2f4I", hit, i * 32)
        assert (hx == -1 and hy == -1) or (0 <= hx < w and 0 <= hy < h)
        assert p0 == p1 == p2 == 0
        hit_count += hx >= 0
    results[kind] = {"states": dict(states), "hits": hit_count, "nonzero_projected_low_words": low_words_nonzero, "complete_original_id_permutation": True}
print(json.dumps({"scope": "snapshot structure and permutation, not independent intersection correctness", "full_scene_fps_proven": False, "rays": results, "sha256": {k: hashlib.sha256(v).hexdigest() for k, v in files.items()}}, indent=2))

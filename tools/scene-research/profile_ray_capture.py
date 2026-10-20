#!/usr/bin/env python3
"""Work distribution only: visited cells are not GPU time or FPS."""
import collections, json, math, struct, sys
from pathlib import Path
root = Path(sys.argv[1])
m = json.loads((root / "metadata.json").read_text())
results = {}
for kind in m["ray_kinds"]:
    segments = (root / (kind + ".segments.bin")).read_bytes()
    hits = (root / (kind + ".hits.bin")).read_bytes()
    order = [v[0] for v in struct.iter_unpack("<I", (root / (kind + ".order.bin")).read_bytes())]
    n = m["ray_count_per_kind"]
    assert len(segments) == n * 176 and len(hits) == n * 32
    assert sorted(order) == list(range(n))
    visits = [struct.unpack_from("<I", hits, i * 32 + 16)[0] for i in range(n)]
    states = [struct.unpack_from("<I", segments, i * 176 + 160)[0] for i in range(n)]
    active = [visits[i] for i in range(n) if states[i] == 1]
    def quantiles(values):
        v = sorted(values)
        return {str(q): v[min(len(v)-1, int((len(v)-1)*q))] for q in [0, .5, .9, .95, .99, 1]} if v else {}
    def distribution(ids, width):
        capacity = sum(max(visits[i] for i in ids[start:start+width]) * len(ids[start:start+width]) for start in range(0,n,width))
        return {"visited_cell_lane_capacity_proxy":capacity,"visited_cell_lane_utilization_proxy":sum(visits)/capacity if capacity else None}
    results[kind] = {"rays": n, "states": dict(collections.Counter(states)), "active_visited_quantiles":quantiles(active), "total_visited_cells":sum(visits), "groups": {str(width): {"original":distribution(list(range(n)),width), "queued":distribution(order,width)} for width in [32,64]}}
print(json.dumps({"scope":"work distribution hypothesis only; visited counts exclude other work and cannot prove speedup", "full_scene_fps_proven":False,"results":results},indent=2))

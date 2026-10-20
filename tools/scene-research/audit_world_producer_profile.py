#!/usr/bin/env python3
"""Check last-source-frame timestamp coverage, including the ray producer.

Intervals may overlap. Sums are diagnostic and are never exclusive frame costs.
"""
import argparse
import json
import math
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('profile', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
r = json.loads(a.profile.read_text())
names = r['stages']
assert names[0] == 'producer' and len(names) == len(set(names))
split = r.get('consumer_split_diagnostic', False)
sorted_mode = r.get('consumer_sorted', False)
assert not (split and sorted_mode)
if sorted_mode:
    assert names[1:6] == ['consumer_project', 'consumer_prefix', 'consumer_scatter', 'consumer_traverse', 'consumer_colour']
if split:
    assert names[1:4] == ['consumer_project', 'consumer_traverse', 'consumer_colour']
assert r['intervals_may_overlap'] and not r['exclusive_frame_cost']
assert not r['frame_fps_measured'] and r['encoded_source_frame'] > 0
kinds, tiles, stages = len(r['ray_kinds']), r['tiles_per_kind'], len(names)
regions = kinds * tiles
prefix = regions * stages * 2
assert r['source_region_tick_words'] == prefix
cache = r['consumer_components_profiled']
extra = kinds * (tiles - 1) * 2 if cache else 0
assert len(r['raw_ticks']) == prefix + extra
assert len(r['region_stage_ms']) == regions
periods = []
for region, values in enumerate(r['region_stage_ms']):
    assert len(values) == stages
    for stage, ms in enumerate(values):
        at = 2 * (region * stages + stage)
        begin, end = r['raw_ticks'][at:at+2]
        assert 0 < begin < end and math.isfinite(ms) and ms > 0
        periods.append(ms * 1e6 / (end - begin))
period = periods[0]
assert all(math.isclose(v, period, rel_tol=1e-9, abs_tol=1e-9) for v in periods)
for kind in range(kinds):
    for stage in range(stages):
        total = sum(r['region_stage_ms'][kind*tiles+tile][stage] for tile in range(tiles))
        assert math.isclose(total, r['sum_stage_ms_per_kind'][kind][stage], rel_tol=1e-12)
    samples = r['consumer_resolve_samples_ms'][kind]
    assert len(samples) == (tiles - 1 if cache else 0)
    for tile, ms in enumerate(samples):
        at = prefix + 2 * (kind*(tiles-1) + tile)
        begin, end = r['raw_ticks'][at:at+2]
        assert 0 < begin < end
        assert math.isclose(ms, (end-begin)*period/1e6, rel_tol=1e-9)
result = dict(timestamp_slots_complete=True, consumer_split_diagnostic=split, consumer_sorted=sorted_mode,
              transport_overhead_included=split or sorted_mode, timestamp_period_ns_inferred=period,
              encoded_source_frame=r['encoded_source_frame'], tiles_per_kind=tiles,
              sum_intervals_ms_by_kind={kind: dict(zip(names, values)) for kind, values in
                                       zip(r['ray_kinds'], r['sum_stage_ms_per_kind'])},
              intervals_may_overlap=True, exclusive_frame_cost=False,
              fps_qualification=False)
a.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))

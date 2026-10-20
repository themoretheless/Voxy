#!/usr/bin/env python3
"""Every GPU high/low word must match exact rational RN-even gold, no tolerance."""
import argparse
import json
import struct
from collections import Counter
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('cases', type=Path)
parser.add_argument('truth', type=Path)
parser.add_argument('results', type=Path)
parser.add_argument('--queued', action='store_true')
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
case_rows = [json.loads(line) for line in args.cases.read_text().splitlines() if line]
cases = {row['global_ray']: row for row in case_rows}
truth_rows = json.loads(args.truth.read_text())['rows']
gold = {row['global_ray']: row for row in truth_rows}
assert len(cases) == len(case_rows) == len(gold) == len(truth_rows)
raw = args.results.read_bytes()
stride = 72 if args.queued else 64
assert len(raw) == len(cases)*stride
seen = set()
params = 0
states, masks = Counter(), Counter()
max_slots = 0
for index, values in enumerate(struct.iter_unpack('<18I' if args.queued else '<16I', raw)):
    record = values[2:] if args.queued else values
    state, reason, bits, slots, low, high, source_slot, primitive = record[:8]
    ray_id = low | high << 32
    if args.queued:
        assert ray_id == values[0]*65536+values[1]
    else:
        assert ray_id == case_rows[index]['global_ray']
    assert ray_id not in seen
    row = gold[ray_id]
    assert state in (0, 1) and (state == 1) == row['exact_hit']
    assert reason == 0 and bits == 2048 and 0 < slots <= 512
    assert [source_slot, primitive] == cases[ray_id]['ray_words'][16:18]
    if state == 1:
        assert record[14] == 1
        assert list(record[8:14]) == row['parameter_words'], (ray_id, record[8:14], row['parameter_words'])
        assert record[15] == row['residual_mask']
        params += 1
        masks[record[15]] += 1
    else:
        assert not any(record[8:])
    seen.add(ray_id)
    states[state] += 1
    max_slots = max(max_slots, slots)
assert seen == set(gold)
report = {'cases': len(cases), 'states': dict(states), 'parameter_cases': params,
          'parameter_words_compared': params*6, 'remaining_rational_residual_masks': dict(masks),
          'maximum_workspace_slots': max_slots, 'all_words_match_exact_rational_rn_even': True,
          'all_ids_checked': True, 'world_nearest_qualified': False, 'fps_qualified': False}
with args.output.open('x') as output:
    json.dump(report, output, indent=2)
print(json.dumps(report, indent=2))

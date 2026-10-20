#!/usr/bin/env python3
"""Independently join fast GPU records, actual GPU queue results and rational gold."""
import argparse
import json
import struct
from collections import Counter
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('capture', type=Path)
parser.add_argument('truth', type=Path)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
fast = (args.capture/'results.bin').read_bytes()
assert len(fast)%32 == 0
states = Counter()
unresolved = set()
for index, record in enumerate(struct.iter_unpack('<8I', fast)):
    states[record[0]] += 1
    if record[0] == 2:
        unresolved.add(index)
assert states[3] == 0
cases = {row['global_ray']: row for row in map(json.loads, (args.capture/'fallback-cases.jsonl').read_text().splitlines())}
gold_rows = json.loads(args.truth.read_text())['rows']
gold = {row['global_ray']: row['exact_hit'] for row in gold_rows}
assert len(gold) == len(gold_rows)
assert set(cases) == set(gold) == unresolved
raw = (args.capture/'queued-exact.bin').read_bytes()
assert len(raw)%40 == 0
seen = set()
exact = Counter()
max_slots = 0
for record in struct.iter_unpack('<10I', raw):
    batch, local, state, reason, bits, slots, low, high, source_slot, primitive = record
    index = low | (high << 32)
    assert index == batch*65536+local
    assert index not in seen and index in unresolved
    assert state in (0, 1) and reason == 0 and bits == 2048 and 0 < slots <= 2048
    assert (state == 1) == gold[index]
    assert cases[index]['ray_words'][16:18] == [source_slot, primitive]
    seen.add(index)
    exact[state] += 1
    max_slots = max(max_slots, slots)
assert seen == unresolved
report = {'rays': len(fast)//32, 'fast_states': dict(states), 'gpu_queued_cases': len(seen),
          'exact_states': dict(exact), 'combined_misses': states[0]+exact[0],
          'combined_hits': states[1]+exact[1], 'unresolved_decisions': 0,
          'max_workspace_slots': max_slots, 'all_ids_checked': True,
          'cpu_selection_used': False, 'gold_uploaded': False,
          'parameters_complete': False, 'nearest_material_qualified': False, 'fps_qualified': False}
assert report['combined_misses']+report['combined_hits'] == report['rays']
with args.output.open('x') as output:
    json.dump(report, output, indent=2)
print(json.dumps(report, indent=2))

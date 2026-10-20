#!/usr/bin/env python3
"""Check every raw exact GPU result and its full identity against rational gold."""
import argparse
import json
import struct
from collections import Counter
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('cases', type=Path)
parser.add_argument('truth', type=Path)
parser.add_argument('results', type=Path)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
cases = [json.loads(line) for line in args.cases.read_text().splitlines() if line]
rows = json.loads(args.truth.read_text())['rows']
gold = {row['global_ray']: row['exact_hit'] for row in rows}
assert len(gold) == len(rows) == len(cases)
raw = args.results.read_bytes()
assert len(raw) == len(cases)*32
states = Counter()
max_slots = 0
seen = set()
for case, record in zip(cases, struct.iter_unpack('<8I', raw)):
    state, reason, bits, slots, low, high, source_slot, primitive = record
    index = low | high << 32
    assert index == case['global_ray'] and index not in seen
    assert state in (0, 1) and (state == 1) == gold[index]
    assert reason == 0 and bits == 2048 and 0 < slots <= 2048
    assert [source_slot, primitive] == case['ray_words'][16:18]
    seen.add(index)
    states[state] += 1
    max_slots = max(max_slots, slots)
assert seen == set(gold)
report = {'cases': len(cases), 'states': dict(states), 'max_workspace_slots': max_slots,
          'all_source_ids_checked': True, 'all_decisions_match_exact_rational': True,
          'parameters_implemented': False, 'fps_qualified': False}
with args.output.open('x') as output:
    json.dump(report, output, indent=2)
print(json.dumps(report, indent=2))

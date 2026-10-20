#!/usr/bin/env python3
"""Audit complete raw 32-byte narrow records, without floating-point conversion of low words."""
import argparse
import json
import math
import struct
from collections import Counter
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('results', type=Path)
parser.add_argument('--count', required=True, type=int)
parser.add_argument('--cases', type=Path)
parser.add_argument('--output', type=Path)
args = parser.parse_args()
raw = args.results.read_bytes()
assert len(raw) == args.count*32, (len(raw), args.count*32)
cases = json.loads(args.cases.read_text()) if args.cases else None
if cases:
    assert len(cases) == args.count
states, reasons = Counter(), Counter()
nonzero_low = 0
for index, record in enumerate(struct.iter_unpack('<8I', raw)):
    state = record[0]
    assert state in (0, 1, 2, 3)
    states[state] += 1
    if state == 2:
        reasons[record[7]] += 1
    if state == 1:
        for component in range(3):
            hi, lo = record[1+component*2:3+component*2]
            assert all(math.isfinite(struct.unpack('<f', struct.pack('<I', v))[0]) for v in (hi, lo))
            nonzero_low += (lo & 0x7fffffff) != 0
    if cases:
        assert state == cases[index]['expected_state'], (index, state, cases[index])
        if cases[index].get('t_low_must_be_nonzero'):
            assert state == 1 and record[2] & 0x7fffffff
report = {'records': args.count, 'bytes': len(raw), 'states': dict(states),
          'fallback_reasons': dict(reasons), 'nonzero_low_components': nonzero_low,
          'boundary_states_checked': cases is not None, 'fallback_complete': False,
          'universal_precision_qualified': False, 'fps_qualified': False}
encoded = json.dumps(report, indent=2)
if args.output:
    with args.output.open('x') as output:
        output.write(encoded+'\n')
print(encoded)

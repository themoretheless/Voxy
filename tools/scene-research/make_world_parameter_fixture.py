#!/usr/bin/env python3
"""Extend exact conformance scene with rounding ties and underflow visibility."""
import copy
import json
import runpy
import struct
import sys
from pathlib import Path
root = Path(sys.argv[1])
runpy.run_path(str(Path(__file__).with_name('make_world_exact_fixture.py')), run_name='__main__')
path = root/'exact-cases.jsonl'
cases = [json.loads(line) for line in path.read_text().splitlines()]

def word(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]

for name, high, low, direction in [
    ('normal-tie-carry-even', 1., -2**-25, -1.),
    ('normal-tie-even-lower', 1., -3*2**-25, -1.),
    ('half-min-subnormal', 2**-149, 0., -2.),
    ('odd-half-min-subnormal', 3*2**-149, 0., -2.),
    ('subnormal-normal-tie', 2**-125, -2**-149, -2.),
]:
    case = copy.deepcopy(cases[0])
    case['name'] = name
    case['global_ray'] = (1 << 42)+len(cases)
    case['ray_words'][2] = word(high)
    case['ray_words'][6] = word(low)
    case['ray_words'][10] = word(direction)
    cases.append(case)
path.write_text(''.join(json.dumps(case)+'\n' for case in cases))
print(f'parameter fixtures={len(cases)}')

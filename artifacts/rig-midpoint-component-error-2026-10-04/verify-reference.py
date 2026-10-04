import ast
from fractions import Fraction as F
from math import isqrt
from pathlib import Path
count = 0
for line in Path('artifacts/rig-midpoint-component-error-2026-10-04/reference.log').read_text().splitlines():
    if 'MIDPOINT_COMPONENT_ERROR ' not in line: continue
    raw, actual, caps = ast.literal_eval(line.split('MIDPOINT_COMPONENT_ERROR ', 1)[1])
    raw = [F.from_float(v) for v in raw]
    squared = sum(v*v for v in raw)
    scale = 1 << 384
    floor = isqrt(squared.numerator*scale*scale//squared.denominator)
    lo, hi = F(floor, scale), F(floor+1, scale)
    assert 0 < lo and lo*lo <= squared <= hi*hi
    for value,stored,cap in zip(raw,actual,caps):
        interval = [value/lo, value/hi]
        assert max(abs(F.from_float(stored)-v) for v in interval) <= F.from_float(cap)
        count += 1
assert count == 8, count
print('PASS: 8 selected quaternion components lie within caps against exact normalized-source rational brackets; no tolerance')

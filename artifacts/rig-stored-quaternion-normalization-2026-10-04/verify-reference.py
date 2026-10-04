import ast
from fractions import Fraction as F
from math import isqrt
from pathlib import Path
count = 0
for line in Path('artifacts/rig-stored-quaternion-normalization-2026-10-04/reference.log').read_text().splitlines():
    if 'STORED_NORMALIZATION ' not in line:
        continue
    values, actual, caps = ast.literal_eval(line.split('STORED_NORMALIZATION ', 1)[1])
    values = [F.from_float(v) for v in values]
    squared = sum(v*v for v in values)
    scale = 1 << 384
    root_floor = isqrt(squared.numerator * scale * scale // squared.denominator)
    lo, hi = F(root_floor, scale), F(root_floor + 1, scale)
    assert lo*lo <= squared <= hi*hi and lo > 0
    for value, stored, cap in zip(values, actual, caps):
        normalized = sorted([value/lo, value/hi])
        observed = F.from_float(stored)
        discrepancy = max(abs(observed - bound) for bound in normalized)
        assert discrepancy <= F.from_float(cap)
        count += 1
assert count == 16, count
print('PASS: 16 normalized components enclosed against rational square-root brackets; no tolerance')

"""Exact rational Hamilton chain and square-root enclosures, no float oracle."""
from fractions import Fraction as F
from math import isqrt
import json
import struct
import sys


def stored(v):
    return F(struct.unpack('<f', struct.pack('<f', v))[0])


def mul(a, b):
    x, y, z, w = a
    X, Y, Z, W = b
    return [w*X+x*W+y*Z-z*Y, w*Y-x*Z+y*W+z*X,
            w*Z+x*Y-y*X+z*W, w*W-x*X-y*Y-z*Z]


def conjugate(q):
    return [-q[0], -q[1], -q[2], q[3]]


def unit_intervals(q):
    norm2 = sum(v*v for v in q)
    scale = 1 << 192
    n = isqrt((norm2.numerator*scale*scale)//norm2.denominator)
    lo, hi = F(n, scale), F(n+1, scale)
    assert lo > 0 and lo*lo <= norm2 <= hi*hi
    return [sorted([v/lo, v/hi]) for v in q]


input_checks = output_checks = records = 0
for line in open(sys.argv[1], encoding='utf-8'):
    marker = 'retarget_rotation_error_reference='
    if marker not in line:
        continue
    d = json.loads(line.split(marker, 1)[1])
    source, target, correction, animated = [list(map(stored, d[k]))
        for k in ['source_bind', 'target_bind', 'correction', 'animated']]
    for raw, interval, cap in zip(animated, unit_intervals(animated), d['source_error']):
        assert max(abs(raw-v) for v in interval) <= F(cap)
        input_checks += 1
    # Positive normalization factors cancel from the final unit orientation.
    delta = mul(conjugate(source), animated)
    product = mul(mul(mul(target, correction), delta), conjugate(correction))
    for actual, interval, cap in zip(map(stored, d['actual']), unit_intervals(product), d['caps']):
        assert max(abs(actual-v) for v in interval) <= F(cap), (records, d)
        output_checks += 1
    records += 1
assert records == 256 and input_checks == output_checks == 1024
print(f'Exact source-error component checks: {input_checks}')
print(f'Exact retarget-rotation component checks: {output_checks}')
print(f'Original Hamilton chains: {records}')

#!/usr/bin/env python3
"""Independently prove captured GPU reciprocal enclosures and box rejections."""
import argparse
import json
import struct
from fractions import Fraction as Q
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('root', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()

def read(name, count):
    return list(struct.iter_unpack('<' + 'I' * count, (a.root / name).read_bytes()))

def value(bits):
    return Q.from_float(struct.unpack('<f', struct.pack('<I', bits))[0])

cases = read('reciprocal-cases.bin', 32)
nodes = read('reciprocal-nodes.bin', 8)
stride = (a.root / 'reciprocal-output.bin').stat().st_size // (len(cases) * 4)
assert stride in [40, 56]
results = read('reciprocal-output.bin', stride)
assert len(cases) == len(nodes) == len(results)
ready = old_rejections = cached_rejections = true_hits = inverse_checks = endpoint_checks = 0
for index, (ray, node, result) in enumerate(zip(cases, nodes, results)):
    old, cached, admitted = result[:3]
    assert old in [0, 1] and cached in [0, 1] and admitted in [0, 1]
    origin = [value(ray[k]) + value(ray[k+4]) for k in range(3)]
    direction = [value(ray[k+8]) + value(ray[k+12]) for k in range(3)]
    near, far = Q(0), value(ray[16])
    possible = True
    for axis in range(3):
        lo, hi = value(node[axis]), value(node[axis+3])
        assert lo <= hi
        d, o = direction[axis], origin[axis]
        if d == 0:
            possible &= lo <= o <= hi
        else:
            ends = sorted([(lo-o)/d, (hi-o)/d])
            near, far = max(near, ends[0]), min(far, ends[1])
    hit = possible and near <= far
    assert not old or not hit, ('original false rejection', index)
    assert not cached or not hit, ('cached false rejection', index)
    if not admitted:
        assert old == cached, ('unadmitted inverse changed old path', index)
    for axis in range(3):
        inv_lo, inv_hi, inv_valid, _ = result[4+axis*4:8+axis*4]
        o_lo, o_hi, o_valid, _ = result[16+axis*4:20+axis*4]
        d_lo, d_hi, d_valid, _ = result[28+axis*4:32+axis*4]
        if o_valid:
            assert value(o_lo) <= origin[axis] <= value(o_hi)
        if d_valid:
            assert value(d_lo) <= direction[axis] <= value(d_hi)
        if inv_valid:
            assert d_valid
            dl, dh = value(d_lo), value(d_hi)
            if dl == dh == 0:
                assert inv_lo == inv_hi == 0
                continue
            if dh < 0:
                dl, dh = -dh, -dl
            assert dl > 0
            assert value(inv_lo) <= 1/dh and value(inv_hi) >= 1/dl
            inverse_checks += 1
        if admitted:
            assert inv_valid
    if stride == 56:
        normal = lambda bits: (bits & 0x7fffffff) == 0 or 0x01000000 <= (bits & 0x7fffffff) < 0x7f000000
        expected_admission = normal(ray[0]) and normal(ray[8])
        for op in range(4):
            bound, valid, checked, _ = result[40+op*4:44+op*4]
            assert checked == int(expected_admission)
            if valid:
                assert checked and normal(bound)
                exact = value(ray[0]) + value(ray[8]) if op < 2 else value(ray[0]) * value(ray[8])
                assert value(bound) <= exact if op % 2 == 0 else value(bound) >= exact
                endpoint_checks += 1
    ready += admitted
    old_rejections += old
    cached_rejections += cached
    true_hits += hit
assert ready > 1000 and cached_rejections > 1000 and true_hits > 100
report = {'cases': len(cases), 'admitted_reciprocals': ready,
          'inverse_interval_enclosures_checked': inverse_checks,
          'exact_box_hits': true_hits, 'old_rejections': old_rejections,
          'cached_rejections': cached_rejections,
          'false_rejections': 0, 'one_sided_scalar_enclosures_checked': endpoint_checks, 'unadmitted_inverse_uses_old_path': True,
          'oracle': 'independent Fraction slab intersection and reciprocal enclosure',
          'fps_qualified': False}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))

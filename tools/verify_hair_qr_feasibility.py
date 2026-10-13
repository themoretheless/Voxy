#!/usr/bin/env python3
"""Exactly verify a coordinate witness against original VQC1 physical loads.

Finite binary64 inputs are treated as exact rational numbers. This proves only
linear-inequality feasibility, not energy optimality or physical-step admission.
"""
import argparse
from fractions import Fraction
import hashlib
import json
import math
import mmap
import struct
from audit_hair_qr_input import audit

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('capture')
parser.add_argument('coordinates_json')
args = parser.parse_args()
metadata = audit(args.capture)
raw = open(args.coordinates_json, 'rb').read()
coordinates = [float(x) for x in json.loads(raw)]
if len(coordinates) != metadata['coordinates'] or not all(map(math.isfinite, coordinates)):
    raise ValueError('invalid coordinate witness')
coordinates = list(map(Fraction.from_float, coordinates))
rows = metadata['rows']
with open(args.capture, 'rb') as source:
    with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
        bounds = [Fraction.from_float(x[0]) for x in struct.iter_unpack('<d', data[28:28+8*rows])]
        values = [Fraction() for _ in range(rows)]
        cursor = 28 + 16 * rows + 8 * rows * metadata['coordinates']
        base = 0
        for _ in range(metadata['systems']):
            n, band, lo, hi = struct.unpack_from('<IIII', data, cursor)
            cursor += 16 + 8 * n * (band + 1)
            local = coordinates[base:base+n]
            if any(local[:lo]) or any(local[hi:]):
                raise ValueError('witness moves fixed coordinates')
            for row in range(rows):
                for (coefficient,), value in zip(struct.iter_unpack('<d', data[cursor:cursor+8*n]), local):
                    if coefficient and value:
                        values[row] += Fraction.from_float(coefficient) * value
                cursor += 8 * n
            base += n
        gaps = [value - bound for value, bound in zip(values, bounds)]
        minimum = min(gaps)
        passed = minimum >= 0
        print(json.dumps(dict(exact_original_inequalities_satisfied=passed,
            minimum_gap_numerator=str(minimum.numerator), minimum_gap_denominator=str(minimum.denominator),
            minimum_gap_float=float(minimum), rows=rows, capture_sha256=metadata['sha256'],
            coordinate_sha256=hashlib.sha256(raw).hexdigest(),
            scope='exact linear feasibility only; no energy, force-balance or swept-motion admission'), indent=2))
        raise SystemExit(0 if passed else 1)

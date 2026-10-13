#!/usr/bin/env python3
"""Verify a Farkas witness against exact original VQC1 binary64 loads.

Only proves infeasibility of captured linear inequalities, not of nonlinear
geometry or the requested animation. No third-party solver is trusted here.
"""
import argparse
from fractions import Fraction
import hashlib
import json
import mmap
import struct
from audit_hair_qr_input import audit


def verify(capture, witness):
    metadata = audit(capture)
    with open(witness, 'rb') as source:
        raw = source.read()
    weights = [Fraction(x) for x in json.loads(raw)]
    rows = metadata['rows']
    if len(weights) != rows or any(x < 0 for x in weights) or not any(weights):
        raise ValueError('expected nonzero nonnegative row weights')
    with open(capture, 'rb') as source:
        with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
            bounds = [Fraction.from_float(x[0]) for x in
                      struct.iter_unpack('<d', data[28:28 + 8 * rows])]
            cursor = 28 + 16 * rows + 8 * rows * metadata['coordinates']
            nonzero_coordinates = 0
            for _ in range(metadata['systems']):
                n, band, lo, hi = struct.unpack_from('<IIII', data, cursor)
                cursor += 16 + 8 * n * (band + 1)
                total = [Fraction() for _ in range(n)]
                for weight in weights:
                    for j, (value,) in enumerate(struct.iter_unpack(
                            '<d', data[cursor:cursor + 8 * n])):
                        if weight and value:
                            total[j] += weight * Fraction.from_float(value)
                    cursor += 8 * n
                # Check every coordinate, including fixed coordinates. This
                # deliberately requires the stronger full-space certificate.
                nonzero_coordinates += sum(x != 0 for x in total)
            bound = sum(x * y for x, y in zip(weights, bounds))
    passed = nonzero_coordinates == 0 and bound > 0
    return dict(exact_original_linear_infeasibility=passed,
                nonzero_transpose_coordinates=nonzero_coordinates,
                weighted_bound=str(bound), weighted_bound_float=float(bound),
                rows=rows, capture_sha256=metadata['sha256'],
                witness_sha256=hashlib.sha256(raw).hexdigest(),
                scope='captured linear inequalities only; no nonlinear infeasibility claim')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture')
    parser.add_argument('weights_json')
    args = parser.parse_args()
    result = verify(args.capture, args.weights_json)
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['exact_original_linear_infeasibility'] else 1)

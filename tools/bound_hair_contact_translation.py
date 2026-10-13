#!/usr/bin/env python3
"""Exact necessary translation bound from two original contact inequalities.

Adding B_i*x >= b_i and B_j*x >= b_j gives
max(abs(x)) >= (b_i+b_j)/sum(abs(B_i+B_j)).
This is a necessary bound, not a feasible physical step or optimal solution.
Rejects rows with any angular load, so the reported bound is translational.
"""
import argparse
from fractions import Fraction as F
import json
import mmap
import struct
from audit_hair_qr_input import audit


def bound(path, row_a, row_b):
    metadata = audit(path)
    rows = metadata['rows']
    if not (0 <= row_a < rows and 0 <= row_b < rows and row_a != row_b):
        raise ValueError('expected two different valid row indices')
    coefficients = []
    with open(path, 'rb') as source:
        with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
            rhs = sum(F.from_float(struct.unpack_from('<d', data, 28 + 8 * row)[0])
                      for row in (row_a, row_b))
            cursor = 28 + 16 * rows + 8 * rows * metadata['coordinates']
            for _ in range(metadata['systems']):
                n, band, lo, hi = struct.unpack_from('<IIII', data, cursor)
                cursor += 16 + 8 * n * (band + 1)
                if n % 6:
                    raise ValueError('expected six rod coordinates per point')
                local = []
                for row in (row_a, row_b):
                    local.append([F.from_float(v[0]) for v in struct.iter_unpack(
                        '<d', data[cursor + 8 * n * row:cursor + 8 * n * (row + 1)])])
                if any(value for load in local for j, value in enumerate(load) if j % 6 >= 3):
                    raise ValueError('row includes angular load; translation bound invalid')
                coefficients.extend(a + b for a, b in zip(*local))
                cursor += 8 * n * rows
    slope = sum(map(abs, coefficients))
    if rhs <= 0 or slope == 0:
        raise ValueError('pair does not give a finite positive translation bound')
    lower = rhs / slope
    return dict(capture_sha256=metadata['sha256'], rows=[row_a, row_b],
                exact_bound_sum=str(rhs), exact_coefficient_l1_sum=str(slope),
                maximum_translation_coordinate_lower_bound_m=str(lower),
                lower_bound_float_m=float(lower),
                scope='necessary exact original-linear-inequality bound; not nonlinear feasibility or an admitted motion')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture')
    parser.add_argument('row_a', type=int)
    parser.add_argument('row_b', type=int)
    args = parser.parse_args()
    print(json.dumps(bound(args.capture, args.row_a, args.row_b), indent=2))

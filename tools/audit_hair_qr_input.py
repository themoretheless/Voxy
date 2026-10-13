#!/usr/bin/env python3
"""Audit exact VQC1 captures without allocating dense solver matrices.

Run only on a completed/frozen capture. This checks format and finite values,
not contact feasibility, solver convergence or physical admission.
"""
import argparse
import hashlib
import json
import math
import mmap
import struct


def audit(path):
    with open(path, 'rb') as source:
        with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
            if len(data) < 28 or data[:4] != b'VQC1':
                raise ValueError('not a complete VQC1 header')
            rows, width, count, refinement, tolerance = struct.unpack_from('<IIIId', data, 4)
            if not (0 < rows <= 4096 and 0 < width <= 65536 and 0 < count <= 4096
                    and math.isfinite(tolerance) and tolerance > 0):
                raise ValueError('invalid dimensions or tolerance')
            cursor = 28

            def values(n):
                nonlocal cursor
                end = cursor + n * 8
                if end > len(data):
                    raise ValueError(f'truncated scalar payload at byte {cursor}')
                minimum, maximum = math.inf, -math.inf
                for start in range(cursor, end, 65536):
                    for (value,) in struct.iter_unpack('<d', data[start:min(start + 65536, end)]):
                        if not math.isfinite(value):
                            raise ValueError(f'nonfinite value in payload at byte {start}')
                        minimum, maximum = min(minimum, value), max(maximum, value)
                cursor = end
                return [minimum, maximum]

            bounds = values(rows)
            effective = values(rows)
            values(rows * width)
            actual_width = 0
            for system in range(count):
                if cursor + 16 > len(data):
                    raise ValueError('truncated system header')
                n, band, lo, hi = struct.unpack_from('<IIII', data, cursor)
                cursor += 16
                if not (0 < n <= width and band == 9 and lo <= hi <= n):
                    raise ValueError(f'invalid system {system}')
                actual_width += n
                if actual_width > width:
                    raise ValueError('system coordinates exceed declared width')
                values(n * band + n + rows * n)
            if actual_width != width or cursor != len(data):
                raise ValueError('coordinate count or trailing byte mismatch')
            digest = hashlib.sha256()
            for start in range(0, len(data), 1024 * 1024):
                digest.update(data[start:start + 1024 * 1024])
            return dict(format='VQC1', rows=rows, coordinates=width, systems=count,
                        refinement=refinement, tolerance=tolerance, bounds_range=bounds,
                        effective_bounds_range=effective, bytes=len(data), sha256=digest.hexdigest(),
                        scope='structural and finite-value audit only; no feasibility or convergence proof')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture')
    args = parser.parse_args()
    print(json.dumps(audit(args.capture), indent=2))

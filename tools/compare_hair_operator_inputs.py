#!/usr/bin/env python3
"""Compare exact VQC1 inputs by stored order; does not establish row identity or physics admission."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct


def read(path):
    raw = path.read_bytes()
    offset = 0

    def take(fmt):
        nonlocal offset
        size = struct.calcsize(fmt)
        if size > len(raw) - offset:
            raise ValueError(f'{path}: truncated input at {offset}')
        result = struct.unpack_from(fmt, raw, offset)
        offset += size
        return result

    def values(n):
        if n > (len(raw) - offset) // 8:
            raise ValueError(f'{path}: invalid scalar count')
        start = offset
        result = take(f'<{n}d')
        if not all(math.isfinite(v) for v in result):
            raise ValueError(f'{path}: nonfinite scalar')
        return raw[start:offset], result

    if take('<4s')[0] != b'VQC1':
        raise ValueError(f'{path}: unsupported schema')
    rows, coordinates, count, refinement = take('<4I')
    tolerance = take('<d')[0]
    if not math.isfinite(tolerance) or tolerance <= 0:
        raise ValueError(f'{path}: invalid tolerance')
    sections = {'bounds': values(rows), 'effective_bounds': values(rows),
                'columns': values(rows * coordinates)}
    layouts = []
    for i in range(count):
        n, band, lo, hi = take('<4I')
        if n == 0 or band == 0 or band > n or not 0 <= lo <= hi <= n:
            raise ValueError(f'{path}: invalid system layout')
        layouts.append([n, band, lo, hi])
        for name, size in [('matrix', n * band), ('rhs', n), ('loads', rows * n)]:
            sections[f'system_{i}_{name}'] = values(size)
    if sum(v[0] for v in layouts) != coordinates or offset != len(raw):
        raise ValueError(f'{path}: inconsistent coordinates or trailing data')
    return {'path': str(path), 'sha256': hashlib.sha256(raw).hexdigest(),
            'rows': rows, 'coordinates': coordinates, 'refinement': refinement,
            'tolerance': tolerance, 'layouts': layouts}, sections


def compare(left, right):
    a, av = read(left)
    b, bv = read(right)
    aligned = all(a[k] == b[k] for k in ['rows', 'coordinates', 'layouts'])
    differences = {}
    if aligned:
        for key, (bits, values) in av.items():
            other_bits, other = bv[key]
            maximum = max((abs(x - y) for x, y in zip(values, other)), default=0.)
            differences[key] = {
                'bitwise_equal': bits == other_bits,
                'changed_scalars': sum(bits[i:i+8] != other_bits[i:i+8] for i in range(0, len(bits), 8)),
                'maximum_absolute_difference': maximum if math.isfinite(maximum) else None,
                'difference_overflow': not math.isfinite(maximum),
            }
    return {'schema': 'voxy-vqc-input-comparison-v1', 'left': a, 'right': b,
            'same_stored_layout': aligned, 'sections': differences,
            'limits': 'Stored-order comparison only. Matching dimensions do not prove semantic rod or row correspondence. No solver accuracy, conditioning, trajectory admission or FPS proof.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('left', type=Path)
    parser.add_argument('right', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(compare(args.left, args.right), indent=2, allow_nan=False))
    except (OSError, ValueError, struct.error) as error:
        parser.exit(2, f'error: {error}\n')


if __name__ == '__main__':
    main()

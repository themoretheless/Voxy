#!/usr/bin/env python3
"""Exact dyadic source transform and rational ray/triangle oracle for a captured case."""
import argparse
import json
import struct
from fractions import Fraction as Q
from pathlib import Path


def word(value):
    return Q.from_float(struct.unpack('<f', struct.pack('<I', value))[0])


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def cross(a, b):
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def dot(a, b):
    return sum((x*y for x, y in zip(a, b)), Q(0))


def decode_geometry(case):
    matrix = [[Q.from_float(x) for x in column] for column in case['world']]
    points = [tuple(sum((matrix[k][axis]*Q.from_float(p[k]) for k in range(3)), matrix[3][axis]) for axis in range(3)) for p in case['local_triangle']]
    words = case['ray_words']
    origin = tuple(word(words[k])+word(words[k+4]) for k in range(3))
    direction = tuple(word(words[k+8])+word(words[k+12]) for k in range(3))
    return points, origin, direction


def audit(case):
    points, origin, direction = decode_geometry(case)
    edge1, edge2 = sub(points[1], points[0]), sub(points[2], points[0])
    p = cross(direction, edge2)
    determinant = dot(edge1, p)
    if not determinant:
        return {'exact_degenerate': True, 'state': case['gpu_words'][0]}
    delta = sub(origin, points[0])
    q = cross(delta, edge1)
    exact = [dot(edge2, q)/determinant, dot(delta, p)/determinant, dot(direction, q)/determinant]
    result = case['gpu_words']
    gpu = [word(result[1+k*2])+word(result[2+k*2]) for k in range(3)]
    reference = [Q.from_float(x) for x in case['f64_reference']]
    return {'exact_degenerate': False, 'state': result[0],
            'exact_closed_triangle_hit': 0 <= exact[0] <= 1 and exact[1] >= 0 and exact[2] >= 0 and exact[1]+exact[2] <= 1,
            'exact_rational': [str(x) for x in exact], 'exact_rounded_f64': [float(x) for x in exact],
            'gpu_error_exact': [float(abs(x-y)) for x, y in zip(gpu, exact)] if result[0] == 1 else None,
            'f64_reference_error_exact': [float(abs(x-y)) for x, y in zip(reference, exact)],
            'gpu_f64_gate_passed': all(abs(x-y) <= Q.from_float(2e-10)*(1+abs(y)) for x, y in zip(gpu, exact)) if result[0] == 1 else None}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('case', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = audit(json.loads(args.case.read_text()))
    encoded = json.dumps(report, indent=2)
    if args.output:
        with args.output.open('x') as output:
            output.write(encoded+'\n')
    print(encoded)

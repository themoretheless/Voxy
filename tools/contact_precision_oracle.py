#!/usr/bin/env python3
"""Selected interior-edge distance oracle on exact binary64 fixture inputs.

This does not certify nearest features across all triangles or continuous CCD.
Uses only the Python standard library. Output is JSON on stdout.
"""
import argparse
from decimal import Decimal, localcontext
import hashlib
import json
from pathlib import Path


def subtract(a, b):
    return [x - y for x, y in zip(a, b)]


def dot(a, b):
    return sum((x * y for x, y in zip(a, b)), Decimal(0))


def report(path):
    raw = path.read_bytes()
    fixture = json.loads(raw)
    if fixture.get('version') != 1 or fixture.get('scope') != 'last-iterate-contact-precision':
        raise ValueError('unsupported fixture')
    poses = []
    with localcontext() as context:
        context.prec = 90
        minimum = Decimal.from_float(fixture['contact_parameters']['minimum_distance_m'])
        for pose in fixture['poses']:
            body = [[Decimal.from_float(x) for x in p] for p in pose['body_triangle']]
            obstacle = [[Decimal.from_float(x) for x in p] for p in pose['obstacle_triangle']]
            # Captured nearest feature: body edge 0--1, obstacle edge 0--2.
            u = subtract(body[1], body[0])
            v = subtract(obstacle[2], obstacle[0])
            r = subtract(body[0], obstacle[0])
            aa, bb, cc, dd, ee = dot(u, u), dot(u, v), dot(v, v), dot(u, r), dot(v, r)
            denominator = aa * cc - bb * bb
            if denominator <= 0:
                raise ValueError('oracle requires nonparallel edges')
            s = (bb * ee - cc * dd) / denominator
            t = (aa * ee - bb * dd) / denominator
            if not (0 < s < 1 and 0 < t < 1):
                raise ValueError('selected closest feature is not interior edge-edge')
            delta = [r[i] + s * u[i] - t * v[i] for i in range(3)]
            distance = dot(delta, delta).sqrt()
            poses.append({'body_edge': [0, 1], 'obstacle_edge': [0, 2],
                          'body_edge_parameter': str(s), 'obstacle_edge_parameter': str(t),
                          'distance_m': str(distance), 'gap_m': str(distance - minimum)})
    return {'scope': 'selected interior edge-edge oracle, not all-feature CCD',
            'decimal_precision': 90, 'input_semantics': 'exact binary64 coordinates',
            'fixture_sha256': hashlib.sha256(raw).hexdigest(), 'poses': poses}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fixture', type=Path)
    args = parser.parse_args()
    print(json.dumps(report(args.fixture), indent=2))

#!/usr/bin/env python3
"""Rational closed convex-hull oracle; degenerate triangles remain segments/points.

This is diagnostic CPU evidence, not an implemented GPU runtime fallback.
"""
import argparse
import json
from collections import Counter
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import cross, decode_geometry, dot, sub


def result(kind, lo=None, hi=None):
    return {'kind': kind, 'exact_hit': lo is not None,
            't_begin': str(lo) if lo is not None else None,
            't_end': str(hi) if hi is not None else None}


def point_or_segment(points, origin, direction):
    a, b = max(((points[i], points[j]) for i, j in [(0, 1), (0, 2), (1, 2)]),
               key=lambda pair: dot(sub(pair[1], pair[0]), sub(pair[1], pair[0])))
    edge, delta = sub(b, a), sub(a, origin)
    kind = 'point' if not any(edge) else 'segment'
    if not any(direction):
        if not any(edge):
            return result(kind, Q(0), Q(1)) if not any(delta) else result(kind)
        axis = next(i for i, v in enumerate(edge) if v)
        s = -delta[axis]/edge[axis]
        hit = 0 <= s <= 1 and all(delta[k]+edge[k]*s == 0 for k in range(3))
        return result(kind, Q(0), Q(1)) if hit else result(kind)
    for i, j in [(0, 1), (0, 2), (1, 2)]:
        determinant = direction[i]*edge[j]-direction[j]*edge[i]
        if determinant:
            t = (delta[i]*edge[j]-delta[j]*edge[i])/determinant
            s = (delta[i]*direction[j]-delta[j]*direction[i])/determinant
            hit = 0 <= t <= 1 and 0 <= s <= 1 and all(direction[k]*t == delta[k]+edge[k]*s for k in range(3))
            return result(kind, t, t) if hit else result(kind)
    if any(cross(delta, direction)):
        return result(kind)
    axis = next(i for i, v in enumerate(direction) if v)
    times = [delta[axis]/direction[axis], (delta[axis]+edge[axis])/direction[axis]]
    lo, hi = max(Q(0), min(times)), min(Q(1), max(times))
    return result(kind, lo, hi) if lo <= hi else result(kind)


def classify(points, origin, direction):
    edge1, edge2 = sub(points[1], points[0]), sub(points[2], points[0])
    normal = cross(edge1, edge2)
    if not any(normal):
        return point_or_segment(points, origin, direction)
    p = cross(direction, edge2)
    determinant = dot(edge1, p)
    delta = sub(origin, points[0])
    if determinant:
        q = cross(delta, edge1)
        t, u, v = dot(edge2, q)/determinant, dot(delta, p)/determinant, dot(direction, q)/determinant
        hit = 0 <= t <= 1 and u >= 0 and v >= 0 and u+v <= 1
        return result('noncoplanar', t, t) if hit else result('noncoplanar')
    if dot(normal, delta):
        return result('parallel-off-plane')
    drop = max(range(3), key=lambda k: abs(normal[k]))
    axes = [k for k in range(3) if k != drop]
    def orient(a, b):
        return a[axes[0]]*b[axes[1]]-a[axes[1]]*b[axes[0]]
    sign = 1 if orient(edge1, edge2) > 0 else -1
    lo, hi = Q(0), Q(1)
    for a, b in zip(points, points[1:]+points[:1]):
        edge = sub(b, a)
        constant = orient(edge, sub(origin, a))*sign
        slope = orient(edge, direction)*sign
        if not slope:
            if constant < 0:
                return result('coplanar')
        elif slope > 0:
            lo = max(lo, -constant/slope)
        else:
            hi = min(hi, -constant/slope)
    return result('coplanar', lo, hi) if lo <= hi else result('coplanar')


def self_test():
    vec = lambda *p: tuple(Q(x) for x in p)
    triangle = [vec(-1, -1, 0), vec(1, -1, 0), vec(0, 1, 0)]
    assert classify(triangle, vec(0, -1, 1), vec(0, 0, -1))['t_begin'] == '1'
    assert classify(triangle, vec(-1, -1, 0), vec(0, 0, -1))['exact_hit']
    assert classify(triangle, vec(0, 0, 0), vec(1, 0, 0))['t_end'] == '1/2'
    assert not classify(triangle, vec(0, 0, 1), vec(1, 0, 0))['exact_hit']
    segment = [vec(-1, 0, 0), vec(1, 0, 0), vec(0, 0, 0)]
    assert classify(segment, vec(0, 0, 1), vec(0, 0, -1))['exact_hit']
    assert classify(segment, vec(-2, 0, 0), vec(1, 0, 0))['t_begin'] == '1'
    assert not classify(segment, vec(-2, 1, 0), vec(1, 0, 0))['exact_hit']
    assert classify(segment, vec(0, 0, 0), vec(0, 0, 0))['exact_hit']
    point = [vec(0, 0, 0)]*3
    assert classify(point, vec(0, 0, 1), vec(0, 0, -1))['exact_hit']
    assert not classify(point, vec(1, 0, 1), vec(0, 0, -1))['exact_hit']
    print('exact closed oracle: 10 boundary/degenerate/coplanar checks passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('cases', nargs='?', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
    if args.cases:
        cases = [json.loads(line) for line in args.cases.read_text().splitlines() if line]
        rows = []
        for case in cases:
            row = classify(*decode_geometry(case))
            row['global_ray'] = case.get('global_ray')
            rows.append(row)
        report = {'cases': len(rows), 'classification': dict(Counter(row['kind'] for row in rows)),
                  'exact_hits': sum(row['exact_hit'] for row in rows), 'rows': rows,
                  'gpu_runtime_fallback_implemented': False}
        if args.output:
            with args.output.open('x') as output:
                json.dump(report, output, indent=2)
        print(json.dumps({k: v for k, v in report.items() if k != 'rows'}, indent=2))

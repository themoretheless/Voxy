#!/usr/bin/env python3
"""Correct RN-even high/low words directly from exact rational source geometry."""
import json
import sys
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import decode_geometry, cross, dot, sub, word
from exact_closed_world_triangle import classify


def power(exponent):
    return Q(2**exponent) if exponent >= 0 else Q(1, 2**-exponent)


def rounded(value):
    if not value:
        return 0
    sign = 0x80000000 if value < 0 else 0
    n, d = abs(value).numerator, abs(value).denominator
    exponent = n.bit_length()-d.bit_length()
    if Q(n, d) < power(exponent):
        exponent -= 1
    assert exponent <= 127
    shift = 23-exponent if exponent >= -126 else 149
    if shift >= 0:
        n <<= shift
    else:
        d <<= -shift
    quotient, remainder = divmod(n, d)
    if remainder*2 > d or remainder*2 == d and quotient & 1:
        quotient += 1
    if exponent < -126:
        assert quotient <= 1 << 23
        return sign | quotient
    if quotient == 1 << 24:
        quotient >>= 1
        exponent += 1
    assert 1 << 23 <= quotient < 1 << 24 and exponent <= 127
    return sign | (exponent+127) << 23 | quotient & 0x7fffff


def pair(value):
    hi = rounded(value)
    residual = value-word(hi)
    lo = rounded(residual)
    return [hi, lo], residual-word(lo) != 0


def segment_parameters(a, b, origin, direction):
    edge, delta = sub(b, a), sub(a, origin)
    if not any(direction):
        if not any(edge):
            return (Q(0), Q(0)) if not any(delta) else None
        axis = next(k for k, v in enumerate(edge) if v)
        s = -delta[axis]/edge[axis]
        return (Q(0), s) if 0 <= s <= 1 and all(delta[k]+edge[k]*s == 0 for k in range(3)) else None
    for i, j in [(0, 1), (0, 2), (1, 2)]:
        determinant = direction[i]*edge[j]-direction[j]*edge[i]
        if determinant:
            t = (delta[i]*edge[j]-delta[j]*edge[i])/determinant
            s = (delta[i]*direction[j]-delta[j]*direction[i])/determinant
            return (t, s) if 0 <= t <= 1 and 0 <= s <= 1 and all(direction[k]*t == delta[k]+edge[k]*s for k in range(3)) else None
    if any(cross(delta, direction)):
        return None
    axis = next(k for k, v in enumerate(direction) if v)
    endpoints = [delta[axis]/direction[axis], (delta[axis]+edge[axis])/direction[axis]]
    lo, hi = max(Q(0), min(endpoints)), min(Q(1), max(endpoints))
    if lo > hi:
        return None
    s = (direction[axis]*lo-delta[axis])/edge[axis] if any(edge) else Q(0)
    return lo, s


def exact_parameters(points, origin, direction):
    e1, e2 = sub(points[1], points[0]), sub(points[2], points[0])
    p = cross(direction, e2)
    determinant = dot(e1, p)
    delta = sub(origin, points[0])
    if determinant:
        q = cross(delta, e1)
        return [dot(e2, q)/determinant, dot(delta, p)/determinant, dot(direction, q)/determinant]
    normal = cross(e1, e2)
    if any(normal):
        drop = max(range(3), key=lambda k: abs(normal[k]))
        i, j = [k for k in range(3) if k != drop]
        area = e1[i]*e2[j]-e1[j]*e2[i]
        u = (delta[i]*e2[j]-delta[j]*e2[i])/area
        v = (e1[i]*delta[j]-e1[j]*delta[i])/area
        if u >= 0 and v >= 0 and u+v <= 1:
            return [Q(0), u, v]
    candidates = []
    for index, (a, b) in enumerate(zip(points, points[1:]+points[:1])):
        hit = segment_parameters(a, b, origin, direction)
        if hit is not None:
            t, s = hit
            u, v = [(s, Q(0)), (1-s, s), (Q(0), 1-s)][index]
            candidates.append((t, index, u, v))
    t, _, u, v = min(candidates)
    return [t, u, v]


def build(cases):
    rows = []
    for case in cases:
        points, origin, direction = decode_geometry(case)
        row = classify(points, origin, direction)
        row['global_ray'] = case['global_ray']
        row['parameter_words'] = None
        row['residual_mask'] = 0
        if row['exact_hit']:
            values = exact_parameters(points, origin, direction)
            pairs = [pair(v) for v in values]
            row['parameter_words'] = [v for words, _ in pairs for v in words]
            row['residual_mask'] = sum(int(rem) << k for k, (_, rem) in enumerate(pairs))
            row['exact_parameters'] = [str(v) for v in values]
        rows.append(row)
    return {'cases': len(rows), 'rows': rows, 'oracle': 'exact rational + RN-even twice'}


if __name__ == '__main__':
    assert rounded(Q(1)-power(-25)) == 0x3f800000
    assert rounded(power(-150)) == 0
    assert rounded(3*power(-150)) == 2
    assert pair(Q(1))[0] == [0x3f800000, 0]
    cases = [json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines() if line]
    result = build(cases)
    with Path(sys.argv[2]).open('x') as output:
        json.dump(result, output, indent=2)
    print(json.dumps({'cases': len(cases), 'parameter_cases': sum(row['parameter_words'] is not None for row in result['rows'])}))

#!/usr/bin/env python3
"""Exact residual certificates for experimental GPU rectangular equality QR."""
from fractions import Fraction as F
import json
import math
from pathlib import Path
import struct
import sys
from verify_five_products import exact_word


def scalar(bits):
    value = struct.unpack('<d', struct.pack('<Q', bits))[0]
    if not math.isfinite(value):
        raise ValueError('nonfinite native coefficient')
    return F.from_float(value)


def value(row):
    if len(row['words']) != 5 or not isinstance(row['exponent'], int) or abs(row['exponent']) > 2048:
        raise ValueError('invalid runtime expansion')
    scale = F(2) ** row['exponent']
    bound = exact_word(row['error_bound_word']) * scale
    if bound < 0:
        raise ValueError('negative certificate')
    return sum(map(exact_word, row['words']), F()) * scale, bound


def product(a, b):
    x, ex = a
    y, ey = b
    return x*y, abs(x)*ey + abs(y)*ex + ex*ey


def dot(a, b):
    if len(a) != len(b):
        raise ValueError('dot shape mismatch')
    terms = [product(x, y) for x, y in zip(a, b)]
    return sum((v for v, _ in terms), F()), sum((e for _, e in terms), F())


def verify(path):
    data = json.loads(Path(path).read_text())
    n = len(data['active_rows'])
    m = data['width']
    if not n or m < n or len(set(data['active_rows'])) != n:
        raise ValueError('invalid QR shape')
    cols = [[sum(map(scalar, parts), F()) for parts in col] for col in data['column_parts_bits']]
    if len(cols) != n or any(len(c) != m for c in cols) or len(data['bounds_bits']) != n:
        raise ValueError('invalid original column shape')
    state_scale = scalar(data['state_scale_bits'])
    reaction_scale = scalar(data['reaction_scale_bits'])
    if state_scale <= 0 or reaction_scale <= 0:
        raise ValueError('invalid scaling')
    column_scale = state_scale / reaction_scale
    q = list(map(value, data['basis']))
    r = list(map(value, data['triangular']))
    x = [(v*state_scale, e*state_scale) for v, e in map(value, data['coordinates'])]
    reactions = [(v*reaction_scale, e*reaction_scale) for v, e in map(value, data['reactions'])]
    if (len(q), len(r), len(x), len(reactions)) != (n*m, n*n, m, n):
        raise ValueError('invalid output shape')
    stats = {}
    def check(name, triples):
        failed = count = 0
        worst = F()
        for actual, target, bound in triples:
            count += 1
            error = abs(actual-target)
            worst = max(worst, error)
            failed += error > bound
        stats[name] = {'checked': count, 'outside_certificate': failed, 'worst_absolute_residual': float(worst)}
    def reconstruction():
        for col in range(n):
            for i in range(m):
                v, e = dot([q[j*m+i] for j in range(n)], [r[j*n+col] for j in range(n)])
                yield v, cols[col][i]/column_scale, e
    check('rectangular_reconstruction', reconstruction())
    def orthogonality():
        for a in range(n):
            for b in range(n):
                v, e = dot(q[a*m:(a+1)*m], q[b*m:(b+1)*m])
                yield v, F(a == b), e
    check('orthogonality', orthogonality())
    def equality():
        for col in range(n):
            v, e = dot([(v, F()) for v in cols[col]], x)
            yield v, scalar(data['bounds_bits'][col]), e
    check('active_equalities', equality())
    def equilibrium():
        for i in range(m):
            v, e = dot([(col[i], F()) for col in cols], reactions)
            yield v, x[i][0], e+x[i][1]
    check('coordinate_equilibrium', equilibrium())
    return {'scope': 'Exact rational residual enclosure on the supplied active-set GPU QR artifact; not active-set selection or full trajectory', 'active_rows': n, 'width': m, 'checks': stats,
            'status': 'passed' if all(s['outside_certificate'] == 0 for s in stats.values()) else 'failed'}


if __name__ == '__main__':
    report = verify(sys.argv[1])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
    print(json.dumps(report, indent=2))
    if report['status'] != 'passed':
        sys.exit(1)

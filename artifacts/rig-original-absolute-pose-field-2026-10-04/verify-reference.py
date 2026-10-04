"""Exact whole-interval oracle with nonbind translation and initial rotation."""
import json
import math
import struct
import sys
from fractions import Fraction as F
from functools import lru_cache


def f32(x):
    return F(struct.unpack("<f", struct.pack("<f", x))[0])


def add(a, b):
    return (a[0]+b[0], a[1]+b[1])


def mul(a, b):
    values = [x*y for x in a for y in b]
    return (min(values), max(values))


def neg(a):
    return (-a[1], -a[0])


def exact(x):
    return (x, x)


def atan_bounds(x):
    total = sum((-1)**n * x**(2*n+1)/F(2*n+1) for n in range(40))
    remainder = x**81/F(81)
    return (total-remainder, total+remainder)


pi = add(mul(exact(F(16)), atan_bounds(F(1, 5))),
         mul(exact(F(-4)), atan_bounds(F(1, 239))))


@lru_cache(maxsize=64)
def series(x, cosine):
    offset = 0 if cosine else 1
    total = sum((-1)**n * x**(2*n+offset)/F(math.factorial(2*n+offset))
                for n in range(40))
    error = x**(80+offset)/F(math.factorial(80+offset))
    return (total-error, total+error)


@lru_cache(maxsize=64)
def trig(angle):
    assert 0 <= angle[0] <= angle[1] <= 1
    # sin increases, cos decreases on this independently qualified domain.
    sine = (series(angle[0], False)[0], series(angle[1], False)[1])
    cosine = (series(angle[1], True)[0], series(angle[0], True)[1])
    return sine, cosine


def rotation(values):
    x, y, z, w = map(f32, values)
    norm = x*x+y*y+z*z+w*w
    return [[v/norm for v in row] for row in (
        (w*w+x*x-y*y-z*z, 2*(x*y-w*z), 2*(x*z+w*y)),
        (2*(x*y+w*z), w*w-x*x+y*y-z*z, 2*(y*z-w*x)),
        (2*(x*z-w*y), 2*(y*z+w*x), w*w-x*x-y*y+z*z))]


def matrix_product(a, b):
    return [[sum(a[i][k]*b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def apply(matrix, point):
    result = []
    for row in matrix:
        value = exact(F(0))
        for coefficient, coordinate in zip(row, point):
            value = add(value, mul(exact(coefficient), coordinate))
        result.append(value)
    return result


whole_checks = point_checks = 0
with open(sys.argv[1], encoding="utf-8") as log:
    for line in log:
        marker = "retarget_absolute_interval_reference="
        if marker not in line:
            continue
        data = json.loads(line.split(marker, 1)[1])
        correction = rotation(data["correction"])
        inverse = [list(row) for row in zip(*correction)]
        outer = matrix_product(rotation(data["target_rotation"]), correction)
        bind_inverse = [list(row) for row in zip(*rotation(data["source_bind_rotation"]))]
        outer = matrix_product(outer, bind_inverse)
        basis = rotation(data["basis"])
        point = apply(inverse, [exact(f32(x)) for x in data["point"]])
        point = apply(rotation(data["source_initial_rotation"]), point)
        start, end = map(F, data["times"])
        phases = [(start, end)] + [exact(start+(end-start)*F(i, 4)) for i in range(5)]
        for index, phase in enumerate(phases):
            sine, cosine = trig(mul(pi, mul(exact(F(1, 2)), phase)))
            turned = [add(mul(cosine, point[0]), mul(sine, point[2])), point[1],
                      add(mul(neg(sine), point[0]), mul(cosine, point[2]))]
            result = apply(outer, turned)
            source_position = [add(exact(f32(initial)-f32(bind)), mul(exact(f32(delta)), phase))
                               for initial, bind, delta in zip(data["source_initial_translation"],
                                                              data["source_bind_translation"],
                                                              data["end_translation"])]
            delta = apply(basis, source_position)
            for axis in range(3):
                value = add(exact(f32(data["target_translation"][axis])),
                            add(mul(exact(f32(data["scale"])), delta[axis]), result[axis]))
                lo, hi = map(F, data["image"][axis])
                assert lo <= value[0] <= value[1] <= hi, (whole_checks, index, axis)
                if index == 0:
                    whole_checks += 1
                else:
                    point_checks += 1
assert whole_checks == 72 and point_checks == 360, (whole_checks, point_checks)
print(f"Whole continuous interval coordinate checks: {whole_checks}")
print(f"Independent interior/end pose coordinate checks: {point_checks}")

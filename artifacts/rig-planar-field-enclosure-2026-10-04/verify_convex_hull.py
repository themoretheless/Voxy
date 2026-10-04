"""Independent exact rational oracle for interval interpolation/hull intersection."""
from fractions import Fraction as F
from math import nextafter, inf
import random
r = random.Random(547)
def add(a, b):
    return nextafter(a[0] + b[0], -inf), nextafter(a[1] + b[1], inf)
def mul(a, b):
    values = [x * y for x in a for y in b]
    return nextafter(min(values), -inf), nextafter(max(values), inf)
count = 0
for _ in range(2000):
    a = sorted([r.uniform(-100, 100), r.uniform(-100, 100)])
    b = sorted([r.uniform(-100, 100), r.uniform(-100, 100)])
    u = sorted([r.random(), r.random()])
    one = nextafter(1 - u[1], -inf), nextafter(1 - u[0], inf)
    value = add(mul(a, one), mul(b, u))
    value = max(value[0], min(a[0], b[0])), min(value[1], max(a[1], b[1]))
    for x in a:
        for y in b:
            for t in u:
                reference = (1 - F(t)) * F(x) + F(t) * F(y)
                assert F(value[0]) <= reference <= F(value[1])
                count += 1
print(count, 'exact rational corner checks passed')

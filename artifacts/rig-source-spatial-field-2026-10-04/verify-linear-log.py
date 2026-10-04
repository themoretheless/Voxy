import ast
import sys
from fractions import Fraction as F
from math import isqrt
from pathlib import Path

# Rational alternating-series bounds, independent of the engine float evaluator.
def small(x):
    total = F(0)
    for k in range(80):
        total += (-1)**k * x**(2*k+1) / (2*k+1)
    remainder = abs(x**161 / 161)
    return total-remainder, total+remainder

a = small(F(1, 5)); b = small(F(1, 239))
pi = (16*a[0]-4*b[1], 16*a[1]-4*b[0])
def unit(x):
    if x <= F(1, 2):
        return small(x)
    s = small((x-1)/(x+1))
    return pi[0]/4+s[0], pi[1]/4+s[1]
def angle(y, x):
    if x == 0:
        return pi[0]/2, pi[1]/2
    if y <= x:
        return unit(y/x)
    s = unit(x/y)
    return pi[0]/2-s[1], pi[1]/2-s[0]
def sqrt_bounds(x):
    scale = 10**200
    lower = isqrt(x.numerator*scale*scale//x.denominator)
    return F(lower, scale), F(lower+1, scale)

count = 0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'SOURCE_LINEAR_LOG ' not in line:
        continue
    keys, times, bounds = ast.literal_eval(line.split('SOURCE_LINEAR_LOG ', 1)[1])
    a,b = [[F.from_float(v) for v in q] for q in keys]
    vector = [b[0]*a[3]-b[3]*a[0]-b[1]*a[2]+b[2]*a[1],
              b[1]*a[3]-b[3]*a[1]-b[2]*a[0]+b[0]*a[2],
              b[2]*a[3]-b[3]*a[2]-b[0]*a[1]+b[1]*a[0]]
    dot = sum(x*y for x,y in zip(a,b))
    if dot < 0:
        dot = -dot; vector = [-v for v in vector]
    norm = sqrt_bounds(sum(v*v for v in vector))
    lo = angle(norm[0], dot)[0]; hi = angle(norm[1], dot)[1]
    duration = F.from_float(times[1])-F.from_float(times[0])
    factor = 2*lo/(norm[1]*duration), 2*hi/(norm[0]*duration)
    for v, actual in zip(vector,bounds):
        reference = sorted([v*factor[0], v*factor[1]])
        assert F.from_float(actual[0]) <= reference[0] <= reference[1] <= F.from_float(actual[1]), (keys, actual)
        count += 1
assert count == 18, count
print(f'PASS: {count} source LINEAR angular components; rational sqrt and atan bounds, no tolerance')

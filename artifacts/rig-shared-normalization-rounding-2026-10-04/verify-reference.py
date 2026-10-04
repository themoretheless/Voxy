import ast
from fractions import Fraction as F
from pathlib import Path
count = 0
for line in Path('artifacts/rig-shared-normalization-rounding-2026-10-04/reference.log').read_text().splitlines():
    if 'NORMALIZED_COMPOSITION ' not in line:
        continue
    a,b,actual,caps = ast.literal_eval(line.split('NORMALIZED_COMPOSITION ',1)[1])
    a,b = [[F.from_float(v) for v in q] for q in [a,b]]
    product = [a[3]*b[0]+a[0]*b[3]+a[1]*b[2]-a[2]*b[1], a[3]*b[1]-a[0]*b[2]+a[1]*b[3]+a[2]*b[0], a[3]*b[2]+a[0]*b[1]-a[1]*b[0]+a[2]*b[3], a[3]*b[3]-a[0]*b[0]-a[1]*b[1]-a[2]*b[2]]
    assert sum(v*v for v in a)==sum(v*v for v in b)==sum(v*v for v in product)==1
    for source,stored,cap in zip(product,actual,caps):
        assert abs(source-F.from_float(stored)) <= F.from_float(cap)
        count += 1
assert count==12, count
print('PASS: 12 composition components enclose exact unit-source Hamilton products; no tolerance')

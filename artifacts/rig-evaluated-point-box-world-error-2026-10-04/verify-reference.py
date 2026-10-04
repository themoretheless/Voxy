import ast
from fractions import Fraction as F
from pathlib import Path
count = 0
for line in Path('artifacts/rig-evaluated-point-box-world-error-2026-10-04/reference.log').read_text().splitlines():
    if 'MAPPED_EVALUATED_POINT ' not in line: continue
    point, actual, axes, radius = ast.literal_eval(line.split('MAPPED_EVALUATED_POINT ',1)[1])
    x,y,z = [F.from_float(v) for v in point]
    source = [F(65536)+F(5,512)-2*(F(1,2)-z), F(2)-2*(F(1,8)-x), F(-3)-2*(y-F(1,4))]
    differences = [abs(exact-F.from_float(stored)) for exact,stored in zip(source,actual)]
    for difference,cap in zip(differences,axes):
        assert difference <= F.from_float(cap)
        count += 1
    assert sum(differences) <= F.from_float(radius)
    count += 1
assert count == 32, count
print('PASS: 24 axis and 8 L1 discrepancies enclose exact dyadic half-turn, frame permutation, signed scale and f32 publication; no tolerance')

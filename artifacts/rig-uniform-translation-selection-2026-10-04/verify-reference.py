import ast
from fractions import Fraction as F
from pathlib import Path
count = 0
nonzero = 0
for line in Path('artifacts/rig-uniform-translation-selection-2026-10-04/reference.log').read_text().splitlines():
    if 'UNIFORM_TRANSLATION_SELECTION ' not in line: continue
    span, fraction, actual, caps = ast.literal_eval(line.split('UNIFORM_TRANSLATION_SELECTION ', 1)[1])
    u = F.from_float(fraction)
    if span == 0:
        source = [F(250)*u, -F(1,2)*u, F(1,32)*u]
    else:
        source = [F(250)-F(250)*u, -F(1,2)+F(3,2)*u, F(1,32)-F(1,8)*u]
    for exact,stored,cap in zip(source,actual,caps):
        error = abs(exact-F.from_float(stored))
        assert error <= F.from_float(cap)
        nonzero += error != 0
        count += 1
assert count == 42, count
assert nonzero > 0, nonzero
print(f'PASS: {count} exact component discrepancies enclosed by whole-path selection caps; {nonzero} nonzero observed errors; no tolerance')

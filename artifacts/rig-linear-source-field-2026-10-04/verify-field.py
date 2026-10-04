import ast, runpy, sys
from fractions import Fraction as F
from pathlib import Path
proof = runpy.run_path('artifacts/rig-source-spatial-field-2026-10-04/verify-linear-log.py')
pi = proof['pi']
count = 0
for line in Path(sys.argv[2]).read_text().splitlines():
    if 'LINEAR_SOURCE_FIELD ' not in line: continue
    linear, angular, loop_linear, loop_angular, _, _, _ = ast.literal_eval(line.split('LINEAR_SOURCE_FIELD ', 1)[1])
    for boxes, reference in [(linear,[(F(0),F(0)),(F(0),F(0)),(2*pi[0],2*pi[1])]),
                              (angular,[(F(0),F(0)),pi,(F(0),F(0))]),
                              (loop_linear,[(F(0),F(0)),(F(0),F(0)),(2*pi[0],2*pi[1])]),
                              (loop_angular,[(F(0),F(0)),pi,(F(0),F(0))])]:
        for actual, exact in zip(boxes,reference):
            assert F.from_float(actual[0])<=exact[0]<=exact[1]<=F.from_float(actual[1]),actual
            count += 1
assert count==12,count
print(f'PASS: {count} local/loop source field components, exact rational pi enclosure')

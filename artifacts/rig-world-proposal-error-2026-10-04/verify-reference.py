import ast
from fractions import Fraction as F
from pathlib import Path
count=0
for line in Path('artifacts/rig-world-proposal-error-2026-10-04/reference.log').read_text().splitlines():
    if 'WORLD_POSE_ERROR ' not in line: continue
    axes,radius=ast.literal_eval(line.split('WORLD_POSE_ERROR ',1)[1])
    for actual,expected in zip(axes,[F(1,8),F(1,4),F(1,2)]):
        assert F.from_float(actual)>=expected
        count+=1
    assert F.from_float(radius)>=F(7,8)
    count+=1
assert count==4,count
print('PASS: 3 world-axis caps and whole-body L1 cap contain exact dyadic affine displacement; no tolerance')

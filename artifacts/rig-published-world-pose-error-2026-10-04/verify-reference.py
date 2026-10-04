import ast
from fractions import Fraction as F
from pathlib import Path
count = 0
for line in Path('artifacts/rig-published-world-pose-error-2026-10-04/reference.log').read_text().splitlines():
    if 'PUBLISHED_WORLD_ERROR ' not in line:
        continue
    axes, radius = ast.literal_eval(line.split('PUBLISHED_WORLD_ERROR ', 1)[1])
    for cap, exact in zip(axes, [F(1, 512), F(0), F(0)]):
        assert F.from_float(cap) >= exact
        count += 1
    assert F.from_float(radius) >= F(1, 512)
    count += 1
assert count == 4, count
print('PASS: published axis and L1 caps enclose exact dyadic f32 displacement; no tolerance')

import ast
import sys
from fractions import Fraction as F
from pathlib import Path

# Check that the supplied control error contains the rational thirds.
for stored, exact in [(1 / 3, F(1, 3)), (2 / 3, F(2, 3))]:
    assert abs(F.from_float(stored) - exact) <= F.from_float(1e-15)
count = 0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'SOURCE_ANGULAR_COMPONENTS ' not in line:
        continue
    u, bounds = ast.literal_eval(line.split('SOURCE_ANGULAR_COMPONENTS ', 1)[1])
    u = F.from_float(u)
    denominator = 1 + u**2 + u**4 + u**6
    # 2 vec(raw prime * conjugate(raw)) / squared norm:
    expected = [2 * (1 + u**4) / denominator,
                4 * (u - u**3) / denominator,
                8 * u**2 / denominator]
    for interval, exact in zip(bounds, expected):
        assert F.from_float(interval[0]) <= exact <= F.from_float(interval[1]), (u, interval, exact)
        count += 1
assert count == 51, count
print(f'PASS: {count} source angular components, exact rational comparison, no tolerance')

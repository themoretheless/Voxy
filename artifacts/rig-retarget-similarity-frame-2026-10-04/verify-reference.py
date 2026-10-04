"""Check actual Rust enclosures against exact rational normalized rotations."""
import json
import struct
import sys
from fractions import Fraction as F


def stored_f32(value):
    return F(struct.unpack("<f", struct.pack("<f", value))[0])


checks = 0
with open(sys.argv[1], encoding="utf-8") as log:
    for line in log:
        marker = "retarget_similarity_reference="
        if marker not in line:
            continue
        data = json.loads(line.split(marker, 1)[1])
        x, y, z, w = map(stored_f32, data["basis"])
        norm_squared = x*x + y*y + z*z + w*w
        matrix = [
            [w*w+x*x-y*y-z*z, 2*(x*y-w*z), 2*(x*z+w*y)],
            [2*(x*y+w*z), w*w-x*x+y*y-z*z, 2*(y*z-w*x)],
            [2*(x*z-w*y), 2*(y*z+w*x), w*w-x*x-y*y+z*z],
        ]
        source = list(map(stored_f32, data["source"]))
        target = list(map(stored_f32, data["target"]))
        point = list(map(F, data["point"]))
        scale = stored_f32(data["scale"])
        for axis in range(3):
            exact = target[axis] + scale * sum(
                matrix[axis][i] * (point[i] - source[i]) for i in range(3)
            ) / norm_squared
            lo, hi = map(F, data["bounds"][axis])
            assert lo <= exact <= hi, (data, axis, exact, lo, hi)
            checks += 1
assert checks == 180, checks
print(f"Exact rational retarget similarity checks passed: {checks}")

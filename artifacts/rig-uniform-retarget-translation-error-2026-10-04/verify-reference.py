"""Independent exact same-time translation discrepancies against uniform caps."""
import json
import struct
import sys
from fractions import Fraction as F


def f32(value):
    return F(struct.unpack("<f", struct.pack("<f", value))[0])


checks = radii = 0
with open(sys.argv[1], encoding="utf-8") as log:
    for line in log:
        marker = "retarget_translation_error_reference="
        if marker not in line:
            continue
        data = json.loads(line.split(marker, 1)[1])
        x, y, z, w = map(f32, data["basis"])
        norm = x*x+y*y+z*z+w*w
        matrix = [
            [w*w+x*x-y*y-z*z, 2*(x*y-w*z), 2*(x*z+w*y)],
            [2*(x*y+w*z), w*w-x*x+y*y-z*z, 2*(y*z-w*x)],
            [2*(x*z-w*y), 2*(y*z+w*x), w*w-x*x-y*y+z*z],
        ]
        delta = [f32(a)-f32(b) for a, b in zip(data["source_input"], data["source_bind"])]
        total = F(0)
        for axis in range(3):
            ideal = f32(data["target_bind"][axis]) + f32(data["scale"]) * sum(
                matrix[axis][i]*delta[i] for i in range(3)
            )/norm
            error = abs(ideal-f32(data["evaluated"][axis]))
            assert error <= F(data["axes"][axis]), (checks, data, ideal)
            total += error
            checks += 1
        assert total <= F(data["radius"]), (radii, data)
        if data["scale"] == 0:
            assert data["axes"] == [0, 0, 0] and data["radius"] == 0
        if data["source_error"] == [0, 0, 0]:
            assert data["radius"] < (F(1, 10000) if data["scale"] <= 2 else F(100))
        radii += 1
assert checks == 324 and radii == 108, (checks, radii)
print(f"Uniform translation axis discrepancy checks: {checks}")
print(f"Uniform translation L1 discrepancy checks: {radii}")

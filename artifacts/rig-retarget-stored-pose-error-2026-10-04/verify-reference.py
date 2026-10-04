"""Independent exact rational retarget pose and actual f32 point discrepancy."""
import json
import struct
import sys
from fractions import Fraction as F


def f32(value):
    return F(struct.unpack("<f", struct.pack("<f", value))[0])


def quat_product(a, b):
    x, y, z, w = a
    u, v, s, t = b
    return (w*u+x*t+y*s-z*v, w*v-x*s+y*t+z*u,
            w*s+x*v-y*u+z*t, w*t-x*u-y*v-z*s)


def conjugate(q):
    return (-q[0], -q[1], -q[2], q[3])


def rotate(q, point):
    x, y, z, w = q
    norm = sum(component*component for component in q)
    rows = ((w*w+x*x-y*y-z*z, 2*(x*y-w*z), 2*(x*z+w*y)),
            (2*(x*y+w*z), w*w-x*x+y*y-z*z, 2*(y*z-w*x)),
            (2*(x*z-w*y), 2*(y*z+w*x), w*w-x*x-y*y+z*z))
    return [sum(a*b for a, b in zip(row, point))/norm for row in rows]


points = 0
with open(sys.argv[1], encoding="utf-8") as log:
    for line in log:
        marker = "retarget_pose_reference="
        if marker not in line:
            continue
        data = json.loads(line.split(marker, 1)[1])
        q = {key: list(map(f32, data[key])) for key in (
            "source_rotation", "target_rotation", "correction", "basis", "animated_rotation")}
        translation_delta = [f32(a)-f32(b) for a, b in zip(
            data["animated_translation"], data["source_translation"])]
        translation_delta = rotate(q["basis"], translation_delta)
        position = [f32(t)+f32(data["scale"])*delta for t, delta in zip(
            data["target_translation"], translation_delta)]
        rotation = q["target_rotation"]
        # Normalization factors cancel from the homogeneous rotation matrix.
        for factor in (q["correction"], conjugate(q["source_rotation"]),
                       q["animated_rotation"], conjugate(q["correction"])):
            rotation = quat_product(rotation, factor)
        rotated = rotate(rotation, list(map(f32, data["point"])))
        exact = [a+b for a, b in zip(position, rotated)]
        total = F(0)
        for axis in range(3):
            lo, hi = map(F, data["image"][axis])
            assert lo <= exact[axis] <= hi, (points, axis, "image")
            error = abs(exact[axis] - f32(data["evaluated"][axis]))
            assert error <= F(data["axes"][axis]), (points, axis, "axis error")
            total += error
        assert total <= F(data["radius"]), (points, "L1 error")
        points += 1
assert points == 96, points
print(f"Exact retarget pose image checks: {points*3}")
print(f"Actual f32 retarget point discrepancy axis checks: {points*3}")
print(f"Actual f32 retarget point discrepancy L1 checks: {points}")

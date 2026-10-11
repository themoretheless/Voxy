"""Locate the strain introduced between recorded native hair solver phases."""
import ast
import json
import math
import re
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text()
failure = re.search(r"signed_strain=([-\d.e+]+) endpoints=(\[.*\])", source)
assert failure
endpoints = ast.literal_eval(failure[2])
rest = math.dist(*endpoints) / (1 + float(failure[1]))
rows = []
for line in source.splitlines():
    if "HAIR PHASE TRACE frame=40 native" not in line:
        continue
    points = ast.literal_eval(line.split("positions: ", 1)[1].split(", velocities:", 1)[0])
    rows.append({
        "phase": re.search(r'phase: "([^"]+)"', line)[1],
        "substep": int(re.search(r"substep: (\d+)", line)[1]),
        "iteration": int(re.search(r"iteration: (\d+)", line)[1]),
        "segment14_strain": math.dist(points[14], points[15]) / rest - 1,
    })
assert rows
Path(sys.argv[2]).write_text(json.dumps({"rest_length_m": rest, "frame": 40, "rod": 9, "segment": 14, "phases": rows}, indent=2) + "\n")

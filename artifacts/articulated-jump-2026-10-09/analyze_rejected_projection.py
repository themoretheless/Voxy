"""Extract selected-component gap observations from rejected staged diagnostics.

These observations are not the full-system merit or a clearance certificate.
"""
import json
import re
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text()
rows = []
for piece in source.split("HairContactProjectionDiagnostic {")[1:]:
    after = piece.split("after:", 1)[1].split("pairs:", 1)[0]
    gaps = [float(v) for v in re.findall(r"gap_m: ([-\d.e+]+)", after)]
    rows.append({
        "substep": int(re.search(r"substep: (\d+)", piece)[1]),
        "structural_iteration": int(re.search(r"structural_iteration: (\d+)", piece)[1]),
        "iteration": int(re.search(r"\n\s+iteration: (\d+)", piece)[1]),
        "component_min_gap_m": min(gaps, default=0.0),
        "component_negative_gap_norm": sum(min(0.0, v) ** 2 for v in gaps) ** 0.5,
        "observed_contacts": len(gaps),
    })
assert rows
Path(sys.argv[2]).write_text(json.dumps(rows, indent=2) + "\n")

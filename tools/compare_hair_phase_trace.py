#!/usr/bin/env python3
"""Compare preserved paired per-rod traces without rerunning the simulation."""
import argparse
import ast
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("log", type=Path)
args = parser.parse_args()
pattern = re.compile(
    r'HAIR PHASE TRACE frame=(\d+) (native|external) HairPhaseDiagnostic '
    r'\{ phase: "([^"]+)", substep: (\d+), iteration: (\d+), rod: (\d+), '
    r'positions: (.*?), velocities: (.*?), orientations: (.*?), '
    r'contacts: (.*?), relative_strains:'
)
traces = {"native": [], "external": []}
for line in args.log.read_text().splitlines():
    match = pattern.search(line)
    if match:
        frame, owner, phase, substep, iteration, rod, positions, velocities, orientations, contacts = match.groups()
        vectors = tuple(ast.literal_eval(v) for v in (positions, velocities, orientations))
        traces[owner].append(((int(frame), phase, int(substep), int(iteration), int(rod)), vectors, contacts.count("HairContactDiagnostic {")))
if not traces["native"] or not traces["external"]:
    parser.error("paired traces are not available yet; this does not establish a terminal run")
records = []
for index, ((key_a, vectors_a, contacts_a), (key_b, vectors_b, contacts_b)) in enumerate(zip(traces["native"], traces["external"])):
    if key_a != key_b or any(len(a) != len(b) or any(len(x) != width for x in a + b) for a, b, width in zip(vectors_a, vectors_b, (3, 3, 4))):
        parser.error(f"trace alignment/shape mismatch at index {index}: {key_a} / {key_b}")
    a, b = vectors_a[0], vectors_b[0]
    error, point, axis = max((abs(x[c] - y[c]), i, c) for i, (x, y) in enumerate(zip(a, b)) for c in range(3))
    velocity_error = max(abs(x[c] - y[c]) for x, y in zip(vectors_a[1], vectors_b[1]) for c in range(3))
    rotation_error = max(abs(x[c] - y[c]) for x, y in zip(vectors_a[2], vectors_b[2]) for c in range(4))
    records.append(dict(frame=key_a[0], phase=key_a[1], substep=key_a[2], iteration=key_a[3], rod=key_a[4], maximum_position_error_m=error, maximum_velocity_error_m_s=velocity_error, maximum_quaternion_component_error=rotation_error, native_contact_count=contacts_a, external_contact_count=contacts_b, point=point, axis=axis))
if len(traces["native"]) != len(traces["external"]):
    parser.error("trace counts differ; partial comparison cannot qualify the trajectory")
print(json.dumps(dict(scope="observed per-rod phases only; not whole-model qualification", first_nonzero=next((r for r in records if r["maximum_position_error_m"] > 0), None), first_over_position_gate=next((r for r in records if r["maximum_position_error_m"] >= 1e-6), None), first_velocity_difference=next((r for r in records if r["maximum_velocity_error_m_s"] > 0), None), first_contact_count_difference=next((r for r in records if r["native_contact_count"] != r["external_contact_count"]), None), phases=records), indent=2))

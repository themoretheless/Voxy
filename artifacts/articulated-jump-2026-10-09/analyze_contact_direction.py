"""Audit frozen mesh-contact merit for the rejected actual geometry direction."""
import json
from pathlib import Path
import argparse
parser = argparse.ArgumentParser()
parser.add_argument("capture", type=Path)
args = parser.parse_args()
data = json.loads(args.capture.read_text())
contacts = data["mesh_contacts"]
scales = [1.0] + [2.0 ** -i for i in range(1, 13)]
initial = sum(min(c["gap"], 0.0) ** 2 for c in contacts)
slope = sum(2.0 * min(c["gap"], 0.0) * c["directional_gap"] for c in contacts)
result = {
    "iteration": data["iteration"],
    "mesh_contact_count": len(contacts),
    "actual_before_merit_all_contacts": data["before_merit"],
    "actual_full_merit_all_contacts": data["full_merit"],
    "frozen_mesh_initial_merit": initial,
    "frozen_mesh_directional_derivative": slope,
    "frozen_mesh_trials": [
        {"scale": scale, "merit": sum(min(c["gap"] + scale * c["directional_gap"], 0.0) ** 2 for c in contacts)}
        for scale in scales
    ],
    "scope": "Frozen mesh planes only; excludes strand gaps and newly discovered mesh features."
}
print(json.dumps(result, indent=2))

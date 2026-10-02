#!/usr/bin/env python3
"""Compare exported pore states on the same mesh; finest state is not exact truth.

Usage: compare_pore_states.py REFERENCE.csv COARSER.csv [COARSER.csv ...]
Outputs unweighted pressure/concentration RMS and max differences in SI units.
When all matching .blood.csv files exist, also compares vascular states and
combined inventories. It rejects mismatched cell/compartment IDs and invalid data.
"""
import argparse
import csv
import json
import math
from pathlib import Path


def read_state(path, identifier):
    with path.open(newline="") as source:
        reader = csv.DictReader(source)
        required = {identifier, "pressure_pa", "fluid_volume_m3", "protein_kg"}
        if not required.issubset(reader.fieldnames or []):
            raise ValueError(f"{path}: missing state columns")
        result = {}
        for row in reader:
            key = int(row[identifier])
            if key < 0 or key in result:
                raise ValueError(f"{path}: duplicate/negative {identifier} ID")
            pressure, volume, mass = (float(row[k]) for k in
                ("pressure_pa", "fluid_volume_m3", "protein_kg"))
            if not all(map(math.isfinite, (pressure, volume, mass))) or volume <= 0 or mass < 0:
                raise ValueError(f"{path}: invalid fluid/protein state")
            concentration = mass / volume
            if not math.isfinite(concentration):
                raise ValueError(f"{path}: concentration overflow")
            result[key] = (pressure, volume, mass, concentration)
        if not result:
            raise ValueError(f"{path}: empty state")
        return result


def compare(reference, candidate):
    if reference.keys() != candidate.keys():
        raise ValueError("incompatible mesh/compartment identifiers")
    report = {"records": len(reference)}
    for index, name in ((0, "pressure_pa"), (3, "concentration_kg_per_m3")):
        differences = [candidate[k][index] - reference[k][index] for k in reference]
        report[name] = {"rms_difference": math.hypot(*differences) / math.sqrt(len(differences)),
                        "max_difference": max(map(abs, differences))}
    return report


def inventories(*states):
    return {"fluid_volume_m3": math.fsum(v[1] for state in states for v in state.values()),
            "protein_kg": math.fsum(v[2] for state in states for v in state.values())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidates", nargs="+", type=Path)
    args = parser.parse_args()
    reference = read_state(args.reference, "cell")
    reference_blood_path = Path(str(args.reference) + ".blood.csv")
    for path in args.candidates:
        state = read_state(path, "cell")
        report = {"reference": str(args.reference), "candidate": str(path),
                  "tissue": compare(reference, state)}
        blood_path = Path(str(path) + ".blood.csv")
        if reference_blood_path.exists() != blood_path.exists():
            raise ValueError("both or neither state must include vascular inventory")
        if blood_path.exists():
            ref_blood = read_state(reference_blood_path, "compartment")
            blood = read_state(blood_path, "compartment")
            report["blood"] = compare(ref_blood, blood)
            ref_totals, totals = inventories(reference, ref_blood), inventories(state, blood)
            report["combined_inventory_difference"] = {
                k: totals[k] - ref_totals[k] for k in totals}
        print(json.dumps(report, allow_nan=False))


if __name__ == "__main__":
    main()

"""Summarize actual element stress CSVs using reference-volume weights."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path


def summarize(path):
    rows = list(csv.DictReader(path.open()))
    if not rows:
        raise ValueError(f"empty stress file: {path}")
    for row in rows:
        for key in ("reference_volume_m3", "von_mises_pa", "j", "reference_z_m"):
            if not math.isfinite(float(row[key])):
                raise ValueError(f"nonfinite {key}: {path}")
        if float(row["reference_volume_m3"]) <= 0 or float(row["j"]) <= 0:
            raise ValueError(f"invalid element volume: {path}")
    rows.sort(key=lambda row: float(row["von_mises_pa"]))
    volume = sum(float(row["reference_volume_m3"]) for row in rows)
    cumulative = 0.0
    quantiles = {}
    for row in rows:
        cumulative += float(row["reference_volume_m3"])
        for fraction in (0.95, 0.99, 0.999):
            if str(fraction) not in quantiles and cumulative >= fraction * volume:
                quantiles[str(fraction)] = float(row["von_mises_pa"])
    peak = rows[-1]
    return {
        "source": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "elements": len(rows),
        "reference_volume_m3": volume,
        "mean_von_mises_pa": sum(float(r["reference_volume_m3"]) * float(r["von_mises_pa"]) for r in rows) / volume,
        "volume_weighted_quantiles_pa": quantiles,
        "peak_von_mises_pa": float(peak["von_mises_pa"]),
        "peak_element": int(peak["element"]),
        "peak_region": int(peak["region"]),
        "peak_reference_z_m": float(peak["reference_z_m"]),
        "peak_base_nodes": int(peak["base_nodes"]),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    args.output.write_text(json.dumps([summarize(path) for path in args.files], indent=2) + "\n")

"""Summarize all nine alternating pairs, without trimming noisy samples."""
import csv
import json
import pathlib
import re
import statistics

root = pathlib.Path(__file__).resolve().parent
summary = []
for kind in ("upload", "rigid"):
    for series in (1, 2):
        suffix = "csv" if kind == "upload" else "log"
        path = root / f"{kind}-pairs-{series}.{suffix}"
        if kind == "upload":
            with path.open() as source:
                rows = list(csv.DictReader(source))
        else:
            matches = re.findall(
                r"^(\d+),(\d+),(legacy|shared),([\d.]+),([\d.]+)$",
                path.read_text(), re.M,
            )
            rows = [dict(zip(("joints", "pair", "path", "cpu_us_per_frame", "drained_us_per_frame"), row)) for row in matches]
        dimension = "bytes" if kind == "upload" else "joints"
        for size in sorted({int(row[dimension]) for row in rows}):
            selected = [row for row in rows if int(row[dimension]) == size]
            for metric in ("cpu_us_per_frame", "drained_us_per_frame"):
                paired = {
                    name: {row["pair"]: float(row[metric]) for row in selected if row["path"] == name}
                    for name in ("legacy", "shared")
                }
                assert len(paired["legacy"]) == 9
                assert paired["legacy"].keys() == paired["shared"].keys()
                ratios = [paired["shared"][pair] / value for pair, value in paired["legacy"].items()]
                summary.append(dict(
                    kind=kind, series=series, size=size, metric=metric, pairs=len(ratios),
                    legacy_median=statistics.median(paired["legacy"].values()),
                    shared_median=statistics.median(paired["shared"].values()),
                    paired_ratio_median=statistics.median(ratios),
                    ratio_min=min(ratios), ratio_max=max(ratios),
                ))
print(json.dumps(summary, indent=2))

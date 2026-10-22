"""All measured pairs, including regressions; no trimming or selected reruns."""
import json
from pathlib import Path
import re
from statistics import median

root = Path(__file__).resolve().parent
summary = []
for series in (1, 2):
    source = (root / f"pairs-{series}.log").read_text()
    assert "test result: ok. 3 passed; 0 failed" in source
    rows = re.findall(r"^([a-z_]+),(\d+),(\d+),(legacy|shared),([\d.]+),([\d.]+)$", source, re.M)
    cases = sorted({row[:2] for row in rows})
    assert len(cases) == 10
    for path, count in cases:
        selected = [row for row in rows if row[:2] == (path, count)]
        for column, metric in ((4, "cpu_us"), (5, "drained_us")):
            values = {mode: {int(row[2]): float(row[column]) for row in selected if row[3] == mode}
                      for mode in ("legacy", "shared")}
            assert set(values["legacy"]) == set(values["shared"]) == set(range(1, 10))
            ratios = [values["shared"][pair] / values["legacy"][pair] for pair in range(1, 10)]
            summary.append(dict(path=path, count=int(count), series=series, metric=metric, pairs=9,
                                legacy_median=median(values["legacy"].values()),
                                shared_median=median(values["shared"].values()),
                                paired_ratio=median(ratios), ratio_min=min(ratios), ratio_max=max(ratios)))
print(json.dumps(summary, indent=2))

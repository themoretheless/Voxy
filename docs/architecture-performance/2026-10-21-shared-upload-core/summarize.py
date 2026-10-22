"""Summarize every pair, retaining the rejected coarse-protocol runs."""
import hashlib
import json
from pathlib import Path
import re
from statistics import median

root = Path(__file__).resolve().parent
baseline = root / 'shared_upload_before.rs'
fixture = root.parents[2] / 'crates/voxy_render/tests/fixtures/shared_upload_before.rs'
assert baseline.read_bytes() == fixture.read_bytes(), 'pre-move fixture changed'
results = []
for protocol in ('initial', 'interleaved'):
    for series in (1, 2):
        filename = f'initial-pairs-{series}.log' if protocol == 'initial' else f'pairs-{series}.log'
        text = (root / filename).read_text()
        assert 'test result: ok. 1 passed; 0 failed' in text
        fields = r',([\d.]+),([\d.]+)' if protocol == 'initial' else r',([\d.]+),([\d.]+),([\d.]+)'
        rows = re.findall(r'^([a-z_]+),(\d+),(\d+),(before|after)' + fields + r'$', text, re.M)
        assert len(rows) == 5 * 9 * 2
        for case in sorted({row[0] for row in rows}):
            selected = [row for row in rows if row[0] == case]
            metrics = ('cpu_us', 'drained_us') if protocol == 'initial' else ('upload_us', 'cpu_us', 'drained_us')
            for column, metric in enumerate(metrics, 4):
                values = {mode: {int(row[2]): float(row[column]) for row in selected if row[3] == mode}
                          for mode in ('before', 'after')}
                assert set(values['before']) == set(values['after']) == set(range(1, 10))
                ratios = [values['after'][pair] / values['before'][pair] for pair in range(1, 10)]
                results.append(dict(protocol=protocol, series=series, case=case, bytes=int(selected[0][1]),
                                    metric=metric, pairs=9,
                                    before_median=median(values['before'].values()), after_median=median(values['after'].values()),
                                    paired_ratio=median(ratios), ratio_min=min(ratios), ratio_max=max(ratios)))
print(json.dumps(dict(baseline_sha256=hashlib.sha256(baseline.read_bytes()).hexdigest(), results=results), indent=2))

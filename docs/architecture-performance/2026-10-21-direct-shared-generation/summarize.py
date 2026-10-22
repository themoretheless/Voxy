#!/usr/bin/env python3
"""Summarize paired CPU input-generation blocks, not GPU or FPS measurements."""
from collections import defaultdict
from pathlib import Path
import hashlib
import json
import random
import statistics

ROOT = Path(__file__).resolve().parent
ACCEPTED = (10, 11)
rows = defaultdict(dict)
for run in ACCEPTED:
    samples = json.loads((ROOT / f'clients-pairs-{run}.json').read_text())
    assert not any('rustc' in line or 'cpu-replay' in line for s in samples for line in s['clients'])
    for line in (ROOT / f'pairs-{run}.log').read_text().splitlines():
        if line.startswith('GEN,'):
            _, case, count, pair, mode, cpu, drained = line.split(',')
            rows[case, int(count)].setdefault((run, int(pair)), {})[mode] = float(cpu)
rng = random.Random(20261021)
summary = []
for (case, count), pairs in rows.items():
    assert len(pairs) == 18 and all(set(p) == {'before', 'after'} for p in pairs.values())
    ratios = [p['after'] / p['before'] for p in pairs.values()]
    bootstrap = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(10000))
    summary.append(dict(case=case, count=count, pairs=len(pairs),
        before_us=statistics.median(p['before'] for p in pairs.values()),
        after_us=statistics.median(p['after'] for p in pairs.values()),
        median_paired_ratio=statistics.median(ratios),
        bootstrap_95=[bootstrap[249], bootstrap[9749]],
        per_run_ratios={str(run): statistics.median(p['after']/p['before'] for (r,_),p in pairs.items() if r==run) for run in ACCEPTED}))
result = dict(accepted_runs=list(ACCEPTED), scope='CPU generation + validation + buffer selection + encoder finish/discard; no draw or queue submission',
    gpu_environment='Concurrent street_walk rendering; CPU-only results, no GPU/FPS qualification',
    exclusions={'1':'exploratory old writer plus concurrent compilation', '2':'old scalar writer before regression fix; monitor omitted street_walk',
        '3':'old scalar writer, concurrent compilation', '4':'aborted by this task with SIGTERM due GPU contention; partial log retained',
        '6':'CPU microbench with concurrent compilation and CPU replay', '7':'CPU microbench with concurrent compilation',
        '8':'CPU microbench with concurrent compilation and unstable A/A', '9':'CPU microbench with concurrent compilation'},
    cases=summary)
(ROOT / 'summary.json').write_text(json.dumps(result, indent=2)+'\n')
for row in summary:
    print(row['case'], row['count'], f"{row['before_us']:.3f} -> {row['after_us']:.3f} us", f"ratio {row['median_paired_ratio']:.4f}", row['bootstrap_95'])

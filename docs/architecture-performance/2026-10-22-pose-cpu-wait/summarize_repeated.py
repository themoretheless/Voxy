#!/usr/bin/env python3
"""Combine every completed phase from both series; never select favorable pairs."""
import csv
import json
import math
from pathlib import Path
import statistics
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / 'tools/performance'))
from paired_cpu_replay import read_phase


def main():
    phases = {case: [] for case in ('idle', 'motion', 'catchup')}
    hashes = None
    seen = set()
    for series in ('paired', 'repeat'):
        folder = HERE / series
        report = json.loads((folder / 'summary.json').read_text())
        digest = {v: report['binaries'][v]['sha256'] for v in ('before', 'after')}
        if hashes is not None and digest != hashes:
            raise ValueError('Changed executable across repeated series')
        hashes = digest
        for case in [r['scenario'] for r in report['summary']]:
            for number in range(1, report['pairs_per_scenario'] + 1):
                pair = {}
                for variant in ('before', 'after'):
                    path = folder / f'{case}-{number}-{variant}.csv'
                    if path.resolve() in seen:
                        raise ValueError('Duplicated phase')
                    seen.add(path.resolve())
                    lifecycle = json.loads(path.with_suffix('.run.json').read_text())
                    if lifecycle['timing_valid'] is not True or lifecycle['events']:
                        raise ValueError('Stopped or incomplete repeated phase')
                    result = read_phase(path)
                    if result['scenario'] != case or result.get('pose_profiled') is not False:
                        raise ValueError('Changed scenario/instrumentation')
                    with path.open() as source:
                        rows = list(csv.DictReader(source))
                    pair[variant] = (result, rows)
                phases[case].append(pair)
    summary = []
    for case, pairs in phases.items():
        if len(pairs) < 5:
            raise ValueError('Insufficient paired coverage')
        for key in ('seed', 'vertices', 'geometry_blake3', 'indices_blake3', 'state_blake3',
                    'warmup_frames', 'fixed_dt', 'fixed_ticks_total'):
            if len({pair[v][0][key] for pair in pairs for v in ('before', 'after')}) != 1:
                raise ValueError(f'Changed state/fixture: {case}/{key}')
        values = {}
        for variant in ('before', 'after'):
            rows = [row for pair in pairs for row in pair[variant][1]]
            total = sorted(float(row['total_cpu_us']) for row in rows)
            values[variant] = {
                'frames': len(total), 'pooled_us': {f'p{q}': total[math.ceil(len(total)*q/100)-1]
                                                  for q in (50, 95, 99)},
                'median_run_p99_us': statistics.median(pair[variant][0]['stages']['total_cpu']['p99_us'] for pair in pairs),
                'max_us': total[-1], 'over_8333_us': sum(x > 1e6/120 for x in total),
                'over_16667_us': sum(x > 1e6/60 for x in total)}
        pooled_ratio = values['after']['pooled_us']['p99'] / values['before']['pooled_us']['p99']
        median_ratio = values['after']['median_run_p99_us'] / values['before']['median_run_p99_us']
        summary.append({'scenario': case, 'pairs': len(pairs), **values,
                        'pooled_ratio': pooled_ratio, 'median_ratio': median_ratio,
                        'pass': max(pooled_ratio, median_ratio) <= (.95 if case == 'catchup' else 1.03)})
    result = {'binaries_sha256': hashes, 'all_phases_included': len(seen), 'focus': 'catchup',
              'summary': summary, 'cpu_gate': all(r['pass'] for r in summary)}
    (HERE / 'combined-summary.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()

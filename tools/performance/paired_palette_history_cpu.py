#!/usr/bin/env python3
"""Run CPU-only palette/history oracle pairs; no window or GPU is created."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess

CASES = {'palette_1', 'palette_2', 'history_repeat_large', 'history_change_large', 'history_change_small'}


def summarize(folder, binaries):
    rows = []
    for filename, marker, prefix in [('palette.log', 'PALETTE_PAIR', 'palette_'),
                                     ('history.log', 'HISTORY_PAIR', 'history_')]:
        for line in (folder / filename).read_text().splitlines():
            if line.startswith(marker + ','):
                _, round_number, variant, case, value = line.split(',')
                value = float(value)
                if not math.isfinite(value) or value <= 0:
                    raise ValueError('Invalid timing')
                rows.append((int(round_number), variant, prefix + case, value))
    if {row[2] for row in rows} != CASES:
        raise ValueError('Missing or unexpected cases')
    results = []
    for case in sorted(CASES):
        group = [row for row in rows if row[2] == case]
        expected = {(n, variant) for n in range(1, 6) for variant in ('before', 'after')}
        if len(group) != 10 or {(r[0], r[1]) for r in group} != expected:
            raise ValueError('Expected five unique complete pairs: ' + case)
        values = {v: statistics.median(r[3] for r in group if r[1] == v) for v in ('before', 'after')}
        ratios = [next(r[3] for r in group if r[:2] == (n, 'after')) /
                  next(r[3] for r in group if r[:2] == (n, 'before')) for n in range(1, 6)]
        results.append({'case': case, 'before_p50_us': values['before'], 'after_p50_us': values['after'],
                        'ratio': values['after'] / values['before'], 'paired_ratios': ratios})
    with (folder / 'pairs.csv').open('w', newline='') as output:
        writer = csv.writer(output)
        writer.writerow(['round', 'variant', 'case', 'p50_us'])
        writer.writerows(rows)
    report = {'rounds': 5, 'metric': 'median of per-run CPU p50; not application frame p99',
              'method': 'one executable per subsystem; before/after order reverses each pair',
              'summary': results,
              'controls_within_3_percent': all(.97 <= r['ratio'] <= 1.03 for r in results
                                              if r['case'].startswith('history_change_')),
              'binaries': {name: {'path': str(path), 'sha256': digest(path)} for name, path in binaries.items()}}
    (folder / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app-tests', type=Path, required=True)
    parser.add_argument('--render-tests', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--summarize-only', action='store_true')
    args = parser.parse_args()
    binaries = {'app': args.app_tests.resolve(), 'render': args.render_tests.resolve()}
    hashes = {name: digest(path) for name, path in binaries.items()}
    args.output.mkdir(parents=True, exist_ok=True)
    if not args.summarize_only:
        for name, test, filename in [
            ('app', 'animals::tests::paired_palette_cpu_bench', 'palette.log'),
            ('render', 'skinned_motion::admission_cache_tests::paired_history_cpu_bench', 'history.log')]:
            with (args.output / filename).open('w') as output:
                subprocess.run([str(binaries[name]), '--ignored', '--exact', test, '--nocapture'],
                               stdout=output, stderr=subprocess.STDOUT, check=True, timeout=120)
    if any(digest(path) != hashes[name] for name, path in binaries.items()):
        raise RuntimeError('Executable changed during measurement')
    print(json.dumps(summarize(args.output, binaries), indent=2))


if __name__ == '__main__':
    main()

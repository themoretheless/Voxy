#!/usr/bin/env python3
"""Collect five same-process pairs from the two CPU-only ignored test benchmarks."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess


def summarize(folder, binaries):
    rows = []
    for line in (folder / 'ticks-same-process.log').read_text().splitlines():
        if line.startswith('TICK_PAIR,'):
            _, round_number, variant, species, p50, p99, checksum = line.split(',')
            rows.append([int(round_number), variant, 'tick_' + species, float(p50), float(p99), checksum])
    for line in (folder / 'pose-same-process.log').read_text().splitlines():
        if line.startswith('KERNEL_PAIR,'):
            _, round_number, variant, layout, p50 = line.split(',')
            rows.append([int(round_number), variant, 'pose_' + layout, float(p50), None, ''])
    if any(not math.isfinite(row[3]) or row[3] <= 0 or (row[4] is not None and (not math.isfinite(row[4]) or row[4] < row[3])) for row in rows):
        raise RuntimeError("Invalid CPU timing sample")
    expected = {'tick_false', 'tick_true', 'pose_coherent', 'pose_alternating', 'pose_mixed_quarter'}
    if {row[2] for row in rows} != expected:
        raise RuntimeError('Missing or unexpected benchmark cases')
    results = []
    for case in sorted(expected):
        group = [row for row in rows if row[2] == case]
        if len(group) != 10 or {(row[0], row[1]) for row in group} != {
                (round_number, variant) for round_number in range(1, 6) for variant in ('before', 'after')}:
            raise RuntimeError('Expected five complete pairs for ' + case)
        checksums = {row[5] for row in group if row[5]}
        if checksums and len(checksums) != 1:
            raise RuntimeError('State mismatch in ' + case)
        values = {variant: statistics.median(row[3] for row in group if row[1] == variant)
                  for variant in ('before', 'after')}
        tails = {variant: statistics.median(row[4] for row in group if row[1] == variant and row[4] is not None)
                 for variant in ('before', 'after')} if case.startswith('tick_') else None
        results.append({'case': case, 'before_p50_us': values['before'], 'after_p50_us': values['after'],
                        'ratio': values['after'] / values['before'], 'run_p99_us': tails,
                        'checksum_identical': True if checksums else None})
    controls_stable = all(.97 <= result['ratio'] <= 1.03 for result in results
                          if result['case'] in ('pose_coherent', 'pose_alternating'))
    report = {'rounds': 5, 'metric': 'median of per-run CPU p50; run p99 is a CPU tick tail, not frame p99',
              'method': 'same compiled executable, same thread, alternating reference and candidate paths',
              'controls_stable_within_3_percent': controls_stable, 'summary': results,
              'binaries': {name: {'path': str(path), 'sha256': digest(path)} for name, path in binaries.items()}}
    with (folder / 'same-process.csv').open('w', newline='') as output:
        writer = csv.writer(output)
        writer.writerow(['round', 'variant', 'case', 'p50_us', 'p99_us', 'state_checksum'])
        writer.writerows(rows)
    (folder / 'same-process-summary.json').write_text(json.dumps(report, indent=2) + '\n')
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
                ('app', 'animals::tests::paired_transaction_cpu_bench', 'ticks-same-process.log'),
                ('render', 'skinned::coherent_pose_tests::paired_cache_policy_cpu_bench', 'pose-same-process.log')]:
            with (args.output / filename).open('w') as output:
                subprocess.run([str(binaries[name]), '--ignored', '--exact', test, '--nocapture'],
                               stdout=output, stderr=subprocess.STDOUT, check=True, timeout=120)
    if any(digest(path) != hashes[name] for name, path in binaries.items()):
        raise RuntimeError('A test executable changed during measurement')
    print(json.dumps(summarize(args.output, binaries), indent=2))


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Alternate immutable CPU replay binaries; retain every phase, no GPU/window."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import cpu_replay_process

SCENARIOS = ('idle', 'motion', 'catchup')


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def read_phase(path):
    metadata = path.with_suffix('.run.json')
    if metadata.exists() and json.loads(metadata.read_text()).get('timing_valid') is not True:
        raise ValueError('Invalid CPU process lifecycle')
    report = json.loads(path.with_suffix('.json').read_text())
    with path.open() as source:
        rows = list(csv.DictReader(source))
    if report['gpu_created'] is not False or report['measured_frames'] != len(rows) or len(rows) != 240:
        raise ValueError('Invalid replay coverage')
    for row in rows:
        for key, value in row.items():
            if key.endswith('_us') and (not math.isfinite(float(value)) or float(value) < 0):
                raise ValueError('Invalid replay timing')
    total = sorted(float(row['total_cpu_us']) for row in rows)
    for p in (50, 95, 99):
        expected = total[math.ceil(len(total) * p / 100) - 1]
        if not math.isclose(expected, report['stages']['total_cpu'][f'p{p}_us'], abs_tol=1e-5):
            raise ValueError('CSV/report percentile mismatch')
    report['over_8333_us'] = sum(value > 1000e3 / 120 for value in total)
    report['over_16667_us'] = sum(value > 1000e3 / 60 for value in total)
    if report.get('pose_profiled', False):
        with path.with_suffix('.pose.csv').open() as source:
            poses = list(csv.DictReader(source))
        if len(poses) != len(rows):
            raise ValueError('Incomplete pose clock coverage')
        for frame, (row, pose) in enumerate(zip(rows, poses)):
            if int(pose['frame']) != frame or (pose['active'] == 'true') != (int(row['fixed_ticks']) > 0):
                raise ValueError('Pose clock frame mismatch')
            for key, value in pose.items():
                if key.endswith('_us') and (not math.isfinite(float(value)) or float(value) < 0):
                    raise ValueError('Invalid pose clock timing')
    return report


def summarize(folder, binaries, scenarios=SCENARIOS, focus="motion"):
    results, raw = [], []
    for scenario in scenarios:
        phases = []
        pooled = {variant: [] for variant in ('before', 'after')}
        for number in range(1, 6):
            for variant in ('before', 'after'):
                report = read_phase(folder / f'{scenario}-{number}-{variant}.csv')
                if report['scenario'] != scenario:
                    raise ValueError('Scenario mismatch')
                phases.append((variant, report))
                with (folder / f'{scenario}-{number}-{variant}.csv').open() as source:
                    pooled[variant].extend(float(row['total_cpu_us']) for row in csv.DictReader(source))
                raw.append({'scenario': scenario, 'round': number, 'variant': variant,
                            'p50_us': report['stages']['total_cpu']['p50_us'],
                            'p95_us': report['stages']['total_cpu']['p95_us'],
                            'p99_us': report['stages']['total_cpu']['p99_us'],
                            'max_us': report['stages']['total_cpu']['max_us'],
                            'over_8333_us': report['over_8333_us'], 'over_16667_us': report['over_16667_us'],
                            'state_blake3': report['state_blake3']})
        identity_keys = ('seed', 'vertices', 'geometry_blake3', 'indices_blake3', 'state_blake3',
                         'warmup_frames', 'fixed_dt', 'fixed_ticks_total')
        if len({r.get('pose_profiled', False) for _, r in phases}) != 1:
            raise ValueError('Mixed pose instrumentation')
        for key in identity_keys:
            if len({r[key] for _, r in phases}) != 1:
                raise ValueError(f'Changed replay fixture/state: {scenario}/{key}')
        values = {variant: statistics.median(r['stages']['total_cpu']['p99_us'] for v, r in phases if v == variant)
                  for variant in ('before', 'after')}
        ratios = [phases[i + 1][1]['stages']['total_cpu']['p99_us'] /
                  phases[i][1]['stages']['total_cpu']['p99_us'] for i in range(0, 10, 2)]
        stages = {variant: {stage: statistics.median(r['stages'][stage]['sum_us'] / 240
                                                    for v, r in phases if v == variant)
                            for stage in phases[0][1]['stages']} for variant in ('before', 'after')}
        pooled_p99 = {variant: sorted(samples)[math.ceil(len(samples) * .99) - 1]
                      for variant, samples in pooled.items()}
        results.append({'pooled_p99_us': pooled_p99,
                        'pooled_p99_ratio': pooled_p99['after'] / pooled_p99['before'],
                        'scenario': scenario, 'before_p99_us': values['before'], 'after_p99_us': values['after'],
                        'ratio': values['after'] / values['before'], 'paired_p99_ratios': ratios,
                        'mean_stage_us': stages, 'state_blake3': phases[0][1]['state_blake3']})
    with (folder / 'pairs.csv').open('w', newline='') as output:
        writer = csv.DictWriter(output, fieldnames=raw[0].keys())
        writer.writeheader()
        writer.writerows(raw)
    report = {'pairs_per_scenario': 5, 'metric': 'median run and pooled CPU replay p99; not displayed frame p99', 'focus': focus,
              'summary': results,
              'cpu_gate': (all(max(r['ratio'], r['pooled_p99_ratio']) <=
                               (0.95 if r['scenario'] == focus else 1.03) for r in results)
                           if set(scenarios) == set(SCENARIOS) else None),
              'binaries': {v: {'path': str(p), 'sha256': digest(p)} for v, p in binaries.items()}}
    (folder / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--summarize-only', action='store_true')
    parser.add_argument('--profile-pose', action='store_true')
    parser.add_argument('--scenarios', nargs='+', choices=SCENARIOS, default=list(SCENARIOS))
    parser.add_argument('--focus', choices=SCENARIOS, default='motion')
    args = parser.parse_args()
    binaries = {'before': args.before.resolve(), 'after': args.after.resolve()}
    hashes = {v: digest(p) for v, p in binaries.items()}
    args.output.mkdir(parents=True, exist_ok=True)
    if not args.summarize_only:
        for number in range(1, 6):
            for scenario in args.scenarios:
                for variant in (('before', 'after') if number % 2 else ('after', 'before')):
                    path = args.output / f'{scenario}-{number}-{variant}.csv'
                    cpu_replay_process.run([str(binaries[variant]), '--cpu-replay', str(path),
                                            '--cpu-replay-scenario', scenario] +
                                           (['--cpu-replay-profile-pose'] if args.profile_pose else []),
                                           path.with_suffix('.log'))
                    read_phase(path)
                    print(f'{scenario} pair {number} {variant}: complete', flush=True)
    if any(digest(p) != hashes[v] for v, p in binaries.items()):
        raise RuntimeError('Binary changed during measurement')
    print(json.dumps(summarize(args.output, binaries, args.scenarios, args.focus), indent=2))


if __name__ == '__main__':
    main()

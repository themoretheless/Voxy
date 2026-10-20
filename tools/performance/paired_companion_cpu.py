#!/usr/bin/env python3
"""CPU-only paired companion ticks and synthetic pose admission; no window or GPU."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--companion', type=Path, required=True,
                        help='Saved release voxy_app test executable with the ignored CPU benchmark')
    parser.add_argument('--pose-before', type=Path, required=True)
    parser.add_argument('--pose-after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binaries = {name: getattr(args, name.replace('-', '_')).resolve()
                for name in ('companion', 'pose-before', 'pose-after')}
    hashes = {name: digest(path) for name, path in binaries.items()}
    args.output.mkdir(parents=True, exist_ok=True)
    rows, geometry = [], set()
    environment = os.environ.copy()
    environment.pop('VOXY_CPU_REFERENCE_TICK', None)
    for round_number in range(1, 6):
        for variant in ('before', 'after'):
            for name, path in binaries.items():
                if digest(path) != hashes[name]:
                    raise RuntimeError(f'Executable changed: {path}')
            env = environment.copy()
            if variant == 'before':
                env['VOXY_CPU_REFERENCE_TICK'] = '1'
            command = [str(binaries['companion']), '--ignored', '--exact',
                       'animals::tests::cpu_companion_tick_and_pose_bench', '--nocapture']
            result = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, check=True, timeout=120)
            (args.output / f'companion-{round_number}-{variant}.log').write_text(result.stdout)
            found = set()
            for line in result.stdout.splitlines():
                if line.startswith('CPU_PAIR,'):
                    _, case, value, checksum = line.split(',')
                    found.add(case)
                    if case.startswith('tick_'):
                        rows.append([round_number, variant, case, float(value), checksum])
                elif line.startswith('CPU_META,'):
                    geometry.add(line)
            if found != {'tick_false', 'tick_true', 'real_pose'}:
                raise RuntimeError('Missing companion measurements')
            pose = subprocess.check_output([str(binaries['pose-' + variant])], text=True,
                                           env=environment, timeout=30)
            (args.output / f'pose-{round_number}-{variant}.log').write_text(pose)
            cases = set()
            for line in pose.splitlines():
                case, value = line.split(',')
                cases.add(case)
                rows.append([round_number, variant, 'pose_' + case, float(value), ''])
            if cases != {'coherent', 'alternating', 'mixed_quarter'}:
                raise RuntimeError('Unexpected pose fixture set')
            print(f'CPU pair {round_number}/5 {variant} completed', flush=True)
    if len(geometry) != 1:
        raise RuntimeError('Companion geometry changed or metadata is missing')
    with (args.output / 'paired.csv').open('w', newline='') as output:
        writer = csv.writer(output)
        writer.writerow(['round', 'variant', 'case', 'p50_us', 'state_checksum'])
        writer.writerows(rows)
    summary = []
    for case in sorted({row[2] for row in rows}):
        group = [row for row in rows if row[2] == case]
        times = {variant: statistics.median(row[3] for row in group if row[1] == variant)
                 for variant in ('before', 'after')}
        checksums = {row[4] for row in group if row[4]}
        if checksums and len(checksums) != 1:
            raise RuntimeError(f'State checksum mismatch: {case}')
        summary.append({'case': case, 'before_us': times['before'], 'after_us': times['after'],
                        'ratio': times['after'] / times['before'],
                        'checksum_identical': True if checksums else None})
    report = {'binary_sha256': hashes, 'binaries': {k: str(v) for k, v in binaries.items()},
              'rounds': 5, 'companion_geometry': next(iter(geometry)),
              'method': 'same compiled companion fixture, clone-per-tick reference vs reusable transaction; separate pose binaries',
              'metric': 'median of run p50 CPU duration, not application frame p99', 'summary': summary}
    (args.output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()

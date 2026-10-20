#!/usr/bin/env python3
"""Alternate two already-built, instrumented versions; never rebuild during a pair."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess


def digest(path):
    with open(path, 'rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def compare(reports, focus):
    ratios = {}
    for scenario in ('idle', 'motion', 'water', 'street'):
        before = statistics.median(r['p99_ms'] for r in reports[scenario]['before'])
        after = statistics.median(r['p99_ms'] for r in reports[scenario]['after'])
        ratios[scenario] = {'before_p99_ms': before, 'after_p99_ms': after,
                            'ratio': after / before}
    adapters = {r['adapter'] for variants in reports.values() for runs in variants.values() for r in runs}
    valid = len(adapters) == 1 and all(not r['skipped_frames'] and
                any(kind in r['adapter'] for kind in ('device_type: IntegratedGpu', 'device_type: DiscreteGpu'))
                for variants in reports.values() for runs in variants.values() for r in runs)
    return {'scenarios': ratios, 'valid': valid,
            'candidate_accepted': valid and ratios[focus]['ratio'] <= .95
            and all(v['ratio'] <= 1.03 for v in ratios.values()),
            'target_met': valid and all(v['after_p99_ms'] <= 1000 / 120 for v in ratios.values())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('before', 'after', 'street-before', 'street-after'):
        parser.add_argument('--' + flag, type=Path, required=True)
    parser.add_argument('--focus', choices=('idle', 'motion', 'water', 'street'), required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binaries = {k: getattr(args, k.replace('-', '_')).resolve()
                for k in ('before', 'after', 'street-before', 'street-after')}
    hashes = {k: digest(p) for k, p in binaries.items()}
    args.output.mkdir(parents=True, exist_ok=True)
    metadata = {'binaries': {k: str(p) for k, p in binaries.items()}, 'sha256': hashes,
                'warmup_seconds': 10, 'measurement_seconds': 60, 'rounds': 5,
                'focus': args.focus}
    (args.output / 'manifest.json').write_text(json.dumps(metadata, indent=2))
    reports = {}
    for scenario in ('idle', 'motion', 'water', 'street'):
        reports[scenario] = {'before': [], 'after': []}
        for round_number in range(5):
            for variant in ('before', 'after'):
                key = ('street-' if scenario == 'street' else '') + variant
                binary = binaries[key]
                if digest(binary) != hashes[key]:
                    raise RuntimeError(f'Binary changed during measurement: {binary}')
                stem = args.output / f'{scenario}-{round_number + 1}-{variant}'
                command = [str(binary), '--profile', str(stem.with_suffix('.csv')),
                           '--profile-scenario', scenario]
                if scenario != 'street':
                    command += ['--backend', 'metal', '--present-mode', 'immediate']
                print(f'{scenario} pair {round_number + 1}/5 {variant}', flush=True)
                with stem.with_suffix('.log').open('w') as log:
                    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                                   check=True, timeout=300)
                report = json.loads(stem.with_suffix('.json').read_text())
                if report['scenario'] != scenario or report['measurement_seconds'] != 60:
                    raise RuntimeError('Unexpected scenario or measurement duration')
                reports[scenario][variant].append(report)
    result = compare(reports, args.focus)
    (args.output / 'comparison.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()

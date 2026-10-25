#!/usr/bin/env python3
"""Frozen, sequential full-cycle street replay. Never retries a noisy run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def digest(path):
    return hashlib.file_digest(open(path, 'rb'), 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--before', required=True, type=Path)
    parser.add_argument('--after', required=True, type=Path)
    parser.add_argument('--materials', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    if (args.output / 'manifest.json').exists():
        raise SystemExit('Refusing to replace an existing campaign')
    binaries = {'A': args.before.resolve(), 'B': args.after.resolve()}
    schedule = [('aa-start-0', 'A'), ('aa-start-1', 'A')]
    for pair in range(5):
        for arm in ('AB' if pair % 2 == 0 else 'BA'):
            schedule.append((f'pair-{pair}-{arm}', arm))
    schedule += [('aa-end-0', 'A'), ('aa-end-1', 'A')]
    manifest = {'binaries': {k: {'path': str(p), 'sha256': digest(p)}
                             for k, p in binaries.items()},
                'schedule': schedule, 'duration_simulation_seconds': 20,
                'resolution': [1920, 1080], 'target_p99_ms': 8.33,
                'isolation_requested': False, 'runs': []}
    for name, arm in schedule:
        env = {k: v for k, v in os.environ.items()
               if not k.startswith(('VOXY_STREET_', 'VOXY_FX_'))}
        env.update(VOXY_STREET_MATERIALS=str(args.materials.resolve()),
                   VOXY_STREET_GPU_TIMING='1',
                   VOXY_STREET_BENCHMARK_REPORT=str((args.output / f'{name}.json').resolve()))
        subprocess.run(['ps', '-Ao', 'pid,etime,%cpu,comm'],
                       stdout=open(args.output / f'{name}-processes.txt', 'w'), check=True)
        command = [str(binaries[arm]), '--offscreen-benchmark', '--benchmark', '20']
        start = time.time()
        with open(args.output / f'{name}.log', 'w') as log:
            result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT)
        manifest['runs'].append({'name': name, 'arm': arm, 'start': start,
                                 'end': time.time(), 'exit_code': result.returncode})
        (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        print(name, result.returncode, flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
        if digest(binaries[arm]) != manifest['binaries'][arm]['sha256']:
            raise SystemExit('Frozen binary changed during campaign')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""ENG-00 source inventory and opt-in CPU-only checks. Never qualifies product gates."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / 'docs/engine-audit-2026-10-27/inventory.json'
GATES = {'core', 'runtime', 'editor', 'persistence', 'standalone', 'platform'}
STAGES = {f'ENG-{i:02}' for i in range(1, 14)}


def validate(data, root=ROOT):
    if data.get('schema_version') != 1 or not data.get('domains'):
        raise ValueError('unsupported or empty inventory')
    seen = set()
    for row in data['domains']:
        if not row.get('id') or row['id'] in seen:
            raise ValueError('missing or duplicate domain ID')
        seen.add(row['id'])
        if row.get('stage') not in STAGES or row.get('status') != 'partial':
            raise ValueError('source inventory cannot qualify implementation stages')
        for key in ('owner', 'acceptance', 'gap'):
            if not row.get(key):
                raise ValueError(f'missing {key}')
        if set(row.get('gates', {})) != GATES:
            raise ValueError('incomplete product gates')
        if any(v != 'not_requalified' for v in row['gates'].values()):
            raise ValueError('source pointers cannot qualify runtime/product gates')
        if set(row.get('evidence', {})) != {'api', 'consumer', 'test'}:
            raise ValueError('incomplete source evidence')
        for value in row['evidence'].values():
            path = (root / value).resolve()
            if Path(value).is_absolute() or not path.is_relative_to(root.resolve()) or not path.is_file():
                raise ValueError(f'invalid source evidence: {value}')
    return data


def fingerprint():
    paths = subprocess.check_output(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT
    ).decode().split('\0')
    files = {}
    for name in sorted(set(paths)):
        path = ROOT / name
        if (name.startswith('crates/') and path.suffix in {'.rs', '.toml', '.wgsl'}) or name in {'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'}:
            if path.is_file():
                files[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    digest = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    return {'sha256': digest, 'files': files}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-checks', action='store_true', help='run only built-in CPU/no-device checks')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    data = validate(json.loads(INVENTORY.read_text()))
    if not args.run_checks:
        print(f"Inventory valid: {len(data['domains'])} domains; product gates remain unqualified.")
        return 0
    output = args.output or Path(tempfile.mkdtemp(prefix='voxy-eng00-report-'))
    output.mkdir(parents=True, exist_ok=True)
    if (output / 'report.json').exists():
        raise ValueError('report already exists; choose a fresh output directory')
    before = fingerprint()
    commands = [
        [sys.executable, 'tools/check_engine_boundaries.py'],
        [sys.executable, 'tools/test_engine_boundaries.py'],
        ['cargo', 'test', '--locked', '--offline', '-p', 'voxy_core', '-p', 'voxy_time', '-p', 'voxy_input', '-p', 'voxy_navigation', '-p', 'voxy_runtime', '--lib'],
        ['cargo', 'clippy', '--locked', '--offline', '-p', 'voxy_time', '-p', 'voxy_input', '-p', 'voxy_navigation', '--lib', '--', '-D', 'warnings'],
    ]
    env = dict(os.environ, CARGO_TARGET_DIR='/tmp/voxy-eng00-20261027-target', CARGO_BUILD_JOBS='2')
    checks = []
    for i, command in enumerate(commands):
        logfile = output / f'check-{i + 1}.log'
        print('Running:', ' '.join(command), flush=True)
        with logfile.open('x') as stream:
            try:
                result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=180)
                code = result.returncode
            except subprocess.TimeoutExpired:
                code = 124
        checks.append({'command': command, 'exit_code': code, 'log': logfile.name})
    after = fingerprint()
    report = dict(
        schema_version=1, utc=datetime.now(timezone.utc).isoformat(),
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        dirty_status=subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True),
        inventory_sha256=hashlib.sha256(INVENTORY.read_bytes()).hexdigest(),
        scope='CPU-only focused checks; no editor/window/GPU/audio-device tests',
        source_before=before, source_after=after, source_stable=before == after,
        checks=checks, product_qualification=False,
    )
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Report:', output / 'report.json')
    return int(before != after or any(c['exit_code'] for c in checks))


if __name__ == '__main__':
    raise SystemExit(main())

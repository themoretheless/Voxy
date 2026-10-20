#!/usr/bin/env python3
"""Audit actual native pipeline bounds; never infer occupancy or FPS from them."""
import argparse
import json
from pathlib import Path
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('log', type=Path)
parser.add_argument('--requested', type=int, required=True, choices=[0, 64, 128, 256, 512, 768, 1024])
parser.add_argument('--require-world-consumer', action='store_true')
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
pattern = re.compile(r'^WGPU_METAL_COMPUTE_AUDIT entry=(\S+) local=(\d+) requested=(\d+) applied=(\d+) native_max=(\d+) execution_width=(\d+) static_shared=(\d+)$')
rows = []
for line in args.log.read_text(errors='replace').splitlines():
    if not line.startswith('WGPU_METAL_COMPUTE_AUDIT '):
        continue
    match = pattern.fullmatch(line)
    assert match, f'malformed native audit row: {line}'
    row = dict(zip(['entry', 'local', 'requested', 'applied', 'native_max', 'execution_width', 'static_shared'], match.groups()))
    row.update({key: int(value) for key, value in row.items() if key != 'entry'})
    assert row['requested'] == args.requested
    assert row['native_max'] >= row['local'] > 0
    assert row['execution_width'] > 0
    expected = args.requested if args.requested and row['local'] <= args.requested else 0
    assert row['applied'] == expected
    if expected:
        assert row['native_max'] <= expected
    rows.append(row)
assert rows, 'no native pipeline observations'
consumers = [row for row in rows if row['entry'] == 'cs_world_consumer_cache']
if args.require_world_consumer:
    assert consumers, 'actual world-consumer pipeline was not observed'
    assert all(row['local'] == 64 for row in consumers)
result = {'requested': args.requested, 'native_pipelines_verified': len(rows),
          'world_consumers_verified': len(consumers), 'declared_workgroup_sizes_recorded': True,
          'native_limits_admit_every_recorded_dispatch': True, 'records': rows,
          'hardware_occupancy_measured': False, 'source_precision_verified': False,
          'fps_qualified': False}
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({key: value for key, value in result.items() if key != 'records'}))

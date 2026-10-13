#!/usr/bin/env python3
"""Audit a completed VQC island capture without inferring native/GPU correspondence."""
import argparse
import json
from pathlib import Path
import struct

from audit_hair_island_rows import audit


def inspect(directory, expected_frame=None, expected_rod=None):
    operators = sorted(directory.glob('*.vqc'))
    if not operators:
        raise ValueError('no VQC operators; capture may not have reached the selected frame')
    records, errors = [], []
    for path in operators:
        try:
            result = audit(path)
            metadata = json.loads(path.with_suffix('.metadata.json').read_text())
            backend = metadata.get('backend')
            if backend not in ('native', 'external'):
                raise ValueError('unknown backend')
            index = metadata.get('observation_index')
            if type(index) is not int or index < 0:
                raise ValueError('invalid observation index')
            if expected_frame is not None and metadata['frame'] != expected_frame:
                raise ValueError('unexpected observation frame')
            if expected_rod is not None and expected_rod not in metadata['rod_ids']:
                raise ValueError('selected rod missing from island')
            records.append({'path': str(path), 'backend': backend, 'index': index,
                            'frame': metadata['frame'], 'rod_ids': metadata['rod_ids'],
                            'operator': result['operator'],
                            'checked_load_scalars': result['checked_load_scalars']})
        except (OSError, ValueError, KeyError, TypeError, struct.error) as error:
            errors.append({'path': str(path), 'error': str(error)})
    seen = set()
    for item in records:
        key = (item['backend'], item['index'])
        if key in seen:
            errors.append({'path': item['path'], 'error': 'duplicate backend observation index'})
        seen.add(key)
    orphan_sidecars = [str(p) for p in sorted(directory.glob('*.metadata.json'))
                       if not p.with_name(p.name.removesuffix('.metadata.json') + '.vqc').exists()]
    return {'schema': 'voxy-island-directory-audit-v1', 'directory': str(directory),
            'operator_count': len(operators), 'audited_count': len(records),
            'all_original_loads_verified': not errors and not orphan_sidecars,
            'orphan_sidecars': orphan_sidecars, 'errors': errors, 'records': records,
            'limits': 'Run on a completed capture. Every operator is audited locally against its own v2 sidecar. '
                      'Observation indices do not align backends or identify projection phases. '
                      'This does not prove solver parity, trajectory admission or FPS.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--expected-frame', type=int)
    parser.add_argument('--expected-rod', type=int)
    args = parser.parse_args()
    try:
        result = inspect(args.directory, args.expected_frame, args.expected_rod)
        print(json.dumps(result, indent=2, allow_nan=False))
        if not result['all_original_loads_verified']:
            parser.exit(1)
    except (OSError, ValueError) as error:
        parser.exit(2, f'error: {error}\n')


if __name__ == '__main__':
    main()

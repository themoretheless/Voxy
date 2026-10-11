#!/usr/bin/env python3
"""Check v2 original contact rows against every raw VQC1 load byte."""
import argparse
import json
import math
from pathlib import Path
import struct

from compare_hair_operator_inputs import read


def integer(value, name):
    if type(value) is not int or not 0 <= value < 2**64:
        raise ValueError(f'invalid {name}')
    return value


def scalar(bits):
    value = struct.unpack('<d', struct.pack('<Q', integer(bits, 'f64 bits')))[0]
    if not math.isfinite(value):
        raise ValueError('nonfinite original row scalar')
    return value


def audit(path, metadata_path=None):
    header, sections = read(path)
    sidecar = metadata_path or path.with_suffix('.metadata.json')
    metadata = json.loads(sidecar.read_text())
    if metadata.get('schema') != 'voxy-observed-island-v2':
        raise ValueError('requires v2 original row metadata; v1 does not establish row ownership')
    frame = metadata.get('frame')
    if frame is not None and integer(frame, 'observation frame') == 0:
        raise ValueError('observation frame must be one-based')
    rods, rows = metadata['rod_ids'], metadata['original_rows']
    if len(rods) != len(header['layouts']) or len(rows) != header['rows']:
        raise ValueError('metadata/operator dimensions differ')
    for rod in rods:
        integer(rod, 'rod index')
    if len(set(rods)) != len(rods):
        raise ValueError('duplicate rod owner')
    if metadata['tolerance'] != header['tolerance']:
        raise ValueError('metadata/operator tolerance differs')
    indices = []
    shapes = dict(zip(rods, (layout[0] for layout in header['layouts'])))
    for row in rows:
        indices.append(integer(row['global_row_index'], 'global row index'))
        scalar(row['bound_bits'])
        if len(row['entries']) != 4:
            raise ValueError('original constraint must have four entries')
        for entry in row['entries']:
            rod = integer(entry['rod'], 'entry rod index')
            point = integer(entry['point'], 'entry point index')
            gradient = entry['gradient_bits']
            if len(gradient) != 3:
                raise ValueError('invalid original gradient shape')
            for bits in gradient:
                scalar(bits)
            mobility = scalar(entry['mobility_bits'])
            if mobility < 0:
                raise ValueError('negative original mobility')
            # Zero padding may name a rod outside this island. Any actual
            # non-root entry must have its owner and complete point block.
            if point > 0:
                if rod not in shapes or point * 6 + 2 >= shapes[rod]:
                    raise ValueError('original row load has no valid rod/point owner')
    if len(set(indices)) != len(indices):
        raise ValueError('duplicate original global row index')
    checked = 0
    for slot, (rod, layout) in enumerate(zip(rods, header['layouts'])):
        width = layout[0]
        loads = [0.] * (len(rows) * width)
        for j, row in enumerate(rows):
            for entry in row['entries']:
                if entry['rod'] == rod and entry['point'] > 0:
                    point = entry['point']
                    if point * 6 + 2 >= width:
                        raise ValueError('original point is outside its rod system')
                    for axis, bits in enumerate(entry['gradient_bits']):
                        loads[j * width + point * 6 + axis] += scalar(bits)
        actual = sections[f'system_{slot}_loads'][0]
        if struct.pack(f'<{len(loads)}d', *loads) != actual:
            raise ValueError(f'original row/load byte mismatch for rod {rod}')
        checked += len(loads)
    return {'schema': 'voxy-original-row-load-audit-v1', 'operator': header,
            'metadata': str(sidecar), 'observation_frame': frame,
            'checked_load_scalars': checked,
            'exact_original_load_reconstruction': True,
            'limits': 'Snapshot-local original row ordering and raw non-root loads only. '
                      'Fixed-root gradients, mobilities and original bounds are not '
                      'independently encoded in VQC1; only their metadata shape/finiteness is checked. '
                      'No persistent contact identity, response quantity, solver accuracy, '
                      'whole trajectory or FPS claim.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operator', type=Path)
    parser.add_argument('--metadata', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(audit(args.operator, args.metadata), indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, struct.error) as error:
        parser.exit(2, f'error: {error}\n')


if __name__ == '__main__':
    main()

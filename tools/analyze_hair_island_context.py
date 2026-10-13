#!/usr/bin/env python3
"""Audit failed-island row ownership and exact endpoint-Jacobian matches.

Matches identify candidate frozen contacts, not triangle features or the
historical iteration which created a plane. Clock-scaled rows can remain
unmatched; an unmatched row must not be discarded.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from audit_hair_qr_input import audit


def scalar(bits):
    return struct.unpack('<d', struct.pack('<Q', bits))[0]


def vector(bits):
    return list(map(scalar, bits))


def analyze(path):
    path = Path(path)
    # Path.with_suffix would retain .metadata, so use the paired basename.
    capture = path.with_name(path.name.removesuffix('.metadata.json') + '.vqc')
    metadata = audit(capture)
    raw = path.read_bytes()
    context = json.loads(raw)
    if context['schema'] != 'voxy-failed-island-context-v1':
        raise ValueError('unsupported context schema')
    rows = context['original_rows']
    ids = context['rod_ids']
    if len(rows) != metadata['rows'] or len(ids) != metadata['systems'] or len(set(ids)) != len(ids):
        raise ValueError('context and capture dimensions disagree')
    data = capture.read_bytes()
    cursor = 28 + 16 * len(rows) + 8 * len(rows) * metadata['coordinates']
    for rod_id in ids:
        n, band, lo, hi = struct.unpack_from('<IIII', data, cursor)
        cursor += 16 + 8 * n * (band + 1)
        for row in rows:
            actual = struct.unpack_from('<' + 'd' * n, data, cursor)
            cursor += 8 * n
            expected = [0.] * n
            for entry in row['entries']:
                if entry['rod'] != rod_id:
                    continue
                for axis, value in enumerate(vector(entry['gradient_bits'])):
                    coordinate = entry['point'] * 6 + axis
                    if coordinate >= n:
                        raise ValueError('entry outside captured rod')
                    expected[coordinate] += value
            if list(actual) != expected:
                raise ValueError('context row does not match original physical load')
    frozen = {rod['rod']: rod for rod in context['frozen_rods']}
    results = []
    for local, row in enumerate(rows):
        actual = {(e['rod'], e['point']): vector(e['gradient_bits'])
                  for e in row['entries'] if any(vector(e['gradient_bits']))}
        matches = []
        for rod_id, rod in frozen.items():
            for contact_index, contact in enumerate(rod['contacts']):
                normal = vector(contact['normal_bits'])
                metric = scalar(contact['metric_scale_bits'])
                fraction = scalar(contact['fraction_bits'])
                expected = {}
                for end, weight in enumerate((1. - fraction, fraction)):
                    point = contact['segment'] + end
                    gradient = [(v * metric) * weight for v in normal]
                    if point and any(gradient):
                        expected[(rod_id, point)] = gradient
                if actual == expected:
                    matches.append(dict(rod=rod_id, contact_index=contact_index,
                                        source=contact['source'],
                                        trajectory_time_bits=contact['trajectory_time_bits']))
        results.append(dict(local_row=local, global_row=row['global_row'],
                            exact_unscaled_frozen_contact_jacobian_matches=matches))
    return dict(capture=metadata, context_sha256=hashlib.sha256(raw).hexdigest(),
                original_loads_match=True, rows=results,
                scope='exact original-load audit and unscaled Jacobian matches only; no feature provenance or geometric validity proof')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('metadata_json')
    args = parser.parse_args()
    print(json.dumps(analyze(args.metadata_json), indent=2))

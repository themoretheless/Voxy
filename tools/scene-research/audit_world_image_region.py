#!/usr/bin/env python3
"""Independent rational source geometry checks for live image-consumer fixtures."""
import argparse
import json
import struct
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import word
from exact_closed_world_triangle import classify
from make_world_exact_parameter_gold import exact_parameters, pair

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('root', type=Path)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
reports = []
for prefix in ['exact-image-hit', 'optical-image-hit']:
    fixture = json.loads((args.root / f'{prefix}-fixture.json').read_text())
    rays = list(struct.iter_unpack('<20I', (args.root / f'{prefix}-input.bin').read_bytes()))
    hits = list(struct.iter_unpack('<20I', (args.root / f'{prefix}-output.bin').read_bytes()))
    assert len(rays) == len(hits) == 16
    triangles = []
    for slot, mesh in enumerate(fixture['source_geometry']):
        matrix = [[word(v) for v in column] for column in mesh['world']]
        points = []
        for raw in mesh['vertices']:
            local = list(map(word, raw)) + [Q(1)]
            points.append([sum((matrix[c][axis] * local[c] for c in range(4)), Q(0)) for axis in range(3)])
        for offset in range(0, len(mesh['indices']), 3):
            triangles.append((slot, offset // 3, mesh['material'], [points[i] for i in mesh['indices'][offset:offset + 3]]))
    found = 0
    for index, (ray, hit) in enumerate(zip(rays, hits)):
        assert list(hit[4:6]) == list(ray[18:20])
        if ray[3] == 0:
            assert hit[0] == 3
            continue
        assert ray[3] == 1
        origin = [word(ray[k]) + word(ray[4 + k]) for k in range(3)]
        direction = [word(ray[8 + k]) + word(ray[12 + k]) for k in range(3)]
        candidates = []
        for slot, primitive, material, points in triangles:
            if classify(points, origin, direction)['exact_hit']:
                parameters = exact_parameters(points, origin, direction)
                candidates.append((parameters[0], slot, primitive, material, parameters))
        if not candidates:
            assert hit[0] == 0
            continue
        t, slot, primitive, material, parameters = min(candidates)
        encoded = [pair(value) for value in parameters]
        assert hit[0] == 1 and (hit[6], hit[7], hit[17]) == (slot, primitive, material)
        assert list(hit[8:14]) == [v for words, _ in encoded for v in words]
        assert hit[15] == sum(int(tail) << k for k, (_, tail) in enumerate(encoded))
        if index == 0:
            assert t == Q(1, 64) and hit[9] == 0 and hit[15] & 1 == 0
        found += 1
    counts = fixture['branch_counts']
    assert sum(counts) == 16
    if fixture['optical']:
        assert counts[4] > 0 and counts[2] == 0
    else:
        assert counts[2] > 0
    reports.append({'fixture': prefix, 'queries': 16, 'exact_source_hits': found,
                    'source_parameters_and_residuals_exact': True, 'branch_counts': counts})
result = {'fixtures': reports, 'full_street_qualified': False, 'fps_qualified': False}
args.output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))

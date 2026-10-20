#!/usr/bin/env python3
"""Classify every unresolved source triangle using exact rational affine geometry."""
import argparse
import json
import struct
from collections import Counter
from fractions import Fraction as Q
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('capture', type=Path)
parser.add_argument('results', type=Path)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
manifest = json.loads((args.capture/'manifest.json').read_text())
raw = args.results.read_bytes()
assert len(raw)%32 == 0
unresolved = [i for i, record in enumerate(struct.iter_unpack('<8I', raw)) if record[0] == 2]
rows = []
base = 0
for mesh in manifest['meshes']:
    triangles = mesh['index_count']//3
    end = base+triangles*mesh['instance_count']
    selected = [i for i in unresolved if base <= i < end]
    if selected:
        vertices = (args.capture/mesh['vertices']['file']).read_bytes()
        indices = (args.capture/mesh['indices']['file']).read_bytes()
        instances = (args.capture/mesh['instances']['file']).read_bytes()
        for index in selected:
            instance, primitive = divmod(index-base, triangles)
            matrix = [Q.from_float(v) for v in struct.unpack_from('<16f', instances, instance*112)]
            ids = struct.unpack_from('<3I', indices, primitive*12)
            local = [struct.unpack_from('<3f', vertices, vertex*48) for vertex in ids]
            points = [tuple(sum((matrix[k*4+axis]*Q.from_float(p[k]) for k in range(3)), matrix[12+axis]) for axis in range(3)) for p in local]
            e1 = tuple(x-y for x, y in zip(points[1], points[0]))
            e2 = tuple(x-y for x, y in zip(points[2], points[0]))
            normal = (e1[1]*e2[2]-e1[2]*e2[1], e1[2]*e2[0]-e1[0]*e2[2], e1[0]*e2[1]-e1[1]*e2[0])
            kind = 'point' if points[0] == points[1] == points[2] else ('segment' if not any(normal) else 'nondegenerate')
            rows.append({'ray': index, 'mesh': mesh['id'], 'instance': instance, 'primitive': primitive, 'exact_kind': kind})
    base = end
assert base == len(raw)//32
assert len(rows) == len(unresolved)
report = {'rays': base, 'unresolved': len(rows), 'exact_geometry': dict(Counter(row['exact_kind'] for row in rows)), 'rows': rows}
with args.output.open('x') as output:
    json.dump(report, output, indent=2)
print(json.dumps({k: v for k, v in report.items() if k != 'rows'}, indent=2))

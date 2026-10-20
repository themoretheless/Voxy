#!/usr/bin/env python3
"""Independent rational enclosure and original primitive coverage audit."""
import argparse
import json
import math
import struct
from fractions import Fraction as Q
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('source', type=Path, help='GPU fixture folder or wide-source CPU dump')
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
sources = []
if a.source.is_file():
    data = json.loads(a.source.read_text())
    words = data['hierarchy']
    for mesh in data['meshes']:
        sources.extend((mesh['positions'], mesh['indices'], matrix)
                       for matrix in mesh['instances'])
else:
    manifest = json.loads((a.source / 'manifest.json').read_text())
    raw = (a.source / 'source-hierarchy.bin').read_bytes()
    words = list(struct.unpack('<' + 'I' * (len(raw) // 4), raw))
    for mesh in manifest['meshes']:
        raw = (a.source / mesh['vertices']['file']).read_bytes()
        positions = [struct.unpack_from('<3I', raw, i * 48)
                     for i in range(mesh['vertex_count'])]
        raw = (a.source / mesh['indices']['file']).read_bytes()
        indices = list(struct.unpack('<' + 'I' * mesh['index_count'], raw))
        raw = (a.source / mesh['instances']['file']).read_bytes()
        for i in range(mesh['instance_count']):
            matrix = struct.unpack_from('<16I', raw, i * 112)
            sources.append((positions, indices, [matrix[k:k+4] for k in range(0, 16, 4)]))

def float_word(v):
    return struct.unpack('<f', struct.pack('<I', v))[0]

def rational(v):
    return Q.from_float(float_word(v))

assert words[0] == len(sources)
offset, count, root = words[2], words[1], words[3]
assert len(words) == offset + count * 8
assert offset >= 4 + len(sources)
nodes = [words[offset+i*8:offset+(i+1)*8] for i in range(count)]
seen, covered, descriptors, points = set(), set(), set(), {}
stack = [(root, 0)] if root != 0xffffffff else []
vertices_checked = 0
while stack:
    node_id, depth = stack.pop()
    assert node_id < count and node_id not in seen and depth < 63
    seen.add(node_id)
    node = nodes[node_id]
    bounds = list(map(float_word, node[:6]))
    assert not any(math.isnan(v) for v in bounds)
    assert all(bounds[k] <= bounds[k+3] for k in range(3))
    left, right = node[6:8]
    if right < 0xfffffffe:
        for child in [left, right]:
            assert child < count
            cb = list(map(float_word, nodes[child][:6]))
            assert all(bounds[k] <= cb[k] and bounds[k+3] >= cb[k+3] for k in range(3))
            stack.append((child, depth + 1))
        continue
    if right == 0xfffffffe:
        assert left not in descriptors
        descriptors.add(left)
        begin = 4 + len(sources) + left * 3
        assert begin + 3 <= offset
        slot, first, amount = words[begin:begin+3]
        assert 0 < amount <= 8
    else:
        slot, first = left, 0
        amount = len(sources[slot][1]) // 3
    assert slot < len(sources)
    positions, indices, matrix = sources[slot]
    assert first + amount <= len(indices) // 3
    for primitive in range(first, first + amount):
        assert (slot, primitive) not in covered
        covered.add((slot, primitive))
        for vertex in indices[primitive*3:primitive*3+3]:
            key = (slot, vertex)
            if key not in points:
                points[key] = [sum((rational(matrix[k][axis]) * rational(positions[vertex][k])
                                    for k in range(3)), rational(matrix[3][axis]))
                               for axis in range(3)]
            for axis, value in enumerate(points[key]):
                lo, hi = bounds[axis], bounds[axis+3]
                assert (lo == -math.inf or Q.from_float(lo) <= value)
                assert (hi == math.inf or Q.from_float(hi) >= value)
            vertices_checked += 1
assert len(seen) == count
assert covered == {(slot, primitive) for slot, (_, indices, _) in enumerate(sources)
                   for primitive in range(len(indices) // 3)}
assert len(descriptors) * 3 == offset - 4 - len(sources)
report = {'instances': len(sources), 'nodes': count, 'group_leaves': len(descriptors),
          'original_primitives_once': len(covered), 'source_vertex_enclosures_exact': vertices_checked,
          'independent_rational_affine_arithmetic': True, 'replacement_geometry': False,
          'fps_qualified': False}
a.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))

#!/usr/bin/env python3
"""Small analytic scene plus exact dyadic edge/endpoint/high-low diagnostic rays."""
import json
import struct
import sys
from pathlib import Path
root = Path(sys.argv[1])
root.mkdir()


def blob(name, data):
    (root/name).write_bytes(data)
    return {'file': name, 'bytes': len(data)}


meshes = []
for mesh in range(2):
    vertices = b''.join(struct.pack('<12f', x, y, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0) for x, y in [(-1, -1), (1, -1), (0, 1)])
    instances = b''
    for instance in range(2):
        world = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, mesh*5, instance*5, 0, 1]
        normal = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0]
        instances += struct.pack('<28f', *(world+normal))
    meshes.append({'id': mesh, 'material': mesh, 'vertex_count': 3, 'index_count': 3, 'instance_count': 2,
                   'vertices': blob(f'v{mesh}.bin', vertices), 'indices': blob(f'i{mesh}.bin', struct.pack('<3I', 0, 1, 2)),
                   'instances': blob(f't{mesh}.bin', instances)})
(root/'manifest.json').write_text(json.dumps({'schema': 'voxy-full-world-ray-geometry-v1', 'meshes': meshes}, indent=2))


def word(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def case(name, origin, state, low_z=0., low_direction=0.):
    ray = [0]*20
    ray[:3] = [word(x) for x in origin]
    ray[6] = word(low_z)
    ray[10] = word(-1. if low_direction == 0. else 0.)
    ray[14] = word(low_direction)
    return {'name': name, 'ray_words': ray, 'expected_state': state, 't_low_must_be_nonzero': low_z != 0.}


cases = [case('interior', [0, 0, .25], 1), case('edge', [0, -1, .25], 2),
         case('vertex', [-1, -1, .25], 2), case('near-endpoint', [0, 0, 0], 2),
         case('far-endpoint', [0, 0, 1], 2), case('outside-triangle-inside-box', [.9, .9, .25], 0),
         case('origin-low-word', [0, 0, .25], 1, low_z=2**-30),
         case('zero-high-subnormal-direction', [0, 0, 0], 2, low_direction=-2**-130)]
(root/'cases.json').write_text(json.dumps(cases, indent=2))
print(root)

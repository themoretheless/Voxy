#!/usr/bin/env python3
"""Original-bit exact GPU decision fixtures: ordinary, collapsed, coplanar, extreme."""
import json
import runpy
import struct
import sys
from pathlib import Path
root = Path(sys.argv[1])
runpy.run_path(str(Path(__file__).with_name('make_world_narrow_boundary_fixture.py')), run_name='__main__')
manifest = json.loads((root/'manifest.json').read_text())
meshes = manifest['meshes']
identity = [1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def blob(name, data):
    (root/name).write_bytes(data)
    return {'file': name, 'bytes': len(data)}


def add(points, world):
    mesh = len(meshes)
    vertices = b''.join(struct.pack('<12f', *p, 0,0,1, 1,0,0,1, 0,0) for p in points)
    instances = struct.pack('<28f', *(world+[1,0,0,0,0,1,0,0,0,0,1,0]))
    meshes.append({'id':mesh,'material':mesh,'vertex_count':3,'index_count':3,'instance_count':1,
                   'vertices':blob(f'v{mesh}.bin',vertices),'indices':blob(f'i{mesh}.bin',struct.pack('<3I',0,1,2)),
                   'instances':blob(f't{mesh}.bin',instances)})


world = identity.copy();world[12] = 10
add([(0,0,0)]*3, world)
world = identity.copy();world[12] = 15
add([(-1,0,0),(1,0,0),(0,0,0)], world)
add([(-2**60,-2**60,0),(2**60,-2**60,0),(0,2**60,0)], identity)
add([(-2**-140,-2**-140,0),(2**-140,-2**-140,0),(0,2**-140,0)], identity)
add([(-1,-1,0),(0,1,0),(1,-1,0)], [-2,0,1,0, 1,3,0,0, 0,0,1,0, 20,0,0,1])
add([(-1,-1,-1),(1,-1,1),(0,1,0)], [1,0,0,0, 0,1,0,0, 0,0,0,0, 30,0,0,1])
add([(-2**-149,-2**-149,0),(2**-149,-2**-149,0),(0,2**-149,0)], [2**-149,0,0,0, 0,2**-149,0,0, 0,0,2**-149,0, 0,0,0,1])
dense=struct.unpack('<f',struct.pack('<I',0x5d7fffff))[0]
scale=struct.unpack('<f',struct.pack('<I',0x3fffffff))[0]
add([(-dense,-dense,0),(dense,-dense,0),(0,dense,0)], [scale,0,0,0, 0,scale,0,0, 0,0,1,0, 0,0,0,1])
(root/'manifest.json').write_text(json.dumps(manifest, indent=2))
source = []
for mesh in meshes:
    vertices = (root/mesh['vertices']['file']).read_bytes()
    indices = (root/mesh['indices']['file']).read_bytes()
    instances = (root/mesh['instances']['file']).read_bytes()
    ids = struct.unpack_from('<3I', indices)
    local = [struct.unpack_from('<3f', vertices, vertex*48) for vertex in ids]
    for instance in range(mesh['instance_count']):
        world = struct.unpack_from('<16f', instances, instance*112)
        source.append((local,[list(world[k*4:k*4+4]) for k in range(4)]))
cases = []


def word(v):
    return struct.unpack('<I',struct.pack('<f',v))[0]


def emit(name, slot, origin, direction, low=(0,0,0), dlow=(0,0,0)):
    ray = [0]*20
    ray[:3] = map(word,origin);ray[4:7] = map(word,low)
    ray[8:11] = map(word,direction);ray[12:15] = map(word,dlow)
    ray[16] = slot
    local, world = source[slot]
    cases.append({'name':name,'global_ray':len(cases),'ray_words':ray,'world':world,'local_triangle':local})


emit('interior',0,(0,0,.25),(0,0,-1))
emit('edge',0,(0,-1,.25),(0,0,-1))
emit('vertex',0,(-1,-1,.25),(0,0,-1))
emit('near',0,(0,0,0),(0,0,-1))
emit('far',0,(0,0,1),(0,0,-1))
emit('outside',0,(.9,.9,.25),(0,0,-1))
emit('origin-low',0,(0,0,.25),(0,0,-1),low=(0,0,2**-30))
emit('direction-subnormal-near',0,(0,0,0),(0,0,0),dlow=(0,0,-2**-149))
emit('direction-subnormal-too-short',0,(0,0,.25),(0,0,0),dlow=(0,0,-2**-149))
emit('coplanar-enter',0,(-2,0,0),(2,0,0))
emit('coplanar-miss',0,(-2,2,0),(2,0,0))
emit('parallel-off-plane',0,(0,0,1),(1,0,0))
emit('constant-inside',0,(0,0,0),(0,0,0))
emit('constant-outside',0,(3,3,0),(0,0,0))
maximum=struct.unpack('<f',struct.pack('<I',0x7f7fffff))[0]
emit('direction-full-cancellation',0,(0,0,0),(0,0,maximum),dlow=(0,0,-maximum))
emit('direction-maximum',0,(0,0,.25),(0,0,-maximum))
emit('translated-hit',2,(5,0,.25),(0,0,-1))
emit('translated-away',2,(5,0,.25),(0,0,1))
emit('point-cross',4,(10,0,.25),(0,0,-1))
emit('point-low-miss',4,(10,0,.25),(0,0,-1),low=(2**-149,0,0))
emit('point-constant-hit',4,(10,0,0),(0,0,0))
emit('point-constant-miss',4,(10,1,0),(0,0,0))
emit('segment-cross',5,(15,0,.25),(0,0,-1))
emit('segment-low-miss',5,(15,0,.25),(0,0,-1),low=(0,2**-100,0))
emit('segment-collinear-far',5,(13,0,0),(1,0,0))
emit('segment-negative-direction',5,(17,0,0),(-1,0,0))
emit('segment-parallel-miss',5,(13,1,0),(1,0,0))
emit('segment-constant-hit',5,(15,0,0),(0,0,0))
emit('segment-constant-miss',5,(17,0,0),(0,0,0))
emit('huge-interior',6,(0,0,.25),(0,0,-1))
emit('huge-min-subnormal-inside',6,(2**60,-2**60,.25),(0,0,-1),low=(-2**-149,0,0))
emit('huge-min-subnormal-outside',6,(2**60,-2**60,.25),(0,0,-1),low=(2**-149,0,0))
emit('huge-corner-diagonal-inside',6,(2**60,-2**60,.25),(0,0,-1),low=(-2**-149,2**-149,0))
emit('tiny-source-interior',7,(0,0,.25),(0,0,-1))
emit('tiny-source-outside',7,(2**-139,0,.25),(0,0,-1))
emit('tiny-source-low-inside',7,(0,0,.25),(0,0,-1),low=(2**-149,0,0))
emit('reversed-sheared-hit',8,(23,-1,6),(-12,4,-24))
emit('reversed-sheared-near',8,(20,0,0),(-12,4,-24))
emit('reversed-sheared-far',8,(32,-4,24),(-12,4,-24))
emit('reversed-sheared-coplanar',8,(20,0,0),(0,6,1))
emit('singular-source-hit',9,(30,0,.25),(0,0,-1))
emit('singular-source-edge',9,(30,-1,.25),(0,0,-1))
emit('minimum-source-product-interior',10,(0,0,.25),(0,0,-1))
emit('dense-mantissa-product-interior',11,(0,0,.25),(0,0,-1))
product=dense*scale
high=f32(product);low=f32(product-high)
emit('dense-product-vertex',11,(high,-high,.25),(0,0,-1),low=(low,-low,0))
cases[-1]['global_ray']=(1<<40)+len(cases)
with (root/'exact-cases.jsonl').open('x') as file:
    for case in cases:
        file.write(json.dumps(case)+'\n')
print(f'exact fixtures={len(cases)} source_instances={len(source)}')

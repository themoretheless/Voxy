#!/usr/bin/env python3
"""Separated planes, glass, unknown alpha, ties and closed-contact nearest probes."""
import json, random, struct, sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(exist_ok=False)
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
uvs=[(0x3eaaaaab,1),(0x3f666666,0x80000001),(0x3dcccccd,2)]
vertices=b''.join(struct.pack('<10f2I',*position,0.,0.,1.,1.,0.,0.,1.,*uv)
                  for position,uv in zip([(-1.,-1.,0.),(1.,-1.,0.),(0.,1.,0.)],uvs))
indices=struct.pack('<3I',0,1,2)
def instance(x,z):
    return struct.pack('<28f',1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,x,0.,z,1.,
                       1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.)
meshes=[]
for mid,(material,transforms) in enumerate([
    (0,[(0.,float(z)) for z in range(130)]+[(x,float(z)) for x in [4.,8.] for z in range(1,130)]+[(12.,0.),(12.,0.)]),
    (1,[(4.,0.)]),(2,[(8.,0.)]),
]):
    mesh=dict(id=mid,material=material,vertex_count=3,index_count=3,instance_count=len(transforms))
    for field,data in [('vertices',vertices),('indices',indices),('instances',b''.join(instance(*t) for t in transforms))]:
        name=f'{mid}-{field}.bin';(root/name).write_bytes(data);mesh[field]=dict(file=name,bytes=len(data))
    meshes.append(mesh)
materials=[]
for mid,(transmission,cutoff,image) in enumerate([(0.,-1.,None),(.8,-1.,None),(0.,.5,0)]):
    values=[1.,1.,1.,1.]+[0.]*16;values[14]=transmission;values[15]=cutoff
    name=f'material-{mid}.bin';(root/name).write_bytes(struct.pack('<20f',*values))
    materials.append(dict(id=mid,parameters=dict(file=name,bytes=80),images=[image,None,None,None],double_sided=True))
cases=[]
for i,(name,origin,direction) in enumerate([
    ('far-first-130',(0.,0.,-10.),(0.,0.,150.)),
    ('reverse-130',(0.,0.,140.),(0.,0.,-150.)),
    ('known-glass-first',(4.,0.,-10.),(0.,0.,150.)),
    ('unknown-mask-first',(8.,0.,-10.),(0.,0.,150.)),
    ('equal-depth-tie',(12.,0.,-10.),(0.,0.,150.)),
    ('closed-start',(0.,0.,0.),(0.,0.,150.)),
    ('closed-edge',(0.,-1.,-10.),(0.,0.,150.)),
    ('closed-vertex',(-1.,-1.,-10.),(0.,0.,150.)),
    ('clear',(2.,0.,-10.),(0.,0.,150.)),
    ('subnormal-uv-residual',(.25,0.,-10.),(0.,0.,150.)),
    ('unknown-mask-behind-known',(8.,0.,140.),(0.,0.,-150.)),
]):
    words=[0]*20;words[:3]=map(bits,origin);words[8:11]=map(bits,direction)
    cases.append(dict(name=name,global_ray=(1<<48)+i,ray_words=words))
# Independent Fraction gold evaluates these exact source words, not a nearest
# guess. Include tiny low components and both traversal directions.
rng=random.Random(0x564f5859)
for i in range(int(sys.argv[2]) if len(sys.argv)>2 else 0):
    words=[0]*20
    words[:3]=map(bits,[rng.uniform(-.45,.45),rng.uniform(-.45,.45),-10. if i%2==0 else 140.])
    words[8:11]=map(bits,[rng.uniform(-.6,.6),rng.uniform(-.6,.6),150. if i%2==0 else -150.])
    words[4]=1 if i%13==0 else bits(2.**-30*(-1 if i%3==0 else 1))
    words[6]=bits(2.**-26)
    words[14]=1 if i%17==0 else bits(2.**-24)
    cases.append(dict(name=f'random-exact-words-{i}',global_ray=(1<<49)+i,ray_words=words))
(root/'manifest.json').write_text(json.dumps(dict(schema='voxy-full-world-ray-geometry-v1',meshes=meshes,materials=materials,image_count=1,images_exported=False),indent=2))
(root/'cases.jsonl').write_text(''.join(json.dumps(c)+'\n' for c in cases))

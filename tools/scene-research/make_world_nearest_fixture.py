#!/usr/bin/env python3
"""Nearest planes share rounded t high words; source slots deliberately reverse depth."""
import json
import struct
import sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(exist_ok=False)
vertices=b''.join(struct.pack('<12f',*p,*([0.]*9)) for p in [(-1.,-1.,1.),(1.,-1.,1.),(0.,1.,1.)])
indices=struct.pack('<3I',0,1,2)
instances=b''
count=int(sys.argv[2]) if len(sys.argv)>2 else 3
assert count>=3
for scale in [2.**-30]+[2.**-31]*(count-1):
    world=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,scale,0.,0.,0.,1.,1.]
    normal=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.]
    instances+=struct.pack('<28f',*world,*normal)
mesh=dict(id=0,material=7,vertex_count=3,index_count=3,instance_count=count)
for key,name,data in [('vertices','v.bin',vertices),('indices','i.bin',indices),('instances','t.bin',instances)]:
    (root/name).write_bytes(data);mesh[key]=dict(file=name,bytes=len(data))
(root/'manifest.json').write_text(json.dumps(dict(schema='voxy-full-world-ray-geometry-v1',meshes=[mesh]),indent=2))
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
cases=[]
for i,(origin,dhi,dlo) in enumerate([(0.,2.,0.),(1.,0.,2.**-29),(2.,-2.,0.)]):
    words=[0]*20;words[2]=bits(origin);words[10]=bits(dhi);words[14]=bits(dlo)
    cases.append(dict(global_ray=(1<<40)+i,ray_words=words))
(root/'cases.jsonl').write_text(''.join(json.dumps(c)+'\n' for c in cases))

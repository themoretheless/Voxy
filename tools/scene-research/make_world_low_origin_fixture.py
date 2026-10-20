#!/usr/bin/env python3
"""Canonical low origin crosses a small local BLAS at a large world translation."""
import json
import struct
import sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(exist_ok=False)
vertices=b''.join(struct.pack('<12f',*p,*([0.]*9)) for p in [(-16.,-1.,4.),(16.,-1.,4.),(0.,1.,4.),(1.75,-1.,1.),(2.25,-1.,1.),(2.,1.,1.)])
indices=struct.pack('<6I',0,1,2,3,4,5)
world=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,1e8,0.,0.,1.]
normal=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.]
instances=struct.pack('<28f',*world,*normal)
mesh=dict(id=0,material=19,vertex_count=6,index_count=6,instance_count=1)
for key,name,data in [('vertices','v.bin',vertices),('indices','i.bin',indices),('instances','t.bin',instances)]:
 (root/name).write_bytes(data);mesh[key]=dict(file=name,bytes=len(data))
(root/'manifest.json').write_text(json.dumps(dict(schema='voxy-full-world-ray-geometry-v1',meshes=[mesh]),indent=2))
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
cases=[]
for i,low in enumerate([2.,0.,-2.]):
 words=[0]*20;words[0]=bits(1e8);words[4]=bits(low);words[10]=bits(2.)
 cases.append(dict(global_ray=i,ray_words=words))
(root/'cases.jsonl').write_text(''.join(json.dumps(c)+'\n' for c in cases))

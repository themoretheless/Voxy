#!/usr/bin/env python3
"""Closed alpha boundaries and layered surfaces, using original archive material ABI."""
import json
import struct
import sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(exist_ok=False)
# Opaque ignores alpha; equal cutoff is included; subnormal comparisons remain exact.
specs=[('opaque-zero',0.,-1.,0.,None),('mask-below',.25,.5,0.,None),('mask-equal',.5,.5,0.,None),('mask-above',.75,.5,0.,None),('blend-zero',0.,-2.,0.,None),('blend-quarter',.25,-2.,0.,None),('transmission',0.,-1.,.9,None),('mask-textured',1.,.5,0.,0),('blend-textured',1.,-2.,0.,0),('mask-subnormal-equal',2.**-149,2.**-149,0.,None),('mask-subnormal-below',0.,2.**-149,0.,None)]
specs += [('blend-packed-map',.25,-2.,0.,0),('blend-packed-zero',0.,-2.,0.,0),('mask-packed-map',1.,.5,0.,0),('blend-packed-threshold',1.,-2.,0.,0),('blend-packed-next',1.,-2.,0.,0)]
workflows={'blend-packed-map':2.,'blend-packed-zero':2.,'mask-packed-map':2.,'blend-packed-threshold':1.5,'blend-packed-next':struct.unpack('<f',struct.pack('<I',0x3fc00001))[0]}
meshes=[];materials=[];cases=[]
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
for group,(name,alpha,cutoff,transmission,image) in enumerate(specs):
 for layer in range(2):
  index=len(meshes);z=1.+layer;x=group*4.
  vertices=b''.join(struct.pack('<12f',*p,*([0.]*9)) for p in [(x-1.,-1.,z),(x+1.,-1.,z),(x,1.,z)])
  indices=struct.pack('<3I',0,1,2)
  world=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
  normal=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.]
  instance=struct.pack('<28f',*world,*normal)
  mesh=dict(id=index,material=index,vertex_count=3,index_count=3,instance_count=1)
  for field,data in [('vertices',vertices),('indices',indices),('instances',instance)]:
   filename=f'{index}-{field}.bin';(root/filename).write_bytes(data);mesh[field]=dict(file=filename,bytes=len(data))
  meshes.append(mesh)
  a,c,t,im=(alpha,cutoff,transmission,image) if layer==0 else (1.,-1.,0.,None)
  parameters=[1.,1.,1.,a]+[1.]*4+[0.,0.,0.,1.]+[0.,.5,t,c]+[1.,workflows.get(name,0.) if layer==0 else 0.,0.,0.]
  filename=f'{index}-material.bin';(root/filename).write_bytes(struct.pack('<20f',*parameters))
  materials.append(dict(id=index,parameters=dict(file=filename,bytes=80),images=[im,None,None,None],double_sided=True))
 words=[0]*20;words[0]=bits(group*4.);words[10]=bits(3.)
 cases.append(dict(global_ray=group,ray_words=words,name=name))
(root/'manifest.json').write_text(json.dumps(dict(schema='voxy-full-world-ray-geometry-v1',meshes=meshes,materials=materials,image_count=1,images_exported=False),indent=2))
(root/'cases.jsonl').write_text(''.join(json.dumps(c)+'\n' for c in cases))

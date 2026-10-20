#!/usr/bin/env python3
"""Retain geometry/identity; exercise original UV low bits, cancellation and extremes."""
import json,shutil,struct,sys
from pathlib import Path
source,root=map(Path,sys.argv[1:]);shutil.copytree(source,root)
m=json.loads((root/'manifest.json').read_text())
uv=[(0x3eaaaaab,1),(0x3f666666,0x80000001),(0x3dcccccd,2),(0x7f7fffff,0xff7fffff),(0xff7fffff,0x7f7fffff),(1,0x80000001),(0x80000000,0),(0x3f800001,0x3f7fffff),(0xbf800001,0xbf7fffff)]
for mesh in m['meshes']:
 path=root/mesh['vertices']['file'];data=bytearray(path.read_bytes())
 for i in range(mesh['vertex_count']):struct.pack_into('<2I',data,i*48+40,*uv[(mesh['id']*3+i)%len(uv)])
 path.write_bytes(data)
print(json.dumps({'meshes':len(m['meshes']),'positions_indices_instances_unchanged':True,'uv_cases':len(uv)}))

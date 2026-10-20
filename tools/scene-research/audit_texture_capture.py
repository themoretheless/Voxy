#!/usr/bin/env python3
"""Inspect every exported source mip and every float channel; never sample/downconvert."""
import argparse
import json
import subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('capture',type=Path);p.add_argument('--hasher',type=Path,required=True);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
manifest=json.loads((args.capture/'manifest.json').read_text());assert manifest['images_exported'] is True
images=manifest['images'];assert len(images)==manifest['image_count'];seen=set();rows=[];total=0
for image in images:
 identity=image['id'];assert identity not in seen;seen.add(identity);assert identity==len(rows)
 width,height=image['width'],image['height'];assert width>0 and height>0
 mips=image['mips'];assert len(mips)==image['source_mip_count'] and 0<len(mips)<=max(width,height).bit_length()
 format=image['format'];levels=[]
 if format in ['rgba8unorm','rgba32float']:assert len(mips)==1
 for level,mip in enumerate(mips):
  filename=mip['file'];assert Path(filename).name==filename and filename not in ['.','..']
  w,h=max(1,width>>level),max(1,height>>level)
  if format=='rgba32float':expected=w*h*16
  elif format=='rgba8unorm':expected=w*h*4
  else:
   assert format in ['bc7-rgba-unorm','bc7-rgba-unorm-srgb'];expected=((w+3)//4)*((h+3)//4)*16
  assert mip['bytes']==expected
  actual=json.loads(subprocess.check_output([str(args.hasher),format,str(args.capture/filename)],text=True))
  assert actual['bytes']==expected and actual['blake3']==mip['blake3']
  total+=expected;levels.append(dict(level=level,width=w,height=h,**actual))
 rows.append(dict(id=identity,format=format,levels=levels))
for material in manifest['materials']:
 for identity in material['images']:
  assert identity is None or identity in seen
result=dict(images=len(rows),source_bytes=total,all_payload_checksums_match=True,all_float_channels_finite=True,all_original_source_mips_present=True,upload_generated_mips_captured=False,ray_alpha_sampling_qualified=False,live_rendering_qualified=False,frame_fps_measured=False,rows=rows)
with args.output.open('x') as f:json.dump(result,f,indent=2)
print(json.dumps({k:v for k,v in result.items() if k!='rows'}))

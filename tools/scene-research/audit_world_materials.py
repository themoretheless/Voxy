#!/usr/bin/env python3
"""Verify material source readback and policy outcomes against independent rational gold."""
import argparse
import json
import struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('scene',type=Path);p.add_argument('gold',type=Path);p.add_argument('results',type=Path);p.add_argument('--upload',type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args()
manifest=json.loads((args.scene/'manifest.json').read_text())
source_bytes=0
if args.upload:
 data=args.upload.read_bytes();words=struct.unpack('<'+'I'*(len(data)//4),data);materials=manifest['materials']
 assert words[:4]==(len(materials),1,24,0) and len(words)==4+24*len(materials)
 for i,material in enumerate(materials):
  offset=(4+24*i)*4
  original=(args.scene/material['parameters']['file']).read_bytes()
  assert len(original)==80 and data[offset:offset+80]==original
  expected_image=0xffffffff if material['images'][0] is None else material['images'][0]
  assert words[4+24*i+20:4+24*i+24]==(expected_image,int(material['double_sided']),0,0)
  source_bytes+=80
records=list(struct.iter_unpack('<20I',args.results.read_bytes()));gold={r['global_ray']:r for r in json.loads(args.gold.read_text())['rows']}
seen=set();states=[0,0,0];kinds=[0,0,0];details=[]
for record in records:
 identity=record[4]|record[5]<<32;assert identity not in seen;seen.add(identity);expected=gold[identity]
 state=expected['expected_state'];assert record[0]==state
 assert record[2]==2048 and 0<record[3]<=512
 assert record[6:8]==(expected['winner_slot'],expected['winner_primitive']) and record[17]==expected['winner_material']
 assert record[18]&2 and (record[18]>>2)&3==expected['material_kind']
 if state==2:
  assert record[1]==256 and record[18]&16 and record[6:8]==(0xffffffff,0xffffffff) and record[17]==0xffffffff
 else:
  assert record[1]==0 and not record[18]&16
 if state==1:
  assert record[14]==1 and list(record[8:14])==expected['parameter_words'] and record[15]==expected['residual_mask']
 else:
  assert not any(record[8:16])
 states[state]+=1;kinds[expected['material_kind']]+=int(state==1)
 details.append(dict(global_ray=identity,state=state,material=record[17],kind=expected['material_kind']))
assert seen==set(gold)
report=dict(cases=len(records),states=states,accepted_opaque_masked_blend=kinds,source_parameter_bytes_compared=source_bytes,all_ids_parameters_flags_match=True,unsupported_mapped_alpha_explicit=True,face_culling_qualified=False,texture_alpha_sampling_qualified=False,layer_transport_qualified=False,live_rendering_qualified=False,minimum_120_fps_qualified=False,rows=details)
with args.output.open('x') as f:json.dump(report,f,indent=2)
print(json.dumps({k:v for k,v in report.items() if k!='rows'}))

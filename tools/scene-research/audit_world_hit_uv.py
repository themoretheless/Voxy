#!/usr/bin/env python3
"""GPU UV output must use exact source attributes and unrounded oracle weights."""
import argparse,json,struct
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import word
from make_world_exact_parameter_gold import pair
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('gold',type=Path);p.add_argument('results',type=Path);p.add_argument('--stats',action='store_true');p.add_argument('--output',type=Path,required=True);a=p.parse_args()
m=json.loads((a.root/'manifest.json').read_text());sources=[]
for mesh in m['meshes']:
 vertices=(a.root/mesh['vertices']['file']).read_bytes();indices=list(struct.unpack('<'+'I'*mesh['index_count'],(a.root/mesh['indices']['file']).read_bytes()))
 for _ in range(mesh['instance_count']):sources.append((mesh,vertices,indices))
gold={r['global_ray']:r for r in json.loads(a.gold.read_text())['rows']};width=32 if a.stats else 28;uv_base=24 if a.stats else 20;raw=a.results.read_bytes();assert len(raw)==len(gold)*width*4
hits=misses=unknown=nonzero_lows=residuals=tiny=rounded_weight_failures=0;max_slots=0
for w in struct.iter_unpack('<'+'I'*width,raw):
 identifier=w[4]|w[5]<<32;g=gold[identifier];assert w[0]==g.get('expected_state',int(g['exact_hit']));max_slots=max(max_slots,w[3]);assert w[2]==2048 and 0<w[3]<=512
 if w[0]!=1:
  assert all(v==0 for v in w[uv_base:uv_base+8]);misses+=int(w[0]==0);unknown+=int(w[0]==2);continue
 hits+=1;assert [w[6],w[7],w[17]]==[g['winner_slot'],g['winner_primitive'],g['winner_material']];assert list(w[8:14])==g['parameter_words'];assert w[15]==g['residual_mask']
 mesh,vertices,indices=sources[w[6]];triangle=indices[w[7]*3:w[7]*3+3];bary=list(map(Q,g['exact_parameters']));weights=[1-bary[1]-bary[2],bary[1],bary[2]]
 expected=[];mask=0;rounded_expected=[];rounded_mask=0
 rounded_u=word(w[10])+word(w[11]);rounded_v=word(w[12])+word(w[13]);rounded_weights=[1-rounded_u-rounded_v,rounded_u,rounded_v]
 for axis in range(2):
  value=sum((word(struct.unpack_from('<I',vertices,v*48+40+axis*4)[0])*weight for v,weight in zip(triangle,weights)),Q(0));bits,remaining=pair(value);expected+=bits;mask|=int(remaining)<<axis
  rounded_value=sum((word(struct.unpack_from('<I',vertices,v*48+40+axis*4)[0])*weight for v,weight in zip(triangle,rounded_weights)),Q(0));rounded_bits,rounded_remaining=pair(rounded_value);rounded_expected+=rounded_bits;rounded_mask|=int(rounded_remaining)<<axis
 assert list(w[uv_base:uv_base+4])==expected,(identifier,w[uv_base:uv_base+8],expected)
 assert list(w[uv_base+4:uv_base+8])==[3,mask,0,0]
 for low in expected[1::2]:nonzero_lows+=int(low&0x7fffffff!=0);tiny+=int(0<low&0x7fffffff<0x00800000)
 residuals+=int(mask!=0);rounded_weight_failures+=int(rounded_expected!=expected or rounded_mask!=mask)
report={'records':len(gold),'hits_with_exact_uv':hits,'misses':misses,'unresolved':unknown,'UV_parameter_words_exact':True,'residual_masks_exact':True,'nonzero_uv_low_words':nonzero_lows,'subnormal_uv_low_words':tiny,'UV_residual_cases':residuals,'rounded_weight_negative_control_failures':rounded_weight_failures,'max_arena_slots':max_slots,'UV_uses_unrounded_weights':True,'fps_qualified':False};a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

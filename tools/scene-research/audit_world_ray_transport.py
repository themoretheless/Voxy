#!/usr/bin/env python3
"""Independent exact f32 coefficient * 60 transport and live-ray result audit."""
import argparse,json,struct
from pathlib import Path
from audit_world_triangle_case import word
from make_world_exact_parameter_gold import rounded
from make_world_nearest_gold import build
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--uv',action='store_true');p.add_argument('--visibility',action='store_true');a=p.parse_args();root=a.root
inputs=[w[0] for w in struct.iter_unpack('<I',(root/'scale-input.bin').read_bytes())];results=list(struct.iter_unpack('<4I',(root/'scale-output.bin').read_bytes()));assert len(inputs)==len(results)
valid=unsupported=low_words=subnormal_lows=0
for raw,(hi,lo,admitted,echo) in zip(inputs,results):
    assert raw==echo
    if raw&0x7f800000==0x7f800000:assert admitted==0;unsupported+=1;continue
    value=word(raw)*60
    try:expected=rounded(value)
    except AssertionError:assert admitted==0;unsupported+=1;continue
    if raw&0x7fffffff==0:expected=raw&0x80000000
    assert admitted==1 and hi==expected,(hex(raw),hex(hi),hex(expected),admitted)
    expected_low=rounded(value-word(hi));assert lo==expected_low;assert word(hi)+word(lo)==value
    valid+=1;low_words+=int(lo&0x7fffffff!=0);subnormal_lows+=int(0<lo&0x7fffffff<0x00800000)
index=inputs.index(0x00800001);assert results[index][1]==0x80000004
width=28 if a.uv else 20
protocol=list(struct.iter_unpack('<'+'I'*width,(root/'guard-protocol.bin').read_bytes()))
assert [row[0] for row in protocol]==[3,1,2] and protocol[2][1]==512
if a.visibility:assert all(v==0 for v in protocol[1][8:16])
else:assert protocol[1][8]==0x3f000000 and protocol[1][9]==0
if a.uv:
    assert protocol[1][24]==3
    assert all(v==0 for row in [protocol[0],protocol[2]] for v in row[20:28])
live=[]
names=['first-frame-diffuse','diffuse','reflection','diffuse-moved']
names += [n for n in ['diffuse-repeat','diffuse-small-move','diffuse-large-move'] if (root/f'{n}-input.bin').exists()]
for name in names:
    rays=list(struct.iter_unpack('<20I',(root/f'{name}-input.bin').read_bytes()));hits=list(struct.iter_unpack('<'+'I'*width,(root/f'{name}-hits.bin').read_bytes()));assert len(rays)==len(hits)==16
    cases=[json.loads(s) for s in (root/f'{name}-cases.jsonl').read_text().splitlines() if s]
    assert [r[18] for r in rays]==[(12+y)*32+12+x for y in range(4) for x in range(4)]
    assert all(r[19]==int(name=='reflection') for r in rays)
    assert len({r[18]|r[19]<<32 for r in rays})==16
    oracle=build(root,cases);expected={r['global_ray']:r for r in oracle['rows']};
    if a.uv:(root/f'{name}-gold-for-uv.json').write_text(json.dumps(oracle,indent=2)+'\n')
    if a.visibility:(root/f'{name}-gold-for-visibility.json').write_text(json.dumps(oracle,indent=2)+'\n')
    counts={'hit':0,'miss':0,'inactive':0,'unresolved':0};parameters=0
    for r,h in zip(rays,hits):
        identifier=r[18]|r[19]<<32;assert identifier==h[4]|h[5]<<32
        if a.uv and h[0]!=1:assert all(v==0 for v in h[20:28])
        if r[3]==0:assert h[0]==3 and h[6]==h[7]==0xffffffff;counts['inactive']+=1;continue
        if r[3]==2:assert h[0]==2 and h[1]==512;counts['unresolved']+=1;continue
        assert r[3]==1;g=expected[identifier];assert h[0]==int(g['exact_hit'])
        if a.visibility:
            assert all(v==0 for v in h[8:16]);counts['hit' if h[0]==1 else 'miss']+=1;continue
        assert [h[6],h[7],h[17]]==[g['winner_slot'],g['winner_primitive'],g['winner_material']]
        if g['exact_hit']:
            assert list(h[8:14])==g['parameter_words'];assert h[14]==1 and h[15]==g['residual_mask'];parameters+=1;counts['hit']+=1
        else:counts['miss']+=1
    if name=='first-frame-diffuse':assert counts['inactive']==16
    else:assert counts['hit']>0 and counts['inactive']==counts['unresolved']==0
    live.append({'frame':name,'rays':16,**counts,'exact_parameters_verified':parameters})
def coefficients(name):
    return [tuple(r[k] for k in [0,1,2,4,5,6,8,9,10,12,13,14]) for r in struct.iter_unpack('<20I',(root/f'{name}-input.bin').read_bytes())]
assert coefficients('diffuse')!=coefficients('diffuse-moved')
if 'diffuse-repeat' in names:
    assert coefficients('diffuse-repeat')==coefficients('diffuse-moved')
    assert coefficients('diffuse-repeat')!=coefficients('diffuse-small-move')
if 'diffuse-large-move' in names:
    assert coefficients('diffuse-small-move')!=coefficients('diffuse-large-move')
report={'scale_cases':len(inputs),'exact_pairs':valid,'unsupported_explicit':unsupported,'nonzero_low_words':low_words,'subnormal_low_words':subnormal_lows,'minimum_normal_plus_one_ulp_low_word':'0x80000004','all_transport_bits_exact':True,'hit_uv_enabled':a.uv,'opaque_visibility':a.visibility,'guard_protocol_cases':3,'live':live,'live_ray_source':'actual archive_ray_input with current frame guides/camera','GPU_producer_exact_consumer':True,'live_camera_changed':True,'image_shading_integrated':False,'fps_qualified':False};a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

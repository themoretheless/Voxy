#!/usr/bin/env python3
"""Validate GPU outward t brackets with independent exact dyadic/rational math."""
import argparse,json,struct
from fractions import Fraction as Q
from pathlib import Path
from audit_world_triangle_case import word,sub,cross,dot
p=argparse.ArgumentParser();p.add_argument('cases',type=Path);p.add_argument('gold',type=Path);p.add_argument('results',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
cases=[json.loads(s) for s in a.cases.read_text().splitlines() if s];gold={r['global_ray']:r for r in json.loads(a.gold.read_text())['rows']};raw=a.results.read_bytes();assert len(raw)==len(cases)*80
bounds=hits=rejected=0
f=lambda v:word(struct.unpack('<I',struct.pack('<f',v))[0])
for case,w in zip(cases,struct.iter_unpack('<20I',raw)):
    g=gold[case['global_ray']];assert w[4]|w[5]<<32==case['global_ray'];assert w[0]==int(g['exact_hit']);assert w[2]==2048
    if g['exact_hit']:
        hits+=1;assert list(w[8:14])==g['parameter_words'];assert w[14]==1;assert w[15]==g['residual_mask']
    if w[16]:assert not g['exact_hit'];rejected+=1
    if not w[17]:continue
    assert w[17]==1
    matrix=[[f(v) for v in column] for column in case['world']]
    points=[tuple(matrix[3][k]+sum((matrix[j][k]*f(point[j]) for j in range(3)),Q(0)) for k in range(3)) for point in case['local_triangle']]
    ray=case['ray_words'];origin=tuple(word(ray[k])+word(ray[4+k]) for k in range(3));direction=tuple(word(ray[8+k])+word(ray[12+k]) for k in range(3))
    e1,e2,delta=sub(points[1],points[0]),sub(points[2],points[0]),sub(origin,points[0]);den=dot(e1,cross(direction,e2));assert den
    t=dot(e2,cross(delta,e1))/den;lo,hi=word(w[18]),word(w[19]);assert lo<=t<=hi,(case['global_ray'],lo,t,hi)
    bounds+=1
report={'cases':len(cases),'exact_parameterized_hits':hits,'certified_misses':rejected,'outward_t_brackets_verified':bounds,'all_parameter_bits_exact':True,'rational_brackets_valid':True,'gold_uploaded_to_gpu':False,'frame_fps_measured':False};a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

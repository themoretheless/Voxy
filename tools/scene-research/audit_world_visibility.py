#!/usr/bin/env python3
"""Check declared visibility and every reported blocker against exact source data."""
import argparse,json,struct
from pathlib import Path
from fractions import Fraction as Q
from audit_world_triangle_case import decode_geometry
from exact_closed_world_triangle import classify
from make_world_visibility_gold import material_classes
p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('cases',type=Path);p.add_argument('gold',type=Path);p.add_argument('results',type=Path);p.add_argument('--stats',action='store_true');p.add_argument('--materials',action='store_true');p.add_argument('--output',type=Path,required=True);a=p.parse_args()
m=json.loads((a.root/'manifest.json').read_text());classes=material_classes(a.root,m,a.materials);sources=[]
for mesh in m['meshes']:
    vertices=(a.root/mesh['vertices']['file']).read_bytes();indices=struct.unpack('<'+'I'*mesh['index_count'],(a.root/mesh['indices']['file']).read_bytes());instances=(a.root/mesh['instances']['file']).read_bytes()
    for i in range(mesh['instance_count']):sources.append((mesh,vertices,indices,struct.unpack_from('<16f',instances,i*112)))
cases={r['global_ray']:r for r in (json.loads(s) for s in a.cases.read_text().splitlines() if s)};gold={r['global_ray']:r for r in json.loads(a.gold.read_text())['rows']};width=24 if a.stats else 20;raw=a.results.read_bytes();assert len(raw)==len(cases)*width*4
states=[0,0,0];routes={256:0,512:0};counts=[0,0,0,0];seen=set()
for w in struct.iter_unpack('<'+'I'*width,raw):
    identifier=w[4]|w[5]<<32;assert identifier not in seen;seen.add(identifier);g=gold[identifier];expected=g.get('visibility_state',int(g['exact_hit']));assert w[0]==expected;assert w[2]==2048 and 0<w[3]<=512;assert all(v==0 for v in w[8:16]);states[w[0]]+=1
    if a.stats:
        assert sum(w[20:23])==w[16]
        counts=[x+y for x,y in zip(counts,w[20:24])]
    if w[0]!=1:assert w[6]==w[7]==w[17]==0xffffffff;continue
    mesh,vertices,indices,matrix=sources[w[6]];assert w[17]==mesh['material'];assert not a.materials or classes[mesh['material']]==1
    selected=indices[w[7]*3:w[7]*3+3];assert len(selected)==3
    local=[struct.unpack_from('<3f',vertices,index*48) for index in selected];world=[list(matrix[i*4:i*4+4]) for i in range(4)]
    points,origin,direction=decode_geometry({'world':world,'local_triangle':local,'ray_words':cases[identifier]['ray_words']});assert classify(points,origin,direction)['exact_hit']
    route=w[18]&768;assert route in routes;routes[route]+=1
report={'cases':len(cases),'states':states,'reported_blockers_verified_exact':states[1],'float_certificate_blockers':routes[256],'integer_fallback_blockers':routes[512],'certificate_misses':counts[0] if a.stats else None,'certificate_hits':counts[1] if a.stats else None,'exact_fallback_triangles':counts[2] if a.stats else None,'source_nodes_visited':counts[3] if a.stats else None,'all_visibility_states_match_exact_oracle':True,'frame_fps_measured':False};a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

#!/usr/bin/env python3
"""Opaque blockers dominate uncertain layers; source Fraction predicates stay exact."""
import argparse,json,os,struct
from pathlib import Path
from audit_world_triangle_case import word
from make_world_nearest_gold import build,material_policy

def material_classes(root,manifest,enabled):
    if not enabled:return [1]*len(manifest.get('materials',[]))
    result=[]
    for m in manifest['materials']:
        values=struct.unpack('<20I',(root/m['parameters']['file']).read_bytes());decision=material_policy((values,m['images'][0]))
        if decision is None:result.append(2)
        elif not decision[0]:result.append(0)
        elif decision[1]!=2 or word(values[3])==1 and word(values[14])==0:result.append(1)
        else:result.append(2)
    return result

def visibility_gold(root,cases,materials=False):
    manifest=json.loads((root/'manifest.json').read_text());classes=material_classes(root,manifest,materials);slots=[]
    for mesh in manifest['meshes']:slots.extend([classes[mesh['material']] if materials else 1]*mesh['instance_count'])
    saved={key:os.environ.get(key) for key in ['VOXY_NEAREST_MATERIALS','VOXY_NEAREST_ORACLE_INACTIVE']}
    try:
        os.environ['VOXY_NEAREST_MATERIALS']='0'
        os.environ['VOXY_NEAREST_ORACLE_INACTIVE']=','.join(str(i) for i,c in enumerate(slots) if c!=1)
        opaque={r['global_ray']:r for r in build(root,cases)['rows']}
        unknown={}
        pending=[c for c in cases if not opaque[c['global_ray']]['exact_hit']]
        if materials and pending:
            os.environ['VOXY_NEAREST_ORACLE_INACTIVE']=','.join(str(i) for i,c in enumerate(slots) if c!=2)
            unknown={r['global_ray']:r for r in build(root,pending)['rows']}
    finally:
        for k,v in saved.items():
            if v is None:os.environ.pop(k,None)
            else:os.environ[k]=v
    rows=[]
    for case in cases:
        identifier=case['global_ray'];blocked=opaque[identifier]['exact_hit'];state=1 if blocked else 2 if unknown.get(identifier,{}).get('exact_hit') else 0
        rows.append({'global_ray':identifier,'exact_hit':blocked,'visibility_state':state,'parameter_words':None})
    return {'rows':rows,'cases':len(rows),'oracle':'source material Fraction classification + exhaustive outward-pruned exact closed-triangle predicates'}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('cases',type=Path);p.add_argument('output',type=Path);p.add_argument('--materials',action='store_true');a=p.parse_args();cases=[json.loads(s) for s in a.cases.read_text().splitlines() if s];result=visibility_gold(a.root,cases,a.materials)
    with a.output.open('x') as stream:json.dump(result,stream,indent=2)
    print(json.dumps({'cases':len(cases),'states':{str(i):sum(r['visibility_state']==i for r in result['rows']) for i in range(3)}}))

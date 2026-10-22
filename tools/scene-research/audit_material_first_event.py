#!/usr/bin/env python3
"""Independent rational first-event oracle over original GPU fixture sources.

MaterialSurface uncertainty is relevant only if it can precede or tie the first
known event. Ordinary Nearest retains its global unknown-opacity contract.
"""
import argparse
import functools
import json
import struct
from fractions import Fraction as Q
from pathlib import Path
from audit_material_surface_matrix import CONFIGS, configurations, rows, stem, signature, audit as matrix_audit
from audit_world_triangle_case import word, sub, dot, cross
from exact_closed_world_triangle import classify
from make_world_exact_parameter_gold import exact_parameters, pair
from make_world_nearest_gold import material_policy


def source_triangles(source):
    triangles=[]
    for slot,mesh in enumerate(source['geometry']):
        matrix=[[word(v)for v in c]for c in mesh['world']]
        points=[]
        for raw in mesh['positions']:
            local=[*map(word,raw),Q(1)]
            points.append([sum((matrix[c][axis]*local[c]for c in range(4)),Q(0))for axis in range(3)])
        for at in range(0,len(mesh['indices']),3):
            triangles.append((slot,at//3,mesh['material'],[points[v]for v in mesh['indices'][at:at+3]]))
    return triangles


def oracle_factory(source):
    triangles=source_triangles(source)
    materials=source['materials']
    @functools.lru_cache(None)
    def solve(key,policy):
        # signature preserves all source/admission/target words except queue
        # padding and stable IDs; rebuild their original indexed locations.
        ray=[0]*20
        indices=[i for i in range(20)if i not in (7,11,15,18,19)]
        for i,v in zip(indices,key):ray[i]=v
        if ray[3]==0:return (3,0,None,None,None)
        if ray[3]!=1:return (2,512,None,None,None)
        origin=[word(ray[k])+word(ray[k+4])for k in range(3)]
        direction=[word(ray[k+8])+word(ray[k+12])for k in range(3)]
        known=[];unknown=[]
        for slot,primitive,material,points in triangles:
            if not classify(points,origin,direction)['exact_hit']:continue
            parameters=exact_parameters(points,origin,direction)
            assert parameters is not None
            t=parameters[0]
            m=materials[material]
            if policy=='material'and not m['double_sided']:
                det=dot(sub(points[1],points[0]),cross(direction,sub(points[2],points[0])))
                if det<0:continue
                if det==0:
                    unknown.append((t,False));continue
            admission=material_policy((m['words'],m['image']))
            if admission is None:
                unknown.append((t,True));continue
            admitted,kind=admission
            if admitted:known.append((t,slot,primitive,material,parameters,kind))
        best=min(known,key=lambda v:v[:3])if known else None
        if unknown:
            earliest=min(t for t,_ in unknown)
            relevant=policy=='geometric'or best is None or earliest<=best[0]
            if relevant:
                material_unknown=any(m for t,m in unknown if policy=='geometric'or t==earliest)
                return (2,256 if material_unknown else 64,None,None,None)
        if best is None:return (0,0,None,None,None)
        t,slot,primitive,material,parameters,kind=best
        coords=[origin[k]+direction[k]*t for k in range(3)]
        return (1,0,(slot,primitive,material,kind),parameters,coords)
    return solve


def check(root):
    comparison=matrix_audit(root)
    checked=hits=0;reports=[]
    for name in configurations(root):
        directory=root/name/'data';proof=json.loads((directory/'proof.json').read_text())
        assert proof['surface_policy']=='first-material-event-exact'
        source=json.loads((directory/'fixture-source.json').read_text())
        solve=oracle_factory(source)
        for run in proof['runs']:
            key=stem(run);rays=rows(directory/(key+'-input-before.bin'))
            outputs=rows(directory/(key+'.bin'),proof.get('output_record_words',20))
            positions=list(struct.iter_unpack('<8I',(directory/(key+'-position.bin')).read_bytes()))if proof['hit_position']else None
            for i,(ray,hit)in enumerate(zip(rays,outputs)):
                state,reason,identity,parameters,coords=solve(signature(ray),run['policy'])
                assert hit[0]==state and hit[1]==reason,(name,key,i,proof['cases'][i],hit[:2],(state,reason))
                assert hit[4:6]==ray[18:20]
                if state==1:
                    slot,primitive,material,kind=identity
                    assert (hit[6],hit[7],hit[17])==(slot,primitive,material)
                    assert (hit[18]>>2)&3==kind
                    encoded=[pair(v)for v in parameters]
                    assert list(hit[8:14])==[v for words,_ in encoded for v in words],(name,key,i,'exact source TUV')
                    assert hit[14]==1 and hit[15]==sum(int(tail)<<axis for axis,(_,tail)in enumerate(encoded))
                    if positions is not None:
                        encoded_pos=[pair(v)for v in coords]
                        expected=[v for words,_ in encoded_pos for v in words]+[7,sum(int(tail)<<axis for axis,(_,tail)in enumerate(encoded_pos))]
                        assert list(positions[i])==expected,(name,key,i,'original-rational world position')
                    hits+=1
                else:
                    assert hit[6:8]==(0xffffffff,0xffffffff)and hit[8:16]==(0,)*8
                    if positions is not None:assert positions[i]==(0,)*8
                checked+=1
            reports.append(dict(configuration=name,run=key,rays=len(rays)))
    return dict(status='original_source_first_material_event_passed',checked_query_results=checked,
                checked_source_hits=hits,ordinary_nearest_global_unknown_preserved=True,
                material_surface_uses_exact_first_event_order=True,source_query_matrix=comparison,
                runs=reports,full_scene_qualified=False,fps_qualified=False)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();r=check(a.root);a.output.write_text(json.dumps(r,indent=2)+'\n')
    print(json.dumps({k:v for k,v in r.items()if k not in ('source_query_matrix','runs')}))

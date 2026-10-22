#!/usr/bin/env python3
"""One actual material query capture vs independent original-source rational math.
Unlike the cross-configuration matrix checker this does not require eight other
captures: it validates every query directly against original source geometry.
"""
import argparse,json,struct
from pathlib import Path
from audit_material_first_event import oracle_factory
from audit_material_surface_matrix import rows,stem,signature,PROTECTED
from make_world_exact_parameter_gold import pair


def audit(directory):
    state=json.loads((directory.parent/'process.json').read_text())
    assert state['state']=='terminal' and state['returncode']==0,'capture producer has not terminated successfully'
    proof=json.loads((directory/'proof.json').read_text());assert proof['schema']=='voxy-material-surface-nearest-v2' and proof['surface_policy']=='first-material-event-exact'
    source=json.loads((directory/'fixture-source.json').read_text());solve=oracle_factory(source)
    checked=hits=partial=0;reports=[];words=proof.get('output_record_words',20)
    for run in proof['runs']:
        key=stem(run);before=rows(directory/(key+'-input-before.bin'));after=rows(directory/(key+'-input-after.bin'));output=rows(directory/(key+'.bin'),words)
        assert len(before)==len(after)==len(output)==len(proof['cases'])==run['rays']
        positions=list(struct.iter_unpack('<8I',(directory/(key+'-position.bin')).read_bytes())) if proof['hit_position'] else None
        if positions is not None:assert len(positions)==len(before)
        for index,(ray,new,actual) in enumerate(zip(before,after,output)):
            indices=PROTECTED if proof['deferred'] else range(20);assert all(ray[k]==new[k] for k in indices),(key,index,'original source ray changed')
            state,reason,identity,parameters,coordinates=solve(signature(ray),run['policy'])
            assert actual[0:3]==(state,reason,2048),(key,index,'state/reason/precision',actual[:3],state,reason)
            assert actual[4:6]==ray[18:20],(key,index,'ray ID')
            if state==1:
                slot,primitive,material,kind=identity;assert (actual[6],actual[7],actual[17])==(slot,primitive,material)
                assert (actual[18]>>2)&3==kind
                pairs=[pair(v) for v in parameters];assert list(actual[8:14])==[v for encoded,_ in pairs for v in encoded]
                assert actual[14]==1 and actual[15]==sum(int(residual)<<axis for axis,(_,residual) in enumerate(pairs))
                if positions is not None:
                    pairs=[pair(v) for v in coordinates];expected=[v for encoded,_ in pairs for v in encoded]+[7,sum(int(residual)<<axis for axis,(_,residual) in enumerate(pairs))]
                    assert list(positions[index])==expected,(key,index,'exact world point pairs/residuals')
                hits+=1
            else:
                assert actual[6:8]==(0xffffffff,0xffffffff) and actual[8:16]==(0,)*8
                if positions is not None:assert positions[index]==(0,)*8
            checked+=1
        if proof.get('partial_position_check') and positions is not None:
            data=(directory/(key+'-partial.bin')).read_bytes();primary=(directory/(key+'.bin')).read_bytes();tail=(directory/(key+'-position.bin')).read_bytes();size=len(before)*words*4
            assert len(data)==size+len(tail) and struct.unpack_from('<I',data)[0]==3
            assert data[words*4:size]==primary[words*4:] and data[size:size+32]==bytes(32) and data[size+32:]==tail[32:]
            partial+=1
        reports.append({'run':key,'queries':len(before)})
    assert checked>0
    return {'status':'single_actual_capture_matches_independent_original_source_rational_oracle','checked_query_results':checked,'checked_hit_positions':hits if proof['hit_position'] else 0,'partial_hit_to_inactive_checks':partial,'source_slots':len(source['geometry']),'all_original_source_coefficients_and_ray_ids_preserved':True,'CPU_geometry_gold_uploaded':False,'source_runs':reports,'full_scene_fidelity_qualified':False,'FPS_qualified':False}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();report=audit(args.directory);args.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='source_runs'}))

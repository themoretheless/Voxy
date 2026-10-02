#!/usr/bin/env python3
"""Audit static nonadjacent boundary intersections without welding FEM nodes."""
import argparse,collections,json
from pathlib import Path
from compare_supported_wall_time import mesh,digest
from compare_anatomy_hgo_time import read
from compare_anatomy_hgo_space import validate_protocol
from prepare_body_geometry import intersections
from classify_body_intersections import classify

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--meshes',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    rows,ch=read(Path(str(a.meshes)+'.csv'));validate_protocol(rows,a.meshes)
    ref,faces=mesh(a.meshes/'rest.obj');states=[]
    for stage in ['rest']+[r['stage'] for r in rows]:
        path=a.meshes/(stage+'.obj');points,f=mesh(path)
        if f!=faces or len(points)!=len(ref):raise ValueError('state topology differs')
        result=intersections(points,f,limit=100000)
        if result['intersection_limit_reached']:raise ValueError('intersection audit truncated')
        counts=collections.Counter(classify([points[i] for i in f[x]],[points[i] for i in f[y]]) for x,y in result['intersection_pairs'])
        states.append(dict(stage=stage,sha256=digest(path),candidate_pairs=result['intersection_pairs'],classification_counts=dict(counts),narrowphase_pairs=result['narrowphase_pairs']))
    report=dict(scope='Static floating-point nonadjacent boundary audit; shared-vertex pairs excluded; no welding, CCD, contact solver or anatomical certification',nodes=len(ref),boundary_faces=len(faces),csv_sha256=ch,states=states,total_candidate_pairs=sum(len(s['candidate_pairs']) for s in states))
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['nodes','boundary_faces','total_candidate_pairs']}))
if __name__=='__main__':main()

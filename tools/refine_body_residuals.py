#!/usr/bin/env python3
"""Paired conforming refinement with explicit parent/vertex correspondence."""
import argparse,hashlib,json,math
from pathlib import Path
import prepare_body_geometry as g

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('source','reference','residual_report','output','reference_output'):parser.add_argument(name,type=Path)
    args=parser.parse_args()
    if args.output.exists() or args.reference_output.exists():parser.error('output already exists')
    points,faces=g.load(args.source);reference,_=g.load(args.reference)
    if len(points)!=len(reference):raise ValueError('reference vertex identity mismatch')
    report=json.loads(args.residual_report.read_text());remaining=report['remaining']
    declared=report.get('candidate_sha256',report.get('candidateSha256'))
    if declared!=hashlib.sha256(args.source.read_bytes()).hexdigest():raise ValueError('residual report does not match candidate bytes')
    pairs=remaining['intersection_pairs']+remaining.get('adjacent',{}).get('forbidden_pairs',[])
    if not pairs:raise ValueError('no residual faces')
    marked={tuple(sorted((faces[i][k],faces[i][(k+1)%3]))) for pair in pairs for i in pair for k in range(3)}
    new,newfaces=g.split_edges(points,faces,marked,200000)
    new_reference,reference_faces=g.split_edges(reference,faces,marked,200000)
    if newfaces!=reference_faces:raise ValueError('paired topology mismatch')
    parents=[];weights=[[(i,1.)] for i in range(len(points))]
    weights.extend([[(a,.5),(b,.5)] for a,b in sorted(marked)])
    for i,t in enumerate(faces):
        count=sum(tuple(sorted((t[k],t[(k+1)%3]))) in marked for k in range(3))
        parents.extend([i]*(3+count if count else 1))
        if count:weights.append([(v,1/3) for v in t])
    if len(weights)!=len(new) or len(parents)!=len(newfaces):raise ValueError('correspondence length mismatch')
    for old,current in ((points,new),(reference,new_reference)):
        for i,row in enumerate(weights):
            expected=tuple(sum(old[v][k]*w for v,w in row) for k in range(3))
            if math.dist(expected,current[i])>1e-14:raise ValueError('parent interpolation mismatch')
    before=g.audit(points,faces);after=g.audit(new,newfaces)
    if any(after[k] for k in ('degenerate_triangles','duplicate_triangles','boundary_edges','nonmanifold_edges','inconsistent_winding_edges')):raise ValueError('invalid refined topology')
    if abs(after['area_m2']-before['area_m2'])/before['area_m2']>1e-12:raise ValueError('surface area changed')
    for path,positions in ((args.output,new),(args.reference_output,new_reference)):
        with path.open('x') as stream:
            for p in positions:stream.write('v '+' '.join(format(x,'.17g') for x in p)+'\n')
            for t in newfaces:stream.write('f '+' '.join(str(i+1) for i in t)+'\n')
        if g.load(path)!=(positions,newfaces):raise ValueError('export reload mismatch')
    proof=dict(source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),marked_edges=len(marked),added_vertices=len(new)-len(points),before=before,after=after,surface_preserved=True,maximum_cumulative_displacement_m=max(math.dist(p,q) for p,q in zip(new,new_reference)),adopted=False)
    proof.update(output_sha256=hashlib.sha256(args.output.read_bytes()).hexdigest(),reference_output_sha256=hashlib.sha256(args.reference_output.read_bytes()).hexdigest())
    args.output.with_suffix('.parents.json').write_text(json.dumps(dict(parent_cells=parents,vertex_sources=weights,proof=proof),separators=(',',':'))+'\n')
    print(json.dumps(proof))

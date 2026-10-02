#!/usr/bin/env python3
"""Conforming refinement around reference nipple landmarks; source is preserved.
This prepares surface density, not internal anatomy or a calibrated nipple shape.
"""
import argparse, hashlib, json
from pathlib import Path
from prepare_body_geometry import load, audit, length, sub


def split_edges(points, faces, marked, budget, parents):
    points=list(points); mid={}
    for a,b in sorted(marked):
        mid[(a,b)]=len(points)
        points.append(tuple((x+y)/2 for x,y in zip(points[a],points[b])))
    result=[];new_parents=[]
    for face,parent in zip(faces,parents):
        t=list(face)
        edges=[tuple(sorted((t[i],t[(i+1)%3]))) for i in range(3)]
        count=sum(e in mid for e in edges)
        if count==0: result.append(face);new_parents.append(parent);continue
        if count==3:
            a,b,c=t;m,n,o=[mid[e] for e in edges]
            result.extend(((a,m,o),(m,b,n),(o,n,c),(m,n,o)));new_parents.extend([parent]*4);continue
        # Rotate to put the single marked edge first, or the unmarked edge last.
        rotation=next(i for i in range(3) if (edges[i] in mid and (count==1 or edges[(i+1)%3] in mid)))
        a,b,c=t[rotation:]+t[:rotation];m=mid[tuple(sorted((a,b)))]
        if count==1: result.extend(((a,m,c),(m,b,c)));new_parents.extend([parent]*2)
        else:
            n=mid[tuple(sorted((b,c)))]
            result.extend(((b,n,m),(a,m,c),(m,n,c)));new_parents.extend([parent]*3)
    if len(result)>budget: raise ValueError('triangle budget exceeded')
    return points,result,new_parents


def refine(points, faces, target_edge=0.002, radius=0.025, rounds=16):
    stages=[];parents=list(range(len(faces)))
    for _ in range(rounds):
        marked=set()
        for face in faces:
            for a,b in zip(face,face[1:]+face[:1]):
                p,q=points[a],points[b]
                midpoint=tuple((x+y)/2 for x,y in zip(p,q))
                if midpoint[2] > 0.08 and (abs(midpoint[0])-0.08)**2+(midpoint[1]-0.36)**2 < radius**2 and length(sub(p,q))>target_edge:
                    marked.add(tuple(sorted((a,b))))
        if not marked: break
        points,faces,parents=split_edges(points,faces,marked,2_000_000,parents)
        stages.append(dict(split_edges=len(marked),vertices=len(points),triangles=len(faces)))
    return points,faces,stages,parents


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path);parser.add_argument('output',type=Path)
    parser.add_argument('--correspondence-only',action='store_true',help='write parent map without modifying adopted mesh')
    args=parser.parse_args()
    points,faces=load(args.source);before=audit(points,faces)
    source_faces=list(faces)
    points,faces,stages,parents=refine(points,faces);after=audit(points,faces)
    for key in ('boundary_edges','nonmanifold_edges','inconsistent_winding_edges','duplicate_triangles'):
        if before[key]!=after[key]: raise ValueError('refinement changed topology: '+key)
    if after['degenerate_triangles'] or abs(after['area_m2']-before['area_m2'])>1e-10: raise ValueError('refinement changed surface area or introduced degeneracy')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    if not args.correspondence_only: args.output.write_text('# Reference surface with locally refined landmarks; original surface retained\n'+''.join('v '+' '.join(format(x,'.17g') for x in p)+'\n' for p in points)+''.join('f '+' '.join(str(i+1) for i in t)+'\n' for t in faces))
    local_edges=set()
    for t in faces:
        for a,b in zip(t,t[1:]+t[:1]):
            q=tuple((x+y)/2 for x,y in zip(points[a],points[b]))
            if q[2]>.08 and (abs(q[0])-.08)**2+(q[1]-.36)**2<.025**2:
                local_edges.add(tuple(sorted((a,b))))
    maximum=max(length(sub(points[a],points[b])) for a,b in local_edges)
    if maximum > .002000001: raise ValueError('local edge refinement did not converge')
    report=dict(maximum_local_edge_m=maximum,source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),output_sha256=hashlib.sha256(args.output.read_bytes()).hexdigest(),before=before,after=after,stages=stages,runtime_adopted=False,internal_anatomy=False)
    if not args.correspondence_only: args.output.with_suffix('.audit.json').write_text(json.dumps(report,indent=2)+'\n')
    correspondence=dict(source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),source_topology_sha256=hashlib.sha256(json.dumps(source_faces,separators=(',',':')).encode()).hexdigest(),refined_topology_sha256=hashlib.sha256(json.dumps(faces,separators=(',',':')).encode()).hexdigest(),parent_cells=parents,source_cells=len(source_faces),refined_cells=len(faces))
    args.output.with_suffix('.parents.json').write_text(json.dumps(correspondence,separators=(',',':'))+'\n')
    print(json.dumps(dict(stages=stages,before_vertices=before['vertices'],after_vertices=after['vertices'],area_error=after['area_m2']-before['area_m2'])))

if __name__=='__main__':main()

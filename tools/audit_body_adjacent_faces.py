#!/usr/bin/env python3
"""Detect positive-area/length intersections beyond shared topology."""
import collections,hashlib,json
from pathlib import Path
import prepare_body_geometry as g

def overlap_area(a,b):
    def cross(u,v):return u[0]*v[1]-u[1]*v[0]
    def area(p):return sum(cross(u,v) for u,v in zip(p,p[1:]+p[:1]))/2
    polygon=list(a);clip=list(b)
    if area(clip)<0:clip.reverse()
    for x,y in zip(clip,clip[1:]+clip[:1]):
        result=[]
        if not polygon:return 0.
        previous=polygon[-1];dp=cross(g.sub(y,x),g.sub(previous,x))
        for current in polygon:
            dc=cross(g.sub(y,x),g.sub(current,x))
            if (dp>=0)!=(dc>=0):
                t=dp/(dp-dc);result.append(tuple(previous[k]+t*(current[k]-previous[k]) for k in range(2)))
            if dc>=0:result.append(current)
            previous=current;dp=dc
        polygon=result
    return abs(area(polygon)) if len(polygon)>=3 else 0.

def forbidden(a,b,shared):
    if shared==1:
        common=set(a).intersection(b)
        if len(common)!=1:raise ValueError('shared vertex identity mismatch')
        point=next(iter(common))
        # Anchor both plane intervals at their exact shared point. This avoids
        # manufacturing a tiny segment through cancellation at a valid contact.
        i=a.index(point);a=a[i:]+a[:i]
        i=b.index(point);b=b[i:]+b[:i]
    n=g.cross(g.sub(a[1],a[0]),g.sub(a[2],a[0]));m=g.cross(g.sub(b[1],b[0]),g.sub(b[2],b[0]))
    scale=max(g.length(g.sub(t[k],t[(k+1)%3])) for t in (a,b) for k in range(3))
    parallel=g.length(g.cross(n,m))<=1e-10*g.length(n)*g.length(m)
    if parallel:
        if max(abs(g.dot(n,g.sub(p,a[0]))) for p in b)>1e-10*scale*g.length(n):return False
        axis=max(range(3),key=lambda k:abs(n[k]));axes=[k for k in range(3) if k!=axis]
        project=lambda t:[tuple(p[k] for k in axes) for p in t]
        return overlap_area(project(a),project(b))>scale*scale*1e-10
    if shared==2:return False # Distinct planes intersect only on their shared edge.
    return g.intersection_segment_length(a,b)>scale*1e-10

def audit(points,faces):
    incident=collections.defaultdict(list)
    for i,t in enumerate(faces):
        for v in t:incident[v].append(i)
    pairs={tuple(sorted((a,b))) for rows in incident.values() for i,a in enumerate(rows) for b in rows[i+1:]}
    hits=[]
    for a,b in sorted(pairs):
        shared=len(set(faces[a]).intersection(faces[b]))
        if forbidden([points[v] for v in faces[a]],[points[v] for v in faces[b]],shared):hits.append([a,b])
    return dict(tested_pairs=len(pairs),forbidden_pairs=hits,
        scope='static adjacent positive length/area beyond shared vertex/edge; relative tolerance 1e-10; isolated extra point contacts not detected')

if __name__=='__main__':
    source=Path('assets/characters/blender-male/body-fine-vertex-descent-candidate.obj')
    points,faces=g.load(source);result=audit(points,faces)
    result['source_sha256']=hashlib.sha256(source.read_bytes()).hexdigest()
    Path('docs/body-male-adjacent-face-audit.json').write_text(json.dumps(result,indent=2)+'\n')
    print('tested',result['tested_pairs'],'forbidden',len(result['forbidden_pairs']))

#!/usr/bin/env python3
"""Conservative material charts for local body retriangulation."""
import collections,hashlib,json,math
from pathlib import Path
import numpy as np
from prepare_body_geometry import load
from body_film_correspondence import overlap,signed_area

def edges(faces):
    return collections.Counter(tuple(sorted((a,b))) for t in faces for a,b in zip(t,t[1:]+t[:1]))

def chart(points,source,target):
    boundary={e for e,n in edges(source).items() if n==1}
    if boundary!={e for e,n in edges(target).items() if n==1}:raise ValueError('patch boundaries differ')
    neighbors=collections.defaultdict(set)
    for a,b in boundary:neighbors[a].add(b);neighbors[b].add(a)
    if not boundary or any(len(n)!=2 for n in neighbors.values()):raise ValueError('patch is not a boundary disk')
    start=min(neighbors);cycle=[start];previous=None;current=start
    while True:
        following=min(neighbors[current]-({previous} if previous is not None else set()))
        if following==start:break
        if following in cycle:raise ValueError('invalid boundary cycle')
        cycle.append(following);previous,current=current,following
    if len(cycle)!=len(neighbors):raise ValueError('multiple boundary loops')
    lengths=[math.dist(points[a],points[b]) for a,b in zip(cycle,cycle[1:]+cycle[:1])]
    total=sum(lengths);cursor=0.;uv={}
    for vertex,length in zip(cycle,lengths):
        angle=cursor/total*2*math.pi;uv[vertex]=(math.cos(angle),math.sin(angle));cursor+=length
    graph=collections.defaultdict(set)
    for a,b in edges(source):graph[a].add(b);graph[b].add(a)
    interior=sorted(set(graph)-set(uv));lookup={v:i for i,v in enumerate(interior)}
    matrix=np.zeros((len(interior),len(interior)));rhs=np.zeros((len(interior),2))
    for v,i in lookup.items():
        matrix[i,i]=len(graph[v])
        for n in graph[v]:
            if n in lookup:matrix[i,lookup[n]]-=1
            else:rhs[i]+=uv[n]
    if interior:
        solution=np.linalg.solve(matrix,rhs)
        uv.update({v:tuple(solution[i]) for v,i in lookup.items()})
    source_uv=[[uv[v] for v in t] for t in source];target_uv=[[uv[v] for v in t] for t in target]
    signs=[signed_area(t) for t in source_uv+target_uv]
    if min(abs(s) for s in signs)<1e-12 or min(signs)*max(signs)<=0:raise ValueError('folded or degenerate material chart')
    return source_uv,target_uv

def build(points,source,target):
    if len(source)!=len(target):raise ValueError('cell count changed')
    changed={i for i,(a,b) in enumerate(zip(source,target)) if set(a)!=set(b)}
    incident=collections.defaultdict(set)
    for i in changed:
        for e in edges([source[i]]):incident[e].add(i)
    groups=[];pending=set(changed)
    while pending:
        group={pending.pop()};todo=list(group)
        while todo:
            i=todo.pop()
            for e in edges([source[i]]):
                for j in incident[e]&pending:pending.remove(j);group.add(j);todo.append(j)
        groups.append(sorted(group))
    forward=[[(i,1.)] for i in range(len(source))];reverse=[[(i,1.)] for i in range(len(source))];maximum=0.
    for group in groups:
        a,b=chart(points,[source[i] for i in group],[target[i] for i in group])
        for donor,recipient,rows in ((a,b,forward),(b,a,reverse)):
            for k,t in enumerate(donor):
                row=[(group[j],overlap(t,u)/abs(signed_area(t))) for j,u in enumerate(recipient)]
                row=[(i,w) for i,w in row if w>1e-13];total=sum(w for _,w in row)
                maximum=max(maximum,abs(total-1))
                if abs(total-1)>1e-9:raise ValueError('incomplete material chart coverage')
                rows[group[k]]=[(i,w/total) for i,w in row]
    return forward,reverse,dict(changed_cells=len(changed),patches=len(groups),patch_sizes=list(map(len,groups)),maximum_raw_row_sum_error=maximum,
        mapping='harmonic disk charts on unchanged patch boundaries; material-space remap, not physical projection')

def compose(first,second):
    rows=[]
    for row in first:
        accumulated=collections.defaultdict(float)
        for middle,weight in row:
            for target,fraction in second[middle]:accumulated[target]+=weight*fraction
        total=sum(accumulated.values())
        if abs(total-1)>1e-9:raise ValueError('nonconservative composition')
        rows.append([(i,w/total) for i,w in sorted(accumulated.items()) if w>0])
    return rows

def refinement_map(points,faces,refined_points,refined_faces,parents):
    """Conservative distribution on an unchanged, conformingly refined surface."""
    p=np.asarray(points,dtype=float);q=np.asarray(refined_points,dtype=float)
    if p.ndim!=2 or q.ndim!=2 or p.shape[1]!=3 or q.shape[1]!=3 or not np.all(np.isfinite(p)) or not np.all(np.isfinite(q)):
        raise ValueError('invalid refinement coordinates')
    if len(parents)!=len(refined_faces):raise ValueError('parent cell count mismatch')
    groups=collections.defaultdict(list)
    for i,parent in enumerate(parents):
        if not isinstance(parent,int) or not 0<=parent<len(faces):raise ValueError('invalid parent cell')
        groups[parent].append(i)
    if len(groups)!=len(faces):raise ValueError('missing parent coverage')
    forward=[];reverse=[None]*len(refined_faces);maximum=0.
    for parent,face in enumerate(faces):
        a,b,c=p[list(face)];u=b-a;v=c-a
        uu=float(u@u);uv=float(u@v);vv=float(v@v);den=uu*vv-uv*uv
        if den<=0:raise ValueError('degenerate parent cell')
        projected=[];areas=[]
        for child in groups[parent]:
            tri=q[list(refined_faces[child])];w=tri-a
            y=(vv*(w@u)-uv*(w@v))/den;z=(uu*(w@v)-uv*(w@u))/den
            if np.max(np.linalg.norm(w-y[:,None]*u-z[:,None]*v,axis=1))>1e-10 or np.min(np.stack((1-y-z,y,z))) < -1e-10:
                raise ValueError('child lies outside parent surface')
            chart_tri=list(zip(y,z));area=signed_area(chart_tri)
            if area<=0:raise ValueError('reversed or degenerate child cell')
            for previous in projected:
                if overlap(chart_tri,previous)>1e-10:raise ValueError('overlapping child cells')
            projected.append(chart_tri);areas.append(area)
            reverse[child]=[(parent,1.)]
        total=math.fsum(areas);maximum=max(maximum,abs(total-.5))
        if abs(total-.5)>1e-10:raise ValueError('incomplete parent coverage')
        forward.append([(i,area/total) for i,area in zip(groups[parent],areas)])
    return forward,reverse,dict(source_cells=len(faces),target_cells=len(refined_faces),maximum_parent_chart_area_error=maximum,mapping='unchanged parent surface area fractions; extensive liquid volume distribution')

if __name__=='__main__':
    source=Path('assets/characters/blender-male/body-refined.obj');target=Path('assets/characters/blender-male/body-repaired-render-candidate.obj')
    points,faces=load(source);_,newfaces=load(target)
    forward,reverse,proof=build(points,faces,newfaces)
    proof['mesh_sha256']={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in (source,target)}
    for name,rows in (('male-original-to-repaired-film-map.json',forward),('male-repaired-to-original-film-map.json',reverse)):
        Path('assets/characters',name).write_text(json.dumps(dict(distribution=rows,proof=proof),separators=(',',':'))+'\n')
    female_forward=json.loads(Path('assets/characters/female-to-male-film-map.json').read_text())['distribution']
    female_reverse=json.loads(Path('assets/characters/male-to-female-film-map.json').read_text())['distribution']
    for name,rows in (('female-to-repaired-male-film-map.json',compose(female_forward,forward)),('repaired-male-to-female-film-map.json',compose(reverse,female_reverse))):
        Path('assets/characters',name).write_text(json.dumps(dict(distribution=rows,proof=proof),separators=(',',':'))+'\n')
    Path('docs/body-male-repair-film-map-proof.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps(proof))

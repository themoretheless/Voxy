#!/usr/bin/env python3
"""Conservative correspondence in shared quad charts, not closest-point matching.
Requires equal source quad vertex keys and NumPy. Refined faces retain parent ids.
"""
import argparse,collections,hashlib,json
from pathlib import Path
import numpy as np
from prepare_body_geometry import load


def cross(a,b):return a[0]*b[1]-a[1]*b[0]
def sub(a,b):return (a[0]-b[0],a[1]-b[1])
def signed_area(p):return sum(cross(a,b) for a,b in zip(p,p[1:]+p[:1]))/2

def overlap(a,b):
    polygon=list(a);clip=list(b)
    if signed_area(clip)<0:clip.reverse()
    for x,y in zip(clip,clip[1:]+clip[:1]):
        result=[]
        if not polygon:return 0.
        previous=polygon[-1];dp=cross(sub(y,x),sub(previous,x))
        for current in polygon:
            dc=cross(sub(y,x),sub(current,x))
            if (dp>=0)!=(dc>=0):
                t=dp/(dp-dc)
                result.append(tuple(previous[k]+t*(current[k]-previous[k]) for k in range(2)))
            if dc>=0:result.append(current)
            previous=current;dp=dc
        polygon=result
    return abs(signed_area(polygon)) if len(polygon)>=3 else 0.


def charts(source,refined):
    points,faces=load(source);new_points,new_faces=load(refined)
    parent=json.loads(Path(refined).with_suffix('.parents.json').read_text())
    if hashlib.sha256(json.dumps(new_faces,separators=(',',':')).encode()).hexdigest()!=parent['refined_topology_sha256']:raise ValueError('refined topology changed')
    if hashlib.sha256(Path(source).read_bytes()).hexdigest()!=parent['source_sha256']:raise ValueError('source mesh changed')
    face_chart=[];keys=[];boundary_keys=[]
    for i in range(0,len(faces),2):
        pair=faces[i:i+2];key=tuple(sorted(set(pair[0]+pair[1])))
        if len(key)!=4:raise ValueError('face pair is not a quad')
        edges=collections.Counter(tuple(sorted((a,b))) for t in pair for a,b in zip(t,t[1:]+t[:1]))
        boundary=sorted(e for e,n in edges.items() if n==1)
        neighbors={v:sorted(b if a==v else a for a,b in boundary if v in (a,b)) for v in key}
        start=key[0];second=neighbors[start][0];third=next(v for v in neighbors[second] if v!=start);fourth=next(v for v in neighbors[third] if v!=second)
        if fourth not in neighbors[start]:raise ValueError('quad boundary not cyclic')
        uv=dict(zip((start,second,third,fourth),((0.,0.),(1.,0.),(1.,1.),(0.,1.))))
        face_chart.extend([[uv[v] for v in t] for t in pair]);keys.append(key);boundary_keys.append(boundary)
    p=np.asarray(points);f=np.asarray(faces);nf=np.asarray(new_faces);npnt=np.asarray(new_points);parents=np.asarray(parent['parent_cells'])
    source_tri=p[f[parents]];tri=npnt[nf];u=source_tri[:,1]-source_tri[:,0];v=source_tri[:,2]-source_tri[:,0]
    uu=np.sum(u*u,axis=1);uv=np.sum(u*v,axis=1);vv=np.sum(v*v,axis=1);den=uu*vv-uv*uv
    w=tri-source_tri[:,0,None,:];wu=np.sum(w*u[:,None,:],axis=2);wv=np.sum(w*v[:,None,:],axis=2)
    y=(vv[:,None]*wu-uv[:,None]*wv)/den[:,None];z=(uu[:,None]*wv-uv[:,None]*wu)/den[:,None]
    bary=np.stack((1-y-z,y,z),axis=2)
    if np.min(bary)<-1e-9:raise ValueError('child leaves parent triangle')
    reconstructed=np.einsum('nki,nij->nkj',bary,source_tri)
    if np.max(np.abs(reconstructed-tri))>1e-10:raise ValueError('child leaves source surface')
    chart=np.einsum('nki,nij->nkj',bary,np.asarray(face_chart)[parents]).tolist()
    return chart,[keys[i//2] for i in parents],dict(zip(keys,boundary_keys))


def build(source,target,source_refined,target_refined):
    donor,donor_keys,source_boundary=charts(source,source_refined)
    recipient,recipient_keys,target_boundary=charts(target,target_refined)
    if source_boundary!=target_boundary:raise ValueError('source quad keys/boundaries differ')
    groups=collections.defaultdict(list)
    for i,key in enumerate(recipient_keys):groups[key].append(i)
    rows=[];maximum_error=0.
    for triangle,key in zip(donor,donor_keys):
        area=abs(signed_area(triangle))
        if area<=0:raise ValueError('invalid chart triangle')
        row=[]
        for i in groups[key]:
            fraction=overlap(triangle,recipient[i])/area
            if fraction>1e-13:row.append((i,fraction))
        total=sum(w for _,w in row);error=abs(total-1.);maximum_error=max(maximum_error,error)
        if error>1e-9:raise ValueError('chart coverage is incomplete')
        rows.append([(i,w/total) for i,w in row])
    return rows,dict(donors=len(donor),recipients=len(recipient),shared_quads=len(source_boundary),maximum_raw_row_sum_error=maximum_error,entries=sum(map(len,rows)),mapping='piecewise affine shared quad charts; no physiological calibration')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('source','target','source_refined','target_refined','output'):p.add_argument(name,type=Path)
    a=p.parse_args();rows,proof=build(a.source,a.target,a.source_refined,a.target_refined)
    proof['mesh_sha256']={str(path):hashlib.sha256(path.read_bytes()).hexdigest() for path in (a.source,a.target,a.source_refined,a.target_refined)}
    a.output.write_text(json.dumps(dict(distribution=rows,proof=proof),separators=(',',':'))+'\n');print(json.dumps(proof))
if __name__=='__main__':main()

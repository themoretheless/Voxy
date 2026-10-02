#!/usr/bin/env python3
"""Reproject moved body vertices onto the existing physics shell."""
import argparse,hashlib,json,math
from pathlib import Path
import numpy as np
from prepare_body_geometry import load

def closest(triangles,point):
    u=triangles[:,1]-triangles[:,0];v=triangles[:,2]-triangles[:,0]
    uu=np.sum(u*u,axis=1);uv=np.sum(u*v,axis=1);vv=np.sum(v*v,axis=1)
    den=uu*vv-uv*uv
    if np.any(den<=0):raise ValueError('degenerate shell triangle')
    w=point-triangles[:,0];wu=np.sum(w*u,axis=1);wv=np.sum(w*v,axis=1)
    y=(vv*wu-uv*wv)/den;z=(uu*wv-uv*wu)/den
    weights=np.stack((1-y-z,y,z),axis=1)
    distance=np.sum((np.sum(triangles*weights[:,:,None],axis=1)-point)**2,axis=1)
    distance[np.any(weights<0,axis=1)]=np.inf
    for i,j in ((0,1),(1,2),(2,0)):
        edge=triangles[:,j]-triangles[:,i]
        t=np.clip(np.sum((point-triangles[:,i])*edge,axis=1)/np.sum(edge*edge,axis=1),0,1)
        d=np.sum((triangles[:,i]+t[:,None]*edge-point)**2,axis=1);mask=d<distance
        bary=np.zeros_like(weights);bary[:,i]=1-t;bary[:,j]=t
        distance[mask]=d[mask];weights[mask]=bary[mask]
    face=int(np.argmin(distance));return face,weights[face].tolist(),math.sqrt(float(distance[face]))

def rebind(reference,points,data,allow_added=False,added_fades=None):
    old=data['bindings']
    if len(old)!=len(reference) or len(points)<len(reference) or (len(points)!=len(reference) and not allow_added):raise ValueError('vertex identity mismatch')
    triangles=np.asarray(data['positions'])[np.asarray(data['triangles'])]
    bindings=[];moved=[];distances=[]
    for i,point in enumerate(points):
        if len(point)!=3 or not all(math.isfinite(x) for x in point):raise ValueError('invalid vertex coordinates')
        rest=reference[i] if i<len(reference) else None
        fade=(added_fades or {}).get(i,1.)
        if not math.isfinite(fade) or not 0<=fade<=1:raise ValueError('invalid added attachment fade')
        binding=old[i] if rest is not None else {'vertex':i,'fade':fade}
        if rest is not None and (binding['vertex']!=i or tuple(np.asarray(binding['position'],dtype=np.float32))!=tuple(np.asarray(rest,dtype=np.float32))):raise ValueError('binding identity mismatch')
        result=dict(binding);result['position']=list(point)
        if rest!=point:
            face,weights,distance=closest(triangles,np.asarray(point))
            if not math.isfinite(distance) or min(weights)<0 or abs(sum(weights)-1)>1e-10:raise ValueError('invalid projection')
            result.update(triangle=face,weights=weights);moved.append(i);distances.append(distance)
        bindings.append(result)
    output=dict(data);output.update(bindings=bindings,render_vertices=len(points),max_binding_distance_m=max(data['max_binding_distance_m'],max(distances,default=0)))
    return output,dict(vertices=len(points),reprojected_vertices=len(moved),added_vertices=len(points)-len(reference),preserved_vertices=len(points)-len(moved),maximum_reprojected_distance_m=max(distances,default=0),shell_unchanged=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference',type=Path);parser.add_argument('candidate',type=Path)
    parser.add_argument('skin',type=Path);parser.add_argument('output',type=Path)
    parser.add_argument('--vertex-sources',type=Path)
    args=parser.parse_args()
    if args.output.exists():parser.error('output already exists')
    reference,_=load(args.reference);points,_=load(args.candidate)
    data=json.loads(args.skin.read_text())
    if args.vertex_sources:
        sources=json.loads(args.vertex_sources.read_text())
        if sources['proof']['reference_sha256']!=hashlib.sha256(args.reference.read_bytes()).hexdigest():raise ValueError('reference provenance mismatch')
        rows=sources['vertex_sources']
        if len(rows)!=len(points) or rows[:len(reference)]!=[[[i,1.]] for i in range(len(reference))]:raise ValueError('source vertex identity mismatch')
        for row in rows:
            if not row or any(not isinstance(i,int) or i<0 or i>=len(reference) or not math.isfinite(w) or w<=0 for i,w in row) or abs(sum(w for _,w in row)-1)>1e-12:raise ValueError('invalid refinement weights')
    fades={i:sum(data['bindings'][j]['fade']*w for j,w in row) for i,row in enumerate(rows) if i>=len(reference)} if args.vertex_sources else None
    data,report=rebind(reference,points,data,allow_added=args.vertex_sources is not None,added_fades=fades)
    args.output.write_text(json.dumps(data,separators=(',',':'))+'\n')
    report.update(reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),candidate_sha256=hashlib.sha256(args.candidate.read_bytes()).hexdigest(),source_skin_sha256=hashlib.sha256(args.skin.read_bytes()).hexdigest(),output_sha256=hashlib.sha256(args.output.read_bytes()).hexdigest(),adopted=False)
    if args.vertex_sources:report['vertex_sources_sha256']=hashlib.sha256(args.vertex_sources.read_bytes()).hexdigest()
    args.output.with_suffix('.audit.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

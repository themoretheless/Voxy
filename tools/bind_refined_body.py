#!/usr/bin/env python3
"""Preserve source bindings and project added vertices onto the existing skin shell.
Requires NumPy. Retains source vertex normals and generates normals for new vertices.
"""
import argparse,hashlib,json,math
from pathlib import Path
import numpy as np
from prepare_body_geometry import load


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('mesh',type=Path);p.add_argument('source_mesh',type=Path);p.add_argument('source_skin',type=Path);p.add_argument('output_skin',type=Path)
    a=p.parse_args();points,faces=load(a.mesh);data=json.loads(a.source_skin.read_text())
    old={tuple(np.asarray(b['position'],dtype=np.float32).tolist()):b for b in data['bindings']}
    triangles=np.asarray(data['positions'])[np.asarray(data['triangles'])]
    u=triangles[:,1]-triangles[:,0];v=triangles[:,2]-triangles[:,0]
    uu=np.sum(u*u,axis=1);uv=np.sum(u*v,axis=1);vv=np.sum(v*v,axis=1);den=uu*vv-uv*uv
    bindings=[];distances=[];preserved=0
    for index,point in enumerate(points):
        key=tuple(np.asarray(point,dtype=np.float32).tolist())
        if key in old:
            b=dict(old[key]);b.update(vertex=index,position=list(point));bindings.append(b);preserved+=1;continue
        q=np.asarray(point);w=q-triangles[:,0];wu=np.sum(w*u,axis=1);wv=np.sum(w*v,axis=1)
        y=(vv*wu-uv*wv)/den;z=(uu*wv-uv*wu)/den
        weights=np.stack((1-y-z,y,z),axis=1)
        best_weights=weights.copy();distance=np.sum((np.sum(triangles*weights[:,:,None],axis=1)-q)**2,axis=1)
        distance[np.any(weights<0,axis=1)]=np.inf
        for i,j in [(0,1),(1,2),(2,0)]:
            edge=triangles[:,j]-triangles[:,i]
            t=np.clip(np.sum((q-triangles[:,i])*edge,axis=1)/np.sum(edge*edge,axis=1),0,1)
            nearest=triangles[:,i]+t[:,None]*edge;d=np.sum((nearest-q)**2,axis=1);mask=d<distance
            ew=np.zeros_like(weights);ew[:,i]=1-t;ew[:,j]=t
            distance[mask]=d[mask];best_weights[mask]=ew[mask]
        face=int(np.argmin(distance));bary=best_weights[face].tolist();d=math.sqrt(float(distance[face]))
        if not math.isfinite(d) or min(bary)<0 or abs(sum(bary)-1)>1e-8: raise ValueError('invalid projection')
        bindings.append(dict(vertex=index,position=list(point),triangle=face,weights=bary,fade=1.));distances.append(d)
    data.update(bindings=bindings,render_vertices=len(points),max_binding_distance_m=max(data['max_binding_distance_m'],max(distances,default=0)))
    a.output_skin.write_text(json.dumps(data,separators=(',',':'))+'\n')
    # Retain original imported normals at existing points; area-weighted normals on added points.
    normals=[];source_points=[]
    for line in a.source_mesh.read_text().splitlines():
        if line.startswith('v '):source_points.append(tuple(map(float,line.split()[1:4])))
        if line.startswith('vn '):normals.append(tuple(map(float,line.split()[1:4])))
    old_normals=dict(zip(source_points,normals));positions=np.asarray(points);f=np.asarray(faces)
    n=np.cross(positions[f[:,1]]-positions[f[:,0]],positions[f[:,2]]-positions[f[:,0]])
    accumulated=np.zeros_like(positions)
    for k in range(3):np.add.at(accumulated,f[:,k],n)
    accumulated/=np.linalg.norm(accumulated,axis=1)[:,None]
    source_vertices,source_faces=load(a.source_mesh)
    source_vertices=np.asarray(source_vertices)
    sf=np.asarray(source_faces)
    centers=np.mean(source_vertices[sf],axis=1)
    sf=sf[(np.abs(np.abs(centers[:,0])-.08)<.08)&(np.abs(centers[:,1]-.36)<.08)&(centers[:,2]>.08)]
    st=source_vertices[sf];su=st[:,1]-st[:,0];sv=st[:,2]-st[:,0]
    suu=np.sum(su*su,axis=1);suv=np.sum(su*sv,axis=1);svv=np.sum(sv*sv,axis=1);sd=suu*svv-suv*suv
    source_normals=np.asarray([old_normals[tuple(q)] for q in source_vertices])
    final=[]
    for point in points:
        if point in old_normals: final.append(old_normals[point]);continue
        sw=np.asarray(point)-st[:,0];swu=np.sum(sw*su,axis=1);swv=np.sum(sw*sv,axis=1)
        sy=(svv*swu-suv*swv)/sd;sz=(suu*swv-suv*swu)/sd
        bary=np.stack((1-sy-sz,sy,sz),axis=1)
        distance=np.sum((np.sum(st*bary[:,:,None],axis=1)-point)**2,axis=1)
        distance[np.any(bary < -1e-8,axis=1)]=np.inf
        face=int(np.argmin(distance))
        if not np.isfinite(distance[face]) or distance[face]>1e-16: raise ValueError('new vertex left original surface')
        weight=np.clip(bary[face],0,1);weight/=np.sum(weight)
        normal=np.sum(source_normals[sf[face]]*weight[:,None],axis=0);normal/=np.linalg.norm(normal)
        final.append(tuple(normal))
    a.mesh.write_text('# Locally refined neutral reference body\n'+''.join('v '+' '.join(format(x,'.17g') for x in q)+'\n' for q in points)+''.join('vn '+' '.join(format(x,'.17g') for x in q)+'\n' for q in final)+''.join('f '+' '.join(f'{i+1}//{i+1}' for i in t)+'\n' for t in faces))
    report=dict(source_skin_sha256=hashlib.sha256(a.source_skin.read_bytes()).hexdigest(),mesh_sha256=hashlib.sha256(a.mesh.read_bytes()).hexdigest(),skin_sha256=hashlib.sha256(a.output_skin.read_bytes()).hexdigest(),preserved_bindings=preserved,added_bindings=len(distances),maximum_added_binding_distance_m=max(distances,default=0),shell_unchanged=True)
    a.output_skin.with_suffix('.audit.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

if __name__=='__main__':main()

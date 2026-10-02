#!/usr/bin/env python3
"""Transport authored vertex normals through repaired triangle surface frames."""
import argparse,hashlib,json
from pathlib import Path
import numpy as np
from prepare_body_geometry import load

def smooth_normals(points,faces):
    triangles=points[faces]
    normals=np.cross(triangles[:,1]-triangles[:,0],triangles[:,2]-triangles[:,0])
    size=np.linalg.norm(normals,axis=1)
    if np.any(size<=2e-14):raise ValueError('degenerate smoothing face')
    normals/=size[:,None]
    result=np.zeros_like(points)
    for k in range(3):
        u=triangles[:,(k+1)%3]-triangles[:,k]
        v=triangles[:,(k+2)%3]-triangles[:,k]
        angle=np.arctan2(np.linalg.norm(np.cross(u,v),axis=1),np.sum(u*v,axis=1))
        np.add.at(result,faces[:,k],normals*angle[:,None])
    size=np.linalg.norm(result,axis=1)
    if np.any(size<=1e-12) or not np.all(np.isfinite(result)):raise ValueError('undefined vertex normal')
    return result/size[:,None]

def vertex_transport(reference,positions,reference_faces,faces,authored):
    old=smooth_normals(reference,reference_faces);new=smooth_normals(positions,faces)
    return rotate_normals(old,new,authored)

def rotate_normals(old,new,authored):
    if old.shape!=new.shape or old.shape!=authored.shape or old.ndim!=2 or old.shape[1]!=3 or not all(np.all(np.isfinite(a)) for a in (old,new,authored)):raise ValueError('invalid normal frame arrays')
    if np.max(np.abs(np.linalg.norm(old,axis=1)-1))>1e-10 or np.max(np.abs(np.linalg.norm(new,axis=1)-1))>1e-10:raise ValueError('normal frames must be unit length')
    axis=np.cross(old,new);sine=np.linalg.norm(axis,axis=1);cosine=np.clip(np.sum(old*new,axis=1),-1,1)
    moving=sine>1e-14
    axis[moving]/=sine[moving,None]
    opposite=(~moving)&(cosine<0)
    for i in np.flatnonzero(opposite):
        basis=np.eye(3)[np.argmin(np.abs(old[i]))]
        axis[i]=np.cross(old[i],basis);axis[i]/=np.linalg.norm(axis[i])
    result=authored*cosine[:,None]+np.cross(axis,authored)*sine[:,None]+axis*np.sum(axis*authored,axis=1)[:,None]*(1-cosine[:,None])
    size=np.linalg.norm(result,axis=1)
    if np.any(size<=1e-12) or not np.all(np.isfinite(result)):raise ValueError('invalid vertex transport')
    return result/size[:,None]

def refined_authored(reference,original,authored,sources):
    if len(sources)!=len(reference) or len(original)!=len(authored):raise ValueError('incomplete refinement identity')
    if any(a.ndim!=2 or a.shape[1]!=3 or not np.all(np.isfinite(a)) for a in (reference,original,authored)):raise ValueError('invalid reference vectors')
    result=[]
    for target,row in zip(reference,sources):
        if not row or any(not isinstance(i,int) or i<0 or i>=len(original) or not np.isfinite(w) or w<=0 for i,w in row):raise ValueError('invalid interpolation weights')
        if abs(sum(w for _,w in row)-1)>1e-12:raise ValueError('interpolation weights do not sum to one')
        position=sum((original[i]*w for i,w in row),np.zeros(3))
        if np.max(np.abs(position-target))>1e-12:raise ValueError('refinement reference mismatch')
        normal=sum((authored[i]*w for i,w in row),np.zeros(3))
        length=np.linalg.norm(normal)
        if not np.isfinite(length) or length<=1e-12:raise ValueError('invalid interpolated normal')
        result.append(normal/length)
    return np.asarray(result)

def transport(reference,positions,faces,authored):
    def frames(points):
        t=points[faces];u=t[:,1]-t[:,0];v=t[:,2]-t[:,0]
        n=np.cross(u,v);size=np.linalg.norm(n,axis=1)
        if np.any(size<=2e-14):raise ValueError('degenerate transport frame')
        return np.stack((u,v,n/size[:,None]),axis=2)
    rest=frames(reference);current=frames(positions)
    transform=np.swapaxes(np.linalg.inv(current),1,2)@np.swapaxes(rest,1,2)
    normals=np.einsum('nij,nkj->nki',transform,authored[faces])
    normals/=np.linalg.norm(normals,axis=2)[:,:,None]
    result=np.zeros_like(positions)
    for k in range(3):
        u=reference[faces[:,(k+1)%3]]-reference[faces[:,k]]
        v=reference[faces[:,(k+2)%3]]-reference[faces[:,k]]
        angle=np.arctan2(np.linalg.norm(np.cross(u,v),axis=1),np.sum(u*v,axis=1))
        np.add.at(result,faces[:,k],normals[:,k]*angle[:,None])
    size=np.linalg.norm(result,axis=1)
    if np.any(size<=1e-12) or not np.all(np.isfinite(result)):raise ValueError('invalid transported normals')
    return result/size[:,None]

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference',type=Path);parser.add_argument('candidate',type=Path)
    parser.add_argument('output',type=Path);parser.add_argument('--report',type=Path,required=True)
    parser.add_argument('--authored-reference',type=Path)
    parser.add_argument('--vertex-sources',type=Path)
    parser.add_argument('--transport-reference-topology',action='store_true')
    parser.add_argument('--vertex-frame',action='store_true')
    parser.add_argument('--source-vertex-frames',action='store_true')
    args=parser.parse_args()
    if args.output.exists():parser.error('output already exists')
    if args.vertex_frame and args.transport_reference_topology:parser.error('choose one normal transport mode')
    if args.source_vertex_frames and not (args.vertex_frame and args.vertex_sources):parser.error('source vertex frames require vertex-frame mode and refinement correspondence')
    reference,reference_faces=load(args.reference);points,faces=load(args.candidate)
    if bool(args.authored_reference)!=bool(args.vertex_sources):parser.error('authored reference and vertex sources must be supplied together')
    authored_source=args.authored_reference or args.reference
    authored=[list(map(float,line.split()[1:4])) for line in authored_source.read_text().splitlines() if line.startswith('vn ')]
    if args.vertex_sources:
        original,original_faces=load(authored_source)
        sources=json.loads(args.vertex_sources.read_text())
        if sources['proof']['reference_sha256']!=hashlib.sha256(authored_source.read_bytes()).hexdigest():raise ValueError('authored reference provenance mismatch')
        if sources['proof']['reference_output_sha256']!=hashlib.sha256(args.reference.read_bytes()).hexdigest():raise ValueError('refined reference provenance mismatch')
        authored=refined_authored(np.asarray(reference),np.asarray(original),np.asarray(authored),sources['vertex_sources'])
    if len(reference)!=len(points) or len(authored)!=len(points):raise ValueError('vertex identity or authored normals unavailable')
    p=np.asarray(points);r=np.asarray(reference);f=np.asarray(reference_faces if args.transport_reference_topology else faces);a=np.asarray(authored)
    if args.source_vertex_frames:
        original_geometry=smooth_normals(np.asarray(original),np.asarray(original_faces))
        source_frames=refined_authored(r,np.asarray(original),original_geometry,sources['vertex_sources'])
        normals=rotate_normals(source_frames,smooth_normals(p,np.asarray(faces)),a)
        rest=rotate_normals(source_frames,source_frames,a)
    else:
        normals=vertex_transport(r,p,np.asarray(reference_faces),np.asarray(faces),a) if args.vertex_frame else transport(r,p,f,a)
        rest=vertex_transport(r,r,np.asarray(reference_faces),np.asarray(reference_faces),a) if args.vertex_frame else transport(r,r,f,a)
    normalized=a/np.linalg.norm(a,axis=1)[:,None]
    if np.max(np.abs(rest-normalized))>1e-10:raise ValueError('rest normal preservation failed')
    with args.output.open('x') as stream:
        stream.write('# Repaired candidate with transported authored normals; not adopted\n')
        for q in points:stream.write('v '+' '.join(format(x,'.17g') for x in q)+'\n')
        for q in normals:stream.write('vn '+' '.join(format(x,'.17g') for x in q)+'\n')
        for t in faces:stream.write('f '+' '.join(f'{i+1}//{i+1}' for i in t)+'\n')
    if load(args.output)!=(points,faces):raise ValueError('geometry changed during export')
    report=dict(vertices=len(points),triangles=len(faces),rest_max_component_error=float(np.max(np.abs(rest-normalized))),
        maximum_normal_change_degrees=float(np.max(np.degrees(np.arccos(np.clip(np.sum(normals*normalized,axis=1),-1,1))))),
        reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),candidate_sha256=hashlib.sha256(args.candidate.read_bytes()).hexdigest(),output_sha256=hashlib.sha256(args.output.read_bytes()).hexdigest(),geometry_preserved=True,adopted=False)
    if args.vertex_sources:report.update(authored_reference_sha256=hashlib.sha256(authored_source.read_bytes()).hexdigest(),vertex_sources_sha256=hashlib.sha256(args.vertex_sources.read_bytes()).hexdigest())
    report['transport_reference_topology']=args.transport_reference_topology
    report['vertex_frame']=args.vertex_frame
    report['source_vertex_frames']=args.source_vertex_frames
    if args.source_vertex_frames:report['rest_check_scope']='Unchanged source normal-frame rotation; target triangulation is independently reconstructed'
    geometric=smooth_normals(p,np.asarray(faces))
    agreement=np.sum(normals*geometric,axis=1)
    report['normals_opposed_to_current_geometry']=int(np.sum(agreement<0))
    report['maximum_current_geometry_angle_degrees']=float(np.max(np.degrees(np.arccos(np.clip(agreement,-1,1)))))
    args.report.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))

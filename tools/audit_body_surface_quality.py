#!/usr/bin/env python3
"""Static triangle-quality and adjacent-face-angle diagnostics, not anatomical certification."""
import argparse,collections,hashlib,json,math
from pathlib import Path
import numpy as np
from prepare_body_geometry import load

def quality(points,faces,bounds=None):
    points=np.asarray(points,dtype=float);faces=np.asarray(faces,dtype=int)
    if points.ndim!=2 or points.shape[1]!=3 or not np.all(np.isfinite(points)):raise ValueError('invalid coordinates')
    triangles=points[faces]
    edges=triangles[:,[1,2,0]]-triangles
    normal=np.cross(edges[:,0],triangles[:,2]-triangles[:,0])
    double_area=np.linalg.norm(normal,axis=1)
    denominator=np.sum(edges*edges,axis=(1,2))
    score=np.divide(2*math.sqrt(3)*double_area,denominator,out=np.zeros_like(double_area),where=denominator>0)
    centers=np.mean(triangles,axis=1)
    selected=np.ones(len(faces),dtype=bool)
    if bounds is not None:
        selected=np.all((centers>=np.array(bounds[::2]))&(centers<=np.array(bounds[1::2])),axis=1)
    if not np.any(selected):raise ValueError('empty selected region')
    incidence=collections.defaultdict(list)
    for i,t in enumerate(faces):
        for k in range(3):incidence[tuple(sorted((int(t[k]),int(t[(k+1)%3]))))].append(i)
    angles=[]
    for edge,ids in incidence.items():
        if len(ids)!=2:continue
        a,b=ids
        if not (selected[a] or selected[b]) or double_area[a]==0 or double_area[b]==0:continue
        n=normal[a]/double_area[a];m=normal[b]/double_area[b]
        angle=math.degrees(math.atan2(float(np.linalg.norm(np.cross(n,m))),float(np.dot(n,m))))
        angles.append((angle,edge,a,b))
    values=score[selected]
    return {'selected_triangles':int(np.sum(selected)),'minimum_triangle_quality':float(np.min(values)),
        'triangle_quality_percentiles':dict(zip(('p01','p05','p50'),map(float,np.percentile(values,[1,5,50])))),
        'triangles_below_quality_005':int(np.sum(values<.05)),
        'zero_area_triangles':int(np.sum(double_area[selected]==0)),
        'maximum_adjacent_face_angle_degrees':max((v[0] for v in angles),default=None),
        'largest_face_angles':[{'degrees':a,'edge':edge,'faces':[i,j]} for a,edge,i,j in sorted(angles,reverse=True)[:20]],
        'scope':'Static scale-invariant diagnostics; low triangle quality or large face angle is not by itself an anatomical defect.'}

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path);parser.add_argument('--report',type=Path,required=True)
    parser.add_argument('--bounds',nargs=6,type=float,metavar=('XMIN','XMAX','YMIN','YMAX','ZMIN','ZMAX'))
    args=parser.parse_args()
    if args.bounds is not None and (not all(math.isfinite(x) for x in args.bounds) or any(args.bounds[k]>args.bounds[k+1] for k in (0,2,4))):parser.error('invalid bounds')
    points,faces=load(args.source);report=quality(points,faces,args.bounds)
    report.update(source=str(args.source),source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),bounds_m=args.bounds)
    args.report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='largest_face_angles'}))

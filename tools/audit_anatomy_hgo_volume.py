#!/usr/bin/env python3
"""Independent signed-volume audit of committed atlas HGO tetrahedral states."""
import argparse,csv,json,math,struct
from pathlib import Path
from compare_supported_wall_time import mesh,digest
from compare_anatomy_hgo_space import profile,validate_protocol
from compare_anatomy_hgo_time import read

def determinant(points,cell):
    a,b,c,d=(points[i] for i in cell)
    u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)];w=[d[i]-a[i] for i in range(3)]
    return u[0]*(v[1]*w[2]-v[2]*w[1])-u[1]*(v[0]*w[2]-v[2]*w[0])+u[2]*(v[0]*w[1]-v[1]*w[0])

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source',type=Path,required=True);p.add_argument('--meshes',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    raw=a.source.read_bytes();magic,version,np,nc,nf=struct.unpack_from('<4s4I',raw)
    if magic!=b'VXTM' or version!=1 or len(raw)!=20+24*np+16*nc+12*nf:raise ValueError('invalid source mesh')
    points=[struct.unpack_from('<3d',raw,20+24*i) for i in range(np)]
    cells=[struct.unpack_from('<4I',raw,20+24*np+16*i) for i in range(nc)]
    level=int(profile(a.meshes/'load-profile.txt')['refinements'])
    for n in range(level):
        parents=list(csv.DictReader((a.meshes/f'vertex-parents-{n}.csv').open()));new=[];edges={}
        for i,row in enumerate(parents):
            aa,bb=int(row['parent_a']),int(row['parent_b'])
            if int(row['node'])!=i or not 0<=aa<len(points) or not 0<=bb<len(points):raise ValueError('invalid parent map')
            new.append(tuple((u+v)/2 for u,v in zip(points[aa],points[bb])))
            if aa!=bb:edges[tuple(sorted((aa,bb)))]=i
        child=[]
        for aa,b,c,d in cells:
            ab,ac,ad,bc,bd,cd=[edges[tuple(sorted(e))] for e in [(aa,b),(aa,c),(aa,d),(b,c),(b,d),(c,d)]]
            for cell in [(aa,ab,ac,ad),(b,ab,bc,bd),(c,ac,bc,cd),(d,ad,bd,cd),(ab,cd,ac,ad),(ab,cd,ad,bd),(ab,cd,bd,bc),(ab,cd,bc,ac)]:
                if determinant(new,cell)<0:cell=(cell[0],cell[2],cell[1],cell[3])
                child.append(cell)
        points,cells=new,child
    rest,faces=mesh(a.meshes/'rest.obj')
    if len(rest)!=len(points) or any(math.dist(u,v)>1e-14 for u,v in zip(rest,points)):raise ValueError('reference reconstruction differs')
    volumes=[determinant(rest,c) for c in cells]
    if any(v<=0 or not math.isfinite(v) for v in volumes):raise ValueError('invalid reference volume')
    rows,ch=read(Path(str(a.meshes)+'.csv'));validate_protocol(rows,a.meshes);states=[]
    for row in rows:
        path=a.meshes/(row['stage']+'.obj');x,f=mesh(path)
        if len(x)!=len(rest) or f!=faces:raise ValueError('state topology differs')
        jacobians=[determinant(x,c)/v for c,v in zip(cells,volumes)]
        if any(not math.isfinite(j) or j<=0 for j in jacobians):raise ValueError('inverted committed tetrahedron')
        states.append(dict(stage=row['stage'],time_s=float(row['physical_time_s']),minimum_j=min(jacobians),maximum_j=max(jacobians),sha256=digest(path)))
    report=dict(scope='Static signed-volume audit; excludes self-contact, CCD, anatomical calibration and convergence claims',nodes=len(rest),cells=len(cells),source_sha256=digest(a.source),csv_sha256=ch,minimum_j=min(s['minimum_j'] for s in states),states=states)
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['nodes','cells','minimum_j']}))
if __name__=='__main__':main()

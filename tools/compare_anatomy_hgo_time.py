#!/usr/bin/env python3
"""Compare complete atlas-HGO schedules without interpolation or mesh welding."""
import argparse,csv,hashlib,io,json,math
from pathlib import Path
from compare_supported_wall_time import mesh,digest

def read(path):
    data=path.read_bytes(); rows=list(csv.DictReader(io.StringIO(data.decode())))
    previous=0.
    for r in rows:
        t=float(r['physical_time_s']); force=float(r['force_n']); residual=float(r['residual_n'])
        if not all(math.isfinite(x) for x in [t,force,residual]) or t<=previous or force<0 or not 0<=residual<=1e-7:
            raise ValueError('invalid or unconverged physical row')
        previous=t
    if not rows:raise ValueError('empty physical schedule')
    return rows,hashlib.sha256(data).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['coarse','fine','coarse-meshes','fine-meshes','output']:
        p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args();coarse,ch=read(a.coarse);fine,fh=read(a.fine)
    ref,faces=mesh(a.coarse_meshes/'rest.obj')
    if mesh(a.fine_meshes/'rest.obj')!=(ref,faces):raise ValueError('reference geometry differs')
    hashes={}
    for name in ['material-profile.txt','force-targets.txt']:
        hashes[name]=digest(a.coarse_meshes/name)
        if hashes[name]!=digest(a.fine_meshes/name):raise ValueError('material or boundary profile differs')
    if abs(float(coarse[-1]['physical_time_s'])-float(fine[-1]['physical_time_s']))>1e-9:
        raise ValueError('incomplete or mismatched duration')
    previous=0.;comparisons=[]
    for r in coarse:
        t=float(r['physical_time_s']);force=float(r['force_n'])
        matches=[s for s in fine if abs(float(s['physical_time_s'])-t)<=1e-9]
        if len(matches)!=1:raise ValueError('missing common physical endpoint')
        interval=[s for s in fine if previous+1e-9<float(s['physical_time_s'])<=t+1e-9]
        if not interval or any(abs(float(s['force_n'])-force)>1e-12 for s in interval):raise ValueError('intermediate force differs')
        previous=t;s=matches[0]
        ap=a.coarse_meshes/(r['stage']+'.obj');bp=a.fine_meshes/(s['stage']+'.obj')
        x,xf=mesh(ap);y,yf=mesh(bp)
        if xf!=faces or yf!=faces or len(x)!=len(ref) or len(y)!=len(ref):raise ValueError('state topology differs')
        differences=[math.dist(i,j) for i,j in zip(x,y)];index=max(range(len(x)),key=differences.__getitem__)
        comparisons.append(dict(time_s=t,force_n=force,stage=r['stage'],fine_stage=s['stage'],
            max_node_difference_m=max(differences),rms_node_difference_m=math.sqrt(sum(v*v for v in differences)/len(x)),
            maximum_difference_node=index,coarse_mesh_sha256=digest(ap),fine_mesh_sha256=digest(bp)))
    report=dict(scope='Full-field temporal sensitivity of a synthetic HGO law on atlas geometry; no anatomical calibration or formal order claim',
        coarse_csv_sha256=ch,fine_csv_sha256=fh,profile_hashes=hashes,
        coarse_steps=len(coarse),fine_steps=len(fine),duration_s=previous,comparisons=comparisons,
        max_node_difference_m=max(r['max_node_difference_m'] for r in comparisons))
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'common_times':len(comparisons),'max_difference_m':report['max_node_difference_m']}))

if __name__=='__main__':main()

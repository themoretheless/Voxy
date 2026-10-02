#!/usr/bin/env python3
"""Compare one red refinement with parent prolongation and inherited supports."""
import argparse,ast,csv,json,math
from pathlib import Path
from compare_anatomy_hgo_time import read
from compare_supported_wall_time import mesh,digest

def profile(path):
    return dict(line.split('=',1) for line in path.read_text().splitlines())

def validate_protocol(rows,root):
    substeps=int((root/'time-substeps.txt').read_text())
    if not 1<=substeps<=16 or len(rows)!=6*substeps:
        raise ValueError('incomplete physical schedule')
    stages=['loaded','hold-1','hold-2','released','recover-1','recover-2']
    for i,row in enumerate(rows):
        step,sub=divmod(i,substeps);sub+=1
        expected=stages[step] if sub==substeps else f'{stages[step]}-sub-{sub}'
        t=(step+sub/substeps)*0.1;force=0.001 if step<3 else 0.0
        if row['stage']!=expected or abs(float(row['physical_time_s'])-t)>1e-8 or abs(float(row['force_n'])-force)>1e-12:
            raise ValueError('physical protocol differs from prescribed load/hold/release')
        displacement=float(row['max_displacement_m'])
        if not math.isfinite(displacement) or displacement<0:
            raise ValueError('invalid committed displacement')

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--coarse',type=Path,required=True)
    p.add_argument('--fine',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args(); c=a.coarse; f=a.fine
    cr,ch=read(Path(str(c)+'.csv'));fr,fh=read(Path(str(f)+'.csv'))
    validate_protocol(cr,c);validate_protocol(fr,f)
    x,xf=mesh(c/'rest.obj'); y,yf=mesh(f/'rest.obj')
    cl=profile(c/'load-profile.txt');fl=profile(f/'load-profile.txt')
    level=int(cl['refinements'])
    if level<0 or int(fl['refinements'])!=level+1:raise ValueError('requires adjacent refinement levels')
    parents=list(csv.DictReader((f/f'vertex-parents-{level}.csv').open()))
    if len(parents)!=len(y):raise ValueError('parent count differs')
    pairs=[]
    for i,row in enumerate(parents):
        aa,bb=int(row['parent_a']),int(row['parent_b'])
        if int(row['node'])!=i or not 0<=aa<len(x) or not 0<=bb<len(x):raise ValueError('invalid parent')
        if math.dist(y[i],tuple((u+v)/2 for u,v in zip(x[aa],x[bb])))>1e-14:raise ValueError('reference transfer differs')
        pairs.append((aa,bb))
    cp=profile(c/'clamp-mode.txt');fp=profile(f/'clamp-mode.txt')
    if cp['inherited_clamp']!='true' or fp['inherited_clamp']!='true':raise ValueError('supports not inherited')
    pins=set(ast.literal_eval(cp['pins']));finepins=set(ast.literal_eval(fp['pins']))
    if finepins!={i for i,(aa,bb) in enumerate(pairs) if aa in pins and bb in pins}:raise ValueError('support transfer differs')
    if digest(c/'material-profile.txt')!=digest(f/'material-profile.txt'):raise ValueError('material differs')
    cl=profile(c/'load-profile.txt');fl=profile(f/'load-profile.txt')
    if cl['surface_patch']!='true' or fl['surface_patch']!='true':raise ValueError('surface load required')
    oldpatch=ast.literal_eval(cl['patch_faces']);newpatch=ast.literal_eval(fl['patch_faces'])
    if newpatch!=[4*i+k for i in oldpatch for k in range(4)]:raise ValueError('reference patch descendants differ')
    moments=[]
    for points,lp in [(x,cl),(y,fl)]:
        weights=ast.literal_eval(lp['weights'])
        if len(weights)!=len(points):raise ValueError('load weight count')
        moments.append([sum(weights)]+[sum(w*v[k] for w,v in zip(weights,points)) for k in range(3)])
    if any(abs(u-v)>1e-16 for u,v in zip(*moments)):raise ValueError('load area or first moment differs')
    if len(cr)!=len(fr):raise ValueError('schedule differs')
    if abs(float(cr[-1]['physical_time_s'])-0.6)>1e-9 or cr[-1]['stage']!='recover-2' or fr[-1]['stage']!='recover-2':raise ValueError('incomplete protocol')
    if len(yf)!=4*len(xf):raise ValueError('requires exactly one red refinement')
    comparisons=[]
    for r,s in zip(cr,fr):
        if any(abs(float(r[k])-float(s[k]))>1e-12 for k in ['physical_time_s','force_n']):raise ValueError('physical schedule differs')
        ap=c/(r['stage']+'.obj');bp=f/(s['stage']+'.obj');xx,ff=mesh(ap);yy,gg=mesh(bp)
        if ff!=xf or gg!=yf or len(xx)!=len(x) or len(yy)!=len(y):raise ValueError('state topology differs')
        u=[tuple(a-b for a,b in zip(v,ref)) for v,ref in zip(xx,x)]
        du=[]
        for i,(aa,bb) in enumerate(pairs):
            prolonged=tuple((v+w)/2 for v,w in zip(u[aa],u[bb]))
            if i in finepins and math.dist(prolonged,(0,0,0))>1e-14:raise ValueError('inadmissible prolongation')
            du.append(math.dist(tuple(v-w for v,w in zip(yy[i],y[i])),prolonged))
        comparisons.append(dict(time_s=float(r['physical_time_s']),stage=r['stage'],max_difference_m=max(du),rms_node_difference_m=math.sqrt(sum(v*v for v in du)/len(du)),maximum_node=du.index(max(du)),coarse_mesh_sha256=digest(ap),fine_mesh_sha256=digest(bp)))
    report=dict(scope='Two-level spatial sensitivity with exact parent prolongation; synthetic material, no formal convergence or anatomical calibration claim',coarse_refinement=level,fine_refinement=level+1,coarse_nodes=len(x),fine_nodes=len(y),coarse_pins=len(pins),fine_pins=len(finepins),coarse_csv_sha256=ch,fine_csv_sha256=fh,load_area_and_first_moment=moments,comparisons=comparisons,max_difference_m=max(r['max_difference_m'] for r in comparisons))
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['coarse_nodes','fine_nodes','coarse_pins','fine_pins','max_difference_m']}))
if __name__=='__main__':main()

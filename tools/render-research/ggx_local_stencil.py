"""Dyadic local refinement diagnostic; GPU integration, CPU interpolation.

No lookup texture or production admission is implied. Shared dyadic axes are
selected from distance to physical boundaries, not placed at target queries.
"""
import argparse
import json
import math
import struct
from pathlib import Path
from ggx_fresnel_quintic import lagrange

CASES=[(1.,.005),(1.,.0001),(.999,.01),(.001,.005),(.005,.05),(.01,.1),(.05,.005),(.05,.25)]

def f32(x):return struct.unpack('<f',struct.pack('<f',x))[0]
def key(n,r):return struct.pack('<ff',n,r).hex()

def axis(value, refinement, positive=False):
    distances=[v for v in (value,1-value) if v>0]
    scale=2.**math.floor(math.log2(min(distances)))
    step=min(1/64,scale/8)/refinement
    origin=min(max(math.floor(value/step)-2,int(positive)),round(1/step)-5)
    return [f32((origin+i)*step) for i in range(6)]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    sub=p.add_subparsers(dest='mode',required=True)
    g=sub.add_parser('generate');g.add_argument('prefix',type=Path);g.add_argument('--refinement',type=int,choices=[1,2,4,8],default=1)
    c=sub.add_parser('compare');c.add_argument('manifest',type=Path);c.add_argument('gpu',type=Path);c.add_argument('reference',type=Path)
    g.add_argument('--cell-probes',action='store_true')
    a=p.parse_args()
    if a.mode=='generate':
        cases=[];nodes={}
        targets=CASES
        if a.cell_probes:
            targets=[]
            for nv,r in CASES:
                nv,r=f32(nv),f32(r)
                xs,ys=axis(nv,1),axis(r,1,True)
                dx,dy=xs[1]-xs[0],ys[1]-ys[0]
                left=min(math.floor(nv/dx)*dx,1-dx)
                bottom=min(math.floor(r/dy)*dy,1-dy)
                for fy in [.125,.5,.875]:
                    for fx in [.125,.5,.875]:targets.append((f32(left+fx*dx),f32(bottom+fy*dy)))
        for nv,r in targets:
            nv,r=f32(nv),f32(r);xs,ys=axis(nv,a.refinement),axis(r,a.refinement,True)
            cases.append(dict(nv=nv,roughness=r,xs=xs,ys=ys))
            for y in ys:
                for x in xs:nodes[key(x,y)]=(x,y)
        assert len({key(c['nv'],c['roughness']) for c in cases})==len(cases)
        a.prefix.with_suffix('.probes.txt').write_text(''.join(f"{c['nv']:.17g} {c['roughness']:.17g}\n" for c in cases))
        a.prefix.with_suffix('.txt').write_text(''.join(f'{x:.17g} {y:.17g}\n' for x,y in nodes.values()))
        a.prefix.with_suffix('.json').write_text(json.dumps(dict(refinement=a.refinement,node_count=len(nodes),cases=cases),indent=2)+'\n')
        print(json.dumps(dict(node_count=len(nodes),cases=len(cases),refinement=a.refinement)))
        return
    manifest=json.loads(a.manifest.read_text())
    gpu=[json.loads(l) for l in a.gpu.read_text().splitlines()];nodes={key(r['nv'],r['roughness']):r for r in gpu if 'nv'in r}
    assert len(nodes)==manifest['node_count']
    refs=[json.loads(l) for l in a.reference.read_text().splitlines()];refs={key(r['nv'],r['roughness']):r for r in refs if 'nv'in r}
    results=[]
    for case in manifest['cases']:
        nv,r,xs,ys=[case[k] for k in ('nv','roughness','xs','ys')];wx,wy=lagrange(xs,nv),lagrange(ys,r)
        loss_coefficient=math.fsum(wx[x]*wy[y]*nodes[key(xs[x],ys[y])]['unit_loss']/ys[y]**4 for y in range(6) for x in range(6))
        bias_coefficient=math.fsum(wx[x]*wy[y]*(nodes[key(xs[x],ys[y])]['B']-(1-xs[x])**5)/ys[y]**4 for y in range(6) for x in range(6))
        a_value=math.fsum(wx[x]*wy[y]*nodes[key(xs[x],ys[y])]['A'] for y in range(6) for x in range(6))
        loss=r**4*loss_coefficient;b=(1-nv)**5+r**4*bias_coefficient
        # Preserve the smaller directly interpolated Fresnel channel. Never
        # subtract a nearly unit B to obtain a tiny A at grazing incidence.
        if a_value<=b:b=(1-loss)-a_value
        else:a_value=(1-loss)-b
        values={'A':a_value,'B':b,'loss':loss}
        reference=refs[key(nv,r)]['reference'];errors={c:abs(v-reference[c])/reference[c] if reference[c] else abs(v) for c,v in values.items()}
        results.append(dict(nv=nv,roughness=r,values=values,relative_errors=errors,passed=all(0<=v<=1 for v in values.values()) and max(errors.values())<1e-4))
    print(json.dumps(dict(scope='GPU moment producer with CPU local interpolation; GPU lookup not qualified',refinement=manifest['refinement'],node_count=len(nodes),cases=len(results),failures=sum(not r['passed'] for r in results),relative_threshold=1e-4,results=results),indent=2))

if __name__=='__main__':main()

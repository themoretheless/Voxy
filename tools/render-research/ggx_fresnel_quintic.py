"""Diagnostic six-node physical-coordinate Fresnel interpolation.

Uses the actual square-view GPU table. No epsilon for zero channels and no
range clamp; invalid or inaccurate interpolation is explicitly rejected.
"""
import argparse
import json
import math
from pathlib import Path


def lagrange(nodes, position):
    return [math.prod((position-other)/(node-other) for j,other in enumerate(nodes) if j!=i)
            for i,node in enumerate(nodes)]


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('table',type=Path)
    p.add_argument('reference',type=Path)
    p.add_argument('--smooth-residual',action='store_true')
    args=p.parse_args()
    rows=[json.loads(l) for l in args.table.read_text().splitlines()]
    cells=[r for r in rows if 'nv'in r and not r.get('interpolated')]
    n=math.isqrt(len(cells));assert n*n==len(cells) and n>=6
    table={}
    for r in cells:
        key=round(math.sqrt(r['nv'])*(n-1)),round(r['roughness']*(n-1))
        assert key not in table
        table[key]=r
    results=[]
    for ref in [json.loads(l) for l in args.reference.read_text().splitlines()]:
        nv,roughness=ref['nv'],ref['roughness']
        ox,oy=[min(max(math.floor(v)-2,0),n-6) for v in (math.sqrt(nv)*(n-1),roughness*(n-1))]
        if args.smooth_residual:oy=max(1,oy)
        # Use the actual producer-rounded node coordinates, not idealized f64 nodes.
        wx=lagrange([table[ox+x,oy]['nv'] for x in range(6)],nv)
        wy=lagrange([table[ox,oy+y]['roughness'] for y in range(6)],roughness)
        if args.smooth_residual:
            coefficient=math.fsum(wx[x]*wy[y]*(table[ox+x,oy+y]['B']-(1-table[ox+x,oy+y]['nv'])**5)
                                  /table[ox+x,oy+y]['roughness']**4 for y in range(6) for x in range(6))
            b=(1-nv)**5+roughness**4*coefficient
        else:
            b=math.fsum(wx[x]*wy[y]*table[ox+x,oy+y]['B'] for y in range(6) for x in range(6))
        expected=ref['reference']['B'];error=abs(b-expected)/expected if expected else abs(b)
        results.append(dict(nv=nv,roughness=roughness,B=b,reference_B=expected,
                            relative_B_error=error,passed=0<=b<=1 and error<=1e-4))
    print(json.dumps(dict(scope='CPU physical-coordinate quintic diagnostic on actual GPU nodes',
                          relative_threshold=1e-4,smooth_residual=args.smooth_residual,cases=len(results),failures=sum(not r['passed'] for r in results),
                          results=results),indent=2))


if __name__=='__main__':main()

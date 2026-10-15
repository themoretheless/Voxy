"""Compare raw and logarithmic B interpolation on a square-view GPU table.

Diagnostic only. Zero-valued nodes make logarithmic interpolation undefined;
those cases are reported as rejected, never replaced by an epsilon.
"""
import argparse
import json
import math
from pathlib import Path
from ggx_loss_representation import weights


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('table',type=Path)
    p.add_argument('reference',type=Path)
    args=p.parse_args()
    rows=[json.loads(l) for l in args.table.read_text().splitlines()]
    cells=[r for r in rows if 'nv' in r and not r.get('interpolated')]
    n=math.isqrt(len(cells));assert n*n==len(cells) and n>=4
    table={}
    for r in cells:
        key=(round(math.sqrt(r['nv'])*(n-1)),round(r['roughness']*(n-1)))
        assert key not in table
        table[key]=r
    results=[]
    for ref in [json.loads(l) for l in args.reference.read_text().splitlines()]:
        px,py=math.sqrt(ref['nv'])*(n-1),ref['roughness']*(n-1)
        ox,oy=[min(max(math.floor(v)-1,0),n-4) for v in (px,py)]
        wx,wy=weights(px-ox-1),weights(py-oy-1)
        nodes=[(wx[x]*wy[y],table[ox+x,oy+y]['B']) for y in range(4) for x in range(4)]
        expected=ref['reference']['B']
        raw=math.fsum(w*b for w,b in nodes)
        log_value=math.exp(math.fsum(w*math.log(b) for w,b in nodes)) if all(b>0 for _,b in nodes) else None
        error=lambda b:abs(b-expected)/expected if expected else abs(b)
        results.append(dict(nv=ref['nv'],roughness=ref['roughness'],reference_B=expected,
                            raw_relative_error=error(raw),log_relative_error=error(log_value) if log_value is not None else None,
                            log_rejected_zero_node=log_value is None))
    print(json.dumps(dict(scope='CPU interpolation diagnostic; no GPU or production admission',
                          relative_threshold=1e-4,results=results),indent=2))


if __name__=='__main__':main()

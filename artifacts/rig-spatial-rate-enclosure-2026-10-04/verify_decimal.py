"""Independent exact-input Bernstein moving-pivot speed/rate reference."""
from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys

def polynomial_bounds(raw,h):
    c=[list(map(D.from_float,v)) for v in raw]
    a=max(sum(abs(x) for x in v) for v in c)
    first=max(sum(abs(3*(c[i+1][j]-c[i][j])/h) for j in range(3)) for i in range(3))
    second=max(sum(abs(6*(c[i+2][j]-2*c[i+1][j]+c[i][j])/(h*h)) for j in range(3)) for i in range(2))
    return a,first,second
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('SPATIAL_RATE_ENCLOSURE '):continue
        additive,pivot,start,end,angular,bounds=ast.literal_eval(line.removeprefix('SPATIAL_RATE_ENCLOSURE '))
        h=D.from_float(end)-D.from_float(start);a,a1,a2=polynomial_bounds(additive,h);p,p1,p2=polynomial_bounds(pivot,h)
        omega,alpha=map(D.from_float,angular)
        speed=a1+omega*a+p1;rate=a2+alpha*a+omega*(a1+p1)+p2
        assert D.from_float(bounds[0])>=speed,(speed,bounds)
        assert D.from_float(bounds[2])>=rate,(rate,bounds)
        assert D.from_float(bounds[1])>=omega
        assert D.from_float(bounds[3])>=alpha
        rows.append({'start':start,'end':end,'speed_reference':str(speed),'rate_reference':str(rate)})
assert rows and len(rows)%8==0
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

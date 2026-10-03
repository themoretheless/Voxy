"""Exact-input high-precision Bernstein angular bound reference."""
from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('ANGULAR_RATE_ENCLOSURE '):continue
        control,start,end,bounds=ast.literal_eval(line.removeprefix('ANGULAR_RATE_ENCLOSURE '))
        c=[list(map(D.from_float,v)) for v in control];h=D.from_float(end)-D.from_float(start)
        nearest=[]
        for j in range(4):
            lo=min(v[j] for v in c);hi=max(v[j] for v in c)
            nearest.append(D(0) if lo<=0<=hi else min(abs(lo),abs(hi)))
        minimum=sum(v*v for v in nearest).sqrt();assert minimum>0
        first=max(sum(abs(3*(c[i+1][j]-c[i][j])/h) for j in range(4)) for i in range(3))
        second=max(sum(abs(6*(c[i+2][j]-2*c[i+1][j]+c[i][j])/(h*h)) for j in range(4)) for i in range(2))
        speed=2*first/minimum;acceleration=2*second/minimum+4*(first/minimum)**2
        assert D.from_float(bounds[0])>=speed,(speed,bounds)
        assert D.from_float(bounds[1])>=acceleration,(acceleration,bounds)
        rows.append({'start':start,'end':end,'speed_reference':str(speed),'acceleration_reference':str(acceleration)})
assert rows
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

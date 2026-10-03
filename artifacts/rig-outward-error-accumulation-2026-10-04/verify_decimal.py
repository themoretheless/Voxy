"""Independent exact-input Decimal evaluation of cumulative error inequality."""
from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    origin=angular=D(0)
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('ERROR_ACCUMULATION '):continue
        start,end,prefix,velocity,rates,errors,bounds=ast.literal_eval(line.removeprefix('ERROR_ACCUMULATION '))
        h=D.from_float(end)-D.from_float(start)
        r=sum(abs(D.from_float(x)) for x in prefix);v=sum(abs(D.from_float(x)) for x in velocity)
        lv,lw=map(D.from_float,rates);ev,ew=map(D.from_float,errors)
        origin+=h*h*(lv+lw*r)/2+lw*v*h*h*h/3+ev*h+ew*(r*h+v*h*h/2)
        angular+=lw*h*h/2+ew*h
        assert D.from_float(bounds[0])>=origin,(origin,bounds)
        assert D.from_float(bounds[1])>=angular,(angular,bounds)
        rows.append({'start':start,'end':end,'origin_reference':str(origin),'angular_reference':str(angular)})
assert len(rows)==3
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

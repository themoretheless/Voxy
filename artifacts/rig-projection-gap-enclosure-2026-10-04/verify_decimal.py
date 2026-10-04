from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
 ctx.prec=400
 for line in Path(sys.argv[1]).read_text().splitlines():
  if not line.startswith('GAP_ENCLOSURE '):continue
  center,edges,origin,obstacle,axis,clearance,lower=ast.literal_eval(line.split(' ',1)[1])
  n=list(map(D.from_float,axis));norm=sum(v*v for v in n).sqrt()
  relative=[D.from_float(c)-D.from_float(o) for c,o in zip(center,origin)]
  gap=abs(sum(c*v for c,v in zip(relative,n)))
  for edge in edges+obstacle:gap-=abs(sum(D.from_float(c)*v for c,v in zip(edge,n)))
  reference=gap/norm-D.from_float(clearance)
  assert D(0)<D.from_float(lower)<=reference,(lower,reference)
  rows.append({'axis':axis,'clearance':clearance,'lower':lower,'reference':str(reference)})
assert len(rows)==9,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

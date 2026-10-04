from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
 ctx.prec=400
 for line in Path(sys.argv[1]).read_text().splitlines():
  if not line.startswith('ADVANCE_TIME_ENCLOSURE '):continue
  time,gap,speed,next_time=ast.literal_eval(line.split(' ',1)[1])
  reference=min(D(1),D.from_float(time)+D.from_float(0.8)*D.from_float(gap)/D.from_float(speed))
  assert D.from_float(time)<=D.from_float(next_time)<=reference,(time,gap,speed,next_time,reference)
  rows.append({'time':time,'gap':gap,'speed':speed,'next_lower':next_time,'reference':str(reference)})
assert len(rows)==6,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

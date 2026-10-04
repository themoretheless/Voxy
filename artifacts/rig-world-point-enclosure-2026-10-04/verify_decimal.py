from decimal import Decimal as D, localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
 ctx.prec=400
 for line in Path(sys.argv[1]).read_text().splitlines():
  if not line.startswith('WORLD_POINT_ENCLOSURE '):continue
  angle,point,scale,origin,world,bound=ast.literal_eval(line.split(' ',1)[1])
  a=D.from_float(angle);r=sum(D.from_float(v)**2 for v in point).sqrt()
  if angle>=3.141592653589793: chord=D(2)
  else:
   z=a/2;term=z;sine=z
   for n in range(1,201):
    term=-term*z*z/D((2*n)*(2*n+1));sine+=term
   chord=2*sine
  reference=abs(D.from_float(scale))*(D.from_float(origin)+chord*r)+D.from_float(world)
  assert D.from_float(bound)>=reference,(angle,point,scale,reference,bound)
  rows.append({'angle':angle,'point':point,'scale':scale,'reference':str(reference),'upper':bound})
assert len(rows)==96,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

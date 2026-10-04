from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
rows=[]
with localcontext() as ctx:
 ctx.prec=400
 for line in Path(sys.argv[1]).read_text().splitlines():
  if not line.startswith('INVERSE_POINT_ENCLOSURE '):continue
  points,origin,q,scale,bounds,error=ast.literal_eval(line.split(' ',1)[1])
  q=list(map(D.from_float,q));norm=sum(v*v for v in q).sqrt();x,y,z,w=[v/norm for v in q]
  matrix=[[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],
          [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],
          [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]]
  p=[sum(D.from_float(v[i]) for v in points)-D.from_float(origin[i]) for i in range(3)]
  mapped=[sum(matrix[j][i]*p[j] for j in range(3))/D.from_float(scale) for i in range(3)]
  for ref,box in zip(mapped,bounds): assert D.from_float(box[0])<=ref<=D.from_float(box[1]),(ref,box)
  a=D.from_float(0.2)/2;term=a;sine=a
  for n in range(1,201):term=-term*a*a/D((2*n)*(2*n+1));sine+=term
  reference=abs(D.from_float(scale))*(D.from_float(0.01)+2*sine*sum(v*v for v in mapped).sqrt())+D.from_float(0.003)
  assert D.from_float(error)>=reference,(reference,error)
  rows.append({'scale':scale,'point_reference':[str(v) for v in mapped],'clearance_reference':str(reference),'clearance_upper':error})
assert len(rows)==24,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

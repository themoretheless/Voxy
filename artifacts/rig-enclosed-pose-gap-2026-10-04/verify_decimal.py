from decimal import Decimal as D,localcontext
from pathlib import Path
import ast,json,sys
images=[];gaps=[]
with localcontext() as ctx:
 ctx.prec=400
 for filename in sys.argv[1:]:
  for line in Path(filename).read_text().splitlines():
   if line.startswith('POINT_BOX_IMAGE '):
    box,t,q,s,image=ast.literal_eval(line.split(' ',1)[1]);q=list(map(D.from_float,q));n=sum(v*v for v in q).sqrt();x,y,z,w=[v/n for v in q]
    m=[[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)], [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)], [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]]
    for corner in range(8):
     p=[D.from_float(box[i][bool(corner&(1<<i))]) for i in range(3)]
     mapped=[D.from_float(t[i])+D.from_float(s)*sum(m[i][j]*p[j] for j in range(3)) for i in range(3)]
     for value,b in zip(mapped,image):assert D.from_float(b[0])<=value<=D.from_float(b[1]),(value,b)
     images.append({'scale':s,'corner':corner,'reference':[str(v) for v in mapped]})
   if line.startswith('POINT_GAP_ENCLOSURE '):
    points,c,edges,axis,clearance,lower=ast.literal_eval(line.split(' ',1)[1]);axis=list(map(D.from_float,axis));norm=sum(v*v for v in axis).sqrt()
    lo=min(sum(D.from_float(p[i][int(axis[i]<0)])*axis[i] for i in range(3)) for p in points)
    hi=max(sum(D.from_float(p[i][int(axis[i]>=0)])*axis[i] for i in range(3)) for p in points)
    center=sum(D.from_float(c[i])*axis[i] for i in range(3))
    support=sum(abs(sum(D.from_float(e[i])*axis[i] for i in range(3))) for e in edges)
    reference=max(lo-center-support,center-hi-support)/norm-D.from_float(clearance)
    assert D(0)<D.from_float(lower)<=reference,(lower,reference)
    gaps.append({'axis':[str(v) for v in axis],'lower':lower,'reference':str(reference)})
assert len(images)==32 and len(gaps)==3,(len(images),len(gaps))
print(json.dumps({'passed':True,'decimal_precision':400,'point_images':images,'point_gaps':gaps},indent=2))

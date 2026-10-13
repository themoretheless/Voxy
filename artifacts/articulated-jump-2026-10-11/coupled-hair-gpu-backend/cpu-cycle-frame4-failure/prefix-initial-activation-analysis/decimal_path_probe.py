"""80-digit diagnostic minimum search; sampling/minimization is NOT a certificate."""
from decimal import Decimal as D, getcontext
from pathlib import Path
import json
getcontext().prec=80
root=Path(__file__).resolve().parent
raw=json.loads((root/'path-counterexample.json').read_text())
convert=lambda rows:[[D.from_float(v) for v in p] for p in rows]
a,b=convert(raw['capsule_start']),convert(raw['capsule_end'])
c,d=convert(raw['triangle_start']),convert(raw['triangle_end'])
radius=D.from_float(raw['radius'])
add=lambda a,b:[x+y for x,y in zip(a,b)]
sub=lambda a,b:[x-y for x,y in zip(a,b)]
mul=lambda a,t:[x*t for x in a]
dot=lambda a,b:sum((x*y for x,y in zip(a,b)),D(0))
cross=lambda a,b:[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
clamp=lambda x:max(D(0),min(D(1),x))
def point_segment(p,a,b):
 e=sub(b,a);den=dot(e,e)
 return add(a,mul(e,clamp(dot(sub(p,a),e)/den) if den else D(0)))
def inside(p,a,b,c):
 n=cross(sub(b,a),sub(c,a))
 return all(dot(cross(sub(v,u),sub(p,u)),n)>=0 for u,v in [(a,b),(b,c),(c,a)])
def point_triangle(p,a,b,c):
 n=cross(sub(b,a),sub(c,a));den=dot(n,n)
 if den:
  q=sub(p,mul(n,dot(sub(p,a),n)/den))
  if inside(q,a,b,c):return q
 return min([point_segment(p,u,v) for u,v in [(a,b),(b,c),(c,a)]],key=lambda q:dot(sub(p,q),sub(p,q)))
def pair(p,q,a,b):
 u=sub(q,p);v=sub(b,a);w=sub(p,a)
 aa,bb,cc,dd,ee=dot(u,u),dot(u,v),dot(v,v),dot(u,w),dot(v,w)
 candidates=[]
 den=aa*cc-bb*bb
 if den:
  s=(bb*ee-cc*dd)/den;t=(aa*ee-bb*dd)/den
  if 0<=s<=1 and 0<=t<=1:candidates.append((add(p,mul(u,s)),add(a,mul(v,t))))
 candidates.extend([(p,point_segment(p,a,b)),(q,point_segment(q,a,b)),(point_segment(a,p,q),a),(point_segment(b,p,q),b)])
 return min(candidates,key=lambda z:dot(sub(*z),sub(*z)))
def closest_at(t):
 lerp=lambda p,q:add(p,mul(sub(q,p),t))
 p,q=[lerp(u,v) for u,v in zip(a,b)];x,y,z=[lerp(u,v) for u,v in zip(c,d)]
 n=cross(sub(y,x),sub(z,x));den=dot(sub(q,p),n)
 if den:
  u=dot(sub(x,p),n)/den
  if 0<=u<=1 and inside(add(p,mul(sub(q,p),u)),x,y,z):
   hit=add(p,mul(sub(q,p),u));return hit,hit
 candidates=[(p,point_triangle(p,x,y,z)),(q,point_triangle(q,x,y,z))]
 candidates.extend(pair(p,q,u,v) for u,v in [(x,y),(y,z),(z,x)])
 return min(candidates,key=lambda v:dot(sub(*v),sub(*v)))
def gap(t):
 p,q=closest_at(t)
 return dot(sub(p,q),sub(p,q)).sqrt()-radius
if __name__=="__main__":
 samples=[(D(i)/256,gap(D(i)/256)) for i in range(257)]
 best=min(samples,key=lambda x:x[1]);ratio=(D(5).sqrt()-1)/2
 for i in range(257):
  if (i and samples[i][1]>samples[i-1][1]) or (i<256 and samples[i][1]>samples[i+1][1]):continue
  lo,hi=samples[max(0,i-1)][0],samples[min(256,i+1)][0]
  l=hi-ratio*(hi-lo);h=lo+ratio*(hi-lo);gl,gh=gap(l),gap(h)
  for _ in range(160):
   best=min(best,(l,gl),(h,gh),key=lambda x:x[1])
   if gl<=gh:hi=h;h=l;gh=gl;l=hi-ratio*(hi-lo);gl=gap(l)
   else:lo=l;l=h;gl=gh;h=lo+ratio*(hi-lo);gh=gap(h)
 report={'precision_decimal_digits':80,'start_gap_m':str(samples[0][1]),'end_gap_m':str(samples[-1][1]),'minimum_time_probe':str(best[0]),'minimum_gap_probe_m':str(best[1]),'clearance_above_negative_1e_10_m':str(best[1]+D('1e-10')),'scope':'Independent decimal feature-distance sampling and local minimization; not exhaustive trajectory certificate or admission'}
 print(json.dumps(report,indent=2));(root/'decimal-path-probe.json').write_text(json.dumps(report,indent=2)+'\n')

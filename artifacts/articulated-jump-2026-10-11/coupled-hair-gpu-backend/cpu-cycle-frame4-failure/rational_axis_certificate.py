"""Exact rational support-plane certificate, diagnostic reference for one primitive."""
from decimal_path_probe import closest_at, D, raw, root
from fractions import Fraction as F
import json, hashlib, time
began=time.monotonic()
vec=lambda rows:[[F.from_float(v) for v in p] for p in rows]
a,b=vec(raw['capsule_start']),vec(raw['capsule_end'])
c,d=vec(raw['triangle_start']),vec(raw['triangle_end'])
r=F.from_float(raw['radius'])-F.from_float(1e-10)
assert r>0

def proves(lo,hi,axis):
 norm=sum(v*v for v in axis)
 if norm<=0:return False
 required=r*r*norm
 for t in [lo,hi]:
  for i in range(2):
   for j in range(3):
    # ORIGINAL relative affine motion, exact dyadic arithmetic.
    projection=sum(((a[i][k]-c[j][k])+t*((b[i][k]-d[j][k])-(a[i][k]-c[j][k])))*axis[k] for k in range(3))
    if projection<0 or projection*projection<required:return False
 return True

stack=[(F(0),F(1),0)];leaves=[];visited=0;max_depth=0
while stack:
 lo,hi,depth=stack.pop();visited+=1;max_depth=max(max_depth,depth)
 assert visited<=200000 and depth<=60,'certificate budget exhausted; no admission'
 mid=(lo+hi)/2
 p,q=closest_at(D(mid.numerator)/D(mid.denominator))
 floats=[float(x-y) for x,y in zip(p,q)]
 axis=[F.from_float(v) for v in floats]
 if proves(lo,hi,axis):
  leaves.append({'lo':[lo.numerator,lo.denominator],'hi':[hi.numerator,hi.denominator],'axis_hex':[v.hex() for v in floats]})
 else:stack.extend([(mid,hi,depth+1),(lo,mid,depth+1)])
# Recheck serialized witnesses independently; require exact contiguous coverage.
leaves.sort(key=lambda v:F(*v['lo']))
end=F(0)
for leaf in leaves:
 lo,hi=F(*leaf['lo']),F(*leaf['hi'])
 assert lo==end and hi>lo
 assert proves(lo,hi,[F.from_float(float.fromhex(v)) for v in leaf['axis_hex']])
 end=hi
assert end==1
report={'status':'exact support-plane certificate verified','primitive_sha256':hashlib.sha256((root/'path-counterexample.json').read_bytes()).hexdigest(),'tolerance_hex':(1e-10).hex(),'visited_intervals':visited,'max_depth':max_depth,'leaves':leaves,'elapsed_seconds':time.monotonic()-began,'scope':'Entire exact affine capsule/triangle trajectory at original radius minus original binary64 tolerance. Independent exact rational check of serialized supporting axes and contiguous [0,1] coverage. Not engine implementation, closed-volume containment, complete groom, tangent convergence, frame, or FPS qualification.'}
(root/'rational-axis-certificate.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='leaves'},indent=2))

"""Exact rational Bernstein proof for an arbitrary moving supporting axis."""
from decimal_path_probe import closest_at, D, raw, root
from fractions import Fraction as F
from math import comb
import json,hashlib,time
began=time.monotonic()
vec=lambda rows:[[F.from_float(v) for v in p] for p in rows]
a,b=vec(raw['capsule_start']),vec(raw['capsule_end'])
c,d=vec(raw['triangle_start']),vec(raw['triangle_end'])
r=F.from_float(raw['radius'])-F.from_float(1e-10)
assert r>0

def plus(a,b):return [(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))]
def product(a,b):
 out=[F(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b):out[i+j]+=x*y
 return out
def nonnegative(p,degree,strict=False):
 for i in range(degree+1):
  value=sum((p[j] if j<len(p) else F(0))*F(comb(i,j),comb(degree,j)) for j in range(i+1))
  if value<0 or (strict and value==0):return False
 return True

def proves(lo,hi,axis):
 norm=[F(0)]
 for component in axis:norm=plus(norm,product(component,component))
 if not nonnegative(norm,4,True):return False
 for i in range(2):
  for j in range(3):
   projection=[F(0)]
   for k in range(3):
    first=a[i][k]-c[j][k];slope=(b[i][k]-d[j][k])-first
    relative=[first+lo*slope,(hi-lo)*slope]
    projection=plus(projection,product(relative,axis[k]))
   if not nonnegative(projection,3):return False
   clearance=plus(product(projection,projection),[-v*r*r for v in norm])
   if not nonnegative(clearance,6):return False
 return True

def axis_at(t):
 p,q=closest_at(D(t.numerator)/D(t.denominator))
 return [F.from_float(float(x-y)) for x,y in zip(p,q)]
stack=[(F(0),F(1),0)];leaves=[];visited=0;max_depth=0
while stack:
 lo,hi,depth=stack.pop();visited+=1;max_depth=max(max_depth,depth)
 assert visited<=20000 and depth<=60,'certificate budget exhausted; no admission'
 mid=(lo+hi)/2
 first,middle,last=axis_at(lo),axis_at(mid),axis_at(hi)
 axis=[[first[k],4*middle[k]-3*first[k]-last[k],2*first[k]-4*middle[k]+2*last[k]] for k in range(3)]
 if proves(lo,hi,axis):leaves.append({'lo':[lo.numerator,lo.denominator],'hi':[hi.numerator,hi.denominator],'axis':[[[v.numerator,v.denominator] for v in p] for p in axis]})
 else:stack.extend([(mid,hi,depth+1),(lo,mid,depth+1)])
 if visited%100==0:
  (root/'rational-polynomial-live.json').write_text(json.dumps({'visited':visited,'proven_intervals':len(leaves),'pending_intervals':len(stack),'max_depth':max_depth,'elapsed_seconds':time.monotonic()-began})+'\n')
# Recheck only serialized witnesses, not closest-point construction.
leaves.sort(key=lambda v:F(*v['lo']));end=F(0)
for leaf in leaves:
 lo,hi=F(*leaf['lo']),F(*leaf['hi'])
 assert lo==end and hi>lo
 assert proves(lo,hi,[[F(*v) for v in p] for p in leaf['axis']])
 end=hi
assert end==1
report={'status':'exact moving-axis polynomial certificate verified','primitive_sha256':hashlib.sha256((root/'path-counterexample.json').read_bytes()).hexdigest(),'tolerance_hex':(1e-10).hex(),'visited_intervals':visited,'max_depth':max_depth,'leaves':leaves,'elapsed_seconds':time.monotonic()-began,'scope':'Entire exact affine capsule/triangle trajectory, original radius minus original tolerance; exact rational Bernstein projection/norm/clearance proofs and contiguous serialized interval coverage. Not production implementation, whole groom, volume containment, tangent completion, frame or FPS proof.'}
(root/'rational-polynomial-certificate.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='leaves'},indent=2))

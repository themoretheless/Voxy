"""Independent Bernstein-basis verifier; does not use builder power polynomials."""
from pathlib import Path
from fractions import Fraction as F
from math import comb
import json,hashlib
root=Path(__file__).resolve().parent
raw_bytes=(root/'path-counterexample.json').read_bytes();raw=json.loads(raw_bytes)
proof=json.loads((root/'rational-polynomial-certificate.json').read_text())
assert hashlib.sha256(raw_bytes).hexdigest()==proof['primitive_sha256']
vec=lambda rows:[[F.from_float(v) for v in p] for p in rows]
a,b=vec(raw['capsule_start']),vec(raw['capsule_end'])
c,d=vec(raw['triangle_start']),vec(raw['triangle_end'])
tolerance=F.from_float(float.fromhex(proof['tolerance_hex']))
assert tolerance==F.from_float(1e-10)

def product(p,q):
 m,n=len(p)-1,len(q)-1
 return [sum(p[i]*q[k-i]*F(comb(m,i)*comb(n,k-i),comb(m+n,k)) for i in range(max(0,k-n),min(m,k)+1)) for k in range(m+n+1)]
def elevate(p,degree):
 m=len(p)-1;n=degree-m
 return [sum(p[i]*F(comb(m,i)*comb(n,k-i),comb(degree,k)) for i in range(max(0,k-n),min(m,k)+1)) for k in range(degree+1)]
def verify(radius):
 r=radius-tolerance
 if r<=0:return False
 end=F(0)
 for leaf in proof['leaves']:
  lo,hi=F(*leaf['lo']),F(*leaf['hi'])
  if lo!=end or hi<=lo:return False
  power=[[F(*v) for v in p] for p in leaf['axis']]
  if len(power)!=3 or any(len(p)!=3 for p in power):return False
  axis=[[p[0],p[0]+p[1]/2,sum(p)] for p in power]
  norm=[sum(v) for v in zip(*(product(p,p) for p in axis))]
  if any(v<=0 for v in norm):return False
  norm=elevate(norm,6)
  for i in range(2):
   for j in range(3):
    terms=[]
    for k in range(3):
     first=a[i][k]-c[j][k];slope=(b[i][k]-d[j][k])-first
     terms.append(product([first+lo*slope,first+hi*slope],axis[k]))
    projection=[sum(v) for v in zip(*terms)]
    if any(v<0 for v in projection):return False
    squared=product(projection,projection)
    if any(x-r*r*y<0 for x,y in zip(squared,norm)):return False
  end=hi
 return end==1
radius=F.from_float(raw['radius'])
assert verify(radius)
assert not verify(radius+F.from_float(1e-9)), 'certificate must reject penetrating larger capsule'
report={'status':'independent Bernstein verifier passed','certified_intervals':len(proof['leaves']),'original_radius_accepted':True,'radius_plus_1nm_rejected':True,'scope':'Exact rational certificate for one captured primitive; independent Bernstein products/elevation, no sampling or power-basis builder code reused. Not engine or full-frame qualification.'}
(root/'rational-certificate-verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))

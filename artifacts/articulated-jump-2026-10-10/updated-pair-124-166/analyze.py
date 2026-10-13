import json,math,struct,hashlib
from pathlib import Path
p=Path('artifacts/articulated-jump-2026-10-10/authored-jump-continuous-updated-2026-10-11/frozen-root-motion-01.vjr');raw=p.read_bytes();offset=4
assert raw[:4]==b'VJR1'
def read(fmt):
 global offset
 s=struct.Struct('<'+fmt);v=s.unpack_from(raw,offset);offset+=s.size;return v[0] if len(v)==1 else v
def points(n):return [read('ddd') for _ in range(n)]
n=read('I');radius=read('d');assert 0<n<=8192
rods=[]
for _ in range(n):
 k=read('I');assert 3<=k<=1024
 rods.append([points(k),points(k),points(k),points(k-1)])
count=read('I');groups=[read('II') for _ in range(count)];assert offset==len(raw)
def add(a,b):return tuple(x+y for x,y in zip(a,b))
def sub(a,b):return tuple(x-y for x,y in zip(a,b))
def mul(a,s):return tuple(x*s for x in a)
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def clamp(x):return min(1.,max(0.,x))
def closest(a,b,c,d):
 u,v,w=sub(b,a),sub(d,c),sub(a,c);aa,bb,cc,dd,ee=dot(u,u),dot(u,v),dot(v,v),dot(u,w),dot(v,w)
 assert aa>0 and cc>0
 uv=cross(u,v);den=dot(uv,uv);s=clamp(dot(cross(v,w),uv)/den) if den>0 else 0.;t=(bb*s+ee)/cc
 if t<0:t=0.;s=clamp(-dd/aa)
 elif t>1:t=1.;s=clamp((bb-dd)/aa)
 delta=sub(add(a,mul(u,s)),add(c,mul(v,t)));return math.sqrt(dot(delta,delta)),s,t
rows=[]
for scale in [0.,1.]:
 for time in [0.,1.3713347833893029e-6,1e-5,1e-4,.01,.5,1.]:
  at=[]
  for r in [124,166]:
   old,current,linear,_=rods[r];end=[add(x,mul(v,scale)) for x,v in zip(current,linear)]
   at.append([add(a,mul(sub(b,a),time)) for a,b in zip(old,end)])
  candidates=[]
  for i in range(len(at[0])-1):
   for j in range(len(at[1])-1):
    a,b=at[0][i:i+2];c,d=at[1][j:j+2]
    regions=[(add(a,mul(sub(b,a),.2)),b,c,d),(a,b,add(c,mul(sub(d,c),.2)),d)] if i==j==0 else [(a,b,c,d)]
    for region in regions:
     distance,s,t=closest(*region);candidates.append((distance,i,j,s,t))
  distance,i,j,s,t=min(candidates);rows.append({'correction_scale':scale,'time':time,'distance_m':distance,'gap_m':distance-2*radius,'segments':[i,j],'local_fractions':[s,t]})
report={'fixture':str(p),'sha256':hashlib.sha256(raw).hexdigest(),'rod_pair':[124,166],'contact_diameter_m':2*radius,'sampled_geometry':rows,'scope':'diagnostic sampled distances; independent full continuous guard remains authoritative; zero correction includes all captured staged motion, not only prescribed roots','root_displacements_m':{str(r):math.sqrt(dot(sub(rods[r][1][0],rods[r][0][0]),sub(rods[r][1][0],rods[r][0][0]))) for r in [124,166]}}
Path('artifacts/articulated-jump-2026-10-10/updated-pair-124-166/analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))

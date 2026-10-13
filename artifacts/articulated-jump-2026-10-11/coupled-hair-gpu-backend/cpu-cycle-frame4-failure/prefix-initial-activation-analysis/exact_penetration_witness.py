"""Exact convex-point penetration witness; no sampled clearance admission."""
from fractions import Fraction as F
from pathlib import Path
import json,hashlib
import decimal_path_probe as g
root=Path(__file__).resolve().parent
raw=g.raw
report=json.loads((root/'decimal-path-probe.json').read_text())
time=F.from_float(float(report['minimum_time_probe']))
g.D=F
for name,key in [('a','capsule_start'),('b','capsule_end'),('c','triangle_start'),('d','triangle_end')]:
 setattr(g,name,[[F.from_float(v) for v in p] for p in raw[key]])
p,q=g.closest_at(time)
# Serialize exact points; the independent verifier checks their convex
# membership and strict distance directly, without closest-feature discovery.
lerp=lambda a,b:[x+time*(y-x) for x,y in zip(a,b)]
start,end=[lerp(a,b) for a,b in zip(g.a,g.b)]
a,b,c=[lerp(a,b) for a,b in zip(g.c,g.d)]
u=g.sub(end,start);s=g.dot(g.sub(p,start),u)/g.dot(u,u)
u,v,w=g.sub(b,a),g.sub(c,a),g.sub(q,a)
uu,uv,vv,wu,wv=g.dot(u,u),g.dot(u,v),g.dot(v,v),g.dot(w,u),g.dot(w,v)
den=uu*vv-uv*uv
beta=(wu*vv-wv*uv)/den;gamma=(wv*uu-wu*uv)/den
r=F.from_float(raw['radius'])-F.from_float(1e-10)
distance2=g.dot(g.sub(p,q),g.sub(p,q))
assert 0<=s<=1 and beta>=0 and gamma>=0 and beta+gamma<=1
assert distance2<r*r
out={'status':'candidate exact penetration witness','input_sha256':hashlib.sha256((root/'path-counterexample.json').read_bytes()).hexdigest(),'time':str(time),'capsule_point':[str(x) for x in p],'triangle_point':[str(x) for x in q],'segment_fraction':str(s),'triangle_weights':[str(1-beta-gamma),str(beta),str(gamma)],'squared_clearance_deficit':str(r*r-distance2),'tolerance_hex':(1e-10).hex(),'scope':'One exact trajectory time and convex point pair proves violation of radius minus original tolerance; not full solver/FPS evidence'}
(root/'exact-penetration-witness.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'status':out['status'],'time':float(time),'squared_deficit_m2':float(r*r-distance2)}))

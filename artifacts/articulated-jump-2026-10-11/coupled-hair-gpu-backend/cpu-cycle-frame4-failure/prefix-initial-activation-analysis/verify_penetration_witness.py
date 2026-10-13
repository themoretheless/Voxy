"""Independent exact convex-membership/distance verifier; no feature search."""
from pathlib import Path
from fractions import Fraction as F
import json,hashlib
root=Path(__file__).resolve().parent
raw=json.loads((root/'path-counterexample.json').read_text())
w=json.loads((root/'exact-penetration-witness.json').read_text())
assert hashlib.sha256((root/'path-counterexample.json').read_bytes()).hexdigest()==w['input_sha256']
t=F(w['time']);assert 0<=t<=1
motion=lambda start,end:[[F.from_float(a)+t*(F.from_float(b)-F.from_float(a)) for a,b in zip(x,y)] for x,y in zip(raw[start],raw[end])]
segment=motion('capsule_start','capsule_end');triangle=motion('triangle_start','triangle_end')
s=F(w['segment_fraction']);weights=list(map(F,w['triangle_weights']))
assert 0<=s<=1 and len(weights)==3 and all(x>=0 for x in weights) and sum(weights)==1
p=list(map(F,w['capsule_point']));q=list(map(F,w['triangle_point']))
assert len(p)==len(q)==3
assert p==[(1-s)*segment[0][i]+s*segment[1][i] for i in range(3)]
assert q==[sum(weights[j]*triangle[j][i] for j in range(3)) for i in range(3)]
assert w['tolerance_hex']==(1e-10).hex()
r=F.from_float(raw['radius'])-F.from_float(float.fromhex(w['tolerance_hex']));assert r>0
margin=r*r-sum((a-b)**2 for a,b in zip(p,q));assert margin>0 and margin==F(w['squared_clearance_deficit'])
result={'status':'independently verified exact penetration','input_sha256':w['input_sha256'],'time':w['time'],'strict_squared_deficit':str(margin),'scope':'Strict violation at one exact time with independently checked convex membership; original physical tolerance unchanged'}
(root/'exact-penetration-verification.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':result['status'],'squared_deficit_m2':float(margin)}))

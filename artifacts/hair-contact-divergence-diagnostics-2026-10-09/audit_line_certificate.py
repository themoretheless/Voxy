"""Exact rational moving-line lower-bound audit; no runtime fallback.

For nonparallel lines, segment distance >= |w dot (u cross v)|/|u cross v|.
Bernstein bounds prove a sufficient clearance condition throughout each interval.
This does not certify pairs whose infinite lines approach outside their segments.
"""
import ast
import json
import math
import re
import sys
from fractions import Fraction as F
from pathlib import Path


def add(a, b):
    size = max(len(a), len(b))
    return [sum(values, F(0)) for values in zip(a+[F(0)]*(size-len(a)), b+[F(0)]*(size-len(b)))]


def mul(a, b):
    out = [F(0)]*(len(a)+len(b)-1)
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            out[i+j] += x*y
    return out


def sub(a, b):
    return add(a, [-x for x in b])


def bernstein(power, degree):
    return [sum((power[j]*F(math.comb(i,j), math.comb(degree,j))
                 for j in range(min(i+1,len(power)))), F(0)) for i in range(degree+1)]


def split(values):
    left, right = [values[0]], [values[-1]]
    while len(values) > 1:
        values = [(a+b)/2 for a,b in zip(values,values[1:])]
        left.append(values[0]); right.append(values[-1])
    return left, right[::-1]


def certificate(a0, b0, a1, b1, budget=32768):
    def motion(start, end):
        return [[F(x),F(y)-F(x)] for x,y in zip(start,end)]
    aa=[motion(p,q) for p,q in zip(a0,a1)]
    bb=[motion(p,q) for p,q in zip(b0,b1)]
    u=[sub(aa[1][i],aa[0][i]) for i in range(3)]
    v=[sub(bb[1][i],bb[0][i]) for i in range(3)]
    w=[sub(aa[0][i],bb[0][i]) for i in range(3)]
    cross=[sub(mul(u[(i+1)%3],v[(i+2)%3]),mul(u[(i+2)%3],v[(i+1)%3])) for i in range(3)]
    norm=[F(0)]; triple=[F(0)]
    for i in range(3):
        norm=add(norm,mul(cross[i],cross[i])); triple=add(triple,mul(w[i],cross[i]))
    threshold=2*F(40e-6)-F(1e-10)
    polynomial=sub(mul(triple,triple),[c*threshold*threshold for c in norm])
    stack=[(bernstein(polynomial,6),bernstein(norm,4),0)];visited=0
    while stack:
        visited+=1
        if visited>budget:return {"certified":False,"reason":"budget","visited":visited}
        clearance, denominator, depth=stack.pop()
        if min(clearance)>=0 and min(denominator)>0:continue
        if max(clearance)<0:return {"certified":False,"reason":"line lower bound below threshold","visited":visited}
        if max(denominator)==0:return {"certified":False,"reason":"parallel lines","visited":visited}
        if denominator[0]==0 or denominator[-1]==0:
            return {"certified":False,"reason":"parallel lines at interval endpoint","visited":visited}
        if depth>=64:return {"certified":False,"reason":"subdivision depth","visited":visited}
        al,ar=split(clearance);bl,br=split(denominator)
        stack.extend([(ar,br,depth+1),(al,bl,depth+1)])
    return {"certified":True,"visited":visited}


rows=[];seen=set()
for line in Path(sys.argv[1]).read_text().splitlines():
    if "HAIR SWEPT LIMIT " not in line:continue
    key=line.split(" fraction=")[0]
    if key in seen:break
    seen.add(key)
    endpoints=[ast.literal_eval(v) for v in re.search(r"start_a=(.*) start_b=(.*) end_a=(.*) end_b=(.*)",line).groups()]
    rows.append({"identity":key,**certificate(*endpoints)})
result={"scope":"exact rational sufficient certificate for traced binary endpoints and physical clearance threshold; no full-model qualification", "rows":rows}
Path(sys.argv[2]).write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps(result,indent=2))

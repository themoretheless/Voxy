"""Exact original constant unit orientation against double f32 normalization."""
from fractions import Fraction as F
from math import isqrt
import json,struct,sys


def raw(v):
    return F(struct.unpack('<f',struct.pack('<f',v))[0])


checks=records=0
for line in open(sys.argv[1],encoding='utf-8'):
    tag='stationary_nlerp_reference='
    if tag not in line:
        continue
    d=json.loads(line.split(tag,1)[1])
    q=list(map(raw,d['source']))
    squared=sum(v*v for v in q)
    scale=1<<192
    n=isqrt(squared.numerator*scale*scale//squared.denominator)
    lo=F(n,scale)
    hi=lo if lo*lo==squared else F(n+1,scale)
    assert lo>0 and lo*lo<=squared<=hi*hi
    for original,actual,cap in zip(q,map(raw,d['actual']),d['caps']):
        ideal=sorted([original/lo,original/hi])
        assert max(abs(actual-v) for v in ideal)<=F(cap),d
        checks+=1
    records+=1
assert records==136 and checks==544,(records,checks)
print(f'Exact constant-source NLERP component checks: {checks}')

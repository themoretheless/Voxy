"""Exact original continuous quarter-turn with source key time arithmetic."""
from fractions import Fraction as F
from functools import lru_cache
from math import factorial
import struct,json,sys


def raw(v):
    return F(struct.unpack('<f',struct.pack('<f',v))[0])


def atan(x):
    total=sum((-1)**i*x**(2*i+1)/(2*i+1) for i in range(40))
    r=abs(x**81/81)
    return total-r,total+r


a,b=atan(F(1,5)),atan(F(1,239))
pi=(16*a[0]-4*b[1],16*a[1]-4*b[0])


@lru_cache(maxsize=None)
def series(x,cosine=False):
    if cosine:
        value=sum((-1)**i*x**(2*i)/factorial(2*i) for i in range(40))
        radius=abs(x**80)/factorial(80)
    else:
        value=sum((-1)**i*x**(2*i+1)/factorial(2*i+1) for i in range(40))
        radius=abs(x**81)/factorial(81)
    return value-radius,value+radius


def scale(c,interval):
    values=[c*v for v in interval]
    return min(values),max(values)


def add(a,b):
    return a[0]+b[0],a[1]+b[1]


records=checks=0
for line in open(sys.argv[1],encoding='utf-8'):
    marker='complete_neon_slerp_reference='
    if marker not in line:
        continue
    d=json.loads(line.split(marker,1)[1])
    t0,t1=map(raw,d['times'])
    fraction=(raw(d['time'])-t0)/(t1-t0)
    assert 0<=fraction<=1
    lo,hi=[v*fraction/4 for v in pi]
    sine=(series(lo)[0],series(hi)[1])
    cosine=(series(hi,True)[0],series(lo,True)[1])
    x,y,z,w=map(raw,d['initial'])
    assert x*x+y*y+z*z+w*w==1
    # Original keys are initial * (0,h,0,h), with h an exactly stored
    # common f32 component; real unit normalization gives a quarter-turn.
    ideal=[add(scale(x,cosine),scale(-z,sine)),
           add(scale(y,cosine),scale(w,sine)),
           add(scale(z,cosine),scale(x,sine)),
           add(scale(w,cosine),scale(-y,sine))]
    for actual,interval,cap in zip(map(raw,d['actual']),ideal,d['caps']):
        assert max(abs(actual-v) for v in interval)<=F(cap),(records,d)
        checks+=1
    records+=1
assert records==272 and checks==1088,(records,checks)
print(f'Original continuous SLERP component checks: {checks}')
print(f'Original time/initial-orientation records: {records}')

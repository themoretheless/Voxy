"""Independent rational acos oracle using integer roots and atan remainders."""
from fractions import Fraction as F
from math import isqrt
import json,struct,sys


def small(x):
    total = F(0)
    for i in range(40):
        total += (-1)**i*x**(2*i+1)/(2*i+1)
    radius = abs(x**81/81)
    return total-radius,total+radius


A,B = small(F(1,5)),small(F(1,239))
PI = (16*A[0]-4*B[1],16*A[1]-4*B[0])


def atan(x):
    if x <= F(1,2):
        return small(x)
    a = small((x-1)/(x+1))
    return PI[0]/4+a[0],PI[1]/4+a[1]


def angle(y,x):
    if y <= x:
        return atan(y/x)
    a = atan(x/y)
    return PI[0]/2-a[1],PI[1]/2-a[0]


checks = 0
for line in open(sys.argv[1],encoding='utf-8'):
    marker='slerp_acos_reference='
    if marker not in line:
        continue
    d=json.loads(line.split(marker,1)[1])
    x=F(struct.unpack('<f',struct.pack('<f',d['input']))[0])
    if x == 1:
        oracle=(F(0),F(0))
    elif x == 0:
        oracle=(PI[0]/2,PI[1]/2)
    else:
        square=1-x*x
        scale=1<<192
        n=isqrt(square.numerator*scale*scale//square.denominator)
        lo=F(n,scale)
        hi=lo if lo*lo==square else F(n+1,scale)
        oracle=(angle(lo,x)[0],angle(hi,x)[1])
    assert F(d['ideal'][0]) <= oracle[0] <= oracle[1] <= F(d['ideal'][1]),d
    assert max(abs(F(d['actual'])-v) for v in oracle) <= F(d['cap']),d
    checks += 1
assert checks == 131
print(f'Exact ideal acos enclosure checks: {checks}')
print(f'Exact actual-glam discrepancy checks: {checks}')

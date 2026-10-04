"""Independent exact sine Taylor reference against actual glam NEON outputs."""
from fractions import Fraction as F
from math import factorial
import json,struct,sys


def raw(v):
    return F(struct.unpack('<f',struct.pack('<f',v))[0])


checks=0
for line in open(sys.argv[1],encoding='utf-8'):
    marker='neon_sine_reference='
    if marker not in line:
        continue
    d=json.loads(line.split(marker,1)[1])
    x=raw(d['input'])
    lo,hi=map(raw,d['domain'])
    assert 0 <= lo <= x <= hi
    ideal=sum((-1)**i*x**(2*i+1)/factorial(2*i+1) for i in range(40))
    remainder=abs(x**81)/factorial(81)
    actual=raw(d['actual'])
    assert max(abs(actual-(ideal-remainder)),abs(actual-(ideal+remainder))) <= F(d['cap']),d
    checks+=1
assert checks == 85
print(f'Exact NEON sine discrepancy checks: {checks}')

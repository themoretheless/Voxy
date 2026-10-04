"""Exact same-member quotient reference against actual f32 division."""
import json,struct,sys
from fractions import Fraction as F


def raw(v):
    return F(struct.unpack('<f',struct.pack('<f',v))[0])


checks=0
for line in open(sys.argv[1],encoding='utf-8'):
    marker='positive_division_reference='
    if marker not in line:
        continue
    d=json.loads(line.split(marker,1)[1])
    ideal=raw(d['numerator'])/raw(d['denominator'])
    assert abs(ideal-raw(d['actual']))<=F(d['cap']),d
    checks+=1
assert checks==36
print(f'Exact same-member positive division checks: {checks}')

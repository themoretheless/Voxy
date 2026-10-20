#!/usr/bin/env python3
"""Independent exact-product audit of the overflow-free far-plane exponent gate."""
import argparse
from fractions import Fraction
import json
from pathlib import Path
import random
import struct
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args()
rng=random.Random(0x464152455850)
exponents=[0,1,2,118,119,120,126,127,128,244,245,246,247,248,249,250,251,253,254]
rows=[]
for ea in exponents:
    for eb in exponents:
        for fa,fb in [(0,0),(0x7fffff,0x7fffff),(1,0x7ffffe)]:
            rows.append(((ea<<23)|fa,(eb<<23)|fb))
for _ in range(32768):
    ea=rng.randrange(255);eb=rng.randrange(255)
    rows.append(((ea<<23)|rng.randrange(1<<23),(eb<<23)|rng.randrange(1<<23)))
checked=0
for aa,bb in rows:
    if aa>0x7d7fffff or (aa>>23)+(bb>>23)>372:
        continue
    x,y=(struct.unpack('<f',struct.pack('<I',v))[0] for v in [aa,bb])
    exact=Fraction(x)*Fraction(y)
    assert exact < 2**120
    checked+=1
assert checked>0
result=dict(finite_pairs=len(rows),admitted_exact_product_checks=checked,
            exponent_sum_limit=372,exact_raw_product_strict_upper_bound='2^120',
            false_headroom_admissions=0,gpu_and_full_street_qualified_by_this_audit=False)
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))

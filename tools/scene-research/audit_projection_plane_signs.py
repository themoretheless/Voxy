#!/usr/bin/env python3
"""Check real GPU admission against exact rationals and original paired signs."""
import argparse
from fractions import Fraction
import json
import math
from pathlib import Path
import struct
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('capture',type=Path)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args()
inputs=(a.capture/'inputs.bin').read_bytes();outputs=(a.capture/'certificates.bin').read_bytes()
assert len(inputs)%80==0 and len(outputs)==len(inputs)//80*64
certificates=[0]*6;subnormal_point_cases=0;original_nonfinite_planes=0;rejected_planes=0
for index,(values,words) in enumerate(zip(struct.iter_unpack('<20f',inputs),struct.iter_unpack('<16I',outputs))):
    assert all(math.isfinite(v) for v in values)
    matrix,point=values[:16],values[16:]
    mask=words[0];assert mask<64 and words[13:]==(0,0,0)
    bits=struct.unpack_from('<4I',inputs,index*80+64)
    if any((v&0x7f800000)==0 and (v&0x007fffff)!=0 for v in bits):
        assert mask==0,'subnormal endpoint must retain the complete original path'
        subnormal_point_cases+=1
    for plane in range(6):
        high,low=struct.unpack('<2f',struct.pack('<2I',*words[1+2*plane:3+2*plane]))
        original_finite=math.isfinite(high) and math.isfinite(low)
        original_nonfinite_planes+=not original_finite
        if not mask&(1<<plane):
            rejected_planes+=1
            continue
        assert original_finite and not(high<0 or high==0 and low<0),(index,plane,high,low)
        axis=min(plane//2,2);total=Fraction(0)
        for column,component in enumerate(point):
            row=Fraction(matrix[column*4+axis])
            coefficient=row if plane==4 else Fraction(matrix[column*4+3])+(-row if plane&1 else row)
            total+=coefficient*Fraction(component)
        assert total>0,(index,plane,total)
        certificates[plane]+=1
assert all(certificates) and subnormal_point_cases>0 and original_nonfinite_planes>0
result=dict(cases=len(inputs)//80,exact_positive_certificates_per_plane=certificates,
            exact_positive_certificates=sum(certificates),false_positive_certificates=0,
            original_paired_nonfinite_planes_retained=original_nonfinite_planes,
            original_paired_negative_certified=0,subnormal_point_cases_retained=subnormal_point_cases,
            conservative_nonadmissions=rejected_planes,full_street_qualified=False,fps_qualified=False)
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))

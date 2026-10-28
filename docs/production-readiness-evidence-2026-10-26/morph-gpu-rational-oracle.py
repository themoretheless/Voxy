#!/usr/bin/env python3
"""Independently round source binary32 rational morph sums, without f64 arithmetic."""
from fractions import Fraction
from pathlib import Path
import json
import sys

EVIDENCE = Path(__file__).resolve().parent
log = Path(sys.argv[1]) if len(sys.argv)>1 else EVIDENCE/'morph-gpu-exact-cases.log'
records = [line.removeprefix('MORPH_EXACT_CASES ') for line in log.read_text().splitlines()
           if line.startswith('MORPH_EXACT_CASES ')]
assert len(records)==1, 'missing or ambiguous physical GPU payload'
records = json.loads(records[0])

def binary32(bits):
    exponent=(bits>>23)&255
    assert exponent!=255
    mantissa=(bits&0x7fffff) | (0x800000 if exponent else 0)
    value=Fraction(mantissa)*(Fraction(2)**(exponent-150 if exponent else -149))
    return -value if bits>>31 else value

def rounded_binary32(value):
    if not value: return 0
    negative=value<0
    value=abs(value)
    exponent=value.numerator.bit_length()-value.denominator.bit_length()
    if value<Fraction(2)**exponent: exponent-=1
    scaled=value*(Fraction(2)**(149 if exponent < -126 else 23-exponent))
    quotient,remainder=divmod(scaled.numerator,scaled.denominator)
    if 2*remainder>scaled.denominator or (2*remainder==scaled.denominator and quotient%2): quotient+=1
    if exponent < -126: magnitude=quotient
    else:
        if quotient==1<<24: quotient>>=1;exponent+=1
        assert exponent<=127, 'unadmitted overflow'
        magnitude=((exponent+127)<<23)|(quotient&0x7fffff)
    return (int(negative)<<31)|magnitude

def rounded_binary64(value):
    if not value: return 0
    negative=value<0
    value=abs(value)
    exponent=value.numerator.bit_length()-value.denominator.bit_length()
    if value<Fraction(2)**exponent: exponent-=1
    # Finite binary32 products and <=256 terms fit normal binary64.
    assert -1022<=exponent<=1023
    scaled=value*(Fraction(2)**(52-exponent))
    quotient,remainder=divmod(scaled.numerator,scaled.denominator)
    if 2*remainder>scaled.denominator or (2*remainder==scaled.denominator and quotient%2): quotient+=1
    if quotient==1<<53: quotient>>=1;exponent+=1
    return (int(negative)<<63)|((exponent+1023)<<52)|(quotient&((1<<52)-1))

for index,record in enumerate(records):
    exact=binary32(record['base_bits'])+sum((binary32(a)*binary32(b) for a,b in record['terms']),Fraction())
    expected=rounded_binary32(exact)
    assert record['gpu_bits']==record['cpu_bits']==expected, (index,record,expected)
    assert record['cpu64_bits']==rounded_binary64(exact), (index,record,exact)
result={'status':'physical_gpu_and_cpu_match_independent_rational_oracle',
        'cpu64_matches_rational':True,'cases':len(records),'max_nonzero_terms':max(len(r['terms']) for r in records),
        'arithmetic':'exact Fraction source values, independent integer quotient/remainder rounding to binary32',
        'universal_hardware_claim':False,'fps_qualified':False}
(EVIDENCE/'morph-gpu-rational-oracle.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))

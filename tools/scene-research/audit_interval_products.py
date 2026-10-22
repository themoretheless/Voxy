#!/usr/bin/env python3
"""Check raw GPU interval products against exact rational endpoint products."""
import argparse
import hashlib
import json
import math
import struct
from fractions import Fraction as Q
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('root',type=Path)
a=parser.parse_args()
inputs=(a.root/'inputs.bin').read_bytes()
outputs=(a.root/'paired-results.bin').read_bytes()
assert len(inputs)%144==0
count=len(inputs)//144
assert len(outputs)==count*64
assert outputs[:count*32]==outputs[count*32:]
counts={'valid':0,'unresolved':0,'invalid_input':0,'both_singletons':0,'one_singleton':0,'two_ranges':0}
def number(word):
    return struct.unpack('<f',struct.pack('<I',word))[0]
for i,w in enumerate(struct.iter_unpack('<36I',inputs)):
    valid,lo,hi,index,*padding=struct.unpack_from('<8I',outputs,i*32)
    assert valid in (0,1) and index==i and not any(padding)
    scalar_a=w[0]==w[1]
    scalar_b=w[4]==w[5]
    counts['both_singletons'if scalar_a and scalar_b else 'one_singleton'if scalar_a or scalar_b else 'two_ranges']+=1
    if w[2]==0 or w[6]==0:
        assert (valid,lo,hi)==(0,0,0)
        counts['invalid_input']+=1
    if valid==0:
        counts['unresolved']+=1
        continue
    vals=[number(w[k]) for k in [0,1,4,5]]
    lower,upper=number(lo),number(hi)
    assert all(map(math.isfinite,vals+[lower,upper]))
    products=[Q.from_float(x)*Q.from_float(y) for x in vals[:2] for y in vals[2:]]
    assert Q.from_float(lower)<=min(products)<=max(products)<=Q.from_float(upper),(i,w,lower,upper)
    counts['valid']+=1
assert counts['valid']>0 and counts['unresolved']>0 and counts['invalid_input']>0
assert all(counts[k]>0 for k in ['both_singletons','one_singleton','two_ranges'])
proof={'cases':count,'counts':counts,'all_output_words_bit_identical':True,'all_valid_bounds_enclose_exact_endpoint_products':True,'input_sha256':hashlib.sha256(inputs).hexdigest(),'output_sha256':hashlib.sha256(outputs).hexdigest(),'full_scene_qualified':False,'fps_qualified':False}
(a.root/'exact-oracle-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps(proof,indent=2))

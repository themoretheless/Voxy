#!/usr/bin/env python3
"""Audit differential triangle certificates against the closed exact rational oracle."""
import argparse
import hashlib
import json
import math
import struct
from fractions import Fraction as Q
from pathlib import Path
from exact_closed_world_triangle import classify

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('root',type=Path)
args=parser.parse_args()
inputs=(args.root/'inputs.bin').read_bytes()
results=(args.root/'paired-results.bin').read_bytes()
assert len(inputs)%144==0
count=len(inputs)//144
assert len(results)==count*32*2
assert results[:count*32]==results[count*32:]
counts={'proved_miss':0,'proved_hit':0,'unresolved_finite':0,'nonfinite_unknown':0}
for index,values in enumerate(struct.iter_unpack('<36f',inputs)):
    state=struct.unpack_from('<I',results,index*32)[0]
    assert state in (0,1,2)
    used=[values[j*4+k] for j in range(8) for k in range(3)]
    if not all(map(math.isfinite,used)):
        assert state==2,(index,'nonfinite source admitted')
        counts['nonfinite_unknown']+=1
        continue
    assert values[32:36]==(0.,0.,1.,0.),'oracle currently owns closed [0,1] segments'
    def vector(j):
        return tuple(Q.from_float(values[j*4+k])+Q.from_float(values[(j+4)*4+k]) for k in range(3))
    if state==2:
        counts['unresolved_finite']+=1
        continue
    direction,edge1,edge2,origin=(vector(j)for j in range(4))
    exact=classify([(Q(0),)*3,edge1,edge2],origin,direction)
    assert exact['exact_hit']==bool(state),(index,state,exact)
    counts['proved_hit' if state else 'proved_miss']+=1
assert counts['proved_miss']>0 and counts['proved_hit']>0, 'fixture must exercise both definite outcomes'
proof={'cases':count,'counts':counts,'all_definite_states_match_exact_closed_oracle':True,'complete_gpu_results_bit_identical':True,'input_sha256':hashlib.sha256(inputs).hexdigest(),'results_sha256':hashlib.sha256(results).hexdigest(),'full_scene_qualified':False,'fps_qualified':False}
(args.root/'exact-oracle-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps(proof,indent=2))

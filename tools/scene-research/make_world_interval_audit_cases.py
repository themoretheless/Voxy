#!/usr/bin/env python3
"""Keep original source triangles; perturb exact ray words for isolated miss-certificate audits."""
import json,random,struct,sys
from pathlib import Path
from make_world_exact_parameter_gold import build
source,cases_out,gold_out=map(Path,sys.argv[1:]);original=[json.loads(s) for s in source.read_text().splitlines() if s];rng=random.Random(0x564f5859);cases=[]
decode=lambda b:struct.unpack('<f',struct.pack('<I',b))[0]
encode=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
for parent in original:
 for variant in range(21):
  case=dict(parent);case['ray_words']=list(parent['ray_words']);case['global_ray']=(1<<48)+len(cases);case['certificate_variant']=variant
  if variant:
   for base in (0,8):
    axis=rng.randrange(3);slot=base+axis
    case['ray_words'][slot]=encode(decode(case['ray_words'][slot])+rng.choice([-2.,-.25,0.,.25,2.]))
  cases.append(case)
with cases_out.open('x') as f:f.write(''.join(json.dumps(c)+'\n' for c in cases))
gold=build(cases)
with gold_out.open('x') as f:json.dump(gold,f,indent=2)
print(json.dumps(dict(cases=len(cases),hits=sum(r['exact_hit'] for r in gold['rows']))))

#!/usr/bin/env python3
"""Independent IEEE-f32 replay of observed shader operands; no epsilon gates.

Consumes diagnostic-only native u32 records, never uploads expected GPU results.
A disagreement diagnoses arithmetic and does not establish a renderer speedup.
"""
import argparse, array, json, math, struct, sys
from pathlib import Path

def f(x): return struct.unpack('<f',struct.pack('<f',x))[0]
def bits(x): return struct.unpack('<I',struct.pack('<f',x))[0]
def load(root):
 m=json.loads((root/'metadata.json').read_text());w=array.array('I');w.frombytes((root/'operands.u32.bin').read_bytes())
 if sys.byteorder!='little': w.byteswap()
 assert m['schema'] in (1,2) and m['face_vec4_stride']==(528 if m['schema']==1 else 656) and m['sample_capacity']==65536
 assert len(w)*4==m['capture']['bytes']
 assert len(w)==(m['probe_count']*6*m['face_vec4_stride']+6*2*65536)*4
 return m,w

def audit(root):
 m,w=load(root); face_stride=m['face_vec4_stride']; failures=[]; counts={'faces':0,'sample_records':0,'thread_rgb_words':0,'reduction_rgb_words':0,'accumulation_rgb_words':0}
 def val(i): return struct.unpack('<f',struct.pack('<I',w[i]))[0]
 def check(actual,expected,where):
  if actual!=bits(expected) and len(failures)<32: failures.append({'where':where,'actual_u32':actual,'expected_u32':bits(expected)})
 # Every probe's parallel reduction and final accumulation, including untargeted probes.
 for probe in range(m['probe_count']):
  for face in range(6):
   b=(probe*6+face)*face_stride*4;counts['faces']+=1
   assert val(b+512*4+3)==probe and val(b+513*4+3)==face
   partial=[[[val(b+(t*4+k)*4+c) for c in range(3)] for k in range(4)] for t in range(128)]
   for stride in (64,32,16,8,4,2,1):
    for t in range(stride):
     for k in range(4):
      for c in range(3): partial[t][k][c]=f(partial[t][k][c]+partial[t+stride][k][c])
   for k in range(4):
    for c in range(3):
     check(w[b+(520+k)*4+c],partial[0][k][c],f'{probe}/{face}/reduction/{k}/{c}');counts['reduction_rgb_words']+=1
     previous=val(b+(516+k)*4+c)
     check(w[b+(524+k)*4+c],f(previous+val(b+(520+k)*4+c)),f'{probe}/{face}/accumulation/{k}/{c}');counts['accumulation_rgb_words']+=1
     if face==0: assert previous==0
     else: assert w[b+(516+k)*4+c]==w[b-face_stride*4+(524+k)*4+c]
 # Target probe: exact per-thread sums from observed radiance, weights and n.
 for face in range(6):
  b=(m['target_probe']*6+face)*face_stride*4
  size,step=val(b+515*4+1),val(b+515*4+2);n=int(size/step);assert n*n<=65536
  partial=[[[0.]*3 for _ in range(4)] for _ in range(128)]; hashes=[[2166136261]*4 for _ in range(128)]
  for i in range(n*n):
   j=(m['sample_vec4_base']+face*131072+i*2)*4;assert w[j+7]==0x50425431
   h=hashes[i%128]
   for c in range(3):h[c]=((h[c]^w[j+c])*16777619)&0xffffffff
   for c in (3,4,5,6):h[3]=((h[3]^w[j+c])*16777619)&0xffffffff
   r=[val(j+c) for c in range(3)];weight=val(j+3);normal=[val(j+4+c) for c in range(3)]
   assert all(math.isfinite(v) and (v==0 or abs(v)>=2**-126) for v in r+[weight]+normal), 'non-normal numeric domain requires explicit FTZ analysis'
   multipliers=[f(f(.282095)*weight)]+[f(f(f(.488603)*normal[c])*weight) for c in (1,2,0)]
   for k in range(4):
    for c in range(3):partial[i%128][k][c]=f(partial[i%128][k][c]+f(r[c]*multipliers[k]))
   counts['sample_records']+=1
  for t in range(128):
   if m['schema']==2: assert list(w[b+(528+t)*4:b+(529+t)*4])==hashes[t], 'observed target input hash must match complete raw operands'
   for k in range(4):
    for c in range(3):check(w[b+(t*4+k)*4+c],partial[t][k][c],f'{m["target_probe"]}/{face}/thread/{t}/{k}/{c}');counts['thread_rgb_words']+=1
 return {'root':str(root),'counts':counts,'first_failures':failures,'all_observed_arithmetic_bit_exact':not failures,'performance_qualified':False}

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('--compare',type=Path);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 result=audit(args.capture)
 if args.compare:
  left,a=load(args.capture);right,b=load(args.compare);assert (left['probe_count'],left['target_probe'],left['schema'])==(right['probe_count'],right['target_probe'],right['schema'])
  result['other_audit']=audit(args.compare)
  assert len(a)==len(b)
  differences=[i for i,(x,y) in enumerate(zip(a,b)) if x!=y]
  categories={}; details=[]; changed_inputs={}
  for i in differences:
   vec,c=divmod(i,4)
   if vec<left['sample_vec4_base']:
    pf,local=divmod(vec,left['face_vec4_stride']);probe,face=divmod(pf,6)
    category='thread_partial' if local<512 else 'uniform' if local<516 else 'previous' if local<520 else 'reduced' if local<524 else 'result' if local<528 else 'radiance_hash' if c<3 else 'geometry_hash'
    where={'probe':probe,'face':face,'record':local,'channel':c}
    if local>=528:
     key=f'{probe}/{face}';changed_inputs.setdefault(key,{'radiance_threads':set(),'geometry_threads':set()})
     changed_inputs[key]['radiance_threads' if c<3 else 'geometry_threads'].add(local-528)
   else:
    face,local=divmod(vec-left['sample_vec4_base'],131072);sample,part=divmod(local,2)
    category=('radiance' if c<3 else 'weight') if part==0 else ('normal' if c<3 else 'marker')
    where={'probe':left['target_probe'],'face':face,'sample':sample,'channel':c}
   categories[category]=categories.get(category,0)+1
   if len(details)<64: details.append(dict(where,category=category,word=i,left=a[i],right=b[i]))
  result['comparison']={'different_words':len(differences),'categories':categories,'changed_input_hash_threads':{k:{kind:sorted(v) for kind,v in row.items()} for k,row in changed_inputs.items()},'hash_equality_is_not_operand_equality_proof':True,'first_differences':details,'all_native_words_identical':not differences}
 args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'all_observed_arithmetic_bit_exact':result['all_observed_arithmetic_bit_exact'],'comparison':{k:v for k,v in result.get('comparison',{}).items() if k!='first_differences'},'counts':result['counts']}))
 if not result['all_observed_arithmetic_bit_exact'] or ('other_audit' in result and not result['other_audit']['all_observed_arithmetic_bit_exact']):raise SystemExit(1)
if __name__=='__main__':main()

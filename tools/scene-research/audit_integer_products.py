#!/usr/bin/env python3
"""Independent Python arbitrary-integer reference for 2048/4096-bit GPU products."""
import argparse,json,random,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('mode',choices=['generate','verify']);p.add_argument('input',type=Path);p.add_argument('--results',type=Path);p.add_argument('--output',type=Path);a=p.parse_args();mask=(1<<2048)-1
if a.mode=='generate':
 rows=[(0,mask,0,1),(1,mask,mask,1),(mask,mask,mask,mask-1),(mask,mask,mask,mask),(1<<1024,1<<1023,1<<2047,1),(1<<1024,1<<1024,mask,1),((1<<1024)-1,(1<<1024)-1,1<<1024,(1<<1024)-2),(65535,mask,mask,65535)]
 rng=random.Random(0x564f5859)
 for k in range(248):
  values=[]
  for j in range(4):
   bits=rng.choice([1,16,31,64,149,298,512,1023,1024,1025,1700,2047,2048]);value=rng.getrandbits(bits)
   if k%3==0:value=1<<rng.randrange(bits)
   if k%5==0:value=((1<<bits)-1)
   values.append(value)
  rows.append(tuple(values))
 with a.input.open('xb') as f:
  for row in rows:
   for value in row:f.write(struct.pack('<128I',*[(value>>(16*k))&65535 for k in range(128)]))
 print(json.dumps(dict(cases=len(rows))))
else:
 raw=list(struct.iter_unpack('<512I',a.input.read_bytes()));actual=list(struct.iter_unpack('<132I',a.results.read_bytes()));assert len(raw)==len(actual);valid_products=0;wide=0
 for i,(operands,result) in enumerate(zip(raw,actual)):
  values=[sum(operands[j*128+k]<<(16*k) for k in range(128)) for j in range(4)];left=values[0]*values[1];right=values[2]*values[3];valid=left<=mask
  assert result[0]==int(valid),(i,'capacity');assert result[2]==1,(i,'comparison valid')
  expected=(left>right)-(left<right);observed=struct.unpack('<i',struct.pack('<I',result[3]))[0];assert observed==expected,(i,'full product ordering')
  if valid:
   assert result[1]==(left.bit_length()+15)//16,(i,'length');assert sum(result[4+k]<<(16*k) for k in range(128))==left,(i,'exact product');valid_products+=1
  else:assert not any(result[1:2]+result[4:]),(i,'overflow payload')
  wide+=int(max(left.bit_length(),right.bit_length())>2048)
 report=dict(cases=len(raw),exact_products=valid_products,comparisons_beyond_2048_bits=wide,full_4096_bit_ordering_verified=True,all_bits_exact=True)
 with a.output.open('x') as f:json.dump(report,f,indent=2)
 print(json.dumps(report))

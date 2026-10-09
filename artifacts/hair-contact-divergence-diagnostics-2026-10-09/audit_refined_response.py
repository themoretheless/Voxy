"""100-digit check of returned VQR1 responses against original VQI1 loads."""
import json,struct,sys
from decimal import Decimal,localcontext
from pathlib import Path
class Reader:
 def __init__(self,path):self.raw=Path(path).read_bytes();self.offset=0
 def take(self,fmt):
  result=struct.unpack_from(fmt,self.raw,self.offset);self.offset+=struct.calcsize(fmt)
  return result[0] if len(result)==1 else result
 def values(self,n):return list(self.take('<'+str(n)+'d')) if n!=1 else [self.take('<d')]
 def skip(self,n):self.offset+=n;assert self.offset<=len(self.raw)
a=Reader(sys.argv[1]);assert a.take('<4s')==b'VQI1'
m,d,count,failed=a.take('<4I');tolerance=a.take('<d');bounds=a.values(m)
a.skip(8*m+8*m*d+8*d);loads=[];fixed=[];dimensions=[];base=0
for _ in range(count):
 n,band,lo,hi=a.take('<4I');dimensions.append(n);a.skip(8*n*band+8*n)
 loads.append([a.values(n) for _ in range(m)]);a.skip(8*n)
 fixed.extend(range(base,base+lo));fixed.extend(range(base+hi,base+n));base+=n
assert a.offset==len(a.raw) and base==d
b=Reader(sys.argv[2]);assert b.take('<4s')==b'VQR1' and b.take('<I')==count
responses=[]
for n in dimensions:
 assert b.take('<I')==n;responses.append(b.values(n))
assert b.take('<I')==m;reactions=b.values(m);assert b.offset==len(b.raw)
gaps=[]
with localcontext() as ctx:
 ctx.prec=100
 for i,bound in enumerate(bounds):
  actual=sum((Decimal.from_float(c)*Decimal.from_float(x) for request,response in zip(loads,responses) for c,x in zip(request[i],response) if c!=0.),Decimal(0))
  gaps.append(float(actual-Decimal.from_float(bound)))
active=[abs(g) for g,l in zip(gaps,reactions) if l>0.];inactive=[g for g,l in zip(gaps,reactions) if l==0.]
x=[v for response in responses for v in response]
r={'scope':'100-digit original binary loads, returned response/reactions and unchanged original bounds; no full-animation proof','rows':m,'tolerance':tolerance,'failed_row_new_exact_gap':gaps[failed],'max_active_exact_gap_abs':max(active,default=0),'minimum_inactive_exact_gap':min(inactive,default=0),'nonnegative_reactions':all(l>=0 for l in reactions),'maximum_fixed_response':max((abs(x[i]) for i in fixed),default=0),'passed':max(active,default=0)<=tolerance and min(inactive,default=0)>=-tolerance and all(l>=0 for l in reactions) and all(x[i]==0 for i in fixed)}
Path(sys.argv[3]).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))

"""Independent original-load/whitened diagnostics; LP is not runtime admission."""
import json,struct,sys
from decimal import Decimal,localcontext
from pathlib import Path
import numpy as np
from scipy.optimize import linprog
from scipy.sparse import csr_matrix
raw=Path(sys.argv[1]).read_bytes();offset=0
def take(fmt):
 global offset
 size=struct.calcsize(fmt);value=struct.unpack_from(fmt,raw,offset);offset+=size
 return value[0] if len(value)==1 else value
def floats(n):
 global offset
 value=np.frombuffer(raw,dtype='<f8',count=n,offset=offset).copy();offset+=n*8
 return value
assert take('<4s')==b'VQI1'
m,d,count,failed=take('<4I');tolerance=take('<d')
bounds=floats(m);reactions=floats(m);w=floats(m*d).reshape(m,d);coordinates=floats(d)
loads=[];responses=[];fixed=[];total=0
for _ in range(count):
 n,band,lo,hi=take('<4I');matrix=floats(n*band);rhs=floats(n)
 loads.append(floats(m*n).reshape(m,n));responses.append(floats(n))
 fixed.extend(range(total,total+lo));fixed.extend(range(total+hi,total+n));total+=n
assert offset==len(raw) and total==d
j=np.concatenate(loads,axis=1);x=np.concatenate(responses)
gap=j@x-bounds;white_gap=w@coordinates-bounds
active=reactions>0
bad=np.where(np.where(active,np.abs(gap)>tolerance,gap < -tolerance))[0]
def exact_gap(row,values,bound):
 with localcontext() as ctx:
  ctx.prec=100
  return float(sum((Decimal.from_float(float(a))*Decimal.from_float(float(b)) for a,b in zip(row,values)),Decimal(0))-Decimal.from_float(float(bound)))
rows=[{'row':int(i),'reaction':float(reactions[i]),'original_gap_m':exact_gap(j[i],x,bounds[i]),'whitened_gap_m':exact_gap(w[i],coordinates,bounds[i])} for i in sorted(set(list(bad)+[failed]))[:32]]
used=np.any(j!=0,axis=0);a=csr_matrix(j[:,used])
lp=linprog(np.zeros(int(used.sum())),A_ub=-a,b_ub=-bounds,bounds=[(None,None)]*int(used.sum()),method='highs',options={'primal_feasibility_tolerance':1e-10,'dual_feasibility_tolerance':1e-10})
primal={'status':int(lp.status),'message':lp.message,'scope':'numerical feasibility only; no physical pose, energy or continuous-geometry proof'}
if lp.success:
 lx=np.zeros(d);lx[used]=lp.x
 exact=[exact_gap(row,lx,bound) for row,bound in zip(j,bounds)]
 primal.update(minimum_exact_pointwise_gap_m=min(exact),within_captured_absolute_tolerance=min(exact)>=-tolerance,maximum_linear_coordinate=float(np.max(np.abs(lx))))
r={'rows':m,'coordinates':d,'systems':count,'failed_row':failed,'absolute_tolerance':tolerance,'captured_bytes':len(raw),'native_original_bad_rows':len(bad),'max_original_gap_abs_m':float(np.max(np.abs(gap[active]))) if active.any() else 0,'max_whitened_active_gap_abs_m':float(np.max(np.abs(white_gap[active]))) if active.any() else 0,'max_coordinate':float(np.max(np.abs(coordinates))),'max_response':float(np.max(np.abs(x))),'maximum_fixed_response':float(np.max(np.abs(x[fixed]))) if fixed else 0,'pointwise_exact_rows':rows,'original_load_lp':primal}
Path(sys.argv[2]).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))

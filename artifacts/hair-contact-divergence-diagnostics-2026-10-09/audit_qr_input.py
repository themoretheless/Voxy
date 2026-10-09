"""Feasibility diagnostics for unsolved VQC1; no energy/motion admission."""
import json,struct,sys
from decimal import Decimal,localcontext
from pathlib import Path
import numpy as np
from scipy.optimize import linprog
from scipy.sparse import csr_matrix
raw=Path(sys.argv[1]).read_bytes();o=0
def take(fmt):
 global o
 v=struct.unpack_from(fmt,raw,o);o+=struct.calcsize(fmt);return v[0] if len(v)==1 else v
def values(n):
 global o
 a=np.frombuffer(raw,dtype='<f8',count=n,offset=o).copy();o+=n*8;return a
assert take('<4s')==b'VQC1';m,d,count,refinement=take('<4I');tol=take('<d')
bounds=values(m);effective=values(m);w=values(m*d).reshape(m,d);loads=[]
for _ in range(count):
 n,band,lo,hi=take('<4I');matrix=values(n*band);rhs=values(n)
 loads.append(values(m*n).reshape(m,n))
assert o==len(raw)
j=np.concatenate(loads,axis=1);assert j.shape==(m,d)
def feasibility(a,b):
 used=np.any(a!=0,axis=0)
 lp=linprog(np.zeros(int(used.sum())),A_ub=-csr_matrix(a[:,used]),b_ub=-b,bounds=[(None,None)]*int(used.sum()),method='highs',options={'primal_feasibility_tolerance':1e-10,'dual_feasibility_tolerance':1e-10})
 r={'status':int(lp.status),'message':lp.message,'scope':'numerical linear feasibility; no energy, force-balance or continuous-geometry proof'}
 if lp.success:
  x=np.zeros(a.shape[1]);x[used]=lp.x
  with localcontext() as ctx:
   ctx.prec=100
   gaps=[float(sum((Decimal.from_float(float(v))*Decimal.from_float(float(y)) for v,y in zip(row,x) if v!=0.),Decimal(0))-Decimal.from_float(float(bound))) for row,bound in zip(a,b)]
  r.update(minimum_exact_gap=min(gaps),within_requested_tolerance=min(gaps)>=-tol,maximum_parameter=float(np.max(np.abs(x))))
 return r
r={'rows':m,'coordinates':d,'systems':count,'defect_refinement':refinement,'bytes':len(raw),'tolerance':tol,'maximum_numeric_bound_shift':float(np.max(np.abs(bounds-effective))),'original_load_feasibility':feasibility(j,bounds),'whitened_feasibility':feasibility(w,effective)}
Path(sys.argv[2]).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))

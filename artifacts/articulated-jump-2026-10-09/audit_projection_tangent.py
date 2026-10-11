"""Independent tangent feasibility audit; does not advance the physical model."""
import json, struct, sys
from pathlib import Path
import numpy as np
from scipy.optimize import linprog, nnls
from decimal import Decimal, localcontext
raw=Path(sys.argv[1]).read_bytes()
assert raw[:4]==b'VQP1'
n=struct.unpack_from('<I',raw,4)[0]
tolerance=struct.unpack_from('<d',raw,8)[0]
offset=16
G=np.array(struct.unpack_from(f'<{n*n}d',raw,offset)).reshape(n,n);offset+=n*n*8
b=np.array(struct.unpack_from(f'<{n}d',raw,offset));offset+=n*8
reaction=np.array(struct.unpack_from(f'<{n}d',raw,offset));offset+=n*8
nr=struct.unpack_from('<I',raw,offset)[0];offset+=4
shape=struct.unpack_from(f'<{nr}I',raw,offset);offset+=nr*4
rows=[]
for _ in range(n):
    row={}
    for _ in range(4):
        rod,point,*gradient=struct.unpack_from('<II3d',raw,offset);offset+=32
        assert rod<nr and point<shape[rod]
        if point==0:continue # prescribed scalp degrees of freedom
        for axis,value in enumerate(gradient):
            if value:row[(rod,point,axis)]=row.get((rod,point,axis),0.0)+value
    rows.append(row)
assert offset==len(raw)
keys=sorted(set().union(*(row.keys() for row in rows)));index={key:i for i,key in enumerate(keys)}
J=np.zeros((n,len(keys)))
for i,row in enumerate(rows):
    for key,value in row.items():J[i,index[key]]=value
lp=linprog(np.zeros(len(keys)),A_ub=-J,b_ub=-b,bounds=[(None,None)]*len(keys),method='highs',options={'primal_feasibility_tolerance':1e-10,'dual_feasibility_tolerance':1e-10})
result={'constraints':n,'free_position_coordinates':len(keys),'original_tolerance_m':tolerance,'lp_status':int(lp.status),'lp_message':lp.message,'scope':'Original tangent inequalities only; not implicit equilibrium, nonlinear contact, strain or motion qualification.'}
if lp.success:
    with localcontext() as ctx:
        ctx.prec=100
        x=[Decimal.from_float(float(v)) for v in lp.x]
        gaps=[sum((Decimal.from_float(v)*x[index[k]] for k,v in row.items()),Decimal(0))-Decimal.from_float(float(bound)) for row,bound in zip(rows,b)]
        result['lp_original_max_violation_m']=str(max(Decimal(0),-min(gaps)))
    result['lp_max_position_coordinate_m']=float(np.max(np.abs(lp.x)))
with localcontext() as ctx:
    ctx.prec=100
    lam=[Decimal.from_float(float(v)) for v in reaction]
    gaps=[sum((Decimal.from_float(float(v))*l for v,l in zip(row,lam)),Decimal(0))-Decimal.from_float(float(bound)) for row,bound in zip(G,b)]
    result['captured_original_max_violation_m']=str(max(Decimal(0),-min(gaps)))
    result['captured_complementarity_residual_m']=str(max(abs(gap) if l>0 else max(Decimal(0),-gap) for gap,l in zip(gaps,lam)))
result['gram_max_asymmetry']=float(np.max(np.abs(G-G.T)))
result['gram_min_eigenvalue']=float(np.linalg.eigvalsh((G+G.T)*0.5)[0])

# Singular Gram systems still have meaningful primal tangent inequalities.
# Exact opposite original rows with positive bound sum prove inconsistency.
result['exact_opposite_tangent_rows'] = []
for i in range(n):
    for j in range(i):
        if np.array_equal(J[i], -J[j]):
            with localcontext() as ctx:
                ctx.prec = 100
                total = Decimal.from_float(float(b[i])) + Decimal.from_float(float(b[j]))
                if total > Decimal.from_float(2*tolerance):
                    result['exact_opposite_tangent_rows'].append({
                        'a': i, 'b': j, 'exact_sum_bounds': str(total),
                        'coordinates': [{'rod': key[0], 'point': key[1], 'axis': key[2], 'gradient': value} for key, value in rows[i].items()],
                    })
print(json.dumps(result, indent=2))

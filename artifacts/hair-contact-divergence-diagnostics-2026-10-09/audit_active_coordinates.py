"""100-digit audit of a VQC1 point; no motion or force-balance certificate."""
import struct,json,sys
from decimal import Decimal,localcontext
from pathlib import Path
raw=Path(sys.argv[1]).read_bytes()
assert raw[:4]==b'VQC1'
m,d,systems,refinement=struct.unpack_from('<4I',raw,4)
tol=struct.unpack_from('<d',raw,20)[0]
bounds=struct.unpack_from('<%dd'%m,raw,28+8*m)
w=struct.unpack_from('<%dd'%(m*d),raw,28+16*m)
x=struct.unpack('<%dd'%(d+m),Path(sys.argv[2]).read_bytes())
with localcontext() as ctx:
 ctx.prec=100
 gaps=[float(sum((Decimal.from_float(a)*Decimal.from_float(y) for a,y in zip(w[i*d:(i+1)*d],x[:d]) if a),Decimal(0))-Decimal.from_float(bounds[i])) for i in range(m)]
r={'rows':m,'tolerance':tol,'max_active_exact_residual':max((abs(gaps[i]) for i in range(m) if x[d+i]>0),default=0.),'minimum_inactive_exact_gap':min((gaps[i] for i in range(m) if x[d+i]==0),default=None),'all_exact_rows_admitted':all(x[d+i]>=0 and (abs(gaps[i])<=tol if x[d+i]>0 else gaps[i]>=-tol) for i in range(m))}
Path(sys.argv[3]).write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps(r,indent=2))

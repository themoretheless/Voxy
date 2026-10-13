"""Offline 80-digit diagnostic; this is not a production contact solver."""
import decimal, struct, json, pathlib, math
D=decimal.Decimal
decimal.getcontext().prec=80
root=pathlib.Path(__file__).parent
data=json.loads((root/'rejected-original-operator.json').read_text())
def f64(bits):return struct.unpack('<d',struct.pack('<Q',bits))[0]
def original(bits):return D.from_float(f64(bits))
def component(bits):return D.from_float(struct.unpack('<f',struct.pack('<I',bits))[0])
def dot(a,b):return sum((x*y for x,y in zip(a,b)),D(0))
def solve(a,b):
 a=[row[:]+[rhs] for row,rhs in zip(a,b)];n=len(b)
 for i in range(n):
  k=max(range(i,n),key=lambda j:abs(a[j][i]));a[i],a[k]=a[k],a[i]
  pivot=a[i][i]
  if not pivot:raise ValueError('exact zero pivot')
  a[i]=[v/pivot for v in a[i]]
  for j in range(n):
   if j!=i:
    factor=a[j][i];a[j]=[x-factor*y for x,y in zip(a[j],a[i])]
 return [row[-1] for row in a]
cols=[[original(v) for v in row] for row in data['columns_bits']]
bounds=[original(v) for v in data['bounds_bits']]
active=[i for i,v in enumerate(data['reactions_bits']) if f64(v)>0]
words=data['packed_input_words'];width,rows=words[:2]
packed=[component(words[i])+component(words[i+1]) for i in range(4,len(words),2)]
pc=[packed[i*width:(i+1)*width] for i in range(rows)];pb=packed[rows*width:rows*width+rows]
def split3(value):
 remainder=D.from_float(value);total=D(0)
 for _ in range(3):
  part=struct.unpack('<f',struct.pack('<f',float(remainder)))[0]
  exact=D.from_float(part);total+=exact;remainder-=exact
 return total
cs=f64(data['column_scale_bits']);bs=max(abs(float(v)) for v in bounds)
regular3=[[split3(float(v)/cs) for v in row] for row in cols]
regular_b3=[split3(float(v)/bs) for v in bounds]
pcscale=math.ldexp(1.,math.frexp(cs)[1]-1);pbscale=math.ldexp(1.,math.frexp(bs)[1]-1)
power3=[[split3(float(v)/pcscale) for v in row] for row in cols]
power_b3=[split3(float(v)/pbscale) for v in bounds]
reports=[]
for name,c,b,scale in [('original_f64_input',cols,bounds,D(1)),('packed_input',pc,pb,original(data['state_scale_bits'])),('three_floats_existing_scaling',regular3,regular_b3,original(data['state_scale_bits'])),('three_floats_power_of_two_scaling',power3,power_b3,D.from_float(pbscale/pcscale))]:
 ac=[c[i] for i in active];gram=[[dot(a,b) for b in ac] for a in ac]
 reactions=solve(gram,[b[i] for i in active]);x=[sum((row[j]*v for row,v in zip(ac,reactions)),D(0))*scale for j in range(width)]
 gaps=[dot(row,x)-bound for row,bound in zip(cols,bounds)]
 defects=[abs(g) if i in active else max(-g,D(0)) for i,g in enumerate(gaps)]
 worst=max(range(rows),key=lambda i:defects[i])
 inverse=[solve(gram,[D(int(i==j)) for i in range(len(active))]) for j in range(len(active))]
 cond=max(sum(map(abs,row)) for row in gram)*max(sum(abs(col[i]) for col in inverse) for i in range(len(active)))
 reports.append(dict(input=name,active_rows=active,original_max_defect=str(defects[worst]),worst_row=worst,min_active_reaction=str(min(reactions)),gram_condition_inf=str(cond)))
result=dict(reports=reports,decimal_precision=80,scope='offline fixed-active-set diagnosis; not nonlinear/global contact admission or runtime performance; rounded Gram is not used in production')
(root/'packing-error-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))

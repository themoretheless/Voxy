"""Original Newton inequalities, binary inputs and 100-digit pointwise audit."""
import json,math,struct,sys
from decimal import Decimal,localcontext
from pathlib import Path
raw=Path(sys.argv[1]).read_bytes();offset=0
def take(fmt):
 global offset
 v=struct.unpack_from(fmt,raw,offset);offset+=struct.calcsize(fmt)
 return v[0] if len(v)==1 else v
def points(n):return [list(take('<3d')) for _ in range(n)]
assert take('<4s')==b'VJN1';count,m,failed=take('<3I');tol=take('<d');rods=[]
for _ in range(count):
 n,q=take('<2I');rods.append({'before':points(n),'angular_before':points(q),'after':points(n),'angular_after':points(q)})
rows=[]
for i in range(m):
 bound,reaction=take('<2d');entries=[]
 for _ in range(4):
  rod,point=take('<2I');gradient=list(take('<3d'));mobility=take('<d');entries.append((rod,point,gradient,mobility))
 def native(side):
  total=0.
  for r,p,g,_ in entries:
   x=rods[r][side][p];total+=((g[0]*x[0]+g[1]*x[1])+g[2]*x[2])
  return total
 with localcontext() as ctx:
  ctx.prec=100
  exact=sum((Decimal.from_float(g[k])*Decimal.from_float(rods[r]['after'][p][k]) for r,p,g,_ in entries for k in range(3)),Decimal(0))-Decimal.from_float(bound)
 ng=native('after')-bound;eg=float(exact)
 bad=lambda gap:abs(gap)>tol if reaction>0 else gap < -tol
 if bad(ng) or bad(eg) or i==failed:
  rows.append({'row':i,'bound':bound,'reaction':reaction,'original_speed':native('before'),'native_candidate_speed':native('after'),'native_gap':ng,'reference_gap':eg,'native_bad':bad(ng),'reference_bad':bad(eg),'entries':[{'rod':r,'point':p,'gradient':g,'before':rods[r]['before'][p],'after':rods[r]['after'][p]} for r,p,g,_ in entries]})
assert offset==len(raw)
r={'scope':'pointwise original candidate admission; no motion/energy/GPU proof','rods':count,'constraints':m,'failed_row':failed,'tolerance':tol,'bytes':len(raw),'native_bad_rows':sum(x['native_bad'] for x in rows),'reference_bad_rows':sum(x['reference_bad'] for x in rows),'maximum_original_angular_norm':max(math.hypot(*p) for rod in rods for p in rod['angular_before']),'maximum_candidate_angular_norm':max(math.hypot(*p) for rod in rods for p in rod['angular_after']),'rows':rows[:32]}
Path(sys.argv[2]).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))

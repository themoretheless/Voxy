from fractions import Fraction as F
from pathlib import Path
import ast,json,sys
rows=[]
for line in Path(sys.argv[1]).read_text().splitlines():
 if not line.startswith('EXACT_GAP_SIGN '):continue
 center,edges,origin,obstacle,axis,result=ast.literal_eval(line.split(' ',1)[1]);axis=list(map(F.from_float,axis))
 gap=abs(sum((F.from_float(c)-F.from_float(o))*n for c,o,n in zip(center,origin,axis)))
 for edge in edges+obstacle:gap-=abs(sum(F.from_float(v)*n for v,n in zip(edge,axis)))
 sign=(gap>0)-(gap<0)
 assert sign==result,(center,axis,sign,result)
 rows.append({'center':center,'axis':[str(v) for v in axis],'gap_numerator':str(gap.numerator),'gap_denominator':str(gap.denominator),'sign':sign})
assert len(rows)==21,len(rows)
print(json.dumps({'passed':True,'arithmetic':'independent arbitrary precision exact rational','cases':rows},indent=2))

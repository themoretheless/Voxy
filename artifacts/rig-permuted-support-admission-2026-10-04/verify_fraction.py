from fractions import Fraction as F
from pathlib import Path
import ast,json,sys
rows=[]
for line in Path(sys.argv[1]).read_text().splitlines():
 if not line.startswith('EXACT_COORDINATE_ROW '):continue
 q,coordinate,source,sign=ast.literal_eval(line.split(' ',1)[1]);x,y,z,w=map(F.from_float,q);n=x*x+y*y+z*z+w*w
 m=[[n-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)], [2*(x*y+z*w),n-2*(x*x+z*z),2*(y*z-x*w)], [2*(x*z-y*w),2*(y*z+x*w),n-2*(x*x+y*y)]]
 expected=[F(sign) if i==source else F(0) for i in range(3)]
 assert [v/n for v in m[coordinate]]==expected,(q,coordinate,source,sign)
 rows.append({'quaternion':q,'coordinate':coordinate,'source':source,'sign':sign})
assert len(rows)==9,len(rows)
print(json.dumps({'passed':True,'arithmetic':'independent arbitrary precision rational matrix','cases':rows},indent=2))

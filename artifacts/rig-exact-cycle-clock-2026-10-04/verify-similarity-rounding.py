from fractions import Fraction as F
from pathlib import Path
import ast, sys
count=0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'SIMILARITY_ROUND_REFERENCE ' not in line: continue
    q,p,scale,offset,actual,error=ast.literal_eval(line.split('SIMILARITY_ROUND_REFERENCE ')[1])
    q=list(map(F,q));p=list(map(F,p));offset=list(map(F,offset));scale=F(scale)
    b=q[:3];w=q[3]
    first=w*w-sum(v*v for v in b)
    second=2*sum(a*c for a,c in zip(p,b))
    cross=[b[1]*p[2]-p[1]*b[2],b[2]*p[0]-p[2]*b[0],b[0]*p[1]-p[0]*b[1]]
    for i in range(3):
        exact=scale*(p[i]*first+b[i]*second+cross[i]*w*2)+offset[i]
        assert abs(F(actual[i])-exact)<=F(error[i])
        count+=1
assert count==6
print('Verified 6 similarity components with exact rational arithmetic; no tolerance.')

from fractions import Fraction as F
from pathlib import Path
import ast,sys
count=0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'RIGID_CYCLE_POINT_REFERENCE ' not in line: continue
    cycle,phase,actual,error=ast.literal_eval(line.split('RIGID_CYCLE_POINT_REFERENCE ')[1])
    u=F(phase);y=u*u*(3-2*u);w=1-y;d=y*y+w*w
    x=u+2-(w*w-y*y)/d
    z=2*w*y/d
    if cycle%2: x,z=5-x,-z
    exact=[x,F(0),z]
    for axis in range(3):
        assert abs(F(actual[axis])-exact[axis])<=F(error[axis]), (cycle,phase,axis)
        count+=1
assert count==153
print('Verified 153 source/runtime point components across 3 cycles over [1/4,3/4], exact rational arithmetic, no tolerance.')

from fractions import Fraction as F
from pathlib import Path
import ast,sys
count=0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'WALL_WORLD_POINT_REFERENCE ' not in line: continue
    time,actual,error=ast.literal_eval(line.split('WALL_WORLD_POINT_REFERENCE ')[1])
    cycle=int(F(time)//1);phase=F(time)-cycle
    u=F(phase);y=u*u*(3-2*u);w=1-y;d=y*y+w*w
    x=u+2-(w*w-y*y)/d
    z=2*w*y/d
    if cycle%2: x,z=5-x,-z
    exact=[F(100000000)+x/2,F(0),z/2]
    for axis in range(3):
        assert abs(F(actual[axis])-exact[axis])<=F(error[axis]), (cycle,phase,axis)
        count+=1
assert count==18
print('Verified 18 wall-clock published components including loop seams, exact rational arithmetic, no tolerance.')

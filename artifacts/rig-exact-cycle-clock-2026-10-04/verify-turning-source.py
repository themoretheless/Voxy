from fractions import Fraction as F
from pathlib import Path
import ast
import sys
count=0
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'SOURCE_TURN_REFERENCE ' not in line: continue
    cycle,translation,rotation=ast.literal_eval(line.split('SOURCE_TURN_REFERENCE ')[1])
    exact=[F(5,2),F(0),F(2 if cycle%2==0 else -2)]
    for bounds,value in zip(translation,exact):
        assert F(bounds[0])<=value<=F(bounds[1])
        count+=1
    signs=[(1,1),(1,-1),(-1,-1),(-1,1)][cycle%4]
    for axis in [0,2]:
        assert F(rotation[axis][0])<=0<=F(rotation[axis][1])
        count+=1
    for axis,sign in zip([1,3],signs):
        lo,hi=map(F,rotation[axis])
        if sign<0: lo,hi=-hi,-lo
        assert hi>=0 and hi*hi>=F(1,2)
        assert lo<=0 or lo*lo<=F(1,2)
        count+=1
assert count==63
print('Verified 63 exact analytic components over 9 turning cycles. Rational comparisons and squared sqrt bounds; no tolerance.')

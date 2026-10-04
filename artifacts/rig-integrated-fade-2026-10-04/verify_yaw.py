"""Verify commuting canonical screw rotation with exact stored binary64 inputs."""
import ast
from fractions import Fraction as F
from pathlib import Path
import sys
lines=Path(sys.argv[1]).read_text().splitlines()
records=[ast.literal_eval(line.split('CERTIFIED_YAW_PHASE ',1)[1]) for line in lines if line.startswith('CERTIFIED_YAW_PHASE ')]
assert len(records)==1
bound,spans=records[0]
phase=sum((F(omega)*(F(end)-F(start)) for start,end,omega in spans),F(0))
assert abs(phase-F(1,2)) <= F(bound)
assert spans[0][0]==0. and spans[-1][1]==1.
assert all(left[1]==right[0] for left,right in zip(spans,spans[1:]))
print('exact canonical yaw phase verified across',len(spans),'spans')

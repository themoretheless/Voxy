"""Canonical translation phase checked using exact stored binary64 values."""
import ast
from fractions import Fraction as F
from pathlib import Path
import sys
lines=Path(sys.argv[1]).read_text().splitlines()
a=[ast.literal_eval(line.split('AUTOMATIC_KEY_PHASE ',1)[1]) for line in lines if line.startswith('AUTOMATIC_KEY_PHASE ')]
b=[ast.literal_eval(line.split('AUTOMATIC_PAIRED_PHASE ',1)[1]) for line in lines if line.startswith('AUTOMATIC_PAIRED_PHASE ')]
assert len(a)==len(b)==1
bound,rounded,spans=a[0]
phase=sum((F(velocity)*(F(end)-F(start)) for start,end,velocity in spans),F(0))
assert abs(phase-F(25,6))<=F(bound)
print('exact target-key phase verified:',len(spans),'spans')
print('rounded-pose comparison gap:',float(abs(F(rounded)-F(25,6))-F(bound)))
bound,spans=b[0]
phase=[sum((F(velocity[axis])*(F(end)-F(start)) for start,end,velocity in spans),F(0)) for axis in range(3)]
expected=[F(17,3),F(0),F(1)]
assert sum(((actual-reference)**2 for actual,reference in zip(phase,expected)),F(0))<=F(bound)**2
print('exact paired-key phase verified:',len(spans),'spans')

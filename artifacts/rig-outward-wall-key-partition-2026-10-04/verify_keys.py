import ast
from fractions import Fraction as F
from pathlib import Path
import sys
records=[ast.literal_eval(line.split('WALL_KEY_GUARD ',1)[1]) for line in Path(sys.argv[1]).read_text().splitlines() if line.startswith('WALL_KEY_GUARD ')]
assert len(records)==1
for key,clip,wall,bounds in records:
    exact=F(key)/F(clip)*F(wall)
    assert F(bounds[0])<=exact<=F(bounds[1])
    assert bounds[0]<bounds[1]
print('exact rational wall-key enclosure verified')

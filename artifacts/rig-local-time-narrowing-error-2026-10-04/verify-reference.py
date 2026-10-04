"""Exact same-time original-key reference including local f64-to-f32 narrowing."""
import json
import struct
import sys
from fractions import Fraction as F


def raw(v):
    return F(struct.unpack('<f',struct.pack('<f',v))[0])


start = list(map(raw,[1e6,-1e6,0.1]))
end = list(map(raw,[-1e6,1e6,-0.2]))
checks = records = 0
for line in open(sys.argv[1],encoding='utf-8'):
    marker = 'source_local_time_reference='
    if marker not in line:
        continue
    d = json.loads(line.split(marker,1)[1])
    t = F(d['time'])
    assert 0 <= t <= F(1,2)
    for axis in range(3):
        ideal = start[axis] + 2*t*(end[axis]-start[axis])
        assert abs(ideal-raw(d['actual'][axis])) <= F(d['caps'][axis]), (records,d)
        checks += 1
    records += 1
assert records == 18 and checks == 54
print(f'Exact original f64-time position checks: {checks}')

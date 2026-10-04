"""Original f32 sampler and chained retarget discrepancies via exact Fractions."""
from fractions import Fraction as F
import json
import struct
import sys


def raw(v):
    return F(struct.unpack('<f', struct.pack('<f', v))[0])


records = source_checks = target_checks = radii = 0
for line in open(sys.argv[1], encoding='utf-8'):
    tag = 'source_linear_sample_reference='
    if tag not in line:
        continue
    d = json.loads(line.split(tag,1)[1])
    t0, t1 = map(raw, d['keys'])
    alpha = (raw(d['time'])-t0)/(t1-t0)
    assert 0 <= alpha <= 1
    position = [raw(a)*(1-alpha)+raw(b)*alpha for a,b in zip(d['from'],d['to'])]
    for ideal, actual, cap in zip(position, d['source_actual'], d['source_caps']):
        assert abs(ideal-raw(actual)) <= F(cap), (records, d)
        source_checks += 1
    x,y,z,w = map(raw, d['basis'])
    norm = x*x+y*y+z*z+w*w
    matrix = [[w*w+x*x-y*y-z*z,2*(x*y-w*z),2*(x*z+w*y)],
              [2*(x*y+w*z),w*w-x*x+y*y-z*z,2*(y*z-w*x)],
              [2*(x*z-w*y),2*(y*z+w*x),w*w-x*x-y*y+z*z]]
    delta = [a-raw(b) for a,b in zip(position,d['source_bind'])]
    total = F(0)
    for axis in range(3):
        ideal = raw(d['target_bind'][axis]) + raw(d['scale']) * sum(
            matrix[axis][i]*delta[i] for i in range(3))/norm
        error = abs(ideal-raw(d['actual'][axis]))
        assert error <= F(d['caps'][axis]), (records,d)
        total += error
        target_checks += 1
    assert total <= F(d['radius']), (records,d)
    records += 1
    radii += 1
assert records == radii == 306 and source_checks == target_checks == 918
print(f'Exact original-sampler axis checks: {source_checks}')
print(f'Exact chained-retarget axis checks: {target_checks}')
print(f'Exact chained-retarget L1 checks: {radii}')

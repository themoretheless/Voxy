import ast, struct
from decimal import Decimal, localcontext
from pathlib import Path

def d(value):
    return Decimal.from_float(value)
def f32(value):
    return struct.unpack('f', struct.pack('f', value))[0]

count = 0
with localcontext() as ctx:
    ctx.prec = 100
    duration = d(f32(0.3))
    out = d(f32(0.1)) * duration / 3
    incoming = d(f32(-0.2)) * duration / 3
    controls = [[Decimal(0), Decimal(0), Decimal(0), Decimal(1)],
                [Decimal(0), out, Decimal(0), Decimal(1)],
                [Decimal(0), Decimal('0.5')-incoming, Decimal(0), Decimal('0.5')],
                [Decimal(0), Decimal('0.5'), Decimal(0), Decimal('0.5')]]
    for line in Path(__file__).with_name('reference-samples.log').read_text().splitlines():
        if 'CUBIC_UNIFORM_SAMPLE ' not in line:
            continue
        phase, actual, bounds = ast.literal_eval(line.split('CUBIC_UNIFORM_SAMPLE ', 1)[1])
        u = d(phase) / duration
        weights = [(1-u)**3, 3*(1-u)**2*u, 3*(1-u)*u*u, u**3]
        raw = [sum(weights[i]*controls[i][axis] for i in range(4)) for axis in range(4)]
        norm = sum(value*value for value in raw).sqrt()
        for axis in range(4):
            error = abs(raw[axis]/norm-d(actual[axis]))
            assert error <= d(bounds[axis]), (phase, axis, error, bounds[axis])
            count += 1
assert count == 132, count
print(f'{count} high precision component checks passed without extra tolerance')

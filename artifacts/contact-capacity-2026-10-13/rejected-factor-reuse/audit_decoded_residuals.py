"""Exact rational audit of decoded f64 coordinates; never a physical admission."""
import json
import struct
import sys
from fractions import Fraction

def scalar(bits):
    return Fraction.from_float(struct.unpack('<d', struct.pack('<Q', bits))[0])

report = json.load(open(sys.argv[1]))
columns = [[scalar(v) for v in row] for row in report['columns_bits']]
bounds = [scalar(v) for v in report['bounds_bits']]
tolerance = scalar(report['tolerance_bits'])
results = {}
for variant in ('cold', 'warm'):
    state = [scalar(v) for v in report[variant + '_state_bits']]
    reactions = [scalar(v) for v in report[variant + '_reactions_bits']]
    violations = []
    residuals = []
    for row, (column, bound, reaction) in enumerate(zip(columns, bounds, reactions)):
        gap = sum((a * b for a, b in zip(column, state)), Fraction()) - bound
        residual = abs(gap) if reaction > 0 else max(-gap, Fraction())
        residuals.append(residual)
        if residual > tolerance:
            violations.append({'row': row, 'residual': float(residual), 'gap': float(gap)})
    results[variant] = {'worst_exact_residual': float(max(residuals)), 'violations': violations}
print(json.dumps({'tolerance': float(tolerance), 'scope': 'exact decoded-coordinate residuals; not physical admission', 'results': results}, indent=2))

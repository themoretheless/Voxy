#!/usr/bin/env python3
"""Exact rational feasibility probe; does not implement GPU arithmetic."""
from fractions import Fraction
import json
from pathlib import Path
import struct
import sys

source, output, report_path = map(Path, sys.argv[1:])
data = source.read_bytes()
assert data[:4] == b'VPA1'
rows, width, original_scale = struct.unpack_from('<IId', data, 4)
records = list(struct.iter_unpack('<ddIII', data[20:]))
assert len(records) == rows * width
values = [(hi, lo, Fraction.from_float(hi) + Fraction.from_float(lo)) for hi, lo, *_ in records]
active = [(i, hi, value) for i, (hi, lo, value) in enumerate(values) if value]
found = None
for shift in range(65):
    scale = original_scale * 2.0**-shift
    if max(abs(hi / scale) for _, hi, _ in active) > 2.0**48:
        break
    exact_scale = Fraction.from_float(scale)
    packed = {}
    for index, hi, value in active:
        residual = value / exact_scale
        words = []
        for _ in range(5):
            word = struct.unpack('<I', struct.pack('<f', float(residual)))[0]
            if word & 0x7f800000 == 0 and word & 0x7fffff:
                break
            term = struct.unpack('<f', struct.pack('<I', word))[0]
            words.append(word)
            residual -= Fraction.from_float(term)
        if len(words) != 5 or residual:
            break
        packed[index] = words
    else:
        found = (shift, scale, packed)
        break
assert found is not None, 'five normal f32 words did not represent this fixture exactly within the existing magnitude bound'
shift, scale, packed = found
with output.open('xb') as stream:
    stream.write(b'VPA2' + struct.pack('<IIdI', rows, width, scale, 5))
    for index, (hi, lo, _) in enumerate(values):
        stream.write(struct.pack('<ddIIIII', hi, lo, *packed.get(index, [0] * 5)))
report = {'scope': 'Exact rational five-f32 coefficient transport feasibility only; no runtime or shader integration.',
          'rows': rows, 'width': width, 'nonzero_coefficients': len(active),
          'scale_shift': shift, 'normal_f32_components_only': True,
          'exact_residual_zero_for_all_coefficients': True,
          'maximum_normalized_high': max(abs(hi / scale) for _, hi, _ in active)}
with report_path.open('x') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(report, indent=2))

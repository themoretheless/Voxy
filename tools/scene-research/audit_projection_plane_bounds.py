#!/usr/bin/env python3
"""Audit GPU positive-plane certificates against exact encoded-input rationals."""
import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('inputs', type=Path)
p.add_argument('reference', type=Path)
p.add_argument('candidate', type=Path)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
source, reference, candidate = (f.read_bytes() for f in [a.inputs, a.reference, a.candidate])
assert len(source) % 112 == 0 and len(candidate) == len(source)//112*240
assert candidate == reference, 'plane admission changed projected/reconstructed bits'
certificates = [0] * 6
checked = skipped = 0
for index, values in enumerate(struct.iter_unpack('<28f', source)):
    matrix, begin, end = values[:16], values[16:19], values[20:23]
    if not all(math.isfinite(v) for v in matrix + begin + end):
        skipped += 1
        continue
    masks = struct.unpack_from('<2f', candidate, index*240+216)
    for point, encoded in zip([begin, end], masks):
        assert math.isfinite(encoded) and encoded == int(encoded) and 0 <= encoded < 64
        mask = int(encoded)
        for plane in range(6):
            if not mask & (1 << plane):
                continue
            axis = min(plane//2, 2)
            total = Fraction(0)
            for column, component in enumerate((*point, 1.0)):
                row = Fraction(matrix[column*4+axis])
                coefficient = row if plane == 4 else Fraction(matrix[column*4+3]) + (-row if plane & 1 else row)
                total += coefficient * Fraction(component)
            assert total > 0, (index, plane, point, total)
            checked += 1
            certificates[plane] += 1
assert checked > 0 and all(v > 0 for v in certificates)
result = dict(cases=len(source)//112, finite_cases=len(source)//112-skipped,
              invalid_input_cases_excluded_from_rational_sign_check=skipped,
              exact_fraction_positive_certificates=checked,
              certificates_per_plane=certificates, false_positive_certificates=0,
              full_projection_reconstruction_records_bit_identical=True,
              output_sha256=hashlib.sha256(candidate).hexdigest(),
              full_street_qualified=False, fps_qualified=False)
a.output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result))

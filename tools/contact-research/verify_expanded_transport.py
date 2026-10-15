#!/usr/bin/env python3
"""Audit VPA1 GPU coefficient packing using exact dyadic rational sums."""
import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct


def audit(path):
    data = Path(path).read_bytes()
    if len(data) < 20 or data[:4] not in (b'VPA1', b'VPA2'):
        raise ValueError('invalid audit header')
    rows, width, scale = struct.unpack_from('<IId', data, 4)
    if data[:4] == b'VPA2' and len(data) < 24:
        raise ValueError('truncated variable-width audit header')
    word_count = 3 if data[:4] == b'VPA1' else struct.unpack_from('<I', data, 20)[0]
    header = 20 if data[:4] == b'VPA1' else 24
    if word_count not in (3, 5) or not rows or not width or len(data) != header + rows * width * (16 + 4 * word_count):
        raise ValueError('invalid audit shape or truncated records')
    if not math.isfinite(scale) or scale <= 0 or math.frexp(scale)[0] != 0.5:
        raise ValueError('scale must be a positive finite power of two')
    exact_scale = Fraction.from_float(scale)
    nonzero = lost = outside = 0
    worst = Fraction(0)
    witnesses = []
    for index, record in enumerate(struct.iter_unpack('<dd' + 'I' * word_count, data[header:])):
        hi, lo, *words = record
        terms = [struct.unpack('<f', struct.pack('<I', w))[0] for w in words]
        if not all(map(math.isfinite, [hi, lo, *terms])) or hi + lo != hi:
            raise ValueError('nonfinite or unnormalized coefficient')
        if hi == lo == 0 and all(t == 0 for t in terms):
            continue
        source = Fraction.from_float(hi) + Fraction.from_float(lo)
        decoded = sum(map(Fraction.from_float, terms), Fraction(0)) * exact_scale
        nonzero += source != 0
        error = abs(decoded - source)
        if not error:
            continue
        lost += 1
        outside += error > abs(Fraction.from_float(hi)) / (1 << 68)
        relative = error / abs(source) if source else Fraction(0)
        worst = max(worst, relative)
        if len(witnesses) < 16:
            witnesses.append({'row': index // width, 'coordinate': index % width,
                              'source_f64_bits': [struct.unpack('<Q', struct.pack('<d', v))[0] for v in (hi, lo)],
                              'packed_f32_bits': list(words),
                              'exact_error_numerator': str(error.numerator),
                              'exact_error_denominator': str(error.denominator),
                              'relative_error_approx': float(relative)})
    return {'scope': 'Exact rational input transport audit; no GPU arithmetic, solution admission, frame or FPS proof.',
            'sha256': hashlib.sha256(data).hexdigest(), 'rows': rows, 'width': width,
            'coefficient_count': rows * width, 'f32_words_per_coefficient': word_count, 'nonzero_coefficients': nonzero,
            'coefficients_with_lost_bits': lost, 'exact_sum_preserved_for_all_coefficients': lost == 0,
            'coefficients_exceeding_current_2_pow_minus_68_bound': outside,
            'worst_relative_error_approx': float(worst),
            'worst_relative_error_exact': [str(worst.numerator), str(worst.denominator)],
            'witnesses': witnesses}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input')
    parser.add_argument('output')
    args = parser.parse_args()
    report = audit(args.input)
    with open(args.output, 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ('witnesses', 'worst_relative_error_exact')}, indent=2))

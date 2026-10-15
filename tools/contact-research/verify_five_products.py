#!/usr/bin/env python3
"""Exact rational verification of real GPU FMA product/error pairs."""
from fractions import Fraction
import json
import math
from pathlib import Path
import struct
import sys


def exact_word(word):
    value = struct.unpack('<f', struct.pack('<I', word))[0]
    if not math.isfinite(value):
        raise ValueError('nonfinite GPU output')
    return Fraction.from_float(value)


def verify(path, require_rounded=False):
    records = json.loads(Path(path).read_text())['records']
    if not records:
        raise ValueError('empty arithmetic qualification')
    raw_failed = normalized_failed = bound_failed = zero_cases = 0
    worst_raw = worst_normalized = Fraction(0)
    rounded_count = rounded_failed = rounded_bound_failed = rounded_bound_inadequate = 0
    worst_rounded = worst_rounded_bound = Fraction(0)
    results = []
    for index, row in enumerate(records):
        if len(row['input_a_words']) != 5 or len(row['input_b_words']) != 5 or len(row['raw_product_words']) != 50 or len(row['normalized_product_words']) != 50:
            raise ValueError('invalid arithmetic shape')
        a = sum(map(exact_word, row['input_a_words']), Fraction(0))
        b = sum(map(exact_word, row['input_b_words']), Fraction(0))
        expected = a * b
        zero_cases += expected == 0
        exponent = row['normalized_exponent']
        if not isinstance(exponent, int) or abs(exponent) > 2048:
            raise ValueError('invalid result exponent')
        scale = Fraction(2)**exponent
        raw = sum(map(exact_word, row['raw_product_words']), Fraction(0))
        normalized = sum(map(exact_word, row['normalized_product_words']), Fraction(0)) * scale
        bound = exact_word(row['normalized_error_bound_word']) * scale
        if bound < 0:
            raise ValueError('negative error certificate')
        raw_error = abs(raw - expected)
        normalized_error = abs(normalized - expected)
        raw_relative = raw_error / abs(expected) if expected else Fraction(raw_error != 0)
        normalized_relative = normalized_error / abs(expected) if expected else Fraction(normalized_error != 0)
        raw_failed += raw_relative > Fraction(1, 1 << 106)
        normalized_failed += normalized_relative > Fraction(1, 1 << 106)
        bound_failed += normalized_error > bound
        worst_raw = max(worst_raw, raw_relative)
        worst_normalized = max(worst_normalized, normalized_relative)
        if 'rounded_product_words' in row:
            if len(row['rounded_product_words']) != 5:
                raise ValueError('invalid five-component result shape')
            rounded_exponent = row['rounded_exponent']
            if not isinstance(rounded_exponent, int) or abs(rounded_exponent) > 2048:
                raise ValueError('invalid rounded exponent')
            rounded_scale = Fraction(2)**rounded_exponent
            rounded = sum(map(exact_word, row['rounded_product_words']), Fraction(0)) * rounded_scale
            rounded_bound = exact_word(row['rounded_error_bound_word']) * rounded_scale
            if rounded_bound < 0:
                raise ValueError('negative rounded error certificate')
            rounded_error = abs(rounded - expected)
            rounded_relative = rounded_error / abs(expected) if expected else Fraction(rounded_error != 0)
            rounded_bound_relative = rounded_bound / abs(expected) if expected else Fraction(rounded_bound != 0)
            rounded_count += 1
            rounded_failed += rounded_relative > Fraction(1, 1 << 106)
            rounded_bound_failed += rounded_error > rounded_bound
            rounded_bound_inadequate += rounded_bound_relative > Fraction(1, 1 << 106)
            worst_rounded = max(worst_rounded, rounded_relative)
            worst_rounded_bound = max(worst_rounded_bound, rounded_bound_relative)
        results.append({'case': index, 'raw_relative_error': float(raw_relative),
                        'normalized_relative_error': float(normalized_relative),
                        'normalized_absolute_error': [str(normalized_error.numerator), str(normalized_error.denominator)],
                        'error_within_primitive_bound': normalized_error <= bound})
    return {'scope': 'Selected real coefficient products and optional five-component compression on GPU; not division, QR or frame qualification.',
            'cases': len(records), 'exact_zero_cases': zero_cases, 'target_relative_error': '2^-106',
            'raw_cases_exceeding_target': raw_failed,
            'normalized_cases_exceeding_target': normalized_failed,
            'normalized_cases_exceeding_primitive_bound': bound_failed,
            'worst_raw_relative_error': float(worst_raw),
            'worst_normalized_relative_error': float(worst_normalized),
            'compressed_products_checked': rounded_count,
            'compressed_cases_exceeding_target': rounded_failed,
            'compressed_cases_exceeding_error_bound': rounded_bound_failed,
            'compressed_cases_with_inadequate_error_bound': rounded_bound_inadequate,
            'worst_compressed_relative_error': float(worst_rounded),
            'worst_compressed_relative_error_bound': float(worst_rounded_bound),
            'status': 'passed' if normalized_failed == bound_failed == rounded_failed == rounded_bound_failed == rounded_bound_inadequate == 0 and (not require_rounded or rounded_count == len(records)) else 'failed', 'results': results}


if __name__ == '__main__':
    report = verify(sys.argv[1], require_rounded="--require-rounded" in sys.argv[3:])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'results'}, indent=2))
    if report['status'] != 'passed':
        sys.exit(1)

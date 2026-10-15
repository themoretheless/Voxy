#!/usr/bin/env python3
"""Exact endpoint verification of GPU five-component interval multiplication."""
from fractions import Fraction
import json
from pathlib import Path
import sys
from verify_five_products import exact_word


def normalized_exponent(words):
    leading = max(word & 0x7fffffff for word in words)
    return ((leading >> 23) & 255) - 127 if leading else 0


def verify(path):
    records = json.loads(Path(path).read_text())['records']
    if not records:
        raise ValueError('empty bounded multiplication qualification')
    accepted = invalid = underestimated = rejected_precision = invalid_admitted = 0
    radii = set()
    for row in records:
        a_words, b_words = row['input_a_words'], row['input_b_words']
        if len(a_words) != 5 or len(b_words) != 5 or len(row['bounded_product_words']) != 5:
            raise ValueError('invalid operand or product shape')
        ea = exact_word(row['input_error_a_word'])
        eb = exact_word(row['input_error_b_word'])
        valid = row['bounded_product_valid']
        if valid not in (0, 1):
            raise ValueError('invalid range-valid flag')
        if ea < 0 or eb < 0:
            invalid += 1
            invalid_admitted += valid != 0
            continue
        if valid != 1:
            raise ValueError('finite selected uncertainty interval rejected')
        a = sum(map(exact_word, a_words), Fraction(0))
        b = sum(map(exact_word, b_words), Fraction(0))
        radius_a = ea * Fraction(2)**normalized_exponent(a_words)
        radius_b = eb * Fraction(2)**normalized_exponent(b_words)
        radii.add((row['input_error_a_word'], row['input_error_b_word']))
        exponent = row['bounded_product_exponent']
        if not isinstance(exponent, int) or abs(exponent) > 2048:
            raise ValueError('invalid product exponent')
        scale = Fraction(2)**exponent
        result = sum(map(exact_word, row['bounded_product_words']), Fraction(0)) * scale
        bound = exact_word(row['bounded_product_error_bound_word']) * scale
        if bound < 0:
            raise ValueError('negative output error bound')
        # Bilinear extrema on a rectangle occur at its four corners.
        endpoints = [(a + da*radius_a) * (b + db*radius_b) for da in (-1, 1) for db in (-1, 1)]
        needed = max(abs(result - endpoint) for endpoint in endpoints)
        underestimated += needed > bound
        rejected_precision += bound > Fraction(1, 1 << 106) * max(abs(result) - bound, Fraction(0))
        accepted += 1
    return {'scope': 'Selected independent input-error rectangles checked at all four exact corners; not arbitrary-domain or chained contact QR proof.',
            'interval_products_checked': accepted, 'input_radius_combinations': len(radii),
            'invalid_input_radii_checked': invalid, 'invalid_input_radii_admitted': invalid_admitted,
            'output_intervals_underestimated': underestimated,
            'output_intervals_insufficient_for_relative_2_pow_minus_106': rejected_precision,
            'status': 'passed' if underestimated == invalid_admitted == 0 else 'failed'}


if __name__ == '__main__':
    report = verify(sys.argv[1])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps(report, indent=2))
    if report['status'] != 'passed':
        sys.exit(1)

#!/usr/bin/env python3
"""Exact rational quotient and endpoint-interval verification of GPU division."""
from fractions import Fraction
import json
from pathlib import Path
import sys
from verify_five_products import exact_word
from verify_five_bounded_products import normalized_exponent


def output(row, prefix):
    words = row[prefix + '_words']
    exponent = row[prefix + '_exponent']
    if len(words) != 5 or not isinstance(exponent, int) or abs(exponent) > 2048:
        raise ValueError('invalid division result')
    scale = Fraction(2)**exponent
    value = sum(map(exact_word, words), Fraction(0)) * scale
    bound = exact_word(row[prefix + '_error_bound_word']) * scale
    if bound < 0:
        raise ValueError('negative division bound')
    return value, bound


def verify(path):
    records = json.loads(Path(path).read_text())['records']
    if not records:
        raise ValueError('empty division qualification')
    exact_count = exact_rejected = exact_failed = exact_bound_failed = exact_weak_bound = 0
    intervals = invalid_intervals = invalid_admitted = interval_underestimated = interval_weak = 0
    worst_error = worst_bound = Fraction(0)
    for row in records:
        a_words, b_words = row['input_a_words'], row['input_b_words']
        if len(a_words) != 5 or len(b_words) != 5:
            raise ValueError('invalid division input shape')
        a = sum(map(exact_word, a_words), Fraction(0))
        b = sum(map(exact_word, b_words), Fraction(0))
        if not b:
            exact_rejected += 1
            invalid_admitted += row['exact_division_valid'] != 0
        else:
            if row['exact_division_valid'] != 1:
                raise ValueError('selected finite nonzero exact denominator rejected')
            value, bound = output(row, 'exact_division')
            expected = a / b
            error = abs(value - expected)
            relative_error = error / abs(expected) if expected else Fraction(error != 0)
            relative_bound = bound / abs(expected) if expected else Fraction(bound != 0)
            exact_failed += relative_error > Fraction(1, 1 << 106)
            exact_bound_failed += error > bound
            exact_weak_bound += relative_bound > Fraction(1, 1 << 106)
            worst_error = max(worst_error, relative_error)
            worst_bound = max(worst_bound, relative_bound)
            exact_count += 1
        ea = exact_word(row['input_error_a_word'])
        eb = exact_word(row['input_error_b_word'])
        radius_a = ea * Fraction(2)**normalized_exponent(a_words)
        radius_b = eb * Fraction(2)**normalized_exponent(b_words)
        domain_invalid = ea < 0 or eb < 0 or b - radius_b <= 0 <= b + radius_b
        if domain_invalid:
            invalid_intervals += 1
            invalid_admitted += row['bounded_division_valid'] != 0
            continue
        if row['bounded_division_valid'] != 1:
            raise ValueError('selected finite denominator interval excluding zero rejected')
        value, bound = output(row, 'bounded_division')
        corners = [(a + da*radius_a)/(b + db*radius_b) for da in (-1, 1) for db in (-1, 1)]
        needed = max(abs(value - endpoint) for endpoint in corners)
        interval_underestimated += needed > bound
        interval_weak += bound > Fraction(1, 1 << 106) * max(abs(value) - bound, Fraction(0))
        intervals += 1
    return {'scope': 'Selected real exact and uncertain quotients with exact rational endpoint verification; not square root, QR, arbitrary error chains or trajectory qualification.',
            'exact_quotients_checked': exact_count, 'exact_zero_denominators_rejected': exact_rejected,
            'exact_quotients_exceeding_relative_2_pow_minus_106': exact_failed,
            'exact_quotients_exceeding_error_bound': exact_bound_failed,
            'exact_quotients_with_inadequate_error_bound': exact_weak_bound,
            'worst_exact_relative_error': float(worst_error), 'worst_exact_relative_error_bound': float(worst_bound),
            'valid_uncertain_quotient_intervals_checked': intervals,
            'invalid_input_or_zero_crossing_intervals_checked': invalid_intervals,
            'invalid_domains_admitted': invalid_admitted,
            'uncertain_intervals_underestimated': interval_underestimated,
            'uncertain_intervals_insufficient_for_2_pow_minus_106': interval_weak,
            'status': 'passed' if exact_failed == exact_bound_failed == exact_weak_bound == invalid_admitted == interval_underestimated == 0 else 'failed'}


if __name__ == '__main__':
    report = verify(sys.argv[1])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps(report, indent=2))
    if report['status'] != 'passed':sys.exit(1)

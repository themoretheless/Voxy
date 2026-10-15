#!/usr/bin/env python3
"""Exact rational squared-endpoint verification; Decimal is reporting only."""
from decimal import Decimal, localcontext
from fractions import Fraction
import json
from pathlib import Path
import sys
from verify_five_products import exact_word
from verify_five_bounded_products import normalized_exponent
from verify_five_division import output


def covers_root(value, bound, low, high):
    if value < 0 or bound < 0:
        return False
    return max(value-bound, Fraction(0))**2 <= low and (value+bound)**2 >= high


def verify(path):
    records = json.loads(Path(path).read_text())['records']
    if not records:
        raise ValueError('empty square-root qualification')
    exact_count = exact_zeros = invalid_exact = invalid_admitted = 0
    exact_failed = exact_bound_failed = exact_weak = 0
    interval_count = boundary_count = invalid_intervals = interval_underestimated = interval_weak = 0
    worst_error = worst_bound = 0.0
    epsilon = Fraction(1, 1 << 106)
    for row in records:
        words = row['input_a_words']
        if len(words) != 5:
            raise ValueError('invalid sqrt operand shape')
        a = sum(map(exact_word, words), Fraction(0))
        if a < 0:
            invalid_exact += 1
            invalid_admitted += row['exact_sqrt_valid'] != 0
        else:
            if row['exact_sqrt_valid'] != 1:
                raise ValueError('selected nonnegative exact square root rejected')
            value, bound = output(row, 'exact_sqrt')
            if value < 0:
                raise ValueError('negative square-root candidate')
            exact_bound_failed += not covers_root(value, bound, a, a)
            if a == 0:
                exact_zeros += 1
                exact_failed += value != 0
                exact_weak += bound != 0
            else:
                exact_failed += not (value**2/(1+epsilon)**2 <= a <= value**2/(1-epsilon)**2)
                exact_weak += bound**2 > epsilon**2 * a
                with localcontext() as context:
                    context.prec = 120
                    root = (Decimal(a.numerator)/Decimal(a.denominator)).sqrt()
                    candidate = Decimal(value.numerator)/Decimal(value.denominator)
                    estimated = Decimal(bound.numerator)/Decimal(bound.denominator)
                    worst_error = max(worst_error, float(abs(candidate-root)/root))
                    worst_bound = max(worst_bound, float(estimated/root))
            exact_count += 1
        radius = exact_word(row['input_error_a_word']) * Fraction(2)**normalized_exponent(words)
        if radius < 0 or a - radius < 0:
            invalid_intervals += 1
            invalid_admitted += row['bounded_sqrt_valid'] != 0
            continue
        if row['bounded_sqrt_valid'] != 1:
            raise ValueError('selected valid square-root interval rejected, including its zero boundary')
        value, bound = output(row, 'bounded_sqrt')
        low, high = a-radius, a+radius
        interval_underestimated += not covers_root(value, bound, low, high)
        interval_weak += bound**2 > epsilon**2 * low
        boundary_count += low == 0
        interval_count += 1
    return {'scope': 'Selected five-component GPU square roots; exact squared rational acceptance. Decimal statistics do not control acceptance; not QR or trajectory qualification.',
            'exact_nonnegative_roots_checked': exact_count, 'exact_zeros_checked': exact_zeros,
            'negative_exact_inputs_rejected': invalid_exact, 'invalid_domains_admitted': invalid_admitted,
            'exact_roots_exceeding_relative_2_pow_minus_106': exact_failed,
            'exact_roots_exceeding_error_bound': exact_bound_failed,
            'exact_roots_with_inadequate_error_bound': exact_weak,
            'worst_relative_error_approx': worst_error, 'worst_relative_error_bound_approx': worst_bound,
            'valid_input_intervals_checked': interval_count, 'valid_intervals_touching_zero_checked': boundary_count,
            'negative_crossing_or_invalid_input_intervals_rejected': invalid_intervals,
            'interval_root_bounds_underestimated': interval_underestimated,
            'wide_intervals_insufficient_for_relative_precision': interval_weak,
            'status': 'passed' if exact_failed == exact_bound_failed == exact_weak == invalid_admitted == interval_underestimated == 0 else 'failed'}


if __name__ == '__main__':
    report = verify(sys.argv[1])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps(report, indent=2))
    if report['status'] != 'passed':sys.exit(1)

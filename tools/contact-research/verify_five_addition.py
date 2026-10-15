#!/usr/bin/env python3
"""Exact rational addition/subtraction oracle for five-component GPU words."""
from fractions import Fraction
import json
from pathlib import Path
import sys
from verify_five_products import exact_word


def verify(path, require_propagated=False):
    records = json.loads(Path(path).read_text())['records']
    if not records:
        raise ValueError('empty addition qualification')
    operations = exact_zero = errors = bounds = inadequate = 0
    worst_error = worst_bound = Fraction(0)
    propagated_count = propagated_underestimated = propagated_rejected = 0
    details = []
    for index, row in enumerate(records):
        if len(row['input_a_words']) != 5 or len(row['input_b_words']) != 5:
            raise ValueError('invalid operand shape')
        a = sum(map(exact_word, row['input_a_words']), Fraction(0))
        b = sum(map(exact_word, row['input_b_words']), Fraction(0))
        if 'propagated_words' in row:
            words = row['propagated_words']
            exponent = row['propagated_exponent']
            if len(words) != 5 or not isinstance(exponent, int) or abs(exponent) > 2048:
                raise ValueError('invalid propagated result')
            def source_error(source_words):
                leading = max(w & 0x7fffffff for w in source_words)
                exponent = ((leading >> 23) & 255) - 127 if leading else 0
                return Fraction(2)**(exponent - 110)
            scale = Fraction(2)**exponent
            result = sum(map(exact_word, words), Fraction(0)) * scale
            estimate = exact_word(row['propagated_error_bound_word']) * scale
            if estimate < 0:
                raise ValueError('negative propagated estimate')
            required = abs(result - (a + b)) + source_error(row['input_a_words']) + source_error(row['input_b_words'])
            propagated_count += 1
            propagated_underestimated += estimate < required
            # A precision gate must use a lower bound on the true magnitude.
            propagated_rejected += estimate > Fraction(1, 1 << 106) * max(abs(result) - estimate, Fraction(0))
        for op, expected in [('addition', a + b), ('subtraction', a - b)]:
            words = row[op + '_words']
            exponent = row[op + '_exponent']
            if len(words) != 5 or not isinstance(exponent, int) or abs(exponent) > 2048:
                raise ValueError('invalid result shape or exponent')
            scale = Fraction(2)**exponent
            value = sum(map(exact_word, words), Fraction(0)) * scale
            bound = exact_word(row[op + '_error_bound_word']) * scale
            if bound < 0:
                raise ValueError('negative error estimate')
            error = abs(value - expected)
            relative_error = error / abs(expected) if expected else Fraction(error != 0)
            relative_bound = bound / abs(expected) if expected else Fraction(bound != 0)
            errors += relative_error > Fraction(1, 1 << 106)
            bounds += error > bound
            inadequate += relative_bound > Fraction(1, 1 << 106)
            exact_zero += expected == 0
            operations += 1
            worst_error = max(worst_error, relative_error)
            worst_bound = max(worst_bound, relative_bound)
            details.append({'case': index, 'operation': op, 'expected_zero': expected == 0,
                            'relative_error': float(relative_error), 'relative_error_bound': float(relative_bound),
                            'error_within_bound': error <= bound})
    return {'scope': 'Selected real and cancellation operands for GPU addition/subtraction and optional fixed 2^-110 normalized input error intervals; not arbitrary errors or contact QR qualification.',
            'operations': operations, 'exact_zero_operations': exact_zero,
            'relative_error_gate': '2^-106', 'operations_exceeding_gate': errors,
            'operations_exceeding_bound': bounds, 'operations_with_inadequate_bound': inadequate,
            'worst_relative_error': float(worst_error), 'worst_relative_error_bound': float(worst_bound),
            'propagated_error_cases_checked': propagated_count,
            'propagated_error_cases_underestimated': propagated_underestimated,
            'propagated_error_cases_rejected_by_precision_gate': propagated_rejected,
            'status': 'passed' if errors == bounds == inadequate == propagated_underestimated == 0 and (not require_propagated or propagated_count == len(records)) else 'failed', 'details': details}


if __name__ == '__main__':
    report = verify(sys.argv[1], require_propagated="--require-propagated" in sys.argv[3:])
    with open(sys.argv[2], 'x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'details'}, indent=2))
    if report['status'] != 'passed':
        sys.exit(1)

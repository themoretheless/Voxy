#!/usr/bin/env python3
"""Independent integer rounding oracle for the isolated repeated-divisor GPU probe."""
import argparse, hashlib, json, struct
from pathlib import Path

def ratio_bits(a, b):
    sign = (a ^ b) & 0x80000000
    ea, eb = (a >> 23) & 255, (b >> 23) & 255
    ma, mb = a & 0x7fffff, b & 0x7fffff
    if ea == 255 or eb == 255:
        if (ea == 255 and ma) or (eb == 255 and mb) or ea == eb == 255:
            return None
        return sign | (0x7f800000 if ea == 255 else 0)
    ma |= 0x800000 if ea else 0
    mb |= 0x800000 if eb else 0
    if not mb:
        return None if not ma else sign | 0x7f800000
    if not ma:
        return sign
    exponent = (ea - 150 if ea else -149) - (eb - 150 if eb else -149)
    n, d = ma, mb
    if exponent >= 0:
        n <<= exponent
    else:
        d <<= -exponent
    k = n.bit_length() - d.bit_length()
    if (n < d << k) if k >= 0 else (n << -k < d):
        k -= 1
    if k > 127:
        return sign | 0x7f800000
    shift = 149 if k < -126 else 23 - k
    nn, dd = (n << shift, d) if shift >= 0 else (n, d << -shift)
    q, rem = divmod(nn, dd)
    if rem * 2 > dd or (rem * 2 == dd and q & 1):
        q += 1
    if k < -126:
        return sign | q
    if q == 1 << 24:
        q >>= 1
        k += 1
    return sign | (0x7f800000 if k > 127 else ((k + 127) << 23) | (q & 0x7fffff))

def audit(root):
    raw_inputs, raw_outputs = (root / 'inputs.bin').read_bytes(), (root / 'outputs.bin').read_bytes()
    assert len(raw_inputs) % 32 == 0
    count = len(raw_inputs) // 32
    assert len(raw_outputs) == count * 64
    totals = dict(cases=count, certified_reciprocals=0, forced_reciprocals=0,
                  forced_reciprocals_rejected=0, scalar_fast=0, pair_main_fast=0,
                  pair_correction_fast=0, certificate_errors=0, integer_product_errors=0,
                  fast_rounding_errors=0, native_fast_rounding_errors=0,
                  fast_native_bit_mismatches=0, fallback_native_bit_mismatches=0,
                  complete_pair_bit_mismatches=0, native_rounding_mismatches_all=0)
    examples = []
    for index, (c, out) in enumerate(zip(struct.iter_unpack('<8I', raw_inputs), struct.iter_unpack('<16I', raw_outputs))):
        a, b, _, forced, *_ = c
        native, actual, reciprocal, certified = out[:4]
        original_pair, actual_pair = out[4:6], out[6:8]
        fast, main_fast, correction_fast, identity = out[8:12]
        low, high, left, right = out[12:16]
        assert identity == index and certified in (0, 1) and fast in (0, 1)
        totals['integer_product_errors'] += (low | high << 32) != left * right
        exact_reciprocal = ratio_bits(0x3f800000, b)
        exact = ratio_bits(a, b)
        totals['certified_reciprocals'] += certified
        totals['certificate_errors'] += bool(certified and reciprocal != exact_reciprocal)
        totals['forced_reciprocals'] += bool(forced)
        totals['forced_reciprocals_rejected'] += bool(forced and not certified)
        totals['scalar_fast'] += fast
        totals['pair_main_fast'] += main_fast
        totals['pair_correction_fast'] += correction_fast
        totals['fast_rounding_errors'] += bool(fast and actual != exact)
        totals['native_fast_rounding_errors'] += bool(fast and native != exact)
        totals['native_rounding_mismatches_all'] += bool(exact is not None and native != exact)
        totals['fast_native_bit_mismatches'] += bool(fast and native != actual)
        totals['fallback_native_bit_mismatches'] += bool(not fast and native != actual)
        totals['complete_pair_bit_mismatches'] += original_pair != actual_pair
        if len(examples) < 8 and ((fast and actual != exact) or original_pair != actual_pair or (certified and reciprocal != exact_reciprocal)):
            examples.append(dict(index=index, input_words=[hex(v) for v in c], output_words=[hex(v) for v in out], exact_scalar=None if exact is None else hex(exact)))
    safe = all(totals[k] == 0 for k in ('certificate_errors', 'integer_product_errors', 'fast_rounding_errors', 'fast_native_bit_mismatches', 'fallback_native_bit_mismatches', 'complete_pair_bit_mismatches'))
    assert totals['scalar_fast'] > 10000 and totals['forced_reciprocals_rejected'] > 10000
    return dict(status='bounded_arithmetic_passed' if safe else 'arithmetic_not_admitted', **totals,
                examples=examples, input_sha256=hashlib.sha256(raw_inputs).hexdigest(),
                output_sha256=hashlib.sha256(raw_outputs).hexdigest(),
                oracle='integer quotient and remainder, ties to even; no floating arithmetic',
                native_errors_outside_fast_domain_are_observations=True,
                production_connected=False, universal_backend_proof=False, fps_qualified=False)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.root)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
    raise SystemExit(0 if result['status'] == 'bounded_arithmetic_passed' else 1)

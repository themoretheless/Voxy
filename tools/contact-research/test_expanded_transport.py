import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('transport', Path(__file__).with_name('verify_expanded_transport.py'))
transport = importlib.util.module_from_spec(spec)
spec.loader.exec_module(transport)


def word(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


class ExactTransportAuditTests(unittest.TestCase):
    def run_audit(self, records, suffix=b''):
        data = b'VPA1' + struct.pack('<IId', 1, len(records), 1.0)
        data += b''.join(struct.pack('<ddIII', hi, lo, *map(word, terms)) for hi, lo, terms in records)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'input.bin'
            path.write_bytes(data + suffix)
            return transport.audit(path)

    def test_detects_low_part_loss_hidden_by_rounded_f64_comparison(self):
        self.assertEqual(1.0 + 2.0**-100, 1.0)
        report = self.run_audit([(1.0, 2.0**-100, [1.0, 0.0, 0.0])])
        self.assertEqual(report['coefficients_with_lost_bits'], 1)
        witness = report['witnesses'][0]
        self.assertEqual(witness['exact_error_numerator'], '1')
        self.assertEqual(witness['exact_error_denominator'], str(1 << 100))
        self.assertEqual(report['coefficients_exceeding_current_2_pow_minus_68_bound'], 0)

    def test_accepts_exact_tail_and_zero(self):
        report = self.run_audit([(1.0, 2.0**-60, [1.0, 2.0**-60, 0.0]), (0.0, 0.0, [0.0, 0.0, 0.0])])
        self.assertTrue(report['exact_sum_preserved_for_all_coefficients'])
        self.assertEqual(report['nonzero_coefficients'], 1)

    def test_detects_error_outside_existing_bound(self):
        report = self.run_audit([(1.0, 2.0**-60, [1.0, 0.0, 0.0])])
        self.assertEqual(report['coefficients_exceeding_current_2_pow_minus_68_bound'], 1)

    def test_five_word_format_retains_a_tail_invisible_in_f64_sum(self):
        data = b'VPA2' + struct.pack('<IIdI', 1, 1, 1.0, 5)
        data += struct.pack('<ddIIIII', 1.0, 2.0**-100, word(1.0), word(2.0**-100), 0, 0, 0)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'five.bin'
            path.write_bytes(data)
            report = transport.audit(path)
            self.assertEqual(report['f32_words_per_coefficient'], 5)
            self.assertTrue(report['exact_sum_preserved_for_all_coefficients'])
            path.write_bytes(data[:20])
            with self.assertRaises(ValueError):
                transport.audit(path)

    def test_rejects_extra_truncated_and_nonfinite_data(self):
        with self.assertRaises(ValueError):
            self.run_audit([(1.0, 0.0, [1.0, 0.0, 0.0])], b'bad')
        with self.assertRaises(ValueError):
            self.run_audit([(float('inf'), 0.0, [1.0, 0.0, 0.0])])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'truncated.bin'
            path.write_bytes(b'VPA1' + struct.pack('<IId', 1, 1, 1.0))
            with self.assertRaises(ValueError):
                transport.audit(path)


if __name__ == '__main__':
    unittest.main()

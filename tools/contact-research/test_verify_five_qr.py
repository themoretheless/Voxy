import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest
from verify_five_qr import verify


def bits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def runtime(v):
    return {'words': [struct.unpack('<I', struct.pack('<f', v))[0], 0, 0, 0, 0], 'exponent': 0, 'error_bound_word': 0}


class QrCertificates(unittest.TestCase):
    def setUp(self):
        self.data = {'active_rows': [0], 'width': 1,
                     'column_parts_bits': [[[bits(1.), bits(0.)]]],
                     'bounds_bits': [bits(2.)], 'state_scale_bits': bits(1.),
                     'reaction_scale_bits': bits(1.), 'basis': [runtime(1.)],
                     'triangular': [runtime(1.)], 'coordinates': [runtime(2.)],
                     'reactions': [runtime(2.)]}

    def report(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'candidate.json'
            path.write_text(json.dumps(data))
            return verify(path)

    def test_exact_identity(self):
        self.assertEqual(self.report(self.data)['status'], 'passed')

    def test_each_corrupted_output_is_rejected(self):
        for key in ['basis', 'triangular', 'coordinates', 'reactions']:
            with self.subTest(key=key):
                data = copy.deepcopy(self.data)
                data[key][0] = runtime(3.)
                self.assertEqual(self.report(data)['status'], 'failed')

    def test_original_low_component_is_checked(self):
        self.data['column_parts_bits'][0][0][1] = bits(2.**-60)
        self.assertEqual(self.report(self.data)['status'], 'failed')

    def test_nonfinite_and_bad_shapes_are_rejected(self):
        for key, replacement in [('basis', []), ('coordinates', [runtime(float('inf'))])]:
            data = copy.deepcopy(self.data)
            data[key] = replacement
            with self.assertRaises(ValueError):
                self.report(data)

    def test_negative_certificate_is_rejected(self):
        self.data['basis'][0]['error_bound_word'] = struct.unpack('<I', struct.pack('<f', -1.))[0]
        with self.assertRaises(ValueError):
            self.report(self.data)

import json
from pathlib import Path
import tempfile
import unittest
from test_verify_five_qr import bits,runtime
from verify_five_gaps import verify

class GapCertificates(unittest.TestCase):
    def report(self,low=0.,residual=0.,decision=None):
        data={'rows':1,'width':1,'column_parts_bits':[[[bits(1.),bits(low)]]], 'bounds_bits':[bits(1.)], 'cases':[{'name':'fixture','state_parts_bits':[[bits(1.),bits(0.)]],'residual_scale_bits':bits(1.),'residuals':[runtime(residual)]}]}
        if decision is not None:
            data['tolerance_bits']=bits(2.**-61);data['cases'][0]['decisions']=[decision]
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'gaps.json';path.write_text(json.dumps(data));return verify(path)
    def test_exact_zero(self):self.assertEqual(self.report()['status'],'passed')
    def test_erased_low_component_rejected(self):self.assertEqual(self.report(2.**-60)['status'],'failed')
    def test_corrupted_residual_rejected(self):self.assertEqual(self.report(residual=1.)['status'],'failed')

    def test_false_violation_is_rejected(self):self.assertEqual(self.report(decision=1)['status'],'failed')
    def test_uncertain_never_claims_a_certificate(self):self.assertEqual(self.report(decision=4)['status'],'passed')
    def test_invalid_decision_is_rejected(self):
        with self.assertRaises(ValueError):self.report(decision=0)

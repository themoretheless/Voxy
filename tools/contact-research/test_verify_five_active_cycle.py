import json
from pathlib import Path
import tempfile
import unittest
from test_verify_five_qr import bits,runtime
from verify_five_active_cycle import verify
class ActiveCycleCertificates(unittest.TestCase):
    def report(self,state=1.,reaction=1.,tolerance=1e-14):
        result={'original_rows':1,'width':1,'original_tolerance_bits':bits(tolerance),'active_rows':[0],'coordinates':[runtime(state)],'reactions':[runtime(reaction)]}
        operator={'column_parts_bits':[[[bits(1.),bits(0.)]]],'bounds_bits':[bits(1.)],'tolerance_bits':bits(1e-14)}
        with tempfile.TemporaryDirectory() as directory:
            a=Path(directory)/'result.json';b=Path(directory)/'operator.json';a.write_text(json.dumps(result));b.write_text(json.dumps(operator));return verify(a,b)
    def test_exact_solution(self):self.assertEqual(self.report()['status'],'passed')
    def test_invalid_primal_is_rejected(self):self.assertEqual(self.report(state=1.5)['status'],'failed')
    def test_invalid_equilibrium_is_rejected(self):self.assertEqual(self.report(reaction=2.)['status'],'failed')
    def test_changed_tolerance_is_rejected(self):
        with self.assertRaises(ValueError):self.report(tolerance=1e-8)

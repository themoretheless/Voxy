import json
from pathlib import Path
import tempfile
import unittest
from test_verify_five_qr import runtime
from verify_five_selection import verify
class SelectionCertificates(unittest.TestCase):
    def report(self,row=1,active=None):
        case={'name':'fixture','gaps':[runtime(-1.),runtime(-2.)],'decisions':[1,1],'active':active or [False,False],'status':'enter','row':row}
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'selection.json';path.write_text(json.dumps({'cases':[case]}));return verify(path)
    def test_certified_minimum(self):self.assertEqual(self.report()['status'],'passed')
    def test_wrong_minimum_rejected(self):self.assertEqual(self.report(row=0)['status'],'failed')
    def test_active_row_rejected(self):self.assertEqual(self.report(active=[False,True])['status'],'failed')

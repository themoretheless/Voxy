import json
from pathlib import Path
import tempfile
import unittest
from test_verify_five_qr import runtime
from verify_five_release import verify
class ReleaseCertificates(unittest.TestCase):
    def report(self,fraction=.5,reaction=0.,coordinate=None):
        case={'name':'fixture','current':[runtime(1.)],'candidate':[runtime(-1.)],'output':{'status':'release','row':0,'fraction':runtime(fraction),'reactions':[runtime(reaction)]}}
        if coordinate is not None:
            case['state']=[runtime(1.)];case['next_state']=[runtime(-1.)];case['output']['coordinates']=[runtime(coordinate)]
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'release.json';path.write_text(json.dumps({'cases':[case]}));return verify(path)
    def test_exact_release(self):self.assertEqual(self.report()['status'],'passed')
    def test_wrong_fraction_rejected(self):self.assertEqual(self.report(fraction=.25)['status'],'failed')
    def test_nonzero_released_reaction_rejected(self):self.assertEqual(self.report(reaction=.1)['status'],'failed')

    def test_exact_coordinate_update(self):self.assertEqual(self.report(coordinate=0.)['status'],'passed')
    def test_corrupted_coordinate_update_rejected(self):self.assertEqual(self.report(coordinate=.1)['status'],'failed')

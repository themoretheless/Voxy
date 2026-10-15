import json
from pathlib import Path
import struct
import tempfile
import unittest
from verify_five_addition import verify

def word(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def row():
    return {'input_a_words':[word(1.),word(2.**-100),0,0,0], 'input_b_words':[word(-1.),0,0,0,0],
            'addition_words':[word(1.),0,0,0,0],'addition_exponent':-100,'addition_error_bound_word':0,
            'subtraction_words':[word(2.),word(2.**-100),0,0,0],'subtraction_exponent':0,'subtraction_error_bound_word':0}
class AdditionOracleTests(unittest.TestCase):
    def check(self,r,required=False):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'input.json';path.write_text(json.dumps({'records':[r]}))
            return verify(path,require_propagated=required)
    def test_cancellation_preserves_a_tail_hidden_by_f64(self):
        self.assertEqual(1.+2.**-100,1.)
        self.assertEqual(self.check(row())['status'],'passed')
    def test_lost_cancellation_tail_fails(self):
        r=row();r['addition_words']=[0]*5
        self.assertEqual(self.check(r)['status'],'failed')
    def test_required_propagation_cannot_be_omitted(self):
        self.assertEqual(self.check(row(),True)['status'],'failed')
    def test_inherited_input_errors_cannot_disappear(self):
        r=row();r.update(propagated_words=[word(1.),0,0,0,0],propagated_exponent=-100,propagated_error_bound_word=0)
        result=self.check(r,True)
        self.assertEqual(result['status'],'failed')
        self.assertEqual(result['propagated_error_cases_underestimated'],1)
    def test_truthful_bound_does_not_claim_precision_after_cancellation(self):
        r=row();r.update(propagated_words=[word(1.),0,0,0,0],propagated_exponent=-100,propagated_error_bound_word=word(2.**-9))
        result=self.check(r,True)
        self.assertEqual(result['status'],'passed')
        self.assertEqual(result['propagated_error_cases_underestimated'],0)
        self.assertEqual(result['propagated_error_cases_rejected_by_precision_gate'],1)
    def test_negative_error_bound_is_rejected(self):
        r=row();r['addition_error_bound_word']=word(-1.)
        with self.assertRaises(ValueError):self.check(r)
if __name__=='__main__':unittest.main()

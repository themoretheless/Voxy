import json
from pathlib import Path
import struct
import tempfile
import unittest
from verify_five_division import verify

def word(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def row():
    return {'input_a_words':[word(2.),0,0,0,0],'input_b_words':[word(4.),0,0,0,0],
            'input_error_a_word':word(0.125),'input_error_b_word':word(0.125),
            'exact_division_words':[word(1.),0,0,0,0],'exact_division_exponent':-1,'exact_division_error_bound_word':0,'exact_division_valid':1,
            'bounded_division_words':[word(1.),0,0,0,0],'bounded_division_exponent':-1,'bounded_division_error_bound_word':word(0.5),'bounded_division_valid':1}
class DivisionOracleTests(unittest.TestCase):
    def check(self,r):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'input.json';path.write_text(json.dumps({'records':[r]}))
            return verify(path)
    def test_exact_quotient_and_interval_corners_pass(self):
        self.assertEqual(self.check(row())['status'],'passed')
    def test_correct_nominal_quotient_cannot_hide_an_interval_corner(self):
        r=row();r['bounded_division_error_bound_word']=word(0.2)
        self.assertEqual(self.check(r)['uncertain_intervals_underestimated'],1)
    def test_zero_denominator_must_be_rejected(self):
        r=row();r.update(input_b_words=[0]*5,input_error_b_word=0,exact_division_valid=0,bounded_division_valid=0)
        self.assertEqual(self.check(r)['status'],'passed')
        r['exact_division_valid']=1
        self.assertEqual(self.check(r)['invalid_domains_admitted'],1)
    def test_denominator_interval_crossing_zero_must_be_rejected(self):
        r=row();r.update(input_error_b_word=word(2.),bounded_division_valid=0)
        self.assertEqual(self.check(r)['status'],'passed')
        r['bounded_division_valid']=1
        self.assertEqual(self.check(r)['status'],'failed')
    def test_negative_input_error_must_be_rejected(self):
        r=row();r.update(input_error_a_word=word(-1.),bounded_division_valid=0)
        self.assertEqual(self.check(r)['status'],'passed')
    def test_correct_answer_with_inadequate_bound_fails_precision(self):
        r=row();r['exact_division_error_bound_word']=word(0.5)
        self.assertEqual(self.check(r)['exact_quotients_with_inadequate_error_bound'],1)
    def test_finite_nonzero_denominator_cannot_be_silently_skipped(self):
        r=row();r['exact_division_valid']=0
        with self.assertRaises(ValueError):self.check(r)
if __name__=='__main__':unittest.main()

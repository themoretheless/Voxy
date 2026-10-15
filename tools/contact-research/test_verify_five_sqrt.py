import json
from pathlib import Path
import struct
import tempfile
import unittest
from verify_five_sqrt import verify

def word(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def row():
    return {'input_a_words':[word(4.),0,0,0,0],'input_error_a_word':word(0.125),
            'exact_sqrt_words':[word(1.),0,0,0,0],'exact_sqrt_exponent':1,'exact_sqrt_error_bound_word':0,'exact_sqrt_valid':1,
            'bounded_sqrt_words':[word(1.),0,0,0,0],'bounded_sqrt_exponent':1,'bounded_sqrt_error_bound_word':word(0.125),'bounded_sqrt_valid':1}
class SquareRootOracleTests(unittest.TestCase):
    def check(self,r):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'input.json';path.write_text(json.dumps({'records':[r]}))
            return verify(path)
    def test_exact_square_and_interval_pass(self):
        self.assertEqual(self.check(row())['status'],'passed')
    def test_interval_bound_cannot_hide_a_missing_endpoint(self):
        r=row();r['bounded_sqrt_error_bound_word']=word(0.05)
        self.assertEqual(self.check(r)['interval_root_bounds_underestimated'],1)
    def test_f64_invisible_input_tail_is_checked_by_rational_squares(self):
        r=row();r.update(input_a_words=[word(1.),word(2.**-100),0,0,0],input_error_a_word=0,
            exact_sqrt_words=[word(1.),word(2.**-101),0,0,0],exact_sqrt_exponent=0,exact_sqrt_error_bound_word=word(2.**-120),
            bounded_sqrt_words=[word(1.),word(2.**-101),0,0,0],bounded_sqrt_exponent=0,bounded_sqrt_error_bound_word=word(2.**-120))
        self.assertEqual(self.check(r)['status'],'passed')
    def test_zero_boundary_is_valid(self):
        r=row();r.update(input_a_words=[word(1.),0,0,0,0],input_error_a_word=word(1.),exact_sqrt_exponent=0,
            bounded_sqrt_exponent=0,bounded_sqrt_error_bound_word=word(1.))
        self.assertEqual(self.check(r)['valid_intervals_touching_zero_checked'],1)
        self.assertEqual(self.check(r)['status'],'passed')
    def test_exact_zero_and_uncertain_zero_have_different_domains(self):
        r=row();r.update(input_a_words=[0]*5,input_error_a_word=word(0.125),exact_sqrt_words=[0]*5,exact_sqrt_exponent=0,bounded_sqrt_valid=0)
        self.assertEqual(self.check(r)['status'],'passed')
        r['bounded_sqrt_valid']=1
        self.assertEqual(self.check(r)['invalid_domains_admitted'],1)
    def test_negative_input_must_be_rejected(self):
        r=row();r.update(input_a_words=[word(-1.),0,0,0,0],input_error_a_word=0,exact_sqrt_valid=0,bounded_sqrt_valid=0)
        self.assertEqual(self.check(r)['status'],'passed')
        r['exact_sqrt_valid']=1
        self.assertEqual(self.check(r)['invalid_domains_admitted'],1)
    def test_wide_bound_cannot_certify_precision_for_exact_input(self):
        r=row();r['exact_sqrt_error_bound_word']=word(0.125)
        self.assertEqual(self.check(r)['exact_roots_with_inadequate_error_bound'],1)
    def test_positive_input_cannot_be_silently_skipped(self):
        r=row();r['exact_sqrt_valid']=0
        with self.assertRaises(ValueError):self.check(r)
if __name__=='__main__':unittest.main()

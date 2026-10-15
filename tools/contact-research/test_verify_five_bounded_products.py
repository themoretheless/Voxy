import json
from pathlib import Path
import struct
import tempfile
import unittest
from verify_five_bounded_products import verify

def word(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def row():
    return {'input_a_words':[word(2.),0,0,0,0],'input_b_words':[word(-3.),0,0,0,0],
            'input_error_a_word':word(0.125),'input_error_b_word':word(0.25),
            'bounded_product_words':[word(-1.5),0,0,0,0],'bounded_product_exponent':2,
            'bounded_product_error_bound_word':word(0.46875),'bounded_product_valid':1}
class BoundedProductOracleTests(unittest.TestCase):
    def check(self,r):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'input.json';path.write_text(json.dumps({'records':[r]}))
            return verify(path)
    def test_four_corners_cover_mixed_sign_interval(self):
        self.assertEqual(self.check(row())['status'],'passed')
    def test_correct_nominal_product_does_not_hide_missing_corner(self):
        r=row();r['bounded_product_error_bound_word']=word(0.375)
        self.assertEqual(self.check(r)['output_intervals_underestimated'],1)
    def test_cross_error_term_is_required_when_both_intervals_cross_zero(self):
        r=row();r.update(input_a_words=[0]*5,input_b_words=[0]*5,input_error_a_word=word(1.),input_error_b_word=word(1.),bounded_product_words=[0]*5,bounded_product_exponent=0,bounded_product_error_bound_word=word(1.))
        self.assertEqual(self.check(r)['status'],'passed')
        r['bounded_product_error_bound_word']=0
        self.assertEqual(self.check(r)['status'],'failed')
    def test_negative_input_radius_must_be_rejected(self):
        r=row();r['input_error_a_word']=word(-1.);r['bounded_product_valid']=0
        self.assertEqual(self.check(r)['status'],'passed')
        r['bounded_product_valid']=1
        self.assertEqual(self.check(r)['invalid_input_radii_admitted'],1)
    def test_error_free_product_can_pass_precision_gate(self):
        r=row();r.update(input_error_a_word=0,input_error_b_word=0,bounded_product_error_bound_word=0)
        self.assertEqual(self.check(r)['output_intervals_insufficient_for_relative_2_pow_minus_106'],0)
    def test_malformed_result_is_rejected(self):
        r=row();r['bounded_product_words'].pop()
        with self.assertRaises(ValueError):self.check(r)
if __name__=='__main__':unittest.main()

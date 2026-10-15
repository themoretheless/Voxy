import importlib.util
from pathlib import Path
import json
import struct
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('products',Path(__file__).with_name('verify_five_products.py'))
products=importlib.util.module_from_spec(spec)
spec.loader.exec_module(products)
def word(v): return struct.unpack('<I',struct.pack('<f',v))[0]
def row():
    return {'input_a_words':[word(1.),word(2.**-100),0,0,0], 'input_b_words':[word(1.),0,0,0,0],
            'raw_product_words':[word(1.)]+[0]*49,
            'normalized_product_words':[word(1.),word(2.**-100)]+[0]*48,
            'normalized_exponent':0,'normalized_error_bound_word':0}
class ProductOracleTests(unittest.TestCase):
    def verify(self,r,require_rounded=False):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'input.json';path.write_text(json.dumps({'records':[r]}))
            return products.verify(path,require_rounded=require_rounded)
    def test_f64_invisible_tail_is_checked_exactly(self):
        result=self.verify(row())
        self.assertEqual(result['status'],'passed')
        self.assertEqual(result['raw_cases_exceeding_target'],1)
        self.assertEqual(result['normalized_cases_exceeding_target'],0)
    def test_wrong_answer_does_not_pass_with_large_error_bound(self):
        r=row();r['normalized_product_words']=r['raw_product_words'];r['normalized_error_bound_word']=word(2.**-90)
        result=self.verify(r)
        self.assertEqual(result['status'],'failed')
        self.assertEqual(result['normalized_cases_exceeding_primitive_bound'],0)
        self.assertEqual(result['normalized_cases_exceeding_target'],1)
    def test_wrong_error_bound_is_rejected(self):
        r=row();r['normalized_product_words']=r['raw_product_words']
        self.assertEqual(self.verify(r)['normalized_cases_exceeding_primitive_bound'],1)
    def test_required_five_component_result_cannot_be_omitted(self):
        self.assertEqual(self.verify(row(),require_rounded=True)['status'],'failed')
    def test_exact_five_component_result_passes_required_check(self):
        r=row();r['rounded_product_words']=[word(1.),word(2.**-100),0,0,0]
        r['rounded_exponent']=0;r['rounded_error_bound_word']=0
        report=self.verify(r,require_rounded=True)
        self.assertEqual(report['status'],'passed')
        self.assertEqual(report['compressed_products_checked'],1)
    def test_exact_answer_does_not_make_an_inadequate_bound_pass(self):
        r=row();r['rounded_product_words']=[word(1.),word(2.**-100),0,0,0]
        r['rounded_exponent']=0;r['rounded_error_bound_word']=word(2.**-90)
        report=self.verify(r,require_rounded=True)
        self.assertEqual(report['status'],'failed')
        self.assertEqual(report['compressed_cases_exceeding_target'],0)
        self.assertEqual(report['compressed_cases_with_inadequate_error_bound'],1)
    def test_exact_zero_requires_zero_result_and_zero_error_bound(self):
        r=row();r['input_b_words']=[0]*5;r['raw_product_words']=[0]*50;r['normalized_product_words']=[0]*50
        r['rounded_product_words']=[0]*5;r['rounded_exponent']=0;r['rounded_error_bound_word']=0
        self.assertEqual(self.verify(r,require_rounded=True)['status'],'passed')
        r['rounded_error_bound_word']=word(2.**-120)
        self.assertEqual(self.verify(r,require_rounded=True)['status'],'failed')
    def test_nonfinite_or_malformed_output_is_rejected(self):
        r=row();r['normalized_product_words'][0]=word(float('nan'))
        with self.assertRaises(ValueError):self.verify(r)
        r=row();r['input_a_words'].pop()
        with self.assertRaises(ValueError):self.verify(r)
if __name__=='__main__':unittest.main()

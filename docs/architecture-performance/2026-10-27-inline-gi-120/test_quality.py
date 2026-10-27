import json,tempfile,unittest
from pathlib import Path
import numpy as np
from analyze_quality import analyze
class NativeQualityTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)/'quality-route';self.root.mkdir()
  policy=json.loads(Path(__file__).with_name('quality-policy.json').read_text());policy['frames']=[120];(self.root.parent/'quality-policy.json').write_text(json.dumps(policy))
  for arm in ['A0','A1','B']:
   p=self.root/arm/'frame-120';p.mkdir(parents=True)
   np.ones(4,dtype='<f2').tofile(p/'scene-before-taa.native.bin')
   (p/'stages.json').write_text(json.dumps({'temporal_uniform':{'blake3':'same'},'stages':[{'stage':'scene-before-taa','source_format':'Rgba16Float','size':[1,1]}]}))
 def mutate(self,arm,values):np.array(values,dtype='<f2').tofile(self.root/arm/'frame-120'/'scene-before-taa.native.bin')
 def test_identity(self):self.assertTrue(analyze(self.root)['strict_bit_parity_passed'])
 def test_one_source_ulp(self):
  self.mutate('B',[1+2**-10,1,1,1]);r=analyze(self.root);self.assertTrue(r['native_quality_bound_passed']);self.assertFalse(r['strict_bit_parity_passed'])
 def test_two_source_ulps_rejected(self):
  self.mutate('B',[1+2**-9,1,1,1]);self.assertFalse(analyze(self.root)['native_quality_bound_passed'])
 def test_zero_has_subnormal_quantum(self):
  for arm in ['A0','A1']:self.mutate(arm,[0,0,0,0])
  self.mutate('B',[2**-24,0,0,0]);self.assertTrue(analyze(self.root)['native_quality_bound_passed'])
  self.mutate('B',[2**-23,0,0,0]);self.assertFalse(analyze(self.root)['native_quality_bound_passed'])
 def test_truncated_capture_rejected(self):
  self.mutate('B',[1,1]);
  with self.assertRaises(ValueError):analyze(self.root)
 def test_nonfinite_rejected(self):
  self.mutate('B',[np.nan,1,1,1]);self.assertFalse(analyze(self.root)['native_quality_bound_passed'])
 def test_noisy_control_rejected(self):
  self.mutate('A1',[1.1,1,1,1]);self.assertFalse(analyze(self.root)['native_quality_bound_passed'])
 def test_wrong_uniform_rejected(self):
  p=self.root/'B'/'frame-120'/'stages.json';d=json.loads(p.read_text());d['temporal_uniform']['blake3']='different';p.write_text(json.dumps(d));self.assertFalse(analyze(self.root)['native_quality_bound_passed'])
if __name__=='__main__':unittest.main()

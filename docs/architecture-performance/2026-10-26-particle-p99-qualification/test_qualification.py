import copy,importlib.util,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('confirmation',Path(__file__).with_name('qualify.py'))
q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)
class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.manifest=[dict(stage=s,repeat=r,binary_sha256=q.EXPECTED,correctness_passed=True,schedule_valid=True,control_stable=True if s=='control' else None) for s,r in q.ORDER]
        self.cases=[dict(stage=s,pairs=36,frames_per_mode=3456,metrics={m:dict(ci95=[.99,1.01]) for m in q.BOUNDS}) for s in ['control','upload']]
    def test_complete_stable_matrix_passes(self):
        self.assertTrue(q.decision(self.manifest,self.cases)['qualified'])
    def test_failed_control_cannot_be_waived(self):
        self.manifest[-1]['control_stable']=False
        self.assertFalse(q.decision(self.manifest,self.cases)['qualified'])
    def test_p99_bound_failure_cannot_pass(self):
        self.cases[-1]['metrics']['gpu_gap_p99']['ci95'][1]=1.10001
        self.assertFalse(q.decision(self.manifest,self.cases)['relative_p99_qualified'])
    def test_binary_mismatch_rejected(self):
        self.manifest[-1]['binary_sha256']='other'
        with self.assertRaises(AssertionError):q.decision(self.manifest,self.cases)
    def test_missing_or_duplicate_control_rejected(self):
        with self.assertRaises(AssertionError):q.decision(self.manifest[:-1],self.cases)
        self.manifest[-1]=copy.deepcopy(self.manifest[0])
        with self.assertRaises(AssertionError):q.decision(self.manifest,self.cases)
    def test_reduced_sample_count_rejected(self):
        self.cases[-1]['pairs']=35
        with self.assertRaises(AssertionError):q.decision(self.manifest,self.cases)
    def test_schedule_or_correctness_failure_rejected(self):
        for key in ['correctness_passed','schedule_valid']:
            m=copy.deepcopy(self.manifest);m[0][key]=False
            with self.assertRaises(AssertionError):q.decision(m,self.cases)
if __name__=='__main__':unittest.main()

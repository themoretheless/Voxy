import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('analysis', Path(__file__).with_name('analyze_pairs.py'))
analysis = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analysis)


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'pairs'
        self.root.mkdir()
        self.quality = True
        (self.root.parent / "quality-route").mkdir()
        schedule = [('aa-start-0', 'A'), ('aa-start-1', 'A')]
        for i in range(5):
            schedule.extend((f'pair-{i}-{arm}', arm) for arm in ('AB' if i % 2 == 0 else 'BA'))
        schedule += [('aa-end-0', 'A'), ('aa-end-1', 'A')]
        self.manifest = {'binaries':{'A':{'sha256':'A'},'B':{'sha256':'B'}},'schedule': schedule,
                         'runs': [dict(name=n, arm=a, exit_code=0) for n, a in schedule]}
        self.report = dict(gi_quad_cache=True, completed_offscreen_measurement_valid=True,
                           frame_sample_clock='current_completed_frame',
                           simulation_clock='fixed_one_120Hz_step_per_render',
                           resolution=[1920, 1080], internal_resolution=[1920, 1080],
                           source_simplified=False, antialiasing='Temporal', frames=2400,
                           complete_motion_cycles=1, measured_physics_steps=2400,
                           completed_frame_samples_seconds=[.007] * 2400, host_completion_intervals_seconds=[.007] * 2400, gpu_samples=100,
                           fx_off=None, compute_gi=False, correctness_capture_active=False, paired_depth_dda=False,
                           ray_capture_enabled=False, replay_state_blake3='same',
                           p50_ms=7., p99_ms=7., gpu_mean_ms=6.)
        self.reports = {n: dict(copy.deepcopy(self.report), compute_gi=False, original_gi_queue_requested=False, original_gi_queue_direct_scatter_requested=False) for n, a in schedule}

    def run_analysis(self):
        (self.root.parent / 'quality-route' / 'quality.json').write_text(json.dumps({'passed':self.quality,'native_quality_bound_passed':self.quality,'binary_sha256':{'A0':'A','A1':'A','B':'B'}}))
        (self.root / 'manifest.json').write_text(json.dumps(self.manifest))
        for name, report in self.reports.items():
            (self.root / f'{name}.json').write_text(json.dumps(report))
        return analysis.analyze(self.root)

    def test_valid_campaign(self):
        self.assertTrue(self.run_analysis()['qualified_120_fps'])

    def test_relative_parity_does_not_imply_absolute_120(self):
        for d in self.reports.values():
            d.update(p50_ms=12., p99_ms=12., completed_frame_samples_seconds=[.012] * 2400)
        result = self.run_analysis()
        self.assertTrue(result['relative_p99_nonregression_passed'])
        self.assertFalse(result['qualified_120_fps'])

    def test_noisy_control(self):
        self.reports['aa-end-1']['p99_ms'] = 9.
        self.reports['aa-end-1']['completed_frame_samples_seconds'] = [.007] * 2376 + [.009] * 24
        result = self.run_analysis()
        self.assertFalse(result['aa_passed'])
        self.assertFalse(result['qualified_120_fps'])

    def test_replay_mismatch(self):
        self.reports['pair-2-B']['replay_state_blake3'] = 'changed'
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_shortened_workload(self):
        self.reports['pair-0-B']['complete_motion_cycles'] = 0
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_missing_final_control(self):
        self.manifest['runs'].pop()
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_reduced_quality(self):
        self.reports['pair-1-B']['fx_off'] = '1'
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_failed_gi_parity(self):
        self.quality = False
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_completion_pacing_misses_deadline(self):
        self.reports['pair-1-B']['host_completion_intervals_seconds'] = [.010] * 2400
        self.assertFalse(self.run_analysis()['absolute_p99_passed'])

    def test_missing_completion_sample(self):
        self.reports['pair-1-B']['host_completion_intervals_seconds'].pop()
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_missing_gpu_timing(self):
        self.reports['pair-1-B']['gpu_mean_ms'] = None
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_nonfinite_completion_interval(self):
        self.reports['pair-1-B']['host_completion_intervals_seconds'][0] = float('nan')
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_readback_work_cannot_qualify(self):
        self.reports['pair-1-B']['correctness_capture_active'] = True
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_wrong_proof_binary(self):
        self.manifest['binaries']['B']['sha256']='wrong'
        self.assertFalse(self.run_analysis()['qualified_120_fps'])

    def test_forged_quantile(self):
        self.reports['pair-1-B']['p99_ms'] = 1.
        self.assertFalse(self.run_analysis()['qualified_120_fps'])


if __name__ == '__main__':
    unittest.main()

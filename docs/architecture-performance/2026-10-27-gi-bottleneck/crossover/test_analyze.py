import copy,json,tempfile,unittest
from pathlib import Path
from analyze import analyze,SCHEDULE,label,quantile
class CrossoverTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)/'runs';self.root.mkdir();self.q=Path(self.tmp.name)/'quality-route';self.q.mkdir();self.manifest={'sha256':'frozen','assets':{'fixture':'hash'},'runs':[dict(name=n,mode=m,exit_code=0) for n,m in SCHEDULE]};self.proof={'passed':True,'native_quality_bound_passed':True,'assets':{'fixture':'hash'},'binary_sha256':dict(A0='frozen',A1='frozen',B='frozen')};self.reports={}
  for n,m in SCHEDULE:
   ids=list(range(181,2581));cache=lambda f:label(m,f) and m!='aa';cpu=[.0065 if cache(f) else .007 for f in ids];stamps=[[f,label(m,f),cache(f)] for f in ids[::2]];gpu=[5.4 if c else 6. for f,b,c in stamps]
   self.reports[n]=dict(frames=2400,measured_frame_ids=ids,completed_frame_samples_seconds=cpu,host_completion_intervals_seconds=cpu.copy(),measurement_start_simulation_steps=180,measured_physics_steps=2400,complete_motion_cycles=1,replay_render_frames=2580,replay_state_blake3='same',simulation_clock='fixed_one_120Hz_step_per_render',frame_sample_clock='current_completed_frame',resolution=[1920,1080],internal_resolution=[1920,1080],antialiasing='Temporal',source_simplified=False,crossover_measurement_valid=True,completed_offscreen_measurement_valid=m=='bb',refine_crossover_mode=m,correctness_capture_active=False,gpu_frame_stamps=stamps,gpu_completed_samples_ms=gpu,gpu_samples=len(gpu),p50_ms=quantile(cpu,50)*1000,p99_ms=quantile(cpu,99)*1000,gpu_mean_ms=sum(gpu)/len(gpu))
 def run_analysis(self):
  (self.root/'manifest.json').write_text(json.dumps(self.manifest));(self.q/'quality.json').write_text(json.dumps(self.proof))
  for n,d in self.reports.items():(self.root/f'{n}.json').write_text(json.dumps(d))
  return analyze(self.root)
 def test_valid_counterbalanced_campaign_and_single_arm_deadline(self):self.assertTrue(self.run_analysis()['qualified_120_fps'])
 def test_producer_not_collection_frame(self):
  self.reports['ab-0']['gpu_frame_stamps'][0][0]=179
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_wrong_arm(self):
  self.reports['ab-0']['gpu_frame_stamps'][0][1]^=True
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_aa_must_use_identical_pipeline_policy(self):
  self.reports['aa-start']['gpu_frame_stamps'][0][2]=True
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_duplicate_timestamp_producer(self):
  self.reports['ab-0']['gpu_frame_stamps'][1]=self.reports['ab-0']['gpu_frame_stamps'][0]
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_missing_gpu_mean(self):
  del self.reports['ab-0']['gpu_mean_ms']
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_malformed_timestamp(self):
  self.reports['ab-0']['gpu_frame_stamps'][0]=[]
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_capture_contamination(self):
  self.reports['bb-0']['correctness_capture_active']=True
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_mixed_pipeline_must_not_claim_single_arm_completion_validity(self):
  self.reports['ab-0']['completed_offscreen_measurement_valid']=True
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_relative_improvement_does_not_imply_deadline(self):
  d=self.reports['bb-0'];d['completed_frame_samples_seconds']=[.01]*2400;d['p50_ms']=d['p99_ms']=10.
  r=self.run_analysis();self.assertTrue(r['candidate_promotion_passed']);self.assertFalse(r['qualified_120_fps'])
 def test_failed_quality_is_not_promoted(self):
  self.proof['passed']=False
  self.assertFalse(self.run_analysis()['candidate_promotion_passed'])
 def test_asset_identity_required(self):
  self.proof['assets']['fixture']='different'
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_executable_identity_required(self):
  self.proof['binary_sha256']['B']='different'
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_internal_aa_noise_rejects_contrast(self):
  d=self.reports['aa-start'];d['gpu_completed_samples_ms']=[7. if b else 6. for f,b,c in d['gpu_frame_stamps']];d['gpu_mean_ms']=sum(d['gpu_completed_samples_ms'])/len(d['gpu_completed_samples_ms'])
  self.assertFalse(self.run_analysis()['aa_passed'])
 def test_global_drift_is_reported_not_hidden_or_used_to_normalize(self):
  d=self.reports['aa-end'];d['gpu_completed_samples_ms']=[v*1.5 for v in d['gpu_completed_samples_ms']];d['gpu_mean_ms']*=1.5
  r=self.run_analysis();self.assertTrue(r['aa_passed']);self.assertAlmostEqual(r['external_start_end_drift']['gpu_mean'],1.5)
 def test_wrong_replay(self):
  self.reports['ba-1']['replay_state_blake3']='different'
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_incomplete_route(self):
  self.reports['ab-0']['frames']=1200
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
 def test_lied_quantiles(self):
  self.reports['bb-1']['p99_ms']=1.
  self.assertFalse(self.run_analysis()['qualified_120_fps'])
if __name__=='__main__':unittest.main()

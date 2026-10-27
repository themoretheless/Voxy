"""Preregistered crossover analysis; never infer absolute FPS from mixed frames."""
import json,math,random,sys
from pathlib import Path
SCHEDULE=[('aa-start','aa'),('ab-0','ab'),('ba-0','ba'),('ab-1','ab'),('ba-1','ba'),('bb-0','bb'),('bb-1','bb'),('aa-end','aa')]
PATTERN=[False,True,True,False,True,False,False,True]
def label(mode,frame):return True if mode=='bb' else PATTERN[(frame-1)%8]^(mode=='ba')
def quantile(xs,p):return sorted(xs)[len(xs)*p//100]
def metrics(d,mode):
 ids=d['measured_frame_ids'];arms={False:{},True:{}}
 for arm in [False,True]:
  ix=[i for i,f in enumerate(ids) if label(mode,f)==arm]
  gpu=[v for (f,b,c),v in zip(d['gpu_frame_stamps'],d['gpu_completed_samples_ms']) if b==arm]
  if not ix:continue
  service=[d['completed_frame_samples_seconds'][i]*1000 for i in ix];cadence=[d['host_completion_intervals_seconds'][i]*1000 for i in ix]
  arms[arm]=dict(service_p50=quantile(service,50),service_p99=quantile(service,99),host_completion_p99=quantile(cadence,99),gpu_mean=sum(gpu)/len(gpu),frames=len(ix),gpu_samples=len(gpu))
 return arms
def ratio(arms):return {k:arms[True][k]/arms[False][k] for k in ['service_p50','service_p99','host_completion_p99','gpu_mean']}
def ci(contrasts,key):
 rng=random.Random(20261027);samples=sorted(math.exp(sum(math.log(rng.choice(contrasts)[key]) for _ in contrasts)/len(contrasts)) for _ in range(10000));return [samples[250],samples[9749]]
def analyze(root):
 root=Path(root);manifest=json.loads((root/'manifest.json').read_text());errors=[];runs={};arms={}
 if [(r['name'],r['mode']) for r in manifest['runs']]!=SCHEDULE:errors.append('missing or reordered campaign runs')
 for name,mode in SCHEDULE:
  path=root/f'{name}.json'
  if not path.exists():errors.append(name+': missing report');continue
  d=json.loads(path.read_text());runs[name]=d
  ids=d.get('measured_frame_ids',[]);cpu=d.get('completed_frame_samples_seconds',[]);cad=d.get('host_completion_intervals_seconds',[]);stamps=d.get('gpu_frame_stamps',[]);gpu=d.get('gpu_completed_samples_ms',[])
  valid=(d.get('frames')==2400 and ids==list(range(181,2581)) and len(cpu)==len(cad)==2400 and all(isinstance(x,(int,float)) and math.isfinite(x) and x>0 for x in cpu+cad+gpu) and d.get('measurement_start_simulation_steps')==180 and d.get('measured_physics_steps')==2400 and d.get('complete_motion_cycles',0)>=1 and d.get('replay_render_frames')==2580 and d.get('simulation_clock')=='fixed_one_120Hz_step_per_render' and d.get('frame_sample_clock')=='current_completed_frame' and d.get('resolution')==[1920,1080] and d.get('internal_resolution')==[1920,1080] and d.get('antialiasing')=='Temporal' and d.get('source_simplified') is False and d.get('crossover_measurement_valid') is True and d.get('completed_offscreen_measurement_valid') is (mode=='bb') and d.get('refine_crossover_mode')==mode and d.get('correctness_capture_active') is False and not d.get('fx_off') and not d.get('compute_gi') and not d.get('paired_depth_dda') and not d.get('ray_capture_enabled') and not d.get('ray_queue') and not d.get('world_ray_frame') and len(stamps)==len(gpu)==d.get('gpu_samples') and len(gpu)>0 and all(isinstance(d.get(k),(int,float)) for k in ['p50_ms','p99_ms','gpu_mean_ms']))
  if not valid:errors.append(name+': invalid workload or missing full-frame metadata');continue
  if any(not isinstance(stamp,list) or len(stamp)!=3 for stamp in stamps):errors.append(name+': malformed GPU producer stamp');continue
  stamp_errors=len(errors)
  seen=set()
  for stamp in stamps:
   if len(stamp)!=3:errors.append(name+': malformed GPU producer stamp');break
   f,b,c=stamp
   if f not in ids or f in seen or b is not label(mode,f) or c is not (b and mode!='aa'):errors.append(name+': wrong GPU producer identity/variant');break
   seen.add(f)
  if [s[0] for s in stamps]!=sorted(seen):errors.append(name+': unordered GPU producer stamps')
  if len(errors)>stamp_errors:continue
  if not math.isclose(d['p50_ms'],quantile(cpu,50)*1000,rel_tol=1e-9) or not math.isclose(d['p99_ms'],quantile(cpu,99)*1000,rel_tol=1e-9) or not math.isclose(d['gpu_mean_ms'],sum(gpu)/len(gpu),rel_tol=1e-9):errors.append(name+': quantiles/mean disagree with raw samples')
  arms[name]=metrics(d,mode)
  required=[True] if mode=='bb' else [False,True]
  if any(arms[name][a].get('gpu_samples',0)<400 for a in required):errors.append(name+': insufficient GPU samples by label')
 if len(runs)==8 and len({d.get('replay_state_blake3') for d in runs.values()})!=1:errors.append('replay differs across complete routes')
 if any(r['exit_code']!=0 for r in manifest['runs']):errors.append('process failed')
 if errors:return dict(qualified_120_fps=False,candidate_promotion_passed=False,errors=errors)
 controls=[]
 for name in ['aa-start','aa-end']:
  ratios=ratio(arms[name]);passed=all(1-limit<=ratios[k]<=1+limit for k,limit in [('service_p50',.1),('service_p99',.1),('host_completion_p99',.1),('gpu_mean',.05)])
  controls.append(dict(run=name,ratios=ratios,passed=passed))
 contrasts=[dict(run=name,**ratio(arms[name])) for name in ['ab-0','ba-0','ab-1','ba-1']];intervals={k:ci(contrasts,k) for k in ['service_p99','host_completion_p99','gpu_mean']}
 deadline=all(arms[n][True]['service_p99']<=8.33 and arms[n][True]['host_completion_p99']<=8.33 for n in ['bb-0','bb-1'])
 p=root.parent/'quality-route/quality.json';proof=json.loads(p.read_text()) if p.exists() else {}
 quality=proof.get('passed') is True and proof.get('native_quality_bound_passed') is True and all(proof.get('binary_sha256',{}).get(k)==manifest['sha256'] for k in ['A0','A1','B']) and proof.get('assets')==manifest.get('assets') and bool(manifest.get('assets'))
 aa=all(c['passed'] for c in controls);relative=aa and intervals['service_p99'][1]<=1.05 and intervals['host_completion_p99'][1]<=1.05
 promotion=relative and quality and intervals['gpu_mean'][1]<1.
 external={k:arms['aa-end'][False][k]/arms['aa-start'][False][k] for k in ['service_p50','service_p99','host_completion_p99','gpu_mean']}
 return dict(qualified_120_fps=promotion and deadline,candidate_promotion_passed=promotion,errors=[],aa_passed=aa,native_quality_bound_passed=quality,strict_bit_parity_passed=proof.get('strict_bit_parity_passed'),absolute_p99_passed=deadline,relative_p99_nonregression_passed=relative,controls=controls,contrasts=contrasts,ci95=intervals,arms=arms,external_start_end_drift=external,scope='counterbalanced full-frame contrasts; absolute gate uses separate single-pipeline BB routes; not scanout')
if __name__=='__main__':print(json.dumps(analyze(Path(sys.argv[1])),indent=2))

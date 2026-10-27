import hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parent;binary=Path(sys.argv[1]).resolve();materials=Path(sys.argv[2]).resolve();out=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=False)
reports={};binaries={}
assets={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}
for name,mode in [('A0','aa'),('A1','aa'),('B','bb')]:
 env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))};env.update(VOXY_STREET_COMPUTE_GI='1',VOXY_STREET_GI_RASTER_MATH='1',VOXY_STREET_GI_STAGE_CROSSOVER='1',VOXY_STREET_MATERIALS=str(materials),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_REFINE_CROSSOVER=mode,VOXY_STREET_HDR_CHECK_DIR=str(out/name),VOXY_STREET_BENCHMARK_REPORT=str(out/f'{name}.json'))
 binaries[name]=hashlib.sha256(binary.read_bytes()).hexdigest()
 with (out/f'{name}.log').open('w') as log:subprocess.run([str(binary),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','20'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 reports[name]=json.loads((out/f'{name}.json').read_text());print(name,'complete',flush=True)
sys.path.insert(0,str(root.parents[2]/'2026-10-27-inline-gi-120'))
from analyze_quality import analyze
result=analyze(out);result['assets']=assets;result['same_replay']=len({d['replay_state_blake3'] for d in reports.values()})==1;result['all_captures_marked']=all(d.get('correctness_capture_active') is True for d in reports.values());result['binary_sha256']=binaries
result['passed']=all(result[k] for k in ['native_quality_bound_passed','same_replay','all_captures_marked']);(out/'quality.json').write_text(json.dumps(result,indent=2)+'\n')
if assets!={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}:raise SystemExit('Assets changed')
if not result['passed']:raise SystemExit('Native quality bound failed')

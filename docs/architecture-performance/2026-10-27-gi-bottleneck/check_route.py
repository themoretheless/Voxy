"""Full-route A/A + A/B linear HDR captures, separate from performance runs."""
import hashlib,json,os,subprocess,sys
from pathlib import Path
before,after,output,materials=map(Path,sys.argv[1:]);output.mkdir(parents=True,exist_ok=False)
frames=[120,600,1200,1800,2520];reports={};images={};binaries={}
for name,binary in [('A0',before),('A1',before),('B',after)]:
 env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))}
 env.update(VOXY_STREET_MATERIALS=str(materials.resolve()),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_HDR_CHECK_DIR=str((output/name).resolve()),VOXY_STREET_BENCHMARK_REPORT=str((output/f'{name}.json').resolve()))
 if name=='B':env['VOXY_STREET_GI_PREFETCH']='1'
 binaries[name]=hashlib.sha256(binary.read_bytes()).hexdigest()
 with (output/f'{name}.log').open('w') as log:subprocess.run([str(binary.resolve()),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','20'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 reports[name]=json.loads((output/f'{name}.json').read_text());images[name]={frame:hashlib.sha256((output/name/f'frame-{frame}.pfm').read_bytes()).hexdigest() for frame in frames}
 print(name,images[name],flush=True)
result={'scope':'five full linear Float32 HDR RGB images per deterministic complete route; no performance qualification',
        'display_pfm_scope':'linear final output after display transform; native pre-TAA stages are evaluated separately',
        'binary_sha256':binaries,'frame_sha256':images,'aa_byte_equal':images['A0']==images['A1'],'ab_byte_equal':images['A0']==images['B'],
        'same_replay':len({d['replay_state_blake3'] for d in reports.values()})==1,
        'all_captures_marked':all(d.get('correctness_capture_active') is True for d in reports.values())}
sys.path.insert(0,str(Path(__file__).resolve().parent.parent/'2026-10-27-inline-gi-120'))
from analyze_quality import analyze
result.update(analyze(output))
result['passed']=all(result[k] for k in ['native_quality_bound_passed','same_replay','all_captures_marked']);(output/'quality.json').write_text(json.dumps(result,indent=2)+'\n')
if not result['passed']:raise SystemExit('Native quality bound failed')

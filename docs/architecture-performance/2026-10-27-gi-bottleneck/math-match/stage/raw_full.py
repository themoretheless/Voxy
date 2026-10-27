import hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parent;binary=Path(sys.argv[1]).resolve();materials=Path(sys.argv[2]).resolve();out=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=False)
reports={};binaries={}
assets={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}
for name,mode in [('raw','bb')]:
 env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))};env.update(VOXY_STREET_GI_PRECISION_PROOF='1',VOXY_STREET_GI_RAW_CAPTURE='1',VOXY_STREET_COMPUTE_GI='1',VOXY_STREET_GI_RASTER_MATH='1',VOXY_STREET_GI_STAGE_CROSSOVER='1',VOXY_STREET_MATERIALS=str(materials),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_REFINE_CROSSOVER=mode,VOXY_STREET_HDR_CHECK_DIR=str(out/name),VOXY_STREET_BENCHMARK_REPORT=str(out/f'{name}.json'))
 binaries[name]=hashlib.sha256(binary.read_bytes()).hexdigest()
 with (out/f'{name}.log').open('w') as log:subprocess.run([str(binary),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','20'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 reports[name]=json.loads((out/f'{name}.json').read_text());print(name,'complete',flush=True)
import numpy as np
rows=[]
for frame in [120,600,1200,1800,2520]:
 base=out/'raw'/f'frame-{frame}'
 a=np.fromfile(base/'actual-gi.native.bin',dtype='<f4');b=np.fromfile(base/'reference-gi.native.bin',dtype='<f4')
 assert a.shape==b.shape==(8294400,)
 rows.append(dict(frame=frame,words=len(a),different_words=int(np.count_nonzero(a.view('<u4')!=b.view('<u4'))),finite=bool(np.isfinite(a).all() and np.isfinite(b).all()),reference_positive_coverage_pixels=int(np.count_nonzero(b.reshape(-1,4)[:,3]>0))))
result=dict(frames=rows,binary_sha256=binaries['raw'],assets=assets,replay=reports['raw']['replay_state_blake3'])
result['passed']=all(r['different_words']==0 and r['finite'] and r['reference_positive_coverage_pixels']>=1000 for r in rows)
(out/'raw-proof.json').write_text(json.dumps(result,indent=2)+'\n');print(result,flush=True)
if assets!={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}:raise SystemExit('Assets changed')
if hashlib.sha256(binary.read_bytes()).hexdigest()!=binaries['raw']:raise SystemExit('Executable changed')
if not result['passed']:raise SystemExit('Raw precision failed')

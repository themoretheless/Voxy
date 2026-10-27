import os,subprocess,json,hashlib
from pathlib import Path
r=Path(__file__).resolve().parent;binary=Path('/tmp/voxy-inline-retained-20261027');manifest={'scope':'ablation diagnostics; disabled GI is not a candidate','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'runs':[]}
for name,flags in [('A0',{}),('noGI0',{'VOXY_FX_OFF':'1'}),('noTiming0',{'VOXY_STREET_GPU_TIMING':None}),('noTiming1',{'VOXY_STREET_GPU_TIMING':None}),('noGI1',{'VOXY_FX_OFF':'1'}),('A1',{})]:
 e={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))};e.update(VOXY_STREET_MATERIALS='/Users/themoretheless/Documents/Sources/voxy/assets/environments/original-street/materials',VOXY_STREET_GPU_TIMING='1',VOXY_STREET_GPU_TIMING_TRACE='1',VOXY_STREET_BENCHMARK_REPORT=str(r/f'{name}.json'))
 for k,v in flags.items():
  if v is None:e.pop(k,None)
  else:e[k]=v
 with (r/f'{name}.log').open('w') as log:p=subprocess.run([str(binary),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','2'],env=e,stdout=log,stderr=subprocess.STDOUT)
 manifest['runs'].append({'name':name,'flags':flags,'exit_code':p.returncode});(r/'profile-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 if p.returncode:raise SystemExit(p.returncode)
 d=json.loads((r/f'{name}.json').read_text());print(name,d['p50_ms'],d['p99_ms'],d['gpu_mean_ms'],flush=True)

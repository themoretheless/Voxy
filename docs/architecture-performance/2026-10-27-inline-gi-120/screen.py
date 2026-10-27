"""Short ABBA screening only; never qualifies full-cycle 120 Hz."""
import hashlib,json,os,subprocess,sys
from pathlib import Path
before,after,output,materials=map(Path,sys.argv[1:])
output.mkdir(parents=True,exist_ok=False)
manifest={'scope':'short screening, not qualification','runs':[],'binaries':{k:{'path':str(p.resolve()),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for k,p in [('A',before),('B',after)]}}
for name,binary in [('A0',before),('B0',after),('B1',after),('A1',before)]:
 env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))}
 env.update(VOXY_STREET_MATERIALS=str(materials.resolve()),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_BENCHMARK_REPORT=str((output/f'{name}.json').resolve()))
 with (output/f'{name}.log').open('w') as log:r=subprocess.run([str(binary.resolve()),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','2'],env=env,stdout=log,stderr=subprocess.STDOUT)
 manifest['runs'].append({'name':name,'exit_code':r.returncode});(output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 if r.returncode:raise SystemExit(r.returncode)
 d=json.loads((output/f'{name}.json').read_text());print(name,d['p50_ms'],d['p99_ms'],d['gpu_mean_ms'],flush=True)

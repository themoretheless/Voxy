import hashlib,json,os,subprocess,sys,time
from pathlib import Path
binary=Path(sys.argv[1]).resolve();materials=Path(sys.argv[2]).resolve();out=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=False)
root=Path(__file__).resolve().parent;protocol=json.loads((root/'protocol.json').read_text())
assets={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}
manifest={'binary':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'assets':assets,'protocol':protocol,'runs':[]}
for name,mode in protocol['schedule']:
 env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))};env.update(VOXY_STREET_MATERIALS=str(materials),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_REFINE_CROSSOVER=mode,VOXY_STREET_BENCHMARK_REPORT=str(out/f'{name}.json'))
 start=time.time()
 with (out/f'{name}.log').open('w') as log:p=subprocess.run([str(binary),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','20'],env=env,stdout=log,stderr=subprocess.STDOUT)
 manifest['runs'].append(dict(name=name,mode=mode,exit_code=p.returncode,start=start,end=time.time()));(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 if p.returncode:raise SystemExit(p.returncode)
 d=json.loads((out/f'{name}.json').read_text());print(name,'p99',d['p99_ms'],'gpu',d['gpu_mean_ms'],flush=True)
 if hashlib.sha256(binary.read_bytes()).hexdigest()!=manifest['sha256']:raise SystemExit('Executable changed')
if assets!={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(materials.iterdir()) if p.is_file()}:raise SystemExit('Assets changed')

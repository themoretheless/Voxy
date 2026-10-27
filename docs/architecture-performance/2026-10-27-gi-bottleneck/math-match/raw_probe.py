import hashlib,json,os,subprocess,sys
from pathlib import Path
import numpy as np
root=Path(__file__).resolve().parent;binary=Path(sys.argv[1]).resolve();materials=Path(sys.argv[2]).resolve();manifest={'scope':'native per-source Float32 opaque GI precision, not performance','sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'runs':[]}
for name,fast in [('safe-A0',False),('safe-A1',False),('raster-B',True)]:
 out=root/name;env={k:v for k,v in os.environ.items() if not k.startswith(('VOXY_STREET_','VOXY_FX_'))};env.update(VOXY_STREET_COMPUTE_GI='1',VOXY_STREET_GI_PRECISION_PROOF='1',VOXY_STREET_GI_RAW_CAPTURE='1',VOXY_STREET_MATERIALS=str(materials),VOXY_STREET_GPU_TIMING='1',VOXY_STREET_HDR_CHECK_DIR=str(out),VOXY_STREET_BENCHMARK_REPORT=str(root/f'{name}.json'))
 if fast:env['VOXY_STREET_GI_RASTER_MATH']='1'
 with (root/f'{name}.log').open('w') as log:p=subprocess.run([str(binary),'--offscreen-benchmark','--offscreen-render-every-step','--benchmark','.1'],env=env,stdout=log,stderr=subprocess.STDOUT)
 if p.returncode:raise SystemExit(p.returncode)
 base=out/'frame-120';a=np.fromfile(base/'actual-gi.native.bin',dtype='<f4');b=np.fromfile(base/'reference-gi.native.bin',dtype='<f4');assert a.shape==b.shape==(8294400,)
 mask=a.view('<u4')!=b.view('<u4');indices=np.flatnonzero(mask);result=dict(name=name,match_raster=fast,words=len(a),different_words=int(mask.sum()),max_absolute_error=float(np.max(np.abs(a.astype(np.float64)-b.astype(np.float64)))),finite=bool(np.isfinite(a).all() and np.isfinite(b).all()),first_difference=None if len(indices)==0 else dict(word=int(indices[0]),actual=float(a[indices[0]]),reference=float(b[indices[0]])),actual_sha256=hashlib.sha256((base/'actual-gi.native.bin').read_bytes()).hexdigest(),reference_sha256=hashlib.sha256((base/'reference-gi.native.bin').read_bytes()).hexdigest())
 manifest['runs'].append(result);(root/'raw-proof.json').write_text(json.dumps(manifest,indent=2)+'\n');print(result,flush=True)

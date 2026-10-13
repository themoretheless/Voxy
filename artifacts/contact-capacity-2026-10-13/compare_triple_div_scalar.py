import os,re,json,statistics,subprocess
from pathlib import Path
root=Path(__file__).resolve().parent
exe='/tmp/voxy-face-refit-20261011/release/deps/voxy_app-53370c422bb1c776'
env=os.environ.copy();env['VOXY_HAIR_TRIPLE_PARALLEL']='1';env['VOXY_HAIR_QR_INPUT_FIXTURE']=str(root/'large-operator-capture/captured-original-input.bin')
results=[]
for index,reuse in enumerate([False,True,True,False,False,True,True,False]):
 selected=env.copy();selected.pop('VOXY_HAIR_TRIPLE_SHADER_FIXTURE',None)
 selected['VOXY_HAIR_TRIPLE_SHADER_FIXTURE']=str(root/('triple-div-scalar-candidate.wgsl' if reuse else 'triple-parallel-before-scalar-product.wgsl'))
 process=subprocess.run([exe,'gpu_triple_active_admits_captured_original_physics','--ignored','--nocapture'],env=selected,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 name=f'triple-div_scalar-comparison-{index}.log';(root/name).write_text(process.stdout)
 match=re.search(r'accelerated=true submissions=1 elapsed_ms=([0-9.]+) error=None',process.stdout)
 record=dict(index=index,div_scalar_reuse=reuse,exit_code=process.returncode,elapsed_ms=float(match.group(1)) if match else None,log=name);results.append(record)
 print(record,flush=True)
 (root/'triple-div_scalar-comparison.json').write_text(json.dumps(dict(runs=results),indent=2))
 if process.returncode or not match:raise SystemExit('original physical gate failed')
before=statistics.median(r['elapsed_ms'] for r in results if not r['div_scalar_reuse']);after=statistics.median(r['elapsed_ms'] for r in results if r['div_scalar_reuse'])
report=dict(scope='one original physical operator; alternating wall-clock measurements; not full trajectory or FPS',runs=results,before_median_ms=before,after_median_ms=after,speedup=before/after)
(root/'triple-div_scalar-comparison.json').write_text(json.dumps(report,indent=2));print(report,flush=True)

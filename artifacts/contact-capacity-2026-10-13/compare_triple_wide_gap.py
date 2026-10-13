import os,re,json,statistics,subprocess,hashlib
from pathlib import Path
root=Path(__file__).resolve().parent
exe=Path('/tmp/voxy-face-refit-20261011/release/deps/voxy_app-53370c422bb1c776')
env=os.environ.copy()
for k in ['VOXY_HAIR_TRIPLE_OPERATOR_REUSE','VOXY_HAIR_JOINT_STAGE_PROFILE']:env.pop(k,None)
env['VOXY_HAIR_TRIPLE_PARALLEL']='1'
env['VOXY_HAIR_QR_INPUT_FIXTURE']=str(root/'large-operator-capture/captured-original-input.bin')
report={'scope':'one original physical operator, host wall time, not FPS','executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'runs':[]}
for index,lanes in enumerate([32,16,64,128,256,256,128,64,16,32,32,64,16,256,128,128,256,16,64,32]):
 source=root/f'triple-wide-gap-{lanes}.wgsl';env['VOXY_HAIR_TRIPLE_SHADER_FIXTURE']=str(source)
 p=subprocess.run([str(exe),'gpu_triple_active_admits_captured_original_physics','--ignored','--nocapture'],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 name=f'triple-wide-gap-run-{index}.log';(root/name).write_text(p.stdout)
 match=re.search(r'accelerated=true submissions=1 elapsed_ms=([0-9.]+) error=None',p.stdout)
 record={'index':index,'lanes_per_row':lanes,'elapsed_ms':float(match.group(1)) if match else None,'exit_code':p.returncode,'log':name,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest()};report['runs'].append(record);print(record,flush=True)
 (root/'triple-wide-gap-comparison.json').write_text(json.dumps(report,indent=2))
 if p.returncode or not match:raise SystemExit('physical admission failed')
report['medians_ms']={str(n):statistics.median(x['elapsed_ms'] for x in report['runs'] if x['lanes_per_row']==n) for n in [16,32,64,128,256]}
(root/'triple-wide-gap-comparison.json').write_text(json.dumps(report,indent=2));print(report['medians_ms'])

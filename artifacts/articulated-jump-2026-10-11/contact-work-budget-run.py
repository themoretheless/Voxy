import hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
root=Path.cwd()
base=root/'artifacts/articulated-jump-2026-10-11'
log=(base/'contact-work-budget-app-build.log').read_text()
m=re.findall(r'Executable unittests .*? \(([^)]+)\)',log)
assert m and 'Finished' in log,'successful app build required'
d=base/'contact-work-budget-full-coupled-cycle'
d.mkdir(exist_ok=False)
binary=d/'app-tests.bin'
shutil.copy2(root/m[-1],binary)
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sources={str(p.relative_to(root)):h(p) for c in ['physics','voxy_app'] for p in (root/'crates'/c).rglob('*.rs')}
sources.update({p:h(root/p) for p in ['Cargo.lock','Cargo.toml']})
command=[str(binary),'female_demo::authored_jump_integration_tests::authored_jump_full_coupled_cycle_qualification','--exact','--ignored','--nocapture']
env=os.environ.copy()
env.update(VOXY_AUTHORED_JUMP_CONTINUOUS_CONTACTS='1',VOXY_AUTHORED_JUMP_QUALIFICATION_DIR=str(d),VOXY_HAIR_CONTACT_REPLAY_EXPORT=str(d/'failed-contact.vhr'),VOXY_HAIR_NONLINEAR_MERIT_TRACE='1',VOXY_HAIR_SWEEP_REFINEMENT_TRACE='1')
(d/'run-manifest.json').write_text(json.dumps(dict(command=command,sources=sources,binary_sha256=h(binary),scope='Full 720-frame CPU coupled trajectory; not rendered FPS or nonlinear stationarity proof'),indent=2))
start=time.time()
with (d/'qualification.log').open('w') as out:
 proc=subprocess.Popen(command,env=env,stdout=out,stderr=subprocess.STDOUT)
 (d/'process.json').write_text(json.dumps(dict(pid=proc.pid,started_unix=start)))
 code=proc.wait()
(d/'terminal.json').write_text(json.dumps(dict(exit_code=code,elapsed_seconds=time.time()-start),indent=2))
print(json.dumps(dict(exit_code=code,directory=str(d))))

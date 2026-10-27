import subprocess,sys,time
from pathlib import Path
r=Path(__file__).resolve().parent
while not (r/'quality-route/quality.json').exists():time.sleep(2)
b='/tmp/voxy-gi-stage-final-20261027';m='/tmp/voxy-street-materials-frozen-20261027-gi';py='/Users/themoretheless/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'
for script,out in [('raw_full.py','raw-route'),('run.py','runs')]:
 print('START',script,flush=True)
 subprocess.run([py,str(r/script),b,m,str(r/out)],check=True)
 print('DONE',script,flush=True)
with (r/'qualification.json').open('w') as f:subprocess.run([py,str(r/'analyze.py'),str(r/'runs')],stdout=f,check=True)

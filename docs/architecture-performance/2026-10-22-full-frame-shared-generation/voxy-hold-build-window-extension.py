import subprocess,os,signal,time,json
from pathlib import Path
root=Path('/Users/themoretheless/Documents/Sources/voxy/docs/architecture-performance/2026-10-22-full-frame-shared-generation')
record_file=root/'paused-common-builds.json';release=root/'release-build-window';deadline=time.monotonic()+180
known=set();stopping=False
for sig in (signal.SIGTERM,signal.SIGINT):
 signal.signal(sig,lambda *_:globals().__setitem__('stopping',True))
try:
 while not stopping and not release.exists() and time.monotonic()<deadline:
  rows={}
  for line in subprocess.check_output(['ps','-axo','pid,ppid,state,comm'],text=True).splitlines()[1:]:
   f=line.split(None,3)
   if len(f)==4:rows[int(f[0])]=(int(f[1]),f[2],f[3])
  for pid,(parent,state,comm) in rows.items():
   if Path(comm).name not in ('cargo','cargo-clippy') or pid in known:continue
   cwd=subprocess.run(['lsof','-a','-p',str(pid),'-d','cwd','-Fn'],capture_output=True,text=True).stdout
   if '/Documents/Sources/' in cwd or '/Documents/ChatGPT/' in cwd:known.add(pid)
  owned={p for p in known if p in rows and Path(rows[p][2]).name in ('cargo','cargo-clippy')}
  while True:
   new=owned|{p for p,(parent,*_) in rows.items() if parent in owned}
   if new==owned:break
   owned=new
  d=json.loads(record_file.read_text());changed=False
  for pid in sorted(owned,key=lambda p:(p not in known,p)):
   if any(c in rows[pid][1] for c in 'TZ'):continue
   identity=subprocess.run(['ps','-p',str(pid),'-o','pid=,ppid=,state=,lstart=,comm='],capture_output=True,text=True).stdout.strip()
   if not identity:continue
   try:os.kill(pid,signal.SIGSTOP)
   except ProcessLookupError:continue
   d['processes'].append(dict(pid=pid,identity=identity,signal='SIGSTOP',time=time.time(),initiator='user-authorized temporary common build window'))
   changed=True;print('paused',identity,flush=True)
  if changed:record_file.write_text(json.dumps(d,indent=2)+'\n')
  time.sleep(.4)
finally:
 d=json.loads(record_file.read_text());resumed=[];seen=set()
 for rec in reversed(d['processes']):
  pid=rec['pid']
  if pid in seen:continue
  seen.add(pid)
  identity=subprocess.run(['ps','-p',str(pid),'-o','pid=,ppid=,state=,lstart=,comm='],capture_output=True,text=True).stdout.strip()
  before=rec['identity'].split(None,3);now=identity.split(None,3)
  if len(now)==4 and now[3]==before[3] and 'T' in now[2]:
   try:os.kill(pid,signal.SIGCONT);resumed.append(dict(pid=pid,time=time.time(),signal='SIGCONT'))
   except ProcessLookupError:pass
 (root/'resumed-common-builds.json').write_text(json.dumps(dict(reason='release requested' if release.exists() else 'safety deadline or controller stop',processes=resumed),indent=2)+'\n')

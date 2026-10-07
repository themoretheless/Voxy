"""Supervise the existing native capture entrypoint and preserve its terminal code."""
import datetime, hashlib, json, pathlib, subprocess, sys, os
root=pathlib.Path.cwd()
output=pathlib.Path(__file__).resolve().parent
binary=pathlib.Path(sys.argv[1]).resolve()
if (output/'launch.json').exists():
    raise RuntimeError('capture already launched; inspect its process and status before another run')
command=[str(binary),str(output/'capture.png'),str(output/'frames'),'--cesium','--contact','--capture-steps=480','--tissue-regions='+str(root/'artifacts/tissue-quality-remesh-2026-10-07/candidate-regions.json')]
with (output/'runtime.log').open('w') as stream:
    environment=os.environ.copy()
    environment["VOXY_CAPTURE_NODE_STATE"]="1"
    environment["VOXY_REFINEMENT_DIAGNOSTICS"]="1"
    child=subprocess.Popen(command,cwd=root,stdout=stream,stderr=subprocess.STDOUT,env=environment)
    sources=['crates/physics/src/biomechanics/harmonic_reference.rs','crates/voxy_app/src/tissue_demo/assembly.rs','crates/voxy_app/examples/body_motion_snapshot.rs']
    launch={'capture_environment':{'VOXY_CAPTURE_NODE_STATE':'1','VOXY_REFINEMENT_DIAGNOSTICS':'1'},'pid':child.pid,'supervisor_pid':__import__('os').getpid(),'command':command,'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'source_sha256':{p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sources},'requested_steps':480,'nodes':3227,'cells':12284,'energy_budget_rate_j_s':5e-10,'source_reference_relative_residual':1e-12,'self_contact_qualified':False,'full_clip_qualified':False,'realtime_qualified':False,'baseline_pid_preserved':90673}
    (output/'launch.json').write_text(json.dumps(launch,indent=2)+'\n')
    code=child.wait()
    (output/'terminal.json').write_text(json.dumps({'exit_code':code,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'process terminal status; inspect captures and audits before qualification'},indent=2)+'\n')

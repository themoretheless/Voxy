#!/usr/bin/env python3
"""One preregistered confirmation, bracketed by controls; never retry noisy runs."""
import argparse,gzip,hashlib,json,os,random,statistics,subprocess,sys,threading
from pathlib import Path
sys.dont_write_bytecode=True
sys.path.insert(0,str(Path(__file__).resolve().parent.parent/'2026-10-25-direct-shared-followup'))
# A distinct import name avoids importing this script as qualify.py.
import importlib.util
spec=importlib.util.spec_from_file_location('prior_upload_protocol',Path(__file__).resolve().parent.parent/'2026-10-25-direct-shared-followup/qualify.py')
prior=importlib.util.module_from_spec(spec);spec.loader.exec_module(prior)
EXPECTED='faa40b0726c3283bbb05d7e0b168f634bdd4f621c9a8aaf634315e17524c16c3'
ORDER=[('control',1),('upload',1),('upload',2),('control',2)]
BOUNDS={'cpu_p50':1.05,'gpu_p50':1.05,'completion_p50':1.05,'gpu_gap_p50':1.05,'gpu_gap_p99':1.10}

def decision(manifest,cases):
    assert [(e['stage'],e['repeat']) for e in manifest]==ORDER,'incomplete, duplicated or reordered experiment'
    assert {e['binary_sha256'] for e in manifest}=={EXPECTED},'binary changed'
    assert all(e['correctness_passed'] and e['schedule_valid'] for e in manifest),'validation failed'
    assert len(cases)==2 and {c['stage'] for c in cases}=={'control','upload'}
    assert all(c['pairs']==36 and c['frames_per_mode']==3456 for c in cases),'sample count changed'
    controls=all(e['control_stable'] for e in manifest if e['stage']=='control')
    upload=next(c for c in cases if c['stage']=='upload')
    gates={m:upload['metrics'][m]['ci95'][1]<=bound for m,bound in BOUNDS.items()}
    return dict(complete=True,controls_stable=controls,upload_gates=gates,relative_p99_qualified=controls and gates['gpu_gap_p99'],qualified=controls and all(gates.values()),scope='relative noninferiority at 120 Hz offscreen under observed load; no absolute 120 FPS claim')

def trace(path):
    return (path.read_text() if path.exists() else gzip.decompress(path.with_name(path.name+'.gz').read_bytes()).decode()).splitlines()

def validate(path):
    lines=trace(path);assert 'FULL_PACER,bounded,global_deadlines_retained' in lines
    full=[l.split(',') for l in lines if l.startswith('FULL,')];pace=[l.split(',') for l in lines if l.startswith('PACE,')]
    assert len(full)==len(pace)==3456
    assert all(int(p[7])==1 for p in pace)
    assert all(float(f[16])>=4166.665 for f in full if int(f[7])>0)
    full_keys={tuple(f[i] for i in [4,5,6,7]) for f in full}
    pace_keys={tuple(p[i] for i in [3,4,5,6]) for p in pace}
    assert len(full_keys)==len(pace_keys)==3456 and full_keys==pace_keys
    return True

def summarize(out):
    manifest=json.loads((out/'manifest.json').read_text());rng=random.Random(20261026);cases=[]
    for stage in ['control','upload']:
        entries=[e for e in manifest if e['stage']==stage];assert len(entries)==2
        pairs=[p for e in entries for p in prior.parse(out/(e['stem']+'.log')).values()];assert len(pairs)==36
        metrics={}
        for name in prior.METRICS:
            ratios=[p['after'][name]/p['before'][name] for p in pairs]
            boot=sorted(statistics.median(rng.choices(ratios,k=36)) for _ in range(10000))
            metrics[name]=dict(before=statistics.median(p['before'][name] for p in pairs),after=statistics.median(p['after'][name] for p in pairs),ratio=statistics.median(ratios),ci95=[boot[249],boot[9749]])
        telemetry={mode:dict(frames=3456,grid_deadline_misses=sum(p[mode]['misses'] for p in pairs),assigned_deadline_misses=0,catch_up_starts=0) for mode in ['before','after']}
        for entry in entries:
            path=out/(entry['stem']+'.log');assert validate(path)
            for line in trace(path):
                f=line.split(',')
                if f[0]=='PACE':telemetry[f[5]]['assigned_deadline_misses']+=int(f[10])
                elif f[0]=='FULL' and int(f[7])>0:telemetry[f[6]]['catch_up_starts']+=float(f[16])<4166.666
        cases.append(dict(stage=stage,pairs=36,frames_per_mode=3456,metrics=metrics,telemetry=telemetry))
    result=decision(manifest,cases);result.update(cases=cases,binary_sha256=EXPECTED,isolated=False)
    (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');return result

def run(binary,out):
    binary=binary.resolve();assert hashlib.sha256(binary.read_bytes()).hexdigest()==EXPECTED
    out.mkdir(parents=True,exist_ok=False);manifest=[]
    for stage,repeat in ORDER:
        stem=f'{stage}-r{repeat}';env=os.environ.copy();env.update(VOXY_FULL_PHASE='aa' if stage=='control' else 'ab',VOXY_FULL_PROTOCOL='paced',VOXY_FULL_PACER='bounded',VOXY_FULL_BLOCKS='2',VOXY_FULL_PAIRS='18')
        observations=[];errors=[];stop=threading.Event()
        def watch():
            try:
                while not stop.is_set():observations.append(prior.snapshot(binary));stop.wait(.2)
            except Exception as e:errors.append(str(e))
        watcher=threading.Thread(target=watch);watcher.start()
        try:
            with (out/(stem+'.log')).open('w') as f:res=subprocess.run([str(binary),'paired_full_frame_particles','--ignored','--nocapture','--test-threads=1'],env=env,stdout=f,stderr=subprocess.STDOUT)
        finally:stop.set();watcher.join()
        (out/(stem+'-clients.json.gz')).write_bytes(gzip.compress(json.dumps(observations).encode(),mtime=0))
        assert res.returncode==0 and observations and not errors,(stem,res.returncode,errors)
        data=prior.parse(out/(stem+'.log'));stable,ratios=prior.aa_stable(data,'paced')
        entry=dict(stage=stage,repeat=repeat,stem=stem,binary_sha256=EXPECTED,correctness_passed=True,schedule_valid=validate(out/(stem+'.log')),control_stable=stable if stage=='control' else None,ratios=ratios)
        manifest.append(entry);(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(entry),flush=True)
    result=summarize(out);print(json.dumps(result),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--summarize-only',action='store_true');a=p.parse_args()
    if a.summarize_only:print(json.dumps(summarize(a.output)))
    else:run(a.binary,a.output)

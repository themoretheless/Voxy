#!/usr/bin/env python3
"""Fixed observed-load experiment: upload A/A, pacing-only A/B, upload A/B."""
import argparse,gzip,hashlib,json,os,random,statistics,subprocess,sys,threading,time
from pathlib import Path
sys.dont_write_bytecode=True
sys.path.insert(0,str(Path(__file__).resolve().parent.parent/'2026-10-25-direct-shared-followup'))
from qualify import parse,ratios,snapshot,METRICS,aa_stable

def run(binary,out):
    binary=binary.resolve();out.mkdir(parents=True,exist_ok=False)
    digest=hashlib.sha256(binary.read_bytes()).hexdigest();manifest=[]
    (out/'binary.sha256').write_text(digest+'\n')
    for stage,phase,pacer in [('control','aa','bounded'),('pacer','aa','compare'),('upload','ab','bounded')]:
        for repeat in [1,2]:
            stem=f'{stage}-r{repeat}';env=os.environ.copy();env.update(VOXY_FULL_PHASE=phase,VOXY_FULL_PROTOCOL='paced',VOXY_FULL_PACER=pacer,VOXY_FULL_BLOCKS='2',VOXY_FULL_PAIRS='18')
            observations=[];stop=threading.Event()
            def watch():
                while not stop.is_set():
                    observations.append(snapshot(binary));stop.wait(.2)
            worker=threading.Thread(target=watch);worker.start()
            try:
                with (out/(stem+'.log')).open('w') as f:
                    result=subprocess.run([str(binary),'paired_full_frame_particles','--ignored','--nocapture','--test-threads=1'],env=env,stdout=f,stderr=subprocess.STDOUT)
            finally:stop.set();worker.join()
            (out/(stem+'-clients.json.gz')).write_bytes(gzip.compress(json.dumps(observations).encode(),mtime=0))
            assert result.returncode==0,(stem,result.returncode)
            data=parse(out/(stem+'.log'));stable,r=aa_stable(data,'paced')
            entry=dict(stage=stage,repeat=repeat,stem=stem,binary_sha256=digest,ratios=r,control_stable=stable if stage=='control' else None)
            manifest.append(entry);(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(entry),flush=True)
    summarize(out)

def summarize(out):
    manifest=json.loads((out/'manifest.json').read_text());rng=random.Random(20261026);cases=[]
    assert len(manifest)==6
    for stage in ['control','pacer','upload']:
        entries=[e for e in manifest if e['stage']==stage];assert len(entries)==2
        pairs=[p for e in entries for p in parse(out/(e['stem']+'.log')).values()];assert len(pairs)==36
        metrics={}
        for name in METRICS:
            r=[p['after'][name]/p['before'][name] for p in pairs];boot=sorted(statistics.median(rng.choices(r,k=36)) for _ in range(10000))
            metrics[name]=dict(before=statistics.median(p['before'][name] for p in pairs),after=statistics.median(p['after'][name] for p in pairs),ratio=statistics.median(r),ci95=[boot[249],boot[9749]])
        telemetry={m:dict(catch_up_starts=0,assigned_deadline_misses=0,grid_deadline_misses=sum(p[m]['misses'] for p in pairs),frames=3456,min_start_gap_us=float('inf'),late_start_over_period=0) for m in ['before','after']}
        for e in entries:
            path=out/(e['stem']+'.log');text=path.read_text() if path.exists() else gzip.decompress(path.with_name(path.name+'.gz').read_bytes()).decode()
            for line in text.splitlines():
                f=line.split(',')
                if f[0]=='FULL':
                    mode=f[6];gap=float(f[16])
                    if int(f[7])>0:
                        telemetry[mode]['catch_up_starts']+=gap<4166.666
                        telemetry[mode]['min_start_gap_us']=min(telemetry[mode]['min_start_gap_us'],gap)
                elif f[0]=='PACE':
                    mode=f[5];telemetry[mode]['assigned_deadline_misses']+=int(f[10]);telemetry[mode]['late_start_over_period']+=float(f[9])>8333.333
        gates={m:metrics[m]['ci95'][1]<=limit for m,limit in [('cpu_p50',1.05),('gpu_p50',1.05),('completion_p50',1.05),('gpu_gap_p50',1.05),('gpu_gap_p99',1.10)]}
        cases.append(dict(stage=stage,pairs=36,frames_per_mode=3456,metrics=metrics,telemetry=telemetry,gates=gates))
    result=dict(complete=True,isolated=False,controls_stable=all(e['control_stable'] for e in manifest if e['stage']=='control'),cases=cases)
    (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--summarize-only',action='store_true');a=p.parse_args()
    if a.summarize_only:summarize(a.output)
    else:run(a.binary,a.output)

#!/usr/bin/env python3
"""Immediate paired full-pass qualification. Never signals competing processes."""
import argparse
import gzip
from collections import defaultdict
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re
import statistics
import subprocess
import threading
import time

METRICS = ['cpu_p50','gpu_p50','completion_p50','completion_p99','gpu_gap_p50','gpu_gap_p95','gpu_gap_p99','start_gap_p99']

def quantile(values,p):
    a=sorted(values)
    if not a: raise ValueError('empty sample')
    x=(len(a)-1)*p; lo=int(x);hi=math.ceil(x)
    return a[lo]+(a[hi]-a[lo])*(x-lo)

def parse(path):
    pairs=defaultdict(lambda:defaultdict(list))
    path=Path(path)
    text=path.read_text() if path.exists() else gzip.decompress(path.with_name(path.name+'.gz').read_bytes()).decode()
    assert 'FULL_PARITY,' in text and 'test result: ok. 1 passed;' in text
    for line in text.splitlines():
        if line.startswith('FULL,'):
            f=line.split(',');assert len(f)==20,len(f)
            pairs[int(f[4])][f[6]].append(dict(cpu=float(f[8]),gpu=float(f[9]),completion=float(f[10]),
                start_gap=float(f[16]),gpu_gap=float(f[17]),backpressure=float(f[18]),miss=int(f[19]),frame=int(f[7])))
    assert len(pairs)==18
    result={}
    for pair,modes in pairs.items():
        assert set(modes)=={'before','after'}
        result[pair]={}
        for mode,frames in modes.items():
            assert len(frames)==96,len(frames)
            d={}
            for name in METRICS:
                source,p=name.rsplit('_p',1)
                values=[f[source] for f in frames if not source.endswith('_gap') or f['frame']>0]
                d[name]=quantile(values,int(p)/100)
            d['misses']=sum(f['miss'] for f in frames);d['frames']=len(frames)
            result[pair][mode]=d
    return result

def ratios(data):
    return {m:statistics.median(p['after'][m]/p['before'][m] for p in data.values()) for m in METRICS}

def aa_stable(data,protocol):
    r=ratios(data)
    limits={'cpu_p50':.10,'gpu_p50':.05,'completion_p50':.10,'gpu_gap_p50':.05}
    if protocol=='paced':limits.update(gpu_gap_p95=.05,gpu_gap_p99=.10,start_gap_p99=.10)
    return all(abs(r[m]-1)<=limit for m,limit in limits.items()),r

def snapshot(binary):
    rivals=[];clients=[]
    for line in subprocess.check_output(['ps','-axo','pid,ppid,state,%cpu,comm'],text=True).splitlines()[1:]:
        fields=line.split(None,4)
        if len(fields)!=5:continue
        pid,ppid,state,cpu,comm=fields
        if comm==str(binary):continue
        name=Path(comm).name
        compiled=any(x in comm for x in ['/target/','/artifacts/','/output/qualification/','/voxy-shared-20261025-target/'])
        known=any(x in name.lower() for x in ['street_walk','sdf_core','allpaka','llama','voxy_app','cpu-replay','liquid_snapshot'])
        compiler=name in {'rustc','rust-lld','ld','cc','clang','clang++','cc1','gcc','g++','ninja','cmake','rust-objcopy','MTLCompilerService'}
        if name=='ollama' and int(ppid)==1 and float(cpu)<=1 and not any(s in state for s in 'RU'):
            known=False
        if compiled or known or compiler:
            record=dict(pid=int(pid),ppid=int(ppid),state=state,cpu_percent=float(cpu),comm=comm)
            clients.append(record)
            if not any(s in state for s in 'TZ') and (not compiler or any(s in state for s in 'RU') or float(cpu)>.5):rivals.append(record)
    raw=subprocess.check_output(['ioreg','-r','-c','AGXAccelerator','-d','1'],text=True)
    match=re.search(r'"Device Utilization %"=(\d+)',raw)
    if match is None:raise RuntimeError('AGX utilization unavailable')
    return dict(time=time.time(),gpu_percent=int(match[1]),rivals=rivals,clients=clients)

def run(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=args.resume)
    binary=args.binary.resolve()
    digest=hashlib.sha256(binary.read_bytes()).hexdigest()
    if args.resume:
        assert (out/'binary.sha256').read_text().split()[0]==digest, 'resume binary changed'
    (out/'binary.sha256').write_text(digest+'  '+str(binary)+'\n')
    manifest=json.loads((out/'manifest.json').read_text()) if args.resume and (out/'manifest.json').exists() else []
    (out/'status.json').write_text(json.dumps(dict(status='running', binary_sha256=digest))+'\n')
    deadline=time.monotonic()+args.wait_seconds
    log=(out/'observer.jsonl').open('a' if args.resume else 'w')
    def observe(stage):
        s=snapshot(binary);s['stage']=stage;log.write(json.dumps(s)+'\n');log.flush();return s
    def quiet():
        if args.allow_contention:
            observe('admission-contention-allowed');return
        since=None;last_print=0
        while True:
            s=observe('waiting');now=time.monotonic()
            if s['rivals'] or s['gpu_percent']>10:since=None
            elif since is None:since=now
            if since is not None and now-since>=5:return
            if now-last_print>=30:
                print(json.dumps(dict(waiting=True,gpu=s['gpu_percent'],rivals=s['rivals'])),flush=True);last_print=now
            if now>deadline:raise TimeoutError('no uncontended GPU admission window')
            time.sleep(.25)
    try:
        # ALL A/A controls precede A/B, with both scheduling protocols and owners.
        for phase in ['aa','ab']:
            for repeat in [1,2]:
                for protocol in ['throughput','paced']:
                    for owner in ['instances','particles']:
                        matches=[e for e in manifest if e['phase']==phase and e['repeat']==repeat and e['protocol']==protocol and e['owner']==owner]
                        if any(e['accepted'] for e in matches): continue
                        accepted=False
                        first=max([int(e['stem'].rsplit('-a',1)[1]) for e in matches],default=0)+1
                        for attempt in range(first,first+4):
                            quiet()
                            stem=f'{phase}-{owner}-{protocol}-r{repeat}-a{attempt}'
                            env=dict(os.environ,VOXY_FULL_PHASE=phase,VOXY_FULL_PROTOCOL=protocol,VOXY_FULL_BLOCKS='2',VOXY_FULL_PAIRS='18')
                            monitor=[];stop=threading.Event();contaminated=[]
                            with (out/f'{stem}.log').open('w') as output:
                                child=subprocess.Popen([str(binary),'paired_full_frame_'+owner,'--ignored','--nocapture','--test-threads=1'],env=env,stdout=output,stderr=subprocess.STDOUT)
                                def watching():
                                    while not stop.is_set():
                                        s=snapshot(binary);monitor.append(s)
                                        if s['rivals'] and not args.allow_contention:
                                            contaminated.append(s)
                                            if child.poll() is None:
                                                # Signal only this exact process we just spawned.
                                                child.terminate()
                                            return
                                        stop.wait(.2)
                                worker=threading.Thread(target=watching);worker.start()
                                code=child.wait();stop.set();worker.join()
                            (out/f'{stem}-clients.json').write_text(json.dumps(monitor,indent=2)+'\n')
                            entry=dict(stem=stem,phase=phase,repeat=repeat,protocol=protocol,owner=owner,exit_code=code,binary_sha256=digest)
                            if contaminated:
                                entry.update(accepted=False,reason='competing executable appeared; own child received SIGTERM',signal_initiator='this controller',signalled_pid=child.pid)
                            elif code:
                                raise RuntimeError(f'full-frame validation failed: {stem}, exit={code}')
                            else:
                                # Observe outside our completed work, not while our own GPU is busy.
                                time.sleep(1)
                                post=[observe('post-run') for _ in range(3)]
                                if args.allow_contention:
                                    data=parse(out/f'{stem}.log')
                                    stable,control=aa_stable(data,protocol)
                                    entry.update(accepted=True,ratios=control,control_stable=stable if phase=='aa' else None,contention_allowed=True,reason='completed under observed load; no isolation claim')
                                elif any(s['rivals'] for s in post):
                                    entry.update(accepted=False,reason='post-run rival activity')
                                elif any(s['gpu_percent']>10 for s in post):
                                    entry.update(accepted=False,reason='GPU did not drain to idle after run')
                                else:
                                    data=parse(out/f'{stem}.log')
                                    stable,control=aa_stable(data,protocol)
                                    entry.update(accepted=stable if phase=='aa' else True,ratios=control,reason='accepted' if phase!='aa' or stable else 'unstable A/A control')
                            manifest.append(entry);(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
                            print(json.dumps(entry),flush=True)
                            if entry['accepted']:accepted=True;break
                        if not accepted:raise RuntimeError(f'no qualifying attempt for {phase}/{owner}/{protocol}/{repeat}')
        (out/'status.json').write_text(json.dumps(dict(status='complete',binary_sha256=digest),indent=2)+'\n')
    except Exception as error:
        (out/'status.json').write_text(json.dumps(dict(status='incomplete',reason=str(error)),indent=2)+'\n')
        raise
    finally:log.close()

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--wait-seconds',type=int,default=1200);parser.add_argument('--resume',action='store_true');parser.add_argument('--allow-contention',action='store_true')
    run(parser.parse_args())

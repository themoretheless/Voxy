#!/usr/bin/env python3
"""Check real scheduling invariants without filtering any completion gap."""
import argparse,gzip,json,statistics
from pathlib import Path

def lines(path):
    return (path.read_text() if path.exists() else gzip.decompress(path.with_name(path.name+'.gz').read_bytes()).decode()).splitlines()

def quantile(a,p):
    a=sorted(a);x=(len(a)-1)*p;lo=int(x);hi=min(lo+1,len(a)-1)
    return a[lo]+(a[hi]-a[lo])*(x-lo)

def analyze(root):
    manifest=json.loads((root/'manifest.json').read_text());summary=json.loads((root/'summary.json').read_text());assert len(manifest)==6 and summary['complete']
    result={}
    for stage in ['control','pacer','upload']:
        values={m:dict(scheduled_latency_us=[],gaps=[],queue_depths=[],lateness_periods=0,grid_misses=0,assigned_misses=0) for m in ['before','after']}
        for e in [x for x in manifest if x['stage']==stage]:
            full={};pace={}
            for line in lines(root/(e['stem']+'.log')):
                f=line.split(',')
                if f[0]=='FULL':full[tuple(f[i] for i in [4,5,6,7])]=f
                elif f[0]=='PACE':pace[tuple(f[i] for i in [3,4,5,6])]=f
            assert len(full)==len(pace)==3456
            for key,f in full.items():
                p=pace[key];d=values[f[6]];latency=float(f[13])-float(p[8]);assert latency>=0
                d['scheduled_latency_us'].append(latency);d['queue_depths'].append(int(p[7]));d['lateness_periods']+=int(float(p[9])/8333.333);d['grid_misses']+=int(f[19]);d['assigned_misses']+=int(p[10])
                if int(f[7])>0:d['gaps'].append(float(f[16]))
        result[stage]={}
        for mode,d in values.items():
            bounded=stage!='pacer' or mode=='after'
            if bounded:
                assert set(d['queue_depths'])=={1}
                assert min(d['gaps'])>=4166.665, (stage,mode,min(d['gaps']))
            result[stage][mode]=dict(frames=len(d['scheduled_latency_us']),min_start_gap_us=min(d['gaps']),catch_up_starts=sum(g<4166.666 for g in d['gaps']),scheduled_to_completion_p50_us=quantile(d['scheduled_latency_us'],.5),scheduled_to_completion_p99_us=quantile(d['scheduled_latency_us'],.99),assigned_deadline_misses=d['assigned_misses'],original_grid_deadline_misses=d['grid_misses'],lateness_periods=d['lateness_periods'])
    upload=next(c for c in summary['cases'] if c['stage']=='upload')
    output=dict(complete=True,controls_stable=summary['controls_stable'],upload_metric_gates_passed=all(upload['gates'].values()),relative_p99_qualified=summary['controls_stable'] and upload['gates']['gpu_gap_p99'],absolute_120hz_claim=False,details=result)
    (root/'schedule-validation.json').write_text(json.dumps(output,indent=2)+'\n');print(json.dumps(output,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);analyze(p.parse_args().directory)

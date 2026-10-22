#!/usr/bin/env python3
"""Fail-closed noninferiority summary; retain failed and contaminated attempts."""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import random
import statistics
from qualify import METRICS,parse,quantile

def summarize(root):
    manifest=json.loads((root/'manifest.json').read_text())
    accepted=[e for e in manifest if e['accepted']]
    grouped=defaultdict(list)
    for entry in accepted:grouped[entry['phase'],entry['owner'],entry['protocol']].append(entry)
    rng=random.Random(20261022);output=[]
    for (phase,owner,protocol),entries in sorted(grouped.items()):
        assert len(entries)==2, (phase,owner,protocol,len(entries))
        pairs=[p for e in entries for p in parse(root/(e['stem']+'.log')).values()]
        assert len(pairs)==18
        metrics={}
        for name in METRICS:
            r=[p['after'][name]/p['before'][name] for p in pairs]
            boot=sorted(statistics.median(rng.choices(r,k=18)) for _ in range(10000))
            metrics[name]=dict(before=statistics.median(p['before'][name] for p in pairs),after=statistics.median(p['after'][name] for p in pairs),ratio=statistics.median(r),ci95=[boot[249],boot[9749]])
        misses={mode:sum(p[mode]['misses'] for p in pairs) for mode in ['before','after']}
        frames=sum(p['before']['frames'] for p in pairs)
        gates={name:metrics[name]['ci95'][1]<=1.05 for name in ['cpu_p50','gpu_p50','completion_p50','gpu_gap_p50']}
        if protocol=='paced':
            gates['gpu_gap_p99']=metrics['gpu_gap_p99']['ci95'][1]<=1.10
            gates['deadline_misses']=(misses['after']-misses['before'])/frames<=.005
        output.append(dict(phase=phase,owner=owner,protocol=protocol,pairs=18,frames_per_mode=frames,metrics=metrics,deadline_misses=misses,gates=gates,noninferior=all(gates.values()) if phase=='ab' else None))
    complete=len(grouped)==8 and len(accepted)==16
    result=dict(complete=complete,accepted_runs=len(accepted),rejected_runs=len(manifest)-len(accepted),cases=output,qualified=complete and all(c['noninferior'] for c in output if c['phase']=='ab'),scope='offscreen owner full frames; no display/gameplay FPS claim')
    (root/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    for c in output:
        print(c['phase'],c['owner'],c['protocol'],'noninferior',c['noninferior'])
        for m in ['cpu_p50','gpu_p50','completion_p50','gpu_gap_p99']:
            x=c['metrics'][m];print(' ',m,round(x['before'],3),'->',round(x['after'],3),'ratio',round(x['ratio'],4),'CI',[round(n,4) for n in x['ci95']])
        print(' misses',c['deadline_misses'],'gates',c['gates'])
    return result
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path)
    summarize(parser.parse_args().directory)

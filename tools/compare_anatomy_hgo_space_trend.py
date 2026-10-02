#!/usr/bin/env python3
"""Report measured changes between adjacent spatial sensitivity pairs."""
import argparse,json,math
from pathlib import Path

def compare(first,second):
    if first['fine_refinement']!=second['coarse_refinement'] or first['fine_nodes']!=second['coarse_nodes'] or first['fine_pins']!=second['coarse_pins']:
        raise ValueError('middle mesh differs')
    if first['fine_csv_sha256']!=second['coarse_csv_sha256']:
        raise ValueError('middle physical schedule differs')
    old=first['comparisons'];new=second['comparisons']
    if not old or len(old)!=len(new):raise ValueError('schedule lengths differ')
    rows=[]
    for a,b in zip(old,new):
        if a['time_s']!=b['time_s'] or a['stage']!=b['stage'] or a['fine_mesh_sha256']!=b['coarse_mesh_sha256']:
            raise ValueError('middle state or physical time differs')
        x,y=a['max_difference_m'],b['max_difference_m']
        if not all(math.isfinite(v) and v>=0 for v in [x,y]):raise ValueError('invalid difference')
        rows.append(dict(time_s=a['time_s'],stage=a['stage'],first_pair_max_m=x,second_pair_max_m=y,decreases=y<x,ratio=y/x if x>0 else None))
    return dict(scope='Measured two-pair spatial sensitivity trend only; no order, error bound or physiological calibration claim',comparisons=rows,decreases_at_all_times=all(r['decreases'] for r in rows),first_pair_max_m=max(r['first_pair_max_m'] for r in rows),second_pair_max_m=max(r['second_pair_max_m'] for r in rows))

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--first',type=Path,required=True);p.add_argument('--second',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();r=compare(json.loads(a.first.read_text()),json.loads(a.second.read_text()));a.output.write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:v for k,v in r.items() if k!='comparisons'}))
if __name__=='__main__':main()

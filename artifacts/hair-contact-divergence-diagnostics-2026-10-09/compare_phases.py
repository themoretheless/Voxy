"""Compare matched physical phases of a selected hair rod."""
import argparse,ast,json,re
from pathlib import Path

def compare(path):
    stages={}
    for line in Path(path).read_text().splitlines(keepends=True):
        if not line.endswith('\n'):continue
        prefix=re.match(r'HAIR PHASE TRACE frame=(\d+) (native|external) (.*)',line)
        if not prefix:continue
        fields=re.search(r'phase: "([^"]+)", substep: (\d+), iteration: (\d+), rod: (\d+)',prefix[3])
        if not fields:raise ValueError('invalid phase metadata')
        key=(int(prefix[1]),int(fields[2]),int(fields[3]),fields[1],int(fields[4]))
        state={}
        for name in ['positions','velocities','orientations']:
            match=re.search(name+r': (\[\[.*?\]\])',prefix[3])
            if not match:raise ValueError('missing '+name)
            state[name]=ast.literal_eval(match[1])
        sides=stages.setdefault(key,{})
        if prefix[2] in sides:raise ValueError('duplicate phase identity: '+str(key))
        sides[prefix[2]]=state
    rows=[]
    # Preserve emitted native chronology rather than alphabetical phase order.
    for key,sides in stages.items():
        if set(sides)!={'native','external'}:continue
        row={'frame':key[0],'substep':key[1],'iteration':key[2],'phase':key[3],'rod':key[4]}
        for name in ['positions','velocities','orientations']:
            a,b=sides['native'][name],sides['external'][name]
            if len(a)!=len(b):raise ValueError('phase vector shape mismatch')
            error,point,axis=max(((abs(x-y),i,j) for i,(p,q) in enumerate(zip(a,b)) for j,(x,y) in enumerate(zip(p,q))),default=(0,0,0))
            row[name+'_max_delta']=error;row[name+'_worst_point']=point;row[name+'_worst_axis']=axis
        rows.append(row)
    jumps=[]
    for before,after in zip(rows,rows[1:]):
        # Compare adjacent observed physical phases, preserving chronology.
        # These are diagnostics only; they do not change admission thresholds.
        jumps.append({'before':{key:before[key] for key in ['frame','substep','iteration','phase']},
                      'after':{key:after[key] for key in ['frame','substep','iteration','phase']},
                      **{name+'_delta_increase':after[name+'_max_delta']-before[name+'_max_delta'] for name in ['positions','velocities','orientations']}})
    return {'paired_phases':len(rows),'unmatched_phases':sum(set(sides)!={'native','external'} for sides in stages.values()),'first_velocity_delta_over_1um_per_s':next((r for r in rows if r['velocities_max_delta']>1e-6),None),'first_position_delta_over_10nm':next((r for r in rows if r['positions_max_delta']>1e-8),None),'first_position_delta_over_100nm':next((r for r in rows if r['positions_max_delta']>1e-7),None),'largest_position_increases':sorted(jumps,key=lambda row:row['positions_delta_increase'],reverse=True)[:8],'largest_velocity_increases':sorted(jumps,key=lambda row:row['velocities_delta_increase'],reverse=True)[:8],'phases':rows}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('log');parser.add_argument('--output');args=parser.parse_args();result=compare(args.log)
    if args.output:Path(args.output).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k!='phases'},indent=2))

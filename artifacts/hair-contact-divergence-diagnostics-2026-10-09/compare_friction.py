"""Compare per-pair Coulomb inputs in deterministic friction application order."""
import argparse,ast,json,re
from pathlib import Path

def compare(path):
    groups={}
    for line in Path(path).read_text().splitlines(keepends=True):
        if not line.endswith('\n'):continue
        match=re.match(r'HAIR FRICTION TRACE frame=(\d+) (native|external) (.*)',line)
        if not match:continue
        body=match[3];step=int(re.search(r'substep: (\d+)',body)[1]);item={}
        for name in ['a','b']:
            item[name]=ast.literal_eval(re.search(name+r': (\([^)]*\))',body)[1])
        for name in ['normal','relative_velocity','applied_impulse']:
            item[name]=ast.literal_eval(re.search(name+r': (\[[^]]*\])',body)[1])
        for name in ['position_impulse','normal_speed','tangent_speed','mobility_a','mobility_b','friction','normal_impulse','tangent_impulse']:
            item[name]=float(re.search(name+r': ([^, }]+)',body)[1])
        groups.setdefault((int(match[1]),step),{}).setdefault(match[2],[]).append(item)
    rows=[];topology=[]
    identity=lambda item:(item['a'][0],item['a'][1],item['b'][0],item['b'][1])
    for (frame,step),sides in groups.items():
        if set(sides)!={'native','external'}:continue
        a,b=sides['native'],sides['external'];ka,kb=[identity(x) for x in a],[identity(x) for x in b]
        if ka!=kb:topology.append({'frame':frame,'substep':step,'native_count':len(a),'external_count':len(b),'native_only':[k for k in ka if k not in set(kb)],'external_only':[k for k in kb if k not in set(ka)]})
        mapping={identity(item):item for item in b}
        if len(mapping)!=len(b):raise ValueError('duplicate pair identity')
        for sequence,x in enumerate(a):
            key=identity(x)
            if key not in mapping:continue
            y=mapping[key];delta=max(abs(u-v) for u,v in zip(x['applied_impulse'],y['applied_impulse']))
            mobility=max(x['mobility_a']+x['mobility_b'],y['mobility_a']+y['mobility_b'])
            rows.append({'frame':frame,'substep':step,'native_sequence':sequence,'pair':key,'position_impulse_delta':abs(x['position_impulse']-y['position_impulse']),'relative_velocity_delta':max(abs(u-v) for u,v in zip(x['relative_velocity'],y['relative_velocity'])),'applied_impulse_delta':delta,'estimated_relative_contact_velocity_effect_delta':delta*mobility,'native':x,'external':y})
    return {'paired_records':len(rows),'topology_differences':topology,'first_contact_velocity_effect_delta_over_1um_per_s':next((x for x in rows if x['estimated_relative_contact_velocity_effect_delta']>1e-6),None),'records':rows}
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('log');parser.add_argument('--output');args=parser.parse_args();r=compare(args.log)
    if args.output:Path(args.output).write_text(json.dumps(r,indent=2)+'\n')
    print(json.dumps({k:v for k,v in r.items() if k!='records'},indent=2))

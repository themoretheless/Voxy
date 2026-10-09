"""Parse deterministic contact-component snapshots and compare matched queries."""
import argparse,ast,json,re
from pathlib import Path

def split_top(text):
    parts=[];start=0;depth=0;quoted=False;escape=False
    for index,char in enumerate(text):
        if quoted:
            if escape:escape=False
            elif char=='\\':escape=True
            elif char=='"':quoted=False
            continue
        if char=='"':quoted=True
        elif char in '([{':depth+=1
        elif char in ')]}':depth-=1
        elif char==',' and depth==0:
            if text[start:index].strip():parts.append(text[start:index].strip())
            start=index+1
        if depth<0:raise ValueError('unbalanced Rust diagnostic')
    if depth or quoted:raise ValueError('truncated Rust diagnostic')
    if text[start:].strip():parts.append(text[start:].strip())
    return parts

def fields(text):
    start=text.index('{');assert text.rstrip().endswith('}')
    return dict(part.split(':',1) for part in split_top(text[start+1:text.rfind('}')]))

def normalized_fields(text):return {key.strip():value.strip() for key,value in fields(text).items()}

def items(text):
    assert text.startswith('[') and text.endswith(']')
    return split_top(text[1:-1])

def contact(text):
    raw=normalized_fields(text);source=raw['source']
    if source.startswith('Mesh('):source={'mesh':int(source[5:-1])}
    else:source={'strand':{key:int(value) for key,value in normalized_fields(source).items()}}
    return {'source':source,**{name:ast.literal_eval(raw[name]) for name in ['segment','fraction','normal','target','surface_velocity','gap_m']}}

def state(text):
    raw=normalized_fields(text)
    return {**{name:ast.literal_eval(raw[name]) for name in ['phase','substep','iteration','rod','positions','velocities','orientations']},'contacts':[contact(item) for item in items(raw['contacts'])]}

def parse(text):
    raw=normalized_fields(text)
    return {**{name:int(raw[name]) for name in ['substep','structural_iteration','iteration','rod']},
            'before':[state(item) for item in items(raw['before'])],
            'after':[state(item) for item in items(raw['after'])],
            'pairs':[{name:ast.literal_eval(value) for name,value in normalized_fields(item).items()} for item in items(raw['pairs'])]}

def delta(a,b,field):
    return max((abs(x-y) for p,q in zip(a[field],b[field]) for x,y in zip(p,q)),default=0.)

def compare(path):
    records={}
    for line in Path(path).read_text().splitlines(keepends=True):
        if not line.endswith('\n'):continue
        match=re.match(r'HAIR PROJECTION TRACE frame=(\d+) (native|external) (.*)',line)
        if not match:continue
        item=parse(match[3]);key=(int(match[1]),item['substep'],item['structural_iteration'],item['iteration'],item['rod'])
        sides=records.setdefault(key,{})
        if match[2] in sides:raise ValueError('duplicate contact query identity')
        sides[match[2]]=item
    rows=[]
    for key,sides in records.items():
        if set(sides)!={'native','external'}:continue
        native,external=sides['native'],sides['external'];row=dict(zip(['frame','substep','structural_iteration','iteration','rod'],key))
        for phase in ['before','after']:
            a={state['rod']:state for state in native[phase]};b={state['rod']:state for state in external[phase]}
            row[phase+'_component_native']=list(a);row[phase+'_component_external']=list(b)
            row[phase+'_position_delta_all_m']=max((delta(a[index],b[index],'positions') for index in a.keys()&b.keys()),default=0.)
            row[phase+'_selected_position_delta_m']=delta(a[key[4]],b[key[4]],'positions')
        identity=lambda pair:(pair['a'][0],pair['a'][1],pair['b'][0],pair['b'][1])
        row['pair_ids_native']=[identity(pair) for pair in native['pairs']]
        row['pair_ids_external']=[identity(pair) for pair in external['pairs']]
        row['pair_topology_equal']=row['pair_ids_native']==row['pair_ids_external']
        row['selected_delta_increase_m']=row['after_selected_position_delta_m']-row['before_selected_position_delta_m']
        row['native']=native;row['external']=external;rows.append(row)
    overview=lambda row:{key:value for key,value in row.items() if key not in ['native','external']}
    return {'paired_queries':len(rows),'unmatched_queries':sum(set(value)!={'native','external'} for value in records.values()),
            'first_selected_amplification_over_100nm':next((overview(row) for row in rows if row['selected_delta_increase_m']>1e-7),None),
            'largest_selected_amplifications':[overview(row) for row in sorted(rows,key=lambda row:row['selected_delta_increase_m'],reverse=True)[:8]],'records':rows}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('log');parser.add_argument('--output',required=True);args=parser.parse_args();result=compare(args.log)
    Path(args.output).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({key:value for key,value in result.items() if key!='records'},indent=2))

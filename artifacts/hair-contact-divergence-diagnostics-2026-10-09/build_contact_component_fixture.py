"""Freeze the first captured amplification as a portable paired-state replay."""
import argparse,json,math,struct
from pathlib import Path

def freeze(trace_path,rest_path,output):
    trace=json.loads(Path(trace_path).read_text());rest=json.loads(Path(rest_path).read_text())['rest_curves']
    row=next(row for row in trace['records'] if row['selected_delta_increase_m']>1e-7)
    buffer=bytearray(b'VCP1')
    def u(value):buffer.extend(struct.pack('<I',value))
    def f(value):
        if not math.isfinite(value):raise ValueError('nonfinite captured state')
        buffer.extend(struct.pack('<d',value))
    def vectors(values):
        for value in values:
            for coordinate in value:f(coordinate)
    u(2);u(row['rod']);f(1/240);f(40e-6)
    for side in ['native','external']:
        item=row[side];mapping={state['rod']:index for index,state in enumerate(item['before'])}
        after={state['rod']:state for state in item['after']}
        u(len(item['before']));u(len(item['pairs']))
        for state in item['before']:
            index=state['rod'];curve=rest[index];positions=state['positions'];orientations=state['orientations']
            assert len(curve)==len(positions)==len(orientations)+1
            contacts=[contact for contact in state['contacts'] if 'mesh' in contact['source']]
            u(index);u(len(curve));u(len(contacts))
            vectors(curve);vectors(positions);vectors(orientations)
            vectors(after[index]['positions']);vectors(after[index]['orientations'])
            for contact in contacts:
                u(contact['segment']);f(contact['fraction']);vectors([contact[name] for name in ['normal','target','surface_velocity']]);u(contact['source']['mesh'])
        for pair in item['pairs']:
            for name in ['a','b']:
                rod,segment,fraction=pair[name];u(mapping[rod]);u(segment);f(fraction)
            vectors([pair['normal']])
    Path(output).write_bytes(buffer)
    metadata={key:value for key,value in row.items() if key not in ['native','external']}
    metadata['fixture_bytes']=len(buffer);metadata['rest_source']=str(rest_path)
    metadata['limits']='Fixed contact-connected component replay; other independent components and nonlinear surface re-query are excluded.'
    Path(str(output)+'.json').write_text(json.dumps(metadata,indent=2)+'\n')
    return metadata

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('trace');parser.add_argument('rest');parser.add_argument('output');args=parser.parse_args()
    print(json.dumps(freeze(args.trace,args.rest,args.output),indent=2))

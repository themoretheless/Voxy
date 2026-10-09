"""Compare contact provenance before numerical geometry in qualification logs."""
import argparse, json, re
from pathlib import Path

def contacts(text):
    result=[]
    for match in re.finditer(r'HairContactDiagnostic \{',text):
        start=match.end()-1; depth=0; end=start
        for end in range(start,len(text)):
            depth+=(text[end]=='{')-(text[end]=='}')
            if depth==0:break
        item=text[start:end+1]
        source=re.search(r'source: (.*?), segment: (\d+)',item)
        if not source:raise ValueError('missing contact provenance')
        entry={'key':(source[1],int(source[2]))}
        for name in ['fraction','gap_m']:
            entry[name]=float(re.search(name+r': ([^, }]+)',item)[1])
        for name in ['normal','target','surface_velocity']:
            entry[name]=[float(value) for value in re.search(name+r': \[([^]]+)\]',item)[1].split(',')]
        result.append(entry)
    return result

def compare(path):
    frames={}
    for line in Path(path).read_text().splitlines():
        match=re.match(r'HAIR CONTACT TRACE frame=(\d+) (native|external) rod=(\d+) (.*)',line)
        if match:frames.setdefault((int(match[1]),int(match[3])),{})[match[2]]=contacts(match[4])
    rows=[]
    for (frame,rod),sides in sorted(frames.items()):
        if set(sides)!={'native','external'}:continue
        a,b=sides['native'],sides['external']
        keys_a=[x['key'] for x in a];keys_b=[x['key'] for x in b]
        row={'frame':frame,'rod':rod,'native_count':len(a),'external_count':len(b),'same_ordered_provenance':keys_a==keys_b}
        if keys_a==keys_b:
            for name in ['fraction','gap_m','normal','target','surface_velocity']:
                differences=[]
                for x,y in zip(a,b):
                    if isinstance(x[name],list):differences.extend(abs(u-v) for u,v in zip(x[name],y[name]))
                    else:differences.append(abs(x[name]-y[name]))
                row['max_'+name+'_delta']=max(differences,default=0.)
        else:
            row['native_keys']=keys_a;row['external_keys']=keys_b
        rows.append(row)
    return {'paired_frames':len(rows),'last_paired_frame':rows[-1]['frame'] if rows else None,'first_provenance_difference':next((row for row in rows if not row['same_ordered_provenance']),None),'first_target_delta_over_1nm':next((row for row in rows if row.get('max_target_delta',0)>1e-9),None),'frames':rows}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('log');parser.add_argument('--output');args=parser.parse_args()
    result=compare(args.log);data=json.dumps(result,indent=2)+'\n'
    if args.output:Path(args.output).write_text(data)
    print(json.dumps({k:v for k,v in result.items() if k!='frames'},indent=2))

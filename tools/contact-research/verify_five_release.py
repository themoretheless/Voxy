#!/usr/bin/env python3
"""Exact rational release ratio and updated-reaction certificates."""
import json
from pathlib import Path
import sys
from verify_five_qr import value


def verify(path):
    data=json.loads(Path(path).read_text())
    if not data['cases']:raise ValueError('empty release qualification')
    reports=[]
    for case in data['cases']:
        current=list(map(value,case['current']));candidate=list(map(value,case['candidate']))
        if not current or len(current)!=len(candidate):raise ValueError('invalid reactions shape')
        out=case['output'];status=out['status'];valid=False
        if status=='release':
            row=out['row']
            if type(row)!=int or not 0<=row<len(current) or len(out['reactions'])!=len(current):raise ValueError('invalid release output')
            ratios=[(a/(a-b),i) for i,((a,_),(b,_)) in enumerate(zip(current,candidate)) if b<0]
            if not ratios:raise ValueError('release without negative candidate')
            expected,index=min(ratios)
            fraction,error=value(out['fraction'])
            valid=row==index and 0<=expected<=1 and abs(fraction-expected)<=error
            for (a,_),(b,_),result in zip(current,candidate,out['reactions']):
                centre,bound=value(result);exact=a+expected*(b-a)
                valid &= exact>=0 and abs(centre-exact)<=bound
            if 'state' in case or 'next_state' in case:
                state=list(map(value,case['state']));next_state=list(map(value,case['next_state']))
                if len(state)!=len(next_state) or len(out['coordinates'])!=len(state):raise ValueError('invalid coordinate shape')
                for (a,_),(b,_),result in zip(state,next_state,out['coordinates']):
                    centre,bound=value(result);exact=a+expected*(b-a)
                    valid &= abs(centre-exact)<=bound
        elif status=='accept':valid=all(a-e>=0 and b-f>=0 for(a,e),(b,f)in zip(current,candidate))
        elif status=='uncertain':valid=True
        else:raise ValueError('unknown release status')
        reports.append({'name':case['name'],'status':status,'certificate_valid':bool(valid)})
    return {'scope':'Exact point-source ratio and reaction enclosures; uncertain inputs are separately rejected; not full interval-domain or trajectory proof','cases':reports,'status':'passed' if all(r['certificate_valid'] for r in reports) else 'failed'}

if __name__=='__main__':
    report=verify(sys.argv[1])
    with open(sys.argv[2],'x') as stream:json.dump(report,stream,indent=2)
    print(json.dumps(report,indent=2))
    if report['status']!='passed':sys.exit(1)

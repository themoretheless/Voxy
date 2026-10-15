#!/usr/bin/env python3
"""Exact rational check of GPU entering-row certificates from validated gaps."""
import json
from pathlib import Path
import sys
from verify_five_qr import value


def verify(path):
    data=json.loads(Path(path).read_text())
    if not data['cases']:raise ValueError('empty selection qualification')
    reports=[]
    for case in data['cases']:
        gaps=list(map(value,case['gaps']));active=case['active'];decisions=case['decisions'];row=case['row']
        if not gaps or len(gaps)!=len(active) or len(gaps)!=len(decisions) or any(type(a)!=bool for a in active) or any(d not in (1,2,3,4) for d in decisions):raise ValueError('invalid selection shape')
        status=case['status'];valid=False
        if status=='enter':
            if type(row)!=int or row<0 or row>=len(gaps):raise ValueError('invalid selected row')
            best,error=gaps[row]
            valid=not active[row] and decisions[row]==1
            for i,(other,other_error) in enumerate(gaps):
                if active[i] or i==row:continue
                valid &= other-other_error >= best+error
        elif status=='inactive_feasible':valid=row is None and all(a or d in (2,3) for a,d in zip(active,decisions))
        elif status=='uncertain':valid=row is None
        else:raise ValueError('unknown selection status')
        reports.append({'name':case['name'],'status':status,'row':row,'certificate_valid':bool(valid)})
    return {'scope':'Exact interval comparison of GPU selections; supplied gap classifications require their separate original-operator verification','cases':reports,'status':'passed' if all(r['certificate_valid'] for r in reports) else 'failed'}

if __name__=='__main__':
    report=verify(sys.argv[1])
    with open(sys.argv[2],'x') as stream:json.dump(report,stream,indent=2)
    print(json.dumps(report,indent=2))
    if report['status']!='passed':sys.exit(1)

#!/usr/bin/env python3
"""Exact rational enclosure of all GPU inequality residuals."""
from fractions import Fraction as F
import json
from pathlib import Path
import sys
from verify_five_qr import scalar, value


def verify(path):
    data=json.loads(Path(path).read_text())
    n,m=data['rows'],data['width']
    columns=[[sum(map(scalar,parts),F()) for parts in c] for c in data['column_parts_bits']]
    bounds=list(map(scalar,data['bounds_bits']))
    if len(columns)!=n or len(bounds)!=n or any(len(c)!=m for c in columns):
        raise ValueError('invalid original operator')
    reports=[]
    tolerance=scalar(data['tolerance_bits']) if 'tolerance_bits' in data else None
    if tolerance is not None and tolerance<=0:raise ValueError('invalid tolerance')
    if not data['cases']:
        raise ValueError('empty residual qualification')
    for case in data['cases']:
        state=[sum(map(scalar,parts),F()) for parts in case['state_parts_bits']]
        if len(state)!=m or len(case['residuals'])!=n:
            raise ValueError('invalid result shape')
        scale=scalar(case['residual_scale_bits'])
        if scale<=0:
            raise ValueError('invalid residual scale')
        failures=0;worst=F();negative=[];decision_failures=0;uncertain=0
        decisions=case.get('decisions')
        if decisions is not None and (tolerance is None or len(decisions)!=n):raise ValueError('invalid decisions')
        for index,(column,bound,row) in enumerate(zip(columns,bounds,case['residuals'])):
            expected=sum((a*b for a,b in zip(column,state)),F())-bound
            centre,error=value(row);centre*=scale;error*=scale
            defect=abs(centre-expected);failures+=defect>error;worst=max(worst,defect)
            if expected<0:negative.append(index)
            if decisions is not None:
                decision=decisions[index]
                if decision not in (1,2,3,4):raise ValueError('invalid decision code')
                uncertain+=decision==4
                valid=(decision==4 or (decision==1 and centre+error < -tolerance and expected < -tolerance)
                       or (decision==2 and centre-error > tolerance and expected > tolerance)
                       or (decision==3 and centre-error >= -tolerance and centre+error <= tolerance and abs(expected)<=tolerance))
                decision_failures+=not valid
        reports.append({'name':case['name'],'checked':n,'outside_certificate':failures,'worst_absolute_error':float(worst),'exact_negative_gap_rows':negative,'false_certified_decisions':decision_failures,'uncertain_decisions':uncertain})
    return {'scope':'Exact enclosure of every supplied original inequality residual; not active-set or trajectory proof','cases':reports,'status':'passed' if all(r['outside_certificate']==0 and r['false_certified_decisions']==0 for r in reports) else 'failed'}

if __name__=='__main__':
    report=verify(sys.argv[1])
    with open(sys.argv[2],'x') as stream:json.dump(report,stream,indent=2)
    print(json.dumps(report,indent=2))
    if report['status']!='passed':sys.exit(1)

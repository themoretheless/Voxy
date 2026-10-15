#!/usr/bin/env python3
"""Independent exact KKT checks of a complete GPU active-set result."""
from fractions import Fraction as F
import json
from pathlib import Path
import sys
from verify_five_qr import scalar,value


def verify(result_path,operator_path):
    result=json.loads(Path(result_path).read_text());operator=json.loads(Path(operator_path).read_text())
    columns=[[sum(map(scalar,p),F()) for p in c] for c in operator['column_parts_bits']]
    bounds=list(map(scalar,operator['bounds_bits']));state=list(map(value,result['coordinates']));reactions=list(map(value,result['reactions']))
    n,m=result['original_rows'],result['width'];tolerance=scalar(result['original_tolerance_bits'])
    if (len(columns),len(bounds),len(state),len(reactions))!=(n,n,m,n) or any(len(c)!=m for c in columns) or result['original_tolerance_bits']!=operator['tolerance_bits'] or tolerance<=0:raise ValueError('invalid original operator/result shape or tolerance')
    active=set(result['active_rows'])
    if len(active)!=len(result['active_rows']) or any(type(i)!=int or not 0<=i<n for i in active):raise ValueError('invalid active rows')
    negative=gap_failed=equilibrium_failed=0;worst_gap=worst_equilibrium=F()
    for i,(column,bound,(reaction,reaction_error)) in enumerate(zip(columns,bounds,reactions)):
        negative+=reaction-reaction_error<0
        gap=sum((c*x for c,(x,_) in zip(column,state)),F())-bound
        error=sum((abs(c)*e for c,(_,e) in zip(column,state)),F())
        gap_failed+=(abs(gap)+error>tolerance) if i in active else (gap-error < -tolerance)
        worst_gap=max(worst_gap,max(F(),-gap))
        if i not in active and (reaction!=0 or reaction_error!=0):gap_failed+=1
    for j,(coordinate,error) in enumerate(state):
        force=sum((c[j]*v for c,(v,_) in zip(columns,reactions)),F())
        bound=error+sum((abs(c[j])*e for c,(_,e) in zip(columns,reactions)),F())
        defect=abs(force-coordinate);equilibrium_failed+=defect>bound;worst_equilibrium=max(worst_equilibrium,defect)
    return {'scope':'Exact whitened KKT interval certificates for all original constraints; original physical admission is separately required from the native owner; not a trajectory proof','rows':n,'width':m,'negative_reaction_intervals':negative,'uncertified_original_gap_rows':gap_failed,'equilibrium_outside_certificate':equilibrium_failed,'worst_nominal_gap_violation':float(worst_gap),'worst_nominal_equilibrium_defect':float(worst_equilibrium),'status':'passed' if negative==gap_failed==equilibrium_failed==0 else 'failed'}

if __name__=='__main__':
    report=verify(sys.argv[1],sys.argv[2])
    with open(sys.argv[3],'x') as stream:json.dump(report,stream,indent=2)
    print(json.dumps(report,indent=2))
    if report['status']!='passed':sys.exit(1)

"""Audit authored guide capsules; preserve follicle-corner permissions only.
Float geometry locates candidates; each penetration is independently checked
with the 80-digit convex closest-point reference. Does not repair the groom.
"""
import argparse,json,math
from pathlib import Path
from audit_segment_pairs import runtime_formula,reference

def trimmed(p,q):return [p[i]+.2*(q[i]-p[i]) for i in range(3)]
def audit(path):
    curves=json.loads(Path(path).read_text())['rest_curves']; radius=40e-6
    segments=[]
    for r,curve in enumerate(curves):
        for i in range(len(curve)-1):
            a,b=curve[i:i+2]
            lo=[min(a[k],b[k])-radius-1e-10 for k in range(3)]
            hi=[max(a[k],b[k])+radius+1e-10 for k in range(3)]
            segments.append(((r,i),a,b,lo,hi))
    axis=max(range(3),key=lambda k:max(s[4][k] for s in segments)-min(s[3][k] for s in segments))
    order=sorted(range(len(segments)),key=lambda i:(segments[i][3][axis],i))
    rows=[];count=0
    for slot,ai in enumerate(order):
        a=segments[ai]
        for bi in order[slot+1:]:
            b=segments[bi]
            if b[3][axis]>a[4][axis]:break
            if a[0][0]==b[0][0] and abs(a[0][1]-b[0][1])<=2:continue
            if any(a[3][k]>b[4][k] or b[3][k]>a[4][k] for k in range(3)):continue
            count+=1
            points=[a[1],a[2],b[1],b[2]]
            candidates=[('full',points)]
            if a[0][1]==b[0][1]==0:
                candidates=[('trim_a',[trimmed(a[1],a[2]),a[2],b[1],b[2]]),('trim_b',[a[1],a[2],trimmed(b[1],b[2]),b[2]])]
            # Root/root corner alone is permitted; all remaining parameter
            # pairs belong to at least one of these two strips.
            for region,points in candidates:
                actual=runtime_formula(points)
                if actual['distance_m']>=2*radius-1e-10:continue
                exact=reference(points)
                if exact['distance_m']>=2*radius-1e-10:continue
                rows.append({'a':a[0],'b':b[0],'region':region,'points':points,'distance_m':exact['distance_m'],'gap_m':exact['distance_m']-2*radius,'reference':exact,'runtime_distance_error_m':abs(exact['distance_m']-actual['distance_m'])})
    rows.sort(key=lambda row:(row['gap_m'],row['a'],row['b'],row['region']))
    return {'scope':'Authored rest geometry only, before any time integration; not full animation or FPS qualification','guides':len(curves),'segments':len(segments),'candidate_pairs':count,'penetrating_regions':len(rows),'penetrating_pairs':len({tuple(sorted((tuple(row['a']),tuple(row['b'])))) for row in rows}),'minimum_gap_m':min((row['gap_m'] for row in rows),default=0),'max_distance_error_m':max((row['runtime_distance_error_m'] for row in rows),default=0),'rows':rows}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('rest');p.add_argument('output');args=p.parse_args()
    result=audit(args.rest);Path(args.output).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k!='rows'},indent=2))

"""Sample a captured joint-root candidate; sampling is not a CCD certificate."""
import argparse,json,struct,math
from pathlib import Path
from audit_segment_pairs import runtime_formula,reference
from audit_groom_initial_contacts import trimmed

def read(path):
    data=Path(path).read_bytes();offset=0
    def take(fmt):
        nonlocal offset
        size=struct.calcsize(fmt);value=struct.unpack_from(fmt,data,offset);offset+=size
        return value[0] if len(value)==1 else value
    assert take('<4s')==b'VJR1';count=take('<I');radius=take('<d');rows=[]
    def points(n):return [list(take('<3d')) for _ in range(n)]
    for _ in range(count):
        n=take('<I');rows.append({'start':points(n),'staged':points(n),'linear':points(n),'angular':points(n-1)})
    pairs=[take('<2I') for _ in range(take('<I'))];assert offset==len(data)
    return rows,pairs,radius

def audit(path):
    rods,contacts,radius=read(path);parent=list(range(len(rods)))
    def root(r):
        while parent[r]!=r:r=parent[r]
        return r
    for a,b in contacts:
        a,b=root(a),root(b);parent[max(a,b)]=min(a,b)
    scales=[min(1.,.35/max((math.sqrt(sum(v*v for v in p)) for p in r['angular']),default=0.)) if any(any(p) for p in r['angular']) else 1. for r in rods]
    minima={}
    for r,scale in enumerate(scales):minima[root(r)]=min(minima.get(root(r),1.),scale)
    scales=[minima[root(r)] for r in range(len(rods))]
    segments=[]
    for r,rod in enumerate(rods):
        end=[[x+delta*scales[r] for x,delta in zip(p,d)] for p,d in zip(rod['staged'],rod['linear'])]
        for i in range(len(end)-1):
            start=rod['start'][i:i+2];final=end[i:i+2];allpoints=start+final
            lo=[min(p[k] for p in allpoints)-radius-1e-10 for k in range(3)]
            hi=[max(p[k] for p in allpoints)+radius+1e-10 for k in range(3)]
            segments.append(((r,i),start,final,lo,hi))
    axis=max(range(3),key=lambda k:max(s[4][k] for s in segments)-min(s[3][k] for s in segments))
    order=sorted(range(len(segments)),key=lambda i:(segments[i][3][axis],i));rows=[];candidates=0
    for slot,ai in enumerate(order):
        a=segments[ai]
        for bi in order[slot+1:]:
            b=segments[bi]
            if b[3][axis]>a[4][axis]:break
            if a[0][0]==b[0][0] and abs(a[0][1]-b[0][1])<=2:continue
            if any(a[3][k]>b[4][k] or b[3][k]>a[4][k] for k in range(3)):continue
            candidates+=1;worst=None
            for sample in range(33):
                t=sample/32
                pa=[[x+(y-x)*t for x,y in zip(p,q)] for p,q in zip(a[1],a[2])]
                pb=[[x+(y-x)*t for x,y in zip(p,q)] for p,q in zip(b[1],b[2])]
                regions=[('full',pa+pb)]
                if a[0][1]==b[0][1]==0:regions=[('trim_a',[trimmed(*pa),pa[1]]+pb),('trim_b',pa+[trimmed(*pb),pb[1]])]
                for region,points in regions:
                    distance=runtime_formula(points)['distance_m']
                    if worst is None or distance<worst[0]:worst=(distance,t,region,points)
            distance,t,region,points=worst
            if distance<2*radius-1e-10:
                exact=reference(points)
                rows.append({'a':a[0],'b':b[0],'sample_time':t,'region':region,'gap_m':exact['distance_m']-2*radius,'points':points,'reference':exact})
    rows.sort(key=lambda r:r['gap_m'])
    return {'scope':'33 sampled times on the initial trust-scaled candidate. Negative gaps are counterexamples; no negative sample does not prove separation. Later component reductions not sampled.','guides':len(rods),'radius_m':radius,'candidate_pairs':candidates,'sampled_penetrating_pairs':len(rows),'minimum_sampled_gap_m':min((r['gap_m'] for r in rows),default=0),'minimum_initial_trust_scale':min(scales),'rows':rows}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('fixture');p.add_argument('output');args=p.parse_args();result=audit(args.fixture)
    Path(args.output).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='rows'},indent=2))

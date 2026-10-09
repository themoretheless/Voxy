"""Audit captured hair segment pairs against an 80-digit closest-point solve."""
import argparse, ast, json, math, re
from decimal import Decimal, localcontext
from pathlib import Path

def dot(a,b):return sum(x*y for x,y in zip(a,b))
def sub(a,b):return [x-y for x,y in zip(a,b)]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def clamp(x):return max(0,min(1,x))
def position(a,u,s):return [x+s*y for x,y in zip(a,u)]
def reference(points):
    with localcontext() as ctx:
        ctx.prec=80
        a,b,c,d=[[Decimal.from_float(x) for x in point] for point in points]
        u,v,w=sub(b,a),sub(d,c),sub(a,c)
        aa,bb,cc,dd,ee=dot(u,u),dot(u,v),dot(v,v),dot(u,w),dot(v,w)
        candidates=[]
        for s in [Decimal(0),Decimal(1)]:candidates.append((s,clamp((ee+bb*s)/cc) if cc else Decimal(0)))
        for t in [Decimal(0),Decimal(1)]:candidates.append((clamp((bb*t-dd)/aa) if aa else Decimal(0),t))
        det=aa*cc-bb*bb
        if det>0:
            s,t=(bb*ee-cc*dd)/det,(aa*ee-bb*dd)/det
            if 0<=s<=1 and 0<=t<=1:candidates.append((s,t))
        scored=[]
        for s,t in candidates:
            p,q=position(a,u,s),position(c,v,t);delta=sub(p,q)
            scored.append((dot(delta,delta),s,t,p,q))
        distance,s,t,p,q=min(scored,key=lambda item:item[:3])
        return {'s':float(s),'t':float(t),'distance_m':float(distance.sqrt()),'p':[float(x) for x in p],'q':[float(x) for x in q]}

def runtime_formula(points):
    a,b,c,d=points;u,v,w=sub(b,a),sub(d,c),sub(a,c)
    aa,bb,cc,dd,ee=dot(u,u),dot(u,v),dot(v,v),dot(u,w),dot(v,w)
    if aa==0:s,t=0,clamp(ee/cc) if cc else 0
    elif cc==0:s,t=clamp(-dd/aa),0
    else:
        uv=cross(u,v);den=dot(uv,uv)
        s=clamp(dot(cross(v,w),uv)/den) if den>0 else 0
        t=(bb*s+ee)/cc
        if t<0:t,s=0,clamp(-dd/aa)
        elif t>1:t,s=1,clamp((bb-dd)/aa)
    p,q=position(a,u,s),position(c,v,t);delta=sub(p,q)
    return {'s':s,'t':t,'distance_m':math.sqrt(dot(delta,delta)),'p':p,'q':q}

def audit(path):
    rows=[];sides={}
    pattern=r'HAIR SEGMENT PAIR frame=(\d+) (native|external) rod=(\d+) segment=(\d+) other_rod=(\d+) other_segment=(\d+) (.*)'
    for line in Path(path).read_text().splitlines():
        match=re.match(pattern,line)
        if not match:continue
        key=tuple(int(match[i]) for i in [1,3,4,5,6]);side=match[2]
        points=[ast.literal_eval(re.search(name+r'=(\[[^]]+\])',match[7])[1]) for name in 'abcd']
        exact,actual=reference(points),runtime_formula(points)
        u,v=sub(points[1],points[0]),sub(points[3],points[2]);uv=cross(u,v)
        row={'frame':key[0],'side':side,'rod':key[1],'segment':key[2],'other_rod':key[3],'other_segment':key[4],'points':points,'reference':exact,'runtime_formula':actual,'sin_squared_angle':dot(uv,uv)/(dot(u,u)*dot(v,v)) if dot(u,u)*dot(v,v) else 0,'distance_error_m':abs(exact['distance_m']-actual['distance_m']),'closest_point_error_m':max(abs(x-y) for name in ['p','q'] for x,y in zip(exact[name],actual[name]))}
        rows.append(row);sides.setdefault(key,{})[side]=row
    paired=[]
    for key,pair in sorted(sides.items()):
        if set(pair)!={'native','external'}:continue
        a,b=pair['native'],pair['external']
        paired.append({'frame':key[0],'rod':key[1],'segment':key[2],'other_rod':key[3],'other_segment':key[4],'endpoint_delta_m':max(abs(x-y) for p,q in zip(a['points'],b['points']) for x,y in zip(p,q)),'reference_closest_point_delta_m':max(abs(x-y) for name in ['p','q'] for x,y in zip(a['reference'][name],b['reference'][name])),'reference_s_delta':abs(a['reference']['s']-b['reference']['s']),'reference_t_delta':abs(a['reference']['t']-b['reference']['t'])})
    return {'captured_pairs':len(rows),'paired_geometry_count':len(paired),'max_distance_error_m':max((x['distance_error_m'] for x in rows),default=0),'max_closest_point_error_m':max((x['closest_point_error_m'] for x in rows),default=0),'rows':rows,'paired_geometry':paired}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('log',nargs='?');parser.add_argument('--output');parser.add_argument('--self-test',action='store_true');args=parser.parse_args()
    if args.self_test:
        for points,expected in [([[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[1.,1.,0.]],1),([[0.,0.,0.],[1.,0.,0.],[.5,-1.,0.],[.5,1.,0.]],0),([[0.,0.,0.],[0.,0.,0.],[1.,0.,0.],[2.,0.,0.]],1)]:assert reference(points)['distance_m']==expected
        print('80-digit reference: parallel, crossing and degenerate analytic cases passed')
    if args.log:
        result=audit(args.log)
        if args.output:Path(args.output).write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps({k:v for k,v in result.items() if k not in ['rows','paired_geometry']},indent=2))

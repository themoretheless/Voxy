"""Pointwise counterexample search; local minima never certify full clearance."""
import ast,json,math,re,sys
from pathlib import Path
from scipy.optimize import minimize_scalar
from audit_joint_root_motion import read
from audit_segment_pairs import reference,runtime_formula
rods,pairs,radius=read(sys.argv[2]);parent=list(range(len(rods)))
def root(i):
    while parent[i]!=i:i=parent[i]
    return i
for a,b in pairs:
    a,b=root(a),root(b);parent[max(a,b)]=min(a,b)
scales=[min(1.,.35/max((math.sqrt(sum(x*x for x in p)) for p in rod['angular']),default=0.)) if any(any(p) for p in rod['angular']) else 1. for rod in rods]
minimum={}
for i,s in enumerate(scales):minimum[root(i)]=min(minimum.get(root(i),1.),s)
scales=[minimum[root(i)] for i in range(len(rods))]
rows=[];seen=set()
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'HAIR SWEPT LIMIT ' not in line:continue
    key=line.split(' fraction=')[0]
    if key in seen:break
    seen.add(key)
    ids=[tuple(map(int,x)) for x in re.findall(r'[ab]=\((\d+),(\d+)\)',key)]
    a0,b0,a1,b1=[ast.literal_eval(x) for x in re.search(r'start_a=(.*) start_b=(.*) end_a=(.*) end_b=(.*)',line).groups()]
    errors=[]
    for (r,s),end in zip(ids,[a1,b1]):
        errors.extend(abs(end[p][k]-(rods[r]['staged'][s+p][k]+scales[r]*rods[r]['linear'][s+p][k])) for p in range(2) for k in range(3))
    def points(t):return [[x+t*(y-x) for x,y in zip(p,q)] for start,end in [(a0,a1),(b0,b1)] for p,q in zip(start,end)]
    def distance(t):return runtime_formula(points(t))['distance_m']
    times=[i/32 for i in range(33)]+[10.**(-i) for i in range(1,14)]
    for lo,hi in zip(times[:32],times[1:33]):times.append(float(minimize_scalar(distance,bounds=(lo,hi),method='bounded',options={'xatol':1e-15}).x))
    time=min(times,key=distance);exact=reference(points(time))
    rows.append({'identity':key,'nominal_initial_trust_candidate':max(errors)<1e-15,'nominal_endpoint_error_m':max(errors),'local_search_time':time,'reference_gap_m':exact['distance_m']-2*radius,'reference':exact})
result={'scope':'localized pointwise searches checked with an 80-digit reference; no universal clearance certificate; both-root trimmed paths need separate classification','rows':rows}
Path(sys.argv[3]).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))

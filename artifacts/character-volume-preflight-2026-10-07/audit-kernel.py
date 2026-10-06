"""Find and exactly verify a four-face separating certificate on native coordinates.
No geometry is changed. Floating search selects candidates; Fraction checks the proof.
"""
from pathlib import Path
from fractions import Fraction
import json, math, random, hashlib
base = Path(__file__).parent
source = base / 'source-surface.json'
surface = json.loads(source.read_text())
points, faces = surface['points'], surface['boundary']
def sub(a,b): return [x-y for x,y in zip(a,b)]
def cross(a,b): return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def solve(normals, one):
    rows = [[normals[c][r] for c in range(4)] + [one*0] for r in range(3)]
    rows.append([one]*5)
    for col in range(4):
        pivot = max(range(col,4), key=lambda i: abs(rows[i][col]))
        if rows[pivot][col] == 0: return None
        rows[col],rows[pivot] = rows[pivot],rows[col]
        d = rows[col][col]
        rows[col] = [x/d for x in rows[col]]
        for i in range(4):
            if i != col:
                factor = rows[i][col]
                rows[i] = [x-factor*y for x,y in zip(rows[i],rows[col])]
    return [row[4] for row in rows]
planes=[]
for face in faces:
    a,b,c = [points[i] for i in face]
    n = cross(sub(b,a),sub(c,a)); size=math.sqrt(dot(n,n))
    if size == 0: raise ValueError('degenerate native triangle')
    n = [x/size for x in n]; planes.append((n,dot(n,a)))
rng=random.Random(20261007)
result=None
for attempt in range(30000):
    indices=rng.sample(range(len(faces)),4)
    weights=solve([planes[i][0] for i in indices],1.)
    if weights is None or min(weights) <= 0 or sum(w*planes[i][1] for w,i in zip(weights,indices)) >= -1e-8: continue
    exact=[]
    for i in indices:
        a,b,c = [[Fraction(x) for x in points[v]] for v in faces[i]]
        n = cross(sub(b,a),sub(c,a)); exact.append((n,dot(n,a)))
    weights=solve([plane[0] for plane in exact],Fraction(1))
    if weights is None or min(weights) < 0: continue
    bound=sum(w*p[1] for w,p in zip(weights,exact))
    normal=[sum(w*p[0][axis] for w,p in zip(weights,exact)) for axis in range(3)]
    if normal != [0,0,0] or bound >= 0: continue
    def rational(v): return {'numerator':str(v.numerator),'denominator':str(v.denominator)}
    result={'status':'verified infeasible surface kernel','source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(), 'source_triangle_indices':indices, 'source_vertex_indices':[faces[i] for i in indices], 'nonnegative_weights':[rational(w) for w in weights], 'weighted_plane_normal_exact':[rational(v) for v in normal], 'weighted_plane_offset_exact':rational(bound), 'search_candidates_examined':attempt+1, 'proof':'Interior must satisfy n_i dot x <= b_i for every outward face. These nonnegative weights yield sum(weight*n)=0 and sum(weight*b)<0, hence the selected four halfspaces have no common point.', 'scope':'Exact rational arithmetic on the binary f64 coordinates exported by native phase-zero preview; not a continuous animation or anatomical certificate.'}
    break
if result is None:
    result={'status':'no certificate found in bounded search','scope':'Search failure does not establish a feasible kernel.'}
(base/'kernel-certificate.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['nonnegative_weights','weighted_plane_offset_exact']},indent=2))

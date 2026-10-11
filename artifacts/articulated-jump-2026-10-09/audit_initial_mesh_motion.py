"""Independent sampled/optimized diagnostic; not a continuous certificate."""
import json
import sys
import numpy as np
from scipy.optimize import minimize_scalar
from pathlib import Path
source = json.loads(Path(sys.argv[1]).read_text())
a, b = np.array(source['capsule_start']), np.array(source['capsule_end'])
u, v = np.array(source['triangle_start']), np.array(source['triangle_end'])
radius = source['radius']
def point_distance(p, vertices):
    candidates = []
    for i in range(3):
        q, end = vertices[i], vertices[(i+1)%3]
        direction = end-q
        squared = np.dot(direction,direction)
        t = np.clip(np.dot(p-q,direction)/squared,0.,1.) if squared else 0.
        candidates.append(q+t*direction)
    n = np.cross(vertices[1]-vertices[0],vertices[2]-vertices[0])
    squared = np.dot(n,n)
    if squared:
        projection = p-n*np.dot(p-vertices[0],n)/squared
        if all(np.dot(np.cross(vertices[(i+1)%3]-vertices[i],projection-vertices[i]),n)>=0. for i in range(3)):
            candidates.append(projection)
    return min(float(np.linalg.norm(p-q)) for q in candidates)
def gap(time):
    capsule = (1.-time)*a+time*b
    triangle = (1.-time)*u+time*v
    distance = lambda s:point_distance((1.-s)*capsule[0]+s*capsule[1],triangle)
    result = minimize_scalar(distance,bounds=(0.,1.),method='bounded',options={'xatol':1e-14})
    return min(distance(0.),distance(1.),result.fun)-radius
samples = [(i/1000.,gap(i/1000.)) for i in range(1001)]
best = min(samples,key=lambda row:row[1])
print(json.dumps({'diagnostic_only':True,'rod':source['rod'],'segment':source['segment'],'gap_start_m':gap(0.),'gap_end_m':gap(1.),'gap_midpoint_m':gap(.5),'sampled_minimum_time':best[0],'sampled_minimum_gap_m':best[1]},indent=2))

"""Exact rational transverse intersections near diagnosed source vertices."""
from fractions import Fraction as Q
import json,pathlib,sys
root=pathlib.Path('artifacts/character-volume-filtering-2026-10-07')
lines=pathlib.Path('artifacts/general-tissue-mesher-2026-10-07/character-input.obj').read_text().splitlines()
p=[[float(x) for x in l.split()[1:]] for l in lines if l.startswith('v ')]
f=[[int(x)-1 for x in l.split()[1:]] for l in lines if l.startswith('f ')]
if len(sys.argv)>1:
    source=json.loads(pathlib.Path(sys.argv[1]).read_text())
    p=source['points']; f=source['boundary']
pq=[[Q(x) for x in point] for point in p]
def sub(a,b):return [x-y for x,y in zip(a,b)]
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
locations=json.loads((root/'original-boundary-audit.json').read_text())['bad_boundary_vertices']
near={min(range(len(p)),key=lambda i:sum((x-y)**2 for x,y in zip(p[i],v['position_m']))) for v in locations}
local=[i for i,t in enumerate(f) if near.intersection(t)]
if len(sys.argv)>1:
    local=json.loads((root/'local-intersections.json').read_text())['local_faces']
boxes=[[(min(p[i][k] for i in t),max(p[i][k] for i in t)) for k in range(3)] for t in f]
seen=set();hits=[]; tested=0
for i in local:
    for j in range(len(f)):
        if i==j or tuple(sorted((i,j))) in seen:continue
        seen.add(tuple(sorted((i,j))))
        if any(a[1]<b[0] or b[1]<a[0] for a,b in zip(boxes[i],boxes[j])):continue
        tested+=1
        for edge_face,tri_face in ((i,j),(j,i)):
            a,b,c=[pq[v] for v in f[tri_face]]; e1=sub(b,a);e2=sub(c,a)
            for u,v in zip(f[edge_face],f[edge_face][1:]+f[edge_face][:1]):
                start=pq[u];d=sub(pq[v],start); h=cross(d,e2);den=dot(e1,h)
                if den==0:continue
                s=sub(start,a); w1=dot(s,h)/den;q=cross(s,e1);w2=dot(d,q)/den;t=dot(e2,q)/den
                if 0<t<1 and w1>0 and w2>0 and w1+w2<1:
                    hits.append({'edge_face':edge_face,'triangle_face':tri_face,'edge':[u,v],'t':str(t),'weights':[str(1-w1-w2),str(w1),str(w2)]})
(pathlib.Path(sys.argv[2]) if len(sys.argv)>2 else root/'local-intersections.json').write_text(json.dumps({'arithmetic':'exact rationals of binary-f64 OBJ coordinates','local_vertices':sorted(near),'local_faces':local,'bbox_candidate_pairs':tested,'strict_transverse_intersections':hits,'scope':'local noncoplanar test; absence does not prove global nonintersection'},indent=2)+'\n')
print({'local_faces':len(local),'candidate_pairs':tested,'strict_intersections':len(hits)})

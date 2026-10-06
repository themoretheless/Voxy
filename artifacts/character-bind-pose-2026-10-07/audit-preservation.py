"""Compare generated coordinates/facets and volume with the native reference skin."""
import json,pathlib
p=pathlib.Path('artifacts/character-bind-pose-2026-10-07')
s=json.loads((p/'bind-pose-surface.json').read_text());mapping=json.loads((p/'result.json').read_text())['source_to_geometry']
def rows(file):return [l.split('#')[0].split() for l in file.read_text().splitlines() if l.split('#')[0].strip()]
n=rows(p/'bind-pose.1.node'); e=rows(p/'bind-pose.1.ele');nodes={int(r[0]):[float(x) for x in r[1:4]] for r in n[1:]};tets=[[int(x) for x in r[1:5]] for r in e[1:]]
faces={}
for a,b,c,d in tets:
 for f in ((b,c,d),(a,d,c),(a,b,d),(a,c,b)):
  k=tuple(sorted(f));faces[k]=faces.get(k,0)+1
boundary={f for f,c in faces.items() if c==1};sourcefaces={tuple(sorted(mapping[i] for i in f)) for f in s['boundary']}
def sub(a,b):return [x-y for x,y in zip(a,b)]
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
volume=sum(abs(dot(sub(nodes[b],nodes[a]),cross(sub(nodes[c],nodes[a]),sub(nodes[d],nodes[a]))))/6 for a,b,c,d in tets)
source_volume=sum(dot(s['points'][a],cross(s['points'][b],s['points'][c]))/6 for a,b,c in s['boundary'])
r={'all_source_points_retained_exactly':all(nodes[mapping[i]]==point for i,point in enumerate(s['points'])),'maximum_source_coordinate_error_m':max(abs(nodes[mapping[i]][k]-point[k]) for i,point in enumerate(s['points']) for k in range(3)),'all_source_facets_retained':boundary==sourcefaces,'boundary_facets':len(boundary),'source_facets':len(sourcefaces),'tetrahedral_volume_m3':volume,'source_signed_surface_volume_m3':source_volume,'volume_difference_m3':abs(volume-source_volume),'scope':'independent rest geometry preservation and ordinary floating volume sum; no anatomical calibration or dynamics'}
assert r['all_source_points_retained_exactly'];assert r['all_source_facets_retained'];assert r['volume_difference_m3']<1e-14
(p/'preservation-audit.json').write_text(json.dumps(r,indent=2)+'\n');print(r)

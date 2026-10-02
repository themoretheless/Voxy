from pathlib import Path
import csv,math,json
from mathutils import Vector
ROOT=Path('/Users/themoretheless/Documents/ChatGPT/Voxy');v=[];faces=[]
for line in (ROOT/'assets/characters/blender-female/body.obj').read_text().splitlines():
 if line.startswith('v '):v.append(Vector(tuple(map(float,line.split()[1:4]))))
 elif line.startswith('f '):
  ids=[int(x.split('/')[0])-1 for x in line.split()[1:]]
  for j in range(1,len(ids)-1):faces.append((ids[0],ids[j],ids[j+1]))
def key(p):return tuple(Vector(tuple(float(x)for x in p)))
def angle(p):
 a=(p[1]-p[0]).cross(p[2]-p[0]).normalized();b=(p[0]-p[1]).cross(p[3]-p[1]).normalized();return math.acos(max(-1,min(1,a.dot(b))))
adj={};hinges=[]
for face in faces:
 for j in range(3):
  a,b,c=face[j],face[(j+1)%3],face[(j+2)%3];edge=tuple(sorted((a,b)))
  if edge in adj:
   ids=[*edge,c,adj[edge]];p=[v[i]for i in ids];center=sum(p,Vector())/4
   if .33<center.x<.37 and -.025<center.y<.025 and .070<center.z<.115:hinges.append((ids,angle(p)))
  adj[edge]=c
selected=[f for f in faces if all(v[i].x>.32 and -.16<v[i].y<.03 for i in f)]
for folder in ['hand-local-global-diagnostic']:
 out=ROOT/'target'/folder;results=[]
 for path in sorted(out.glob('finger-0-*.csv')):
  positions={key([float(r['rest_'+a])for a in 'xyz']):Vector(tuple(float(r['posed_'+a])for a in 'xyz'))for r in csv.DictReader(path.open())}
  posed=[positions.get(key(p),p)for p in v];extra=0.;min_area=1e9;worst_face=None;worst_hinge=None;rest_error=max((posed[i]-v[i]).length for i in range(len(v))) if float(path.stem.rsplit('-',1)[1])==0 else None
  for ids,rest_angle in hinges:
   value=angle([posed[i]for i in ids])-rest_angle
   if value>extra:extra=value;worst_hinge=[list(v[i])for i in ids]
  for ids in selected:
   a,b,c=[v[i]for i in ids];area=(b-a).cross(c-a).length
   if area>1e-10:
    a,b,c=[posed[i]for i in ids];value=(b-a).cross(c-a).length/area
    if value<min_area:min_area=value;worst_face=[list(v[i])for i in ids]
  results.append({'file':path.name,'max_extra_web_crease_rad':extra,'min_hand_triangle_area_ratio':min_area,'rest_error_m':rest_error,'worst_face':worst_face,'worst_hinge':worst_hinge})
 (out/'measurement.json').write_text(json.dumps({'left_hand_only':True,'samples':5,'results':results},indent=2));print(folder,json.dumps(results))

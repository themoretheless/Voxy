import pathlib,csv,json,hashlib
from mathutils import Vector
from mathutils.bvhtree import BVHTree
ROOT=pathlib.Path('/Users/themoretheless/Documents/ChatGPT/Voxy');OUT=ROOT/'target/hand-metacarpal-engine-audit'
SOURCE=ROOT/'assets/characters/blender-female/body.obj';TARGET=ROOT/'assets/characters/blender-female/prepared/body-forehead-refined.obj'
def read_obj(path):
 vertices=[];faces=[]
 for line in path.read_text().splitlines():
  if line.startswith('v '):vertices.append(Vector(tuple(map(float,line.split()[1:4]))))
  elif line.startswith('f '):
   ids=[int(p.split('/')[0])-1 for p in line.split()[1:]]
   for i in range(1,len(ids)-1):faces.append((ids[0],ids[i],ids[i+1]))
 return vertices,faces
def key(p):return tuple(round(float(x)*1000000)for x in p)
source,faces=read_obj(SOURCE);prepared,_=read_obj(TARGET)
bind={};rows=[]
for line in (OUT/'weights.csv').read_text().splitlines():
 r=list(map(float,line.split(',')));bind[key(r[:3])]=r[3:];rows.append(r)
identity=[1.]+[0.]*53
bvhs={}
for sign in [1,-1]:
 selected=[f for f in faces if all(source[i].x*sign>.315 and -.165<source[i].y<.07 for i in f)]
 bvhs[sign]=(BVHTree.FromPolygons(source,selected,all_triangles=True),selected)
result=[];max_dist=0.;max_sum_error=0.;interpolated=0;exact=0
for p in prepared:
 if not(abs(p.x)>.32 and -.16<p.y<.03):continue
 if key(p)in bind:w=bind[key(p)];exact+=1
 else:
  bvh,selected=bvhs[1 if p.x>0 else -1];q,n,face,dist=bvh.find_nearest(p);max_dist=max(max_dist,dist)
  if dist>.001:raise RuntimeError('Runtime hand differs by >1 mm: '+str(dist))
  ids=selected[face];a,b,c=[source[i]for i in ids];ab=b-a;ac=c-a;aq=q-a;aa=ab.dot(ab);bb=ac.dot(ac);cross=ab.dot(ac);den=aa*bb-cross*cross
  if den<=1e-25:raise RuntimeError('Degenerate binding triangle')
  v=(bb*aq.dot(ab)-cross*aq.dot(ac))/den;t=(aa*aq.dot(ac)-cross*aq.dot(ab))/den;bc=[1-v-t,v,t]
  bc=[max(0,min(1,x))for x in bc];total=sum(bc);bc=[x/total for x in bc]
  donors=[bind.get(key(source[i]),identity)for i in ids]
  w=[sum(bc[j]*donors[j][i]for j in range(3))for i in range(54)];interpolated+=1
 max_sum_error=max(max_sum_error,abs(sum(w)-1));assert min(w)>=-1e-8;result.append(list(p)+w)
(OUT/'runtime-weights.csv').write_text('\n'.join(','.join(map(str,r))for r in result))
audit={'source_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'runtime_mesh':str(TARGET),'runtime_sha256':hashlib.sha256(TARGET.read_bytes()).hexdigest(),'runtime_hand_vertices':len(result),'exact_source_matches':exact,'surface_interpolated':interpolated,'max_transfer_distance_m':max_dist,'max_weight_sum_error':max_sum_error,'method':'Barycentric interpolation on nearest same-side original hand surface; convex normalized weights; >1 mm rejects transfer'}
(OUT/'runtime-transfer.json').write_text(json.dumps(audit,indent=2));print(json.dumps(audit))

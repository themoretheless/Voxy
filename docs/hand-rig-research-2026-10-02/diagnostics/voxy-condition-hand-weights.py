from pathlib import Path
import math,struct,json
ROOT=Path('/Users/themoretheless/Documents/ChatGPT/Voxy');OUT=ROOT/'target/hand-metacarpal-engine-audit'
def f32(x):return struct.unpack('f',struct.pack('f',x))[0]
def key(p):return tuple(round(float(x)*1000000)for x in p)
vertices=[];faces=[]
for l in (ROOT/'assets/characters/blender-female/body.obj').read_text().splitlines():
 if l.startswith('v '):vertices.append(tuple(f32(float(x))for x in l.split()[1:4]))
 elif l.startswith('f '):
  ids=[int(x.split('/')[0])-1 for x in l.split()[1:]]
  for j in range(1,len(ids)-1):faces.append((ids[0],ids[j],ids[j+1]))
rows=[list(map(float,l.split(',')))for l in (OUT/'weights.csv').read_text().splitlines()];weights={key(r[:3]):r[3:]for r in rows}
constraints=[]
for face in faces:
 if any(key(vertices[i])not in weights for i in face):continue
 pairs=[(face[0],face[1],face[2]),(face[1],face[2],face[0]),(face[2],face[0],face[1])]
 a,b,c=max(pairs,key=lambda p:sum((vertices[p[0]][k]-vertices[p[1]][k])**2 for k in range(3)))
 ab=[vertices[b][k]-vertices[a][k]for k in range(3)];ac=[vertices[c][k]-vertices[a][k]for k in range(3)];length2=sum(x*x for x in ab);t=sum(ab[k]*ac[k]for k in range(3))/length2;height=math.sqrt(sum((ac[k]-t*ab[k])**2 for k in range(3)))
 if 0.005<t<.995 and height<.0001 and math.sqrt(length2)/max(height,1e-12)>50:
  constraints.append((key(vertices[a]),key(vertices[b]),key(vertices[c]),t))
for iteration in range(3):
 requests={}
 for a,b,c,t in constraints:
  proposed=[(1-t)*weights[a][k]+t*weights[b][k]for k in range(54)]
  requests.setdefault(c,[]).append(proposed)
 for c,values in requests.items():
  weights[c]=[sum(v[k]for v in values)/len(values)for k in range(54)]
output=[]
for r in rows:output.append(','.join(map(str,r[:3]+weights[key(r[:3])])) )
(OUT/'weights-conditioned.csv').write_text('\n'.join(output));audit={'method':'Interpolate binding along long edge of highly slender source triangles, average conflicting proposals; geometry and pose unchanged','height_max_m':.0001,'aspect_min':50,'iterations':3,'constraints':len(constraints),'changed_vertices':len(set(c for a,b,c,t in constraints))}
(OUT/'conditioning.json').write_text(json.dumps(audit,indent=2));print(json.dumps(audit))

"""Independent 400-digit matrix/Rodrigues prefix and point oracle."""
from decimal import Decimal as D, localcontext
from pathlib import Path
import ast,json,sys

def sincos(x):
    sine,cosine=x,D(1);st,ct=x,D(1)
    for n in range(1,201):
        st=-st*x*x/D((2*n)*(2*n+1));ct=-ct*x*x/D((2*n-1)*(2*n))
        sine+=st;cosine+=ct
    return sine,cosine

def cross(a,b):
    return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]

def mv(m,v):return [sum(m[i][j]*v[j] for j in range(3)) for i in range(3)]
def mm(a,b):return [[sum(a[i][k]*b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
def identity():return [[D(i==j) for j in range(3)] for i in range(3)]
def increment(linear,angular,dt):
    v=[D.from_float(x)*dt for x in linear]
    omega=[D.from_float(x)*dt for x in angular]
    x=sum(a*a for a in omega);theta=x.sqrt()
    if not theta:return identity(),v
    sine,cosine=sincos(theta);axis=[a/theta for a in omega]
    skew=[[D(0),-axis[2],axis[1]],[axis[2],D(0),-axis[0]],[-axis[1],axis[0],D(0)]]
    matrix=[[D(i==j)*cosine+(1-cosine)*axis[i]*axis[j]+sine*skew[i][j] for j in range(3)] for i in range(3)]
    a=(1-cosine)/x;b=(theta-sine)/(theta*x)
    first=cross(omega,v);second=cross(omega,first)
    return matrix,[v[i]+a*first[i]+b*second[i] for i in range(3)]

rows=[]
with localcontext() as ctx:
 ctx.prec=400
 for line in Path(sys.argv[1]).read_text().splitlines():
  if not line.startswith('SCREW_SPEED_ENCLOSURE '):continue
  segments,times,index,point,scale,bound=ast.literal_eval(line.split(' ',1)[1])
  matrix=identity();translation=[D(0)]*3
  for i in range(index):
   linear,angular,_=segments[i];dt=D.from_float(times[i][1])-D.from_float(times[i][0])
   r,t=increment(linear,angular,dt)
   translation=[a+b for a,b in zip(mv(r,translation),t)];matrix=mm(r,matrix)
  position=[a+b for a,b in zip(mv(matrix,list(map(D.from_float,point))),translation)]
  linear,angular,_=segments[index];dt=D.from_float(times[index][1])-D.from_float(times[index][0])
  rotational=cross(list(map(D.from_float,angular)),position)
  velocity=[a+D.from_float(b) for a,b in zip(rotational,linear)]
  speed=sum(v*v for v in velocity).sqrt()*dt*abs(D.from_float(scale))
  assert speed<=D.from_float(bound),(index,speed,bound)
  rows.append({'index':index,'point':point,'scale':scale,'reference':str(speed),'upper':bound})
assert len(rows)==27,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

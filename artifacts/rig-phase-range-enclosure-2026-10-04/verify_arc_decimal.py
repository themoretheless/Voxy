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
    v=[D.from_float(x)*D.from_float(dt) for x in linear]
    omega=[D.from_float(x)*D.from_float(dt) for x in angular]
    x=sum(a*a for a in omega);theta=x.sqrt()
    if not theta:return identity(),v
    sine,cosine=sincos(theta);axis=[a/theta for a in omega]
    skew=[[D(0),-axis[2],axis[1]],[axis[2],D(0),-axis[0]],[-axis[1],axis[0],D(0)]]
    matrix=[[D(i==j)*cosine+(1-cosine)*axis[i]*axis[j]+sine*skew[i][j] for j in range(3)] for i in range(3)]
    a=(1-cosine)/x;b=(theta-sine)/(theta*x)
    first=cross(omega,v);second=cross(omega,first)
    return matrix,[v[i]+a*first[i]+b*second[i] for i in range(3)]

def transpose(m):return [[m[j][i] for j in range(3)] for i in range(3)]
def quaternion_matrix(q):
    x,y,z,w=map(D.from_float,q);n=x*x+y*y+z*z+w*w
    return [[1-2*(y*y+z*z)/n,2*(x*y-z*w)/n,2*(x*z+y*w)/n],
        [2*(x*y+z*w)/n,1-2*(x*x+z*z)/n,2*(y*z-x*w)/n],
        [2*(x*z-y*w)/n,2*(y*z+x*w)/n,1-2*(x*x+y*y)/n]]
def numerator(q):
    x,y,z,w=q
    return [[-(y*y+z*z),x*y-z*w,x*z+y*w],
        [x*y+z*w,-(x*x+z*z),y*z-x*w],
        [x*z-y*w,y*z+x*w,-(x*x+y*y)]]
def polynomial(c,u,dt):
    c=[list(map(D.from_float,v)) for v in c];v=1-u;size=len(c[0])
    return ([c[0][j]*v**3+3*c[1][j]*u*v*v+3*c[2][j]*u*u*v+c[3][j]*u**3 for j in range(size)],
        [3*((c[1][j]-c[0][j])*v*v+2*(c[2][j]-c[1][j])*u*v+(c[3][j]-c[2][j])*u*u)/dt for j in range(size)])
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('ARC_RANGE_ENCLOSURE '):continue
        additive,pivot,frames,start,end,fraction,vbox,wbox=ast.literal_eval(line.removeprefix('ARC_RANGE_ENCLOSURE '))
        initial,axis,left,right=frames;dt=D.from_float(end)-D.from_float(start);u=D.from_float(fraction)
        a,da=polynomial(additive,u,dt);p,dp=polynomial(pivot,u,dt)
        l=mm(quaternion_matrix(left),quaternion_matrix(initial))
        arc,_=increment([0.,0.,0.],axis,fraction)
        rotation=mm(mm(l,arc),quaternion_matrix(right))
        angular=mv(l,[D.from_float(v)/dt for v in axis])
        coupling=cross(angular,a);moved=mv(rotation,dp)
        linear=[da[i]-coupling[i]-moved[i] for i in range(3)]
        for reference,bounds in zip(linear+angular,vbox+wbox):
            lo,hi=map(D.from_float,bounds)
            assert lo<=reference<=hi,(fraction,reference,bounds)
        rows.append({'fraction':fraction,'linear_reference':[str(x) for x in linear],'angular_reference':[str(x) for x in angular]})
assert rows and len(rows)%40==0,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

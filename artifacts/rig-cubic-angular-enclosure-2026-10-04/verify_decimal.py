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
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('CUBIC_ANGULAR_ENCLOSURE '):continue
        control,left,start,end,fraction,bounds=ast.literal_eval(line.removeprefix('CUBIC_ANGULAR_ENCLOSURE '))
        c=[list(map(D.from_float,v)) for v in control];u=D.from_float(fraction);v=1-u
        dt=D.from_float(end)-D.from_float(start)
        q=[c[0][j]*v*v*v+3*c[1][j]*u*v*v+3*c[2][j]*u*u*v+c[3][j]*u*u*u for j in range(4)]
        dq=[3*((c[1][j]-c[0][j])*v*v+2*(c[2][j]-c[1][j])*u*v+(c[3][j]-c[2][j])*u*u)/dt for j in range(4)]
        n=sum(x*x for x in q);dn=2*sum(x*y for x,y in zip(q,dq));a=numerator(q)
        plus=numerator([x+y for x,y in zip(q,dq)]);minus=numerator([x-y for x,y in zip(q,dq)])
        da=[[(plus[i][j]-minus[i][j])/2 for j in range(3)] for i in range(3)]
        matrix=[[D(i==j)+2*a[i][j]/n for j in range(3)] for i in range(3)]
        derivative=[[2*da[i][j]/n-2*a[i][j]*dn/(n*n) for j in range(3)] for i in range(3)]
        transpose=[[matrix[j][i] for j in range(3)] for i in range(3)]
        skew=mm(derivative,transpose)
        angular=[(skew[2][1]-skew[1][2])/2,(skew[0][2]-skew[2][0])/2,(skew[1][0]-skew[0][1])/2]
        angular=mv(quaternion_matrix(left),angular)
        for reference,bound in zip(angular,bounds):
            lo,hi=map(D.from_float,bound)
            assert lo<=reference<=hi,(fraction,reference,bound)
        rows.append({'start':start,'end':end,'fraction':fraction,'angular_reference':[str(x) for x in angular]})
assert rows and len(rows)%5==0,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

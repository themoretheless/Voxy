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
    v=[D.from_float(x)*(dt if isinstance(dt,D) else D.from_float(dt)) for x in linear]
    omega=[D.from_float(x)*(dt if isinstance(dt,D) else D.from_float(dt)) for x in angular]
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
        if not line.startswith('CACHED_SCREW_ENCLOSURE '):continue
        definitions,index,fraction,point,tbox,qbox,pbox=ast.literal_eval(line.removeprefix('CACHED_SCREW_ENCLOSURE '))
        matrix=identity();translation=[D(0)]*3
        for i,(linear,angular,start,end) in enumerate(definitions[:index+1]):
            duration=D.from_float(end)-D.from_float(start)
            if i==index:duration*=D.from_float(fraction)
            rotation,delta=increment(linear,angular,duration)
            translation=[a+b for a,b in zip(mv(rotation,translation),delta)]
            matrix=mm(rotation,matrix)
        w=(1+sum(matrix[i][i] for i in range(3))).sqrt()/2
        q=[(matrix[2][1]-matrix[1][2])/(4*w),(matrix[0][2]-matrix[2][0])/(4*w),
            (matrix[1][0]-matrix[0][1])/(4*w),w]
        transformed=[a+b for a,b in zip(mv(matrix,list(map(D.from_float,point))),translation)]
        for reference,bounds in zip(translation+q+transformed,tbox+qbox+pbox):
            lower,upper=map(D.from_float,bounds)
            assert lower<=reference<=upper,(index,fraction,reference,bounds)
        rows.append({'span':index,'fraction':fraction,'translation_reference':[str(v) for v in translation],
            'rotation_reference':[str(v) for v in q],'point_reference':[str(v) for v in transformed]})
assert len(rows)==12,len(rows)
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

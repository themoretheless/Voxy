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
rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('TWIST_FRAME_ENCLOSURE '):continue
        source,target,weights,progress,frame,scale,vbox,wbox=ast.literal_eval(line.removeprefix('TWIST_FRAME_ENCLOSURE '))
        source_v,source_w,source_clip,source_wall=source
        target_v,target_w,target_clip,target_wall=target
        a=D.from_float(source_clip)/D.from_float(source_wall)
        b=D.from_float(target_clip)/D.from_float(target_wall)
        weight=D.from_float(weights[0])+(D.from_float(weights[1])-D.from_float(weights[0]))*D.from_float(progress)
        v=[(1-weight)*a*D.from_float(x)+weight*b*D.from_float(y) for x,y in zip(source_v,target_v)]
        w=[(1-weight)*a*D.from_float(x)+weight*b*D.from_float(y) for x,y in zip(source_w,target_w)]
        offset,q=frame;matrix=quaternion_matrix(q);offset=list(map(D.from_float,offset))
        w=mv(matrix,w);coupling=cross(w,offset)
        v=[D.from_float(scale)*x-y for x,y in zip(mv(matrix,v),coupling)]
        for reference,bounds in zip(v+w,vbox+wbox):
            lower,upper=map(D.from_float,bounds)
            assert lower<=reference<=upper,(scale,reference,bounds)
        rows.append({'scale':scale,'linear_reference':[str(x) for x in v],'angular_reference':[str(x) for x in w]})
assert [r['scale'] for r in rows]==[-2.0,0.0,0.5,2.0]
print(json.dumps({'passed':True,'decimal_precision':400,'cases':rows},indent=2))

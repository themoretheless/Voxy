import ast
from fractions import Fraction as F
from pathlib import Path

def exact(v): return (F(v), F(v))
def add(a,b): return (a[0]+b[0], a[1]+b[1])
def neg(a): return (-a[1],-a[0])
def sub(a,b): return add(a,neg(b))
def mul(a,b):
    values=[x*y for x in a for y in b]
    return min(values),max(values)
def scale(a,b): return mul(a,exact(b))
def cross(a,b):
    return [sub(mul(a[1],b[2]),mul(a[2],b[1])),sub(mul(a[2],b[0]),mul(a[0],b[2])),sub(mul(a[0],b[1]),mul(a[1],b[0]))]
def trig(x, sine):
    term=x if sine else F(1)
    total=term
    for n in range(1,24):
        denominator=(2*n)*(2*n+1) if sine else (2*n-1)*(2*n)
        term=-term*x*x/denominator
        total+=term
    denominator=48*49 if sine else 47*48
    remainder=abs(term*x*x/denominator)
    assert abs(x)<=1
    return total-remainder,total+remainder

def increment(index, fraction):
    axis,rate,duration,velocity = (1,F.from_float(0.7),F(1,2),[F(1),F(0),F(1,2)]) if index==0 else (0,F.from_float(-0.6),F(1,4),[F(0),F(1,4),F(0)])
    dt=duration*fraction
    angle=rate*dt
    e=[exact(int(i==axis)) for i in range(3)]
    v=[exact(x) for x in velocity]
    first=cross(e,v)
    second=cross(e,first)
    a=scale(sub(exact(1),trig(angle,False)),1/rate)
    b=scale(sub(exact(angle),trig(angle,True)),1/rate)
    t=[add(add(scale(v[i],dt),mul(a,first[i])),mul(b,second[i])) for i in range(3)]
    q=[scale(trig(angle/2,True),int(i==axis)) for i in range(3)]+[trig(angle/2,False)]
    return t,q

def rotate(q,v):
    twice=[scale(x,2) for x in cross(q[:3],v)]
    second=cross(q[:3],twice)
    return [add(add(v[i],mul(q[3],twice[i])),second[i]) for i in range(3)]
def compose(a,b):
    at,aq=a;bt,bq=b
    moved=rotate(aq,bt)
    t=[add(at[i],moved[i]) for i in range(3)]
    q=[
        sub(add(add(mul(aq[3],bq[0]),mul(aq[0],bq[3])),mul(aq[1],bq[2])),mul(aq[2],bq[1])),
        add(add(sub(mul(aq[3],bq[1]),mul(aq[0],bq[2])),mul(aq[1],bq[3])),mul(aq[2],bq[0])),
        add(sub(add(mul(aq[3],bq[2]),mul(aq[0],bq[1])),mul(aq[1],bq[0])),mul(aq[2],bq[3])),
        sub(sub(sub(mul(aq[3],bq[3]),mul(aq[0],bq[0])),mul(aq[1],bq[1])),mul(aq[2],bq[2])),
    ]
    return t,q

identity=([exact(0)]*3,[exact(0)]*3+[exact(1)])
prefix=increment(0,F(1))
zeros=[exact(0)]*3
basis=[exact(F(1,2))]*4
inverse=[neg(v) for v in basis[:3]]+[basis[3]]
actor=[exact(F(1,2)),exact(F(-1,2)),exact(F(1,2)),exact(F(1,2))]
origin=[exact(F(1,4)),exact(F(-1,8)),exact(F(1,2))]
center=[exact(F(65536)+F(5,512)),exact(2),exact(-3)]
def product(a,b): return compose((zeros,a),(zeros,b))[1]
poses={}
count=0
for line in Path('artifacts/rig-physical-whole-body-uniform-error-2026-10-04/reference.log').read_text().splitlines():
    if 'PHYSICAL_BODY_ERROR ' not in line: continue
    span,fraction,corner,actual,axes,radius=ast.literal_eval(line.split('PHYSICAL_BODY_ERROR ',1)[1])
    key=(span,fraction)
    if key not in poses:
        t,q=compose(increment(span,F.from_float(fraction)),identity if span==0 else prefix)
        reframe=product(product(basis,q),inverse)
        orientation=product(actor,reframe)
        scaled=[scale(v,-2) for v in rotate(basis,t)]
        rotated_pivot=rotate(reframe,origin)
        offset=[sub(add(scaled[i],origin[i]),rotated_pivot[i]) for i in range(3)]
        displacement=rotate(actor,offset)
        moved=[add(center[i],displacement[i]) for i in range(3)]
        poses[key]=(moved,orientation)
    moved,orientation=poses[key]
    point=[exact((F(1,8),F(1,4),F(1,2))[i]*(1 if corner & (1<<i) else -1)) for i in range(3)]
    image=rotate(orientation,point)
    reference=[add(moved[i],image[i]) for i in range(3)]
    errors=[]
    for interval,stored,cap in zip(reference,actual,axes):
        error=max(abs(F.from_float(stored)-v) for v in interval)
        assert error<=F.from_float(cap)
        errors.append(error)
        count+=1
    assert sum(errors)<=F.from_float(radius)
    count+=1
assert count==320,count
print('PASS: 240 axis and 80 L1 actual physical corner errors enclosed against rational pivoted SE(3), basis/actor composition and affine box; no tolerance')

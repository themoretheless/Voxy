import ast
import itertools
import re
import sys
from fractions import Fraction as F
from pathlib import Path

source, diagnostic = map(lambda p: Path(p).read_text(), sys.argv[1:])
def value(text):
    return ast.literal_eval(text)

def vector(values):
    return tuple(map(F, values))

def add(a,b):
    return tuple(x+y for x,y in zip(a,b))

def sub(a,b):
    return tuple(x-y for x,y in zip(a,b))

def scale(a,k):
    return tuple(x*k for x in a)

def dot(a,b):
    return sum(x*y for x,y in zip(a,b))

def cross(a,b):
    x,y,z = a
    u,v,w = b
    return (y*w-z*v,z*u-x*w,x*v-y*u)

def l1(v):
    return sum(map(abs,v))

def rotate(q,v):
    u,w = q[:3],q[3]
    t = scale(cross(u,v),2)
    return add(v,add(scale(t,w),cross(u,t)))

positions = [vector(value(x)) for x in re.findall(r'position: (\[[^\]]+\])', source)]
velocities = [vector(value(x)) for x in re.findall(r'velocity: (\[[^\]]+\])', source)]
q = vector(value(re.search(r'orientation: (\[[^\]]+\])',source)[1]))
arcs = value(re.search(r'second_arcs=(\[.*?\]) first_jerk=',diagnostic)[1])
assert 'first_arcs=[]' in diagnostic
assert len(arcs)==1 and arcs[0][0]==0
duration=F(float(re.search(r'MANIFOLD ENDPOINTS[^\n]+duration=([^ ]+)',diagnostic)[1]))
assert F(arcs[0][1])==duration
omega=vector(arcs[0][2])
a1,a2=(vector(value(re.search(rf'{owner}_acceleration=(\[[^\]]+\])',diagnostic)[1])) for owner in ('first','second'))
j1,j2=(vector(value(re.search(rf'{owner}_jerk=(\[[^\]]+\])',diagnostic)[1])) for owner in ('first','second'))
x,y,z=((1,0,0),(0,1,0),(0,0,1))
rot_z=rotate(q,z)
axis=scale(cross(y,rot_z),-1)
axis_rate=scale(cross(y,cross(omega,rot_z)),-1)
dpos=sub(*positions)
dvel=sub(*velocities)
dacc=sub(a1,a2)
djerk=sub(j1,j2)
speed=l1(omega)
# An orthogonal rotation preserves the Euclidean norm. L1 dominates
# the norm, giving rational derivative majorants for the complete arc.
axis_0=l1(rot_z)
axis_1=axis_0*speed
axis_2=axis_1*speed
axis_delta=axis_1*duration
unit_lo=1-F(1,10**8)
unit_hi=1+F(1,10**8)
assert (unit_lo+axis_delta)**2 < dot(axis,axis) < (unit_hi-axis_delta)**2
minimum_lower=None
minimum_upper=None
for a,b,c,d in itertools.product((-1,1), repeat=4):
    left=(F(0.025)*a,0,F(0.025)*b)
    right=rotate(q,(F(0.0625)*c,F(0.0625)*d,0))
    r=sub(add(dpos,left),right)
    v=sub(dvel,cross(omega,right))
    right_norm=l1(right)
    r0=l1(dpos)+l1(left)+right_norm+l1(dvel)*duration+l1(dacc)*duration**2/2+l1(djerk)*duration**3/6
    r1=l1(dvel)+l1(dacc)*duration+l1(djerk)*duration**2/2+speed*right_norm
    r2=l1(dacc)+l1(djerk)*duration+speed**2*right_norm
    error=(axis_2*r0+2*axis_1*r1+axis_0*r2)*duration**2/2
    start=dot(axis,r)
    end_linear=start+dot(axis_rate,r)*duration+dot(axis,v)*duration
    lo=min(start,end_linear)-error
    hi=max(start,end_linear)+error
    bounds=[lo/unit_lo,lo/unit_hi,hi/unit_lo,hi/unit_hi]
    lower,upper=min(bounds),max(bounds)
    minimum_lower=lower if minimum_lower is None else min(lower,minimum_lower)
    minimum_upper=upper if minimum_upper is None else min(upper,minimum_upper)
limit=F(1,10**10)
assert -limit < minimum_lower <= minimum_upper < limit
print(f'Nominal rational interval: {float(minimum_lower):.17e} .. {float(minimum_upper):.17e} m')
print(f'Absolute margin: {float(limit-max(abs(minimum_lower),abs(minimum_upper))):.17e} m')

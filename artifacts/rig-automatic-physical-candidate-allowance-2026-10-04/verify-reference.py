import ast
from fractions import Fraction as F
from pathlib import Path
rows=[ast.literal_eval(line.split('AUTOMATIC_PHYSICAL_MARGIN ',1)[1]) for line in Path('artifacts/rig-automatic-physical-candidate-allowance-2026-10-04/reference.log').read_text().splitlines() if 'AUTOMATIC_PHYSICAL_MARGIN ' in line]
assert len(rows)==1
radius,wall,displacement=map(F.from_float,rows[0])
assert 0 < wall-F(1,2) < radius
assert 0 < displacement < F(1,4)
# Exact dyadic support separation for the accepted translating body.
assert displacement+F(1,8) < wall-F(1,8)
actual_center=F.from_float(float(displacement))
assert actual_center+F(1,8) < wall-F(1,8)
print('PASS: exact rational near-wall gap is inside derived allowance, accepted displacement precedes the endpoint, and accepted support remains separated; no tolerance')

# Independent homogeneous rotation matrix proof for the admitted actor row.
# Each component is a linear polynomial in arbitrary cosine/sine coordinates.
def padd(a,b): return tuple(x+y for x,y in zip(a,b))
def pneg(a): return tuple(-x for x in a)
def psub(a,b): return padd(a,pneg(b))
def product(a,b): return (a[0]*b[0],a[0]*b[1]+a[1]*b[0],a[1]*b[1])
def twice(a): return tuple(2*x for x in a)
def matrix(q):
    x,y,z,w=q
    xx,yy,zz,ww=[product(v,v) for v in q]
    xy,xz,yz,wx,wy,wz=[product(a,b) for a,b in [(x,y),(x,z),(y,z),(w,x),(w,y),(w,z)]]
    return [[psub(psub(padd(ww,xx),yy),zz),twice(psub(xy,wz)),twice(padd(xz,wy))],
            [twice(padd(xy,wz)),psub(psub(padd(ww,yy),xx),zz),twice(psub(yz,wx))],
            [twice(psub(xz,wy)),twice(padd(yz,wx)),psub(psub(padd(ww,zz),xx),yy)]]
def hamilton(a,b):
    x,y,z,w=a; X,Y,Z,W=b
    return [w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z]
actors=[[F(0),F(0),F(0),F(1)]]
for axis in range(3):
    for scalar in [0,1]:
        for sign in [-1,1]:
            q=[F(0)]*4;q[axis]=F(sign);q[3]=F(scalar);actors.append(q)
for i in range(3):
    for j in range(i+1,3):
        for sign in [-1,1]:
            q=[F(0)]*4;q[i]=F(1);q[j]=F(sign);actors.append(q)
for bits in range(8):
    actors.append([F(1 if bits&(1<<i) else -1,2) for i in range(3)]+[F(1,2)])
actors.append([F.from_float(v) for v in [0.123,0.123,-0.7,0.7]])
checks=0
for actor in actors:
    norm=sum(v*v for v in actor)
    original=matrix([(v,F(0)) for v in actor])
    for world_axis in range(3):
        rows=[i for i in range(3) if abs(original[world_axis][i][0])==norm and all(original[world_axis][j][0]==0 for j in range(3) if j!=i)]
        for actor_axis in rows:
            sign=original[world_axis][actor_axis][0]/norm
            imaginary=[F(int(i==actor_axis)) for i in range(3)]+[F(0)]
            q=list(zip(actor,hamilton(actor,imaginary)))
            squared=(F(0),F(0),F(0))
            for v in q: squared=padd(squared,product(v,v))
            row=matrix(q)[world_axis]
            for axis in range(3):
                assert row[axis] == (tuple(sign*v for v in squared) if axis==actor_axis else (F(0),F(0),F(0)))
                checks+=1
assert checks>=200,checks
print(f'PASS: {checks} exact polynomial row comparisons for arbitrary axial sine/cosine, signed actor permutations and continuous coordinate-row actor; no tolerance')

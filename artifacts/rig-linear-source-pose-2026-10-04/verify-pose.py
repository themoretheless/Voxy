import ast, sys, runpy
from fractions import Fraction as F
from pathlib import Path
# Reuse the independent rational angle/sqrt proof, not engine output values.
n = runpy.run_path('artifacts/rig-source-spatial-field-2026-10-04/verify-linear-log.py')
root, angle = n['sqrt_bounds'], n['angle']
scale=10**100
def snap(x):
    lo=x[0].numerator*scale//x[0].denominator
    hi=-((-x[1].numerator*scale)//x[1].denominator)
    return F(lo,scale),F(hi,scale)
def add(a,b): return snap((a[0]+b[0],a[1]+b[1]))
def neg(a): return -a[1],-a[0]
def sub(a,b): return add(a,neg(b))
def mul(a,b):
    v=[x*y for x in a for y in b]
    return snap((min(v),max(v)))
def div(a,b): return mul(a,(1/b[1],1/b[0]))
def exact(x): return x,x
def trig_point(x,cosine):
    x=exact(x); sq=mul(x,x)
    term=exact(F(1)) if cosine else x
    total=term
    for k in range(1,60):
        d=(2*k-1)*(2*k) if cosine else (2*k)*(2*k+1)
        term=div(mul(term,sq),exact(F(d)))
        total=add(total,term) if k%2==0 else sub(total,term)
    d=119*120 if cosine else 120*121
    tail=div(mul(term,sq),exact(F(d)))[1]
    return add(total,(-tail,tail))
count=0
for line in Path(sys.argv[2]).read_text().splitlines():
    if 'SOURCE_LINEAR_POSE ' not in line: continue
    keys,u,bounds=ast.literal_eval(line.split('SOURCE_LINEAR_POSE ',1)[1])
    a,b=[[F.from_float(x) for x in q] for q in keys]
    v=[b[0]*a[3]-b[3]*a[0]-b[1]*a[2]+b[2]*a[1],
       b[1]*a[3]-b[3]*a[1]-b[2]*a[0]+b[0]*a[2],
       b[2]*a[3]-b[3]*a[2]-b[0]*a[1]+b[1]*a[0]]
    dot=sum(x*y for x,y in zip(a,b))
    if dot<0: dot=-dot;v=[-x for x in v]
    norm=root(sum(x*x for x in v))
    phi=snap((angle(norm[0],dot)[0],angle(norm[1],dot)[1]))
    t=mul(phi,exact(F.from_float(u)))
    # Evaluate each endpoint; derivative <=1 covers the tiny rational corridor.
    # This avoids assuming sin/cos monotonicity at the enclosed pi/2 endpoint.
    sin=trig_point(t[0],False); cos=trig_point(t[0],True)
    radius=t[1]-t[0]
    sin=add(sin,(-radius,radius));cos=add(cos,(-radius,radius))
    delta=[mul(div(exact(x),norm),sin) for x in v]+[cos]
    an=root(sum(x*x for x in a)); a=[div(exact(x),an) for x in a]
    x,y,z,w=delta; ax,ay,az,aw=a
    ref=[sub(add(add(mul(w,ax),mul(x,aw)),mul(y,az)),mul(z,ay)),
         add(add(sub(mul(w,ay),mul(x,az)),mul(y,aw)),mul(z,ax)),
         add(sub(add(mul(w,az),mul(x,ay)),mul(y,ax)),mul(z,aw)),
         sub(sub(sub(mul(w,aw),mul(x,ax)),mul(y,ay)),mul(z,az))]
    for i,(r,actual) in enumerate(zip(ref,bounds)):
        assert F.from_float(actual[0])<=r[0]<=r[1]<=F.from_float(actual[1]),(keys,u,i,actual)
        count+=1
assert count==120,count
print(f'PASS: {count} source LINEAR pose components, independent rational series/sqrt bounds')

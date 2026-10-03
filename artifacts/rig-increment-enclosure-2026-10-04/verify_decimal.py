"""Independent 400-digit Rodrigues/quaternion reference from exact f64 inputs."""
from decimal import Decimal as D, localcontext
from pathlib import Path
import ast,json,sys

def sincos(x):
    sine,cosine=x,D(1)
    st,ct=x,D(1)
    for n in range(1,201):
        st=-st*x*x/D((2*n)*(2*n+1))
        ct=-ct*x*x/D((2*n-1)*(2*n))
        sine+=st;cosine+=ct
    return sine,cosine

def cross(a,b):
    return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]

rows=[]
with localcontext() as ctx:
    ctx.prec=400
    for line in Path(sys.argv[1]).read_text().splitlines():
        if not line.startswith('SCREW_ENCLOSURE '):continue
        linear,angular,dt,tbox,qbox=ast.literal_eval(line.removeprefix('SCREW_ENCLOSURE '))
        v=[D.from_float(x)*D.from_float(dt) for x in linear]
        omega=[D.from_float(x)*D.from_float(dt) for x in angular]
        x=sum(a*a for a in omega);theta=x.sqrt()
        if not theta:
            translation=v;rotation=[D(0),D(0),D(0),D(1)]
        else:
            sine,cosine=sincos(theta)
            sh,ch=sincos(theta/2)
            a=(1-cosine)/x;b=(theta-sine)/(theta*x)
            first=cross(omega,v);second=cross(omega,first)
            translation=[v[i]+a*first[i]+b*second[i] for i in range(3)]
            rotation=[w*sh/theta for w in omega]+[ch]
        for reference,bounds in zip(translation+rotation,tbox+qbox):
            lower,upper=map(D.from_float,bounds)
            assert lower<=reference<=upper,(reference,bounds)
            assert upper-lower<=D('1e-12')*(1+abs(reference))
        rows.append({'linear':linear,'angular':angular,'duration':dt,
            'translation_reference':[str(v) for v in translation],
            'rotation_reference':[str(v) for v in rotation]})
assert len(rows)==5,len(rows)
print(json.dumps({'passed':True,'cases':len(rows),'decimal_precision':400,'references':rows},indent=2))

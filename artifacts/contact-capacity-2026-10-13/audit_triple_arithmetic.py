"""Exact rational audit of exported GPU components; never admits contacts."""
import ast, json, struct, sys
from fractions import Fraction as F

def f64(bits): return struct.unpack('<d',struct.pack('<Q',bits))[0]
def f32(bits): return struct.unpack('<f',struct.pack('<I',bits))[0]
counts=dict(cases=0,sum_decode_mismatch=0,product_decode_mismatch=0,sum_exact_rounded_mismatch=0,product_exact_rounded_mismatch=0)
examples=[]
for line in open(sys.argv[1]):
    index,ab,bb,sb,parts=line.split(' ',4)
    a,b,scale=map(f64,map(int,(ab,bb,sb)))
    parts=list(map(f32,ast.literal_eval(parts)))
    counts['cases']+=1
    for op,offset,power,expected in [('sum',0,1,a+b),('product',3,2,a*b)]:
        x,y,z=parts[offset:offset+3]
        exact=(F(x)+F(y)+F(z))*F(scale)**power
        decoded=((x+y)+z)*scale
        if power==2: decoded*=scale
        if decoded!=expected: counts[op+'_decode_mismatch']+=1
        if float(exact)!=expected:
            counts[op+'_exact_rounded_mismatch']+=1
            if len(examples)<8: examples.append(dict(case=int(index),operation=op,a=a,b=b,parts=[x,y,z],scale=scale,expected=expected,exact_rounded=float(exact),decoded=decoded))
print(json.dumps(dict(counts=counts,examples=examples,scope='arithmetic output audit only; not physical admission'),indent=2))

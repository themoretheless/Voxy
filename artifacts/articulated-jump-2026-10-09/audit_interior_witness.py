"""Brute-force triangle-distance audit near an interior segment witness."""
import struct, json, sys
from pathlib import Path
import numpy as np
raw=Path(sys.argv[1]).read_bytes();assert raw[:4]==b'VHR1';offset=36
# VHR1: three scalar values, self-collision count, rod count, then rod records.
offset=4+3*8+8
nr=struct.unpack_from('<Q',raw,offset)[0];offset+=8
for _ in range(nr):
    offset+=7*8
    for width in [3]*6+[4]*5:
        n=struct.unpack_from('<Q',raw,offset)[0];offset+=8+n*width*8
nm=struct.unpack_from('<Q',raw,offset)[0];offset+=8;assert nm==1
vectors=[]
for _ in range(3):
    n=struct.unpack_from('<Q',raw,offset)[0];offset+=8
    vectors.append(np.frombuffer(raw,dtype='<f8',count=n*3,offset=offset).reshape(n,3));offset+=n*3*8
n=struct.unpack_from('<Q',raw,offset)[0];offset+=8
faces=np.frombuffer(raw,dtype='<u8',count=n*3,offset=offset).reshape(n,3)
tri=vectors[0][faces];a,b,c=tri[:,0],tri[:,1],tri[:,2];ab=b-a;ac=c-a
normal=np.cross(ab,ac);normal/=np.linalg.norm(normal,axis=1)[:,None]
d00=np.einsum('ij,ij->i',ab,ab);d01=np.einsum('ij,ij->i',ab,ac);d11=np.einsum('ij,ij->i',ac,ac);den=d00*d11-d01*d01

def nearest(p):
    delta=p-a;d20=np.einsum('ij,ij->i',delta,ab);d21=np.einsum('ij,ij->i',delta,ac)
    v=(d11*d20-d01*d21)/den;w=(d00*d21-d01*d20)/den
    plane=a+v[:,None]*ab+w[:,None]*ac
    candidates=[plane]
    for x,y in [(a,b),(b,c),(c,a)]:
        edge=y-x;t=np.clip(np.einsum('ij,ij->i',p-x,edge)/np.einsum('ij,ij->i',edge,edge),0,1)
        candidates.append(x+t[:,None]*edge)
    distance=[np.einsum('ij,ij->i',q-p,q-p) for q in candidates]
    distance[0][(v<0)|(w<0)|(v+w>1)]=np.inf
    stack=np.stack(distance);k,i=np.unravel_index(np.argmin(stack),stack.shape)
    q=candidates[k][i];d=np.sqrt(stack[k,i])
    # This audit's probes are inside the captured solid interval.
    return d,(q-p)/d,int(i)

data=json.loads(Path(sys.argv[2]).read_text());contacts=data['mesh_contacts'];worst=min(contacts,key=lambda c:c['gap']);r=worst['rod'];i=worst['segment'];t=worst['fraction']
x=np.array(data['before_positions'][r][i]);y=np.array(data['before_positions'][r][i+1]);direction=y-x
rows=[]
for delta in [-1e-5,-1e-6,0,1e-6,1e-5]:
    d,n,face=nearest(x+(t+delta)*direction)
    rows.append({'fraction_offset':delta,'distance_m':d,'metric_normal':n.tolist(),'normal_dot_segment_m':float(n@direction),'triangle':face})
left=np.array(rows[1]['metric_normal']);right=np.array(rows[3]['metric_normal']);sl=left@direction;sr=right@direction
result={'rod':r,'segment':i,'fraction':t,'probes':rows,'scope':'Nearest triangle distances only at interior probes; no global clearance certificate.'}
if sl<0<sr:
    weight=float(sr/(sr-sl));g=weight*left+(1-weight)*right
    result.update({'left_weight':weight,'stationary_subgradient':g.tolist(),'subgradient_norm':float(np.linalg.norm(g)),'stationary_dot_segment_m':float(g@direction)})
print(json.dumps(result,indent=2))

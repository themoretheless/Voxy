#!/usr/bin/env python3
"""Classify static audit candidates; does not waive contacts or certify anatomy."""
import collections,json,math
from pathlib import Path
import prepare_body_geometry as g

def classify(a,b):
 n=g.cross(g.sub(a[1],a[0]),g.sub(a[2],a[0]));m=g.cross(g.sub(b[1],b[0]),g.sub(b[2],b[0]))
 if g.length(n)==0 or g.length(m)==0:return 'degenerate'
 scale=max(g.length(g.sub(t[k],t[(k+1)%3])) for t in (a,b) for k in range(3));tol=scale*1e-10
 distances=lambda triangle,origin,normal:[g.dot(normal,g.sub(p,origin))/g.length(normal) for p in triangle]
 da=distances(a,b[0],m);db=distances(b,a[0],n)
 direction=g.cross(n,m);parallel=g.length(direction)<=1e-10*g.length(n)*g.length(m)
 if parallel:
  if max(abs(x) for x in db)>tol:return 'parallel_separated'
  axis=max(range(3),key=lambda k:abs(n[k]));axes=[k for k in range(3) if k!=axis]
  polygon=[tuple(p[k] for k in axes) for p in a];clip=[tuple(p[k] for k in axes) for p in b]
  def cross2(p,q):return p[0]*q[1]-p[1]*q[0]
  def sub2(p,q):return p[0]-q[0],p[1]-q[1]
  signed=sum(cross2(clip[k],clip[(k+1)%3]) for k in range(3));sign=1 if signed>=0 else -1
  for k in range(3):
   start,end=clip[k],clip[(k+1)%3];edge=sub2(end,start);output=[]
   if not polygon:break
   for p,q in zip(polygon,polygon[1:]+polygon[:1]):
    dp=sign*cross2(edge,sub2(p,start));dq=sign*cross2(edge,sub2(q,start));pin=dp>=0;qin=dq>=0
    if pin:output.append(p)
    if pin!=qin:
     t=dp/(dp-dq);output.append(tuple(p[j]+t*(q[j]-p[j]) for j in range(2)))
   polygon=output
  area=abs(sum(cross2(polygon[k],polygon[(k+1)%len(polygon)]) for k in range(len(polygon))))/2 if polygon else 0
  return 'coplanar_area_overlap' if area>scale*scale*1e-10 else 'coplanar_boundary_contact'
 if not g.crossing(a,b):return 'separated'
 axis=tuple(x/g.length(direction) for x in direction)
 def interval(triangle,ds):
  points=[]
  for k,p in enumerate(triangle):
   q=triangle[(k+1)%3];d,e=ds[k],ds[(k+1)%3]
   if abs(d)<=tol:points.append(p)
   if d*e<0:
    t=d/(d-e);points.append(tuple(p[j]+t*(q[j]-p[j]) for j in range(3)))
  values=[g.dot(p,axis) for p in points]
  return (min(values),max(values)) if values else None
 ia,ib=interval(a,da),interval(b,db)
 if ia is None or ib is None:return 'tolerance_candidate'
 overlap=min(ia[1],ib[1])-max(ia[0],ib[0])
 if overlap<=tol:return 'point_contact'
 both=min(da)<-tol and max(da)>tol and min(db)<-tol and max(db)>tol
 return 'transverse_segment_crossing' if both else 'tangential_segment_contact'

if __name__=='__main__':
 source=Path('assets/characters/blender-female/prepared/body-foot-local-repair-v9.obj');points,faces=g.load(source);r=g.intersections(points,faces,limit=100000);counts=collections.Counter();details=[]
 for i,j in r['intersection_pairs']:
  kind=classify([points[v] for v in faces[i]],[points[v] for v in faces[j]]);counts[kind]+=1;details.append({'faces':[i,j],'classification':kind})
 report={'source':str(source),'candidateCount':len(details),'classificationCounts':dict(counts),'pairs':details,'scope':'Floating-point static classification; contacts remain audit findings; shared-vertex pairs excluded; no anatomy or CCD certification.'}
 Path('docs/body-intersection-classification.json').write_text(json.dumps(report,indent=2)+'\n');print(dict(counts))

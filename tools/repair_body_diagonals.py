#!/usr/bin/env python3
"""Conservative local edge flips; candidates only, no inferred anatomy."""
import argparse,collections,hashlib,itertools,json,math,time
from pathlib import Path
import prepare_body_geometry as g

def quality_pair_allowed(old,new,floor):
 return all(q+1e-12>=min(floor,start) for q,start in zip(sorted(new),sorted(old)))

def repair(points,faces,passes=8,max_edge=.012,include_adjacent=False,reduce_measure=False,quality_floor=0.):
 if not math.isfinite(quality_floor) or not 0<=quality_floor<=1:raise ValueError('quality floor must be within 0..1')
 faces=list(faces);triangles=[[points[i] for i in t] for t in faces]
 if quality_floor:
  from repair_body_vertex_descent import triangle_quality
  qualities=[triangle_quality(t) for t in triangles];initial_qualities=list(qualities)
 initial=g.intersections(points,faces,limit=100000)
 if initial['intersection_limit_reached']:raise ValueError('uncapped audit required')
 hits={tuple(pair) for pair in initial['intersection_pairs']};history=[]
 if include_adjacent:
  from audit_body_adjacent_faces import audit as adjacent_audit,forbidden
  hits.update(tuple(pair) for pair in adjacent_audit(points,faces)['forbidden_pairs'])
 initial_count=len(hits)
 if reduce_measure:
  from repair_body_vertex_descent import intersection_measure
 def measure(pairs,replacement=None):
  replacement=replacement or {}
  return math.fsum(intersection_measure(replacement.get(a,triangles[a]),replacement.get(b,triangles[b])) for a,b in sorted(pairs))
 initial_measure=measure(hits) if reduce_measure else None
 edges=collections.defaultdict(set);buckets=collections.defaultdict(set);cell=.012
 def keys(triangle):
  ranges=[range(math.floor(min(p[k] for p in triangle)/cell),math.floor(max(p[k] for p in triangle)/cell)+1) for k in range(3)]
  return list(itertools.product(*ranges))
 def face_edges(t):return [tuple(sorted((t[k],t[(k+1)%3]))) for k in range(3)]
 for i,t in enumerate(faces):
  for edge in face_edges(t):edges[edge].add(i)
  for key in keys(triangles[i]):buckets[key].add(i)
 for iteration in range(passes):
  changed=False
  candidates=sorted({edge for pair in hits for i in pair for edge in face_edges(faces[i])})
  for edge in candidates:
   adjacent=sorted(edges.get(edge,()))
   if len(adjacent)!=2:continue
   i,j=adjacent
   oldhits={pair for pair in hits if i in pair or j in pair}
   if not oldhits:continue
   first,second=faces[i],faces[j]
   k=next(k for k in range(3) if tuple(sorted((first[k],first[(k+1)%3])))==edge)
   a,b,c=first[k],first[(k+1)%3],first[(k+2)%3]
   d=next(v for v in second if v not in edge)
   if c==d or edges.get(tuple(sorted((c,d)))):continue
   if g.length(g.sub(points[c],points[d]))>max_edge:continue
   replacement={i:(c,d,b),j:(d,c,a)}
   def normal(t):return g.cross(g.sub(points[t[1]],points[t[0]]),g.sub(points[t[2]],points[t[0]]))
   oldnormal=tuple(x+y for x,y in zip(normal(first),normal(second)))
   if any(g.length(normal(t))<=2e-14 or g.dot(normal(t),oldnormal)<=0 for t in replacement.values()):continue
   newtri={idx:[points[v] for v in t] for idx,t in replacement.items()};newhits=set()
   newqualities={idx:triangle_quality(t) for idx,t in newtri.items()} if quality_floor else None
   if newqualities is not None and not quality_pair_allowed([qualities[i],qualities[j]],list(newqualities.values()),quality_floor):continue
   for idx,t in replacement.items():
    nearby=set()
    for key in keys(newtri[idx]):nearby.update(buckets.get(key,()))
    nearby.update(replacement)
    for other in nearby:
     otherface=replacement.get(other,faces[other])
     if idx==other:continue
     shared=len(set(t)&set(otherface));othertri=newtri.get(other,triangles[other])
     if shared:
      if not include_adjacent:continue
      crosses=forbidden(newtri[idx],othertri,shared)
     else:crosses=g.crossing(newtri[idx],othertri)
     if crosses:newhits.add(tuple(sorted((idx,other))))
   old_measure=measure(oldhits) if reduce_measure else None
   new_measure=measure(newhits,newtri) if reduce_measure else None
   if reduce_measure and not newhits.issubset(oldhits):continue
   shorter=reduce_measure and len(newhits)==len(oldhits) and new_measure<old_measure-max(1e-12,old_measure*1e-8)
   if len(newhits)>=len(oldhits) and not shorter:continue
   for idx in replacement:
    for e in face_edges(faces[idx]):edges[e].discard(idx)
    for key in keys(triangles[idx]):buckets[key].discard(idx)
   for idx,t in replacement.items():
    faces[idx]=t;triangles[idx]=newtri[idx]
    if newqualities is not None:qualities[idx]=newqualities[idx]
    for e in face_edges(t):edges[e].add(idx)
    for key in keys(triangles[idx]):buckets[key].add(idx)
   before=len(hits);hits.difference_update(oldhits);hits.update(newhits)
   history.append({'pass':iteration,'edge':edge,'faces':[i,j],'before':before,'after':len(hits),'local_measure_before_m':old_measure,'local_measure_after_m':new_measure});changed=True
  if not changed:break
 final=g.intersections(points,faces,limit=100000)
 final_pairs={tuple(p) for p in final['intersection_pairs']}
 if include_adjacent:
  final['adjacent']=adjacent_audit(points,faces)
  final_pairs.update(tuple(p) for p in final['adjacent']['forbidden_pairs'])
 if final['intersection_limit_reached'] or final_pairs!=hits:raise ValueError('incremental/full audit mismatch')
 audit=g.audit(points,faces)
 if any(audit[k] for k in ('degenerate_triangles','duplicate_triangles','boundary_edges','nonmanifold_edges','inconsistent_winding_edges')):raise ValueError('invalid repaired topology')
 quality_audit=None
 if quality_floor:
  full=[triangle_quality(t) for t in triangles]
  if any(abs(a-b)>1e-12 for a,b in zip(full,qualities)) or any(q+1e-10<start for q,start in zip(sorted(min(quality_floor,q) for q in full),sorted(min(quality_floor,q) for q in initial_qualities))):raise ValueError('full quality audit mismatch')
  quality_audit={'floor':quality_floor,'initialMinimum':min(initial_qualities),'finalMinimum':min(full),'initialBelowFloor':sum(q<quality_floor for q in initial_qualities),'finalBelowFloor':sum(q<quality_floor for q in full),'verified':True}
 return faces,{'initialCrossings':initial_count,'finalCrossings':len(hits),'includesAdjacent':include_adjacent,'reduceMeasure':reduce_measure,'qualityAudit':quality_audit,'initialMeasureM':initial_measure,'finalMeasureM':measure(hits) if reduce_measure else None,'remaining':final,'steps':history,'audit':audit,'verticesMoved':False,'scope':'Static candidate audit; new triangulation requires anatomical review and is not runtime geometry.'}

if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('source',type=Path)
 parser.add_argument('output',type=Path)
 parser.add_argument('--report',type=Path,required=True)
 parser.add_argument('--passes',type=int,default=8)
 parser.add_argument('--max-edge-mm',type=float,default=12.)
 parser.add_argument('--adjacent',action='store_true')
 parser.add_argument('--reduce-measure',action='store_true')
 parser.add_argument('--quality-floor',type=float,default=0.)
 args=parser.parse_args()
 if args.passes<1 or not math.isfinite(args.max_edge_mm) or args.max_edge_mm<=0:parser.error('positive passes and finite positive edge limit required')
 source=args.source;out=args.output
 if out.exists():parser.error('output already exists; preserve previous candidate')
 points,faces=g.load(source);start=time.perf_counter();candidate,r=repair(points,faces,passes=args.passes,max_edge=args.max_edge_mm/1000.,include_adjacent=args.adjacent,reduce_measure=args.reduce_measure,quality_floor=args.quality_floor);r['elapsedSeconds']=time.perf_counter()-start
 with out.open('x') as f:
  for p in points:f.write('v '+' '.join(format(x,'.17g') for x in p)+'\n')
  for t in candidate:f.write('f '+' '.join(str(i+1) for i in t)+'\n')
 reloaded_points,reloaded_faces=g.load(out)
 if reloaded_points!=points or reloaded_faces!=candidate:raise ValueError('export reload mismatch')
 r.update(source=str(source),sourceSha256=hashlib.sha256(source.read_bytes()).hexdigest(),candidateSha256=hashlib.sha256(out.read_bytes()).hexdigest(),exportReloadVerified=True,adopted=False)
 args.report.write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps({k:v for k,v in r.items() if k not in ('steps','remaining')},indent=2));print('accepted flips:',len(r['steps']))

#!/usr/bin/env python3
"""Auditable neutral body-mesh preparation; no inferred internal anatomy.
Writes a new mesh and sidecar, never replaces the source. Python standard library only.
"""
import argparse, collections, hashlib, json, math
from pathlib import Path


def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def cross(a,b): return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def length(a): return math.sqrt(dot(a,a))

def load(path):
    points=[]; faces=[]
    for line in Path(path).read_text().splitlines():
        if line.startswith('v '): points.append(tuple(map(float,line.split()[1:4])))
        if line.startswith('f '):
            face=[int(v.split('/')[0]) for v in line.split()[1:]]
            face=[v-1 if v>0 else len(points)+v for v in face]
            if len(face)!=3: raise ValueError('only triangular OBJ supported; triangulate explicitly')
            faces.append(tuple(face))
    if not points or not faces or any(not math.isfinite(x) for p in points for x in p): raise ValueError('missing/nonfinite geometry')
    if any(i<0 or i>=len(points) for t in faces for i in t): raise ValueError('invalid OBJ index')
    # Exact-position welding; records physical topology, not UV seams.
    mapping={}; welded=[]; ids=[]
    for p in points:
        if p not in mapping: mapping[p]=len(welded);welded.append(p)
        ids.append(mapping[p])
    return welded,[tuple(ids[i] for i in t) for t in faces]

def region(p):
    x,y,z=p; side='left' if x<0 else 'right'
    if y>0.56: return 'head'
    if y>0.48: return 'neck'
    if abs(x)>0.24 and y<0.27: return side+('_hand' if y<0. else '_forearm')
    if abs(x)>0.17 and y>0.20: return side+'_upper_arm'
    if y<-0.68:return side+'_foot'
    if y<-0.46:return side+'_shin'
    if y<-0.16:return side+'_thigh'
    if y<0.02:return 'pelvis'
    if y<0.24:return 'abdomen'
    return 'chest'

def audit(points,faces):
    edges=collections.defaultdict(list); duplicate=0;seen=set();bad=[];area=0.;max_edge=0.
    for i,t in enumerate(faces):
        key=tuple(sorted(t))
        if key in seen:duplicate+=1
        seen.add(key)
        a,b,c=[points[v] for v in t]; measure=length(cross(sub(b,a),sub(c,a)))/2
        if measure<=1e-14:bad.append(i)
        area+=measure
        for a,b in [(t[0],t[1]),(t[1],t[2]),(t[2],t[0])]:
            edges[tuple(sorted((a,b)))].append((i,a<b));max_edge=max(max_edge,length(sub(points[a],points[b])))
    boundaries=[e for e,v in edges.items() if len(v)==1]
    nonmanifold=[e for e,v in edges.items() if len(v)>2]
    winding=[e for e,v in edges.items() if len(v)==2 and v[0][1]==v[1][1]]
    return dict(vertices=len(points),triangles=len(faces),area_m2=area,max_edge_m=max_edge,
                degenerate_triangles=bad,duplicate_triangles=duplicate,boundary_edges=len(boundaries),
                nonmanifold_edges=len(nonmanifold),inconsistent_winding_edges=len(winding))

def split_edges(points,faces,marked,budget):
    """Conforming midpoint/centre refinement with unchanged piecewise surface."""
    requested=set()
    for edge in marked:
        if len(edge)!=2 or any(not isinstance(i,int) or i<0 or i>=len(points) for i in edge) or edge[0]==edge[1]:
            raise ValueError('invalid refinement edge')
        requested.add(tuple(sorted(edge)))
    found={tuple(sorted((t[k],t[(k+1)%3]))) for t in faces for k in range(3)}&requested
    if found!=requested:raise ValueError('refinement edge not in mesh')
    points=list(points);marked={edge:None for edge in sorted(requested)}
    required=sum(1 if not any(tuple(sorted((t[k],t[(k+1)%3]))) in marked for k in range(3)) else 3+sum(tuple(sorted((t[k],t[(k+1)%3]))) in marked for k in range(3)) for t in faces)
    if required>budget:raise ValueError(f'refinement requires {required} triangles, budget {budget}')
    for (a,b) in marked:marked[(a,b)]=len(points);points.append(tuple((x+y)/2 for x,y in zip(points[a],points[b])))
    refined=[]
    for t in faces:
        polygon=[];split=False
        for k,a in enumerate(t):
            b=t[(k+1)%3];polygon.append(a);edge=tuple(sorted((a,b)))
            if edge in marked:polygon.append(marked[edge]);split=True
        if not split:refined.append(t);continue
        center=len(points);points.append(tuple(sum(points[i][k] for i in t)/3 for k in range(3)))
        refined.extend((center,a,polygon[(k+1)%len(polygon)]) for k,a in enumerate(polygon))
    return points,refined

def refine(points,faces,max_edge,budget):
    """Global edge splitting and per-face centre fans: conforming, linear surface only."""
    if max_edge<=0 or not math.isfinite(max_edge):raise ValueError('invalid target edge length')
    points=list(points);faces=list(faces)
    for _ in range(12):
        marked={}
        for t in faces:
            for a,b in [(t[0],t[1]),(t[1],t[2]),(t[2],t[0])]:
                key=tuple(sorted((a,b)))
                if length(sub(points[a],points[b]))>max_edge:marked[key]=None
        if not marked:return points,faces
        points,faces=split_edges(points,faces,marked,budget)
    raise ValueError('refinement did not converge within 12 levels')

def segment_triangle(p,q,triangle,eps=1e-10):
    a,b,c=triangle; direction=sub(q,p);ab=sub(b,a);ac=sub(c,a)
    h=cross(direction,ac);det=dot(ab,h)
    # Determinant has units of length cubed. An absolute tolerance misses
    # real crossings after refinement or changes of mesh units.
    scale=length(direction)*length(ab)*length(ac)
    if scale==0 or abs(det)<=eps*scale:return False
    inv=1/det;s=sub(p,a);u=inv*dot(s,h)
    if u<-eps or u>1+eps:return False
    r=cross(s,ab);v=inv*dot(direction,r);t=inv*dot(ac,r)
    return v>=-eps and u+v<=1+eps and -eps<=t<=1+eps

def crossing(a,b):
    # Noncoplanar triangle intersections. Coplanar tests use projected polygons.
    n=cross(sub(a[1],a[0]),sub(a[2],a[0]));m=cross(sub(b[1],b[0]),sub(b[2],b[0]))
    if length(n)==0 or length(m)==0:return False
    edge_scale=max(length(sub(t[i],t[(i+1)%3])) for t in (a,b) for i in range(3))
    coplanar=length(cross(n,m))<=1e-10*length(n)*length(m) and max(abs(dot(n,sub(p,a[0]))) for p in b)<=1e-10*edge_scale*length(n)
    if not coplanar:return any(segment_triangle(t[k],t[(k+1)%3],other) for t,other in [(a,b),(b,a)] for k in range(3))
    axis=max(range(3),key=lambda k:abs(n[k]));indices=[k for k in range(3) if k!=axis]
    a=[tuple(p[k] for k in indices) for p in a];b=[tuple(p[k] for k in indices) for p in b]
    def orient(p,q,r):return (q[0]-p[0])*(r[1]-p[1])-(q[1]-p[1])*(r[0]-p[0])
    def inside(p,t):
        signs=[orient(t[k],t[(k+1)%3],p) for k in range(3)]
        tolerance=1e-10*edge_scale*edge_scale
        return min(signs)>=-tolerance or max(signs)<=tolerance
    if any(inside(p,b) for p in a) or any(inside(p,a) for p in b):return True
    return any(orient(p,q,r)*orient(p,q,s)<0 and orient(r,s,p)*orient(r,s,q)<0 for p,q in [(a[k],a[(k+1)%3]) for k in range(3)] for r,s in [(b[k],b[(k+1)%3]) for k in range(3)])

def intersections(points,faces,limit=100):
    """BVH broadphase; shared-vertex pairs excluded. Static diagnostic, not CCD."""
    triangles=[[points[i] for i in t] for t in faces]
    bounds=[([min(p[k] for p in t) for k in range(3)],[max(p[k] for p in t) for k in range(3)]) for t in triangles]
    def tree(ids):
        lo=[min(bounds[i][0][k] for i in ids) for k in range(3)];hi=[max(bounds[i][1][k] for i in ids) for k in range(3)]
        if len(ids)<=8:return (lo,hi,ids,None)
        axis=max(range(3),key=lambda k:hi[k]-lo[k]);ids.sort(key=lambda i:bounds[i][0][axis]+bounds[i][1][axis]);mid=len(ids)//2
        return (lo,hi,tree(ids[:mid]),tree(ids[mid:]))
    root=tree(list(range(len(faces))));hits=[];tested=0
    def overlap(a,b):return all(a[0][k]<=b[1][k]+1e-12 and b[0][k]<=a[1][k]+1e-12 for k in range(3))
    def visit(a,b,same=False):
        nonlocal tested
        if len(hits)>=limit or not overlap(a,b):return
        if a[3] is None and b[3] is None:
            for i in a[2]:
                for j in b[2]:
                    if (same and i>=j) or i==j or set(faces[i])&set(faces[j]) or not overlap(bounds[i],bounds[j]):continue
                    tested+=1
                    if crossing(triangles[i],triangles[j]):
                        hits.append(sorted([i,j]))
                        if len(hits)>=limit:return
            return
        if same:
            visit(a[2],a[2],True);visit(a[2],a[3]);visit(a[3],a[3],True)
        elif a[3] is None:visit(a,b[2]);visit(a,b[3])
        elif b[3] is None:visit(a[2],b);visit(a[3],b)
        else:
            for x in a[2:]:
                for y in b[2:]:visit(x,y)
    visit(root,root,True)
    return dict(intersection_pairs=hits,intersection_limit_reached=len(hits)>=limit,narrowphase_pairs=tested,intersection_scope='static nonadjacent faces; pairs sharing any vertex excluded')

def intersection_segment_length(a,b):
    """Nonparallel triangle/plane interval overlap in metres; None for coplanar."""
    n=cross(sub(a[1],a[0]),sub(a[2],a[0]));m=cross(sub(b[1],b[0]),sub(b[2],b[0]))
    direction=cross(n,m);size=length(direction)
    if size<=1e-10*length(n)*length(m):return None
    axis=tuple(x/size for x in direction)
    scale=max(length(sub(t[k],t[(k+1)%3])) for t in (a,b) for k in range(3));tol=scale*1e-10
    def interval(triangle,origin,normal):
        distances=[dot(normal,sub(p,origin))/length(normal) for p in triangle];values=[]
        for k,p in enumerate(triangle):
            q=triangle[(k+1)%3];d,e=distances[k],distances[(k+1)%3]
            if abs(d)<=tol:values.append(dot(sub(p,a[0]),axis))
            if d*e<0:
                t=d/(d-e);point=tuple(p[j]+t*(q[j]-p[j]) for j in range(3));values.append(dot(sub(point,a[0]),axis))
        return (min(values),max(values)) if values else None
    ia,ib=interval(a,b[0],m),interval(b,a[0],n)
    if ia is None or ib is None:return 0.
    return max(0.,min(ia[1],ib[1])-max(ia[0],ib[0]))

def repair_intersections(points,faces,iterations=12,clearance=0.0002,max_displacement=0.005,smoothing_steps=3,reference_points=None,movable_vertices=None,reduce_segment_length=False,separation_mode='face'):
    """Bounded candidate separation. Retains topology; does not certify anatomy.
    Publishes topology-valid candidates with fewer detected crossings;
    optional equal-count descent strictly reduces noncoplanar segment length.
    """
    if iterations<1 or not math.isfinite(clearance) or not math.isfinite(max_displacement) or clearance<=0 or max_displacement<=0:raise ValueError('invalid repair limits')
    if smoothing_steps<0:raise ValueError('invalid smoothing steps')
    if separation_mode not in ('face','penetrating_vertices'):raise ValueError('invalid separation mode')
    original=list(points if reference_points is None else reference_points);orientation_reference=list(points);current=list(points);history=[];rejections=[]
    if len(original)!=len(points) or any(not math.isfinite(x) for p in original for x in p):
        raise ValueError('invalid displacement reference')
    if any(length(sub(p,q))>max_displacement*(1+1e-12) for p,q in zip(points,original)):
        raise ValueError('input already exceeds displacement bound')
    if movable_vertices is not None:
        movable_vertices=set(movable_vertices)
        if any(not isinstance(i,int) or i<0 or i>=len(points) for i in movable_vertices):
            raise ValueError('invalid movable vertex')
    neighbors=collections.defaultdict(set)
    for face in faces:
        for vertex in face:neighbors[vertex].update(v for v in face if v!=vertex)
    report=intersections(current,faces,limit=1000)
    if report['intersection_limit_reached']:
        raise ValueError('repair requires an uncapped initial crossing count')
    def segment_score(positions,pairs):
        values=[intersection_segment_length([positions[i] for i in faces[a]],[positions[i] for i in faces[b]]) for a,b in pairs]
        return None if any(value is None for value in values) else sum(values)
    for _ in range(iterations):
        hits=report['intersection_pairs']
        if not hits:break
        score=segment_score(current,hits) if reduce_segment_length else None
        normals=[]
        for t in faces:
            a,b,c=[current[i] for i in t];n=cross(sub(b,a),sub(c,a));mag=length(n)
            normals.append(tuple(x/mag for x in n) if mag>0 else (0.,0.,0.))
        shifts=collections.defaultdict(lambda:[0.,0.,0.,0])
        for a,b in hits:
            ca=tuple(sum(current[i][k] for i in faces[a])/3 for k in range(3));cb=tuple(sum(current[i][k] for i in faces[b])/3 for k in range(3))
            for face,other,center,other_center in [(a,b,ca,cb),(b,a,cb,ca)]:
                n=normals[other];sign=1. if dot(n,sub(center,other_center))>=0 else -1.
                distances=[sign*dot(n,sub(current[i],other_center)) for i in faces[face]]
                amount=max(clearance,clearance-min(distances))
                for vertex,distance in zip(faces[face],distances):
                    vertex_amount=amount if separation_mode=='face' else max(0.,clearance-distance)
                    if vertex_amount==0.:continue
                    for k in range(3):shifts[vertex][k]+=sign*n[k]*vertex_amount
                    shifts[vertex][3]+=1
        # Smooth the displacement field, not positions: untouched geometry
        # remains unchanged, and contact motion spreads over local supports.
        field={vertex:tuple(delta[k]/delta[3] for k in range(3)) for vertex,delta in shifts.items()}
        for _ in range(smoothing_steps):
            active=set(field)
            for vertex in field:active.update(neighbors[vertex])
            field={vertex:tuple(0.5*field.get(vertex,(0.,0.,0.))[k]
                +0.5*sum(field.get(other,(0.,0.,0.))[k] for other in neighbors[vertex])/len(neighbors[vertex])
                for k in range(3)) for vertex in active}
        if movable_vertices is not None:field={i:d for i,d in field.items() if i in movable_vertices}
        accepted=False
        for fraction in ([1.,0.5,0.25,0.125,0.0625] if reduce_segment_length else [1.,0.5,0.25]):
            candidate=list(current)
            for vertex,delta in field.items():
                position=tuple(current[vertex][k]+fraction*delta[k] for k in range(3))
                shift=sub(position,original[vertex]);distance=length(shift)
                if distance>max_displacement:position=tuple(original[vertex][k]+shift[k]*max_displacement/distance for k in range(3))
                candidate[vertex]=position
            # A skinny face in one region must not invalidate a safe proposal
            # elsewhere. Freeze its incident vertices, then recheck neighbors.
            # The final global winding/count gates still apply.
            frozen=set()
            for _ in range(8):
                inverted=[t for t in faces if dot(cross(sub(candidate[t[1]],candidate[t[0]]),sub(candidate[t[2]],candidate[t[0]])),cross(sub(orientation_reference[t[1]],orientation_reference[t[0]]),sub(orientation_reference[t[2]],orientation_reference[t[0]])))<=0]
                if not inverted:break
                for face in inverted:
                    for vertex in face:
                        frozen.add(vertex);candidate[vertex]=current[vertex]
            topology=audit(candidate,faces)
            if topology['degenerate_triangles']:
                rejections.append({'fraction':fraction,'reason':'degenerate triangles'});continue
            # Reject inverted local faces relative to the input winding.
            if any(dot(cross(sub(candidate[t[1]],candidate[t[0]]),sub(candidate[t[2]],candidate[t[0]])),cross(sub(orientation_reference[t[1]],orientation_reference[t[0]]),sub(orientation_reference[t[2]],orientation_reference[t[0]])))<=0 for t in faces):
                rejections.append({'fraction':fraction,'reason':'face normal reversal'});continue
            after=intersections(candidate,faces,limit=1000)
            next_score=segment_score(candidate,after['intersection_pairs']) if reduce_segment_length else None
            shorter=score is not None and next_score is not None and next_score<score-max(1e-12,score*1e-8)
            if len(after['intersection_pairs'])<len(hits) or (len(after['intersection_pairs'])==len(hits) and shorter):
                history.append({'before':len(hits),'after':len(after['intersection_pairs']),'beforeSegmentLengthM':score,'afterSegmentLengthM':next_score,'fraction':fraction,'frozen_vertices':len(frozen)})
                current=candidate;report=after;accepted=True;break
            rejections.append({'fraction':fraction,'reason':'crossing count/segment score did not improve' if reduce_segment_length else 'crossing count did not decrease','crossings':len(after['intersection_pairs']),'beforeSegmentLengthM':score,'afterSegmentLengthM':next_score})
        if not accepted:break
    return current,dict(repair_steps=history,repair_rejections=rejections,repair_separation_mode=separation_mode,repair_smoothing_steps=smoothing_steps,repair_max_displacement_m=max((length(sub(a,b)) for a,b in zip(original,current)),default=0.),repair_complete=not report['intersection_pairs'],repair_remaining=report)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('source',type=Path);parser.add_argument('output',type=Path)
    parser.add_argument('--max-edge-mm',type=float);parser.add_argument('--triangle-budget',type=int,default=1000000);parser.add_argument('--intersections',action='store_true');parser.add_argument('--repair',action='store_true')
    args=parser.parse_args();points,faces=load(args.source);before=audit(points,faces)
    if args.max_edge_mm:points,faces=refine(points,faces,args.max_edge_mm/1000,args.triangle_budget)
    repair={}
    if args.repair:points,repair=repair_intersections(points,faces)
    report=audit(points,faces);report.update(repair)
    if args.intersections:report.update(intersections(points,faces))
    labels=collections.defaultdict(list)
    for i,t in enumerate(faces):labels[region(tuple(sum(points[j][k] for j in t)/3 for k in range(3)))].append(i)
    report.update(source_sha256=hashlib.sha256(args.source.read_bytes()).hexdigest(),source_audit=before,
        region_assignment='heuristic rest-space coordinates, requires anatomical review',regions=dict(labels),internal_surfaces=[],
        missing_internal_anatomy=True,refinement='linear conforming subdivision; no new anatomical detail',uv_preserved=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    # Explicit new geometry output: source texture coordinates/normals are not copied.
    with args.output.open('x') as out:
        out.write('# Prepared geometry-only body; see adjacent audit JSON\n')
        # Round-trip double precision; audited coordinates must survive export.
        for p in points:out.write('v '+' '.join(format(x,'.17g') for x in p)+'\n')
        for t in faces:out.write('f '+' '.join(str(i+1) for i in t)+'\n')
    args.output.with_suffix('.audit.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ('regions','source_audit')},indent=2))
if __name__=='__main__':main()

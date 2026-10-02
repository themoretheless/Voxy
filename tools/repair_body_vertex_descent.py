#!/usr/bin/env python3
"""Single-vertex candidate descent with global final crossing verification."""
import argparse, collections, hashlib, itertools, json, math
from pathlib import Path
import prepare_body_geometry as g

def lattice_directions():
    """Thirteen unit directions; signed distances cover all 26 neighbours."""
    directions=[]
    for v in itertools.product((-1.,0.,1.),repeat=3):
        first=next((x for x in v if x),0.)
        if first<=0:continue
        size=g.length(v)
        directions.append(tuple(x/size for x in v))
    return directions

def fairing_direction(points,neighbours,group):
    delta=[0.,0.,0.]
    for v in group:
        adjacent=sorted(neighbours[v])
        if not adjacent:continue
        for k in range(3):delta[k]+=math.fsum(points[w][k]-points[v][k] for w in adjacent)/len(adjacent)
    size=g.length(delta)
    return tuple(x/size for x in delta) if size>1e-15 else None

def intersection_measure(a,b):
    segment=g.intersection_segment_length(a,b)
    if segment is not None:return segment
    from audit_body_adjacent_faces import overlap_area
    n=g.cross(g.sub(a[1],a[0]),g.sub(a[2],a[0]))
    axis=max(range(3),key=lambda k:abs(n[k]));axes=[k for k in range(3) if k!=axis]
    project=lambda tri:[tuple(p[k] for k in axes) for p in tri]
    return math.sqrt(overlap_area(project(a),project(b))*g.length(n)/abs(n[axis]))

def triangle_quality(tri):
    normal=g.cross(g.sub(tri[1],tri[0]),g.sub(tri[2],tri[0]))
    denominator=math.fsum(g.dot(g.sub(tri[(k+1)%3],tri[k]),g.sub(tri[(k+1)%3],tri[k])) for k in range(3))
    return 2*math.sqrt(3)*g.length(normal)/denominator if denominator else 0.

def fold_angle(a,b):
    n=g.cross(g.sub(a[1],a[0]),g.sub(a[2],a[0]));m=g.cross(g.sub(b[1],b[0]),g.sub(b[2],b[0]))
    return math.atan2(g.length(g.cross(n,m)),g.dot(n,m))

def repair(points, faces, reference, passes=4, bound=.003,distances=(.0002,-.0002,.0005,-.0005,.001,-.001),include_adjacent=False,face_groups=False,edge_groups=False,project_bound=False,reduce_measure=False,diagonal_directions=False,ring_groups=False,fairing=False,quality_floor=0.,repair_slivers=False,fold_threshold_degrees=0.,fold_bounds=None):
    if not math.isfinite(quality_floor) or not 0<=quality_floor<=1:raise ValueError('quality floor must be within 0..1')
    if repair_slivers and (not quality_floor or not reduce_measure or not include_adjacent):raise ValueError('sliver repair requires quality floor and complete measure-mode audit')
    if not math.isfinite(fold_threshold_degrees) or not 0<=fold_threshold_degrees<180:raise ValueError('fold threshold must be within [0,180)')
    if fold_threshold_degrees and (not quality_floor or not reduce_measure or not include_adjacent):raise ValueError('fold repair requires quality floor and complete measure-mode audit')
    if fold_bounds is not None and (len(fold_bounds)!=6 or not all(math.isfinite(x) for x in fold_bounds) or any(fold_bounds[k]>fold_bounds[k+1] for k in (0,2,4))):raise ValueError('invalid fold region')
    if sum((face_groups,edge_groups,ring_groups))>1:raise ValueError('choose one group mode')
    if len(points)!=len(reference) or passes<1 or not math.isfinite(bound) or bound<=0:
        raise ValueError('invalid reference or search limits')
    if not points or any(len(p)!=3 or any(not math.isfinite(x) for x in p) for mesh in (points,reference) for p in mesh):
        raise ValueError('source and reference require finite 3D coordinates')
    if not faces or any(len(t)!=3 or len(set(t))!=3 or any(not isinstance(v,int) or v<0 or v>=len(points) for v in t) for t in faces):
        raise ValueError('invalid triangle indices')
    if any(g.length(g.sub(p,q))>bound for p,q in zip(points,reference)):
        raise ValueError('input exceeds cumulative displacement bound')
    if not distances or any(not math.isfinite(d) or d==0 for d in distances):
        raise ValueError('invalid proposal distances')
    points=list(points)
    triangles=[[points[v] for v in t] for t in faces]
    def bounds(tri):return tuple((min(p[k] for p in tri),max(p[k] for p in tri)) for k in range(3))
    boxes=[bounds(t) for t in triangles]
    incidence=collections.defaultdict(set); buckets=collections.defaultdict(set);neighbours=collections.defaultdict(set)
    def keys(tri):
        return list(itertools.product(*(range(math.floor(min(p[k] for p in tri)/.012),math.floor(max(p[k] for p in tri)/.012)+1) for k in range(3))))
    def normal(tri): return g.cross(g.sub(tri[1],tri[0]),g.sub(tri[2],tri[0]))
    normals=[normal(t) for t in triangles]
    qualities=[triangle_quality(t) for t in triangles] if quality_floor else None
    initial_qualities=list(qualities) if qualities is not None else None
    for i,t in enumerate(faces):
        for v in t:
            incidence[v].add(i);neighbours[v].update(w for w in t if w!=v)
        for key in keys(triangles[i]): buckets[key].add(i)
    fold_pairs=[];fold_incidence=collections.defaultdict(set)
    threshold=math.radians(fold_threshold_degrees)
    if fold_threshold_degrees:
        edge_faces=collections.defaultdict(list)
        for i,t in enumerate(faces):
            for k in range(3):edge_faces[tuple(sorted((t[k],t[(k+1)%3])))].append(i)
        for edge,ids in sorted(edge_faces.items()):
            if len(ids)!=2:continue
            center=tuple((points[edge[0]][k]+points[edge[1]][k])/2 for k in range(3))
            if fold_bounds is not None and not all(fold_bounds[2*k]<=center[k]<=fold_bounds[2*k+1] for k in range(3)):continue
            idx=len(fold_pairs);fold_pairs.append(tuple(ids))
            for i in ids:fold_incidence[i].add(idx)
    def fold_deficit(ids,replacement=None):
        replacement=replacement or {}
        return math.fsum(max(0.,fold_angle(replacement.get(a,triangles[a]),replacement.get(b,triangles[b]))-threshold)**2 for idx in sorted(ids) for a,b in [fold_pairs[idx]])
    def maximum_fold(ids,replacement=None):
        replacement=replacement or {}
        return max((fold_angle(replacement.get(a,triangles[a]),replacement.get(b,triangles[b])) for idx in ids for a,b in [fold_pairs[idx]]),default=0.)
    initial_fold_deficit=fold_deficit(range(len(fold_pairs))) if fold_threshold_degrees else None
    initial_maximum_fold=maximum_fold(range(len(fold_pairs))) if fold_threshold_degrees else None
    initial=g.intersections(points,faces,10000)
    if initial['intersection_limit_reached']: raise ValueError('uncapped audit required')
    hits={tuple(p) for p in initial['intersection_pairs']}; history=[]
    if include_adjacent:
        from audit_body_adjacent_faces import audit as adjacent_audit,forbidden
        hits.update(tuple(p) for p in adjacent_audit(points,faces)['forbidden_pairs'])
    initial_count=len(hits)
    measure=lambda pairs,replacement={}:math.fsum(intersection_measure(replacement.get(a,triangles[a]),replacement.get(b,triangles[b])) for a,b in sorted(pairs))
    initial_measure=measure(hits) if reduce_measure else None
    for iteration in range(passes):
        changed=False
        active_faces={i for pair in hits for i in pair}
        if repair_slivers:active_faces.update(i for i,q in enumerate(qualities) if q<quality_floor)
        if fold_threshold_degrees:active_faces.update(i for a,b in fold_pairs if fold_angle(triangles[a],triangles[b])>threshold for i in (a,b))
        groups=sorted({tuple(sorted(faces[i])) for i in active_faces}) if face_groups else [(v,) for v in sorted({v for i in active_faces for v in faces[i]})]
        if edge_groups:
            groups=sorted({tuple(sorted((faces[i][k],faces[i][(k+1)%3]))) for i in active_faces for k in range(3)})
        if ring_groups:
            seeds={v for i in active_faces for v in faces[i]}
            groups=sorted({tuple(sorted({w for i in incidence[v] for w in faces[i]})) for v in seeds},key=lambda group:(-len(group),group))
        for group in groups:
            affected=set().union(*(incidence[v] for v in group))
            oldhits={p for p in hits if affected.intersection(p)}
            if not oldhits and not (repair_slivers or fold_threshold_degrees): continue
            old_measure=measure(oldhits) if reduce_measure else None
            old_deficit=math.fsum(max(0.,quality_floor-qualities[i])**2 for i in affected) if repair_slivers else None
            affected_folds=set().union(*(fold_incidence[i] for i in affected)) if fold_threshold_degrees else set()
            old_fold_deficit=fold_deficit(affected_folds) if fold_threshold_degrees else None
            old_maximum_fold=maximum_fold(affected_folds) if fold_threshold_degrees else None
            directions=[]
            if fairing:
                direction=fairing_direction(points,neighbours,group)
                if direction is not None:directions.append(direction)
            for a,b in sorted(oldhits):
                for face,other in ((a,b),(b,a)):
                    if not set(group).intersection(faces[face]): continue
                    n=normal(triangles[other]); length=g.length(n)
                    if length: directions.append(tuple(x/length for x in n))
            directions.extend([(1.,0.,0.),(0.,1.,0.),(0.,0.,1.)])
            if diagonal_directions:
                directions.extend(n for n in lattice_directions() if sum(x!=0 for x in n)>1)
            accepted=False
            # Several intersecting faces can propose the exact same normal.
            # Preserve proposal order while avoiding identical expensive audits.
            for n in dict.fromkeys(directions):
                if accepted: break
                for distance in distances:
                    proposed={v:tuple(points[v][k]+n[k]*distance for k in range(3)) for v in group}
                    outside=any(g.length(g.sub(p,reference[v]))>bound for v,p in proposed.items())
                    if outside and not project_bound:continue
                    if outside:
                        for v,p in proposed.items():
                            delta=g.sub(p,reference[v]);size=g.length(delta)
                            if size>bound:proposed[v]=tuple(reference[v][k]+delta[k]*(bound*(1-1e-12)/size) for k in range(3))
                    newtri={i:[proposed.get(v,points[v]) for v in faces[i]] for i in affected}
                    newboxes={i:bounds(t) for i,t in newtri.items()}
                    if any(g.length(normal(t))<=2e-14 or g.dot(normal(t),normals[i])<=0 for i,t in newtri.items()): continue
                    newqualities={i:triangle_quality(t) for i,t in newtri.items()} if quality_floor else None
                    if newqualities is not None and any(q+1e-12<min(quality_floor,qualities[i]) for i,q in newqualities.items()):continue
                    newhits=set()
                    checked_pairs=set()
                    for i,t in newtri.items():
                        nearby=set(affected)
                        for key in keys(t): nearby.update(buckets.get(key,()))
                        for j in nearby:
                            if i==j:continue
                            box=newboxes.get(j,boxes[j])
                            if any(newboxes[i][k][0]>box[k][1]+1e-12 or box[k][0]>newboxes[i][k][1]+1e-12 for k in range(3)):continue
                            pair=tuple(sorted((i,j)))
                            if pair in checked_pairs:continue
                            checked_pairs.add(pair)
                            shared=len(set(faces[i]).intersection(faces[j]))
                            first=newtri.get(pair[0],triangles[pair[0]])
                            other=newtri.get(pair[1],triangles[pair[1]])
                            if shared:
                                if not include_adjacent:continue
                                crosses=forbidden(first,other,shared)
                            else:crosses=g.crossing(first,other)
                            if crosses:newhits.add(pair)
                    new_measure=measure(newhits,newtri) if reduce_measure else None
                    if reduce_measure and not newhits.issubset(oldhits):continue
                    shorter=reduce_measure and len(newhits)==len(oldhits) and new_measure<old_measure-max(1e-12,old_measure*1e-8)
                    new_deficit=math.fsum(max(0.,quality_floor-q)**2 for q in newqualities.values()) if repair_slivers else None
                    improved_shape=repair_slivers and len(newhits)==len(oldhits) and new_deficit<old_deficit-max(1e-14,old_deficit*1e-8)
                    new_fold_deficit=fold_deficit(affected_folds,newtri) if fold_threshold_degrees else None
                    if fold_threshold_degrees and new_fold_deficit>old_fold_deficit+1e-12:continue
                    if fold_threshold_degrees and maximum_fold(affected_folds,newtri)>old_maximum_fold+1e-12:continue
                    improved_fold=fold_threshold_degrees and len(newhits)==len(oldhits) and new_fold_deficit<old_fold_deficit-max(1e-12,old_fold_deficit*1e-8)
                    if len(newhits)>=len(oldhits) and not (shorter or improved_shape or improved_fold): continue
                    for i in affected:
                        for key in keys(triangles[i]): buckets[key].discard(i)
                    for v,p in proposed.items():points[v]=p
                    for i,t in newtri.items():
                        triangles[i]=t;boxes[i]=newboxes[i]
                        if newqualities is not None:qualities[i]=newqualities[i]
                        for key in keys(t): buckets[key].add(i)
                    before=len(hits);hits.difference_update(oldhits);hits.update(newhits)
                    history.append(dict(pass_index=iteration,vertices=group,before=before,after=len(hits),local_measure_before_m=old_measure,local_measure_after_m=new_measure,local_quality_deficit_before=old_deficit,local_quality_deficit_after=new_deficit,local_fold_deficit_before=old_fold_deficit,local_fold_deficit_after=new_fold_deficit))
                    changed=accepted=True;break
        if not changed: break
    final=g.intersections(points,faces,10000)
    final_pairs={tuple(p) for p in final['intersection_pairs']}
    if include_adjacent:
        final['adjacent']=adjacent_audit(points,faces)
        final_pairs.update(tuple(p) for p in final['adjacent']['forbidden_pairs'])
    if final['intersection_limit_reached'] or final_pairs!=hits: raise ValueError('incremental/full mismatch')
    quality_audit=None
    if quality_floor:
        full=[triangle_quality([points[v] for v in t]) for t in faces]
        if any(abs(a-b)>1e-12 for a,b in zip(full,qualities)) or any(q+1e-10<min(quality_floor,start) for q,start in zip(full,initial_qualities)):raise ValueError('quality verification failed')
        quality_audit=dict(floor=quality_floor,initial_minimum=min(initial_qualities),final_minimum=min(full),initial_below_floor=sum(q<quality_floor for q in initial_qualities),final_below_floor=sum(q<quality_floor for q in full),verified=True)
    final_fold_deficit=fold_deficit(range(len(fold_pairs))) if fold_threshold_degrees else None
    if fold_threshold_degrees and final_fold_deficit>initial_fold_deficit+1e-10:raise ValueError('fold objective verification failed')
    if fold_threshold_degrees and maximum_fold(range(len(fold_pairs)))>initial_maximum_fold+1e-10:raise ValueError('maximum fold verification failed')
    return points,dict(before=initial_count,after=len(hits),steps=history,includes_adjacent=include_adjacent,face_groups=face_groups,edge_groups=edge_groups,ring_groups=ring_groups,fairing=fairing,repair_slivers=repair_slivers,quality_audit=quality_audit,fold_audit=dict(threshold_degrees=fold_threshold_degrees,bounds_m=fold_bounds,selected_edges=len(fold_pairs),initial_deficit=initial_fold_deficit,final_deficit=final_fold_deficit,initial_maximum_angle_degrees=math.degrees(initial_maximum_fold),final_maximum_angle_degrees=math.degrees(maximum_fold(range(len(fold_pairs))))) if fold_threshold_degrees else None,project_bound=project_bound,reduce_measure=reduce_measure,diagonal_directions=diagonal_directions,initial_measure_m=initial_measure,final_measure_m=measure(hits) if reduce_measure else None,
        remaining=final,maximum_cumulative_displacement_m=max(g.length(g.sub(p,q)) for p,q in zip(points,reference)),audit=g.audit(points,faces))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path);parser.add_argument('reference',type=Path)
    parser.add_argument('output',type=Path);parser.add_argument('--report',type=Path,required=True)
    parser.add_argument('--passes',type=int,default=4)
    parser.add_argument('--bound-mm',type=float,default=3.)
    parser.add_argument('--fine',action='store_true')
    parser.add_argument('--adjacent',action='store_true')
    parser.add_argument('--face-groups',action='store_true')
    parser.add_argument('--edge-groups',action='store_true')
    parser.add_argument('--ring-groups',action='store_true')
    parser.add_argument('--project-bound',action='store_true')
    parser.add_argument('--reduce-measure',action='store_true')
    parser.add_argument('--largest-first',action='store_true')
    parser.add_argument('--diagonal-directions',action='store_true')
    parser.add_argument('--fairing',action='store_true')
    parser.add_argument('--quality-floor',type=float,default=0.)
    parser.add_argument('--repair-slivers',action='store_true')
    parser.add_argument('--fold-threshold-degrees',type=float,default=0.)
    parser.add_argument('--fold-bounds',nargs=6,type=float)
    args=parser.parse_args();source=args.source;out=args.output
    if out.exists():parser.error('output already exists')
    points,faces=g.load(source);reference,_=g.load(args.reference)
    distances=tuple(sign*d for d in ((.000025,.00005,.0001,.0003,.0015,.002) if args.fine else (.0002,.0005,.001)) for sign in (1,-1))
    if args.largest_first:distances=tuple(sorted(distances,key=lambda d:-abs(d)))
    fixed,report=repair(points,faces,reference,passes=args.passes,bound=args.bound_mm/1000.,distances=distances,include_adjacent=args.adjacent,face_groups=args.face_groups,edge_groups=args.edge_groups,project_bound=args.project_bound,reduce_measure=args.reduce_measure,diagonal_directions=args.diagonal_directions,ring_groups=args.ring_groups,fairing=args.fairing,quality_floor=args.quality_floor,repair_slivers=args.repair_slivers,fold_threshold_degrees=args.fold_threshold_degrees,fold_bounds=args.fold_bounds)
    with out.open('x') as stream:
        for p in fixed:stream.write('v '+' '.join(format(x,'.17g') for x in p)+'\n')
        for t in faces:stream.write('f '+' '.join(str(i+1) for i in t)+'\n')
    loaded,loaded_faces=g.load(out)
    assert loaded==fixed and loaded_faces==faces
    report.update(source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),candidate_sha256=hashlib.sha256(out.read_bytes()).hexdigest(),proposal_distances_m=distances,adopted=False,export_reload_verified=True)
    args.report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ('steps','remaining')}))

//! Native exact feature queries choose axes only; polynomial inequalities own proof.
use super::*;
use num_traits::ToPrimitive;
type RV=[R;3];
fn add(a:&RV,b:&RV)->RV {std::array::from_fn(|i|&a[i]+&b[i])}
fn sub(a:&RV,b:&RV)->RV {std::array::from_fn(|i|&a[i]-&b[i])}
fn scale(a:&RV,t:&R)->RV {std::array::from_fn(|i|&a[i]*t)}
fn dot(a:&RV,b:&RV)->R {(0..3).fold(zero_r(),|s,i|s+&a[i]*&b[i])}
fn cross(a:&RV,b:&RV)->RV {[
    &a[1]*&b[2]-&a[2]*&b[1],&a[2]*&b[0]-&a[0]*&b[2],&a[0]*&b[1]-&a[1]*&b[0],
]}
fn one()->R {R::from_integer(1.into())}
#[cfg(test)]
fn point_segment(p:&RV,a:&RV,b:&RV)->RV {
    let e=sub(b,a);let den=dot(&e,&e);
    let t=if den>zero_r() {(dot(&sub(p,a),&e)/den).max(zero_r()).min(one())} else {zero_r()};
    add(a,&scale(&e,&t))
}
#[cfg(test)]
fn inside(p:&RV,a:&RV,b:&RV,c:&RV)->bool {
    let n=cross(&sub(b,a),&sub(c,a));
    [(a,b),(b,c),(c,a)].iter().all(|(u,v)|dot(&cross(&sub(v,u),&sub(p,u)),&n)>=zero_r())
}
fn distance2(a:&RV,b:&RV)->R {let d=sub(a,b);dot(&d,&d)}
#[cfg(test)]
fn point_triangle(p:&RV,a:&RV,b:&RV,c:&RV)->RV {
    let n=cross(&sub(b,a),&sub(c,a));let den=dot(&n,&n);
    if den>zero_r() {
        let q=sub(p,&scale(&n,&(dot(&sub(p,a),&n)/den)));
        if inside(&q,a,b,c) {return q;}
    }
    [(a,b),(b,c),(c,a)].iter().map(|(u,v)|point_segment(p,u,v)).min_by_key(|q|distance2(p,q)).unwrap()
}
fn point_segment_prepared(p:&RV,a:&RV,edge:&RV,norm2:&R)->RV {
    let t=if *norm2>zero_r() {(dot(&sub(p,a),edge)/norm2).max(zero_r()).min(one())} else {zero_r()};
    add(a,&scale(edge,&t))
}
struct TriangleFeatures<'a> {vertices:[&'a RV;3],edges:[RV;3],edge_norms:[std::cell::OnceCell<R>;3],normal:RV,norm2:std::cell::OnceCell<R>}
impl<'a> TriangleFeatures<'a> {
    fn new(a:&'a RV,b:&'a RV,c:&'a RV)->Self {
        let edges=[sub(b,a),sub(c,b),sub(a,c)];let edge_norms=std::array::from_fn(|_|std::cell::OnceCell::new());
        let normal=cross(&edges[0],&sub(c,a));let norm2=std::cell::OnceCell::new();
        Self {vertices:[a,b,c],edges,edge_norms,normal,norm2}
    }
    fn inside(&self,p:&RV)->bool {
        (0..3).all(|i|dot(&cross(&self.edges[i],&sub(p,self.vertices[i])),&self.normal)>=zero_r())
    }
    fn closest(&self,p:&RV)->RV {
        let norm2=self.norm2.get_or_init(||dot(&self.normal,&self.normal));
        if *norm2>zero_r() {
            let q=sub(p,&scale(&self.normal,&(dot(&sub(p,self.vertices[0]),&self.normal)/norm2)));
            if self.inside(&q) {return q;}
        }
        (0..3).map(|i|point_segment_prepared(p,self.vertices[i],&self.edges[i],self.edge_norms[i].get_or_init(||dot(&self.edges[i],&self.edges[i])))).min_by_key(|q|distance2(p,q)).unwrap()
    }
}
fn segment_pair(p:&RV,q:&RV,a:&RV,b:&RV)->(RV,RV) {
    let u=sub(q,p);let v=sub(b,a);let w=sub(p,a);
    let aa=dot(&u,&u);let bb=dot(&u,&v);let cc=dot(&v,&v);let dd=dot(&u,&w);let ee=dot(&v,&w);
    let den=&aa*&cc-&bb*&bb;
    let mut candidates=vec![(p.clone(),point_segment_prepared(p,a,&v,&cc)),(q.clone(),point_segment_prepared(q,a,&v,&cc)),
        (point_segment_prepared(a,p,&u,&aa),a.clone()),(point_segment_prepared(b,p,&u,&aa),b.clone())];
    if den>zero_r() {
        let s=(&bb*&ee-&cc*&dd)/&den;let t=(&aa*&ee-&bb*&dd)/den;
        if s>=zero_r() && s<=one() && t>=zero_r() && t<=one() {candidates.push((add(p,&scale(&u,&s)),add(a,&scale(&v,&t))));}
    }
    candidates.into_iter().min_by_key(|(a,b)|distance2(a,b)).unwrap()
}
#[cfg(test)]
fn segment_pair_reference(p:&RV,q:&RV,a:&RV,b:&RV)->(RV,RV) {
    let u=sub(q,p);let v=sub(b,a);let w=sub(p,a);
    let aa=dot(&u,&u);let bb=dot(&u,&v);let cc=dot(&v,&v);let dd=dot(&u,&w);let ee=dot(&v,&w);
    let den=&aa*&cc-&bb*&bb;
    let mut candidates=vec![(p.clone(),point_segment(p,a,b)),(q.clone(),point_segment(q,a,b)),
        (point_segment(a,p,q),a.clone()),(point_segment(b,p,q),b.clone())];
    if den>zero_r() {
        let s=(&bb*&ee-&cc*&dd)/&den;let t=(&aa*&ee-&bb*&dd)/den;
        if s>=zero_r() && s<=one() && t>=zero_r() && t<=one() {candidates.push((add(p,&scale(&u,&s)),add(a,&scale(&v,&t))));}
    }
    candidates.into_iter().min_by_key(|(a,b)|distance2(a,b)).unwrap()
}
#[cfg(test)]
fn axis_at(capsule:CapsuleMotion,triangle:TriangleMotion,time:&R,clearance_radius:&R)->Option<RV> {
    let interpolate=|a:V,b:V|std::array::from_fn(|i|real(a[i])+time*(real(b[i])-real(a[i])));
    let [p,q]=std::array::from_fn(|i|interpolate(capsule.start[i],capsule.end[i]));
    let [a,b,c]=std::array::from_fn(|i|interpolate(triangle.start[i],triangle.end[i]));
    axis_at_points_reference(p,q,a,b,c,&(clearance_radius*clearance_radius))
}
// Translation-invariant exact geometry, prepared once for the complete
// bounded search. The moving first triangle vertex is the coordinate origin.
struct RelativeGeometry {first:[RV;5],slope:[RV;5],radius2:R}
impl RelativeGeometry {
    fn new(motion:&ExactMotion)->Self {
        let coordinates=|values:&[[[R;3];3];2]| [values[0][0].clone(),values[1][0].clone(),std::array::from_fn(|_|zero_r()),sub(&values[0][0],&values[0][1]),sub(&values[0][0],&values[0][2])];
        Self {first:coordinates(&motion.first),slope:coordinates(&motion.slope),radius2:&motion.radius*&motion.radius}
    }
    fn points_at(&self,time:&R)->[RV;5] {
        std::array::from_fn(|i|add(&self.first[i],&scale(&self.slope[i],time)))
    }
    fn axis_at(&self,time:&R)->Option<RV> {
        let [p,q,a,b,c]=self.points_at(time);
        axis_at_points(p,q,a,b,c,&self.radius2)
    }
    fn penetrates_at(&self,time:&R)->bool {
        if time<&zero_r() || time>&one() {return false;}
        let [p,q,a,b,c]=self.points_at(time);
        let (first,last)=closest_pair_points(p,q,a,b,c);
        distance2(&first,&last)<self.radius2
    }
}
fn closest_pair_points(p:RV,q:RV,a:RV,b:RV,c:RV)->(RV,RV) {
    let face=TriangleFeatures::new(&a,&b,&c);let n=&face.normal;let den=dot(&sub(&q,&p),n);
    if den!=zero_r() {
        let u=dot(&sub(&a,&p),&n)/den;
        let point=add(&p,&scale(&sub(&q,&p),&u));
        if u>=zero_r() && u<=one() && face.inside(&point) {return (point.clone(),point);}
    }
    let mut candidates=vec![(p.clone(),face.closest(&p)),(q.clone(),face.closest(&q))];
    for (u,v) in [(&a,&b),(&b,&c),(&c,&a)] {candidates.push(segment_pair(&p,&q,u,v));}
    candidates.into_iter().min_by_key(|(a,b)|distance2(a,b)).unwrap()
}
fn axis_at_points(p:RV,q:RV,a:RV,b:RV,c:RV,clearance_radius2:&R)->Option<RV> {
    let (first,last)=closest_pair_points(p,q,a,b,c);
    // A sampled pair can reject penetration, never certify a trajectory.
    if distance2(&first,&last)<*clearance_radius2 {return None;}
    // Arbitrary finite rounded directions keep polynomial operands small.
    // Exact closest points are not themselves clearance certificates.
    let direction=sub(&first,&last);
    let mut axis=std::array::from_fn(|_|zero_r());
    for i in 0..3 {let value=direction[i].to_f64()?;if !value.is_finite() {return None;}axis[i]=real(value);}
    Some(axis)
}
#[cfg(test)]
fn axis_at_points_reference(p:RV,q:RV,a:RV,b:RV,c:RV,clearance_radius2:&R)->Option<RV> {
    let n=cross(&sub(&b,&a),&sub(&c,&a));let den=dot(&sub(&q,&p),&n);
    if den!=zero_r() {
        let u=dot(&sub(&a,&p),&n)/den;
        if u>=zero_r() && u<=one() && inside(&add(&p,&scale(&sub(&q,&p),&u)),&a,&b,&c) {return None;}
    }
    let mut candidates=vec![(p.clone(),point_triangle(&p,&a,&b,&c)),(q.clone(),point_triangle(&q,&a,&b,&c))];
    for (u,v) in [(&a,&b),(&b,&c),(&c,&a)] {candidates.push(segment_pair_reference(&p,&q,u,v));}
    let (first,last)=candidates.into_iter().min_by_key(|(a,b)|distance2(a,b)).unwrap();
    // A sampled pair can reject penetration, never certify a trajectory.
    if distance2(&first,&last)<*clearance_radius2 {return None;}
    // Arbitrary finite rounded directions keep polynomial operands small.
    // Exact closest points are not themselves clearance certificates.
    let direction=sub(&first,&last);
    let mut axis=std::array::from_fn(|_|zero_r());
    for i in 0..3 {let value=direction[i].to_f64()?;if !value.is_finite() {return None;}axis[i]=real(value);}
    Some(axis)
}
fn selected_axis<const EXACT:bool>(capsule:CapsuleMotion,triangle:TriangleMotion,time:&R,geometry:&RelativeGeometry)->Option<RV> {
    if EXACT {return geometry.axis_at(time);}
    // Rounded closest points choose an arbitrary finite axis only. They never
    // reject/accept physical motion: the exact interval proof owns admission.
    let (a,b)=super::super::closest_points(capsule,triangle,time.to_f64()?).ok()?;
    let axis=std::array::from_fn(|k|a[k]-b[k]);
    if axis.iter().any(|v|!v.is_finite()) || axis.iter().all(|v|*v==0.) {return None;}
    Some(axis.map(real))
}
pub(in crate::hair::contact::triangle_sweep) fn certifies(capsule:CapsuleMotion,triangle:TriangleMotion,tolerance:f64,budget:usize)->bool {
    if validate_capsule(capsule,CapsuleSweepOptions {tolerance_m:tolerance,max_iterations:budget}).is_err()
        || !triangle.start.iter().chain(&triangle.end).all(|p|finite(*p)) {return false;}
    let motion=ExactMotion::new(capsule,triangle,tolerance);
    // A discovered floating-point minimum is only a time hint. Reject a
    // clearance proof early solely when exact interpolated feature distances
    // at that dyadic time violate the original radius minus tolerance.
    // An unconfirmed hint leaves every existing proof path intact.
    if motion.radius>zero_r() {
        if let Ok(Some(witness))=super::super::trajectory::trajectory_contact_after_nonclear_sweep(
            capsule,triangle,CapsuleSweepOptions {tolerance_m:tolerance,max_iterations:budget},false) {
            if RelativeGeometry::new(&motion).penetrates_at(&real(witness.time)) {return false;}
        }
    }
    // Each exact certificate family has the original bounded proof budget.
    // Unknown feature-axis proofs retain the unchanged general discovery path.
    super::dyadic::feature_certifies(&motion,budget)
        || walk::<false,true>(capsule,triangle,tolerance,budget).is_some()
}
#[cfg(test)]
pub(super) fn build(capsule:CapsuleMotion,triangle:TriangleMotion,tolerance:f64,budget:usize)->Option<Vec<(R,R,[Vec<R>;3])>> {
    walk::<true,true>(capsule,triangle,tolerance,budget)
}
fn walk<const RETAIN:bool,const EXACT:bool>(capsule:CapsuleMotion,triangle:TriangleMotion,tolerance:f64,budget:usize)->Option<Vec<(R,R,[Vec<R>;3])>> {
    if validate_capsule(capsule,CapsuleSweepOptions {tolerance_m:tolerance,max_iterations:budget}).is_err()
        || !triangle.start.iter().chain(&triangle.end).all(|p|finite(*p)) {return None;}
    let motion=ExactMotion::new(capsule,triangle,tolerance);
    if motion.radius<=zero_r() {return None;}
    let geometry=RelativeGeometry::new(&motion);
    // Dyadic children share their parent's endpoints exactly. Reuse only these
    // immutable axes within this one motion query, never across changed geometry.
    let first=selected_axis::<EXACT>(capsule,triangle,&zero_r(),&geometry)?;
    let last=selected_axis::<EXACT>(capsule,triangle,&one(),&geometry)?;
    let mut stack=vec![(zero_r(),one(),0,first,last)];let mut visited=0;let mut leaves=Vec::new();
    while let Some((lo,hi,depth,first,last))=stack.pop() {
        visited+=1;if visited>budget || depth>48 {return None;}
        let mid=(&lo+&hi)/R::from_integer(2.into());
        let middle=selected_axis::<EXACT>(capsule,triangle,&mid,&geometry)?;
        let axis=std::array::from_fn(|i|vec![first[i].clone(),
            &middle[i]*R::from_integer(4.into())-&first[i]*R::from_integer(3.into())-&last[i],
            &first[i]*R::from_integer(2.into())-&middle[i]*R::from_integer(4.into())+&last[i]*R::from_integer(2.into())]);
        if motion.verify(&lo,&hi,&axis) {if RETAIN {leaves.push((lo,hi,axis));}}
        else {stack.push((mid.clone(),hi,depth+1,middle.clone(),last));stack.push((lo,mid,depth+1,first,middle));}
    }
    Some(leaves)
}
#[test]
#[ignore = "requires original VRC1 geometry; constructs axes natively without serialized witnesses"]
fn captured_native_exact_axis_discovery_certifies_and_rejects_penetration() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();
    let mut lines=data.lines();assert_eq!(lines.next(),Some("VRC1"));
    let values=lines.next().unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap()).collect::<Vec<_>>();
    assert_eq!(values.len(),32);
    let p=|i:usize|[values[i],values[i+1],values[i+2]];
    let capsule=CapsuleMotion {start:[p(0),p(3)],end:[p(6),p(9)],radius:values[30]};
    let triangle=TriangleMotion {start:[p(12),p(15),p(18)],end:[p(21),p(24),p(27)]};
    let motion=ExactMotion::new(capsule,triangle,values[31]);let geometry=RelativeGeometry::new(&motion);
    for i in 0..=32 {let time=R::new(i.into(),32.into());assert_eq!(geometry.axis_at(&time),axis_at(capsule,triangle,&time,&motion.radius));}
    if std::env::var_os("VOXY_HAIR_EXACT_GEOMETRY_BENCHMARK").is_some() {
        let mut times=[Vec::new(),Vec::new()];
        for round in 0..6 {for mode in [round%2,1-round%2] {
            let began=std::time::Instant::now();
            for i in 0..=32 {let time=R::new(i.into(),32.into());let axis=if mode==0 {axis_at(capsule,triangle,&time,&motion.radius)} else {geometry.axis_at(&time)};assert!(std::hint::black_box(axis).is_some());}
            if round>0 {times[mode].push(began.elapsed().as_secs_f64()*1000.);}
        }}
        let raw=times.clone();for samples in &mut times {samples.sort_by(f64::total_cmp);}
        eprintln!("EXACT GEOMETRY BENCHMARK reference_ms={:?} prepared_ms={:?} reference_median_ms={} prepared_median_ms={} ratio={} scope=33_exact_axis_queries_not_certificate_or_frame_or_FPS",raw[0],raw[1],times[0][2],times[1][2],times[0][2]/times[1][2]);
    }
    let began=std::time::Instant::now();let leaves=build(capsule,triangle,values[31],1024).expect("native exact reference must certify original trajectory");
    let mut previous=zero_r();
    for (lo,hi,axis) in &leaves {assert_eq!(*lo,previous);assert!(verify_interval(capsule,triangle,values[31],lo,hi,axis));previous=hi.clone();}
    assert_eq!(previous,one());assert!(!leaves.is_empty());
    eprintln!("NATIVE EXACT CERTIFICATE intervals={} elapsed_s={} scope=single_primitive_no_runtime_or_FPS_claim",leaves.len(),began.elapsed().as_secs_f64());
    assert!(build(CapsuleMotion {radius:capsule.radius+1e-9,..capsule},triangle,values[31],1024).is_none());
    assert!(build(capsule,triangle,values[31],1).is_none(),"exhausted proof budget is unknown");
    assert!(walk::<false,true>(capsule,triangle,values[31],1024).unwrap().is_empty(),"production proof must not retain interval witnesses");
    let options=CapsuleSweepOptions {tolerance_m:values[31],max_iterations:1024};
    assert_eq!(sweep_capsule_triangle(capsule,triangle,options).unwrap(),CapsuleSweep::Clear);
    assert_ne!(sweep_capsule_triangle(CapsuleMotion {radius:capsule.radius+1e-9,..capsule},triangle,options).unwrap(),CapsuleSweep::Clear);
}

#[test]
fn exact_axis_discovery_rejects_interior_crossing_and_accepts_clear_motion() {
    let triangle=TriangleMotion {
        start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],
        end:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],
    };
    let clear=CapsuleMotion {start:[[-0.1,0.,1.],[0.1,0.,1.]],end:[[-0.1,0.,2.],[0.1,0.,2.]],radius:0.1};
    assert!(build(clear,triangle,1e-10,64).is_some());
    let crossing=CapsuleMotion {end:[[-0.1,0.,-1.],[0.1,0.,-1.]],..clear};
    assert!(build(crossing,triangle,1e-10,64).is_none(),"clear endpoints cannot admit an interior collision");
    let penetrating=CapsuleMotion {start:[[-0.1,0.,0.05],[0.1,0.,0.05]],end:[[-0.1,0.,0.05],[0.1,0.,0.05]],..clear};
    assert!(build(penetrating,triangle,1e-10,64).is_none());
    assert!(build(clear,triangle,1e-10,0).is_none());
    assert!(build(CapsuleMotion {radius:f64::NAN,..clear},triangle,1e-10,64).is_none());
}

#[test]
fn retained_and_discarded_witnesses_have_identical_admission() {
    let triangle=TriangleMotion {start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],end:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]]};
    for z in [-1.,-0.05,0.,0.05,1.] {
        for end_z in [-1.,0.,1.] {
            let capsule=CapsuleMotion {start:[[-0.1,0.,z],[0.1,0.,z]],end:[[-0.1,0.,end_z],[0.1,0.,end_z]],radius:0.1};
            for budget in [0,1,8] {
                assert_eq!(certifies(capsule,triangle,1e-10,budget),build(capsule,triangle,1e-10,budget).is_some());
            }
        }
    }
}

#[test]
fn exact_interval_proofs_guard_rounded_axis_hints() {
    let triangle=TriangleMotion {start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],end:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]]};
    let capsule=CapsuleMotion {start:[[-0.1,0.,1.],[0.1,0.,1.]],end:[[-0.1,0.,2.],[0.1,0.,2.]],radius:0.1};
    let leaves=walk::<true,false>(capsule,triangle,1e-10,64).unwrap();
    let motion=ExactMotion::new(capsule,triangle,1e-10);let mut previous=zero_r();
    for (lo,hi,axis) in &leaves {assert_eq!(*lo,previous);assert!(motion.verify_rational(lo,hi,axis));previous=hi.clone();}
    assert_eq!(previous,one());
    assert!(walk::<true,false>(CapsuleMotion {end:[[-0.1,0.,-1.],[0.1,0.,-1.]],..capsule},triangle,1e-10,64).is_none());
    assert!(walk::<true,false>(capsule,triangle,1e-10,0).is_none());
}

#[test]
#[ignore="requires captured exact axis geometry"]
fn captured_exact_interval_proofs_with_rounded_axis_hints() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();
    let values=data.lines().nth(1).unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap()).collect::<Vec<_>>();
    let p=|i:usize|[values[i],values[i+1],values[i+2]];
    let capsule=CapsuleMotion {start:[p(0),p(3)],end:[p(6),p(9)],radius:values[30]};
    let triangle=TriangleMotion {start:[p(12),p(15),p(18)],end:[p(21),p(24),p(27)]};
    let began=std::time::Instant::now();
    let leaves=walk::<true,false>(capsule,triangle,values[31],1024);
    eprintln!("EXACT ROUNDED AXIS HINTS certified={} elapsed_s={} scope=captured_primitive_not_FPS",leaves.is_some(),began.elapsed().as_secs_f64());
    if let Some(leaves)=leaves {
        let motion=ExactMotion::new(capsule,triangle,values[31]);let mut previous=zero_r();
        for (lo,hi,axis) in &leaves {assert_eq!(*lo,previous);assert!(motion.verify_rational(lo,hi,axis));previous=hi.clone();}
        assert_eq!(previous,one());
    }
    assert!(walk::<true,false>(CapsuleMotion {radius:capsule.radius+1e-9,..capsule},triangle,values[31],1024).is_none());
}

#[test]
fn exact_relative_geometry_preserves_moving_and_degenerate_axis_queries() {
    for moving in [false,true] {for collapsed in [false,true] {
        let start=if collapsed {[[100.,-20.,0.];3]} else {[[98.,-22.,0.],[102.,-22.,0.],[100.,-18.,0.]]};
        let triangle=TriangleMotion {start,end:if moving {start.map(|p|[p[0]+0.125,p[1]-0.25,p[2]+0.0625])} else {start}};
        for height in [0.,0.05,0.5] {
            let capsule=CapsuleMotion {start:[[99.9,-20.,height],[100.1,-20.,height]],end:[[100.,-19.9,height+0.125],[100.2,-19.9,height+0.125]],radius:0.1};
            let motion=ExactMotion::new(capsule,triangle,1e-10);let geometry=RelativeGeometry::new(&motion);
            for i in 0..=8 {let time=R::new(i.into(),8.into());assert_eq!(geometry.axis_at(&time),axis_at(capsule,triangle,&time,&motion.radius));}
        }
    }}
}

#[test]
fn exact_time_rejection_distinguishes_penetration_clearance_and_invalid_time() {
    let triangle=TriangleMotion {start:[[-1.,-1.,0.],[1.,-1.,0.],[0.,1.,0.]],end:[[-1.,-1.,0.],[1.,-1.,0.],[0.,1.,0.]]};
    let capsule=CapsuleMotion {start:[[0.,0.,0.5],[0.1,0.,0.5]],end:[[0.,0.,-0.5],[0.1,0.,-0.5]],radius:0.01};
    let geometry=RelativeGeometry::new(&ExactMotion::new(capsule,triangle,1e-10));
    assert!(!geometry.penetrates_at(&zero_r()));
    assert!(geometry.penetrates_at(&real(0.5)));
    assert!(!geometry.penetrates_at(&one()));
    assert!(!geometry.penetrates_at(&real(-0.5)));
    assert!(!geometry.penetrates_at(&real(1.5)));
    let degenerate=TriangleMotion {start:[[0.;3];3],end:[[0.;3];3]};
    let point=RelativeGeometry::new(&ExactMotion::new(capsule,degenerate,1e-10));
    assert!(point.penetrates_at(&real(0.5)));
    assert!(!point.penetrates_at(&zero_r()));
}

#[test]
#[ignore = "requires original VRC1 geometry; paired exact rejection benchmark"]
fn captured_exact_time_rejection_benchmark() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();
    let values=data.lines().nth(1).unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap()).collect::<Vec<_>>();
    assert_eq!(values.len(),32);
    let p=|i:usize|[values[i],values[i+1],values[i+2]];
    let original=CapsuleMotion {start:[p(0),p(3)],end:[p(6),p(9)],radius:values[30]};
    let triangle=TriangleMotion {start:[p(12),p(15),p(18)],end:[p(21),p(24),p(27)]};
    let mut inward=original;
    for end in &mut inward.end {end[0]+=1e-5;}
    let tolerance=values[31];let budget=1024;
    let hint=super::super::trajectory::trajectory_contact_after_nonclear_sweep(inward,triangle,
        CapsuleSweepOptions {tolerance_m:tolerance,max_iterations:budget},false).unwrap().unwrap();
    assert!(RelativeGeometry::new(&ExactMotion::new(inward,triangle,tolerance)).penetrates_at(&real(hint.time)),
        "benchmark requires an independently exact penetrating time, not a rounded minimum claim");
    for (name,capsule,expected) in [("clear",original,true),("penetrating",inward,false)] {
        let mut times=[Vec::new(),Vec::new()];
        for round in 0..4 {for mode in [round%2,1-round%2] {
            let began=std::time::Instant::now();
            let outcome=if mode==0 {
                let motion=ExactMotion::new(capsule,triangle,tolerance);
                super::dyadic::feature_certifies(&motion,budget)
                    || walk::<false,true>(capsule,triangle,tolerance,budget).is_some()
            } else {certifies(capsule,triangle,tolerance,budget)};
            assert_eq!(outcome,expected);
            if round>0 {times[mode].push(began.elapsed().as_secs_f64()*1000.);}
        }}
        let raw=times.clone();for samples in &mut times {samples.sort_by(f64::total_cmp);}
        eprintln!("EXACT TIME REJECTION BENCHMARK case={name} baseline_ms={:?} early_rejection_ms={:?} baseline_median_ms={} early_median_ms={} ratio={} scope=single_primitive_not_full_frame_or_FPS",raw[0],raw[1],times[0][1],times[1][1],times[0][1]/times[1][1]);
    }
}

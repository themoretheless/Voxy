//! Exact moving-axis certificate used only after floating-point proofs remain inconclusive.
use super::*;
use num_rational::BigRational as R;
fn zero_r()->R {R::from_integer(0.into())}
fn real(value:f64)->R {R::from_float(value).expect("finite exact fixture coordinate")}
fn plus(a:&[R],b:&[R])->Vec<R> {
    (0..a.len().max(b.len())).map(|i|a.get(i).cloned().unwrap_or_else(zero_r)+b.get(i).cloned().unwrap_or_else(zero_r)).collect()
}
fn product(a:&[R],b:&[R])->Vec<R> {
    let mut out=vec![zero_r();a.len()+b.len()-1];
    for (i,x) in a.iter().enumerate() {for (j,y) in b.iter().enumerate() {out[i+j]+=x*y;}}
    out
}
fn choose(n:usize,k:usize)->u32 {(0..k).fold(1,|a,i|a*(n-i) as u32/(i+1) as u32)}
fn nonnegative(power:&[R],degree:usize,strict:bool)->bool {
    (0..=degree).all(|i| {
        let value=(0..=i).fold(zero_r(),|sum,j|sum+power.get(j).cloned().unwrap_or_else(zero_r)*R::new(choose(i,j).into(),choose(degree,j).into()));
        if strict {value>zero_r()} else {value>=zero_r()}
    })
}
struct ExactMotion {
    radius:R,
    first:[[[R;3];3];2],
    slope:[[[R;3];3];2],
}
impl ExactMotion {
    fn new(capsule:CapsuleMotion,triangle:TriangleMotion,tolerance:f64)->Self {
        let first=std::array::from_fn(|e|std::array::from_fn(|v|std::array::from_fn(|k|
            real(capsule.start[e][k])-real(triangle.start[v][k]))));
        let slope=std::array::from_fn(|e|std::array::from_fn(|v|std::array::from_fn(|k|
            real(capsule.end[e][k])-real(triangle.end[v][k])-&first[e][v][k])));
        Self {radius:real(capsule.radius)-real(tolerance),first,slope}
    }
    fn verify(&self,lo:&R,hi:&R,axis:&[Vec<R>;3])->bool {
        dyadic::verify(self,lo,hi,axis).unwrap_or_else(||self.verify_rational(lo,hi,axis))
    }
    fn verify_rational(&self,lo:&R,hi:&R,axis:&[Vec<R>;3])->bool {
        let radius=&self.radius;
        let degree=axis[0].len().saturating_sub(1);
        if *radius<=zero_r() || hi<=lo || axis[0].is_empty() || degree>3 || axis.iter().any(|a|a.len()!=degree+1) {return false;}
        let norm=axis.iter().fold(vec![zero_r()],|sum,a|plus(&sum,&product(a,a)));
        if !nonnegative(&norm,2*degree,true) {return false;}
        let negative_norm=norm.iter().map(|v|-v*radius*radius).collect::<Vec<_>>();
        let width=hi-lo;
        for endpoint in 0..2 {for vertex in 0..3 {
            let mut projection=vec![zero_r()];
            for k in 0..3 {
                let first=&self.first[endpoint][vertex][k];
                let slope=&self.slope[endpoint][vertex][k];
                let relative=vec![first+lo*slope,&width*slope];
                projection=plus(&projection,&product(&relative,&axis[k]));
            }
            if !nonnegative(&projection,degree+1,false) {return false;}
            let clearance=plus(&product(&projection,&projection),&negative_norm);
            if !nonnegative(&clearance,2*degree+2,false) {return false;}
        }}
        true
    }
}
#[cfg(test)]
fn verify_interval(capsule:CapsuleMotion,triangle:TriangleMotion,tolerance:f64,lo:&R,hi:&R,axis:&[Vec<R>;3])->bool {
    ExactMotion::new(capsule,triangle,tolerance).verify_rational(lo,hi,axis)
}

#[test]
#[ignore = "requires serialized exact-rational witness fixture"]
fn captured_rational_axis_witnesses_cover_complete_trajectory() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();
    let mut lines=data.lines();assert_eq!(lines.next(),Some("VRC1"));
    let mut fields=lines.next().unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap());
    let mut vector=||std::array::from_fn(|_|fields.next().unwrap());
    let start=[vector(),vector()];let end=[vector(),vector()];
    let ts=[vector(),vector(),vector()];let te=[vector(),vector(),vector()];
    let radius=fields.next().unwrap();let tolerance=fields.next().unwrap();assert!(fields.next().is_none());
    let capsule=CapsuleMotion {start,end,radius};let triangle=TriangleMotion {start:ts,end:te};
    validate_capsule(capsule,CapsuleSweepOptions {tolerance_m:tolerance,..Default::default()}).unwrap();
    assert!(ts.iter().chain(&te).all(|p|finite(*p)));
    let mut previous=zero_r();let mut count=0;let mut rejects_larger=false;
    for line in lines {
        let mut values=line.split_whitespace().map(|v|v.parse::<R>().unwrap());
        let lo=values.next().unwrap();let hi=values.next().unwrap();
        let axis:[Vec<R>;3]=std::array::from_fn(|_|(0..3).map(|_|values.next().unwrap()).collect());
        assert!(values.next().is_none());assert_eq!(lo,previous);assert!(hi>lo);
        assert!(verify_interval(capsule,triangle,tolerance,&lo,&hi,&axis),"invalid exact witness interval {count}");
        let larger=CapsuleMotion {radius:radius+1e-9,..capsule};
        rejects_larger|=!verify_interval(larger,triangle,tolerance,&lo,&hi,&axis);
        previous=hi;count+=1;
    }
    assert_eq!(previous,R::from_integer(1.into()));assert_eq!(count,17);assert!(rejects_larger);
    eprintln!("EXACT RUST CERTIFICATE intervals={count} larger_radius_rejected={rejects_larger} scope=one_captured_primitive_not_engine_admission");
}

#[path = "contact_triangle_exact_discovery.rs"]
pub(super) mod discovery;

#[path = "contact_triangle_dyadic.rs"]
mod dyadic;

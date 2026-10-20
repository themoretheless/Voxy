//! Shared independent f64 reference; contains no GPU code.
#[repr(C)]
#[derive(Clone,Copy,Debug,bytemuck::Pod,bytemuck::Zeroable)]
pub(crate) struct Case {pub matrix:[f32;16],pub begin:[f32;4],pub end:[f32;4],pub sample:[f32;4]}
pub(crate) fn oracle_at(case:Case,screen_parameter:f64)->Option<([f64;2],glam::DVec3,glam::DVec3,f64,glam::DVec3,glam::DVec3)> {
 let m=glam::DMat4::from_cols_array(&case.matrix.map(f64::from));let a=glam::DVec3::from_array([case.begin[0] as f64,case.begin[1] as f64,case.begin[2] as f64]);let b=glam::DVec3::from_array([case.end[0] as f64,case.end[1] as f64,case.end[2] as f64]);let qa=m*a.extend(1.);let qb=m*b.extend(1.);let mut lo=0f64;let mut hi=1f64;
 for (a,b) in [(qa.w+qa.x,qb.w+qb.x),(qa.w-qa.x,qb.w-qb.x),(qa.w+qa.y,qb.w+qb.y),(qa.w-qa.y,qb.w-qb.y),(qa.z,qb.z),(qa.w-qa.z,qb.w-qb.z)] {
  if a<0.&&b<0. {return None;}if a<0. {lo=lo.max(-a/(b-a));}if b<0. {hi=hi.min(-a/(b-a));}
 }
 if lo>hi{return None;}
 let wa=a+(b-a)*lo;let wb=a+(b-a)*hi;let ca=m*wa.extend(1.);let cb=m*wb.extend(1.);if ca.w<=0.||cb.w<=0. {return None;}
 let pa=ca.truncate()/ca.w;let pb=cb.truncate()/cb.w;let s=screen_parameter;let local=s*ca.w/((1.-s)*cb.w+s*ca.w);let u=lo+(hi-lo)*local;let world=a+(b-a)*u;let expected_ndc=pa+(pb-pa)*s;
 Some(([lo,hi],pa,pb,u,world,expected_ndc))
}
pub(crate) fn oracle(case:Case)->Option<([f64;2],glam::DVec3,glam::DVec3,f64,glam::DVec3,glam::DVec3)> {oracle_at(case,f64::from(case.sample[0]))}

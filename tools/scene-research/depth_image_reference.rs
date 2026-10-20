//! Exhaustive f64 closed-pixel geometry reference, independent of traversal.
pub(crate) fn intersect(depth:&[f32],width:usize,height:usize,begin:[f64;3],end:[f64;3])->Option<([i32;2],f64)> {
 let d=std::array::from_fn::<_,3,_>(|i|end[i]-begin[i]);let mut best:Option<([i32;2],f64)>=None;
 for y in 0..height {for x in 0..width {
  let z=f64::from(depth[y*width+x]);if !(0.0..1.0).contains(&z){continue;}
  let mut lo=0f64;let mut hi=1f64;
  for (axis,min,max) in [(0,x as f64,(x+1) as f64),(1,y as f64,(y+1) as f64)] {
   if d[axis]==0. {if begin[axis]<min||begin[axis]>max {lo=2.;break;}}
   else {let a=(min-begin[axis])/d[axis];let b=(max-begin[axis])/d[axis];lo=lo.max(a.min(b));hi=hi.min(a.max(b));}
  }
  if lo>hi {continue;}
  let t=if d[2]==0. {if begin[2]!=z {continue;}lo}else{(z-begin[2])/d[2]};
  if t<lo||t>hi {continue;}
  let pixel=[x as i32,y as i32];
  if best.is_none_or(|(p,b)| t<b || (t==b&&(y*width+x)<(p[1] as usize*width+p[0] as usize))) {best=Some((pixel,t));}
 }}best
}

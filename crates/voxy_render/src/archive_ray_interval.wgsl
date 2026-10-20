// Conservative rejection only. These intervals never change hit geometry.
// WGSL 15.7.4: +/-/* correctly rounded; / permits 2.5 ULP. Division
// endpoints expand by eight representable neighbours to cover binade changes.
// Subnormal inputs/results expand to +/- minimum normal (flush-to-zero safe).
struct DdaInterval {bounds:vec2<f32>,valid:bool}
fn dda_interval_finite(x:vec2<f32>)->bool{return !any((bitcast<vec2<u32>>(x)&vec2<u32>(0x7f800000u))==vec2<u32>(0x7f800000u));}
fn dda_interval_neighbor(x:f32,up:bool,steps:u32)->f32 {
 if abs(x)<bitcast<f32>(0x00800000u){return select(-bitcast<f32>(0x00800000u),bitcast<f32>(0x00800000u),up);}
 let bits=bitcast<u32>(x);let outward=select(bits-steps,bits+steps,(x>0.)==up);
 // A subnormal bound could be flushed on the next operation. Zero is
 // outward here: only positive lower / negative upper bounds enter this case.
 if (outward&0x7f800000u)==0u&&(outward&0x007fffffu)!=0u {return 0.;}
 return bitcast<f32>(outward);
}
fn dda_interval_expand(bounds:vec2<f32>,steps:u32)->DdaInterval {
 if !dda_interval_finite(bounds){return DdaInterval(vec2<f32>(0.),false);}
 let v=vec2<f32>(dda_interval_neighbor(bounds.x,false,steps),dda_interval_neighbor(bounds.y,true,steps));
 return DdaInterval(v,dda_interval_finite(v));
}
fn dda_interval_pair(pair:vec2<f32>)->DdaInterval {
 if !dda_interval_finite(pair){return DdaInterval(vec2<f32>(0.),false);}
 // Replacing a flushed input by its containing interval is conservative.
 let small=bitcast<f32>(0x00800000u);
 let a=select(vec2<f32>(pair.x),vec2<f32>(-small,small),(bitcast<u32>(pair.x)&0x7f800000u)==0u&&(bitcast<u32>(pair.x)&0x007fffffu)!=0u);
 let b=select(vec2<f32>(pair.y),vec2<f32>(-small,small),(bitcast<u32>(pair.y)&0x7f800000u)==0u&&(bitcast<u32>(pair.y)&0x007fffffu)!=0u);
 return dda_interval_expand(vec2<f32>(projection_add_scalar(a.x,b.x),projection_add_scalar(a.y,b.y)),1u);
}
fn dda_interval_add(a:DdaInterval,b:DdaInterval)->DdaInterval {
 if !a.valid||!b.valid{return DdaInterval(vec2<f32>(0.),false);}
 return dda_interval_expand(vec2<f32>(projection_add_scalar(a.bounds.x,b.bounds.x),projection_add_scalar(a.bounds.y,b.bounds.y)),1u);
}
fn dda_interval_neg(a:DdaInterval)->DdaInterval{return DdaInterval(-a.bounds.yx,a.valid);}
fn dda_interval_mul(a:DdaInterval,b:DdaInterval)->DdaInterval {
 if !a.valid||!b.valid{return DdaInterval(vec2<f32>(0.),false);}
 let values=vec4<f32>(projection_mul_scalar(a.bounds.x,b.bounds.x),projection_mul_scalar(a.bounds.x,b.bounds.y),projection_mul_scalar(a.bounds.y,b.bounds.x),projection_mul_scalar(a.bounds.y,b.bounds.y));
 if !dda_finite4(values){return DdaInterval(vec2<f32>(0.),false);}
 return dda_interval_expand(vec2<f32>(min(min(values.x,values.y),min(values.z,values.w)),max(max(values.x,values.y),max(values.z,values.w))),1u);
}
fn dda_interval_div(a:DdaInterval,b:DdaInterval)->DdaInterval {
 if !a.valid||!b.valid||(b.bounds.x<=0.&&b.bounds.y>=0.)||min(abs(b.bounds.x),abs(b.bounds.y))<bitcast<f32>(0x00800000u)||max(abs(b.bounds.x),abs(b.bounds.y))>bitcast<f32>(0x7e800000u){return DdaInterval(vec2<f32>(0.),false);}
 let values=vec4<f32>(a.bounds.x/b.bounds.x,a.bounds.x/b.bounds.y,a.bounds.y/b.bounds.x,a.bounds.y/b.bounds.y);
 if !dda_finite4(values){return DdaInterval(vec2<f32>(0.),false);}
 return dda_interval_expand(vec2<f32>(min(min(values.x,values.y),min(values.z,values.w)),max(max(values.x,values.y),max(values.z,values.w))),8u);
}
// Per-ray rejection invariants. These are the same operations as the uncached
// filter, evaluated once; no approximate reciprocal or narrower interval.
struct DdaIntervalRay {begin:array<DdaInterval,3>,delta:array<DdaInterval,3>}
fn dda_interval_ray(begin:DdaVectorPair,end:DdaVectorPair)->DdaIntervalRay {
 var ray:DdaIntervalRay;
 for(var axis=0u;axis<3u;axis++) {
  ray.begin[axis]=dda_interval_pair(vec2<f32>(begin.high[axis],begin.low[axis]));
  ray.delta[axis]=dda_interval_add(dda_interval_pair(vec2<f32>(end.high[axis],end.low[axis])),dda_interval_neg(ray.begin[axis]));
 }
 return ray;
}
fn dda_interval_can_skip_cached(ray:DdaIntervalRay,entry:vec2<f32>,edge:vec2<f32>,moving:vec2<bool>,depth_range:vec2<f32>)->bool {
 var exits=array<DdaInterval,2>(DdaInterval(vec2<f32>(1.),true),DdaInterval(vec2<f32>(1.),true));
 for(var axis=0u;axis<2u;axis++) {if moving[axis] {
  let b=ray.begin[axis];
  exits[axis]=dda_interval_div(dda_interval_add(DdaInterval(vec2<f32>(edge[axis]),true),dda_interval_neg(b)),ray.delta[axis]);
 }}
 if !exits[0].valid||!exits[1].valid{return false;}
 let start=dda_interval_pair(entry);if !start.valid{return false;}
 let end_t=min(1.,min(exits[0].bounds.y,exits[1].bounds.y));
 let interval=DdaInterval(vec2<f32>(min(start.bounds.x,end_t),max(start.bounds.y,end_t)),true);
 let z=ray.begin[2];let delta=ray.delta[2];
 let depth=dda_interval_add(z,dda_interval_mul(delta,interval));
 return depth.valid&&(depth.bounds.y<depth_range.x||depth.bounds.x>depth_range.y);
}

fn dda_interval_can_skip(begin:DdaVectorPair,end:DdaVectorPair,entry:vec2<f32>,edge:vec2<f32>,moving:vec2<bool>,depth_range:vec2<f32>)->bool {
 var exits=array<DdaInterval,2>(DdaInterval(vec2<f32>(1.),true),DdaInterval(vec2<f32>(1.),true));
 for(var axis=0u;axis<2u;axis++) {if moving[axis] {
  let b=dda_interval_pair(vec2<f32>(begin.high[axis],begin.low[axis]));let e=dda_interval_pair(vec2<f32>(end.high[axis],end.low[axis]));
  exits[axis]=dda_interval_div(dda_interval_add(DdaInterval(vec2<f32>(edge[axis]),true),dda_interval_neg(b)),dda_interval_add(e,dda_interval_neg(b)));
 }}
 if !exits[0].valid||!exits[1].valid{return false;}
 let start=dda_interval_pair(entry);if !start.valid{return false;}
 let end_t=min(1.,min(exits[0].bounds.y,exits[1].bounds.y));
 let interval=DdaInterval(vec2<f32>(min(start.bounds.x,end_t),max(start.bounds.y,end_t)),true);
 let z=dda_interval_pair(vec2<f32>(begin.high.z,begin.low.z));let delta=dda_interval_add(dda_interval_pair(vec2<f32>(end.high.z,end.low.z)),dda_interval_neg(z));
 let depth=dda_interval_add(z,dda_interval_mul(delta,interval));
 return depth.valid&&(depth.bounds.y<depth_range.x||depth.bounds.x>depth_range.y);
}

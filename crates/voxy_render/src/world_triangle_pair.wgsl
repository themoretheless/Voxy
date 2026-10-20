// Compensated world triangle diagnostic with rejection-only interval certificates.
// Uses archive_projection_pair.wgsl arithmetic. State 2 requires exact fallback;
// this module does not assert that a native f32 BVH encloses arbitrary hi/lo rays.
struct WorldPair3 {x:vec2<f32>,y:vec2<f32>,z:vec2<f32>}
struct WorldInterval {lo:f32,hi:f32,valid:u32}
struct WorldInterval3 {x:WorldInterval,y:WorldInterval,z:WorldInterval}
struct WorldTriangleResult {state:u32,reason:u32,t:vec2<f32>,u:vec2<f32>,v:vec2<f32>}
fn wp_scalar(p:vec3<f32>)->WorldPair3 {return WorldPair3(vec2<f32>(p.x,0.),vec2<f32>(p.y,0.),vec2<f32>(p.z,0.));}
fn wp_sub(a:WorldPair3,b:WorldPair3)->WorldPair3 {return WorldPair3(projection_pair_add(a.x,-b.x),projection_pair_add(a.y,-b.y),projection_pair_add(a.z,-b.z));}
fn wp_cross(a:WorldPair3,b:WorldPair3)->WorldPair3 {return WorldPair3(
 projection_pair_add(projection_pair_mul(a.y,b.z),-projection_pair_mul(a.z,b.y)),
 projection_pair_add(projection_pair_mul(a.z,b.x),-projection_pair_mul(a.x,b.z)),
 projection_pair_add(projection_pair_mul(a.x,b.y),-projection_pair_mul(a.y,b.x)));}
fn wp_dot(a:WorldPair3,b:WorldPair3)->vec2<f32> {return projection_pair_add(projection_pair_add(projection_pair_mul(a.x,b.x),projection_pair_mul(a.y,b.y)),projection_pair_mul(a.z,b.z));}
fn wi_bad()->WorldInterval {return WorldInterval(0.,0.,0u);}
fn wi_finite(v:f32)->bool {return (bitcast<u32>(v)&0x7f800000u)!=0x7f800000u;}
fn wi_normal_bits(raw:u32)->bool {let bits=raw&0x7fffffffu;return bits==0u||(bits>=0x01000000u&&bits<0x7f000000u);}
fn wi_normal_or_zero(v:f32)->bool {return wi_normal_bits(bitcast<u32>(v));}
// Guard raw source bits before exposing them to any floating-point operation.
fn wi_source_word(bits:u32)->WorldInterval {if !wi_normal_bits(bits) {return wi_bad();}return wi_scalar(bitcast<f32>(bits));}
fn wi_down(v:f32)->f32 {let bits=bitcast<u32>(v);if v==0. {return bitcast<f32>(0x80000001u);}return bitcast<f32>(select(bits-1u,bits+1u,v<0.));}
fn wi_up(v:f32)->f32 {let bits=bitcast<u32>(v);if v==0. {return bitcast<f32>(1u);}return bitcast<f32>(select(bits+1u,bits-1u,v<0.));}
fn wi_scalar(v:f32)->WorldInterval {return WorldInterval(v,v,select(0u,1u,wi_finite(v)&&wi_normal_or_zero(v)));}
fn wi_neg(a:WorldInterval)->WorldInterval {return WorldInterval(-a.hi,-a.lo,a.valid);}
fn wi_add_endpoint(a:f32,b:f32)->WorldInterval {
 if !wi_normal_or_zero(a)||!wi_normal_or_zero(b) {return wi_bad();}
 if a==0. {return wi_scalar(b);}if b==0. {return wi_scalar(a);}if a== -b {return wi_scalar(0.);}
 let v=projection_add_scalar(a,b);if !wi_finite(v)||!wi_normal_or_zero(v)||v==0. {return wi_bad();}
 let lo=wi_down(v);let hi=wi_up(v);return WorldInterval(lo,hi,select(0u,1u,wi_normal_or_zero(lo)&&wi_normal_or_zero(hi)));
}
fn wi_add(a:WorldInterval,b:WorldInterval)->WorldInterval {
 if a.valid==0u||b.valid==0u {return wi_bad();}
 let low=wi_add_endpoint(a.lo,b.lo);let high=wi_add_endpoint(a.hi,b.hi);
 return WorldInterval(low.lo,high.hi,low.valid&high.valid);
}
fn wi_sub(a:WorldInterval,b:WorldInterval)->WorldInterval {return wi_add(a,wi_neg(b));}
fn wi_mul_endpoint(a:f32,b:f32)->WorldInterval {
 if !wi_normal_or_zero(a)||!wi_normal_or_zero(b) {return wi_bad();}
 if a==0.||b==0. {return wi_scalar(0.);}
 let v=projection_mul_scalar(a,b);if !wi_finite(v)||!wi_normal_or_zero(v)||v==0. {return wi_bad();}
 let lo=wi_down(v);let hi=wi_up(v);return WorldInterval(lo,hi,select(0u,1u,wi_normal_or_zero(lo)&&wi_normal_or_zero(hi)));
}
fn wi_mul(a:WorldInterval,b:WorldInterval)->WorldInterval {
 if a.valid==0u||b.valid==0u {return wi_bad();}
 let p=wi_mul_endpoint(a.lo,b.lo);let q=wi_mul_endpoint(a.lo,b.hi);let r=wi_mul_endpoint(a.hi,b.lo);let s=wi_mul_endpoint(a.hi,b.hi);
 return WorldInterval(min(min(p.lo,q.lo),min(r.lo,s.lo)),max(max(p.hi,q.hi),max(r.hi,s.hi)),p.valid&q.valid&r.valid&s.valid);
}
fn wi_pair(p:vec2<f32>)->WorldInterval {return wi_add(wi_scalar(p.x),wi_scalar(p.y));}
fn wi_vec(p:WorldPair3)->WorldInterval3 {return WorldInterval3(wi_pair(p.x),wi_pair(p.y),wi_pair(p.z));}
fn wi3_sub(a:WorldInterval3,b:WorldInterval3)->WorldInterval3 {return WorldInterval3(wi_sub(a.x,b.x),wi_sub(a.y,b.y),wi_sub(a.z,b.z));}
fn wi3_cross(a:WorldInterval3,b:WorldInterval3)->WorldInterval3 {return WorldInterval3(wi_sub(wi_mul(a.y,b.z),wi_mul(a.z,b.y)),wi_sub(wi_mul(a.z,b.x),wi_mul(a.x,b.z)),wi_sub(wi_mul(a.x,b.y),wi_mul(a.y,b.x)));}
fn wi3_dot(a:WorldInterval3,b:WorldInterval3)->WorldInterval {return wi_add(wi_add(wi_mul(a.x,b.x),wi_mul(a.y,b.y)),wi_mul(a.z,b.z));}
fn wi_positive(a:WorldInterval)->bool {return a.valid!=0u&&a.lo>0.;}
fn wi_negative(a:WorldInterval)->bool {return a.valid!=0u&&a.hi<0.;}
struct WorldTriangleCertificate {state:u32,reason:u32,reverse:u32}
// Shared interval certificate: state 0 alone proves a closed-primitive miss.
fn world_triangle_certificate(direction:WorldInterval3,ie1:WorldInterval3,ie2:WorldInterval3,idelta:WorldInterval3,near_t:WorldInterval,far_t:WorldInterval)->WorldTriangleCertificate {
 var reverse=0u;
 let ip=wi3_cross(direction,ie2);let iq=wi3_cross(idelta,ie1);
 var di=wi3_dot(ie1,ip);var ui=wi3_dot(idelta,ip);var vi=wi3_dot(direction,iq);var ti=wi3_dot(ie2,iq);
 if wi_negative(di) {di=wi_neg(di);ui=wi_neg(ui);vi=wi_neg(vi);ti=wi_neg(ti);reverse=1u;}
 if !wi_positive(di) {return WorldTriangleCertificate(2u,1u|select(0u,8u,di.valid==0u),reverse);}
 let remaining=wi_sub(wi_sub(di,ui),vi);
 let front=wi_sub(ti,wi_mul(di,near_t));let back=wi_sub(wi_mul(di,far_t),ti);
 if wi_negative(ui)||wi_negative(vi)||wi_negative(remaining)||wi_negative(front)||wi_negative(back) {return WorldTriangleCertificate(0u,0u,reverse);}
 var reason=0u;
 if !wi_positive(ui)||!wi_positive(vi)||!wi_positive(remaining) {reason|=2u;}
 if !wi_positive(front)||!wi_positive(back) {reason|=4u;}
 if ui.valid==0u||vi.valid==0u||remaining.valid==0u||front.valid==0u||back.valid==0u {reason|=8u;}
 if reason!=0u {return WorldTriangleCertificate(2u,reason,reverse);}
 return WorldTriangleCertificate(1u,0u,reverse);
}
// Interval edge1/edge2/delta must enclose exact source differences in one frame.
// Pair vertices may share a common translation applied before compression.
// Keep correlated ray-origin uncertainty out of the interval edge differences.
fn world_triangle_pair(origin:WorldPair3,direction:WorldPair3,v0:WorldPair3,v1:WorldPair3,v2:WorldPair3,
 ie1:WorldInterval3,ie2:WorldInterval3,idelta:WorldInterval3,near_t:vec2<f32>,far_t:vec2<f32>)->WorldTriangleResult {
 let edge1=wp_sub(v1,v0);let edge2=wp_sub(v2,v0);let p=wp_cross(direction,edge2);let delta=wp_sub(origin,v0);let q=wp_cross(delta,edge1);
 var determinant=wp_dot(edge1,p);var u=wp_dot(delta,p);var v=wp_dot(direction,q);var t=wp_dot(edge2,q);
 let certificate=world_triangle_certificate(wi_vec(direction),ie1,ie2,idelta,wi_pair(near_t),wi_pair(far_t));
 if certificate.reverse!=0u {determinant=-determinant;u=-u;v=-v;t=-t;}
 if certificate.state!=1u {return WorldTriangleResult(certificate.state,certificate.reason,vec2<f32>(0.),vec2<f32>(0.),vec2<f32>(0.));}
 return WorldTriangleResult(1u,0u,projection_pair_div(t,determinant),projection_pair_div(u,determinant),projection_pair_div(v,determinant));
}

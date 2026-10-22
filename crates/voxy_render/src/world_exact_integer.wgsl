// Exact signed base-2^16 integers. Finite f32 source transforms plus hi/lo rays
// need fewer than 1700 bits for the predicates below. Capacity is 2048 bits;
// every operation propagates overflow as unresolved, never a hit or a miss.
const EX_WORDS:u32=128u;
// Decision/closed-parameter allocation <=430 slots; parameter mode reserves four.
// Quotient temporaries reuse the arena after copying those four live operands.
const EX_SLOTS:u32=512u;
const EX_ARENA_WORDS:u32=EX_SLOTS*EX_WORDS;
@group(0) @binding(6) var<storage,read_write> exact_scratch:array<u32>;
var<private> ex_begin:u32;
var<private> ex_cursor:u32;
var<private> ex_peak:u32;
struct ExInt {offset:u32,length:u32,negative:u32,valid:u32}
struct ExVec {x:ExInt,y:ExInt,z:ExInt}
struct ExDecision {state:u32,reason:u32}
struct ExParameters {t:ExInt,u:ExInt,v:ExInt,denominator:ExInt,valid:u32}
var<private> ex_parameters:ExParameters;
// 0 unknown/coplanar, 1 positive determinant (front), 2 negative (back).
var<private> ex_triangle_facing:u32;
fn ex_zero()->ExInt {
 var r:ExInt;if ex_cursor>=EX_SLOTS {return r;}
 let offset=ex_begin+ex_cursor*EX_WORDS;if offset+EX_WORDS>arrayLength(&exact_scratch) {return r;}
 ex_cursor+=1u;ex_peak=max(ex_peak,ex_cursor);r.offset=offset;r.valid=1u;
 // A zero value has no live digits. Constructors initialize every digit they expose.
 return r;
}
fn ex_word(a:ExInt,k:u32)->u32 {if a.valid==0u||k>=a.length {return 0u;}return exact_scratch[a.offset+k];}
fn ex_raw(a:ExInt,k:u32)->u32 {return exact_scratch[a.offset+k];}
fn ex_bad()->ExInt {var r:ExInt;return r;}
fn ex_trim(a:ExInt)->ExInt {var r=a;while r.length>0u {if ex_raw(r,r.length-1u)!=0u {break;}r.length-=1u;}if r.length==0u {r.negative=0u;}return r;}
fn ex_neg(a:ExInt)->ExInt {var r=a;if r.length!=0u {r.negative^=1u;}return r;}
fn ex_cmp_abs(a:ExInt,b:ExInt)->i32 {
 if a.length<b.length {return -1;}if a.length>b.length {return 1;}
 var k=a.length;while k>0u {k-=1u;if ex_word(a,k)<ex_word(b,k) {return -1;}if ex_word(a,k)>ex_word(b,k) {return 1;}}return 0;
}
fn ex_cmp(a:ExInt,b:ExInt)->i32 {if a.negative!=b.negative {return select(1,-1,a.negative!=0u);}let c=ex_cmp_abs(a,b);return select(c,-c,a.negative!=0u);}
fn ex_add_abs(a:ExInt,b:ExInt)->ExInt {
 var r=ex_zero();if r.valid==0u {return r;}r.length=max(a.length,b.length);var carry=0u;
 for(var k=0u;k<r.length;k+=1u) {let sum=ex_word(a,k)+ex_word(b,k)+carry;exact_scratch[r.offset+(k)]=sum&65535u;carry=sum>>16u;}
 if carry!=0u {if r.length==EX_WORDS {return ex_bad();}exact_scratch[r.offset+(r.length)]=carry;r.length+=1u;}return r;
}
// Preconditions: |a|>=|b| and valid inputs.
fn ex_sub_abs(a:ExInt,b:ExInt)->ExInt {
 var r=ex_zero();if r.valid==0u {return r;}r.length=a.length;var borrow=0u;
 for(var k=0u;k<a.length;k+=1u) {let rhs=ex_word(b,k)+borrow;let lhs=ex_word(a,k);exact_scratch[r.offset+(k)]=(lhs-rhs)&65535u;borrow=select(0u,1u,lhs<rhs);}
 if borrow!=0u {return ex_bad();}return ex_trim(r);
}
fn ex_add(a:ExInt,b:ExInt)->ExInt {
 if (a.valid&b.valid)==0u {return ex_bad();}var r:ExInt;
 if a.negative==b.negative {r=ex_add_abs(a,b);r.negative=a.negative;}
 else {let order=ex_cmp_abs(a,b);if order>=0 {r=ex_sub_abs(a,b);r.negative=a.negative;}else {r=ex_sub_abs(b,a);r.negative=b.negative;}}
 return ex_trim(r);
}
fn ex_sub(a:ExInt,b:ExInt)->ExInt {return ex_add(a,ex_neg(b));}
fn ex_mul(a:ExInt,b:ExInt)->ExInt {
 if (a.valid&b.valid)==0u {return ex_bad();}var r=ex_zero();if r.valid==0u {return r;}if a.length==0u||b.length==0u {return r;}
 for(var k=0u;k<min(EX_WORDS,a.length+b.length);k+=1u) {exact_scratch[r.offset+k]=0u;}
 for(var i=0u;i<a.length;i+=1u) {
  let left=ex_word(a,i);if left==0u {continue;}
  var carry=0u;
  for(var j=0u;j<b.length;j+=1u) {let k=i+j;if k>=EX_WORDS {return ex_bad();}let right=ex_word(b,j);if right==0u&&carry==0u {continue;}let value=left*right+ex_raw(r,k)+carry;exact_scratch[r.offset+(k)]=value&65535u;carry=value>>16u;}
  var k=i+b.length;while carry!=0u {if k>=EX_WORDS {return ex_bad();}let value=ex_raw(r,k)+carry;exact_scratch[r.offset+(k)]=value&65535u;carry=value>>16u;k+=1u;}
 }
 r.length=min(EX_WORDS,a.length+b.length);r.negative=a.negative^b.negative;return ex_trim(r);
}
fn ex_shl(a:ExInt,bits:u32)->ExInt {
 if a.valid==0u {return ex_bad();}if a.length==0u {return a;}
 let whole=bits/16u;let part=bits%16u;if whole>=EX_WORDS {return ex_bad();}var r=ex_zero();if r.valid==0u {return r;}var carry=0u;
 for(var k=0u;k<whole;k+=1u) {exact_scratch[r.offset+k]=0u;}
 for(var i=0u;i<a.length;i+=1u) {let k=i+whole;if k>=EX_WORDS {return ex_bad();}let value=(ex_word(a,i)<<part)|carry;exact_scratch[r.offset+(k)]=value&65535u;carry=value>>16u;}
 let end=a.length+whole;if carry!=0u {if end>=EX_WORDS {return ex_bad();}exact_scratch[r.offset+(end)]=carry;}
 r.length=min(EX_WORDS,end+select(0u,1u,carry!=0u));r.negative=a.negative;return ex_trim(r);
}
// Unit 2^base. f32 is decoded entirely as integer bits, including subnormals.
fn ex_float(bits:u32,base:i32)->ExInt {
 let exponent=(bits>>23u)&255u;let fraction=bits&8388607u;if exponent==255u {return ex_bad();}
 var mantissa=fraction;var power=-149;if exponent!=0u {mantissa|=8388608u;power=i32(exponent)-150;}
 if mantissa==0u {return ex_zero();}if power<base {return ex_bad();}
 var r=ex_zero();if r.valid==0u {return r;}exact_scratch[r.offset+(0)]=mantissa&65535u;exact_scratch[r.offset+(1)]=mantissa>>16u;r.length=select(1u,2u,ex_raw(r,1)!=0u);r.negative=bits>>31u;
 return ex_shl(r,u32(power-base));
}
fn ex_vec_valid(v:ExVec)->u32 {return v.x.valid&v.y.valid&v.z.valid;}
// Exact product of the original f32 bit patterns, with no floating operations.
// A 24x24-bit mantissa product has at most three radix-16 digits. Embed these
// directly at the requested integer unit instead of multiplying sparse arenas.
fn ex_float_product(a:u32,b:u32,base:i32)->ExInt {
 let ae=(a>>23u)&255u;let be=(b>>23u)&255u;
 if ae==255u||be==255u {return ex_bad();}
 var am=a&8388607u;var bm=b&8388607u;var ap=-149;var bp=-149;
 if ae!=0u {am|=8388608u;ap=i32(ae)-150;}
 if be!=0u {bm|=8388608u;bp=i32(be)-150;}
 if am==0u||bm==0u {return ex_zero();}
 let power=ap+bp;if power<base {return ex_bad();}
 let a0=am&65535u;let a1=am>>16u;let b0=bm&65535u;let b1=bm>>16u;
 let low=a0*b0;let middle=a0*b1+a1*b0+(low>>16u);
 let high=a1*b1+(middle>>16u);
 let shift=u32(power-base);let whole=shift/16u;let part=shift%16u;
 let p0=low&65535u;let p1=middle&65535u;
 let w0=(p0<<part)&65535u;
 let w1=((p1<<part)|(p0>>(16u-part)))&65535u;
 let w2=((high<<part)|(p1>>(16u-part)))&65535u;
 let w3=high>>(16u-part);
 var digits=1u;if w1!=0u {digits=2u;}if w2!=0u {digits=3u;}if w3!=0u {digits=4u;}
 if whole>=EX_WORDS||digits>EX_WORDS-whole {return ex_bad();}
 var result=ex_zero();if result.valid==0u {return result;}
 for(var k=0u;k<whole;k+=1u) {exact_scratch[result.offset+k]=0u;}
 exact_scratch[result.offset+whole]=w0;
 if digits>1u {exact_scratch[result.offset+whole+1u]=w1;}
 if digits>2u {exact_scratch[result.offset+whole+2u]=w2;}
 if digits>3u {exact_scratch[result.offset+whole+3u]=w3;}
 result.length=whole+digits;result.negative=(a^b)>>31u;return result;
}
fn ex_vec_zero(v:ExVec)->bool {return v.x.length==0u&&v.y.length==0u&&v.z.length==0u;}
fn ex_vsub(a:ExVec,b:ExVec)->ExVec {return ExVec(ex_sub(a.x,b.x),ex_sub(a.y,b.y),ex_sub(a.z,b.z));}
fn ex_vneg(a:ExVec)->ExVec {return ExVec(ex_neg(a.x),ex_neg(a.y),ex_neg(a.z));}
fn ex_cross(a:ExVec,b:ExVec)->ExVec {return ExVec(ex_sub(ex_mul(a.y,b.z),ex_mul(a.z,b.y)),ex_sub(ex_mul(a.z,b.x),ex_mul(a.x,b.z)),ex_sub(ex_mul(a.x,b.y),ex_mul(a.y,b.x)));}
fn ex_dot(a:ExVec,b:ExVec)->ExInt {return ex_add(ex_add(ex_mul(a.x,b.x),ex_mul(a.y,b.y)),ex_mul(a.z,b.z));}
fn ex_component(a:ExVec,k:u32)->ExInt {if k==0u {return a.x;}if k==1u {return a.y;}return a.z;}
fn ex_nonzero_axis(a:ExVec)->u32 {if a.x.length!=0u {return 0u;}if a.y.length!=0u {return 1u;}return 2u;}
fn ex_unresolved()->ExDecision {return ExDecision(2u,64u);}
fn ex_truth(hit:bool)->ExDecision {return ExDecision(select(0u,1u,hit),0u);}
fn ex_one()->ExInt {var r=ex_zero();if r.valid!=0u {exact_scratch[r.offset]=1u;r.length=1u;}return r;}
fn ex_set_parameters(t:ExInt,u:ExInt,v:ExInt,den:ExInt) {ex_parameters=ExParameters(t,u,v,den,t.valid&u.valid&v.valid&den.valid);}
struct ExProductDigit {digit:u32,carry:u32}
struct ExOrdering {order:i32,valid:u32}
// At most 128 products per diagonal: the emulated accumulator is <2^39.
fn ex_low_word(a:ExInt)->u32 {var k=0u;while k<a.length {if ex_word(a,k)!=0u {break;}k+=1u;}return k;}
fn ex_product_digit(a:ExInt,b:ExInt,k:u32,carry:u32,a_low:u32,b_low:u32)->ExProductDigit {
 var lo=carry;var hi=0u;
 if a_low<a.length&&b_low<b.length&&k>=a_low+b_low {
  var first=a_low;if k>=b.length {first=max(first,k-b.length+1u);}
  let last=min(a.length-1u,k-b_low);
  for(var i=first;i<=last;i+=1u) {
   let product=ex_word(a,i)*ex_word(b,k-i);let sum=lo+product;hi+=select(0u,1u,sum<lo);lo=sum;
  }
 }
 return ExProductDigit(lo&65535u,(lo>>16u)|(hi<<16u));
}
// Stream every nonzero-capable product digit, including all high carries. Only
// algebraically zero low diagonals/terms are omitted; capacity is not truncated.
fn ex_cmp_products(a:ExInt,b:ExInt,c:ExInt,d:ExInt)->ExOrdering {
 if (a.valid&b.valid&c.valid&d.valid)==0u||a.negative!=0u||b.negative!=0u||c.negative!=0u||d.negative!=0u {return ExOrdering(0,0u);}
 let al=ex_low_word(a);let bl=ex_low_word(b);let cl=ex_low_word(c);let dl=ex_low_word(d);
 var ac=0u;var bc=0u;var order=0;
 for(var k=min(al+bl,cl+dl);k<max(a.length+b.length,c.length+d.length);k+=1u) {
  let left=ex_product_digit(a,b,k,ac,al,bl);let right=ex_product_digit(c,d,k,bc,cl,dl);ac=left.carry;bc=right.carry;
  if left.digit<right.digit {order=-1;}else if left.digit>right.digit {order=1;}
 }
 return ExOrdering(order,select(0u,1u,ac==0u&&bc==0u));
}
// Relative A and edge B have unit 2^-298, direction D has unit 2^-149.
fn ex_segment(a:ExVec,b:ExVec,d:ExVec)->ExDecision {
 if (ex_vec_valid(a)&ex_vec_valid(b)&ex_vec_valid(d))==0u {return ex_unresolved();}
 if ex_vec_zero(d) {
  if ex_vec_zero(b) {let hit=ex_vec_zero(a);if hit {let z=ex_zero();ex_set_parameters(z,z,z,ex_one());}return ex_truth(hit);}
  let axis=ex_nonzero_axis(b);var den=ex_component(b,axis);var s=ex_neg(ex_component(a,axis));
  if den.negative!=0u {den=ex_neg(den);s=ex_neg(s);}if s.negative!=0u||ex_cmp(s,den)>0 {return ex_truth(false);}
  for(var k=0u;k<3u;k+=1u) {let r=ex_add(ex_mul(ex_component(a,k),den),ex_mul(ex_component(b,k),s));if r.valid==0u {return ex_unresolved();}if r.length!=0u {return ex_truth(false);}}
  let z=ex_zero();ex_set_parameters(z,s,z,den);return ex_truth(true);
 }
 let db=ex_cross(d,b);if ex_vec_valid(db)==0u {return ex_unresolved();}
 if !ex_vec_zero(db) {
  let axis=ex_nonzero_axis(db);var den=ex_component(db,axis);
  let ab=ex_cross(a,b);let ad=ex_cross(a,d);if (ex_vec_valid(ab)&ex_vec_valid(ad))==0u {return ex_unresolved();}
  var t=ex_component(ab,axis);var s=ex_component(ad,axis);
  if den.negative!=0u {den=ex_neg(den);t=ex_neg(t);s=ex_neg(s);}
  let far_den=ex_shl(den,149u);if far_den.valid==0u {return ex_unresolved();}
  if t.negative!=0u||s.negative!=0u||ex_cmp(t,far_den)>0||ex_cmp(s,den)>0 {return ex_truth(false);}
  for(var k=0u;k<3u;k+=1u) {
   let lhs=ex_mul(ex_component(d,k),t);let rhs=ex_add(ex_mul(ex_component(a,k),den),ex_mul(ex_component(b,k),s));
   if (lhs.valid&rhs.valid)==0u {return ex_unresolved();}if ex_cmp(lhs,rhs)!=0 {return ex_truth(false);}
  }
  ex_set_parameters(t,s,ex_zero(),den);return ex_truth(true);
 }
 let ad=ex_cross(a,d);if ex_vec_valid(ad)==0u {return ex_unresolved();}if !ex_vec_zero(ad) {return ex_truth(false);}
 let axis=ex_nonzero_axis(d);var den=ex_component(d,axis);var first=ex_component(a,axis);var last=ex_add(first,ex_component(b,axis));
 if last.valid==0u {return ex_unresolved();}if den.negative!=0u {den=ex_neg(den);first=ex_neg(first);last=ex_neg(last);}
 var lo=first;var hi=last;if ex_cmp(lo,hi)>0 {lo=last;hi=first;}
 let far_den=ex_shl(den,149u);if far_den.valid==0u {return ex_unresolved();}
 let hit=hi.negative==0u&&ex_cmp(lo,far_den)<=0;if !hit {return ex_truth(false);}
 let z=ex_zero();
 if lo.negative==0u {var s=z;if ex_cmp(lo,first)!=0 {s=den;}ex_set_parameters(lo,s,z,den);}
 else if ex_vec_zero(b) {ex_set_parameters(z,z,z,den);}
 else {var sd=ex_component(b,axis);var sn=ex_neg(ex_component(a,axis));if sd.negative!=0u {sd=ex_neg(sd);sn=ex_neg(sn);}ex_set_parameters(z,sn,z,sd);}
 return ex_truth(true);
}
fn ex_edges(a:ExVec,b:ExVec,c:ExVec,d:ExVec)->ExDecision {
 ex_parameters.valid=0u;let r0=ex_segment(a,ex_vsub(b,a),d);let p0=ex_parameters;
 ex_parameters.valid=0u;let r1=ex_segment(b,ex_vsub(c,b),d);var p1=ex_parameters;
 if r1.state==1u {p1.v=p1.u;p1.u=ex_sub(p1.denominator,p1.v);p1.valid&=p1.u.valid;}
 ex_parameters.valid=0u;let r2=ex_segment(c,ex_vsub(a,c),d);var p2=ex_parameters;
 if r2.state==1u {p2.v=ex_sub(p2.denominator,p2.u);p2.u=ex_zero();p2.valid&=p2.u.valid&p2.v.valid;}
 if r0.state==2u||r1.state==2u||r2.state==2u {ex_parameters.valid=0u;return ex_unresolved();}
 var best=p0;var hit=r0.state==1u;
 if r1.state==1u {if !hit {best=p1;hit=true;}else {let cmp=ex_cmp_products(p1.t,best.denominator,best.t,p1.denominator);if cmp.valid==0u {return ex_unresolved();}if cmp.order<0 {best=p1;}}}
 if r2.state==1u {if !hit {best=p2;hit=true;}else {let cmp=ex_cmp_products(p2.t,best.denominator,best.t,p2.denominator);if cmp.valid==0u {return ex_unresolved();}if cmp.order<0 {best=p2;}}}
 ex_parameters=best;if hit&&best.valid==0u {return ex_unresolved();}return ex_truth(hit);
}
// Exact closed triangle/segment/point existence in t in [0,1]. Parameters and
// nearest-candidate ordering are separate work; this is an exact decision pass.
fn ex_triangle_regular_scaled(a:ExVec,d:ExVec,e1:ExVec,e2:ExVec,p:ExVec,determinant:ExInt,scale_delta:i32)->ExDecision {
 var det=determinant;
  let delta=ex_vneg(a);let q=ex_cross(delta,e1);var u=ex_dot(delta,p);var v=ex_dot(d,q);var t=ex_dot(e2,q);
  if (u.valid&v.valid&t.valid)==0u {return ex_unresolved();}
  if det.negative!=0u {det=ex_neg(det);u=ex_neg(u);v=ex_neg(v);t=ex_neg(t);}
  let sum=ex_add(u,v);var far_den=det;var far_t=t;
  if scale_delta<=149 {far_den=ex_shl(det,u32(149-scale_delta));}
  else {far_t=ex_shl(t,u32(scale_delta-149));}
  if (sum.valid&far_den.valid&far_t.valid)==0u {return ex_unresolved();}
  let hit=u.negative==0u&&v.negative==0u&&t.negative==0u&&ex_cmp(sum,det)<=0&&ex_cmp(far_t,far_den)<=0;
  if hit {
   // Restore the public t exponent -149. The four saved operands differ from
   // original operands only by ONE common exact positive power of two.
   if scale_delta>0 {t=ex_shl(t,u32(scale_delta));}
   else if scale_delta<0 {let shift=u32(-scale_delta);u=ex_shl(u,shift);v=ex_shl(v,shift);det=ex_shl(det,shift);}
   if (t.valid&u.valid&v.valid&det.valid)==0u {return ex_unresolved();}
   ex_parameters=ExParameters(t,u,v,det,1u);
  }
  return ex_truth(hit);
}
fn ex_triangle_regular(a:ExVec,d:ExVec,e1:ExVec,e2:ExVec,p:ExVec,determinant:ExInt)->ExDecision {
 return ex_triangle_regular_scaled(a,d,e1,e2,p,determinant,0);
}
const EX_NORMALIZE_CERTIFIED:bool=true;
fn ex_cert_low(a:ExInt)->u32 {if a.length==0u {return EX_WORDS;}return ex_low_word(a);}
fn ex_cert_vec_low(v:ExVec)->u32 {return min(ex_cert_low(v.x),min(ex_cert_low(v.y),ex_cert_low(v.z)));}
fn ex_cert_view(a:ExInt,digits:u32)->ExInt {var r=a;if r.length!=0u {r.offset+=digits;r.length-=digits;}return r;}
fn ex_cert_vec_view(v:ExVec,digits:u32)->ExVec {return ExVec(ex_cert_view(v.x,digits),ex_cert_view(v.y,digits),ex_cert_view(v.z,digits));}
// This path still checks the original integer predicate. A zero determinant
// invalidates the strict source certificate and requests complete fallback.
fn ex_triangle_certified(a:ExVec,b:ExVec,c:ExVec,d:ExVec)->ExDecision {
 ex_triangle_facing=0u;
 if (ex_vec_valid(a)&ex_vec_valid(b)&ex_vec_valid(c)&ex_vec_valid(d))==0u {return ex_unresolved();}
 var av=a;var bv=b;var cv=c;var dv=d;var scale_delta=0;
 if EX_NORMALIZE_CERTIFIED {
  var positions=min(ex_cert_vec_low(a),min(ex_cert_vec_low(b),ex_cert_vec_low(c)));
  var direction=ex_cert_vec_low(d);
  if positions==EX_WORDS {positions=0u;}if direction==EX_WORDS {direction=0u;}
  av=ex_cert_vec_view(a,positions);bv=ex_cert_vec_view(b,positions);cv=ex_cert_vec_view(c,positions);dv=ex_cert_vec_view(d,direction);
  scale_delta=16*(i32(positions)-i32(direction));
 }
 let e1=ex_vsub(bv,av);let e2=ex_vsub(cv,av);let p=ex_cross(dv,e2);let det=ex_dot(e1,p);
 if det.valid==0u||det.length==0u {return ex_unresolved();}
 ex_triangle_facing=select(1u,2u,det.negative!=0u);
 return ex_triangle_regular_scaled(av,dv,e1,e2,p,det,scale_delta);
}
fn ex_triangle(a:ExVec,b:ExVec,c:ExVec,d:ExVec)->ExDecision {
 ex_triangle_facing=0u;
 if (ex_vec_valid(a)&ex_vec_valid(b)&ex_vec_valid(c)&ex_vec_valid(d))==0u {return ex_unresolved();}
 let e1=ex_vsub(b,a);let e2=ex_vsub(c,a);let p=ex_cross(d,e2);var det=ex_dot(e1,p);
 if det.valid==0u {return ex_unresolved();}
 if det.length!=0u {
  ex_triangle_facing=select(1u,2u,det.negative!=0u);
  return ex_triangle_regular(a,d,e1,e2,p,det);
 }
 let normal=ex_cross(e1,e2);if ex_vec_valid(normal)==0u {return ex_unresolved();}
 if ex_vec_zero(normal) {return ex_edges(a,b,c,d);}
 let plane=ex_dot(normal,a);if plane.valid==0u {return ex_unresolved();}if plane.length!=0u {return ex_truth(false);}
 // Coplanar: origin inside the projected triangle, or a ray/edge intersection.
 let drop=ex_nonzero_axis(normal);var i=0u;var j=1u;if drop==0u {i=1u;j=2u;}else if drop==1u {i=0u;j=2u;}
 var area=ex_sub(ex_mul(ex_component(e1,i),ex_component(e2,j)),ex_mul(ex_component(e1,j),ex_component(e2,i)));
 let delta=ex_vneg(a);
 var u=ex_sub(ex_mul(ex_component(delta,i),ex_component(e2,j)),ex_mul(ex_component(delta,j),ex_component(e2,i)));
 var v=ex_sub(ex_mul(ex_component(e1,i),ex_component(delta,j)),ex_mul(ex_component(e1,j),ex_component(delta,i)));
 if (area.valid&u.valid&v.valid)==0u {return ex_unresolved();}if area.negative!=0u {area=ex_neg(area);u=ex_neg(u);v=ex_neg(v);}
 let sum=ex_add(u,v);if sum.valid==0u {return ex_unresolved();}
 if u.negative==0u&&v.negative==0u&&ex_cmp(sum,area)<=0 {ex_set_parameters(ex_zero(),u,v,area);return ex_truth(true);}
 return ex_edges(a,b,c,d);
}

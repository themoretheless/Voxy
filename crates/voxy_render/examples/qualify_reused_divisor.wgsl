override ARCHIVE_DDA_REUSED_DIVISOR:bool=true;
fn dda_reuse_product64(a:u32,b:u32)->vec2<u32> {
 let low=(a&65535u)*(b&65535u);
 let mid=(a>>16u)*(b&65535u)+(low>>16u);
 let next=(a&65535u)*(b>>16u)+(mid&65535u);
 return vec2<u32>((next<<16u)|(low&65535u),(a>>16u)*(b>>16u)+(mid>>16u)+(next>>16u));
}
fn dda_reuse_power64(e:u32)->vec2<u32> {if e<32u {return vec2<u32>(1u<<e,0u);}return vec2<u32>(0u,1u<<(e-32u));}
fn dda_reuse_less64(a:vec2<u32>,b:vec2<u32>)->bool {return a.y<b.y||(a.y==b.y&&a.x<b.x);}
// Exact integer comparison against the two rounding midpoints. A normal
// correctly rounded reciprocal is certified without assuming GPU division accuracy.
fn dda_reuse_reciprocal_certificate(y:u32,z:u32)->bool {
 let ey=(y>>23u)&255u;let ez=(z>>23u)&255u;
 if ey==0u||ey==255u||ez==0u||ez==255u||((y^z)&0x80000000u)!=0u {return false;}
 let my=(y&8388607u)|8388608u;let mz=(z&8388607u)|8388608u;
 let asymmetric=mz==8388608u&&ez>1u;
 let upper_e=301i-i32(ey)-i32(ez);let lower_e=upper_e+select(0i,1i,asymmetric);
 if upper_e<0i||upper_e>63i||lower_e<0i||lower_e>63i {return false;}
 let upper=dda_reuse_product64(my,2u*mz+1u);
 let lower=dda_reuse_product64(my,select(2u*mz-1u,4u*mz-1u,asymmetric));
 let ut=dda_reuse_power64(u32(upper_e));let lt=dda_reuse_power64(u32(lower_e));let even=(mz&1u)==0u;
 return (dda_reuse_less64(lower,lt)||(all(lower==lt)&&even))&&(dda_reuse_less64(ut,upper)||(all(upper==ut)&&even));
}
// A zero sentinel encodes an unsupported reciprocal; admitted reciprocals
// are normal and nonzero. Only one float is retained per ray axis.
alias dda_reuse_Prepared=f32;
fn dda_reuse_prepare(y:f32)->dda_reuse_Prepared {
 let ey=(bitcast<u32>(y)>>23u)&255u;if ey<68u||ey>190u {return 0.;}
 let z=1./y;
 if !dda_reuse_reciprocal_certificate(bitcast<u32>(y),bitcast<u32>(z)) {return 0.;}
 return z;
}
fn dda_reuse_quotient(x:f32,y:f32,z:f32)->f32 {
 let q=projection_mul_scalar(x,z);let residual=fma(-q,y,x);return fma(residual,z,q);
}
fn dda_reuse_pair_or_original(a:vec2<f32>,b:vec2<f32>,z:dda_reuse_Prepared)->vec2<f32> {
 if !ARCHIVE_DDA_REUSED_DIVISOR||z==0. {return projection_pair_div(a,b);}
 let words=bitcast<vec4<u32>>(vec4<f32>(a,b));
 if any((words&vec4<u32>(0x7f800000u))==vec4<u32>(0x7f800000u)) {return projection_pair_div(a,b);}
 let ex=(words.x>>23u)&255u;
 if ex<68u||ex>190u {return projection_pair_div(a,b);}
 // Exponents 68..190 keep quotient, product and residual normal (or exact
 // zero), so the theorem's no-overflow/no-underflow requirements hold.
 let q=vec2<f32>(dda_reuse_quotient(a.x,b.x,z),0.);
 let remainder=projection_pair_add(a,-projection_pair_mul(q,b));
 let x=projection_add_scalar(remainder.x,remainder.y);let e=(bitcast<u32>(x)>>23u)&255u;
 var correction=0.;
 if e>=68u&&e<=190u {correction=dda_reuse_quotient(x,b.x,z);}else{correction=x/b.x;}
 return projection_pair_add(q,vec2<f32>(correction,0.));
}

// Isolated repeated-divisor experiment. No production pipeline includes this file.
struct Case {words:vec4<u32>,low:vec4<u32>}
@group(0) @binding(0) var<storage,read> cases:array<Case>;
@group(0) @binding(1) var<storage,read_write> results:array<vec4<u32>>;
fn product64(a:u32,b:u32)->vec2<u32> {
 let low=(a&65535u)*(b&65535u);
 let mid=(a>>16u)*(b&65535u)+(low>>16u);
 let next=(a&65535u)*(b>>16u)+(mid&65535u);
 return vec2<u32>((next<<16u)|(low&65535u),(a>>16u)*(b>>16u)+(mid>>16u)+(next>>16u));
}
fn power64(e:u32)->vec2<u32> {if e<32u {return vec2<u32>(1u<<e,0u);}return vec2<u32>(0u,1u<<(e-32u));}
fn less64(a:vec2<u32>,b:vec2<u32>)->bool {return a.y<b.y||(a.y==b.y&&a.x<b.x);}
// Exact integer comparison against the two rounding midpoints. A normal
// correctly rounded reciprocal is certified without assuming GPU division accuracy.
fn reciprocal_certificate(y:u32,z:u32)->bool {
 let ey=(y>>23u)&255u;let ez=(z>>23u)&255u;
 if ey==0u||ey==255u||ez==0u||ez==255u||((y^z)&0x80000000u)!=0u {return false;}
 let my=(y&8388607u)|8388608u;let mz=(z&8388607u)|8388608u;
 let asymmetric=mz==8388608u&&ez>1u;
 let upper_e=301i-i32(ey)-i32(ez);let lower_e=upper_e+select(0i,1i,asymmetric);
 if upper_e<0i||upper_e>63i||lower_e<0i||lower_e>63i {return false;}
 let upper=product64(my,2u*mz+1u);
 let lower=product64(my,select(2u*mz-1u,4u*mz-1u,asymmetric));
 let ut=power64(u32(upper_e));let lt=power64(u32(lower_e));let even=(mz&1u)==0u;
 return (less64(lower,lt)||(all(lower==lt)&&even))&&(less64(ut,upper)||(all(upper==ut)&&even));
}
struct Prepared {reciprocal:f32,certified:bool}
fn prepare(y:f32,forced:u32,use_forced:bool)->Prepared {
 let reciprocal=select(1./y,bitcast<f32>(forced),use_forced);
 return Prepared(reciprocal,reciprocal_certificate(bitcast<u32>(y),bitcast<u32>(reciprocal)));
}
struct Quotient {value:f32,fast:bool}
fn corrected_divide(x:f32,y:f32,p:Prepared)->Quotient {
 let ex=(bitcast<u32>(x)>>23u)&255u;let ey=(bitcast<u32>(y)>>23u)&255u;
 // Restrict the proof domain: no tiny/scaled operands or overflow/underflow.
 // The original division handles every unadmitted or exceptional input.
 if !p.certified||ex<68u||ex>190u||ey<68u||ey>190u {return Quotient(x/y,false);}
 let q=projection_mul_scalar(x,p.reciprocal);
 let qe=(bitcast<u32>(q)>>23u)&255u;
 if qe==0u||qe==255u {return Quotient(x/y,false);}
 let residual=fma(-q,y,x);
 let re=(bitcast<u32>(residual)>>23u)&255u;
 if (re==0u&&(bitcast<u32>(residual)&0x7fffffffu)!=0u)||re==255u {return Quotient(x/y,false);}
 let corrected=fma(residual,p.reciprocal,q);
 let ce=(bitcast<u32>(corrected)>>23u)&255u;
 if ce==0u||ce==255u {return Quotient(x/y,false);}
 return Quotient(corrected,true);
}
struct PairQuotient {value:vec2<f32>,main_fast:bool,correction_fast:bool}
fn corrected_pair(a:vec2<f32>,b:vec2<f32>,p:Prepared)->PairQuotient {
 if any((bitcast<vec2<u32>>(a)&vec2<u32>(0x7f800000u))==vec2<u32>(0x7f800000u))||any((bitcast<vec2<u32>>(b)&vec2<u32>(0x7f800000u))==vec2<u32>(0x7f800000u)) {return PairQuotient(projection_pair_div(a,b),false,false);}
 let main=corrected_divide(a.x,b.x,p);
 if !main.fast {return PairQuotient(projection_pair_div(a,b),false,false);}
 let q=vec2<f32>(main.value,0.);
 let remainder=projection_pair_add(a,-projection_pair_mul(q,b));
 let correction=corrected_divide(projection_add_scalar(remainder.x,remainder.y),b.x,p);
 return PairQuotient(projection_pair_add(q,vec2<f32>(correction.value,0.)),true,correction.fast);
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&cases) {return;}
 let c=cases[id.x];let x=bitcast<f32>(c.words.x);let y=bitcast<f32>(c.words.y);
 let p=prepare(y,c.words.z,c.words.w!=0u);let original=x/y;let actual=corrected_divide(x,y,p);
 let a=vec2<f32>(x,bitcast<f32>(c.low.x));let b=vec2<f32>(y,bitcast<f32>(c.low.y));
 let pair_original=projection_pair_div(a,b);let pair_actual=corrected_pair(a,b,p);
 let by=(c.words.y>>23u)&255u;let lean_reciprocal=select(0.,p.reciprocal,p.certified&&by>=68u&&by<=190u);
 let lean_pair=dda_reuse_pair_or_original(a,b,lean_reciprocal);
 results[id.x*4u]=vec4<u32>(bitcast<u32>(original),bitcast<u32>(actual.value),bitcast<u32>(p.reciprocal),select(0u,1u,p.certified));
 results[id.x*4u+1u]=vec4<u32>(bitcast<vec2<u32>>(pair_original),bitcast<vec2<u32>>(lean_pair));
 results[id.x*4u+2u]=vec4<u32>(select(0u,1u,actual.fast),select(0u,1u,pair_actual.main_fast),select(0u,1u,pair_actual.correction_fast),id.x);
 let left=(c.words.x&0x1ffffffu);let right=(c.words.y&0xffffffu);
 results[id.x*4u+3u]=vec4<u32>(product64(left,right),left,right);
}

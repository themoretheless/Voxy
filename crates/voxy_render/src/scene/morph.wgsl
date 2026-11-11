// Register-only compensated floating-point (Dekker/Neumaier two_prod + two_sum)
// morph target evaluation. Eliminates the 576-bit (36x16-bit) stack accumulator
// and world_exact_fused_affine dependency while preserving large-cancellation
// precision (e.g. 1e30 + 1.0 - 1e30 == 1.0) in 2 scalar registers.
@group(0) @binding(5) var<storage,read> morph_deltas:array<u32>;
@group(0) @binding(6) var<storage,read> morph_weights:array<u32>;

fn morph_two_sum(a:f32,b:f32)->vec2<f32> {
 let s=a+b;
 let v=s-a;
 let e=(a-(s-v))+(b-v);
 return vec2<f32>(s,e);
}

fn morph_split(a:f32)->vec2<f32> {
 let c=4097.0*a;
 let hi=c-(c-a);
 let lo=a-hi;
 return vec2<f32>(hi,lo);
}

fn morph_two_prod(a:f32,b:f32)->vec2<f32> {
 let p=a*b;
 let sa=morph_split(a);
 let sb=morph_split(b);
 let e=((sa.x*sb.x-p)+sa.x*sb.y+sa.y*sb.x)+sa.y*sb.y;
 return vec2<f32>(p,e);
}

fn morph_add_expansion(acc:vec2<f32>,term:vec2<f32>)->vec2<f32> {
 let s=morph_two_sum(acc.x,term.x);
 return vec2<f32>(s.x,acc.y+s.y+term.y);
}

fn morph_component(vertex:u32,component:u32)->f32 {
 var acc=vec2<f32>(source[vertex*16u+component],0.0);
 for(var shape=0u;shape<parameters.pad0;shape++) {
  let delta=bitcast<f32>(morph_deltas[(shape*parameters.count+vertex)*6u+component]);
  let weight=bitcast<f32>(morph_weights[shape]);
  if delta!=0.0 && weight!=0.0 {
   acc=morph_add_expansion(acc,morph_two_prod(delta,weight));
  }
 }
 let result=acc.x+acc.y;
 return select(result,0.0,result==0.0);
}

fn morph_component_bits(vertex:u32,component:u32)->u32 {
 return bitcast<u32>(morph_component(vertex,component));
}


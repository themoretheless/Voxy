fn exact_copy_parameter(a:ExInt,index:u32)->ExInt {
 var r=a;r.offset=ex_begin+index*EX_WORDS;
 for(var k=0u;k<EX_WORDS;k+=1u) {exact_scratch[r.offset+k]=ex_word(a,k);}return r;
}
fn exact_axis(slot:u32,vertex:u32,axis:u32,origin_hi:u32,origin_lo:u32)->ExInt {
 var value=ex_float(exact_instances[slot*28u+12u+axis],-298);
 for(var k=0u;k<3u;k+=1u) {
  let product=ex_float_product(exact_instances[slot*28u+k*4u+axis],exact_vertices[vertex*12u+k],-298);
  value=ex_add(value,product);
 }
 value=ex_sub(value,ex_float(origin_hi,-298));return ex_sub(value,ex_float(origin_lo,-298));
}
fn exact_vertex(slot:u32,vertex:u32,ray:u32)->ExVec {
 return ExVec(exact_axis(slot,vertex,0u,exact_ray_word(ray),exact_ray_word(ray+4u)),exact_axis(slot,vertex,1u,exact_ray_word(ray+1u),exact_ray_word(ray+5u)),exact_axis(slot,vertex,2u,exact_ray_word(ray+2u),exact_ray_word(ray+6u)));
}
fn exact_process(input_index:u32,output_index:u32,stride:u32,parameters:bool) {
 let ray=input_index*20u;if ray+19u>=exact_ray_word_count() {return;}
 ex_cursor=select(0u,4u,parameters);ex_peak=ex_cursor;ex_parameters.valid=0u;
 exact_output[output_index*stride]=vec4<u32>(2u,128u,2048u,0u);
 exact_output[output_index*stride+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),exact_ray_word(ray+16u),exact_ray_word(ray+17u));

 let slot=exact_ray_word(ray+16u);let primitive=exact_ray_word(ray+17u);let info=exact_layout[slot];let first=info.z+primitive*3u;
 let a=exact_vertex(slot,info.y+exact_indices[first],ray);let b=exact_vertex(slot,info.y+exact_indices[first+1u],ray);let c=exact_vertex(slot,info.y+exact_indices[first+2u],ray);
 let d=ExVec(ex_add(ex_float(exact_ray_word(ray+8u),-149),ex_float(exact_ray_word(ray+12u),-149)),ex_add(ex_float(exact_ray_word(ray+9u),-149),ex_float(exact_ray_word(ray+13u),-149)),ex_add(ex_float(exact_ray_word(ray+10u),-149),ex_float(exact_ray_word(ray+14u),-149)));
 let decision=ex_triangle(a,b,c,d);
 if parameters {
  var t=ExPairBits(0u,0u,0u,0u);var u=t;var v=t;
  if ex_parameters.valid!=0u {
   let pt=exact_copy_parameter(ex_parameters.t,0u);let pu=exact_copy_parameter(ex_parameters.u,1u);let pv=exact_copy_parameter(ex_parameters.v,2u);let pd=exact_copy_parameter(ex_parameters.denominator,3u);
   ex_parameters=ExParameters(pt,pu,pv,pd,pt.valid&pu.valid&pv.valid&pd.valid);ex_cursor=4u;
   let mark=ex_cursor;
   t=ex_ratio_pair(ex_parameters.t,ex_parameters.denominator,-149);ex_cursor=mark;
   u=ex_ratio_pair(ex_parameters.u,ex_parameters.denominator,0);ex_cursor=mark;
   v=ex_ratio_pair(ex_parameters.v,ex_parameters.denominator,0);ex_cursor=mark;
  }
  let valid=t.valid&u.valid&v.valid;
  exact_output[output_index*stride+2u]=vec4<u32>(t.hi,t.lo,u.hi,u.lo);
  exact_output[output_index*stride+3u]=vec4<u32>(v.hi,v.lo,valid,t.residual_nonzero|(u.residual_nonzero<<1u)|(v.residual_nonzero<<2u));
 }
 exact_output[output_index*stride]=vec4<u32>(decision.state,decision.reason,2048u,ex_peak);
}

struct ExactWorkQueue {count:atomic<u32>,ids:array<u32>}
@group(0) @binding(7) var<storage,read_write> exact_work:ExactWorkQueue;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 ex_begin=id.x*EX_ARENA_WORDS;exact_process(id.x,id.x,2u,false);
}
@compute @workgroup_size(1) fn main_queued(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=16u {return;}ex_begin=id.x*EX_ARENA_WORDS;
 let count=atomicLoad(&exact_work.count);if count>arrayLength(&exact_work.ids) {return;}
 for(var index=id.x;index<count;index+=16u) {exact_process(exact_work.ids[index],index,2u,false);}
}

@compute @workgroup_size(1) fn main_parameters(@builtin(global_invocation_id) id:vec3<u32>) {
 ex_begin=id.x*EX_ARENA_WORDS;exact_process(id.x,id.x,4u,true);
}
@compute @workgroup_size(1) fn main_queued_parameters(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=16u {return;}ex_begin=id.x*EX_ARENA_WORDS;
 let count=atomicLoad(&exact_work.count);if count>arrayLength(&exact_work.ids) {return;}
 for(var index=id.x;index<count;index+=16u) {exact_process(exact_work.ids[index],index,4u,true);}
}

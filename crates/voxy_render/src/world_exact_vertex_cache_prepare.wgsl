@group(0) @binding(1) var<storage,read> cache_source:array<u32>;
@group(0) @binding(3) var<storage,read> cache_instances:array<u32>;
@group(0) @binding(4) var<storage,read> cache_layout:array<vec4<u32>>;
@group(0) @binding(5) var<storage,read_write> cache_words:array<atomic<u32>>;
struct CacheParameters {values:array<vec4<u32>,2>}
@group(0) @binding(9) var<uniform> cache_parameters:CacheParameters;
fn cache_header()->u32{return cache_parameters.values[0].x;}
fn cache_load(at:u32)->u32{return atomicLoad(&cache_words[at]);}
fn cache_slot(index:u32)->u32 {
 let header=cache_header();var lo=0u;var hi=cache_parameters.values[0].y+1u;
 while lo<hi {let mid=lo+(hi-lo)/2u;if cache_load(header+16u+mid)<=index {lo=mid+1u;}else{hi=mid;}}
 return lo-1u;
}
fn cache_original_axis(slot:u32,vertex:u32,axis:u32)->ExInt {
 var value=ex_float(cache_instances[slot*28u+12u+axis],-298);
 for(var k=0u;k<3u;k++) {value=ex_add(value,ex_float_product(cache_instances[slot*28u+k*4u+axis],cache_source[vertex*12u+k],-298));}
 return value;
}
fn cache_expected(index:u32)->array<u32,12> {
 var result:array<u32,12>;let slot=cache_slot(index);let header=cache_header();
 let vertex=cache_layout[slot].y+index-cache_load(header+16u+slot);
 for(var axis=0u;axis<3u;axis++) {
  ex_cursor=0u;ex_peak=0u;let value=cache_original_axis(slot,vertex,axis);
  if value.valid==0u {var empty:array<u32,12>;return empty;}
  let low=ex_low_word(value);let live=value.length-low;
  if live>6u||low+live>EX_WORDS {var empty:array<u32,12>;return empty;}
  let base=axis*4u;result[base]=32768u|(value.negative<<14u)|(live<<8u)|low;
  for(var word=0u;word<3u;word++) {result[base+1u+word]=ex_word(value,low+word*2u)|(ex_word(value,low+word*2u+1u)<<16u);}
 }
 return result;
}
@compute @workgroup_size(1) fn cs_vertex_cache_prepare(@builtin(global_invocation_id) id:vec3<u32>) {
 let workers=cache_parameters.values[1].x;if id.x>=workers{return;}
 ex_begin=id.x*EX_ARENA_WORDS;
 for(var index=id.x;index<cache_parameters.values[0].z;index+=workers) {
  let expected=cache_expected(index);let base=cache_parameters.values[0].w+index*12u;
  for(var k=0u;k<12u;k++){atomicStore(&cache_words[base+k],expected[k]);}
 }
}
@compute @workgroup_size(1) fn cs_vertex_cache_check(@builtin(global_invocation_id) id:vec3<u32>) {
 let workers=cache_parameters.values[1].x;if id.x>=workers{return;}
 ex_begin=id.x*EX_ARENA_WORDS;
 for(var index=id.x;index<cache_parameters.values[0].z;index+=workers) {
  let expected=cache_expected(index);let base=cache_parameters.values[0].w+index*12u;var different=false;
  for(var k=0u;k<12u;k++){different=different||cache_load(base+k)!=expected[k];}
  if different {atomicAdd(&cache_words[cache_header()+5u],1u);}atomicAdd(&cache_words[cache_header()+6u],1u);
 }
}
@compute @workgroup_size(1) fn cs_vertex_cache_ready() {
 let h=cache_header();let valid=cache_load(h+5u)==0u&&cache_load(h+6u)==cache_parameters.values[0].z;
 atomicStore(&cache_words[h],select(0u,1u,valid));
}

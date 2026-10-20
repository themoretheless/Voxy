@group(0) @binding(0) var<storage,read> audit_input:array<u32>;
@group(0) @binding(5) var<storage,read_write> audit_output:array<u32>;
fn audit_operand(base:u32)->ExInt {
 var value=ex_zero();if value.valid==0u {return value;}
 for(var k=0u;k<128u;k+=1u) {exact_scratch[value.offset+k]=audit_input[base+k];}
 value.length=128u;return ex_trim(value);
}
@compute @workgroup_size(1) fn main_integer_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*512u;if base+511u>=arrayLength(&audit_input) {return;}
 ex_begin=id.x*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 let a=audit_operand(base);let b=audit_operand(base+128u);let c=audit_operand(base+256u);let d=audit_operand(base+384u);
 let product=ex_mul(a,b);let order=ex_cmp_products(a,b,c,d);let out=id.x*132u;
 audit_output[out]=product.valid;audit_output[out+1u]=product.length;audit_output[out+2u]=order.valid;audit_output[out+3u]=bitcast<u32>(order.order);
 for(var k=0u;k<128u;k+=1u) {audit_output[out+4u+k]=ex_word(product,k);}
}

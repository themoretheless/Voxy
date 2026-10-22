@compute @workgroup_size(1) fn main_float_product_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*512u;if base+511u>=arrayLength(&audit_input) {return;}
 ex_begin=id.x*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 let a=audit_input[base]|(audit_input[base+1u]<<16u);
 let b=audit_input[base+2u]|(audit_input[base+3u]<<16u);
 var product:ExInt;
 if audit_input[base+4u]==0u {product=ex_float_product(a,b,-298);}
 else {product=ex_mul(ex_float(a,-149),ex_float(b,-149));}
 let out=id.x*132u;
 audit_output[out]=product.valid;audit_output[out+1u]=product.length;
 audit_output[out+2u]=product.negative;audit_output[out+3u]=ex_peak;
 for(var k=0u;k<128u;k+=1u) {audit_output[out+4u+k]=ex_word(product,k);}
}

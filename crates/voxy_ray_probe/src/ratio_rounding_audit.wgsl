@compute @workgroup_size(1) fn main_ratio_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*512u;if base+511u>=arrayLength(&audit_input) {return;}
 ex_begin=id.x*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 var n=audit_operand(base);let d=audit_operand(base+128u);
 n.negative=audit_input[base+385u];let scale=i32(audit_input[base+384u])-1024;
 let pair=ex_ratio_pair(n,d,scale);let out=id.x*132u;
 audit_output[out]=pair.hi;audit_output[out+1u]=pair.lo;
 audit_output[out+2u]=pair.valid;audit_output[out+3u]=pair.residual_nonzero;
 audit_output[out+4u]=ex_peak;
}

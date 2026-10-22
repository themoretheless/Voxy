@compute @workgroup_size(1) fn main_ratio_seed_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*512u;if base+511u>=arrayLength(&audit_input) {return;}
 ex_begin=id.x*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 var n=audit_operand(base);let d=audit_operand(base+128u);
 n.negative=audit_input[base+385u];let scale=i32(audit_input[base+384u])-1024;let mode=audit_input[base+386u];
 let out=id.x*132u;
 if mode==2u {
  let seed=audit_input[base+387u]|(audit_input[base+388u]<<16u);
  let proof=ex_seed_verify(n,d,scale,seed);audit_output[out]=proof.valid;audit_output[out+1u]=bitcast<u32>(proof.direction);audit_output[out+2u]=ex_seed_initial(n,d,scale);audit_output[out+4u]=ex_peak;return;
 }
 var pair:ExPairBits;
 if mode==0u {pair=ex_ratio_pair(n,d,scale);}
 else {
  let high=ex_round_ratio_seed(n,d,scale);
  if high.valid!=0u {
   let residual=ex_ratio_residual(n,d,scale,high.bits);let low=ex_round_ratio_seed(residual.numerator,d,residual.scale);
   if low.valid!=0u {let tail=ex_ratio_residual(residual.numerator,d,residual.scale,low.bits);pair=ExPairBits(high.bits,low.bits,tail.numerator.valid,select(0u,1u,tail.numerator.length!=0u));}
  }
 }
 audit_output[out]=pair.hi;audit_output[out+1u]=pair.lo;audit_output[out+2u]=pair.valid;audit_output[out+3u]=pair.residual_nonzero;audit_output[out+4u]=ex_peak;
}

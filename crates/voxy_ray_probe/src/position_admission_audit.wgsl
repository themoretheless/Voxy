@compute @workgroup_size(1) fn main_position_admission_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*512u;if base+511u>=arrayLength(&audit_input) {return;}
 ex_begin=id.x*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 let t=audit_operand(base);let den=audit_operand(base+128u);let direction=audit_operand(base+384u);let zero=ex_zero();
 var d=ExVec(direction,zero,zero);let axis=audit_input[base+256u];
 if axis==1u {d=ExVec(zero,direction,zero);}if axis==2u {d=ExVec(zero,zero,direction);}
 let result=ex_history_position_non_dyadic(t,den,d);let out=id.x*132u;
 audit_output[out]=select(0u,1u,result);audit_output[out+1u]=ex_history_gcd(ex_history_digit_sum(den),65535u);
 audit_output[out+2u]=ex_history_digit_sum(den);audit_output[out+3u]=ex_history_digit_sum(t);
 audit_output[out+4u]=ex_history_digit_sum(direction);audit_output[out+5u]=ex_peak;
 let dm=ex_history_moments(den);let tm=ex_history_moments(t);let vm=ex_history_moments(direction);
 for(var k=0u;k<3u;k+=1u) {audit_output[out+6u+k]=dm[k];audit_output[out+9u+k]=tm[k];audit_output[out+12u+k]=vm[k];}
 let primes=vec4<u32>(3u,5u,17u,257u);
 for(var i=0u;i<4u;i+=1u) {
  let p=primes[i];let a=ex_history_residue_cube(dm,p);let b=ex_history_residue_cube(tm,p);let c=ex_history_residue_cube(vm,p);
  audit_output[out+15u+i*3u]=a;audit_output[out+16u+i*3u]=b;audit_output[out+17u+i*3u]=c;
  audit_output[out+27u+i*3u]=ex_history_valuation_cube(a,p);audit_output[out+28u+i*3u]=ex_history_valuation_cube(b,p);audit_output[out+29u+i*3u]=ex_history_valuation_cube(c,p);
 }
}

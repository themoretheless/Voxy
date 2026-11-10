@group(0) @binding(0) var<storage,read_write> gap_words:array<u32>;
var<workgroup> gap_partial:array<FiveRounded,64>;
fn gap_input(at:u32)->FiveRounded{
 var parts:array<f32,5>;for(var i=0u;i<5u;i++){parts[i]=bitcast<f32>(gap_words[at+i]);}
 return five_from_operand(five_normalize_operand(parts));
}
fn gap_state(at:u32)->FiveRounded{
 var value:FiveRounded;for(var i=0u;i<5u;i++){value.terms[i]=bitcast<f32>(gap_words[at+i]);}
 value.exponent=bitcast<i32>(gap_words[at+5u]);value.error_bound=bitcast<f32>(gap_words[at+6u]);value.valid=gap_words[at+7u];return value;
}
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>){
 let row=group.x;let count=gap_words[2];let width=gap_words[3];
 if gap_words[0]!=0x31473556u || gap_words[1]!=3u || gap_words[4]!=5u || gap_words[10]!=arrayLength(&gap_words) || row>=count {return;}
 var sum:FiveRounded;sum.valid=1u;
 for(var i=lane;i<width;i+=64u){sum=five_add(sum,five_multiply(gap_input(gap_words[6]+5u*(row*width+i)),gap_state(gap_words[7]+8u*i)));}
 gap_partial[lane]=sum;workgroupBarrier();
 for(var stride=32u;stride>0u;stride/=2u){if lane<stride {gap_partial[lane]=five_add(gap_partial[lane],gap_partial[lane+stride]);}workgroupBarrier();}
 if lane==0u {
  var dot=gap_partial[0];dot.exponent+=bitcast<i32>(gap_words[11]);
  let residual=five_subtract(dot,gap_input(gap_words[8]+5u*row));let at=gap_words[9]+9u*row;
  // The shifted residual interval must lie wholly on one side of the
  // unchanged tolerance. An interval crossing a boundary never certifies KKT.
  let tolerance=gap_input(gap_words[12]);
  let lower=five_add(residual,tolerance);let upper=five_subtract(residual,tolerance);
  var decision=4u;
  if lower.valid==1u && upper.valid==1u {
   if lower.terms[0]<0.0 && five_lower_magnitude(lower)>0.0 {decision=1u;}
   else if upper.terms[0]>0.0 && five_lower_magnitude(upper)>0.0 {decision=2u;}
   else if (five_exact_zero(lower) || (lower.terms[0]>0.0 && five_lower_magnitude(lower)>0.0))
        && (five_exact_zero(upper) || (upper.terms[0]<0.0 && five_lower_magnitude(upper)>0.0)) {decision=3u;}
  }
  gap_words[at+8u]=decision;
  for(var i=0u;i<5u;i++){gap_words[at+i]=bitcast<u32>(residual.terms[i]);}
  gap_words[at+5u]=bitcast<u32>(residual.exponent);gap_words[at+6u]=bitcast<u32>(residual.error_bound);gap_words[at+7u]=residual.valid;
 }
}

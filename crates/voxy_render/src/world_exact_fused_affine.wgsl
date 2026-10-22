// Experimental, default-off exact source affine sum in units of 2^-298.
// Every term is represented by complete radix-16 digits, with no float arithmetic.
struct AffineTerm {digits:vec4<u32>,shift:u32,negative:u32,valid:u32}
fn affine_float_term(bits:u32,subtract:u32)->AffineTerm {
 let exponent=(bits>>23u)&255u;
 let mantissa=(bits&0x7fffffu)|select(0u,0x800000u,exponent!=0u);
 return AffineTerm(vec4<u32>(mantissa&65535u,mantissa>>16u,0u,0u),select(149u,exponent+148u,exponent!=0u),(bits>>31u)^subtract,select(1u,0u,exponent==255u));
}
fn affine_product_term(a:u32,b:u32)->AffineTerm {
 let ae=(a>>23u)&255u;let be=(b>>23u)&255u;
 let am=(a&0x7fffffu)|select(0u,0x800000u,ae!=0u);
 let bm=(b&0x7fffffu)|select(0u,0x800000u,be!=0u);
 let low=(am&65535u)*(bm&65535u);
 let middle=(am>>16u)*(bm&65535u)+(bm>>16u)*(am&65535u)+(low>>16u);
 let upper=(am>>16u)*(bm>>16u)+(middle>>16u);
 return AffineTerm(vec4<u32>(low&65535u,middle&65535u,upper,0u),select(0u,ae-1u,ae!=0u)+select(0u,be-1u,be!=0u),(a^b)>>31u,select(1u,0u,ae==255u||be==255u));
}
fn affine_base_digit(term:AffineTerm,k:u32)->u32 {
 if k>=4u {return 0u;}return term.digits[k];
}
fn affine_shifted_digit(term:AffineTerm,k:u32)->u32 {
 let whole=term.shift/16u;let part=term.shift%16u;
 if k<whole {return 0u;}let local=k-whole;
 var digit=(affine_base_digit(term,local)<<part)&65535u;
 if part!=0u&&local!=0u {digit|=affine_base_digit(term,local-1u)>>(16u-part);}
 return digit;
}
fn affine_extent(term:AffineTerm)->u32 {
 var k=4u;
 while k>0u {k-=1u;if term.digits[k]!=0u {return (term.shift+k*16u+32u-countLeadingZeros(term.digits[k])+15u)/16u;}}
 return 0u;
}
fn affine_fused_axis(m:vec3<u32>,p:vec3<u32>,translation:u32,hi:u32,lo:u32)->ExInt {
 var terms=array<AffineTerm,6>(affine_product_term(m.x,p.x),affine_product_term(m.y,p.y),affine_product_term(m.z,p.z),affine_float_term(translation,0u),affine_float_term(hi,1u),affine_float_term(lo,1u));
 var count=0u;
 for(var k=0u;k<6u;k++) {if terms[k].valid==0u {return ex_bad();}count=max(count,affine_extent(terms[k]));}
 count+=1u;
 // Finite binary32 affine sums need at most 556 magnitude bits plus sign.
 if count>36u||count>EX_WORDS {return ex_bad();}
 var result=ex_zero();if result.valid==0u {return result;}
 var carry=0i;
 for(var k=0u;k<count;k++) {
  var digit=carry;
  for(var j=0u;j<6u;j++) {let magnitude=i32(affine_shifted_digit(terms[j],k));digit+=select(magnitude,-magnitude,terms[j].negative!=0u);}
  exact_scratch[result.offset+k]=bitcast<u32>(digit)&65535u;
  // Signed arithmetic shift is floor division by 65536, including negatives.
  carry=digit>>16u;
 }
 if carry!=0i&&carry!= -1i {return ex_bad();}
 if carry== -1i {
  var magnitude_carry=1u;
  for(var k=0u;k<count;k++) {let digit=65535u-exact_scratch[result.offset+k]+magnitude_carry;exact_scratch[result.offset+k]=digit&65535u;magnitude_carry=digit>>16u;}
  if magnitude_carry!=0u {return ex_bad();}result.negative=1u;
 }
 result.length=count;return ex_trim(result);
}

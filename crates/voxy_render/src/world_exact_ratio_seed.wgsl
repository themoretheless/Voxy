// Optional quotient seed; every accepted result needs an exact midpoint proof.
// Compare |n| * 2^scale with d * mantissa * 2^power. All source digits
// participate; shifts and the 25-bit midpoint product are streamed, not rounded.
fn ex_seed_shift_digit(a:ExInt,k:u32,bits:u32)->u32 {
 let whole=bits/16u;let part=bits%16u;if k<whole{return 0u;}
 let at=k-whole;var v=(ex_word(a,at)<<part)&65535u;
 if part!=0u&&at>0u {v|=ex_word(a,at-1u)>>(16u-part);}return v;
}
fn ex_seed_compare_midpoint(n:ExInt,d:ExInt,mantissa:u32,power:i32,scale:i32)->ExOrdering {
 if (n.valid&d.valid)==0u||d.length==0u||d.negative!=0u||mantissa==0u||mantissa>33554431u||scale< -8192||scale>8192||power< -151||power>104 {return ExOrdering(0,0u);}
 let shift=power-scale;let ns=u32(max(0,-shift));let ds=u32(max(0,shift));
 let whole=ds/16u;let part=ds%16u;let m0=mantissa&65535u;let m1=mantissa>>16u;
 let count=max(n.length+(ns+15u)/16u,d.length+2u+(ds+15u)/16u);
 var carry=0u;var previous=0u;var order=0;
 for(var k=0u;k<count;k++) {
  var right=0u;
  if k>=whole {
   let j=k-whole;let a=ex_word(d,j)*m0;
   var b=0u;if j>0u {b=ex_word(d,j-1u)*m1;}
   let s=a+carry;var high=select(0u,1u,s<a);let t=s+b;high+=select(0u,1u,t<s);
   carry=(t>>16u)|(high<<16u);let raw=t&65535u;
   right=(raw<<part)&65535u;if part!=0u {right|=previous>>(16u-part);}previous=raw;
  }
  let left=ex_seed_shift_digit(n,k,ns);
  if left<right {order=-1;}else if left>right {order=1;}
 }
 return ExOrdering(order,select(0u,1u,carry==0u));
}
struct ExSeedProof {valid:u32,direction:i32}
// direction -1: seed too high; +1: seed too low. No rounded comparison
// contributes to accepting the quotient; both midpoint tests are integer-only.
fn ex_seed_verify(n:ExInt,d:ExInt,scale:i32,bits:u32)->ExSeedProof {
 if bits>2139095039u{return ExSeedProof(0u,0);}
 let even=(bits&1u)==0u;
 var upper_m=1u;var upper_power=-150;
 if bits!=0u {
  let e=bits>>23u;var m=bits&8388607u;var p=-149;
  if e!=0u {m|=8388608u;p=i32(e)-150;}
  var lower_m=2u*m-1u;var lower_power=p-1;
  if e>1u&&(bits&8388607u)==0u {lower_m=4u*m-1u;lower_power=p-2;}
  let lower=ex_seed_compare_midpoint(n,d,lower_m,lower_power,scale);
  if lower.valid==0u{return ExSeedProof(0u,0);}
  if lower.order<0||(lower.order==0&&!even){return ExSeedProof(1u,-1);}
  upper_m=2u*m+1u;upper_power=p-1;
 }
 let upper=ex_seed_compare_midpoint(n,d,upper_m,upper_power,scale);
 if upper.valid==0u{return ExSeedProof(0u,0);}
 if upper.order>0||(upper.order==0&&!even){return ExSeedProof(1u,1);}
 return ExSeedProof(1u,0);
}
fn ex_seed_top24(a:ExInt,bits:u32)->u32 {
 if bits<=24u{return (ex_word(a,0u)|(ex_word(a,1u)<<16u))<<(24u-bits);}
 let shift=bits-24u;let at=shift/16u;let part=shift%16u;
 let low=ex_word(a,at)|(ex_word(a,at+1u)<<16u);var v=low>>part;
 if part!=0u {v|=ex_word(a,at+2u)<<(32u-part);}return v&16777215u;
}
fn ex_seed_initial(n:ExInt,d:ExInt,scale:i32)->u32 {
 let nb=ex_bits(n);let db=ex_bits(d);
 let nf=bitcast<f32>(1065353216u|(ex_seed_top24(n,nb)&8388607u));
 let df=bitcast<f32>(1065353216u|(ex_seed_top24(d,db)&8388607u));
 var q=nf/df;var exponent=i32(nb)-i32(db)+scale;
 if q<1. {q*=2.;exponent-=1;}else if q>=2. {q*=0.5;exponent+=1;}
 if exponent>127{return 2139095039u;}if exponent< -150{return 0u;}
 return min(bitcast<u32>(ldexp(q,exponent))&2147483647u,2139095039u);
}
// A candidate result is published only after exact RN-even certification. Any
// unavailable proof must use the existing full ex_round_ratio, never a miss.
fn ex_round_ratio_seed(numerator:ExInt,denominator:ExInt,scale:i32)->ExRounded {
 if (numerator.valid&denominator.valid)==0u||denominator.length==0u||denominator.negative!=0u||scale< -8192||scale>8192 {return ex_round_ratio(numerator,denominator,scale);}
 if numerator.length==0u{return ExRounded(0u,1u);}
 if ex_cursor+56u>EX_SLOTS{return ex_round_ratio(numerator,denominator,scale);}
 var n=numerator;var d=denominator;
 if EX_CANCEL_RATIO {
  let zeros=min(ex_low_word(n),ex_low_word(d));n.offset+=zeros;n.length-=zeros;d.offset+=zeros;d.length-=zeros;
 }
 // Preserve the original full-division bit-capacity gate. The exponent can
 // differ by one from the bit-length estimate, so cover both exact outcomes.
 let difference=i32(ex_bits(n))-i32(ex_bits(d));
 for(var delta=0;delta<2;delta++) {
  let exponent=difference-delta;var shift=scale+149;
  if exponent+scale>= -126 {shift=23-exponent;}
  if i32(ex_bits(n))+max(0,shift)>2048||i32(ex_bits(d))+max(0,-shift)>2048 {return ex_round_ratio(numerator,denominator,scale);}
 }
 var seed=ex_seed_initial(n,d,scale);
 for(var attempt=0u;attempt<3u;attempt++) {
  let proof=ex_seed_verify(n,d,scale,seed);
  if proof.valid==0u{break;}
  if proof.direction==0{return ExRounded(seed|(numerator.negative<<31u),1u);}
  if proof.direction<0 {if seed==0u{break;}seed-=1u;}
  else {if seed==2139095039u{break;}seed+=1u;}
 }
 return ex_round_ratio(numerator,denominator,scale);
}

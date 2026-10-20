// RN-even f32 high word, then RN-even exact residual low word. No float math.
// A remaining rational residual is explicit; pairs never stand in for exact ordering.
const EX_CANCEL_RATIO:bool=true;
struct ExRounded {bits:u32,valid:u32}
struct ExResidual {numerator:ExInt,scale:i32}
struct ExPairBits {hi:u32,lo:u32,valid:u32,residual_nonzero:u32}
fn ex_bits(a:ExInt)->u32 {if a.length==0u {return 0u;}return (a.length-1u)*16u+32u-countLeadingZeros(ex_word(a,a.length-1u));}
fn ex_round_ratio(numerator:ExInt,denominator:ExInt,scale:i32)->ExRounded {
 if (numerator.valid&denominator.valid)==0u||denominator.length==0u||denominator.negative!=0u {return ExRounded(0u,0u);}
 if numerator.length==0u {return ExRounded(0u,1u);}
 // Equal powers of 2^16 cancel exactly. These are immutable views into the
 // original digits, with no shift, allocation, truncation or source mutation.
 var n=numerator;var den=denominator;
 if EX_CANCEL_RATIO {
  let shared_zero_digits=min(ex_low_word(n),ex_low_word(den));
  n.offset+=shared_zero_digits;n.length-=shared_zero_digits;den.offset+=shared_zero_digits;den.length-=shared_zero_digits;
 }
 n.negative=0u;let sign=numerator.negative<<31u;
 var exponent=i32(ex_bits(n))-i32(ex_bits(den));
 var left=n;var right=den;
 if exponent>=0 {right=ex_shl(right,u32(exponent));}else {left=ex_shl(left,u32(-exponent));}
 if (left.valid&right.valid)==0u {return ExRounded(0u,0u);}
 if ex_cmp_abs(left,right)<0 {exponent-=1;}
 var floating_exponent=exponent+scale;if floating_exponent>127 {return ExRounded(0u,0u);}
 var shift=scale+149;if floating_exponent>= -126 {shift=23-exponent;}
 var a=n;var b=den;if shift>=0 {a=ex_shl(a,u32(shift));}else {b=ex_shl(b,u32(-shift));}
 if (a.valid&b.valid)==0u {return ExRounded(0u,0u);}
 var remainder=a;var quotient=0u;
 for(var bit=24;bit>=0;bit-=1) {
  if ex_bits(b)+u32(bit)>EX_WORDS*16u {continue;}
  let divisor=ex_shl(b,u32(bit));if divisor.valid==0u {return ExRounded(0u,0u);}
  if ex_cmp_abs(remainder,divisor)>=0 {remainder=ex_sub_abs(remainder,divisor);quotient|=1u<<u32(bit);}
 }
 if remainder.valid==0u||ex_cmp_abs(remainder,b)>=0 {return ExRounded(0u,0u);}
 var comparison=-1;
 if remainder.length!=0u {
  let rb=ex_bits(remainder)+1u;let db=ex_bits(b);
  if rb>db {comparison=1;}else if rb==db {let twice=ex_shl(remainder,1u);if twice.valid==0u {return ExRounded(0u,0u);}comparison=ex_cmp_abs(twice,b);}
 }
 if comparison>0||(comparison==0&&(quotient&1u)!=0u) {quotient+=1u;}
 if floating_exponent< -126 {return ExRounded(sign|quotient,select(0u,1u,quotient<=8388608u));}
 if quotient==16777216u {quotient>>=1u;floating_exponent+=1;}
 if floating_exponent>127||quotient<8388608u||quotient>=16777216u {return ExRounded(0u,0u);}
 return ExRounded(sign|(u32(floating_exponent+127)<<23u)|(quotient&8388607u),1u);
}
fn ex_ratio_residual(n:ExInt,d:ExInt,scale:i32,bits:u32)->ExResidual {
 if (bits&2147483647u)==0u {return ExResidual(n,scale);}
 let exponent=(bits>>23u)&255u;var power=-149;if exponent!=0u {power=i32(exponent)-150;}
 let base=min(scale,power);
 let value=ex_float(bits,power);let original=ex_shl(n,u32(scale-base));
 let rounded=ex_shl(ex_mul(d,value),u32(power-base));return ExResidual(ex_sub(original,rounded),base);
}
fn ex_ratio_pair(n:ExInt,d:ExInt,scale:i32)->ExPairBits {
 let high=ex_round_ratio(n,d,scale);if high.valid==0u {return ExPairBits(0u,0u,0u,0u);}
 let residual=ex_ratio_residual(n,d,scale,high.bits);
 let low=ex_round_ratio(residual.numerator,d,residual.scale);if low.valid==0u {return ExPairBits(0u,0u,0u,0u);}
 let tail=ex_ratio_residual(residual.numerator,d,residual.scale,low.bits);
 return ExPairBits(high.bits,low.bits,tail.numerator.valid,select(0u,1u,tail.numerator.length!=0u));
}

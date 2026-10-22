// A consumer proof only: a non-dyadic rational cannot equal any finite f32 pair.
// Radix 2^16 is 1 modulo 65535. Thus gcd(sum_digits(den),65535) divides den.
// All divisors tested here are odd; no rounding, allocation, or epsilon occurs.
fn ex_history_digit_sum(a:ExInt)->u32 {
 var sum=0u;for(var k=0u;k<a.length;k+=1u) {sum+=ex_word(a,k);}
 return sum;
}
fn ex_history_gcd(a:u32,b:u32)->u32 {
 var x=a;var y=b;while y!=0u {let next=x%y;x=y;y=next;}return x;
}
// With radix B=65536=1+65535, the first three binomial moments determine
// the complete integer modulo p^3 for p in {3,5,17,257}. No digits are dropped.
fn ex_history_moments(a:ExInt)->vec3<u32> {
 var sum=0u;var first=0u;var second=0u;var carries=0u;
 for(var k=0u;k<a.length;k+=1u) {
  let word=ex_word(a,k);sum+=word;first+=k*word;
  var weight=0u;if k>1u {weight=k*(k-1u)/2u;}
  let before=second;second+=weight*word; carries+=select(0u,1u,second<before);
 }
 // 2^32 is 1 modulo 65535. Keep every carry in the second moment.
 return vec3<u32>(sum,first,(second%65535u+carries)%65535u);
}
fn ex_history_residue_cube(m:vec3<u32>,p:u32)->u32 {
 let square=p*p;let cube=square*p;let q=65535u/p;
 let first=p*(((q%square)*(m.y%square))%square);
 let second=square*((((q%p)*(q%p)%p)*(m.z%p))%p);
 return (m.x+first+second)%cube;
}
fn ex_history_valuation_cube(residue:u32,p:u32)->u32 {
 var value=residue;
 for(var order=0u;order<3u;order+=1u) {if value%p!=0u {return order;}value/=p;}
 return 3u;
}
fn ex_history_axis_non_dyadic(t:vec3<u32>,den:vec3<u32>,d:ExInt)->bool {
 let direction=ex_history_moments(d);let primes=vec4<u32>(3u,5u,17u,257u);
 for(var index=0u;index<4u;index+=1u) {
  let p=primes[index];let dv=ex_history_valuation_cube(ex_history_residue_cube(den,p),p);
  let tv=ex_history_valuation_cube(ex_history_residue_cube(t,p),p);
  let vv=ex_history_valuation_cube(ex_history_residue_cube(direction,p),p);
  // A value below 3 is exact; 3 is a conservative lower bound. If true,
  // both numerator valuations are exact and their sum is strictly smaller.
  if dv>tv+vv {return true;}
 }
 return false;
}
fn ex_history_position_non_dyadic(t:ExInt,den:ExInt,d:ExVec)->bool {
 if (t.valid&den.valid&ex_vec_valid(d))==0u||den.length==0u||den.negative!=0u {return false;}
 let denominator=ex_history_moments(den);let numerator=ex_history_moments(t);
 if ex_history_axis_non_dyadic(numerator,denominator,d.x) {return true;}
 if ex_history_axis_non_dyadic(numerator,denominator,d.y) {return true;}
 return ex_history_axis_non_dyadic(numerator,denominator,d.z);
}

// Uses the exact binary32 term decoder shared with source-affine predicates.
// 36 radix-65536 digits cover every sum of <=256 finite binary32 products.
@group(0) @binding(5) var<storage,read> morph_deltas:array<u32>;
@group(0) @binding(6) var<storage,read> morph_weights:array<u32>;
fn morph_accumulate(digits:ptr<function,array<i32,36>>,term:AffineTerm) {
 let first=term.shift/16u;
 for(var j=0u;j<4u;j++) {
  let magnitude=i32(affine_shifted_digit(term,first+j));
  (*digits)[first+j]+=select(magnitude,-magnitude,term.negative!=0u);
 }
}
fn morph_component_bits(vertex:u32,component:u32)->u32 {
 var digits:array<i32,36>;
 let base=bitcast<u32>(source[vertex*16u+component]);
 morph_accumulate(&digits,affine_float_term(base,0u));
 for(var shape=0u;shape<parameters.pad0;shape++) {
  let delta=morph_deltas[(shape*parameters.count+vertex)*6u+component];
  let weight=morph_weights[shape];
  if (delta&0x7fffffffu)!=0u && (weight&0x7fffffffu)!=0u {
   morph_accumulate(&digits,affine_product_term(delta,weight));
  }
 }
 var words:array<u32,36>;var carry=0i;
 for(var k=0u;k<36u;k++) {
  let value=digits[k]+carry;words[k]=bitcast<u32>(value)&65535u;carry=value>>16u;
 }
 let negative=carry== -1i;
 if negative {
  var carry=1u;
  for(var k=0u;k<36u;k++) {let value=(words[k]^65535u)+carry;words[k]=value&65535u;carry=value>>16u;}
 }
 var length=36u;
 while length>0u {if words[length-1u]!=0u {break;}length-=1u;}
 if length==0u {return 0u;}
 var high=(length-1u)*16u+31u-countLeadingZeros(words[length-1u]);
 let shift=max(149u,u32(max(0i,i32(high)-23i)));
 // Read the 24-bit significand across adjacent radix-65536 digits.
 let first=shift/16u;
 let offset=shift%16u;
 var significand=(words[first]>>offset)|(words[first+1u]<<(16u-offset));
 if offset>8u {
  significand|=words[first+2u]<<(32u-offset);
 }
 significand&=0x00ffffffu;

 let guard_index=shift-1u;
 let guard_word=guard_index/16u;
 let guard_offset=guard_index%16u;
 let guard=(words[guard_word]&(1u<<guard_offset))!=0u;
 if guard {
  // Sticky is the OR of all bits below the guard, including its partial digit.
  var lower=words[guard_word]&((1u<<guard_offset)-1u);
  for(var k=0u;k<guard_word;k++) {
   lower|=words[k];
  }
  if lower!=0u || (significand&1u)!=0u {
   significand+=1u;
  }
 }
 let sign=select(0u,0x80000000u,negative);
 if high<172u {return sign|significand;}
 if significand==0x1000000u {significand>>=1u;high+=1u;}
 let exponent=high-171u;
 // Host preflight rejects overflow before any queue writes.
 if exponent>=255u {return 0x7fc00000u;}
 return sign|(exponent<<23u)|(significand&0x7fffffu);
}

// Cast the runtime word; constant evaluation must never materialize a NaN.
fn morph_component(vertex:u32,component:u32)->f32 {
 return bitcast<f32>(morph_component_bits(vertex,component));
}

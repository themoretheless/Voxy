// Standalone numeric experiment. No scene ownership or source predicate changes.
struct RelativeParts {bits:vec4<u32>,valid:u32}
struct RelativeEndpoint {value:f32,valid:u32}
struct RelativeResult {bounds:WorldInterval,parts:vec4<u32>,reason:u32}
fn relative_parts(record:vec4<u32>)->RelativeParts {
 let h=record.x;let live=(h>>8u)&7u;let low=h&255u;
 if (h&32768u)==0u||(h&0xffff3800u)!=0u||live>6u||low+live>128u {return RelativeParts(vec4<u32>(0u),0u);}
 let payload=record.yzw;
 for(var k=0u;k<6u;k++) {
  let digit=(payload[k/2u]>>((k%2u)*16u))&65535u;
  if k>=live&&digit!=0u {return RelativeParts(vec4<u32>(0u),0u);}
 }
 if live==0u&&h!=32768u {return RelativeParts(vec4<u32>(0u),0u);}
 let chunks=vec4<u32>(record.y&0xffffffu,(record.y>>24u)|((record.z&0xffffu)<<8u),(record.z>>16u)|((record.w&255u)<<16u),record.w>>8u);
 var result=vec4<u32>(0u);let sign=(h&16384u)<<17u;
 for(var k=0u;k<4u;k++) {
  let chunk=chunks[k];if chunk==0u {continue;}
  let leading=31u-countLeadingZeros(chunk);let exponent=i32(low)*16-298+i32(k)*24+i32(leading);
  if exponent< -126||exponent>127 {return RelativeParts(vec4<u32>(0u),0u);}
  let bits=sign|(u32(exponent+127)<<23u)|((chunk<<(23u-leading))&0x7fffffu);
  if !wi_normal_bits(bits) {return RelativeParts(vec4<u32>(0u),0u);}
  result[k]=bits;
 }
 return RelativeParts(result,1u);
}
fn relative_endpoint(a:f32,b:f32,upper:bool)->RelativeEndpoint {
 if !wi_normal_or_zero(a)||!wi_normal_or_zero(b) {return RelativeEndpoint(0.,0u);}
 if a==0. {return RelativeEndpoint(b,1u);}if b==0. {return RelativeEndpoint(a,1u);}
 if a== -b {return RelativeEndpoint(0.,1u);}
 let value=projection_add_scalar(a,b);
 if !wi_normal_or_zero(value)||value==0. {return RelativeEndpoint(0.,0u);}
 let bound=select(wi_down(value),wi_up(value),upper);
 if !wi_normal_or_zero(bound) {return RelativeEndpoint(0.,0u);}
 return RelativeEndpoint(bound,1u);
}
fn relative_interval(record:vec4<u32>,origin:vec2<u32>)->RelativeResult {
 if !wi_normal_bits(origin.x)||!wi_normal_bits(origin.y) {return RelativeResult(wi_bad(),vec4<u32>(0u),3u);}
 let decoded=relative_parts(record);
 if decoded.valid==0u {return RelativeResult(wi_bad(),vec4<u32>(0u),2u);}
 var low=RelativeEndpoint(bitcast<f32>(origin.x),1u);var high=low;var started=false;
 for(var step=0u;step<4u;step++) {
  let bits=decoded.bits[3u-step];if (bits&0x7fffffffu)==0u {continue;}
  let part=bitcast<f32>(bits);
  low=relative_endpoint(low.value,-part,false);high=relative_endpoint(high.value,-part,true);
  if (low.valid&high.valid)==0u {return RelativeResult(wi_bad(),decoded.bits,4u);}
  if !started {
   low=relative_endpoint(low.value,bitcast<f32>(origin.y),false);high=relative_endpoint(high.value,bitcast<f32>(origin.y),true);started=true;
   if (low.valid&high.valid)==0u {return RelativeResult(wi_bad(),decoded.bits,4u);}
  }
 }
 if !started {
  low=relative_endpoint(low.value,bitcast<f32>(origin.y),false);high=relative_endpoint(high.value,bitcast<f32>(origin.y),true);
 }
 if (low.valid&high.valid)==0u {return RelativeResult(wi_bad(),decoded.bits,4u);}
 return RelativeResult(WorldInterval(low.value,high.value,1u),decoded.bits,0u);
}

// GPU bridge from the actual archive GI/reflection ray inputs. The source ray
// origin/direction are existing f32 coefficients; no missing upstream bits are invented.
@group(1) @binding(17) var<storage,read_write> world_exact_ray_words:array<u32>;
struct WorldScaledWord {hi:u32,lo:u32,valid:u32}
fn world_word_exact(m:u32,power:i32,sign:u32)->u32 {
 if m==0u {return 0u;}
 let top=31u-countLeadingZeros(m);let exponent=i32(top)+power;
 if exponent< -126 {return sign|(m<<u32(power+149));}
 return sign|(u32(exponent+127)<<23u)|((m<<(23u-top))&0x7fffffu);
}
// Integer decomposition of the exact dyadic f32 coefficient * 60. The product
// has at most 30 significant bits. Its RN-even high word and exact remainder
// therefore fit two f32 words, including every subnormal low-word bit.
fn world_scale_60(raw:u32)->WorldScaledWord {
 let sign=raw&0x80000000u;let biased=(raw>>23u)&255u;
 if biased==255u {return WorldScaledWord(raw,0u,0u);}
 var m=raw&0x7fffffu;var power=-149;
 if biased!=0u {m|=0x800000u;power=i32(biased)-150;}
 let product=m*60u;if product==0u {return WorldScaledWord(sign,0u,1u);}
 let top=31u-countLeadingZeros(product);var exponent=i32(top)+power;
 if exponent>127 {return WorldScaledWord(raw,0u,0u);}
 if top<=23u||exponent< -126 {return WorldScaledWord(world_word_exact(product,power,sign),0u,1u);}
 let shift=top-23u;var rounded=product>>shift;let tail=product&((1u<<shift)-1u);let half=1u<<(shift-1u);
 if tail>half||(tail==half&&(rounded&1u)!=0u) {rounded+=1u;}
 let reconstructed=rounded<<shift;
 if rounded==0x1000000u {rounded>>=1u;exponent+=1;}
 if exponent>127 {return WorldScaledWord(raw,0u,0u);}
 var remainder=0u;var low_sign=sign;
 if reconstructed>product {remainder=reconstructed-product;low_sign^=0x80000000u;}else{remainder=product-reconstructed;}
 let high=sign|(u32(exponent+127)<<23u)|(rounded&0x7fffffu);
 return WorldScaledWord(high,world_word_exact(remainder,power,low_sign),1u);
}
@compute @workgroup_size(8,8) fn cs_world_ray_produce(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let pixel=ray_tile.offset+id.xy;let index=id.y*ray_tile.extent.x+id.x;let base=index*20u;
 if base+19u>=arrayLength(&world_exact_ray_words) {return;}
 let ray=archive_ray_input(pixel);var state=select(0u,1u,ray.admitted);
 for(var k=0u;k<20u;k++){world_exact_ray_words[base+k]=0u;}
 for(var axis=0u;axis<3u;axis++) {
  let origin=bitcast<u32>(ray.origin[axis]);let scaled=world_scale_60(bitcast<u32>(ray.direction[axis]));
  world_exact_ray_words[base+axis]=origin;
  world_exact_ray_words[base+8u+axis]=scaled.hi;world_exact_ray_words[base+12u+axis]=scaled.lo;
  if state!=0u&&(((origin>>23u)&255u)==255u||scaled.valid==0u){state=2u;}
 }
 world_exact_ray_words[base+3u]=state;
 world_exact_ray_words[base+18u]=pixel.y*textureDimensions(gi_world).x+pixel.x;
 world_exact_ray_words[base+19u]=select(0u,1u,ARCHIVE_TRACE_SSR);
}

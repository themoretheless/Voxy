// Raw copies of the actual RGBA32Float output. No float conversion or tolerance.
@group(0) @binding(0) var<storage,read> original:array<u32>;
@group(0) @binding(1) var<storage,read> candidate:array<u32>;
@group(0) @binding(2) var<storage,read_write> stats:array<atomic<u32>,16>;
struct Region { rect:vec4<u32>, packing:vec4<u32> }
@group(0) @binding(3) var<uniform> region:Region;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if(id.x>=region.rect.z || id.y>=region.rect.w){return;}
 let old=atomicAdd(&stats[1],1u);
 if(old==0xffffffffu){if(atomicAdd(&stats[2],1u)==0xffffffffu){atomicOr(&stats[0],4u);}}
 let base=id.y*region.packing.x+id.x*4u;
 if(base+3u>=arrayLength(&original) || base+3u>=arrayLength(&candidate)){atomicOr(&stats[0],8u);return;}
 for(var c=0u;c<4u;c++){
  let a=original[base+c];let b=candidate[base+c];
  let nonfinite=((a&0x7f800000u)==0x7f800000u)||((b&0x7f800000u)==0x7f800000u);
  if(a!=b || nonfinite){
   atomicOr(&stats[0],select(0u,1u,a!=b)|select(0u,2u,nonfinite));
   loop {let claim=atomicCompareExchangeWeak(&stats[3],0u,1u);
    if(claim.exchanged){
     atomicStore(&stats[4],region.rect.x+id.x);atomicStore(&stats[5],region.rect.y+id.y);
     atomicStore(&stats[6],c);atomicStore(&stats[7],region.packing.y);
     for(var k=0u;k<4u;k++){atomicStore(&stats[8u+k],original[base+k]);atomicStore(&stats[12u+k],candidate[base+k]);}
     break;
    }
    if(claim.old_value!=0u){break;}
   }
  }
 }
}

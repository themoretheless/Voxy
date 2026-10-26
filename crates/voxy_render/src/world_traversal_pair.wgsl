// Diagnostic only: exact native words, never a float tolerance. Both buffers
// are results of GPU queries over the same actual producer inputs and owner.
override PAIR_RECORD_WORDS:u32=20u;
override PAIR_POSITION_WORD:u32=0u;
@group(0) @binding(0) var<storage,read> original:array<u32>;
@group(0) @binding(1) var<storage,read> candidate:array<u32>;
@group(0) @binding(2) var<storage,read> rays:array<u32>;
@group(0) @binding(3) var<storage,read_write> stats:array<atomic<u32>,16>;
@group(0) @binding(4) var<storage,read> original_rays:array<u32>;
@group(0) @binding(5) var<storage,read_write> failure_capture:array<u32,128>;
fn pair_failure(index:u32,word:u32,a:u32,b:u32) {
 atomicOr(&stats[0],1u);
 var ticket=atomicCompareExchangeWeak(&stats[3],0u,1u);
 while !ticket.exchanged&&ticket.old_value==0u {ticket=atomicCompareExchangeWeak(&stats[3],0u,1u);}
 if ticket.exchanged {
  atomicStore(&stats[4],index);atomicStore(&stats[5],word);
  atomicStore(&stats[6],a);atomicStore(&stats[7],b);
  atomicStore(&stats[8],original_rays[index*20u+18u]);atomicStore(&stats[9],original_rays[index*20u+19u]);
  failure_capture[0]=1u;failure_capture[1]=index;failure_capture[2]=word;failure_capture[3]=PAIR_RECORD_WORDS;
  failure_capture[4]=PAIR_POSITION_WORD;failure_capture[5]=arrayLength(&original);failure_capture[6]=arrayLength(&candidate);failure_capture[7]=arrayLength(&rays)/20u;
  for(var k=0u;k<20u;k++){failure_capture[8u+k]=original_rays[index*20u+k];failure_capture[28u+k]=rays[index*20u+k];}
  for(var k=0u;k<28u;k++) {
   let at=index*PAIR_RECORD_WORDS+k;
   if k<PAIR_RECORD_WORDS&&at<arrayLength(&original) {failure_capture[48u+k]=original[at];}
   if k<PAIR_RECORD_WORDS&&at<arrayLength(&candidate) {failure_capture[76u+k]=candidate[at];}
  }
  if PAIR_POSITION_WORD!=0u {for(var k=0u;k<8u;k++) {
   let at=PAIR_POSITION_WORD+index*8u+k;
   if at<arrayLength(&original) {failure_capture[104u+k]=original[at];}
   if at<arrayLength(&candidate) {failure_capture[112u+k]=candidate[at];}
  }}

 }
}
fn pair_increment(low_word:u32) {
 let low=atomicAdd(&stats[low_word],1u);
 if low==0xffffffffu {let high=atomicAdd(&stats[low_word+1u],1u);if high==0xffffffffu {atomicOr(&stats[0],2u);}}
}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let count=arrayLength(&rays)/20u;if id.x>=count {return;}
 if id.x==0u {
  let low=atomicAdd(&stats[1],count);
  if low>0xffffffffu-count {let high=atomicAdd(&stats[2],1u);if high==0xffffffffu {atomicOr(&stats[0],2u);}}
 }
 for(var k=0u;k<20u;k++) {
  // Only the existing deferred queue's reserved padding may change.
  if k==7u||k==11u||k==15u {continue;}
  let at=id.x*20u+k;let a=original_rays[at];let b=rays[at];
  if a!=b {pair_failure(id.x,0x40000000u|k,a,b);return;}
 }
 if original_rays[id.x*20u+3u]==1u {pair_increment(10u);}
 let base=id.x*PAIR_RECORD_WORDS;
 if base+PAIR_RECORD_WORDS>arrayLength(&original)||base+PAIR_RECORD_WORDS>arrayLength(&candidate) {pair_failure(id.x,0xffffffffu,0u,1u);return;}
 for(var k=0u;k<PAIR_RECORD_WORDS;k++) {
  // Word 3 is scratch/seed provenance; word 16 counts visited candidates.
  // Word 18 retains material enable/kind/unknown bits (1..4); other bits
  // describe native/certificate routes. No numeric result word is excluded.
  if k==3u||k==16u {continue;}
  var a=original[base+k];var b=candidate[base+k];
  if k==18u {a&=0x1eu;b&=0x1eu;}
  if a!=b {pair_failure(id.x,k,a,b);return;}
 }
 if PAIR_POSITION_WORD!=0u {
  let position=PAIR_POSITION_WORD+id.x*8u;
  if position+8u>arrayLength(&original)||position+8u>arrayLength(&candidate) {pair_failure(id.x,0xfffffffeu,0u,1u);return;}
  for(var k=0u;k<8u;k++) {let a=original[position+k];let b=candidate[position+k];if a!=b {pair_failure(id.x,PAIR_RECORD_WORDS+k,a,b);return;}}
 }
 if original[base]==1u {pair_increment(12u);}
 if original[base]==2u {pair_increment(14u);}
}

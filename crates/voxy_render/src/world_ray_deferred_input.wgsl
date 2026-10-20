// Same 80-byte producer record. Only reserved padding words 7,11,15 may
// be mutated by the deferred queue; all coefficient, admission and ID bits stay.
struct DeferredSourceRay {
 head:array<u32,7>, count:atomic<u32>,
 middle:array<u32,3>, cursor:atomic<u32>,
 low:array<u32,3>, queued_ray:u32, tail:array<u32,4>,
}
@group(0) @binding(0) var<storage,read_write> deferred_rays:array<DeferredSourceRay>;
fn exact_ray_word_count()->u32 {return arrayLength(&deferred_rays)*20u;}
fn exact_ray_word(index:u32)->u32 {
 let ray=index/20u;let word=index%20u;
 if word<7u {return deferred_rays[ray].head[word];}
 if word==7u {return atomicLoad(&deferred_rays[ray].count);}
 if word<11u {return deferred_rays[ray].middle[word-8u];}
 if word==11u {return atomicLoad(&deferred_rays[ray].cursor);}
 if word<15u {return deferred_rays[ray].low[word-12u];}
 if word==15u {return deferred_rays[ray].queued_ray;}
 return deferred_rays[ray].tail[word-16u];
}
// One enqueue per ray maximum. A pass boundary separates append from consume.
fn deferred_append(ray:u32)->bool {
 let position=atomicAdd(&deferred_rays[0].count,1u);
 if position>=arrayLength(&deferred_rays) {return false;}
 deferred_rays[position].queued_ray=ray;return true;
}
fn deferred_take()->u32 {
 let position=atomicAdd(&deferred_rays[0].cursor,1u);
 let count=atomicLoad(&deferred_rays[0].count);
 if position>=count||count>arrayLength(&deferred_rays) {return 0xffffffffu;}
 return deferred_rays[position].queued_ray;
}

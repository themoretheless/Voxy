// Cumulative software-certificate work. Each encode excludes its first ray,
// whose padding belongs to the exact queue. No original coefficient is read.
@group(0) @binding(1) var<storage,read_write> work_stats:array<atomic<u32>>;
fn work_add_pair(base:u32,value:u32) {
 let old=atomicAdd(&work_stats[base],value);
 if old>0xffffffffu-value {atomicAdd(&work_stats[base+1u],1u);}
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;if index==0u||index>=arrayLength(&deferred_rays) {return;}
 let nodes=atomicLoad(&deferred_rays[index].count);let triangles=atomicLoad(&deferred_rays[index].cursor);
 work_add_pair(0u,1u);work_add_pair(2u,nodes);work_add_pair(4u,triangles);
 work_add_pair(6u,select(0u,1u,nodes>0u));work_add_pair(8u,select(0u,1u,triangles>0u));
 atomicMax(&work_stats[10u],nodes);atomicMax(&work_stats[11u],triangles);
}

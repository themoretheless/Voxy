// Native subgroup sums retain stable local ordering (qualified on the target GPU).
var<workgroup> subgroup_counts:array<u32,256>;
@compute @workgroup_size(256) fn mark(@builtin(global_invocation_id) global:vec3<u32>,@builtin(local_invocation_id) local:vec3<u32>,@builtin(workgroup_id) group:vec3<u32>,@builtin(subgroup_id) sid:u32,@builtin(subgroup_invocation_id) lane:u32,@builtin(num_subgroups) subgroup_count:u32) {
 var keep=0u;if global.x<mesh.count {keep=visible(spheres[global.x]);}
 let local_prefix=subgroupExclusiveAdd(keep);let local_count=subgroupAdd(keep);
 if lane==0u {subgroup_counts[sid]=local_count;}
 workgroupBarrier();
 var before=0u;for(var i=0u;i<sid;i++){before+=subgroup_counts[i];}
 if global.x<mesh.count {flags[global.x]=Flag(before+local_prefix,keep);}
 if local.x==0u {var count=0u;for(var i=0u;i<subgroup_count;i++){count+=subgroup_counts[i];}blocks[group.x].count=count;}
}

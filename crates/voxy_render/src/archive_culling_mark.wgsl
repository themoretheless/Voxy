var<workgroup> prefix:array<u32,256>;
@compute @workgroup_size(256) fn mark(@builtin(global_invocation_id) global:vec3<u32>,@builtin(local_invocation_id) local:vec3<u32>,@builtin(workgroup_id) group:vec3<u32>) {
 var keep=0u;if global.x<mesh.count {keep=visible(spheres[global.x]);}
 prefix[local.x]=keep;workgroupBarrier();
 for(var offset=1u;offset<256u;offset*=2u) {
  var add=0u;if local.x>=offset {add=prefix[local.x-offset];}
  workgroupBarrier();prefix[local.x]+=add;workgroupBarrier();
 }
 if global.x<mesh.count {flags[global.x]=Flag(prefix[local.x]-keep,keep);}
 if local.x==255u {blocks[group.x].count=prefix[255];}
}

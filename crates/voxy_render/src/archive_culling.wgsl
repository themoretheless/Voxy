// Stable three-pass compaction. Instance payloads are copied as uint bits.
struct Planes { values:array<vec4<f32>,6>, lengths:array<vec4<f32>,2> }
struct Mesh { count:u32, groups:u32, index_count:u32, reserved:u32 }
struct Instance { rows:array<vec4<u32>,7> }
struct Flag { prefix:u32, visible:u32 }
struct Block { count:u32, base:u32 }
@group(0) @binding(0) var<uniform> planes:Planes;
@group(0) @binding(1) var<uniform> mesh:Mesh;
@group(0) @binding(2) var<storage,read> source:array<Instance>;
@group(0) @binding(3) var<storage,read> spheres:array<vec4<f32>>;
@group(0) @binding(4) var<storage,read_write> flags:array<Flag>;
@group(0) @binding(5) var<storage,read_write> blocks:array<Block>;
@group(0) @binding(6) var<storage,read_write> output:array<Instance>;
@group(0) @binding(7) var<storage,read_write> arguments:array<u32,5>;
fn visible(s:vec4<f32>)->u32 {
 for(var i=0u;i<6u;i++) {
  let p=planes.values[i];let cutoff=-s.w*planes.lengths[i/4u][i%4u];
  let margin=0.00000762939453125*(dot(abs(p.xyz),abs(s.xyz))+abs(p.w)+abs(cutoff)+1.);
  if dot(p.xyz,s.xyz)+p.w<cutoff-margin {return 0u;}
 }
 return 1u;
}
@compute @workgroup_size(1) fn scan() {
 var count=0u;
 for(var i=0u;i<mesh.groups;i++){blocks[i].base=count;count+=blocks[i].count;}
 arguments[0]=mesh.index_count;arguments[1]=count;arguments[2]=0u;arguments[3]=0u;arguments[4]=0u;
}
@compute @workgroup_size(256) fn scatter(@builtin(global_invocation_id) global:vec3<u32>,@builtin(workgroup_id) group:vec3<u32>) {
 if global.x>=mesh.count {return;}let flag=flags[global.x];if flag.visible==0u {return;}
 let destination=blocks[group.x].base+flag.prefix;
 output[destination]=source[global.x];
}

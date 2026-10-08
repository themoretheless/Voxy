struct Parameters { count:u32, source:u32, vertex_base:u32, pad:u32 }
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read_write> vertices:array<u32>;
@group(0) @binding(2) var<storage,read_write> normals:array<u32>;
@group(0) @binding(3) var<uniform> parameters:Parameters;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3u) {
 let i=id.x;
 if i>=parameters.count { return; }
 let input=parameters.source+i*12u;
 let destination=parameters.vertex_base+i;
 for(var k=0u;k<9u;k++) { vertices[destination*9u+k]=source[input+k]; }
 for(var k=0u;k<3u;k++) { normals[destination*3u+k]=source[input+9u+k]; }
}

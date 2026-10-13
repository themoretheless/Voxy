struct Parameters { count:u32, source:u32, vertex_base:u32, pad:u32 }
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read_write> vertices:array<u32>;
@group(0) @binding(2) var<storage,read_write> normals:array<u32>;
@group(0) @binding(3) var<uniform> parameters:Parameters;
// Explicit solved frame director, transformed by the same final object pose as geometry.
@group(0) @binding(4) var<storage,read_write> tangents:array<vec4f>;
fn vector_at(word:u32)->vec4f {
 return vec4f(bitcast<f32>(source[word]),bitcast<f32>(source[word+1u]),bitcast<f32>(source[word+2u]),bitcast<f32>(source[word+3u]));
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3u) {
 let i=id.x;
 if i>=parameters.count { return; }
 let input=parameters.source+i*12u;
 let destination=parameters.vertex_base+i;
 let frame=source[2]+(i/(4u*source[1]))*16u;
 let pose=source[7];
 let matrix=mat4x4f(vector_at(pose),vector_at(pose+4u),vector_at(pose+8u),vector_at(pose+12u));
 let director=(matrix*vec4f(vector_at(frame+12u).xyz,0.0)).xyz;
 tangents[destination]=vec4f(normalize(director),1.0);
 for(var k=0u;k<9u;k++) { vertices[destination*9u+k]=source[input+k]; }
 for(var k=0u;k<3u;k++) { normals[destination*3u+k]=source[input+9u+k]; }
}

struct Parameters { key:vec4f, fill:vec4f, ambient:f32, count:u32, padding:vec2u }
@group(0) @binding(0) var<uniform> params:Parameters;
@group(0) @binding(1) var<storage,read> normals:array<f32>;
@group(0) @binding(2) var<storage,read_write> incident:array<vec4f>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=params.count {return;}
    let n=vec3f(normals[i*3u],normals[i*3u+1u],normals[i*3u+2u]);
    let squared=dot(n,n);
    if any((bitcast<vec3u>(n)&vec3u(0x7fffffffu))>=vec3u(0x7f800000u)) || squared<=0.0
       || (bitcast<u32>(squared)&0x7fffffffu)>=0x7f800000u {
        incident[i]=vec4f(bitcast<f32>(0x7fc00000u));return;
    }
    let unit=n*inverseSqrt(squared);
    let light=params.ambient+params.key.w*max(dot(unit,params.key.xyz),0.0)
        +params.fill.w*max(dot(unit,params.fill.xyz),0.0);
    incident[i]=vec4f(vec3f(light),1.0);
}

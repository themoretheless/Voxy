struct Binding { albedo:vec4f, vertex:u32, light:u32, surface_fraction:f32, padding:u32 }
struct Parameters { bindings:u32, lights:u32, padding:vec2u }
@group(0) @binding(0) var<storage,read> mapping:array<Binding>;
@group(0) @binding(1) var<storage,read> accepted:array<vec4f>;
@group(0) @binding(2) var<storage,read> header:array<u32>;
@group(0) @binding(3) var<storage,read_write> vertices:array<f32>;
@group(0) @binding(4) var<uniform> params:Parameters;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3u) {
    if id.x>=params.bindings || header[2]!=1u {return;}
    let m=mapping[id.x];
    let diffuse=accepted[m.light].rgb;
    let incident=accepted[params.lights+m.light].rgb;
    let color=m.albedo.rgb*(m.surface_fraction*incident+(1.0-m.surface_fraction)*diffuse);
    let base=m.vertex*9u+5u;
    vertices[base]=color.r; vertices[base+1u]=color.g; vertices[base+2u]=color.b;
    vertices[base+3u]=m.albedo.a;
}

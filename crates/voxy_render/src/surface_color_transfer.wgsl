struct Binding { sources:vec3u, destination:u32, weights:vec3f, padding:f32 }
@group(0) @binding(0) var<storage,read> mapping:array<Binding>;
@group(0) @binding(1) var<storage,read_write> vertices:array<f32>;
fn color(vertex:u32)->vec4f {
    let b=vertex*9u+5u;return vec4f(vertices[b],vertices[b+1u],vertices[b+2u],vertices[b+3u]);
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3u) {
    if id.x>=arrayLength(&mapping) {return;}
    let m=mapping[id.x];
    let value=color(m.sources.x)*m.weights.x+color(m.sources.y)*m.weights.y+color(m.sources.z)*m.weights.z;
    let b=m.destination*9u+5u;
    vertices[b]=value.r;vertices[b+1u]=value.g;vertices[b+2u]=value.b;vertices[b+3u]=value.a;
}

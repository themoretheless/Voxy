// Same barycentric mass and positive edge weights as the CPU surface equation.
// Immutable incidence lists gather contributions in source triangle order.
struct Parameters { vertices:u32, triangles:u32, edges:u32, stride:u32,
                    edge_rows:u32, pad0:u32, pad1:u32, pad2:u32 }
struct Edge { neighbor:u32, weight:f32 }
@group(0) @binding(0) var<uniform> params:Parameters;
@group(0) @binding(1) var<storage,read> positions:array<f32>;
@group(0) @binding(2) var<storage,read> triangles:array<vec4u>;
@group(0) @binding(3) var<storage,read> offsets:array<u32>;
@group(0) @binding(4) var<storage,read> incidence:array<u32>;
@group(0) @binding(5) var<storage,read_write> coefficients:array<vec4f>;
@group(0) @binding(6) var<storage,read_write> mass:array<f32>;
@group(0) @binding(7) var<storage,read_write> edges:array<Edge>;
@group(0) @binding(8) var<storage,read_write> status:atomic<u32>;
fn finite(v:vec3f)->bool {
    return all((bitcast<vec3u>(v)&vec3u(0x7fffffffu))<vec3u(0x7f800000u));
}
fn point(i:u32)->vec3f {
    let start=i*params.stride;
    return vec3f(positions[start],positions[start+1u],positions[start+2u]);
}
@compute @workgroup_size(64)
fn triangle_coefficients(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=params.triangles {return;}
    let t=triangles[i];let a=point(t.x);let b=point(t.y);let c=point(t.z);
    let ab=b-a;let ac=c-a;let bc=c-b;
    let area=0.5*length(cross(ab,ac));
    let lengths=vec3f(dot(ab,ab),dot(bc,bc),dot(ac,ac));
    if !finite(a)||!finite(b)||!finite(c)||!finite(lengths)||any(lengths<=vec3f(0.0))
        ||!finite(vec3f(area))||area<=0.0 {
        atomicOr(&status,1u);coefficients[i]=vec4f(0.0);return;
    }
    let weights=vec3f(2.0*area)/(3.0*lengths);
    if !finite(weights)||any(weights<=vec3f(0.0)) {
        atomicOr(&status,1u);coefficients[i]=vec4f(0.0);return;
    }
    coefficients[i]=vec4f(area/3.0,weights);
}
@compute @workgroup_size(64)
fn gather_mass(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=params.vertices {return;}
    var value=0.0;
    for(var j=offsets[i];j<offsets[i+1u];j++) {value+=coefficients[incidence[j]/3u].x;}
    if !finite(vec3f(value)) {atomicOr(&status,2u);value=0.0;}
    mass[i]=value;
}
@compute @workgroup_size(64)
fn gather_edges(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=params.edges {return;}
    let row=params.edge_rows+i;
    var value=0.0;
    for(var j=offsets[row];j<offsets[row+1u];j++) {
        let code=incidence[j];value+=coefficients[code/3u][1u+code%3u];
    }
    if !finite(vec3f(value)) {atomicOr(&status,2u);value=0.0;}
    edges[i].weight=value;
}

// Screened surface diffusion: (M + lambda L) x = M source.
struct Parameters { lambda: vec4f, count: u32, pad0:u32, pad1:u32, pad2:u32, alpha:vec4f, beta:vec4f }
struct Edge { neighbor:u32, weight:f32 }
@group(0) @binding(0) var<uniform> parameters:Parameters;
@group(0) @binding(1) var<storage,read> mass:array<f32>;
@group(0) @binding(2) var<storage,read> offsets:array<u32>;
@group(0) @binding(3) var<storage,read> edges:array<Edge>;
@group(0) @binding(4) var<storage,read> source:array<vec4f>;
@group(0) @binding(5) var<storage,read> previous:array<vec4f>;
@group(0) @binding(6) var<storage,read_write> next:array<vec4f>;
@group(0) @binding(7) var<storage,read_write> velocity:array<vec4f>;
@compute @workgroup_size(64)
fn diffuse(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i>=parameters.count { return; }
    var sum=vec3f(0.0);
    var degree=0.0;
    for(var j=offsets[i];j<offsets[i+1u];j++) {
        let e=edges[j];
        sum+=e.weight*previous[e.neighbor].xyz;
        degree+=e.weight;
    }
    let diagonal=vec3f(mass[i])+parameters.lambda.xyz*degree;
    if mass[i]==0.0 { next[i]=source[i]; return; }
    let value=(mass[i]*source[i].xyz+parameters.lambda.xyz*sum)/diagonal;
    var momentum=vec3f(0.0);
    if any(parameters.beta.xyz != vec3f(0.0)) { momentum=parameters.beta.xyz*velocity[i].xyz; }
    let step=parameters.alpha.xyz*(value-previous[i].xyz)+momentum;
    velocity[i]=vec4f(step,0.0);
    next[i]=vec4f(previous[i].xyz+step,1.0);
}

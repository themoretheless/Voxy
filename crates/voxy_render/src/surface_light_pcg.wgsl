// Jacobi-preconditioned CG for (M + lambda L)x = M source.
// RGB channels have independent reductions and recurrences. Dispatch boundaries
// separate global dependencies; no floating-point atomics or CPU readback.
struct Parameters { lambda:vec4f, count:u32, groups:u32, pad0:u32, pad1:u32 }
struct Edge { neighbor:u32, weight:f32 }
struct State { r:vec4f, ap:vec4f, z:vec4f }
struct Scalars { rho:vec4f, alpha:vec4f, beta:vec4f }
@group(0) @binding(0) var<uniform> params:Parameters;
@group(0) @binding(1) var<storage,read> mass:array<f32>;
@group(0) @binding(2) var<storage,read> offsets:array<u32>;
@group(0) @binding(3) var<storage,read> edges:array<Edge>;
// The uploaded incident buffer becomes the contiguous search-direction vector
// only after initialization has read every neighbor and copied the initial x.
@group(0) @binding(4) var<storage,read_write> source:array<vec4f>;
@group(0) @binding(5) var<storage,read_write> state:array<State>;
@group(0) @binding(6) var<storage,read_write> partial:array<vec4f>;
@group(0) @binding(7) var<storage,read_write> scalars:Scalars;
@group(0) @binding(8) var<storage,read_write> output:array<vec4f>;
var<workgroup> sums:array<vec4f,64>;
fn reduce(value:vec4f,lane:u32)->vec4f {
    sums[lane]=value;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride { sums[lane]+=sums[lane+stride]; }
        workgroupBarrier();
    }
    return sums[0];
}
fn divide(a:vec3f,b:vec3f)->vec3f {
    return select(vec3f(0.0),a/max(b,vec3f(1e-30)),b>vec3f(1e-30));
}
@compute @workgroup_size(64)
fn initialize(@builtin(global_invocation_id) id:vec3u,
              @builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3u) {
    let i=id.x;
    var rz=vec3f(0.0);
    if i<params.count {
        var laplace=vec3f(0.0);
        var degree=0.0;
        for(var j=offsets[i];j<offsets[i+1u];j++) {
            let e=edges[j]; degree+=e.weight; laplace+=e.weight*(source[i].xyz-source[e.neighbor].xyz);
        }
        let r=-params.lambda.xyz*laplace;
        let z=r/max(vec3f(mass[i])+params.lambda.xyz*degree,vec3f(1e-30));
        output[i]=source[i];
        state[i]=State(vec4f(r,degree),vec4f(0.0),vec4f(z,0.0));
        rz=r*z;
    }
    let sum=reduce(vec4f(rz,0.0),lane);
    if lane==0u { partial[group.x]=sum; }
}
fn partial_sum(lane:u32)->vec4f {
    var value=vec4f(0.0);
    for(var i=lane;i<params.groups;i+=64u) { value+=partial[i]; }
    return reduce(value,lane);
}
@compute @workgroup_size(64)
fn initial_rho(@builtin(local_invocation_index) lane:u32) {
    let sum=partial_sum(lane);
    if lane==0u { scalars=Scalars(sum,vec4f(0.0),vec4f(0.0)); }
}
@compute @workgroup_size(64)
fn multiply(@builtin(global_invocation_id) id:vec3u,
            @builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3u) {
    let i=id.x;
    var pap=vec3f(0.0);
    if i<params.count {
        let p=source[i].xyz;
        var laplace=vec3f(0.0);
        for(var j=offsets[i];j<offsets[i+1u];j++) {
            let e=edges[j];laplace+=e.weight*(p-source[e.neighbor].xyz);
        }
        let ap=mass[i]*p+params.lambda.xyz*laplace;
        state[i].ap=vec4f(ap,0.0);
        pap=p*ap;
    }
    let sum=reduce(vec4f(pap,0.0),lane);
    if lane==0u { partial[group.x]=sum; }
}
@compute @workgroup_size(64)
fn alpha(@builtin(local_invocation_index) lane:u32) {
    let sum=partial_sum(lane);
    if lane==0u { scalars.alpha=vec4f(divide(scalars.rho.xyz,sum.xyz),0.0); }
}
@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) id:vec3u,
          @builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3u) {
    let i=id.x;
    var rz=vec3f(0.0);
    if i<params.count {
        let a=scalars.alpha.xyz;
        output[i]=vec4f(output[i].xyz+a*source[i].xyz,1.0);
        let r=state[i].r.xyz-a*state[i].ap.xyz;
        let degree=state[i].r.w;
        let z=r/max(vec3f(mass[i])+params.lambda.xyz*degree,vec3f(1e-30));
        state[i].r=vec4f(r,degree);state[i].z=vec4f(z,0.0);
        rz=r*z;
    }
    let sum=reduce(vec4f(rz,0.0),lane);
    if lane==0u { partial[group.x]=sum; }
}
@compute @workgroup_size(64)
fn beta(@builtin(local_invocation_index) lane:u32) {
    let sum=partial_sum(lane);
    if lane==0u {
        scalars.beta=vec4f(divide(sum.xyz,scalars.rho.xyz),0.0);
        scalars.rho=sum;
    }
}
@compute @workgroup_size(64)
fn direction(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i<params.count { source[i]=vec4f(state[i].z.xyz+scalars.beta.xyz*source[i].xyz,0.0); }
}

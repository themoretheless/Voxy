struct Parameters { count:u32, weights:u32, joints:u32, trailing:u32 }
struct Weight { joints:vec4u, weights:vec4f }
struct Dual { real:vec4f, dual:vec4f }
@group(0) @binding(0) var<uniform> parameters:Parameters;
@group(0) @binding(1) var<storage,read> rest:array<f32>;
@group(0) @binding(2) var<storage,read> rest_normals:array<f32>;
@group(0) @binding(3) var<storage,read> weights:array<Weight>;
@group(0) @binding(4) var<storage,read> palette:array<Dual>;
@group(0) @binding(5) var<storage,read_write> posed:array<f32>;
@group(0) @binding(6) var<storage,read_write> normals:array<f32>;
fn multiply(a:vec4f,b:vec4f)->vec4f {
    return vec4f(a.w*b.xyz+b.w*a.xyz+cross(a.xyz,b.xyz),a.w*b.w-dot(a.xyz,b.xyz));
}
fn rotate(q:vec4f,p:vec3f)->vec3f { return p+2.0*cross(q.xyz,cross(q.xyz,p)+q.w*p); }
@compute @workgroup_size(64)
fn skin(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i>=parameters.count {return;}
    var real=vec4f(0.0,0.0,0.0,1.0);
    var dual=vec4f(0.0);
    if i<parameters.weights {
        let row=weights[i];
        var reference=0u;
        for(var k=1u;k<4u;k++) { if row.weights[k]>row.weights[reference] {reference=k;} }
        let anchor=palette[row.joints[reference]].real;
        real=vec4f(0.0);
        for(var k=0u;k<4u;k++) {
            let joint=palette[row.joints[k]];
            let w=row.weights[k]*select(1.0,-1.0,dot(anchor,joint.real)<0.0);
            real+=joint.real*w; dual+=joint.dual*w;
        }
        let magnitude=length(real);
        real/=magnitude; dual/=magnitude;
        dual-=real*dot(real,dual);
    } else if parameters.trailing<parameters.joints {
        real=palette[parameters.trailing].real; dual=palette[parameters.trailing].dual;
    }
    let translation=2.0*multiply(dual,vec4f(-real.xyz,real.w)).xyz;
    let v=i*9u;
    for(var k=0u;k<9u;k++) {posed[v+k]=rest[v+k];}
    let p=rotate(real,vec3f(rest[v],rest[v+1u],rest[v+2u]))+translation;
    posed[v]=p.x;posed[v+1u]=p.y;posed[v+2u]=p.z;
    let n=i*3u;
    let normal=rotate(real,vec3f(rest_normals[n],rest_normals[n+1u],rest_normals[n+2u]));
    normals[n]=normal.x;normals[n+1u]=normal.y;normals[n+2u]=normal.z;
}

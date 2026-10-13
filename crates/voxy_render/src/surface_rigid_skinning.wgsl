struct Parameters { count:u32, weights:u32, joints:u32, trailing:u32, offset:vec4f }
struct Weight { joints:vec4u, weights:vec4f }
struct Dual { real:vec4f, dual:vec4f }
@group(0) @binding(0) var<uniform> parameters:Parameters;
@group(0) @binding(1) var<storage,read> rest:array<f32>;
@group(0) @binding(2) var<storage,read> rest_normals:array<f32>;
@group(0) @binding(3) var<storage,read> weights:array<Weight>;
@group(0) @binding(4) var<storage,read> palette:array<Dual>;
@group(0) @binding(5) var<storage,read_write> posed:array<f32>;
@group(0) @binding(6) var<storage,read_write> normals:array<f32>;
// Explicit pairwise reduction matches the native NEON quaternion dot product.
fn paired_dot4(a:vec4f,b:vec4f)->f32 {
    let p=a*b;return (p.x+p.y)+(p.z+p.w);
}
fn refined_divide(a:vec4f,b:f32)->vec4f {
    let d=vec4f(b);let q=a/d;
    return q+fma(-q,d,a)/d;
}
fn multiply(a:vec4f,b:vec4f)->vec4f {
    // Match the linked glam 0.33.6 NEON paired product accumulation.
    let t0=a.w*b;
    let t1=a.x*b.wzyx;
    let t2=(a.y*b.zwxy)*vec4f(1.0,1.0,-1.0,-1.0);
    let t3=a.z*b.yxwz;
    return (t0+t1*vec4f(1.0,-1.0,1.0,-1.0))
        +(t2+t3*vec4f(-1.0,1.0,1.0,-1.0));
}
// Match the native quaternion-vector product's arithmetic form. The two
// algebraically equivalent forms differ in f32 and affect fine skin highlights.
fn native_cross(a:vec3f,b:vec3f)->vec3f {
    // Linked glam 0.33.6 uses non-fused vmlsq, unlike newer glam releases.
    return a.yzx*b.zxy-a.zxy*b.yzx;
}
fn rotate(q:vec4f,p:vec3f)->vec3f {
    return (p*(q.w*q.w-dot(q.xyz,q.xyz))+q.xyz*(dot(p,q.xyz)*2.0))+native_cross(q.xyz,p)*(q.w*2.0);
}
@compute @workgroup_size(64)
fn skin(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i>=parameters.count {return;}
    var real=vec4f(0.0,0.0,0.0,1.0);
    var dual=vec4f(0.0);
    if i<parameters.weights {
        let row=weights[i];
        var reference=0u;
        // CPU Iterator::max_by selects the last equal maximum. The anchor
        // determines hemisphere choices when three or more rotations blend.
        for(var k=1u;k<4u;k++) { if row.weights[k]>=row.weights[reference] {reference=k;} }
        let anchor=palette[row.joints[reference]].real;
        real=vec4f(0.0);
        for(var k=0u;k<4u;k++) {
            let joint=palette[row.joints[k]];
            let w=row.weights[k]*select(1.0,-1.0,paired_dot4(anchor,joint.real)<0.0);
            real+=joint.real*w; dual+=joint.dual*w;
        }
        let squared=paired_dot4(real,real);
        let root=sqrt(squared);
        let magnitude=root+fma(-root,root,squared)/(2.0*root);
        real=refined_divide(real,magnitude); dual=refined_divide(dual,magnitude);
        dual-=real*paired_dot4(real,dual);
    } else if parameters.trailing<parameters.joints {
        real=palette[parameters.trailing].real; dual=palette[parameters.trailing].dual;
    }
    let translation=2.0*multiply(dual,vec4f(-real.xyz,real.w)).xyz;
    let v=i*9u;
    for(var k=0u;k<9u;k++) {posed[v+k]=rest[v+k];}
    let p=rotate(real,vec3f(rest[v],rest[v+1u],rest[v+2u]))+translation+parameters.offset.xyz;
    posed[v]=p.x;posed[v+1u]=p.y;posed[v+2u]=p.z;
    let n=i*3u;
    let normal=rotate(real,vec3f(rest_normals[n],rest_normals[n+1u],rest_normals[n+2u]));
    normals[n]=normal.x;normals[n+1u]=normal.y;normals[n+2u]=normal.z;
}

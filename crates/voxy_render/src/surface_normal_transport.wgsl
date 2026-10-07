struct Triangle { ids:vec4u, u:vec4f, v:vec4f, n:vec4f, angles:vec4f }
struct Corner { triangle:u32, corner:u32 }
@group(0) @binding(0) var<storage,read> vertices:array<f32>;
@group(0) @binding(1) var<storage,read> authored:array<f32>;
@group(0) @binding(2) var<storage,read> rows:array<u32>;
@group(0) @binding(3) var<storage,read> corners:array<Corner>;
@group(0) @binding(4) var<storage,read> triangles:array<Triangle>;
@group(0) @binding(5) var<storage,read_write> normals:array<f32>;
fn position(i:u32)->vec3f { let b=i*9u; return vec3f(vertices[b],vertices[b+1u],vertices[b+2u]); }
fn unit(v:vec3f, fallback:vec3f)->vec3f {
    let length2=dot(v,v);
    if length2>0.0 { return v*inverseSqrt(length2); }
    return fallback;
}
@compute @workgroup_size(64)
fn transport(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i+1u>=arrayLength(&rows) { return; }
    let b=i*3u;
    let normal=vec3f(authored[b],authored[b+1u],authored[b+2u]);
    var sum=vec3f(0.0);
    for(var j=rows[i]; j<rows[i+1u]; j++) {
        let corner=corners[j];
        let t=triangles[corner.triangle];
        let p=position(t.ids.y)-position(t.ids.x);
        let q=position(t.ids.z)-position(t.ids.x);
        let cross_pq=cross(p,q);
        let length2=dot(cross_pq,cross_pq);
        if length2<=0.0 { continue; }
        let n1=cross_pq*inverseSqrt(length2);
        let determinant=dot(p,cross(q,n1));
        if determinant==0.0 { continue; }
        let reference=vec3f(dot(t.u.xyz,normal),dot(t.v.xyz,normal),dot(t.n.xyz,normal));
        let mapped=(cross(q,n1)*reference.x+cross(n1,p)*reference.y+cross_pq*reference.z)/determinant;
        sum+=t.angles[corner.corner]*unit(mapped,n1);
    }
    let transported=unit(sum,normal);
    normals[b]=transported.x; normals[b+1u]=transported.y; normals[b+2u]=transported.z;
}

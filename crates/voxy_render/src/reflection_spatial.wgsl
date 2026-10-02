struct Surface { position: vec4<f32>, normal_roughness: vec4<f32> }
struct Hit { position_distance: vec4<f32>, identity: vec4<u32>, barycentrics_valid: vec4<f32> }
@group(0) @binding(0) var radiance: texture_2d<f32>;
@group(0) @binding(1) var<storage,read> surfaces: array<Surface>;
@group(0) @binding(2) var<storage,read> hits: array<Hit>;
// Normal cosine, primary tangent-plane separation in world units,
// relative reflected distance difference, minimum roughness.
@group(0) @binding(3) var<uniform> settings: vec4<f32>;
@group(0) @binding(4) var output: texture_storage_2d<rgba32float,write>;
fn finite3(v: vec3<f32>) -> bool { return all(abs(v)<=vec3<f32>(3.402823e38)); }
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(output);
    if any(id.xy>=size) { return; }
    let p=vec2<i32>(id.xy); let index=id.y*size.x+id.x;
    let s=surfaces[index]; let h=hits[index];
    var centre=textureLoad(radiance,p,0).rgb;
    if !finite3(centre) { centre=vec3<f32>(0.); }
    centre=max(centre,vec3<f32>(0.));
    if h.barycentrics_valid.w!=1. { centre=vec3<f32>(0.); }
    if s.position.w!=1. || !finite3(s.position.xyz) || !finite3(s.normal_roughness.xyz) || !finite3(vec3<f32>(s.normal_roughness.w,h.position_distance.w,0.)) || s.normal_roughness.w<settings.w {
        textureStore(output,p,vec4<f32>(centre,1.)); return;
    }
    var color=centre; var weight=1.;
    for(var y:i32=-1;y<=1;y+=1) { for(var x:i32=-1;x<=1;x+=1) {
        if x==0 && y==0 { continue; }
        let q=p+vec2<i32>(x,y);
        if any(q<vec2<i32>(0)) || any(q>=vec2<i32>(size)) { continue; }
        let j=u32(q.y)*size.x+u32(q.x); let t=surfaces[j]; let k=hits[j];
        if t.position.w!=1. || !finite3(vec3<f32>(t.normal_roughness.w,k.position_distance.w,0.)) { continue; }
        if h.barycentrics_valid.w==1. && k.barycentrics_valid.w==1. && any(h.identity.xyz!=k.identity.xyz) { continue; }
        let n=s.normal_roughness.xyz; let m=t.normal_roughness.xyz;
        if !finite3(n) || !finite3(m) || !finite3(t.position.xyz) || !finite3(s.position.xyz) { continue; }
        if dot(n,m)<settings.x || abs(s.normal_roughness.w-t.normal_roughness.w)>.05 { continue; }
        let separation=t.position.xyz-s.position.xyz;
        if max(abs(dot(separation,n)),abs(dot(separation,m)))>settings.y { continue; }
        let distance=max(abs(h.position_distance.w),.000001);
        if h.barycentrics_valid.w==1. && k.barycentrics_valid.w==1. && abs(h.position_distance.w-k.position_distance.w)>settings.z*distance { continue; }
        var sample=textureLoad(radiance,q,0).rgb;
        if k.barycentrics_valid.w!=1. { sample=vec3<f32>(0.); }
        if !finite3(sample) { continue; }
        let w=select(.5,.25,x!=0 && y!=0);
        // Convex incremental blend avoids overflow from summing large HDR samples.
        color=color+(max(sample,vec3<f32>(0.))-color)*(w/(weight+w)); weight+=w;
    }}
    textureStore(output,p,vec4<f32>(color,1.));
}

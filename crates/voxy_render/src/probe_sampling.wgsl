// Baked irradiance probes (order-1 SH of scene radiance, see probe_gi.wgsl).
// Returns cosine-convolved radiance / pi (the same quantity as the diffuse
// environment cube) and a weight that fades to zero outside the grid.
struct ProbeGrid { origin:vec4<f32>, spacing:vec4<f32>, dims:vec4<u32> }
@group(2) @binding(5) var<storage,read> probe_sh:array<vec4<f32>>;
@group(2) @binding(6) var<uniform> probe_grid:ProbeGrid;
fn probe_eval(index:u32,n:vec3<f32>)->vec3<f32>{
    let b=index*4u;
    // E(n)/pi = Y00 L00 + (2/3) * 0.488603 * (L1 . n), Ramamoorthi & Hanrahan 2001.
    let e=probe_sh[b].xyz*0.282095+(probe_sh[b+1u].xyz*n.y+probe_sh[b+2u].xyz*n.z+probe_sh[b+3u].xyz*n.x)*0.325735;
    return max(e,vec3<f32>(0.));
}
fn probe_irradiance(p:vec3<f32>,n:vec3<f32>)->vec4<f32>{
    if probe_grid.origin.w<0.5 {return vec4<f32>(0.);}
    let dims=vec3<f32>(probe_grid.dims.xyz);
    // Bias along the normal so surfaces read probes on their lit side.
    let g=(p+n*0.4-probe_grid.origin.xyz)/probe_grid.spacing.xyz;
    let inside=clamp(min(g+0.5,dims-0.5-g),vec3<f32>(0.),vec3<f32>(1.));
    let fade=inside.x*inside.y*inside.z;
    if fade<=0. {return vec4<f32>(0.);}
    let c=clamp(g,vec3<f32>(0.),dims-1.);
    let base=vec3<u32>(min(floor(c),dims-2.));
    let f=c-vec3<f32>(base);
    var sum=vec3<f32>(0.);
    for(var i=0u;i<8u;i++){
        let o=vec3<u32>(i&1u,(i>>1u)&1u,(i>>2u)&1u);
        let q=base+o;
        let w=select(1.-f.x,f.x,o.x==1u)*select(1.-f.y,f.y,o.y==1u)*select(1.-f.z,f.z,o.z==1u);
        sum+=probe_eval(q.x+probe_grid.dims.x*(q.y+probe_grid.dims.y*q.z),n)*w;
    }
    return vec4<f32>(sum,fade);
}

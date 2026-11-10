// Dual Kawase bloom (Bjorge 2015): each downsample halves resolution with a
// 5-tap filter, each upsample doubles it with an 8-tap tent and is added onto
// the level above. Bilinear taps at half-texel offsets make every pass cover a
// wide footprint cheaply, so a tiny bright sun spreads into a smooth halo.
// #include "fullscreen.wgsl"
// #include "tonemap.wgsl"
@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var linear:sampler;
// xy = source texel size, z = 1 on the first downsample (firefly weighting)
@group(0) @binding(2) var<uniform> params:vec4<f32>;

fn tap(uv:vec2<f32>)->vec3<f32>{
    let c=max(textureSampleLevel(source,linear,uv,0.).rgb,vec3<f32>(0.));
    // Karis average on the first level: weight by 1/(1+luma) so single very
    // bright pixels cannot flicker as the TAA jitter moves them.
    if params.z>0.5 {return karis_weight_rgb(c);}
    return c;
}
@fragment fn fs_down(o:FullscreenVertexOutput)->@location(0) vec4<f32> {
    let h=params.xy*0.5;
    var sum=tap(o.uv)*4.;
    sum+=tap(o.uv-h);sum+=tap(o.uv+h);
    sum+=tap(o.uv+vec2<f32>(h.x,-h.y));sum+=tap(o.uv-vec2<f32>(h.x,-h.y));
    var c=sum/8.;
    // Undo the Karis weighting so the chain stays in linear radiance.
    if params.z>0.5 {c=c/max(1.-luma_bt709(c),0.0001);}
    return vec4<f32>(c,1.);
}
@fragment fn fs_up(o:FullscreenVertexOutput)->@location(0) vec4<f32> {
    let h=params.xy*0.5;
    var sum=tap(o.uv+vec2<f32>(-h.x*2.,0.));
    sum+=tap(o.uv+vec2<f32>(-h.x,h.y))*2.;
    sum+=tap(o.uv+vec2<f32>(0.,h.y*2.));
    sum+=tap(o.uv+vec2<f32>(h.x,h.y))*2.;
    sum+=tap(o.uv+vec2<f32>(h.x*2.,0.));
    sum+=tap(o.uv+vec2<f32>(h.x,-h.y))*2.;
    sum+=tap(o.uv+vec2<f32>(0.,-h.y*2.));
    sum+=tap(o.uv+vec2<f32>(-h.x,-h.y))*2.;
    return vec4<f32>(sum/12.,1.);
}

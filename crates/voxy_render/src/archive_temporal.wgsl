struct Frame {
    inverse_vp:mat4x4<f32>, previous_vp:mat4x4<f32>,
    current_vp:mat4x4<f32>, eye:vec4<f32>,
    // current jitter UV, previous jitter UV (history is jittered raster space).
    jitter:vec4<f32>,
    // reset, history weight, display transform mode, display headroom
    settings:vec4<f32>,
}
@group(0) @binding(0) var depth:texture_depth_2d;
@group(0) @binding(1) var<uniform> frame:Frame;
struct Vertex { @builtin(position) position:vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) i:u32)->Vertex {
    var v:Vertex;
    v.position=vec4<f32>(f32((i<<1u)&2u)*2.-1.,1.-f32(i&2u)*2.,0.,1.);
    return v;
}
// Reconstruct planar depth on the same stable grid without blending across
// foreground/background discontinuities. Reuse this for guides and saved depth.
fn stable_depth(p:vec2<i32>,size:vec2<u32>)->f32 {
    let center=textureLoad(depth,p,0);
    let position=vec2<f32>(p)+frame.jitter.xy*vec2<f32>(size);
    let base=vec2<i32>(floor(position));let f=fract(position);
    var sum=0.;var total=0.;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let q=clamp(base+vec2<i32>(x,y),vec2<i32>(0),vec2<i32>(size)-1);
        let z=textureLoad(depth,q,0);
        let same_sky=center>=1. && z>=1.;
        let same_surface=center<1. && z<1. && abs(z-center)<max(0.00005,(1.-center)*0.02);
        if same_sky || same_surface {
            let weight=select(1.-f.x,f.x,x==1)*select(1.-f.y,f.y,y==1);
            sum+=z*weight;total+=weight;
        }
    }}
    if total>0.01 {return sum/total;}
    return center;
}
struct Guides { @location(0) motion:vec2<f32>, @location(1) expected:f32 }
@fragment fn fs_guides(@builtin(position) p:vec4<f32>)->Guides {
    let size=vec2<f32>(textureDimensions(depth));
    // Our TAA accumulates on the stable output grid. MetalFX receives guides
    // on its jittered input grid and removes jitter internally.
    var uv=p.xy/size;
    if frame.settings.y>0. {uv+=frame.jitter.xy;}
    var z=textureLoad(depth,vec2<i32>(p.xy),0);
    if frame.settings.y>0. {z=stable_depth(vec2<i32>(p.xy),textureDimensions(depth));}
    let h=frame.inverse_vp*vec4<f32>(uv*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),z,1.);
    var world=vec4<f32>(h.xyz/h.w,1.);
    // Sky has rotation-only motion. Camera translation must not move infinity.
    if z>=1. { world=vec4<f32>(world.xyz-frame.eye.xyz,0.); }
    let previous=frame.previous_vp*world;
    let current=frame.current_vp*world;
    var g:Guides;
    g.motion=vec2<f32>(2.);g.expected=-1.;
    if previous.w>0. && current.w>0. {
        let old_uv=previous.xy/previous.w*vec2<f32>(0.5,-0.5)+0.5;
        let current_uv=current.xy/current.w*vec2<f32>(0.5,-0.5)+0.5;
        g.motion=old_uv-current_uv;
        g.expected=select(previous.z/previous.w,1.,z>=1.);
    }
    return g;
}
@group(1) @binding(0) var current:texture_2d<f32>;
@group(1) @binding(1) var history:texture_2d<f32>;
@group(1) @binding(2) var history_depth:texture_2d<f32>;
@group(1) @binding(3) var motion:texture_2d<f32>;
@group(1) @binding(4) var expected:texture_2d<f32>;
@group(1) @binding(5) var resolved:texture_storage_2d<rgba16float,write>;
@group(1) @binding(6) var saved_depth:texture_storage_2d<r32float,write>;
fn finite(c:vec3<f32>)->bool {return all((bitcast<vec3<u32>>(c)&vec3<u32>(0x7f800000u))!=vec3<u32>(0x7f800000u));}
fn current_stable(p:vec2<i32>,size:vec2<u32>)->vec3<f32> {
    let position=vec2<f32>(p)+frame.jitter.xy*vec2<f32>(size);
    let base=vec2<i32>(floor(position));let f=fract(position);
    let limit=vec2<i32>(size)-1;
    let a=textureLoad(current,clamp(base,vec2<i32>(0),limit),0).rgb;
    let b=textureLoad(current,clamp(base+vec2<i32>(1,0),vec2<i32>(0),limit),0).rgb;
    let c=textureLoad(current,clamp(base+vec2<i32>(0,1),vec2<i32>(0),limit),0).rgb;
    let d=textureLoad(current,clamp(base+vec2<i32>(1,1),vec2<i32>(0),limit),0).rgb;
    return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
@compute @workgroup_size(8,8) fn cs_resolve(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=textureDimensions(current);
    if any(id.xy>=size) {return;}
    let p=vec2<i32>(id.xy);
    var color=max(textureLoad(current,p,0).rgb,vec3<f32>(0.));
    if frame.settings.y>0. {color=max(current_stable(p,size),vec3<f32>(0.));}
    var z=textureLoad(depth,p,0);
    if frame.settings.y>0. {z=stable_depth(p,size);}
    let uv=(vec2<f32>(p)+0.5)/vec2<f32>(size)+textureLoad(motion,p,0).xy;
    let wanted=textureLoad(expected,p,0).x;
    if frame.settings.x<0.5 && frame.settings.y>0. && wanted>=0. && all(uv>=vec2<f32>(0.)) && all(uv<vec2<f32>(1.)) {
        let pos=uv*vec2<f32>(size)-0.5;
        let base=vec2<i32>(floor(pos));let f=fract(pos);
        var accumulated=vec3<f32>(0.);var total=0.;
        // Depth-reject each bilinear tap to avoid mixing foreground/background.
        for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
            let q=base+vec2<i32>(x,y);
            if any(q<vec2<i32>(0)) || any(q>=vec2<i32>(size)) {continue;}
            let old_z=textureLoad(history_depth,q,0).x;
            let sky=wanted>=1. && old_z>=1.;
            let surface=wanted<1. && old_z<1. && abs(wanted-old_z)<max(0.00005,(1.-wanted)*0.02);
            if sky || surface {
                let weight=select(1.-f.x,f.x,x==1)*select(1.-f.y,f.y,y==1);
                let old=textureLoad(history,q,0).rgb;
                if finite(old) {accumulated+=old*weight;total+=weight;}
            }
        }}
        if total>0.01 {
            var low=color;var high=color;
            for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
                let c=textureLoad(current,clamp(p+vec2<i32>(x,y),vec2<i32>(0),vec2<i32>(size)-1),0).rgb;
                low=min(low,c);high=max(high,c);
            }}
            let old=clamp(accumulated/total,low,high);
            // Luminance disagreement reduces history on changing/translucent surfaces.
            let luma=vec3<f32>(0.2126,0.7152,0.0722);
            let disagreement=abs(dot(old-color,luma))/max(max(dot(old,luma),dot(color,luma)),0.01);
            let weight=frame.settings.y*(1.-0.8*disagreement);
            color=mix(color,old,weight);
        }
    }
    textureStore(resolved,p,vec4<f32>(color,1.));
    textureStore(saved_depth,p,vec4<f32>(z));
}
@group(2) @binding(0) var final_colour:texture_2d<f32>;
@fragment fn fs_display(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32> {
    var x=max(textureLoad(final_colour,vec2<i32>(p.xy),0).rgb,vec3<f32>(0.));
    x=display_transform(x,frame.settings.z,frame.settings.w);
    return vec4<f32>(x,1.);
}

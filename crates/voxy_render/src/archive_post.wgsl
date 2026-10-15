// The scene remains linear HDR. Only the selected display transform is applied.
@group(0) @binding(0) var hdr:texture_2d<f32>;
@group(0) @binding(1) var<uniform> display:vec4<f32>;
struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32> }
@group(0) @binding(2) var scene_depth:texture_depth_2d;
@group(0) @binding(3) var<uniform> camera:Camera;
@group(1) @binding(0) var environment_specular:texture_cube<f32>;
@group(1) @binding(2) var environment_sampler:sampler;
@group(1) @binding(4) var<uniform> environment_intensity:vec4<f32>;
struct Out { @builtin(position) position:vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) i:u32)->Out {
    var o:Out;
    o.position=vec4<f32>(f32((i<<1u)&2u)*2.-1.,1.-f32(i&2u)*2.,0.,1.);
    return o;
}
fn display_colour(x:vec3<f32>)->vec3<f32> {
    if display.x<0.5 {
        return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),vec3<f32>(0.),vec3<f32>(1.));
    }
    // Preserve linear midtones and hue; smoothly compress only highlights into
    // the current display headroom, rather than clipping everything above 1.
    let peak=max(x.r,max(x.g,x.b));
    let knee=0.75;
    if peak<=knee { return x; }
    let span=max(display.y-knee,0.0001);
    let mapped=knee+span*(1.-exp(-(peak-knee)/span));
    return x*(mapped/peak);
}
fn pixel_colour(pixel:vec2<i32>)->vec3<f32> {
    let q=clamp(pixel,vec2<i32>(0),vec2<i32>(textureDimensions(hdr))-vec2<i32>(1));
    var x=max(textureLoad(hdr,q,0).rgb,vec3<f32>(0.));
    if textureLoad(scene_depth,q,0)>=1. {
        let h=camera.inverse_vp*vec4<f32>((vec2<f32>(q)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),1.,1.);
        let direction=normalize(h.xyz/h.w-camera.eye.xyz);
        if camera.eye.w>0.5 {
            x=textureSampleLevel(environment_specular,environment_sampler,direction,0.).rgb*environment_intensity.x*camera.viewport.z;
        } else {x=daylight(direction,camera.sun.xyz)*camera.viewport.z;}
    }
    return display_colour(x);
}
// Filter after the display transform, before the attachment's sRGB encoding.
// Reconstruct sky at every tap so silhouette edges participate in AA too.
fn filtered_colour(p:vec2<f32>)->vec3<f32> {
    let base=vec2<i32>(floor(p));
    let f=fract(p);
    return mix(mix(pixel_colour(base),pixel_colour(base+vec2<i32>(1,0)),f.x),
               mix(pixel_colour(base+vec2<i32>(0,1)),pixel_colour(base+vec2<i32>(1,1)),f.x),f.y);
}
fn luminance(c:vec3<f32>)->f32 { return dot(c,vec3<f32>(0.299,0.587,0.114)); }
@fragment fn fs_main(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32> {
    let q=vec2<i32>(p.xy);
    let center=pixel_colour(q);
    if display.z<0.5 { return vec4<f32>(center,1.); }
    let nw=luminance(pixel_colour(q+vec2<i32>(-1,-1)));
    let ne=luminance(pixel_colour(q+vec2<i32>(1,-1)));
    let sw=luminance(pixel_colour(q+vec2<i32>(-1,1)));
    let se=luminance(pixel_colour(q+vec2<i32>(1,1)));
    let mid=luminance(center);
    let low=min(mid,min(min(nw,ne),min(sw,se)));
    let high=max(mid,max(max(nw,ne),max(sw,se)));
    // Leave low-contrast texture detail untouched, including extended HDR.
    if high-low<max(0.0312,high*0.125) { return vec4<f32>(center,1.); }
    var direction=vec2<f32>(-(nw+ne-sw-se),nw+sw-ne-se);
    let reduction=max((nw+ne+sw+se)*(0.25/8.),1./128.);
    direction=clamp(direction/(min(abs(direction.x),abs(direction.y))+reduction),vec2<f32>(-8.),vec2<f32>(8.));
    let position=vec2<f32>(q);
    let a=0.5*(filtered_colour(position+direction*(1./3.-0.5))+
               filtered_colour(position+direction*(2./3.-0.5)));
    let b=a*0.5+0.25*(filtered_colour(position-direction*0.5)+filtered_colour(position+direction*0.5));
    let lb=luminance(b);
    return vec4<f32>(select(b,a,lb<low || lb>high),1.);
}

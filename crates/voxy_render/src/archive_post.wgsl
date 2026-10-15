// The scene remains linear HDR. Only the selected display transform is applied.
@group(0) @binding(0) var hdr:texture_2d<f32>;
@group(0) @binding(1) var<uniform> display:vec4<f32>;
struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32>, weather:vec4<f32>, atmosphere:vec4<f32>, wind:vec4<f32> }
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
fn weather_hash(p:vec3<f32>)->f32 {return fract(sin(dot(p,vec3<f32>(127.1,311.7,74.7)))*43758.5453);}
fn weather_noise(p:vec3<f32>)->f32 {
 let i=floor(p);let f=fract(p);let u=f*f*(3.-2.*f);
 return mix(mix(mix(weather_hash(i),weather_hash(i+vec3<f32>(1.,0.,0.)),u.x),mix(weather_hash(i+vec3<f32>(0.,1.,0.)),weather_hash(i+vec3<f32>(1.,1.,0.)),u.x),u.y),mix(mix(weather_hash(i+vec3<f32>(0.,0.,1.)),weather_hash(i+vec3<f32>(1.,0.,1.)),u.x),mix(weather_hash(i+vec3<f32>(0.,1.,1.)),weather_hash(i+vec3<f32>(1.,1.,1.)),u.x),u.y),u.z);
}
// Analytic gust integration keeps cloud and precipitation motion continuous.
fn wind_displacement()->vec3<f32> {
 let t=camera.atmosphere.x;
 let travel=t+camera.wind.z*(0.65*(1.-cos(0.7*t))/0.7+0.35*(1.-cos(1.9*t))/1.9);
 return vec3<f32>(camera.wind.x,0.,camera.wind.y)*travel;
}
fn wind_velocity()->vec3<f32> {
 let t=camera.atmosphere.x;
 let gust=1.+camera.wind.z*(0.65*sin(0.7*t)+0.35*sin(1.9*t));
 return vec3<f32>(camera.wind.x,0.,camera.wind.y)*gust;
}
fn cloud_density(p:vec3<f32>)->f32 {
 let height=(p.y-camera.atmosphere.y)/camera.atmosphere.z;
 let drift=wind_displacement();
 let q=(p-drift)*0.006;
 let n=weather_noise(q)*0.65+weather_noise(q*2.1)*0.25+weather_noise(q*4.3)*0.1;
 return max(n-(1.-camera.weather.x),0.)*camera.weather.y*smoothstep(0.,0.15,height)*(1.-smoothstep(0.65,1.,height));
}
fn atmosphere_colour(base:vec3<f32>,ray:vec3<f32>,distance:f32)->vec3<f32> {
 var result=base;
 // Intersect an actual world-space cloud slab, including views from inside it.
 if camera.weather.x>0. && abs(ray.y)>0.0001 {
  let a=(camera.atmosphere.y-camera.eye.y)/ray.y;
  let b=(camera.atmosphere.y+camera.atmosphere.z-camera.eye.y)/ray.y;
  let start=max(min(a,b),0.);let end=min(max(a,b),min(distance,12000.));
  if end>start {
   let step=(end-start)/32.;var transmission=1.;var scattered=vec3<f32>(0.);
   for(var i=0;i<32;i++) {
    let p=camera.eye.xyz+ray*(start+(f32(i)+0.5)*step);
    let density=cloud_density(p);let opacity=1.-exp(-density*step*0.035);
    let shade=exp(-cloud_density(p+camera.sun.xyz*70.)*5.);
    let light=mix(vec3<f32>(65.,80.,105.),vec3<f32>(740.,710.,660.),shade*max(camera.sun.y,0.));
    scattered+=transmission*opacity*light;transmission*=1.-opacity;
    if transmission<0.01 {break;}
   }
   result=result*transmission+scattered*camera.viewport.z;
  }
 }
 // World-space precipitation cells stop at the first scene surface.
 if camera.weather.z+camera.weather.w>0. {
  let reach=min(distance,36.);let step=reach/64.;var opacity=0.;
  for(var i=0;i<64;i++) {
   let p=camera.eye.xyz+ray*((f32(i)+0.5)*step);
   let drift=wind_displacement();
   // Shear streaks along horizontal wind / terminal fall velocity.
   let rain=p-drift*0.3+wind_velocity()*(p.y-camera.eye.y)*0.3/24.+vec3<f32>(0.,camera.atmosphere.x*24.,0.);
   let flutter=vec3<f32>(sin(camera.atmosphere.x*1.3+p.y),0.,cos(camera.atmosphere.x+p.y))*0.08*length(camera.wind.xy);
   let snow=p-drift*0.2+flutter+vec3<f32>(0.,camera.atmosphere.x*1.5,0.);
   let rc=floor(rain*vec3<f32>(3.,0.6,3.));let rf=fract(rain*vec3<f32>(3.,0.6,3.));
   let sc=floor(snow*2.);let sf=fract(snow*2.)-vec3<f32>(0.5);
   let streak=(1.-smoothstep(0.025,0.07,length(rf.xz-vec2<f32>(0.5))))*select(0.,1.,weather_hash(rc)<camera.weather.z*0.35);
   let flake=(1.-smoothstep(0.035,0.11,length(sf)))*select(0.,1.,weather_hash(sc)<camera.weather.w*0.45);
   opacity+= (streak*0.3+flake*1.5)*step;
  }
  result=mix(result,vec3<f32>(350.,400.,450.)*camera.viewport.z,1.-exp(-opacity));
 }
 return result;
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
    if camera.weather.x+camera.weather.z+camera.weather.w>0. {
        let uv=(vec2<f32>(q)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
        let far=camera.inverse_vp*vec4<f32>(uv,1.,1.);
        let ray=normalize(far.xyz/far.w-camera.eye.xyz);
        let d=textureLoad(scene_depth,q,0);
        let hit=camera.inverse_vp*vec4<f32>(uv,d,1.);
        let distance=select(length(hit.xyz/hit.w-camera.eye.xyz),12000.,d>=1.);
        x=atmosphere_colour(x,ray,distance);
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

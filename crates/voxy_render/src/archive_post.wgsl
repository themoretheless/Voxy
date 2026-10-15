// The scene remains linear HDR. Only the selected display transform is applied.
@group(0) @binding(0) var hdr:texture_2d<f32>;
@group(0) @binding(1) var<uniform> display:vec4<f32>;
struct Camera { vp:mat4x4<f32>, shadow:mat4x4<f32>, inverse_vp:mat4x4<f32>, eye:vec4<f32>, sun:vec4<f32>, sun_colour:vec4<f32>, viewport:vec4<f32>, weather:vec4<f32>, atmosphere:vec4<f32>, wind:vec4<f32> }
@group(0) @binding(2) var scene_depth:texture_depth_2d;
@group(0) @binding(3) var<uniform> camera:Camera;
@group(1) @binding(0) var environment_specular:texture_cube<f32>;
@group(1) @binding(2) var environment_sampler:sampler;
@group(1) @binding(4) var<uniform> environment_intensity:vec4<f32>;
@group(0) @binding(4) var sky_panorama:texture_2d<f32>;
@group(0) @binding(5) var sky_sampler:sampler;
@group(0) @binding(6) var bloom_chain:texture_2d<f32>;
// Energy-conserving bloom from the dual Kawase chain (bloom.rs, six levels).
// Only for direct display; temporal and linear output bloom elsewhere.
fn with_bloom(x:vec3<f32>,q:vec2<i32>)->vec3<f32>{
    if display.w>0.5 || display.x>2.5 {return x;}
    let uv=(vec2<f32>(q)+0.5)/vec2<f32>(textureDimensions(hdr));
    return mix(x,textureSampleLevel(bloom_chain,sky_sampler,uv,0.).rgb/6.,0.04);
}
struct Out { @builtin(position) position:vec4<f32> }
@vertex fn vs_main(@builtin(vertex_index) i:u32)->Out {
    var o:Out;
    o.position=vec4<f32>(f32((i<<1u)&2u)*2.-1.,1.-f32(i&2u)*2.,0.,1.);
    return o;
}
fn display_colour(x:vec3<f32>)->vec3<f32> {
    if display.w>0.5 {return x;}
    return display_transform(x,display.x,display.y);
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
 // Scroll each octave through the noise's vertical axis at its own rate so the
 // field evolves in shape instead of only translating; fine detail churns fastest.
 let t=camera.atmosphere.x;
 let n=weather_noise(q+vec3<f32>(0.,t*0.003,0.))*0.65
  +weather_noise(q*2.1+vec3<f32>(t*0.002,-t*0.009,0.))*0.25
  +weather_noise(q*4.3+vec3<f32>(0.,t*0.025,-t*0.006))*0.1;
 return max(n-(1.-camera.weather.x),0.)*camera.weather.y*smoothstep(0.,0.15,height)*(1.-smoothstep(0.65,1.,height));
}
// Interleaved gradient noise decorrelates march start between neighbours so
// step/cell aliasing becomes fine noise instead of arcs. A sin hash of raw
// pixel coordinates loses fp32 precision and keeps visible structure. Kept
// static per pixel: a per-frame rotation lets TAA average the streaks away.
fn march_jitter(pixel:vec2<i32>)->f32 {
 let p=vec2<f32>(pixel);
 return fract(52.9829189*fract(dot(p,vec2<f32>(0.06711056,0.00583715))));
}
// World-space precipitation cells stop at the first scene surface.
// Returns opacity of the precipitation layer; evaluated once per output pixel.
const STREAK_GAIN:f32=1.5;
fn precipitation_opacity(ray:vec3<f32>,distance:f32,pixel:vec2<i32>)->f32 {
 let reach=min(distance,36.);let step=reach/48.;var opacity=0.;
 let jitter=march_jitter(pixel);
 let drift=wind_displacement();let t=camera.atmosphere.x;
 // Rain space is linear in p: shear streaks along wind / terminal fall velocity.
 let shear=wind_velocity()*(0.3/24.);
 let rain_scale=vec3<f32>(3.,0.6,3.);
 let rain_origin=(camera.eye.xyz-drift*0.3+vec3<f32>(0.,t*24.,0.))*rain_scale;
 let rain_dir=(ray+shear*ray.y)*rain_scale;
 let rain_on=camera.weather.z*0.35;let snow_on=camera.weather.w*0.45;
 let flutter_amp=0.08*length(camera.wind.xy);
 // Looking steeply up, streaks collapse into lines converging on the zenith; fade them there.
 let side=smoothstep(0.25,0.65,length(ray.xz));
 // World length of the ray inside one streak core (~0.1 cell wide).
 let streak_length=min(reach/48.,0.1/max(length(rain_dir.xz),1e-3))*STREAK_GAIN;
 for(var i=0;i<48;i++) {
  let d=(f32(i)+jitter)*step;
  if rain_on>0. && side>0. {
   // Closest approach of this step's segment to the streak column of the cell
   // under its midpoint: coverage stays continuous instead of point-sampled.
   let a=rain_origin+rain_dir*(d-0.5*step);let ab=rain_dir*step;
   let rc=floor(a+ab*0.5);
   let u=clamp(dot(rc.xz+vec2<f32>(0.5)-a.xz,ab.xz)/max(dot(ab.xz,ab.xz),1e-6),0.,1.);
   let rf=a+ab*u-rc;
   // Finite streaks: unbroken per-cell columns line up toward the zenith vanishing point.
   let along=smoothstep(0.,0.15,rf.y)*(1.-smoothstep(0.45,0.6,rf.y));
   let streak=(1.-smoothstep(0.025,0.07,length(rf.xz-vec2<f32>(0.5))))*along*select(0.,1.,weather_hash(rc)<rain_on)*streak_length;
   opacity+=streak*0.3*side;
  }
  if snow_on>0. {
   let p=camera.eye.xyz+ray*d;
   let flutter=vec3<f32>(sin(t*1.3+p.y),0.,cos(t+p.y))*flutter_amp;
   let snow=(p-drift*0.2+flutter+vec3<f32>(0.,t*1.5,0.))*2.;
   let sc=floor(snow);let sf=snow-sc-vec3<f32>(0.5);
   let flake=(1.-smoothstep(0.035,0.11,length(sf)))*select(0.,1.,weather_hash(sc)<snow_on);
   opacity+=flake*1.5*step;
  }
 }
 return 1.-exp(-opacity);
}
fn atmosphere_colour(base:vec3<f32>,ray:vec3<f32>,distance:f32,pixel:vec2<i32>)->vec3<f32> {
 var result=base;
 let jitter=march_jitter(pixel);
 // Intersect an actual world-space cloud slab, including views from inside it.
 if camera.weather.x>0. && abs(ray.y)>0.0001 {
  let a=(camera.atmosphere.y-camera.eye.y)/ray.y;
  let b=(camera.atmosphere.y+camera.atmosphere.z-camera.eye.y)/ray.y;
  let start=max(min(a,b),0.);let end=min(max(a,b),min(distance,12000.));
  if end>start {
   let step=(end-start)/32.;var transmission=1.;var scattered=vec3<f32>(0.);
   for(var i=0;i<32;i++) {
    let p=camera.eye.xyz+ray*(start+(f32(i)+jitter)*step);
    let density=cloud_density(p);let opacity=1.-exp(-density*step*0.035);
    let shade=exp(-cloud_density(p+camera.sun.xyz*70.)*5.);
    let light=mix(vec3<f32>(65.,80.,105.),vec3<f32>(740.,710.,660.),shade*max(camera.sun.y,0.));
    scattered+=transmission*opacity*light;transmission*=1.-opacity;
    if transmission<0.01 {break;}
   }
   result=result*transmission+scattered*camera.viewport.z;
  }
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
            let uv=vec2<f32>(atan2(direction.z,direction.x)/6.2831853+0.5,acos(clamp(direction.y,-1.,1.))/3.14159265);
            x=textureSampleLevel(sky_panorama,sky_sampler,uv,0.).rgb*environment_intensity.x*camera.viewport.z;
        } else {x=daylight(direction,camera.sun.xyz)*camera.viewport.z;}
    }
    if camera.weather.x>0. {
        let uv=(vec2<f32>(q)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
        let far=camera.inverse_vp*vec4<f32>(uv,1.,1.);
        let ray=normalize(far.xyz/far.w-camera.eye.xyz);
        let d=textureLoad(scene_depth,q,0);
        let hit=camera.inverse_vp*vec4<f32>(uv,d,1.);
        let distance=select(length(hit.xyz/hit.w-camera.eye.xyz),12000.,d>=1.);
        x=atmosphere_colour(x,ray,distance,q);
    }
    return display_colour(with_bloom(x,q));
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
    return vec4<f32>(with_precipitation(antialiased(q),q),1.);
}
// Precipitation is composited once, after FXAA, instead of in every FXAA tap.
fn world_at(q:vec2<i32>)->vec3<f32> {
    let p=clamp(q,vec2<i32>(0),vec2<i32>(textureDimensions(scene_depth))-vec2<i32>(1));
    let uv=(vec2<f32>(p)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
    let h=camera.inverse_vp*vec4<f32>(uv,textureLoad(scene_depth,p,0),1.);
    return h.xyz/h.w;
}
// Raindrop impacts on upward-facing surfaces: per world cell, a drop lands at a
// hashed spot and time, leaving a short expanding ring and a brief bright crown.
fn splash_opacity(p:vec3<f32>,q:vec2<i32>,distance:f32)->f32 {
    // Normal from depth neighbours; only roughly horizontal ground gets splashes.
    let n=normalize(cross(world_at(q+vec2<i32>(0,1))-p,world_at(q+vec2<i32>(1,0))-p));
    let up=smoothstep(0.75,0.95,abs(n.y));
    if up<=0. { return 0.; }
    let t=camera.atmosphere.x;var cover=0.;
    for(var layer=0;layer<3;layer++) {
        let o=f32(layer)*vec2<f32>(0.37,0.71);
        let g=p.xz*3.+o;let cell=floor(g);
        let h=vec3<f32>(cell,f32(layer));
        if weather_hash(h+vec3<f32>(0.,0.,7.))>camera.weather.z*0.8 { continue; }
        let period=0.45+0.35*weather_hash(h+vec3<f32>(3.,0.,0.));
        let age=fract(t/period+weather_hash(h))*period;
        let life=0.22;
        if age>life { continue; }
        let k=age/life;
        let centre=cell+vec2<f32>(0.25)+0.5*vec2<f32>(weather_hash(h+vec3<f32>(1.,0.,0.)),weather_hash(h+vec3<f32>(0.,1.,0.)));
        let r=length(g-centre)/3.;
        let radius=0.015+0.07*sqrt(k);
        // Ring width grows with distance so far splashes stay at least ~a pixel wide.
        let width=max(0.006,distance*0.0012);
        let ring=(1.-smoothstep(0.,width,abs(r-radius)))*(1.-k);
        let crown=(1.-smoothstep(0.,0.012+width,r))*(1.-smoothstep(0.,0.25,k));
        cover=max(cover,max(ring*0.55,crown*0.8));
    }
    return cover*up*(1.-smoothstep(15.,30.,distance));
}
fn with_precipitation(c:vec3<f32>,q:vec2<i32>)->vec3<f32> {
    if camera.weather.z+camera.weather.w<=0. { return c; }
    let uv=(vec2<f32>(q)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
    let far=camera.inverse_vp*vec4<f32>(uv,1.,1.);
    let ray=normalize(far.xyz/far.w-camera.eye.xyz);
    let d=textureLoad(scene_depth,q,0);
    let hit=camera.inverse_vp*vec4<f32>(uv,d,1.);
    let distance=select(length(hit.xyz/hit.w-camera.eye.xyz),12000.,d>=1.);
    var a=precipitation_opacity(ray,distance,q);
    if camera.weather.z>0. && d<1. && distance<30. {
        a=1.-(1.-a)*(1.-splash_opacity(hit.xyz/hit.w,q,distance));
    }
    return mix(c,display_colour(vec3<f32>(350.,400.,450.)*camera.viewport.z),a);
}
fn antialiased(q:vec2<i32>)->vec3<f32> {
    let center=pixel_colour(q);
    if display.z<0.5 { return center; }
    let nw=luminance(pixel_colour(q+vec2<i32>(-1,-1)));
    let ne=luminance(pixel_colour(q+vec2<i32>(1,-1)));
    let sw=luminance(pixel_colour(q+vec2<i32>(-1,1)));
    let se=luminance(pixel_colour(q+vec2<i32>(1,1)));
    let mid=luminance(center);
    let low=min(mid,min(min(nw,ne),min(sw,se)));
    let high=max(mid,max(max(nw,ne),max(sw,se)));
    // Leave low-contrast texture detail untouched, including extended HDR.
    if high-low<max(0.0312,high*0.125) { return center; }
    var direction=vec2<f32>(-(nw+ne-sw-se),nw+sw-ne-se);
    let reduction=max((nw+ne+sw+se)*(0.25/8.),1./128.);
    direction=clamp(direction/(min(abs(direction.x),abs(direction.y))+reduction),vec2<f32>(-8.),vec2<f32>(8.));
    let position=vec2<f32>(q);
    let a=0.5*(filtered_colour(position+direction*(1./3.-0.5))+
               filtered_colour(position+direction*(2./3.-0.5)));
    let b=a*0.5+0.25*(filtered_colour(position-direction*0.5)+filtered_colour(position+direction*0.5));
    let lb=luminance(b);
    return select(b,a,lb<low || lb>high);
}

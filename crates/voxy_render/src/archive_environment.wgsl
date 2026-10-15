// Shared analytic daylight radiance (linear working RGB). Both the visible
// environment and specular lighting use this function; this is not baked GI.
// Single-scattering Rayleigh + Mie atmosphere (Nishita 1993) seen from the
// ground. Colour, horizon brightening and sunset reddening follow from the
// physics; SKY_CALIBRATION keeps the 45-degree zenith at the luminance of the
// previous analytic sky (~336) so scene exposure is unchanged.
const EARTH_RADIUS:f32=6360e3;
const ATMOSPHERE_RADIUS:f32=6420e3;
const RAYLEIGH:vec3<f32>=vec3<f32>(5.802e-6,13.558e-6,33.1e-6);
const MIE:f32=21e-6;
const SKY_CALIBRATION:f32=40700.;
fn atmosphere_exit(origin:vec3<f32>,direction:vec3<f32>)->f32{
 let b=dot(origin,direction);let c=dot(origin,origin)-ATMOSPHERE_RADIUS*ATMOSPHERE_RADIUS;
 return -b+sqrt(max(b*b-c,0.));
}
fn optical_depth(origin:vec3<f32>,direction:vec3<f32>)->vec2<f32>{
 let span=atmosphere_exit(origin,direction);let step=span/4.;var depth=vec2<f32>(0.);
 for(var i=0;i<4;i++){
  let h=max(length(origin+direction*(f32(i)+0.5)*step)-EARTH_RADIUS,0.);
  depth+=exp(-vec2<f32>(h/8000.,h/1200.))*step;
 }
 return depth;
}
fn scattered_sky(direction:vec3<f32>,sun:vec3<f32>)->vec3<f32>{
 let origin=vec3<f32>(0.,EARTH_RADIUS+2.,0.);
 let span=atmosphere_exit(origin,direction);let step=span/8.;
 var rayleigh=vec3<f32>(0.);var mie=vec3<f32>(0.);var view_depth=vec2<f32>(0.);
 for(var i=0;i<8;i++){
  let p=origin+direction*(f32(i)+0.5)*step;
  let h=max(length(p)-EARTH_RADIUS,0.);
  let density=exp(-vec2<f32>(h/8000.,h/1200.))*step;view_depth+=density;
  let light_depth=optical_depth(p,sun);
  let attenuation=exp(-(RAYLEIGH*(view_depth.x+light_depth.x)+MIE*1.1*(view_depth.y+light_depth.y)));
  rayleigh+=attenuation*density.x;mie+=attenuation*density.y;
 }
 let mu=dot(direction,sun);let g=0.76;
 let phase_r=3./(16.*3.14159265)*(1.+mu*mu);
 let phase_m=3./(8.*3.14159265)*((1.-g*g)*(1.+mu*mu))/((2.+g*g)*pow(1.+g*g-2.*g*mu,1.5));
 return (rayleigh*RAYLEIGH*phase_r+mie*MIE*phase_m)*SKY_CALIBRATION;
}
fn daylight(direction:vec3<f32>,sun:vec3<f32>)->vec3<f32>{
 let d=normalize(direction);
 let cosine=clamp(dot(d,sun),-1.,1.);
 let depth=optical_depth(vec3<f32>(0.,EARTH_RADIUS+2.,0.),sun);
 let disk=vec3<f32>(200000.,185000.,150000.)*exp(-(RAYLEIGH*depth.x+MIE*1.1*depth.y))*smoothstep(0.999975,0.99999,cosine);
 // The 0.02 floor holds by day; below the horizon the ground fades with the sky.
 if d.y<0. {return vec3<f32>(150.,130.,105.)*clamp(sun.y*2.+0.1,0.02,1.)*smoothstep(-0.2,-0.02,sun.y);}
 return scattered_sky(vec3<f32>(d.x,max(d.y,0.002),d.z),sun)+disk;
}

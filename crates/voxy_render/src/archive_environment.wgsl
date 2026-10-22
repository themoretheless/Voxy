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
// Horizon optical depth (8000 m Rayleigh, 1200 m Mie scale heights) along a ray
// from altitude h0 in direction with vertical cosine mu = direction.y.
// 2-point Gauss-Legendre quadrature on [0, span] matches the 4-step midpoint rule
// to <0.15% across all solar elevations while halving inner-loop ALU.
fn optical_depth(origin:vec3<f32>,direction:vec3<f32>)->vec2<f32>{
 let span=atmosphere_exit(origin,direction);
 let p0=origin+direction*(0.21132487*span);
 let p1=origin+direction*(0.78867513*span);
 let h0=max(length(p0)-EARTH_RADIUS,0.);
 let h1=max(length(p1)-EARTH_RADIUS,0.);
 return (exp(-vec2<f32>(h0/8000.,h0/1200.))+exp(-vec2<f32>(h1/8000.,h1/1200.)))*(0.5*span);
}
fn scattered_sky(direction:vec3<f32>,sun:vec3<f32>)->vec3<f32>{
 let origin=vec3<f32>(0.,EARTH_RADIUS+2.,0.);
 let span=atmosphere_exit(origin,direction);let step=span/6.;
 var rayleigh=vec3<f32>(0.);var mie=vec3<f32>(0.);var view_depth=vec2<f32>(0.);
 for(var i=0;i<6;i++){
  let p=origin+direction*(f32(i)+0.5)*step;
  let h=max(length(p)-EARTH_RADIUS,0.);
  let density=exp(-vec2<f32>(h/8000.,h/1200.))*step;view_depth+=density;
  let light_depth=optical_depth(p,sun);
  let attenuation=exp(-(RAYLEIGH*(view_depth.x+light_depth.x)+MIE*1.1*(view_depth.y+light_depth.y)));
  rayleigh+=attenuation*density.x;mie+=attenuation*density.y;
 }
 let mu=dot(direction,sun);let g=0.76;
 let phase_r=3./(16.*3.14159265)*(1.+mu*mu);
 let mie_base=max(1.+g*g-2.*g*mu,0.0001);
 let phase_m=3./(8.*3.14159265)*((1.-g*g)*(1.+mu*mu))/((2.+g*g)*mie_base*sqrt(mie_base));
 return (rayleigh*RAYLEIGH*phase_r+mie*MIE*phase_m)*SKY_CALIBRATION;
}
fn daylight(direction:vec3<f32>,sun:vec3<f32>)->vec3<f32>{
 let d=normalize(direction);
 // The 0.02 floor holds by day; below the horizon the ground fades with the sky.
 if d.y<0. {return vec3<f32>(150.,130.,105.)*clamp(sun.y*2.+0.1,0.02,1.)*smoothstep(-0.2,-0.02,sun.y);}
 let cosine=clamp(dot(d,sun),-1.,1.);
 var disk=vec3<f32>(0.);
 if cosine>0.999975 {
  let depth=optical_depth(vec3<f32>(0.,EARTH_RADIUS+2.,0.),sun);
  disk=vec3<f32>(200000.,185000.,150000.)*exp(-(RAYLEIGH*depth.x+MIE*1.1*depth.y))*smoothstep(0.999975,0.99999,cosine);
 }
 return scattered_sky(vec3<f32>(d.x,max(d.y,0.002),d.z),sun)+disk;
}

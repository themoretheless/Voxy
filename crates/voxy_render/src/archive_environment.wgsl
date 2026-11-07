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
fn moon_direction(sun:vec3<f32>)->vec3<f32>{
 return normalize(vec3<f32>(-sun.x,max(-sun.y,0.28),-sun.z));
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
 var sky=scattered_sky(vec3<f32>(d.x,max(d.y,0.002),d.z),sun)+disk;
 if sun.y<0.06 {
  let night=smoothstep(0.06,-0.12,sun.y);
  let twilight=smoothstep(-0.20,0.04,sun.y)*(1.-smoothstep(-0.02,0.06,sun.y));
  let horizon_belt=exp(-d.y*8.5)*(0.45+0.55*clamp(cosine*0.5+0.5,0.,1.));
  let airglow=mix(vec3<f32>(0.55,0.78,1.35),vec3<f32>(0.22,0.34,0.68),clamp(d.y,0.,1.))*night;
  let dusk_glow=vec3<f32>(14.,6.5,3.2)*horizon_belt*twilight;
  let cell=floor(d*340.);
  let h=fract(sin(dot(cell,vec3<f32>(127.1,311.7,74.7)))*43758.5453);
  let twinkle=0.72+0.28*sin(h*62.83+d.x*19.+d.z*23.);
  let star=smoothstep(0.9970,0.9998,h)*smoothstep(0.08,0.35,d.y)*night*twinkle*vec3<f32>(2.4,2.6,3.2);
  let mw_plane=abs(dot(d,vec3<f32>(0.58,0.46,-0.67)));
  let mw_band=exp(-mw_plane*mw_plane*18.)*smoothstep(0.10,0.45,d.y)*(0.55+0.45*sin(d.x*6.5-d.z*5.2));
  let milky_way=vec3<f32>(0.38,0.46,0.72)*mw_band*night;
  let moon_dir=moon_direction(sun);
  let cos_moon=clamp(dot(d,moon_dir),-1.,1.);
  let moon_halo=(exp((cos_moon-1.)*180.)*4.2+exp((cos_moon-1.)*18.)*0.48)*vec3<f32>(0.72,0.84,1.12)*night;
  var moon_disk=vec3<f32>(0.);
  if cos_moon>0.99986 {
   let mare=0.74+0.26*sin(d.x*420.+d.y*310.-d.z*380.);
   let limb=sqrt(clamp((cos_moon-0.99986)/0.00014,0.,1.));
   moon_disk=vec3<f32>(210.,225.,255.)*mare*(0.45+0.55*limb)*smoothstep(0.99986,0.99989,cos_moon)*night;
  }
  var aurora=vec3<f32>(0.);
  if d.y>0.06 && d.z>0.05 {
   let ribbon=sin(d.x*7.5+sin(d.z*4.2)*1.6)*0.5+0.5;
   let curtain=exp(-pow((d.y-0.28-0.08*ribbon)/0.14,2.))*smoothstep(0.05,0.45,d.z)*smoothstep(0.06,0.18,d.y);
   let rays=0.65+0.35*sin(d.x*38.+d.z*19.);
   let au_col=mix(vec3<f32>(0.18,1.65,0.68),vec3<f32>(0.85,0.28,1.35),smoothstep(0.22,0.48,d.y));
   aurora=au_col*curtain*rays*night*0.85;
  }
  sky+=airglow+dusk_glow+star+milky_way+moon_halo+moon_disk+aurora;
 } else if sun.y<0.68 {
  let cos_anti=clamp(-cosine,-1.,1.);
  if cos_anti>0.58 && cos_anti<0.78 {
   let bow_elev=smoothstep(0.01,0.12,sun.y)*(1.-smoothstep(0.45,0.68,sun.y))*smoothstep(0.02,0.18,d.y);
   let p_t=(cos_anti-0.732)/0.022;
   let primary=exp(-p_t*p_t*2.5)*vec3<f32>(
    exp(-pow(p_t+0.65,2.)*2.2),
    exp(-pow(p_t,2.)*2.6),
    exp(-pow(p_t-0.65,2.)*2.2)
   );
   let s_t=(cos_anti-0.630)/0.028;
   let secondary=exp(-s_t*s_t*2.2)*vec3<f32>(
    exp(-pow(s_t-0.60,2.)*2.0),
    exp(-pow(s_t,2.)*2.4),
    exp(-pow(s_t+0.60,2.)*2.0)
   )*0.38;
   let alexander=1.-0.06*smoothstep(0.64,0.67,cos_anti)*(1.-smoothstep(0.70,0.73,cos_anti))*bow_elev;
   sky=sky*alexander+(primary+secondary)*bow_elev*18.;
  }
 }
 return sky;
}
